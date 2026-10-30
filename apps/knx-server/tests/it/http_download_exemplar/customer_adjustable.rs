//! Proves typed HTTP refusal before tunnels, frames or History after a shown-plan source change.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

const CATEGORY: &str = "customer-adjustable-parameters";
const PARAMETER: &str = r#"<Parameters><Parameter Id="M-FFF6_A-0001-01-0000_P-991" CustomerAdjustable="privateSentinel" /></Parameters>"#;
const OPTIONS: &str = r#"<Options CustomerAdjustableParameters="privateSentinel" />"#;

struct CountingConnector {
    opened: Arc<AtomicUsize>,
    device: Arc<SimulatedDevice>,
}

impl GatewayConnector for CountingConnector {
    fn connect_tunnel(
        &self,
        gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.opened.fetch_add(1, Ordering::SeqCst);
        let _ = gateway;
        let tunnel: Box<dyn BusTunnel> = Box::new(Tunnel(Arc::clone(&self.device)));
        Box::pin(async move { Ok(tunnel) })
    }
    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async { Ok(Vec::new()) })
    }
}

fn app(
    dir: &std::path::Path,
    contents: &str,
) -> (
    axum::Router,
    Arc<knx_server::AppState>,
    Arc<AtomicUsize>,
    Arc<SimulatedDevice>,
) {
    let path = dir.join("products.sqlite");
    let products = knx_productdb::open_and_migrate(&path).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/A.xml", program_xml("").as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    let plain = knx_app::download_support::offline_plan(&products, PROGRAM_ID).unwrap();
    let device = Arc::new(device_for(&plain));
    let opened = Arc::new(AtomicUsize::new(0));
    let mut state = knx_server::AppState::new(dir.to_path_buf());
    state.product_db = Some(knx_server::CatalogueOwner::adopt_opened(products, path).unwrap());
    state.connector = Box::new(CountingConnector {
        opened: opened.clone(),
        device: device.clone(),
    });
    *state.project.lock().unwrap() = Some(project());
    if !contents.is_empty() {
        replace_source(dir, contents);
    }
    let state = Arc::new(state);
    (knx_server::app(state.clone(), None), state, opened, device)
}

fn replace_source(dir: &std::path::Path, contents: &str) {
    // A new retained synthetic source selected by the program row, not a mocked preparation error.
    let products = knx_productdb::open_and_migrate(&dir.join("products.sqlite")).unwrap();
    let bytes = program_xml(contents).into_bytes();
    let sha = knx_productdb::blob::sha256_hex(&bytes);
    knx_productdb::store_source_file(
        &products,
        &knx_productdb::SourceFile {
            source_path: "M-FFF6/current.xml".into(),
            manufacturer_id: Some("M-FFF6".into()),
            bytes,
        },
    )
    .unwrap();
    assert_eq!(
        products
            .execute(
                "UPDATE application_program SET source_sha256=?1 WHERE id=?2",
                [&sha, PROGRAM_ID]
            )
            .unwrap(),
        1
    );
}

fn assert_refusal(status: StatusCode, body: &Value) {
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["kind"], "downloadPreparationRefused");
    assert_eq!(body["category"], CATEGORY);
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("no regular download scope"));
    let text = body.to_string();
    for private in ["privateSentinel", "P-991", "ModuleDef(", "Static("] {
        assert!(
            !text.contains(private),
            "new HTTP refusal leaked source payload"
        );
    }
}

fn scopes() -> [Value; 4] {
    [
        Value::Null,
        json!({"parameters":true,"groupAddresses":false}),
        json!({"parameters":false,"groupAddresses":true}),
        json!({"parameters":true,"groupAddresses":true}),
    ]
}

async fn assert_no_activity(
    app: &axum::Router,
    state: &knx_server::AppState,
    opened: &AtomicUsize,
    device: &SimulatedDevice,
) {
    assert_eq!(
        opened.load(Ordering::SeqCst),
        0,
        "no tunnel may have been requested"
    );
    assert!(
        device.seen().is_empty(),
        "no frame may have reached the simulator"
    );
    assert!(state.device_download.lock().await.is_none());
    let (status, history, _) = call(app, get("/api/bus/history")).await;
    assert_eq!(status, StatusCode::OK, "{history}");
    assert_eq!(
        history["entries"],
        json!([]),
        "preflight refusal invents no persisted run"
    );
    let (status, _, _) = call(app, get("/api/device-download/report")).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "preflight creates no download report"
    );
}

#[tokio::test]
async fn http_plan_all_scopes_readiness_and_support_refuse_with_private_tokens_withheld() {
    for marker in [PARAMETER, OPTIONS] {
        let dir = tempfile::tempdir().unwrap();
        let (app, state, opened, device) = app(dir.path(), marker);
        for partial in scopes() {
            let mut request = json!({"address":TARGET});
            if !partial.is_null() {
                request["partial"] = partial;
            }
            let (status, body, _) = call(&app, post("/api/device-download/plan", request)).await;
            assert_refusal(status, &body);
        }
        let (status, body, text) = call(&app, get("/api/device-readiness")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["devices"][0]["category"], CATEGORY);
        assert!(!text.contains("privateSentinel"));
        let (status, preview, text) = call(&app, post("/api/support/preview", json!({"input":{"programId":PROGRAM_ID,"includeContext":true,"contextRights":true}}))).await;
        assert_eq!(status, StatusCode::OK, "{preview}");
        assert_eq!(preview["report"]["support"], "unsupported");
        assert!(preview["report"]["facts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|fact| fact["field"] == "refusalCategory" && fact["value"] == CATEGORY));
        assert!(!text.contains("privateSentinel"));
        assert!(!text.contains("P-991"));
        assert_no_activity(&app, &state, &opened, &device).await;
    }
}

#[tokio::test]
async fn http_start_rechecks_each_genuinely_shown_scope_after_source_changes_before_tunnel_or_frame(
) {
    for marker in [PARAMETER, OPTIONS] {
        for partial in scopes() {
            let dir = tempfile::tempdir().unwrap();
            let (app, state, opened, device) = app(dir.path(), "");
            let mut request = json!({"address":TARGET});
            if !partial.is_null() {
                request["partial"] = partial;
            }
            let (status, shown, _) = call(&app, post("/api/device-download/plan", request)).await;
            assert_eq!(
                status,
                StatusCode::OK,
                "markerless source must actually plan: {shown}"
            );
            assert_eq!(shown["support"]["level"], "untested");
            let confirmation = shown["confirmationPhrase"].as_str().unwrap();
            replace_source(dir.path(), marker);
            let request = json!({"planId":shown["planId"],"gateway":"192.0.2.10:3671","confirmation":confirmation,"acceptUntested":format!("I accept an untested download to {TARGET}")});
            // Earlier confirmation safety remains authoritative even after source change.
            let mut invalid = request.clone();
            invalid["confirmation"] = json!("not the required phrase");
            let (status, body, _) = call(&app, post("/api/device-download/start", invalid)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(body["kind"], "downloadConfirmationRefused");
            // Legitimate actual Start request: no artificial shown plan or invalid prerequisite.
            let (status, body, _) = call(&app, post("/api/device-download/start", request)).await;
            assert_refusal(status, &body);
            assert_no_activity(&app, &state, &opened, &device).await;
        }
    }
}
