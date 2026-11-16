//! Mounted file APIs refuse private catalogue data and keys, including nested and symlink aliases.

use axum::http::StatusCode;
use serde_json::json;

use super::super::{CONFIG_DIR_NAME, DATA_DIR_NAME};
use super::support::*;

#[tokio::test]
async fn file_routes_cannot_list_open_or_replace_private_catalogue_state() {
    let f = Fixture::new(vec![one_index()]).await;
    let state = f.state(false);
    let router = crate::app(state.clone(), None);
    let public = state.data_dir.join("public");
    std::fs::create_dir_all(&public).unwrap();
    std::fs::write(public.join("visible.txt"), b"ordinary public fixture").unwrap();
    let (code, listing) = request(&router, "GET", "/api/fs/list?path=public", None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(listing, json!([{"name":"visible.txt", "is_dir":false}]));
    for root in [CONFIG_DIR_NAME, DATA_DIR_NAME] {
        for (index, parent) in [&state.data_dir, &public, f.temp.path()]
            .into_iter()
            .enumerate()
        {
            let hidden = parent.join(root);
            std::fs::create_dir_all(&hidden).unwrap();
            let sentinel = hidden.join("private.knxdb");
            std::fs::write(&sentinel, b"fictional private state must remain intact").unwrap();
            let alias = public.join(format!("alias-{root}-{index}"));
            std::os::unix::fs::symlink(&hidden, &alias).unwrap();
            let relative_alias = alias
                .strip_prefix(&state.data_dir)
                .unwrap()
                .to_string_lossy();
            let query = format!("path={relative_alias}");
            let (code, error) =
                request(&router, "GET", &format!("/api/fs/list?{query}"), None).await;
            assert_eq!(code, StatusCode::BAD_REQUEST, "{error}");
            assert_eq!(
                error["error"],
                if hidden.starts_with(&state.data_dir) {
                    "path is reserved"
                } else {
                    "path escapes the data directory"
                }
            );
            for target in [sentinel.clone(), alias.join("private.knxdb")] {
                for endpoint in ["open", "save-as"] {
                    let (code, error) = request(
                        &router,
                        "POST",
                        &format!("/api/project/{endpoint}"),
                        Some(json!({"path":target, "overwrite":true})),
                    )
                    .await;
                    assert_eq!(code, StatusCode::BAD_REQUEST, "{endpoint}: {error}");
                    assert_eq!(error["error"], "path is reserved");
                }
            }
            assert_eq!(
                std::fs::read(sentinel).unwrap(),
                b"fictional private state must remain intact"
            );
        }
    }
    let (code, listing) = request(&router, "GET", "/api/fs/list?path=public", None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(listing, json!([{"name":"visible.txt", "is_dir":false}]));
    assert!(state.project.lock().unwrap().is_none());
    assert!(f.server.requests().is_empty());
}
