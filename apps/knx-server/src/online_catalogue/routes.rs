//! Online catalogue API: status, manual sync, local search, account and verified import feed.

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::body::verified_file_body;
use super::download::DownloadStatus;
use super::error::OnlineError;
use super::index::{Entry, DOWNLOADABLE_TYPES, MAX_PAGE};
use super::service::{SourceService, Status, SyncOutcome};
use super::source::SOURCES;
use super::vault::Credentials;
use crate::catalogue_jobs::http as jobs_http;
use crate::errors::ApiError;
use crate::SharedState;

const MAX_QUERY_CHARS: usize = 200;
const MAX_OFFSET: usize = 10_000_000;
const MAX_LEGACY_PASSWORD_BYTES: usize = 1024;

pub fn router() -> Router<SharedState> {
    const BASE: &str = "/api/online-catalogue";
    Router::new()
        .route(BASE, get(sources))
        .route(
            "/api/online-catalogue/trust-store-license",
            get(trust_store_license),
        )
        .route("/api/online-catalogue/{source}", get(status))
        .route("/api/online-catalogue/{source}/config", put(set_config))
        .route("/api/online-catalogue/{source}/sync", post(sync))
        .route("/api/online-catalogue/{source}/search", get(search))
        .route(
            "/api/online-catalogue/{source}/credentials",
            put(set_credentials).delete(forget_credentials),
        )
        .route(
            "/api/online-catalogue/{source}/entries/{entry}/download",
            post(start_download),
        )
        .route(
            "/api/online-catalogue/{source}/download",
            delete(cancel_download),
        )
        .route(
            "/api/online-catalogue/{source}/jobs/{job}/feed/{entry}",
            post(feed),
        )
        .route(
            "/api/online-catalogue/{source}/entries/{entry}/install-legacy",
            post(install_legacy),
        )
}

/// Keep the agreement available in standalone and embedded binaries as well as source archives.
async fn trust_store_license() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/third-party/webpki-roots-CDLA-Permissive-2.0.txt"
    ))
}

impl IntoResponse for OnlineError {
    fn into_response(self) -> Response {
        ApiError::from(self).into_response()
    }
}

impl From<OnlineError> for ApiError {
    fn from(error: OnlineError) -> Self {
        let mut details = serde_json::Map::new();
        match &error {
            OnlineError::DailyLimit {
                reset_at,
                retry_after_secs,
            } => {
                details.insert("resetAt".into(), serde_json::json!(reset_at));
                details.insert("retryAfterSecs".into(), serde_json::json!(retry_after_secs));
            }
            OnlineError::RateLimited { retry_after_secs } => {
                details.insert("retryAfterSecs".into(), serde_json::json!(retry_after_secs));
            }
            _ => {}
        }
        ApiError::with_details(error.http_status(), error.kind(), error.message(), details)
    }
}

fn service(state: &SharedState, source: &str) -> Result<Arc<SourceService>, OnlineError> {
    let catalogue = state
        .online_catalogue
        .as_ref()
        .ok_or(OnlineError::Unavailable)?;
    Ok(Arc::clone(catalogue.source(source)?))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceDto {
    id: &'static str,
    label: &'static str,
    default_base_url: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SourcesDto {
    available: bool,
    sources: Vec<SourceDto>,
}

async fn sources(State(state): State<SharedState>) -> Json<SourcesDto> {
    let available = state.online_catalogue.is_some();
    Json(SourcesDto {
        available,
        sources: if available {
            SOURCES
                .iter()
                .map(|s| SourceDto {
                    id: s.id,
                    label: s.label,
                    default_base_url: s.default_base_url,
                })
                .collect()
        } else {
            Vec::new()
        },
    })
}

async fn status(
    State(state): State<SharedState>,
    Path(source): Path<String>,
) -> Result<Json<Status>, OnlineError> {
    Ok(Json(service(&state, &source)?.status()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConfigBody {
    /// `null` restores the built-in default address.
    base_url: Option<String>,
}

async fn set_config(
    State(state): State<SharedState>,
    Path(source): Path<String>,
    Json(body): Json<ConfigBody>,
) -> Result<Json<Status>, OnlineError> {
    let service = service(&state, &source)?;
    service.set_base_url(body.base_url.as_deref()).await?;
    Ok(Json(service.status()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncDto {
    #[serde(flatten)]
    outcome: SyncOutcome,
    status: Status,
}

async fn sync(
    State(state): State<SharedState>,
    Path(source): Path<String>,
) -> Result<Json<SyncDto>, OnlineError> {
    let service = service(&state, &source)?;
    let outcome = service.sync().await?;
    Ok(Json(SyncDto {
        outcome,
        status: service.status(),
    }))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    #[serde(rename = "type")]
    file_type: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgramDto {
    display_key: String,
    name: String,
    application_number: String,
    version: String,
    mask_version: String,
    product_names: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EntryDto {
    id: String,
    manufacturer: String,
    manufacturer_id: String,
    name: String,
    order_numbers: Vec<String>,
    file_name: String,
    file_type: String,
    size: u64,
    sha256: String,
    updated: String,
    programs: Vec<ProgramDto>,
    /// Whether KNXBench has an import path for this file type.
    importable: bool,
    /// A verified copy is already stored; installing it costs no download allowance.
    downloaded: bool,
}

impl EntryDto {
    fn new(entry: Entry, downloaded: bool) -> Self {
        let importable = entry.importable();
        Self {
            id: entry.id,
            manufacturer: entry.manufacturer,
            manufacturer_id: entry.manufacturer_id,
            name: entry.name,
            order_numbers: entry.order_numbers,
            file_name: entry.file_name,
            file_type: entry.file_type,
            size: entry.size,
            sha256: entry.sha256,
            updated: entry.updated,
            programs: entry
                .programs
                .into_iter()
                .map(|p| ProgramDto {
                    display_key: p.display_key,
                    name: p.name,
                    application_number: p.application_number,
                    version: p.version,
                    mask_version: p.mask_version,
                    product_names: p.product_names,
                })
                .collect(),
            importable,
            downloaded,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchDto {
    total: usize,
    offset: usize,
    limit: usize,
    entries: Vec<EntryDto>,
}

async fn search(
    State(state): State<SharedState>,
    Path(source): Path<String>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchDto>, ApiError> {
    let service = service(&state, &source)?;
    let text = query.q.unwrap_or_default();
    if text.chars().count() > MAX_QUERY_CHARS {
        return Err(ApiError::bad_request("the search text is too long"));
    }
    if let Some(t) = &query.file_type {
        if !DOWNLOADABLE_TYPES.contains(&t.as_str()) {
            return Err(ApiError::bad_request("unknown file type"));
        }
    }
    let offset = query.offset.unwrap_or(0);
    if offset > MAX_OFFSET {
        return Err(ApiError::bad_request("the offset is too large"));
    }
    let limit = query.limit.unwrap_or(25).clamp(1, MAX_PAGE);
    let result = service
        .search(&text, query.file_type.as_deref(), offset, limit)
        .await?;
    Ok(Json(SearchDto {
        total: result.total,
        offset,
        limit,
        entries: result
            .entries
            .into_iter()
            .map(|(entry, downloaded)| EntryDto::new(entry, downloaded))
            .collect(),
    }))
}

async fn set_credentials(
    State(state): State<SharedState>,
    Path(source): Path<String>,
    Json(credentials): Json<Credentials>,
) -> Result<Json<Status>, OnlineError> {
    let service = service(&state, &source)?;
    service.set_credentials(credentials).await?;
    Ok(Json(service.status()))
}

async fn forget_credentials(
    State(state): State<SharedState>,
    Path(source): Path<String>,
) -> Result<Json<Status>, OnlineError> {
    let service = service(&state, &source)?;
    service.forget_credentials().await?;
    Ok(Json(service.status()))
}

#[derive(Deserialize)]
struct Selection {
    sha256: String,
}

async fn start_download(
    State(state): State<SharedState>,
    Path((source, entry)): Path<(String, String)>,
    Query(selection): Query<Selection>,
) -> Result<(StatusCode, Json<DownloadStatus>), OnlineError> {
    let service = service(&state, &source)?;
    let status = service.start_download(&entry, &selection.sha256).await?;
    let code = if status.state == "done" {
        StatusCode::OK
    } else {
        StatusCode::ACCEPTED
    };
    Ok((code, Json(status)))
}

async fn cancel_download(
    State(state): State<SharedState>,
    Path(source): Path<String>,
) -> Result<Json<DownloadStatus>, OnlineError> {
    let service = service(&state, &source)?;
    service.cancel_download();
    Ok(Json(service.download_status()))
}

/// Streams the verified local copy into an install job created for that very file, exactly as
/// the browser's upload would, so preview, consent and apply stay the one existing job path.
async fn feed(
    State(state): State<SharedState>,
    Path((source, job, entry)): Path<(String, String, String)>,
    Query(selection): Query<Selection>,
    headers: HeaderMap,
) -> Response {
    let prepared = async {
        let service = service(&state, &source)?;
        let runtime = state
            .catalogue_jobs
            .attached_runtime()
            .ok_or(OnlineError::Unavailable)?;
        let selected = service.entry(&entry).await?;
        if selected.sha256 != selection.sha256 {
            return Err(OnlineError::SelectionChanged);
        }
        // Only the modern package takes the job path; `.vd3`–`.vd5` use the legacy import.
        if selected.file_type != "knxprod" {
            return Err(OnlineError::NotDownloadable);
        }
        let (file, entry) = service.open_cached(&entry).await?;
        if entry.sha256 != selection.sha256 {
            return Err(OnlineError::SelectionChanged);
        }
        Ok::<_, OnlineError>((runtime, file, entry))
    }
    .await;
    let (runtime, file, entry) = match prepared {
        Ok(parts) => parts,
        Err(error) => return error.into_response(),
    };
    let body: Body = verified_file_body(file, entry.size, entry.sha256.clone());
    jobs_http::feed(
        &runtime,
        &headers,
        &job,
        jobs_http::Feed {
            name: &entry.file_name,
            bytes: entry.size,
        },
        body,
    )
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyBody {
    password: Option<String>,
    #[serde(default)]
    remember: bool,
}

/// Installs a cached `.vd3`–`.vd5` file through the existing legacy import (its own password
/// prompt; the catalogue account's password is never used for the file).
async fn install_legacy(
    State(state): State<SharedState>,
    Path((source, entry)): Path<(String, String)>,
    Query(selection): Query<Selection>,
    Json(body): Json<LegacyBody>,
) -> Result<Response, ApiError> {
    let service = service(&state, &source)?;
    if body
        .password
        .as_ref()
        .is_some_and(|p| p.len() > MAX_LEGACY_PASSWORD_BYTES)
    {
        return Err(ApiError::bad_request("the password is too long"));
    }
    let selected = service.entry(&entry).await?;
    if selected.sha256 != selection.sha256 {
        return Err(OnlineError::SelectionChanged.into());
    }
    if !matches!(selected.file_type.as_str(), "vd3" | "vd4" | "vd5") {
        return Err(ApiError::with_kind(
            StatusCode::BAD_REQUEST,
            "unsupportedFileType",
            "only .vd3, .vd4 and .vd5 files take the legacy import",
        ));
    }
    let (bytes, entry) = service.read_verified(&entry).await?;
    if entry.sha256 != selection.sha256 {
        return Err(OnlineError::SelectionChanged.into());
    }
    let password = body.password.map(knx_app::legacy::LegacyPassword::new);
    let report = crate::routes::run_legacy_install(
        state.clone(),
        entry.file_name,
        bytes.into(),
        password,
        body.remember,
    )
    .await?;
    Ok(report.into_response())
}
