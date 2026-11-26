//! Verifies Q5/Q6/Q9/Q13/Q16/Q17 missing-name detection and provenance invariants.

use knx_core::commissioning::{CommissioningState, CompletionStatus};
use knx_core::flags::ResolvedFlags;
use knx_core::provenance::Override;
use knx_core::{
    Area, AreaId, BinaryDataRef, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance,
    ComObjectInstanceId, DeviceId, DeviceInstance, Function, FunctionDetails, FunctionId,
    FunctionMember, FunctionMemberId, GroupAddress, GroupAddressEntry, GroupAddressId, GroupRange,
    GroupRangeId, IndividualAddress, Installation, InstallationId, Language, Line, LineId, Project,
    SourceRef, Topology,
};
use knx_edit::missing_names::{
    detect_missing_names, BuildingPartKind, EvidenceContribution, EvidenceGap, EvidenceSourceKind,
    MissingNameScanRequest, NameOrigin, NameProvenance, NameTargetId, NameTargetKind,
    ParentRefState, ParentRelation, ScanCompleteness, TargetIdentityStatus,
};

const ALL_BUILDING_KINDS: [BuildingPartType; 11] = [
    BuildingPartType::Building,
    BuildingPartType::Floor,
    BuildingPartType::Room,
    BuildingPartType::Corridor,
    BuildingPartType::DistributionBoard,
    BuildingPartType::BuildingPart,
    BuildingPartType::Stairway,
    BuildingPartType::RoomPart,
    BuildingPartType::Area,
    BuildingPartType::Ground,
    BuildingPartType::Segment,
];

fn src(tag: &str) -> SourceRef {
    SourceRef {
        path: "synthetic.xml".into(),
        ets_id: tag.into(),
    }
}

fn make_part(
    id: u32,
    kind: BuildingPartType,
    name: &str,
    parent: Option<u32>,
    devices: Vec<DeviceId>,
) -> BuildingPart {
    BuildingPart {
        id: BuildingPartId(id),
        source: src(&format!("BP-{id}")),
        name: name.into(),
        number: Some(format!("N-{id}")),
        kind,
        default_line: None,
        completion: CompletionStatus::Undefined,
        children: Vec::new(),
        devices,
        parent: parent.map(BuildingPartId),
    }
}

fn make_device(id: u32, name: &str) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(id),
        source: src(&format!("D-{id}")),
        name: name.into(),
        description: Some(String::new()),
        address: Some(IndividualAddress::new(1, 1, id as u8).unwrap()),
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![ComObjectInstanceId(id)],
        binary_data: vec![BinaryDataRef {
            id: format!("bin-{id}"),
            name: String::new(),
        }],
    }
}

fn full_blank_project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.project_id = "P-TEST".into();
    project.info.name = String::new(); // blank: empty

    // Build a chain of all 11 BuildingPart kinds, stored in REVERSE order to test topo sort.
    let mut buildings = Vec::new();
    for (i, &kind) in ALL_BUILDING_KINDS.iter().enumerate() {
        let id = (i + 1) as u32;
        let parent = if i == 0 { None } else { Some(i as u32) };
        let blank = match i % 4 {
            0 => "",
            1 => " ",
            2 => "\t\r\n",
            _ => "\u{85}\u{2003}",
        };
        let devices = if i == 10 { vec![DeviceId(1)] } else { vec![] };
        buildings.push(make_part(id, kind, blank, parent, devices));
    }
    buildings.reverse();

    // 3-level nested GroupRanges, also stored in reverse order.
    let group_ranges = vec![
        GroupRange {
            id: GroupRangeId(30),
            source: src("GR-30"),
            name: "\u{2003}".into(),
            start: GroupAddress::from_raw(2048),
            end: GroupAddress::from_raw(2063),
            parent: Some(GroupRangeId(20)),
            children: vec![],
        },
        GroupRange {
            id: GroupRangeId(20),
            source: src("GR-20"),
            name: "  ".into(),
            start: GroupAddress::from_raw(2048),
            end: GroupAddress::from_raw(2303),
            parent: Some(GroupRangeId(10)),
            children: vec![GroupRangeId(30)],
        },
        GroupRange {
            id: GroupRangeId(10),
            source: src("GR-10"),
            name: String::new(),
            start: GroupAddress::from_raw(2048),
            end: GroupAddress::from_raw(4095),
            parent: None,
            children: vec![GroupRangeId(20)],
        },
    ];

    let inst = Installation {
        id: InstallationId(0),
        name: " \t ".into(),
        default_line: Some(LineId(1)),
        multicast_address: None,
        completion: CompletionStatus::Undefined,
        topology: Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: src("A-1"),
                name: String::new(),
                address: 1,
                completion: CompletionStatus::Undefined,
                lines: vec![LineId(1)],
            }],
            lines: vec![Line {
                id: LineId(1),
                source: src("L-1"),
                name: "\n".into(),
                address: 2,
                medium_ref: String::new(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::Undefined,
                devices: vec![DeviceId(1)],
            }],
            unassigned: vec![],
        },
        buildings,
        group_ranges,
        group_addresses: vec![GroupAddressEntry {
            id: GroupAddressId(1),
            source: src("GA-1"),
            name: String::new(),
            address: GroupAddress::from_raw(2049),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(30)),
            declared_dpt: Override::Absent,
            description: Some(String::new()),
        }],
        parameters: vec![],
    };
    project.installations.push(inst);

    project.devices.insert(make_device(1, "   "));
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: src("CO-1"),
        device: DeviceId(1),
        number: 0,
        text: Override::Empty,
        description: Override::Empty,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
        module_instance: None,
        priority_edit: None,
        saved_instance_priority: Default::default(),
    });

    project.functions.push(Function {
        id: FunctionId(1),
        source: src("F-1"),
        location: BuildingPartId(11),
        details: FunctionDetails {
            name: String::new(),
            function_type: Some("FT-1".into()),
            number: None,
            comment: Some(String::new()),
            description: Some(String::new()),
            completion: CompletionStatus::Undefined,
        },
        members: vec![FunctionMember {
            id: FunctionMemberId(1),
            source: src("FM-1"),
            group_address: GroupAddressId(1),
            name: String::new(),
            role: None,
        }],
    });

    project
}

#[test]
fn detects_all_q9_types_including_all_11_building_kinds_in_parent_before_child_order() {
    let project = full_blank_project();
    let before = project.clone();

    let result = detect_missing_names(&project, MissingNameScanRequest::unbounded(0, 100));

    // Strictly read-only: project is byte-for-byte unchanged.
    assert_eq!(project, before);
    assert_eq!(result.completeness, ScanCompleteness::Complete);
    // 1 Project + 1 Installation + 11 BuildingParts + 1 Area + 1 Line + 3 GroupRanges
    // + 1 GroupAddress + 1 Device + 1 Function = 21 Q9 entities.
    assert_eq!(result.counts.entities_scanned, 21);
    assert_eq!(result.counts.candidates_found_in_scan, 21);
    assert_eq!(result.counts.total_candidates, Some(21));
    assert!(!result.has_more_in_scan);

    let by_kind = result.counts.total_by_kind.as_ref().unwrap();
    assert_eq!(by_kind.get(&NameTargetKind::Project), Some(&1));
    assert_eq!(by_kind.get(&NameTargetKind::Installation), Some(&1));
    assert_eq!(by_kind.get(&NameTargetKind::BuildingPart), Some(&11));
    assert_eq!(by_kind.get(&NameTargetKind::Area), Some(&1));
    assert_eq!(by_kind.get(&NameTargetKind::Line), Some(&1));
    assert_eq!(by_kind.get(&NameTargetKind::GroupRange), Some(&3));
    assert_eq!(by_kind.get(&NameTargetKind::GroupAddress), Some(&1));
    assert_eq!(by_kind.get(&NameTargetKind::Device), Some(&1));
    assert_eq!(by_kind.get(&NameTargetKind::Function), Some(&1));

    // Exact parent-before-child ordering despite reversed input Vecs.
    let targets: Vec<NameTargetId> = result.items.iter().map(|c| c.target).collect();
    let expected: Vec<NameTargetId> = vec![
        NameTargetId::Project,
        NameTargetId::Installation(0),
        NameTargetId::BuildingPart(1),
        NameTargetId::BuildingPart(2),
        NameTargetId::BuildingPart(3),
        NameTargetId::BuildingPart(4),
        NameTargetId::BuildingPart(5),
        NameTargetId::BuildingPart(6),
        NameTargetId::BuildingPart(7),
        NameTargetId::BuildingPart(8),
        NameTargetId::BuildingPart(9),
        NameTargetId::BuildingPart(10),
        NameTargetId::BuildingPart(11),
        NameTargetId::Area(1),
        NameTargetId::Line(1),
        NameTargetId::GroupRange(10),
        NameTargetId::GroupRange(20),
        NameTargetId::GroupRange(30),
        NameTargetId::GroupAddress(1),
        NameTargetId::Device(1),
        NameTargetId::Function(1),
    ];
    assert_eq!(targets, expected);

    // Every one of the 11 BuildingPartKind variants is represented.
    let observed_kinds: Vec<BuildingPartKind> = result
        .items
        .iter()
        .filter(|c| c.kind == NameTargetKind::BuildingPart)
        .map(|c| c.context.building_part_kind.unwrap())
        .collect();
    let expected_kinds: Vec<BuildingPartKind> = ALL_BUILDING_KINDS
        .into_iter()
        .map(BuildingPartKind::from)
        .collect();
    assert_eq!(observed_kinds, expected_kinds);

    // Verify Function has 255 codepoint ceiling while all other Q9 types have 1024.
    for item in &result.items {
        assert_eq!(item.identity_status, TargetIdentityStatus::Unique);
        let expected_max = if item.kind == NameTargetKind::Function {
            255
        } else {
            1024
        };
        assert_eq!(item.max_name_codepoints, expected_max, "{:?}", item.target);
    }

    // Verify resolved parent references carry parent_is_blank = true.
    let device = result
        .items
        .iter()
        .find(|c| c.target == NameTargetId::Device(1))
        .unwrap();
    assert_eq!(device.context.individual_address.as_deref(), Some("1.1.1"));
    assert!(device.parents.iter().any(|p| {
        p.relation == ParentRelation::TopologyLine
            && p.state
                == ParentRefState::Resolved {
                    target: NameTargetId::Line(1),
                    parent_is_blank: true,
                }
    }));
    assert!(device.parents.iter().any(|p| {
        p.relation == ParentRelation::DeviceLocation
            && p.state
                == ParentRefState::Resolved {
                    target: NameTargetId::BuildingPart(11),
                    parent_is_blank: true,
                }
    }));

    let line = result
        .items
        .iter()
        .find(|c| c.target == NameTargetId::Line(1))
        .unwrap();
    assert_eq!(line.context.topology_area_address, Some(1));
    assert_eq!(line.context.topology_line_address, Some(2));
}

#[test]
fn preserves_nonempty_defaults_placeholders_addresses_feff_and_excludes_non_q9_fields() {
    let mut project = full_blank_project();
    // Give every Q9 entity a nonempty name that MUST be preserved (Q5).
    project.info.name = "\u{feff}".into(); // BOM is NOT Unicode White_Space
    let inst = &mut project.installations[0];
    inst.name = "Default Installation".into();
    let nonempty_samples = [
        "New Building",
        "TBD",
        "-",
        "1.1.1",
        "1/2/3",
        "\u{feff}",
        " \u{feff} ",
        "Unnamed Room",
        "N/A",
        "0",
        "Room ?",
    ];
    for (part, sample) in inst.buildings.iter_mut().zip(nonempty_samples) {
        part.name = sample.into();
    }
    inst.topology.areas[0].name = "1".into();
    inst.topology.lines[0].name = "1.2".into();
    for (range, label) in inst
        .group_ranges
        .iter_mut()
        .zip(["1/0/0-1/7/255", "TBD", "\u{feff}"])
    {
        range.name = label.into();
    }
    inst.group_addresses[0].name = "1/0/1".into();
    // Leave GA description, Device description, product_ref, program_ref, medium_ref,
    // BinaryDataRef.name, ComObjectInstance.text/description, FunctionMember.name all empty!
    project.devices.get_mut(DeviceId(1)).unwrap().name = "Default Device".into();
    project.functions[0].details.name = "Placeholder Function".into();

    let result = detect_missing_names(&project, MissingNameScanRequest::unbounded(0, 100));
    assert_eq!(result.completeness, ScanCompleteness::Complete);
    assert_eq!(result.counts.entities_scanned, 21);
    assert_eq!(result.counts.candidates_found_in_scan, 0);
    assert_eq!(result.counts.total_candidates, Some(0));
    assert!(result.items.is_empty());
}

#[test]
fn surfaces_duplicate_zero_identities_and_missing_ambiguous_cyclic_parents() {
    let mut project = Project::new(Language("en".into()));
    project.info.name = "Named Project".into();

    // Two installations sharing InstallationId(1) — both blank.
    let inst_a = Installation {
        id: InstallationId(1),
        name: String::new(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::Undefined,
        topology: Topology {
            areas: vec![
                Area {
                    id: AreaId(1),
                    source: src("A-1"),
                    name: "Area One".into(),
                    address: 1,
                    completion: CompletionStatus::Undefined,
                    lines: vec![LineId(1)],
                },
                Area {
                    id: AreaId(2),
                    source: src("A-2"),
                    name: "Area Two".into(),
                    address: 2,
                    completion: CompletionStatus::Undefined,
                    lines: vec![LineId(1)], // Ambiguous line ownership!
                },
            ],
            lines: vec![
                Line {
                    id: LineId(1),
                    source: src("L-1"),
                    name: String::new(),
                    address: 1,
                    medium_ref: "TP".into(),
                    domain_address: None,
                    domain_address_is_checked: None,
                    ip_routing_multicast_address: None,
                    multicast_ttl: None,
                    completion: CompletionStatus::Undefined,
                    devices: vec![DeviceId(1)],
                },
                Line {
                    id: LineId(2),
                    source: src("L-2"),
                    name: String::new(), // Orphan line (no area)
                    address: 2,
                    medium_ref: "TP".into(),
                    domain_address: None,
                    domain_address_is_checked: None,
                    ip_routing_multicast_address: None,
                    multicast_ttl: None,
                    completion: CompletionStatus::Undefined,
                    devices: vec![DeviceId(1)], // Device 1 in two lines!
                },
            ],
            unassigned: vec![],
        },
        buildings: vec![
            // Cyclic building parts: 1 -> 2 -> 1
            make_part(
                1,
                BuildingPartType::Building,
                "",
                Some(2),
                vec![DeviceId(1)],
            ),
            make_part(2, BuildingPartType::Floor, "", Some(1), vec![DeviceId(1)]),
            // Dangling parent reference
            make_part(3, BuildingPartType::Room, "", Some(999), vec![]),
        ],
        group_ranges: vec![
            // Cyclic group ranges: 10 -> 11 -> 10
            GroupRange {
                id: GroupRangeId(10),
                source: src("GR-10"),
                name: String::new(),
                start: GroupAddress::from_raw(1),
                end: GroupAddress::from_raw(10),
                parent: Some(GroupRangeId(11)),
                children: vec![GroupRangeId(11)],
            },
            GroupRange {
                id: GroupRangeId(11),
                source: src("GR-11"),
                name: String::new(),
                start: GroupAddress::from_raw(1),
                end: GroupAddress::from_raw(5),
                parent: Some(GroupRangeId(10)),
                children: vec![GroupRangeId(10)],
            },
        ],
        group_addresses: vec![GroupAddressEntry {
            id: GroupAddressId(5),
            source: src("GA-5"),
            name: String::new(),
            address: GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(888)), // Missing range
            declared_dpt: Override::Absent,
            description: None,
        }],
        parameters: vec![],
    };

    let mut inst_b = inst_a.clone();
    inst_b.topology.areas.clear();
    inst_b.topology.lines.clear();
    inst_b.buildings.clear();
    inst_b.group_ranges.clear();
    // Duplicate GA ID 5 across installations!
    inst_b.group_addresses = vec![GroupAddressEntry {
        id: GroupAddressId(5),
        source: src("GA-5-dup"),
        name: " ".into(),
        address: GroupAddress::from_raw(2),
        central: false,
        unfiltered: false,
        range: None,
        declared_dpt: Override::Absent,
        description: None,
    }];

    project.installations.push(inst_a);
    project.installations.push(inst_b);
    project.devices.insert(make_device(1, ""));
    project.functions.push(Function {
        id: FunctionId(0), // Reserved zero ID!
        source: src("F-0"),
        location: BuildingPartId(777), // Missing building part
        details: FunctionDetails::default(),
        members: vec![],
    });

    let result = detect_missing_names(&project, MissingNameScanRequest::unbounded(0, 100));
    assert_eq!(result.completeness, ScanCompleteness::Complete);

    // Both duplicate installations surfaced with distinct occurrence_index.
    let insts: Vec<_> = result
        .items
        .iter()
        .filter(|c| c.target == NameTargetId::Installation(1))
        .collect();
    assert_eq!(insts.len(), 2);
    assert_eq!(insts[0].occurrence_index, 0);
    assert_eq!(insts[1].occurrence_index, 1);
    assert_eq!(
        insts[0].identity_status,
        TargetIdentityStatus::DuplicateId { occurrences: 2 }
    );

    // Both duplicate GAs surfaced across installations.
    let gas: Vec<_> = result
        .items
        .iter()
        .filter(|c| c.target == NameTargetId::GroupAddress(5))
        .collect();
    assert_eq!(gas.len(), 2);
    assert_eq!(
        gas[0].identity_status,
        TargetIdentityStatus::DuplicateId { occurrences: 2 }
    );
    assert!(gas[0].parents.iter().any(|p| {
        p.relation == ParentRelation::GroupAddressRange
            && p.state
                == ParentRefState::Missing {
                    expected: Some(NameTargetId::GroupRange(888)),
                }
    }));

    // Cyclic building parts & group ranges surfaced cleanly without infinite loop.
    let bp1 = result
        .items
        .iter()
        .find(|c| c.target == NameTargetId::BuildingPart(1))
        .unwrap();
    assert!(bp1.parents.iter().any(|p| {
        p.relation == ParentRelation::BuildingParent
            && p.state
                == ParentRefState::Cyclic {
                    target: NameTargetId::BuildingPart(2),
                }
    }));
    let bp3 = result
        .items
        .iter()
        .find(|c| c.target == NameTargetId::BuildingPart(3))
        .unwrap();
    assert!(bp3.parents.iter().any(|p| {
        p.relation == ParentRelation::BuildingParent
            && p.state
                == ParentRefState::Missing {
                    expected: Some(NameTargetId::BuildingPart(999)),
                }
    }));

    let gr10 = result
        .items
        .iter()
        .find(|c| c.target == NameTargetId::GroupRange(10))
        .unwrap();
    assert!(gr10.parents.iter().any(|p| {
        p.relation == ParentRelation::GroupRangeParent
            && p.state
                == ParentRefState::Cyclic {
                    target: NameTargetId::GroupRange(11),
                }
    }));

    // Ambiguous area ownership for Line 1, missing area ownership for Line 2.
    let line1 = result
        .items
        .iter()
        .find(|c| c.target == NameTargetId::Line(1))
        .unwrap();
    assert!(line1.parents.iter().any(|p| {
        p.relation == ParentRelation::TopologyArea
            && p.state
                == ParentRefState::Ambiguous {
                    candidates: vec![NameTargetId::Area(1), NameTargetId::Area(2)],
                }
    }));
    let line2 = result
        .items
        .iter()
        .find(|c| c.target == NameTargetId::Line(2))
        .unwrap();
    assert!(line2.parents.iter().any(|p| {
        p.relation == ParentRelation::TopologyArea
            && p.state == ParentRefState::Missing { expected: None }
    }));

    // Device 1 has ambiguous line and building placements.
    let dev1 = result
        .items
        .iter()
        .find(|c| c.target == NameTargetId::Device(1))
        .unwrap();
    assert!(dev1.parents.iter().any(|p| {
        p.relation == ParentRelation::TopologyLine
            && p.state
                == ParentRefState::Ambiguous {
                    candidates: vec![NameTargetId::Line(1), NameTargetId::Line(2)],
                }
    }));
    assert!(dev1.parents.iter().any(|p| {
        p.relation == ParentRelation::DeviceLocation
            && p.state
                == ParentRefState::Ambiguous {
                    candidates: vec![NameTargetId::BuildingPart(1), NameTargetId::BuildingPart(2)],
                }
    }));

    // FunctionId(0) surfaces InvalidZeroId and missing location.
    let func0 = result
        .items
        .iter()
        .find(|c| c.target == NameTargetId::Function(0))
        .unwrap();
    assert_eq!(func0.identity_status, TargetIdentityStatus::InvalidZeroId);
    assert!(func0.parents.iter().any(|p| {
        p.relation == ParentRelation::FunctionLocation
            && p.state
                == ParentRefState::Missing {
                    expected: Some(NameTargetId::BuildingPart(777)),
                }
    }));
}

#[test]
fn bounded_scan_suppresses_full_totals_when_incomplete_and_paginates_deterministically() {
    let project = full_blank_project(); // 21 Q9 entities, all blank

    // 0-entity scan budget is explicitly incomplete and withholds totals.
    let zero = detect_missing_names(&project, MissingNameScanRequest::bounded(0, 0, 10));
    assert_eq!(zero.completeness, ScanCompleteness::Incomplete);
    assert_eq!(zero.counts.entities_scanned, 0);
    assert_eq!(zero.counts.candidates_found_in_scan, 0);
    assert_eq!(zero.counts.total_candidates, None);
    assert_eq!(zero.counts.total_by_kind, None);
    assert!(zero.items.is_empty());

    // 20-entity budget on a 21-entity project is incomplete and MUST NOT report full totals.
    let truncated = detect_missing_names(&project, MissingNameScanRequest::bounded(20, 0, 100));
    assert_eq!(truncated.completeness, ScanCompleteness::Incomplete);
    assert_eq!(truncated.counts.entities_scanned, 20);
    assert_eq!(truncated.counts.candidates_found_in_scan, 20);
    assert_eq!(truncated.counts.total_candidates, None);
    assert_eq!(truncated.counts.total_by_kind, None);
    assert_eq!(truncated.items.len(), 20);

    // Exact 21-entity budget completes the scan and reports full totals.
    let exact = detect_missing_names(&project, MissingNameScanRequest::bounded(21, 0, 100));
    assert_eq!(exact.completeness, ScanCompleteness::Complete);
    assert_eq!(exact.counts.entities_scanned, 21);
    assert_eq!(exact.counts.total_candidates, Some(21));
    assert!(exact.counts.total_by_kind.is_some());

    // Paginate through all 21 candidates in pages of 5.
    let mut collected = Vec::new();
    let mut offset = 0;
    loop {
        let page = detect_missing_names(&project, MissingNameScanRequest::unbounded(offset, 5));
        assert_eq!(page.completeness, ScanCompleteness::Complete);
        assert_eq!(page.counts.total_candidates, Some(21));
        collected.extend(page.items.iter().cloned());
        if !page.has_more_in_scan {
            break;
        }
        offset += 5;
    }
    assert_eq!(collected, exact.items);

    // Out-of-range offset returns empty page with has_more_in_scan = false.
    let past_end = detect_missing_names(&project, MissingNameScanRequest::unbounded(100, 10));
    assert!(past_end.items.is_empty());
    assert!(!past_end.has_more_in_scan);
    assert_eq!(past_end.counts.total_candidates, Some(21));
}

#[test]
fn detection_is_deterministic_across_repeated_runs_and_input_permutations() {
    let project_a = full_blank_project();
    let mut project_b = full_blank_project();
    // Reverse orderings in project_b to ensure output is order-independent.
    project_b.installations[0].buildings.reverse();
    project_b.installations[0].group_ranges.reverse();

    let run1 = detect_missing_names(&project_a, MissingNameScanRequest::unbounded(0, 100));
    let run2 = detect_missing_names(&project_a, MissingNameScanRequest::unbounded(0, 100));
    let run_b = detect_missing_names(&project_b, MissingNameScanRequest::unbounded(0, 100));

    assert_eq!(run1, run2);
    assert_eq!(run1, run_b);
}

#[test]
fn provenance_model_distinguishes_origins_sources_and_gaps_and_roundtrips_json() {
    let auto = NameProvenance::automatic(
        vec![EvidenceContribution {
            kind: EvidenceSourceKind::BuildingLocationPath,
            source_target: Some(NameTargetId::BuildingPart(3)),
            value: "EG / Living Room".into(),
        }],
        vec![EvidenceGap::Missing {
            kind: EvidenceSourceKind::ProductDesignation,
            detail: "no product designation available".into(),
        }],
    );
    let manual_req = NameProvenance::manual_required(vec![
        EvidenceGap::Ambiguous {
            kind: EvidenceSourceKind::FunctionContext,
            detail: "conflicting function contexts".into(),
            candidates: vec!["Lighting".into(), "Shading".into()],
        },
        EvidenceGap::Insufficient {
            kind: EvidenceSourceKind::ComObjectText,
            detail: "DPT alone is insufficient".into(),
        },
        EvidenceGap::UnselectedParent {
            parent: NameTargetId::BuildingPart(1),
        },
    ]);
    let manual_edited = NameProvenance::manual(true);
    let manual_fresh = NameProvenance::manual(false);

    assert_eq!(auto.origin, NameOrigin::Automatic);
    assert!(!auto.derived_from_automatic);
    assert_eq!(manual_req.origin, NameOrigin::ManualRequired);
    assert_eq!(manual_edited.origin, NameOrigin::Manual);
    assert!(manual_edited.derived_from_automatic);
    assert!(!manual_fresh.derived_from_automatic);

    for prov in [auto, manual_req, manual_edited, manual_fresh] {
        let json = serde_json::to_string(&prov).unwrap();
        let decoded: NameProvenance = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, prov);
    }
}
