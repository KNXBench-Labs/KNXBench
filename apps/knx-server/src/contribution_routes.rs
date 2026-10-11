//! Own-instance contribution analysis; no AppState mutation or outbound client.
use crate::{errors::ApiError, SharedState};
use axum::{
    extract::{multipart::Field, DefaultBodyLimit, Multipart},
    response::IntoResponse,
    routing::post,
    Extension, Json, Router,
};
use knx_app::contribution::{self, Analysis, AnalysisError};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const MAX_FORM_BYTES: usize = contribution::MAX_INPUT_BYTES + 16 * 1024;

pub fn contribution_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/contributions/analyze", post(analyze))
        .route("/api/contributions/export", post(export))
        .route("/api/contributions/preview", post(preview))
        .layer(DefaultBodyLimit::max(MAX_FORM_BYTES))
        .layer(Extension(Arc::new(Semaphore::new(1))))
        .layer(axum::middleware::from_fn(
            |req: axum::extract::Request, next: axum::middleware::Next| async move {
                // A declared oversize is rejectable before receiving/buffering bytes.
                // Missing or understated lengths still meet the streaming body/field guards.
                let oversized = req
                    .headers()
                    .get(axum::http::header::CONTENT_LENGTH)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .is_some_and(|length| length > MAX_FORM_BYTES as u64);
                let mut response = if oversized {
                    ApiError::with_status(
                        axum::http::StatusCode::PAYLOAD_TOO_LARGE,
                        "contribution form exceeds 1024 MiB input plus 16 KiB multipart allowance",
                    )
                    .into_response()
                } else {
                    next.run(req).await
                };
                response.headers_mut().insert(
                    axum::http::header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("no-store"),
                );
                response
            },
        ))
}
const MAX_OPTIONS_BYTES: usize = 8 * 1024;

fn checked_field_size(current: usize, additional: usize, limit: usize) -> Result<usize, ApiError> {
    current
        .checked_add(additional)
        .filter(|size| *size <= limit)
        .ok_or_else(|| {
            ApiError::with_status(
                axum::http::StatusCode::PAYLOAD_TOO_LARGE,
                format!("contribution field exceeds its {limit}-byte limit"),
            )
        })
}

/// Bound each field while receiving it; never buffer then clone an entire upload.
async fn read_field(mut field: Field<'_>, limit: usize) -> Result<Vec<u8>, ApiError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|e| ApiError::with_status(e.status(), "invalid or oversized contribution field"))?
    {
        checked_field_size(bytes.len(), chunk.len(), limit)?;
        bytes
            .try_reserve(chunk.len())
            .map_err(|_| ApiError::internal("contribution input allocation failed"))?;
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn source(
    mut form: Multipart,
) -> Result<
    (
        String,
        Vec<u8>,
        Option<knx_app::contribution_bundle::BundleOptions>,
    ),
    ApiError,
> {
    let mut result = None;
    let mut options = None;
    while let Some(field) = form
        .next_field()
        .await
        .map_err(|e| ApiError::with_status(e.status(), "invalid or oversized contribution form"))?
    {
        if field.name() == Some("options") {
            if options.is_some() {
                return Err(ApiError::bad_request("duplicate options field"));
            }
            let bytes = read_field(field, MAX_OPTIONS_BYTES).await?;
            options = Some(
                serde_json::from_slice(&bytes)
                    .map_err(|_| ApiError::bad_request("invalid contribution options"))?,
            );
            continue;
        }
        if field.name() != Some("file") || result.is_some() {
            return Err(ApiError::bad_request("exactly one file field is required"));
        }
        let filename = field.file_name().unwrap_or_default().to_owned();
        let bytes = read_field(field, contribution::MAX_INPUT_BYTES).await?;
        result = Some((filename, bytes));
    }
    result
        .map(|(filename, bytes)| (filename, bytes, options))
        .ok_or_else(|| ApiError::bad_request("file field is required"))
}
fn claim_worker(limit: Arc<Semaphore>) -> Result<OwnedSemaphorePermit, ApiError> {
    limit.try_acquire_owned().map_err(|_| {
        ApiError::with_status(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "another evidence analysis is running; retry after it finishes",
        )
    })
}

fn error(e: AnalysisError) -> ApiError {
    match e {
        AnalysisError::Input(message) => ApiError::bad_request(message),
        AnalysisError::Internal(message) => ApiError::internal(message),
    }
}
async fn analyze(
    Extension(limit): Extension<Arc<Semaphore>>,
    form: Multipart,
) -> Result<Json<Analysis>, ApiError> {
    let permit = claim_worker(limit)?;
    let (filename, bytes, options) = source(form).await?;
    if options.is_some() {
        return Err(ApiError::bad_request(
            "analysis does not accept export options",
        ));
    }
    let report = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        contribution::analyze(&bytes, &filename)
    })
    .await
    .map_err(|_| ApiError::internal("analysis worker failed"))?
    .map_err(error)?;
    Ok(Json(report))
}

async fn preview(
    Extension(limit): Extension<Arc<Semaphore>>,
    form: Multipart,
) -> Result<Json<knx_app::contribution_bundle::Preview>, ApiError> {
    let permit = claim_worker(limit)?;
    let (filename, bytes, options) = source(form).await?;
    let options =
        options.ok_or_else(|| ApiError::bad_request("explicit disclosure options required"))?;
    let preview = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        knx_app::contribution_bundle::preview_bundle(&bytes, &filename, &options)
    })
    .await
    .map_err(|_| ApiError::internal("evidence worker failed"))?
    .map_err(error)?;
    Ok(Json(preview))
}

async fn export(
    Extension(limit): Extension<Arc<Semaphore>>,
    form: Multipart,
) -> Result<axum::response::Response, ApiError> {
    use axum::response::IntoResponse;
    let permit = claim_worker(limit)?;
    let (filename, bytes, options) = source(form).await?;
    let options =
        options.ok_or_else(|| ApiError::bad_request("explicit disclosure options required"))?;
    let bundle = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        knx_app::contribution_bundle::build_bundle(&bytes, &filename, &options)
    })
    .await
    .map_err(|_| ApiError::internal("evidence worker failed"))?
    .map_err(error)?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/zip"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"knxbench-evidence.zip\"",
            ),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        bundle.bytes,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    #[test]
    fn field_size_limits_are_inclusive_and_overflow_safe_without_large_allocations() {
        let limit = knx_app::contribution::MAX_INPUT_BYTES;
        assert_eq!(limit, 1024 * 1024 * 1024);
        assert_eq!(
            super::checked_field_size(limit - 1, 1, limit).unwrap(),
            limit
        );
        assert!(super::checked_field_size(limit, 1, limit).is_err());
        assert!(super::checked_field_size(usize::MAX, 1, limit).is_err());
        assert_eq!(
            super::checked_field_size(0, super::MAX_OPTIONS_BYTES, super::MAX_OPTIONS_BYTES)
                .unwrap(),
            8192
        );
        assert!(super::checked_field_size(8192, 1, super::MAX_OPTIONS_BYTES).is_err());
    }
    #[tokio::test]
    async fn worker_limit_refuses_instead_of_queuing_more_analyses() {
        let limit = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
        let permit = super::claim_worker(limit.clone()).unwrap();
        assert!(super::claim_worker(limit.clone()).is_err());
        drop(permit);
        assert!(super::claim_worker(limit).is_ok());
    }
}
