//! Borrowed monitoring export preflight and snapshot admission tests.
use super::*;

#[test]
fn borrowed_snapshot_admission_refuses_before_adapter_clone() {
    let mut p = fixture();
    knx_app::monitor_export::admit_snapshot(&p).unwrap();
    p.info.name = "x".repeat(33 * 1024 * 1024);
    let error = knx_app::monitor_export::admit_snapshot(&p).unwrap_err();
    assert!(error.contains("input limit"));
}

#[test]
fn preflight_admits_without_mutation_and_rejects_excluded_heap_text() {
    let mut p = fixture();
    let before = p.clone();
    knx_app::monitor_export::validate_request(&p, &ExportOptions::default()).unwrap();
    assert_eq!(p, before);
    let note = knx_core::note::ProjectNote::new(
        ProjectNoteId(1),
        knx_core::note::ProjectNoteTarget::Project,
        "Note".into(),
        "x".repeat(64 * 1024),
    )
    .unwrap();
    p.notes = vec![note; 513];
    let error =
        knx_app::monitor_export::validate_request(&p, &ExportOptions::default()).unwrap_err();
    assert!(error.contains("input limit"));
}

#[test]
fn clone_preflight_also_bounds_private_orphan_defaults_not_read_by_projection() {
    let mut p = fixture();
    p.devices.set_program_defaults(
        ComObjectInstanceId(999),
        ProgramDefaults {
            text: Some(Resolved {
                value: Text::Literal("x".repeat(33 * 1024 * 1024)),
                layer: Layer::Program,
            }),
            ..Default::default()
        },
    );
    let e = knx_app::monitor_export::validate_request(&p, &ExportOptions::default()).unwrap_err();
    assert!(e.contains("snapshot clone input limit"));
}
