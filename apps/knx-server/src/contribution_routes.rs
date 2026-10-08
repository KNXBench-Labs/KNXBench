//! Own-instance contribution analysis; no AppState mutation or outbound client.
use crate::{errors::ApiError, SharedState};
use axum::{
    extract::{DefaultBodyLimit, Multipart},
    routing::post,
    Extension, Json, Router,
};
use knx_app::contribution::{self, Analysis, AnalysisError};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub fn contribution_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/contributions/analyze", post(analyze))
        .route("/api/contributions/export", post(export))
        .route("/api/contributions/preview", post(preview))
        .layer(DefaultBodyLimit::max(
            contribution::MAX_INPUT_BYTES + 16 * 1024,
        ))
        .layer(Extension(Arc::new(Semaphore::new(1))))
        .layer(axum::middleware::from_fn(
            |req: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut response = next.run(req).await;
                response.headers_mut().insert(
                    axum::http::header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("no-store"),
                );
                response
            },
        ))
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
            let bytes = field
                .bytes()
                .await
                .map_err(|e| ApiError::with_status(e.status(), "invalid contribution options"))?;
            if bytes.len() > 8 * 1024 {
                return Err(ApiError::bad_request("contribution options exceed 8 KiB"));
            }
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
        let bytes = field.bytes().await.map_err(|e| {
            ApiError::with_status(e.status(), "invalid or oversized contribution file")
        })?;
        if bytes.len() > contribution::MAX_INPUT_BYTES {
            return Err(ApiError::with_status(
                axum::http::StatusCode::PAYLOAD_TOO_LARGE,
                "contribution file exceeds 32 MiB",
            ));
        }
        result = Some((filename, bytes.to_vec()));
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
    #[tokio::test]
    async fn worker_limit_refuses_instead_of_queuing_more_analyses() {
        let limit = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
        let permit = super::claim_worker(limit.clone()).unwrap();
        assert!(super::claim_worker(limit.clone()).is_err());
        drop(permit);
        assert!(super::claim_worker(limit).is_ok());
    }
}
