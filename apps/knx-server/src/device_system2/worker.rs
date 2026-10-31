//! Real tunnel bridge, existing admission, retained task and durable App executor.
use super::*;
use dto::{Recover, Start};
use knx_app::{
    commissioning_activity::{DownloadGuard, DownloadResult, Restart, Written},
    property_validation::{OperationProgress, OperationStore, System2OperationLease},
    property_validation_bundle::PreparedBundle,
    system2_validation_preparation::{self as app, CurrentProjectHardware},
};
use knx_net::{
    ApplicationService, BusError, Destination, ManagementTransport, Tpci, TunnelAckWait,
};
use settlement::Run;
use std::{net::SocketAddrV4, sync::atomic::Ordering};

use crate::tunnel_workers::off_tunnel_workers;

// Only the owning mounted fixture installs this observer. No global state,
// production observer, error text, changed error propagation or extra timeout.
macro_rules! observe {
    ($state:expr, $method:ident($($arg:expr),* $(,)?)) => {
        #[cfg(test)]
        if let Some(observer) = &$state.device_system2.observation {
            observer.$method($($arg),*);
        }
    };
}

pub(super) fn gateway(text: &str) -> Result<SocketAddrV4, ApiError> {
    let g: SocketAddrV4 = text
        .parse()
        .map_err(|_| ApiError::bad_request("system2InvalidGateway"))?;
    if g.port() == 0 || g.ip().is_unspecified() || g.ip().is_multicast() || g.ip().is_broadcast() {
        return Err(ApiError::bad_request("system2InvalidGateway"));
    }
    Ok(g)
}
/// Established download → programming → monitor → scan ordering. All uses are
/// try_lock, so reverse-order short routes cannot form a wait cycle.
pub(super) fn reserve(state: &SharedState) -> Result<Reservation<'_>, ApiError> {
    let busy = || refuse("gatewayBusy");
    let download = state.device_download.try_lock().map_err(|_| busy())?;
    if download.as_ref().is_some_and(|s| s.is_running()) {
        return Err(busy());
    }
    let programming = state.address_programming.try_lock().map_err(|_| busy())?;
    if programming.as_ref().is_some_and(|s| s.is_active()) {
        return Err(busy());
    }
    let monitor = state.bus_session.try_lock().map_err(|_| busy())?;
    if monitor.is_some() {
        return Err(busy());
    }
    let scan = state.line_scan_session.try_lock().map_err(|_| busy())?;
    if scan
        .as_ref()
        .and_then(crate::tunnel_holder::scan_holder_of)
        .is_some()
    {
        return Err(busy());
    }
    Ok((download, programming, monitor, scan))
}
type Reservation<'a> = (
    tokio::sync::MutexGuard<'a, Option<crate::DeviceDownloadSession>>,
    tokio::sync::MutexGuard<'a, Option<crate::AddressProgrammingSession>>,
    tokio::sync::MutexGuard<'a, Option<crate::BusSession>>,
    tokio::sync::MutexGuard<'a, Option<crate::bus_scan::LineScanSession>>,
);
pub(super) fn check_source(state: &SharedState, preparation: &Preparation) -> Result<(), ApiError> {
    let project = state
        .project
        .lock()
        .map_err(|_| refuse("system2Unavailable"))?;
    check_binding(state, &preparation.metadata.binding)?;
    let project = project
        .as_ref()
        .ok_or_else(|| refuse("system2ProjectRequired"))?;
    state
        .product_db
        .as_ref()
        .ok_or_else(|| refuse("system2RetainedSourcesRequired"))?
        .with_read(|conn| {
            preparation.source.check_current(
                conn,
                project,
                knx_core::DeviceId(preparation.metadata.binding.device_id),
            )
        })
        .map_err(|_| refuse("system2CatalogueUnavailable"))?
        .map_err(|_| refuse("system2PreparationStale"))
}
fn admit(
    state: &SharedState,
    metadata: &Metadata,
    bundle: &PreparedBundle,
) -> Result<CurrentProjectHardware, ApiError> {
    let project = state
        .project
        .lock()
        .map_err(|_| refuse("system2Unavailable"))?;
    check_binding(state, &metadata.binding)?;
    let project = project
        .as_ref()
        .ok_or_else(|| refuse("system2ProjectRequired"))?;
    state
        .product_db
        .as_ref()
        .ok_or_else(|| refuse("system2RetainedSourcesRequired"))?
        .with_read(|conn| {
            app::admit_current_project_hardware(
                conn,
                project,
                knx_core::DeviceId(metadata.binding.device_id),
                bundle,
            )
        })
        .map_err(|_| refuse("system2CatalogueUnavailable"))?
        .map_err(|_| refuse("system2PreparationStale"))
}
/// Hardware kind is derived from actual tunnel provenance, NEVER forced from a
/// source flag. Only the exact System-2 service grammar may pass this bridge.
struct Transport<'a> {
    tunnel: &'a dyn crate::BusTunnel,
    writes: Option<(&'a Activity, watch::Receiver<OperationProgress>)>,
    #[cfg(test)]
    observation: Option<&'a test_observation::Observation>,
}
impl ManagementTransport for Transport<'_> {
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        self.tunnel.assigned_address()
    }
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<knx_net::TunnelEvent> {
        self.tunnel.subscribe()
    }
    #[cfg(test)]
    fn observe_memory_read(
        &self,
        snapshot: impl FnOnce() -> knx_net::management::MemoryReadObservation,
    ) {
        // Only the actual download/recovery executor bridge opts in. Read-only
        // initial identity observation remains inert.
        if self.writes.is_some() {
            if let Some(observer) = self.observation {
                observer.memory_read(snapshot());
            }
        }
    }
    fn target_kind(&self) -> knx_core::commissioning::mutation::TargetKind {
        if self.tunnel.diagnostics_are_simulated() {
            knx_core::commissioning::mutation::TargetKind::Simulator
        } else {
            knx_core::commissioning::mutation::TargetKind::Hardware
        }
    }
    async fn send_frame(
        &self,
        d: Destination,
        t: Tpci,
        s: ApplicationService,
    ) -> Result<(), BusError> {
        #[cfg(test)]
        if let (Some(observer), Some((_, updates))) = (self.observation, &self.writes) {
            observer.progress(&updates.borrow());
        }
        match &s {
            ApplicationService::MemoryWrite { .. }
            | ApplicationService::UserMemoryWrite { .. }
            | ApplicationService::PropertyValueWrite { .. } => {
                let (activity, progress) = self.writes.as_ref().ok_or_else(|| {
                    BusError::Protocol("System-2 observation is read-only".into())
                })?;
                // App sends this notification ONLY after actual recovery bytes,
                // committed HEAD and full readback. This includes VerifyMode setup.
                if !progress.borrow().backup_available {
                    return Err(BusError::Protocol(
                        "System-2 durable actual backup required before mutation".into(),
                    ));
                }
                activity.mark_send_possible().map_err(BusError::Io)?;
            }
            ApplicationService::NoApplicationPdu
            | ApplicationService::MemoryRead { .. }
            | ApplicationService::UserMemoryRead { .. }
            | ApplicationService::PropertyValueRead { .. }
            | ApplicationService::DeviceDescriptorRead { .. } => {}
            _ => {
                return Err(BusError::Protocol(
                    "System-2 unexpected application service refused".into(),
                ))
            }
        }
        self.tunnel.send_frame(d, t, s).await
    }
}
fn cross_process(state: &crate::AppState) -> Result<std::fs::File, ApiError> {
    match &state.diagnostic_lease_dir {
        Some(dir) => knx_app::device_diagnostics::lease_in(dir),
        None => knx_app::device_diagnostics::gateway_lease(),
    }
    .map_err(|_error| {
        observe!(state, failure("cross-process-lease", "lease-refused", None));
        refuse("gatewayBusy")
    })
}
struct Activity {
    guard: Mutex<Option<DownloadGuard>>,
    send_possible: std::sync::atomic::AtomicBool,
}
impl Activity {
    fn mark_send_possible(&self) -> std::io::Result<()> {
        if !self.send_possible.load(Ordering::SeqCst) {
            self.guard
                .lock()
                .map_err(|_| std::io::Error::other("activity unavailable"))?
                .as_mut()
                .ok_or_else(|| std::io::Error::other("activity ended"))?
                .mark_send_possible()?;
            self.send_possible.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
}
fn activity(state: &crate::AppState, address: &str) -> Result<Activity, ApiError> {
    state
        .one_shot_activity
        .ensure_write_available()
        .map_err(|_| refuse("system2ActivityUnavailable"))?;
    let id = state
        .next_device_download_id
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |id| {
            id.checked_add(1).filter(|n| *n <= i64::MAX as u64)
        })
        .map_err(|_| refuse("system2ActivityUnavailable"))?;
    let target = address
        .parse()
        .map_err(|_| refuse("system2InvalidTarget"))?;
    let guard = state
        .one_shot_activity
        .start_download(id, target)
        .map_err(|_| refuse("system2ActivityUnavailable"))?;
    Ok(Activity {
        guard: Mutex::new(Some(guard)),
        send_possible: std::sync::atomic::AtomicBool::new(false),
    })
}
fn finish_activity(activity: Activity, result: bool, clean: bool) {
    if let Some(mut guard) = activity
        .guard
        .into_inner()
        .unwrap_or_else(|p| p.into_inner())
    {
        let written = if activity.send_possible.load(Ordering::SeqCst) {
            if result {
                Written::Yes
            } else {
                Written::Partially
            }
        } else {
            Written::No
        };
        let outcome = if result {
            DownloadResult::Finished {
                written,
                restart: Restart::NotInPlan,
            }
        } else {
            DownloadResult::Failed { written }
        };
        let _ = guard.record_result(&outcome);
        let _ = guard.record_cleanup(clean);
    }
}
/// Terminal project-belief settlement on the blocking pool: the project lock, catalogue read
/// and durable persistence never run on a runtime worker.
async fn settle_off_runtime(
    state: &SharedState,
    meta: &Metadata,
    progress: OperationProgress,
    run: Run,
    bundle: Option<PreparedBundle>,
) -> Receipt {
    let copy = state.clone();
    let meta = meta.clone();
    tokio::task::spawn_blocking(move || {
        settlement::settle(&copy, &meta, &progress, run, |project, device| {
            bundle.as_ref().is_some_and(|bundle| {
                copy.product_db.as_ref().is_some_and(|owner| {
                    owner
                        .with_read(|conn| {
                            Ok::<_, std::convert::Infallible>(
                                app::admit_current_project_hardware(conn, project, device, bundle)
                                    .is_ok(),
                            )
                        })
                        .ok()
                        .and_then(Result::ok)
                        .unwrap_or(false)
                })
            })
        })
    })
    .await
    .unwrap_or(Receipt::Unavailable)
}
fn terminal(store: &OperationStore) -> LocalOutcome {
    match store.outcome() {
        LocalOutcome::Running | LocalOutcome::Restoring => LocalOutcome::Interrupted,
        outcome => outcome,
    }
}

/// Aborts the supervised worker if the supervisor itself is dropped (runtime shutdown), so a
/// detached worker can never outlive the handle the service retains.
struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// The retained service handle: ends only after the worker AND, on an unexpected end, the
/// conservative terminal settlement, so no status is terminal before its belief is settled.
pub(super) async fn supervised_download(
    state: SharedState,
    root: Arc<storage::Storage>,
    preparation: Preparation,
    body: Start,
    gateway: SocketAddrV4,
    lease: System2OperationLease,
) {
    let meta = preparation.metadata.clone();
    let (copy, storage) = (state.clone(), root.clone());
    let mut worker = AbortOnDrop(tokio::spawn(download(
        state,
        root,
        preparation,
        body,
        gateway,
        lease,
    )));
    if (&mut worker.0).await.is_err() {
        unexpected_end(&copy, &storage, &meta, Run::Download).await;
    }
}

/// Recovery counterpart of [`supervised_download`].
pub(super) async fn supervised_recover(
    state: SharedState,
    root: Arc<storage::Storage>,
    bundle: PreparedBundle,
    store: OperationStore,
    body: Recover,
    gateway: SocketAddrV4,
    lease: System2OperationLease,
) {
    let id = body.operation_id.clone();
    let (copy, storage) = (state.clone(), root.clone());
    let mut worker = AbortOnDrop(tokio::spawn(recover(
        state, root, bundle, store, body, gateway, lease,
    )));
    if (&mut worker.0).await.is_err() {
        match storage.metadata(&id) {
            Ok(meta) => unexpected_end(&copy, &storage, &meta, Run::Recovery).await,
            Err(_) => copy.device_system2.publish_end(
                &id,
                LocalOutcome::Interrupted,
                Some(UNEXPECTED_END),
                Some("system2WorkerEnded"),
            ),
        }
    }
}

const UNEXPECTED_END: &str =
    "The worker ended unexpectedly; the device state is uncertain. No automatic retry.";

/// A panic or abort of the worker inside this process. Settles once, from the durable journal
/// only (never a verified claim, since the run did not end under its own control), then
/// publishes the end. A process crash or runtime shutdown cannot run this and promises
/// nothing; GET, select and reopen never reach it.
async fn unexpected_end(
    state: &SharedState,
    root: &Arc<storage::Storage>,
    meta: &Metadata,
    run: Run,
) {
    let (copy, storage, metadata) = (state.clone(), root.clone(), meta.clone());
    let receipt = tokio::task::spawn_blocking(move || {
        let progress = match storage.reopen(&metadata.operation_id) {
            Ok((_, store)) => {
                let mut progress = store.progress();
                // Whatever the journal says, this run did not end by its own checkpoint.
                if progress.outcome == LocalOutcome::Finished {
                    progress.outcome = LocalOutcome::Interrupted;
                }
                progress
            }
            // Neither journal nor observed record can prove that nothing was written.
            Err(_) => journalless(),
        };
        settlement::settle(&copy, &metadata, &progress, run, |_, _| false)
    })
    .await
    .unwrap_or(Receipt::Unavailable);
    state
        .device_system2
        .publish_settlement(&meta.operation_id, receipt);
    state.device_system2.publish_end(
        &meta.operation_id,
        LocalOutcome::Interrupted,
        Some(UNEXPECTED_END),
        Some("system2WorkerEnded"),
    );
}

pub(super) async fn download(
    state: SharedState,
    root: Arc<storage::Storage>,
    preparation: Preparation,
    body: Start,
    gateway: SocketAddrV4,
    lease: System2OperationLease,
) {
    let id = preparation.metadata.operation_id.clone();
    let cancel = lease.cancel_token();
    observe!(state, begin(&preparation, &body.preparation_id));
    let meta = preparation.metadata.clone();
    let result = download_inner(&state, root, preparation, body, gateway, &cancel).await;
    // The refusal kind is a stable token already on the error; keep it for the
    // volatile status, never the message. Journal and settlement are untouched.
    let (outcome, clean, refusal, evidence) = match result {
        Ok((outcome, clean, evidence)) => (outcome, clean, None, evidence),
        Err(error) => (
            if cancel.load(Ordering::SeqCst) {
                LocalOutcome::Cancelled
            } else {
                LocalOutcome::Failed
            },
            true,
            error.kind(),
            None,
        ),
    };
    // Once, before the status becomes terminal: the client that sees the end and reloads
    // the project sees the settled belief. A refusal before any device contact has no
    // evidence and so no mutation to settle.
    let receipt = match evidence {
        Some((progress, bundle)) => {
            settle_off_runtime(&state, &meta, progress, Run::Download, Some(bundle)).await
        }
        None => Receipt::NoMutation,
    };
    state.device_system2.publish_settlement(&id, receipt);
    state.device_system2.publish_end(
        &id,
        outcome,
        if clean {
            None
        } else {
            Some("Tunnel disconnect unconfirmed; outcome remains uncertain. No automatic retry.")
        },
        refusal,
    );
    // A failed teardown/panic never certifies settled transport ownership.
    if clean {
        let _ = lease.settle(outcome);
    }
    observe!(state, terminal(outcome, clean));
}
async fn download_inner(
    state: &SharedState,
    root: Arc<storage::Storage>,
    preparation: Preparation,
    body: Start,
    gateway: SocketAddrV4,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<
    (
        LocalOutcome,
        bool,
        Option<(OperationProgress, PreparedBundle)>,
    ),
    ApiError,
> {
    observe!(state, phase("reservation"));
    let _guards = reserve(state).inspect_err(|_| {
        observe!(state, failure("reservation", "reservation-refused", None));
    })?;
    observe!(state, phase("cross-process-lease"));
    let _cross_process = cross_process(state).inspect_err(|_| {
        observe!(state, failure("cross-process-lease", "lease-refused", None));
    })?;
    observe!(state, phase("activity"));
    let activity = activity(state, &preparation.metadata.address).inspect_err(|_| {
        observe!(state, failure("activity", "activity-refused", None));
    })?;
    let copy = state.clone();
    let checked = preparation.clone();
    let storage = root.clone();
    #[cfg(test)]
    let observation = state.device_system2.observation.clone();
    observe!(state, phase("source-and-create"));
    if let Err(error) = blocking(move || {
        observe!(copy, phase("source-recheck"));
        check_source(&copy, &checked).inspect_err(|_| {
            observe!(copy, failure("source-recheck", "source-refused", None));
        })?;
        observe!(copy, phase("evidence-create"));
        storage.create(&checked.metadata).map_err(|_error| {
            observe!(copy, failure("evidence-create", "io", Some(_error.kind())));
            refuse("system2EvidenceUnavailable")
        })?;
        // Provenance only: a missing sidecar reports "unknown", it never stops the run.
        let _ = storage
            .record_execution_build(&checked.metadata.operation_id, &crate::version_string());
        Ok(())
    })
    .await
    {
        observe!(
            state,
            failure(
                "source-and-create",
                "blocking-or-source-evidence-refused",
                None
            )
        );
        finish_activity(activity, false, false);
        return Err(error);
    }
    if cancel.load(Ordering::SeqCst) || state.stop_admission.admit().is_err() {
        observe!(
            state,
            failure("admission", "cancelled-or-admission-refused", None)
        );
        finish_activity(activity, false, false);
        return Err(refuse("system2Cancelled"));
    }
    observe!(state, phase("connector"));
    let recorder = crate::operation_recordings::start(
        state,
        knx_app::operation_recording::OperationStart::new(
            knx_app::operation_recording::OperationKind::System2Download,
        )
        .target(recorded_target(&preparation.metadata))
        .gateway(gateway)
        .related("operationId", &preparation.metadata.operation_id)
        .settings(crate::operation_recordings::tunnel_settings(
            TunnelAckWait::Patient,
            &state.system2_timing,
        )),
    );
    let tunnel = match crate::operation_recordings::connect(
        state,
        gateway,
        TunnelAckWait::Patient,
        &recorder,
    )
    .await
    {
        Ok(t) => t,
        Err(_error) => {
            crate::operation_recordings::finish_error(&recorder, &_error, serde_json::Value::Null);
            observe!(state, session_failure("connector", &_error));
            finish_activity(activity, false, false);
            return Err(refuse("system2ConnectFailed"));
        }
    };
    let mut retained_store = None;
    let mut retained_admission = None;
    let result = async {
        observe!(state, phase("observe-hardware"));
        let observed = app::observe_project_hardware(
            &Transport {
                tunnel: tunnel.as_ref(),
                writes: None,
                #[cfg(test)]
                observation: observation.as_deref(),
            },
            preparation.source,
            state.system2_timing,
            cancel,
        )
        .await
        .map_err(|_error| {
            observe!(
                state,
                failure("observe-hardware", "io", Some(_error.kind()))
            );
            refuse("system2ObservationRefused")
        })?;
        let copy = state.clone();
        let metadata = preparation.metadata.clone();
        let storage = root.clone();
        observe!(state, phase("observed-evidence-and-admission"));
        let (admission, store) = blocking(move || {
            observe!(copy, phase("observed-evidence"));
            let store = storage
                .observed(&metadata.operation_id, &observed)
                .map_err(|_error| {
                    observe!(
                        copy,
                        failure("observed-evidence", "io", Some(_error.kind()))
                    );
                    refuse("system2EvidenceUnavailable")
                })?;
            // Evidence fsync/readback may take time. Rebuild the opaque current
            // project/source witness AFTER that disk work, immediately before
            // handing execution back to the async owner.
            observe!(copy, phase("current-admission"));
            let admission = admit(&copy, &metadata, &observed).inspect_err(|_| {
                observe!(
                    copy,
                    failure("current-admission", "admission-refused", None)
                );
            })?;
            Ok((admission, store))
        })
        .await
        .inspect_err(|_| {
            observe!(
                state,
                failure(
                    "observed-evidence-and-admission",
                    "blocking-or-evidence-admission-refused",
                    None
                )
            );
        })?;
        retained_store = Some(store);
        retained_admission = Some(admission);
        let store = retained_store.as_mut().unwrap();
        let admission = retained_admission.as_ref().unwrap();
        let updates = store.subscribe_progress();
        observe!(state, progress(&updates.borrow()));
        state
            .device_system2
            .publish_progress(&preparation.metadata.operation_id, updates.clone());
        {
            observe!(state, phase("binding-recheck"));
            let project = state.project.lock().map_err(|_| {
                observe!(
                    state,
                    failure("binding-recheck", "project-lock-unavailable", None)
                );
                refuse("system2Unavailable")
            })?;
            check_binding(state, &preparation.metadata.binding).inspect_err(|_| {
                observe!(state, failure("binding-recheck", "binding-stale", None));
            })?;
            drop(project);
        }
        observe!(state, phase("executor"));
        let result = off_tunnel_workers(app::execute_project_hardware(
            &Transport {
                tunnel: tunnel.as_ref(),
                writes: Some((&activity, updates)),
                #[cfg(test)]
                observation: observation.as_deref(),
            },
            admission,
            store,
            state.system2_timing,
            &body.confirmations(),
            cancel,
        ))
        .await;
        #[cfg(test)]
        if let Err(error) = &result {
            observe!(state, failure("executor", "io", Some(error.kind())));
        }
        if let Err(error) = &result {
            recorder.note("executorError", executor_error(error));
        }
        Ok::<_, ApiError>(result.is_ok())
    }
    .await;
    // No select/timeout can drop this disconnect or its guards. The Net client's
    // own protocol timeout bounds teardown; an error quarantines the coordinator.
    observe!(state, phase("disconnect"));
    let clean = tunnel
        .disconnect()
        .await
        .inspect_err(|_error| {
            observe!(state, session_failure("disconnect", _error));
        })
        .is_ok();
    #[cfg(test)]
    if result.is_err() {
        observe!(state, failure("worker-result", "api-refused", None));
    }
    let verified = *result.as_ref().unwrap_or(&false);
    let outcome = retained_store.as_ref().map(terminal).unwrap_or_else(|| {
        if cancel.load(Ordering::SeqCst) {
            LocalOutcome::Cancelled
        } else {
            LocalOutcome::Failed
        }
    });
    let evidence = retained_store
        .as_ref()
        .zip(retained_admission.as_ref())
        .map(|(store, admission)| (store.progress(), admission.bundle().clone()));
    finish_recording(
        &recorder,
        verified,
        &outcome,
        clean,
        result.as_ref().err(),
        evidence.as_ref().map(|(progress, _)| progress),
    );
    finish_activity(activity, verified, clean);
    Ok((outcome, clean, evidence))
}

// Keep original durable store and target independent of the current project.
#[allow(clippy::too_many_arguments)]
pub(super) async fn recover(
    state: SharedState,
    root: Arc<storage::Storage>,
    bundle: PreparedBundle,
    mut store: OperationStore,
    body: Recover,
    gateway: SocketAddrV4,
    lease: System2OperationLease,
) {
    let id = body.operation_id.clone();
    observe!(
        state,
        begin_recovery(&bundle, &store, &id, &state.server_incarnation)
    );
    #[cfg(test)]
    let observation = state.device_system2.observation.clone();
    let cancel = lease.cancel_token();
    let result = async {
        observe!(state, phase("reservation"));
        let _guards = reserve(&state).inspect_err(|_| {
            observe!(state, failure("reservation", "reservation-refused", None));
        })?;
        observe!(state, phase("cross-process-lease"));
        let _cross_process = cross_process(&state).inspect_err(|_| {
            observe!(state, failure("cross-process-lease", "lease-refused", None));
        })?;
        observe!(state, phase("activity"));
        let activity = activity(&state, &bundle.binding.target).inspect_err(|_| {
            observe!(state, failure("activity", "activity-refused", None));
        })?;
        let storage = root.clone();
        let selected = id.clone();
        #[cfg(test)]
        let copy = state.clone();
        observe!(state, phase("select-and-claim"));
        if let Err(e) = blocking(move || {
            observe!(copy, phase("select-original"));
            storage.select(&selected).map_err(|_error| {
                observe!(copy, failure("select-original", "io", Some(_error.kind())));
                refuse("system2EvidenceUnavailable")
            })?;
            observe!(copy, phase("claim-recovery"));
            storage.claim_recovery(&selected).map_err(|_error| {
                observe!(copy, failure("claim-recovery", "io", Some(_error.kind())));
                refuse("system2RecoveryConsumed")
            })
        })
        .await
        {
            observe!(
                state,
                failure("select-and-claim", "blocking-or-evidence-refused", None)
            );
            observe!(state, phase("activity-finish"));
            finish_activity(activity, false, false);
            return Err(e);
        }
        observe!(state, phase("admission-recheck"));
        if cancel.load(Ordering::SeqCst) || state.stop_admission.admit().is_err() {
            observe!(
                state,
                failure("admission-recheck", "cancelled-or-admission-closed", None)
            );
            observe!(state, phase("activity-finish"));
            finish_activity(activity, false, false);
            return Err(refuse("system2Cancelled"));
        }
        observe!(state, phase("connector"));
        let recorder = crate::operation_recordings::start(
            &state,
            knx_app::operation_recording::OperationStart::new(
                knx_app::operation_recording::OperationKind::System2Recovery,
            )
            .target(serde_json::json!({ "individualAddress": bundle.binding.target.to_string() }))
            .gateway(gateway)
            .related("operationId", &id)
            .settings(crate::operation_recordings::tunnel_settings(
                TunnelAckWait::Patient,
                &state.system2_timing,
            )),
        );
        let tunnel = match crate::operation_recordings::connect(
            &state,
            gateway,
            TunnelAckWait::Patient,
            &recorder,
        )
        .await
        {
            Ok(t) => t,
            Err(_error) => {
                crate::operation_recordings::finish_error(
                    &recorder,
                    &_error,
                    serde_json::Value::Null,
                );
                observe!(state, session_failure("connector", &_error));
                observe!(state, phase("activity-finish"));
                finish_activity(activity, false, false);
                return Err(refuse("system2ConnectFailed"));
            }
        };
        observe!(state, phase("progress-attachment"));
        let updates = store.subscribe_progress();
        state.device_system2.publish_progress(&id, updates.clone());
        observe!(state, phase("executor"));
        let result = if tunnel.diagnostics_are_simulated() {
            observe!(
                state,
                failure("executor", "simulated-transport-refused", None)
            );
            Err(std::io::Error::other(
                "physical recovery requires an actual hardware transport",
            ))
        } else {
            off_tunnel_workers(app::restore_project_hardware(
                &Transport {
                    tunnel: tunnel.as_ref(),
                    writes: Some((&activity, updates)),
                    #[cfg(test)]
                    observation: observation.as_deref(),
                },
                &bundle,
                &mut store,
                state.system2_timing,
                &body.confirmations(),
                &cancel,
            ))
            .await
        };
        #[cfg(test)]
        if let Err(error) = &result {
            observe!(state, failure("executor", "io", Some(error.kind())));
        }
        observe!(state, phase("disconnect"));
        let clean = tunnel
            .disconnect()
            .await
            .inspect_err(|_error| {
                observe!(state, session_failure("disconnect", _error));
            })
            .is_ok();
        if let Err(error) = &result {
            recorder.note("executorError", executor_error(error));
        }
        let outcome = terminal(&store);
        let progress = store.progress();
        finish_recording(
            &recorder,
            result.is_ok(),
            &outcome,
            clean,
            None,
            Some(&progress),
        );
        observe!(state, phase("activity-finish"));
        finish_activity(activity, result.is_ok(), clean);
        Ok::<_, ApiError>(clean)
    }
    .await;
    #[cfg(test)]
    if result.is_err() {
        observe!(state, failure("worker-result", "api-refused", None));
    }
    observe!(state, phase("end-publication"));
    let outcome = terminal(&store);
    let refusal = result.as_ref().err().and_then(ApiError::kind);
    // A recovery never earns "programmed"; if it reached the device it invalidates memory beliefs.
    let receipt = match root.metadata(&id) {
        Ok(metadata) => {
            settle_off_runtime(&state, &metadata, store.progress(), Run::Recovery, None).await
        }
        Err(_) => Receipt::Unavailable,
    };
    state.device_system2.publish_settlement(&id, receipt);
    state.device_system2.publish_end(&id, outcome, if matches!(result, Ok(true)) { None } else { Some("Recovery did not complete with confirmed teardown; inspect the original retained operation. No automatic retry.") }, refusal);
    // Release original store/bundle ownership only AFTER actual IP teardown.
    // Immutable evidence stays on disk; no timeout/HTTP/component can erase it.
    observe!(state, phase("release-original-ownership"));
    drop(store);
    drop(bundle);
    drop(root);
    // The store's original durable outcome, not an inferred rollback, settles.
    observe!(state, phase("settlement"));
    if !matches!(result, Ok(false)) {
        let _ = lease.settle(outcome).inspect_err(|_| {
            observe!(
                state,
                failure("settlement", "lease-settlement-refused", None)
            );
        });
    }
    observe!(state, terminal(outcome, matches!(result, Ok(true))));
}

/// The device a System-2 recording addresses, as the preparation bound it.
fn recorded_target(metadata: &dto::Metadata) -> serde_json::Value {
    serde_json::json!({
        "deviceId": metadata.binding.device_id,
        "individualAddress": metadata.address,
        "deviceName": metadata.device_name,
        "projectRevision": metadata.binding.project_revision,
    })
}

/// The executor's whole error for the recording; the journal keeps its own typed facts.
fn executor_error(error: &std::io::Error) -> serde_json::Value {
    serde_json::json!({
        "kind": format!("{:?}", error.kind()),
        "display": error.to_string(),
        "debug": format!("{error:?}"),
    })
}

/// Ends a System-2 recording with the run's local outcome and journal progress.
fn finish_recording(
    recorder: &knx_app::operation_recording::OperationRecorder,
    verified: bool,
    outcome: &LocalOutcome,
    clean: bool,
    refusal: Option<&ApiError>,
    progress: Option<&OperationProgress>,
) {
    let summary = serde_json::json!({
        "localOutcome": format!("{outcome:?}"),
        "verified": verified,
        "disconnectConfirmed": clean,
        "progress": progress.map(|p| serde_json::to_value(p).unwrap_or_default()),
    });
    if verified {
        recorder.finish(knx_app::operation_recording::RecordedOutcome::succeeded(
            summary,
        ));
    } else {
        let reason = match refusal {
            Some(refusal) => format!("refused: {refusal:?}"),
            None => format!("System-2 run ended {outcome:?}"),
        };
        crate::operation_recordings::finish_failed(recorder, reason, summary);
    }
}
