//! Server glue that gives every device-connection route one operation recording (DIAG-03).
//!
//! A route starts a recorder with [`start`], opens its tunnel through
//! [`connect`] so the real connector taps every datagram, adds notes, and
//! ends it with [`finish_with`]. Recording never changes the route: every
//! call returns at once and none can fail into the caller.

use std::net::SocketAddrV4;
use std::path::Path;

use knx_app::operation_recording::{
    error_chain, OperationRecorder, OperationStart, RecordedOutcome, RecordingStore, WriterIdentity,
};
use knx_net::TunnelAckWait;
use serde_json::{json, Value};

use crate::bus::{BusSessionError, BusTunnel};
use crate::domain::AppState;

/// This binary as the header's `writer`; the version is `GET /api/version`'s.
pub fn writer_identity() -> WriterIdentity {
    WriterIdentity {
        program: "knx-server".to_string(),
        version: crate::version_string(),
        commit: option_env!("KNX_BUILD_SHA")
            .filter(|sha| !sha.is_empty())
            .map(str::to_string),
        build_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
        .to_string(),
    }
}

/// The recording store of `data_dir`.
pub fn store(data_dir: &Path, disabled: bool) -> RecordingStore {
    RecordingStore::new(data_dir, writer_identity(), disabled)
}

/// Starts the recording of one operation of this server incarnation.
pub(crate) fn start(state: &AppState, mut start: OperationStart) -> OperationRecorder {
    // Some callers already hold project. Never deadlock or guess its provenance then.
    // Those callers may supply their already-validated context explicitly.
    if !start.related.contains_key("projectContext") {
        if let Ok(_project) = state.project.try_lock() {
            start = start.related(
                "projectContext",
                serde_json::to_string(&crate::diagnostic_bundle_context::ProjectContext::locked(
                    state,
                ))
                .expect("project context serializes"),
            );
        }
    }
    state
        .recordings
        .start(start.related("serverIncarnation", &state.server_incarnation))
}

/// The effective tunnel settings every recording header carries.
pub(crate) fn tunnel_settings(wait: TunnelAckWait, session: &knx_net::SessionTiming) -> Value {
    json!({
        "tunnelAckWaitMs": wait.timeout().as_millis() as u64,
        "session": format!("{session:?}"),
    })
}

/// Opens the route's tunnel with `wait`, tapped into `recorder`.
pub(crate) async fn connect(
    state: &AppState,
    gateway: SocketAddrV4,
    wait: TunnelAckWait,
    recorder: &OperationRecorder,
) -> Result<Box<dyn BusTunnel>, BusSessionError> {
    state
        .connector
        .connect_tunnel_recorded(gateway, wait, recorder.wire_tap())
        .await
}

/// Starts a recording with the tunnel settings filled in, then opens the
/// route's tunnel through it. A connection that fails ends the recording.
pub(crate) async fn open(
    state: &AppState,
    operation: OperationStart,
    gateway: SocketAddrV4,
    wait: TunnelAckWait,
    timing: &knx_net::SessionTiming,
) -> (
    OperationRecorder,
    Result<Box<dyn BusTunnel>, BusSessionError>,
) {
    let recorder = start(
        state,
        operation
            .gateway(gateway)
            .settings(tunnel_settings(wait, timing)),
    );
    let connected = connect(state, gateway, wait, &recorder).await;
    if let Err(error) = &connected {
        finish_error(&recorder, error, Value::Null);
    }
    (recorder, connected)
}

/// Ends the recording with `result`: succeeded with `summary`, or failed.
pub(crate) fn finish_result<T, E>(
    recorder: &OperationRecorder,
    result: &Result<T, E>,
    summary: Value,
) where
    E: std::fmt::Display + std::fmt::Debug,
{
    match result {
        Ok(_) => recorder.finish(RecordedOutcome::succeeded(summary)),
        Err(error) => finish_debug(recorder, error, summary),
    }
}

/// Ends the recording as failed with an error that is no `std::error::Error`:
/// its text, and its full `Debug` form for the bytes a text leaves out.
pub(crate) fn finish_debug<E>(recorder: &OperationRecorder, error: &E, summary: Value)
where
    E: std::fmt::Display + std::fmt::Debug,
{
    recorder.finish(RecordedOutcome::failed(
        vec![error.to_string(), format!("debug: {error:?}")],
        summary,
    ));
}

/// Ends the recording as failed with `error` and its source chain.
pub(crate) fn finish_error(
    recorder: &OperationRecorder,
    error: &(dyn std::error::Error + 'static),
    summary: Value,
) {
    recorder.finish(RecordedOutcome::failed(error_chain(error), summary));
}

/// Ends the recording as failed with a plain description.
pub(crate) fn finish_failed(
    recorder: &OperationRecorder,
    reason: impl Into<String>,
    summary: Value,
) {
    recorder.finish(RecordedOutcome::failed(vec![reason.into()], summary));
}
