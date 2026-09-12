use axum::extract::DefaultBodyLimit;
use axum::extract::Multipart;
use axum::extract::Path as AxumPath;
use axum::extract::Query;
use axum::extract::State;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::domain;
use crate::errors::ApiError;
use crate::paths::{resolve_new_project_path, resolve_project_path};
use crate::SharedState;

pub fn project_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/project/import", post(import_project))
        .route("/api/project/open", post(open_native_project))
        .route("/api/project/save", post(save_project))
        .route("/api/project/save-as", post(save_project_as))
        .route("/api/project/export", post(export_project))
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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogInstallReportDto {
    sha256: String,
    scheme: u32,
    skipped: bool,
    members: Vec<CatalogInstallMemberDto>,
    unknown: usize,
    conflicts: usize,
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
                    role: member.role,
                    sha256: member.sha256,
                    size: member.size,
                })
                .collect(),
            unknown: report.unknown,
            conflicts: report.conflicts.len(),
        }
    }
}

/// T18 slice 3 task 3 (design D20-D26): the parameter panel's read model
/// plus the response of a successful write, both riding this one DTO
/// (D24's "same response, no second GET"). Plain `#[derive(Serialize)]`,
/// following `CatalogInstallReportDto`'s own precedent above — not
/// `ts-rs`, not `knx-projection`, per the coordinator's ruling on DTO
/// placement (design doc D20: "no new crate").
#[derive(serde::Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParameterPanelDto {
    pub(crate) program_id: Option<String>,
    pub(crate) sections: Vec<ParameterSectionDto>,
    pub(crate) stale: Vec<StaleParameterDto>,
    pub(crate) diagnostics: Vec<ParameterDiagnosticDto>,
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

#[derive(serde::Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParameterDiagnosticDto {
    pub(crate) scope: Option<ModuleScopeDto>,
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
}

async fn import_project(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let path = resolve_project_path(&state.data_dir, &body.path)?;
    domain::open_project(&state, &path)
        .map(Json)
        .map_err(ApiError::internal)
}

async fn open_native_project(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let path = resolve_project_path(&state.data_dir, &body.path)?;
    domain::open_native_project(&state, &path)
        .map(Json)
        .map_err(ApiError::internal)
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

/// `ExportWarning` does not derive `Serialize` (it lives in `knx-etsproj`,
/// which has no reason to know about JSON) — same conversion shape as
/// `CreationDiagnosticDto` below, converted explicitly at the HTTP
/// boundary rather than reaching into `knx-etsproj` to add a derive that
/// would only ever be used here.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
enum ExportWarningDto {
    Unsigned { detail: String },
    StaleSignature { source_path: String },
    ManufacturerDataFromProductDb { entries: usize },
    MissingManufacturerData { source_path: String, sha256: String },
}

impl From<knx_etsproj::export::ExportWarning> for ExportWarningDto {
    fn from(value: knx_etsproj::export::ExportWarning) -> Self {
        use knx_etsproj::export::ExportWarning as W;
        match value {
            W::Unsigned { detail } => Self::Unsigned { detail },
            W::StaleSignature { source_path } => Self::StaleSignature { source_path },
            W::ManufacturerDataFromProductDb { entries } => {
                Self::ManufacturerDataFromProductDb { entries }
            }
            W::MissingManufacturerData {
                source_path,
                sha256,
            } => Self::MissingManufacturerData {
                source_path,
                sha256,
            },
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportReportDto {
    warnings: Vec<ExportWarningDto>,
}

/// Unlike `save`/`save_project_as` (whose failures are filesystem/store
/// problems -> `internal`, see `errors.rs`'s own doc comment), export's one
/// realistic failure mode reachable from the UI is "no store path yet" —
/// a caller-fixable precondition ("save as .knxdb first"), not an
/// environment problem, so this maps to `bad_request` like the rest of the
/// project-editing routes.
async fn export_project(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<ExportReportDto>, ApiError> {
    let path = resolve_new_project_path(&state.data_dir, &body.path)?;
    domain::export_project(&state, &path)
        .map(|outcome| ExportReportDto {
            warnings: outcome.warnings.into_iter().map(Into::into).collect(),
        })
        .map(Json)
        .map_err(ApiError::bad_request)
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
/// of JSON — so it gets the same explicit at-the-boundary conversion
/// `ExportWarningDto` above uses. Doubles as both an import-side problem
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
/// write target, resolved exactly like `/api/project/export`'s.
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
/// `CsvProblemDto`/`ExportWarningDto` above use.
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

/// Writes the live project as one self-contained "project documentation"
/// HTML file to `body.path` (`crates/knx-report`) — never called an "ETS
/// report" anywhere, because no ETS-produced sample exists in this
/// repository to be compatible with (see `knx-report`'s own module docs).
/// `path` is a fresh write target, resolved exactly like
/// `/api/group-addresses/csv-export`'s.
async fn export_documentation(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<DocumentationExportReportDto>, ApiError> {
    let path = resolve_new_project_path(&state.data_dir, &body.path)?;
    domain::export_documentation_impl(&state, &path)
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
    domain::undo_impl(&state)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn redo(
    State(state): State<SharedState>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::redo_impl(&state)
        .map(Json)
        .map_err(ApiError::bad_request)
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
