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
    /// The shared product database, opened once at startup from
    /// `knx_productdb::default_path()`. `None` if no path could be
    /// derived, the file doesn't exist yet, or it failed to open/migrate
    /// — never a startup error (ADR-0012's "missing product database is
    /// ordinary, not an error"). `Mutex`, not `RwLock`: every access here
    /// is a handful of `SELECT`s or one `enrich()` pass, never held long
    /// enough for reader/writer contention to matter.
    pub product_db: Option<Mutex<knx_productdb::Connection>>,
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
        let product_db = knx_productdb::default_path()
            .and_then(|path| knx_productdb::open_and_migrate(&path).ok())
            .map(Mutex::new);
        Self {
            project: Mutex::new(None),
            store_path: Mutex::new(None),
            command_stack: Mutex::new(knx_core::CommandStack::new()),
            import_counts: Mutex::new((0, 0)),
            product_db,
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
/// implementation instead of two. `product_db` is locked by the caller —
/// this function only borrows it for the duration of the import call.
fn import_and_project(
    path: &Path,
    product_db: Option<&knx_productdb::Connection>,
) -> Result<(ProjectTree, knx_core::Project), AppError> {
    let conn = knx_store::open_and_migrate_in_memory()?;
    let imported = knx_app::import_ets_project_with(path, &conn, ImportOptions { product_db })?;
    let mut tree = knx_projection::build_project_tree(&imported.project);
    apply_report_counts(&mut tree, &imported.report);
    Ok((tree, imported.project))
}

/// Imports `path` and projects it without touching `state` — what
/// `open_reference_project.rs` exercises directly, no server needed. Never
/// enriched from a product database: there is no `state` here to read one
/// from, and this path exists specifically for a server-free golden test
/// whose counts must stay deterministic.
pub fn open_project_impl(path: &Path) -> Result<ProjectTree, AppError> {
    import_and_project(path, None).map(|(tree, _)| tree)
}

/// Imports `path`, replaces `state`'s project, and resets undo history and
/// import counts — what the `/api/project/import` route calls. Enriched
/// from `state.product_db` when one is configured (the bonus fix this
/// task adds: nothing previously wired a connection in for this path to
/// use, unlike `knx import --product-db` on the CLI).
pub fn open_project(state: &AppState, path: &Path) -> Result<ProjectTree, String> {
    let guard = state
        .product_db
        .as_ref()
        .map(|m| m.lock().expect("state mutex poisoned"));
    let (tree, project) =
        import_and_project(path, guard.as_deref()).map_err(|e| e.to_string())?;
    drop(guard);
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

pub fn set_device_description_impl(
    state: &AppState,
    device_id: u32,
    description: Option<String>,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::SetDeviceDescription {
            device: knx_core::DeviceId(device_id),
            description,
        },
    )
}

pub fn set_com_object_description_impl(
    state: &AppState,
    com_object_id: u32,
    description: Option<String>,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::SetComObjectDescription {
            com_object: knx_core::ComObjectInstanceId(com_object_id),
            description,
        },
    )
}

/// Allocates a fresh `GroupAddressId` and creates a new group address in
/// `installations[0]` — the only installation any `Command` targets
/// (`Command::apply`'s own doc comment). `address` is parsed against the
/// project's own `GroupAddressStyle`. `entry.source` gets a synthetic,
/// stable id (`KB-GA-<id>`) instead of the empty string this used to
/// write — a UI-created entity has no ETS origin to preserve, but an
/// empty `ets_id` produced an invalid, colliding `Id=""` attribute if it
/// ever reached export (KNOWN_LIMITATIONS.md #21). `range_id` stays
/// optional: forcing every UI-created address into a range needs a range
/// *picker* in the UI, which does not exist yet (Sub-Project 2) — until
/// then, a `None` range keeps behaving exactly as before, and a `Some`
/// range is now validated (`Command::CreateGroupAddress`'s own
/// in-range check) rather than trusted blindly.
pub fn create_group_address_impl(
    state: &AppState,
    name: String,
    address: String,
    range_id: Option<u32>,
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
                    path: format!("KB-GA-{}", id.0),
                    ets_id: format!("KB-GA-{}", id.0),
                },
                name,
                address,
                central: false,
                unfiltered: false,
                range: range_id.map(knx_core::GroupRangeId),
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

pub fn create_area_impl(
    state: &AppState,
    name: String,
    address: u8,
) -> Result<knx_projection::ProjectTree, String> {
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let id = project.ids.next_area_id();
        knx_core::Command::CreateArea {
            area: knx_core::Area {
                id,
                source: knx_core::SourceRef {
                    path: format!("KB-Area-{}", id.0),
                    ets_id: format!("KB-Area-{}", id.0),
                },
                name,
                address,
                completion: knx_core::CompletionStatus::Editing,
                lines: vec![],
            },
        }
    };
    apply(state, cmd)
}

pub fn delete_area_impl(state: &AppState, id: u32) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::DeleteArea {
            id: knx_core::AreaId(id),
        },
    )
}

pub fn create_line_impl(
    state: &AppState,
    area_id: u32,
    name: String,
    address: u8,
    medium_ref: String,
) -> Result<knx_projection::ProjectTree, String> {
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let id = project.ids.next_line_id();
        knx_core::Command::CreateLine {
            area: knx_core::AreaId(area_id),
            line: knx_core::Line {
                id,
                source: knx_core::SourceRef {
                    path: format!("KB-Line-{}", id.0),
                    ets_id: format!("KB-Line-{}", id.0),
                },
                name,
                address,
                medium_ref,
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: knx_core::CompletionStatus::Editing,
                devices: vec![],
            },
        }
    };
    apply(state, cmd)
}

pub fn delete_line_impl(state: &AppState, id: u32) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::DeleteLine {
            id: knx_core::LineId(id),
        },
    )
}

pub fn move_device_to_line_impl(
    state: &AppState,
    device_id: u32,
    line_id: Option<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::MoveDeviceToLine {
            device: knx_core::DeviceId(device_id),
            line: line_id.map(knx_core::LineId),
        },
    )
}

pub fn create_group_range_impl(
    state: &AppState,
    name: String,
    start: String,
    end: String,
    parent_id: Option<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let style = project.info.group_address_style;
        let start = knx_core::GroupAddress::parse(&start, style).map_err(|e| e.to_string())?;
        let end = knx_core::GroupAddress::parse(&end, style).map_err(|e| e.to_string())?;
        let id = project.ids.next_group_range_id();
        knx_core::Command::CreateGroupRange {
            range: knx_core::GroupRange {
                id,
                source: knx_core::SourceRef {
                    path: format!("KB-Range-{}", id.0),
                    ets_id: format!("KB-Range-{}", id.0),
                },
                name,
                start,
                end,
                parent: parent_id.map(knx_core::GroupRangeId),
                children: vec![],
            },
        }
    };
    apply(state, cmd)
}

pub fn delete_group_range_impl(
    state: &AppState,
    id: u32,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::DeleteGroupRange {
            id: knx_core::GroupRangeId(id),
        },
    )
}

pub fn rename_group_range_impl(
    state: &AppState,
    id: u32,
    name: String,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::RenameGroupRange {
            id: knx_core::GroupRangeId(id),
            name,
        },
    )
}

fn parse_direction(direction: &str) -> Result<knx_core::Direction, String> {
    match direction {
        "Send" => Ok(knx_core::Direction::Send),
        "Receive" => Ok(knx_core::Direction::Receive),
        other => Err(format!(
            "unknown direction '{other}', expected 'Send' or 'Receive'"
        )),
    }
}

pub fn link_com_object_impl(
    state: &AppState,
    com_object_id: u32,
    ga_id: u32,
    direction: String,
) -> Result<knx_projection::ProjectTree, String> {
    let direction = parse_direction(&direction)?;
    apply(
        state,
        knx_core::Command::LinkComObject {
            com_object: knx_core::ComObjectInstanceId(com_object_id),
            ga: knx_core::GroupAddressId(ga_id),
            direction,
        },
    )
}

pub fn unlink_com_object_impl(
    state: &AppState,
    com_object_id: u32,
    ga_id: u32,
    direction: String,
) -> Result<knx_projection::ProjectTree, String> {
    let direction = parse_direction(&direction)?;
    apply(
        state,
        knx_core::Command::UnlinkComObject {
            com_object: knx_core::ComObjectInstanceId(com_object_id),
            ga: knx_core::GroupAddressId(ga_id),
            direction,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate lives at <root>/apps/knx-server")
            .to_path_buf()
    }

    fn reference_project_path() -> PathBuf {
        workspace_root().join("Unser Zuhause ets4 - 2025-12-15.knxproj")
    }

    #[test]
    fn opening_a_project_through_a_wired_product_db_enriches_more_than_without() {
        let dir = tempfile::tempdir().unwrap();
        let products_path = dir.path().join("products.sqlite");
        {
            // Ingest the reference project's own manufacturer files into a
            // fresh product database — same two-step dance
            // `crates/knx-app/tests/product_db.rs` already uses.
            let throwaway = knx_store::open_and_migrate_in_memory().unwrap();
            let products = knx_productdb::open_and_migrate(&products_path).unwrap();
            knx_app::import_ets_project_with(
                &reference_project_path(),
                &throwaway,
                ImportOptions {
                    product_db: Some(&products),
                },
            )
            .unwrap();
        }

        let (_, without) = import_and_project(&reference_project_path(), None).unwrap();
        let without_filled = without
            .devices
            .com_objects()
            .filter(|c| c.dpt.value().is_some())
            .count();

        let products = knx_productdb::open_and_migrate(&products_path).unwrap();
        let state = AppState {
            product_db: Some(Mutex::new(products)),
            ..AppState::default()
        };
        open_project(&state, &reference_project_path()).unwrap();
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let with_filled = project
            .devices
            .com_objects()
            .filter(|c| c.dpt.value().is_some())
            .count();

        assert!(
            with_filled > without_filled,
            "wiring state.product_db through open_project should enrich at least \
             one more com object's dpt ({with_filled} vs {without_filled})"
        );
    }
}
