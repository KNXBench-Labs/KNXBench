//! `POST /api/bus/discover` — the interface search, against a scripted connector only.
//!
//! T25. Same injection pattern as `http_bus_monitor.rs`
//! (`AppState { connector: Box::new(FakeConnector::...), ..Default::default() }`),
//! and the same absolute rule: no socket is opened and no datagram leaves
//! this process in any test here. `FakeConnector::discovering` /
//! `FakeConnector::discovery_failing` answer the search outright, so these
//! tests behave identically on a developer's LAN and in a sandbox with no
//! route to the multicast group at all.
//!
//! Every address below is from RFC 5737's documentation range
//! (`192.0.2.0/24`). Nothing here is anybody's network.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use knx_net::DiscoveredGateway;
use knx_server::fake::FakeConnector;

fn interface(last_octet: u8, device: u8, name: &str, tunnelling: bool) -> DiscoveredGateway {
    DiscoveredGateway {
        control_endpoint: std::net::SocketAddrV4::new(
            std::net::Ipv4Addr::new(192, 0, 2, last_octet),
            3671,
        ),
        individual_address: knx_core::IndividualAddress::new(1, 1, device)
            .expect("valid test address"),
        friendly_name: name.to_string(),
        supports_tunnelling: tunnelling,
        device_info: Some(knx_net::core::dib::DeviceInfo {
            medium: 129,
            status: 128,
            individual_address: knx_core::IndividualAddress::new(1, 1, device).unwrap(),
            project_installation_id: 4660,
            serial_number: [1, 2, 3, 4, 5, 6],
            routing_multicast: std::net::Ipv4Addr::new(224, 0, 23, 12),
            mac_address: [170, 187, 204, 221, 238, 255],
            friendly_name: name.into(),
        }),
    }
}

fn state_with_connector(connector: FakeConnector) -> knx_server::AppState {
    knx_server::AppState {
        connector: Box::new(connector),
        ..Default::default()
    }
}

async fn discover(app: &axum::Router) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/bus/discover")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Several interfaces answered: all of them come back, each carrying every
/// field `knx_net::DiscoveredGateway` has — including the one that says it
/// does *not* offer tunnelling, which is exactly the fact a user needs
/// before picking it.
#[tokio::test]
async fn discover_returns_every_interface_that_answered() {
    let connector = FakeConnector::discovering(vec![
        interface(11, 0, "Hallway interface", true),
        interface(12, 1, "Workshop router", false),
    ]);
    let app = knx_server::app(Arc::new(state_with_connector(connector)), None);

    let response = discover(&app).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let interfaces = body["interfaces"].as_array().unwrap();
    assert_eq!(interfaces.len(), 2);

    assert_eq!(interfaces[0]["controlEndpoint"], "192.0.2.11:3671");
    assert_eq!(interfaces[0]["individualAddress"], "1.1.0");
    assert_eq!(interfaces[0]["friendlyName"], "Hallway interface");
    assert_eq!(interfaces[0]["supportsTunnelling"], true);
    assert_eq!(
        interfaces[0]["deviceInfo"],
        serde_json::json!({
            "medium": 129, "status": 128, "projectInstallationId": 4660,
            "serialNumber": [1, 2, 3, 4, 5, 6], "routingMulticast": "224.0.23.12",
            "macAddress": [170, 187, 204, 221, 238, 255]
        })
    );

    assert_eq!(interfaces[1]["controlEndpoint"], "192.0.2.12:3671");
    assert_eq!(interfaces[1]["individualAddress"], "1.1.1");
    assert_eq!(interfaces[1]["friendlyName"], "Workshop router");
    assert_eq!(interfaces[1]["supportsTunnelling"], false);
}

#[tokio::test]
async fn discover_explicitly_reports_unavailable_adapter_metadata() {
    let mut gateway = interface(11, 0, "Legacy adapter", true);
    gateway.device_info = None;
    let app = knx_server::app(
        Arc::new(state_with_connector(FakeConnector::discovering(vec![
            gateway,
        ]))),
        None,
    );
    let body = body_json(discover(&app).await).await;
    assert!(body["interfaces"][0]
        .get("deviceInfo")
        .expect("explicit availability field")
        .is_null());
}

/// Nobody answered. That is a `200` with an empty list — the ordinary
/// result on a network with no KNX-compatible interface on it, and on any
/// host whose multicast does not leave its container. Never a `4xx`, never
/// a `5xx`: there is nothing here for the caller to fix.
#[tokio::test]
async fn discover_with_no_answers_is_a_success_with_an_empty_list() {
    let app = knx_server::app(
        Arc::new(state_with_connector(FakeConnector::discovering(Vec::new()))),
        None,
    );

    let response = discover(&app).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["interfaces"].as_array().unwrap().len(), 0);
}

/// The search itself could not run (no route to the multicast group, a
/// socket that would not bind). That *is* a failure, and it maps to `502`
/// like every other transport failure on this API — distinct from the
/// empty-result case above, which is the whole point of not collapsing
/// the two.
#[tokio::test]
async fn discover_that_could_not_run_is_a_bad_gateway() {
    let app = knx_server::app(
        Arc::new(state_with_connector(FakeConnector::discovery_failing())),
        None,
    );

    let response = discover(&app).await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

/// The route holds no state and consumes nothing: searching twice answers
/// twice. This is the Search button's premise — a network changes, and a
/// result from ten minutes ago is worth discarding.
#[tokio::test]
async fn discover_can_be_repeated() {
    let app = knx_server::app(
        Arc::new(state_with_connector(FakeConnector::discovering(vec![
            interface(11, 0, "Hallway interface", true),
        ]))),
        None,
    );

    for _ in 0..2 {
        let response = discover(&app).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["interfaces"].as_array().unwrap().len(), 1);
    }
}

/// A `GET` is not an accepted shape for this route. `POST` is deliberate:
/// a `GET` invites prefetching and speculative revalidation, and every one
/// of those would put an unasked-for multicast datagram on somebody's
/// installation network.
#[tokio::test]
async fn discover_is_not_reachable_by_get() {
    let app = knx_server::app(
        Arc::new(state_with_connector(FakeConnector::discovering(Vec::new()))),
        None,
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/bus/discover")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
