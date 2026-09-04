//! The Tauri shell. Holds the imported project in memory
//! (`Mutex<Option<Project>>`) and exposes it to the frontend only as
//! display-shaped projections (ADR-0009).
//!
//! Two independent file formats meet here: `open_project` imports an ETS
//! `.knxproj` (always through a throwaway in-memory store — Task 1's
//! comment on `open_project_impl` still holds, ETS import never touches a
//! `.knxdb` file); `save_project`/`save_project_as`/`open_native_project`
//! persist/restore knx-desktop's own project state as a `.knxdb` SQLite
//! file via `knx_store::{save_project, load_project}`. Neither path calls
//! into the other.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use knx_app::{AppError, ImportOptions};
use knx_projection::ProjectTree;

pub struct AppState {
    pub project: Mutex<Option<knx_core::Project>>,
    /// The `.knxdb` file the in-memory project was last saved to or loaded
    /// from, if any. `None` until `save_project_as`/`open_native_project`
    /// sets it; plain `save_project` requires it already set.
    pub store_path: Mutex<Option<PathBuf>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project: Mutex::new(None),
            store_path: Mutex::new(None),
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

/// Persists `project` to a fresh or existing `.knxdb` file at `path`,
/// overwriting whatever it held. Split out from the `save_project_as`
/// command so a test can round-trip it without any Tauri machinery, the
/// same reasoning as `open_project_impl`.
pub fn save_project_as_impl(path: &Path, project: &knx_core::Project) -> Result<(), String> {
    let conn = knx_store::open_and_migrate(path).map_err(|e| e.to_string())?;
    knx_store::save_project(&conn, project).map_err(|e| e.to_string())
}

/// Loads a `.knxdb` file at `path` and projects it. No `ImportReport`
/// exists for a native load — nothing was reinterpreted from an external
/// format — so `tree.errors`/`tree.warnings` stay at their default zero.
pub fn open_native_project_impl(path: &Path) -> Result<ProjectTree, String> {
    let conn = knx_store::open_and_migrate(path).map_err(|e| e.to_string())?;
    let project = knx_store::load_project(&conn).map_err(|e| e.to_string())?;
    Ok(knx_projection::build_project_tree(&project))
}

/// Projects one device's detail. `Err` names the device id when it no
/// longer exists in `project` — a stale selection after an edit, for
/// instance.
pub fn device_detail_impl(
    project: &knx_core::Project,
    device_id: u32,
) -> Result<knx_projection::DeviceDetail, String> {
    knx_projection::build_device_detail(project, knx_core::DeviceId(device_id))
        .ok_or_else(|| format!("device {device_id} not found"))
}

#[tauri::command]
fn device_detail(
    device_id: u32,
    state: tauri::State<AppState>,
) -> Result<knx_projection::DeviceDetail, String> {
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_ref().ok_or("no project open")?;
    device_detail_impl(project, device_id)
}

#[tauri::command]
fn save_project_as(path: String, state: tauri::State<AppState>) -> Result<(), String> {
    let path = PathBuf::from(path);
    {
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or("no project open")?;
        save_project_as_impl(&path, project)?;
    }
    *state.store_path.lock().expect("state mutex poisoned") = Some(path);
    Ok(())
}

#[tauri::command]
fn save_project(state: tauri::State<AppState>) -> Result<(), String> {
    let path = state
        .store_path
        .lock()
        .expect("state mutex poisoned")
        .clone()
        .ok_or("no save location yet — use Save As")?;
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_ref().ok_or("no project open")?;
    save_project_as_impl(&path, project)
}

#[tauri::command]
fn open_native_project(path: String, state: tauri::State<AppState>) -> Result<ProjectTree, String> {
    let path = PathBuf::from(path);
    let conn = knx_store::open_and_migrate(&path).map_err(|e| e.to_string())?;
    let project = knx_store::load_project(&conn).map_err(|e| e.to_string())?;
    let tree = knx_projection::build_project_tree(&project);
    *state.project.lock().expect("state mutex poisoned") = Some(project);
    *state.store_path.lock().expect("state mutex poisoned") = Some(path);
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
        .invoke_handler(tauri::generate_handler![
            open_project,
            save_project,
            save_project_as,
            open_native_project,
            device_detail
        ])
        .run(tauri::generate_context!())
        .expect("error while running knx-desktop");
}
