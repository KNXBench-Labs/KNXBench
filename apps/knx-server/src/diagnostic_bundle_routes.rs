//! Diagnostic bundle routes: recordings, preview, section text, consented ZIP export (DIAG-03).
//!
//! `GET/DELETE /api/diagnostics/recordings` list and delete the operation
//! recordings; `POST /api/diagnostics/bundle/preview` collects the sections a
//! problem type offers and shows size, entries and content markers;
//! `POST /api/diagnostics/bundle/section` returns one section as text; and
//! `POST /api/diagnostics/bundle` writes the ZIP only with `consent: true`.
//! Every route is read-only towards devices: nothing here opens a tunnel.

use std::sync::atomic::Ordering;
use std::time::Duration;

use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use knx_app::diagnostic_bundle::{
    self as bundle, BundleFile, BundleRequest, BundleSection, ContentMarkers, Offer, ProblemType,
    SectionKind, SectionPreview,
};
use knx_app::operation_recording::RecordingSummary;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::SharedState;

/// The bus monitor rows a bundle may carry: the last ten minutes (decision 14).
const BUS_MONITOR_WINDOW: chrono::Duration = chrono::Duration::minutes(10);
/// The client may describe itself; anything larger is refused.
const MAX_CLIENT_STATE_BYTES: usize = 64 * 1024;
/// The user's own words; anything longer is refused.
const MAX_DESCRIPTION_CHARS: usize = 8000;

pub(crate) fn routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/api/diagnostics/recordings",
            get(list_recordings).delete(delete_recordings),
        )
        .route("/api/diagnostics/bundle/preview", post(preview))
        .route("/api/diagnostics/bundle/section", post(section_text))
        .route("/api/diagnostics/bundle", post(export))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordingsView {
    /// `false`: the operator disabled recording; listed files are older.
    enabled: bool,
    recordings: Vec<RecordingSummary>,
}

async fn list_recordings(
    State(state): State<SharedState>,
) -> Result<Json<RecordingsView>, ApiError> {
    let store = state.recordings.clone();
    let recordings = tokio::task::spawn_blocking(move || store.list())
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map_err(|e| ApiError::internal(format!("recordings cannot be listed: {e}")))?;
    Ok(Json(RecordingsView {
        enabled: state.recordings.is_enabled(),
        recordings,
    }))
}

async fn delete_recordings(
    State(state): State<SharedState>,
) -> Result<Json<knx_app::operation_recording::DeleteOutcome>, ApiError> {
    let store = state.recordings.clone();
    tokio::task::spawn_blocking(move || store.delete_all())
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map(Json)
        .map_err(|e| ApiError::internal(format!("recordings cannot be deleted: {e}")))
}

/// Which operation a shortcut means: a recording id, or an identity the
/// operation already has elsewhere (`downloadId`, `operationId`, ...).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OperationRef {
    #[serde(default)]
    recording_id: Option<String>,
    #[serde(default)]
    related_key: Option<String>,
    #[serde(default)]
    related_value: Option<String>,
}

/// One wizard state: problem type, chosen operations and sections.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BundleQuery {
    problem_type: ProblemType,
    /// The operation the report is about; `None`: the latest that fits.
    #[serde(default)]
    operation: Option<OperationRef>,
    /// Further recordings the user picked (earlier operations).
    #[serde(default)]
    earlier_recording_ids: Vec<String>,
    /// The affected device; `None`: the operation's device, if it names one.
    #[serde(default)]
    device_id: Option<u32>,
    /// Ticked sections; `None`: the problem type's defaults.
    #[serde(default)]
    sections: Option<Vec<SectionKind>>,
    /// What the browser says about itself (language, theme, viewport, errors).
    #[serde(default)]
    client: Option<Value>,
    #[serde(default)]
    description: Option<String>,
    /// Export only: the user's explicit consent.
    #[serde(default)]
    consent: bool,
    /// Section text only: which section.
    #[serde(default)]
    key: Option<String>,
}

/// A section the problem type offers, ticked or not, and whether it exists.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OfferView {
    kind: SectionKind,
    offer: Offer,
    selected: bool,
    /// `None`: available. Otherwise why it cannot be included.
    unavailable: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewView {
    problem_type: ProblemType,
    recording_enabled: bool,
    operation_recording_id: Option<String>,
    device_id: Option<u32>,
    offers: Vec<OfferView>,
    sections: Vec<SectionPreview>,
    total_bytes: u64,
    /// Above common mail limits; the wizard says so.
    exceeds_mail_hint: bool,
    file_name: String,
    mail_draft: String,
    recordings: Vec<RecordingSummary>,
}

/// Everything one wizard state resolves to.
struct Collected {
    operation: Option<RecordingSummary>,
    device_id: Option<u32>,
    offers: Vec<OfferView>,
    sections: Vec<BundleSection>,
    recordings: Vec<RecordingSummary>,
    created_at: String,
}

fn check_query(query: &BundleQuery) -> Result<(), ApiError> {
    if let Some(description) = &query.description {
        if description.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(ApiError::bad_request("bundleDescriptionTooLong"));
        }
    }
    if let Some(client) = &query.client {
        if !client.is_object() || client.to_string().len() > MAX_CLIENT_STATE_BYTES {
            return Err(ApiError::bad_request("bundleClientStateRefused"));
        }
    }
    for id in query.earlier_recording_ids.iter().chain(
        query
            .operation
            .as_ref()
            .and_then(|o| o.recording_id.as_ref()),
    ) {
        if !knx_app::operation_recording::is_operation_id(id) {
            return Err(ApiError::bad_request("bundleRecordingUnknown"));
        }
    }
    Ok(())
}

fn kind_fits(problem: ProblemType, kind: Option<&str>) -> bool {
    let Some(kind) = kind else {
        return false;
    };
    match problem {
        ProblemType::DeviceProgramming => matches!(
            kind,
            "download"
                | "system2-download"
                | "system2-recovery"
                | "individual-address-programming"
                | "serial-number-address"
                | "individual-address-reset"
                | "service-control"
        ),
        ProblemType::DeviceReading => matches!(
            kind,
            "device-diagnostics"
                | "device-read"
                | "compare"
                | "support-read"
                | "line-scan"
                | "byte-order-probe"
        ),
        _ => true,
    }
}

/// The recording `operation` names, else the newest one of this problem's kind.
fn resolve_operation(
    query: &BundleQuery,
    recordings: &[RecordingSummary],
    server_incarnation: &str,
) -> Result<Option<RecordingSummary>, ApiError> {
    let Some(reference) = &query.operation else {
        return Ok(recordings
            .iter()
            .find(|r| !r.active && kind_fits(query.problem_type, r.operation_kind.as_deref()))
            .cloned());
    };
    let found = match (
        &reference.recording_id,
        &reference.related_key,
        &reference.related_value,
    ) {
        (Some(id), None, None) => recordings.iter().find(|r| &r.id == id),
        (None, Some(key), Some(value)) => recordings.iter().find(|r| {
            r.related[key.as_str()].as_str() == Some(value.as_str())
                // Download ids restart with every server run.
                && (key != "downloadId"
                    || r.related["serverIncarnation"].as_str() == Some(server_incarnation))
        }),
        _ => return Err(ApiError::bad_request("bundleOperationReferenceRefused")),
    };
    Ok(found.cloned())
}

fn markers(addresses: bool, names: bool, parameter_values: bool, network: bool) -> ContentMarkers {
    ContentMarkers {
        addresses,
        names,
        parameter_values,
        network_addresses: network,
    }
}

async fn collect(state: &SharedState, query: &BundleQuery) -> Result<Collected, ApiError> {
    check_query(query)?;
    let store = state.recordings.clone();
    let recordings = tokio::task::spawn_blocking(move || {
        // A recording still being written shows its last lines a moment later.
        store.wait_until_idle(Duration::from_millis(500));
        store.list()
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?
    .map_err(|e| ApiError::internal(format!("recordings cannot be listed: {e}")))?;
    let problem = query.problem_type;
    let operation = if bundle::default_offer(problem, SectionKind::Operation) == Offer::NotOffered {
        None
    } else {
        resolve_operation(query, &recordings, &state.server_incarnation)?
    };
    let device_id = query.device_id.or_else(|| {
        operation
            .as_ref()
            .and_then(|op| op.target["deviceId"].as_u64())
            .and_then(|id| u32::try_from(id).ok())
    });
    let selected: Vec<SectionKind> = match &query.sections {
        Some(chosen) => chosen.clone(),
        None => SectionKind::ALL
            .into_iter()
            .filter(|kind| {
                matches!(
                    bundle::default_offer(problem, *kind),
                    Offer::Fixed | Offer::Preselected
                )
            })
            .collect(),
    };
    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut offers = Vec::new();
    let mut sections = Vec::new();
    for kind in SectionKind::ALL {
        let offer = bundle::default_offer(problem, kind);
        if offer == Offer::NotOffered {
            if selected.contains(&kind) {
                return Err(ApiError::bad_request("bundleSectionNotOffered"));
            }
            continue;
        }
        let wanted = offer == Offer::Fixed || selected.contains(&kind);
        let built = build_section(state, query, kind, operation.as_ref(), device_id).await;
        let unavailable = match built {
            Ok(built) => {
                if wanted {
                    sections.extend(built);
                }
                None
            }
            Err(reason) => Some(reason),
        };
        offers.push(OfferView {
            kind,
            offer,
            selected: wanted && unavailable.is_none(),
            unavailable,
        });
    }
    Ok(Collected {
        operation,
        device_id,
        offers,
        sections,
        recordings,
        created_at,
    })
}

/// Builds the sections of `kind`, or says why there are none.
async fn build_section(
    state: &SharedState,
    query: &BundleQuery,
    kind: SectionKind,
    operation: Option<&RecordingSummary>,
    device_id: Option<u32>,
) -> Result<Vec<BundleSection>, String> {
    match kind {
        SectionKind::Environment => Ok(vec![environment(state)]),
        SectionKind::SessionLog => Ok(vec![session_log(state)]),
        SectionKind::Operation => {
            let operation = operation.ok_or("no recorded operation fits this problem")?;
            Ok(vec![operation_section(state, operation, SectionKind::Operation).await?])
        }
        SectionKind::EarlierOperation => {
            let mut out = Vec::new();
            for id in &query.earlier_recording_ids {
                if operation.is_some_and(|op| &op.id == id) {
                    continue;
                }
                let summary = state
                    .recordings
                    .list()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .find(|r| &r.id == id)
                    .ok_or("a chosen earlier recording no longer exists")?;
                out.push(operation_section(state, &summary, SectionKind::EarlierOperation).await?);
            }
            Ok(out)
        }
        SectionKind::Device => Ok(vec![device_section(state, device_id)?]),
        SectionKind::LegacyReports => {
            let operation = operation.ok_or("no recorded operation fits this problem")?;
            Ok(vec![legacy_reports(state, operation).await?])
        }
        SectionKind::ImportReport => Ok(vec![import_report(state)?]),
        SectionKind::ImportFile => Err(
            "this server session does not keep the original import file; attach it yourself if needed"
                .into(),
        ),
        SectionKind::Catalogue => Ok(vec![catalogue(state, device_id)]),
        SectionKind::Project => Ok(vec![project_section(state).await?]),
        SectionKind::BusMonitor => Ok(vec![bus_monitor(state).await?]),
        SectionKind::ClientState => {
            let client = query
                .client
                .as_ref()
                .ok_or("the browser did not describe itself")?;
            let mut section = BundleSection::new(SectionKind::ClientState);
            section.files.push(BundleFile::json("client.json", client));
            section.entries = client["recentErrors"].as_array().map_or(0, Vec::len);
            Ok(vec![section])
        }
    }
}

fn environment(state: &SharedState) -> BundleSection {
    let identity = crate::operation_recordings::writer_identity();
    let project_open = state
        .project
        .lock()
        .expect("state mutex poisoned")
        .is_some();
    let saved = state
        .store_path
        .lock()
        .expect("state mutex poisoned")
        .is_some();
    let value = json!({
        "writer": identity,
        "platform": {
            "os": std::env::consts::OS,
            "family": std::env::consts::FAMILY,
            "arch": std::env::consts::ARCH,
            "container": cfg!(target_os = "linux")
                && ["/.dockerenv", "/run/.containerenv"]
                    .iter()
                    .any(|m| std::path::Path::new(m).exists()),
        },
        "serverIncarnation": state.server_incarnation,
        "recording": {
            "enabled": state.recordings.is_enabled(),
            "disableVariable": knx_app::operation_recording::DISABLE_ENV,
        },
        "settings": {
            "deviceDownloadTiming": format!("{:?}", state.device_download_timing),
            "system2Timing": format!("{:?}", state.system2_timing),
            "addressProgrammingTiming": format!("{:?}", state.address_programming_timing),
            "tunnelRouteBack": std::env::var("KNX_TUNNEL_ROUTE_BACK").ok(),
        },
        "productDatabase": {
            "open": state.product_db.is_some(),
            "unavailableReason": state.product_db_unavailable,
        },
        "project": {
            "open": project_open,
            "saved": saved,
            "incarnation": state.project_incarnation.load(Ordering::Relaxed),
            "revision": state.project_revision.load(Ordering::Relaxed),
        },
    });
    let mut section = BundleSection::new(SectionKind::Environment);
    section
        .files
        .push(BundleFile::json("environment.json", &value));
    section.entries = 1;
    section
}

fn session_log(state: &SharedState) -> BundleSection {
    let log = state.session_log.lock().expect("state mutex poisoned");
    let entries = log.entries();
    let mut section = BundleSection::new(SectionKind::SessionLog);
    section.entries = entries.len();
    section
        .files
        .push(BundleFile::json("session-log.json", &json!(entries)));
    section.markers = markers(true, true, false, true);
    section
}

async fn operation_section(
    state: &SharedState,
    operation: &RecordingSummary,
    kind: SectionKind,
) -> Result<BundleSection, String> {
    let store = state.recordings.clone();
    let id = operation.id.clone();
    let bytes = tokio::task::spawn_blocking(move || store.read(&id))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("the recording cannot be read: {e}"))?;
    let mut section = BundleSection::new(kind);
    section.label = Some(operation.id.clone());
    section.entries = bytes.iter().filter(|b| **b == b'\n').count();
    section.markers = markers(true, true, true, true);
    if operation.torn_tail {
        section
            .notes
            .push("the recording's last line is incomplete (interrupted write)".into());
    }
    section
        .files
        .push(BundleFile::new("recording.jsonl", bytes));
    if let Some(system2) = operation.related["operationId"].as_str() {
        match crate::device_system2::diagnostic_projection(state, system2.to_string()).await {
            Ok(Some((progress, _report))) => section
                .files
                .push(BundleFile::json("system2-progress.json", &progress)),
            Ok(None) => section
                .notes
                .push("the System-2 journal of this operation no longer exists".into()),
            Err(reason) => section
                .notes
                .push(format!("System-2 journal unavailable: {reason}")),
        }
    }
    if let Some(status) = current_download(state, operation, |session| {
        serde_json::to_value(session.status()).unwrap_or_default()
    })
    .await
    {
        section
            .files
            .push(BundleFile::json("download-status.json", &status));
    }
    Ok(section)
}

/// `read` applied to the server's download session when it is this
/// recording's run (same id, same server incarnation).
async fn current_download<T>(
    state: &SharedState,
    operation: &RecordingSummary,
    read: impl FnOnce(&crate::DeviceDownloadSession) -> T,
) -> Option<T> {
    let id: u64 = operation.related["downloadId"].as_str()?.parse().ok()?;
    if operation.related["serverIncarnation"].as_str() != Some(state.server_incarnation.as_str()) {
        return None;
    }
    let guard = state.device_download.try_lock().ok()?;
    let session = guard.as_ref().filter(|s| s.id() == id)?;
    Some(read(session))
}

async fn legacy_reports(
    state: &SharedState,
    operation: &RecordingSummary,
) -> Result<BundleSection, String> {
    let mut section = BundleSection::new(SectionKind::LegacyReports);
    if let Some(Some(report)) = current_download(state, operation, |session| session.report()).await
    {
        section.files.push(BundleFile::json(
            "download-report.json",
            &serde_json::to_value(&report).unwrap_or_default(),
        ));
    }
    if let Some(system2) = operation.related["operationId"].as_str() {
        if let Ok(Some((_, report))) =
            crate::device_system2::diagnostic_projection(state, system2.to_string()).await
        {
            section
                .files
                .push(BundleFile::json("system2-report.json", &report));
        }
    }
    if section.files.is_empty() {
        return Err("no minimised report exists for this operation".into());
    }
    section.entries = section.files.len();
    section.markers = markers(false, false, true, false);
    Ok(section)
}

fn device_section(state: &SharedState, device_id: Option<u32>) -> Result<BundleSection, String> {
    let id = device_id.ok_or("no device is chosen")?;
    let detail = crate::domain::device_detail(state, id, None)?;
    let parameters = crate::domain::parameter_panel_impl(state, id, None).ok();
    let mut section = BundleSection::new(SectionKind::Device);
    section.label = Some(id.to_string());
    section.files.push(BundleFile::json(
        "device.json",
        &serde_json::to_value(&detail).unwrap_or_default(),
    ));
    match parameters {
        Some(parameters) => {
            let value = serde_json::to_value(&parameters).unwrap_or_default();
            section.entries = count_leaves(&value);
            section
                .files
                .push(BundleFile::json("parameters.json", &value));
        }
        None => section
            .notes
            .push("the parameter values could not be projected for this device".into()),
    }
    section.markers = markers(true, true, true, false);
    Ok(section)
}

fn count_leaves(value: &Value) -> usize {
    match value {
        Value::Array(items) => items.iter().map(count_leaves).sum(),
        Value::Object(map) => map.values().map(count_leaves).sum(),
        _ => 1,
    }
}

fn import_report(state: &SharedState) -> Result<BundleSection, String> {
    let project = state.project.lock().expect("state mutex poisoned");
    project.as_ref().ok_or("no project is open")?;
    let opaque = state.opaque.lock().expect("state mutex poisoned");
    let evidence =
        knx_app::import_evidence::read(&opaque)?.ok_or("this project carries no import report")?;
    let copy = knx_app::secret_free_copy::import_report_without_secrets(&evidence.report)
        .map_err(|error| error.to_string())?;
    let mut section = BundleSection::new(SectionKind::ImportReport);
    for redaction in &copy.redactions {
        section.notes.push(format!(
            "removed {} at {}",
            redaction.kind, redaction.location
        ));
    }
    for withheld in &copy.withheld {
        section.notes.push(format!("left out: {withheld}"));
    }
    section
        .files
        .push(BundleFile::new("import-report.json", copy.bytes));
    section.entries = 1;
    section.markers = markers(true, true, false, false);
    Ok(section)
}

fn catalogue(state: &SharedState, device_id: Option<u32>) -> BundleSection {
    let product = device_id
        .and_then(|id| crate::domain::device_detail(state, id, None).ok())
        .map(|detail| serde_json::to_value(&detail).unwrap_or_default())
        .map(|detail| {
            json!({
                "application": detail.get("application").cloned().unwrap_or(Value::Null),
                "product": detail.get("product").cloned().unwrap_or(Value::Null),
                "manufacturer": detail.get("manufacturer").cloned().unwrap_or(Value::Null),
            })
        });
    let mut section = BundleSection::new(SectionKind::Catalogue);
    section.files.push(BundleFile::json(
        "catalogue.json",
        &json!({
            "productDatabaseOpen": state.product_db.is_some(),
            "unavailableReason": state.product_db_unavailable,
            "deviceProductIdentity": product,
        }),
    ));
    section.entries = 1;
    section
}

async fn project_section(state: &SharedState) -> Result<BundleSection, String> {
    let path = state
        .store_path
        .lock()
        .expect("state mutex poisoned")
        .clone()
        .ok_or("the project has not been saved as a KNXBench project file yet")?;
    let copy = tokio::task::spawn_blocking(move || {
        crate::diagnostic_bundle_routes::secret_free_project(&path)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("a secret-free copy of the project could not be made: {e}"))?;
    let mut section = BundleSection::new(SectionKind::Project);
    section.entries = 1;
    section.markers = markers(true, true, true, true);
    section
        .notes
        .push("the last saved state of the project; unsaved changes are not included".into());
    for redaction in &copy.redactions {
        section.notes.push(format!(
            "removed {} at {}",
            redaction.kind, redaction.location
        ));
    }
    for withheld in &copy.withheld {
        section.notes.push(format!("left out: {withheld}"));
    }
    section
        .files
        .push(BundleFile::new("project.knxdb", copy.bytes));
    Ok(section)
}

async fn bus_monitor(state: &SharedState) -> Result<BundleSection, String> {
    let guard = state.bus_session.lock().await;
    let session = guard.as_ref().ok_or("the bus monitor is not running")?;
    let since = chrono::Utc::now() - BUS_MONITOR_WINDOW;
    let rows: Vec<Value> = {
        let buffer = session.buffer();
        let buffer = buffer.lock().expect("bus session buffer poisoned");
        buffer
            .telegrams_since(0)
            .iter()
            .filter(|row| {
                chrono::DateTime::parse_from_rfc3339(&row.timestamp)
                    .map(|at| at >= since)
                    .unwrap_or(true)
            })
            .map(crate::debug_report_routes::telegram_json)
            .collect()
    };
    let mut section = BundleSection::new(SectionKind::BusMonitor);
    section.entries = rows.len();
    section
        .files
        .push(BundleFile::json("bus-monitor.json", &json!(rows)));
    section.markers = markers(true, false, false, false);
    Ok(section)
}

/// Placeholder until the secret-free project copy lands: refuses, never copies.
fn secret_free_project(_path: &std::path::Path) -> Result<SecretFreeStub, String> {
    Err("not yet available in this build".into())
}

struct SecretFreeStub {
    bytes: Vec<u8>,
    redactions: Vec<StubRedaction>,
    withheld: Vec<String>,
}

struct StubRedaction {
    kind: String,
    location: String,
}

fn request(state: &SharedState, query: &BundleQuery) -> BundleRequest {
    BundleRequest {
        problem: query.problem_type,
        writer: crate::operation_recordings::writer_identity(),
        recording_enabled: state.recordings.is_enabled(),
        consent_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        description: query.description.clone(),
    }
}

fn names(collected: &Collected, query: &BundleQuery) -> (String, String) {
    let version = crate::version_string();
    let operation = collected.operation.as_ref().map(|op| op.id.as_str());
    let file_name = bundle::file_name(&version, operation, &collected.created_at);
    let draft = bundle::mail_draft(&version, query.problem_type, operation, &file_name);
    (file_name, draft)
}

async fn preview(
    State(state): State<SharedState>,
    Json(query): Json<BundleQuery>,
) -> Result<Json<PreviewView>, ApiError> {
    let collected = collect(&state, &query).await?;
    let (file_name, mail_draft) = names(&collected, &query);
    let total_bytes = collected.sections.iter().map(BundleSection::size).sum();
    Ok(Json(PreviewView {
        problem_type: query.problem_type,
        recording_enabled: state.recordings.is_enabled(),
        operation_recording_id: collected.operation.as_ref().map(|op| op.id.clone()),
        device_id: collected.device_id,
        sections: collected.sections.iter().map(bundle::preview).collect(),
        offers: collected.offers,
        total_bytes,
        exceeds_mail_hint: total_bytes > bundle::MAIL_SIZE_HINT_BYTES,
        file_name,
        mail_draft,
        recordings: collected.recordings,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SectionTextView {
    key: String,
    text: String,
}

async fn section_text(
    State(state): State<SharedState>,
    Json(query): Json<BundleQuery>,
) -> Result<Json<SectionTextView>, ApiError> {
    let key = query
        .key
        .clone()
        .ok_or_else(|| ApiError::bad_request("bundleSectionUnknown"))?;
    let collected = collect(&state, &query).await?;
    let section = collected
        .sections
        .iter()
        .find(|section| section.key() == key)
        .ok_or_else(|| ApiError::bad_request("bundleSectionUnknown"))?;
    Ok(Json(SectionTextView {
        text: bundle::section_text(section),
        key,
    }))
}

async fn export(
    State(state): State<SharedState>,
    Json(query): Json<BundleQuery>,
) -> Result<Response, ApiError> {
    if !query.consent {
        return Err(ApiError::bad_request("bundleConsentRequired"));
    }
    let collected = collect(&state, &query).await?;
    let (file_name, _) = names(&collected, &query);
    let request = request(&state, &query);
    let created_at = collected.created_at.clone();
    let sections = collected.sections;
    let bytes =
        tokio::task::spawn_blocking(move || bundle::build_zip(&request, &sections, &created_at))
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?
            .map_err(|e| ApiError::bad_request(e.to_string()))?;
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{file_name}\""))
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok((
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/zip"),
            ),
            (header::CONTENT_DISPOSITION, disposition),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        bytes,
    )
        .into_response())
}

#[cfg(test)]
#[path = "diagnostic_bundle_routes_tests.rs"]
mod tests;
