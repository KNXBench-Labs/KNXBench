//! Audits selective schema21 mapping without copying or changing file-level download evidence.

use super::*;

#[test]
fn native21_selective_mapping_keeps_destination_evidence_and_copied_devices_inherit_none() {
    use knx_core::commissioning::parameter_baseline::{
        BaselineChange, BaselineEdit, LinkBaseline, LinkBaselineChange, ParameterBaseline,
        ParameterBaselines,
    };
    use knx_store::project_history as history;
    let (source, mut target, selection) = fixture();
    let mut existing = source.project.devices.iter().next().unwrap().clone();
    existing.id = target.project.ids.next_device_id().unwrap();
    existing.source.ets_id = "destination-owned-device".into();
    existing.address = None;
    existing.com_objects.clear();
    let existing_id = existing.id;
    target.project.devices.insert(existing);
    target.project.installations[0]
        .topology
        .unassigned
        .push(existing_id);
    let mut held = ParameterBaselines::new();
    held.apply(&BaselineEdit {
        device: existing_id,
        parameter: BaselineChange::Record(ParameterBaseline {
            program_id: "P".into(),
            values: Default::default(),
            object_flags: Some(Default::default()),
            at: chrono::DateTime::UNIX_EPOCH,
        }),
        links: LinkBaselineChange::Record(LinkBaseline {
            program_id: "P".into(),
            links: vec![],
            at: chrono::DateTime::UNIX_EPOCH,
        }),
    });
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let mut stack = CommandStack::new();
    let generation =
        history::save_editor_with_baselines(&conn, &target, &stack, None, &[], &held).unwrap();
    let plan = import::plan(&target, &source, selection).unwrap();
    let copied_id = plan
        .preview
        .mappings
        .iter()
        .find(|mapping| mapping.kind == "device")
        .unwrap()
        .target;
    assert_ne!(copied_id, existing_id.0);
    import::apply(&mut target, &mut stack, plan).unwrap();
    history::save_editor_with_sources(&conn, &target, &stack, Some(generation), &[]).unwrap();
    let after = history::load_parameter_baselines(&conn).unwrap();
    assert_eq!(
        after, held,
        "schema21 adds no normalized model dependency to map"
    );
    assert!(after.get(DeviceId(copied_id)).is_none());
    assert!(after.get_links(DeviceId(copied_id)).is_none());
}
