//! The Tauri shell. Holds the imported project in memory
//! (`Mutex<Option<Project>>`) and exposes it to the frontend only as
//! display-shaped projections (ADR-0009) — `open_project` is the one
//! command this cycle needs.

use std::path::Path;
use std::sync::Mutex;

use knx_app::{AppError, ImportOptions};
use knx_projection::ProjectTree;

pub struct AppState {
    pub project: Mutex<Option<knx_core::Project>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project: Mutex::new(None),
        }
    }
}

/// Imports `path` and projects it, without touching any Tauri machinery —
/// this is what both the `open_project` command and the integration test
/// call, so the test needs no running `tauri::App` at all.
///
/// The store connection is in-memory and discarded when it drops
/// (`knx_store::open_and_migrate_in_memory`, Task 1): this cycle only
/// displays a project, it does not save or reload knx-desktop's own state.
pub fn open_project_impl(path: &Path) -> Result<ProjectTree, AppError> {
    let conn = knx_store::open_and_migrate_in_memory()?;
    let imported = knx_app::import_ets_project_with(path, &conn, ImportOptions::default())?;

    let mut tree = knx_projection::build_project_tree(&imported.project);
    apply_report_counts(&mut tree, &imported.report);

    Ok(tree)
}

#[tauri::command]
fn open_project(path: String, state: tauri::State<AppState>) -> Result<ProjectTree, String> {
    let (tree, project) = {
        let conn = knx_store::open_and_migrate_in_memory().map_err(|e| e.to_string())?;
        let imported =
            knx_app::import_ets_project_with(Path::new(&path), &conn, ImportOptions::default())
                .map_err(|e| e.to_string())?;
        let mut tree = knx_projection::build_project_tree(&imported.project);
        apply_report_counts(&mut tree, &imported.report);
        (tree, imported.project)
    };
    *state.project.lock().expect("state mutex poisoned") = Some(project);
    Ok(tree)
}

/// Fills in `tree.errors`/`tree.warnings` from `report`, keeping a genuine
/// `Severity::Error` (data actually lost or misread) distinct from
/// everything else merely worth a look — the same split
/// `apps/knx-cli/src/main.rs`'s `error_count()` draws, since `ImportReport`
/// keeps both severities in one `errors` Vec.
fn apply_report_counts(tree: &mut ProjectTree, report: &knx_etsproj::ImportReport) {
    let error_count = report
        .errors
        .iter()
        .filter(|e| e.severity == knx_etsproj::report::Severity::Error)
        .count();
    let warning_count = report.errors.len() - error_count;

    tree.errors = error_count;
    tree.warnings =
        warning_count + report.unknown.len() + report.conflicts.len() + report.unsupported.len();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![open_project])
        .run(tauri::generate_context!())
        .expect("error while running knx-desktop");
}
