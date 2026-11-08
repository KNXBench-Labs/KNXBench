//! Prove a start re-checks stop admission under its reservation, before any connector call.

use std::future::Future;
use std::net::SocketAddrV4;
use std::pin::Pin;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use knx_net::TunnelTransport;

use super::{start_monitor_target, MonitorTarget};
use crate::bus::fake::{FakeConnector, FakeTunnel};
use crate::bus::{BusSessionError, BusTunnel, GatewayConnector};
use crate::AppState;

/// Lets the test read the connector's counters after handing it to the state.
struct Shared(Arc<FakeConnector>);
impl GatewayConnector for Shared {
    fn connect_tunnel(
        &self,
        gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.0.connect_tunnel(gateway)
    }
    fn connect_tunnel_with_transport(
        &self,
        gateway: SocketAddrV4,
        transport: TunnelTransport,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.0.connect_tunnel_with_transport(gateway, transport)
    }
    #[allow(clippy::type_complexity)]
    fn discover(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Vec<knx_net::DiscoveredGateway>, BusSessionError>>
                + Send
                + '_,
        >,
    > {
        self.0.discover()
    }
}

fn state_with(connector: Arc<FakeConnector>) -> Arc<AppState> {
    Arc::new(AppState {
        connector: Box::new(Shared(connector)),
        ..Default::default()
    })
}

fn tcp_fake() -> Arc<FakeConnector> {
    let (tunnel, _handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 8);
    Arc::new(FakeConnector::succeeding(tunnel.with_transport(TunnelTransport::Tcp)).accepting_tcp())
}

const GATEWAY: &str = "192.0.2.10:3671";

#[tokio::test]
async fn admission_closed_after_the_handler_but_before_the_task_runs_opens_nothing() {
    let connector = tcp_fake();
    let state = state_with(connector.clone());
    let gateway: SocketAddrV4 = GATEWAY.parse().unwrap();
    // The handler has admitted; the task it spawned has not run yet. On this single-threaded
    // runtime nothing runs until the test awaits, so the close lands exactly in that window.
    state.stop_admission.admit().unwrap();
    let task = tokio::spawn(start_monitor_target(
        state.clone(),
        MonitorTarget::Tcp(gateway),
    ));
    state.stop_admission.close();
    let refusal = task
        .await
        .unwrap()
        .err()
        .expect("a closed admission must refuse");
    assert_eq!(
        refusal.into_response().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(connector.tcp_call_count(), 0);
    assert_eq!(connector.call_count(), 0);
    assert!(state.bus_session.try_lock().unwrap().is_none());
}

#[tokio::test]
async fn every_target_is_rechecked_not_only_tcp() {
    for target in [
        MonitorTarget::Ip(GATEWAY.parse().unwrap()),
        MonitorTarget::Tcp(GATEWAY.parse().unwrap()),
    ] {
        let connector = tcp_fake();
        let state = state_with(connector.clone());
        state.stop_admission.close();
        let refusal = start_monitor_target(state.clone(), target)
            .await
            .err()
            .expect("closed admission refuses");
        assert_eq!(
            refusal.into_response().status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(connector.tcp_call_count() + connector.call_count(), 0);
    }
}

#[tokio::test]
async fn open_admission_is_the_positive_control_and_stores_the_session() {
    let connector = tcp_fake();
    let state = state_with(connector.clone());
    let gateway: SocketAddrV4 = GATEWAY.parse().unwrap();
    state.stop_admission.admit().unwrap();
    let task = tokio::spawn(start_monitor_target(
        state.clone(),
        MonitorTarget::Tcp(gateway),
    ));
    let started = task
        .await
        .unwrap()
        .unwrap_or_else(|_| panic!("an open admission starts"));
    assert_eq!(started.0.transport, Some("tcp"));
    assert_eq!(connector.tcp_call_count(), 1);
    let session = state
        .bus_session
        .lock()
        .await
        .take()
        .expect("the session is stored");
    session.stop().await;
}
