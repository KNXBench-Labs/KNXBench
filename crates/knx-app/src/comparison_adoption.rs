//! Source-bound read-only comparisons and atomic device → project adoption.
//!
//! No deserializable evidence or value-accepting constructor is exposed. Retain
//! these transient objects in the caller; request payloads may select row indices
//! only. See `docs/contracts/alpha16-comparison-adoption.md` for trust/locking rules.
use crate::device_download::prepare_device_download;
use knx_core::command::comparison_adoption::{
    AdoptionCommandError, ParameterAdoption, PreparedComparisonAdoption,
};
use knx_core::commissioning::memory_download::MemoryDownloadPlan;
use knx_core::{CommandStack, ContactableAddress, DeviceId, IndividualAddress, Project};
use knx_net::commissioning::memory_download::{compare_with_plan, CompareError, PlanComparison};
use knx_net::commissioning::SessionTiming;
use knx_net::management::ManagementTransport;
use knx_productdb::image::{DownloadImage, ImageRequest, ParameterAttribution};
use knx_productdb::image_comparison::{decode, ComparisonRow};
use knx_productdb::Connection;
use knx_store::project_history::NativeSnapshot;
use std::collections::BTreeSet;
use std::path::Path;

/// Caller-owned current session identity, never authority supplied by a client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonScope {
    pub server_incarnation: String,
    pub project_incarnation: u64,
}
#[derive(Debug)]
pub enum AdoptionError {
    InvalidScope,
    Stale,
    SourceChanged,
    Preparation(String),
    Compare(CompareError),
    EmptySelection,
    DuplicateRow(usize),
    MissingRow(usize),
    UnsupportedRow { index: usize, kind: &'static str },
    UnknownRow { index: usize },
    AmbiguousParameter(String),
    CandidateFieldChanged(String),
    Command(AdoptionCommandError),
    Native(String),
}
impl std::fmt::Display for AdoptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidScope => f.write_str("invalid comparison session scope"),
            Self::Stale => f.write_str(
                "comparison project, scope, revision or complete history changed; compare again",
            ),
            Self::SourceChanged => {
                f.write_str("comparison source request/image/plan changed; compare again")
            }
            Self::Preparation(error) => write!(f, "comparison preparation: {error}"),
            Self::Compare(error) => write!(f, "comparison read: {error}"),
            Self::EmptySelection => f.write_str("select at least one supported comparison row"),
            Self::DuplicateRow(index) => write!(f, "duplicate requested comparison row {index}"),
            Self::MissingRow(index) => write!(f, "comparison row {index} does not exist"),
            Self::UnsupportedRow { index, kind } => write!(
                f,
                "comparison row {index} ({kind}) has no supported project adoption mapping"
            ),
            Self::UnknownRow { index } => {
                write!(f, "comparison row {index} is not completely decoded")
            }
            Self::AmbiguousParameter(reference) => {
                write!(f, "ambiguous parameter attribution: {reference}")
            }
            Self::CandidateFieldChanged(reference) => write!(
                f,
                "adopted field {reference} no longer has the same verified placement/value"
            ),
            Self::Command(error) => write!(f, "{error}"),
            Self::Native(error) => write!(
                f,
                "comparison adoption native/history preflight or persistence: {error}"
            ),
        }
    }
}
impl std::error::Error for AdoptionError {}
impl From<AdoptionCommandError> for AdoptionError {
    fn from(value: AdoptionCommandError) -> Self {
        Self::Command(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceAuthority {
    request: ImageRequest,
    image: DownloadImage,
    plan: MemoryDownloadPlan,
}
/// Construct all source-derived inputs under one SQLite read snapshot. Refuse
/// an already active transaction rather than changing its ownership/semantics.
fn source(
    conn: &Connection,
    project: &Project,
    target: IndividualAddress,
) -> Result<SourceAuthority, AdoptionError> {
    let transaction = conn
        .unchecked_transaction()
        .map_err(|e| AdoptionError::Preparation(e.to_string()))?;
    let prepared = prepare_device_download(&transaction, project, target)
        .map_err(|e| AdoptionError::Preparation(e.to_string()))?;
    transaction
        .commit()
        .map_err(|e| AdoptionError::Preparation(e.to_string()))?;
    Ok(SourceAuthority {
        request: prepared.request,
        image: prepared.image,
        plan: prepared.plan,
    })
}

/// Prepared from the actual project/source, not caller-supplied image/plan bytes.
#[derive(Debug)]
pub struct PreparedComparison {
    project: Project,
    scope: ComparisonScope,
    device: DeviceId,
    revision: u64,
    target: ContactableAddress,
    source: SourceAuthority,
}
impl PreparedComparison {
    pub fn device(&self) -> DeviceId {
        self.device
    }
    pub fn target(&self) -> IndividualAddress {
        self.target.address()
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn scope(&self) -> &ComparisonScope {
        &self.scope
    }
    /// Display only. Returning shared references cannot change read authority.
    pub fn image(&self) -> &DownloadImage {
        &self.source.image
    }
    pub fn plan(&self) -> &MemoryDownloadPlan {
        &self.source.plan
    }
    pub fn program_id(&self) -> &str {
        &self.source.request.program_id
    }
    /// Call immediately before tunnel admission, under the project/stack locks.
    /// Recheck after the read via `ObservedComparison::is_current`.
    pub fn is_current(
        &self,
        project: &Project,
        stack: &CommandStack,
        scope: &ComparisonScope,
    ) -> bool {
        use knx_core::commissioning::pending::programming_content;
        let devices = BTreeSet::from([self.device]);
        self.scope == *scope
            && stack.comparison_revision(self.device) == self.revision
            && programming_content(&self.project, &devices)
                == programming_content(project, &devices)
    }
    /// Runs the existing read-only identity-checked comparison itself. No public
    /// API can promote a fabricated `PlanComparison` or JSON result to evidence.
    pub async fn observe<T: ManagementTransport>(
        self,
        transport: &T,
        timing: SessionTiming,
    ) -> Result<ObservedComparison, AdoptionError> {
        let comparison = compare_with_plan(transport, self.target, timing, &self.source.plan)
            .await
            .map_err(AdoptionError::Compare)?;
        let rows = decode(&self.source.image, &comparison.held, &self.source.plan);
        Ok(ObservedComparison {
            prepared: self,
            comparison,
            rows,
        })
    }
}

pub fn prepare_comparison(
    conn: &Connection,
    project: &Project,
    stack: &CommandStack,
    scope: &ComparisonScope,
    device: DeviceId,
) -> Result<PreparedComparison, AdoptionError> {
    if scope.server_incarnation.is_empty() || scope.server_incarnation.len() > 128 {
        return Err(AdoptionError::InvalidScope);
    }
    let revision = stack.comparison_revision(device);
    if revision == u64::MAX {
        return Err(AdoptionError::Stale);
    }
    let target = project
        .devices
        .get(device)
        .and_then(|d| d.address)
        .ok_or_else(|| {
            AdoptionError::Preparation("device has no contactable project address".into())
        })?;
    // Uses existing single-device-at-address admission, not first-match selection.
    let authority = source(conn, project, target)?;
    let target =
        ContactableAddress::new(target).map_err(|e| AdoptionError::Preparation(e.to_string()))?;
    Ok(PreparedComparison {
        project: project.clone(),
        scope: scope.clone(),
        device,
        revision,
        target,
        source: authority,
    })
}

/// Genuine successful read; no public constructor, mutable getters or serde.
#[derive(Debug)]
pub struct ObservedComparison {
    prepared: PreparedComparison,
    comparison: PlanComparison,
    rows: Vec<ComparisonRow>,
}
impl ObservedComparison {
    pub fn rows(&self) -> &[ComparisonRow] {
        &self.rows
    }
    pub fn comparison(&self) -> &PlanComparison {
        &self.comparison
    }
    pub fn preparation(&self) -> &PreparedComparison {
        &self.prepared
    }
    pub fn is_current(
        &self,
        project: &Project,
        stack: &CommandStack,
        scope: &ComparisonScope,
    ) -> bool {
        self.prepared.is_current(project, stack, scope)
    }
}

/// Complete preview, including unsupported/unread rows. Candidate authority and
/// Core command are immutable and are never reconstructed from a client response.
#[derive(Debug)]
pub struct PreparedAdoption {
    before: NativeSnapshot,
    scope: ComparisonScope,
    target: IndividualAddress,
    original_source: SourceAuthority,
    candidate_source: SourceAuthority,
    command: PreparedComparisonAdoption,
    rows: Vec<ComparisonRow>,
    selected: Vec<usize>,
}
impl PreparedAdoption {
    pub fn rows(&self) -> &[ComparisonRow] {
        &self.rows
    }
    pub fn selected(&self) -> &[usize] {
        &self.selected
    }
    pub fn changed(&self) -> bool {
        self.command.changed()
    }
    pub fn candidate(&self) -> (&Project, &CommandStack) {
        self.command.candidate()
    }
}

fn same_placement(a: &ParameterAttribution, b: &ParameterAttribution) -> bool {
    a.reference == b.reference && a.segment == b.segment && a.field == b.field
}
fn ambiguous(image: &DownloadImage, selected: &ParameterAttribution) -> bool {
    let start_bit = |p: &ParameterAttribution| {
        image.segment(&p.segment).map(|s| {
            (u64::from(s.address) + u64::from(p.field.offset)) * 8 + u64::from(p.field.bit_offset)
        })
    };
    let Some(start) = start_bit(selected) else {
        return true;
    };
    let end = start + u64::from(selected.field.size_in_bit);
    image
        .parameter_fields
        .iter()
        .filter(|p| p.reference == selected.reference)
        .count()
        != 1
        || image
            .parameter_fields
            .iter()
            .filter(|p| p.reference != selected.reference)
            .any(|p| {
                start_bit(p).is_some_and(|other| {
                    start < other + u64::from(p.field.size_in_bit) && other < end
                })
            })
}

/// Preflight the complete native candidate plus both history directions using
/// the actual native history encoder/admission, preserving retained context.
fn preflight(snapshot: &NativeSnapshot, stack: &CommandStack) -> Result<(), AdoptionError> {
    let memory = knx_store::open_and_migrate_in_memory()
        .map_err(|e| AdoptionError::Native(e.to_string()))?;
    knx_store::project_history::save_editor(&memory, snapshot, stack, None, true)
        .map_err(|e| AdoptionError::Native(e.to_string()))?;
    Ok(())
}

pub fn preview_adoption(
    conn: &Connection,
    snapshot: &NativeSnapshot,
    stack: &CommandStack,
    scope: &ComparisonScope,
    observed: &ObservedComparison,
    selected: &[usize],
) -> Result<PreparedAdoption, AdoptionError> {
    if !observed.is_current(&snapshot.project, stack, scope) {
        return Err(AdoptionError::Stale);
    }
    let original = &observed.prepared;
    if source(conn, &snapshot.project, original.target.address())
        .ok()
        .as_ref()
        != Some(&original.source)
    {
        return Err(AdoptionError::SourceChanged);
    }
    if selected.is_empty() {
        return Err(AdoptionError::EmptySelection);
    }
    let mut seen = BTreeSet::new();
    // Validate selection identity before deriving any mutation.
    for &index in selected {
        if !seen.insert(index) {
            return Err(AdoptionError::DuplicateRow(index));
        }
        if index >= observed.rows.len() {
            return Err(AdoptionError::MissingRow(index));
        }
    }
    let mut values = Vec::new();
    for &index in selected {
        let row = &observed.rows[index];
        if row.kind != "parameter" {
            return Err(AdoptionError::UnsupportedRow {
                index,
                kind: row.kind,
            });
        }
        let parameter = row
            .parameter
            .as_ref()
            .ok_or(AdoptionError::UnknownRow { index })?;
        if row.reason.is_some() || row.project.is_none() || row.device.is_none() {
            return Err(AdoptionError::UnknownRow { index });
        }
        if ambiguous(&original.source.image, parameter) {
            return Err(AdoptionError::AmbiguousParameter(
                parameter.reference.clone(),
            ));
        }
        values.push(ParameterAdoption {
            reference: parameter.reference.clone(),
            raw: row
                .device
                .clone()
                .ok_or(AdoptionError::UnknownRow { index })?,
        });
    }
    let command = PreparedComparisonAdoption::prepare(
        &snapshot.project,
        stack,
        original.device,
        original.revision,
        values,
    )?;
    let (project, history) = command.candidate();
    // Existing image builder revalidates ranges, activation, unions, links and
    // encodings for the entire proposed configuration before accepting any edit.
    let candidate_source = source(conn, project, original.target.address())?;
    let candidate_rows = decode(
        &candidate_source.image,
        &observed.comparison.held,
        &candidate_source.plan,
    );
    for &index in selected {
        let before = &observed.rows[index];
        let parameter = before
            .parameter
            .as_ref()
            .ok_or(AdoptionError::UnknownRow { index })?;
        let matches: Vec<_> = candidate_rows
            .iter()
            .filter(|r| {
                r.parameter
                    .as_ref()
                    .is_some_and(|p| same_placement(p, parameter))
            })
            .collect();
        if matches.len() != 1
            || matches[0].reason.is_some()
            || matches[0].project != before.device
            || matches[0].device != before.device
            || ambiguous(&candidate_source.image, parameter)
        {
            return Err(AdoptionError::CandidateFieldChanged(
                parameter.reference.clone(),
            ));
        }
    }
    let mut candidate = snapshot.clone();
    candidate.project = project.clone();
    preflight(&candidate, history)?;
    Ok(PreparedAdoption {
        before: snapshot.clone(),
        scope: scope.clone(),
        target: original.target.address(),
        original_source: original.source.clone(),
        candidate_source,
        command,
        rows: observed.rows.clone(),
        selected: selected.to_vec(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptionOutcome {
    pub changed: bool,
    pub native_generation: Option<i64>,
}
#[derive(Debug, Clone, Copy)]
pub struct AdoptionNativeTarget<'a> {
    pub path: &'a Path,
    pub history_generation: i64,
}

/// Caller holds project, context, stack and source ownership throughout this
/// non-async operation. Supply `native` for a saved project. A failed native
/// write leaves all live fields untouched; the same preview may be retried.
pub fn apply_adoption(
    conn: &Connection,
    snapshot: &mut NativeSnapshot,
    stack: &mut CommandStack,
    scope: &ComparisonScope,
    preview: &PreparedAdoption,
    native: Option<AdoptionNativeTarget<'_>>,
) -> Result<AdoptionOutcome, AdoptionError> {
    if *scope != preview.scope
        || *snapshot != preview.before
        || !preview.command.is_current(&snapshot.project, stack)
    {
        return Err(AdoptionError::Stale);
    }
    let (project, history) = preview.command.candidate();
    if source(conn, &snapshot.project, preview.target)
        .ok()
        .as_ref()
        != Some(&preview.original_source)
        || source(conn, project, preview.target).ok().as_ref() != Some(&preview.candidate_source)
    {
        return Err(AdoptionError::SourceChanged);
    }
    let mut candidate = snapshot.clone();
    candidate.project = project.clone();
    preflight(&candidate, history)?;
    if !preview.changed() {
        return Ok(AdoptionOutcome {
            changed: false,
            native_generation: None,
        });
    }
    let native_generation = native
        .map(|target| {
            let conn = knx_store::project_history::open_editor_store(target.path, false)
                .map_err(|e| AdoptionError::Native(e.to_string()))?;
            let before_hash = preview
                .before
                .semantic_hash()
                .map_err(|e| AdoptionError::Native(e.to_string()))?;
            knx_store::project_history::save_editor_if_unchanged(
                &conn,
                &candidate,
                history,
                target.history_generation,
                &before_hash,
                false,
            )
            .map_err(|e| AdoptionError::Native(e.to_string()))
        })
        .transpose()?;
    // The candidate was produced through do_command, never direct field edits.
    preview.command.apply(&mut snapshot.project, stack)?;
    Ok(AdoptionOutcome {
        changed: true,
        native_generation,
    })
}

#[cfg(test)]
mod tests;
