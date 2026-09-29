//! Actionable 422 responses must identify the parser, syntax, and a usable example.
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{GroupAddressStyle, Language, Project};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn group_address_hint_uses_the_active_project_address_style() {
    for (style, syntax, example) in [
        (GroupAddressStyle::TwoLevel, "main/sub", "1/42"),
        (GroupAddressStyle::Free, "0..65535", "1234"),
    ] {
        let state = Arc::new(knx_server::AppState::default());
        let mut project = Project::new(Language("en".into()));
        project.info.group_address_style = style;
        *state.project.lock().unwrap() = Some(project);
        let response = knx_server::app(state, None)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/group-addresses")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"name": "Test", "address": "bad"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let refusal: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(refusal["kind"], "group_address");
        assert_eq!(refusal["syntax"], syntax);
        assert_eq!(refusal["example"], example);
    }
}

#[tokio::test]
async fn malformed_editor_values_have_stable_kinds_and_exact_syntax_hints() {
    let state = Arc::new(knx_server::AppState::default());
    *state.project.lock().unwrap() = Some(Project::new(Language("en".into())));
    let app = knx_server::app(state.clone(), None);
    for (route, body, kind, syntax, example) in [
        (
            "/api/individual-address",
            json!({"deviceId": 1, "address": "bad"}),
            "individual_address",
            "area.line.device",
            "1.1.10",
        ),
        (
            "/api/group-addresses",
            json!({"name": "Test", "address": "bad"}),
            "group_address",
            "main/middle/sub",
            "1/2/3",
        ),
        (
            "/api/com-object-dpt",
            json!({"comObjectId": 1, "dpt": "5.1"}),
            "dpt",
            "DPT-<main> or DPST-<main>-<sub>",
            "DPST-5-1",
        ),
        (
            "/api/project/new",
            json!({"language": "en_US"}),
            "language_tag",
            "BCP-47 language tag",
            "en-US",
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(route)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{route}"
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let refusal: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(refusal["kind"], kind, "{route}");
        assert_eq!(refusal["syntax"], syntax, "{route}");
        assert_eq!(refusal["example"], example, "{route}");
        assert!(
            refusal["detail"].as_str().is_some_and(|s| !s.is_empty()),
            "{route}"
        );
        assert_eq!(
            refusal["error"], refusal["detail"],
            "legacy error stays readable: {route}"
        );
    }
    // A rejected project-creation request must not replace the open project.
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .strings
            .default_language()
            .0,
        "en"
    );
}
