//! `/api/device-download/batch/*`: review, start, watch and stop "download all pending" (ADR-0172).
//!
//! `plan` reviews every device the pending block lists, in its address
//! order, from one project snapshot: each is either in the batch with the plan
//! of its advised scope, or *not in this batch* with a reason. Nothing is sent.
//! `start` checks every phrase and acknowledgement before it takes the
//! reservation and then hands the devices to one server-held run
//! ([`crate::device_download_batch_run`]), which starts each exactly as the
//! single `start` does and stops at the first doubt. `status` reports a result
//! per device; `stop` ends the batch after the device in progress.

use std::net::SocketAddrV4;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use knx_app::device_download::PrepareError;
use knx_app::download_failure::FailureCode;
use knx_app::download_scope::{
    prepare_scoped_download, ScopeRequest, ScopedDownload, ScopedPrepareError,
};
use knx_app::download_support::{
    download_level, shipped_evidence, untested_acknowledgement, SupportLevel,
};
use knx_app::post_download_check::{
    IdentityCheck, Observation, PlanLoadVerdict, PostDownloadCheck, ReachCheck,
};
use knx_core::commissioning::memory_download::MemoryDownloadPlan;
use knx_core::commissioning::mutation::required_confirmation_phrase;
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::commissioning::scope_advice::{AddressNeed, AdvisedWrite, ScopeAdvice};
use knx_core::{
    is_project_excluded, ContactableAddress, DeviceInstance, IndividualAddress, WriteScope,
};
use knx_productdb::download_plan::DownloadPlanError;

use crate::device_download::{DownloadStatus, Restart, Written};
use crate::device_download_routes::{authorise, conflict, parse_gateway, ShownDownload};
use crate::errors::ApiError;
use crate::tunnel_holder::TunnelHolder;
use crate::SharedState;

/// The mask whose devices keep their own panel and phrase (ADR-0152).
const SYSTEM2_MASK: &str = "MV-0025";

pub fn device_download_batch_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/device-download/batch/plan", post(plan))
        .route("/api/device-download/batch/start", post(start))
        .route("/api/device-download/batch/status", get(status))
        .route("/api/device-download/batch/stop", post(stop))
}

/// Why a listed device takes no part (ADR-0172 §1). Never dropped silently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "code",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NotInBatch {
    /// The device has no individual address.
    NoAddress,
    /// The address is on the project's exclusion list.
    Excluded,
    /// Only the address (or the medium configuration) is pending: no memory
    /// download writes either.
    NothingToDownload,
    /// The project marks the address pending: a download would reach
    /// whichever device answers there now.
    AddressPending,
    /// A System-2 device: its own panel and phrase.
    System2,
    /// The plan of the advised scope does not build or grades `unsupported`.
    Unsupported { category: String, detail: String },
    /// No check after the download can be performed for the plan.
    NoPostDownloadCheck { detail: String },
}

/// One reviewed device's plan, kept for the run.
#[derive(Debug, Clone)]
pub struct BatchPlan {
    /// The plan id; the device's download keeps it as its id.
    pub(crate) id: u64,
    pub(crate) target: IndividualAddress,
    pub(crate) plan: MemoryDownloadPlan,
    pub(crate) partial: Option<PartialDownloadParts>,
    /// `true` when the plan grades `untested`.
    pub(crate) needs_acknowledgement: bool,
}

/// A reviewed device: in the batch with its plan, or not, with the reason.
#[derive(Debug, Clone)]
pub struct ReviewedDevice {
    pub(crate) device_id: u32,
    pub(crate) address: Option<IndividualAddress>,
    pub(crate) name: String,
    pub(crate) verdict: Result<BatchPlan, NotInBatch>,
}

/// The batch last reviewed, kept in the shown-plan slot until `start` takes it.
#[derive(Debug, Clone)]
pub struct ShownBatch {
    pub id: u64,
    pub(crate) devices: Vec<ReviewedDevice>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewRowDto {
    device_id: u32,
    address: Option<String>,
    name: String,
    #[serde(flatten)]
    verdict: VerdictDto,
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum VerdictDto {
    InBatch {
        plan_id: u64,
        program_id: String,
        /// `complete`, `parameters`, `groupAddresses` or `both`.
        scope: &'static str,
        steps: usize,
        data_octets: usize,
        /// `verified` or `untested`.
        support: &'static str,
        inferences: Vec<String>,
        warnings: Vec<String>,
        confirmation_phrase: String,
        /// The device's own acknowledgement, for `untested` only.
        untested_acknowledgement: Option<String>,
    },
    NotInBatch {
        reason: NotInBatch,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanResponse {
    batch_id: u64,
    devices: Vec<ReviewRowDto>,
}

/// A device the pending block lists: at least one of its four parts pending.
fn listed(device: &DeviceInstance) -> bool {
    let pending = device.commissioning.pending_parts();
    pending.address || pending.application || pending.parameters || pending.group_addresses
}

/// A reviewed candidate before ids and phrases are assigned.
struct Candidate {
    device_id: u32,
    address: Option<IndividualAddress>,
    name: String,
    verdict: Result<(knx_app::device_download::PreparedDownload, SupportLevel), NotInBatch>,
}

/// The System-2 refusal: a ProductProcedure on mask `0025h` the regular
/// memory download does not load.
fn is_system2(error: &PrepareError) -> bool {
    matches!(
        error,
        PrepareError::Plan(DownloadPlanError::NotMemoryMapped(Some(mask))) if mask == SYSTEM2_MASK
    )
}

fn unsupported(category: &str, detail: impl ToString) -> NotInBatch {
    NotInBatch::Unsupported {
        category: category.to_string(),
        detail: detail.to_string(),
    }
}

/// Grades one listed device by the rules of ADR-0172 §1, in this order.
fn candidate(
    products: &knx_productdb::Connection,
    project: &knx_core::Project,
    device: &DeviceInstance,
    evidence: &std::collections::BTreeMap<String, knx_app::download_support::VerifiedEvidence>,
) -> Candidate {
    let verdict = (|| {
        let address = device.address.ok_or(NotInBatch::NoAddress)?;
        if is_project_excluded(address) || ContactableAddress::new(address).is_err() {
            return Err(NotInBatch::Excluded);
        }
        let advice = ScopeAdvice::for_device(device);
        if advice.write == AdvisedWrite::Nothing {
            return Err(NotInBatch::NothingToDownload);
        }
        if advice.address == AddressNeed::Pending {
            return Err(NotInBatch::AddressPending);
        }
        let prepared =
            match prepare_scoped_download(products, project, address, ScopeRequest::Changes) {
                Ok(ScopedDownload {
                    prepared: Some(prepared),
                    ..
                }) => prepared,
                Ok(ScopedDownload { prepared: None, .. }) => {
                    return Err(NotInBatch::NothingToDownload)
                }
                Err(ScopedPrepareError::Prepare(error)) if is_system2(&error) => {
                    return Err(NotInBatch::System2)
                }
                Err(ScopedPrepareError::Prepare(error)) => {
                    return Err(unsupported(
                        knx_app::project_readiness::prepare_category(&error).code(),
                        error,
                    ))
                }
                Err(ScopedPrepareError::Partial(error)) => {
                    return Err(unsupported("partial", error))
                }
            };
        let level = download_level(&prepared, evidence);
        if let SupportLevel::Unsupported { category, detail } = &level {
            return Err(unsupported(category.code(), detail));
        }
        knx_net::commissioning::post_download::check_performable(&prepared.plan).map_err(
            |reason| NotInBatch::NoPostDownloadCheck {
                detail: reason.to_string(),
            },
        )?;
        Ok((prepared, level))
    })();
    Candidate {
        device_id: device.id.0,
        address: device.address,
        name: device.name.clone(),
        verdict,
    }
}

/// Reviews every listed device from one project snapshot and one catalogue
/// read, in the pending block's order: no address first, then by address.
fn review(state: &SharedState) -> Result<Vec<Candidate>, ApiError> {
    let evidence = shipped_evidence()
        .map_err(|e| ApiError::with_status(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let Some(owner) = state.product_db.as_ref() else {
        return Err(ApiError::with_kind(
            StatusCode::CONFLICT,
            "downloadProductDataMissing",
            crate::device_download_routes::no_application_program(state),
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
    let mut devices: Vec<&DeviceInstance> = project.devices.iter().filter(|d| listed(d)).collect();
    devices.sort_by_key(|device| (device.address, device.id));
    owner
        .with_read(|products| {
            Ok::<_, std::convert::Infallible>(
                devices
                    .into_iter()
                    .map(|device| candidate(products, project, device, &evidence))
                    .collect(),
            )
        })
        .map_err(crate::catalogue_owner::owner_error)?
        .map_err(|never| match never {})
}

async fn plan(State(state): State<SharedState>) -> Result<Json<PlanResponse>, ApiError> {
    let candidates = review(&state)?;
    let batch_id = state.next_device_download_id.fetch_add(1, Ordering::SeqCst);
    let mut rows = Vec::with_capacity(candidates.len());
    let mut devices = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let verdict = match candidate.verdict {
            Ok((prepared, level)) => match crate::device_download_routes::keying(&state, &prepared)
            {
                Err(error) => Err(unsupported("access-key", error.message())),
                Ok(_) => Ok((prepared, level)),
            },
            Err(reason) => Err(reason),
        };
        let (dto, kept) = match verdict {
            Err(reason) => (
                VerdictDto::NotInBatch {
                    reason: reason.clone(),
                },
                Err(reason),
            ),
            Ok((prepared, level)) => {
                let target = prepared.target;
                let id = state.next_device_download_id.fetch_add(1, Ordering::SeqCst);
                let partial = prepared.partial.as_ref().map(|(parts, _)| *parts);
                let needs_acknowledgement = level.needs_acknowledgement();
                let dto = VerdictDto::InBatch {
                    plan_id: id,
                    program_id: prepared.request.program_id.clone(),
                    scope: partial
                        .map_or(AdvisedWrite::Complete, AdvisedWrite::Partial)
                        .code(),
                    steps: prepared.plan.steps.len(),
                    data_octets: prepared.plan.data_octets(),
                    support: level.code(),
                    inferences: match &level {
                        SupportLevel::Untested { inferences, .. } => {
                            inferences.iter().map(ToString::to_string).collect()
                        }
                        _ => Vec::new(),
                    },
                    warnings: prepared.warnings.iter().map(ToString::to_string).collect(),
                    confirmation_phrase: required_confirmation_phrase(target, WriteScope::Download),
                    untested_acknowledgement: needs_acknowledgement
                        .then(|| untested_acknowledgement(target)),
                };
                let kept = Ok(BatchPlan {
                    id,
                    target,
                    plan: prepared.plan,
                    partial,
                    needs_acknowledgement,
                });
                (dto, kept)
            }
        };
        rows.push(ReviewRowDto {
            device_id: candidate.device_id,
            address: candidate.address.map(|address| address.to_string()),
            name: candidate.name.clone(),
            verdict: dto,
        });
        devices.push(ReviewedDevice {
            device_id: candidate.device_id,
            address: candidate.address,
            name: candidate.name,
            verdict: kept,
        });
    }
    // The batch replaces whatever plan was shown; a running batch keeps the
    // plans it took at its start and is not touched.
    *state
        .device_download_plan
        .lock()
        .expect("state mutex poisoned") = Some(ShownDownload::Batch(ShownBatch {
        id: batch_id,
        devices,
    }));
    Ok(Json(PlanResponse {
        batch_id,
        devices: rows,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartDevice {
    address: String,
    confirmation: String,
    #[serde(default)]
    accept_untested: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartRequest {
    batch_id: u64,
    gateway: String,
    /// Exactly the devices to write, in any order; the batch keeps its own.
    devices: Vec<StartDevice>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartResponse {
    batch_id: u64,
}

/// One device the run starts, with what its start needs.
#[derive(Debug, Clone)]
pub(crate) struct RunDevice {
    pub(crate) plan: BatchPlan,
    pub(crate) confirmation: String,
    pub(crate) accept_untested: Option<String>,
}

fn device_refused(address: &str, message: impl std::fmt::Display) -> ApiError {
    ApiError::with_kind(
        StatusCode::BAD_REQUEST,
        "downloadConfirmationRefused",
        format!("nothing was started: device {address}: {message}"),
    )
}

/// Every listed device checked against the batch, before any reservation:
/// one wrong phrase or acknowledgement refuses the whole batch.
fn check_devices(batch: &ShownBatch, listed: &[StartDevice]) -> Result<Vec<RunDevice>, ApiError> {
    if listed.is_empty() {
        return Err(ApiError::with_kind(
            StatusCode::BAD_REQUEST,
            "batchEmpty",
            "nothing was started: the request lists no device",
        ));
    }
    let mut chosen: Vec<(usize, &StartDevice)> = Vec::with_capacity(listed.len());
    for device in listed {
        let position = batch
            .devices
            .iter()
            .position(|reviewed| {
                reviewed.verdict.is_ok()
                    && reviewed.address.map(|a| a.to_string()).as_deref()
                        == Some(device.address.as_str())
            })
            .ok_or_else(|| {
                ApiError::with_kind(
                    StatusCode::BAD_REQUEST,
                    "batchDeviceNotInBatch",
                    format!(
                        "nothing was started: device {} is not in this batch",
                        device.address
                    ),
                )
            })?;
        if chosen.iter().any(|(earlier, _)| *earlier == position) {
            return Err(ApiError::with_kind(
                StatusCode::BAD_REQUEST,
                "batchDeviceRepeated",
                format!(
                    "nothing was started: device {} is listed twice",
                    device.address
                ),
            ));
        }
        chosen.push((position, device));
    }
    chosen.sort_by_key(|(position, _)| *position);
    chosen
        .into_iter()
        .map(|(position, device)| {
            let plan = batch.devices[position]
                .verdict
                .clone()
                .expect("only devices in the batch were chosen");
            authorise(plan.target, &device.confirmation)
                .map_err(|e| device_refused(&device.address, e.message()))?;
            if plan.needs_acknowledgement {
                let expected = untested_acknowledgement(plan.target);
                if device.accept_untested.as_deref() != Some(expected.as_str()) {
                    return Err(device_refused(
                        &device.address,
                        format!(
                            "this download is untested on hardware; acceptUntested must read \
                             exactly {expected:?}"
                        ),
                    ));
                }
            }
            Ok(RunDevice {
                plan,
                confirmation: device.confirmation.clone(),
                accept_untested: device.accept_untested.clone(),
            })
        })
        .collect()
}

async fn start(
    State(state): State<SharedState>,
    Json(body): Json<StartRequest>,
) -> Result<Json<StartResponse>, ApiError> {
    state.stop_admission.admit()?;
    let gateway: SocketAddrV4 = parse_gateway(&body.gateway)?;
    let shown = {
        let slot = state
            .device_download_plan
            .lock()
            .expect("state mutex poisoned");
        match slot.as_ref() {
            Some(ShownDownload::Batch(shown)) if shown.id == body.batch_id => shown.clone(),
            _ => {
                return Err(conflict(
                    "that batch is no longer the one shown; ask for the review again",
                ))
            }
        }
    };
    let run = check_devices(&shown, &body.devices)?;
    // Every phrase is right; now the reservation, as a single start takes it.
    let locks = crate::device_download_routes::lock_tunnel(&state, None).await?;
    let batch = Arc::new(DownloadBatch::new(&shown, &run));
    {
        let mut slot = state
            .device_download_plan
            .lock()
            .expect("state mutex poisoned");
        // The review is the one checked above, or nothing starts.
        if !matches!(slot.as_ref(), Some(ShownDownload::Batch(now)) if now.id == shown.id) {
            return Err(conflict(
                "that batch is no longer the one shown; ask for the review again",
            ));
        }
        // The plans move into the run: a later `/plan` cannot change them.
        *slot = None;
    }
    *state
        .device_download_batch
        .lock()
        .expect("state mutex poisoned") = Some(Arc::clone(&batch));
    drop(locks);
    crate::device_download_batch_run::spawn(Arc::clone(&state), batch, gateway, run);
    Ok(Json(StartResponse {
        batch_id: body.batch_id,
    }))
}

/// Why a device of the batch was not started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NotStartedReason {
    /// An earlier device did not end with every condition met.
    EarlierDevice,
    /// The user asked to stop after the device before.
    StopRequest,
    /// The server is stopping.
    ServerStop,
    /// The run ended without reaching this device.
    RunEnded,
}

/// Where one reviewed device of the batch stands (ADR-0172 §7 and amendment).
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DeviceState {
    Waiting {
        download_id: u64,
    },
    Running {
        download_id: u64,
    },
    Done {
        download_id: u64,
        written: Written,
        restart: Restart,
        post_download_check: PostDownloadCheck,
    },
    Unconfirmed {
        download_id: u64,
        /// The condition of §5 that did not hold.
        doubt: Doubt,
        written: Written,
        restart: Restart,
        restart_note: Option<String>,
        post_download_check: PostDownloadCheck,
    },
    Failed {
        download_id: u64,
        written: Written,
        stopped_in_step: Option<usize>,
        error: String,
        hint: Option<String>,
        /// The refusal's kind when the start ended before the download ran.
        kind: Option<&'static str>,
        backup_file: Option<String>,
    },
    Stopped {
        download_id: u64,
        backup_file: Option<String>,
    },
    NotStarted {
        download_id: u64,
        reason: NotStartedReason,
    },
    NotInBatch {
        reason: NotInBatch,
    },
    Deselected,
}

/// The first condition of ADR-0172 §5 a `Finished` device did not meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Doubt {
    NotWritten,
    RestartUnconfirmed,
    CheckNotChecked,
    CheckUnsupported,
    CheckNotPerformed,
    CheckInterrupted,
    CheckNotReached,
    CheckReachUnknown,
    IdentityUnavailable,
    IdentityMismatch,
    LoadStateDiffers,
    LoadStateUnknown,
}

/// The doubt, if any, that a finished device leaves: `None` only for
/// `written: yes`, a restart acknowledged or not in the plan, and a check
/// that reached the device, matched its identity and read every load state
/// as the plan leaves it.
pub fn doubt(written: Written, restart: Restart, check: &PostDownloadCheck) -> Option<Doubt> {
    if written != Written::Yes {
        return Some(Doubt::NotWritten);
    }
    if !matches!(restart, Restart::Acknowledged | Restart::NotInPlan) {
        return Some(Doubt::RestartUnconfirmed);
    }
    match &check.observation {
        Observation::NotChecked {} => Some(Doubt::CheckNotChecked),
        Observation::Unsupported { .. } => Some(Doubt::CheckUnsupported),
        Observation::NotPerformed {} => Some(Doubt::CheckNotPerformed),
        Observation::Interrupted {} => Some(Doubt::CheckInterrupted),
        Observation::Attempted { reach } => match reach {
            ReachCheck::NotReached {} => Some(Doubt::CheckNotReached),
            ReachCheck::Unknown {} => Some(Doubt::CheckReachUnknown),
            ReachCheck::Reached { identity } => match identity {
                IdentityCheck::Unavailable { .. } => Some(Doubt::IdentityUnavailable),
                IdentityCheck::Mismatch { .. } => Some(Doubt::IdentityMismatch),
                IdentityCheck::Match(matched) => match matched.plan_load_verdict() {
                    PlanLoadVerdict::MatchesPlan => None,
                    PlanLoadVerdict::DiffersFromPlan => Some(Doubt::LoadStateDiffers),
                    PlanLoadVerdict::Unknown => Some(Doubt::LoadStateUnknown),
                },
            },
        },
    }
}

/// A device's terminal download status as its batch state, and whether the
/// batch may go on to the next device.
pub fn judge(
    download_id: u64,
    status: DownloadStatus,
    backup_file: Option<String>,
) -> (DeviceState, bool) {
    match status {
        DownloadStatus::Finished {
            written,
            restart,
            restart_note,
            post_download_check,
        } => match doubt(written, restart, &post_download_check) {
            None => (
                DeviceState::Done {
                    download_id,
                    written,
                    restart,
                    post_download_check,
                },
                true,
            ),
            Some(doubt) => (
                DeviceState::Unconfirmed {
                    download_id,
                    doubt,
                    written,
                    restart,
                    restart_note,
                    post_download_check,
                },
                false,
            ),
        },
        DownloadStatus::Failed { failure, .. } if failure.code == FailureCode::Stopped => (
            DeviceState::Stopped {
                download_id,
                backup_file,
            },
            false,
        ),
        DownloadStatus::Failed {
            written,
            stopped_in_step,
            error,
            hint,
            ..
        } => (
            DeviceState::Failed {
                download_id,
                written,
                stopped_in_step,
                error,
                hint,
                kind: None,
                backup_file,
            },
            false,
        ),
        // Unreachable while the batch holds the reservation. `Written` has no
        // `unknown`; `partially` is the value that never understates a write
        // (the same choice as an unwitnessed end, ADR-0067), and the text says
        // the outcome is unknown.
        DownloadStatus::Running => (
            DeviceState::Failed {
                download_id,
                written: Written::Partially,
                stopped_in_step: None,
                error: "the download did not report an end; its outcome is unknown".into(),
                hint: None,
                kind: None,
                backup_file,
            },
            false,
        ),
    }
}

struct BatchRow {
    device_id: u32,
    address: Option<IndividualAddress>,
    name: String,
    state: DeviceState,
}

/// One batch run, kept after it ends so its results stay readable until the
/// next batch replaces it (ADR-0172 §7: server memory, not a durable record).
pub struct DownloadBatch {
    pub(crate) id: u64,
    rows: Mutex<Vec<BatchRow>>,
    stop_requested: AtomicBool,
    ended: AtomicBool,
}

impl DownloadBatch {
    fn new(shown: &ShownBatch, run: &[RunDevice]) -> Self {
        let rows = shown
            .devices
            .iter()
            .map(|reviewed| BatchRow {
                device_id: reviewed.device_id,
                address: reviewed.address,
                name: reviewed.name.clone(),
                state: match &reviewed.verdict {
                    Err(reason) => DeviceState::NotInBatch {
                        reason: reason.clone(),
                    },
                    Ok(plan) if run.iter().any(|device| device.plan.id == plan.id) => {
                        DeviceState::Waiting {
                            download_id: plan.id,
                        }
                    }
                    Ok(_) => DeviceState::Deselected,
                },
            })
            .collect();
        Self {
            id: shown.id,
            rows: Mutex::new(rows),
            stop_requested: AtomicBool::new(false),
            ended: AtomicBool::new(false),
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        !self.ended.load(Ordering::SeqCst)
    }

    pub(crate) fn stop_requested(&self) -> bool {
        self.stop_requested.load(Ordering::SeqCst)
    }

    /// Ends the run; every device still waiting becomes `notStarted`.
    pub(crate) fn end(&self, reason: NotStartedReason) {
        let mut rows = self.rows.lock().expect("batch rows poisoned");
        for row in rows.iter_mut() {
            if let DeviceState::Waiting { download_id } = row.state {
                row.state = DeviceState::NotStarted {
                    download_id,
                    reason,
                };
            }
        }
        self.ended.store(true, Ordering::SeqCst);
    }

    /// Sets the state of the device whose download id is `download_id`.
    pub(crate) fn set(&self, download_id: u64, state: DeviceState) {
        let mut rows = self.rows.lock().expect("batch rows poisoned");
        if let Some(row) = rows
            .iter_mut()
            .find(|row| row_download_id(&row.state) == Some(download_id))
        {
            row.state = state;
        }
    }

    /// The device in progress, or else the next one, with its 1-based
    /// position among the devices started or to be started.
    fn current(&self) -> (Option<IndividualAddress>, usize, usize) {
        let rows = self.rows.lock().expect("batch rows poisoned");
        let run: Vec<&BatchRow> = rows
            .iter()
            .filter(|row| row_download_id(&row.state).is_some())
            .collect();
        let at = run
            .iter()
            .position(|row| matches!(row.state, DeviceState::Running { .. }))
            .or_else(|| {
                run.iter()
                    .position(|row| matches!(row.state, DeviceState::Waiting { .. }))
            })
            .unwrap_or(run.len().saturating_sub(1));
        (run.get(at).and_then(|row| row.address), at + 1, run.len())
    }

    #[cfg(test)]
    pub(crate) fn running_for_test(id: u64, address: IndividualAddress) -> Self {
        Self {
            id,
            rows: Mutex::new(vec![BatchRow {
                device_id: 1,
                address: Some(address),
                name: "synthetic device".into(),
                state: DeviceState::Waiting {
                    download_id: id + 1,
                },
            }]),
            stop_requested: AtomicBool::new(false),
            ended: AtomicBool::new(false),
        }
    }
}

fn row_download_id(state: &DeviceState) -> Option<u64> {
    match state {
        DeviceState::Waiting { download_id }
        | DeviceState::Running { download_id }
        | DeviceState::Done { download_id, .. }
        | DeviceState::Unconfirmed { download_id, .. }
        | DeviceState::Failed { download_id, .. }
        | DeviceState::Stopped { download_id, .. }
        | DeviceState::NotStarted { download_id, .. } => Some(*download_id),
        DeviceState::NotInBatch { .. } | DeviceState::Deselected => None,
    }
}

/// The batch holding the reservation, unless it is `own`: its holder and the
/// busy message naming it.
pub(crate) fn busy_except(
    state: &crate::AppState,
    own: Option<u64>,
) -> Option<(TunnelHolder, String)> {
    let batch = state
        .device_download_batch
        .lock()
        .expect("state mutex poisoned")
        .clone()?;
    if !batch.is_running() || Some(batch.id) == own {
        return None;
    }
    let (address, position, total) = batch.current();
    let message = format!(
        "a batch download is running (device {position} of {total}): the gateway serves one tunnel"
    );
    Some((
        TunnelHolder::DeviceDownloadBatch {
            batch_id: batch.id,
            address: address.map(|address| address.to_string()),
            position,
            total,
        },
        message,
    ))
}

/// Whether a batch run holds the reservation.
pub(crate) fn running(state: &crate::AppState) -> bool {
    state
        .device_download_batch
        .lock()
        .expect("state mutex poisoned")
        .as_ref()
        .is_some_and(|batch| batch.is_running())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchQuery {
    batch_id: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusRow {
    device_id: u32,
    address: Option<String>,
    name: String,
    #[serde(flatten)]
    state: DeviceState,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusResponse {
    batch_id: u64,
    /// `running` or `ended`.
    state: &'static str,
    stop_requested: bool,
    devices: Vec<StatusRow>,
}

fn current_batch(
    state: &crate::AppState,
    asked: Option<u64>,
) -> Result<Arc<DownloadBatch>, ApiError> {
    let batch = state
        .device_download_batch
        .lock()
        .expect("state mutex poisoned")
        .clone()
        .ok_or_else(|| ApiError::with_status(StatusCode::NOT_FOUND, "no batch download yet"))?;
    if asked.is_some_and(|id| id != batch.id) {
        return Err(ApiError::with_status(
            StatusCode::NOT_FOUND,
            format!("batch {} has been replaced", asked.unwrap_or_default()),
        ));
    }
    Ok(batch)
}

async fn status(
    State(state): State<SharedState>,
    Query(query): Query<BatchQuery>,
) -> Result<Json<StatusResponse>, ApiError> {
    let batch = current_batch(&state, query.batch_id)?;
    // Read before the rows: an `ended` answer only ever comes with final rows.
    let running = batch.is_running();
    let rows = batch.rows.lock().expect("batch rows poisoned");
    Ok(Json(StatusResponse {
        batch_id: batch.id,
        state: if running { "running" } else { "ended" },
        stop_requested: batch.stop_requested(),
        devices: rows
            .iter()
            .map(|row| StatusRow {
                device_id: row.device_id,
                address: row.address.map(|address| address.to_string()),
                name: row.name.clone(),
                state: row.state.clone(),
            })
            .collect(),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StopRequest {
    batch_id: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StopResponse {
    /// `afterThisDevice` while the batch runs, `ended` once it is over.
    stop: &'static str,
}

/// *Stop after this device*: the device in progress runs to its end, the rest
/// become `notStarted`. Never stops a device mid-download (ADR-0172 §6).
async fn stop(
    State(state): State<SharedState>,
    Json(body): Json<StopRequest>,
) -> Result<Json<StopResponse>, ApiError> {
    let batch = current_batch(&state, Some(body.batch_id))?;
    if !batch.is_running() {
        return Ok(Json(StopResponse { stop: "ended" }));
    }
    batch.stop_requested.store(true, Ordering::SeqCst);
    Ok(Json(StopResponse {
        stop: "afterThisDevice",
    }))
}

#[cfg(test)]
#[path = "device_download_batch_tests.rs"]
mod tests;
