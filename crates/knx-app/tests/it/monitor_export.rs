//! Independently authored monitoring projection tests; no imported project required.
use knx_app::monitor_export::{build, ExportOptions, ExportReport};
use knx_core::*;
use serde_json::{json, Value};
use std::io::{Cursor, Read};

mod fixtures;
use fixtures::*;

#[test]
fn synthetic_current_snapshot_is_deterministic_without_mutation() {
    let p = fixture();
    let before = p.clone();
    let a = build(&p, None, &ExportOptions::default()).unwrap();
    let b = build(&p, None, &ExportOptions::default()).unwrap();
    assert_eq!(a.bytes, b.bytes);
    assert_eq!(p, before);
    let (v, r, readme) = unpack(&a.bytes);
    assert_eq!(v.as_object().unwrap().len(), 8);
    assert_eq!(v["info"]["name"], "Current project");
    assert_eq!(r.profile, "monitoring-sparse-v1");
    assert_eq!(r.format_version, 1);
    assert_eq!(r.producer, "KNXBench");
    for c in [
        &r.counts.devices,
        &r.counts.communication_objects,
        &r.counts.group_addresses,
        &r.counts.spaces,
        &r.counts.functions,
    ] {
        assert_eq!((c.selected, c.exported, c.omitted), (1, 1, 0));
    }
    assert!(readme.is_ascii());
    assert!(readme.contains("not a backup"));
    assert!(readme.contains("receiver"));
    assert!(!r.excluded_data.is_empty());
}
#[test]
fn unknown_security_and_each_unknown_flag_are_omitted_not_false() {
    let (v, r) = document(&fixture());
    assert!(v["group_addresses"]["1/0/3"].get("data_secure").is_none());
    assert_eq!(
        v["communication_objects"]["1.1.9/O-1"]["flags"],
        json!({"read":false,"write":true})
    );
    for name in ["communication", "transmit", "update", "read_on_init"] {
        assert!(has_issue(&r, "O-1", &format!("flags.{name}")));
    }
    assert!(has_issue(&r, "GA-1", "data_secure"));
}
#[test]
fn declared_ga_and_current_ko_dpts_stay_separate() {
    let (v, r) = document(&fixture());
    assert_eq!(
        v["group_addresses"]["1/0/3"]["dpt"],
        json!({"main":1,"sub":2})
    );
    assert_eq!(
        v["communication_objects"]["1.1.9/O-1"]["dpts"],
        json!([{"main":1,"sub":1}])
    );
    assert!(has_issue(&r, "GA-1", "dpt"));
}
#[test]
fn cleared_or_malformed_ko_dpt_never_resurrects_program_default() {
    for state in [
        Override::Empty,
        Override::Malformed("not-a-dpt".into()),
        Override::Absent,
    ] {
        let mut p = fixture();
        p.devices
            .com_object_mut(ComObjectInstanceId(1))
            .unwrap()
            .dpt = state;
        p.devices.set_program_defaults(
            ComObjectInstanceId(1),
            ProgramDefaults {
                dpt: Some(Resolved {
                    value: DptRef {
                        main: 9,
                        sub: Some(1),
                    },
                    layer: Layer::Program,
                }),
                ..Default::default()
            },
        );
        let (v, r) = document(&p);
        assert_eq!(v["communication_objects"]["1.1.9/O-1"]["dpts"], json!([]));
        assert!(has_issue(&r, "O-1", "dpts"));
    }
}
#[test]
fn missing_ga_dpt_uses_only_unambiguous_linked_current_values() {
    let mut p = fixture();
    p.installations[0].group_addresses[0].declared_dpt = Override::Absent;
    let (v, r) = document(&p);
    assert_eq!(
        v["group_addresses"]["1/0/3"]["dpt"],
        json!({"main":1,"sub":1})
    );
    assert!(r
        .issues
        .iter()
        .any(|i| i.entity == "GA-1" && i.code == "inferred"));
    let mut other = p
        .devices
        .com_object(ComObjectInstanceId(1))
        .unwrap()
        .clone();
    other.id = ComObjectInstanceId(2);
    other.dpt = value(DptRef {
        main: 5,
        sub: Some(1),
    });
    p.devices.insert_com_object(other);
    p.devices
        .get_mut(DeviceId(1))
        .unwrap()
        .com_objects
        .push(ComObjectInstanceId(2));
    let (v, r) = document(&p);
    assert!(v["group_addresses"]["1/0/3"]["dpt"].is_null());
    assert!(has_issue(&r, "GA-1", "dpt"));
}
#[test]
fn numeric_unknown_and_main_only_dpts_are_preserved() {
    let mut p = fixture();
    p.installations[0].group_addresses[0].declared_dpt = value(DptRef {
        main: 65000,
        sub: None,
    });
    p.devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .dpt = value(DptRef {
        main: 65000,
        sub: None,
    });
    let (v, _) = document(&p);
    assert_eq!(
        v["group_addresses"]["1/0/3"]["dpt"],
        json!({"main":65000,"sub":null})
    );
}
#[test]
fn three_styles_rewrite_every_reference_consistently() {
    for (style, key) in [
        (GroupAddressStyle::ThreeLevel, "1/0/3"),
        (GroupAddressStyle::TwoLevel, "1/3"),
        (GroupAddressStyle::Free, "2051"),
    ] {
        let mut p = fixture();
        p.info.group_address_style = style;
        let (v, _) = document(&p);
        assert_eq!(
            v["communication_objects"]["1.1.9/O-1"]["group_address_links"],
            json!([key])
        );
        assert_eq!(
            v["group_addresses"][key]["communication_object_ids"],
            json!(["1.1.9/O-1"])
        );
        assert_eq!(v["group_ranges"]["R-1"]["group_addresses"], json!([key]));
        assert_eq!(
            v["functions"]["F-1"]["group_addresses"][key]["address"],
            key
        );
    }
}
#[test]
fn addressless_devices_and_owned_objects_have_explicit_omission_counts() {
    let mut p = fixture();
    p.devices.get_mut(DeviceId(1)).unwrap().address = None;
    let (v, r) = document(&p);
    assert_eq!(
        (
            r.counts.devices.selected,
            r.counts.devices.exported,
            r.counts.devices.omitted
        ),
        (1, 0, 1)
    );
    assert_eq!(
        (
            r.counts.communication_objects.selected,
            r.counts.communication_objects.exported,
            r.counts.communication_objects.omitted
        ),
        (1, 0, 1)
    );
    assert_eq!(v["devices"], json!({}));
    assert_eq!(v["communication_objects"], json!({}));
    assert_eq!(v["locations"]["Room"]["devices"], json!([]));
    assert_eq!(
        v["group_addresses"]["1/0/3"]["communication_object_ids"],
        json!([])
    );
    assert!(has_issue(&r, "D-1", "individual_address"));
}
#[test]
fn building_only_and_addressed_unassigned_devices_do_not_need_ia_ownership_guess() {
    let mut p = fixture();
    p.installations[0].topology.lines[0].devices.clear();
    let (v, _) = document(&p);
    assert!(v["devices"].get("1.1.9").is_some());
    p.installations[0].buildings[0].devices.clear();
    p.installations[0].topology.unassigned.push(DeviceId(1));
    let (v, _) = document(&p);
    assert!(v["devices"].get("1.1.9").is_some());
}
#[test]
fn device_without_any_ownership_reference_is_refused_not_assigned_by_ia() {
    let mut p = fixture();
    p.installations[0].topology.lines[0].devices.clear();
    p.installations[0].buildings[0].devices.clear();
    refuses(&p, "ownership");
}
#[test]
fn duplicate_ga_and_individual_address_keys_refuse() {
    let mut p = fixture();
    let mut ga = p.installations[0].group_addresses[0].clone();
    ga.id = GroupAddressId(2);
    p.installations[0].group_addresses.push(ga);
    refuses(&p, "duplicate");
    let mut p = fixture();
    let mut d = p.devices.get(DeviceId(1)).unwrap().clone();
    d.id = DeviceId(2);
    d.com_objects.clear();
    p.devices.insert(d);
    p.installations[0].topology.unassigned.push(DeviceId(2));
    refuses(&p, "duplicate");
}
#[test]
fn duplicate_vector_ids_refuse_before_dictionary_insertion() {
    let mut p = fixture();
    let duplicate = p.installations[0].buildings[0].clone();
    p.installations[0].buildings.push(duplicate);
    refuses(&p, "duplicate");
}
#[test]
fn duplicate_sibling_names_and_unsupported_space_kinds_refuse() {
    let mut p = fixture();
    let mut b = p.installations[0].buildings[0].clone();
    b.id = BuildingPartId(2);
    p.installations[0].buildings.push(b);
    refuses(&p, "sibling");
    let mut p = fixture();
    p.installations[0].buildings[0].kind = BuildingPartType::RoomPart;
    refuses(&p, "RoomPart");
}
#[test]
fn dangling_and_cross_owner_refs_refuse() {
    for case in 0..5 {
        let mut p = fixture();
        match case {
            0 => p.installations[0].topology.lines[0]
                .devices
                .push(DeviceId(999)),
            1 => {
                p.devices
                    .com_object_mut(ComObjectInstanceId(1))
                    .unwrap()
                    .links[0]
                    .ga = GroupAddressId(999)
            }
            2 => {
                p.devices
                    .com_object_mut(ComObjectInstanceId(1))
                    .unwrap()
                    .device = DeviceId(999)
            }
            3 => p.functions[0].members[0].group_address = GroupAddressId(999),
            _ => p.installations[0].buildings[0]
                .children
                .push(BuildingPartId(999)),
        };
        refuses(&p, "reference");
    }
}
#[test]
fn orphan_or_multiply_listed_objects_refuse() {
    let mut p = fixture();
    p.devices.get_mut(DeviceId(1)).unwrap().com_objects.clear();
    refuses(&p, "object");
    let mut p = fixture();
    p.devices
        .get_mut(DeviceId(1))
        .unwrap()
        .com_objects
        .push(ComObjectInstanceId(1));
    refuses(&p, "duplicate");
}
#[test]
fn inconsistent_and_cyclic_hierarchies_refuse_without_panicking() {
    let mut p = fixture();
    p.installations[0].buildings[0].parent = Some(BuildingPartId(1));
    p.installations[0].buildings[0]
        .children
        .push(BuildingPartId(1));
    refuses(&p, "hierarchy");
    let mut p = fixture();
    p.installations[0].group_ranges[0]
        .children
        .push(GroupRangeId(999));
    refuses(&p, "reference");
}
#[test]
fn duplicate_function_member_ga_refuses() {
    let mut p = fixture();
    let mut m = p.functions[0].members[0].clone();
    m.id = FunctionMemberId(2);
    p.functions[0].members.push(m);
    refuses(&p, "duplicate");
}
#[test]
fn installation_selection_requires_explicit_choice_and_disallows_foreign_links() {
    let mut p = fixture();
    let mut i = p.installations[0].clone();
    i.id = InstallationId(1);
    i.topology.areas.clear();
    i.topology.lines.clear();
    i.buildings.clear();
    i.group_addresses.clear();
    i.group_ranges.clear();
    i.default_line = None;
    p.installations.push(i);
    refuses(&p, "installation");
    let b = build(
        &p,
        None,
        &ExportOptions {
            installation: Some(InstallationId(0)),
            language: None,
        },
    )
    .unwrap();
    assert_eq!(b.report.installation, 0);
    p.installations[1].topology.unassigned.push(DeviceId(1));
    assert!(build(
        &p,
        None,
        &ExportOptions {
            installation: Some(InstallationId(0)),
            language: None
        }
    )
    .is_err());
}
#[test]
fn current_edits_survive_native_reopen_and_input_stays_unchanged() {
    let mut p = fixture();
    p.info.name = "Renamed current".into();
    p.devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .text = value(Text::Literal("Edited".into()));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &p).unwrap();
    drop(conn);
    let before = std::fs::read(&path).unwrap();
    let conn = knx_store::open_existing_read_only(&path).unwrap();
    let reopened = knx_store::load_project(&conn.conn).unwrap();
    let (v, _) = document(&reopened);
    assert_eq!(v["info"]["name"], "Renamed current");
    assert_eq!(v["communication_objects"]["1.1.9/O-1"]["text"], "Edited");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    if let Some(output) = std::env::var_os("KNXBENCH_MONITOR_ACCEPTANCE_DIR") {
        let output = std::path::PathBuf::from(output);
        std::fs::create_dir(&output).unwrap();
        std::fs::copy(&path, output.join("synthetic.knxdb")).unwrap();
        let b = build(&reopened, None, &ExportOptions::default()).unwrap();
        std::fs::write(output.join("synthetic.zip"), &b.bytes).unwrap();
        std::fs::write(
            output.join("report.json"),
            serde_json::to_vec_pretty(&b.report).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn metadata_gaps_module_activity_and_product_unavailability_are_explicit() {
    let (v, r) = document(&fixture());
    assert_eq!(v["info"]["guid"], "");
    assert_eq!(v["info"]["schema_version"], "");
    assert_eq!(v["info"]["xknxproject_version"], "");
    assert_eq!(
        v["communication_objects"]["1.1.9/O-1"]["module_def"],
        Value::Null
    );
    assert!(v["communication_objects"]["1.1.9/O-1"]
        .get("module")
        .is_none());
    assert_eq!(v["devices"]["1.1.9"]["channels"], json!({}));
    assert!(has_issue(&r, "O-1", "is_active"));
    assert!(has_issue(&r, "D-1", "product_metadata"));
}
#[test]
fn text_uses_requested_string_table_language_without_overwriting_literals() {
    let mut p = fixture();
    let k = TranslationKey("text".into());
    p.strings
        .insert(k.clone(), Language("en-US".into()), "Default".into());
    p.strings
        .insert(k.clone(), Language("fr-FR".into()), "Actuel".into());
    p.devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .text = value(Text::Localized(LocalizedString(k)));
    let b = build(
        &p,
        None,
        &ExportOptions {
            installation: None,
            language: Some("fr-FR".into()),
        },
    )
    .unwrap();
    let (v, _, _) = unpack(&b.bytes);
    assert_eq!(v["communication_objects"]["1.1.9/O-1"]["text"], "Actuel");
    assert_eq!(v["info"]["language_code"], "fr-FR");
}
#[test]
fn verified_media_vocabulary_does_not_guess_mt3_is_ip() {
    for (raw, expected) in [
        ("MT-0", "Twisted Pair (TP)"),
        ("MT-1", "Powerline (PL)"),
        ("MT-2", "KNX RF (RF)"),
        ("MT-5", "KNXnet/IP (IP)"),
        ("MT-3", "Unknown"),
    ] {
        let mut p = fixture();
        p.installations[0].topology.lines[0].medium_ref = raw.into();
        let (v, r) = document(&p);
        assert_eq!(v["topology"]["1"]["lines"]["1"]["medium_type"], expected);
        if raw == "MT-3" {
            assert!(has_issue(&r, "L-1", "medium_type"));
        }
    }
}
#[test]
fn send_links_precede_receive_and_direction_loss_is_reported() {
    let mut p = fixture();
    let mut ga = p.installations[0].group_addresses[0].clone();
    ga.id = GroupAddressId(2);
    ga.address = GroupAddress::from_raw(2052);
    p.installations[0].group_addresses.push(ga);
    p.devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .links = vec![
        GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Receive,
        },
        GroupLink {
            ga: GroupAddressId(2),
            direction: Direction::Send,
        },
    ];
    let (v, r) = document(&p);
    assert_eq!(
        v["communication_objects"]["1.1.9/O-1"]["group_address_links"],
        json!(["1/0/4", "1/0/3"])
    );
    assert!(has_issue(&r, "O-1", "group_address_links"));
}
#[test]
fn empty_project_and_unknown_installation_refuse() {
    let p = Project::new(Language("en".into()));
    refuses(&p, "installation");
    assert!(build(
        &fixture(),
        None,
        &ExportOptions {
            installation: Some(InstallationId(99)),
            language: None
        }
    )
    .is_err());
}

mod corrections;
mod integrity;
mod limits;
mod preflight;
mod products;
