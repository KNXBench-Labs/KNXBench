//! Prove bound group actions and serial cleanup admission with deterministic async fake barriers.

use super::*;
use knx_core::{GroupAddress, GroupAddressStyle, GroupValue, Language};
use tokio::sync::Semaphore;

struct Control {
    serial: Mutex<Vec<SerialTarget>>,
    ip: AtomicUsize,
    peers: Mutex<Vec<FakeTunnelHandle>>,
    block_connect: AtomicBool,
    connect_entered: Semaphore,
    connect_release: Semaphore,
    block_cleanup: AtomicBool,
    fail_cleanup: AtomicBool,
    cleanup_entered: Semaphore,
    cleanup_release: Semaphore,
}
impl Default for Control {
    fn default() -> Self {
        Self {
            serial: Mutex::new(vec![]),
            ip: AtomicUsize::new(0),
            peers: Mutex::new(vec![]),
            block_connect: AtomicBool::new(false),
            connect_entered: Semaphore::new(0),
            connect_release: Semaphore::new(0),
            block_cleanup: AtomicBool::new(false),
            fail_cleanup: AtomicBool::new(false),
            cleanup_entered: Semaphore::new(0),
            cleanup_release: Semaphore::new(0),
        }
    }
}
struct LifecycleConnector(Arc<Control>);
impl LifecycleConnector {
    fn link(&self) -> Box<dyn BusTunnel> {
        let (tunnel, handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 64);
        let mut peers = self.0.peers.lock().unwrap();
        let first = peers.is_empty();
        peers.push(handle);
        Box::new(LifecycleLink {
            inner: tunnel,
            control: self.0.clone(),
            first,
        })
    }
}
impl GatewayConnector for LifecycleConnector {
    fn connect_tunnel(&self, _: SocketAddrV4) -> LinkFuture<'_> {
        self.0.ip.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { Ok(self.link()) })
    }
    fn connect_serial<'a>(&'a self, target: &'a SerialTarget) -> LinkFuture<'a> {
        self.0.serial.lock().unwrap().push(target.clone());
        Box::pin(async move {
            if self.0.block_connect.load(Ordering::SeqCst) {
                self.0.connect_entered.add_permits(1);
                self.0.connect_release.acquire().await.unwrap().forget();
            }
            Ok(self.link())
        })
    }
    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        panic!("no discovery")
    }
}
struct LifecycleLink {
    inner: FakeTunnel,
    control: Arc<Control>,
    first: bool,
}
impl BusTunnel for LifecycleLink {
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        self.inner.assigned_address()
    }
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TunnelEvent> {
        self.inner.subscribe()
    }
    fn send(
        &self,
        d: Destination,
        s: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        self.inner.send(d, s)
    }
    fn send_frame(
        &self,
        d: Destination,
        t: Tpci,
        s: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        self.inner.send_frame(d, t, s)
    }
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async move {
            if self.first && self.control.block_cleanup.load(Ordering::SeqCst) {
                self.control.cleanup_entered.add_permits(1);
                self.control
                    .cleanup_release
                    .acquire()
                    .await
                    .unwrap()
                    .forget();
            }
            Box::new(self.inner).disconnect().await?;
            if self.first && self.control.fail_cleanup.load(Ordering::SeqCst) {
                return Err(BusSessionError::Transport(BusError::Timeout));
            }
            Ok(())
        })
    }
}
struct Lifecycle {
    state: Arc<AppState>,
    app: Router,
    control: Arc<Control>,
}
fn lifecycle(style: Option<GroupAddressStyle>) -> Lifecycle {
    let control = Arc::new(Control::default());
    let project = style.map(|style| {
        let mut project = knx_core::Project::new(Language("en".into()));
        project.info.group_address_style = style;
        project
    });
    let state = Arc::new(AppState {
        connector: Box::new(LifecycleConnector(control.clone())),
        project: Mutex::new(project),
        ..Default::default()
    });
    let app = knx_server::app(state.clone(), None);
    Lifecycle {
        state,
        app,
        control,
    }
}
pub(super) fn binding(started: &Value, mut body: Value) -> Value {
    body["sessionId"] = started["sessionId"].clone();
    body["serverIncarnation"] = started["serverIncarnation"].clone();
    body
}
fn stop_uri(started: &Value) -> String {
    format!(
        "/api/bus/monitor/stop-bound?sessionId={}&serverIncarnation={}",
        started["sessionId"],
        started["serverIncarnation"].as_str().unwrap()
    )
}
async fn begin(f: &Lifecycle, serial: bool, path: &str) -> Value {
    let mut body = target("active");
    body["path"] = path.into();
    let (status, started) = if serial {
        call(&f.app, "POST", "/api/bus/monitor/start-serial", body).await
    } else {
        call(
            &f.app,
            "POST",
            "/api/bus/monitor/start",
            json!({"gateway":"192.0.2.1:3671"}),
        )
        .await
    };
    assert_eq!(status, StatusCode::OK, "{started}");
    started
}
async fn end(f: &Lifecycle, started: &Value) {
    assert_eq!(
        call(&f.app, "POST", &stop_uri(started), Value::Null)
            .await
            .0,
        StatusCode::OK
    );
}
fn group_write() -> Value {
    json!({"destination":"1/1/1","dpt":"DPST-1-1","inputFormat":null,"value":"on"})
}
async fn bounded_call(
    app: &Router,
    uri: &str,
    body: Value,
) -> Result<(StatusCode, Value), tokio::time::error::Elapsed> {
    tokio::time::timeout(Duration::from_secs(1), call(app, "POST", uri, body)).await
}
#[tokio::test]
async fn stale_serial_a_actions_never_send_on_serial_or_ip_b() {
    for serial_b in [true, false] {
        let f = lifecycle(None);
        let a = begin(&f, true, "/synthetic/a").await;
        let stale_read = binding(&a, json!({"destination":"1/1/1"}));
        let stale_write = binding(&a, group_write());
        end(&f, &a).await;
        let b = begin(&f, serial_b, "/synthetic/b").await;
        for (uri, body) in [
            ("/api/bus/read", stale_read),
            ("/api/bus/write-bound", stale_write),
        ] {
            let (status, refusal) = call(&f.app, "POST", uri, body).await;
            assert_eq!(status, StatusCode::CONFLICT);
            assert_eq!(refusal["kind"], "busSessionChanged");
        }
        let mut foreign = binding(&b, group_write());
        foreign["serverIncarnation"] = "foreign".into();
        assert_eq!(
            call(&f.app, "POST", "/api/bus/write-bound", foreign)
                .await
                .1["kind"],
            "busSessionChanged"
        );
        assert!(f.control.peers.lock().unwrap()[1].sent_calls().is_empty());
        if serial_b {
            assert_eq!(
                call(&f.app, "POST", "/api/bus/write", group_write())
                    .await
                    .1["kind"],
                "busSessionBindingRequired"
            );
            assert!(f.control.peers.lock().unwrap()[1].sent_calls().is_empty());
        } else {
            assert_eq!(
                call(&f.app, "POST", "/api/bus/write", group_write())
                    .await
                    .0,
                StatusCode::OK
            );
            assert_eq!(
                f.control.peers.lock().unwrap()[1].sent_calls(),
                vec![(
                    Destination::Group(GroupAddress::from_raw(0x0901)),
                    ApplicationService::GroupValueWrite(GroupValue::Short(1))
                )]
            );
        }
        end(&f, &b).await;
    }
}
#[tokio::test]
async fn bound_group_requests_refuse_immediately_while_a_real_start_holds_the_reservation() {
    let f = lifecycle(None);
    f.control.block_connect.store(true, Ordering::SeqCst);
    let app = f.app.clone();
    let connecting = tokio::spawn(async move {
        call(
            &app,
            "POST",
            "/api/bus/monitor/start-serial",
            target("active"),
        )
        .await
    });
    f.control.connect_entered.acquire().await.unwrap().forget();
    let pending = json!({"sessionId":1,"serverIncarnation":f.state.server_incarnation});
    let read = bounded_call(
        &f.app,
        "/api/bus/read",
        binding(&pending, json!({"destination":"1/1/1"})),
    )
    .await;
    let write = bounded_call(
        &f.app,
        "/api/bus/write-bound",
        binding(&pending, group_write()),
    )
    .await;
    let no_peer = f.control.peers.lock().unwrap().is_empty();
    f.control.connect_release.add_permits(1);
    let (_, started) = connecting.await.unwrap();
    for answer in [read, write] {
        let (status, body) = answer.expect("bound action queued behind connect");
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["kind"], "gatewayBusy");
    }
    assert!(no_peer);
    assert!(f.control.peers.lock().unwrap()[0].sent_calls().is_empty());
    end(&f, &started).await;
}
#[tokio::test]
async fn bound_read_write_validate_exact_bodies_before_any_lock_or_send() {
    let f = lifecycle(None);
    let started = begin(&f, true, "/synthetic/a").await;
    for uri in ["/api/bus/read", "/api/bus/write-bound"] {
        let valid = if uri.ends_with("read") {
            binding(&started, json!({"destination":"1/1/1"}))
        } else {
            binding(&started, group_write())
        };
        let mut cases = vec![json!({}), Value::Null];
        for field in ["sessionId", "serverIncarnation", "destination"] {
            let mut body = valid.clone();
            body.as_object_mut().unwrap().remove(field);
            cases.push(body);
        }
        for id in [
            json!(0),
            json!(-1),
            json!(1.5),
            json!("1"),
            json!(9_007_199_254_740_992_u64),
        ] {
            let mut body = valid.clone();
            body["sessionId"] = id;
            cases.push(body);
        }
        for inc in [json!(""), json!("x y"), json!(null)] {
            let mut body = valid.clone();
            body["serverIncarnation"] = inc;
            cases.push(body);
        }
        let mut extra = valid.clone();
        extra["gateway"] = "192.0.2.1:3671".into();
        cases.push(extra);
        if uri.ends_with("write-bound") {
            let mut body = valid.clone();
            body.as_object_mut().unwrap().remove("value");
            cases.push(body);
        }
        let held = f.state.bus_session.lock().await;
        for body in cases {
            assert_eq!(
                bounded_call(&f.app, uri, body)
                    .await
                    .expect("malformed request queued")
                    .0,
                StatusCode::BAD_REQUEST
            );
        }
        let encoded = valid.to_string();
        for duplicate in ["sessionId", "serverIncarnation", "destination"] {
            let raw_body = format!("{{\"{duplicate}\":{},{}", valid[duplicate], &encoded[1..]);
            assert_eq!(
                raw(&f.app, "POST", uri, &raw_body).await.0,
                StatusCode::BAD_REQUEST
            );
        }
        assert_eq!(
            raw(&f.app, "POST", uri, "{").await.0,
            StatusCode::BAD_REQUEST
        );
        drop(held);
    }
    for uri in ["/api/bus/read", "/api/bus/write-bound"] {
        let body = binding(
            &started,
            if uri.ends_with("read") {
                json!({"destination":"bad"})
            } else {
                let mut w = group_write();
                w["destination"] = "bad".into();
                w
            },
        );
        assert_eq!(
            call(&f.app, "POST", uri, body).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert!(f.control.peers.lock().unwrap()[0].sent_calls().is_empty());
    end(&f, &started).await;
}
#[tokio::test]
async fn exact_bound_controls_send_one_service_payload_and_destination_in_each_address_style() {
    for (style, destination) in [
        (GroupAddressStyle::Free, "2305"),
        (GroupAddressStyle::TwoLevel, "1/257"),
        (GroupAddressStyle::ThreeLevel, "1/1/1"),
    ] {
        let f = lifecycle(Some(style));
        let started = begin(&f, true, "/synthetic/a").await;
        let read = binding(&started, json!({"destination":destination}));
        assert_eq!(
            call(&f.app, "POST", "/api/bus/read", read).await.0,
            StatusCode::OK
        );
        assert_eq!(
            f.control.peers.lock().unwrap()[0].sent_calls(),
            vec![(
                Destination::Group(GroupAddress::from_raw(2305)),
                ApplicationService::GroupValueRead
            )]
        );
        let mut write = group_write();
        write["destination"] = destination.into();
        assert_eq!(
            call(
                &f.app,
                "POST",
                "/api/bus/write-bound",
                binding(&started, write)
            )
            .await
            .0,
            StatusCode::OK
        );
        assert_eq!(
            f.control.peers.lock().unwrap()[0].sent_calls(),
            vec![
                (
                    Destination::Group(GroupAddress::from_raw(2305)),
                    ApplicationService::GroupValueRead
                ),
                (
                    Destination::Group(GroupAddress::from_raw(2305)),
                    ApplicationService::GroupValueWrite(GroupValue::Short(1))
                )
            ]
        );
        end(&f, &started).await;
    }
}
async fn cleanup_barrier(legacy: bool, fail: bool) {
    let f = lifecycle(None);
    f.control.block_cleanup.store(true, Ordering::SeqCst);
    f.control.fail_cleanup.store(fail, Ordering::SeqCst);
    let a = begin(&f, true, "/synthetic/a").await;
    let uri = if legacy {
        "/api/bus/monitor/stop".to_owned()
    } else {
        stop_uri(&a)
    };
    let app = f.app.clone();
    let stopping = tokio::spawn(async move { call(&app, "POST", &uri, Value::Null).await });
    f.control.cleanup_entered.acquire().await.unwrap().forget();
    let mut other = target("active");
    other["path"] = "/synthetic/b".into();
    let next_serial = bounded_call(&f.app, "/api/bus/monitor/start-serial", other).await;
    let next_ip = bounded_call(
        &f.app,
        "/api/bus/monitor/start",
        json!({"gateway":"192.0.2.1:3671"}),
    )
    .await;
    let second_bound = bounded_call(&f.app, &stop_uri(&a), Value::Null).await;
    let second_legacy = bounded_call(&f.app, "/api/bus/monitor/stop", Value::Null).await;
    let attempts = (
        f.control.serial.lock().unwrap().len(),
        f.control.ip.load(Ordering::SeqCst),
    );
    f.control.cleanup_release.add_permits(1);
    let (status, summary) = stopping.await.unwrap();
    for answer in [next_serial, next_ip, second_bound, second_legacy] {
        let (status, body) = answer.expect("operation queued through serial cleanup");
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["kind"], "gatewayBusy");
    }
    assert_eq!(attempts, (1, 0));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary.get("warning").is_some(), fail);
    let b = begin(&f, true, "/synthetic/b").await;
    assert_eq!(f.control.serial.lock().unwrap().len(), 2);
    assert_eq!(
        call(&f.app, "POST", &stop_uri(&a), Value::Null).await.1["kind"],
        "busSessionChanged"
    );
    assert_eq!(
        f.state
            .bus_session
            .try_lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .id(),
        b["sessionId"].as_u64().unwrap()
    );
    assert!(!f.control.peers.lock().unwrap()[1].disconnected());
    end(&f, &b).await;
}
#[tokio::test]
async fn bound_stop_reserves_serial_until_clean_disconnect() {
    cleanup_barrier(false, false).await;
}
#[tokio::test]
async fn bound_stop_reserves_serial_until_failed_disconnect() {
    cleanup_barrier(false, true).await;
}
#[tokio::test]
async fn legacy_stop_reserves_serial_until_clean_disconnect() {
    cleanup_barrier(true, false).await;
}
#[tokio::test]
async fn legacy_stop_reserves_serial_until_failed_disconnect() {
    cleanup_barrier(true, true).await;
}
