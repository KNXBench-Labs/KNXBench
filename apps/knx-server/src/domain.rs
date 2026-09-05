// apps/knx-server/src/domain.rs
//! Moved from `apps/knx-desktop/src-tauri/src/lib.rs` (the web/Docker
//! deployment target, see the design doc linked from the plan this task
//! belongs to) — these functions know nothing about Tauri or HTTP, only
//! `knx-core`/`knx-app`/`knx-store`/`knx-etsproj`/`knx-projection`.
//! `routes.rs`/`fs_routes.rs` are the only places that translate them to
//! JSON.
//!
//! Two independent file formats meet here, same as before the move:
//! `open_project`/`open_project_impl` import an ETS `.knxproj` (always
//! through a throwaway in-memory store — never touches a `.knxdb` file);
//! `save_project`/`save_project_as`/`open_native_project` persist/restore
//! this app's own project state as a `.knxdb` SQLite file. Neither path
//! calls into the other.

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
    /// Root directory web-originated file access is confined to:
    /// `fs_routes.rs`'s `/api/fs/*` routes entirely, plus any *relative*
    /// path a `/api/project/*` route is given (`crate::paths`). Absolute
    /// paths — what the Tauri build's native dialogs return — bypass it by
    /// design. Irrelevant to every function in this file: the route layer
    /// has already resolved the path by the time it calls in here.
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            project: Mutex::new(None),
            store_path: Mutex::new(None),
            command_stack: Mutex::new(knx_core::CommandStack::new()),
            import_counts: Mutex::new((0, 0)),
            data_dir,
        }
    }
}

impl Default for AppState {
    /// Test-only convenience — production always calls `AppState::new`
    /// with `KNX_DATA_DIR`. Falls back to the OS temp dir so tests that
    /// never touch `/api/fs/*` don't need to care.
    fn default() -> Self {
        Self::new(std::env::temp_dir())
    }
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

/// Shared by `open_project_impl` (display-only) and `open_project`
/// (display + replaces `state`'s project) so there is exactly one import
/// implementation instead of two.
fn import_and_project(path: &Path) -> Result<(ProjectTree, knx_core::Project), AppError> {
    let conn = knx_store::open_and_migrate_in_memory()?;
    let imported = knx_app::import_ets_project_with(path, &conn, ImportOptions::default())?;
    let mut tree = knx_projection::build_project_tree(&imported.project);
    apply_report_counts(&mut tree, &imported.report);
    Ok((tree, imported.project))
}

/// Imports `path` and projects it without touching `state` — what
/// `open_reference_project.rs` exercises directly, no server needed.
pub fn open_project_impl(path: &Path) -> Result<ProjectTree, AppError> {
    import_and_project(path).map(|(tree, _)| tree)
}

/// Imports `path`, replaces `state`'s project, and resets undo history and
/// import counts — what the `/api/project/import` route calls.
pub fn open_project(state: &AppState, path: &Path) -> Result<ProjectTree, String> {
    let (tree, project) = import_and_project(path).map_err(|e| e.to_string())?;
    *state.project.lock().expect("state mutex poisoned") = Some(project);
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (tree.errors, tree.warnings);
    Ok(tree)
}

/// Persists `project` to a fresh or existing `.knxdb` file at `path`,
/// overwriting whatever it held.
pub fn save_project_as_impl(path: &Path, project: &knx_core::Project) -> Result<(), String> {
    let conn = knx_store::open_and_migrate(path).map_err(|e| e.to_string())?;
    knx_store::save_project(&conn, project).map_err(|e| e.to_string())
}

/// Shared by `open_native_project_impl` (display-only) and
/// `open_native_project` (display + replaces `state`'s project) — same
/// reasoning as `import_and_project` above.
fn load_native(path: &Path) -> Result<(ProjectTree, knx_core::Project), String> {
    let conn = knx_store::open_and_migrate(path).map_err(|e| e.to_string())?;
    let project = knx_store::load_project(&conn).map_err(|e| e.to_string())?;
    let tree = knx_projection::build_project_tree(&project);
    Ok((tree, project))
}

/// Loads a `.knxdb` file at `path` and projects it, without touching
/// `state`. No `ImportReport` exists for a native load — nothing was
/// reinterpreted from an external format — so `tree.errors`/`tree.warnings`
/// stay at their default zero.
pub fn open_native_project_impl(path: &Path) -> Result<ProjectTree, String> {
    load_native(path).map(|(tree, _)| tree)
}

/// Loads a `.knxdb` file at `path`, replaces `state`'s project, and points
/// `store_path` at it — what the `/api/project/open` route calls.
pub fn open_native_project(state: &AppState, path: &Path) -> Result<ProjectTree, String> {
    let (tree, project) = load_native(path)?;
    *state.project.lock().expect("state mutex poisoned") = Some(project);
    *state.store_path.lock().expect("state mutex poisoned") = Some(path.to_path_buf());
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (0, 0);
    Ok(tree)
}

pub fn save_project_as(state: &AppState, path: &Path) -> Result<(), String> {
    {
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or("no project open")?;
        save_project_as_impl(path, project)?;
    }
    *state.store_path.lock().expect("state mutex poisoned") = Some(path.to_path_buf());
    Ok(())
}

pub fn save_project(state: &AppState) -> Result<(), String> {
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

pub fn device_detail(
    state: &AppState,
    device_id: u32,
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
/// project's own `GroupAddressStyle`. `entry.source` is empty: a
/// UI-created object has no ETS origin to preserve.
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
