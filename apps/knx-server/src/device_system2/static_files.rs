//! Static fallback guards private recovery evidence against overlapping roots.
//!
//! root or symlink. Local-owner filesystem replacement races are not a sandbox.
use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::path::{Component, Path};
fn decode_path(path: &str) -> Option<String> {
    if path.len() > 4096 {
        return None;
    }
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let a = (*bytes.get(i + 1)? as char).to_digit(16)?;
            let b = (*bytes.get(i + 2)? as char).to_digit(16)?;
            out.push((a * 16 + b) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    let out = String::from_utf8(out).ok()?;
    if out.contains(['\\', '\0']) {
        return None;
    }
    Some(out)
}
pub(crate) async fn guard(
    data: &Path,
    static_dir: &Path,
    request: Request,
    next: Next,
) -> Response {
    let Some(decoded) = decode_path(request.uri().path()) else {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    };
    let relative = Path::new(decoded.trim_start_matches('/'));
    if relative.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }
    // Literal names are refused even before a private directory exists.
    if relative
        .components()
        .any(|c| c.as_os_str() == super::ROOT_NAME)
    {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }
    if let Ok(root) = tokio::fs::canonicalize(data).await {
        let candidate = static_dir.join(relative);
        // ServeDir appends index.html for directories. Inspect that implicit
        // target too, otherwise a root/index symlink can bypass the URI check.
        for target in [candidate.clone(), candidate.join("index.html")] {
            if let Ok(file) = tokio::fs::canonicalize(target).await {
                if crate::paths::refuse_reserved(&root, &file).is_err() {
                    return axum::http::StatusCode::NOT_FOUND.into_response();
                }
            }
        }
    }
    next.run(request).await
}
