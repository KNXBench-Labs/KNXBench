//! Verifies evidence-only Q9 recipes, refusal isolation, provenance and project immutability.

use knx_core::commissioning::{CommissioningState, CompletionStatus};
use knx_core::provenance::Override;
use knx_core::*;
use knx_edit::missing_name_recipes::*;
use knx_edit::missing_names::*;

const KINDS: [(BuildingPartType, &str); 11] = [
    (BuildingPartType::Building, "Building"),
    (BuildingPartType::Floor, "Floor"),
    (BuildingPartType::Room, "Room"),
    (BuildingPartType::Corridor, "Corridor"),
    (BuildingPartType::DistributionBoard, "Distribution board"),
    (BuildingPartType::BuildingPart, "Building part"),
    (BuildingPartType::Stairway, "Stairway"),
    (BuildingPartType::RoomPart, "Room part"),
    (BuildingPartType::Area, "Area"),
    (BuildingPartType::Ground, "Ground"),
    (BuildingPartType::Segment, "Segment"),
];
fn src() -> SourceRef {
    SourceRef {
        path: "synthetic.xml".into(),
        ets_id: "fixture".into(),
    }
}
fn part(id: u32, name: &str, kind: BuildingPartType) -> BuildingPart {
    BuildingPart {
        id: BuildingPartId(id),
        source: src(),
        name: name.into(),
        number: Some("7".into()),
        kind,
        default_line: None,
        completion: CompletionStatus::Undefined,
        children: vec![],
        devices: vec![],
        parent: None,
    }
}
fn fixture() -> Project {
    let mut p = Project::new(Language("en".into()));
    let mut parent = part(100, "House", BuildingPartType::Building);
    parent.children.push(BuildingPartId(101));
    let mut room = part(101, "Kitchen", BuildingPartType::Room);
    room.parent = Some(parent.id);
    room.devices.push(DeviceId(1));
    let mut buildings = vec![parent, room];
    buildings.extend(
        KINDS
            .iter()
            .enumerate()
            .map(|(n, (kind, _))| part(n as u32 + 1, "", *kind)),
    );
    p.installations.push(Installation {
        id: InstallationId(0),
        name: "".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::Undefined,
        buildings,
        topology: Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: src(),
                name: "".into(),
                address: 3,
                completion: CompletionStatus::Undefined,
                lines: vec![LineId(1)],
            }],
            lines: vec![Line {
                id: LineId(1),
                source: src(),
                name: "".into(),
                address: 4,
                medium_ref: "MT-0".into(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::Undefined,
                devices: vec![DeviceId(1)],
            }],
            unassigned: vec![],
        },
        group_ranges: vec![GroupRange {
            id: GroupRangeId(1),
            source: src(),
            name: "".into(),
            start: GroupAddress::from_raw(2048),
            end: GroupAddress::from_raw(2303),
            parent: None,
            children: vec![],
        }],
        group_addresses: vec![GroupAddressEntry {
            id: GroupAddressId(1),
            source: src(),
            name: "".into(),
            address: GroupAddress::from_raw(2049),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(1)),
            declared_dpt: Override::Absent,
            description: Some("GA description".into()),
        }],
        parameters: vec![],
    });
    p.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: src(),
        name: "".into(),
        description: Some("Device description".into()),
        address: Some(IndividualAddress::new(3, 4, 5).unwrap()),
        product_ref: "M-TEST_P-1".into(),
        program_ref: "M-TEST_HP-1".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    p.functions.push(Function {
        id: FunctionId(1),
        source: src(),
        location: BuildingPartId(101),
        details: FunctionDetails {
            description: Some("Function description".into()),
            ..Default::default()
        },
        members: vec![],
    });
    p.functions.push(Function {
        id: FunctionId(2),
        source: src(),
        location: BuildingPartId(101),
        details: FunctionDetails {
            name: "Light".into(),
            ..Default::default()
        },
        members: vec![FunctionMember {
            id: FunctionMemberId(1),
            source: src(),
            group_address: GroupAddressId(1),
            name: "".into(),
            role: Some("SwitchOnOff".into()),
        }],
    });
    p
}
fn evidence() -> Vec<MissingNameEvidence> {
    vec![
        MissingNameEvidence::ProjectDescription {
            text: "Building renovation".into(),
            association: TextAssociation::Confirmed,
        },
        MissingNameEvidence::InstallationDescription {
            installation: 0,
            text: "West wing".into(),
            association: TextAssociation::Confirmed,
        },
        MissingNameEvidence::ResolvedProduct {
            device: DeviceId(1),
            product_ref: "M-TEST_P-1".into(),
            program_ref: "M-TEST_HP-1".into(),
            product_text: "Actuator".into(),
            association: TextAssociation::Confirmed,
        },
    ]
}
fn proposals(p: &Project, e: &[MissingNameEvidence]) -> MissingNameProposals {
    propose_missing_names(p, e, MissingNameScanRequest::unbounded(0, usize::MAX))
}
fn row(p: &Project, e: &[MissingNameEvidence], target: NameTargetId) -> MissingNameProposal {
    proposals(p, e)
        .items
        .into_iter()
        .find(|r| r.candidate.target == target)
        .unwrap()
}
fn exact(p: &Project, e: &[MissingNameEvidence], target: NameTargetId, expected: &str) {
    let r = row(p, e, target);
    assert_eq!(
        r.proposed_name.as_deref(),
        Some(expected),
        "{target:?}: {:?}",
        r.provenance
    );
    assert_eq!(r.provenance.origin, NameOrigin::Automatic);
    assert!(!r.provenance.sources.is_empty());
}
fn manual(r: &MissingNameProposal) {
    assert_eq!(r.proposed_name, None, "{:?}", r.candidate.target);
    assert_eq!(r.provenance.origin, NameOrigin::ManualRequired);
    assert!(!r.provenance.gaps.is_empty());
}
fn sole_refusal(r: &MissingNameProposal, kind: EvidenceSourceKind) {
    manual(r);
    let blockers: Vec<_> = r
        .provenance
        .gaps
        .iter()
        .filter_map(|g| match g {
            EvidenceGap::Ambiguous { kind, .. } | EvidenceGap::Insufficient { kind, .. } => {
                Some(*kind)
            }
            _ => None,
        })
        .collect();
    assert_eq!(blockers, vec![kind]);
}

#[test]
fn exact_recipes_cover_every_q9_kind_and_all_eleven_building_types_without_mutation() {
    let p = fixture();
    let before = p.clone();
    let e = evidence();
    let e_before = e.clone();
    exact(&p, &e, NameTargetId::Project, "Building renovation");
    exact(&p, &e, NameTargetId::Installation(0), "West wing");
    for (n, (_, label)) in KINDS.iter().enumerate() {
        exact(
            &p,
            &e,
            NameTargetId::BuildingPart(n as u32 + 1),
            &format!("7 {label}"),
        );
    }
    exact(
        &p,
        &e,
        NameTargetId::Device(1),
        "House / Kitchen – Actuator",
    );
    exact(&p, &e, NameTargetId::Area(1), "Area 3");
    exact(&p, &e, NameTargetId::Line(1), "Line 3.4");
    exact(&p, &e, NameTargetId::GroupRange(1), "1/0/0–1/0/255");
    exact(
        &p,
        &e,
        NameTargetId::GroupAddress(1),
        "House / Kitchen – Light – SwitchOnOff",
    );
    exact(&p, &e, NameTargetId::Function(1), "Function description");
    assert_eq!(p, before);
    assert_eq!(e, e_before);
    let result = proposals(&p, &e);
    assert_eq!(result.discovery.items.len(), result.items.len());
    assert_eq!(result.discovery.counts.total_candidates, Some(19));
}

#[test]
fn absent_evidence_never_turns_ids_numbers_or_product_refs_into_semantics() {
    let mut p = fixture();
    p.info.project_number = Some("123".into());
    p.installations[0].buildings[2].number = None;
    p.devices.get_mut(DeviceId(1)).unwrap().description = None;
    p.functions[0].details.description = None;
    p.functions[0].details.number = Some("99".into());
    for target in [
        NameTargetId::Project,
        NameTargetId::Installation(0),
        NameTargetId::BuildingPart(1),
        NameTargetId::Device(1),
        NameTargetId::Function(1),
    ] {
        manual(&row(&p, &[], target));
    }
}

#[test]
fn device_product_alone_without_location_and_description_without_product_show_gaps() {
    let mut p = fixture();
    p.installations[0].buildings[1].devices.clear();
    exact(&p, &evidence(), NameTargetId::Device(1), "Actuator");
    assert!(row(&p, &evidence(), NameTargetId::Device(1))
        .provenance
        .gaps
        .iter()
        .any(|g| matches!(
            g,
            EvidenceGap::Missing {
                kind: EvidenceSourceKind::BuildingLocationPath,
                ..
            }
        )));
    exact(&p, &[], NameTargetId::Device(1), "Device description");
    assert!(row(&p, &[], NameTargetId::Device(1))
        .provenance
        .gaps
        .iter()
        .any(|g| matches!(
            g,
            EvidenceGap::Missing {
                kind: EvidenceSourceKind::ProductDesignation,
                ..
            }
        )));
}

#[test]
fn blank_parent_never_contributes_or_uses_its_automatic_proposal() {
    let mut p = fixture();
    p.installations[0].buildings[0].name.clear();
    exact(
        &p,
        &evidence(),
        NameTargetId::BuildingPart(100),
        "7 Building",
    );
    exact(&p, &evidence(), NameTargetId::Device(1), "Actuator");
    let r = row(&p, &evidence(), NameTargetId::Device(1));
    assert!(r
        .provenance
        .sources
        .iter()
        .all(|s| s.kind != EvidenceSourceKind::BuildingLocationPath));
    assert!(r.provenance.gaps.iter().any(|g| matches!(
        g,
        EvidenceGap::Missing {
            kind: EvidenceSourceKind::ParentName,
            ..
        }
    )));
}

#[test]
fn conflicting_ga_context_cannot_silently_fall_back_to_description() {
    let mut p = fixture();
    let mut f = p.functions[1].clone();
    f.id = FunctionId(3);
    f.details.name = "Heating".into();
    f.members[0].id = FunctionMemberId(2);
    p.functions.push(f);
    sole_refusal(
        &row(&p, &evidence(), NameTargetId::GroupAddress(1)),
        EvidenceSourceKind::FunctionContext,
    );
}

#[test]
fn contradictory_member_text_requires_manual_even_with_description() {
    let mut p = fixture();
    p.functions[1].members[0].name = "Different role".into();
    sole_refusal(
        &row(&p, &evidence(), NameTargetId::GroupAddress(1)),
        EvidenceSourceKind::FunctionMemberContext,
    );
}

#[test]
fn ga_description_is_fallback_and_dpt_alone_is_insufficient() {
    let mut p = fixture();
    p.functions[1].members.clear();
    exact(&p, &[], NameTargetId::GroupAddress(1), "GA description");
    p.installations[0].group_addresses[0].description = None;
    p.installations[0].group_addresses[0].declared_dpt = Override::Value(Resolved {
        value: DptRef {
            main: 1,
            sub: Some(1),
        },
        layer: Layer::Instance,
    });
    manual(&row(&p, &[], NameTargetId::GroupAddress(1)));
}

#[test]
fn unclear_or_contradictory_external_text_and_product_association_require_manual() {
    let p = fixture();
    let mut e = evidence();
    e.push(MissingNameEvidence::ProjectDescription {
        text: "Other purpose".into(),
        association: TextAssociation::Confirmed,
    });
    sole_refusal(
        &row(&p, &e, NameTargetId::Project),
        EvidenceSourceKind::ProjectDescriptiveText,
    );
    let mut e = evidence();
    if let MissingNameEvidence::InstallationDescription { association, .. } = &mut e[1] {
        *association = TextAssociation::Unclear;
    }
    sole_refusal(
        &row(&p, &e, NameTargetId::Installation(0)),
        EvidenceSourceKind::InstallationDescriptiveText,
    );
    let mut e = evidence();
    if let MissingNameEvidence::ResolvedProduct { product_ref, .. } = &mut e[2] {
        *product_ref = "wrong product".into();
    }
    sole_refusal(
        &row(&p, &e, NameTargetId::Device(1)),
        EvidenceSourceKind::ProductDesignation,
    );
    let mut e = evidence();
    let mut other = e[2].clone();
    if let MissingNameEvidence::ResolvedProduct { product_text, .. } = &mut other {
        *product_text = "Other product".into();
    }
    e.push(other);
    sole_refusal(
        &row(&p, &e, NameTargetId::Device(1)),
        EvidenceSourceKind::ProductDesignation,
    );
}

#[test]
fn function_contradictory_description_and_comment_are_manual() {
    let mut p = fixture();
    p.functions[0].details.comment = Some("Different purpose".into());
    sole_refusal(
        &row(&p, &[], NameTargetId::Function(1)),
        EvidenceSourceKind::FunctionContext,
    );
}

#[test]
fn duplicate_target_identity_and_zero_function_id_cannot_auto_propose() {
    let mut p = fixture();
    p.functions[0].id = FunctionId(0);
    sole_refusal(
        &row(&p, &[], NameTargetId::Function(0)),
        EvidenceSourceKind::ParentName,
    );
    let mut p = fixture();
    let duplicate = p.installations[0].group_addresses[0].clone();
    p.installations[0].group_addresses.push(duplicate);
    // Remove function members so duplicate identity is the sole blocker.
    p.functions[1].members.clear();
    let rows: Vec<_> = proposals(&p, &evidence())
        .items
        .into_iter()
        .filter(|r| r.candidate.target == NameTargetId::GroupAddress(1))
        .collect();
    assert_eq!(rows.len(), 2);
    for r in rows {
        sole_refusal(&r, EvidenceSourceKind::ParentName);
    }
}

#[test]
fn line_uses_actual_unique_area_context_even_when_parent_name_is_blank() {
    let mut p = fixture();
    exact(&p, &[], NameTargetId::Line(1), "Line 3.4");
    p.installations[0].topology.areas[0].address = 8;
    exact(&p, &[], NameTargetId::Line(1), "Line 8.4");
    p.installations[0].topology.areas[0].lines.clear();
    sole_refusal(
        &row(&p, &[], NameTargetId::Line(1)),
        EvidenceSourceKind::TopologyAddress,
    );
}

#[test]
fn ambiguous_line_ownership_is_visible_and_manual() {
    let mut p = fixture();
    let mut area = p.installations[0].topology.areas[0].clone();
    area.id = AreaId(2);
    area.address = 2;
    p.installations[0].topology.areas.push(area);
    let r = row(&p, &[], NameTargetId::Line(1));
    sole_refusal(&r, EvidenceSourceKind::ParentName);
    assert!(r.provenance.gaps.iter().any(|g| matches!(
        g,
        EvidenceGap::Ambiguous {
            kind: EvidenceSourceKind::ParentName,
            ..
        }
    )));
}

#[test]
fn dangling_cyclic_and_ambiguous_named_location_ancestors_cannot_be_used() {
    for case in 0..3 {
        let mut p = fixture();
        match case {
            0 => p.installations[0].buildings[0].parent = Some(BuildingPartId(999)),
            1 => p.installations[0].buildings[0].parent = Some(BuildingPartId(101)),
            _ => {
                let parent = p.installations[0].buildings[0].clone();
                p.installations[0].buildings.push(parent);
            }
        }
        sole_refusal(
            &row(&p, &evidence(), NameTargetId::Device(1)),
            EvidenceSourceKind::BuildingLocationPath,
        );
    }
}

#[test]
fn multiple_device_locations_are_manual_despite_usable_product() {
    let mut p = fixture();
    p.installations[0].buildings[0].devices.push(DeviceId(1));
    let r = row(&p, &evidence(), NameTargetId::Device(1));
    sole_refusal(&r, EvidenceSourceKind::ParentName);
    assert!(r.provenance.gaps.iter().any(|g| matches!(
        g,
        EvidenceGap::Ambiguous {
            kind: EvidenceSourceKind::ParentName,
            ..
        }
    )));
}

#[test]
fn range_recipes_use_each_project_address_format_and_reject_reversed_spans() {
    let mut p = fixture();
    for (style, expected) in [
        (GroupAddressStyle::Free, "2048–2303"),
        (GroupAddressStyle::TwoLevel, "1/0–1/255"),
        (GroupAddressStyle::ThreeLevel, "1/0/0–1/0/255"),
    ] {
        p.info.group_address_style = style;
        exact(&p, &[], NameTargetId::GroupRange(1), expected);
    }
    p.installations[0].group_ranges[0].end = GroupAddress::from_raw(100);
    sole_refusal(
        &row(&p, &[], NameTargetId::GroupRange(1)),
        EvidenceSourceKind::GroupRangeSpan,
    );
}

#[test]
fn duplicate_labels_are_allowed_and_warned_without_suffixes() {
    let mut p = fixture();
    p.functions[1].members.clear();
    let mut ga = p.installations[0].group_addresses[0].clone();
    ga.id = GroupAddressId(2);
    p.installations[0].group_addresses.push(ga);
    for id in [1, 2] {
        let r = row(&p, &[], NameTargetId::GroupAddress(id));
        assert_eq!(r.proposed_name.as_deref(), Some("GA description"));
        assert!(r.duplicate_label);
    }
    p.installations[0].group_addresses[1].name = "GA description".into();
    assert!(row(&p, &[], NameTargetId::GroupAddress(1)).duplicate_label);
}

#[test]
fn all_nonblank_names_including_defaults_placeholders_addresses_and_feff_are_preserved() {
    for name in ["Default", "-", "3.4.5", "\u{feff}", "  Existing  "] {
        let mut p = fixture();
        p.info.name = name.into();
        for i in &mut p.installations {
            i.name = name.into();
            for b in &mut i.buildings {
                b.name = name.into();
            }
            for a in &mut i.topology.areas {
                a.name = name.into();
            }
            for l in &mut i.topology.lines {
                l.name = name.into();
            }
            for r in &mut i.group_ranges {
                r.name = name.into();
            }
            for g in &mut i.group_addresses {
                g.name = name.into();
            }
        }
        p.devices.get_mut(DeviceId(1)).unwrap().name = name.into();
        for f in &mut p.functions {
            f.details.name = name.into();
        }
        let before = p.clone();
        assert!(proposals(&p, &evidence()).items.is_empty());
        assert_eq!(p, before);
    }
}

#[test]
fn generated_invalid_names_fail_manual_without_truncation_or_sanitization() {
    for bad in [
        "bad\ntext".into(),
        "x".repeat(1025),
        "bad\u{2028}text".into(),
    ] {
        let p = fixture();
        let e = vec![MissingNameEvidence::ProjectDescription {
            text: bad.clone(),
            association: TextAssociation::Confirmed,
        }];
        let r = row(&p, &e, NameTargetId::Project);
        sole_refusal(&r, EvidenceSourceKind::ParentName);
        assert_eq!(r.provenance.sources[0].value, bad);
    }
    let mut p = fixture();
    p.functions[0].details.description = Some("x".repeat(256));
    sole_refusal(
        &row(&p, &[], NameTargetId::Function(1)),
        EvidenceSourceKind::ParentName,
    );
    p.installations[0].buildings[2].number = Some("bad\t7".into());
    sole_refusal(
        &row(&p, &[], NameTargetId::BuildingPart(1)),
        EvidenceSourceKind::ParentName,
    );
}

#[test]
fn bounded_page_keeps_discovery_contract_and_is_deterministic() {
    let p = fixture();
    let before = p.clone();
    let request = MissingNameScanRequest::bounded(6, 1, 2);
    let r = propose_missing_names(&p, &evidence(), request);
    assert_eq!(r.discovery, detect_missing_names(&p, request));
    assert_eq!(r.discovery.completeness, ScanCompleteness::Incomplete);
    assert_eq!(r.discovery.counts.total_candidates, None);
    assert_eq!(r, propose_missing_names(&p, &evidence(), request));
    assert_eq!(p, before);
}

#[test]
fn incomplete_competing_function_context_is_not_ignored_for_description_or_complete_context() {
    let mut p = fixture();
    let mut f = p.functions[1].clone();
    f.id = FunctionId(3);
    f.details.name.clear();
    f.members[0].id = FunctionMemberId(2);
    p.functions.push(f);
    sole_refusal(
        &row(&p, &evidence(), NameTargetId::GroupAddress(1)),
        EvidenceSourceKind::FunctionContext,
    );
}

#[test]
fn ga_never_uses_an_automatic_function_or_location_name() {
    let mut p = fixture();
    p.functions[1].details.name.clear();
    p.functions[1].details.description = Some("Suggested function".into());
    exact(&p, &[], NameTargetId::Function(2), "Suggested function");
    exact(&p, &[], NameTargetId::GroupAddress(1), "GA description");
    p.functions[1].details.name = "Light".into();
    p.installations[0].buildings[1].name.clear();
    exact(&p, &[], NameTargetId::BuildingPart(101), "7 Room");
    exact(&p, &[], NameTargetId::GroupAddress(1), "GA description");
}

#[test]
fn invalid_topology_addresses_and_structural_parent_references_are_manual() {
    let mut p = fixture();
    p.installations[0].topology.areas[0].address = 16;
    sole_refusal(
        &row(&p, &[], NameTargetId::Area(1)),
        EvidenceSourceKind::TopologyAddress,
    );
    sole_refusal(
        &row(&p, &[], NameTargetId::Line(1)),
        EvidenceSourceKind::TopologyAddress,
    );
    let mut p = fixture();
    p.installations[0].buildings[2].parent = Some(BuildingPartId(999));
    sole_refusal(
        &row(&p, &[], NameTargetId::BuildingPart(1)),
        EvidenceSourceKind::ParentName,
    );
    let mut p = fixture();
    p.installations[0].group_ranges[0].parent = Some(GroupRangeId(1));
    sole_refusal(
        &row(&p, &[], NameTargetId::GroupRange(1)),
        EvidenceSourceKind::ParentName,
    );
}

#[test]
fn review_function_location_cycle_must_refuse() {
    let mut p = fixture();
    p.installations[0].buildings[0].parent = Some(BuildingPartId(101));
    sole_refusal(
        &row(&p, &[], NameTargetId::Function(1)),
        EvidenceSourceKind::BuildingLocationPath,
    );
}

#[test]
fn review_function_location_dangling_ancestor_must_refuse() {
    let mut p = fixture();
    p.installations[0].buildings[0].parent = Some(BuildingPartId(999));
    sole_refusal(
        &row(&p, &[], NameTargetId::Function(1)),
        EvidenceSourceKind::BuildingLocationPath,
    );
}

#[test]
fn review_function_ambiguous_installation_must_refuse() {
    let mut p = fixture();
    let mut other = p.installations[0].clone();
    other.buildings.clear();
    other.topology.areas.clear();
    other.topology.lines.clear();
    other.group_ranges.clear();
    other.group_addresses.clear();
    p.installations.push(other);
    sole_refusal(
        &row(&p, &[], NameTargetId::Function(1)),
        EvidenceSourceKind::BuildingLocationPath,
    );
}

#[test]
fn review_ga_cyclic_range_ancestor_must_refuse() {
    let mut p = fixture();
    p.functions[1].members.clear();
    p.installations[0].group_ranges[0].parent = Some(GroupRangeId(1));
    sole_refusal(
        &row(&p, &[], NameTargetId::GroupAddress(1)),
        EvidenceSourceKind::ParentName,
    );
}

#[test]
fn review_building_dangling_ancestor_must_refuse() {
    let mut p = fixture();
    p.installations[0].buildings[1].name.clear();
    p.installations[0].buildings[0].parent = Some(BuildingPartId(999));
    sole_refusal(
        &row(&p, &[], NameTargetId::BuildingPart(101)),
        EvidenceSourceKind::ParentName,
    );
}

#[test]
fn review_device_ambiguous_topology_ancestor_must_refuse() {
    let mut p = fixture();
    let mut a = p.installations[0].topology.areas[0].clone();
    a.id = AreaId(2);
    p.installations[0].topology.areas.push(a);
    sole_refusal(
        &row(&p, &evidence(), NameTargetId::Device(1)),
        EvidenceSourceKind::ParentName,
    );
}

#[test]
fn review_fallback_provenance_only_contributors() {
    let p = fixture();
    let r = row(&p, &[], NameTargetId::Device(1));
    assert!(r.provenance.gaps.iter().any(|gap| matches!(
        gap,
        EvidenceGap::Missing {
            kind: EvidenceSourceKind::ProductDesignation,
            ..
        }
    )));
    assert_eq!(r.proposed_name.as_deref(), Some("Device description"));
    assert_eq!(r.provenance.origin, NameOrigin::Automatic);
    assert_eq!(
        r.provenance.sources,
        vec![EvidenceContribution {
            kind: EvidenceSourceKind::DeviceDescription,
            source_target: Some(NameTargetId::Device(1)),
            value: "Device description".into(),
        }]
    );
}

#[test]
fn ga_fallback_has_only_contributing_sources_and_preserves_gaps() {
    for missing in 0..3 {
        let mut p = fixture();
        let missing_kind = match missing {
            0 => {
                p.functions[1].members[0].role = None;
                EvidenceSourceKind::FunctionMemberContext
            }
            1 => {
                p.functions[1].details.name.clear();
                EvidenceSourceKind::FunctionContext
            }
            _ => {
                p.installations[0].buildings[1].name.clear();
                EvidenceSourceKind::ParentName
            }
        };
        let before = p.clone();
        let r = row(&p, &[], NameTargetId::GroupAddress(1));
        assert_eq!(p, before);
        assert_eq!(r.proposed_name.as_deref(), Some("GA description"));
        assert_eq!(r.provenance.origin, NameOrigin::Automatic);
        assert_eq!(
            r.provenance.sources,
            vec![EvidenceContribution {
                kind: EvidenceSourceKind::GroupAddressDescription,
                source_target: Some(NameTargetId::GroupAddress(1)),
                value: "GA description".into(),
            }]
        );
        assert!(r.provenance.gaps.iter().any(|gap| matches!(
            gap,
            EvidenceGap::Missing { kind, .. } if *kind == missing_kind
        )));
        assert!(r.provenance.gaps.iter().any(|gap| matches!(
            gap,
            EvidenceGap::Missing {
                kind: EvidenceSourceKind::FunctionContext,
                ..
            }
        )));
    }
}

#[test]
fn complete_ga_context_keeps_exact_contributing_sources() {
    let p = fixture();
    let before = p.clone();
    let r = row(&p, &[], NameTargetId::GroupAddress(1));
    assert_eq!(p, before);
    assert_eq!(
        r.proposed_name.as_deref(),
        Some("House / Kitchen – Light – SwitchOnOff")
    );
    assert_eq!(r.provenance.origin, NameOrigin::Automatic);
    assert!(r.provenance.gaps.is_empty());
    assert_eq!(
        r.provenance.sources,
        vec![
            (
                EvidenceSourceKind::ParentName,
                NameTargetId::BuildingPart(100),
                "House"
            ),
            (
                EvidenceSourceKind::ParentName,
                NameTargetId::BuildingPart(101),
                "Kitchen"
            ),
            (
                EvidenceSourceKind::BuildingLocationPath,
                NameTargetId::BuildingPart(101),
                "House / Kitchen"
            ),
            (
                EvidenceSourceKind::FunctionMemberContext,
                NameTargetId::Function(2),
                "SwitchOnOff"
            ),
            (
                EvidenceSourceKind::FunctionContext,
                NameTargetId::Function(2),
                "Light"
            ),
            (
                EvidenceSourceKind::FunctionContext,
                NameTargetId::GroupAddress(1),
                "House / Kitchen – Light – SwitchOnOff"
            ),
        ]
        .into_iter()
        .map(|(kind, target, value)| EvidenceContribution {
            kind,
            source_target: Some(target),
            value: value.into(),
        })
        .collect::<Vec<_>>()
    );
}

#[test]
fn structural_checks_ignore_unrelated_invalid_named_contexts() {
    let mut p = fixture();
    let mut unrelated = part(900, "Unrelated", BuildingPartType::Building);
    unrelated.parent = Some(BuildingPartId(900));
    p.installations[0].buildings.push(unrelated);
    let mut unrelated = p.installations[0].group_ranges[0].clone();
    unrelated.id = GroupRangeId(900);
    unrelated.name = "Unrelated".into();
    unrelated.parent = Some(unrelated.id);
    p.installations[0].group_ranges.push(unrelated);
    exact(&p, &[], NameTargetId::Function(1), "Function description");
    exact(
        &p,
        &evidence(),
        NameTargetId::Device(1),
        "House / Kitchen – Actuator",
    );
    p.functions[1].members.clear();
    exact(&p, &[], NameTargetId::GroupAddress(1), "GA description");
}

#[test]
fn named_range_ancestors_require_unique_identity_and_same_installation() {
    for case in 0..3 {
        let mut p = fixture();
        p.functions[1].members.clear();
        let mut parent = p.installations[0].group_ranges[0].clone();
        parent.id = GroupRangeId(2);
        parent.name = "Named ancestor".into();
        p.installations[0].group_ranges[0].parent = Some(parent.id);
        match case {
            0 => parent.parent = Some(GroupRangeId(999)),
            1 => p.installations[0].group_ranges.push(parent.clone()),
            _ => {
                let mut other = p.installations[0].clone();
                other.id = InstallationId(1);
                other.buildings.clear();
                other.topology.areas.clear();
                other.topology.lines.clear();
                other.group_addresses.clear();
                other.group_ranges = vec![parent];
                p.installations.push(other);
                sole_refusal(
                    &row(&p, &[], NameTargetId::GroupAddress(1)),
                    EvidenceSourceKind::ParentName,
                );
                continue;
            }
        }
        p.installations[0].group_ranges.push(parent);
        sole_refusal(
            &row(&p, &[], NameTargetId::GroupAddress(1)),
            EvidenceSourceKind::ParentName,
        );
    }
}

#[test]
fn duplicate_named_topology_area_identity_blocks_device() {
    let mut p = fixture();
    let mut duplicate = p.installations[0].topology.areas[0].clone();
    duplicate.name = "Other area".into();
    duplicate.lines.clear();
    p.installations[0].topology.areas.push(duplicate);
    sole_refusal(
        &row(&p, &evidence(), NameTargetId::Device(1)),
        EvidenceSourceKind::ParentName,
    );
}

#[test]
fn device_location_and_topology_must_share_installation_in_either_order() {
    for reverse in [false, true] {
        let mut p = fixture();
        let mut other = p.installations[0].clone();
        other.id = InstallationId(1);
        other.buildings.clear();
        other.group_ranges.clear();
        other.group_addresses.clear();
        p.installations[0].topology.areas.clear();
        p.installations[0].topology.lines.clear();
        p.installations.push(other);
        if reverse {
            p.installations.reverse();
        }
        manual(&row(&p, &evidence(), NameTargetId::Device(1)));
    }
}
