//! HTTP/session-log witness for unverified opaque payload roles.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn payload_evidence_reaches_the_http_session_log_without_certifying_an_app() {
    let xml = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations><Installation InstallationId="0"><Topology/><GroupAddresses/></Installation></Installations></Project></KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic.knxproj");
    std::fs::write(
        &source,
        knx_testsupport::zip_with_entries(&[
            ("P-TEST.signature", b""),
            ("P-TEST/0.xml", xml),
            ("P-TEST/project.xml", xml),
            ("P-TEST/UserFiles/opaque.etsapp", b"not an archive"),
            ("M-TEST/Baggages/picture.dll", b"\x89PNG\r\n\x1a\nsynthetic"),
        ]),
    )
    .unwrap();
    let state = Arc::new(knx_server::AppState {
        product_db: None,
        ..Default::default()
    });
    let app = knx_server::app(state, None);
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
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/log")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let log: Value = serde_json::from_slice(&bytes).unwrap();
    let rows = log.as_array().unwrap();
    for (path, class, role) in [
        (
            "P-TEST/UserFiles/opaque.etsapp",
            "unknown",
            Some("ETS-app-named candidate (identity unverified)"),
        ),
        ("M-TEST/Baggages/picture.dll", "png", None),
    ] {
        let row = rows
            .iter()
            .find(|row| {
                row["message"].as_str().unwrap_or("").contains(path)
                    && row["message"]
                        .as_str()
                        .unwrap_or("")
                        .contains(&format!("payload ({class})"))
            })
            .unwrap();
        let detail = row["message"].as_str().unwrap();
        assert!(detail.contains("uninterpreted"));
        assert!(detail.contains("not rendered or executed"));
        assert!(detail.contains("not unpacked"));
        if let Some(role) = role {
            assert!(detail.contains(role));
        }
        assert_eq!(row["severity"], "warning");
    }
}
