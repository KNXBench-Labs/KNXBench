//! Monitoring export resource admission and bounded output tests.
use super::*;

#[test]
fn entity_budget_is_checked_before_duplicate_map_construction() {
    let mut p = fixture();
    let ga = p.installations[0].group_addresses[0].clone();
    p.installations[0].group_addresses = vec![ga; 100001];
    refuses(&p, "entity limit");
}
#[test]
fn cumulative_text_limit_rejects_before_project_copy() {
    let mut p = fixture();
    p.info.name = "x".repeat(17 * 1024 * 1024);
    p.installations[0].name = "y".repeat(16 * 1024 * 1024);
    refuses(&p, "input limit");
}
fn hierarchy(depth: u32) -> Project {
    let mut p = fixture();
    p.functions.clear();
    p.installations[0].buildings.clear();
    for n in 1..=depth {
        p.installations[0].buildings.push(BuildingPart {
            id: BuildingPartId(n),
            source: source(),
            name: format!("Room {n}"),
            number: None,
            kind: BuildingPartType::Room,
            default_line: None,
            completion: CompletionStatus::default(),
            children: if n < depth {
                vec![BuildingPartId(n + 1)]
            } else {
                vec![]
            },
            devices: vec![],
            parent: if n == 1 {
                None
            } else {
                Some(BuildingPartId(n - 1))
            },
        });
    }
    p
}
#[test]
fn hierarchy_depth_64_is_admitted_and_65_refused() {
    let p = hierarchy(64);
    let b = build(&p, None, &ExportOptions::default()).expect("64 hierarchy levels are admitted");
    assert_eq!(b.report.counts.spaces.exported, 64);
    refuses(&hierarchy(65), "depth limit");
}
#[test]
fn malformed_flags_missing_size_and_missing_translation_have_diagnostics() {
    let mut p = fixture();
    let o = p.devices.com_object_mut(ComObjectInstanceId(1)).unwrap();
    o.flags.read = Override::Malformed("Enabled?".into());
    o.size = None;
    o.text = value(Text::Localized(LocalizedString(TranslationKey(
        "not-present".into(),
    ))));
    let (v, r) = document(&p);
    assert!(v["communication_objects"]["1.1.9/O-1"]["flags"]
        .get("read")
        .is_none());
    assert_eq!(v["communication_objects"]["1.1.9/O-1"]["object_size"], "");
    assert_eq!(v["communication_objects"]["1.1.9/O-1"]["text"], "");
    for field in ["flags.read", "object_size", "text"] {
        assert!(has_issue(&r, "O-1", field));
    }
}
