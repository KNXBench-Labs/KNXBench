//! Read-only missing-name detection and suggestion provenance model for all Q9 entities.

use std::collections::{BTreeMap, BTreeSet};

use knx_core::function::MAX_FUNCTION_SHORT_TEXT_CODEPOINTS;
use knx_core::names::{is_blank_name, MAX_EDITED_NAME_CODEPOINTS};
use knx_core::{BuildingPartType, Project};
use serde::{Deserialize, Serialize};

/// Default upper bound on Q9 entities inspected in a single bounded scan (Q17).
pub const DEFAULT_SCAN_LIMIT: usize = 10_000;
/// Default page size for paginated candidate retrieval (Q17).
pub const DEFAULT_PAGE_SIZE: usize = 100;

/// Entity categories eligible for missing-name detection and renaming (Q9).
///
/// Explicitly excludes product/manufacturer data, communication-object text/descriptions,
/// function-member labels, binary data refs, and free-text descriptions/comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameTargetKind {
    Project,
    Installation,
    BuildingPart,
    Area,
    Line,
    Device,
    GroupRange,
    GroupAddress,
    Function,
}

/// Stable identifier of a Q9 rename target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum NameTargetId {
    Project,
    Installation(u8),
    BuildingPart(u32),
    Area(u32),
    Line(u32),
    Device(u32),
    GroupRange(u32),
    GroupAddress(u32),
    Function(u32),
}

impl NameTargetId {
    pub const fn kind(self) -> NameTargetKind {
        match self {
            Self::Project => NameTargetKind::Project,
            Self::Installation(_) => NameTargetKind::Installation,
            Self::BuildingPart(_) => NameTargetKind::BuildingPart,
            Self::Area(_) => NameTargetKind::Area,
            Self::Line(_) => NameTargetKind::Line,
            Self::Device(_) => NameTargetKind::Device,
            Self::GroupRange(_) => NameTargetKind::GroupRange,
            Self::GroupAddress(_) => NameTargetKind::GroupAddress,
            Self::Function(_) => NameTargetKind::Function,
        }
    }
}

/// Serializable mirror of all 11 supported [`BuildingPartType`] kinds (Q9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingPartKind {
    Building,
    Floor,
    Room,
    Corridor,
    DistributionBoard,
    BuildingPart,
    Stairway,
    RoomPart,
    Area,
    Ground,
    Segment,
}

impl From<BuildingPartType> for BuildingPartKind {
    fn from(kind: BuildingPartType) -> Self {
        match kind {
            BuildingPartType::Building => Self::Building,
            BuildingPartType::Floor => Self::Floor,
            BuildingPartType::Room => Self::Room,
            BuildingPartType::Corridor => Self::Corridor,
            BuildingPartType::DistributionBoard => Self::DistributionBoard,
            BuildingPartType::BuildingPart => Self::BuildingPart,
            BuildingPartType::Stairway => Self::Stairway,
            BuildingPartType::RoomPart => Self::RoomPart,
            BuildingPartType::Area => Self::Area,
            BuildingPartType::Ground => Self::Ground,
            BuildingPartType::Segment => Self::Segment,
        }
    }
}

/// Target identity health surfaced explicitly rather than silently collapsing duplicates (Q13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TargetIdentityStatus {
    Unique,
    DuplicateId { occurrences: usize },
    InvalidZeroId,
}

impl TargetIdentityStatus {
    pub const fn is_unique(&self) -> bool {
        matches!(self, Self::Unique)
    }
}

/// Semantic role of a parent reference in the domain hierarchy (Q16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParentRelation {
    ProjectRoot,
    Installation,
    BuildingParent,
    TopologyArea,
    TopologyLine,
    DeviceLocation,
    GroupRangeParent,
    GroupAddressRange,
    FunctionLocation,
}

/// Resolution state of a structural parent reference (Q13, Q16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ParentRefState {
    Resolved {
        target: NameTargetId,
        /// True when the resolved parent currently has a blank name (resolve parent first in 51.2).
        parent_is_blank: bool,
    },
    Missing {
        expected: Option<NameTargetId>,
    },
    Ambiguous {
        candidates: Vec<NameTargetId>,
    },
    Cyclic {
        target: NameTargetId,
    },
}

/// Explicit parent link for dependency resolution and invalidation in 51.2 (Q16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentReference {
    pub relation: ParentRelation,
    pub state: ParentRefState,
}

/// Read-only context metadata carried by a detected missing-name candidate (Q7, Q10–Q16).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateContext {
    pub installation_id: Option<u8>,
    pub source_ets_id: Option<String>,
    pub building_part_kind: Option<BuildingPartKind>,
    pub building_part_number: Option<String>,
    pub individual_address: Option<String>,
    pub topology_area_address: Option<u8>,
    pub topology_line_address: Option<u8>,
    pub group_address: Option<u16>,
    pub group_range_span: Option<(u16, u16)>,
}

/// One detected entity whose current name is blank under Q5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingNameCandidate {
    pub target: NameTargetId,
    pub kind: NameTargetKind,
    /// Exact verbatim blank name currently stored on the entity (empty or Unicode whitespace-only).
    pub current_name: String,
    /// Deterministic 0-based index among entities sharing `target`.
    pub occurrence_index: usize,
    pub identity_status: TargetIdentityStatus,
    /// Structural parent references for parent-before-child resolution and invalidation (Q16).
    pub parents: Vec<ParentReference>,
    /// Maximum Unicode code points allowed for an edited name on this entity type.
    pub max_name_codepoints: usize,
    pub context: CandidateContext,
}

/// How a proposed name was produced (Q6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameOrigin {
    /// Proposed automatically from existing read-only evidence.
    Automatic,
    /// Entered or edited manually by the user.
    Manual,
    /// No automatic suggestion could be formed; manual input is required.
    ManualRequired,
}

/// Read-only evidence categories consulted when forming suggestions in 51.2 (Q6, Q9–Q12, Q15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceKind {
    ProjectDescriptiveText,
    InstallationDescriptiveText,
    BuildingPartType,
    BuildingPartNumber,
    BuildingLocationPath,
    ProductDesignation,
    DeviceDescription,
    TopologyAddress,
    GroupRangeSpan,
    GroupAddressDescription,
    FunctionContext,
    FunctionMemberContext,
    ComObjectText,
    ParentName,
}

/// One contributing read-only evidence input recorded in suggestion provenance (Q6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceContribution {
    pub kind: EvidenceSourceKind,
    pub source_target: Option<NameTargetId>,
    pub value: String,
}

/// Missing, ambiguous, or insufficient evidence marker recorded in suggestion provenance (Q6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "gap", rename_all = "snake_case")]
pub enum EvidenceGap {
    Missing {
        kind: EvidenceSourceKind,
        detail: String,
    },
    Ambiguous {
        kind: EvidenceSourceKind,
        detail: String,
        candidates: Vec<String>,
    },
    Insufficient {
        kind: EvidenceSourceKind,
        detail: String,
    },
    UnselectedParent {
        parent: NameTargetId,
    },
}

/// Structured, visible provenance for a proposed or edited name (Q6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameProvenance {
    pub origin: NameOrigin,
    /// Contributing read-only evidence sources (empty for pure manual input).
    pub sources: Vec<EvidenceContribution>,
    /// Missing, ambiguous, or insufficient evidence inputs.
    pub gaps: Vec<EvidenceGap>,
    /// True when an automatic suggestion was subsequently edited manually.
    pub derived_from_automatic: bool,
}

impl NameProvenance {
    pub fn automatic(sources: Vec<EvidenceContribution>, gaps: Vec<EvidenceGap>) -> Self {
        Self {
            origin: NameOrigin::Automatic,
            sources,
            gaps,
            derived_from_automatic: false,
        }
    }

    pub fn manual_required(gaps: Vec<EvidenceGap>) -> Self {
        Self {
            origin: NameOrigin::ManualRequired,
            sources: Vec::new(),
            gaps,
            derived_from_automatic: false,
        }
    }

    pub fn manual(derived_from_automatic: bool) -> Self {
        Self {
            origin: NameOrigin::Manual,
            sources: Vec::new(),
            gaps: Vec::new(),
            derived_from_automatic,
        }
    }
}

/// Bounded scan and pagination parameters (Q17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingNameScanRequest {
    /// Maximum number of Q9 entities to inspect before stopping. `None` scans the whole project.
    pub max_entities_scanned: Option<usize>,
    /// 0-based pagination offset into the candidates discovered during this scan.
    pub offset: usize,
    /// Maximum number of candidates returned in `items`.
    pub limit: usize,
}

impl Default for MissingNameScanRequest {
    fn default() -> Self {
        Self {
            max_entities_scanned: Some(DEFAULT_SCAN_LIMIT),
            offset: 0,
            limit: DEFAULT_PAGE_SIZE,
        }
    }
}

impl MissingNameScanRequest {
    pub const fn unbounded(offset: usize, limit: usize) -> Self {
        Self {
            max_entities_scanned: None,
            offset,
            limit,
        }
    }

    pub const fn bounded(max_entities_scanned: usize, offset: usize, limit: usize) -> Self {
        Self {
            max_entities_scanned: Some(max_entities_scanned),
            offset,
            limit,
        }
    }
}

/// Whether detection inspected every Q9 entity in the project (Q17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanCompleteness {
    Complete,
    Incomplete,
}

/// Entity and candidate counts with strict Q17 full-total suppression on incomplete scans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingNameScanCounts {
    pub entities_scanned: usize,
    pub candidates_found_in_scan: usize,
    /// Full-project candidate total; `Some` only when `completeness == ScanCompleteness::Complete`.
    pub total_candidates: Option<usize>,
    /// Full-project per-kind totals; `Some` only when `completeness == ScanCompleteness::Complete`.
    pub total_by_kind: Option<BTreeMap<NameTargetKind, usize>>,
}

/// Deterministic, strictly read-only result of scanning a project for blank Q9 names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingNameScanResult {
    pub completeness: ScanCompleteness,
    pub counts: MissingNameScanCounts,
    pub offset: usize,
    pub limit: usize,
    pub has_more_in_scan: bool,
    pub items: Vec<MissingNameCandidate>,
}

/// Read-only index of project-wide identity counts, blank flags, and device placements.
struct ProjectIndex {
    project_is_blank: bool,
    installation_counts: BTreeMap<u8, usize>,
    installation_blank: BTreeMap<u8, bool>,
    building_part_counts: BTreeMap<u32, usize>,
    building_part_blank: BTreeMap<u32, bool>,
    area_counts: BTreeMap<u32, usize>,
    area_blank: BTreeMap<u32, bool>,
    line_counts: BTreeMap<u32, usize>,
    line_blank: BTreeMap<u32, bool>,
    group_range_counts: BTreeMap<u32, usize>,
    group_range_blank: BTreeMap<u32, bool>,
    group_address_counts: BTreeMap<u32, usize>,
    function_counts: BTreeMap<u32, usize>,
    device_lines: BTreeMap<u32, Vec<u32>>,
    device_buildings: BTreeMap<u32, Vec<u32>>,
}

impl ProjectIndex {
    fn build(project: &Project) -> Self {
        let mut idx = Self {
            project_is_blank: is_blank_name(&project.info.name),
            installation_counts: BTreeMap::new(),
            installation_blank: BTreeMap::new(),
            building_part_counts: BTreeMap::new(),
            building_part_blank: BTreeMap::new(),
            area_counts: BTreeMap::new(),
            area_blank: BTreeMap::new(),
            line_counts: BTreeMap::new(),
            line_blank: BTreeMap::new(),
            group_range_counts: BTreeMap::new(),
            group_range_blank: BTreeMap::new(),
            group_address_counts: BTreeMap::new(),
            function_counts: BTreeMap::new(),
            device_lines: BTreeMap::new(),
            device_buildings: BTreeMap::new(),
        };

        for inst in &project.installations {
            *idx.installation_counts.entry(inst.id.0).or_insert(0) += 1;
            idx.installation_blank
                .entry(inst.id.0)
                .or_insert_with(|| is_blank_name(&inst.name));

            for part in &inst.buildings {
                *idx.building_part_counts.entry(part.id.0).or_insert(0) += 1;
                idx.building_part_blank
                    .entry(part.id.0)
                    .or_insert_with(|| is_blank_name(&part.name));
                for &dev_id in &part.devices {
                    idx.device_buildings
                        .entry(dev_id.0)
                        .or_default()
                        .push(part.id.0);
                }
            }

            for area in &inst.topology.areas {
                *idx.area_counts.entry(area.id.0).or_insert(0) += 1;
                idx.area_blank
                    .entry(area.id.0)
                    .or_insert_with(|| is_blank_name(&area.name));
            }

            for line in &inst.topology.lines {
                *idx.line_counts.entry(line.id.0).or_insert(0) += 1;
                idx.line_blank
                    .entry(line.id.0)
                    .or_insert_with(|| is_blank_name(&line.name));
                for &dev_id in &line.devices {
                    idx.device_lines
                        .entry(dev_id.0)
                        .or_default()
                        .push(line.id.0);
                }
            }

            for range in &inst.group_ranges {
                *idx.group_range_counts.entry(range.id.0).or_insert(0) += 1;
                idx.group_range_blank
                    .entry(range.id.0)
                    .or_insert_with(|| is_blank_name(&range.name));
            }

            for ga in &inst.group_addresses {
                *idx.group_address_counts.entry(ga.id.0).or_insert(0) += 1;
            }
        }

        for func in &project.functions {
            *idx.function_counts.entry(func.id.0).or_insert(0) += 1;
        }

        for ids in idx.device_lines.values_mut() {
            ids.sort_unstable();
        }
        for ids in idx.device_buildings.values_mut() {
            ids.sort_unstable();
        }

        idx
    }

    fn installation_parent_ref(&self, installation_id: u8) -> ParentReference {
        let count = self
            .installation_counts
            .get(&installation_id)
            .copied()
            .unwrap_or(0);
        let state = if count > 1 {
            ParentRefState::Ambiguous {
                candidates: vec![NameTargetId::Installation(installation_id)],
            }
        } else {
            ParentRefState::Resolved {
                target: NameTargetId::Installation(installation_id),
                parent_is_blank: self
                    .installation_blank
                    .get(&installation_id)
                    .copied()
                    .unwrap_or(false),
            }
        };
        ParentReference {
            relation: ParentRelation::Installation,
            state,
        }
    }
}

fn identity_status(occurrences: usize, zero_invalid: bool) -> TargetIdentityStatus {
    if zero_invalid {
        TargetIdentityStatus::InvalidZeroId
    } else if occurrences > 1 {
        TargetIdentityStatus::DuplicateId { occurrences }
    } else {
        TargetIdentityStatus::Unique
    }
}

/// Deterministic topological ordering (parent before child) with cycle detection.
///
/// Each item is identified by its slice index `i`, stable key `ids[i]`, occurrence index
/// `occ[i]`, and optional parent key `parents[i]`. Returns `(ordered_indices, cyclic_indices)`.
fn topo_order_by_parent(
    ids: &[u32],
    occ: &[usize],
    parents: &[Option<u32>],
) -> (Vec<usize>, BTreeSet<usize>) {
    let n = ids.len();
    let mut id_to_indices: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (i, &id) in ids.iter().enumerate() {
        id_to_indices.entry(id).or_default().push(i);
    }

    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_degree: Vec<usize> = vec![0; n];
    let mut cyclic = BTreeSet::new();

    for i in 0..n {
        if let Some(parent_id) = parents[i] {
            if parent_id == ids[i] {
                cyclic.insert(i);
            } else if let Some(matches) = id_to_indices.get(&parent_id) {
                if matches.len() == 1 {
                    let p_idx = matches[0];
                    children[p_idx].push(i);
                    in_degree[i] += 1;
                }
            }
        }
    }

    let mut ready: BTreeSet<(u32, usize, usize)> = BTreeSet::new();
    for i in 0..n {
        if in_degree[i] == 0 {
            ready.insert((ids[i], occ[i], i));
        }
    }

    let mut ordered = Vec::with_capacity(n);
    let mut visited = vec![false; n];

    while ordered.len() < n {
        if ready.is_empty() {
            // Remaining unvisited nodes form or depend on at least one cycle.
            let mut next_cycle_node: Option<(u32, usize, usize)> = None;
            for i in 0..n {
                if !visited[i] {
                    cyclic.insert(i);
                    let key = (ids[i], occ[i], i);
                    if next_cycle_node.is_none_or(|best| key < best) {
                        next_cycle_node = Some(key);
                    }
                }
            }
            if let Some(key) = next_cycle_node {
                ready.insert(key);
            }
        }

        if let Some(key) = ready.pop_first() {
            let u = key.2;
            if visited[u] {
                continue;
            }
            visited[u] = true;
            ordered.push(u);
            for &v in &children[u] {
                if !visited[v] {
                    in_degree[v] = in_degree[v].saturating_sub(1);
                    if in_degree[v] == 0 {
                        ready.insert((ids[v], occ[v], v));
                    }
                }
            }
        }
    }

    (ordered, cyclic)
}

struct Scanner {
    max_entities_scanned: Option<usize>,
    entities_scanned: usize,
    truncated: bool,
    candidates: Vec<MissingNameCandidate>,
    by_kind: BTreeMap<NameTargetKind, usize>,
}

impl Scanner {
    fn new(max_entities_scanned: Option<usize>) -> Self {
        Self {
            max_entities_scanned,
            entities_scanned: 0,
            truncated: false,
            candidates: Vec::new(),
            by_kind: BTreeMap::new(),
        }
    }

    fn try_visit(&mut self) -> bool {
        if let Some(limit) = self.max_entities_scanned {
            if self.entities_scanned >= limit {
                self.truncated = true;
                return false;
            }
        }
        self.entities_scanned += 1;
        true
    }

    fn record(&mut self, candidate: MissingNameCandidate) {
        *self.by_kind.entry(candidate.kind).or_insert(0) += 1;
        self.candidates.push(candidate);
    }
}

/// Scans `project` read-only for blank names across all Q9 entity types.
///
/// Guarantees:
/// - Never mutates `project`.
/// - Applies exact Q5 blank detection (`knx_core::names::is_blank_name`).
/// - Emits candidates in deterministic parent-before-child order (Q16).
/// - Surfaces duplicate/zero identities and missing/ambiguous/cyclic parents explicitly (Q13, Q16).
/// - Suppresses full-project totals whenever a bounded scan stops early (Q17).
pub fn detect_missing_names(
    project: &Project,
    request: MissingNameScanRequest,
) -> MissingNameScanResult {
    let idx = ProjectIndex::build(project);
    let mut scanner = Scanner::new(request.max_entities_scanned);

    // Track per-ID occurrence counters across the whole project for duplicate-ID surfacing.
    let mut inst_occ: BTreeMap<u8, usize> = BTreeMap::new();
    let mut part_occ: BTreeMap<u32, usize> = BTreeMap::new();
    let mut area_occ: BTreeMap<u32, usize> = BTreeMap::new();
    let mut line_occ: BTreeMap<u32, usize> = BTreeMap::new();
    let mut range_occ: BTreeMap<u32, usize> = BTreeMap::new();
    let mut ga_occ: BTreeMap<u32, usize> = BTreeMap::new();
    let mut func_occ: BTreeMap<u32, usize> = BTreeMap::new();

    // 1. Project root
    if scanner.try_visit() && is_blank_name(&project.info.name) {
        scanner.record(MissingNameCandidate {
            target: NameTargetId::Project,
            kind: NameTargetKind::Project,
            current_name: project.info.name.clone(),
            occurrence_index: 0,
            identity_status: TargetIdentityStatus::Unique,
            parents: Vec::new(),
            max_name_codepoints: MAX_EDITED_NAME_CODEPOINTS,
            context: CandidateContext {
                source_ets_id: (!project.info.project_id.is_empty())
                    .then(|| project.info.project_id.clone()),
                ..Default::default()
            },
        });
    }

    // 2. Installations (sorted deterministically by (InstallationId, source index))
    let mut inst_order: Vec<(u8, usize)> = project
        .installations
        .iter()
        .enumerate()
        .map(|(pos, inst)| (inst.id.0, pos))
        .collect();
    inst_order.sort_unstable();

    for (inst_id_raw, inst_pos) in inst_order {
        if scanner.truncated {
            break;
        }
        let inst = &project.installations[inst_pos];
        let occ = *inst_occ
            .entry(inst_id_raw)
            .and_modify(|c| *c += 1)
            .or_insert(0);
        let inst_total = idx
            .installation_counts
            .get(&inst_id_raw)
            .copied()
            .unwrap_or(1);

        // 2a. Installation itself
        if !scanner.try_visit() {
            break;
        }
        if is_blank_name(&inst.name) {
            scanner.record(MissingNameCandidate {
                target: NameTargetId::Installation(inst_id_raw),
                kind: NameTargetKind::Installation,
                current_name: inst.name.clone(),
                occurrence_index: occ,
                identity_status: identity_status(inst_total, false),
                parents: vec![ParentReference {
                    relation: ParentRelation::ProjectRoot,
                    state: ParentRefState::Resolved {
                        target: NameTargetId::Project,
                        parent_is_blank: idx.project_is_blank,
                    },
                }],
                max_name_codepoints: MAX_EDITED_NAME_CODEPOINTS,
                context: CandidateContext {
                    installation_id: Some(inst_id_raw),
                    ..Default::default()
                },
            });
        }

        // 2b. BuildingParts in parent-before-child order
        let part_ids: Vec<u32> = inst.buildings.iter().map(|p| p.id.0).collect();
        let part_parents: Vec<Option<u32>> = inst
            .buildings
            .iter()
            .map(|p| p.parent.map(|pid| pid.0))
            .collect();
        let mut local_part_occ = Vec::with_capacity(inst.buildings.len());
        let mut local_part_counts: BTreeMap<u32, usize> = BTreeMap::new();
        for &id in &part_ids {
            let entry = part_occ.entry(id).or_insert(0);
            local_part_occ.push(*entry);
            *entry += 1;
            *local_part_counts.entry(id).or_insert(0) += 1;
        }
        let (part_order, cyclic_parts) =
            topo_order_by_parent(&part_ids, &local_part_occ, &part_parents);

        for p_idx in part_order {
            if !scanner.try_visit() {
                break;
            }
            let part = &inst.buildings[p_idx];
            if !is_blank_name(&part.name) {
                continue;
            }
            let total_occ = idx
                .building_part_counts
                .get(&part.id.0)
                .copied()
                .unwrap_or(1);
            let mut parents = vec![idx.installation_parent_ref(inst_id_raw)];
            if let Some(pid) = part.parent {
                let state = if cyclic_parts.contains(&p_idx) {
                    ParentRefState::Cyclic {
                        target: NameTargetId::BuildingPart(pid.0),
                    }
                } else {
                    let global_p = idx.building_part_counts.get(&pid.0).copied().unwrap_or(0);
                    let local_p = local_part_counts.get(&pid.0).copied().unwrap_or(0);
                    if global_p > 1 {
                        ParentRefState::Ambiguous {
                            candidates: vec![NameTargetId::BuildingPart(pid.0)],
                        }
                    } else if local_p == 0 {
                        ParentRefState::Missing {
                            expected: Some(NameTargetId::BuildingPart(pid.0)),
                        }
                    } else {
                        ParentRefState::Resolved {
                            target: NameTargetId::BuildingPart(pid.0),
                            parent_is_blank: idx
                                .building_part_blank
                                .get(&pid.0)
                                .copied()
                                .unwrap_or(false),
                        }
                    }
                };
                parents.push(ParentReference {
                    relation: ParentRelation::BuildingParent,
                    state,
                });
            }

            scanner.record(MissingNameCandidate {
                target: NameTargetId::BuildingPart(part.id.0),
                kind: NameTargetKind::BuildingPart,
                current_name: part.name.clone(),
                occurrence_index: local_part_occ[p_idx],
                identity_status: identity_status(total_occ, false),
                parents,
                max_name_codepoints: MAX_EDITED_NAME_CODEPOINTS,
                context: CandidateContext {
                    installation_id: Some(inst_id_raw),
                    source_ets_id: (!part.source.ets_id.is_empty())
                        .then(|| part.source.ets_id.clone()),
                    building_part_kind: Some(BuildingPartKind::from(part.kind)),
                    building_part_number: part.number.clone(),
                    ..Default::default()
                },
            });
        }
        if scanner.truncated {
            break;
        }

        // 2c. Topology Areas (before Lines)
        let mut area_order: Vec<(u32, usize)> = inst
            .topology
            .areas
            .iter()
            .enumerate()
            .map(|(pos, a)| (a.id.0, pos))
            .collect();
        area_order.sort_unstable();

        for (area_id_raw, a_pos) in area_order {
            if !scanner.try_visit() {
                break;
            }
            let area = &inst.topology.areas[a_pos];
            let occ = *area_occ
                .entry(area_id_raw)
                .and_modify(|c| *c += 1)
                .or_insert(0);
            if !is_blank_name(&area.name) {
                continue;
            }
            let total_occ = idx.area_counts.get(&area_id_raw).copied().unwrap_or(1);
            scanner.record(MissingNameCandidate {
                target: NameTargetId::Area(area_id_raw),
                kind: NameTargetKind::Area,
                current_name: area.name.clone(),
                occurrence_index: occ,
                identity_status: identity_status(total_occ, false),
                parents: vec![idx.installation_parent_ref(inst_id_raw)],
                max_name_codepoints: MAX_EDITED_NAME_CODEPOINTS,
                context: CandidateContext {
                    installation_id: Some(inst_id_raw),
                    source_ets_id: (!area.source.ets_id.is_empty())
                        .then(|| area.source.ets_id.clone()),
                    topology_area_address: Some(area.address),
                    ..Default::default()
                },
            });
        }
        if scanner.truncated {
            break;
        }

        // 2d. Topology Lines (after Areas)
        let mut line_order: Vec<(u32, usize)> = inst
            .topology
            .lines
            .iter()
            .enumerate()
            .map(|(pos, l)| (l.id.0, pos))
            .collect();
        line_order.sort_unstable();

        for (line_id_raw, l_pos) in line_order {
            if !scanner.try_visit() {
                break;
            }
            let line = &inst.topology.lines[l_pos];
            let occ = *line_occ
                .entry(line_id_raw)
                .and_modify(|c| *c += 1)
                .or_insert(0);
            if !is_blank_name(&line.name) {
                continue;
            }
            let total_occ = idx.line_counts.get(&line_id_raw).copied().unwrap_or(1);
            let mut owning_areas: Vec<(u32, u8)> = inst
                .topology
                .areas
                .iter()
                .filter(|a| a.lines.contains(&line.id))
                .map(|a| (a.id.0, a.address))
                .collect();
            owning_areas.sort_unstable();

            let (area_state, area_address) = match owning_areas.as_slice() {
                [] => (ParentRefState::Missing { expected: None }, None),
                [(area_id, addr)] => {
                    if idx.area_counts.get(area_id).copied().unwrap_or(0) > 1 {
                        (
                            ParentRefState::Ambiguous {
                                candidates: vec![NameTargetId::Area(*area_id)],
                            },
                            None,
                        )
                    } else {
                        (
                            ParentRefState::Resolved {
                                target: NameTargetId::Area(*area_id),
                                parent_is_blank: idx
                                    .area_blank
                                    .get(area_id)
                                    .copied()
                                    .unwrap_or(false),
                            },
                            Some(*addr),
                        )
                    }
                }
                many => (
                    ParentRefState::Ambiguous {
                        candidates: many
                            .iter()
                            .map(|(area_id, _)| NameTargetId::Area(*area_id))
                            .collect(),
                    },
                    None,
                ),
            };

            scanner.record(MissingNameCandidate {
                target: NameTargetId::Line(line_id_raw),
                kind: NameTargetKind::Line,
                current_name: line.name.clone(),
                occurrence_index: occ,
                identity_status: identity_status(total_occ, false),
                parents: vec![
                    idx.installation_parent_ref(inst_id_raw),
                    ParentReference {
                        relation: ParentRelation::TopologyArea,
                        state: area_state,
                    },
                ],
                max_name_codepoints: MAX_EDITED_NAME_CODEPOINTS,
                context: CandidateContext {
                    installation_id: Some(inst_id_raw),
                    source_ets_id: (!line.source.ets_id.is_empty())
                        .then(|| line.source.ets_id.clone()),
                    topology_area_address: area_address,
                    topology_line_address: Some(line.address),
                    ..Default::default()
                },
            });
        }
        if scanner.truncated {
            break;
        }

        // 2e. GroupRanges in parent-before-child order (all nesting levels)
        let range_ids: Vec<u32> = inst.group_ranges.iter().map(|r| r.id.0).collect();
        let range_parents: Vec<Option<u32>> = inst
            .group_ranges
            .iter()
            .map(|r| r.parent.map(|pid| pid.0))
            .collect();
        let mut local_range_occ = Vec::with_capacity(inst.group_ranges.len());
        let mut local_range_counts: BTreeMap<u32, usize> = BTreeMap::new();
        for &id in &range_ids {
            let entry = range_occ.entry(id).or_insert(0);
            local_range_occ.push(*entry);
            *entry += 1;
            *local_range_counts.entry(id).or_insert(0) += 1;
        }
        let (range_order, cyclic_ranges) =
            topo_order_by_parent(&range_ids, &local_range_occ, &range_parents);

        for r_idx in range_order {
            if !scanner.try_visit() {
                break;
            }
            let range = &inst.group_ranges[r_idx];
            if !is_blank_name(&range.name) {
                continue;
            }
            let total_occ = idx
                .group_range_counts
                .get(&range.id.0)
                .copied()
                .unwrap_or(1);
            let mut parents = vec![idx.installation_parent_ref(inst_id_raw)];
            if let Some(pid) = range.parent {
                let state = if cyclic_ranges.contains(&r_idx) {
                    ParentRefState::Cyclic {
                        target: NameTargetId::GroupRange(pid.0),
                    }
                } else {
                    let global_r = idx.group_range_counts.get(&pid.0).copied().unwrap_or(0);
                    let local_r = local_range_counts.get(&pid.0).copied().unwrap_or(0);
                    if global_r > 1 {
                        ParentRefState::Ambiguous {
                            candidates: vec![NameTargetId::GroupRange(pid.0)],
                        }
                    } else if local_r == 0 {
                        ParentRefState::Missing {
                            expected: Some(NameTargetId::GroupRange(pid.0)),
                        }
                    } else {
                        ParentRefState::Resolved {
                            target: NameTargetId::GroupRange(pid.0),
                            parent_is_blank: idx
                                .group_range_blank
                                .get(&pid.0)
                                .copied()
                                .unwrap_or(false),
                        }
                    }
                };
                parents.push(ParentReference {
                    relation: ParentRelation::GroupRangeParent,
                    state,
                });
            }

            scanner.record(MissingNameCandidate {
                target: NameTargetId::GroupRange(range.id.0),
                kind: NameTargetKind::GroupRange,
                current_name: range.name.clone(),
                occurrence_index: local_range_occ[r_idx],
                identity_status: identity_status(total_occ, false),
                parents,
                max_name_codepoints: MAX_EDITED_NAME_CODEPOINTS,
                context: CandidateContext {
                    installation_id: Some(inst_id_raw),
                    source_ets_id: (!range.source.ets_id.is_empty())
                        .then(|| range.source.ets_id.clone()),
                    group_range_span: Some((range.start.raw(), range.end.raw())),
                    ..Default::default()
                },
            });
        }
        if scanner.truncated {
            break;
        }

        // 2f. GroupAddresses (after GroupRanges)
        let mut ga_order: Vec<(u32, usize)> = inst
            .group_addresses
            .iter()
            .enumerate()
            .map(|(pos, ga)| (ga.id.0, pos))
            .collect();
        ga_order.sort_unstable();

        for (ga_id_raw, ga_pos) in ga_order {
            if !scanner.try_visit() {
                break;
            }
            let ga = &inst.group_addresses[ga_pos];
            let occ = *ga_occ.entry(ga_id_raw).and_modify(|c| *c += 1).or_insert(0);
            if !is_blank_name(&ga.name) {
                continue;
            }
            let total_occ = idx
                .group_address_counts
                .get(&ga_id_raw)
                .copied()
                .unwrap_or(1);
            let mut parents = vec![idx.installation_parent_ref(inst_id_raw)];
            if let Some(rid) = ga.range {
                let global_r = idx.group_range_counts.get(&rid.0).copied().unwrap_or(0);
                let local_r = local_range_counts.get(&rid.0).copied().unwrap_or(0);
                let state = if global_r > 1 {
                    ParentRefState::Ambiguous {
                        candidates: vec![NameTargetId::GroupRange(rid.0)],
                    }
                } else if local_r == 0 {
                    ParentRefState::Missing {
                        expected: Some(NameTargetId::GroupRange(rid.0)),
                    }
                } else {
                    ParentRefState::Resolved {
                        target: NameTargetId::GroupRange(rid.0),
                        parent_is_blank: idx
                            .group_range_blank
                            .get(&rid.0)
                            .copied()
                            .unwrap_or(false),
                    }
                };
                parents.push(ParentReference {
                    relation: ParentRelation::GroupAddressRange,
                    state,
                });
            }

            scanner.record(MissingNameCandidate {
                target: NameTargetId::GroupAddress(ga_id_raw),
                kind: NameTargetKind::GroupAddress,
                current_name: ga.name.clone(),
                occurrence_index: occ,
                identity_status: identity_status(total_occ, false),
                parents,
                max_name_codepoints: MAX_EDITED_NAME_CODEPOINTS,
                context: CandidateContext {
                    installation_id: Some(inst_id_raw),
                    source_ets_id: (!ga.source.ets_id.is_empty()).then(|| ga.source.ets_id.clone()),
                    group_address: Some(ga.address.raw()),
                    ..Default::default()
                },
            });
        }
    }

    // 3. Devices (after all Installations, BuildingParts, Areas, and Lines)
    if !scanner.truncated {
        for device in project.devices.iter() {
            if !scanner.try_visit() {
                break;
            }
            if !is_blank_name(&device.name) {
                continue;
            }
            let line_refs = idx
                .device_lines
                .get(&device.id.0)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let line_state = match line_refs {
                [] => ParentRefState::Missing { expected: None },
                [line_id] => {
                    if idx.line_counts.get(line_id).copied().unwrap_or(0) > 1 {
                        ParentRefState::Ambiguous {
                            candidates: vec![NameTargetId::Line(*line_id)],
                        }
                    } else {
                        ParentRefState::Resolved {
                            target: NameTargetId::Line(*line_id),
                            parent_is_blank: idx.line_blank.get(line_id).copied().unwrap_or(false),
                        }
                    }
                }
                many => ParentRefState::Ambiguous {
                    candidates: many.iter().map(|&id| NameTargetId::Line(id)).collect(),
                },
            };

            let building_refs = idx
                .device_buildings
                .get(&device.id.0)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let building_state = match building_refs {
                [] => ParentRefState::Missing { expected: None },
                [part_id] => {
                    if idx.building_part_counts.get(part_id).copied().unwrap_or(0) > 1 {
                        ParentRefState::Ambiguous {
                            candidates: vec![NameTargetId::BuildingPart(*part_id)],
                        }
                    } else {
                        ParentRefState::Resolved {
                            target: NameTargetId::BuildingPart(*part_id),
                            parent_is_blank: idx
                                .building_part_blank
                                .get(part_id)
                                .copied()
                                .unwrap_or(false),
                        }
                    }
                }
                many => ParentRefState::Ambiguous {
                    candidates: many
                        .iter()
                        .map(|&id| NameTargetId::BuildingPart(id))
                        .collect(),
                },
            };

            scanner.record(MissingNameCandidate {
                target: NameTargetId::Device(device.id.0),
                kind: NameTargetKind::Device,
                current_name: device.name.clone(),
                occurrence_index: 0,
                identity_status: TargetIdentityStatus::Unique,
                parents: vec![
                    ParentReference {
                        relation: ParentRelation::TopologyLine,
                        state: line_state,
                    },
                    ParentReference {
                        relation: ParentRelation::DeviceLocation,
                        state: building_state,
                    },
                ],
                max_name_codepoints: MAX_EDITED_NAME_CODEPOINTS,
                context: CandidateContext {
                    source_ets_id: (!device.source.ets_id.is_empty())
                        .then(|| device.source.ets_id.clone()),
                    individual_address: device.address.map(|a| a.to_string()),
                    ..Default::default()
                },
            });
        }
    }

    // 4. Project Functions (after all BuildingParts and GroupAddresses)
    if !scanner.truncated {
        let mut func_order: Vec<(u32, usize)> = project
            .functions
            .iter()
            .enumerate()
            .map(|(pos, f)| (f.id.0, pos))
            .collect();
        func_order.sort_unstable();

        for (func_id_raw, f_pos) in func_order {
            if !scanner.try_visit() {
                break;
            }
            let func = &project.functions[f_pos];
            let occ = *func_occ
                .entry(func_id_raw)
                .and_modify(|c| *c += 1)
                .or_insert(0);
            if !is_blank_name(&func.details.name) {
                continue;
            }
            let total_occ = idx.function_counts.get(&func_id_raw).copied().unwrap_or(1);
            let loc_id = func.location.0;
            let loc_count = idx.building_part_counts.get(&loc_id).copied().unwrap_or(0);
            let loc_state = if loc_count == 0 {
                ParentRefState::Missing {
                    expected: Some(NameTargetId::BuildingPart(loc_id)),
                }
            } else if loc_count > 1 {
                ParentRefState::Ambiguous {
                    candidates: vec![NameTargetId::BuildingPart(loc_id)],
                }
            } else {
                ParentRefState::Resolved {
                    target: NameTargetId::BuildingPart(loc_id),
                    parent_is_blank: idx
                        .building_part_blank
                        .get(&loc_id)
                        .copied()
                        .unwrap_or(false),
                }
            };

            scanner.record(MissingNameCandidate {
                target: NameTargetId::Function(func_id_raw),
                kind: NameTargetKind::Function,
                current_name: func.details.name.clone(),
                occurrence_index: occ,
                identity_status: identity_status(total_occ, func_id_raw == 0),
                parents: vec![ParentReference {
                    relation: ParentRelation::FunctionLocation,
                    state: loc_state,
                }],
                max_name_codepoints: MAX_FUNCTION_SHORT_TEXT_CODEPOINTS,
                context: CandidateContext {
                    source_ets_id: (!func.source.ets_id.is_empty())
                        .then(|| func.source.ets_id.clone()),
                    ..Default::default()
                },
            });
        }
    }

    let completeness = if scanner.truncated {
        ScanCompleteness::Incomplete
    } else {
        ScanCompleteness::Complete
    };
    let candidates_found_in_scan = scanner.candidates.len();
    let items: Vec<MissingNameCandidate> = scanner
        .candidates
        .into_iter()
        .skip(request.offset)
        .take(request.limit)
        .collect();
    let has_more_in_scan = request.offset.saturating_add(items.len()) < candidates_found_in_scan;

    let (total_candidates, total_by_kind) = match completeness {
        ScanCompleteness::Complete => (Some(candidates_found_in_scan), Some(scanner.by_kind)),
        ScanCompleteness::Incomplete => (None, None),
    };

    MissingNameScanResult {
        completeness,
        counts: MissingNameScanCounts {
            entities_scanned: scanner.entities_scanned,
            candidates_found_in_scan,
            total_candidates,
            total_by_kind,
        },
        offset: request.offset,
        limit: request.limit,
        has_more_in_scan,
        items,
    }
}
