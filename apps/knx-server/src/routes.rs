use axum::extract::DefaultBodyLimit;
use axum::extract::Multipart;
use axum::extract::Path as AxumPath;
use axum::extract::Query;
use axum::extract::State;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::bus::GroupAddressContext;
use crate::domain;
use crate::errors::ApiError;
use crate::paths::{resolve_new_project_path, resolve_project_path};
use crate::SharedState;

pub fn project_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/project", get(current_project))
        .route("/api/project/import", post(import_project))
        .route("/api/project/load-progress", get(load_progress))
        .route("/api/project/new", post(new_project))
        .route("/api/project/open", post(open_native_project))
        .route("/api/project/save", post(save_project))
        .route("/api/project/save-as", post(save_project_as))
        .route(
            "/api/project/group-address-style",
            post(set_group_address_style),
        )
        .route("/api/device/{id}", axum::routing::get(device_detail))
        .route(
            "/api/device/{id}/parameters",
            get(parameter_panel).post(set_parameter_value),
        )
        .route("/api/individual-address", post(set_individual_address))
        .route("/api/device-description", post(set_device_description))
        .route("/api/com-object-dpt", post(set_com_object_dpt))
        .route(
            "/api/com-object-description",
            post(set_com_object_description),
        )
        .route("/api/com-object-flag", post(set_com_object_flag))
        .route("/api/group-addresses", post(create_group_address))
        .route("/api/group-addresses/{id}", delete(delete_group_address))
        .route(
            "/api/group-addresses/csv-export",
            post(export_group_addresses_csv),
        )
        .route(
            "/api/project/documentation-export",
            post(export_documentation),
        )
        .route(
            "/api/project/documentation-preview",
            post(preview_documentation),
        )
        .route("/api/project/diff", post(diff_project))
        .route(
            "/api/group-addresses/csv-import",
            post(import_group_addresses_csv),
        )
        .route("/api/areas", post(create_area))
        .route("/api/areas/{id}", delete(delete_area))
        .route("/api/lines", post(create_line))
        .route("/api/lines/{id}", delete(delete_line))
        .route("/api/move-device", post(move_device_to_line))
        .route(
            "/api/move-device-to-building-part",
            post(move_device_to_building_part),
        )
        .route("/api/building-parts", post(create_building_part))
        .route(
            "/api/building-parts/{id}",
            delete(delete_building_part).patch(rename_building_part),
        )
        .route("/api/group-ranges", post(create_group_range))
        .route(
            "/api/group-ranges/{id}",
            delete(delete_group_range).patch(rename_group_range),
        )
        .route(
            "/api/group-links",
            post(link_com_object).delete(unlink_com_object),
        )
        .route("/api/catalog/manufacturers", get(catalog_manufacturers))
        .route("/api/catalog/items", get(catalog_items))
        .route("/api/product-languages", get(product_languages))
        .route(
            "/api/catalog/install",
            post(install_catalog_package).layer(DefaultBodyLimit::max(MAX_CATALOG_PACKAGE_BYTES)),
        )
        .route("/api/devices", post(create_device))
        .route("/api/devices/{id}", delete(delete_device))
        .route("/api/devices/batch-delete", post(batch_delete_devices))
        .route(
            "/api/group-addresses/batch-delete",
            post(batch_delete_group_addresses),
        )
        .route(
            "/api/devices/batch-move-line",
            post(batch_move_devices_to_line),
        )
        .route(
            "/api/devices/batch-move-building-part",
            post(batch_move_devices_to_building_part),
        )
        .route("/api/undo", post(undo))
        .route("/api/redo", post(redo))
        .route("/api/log", get(log))
}

/// The product database also validates a 256 MiB package bound. Applying the
/// same bound here prevents multipart buffering from exceeding it first.
const MAX_CATALOG_PACKAGE_BYTES: usize = 256 * 1024 * 1024;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogInstallMemberDto {
    path: String,
    role: String,
    sha256: String,
    size: u64,
}

/// `knx_productdb`'s `"Signature"` role names what a `.signature` member
/// *is*, not that anything checked it — nothing in this codebase verifies
/// one (KNOWN_LIMITATIONS.md §85). The stored `package_member.role` column
/// stays exactly `"Signature"` so the domain layer and its tests keep a
/// stable, unqualified identifier; this function only qualifies the text
/// at the one boundary a human — or a future UI — actually reads it from.
fn display_role(role: String) -> String {
    if role == "Signature" {
        "Signature (stored, not verified)".to_string()
    } else {
        role
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogTranslationCountsDto {
    program: usize,
    catalog: usize,
    hardware: usize,
    master: usize,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogInstallReportDto {
    sha256: String,
    scheme: u32,
    skipped: bool,
    members: Vec<CatalogInstallMemberDto>,
    unknown: usize,
    conflicts: usize,
    /// Both of these used to stop at the CLI: `InstallReport` has carried
    /// them since R3 and §86 respectively, and this DTO simply never
    /// forwarded them, so a web or desktop user saw an install report that
    /// was silently narrower than the one `knx products ingest` prints.
    translations: CatalogTranslationCountsDto,
    dropped_datapoint_types: usize,
}

impl From<knx_productdb::InstallReport> for CatalogInstallReportDto {
    fn from(report: knx_productdb::InstallReport) -> Self {
        Self {
            sha256: report.sha256,
            scheme: report.scheme,
            skipped: report.skipped,
            members: report
                .members
                .into_iter()
                .map(|member| CatalogInstallMemberDto {
                    path: member.path,
                    role: display_role(member.role),
                    sha256: member.sha256,
                    size: member.size,
                })
                .collect(),
            unknown: report.unknown,
            conflicts: report.conflicts.len(),
            translations: CatalogTranslationCountsDto {
                program: report.translations.program,
                catalog: report.translations.catalog,
                hardware: report.translations.hardware,
                master: report.translations.master,
            },
            dropped_datapoint_types: report.dropped_datapoint_types,
        }
    }
}

/// T18 slice 3 task 3 (design D20-D26): the parameter panel's read model
/// plus the response of a successful write, both riding this one DTO
/// (D24's "same response, no second GET"). Plain `#[derive(Serialize)]`,
/// following `CatalogInstallReportDto`'s own precedent above — not
/// `ts-rs`, not `knx-projection`, per the coordinator's ruling on DTO
/// placement (design doc D20: "no new crate"). `PartialEq` dropped (fix
/// round 1, item 6) the moment `tree` below made it unavailable for free:
/// `knx_projection::ProjectTree` does not derive it, nothing in this crate
/// compared two whole `ParameterPanelDto`s for equality, and adding it to
/// `ProjectTree` for this alone would ripple into every type it contains.
#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParameterPanelDto {
    pub(crate) program_id: Option<String>,
    pub(crate) sections: Vec<ParameterSectionDto>,
    pub(crate) stale: Vec<StaleParameterDto>,
    pub(crate) diagnostics: Vec<ParameterDiagnosticDto>,
    /// The authoritative tree `apply()` already built from the genuine
    /// post-write `CommandStack` (T3 fix round 1, item 6) — `Some` only from
    /// `set_parameter_value_impl`, always `None` from the plain `GET`,
    /// which runs no command and has no fresher tree to offer. Replaces
    /// the hand-built `{ ...tree, can_undo: true, can_redo: false }`
    /// overlay `Inspector.tsx` used to construct client-side: that overlay
    /// was exact for `can_undo`/`can_redo` (both come from the same
    /// `CommandStack` this field does) but stale for every other field a
    /// future command could change, since it never asked the server.
    pub(crate) tree: Option<knx_projection::ProjectTree>,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParameterSectionDto {
    pub(crate) scope: Option<ModuleScopeDto>,
    pub(crate) fields: Vec<ParameterFieldDto>,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModuleScopeDto {
    pub(crate) module_node: i64,
    pub(crate) module_id: Option<String>,
    pub(crate) module_def_id: String,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParameterFieldDto {
    pub(crate) ets_id: String,
    pub(crate) name: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) kind: String,
    pub(crate) value: Option<String>,
    pub(crate) value_source: String,
    pub(crate) editable: bool,
    pub(crate) min: Option<String>,
    pub(crate) max: Option<String>,
    pub(crate) enum_options: Vec<EnumOptionDto>,
    /// Verbatim from `ParameterView.display_order` (fix round 1, item 1).
    /// `None` means the package declared no order at all -- every row in
    /// the real corpus -- and must serialise as JSON `null`, never `0`.
    pub(crate) display_order: Option<i64>,
    /// Verbatim from `ParameterView.access` (fix round 1, item 2). Shown,
    /// never used to gate `editable` -- D24 deliberately does not.
    pub(crate) access: Option<String>,
    /// The `ets_id` a `POST` must send to write this field (design D43):
    /// `Some(ets_id)` for an unscoped field, `Some(module-qualified id)`
    /// for an editable module-scoped one (D39's reconstruction), `None`
    /// exactly when `editable` is `false`. The panel is the single
    /// authority on what is writable -- the write path checks this field,
    /// not a second id-shape parser.
    pub(crate) write_ets_id: Option<String>,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnumOptionDto {
    pub(crate) value: String,
    pub(crate) text: Option<String>,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StaleParameterDto {
    pub(crate) ets_id: String,
    pub(crate) raw: String,
}

/// Every distinct `ParameterDiagnosticDto.message` this build can produce,
/// as a closed, machine-readable tag alongside the English prose —
/// KNOWN_LIMITATIONS.md §66's translated surface, the same
/// `CreationDiagnostic`/`CreationDiagnosticDto` idea (`tag = "kind"`
/// above) applied here. Unlike `CreationDiagnostic`, none of these
/// twenty-two messages interpolate a dynamic value — every id, count and
/// name they'd want to name already lives in `.detail` instead (design
/// D26) — so a bare tag is enough; there is no per-variant payload to
/// carry. `apps/knx-web/src/ParameterPanel.tsx`'s
/// `describeParameterDiagnosticMessage` maps each tag to its own
/// catalogue key and falls back to `.message` verbatim for a `kind` a
/// future server variant introduces before that frontend build knows
/// about it — the same fallback `describeCreationDiagnostic` uses.
#[derive(serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ParameterDiagnosticKindDto {
    /// `parameter_views`'s inner joins dropped a declared parameter.
    ParametersUnreadable,
    /// Two stored values target the same unscoped parameter.
    DuplicateUnscopedValue,
    /// Two stored values target the same module-scoped parameter.
    DuplicateModuleScopedValue,
    /// Two sections in one program declare the same module id (a
    /// malformed program).
    DuplicateModuleId,
    /// No imported `ModuleInstance` matches this module (D39 rule 2).
    NoModuleInstanceMatch,
    /// More than one imported `ModuleInstance` claims this module (D40).
    AmbiguousModuleInstance,
    /// An imported `ModuleInstance`'s id does not decompose as expected
    /// (D39 rule 3).
    MalformedModuleInstanceId,
    /// The remaining fifteen tags mirror `knx_productdb::dynamic::Diagnostic`'s
    /// own variants 1:1 (see `diagnostic_kind_and_message` in `domain.rs`).
    NoBranchMatched,
    UnparsableTest,
    UnresolvedParamRef,
    NonNumericValue,
    UnexpectedTypeNoneShape,
    UnrecognizedNode,
    ModuleDefNotFound,
    ModuleCycleDetected,
    ModuleNestingTooDeep,
    ModuleExpansionBudgetExhausted,
    MissingValue,
    ModuleWithoutId,
    /// Added by T12's module-argument work (main), merged into this
    /// Kind-tagged shape during T14's fix round 1 merge — see
    /// `diagnostic_kind_and_message`'s own comment on this trio.
    ModuleArgumentNotBound,
    UnsupportedModuleArgumentKind,
    UnresolvedTextPlaceholder,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParameterDiagnosticDto {
    pub(crate) scope: Option<ModuleScopeDto>,
    pub(crate) kind: ParameterDiagnosticKindDto,
    pub(crate) message: String,
    pub(crate) detail: String,
}

/// The optional display language, shared by both parameter endpoints
/// (T26 Task 2) and `device_detail` (T33 Task 2). Absent means `None` —
/// today's untranslated behaviour, unchanged. Follows `CatalogItemsQuery`'s
/// own precedent above.
#[derive(Deserialize)]
struct ParameterLanguageQuery {
    #[serde(default)]
    language: Option<String>,
}

async fn parameter_panel(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
    Query(q): Query<ParameterLanguageQuery>,
) -> Result<Json<ParameterPanelDto>, ApiError> {
    domain::parameter_panel_impl(&state, id, q.language.as_deref())
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetParameterValueRequest {
    ets_id: String,
    raw: String,
}

async fn set_parameter_value(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
    Query(q): Query<ParameterLanguageQuery>,
    Json(body): Json<SetParameterValueRequest>,
) -> Result<Json<ParameterPanelDto>, ApiError> {
    domain::set_parameter_value_impl(&state, id, body.ets_id, body.raw, q.language.as_deref())
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn install_catalog_package(
    State(state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Json<CatalogInstallReportDto>, ApiError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::with_status(error.status(), error.body_text()))?
    {
        let Some(filename) = field.file_name().map(str::to_owned) else {
            continue;
        };
        let bytes = field
            .bytes()
            .await
            .map_err(|error| ApiError::with_status(error.status(), error.body_text()))?;
        return domain::install_catalog_package_impl(&state, &filename, &bytes)
            .map(CatalogInstallReportDto::from)
            .map(Json)
            .map_err(|error| match error {
                domain::CatalogInstallError::BadRequest(error) => {
                    ApiError::bad_request(error.to_string())
                }
                domain::CatalogInstallError::Internal(error) => ApiError::internal(error),
            });
    }
    Err(ApiError::bad_request("no file field in catalog install"))
}

/// `path` is either an absolute host path (desktop, from a native OS
/// dialog) or one relative to `AppState::data_dir` (web, from
/// `FsPicker.tsx` or `/api/fs/upload`) — see `crate::paths` for how the
/// two are told apart and what confinement the relative case gets.
#[derive(Deserialize)]
pub(crate) struct PathBody {
    pub(crate) path: String,
    /// The opaque per-load id `loadProgress.ts` generates with
    /// `crypto.randomUUID()` (ADR-0023 fix round 3, F9) — read only by
    /// `import_project` and `open_native_project`, which hand it to
    /// `tracked_load` so it lands on the operation and comes back in
    /// every snapshot. Every other handler sharing this body type ignores
    /// it, and an omitted field defaults to `None` rather than failing to
    /// deserialize.
    #[serde(default, rename = "clientToken")]
    pub(crate) client_token: Option<String>,
}

/// The `LoadSnapshot` on the wire. Hand-written rather than derived on
/// the domain type, for the usual reason: the enums
/// live in a module that has no business knowing JSON exists, and their
/// wire spellings are already `as_str()` — one vocabulary shared with
/// `knx-etsproj` and `knx-app`, not a second one restated here.
///
/// `completed`/`total` are `null` for every phase without a real count,
/// which the frontend renders as an indeterminate indicator. There is no
/// timestamp field and there will not be one: ADR-0023 forbids progress
/// derived from elapsed time, and a client that cannot see the clock
/// cannot be tempted by it.
///
/// `client_token` is the id the client sent when it started this
/// operation (ADR-0023 fix round 3, F9), echoed verbatim, or `null` when
/// none was sent. `ownsOperation()` in `loadProgress.ts` is exact equality
/// against it — the whole of this round's ownership test, replacing the
/// three-round id/source heuristic it retires.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadProgressDto {
    operation_id: u64,
    kind: &'static str,
    source: String,
    phase: &'static str,
    completed: Option<u64>,
    total: Option<u64>,
    status: &'static str,
    error: Option<String>,
    client_token: Option<String>,
}

impl From<crate::LoadSnapshot> for LoadProgressDto {
    fn from(snapshot: crate::LoadSnapshot) -> Self {
        Self {
            operation_id: snapshot.operation_id,
            kind: snapshot.kind.as_str(),
            source: snapshot.source,
            phase: snapshot.phase.as_str(),
            completed: snapshot.completed,
            total: snapshot.total,
            status: snapshot.status.as_str(),
            error: snapshot.error,
            client_token: snapshot.client_token,
        }
    }
}

/// The current load operation, running or finished, or `null` when this
/// server run has never loaded a project (ADR-0023). Cheap on purpose:
/// this is polled every few hundred milliseconds while an import runs, and
/// it takes one short-lived lock that the import itself only ever holds
/// for the length of a field assignment.
async fn load_progress(State(state): State<SharedState>) -> Json<Option<LoadProgressDto>> {
    Json(state.load_operations.snapshot().map(LoadProgressDto::from))
}

async fn current_project(
    State(state): State<SharedState>,
) -> Result<Json<domain::CurrentProject>, ApiError> {
    domain::current_project_tree(&state)
        .map(Json)
        .map_err(ApiError::bad_request)
}

/// Refusing a second load while one runs: a state conflict the caller can
/// resolve by waiting, which is `409`, not `400` (see `errors.rs` on the
/// split). The UI disables its own buttons for the same reason, but a
/// command palette, a second tab and a script all reach these routes too.
fn already_running(conflict: crate::AlreadyRunning) -> ApiError {
    ApiError::with_status(
        axum::http::StatusCode::CONFLICT,
        format!(
            "a project load is already running (operation {}); wait for it to finish",
            conflict.operation_id
        ),
    )
}

/// Runs `work` on a blocking thread as one tracked load operation
/// (ADR-0023). Three things this shape buys, all of which the previous
/// `domain::open_project(&state, &path).map(Json)` lacked:
///
/// 1. The import no longer blocks a tokio worker for seconds — which is
///    also what makes the progress poll answer promptly while it runs.
/// 2. The operation outlives its request. A client that disconnects
///    cancels nothing; the project still lands, and the snapshot is
///    waiting for whoever polls next.
/// 3. A panic inside the import marks the operation failed through
///    `LoadHandle`'s `Drop`, instead of leaving the slot claimed for the
///    rest of the server's life.
async fn tracked_load(
    state: SharedState,
    kind: crate::LoadKind,
    path: std::path::PathBuf,
    client_token: Option<String>,
    work: fn(
        &crate::AppState,
        &std::path::Path,
        &crate::LoadHandle,
    ) -> Result<knx_projection::ProjectTree, String>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let handle = state
        .load_operations
        .begin(kind, domain::file_name_of(&path), client_token)
        .map_err(already_running)?;
    tokio::task::spawn_blocking(move || {
        let outcome = work(&state, &path, &handle);
        match &outcome {
            Ok(_) => handle.succeed(),
            Err(error) => handle.fail(error.clone()),
        }
        outcome
    })
    .await
    .map_err(|e| ApiError::internal(format!("the load task did not finish: {e}")))?
    .map(Json)
    .map_err(ApiError::internal)
}

async fn import_project(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let path = resolve_project_path(&state.data_dir, &body.path)?;
    tracked_load(
        state,
        crate::LoadKind::Import,
        path,
        body.client_token,
        domain::open_project,
    )
    .await
}

/// Every field optional, so `POST /api/project/new` with `{}` is a valid
/// request. `discard_changes` is the caller's explicit "yes, throw away the
/// edits I have not saved" — see [`domain::new_project_impl`] for why the
/// default is to refuse instead.
///
/// `group_address_style` is a `String` rather than a derived enum so that
/// an unrecognised value fails through [`parse_group_address_style`] with
/// a message naming the three it could have been, instead of serde's own
/// `422` on a body this handler would otherwise never see at all (`Option
/// <Json<..>>` above).
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct NewProjectBody {
    name: Option<String>,
    installation_name: Option<String>,
    language: Option<String>,
    group_address_style: Option<String>,
    discard_changes: bool,
}

/// The wire spelling of [`knx_core::GroupAddressStyle`]: the same three
/// tokens ETS writes into `ProjectInformation/@GroupAddressStyle`
/// (`knx-etsproj/src/map.rs`) and `knx-store` persists
/// (`knx-store/src/project.rs`'s `style_to_str`), so a client never has to
/// learn a fourth vocabulary for the same three choices.
///
/// Unknown values are refused, deliberately — unlike both of those
/// mappers, which fall back to `ThreeLevel` (the importer at least records
/// a `MapProblem` while doing so). Neither has a choice: they are reading
/// a document that already exists. These two routes are creating or
/// changing one, where a silently wrong style is discovered far too late —
/// `POST /api/project/group-address-style` can restyle a project after the
/// fact (T4), but a value nobody asked for is still a value nobody
/// notices. Data integrity over convenience.
fn parse_group_address_style(value: &str) -> Result<knx_core::GroupAddressStyle, ApiError> {
    match value {
        "Free" => Ok(knx_core::GroupAddressStyle::Free),
        "TwoLevel" => Ok(knx_core::GroupAddressStyle::TwoLevel),
        "ThreeLevel" => Ok(knx_core::GroupAddressStyle::ThreeLevel),
        other => Err(ApiError::bad_request(format!(
            "unknown groupAddressStyle {other:?}: expected Free, TwoLevel or ThreeLevel"
        ))),
    }
}

/// Creates an empty project without touching the filesystem — the only way
/// a project comes into existence that is not an ETS import or a `.knxdb`
/// load, and therefore the entry point for adding a device from the product
/// catalogue with no `.knxproj` anywhere in sight.
///
/// `409 Conflict`, not `400`: the open project's unsaved edits are a state
/// conflict the caller can resolve (save first, or re-send with
/// `discardChanges`), not a malformed request. An unrecognised
/// `groupAddressStyle` is the opposite case and does get the `400`: there
/// is nothing about the server's state to resolve, only the request.
///
/// A wrong choice here is no longer permanent — `POST
/// /api/project/group-address-style` restyles an existing project — but it
/// still refuses the same unrecognised values for the same reason: data
/// integrity over convenience applies at creation time too, not only when
/// changing the mind later.
async fn new_project(
    State(state): State<SharedState>,
    body: Option<Json<NewProjectBody>>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let Json(body) = body.unwrap_or_default();
    let group_address_style = body
        .group_address_style
        .as_deref()
        .map(parse_group_address_style)
        .transpose()?;
    domain::new_project_impl(
        &state,
        body.name,
        body.installation_name,
        body.language,
        group_address_style,
        body.discard_changes,
    )
    .map(Json)
    .map_err(|domain::UnsavedChanges| {
        ApiError::with_status(
            axum::http::StatusCode::CONFLICT,
            "the open project has unsaved changes; save it first or resend with \
             discardChanges: true",
        )
    })
}

async fn open_native_project(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let path = resolve_project_path(&state.data_dir, &body.path)?;
    tracked_load(
        state,
        crate::LoadKind::Open,
        path,
        body.client_token,
        domain::open_native_project,
    )
    .await
}

async fn save_project(State(state): State<SharedState>) -> Result<(), ApiError> {
    domain::save_project(&state).map_err(ApiError::internal)
}

async fn save_project_as(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<(), ApiError> {
    let path = resolve_new_project_path(&state.data_dir, &body.path)?;
    domain::save_project_as(&state, &path).map_err(ApiError::internal)
}

async fn device_detail(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
    Query(q): Query<ParameterLanguageQuery>,
) -> Result<Json<knx_projection::DeviceDetail>, ApiError> {
    domain::device_detail(&state, id, q.language.as_deref())
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetIndividualAddressBody {
    device_id: u32,
    address: Option<String>,
}

async fn set_individual_address(
    State(state): State<SharedState>,
    Json(body): Json<SetIndividualAddressBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::set_individual_address_impl(&state, body.device_id, body.address)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetDeviceDescriptionBody {
    device_id: u32,
    description: Option<String>,
}

async fn set_device_description(
    State(state): State<SharedState>,
    Json(body): Json<SetDeviceDescriptionBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::set_device_description_impl(&state, body.device_id, body.description)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetComObjectDptBody {
    com_object_id: u32,
    dpt: Option<String>,
}

async fn set_com_object_dpt(
    State(state): State<SharedState>,
    Json(body): Json<SetComObjectDptBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::set_com_object_dpt_impl(&state, body.com_object_id, body.dpt)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetComObjectDescriptionBody {
    com_object_id: u32,
    description: Option<String>,
}

async fn set_com_object_description(
    State(state): State<SharedState>,
    Json(body): Json<SetComObjectDescriptionBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::set_com_object_description_impl(&state, body.com_object_id, body.description)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetComObjectFlagBody {
    com_object_id: u32,
    flag: String,
    value: bool,
}

async fn set_com_object_flag(
    State(state): State<SharedState>,
    Json(body): Json<SetComObjectFlagBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::set_com_object_flag_impl(&state, body.com_object_id, body.flag, body.value)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetGroupAddressStyleBody {
    group_address_style: String,
}

struct GroupAddressStylePublication {
    tree: knx_projection::ProjectTree,
    ctx: Option<GroupAddressContext>,
}

fn prepare_group_address_context_publication(
    state: &SharedState,
    refresh_when_style_is_unchanged: bool,
    mutation: impl FnOnce(&crate::AppState) -> Result<knx_projection::ProjectTree, String>,
) -> Result<GroupAddressStylePublication, ApiError> {
    let previous_style = state
        .project
        .lock()
        .expect("project mutex poisoned")
        .as_ref()
        .map(|project| project.info.group_address_style);
    let mutation_tree = mutation(state.as_ref()).map_err(ApiError::bad_request)?;
    let current_style = match mutation_tree.group_address_style.as_str() {
        "Free" => Some(knx_core::GroupAddressStyle::Free),
        "TwoLevel" => Some(knx_core::GroupAddressStyle::TwoLevel),
        "ThreeLevel" => Some(knx_core::GroupAddressStyle::ThreeLevel),
        _ => None,
    };
    let (tree, ctx) = if refresh_when_style_is_unchanged || current_style != previous_style {
        let (tree, ctx) = domain::current_project_tree_and_group_address_context(state)
            .map_err(ApiError::bad_request)?;
        (tree, Some(ctx))
    } else {
        (mutation_tree, None)
    };
    Ok(GroupAddressStylePublication { tree, ctx })
}

#[cfg(test)]
fn prepare_group_address_style_publication(
    state: &SharedState,
    style: knx_core::GroupAddressStyle,
) -> Result<GroupAddressStylePublication, ApiError> {
    prepare_group_address_context_publication(state, true, |state| {
        domain::set_group_address_style_impl(state, style)
    })
}

async fn publish_group_address_style(
    state: &SharedState,
    mut publication: GroupAddressStylePublication,
) -> Json<knx_projection::ProjectTree> {
    if let Some(ctx) = publication.ctx {
        if let Some(session) = state.bus_session.lock().await.as_ref() {
            session.update_group_address_context(ctx);
            publication.tree.group_address_context_session_id = Some(session.id());
        }
    }
    Json(publication.tree)
}

async fn mutate_and_publish_group_address_context(
    state: &SharedState,
    refresh_when_style_is_unchanged: bool,
    mutation: impl FnOnce(&crate::AppState) -> Result<knx_projection::ProjectTree, String>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let _transaction = state.group_address_style_publication.lock().await;
    let publication = prepare_group_address_context_publication(
        state,
        refresh_when_style_is_unchanged,
        mutation,
    )?;
    Ok(publish_group_address_style(state, publication).await)
}

/// Restyles an already-open project. `400`, not `409`: unlike
/// `POST /api/project/new`'s unsaved-changes conflict, there is no state
/// here the caller could resolve by saving first — either every existing
/// group address fits the requested style or it does not, and `apply`'s
/// `bad_request` mapping already carries the offending address's id in the
/// message when it does not (`Command::SetGroupAddressStyle`'s own error).
async fn set_group_address_style(
    State(state): State<SharedState>,
    Json(body): Json<SetGroupAddressStyleBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let style = parse_group_address_style(&body.group_address_style)?;
    mutate_and_publish_group_address_context(&state, true, |state| {
        domain::set_group_address_style_impl(state, style)
    })
    .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateGroupAddressBody {
    name: String,
    address: String,
    #[serde(default)]
    range_id: Option<u32>,
}

async fn create_group_address(
    State(state): State<SharedState>,
    Json(body): Json<CreateGroupAddressBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_group_address_impl(&state, body.name, body.address, body.range_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_group_address(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_group_address_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

/// `CsvProblem` (`knx-csv`) does not derive `Serialize` — it knows nothing
/// of JSON — so it gets an explicit at-the-boundary conversion, like every
/// other domain type that reaches the wire. Doubles as both an import-side problem
/// and an export-side warning: `write.rs`'s own doc comment already
/// reuses `CsvProblem` for both rather than duplicating the shape, and
/// there is no reason for the DTO to duplicate it either.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CsvProblemDto {
    row: Option<usize>,
    severity: knx_csv::Severity,
    detail: String,
}

impl From<&knx_csv::CsvProblem> for CsvProblemDto {
    fn from(problem: &knx_csv::CsvProblem) -> Self {
        Self {
            row: problem.row,
            severity: problem.severity,
            detail: problem.detail.clone(),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CsvExportReportDto {
    warnings: Vec<CsvProblemDto>,
}

/// Writes the live project's group addresses to `body.path` as "KNXBench
/// group-address CSV v1" — see `crates/knx-csv` for the format, and its
/// own module docs for why it is never called "ETS CSV". `path` is a fresh
/// write target, resolved by `resolve_new_project_path` like every other
/// route that writes a file the user named.
async fn export_group_addresses_csv(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<CsvExportReportDto>, ApiError> {
    let path = resolve_new_project_path(&state.data_dir, &body.path)?;
    domain::export_group_addresses_csv_impl(&state, &path)
        .map(|export| CsvExportReportDto {
            warnings: export.warnings.iter().map(CsvProblemDto::from).collect(),
        })
        .map(Json)
        .map_err(ApiError::bad_request)
}

/// `ReportWarning` (`knx-report`) does not derive `Serialize` — that crate
/// has no `serde` dependency at all, deliberately (see its own module
/// docs) — so it gets the same explicit at-the-boundary conversion
/// `CsvProblemDto` above uses.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentationWarningDto {
    location: String,
    detail: String,
}

impl From<&knx_report::ReportWarning> for DocumentationWarningDto {
    fn from(warning: &knx_report::ReportWarning) -> Self {
        Self {
            location: warning.location.clone(),
            detail: warning.detail.clone(),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentationExportReportDto {
    warnings: Vec<DocumentationWarningDto>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentationPreviewReportDto {
    html: String,
    warnings: Vec<DocumentationWarningDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentationOptionsBody {
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    sections: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentationExportBody {
    path: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    sections: Option<Vec<String>>,
}

fn documentation_options(
    language: Option<&str>,
    sections: Option<&[String]>,
) -> Result<
    (
        knx_report::ReportLanguage,
        std::collections::BTreeSet<knx_report::ReportSection>,
    ),
    ApiError,
> {
    let language = match language.unwrap_or("en") {
        "en" | "en-US" => knx_report::ReportLanguage::English,
        "de" | "de-DE" => knx_report::ReportLanguage::German,
        other => {
            return Err(ApiError::bad_request(format!(
                "unsupported report language: {other}"
            )))
        }
    };
    let sections = match sections {
        None => knx_report::ReportSection::ALL.into_iter().collect(),
        Some(values) => {
            let mut selected = std::collections::BTreeSet::new();
            for value in values {
                let section = match value.as_str() {
                    "summary" => knx_report::ReportSection::Summary,
                    "topology" => knx_report::ReportSection::Topology,
                    "buildings" => knx_report::ReportSection::Buildings,
                    "groupAddresses" => knx_report::ReportSection::GroupAddresses,
                    "devices" => knx_report::ReportSection::Devices,
                    other => {
                        return Err(ApiError::bad_request(format!(
                            "unsupported report section: {other}"
                        )))
                    }
                };
                selected.insert(section);
            }
            selected
        }
    };
    Ok((language, sections))
}

/// Writes the live project as one self-contained "project documentation"
/// HTML file to `body.path` (`crates/knx-report`) — never called an "ETS
/// report" anywhere, because no ETS-produced sample exists in this
/// repository to be compatible with (see `knx-report`'s own module docs).
/// `path` is a fresh write target, resolved exactly like
/// `/api/group-addresses/csv-export`'s.
async fn export_documentation(
    State(state): State<SharedState>,
    Json(body): Json<DocumentationExportBody>,
) -> Result<Json<DocumentationExportReportDto>, ApiError> {
    let (language, sections) =
        documentation_options(body.language.as_deref(), body.sections.as_deref())?;
    let path = resolve_new_project_path(&state.data_dir, &body.path)?;
    domain::export_documentation_impl(&state, &path, language, sections)
        .map(|report| DocumentationExportReportDto {
            warnings: report
                .warnings
                .iter()
                .map(DocumentationWarningDto::from)
                .collect(),
        })
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn preview_documentation(
    State(state): State<SharedState>,
    Json(body): Json<DocumentationOptionsBody>,
) -> Result<Json<DocumentationPreviewReportDto>, ApiError> {
    let (language, sections) =
        documentation_options(body.language.as_deref(), body.sections.as_deref())?;
    domain::preview_documentation_impl(&state, language, sections)
        .map(|report| {
            Json(DocumentationPreviewReportDto {
                warnings: report
                    .warnings
                    .iter()
                    .map(DocumentationWarningDto::from)
                    .collect(),
                html: report.html,
            })
        })
        .map_err(ApiError::bad_request)
}

// ---------------------------------------------------------------------
// POST /api/project/diff (T14) — `knx_diff::*` does not derive `Serialize`
// (module doc, `crates/knx-diff/src/lib.rs`), so every type it returns
// gets an explicit DTO here, `#[derive(serde::Serialize)]`, `camelCase`,
// one `From<&knx_diff::X>` per type — the same pattern
// `DocumentationWarningDto`/`DocumentationExportReportDto` above already
// establish. `EntityTable<K, F>`/`EntityChange<K, F>`/`AmbiguityNote<K>`
// are generic in `knx-diff` itself, so one generic DTO trio serves every
// entity table except devices (`DeviceTable`/`DeviceChange`, bespoke in
// `knx-diff` too, for the same reason: nesting a device's communication
// objects and parameters inside a `Fields` type used on both sides of a
// generic `EntityChange` would duplicate the nested diff meaninglessly).
// ---------------------------------------------------------------------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct EntityChangeDto<K: serde::Serialize, F: serde::Serialize> {
    key: K,
    matched_by: MatchKindDto,
    left: F,
    right: F,
    changed_fields: Vec<&'static str>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AmbiguityNoteDto<K: serde::Serialize> {
    key: K,
    left_candidates: usize,
    right_candidates: usize,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct EntityTableDto<K: serde::Serialize, F: serde::Serialize> {
    added: Vec<(K, F)>,
    removed: Vec<(K, F)>,
    changed: Vec<EntityChangeDto<K, F>>,
    ambiguous: Vec<AmbiguityNoteDto<K>>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum MatchKindDto {
    EtsId,
    NaturalKey,
}

impl From<&knx_diff::MatchKind> for MatchKindDto {
    fn from(k: &knx_diff::MatchKind) -> Self {
        match k {
            knx_diff::MatchKind::EtsId => Self::EtsId,
            knx_diff::MatchKind::NaturalKey => Self::NaturalKey,
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum EntityStatusDto {
    Added,
    Removed,
    Matched,
}

impl From<&knx_diff::EntityStatus> for EntityStatusDto {
    fn from(status: &knx_diff::EntityStatus) -> Self {
        match status {
            knx_diff::EntityStatus::Added => Self::Added,
            knx_diff::EntityStatus::Removed => Self::Removed,
            knx_diff::EntityStatus::Matched => Self::Matched,
        }
    }
}

/// Converts one `knx_diff::EntityTable<K, F>` into its DTO given the two
/// per-entity `From<&K>`/`From<&F>` conversions below — shared by every
/// generic entity table (areas, lines, buildings, group ranges, group
/// addresses, communication objects, parameters: seven call sites) so the
/// traversal lives once, not once per entity type. `Vec` order is
/// preserved throughout — nothing here re-sorts or re-groups by key,
/// which is what would silently reintroduce the `HashMap`-ordering problem
/// `knx-diff` itself was built to avoid.
fn convert_table<K, F, KD, FD>(table: &knx_diff::EntityTable<K, F>) -> EntityTableDto<KD, FD>
where
    KD: serde::Serialize + for<'a> From<&'a K>,
    FD: serde::Serialize + for<'a> From<&'a F>,
{
    EntityTableDto {
        added: table
            .added
            .iter()
            .map(|(k, f)| (KD::from(k), FD::from(f)))
            .collect(),
        removed: table
            .removed
            .iter()
            .map(|(k, f)| (KD::from(k), FD::from(f)))
            .collect(),
        changed: table
            .changed
            .iter()
            .map(|c| EntityChangeDto {
                key: KD::from(&c.key),
                matched_by: MatchKindDto::from(&c.matched_by),
                left: FD::from(&c.left),
                right: FD::from(&c.right),
                changed_fields: c.changed_fields.clone(),
            })
            .collect(),
        ambiguous: table
            .ambiguous
            .iter()
            .map(|a| AmbiguityNoteDto {
                key: KD::from(&a.key),
                left_candidates: a.left_candidates,
                right_candidates: a.right_candidates,
            })
            .collect(),
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AreaKeyDto {
    address: u8,
}

impl From<&knx_diff::AreaKey> for AreaKeyDto {
    fn from(key: &knx_diff::AreaKey) -> Self {
        Self {
            address: key.address,
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AreaFieldsDto {
    name: String,
    /// `CompletionStatus`'s `Debug` form (e.g. `"FinishedDesign"`) — same
    /// convention `knx_projection::BuildingNode::kind` already documents
    /// for `BuildingPartType`: not worth a typed TS union for a single
    /// label.
    completion: String,
}

impl From<&knx_diff::AreaFields> for AreaFieldsDto {
    fn from(fields: &knx_diff::AreaFields) -> Self {
        Self {
            name: fields.name.clone(),
            completion: format!("{:?}", fields.completion),
        }
    }
}

type AreaTableDto = EntityTableDto<AreaKeyDto, AreaFieldsDto>;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LineKeyDto {
    area_address: u8,
    line_address: u8,
}

impl From<&knx_diff::LineKey> for LineKeyDto {
    fn from(key: &knx_diff::LineKey) -> Self {
        Self {
            area_address: key.area_address,
            line_address: key.line_address,
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LineFieldsDto {
    name: String,
    medium_ref: String,
    domain_address: Option<String>,
    domain_address_is_checked: Option<bool>,
    ip_routing_multicast_address: Option<String>,
    multicast_ttl: Option<u8>,
    completion: String,
    area: Option<AreaKeyDto>,
}

impl From<&knx_diff::LineFields> for LineFieldsDto {
    fn from(fields: &knx_diff::LineFields) -> Self {
        Self {
            name: fields.name.clone(),
            medium_ref: fields.medium_ref.clone(),
            domain_address: fields.domain_address.clone(),
            domain_address_is_checked: fields.domain_address_is_checked,
            ip_routing_multicast_address: fields
                .ip_routing_multicast_address
                .map(|addr| addr.to_string()),
            multicast_ttl: fields.multicast_ttl,
            completion: format!("{:?}", fields.completion),
            area: fields.area.as_ref().map(AreaKeyDto::from),
        }
    }
}

type LineTableDto = EntityTableDto<LineKeyDto, LineFieldsDto>;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildingPartKeyDto {
    path: Vec<String>,
}

impl From<&knx_diff::BuildingPartKey> for BuildingPartKeyDto {
    fn from(key: &knx_diff::BuildingPartKey) -> Self {
        Self {
            path: key.path.clone(),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildingPartFieldsDto {
    name: String,
    number: Option<String>,
    /// `BuildingPartType`'s `Debug` form — same convention as `AreaFieldsDto::completion`.
    kind: String,
    completion: String,
    default_line: Option<LineKeyDto>,
}

impl From<&knx_diff::BuildingPartFields> for BuildingPartFieldsDto {
    fn from(fields: &knx_diff::BuildingPartFields) -> Self {
        Self {
            name: fields.name.clone(),
            number: fields.number.clone(),
            kind: format!("{:?}", fields.kind),
            completion: format!("{:?}", fields.completion),
            default_line: fields.default_line.as_ref().map(LineKeyDto::from),
        }
    }
}

type BuildingTableDto = EntityTableDto<BuildingPartKeyDto, BuildingPartFieldsDto>;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceKeyDto {
    ets_id: Option<String>,
    address: Option<String>,
}

impl From<&knx_diff::DeviceKey> for DeviceKeyDto {
    fn from(key: &knx_diff::DeviceKey) -> Self {
        Self {
            ets_id: key.ets_id.clone(),
            address: key.address.clone(),
        }
    }
}

/// Field-for-field copy of `knx_core::CommissioningState`, which — like
/// every `knx-diff`/`knx-core` type here — does not derive `Serialize`.
/// `last_modified`/`last_download` become RFC3339 strings: `chrono`'s
/// `serde` feature is not enabled workspace-wide, and `session_log.rs`
/// already established `to_rfc3339()` as this codebase's own convention
/// for putting a `DateTime<Utc>` on the wire.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CommissioningStateDto {
    completion: String,
    individual_address_loaded: bool,
    application_program_loaded: bool,
    parameters_loaded: bool,
    communication_part_loaded: bool,
    medium_config_loaded: bool,
    last_modified: Option<String>,
    last_download: Option<String>,
    broken: bool,
}

impl From<&knx_core::CommissioningState> for CommissioningStateDto {
    fn from(state: &knx_core::CommissioningState) -> Self {
        Self {
            completion: format!("{:?}", state.completion),
            individual_address_loaded: state.individual_address_loaded,
            application_program_loaded: state.application_program_loaded,
            parameters_loaded: state.parameters_loaded,
            communication_part_loaded: state.communication_part_loaded,
            medium_config_loaded: state.medium_config_loaded,
            last_modified: state.last_modified.map(|d| d.to_rfc3339()),
            last_download: state.last_download.map(|d| d.to_rfc3339()),
            broken: state.broken,
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceFieldsDto {
    name: String,
    description: Option<String>,
    address: Option<String>,
    product_ref: String,
    program_ref: String,
    commissioning: CommissioningStateDto,
    line: Option<LineKeyDto>,
    building: Option<BuildingPartKeyDto>,
}

impl From<&knx_diff::DeviceFields> for DeviceFieldsDto {
    fn from(fields: &knx_diff::DeviceFields) -> Self {
        Self {
            name: fields.name.clone(),
            description: fields.description.clone(),
            address: fields.address.clone(),
            product_ref: fields.product_ref.clone(),
            program_ref: fields.program_ref.clone(),
            commissioning: CommissioningStateDto::from(&fields.commissioning),
            line: fields.line.as_ref().map(LineKeyDto::from),
            building: fields.building.as_ref().map(BuildingPartKeyDto::from),
        }
    }
}

/// Devices' own, non-generic table — mirrors `knx_diff::DeviceTable`
/// field for field, same reasoning as the crate's own doc comment on
/// `DeviceTable` for why it doesn't fit `EntityTableDto`.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceTableDto {
    added: Vec<(DeviceKeyDto, DeviceFieldsDto)>,
    removed: Vec<(DeviceKeyDto, DeviceFieldsDto)>,
    changed: Vec<DeviceChangeDto>,
    ambiguous: Vec<AmbiguityNoteDto<DeviceKeyDto>>,
}

impl From<&knx_diff::DeviceTable> for DeviceTableDto {
    fn from(table: &knx_diff::DeviceTable) -> Self {
        Self {
            added: table
                .added
                .iter()
                .map(|(k, f)| (DeviceKeyDto::from(k), DeviceFieldsDto::from(f)))
                .collect(),
            removed: table
                .removed
                .iter()
                .map(|(k, f)| (DeviceKeyDto::from(k), DeviceFieldsDto::from(f)))
                .collect(),
            changed: table.changed.iter().map(DeviceChangeDto::from).collect(),
            ambiguous: table
                .ambiguous
                .iter()
                .map(|a| AmbiguityNoteDto {
                    key: DeviceKeyDto::from(&a.key),
                    left_candidates: a.left_candidates,
                    right_candidates: a.right_candidates,
                })
                .collect(),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceChangeDto {
    key: DeviceKeyDto,
    matched_by: MatchKindDto,
    left: DeviceFieldsDto,
    right: DeviceFieldsDto,
    changed_fields: Vec<&'static str>,
    com_objects: ComObjectTableDto,
    parameters: ParameterTableDto,
}

impl From<&knx_diff::DeviceChange> for DeviceChangeDto {
    fn from(change: &knx_diff::DeviceChange) -> Self {
        Self {
            key: DeviceKeyDto::from(&change.key),
            matched_by: MatchKindDto::from(&change.matched_by),
            left: DeviceFieldsDto::from(&change.left),
            right: DeviceFieldsDto::from(&change.right),
            changed_fields: change.changed_fields.clone(),
            com_objects: convert_table(&change.com_objects),
            parameters: convert_table(&change.parameters),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GroupRangeKeyDto {
    start: u16,
    end: u16,
}

impl From<&knx_diff::GroupRangeKey> for GroupRangeKeyDto {
    fn from(key: &knx_diff::GroupRangeKey) -> Self {
        Self {
            start: key.start,
            end: key.end,
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GroupRangeFieldsDto {
    name: String,
    start: u16,
    end: u16,
    parent: Option<GroupRangeKeyDto>,
}

impl From<&knx_diff::GroupRangeFields> for GroupRangeFieldsDto {
    fn from(fields: &knx_diff::GroupRangeFields) -> Self {
        Self {
            name: fields.name.clone(),
            start: fields.start,
            end: fields.end,
            parent: fields.parent.as_ref().map(GroupRangeKeyDto::from),
        }
    }
}

type GroupRangeTableDto = EntityTableDto<GroupRangeKeyDto, GroupRangeFieldsDto>;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GroupAddressKeyDto {
    ets_id: Option<String>,
    address: String,
}

impl From<&knx_diff::GroupAddressKey> for GroupAddressKeyDto {
    fn from(key: &knx_diff::GroupAddressKey) -> Self {
        Self {
            ets_id: key.ets_id.clone(),
            address: key.address.clone(),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GroupAddressFieldsDto {
    name: String,
    central: bool,
    unfiltered: bool,
    range: Option<GroupRangeKeyDto>,
}

impl From<&knx_diff::GroupAddressFields> for GroupAddressFieldsDto {
    fn from(fields: &knx_diff::GroupAddressFields) -> Self {
        Self {
            name: fields.name.clone(),
            central: fields.central,
            unfiltered: fields.unfiltered,
            range: fields.range.as_ref().map(GroupRangeKeyDto::from),
        }
    }
}

type GroupAddressTableDto = EntityTableDto<GroupAddressKeyDto, GroupAddressFieldsDto>;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ComObjectKeyDto {
    device: DeviceKeyDto,
    number: u16,
}

impl From<&knx_diff::ComObjectKey> for ComObjectKeyDto {
    fn from(key: &knx_diff::ComObjectKey) -> Self {
        Self {
            device: DeviceKeyDto::from(&key.device),
            number: key.number,
        }
    }
}

/// One `ComObjectFields::links` entry: a group-address key plus the
/// direction the communication object uses it in. `direction` is
/// `Direction`'s `Debug` form (`"Send"`/`"Receive"`), same convention
/// `knx_projection::GroupLinkNode::direction` already documents.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ComObjectLinkDto {
    group_address: GroupAddressKeyDto,
    direction: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ComObjectFieldsDto {
    text: Option<String>,
    description: Option<String>,
    dpt: Option<String>,
    read: Option<bool>,
    write: Option<bool>,
    transmit: Option<bool>,
    update: Option<bool>,
    communication: Option<bool>,
    read_on_init: Option<bool>,
    links: Vec<ComObjectLinkDto>,
    module_instance: Option<String>,
}

impl From<&knx_diff::ComObjectFields> for ComObjectFieldsDto {
    fn from(fields: &knx_diff::ComObjectFields) -> Self {
        Self {
            text: fields.text.clone(),
            description: fields.description.clone(),
            dpt: fields.dpt.clone(),
            read: fields.read,
            write: fields.write,
            transmit: fields.transmit,
            update: fields.update,
            communication: fields.communication,
            read_on_init: fields.read_on_init,
            links: fields
                .links
                .iter()
                .map(|(ga, direction)| ComObjectLinkDto {
                    group_address: GroupAddressKeyDto::from(ga),
                    direction: format!("{direction:?}"),
                })
                .collect(),
            module_instance: fields.module_instance.clone(),
        }
    }
}

type ComObjectTableDto = EntityTableDto<ComObjectKeyDto, ComObjectFieldsDto>;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ParameterKeyDto {
    device: DeviceKeyDto,
    ets_id: String,
}

impl From<&knx_diff::ParameterKey> for ParameterKeyDto {
    fn from(key: &knx_diff::ParameterKey) -> Self {
        Self {
            device: DeviceKeyDto::from(&key.device),
            ets_id: key.ets_id.clone(),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ParameterFieldsDto {
    raw: String,
}

impl From<&knx_diff::ParameterFields> for ParameterFieldsDto {
    fn from(fields: &knx_diff::ParameterFields) -> Self {
        Self {
            raw: fields.raw.clone(),
        }
    }
}

type ParameterTableDto = EntityTableDto<ParameterKeyDto, ParameterFieldsDto>;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FieldChangeDto {
    field: &'static str,
    left: String,
    right: String,
}

impl From<&knx_diff::FieldChange> for FieldChangeDto {
    fn from(change: &knx_diff::FieldChange) -> Self {
        Self {
            field: change.field,
            left: change.left.clone(),
            right: change.right.clone(),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallationDiffDto {
    id: u8,
    status: EntityStatusDto,
    field_changes: Vec<FieldChangeDto>,
    areas: AreaTableDto,
    lines: LineTableDto,
    devices: DeviceTableDto,
    group_ranges: GroupRangeTableDto,
    group_addresses: GroupAddressTableDto,
    buildings: BuildingTableDto,
}

impl From<&knx_diff::InstallationDiff> for InstallationDiffDto {
    fn from(diff: &knx_diff::InstallationDiff) -> Self {
        Self {
            id: diff.id,
            status: EntityStatusDto::from(&diff.status),
            field_changes: diff
                .field_changes
                .iter()
                .map(FieldChangeDto::from)
                .collect(),
            areas: convert_table(&diff.areas),
            lines: convert_table(&diff.lines),
            devices: DeviceTableDto::from(&diff.devices),
            group_ranges: convert_table(&diff.group_ranges),
            group_addresses: convert_table(&diff.group_addresses),
            buildings: convert_table(&diff.buildings),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectDiffDto {
    info_changes: Vec<FieldChangeDto>,
    installations: Vec<InstallationDiffDto>,
}

impl From<&knx_diff::ProjectDiff> for ProjectDiffDto {
    fn from(diff: &knx_diff::ProjectDiff) -> Self {
        Self {
            info_changes: diff.info_changes.iter().map(FieldChangeDto::from).collect(),
            installations: diff
                .installations
                .iter()
                .map(InstallationDiffDto::from)
                .collect(),
        }
    }
}

/// Compares the server's live, possibly edited, in-memory project against
/// the `.knxdb` file at `body.path` — "what would Save change", not a
/// comparison of two files on disk (design spec
/// `docs/superpowers/specs/2026-09-10-project-diff-design.md` §7). `path`
/// is resolved with `resolve_project_path`, a *read* of a file that must
/// already exist — same function `import_project`/`open_native_project`
/// use, never `resolve_new_project_path`.
///
/// Every failure `domain::diff_project_impl` can return — "no project
/// open" or "comparison file does not exist" — is the caller's to fix
/// relative to a project that may already be open. That is different from
/// `import_project`/`open_native_project`, which read the *only* project a
/// route establishes, so *their* failures are environment problems mapped
/// to `ApiError::internal` (`errors.rs`'s own documented 400/500 split).
/// This route therefore maps its whole result to `ApiError::bad_request`
/// instead — the same uniform mapping `export_documentation` above already
/// uses. Do not "fix" this back to `ApiError::internal` by analogy with
/// `import_project`/`open_native_project` without re-reading this comment
/// first.
async fn diff_project(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<ProjectDiffDto>, ApiError> {
    let path = resolve_project_path(&state.data_dir, &body.path)?;
    domain::diff_project_impl(&state, &path)
        .map(|diff| ProjectDiffDto::from(&diff))
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum IgnoredColumnReasonDto {
    ExportOnly,
    Unknown,
}

impl From<knx_csv::IgnoredColumnReason> for IgnoredColumnReasonDto {
    fn from(reason: knx_csv::IgnoredColumnReason) -> Self {
        match reason {
            knx_csv::IgnoredColumnReason::ExportOnly => Self::ExportOnly,
            knx_csv::IgnoredColumnReason::Unknown => Self::Unknown,
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct IgnoredColumnDto {
    name: String,
    reason: IgnoredColumnReasonDto,
}

impl From<&knx_csv::IgnoredColumn> for IgnoredColumnDto {
    fn from(column: &knx_csv::IgnoredColumn) -> Self {
        Self {
            name: column.name.clone(),
            reason: column.reason.into(),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CsvImportReportDto {
    separator: char,
    rows_read: usize,
    created: usize,
    updated: usize,
    unchanged: usize,
    ignored_columns: Vec<IgnoredColumnDto>,
    problems: Vec<CsvProblemDto>,
}

impl From<&knx_csv::CsvImportReport> for CsvImportReportDto {
    fn from(report: &knx_csv::CsvImportReport) -> Self {
        Self {
            separator: report.separator,
            rows_read: report.rows_read,
            created: report.created,
            updated: report.updated,
            unchanged: report.unchanged,
            ignored_columns: report.ignored_columns.iter().map(Into::into).collect(),
            problems: report.problems.iter().map(Into::into).collect(),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CsvImportResponseDto {
    tree: knx_projection::ProjectTree,
    report: CsvImportReportDto,
}

/// Reads `body.path` as "KNXBench group-address CSV v1" and plans/applies
/// the edit against the live project (design §4). A file that parses but
/// contains a row-level error is the caller's mistake, not this server's —
/// `domain::import_group_addresses_csv_impl` returns `Err` naming every
/// offending row, mapped to `400` like every other project-editing route,
/// never `500`. `path` is a read target, resolved exactly like
/// `/api/project/import`'s.
async fn import_group_addresses_csv(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<CsvImportResponseDto>, ApiError> {
    let path = resolve_project_path(&state.data_dir, &body.path)?;
    domain::import_group_addresses_csv_impl(&state, &path)
        .map(|(tree, report)| CsvImportResponseDto {
            tree,
            report: CsvImportReportDto::from(&report),
        })
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
struct CreateAreaBody {
    name: String,
    address: u8,
}

async fn create_area(
    State(state): State<SharedState>,
    Json(body): Json<CreateAreaBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_area_impl(&state, body.name, body.address)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_area(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_area_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateLineBody {
    area_id: u32,
    name: String,
    address: u8,
    medium_ref: String,
}

async fn create_line(
    State(state): State<SharedState>,
    Json(body): Json<CreateLineBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_line_impl(
        &state,
        body.area_id,
        body.name,
        body.address,
        body.medium_ref,
    )
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn delete_line(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_line_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveDeviceBody {
    device_id: u32,
    line_id: Option<u32>,
}

async fn move_device_to_line(
    State(state): State<SharedState>,
    Json(body): Json<MoveDeviceBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::move_device_to_line_impl(&state, body.device_id, body.line_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateGroupRangeBody {
    name: String,
    start: String,
    end: String,
    #[serde(default)]
    parent_id: Option<u32>,
}

async fn create_group_range(
    State(state): State<SharedState>,
    Json(body): Json<CreateGroupRangeBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_group_range_impl(&state, body.name, body.start, body.end, body.parent_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_group_range(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_group_range_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
struct RenameGroupRangeBody {
    name: String,
}

async fn rename_group_range(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
    Json(body): Json<RenameGroupRangeBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::rename_group_range_impl(&state, id, body.name)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBuildingPartBody {
    name: String,
    kind: String,
    #[serde(default)]
    parent_id: Option<u32>,
}

async fn create_building_part(
    State(state): State<SharedState>,
    Json(body): Json<CreateBuildingPartBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_building_part_impl(&state, body.name, body.kind, body.parent_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_building_part(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_building_part_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
struct RenameBuildingPartBody {
    name: String,
}

async fn rename_building_part(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
    Json(body): Json<RenameBuildingPartBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::rename_building_part_impl(&state, id, body.name)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveDeviceToBuildingPartBody {
    device_id: u32,
    part_id: Option<u32>,
}

async fn move_device_to_building_part(
    State(state): State<SharedState>,
    Json(body): Json<MoveDeviceToBuildingPartBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::move_device_to_building_part_impl(&state, body.device_id, body.part_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupLinkBody {
    com_object_id: u32,
    ga_id: u32,
    direction: String,
}

async fn link_com_object(
    State(state): State<SharedState>,
    Json(body): Json<GroupLinkBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::link_com_object_impl(&state, body.com_object_id, body.ga_id, body.direction)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn unlink_com_object(
    State(state): State<SharedState>,
    Json(body): Json<GroupLinkBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::unlink_com_object_impl(&state, body.com_object_id, body.ga_id, body.direction)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(serde::Serialize)]
struct CatalogManufacturerDto {
    id: String,
    name: Option<String>,
}

async fn catalog_manufacturers(
    State(state): State<SharedState>,
) -> Result<Json<Vec<CatalogManufacturerDto>>, ApiError> {
    domain::catalog_manufacturers_impl(&state)
        .map(|rows| {
            Json(
                rows.into_iter()
                    .map(|(id, name)| CatalogManufacturerDto { id, name })
                    .collect(),
            )
        })
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
struct CatalogItemsQuery {
    #[serde(default)]
    manufacturer: Option<String>,
    #[serde(default)]
    search: Option<String>,
    /// The optional display language (T32 Task 4). Absent means `None` —
    /// today's untranslated behaviour, unchanged. Follows
    /// `ParameterLanguageQuery`'s own precedent above.
    #[serde(default)]
    language: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogItemDto {
    id: String,
    manufacturer_id: String,
    name: Option<String>,
    number: Option<String>,
    visible_description: Option<String>,
    product_ref_id: Option<String>,
    hardware2program_ref_id: Option<String>,
}

impl From<knx_productdb::query::CatalogItemRow> for CatalogItemDto {
    fn from(r: knx_productdb::query::CatalogItemRow) -> Self {
        Self {
            id: r.id,
            manufacturer_id: r.manufacturer_id,
            name: r.name,
            number: r.number,
            visible_description: r.visible_description,
            product_ref_id: r.product_ref_id,
            hardware2program_ref_id: r.hardware2program_ref_id,
        }
    }
}

async fn catalog_items(
    State(state): State<SharedState>,
    Query(q): Query<CatalogItemsQuery>,
) -> Result<Json<Vec<CatalogItemDto>>, ApiError> {
    domain::catalog_items_impl(&state, q.manufacturer, q.search, q.language)
        .map(|rows| Json(rows.into_iter().map(CatalogItemDto::from).collect()))
        .map_err(ApiError::bad_request)
}

/// One language identifier the product database has any translation rows
/// for, with its row count — backs the Settings panel's "Product data
/// language" control (T26). `rows` is a plain `i64` count, not a UI-facing
/// judgement of completeness.
#[derive(serde::Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductLanguageDto {
    pub(crate) language: String,
    pub(crate) rows: i64,
}

impl From<knx_productdb::query::TranslationLanguage> for ProductLanguageDto {
    fn from(t: knx_productdb::query::TranslationLanguage) -> Self {
        Self {
            language: t.language,
            rows: t.rows,
        }
    }
}

/// `GET /api/product-languages`. With no product database configured this
/// returns `200 []`, not an error — see `domain::product_languages_impl`'s
/// own doc comment for why.
async fn product_languages(
    State(state): State<SharedState>,
) -> Result<Json<Vec<ProductLanguageDto>>, ApiError> {
    domain::product_languages_impl(&state)
        .map(|rows| Json(rows.into_iter().map(ProductLanguageDto::from).collect()))
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateDeviceBody {
    #[serde(default)]
    line_id: Option<u32>,
    catalog_item_id: String,
    name: String,
}

#[derive(serde::Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
enum CreationDiagnosticDto {
    ProgramlessProduct {
        catalog_item_id: String,
        detail: String,
    },
    AmbiguousDpt {
        ref_id: String,
        alternatives: Vec<String>,
        detail: String,
    },
    ComObjectRefMissing {
        ref_id: String,
        detail: String,
    },
    ProgramRefMissing {
        program_ref: String,
        detail: String,
    },
    DynamicOrModuleNotEvaluated {
        program_id: String,
        detail: String,
    },
}

impl From<domain::CreationDiagnostic> for CreationDiagnosticDto {
    fn from(value: domain::CreationDiagnostic) -> Self {
        // `detail()` is computed from `value` before it's moved apart below —
        // it stays the single source of truth for the user-visible sentence
        // (design doc: each diagnostic carries "a machine-readable kind and
        // user-visible detail"), the DTO never re-derives its own wording.
        let detail = value.detail();
        match value {
            domain::CreationDiagnostic::ProgramlessProduct { catalog_item_id } => {
                Self::ProgramlessProduct {
                    catalog_item_id,
                    detail,
                }
            }
            domain::CreationDiagnostic::AmbiguousDpt {
                ref_id,
                alternatives,
            } => Self::AmbiguousDpt {
                ref_id,
                alternatives,
                detail,
            },
            domain::CreationDiagnostic::ComObjectRefMissing { ref_id } => {
                Self::ComObjectRefMissing { ref_id, detail }
            }
            domain::CreationDiagnostic::ProgramRefMissing { program_ref } => {
                Self::ProgramRefMissing {
                    program_ref,
                    detail,
                }
            }
            domain::CreationDiagnostic::DynamicOrModuleNotEvaluated { program_id } => {
                Self::DynamicOrModuleNotEvaluated { program_id, detail }
            }
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateDeviceResponseDto {
    tree: knx_projection::ProjectTree,
    diagnostics: Vec<CreationDiagnosticDto>,
}

impl From<domain::CreateDeviceResponse> for CreateDeviceResponseDto {
    fn from(value: domain::CreateDeviceResponse) -> Self {
        Self {
            tree: value.tree,
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        }
    }
}

async fn create_device(
    State(state): State<SharedState>,
    Json(body): Json<CreateDeviceBody>,
) -> Result<Json<CreateDeviceResponseDto>, ApiError> {
    domain::create_device_impl(&state, body.line_id, body.catalog_item_id, body.name)
        .map(CreateDeviceResponseDto::from)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_device(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_device_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
struct BatchIdsBody {
    ids: Vec<u32>,
}

async fn batch_delete_devices(
    State(state): State<SharedState>,
    Json(body): Json<BatchIdsBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::batch_delete_devices_impl(&state, body.ids)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn batch_delete_group_addresses(
    State(state): State<SharedState>,
    Json(body): Json<BatchIdsBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::batch_delete_group_addresses_impl(&state, body.ids)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchMoveDevicesToLineBody {
    device_ids: Vec<u32>,
    line_id: Option<u32>,
}

async fn batch_move_devices_to_line(
    State(state): State<SharedState>,
    Json(body): Json<BatchMoveDevicesToLineBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::batch_move_devices_to_line_impl(&state, body.device_ids, body.line_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchMoveDevicesToBuildingPartBody {
    device_ids: Vec<u32>,
    building_part_id: Option<u32>,
}

async fn batch_move_devices_to_building_part(
    State(state): State<SharedState>,
    Json(body): Json<BatchMoveDevicesToBuildingPartBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::batch_move_devices_to_building_part_impl(&state, body.device_ids, body.building_part_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn undo(
    State(state): State<SharedState>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    mutate_and_publish_group_address_context(&state, false, domain::undo_impl).await
}

async fn redo(
    State(state): State<SharedState>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    mutate_and_publish_group_address_context(&state, false, domain::redo_impl).await
}

/// No error case: an empty/absent log is just `[]`, not a 404 — there need
/// not be an open project for this route to answer (T11).
async fn log(State(state): State<SharedState>) -> Json<Vec<crate::session_log::LogEntry>> {
    Json(
        state
            .session_log
            .lock()
            .expect("state mutex poisoned")
            .entries()
            .to_vec(),
    )
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::sync::Arc;

    use axum::extract::State;
    use axum::Json;
    use knx_core::{GroupAddressStyle, IndividualAddress, Language, Project};
    use tokio::sync::oneshot;

    use super::{
        prepare_group_address_style_publication, publish_group_address_style,
        set_group_address_style, undo, SetGroupAddressStyleBody,
    };
    use crate::bus::fake::{FakeConnector, FakeTunnel};
    use crate::bus::{BusSession, GroupAddressContext};
    use crate::AppState;

    #[tokio::test]
    async fn concurrent_group_address_style_publications_keep_the_latest_accepted_context() {
        let state = Arc::new(AppState::default());
        let project = Project::new(Language("en".into()));
        let initial_ctx = GroupAddressContext::from_project(Some(&project));
        *state.project.lock().expect("project mutex poisoned") = Some(project);

        let assigned_address = IndividualAddress::new(1, 1, 5).expect("valid test address");
        let (tunnel, handle) = FakeTunnel::new(assigned_address, 4);
        let connector = FakeConnector::succeeding(tunnel);
        let session = BusSession::start(
            1,
            SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0),
            &connector,
            initial_ctx,
        )
        .await
        .expect("fake connector always succeeds");
        *state.bus_session.lock().await = Some(session);

        let transaction_a = state.group_address_style_publication.lock().await;
        // A pauses after its successful project mutation and snapshot while
        // retaining ownership of the complete route transaction.
        let publication_a =
            prepare_group_address_style_publication(&state, GroupAddressStyle::Free).unwrap();

        let (started_tx, started_rx) = oneshot::channel();
        let state_b = Arc::clone(&state);
        let request_b = tokio::spawn(async move {
            let _ = started_tx.send(());
            set_group_address_style(
                State(state_b),
                Json(SetGroupAddressStyleBody {
                    group_address_style: "TwoLevel".to_string(),
                }),
            )
            .await
        });
        started_rx.await.expect("B reaches the route transaction");

        assert_eq!(
            state
                .project
                .lock()
                .expect("project mutex poisoned")
                .as_ref()
                .expect("test project remains open")
                .info
                .group_address_style,
            GroupAddressStyle::Free,
            "B must not mutate the project while A owns the route transaction"
        );

        let _ = publish_group_address_style(&state, publication_a).await;
        drop(transaction_a);
        let _ = request_b
            .await
            .expect("B task completes")
            .expect("B restyle succeeds");

        assert_eq!(
            state
                .project
                .lock()
                .expect("project mutex poisoned")
                .as_ref()
                .expect("test project remains open")
                .info
                .group_address_style,
            GroupAddressStyle::TwoLevel
        );
        assert_eq!(
            state
                .bus_session
                .lock()
                .await
                .as_ref()
                .expect("test session remains active")
                .group_address_style(),
            Some(GroupAddressStyle::TwoLevel)
        );
        assert_eq!(connector.call_count(), 1);
        assert!(handle.sent_calls().is_empty());
        assert!(!handle.disconnected());
    }

    #[tokio::test]
    async fn concurrent_style_publication_and_undo_keep_the_latest_history_context() {
        let state = Arc::new(AppState::default());
        let project = Project::new(Language("en".into()));
        let initial_ctx = GroupAddressContext::from_project(Some(&project));
        *state.project.lock().expect("project mutex poisoned") = Some(project);

        let assigned_address = IndividualAddress::new(1, 1, 5).expect("valid test address");
        let (tunnel, handle) = FakeTunnel::new(assigned_address, 4);
        let connector = FakeConnector::succeeding(tunnel);
        let session = BusSession::start(
            1,
            SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0),
            &connector,
            initial_ctx,
        )
        .await
        .expect("fake connector always succeeds");
        *state.bus_session.lock().await = Some(session);

        let transaction = state.group_address_style_publication.lock().await;
        let publication =
            prepare_group_address_style_publication(&state, GroupAddressStyle::Free).unwrap();

        let (started_tx, started_rx) = oneshot::channel();
        let undo_state = Arc::clone(&state);
        let undo_request = tokio::spawn(async move {
            let _ = started_tx.send(());
            undo(State(undo_state)).await
        });
        started_rx
            .await
            .expect("undo reaches the route transaction");

        assert_eq!(
            state
                .project
                .lock()
                .expect("project mutex poisoned")
                .as_ref()
                .expect("test project remains open")
                .info
                .group_address_style,
            GroupAddressStyle::Free,
            "undo must not overtake an accepted style publication"
        );

        let _ = publish_group_address_style(&state, publication).await;
        drop(transaction);
        let _ = undo_request
            .await
            .expect("undo task completes")
            .expect("undo succeeds");

        assert_eq!(
            state
                .bus_session
                .lock()
                .await
                .as_ref()
                .expect("test session remains active")
                .group_address_style(),
            Some(GroupAddressStyle::ThreeLevel)
        );
        assert_eq!(connector.call_count(), 1);
        assert!(handle.sent_calls().is_empty());
        assert!(!handle.disconnected());
    }
}
