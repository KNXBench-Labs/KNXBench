//! `/api/device-download/*`: plan a download to a device, start it on the device's phrase, poll it.
//!
//! "Download" is KNXBench → device over the bus (docs/GLOSSARY.md). ADR-0045
//! fixes the rules this module enforces, in this order:
//!
//! 1. `plan` prepares from the open project and remembers the plan under an
//!    id. Nothing is sent.
//! 2. `start` refuses before any socket opens unless the confirmation phrase
//!    is the one `WriteAuthorisation::for_hardware` demands for the plan's
//!    device, the plan id is the one shown last, no monitor, scan or other
//!    download is running, and the project still yields the identical plan.
//! 3. `status` returns what the run reported since `since`. There is no
//!    cancel (ADR-0045 §5).

use std::net::SocketAddrV4;
use std::sync::atomic::Ordering;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use knx_app::access_key::{choose_key, download_keying, project_access_key, DownloadKeying};
use knx_app::device_download::{
    prepare_device_download, prepare_device_download_for_reading, PrepareError, PreparedDownload,
};
use knx_app::download_report::{prepare_report, DownloadReport};
use knx_app::download_support::{
    download_level, shipped_evidence, untested_acknowledgement, SupportLevel,
};
use knx_core::commissioning::mutation::{required_confirmation_phrase, WriteAuthorisation};
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::{ContactableAddress, IndividualAddress, WriteScope};
use knx_productdb::image::ImageError;

use crate::device_download::{
    BackupDestination, DeviceDownloadSession, DownloadStatus, ProgressEvent,
};

/// Where the backups before each download go, under the data directory.
pub const BACKUP_DIR: &str = "device-backups";
use crate::errors::ApiError;
use crate::SharedState;

pub fn device_download_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/device-download/plan", post(plan))
        .route("/api/device-download/start", post(start))
        .route("/api/device-download/status", get(status))
        .route("/api/device-download/report", get(report))
}

/// The plan shown to the user, kept so `start` can prove it writes the
/// same one.
pub struct ShownPlan {
    pub id: u64,
    pub target: IndividualAddress,
    pub plan: knx_core::commissioning::memory_download::MemoryDownloadPlan,
    /// The partial download the plan was derived as, if any: `start`
    /// derives it again and must arrive at the same steps.
    pub partial: Option<PartialDownloadParts>,
}

/// CP §3.9.2.4's *"Partial Download Type (Parameters and/or Group
/// Addresses)"*. Absent: the complete download.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PartialDto {
    parameters: bool,
    group_addresses: bool,
}

impl From<PartialDto> for PartialDownloadParts {
    fn from(dto: PartialDto) -> Self {
        PartialDownloadParts {
            parameters: dto.parameters,
            group_addresses: dto.group_addresses,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanRequest {
    address: String,
    #[serde(default)]
    partial: Option<PartialDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SegmentDto {
    id: String,
    address: u32,
    size: usize,
    written: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanResponse {
    plan_id: u64,
    address: String,
    device_id: u32,
    device_name: String,
    program_id: String,
    mask_version: u16,
    manufacturer: u16,
    parameter_values: usize,
    group_links: usize,
    segments: Vec<SegmentDto>,
    data_octets: usize,
    steps: Vec<String>,
    /// The phrase `start` demands. It names this device and this scope;
    /// it is not proof that a person read anything (ADR-0045 §2).
    confirmation_phrase: String,
    /// Where the access key comes from; never the key.
    access_key: String,
    /// `true` when this is a CP §3.9.2.4 partial download.
    partial: bool,
    /// Application writes the partial download does not make (CP §3.9.2.4
    /// rule 3: data outside EEPROM), as `[address, octets]`.
    not_written: Vec<(u16, usize)>,
    /// How far this download is supported: verified on hardware, or
    /// untested (the plan is complete, no hardware has confirmed it).
    support: SupportDto,
    /// The phrase `start` also demands, as `acceptUntested`, when
    /// `support.level` is `untested`; `None` otherwise.
    untested_acknowledgement: Option<String>,
    /// What the user should know before writing that does not stop the
    /// write (ADR-0120), each as `rule: detail (reference)`; empty when
    /// there is nothing.
    warnings: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SupportDto {
    /// `verified` or `untested` (an unsupported download has no plan).
    level: &'static str,
    /// For `verified`: where the hardware run is documented.
    evidence: Option<String>,
    /// The inferences the plan rests on (ADR-0086), each as
    /// `rule: detail (reference)`; empty when there are none.
    inferences: Vec<String>,
}

/// The support level of `prepared`, against the evidence this build ships.
fn support_of(prepared: &PreparedDownload) -> Result<SupportLevel, ApiError> {
    let evidence = shipped_evidence()
        .map_err(|e| ApiError::with_status(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(download_level(
        &prepared.request.program_id,
        prepared.partial.as_ref().map(|(parts, _)| *parts),
        &prepared.plan,
        &prepared.inferences,
        &evidence,
    ))
}

pub(crate) fn parse_target(address: &str) -> Result<IndividualAddress, ApiError> {
    let parsed: IndividualAddress = address.parse().map_err(|e| {
        ApiError::with_kind(
            StatusCode::BAD_REQUEST,
            "downloadInvalidAddress",
            format!("invalid device address {address:?}: {e}"),
        )
    })?;
    ContactableAddress::new(parsed).map_err(|e| {
        ApiError::with_kind(StatusCode::BAD_REQUEST, "downloadExcluded", e.to_string())
    })?;
    Ok(parsed)
}

/// The keying for `prepared`, from the open project's `Installation/@BCUKey`.
///
/// The HTTP API takes no key: a key in a request body is a key in a
/// browser's memory and a proxy's log. A device locked with a key the
/// project does not carry is downloaded through the CLI's `--key-file`.
fn keying(state: &SharedState, prepared: &PreparedDownload) -> Result<DownloadKeying, ApiError> {
    let opaque = state.opaque.lock().expect("state mutex poisoned");
    let project_key = project_access_key(&opaque).map_err(|e| {
        ApiError::with_kind(
            StatusCode::UNPROCESSABLE_ENTITY,
            "downloadKeyRefused",
            format!("no download to device {} prepared: {e}", prepared.target),
        )
    })?;
    let (key, source) = choose_key(None, project_key);
    Ok(download_keying(prepared.plan.mask, key, source))
}

/// A refusal to prepare, as `422`. The KL-170 byte-order refusal is
/// `kind: "unsupportedByteOrder"` with the declared `order` and the
/// `address` (issue #10), so a client can explain it; the text stays.
pub(crate) fn prepare_error(target: IndividualAddress, error: &PrepareError) -> ApiError {
    let message = format!("no download to device {target} prepared: {error}");
    match error {
        PrepareError::Image(ImageError::UnsupportedByteOrder { order }) => {
            let mut details = serde_json::Map::new();
            details.insert("address".into(), target.to_string().into());
            details.insert("order".into(), order.clone().into());
            details.insert("category".into(), "byte-order".into());
            ApiError::with_details(
                StatusCode::UNPROCESSABLE_ENTITY,
                "unsupportedByteOrder",
                message,
                details,
            )
        }
        _ => {
            let mut details = serde_json::Map::new();
            details.insert(
                "category".into(),
                knx_app::project_readiness::prepare_category(error)
                    .code()
                    .into(),
            );
            ApiError::with_details(
                StatusCode::UNPROCESSABLE_ENTITY,
                "downloadPreparationRefused",
                message,
                details,
            )
        }
    }
}

/// The refusal of a download step without a product database: names the
/// startup reason (for example a newer schema) when one is known.
pub(crate) fn no_application_program(state: &crate::AppState) -> String {
    format!(
        "{}, so no application program can be read",
        state.product_db_missing_or("no product database is configured")
    )
}

/// Prepares a download from the project as it is now. Both locks are
/// released before this returns; they are never held together. Refuses what
/// no download may write (KL-006), as the CLI and readiness do.
pub(crate) fn prepare(
    state: &SharedState,
    target: IndividualAddress,
    partial: Option<PartialDownloadParts>,
) -> Result<PreparedDownload, ApiError> {
    prepare_with(state, target, partial, prepare_device_download)
}

/// [`prepare`]'s plan, to read the device only (`/api/device-compare`):
/// a program refused for every download (KL-006) is still compared.
pub(crate) fn prepare_for_reading(
    state: &SharedState,
    target: IndividualAddress,
    partial: Option<PartialDownloadParts>,
) -> Result<PreparedDownload, ApiError> {
    prepare_with(state, target, partial, prepare_device_download_for_reading)
}

type Preparation = fn(
    &knx_productdb::Connection,
    &knx_core::Project,
    IndividualAddress,
) -> Result<PreparedDownload, PrepareError>;

fn prepare_with(
    state: &SharedState,
    target: IndividualAddress,
    partial: Option<PartialDownloadParts>,
    preparation: Preparation,
) -> Result<PreparedDownload, ApiError> {
    let Some(owner) = state.product_db.as_ref() else {
        return Err(ApiError::with_kind(
            StatusCode::CONFLICT,
            "downloadProductDataMissing",
            no_application_program(state),
        ));
    };
    let project_guard = state.project.lock().expect("state mutex poisoned");
    let Some(project) = project_guard.as_ref() else {
        return Err(ApiError::with_kind(
            StatusCode::CONFLICT,
            "downloadProjectMissing",
            "no project is open",
        ));
    };
    let prepared = owner
        .with_read(|products| preparation(products, project, target))
        .map_err(crate::catalogue_owner::owner_error)?
        .map_err(|e| prepare_error(target, &e))?;
    match partial {
        None => Ok(prepared),
        Some(parts) => prepared.into_partial(parts).map_err(|e| {
            ApiError::with_kind(
                StatusCode::UNPROCESSABLE_ENTITY,
                "downloadPartialRefused",
                format!("no partial download to device {target} prepared: {e}"),
            )
        }),
    }
}

async fn plan(
    State(state): State<SharedState>,
    Json(body): Json<PlanRequest>,
) -> Result<Json<PlanResponse>, ApiError> {
    let target = parse_target(&body.address)?;
    let partial = body.partial.map(PartialDownloadParts::from);
    let prepared = prepare(&state, target, partial)?;
    let keying = keying(&state, &prepared)?;
    let level = support_of(&prepared)?;
    let id = state.next_device_download_id.fetch_add(1, Ordering::SeqCst);
    let response = PlanResponse {
        plan_id: id,
        address: target.to_string(),
        device_id: prepared.device.0,
        device_name: prepared.device_name.clone(),
        program_id: prepared.request.program_id.clone(),
        mask_version: prepared.plan.mask.0,
        manufacturer: prepared.plan.manufacturer,
        parameter_values: prepared.request.values.len(),
        group_links: prepared.request.links.len(),
        segments: prepared
            .octets_to_write()
            .into_iter()
            .map(|(id, address, size, written)| SegmentDto {
                id,
                address,
                size,
                written,
            })
            .collect(),
        data_octets: prepared.plan.data_octets(),
        steps: prepared
            .plan
            .steps
            .iter()
            .map(ToString::to_string)
            .collect(),
        confirmation_phrase: required_confirmation_phrase(target, WriteScope::Download),
        access_key: keying.source.to_string(),
        partial: prepared.partial.is_some(),
        not_written: prepared
            .partial
            .as_ref()
            .map(|(_, ignored)| ignored.clone())
            .unwrap_or_default(),
        support: SupportDto {
            level: level.code(),
            evidence: match &level {
                SupportLevel::Verified { evidence, .. } => Some(format!(
                    "{}, {}; {}",
                    evidence.device, evidence.date, evidence.reference
                )),
                _ => None,
            },
            inferences: match &level {
                SupportLevel::Untested { inferences, .. } => {
                    inferences.iter().map(ToString::to_string).collect()
                }
                _ => Vec::new(),
            },
        },
        untested_acknowledgement: level
            .needs_acknowledgement()
            .then(|| untested_acknowledgement(target)),
        warnings: prepared.warnings.iter().map(ToString::to_string).collect(),
    };
    *state
        .device_download_plan
        .lock()
        .expect("state mutex poisoned") = Some(ShownPlan {
        id,
        target,
        plan: prepared.plan,
        partial,
    });
    Ok(Json(response))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartRequest {
    plan_id: u64,
    gateway: String,
    confirmation: String,
    /// The plan's `untestedAcknowledgement`, when it has one.
    #[serde(default)]
    accept_untested: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartResponse {
    download_id: u64,
}

fn conflict(message: impl Into<String>) -> ApiError {
    ApiError::conflict("downloadPlanStale", message)
}

async fn start(
    State(state): State<SharedState>,
    Json(body): Json<StartRequest>,
) -> Result<Json<StartResponse>, ApiError> {
    // Goal A3: no new tunnel once a stop has begun (KL-163).
    state.stop_admission.admit()?;
    let gateway: SocketAddrV4 = body.gateway.parse().map_err(|_| {
        ApiError::with_kind(
            StatusCode::BAD_REQUEST,
            "downloadGatewayInvalid",
            "gateway is not a host:port IPv4 address",
        )
    })?;
    let (target, partial) = {
        let shown = state
            .device_download_plan
            .lock()
            .expect("state mutex poisoned");
        match shown.as_ref() {
            Some(shown) if shown.id == body.plan_id => (shown.target, shown.partial),
            _ => {
                return Err(conflict(
                    "that plan is no longer the one shown; ask for the plan again",
                ))
            }
        }
    };
    let authorisation =
        WriteAuthorisation::for_hardware(target, WriteScope::Download, &body.confirmation)
            .map_err(|e| {
                ApiError::with_kind(
                    StatusCode::BAD_REQUEST,
                    "downloadConfirmationRefused",
                    format!("not written: {e}"),
                )
            })?;

    // Held until the run is registered: a second start waits here and
    // then sees the first one running.
    let mut download = state.device_download.lock().await;
    if let Some(existing) = download.as_ref() {
        if let Some(holder) = crate::tunnel_holder::download_holder_of(existing) {
            return Err(crate::tunnel_holder::gateway_busy(
                holder,
                format!(
                    "a download to device {} is already running",
                    existing.target()
                ),
            ));
        }
    }
    // Held until the download is registered, in the order download →
    // address programming → monitor → scan. The others take their own lock
    // first and then only `try_lock` this one, so none can start between
    // these checks and the registration, and no wait can form a cycle.
    let programming = state.address_programming.lock().await;
    if let Some(holder) = programming
        .as_ref()
        .and_then(crate::tunnel_holder::programming_holder_of)
    {
        return Err(crate::tunnel_holder::gateway_busy(
            holder,
            "an individual-address programming is running: the gateway serves one tunnel",
        ));
    }
    let monitor = state.bus_session.lock().await;
    if let Some(existing) = monitor.as_ref() {
        return Err(crate::tunnel_holder::gateway_busy(
            crate::tunnel_holder::monitor_holder_of(existing),
            "stop the bus monitor first: the gateway serves one tunnel",
        ));
    }
    let scan = state.line_scan_session.lock().await;
    if let Some(holder) = scan.as_ref().and_then(crate::tunnel_holder::scan_holder_of) {
        return Err(crate::tunnel_holder::gateway_busy(
            holder,
            "a line scan is running: the gateway serves one tunnel",
        ));
    }

    // Before `prepare`: a project opened after this read can only make the
    // settlement skip, never land on the wrong project (ADR-0129).
    let incarnation = crate::programming_state::incarnation(&state);
    let prepared = prepare(&state, target, partial)?;
    let keying = keying(&state, &prepared)?;
    // Recomputed from the plan that will run, not trusted from the shown
    // one: the evidence is the build's, the plan is checked equal below.
    let level = support_of(&prepared)?;
    if level.needs_acknowledgement() {
        let expected = untested_acknowledgement(target);
        if body.accept_untested.as_deref() != Some(expected.as_str()) {
            return Err(ApiError::with_kind(
                StatusCode::BAD_REQUEST,
                "downloadConfirmationRefused",
                format!(
                    "not written: this download is untested on hardware; acceptUntested must read \
                 exactly {expected:?}"
                ),
            ));
        }
    }
    {
        let shown = state
            .device_download_plan
            .lock()
            .expect("state mutex poisoned");
        let unchanged = shown
            .as_ref()
            .is_some_and(|shown| shown.id == body.plan_id && shown.plan == prepared.plan);
        if !unchanged {
            return Err(conflict(
                "the project changed since the plan was shown; nothing was sent, ask for the plan again",
            ));
        }
    }

    let report = {
        let Some(owner) = state.product_db.as_ref() else {
            return Err(ApiError::with_status(
                StatusCode::CONFLICT,
                no_application_program(&state),
            ));
        };
        owner
            .with_read(|products| {
                prepare_report(products, &prepared, &level, &crate::version_string())
            })
            .map_err(crate::catalogue_owner::owner_error)?
            .map_err(|e| {
                ApiError::with_status(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("not sent: the download report could not be prepared: {e}"),
                )
            })?
    };
    let activity = state
        .one_shot_activity
        .start_download(body.plan_id, target)
        .map_err(|_| {
            ApiError::with_status(
                StatusCode::SERVICE_UNAVAILABLE,
                "activity history unavailable; not sent",
            )
        })?;
    let tunnel = match state.connector.connect_tunnel(gateway).await {
        Ok(tunnel) => tunnel,
        Err(e) => {
            // No tunnel, no telegram: say so in the history rather than
            // leaving it to the guard's drop, which records `unknown`
            // (AR18 review M6). If even that cannot be recorded, the drop
            // still does.
            let _ = activity.record_never_connected();
            return Err(crate::tunnel_holder::gateway_connect_failed(gateway, &e));
        }
    };
    let id = body.plan_id;
    let backups = BackupDestination {
        dir: state.data_dir.join(BACKUP_DIR),
        taken: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
    };
    let settler = crate::programming_state::download_settler(&state, incarnation, &prepared);
    *download = Some(DeviceDownloadSession::start(
        id,
        tunnel,
        authorisation,
        keying,
        state.device_download_timing,
        prepared,
        backups,
        activity,
        report,
        settler,
    ));
    drop(scan);
    drop(monitor);
    drop(programming);
    // A plan is written at most once.
    *state
        .device_download_plan
        .lock()
        .expect("state mutex poisoned") = None;
    Ok(Json(StartResponse { download_id: id }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportQuery {
    download_id: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportResponse {
    /// The report file's contents (ADR-0121).
    report: DownloadReport,
    /// A file name for it.
    file_name: String,
    /// A short text for an issue or mail; the values stay in the file.
    summary: String,
    /// The issue form with program, version and outcome filled in.
    issue_url: String,
    /// A mail to the report mailbox with the summary as body.
    mailto: String,
}

/// The tester's report of the last download, once it has ended.
async fn report(
    State(state): State<SharedState>,
    Query(query): Query<ReportQuery>,
) -> Result<Json<ReportResponse>, ApiError> {
    let Ok(guard) = state.device_download.try_lock() else {
        return Err(conflict("a download is being started"));
    };
    let Some(session) = guard.as_ref() else {
        return Err(ApiError::with_status(
            StatusCode::NOT_FOUND,
            "no download to a device yet",
        ));
    };
    if query.download_id.is_some_and(|id| id != session.id()) {
        return Err(ApiError::with_status(
            StatusCode::NOT_FOUND,
            format!(
                "download {} has been replaced",
                query.download_id.unwrap_or_default()
            ),
        ));
    }
    let Some(report) = session.report() else {
        return Err(conflict("the download has not ended yet"));
    };
    Ok(Json(ReportResponse {
        file_name: report.file_name(),
        summary: report.summary(),
        issue_url: report.issue_url(),
        mailto: report.mailto(),
        report,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusQuery {
    #[serde(default)]
    since: usize,
    download_id: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusResponse {
    download_id: u64,
    address: String,
    device_name: String,
    steps: usize,
    data_octets: usize,
    status: DownloadStatus,
    next_since: usize,
    events: Vec<ProgressEvent>,
    /// The backup taken before the first write, relative to the server's
    /// data directory, once it is on disk.
    backup_file: Option<String>,
}

async fn status(
    State(state): State<SharedState>,
    Query(query): Query<StatusQuery>,
) -> Result<Json<StatusResponse>, ApiError> {
    // `try_lock`: a start holds this lock while it connects, and a poll
    // must not queue behind a gateway time-out.
    let Ok(guard) = state.device_download.try_lock() else {
        return Err(conflict("a download is being started"));
    };
    let Some(session) = guard.as_ref() else {
        return Err(ApiError::with_status(
            StatusCode::NOT_FOUND,
            "no download to a device yet",
        ));
    };
    if query.download_id.is_some_and(|id| id != session.id()) {
        return Err(ApiError::with_status(
            StatusCode::NOT_FOUND,
            format!(
                "download {} has been replaced",
                query.download_id.unwrap_or_default()
            ),
        ));
    }
    let (status, next_since, events) = session.snapshot_since(query.since);
    Ok(Json(StatusResponse {
        download_id: session.id(),
        address: session.target().to_string(),
        device_name: session.device_name().to_string(),
        steps: session.steps(),
        data_octets: session.data_octets(),
        status,
        next_since,
        events,
        backup_file: session.backup_file().map(|path| {
            path.strip_prefix(&state.data_dir)
                .unwrap_or(&path)
                .display()
                .to_string()
        }),
    }))
}

#[cfg(test)]
mod prepare_error_tests {
    use super::*;
    use axum::response::IntoResponse;
    use knx_app::device_download::PrepareError;
    use knx_productdb::image::ImageError;

    async fn body(error: ApiError) -> (StatusCode, serde_json::Value) {
        let response = error.into_response();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    /// Issue #10 (3b): the deliberate KL-170 refusal stays, but as a code a
    /// client explains, with the declared order and the target; the text is
    /// unchanged.
    #[tokio::test]
    async fn a_byte_order_refusal_is_a_structured_422() {
        let target: IndividualAddress = "0.2.23".parse().unwrap();
        let error = PrepareError::Image(ImageError::UnsupportedByteOrder {
            order: "LittleEndian".into(),
        });
        let (status, body) = body(prepare_error(target, &error)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["kind"], "unsupportedByteOrder", "{body}");
        assert_eq!(body["order"], "LittleEndian", "{body}");
        assert_eq!(body["category"], "byte-order", "{body}");
        assert_eq!(body["address"], "0.2.23", "{body}");
        assert_eq!(
            body["error"],
            "no download to device 0.2.23 prepared: memory image: not supported for a download: \
             ParameterByteOrder \"LittleEndian\": numbers are written high octet first"
        );
    }

    #[tokio::test]
    async fn procedure_and_module_refusals_use_the_shared_typed_category() {
        let target: IndividualAddress = "1.1.5".parse().unwrap();
        for (error, category) in [
            (
                PrepareError::Plan(
                    knx_productdb::download_plan::DownloadPlanError::ProcedureCount(2),
                ),
                "procedure-shape",
            ),
            (
                PrepareError::Image(ImageError::Unsupported {
                    what: "parameter in a module instance".into(),
                }),
                "modules",
            ),
            (
                PrepareError::Plan(
                    knx_productdb::download_plan::DownloadPlanError::VendorDownloadPlugin(
                        "{G}".into(),
                    ),
                ),
                "vendor-download-plugin",
            ),
        ] {
            let expected_detail = format!("no download to device {target} prepared: {error}");
            let (status, result) = body(prepare_error(target, &error)).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(result["kind"], "downloadPreparationRefused");
            assert_eq!(result["category"], category);
            assert_eq!(result["error"], expected_detail);
        }
    }

    #[tokio::test]
    async fn every_other_refusal_retains_its_text_and_adds_the_preparation_category() {
        let target: IndividualAddress = "1.1.5".parse().unwrap();
        let (status, body) = body(prepare_error(target, &PrepareError::NoDeviceAt(target))).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            body,
            serde_json::json!({ "kind": "downloadPreparationRefused", "category": "configuration", "error": "no download to device 1.1.5 prepared: no device in the project has address 1.1.5" })
        );
    }
}

#[cfg(test)]
mod report_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn there_is_no_report_before_any_download() {
        let state: SharedState = std::sync::Arc::new(crate::AppState::default());
        let response = device_download_routes()
            .with_state(state)
            .oneshot(
                Request::get("/api/device-download/report")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
