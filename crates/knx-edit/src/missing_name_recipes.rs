//! Builds read-only missing-name proposals from associated project and resolved product evidence.

use std::collections::{BTreeSet, HashMap, HashSet};

use knx_core::names::{is_blank_name, validate_edited_name};
use knx_core::{BuildingPartId, DeviceId, FunctionDetails, GroupAddress, Installation, Project};
use serde::{Deserialize, Serialize};

use crate::missing_names::*;

/// Association must be established by the adapter reading the original descriptive text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextAssociation {
    Confirmed,
    Unclear,
}

/// External fields absent from Core. References identify evidence; they are never captions.
///
/// Product text corresponds to `knx_productdb::query::DeviceProductRow::product_text`.
/// The caller must resolve it using the exact device product/program references, without
/// substituting catalog, hardware, application or manufacturer names for product text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissingNameEvidence {
    ProjectDescription {
        text: String,
        association: TextAssociation,
    },
    InstallationDescription {
        installation: u8,
        text: String,
        association: TextAssociation,
    },
    ResolvedProduct {
        device: DeviceId,
        product_ref: String,
        program_ref: String,
        product_text: String,
        association: TextAssociation,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingNameProposal {
    pub candidate: MissingNameCandidate,
    pub proposed_name: Option<String>,
    pub provenance: NameProvenance,
    /// Same-kind collision with an existing name or another proposal on this page.
    /// A later selection/preview layer must also check collisions across selected pages.
    pub duplicate_label: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingNameProposals {
    /// Unmodified discovery counts, completeness and pagination contract.
    pub discovery: MissingNameScanResult,
    pub items: Vec<MissingNameProposal>,
}

/// Proposes names for the discovery page only. This is not a selection draft or apply plan.
/// Neither project data nor supplied evidence is modified; proposed parents are never used.
pub fn propose_missing_names(
    project: &Project,
    evidence: &[MissingNameEvidence],
    request: MissingNameScanRequest,
) -> MissingNameProposals {
    let discovery = detect_missing_names(project, request);
    let mut items: Vec<_> = discovery
        .items
        .iter()
        .map(|candidate| evaluate_candidate(project, evidence, candidate))
        .collect();
    mark_duplicate_labels(project, &mut items);
    MissingNameProposals { discovery, items }
}

/// Proposes names only for the requested blank targets in canonical discovery order.
pub(crate) fn propose_missing_names_for_targets(
    project: &Project,
    evidence: &[MissingNameEvidence],
    targets: &BTreeSet<NameTargetId>,
) -> Vec<MissingNameProposal> {
    if targets.is_empty() {
        return Vec::new();
    }
    let discovery = detect_missing_names(project, MissingNameScanRequest::unbounded(0, usize::MAX));
    let mut items: Vec<_> = discovery
        .items
        .iter()
        .filter(|candidate| targets.contains(&candidate.target))
        .map(|candidate| evaluate_candidate(project, evidence, candidate))
        .collect();
    mark_duplicate_labels(project, &mut items);
    items
}

fn evaluate_candidate(
    project: &Project,
    evidence: &[MissingNameEvidence],
    candidate: &MissingNameCandidate,
) -> MissingNameProposal {
    let mut recipe = Recipe {
        project,
        sources: vec![],
        gaps: vec![],
        blocked: false,
    };
    if !candidate.identity_status.is_unique() {
        recipe.refuse(
            EvidenceSourceKind::ParentName,
            "target identity is not unique or valid",
        );
    }
    for parent in &candidate.parents {
        match &parent.state {
            ParentRefState::Ambiguous { candidates } => recipe.ambiguous(
                EvidenceSourceKind::ParentName,
                "ambiguous parent",
                candidates.iter().map(|id| format!("{id:?}")).collect(),
            ),
            ParentRefState::Cyclic { .. } => {
                recipe.refuse(EvidenceSourceKind::ParentName, "cyclic parent")
            }
            ParentRefState::Missing { expected: Some(_) } => {
                recipe.refuse(EvidenceSourceKind::ParentName, "dangling parent")
            }
            _ => {}
        }
    }
    if !recipe.blocked {
        recipe.certify_context(candidate.target);
    }
    let proposed = if recipe.blocked {
        None
    } else {
        recipe.name(candidate, evidence)
    };
    let proposed_name = proposed
        .filter(|name| {
            let valid = validate_edited_name(name).is_ok()
                && name.chars().count() <= candidate.max_name_codepoints
                && (!matches!(candidate.target, NameTargetId::Function(_))
                    || FunctionDetails::validate_name(name).is_ok());
            if !valid {
                recipe.refuse(
                    EvidenceSourceKind::ParentName,
                    "generated name fails edited-name admission",
                );
            }
            valid
        })
        .filter(|_| !recipe.blocked);
    let provenance = NameProvenance {
        origin: if proposed_name.is_some() {
            NameOrigin::Automatic
        } else {
            NameOrigin::ManualRequired
        },
        sources: recipe.sources,
        gaps: recipe.gaps,
        derived_from_automatic: false,
    };
    MissingNameProposal {
        candidate: candidate.clone(),
        proposed_name,
        provenance,
        duplicate_label: false,
    }
}

fn mark_duplicate_labels(project: &Project, items: &mut [MissingNameProposal]) {
    if items.is_empty() {
        return;
    }
    let existing = existing_names(project);
    let mut proposed_counts: HashMap<(NameTargetKind, &str), usize> =
        HashMap::with_capacity(items.len());
    for item in items.iter() {
        if let Some(name) = item.proposed_name.as_deref() {
            *proposed_counts
                .entry((item.candidate.kind, name))
                .or_insert(0) += 1;
        }
    }
    let duplicate_flags: Vec<bool> = items
        .iter()
        .map(|item| {
            item.proposed_name.as_deref().is_some_and(|name| {
                existing.contains(&(item.candidate.kind, name))
                    || proposed_counts
                        .get(&(item.candidate.kind, name))
                        .copied()
                        .unwrap_or(0)
                        > 1
            })
        })
        .collect();
    for (item, is_dup) in items.iter_mut().zip(duplicate_flags) {
        item.duplicate_label = is_dup;
    }
}

struct Recipe<'a> {
    project: &'a Project,
    sources: Vec<EvidenceContribution>,
    gaps: Vec<EvidenceGap>,
    blocked: bool,
}

impl Recipe<'_> {
    fn certify_context(&mut self, target: NameTargetId) {
        if let Err((kind, detail)) = structural_context(self.project, target) {
            self.refuse(kind, detail);
        }
    }
    fn missing(&mut self, kind: EvidenceSourceKind, detail: &str) {
        self.gaps.push(EvidenceGap::Missing {
            kind,
            detail: detail.into(),
        });
    }
    fn refuse(&mut self, kind: EvidenceSourceKind, detail: &str) {
        self.blocked = true;
        self.gaps.push(EvidenceGap::Insufficient {
            kind,
            detail: detail.into(),
        });
    }
    fn ambiguous(&mut self, kind: EvidenceSourceKind, detail: &str, candidates: Vec<String>) {
        self.blocked = true;
        self.gaps.push(EvidenceGap::Ambiguous {
            kind,
            detail: detail.into(),
            candidates,
        });
    }
    fn source(&mut self, kind: EvidenceSourceKind, target: NameTargetId, value: String) -> String {
        self.sources.push(EvidenceContribution {
            kind,
            source_target: Some(target),
            value: value.clone(),
        });
        value
    }
    fn text(
        &mut self,
        kind: EvidenceSourceKind,
        target: NameTargetId,
        texts: Vec<(String, bool)>,
    ) -> Option<String> {
        if texts.iter().any(|(_, confirmed)| !confirmed) {
            self.ambiguous(
                kind,
                "unclear evidence association",
                texts.iter().map(|(s, _)| s.clone()).collect(),
            );
            return None;
        }
        let values: BTreeSet<_> = texts
            .into_iter()
            .map(|(s, _)| s)
            .filter(|s| !is_blank_name(s))
            .collect();
        if values.len() > 1 {
            self.ambiguous(kind, "contradictory evidence", values.into_iter().collect());
            None
        } else if let Some(value) = values.into_iter().next() {
            Some(self.source(kind, target, value))
        } else {
            self.missing(kind, "no associated nonblank text");
            None
        }
    }
    /// Resolve the entire actual chain, requiring unique identity and installation ownership.
    fn location(&mut self, id: BuildingPartId) -> Option<String> {
        let chain = match ancestry(self.project, NameTargetId::BuildingPart(id.0)) {
            Ok((_, chain)) => chain,
            Err(detail) => {
                self.refuse(EvidenceSourceKind::BuildingLocationPath, detail);
                return None;
            }
        };
        let mut names = Vec::new();
        let mut blank = false;
        for target in chain {
            let NameTargetId::BuildingPart(id) = target else {
                unreachable!()
            };
            let (_, part) = unique_owned(self.project, |i| {
                i.buildings.iter().filter(move |b| b.id.0 == id)
            })
            .ok()?;
            if is_blank_name(&part.name) {
                blank = true;
                self.missing(
                    EvidenceSourceKind::ParentName,
                    "location parent has no existing name",
                );
            }
            names.push((target, part.name.clone()));
        }
        if blank {
            return None;
        }
        names.reverse();
        let path = names
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>()
            .join(" / ");
        for (target, name) in names {
            self.source(EvidenceSourceKind::ParentName, target, name);
        }
        Some(self.source(
            EvidenceSourceKind::BuildingLocationPath,
            NameTargetId::BuildingPart(id.0),
            path,
        ))
    }
    fn name(
        &mut self,
        c: &MissingNameCandidate,
        evidence: &[MissingNameEvidence],
    ) -> Option<String> {
        use EvidenceSourceKind as E;
        use NameTargetId as T;
        match c.target {
            T::Project | T::Installation(_) => {
                let texts = evidence
                    .iter()
                    .filter_map(|e| match (c.target, e) {
                        (
                            T::Project,
                            MissingNameEvidence::ProjectDescription { text, association },
                        ) => Some((text.clone(), *association == TextAssociation::Confirmed)),
                        (
                            T::Installation(id),
                            MissingNameEvidence::InstallationDescription {
                                installation,
                                text,
                                association,
                            },
                        ) if id == *installation => {
                            Some((text.clone(), *association == TextAssociation::Confirmed))
                        }
                        _ => None,
                    })
                    .collect();
                self.text(
                    if c.target == T::Project {
                        E::ProjectDescriptiveText
                    } else {
                        E::InstallationDescriptiveText
                    },
                    c.target,
                    texts,
                )
            }
            T::BuildingPart(_) => {
                let kind = c.context.building_part_kind?;
                let number = self.text(
                    E::BuildingPartNumber,
                    c.target,
                    c.context
                        .building_part_number
                        .iter()
                        .map(|s| (s.clone(), true))
                        .collect(),
                )?;
                let kind = self.source(
                    E::BuildingPartType,
                    c.target,
                    building_kind_label(kind).into(),
                );
                Some(format!("{number} {kind}"))
            }
            T::Area(_) => {
                let address = c.context.topology_area_address?;
                if address > 15 {
                    self.refuse(E::TopologyAddress, "area address exceeds 15");
                }
                Some(self.source(E::TopologyAddress, c.target, format!("Area {address}")))
            }
            T::Line(_) => {
                let Some(area) = c.context.topology_area_address else {
                    self.refuse(E::TopologyAddress, "line has no unique owning area");
                    return None;
                };
                let line = c.context.topology_line_address?;
                if area > 15 || line > 15 {
                    self.refuse(E::TopologyAddress, "topology address exceeds 15");
                }
                Some(self.source(E::TopologyAddress, c.target, format!("Line {area}.{line}")))
            }
            T::GroupRange(_) => {
                let (start, end) = c.context.group_range_span?;
                if start > end {
                    self.refuse(E::GroupRangeSpan, "reversed range span");
                }
                let style = self.project.info.group_address_style;
                Some(self.source(
                    E::GroupRangeSpan,
                    c.target,
                    format!(
                        "{}–{}",
                        GroupAddress::from_raw(start).format(style),
                        GroupAddress::from_raw(end).format(style)
                    ),
                ))
            }
            T::Device(id) => {
                let device = self.project.devices.get(DeviceId(id))?;
                let texts = evidence
                    .iter()
                    .filter_map(|e| match e {
                        MissingNameEvidence::ResolvedProduct {
                            device: target,
                            product_ref,
                            program_ref,
                            product_text,
                            association,
                        } if target.0 == id => Some((
                            product_text.clone(),
                            *association == TextAssociation::Confirmed
                                && !product_ref.is_empty()
                                && *product_ref == device.product_ref
                                && *program_ref == device.program_ref,
                        )),
                        _ => None,
                    })
                    .collect();
                let product = self.text(E::ProductDesignation, c.target, texts);
                let placements: Vec<_> = self
                    .project
                    .installations
                    .iter()
                    .flat_map(|i| &i.buildings)
                    .filter(|b| b.devices.contains(&device.id))
                    .map(|b| b.id)
                    .collect();
                let location_sources_start = self.sources.len();
                let location = match placements.as_slice() {
                    [] => {
                        self.missing(E::BuildingLocationPath, "device has no location");
                        None
                    }
                    [id] => self.location(*id),
                    _ => {
                        self.ambiguous(
                            E::BuildingLocationPath,
                            "multiple device locations",
                            placements.iter().map(|id| id.0.to_string()).collect(),
                        );
                        None
                    }
                };
                if let Some(product) = product {
                    Some(
                        location
                            .map_or_else(|| product.clone(), |path| format!("{path} – {product}")),
                    )
                } else {
                    self.sources.truncate(location_sources_start);
                    self.text(
                        E::DeviceDescription,
                        c.target,
                        device
                            .description
                            .iter()
                            .map(|s| (s.clone(), true))
                            .collect(),
                    )
                }
            }
            T::GroupAddress(id) => self.group_address(id),
            T::Function(id) => {
                let function = self.project.functions.iter().find(|f| f.id.0 == id)?;
                self.text(
                    E::FunctionContext,
                    c.target,
                    [
                        function.details.description.clone(),
                        function.details.comment.clone(),
                    ]
                    .into_iter()
                    .flatten()
                    .map(|s| (s, true))
                    .collect(),
                )
            }
        }
    }
    fn group_address(&mut self, id: u32) -> Option<String> {
        use EvidenceSourceKind as E;
        let target = NameTargetId::GroupAddress(id);
        let (installation, ga) = self
            .project
            .installations
            .iter()
            .flat_map(|i| {
                i.group_addresses
                    .iter()
                    .filter(move |g| g.id.0 == id)
                    .map(move |g| (i, g))
            })
            .next()?;
        let context_sources_start = self.sources.len();
        let mut contexts = Vec::new();
        let mut memberships = 0;
        for function in &self.project.functions {
            for member in function.members.iter().filter(|m| m.group_address.0 == id) {
                memberships += 1;
                let owners: Vec<_> = self
                    .project
                    .installations
                    .iter()
                    .filter(|i| i.buildings.iter().any(|b| b.id == function.location))
                    .collect();
                if function.id.0 == 0
                    || self
                        .project
                        .functions
                        .iter()
                        .filter(|f| f.id == function.id)
                        .count()
                        != 1
                    || owners.len() != 1
                    || owners[0].id != installation.id
                    || member.id.0 == 0
                    || self
                        .project
                        .functions
                        .iter()
                        .flat_map(|f| &f.members)
                        .filter(|m| m.id == member.id)
                        .count()
                        != 1
                {
                    self.refuse(
                        E::FunctionContext,
                        "invalid or ambiguous function membership",
                    );
                }
                let path = self.location(function.location);
                let role = self.text(
                    E::FunctionMemberContext,
                    NameTargetId::Function(function.id.0),
                    [Some(member.name.clone()), member.role.clone()]
                        .into_iter()
                        .flatten()
                        .map(|s| (s, true))
                        .collect(),
                );
                if is_blank_name(&function.details.name) {
                    self.missing(E::FunctionContext, "function has no existing name");
                } else {
                    let name = self.source(
                        E::FunctionContext,
                        NameTargetId::Function(function.id.0),
                        function.details.name.clone(),
                    );
                    if let (Some(path), Some(role)) = (path, role) {
                        contexts.push((format!("{path} – {name} – {role}"), true));
                    }
                }
            }
        }
        if memberships > 1 && contexts.len() != memberships && !self.blocked {
            self.ambiguous(
                E::FunctionContext,
                "incomplete competing function contexts",
                contexts.iter().map(|(name, _)| name.clone()).collect(),
            );
        }
        if !contexts.is_empty() {
            self.text(E::FunctionContext, target, contexts)
        } else {
            self.sources.truncate(context_sources_start);
            self.missing(
                E::FunctionContext,
                "no complete function/role/location context",
            );
            self.text(
                E::GroupAddressDescription,
                target,
                ga.description.iter().map(|s| (s.clone(), true)).collect(),
            )
        }
    }
}

/// Resolve a globally unique entity and its uniquely identified installation.
fn unique_owned<'a, T: 'a, I: Iterator<Item = &'a T>>(
    project: &'a Project,
    entities: impl Fn(&'a Installation) -> I,
) -> Result<(&'a Installation, &'a T), &'static str> {
    let mut matches = project
        .installations
        .iter()
        .flat_map(|i| entities(i).map(move |v| (i, v)));
    let value = matches.next().ok_or("dangling structural dependency")?;
    if matches.next().is_some() {
        return Err("ambiguous structural dependency identity");
    }
    if project
        .installations
        .iter()
        .filter(|i| i.id == value.0.id)
        .count()
        != 1
    {
        return Err("ambiguous installation ownership");
    }
    Ok(value)
}

/// Walk only the relevant parent chain, iteratively and at most once per identity.
fn ancestry(
    project: &Project,
    start: NameTargetId,
) -> Result<(u8, Vec<NameTargetId>), &'static str> {
    use NameTargetId as T;
    let mut next = Some(start);
    let mut seen = BTreeSet::new();
    let mut chain = Vec::new();
    let mut owner = None;
    while let Some(target) = next {
        if !seen.insert(target) {
            return Err("cyclic structural ancestry");
        }
        let (installation, parent) = match target {
            T::BuildingPart(id) => {
                let (i, b) = unique_owned(project, |i| {
                    i.buildings.iter().filter(move |b| b.id.0 == id)
                })?;
                (i.id.0, b.parent.map(|id| T::BuildingPart(id.0)))
            }
            T::GroupRange(id) => {
                let (i, r) = unique_owned(project, |i| {
                    i.group_ranges.iter().filter(move |r| r.id.0 == id)
                })?;
                (i.id.0, r.parent.map(|id| T::GroupRange(id.0)))
            }
            _ => unreachable!("ancestry accepts only buildings and group ranges"),
        };
        if owner.is_some_and(|owner| owner != installation) {
            return Err("structural ancestor belongs to another installation");
        }
        owner = Some(installation);
        chain.push(target);
        next = parent;
    }
    Ok((owner.expect("nonempty ancestry"), chain))
}

fn line_owner(project: &Project, id: u32) -> Result<u8, &'static str> {
    let (installation, line) = unique_owned(project, |i| {
        i.topology.lines.iter().filter(move |l| l.id.0 == id)
    })?;
    let mut areas = project.installations.iter().flat_map(|i| {
        i.topology.areas.iter().flat_map(move |a| {
            a.lines
                .iter()
                .filter(move |l| **l == line.id)
                .map(move |_| (i, a))
        })
    });
    let (area_installation, area) = areas.next().ok_or("line has no unique owning area")?;
    if areas.next().is_some() {
        return Err("ambiguous line ownership by areas");
    }
    let (owner, _) = unique_owned(project, |i| {
        i.topology.areas.iter().filter(|a| a.id == area.id)
    })?;
    if owner.id != installation.id || area_installation.id != installation.id {
        return Err("line and area belong to different installations");
    }
    Ok(installation.id.0)
}

/// Certify dependencies even when named ancestors are absent from the discovery page.
fn structural_context(
    project: &Project,
    target: NameTargetId,
) -> Result<(), (EvidenceSourceKind, &'static str)> {
    use EvidenceSourceKind as E;
    use NameTargetId as T;
    let mut kind = if matches!(target, T::Function(_) | T::Device(_)) {
        E::BuildingLocationPath
    } else if matches!(target, T::Line(_)) {
        E::TopologyAddress
    } else {
        E::ParentName
    };
    let mut check = || -> Result<(), &'static str> {
        match target {
            T::BuildingPart(_) | T::GroupRange(_) => {
                ancestry(project, target)?;
            }
            T::Area(id) => {
                unique_owned(project, |i| {
                    i.topology.areas.iter().filter(move |a| a.id.0 == id)
                })?;
            }
            T::Line(id) => {
                line_owner(project, id)?;
            }
            T::GroupAddress(id) => {
                let (i, ga) = unique_owned(project, |i| {
                    i.group_addresses.iter().filter(move |g| g.id.0 == id)
                })?;
                if let Some(range) = ga.range {
                    let (owner, _) = ancestry(project, T::GroupRange(range.0))?;
                    if owner != i.id.0 {
                        return Err("group address range belongs to another installation");
                    }
                }
            }
            T::Function(id) => {
                let function = project
                    .functions
                    .iter()
                    .find(|f| f.id.0 == id)
                    .ok_or("missing function")?;
                ancestry(project, T::BuildingPart(function.location.0))?;
            }
            T::Device(id) => {
                let mut owner = None;
                for i in &project.installations {
                    for b in i
                        .buildings
                        .iter()
                        .filter(|b| b.devices.iter().any(|d| d.0 == id))
                    {
                        kind = E::BuildingLocationPath;
                        let (location_owner, _) = ancestry(project, T::BuildingPart(b.id.0))?;
                        if owner.is_some_and(|owner| owner != location_owner) {
                            return Err(
                                "device location and line belong to different installations",
                            );
                        }
                        owner = Some(location_owner);
                    }
                    for l in i
                        .topology
                        .lines
                        .iter()
                        .filter(|l| l.devices.iter().any(|d| d.0 == id))
                    {
                        kind = E::ParentName;
                        let line_owner = line_owner(project, l.id.0)?;
                        if owner.is_some_and(|owner| owner != line_owner) {
                            return Err(
                                "device location and line belong to different installations",
                            );
                        }
                        owner = Some(line_owner);
                    }
                }
            }
            T::Project | T::Installation(_) => {}
        }
        Ok(())
    };
    let result = check();
    result.map_err(|detail| (kind, detail))
}

fn building_kind_label(kind: BuildingPartKind) -> &'static str {
    match kind {
        BuildingPartKind::Building => "Building",
        BuildingPartKind::Floor => "Floor",
        BuildingPartKind::Room => "Room",
        BuildingPartKind::Corridor => "Corridor",
        BuildingPartKind::DistributionBoard => "Distribution board",
        BuildingPartKind::BuildingPart => "Building part",
        BuildingPartKind::Stairway => "Stairway",
        BuildingPartKind::RoomPart => "Room part",
        BuildingPartKind::Area => "Area",
        BuildingPartKind::Ground => "Ground",
        BuildingPartKind::Segment => "Segment",
    }
}

fn existing_names<'a>(project: &'a Project) -> HashSet<(NameTargetKind, &'a str)> {
    use NameTargetKind as K;
    let mut names = HashSet::new();
    let mut record = |kind: NameTargetKind, name: &'a str| {
        if !is_blank_name(name) {
            names.insert((kind, name));
        }
    };
    record(K::Project, project.info.name.as_str());
    for i in &project.installations {
        record(K::Installation, i.name.as_str());
        for v in &i.buildings {
            record(K::BuildingPart, v.name.as_str());
        }
        for v in &i.topology.areas {
            record(K::Area, v.name.as_str());
        }
        for v in &i.topology.lines {
            record(K::Line, v.name.as_str());
        }
        for v in &i.group_ranges {
            record(K::GroupRange, v.name.as_str());
        }
        for v in &i.group_addresses {
            record(K::GroupAddress, v.name.as_str());
        }
    }
    for v in project.devices.iter() {
        record(K::Device, v.name.as_str());
    }
    for v in &project.functions {
        record(K::Function, v.details.name.as_str());
    }
    names
}
