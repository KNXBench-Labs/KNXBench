//! Diagnostic bundle routes: recordings, preview, section text, consented ZIP export (DIAG-03).
//!
//! `GET/DELETE /api/diagnostics/recordings` list and delete the operation
//! recordings; `POST /api/diagnostics/bundle/preview` collects the sections a
//! problem type offers and shows size, entries and content markers;
//! `POST /api/diagnostics/bundle/section` returns one section as text; and
//! `POST /api/diagnostics/bundle` writes the ZIP only with `consent: true`.
//! Every route is read-only towards devices: nothing here opens a tunnel.

use std::collections::HashMap;
use std::sync::{atomic::Ordering, Arc};
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

use crate::diagnostic_bundle_context::{ProjectContext, Review};
use crate::errors::ApiError;
use crate::SharedState;
use sha2::{Digest, Sha256};

#[path = "diagnostic_bundle_retained.rs"]
mod retained;

/// The client may describe itself; anything larger is refused.
const MAX_CLIENT_STATE_BYTES: usize = 64 * 1024;
/// The user's own words; anything longer is refused.
const MAX_DESCRIPTION_CHARS: usize = 8000;

pub(crate) fn routes() -> Router<SharedState> {
    // Keep retained-result producer APIs type-checked while their raw payloads remain
    // deliberately unadmitted by this bundle consumer.
    let _ = crate::device_diagnostics_routes::Results::evidence;
    let _ = crate::support_routes::Results::evidence;
    let _ = |evidence: crate::diagnostic_bundle_results::Evidence| {
        let crate::diagnostic_bundle_results::Evidence {
            identity,
            context,
            device_id,
            reports,
            operation_files,
        } = evidence;
        drop((identity, context, device_id, reports, operation_files));
    };
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
    selection: Option<String>,
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
    #[serde(default)]
    review: Option<Value>,
    #[serde(default)]
    context: Option<ProjectContext>,
    /// Request-local summary substitution for route regressions. Production
    /// builds reject this field through `deny_unknown_fields` above.
    #[cfg(test)]
    #[serde(default)]
    test_recording_summary_supplier: Option<TestRecordingSummarySupplier>,
}

#[cfg(test)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TestRecordingSummarySupplier {
    recording_id: String,
    size_bytes: u64,
}

#[cfg(test)]
fn supply_test_recording_summary(
    mut recordings: Vec<RecordingSummary>,
    supplier: Option<&TestRecordingSummarySupplier>,
) -> Vec<RecordingSummary> {
    if let Some(supplier) = supplier {
        if let Some(summary) = recordings
            .iter_mut()
            .find(|summary| summary.id == supplier.recording_id)
        {
            summary.size_bytes = supplier.size_bytes;
        }
    }
    recordings
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
    review: Review,
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
    recordings: Vec<BundleRecordingSummary>,
}

/// Everything one wizard state resolves to.
struct Collected {
    context: ProjectContext,
    result_identity: AdmittedResultIdentity,
    operation: Option<BundleRecordingSummary>,
    device_id: Option<u32>,
    offers: Vec<OfferView>,
    sections: Vec<BundleSection>,
    recordings: Vec<BundleRecordingSummary>,
    created_at: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleRecordingSummary {
    id: String,
    size_bytes: u64,
    format_version: u64,
    active: bool,
    torn_tail: bool,
    end: BundleRecordingEnd,
}

#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
enum BundleRecordingEnd {
    Missing,
    Withheld,
    Known { outcome: BundleRecordingOutcome },
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
enum BundleRecordingOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Abandoned,
}

fn admit_bundle_recording(summary: &RecordingSummary) -> Option<BundleRecordingSummary> {
    const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
    if !knx_app::operation_recording::is_operation_id(&summary.id)
        || summary.format_version != Some(1)
        || summary.size_bytes > MAX_SAFE_INTEGER
    {
        return None;
    }
    let end = match summary.outcome.as_deref() {
        None => BundleRecordingEnd::Missing,
        Some("succeeded") => BundleRecordingEnd::Known {
            outcome: BundleRecordingOutcome::Succeeded,
        },
        Some("failed") => BundleRecordingEnd::Known {
            outcome: BundleRecordingOutcome::Failed,
        },
        Some("cancelled") => BundleRecordingEnd::Known {
            outcome: BundleRecordingOutcome::Cancelled,
        },
        Some("abandoned") => BundleRecordingEnd::Known {
            outcome: BundleRecordingOutcome::Abandoned,
        },
        Some(_) => BundleRecordingEnd::Withheld,
    };
    Some(BundleRecordingSummary {
        id: summary.id.clone(),
        size_bytes: summary.size_bytes,
        format_version: 1,
        active: summary.active,
        torn_tail: summary.torn_tail,
        end,
    })
}

#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
enum AdmittedResultIdentity {
    None,
    System2 { operation_id: String },
    Diagnostics { reference: String },
    Support { sha256: String },
    Unavailable,
}

#[derive(Clone, Copy)]
enum BundleUnavailableCode {
    Source,
    OriginatingDevice,
    RetainedMissing,
    RetainedExpired,
    RetainedUnfinished,
    RetainedNonExportable,
    SupportMissing,
    SupportExpired,
    SupportUnfinished,
    RecordingVersion,
    RecordingResult,
}

impl BundleUnavailableCode {
    fn message(self) -> &'static str {
        match self {
            Self::Source => "the selected diagnostic source is unavailable",
            Self::OriginatingDevice => "the originating device source is unavailable",
            Self::RetainedMissing => "the retained diagnostic result is unavailable",
            Self::RetainedExpired => "the retained diagnostic result has expired",
            Self::RetainedUnfinished => "the diagnostic result is not complete",
            Self::RetainedNonExportable => "the diagnostic result is not exportable",
            Self::SupportMissing => "the retained support result is unavailable",
            Self::SupportExpired => "the retained support result has expired",
            Self::SupportUnfinished => "the support result is not complete",
            Self::RecordingVersion => "the diagnostic recording version is unsupported",
            Self::RecordingResult => "the diagnostic recording result is malformed",
        }
    }
}

#[derive(Clone, Copy)]
enum SourceFailure {
    Unavailable(BundleUnavailableCode),
    Conflict,
}

impl SourceFailure {
    fn unavailable() -> Self {
        Self::Unavailable(BundleUnavailableCode::Source)
    }

    fn message(self) -> &'static str {
        match self {
            Self::Unavailable(code) => code.message(),
            Self::Conflict => "the selected diagnostic source conflicts with its recording",
        }
    }
}

fn source_failure(error: SourceFailure) -> ApiError {
    match error {
        SourceFailure::Conflict => {
            ApiError::with_status(StatusCode::CONFLICT, "bundleSourceConflict")
        }
        SourceFailure::Unavailable(_) => ApiError::bad_request("bundleSourceUnavailable"),
    }
}

fn retained_lookup_failure(
    error: crate::diagnostic_bundle_results::EvidenceLookup,
) -> SourceFailure {
    let code = match error {
        crate::diagnostic_bundle_results::EvidenceLookup::Missing => {
            BundleUnavailableCode::RetainedMissing
        }
        crate::diagnostic_bundle_results::EvidenceLookup::Expired => {
            BundleUnavailableCode::RetainedExpired
        }
        crate::diagnostic_bundle_results::EvidenceLookup::Unfinished => {
            BundleUnavailableCode::RetainedUnfinished
        }
        crate::diagnostic_bundle_results::EvidenceLookup::NonExportable => {
            BundleUnavailableCode::RetainedNonExportable
        }
        crate::diagnostic_bundle_results::EvidenceLookup::Unavailable => {
            BundleUnavailableCode::Source
        }
    };
    SourceFailure::Unavailable(code)
}

fn support_lookup_failure(
    error: crate::diagnostic_bundle_results::EvidenceLookup,
) -> SourceFailure {
    use crate::diagnostic_bundle_results::EvidenceLookup;
    SourceFailure::Unavailable(match error {
        EvidenceLookup::Missing => BundleUnavailableCode::SupportMissing,
        EvidenceLookup::Expired => BundleUnavailableCode::SupportExpired,
        EvidenceLookup::Unfinished => BundleUnavailableCode::SupportUnfinished,
        EvidenceLookup::NonExportable | EvidenceLookup::Unavailable => {
            BundleUnavailableCode::Source
        }
    })
}

struct AdmittedEvidence {
    identity: AdmittedResultIdentity,
    context: Option<ProjectContext>,
    device_id: Option<u32>,
    reports: Vec<BundleFile>,
    operation_files: Vec<BundleFile>,
}

fn admit_retained_evidence(
    evidence: crate::diagnostic_bundle_results::Evidence,
) -> Result<AdmittedEvidence, SourceFailure> {
    let crate::diagnostic_bundle_results::Evidence {
        identity,
        context,
        device_id,
        reports,
        operation_files,
    } = evidence;
    let object = identity
        .as_object()
        .ok_or_else(SourceFailure::unavailable)?;
    let admitted_identity = if object.len() == 1 {
        if let Some(reference) = object.get("diagnosticsReference").and_then(Value::as_str) {
            if reference.len() != 64 || !reference.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(SourceFailure::unavailable());
            }
            AdmittedResultIdentity::Diagnostics {
                reference: reference.to_string(),
            }
        } else if let Some(sha256) = object.get("supportReportSha256").and_then(Value::as_str) {
            if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(SourceFailure::unavailable());
            }
            AdmittedResultIdentity::Support {
                sha256: sha256.to_string(),
            }
        } else {
            return Err(SourceFailure::unavailable());
        }
    } else {
        return Err(SourceFailure::unavailable());
    };
    if reports.is_empty() || operation_files.len() != 1 {
        return Err(SourceFailure::unavailable());
    }
    Ok(AdmittedEvidence {
        identity: admitted_identity,
        context,
        device_id,
        reports,
        operation_files,
    })
}

fn validate_retained_provenance(
    operation: Option<&RecordingSummary>,
    related_key: &str,
    token: &str,
    evidence: &mut AdmittedEvidence,
) -> Result<(), SourceFailure> {
    let Some(operation) = operation else {
        return Ok(());
    };
    if operation.format_version != Some(1) || operation.related[related_key].as_str() != Some(token)
    {
        return Err(SourceFailure::unavailable());
    }
    let recorded_context = recording_context(operation);
    if recorded_context.as_ref() != evidence.context.as_ref() {
        evidence.context = None;
        return Ok(());
    }
    let recorded_device = operation.target["deviceId"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok());
    if recorded_device != evidence.device_id {
        return Err(SourceFailure::Conflict);
    }
    Ok(())
}

#[derive(Default)]
struct System2Cache {
    entries: HashMap<
        String,
        Result<Arc<crate::device_system2::DiagnosticBundleAdmission>, BundleUnavailableCode>,
    >,
}

impl System2Cache {
    async fn get(
        &mut self,
        state: &SharedState,
        id: &str,
    ) -> Result<Arc<crate::device_system2::DiagnosticBundleAdmission>, SourceFailure> {
        if !self.entries.contains_key(id) {
            let admitted = state
                .device_system2
                .diagnostic_bundle_admission(state, id.to_string())
                .await
                .ok()
                .flatten()
                .map(Arc::new)
                .ok_or(BundleUnavailableCode::Source);
            self.entries.insert(id.to_string(), admitted);
        }
        match self.entries.get(id).expect("inserted System-2 admission") {
            Ok(admitted) => Ok(admitted.clone()),
            Err(code) => Err(SourceFailure::Unavailable(*code)),
        }
    }
}

struct System2Provenance {
    context: ProjectContext,
    device_id: u32,
    address: knx_core::IndividualAddress,
}

impl System2Provenance {
    fn read(operation: &RecordingSummary, operation_id: &str) -> Result<Self, SourceFailure> {
        if operation.format_version != Some(1)
            || !matches!(
                operation.operation_kind.as_deref(),
                Some("system2-download" | "system2-recovery")
            )
            || operation.related["operationId"].as_str() != Some(operation_id)
        {
            return Err(SourceFailure::unavailable());
        }
        let context = recording_context(operation).ok_or_else(SourceFailure::unavailable)?;
        let device_id = operation.target["deviceId"]
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(SourceFailure::unavailable)?;
        let address = operation.target["individualAddress"]
            .as_str()
            .and_then(|value| value.parse().ok())
            .ok_or_else(SourceFailure::unavailable)?;
        Ok(Self {
            context,
            device_id,
            address,
        })
    }

    fn validate(
        self,
        admitted: &crate::device_system2::DiagnosticBundleAdmission,
    ) -> Result<(), SourceFailure> {
        let admitted_address = admitted
            .address()
            .parse::<knx_core::IndividualAddress>()
            .map_err(|_| SourceFailure::unavailable())?;
        if self.context != admitted.project_context()
            || self.device_id != admitted.device_id()
            || self.address != admitted_address
        {
            return Err(SourceFailure::Conflict);
        }
        Ok(())
    }
}

fn check_query(query: &BundleQuery) -> Result<(), ApiError> {
    if let Some(reference) = &query.operation {
        match (
            &reference.selection,
            &reference.recording_id,
            &reference.related_key,
            &reference.related_value,
        ) {
            (Some(selection), None, None, None) if selection == "none" => {}
            (None, Some(_), None, None) | (None, None, Some(_), Some(_)) => {}
            _ => return Err(ApiError::bad_request("bundleOperationReferenceRefused")),
        }
    }
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

fn preview_changed() -> ApiError {
    ApiError::with_status(StatusCode::CONFLICT, "bundlePreviewChanged")
}

fn parse_review(value: &Value) -> Result<Review, ApiError> {
    let review: Review = serde_json::from_value(value.clone())
        .map_err(|_| ApiError::bad_request("bundlePreviewRequired"))?;
    if review.fingerprint.len() != 64
        || !review
            .fingerprint
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ApiError::bad_request("bundlePreviewRequired"));
    }
    Ok(review)
}

fn required_review(query: &BundleQuery) -> Result<Review, ApiError> {
    parse_review(
        query
            .review
            .as_ref()
            .ok_or_else(|| ApiError::bad_request("bundlePreviewRequired"))?,
    )
}

fn review_of(collected: &Collected, query: &BundleQuery) -> Review {
    let sections: Vec<_> = collected.sections.iter().map(bundle::preview).collect();
    let metadata = json!({
        "context": collected.context,
        "problemType": query.problem_type,
        "operation": collected.operation,
        "result": collected.result_identity,
        "deviceId": collected.device_id,
        "offers": collected.offers,
        "sections": sections,
    });
    let mut hash = Sha256::new();
    hash.update(serde_json::to_vec(&metadata).expect("bundle metadata serializes"));
    for section in &collected.sections {
        for file in &section.files {
            hash.update((file.bytes.len() as u64).to_le_bytes());
            hash.update(&file.bytes);
        }
    }
    Review {
        fingerprint: format!("{:x}", hash.finalize()),
        context: collected.context.clone(),
    }
}

fn verify_review(collected: &Collected, query: &BundleQuery) -> Result<(), ApiError> {
    if required_review(query)? != review_of(collected, query) {
        return Err(preview_changed());
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
            .find(|r| {
                !r.active
                    && kind_fits(query.problem_type, r.operation_kind.as_deref())
                    && admit_bundle_recording(r).is_some()
            })
            .cloned());
    };
    if reference.selection.as_deref() == Some("none") {
        return Ok(None);
    }
    let found = match (
        &reference.recording_id,
        &reference.related_key,
        &reference.related_value,
    ) {
        (Some(id), None, None) => recordings
            .iter()
            .find(|r| &r.id == id && admit_bundle_recording(r).is_some()),
        (None, Some(key), Some(value)) => recordings.iter().find(|r| {
            r.related[key.as_str()].as_str() == Some(value.as_str())
                // Download ids restart with every server run.
                && (key != "downloadId"
                    || r.related["serverIncarnation"].as_str() == Some(server_incarnation))
                && (admit_bundle_recording(r).is_some()
                    || matches!(key.as_str(), "diagnosticsToken" | "supportToken"))
        }),
        _ => return Err(ApiError::bad_request("bundleOperationReferenceRefused")),
    };
    Ok(found.cloned())
}

fn recording_context(operation: &RecordingSummary) -> Option<ProjectContext> {
    if operation.format_version != Some(1) {
        return None;
    }
    serde_json::from_str(operation.related["projectContext"].as_str()?).ok()
}

async fn resolve_evidence(
    state: &SharedState,
    query: &BundleQuery,
    operation: Option<&RecordingSummary>,
    cache: &mut System2Cache,
) -> Result<AdmittedEvidence, SourceFailure> {
    let reference = query.operation.as_ref();
    let related = |key: &str| -> Option<String> {
        operation
            .filter(|op| op.format_version == Some(1))
            .and_then(|op| op.related[key].as_str())
            .map(str::to_string)
            .or_else(|| {
                reference
                    .filter(|r| r.related_key.as_deref() == Some(key))
                    .and_then(|r| r.related_value.clone())
            })
    };
    if let Some(id) = related("operationId") {
        let provenance = operation
            .map(|summary| System2Provenance::read(summary, &id))
            .transpose()?;
        let admitted = cache.get(state, &id).await?;
        if let Some(provenance) = provenance {
            provenance.validate(&admitted)?;
        }
        let report = admitted.report().ok_or_else(SourceFailure::unavailable)?;
        let report = serde_json::to_value(report).map_err(|_| SourceFailure::unavailable())?;
        return Ok(AdmittedEvidence {
            identity: AdmittedResultIdentity::System2 { operation_id: id },
            context: Some(admitted.project_context()),
            device_id: Some(admitted.device_id()),
            reports: vec![BundleFile::json("system2-report.json", &report)],
            operation_files: vec![BundleFile::new(
                "system2-private.json",
                admitted.projection_bytes().to_vec(),
            )],
        });
    }
    if let Some(token) = related("diagnosticsToken") {
        let mut evidence = match state.diagnostic_results.evidence(&token) {
            Ok(evidence) => admit_retained_evidence(evidence)?,
            Err(crate::diagnostic_bundle_results::EvidenceLookup::Missing)
                if operation.is_some() =>
            {
                retained::diagnostic_recording(state, operation, &token)?
            }
            Err(error) => return Err(retained_lookup_failure(error)),
        };
        if !matches!(
            &evidence.identity,
            AdmittedResultIdentity::Diagnostics { .. }
        ) {
            return Err(SourceFailure::unavailable());
        }
        validate_retained_provenance(operation, "diagnosticsToken", &token, &mut evidence)?;
        return Ok(evidence);
    }
    if let Some(token) = related("supportToken") {
        let mut evidence = admit_retained_evidence(
            state
                .support_results
                .evidence(&token)
                .map_err(support_lookup_failure)?,
        )?;
        if !matches!(&evidence.identity, AdmittedResultIdentity::Support { .. }) {
            return Err(SourceFailure::unavailable());
        }
        validate_retained_provenance(operation, "supportToken", &token, &mut evidence)?;
        return Ok(evidence);
    }
    Err(SourceFailure::unavailable())
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
    let context = ProjectContext::capture(state);
    if query
        .context
        .as_ref()
        .is_some_and(|expected| expected != &context)
    {
        return Err(preview_changed());
    }
    if let Some(review) = &query.review {
        if parse_review(review)?.context != context {
            return Err(preview_changed());
        }
    }
    let store = state.recordings.clone();
    let recordings = tokio::task::spawn_blocking(move || {
        // A recording still being written shows its last lines a moment later.
        store.wait_until_idle(Duration::from_millis(500));
        store.list()
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?
    .map_err(|e| ApiError::internal(format!("recordings cannot be listed: {e}")))?;
    #[cfg(test)]
    let recordings =
        supply_test_recording_summary(recordings, query.test_recording_summary_supplier.as_ref());
    let problem = query.problem_type;
    let operation = if bundle::default_offer(problem, SectionKind::Operation) == Offer::NotOffered {
        None
    } else {
        resolve_operation(query, &recordings, &state.server_incarnation)?
    };
    let operation_view = operation.as_ref().and_then(admit_bundle_recording);
    let recording_views = recordings
        .iter()
        .filter_map(admit_bundle_recording)
        .collect();
    let mut system2_cache = System2Cache::default();
    let mut evidence = resolve_evidence(state, query, operation.as_ref(), &mut system2_cache).await;
    if matches!(evidence, Err(SourceFailure::Conflict)) {
        return Err(source_failure(SourceFailure::Conflict));
    }
    let run_device = evidence
        .as_ref()
        .ok()
        .and_then(|result| result.device_id)
        .or_else(|| {
            operation
                .as_ref()
                .and_then(|op| op.target["deviceId"].as_u64())
                .and_then(|id| u32::try_from(id).ok())
        });
    // An admitted result speaks only for its own context; one whose context was
    // cleared as contradicting its recording speaks for none, so its ids are never
    // compared or projected.
    let source = match &evidence {
        Ok(result) => result.context.clone(),
        Err(_) => operation.as_ref().and_then(recording_context),
    };
    // Address-to-id resolution is allowed only in the proven, unchanged source project.
    let run_device = run_device.or_else(|| {
        let address = operation.as_ref()?.target["individualAddress"]
            .as_str()?
            .parse::<knx_core::IndividualAddress>()
            .ok()?;
        if source.as_ref() != Some(&context) {
            return None;
        }
        let project = state.project.lock().expect("state mutex poisoned");
        if ProjectContext::locked(state) != context {
            return None;
        }
        let mut devices = project
            .as_ref()?
            .devices
            .iter()
            .filter(|device| device.address == Some(address));
        let device = devices.next()?.id.0;
        devices.next().is_none().then_some(device)
    });
    // Device ids are comparable only inside the proven, unchanged source project.
    if source.as_ref() == Some(&context)
        && query
            .device_id
            .zip(run_device)
            .is_some_and(|(chosen, target)| chosen != target)
    {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "bundleTargetMismatch",
        ));
    }
    let device_id = evidence.as_ref().ok().and_then(|result| result.device_id);
    let projectable = projectable_device(
        query,
        operation.as_ref(),
        evidence.as_ref(),
        source.as_ref(),
        run_device,
        &context,
    );
    let result_identity = match &evidence {
        Ok(result) => result.identity.clone(),
        Err(_)
            if operation.is_some()
                || query
                    .operation
                    .as_ref()
                    .is_some_and(|op| op.selection.as_deref() == Some("none")) =>
        {
            AdmittedResultIdentity::None
        }
        Err(_) => AdmittedResultIdentity::Unavailable,
    };
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
        let built = build_section(
            state,
            query,
            kind,
            operation.as_ref(),
            evidence.as_mut().map_err(|error| *error),
            SectionScope {
                recordings: &recordings,
                projectable,
            },
            &mut system2_cache,
        )
        .await;
        let unavailable = match built {
            Ok(built) => {
                if wanted {
                    sections.extend(built);
                }
                None
            }
            Err(SourceFailure::Conflict) => return Err(source_failure(SourceFailure::Conflict)),
            Err(reason) => Some(reason.message().to_string()),
        };
        offers.push(OfferView {
            kind,
            offer,
            selected: wanted && unavailable.is_none(),
            unavailable,
        });
    }
    if ProjectContext::capture(state) != context {
        return Err(preview_changed());
    }
    Ok(Collected {
        context,
        result_identity,
        operation: operation_view,
        device_id,
        offers,
        sections,
        recordings: recording_views,
        created_at,
    })
}

/// What `collect` decided once per request for every section kind.
struct SectionScope<'a> {
    /// The admitted recording listing earlier operations are chosen from.
    recordings: &'a [RecordingSummary],
    /// See [`projectable_device`].
    projectable: Result<Option<u32>, SourceFailure>,
}

/// Builds the sections of `kind`, or says why there are none.
async fn build_section(
    state: &SharedState,
    query: &BundleQuery,
    kind: SectionKind,
    operation: Option<&RecordingSummary>,
    evidence: Result<&mut AdmittedEvidence, SourceFailure>,
    scope: SectionScope<'_>,
    system2_cache: &mut System2Cache,
) -> Result<Vec<BundleSection>, SourceFailure> {
    let SectionScope {
        recordings,
        projectable,
    } = scope;
    match kind {
        SectionKind::Environment => Ok(vec![environment(state)]),
        SectionKind::SessionLog => Err(SourceFailure::unavailable()),
        SectionKind::Operation => {
            if let Some(operation) = operation {
                if let Ok(evidence) = evidence {
                    if !evidence.operation_files.is_empty() {
                        let mut section = BundleSection::new(SectionKind::Operation);
                        section.label = Some(operation.id.clone());
                        section.files = std::mem::take(&mut evidence.operation_files);
                        section.entries = section.files.len();
                        section.notes.push(if matches!(
                            &evidence.identity,
                            AdmittedResultIdentity::System2 { .. }
                        ) {
                            "the raw operation recording is withheld; this is the admitted private projection"
                                .into()
                        } else {
                            "the raw operation recording is withheld; this is the retained typed result"
                                .into()
                        });
                        return Ok(vec![section]);
                    }
                }
                return Ok(vec![
                    operation_section(state, operation, SectionKind::Operation, system2_cache)
                        .await?,
                ]);
            }
            let evidence = evidence?;
            if evidence.operation_files.is_empty() {
                return Err(SourceFailure::unavailable());
            }
            let mut section = BundleSection::new(SectionKind::Operation);
            section.files = std::mem::take(&mut evidence.operation_files);
            section.entries = section.files.len();
            section.notes.push(if matches!(
                &evidence.identity,
                AdmittedResultIdentity::System2 { .. }
            ) {
                "the operation recording is unavailable; this is the admitted private projection"
                    .into()
            } else {
                "the operation recording is unavailable; this is the retained typed result".into()
            });
            Ok(vec![section])
        }
        SectionKind::EarlierOperation => {
            let mut out = Vec::new();
            // Each listed recording is read once; with retention this bounds the total.
            let mut seen = std::collections::BTreeSet::new();
            for id in &query.earlier_recording_ids {
                if operation.is_some_and(|op| &op.id == id) || !seen.insert(id.as_str()) {
                    continue;
                }
                let summary = recordings
                    .iter()
                    .find(|r| &r.id == id)
                    .filter(|summary| admit_bundle_recording(summary).is_some())
                    .ok_or_else(SourceFailure::unavailable)?;
                out.push(
                    operation_section(state, summary, SectionKind::EarlierOperation, system2_cache)
                        .await?,
                );
            }
            Ok(out)
        }
        SectionKind::Device => {
            let device = projectable?.ok_or_else(SourceFailure::unavailable)?;
            Ok(vec![device_section(state, device)?])
        }
        SectionKind::LegacyReports => {
            let evidence = evidence?;
            let mut section = BundleSection::new(SectionKind::LegacyReports);
            section.files = std::mem::take(&mut evidence.reports);
            section.entries = section.files.len();
            section.markers = markers(false, false, true, false);
            section.notes.extend(report_notes(&section.files));
            Ok(vec![section])
        }
        SectionKind::ImportReport => Ok(vec![import_report(state)?]),
        SectionKind::ImportFile => Err(SourceFailure::unavailable()),
        SectionKind::Catalogue => Ok(vec![catalogue(state, projectable?)]),
        SectionKind::Project => Ok(vec![project_section(state).await?]),
        SectionKind::BusMonitor | SectionKind::ClientState => Err(SourceFailure::unavailable()),
    }
}

/// Which current-project device the device and catalogue extracts may project.
///
/// Decided once per request from what `collect` proved: a source (retained result
/// or recording) names its device only inside its own project context, which must
/// equal the request's unchanged current context; `run_device` is then that
/// source's own device (by id, or by an unambiguous address). A retained result
/// whose context was cleared as contradicting its recording never lends its device.
/// Without any selected source only the user's explicit choice is projected;
/// `Ok(None)` is a device-free catalogue. A named source that failed without any
/// recording projects nothing; a resolved recording may still prove its own device.
fn projectable_device(
    query: &BundleQuery,
    operation: Option<&RecordingSummary>,
    evidence: Result<&AdmittedEvidence, &SourceFailure>,
    source: Option<&ProjectContext>,
    run_device: Option<u32>,
    context: &ProjectContext,
) -> Result<Option<u32>, SourceFailure> {
    if evidence.is_err() && operation.is_none() {
        return if no_source_selected(query) {
            Ok(query.device_id)
        } else {
            Err(SourceFailure::unavailable())
        };
    }
    if source != Some(context) {
        return Err(SourceFailure::Unavailable(
            BundleUnavailableCode::OriginatingDevice,
        ));
    }
    // A differing chosen id in this context was already refused as
    // `bundleTargetMismatch` by `collect`.
    run_device.map(Some).ok_or_else(SourceFailure::unavailable)
}

/// True only when the request explicitly or implicitly names no operation source.
fn no_source_selected(query: &BundleQuery) -> bool {
    query.operation.as_ref().is_none_or(|op| {
        op.selection.as_deref() == Some("none")
            && op.recording_id.is_none()
            && op.related_key.is_none()
            && op.related_value.is_none()
    })
}

fn device_section(state: &SharedState, id: u32) -> Result<BundleSection, SourceFailure> {
    let detail =
        crate::domain::device_detail(state, id, None).map_err(|_| SourceFailure::unavailable())?;
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
            // Availability only: the raw startup reason can hold local paths.
            "productDatabase": {"availability": state.product_database_availability().as_str()},
            "deviceProductIdentity": product,
        }),
    ));
    section.entries = 1;
    section
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
            "tunnelRouteBack": state.tunnel_return_path == knx_net::TunnelReturnPath::RouteBack,
        },
        "productDatabase": {
            "availability": state.product_database_availability().as_str(),
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

async fn operation_section(
    state: &SharedState,
    operation: &RecordingSummary,
    kind: SectionKind,
    cache: &mut System2Cache,
) -> Result<BundleSection, SourceFailure> {
    let Some(operation_id) = operation.related["operationId"].as_str() else {
        // A System-2 recording without its journal id stays unavailable, never raw.
        if operation
            .operation_kind
            .as_deref()
            .is_some_and(|kind| kind.starts_with("system2-"))
        {
            return Err(SourceFailure::unavailable());
        }
        return raw_recording_section(state, operation, kind).await;
    };
    let provenance = System2Provenance::read(operation, operation_id)?;
    let admitted = cache.get(state, operation_id).await?;
    provenance.validate(&admitted)?;
    let mut section = BundleSection::new(kind);
    section.label = Some(operation.id.clone());
    section.entries = 1;
    section.notes.push(
        "the raw operation recording is withheld; this is the admitted private projection".into(),
    );
    section.files.push(BundleFile::new(
        "system2-private.json",
        admitted.projection_bytes().to_vec(),
    ));
    Ok(section)
}

/// Shown wherever declared memory or a raw recording leaves after consent.
const PRIVATE_RAW_NOTE: &str = "contains device configuration, possibly passwords; \
     recognised KNX keys are redacted; keep private";

/// The consent warning for retained reports that carry declared memory.
fn report_notes(files: &[BundleFile]) -> Option<String> {
    files
        .iter()
        .any(|file| file.name == "memory.json")
        .then(|| PRIVATE_RAW_NOTE.to_string())
}

/// Largest raw recording copied into a bundle; a larger one stays unavailable.
const MAX_RAW_RECORDING_BYTES: u64 = 64 * 1024 * 1024;

/// A non-System-2 recording exactly as the recorder wrote it (keys redacted at capture).
///
/// Owner decision 2026-10-31: the redacted raw recording may leave after consent;
/// historical, unknown or malformed recordings are never backfilled or rewritten.
async fn raw_recording_section(
    state: &SharedState,
    operation: &RecordingSummary,
    kind: SectionKind,
) -> Result<BundleSection, SourceFailure> {
    let store = state.recordings.clone();
    let id = operation.id.clone();
    let bytes =
        tokio::task::spawn_blocking(move || store.read_bounded(&id, MAX_RAW_RECORDING_BYTES))
            .await
            .map_err(|_| SourceFailure::unavailable())?
            .map_err(|_| SourceFailure::unavailable())?;
    let mut section = BundleSection::new(kind);
    section.label = Some(operation.id.clone());
    section.entries = bytes.iter().filter(|byte| **byte == b'\n').count();
    section.markers = markers(true, true, true, true);
    section.notes.push(PRIVATE_RAW_NOTE.into());
    if operation.torn_tail {
        section
            .notes
            .push("the recording's last line is incomplete (interrupted write)".into());
    }
    section
        .files
        .push(BundleFile::new("recording.jsonl", bytes));
    Ok(section)
}

fn import_report(state: &SharedState) -> Result<BundleSection, SourceFailure> {
    let project = state.project.lock().expect("state mutex poisoned");
    project.as_ref().ok_or_else(SourceFailure::unavailable)?;
    let opaque = state.opaque.lock().expect("state mutex poisoned");
    let evidence = knx_app::import_evidence::read(&opaque)
        .map_err(|_| SourceFailure::unavailable())?
        .ok_or_else(SourceFailure::unavailable)?;
    let copy = knx_app::secret_free_copy::import_report_for_bundle(&evidence.report)
        .map_err(|_| SourceFailure::unavailable())?;
    let (bytes, notes) = copy.into_parts();
    let mut section = BundleSection::new(SectionKind::ImportReport);
    section.notes = notes;
    section
        .files
        .push(BundleFile::new("import-report.json", bytes));
    section.entries = 1;
    section.markers = markers(true, true, false, false);
    Ok(section)
}

async fn project_section(state: &SharedState) -> Result<BundleSection, SourceFailure> {
    let path = state
        .store_path
        .lock()
        .expect("state mutex poisoned")
        .clone()
        .ok_or_else(SourceFailure::unavailable)?;
    let copy = tokio::task::spawn_blocking(move || {
        knx_app::secret_free_copy::native_project_for_bundle(&path)
    })
    .await
    .map_err(|_| SourceFailure::unavailable())?
    .map_err(|_| SourceFailure::unavailable())?;
    let (bytes, notes) = copy.into_parts();
    let mut section = BundleSection::new(SectionKind::Project);
    section.entries = 1;
    section.markers = markers(true, true, true, true);
    section
        .notes
        .push("the last saved state of the project; unsaved changes are not included".into());
    section.notes.extend(notes);
    section.files.push(BundleFile::new("project.knxdb", bytes));
    Ok(section)
}

fn request(state: &SharedState, query: &BundleQuery) -> BundleRequest {
    BundleRequest {
        problem: query.problem_type,
        writer: crate::operation_recordings::writer_identity(),
        recording_enabled: state.recordings.is_enabled(),
        consent_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        description: query
            .description
            .as_ref()
            .map(|_| "User description withheld from diagnostic bundle.".to_string()),
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
        review: review_of(&collected, &query),
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
    required_review(&query)?;
    let key = query
        .key
        .clone()
        .ok_or_else(|| ApiError::bad_request("bundleSectionUnknown"))?;
    let collected = collect(&state, &query).await?;
    verify_review(&collected, &query)?;
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
    required_review(&query)?;
    let collected = collect(&state, &query).await?;
    verify_review(&collected, &query)?;
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

#[cfg(test)]
#[path = "diagnostic_bundle_binding_tests.rs"]
mod binding_tests;
