//! Each download settlement row of ADR-0173 §3 through the real settler, and its comparison.

use knx_core::commissioning::parameter_baseline::{BaselineEdit, ParameterBaseline};

use super::project_edits::{edit_time, install_rich_project, TIME_EDITED, TIME_ID, TIME_ORIGINAL};
use super::*;
use crate::domain;
use crate::parameter_baseline::{DifferingParameterDto, LastDownloadState};

fn baseline(state: &SharedState) -> Option<ParameterBaseline> {
    state
        .parameter_baselines
        .lock()
        .unwrap()
        .get(DeviceId(1))
        .cloned()
}

/// A baseline an earlier run left, so that keeping and removing differ.
fn seed(state: &SharedState) -> ParameterBaseline {
    let earlier = ParameterBaseline {
        object_flags: None,
        program_id: PROGRAM_ID.into(),
        values: [(TIME_ID.to_string(), "1".to_string())].into(),
        at: chrono::DateTime::UNIX_EPOCH,
    };
    state
        .parameter_baselines
        .lock()
        .unwrap()
        .apply(&BaselineEdit {
            device: DeviceId(1),
            parameter: knx_core::commissioning::parameter_baseline::BaselineChange::Record(
                earlier.clone(),
            ),
            links: knx_core::commissioning::parameter_baseline::LinkBaselineChange::Keep,
        });
    earlier
}

fn recorded(time: &str) -> std::collections::BTreeMap<String, String> {
    [(TIME_ID.to_string(), time.to_string())].into()
}

fn failed_at(written: Written, step: Option<usize>) -> DownloadStatus {
    DownloadStatus::Failed {
        written,
        stopped_in_step: step,
        error: "lost".into(),
        hint: None,
        failure: crate::device_download::synthetic_failure(step),
    }
}

fn panel(state: &SharedState) -> crate::parameter_baseline::LastDownloadDto {
    domain::parameter_panel_impl(state, 1, None)
        .expect("panel")
        .last_download
        .expect("a GET carries the comparison")
}

#[test]
fn a_completed_run_from_an_unchanged_source_records_the_resolved_written_values() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    seed(&f.state);
    assert_eq!(full(&f, false), ProgrammingParts::ALL);
    let recorded_baseline = baseline(&f.state).expect("recorded");
    assert_eq!(recorded_baseline.program_id, PROGRAM_ID);
    assert_eq!(recorded_baseline.values, recorded(TIME_ORIGINAL));
    let view = panel(&f.state);
    assert_eq!(view.state, LastDownloadState::Usable);
    assert_eq!(view.differing, Some(vec![]));
    assert!(view.at.is_some());

    edit_time(&f.state);
    assert_eq!(
        panel(&f.state).differing,
        Some(vec![DifferingParameterDto {
            ref_id: TIME_ID.into(),
            downloaded: Some(TIME_ORIGINAL.into()),
            current: Some(TIME_EDITED.into()),
        }])
    );
}

#[test]
fn a_default_resolves_on_both_sides_and_is_recorded_without_a_chosen_value() {
    // The bare project chooses no value: the product default is what ran.
    let f = fixture(&OLD, &OLD);
    full(&f, false);
    assert_eq!(baseline(&f.state).unwrap().values, recorded("50"));
    assert_eq!(panel(&f.state).differing, Some(vec![]));
}

#[test]
fn a_parameter_edit_during_the_run_records_what_was_written_and_shows_as_differing() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    let prepared = prepare(&f.state);
    let settler = download_settler(&f.state, super::super::incarnation(&f.state), &prepared);
    edit_time(&f.state);
    settler(&finished());
    assert!(!belief(&f.state).parameters, "the edit stays pending");
    assert_eq!(baseline(&f.state).unwrap().values, recorded(TIME_ORIGINAL));
    let differing = panel(&f.state).differing.unwrap();
    assert_eq!(differing.len(), 1);
    assert_eq!(differing[0].current.as_deref(), Some(TIME_EDITED));
}

#[test]
fn a_settlement_whose_file_write_fails_leaves_the_held_baseline_alone() {
    let f = fixture(&OLD, &OLD);
    let earlier = seed(&f.state);
    // The owned store path cannot be opened: its directory does not exist.
    *f.state.store_path.lock().unwrap() = Some(f._dir.path().join("missing/dir/project.knxdb"));
    // A completed run from an unchanged source would record another one.
    run(&f, None, false, finished());
    assert_eq!(belief(&f.state), NONE, "nothing was applied");
    assert_eq!(baseline(&f.state), Some(earlier), "nor the baseline record");
}

#[test]
fn a_changed_source_removes_the_baseline() {
    let f = fixture(&OLD, &new_base());
    seed(&f.state);
    assert_eq!(full(&f, true), ADDRESS);
    assert_eq!(baseline(&f.state), None);
    assert_eq!(panel(&f.state).state, LastDownloadState::Absent);
}

#[test]
fn an_interrupted_run_that_attempted_the_parameters_removes_the_baseline() {
    let f = fixture(&OLD, &OLD);
    seed(&f.state);
    run(&f, None, false, failed_at(Written::Partially, None));
    assert_eq!(baseline(&f.state), None);
}

#[test]
fn an_interrupted_run_that_never_reached_the_parameters_keeps_the_baseline() {
    let f = fixture(&OLD, &OLD);
    let earlier = seed(&f.state);
    // Step 1 is the connect: nothing of the parameters was attempted.
    run(&f, None, false, failed_at(Written::Partially, Some(1)));
    assert_eq!(baseline(&f.state), Some(earlier));
}

#[test]
fn a_run_without_parameters_in_scope_or_without_a_write_keeps_the_baseline() {
    let f = fixture(&OLD, &OLD);
    let earlier = seed(&f.state);
    assert_eq!(partial(&f, false, true, false), GROUP_ADDRESSES);
    assert_eq!(baseline(&f.state), Some(earlier.clone()), "tables only");
    run(&f, None, false, failed_at(Written::No, None));
    assert_eq!(baseline(&f.state), Some(earlier), "nothing written");
}

#[test]
fn a_partial_parameter_run_records_like_a_complete_one() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    assert_eq!(partial(&f, true, false, false), PARAMETERS);
    assert_eq!(baseline(&f.state).unwrap().values, recorded(TIME_ORIGINAL));
}

#[test]
fn an_individual_address_programming_keeps_the_baseline() {
    let f = fixture(&OLD, &OLD);
    let earlier = seed(&f.state);
    let settler = super::super::address_programming_settler(&f.state, address());
    settler(
        &crate::address_programming::AddressProgrammingStatus::Finished {
            written: crate::address_programming::AddressWritten::Yes,
            previous_address: "15.15.255".into(),
            was_free: true,
            restart_confirmed: false,
        },
    );
    assert!(belief(&f.state).address, "the settler ran");
    assert_eq!(baseline(&f.state), Some(earlier));
}

#[test]
fn the_comparison_names_another_program_and_missing_product_data() {
    let f = fixture(&OLD, &OLD);
    full(&f, false);
    // The device names another program now: the record does not apply.
    let mut other = baseline(&f.state).unwrap();
    other.program_id = "M-0001_A-0002-01-0000".into();
    f.state
        .parameter_baselines
        .lock()
        .unwrap()
        .apply(&BaselineEdit {
            device: DeviceId(1),
            parameter: knx_core::commissioning::parameter_baseline::BaselineChange::Record(other),
            links: knx_core::commissioning::parameter_baseline::LinkBaselineChange::Keep,
        });
    assert_eq!(panel(&f.state).state, LastDownloadState::OtherProgram);
    // A program reference the catalogue does not answer.
    full(&f, false);
    f.state
        .project
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .devices
        .get_mut(DeviceId(1))
        .unwrap()
        .program_ref = "M-0001_H-9_HP-9".into();
    let view = panel(&f.state);
    assert_eq!(view.state, LastDownloadState::ProductUnavailable);
    assert_eq!((view.at, view.differing), (None, None));
}

#[test]
fn a_settled_baseline_is_written_with_the_flags_and_reopens() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    let path = f._dir.path().join("project.knxdb");
    domain::save_project_as(&f.state, &path).expect("saves");
    full(&f, false);
    let recorded_baseline = baseline(&f.state).unwrap();

    // No explicit save: settlement wrote the baseline into the file.
    let reopened = Arc::new(AppState::default());
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
    domain::open_native_project(&reopened, &path, true, &progress).expect("reopens");
    assert_eq!(baseline(&reopened), Some(recorded_baseline.clone()));
    assert_eq!(belief(&reopened), ProgrammingParts::ALL);

    // Save As to a new file copies it like the project.
    let copy = f._dir.path().join("copy.knxdb");
    domain::save_project_as(&f.state, &copy).expect("saves as");
    let copied = Arc::new(AppState::default());
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &copy);
    domain::open_native_project(&copied, &copy, true, &progress).expect("reopens");
    assert_eq!(baseline(&copied), Some(recorded_baseline));
}

#[test]
fn a_plain_save_leaves_the_file_baselines_as_settlement_wrote_them() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    let path = f._dir.path().join("project.knxdb");
    domain::save_project_as(&f.state, &path).expect("saves");
    full(&f, false);
    let recorded_baseline = baseline(&f.state).unwrap();
    // An unsaved edit, so the plain save writes a new root.
    edit_time(&f.state);
    domain::save_project(&f.state).expect("saves");
    assert_eq!(baseline(&f.state), Some(recorded_baseline.clone()));
    let reopened = Arc::new(AppState::default());
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
    domain::open_native_project(&reopened, &path, true, &progress).expect("reopens");
    assert_eq!(baseline(&reopened), Some(recorded_baseline));
    assert_eq!(reopened.parameter_baselines.lock().unwrap().len(), 1);
}

#[test]
fn a_session_only_project_keeps_its_baseline_until_the_first_native_save() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    full(&f, false);
    let held = baseline(&f.state).expect("held in memory without a file");
    let path = f._dir.path().join("first.knxdb");
    domain::save_project_as(&f.state, &path).expect("first save");
    let reopened = Arc::new(AppState::default());
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
    domain::open_native_project(&reopened, &path, true, &progress).expect("reopens");
    assert_eq!(baseline(&reopened), Some(held));
}

#[test]
fn a_deleted_device_keeps_its_baseline_until_a_save_prunes_it() {
    let f = fixture(&OLD, &OLD);
    // A second, data-free device the model lets the user delete.
    let mut project = super::project_edits::rich_project();
    project.devices.insert(DeviceInstance {
        id: DeviceId(2),
        source: SourceRef {
            path: "test".into(),
            ets_id: "DEV-2".into(),
        },
        name: "Second".into(),
        description: None,
        address: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project.installations[0]
        .topology
        .unassigned
        .push(DeviceId(2));
    project.ids.next_device_id().unwrap();
    *f.state.project.lock().unwrap() = Some(project);
    let second = |state: &SharedState| {
        state
            .parameter_baselines
            .lock()
            .unwrap()
            .get(DeviceId(2))
            .cloned()
    };
    let earlier = ParameterBaseline {
        object_flags: None,
        program_id: PROGRAM_ID.into(),
        values: Default::default(),
        at: chrono::DateTime::UNIX_EPOCH,
    };
    f.state
        .parameter_baselines
        .lock()
        .unwrap()
        .apply(&BaselineEdit {
            device: DeviceId(2),
            parameter: knx_core::commissioning::parameter_baseline::BaselineChange::Record(
                earlier.clone(),
            ),
            links: knx_core::commissioning::parameter_baseline::LinkBaselineChange::Keep,
        });
    let path = f._dir.path().join("project.knxdb");
    domain::save_project_as(&f.state, &path).expect("saves");
    domain::delete_device_impl(&f.state, 2).expect("deletes");
    assert_eq!(
        second(&f.state),
        Some(earlier),
        "an undo could still bring it back"
    );
    domain::save_project(&f.state).expect("saves");
    assert_eq!(second(&f.state), None, "pruned in memory");
    let reopened = Arc::new(AppState::default());
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
    domain::open_native_project(&reopened, &path, true, &progress).expect("reopens");
    assert_eq!(second(&reopened), None, "pruned in the file");
    // An undo after that save brings the device back with an unknown baseline.
    domain::undo_impl(&f.state).expect("undo of the deletion");
    let view = domain::parameter_panel_impl(&f.state, 2, None)
        .expect("panel")
        .last_download
        .expect("a GET carries the comparison");
    assert_eq!(view.state, LastDownloadState::Absent);
}

#[test]
fn another_project_never_inherits_a_baseline() {
    let f = fixture(&OLD, &OLD);
    full(&f, false);
    assert!(baseline(&f.state).is_some());
    domain::new_project_impl(&f.state, None, None, None, None, true).expect("new project");
    assert!(f.state.parameter_baselines.lock().unwrap().is_empty());
}
