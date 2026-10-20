//! Monitoring export identity, reference and current-value integrity regressions.
use super::*;

fn two_object_fixture() -> Project {
    let mut p = fixture();
    let mut second = p
        .devices
        .com_object(ComObjectInstanceId(1))
        .unwrap()
        .clone();
    second.id = ComObjectInstanceId(2);
    second.text = value(Text::Literal("Second distinct object".into()));
    p.devices.insert_com_object(second);
    p.devices
        .get_mut(DeviceId(1))
        .unwrap()
        .com_objects
        .push(ComObjectInstanceId(2));
    p
}

fn refuses_identity_without_mutation(p: &Project, needle: &str) {
    let before = p.clone();
    let preflight = knx_app::monitor_export::validate_request(p, &ExportOptions::default());
    let bundle = build(p, None, &ExportOptions::default());
    assert_eq!(*p, before, "identity refusal must not repair the input");
    assert_eq!(
        (preflight.is_err(), bundle.is_err()),
        (true, true),
        "validate_request and build must refuse corrupt record identity"
    );
    for error in [preflight.unwrap_err(), bundle.unwrap_err()] {
        assert!(error.contains(needle), "expected {needle}, got {error}");
    }
}

#[test]
fn distinct_actual_object_ids_export_two_records_with_closed_links() {
    let p = two_object_fixture();
    let before = p.clone();
    knx_app::monitor_export::validate_request(&p, &ExportOptions::default()).unwrap();
    let (v, r) = document(&p);
    assert_eq!(v["communication_objects"].as_object().unwrap().len(), 2);
    assert_eq!(
        v["communication_objects"]["1.1.9/O-2"]["text"],
        "Second distinct object"
    );
    for ids in [
        &v["devices"]["1.1.9"]["communication_object_ids"],
        &v["group_addresses"]["1/0/3"]["communication_object_ids"],
    ] {
        assert_eq!(*ids, json!(["1.1.9/O-1", "1.1.9/O-2"]));
    }
    for key in ["1.1.9/O-1", "1.1.9/O-2"] {
        assert_eq!(
            v["communication_objects"][key]["group_address_links"],
            json!(["1/0/3"])
        );
    }
    assert_eq!(
        (
            r.counts.communication_objects.selected,
            r.counts.communication_objects.exported,
            r.counts.communication_objects.omitted
        ),
        (2, 2, 0)
    );
    assert_eq!(p, before);
}

#[test]
fn duplicate_actual_object_ids_are_whole_refusals_without_mutation() {
    let mut p = two_object_fixture();
    p.devices.com_object_mut(ComObjectInstanceId(1)).unwrap().id = ComObjectInstanceId(2);
    refuses_identity_without_mutation(&p, "object ID reference mismatch");
}

#[test]
fn permuted_actual_object_ids_are_whole_refusals_without_mutation() {
    let mut p = two_object_fixture();
    p.devices.com_object_mut(ComObjectInstanceId(1)).unwrap().id = ComObjectInstanceId(2);
    p.devices.com_object_mut(ComObjectInstanceId(2)).unwrap().id = ComObjectInstanceId(1);
    refuses_identity_without_mutation(&p, "object ID reference mismatch");
}

#[test]
fn duplicate_actual_object_ids_cannot_hide_an_unreferenced_record() {
    let mut p = two_object_fixture();
    p.devices.get_mut(DeviceId(1)).unwrap().com_objects.pop();
    p.devices.com_object_mut(ComObjectInstanceId(2)).unwrap().id = ComObjectInstanceId(1);
    refuses_identity_without_mutation(&p, "duplicate actual object ID");
}

#[test]
fn permuted_actual_device_ids_are_whole_refusals_without_mutation() {
    let mut p = fixture();
    let mut second = p.devices.get(DeviceId(1)).unwrap().clone();
    second.id = DeviceId(2);
    second.address = Some(IndividualAddress::new(1, 1, 10).unwrap());
    second.com_objects.clear();
    p.devices.insert(second);
    p.installations[0].topology.unassigned.push(DeviceId(2));
    knx_app::monitor_export::validate_request(&p, &ExportOptions::default()).unwrap();
    p.devices.get_mut(DeviceId(1)).unwrap().id = DeviceId(2);
    p.devices.get_mut(DeviceId(2)).unwrap().id = DeviceId(1);
    p.devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .device = DeviceId(2);
    refuses_identity_without_mutation(&p, "device ID reference mismatch");
}

#[test]
fn all_six_stated_flags_export_as_actual_booleans() {
    let mut p = fixture();
    p.devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .flags = ResolvedFlags {
        read: value(false),
        write: value(true),
        communication: value(false),
        transmit: value(true),
        update: value(false),
        read_on_init: value(true),
    };
    let (v, r) = document(&p);
    assert_eq!(
        v["communication_objects"]["1.1.9/O-1"]["flags"],
        json!({"read":false,"write":true,"communication":false,"transmit":true,"update":false,"read_on_init":true})
    );
    assert!(!r
        .issues
        .iter()
        .any(|i| i.entity == "O-1" && i.field.starts_with("flags.")));
}
#[test]
fn malformed_raw_strings_and_source_identity_never_leak_into_the_bundle() {
    let mut p = fixture();
    let secret = "excluded-source-marker";
    let o = p.devices.com_object_mut(ComObjectInstanceId(1)).unwrap();
    o.flags.read = Override::Malformed(secret.into());
    o.dpt = Override::Malformed(secret.into());
    o.source.path = secret.into();
    o.source.ets_id = secret.into();
    p.installations[0].group_addresses[0].declared_dpt = Override::Malformed(secret.into());
    p.devices.get_mut(DeviceId(1)).unwrap().program_ref = secret.into();
    let b = build(&p, None, &ExportOptions::default()).unwrap();
    assert!(!String::from_utf8_lossy(&b.bytes).contains(secret));
}
#[test]
fn both_directions_on_same_ga_collapse_once_with_closed_reverse_links() {
    let mut p = fixture();
    p.devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .links
        .push(GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Receive,
        });
    let (v, r) = document(&p);
    assert_eq!(
        v["communication_objects"]["1.1.9/O-1"]["group_address_links"],
        json!(["1/0/3"])
    );
    assert_eq!(
        v["group_addresses"]["1/0/3"]["communication_object_ids"],
        json!(["1.1.9/O-1"])
    );
    assert!(has_issue(&r, "O-1", "group_address_links"));
}
#[test]
fn group_hierarchy_is_recursive_and_direct_membership_is_not_guessed() {
    let mut p = fixture();
    let mut child = p.installations[0].group_ranges[0].clone();
    child.id = GroupRangeId(2);
    child.parent = Some(GroupRangeId(1));
    child.start = GroupAddress::from_raw(2050);
    child.end = GroupAddress::from_raw(2055);
    p.installations[0].group_ranges[0].children.push(child.id);
    p.installations[0].group_ranges.push(child);
    p.installations[0].group_addresses[0].range = Some(GroupRangeId(2));
    let (v, _) = document(&p);
    assert_eq!(v["group_ranges"]["R-1"]["group_addresses"], json!([]));
    assert_eq!(
        v["group_ranges"]["R-1"]["group_ranges"]["R-2"]["group_addresses"],
        json!(["1/0/3"])
    );
}
#[test]
fn location_function_backlinks_use_stable_ids_not_names() {
    let (v, _) = document(&fixture());
    assert_eq!(v["locations"]["Room"]["identifier"], "S-1");
    assert_eq!(v["locations"]["Room"]["functions"], json!(["F-1"]));
    assert_eq!(v["functions"]["F-1"]["space_id"], "S-1");
    assert_eq!(
        v["devices"]["1.1.9"]["communication_object_ids"],
        json!(["1.1.9/O-1"])
    );
}
#[test]
fn duplicate_and_cross_owner_object_backlinks_are_whole_refusals() {
    let mut p = fixture();
    let mut d = p.devices.get(DeviceId(1)).unwrap().clone();
    d.id = DeviceId(2);
    d.address = Some(IndividualAddress::new(1, 1, 10).unwrap());
    p.devices.insert(d);
    p.installations[0].topology.unassigned.push(DeviceId(2));
    refuses(&p, "duplicate");
    let mut p = fixture();
    let mut d = p.devices.get(DeviceId(1)).unwrap().clone();
    d.id = DeviceId(2);
    d.address = Some(IndividualAddress::new(1, 1, 10).unwrap());
    d.com_objects.clear();
    p.devices.insert(d);
    p.installations[0].topology.unassigned.push(DeviceId(2));
    p.devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .device = DeviceId(2);
    refuses(&p, "owner reference");
}
#[test]
fn absent_ga_and_ko_types_stay_null_and_empty() {
    let mut p = fixture();
    p.installations[0].group_addresses[0].declared_dpt = Override::Absent;
    p.devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .dpt = Override::Absent;
    let (v, _) = document(&p);
    assert!(v["group_addresses"]["1/0/3"]["dpt"].is_null());
    assert_eq!(v["communication_objects"]["1.1.9/O-1"]["dpts"], json!([]));
}
#[test]
fn issue_order_is_deterministic_and_timestamp_comes_from_current_model() {
    let mut p = fixture();
    p.info.last_modified = Some("2024-01-02T03:04:05Z".parse().unwrap());
    let (v, r) = document(&p);
    assert_eq!(v["info"]["last_modified"], "2024-01-02T03:04:05+00:00");
    assert!(r.issues.windows(2).all(|pair| (
        &pair[0].entity,
        &pair[0].field,
        &pair[0].code,
        &pair[0].detail
    ) <= (
        &pair[1].entity,
        &pair[1].field,
        &pair[1].code,
        &pair[1].detail
    )));
}
