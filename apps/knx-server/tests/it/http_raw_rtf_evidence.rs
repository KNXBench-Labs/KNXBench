//! Synthetic import-to-HTTP evidence for literal, uninterpreted raw previews.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn raw_rtf_preview_reaches_http_log_without_plain_text_extraction_or_project_mutation() {
    let raw = r"{\rtf1\ansi <b>literal</b> \u20320?}";
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations><Installation InstallationId="0" SyntheticRawHint="{}"><Topology/><GroupAddresses/></Installation></Installations></Project></KNX>"#,
        raw.replace('<', "&lt;").replace('>', "&gt;")
    );
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic.knxproj");
    std::fs::write(
        &source,
        knx_testsupport::zip_with_entries(&[
            ("P-TEST.signature", b""),
            ("P-TEST/0.xml", xml.as_bytes()),
            ("P-TEST/project.xml", xml.as_bytes()),
        ]),
    )
    .unwrap();
    let app = knx_server::app(
        Arc::new(knx_server::AppState {
            product_db: None,
            ..Default::default()
        }),
        None,
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(json!({"path":source}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let before = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/project")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let before = axum::body::to_bytes(before.into_body(), usize::MAX)
        .await
        .unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/log")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let log: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let row = log
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["source"] == "import:opaque:raw-text")
        .unwrap();
    assert_eq!(row["detail"], raw);
    let message = row["message"].as_str().unwrap();
    assert!(message.contains("RTF-like raw text candidate"));
    assert!(message.contains("not validated or interpreted"));
    assert!(message.contains("original retained"));
    let after = app
        .oneshot(
            Request::builder()
                .uri("/api/project")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let after = axum::body::to_bytes(after.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(before, after);
}
