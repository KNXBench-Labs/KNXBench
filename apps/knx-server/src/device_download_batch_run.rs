//! The batch run: starts each device exactly as the single start does and stops at the first doubt.
//!
//! One task per batch (ADR-0172 §4–§6). For every device in order it takes
//! the reservation, registers the device's stop token and calls the single
//! start's own [`launch`]: prepared again, compared with the reviewed plan,
//! history start, its own tunnel, backup, settlement. It then waits for the
//! device's run to end and judges it ([`judge`]). Only a device that met every
//! condition lets the next one start; anything else, a stop request or a
//! closing server ends the batch with the rest `notStarted`. Between devices
//! the batch itself holds the reservation
//! ([`crate::device_download_batch::busy_except`]).

use std::net::SocketAddrV4;
use std::sync::Arc;
use std::time::Duration;

use crate::device_download::Written;
use crate::device_download_batch::{
    judge, DeviceState, DownloadBatch, NotStartedReason, RunDevice,
};
use crate::device_download_routes::{authorise, launch, lock_tunnel, DeviceStart, Expected};
use crate::errors::ApiError;
use crate::SharedState;

/// How often the run looks whether the device in progress has ended.
const END_POLL: Duration = Duration::from_millis(20);

/// Starts the run in the background; it owns the batch's end.
pub(crate) fn spawn(
    state: SharedState,
    batch: Arc<DownloadBatch>,
    gateway: SocketAddrV4,
    devices: Vec<RunDevice>,
) {
    tokio::spawn(run(state, batch, gateway, devices));
}

/// Marks the batch ended however the run leaves, a panic included: a batch
/// that no longer runs never keeps the reservation.
struct EndOnDrop(Arc<DownloadBatch>);

impl Drop for EndOnDrop {
    fn drop(&mut self) {
        if self.0.is_running() {
            self.0.end(NotStartedReason::RunEnded);
        }
    }
}

async fn run(
    state: SharedState,
    batch: Arc<DownloadBatch>,
    gateway: SocketAddrV4,
    devices: Vec<RunDevice>,
) {
    let _end = EndOnDrop(Arc::clone(&batch));
    for device in &devices {
        if let Some(reason) = before_next(&state, &batch) {
            batch.end(reason);
            return;
        }
        let (result, go_on) = one_device(&state, &batch, gateway, device).await;
        batch.set(device.plan.id, result);
        if !go_on {
            batch.end(after_doubt(&state));
            return;
        }
    }
    batch.end(NotStartedReason::RunEnded);
}

/// The reason not to start another device, if there is one.
fn before_next(state: &SharedState, batch: &DownloadBatch) -> Option<NotStartedReason> {
    if state.stop_admission.is_closed() {
        Some(NotStartedReason::ServerStop)
    } else if batch.stop_requested() {
        Some(NotStartedReason::StopRequest)
    } else {
        None
    }
}

/// The rest's reason after a device that did not qualify: a closing server
/// is named, since it may be why that device stopped.
fn after_doubt(state: &SharedState) -> NotStartedReason {
    if state.stop_admission.is_closed() {
        NotStartedReason::ServerStop
    } else {
        NotStartedReason::EarlierDevice
    }
}

/// A start that ended before the device's download ran: nothing was written.
fn refused(download_id: u64, error: &ApiError) -> DeviceState {
    DeviceState::Failed {
        download_id,
        written: Written::No,
        stopped_in_step: None,
        error: error.message().to_string(),
        hint: None,
        kind: error.kind(),
        backup_file: None,
    }
}

/// One device, from the reservation to its judged end.
async fn one_device(
    state: &SharedState,
    batch: &DownloadBatch,
    gateway: SocketAddrV4,
    device: &RunDevice,
) -> (DeviceState, bool) {
    let id = device.plan.id;
    let locks = match lock_tunnel(state, Some(batch.id)).await {
        Ok(locks) => locks,
        Err(error) => return (refused(id, &error), false),
    };
    // As the single start: registered under the reservation, before any
    // history, tunnel or spawn (ADR-0153 §5).
    let stop = match state.stop_admission.register_download() {
        Ok(stop) => stop,
        Err(_) => {
            return (
                DeviceState::NotStarted {
                    download_id: id,
                    reason: NotStartedReason::ServerStop,
                },
                false,
            )
        }
    };
    // Checked before the batch started; checked again as the single start does.
    let authorisation = match authorise(device.plan.target, &device.confirmation) {
        Ok(authorisation) => authorisation,
        Err(error) => return (refused(id, &error), false),
    };
    batch.set(id, DeviceState::Running { download_id: id });
    let started = launch(
        state,
        locks,
        stop,
        DeviceStart {
            id,
            target: device.plan.target,
            partial: device.plan.partial,
            changes: true,
            gateway,
            authorisation,
            accept_untested: device.accept_untested.as_deref(),
            expected: Expected::Reviewed(&device.plan.plan),
            batch: Some(batch.id),
        },
    )
    .await;
    if let Err(error) = started {
        return (refused(id, &error), false);
    }
    let (status, backup_file) = wait_for_end(state, id).await;
    judge(id, status, backup_file)
}

/// Waits until the download `id` has ended, cleanup included, and returns
/// its final status and backup file (relative to the data directory).
async fn wait_for_end(state: &SharedState, id: u64) -> (crate::DownloadStatus, Option<String>) {
    loop {
        {
            let guard = state.device_download.lock().await;
            match guard.as_ref() {
                Some(session) if session.id() == id => {
                    if !session.is_running() {
                        let backup = session.backup_file().map(|path| {
                            path.strip_prefix(&state.data_dir)
                                .unwrap_or(&path)
                                .display()
                                .to_string()
                        });
                        return (session.status(), backup);
                    }
                }
                // The batch holds the reservation, so nothing replaces its
                // session; if something did, its outcome is not known here.
                _ => return (crate::DownloadStatus::Running, None),
            }
        }
        tokio::time::sleep(END_POLL).await;
    }
}
