//! HTTP routes for the read-only device diagnostic export.
//!
//! Single-device, snapshot-bound read-only diagnosis with the generic identity
//! recipe or the ProductDB-declared memory profile (ADR-0143). Background ownership
//! keeps partial bytes and teardown alive when a browser disconnects or cancels.
use crate::{errors::ApiError, SharedState};
use axum::{
    extract::{DefaultBodyLimit, Extension, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use knx_app::device_diagnostics::{
    self as service, Export, MemoryEvidence, MemoryPrepared, Prepared, Report,
};
use knx_core::DeviceId;
use serde::{Deserialize, Serialize};
use std::net::SocketAddrV4;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

/// Read-only bridge: forwards to the existing tunnel; simulation provenance
/// cannot grant write authority (the collector always builds read_only/Skip).
struct DiagnosticTransport<'a>(&'a dyn crate::bus::BusTunnel);
impl knx_net::ManagementTransport for DiagnosticTransport<'_> {
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        self.0.assigned_address()
    }
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<knx_net::TunnelEvent> {
        self.0.subscribe()
    }
    fn target_kind(&self) -> knx_core::commissioning::mutation::TargetKind {
        if self.0.diagnostics_are_simulated() {
            knx_core::commissioning::mutation::TargetKind::Simulator
        } else {
            knx_core::commissioning::mutation::TargetKind::Hardware
        }
    }
    async fn send_frame(
        &self,
        d: knx_net::Destination,
        t: knx_net::Tpci,
        s: knx_net::ApplicationService,
    ) -> Result<(), knx_net::BusError> {
        self.0.send_frame(d, t, s).await
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Binding {
    server_incarnation: String,
    project_incarnation: u64,
    revision: u64,
}
/// The opaque plan of exactly one profile; only the service's `prepare*` builds it.
#[derive(Clone)]
enum Plan {
    Identity(Prepared),
    Memory(MemoryPrepared),
}
impl Plan {
    fn base(&self) -> &Prepared {
        match self {
            Plan::Identity(plan) => plan,
            Plan::Memory(plan) => plan.base(),
        }
    }
    fn pending(&self, build: &str) -> Export {
        match self {
            Plan::Identity(plan) => Export {
                report: service::pending(plan, build, false),
                memory: None,
            },
            Plan::Memory(plan) => service::pending_memory(plan, build, false),
        }
    }
}
#[derive(Clone)]
struct Job {
    token: String,
    created: Instant,
    plan: Plan,
    gateway: SocketAddrV4,
    binding: Binding,
    phase: &'static str,
    export: Export,
    /// Whether the finished export passed the archive writer's own check.
    /// A format-2 run stopped before its tunnel (gateway busy, connect
    /// failure, ...) has no admitted archive shape, like the CLI's refusal.
    exportable: bool,
    cancel: Arc<AtomicBool>,
}
type Slot = Arc<Mutex<Option<Job>>>;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct View {
    token: String,
    target: String,
    gateway: String,
    phase: &'static str,
    cancel_requested: bool,
    report: Report,
    /// `memory.json` of format 2; `null` for `generic-identity-v1`.
    memory: Option<MemoryEvidence>,
    exportable: bool,
}
fn view(job: &Job) -> View {
    View {
        token: job.token.clone(),
        target: job.plan.base().target.address().to_string(),
        gateway: job.gateway.to_string(),
        phase: job.phase,
        cancel_requested: job.cancel.load(Ordering::SeqCst),
        report: job.export.report.clone(),
        memory: job.export.memory.clone(),
        exportable: job.exportable,
    }
}
fn error(code: &str) -> ApiError {
    ApiError::with_status(StatusCode::CONFLICT, code)
}
fn fresh(state: &SharedState, job: &Job) -> Result<(), ApiError> {
    if job.created.elapsed() > Duration::from_secs(900)
        || job.binding.server_incarnation != state.server_incarnation
        || job.binding.project_incarnation != state.project_incarnation.load(Ordering::SeqCst)
        || job.binding.revision != state.project_revision.load(Ordering::SeqCst)
    {
        return Err(error("diagnostics-context-changed"));
    }
    Ok(())
}
fn snapshot(slot: &Slot, token: &str) -> Result<Job, ApiError> {
    slot.lock()
        .map_err(|_| error("diagnostics-unavailable"))?
        .as_ref()
        .filter(|j| j.token == token)
        .cloned()
        .ok_or_else(|| error("diagnostics-unknown-token"))
}
pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/device-diagnostics/preview", post(preview))
        .route("/api/device-diagnostics/start", post(start))
        .route("/api/device-diagnostics/export", post(export))
        .route("/api/device-diagnostics/{token}", get(status))
        .route("/api/device-diagnostics/{token}/cancel", post(cancel))
        .layer(DefaultBodyLimit::max(4096))
        .layer(Extension(Slot::default()))
        .layer(axum::middleware::map_response(
            |mut r: Response| async move {
                r.headers_mut().insert(
                    header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("no-store"),
                );
                r
            },
        ))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewRequest {
    device_id: u32,
    gateway: String,
    /// Required: `generic-identity-v1` or `system2-declared-segments-v1`.
    profile: String,
    context: Binding,
}
async fn preview(
    State(state): State<SharedState>,
    Extension(slot): Extension<Slot>,
    Json(body): Json<PreviewRequest>,
) -> Result<Json<View>, ApiError> {
    let gateway: SocketAddrV4 = body
        .gateway
        .parse()
        .map_err(|_| ApiError::bad_request("diagnostics-ipv4-tunnelling-only"))?;
    if gateway.port() == 0
        || gateway.ip().is_unspecified()
        || gateway.ip().is_multicast()
        || gateway.ip().is_broadcast()
    {
        return Err(ApiError::bad_request("diagnostics-invalid-gateway"));
    }
    let memory = match body.profile.as_str() {
        service::PROFILE => false,
        service::MEMORY_PROFILE => true,
        _ => return Err(ApiError::bad_request("diagnostics-unknown-profile")),
    };
    // Like the CLI's `--product-db`: the ranges come only from product data.
    if memory && state.product_db.is_none() {
        return Err(ApiError::bad_request("diagnostics-product-db-required"));
    }
    let project = state
        .project
        .lock()
        .map_err(|_| error("diagnostics-project-unavailable"))?;
    let p = project
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("diagnostics-project-required"))?;
    let device = DeviceId(body.device_id);
    let plan = match state.product_db.as_ref() {
        None => service::prepare(p, device, None).map(Plan::Identity),
        Some(owner) => owner
            .with_read(|conn| {
                if memory {
                    service::prepare_memory(p, device, conn).map(Plan::Memory)
                } else {
                    service::prepare(p, device, Some(conn)).map(Plan::Identity)
                }
            })
            .map_err(|_| error("diagnostics-catalogue-unavailable"))?,
    }
    .map_err(ApiError::bad_request)?;
    // The capability token is never the reference written into a shareable ZIP.
    let mut token = [0u8; 32];
    getrandom::fill(&mut token).map_err(|_| error("diagnostics-token-unavailable"))?;
    let job = Job {
        token: token.iter().map(|b| format!("{b:02x}")).collect(),
        created: Instant::now(),
        export: plan.pending(&crate::version_string()),
        exportable: false,
        plan,
        gateway,
        binding: body.context,
        phase: "preview",
        cancel: Arc::new(AtomicBool::new(false)),
    };
    fresh(&state, &job)?;
    let mut held = slot.lock().map_err(|_| error("diagnostics-unavailable"))?;
    if held.as_ref().is_some_and(|j| j.phase == "running") {
        return Err(error("diagnostics-already-running"));
    }
    let result = view(&job);
    *held = Some(job);
    Ok(Json(result))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirm {
    token: String,
    consent: bool,
}
async fn start(
    State(state): State<SharedState>,
    Extension(slot): Extension<Slot>,
    Json(body): Json<Confirm>,
) -> Result<(StatusCode, Json<View>), ApiError> {
    state.stop_admission.admit()?;
    if !body.consent {
        return Err(ApiError::bad_request("diagnostics-read-consent-required"));
    }
    let mut job = snapshot(&slot, &body.token)?;
    fresh(&state, &job)?;
    if job.phase != "preview" {
        return Err(error("diagnostics-preview-required"));
    }
    job.phase = "running";
    {
        let mut held = slot.lock().map_err(|_| error("diagnostics-unavailable"))?;
        if held
            .as_ref()
            .is_none_or(|j| j.token != body.token || j.phase != "preview")
        {
            return Err(error("diagnostics-already-running"));
        }
        *held = Some(job.clone());
    }
    let result = view(&job);
    tokio::spawn(async move {
        run(state, slot, job).await;
    });
    Ok((StatusCode::ACCEPTED, Json(result)))
}
fn publish(slot: &Slot, token: &str, export: Export, complete: bool) {
    // Checked outside the lock: the same validation and readback as the export.
    let exportable = complete && service::bundle_export(&export).is_ok();
    if let Ok(mut held) = slot.lock() {
        if let Some(job) = held.as_mut().filter(|j| j.token == token) {
            job.export = export;
            if complete {
                job.phase = "complete";
                job.exportable = exportable;
            }
        }
    }
}
async fn run(state: SharedState, slot: Slot, job: Job) {
    if let Err(reason) = run_inner(&state, &slot, &job).await {
        let mut export = snapshot(&slot, &job.token)
            .map(|j| j.export)
            .unwrap_or(job.export);
        export.report.complete = true;
        export.report.stop_reason = Some(reason.into());
        publish(&slot, &job.token, export, true);
    }
}
async fn run_inner(state: &SharedState, slot: &Slot, job: &Job) -> Result<(), &'static str> {
    // Established server order: programming try_lock → download-holder check →
    // monitor → scan. Hold these exact locks through BOTH disconnect attempts.
    let programming = state
        .address_programming
        .try_lock()
        .map_err(|_| "gateway-busy")?;
    if programming
        .as_ref()
        .and_then(crate::tunnel_holder::programming_holder_of)
        .is_some()
        || crate::tunnel_holder::download_holder(state).is_some()
    {
        return Err("gateway-busy");
    }
    let monitor = state.bus_session.try_lock().map_err(|_| "gateway-busy")?;
    if monitor.is_some() {
        return Err("gateway-busy");
    }
    let scan = state
        .line_scan_session
        .try_lock()
        .map_err(|_| "gateway-busy")?;
    if scan
        .as_ref()
        .and_then(crate::tunnel_holder::scan_holder_of)
        .is_some()
    {
        return Err("gateway-busy");
    }
    fresh(state, job).map_err(|_| "context-changed-before-connect")?;
    state
        .stop_admission
        .admit()
        .map_err(|_| "server-stopping")?;
    if job.cancel.load(Ordering::SeqCst) {
        return Err("cancelled");
    }
    let _lease = match &state.diagnostic_lease_dir {
        Some(dir) => service::lease_in(dir),
        None => service::gateway_lease(),
    }
    .map_err(|_| "gateway-busy")?;
    let activity = state.one_shot_activity.start(
        "deviceDiagnostics",
        Some(job.plan.base().target.address().to_string()),
    );
    let recorder = crate::operation_recordings::start(
        state,
        knx_app::operation_recording::OperationStart::new(
            knx_app::operation_recording::OperationKind::DeviceDiagnostics,
        )
        .target(serde_json::json!({
            "individualAddress": job.plan.base().target.address().to_string(),
            "profile": match &job.plan {
                Plan::Identity(_) => "identity",
                Plan::Memory(_) => "declaredMemory",
            },
        }))
        .gateway(job.gateway)
        .related("diagnosticsToken", &job.token)
        .settings(crate::operation_recordings::tunnel_settings(
            knx_net::TunnelAckWait::Standard,
            &state.device_download_timing,
        )),
    );
    // A known connect failure is "failed"; only a dropped future is "unknown".
    let tunnel = match tokio::time::timeout(
        Duration::from_secs(10),
        crate::operation_recordings::connect(
            state,
            job.gateway,
            knx_net::TunnelAckWait::Standard,
            &recorder,
        ),
    )
    .await
    {
        Ok(Ok(tunnel)) => tunnel,
        Ok(Err(error)) => {
            crate::operation_recordings::finish_error(&recorder, &error, serde_json::Value::Null);
            activity.finish("failed");
            return Err("gateway-connect-failed");
        }
        Err(_) => {
            crate::operation_recordings::finish_failed(
                &recorder,
                "no tunnel within 10 s",
                serde_json::Value::Null,
            );
            activity.finish("failed");
            return Err("gateway-timeout");
        }
    };
    let transport = DiagnosticTransport(tunnel.as_ref());
    let build = crate::version_string();
    let timing = state.device_download_timing;
    let mut export = match &job.plan {
        Plan::Identity(plan) => {
            let report = service::acquire(&transport, plan, timing, &build, &job.cancel, |r| {
                let export = Export {
                    report: r.clone(),
                    memory: None,
                };
                publish(slot, &job.token, export, false)
            })
            .await;
            Export {
                report,
                memory: None,
            }
        }
        Plan::Memory(plan) => {
            service::acquire_memory(&transport, plan, timing, &build, &job.cancel, |e| {
                publish(slot, &job.token, e.clone(), false)
            })
            .await
        }
    };
    // The report survives failed tunnel teardown; no "successful disconnect" claim.
    match tokio::time::timeout(Duration::from_secs(3), tunnel.disconnect()).await {
        Ok(Ok(())) => {}
        _ => {
            export
                .report
                .stop_reason
                .get_or_insert(service::TUNNEL_DISCONNECT_UNCONFIRMED.into());
        }
    }
    // The whole acquisition, every value read, for the recording.
    let summary = serde_json::json!({
        "report": serde_json::to_value(&export.report).unwrap_or_default(),
        "memory": export.memory.as_ref().map(|m| serde_json::to_value(m).unwrap_or_default()),
    });
    match &export.report.stop_reason {
        Some(reason) => {
            crate::operation_recordings::finish_failed(&recorder, reason.clone(), summary)
        }
        None => recorder.finish(knx_app::operation_recording::RecordedOutcome::succeeded(
            summary,
        )),
    }
    activity.finish(if export.report.stop_reason.is_some() {
        "failed"
    } else {
        "finished"
    });
    publish(slot, &job.token, export, true);
    drop(scan);
    drop(monitor);
    drop(programming);
    Ok(())
}
async fn status(
    Extension(slot): Extension<Slot>,
    Path(token): Path<String>,
) -> Result<Json<View>, ApiError> {
    Ok(Json(view(&snapshot(&slot, &token)?)))
}
async fn cancel(
    Extension(slot): Extension<Slot>,
    Path(token): Path<String>,
) -> Result<Json<View>, ApiError> {
    let job = snapshot(&slot, &token)?;
    if job.phase != "running" {
        return Err(error("diagnostics-not-running"));
    }
    job.cancel.store(true, Ordering::SeqCst);
    Ok(Json(view(&job)))
}
async fn export(
    Extension(slot): Extension<Slot>,
    Json(body): Json<Confirm>,
) -> Result<Response, ApiError> {
    if !body.consent {
        return Err(ApiError::bad_request("diagnostics-share-consent-required"));
    }
    let job = snapshot(&slot, &body.token)?;
    if job.phase != "complete" {
        return Err(error("diagnostics-wait-for-cleanup"));
    }
    // Deliberately no fresh-project requirement here: old/partial observations
    // belong to their opaque reviewed reference and must remain saveable.
    // Format 1 bytes are exactly `bundle`'s; format 2 adds `memory.json`.
    let bytes = service::bundle_export(&job.export).map_err(ApiError::bad_request)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=knxbench-device-diagnostics.zip",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response())
}
