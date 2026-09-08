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
        .route("/api/device/{id}", axum::routing::get(device_detail))
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
        .route("/api/areas", post(create_area))
        .route("/api/areas/{id}", delete(delete_area))
        .route("/api/lines", post(create_line))
        .route("/api/lines/{id}", delete(delete_line))
        .route("/api/move-device", post(move_device_to_line))
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
        .route("/api/devices", post(create_device))
        .route("/api/devices/{id}", delete(delete_device))
        .route("/api/undo", post(undo))
        .route("/api/redo", post(redo))
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

async fn device_detail(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::DeviceDetail>, ApiError> {
    domain::device_detail(&state, id)
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
    domain::catalog_items_impl(&state, q.manufacturer, q.search)
        .map(|rows| Json(rows.into_iter().map(CatalogItemDto::from).collect()))
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

async fn create_device(
    State(state): State<SharedState>,
    Json(body): Json<CreateDeviceBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_device_impl(&state, body.line_id, body.catalog_item_id, body.name)
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
