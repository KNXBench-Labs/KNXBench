//! Builds read-only missing-name proposals from associated project and resolved product evidence.

use std::collections::BTreeSet;

use knx_core::names::{is_blank_name, validate_edited_name};
use knx_core::{BuildingPartId, DeviceId, FunctionDetails, GroupAddress, Project};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingNameProposal {
    pub candidate: MissingNameCandidate,
    pub proposed_name: Option<String>,
    pub provenance: NameProvenance,
    /// Same-kind collision with an existing name or another proposal on this page.
    /// A later selection/preview layer must also check collisions across selected pages.
    pub duplicate_label: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
        .map(|candidate| {
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
        })
        .collect();
    let existing = existing_names(project);
    let labels: Vec<_> = items
        .iter()
        .map(|p| (p.candidate.kind, p.proposed_name.clone()))
        .collect();
    for item in &mut items {
        if let Some(name) = &item.proposed_name {
            item.duplicate_label = existing
                .iter()
                .any(|(kind, value)| *kind == item.candidate.kind && value == name)
                || labels
                    .iter()
                    .filter(|(kind, value)| {
                        *kind == item.candidate.kind && value.as_ref() == Some(name)
                    })
                    .count()
                    > 1;
        }
    }
    MissingNameProposals { discovery, items }
}

struct Recipe<'a> {
    project: &'a Project,
    sources: Vec<EvidenceContribution>,
    gaps: Vec<EvidenceGap>,
    blocked: bool,
}

impl Recipe<'_> {
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
        let mut next = Some(id);
        let mut seen = BTreeSet::new();
        let mut names = Vec::new();
        let mut owner = None;
        let mut blank = false;
        while let Some(id) = next {
            if !seen.insert(id) {
                self.refuse(
                    EvidenceSourceKind::BuildingLocationPath,
                    "cyclic location path",
                );
                return None;
            }
            let parts: Vec<_> = self
                .project
                .installations
                .iter()
                .flat_map(|i| {
                    i.buildings
                        .iter()
                        .filter(move |b| b.id == id)
                        .map(move |b| (i, b))
                })
                .collect();
            let [(installation, part)] = parts.as_slice() else {
                self.refuse(
                    EvidenceSourceKind::BuildingLocationPath,
                    "missing or ambiguous location path",
                );
                return None;
            };
            if self
                .project
                .installations
                .iter()
                .filter(|i| i.id == installation.id)
                .count()
                != 1
                || owner.is_some_and(|previous| previous != installation.id)
            {
                self.refuse(
                    EvidenceSourceKind::BuildingLocationPath,
                    "ambiguous location ownership",
                );
                return None;
            }
            owner = Some(installation.id);
            if is_blank_name(&part.name) {
                blank = true;
                self.missing(
                    EvidenceSourceKind::ParentName,
                    "location parent has no existing name",
                );
            }
            names.push((NameTargetId::BuildingPart(id.0), part.name.clone()));
            next = part.parent;
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

fn existing_names(project: &Project) -> Vec<(NameTargetKind, String)> {
    use NameTargetKind as K;
    let mut names = vec![(K::Project, project.info.name.clone())];
    for i in &project.installations {
        names.push((K::Installation, i.name.clone()));
        names.extend(
            i.buildings
                .iter()
                .map(|v| (K::BuildingPart, v.name.clone())),
        );
        names.extend(i.topology.areas.iter().map(|v| (K::Area, v.name.clone())));
        names.extend(i.topology.lines.iter().map(|v| (K::Line, v.name.clone())));
        names.extend(
            i.group_ranges
                .iter()
                .map(|v| (K::GroupRange, v.name.clone())),
        );
        names.extend(
            i.group_addresses
                .iter()
                .map(|v| (K::GroupAddress, v.name.clone())),
        );
    }
    names.extend(project.devices.iter().map(|v| (K::Device, v.name.clone())));
    names.extend(
        project
            .functions
            .iter()
            .map(|v| (K::Function, v.details.name.clone())),
    );
    names
}
