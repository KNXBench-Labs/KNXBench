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
use knx_app::device_download::{prepare_device_download, PreparedDownload};
use knx_app::download_support::{
    download_level, shipped_evidence, untested_acknowledgement, SupportLevel,
};
use knx_core::commissioning::mutation::{required_confirmation_phrase, WriteAuthorisation};
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::{ContactableAddress, IndividualAddress, WriteScope};

use crate::bus_scan::LineScanStatus;
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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SupportDto {
    /// `verified` or `untested` (an unsupported download has no plan).
    level: &'static str,
    /// For `verified`: where the hardware run is documented.
    evidence: Option<String>,
}

/// The support level of `prepared`, against the evidence this build ships.
fn support_of(prepared: &PreparedDownload) -> Result<SupportLevel, ApiError> {
    let evidence = shipped_evidence()
        .map_err(|e| ApiError::with_status(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(download_level(
        &prepared.request.program_id,
        prepared.partial.as_ref().map(|(parts, _)| *parts),
        &prepared.plan,
        &evidence,
    ))
}

pub(crate) fn parse_target(address: &str) -> Result<IndividualAddress, ApiError> {
    let parsed: IndividualAddress = address
        .parse()
        .map_err(|e| ApiError::bad_request(format!("invalid device address {address:?}: {e}")))?;
    ContactableAddress::new(parsed).map_err(|e| ApiError::bad_request(e.to_string()))?;
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
        ApiError::with_status(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("no download to device {} prepared: {e}", prepared.target),
        )
    })?;
    let (key, source) = choose_key(None, project_key);
    Ok(download_keying(prepared.plan.mask, key, source))
}

/// Prepares from the project as it is now. Both locks are released before
/// this returns; they are never held together.
pub(crate) fn prepare(
    state: &SharedState,
    target: IndividualAddress,
    partial: Option<PartialDownloadParts>,
) -> Result<PreparedDownload, ApiError> {
    let Some(products) = state.product_db.as_ref() else {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "no product database is configured, so no application program can be read",
        ));
    };
    let project_guard = state.project.lock().expect("state mutex poisoned");
    let Some(project) = project_guard.as_ref() else {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "no project is open",
        ));
    };
    let products = products.lock().expect("state mutex poisoned");
    let prepared = prepare_device_download(&products, project, target).map_err(|e| {
        ApiError::with_status(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("no download to device {target} prepared: {e}"),
        )
    })?;
    match partial {
        None => Ok(prepared),
        Some(parts) => prepared.into_partial(parts).map_err(|e| {
            ApiError::with_status(
                StatusCode::UNPROCESSABLE_ENTITY,
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
        },
        untested_acknowledgement: level
            .needs_acknowledgement()
            .then(|| untested_acknowledgement(target)),
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
    ApiError::with_status(StatusCode::CONFLICT, message)
}

async fn start(
    State(state): State<SharedState>,
    Json(body): Json<StartRequest>,
) -> Result<Json<StartResponse>, ApiError> {
    let gateway: SocketAddrV4 = body
        .gateway
        .parse()
        .map_err(|_| ApiError::bad_request("gateway is not a host:port IPv4 address"))?;
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
            .map_err(|e| ApiError::bad_request(format!("not written: {e}")))?;

    // Held until the run is registered: a second start waits here and
    // then sees the first one running.
    let mut download = state.device_download.lock().await;
    if let Some(existing) = download.as_ref() {
        if existing.is_running() {
            return Err(conflict(format!(
                "a download to device {} is already running",
                existing.target()
            )));
        }
    }
    // Held until the download is registered, in the order download →
    // address programming → monitor → scan. The others take their own lock
    // first and then only `try_lock` this one, so none can start between
    // these checks and the registration, and no wait can form a cycle.
    let programming = state.address_programming.lock().await;
    if programming
        .as_ref()
        .is_some_and(crate::AddressProgrammingSession::is_active)
    {
        return Err(conflict(
            "an individual-address programming is running: the gateway serves one tunnel",
        ));
    }
    let monitor = state.bus_session.lock().await;
    if monitor.is_some() {
        return Err(conflict(
            "stop the bus monitor first: the gateway serves one tunnel",
        ));
    }
    let scan = state.line_scan_session.lock().await;
    if matches!(
        scan.as_ref().map(|scan| scan.status()),
        Some(LineScanStatus::Running)
    ) {
        return Err(conflict(
            "a line scan is running: the gateway serves one tunnel",
        ));
    }

    let prepared = prepare(&state, target, partial)?;
    let keying = keying(&state, &prepared)?;
    // Recomputed from the plan that will run, not trusted from the shown
    // one: the evidence is the build's, the plan is checked equal below.
    if support_of(&prepared)?.needs_acknowledgement() {
        let expected = untested_acknowledgement(target);
        if body.accept_untested.as_deref() != Some(expected.as_str()) {
            return Err(ApiError::bad_request(format!(
                "not written: this download is untested on hardware; acceptUntested must read \
                 exactly {expected:?}"
            )));
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

    let activity = state
        .one_shot_activity
        .start_download(body.plan_id, target)
        .map_err(|_| {
            ApiError::with_status(
                StatusCode::SERVICE_UNAVAILABLE,
                "activity history unavailable; not sent",
            )
        })?;
    let tunnel = state
        .connector
        .connect_tunnel(gateway)
        .await
        .map_err(|e| ApiError::with_status(StatusCode::BAD_GATEWAY, e.to_string()))?;
    let id = body.plan_id;
    let backups = BackupDestination {
        dir: state.data_dir.join(BACKUP_DIR),
        taken: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
    };
    *download = Some(DeviceDownloadSession::start(
        id,
        tunnel,
        authorisation,
        keying,
        state.device_download_timing,
        prepared,
        backups,
        activity,
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

/// For the monitor and scan starts: whether a download to a device is
/// starting or running. Never waits.
pub fn download_in_progress(state: &SharedState) -> bool {
    match state.device_download.try_lock() {
        Err(_) => true,
        Ok(guard) => guard
            .as_ref()
            .is_some_and(DeviceDownloadSession::is_running),
    }
}
