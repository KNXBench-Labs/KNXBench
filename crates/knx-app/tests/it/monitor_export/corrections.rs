//! Monitoring export graph admission and mapping correction regressions.
use super::*;

#[test]
fn ga_width_conflict_is_null_and_reports_both_candidates() {
    let mut p = fixture();
    p.devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .dpt = value(DptRef {
        main: 5,
        sub: Some(1),
    });
    let (v, r) = document(&p);
    assert!(v["group_addresses"]["1/0/3"]["dpt"].is_null());
    let issue = r
        .issues
        .iter()
        .find(|i| i.entity == "GA-1" && i.code == "dpt_conflict")
        .unwrap();
    assert!(issue.detail.contains("SizeConflict"));
    assert!(issue.detail.contains("main: 1"));
    assert!(issue.detail.contains("main: 5"));
}
#[test]
fn duplicate_numeric_topology_keys_and_invalid_numbers_refuse() {
    for case in 0..4 {
        let mut p = fixture();
        match case {
            0 => {
                let mut a = p.installations[0].topology.areas[0].clone();
                a.id = AreaId(2);
                a.lines.clear();
                p.installations[0].topology.areas.push(a);
            }
            1 => {
                let mut l = p.installations[0].topology.lines[0].clone();
                l.id = LineId(2);
                l.devices.clear();
                p.installations[0].topology.lines.push(l);
                p.installations[0].topology.areas[0].lines.push(LineId(2));
            }
            2 => p.installations[0].topology.areas[0].address = 16,
            _ => p.installations[0].topology.lines[0].address = 16,
        }
        refuses(&p, if case < 2 { "duplicate" } else { "exceeds 15" });
    }
}
#[test]
fn selected_lines_need_exactly_one_area_parent() {
    let mut p = fixture();
    p.installations[0].topology.areas[0].lines.clear();
    refuses(&p, "ownership");
    let mut p = fixture();
    let mut a = p.installations[0].topology.areas[0].clone();
    a.id = AreaId(2);
    a.address = 2;
    p.installations[0].topology.areas.push(a);
    refuses(&p, "duplicate");
}
#[test]
fn reference_budget_counts_duplicate_entries_before_deduplication() {
    let mut p = fixture();
    p.installations[0].topology.unassigned = vec![DeviceId(1); 1_000_001];
    refuses(&p, "reference limit");
}
#[test]
fn repeated_device_placement_within_one_installation_counts_once() {
    let mut p = fixture();
    let mut l = p.installations[0].topology.lines[0].clone();
    l.id = LineId(2);
    l.address = 2;
    p.installations[0].topology.lines.push(l);
    p.installations[0].topology.areas[0].lines.push(LineId(2));
    p.installations[0].topology.unassigned.push(DeviceId(1));
    let (v, r) = document(&p);
    assert_eq!(r.counts.devices.selected, 1);
    assert_eq!(r.counts.devices.exported, 1);
    assert_eq!(
        v["topology"]["1"]["lines"]["2"]["devices"],
        json!(["1.1.9"])
    );
}
#[test]
fn nonselected_functions_and_devices_are_outside_counts_not_omissions() {
    let mut p = fixture();
    let mut i = p.installations[0].clone();
    i.id = InstallationId(1);
    i.topology.areas[0].id = AreaId(2);
    i.topology.areas[0].lines = vec![LineId(2)];
    i.topology.lines[0].id = LineId(2);
    i.topology.lines[0].devices = vec![DeviceId(2)];
    i.default_line = Some(LineId(2));
    i.buildings[0].id = BuildingPartId(2);
    i.buildings[0].default_line = Some(LineId(2));
    i.buildings[0].devices = vec![DeviceId(2)];
    i.group_ranges[0].id = GroupRangeId(2);
    i.group_addresses[0].id = GroupAddressId(2);
    i.group_addresses[0].range = Some(GroupRangeId(2));
    p.installations.push(i);
    let mut d = p.devices.get(DeviceId(1)).unwrap().clone();
    d.id = DeviceId(2);
    d.com_objects.clear();
    p.devices.insert(d);
    let mut f = p.functions[0].clone();
    f.id = FunctionId(2);
    f.location = BuildingPartId(2);
    f.members[0].id = FunctionMemberId(2);
    f.members[0].group_address = GroupAddressId(2);
    p.functions.push(f);
    let b = build(
        &p,
        None,
        &ExportOptions {
            installation: Some(InstallationId(0)),
            language: None,
        },
    )
    .unwrap();
    let (v, r, _) = unpack(&b.bytes);
    for c in [
        &r.counts.devices,
        &r.counts.functions,
        &r.counts.group_addresses,
    ] {
        assert_eq!((c.selected, c.exported, c.omitted), (1, 1, 0));
    }
    assert_eq!(v["devices"].as_object().unwrap().len(), 1);
}
#[test]
fn catalogue_query_errors_are_not_silently_missing_metadata() {
    let conn = knx_productdb::Connection::open_in_memory().unwrap();
    let e = match build(&fixture(), Some(&conn), &ExportOptions::default()) {
        Err(e) => e,
        Ok(_) => panic!("expected catalogue query refusal"),
    };
    assert!(e.contains("product metadata query failed for D-1"));
}
#[test]
fn generated_repeated_text_is_bounded_before_object_copies() {
    let mut p = fixture();
    p.devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .text = value(Text::Literal("x".repeat(1024 * 1024)));
    let mut o = p
        .devices
        .com_object(ComObjectInstanceId(1))
        .unwrap()
        .clone();
    for n in 2..=7 {
        o.id = ComObjectInstanceId(n);
        p.devices.insert_com_object(o.clone());
        p.devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(o.id);
    }
    refuses(&p, "generated output limit");
}
