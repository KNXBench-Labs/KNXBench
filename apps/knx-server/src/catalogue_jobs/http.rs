//! Strict bounded HTTP adapter. Mount ONLY inside the existing authenticated,
//! path-fenced router; this module deliberately does not invent auth or CORS.

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post, put};
use axum::Router;
use http_body::Body as _;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;

use super::dto::{self, FailureKind};
use super::persistence::bounded_json;
use super::runtime::CatalogueRuntime;
use super::{Capability, Handle, JobId, Sha256, U64};

pub const PREFIX: &str = "/api/catalog/update/jobs";

/// A failed safe startup leaves the lane explicitly unavailable, never falling
/// through to the synchronous importer or exposing a private startup error.
pub(crate) fn unavailable_router(kind: FailureKind) -> Router {
    let route = axum::routing::any(move || async move { respond(Err(kind)) });
    Router::new()
        .route(PREFIX, route.clone())
        .route("/api/catalog/update/jobs/{*rest}", route)
}

/// State-bound router, mergeable with the parent's AppState router. Existing
/// authentication MUST wrap this router before any bounded body is consumed.
pub fn router(runtime: Arc<CatalogueRuntime>) -> Router {
    Router::new()
        .route("/api/catalog/update/jobs/admission", get(admission))
        .route("/api/catalog/update/jobs", post(create))
        .route("/api/catalog/update/jobs/{id}", get(status).delete(dispose))
        .route("/api/catalog/update/jobs/{id}/upload", put(upload))
        .route("/api/catalog/update/jobs/{id}/details", get(details))
        .route("/api/catalog/update/jobs/{id}/apply", post(apply))
        .route("/api/catalog/update/jobs/{id}/cancel", post(cancel))
        .with_state(runtime)
}

pub async fn admission(State(runtime): State<Arc<CatalogueRuntime>>, request: Request) -> Response {
    respond(
        async {
            no_query(&request)?;
            for header in [CAPABILITY, INCARNATION, SERIAL] {
                if request.headers().contains_key(header) {
                    return Err(FailureKind::InvalidHandle);
                }
            }
            bounded_body(request.into_body(), 0).await?;
            json(StatusCode::OK, &runtime.admission()?)
        }
        .await,
    )
}

pub async fn create(State(runtime): State<Arc<CatalogueRuntime>>, request: Request) -> Response {
    respond(
        async {
            no_query(&request)?;
            let capability = capability(request.headers())?;
            if request.headers().contains_key(INCARNATION) || request.headers().contains_key(SERIAL)
            {
                return Err(FailureKind::InvalidHandle);
            }
            let body: dto::Create = json_body(request, 4096).await?;
            let (fresh, status) = runtime.create(&capability, body)?;
            json(
                if fresh {
                    StatusCode::CREATED
                } else {
                    StatusCode::OK
                },
                &status,
            )
        }
        .await,
    )
}

pub async fn upload(State(runtime): State<Arc<CatalogueRuntime>>, request: Request) -> Response {
    respond(
        async {
            no_query(&request)?;
            let (handle, capability) = authority(&request)?;
            media_type(request.headers(), "application/octet-stream")?;
            encoding(request.headers())?;
            for header in ["range", "content-range", "trailer"] {
                if request.headers().contains_key(header) {
                    return Err(FailureKind::InvalidRequest);
                }
            }
            let status = runtime.status(handle, &capability)?;
            if let Some(length) = content_length(request.headers())? {
                if length != status.source.declared_bytes.get() {
                    return Err(FailureKind::UploadLengthMismatch);
                }
            }
            let status = runtime
                .upload(handle, &capability, request.into_body())
                .await?;
            json(StatusCode::ACCEPTED, &status)
        }
        .await,
    )
}

pub async fn status(State(runtime): State<Arc<CatalogueRuntime>>, request: Request) -> Response {
    respond(
        async {
            no_query(&request)?;
            let (handle, capability) = authority(&request)?;
            bounded_body(request.into_body(), 0).await?;
            let _permit = runtime
                .status_permit
                .try_acquire()
                .map_err(|_| FailureKind::JobBusy)?;
            json(StatusCode::OK, &runtime.status(handle, &capability)?)
        }
        .await,
    )
}

pub async fn details(State(runtime): State<Arc<CatalogueRuntime>>, request: Request) -> Response {
    respond(
        async {
            let (handle, capability) = authority(&request)?;
            let query = detail_query(request.uri().query().ok_or(FailureKind::InvalidRequest)?)?;
            bounded_body(request.into_body(), 0).await?;
            let bytes = runtime
                .details(
                    handle,
                    &capability,
                    query.artifact,
                    query.digest,
                    query.offset,
                    query.limit,
                )
                .await?;
            Ok(response(StatusCode::OK, bytes))
        }
        .await,
    )
}

pub async fn apply(State(runtime): State<Arc<CatalogueRuntime>>, request: Request) -> Response {
    respond(
        async {
            no_query(&request)?;
            let (handle, capability) = authority(&request)?;
            let command: dto::ApplyCommand = json_body(request, 1024).await?;
            let (fresh, status) = runtime.apply(handle, &capability, &command)?;
            json(
                if fresh {
                    StatusCode::ACCEPTED
                } else {
                    StatusCode::OK
                },
                &status,
            )
        }
        .await,
    )
}

pub async fn cancel(State(runtime): State<Arc<CatalogueRuntime>>, request: Request) -> Response {
    respond(
        async {
            no_query(&request)?;
            let (handle, capability) = authority(&request)?;
            let command: dto::CancelCommand = json_body(request, 1024).await?;
            json(
                StatusCode::OK,
                &runtime.cancel(handle, &capability, &command)?,
            )
        }
        .await,
    )
}

pub async fn dispose(State(runtime): State<Arc<CatalogueRuntime>>, request: Request) -> Response {
    respond(
        async {
            no_query(&request)?;
            let (handle, capability) = authority(&request)?;
            bounded_body(request.into_body(), 0).await?;
            json(StatusCode::OK, &runtime.dispose(handle, &capability).await?)
        }
        .await,
    )
}

const CAPABILITY: &str = "x-knx-catalogue-capability";
const INCARNATION: &str = "x-knx-catalogue-incarnation";
const SERIAL: &str = "x-knx-catalogue-serial";

fn header<'a>(
    headers: &'a HeaderMap,
    name: &str,
    max: usize,
) -> Result<Option<&'a str>, FailureKind> {
    let mut values = headers.get_all(name).iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() || value.as_bytes().len() > max {
        return Err(FailureKind::InvalidHandle);
    }
    value
        .to_str()
        .map(Some)
        .map_err(|_| FailureKind::InvalidHandle)
}
fn capability(headers: &HeaderMap) -> Result<Capability, FailureKind> {
    header(headers, CAPABILITY, 64)?
        .ok_or(FailureKind::InvalidHandle)?
        .parse()
        .map_err(|_| FailureKind::InvalidHandle)
}
fn authority(request: &Request) -> Result<(Handle, Capability), FailureKind> {
    let capability = capability(request.headers())?;
    let incarnation = header(request.headers(), INCARNATION, 32)?
        .ok_or(FailureKind::InvalidHandle)?
        .parse()
        .map_err(|_| FailureKind::InvalidHandle)?;
    let serial: U64 = header(request.headers(), SERIAL, 20)?
        .ok_or(FailureKind::InvalidHandle)?
        .parse()
        .map_err(|_| FailureKind::InvalidHandle)?;
    if serial == U64(0) || serial == U64::MAX {
        return Err(FailureKind::InvalidHandle);
    }
    let path = request
        .uri()
        .path()
        .strip_prefix(PREFIX)
        .and_then(|s| s.strip_prefix('/'))
        .ok_or(FailureKind::InvalidHandle)?;
    let job: JobId = path
        .split('/')
        .next()
        .ok_or(FailureKind::InvalidHandle)?
        .parse()
        .map_err(|_| FailureKind::InvalidHandle)?;
    Ok((
        Handle {
            incarnation,
            serial,
            job,
        },
        capability,
    ))
}
fn no_query(request: &Request) -> Result<(), FailureKind> {
    if request.uri().query().is_some() {
        Err(FailureKind::InvalidRequest)
    } else {
        Ok(())
    }
}
fn media_type(headers: &HeaderMap, expected: &str) -> Result<(), FailureKind> {
    let content_type = header(headers, "content-type", 128)
        .map_err(|_| FailureKind::UnsupportedMediaType)?
        .ok_or(FailureKind::UnsupportedMediaType)?;
    if content_type == expected
        || expected == "application/json"
            && matches!(
                content_type,
                "application/json; charset=utf-8" | "application/json;charset=utf-8"
            )
    {
        Ok(())
    } else {
        Err(FailureKind::UnsupportedMediaType)
    }
}
fn encoding(headers: &HeaderMap) -> Result<(), FailureKind> {
    match header(headers, "content-encoding", 32).map_err(|_| FailureKind::UnsupportedMediaType)? {
        None | Some("identity") => Ok(()),
        _ => Err(FailureKind::UnsupportedMediaType),
    }
}
fn content_length(headers: &HeaderMap) -> Result<Option<u64>, FailureKind> {
    header(headers, "content-length", 20)
        .map_err(|_| FailureKind::InvalidRequest)?
        .map(|s| {
            s.parse::<U64>()
                .map(U64::get)
                .map_err(|_| FailureKind::InvalidRequest)
        })
        .transpose()
}
async fn json_body<T: DeserializeOwned>(request: Request, cap: usize) -> Result<T, FailureKind> {
    media_type(request.headers(), "application/json")?;
    encoding(request.headers())?;
    if content_length(request.headers())?.is_some_and(|n| n > cap as u64) {
        return Err(FailureKind::BodyLimit);
    }
    let bytes = bounded_body(request.into_body(), cap).await?;
    let text = std::str::from_utf8(&bytes).map_err(|_| FailureKind::InvalidRequest)?;
    dto::from_str(text).map_err(|_| FailureKind::InvalidRequest)
}
async fn bounded_body(mut body: Body, cap: usize) -> Result<Vec<u8>, FailureKind> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(cap)
        .map_err(|_| FailureKind::BodyLimit)?;
    while let Some(frame) =
        std::future::poll_fn(|cx| std::pin::Pin::new(&mut body).poll_frame(cx)).await
    {
        let frame = frame.map_err(|_| FailureKind::InvalidRequest)?;
        if frame.is_trailers() {
            return Err(FailureKind::InvalidRequest);
        }
        if let Ok(data) = frame.into_data() {
            if bytes.len().checked_add(data.len()).is_none_or(|n| n > cap) {
                return Err(FailureKind::BodyLimit);
            }
            bytes.extend_from_slice(&data);
        }
    }
    Ok(bytes)
}
struct DetailQuery {
    artifact: JobId,
    digest: Sha256,
    offset: U64,
    limit: usize,
}
fn detail_query(query: &str) -> Result<DetailQuery, FailureKind> {
    if query.len() > 512 {
        return Err(FailureKind::InvalidRequest);
    }
    let mut artifact = None;
    let mut digest = None;
    let mut offset = None;
    let mut limit = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').ok_or(FailureKind::InvalidRequest)?;
        // All admitted values are canonical ASCII; percent/plus decoding would
        // create aliases and conceal duplicate key spellings, so refuse it.
        match key {
            "artifactId" if artifact.is_none() => {
                artifact = Some(value.parse().map_err(|_| FailureKind::InvalidRequest)?)
            }
            "publicSha256" if digest.is_none() => {
                digest = Some(value.parse().map_err(|_| FailureKind::InvalidRequest)?)
            }
            "offset" if offset.is_none() => {
                offset = Some(value.parse().map_err(|_| FailureKind::InvalidRequest)?)
            }
            "limit" if limit.is_none() => {
                let n: U64 = value.parse().map_err(|_| FailureKind::InvalidRequest)?;
                if !(1..=200).contains(&n.get()) {
                    return Err(FailureKind::InvalidRequest);
                }
                limit = Some(n.get() as usize);
            }
            _ => return Err(FailureKind::InvalidRequest),
        }
    }
    Ok(DetailQuery {
        artifact: artifact.ok_or(FailureKind::InvalidRequest)?,
        digest: digest.ok_or(FailureKind::InvalidRequest)?,
        offset: offset.ok_or(FailureKind::InvalidRequest)?,
        limit: limit.ok_or(FailureKind::InvalidRequest)?,
    })
}
fn json<T: Serialize>(status: StatusCode, value: &T) -> Result<Response, FailureKind> {
    bounded_json(value, 16_384).map(|bytes| response(status, bytes))
}
fn response(status: StatusCode, bytes: Vec<u8>) -> Response {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("cache-control", "no-store")
        .body(Body::from(bytes))
        .expect("fixed response headers")
}
fn respond(result: Result<Response, FailureKind>) -> Response {
    match result {
        Ok(response) => response,
        Err(kind) => {
            let code = StatusCode::from_u16(kind.http_status())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let bytes = serde_json::to_vec(&dto::Error {
                schema_version: 1,
                kind,
                context: None,
            })
            .expect("fixed error object");
            response(code, bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn metadata_body_refuses_the_first_over_budget_frame() {
        assert_eq!(
            bounded_body(Body::from(vec![b' '; 4097]), 4096).await,
            Err(FailureKind::BodyLimit)
        );
    }
    #[test]
    fn detail_query_and_capability_have_no_duplicate_or_encoding_aliases() {
        let query = format!(
            "artifactId={}&publicSha256={}&offset=0&limit=200",
            "01".repeat(16),
            "02".repeat(32)
        );
        assert!(detail_query(&query).is_ok());
        for suffix in ["&offset=0", "&limit=1", "&extra=x"] {
            assert!(detail_query(&(query.clone() + suffix)).is_err());
        }
        assert!(detail_query(&query.replace("offset=0", "offset=00")).is_err());
        let mut headers = HeaderMap::new();
        headers.append(CAPABILITY, "01".repeat(32).parse().unwrap());
        assert!(capability(&headers).is_ok());
        headers.append(CAPABILITY, "01".repeat(32).parse().unwrap());
        assert!(matches!(
            capability(&headers),
            Err(FailureKind::InvalidHandle)
        ));
    }
}
