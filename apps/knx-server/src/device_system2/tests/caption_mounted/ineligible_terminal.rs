//! Real failed downloads refuse caption credit while healthy controls settle the same source.

use super::*;
use crate::device_system2::{dto::Metadata, settlement::Receipt};
use knx_app::{
    property_validation::{
        DownloadResult, FailureDiagnosticRun, FinalStepCategory, FinalStepDiagnostic,
        FinalStepStage, OperationProgress,
    },
    property_validation_bundle::PreparedBundle,
};
use knx_core::commissioning::properties::PID_PROGRAM_VERSION;
use knx_net::commissioning::simulator::{PropertyReadFault, ReadFault};
use knx_store::project_history::NativeSnapshot;

const OLD_VERSION: [u8; 5] = [0, 1, 0x92, 5, 4];
const HISTORICAL_DOWNLOAD: &str = "2001-02-03T04:05:06Z";

#[derive(Clone, Copy, Debug)]
struct Case {
    namespace: u8,
    retained: bool,
    faulty: bool,
    marked: bool,
}
impl Case {
    fn terminal(self) -> &'static str {
        if self.faulty {
            "failed"
        } else {
            "finished"
        }
    }

    fn outcome(self) -> LocalOutcome {
        if self.faulty {
            LocalOutcome::Failed
        } else {
            LocalOutcome::Finished
        }
    }

    fn result(self) -> DownloadResult {
        if self.faulty {
            DownloadResult::TransferCompleteVerificationUnfinished
        } else {
            DownloadResult::Verified
        }
    }

    fn diagnostics(self) -> Vec<FinalStepDiagnostic> {
        if self.faulty {
            vec![FinalStepDiagnostic {
                run: FailureDiagnosticRun::Download,
                category: FinalStepCategory::VerificationMismatch,
                stage: FinalStepStage::ProgramVersion,
            }]
        } else {
            Vec::new()
        }
    }
}

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
            settlement_log: state
                .session_log
                .lock()
                .unwrap()
                .entries()
                .iter()
                .filter(|entry| entry.source == "System2Download")
                .map(|entry| serde_json::to_value(entry).unwrap())
                .collect(),
        }
    }
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

fn seed_history(state: &crate::SharedState, case: Case) {
    // Real manual commands BEFORE admission; one undo leaves both directions nonempty.
    // A second true mark in the marked case leaves an old true inverse in undo.
    // The pending case instead leaves old true claims in both undo and redo.
    for programmed in [true, case.marked, !case.marked] {
        let binding = crate::domain::SnapshotBinding {
            server_incarnation: state.server_incarnation.clone(),
            project_incarnation: state.project_incarnation.load(Ordering::SeqCst),
            snapshot_revision: crate::domain::current_project_revision(state),
        };
        crate::domain::apply_bound_prepared(
            state,
            &binding,
            "Ineligible terminal fixture",
            knx_core::Command::SetDeviceProgramming {
                device: f::DEVICE,
                change: ProgrammingChange::mark(ProgrammingParts::ALL, programmed),
            },
        )
        .unwrap();
        assert_eq!(
            crate::domain::current_project_revision(state),
            binding.snapshot_revision + 1,
            "{case:?}: real command revision"
        );
    }
    crate::domain::undo_impl(state).unwrap();
    let before = Editor::read(state);
    assert_eq!(
        project_flags(&before.snapshot.project),
        [case.marked, case.marked, case.marked, case.marked, true]
    );
    assert_eq!(before.undo.len(), 2);
    assert_eq!(before.redo.len(), 1);
    assert!(
        before
            .undo
            .iter()
            .chain(&before.redo)
            .any(|project| { project_flags(project)[1..4].iter().any(|claim| *claim) }),
        "{case:?}: stale MEMORY claims make history invalidation observable"
    );
    if !case.marked {
        assert_eq!(project_flags(before.redo.last().unwrap()), [true; 5]);
    }
    for project in std::iter::once(&before.snapshot.project)
        .chain(&before.undo)
        .chain(&before.redo)
    {
        assert_eq!(
            project
                .devices
                .get(f::DEVICE)
                .unwrap()
                .commissioning
                .last_download,
            Some(HISTORICAL_DOWNLOAD.parse().unwrap()),
            "{case:?}: non-vacuous historical timestamp survives genuine setup"
        );
    }
    assert!(before.store_path.is_none());
    assert!(before.settlement_log.is_empty());
}

fn record_metadata(state: &crate::SharedState, running: bool, case: Case) -> Metadata {
    let inner = state.device_system2.inner.lock().unwrap();
    let record = inner.current.as_ref().unwrap();
    assert_eq!(
        record.control_id, record.metadata.operation_id,
        "{case:?}: download owner"
    );
    if running {
        assert!(
            record.task.as_ref().is_some_and(|task| !task.is_finished()),
            "{case:?}: actual admitted worker remains paused"
        );
    } else {
        assert!(
            record.task.is_none(),
            "{case:?}: actual worker joined, ownership released"
        );
        assert_eq!(
            record.settlement,
            Receipt::SessionOnly,
            "{case:?}: worker receipt"
        );
    }
    record.metadata.clone()
}

fn assert_original(original: &PreparedBundle, meta: &Metadata, case: Case) {
    assert_eq!(original.format, 3);
    assert!(!original.binding.simulated);
    assert_eq!(original.binding.target, meta.address);
    assert_eq!(original.normalized_plan.target, meta.address);
    assert_eq!(original.plan().unwrap().target().to_string(), meta.address);
    let preparation = original.project_preparation.as_ref().unwrap();
    assert_eq!(preparation.device_id, f::DEVICE.0);
    assert_eq!(
        preparation.schema_namespace,
        format!("http://knx.org/xml/project/{}", case.namespace),
        "{case:?}: actual admitted source namespace"
    );
    assert_eq!(
        matches!(
            &preparation.product_input,
            knx_app::system2_validation_preparation::ProjectProductInput::RetainedSourceMembers { .. }
        ),
        case.retained,
        "{case:?}: actual admitted source family"
    );
    let hardware = original.hardware_preparation.as_ref().unwrap();
    assert_eq!(hardware.operation, meta.operation_id);
    let observation = &hardware.observation;
    assert_eq!(observation.target, meta.address);
    assert_eq!(observation.serial, SERIAL);
    assert_eq!(observation.mask, 0x0025);
    assert_eq!(observation.manufacturer, 1);
    assert_eq!(observation.hardware, [0, 1, 0, 0, 0, 91]);
    assert_eq!(
        observation.application, original.binding.application,
        "{case:?}: planned version is bound to the genuine admission"
    );
    // OLD and NEW memory may belong to the same application version. Only the
    // injected final response must differ; initial version inequality is not required.
    assert_ne!(
        observation.application, [0; 5],
        "{case:?}: final fault payload differs from the planned version"
    );
    assert_eq!(
        original.binding.application, OLD_VERSION,
        "{case:?}: matching initial version is the simulator's seeded identity"
    );
    // The live observation producer stores only deviations from binding.application.
    // None is a matching read, whereas an early zero fault produces Some(zero bytes).
    assert_eq!(
        observation.observed_application.as_deref(),
        None,
        "{case:?}: BLOCK if fault changed the actual original hardware observation"
    );
    assert!(matches!(
        original.normalized_plan.steps.last().unwrap().operation,
        OperationRecord::Disconnect
    ));
}

fn journal(
    state: &crate::SharedState,
    operation: &Path,
    original: &PreparedBundle,
    case: Case,
) -> (OperationProgress, RecoveryRecord) {
    record_metadata(state, false, case);
    // The completed real worker has released the writer; never reopen at the write barrier.
    let store = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
    let progress = store.progress();
    let recovery = store.recovery().unwrap().clone();
    assert_eq!(
        recovery.original_bundle, *original,
        "{case:?}: original recovery pairing"
    );
    // RecoveryRecord and progress retain version deviations, not redundant matching bytes.
    // The original live binding above is OLD_VERSION; any early zero response is nonmatching.
    assert_eq!(
        recovery.observed_program_version.as_deref(),
        None,
        "{case:?}: BLOCK if fault changed the actual read_identity prestate"
    );
    assert_eq!(
        progress.program_version_observation, None,
        "{case:?}: genuine matching acquisition, not fault bytes"
    );
    assert_eq!(store.outcome(), case.outcome());
    assert_eq!(progress.outcome, case.outcome());
    assert!(progress.mutation_possible);
    assert!(!progress.restore_attempted);
    assert_eq!(progress.download.outcome, case.outcome());
    assert_eq!(progress.download.result, case.result());
    assert_eq!(
        progress.final_step_failures,
        case.diagnostics(),
        "{case:?}: exact final failure"
    );
    assert!(
        progress.failure_diagnostics.is_empty(),
        "{case:?}: no pointer failure"
    );
    let steps = original.normalized_plan.steps.len();
    assert!(steps > 1);
    assert_eq!(progress.download.total_steps, Some(steps));
    assert_eq!(
        progress.download.completed_steps + usize::from(case.faulty),
        steps,
        "{case:?}: every transfer step completed, final Disconnect actually started"
    );
    assert_eq!(progress.completed_steps, progress.download.completed_steps);
    assert_eq!(progress.total_steps, progress.download.total_steps);
    let inner = state.device_system2.inner.lock().unwrap();
    let record = inner.current.as_ref().unwrap();
    assert_eq!(
        record.progress, progress,
        "{case:?}: complete live Record/journal facts"
    );
    (progress, recovery)
}

fn assert_terminal(before: &Editor, after: &Editor, case: Case) {
    let actual = &after
        .snapshot
        .project
        .devices
        .get(f::DEVICE)
        .unwrap()
        .commissioning;
    let historical = Some(HISTORICAL_DOWNLOAD.parse().unwrap());
    assert_eq!(
        before
            .snapshot
            .project
            .devices
            .get(f::DEVICE)
            .unwrap()
            .commissioning
            .last_download,
        historical
    );
    if case.faulty {
        assert_eq!(
            actual.last_download, historical,
            "{case:?}: failure earns no timestamp"
        );
    } else {
        assert!(actual.last_download.is_some());
        assert_ne!(
            actual.last_download, historical,
            "{case:?}: real successful timestamp"
        );
    }
    let mut expected = before.snapshot.clone();
    let c = &mut expected
        .project
        .devices
        .get_mut(f::DEVICE)
        .unwrap()
        .commissioning;
    c.application_program_loaded = !case.faulty;
    c.parameters_loaded = !case.faulty;
    c.communication_part_loaded = !case.faulty;
    if !case.faulty {
        c.individual_address_loaded = true;
        c.last_download = actual.last_download;
    }
    assert_eq!(
        after.snapshot, expected,
        "{case:?}: exact live credit/no-credit, address, medium, timestamps and context"
    );
    assert_eq!(
        project_flags(&after.snapshot.project),
        if case.faulty {
            [case.marked, false, false, false, true]
        } else {
            [true; 5]
        },
        "{case:?}: failed run cannot earn source-positive ALL"
    );
    // Independent expectations: edit captured clones directly, never use settlement,
    // forget_programmed_claims or its history-normalization code to construct the oracle.
    let expected_history = |states: &[knx_core::Project]| {
        states
            .iter()
            .map(|old| {
                let mut expected = old.clone();
                let c = &mut expected.devices.get_mut(f::DEVICE).unwrap().commissioning;
                c.application_program_loaded = false;
                c.parameters_loaded = false;
                c.communication_part_loaded = false;
                if !case.faulty {
                    c.individual_address_loaded = false;
                    // These explicit flag-only inverses do not undo a newly earned timestamp.
                    c.last_download = actual.last_download;
                }
                expected
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        after.undo,
        expected_history(&before.undo),
        "{case:?}: exact stale undo MEMORY oracle"
    );
    assert_eq!(
        after.redo,
        expected_history(&before.redo),
        "{case:?}: exact stale redo MEMORY oracle"
    );
    assert_eq!(
        after.stack.history_lengths(),
        before.stack.history_lengths(),
        "{case:?}: no new undo command"
    );
    assert_eq!(
        after.stack.comparison_revision(f::DEVICE),
        before.stack.comparison_revision(f::DEVICE) + 1,
        "{case:?}: touched claims forgotten once, even when already pending"
    );
    assert_eq!(
        after.revision,
        before.revision + 1,
        "{case:?}: one settlement revision"
    );
    assert_eq!(after.generation, before.generation);
    assert_eq!(after.server, before.server);
    assert_eq!(after.incarnation, before.incarnation);
    assert_eq!(after.saved_at, before.saved_at);
    assert_eq!(after.store_path, before.store_path);
    assert!(before.settlement_log.is_empty());
    assert_eq!(
        after.settlement_log.len(),
        1,
        "{case:?}: one legitimate settlement event"
    );
    let log = &after.settlement_log[0];
    assert_eq!(log["severity"], "info");
    assert_eq!(log["source"], "System2Download");
    let name = &before.snapshot.project.devices.get(f::DEVICE).unwrap().name;
    let suffix = if case.faulty {
        "not verified: application, parameters and group addresses marked not programmed"
    } else {
        "verified: address, application, parameters and group addresses marked programmed"
    };
    assert_eq!(
        log["message"],
        format!("Experimental System-2 download to “{name}” ({TARGET}) {suffix}")
    );
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
        assert_eq!(status["operationId"], meta.operation_id);
        assert_eq!(
            status["binding"],
            serde_json::to_value(&meta.binding).unwrap()
        );
        assert_eq!(status["address"], meta.address);
        assert_eq!(status["deviceName"], meta.device_name);
        assert_eq!(status["state"], case.terminal());
        assert_eq!(
            status["downloadResult"],
            serde_json::to_value(case.result()).unwrap()
        );
        assert_eq!(status["projectSettlement"], "sessionOnly");
        assert_eq!(status["downloadCompletedSteps"], download.completed_steps);
        assert_eq!(status["downloadTotalSteps"], json!(download.total_steps));
        assert_eq!(status["completedSteps"], progress.completed_steps);
        assert_eq!(status["totalSteps"], json!(progress.total_steps));
        assert_eq!(status["mutationMayHaveStarted"], true);
        assert_eq!(status["restoreAttempted"], false);
        assert_eq!(
            status["finalStepFailures"],
            serde_json::to_value(case.diagnostics()).unwrap()
        );
        assert_eq!(status["failureDiagnostics"], json!([]));
        assert_eq!(Editor::read(state), *terminal, "{case:?}: status is inert");
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
                "outcome": download.outcome, "result": download.result,
                "completedSteps": download.completed_steps, "totalSteps": download.total_steps
            }),
            "{case:?}: returned original frozen DownloadFacts, not volatile receipt"
        );
        assert_eq!(
            report["finalStepFailures"],
            serde_json::to_value(case.diagnostics()).unwrap()
        );
        assert_eq!(report["failureDiagnostics"], json!([]));
        assert_eq!(report["recovery"], Value::Null);
        assert_eq!(Editor::read(state), *terminal, "{case:?}: report is inert");
        assert_eq!(
            serde_json::to_value(record_metadata(state, false, case)).unwrap(),
            serde_json::to_value(meta).unwrap(),
            "{case:?}: unchanged actual Record identity"
        );
    }
}

fn exercise_history(state: &crate::SharedState, terminal: &Editor, case: Case) {
    // Explicit user edits only AFTER every terminal read-only oracle.
    for target in terminal.undo.iter().rev() {
        crate::domain::undo_impl(state).unwrap();
        assert_eq!(
            Editor::read(state).snapshot.project,
            *target,
            "{case:?}: actual undo target"
        );
    }
    for _ in &terminal.undo {
        crate::domain::redo_impl(state).unwrap();
    }
    assert_eq!(Editor::read(state).snapshot, terminal.snapshot);
    crate::domain::redo_impl(state).unwrap();
    assert_eq!(
        Editor::read(state).snapshot.project,
        *terminal.redo.last().unwrap(),
        "{case:?}: original redo cannot resurrect stale MEMORY claims"
    );
    crate::domain::undo_impl(state).unwrap();
    let restored = Editor::read(state);
    assert_eq!(restored.snapshot, terminal.snapshot);
    assert_eq!(restored.undo, terminal.undo);
    assert_eq!(restored.redo, terminal.redo);
    assert_eq!(
        restored.stack.history_lengths(),
        terminal.stack.history_lengths()
    );
    assert_eq!(
        restored.revision,
        terminal.revision + terminal.undo.len() as u64 * 2 + 2
    );
    assert_eq!(restored.generation, terminal.generation);
    assert_eq!(restored.settlement_log, terminal.settlement_log);
}

async fn run(case: Case) {
    let input = f::inputs(case.namespace, case.retained);
    let data = input.dir.path();
    let mut project = input.project.clone();
    let device = project.devices.get_mut(f::DEVICE).unwrap();
    device.address = Some(TARGET.parse().unwrap());
    device.commissioning.medium_config_loaded = true;
    // Initial fictional historical input, not a post-admission fabricated state.
    device.commissioning.last_download = Some(HISTORICAL_DOWNLOAD.parse().unwrap());
    let fault = case.faulty.then_some(PropertyReadFault {
        object_index: 3,
        property_id: PID_PROGRAM_VERSION,
        // SOURCE-derived matching transmissions: observation, read_identity, verify_outcome.
        // Not measured read counting; wrong phase must BLOCK, never tune this threshold.
        from_read: 3,
        fault: ReadFault::Data(&[0, 0, 0, 0, 0]),
    });
    let hardware = Arc::new(Hardware {
        device: old_device_with_property_read_fault(fault),
        paused: AtomicBool::new(true),
        reached: Notify::new(),
        release: Notify::new(),
    });
    drop(input.conn);
    let products = data.join("caption-products.sqlite");
    let state = state_for(data, products.clone(), project, &hardware);
    let router = crate::app(state.clone(), None);
    seed_history(&state, case);
    let before = Editor::read(&state);
    let preparation = preview(&state, &router, &input.profile, case.retained).await;
    let started = http(
        &router,
        "/api/device-system2/start",
        Some(start(&preparation)),
        200,
    )
    .await;
    // The unchanged completion helper owns its existing 180 s budget and only joins
    // finished tasks. Cancelling this loser at the barrier cannot take a live handle.
    tokio::select! {
        () = hardware.reached.notified() => {},
        () = super::super::failure_diagnostic::settled_slow(&state, &router, case.terminal()) => {
            panic!("{case:?}: BLOCK setup/phase: actual worker completed before first data write");
        }
    }
    assert!(
        !hardware.paused.load(Ordering::SeqCst),
        "{case:?}: actual first write consumed barrier"
    );
    let root = data.join(crate::device_system2::ROOT_NAME);
    let id = std::fs::read_to_string(root.join("selected")).unwrap();
    let operation = root.join(&id);
    let meta = record_metadata(&state, true, case);
    assert_eq!(meta.operation_id, id);
    assert_eq!(started["operationId"], id);
    assert_eq!(
        started["binding"],
        serde_json::to_value(&meta.binding).unwrap()
    );
    assert_eq!(meta.binding.server_incarnation, before.server);
    assert_eq!(
        meta.binding.project_incarnation,
        before.incarnation.to_string()
    );
    assert_eq!(meta.binding.project_revision, before.revision.to_string());
    assert_eq!(meta.binding.device_id, f::DEVICE.0);
    assert_eq!(meta.address, TARGET);
    assert_eq!(
        meta.device_name,
        before.snapshot.project.devices.get(f::DEVICE).unwrap().name
    );
    let metadata_bytes = std::fs::read(operation.join("metadata.json")).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&metadata_bytes).unwrap(),
        serde_json::to_value(&meta).unwrap()
    );
    let observed = std::fs::read(operation.join("observed.json")).unwrap();
    let original = read_bundle(&operation.join("observed.json")).unwrap();
    assert_original(&original, &meta, case);
    let changed = f::caption(&input.package);
    assert_ne!(changed, input.package);
    let replacement = data.join("ineligible-terminal-replacement.sqlite");
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
                assert_ne!(winner, original.binding.source_sha256);
                assert!(
                    knx_app::system2_validation_preparation::admit_current_project_hardware(
                        conn,
                        project.as_ref().unwrap(),
                        f::DEVICE,
                        &original,
                    )
                    .is_err(),
                    "{case:?}: strict old admission refuses changed caption"
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
                    "{case:?}: genuine healthy current-parts ALL before actual terminal result"
                );
                Ok::<_, std::io::Error>(())
            })
            .unwrap()
            .unwrap();
    }
    assert_eq!(
        Editor::read(&state),
        before,
        "{case:?}: pre-worker baseline and ALL witness are read-only"
    );
    record_metadata(&state, true, case);
    let released_at = chrono::Utc::now();
    hardware.release.notify_one();
    super::super::failure_diagnostic::settled_slow(&state, &router, case.terminal()).await;
    let joined_at = chrono::Utc::now();
    assert_eq!(
        serde_json::to_value(record_metadata(&state, false, case)).unwrap(),
        serde_json::to_value(&meta).unwrap()
    );
    let (progress, recovery) = journal(&state, &operation, &original, case);
    let after = Editor::read(&state);
    assert_terminal(&before, &after, case);
    if !case.faulty {
        let at = after
            .snapshot
            .project
            .devices
            .get(f::DEVICE)
            .unwrap()
            .commissioning
            .last_download
            .unwrap();
        assert!(
            at >= released_at && at <= joined_at,
            "{case:?}: actual terminal download time"
        );
    }
    observe(&state, &router, &meta, &progress, &after, case).await;
    assert_eq!(
        journal(&state, &operation, &original, case),
        (progress.clone(), recovery.clone())
    );
    assert_eq!(
        Editor::read(&state),
        after,
        "{case:?}: journal-only reopen is inert"
    );
    assert_eq!(
        std::fs::read(operation.join("observed.json")).unwrap(),
        observed
    );
    assert_eq!(
        read_bundle(&operation.join("observed.json")).unwrap(),
        original
    );
    assert_eq!(
        std::fs::read(operation.join("metadata.json")).unwrap(),
        metadata_bytes
    );
    exercise_history(&state, &after, case);
    assert_eq!(
        journal(&state, &operation, &original, case),
        (progress, recovery),
        "{case:?}: explicit history commands cannot thaw original download or recovery facts"
    );
    assert_eq!(
        std::fs::read(operation.join("observed.json")).unwrap(),
        observed
    );
    assert_eq!(
        std::fs::read(operation.join("metadata.json")).unwrap(),
        metadata_bytes
    );
    eprintln!(
        "E16_CASE_OK family={} namespace={} terminal={} marked={}",
        if case.retained {
            "retained"
        } else {
            "standalone"
        },
        case.namespace,
        if case.faulty { "fault" } else { "healthy" },
        case.marked
    );
}

async fn run_beliefs(namespace: u8, retained: bool, faulty: bool) {
    for marked in [false, true] {
        run(Case {
            namespace,
            retained,
            faulty,
            marked,
        })
        .await;
    }
}

#[tokio::test]
async fn caption_ineligible_terminal_retained20_final_version_mismatch() {
    run_beliefs(20, true, true).await;
}

#[tokio::test]
async fn caption_ineligible_terminal_retained20_healthy_control() {
    run_beliefs(20, true, false).await;
}

#[tokio::test]
async fn caption_ineligible_terminal_retained23_final_version_mismatch() {
    run_beliefs(23, true, true).await;
}

#[tokio::test]
async fn caption_ineligible_terminal_retained23_healthy_control() {
    run_beliefs(23, true, false).await;
}

#[tokio::test]
async fn caption_ineligible_terminal_standalone20_final_version_mismatch() {
    run_beliefs(20, false, true).await;
}

#[tokio::test]
async fn caption_ineligible_terminal_standalone20_healthy_control() {
    run_beliefs(20, false, false).await;
}

#[tokio::test]
async fn caption_ineligible_terminal_standalone23_final_version_mismatch() {
    run_beliefs(23, false, true).await;
}

#[tokio::test]
async fn caption_ineligible_terminal_standalone23_healthy_control() {
    run_beliefs(23, false, false).await;
}
