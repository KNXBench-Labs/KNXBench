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

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use knx_app::{AppError, ImportOptions};
use knx_projection::ProjectTree;

use crate::bus::{BusSession, GatewayConnector, RealConnector};
use crate::load_progress::{LoadHandle, LoadOperations, LoadPhase};
use crate::session_log::{self, LogEntry, SessionLog, Severity};

pub struct AppState {
    pub project: Mutex<Option<knx_core::Project>>,
    /// The `.knxdb` file the in-memory project was last saved to or loaded
    /// from, if any. `None` until `save_project_as`/`open_native_project`
    /// sets it; plain `save_project` requires it already set.
    pub store_path: Mutex<Option<PathBuf>>,
    /// Opaque passthrough entries + manufacturer manifest carried by the
    /// current project, if any. `open_project` (ETS import) fills this from
    /// the throwaway in-memory store before it's dropped; `open_native_project`
    /// fills it from the `.knxdb` just loaded. Every `save_project`/
    /// `save_project_as` writes it back into the target `.knxdb` — without
    /// this, a server-side ETS import followed by Save As silently produced
    /// a `.knxdb` with empty opaque/manifest tables, since the import-time
    /// connection was never the same one Save touched (see final review of
    /// the export-UI plan, finding B1). `None` for a fresh, never-imported
    /// project.
    pub opaque: Mutex<Vec<knx_store::StoredOpaqueEntry>>,
    pub manufacturer_refs: Mutex<Vec<knx_store::ManufacturerRef>>,
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
    /// In-memory record of import diagnostics and operational feedback for
    /// this server run (T11) — never persisted, never reset by anything
    /// other than a successful `open_project`/`open_native_project`. See
    /// `session_log.rs` for the append rules every call site below follows.
    pub session_log: Mutex<SessionLog>,
    /// Opens a tunnel to a KNXnet/IP gateway (T15, design spec §3 D6) —
    /// `RealConnector` in production, `crate::fake::FakeConnector` in
    /// tests via the `AppState { connector: ..., ..Default::default() }`
    /// struct-update pattern `tests/http_product_install.rs:40-67` already
    /// uses for `product_db`. Not a `Mutex`: the trait object itself is
    /// stateless/`Sync` (it only ever opens tunnels; each open tunnel's own
    /// state lives in `bus_session` below).
    pub connector: Box<dyn GatewayConnector>,
    /// At most one open monitor session (T15, design spec §4.1: "a monitor
    /// session is ... `AppState.bus_session: Mutex<Option<BusSession>>`
    /// holds at most one"). `tokio::sync::Mutex`, not `std::sync::Mutex`
    /// (Task 3 addition, was `std::sync::Mutex` through Task 2, when
    /// nothing read or wrote it yet): `POST /api/bus/write`'s handler
    /// (`bus_routes.rs`) must hold this lock across `BusSession::send`'s
    /// own `.await` — the same tunnel a concurrent `/stop` could otherwise
    /// tear down mid-send — and a `std::sync::MutexGuard` cannot cross an
    /// `.await` (not `Send`). `POST /monitor/start` also holds it for the
    /// duration of `BusSession::start`'s `.await` on purpose: only one
    /// session may ever exist, so serializing concurrent `start` attempts
    /// through this same lock is the correct behaviour, not a cost to
    /// avoid.
    pub bus_session: tokio::sync::Mutex<Option<BusSession>>,
    /// Monotonic source of [`BusSession`] ids (T15 task 3, design spec
    /// §4.1's "session identity": "a `Uuid`-or-incrementing `id`... though
    /// this slice only ever has one [at a time]"). Starts at 1, incremented
    /// on every successful `POST /api/bus/monitor/start`, never reused —
    /// so a client that polls across a stop/restart can tell from
    /// `sessionId` alone that it is looking at a genuinely new session, not
    /// reused bookkeeping for an old one. `AtomicU64`, not behind the
    /// `bus_session` mutex: a new id is read-and-incremented once per
    /// `start`, before a `BusSession` exists to guard it, and this counter
    /// has no other state to stay consistent with.
    pub next_bus_session_id: std::sync::atomic::AtomicU64,
    /// The single project load this server run may have in flight, and
    /// the snapshot `GET /api/project/load-progress` answers with
    /// (ADR-0023). `Arc`, not a plain field: a [`LoadHandle`] outlives the
    /// request that created it — the work runs on a blocking thread the
    /// client is free to stop waiting for.
    pub load_operations: std::sync::Arc<LoadOperations>,
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
            opaque: Mutex::new(Vec::new()),
            manufacturer_refs: Mutex::new(Vec::new()),
            command_stack: Mutex::new(knx_core::CommandStack::new()),
            import_counts: Mutex::new((0, 0)),
            product_db,
            session_log: Mutex::new(SessionLog::default()),
            connector: Box::new(RealConnector::default()),
            bus_session: tokio::sync::Mutex::new(None),
            next_bus_session_id: std::sync::atomic::AtomicU64::new(1),
            load_operations: std::sync::Arc::new(LoadOperations::default()),
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
type ImportedOpaqueData = (
    Vec<knx_store::StoredOpaqueEntry>,
    Vec<knx_store::ManufacturerRef>,
);

fn import_and_project(
    path: &Path,
    product_db: Option<&knx_productdb::Connection>,
    progress: &LoadHandle,
) -> Result<
    (
        ProjectTree,
        knx_core::Project,
        ImportedOpaqueData,
        knx_etsproj::ImportReport,
    ),
    AppError,
> {
    let conn = knx_store::open_and_migrate_in_memory()?;
    let imported =
        knx_app::import_ets_project_observed(path, &conn, ImportOptions { product_db }, progress)?;
    progress.phase(LoadPhase::BuildProjectTree);
    let mut tree = knx_projection::build_project_tree(&imported.project);
    apply_report_counts(&mut tree, &imported.report);
    // The opaque/manifest rows the import just wrote live only in `conn`,
    // which is dropped at the end of this function — read them out now so
    // a later `open_project` can carry them into `state` (see `AppState::opaque`'s
    // doc comment for why this matters).
    let opaque = knx_store::load_opaque(&conn)?;
    let manufacturer_refs = knx_store::load_manufacturer_refs(&conn)?;
    Ok((
        tree,
        imported.project,
        (opaque, manufacturer_refs),
        imported.report,
    ))
}

/// Imports `path` and projects it without touching `state` — what
/// `open_reference_project.rs` exercises directly, no server needed. Never
/// enriched from a product database: there is no `state` here to read one
/// from, and this path exists specifically for a server-free golden test
/// whose counts must stay deterministic.
pub fn open_project_impl(path: &Path) -> Result<ProjectTree, AppError> {
    let progress = detached_progress(crate::load_progress::LoadKind::Import, path);
    let outcome = import_and_project(path, None, &progress).map(|(tree, ..)| tree);
    match &outcome {
        Ok(_) => progress.succeed(),
        Err(e) => progress.fail(e.to_string()),
    }
    outcome
}

/// A progress handle for a caller with nobody watching. Entry points that
/// have no `AppState` — the golden-test import, the native open behind it,
/// the right-hand side of a diff — still owe the pipeline an observer, so
/// they get one wired to a registry no route can see. The handle keeps the
/// registry alive and drops it with itself.
pub(crate) fn detached_progress(kind: crate::load_progress::LoadKind, path: &Path) -> LoadHandle {
    std::sync::Arc::new(LoadOperations::default())
        .begin(kind, file_name_of(path))
        .expect("a fresh registry has no operation in flight")
}

/// The file name a snapshot names its source by — never the full path:
/// the browser renders this, and a path says more about the machine than
/// a progress line needs to.
pub(crate) fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// Imports `path`, replaces `state`'s project, and resets undo history and
/// import counts — what the `/api/project/import` route calls. Enriched
/// from `state.product_db` when one is configured (the bonus fix this
/// task adds: nothing previously wired a connection in for this path to
/// use, unlike `knx import --product-db` on the CLI). Also carries the
/// import's opaque passthrough + manufacturer manifest into `state.opaque`/
/// `state.manufacturer_refs`, so a later Save doesn't silently drop them.
pub fn open_project(
    state: &AppState,
    path: &Path,
    progress: &LoadHandle,
) -> Result<ProjectTree, String> {
    let guard = state
        .product_db
        .as_ref()
        .map(|m| m.lock().expect("state mutex poisoned"));
    let imported = import_and_project(path, guard.as_deref(), progress).map_err(|e| e.to_string());
    drop(guard);
    let (tree, project, (opaque, manufacturer_refs), report) = match imported {
        Ok(v) => v,
        Err(e) => {
            state
                .session_log
                .lock()
                .expect("state mutex poisoned")
                .push(LogEntry {
                    timestamp: session_log::now(),
                    severity: Severity::Error,
                    source: "import".to_string(),
                    message: e.clone(),
                    location: None,
                    detail: None,
                });
            return Err(e);
        }
    };
    *state.project.lock().expect("state mutex poisoned") = Some(project);
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (tree.errors, tree.warnings);
    *state.opaque.lock().expect("state mutex poisoned") = opaque;
    *state
        .manufacturer_refs
        .lock()
        .expect("state mutex poisoned") = manufacturer_refs;

    let mut log = state.session_log.lock().expect("state mutex poisoned");
    log.reset();
    for entry in session_log::from_import_report(&report) {
        log.push(entry);
    }
    let counts_summary = report
        .counts
        .rows
        .iter()
        .map(|c| format!("{}: {}/{}", c.entity, c.mapped, c.read))
        .collect::<Vec<_>>()
        .join(", ");
    log.push(LogEntry {
        timestamp: session_log::now(),
        severity: Severity::Info,
        source: "import".to_string(),
        message: format!("imported {} ({counts_summary})", report.source.file_name),
        location: None,
        detail: None,
    });
    drop(log);

    Ok(tree)
}

/// Persists `project` to a fresh or existing `.knxdb` file at `path`,
/// overwriting whatever it held. `opaque`/`manufacturer_refs` are written
/// alongside it — empty slices are a harmless no-op insert, so a native
/// (`.knxdb`-only) project with nothing to carry costs nothing extra. This
/// is what makes a server-side ETS-import → Save-As → Export round trip
/// keep its opaque passthrough and manufacturer manifest data instead of
/// silently exporting an empty one (final review finding B1).
pub fn save_project_as_impl(
    path: &Path,
    project: &knx_core::Project,
    opaque: &[knx_store::StoredOpaqueEntry],
    manufacturer_refs: &[knx_store::ManufacturerRef],
) -> Result<(), String> {
    let conn = knx_store::open_and_migrate(path).map_err(|e| e.to_string())?;
    knx_store::save_project(&conn, project).map_err(|e| e.to_string())?;
    knx_store::insert_opaque(&conn, opaque).map_err(|e| e.to_string())?;
    knx_store::insert_manufacturer_refs(&conn, manufacturer_refs).map_err(|e| e.to_string())?;
    Ok(())
}

/// Shared by `open_native_project_impl` (display-only) and
/// `open_native_project` (display + replaces `state`'s project) — same
/// reasoning as `import_and_project` above. Also reads back
/// opaque/manifest rows already on disk, so `state.opaque`/
/// `state.manufacturer_refs` stay accurate after a native load too, not
/// just after an ETS import.
fn load_native(
    path: &Path,
    progress: &LoadHandle,
) -> Result<(ProjectTree, knx_core::Project, ImportedOpaqueData), String> {
    // The four phases a native open really has (ADR-0023): the store open
    // (which also runs any pending migration), the normalized read, the
    // two passthrough reads, and the projection. Each is announced before
    // its own work, so the label names what is running.
    progress.phase(LoadPhase::OpenStore);
    let conn = knx_store::open_and_migrate(path).map_err(|e| e.to_string())?;
    progress.phase(LoadPhase::LoadStoredProject);
    let project = knx_store::load_project(&conn).map_err(|e| e.to_string())?;
    progress.phase(LoadPhase::LoadOpaque);
    let opaque = knx_store::load_opaque(&conn).map_err(|e| e.to_string())?;
    progress.phase(LoadPhase::LoadManufacturerRefs);
    let manufacturer_refs = knx_store::load_manufacturer_refs(&conn).map_err(|e| e.to_string())?;
    progress.phase(LoadPhase::BuildProjectTree);
    let tree = knx_projection::build_project_tree(&project);
    Ok((tree, project, (opaque, manufacturer_refs)))
}

/// Loads a `.knxdb` file at `path` and projects it, without touching
/// `state`. No `ImportReport` exists for a native load — nothing was
/// reinterpreted from an external format — so `tree.errors`/`tree.warnings`
/// stay at their default zero.
pub fn open_native_project_impl(path: &Path) -> Result<ProjectTree, String> {
    let progress = detached_progress(crate::load_progress::LoadKind::Open, path);
    let outcome = load_native(path, &progress).map(|(tree, ..)| tree);
    match &outcome {
        Ok(_) => progress.succeed(),
        Err(e) => progress.fail(e.clone()),
    }
    outcome
}

/// Loads a `.knxdb` file at `path`, replaces `state`'s project, and points
/// `store_path` at it — what the `/api/project/open` route calls.
pub fn open_native_project(
    state: &AppState,
    path: &Path,
    progress: &LoadHandle,
) -> Result<ProjectTree, String> {
    let loaded = load_native(path, progress);
    let (tree, project, (opaque, manufacturer_refs)) = match loaded {
        Ok(v) => v,
        Err(e) => {
            state
                .session_log
                .lock()
                .expect("state mutex poisoned")
                .push(LogEntry {
                    timestamp: session_log::now(),
                    severity: Severity::Error,
                    source: "open".to_string(),
                    message: e.clone(),
                    location: None,
                    detail: None,
                });
            return Err(e);
        }
    };
    *state.project.lock().expect("state mutex poisoned") = Some(project);
    *state.store_path.lock().expect("state mutex poisoned") = Some(path.to_path_buf());
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (0, 0);
    *state.opaque.lock().expect("state mutex poisoned") = opaque;
    *state
        .manufacturer_refs
        .lock()
        .expect("state mutex poisoned") = manufacturer_refs;

    let mut log = state.session_log.lock().expect("state mutex poisoned");
    log.reset();
    log.push(LogEntry {
        timestamp: session_log::now(),
        severity: Severity::Info,
        source: "open".to_string(),
        message: format!("opened {}", path.display()),
        location: None,
        detail: None,
    });
    drop(log);

    Ok(tree)
}

/// The only way [`new_project_impl`] can refuse: the project already open
/// has applied edits that creating a new one would throw away. Typed rather
/// than stringly so the route can answer `409 Conflict` — "you could have
/// avoided this" — instead of the blanket `400` every other in-memory
/// failure gets (see `errors.rs`'s own doc comment on that split).
#[derive(Debug, PartialEq, Eq)]
pub struct UnsavedChanges;

/// Creates an empty project in `state`, replacing whatever was open, and
/// resets every piece of per-project state `open_native_project` resets —
/// plus `store_path`, which that function *sets* and this one must clear:
/// a new project has never been saved anywhere, and leaving the previous
/// file's path behind would let the next plain Save overwrite that file
/// with this empty project.
///
/// The seed is one [`knx_core::Installation`] and nothing else.
/// `Command::CreateDevice` (`knx-core/src/command.rs:892-895`) needs
/// `installations.first_mut()` to exist, but takes `line: Option<LineId>`
/// and parks a device with no line in `topology.unassigned`
/// (`command.rs:906-916`), which `topology.rs:44-45` calls "valid project
/// state, not an error". So no area and no line are required to place a
/// device, and none is invented here. `info.project_id` is seeded because
/// `knx-etsproj`'s exporter rejects an empty one
/// (`export/schema11.rs:329`, `export/schema21.rs:100`) and no command in
/// `knx-core` can set it afterwards — an unseeded from-scratch project
/// could never be exported at all.
///
/// Names are the caller's, never this layer's invention: an absent name
/// leaves the installation unnamed, exactly what the ETS mapper produces
/// for an `Installation` with no `Name` attribute
/// (`knx-etsproj/src/map.rs:564`, `unwrap_or_default()`). The localized
/// default label belongs to the frontend's message catalogue, not to a
/// hardcoded string down here.
///
/// `group_address_style` only sets the *initial* style for a brand-new
/// project — restyling one that already has group addresses is
/// `set_group_address_style_impl`'s job, not this one's, and it refuses
/// the whole restyle unless every existing address survives it
/// (`knx_core::Command::SetGroupAddressStyle`, KNOWN_LIMITATIONS.md §84).
/// An absent style here keeps `Project::new`'s own `ThreeLevel` default,
/// so every existing caller behaves exactly as before; the route above
/// refuses an unrecognised one rather than guessing.
///
/// Refuses with [`UnsavedChanges`] when a project is open and its command
/// stack has anything to undo, unless `discard_changes` is set. There is no
/// dirty flag anywhere in `AppState` — `can_undo()` is the only signal that
/// the user changed something — so this over-refuses after a save, which is
/// the direction CLAUDE.md's "data integrity over convenience" points.
pub fn new_project_impl(
    state: &AppState,
    name: Option<String>,
    installation_name: Option<String>,
    language: Option<String>,
    group_address_style: Option<knx_core::GroupAddressStyle>,
    discard_changes: bool,
) -> Result<ProjectTree, UnsavedChanges> {
    if !discard_changes {
        let occupied = state
            .project
            .lock()
            .expect("state mutex poisoned")
            .is_some();
        let edited = state
            .command_stack
            .lock()
            .expect("state mutex poisoned")
            .can_undo();
        if occupied && edited {
            state
                .session_log
                .lock()
                .expect("state mutex poisoned")
                .push(LogEntry {
                    timestamp: session_log::now(),
                    severity: Severity::Warning,
                    source: "new".to_string(),
                    message: "refused to create a new project: the open one has unsaved edits"
                        .to_string(),
                    location: None,
                    detail: None,
                });
            return Err(UnsavedChanges);
        }
    }

    let language = language.unwrap_or_else(|| DEFAULT_NEW_PROJECT_LANGUAGE.to_string());
    let mut project = knx_core::Project::new(knx_core::Language(language));
    project.info.project_id = NEW_PROJECT_ID.to_string();
    project.info.name = name.unwrap_or_default();
    if let Some(style) = group_address_style {
        project.info.group_address_style = style;
    }
    project.installations.push(knx_core::Installation {
        id: knx_core::InstallationId(0),
        name: installation_name.unwrap_or_default(),
        default_line: None,
        multicast_address: None,
        completion: knx_core::CompletionStatus::default(),
        topology: knx_core::Topology {
            areas: Vec::new(),
            lines: Vec::new(),
            unassigned: Vec::new(),
        },
        buildings: Vec::new(),
        group_ranges: Vec::new(),
        group_addresses: Vec::new(),
        parameters: Vec::new(),
    });

    let tree = knx_projection::build_project_tree(&project);
    *state.project.lock().expect("state mutex poisoned") = Some(project);
    *state.store_path.lock().expect("state mutex poisoned") = None;
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (0, 0);
    *state.opaque.lock().expect("state mutex poisoned") = Vec::new();
    *state
        .manufacturer_refs
        .lock()
        .expect("state mutex poisoned") = Vec::new();

    let mut log = state.session_log.lock().expect("state mutex poisoned");
    log.reset();
    log.push(LogEntry {
        timestamp: session_log::now(),
        severity: Severity::Info,
        source: "new".to_string(),
        message: "created a new project with one empty installation".to_string(),
        location: None,
        detail: None,
    });
    drop(log);

    Ok(tree)
}

/// `StringTable`'s default language for a project nobody stated one for.
/// Matches every other `Project::new` call site in this repository; the
/// caller can say otherwise.
const DEFAULT_NEW_PROJECT_LANGUAGE: &str = "en";

/// Shaped like the project ids observed in the reference exports
/// (`P-0512`, `P-03DE`), which the exporter also uses as the ZIP directory
/// name (`knx-etsproj/src/export/mod.rs:73-74`). Only has to be unique
/// inside one project file, so a constant is enough.
const NEW_PROJECT_ID: &str = "P-0001";

/// Pushes one info entry on `Ok`, one error entry on `Err` — shared by
/// every operation that reports outcomes to the session log
/// (`save_project`/`save_project_as`/`export_project`/`undo_impl`/
/// `redo_impl`/`apply`/`create_device_impl`), none of which ever reset the
/// log (see `session_log.rs`'s own doc comment). `detail`, when given,
/// carries extra context the caller doesn't want duplicated into
/// `source`/`message` (e.g. a command's full `Debug` dump — see
/// `command_name` below).
fn log_outcome<T>(
    state: &AppState,
    source: &str,
    success_message: String,
    detail: Option<String>,
    result: &Result<T, String>,
) {
    let (severity, message) = match result {
        Ok(_) => (Severity::Info, success_message),
        Err(e) => (Severity::Error, e.clone()),
    };
    state
        .session_log
        .lock()
        .expect("state mutex poisoned")
        .push(LogEntry {
            timestamp: session_log::now(),
            severity,
            source: source.to_string(),
            message,
            location: None,
            detail,
        });
}

/// The command's own variant name (`"SetIndividualAddress"`, `"Batch"`,
/// ...) — the first token of its `Debug` form, up to the first
/// `(`/`{`/space. Cheap, and needs no match arm per `Command` variant to
/// stay in sync as `knx-core` grows new ones. Used for `source`/`message`
/// on `apply()`/`create_device_impl`'s log entries so a `Command::Batch`
/// or `Command::CreateDevice` (whose full `Debug` form can run to
/// multiple KB) doesn't get that dump stored — and rendered — twice per
/// entry; the full dump still goes into `detail` once.
fn command_name(cmd: &knx_core::Command) -> String {
    let debug = format!("{cmd:?}");
    debug
        .split(['(', '{', ' '])
        .next()
        .unwrap_or(&debug)
        .to_string()
}

pub fn save_project_as(state: &AppState, path: &Path) -> Result<(), String> {
    let result = (|| {
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or("no project open")?;
        let opaque = state.opaque.lock().expect("state mutex poisoned");
        let manufacturer_refs = state
            .manufacturer_refs
            .lock()
            .expect("state mutex poisoned");
        save_project_as_impl(path, project, &opaque, &manufacturer_refs)
    })();
    log_outcome(
        state,
        "save",
        format!("saved as {}", path.display()),
        None,
        &result,
    );
    result?;
    *state.store_path.lock().expect("state mutex poisoned") = Some(path.to_path_buf());
    Ok(())
}

pub fn save_project(state: &AppState) -> Result<(), String> {
    let result = (|| {
        let path = state
            .store_path
            .lock()
            .expect("state mutex poisoned")
            .clone()
            .ok_or("no save location yet — use Save As")?;
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or("no project open")?;
        let opaque = state.opaque.lock().expect("state mutex poisoned");
        let manufacturer_refs = state
            .manufacturer_refs
            .lock()
            .expect("state mutex poisoned");
        save_project_as_impl(&path, project, &opaque, &manufacturer_refs)
    })();
    log_outcome(state, "save", "saved".to_string(), None, &result);
    result
}

/// Exports the live in-memory project to a `.knxproj` file at `path`.
/// Requires `store_path` already set (i.e. the project has been saved or
/// opened as `.knxdb` at least once) — a workflow guarantee that the user
/// has committed the current state to disk before exporting, not a data
/// source: the opaque passthrough table and manufacturer manifest
/// `export_ets_project` needs come from `state.opaque`/
/// `state.manufacturer_refs` (the same live, in-memory copies every save
/// path writes through), copied into a throwaway in-memory `.knxdb` for
/// `export_ets_project`'s `Connection`-shaped interface. Earlier this
/// re-opened `store_path` off disk instead, which (see
/// `KNOWN_LIMITATIONS.md` #18) can lag the in-memory project — reading
/// live state instead removes that staleness risk for this data, even
/// though #18's broader "`store_path` names the wrong project" gap remains
/// for `save_project` itself. The project content itself comes from
/// `state.project` (live, possibly edited since the last save), not from
/// re-loading the `.knxdb` file.
pub fn export_project(
    state: &AppState,
    path: &Path,
) -> Result<knx_etsproj::export::ExportOutcome, String> {
    let result = (|| -> Result<knx_etsproj::export::ExportOutcome, String> {
        {
            let store_path = state.store_path.lock().expect("state mutex poisoned");
            if store_path.is_none() {
                return Err(
                    "save the project as .knxdb first — export reads passthrough data from the saved store"
                        .to_string(),
                );
            }
        }
        let opaque = state.opaque.lock().expect("state mutex poisoned");
        let manufacturer_refs = state
            .manufacturer_refs
            .lock()
            .expect("state mutex poisoned");
        let conn = knx_store::open_and_migrate_in_memory().map_err(|e| e.to_string())?;
        knx_store::insert_opaque(&conn, &opaque).map_err(|e| e.to_string())?;
        knx_store::insert_manufacturer_refs(&conn, &manufacturer_refs)
            .map_err(|e| e.to_string())?;
        drop(opaque);
        drop(manufacturer_refs);

        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or("no project open")?;
        let product_db_guard = state
            .product_db
            .as_ref()
            .map(|m| m.lock().expect("state mutex poisoned"));
        let outcome = knx_app::export_ets_project(project, &conn, product_db_guard.as_deref())
            .map_err(|e| e.to_string())?;
        drop(product_db_guard);
        std::fs::write(path, &outcome.bytes).map_err(|e| e.to_string())?;
        Ok(outcome)
    })();
    log_outcome(
        state,
        "export",
        format!("exported to {}", path.display()),
        None,
        &result,
    );
    result
}

/// Writes the live project's group addresses to `path` as "KNXBench
/// group-address CSV v1" text (`knx_csv::export_group_addresses`, design
/// §3) — the same file `resolve_new_project_path` writes for `.knxproj`
/// export, this format is entirely orthogonal to it. Never mutates the
/// project. Every warning `knx_csv` produces (today: a contested
/// `DatapointType`) is pushed into the session log individually, not just
/// returned in the response body — CLAUDE.md forbids silently discarding
/// information for being "merely" a warning.
pub fn export_group_addresses_csv_impl(
    state: &AppState,
    path: &Path,
) -> Result<knx_csv::CsvExport, String> {
    let result = (|| -> Result<knx_csv::CsvExport, String> {
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or("no project open")?;
        let export = knx_csv::export_group_addresses(project);
        std::fs::write(path, export.text.as_bytes()).map_err(|e| e.to_string())?;
        Ok(export)
    })();

    if let Ok(export) = &result {
        let mut log = state.session_log.lock().expect("state mutex poisoned");
        for warning in &export.warnings {
            log.push(LogEntry {
                timestamp: session_log::now(),
                severity: Severity::Warning,
                source: "csv-export".to_string(),
                message: warning.detail.clone(),
                location: warning.row.map(|row| format!("row {row}")),
                detail: None,
            });
        }
    }

    log_outcome(
        state,
        "csv-export",
        match &result {
            Ok(export) => format!(
                "exported group addresses to {} ({} warning(s))",
                path.display(),
                export.warnings.len()
            ),
            Err(_) => String::new(),
        },
        None,
        &result,
    );

    result
}

/// Renders the live project into one self-contained "project
/// documentation" HTML file at `path` (`knx_report::render_html`, design
/// doc `docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`)
/// — never called or logged as an ETS report, because no ETS-produced
/// report sample exists anywhere in this repository to be compatible
/// with. Never mutates the project. The clock read for the document's
/// generation timestamp happens right here, nowhere inside `knx-report`
/// itself — that crate's own doc comment holds it to reading only its
/// `ReportOptions` argument, which is what keeps its tests deterministic.
/// Every warning the render found (a device in no line, an address in no
/// range, a dangling building-part parent, an orphaned communication
/// object, a dangling group link, a malformed override) is pushed into
/// the session log individually, not just returned in the response body —
/// same reasoning as `export_group_addresses_csv_impl` above.
pub fn export_documentation_impl(
    state: &AppState,
    path: &Path,
) -> Result<knx_report::HtmlReport, String> {
    let result = (|| -> Result<knx_report::HtmlReport, String> {
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or("no project open")?;
        let options = knx_report::ReportOptions {
            generated_at: chrono::Utc::now(),
        };
        let report = knx_report::render_html(project, &options);
        std::fs::write(path, report.html.as_bytes()).map_err(|e| e.to_string())?;
        Ok(report)
    })();

    if let Ok(report) = &result {
        let mut log = state.session_log.lock().expect("state mutex poisoned");
        for warning in &report.warnings {
            log.push(LogEntry {
                timestamp: session_log::now(),
                severity: Severity::Warning,
                source: "doc-export".to_string(),
                message: warning.detail.clone(),
                location: Some(warning.location.clone()),
                detail: None,
            });
        }
    }

    log_outcome(
        state,
        "doc-export",
        match &result {
            Ok(report) => format!(
                "exported documentation to {} ({} warning(s))",
                path.display(),
                report.warnings.len()
            ),
            Err(_) => String::new(),
        },
        None,
        &result,
    );

    result
}

/// Computes what changed between the server's live, possibly edited,
/// in-memory project (`left`) and the `.knxdb` file at `path` (`right`) —
/// "what would Save change", deliberately not a re-read of `store_path`
/// (design spec `docs/superpowers/specs/2026-09-10-project-diff-design.md`
/// §7). Never mutates the project, never touches the session log: a diff
/// mutates nothing and produces no `ReportWarning`-shaped output, so there
/// is nothing established for it to log (task-4 brief).
///
/// `path` is checked with `path.exists()` *before* anything touches
/// `knx-store`: `knx_store::open_and_migrate` "opens, creating if absent"
/// — handed a typo'd path it would happily create an empty `.knxdb` and
/// this function would then dutifully report every entity in `left` as
/// removed instead of failing with a clear "does not exist" (the exact
/// gotcha design spec §7 calls out by name; regression-tested in
/// `tests/http_project_diff.rs`).
pub fn diff_project_impl(state: &AppState, path: &Path) -> Result<knx_diff::ProjectDiff, String> {
    let project = state.project.lock().expect("state mutex poisoned");
    let left = project.as_ref().ok_or("no project open")?;
    if !path.exists() {
        return Err(format!("{} does not exist", path.display()));
    }
    // The right-hand side is read for comparison only and never becomes
    // the open project, so its stages go to a detached handle rather than
    // onto the banner of whatever the user has open.
    let (_, right, _) = load_native(path, &detached_progress(crate::LoadKind::Open, path))?;
    Ok(knx_diff::diff_projects(left, &right))
}

/// Reads `path` as "KNXBench group-address CSV v1" text, plans the edit
/// against the live project (`knx_csv::parse_group_addresses` +
/// `knx_csv::plan_import`, design §4), and — unless any row is a
/// row-level error — applies the single resulting `Command::Batch` through
/// [`apply`], exactly like every other project-editing route.
///
/// Every problem and ignored column `knx_csv` reports reaches the session
/// log via [`session_log::from_csv_import_report`] before the
/// all-or-nothing decision below is even made, so a rejected file still
/// leaves its diagnostics behind — only the project itself stays
/// untouched. `Err` carries every offending row's number and detail, for
/// the route to surface as a `400` (never a `500`: a CSV a user hand-edited
/// wrong is their mistake to fix, not this server's fault).
pub fn import_group_addresses_csv_impl(
    state: &AppState,
    path: &Path,
) -> Result<(knx_projection::ProjectTree, knx_csv::CsvImportReport), String> {
    let result = (|| -> Result<(knx_projection::ProjectTree, knx_csv::CsvImportReport), String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let plan = {
            let project = state.project.lock().expect("state mutex poisoned");
            let project = project.as_ref().ok_or("no project open")?;
            let parsed = knx_csv::parse_group_addresses(&text, project.info.group_address_style);
            knx_csv::plan_import(project, &parsed)
        };

        {
            let mut log = state.session_log.lock().expect("state mutex poisoned");
            for entry in session_log::from_csv_import_report(&plan.report) {
                log.push(entry);
            }
        }

        let error_rows: Vec<String> = plan
            .report
            .problems
            .iter()
            .filter(|p| p.severity == knx_csv::Severity::Error)
            .map(|p| match p.row {
                Some(row) => format!("row {row}: {}", p.detail),
                None => p.detail.clone(),
            })
            .collect();
        if !error_rows.is_empty() {
            return Err(format!(
                "{} row(s) rejected, nothing applied: {}",
                error_rows.len(),
                error_rows.join("; ")
            ));
        }

        let tree = match plan.command {
            Some(cmd) => apply(state, cmd)?,
            // Every row was `unchanged` (or the file was empty of data
            // rows) — nothing to apply, but still a successful import that
            // needs a current tree in the response.
            None => {
                let project = state.project.lock().expect("state mutex poisoned");
                let project = project.as_ref().ok_or("no project open")?;
                let stack = state.command_stack.lock().expect("state mutex poisoned");
                let counts = *state.import_counts.lock().expect("state mutex poisoned");
                tree_with_state(project, &stack, counts)
            }
        };
        Ok((tree, plan.report))
    })();

    log_outcome(
        state,
        "csv-import",
        match &result {
            Ok((_, report)) => format!(
                "imported {} (created {}, updated {}, unchanged {})",
                path.display(),
                report.created,
                report.updated,
                report.unchanged
            ),
            Err(_) => String::new(),
        },
        None,
        &result,
    );

    result
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

/// Everything `device_detail`'s language overlay (T33 Task 2) needs from
/// the project, gathered while only `project` is locked — mirrors
/// `assemble_parameter_panel`'s own step 1, so `product_db`'s lock, taken
/// afterwards if at all, never overlaps this one.
struct ComObjectOverlayInput {
    program_ref: String,
    /// Keyed by `ComObjectNode::id` (== `ComObjectInstanceId.0`): the
    /// instance's ETS ref id, whether it is module-based, and the `Layer`
    /// its `text`/`description` overrides currently sit at. `None` for a
    /// layer means the override is `Absent`/`Empty`/`Malformed` — there is
    /// nothing to overlay, and `build_device_detail`'s own output is left
    /// exactly as it is.
    com_objects: HashMap<
        u32,
        (
            String,
            bool,
            Option<knx_core::Layer>,
            Option<knx_core::Layer>,
        ),
    >,
}

/// Builds one device's detail panel, overlaying two independent things from
/// the product database on top of what `device_detail_impl` alone can know:
///
/// 1. **`DeviceProductNode` resolution (T16).** Whether the device's stated
///    `product_ref`/`program_ref` resolve against an installed product
///    database is a database fact, not a translation — it is resolved
///    whenever a ref is present, `language` or not, overwriting
///    `build_device_detail`'s `NoDatabase` placeholder with `Resolved`,
///    `NoDatabase` (confirmed) or `NotInDatabase`. See
///    `knx_projection::DeviceProductNode::resolution`'s doc comment for why
///    that placeholder is safe to overwrite unconditionally.
/// 2. **Communication object `name`/`description` translation (T33 Task 2;
///    Global Constraint 2 is the invariant this guards: only a
///    `Program`/`ProgramRef`-layer value is product-supplied text, so only
///    those may be overlaid — `Instance`/`Inferred`/`UserEdit` are the
///    project's own words and are never translated).** This part alone
///    needs `language: Some(_)`.
///
/// `language: None` with no product ref stated, or no product database
/// open, issues no product-database query at all and returns
/// byte-identical to `device_detail_impl` alone (Global Constraint 3).
pub fn device_detail(
    state: &AppState,
    device_id: u32,
    language: Option<&str>,
) -> Result<knx_projection::DeviceDetail, String> {
    // Step 1: lock only `project`.
    let (mut detail, overlay_input) = {
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or("no project open")?;
        let detail = device_detail_impl(project, device_id)?;
        let overlay_input = language.and_then(|_| {
            let dev = project.devices.get(knx_core::DeviceId(device_id))?;
            Some(ComObjectOverlayInput {
                program_ref: dev.program_ref.clone(),
                com_objects: dev
                    .com_objects
                    .iter()
                    .filter_map(|com_id| project.devices.com_object(*com_id))
                    .map(|com| {
                        (
                            com.id.0,
                            (
                                com.source.ets_id.clone(),
                                com.module_instance.is_some(),
                                com.text.layer(),
                                com.description.layer(),
                            ),
                        )
                    })
                    .collect(),
            })
        });
        (detail, overlay_input)
    };

    // T16: a product/program ref is present, so `resolution` needs a real
    // answer regardless of `language` — see this function's own doc comment.
    let needs_product_lookup = !matches!(
        detail.product.resolution,
        knx_projection::ProductResolution::NoReference
    );

    if !needs_product_lookup && (language.is_none() || overlay_input.is_none()) {
        return Ok(detail);
    }

    let Some(products_mutex) = state.product_db.as_ref() else {
        if needs_product_lookup {
            detail.product.resolution = knx_projection::ProductResolution::NoDatabase;
        }
        return Ok(detail);
    };

    // Step 2: lock only `product_db` (`project`'s lock above is already
    // dropped — the two mutexes are never held at once). Shared by both
    // overlays below rather than locked twice.
    let products = products_mutex.lock().expect("state mutex poisoned");

    if needs_product_lookup {
        let product_ref = detail.product.product_ref.as_deref().unwrap_or_default();
        let program_ref = detail.product.program_ref.as_deref().unwrap_or_default();
        match knx_productdb::query::device_product(&products, product_ref, program_ref, language)
            .map_err(|e| e.to_string())?
        {
            Some(row) => {
                detail.product.resolution = knx_projection::ProductResolution::Resolved;
                detail.product.catalog = Some(knx_projection::DeviceProductCatalog {
                    manufacturer_id: row.manufacturer_id,
                    manufacturer_name: row.manufacturer_name,
                    product_text: row.product_text,
                    order_number: row.order_number,
                    hardware_name: row.hardware_name,
                    hardware_version: row.hardware_version,
                    hardware_serial_number: row.hardware_serial_number,
                    catalog_item_name: row.catalog_item_name,
                    catalog_item_number: row.catalog_item_number,
                    application_program_id: row.application_program_id,
                    application_name: row.application_name,
                    application_number: row.application_number,
                    application_version: row.application_version,
                    mask_version: row.mask_version,
                });
            }
            None => {
                detail.product.resolution = knx_projection::ProductResolution::NotInDatabase;
                detail.product.catalog = None;
            }
        }
    }

    let (Some(lang), Some(overlay_input)) = (language, overlay_input) else {
        return Ok(detail);
    };

    let program_id = knx_productdb::query::resolve_program(&products, &overlay_input.program_ref)
        .map_err(|e| e.to_string())?;
    // A device whose program is not installed is ordinary project state,
    // not an error — return the untranslated detail unchanged.
    let Some(program_id) = program_id else {
        return Ok(detail);
    };

    // Every lookup id this device needs, collected up front so the overlay
    // (translation table + `translation_overlay`'s own query) is loaded
    // exactly once for the whole device, not once per communication object
    // — `com_object_view` in a loop used to reload it per iteration, and a
    // device can own hundreds of communication objects.
    let lookup_ids: Vec<String> = detail
        .com_objects
        .iter()
        .filter_map(|com| overlay_input.com_objects.get(&com.id))
        .map(|(ref_id, module_based, _, _)| {
            knx_productdb::com_object_lookup_id(&program_id, ref_id, *module_based)
        })
        .collect();
    let lookup_id_refs: Vec<&str> = lookup_ids.iter().map(String::as_str).collect();
    let views =
        knx_productdb::query::com_object_views(&products, &program_id, &lookup_id_refs, Some(lang))
            .map_err(|e| e.to_string())?;

    for com in &mut detail.com_objects {
        let Some((ref_id, module_based, text_layer, description_layer)) =
            overlay_input.com_objects.get(&com.id)
        else {
            continue;
        };
        // Recomputed rather than carried: `com_object_lookup_id` is pure
        // and both sites feed it the same three inputs, so this is the
        // key the collection loop above queried with. If the two ever
        // drift apart the symptom is silent — a `views` miss and
        // untranslated text, with no test to notice — so keep them
        // reading the same `overlay_input` entry.
        let lookup_id = knx_productdb::com_object_lookup_id(&program_id, ref_id, *module_based);
        let Some(view) = views.get(&lookup_id) else {
            continue;
        };
        // Only a `Program`/`ProgramRef`-layer value came from the product
        // database and may be overlaid (Global Constraint 2). The layer
        // read here is the one stored on the project's own
        // `ComObjectInstance`, never the layer `view` itself reports —
        // those two can legitimately disagree (a `ComObjectRef`
        // translation with no matching structural override), and trusting
        // the latter would translate project-authored text.
        //
        // Finding M6: an overlay hit is not enough on its own — `pick()`
        // still returns the product database's untranslated column when
        // the requested language has no `translation` row for this value,
        // and overwriting the project's own resolved text with that
        // untranslated column on a language miss would replace a correct
        // value with a different (wrong-language) one. `*_translated`
        // (Task 1) is `true` only when the value `pick()` actually chose
        // came out of the overlay, so gating on it here means a miss
        // leaves `com.name`/`com.description` exactly as
        // `device_detail_impl` already resolved them.
        if matches!(
            text_layer,
            Some(knx_core::Layer::Program) | Some(knx_core::Layer::ProgramRef)
        ) && view.text_translated
        {
            if let Some(text) = &view.text {
                com.name = Some(text.clone());
            }
        }
        if matches!(
            description_layer,
            Some(knx_core::Layer::Program) | Some(knx_core::Layer::ProgramRef)
        ) && view.visible_description_translated
        {
            if let Some(description) = &view.visible_description {
                com.description = Some(description.clone());
            }
        }
    }

    Ok(detail)
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
    // Captured before `do_command` consumes `cmd` below: `cmd_desc` is the
    // full `Debug` dump, kept for `detail`; `cmd_name` is the short variant
    // name, used for `source`/`message` (see `command_name`'s doc comment).
    let cmd_desc = format!("{cmd:?}");
    let cmd_name = command_name(&cmd);
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    let result = stack.do_command(project, cmd).map_err(|e| e.to_string());
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    let tree = tree_with_state(project, &stack, import_counts);

    log_outcome(state, &cmd_name, cmd_name.clone(), Some(cmd_desc), &result);

    result.map(|()| tree)
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

/// Restyles the whole project — `Command::SetGroupAddressStyle` itself
/// checks every existing group address against the target style first and
/// refuses the whole change, naming the offending address, if even one
/// does not fit (KNOWN_LIMITATIONS.md §84). Undoable like every other
/// command, through the same `apply`/`CommandStack` path.
pub fn set_group_address_style_impl(
    state: &AppState,
    style: knx_core::GroupAddressStyle,
) -> Result<knx_projection::ProjectTree, String> {
    apply(state, knx_core::Command::SetGroupAddressStyle { style })
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

fn parse_building_part_kind(kind: &str) -> Result<knx_core::BuildingPartType, String> {
    match kind {
        "Building" => Ok(knx_core::BuildingPartType::Building),
        "Floor" => Ok(knx_core::BuildingPartType::Floor),
        "Room" => Ok(knx_core::BuildingPartType::Room),
        "Corridor" => Ok(knx_core::BuildingPartType::Corridor),
        "DistributionBoard" => Ok(knx_core::BuildingPartType::DistributionBoard),
        "BuildingPart" => Ok(knx_core::BuildingPartType::BuildingPart),
        other => Err(format!(
            "unknown building-part kind '{other}', expected one of Building/Floor/Room/Corridor/DistributionBoard/BuildingPart"
        )),
    }
}

pub fn create_building_part_impl(
    state: &AppState,
    name: String,
    kind: String,
    parent_id: Option<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    let kind = parse_building_part_kind(&kind)?;
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let id = project.ids.next_building_part_id();
        knx_core::Command::CreateBuildingPart {
            part: knx_core::BuildingPart {
                id,
                source: knx_core::SourceRef {
                    path: format!("KB-Building-{}", id.0),
                    ets_id: format!("KB-Building-{}", id.0),
                },
                name,
                number: None,
                kind,
                default_line: None,
                completion: knx_core::CompletionStatus::Editing,
                children: vec![],
                devices: vec![],
                parent: parent_id.map(knx_core::BuildingPartId),
            },
        }
    };
    apply(state, cmd)
}

pub fn delete_building_part_impl(
    state: &AppState,
    id: u32,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::DeleteBuildingPart {
            id: knx_core::BuildingPartId(id),
        },
    )
}

pub fn rename_building_part_impl(
    state: &AppState,
    id: u32,
    name: String,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::RenameBuildingPart {
            id: knx_core::BuildingPartId(id),
            name,
        },
    )
}

pub fn move_device_to_building_part_impl(
    state: &AppState,
    device_id: u32,
    part_id: Option<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::MoveDeviceToBuildingPart {
            device: knx_core::DeviceId(device_id),
            part: part_id.map(knx_core::BuildingPartId),
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
    language: Option<String>,
) -> Result<Vec<knx_productdb::query::CatalogItemRow>, String> {
    let products = state
        .product_db
        .as_ref()
        .ok_or("no product database configured")?
        .lock()
        .expect("state mutex poisoned");
    knx_productdb::query::catalog_items(
        &products,
        manufacturer.as_deref(),
        search.as_deref(),
        language.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// Every language identifier the installed product database has any
/// translation rows for, database-wide. Unlike `catalog_manufacturers_impl`/
/// `catalog_items_impl`, "no product database configured" is not an error
/// here: the Settings panel that lists these languages must still render on
/// a fresh install that has not imported a single package yet, so `None`
/// yields an empty list, not `Err`. Follows `assemble_parameter_panel`'s own
/// `let Some(products_mutex) = state.product_db.as_ref() else { .. }` idiom.
pub fn product_languages_impl(
    state: &AppState,
) -> Result<Vec<knx_productdb::query::TranslationLanguage>, String> {
    let Some(products_mutex) = state.product_db.as_ref() else {
        return Ok(Vec::new());
    };
    let products = products_mutex.lock().expect("state mutex poisoned");
    knx_productdb::query::translation_languages(&products).map_err(|e| e.to_string())
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
                        let view = knx_productdb::query::com_object_view(
                            &products,
                            &program_id,
                            &ref_id,
                            None,
                        )
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| {
                            format!("catalog communication-object reference is missing: {ref_id}")
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
    // Captured before `do_command` consumes `cmd` below — same convention
    // `apply()` uses, whose `log_outcome` helper this reuses so device
    // creation shows up in the session log too (it can't call `apply()`
    // itself: this function's return type carries creation diagnostics
    // `apply()` doesn't produce, and needs the enrichment pass below run
    // under the same `project` lock before releasing it).
    let cmd_desc = format!("{cmd:?}");
    let cmd_name = command_name(&cmd);
    let result = {
        let mut stack = state.command_stack.lock().expect("state mutex poisoned");
        stack.do_command(project, cmd).map_err(|e| e.to_string())
    };
    log_outcome(state, &cmd_name, cmd_name.clone(), Some(cmd_desc), &result);
    result?;

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

/// Builds a `Command::Batch` of one `DeleteDevice` per id, refusing an
/// empty `ids` up front (a `Batch([])` would apply as a documented no-op —
/// see `Command::Batch`'s own doc comment — but an empty batch reaching
/// here means the caller should not have enabled the action at all).
pub fn batch_delete_devices_impl(
    state: &AppState,
    ids: Vec<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    if ids.is_empty() {
        return Err("ids must not be empty".into());
    }
    apply(
        state,
        knx_core::Command::Batch(
            ids.into_iter()
                .map(|id| knx_core::Command::DeleteDevice {
                    id: knx_core::DeviceId(id),
                })
                .collect(),
        ),
    )
}

/// See `batch_delete_devices_impl` — same shape, `DeleteGroupAddress`.
pub fn batch_delete_group_addresses_impl(
    state: &AppState,
    ids: Vec<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    if ids.is_empty() {
        return Err("ids must not be empty".into());
    }
    apply(
        state,
        knx_core::Command::Batch(
            ids.into_iter()
                .map(|id| knx_core::Command::DeleteGroupAddress {
                    id: knx_core::GroupAddressId(id),
                })
                .collect(),
        ),
    )
}

/// See `batch_delete_devices_impl` — same shape, `MoveDeviceToLine`. Every
/// device in `device_ids` moves to the same `line_id` (or unassigned, if
/// `None`), mirroring `move_device_to_line_impl`'s own placement rule.
pub fn batch_move_devices_to_line_impl(
    state: &AppState,
    device_ids: Vec<u32>,
    line_id: Option<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    if device_ids.is_empty() {
        return Err("deviceIds must not be empty".into());
    }
    let line = line_id.map(knx_core::LineId);
    apply(
        state,
        knx_core::Command::Batch(
            device_ids
                .into_iter()
                .map(|device_id| knx_core::Command::MoveDeviceToLine {
                    device: knx_core::DeviceId(device_id),
                    line,
                })
                .collect(),
        ),
    )
}

/// See `batch_move_devices_to_line_impl` — same shape,
/// `MoveDeviceToBuildingPart`.
pub fn batch_move_devices_to_building_part_impl(
    state: &AppState,
    device_ids: Vec<u32>,
    building_part_id: Option<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    if device_ids.is_empty() {
        return Err("deviceIds must not be empty".into());
    }
    let part = building_part_id.map(knx_core::BuildingPartId);
    apply(
        state,
        knx_core::Command::Batch(
            device_ids
                .into_iter()
                .map(|device_id| knx_core::Command::MoveDeviceToBuildingPart {
                    device: knx_core::DeviceId(device_id),
                    part,
                })
                .collect(),
        ),
    )
}

pub fn undo_impl(state: &AppState) -> Result<knx_projection::ProjectTree, String> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    let result = stack.undo(project).map_err(|e| e.to_string());
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    let tree = tree_with_state(project, &stack, import_counts);
    log_outcome(state, "undo", "undo".to_string(), None, &result);
    result.map(|()| tree)
}

pub fn redo_impl(state: &AppState) -> Result<knx_projection::ProjectTree, String> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    let result = stack.redo(project).map_err(|e| e.to_string());
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    let tree = tree_with_state(project, &stack, import_counts);
    log_outcome(state, "redo", "redo".to_string(), None, &result);
    result.map(|()| tree)
}

// --- Parameter editor (T18 slice 3 task 3, design D20-D26) -----------

/// A `Diagnostic`'s machine-readable tag (KNOWN_LIMITATIONS.md §66) paired
/// with its fixed, hand-authored English sentence (design D26) — no
/// `node_id`, no internal identifiers, just what a device-panel user
/// needs to know. The `Debug` form still reaches the wire, unabridged, as
/// `ParameterDiagnosticDto::detail`; only the tag and the English sentence
/// come from here.
fn diagnostic_kind_and_message(
    diagnostic: &knx_productdb::dynamic::Diagnostic,
) -> (crate::routes::ParameterDiagnosticKindDto, &'static str) {
    use crate::routes::ParameterDiagnosticKindDto as Kind;
    use knx_productdb::dynamic::Diagnostic;
    match diagnostic {
        Diagnostic::NoBranchMatched { .. } => (
            Kind::NoBranchMatched,
            "A choice did not match any of its options.",
        ),
        Diagnostic::UnparsableTest { .. } => (
            Kind::UnparsableTest,
            "A choice's condition could not be understood.",
        ),
        Diagnostic::UnresolvedParamRef { .. } => (
            Kind::UnresolvedParamRef,
            "A choice's controlling parameter could not be found.",
        ),
        Diagnostic::NonNumericValue { .. } => (
            Kind::NonNumericValue,
            "A choice's controlling value was not a valid number.",
        ),
        Diagnostic::UnexpectedTypeNoneShape { .. } => (
            Kind::UnexpectedTypeNoneShape,
            "An unusual choice structure was skipped.",
        ),
        Diagnostic::UnrecognizedNode { .. } => (
            Kind::UnrecognizedNode,
            "An unrecognized program element was skipped.",
        ),
        Diagnostic::ModuleDefNotFound { .. } => (
            Kind::ModuleDefNotFound,
            "A module could not be found in this program.",
        ),
        Diagnostic::ModuleCycleDetected { .. } => (
            Kind::ModuleCycleDetected,
            "A module refers back to one of its own enclosing modules and was not expanded.",
        ),
        Diagnostic::ModuleNestingTooDeep { .. } => (
            Kind::ModuleNestingTooDeep,
            "A module is nested deeper than this program will expand.",
        ),
        Diagnostic::ModuleExpansionBudgetExhausted { .. } => {
            // Fix round 2: this one diagnostic variant now covers two
            // distinct budgets (module-expansion count and total
            // activated-ref count, see the type's own doc comment) —
            // the wording stays generic on purpose so it reads sensibly
            // for either.
            (
                Kind::ModuleExpansionBudgetExhausted,
                "This program's modules are too numerous to fully expand; the rest were skipped.",
            )
        }
        Diagnostic::MissingValue { .. } => (
            Kind::MissingValue,
            "A choice's controlling parameter has no value.",
        ),
        Diagnostic::ModuleWithoutId { .. } => (
            Kind::ModuleWithoutId,
            "A module instance has no identifier and cannot be matched to stored values.",
        ),
        // Fix round 1 (merge): main's module-argument work (T12) added
        // these three variants against the pre-T14 `diagnostic_message`,
        // which returned a bare string. Folded into the Kind-tagged shape
        // here so they get the same §66 translation path as everything
        // else in this match, instead of staying English-only by accident
        // of merge order.
        Diagnostic::ModuleArgumentNotBound { .. } => (
            Kind::ModuleArgumentNotBound,
            "A module argument could not be matched to the module's declaration and was ignored.",
        ),
        Diagnostic::UnsupportedModuleArgumentKind { .. } => (
            Kind::UnsupportedModuleArgumentKind,
            "A module argument uses a kind this build does not interpret and was ignored.",
        ),
        Diagnostic::UnresolvedTextPlaceholder { .. } => (
            Kind::UnresolvedTextPlaceholder,
            "A text placeholder had no matching module argument and was left as written.",
        ),
    }
}

/// Carries only the innermost `Module` — `scope.parent` is never walked.
/// Known limitation (`docs/KNOWN_LIMITATIONS.md`, "`ModuleScopeDto`
/// carries only the innermost scope"): since fix round 1, the server
/// correctly splits two nesting chains that share an innermost
/// `module_node` under different ancestors into two distinct sections,
/// but if both chains' innermost `Module`s are also nameless under the
/// same `ModuleDef`, this DTO is identical for both, so the client's
/// `sameScope()` (`ParameterPanel.tsx`) cannot tell the two sections
/// apart and misattributes each one's diagnostics to both — not merely
/// lost ancestor context, an actual cross-section misattribution.
fn module_scope_dto(scope: &knx_productdb::dynamic::ModuleScope) -> crate::routes::ModuleScopeDto {
    crate::routes::ModuleScopeDto {
        module_node: scope.module_node,
        module_id: scope.module_id.clone(),
        module_def_id: scope.module_def_id.clone(),
    }
}

/// Takes the longest run of ASCII digits at the start of `s`, returning
/// `None` for zero digits (`\d+` needs at least one) — `(digits, rest)`.
fn take_digits(s: &str) -> Option<(&str, &str)> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        None
    } else {
        Some((&s[..end], &s[end..]))
    }
}

/// Hand-rolled stand-in for `^(.*)_M-(\d+)_MI-(\d+)_(.*)$` — no `regex`
/// crate exists anywhere in this workspace (checked; see the parameter
/// editor design D21). A regex engine's greedy `.*` for the first group
/// backtracks only as far as it must, which — since the trailing `.*`
/// matches anything, including empty — is equivalent to picking the
/// *rightmost* position in `ets_id` where the literal
/// `_M-<digits>_MI-<digits>_` shape occurs. This scans left to right and
/// keeps overwriting its candidate on every syntactically valid match, so
/// whatever is left standing after the scan is that rightmost one.
/// Returns `(prefix, module_digits, mi_digits, suffix)`.
fn decompose_module_qualified(ets_id: &str) -> Option<(String, String, String, String)> {
    const MARKER: &str = "_M-";
    let mut best: Option<(usize, String, String, String, String)> = None;
    let mut search_from = 0;
    while let Some(relative) = ets_id.get(search_from..).and_then(|tail| tail.find(MARKER)) {
        let start = search_from + relative;
        let after_marker = &ets_id[start + MARKER.len()..];
        if let Some((module_digits, rest)) = take_digits(after_marker) {
            if let Some(rest) = rest.strip_prefix("_MI-") {
                if let Some((mi_digits, rest)) = take_digits(rest) {
                    if let Some(suffix) = rest.strip_prefix('_') {
                        best = Some((
                            start,
                            ets_id[..start].to_string(),
                            module_digits.to_string(),
                            mi_digits.to_string(),
                            suffix.to_string(),
                        ));
                    }
                }
            }
        }
        search_from = start + 1;
    }
    best.map(|(_, prefix, module_digits, mi_digits, suffix)| {
        (prefix, module_digits, mi_digits, suffix)
    })
}

/// The trailing `_M-<digits>` component of a program-side `Module/@Id`,
/// split into `module_id`'s own prefix (D39: "its text before `_M-<n>`")
/// and the digits. Same hand-rolled greedy-rightmost scan as
/// `decompose_module_qualified`, anchored at the string's end instead of
/// allowing a suffix after it — equivalent to `^(.*)_M-(\d+)$`. `None`
/// when `module_id` has no such trailing component at all, a shape this
/// slice's corpus evidence (E1) never shows but does not assume either.
fn module_id_own_prefix(module_id: &str) -> Option<&str> {
    const MARKER: &str = "_M-";
    let mut best: Option<usize> = None;
    let mut search_from = 0;
    while let Some(relative) = module_id
        .get(search_from..)
        .and_then(|tail| tail.find(MARKER))
    {
        let start = search_from + relative;
        let after_marker = &module_id[start + MARKER.len()..];
        if let Some((_digits, rest)) = take_digits(after_marker) {
            if rest.is_empty() {
                best = Some(start);
            }
        }
        search_from = start + 1;
    }
    best.map(|start| &module_id[..start])
}

/// D39's write target: `format!("{module_id}_MI-{digits}_{suffix}")`,
/// where `suffix` is `declared_id` with `module_id`'s own prefix and one
/// `_` stripped — reproduces the corpus's real stored ids exactly (E1).
/// `None` when `declared_id` does not actually start with that prefix, or
/// `module_id` has no `_M-<n>` shape to strip at all; never invented.
fn module_scoped_write_id(module_id: &str, mi_digits: &str, declared_id: &str) -> Option<String> {
    let prefix = module_id_own_prefix(module_id)?;
    let suffix = declared_id.strip_prefix(prefix)?.strip_prefix('_')?;
    Some(format!("{module_id}_MI-{mi_digits}_{suffix}"))
}

/// D39 rules 2-3: whether one imported `ModuleInstance` can serve as the
/// `MI-` authority for a program-side `module_id`, and if not, exactly
/// why — never a guess, never a default (D40).
enum MiAuthority {
    /// Exactly one imported `ModuleInstance` matches, and its
    /// `instance_ets_id` decomposes cleanly — these are the `MI-` digits
    /// a write target uses.
    Found(String),
    /// No imported `ModuleInstance`'s `source.ets_id` is the trailing
    /// component of `module_id` (D39 rule 2, zero matches).
    NoMatch,
    /// Two or more imported `ModuleInstance`s match one `module_id` — a
    /// genuinely repeated module (`MI-` > 1) whose channels this slice
    /// cannot tell apart on the read side (D40). Carries the shared
    /// `RefId` and every matching `instance_ets_id`, for the diagnostic.
    Ambiguous {
        source_ets_id: String,
        instance_ets_ids: Vec<String>,
    },
    /// Exactly one match, but its `instance_ets_id` is empty or does not
    /// decompose as `<source.ets_id>_MI-<digits>` (D39 rule 3) — D38's
    /// migration note treats empty exactly like a missing instance.
    Malformed {
        source_ets_id: String,
        instance_ets_id: String,
    },
}

/// D39 rules 2-3, verbatim: the instance-matching rule is
/// `module_id.ends_with("_" + instance.source.ets_id)` — the leading
/// underscore is what keeps `MD-1_M-2` from matching a `..._MD-11_M-2`
/// module id. `digits` must be all-ASCII (`\d+`), matching
/// `decompose_module_qualified`'s own definition of a valid `MI-`.
fn resolve_mi_authority(instances: &[knx_core::ModuleInstance], module_id: &str) -> MiAuthority {
    let matches: Vec<&knx_core::ModuleInstance> = instances
        .iter()
        .filter(|m| module_id.ends_with(&format!("_{}", m.source.ets_id)))
        .collect();
    match matches.as_slice() {
        [] => MiAuthority::NoMatch,
        [one] => {
            let expected_prefix = format!("{}_MI-", one.source.ets_id);
            match one
                .instance_ets_id
                .strip_prefix(expected_prefix.as_str())
                .filter(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
            {
                Some(digits) => MiAuthority::Found(digits.to_string()),
                None => MiAuthority::Malformed {
                    source_ets_id: one.source.ets_id.clone(),
                    instance_ets_id: one.instance_ets_id.clone(),
                },
            }
        }
        many => MiAuthority::Ambiguous {
            source_ets_id: many[0].source.ets_id.clone(),
            instance_ets_ids: many.iter().map(|m| m.instance_ets_id.clone()).collect(),
        },
    }
}

/// Everything `parameter_panel_impl`/`set_parameter_value_impl` share:
/// the assembled read model, plus the raw program-level ingredients D24's
/// write validation needs (kind/bounds/enum live on a declared
/// `ParameterRef` independent of whether it is currently active).
struct PanelAssembly {
    dto: crate::routes::ParameterPanelDto,
    program_id: Option<String>,
    views_by_id: HashMap<String, knx_productdb::query::ParameterView>,
    ref_ids: HashSet<String>,
}

fn empty_assembly(stale: Vec<(String, String)>) -> PanelAssembly {
    PanelAssembly {
        dto: crate::routes::ParameterPanelDto {
            program_id: None,
            sections: vec![],
            stale: stale
                .into_iter()
                .map(|(ets_id, raw)| crate::routes::StaleParameterDto { ets_id, raw })
                .collect(),
            diagnostics: vec![],
            tree: None,
        },
        program_id: None,
        views_by_id: HashMap::new(),
        ref_ids: HashSet::new(),
    }
}

/// Builds a device's parameter panel (design D20-D23, D26): project state
/// and product database are each locked at most once, never together
/// (Step 1 below locks only `project`; Step 2 locks only `product_db` —
/// the reverse order from `create_device_impl`, per this task's own
/// brief, since here the program reference comes from already-loaded
/// project state rather than the other way around).
fn assemble_parameter_panel(
    state: &AppState,
    device_id: u32,
    language: Option<&str>,
) -> Result<PanelAssembly, String> {
    let device = knx_core::DeviceId(device_id);

    // Step 1: lock only `project`.
    let (program_ref, stored, module_instances) = {
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or("no project open")?;
        let dev = project
            .devices
            .get(device)
            .ok_or_else(|| format!("device {device_id} not found"))?;
        let stored: Vec<(String, String)> = project
            .installations
            .first()
            .map(|installation| {
                installation
                    .parameters
                    .iter()
                    .filter(|p| p.device == device)
                    .map(|p| (p.source.ets_id.clone(), p.raw.clone()))
                    .collect()
            })
            .unwrap_or_default();
        // D39/D40: this device's own imported `ModuleInstance`s — the
        // project's own answer to "which channel is this," read once here
        // while `project` is locked, cloned so the lock can drop before
        // `product_db` is taken (same discipline as `stored` above).
        let module_instances: Vec<knx_core::ModuleInstance> = project
            .devices
            .module_instances()
            .filter(|m| m.device == device)
            .cloned()
            .collect();
        (dev.program_ref.clone(), stored, module_instances)
    };

    let Some(products_mutex) = state.product_db.as_ref() else {
        return Ok(empty_assembly(stored));
    };

    // Step 2: lock only `product_db` (`project`'s lock above is already
    // dropped — the two mutexes are never held at once).
    let products = products_mutex.lock().expect("state mutex poisoned");
    let program_id = knx_productdb::query::resolve_program(&products, &program_ref)
        .map_err(|e| e.to_string())?;
    let Some(program_id) = program_id else {
        return Ok(empty_assembly(stored));
    };

    let ref_ids = knx_productdb::query::parameter_ref_ids(&products, &program_id)
        .map_err(|e| e.to_string())?;
    // `language` is the request-supplied display language (T26 Task 2);
    // `None` keeps today's untranslated behaviour exactly as Task 1 left it.
    let views = knx_productdb::query::parameter_views(&products, &program_id, language)
        .map_err(|e| e.to_string())?;
    let views_by_id: HashMap<String, knx_productdb::query::ParameterView> =
        views.iter().cloned().map(|v| (v.id.clone(), v)).collect();

    // Coordinator addition: `parameter_views`' inner joins silently drop
    // a row whose `parameter`/`parameter_type` does not resolve — name
    // the gap instead of letting it vanish unremarked (measured 543/543
    // on the current corpus, so this is not expected to fire on real
    // data; it is here for the day a package does not join cleanly).
    let mut diagnostics: Vec<crate::routes::ParameterDiagnosticDto> = Vec::new();
    if views.len() != ref_ids.len() {
        let dropped = ref_ids.len().saturating_sub(views.len());
        diagnostics.push(crate::routes::ParameterDiagnosticDto {
            scope: None,
            kind: crate::routes::ParameterDiagnosticKindDto::ParametersUnreadable,
            message:
                "Some declared parameters could not be read from the product database and are not shown."
                    .to_string(),
            detail: format!(
                "parameter_views returned {} row(s) but parameter_ref_ids declares {} id(s) for program '{program_id}' ({dropped} dropped by an unresolved parameter/parameter_type join)",
                views.len(),
                ref_ids.len()
            ),
        });
    }

    // Pass A (design D21): sort every stored value into unscoped-supplied,
    // a regex candidate awaiting module-id validation, or outright
    // undecomposable (no verbatim match, no regex match at all).
    let mut supplied: HashMap<String, String> = HashMap::new();
    let mut candidates: Vec<(String, String, String, String, String, String)> = Vec::new();
    let mut stale: Vec<crate::routes::StaleParameterDto> = Vec::new();
    for (ets_id, raw) in stored {
        if ref_ids.contains(&ets_id) {
            // I1 (fix round 2): a second stored row for the same
            // unscoped id must not vanish the way the first committed
            // round let it -- named in a diagnostic and kept in `stale`,
            // the same loud treatment Pass B already gives a module-
            // scoped collision (D41) below.
            if let Some(previous_raw) = supplied.get(&ets_id) {
                diagnostics.push(crate::routes::ParameterDiagnosticDto {
                    scope: None,
                    kind: crate::routes::ParameterDiagnosticKindDto::DuplicateUnscopedValue,
                    message:
                        "Two stored values target the same parameter; the later one is ignored."
                            .to_string(),
                    detail: format!(
                        "'{ets_id}' has more than one stored row for this device; keeping '{previous_raw}'."
                    ),
                });
                stale.push(crate::routes::StaleParameterDto { ets_id, raw });
            } else {
                supplied.insert(ets_id, raw);
            }
        } else if let Some((prefix, module_digits, mi_digits, suffix)) =
            decompose_module_qualified(&ets_id)
        {
            candidates.push((ets_id, raw, prefix, module_digits, mi_digits, suffix));
        } else {
            stale.push(crate::routes::StaleParameterDto { ets_id, raw });
        }
    }

    // D42, step 1 of 2: the unscoped-only `ValueMap`, evaluated once to
    // learn which `Module/@Id`s this program's `choose` chain actually
    // reaches — Pass B needs that set before it can validate a single
    // scoped candidate, and `evaluate` is the only place that set is
    // computed (E3: no parallel module-expansion implementation).
    let mut values = knx_productdb::dynamic::resolve_values(&products, &program_id, &supplied)
        .map_err(|e| e.to_string())?;
    let trees = knx_productdb::dynamic::load_program_trees(&products, &program_id)
        .map_err(|e| e.to_string())?;
    let provisional_activation = knx_productdb::dynamic::evaluate(&trees, &values);

    // The declared `Module/@Id` set this provisional activation reached —
    // D21's second half of candidate validation.
    let module_ids: HashSet<String> = provisional_activation
        .parameter_refs
        .iter()
        .filter_map(|r| r.scope.as_ref().and_then(|s| s.module_id.clone()))
        .collect();

    // Pass B (D21, D41): validate every regex candidate against
    // `module_ids` and `ref_ids` as before, plus two new conditions —
    // its `MI-` digits must agree with the one authoritative
    // `ModuleInstance` when one exists (no authority: not checked, so a
    // pre-migration project displays exactly as it did before this
    // slice), and it must not collide with an already-validated row on
    // the same `(module_id, declared_id)` key (no silent overwrite: the
    // loser is `stale`, named alongside the winner in a diagnostic).
    let mut validated_scoped: HashMap<(String, String), String> = HashMap::new();
    let mut validated_scoped_ets_id: HashMap<(String, String), String> = HashMap::new();
    for (ets_id, raw, prefix, module_digits, mi_digits, suffix) in candidates {
        let module_id = format!("{prefix}_M-{module_digits}");
        let declared_id = format!("{prefix}_{suffix}");
        if !module_ids.contains(&module_id) || !ref_ids.contains(&declared_id) {
            stale.push(crate::routes::StaleParameterDto { ets_id, raw });
            continue;
        }
        if let MiAuthority::Found(authoritative_digits) =
            resolve_mi_authority(&module_instances, &module_id)
        {
            if authoritative_digits != mi_digits {
                stale.push(crate::routes::StaleParameterDto { ets_id, raw });
                continue;
            }
        }
        let key = (module_id, declared_id);
        if let Some(winner_ets_id) = validated_scoped_ets_id.get(&key) {
            // I2 (fix round 2): every other section-scoped diagnostic
            // carries a real `scope` the UI can filter by; this one used
            // to say `None` despite naming one specific module. The
            // provisional activation already resolved this exact
            // `module_id` (that is what `module_ids.contains` above just
            // checked), so its own `ModuleScope` is looked up rather
            // than reinvented.
            let scope_dto = provisional_activation.parameter_refs.iter().find_map(|r| {
                r.scope
                    .as_ref()
                    .filter(|s| s.module_id.as_deref() == Some(key.0.as_str()))
                    .map(|s| module_scope_dto(s))
            });
            diagnostics.push(crate::routes::ParameterDiagnosticDto {
                scope: scope_dto,
                kind: crate::routes::ParameterDiagnosticKindDto::DuplicateModuleScopedValue,
                message:
                    "Two stored values target the same module-scoped parameter; the later one is ignored."
                        .to_string(),
                detail: format!(
                    "'{winner_ets_id}' and '{ets_id}' both resolve to module '{}' parameter '{}'; keeping '{winner_ets_id}'.",
                    key.0, key.1
                ),
            });
            stale.push(crate::routes::StaleParameterDto { ets_id, raw });
            continue;
        }
        validated_scoped_ets_id.insert(key.clone(), ets_id);
        validated_scoped.insert(key, raw);
    }

    // D42, step 2 of 2: feed the validated scoped values back into the
    // same `ValueMap` and evaluate again, so a module-scoped `choose`
    // sees its own channel's value instead of the program default (D16).
    // Skipped entirely when there is nothing to feed — every corpus
    // project except KV (E2) — since a second `evaluate` over an
    // unchanged `ValueMap` can only reproduce the first activation.
    let activation = if validated_scoped.is_empty() {
        provisional_activation
    } else {
        for ((module_id, ref_id), raw) in validated_scoped.clone() {
            values.insert_scoped(module_id, ref_id, raw);
        }
        knx_productdb::dynamic::evaluate(&trees, &values)
    };

    // Group `Activation::parameter_refs` into one section per distinct
    // scope (D23), preserving each ref's document-order position and the
    // order sections are first encountered.
    //
    // Fix round 1 (blocking finding 4, goal-completion task 11): this key
    // used to be the flat `Option<i64>` `module_node` D18 introduced,
    // which stopped being unique once nesting shipped —
    // `dynamic_node.node_id` resets per `(program_id, module_def_id)`
    // tree, so two distinct nesting chains can reuse the same
    // `module_node` at the same depth under different ancestors, and the
    // sections would silently merge. `ModuleScope::node_chain()` (made
    // `pub` for this) gives the same full-chain key `evaluate`'s own
    // dedup already uses.
    struct SectionBuild {
        scope: Option<std::rc::Rc<knx_productdb::dynamic::ModuleScope>>,
        ref_ids: Vec<String>,
    }
    let mut section_order: Vec<Option<Vec<i64>>> = Vec::new();
    let mut sections_by_key: HashMap<Option<Vec<i64>>, SectionBuild> = HashMap::new();
    for active in &activation.parameter_refs {
        let key = active.scope.as_ref().map(|s| s.node_chain());
        sections_by_key
            .entry(key.clone())
            .or_insert_with(|| {
                section_order.push(key.clone());
                SectionBuild {
                    scope: active.scope.clone(),
                    ref_ids: Vec::new(),
                }
            })
            .ref_ids
            .push(active.ref_id.clone());
    }

    // S4 (fix round 2): a program that declares two `Module` elements
    // with the same `@Id` is malformed -- ETS's own id grammar makes
    // `@Id` unique per instantiation, so the corpus never shows this --
    // but nothing before this slice refused it, and two sections sharing
    // one `module_id` would silently reconstruct the identical
    // `write_ets_id`, collapsing two channels into one write target. Same
    // species of ambiguity D40 already refuses on the project side;
    // counted once here, before any section decides its own authority.
    let module_id_counts: HashMap<String, usize> = section_order
        .iter()
        .filter_map(|key| sections_by_key.get(key))
        .filter_map(|s| s.scope.as_ref().and_then(|sc| sc.module_id.clone()))
        .fold(HashMap::new(), |mut acc, module_id| {
            *acc.entry(module_id).or_insert(0) += 1;
            acc
        });

    let mut sections = Vec::with_capacity(section_order.len());
    for key in section_order {
        let section = sections_by_key.remove(&key).expect("just inserted above");
        // D39: a section earns editability, it does not start with it. An
        // unscoped section is editable exactly as before D39. A
        // module-scoped section is editable only when exactly one
        // imported `ModuleInstance` is its `MI-` authority (rules 2-3);
        // every other reason is named in a section diagnostic and the
        // section stays read-only rather than guessing (D40). A module
        // with no `@Id` at all is covered by D37's own diagnostic
        // (`activation.diagnostics`, folded in below) — not repeated here.
        let mi_digits: Option<String> = match &section.scope {
            None => None,
            Some(scope) => match &scope.module_id {
                None => None,
                Some(module_id)
                    if module_id_counts
                        .get(module_id.as_str())
                        .copied()
                        .unwrap_or(0)
                        > 1 =>
                {
                    diagnostics.push(crate::routes::ParameterDiagnosticDto {
                        scope: section.scope.as_ref().map(|s| module_scope_dto(s)),
                        kind: crate::routes::ParameterDiagnosticKindDto::DuplicateModuleId,
                        message:
                            "Two or more sections in this program declare the same module id; its fields are read-only."
                                .to_string(),
                        detail: format!(
                            "Module id '{module_id}' is declared by more than one Module element in this program (a malformed program); refusing to guess which section is authoritative."
                        ),
                    });
                    None
                }
                Some(module_id) => match resolve_mi_authority(&module_instances, module_id) {
                    MiAuthority::Found(digits) => Some(digits),
                    MiAuthority::NoMatch => {
                        diagnostics.push(crate::routes::ParameterDiagnosticDto {
                            scope: section.scope.as_ref().map(|s| module_scope_dto(s)),
                            kind: crate::routes::ParameterDiagnosticKindDto::NoModuleInstanceMatch,
                            message:
                                "No imported module instance matches this module; its fields are read-only."
                                    .to_string(),
                            detail: format!(
                                "No imported ModuleInstance's RefId matches module '{module_id}' (D39 rule 2, zero matches)."
                            ),
                        });
                        None
                    }
                    MiAuthority::Ambiguous {
                        source_ets_id,
                        instance_ets_ids,
                    } => {
                        diagnostics.push(crate::routes::ParameterDiagnosticDto {
                            scope: section.scope.as_ref().map(|s| module_scope_dto(s)),
                            kind: crate::routes::ParameterDiagnosticKindDto::AmbiguousModuleInstance,
                            message:
                                "Two or more imported module instances share this module; its fields are read-only."
                                    .to_string(),
                            detail: format!(
                                "RefId '{source_ets_id}' matches module '{module_id}', but {} ModuleInstances claim it ({}); refusing to guess which one is 'MI-' (D40)."
                                    , instance_ets_ids.len(), instance_ets_ids.join(", ")
                            ),
                        });
                        None
                    }
                    MiAuthority::Malformed {
                        source_ets_id,
                        instance_ets_id,
                    } => {
                        diagnostics.push(crate::routes::ParameterDiagnosticDto {
                            scope: section.scope.as_ref().map(|s| module_scope_dto(s)),
                            kind: crate::routes::ParameterDiagnosticKindDto::MalformedModuleInstanceId,
                            message:
                                "An imported module instance's identifier has an unexpected shape; this module's fields are read-only."
                                    .to_string(),
                            detail: format!(
                                "The ModuleInstance for RefId '{source_ets_id}' has Id '{instance_ets_id}', which does not decompose as '<RefId>_MI-<digits>' (D39 rule 3)."
                            ),
                        });
                        None
                    }
                },
            },
        };
        let mut fields = Vec::with_capacity(section.ref_ids.len());
        for ref_id in &section.ref_ids {
            // A ref the join dropped (see the diagnostic above) has no
            // metadata to show; it is not fabricated here.
            let Some(view) = views_by_id.get(ref_id) else {
                continue;
            };
            // D42: display reads through `ValueMap` alone — it already
            // knows, per scope, whether a stored (possibly module-scoped)
            // value or the program default answers.
            let value = values
                .get(section.scope.as_ref().map(|s| s.as_ref()), ref_id)
                .map(str::to_string);
            let is_scoped_stored = section.scope.as_ref().is_some_and(|scope| {
                scope.module_id.as_ref().is_some_and(|module_id| {
                    validated_scoped.contains_key(&(module_id.clone(), ref_id.clone()))
                })
            });
            let value_source =
                if is_scoped_stored || (section.scope.is_none() && supplied.contains_key(ref_id)) {
                    "Stored".to_string()
                } else {
                    "ProgramDefault".to_string()
                };
            // D39/D43: the write target this field's own suffix
            // reconstructs to, if any — a per-field outcome, since one
            // field's suffix failing to strip the module prefix must not
            // silently take its section-siblings down with it. S8 (fix
            // round 2): matched on `(scope, mi_digits)` together rather
            // than gating on a separately-computed `section_earned_
            // authority` bool and falling back to an empty-string
            // sentinel for "no authority" -- a missing `MI-` authority
            // now has no string standing in for it anywhere, not even an
            // unreachable one; the `(Some(_), None)` arm returns `None`
            // directly.
            let write_ets_id = match (&section.scope, mi_digits.as_deref()) {
                (None, _) => Some(view.id.clone()),
                (Some(_), None) => None,
                (Some(scope), Some(digits)) => scope
                    .module_id
                    .as_ref()
                    .and_then(|module_id| module_scoped_write_id(module_id, digits, &view.id)),
            };
            let editable = write_ets_id.is_some();
            fields.push(crate::routes::ParameterFieldDto {
                ets_id: view.id.clone(),
                name: view.name.clone(),
                text: view.text.clone(),
                kind: view.kind.clone(),
                value,
                value_source,
                editable,
                min: view.min_inclusive.clone(),
                max: view.max_inclusive.clone(),
                enum_options: view
                    .enum_options
                    .iter()
                    .map(|(value, text)| crate::routes::EnumOptionDto {
                        value: value.clone(),
                        text: text.clone(),
                    })
                    .collect(),
                display_order: view.display_order,
                access: view.access.clone(),
                write_ets_id,
            });
        }
        sections.push(crate::routes::ParameterSectionDto {
            scope: section.scope.as_ref().map(|s| module_scope_dto(s)),
            fields,
        });
    }

    // D26: every `Activation` diagnostic, mapped 1:1, in order.
    for scoped in &activation.diagnostics {
        let (kind, message) = diagnostic_kind_and_message(&scoped.diagnostic);
        diagnostics.push(crate::routes::ParameterDiagnosticDto {
            scope: scoped.scope.as_ref().map(|s| module_scope_dto(s)),
            kind,
            message: message.to_string(),
            detail: format!("{:?}", scoped.diagnostic),
        });
    }

    Ok(PanelAssembly {
        dto: crate::routes::ParameterPanelDto {
            program_id: Some(program_id.clone()),
            sections,
            stale,
            diagnostics,
            tree: None,
        },
        program_id: Some(program_id),
        views_by_id,
        ref_ids,
    })
}

pub(crate) fn parameter_panel_impl(
    state: &AppState,
    device_id: u32,
    language: Option<&str>,
) -> Result<crate::routes::ParameterPanelDto, String> {
    assemble_parameter_panel(state, device_id, language).map(|assembly| assembly.dto)
}

/// True if `s` holds a character outside XML 1.0's own `Char` production
/// (`#x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] |
/// [#x10000-#x10FFFF]`) — one no XML 1.0 document can represent at all,
/// escaped or not (a lone Unicode surrogate can never occur here: `char`
/// already excludes it). Verified directly against this workspace's own
/// exporter, `quick_xml` 0.42.0 (the version this workspace's `Cargo.lock`
/// pins): `BytesStart::push_attribute` writes such a character straight
/// through, unescaped, rather than rejecting it — so
/// `crates/knx-etsproj/src/export/schema21.rs`'s `p.push("Value",
/// param.raw.clone())` — and `crates/knx-etsproj/src/export/schema11.rs`'s
/// identical line, same call, same shape, a different schema version's
/// exporter making the identical assumption — would silently hand a value
/// this validator let through to a writer that turns it into a
/// not-well-formed `.knxproj`.
/// T18 slice 5's own addition, applied below to every kind that can carry
/// free-form text; `Number`/`Restriction` never need it (already
/// numeric/enum-only) and `None` is never writable at all.
fn contains_disallowed_xml_char(s: &str) -> bool {
    s.chars().any(|c| {
        let cp = c as u32;
        (cp < 0x20 && !matches!(cp, 0x9 | 0xA | 0xD)) || cp == 0xFFFE || cp == 0xFFFF
    })
}

/// The exact `TypeIPAddress` IPv6 encoding the KNX Project Schema
/// documents: "eight groups of four hexadecimal digits, separated by
/// colons, e.g. 2001:0db8:85a3:0000:0000:8a2e:0370:7334" (`Project
/// Schema23 v01.00.00.pdf`, §1.1.3.19 `simpleType Value_t`, p.31/64 — the
/// `TypeFloat` row's citation two arms up is p.30/64; the page breaks
/// between the two rows of this same table, and the IPv6 sentence lands on
/// the far side of it — extracted verbatim via `pdftotext -layout`).
/// Deliberately narrower than
/// `std::net::Ipv6Addr::from_str`, which also accepts `::`-compressed and
/// short-group forms this schema text never mentions; accepting those here
/// would be inventing a rule and presenting it as the schema's, which T18
/// slice 5 was explicitly told not to do (see
/// docs/KNOWN_LIMITATIONS.md §3 for the recorded gap).
fn is_schema_ipv6(s: &str) -> bool {
    let groups: Vec<&str> = s.split(':').collect();
    groups.len() == 8
        && groups
            .iter()
            .all(|g| g.len() == 4 && g.chars().all(|c| c.is_ascii_hexdigit()))
}

/// D24 step 3: kind-appropriate validation of a candidate raw value
/// against its declared `ParameterView`. `Number`/`Restriction` use the
/// program's own bounds/enumeration. `None` is never writable (slice 1's
/// D9). `Float`/`Text`/`IPAddress` (T18 slice 5) validate against the
/// `.knxprod`/`.knxproj` schema's own documented or corpus-observed
/// encoding for that kind — see each arm's own comment for its evidence.
/// `Picture`/`Raw` stay a non-empty-string-plus-XML-safety check: neither
/// kind appears anywhere in the Project Schema's `Value_t` encoding table,
/// in either spec knowledge base, or in any `.knxprod` under
/// `OriginalData/` (checked; zero occurrences of both `<TypePicture>` and
/// `<TypeRawData>`), so there is no format to validate against without
/// inventing one — recorded, not pretended away, in
/// docs/KNOWN_LIMITATIONS.md §3.
fn validate_kind_and_bounds(
    view: &knx_productdb::query::ParameterView,
    raw: &str,
) -> Result<(), String> {
    match view.kind.as_str() {
        "None" => Err(format!(
            "'{}' has parameter kind None, which carries no writable value",
            view.id
        )),
        "Number" => {
            if raw.is_empty() {
                return Err(format!("'{}' requires a non-empty value", view.id));
            }
            let parsed: i64 = raw.parse().map_err(|_| {
                format!(
                    "'{}' is Number-kind; '{raw}' does not parse as an integer",
                    view.id
                )
            })?;
            if let Some(min) = &view.min_inclusive {
                let min: i64 = min.parse().map_err(|_| {
                    format!(
                        "program declares an unparsable min_inclusive '{min}' for '{}'",
                        view.id
                    )
                })?;
                if parsed < min {
                    return Err(format!("'{}' must be >= {min} (got {parsed})", view.id));
                }
            }
            if let Some(max) = &view.max_inclusive {
                let max: i64 = max.parse().map_err(|_| {
                    format!(
                        "program declares an unparsable max_inclusive '{max}' for '{}'",
                        view.id
                    )
                })?;
                if parsed > max {
                    return Err(format!("'{}' must be <= {max} (got {parsed})", view.id));
                }
            }
            Ok(())
        }
        "Restriction" => {
            if view.enum_options.iter().any(|(value, _)| value == raw) {
                Ok(())
            } else {
                Err(format!(
                    "'{}' is Restriction-kind; '{raw}' is not one of its declared values",
                    view.id
                ))
            }
        }
        "Float" => {
            if raw.is_empty() {
                return Err(format!("'{}' requires a non-empty value", view.id));
            }
            if contains_disallowed_xml_char(raw) {
                return Err(format!(
                    "'{}' contains a character no XML 1.0 document can represent",
                    view.id
                ));
            }
            // Value_t's own encoding for a stored `TypeFloat` is scientific
            // notation with 16 significant digits and a 3-digit exponent
            // ("Project Schema23 v01.00.00.pdf" §1.1.3.19, p.30/64) — but
            // that describes the wire format `value.ToString("E15", ...)`
            // produces, not what a person types into a form field, and this
            // design stores `raw` verbatim rather than reformatting it
            // (constraint 5: no change to the stored representation). So
            // this accepts any finite number in ordinary decimal or
            // scientific notation and leaves the E15 wire-format question
            // open — recorded in docs/KNOWN_LIMITATIONS.md §3.
            let parsed: f64 = raw.parse().map_err(|_| {
                format!(
                    "'{}' is Float-kind; '{raw}' does not parse as a number",
                    view.id
                )
            })?;
            if !parsed.is_finite() {
                return Err(format!(
                    "'{}' must be a finite number, not '{raw}'",
                    view.id
                ));
            }
            // `min_inclusive`/`max_inclusive` on a `Float` kind come from
            // `<TypeFloat minInclusive maxInclusive>` (corpus-observed, MDT
            // `M-0083_A-0317-31-7DC6_PT-2ByteFloatTemp`:
            // `minInclusive="-100" maxInclusive="200"`) via the same
            // generic columns `Number` already reads above.
            if let Some(min) = &view.min_inclusive {
                let min: f64 = min.parse().map_err(|_| {
                    format!(
                        "program declares an unparsable min_inclusive '{min}' for '{}'",
                        view.id
                    )
                })?;
                if parsed < min {
                    return Err(format!("'{}' must be >= {min} (got {parsed})", view.id));
                }
            }
            if let Some(max) = &view.max_inclusive {
                let max: f64 = max.parse().map_err(|_| {
                    format!(
                        "program declares an unparsable max_inclusive '{max}' for '{}'",
                        view.id
                    )
                })?;
                if parsed > max {
                    return Err(format!("'{}' must be <= {max} (got {parsed})", view.id));
                }
            }
            Ok(())
        }
        "Text" => {
            if raw.is_empty() {
                return Err(format!("'{}' requires a non-empty value", view.id));
            }
            if contains_disallowed_xml_char(raw) {
                return Err(format!(
                    "'{}' contains a character no XML 1.0 document can represent",
                    view.id
                ));
            }
            // `size_in_bit` on a `Text` kind comes from `<TypeText
            // SizeInBit="…">`'s own declared storage size (corpus-observed,
            // MDT `M-0083_A-0317-31-7DC6`: `SizeInBit="240"` and `"640"`).
            // UTF-8 byte length is used as the size proxy — KNX text
            // parameters are conventionally single-byte-per-character
            // (ISO 8859-1-shaped), unverified for this exact attribute, so
            // this is the conservative direction: it can only reject a
            // value ISO 8859-1 would have allowed, never accept one that
            // overflows the device's declared storage.
            if let Some(size_in_bit) = view.size_in_bit {
                // Rounded up, not floored: a device field declared
                // `SizeInBit="4"` still has a whole byte of storage — ETS
                // devices are byte-addressed, and no `.knxprod` under
                // `OriginalData/` declares a `TypeText` whose `SizeInBit`
                // is not itself a multiple of 8, so this is untested
                // against a non-multiple-of-8 real value either way. Floor
                // division would give `max_bytes = 0` for such a field —
                // unwritable by construction, which is worse than the
                // rounding-up direction's only risk (accepting one byte
                // more than the true storage, for a declaration this
                // corpus has never actually produced).
                //
                // `size_in_bit` is whatever `parse_i64` accepted from an
                // untrusted `TypeText/@SizeInBit` — a manufacturer file
                // could declare `i64::MAX`, and plain `size_in_bit + 7`
                // would panic on overflow in a debug build or wrap to a
                // negative `max_bytes` in release, rejecting every value
                // with a message quoting a negative byte count.
                // `saturating_add` keeps this a rejection-only failure
                // mode: an absurd declaration clamps to "accept anything
                // that fits in memory" rather than "accept nothing and
                // lie about why" — the old floor division could not
                // overflow either, so this restores that property.
                // A negative declaration is not a small field, it is a
                // broken one. Without this guard `max_bytes` comes out
                // non-positive and the rejection message below presents
                // `-8 bits` as though it were a legitimate declared size,
                // which tells the user nothing they can act on. Say the
                // declaration is unparsable, in the same words the
                // `min_inclusive`/`max_inclusive` arms use for theirs.
                if size_in_bit <= 0 {
                    return Err(format!(
                        "program declares an unparsable size_in_bit '{size_in_bit}' for '{}'",
                        view.id
                    ));
                }
                let max_bytes = size_in_bit.saturating_add(7) / 8;
                let actual = raw.len() as i64;
                if actual > max_bytes {
                    return Err(format!(
                        "'{}' is Text-kind with a declared size of {max_bytes} bytes ({size_in_bit} bits); '{raw}' is {actual} bytes",
                        view.id
                    ));
                }
            }
            Ok(())
        }
        "IPAddress" => {
            if raw.is_empty() {
                return Err(format!("'{}' requires a non-empty value", view.id));
            }
            // Value_t documents both address families for `TypeIPAddress`:
            // "IPv4 addresses: decimal dotted notation" and "IPv6
            // addresses: eight groups of four hexadecimal digits,
            // separated by colons" ("Project Schema23 v01.00.00.pdf"
            // §1.1.3.19, p.30-31/64) — so both are accepted here, neither
            // preferred. IPv4 is checked with `std::net::Ipv4Addr`, whose
            // parser was verified in this session (workspace's own pinned
            // std/rustc) to already reject leading zeroes and
            // out-of-range octets exactly as the schema's sibling
            // `Ipv4Address_t` restriction pattern requires. IPv6 uses
            // `is_schema_ipv6` (see its own doc comment) rather than
            // `std::net::Ipv6Addr`, which accepts compressed forms the
            // schema text does not document.
            if raw.parse::<std::net::Ipv4Addr>().is_ok() || is_schema_ipv6(raw) {
                Ok(())
            } else {
                Err(format!(
                    "'{}' is IPAddress-kind; '{raw}' is neither IPv4 in decimal-dotted notation nor IPv6 as eight colon-separated groups of four hex digits",
                    view.id
                ))
            }
        }
        "Picture" | "Raw" => {
            if raw.is_empty() {
                Err(format!("'{}' requires a non-empty value", view.id))
            } else if contains_disallowed_xml_char(raw) {
                Err(format!(
                    "'{}' contains a character no XML 1.0 document can represent",
                    view.id
                ))
            } else {
                Ok(())
            }
        }
        _ => {
            if raw.is_empty() {
                Err(format!("'{}' requires a non-empty value", view.id))
            } else if contains_disallowed_xml_char(raw) {
                Err(format!(
                    "'{}' contains a character no XML 1.0 document can represent",
                    view.id
                ))
            } else {
                Ok(())
            }
        }
    }
}

/// Writes one parameter value (design D24): validates, applies exactly
/// one `Command::SetParameterValue` (undo/redo-able via the same
/// `command_stack` every other edit uses), then re-assembles and returns
/// the fresh `ParameterPanelDto` — no second request needed to see the
/// effect (D24's own rejected alternative names why) — with `apply`'s own
/// freshly rebuilt `ProjectTree` riding along in `tree` (T3 fix round 1, item
/// 6), so a caller that also needs to republish the project context has
/// the server's own answer instead of one it would otherwise have to
/// reconstruct by hand.
pub(crate) fn set_parameter_value_impl(
    state: &AppState,
    device_id: u32,
    ets_id: String,
    raw: String,
    language: Option<&str>,
) -> Result<crate::routes::ParameterPanelDto, String> {
    let before = assemble_parameter_panel(state, device_id, language)?;
    let program_id = before
        .program_id
        .clone()
        .ok_or("device has no resolvable application program")?;

    // D43: the assembled panel is the single authority on what is
    // writable — a field's `write_ets_id` (never `None` unless it is
    // read-only) is the only id this request may legitimately name.
    // Walking the panel replaces the old two-step `ref_ids`/scope check
    // entirely, rather than re-deriving an id shape independently here.
    let mut matched: Option<&crate::routes::ParameterFieldDto> = None;
    let mut bare_match_write_id: Option<Option<String>> = None;
    'search: for section in &before.dto.sections {
        for field in &section.fields {
            if field.write_ets_id.as_deref() == Some(ets_id.as_str()) {
                matched = Some(field);
                // I5 (fix round 2): S4 now refuses editability everywhere
                // two sections could otherwise reconstruct the same
                // `write_ets_id`, so at most one field in the whole panel
                // can ever satisfy this. Stop as soon as it is found
                // instead of reading like a last-wins scan over a
                // uniqueness that was only ever assumed, not enforced.
                break 'search;
            }
            if field.ets_id == ets_id {
                bare_match_write_id = Some(field.write_ets_id.clone());
            }
        }
    }

    let field = match matched {
        Some(field) => field,
        None => {
            let reason = if let Some(actual_write_id) = bare_match_write_id {
                match actual_write_id {
                    Some(correct) => format!(
                        "is shown, but must be written using its module-qualified id '{correct}', not this one"
                    ),
                    None => "is currently shown but not writable (its module-scoped section has no single authoritative module instance, or its write target could not be reconstructed)".to_string(),
                }
            } else if before.ref_ids.contains(&ets_id) {
                "is declared by this program but not currently active".to_string()
            } else if decompose_module_qualified(&ets_id).is_some() {
                // S3 (fix round 2): a module-qualified id whose `MI-`
                // digit or module component doesn't match any panel
                // field used to fall through to "is not a parameter
                // declared by this program" -- false whenever the
                // parameter genuinely is declared and only the instance
                // is wrong (a stale frontend, a foreign module, a wrong
                // `MI-` digit). Reusing the existing decomposer (never a
                // second id-shape parser, per this task's own rule) tells
                // the truth instead: the shape is a module-qualified id,
                // it is simply not one any editable field targets right
                // now.
                "is a module-qualified id, but no editable field's write target matches it"
                    .to_string()
            } else {
                "is not a parameter declared by this program".to_string()
            };
            return Err(format!("'{ets_id}' {reason} (program '{program_id}')"));
        }
    };

    let view = before.views_by_id.get(&field.ets_id).ok_or_else(|| {
        format!(
            "'{}' is declared by program '{program_id}' but its parameter/parameter_type row could not be resolved",
            field.ets_id
        )
    })?;
    validate_kind_and_bounds(view, &raw)?;

    let device = knx_core::DeviceId(device_id);
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let installation = project.installations.first().ok_or("no installation")?;
        let existing_id = installation
            .parameters
            .iter()
            .find(|p| p.device == device && p.source.ets_id == ets_id)
            .map(|p| p.id);
        let id = existing_id.unwrap_or_else(|| project.ids.next_parameter_instance_id());
        knx_core::Command::SetParameterValue {
            id,
            device,
            ets_id: ets_id.clone(),
            raw: raw.clone(),
        }
    };
    // `apply` already rebuilds the authoritative tree from the genuine
    // post-write `CommandStack` (`tree_with_state`, called after
    // `do_command`) -- surface it rather than let the caller reconstruct
    // `can_undo`/`can_redo` by hand from a staler tree it happened to be
    // holding (T3 fix round 1, item 6).
    let tree = apply(state, cmd)?;

    let mut panel = parameter_panel_impl(state, device_id, language)?;
    panel.tree = Some(tree);
    Ok(panel)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_project_path() -> PathBuf {
        knx_testsupport::reference_ets4_path()
    }

    /// A minimal `ParameterView` of the given `kind`, for
    /// `validate_kind_and_bounds` tests that need no more than kind plus
    /// (optionally) bounds/size — every field beyond that is irrelevant to
    /// the function under test.
    fn view_of_kind(kind: &str) -> knx_productdb::query::ParameterView {
        knx_productdb::query::ParameterView {
            id: "P-1".to_string(),
            display_order: None,
            tag: None,
            name: None,
            text: None,
            text_layer: knx_productdb::query::ValueLayer::Program,
            kind: kind.to_string(),
            access: None,
            min_inclusive: None,
            max_inclusive: None,
            size_in_bit: None,
            enum_options: Vec::new(),
        }
    }

    // T18 slice 5: `Float`/`Text`/`IPAddress`/`Picture`/`Raw` deep format
    // validation — accept/reject pairs including the boundary cases, per
    // task-10's own acceptance criterion. `Picture`/`Raw` only get the
    // universal non-empty/XML-safety checks (see `validate_kind_and_bounds`'s
    // own doc comment for why nothing deeper is defensible), so their cases
    // live here too, to keep that absence visibly tested rather than
    // silently unexercised.

    #[test]
    fn float_accepts_a_plain_decimal_within_declared_bounds() {
        let mut view = view_of_kind("Float");
        view.min_inclusive = Some("-100".to_string());
        view.max_inclusive = Some("200".to_string());
        assert!(validate_kind_and_bounds(&view, "36.6").is_ok());
    }

    #[test]
    fn float_accepts_the_inclusive_boundary_values() {
        let mut view = view_of_kind("Float");
        view.min_inclusive = Some("-100".to_string());
        view.max_inclusive = Some("200".to_string());
        assert!(validate_kind_and_bounds(&view, "-100").is_ok());
        assert!(validate_kind_and_bounds(&view, "200").is_ok());
    }

    #[test]
    fn float_rejects_one_past_each_bound() {
        let mut view = view_of_kind("Float");
        view.min_inclusive = Some("-100".to_string());
        view.max_inclusive = Some("200".to_string());
        assert!(validate_kind_and_bounds(&view, "-100.0001").is_err());
        assert!(validate_kind_and_bounds(&view, "200.0001").is_err());
    }

    #[test]
    fn float_rejects_unparsable_and_non_finite_input() {
        let view = view_of_kind("Float");
        assert!(validate_kind_and_bounds(&view, "not-a-number").is_err());
        assert!(validate_kind_and_bounds(&view, "NaN").is_err());
        assert!(validate_kind_and_bounds(&view, "inf").is_err());
        assert!(validate_kind_and_bounds(&view, "").is_err());
    }

    #[test]
    fn float_accepts_scientific_notation_without_bounds() {
        let view = view_of_kind("Float");
        assert!(validate_kind_and_bounds(&view, "1.5E+003").is_ok());
    }

    #[test]
    fn number_rejects_empty_with_the_same_message_every_kind_shares() {
        // Item 8 of the whole-branch review: the `Number` arm's empty-string
        // case was added to match `Float`/`Text`/`IPAddress`'s pre-existing
        // one instead of falling through to a generic parse-failure
        // message. Nothing exercised that until now.
        let view = view_of_kind("Number");
        let err = validate_kind_and_bounds(&view, "").unwrap_err();
        assert_eq!(err, "'P-1' requires a non-empty value");
    }

    #[test]
    fn text_accepts_a_value_within_its_declared_size() {
        let mut view = view_of_kind("Text");
        view.size_in_bit = Some(240); // 30 bytes, corpus-observed shape
        assert!(validate_kind_and_bounds(&view, "hello").is_ok());
    }

    #[test]
    fn text_accepts_exactly_the_declared_byte_boundary() {
        let mut view = view_of_kind("Text");
        view.size_in_bit = Some(240); // 30 bytes
        let exactly_30 = "a".repeat(30);
        assert!(validate_kind_and_bounds(&view, &exactly_30).is_ok());
    }

    #[test]
    fn text_rejects_one_byte_past_its_declared_size() {
        let mut view = view_of_kind("Text");
        view.size_in_bit = Some(240); // 30 bytes
        let thirty_one = "a".repeat(31);
        assert!(validate_kind_and_bounds(&view, &thirty_one).is_err());
    }

    #[test]
    fn text_with_a_negative_declared_size_says_the_declaration_is_broken() {
        // `parse_i64` accepts `SizeInBit="-8"` as happily as `"240"`, and
        // a negative field is not a small field. Before the guard this
        // rejected every value while quoting `-8 bits` back at the user
        // as though the device really had a negative amount of storage.
        let mut view = view_of_kind("Text");
        view.size_in_bit = Some(-8);
        let err = validate_kind_and_bounds(&view, "a")
            .expect_err("a negative declared size must be rejected");
        assert!(
            err.contains("unparsable size_in_bit"),
            "the message must blame the declaration, not the value: {err}"
        );
    }

    #[test]
    fn text_declared_narrower_than_a_byte_still_accepts_one_byte() {
        // `SizeInBit="4"` still has a whole byte of storage (ETS devices
        // are byte-addressed) — this is the ceiling-vs-floor case the
        // whole-branch review's item 4 asked for: floor division
        // (`size_in_bit / 8`) gives `max_bytes = 0` here, an unwritable
        // field by construction, so this test fails if `(size_in_bit + 7)
        // / 8` is ever reverted to plain floor division.
        let mut view = view_of_kind("Text");
        view.size_in_bit = Some(4);
        assert!(validate_kind_and_bounds(&view, "a").is_ok());
    }

    #[test]
    fn text_without_a_declared_size_has_no_length_cap() {
        let view = view_of_kind("Text");
        let long = "a".repeat(10_000);
        assert!(validate_kind_and_bounds(&view, &long).is_ok());
    }

    #[test]
    fn text_rejects_empty_and_a_raw_control_character() {
        let view = view_of_kind("Text");
        assert!(validate_kind_and_bounds(&view, "").is_err());
        assert!(validate_kind_and_bounds(&view, "a\u{1}b").is_err());
    }

    #[test]
    fn ip_address_accepts_ipv4_decimal_dotted_notation() {
        let view = view_of_kind("IPAddress");
        assert!(validate_kind_and_bounds(&view, "192.168.1.1").is_ok());
        assert!(validate_kind_and_bounds(&view, "0.0.0.0").is_ok());
        assert!(validate_kind_and_bounds(&view, "255.255.255.255").is_ok());
    }

    #[test]
    fn ip_address_rejects_leading_zeroes_and_out_of_range_octets() {
        let view = view_of_kind("IPAddress");
        assert!(validate_kind_and_bounds(&view, "192.168.001.1").is_err());
        assert!(validate_kind_and_bounds(&view, "256.1.1.1").is_err());
        assert!(validate_kind_and_bounds(&view, "1.2.3").is_err());
    }

    #[test]
    fn ip_address_accepts_the_schema_example_ipv6_form() {
        let view = view_of_kind("IPAddress");
        assert!(validate_kind_and_bounds(&view, "2001:0db8:85a3:0000:0000:8a2e:0370:7334").is_ok());
    }

    #[test]
    fn ip_address_rejects_compressed_ipv6_notation() {
        // `::1` is valid IPv6 generally, but not the exact eight-group,
        // four-hex-digit form Value_t documents for `TypeIPAddress` — see
        // `is_schema_ipv6`'s own doc comment.
        let view = view_of_kind("IPAddress");
        assert!(validate_kind_and_bounds(&view, "::1").is_err());
        assert!(validate_kind_and_bounds(&view, "2001:db8::1").is_err());
    }

    #[test]
    fn ip_address_rejects_empty_and_garbage() {
        let view = view_of_kind("IPAddress");
        assert!(validate_kind_and_bounds(&view, "").is_err());
        assert!(validate_kind_and_bounds(&view, "not-an-address").is_err());
    }

    #[test]
    fn picture_and_raw_accept_any_non_empty_xml_safe_string() {
        for kind in ["Picture", "Raw"] {
            let view = view_of_kind(kind);
            assert!(validate_kind_and_bounds(&view, "anything at all").is_ok());
        }
    }

    #[test]
    fn picture_and_raw_reject_empty_and_a_raw_control_character() {
        for kind in ["Picture", "Raw"] {
            let view = view_of_kind(kind);
            assert!(validate_kind_and_bounds(&view, "").is_err());
            assert!(validate_kind_and_bounds(&view, "a\u{1}b").is_err());
        }
    }

    // Fix round 1, item 3: an id containing two syntactically valid
    // `_M-<digits>_MI-<digits>_` markers must decompose on the rightmost
    // one, matching `^(.*)_M-(\d+)_MI-(\d+)_(.*)$`'s greedy-backtracking
    // semantics (D21). This id's first marker (`_M-1_MI-1_`) is itself
    // immediately followed by a second, later one (`_M-2_MI-2_`); only the
    // rightmost split's prefix/suffix are correct.
    #[test]
    fn decompose_module_qualified_keeps_the_rightmost_of_two_valid_markers() {
        let id = "P_M-1_MI-1_P_M-2_MI-2_TAIL";
        let (prefix, module_digits, mi_digits, suffix) =
            decompose_module_qualified(id).expect("two valid markers should decompose");
        assert_eq!(prefix, "P_M-1_MI-1_P");
        assert_eq!(module_digits, "2");
        assert_eq!(mi_digits, "2");
        assert_eq!(suffix, "TAIL");
    }

    #[test]
    fn opening_a_project_through_a_wired_product_db_enriches_more_than_without() {
        if !reference_project_path().exists() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
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

        let (_, without, _, _) = import_and_project(
            &reference_project_path(),
            None,
            &detached_progress(crate::LoadKind::Import, &reference_project_path()),
        )
        .unwrap();
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
        open_project(
            &state,
            &reference_project_path(),
            &detached_progress(crate::LoadKind::Import, &reference_project_path()),
        )
        .unwrap();
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

    // Regression for fix-round-1 finding 1: `create_device_impl` calls
    // `do_command` directly instead of through `apply()` (it needs the
    // enrichment pass to run under the same `project` lock, and returns a
    // richer type than `apply()` can), so it must push its own log entries
    // rather than silently skipping the session log for every device
    // created. Note: catalog-lookup failures (unknown product database /
    // catalog item) happen *before* a `Command` is even built and stay
    // unlogged, same as e.g. `set_individual_address_impl`'s own
    // pre-`apply()` address-parse failure — only the `do_command` outcome
    // itself is in scope here.
    #[test]
    fn creating_a_device_logs_an_info_entry_and_a_failed_creation_logs_an_error_entry() {
        let (_dir, state) = state_with_product_db();

        // `do_command` itself fails: line 999 doesn't exist.
        create_device_impl(&state, Some(999), "M-1_CI-1".into(), "Actuator 1".into()).unwrap_err();
        create_device_impl(&state, None, "M-1_CI-1".into(), "Actuator 1".into()).unwrap();

        let log = state.session_log.lock().unwrap();
        let entries = log.entries();
        assert_eq!(
            entries.len(),
            2,
            "both the failed and the successful CreateDevice should reach the session log: {entries:?}"
        );
        assert_eq!(entries[0].severity, Severity::Error);
        assert_eq!(entries[1].severity, Severity::Info);
        assert_eq!(
            entries[1].source, "CreateDevice",
            "source should be the command's short variant name, not its full Debug dump"
        );
        assert!(
            entries[1]
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("CreateDevice")),
            "detail should carry the command's full Debug dump: {:?}",
            entries[1].detail
        );
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

        let detail = device_detail(&state, device_id, None).unwrap();
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

    fn push_sentinel(state: &AppState) {
        state.session_log.lock().unwrap().push(LogEntry {
            timestamp: session_log::now(),
            severity: Severity::Info,
            source: "sentinel".into(),
            message: "pre-existing entry".into(),
            location: None,
            detail: None,
        });
    }

    #[test]
    fn a_failed_import_appends_an_error_entry_without_resetting_the_log() {
        let state = AppState::default();
        push_sentinel(&state);

        let result = open_project(
            &state,
            Path::new("/does/not/exist.knxproj"),
            &detached_progress(
                crate::LoadKind::Import,
                Path::new("/does/not/exist.knxproj"),
            ),
        );
        assert!(result.is_err());

        let entries = state.session_log.lock().unwrap().entries().to_vec();
        assert_eq!(
            entries.len(),
            2,
            "the sentinel entry must survive a failed import"
        );
        assert_eq!(entries[0].source, "sentinel");
        assert_eq!(entries[1].source, "import");
        assert_eq!(entries[1].severity, Severity::Error);
    }

    #[test]
    fn a_successful_import_resets_and_repopulates_the_log() {
        if !reference_project_path().exists() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let state = AppState::default();
        push_sentinel(&state);

        let result = open_project(
            &state,
            &reference_project_path(),
            &detached_progress(crate::LoadKind::Import, &reference_project_path()),
        );
        assert!(result.is_ok());

        let entries = state.session_log.lock().unwrap().entries().to_vec();
        assert!(
            entries.iter().all(|e| e.source != "sentinel"),
            "a successful import must reset the log, dropping any prior entries"
        );
        let last = entries.last().expect("at least the final import notice");
        assert_eq!(last.source, "import");
        assert_eq!(last.severity, Severity::Info);
    }

    // Fix round 1 (Q2): `diagnostic_kind_and_message`'s fifteen literals
    // (twelve at fix round 1, plus three more folded in by this round's
    // merge of main's T12 module-argument work) and `messages/en.ts`'s
    // `parameters.diagnostic.*` entries for the same fifteen kinds are two
    // independent sources of the same English
    // sentence, and nothing before this test asserted they had to agree.
    // This pins this file's half of that pair: every string below is
    // copied verbatim from `apps/knx-web/src/messages/en.ts` (as of this
    // fix round), so an edit to either side that isn't mirrored on the
    // other fails here, on this side, immediately. It does not and
    // cannot fail on a divergence introduced by editing `en.ts` alone —
    // that half has no Rust test to run against it — but a `git blame`
    // on this test block is now the pointer from here to there.
    #[test]
    fn diagnostic_kind_and_message_matches_the_english_catalogue() {
        use crate::routes::ParameterDiagnosticKindDto as Kind;
        use knx_productdb::dynamic::Diagnostic;

        let cases: Vec<(Diagnostic, Kind, &str)> = vec![
            (
                Diagnostic::NoBranchMatched {
                    choose_node: 1,
                    param_ref: None,
                    observed_value: "x".to_string(),
                },
                Kind::NoBranchMatched,
                "A choice did not match any of its options.",
            ),
            (
                Diagnostic::UnparsableTest {
                    when_node: 1,
                    raw: "x".to_string(),
                },
                Kind::UnparsableTest,
                "A choice's condition could not be understood.",
            ),
            (
                Diagnostic::UnresolvedParamRef {
                    choose_node: 1,
                    param_ref: None,
                },
                Kind::UnresolvedParamRef,
                "A choice's controlling parameter could not be found.",
            ),
            (
                Diagnostic::NonNumericValue {
                    choose_node: 1,
                    param_ref: None,
                    raw: "x".to_string(),
                },
                Kind::NonNumericValue,
                "A choice's controlling value was not a valid number.",
            ),
            (
                Diagnostic::UnexpectedTypeNoneShape { choose_node: 1 },
                Kind::UnexpectedTypeNoneShape,
                "An unusual choice structure was skipped.",
            ),
            (
                Diagnostic::UnrecognizedNode {
                    node_id: 1,
                    kind: "x".to_string(),
                },
                Kind::UnrecognizedNode,
                "An unrecognized program element was skipped.",
            ),
            (
                Diagnostic::ModuleDefNotFound {
                    node_id: 1,
                    ref_id: None,
                },
                Kind::ModuleDefNotFound,
                "A module could not be found in this program.",
            ),
            (
                Diagnostic::ModuleCycleDetected {
                    node_id: 1,
                    ref_id: None,
                },
                Kind::ModuleCycleDetected,
                "A module refers back to one of its own enclosing modules and was not expanded.",
            ),
            (
                Diagnostic::ModuleNestingTooDeep {
                    node_id: 1,
                    ref_id: None,
                    depth: 9,
                },
                Kind::ModuleNestingTooDeep,
                "A module is nested deeper than this program will expand.",
            ),
            (
                Diagnostic::ModuleExpansionBudgetExhausted {
                    node_id: 1,
                    ref_id: None,
                    budget: 9,
                },
                Kind::ModuleExpansionBudgetExhausted,
                "This program's modules are too numerous to fully expand; the rest were skipped.",
            ),
            (
                Diagnostic::MissingValue {
                    choose_node: 1,
                    param_ref: None,
                },
                Kind::MissingValue,
                "A choice's controlling parameter has no value.",
            ),
            (
                Diagnostic::ModuleWithoutId { node_id: 1 },
                Kind::ModuleWithoutId,
                "A module instance has no identifier and cannot be matched to stored values.",
            ),
            (
                Diagnostic::ModuleArgumentNotBound {
                    node_id: 1,
                    ref_id: None,
                },
                Kind::ModuleArgumentNotBound,
                "A module argument could not be matched to the module's declaration and was ignored.",
            ),
            (
                Diagnostic::UnsupportedModuleArgumentKind {
                    node_id: 1,
                    kind: "x".to_string(),
                },
                Kind::UnsupportedModuleArgumentKind,
                "A module argument uses a kind this build does not interpret and was ignored.",
            ),
            (
                Diagnostic::UnresolvedTextPlaceholder {
                    node_id: 1,
                    name: "x".to_string(),
                },
                Kind::UnresolvedTextPlaceholder,
                "A text placeholder had no matching module argument and was left as written.",
            ),
        ];

        for (diagnostic, expected_kind, expected_message) in cases {
            let (kind, message) = diagnostic_kind_and_message(&diagnostic);
            assert_eq!(
                kind, expected_kind,
                "diagnostic_kind_and_message returned the wrong kind for {diagnostic:?}"
            );
            assert_eq!(
                message, expected_message,
                "diagnostic_kind_and_message drifted from messages/en.ts for {diagnostic:?}"
            );
        }
    }
}
