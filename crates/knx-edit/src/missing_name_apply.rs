//! Plans, invalidates, previews and atomically applies bounded missing-name batches.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use knx_core::function::FunctionError;
use knx_core::{
    AreaId, BuildingPartId, Command, CommandError, CommandStack, DeviceId, FunctionDetails,
    FunctionId, GroupAddressId, GroupRangeId, InstallationId, LineId, Project,
};
use knx_store::project_history::{self as history, NativeSnapshot};
use serde::{Deserialize, Serialize};

use crate::missing_name_recipes::{
    propose_missing_names_for_targets, MissingNameEvidence, MissingNameProposal,
};
use crate::missing_names::{NameTargetId, NameTargetKind};
use crate::name_recipe::{NameRecipeNativeTarget, NameRecipeOutcome, NameRecipeRevision};

pub const MAX_MISSING_NAME_SELECTION: usize = 256;
pub const MAX_MISSING_NAME_PREVIEW_BYTES: usize = 1024 * 1024;

const MAX_AUTHORITY_BYTES: usize = 256;
const MAX_INCARNATION_BYTES: usize = 128;
const MAX_REVISION_GENERATION: u64 = 9_007_199_254_740_991;
const CONFIRMATION_DOMAIN: &str = "missing-name-apply-v1";

/// One target-name pair in a missing-name draft or apply batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingNameEntry {
    pub target: NameTargetId,
    pub name: String,
}

/// Bounded user-confirmed batch intent for missing-name apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingNameApplyIntent {
    pub entries: Vec<MissingNameEntry>,
}

/// Scope in which sibling names are checked for Q13 duplicate warnings.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DuplicateNameScope {
    ProjectInstallations,
    BuildingPartParent {
        installation: u8,
        parent: Option<u32>,
    },
    InstallationAreas {
        installation: u8,
    },
    AreaLines {
        installation: u8,
        area: Option<u32>,
    },
    BuildingPartDevices {
        building_part: u32,
    },
    LineDevices {
        line: u32,
    },
    ProjectUnassignedDevices,
    GroupRangeParent {
        installation: u8,
        parent: Option<u32>,
    },
    GroupRangeAddresses {
        installation: u8,
        range: Option<u32>,
    },
    BuildingPartFunctions {
        location: u32,
    },
}

/// Whether a Q13 duplicate warning collides with an existing sibling or a batch peer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "source", content = "target", rename_all = "snake_case")]
pub enum DuplicateConflictSource {
    ExistingSibling(NameTargetId),
    BatchSibling(NameTargetId),
}

/// Explicit non-blocking duplicate-name warning emitted under Q13.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateNameWarning {
    pub target: NameTargetId,
    pub proposed_name: String,
    pub scope: DuplicateNameScope,
    pub conflicts_with: Vec<DuplicateConflictSource>,
}

/// One resolved row in a missing-name apply preview, ordered parent-before-child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingNameApplyRow {
    pub target: NameTargetId,
    pub kind: NameTargetKind,
    pub before: String,
    pub after: String,
}

/// Revision-bound preview of a missing-name batch apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingNameApplyPreview {
    pub version: u8,
    pub rows: Vec<MissingNameApplyRow>,
    pub changed: usize,
    pub warnings: Vec<DuplicateNameWarning>,
}

/// Result of Q16 parent-change invalidation and non-mutating child proposal re-evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingNameReevaluation {
    pub invalidated_targets: Vec<NameTargetId>,
    pub proposals: Vec<MissingNameProposal>,
}

/// Fail-closed error surface for missing-name preview, re-evaluation, and apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissingNameApplyError {
    InvalidRevision,
    InvalidAuthority,
    InvalidSelection,
    Budget,
    DuplicateSelection(NameTargetId),
    MissingTarget(NameTargetId),
    AmbiguousTarget(NameTargetId),
    ReservedTarget(NameTargetId),
    TargetNotBlank(NameTargetId),
    InvalidName(NameTargetId, knx_core::names::NameError),
    InvalidFunctionName(NameTargetId, FunctionError),
    InvalidConfirmation,
    Conflict,
    Command(CommandError),
    Persistence(String),
}

impl std::fmt::Display for MissingNameApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing-name apply refused: {self:?}")
    }
}

impl std::error::Error for MissingNameApplyError {}

/// Revision-bound missing-name apply plan holding the dry-run verified batch command.
#[derive(Debug)]
pub struct MissingNameApplyPlan {
    before: NativeSnapshot,
    revision: NameRecipeRevision,
    intent: MissingNameApplyIntent,
    command: Command,
    preview: MissingNameApplyPreview,
}

impl MissingNameApplyPlan {
    pub fn preview(&self) -> &MissingNameApplyPreview {
        &self.preview
    }

    pub fn revision(&self) -> &NameRecipeRevision {
        &self.revision
    }

    /// Bind a stateless consumer's exact authority, revision, snapshot, intent, and preview.
    pub fn confirmation_token(&self, authority: &str) -> Result<String, MissingNameApplyError> {
        if authority.is_empty() || authority.len() > MAX_AUTHORITY_BYTES {
            return Err(MissingNameApplyError::InvalidAuthority);
        }
        let hash = self
            .before
            .semantic_hash()
            .map_err(|e| MissingNameApplyError::Persistence(e.to_string()))?;
        let bytes = serde_json::to_vec(&(
            CONFIRMATION_DOMAIN,
            authority,
            &self.revision,
            hash,
            &self.intent,
            &self.preview,
        ))
        .map_err(|e| MissingNameApplyError::Persistence(e.to_string()))?;
        Ok(history::image_hash(&bytes))
    }
}

/// Deterministically identify all dependent child targets affected by parent draft changes.
///
/// Returned targets are deduplicated, exclude `changed_parents` themselves, and are ordered
/// strictly parent-before-child (Q16).
pub fn invalidated_dependents(
    project: &Project,
    changed_parents: &[NameTargetId],
) -> Vec<NameTargetId> {
    if changed_parents.is_empty() {
        return Vec::new();
    }
    let children_map = build_dependency_graph(project);
    let seeds: BTreeSet<NameTargetId> = changed_parents.iter().copied().collect();
    let mut visited: BTreeSet<NameTargetId> = seeds.clone();
    let mut dependents: BTreeSet<NameTargetId> = BTreeSet::new();
    let mut queue: VecDeque<NameTargetId> = seeds.iter().copied().collect();

    while let Some(node) = queue.pop_front() {
        if let Some(children) = children_map.get(&node) {
            for &child in children {
                if visited.insert(child) {
                    dependents.insert(child);
                    queue.push_back(child);
                }
            }
        }
    }

    let mut ordered: Vec<NameTargetId> = dependents.into_iter().collect();
    let order_index = ParentOrderIndex::build(project);
    ordered.sort_by_key(|target| order_index.key(*target));
    ordered
}

/// Re-compute proposals for affected children using draft parent names without mutating `snapshot`.
///
/// `changed_parents` lists parents that were selected, edited, or deselected; `draft_parents` holds
/// the remaining active draft parent names to overlay onto a cloned `Project`. Always validates
/// `draft_parents` (uniqueness, Q5 blank-only guard, name admission, <= 256 entries) even when
/// `changed_parents` is empty, then re-runs evidence-based proposals only for invalidated children
/// while preserving exact provenance.
pub fn reevaluate_affected_proposals(
    snapshot: &NativeSnapshot,
    evidence: &[MissingNameEvidence],
    changed_parents: &[NameTargetId],
    draft_parents: &[MissingNameEntry],
) -> Result<MissingNameReevaluation, MissingNameApplyError> {
    if draft_parents.len() > MAX_MISSING_NAME_SELECTION {
        return Err(MissingNameApplyError::InvalidSelection);
    }

    let resolved = validate_and_resolve_entries(&snapshot.project, draft_parents)?;
    if changed_parents.is_empty() {
        return Ok(MissingNameReevaluation {
            invalidated_targets: Vec::new(),
            proposals: Vec::new(),
        });
    }

    let invalidated_targets = invalidated_dependents(&snapshot.project, changed_parents);
    if invalidated_targets.is_empty() {
        return Ok(MissingNameReevaluation {
            invalidated_targets,
            proposals: Vec::new(),
        });
    }

    let invalidated_set: BTreeSet<NameTargetId> = invalidated_targets.iter().copied().collect();
    let mut proposals = if resolved.is_empty() {
        propose_missing_names_for_targets(&snapshot.project, evidence, &invalidated_set)
    } else {
        let mut draft_project = snapshot.project.clone();
        let commands: Vec<Command> = resolved
            .iter()
            .map(|entry| rename_command(entry.target, entry.after.clone()))
            .collect();
        Command::Batch(commands)
            .apply(&mut draft_project)
            .map_err(MissingNameApplyError::Command)?;
        propose_missing_names_for_targets(&draft_project, evidence, &invalidated_set)
    };

    let order_index = ParentOrderIndex::build(&snapshot.project);
    proposals.sort_by_key(|proposal| order_index.key(proposal.candidate.target));

    Ok(MissingNameReevaluation {
        invalidated_targets,
        proposals,
    })
}

/// Build a revision-bound preview and dry-run verified batch plan for missing-name apply.
pub fn preview_missing_name_apply(
    snapshot: &NativeSnapshot,
    revision: &NameRecipeRevision,
    intent: MissingNameApplyIntent,
) -> Result<MissingNameApplyPlan, MissingNameApplyError> {
    validate_revision(revision)?;
    if intent.entries.is_empty() || intent.entries.len() > MAX_MISSING_NAME_SELECTION {
        return Err(MissingNameApplyError::InvalidSelection);
    }

    let resolved = validate_and_resolve_entries(&snapshot.project, &intent.entries)?;

    let mut canonical_entries = Vec::with_capacity(resolved.len());
    let mut rows = Vec::with_capacity(resolved.len());
    let mut commands = Vec::with_capacity(resolved.len());
    for entry in &resolved {
        canonical_entries.push(MissingNameEntry {
            target: entry.target,
            name: entry.after.clone(),
        });
        commands.push(rename_command(entry.target, entry.after.clone()));
        rows.push(MissingNameApplyRow {
            target: entry.target,
            kind: entry.target.kind(),
            before: entry.before.clone(),
            after: entry.after.clone(),
        });
    }

    let changed = commands.len();
    let mut preview = MissingNameApplyPreview {
        version: 1,
        rows: Vec::new(),
        changed,
        warnings: Vec::new(),
    };
    let mut budget = PreviewBudget(MAX_MISSING_NAME_PREVIEW_BYTES);
    budget.charge(&preview)?;
    for (index, row) in rows.iter().enumerate() {
        budget.separator(index)?;
        budget.charge(row)?;
    }
    preview.rows = rows;
    preview.warnings = collect_duplicate_warnings(&snapshot.project, &resolved, &mut budget)?;
    let command = Command::Batch(commands);
    command
        .apply(&mut snapshot.project.clone())
        .map_err(MissingNameApplyError::Command)?;

    Ok(MissingNameApplyPlan {
        before: snapshot.clone(),
        revision: revision.clone(),
        intent: MissingNameApplyIntent {
            entries: canonical_entries,
        },
        command,
        preview,
    })
}

/// Atomically apply a previously previewed missing-name batch plan after verifying confirmation.
///
/// In native mode (`native = Some(..)`), writes to SQLite with optimistic concurrency BEFORE
/// publishing to `*snapshot` and `*stack` (Q14). In session-only mode (`native = None`), applies
/// strictly in memory without touching disk.
pub fn apply_missing_name_apply(
    snapshot: &mut NativeSnapshot,
    stack: &mut CommandStack,
    revision: &NameRecipeRevision,
    plan: &MissingNameApplyPlan,
    authority: &str,
    confirmation_token: &str,
    native: Option<NameRecipeNativeTarget<'_>>,
) -> Result<NameRecipeOutcome, MissingNameApplyError> {
    if snapshot != &plan.before || revision != &plan.revision {
        return Err(MissingNameApplyError::Conflict);
    }
    let expected_token = plan.confirmation_token(authority)?;
    if !constant_time_eq(expected_token.as_bytes(), confirmation_token.as_bytes()) {
        return Err(MissingNameApplyError::InvalidConfirmation);
    }
    if plan.preview.changed == 0 {
        return Ok(NameRecipeOutcome {
            changed: false,
            native_generation: None,
        });
    }

    let mut candidate = snapshot.clone();
    let mut candidate_stack = stack.clone();
    candidate_stack
        .do_command(&mut candidate.project, plan.command.clone())
        .map_err(MissingNameApplyError::Command)?;

    let native_generation = if let Some(target) = native {
        let expected_hash = plan
            .before
            .semantic_hash()
            .map_err(|e| MissingNameApplyError::Persistence(e.to_string()))?;
        let conn = history::open_editor_store(target.path, false).map_err(map_history_error)?;
        Some(
            history::save_editor_if_unchanged(
                &conn,
                &candidate,
                &candidate_stack,
                target.history_generation,
                &expected_hash,
                false,
            )
            .map_err(map_history_error)?,
        )
    } else {
        None
    };

    *snapshot = candidate;
    *stack = candidate_stack;
    Ok(NameRecipeOutcome {
        changed: true,
        native_generation,
    })
}

fn map_history_error(error: history::HistoryError) -> MissingNameApplyError {
    match error {
        history::HistoryError::Stale => MissingNameApplyError::Conflict,
        other => MissingNameApplyError::Persistence(other.to_string()),
    }
}

struct ResolvedEntry {
    target: NameTargetId,
    before: String,
    after: String,
}

fn validate_revision(revision: &NameRecipeRevision) -> Result<(), MissingNameApplyError> {
    if revision.project_incarnation.is_empty()
        || revision.project_incarnation.len() > MAX_INCARNATION_BYTES
        || revision.generation > MAX_REVISION_GENERATION
    {
        return Err(MissingNameApplyError::InvalidRevision);
    }
    Ok(())
}

fn validate_and_resolve_entries(
    project: &Project,
    entries: &[MissingNameEntry],
) -> Result<Vec<ResolvedEntry>, MissingNameApplyError> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    let mut selected = BTreeSet::new();
    let mut text_bytes = 0usize;
    let mut resolved = Vec::with_capacity(entries.len());

    for entry in entries {
        let target = entry.target;
        if !selected.insert(target) {
            return Err(MissingNameApplyError::DuplicateSelection(target));
        }
        let current = resolve_unique_name(project, target)?;
        if !knx_core::names::is_blank_name(current) {
            return Err(MissingNameApplyError::TargetNotBlank(target));
        }
        validate_proposed_name(target, &entry.name)?;
        text_bytes = text_bytes
            .checked_add(current.len())
            .ok_or(MissingNameApplyError::Budget)?;
        if text_bytes > MAX_MISSING_NAME_PREVIEW_BYTES {
            return Err(MissingNameApplyError::Budget);
        }
        text_bytes = text_bytes
            .checked_add(entry.name.len())
            .ok_or(MissingNameApplyError::Budget)?;
        if text_bytes > MAX_MISSING_NAME_PREVIEW_BYTES {
            return Err(MissingNameApplyError::Budget);
        }
        resolved.push(ResolvedEntry {
            target,
            before: current.to_owned(),
            after: entry.name.clone(),
        });
    }

    let order_index = ParentOrderIndex::build(project);
    resolved.sort_by_key(|entry| order_index.key(entry.target));
    Ok(resolved)
}

fn resolve_unique_name(
    project: &Project,
    target: NameTargetId,
) -> Result<&str, MissingNameApplyError> {
    match target {
        NameTargetId::Project => Ok(project.info.name.as_str()),
        NameTargetId::Installation(id) => unique_from_iter(
            target,
            project
                .installations
                .iter()
                .filter(|inst| inst.id.0 == id)
                .map(|inst| inst.name.as_str()),
        ),
        NameTargetId::BuildingPart(id) => unique_from_iter(
            target,
            project
                .installations
                .iter()
                .flat_map(|inst| &inst.buildings)
                .filter(|bp| bp.id.0 == id)
                .map(|bp| bp.name.as_str()),
        ),
        NameTargetId::Area(id) => unique_from_iter(
            target,
            project
                .installations
                .iter()
                .flat_map(|inst| &inst.topology.areas)
                .filter(|area| area.id.0 == id)
                .map(|area| area.name.as_str()),
        ),
        NameTargetId::Line(id) => unique_from_iter(
            target,
            project
                .installations
                .iter()
                .flat_map(|inst| &inst.topology.lines)
                .filter(|line| line.id.0 == id)
                .map(|line| line.name.as_str()),
        ),
        NameTargetId::Device(id) => project
            .devices
            .get(DeviceId(id))
            .map(|device| device.name.as_str())
            .ok_or(MissingNameApplyError::MissingTarget(target)),
        NameTargetId::GroupRange(id) => unique_from_iter(
            target,
            project
                .installations
                .iter()
                .flat_map(|inst| &inst.group_ranges)
                .filter(|range| range.id.0 == id)
                .map(|range| range.name.as_str()),
        ),
        NameTargetId::GroupAddress(id) => unique_from_iter(
            target,
            project
                .installations
                .iter()
                .flat_map(|inst| &inst.group_addresses)
                .filter(|ga| ga.id.0 == id)
                .map(|ga| ga.name.as_str()),
        ),
        NameTargetId::Function(id) => {
            if id == 0 {
                return Err(MissingNameApplyError::ReservedTarget(target));
            }
            unique_from_iter(
                target,
                project
                    .functions
                    .iter()
                    .filter(|func| func.id.0 == id)
                    .map(|func| func.details.name.as_str()),
            )
        }
    }
}

fn unique_from_iter<'a, I>(
    target: NameTargetId,
    mut iter: I,
) -> Result<&'a str, MissingNameApplyError>
where
    I: Iterator<Item = &'a str>,
{
    let first = iter
        .next()
        .ok_or(MissingNameApplyError::MissingTarget(target))?;
    if iter.next().is_some() {
        return Err(MissingNameApplyError::AmbiguousTarget(target));
    }
    Ok(first)
}

fn validate_proposed_name(target: NameTargetId, name: &str) -> Result<(), MissingNameApplyError> {
    knx_core::names::validate_edited_name(name)
        .map_err(|e| MissingNameApplyError::InvalidName(target, e))?;
    if matches!(target, NameTargetId::Function(_)) {
        FunctionDetails::validate_name(name)
            .map_err(|e| MissingNameApplyError::InvalidFunctionName(target, e))?;
    }
    Ok(())
}

fn rename_command(target: NameTargetId, name: String) -> Command {
    match target {
        NameTargetId::Project => Command::RenameProject { name },
        NameTargetId::Installation(id) => Command::RenameInstallation {
            id: InstallationId(id),
            name,
        },
        NameTargetId::BuildingPart(id) => Command::RenameBuildingPart {
            id: BuildingPartId(id),
            name,
        },
        NameTargetId::Area(id) => Command::RenameArea {
            id: AreaId(id),
            name,
        },
        NameTargetId::Line(id) => Command::RenameLine {
            id: LineId(id),
            name,
        },
        NameTargetId::Device(id) => Command::RenameDevice {
            device: DeviceId(id),
            name,
        },
        NameTargetId::GroupRange(id) => Command::RenameGroupRange {
            id: GroupRangeId(id),
            name,
        },
        NameTargetId::GroupAddress(id) => Command::RenameGroupAddress {
            id: GroupAddressId(id),
            name,
        },
        NameTargetId::Function(id) => Command::RenameFunction {
            id: FunctionId(id),
            name,
        },
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Deterministic ordering over the same dependency edges used by invalidation.
struct ParentOrderIndex {
    positions: BTreeMap<NameTargetId, usize>,
}

impl ParentOrderIndex {
    fn build(project: &Project) -> Self {
        let edges = build_dependency_graph(project);
        let mut incoming = BTreeMap::from([(NameTargetId::Project, 0usize)]);
        for device in project.devices.iter() {
            incoming
                .entry(NameTargetId::Device(device.id.0))
                .or_default();
        }
        for (&parent, children) in &edges {
            incoming.entry(parent).or_default();
            for &child in children {
                *incoming.entry(child).or_default() += 1;
            }
        }
        let mut ready: BTreeSet<_> = incoming
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(&target, _)| (Self::kind_key(target), target))
            .collect();
        let mut positions = BTreeMap::new();
        while !incoming.is_empty() {
            // Cycles cannot satisfy every edge: break ties deterministically, then
            // resume releasing children so their acyclic dependencies still order them.
            let target = match ready.pop_first() {
                Some((_, target)) => target,
                None => *incoming.keys().min_by_key(|&&t| Self::kind_key(t)).unwrap(),
            };
            incoming.remove(&target);
            positions.insert(target, positions.len());
            if let Some(children) = edges.get(&target) {
                for child in children {
                    if let Some(count) = incoming.get_mut(child) {
                        *count -= 1;
                        if *count == 0 {
                            ready.insert((Self::kind_key(*child), *child));
                        }
                    }
                }
            }
        }
        Self { positions }
    }

    fn key(&self, target: NameTargetId) -> (usize, (u8, u32)) {
        (
            self.positions.get(&target).copied().unwrap_or(usize::MAX),
            Self::kind_key(target),
        )
    }

    fn kind_key(target: NameTargetId) -> (u8, u32) {
        match target {
            NameTargetId::Project => (0, 0),
            NameTargetId::Installation(id) => (1, u32::from(id)),
            NameTargetId::BuildingPart(id) => (2, id),
            NameTargetId::Area(id) => (3, id),
            NameTargetId::Line(id) => (4, id),
            NameTargetId::GroupRange(id) => (5, id),
            NameTargetId::Device(id) => (6, id),
            NameTargetId::Function(id) => (7, id),
            NameTargetId::GroupAddress(id) => (8, id),
        }
    }
}

fn build_dependency_graph(project: &Project) -> BTreeMap<NameTargetId, BTreeSet<NameTargetId>> {
    let mut edges: BTreeMap<NameTargetId, BTreeSet<NameTargetId>> = BTreeMap::new();
    let mut add = |parent: NameTargetId, child: NameTargetId| {
        if parent != child {
            edges.entry(parent).or_default().insert(child);
        }
    };

    for inst in &project.installations {
        let inst_id = NameTargetId::Installation(inst.id.0);
        add(NameTargetId::Project, inst_id);

        for bp in &inst.buildings {
            let bp_id = NameTargetId::BuildingPart(bp.id.0);
            add(inst_id, bp_id);
            if let Some(parent) = bp.parent {
                add(NameTargetId::BuildingPart(parent.0), bp_id);
            }
            for child in &bp.children {
                add(bp_id, NameTargetId::BuildingPart(child.0));
            }
            for dev in &bp.devices {
                add(bp_id, NameTargetId::Device(dev.0));
            }
        }

        for area in &inst.topology.areas {
            let area_id = NameTargetId::Area(area.id.0);
            add(inst_id, area_id);
            for line in &area.lines {
                add(area_id, NameTargetId::Line(line.0));
            }
        }

        for line in &inst.topology.lines {
            let line_id = NameTargetId::Line(line.id.0);
            add(inst_id, line_id);
            for dev in &line.devices {
                add(line_id, NameTargetId::Device(dev.0));
            }
        }
        for dev in &inst.topology.unassigned {
            add(inst_id, NameTargetId::Device(dev.0));
        }

        for range in &inst.group_ranges {
            let range_id = NameTargetId::GroupRange(range.id.0);
            add(inst_id, range_id);
            if let Some(parent) = range.parent {
                add(NameTargetId::GroupRange(parent.0), range_id);
            }
            for child in &range.children {
                add(range_id, NameTargetId::GroupRange(child.0));
            }
        }

        for ga in &inst.group_addresses {
            let ga_id = NameTargetId::GroupAddress(ga.id.0);
            add(inst_id, ga_id);
            if let Some(range) = ga.range {
                add(NameTargetId::GroupRange(range.0), ga_id);
            }
        }
    }

    for device in project.devices.iter() {
        let dev_id = NameTargetId::Device(device.id.0);
        for co_id in &device.com_objects {
            if let Some(co) = project.devices.com_object(*co_id) {
                for link in &co.links {
                    add(dev_id, NameTargetId::GroupAddress(link.ga.0));
                }
            }
        }
    }
    for co in project.devices.com_objects() {
        let dev_id = NameTargetId::Device(co.device.0);
        for link in &co.links {
            add(dev_id, NameTargetId::GroupAddress(link.ga.0));
        }
    }

    for func in &project.functions {
        let fn_id = NameTargetId::Function(func.id.0);
        add(NameTargetId::BuildingPart(func.location.0), fn_id);
        for member in &func.members {
            add(fn_id, NameTargetId::GroupAddress(member.group_address.0));
        }
    }

    edges
}

struct DuplicateScopeIndex {
    line_scopes: BTreeMap<(usize, LineId), BTreeSet<DuplicateNameScope>>,
    device_scopes: BTreeMap<DeviceId, BTreeSet<DuplicateNameScope>>,
}

impl DuplicateScopeIndex {
    fn build(project: &Project) -> Self {
        let mut line_scopes: BTreeMap<(usize, LineId), BTreeSet<DuplicateNameScope>> =
            BTreeMap::new();
        let mut device_scopes: BTreeMap<DeviceId, BTreeSet<DuplicateNameScope>> = BTreeMap::new();

        for (inst_idx, inst) in project.installations.iter().enumerate() {
            for bp in &inst.buildings {
                let scope = DuplicateNameScope::BuildingPartDevices {
                    building_part: bp.id.0,
                };
                for &dev_id in &bp.devices {
                    device_scopes
                        .entry(dev_id)
                        .or_default()
                        .insert(scope.clone());
                }
            }

            let mut line_areas: BTreeMap<LineId, BTreeSet<u32>> = BTreeMap::new();
            for area in &inst.topology.areas {
                for &line_id in &area.lines {
                    line_areas.entry(line_id).or_default().insert(area.id.0);
                }
            }

            for line in &inst.topology.lines {
                let scopes = line_scopes.entry((inst_idx, line.id)).or_default();
                if let Some(areas) = line_areas.get(&line.id) {
                    for &area_id in areas {
                        scopes.insert(DuplicateNameScope::AreaLines {
                            installation: inst.id.0,
                            area: Some(area_id),
                        });
                    }
                } else {
                    scopes.insert(DuplicateNameScope::AreaLines {
                        installation: inst.id.0,
                        area: None,
                    });
                }

                let dev_scope = DuplicateNameScope::LineDevices { line: line.id.0 };
                for &dev_id in &line.devices {
                    device_scopes
                        .entry(dev_id)
                        .or_default()
                        .insert(dev_scope.clone());
                }
            }
        }

        Self {
            line_scopes,
            device_scopes,
        }
    }

    fn scopes_for_device(&self, device: DeviceId) -> Vec<DuplicateNameScope> {
        match self.device_scopes.get(&device) {
            Some(scopes) if !scopes.is_empty() => scopes.iter().cloned().collect(),
            _ => vec![DuplicateNameScope::ProjectUnassignedDevices],
        }
    }
}

/// Counts the exact JSON payload without allocating serialized rows or conflict vectors.
struct PreviewBudget(usize);

impl PreviewBudget {
    fn charge(&mut self, value: &impl Serialize) -> Result<(), MissingNameApplyError> {
        serde_json::to_writer(self, value).map_err(|_| MissingNameApplyError::Budget)
    }

    fn separator(&mut self, index: usize) -> Result<(), MissingNameApplyError> {
        if index > 0 {
            std::io::Write::write_all(self, b",").map_err(|_| MissingNameApplyError::Budget)?;
        }
        Ok(())
    }
}

impl std::io::Write for PreviewBudget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("missing-name preview budget exceeded"))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn collect_duplicate_warnings(
    project: &Project,
    resolved: &[ResolvedEntry],
    budget: &mut PreviewBudget,
) -> Result<Vec<DuplicateNameWarning>, MissingNameApplyError> {
    let scope_index = DuplicateScopeIndex::build(project);
    let mut existing_by_scope: BTreeMap<DuplicateNameScope, Vec<(NameTargetId, String)>> =
        BTreeMap::new();
    index_existing_names(project, &scope_index, &mut existing_by_scope);

    let mut batch_by_scope: BTreeMap<DuplicateNameScope, Vec<(NameTargetId, &str)>> =
        BTreeMap::new();
    let mut entry_scopes: Vec<Vec<DuplicateNameScope>> = Vec::with_capacity(resolved.len());
    for entry in resolved {
        let scopes = scopes_for_target(project, &scope_index, entry.target);
        for scope in &scopes {
            batch_by_scope
                .entry(scope.clone())
                .or_default()
                .push((entry.target, entry.after.as_str()));
        }
        entry_scopes.push(scopes);
    }

    let mut warnings = Vec::new();
    for (entry, scopes) in resolved.iter().zip(entry_scopes) {
        for scope in scopes {
            let mut conflicts = BTreeSet::new();
            let mut add_conflict = |conflict| -> Result<(), MissingNameApplyError> {
                if !conflicts.contains(&conflict) {
                    if conflicts.is_empty() {
                        budget.separator(warnings.len())?;
                        budget.charge(&DuplicateNameWarning {
                            target: entry.target,
                            proposed_name: entry.after.clone(),
                            scope: scope.clone(),
                            conflicts_with: Vec::new(),
                        })?;
                    }
                    budget.separator(conflicts.len())?;
                    budget.charge(&conflict)?;
                    conflicts.insert(conflict);
                }
                Ok(())
            };
            if let Some(siblings) = existing_by_scope.get(&scope) {
                for (sib_id, sib_name) in siblings {
                    if *sib_id != entry.target && sib_name == &entry.after {
                        add_conflict(DuplicateConflictSource::ExistingSibling(*sib_id))?;
                    }
                }
            }
            if let Some(batch_siblings) = batch_by_scope.get(&scope) {
                for (peer_id, peer_name) in batch_siblings {
                    if *peer_id != entry.target && *peer_name == entry.after.as_str() {
                        add_conflict(DuplicateConflictSource::BatchSibling(*peer_id))?;
                    }
                }
            }
            if !conflicts.is_empty() {
                warnings.push(DuplicateNameWarning {
                    target: entry.target,
                    proposed_name: entry.after.clone(),
                    scope,
                    conflicts_with: conflicts.into_iter().collect(),
                });
            }
        }
    }

    Ok(warnings)
}

fn index_existing_names(
    project: &Project,
    scope_index: &DuplicateScopeIndex,
    out: &mut BTreeMap<DuplicateNameScope, Vec<(NameTargetId, String)>>,
) {
    let mut record = |scope: DuplicateNameScope, target: NameTargetId, name: &str| {
        if !knx_core::names::is_blank_name(name) {
            out.entry(scope)
                .or_default()
                .push((target, name.to_owned()));
        }
    };

    for (inst_idx, inst) in project.installations.iter().enumerate() {
        record(
            DuplicateNameScope::ProjectInstallations,
            NameTargetId::Installation(inst.id.0),
            &inst.name,
        );
        for bp in &inst.buildings {
            record(
                DuplicateNameScope::BuildingPartParent {
                    installation: inst.id.0,
                    parent: bp.parent.map(|p| p.0),
                },
                NameTargetId::BuildingPart(bp.id.0),
                &bp.name,
            );
        }
        for area in &inst.topology.areas {
            record(
                DuplicateNameScope::InstallationAreas {
                    installation: inst.id.0,
                },
                NameTargetId::Area(area.id.0),
                &area.name,
            );
        }
        for line in &inst.topology.lines {
            if let Some(scopes) = scope_index.line_scopes.get(&(inst_idx, line.id)) {
                for scope in scopes {
                    record(scope.clone(), NameTargetId::Line(line.id.0), &line.name);
                }
            }
        }
        for range in &inst.group_ranges {
            record(
                DuplicateNameScope::GroupRangeParent {
                    installation: inst.id.0,
                    parent: range.parent.map(|p| p.0),
                },
                NameTargetId::GroupRange(range.id.0),
                &range.name,
            );
        }
        for ga in &inst.group_addresses {
            record(
                DuplicateNameScope::GroupRangeAddresses {
                    installation: inst.id.0,
                    range: ga.range.map(|r| r.0),
                },
                NameTargetId::GroupAddress(ga.id.0),
                &ga.name,
            );
        }
    }

    for device in project.devices.iter() {
        if knx_core::names::is_blank_name(&device.name) {
            continue;
        }
        for scope in scope_index.scopes_for_device(device.id) {
            record(scope, NameTargetId::Device(device.id.0), &device.name);
        }
    }

    for func in &project.functions {
        record(
            DuplicateNameScope::BuildingPartFunctions {
                location: func.location.0,
            },
            NameTargetId::Function(func.id.0),
            &func.details.name,
        );
    }
}

fn scopes_for_target(
    project: &Project,
    scope_index: &DuplicateScopeIndex,
    target: NameTargetId,
) -> Vec<DuplicateNameScope> {
    match target {
        NameTargetId::Project => Vec::new(),
        NameTargetId::Installation(_) => vec![DuplicateNameScope::ProjectInstallations],
        NameTargetId::BuildingPart(id) => {
            for inst in &project.installations {
                if let Some(bp) = inst.buildings.iter().find(|bp| bp.id.0 == id) {
                    return vec![DuplicateNameScope::BuildingPartParent {
                        installation: inst.id.0,
                        parent: bp.parent.map(|p| p.0),
                    }];
                }
            }
            Vec::new()
        }
        NameTargetId::Area(id) => {
            for inst in &project.installations {
                if inst.topology.areas.iter().any(|a| a.id.0 == id) {
                    return vec![DuplicateNameScope::InstallationAreas {
                        installation: inst.id.0,
                    }];
                }
            }
            Vec::new()
        }
        NameTargetId::Line(id) => {
            for (inst_idx, inst) in project.installations.iter().enumerate() {
                if let Some(line) = inst.topology.lines.iter().find(|l| l.id.0 == id) {
                    return scope_index
                        .line_scopes
                        .get(&(inst_idx, line.id))
                        .map(|scopes| scopes.iter().cloned().collect())
                        .unwrap_or_default();
                }
            }
            Vec::new()
        }
        NameTargetId::Device(id) => scope_index.scopes_for_device(DeviceId(id)),
        NameTargetId::GroupRange(id) => {
            for inst in &project.installations {
                if let Some(range) = inst.group_ranges.iter().find(|r| r.id.0 == id) {
                    return vec![DuplicateNameScope::GroupRangeParent {
                        installation: inst.id.0,
                        parent: range.parent.map(|p| p.0),
                    }];
                }
            }
            Vec::new()
        }
        NameTargetId::GroupAddress(id) => {
            for inst in &project.installations {
                if let Some(ga) = inst.group_addresses.iter().find(|g| g.id.0 == id) {
                    return vec![DuplicateNameScope::GroupRangeAddresses {
                        installation: inst.id.0,
                        range: ga.range.map(|r| r.0),
                    }];
                }
            }
            Vec::new()
        }
        NameTargetId::Function(id) => {
            if let Some(func) = project.functions.iter().find(|f| f.id.0 == id) {
                vec![DuplicateNameScope::BuildingPartFunctions {
                    location: func.location.0,
                }]
            } else {
                Vec::new()
            }
        }
    }
}
