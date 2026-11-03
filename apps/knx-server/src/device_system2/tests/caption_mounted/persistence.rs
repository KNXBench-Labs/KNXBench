//! Mounted changed-caption settlement preserves saved editor state when its history write aborts.

use super::*;
use knx_app::property_validation::{DownloadFacts, DownloadResult};
use knx_store::project_history::{self as history, NativeSnapshot};
use rusqlite::{types::Value as SqlValue, Connection};

#[derive(Clone, Copy, Debug)]
struct Case {
    namespace: u8,
    retained: bool,
    marked: bool,
    parameter_edit: bool,
    abort_persistence: bool,
}

#[derive(Debug, PartialEq)]
struct Editor {
    snapshot: NativeSnapshot,
    stack: knx_core::CommandStack,
    undo: Vec<knx_core::Project>,
    redo: Vec<knx_core::Project>,
    revision: u64,
    generation: i64,
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
        }
    }

    fn assert_content(&self, other: &Self, case: Case) {
        assert_eq!(
            self.snapshot, other.snapshot,
            "{case:?}: exact project/context"
        );
        assert_eq!(self.undo, other.undo, "{case:?}: exact undo states");
        assert_eq!(self.redo, other.redo, "{case:?}: exact redo states");
    }
}

// Full logical file state, including all history/context rows and carried source bytes.
// SQLite page layout/sidecar byte identity and crash durability are deliberately not claimed.
#[derive(Debug, PartialEq)]
struct Saved {
    baseline: NativeSnapshot,
    working: NativeSnapshot,
    undo: Vec<knx_core::Project>,
    redo: Vec<knx_core::Project>,
    generation: i64,
    schema: Vec<Vec<SqlValue>>,
    tables: Vec<(String, Vec<Vec<SqlValue>>)>,
}
fn rows(conn: &Connection, sql: &str) -> Vec<Vec<SqlValue>> {
    let mut statement = conn.prepare(sql).unwrap();
    let columns = statement.column_count();
    let mut rows = statement
        .query_map([], |row| {
            (0..columns)
                .map(|column| row.get(column))
                .collect::<Result<Vec<SqlValue>, _>>()
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // SQL row order is not file content. Keep duplicates and compare every typed value.
    rows.sort_by_cached_key(|row| format!("{row:?}"));
    rows
}
fn saved(path: &Path) -> Saved {
    let conn = knx_store::open_existing_read_only(path).unwrap().conn;
    let tx = conn.unchecked_transaction().unwrap();
    let editor = history::load_editor(&tx).unwrap().unwrap();
    assert_eq!(NativeSnapshot::read(&tx).unwrap(), editor.baseline);
    let schema = rows(&tx, "SELECT type, name, tbl_name, sql FROM sqlite_schema");
    let names = tx
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let tables = names
        .into_iter()
        .map(|name| {
            let sql = format!("SELECT * FROM \"{}\"", name.replace('"', "\"\""));
            let values = rows(&tx, &sql);
            (name, values)
        })
        .collect();
    tx.commit().unwrap();
    Saved {
        baseline: editor.baseline,
        working: editor.working,
        undo: editor.undo,
        redo: editor.redo,
        generation: editor.generation,
        schema,
        tables,
    }
}

fn install_fault(path: &Path, generation: i64) {
    let conn = history::open_editor_store(path, false).unwrap();
    // A trigger would be refused by snapshot::check_schema BEFORE write_workspace.
    // This generation-guarded expression index adds no table/view/trigger/column.
    // CASE yields zero for the healthy existing row, avoiding constant-folded errors.
    // SQLite's deterministic abs(i64::MIN) overflow aborts the next generation's
    // actual project_history_state INSERT ... ON CONFLICT UPDATE, after the stack
    // deletion inside save_editor_inner's transaction.
    conn.execute_batch(&format!(
        "CREATE INDEX caption_persistence_abort ON project_history_state \
         (abs(CASE WHEN generation > {generation} THEN -9223372036854775808 ELSE 0 END))"
    ))
    .unwrap();
    // No connection, transaction or lock escapes this synchronous fixture helper.
}
fn remove_fault(path: &Path) {
    let conn = history::open_editor_store(path, false).unwrap();
    conn.execute_batch("DROP INDEX caption_persistence_abort")
        .unwrap();
}
fn apply(state: &crate::SharedState, command: knx_core::Command) {
    let binding = crate::domain::SnapshotBinding {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::SeqCst),
        snapshot_revision: crate::domain::current_project_revision(state),
    };
    crate::domain::apply_bound_prepared(state, &binding, "Caption persistence fixture", command)
        .unwrap();
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
    if case.parameter_edit {
        let id = {
            let project = state.project.lock().unwrap();
            project.as_ref().unwrap().installations[0]
                .parameters
                .iter()
                .find(|p| p.source.ets_id == format!("{}_R", f::PROGRAM))
                .unwrap()
                .id
        };
        apply(
            state,
            knx_core::Command::SetParameterValue {
                id,
                device: f::DEVICE,
                ets_id: format!("{}_R", f::PROGRAM),
                raw: "772".into(),
            },
        );
    }
    // Use real durable commands, not fabricated recovered snapshots. Both directions
    // remain nonempty, with old programmed claims that successful settlement must forget.
    mark(state);
    apply(
        state,
        knx_core::Command::SetDeviceProgramming {
            device: f::DEVICE,
            change: ProgrammingChange::mark(ProgrammingParts::ALL, false),
        },
    );
    if !case.marked {
        mark(state);
    }
    crate::domain::undo_impl(state).unwrap();
    assert_eq!(
        flags(state),
        [case.marked, case.marked, case.marked, case.marked, true],
        "{case:?}: preterminal claims"
    );
}
fn settlement_entries(state: &crate::SharedState) -> Vec<crate::session_log::LogEntry> {
    state
        .session_log
        .lock()
        .unwrap()
        .entries()
        .iter()
        .filter(|entry| entry.source == "System2Download")
        .cloned()
        .collect()
}
fn facts(
    operation: &Path,
    original: &knx_app::property_validation_bundle::PreparedBundle,
) -> DownloadFacts {
    let store = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
    let progress = store.progress();
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    assert_eq!(progress.outcome, LocalOutcome::Finished);
    assert!(progress.mutation_possible);
    assert!(!progress.restore_attempted);
    assert_eq!(progress.download.outcome, LocalOutcome::Finished);
    assert_eq!(progress.download.result, DownloadResult::Verified);
    assert!(progress.download.completed_steps > 0);
    assert_eq!(
        Some(progress.download.completed_steps),
        progress.download.total_steps
    );
    assert_eq!(store.recovery().unwrap().original_bundle, *original);
    progress.download
}
async fn observe(router: &Router, id: &str, receipt: &str, download: DownloadFacts, case: Case) {
    for _ in 0..2 {
        let status = http(router, "/api/device-system2/status", None, 200).await;
        assert_eq!(status["operationId"], id, "{case:?}");
        assert_eq!(status["state"], "finished", "{case:?}");
        assert_eq!(status["downloadResult"], "verified", "{case:?}");
        assert_eq!(status["projectSettlement"], receipt, "{case:?}");
        assert_eq!(status["downloadCompletedSteps"], download.completed_steps);
        assert_eq!(status["downloadTotalSteps"], json!(download.total_steps));
        let report = http(
            router,
            &format!("/api/device-system2/report?operationId={id}"),
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
            "{case:?}: frozen transfer facts, not project-persistence outcome"
        );
        assert_eq!(report["recovery"], Value::Null, "{case:?}");
    }
}
fn assert_saved_editor(saved: &Saved, editor: &Editor, case: Case) {
    assert_eq!(
        saved.working, editor.snapshot,
        "{case:?}: recovered working state"
    );
    assert_eq!(saved.undo, editor.undo, "{case:?}: recovered undo");
    assert_eq!(saved.redo, editor.redo, "{case:?}: recovered redo");
    assert_eq!(saved.generation, editor.generation, "{case:?}");
}
fn exercise_preserved_history(state: &crate::SharedState, before: &Editor, case: Case) {
    crate::domain::undo_impl(state).unwrap();
    assert_eq!(
        Editor::read(state).snapshot.project,
        *before.undo.last().unwrap(),
        "{case:?}: original undo target survives failed persistence and fresh open"
    );
    crate::domain::redo_impl(state).unwrap();
    before.assert_content(&Editor::read(state), case);
    crate::domain::redo_impl(state).unwrap();
    assert_eq!(
        Editor::read(state).snapshot.project,
        *before.redo.last().unwrap(),
        "{case:?}: original redo target survives failed persistence and fresh open"
    );
    crate::domain::undo_impl(state).unwrap();
    before.assert_content(&Editor::read(state), case);
    let path = state.store_path.lock().unwrap().clone().unwrap();
    assert_saved_editor(&saved(&path), &Editor::read(state), case);
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
    if case.retained {
        // Keep the real native-save portability policy, exactly as the parent fixture does.
        knx_productdb::install_package(&input.conn, "portable-original.knxprod", &input.package)
            .unwrap();
    }
    drop(input.conn);
    let products = data.join("caption-products.sqlite");
    let state = state_for(data, products.clone(), project, &hardware);
    let router = crate::app(state.clone(), None);
    if case.marked {
        mark(&state);
    }
    assert_eq!(
        flags(&state),
        [case.marked, case.marked, case.marked, case.marked, true],
        "{case:?}: claims at original start admission"
    );
    let path = data.join("caption-persistence.knxdb");
    crate::domain::save_project_as(&state, &path).unwrap();
    let preparation = preview(&state, &router, &input.profile, case.retained).await;
    http(
        &router,
        "/api/device-system2/start",
        Some(start(&preparation)),
        200,
    )
    .await;
    tokio::time::timeout(Duration::from_secs(60), hardware.reached.notified())
        .await
        .expect("actual first data write after original admission");
    let root = data.join(crate::device_system2::ROOT_NAME);
    let id = std::fs::read_to_string(root.join("selected")).unwrap();
    let operation = root.join(&id);
    let observed = std::fs::read(operation.join("observed.json")).unwrap();
    let original = read_bundle(&operation.join("observed.json")).unwrap();
    assert_eq!(original.format, 3);
    assert!(!original.binding.simulated);
    let changed = f::caption(&input.package);
    assert_ne!(changed, input.package);
    let replacement = data.join("persistence-replacement.sqlite");
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
                        &original
                    )
                    .is_err(),
                    "{case:?}: changed source cannot reuse strict original admission"
                );
                Ok::<_, std::io::Error>(())
            })
            .unwrap()
            .unwrap();
    }
    seed_history(&state, case);
    if case.abort_persistence {
        install_fault(&path, *state.history_generation.lock().unwrap());
    }
    // Capture AFTER fault DDL. Every handle has closed before the worker is released.
    let before = Editor::read(&state);
    let old_file = saved(&path);
    assert_saved_editor(&old_file, &before, case);
    if case.parameter_edit {
        assert_ne!(
            old_file.working, old_file.baseline,
            "{case:?}: working edit differs from root"
        );
    }
    assert!(
        !before.undo.is_empty() && !before.redo.is_empty(),
        "{case:?}"
    );
    assert!(
        before
            .undo
            .iter()
            .chain(&before.redo)
            .any(|project| { project_flags(project)[..4].iter().any(|claim| *claim) }),
        "{case:?}: controls must observe removal of old history claims"
    );
    assert!(settlement_entries(&state).is_empty(), "{case:?}");
    hardware.release.notify_one();
    super::super::failure_diagnostic::settled_slow(&state, &router, "finished").await;
    let download = facts(&operation, &original);
    let after = Editor::read(&state);
    let persisted = saved(&path);
    assert_eq!(
        persisted.baseline, old_file.baseline,
        "{case:?}: root is not saved again"
    );
    let entries = settlement_entries(&state);
    assert_eq!(entries.len(), 1, "{case:?}: one actual terminal settlement");
    let receipt = if case.abort_persistence {
        assert_eq!(entries[0].severity, crate::session_log::Severity::Error);
        assert_eq!(
            entries[0].message, "project history storage: integer overflow",
            "{case:?}: exact injected SQLite failure, not stale/schema/source refusal"
        );
        assert_eq!(
            after, before,
            "{case:?}: no candidate/history/revision publication"
        );
        assert_eq!(
            persisted, old_file,
            "{case:?}: every logical file row and schema unchanged"
        );
        "persistenceFailed"
    } else {
        assert_eq!(entries[0].severity, crate::session_log::Severity::Info);
        let expected = if case.parameter_edit {
            [true, true, false, false, true]
        } else {
            [true; 5]
        };
        assert_eq!(
            flags(&state),
            expected,
            "{case:?}: positive source-comparison control"
        );
        assert_ne!(
            after.snapshot, before.snapshot,
            "{case:?}: candidate is observable"
        );
        assert!(
            after.undo != before.undo || after.redo != before.redo,
            "{case:?}"
        );
        assert_eq!(after.revision, before.revision + 1, "{case:?}");
        assert_eq!(after.generation, before.generation + 1, "{case:?}");
        for project in after.undo.iter().chain(&after.redo) {
            assert_eq!(
                project_flags(project),
                [false, false, false, false, true],
                "{case:?}: no touched old claim can resurrect"
            );
        }
        assert_saved_editor(&persisted, &after, case);
        "persisted"
    };
    observe(&router, &id, receipt, download, case).await;
    assert_eq!(
        Editor::read(&state),
        after,
        "{case:?}: status/report do not settle"
    );
    assert_eq!(
        saved(&path),
        persisted,
        "{case:?}: reads do not rewrite file"
    );
    if case.abort_persistence {
        remove_fault(&path);
    }
    // Retrying GET after the store works again must NOT retry the failed settlement.
    let readable_file = saved(&path);
    observe(&router, &id, receipt, download, case).await;
    assert_eq!(
        Editor::read(&state),
        after,
        "{case:?}: no read-side persistence retry"
    );
    assert_eq!(saved(&path), readable_file, "{case:?}");
    assert_eq!(settlement_entries(&state).len(), 1, "{case:?}");
    assert_eq!(facts(&operation, &original), download, "{case:?}");
    assert_eq!(
        std::fs::read(operation.join("observed.json")).unwrap(),
        observed
    );
    drop(router);
    drop(state);

    // Retire the original operation-root owner before a genuinely fresh service opens it.
    let restarted = Arc::new(crate::AppState::with_product_db_at(
        data.into(),
        Some(products),
    ));
    let progress = crate::domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
    crate::domain::open_native_project(&restarted, &path, true, &progress).unwrap();
    let reopened = Editor::read(&restarted);
    after.assert_content(&reopened, case);
    assert_eq!(reopened.generation, after.generation, "{case:?}");
    assert_saved_editor(&saved(&path), &reopened, case);
    let router = crate::app(restarted.clone(), None);
    observe(&router, &id, "notRecorded", download, case).await;
    assert_eq!(
        Editor::read(&restarted),
        reopened,
        "{case:?}: fresh open/reads never settle"
    );
    assert_eq!(
        saved(&path),
        readable_file,
        "{case:?}: reopen preserves old working file"
    );
    assert!(settlement_entries(&restarted).is_empty(), "{case:?}");
    assert_eq!(facts(&operation, &original), download, "{case:?}");
    assert_eq!(
        std::fs::read(operation.join("observed.json")).unwrap(),
        observed
    );
    if case.abort_persistence {
        // These are explicit project edits only AFTER all no-replay/file oracles.
        exercise_preserved_history(&restarted, &before, case);
    }
}
async fn matrix(retained: bool, abort_persistence: bool) {
    for namespace in [20, 23] {
        for marked in [false, true] {
            for parameter_edit in [false, true] {
                run(Case {
                    namespace,
                    retained,
                    marked,
                    parameter_edit,
                    abort_persistence,
                })
                .await;
            }
        }
    }
}

#[tokio::test]
async fn caption_mounted_retained_persistence_abort_preserves_editor_and_saved_history() {
    matrix(true, true).await;
}
#[tokio::test]
async fn caption_mounted_standalone_persistence_abort_preserves_editor_and_saved_history() {
    matrix(false, true).await;
}
#[tokio::test]
async fn caption_mounted_retained_persistence_control_publishes_only_after_save() {
    matrix(true, false).await;
}
#[tokio::test]
async fn caption_mounted_standalone_persistence_control_publishes_only_after_save() {
    matrix(false, false).await;
}
