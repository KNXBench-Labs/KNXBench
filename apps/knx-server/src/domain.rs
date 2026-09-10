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

use std::fmt;
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
    let (tree, project) = import_and_project(path, guard.as_deref()).map_err(|e| e.to_string())?;
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

/// Parses the wire-format flag name (`"Read"`, `"Write"`, `"Transmit"`,
/// `"Update"`, `"Communication"` — `ComFlagKind`'s own `Debug` form) the
/// same way `parse_direction` parses `"Send"`/`"Receive"` for group links.
fn parse_com_flag_kind(flag: &str) -> Result<knx_core::ComFlagKind, String> {
    match flag {
        "Read" => Ok(knx_core::ComFlagKind::Read),
        "Write" => Ok(knx_core::ComFlagKind::Write),
        "Transmit" => Ok(knx_core::ComFlagKind::Transmit),
        "Update" => Ok(knx_core::ComFlagKind::Update),
        "Communication" => Ok(knx_core::ComFlagKind::Communication),
        other => Err(format!(
            "unknown com-object flag '{other}', expected one of Read/Write/Transmit/Update/Communication"
        )),
    }
}

pub fn set_com_object_flag_impl(
    state: &AppState,
    com_object_id: u32,
    flag: String,
    value: bool,
) -> Result<knx_projection::ProjectTree, String> {
    let flag = parse_com_flag_kind(&flag)?;
    apply(
        state,
        knx_core::Command::SetComObjectFlag {
            com_object: knx_core::ComObjectInstanceId(com_object_id),
            flag,
            value,
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

pub fn catalog_manufacturers_impl(
    state: &AppState,
) -> Result<Vec<(String, Option<String>)>, String> {
    let products = state
        .product_db
        .as_ref()
        .ok_or("no product database configured")?
        .lock()
        .expect("state mutex poisoned");
    knx_productdb::query::manufacturers(&products).map_err(|e| e.to_string())
}

pub fn catalog_items_impl(
    state: &AppState,
    manufacturer: Option<String>,
    search: Option<String>,
) -> Result<Vec<knx_productdb::query::CatalogItemRow>, String> {
    let products = state
        .product_db
        .as_ref()
        .ok_or("no product database configured")?
        .lock()
        .expect("state mutex poisoned");
    knx_productdb::query::catalog_items(&products, manufacturer.as_deref(), search.as_deref())
        .map_err(|e| e.to_string())
}

/// Installs one standalone manufacturer package into the shared catalog.
/// Multipart parsing and response serialization remain at the HTTP boundary.
#[derive(Debug)]
pub enum CatalogInstallError {
    BadRequest(knx_productdb::PackageError),
    Internal(String),
}

impl fmt::Display for CatalogInstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRequest(error) => error.fmt(f),
            Self::Internal(error) => f.write_str(error),
        }
    }
}

pub fn install_catalog_package_impl(
    state: &AppState,
    source_name: &str,
    bytes: &[u8],
) -> Result<knx_productdb::InstallReport, CatalogInstallError> {
    let products = state
        .product_db
        .as_ref()
        .ok_or_else(|| CatalogInstallError::Internal("no product database configured".into()))?
        .lock()
        .expect("state mutex poisoned");
    knx_productdb::install_package(&products, source_name, bytes).map_err(|error| match &error {
        knx_productdb::PackageError::Database(
            knx_productdb::ProductDbError::Sqlite(_)
            | knx_productdb::ProductDbError::FutureVersion { .. },
        ) => CatalogInstallError::Internal(error.to_string()),
        _ => CatalogInstallError::BadRequest(error),
    })
}

/// A catalog-created device and every fact the caller needs to present before
/// closing the catalog dialog.  The domain command remains the sole mutation;
/// diagnostics explain the product-data evidence around it.
#[derive(Debug)]
pub struct CreateDeviceResponse {
    pub tree: ProjectTree,
    pub diagnostics: Vec<CreationDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreationDiagnostic {
    ProgramlessProduct {
        catalog_item_id: String,
    },
    AmbiguousDpt {
        ref_id: String,
        alternatives: Vec<String>,
    },
    ComObjectRefMissing {
        ref_id: String,
    },
    /// A device-level enrichment pass (unlike this create's own upfront
    /// `resolve_catalog_item_program` chain check) found its program
    /// reference unresolvable. Kept distinct from `ComObjectRefMissing` —
    /// conflating a missing *program* with a missing *com-object* would
    /// mislabel the problem for anyone reading the diagnostic.
    ProgramRefMissing {
        program_ref: String,
    },
    /// Manufacturer product programs can contain `Dynamic` and module
    /// activation semantics.  This static seed deliberately does not infer
    /// either; the warning makes that boundary visible for every such create.
    DynamicOrModuleNotEvaluated {
        program_id: String,
    },
}

impl CreationDiagnostic {
    fn from_enrichment(issue: knx_productdb::EnrichmentIssue) -> Self {
        match issue {
            knx_productdb::EnrichmentIssue::ProgramMissing { program_ref, .. } => {
                Self::ProgramRefMissing { program_ref }
            }
            knx_productdb::EnrichmentIssue::ComObjectRefMissing { ref_id, .. } => {
                Self::ComObjectRefMissing { ref_id }
            }
            knx_productdb::EnrichmentIssue::AmbiguousDpt {
                ref_id,
                alternatives,
            } => Self::AmbiguousDpt {
                ref_id,
                alternatives,
            },
        }
    }

    /// Ready-to-display wording for API consumers that don't want to build
    /// their own sentence from the structured fields (design doc §"the
    /// creation response carries ... structured diagnostics"; mirrors the
    /// `detail` convention `ProductDbError::Package` already uses for
    /// package-install errors). `CatalogBrowser.tsx` prefers this over its
    /// own client-side formatting.
    pub fn detail(&self) -> String {
        match self {
            Self::ProgramlessProduct { .. } => {
                "This product explicitly has no application program; it was created without communication objects.".to_string()
            }
            Self::AmbiguousDpt {
                ref_id,
                alternatives,
            } => format!(
                "No DPT was inferred for {ref_id}; alternatives: {}.",
                alternatives.join(", ")
            ),
            Self::ComObjectRefMissing { ref_id } => format!(
                "Communication-object reference is missing from the installed program: {ref_id}."
            ),
            Self::ProgramRefMissing { program_ref } => format!(
                "The installed application program reference is missing: {program_ref}."
            ),
            Self::DynamicOrModuleNotEvaluated { program_id } => format!(
                "Dynamic and module activation was not evaluated for {program_id}; only static product data was seeded."
            ),
        }
    }
}

/// Creates a device from a product-database catalog entry (design doc §3).
/// Only a product whose hardware explicitly declares it programless may be
/// created without a program; every dangling catalog relation is rejected.
pub fn create_device_impl(
    state: &AppState,
    line_id: Option<u32>,
    catalog_item_id: String,
    name: String,
) -> Result<CreateDeviceResponse, String> {
    // Step 1 (design doc §3.1): everything the product database can tell
    // us, gathered while only `product_db` is locked — dropped before
    // `project` is locked below, so the two mutexes are never held at
    // once.
    let (product_ref, program_ref, seeds, mut diagnostics) = {
        let products = state
            .product_db
            .as_ref()
            .ok_or("no product database configured")?
            .lock()
            .expect("state mutex poisoned");
        let item = knx_productdb::query::catalog_item(&products, &catalog_item_id)
            .map_err(|e| e.to_string())?
            .ok_or("catalog item not found")?;
        let mut seeds: Vec<(String, knx_productdb::query::ComObjectView)> = Vec::new();
        let mut diagnostics = Vec::new();
        let (product_ref, program_ref) =
            match knx_productdb::query::resolve_catalog_item_program(&products, &item)
                .map_err(|error| error.to_string())?
            {
                knx_productdb::query::CatalogItemProgram::Program {
                    product_ref_id,
                    hardware2program_ref_id,
                    program_id,
                } => {
                    for ref_id in knx_productdb::query::com_object_ref_ids(&products, &program_id)
                        .map_err(|e| e.to_string())?
                    {
                        let view =
                            knx_productdb::query::com_object_view(&products, &program_id, &ref_id)
                                .map_err(|e| e.to_string())?
                                .ok_or_else(|| {
                                    format!(
                                "catalog communication-object reference is missing: {ref_id}"
                            )
                                })?;
                        seeds.push((ref_id, view));
                    }
                    diagnostics
                        .push(CreationDiagnostic::DynamicOrModuleNotEvaluated { program_id });
                    (product_ref_id, hardware2program_ref_id)
                }
                knx_productdb::query::CatalogItemProgram::Programless { product_ref_id } => {
                    diagnostics.push(CreationDiagnostic::ProgramlessProduct {
                        catalog_item_id: item.id,
                    });
                    (product_ref_id, String::new())
                }
            };
        (product_ref, program_ref, seeds, diagnostics)
    };

    // Step 2 (design doc §3.2): allocate ids and build the command,
    // holding `project`'s own lock continuously through step 3 below —
    // `product_db` is no longer held.
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;

    let device_id = project.ids.next_device_id();
    let mut com_objects = Vec::with_capacity(seeds.len());
    let mut enrich_inputs = Vec::with_capacity(seeds.len());
    for (ref_id, view) in &seeds {
        let com_id = project.ids.next_com_object_instance_id();
        com_objects.push(knx_core::ComObjectInstance {
            id: com_id,
            source: knx_core::SourceRef {
                path: ref_id.clone(),
                ets_id: ref_id.clone(),
            },
            device: device_id,
            number: view.number.unwrap_or(0) as u16,
            text: knx_core::Override::Absent,
            description: knx_core::Override::Absent,
            dpt: knx_core::Override::Absent,
            flags: knx_core::ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        enrich_inputs.push((com_id, ref_id.clone(), view.clone()));
    }
    let device = knx_core::DeviceInstance {
        id: device_id,
        source: knx_core::SourceRef {
            path: format!("KB-DEV-{}", device_id.0),
            ets_id: format!("KB-DEV-{}", device_id.0),
        },
        name,
        description: None,
        address: None,
        product_ref,
        program_ref,
        commissioning: knx_core::CommissioningState::default(),
        visibility_calculated: true,
        com_objects: com_objects.iter().map(|c| c.id).collect(),
        binary_data: vec![],
    };
    let cmd = knx_core::Command::CreateDevice {
        device,
        com_objects,
        line: line_id.map(knx_core::LineId),
    };
    {
        let mut stack = state.command_stack.lock().expect("state mutex poisoned");
        stack.do_command(project, cmd).map_err(|e| e.to_string())?;
    }

    // Step 3 (design doc §3.3): seed enrichment once, same mapping
    // `knx_productdb::enrich()` uses on import, not pushed onto the undo
    // stack — undoing `CreateDevice` removes the device regardless of
    // which slots got filled, and `DeleteDevice`'s own inverse captures
    // the enriched state for redo (Task 1). `issues` (ambiguous DPT
    // lists) are returned as creation diagnostics.
    let mut issues = Vec::new();
    for (com_id, ref_id, view) in &enrich_inputs {
        knx_productdb::enrich::apply(project, *com_id, ref_id, view, &mut issues);
    }

    diagnostics.extend(issues.into_iter().map(CreationDiagnostic::from_enrichment));
    let stack = state.command_stack.lock().expect("state mutex poisoned");
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    Ok(CreateDeviceResponse {
        tree: tree_with_state(project, &stack, import_counts),
        diagnostics,
    })
}

pub fn delete_device_impl(
    state: &AppState,
    id: u32,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::DeleteDevice {
            id: knx_core::DeviceId(id),
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

    // The `product_db = None` line below is a deliberate, documented
    // reassignment (see its own comment) — not an oversight clippy should
    // fold into a `..Default::default()` struct literal.
    #[allow(clippy::field_reassign_with_default)]
    fn state_with_one_installation() -> AppState {
        let mut project = knx_core::Project::new(knx_core::Language("en".into()));
        project.installations.push(knx_core::Installation {
            id: knx_core::InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: knx_core::CompletionStatus::FinishedDesign,
            topology: knx_core::Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        });
        let mut state = AppState::default();
        // `AppState::default()` now runs the real `default_path()` lookup
        // (Task 3) — on a machine that already has a product database at
        // e.g. `~/.local/share/knx/products.sqlite`, `default_path()`
        // would pick it up here, making
        // `creating_a_device_without_a_product_database_is_an_error`
        // depend on the environment. Force `None` explicitly so this
        // helper's guarantee ("no product database configured") holds
        // everywhere, not just on a machine without one.
        state.product_db = None;
        *state.project.lock().unwrap() = Some(project);
        state
    }

    const CATALOG_HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

    const CATALOG_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1"
  MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Schalten" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" WriteFlag="Enabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    const CATALOG_ITEM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-1">
      <Catalog>
        <CatalogSection Id="M-1_CG-1" Name="Actuators" Number="1" DefaultLanguage="de-DE">
          <CatalogItem Id="M-1_CI-1" Name="Schaltaktor" Number="ACT-1"
                       DefaultLanguage="de-DE"
                       ProductRefId="M-1_P-1"
                       Hardware2ProgramRefId="H-1_HP-1" />
        </CatalogSection>
      </Catalog>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    /// Returns the backing `TempDir` alongside the state — same shape as
    /// `knx-productdb`'s own `db()` test helpers — so the temp file isn't
    /// deleted out from under the connection while the test still needs
    /// it.
    fn state_with_product_db() -> (tempfile::TempDir, AppState) {
        let state = state_with_one_installation();
        let dir = tempfile::tempdir().unwrap();
        let products =
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        knx_productdb::parse::hardware::ingest_hardware(
            &products,
            "sha-h",
            "M-1/Hardware.xml",
            CATALOG_HARDWARE.as_bytes(),
        )
        .unwrap();
        knx_productdb::parse::program::ingest_program(
            &products,
            "sha-p",
            "M-1/A.xml",
            CATALOG_PROGRAM.as_bytes(),
        )
        .unwrap();
        knx_productdb::parse::catalog::ingest_catalog(
            &products,
            "sha-c",
            "M-1/Catalog.xml",
            CATALOG_ITEM.as_bytes(),
        )
        .unwrap();
        let mut state = state;
        state.product_db = Some(Mutex::new(products));
        (dir, state)
    }

    #[test]
    fn creating_a_device_without_a_product_database_is_an_error() {
        let state = state_with_one_installation();
        let result = create_device_impl(&state, None, "anything".into(), "D".into());
        assert_eq!(result.unwrap_err(), "no product database configured");
    }

    #[test]
    fn creating_a_device_with_an_unknown_catalog_item_is_an_error() {
        let (_dir, state) = state_with_product_db();
        let result = create_device_impl(&state, None, "nope".into(), "D".into());
        assert_eq!(result.unwrap_err(), "catalog item not found");
    }

    #[test]
    fn creating_a_device_seeds_its_com_objects_and_deleting_it_round_trips() {
        let (_dir, state) = state_with_product_db();
        let response =
            create_device_impl(&state, None, "M-1_CI-1".into(), "Actuator 1".into()).unwrap();
        assert_eq!(
            response.tree.installations[0].unassigned.len(),
            1,
            "the new device lands in unassigned when no line is given"
        );
        let device_id = response.tree.installations[0].unassigned[0].id;

        let detail = device_detail(&state, device_id).unwrap();
        assert_eq!(detail.com_objects.len(), 1);
        assert!(
            detail.com_objects[0].dpt.is_some(),
            "the com object was enriched at creation time from the product database"
        );

        let tree = delete_device_impl(&state, device_id).unwrap();
        assert!(tree.installations[0].unassigned.is_empty());
    }

    // `enrich::apply` (the only source feeding `create_device_impl`'s own
    // `from_enrichment` call) never emits `EnrichmentIssue::ProgramMissing`
    // today — only the top-level `enrich()` pass does. Test the mapping
    // directly and totally anyway, so a `ProgramMissing` issue can never
    // silently collapse back onto `ComObjectRefMissing` if `from_enrichment`
    // is ever reused against that pass's `EnrichmentReport.issues`.
    #[test]
    fn from_enrichment_keeps_program_and_com_object_issues_distinct() {
        let program_missing =
            CreationDiagnostic::from_enrichment(knx_productdb::EnrichmentIssue::ProgramMissing {
                device_ets_id: "KB-DEV-1".into(),
                program_ref: "A-1".into(),
            });
        assert_eq!(
            program_missing,
            CreationDiagnostic::ProgramRefMissing {
                program_ref: "A-1".into(),
            }
        );
        assert_eq!(
            program_missing.detail(),
            "The installed application program reference is missing: A-1."
        );

        let com_object_missing = CreationDiagnostic::from_enrichment(
            knx_productdb::EnrichmentIssue::ComObjectRefMissing {
                device_ets_id: "KB-DEV-1".into(),
                ref_id: "A-1_O-1_R-1".into(),
            },
        );
        assert_eq!(
            com_object_missing,
            CreationDiagnostic::ComObjectRefMissing {
                ref_id: "A-1_O-1_R-1".into(),
            }
        );
        assert_ne!(program_missing, com_object_missing);
    }
}
