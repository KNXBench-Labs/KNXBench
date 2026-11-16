//! Mounted catalogue API contracts, authentication, bounded search and secret redaction.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

use super::super::store::sha256_hex;
use super::super::test_server::Script;
use super::support::*;

#[tokio::test]
async fn bundled_root_data_agreement_is_available_without_provider_access() {
    let f = Fixture::new(vec![one_index()]).await;
    let router = crate::app(f.state(false), None);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/online-catalogue/trust-store-license")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("text/plain"));
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.starts_with("# Community Data License Agreement - Permissive - Version 2.0\n"));
    assert!(text.contains("## 2. Conditions for Sharing Data"));
    assert!(text.contains("makes available the text of this agreement"));
    assert!(text.contains("## 5. Definitions"));
    assert!(f.server.requests().is_empty());
}

#[tokio::test]
async fn every_catalogue_route_in_the_production_mount_is_authentication_guarded() {
    let f = Fixture::new(vec![one_index()]).await;
    let state = f.state(false);
    let router = crate::app_with_auth(
        state,
        None,
        crate::AuthConfig::from_plaintext_password("fictional-app-password").unwrap(),
    );
    for (method, uri, body) in [
        ("GET", "/api/online-catalogue".into(), None),
        (
            "GET",
            "/api/online-catalogue/trust-store-license".into(),
            None,
        ),
        ("GET", PREFIX.into(), None),
        ("GET", format!("{PREFIX}/search"), None),
        (
            "PUT",
            format!("{PREFIX}/config"),
            Some(json!({"baseUrl":"https://outside.example"})),
        ),
        ("POST", format!("{PREFIX}/sync"), None),
        (
            "PUT",
            format!("{PREFIX}/credentials"),
            Some(json!({"mode":"token", "token":TOKEN})),
        ),
        ("DELETE", format!("{PREFIX}/credentials"), None),
        (
            "POST",
            format!(
                "{PREFIX}/entries/sample/download?sha256={}",
                sha256_hex(CONTENT)
            ),
            None,
        ),
        ("DELETE", format!("{PREFIX}/download"), None),
        (
            "POST",
            format!(
                "{PREFIX}/jobs/fictional/feed/sample?sha256={}",
                sha256_hex(CONTENT)
            ),
            None,
        ),
        (
            "POST",
            format!(
                "{PREFIX}/entries/sample/install-legacy?sha256={}",
                sha256_hex(CONTENT)
            ),
            Some(json!({"password":null})),
        ),
    ] {
        let (code, value) = request(&router, method, &uri, body).await;
        assert_eq!(code, StatusCode::UNAUTHORIZED, "{method} {uri}: {value}");
        assert_redacted(&value);
    }
    assert!(f.server.requests().is_empty());
    assert!(f.source.status().index.is_none());
    assert_eq!(f.source.status().credentials.state, "empty");
}

#[tokio::test]
async fn opening_sources_status_settings_and_cold_search_is_offline() {
    let f = Fixture::new(vec![one_index()]).await;
    let router = crate::app(f.state(false), None);
    let (code, sources) = request(&router, "GET", "/api/online-catalogue", None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(sources["available"], true);
    assert_eq!(sources["sources"].as_array().unwrap().len(), 1);
    assert_eq!(sources["sources"][0]["id"], "reko");
    let (code, status) = request(&router, "GET", PREFIX, None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(status["index"].is_null());
    assert_eq!(status["credentials"]["state"], "empty");
    let (code, search) = request(&router, "GET", &format!("{PREFIX}/search?q=dimmer"), None).await;
    assert_eq!(code, StatusCode::CONFLICT);
    assert_eq!(search["kind"], "noIndex");
    let (code, settings) = request(&router, "GET", "/api/settings", None).await;
    assert_eq!(code, StatusCode::OK);
    for body in [&sources, &status, &search, &settings] {
        assert_redacted(body);
    }
    assert!(f.server.requests().is_empty());
}

#[tokio::test]
async fn password_and_token_mutations_return_status_only_and_never_enter_settings() {
    let f = Fixture::new(vec![one_index()]).await;
    let router = crate::app(f.state(false), None);
    for (credentials, mode, user) in [
        (
            json!({"mode":"password", "user":"fictional-user", "password":PASSWORD}),
            "password",
            json!("fictional-user"),
        ),
        (json!({"mode":"token", "token":TOKEN}), "token", json!(null)),
    ] {
        let (code, changed) = request(
            &router,
            "PUT",
            &format!("{PREFIX}/credentials"),
            Some(credentials),
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{changed}");
        assert_eq!(
            changed["credentials"],
            json!({"state":"stored", "mode":mode, "user":user, "problem":null})
        );
        let (_, status) = request(&router, "GET", PREFIX, None).await;
        let (_, settings) = request(&router, "GET", "/api/settings", None).await;
        for value in [&changed, &status, &settings] {
            assert_redacted(value);
        }
        assert_eq!(status["credentials"], changed["credentials"]);
    }
    let (code, forgotten) =
        request(&router, "DELETE", &format!("{PREFIX}/credentials"), None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(
        forgotten["credentials"],
        json!({"state":"empty", "mode":null, "user":null, "problem":null})
    );
    assert!(!f.source_dir().join("credentials.enc").exists());
    assert!(f.server.requests().is_empty());
}

#[tokio::test]
async fn search_paginates_locally_and_preserves_program_versions_and_import_support() {
    let f = Fixture::new(vec![index_reply(vec![
        file_entry("modern", "knxprod", CONTENT),
        file_entry("legacy1", "vd1", CONTENT),
        file_entry("legacy2", "vd2", CONTENT),
        file_entry("legacy5", "vd5", CONTENT),
    ])])
    .await;
    let router = crate::app(f.state(false), None);
    let (code, sync) = request(&router, "POST", &format!("{PREFIX}/sync"), None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(sync["outcome"], "updated");
    assert_eq!(sync["entryCount"], 4);
    assert!(sync.get("entry_count").is_none());
    let (code, page) = request(
        &router,
        "GET",
        &format!("{PREFIX}/search?q=CONTOSO%200001&offset=1&limit=2"),
        None,
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(
        (
            page["total"].as_u64(),
            page["offset"].as_u64(),
            page["limit"].as_u64()
        ),
        (Some(4), Some(1), Some(2))
    );
    assert_eq!(page["entries"].as_array().unwrap().len(), 2);
    assert_eq!(page["entries"][0]["id"], "legacy1");
    assert_eq!(page["entries"][1]["id"], "legacy2");
    for entry in page["entries"].as_array().unwrap() {
        assert_eq!(entry["importable"], false);
        assert_eq!(entry["downloaded"], false);
        assert!(entry.get("download_url").is_none());
        assert!(entry.get("downloadUrl").is_none());
        assert_eq!(
            entry["programs"][0],
            json!({
                "displayKey":"0042 0001 01 [0000]", "name":"Dimmer 4-fold", "applicationNumber":"0001",
                "version":"01", "maskVersion":"0000", "productNames":["Dimmer actuator", "Dim 4x"]
            })
        );
    }
    let (_, filtered) = request(
        &router,
        "GET",
        &format!("{PREFIX}/search?type=vd5&limit=999"),
        None,
    )
    .await;
    assert_eq!(filtered["total"], 1);
    assert_eq!(filtered["limit"], 100);
    assert_eq!(filtered["entries"][0]["id"], "legacy5");
    assert_eq!(filtered["entries"][0]["importable"], true);
    let (_, empty) = request(
        &router,
        "GET",
        &format!("{PREFIX}/search?offset=100&limit=0"),
        None,
    )
    .await;
    assert_eq!(empty["total"], 4);
    assert_eq!(empty["limit"], 1);
    assert!(empty["entries"].as_array().unwrap().is_empty());
    for suffix in [
        "type=zip".to_string(),
        "offset=10000001".into(),
        format!("q={}", "x".repeat(201)),
    ] {
        assert_eq!(
            request(&router, "GET", &format!("{PREFIX}/search?{suffix}"), None)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(f.server.requests().len(), 1);
}

#[tokio::test]
async fn unknown_sources_invalid_config_and_stale_selection_are_typed_no_effect_refusals() {
    let f = Fixture::new(vec![one_index()]).await;
    let state = f.state(true);
    let router = crate::app(state.clone(), None);
    let db = state.data_dir.join("catalogue.sqlite");
    let before = super::import::logical_database(&db);
    let (code, unknown) = request(&router, "GET", "/api/online-catalogue/unknown", None).await;
    assert_eq!(code, StatusCode::NOT_FOUND);
    assert_eq!(unknown["kind"], "unknownSource");
    let (code, invalid) = request(
        &router,
        "PUT",
        &format!("{PREFIX}/config"),
        Some(json!({"baseUrl":"http://outside.example"})),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert_eq!(invalid["kind"], "invalidBaseUrl");
    assert_eq!(f.source.status().base_url, f.server.base_text());
    f.source.sync().await.unwrap();
    f.token().await;
    let stale = sha256_hex(b"previous fictional catalogue revision");
    for (suffix, body) in [
        ("entries/sample/download", None),
        ("jobs/fictional/feed/sample", None),
        (
            "entries/sample/install-legacy",
            Some(json!({"password":null})),
        ),
    ] {
        let (code, error) = request(
            &router,
            "POST",
            &format!("{PREFIX}/{suffix}?sha256={stale}"),
            body,
        )
        .await;
        assert_eq!(code, StatusCode::CONFLICT, "{suffix}: {error}");
        assert_eq!(error["kind"], "selectionChanged");
        assert_redacted(&error);
    }
    assert_eq!(f.server.requests().len(), 1);
    assert_eq!(super::import::logical_database(&db), before);
    assert!(!f.cache_file("sample").exists());
    assert!(state.project.lock().unwrap().is_none());
}

#[tokio::test]
async fn download_failure_status_exposes_only_stable_error_and_limit_metadata() {
    let reset = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
    let bytes = serde_json::to_vec(
        &json!({"code":"DOWNLOAD_DAILY_LIMIT", "resetAt":reset, "message":PASSWORD, "token":TOKEN}),
    )
    .unwrap();
    let f = Fixture::new(vec![
        one_index(),
        Script::reply(429, &[("retry-after", "300")], &bytes),
    ])
    .await;
    let router = crate::app(f.state(false), None);
    f.source.sync().await.unwrap();
    f.token().await;
    let uri = format!(
        "{PREFIX}/entries/sample/download?sha256={}",
        sha256_hex(CONTENT)
    );
    let (code, started) = request(&router, "POST", &uri, None).await;
    assert_eq!(code, StatusCode::ACCEPTED);
    assert_redacted(&started);
    assert_eq!(settled(&f.source).await.state, "failed");
    let (code, status) = request(&router, "GET", PREFIX, None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(status["download"]["error"]["kind"], "dailyLimit");
    let canonical = chrono::DateTime::parse_from_rfc3339(reset.as_str())
        .unwrap()
        .with_timezone(&chrono::Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    assert_eq!(status["download"]["error"]["resetAt"], canonical);
    assert_redacted(&status);
    let (code, refusal) = request(&router, "POST", &uri, None).await;
    assert_eq!(code, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(refusal["kind"], "dailyLimit");
    assert_eq!(refusal["resetAt"], canonical);
    assert!(refusal["retryAfterSecs"].as_u64().unwrap() > 0);
    assert_redacted(&refusal);
    assert_eq!(f.server.requests().len(), 2);
}

#[tokio::test]
async fn mounted_status_and_search_preserve_fictional_program_versions_for_browser_consumers() {
    let mut modern = file_entry("capture-modern", "knxprod", CONTENT);
    let mut version_two = modern["programs"][0].clone();
    version_two["version"] = json!("02");
    version_two["displayKey"] = json!("0042 0001 02 [0000]");
    modern["programs"].as_array_mut().unwrap().push(version_two);
    let f = Fixture::new(vec![index_reply(vec![
        modern,
        file_entry("capture-vd1", "vd1", CONTENT),
        file_entry("capture-vd2", "vd2", CONTENT),
        file_entry("capture-vd5", "vd5", CONTENT),
    ])])
    .await;
    let router = crate::app(f.state(false), None);
    let (code, cold, raw_cold) =
        request_raw(&router, "GET", &format!("{PREFIX}/search"), None).await;
    assert_eq!(code, StatusCode::CONFLICT);
    assert_eq!(cold["kind"], "noIndex");
    assert!(f.server.requests().is_empty());
    let (code, synced) = request(&router, "POST", &format!("{PREFIX}/sync"), None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(synced["outcome"], "updated");
    let (code, status, raw_status) = request_raw(&router, "GET", PREFIX, None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(status["index"]["entryCount"], 4);
    assert_eq!(status["index"]["skippedEntries"], 0);
    assert!(status["index"]["lastCheckedAt"].as_str().is_some());
    let (code, search, raw_search) =
        request_raw(&router, "GET", &format!("{PREFIX}/search?limit=100"), None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(search["total"], 4);
    assert_eq!(search["entries"][0]["id"], "capture-modern");
    assert_eq!(search["entries"][0]["programs"][0]["version"], "01");
    assert_eq!(search["entries"][0]["programs"][1]["version"], "02");
    assert_eq!(
        search["entries"][0]["programs"][1]["displayKey"],
        "0042 0001 02 [0000]"
    );
    assert_eq!(search["entries"][1]["importable"], false);
    assert_eq!(search["entries"][2]["importable"], false);
    assert_eq!(search["entries"][3]["importable"], true);
    let stale = "00".repeat(32);
    let (code, error, raw_error) = request_raw(
        &router,
        "POST",
        &format!("{PREFIX}/entries/capture-modern/download?sha256={stale}"),
        None,
    )
    .await;
    assert_eq!(code, StatusCode::CONFLICT);
    assert_eq!(error["kind"], "selectionChanged");
    for value in [&cold, &status, &search, &error] {
        assert_redacted(value);
    }
    assert_eq!(f.server.requests().len(), 1);
    capture_raw("status-synced.json", &raw_status);
    capture_raw("search-program-versions.json", &raw_search);
    capture_raw("error-no-index.json", &raw_cold);
    capture_raw("error-selection-changed.json", &raw_error);
}
