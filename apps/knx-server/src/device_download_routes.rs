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
//!
//! `plan` with `scope: "changes"` resolves the scope from the device's
//! programming flags ([`knx_app::download_scope`]) and shows the plan of that
//! concrete scope. `start` reads the flags again in the project snapshot it
//! prepares the plan from, and refuses once they advise another scope.

use std::net::SocketAddrV4;
use std::sync::atomic::Ordering;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use knx_app::access_key::{choose_key, download_keying, project_access_key, DownloadKeying};
use knx_app::commissioning_activity::DownloadPreparation;
use knx_app::device_download::{
    prepare_device_download, prepare_device_download_for_reading, PrepareError, PreparedDownload,
};
use knx_app::download_report::{prepare_report, DownloadReport};
use knx_app::download_scope::{
    prepare_scoped_download, resolve_scope, ScopeRequest, ScopedDownload, ScopedPrepareError,
};
use knx_app::download_support::{
    download_level, shipped_evidence, untested_acknowledgement, SupportLevel,
};
use knx_app::operation_recording::{OperationKind, OperationStart};
use knx_core::commissioning::memory_download::MemoryDownloadPlan;
use knx_core::commissioning::mutation::{required_confirmation_phrase, WriteAuthorisation};
use knx_core::commissioning::partial_memory_download::{PartialDownloadParts, PartialPlanError};
use knx_core::commissioning::scope_advice::{AdvisedWrite, ScopeAdvice};
use knx_core::{ContactableAddress, IndividualAddress, WriteScope};
use knx_net::TunnelAckWait;
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
    /// `true` when `partial` was resolved from `scope: "changes"`: `start`
    /// also refuses unless the flags, read in the snapshot it prepares from,
    /// still advise this scope.
    pub changes: bool,
}

/// The shown-plan slot: one single plan or one batch (ADR-0172 §2). Each
/// replaces the other, so a stale single plan never starts beside a batch.
pub enum ShownDownload {
    Single(ShownPlan),
    Batch(crate::device_download_batch::ShownBatch),
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
    /// `"changes"`: the scope the device's programming flags advise.
    /// Absent: `partial` decides, as before.
    #[serde(default)]
    scope: Option<ScopeDto>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ScopeDto {
    Changes,
}

/// [`ScopeAdvice`] on the wire: the project's flags, never device evidence.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScopeAdviceDto {
    /// Always `projectFlags`.
    basis: &'static str,
    /// `complete`, `parameters`, `groupAddresses`, `both` or `nothing`.
    write: &'static str,
    /// `current`, `pending` or `missing`; no memory download writes it.
    address: &'static str,
    /// No KNXBench download writes the medium configuration.
    medium_pending: bool,
    fully_programmed: bool,
}

impl From<&ScopeAdvice> for ScopeAdviceDto {
    fn from(advice: &ScopeAdvice) -> Self {
        Self {
            basis: "projectFlags",
            write: advice.write.code(),
            address: advice.address.code(),
            medium_pending: advice.medium_pending,
            fully_programmed: advice.fully_programmed(),
        }
    }
}

/// The scope `partial` writes, by the advice's names.
fn scope_code(partial: Option<PartialDownloadParts>) -> &'static str {
    partial
        .map_or(AdvisedWrite::Complete, AdvisedWrite::Partial)
        .code()
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
    /// The concrete scope this plan writes: `complete`, `parameters`,
    /// `groupAddresses` or `both`.
    scope: &'static str,
    /// `true` when the request asked for `scope: "changes"`.
    scope_from_changes: bool,
    /// What the device's programming flags advise, for every request.
    scope_advice: ScopeAdviceDto,
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
pub(crate) fn support_of(prepared: &PreparedDownload) -> Result<SupportLevel, ApiError> {
    let evidence = shipped_evidence()
        .map_err(|e| ApiError::with_status(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(download_level(prepared, &evidence))
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
pub(crate) fn keying(
    state: &SharedState,
    prepared: &PreparedDownload,
) -> Result<DownloadKeying, ApiError> {
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
        Some(parts) => prepared
            .into_partial(parts)
            .map_err(|e| partial_refused(target, &e)),
    }
}

fn partial_refused(target: IndividualAddress, error: &PartialPlanError) -> ApiError {
    ApiError::with_kind(
        StatusCode::UNPROCESSABLE_ENTITY,
        "downloadPartialRefused",
        format!("no partial download to device {target} prepared: {error}"),
    )
}

/// [`prepare`] for a [`ScopeRequest`]: resolved and prepared under one
/// project lock, so the advice and the plan come from one project state.
/// Nothing to write needs no product data, and opens none.
fn prepare_scoped(
    state: &SharedState,
    target: IndividualAddress,
    request: ScopeRequest,
) -> Result<ScopedDownload, ApiError> {
    let products_missing = || {
        ApiError::with_kind(
            StatusCode::CONFLICT,
            "downloadProductDataMissing",
            no_application_program(state),
        )
    };
    let project_guard = state.project.lock().expect("state mutex poisoned");
    let Some(project) = project_guard.as_ref() else {
        // Missing product data still answers first, as in `prepare`.
        return Err(match state.product_db {
            None => products_missing(),
            Some(_) => project_missing(),
        });
    };
    let resolved =
        resolve_scope(project, target, request).map_err(|e| prepare_error(target, &e))?;
    if resolved.resolved.is_none() {
        return Ok(ScopedDownload {
            advice: resolved.advice,
            resolved: None,
            prepared: None,
        });
    }
    let Some(owner) = state.product_db.as_ref() else {
        return Err(products_missing());
    };
    owner
        .with_read(|products| prepare_scoped_download(products, project, target, request))
        .map_err(crate::catalogue_owner::owner_error)?
        .map_err(|error| match error {
            ScopedPrepareError::Prepare(e) => prepare_error(target, &e),
            ScopedPrepareError::Partial(e) => partial_refused(target, &e),
        })
}

/// The download a shown plan writes, prepared again for `start`. A plan from
/// `scope: "changes"` is resolved again and prepared under one project lock,
/// so the flags read are those of the plan prepared; nothing pending, or
/// another scope, is a stale plan and no write at all.
fn prepare_shown(
    state: &SharedState,
    target: IndividualAddress,
    partial: Option<PartialDownloadParts>,
    changes: bool,
) -> Result<PreparedDownload, ApiError> {
    if !changes {
        return prepare(state, target, partial);
    }
    let scoped = prepare_scoped(state, target, ScopeRequest::Changes)?;
    match scoped.prepared {
        Some(prepared) if scoped.resolved == Some(partial) => Ok(prepared),
        _ => Err(conflict(
            "the pending parts changed since the plan was shown; nothing was sent, ask for the \
             plan again",
        )),
    }
}

fn project_missing() -> ApiError {
    ApiError::with_kind(
        StatusCode::CONFLICT,
        "downloadProjectMissing",
        "no project is open",
    )
}

/// The request's scope; `changes` beside an explicit `partial` is refused.
fn scope_request(body: &PlanRequest) -> Result<ScopeRequest, ApiError> {
    match (body.scope, body.partial) {
        (Some(ScopeDto::Changes), Some(_)) => Err(ApiError::with_kind(
            StatusCode::BAD_REQUEST,
            "downloadScopeConflict",
            "scope \"changes\" and an explicit partial exclude each other; send one",
        )),
        (Some(ScopeDto::Changes), None) => Ok(ScopeRequest::Changes),
        (None, Some(parts)) => Ok(ScopeRequest::Partial(parts.into())),
        (None, None) => Ok(ScopeRequest::Complete),
    }
}

async fn plan(
    State(state): State<SharedState>,
    Json(body): Json<PlanRequest>,
) -> Result<Json<PlanResponse>, ApiError> {
    let target = parse_target(&body.address)?;
    let request = scope_request(&body)?;
    let scoped = prepare_scoped(&state, target, request)?;
    let advice = ScopeAdviceDto::from(&scoped.advice);
    let Some(prepared) = scoped.prepared else {
        // No plan is the shown plan now: one shown earlier cannot be
        // started after this answer.
        *state
            .device_download_plan
            .lock()
            .expect("state mutex poisoned") = None;
        let mut details = serde_json::Map::new();
        details.insert(
            "scopeAdvice".into(),
            serde_json::to_value(&advice).expect("advice serialises"),
        );
        return Err(ApiError::with_details(
            StatusCode::UNPROCESSABLE_ENTITY,
            "downloadNothingPending",
            format!(
                "nothing to download to device {target}: the project marks no application, \
                 parameter or group address part pending; no plan was built"
            ),
            details,
        ));
    };
    let partial = prepared.partial.as_ref().map(|(parts, _)| *parts);
    let changes = request == ScopeRequest::Changes;
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
        scope: scope_code(partial),
        scope_from_changes: changes,
        scope_advice: advice,
    };
    *state
        .device_download_plan
        .lock()
        .expect("state mutex poisoned") = Some(ShownDownload::Single(ShownPlan {
        id,
        target,
        plan: prepared.plan,
        partial,
        changes,
    }));
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

pub(crate) fn conflict(message: impl Into<String>) -> ApiError {
    ApiError::conflict("downloadPlanStale", message)
}

/// The gateway of a start request, as `host:port` IPv4.
pub(crate) fn parse_gateway(gateway: &str) -> Result<SocketAddrV4, ApiError> {
    gateway.parse().map_err(|_| {
        ApiError::with_kind(
            StatusCode::BAD_REQUEST,
            "downloadGatewayInvalid",
            "gateway is not a host:port IPv4 address",
        )
    })
}

/// The device's confirmation phrase checked, as the authorisation it grants.
pub(crate) fn authorise(
    target: IndividualAddress,
    confirmation: &str,
) -> Result<WriteAuthorisation, ApiError> {
    WriteAuthorisation::for_hardware(target, WriteScope::Download, confirmation).map_err(|e| {
        ApiError::with_kind(
            StatusCode::BAD_REQUEST,
            "downloadConfirmationRefused",
            format!("not written: {e}"),
        )
    })
}

/// The refusal of a missing or wrong `untested` acknowledgement.
pub(crate) fn untested_refused(expected: &str) -> ApiError {
    ApiError::with_kind(
        StatusCode::BAD_REQUEST,
        "downloadConfirmationRefused",
        format!(
            "not written: this download is untested on hardware; acceptUntested must read \
             exactly {expected:?}"
        ),
    )
}

/// The gateway's one tunnel, reserved for a download start until its session
/// is installed: download → address programming → monitor → line scan.
pub(crate) struct TunnelLocks<'a> {
    download: tokio::sync::MutexGuard<'a, Option<DeviceDownloadSession>>,
    programming: tokio::sync::MutexGuard<'a, Option<crate::AddressProgrammingSession>>,
    monitor: tokio::sync::MutexGuard<'a, Option<crate::BusSession>>,
    scan: tokio::sync::MutexGuard<'a, Option<crate::bus_scan::LineScanSession>>,
}

/// Takes the four tunnel locks in their fixed order and refuses while any
/// holder runs. `own_batch` is the batch this start belongs to: it holds the
/// reservation between its devices and is not busy for itself.
///
/// The others take their own lock first and then only `try_lock` the
/// download one, so none can start between these checks and the
/// registration, and no wait can form a cycle.
pub(crate) async fn lock_tunnel(
    state: &SharedState,
    own_batch: Option<u64>,
) -> Result<TunnelLocks<'_>, ApiError> {
    // A running batch is refused before the wait below: its device's start
    // holds the download lock across a gateway connect.
    if let Some((holder, message)) = crate::device_download_batch::busy_except(state, own_batch) {
        return Err(crate::tunnel_holder::gateway_busy(holder, message));
    }
    // Held until the run is registered: a second start waits here and
    // then sees the first one running.
    let download = state.device_download.lock().await;
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
    // Checked again under the lock: a batch started while this one waited.
    if let Some((holder, message)) = crate::device_download_batch::busy_except(state, own_batch) {
        return Err(crate::tunnel_holder::gateway_busy(holder, message));
    }
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
    Ok(TunnelLocks {
        download,
        programming,
        monitor,
        scan,
    })
}

/// Which plan a start must arrive at again before anything is sent.
pub(crate) enum Expected<'a> {
    /// The single plan still in the slot under the start's id.
    Shown,
    /// The plan a batch review prepared for this device (ADR-0172 §4).
    Reviewed(&'a MemoryDownloadPlan),
}

/// One device's start, checked up to its phrase: everything `launch` needs.
pub(crate) struct DeviceStart<'a> {
    /// The plan id; the download keeps it as its id.
    pub(crate) id: u64,
    pub(crate) target: IndividualAddress,
    pub(crate) partial: Option<PartialDownloadParts>,
    pub(crate) changes: bool,
    pub(crate) gateway: SocketAddrV4,
    pub(crate) authorisation: WriteAuthorisation,
    pub(crate) accept_untested: Option<&'a str>,
    pub(crate) expected: Expected<'a>,
    /// The batch this device belongs to, recorded beside its download.
    pub(crate) batch: Option<u64>,
}

/// Starts one device's download with the tunnel reserved and its stop token
/// registered: the single start and every device of a batch run exactly this.
///
/// Prepares the plan again from the project as it is now, grades it, checks
/// the acknowledgement and that the plan is the expected one, writes the
/// durable history start, opens the tunnel and installs the session. Any
/// refusal returns before a tunnel opens, except a failed connect.
pub(crate) async fn launch(
    state: &SharedState,
    locks: TunnelLocks<'_>,
    stop: std::sync::Arc<knx_net::commissioning::memory_download::DownloadStop>,
    start: DeviceStart<'_>,
) -> Result<(), ApiError> {
    let TunnelLocks {
        mut download,
        programming,
        monitor,
        scan,
    } = locks;
    let DeviceStart {
        id,
        target,
        partial,
        changes,
        gateway,
        authorisation,
        accept_untested,
        expected,
        batch,
    } = start;
    // Before `prepare`: a project opened after this read can only make the
    // settlement skip, never land on the wrong project (ADR-0129).
    let incarnation = crate::programming_state::incarnation(state);
    let prepared = prepare_shown(state, target, partial, changes)?;
    let keying = keying(state, &prepared)?;
    // Recomputed from the plan that will run, not trusted from the shown
    // one: the evidence is the build's, the plan is checked equal below.
    let level = support_of(&prepared)?;
    if level.needs_acknowledgement() {
        let expected = untested_acknowledgement(target);
        if accept_untested != Some(expected.as_str()) {
            return Err(untested_refused(&expected));
        }
    }
    let unchanged = match expected {
        Expected::Shown => {
            let shown = state
                .device_download_plan
                .lock()
                .expect("state mutex poisoned");
            matches!(
                shown.as_ref(),
                Some(ShownDownload::Single(shown)) if shown.id == id && shown.plan == prepared.plan
            )
        }
        Expected::Reviewed(plan) => *plan == prepared.plan,
    };
    if !unchanged {
        return Err(conflict(
            "the project changed since the plan was shown; nothing was sent, ask for the plan again",
        ));
    }

    let report = {
        let Some(owner) = state.product_db.as_ref() else {
            return Err(ApiError::with_status(
                StatusCode::CONFLICT,
                no_application_program(state),
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
    // The receipt's preparation comes from the `prepared` that will run, read
    // in the catalogue lease that built it. A plan without a scope has none.
    let started = match DownloadPreparation::from_prepared(&prepared) {
        Some(preparation) => {
            state
                .one_shot_activity
                .start_download_prepared(id, target, preparation)
        }
        None => state.one_shot_activity.start_download(id, target),
    };
    let activity = started.map_err(|_| {
        ApiError::with_status(
            StatusCode::SERVICE_UNAVAILABLE,
            "activity history unavailable; not sent",
        )
    })?;
    let mut operation = OperationStart::new(OperationKind::Download)
        .target(serde_json::json!({
            "individualAddress": target.to_string(),
            "deviceName": prepared.device_name,
            "programId": prepared.request.program_id,
            "partial": partial.is_some(),
        }))
        .gateway(gateway)
        .related("downloadId", id);
    if let Some(batch) = batch {
        operation = operation.related("batchId", batch);
    }
    let recorder = crate::operation_recordings::start(
        state,
        operation.settings(crate::operation_recordings::tunnel_settings(
            TunnelAckWait::Standard,
            &state.device_download_timing,
        )),
    );
    let tunnel = match crate::operation_recordings::connect(
        state,
        gateway,
        TunnelAckWait::Standard,
        &recorder,
    )
    .await
    {
        Ok(tunnel) => tunnel,
        Err(e) => {
            crate::operation_recordings::finish_error(&recorder, &e, serde_json::Value::Null);
            // No tunnel, no telegram: say so in the history rather than
            // leaving it to the guard's drop, which records `unknown`
            // (AR18 review M6). If even that cannot be recorded, the drop
            // still does.
            let _ = activity.record_never_connected();
            return Err(crate::tunnel_holder::gateway_connect_failed(gateway, &e));
        }
    };
    let backups = BackupDestination {
        dir: state.data_dir.join(BACKUP_DIR),
        taken: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
    };
    let (settler, settlement) =
        crate::programming_state::download_settler_receipted(state, incarnation, id, &prepared);
    *download = Some(DeviceDownloadSession::start_with_stop_receipted(
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
        settlement,
        stop,
        recorder,
    ));
    drop(scan);
    drop(monitor);
    drop(programming);
    if matches!(expected, Expected::Shown) {
        // A plan is written at most once.
        *state
            .device_download_plan
            .lock()
            .expect("state mutex poisoned") = None;
    }
    Ok(())
}

async fn start(
    State(state): State<SharedState>,
    Json(body): Json<StartRequest>,
) -> Result<Json<StartResponse>, ApiError> {
    // Goal A3: no new tunnel once a stop has begun (KL-163).
    state.stop_admission.admit()?;
    let gateway = parse_gateway(&body.gateway)?;
    let (target, partial, changes) = {
        let shown = state
            .device_download_plan
            .lock()
            .expect("state mutex poisoned");
        match shown.as_ref() {
            Some(ShownDownload::Single(shown)) if shown.id == body.plan_id => {
                (shown.target, shown.partial, shown.changes)
            }
            _ => {
                return Err(conflict(
                    "that plan is no longer the one shown; ask for the plan again",
                ))
            }
        }
    };
    let authorisation = authorise(target, &body.confirmation)?;
    let locks = lock_tunnel(&state, None).await?;
    // Registered while the reservation is held, after the busy checks and
    // before anything else is started: a stop that begins from here on
    // requests this very token, whatever the async locks are doing
    // (ADR-0153). Refused with 503 once a stop has begun, with nothing
    // started or sent.
    let stop = state.stop_admission.register_download()?;
    launch(
        &state,
        locks,
        stop,
        DeviceStart {
            id: body.plan_id,
            target,
            partial,
            changes,
            gateway,
            authorisation,
            accept_untested: body.accept_untested.as_deref(),
            expected: Expected::Shown,
            batch: None,
        },
    )
    .await?;
    Ok(Json(StartResponse {
        download_id: body.plan_id,
    }))
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

    /// Issue #10 (3b): the remaining KL-170 refusal is a code a client
    /// explains, with the declared order and the target.
    #[tokio::test]
    async fn a_byte_order_refusal_is_a_structured_422() {
        let target: IndividualAddress = "0.2.23".parse().unwrap();
        let error = PrepareError::Image(ImageError::UnsupportedByteOrder {
            order: "MiddleEndian".into(),
        });
        let (status, body) = body(prepare_error(target, &error)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["kind"], "unsupportedByteOrder", "{body}");
        assert_eq!(body["order"], "MiddleEndian", "{body}");
        assert_eq!(body["category"], "byte-order", "{body}");
        assert_eq!(body["address"], "0.2.23", "{body}");
        assert_eq!(
            body["error"],
            "no download to device 0.2.23 prepared: memory image: not supported for a download: \
             ParameterByteOrder \"MiddleEndian\": only BigEndian, and LittleEndian in a memory \
             (070nh) layout, are written"
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
