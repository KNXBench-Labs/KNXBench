//! A manual mark on a background-journaling server: durable when it forgets a baseline.

use std::sync::mpsc;
use std::time::Duration;

use knx_core::commissioning::parameter_baseline::{BaselineEdit, ParameterBaseline};
use knx_core::{DeviceId, DeviceInstance, SourceRef};

use super::background_with;
use crate::domain::{self, SnapshotBinding};
use crate::journal_writer::JournalStatus;
use crate::AppState;
use knx_store::project_history as history;

const DEVICE: DeviceId = DeviceId(1);

fn baseline() -> ParameterBaseline {
    ParameterBaseline {
        object_flags: None,
        program_id: "M-0001_A-0001-01-0000".into(),
        values: [("M-0001_A-0001-01-0000_P-1_R-1".to_string(), "7".to_string())].into(),
        at: chrono::DateTime::UNIX_EPOCH,
    }
}

/// Adds device 1 to the new project and, with `seeded`, a baseline for it,
/// before the fixture's Save As writes both.
fn with_device(seeded: bool) -> impl FnOnce(&AppState) {
    move |state| {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        let id = project.ids.next_device_id().unwrap();
        assert_eq!(id, DEVICE);
        project.devices.insert(DeviceInstance {
            id,
            source: SourceRef {
                path: "test".into(),
                ets_id: "DEV-1".into(),
            },
            name: "Marked".into(),
            description: None,
            address: None,
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        project.installations[0].topology.unassigned.push(id);
        if seeded {
            state
                .parameter_baselines
                .lock()
                .unwrap()
                .apply(&BaselineEdit {
                    device: id,
                    parameter: knx_core::commissioning::parameter_baseline::BaselineChange::Record(
                        baseline(),
                    ),
                    links: knx_core::commissioning::parameter_baseline::LinkBaselineChange::Keep,
                });
        }
    }
}

fn binding(state: &AppState) -> SnapshotBinding {
    SnapshotBinding {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: crate::programming_state::incarnation(state),
        snapshot_revision: domain::current_project_revision(state),
    }
}

fn stored_baseline(path: &std::path::Path) -> Option<ParameterBaseline> {
    let store = knx_store::open_existing_read_only(path).unwrap();
    history::load_parameter_baselines(&store.conn)
        .unwrap()
        .get(DEVICE)
        .cloned()
}

fn stored_marked(path: &std::path::Path) -> bool {
    let conn = knx_store::open_existing_and_migrate(path).unwrap();
    let working = history::load_editor(&conn).unwrap().unwrap().working;
    working
        .project
        .devices
        .get(DEVICE)
        .unwrap()
        .commissioning
        .pending_parts()
        .is_empty()
}

#[test]
fn a_mark_that_forgets_a_baseline_is_in_the_file_before_it_answers() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, _) = background_with(&dir, with_device(true));
    assert_eq!(stored_baseline(&path), Some(baseline()), "Save As wrote it");
    let writes_before = super::writes(&state);

    crate::programming_state::mark_device(&state, &binding(&state), DEVICE, true).unwrap();

    // No flush: the mark's journal write and the removal were one
    // synchronous transaction, not a job for the background writer.
    assert_eq!(stored_baseline(&path), None, "removed with the mark");
    assert!(stored_marked(&path), "the flags in the same write");
    assert_eq!(super::writes(&state), writes_before + 1);
    assert!(state.parameter_baselines.lock().unwrap().is_empty());
    // Whatever the background writer still does, a restarted server reopens
    // the file without the baseline.
    state.journal().flush();
    drop(state);
    let reopened = AppState::default();
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
    domain::open_native_project(&reopened, &path, true, &progress).unwrap();
    assert!(reopened.parameter_baselines.lock().unwrap().is_empty());
    assert_eq!(stored_baseline(&path), None);
}

#[test]
fn a_mark_of_a_device_without_a_baseline_keeps_the_background_path() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, _) = background_with(&dir, with_device(false));
    let state = std::sync::Arc::new(state);
    // Hold the generation: a synchronous journal write would wait for it,
    // a background one is answered first (ADR-0166).
    let held = state.history_generation.value.lock().unwrap();
    let (answered, answer) = mpsc::channel();
    let marking = state.clone();
    let worker = std::thread::spawn(move || {
        let binding = binding(&marking);
        let result = crate::programming_state::mark_device(&marking, &binding, DEVICE, true);
        answered.send(result.is_ok()).unwrap();
    });
    let reply = answer.recv_timeout(Duration::from_secs(20));
    let status = state.journal().status();
    drop(held);
    worker.join().unwrap();
    assert_eq!(
        reply,
        Ok(true),
        "answered while the generation was held: no synchronous write"
    );
    assert_eq!(status, JournalStatus::Saving);
    state.journal().flush();
    assert!(stored_marked(&path), "the background writer wrote the mark");
    assert_eq!(stored_baseline(&path), None);
}

#[test]
fn a_link_only_owner_forces_synchronous_manual_mark_journaling_in_both_directions() {
    use knx_core::commissioning::parameter_baseline::{
        BaselineChange, LinkBaseline, LinkBaselineChange,
    };
    for programmed in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let (state, path, _) = background_with(&dir, |state| {
            with_device(false)(state);
            state
                .parameter_baselines
                .lock()
                .unwrap()
                .apply(&BaselineEdit {
                    device: DEVICE,
                    parameter: BaselineChange::Keep,
                    links: LinkBaselineChange::Record(LinkBaseline {
                        program_id: "P".into(),
                        links: vec![],
                        at: chrono::DateTime::UNIX_EPOCH,
                    }),
                });
        });
        let writes_before = super::writes(&state);
        crate::programming_state::mark_device(&state, &binding(&state), DEVICE, programmed)
            .unwrap();
        let read = knx_store::open_existing_read_only(&path).unwrap();
        assert!(
            history::load_parameter_baselines(&read.conn)
                .unwrap()
                .is_empty(),
            "link-only removal must be durable before answering"
        );
        assert_eq!(super::writes(&state), writes_before + 1);
        assert!(state.parameter_baselines.lock().unwrap().is_empty());
    }
}
