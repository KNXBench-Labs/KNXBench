//! Authenticated thin routes; all network execution is process-owned.
use super::*;
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderMap, HeaderValue},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use dto::*;
use knx_app::system2_validation_preparation::{
    prepare_retained_project_hardware_source, validate_project_hardware_recovery, RecoveryTrack,
};
use knx_core::DeviceId;

pub(crate) fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/device-system2/prepare", post(prepare))
        .route("/api/device-system2/start", post(start))
        .route("/api/device-system2/status", get(status))
        .route("/api/device-system2/cancel", post(cancel))
        .route("/api/device-system2/recover", post(recover))
        .route("/api/device-system2/operations", get(operations))
        .route("/api/device-system2/select", post(select))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(
            |mut r: Response| async move {
                r.headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                r
            },
        ))
}
fn body<T: serde::de::DeserializeOwned>(headers: &HeaderMap, bytes: &[u8]) -> Result<T, ApiError> {
    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .is_none_or(|h| h.split(';').next().unwrap_or("").trim() != "application/json")
    {
        return Err(ApiError::bad_request("system2JsonRequired"));
    }
    decode(bytes)
}
async fn status(State(state): State<SharedState>) -> Result<Json<Option<Status>>, ApiError> {
    state.device_system2.status().map(Json)
}
async fn prepare(
    State(state): State<SharedState>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<Preview>, ApiError> {
    let body: Prepare = body(&headers, &bytes)?;
    let _source_mode = body.source_mode;
    state.stop_admission.admit()?;
    let _command = state
        .device_system2
        .command
        .try_lock()
        .map_err(|_| refuse("system2Busy"))?;
    state.device_system2.idle()?;
    state.device_system2.status()?;
    let copy = state.clone();
    let (retained, preview) = blocking(move || {
        copy.device_system2.root(&copy)?;
        let id = mint()?;
        let project = copy
            .project
            .lock()
            .map_err(|_| refuse("system2Unavailable"))?;
        let p = project
            .as_ref()
            .ok_or_else(|| refuse("system2ProjectRequired"))?;
        let devices: Vec<_> = p
            .devices
            .iter()
            .filter(|d| d.id == DeviceId(body.device_id))
            .collect();
        if devices.len() != 1 {
            return Err(refuse("system2DeviceRequired"));
        }
        let name = devices[0].name.clone();
        if name.len() > 4096 {
            return Err(refuse("system2PreviewTooLarge"));
        }
        let owner = copy
            .product_db
            .as_ref()
            .ok_or_else(|| refuse("system2RetainedSourcesRequired"))?;
        let source = owner
            .with_read(|conn| {
                prepare_retained_project_hardware_source(
                    conn,
                    p,
                    DeviceId(body.device_id),
                    &id,
                    RecoveryTrack::KnxBenchBestEffort,
                )
            })
            .map_err(|_| refuse("system2CatalogueUnavailable"))?
            .map_err(|_| refuse("system2PreparationRefused"))?;
        let summary = source
            .preview_summary()
            .map_err(|_| refuse("system2PreparationRefused"))?;
        let meta = Metadata {
            schema_version: 1,
            operation_id: id.clone(),
            binding: binding(&copy, body.device_id),
            address: summary.address.clone(),
            device_name: name.clone(),
        };
        let preview = Preview {
            schema_version: 1,
            preparation_id: id.clone(),
            binding: meta.binding.clone(),
            device_name: name,
            summary,
        };
        if serde_json::to_vec(&preview)
            .map_err(|_| refuse("system2Unavailable"))?
            .len()
            > 64 * 1024
        {
            return Err(refuse("system2PreviewTooLarge"));
        }
        Ok((
            Preparation {
                id,
                metadata: meta,
                source,
            },
            preview,
        ))
    })
    .await?;
    state.stop_admission.admit()?;
    let mut inner = state
        .device_system2
        .inner
        .lock()
        .map_err(|_| refuse("system2Unavailable"))?;
    check_binding(&state, &retained.metadata.binding)?;
    inner.preparation = Some(retained);
    Ok(Json(preview))
}
async fn start(
    State(state): State<SharedState>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<Status>, ApiError> {
    let body: Start = body(&headers, &bytes)?;
    if !valid_id(&body.preparation_id) {
        return Err(ApiError::bad_request("system2InvalidPreparation"));
    }
    let gateway = worker::gateway(&body.gateway)?;
    state.stop_admission.admit()?;
    let _command = state
        .device_system2
        .command
        .try_lock()
        .map_err(|_| refuse("system2Busy"))?;
    state.device_system2.idle()?;
    let mut preparation = state
        .device_system2
        .inner
        .lock()
        .map_err(|_| refuse("system2Unavailable"))?
        .preparation
        .as_ref()
        .filter(|p| p.id == body.preparation_id)
        .cloned()
        .ok_or_else(|| refuse("system2PreparationStale"))?;
    preparation.source = preparation
        .source
        .with_recovery_track(body.recovery_track.app());
    preparation
        .source
        .validate_confirmations(&body.confirmations())
        .map_err(|_| refuse("system2ConfirmationsRequired"))?;
    let copy = state.clone();
    let checked = preparation.clone();
    let root = blocking(move || {
        worker::check_source(&copy, &checked)?;
        copy.device_system2.root(&copy)
    })
    .await?;
    // Hold the existing guards during admission/publication, then the actual
    // worker reacquires them before connect. A competing session can only make
    // it refuse, never share a tunnel. No network future belongs to this request.
    let reservation = worker::reserve(&state)?;
    state.stop_admission.admit()?;
    check_binding(&state, &preparation.metadata.binding)?;
    state.device_system2.join_finished().await;
    let lease = state
        .device_system2
        .control
        .try_admit(&preparation.metadata.operation_id)
        .map_err(|_| refuse("system2Busy"))?;
    let meta = preparation.metadata.clone();
    let mut inner = state
        .device_system2
        .inner
        .lock()
        .map_err(|_| refuse("system2Unavailable"))?;
    inner.preparation = None;
    inner.current = Some(Record {
        metadata: meta.clone(),
        progress: initial(LocalOutcome::Running),
        updates: None,
        task: None,
        control_id: meta.operation_id.clone(),
        message: Some("Experimental configured download; cancellation is not rollback."),
    });
    drop(reservation);
    let copy = state.clone();
    let task = tokio::spawn(worker::download(
        copy,
        root,
        preparation,
        body,
        gateway,
        lease,
    ));
    inner.current.as_mut().unwrap().task = Some(task);
    let result = inner.current.as_ref().unwrap().view();
    drop(inner);
    Ok(Json(result))
}
async fn cancel(
    State(state): State<SharedState>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<Status>, ApiError> {
    let body: Operation = body(&headers, &bytes)?;
    if !valid_id(&body.operation_id) {
        return Err(ApiError::bad_request("system2InvalidOperation"));
    }
    let inner = state
        .device_system2
        .inner
        .lock()
        .map_err(|_| refuse("system2Unavailable"))?;
    let r = inner
        .current
        .as_ref()
        .filter(|r| r.metadata.operation_id == body.operation_id)
        .ok_or_else(|| refuse("system2OperationUnknown"))?;
    state
        .device_system2
        .control
        .request_cancel(&r.control_id)
        .map_err(|_| refuse("system2NotRunning"))?;
    Ok(Json(r.view()))
}
async fn recover(
    State(state): State<SharedState>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<Status>, ApiError> {
    let body: Recover = body(&headers, &bytes)?;
    if !valid_id(&body.operation_id) {
        return Err(ApiError::bad_request("system2InvalidOperation"));
    }
    let gateway = worker::gateway(&body.gateway)?;
    state.stop_admission.admit()?;
    let _command = state
        .device_system2
        .command
        .try_lock()
        .map_err(|_| refuse("system2Busy"))?;
    state.device_system2.idle()?;
    state.device_system2.join_finished().await;
    let copy = state.clone();
    let id = body.operation_id.clone();
    // Reopen the ORIGINAL evidence and validate every recovery phrase before
    // taking a tunnel. No current project/defaults participate in this path.
    let confirmations = (
        body.recovery_confirmation.clone(),
        body.download_confirmation.clone(),
        body.release_confirmation.clone(),
    );
    let (root, mut record, bundle, store) = blocking(move || {
        let root = copy.device_system2.root(&copy)?;
        root.require_unclaimed_recovery(&id)
            .map_err(|_| refuse("system2RecoveryConsumed"))?;
        let (bundle, store) = root
            .reopen(&id)
            .map_err(|_| refuse("system2RecoveryUnavailable"))?;
        validate_project_hardware_recovery(
            &bundle,
            &store,
            &knx_app::system2_validation_preparation::HardwareRecoveryConfirmations {
                recovery: &confirmations.0,
                download: &confirmations.1,
                risk: &confirmations.2,
            },
        )
        .map_err(|_| refuse("system2RecoveryRefused"))?;
        let record = Record {
            metadata: root
                .metadata(&id)
                .map_err(|_| refuse("system2RecoveryUnavailable"))?,
            progress: store.progress(),
            updates: None,
            task: None,
            control_id: format!("{id}-restore"),
            message: None,
        };
        Ok((root, record, bundle, store))
    })
    .await?;
    let reservation = worker::reserve(&state)?;
    state.stop_admission.admit()?;
    let lease = state
        .device_system2
        .control
        .try_admit(&record.control_id)
        .map_err(|_| refuse("system2Busy"))?;
    record.progress.outcome = LocalOutcome::Restoring;
    record.progress.recovery_phrases = None;
    let mut inner = state
        .device_system2
        .inner
        .lock()
        .map_err(|_| refuse("system2Unavailable"))?;
    let copy = state.clone();
    drop(reservation);
    record.task = Some(tokio::spawn(worker::recover(
        copy, root, bundle, store, body, gateway, lease,
    )));
    inner.current = Some(record);
    inner.preparation = None;
    inner.selection_unavailable = false;
    let result = inner.current.as_ref().unwrap().view();
    drop(inner);
    Ok(Json(result))
}
/// Explicit historical operation discovery/selection is local only. The UI's
/// status attaches the durable selection; recovery itself supplies an exact id.
async fn operations(State(state): State<SharedState>) -> Result<Json<Vec<Status>>, ApiError> {
    let _command = state
        .device_system2
        .command
        .try_lock()
        .map_err(|_| refuse("system2Busy"))?;
    state.device_system2.idle()?;
    let copy = state.clone();
    blocking(move || {
        let root = copy.device_system2.root(&copy)?;
        let rows = root
            .ids()
            .map_err(|_| refuse("system2EvidenceUnavailable"))?
            .into_iter()
            .map(|id| load_record(&root, &id).map(|r| r.view()))
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|_| refuse("system2EvidenceUnavailable"))?;
        if serde_json::to_vec(&rows)
            .map_err(|_| refuse("system2Unavailable"))?
            .len()
            > 256 * 1024
        {
            return Err(refuse("system2StatusTooLarge"));
        }
        Ok(Json(rows))
    })
    .await
}
async fn select(
    State(state): State<SharedState>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<Status>, ApiError> {
    let body: Operation = body(&headers, &bytes)?;
    if !valid_id(&body.operation_id) {
        return Err(ApiError::bad_request("system2InvalidOperation"));
    }
    let _command = state
        .device_system2
        .command
        .try_lock()
        .map_err(|_| refuse("system2Busy"))?;
    state.device_system2.idle()?;
    state.device_system2.join_finished().await;
    let copy = state.clone();
    let record = blocking(move || {
        let root = copy.device_system2.root(&copy)?;
        let record = load_record(&root, &body.operation_id)
            .map_err(|_| refuse("system2RecoveryUnavailable"))?;
        root.select(&body.operation_id)
            .map_err(|_| refuse("system2EvidenceUnavailable"))?;
        Ok(record)
    })
    .await?;
    let result = record.view();
    let mut inner = state
        .device_system2
        .inner
        .lock()
        .map_err(|_| refuse("system2Unavailable"))?;
    inner.current = Some(record);
    inner.preparation = None;
    inner.selection_unavailable = false;
    Ok(Json(result))
}
