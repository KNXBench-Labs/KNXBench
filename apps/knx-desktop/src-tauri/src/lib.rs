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
    /// Every applied command's inverse, for undo/redo. Reset to empty on
    /// `open_project`/`open_native_project` — undo history never survives
    /// loading a different project, and is never persisted to `.knxdb`.
    pub command_stack: Mutex<knx_core::CommandStack>,
    /// (errors, warnings) from the initial import's `ImportReport`,
    /// reapplied to every tree rebuilt after a command/undo/redo — edits
    /// don't change what import lost. `(0, 0)` for a `.knxdb` native load.
    pub import_counts: Mutex<(usize, usize)>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project: Mutex::new(None),
            store_path: Mutex::new(None),
            command_stack: Mutex::new(knx_core::CommandStack::new()),
            import_counts: Mutex::new((0, 0)),
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
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (tree.errors, tree.warnings);
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

/// Rebuilds `tree` from `project` and overlays the counts/undo-redo state
/// that `build_project_tree` alone cannot know about.
fn tree_with_state(
    project: &knx_core::Project,
    stack: &knx_core::CommandStack,
    import_counts: (usize, usize),
) -> knx_projection::ProjectTree {
    let mut tree = knx_projection::build_project_tree(project);
    tree.errors = import_counts.0;
    tree.warnings = import_counts.1;
    tree.can_undo = stack.can_undo();
    tree.can_redo = stack.can_redo();
    tree
}

fn apply(state: &AppState, cmd: knx_core::Command) -> Result<knx_projection::ProjectTree, String> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    stack.do_command(project, cmd).map_err(|e| e.to_string())?;
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    Ok(tree_with_state(project, &stack, import_counts))
}

pub fn set_individual_address_impl(
    state: &AppState,
    device_id: u32,
    address: Option<String>,
) -> Result<knx_projection::ProjectTree, String> {
    let address = match address {
        Some(s) => Some(
            s.parse::<knx_core::IndividualAddress>()
                .map_err(|e| e.to_string())?,
        ),
        None => None,
    };
    apply(
        state,
        knx_core::Command::SetIndividualAddress {
            device: knx_core::DeviceId(device_id),
            address,
        },
    )
}

pub fn set_com_object_dpt_impl(
    state: &AppState,
    com_object_id: u32,
    dpt: Option<String>,
) -> Result<knx_projection::ProjectTree, String> {
    let dpt = match dpt {
        Some(s) => Some(knx_core::DptRef::parse(&s).map_err(|e| e.to_string())?),
        None => None,
    };
    apply(
        state,
        knx_core::Command::SetComObjectDpt {
            com_object: knx_core::ComObjectInstanceId(com_object_id),
            dpt,
        },
    )
}

/// Allocates a fresh `GroupAddressId` and creates a new group address in
/// `installations[0]` — the only installation any `Command` targets
/// (`Command::apply`'s own doc comment). `address` is parsed against the
/// project's own `GroupAddressStyle` (`ProjectInfo::group_address_style`),
/// the same style `GroupAddressNode`'s `address` string was formatted with.
/// `entry.source` is empty: this is the first UI-created domain object in
/// this codebase, with no ETS origin to preserve.
pub fn create_group_address_impl(
    state: &AppState,
    name: String,
    address: String,
) -> Result<knx_projection::ProjectTree, String> {
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let style = project.info.group_address_style;
        let address = knx_core::GroupAddress::parse(&address, style).map_err(|e| e.to_string())?;
        let id = project.ids.next_group_address_id();
        knx_core::Command::CreateGroupAddress {
            entry: knx_core::GroupAddressEntry {
                id,
                source: knx_core::SourceRef {
                    path: String::new(),
                    ets_id: String::new(),
                },
                name,
                address,
                central: false,
                unfiltered: false,
                range: None,
            },
        }
    };
    apply(state, cmd)
}

pub fn delete_group_address_impl(
    state: &AppState,
    id: u32,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::DeleteGroupAddress {
            id: knx_core::GroupAddressId(id),
        },
    )
}

pub fn undo_impl(state: &AppState) -> Result<knx_projection::ProjectTree, String> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    stack.undo(project).map_err(|e| e.to_string())?;
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    Ok(tree_with_state(project, &stack, import_counts))
}

pub fn redo_impl(state: &AppState) -> Result<knx_projection::ProjectTree, String> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    stack.redo(project).map_err(|e| e.to_string())?;
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    Ok(tree_with_state(project, &stack, import_counts))
}

#[tauri::command]
fn set_individual_address(
    device_id: u32,
    address: Option<String>,
    state: tauri::State<AppState>,
) -> Result<knx_projection::ProjectTree, String> {
    set_individual_address_impl(&state, device_id, address)
}

#[tauri::command]
fn set_com_object_dpt(
    com_object_id: u32,
    dpt: Option<String>,
    state: tauri::State<AppState>,
) -> Result<knx_projection::ProjectTree, String> {
    set_com_object_dpt_impl(&state, com_object_id, dpt)
}

#[tauri::command]
fn create_group_address(
    name: String,
    address: String,
    state: tauri::State<AppState>,
) -> Result<knx_projection::ProjectTree, String> {
    create_group_address_impl(&state, name, address)
}

#[tauri::command]
fn delete_group_address(
    id: u32,
    state: tauri::State<AppState>,
) -> Result<knx_projection::ProjectTree, String> {
    delete_group_address_impl(&state, id)
}

#[tauri::command]
fn undo(state: tauri::State<AppState>) -> Result<knx_projection::ProjectTree, String> {
    undo_impl(&state)
}

#[tauri::command]
fn redo(state: tauri::State<AppState>) -> Result<knx_projection::ProjectTree, String> {
    redo_impl(&state)
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
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (0, 0);
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
            device_detail,
            set_individual_address,
            set_com_object_dpt,
            create_group_address,
            delete_group_address,
            undo,
            redo
        ])
        .run(tauri::generate_context!())
        .expect("error while running knx-desktop");
}
