//! Snapshot-bound universal support previews, read-only diagnostics and local ZIP export.
use crate::{device_download::TunnelTransport, errors::ApiError, SharedState};
use axum::response::{IntoResponse, Response};
use axum::{
    extract::{DefaultBodyLimit, Extension, State},
    http::{header, StatusCode},
    routing::post,
    Json, Router,
};
use knx_app::support_report::{build_report, ReportInput, SupportReport};
use knx_core::{ContactableAddress, DeviceId, IndividualAddress};
use knx_net::commissioning::support_diagnostics::{collect, profile, profile_id};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::SocketAddrV4;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(1);
#[derive(Clone)]
struct Snapshot {
    token: String,
    created: Instant,
    revision: u64,
    incarnation: u64,
    has_project: bool,
    server_incarnation: String,
    acquired: bool,
    device_id: Option<u32>,
    input: ReportInput,
    base: SupportReport,
    report: SupportReport,
    address: Option<IndividualAddress>,
    manual_address: Option<IndividualAddress>,
    mask: Option<u16>,
}
type Slot = Arc<Mutex<Option<Snapshot>>>;

#[derive(Clone, Default)]
pub struct Results(Slot);

impl Results {
    pub(crate) fn evidence(
        &self,
        token: &str,
    ) -> Result<crate::diagnostic_bundle_results::Evidence, String> {
        let result =
            snapshot(&self.0, token).map_err(|_| "the retained support result is unavailable")?;
        if result.created.elapsed() > Duration::from_secs(900) {
            return Err("the retained support result has expired".into());
        }
        if !result.acquired {
            return Err("the support result is not complete".into());
        }
        Ok(crate::diagnostic_bundle_results::Evidence {
            identity: serde_json::json!({"supportReportSha256":hash(&result.report)}),
            context: Some(crate::diagnostic_bundle_context::ProjectContext {
                server_incarnation: result.server_incarnation,
                project_incarnation: result.incarnation,
                revision: result.revision,
            }),
            operation_files: Vec::new(),
            device_id: result.device_id,
            reports: crate::diagnostic_bundle_results::members(
                result.report.bundle()?,
                &["report.json", "README.txt"],
            )?,
        })
    }
}
pub fn support_routes(results: Results) -> Router<SharedState> {
    Router::new()
        .route("/api/support/preview", post(preview))
        .route("/api/support/diagnose", post(diagnose))
        .route("/api/support/export", post(export))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("no-store"),
                );
                response
            },
        ))
        .layer(DefaultBodyLimit::max(8192))
        .layer(Extension(results.0))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Binding {
    server_incarnation: String,
    project_incarnation: u64,
    revision: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewRequest {
    device_id: Option<u32>,
    address: Option<String>,
    #[serde(default)]
    input: ReportInput,
    download_id: Option<u64>,
    context: Option<Binding>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Preview {
    token: String,
    digest: String,
    report: SupportReport,
    summary: String,
    local_address: Option<String>,
    mask: Option<u16>,
    profile: &'static str,
    reads: Vec<String>,
    deadline_seconds: u8,
}
fn hash(r: &SupportReport) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(r).expect("report serializes"))
    )
}
fn view(s: &Snapshot) -> Preview {
    let mut reads =
        vec!["DeviceDescriptorRead type 0; verify selected mask before family reads".into()];
    if let Some(mask) = s.mask {
        reads.extend(profile(mask).into_iter().map(|q| q.description()));
    }
    Preview {
        token: s.token.clone(),
        digest: hash(&s.report),
        summary: s.report.summary(),
        report: s.report.clone(),
        local_address: s.address.map(|a| a.to_string()),
        mask: s.mask,
        profile: s.mask.map(profile_id).unwrap_or("descriptor-only-v1"),
        reads,
        deadline_seconds: 30,
    }
}
fn base(
    state: &SharedState,
    device_id: Option<u32>,
    input: &ReportInput,
) -> Result<(SupportReport, Option<IndividualAddress>, u64, u64, bool), ApiError> {
    let project = state
        .project
        .lock()
        .map_err(|_| ApiError::internal("project unavailable"))?;
    let selected = project
        .as_ref()
        .and_then(|p| p.devices.iter().find(|d| Some(d.id.0) == device_id));
    if device_id.is_some() && selected.is_none() {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "support-target-changed",
        ));
    }
    let report = match state.product_db.as_ref() {
        None => build_report(
            None,
            project.as_ref(),
            device_id.map(DeviceId),
            input,
            &crate::version_string(),
        ),
        Some(owner) => owner
            .with_read(|conn| {
                Ok::<_, std::convert::Infallible>(build_report(
                    Some(conn),
                    project.as_ref(),
                    device_id.map(DeviceId),
                    input,
                    &crate::version_string(),
                ))
            })
            .map_err(|_| ApiError::internal("catalogue unavailable"))?
            .unwrap(),
    };
    Ok((
        report,
        selected.and_then(|d| d.address),
        state.project_revision.load(Ordering::SeqCst),
        state.project_incarnation.load(Ordering::SeqCst),
        project.is_some(),
    ))
}
fn fresh(state: &SharedState, s: &Snapshot) -> Result<(), ApiError> {
    if s.created.elapsed() > Duration::from_secs(900) {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "support-preview-expired",
        ));
    }
    let (base, addr, rev, inc, has) = base(state, s.device_id, &s.input)?;
    if rev != s.revision
        || inc != s.incarnation
        || has != s.has_project
        || addr.or(s.manual_address) != s.address
        || base != s.base
    {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "support-context-changed-preview-again",
        ));
    }
    Ok(())
}
async fn preview(
    State(state): State<SharedState>,
    Extension(slot): Extension<Slot>,
    Json(b): Json<PreviewRequest>,
) -> Result<Json<Preview>, ApiError> {
    if b.input.include_context && !b.input.context_rights {
        return Err(ApiError::bad_request("source-rights-confirmation-required"));
    }
    let (base, address, revision, incarnation, has_project) = base(&state, b.device_id, &b.input)?;
    if b.device_id.is_some() && b.address.is_some() {
        return Err(ApiError::bad_request("support-choose-one-target"));
    }
    let manual_address = b
        .address
        .as_deref()
        .map(|a| {
            a.parse::<IndividualAddress>()
                .map_err(|_| ApiError::bad_request("support-invalid-target"))
        })
        .transpose()?;
    let address = address.or(manual_address);
    if b.device_id.is_some()
        && b.context.as_ref().is_none_or(|c| {
            c.server_incarnation != state.server_incarnation
                || c.project_incarnation != incarnation
                || c.revision != revision
        })
    {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "support-selection-stale",
        ));
    }
    let mut report = base.clone();
    if manual_address.is_some_and(|a| ContactableAddress::new(a).is_err()) {
        report.support = "excluded".into();
        report.fact(
            "refusalCategory",
            "target-policy",
            Some("target-excluded".into()),
            "unavailable",
        );
    }
    if let Some(id) = b.download_id {
        let run = state
            .device_download
            .try_lock()
            .map_err(|_| ApiError::with_status(StatusCode::CONFLICT, "support-run-busy"))?;
        let session = run.as_ref().filter(|s| s.id() == id).ok_or_else(|| {
            ApiError::with_status(StatusCode::CONFLICT, "support-run-unavailable")
        })?;
        if address != Some(session.target()) {
            return Err(ApiError::with_status(
                StatusCode::CONFLICT,
                "support-run-target-mismatch",
            ));
        }
        let old = session
            .report()
            .ok_or_else(|| ApiError::with_status(StatusCode::CONFLICT, "support-run-not-ended"))?;
        let same = base
            .facts
            .iter()
            .any(|f| f.field == "applicationId" && f.value.as_deref() == Some(&old.program.id))
            && base
                .facts
                .iter()
                .any(|f| f.field == "sourceSha256" && f.value == old.program.source_sha256);
        if !same {
            return Err(ApiError::with_status(
                StatusCode::CONFLICT,
                "support-run-source-mismatch",
            ));
        }
        report.prior_run(&old);
    }
    let mask = base
        .facts
        .iter()
        .rev()
        .find(|f| f.field == "mask" && f.value.is_some())
        .and_then(|f| u16::from_str_radix(f.value.as_ref()?, 16).ok());
    let token = format!(
        "{:x}",
        Sha256::digest(format!(
            "{}:{}:{}",
            state.server_incarnation,
            NEXT.fetch_add(1, Ordering::Relaxed),
            hash(&report)
        ))
    );
    let s = Snapshot {
        token,
        created: Instant::now(),
        revision,
        incarnation,
        has_project,
        server_incarnation: state.server_incarnation.clone(),
        acquired: false,
        device_id: b.device_id,
        input: b.input,
        base,
        report,
        address,
        manual_address,
        mask,
    };
    let result = view(&s);
    *slot
        .lock()
        .map_err(|_| ApiError::internal("support busy"))? = Some(s);
    Ok(Json(result))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExportRequest {
    token: String,
    digest: String,
    consent: bool,
}
fn snapshot(slot: &Slot, token: &str) -> Result<Snapshot, ApiError> {
    slot.lock()
        .map_err(|_| ApiError::internal("support busy"))?
        .as_ref()
        .filter(|s| s.token == token)
        .cloned()
        .ok_or_else(|| ApiError::with_status(StatusCode::CONFLICT, "support-preview-replaced"))
}
async fn export(
    State(state): State<SharedState>,
    Extension(slot): Extension<Slot>,
    Json(b): Json<ExportRequest>,
) -> Result<Response, ApiError> {
    if !b.consent {
        return Err(ApiError::bad_request("support-export-consent-required"));
    }
    let s = snapshot(&slot, &b.token)?;
    fresh(&state, &s)?;
    if hash(&s.report) != b.digest {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "support-preview-changed",
        ));
    }
    let bytes = s.report.bundle().map_err(ApiError::bad_request)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=knxbench-support.zip",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DiagnoseRequest {
    token: String,
    gateway: String,
    consent: bool,
}
async fn diagnose(
    State(state): State<SharedState>,
    Extension(slot): Extension<Slot>,
    Json(b): Json<DiagnoseRequest>,
) -> Result<Json<Preview>, ApiError> {
    state.stop_admission.admit()?;
    if !b.consent {
        return Err(ApiError::bad_request("support-live-consent-required"));
    }
    let mut s = snapshot(&slot, &b.token)?;
    fresh(&state, &s)?;
    let target = s
        .address
        .ok_or_else(|| ApiError::bad_request("support-no-target-address"))?;
    ContactableAddress::new(target).map_err(|_| ApiError::bad_request("support-excluded"))?;
    if knx_core::is_project_excluded(target) {
        return Err(ApiError::bad_request("support-excluded"));
    }
    {
        let p = state
            .project
            .lock()
            .map_err(|_| ApiError::internal("project unavailable"))?;
        if p.as_ref().is_some_and(|p| {
            let count = p
                .devices
                .iter()
                .filter(|d| d.address == Some(target))
                .count();
            count > 1 || (s.device_id.is_some() && count != 1)
        }) {
            return Err(ApiError::bad_request("support-ambiguous-target"));
        }
    }
    let gateway: SocketAddrV4 = b
        .gateway
        .parse()
        .map_err(|_| ApiError::bad_request("support-invalid-gateway"))?;
    let busy = || ApiError::with_status(StatusCode::CONFLICT, "support-gateway-busy");
    let programming = state.address_programming.try_lock().map_err(|_| busy())?;
    if programming
        .as_ref()
        .and_then(crate::tunnel_holder::programming_holder_of)
        .is_some()
        || crate::tunnel_holder::download_holder(&state).is_some()
    {
        return Err(busy());
    }
    let monitor = state.bus_session.lock().await;
    if monitor.is_some() {
        return Err(busy());
    }
    let scan = state.line_scan_session.lock().await;
    if scan
        .as_ref()
        .and_then(crate::tunnel_holder::scan_holder_of)
        .is_some()
    {
        return Err(busy());
    }
    fresh(&state, &s)?;
    {
        let mut held = slot
            .lock()
            .map_err(|_| ApiError::internal("support busy"))?;
        let current = held
            .as_mut()
            .filter(|current| current.token == s.token)
            .ok_or_else(|| {
                ApiError::with_status(StatusCode::CONFLICT, "support-preview-replaced")
            })?;
        current.acquired = false;
    }
    let activity = state
        .one_shot_activity
        .start("supportDiagnostics", Some(target.to_string()));
    let recorder = crate::operation_recordings::start(
        &state,
        knx_app::operation_recording::OperationStart::new(
            knx_app::operation_recording::OperationKind::SupportRead,
        )
        .target(serde_json::json!({
            "individualAddress": target.to_string(),
            "deviceId": s.device_id,
        }))
        .gateway(gateway)
        .related("supportToken", &s.token)
        .related("projectContext", serde_json::json!({
            "serverIncarnation":s.server_incarnation, "projectIncarnation":s.incarnation, "revision":s.revision
        }).to_string())
        .settings(crate::operation_recordings::tunnel_settings(
            knx_net::TunnelAckWait::Standard,
            &state.device_download_timing,
        )),
    );
    let tunnel = match tokio::time::timeout(
        Duration::from_secs(10),
        crate::operation_recordings::connect(
            &state,
            gateway,
            knx_net::TunnelAckWait::Standard,
            &recorder,
        ),
    )
    .await
    {
        Ok(Ok(t)) => t,
        connected => {
            match connected {
                Ok(Err(error)) => crate::operation_recordings::finish_error(
                    &recorder,
                    &error,
                    serde_json::Value::Null,
                ),
                _ => crate::operation_recordings::finish_failed(
                    &recorder,
                    "no tunnel within 10 s",
                    serde_json::Value::Null,
                ),
            }
            activity.finish("failed");
            return Err(ApiError::with_status(
                StatusCode::BAD_GATEWAY,
                "support-gateway-unavailable",
            ));
        }
    };
    // No write authorisation/key/setup action. Overall deadline bounds failed reads.
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        collect(
            &TunnelTransport(tunnel.as_ref()),
            target,
            state.device_download_timing,
            s.mask,
        ),
    )
    .await;
    let _ = tokio::time::timeout(Duration::from_secs(3), tunnel.disconnect()).await;
    match &result {
        Ok(collected) => crate::operation_recordings::finish_result(
            &recorder,
            collected,
            serde_json::json!({ "facts": collected.as_ref().ok().map(|d| d.facts.len()) }),
        ),
        Err(_) => crate::operation_recordings::finish_failed(
            &recorder,
            "the support read exceeded its 30 s deadline",
            serde_json::Value::Null,
        ),
    }
    activity.finish("finished");
    drop(scan);
    drop(monitor);
    drop(programming);
    fresh(&state, &s)?;
    // One snapshot describes the latest returned acquisition, not an unlabelled
    // mixture of observations from different gateways/attempts.
    s.report.facts.retain(|f| f.origin != "live");
    s.report
        .missing_evidence
        .retain(|m| m != "live-diagnostics-not-collected" && m != "reviewed-family-diagnostics");
    s.report
        .missing_evidence
        .push("live-diagnostics-not-collected".into());
    match result {
        Ok(Ok(d)) => {
            s.report.fact(
                "mask",
                "live",
                Some(format!("{:04X}", d.mask)),
                "unavailable",
            );
            s.report.fact(
                "diagnosticProfile",
                "live",
                Some(d.profile.into()),
                "unavailable",
            );
            for f in d.facts {
                s.report.fact(f.field, "live", f.value, f.status);
            }
            s.report
                .missing_evidence
                .retain(|x| x != "live-diagnostics-not-collected");
            if d.profile == "descriptor-only-v1" {
                s.report
                    .missing_evidence
                    .push("reviewed-family-diagnostics".into());
            }
        }
        Ok(Err("identity-mismatch")) => {
            return Err(ApiError::with_status(
                StatusCode::CONFLICT,
                "support-live-identity-mismatch",
            ))
        }
        _ => {
            s.report
                .fact("liveDiagnostics", "live", None, "partial-unavailable");
        }
    }
    let mut guard = slot
        .lock()
        .map_err(|_| ApiError::internal("support busy"))?;
    if guard.as_ref().is_none_or(|v| v.token != s.token) {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "support-preview-replaced",
        ));
    }
    s.acquired = true;
    let result = view(&s);
    *guard = Some(s);
    Ok(Json(result))
}
