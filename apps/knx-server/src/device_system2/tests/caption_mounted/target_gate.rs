//! Mounted caption replacement distinguishes command-bound target changes from revision-only edits.

use super::*;
use crate::device_system2::{
    dto::Metadata,
    settlement::{self, Receipt, Run},
};
use knx_app::{
    property_validation::{DownloadResult, OperationProgress},
    property_validation_bundle::PreparedBundle,
};
use knx_store::project_history::NativeSnapshot;

#[derive(Clone, Copy, Debug)]
enum Change {
    Address,
    RevisionOnly,
}
#[derive(Clone, Copy, Debug)]
struct Case {
    namespace: u8,
    retained: bool,
    marked: bool,
    change: Change,
}
impl Case {
    fn address(self) -> bool {
        matches!(self.change, Change::Address)
    }

    fn receipt(self) -> Receipt {
        if self.address() {
            Receipt::DeviceMissing
        } else {
            Receipt::SessionOnly
        }
    }

    fn seed_steps(self) -> usize {
        if self.marked {
            3
        } else {
            2
        }
    }

    fn edited_flags(self) -> [bool; 5] {
        [
            !self.address() && self.marked,
            self.marked,
            self.marked,
            self.marked,
            true,
        ]
    }
}

// Includes commissioning timestamps, complete source context, comparison revisions and log times.
#[derive(Debug, PartialEq)]
struct Editor {
    snapshot: NativeSnapshot,
    stack: knx_core::CommandStack,
    undo: Vec<knx_core::Project>,
    redo: Vec<knx_core::Project>,
    revision: u64,
    generation: i64,
    server: String,
    incarnation: u64,
    saved_at: Option<String>,
    store_path: Option<PathBuf>,
    settlement_log: Vec<Value>,
}
impl Editor {
    fn read(state: &crate::SharedState) -> Self {
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let stack = state.command_stack.lock().unwrap();
        let (undo, redo) = stack.snapshot_states(project).unwrap();
        let settlement_log = state
            .session_log
            .lock()
            .unwrap()
            .entries()
            .iter()
            .filter(|entry| entry.source == "System2Download")
            .map(|entry| serde_json::to_value(entry).unwrap())
            .collect();
        Self {
            snapshot: crate::project_history::snapshot(state, project),
            stack: stack.clone(),
            undo,
            redo,
            revision: crate::domain::current_project_revision(state),
            generation: *state.history_generation.lock().unwrap(),
            server: state.server_incarnation.clone(),
            incarnation: state.project_incarnation.load(Ordering::SeqCst),
            saved_at: state.last_saved_at.lock().unwrap().clone(),
            store_path: state.store_path.lock().unwrap().clone(),
            settlement_log,
        }
    }
}

fn apply(state: &crate::SharedState, command: knx_core::Command, case: Case) {
    let binding = crate::domain::SnapshotBinding {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::SeqCst),
        snapshot_revision: crate::domain::current_project_revision(state),
    };
    crate::domain::apply_bound_prepared(state, &binding, "Caption target fixture", command)
        .unwrap_or_else(|error| panic!("{case:?}: real bound command: {error:?}"));
    assert_eq!(
        crate::domain::current_project_revision(state),
        binding.snapshot_revision + 1,
        "{case:?}: command really advanced the project revision"
    );
}
fn rename(state: &crate::SharedState, name: &str, case: Case) {
    apply(
        state,
        knx_core::Command::RenameDevice {
            device: f::DEVICE,
            name: name.into(),
        },
        case,
    );
}
fn seed(state: &crate::SharedState, case: Case) {
    // Before admission only: real manual marks, never a substitute for worker settlement.
    for programmed in [true, false].into_iter().chain(case.marked.then_some(true)) {
        apply(
            state,
            knx_core::Command::SetDeviceProgramming {
                device: f::DEVICE,
                change: ProgrammingChange::mark(ProgrammingParts::ALL, programmed),
            },
            case,
        );
    }
    assert_eq!(
        flags(state),
        [case.marked, case.marked, case.marked, case.marked, true]
    );
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (case.seed_steps(), 0)
    );
}
fn edit_and_seed_redo(state: &crate::SharedState, admitted: &Editor, case: Case) -> Editor {
    if case.address() {
        apply(
            state,
            knx_core::Command::SetIndividualAddress {
                device: f::DEVICE,
                address: Some("1.1.26".parse().unwrap()),
            },
            case,
        );
    } else {
        rename(state, "Current revision caption", case);
    }
    let edited = Editor::read(state);
    rename(state, "Redo-only caption", case);
    crate::domain::undo_impl(state).unwrap();
    let before = Editor::read(state);
    assert_eq!(
        before.snapshot, edited.snapshot,
        "{case:?}: real undo of benign rename"
    );
    assert_eq!(
        before.revision,
        admitted.revision + 3,
        "{case:?}: edit, rename, undo"
    );
    assert_eq!(
        before.stack.history_lengths(),
        (case.seed_steps() + 1, 1),
        "{case:?}"
    );
    assert_eq!(before.undo.len(), case.seed_steps() + 1, "{case:?}");
    assert_eq!(before.redo.len(), 1, "{case:?}");
    assert_eq!(
        flags(state),
        case.edited_flags(),
        "{case:?}: address command clears address belief"
    );
    assert!(
        before.settlement_log.is_empty(),
        "{case:?}: worker still paused"
    );
    assert!(before.store_path.is_none(), "{case:?}: unsaved matrix only");
    assert!(
        before
            .undo
            .iter()
            .any(|p| project_flags(p)[..4].iter().any(|claim| *claim)),
        "{case:?}: real old manual-mark claims must make sanitization observable"
    );
    let expected_address = if case.address() { "1.1.26" } else { TARGET };
    let device = before.snapshot.project.devices.get(f::DEVICE).unwrap();
    assert_eq!(
        device.address.unwrap().to_string(),
        expected_address,
        "{case:?}"
    );
    if !case.address() {
        assert_eq!(device.name, "Current revision caption", "{case:?}");
        assert_eq!(
            before.stack.comparison_revision(f::DEVICE),
            admitted.stack.comparison_revision(f::DEVICE),
            "{case:?}: a real name edit changes revision, not programming content"
        );
    } else {
        assert_eq!(
            before.stack.comparison_revision(f::DEVICE),
            admitted.stack.comparison_revision(f::DEVICE) + 1,
            "{case:?}: real address content edit"
        );
    }
    before
}
fn project_flags(project: &knx_core::Project) -> [bool; 5] {
    let c = &project.devices.get(f::DEVICE).unwrap().commissioning;
    [
        c.individual_address_loaded,
        c.application_program_loaded,
        c.parameters_loaded,
        c.communication_part_loaded,
        c.medium_config_loaded,
    ]
}

// Comparison projection on a clone ONLY: retain all configuration, names and other bookkeeping.
fn without_settled_belief(project: &knx_core::Project) -> knx_core::Project {
    let mut projection = project.clone();
    let c = &mut projection.devices.get_mut(f::DEVICE).unwrap().commissioning;
    c.individual_address_loaded = false;
    c.application_program_loaded = false;
    c.parameters_loaded = false;
    c.communication_part_loaded = false;
    c.last_download = None;
    projection
}
fn assert_positive(before: &Editor, after: &Editor, case: Case) {
    assert_eq!(
        project_flags(&after.snapshot.project),
        [true; 5],
        "{case:?}: all four credited"
    );
    let c = &after
        .snapshot
        .project
        .devices
        .get(f::DEVICE)
        .unwrap()
        .commissioning;
    assert!(
        c.last_download.is_some(),
        "{case:?}: genuine download timestamp"
    );
    assert_ne!(
        c.last_download,
        before
            .snapshot
            .project
            .devices
            .get(f::DEVICE)
            .unwrap()
            .commissioning
            .last_download
    );
    assert_eq!(
        without_settled_belief(&after.snapshot.project),
        without_settled_belief(&before.snapshot.project),
        "{case:?}: configuration, current name, medium and other timestamps unchanged"
    );
    assert_eq!(after.snapshot.opaque, before.snapshot.opaque, "{case:?}");
    assert_eq!(
        after.snapshot.manufacturer_refs, before.snapshot.manufacturer_refs,
        "{case:?}"
    );
    assert_eq!(
        after.revision,
        before.revision + 1,
        "{case:?}: exactly one terminal revision"
    );
    assert_eq!(
        after.generation, before.generation,
        "{case:?}: no saved history generation"
    );
    assert_eq!(after.server, before.server, "{case:?}");
    assert_eq!(after.incarnation, before.incarnation, "{case:?}");
    assert_eq!(after.saved_at, before.saved_at, "{case:?}");
    assert_eq!(after.store_path, before.store_path, "{case:?}");
    assert_eq!(
        after.stack.history_lengths(),
        before.stack.history_lengths(),
        "{case:?}: no new undo step"
    );
    assert_eq!(
        after.stack.comparison_revision(f::DEVICE),
        before.stack.comparison_revision(f::DEVICE) + 1,
        "{case:?}: touched claims forgotten once"
    );
    assert_eq!(after.undo.len(), before.undo.len(), "{case:?}");
    assert_eq!(after.redo.len(), before.redo.len(), "{case:?}");
    for (new, old) in after
        .undo
        .iter()
        .zip(&before.undo)
        .chain(after.redo.iter().zip(&before.redo))
    {
        assert_eq!(
            without_settled_belief(new),
            without_settled_belief(old),
            "{case:?}: exact history configuration/name projection"
        );
        assert!(project_flags(new)[4], "{case:?}: medium history preserved");
    }
    // The last undo and sole redo are name-only inverses: newly earned live credit survives.
    assert_eq!(
        project_flags(after.undo.last().unwrap()),
        [true; 5],
        "{case:?}: name-only undo keeps NEW credit"
    );
    assert_eq!(
        project_flags(after.redo.last().unwrap()),
        [true; 5],
        "{case:?}: name-only redo keeps NEW credit"
    );
    // Earlier entries are precisely the real pre-admission manual marks, not name snapshots.
    for project in &after.undo[..case.seed_steps()] {
        assert_eq!(
            project_flags(project),
            [false, false, false, false, true],
            "{case:?}: old explicit manual claims sanitized"
        );
    }
    assert_eq!(
        after.settlement_log.len(),
        1,
        "{case:?}: exactly one System2Download entry"
    );
    assert_eq!(after.settlement_log[0]["severity"], "info", "{case:?}");
    assert_eq!(
        after.settlement_log[0]["source"], "System2Download",
        "{case:?}"
    );
}
fn record_metadata(state: &crate::SharedState, running: bool, case: Case) -> Metadata {
    let inner = state.device_system2.inner.lock().unwrap();
    let record = inner.current.as_ref().unwrap();
    assert_eq!(
        record.control_id, record.metadata.operation_id,
        "{case:?}: download, not recovery"
    );
    if running {
        assert!(
            record.task.as_ref().is_some_and(|t| !t.is_finished()),
            "{case:?}: same admitted worker is paused"
        );
    } else {
        assert!(
            record.task.is_none(),
            "{case:?}: worker joined and journal ownership released"
        );
        assert_eq!(
            record.settlement,
            case.receipt(),
            "{case:?}: actual worker receipt"
        );
    }
    record.metadata.clone()
}
fn journal(
    state: &crate::SharedState,
    operation: &Path,
    original: &PreparedBundle,
    case: Case,
) -> OperationProgress {
    record_metadata(state, false, case);
    // Never open the journal while the actual task owns it; this is not a project/service reopen.
    let store = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
    let progress = store.progress();
    assert_eq!(store.outcome(), LocalOutcome::Finished, "{case:?}");
    assert_eq!(progress.outcome, LocalOutcome::Finished, "{case:?}");
    assert!(progress.mutation_possible, "{case:?}");
    assert!(!progress.restore_attempted, "{case:?}");
    assert_eq!(
        progress.download.outcome,
        LocalOutcome::Finished,
        "{case:?}"
    );
    assert_eq!(
        progress.download.result,
        DownloadResult::Verified,
        "{case:?}"
    );
    assert!(progress.download.completed_steps > 0, "{case:?}");
    assert_eq!(
        Some(progress.download.completed_steps),
        progress.download.total_steps,
        "{case:?}"
    );
    assert_eq!(
        progress.completed_steps, progress.download.completed_steps,
        "{case:?}"
    );
    assert_eq!(
        progress.total_steps, progress.download.total_steps,
        "{case:?}"
    );
    assert_eq!(
        store.recovery().unwrap().original_bundle,
        *original,
        "{case:?}: original recovery binding"
    );
    let inner = state.device_system2.inner.lock().unwrap();
    let record = inner.current.as_ref().unwrap();
    assert_eq!(
        record.progress.download, progress.download,
        "{case:?}: live Record and journal agree"
    );
    assert_eq!(record.progress.outcome, progress.outcome, "{case:?}");
    assert!(!record.progress.restore_attempted, "{case:?}");
    progress
}
async fn observe(
    state: &crate::SharedState,
    router: &Router,
    meta: &Metadata,
    progress: &OperationProgress,
    terminal: &Editor,
    case: Case,
) {
    let download = progress.download;
    for _ in 0..2 {
        let status = http(router, "/api/device-system2/status", None, 200).await;
        assert_eq!(status["operationId"], meta.operation_id, "{case:?}");
        assert_eq!(
            status["binding"],
            serde_json::to_value(&meta.binding).unwrap(),
            "{case:?}: historical admission revision"
        );
        assert_eq!(status["address"], meta.address, "{case:?}: original target");
        assert_eq!(
            status["deviceName"], meta.device_name,
            "{case:?}: historical display name"
        );
        assert_eq!(status["state"], "finished", "{case:?}");
        assert_eq!(status["downloadResult"], "verified", "{case:?}");
        assert_eq!(
            status["projectSettlement"],
            serde_json::to_value(case.receipt()).unwrap(),
            "{case:?}"
        );
        assert_eq!(
            status["downloadCompletedSteps"], download.completed_steps,
            "{case:?}"
        );
        assert_eq!(
            status["downloadTotalSteps"],
            json!(download.total_steps),
            "{case:?}"
        );
        assert_eq!(
            status["completedSteps"], progress.completed_steps,
            "{case:?}"
        );
        assert_eq!(
            status["totalSteps"],
            json!(progress.total_steps),
            "{case:?}"
        );
        assert_eq!(status["mutationMayHaveStarted"], true, "{case:?}");
        assert_eq!(status["restoreAttempted"], false, "{case:?}");
        assert_eq!(
            Editor::read(state),
            *terminal,
            "{case:?}: status is read-only"
        );
        let report = http(
            router,
            &format!(
                "/api/device-system2/report?operationId={}",
                meta.operation_id
            ),
            None,
            200,
        )
        .await;
        assert_eq!(
            report["download"],
            json!({
                "outcome": "finished", "result": "verified",
                "completedSteps": download.completed_steps, "totalSteps": download.total_steps
            }),
            "{case:?}: exact frozen DownloadFacts"
        );
        assert_eq!(report["recovery"], Value::Null, "{case:?}: no recovery run");
        assert_eq!(
            Editor::read(state),
            *terminal,
            "{case:?}: report is read-only"
        );
        assert_eq!(
            serde_json::to_value(record_metadata(state, false, case)).unwrap(),
            serde_json::to_value(meta).unwrap(),
            "{case:?}: live Record identity unchanged"
        );
    }
}

// Separate negative-only callback-order seam probe, NOT actual-worker catalogue-read counting.
fn negative_callback_probe(
    state: &crate::SharedState,
    meta: &Metadata,
    progress: &OperationProgress,
    terminal: &Editor,
    case: Case,
) {
    assert!(case.address());
    let calls = std::cell::Cell::new(0);
    let receipt = settlement::settle(state, meta, progress, Run::Download, |_, _| {
        calls.set(calls.get() + 1);
        ProgrammingParts::ALL
    });
    assert_eq!(
        receipt,
        Receipt::DeviceMissing,
        "{case:?}: separately labelled no-op seam probe"
    );
    assert_eq!(
        calls.get(),
        0,
        "{case:?}: target gate must precede current-source callback"
    );
    assert_eq!(
        Editor::read(state),
        *terminal,
        "{case:?}: negative seam probe changes nothing"
    );
}
fn exercise_history(state: &crate::SharedState, terminal: &Editor, case: Case) {
    // Only after terminal/no-replay assertions: these are explicit ordinary user edits.
    for target in terminal.undo.iter().rev() {
        crate::domain::undo_impl(state).unwrap();
        assert_eq!(
            Editor::read(state).snapshot.project,
            *target,
            "{case:?}: real domain undo follows exact terminal history"
        );
    }
    for _ in &terminal.undo {
        crate::domain::redo_impl(state).unwrap();
    }
    assert_eq!(
        Editor::read(state).snapshot,
        terminal.snapshot,
        "{case:?}: real redo returns to settled current edit"
    );
    crate::domain::redo_impl(state).unwrap();
    assert_eq!(
        Editor::read(state).snapshot.project,
        *terminal.redo.last().unwrap(),
        "{case:?}: original redo entry remains a real command"
    );
    crate::domain::undo_impl(state).unwrap();
    let restored = Editor::read(state);
    assert_eq!(restored.snapshot, terminal.snapshot, "{case:?}");
    assert_eq!(
        restored.undo, terminal.undo,
        "{case:?}: normalized undo preserved after traversal"
    );
    assert_eq!(
        restored.redo, terminal.redo,
        "{case:?}: normalized redo preserved after traversal"
    );
    assert_eq!(
        restored.stack.history_lengths(),
        terminal.stack.history_lengths(),
        "{case:?}"
    );
    assert_eq!(
        restored.revision,
        terminal.revision + (terminal.undo.len() as u64 * 2) + 2,
        "{case:?}: only explicit history commands advance revision"
    );
    assert_eq!(restored.generation, terminal.generation, "{case:?}");
    assert_eq!(
        restored.settlement_log, terminal.settlement_log,
        "{case:?}: history does not settle again"
    );
}

async fn run(case: Case) {
    let input = f::inputs(case.namespace, case.retained);
    let data = input.dir.path();
    let mut project = input.project.clone();
    let device = project.devices.get_mut(f::DEVICE).unwrap();
    device.address = Some(TARGET.parse().unwrap());
    device.commissioning.medium_config_loaded = true;
    let hardware = Arc::new(Hardware {
        device: old_device(),
        paused: AtomicBool::new(true),
        reached: Notify::new(),
        release: Notify::new(),
    });
    drop(input.conn);
    let products = data.join("caption-products.sqlite");
    let state = state_for(data, products.clone(), project, &hardware);
    let router = crate::app(state.clone(), None);
    seed(&state, case);
    let admitted = Editor::read(&state);
    let preparation = preview(&state, &router, &input.profile, case.retained).await;
    let started = http(
        &router,
        "/api/device-system2/start",
        Some(start(&preparation)),
        200,
    )
    .await;
    hardware.reached.notified().await;
    let root = data.join(crate::device_system2::ROOT_NAME);
    let id = std::fs::read_to_string(root.join("selected")).unwrap();
    let operation = root.join(&id);
    let meta = record_metadata(&state, true, case);
    assert_eq!(meta.operation_id, id, "{case:?}");
    assert_eq!(started["operationId"], id, "{case:?}: real start response");
    assert_eq!(
        started["binding"],
        serde_json::to_value(&meta.binding).unwrap(),
        "{case:?}"
    );
    assert_eq!(meta.binding.server_incarnation, admitted.server, "{case:?}");
    assert_eq!(
        meta.binding.project_incarnation,
        admitted.incarnation.to_string(),
        "{case:?}"
    );
    assert_eq!(
        meta.binding.project_revision,
        admitted.revision.to_string(),
        "{case:?}"
    );
    assert_eq!(meta.binding.device_id, f::DEVICE.0, "{case:?}");
    assert_eq!(meta.address, TARGET, "{case:?}");
    assert_eq!(
        meta.device_name,
        admitted
            .snapshot
            .project
            .devices
            .get(f::DEVICE)
            .unwrap()
            .name,
        "{case:?}"
    );
    let observed = std::fs::read(operation.join("observed.json")).unwrap();
    let original = read_bundle(&operation.join("observed.json")).unwrap();
    assert_eq!(original.format, 3, "{case:?}");
    assert!(!original.binding.simulated, "{case:?}");
    assert_eq!(
        original.project_preparation.as_ref().unwrap().device_id,
        f::DEVICE.0,
        "{case:?}"
    );
    assert_eq!(
        original.binding.target, meta.address,
        "{case:?}: original bound target"
    );
    assert_eq!(original.normalized_plan.target, meta.address, "{case:?}");
    assert_eq!(
        original.plan().unwrap().target().to_string(),
        meta.address,
        "{case:?}"
    );
    let changed = f::caption(&input.package);
    assert_ne!(changed, input.package, "{case:?}");
    let replacement = data.join("target-replacement.sqlite");
    f::replacement(&replacement, &changed, &input.package, false);
    let owner = state.product_db.as_ref().unwrap();
    owner.restore_legacy(&products, &replacement).unwrap();
    {
        let project = state.project.lock().unwrap();
        owner
            .with_read(|conn| {
                let winner: String = conn
                    .query_row(
                        "SELECT source_sha256 FROM application_program WHERE id=?1",
                        [f::PROGRAM],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_ne!(winner, original.binding.source_sha256, "{case:?}");
                assert!(
                    knx_app::system2_validation_preparation::admit_current_project_hardware(
                        conn,
                        project.as_ref().unwrap(),
                        f::DEVICE,
                        &original,
                    )
                    .is_err(),
                    "{case:?}: strict original admission refuses replacement"
                );
                assert_eq!(
                    knx_app::system2_validation_preparation::current_project_hardware_parts(
                        conn,
                        project.as_ref().unwrap(),
                        f::DEVICE,
                        &original,
                    )
                    .unwrap(),
                    ProgrammingParts::ALL,
                    "{case:?}: healthy caption ALL before target/revision edit"
                );
                Ok::<_, std::io::Error>(())
            })
            .unwrap()
            .unwrap();
    }
    assert_eq!(
        Editor::read(&state),
        admitted,
        "{case:?}: source comparison is read-only"
    );
    let before = edit_and_seed_redo(&state, &admitted, case);
    assert_ne!(
        before.revision.to_string(),
        meta.binding.project_revision,
        "{case:?}: genuine revision difference"
    );
    record_metadata(&state, true, case);
    hardware.release.notify_one();
    super::super::failure_diagnostic::settled_slow(&state, &router, "finished").await;
    // Receipt first: a helper refusal must not hide a broken actual terminal target gate.
    assert_eq!(
        state
            .device_system2
            .status()
            .unwrap()
            .unwrap()
            .project_settlement,
        case.receipt(),
        "{case:?}: same actual worker terminal receipt"
    );
    let after = Editor::read(&state);
    if case.address() {
        assert_eq!(after, before, "{case:?}: exact POST-command model, stack, revision, generation, timestamps and logs preserved");
        assert_eq!(
            flags(&state),
            case.edited_flags(),
            "{case:?}: no address recredit or memory invalidation"
        );
    } else {
        assert_positive(&before, &after, case);
    }
    let progress = journal(&state, &operation, &original, case);
    assert_eq!(
        std::fs::read(operation.join("observed.json")).unwrap(),
        observed,
        "{case:?}"
    );
    observe(&state, &router, &meta, &progress, &after, case).await;
    let reopened_progress = journal(&state, &operation, &original, case);
    assert_eq!(
        reopened_progress, progress,
        "{case:?}: journal reopen preserves every terminal fact"
    );
    assert_eq!(
        read_bundle(&operation.join("observed.json")).unwrap(),
        original,
        "{case:?}"
    );
    assert_eq!(
        Editor::read(&state),
        after,
        "{case:?}: journal-only reopen is inert"
    );
    if case.address() {
        // Same live Record metadata paired with this operation's actual reopened journal progress.
        let paired_meta = record_metadata(&state, false, case);
        assert_eq!(
            serde_json::to_value(&paired_meta).unwrap(),
            serde_json::to_value(&meta).unwrap(),
            "{case:?}"
        );
        negative_callback_probe(&state, &paired_meta, &reopened_progress, &after, case);
    }
    exercise_history(&state, &after, case);
    assert_eq!(
        journal(&state, &operation, &original, case),
        progress,
        "{case:?}: user history edits cannot thaw frozen facts"
    );
    assert_eq!(
        std::fs::read(operation.join("observed.json")).unwrap(),
        observed,
        "{case:?}: immutable original bytes through all assertions"
    );
    eprintln!(
        "E14_TARGET_CASE_OK family={} namespace={} marked={} edit={}",
        if case.retained {
            "retained"
        } else {
            "standalone"
        },
        case.namespace,
        case.marked,
        if case.address() {
            "address"
        } else {
            "revision"
        }
    );
}
async fn matrix(retained: bool, change: Change) {
    for namespace in [20, 23] {
        for marked in [false, true] {
            run(Case {
                namespace,
                retained,
                marked,
                change,
            })
            .await;
        }
    }
}

#[tokio::test]
async fn caption_target_retained_address_change_refuses() {
    matrix(true, Change::Address).await;
}
#[tokio::test]
async fn caption_target_standalone_address_change_refuses() {
    matrix(false, Change::Address).await;
}
#[tokio::test]
async fn caption_target_retained_revision_only_credits() {
    matrix(true, Change::RevisionOnly).await;
}
#[tokio::test]
async fn caption_target_standalone_revision_only_credits() {
    matrix(false, Change::RevisionOnly).await;
}
