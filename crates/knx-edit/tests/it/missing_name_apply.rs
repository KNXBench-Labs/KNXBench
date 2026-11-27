//! Verifies missing-name invalidation, preview, confirmation, duplicate warnings, and apply.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use knx_core::commissioning::{CommissioningState, CompletionStatus};
use knx_core::function::FunctionError;
use knx_core::provenance::Override;
use knx_core::*;
use knx_edit::missing_name_apply::*;
use knx_edit::missing_name_recipes::{MissingNameEvidence, TextAssociation};
use knx_edit::missing_names::{NameOrigin, NameTargetId};
use knx_edit::name_recipe::{NameRecipeNativeTarget, NameRecipeRevision};
use knx_store::project_history::{self as history, NativeSnapshot};

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

struct TempDbDir {
    path: PathBuf,
}

impl TempDbDir {
    fn new(label: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "knx-edit-missing-name-apply-{label}-{}-{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn db_path(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TempDbDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn src(id: &str) -> SourceRef {
    SourceRef {
        path: "synthetic.xml".into(),
        ets_id: id.into(),
    }
}

fn rev() -> NameRecipeRevision {
    NameRecipeRevision {
        project_incarnation: "incarnation-wp51-2b".into(),
        generation: 7,
    }
}

fn part(id: u32, name: &str, kind: BuildingPartType, parent: Option<u32>) -> BuildingPart {
    BuildingPart {
        id: BuildingPartId(id),
        source: src(&format!("BP-{id}")),
        name: name.into(),
        number: Some(format!("{id}")),
        kind,
        default_line: None,
        completion: CompletionStatus::Undefined,
        children: vec![],
        devices: vec![],
        parent: parent.map(BuildingPartId),
    }
}

fn snap(project: Project) -> NativeSnapshot {
    NativeSnapshot {
        project,
        opaque: vec![knx_store::StoredOpaqueEntry {
            source_path: "extra.xml".into(),
            xpath: "/extra".into(),
            kind: "element".into(),
            name: "Preserved".into(),
            bytes: b"unchanged-opaque".to_vec(),
            sha256: "opaque-sha".into(),
        }],
        manufacturer_refs: vec![knx_store::ManufacturerRef {
            source_path: "M-0001/Hardware.xml".into(),
            sha256: "mfg-sha".into(),
            len: 42,
            kind: "ManufacturerData".into(),
        }],
    }
}

fn full_q9_fixture() -> NativeSnapshot {
    let mut p = Project::new(Language("en".into()));
    p.info.name = "   ".into(); // whitespace-only blank project name

    // All 11 BuildingPartType kinds, nested under Building(1) -> Floor(2) -> Room(3), etc.
    let mut buildings = Vec::new();
    for (idx, kind) in ALL_BUILDING_KINDS.iter().enumerate() {
        let id = (idx + 1) as u32;
        let parent = if id == 1 { None } else { Some(1) };
        let blank = match idx % 3 {
            0 => "",
            1 => "   ",
            _ => "\t",
        };
        let mut bp = part(id, blank, *kind, parent);
        if id == 1 {
            bp.children = (2..=11).map(BuildingPartId).collect();
        }
        if id == 3 {
            bp.devices.push(DeviceId(1));
        }
        buildings.push(bp);
    }

    p.installations.push(Installation {
        id: InstallationId(0),
        name: "\t".into(),
        default_line: Some(LineId(1)),
        multicast_address: None,
        completion: CompletionStatus::Undefined,
        buildings,
        topology: Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: src("A-1"),
                name: "".into(),
                address: 1,
                completion: CompletionStatus::Undefined,
                lines: vec![LineId(1)],
            }],
            lines: vec![Line {
                id: LineId(1),
                source: src("L-1"),
                name: "  ".into(),
                address: 2,
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
        group_ranges: vec![
            GroupRange {
                id: GroupRangeId(1),
                source: src("GR-1"),
                name: "".into(),
                start: GroupAddress::from_raw(2048),
                end: GroupAddress::from_raw(4095),
                parent: None,
                children: vec![GroupRangeId(2)],
            },
            GroupRange {
                id: GroupRangeId(2),
                source: src("GR-2"),
                name: " \t ".into(),
                start: GroupAddress::from_raw(2048),
                end: GroupAddress::from_raw(2303),
                parent: Some(GroupRangeId(1)),
                children: vec![],
            },
        ],
        group_addresses: vec![GroupAddressEntry {
            id: GroupAddressId(1),
            source: src("GA-1"),
            name: "   ".into(),
            address: GroupAddress::from_raw(2049),
            central: true,
            unfiltered: false,
            range: Some(GroupRangeId(2)),
            declared_dpt: Override::Absent,
            description: Some("Keep this description".into()),
        }],
        parameters: vec![],
    });

    p.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: src("D-1"),
        name: "".into(),
        description: Some("Actuator description".into()),
        address: Some(IndividualAddress::new(1, 2, 3).unwrap()),
        product_ref: "M-TEST_P-1".into(),
        program_ref: "M-TEST_HP-1".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![ComObjectInstanceId(1)],
        binary_data: vec![],
    });
    p.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: src("CO-1"),
        device: DeviceId(1),
        number: 1,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::default(),
        size: None,
        is_active: true,
        links: vec![GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        }],
        module_instance: None,
        priority_edit: None,
        saved_instance_priority: SavedInstancePriority::default(),
    });

    p.functions.push(Function {
        id: FunctionId(1),
        source: src("F-1"),
        location: BuildingPartId(3),
        details: FunctionDetails {
            name: "\t  ".into(),
            description: Some("Light function".into()),
            ..Default::default()
        },
        members: vec![FunctionMember {
            id: FunctionMemberId(1),
            source: src("FM-1"),
            group_address: GroupAddressId(1),
            name: "".into(),
            role: Some("SwitchOnOff".into()),
        }],
    });
    p.ids = IdAllocators::from_counts(1, 1, 1, 1, 2, 1, 11, 0, 0).with_function_counts(1, 1);

    snap(p)
}

#[test]
fn all_nine_q9_categories_and_eleven_building_kinds_apply_undo_redo_and_preserve_other_fields() {
    let mut state = full_q9_fixture();
    let original = state.clone();
    let mut stack = CommandStack::new();
    let revision = rev();

    // Deliberately pass entries in reverse/child-before-parent order to verify Q16 ordering.
    let mut entries = vec![
        MissingNameEntry {
            target: NameTargetId::GroupAddress(1),
            name: "Main Light Switch".into(),
        },
        MissingNameEntry {
            target: NameTargetId::Function(1),
            name: "Ceiling Light".into(),
        },
        MissingNameEntry {
            target: NameTargetId::Device(1),
            name: "Relay 1".into(),
        },
        MissingNameEntry {
            target: NameTargetId::GroupRange(2),
            name: "Switching".into(),
        },
        MissingNameEntry {
            target: NameTargetId::GroupRange(1),
            name: "Lighting".into(),
        },
        MissingNameEntry {
            target: NameTargetId::Line(1),
            name: "Line 1.2".into(),
        },
        MissingNameEntry {
            target: NameTargetId::Area(1),
            name: "Area 1".into(),
        },
    ];
    for id in (1..=11u32).rev() {
        entries.push(MissingNameEntry {
            target: NameTargetId::BuildingPart(id),
            name: format!("Space {id}"),
        });
    }
    entries.push(MissingNameEntry {
        target: NameTargetId::Installation(0),
        name: "Main Installation".into(),
    });
    entries.push(MissingNameEntry {
        target: NameTargetId::Project,
        name: "Office Building".into(),
    });

    let plan = preview_missing_name_apply(&state, &revision, MissingNameApplyIntent { entries })
        .expect("preview must succeed");
    assert_eq!(plan.preview().version, 1);
    assert_eq!(plan.preview().changed, 20);
    assert!(plan.preview().warnings.is_empty());

    // Verify deterministic parent-before-child ordering in preview rows.
    let targets: Vec<NameTargetId> = plan.preview().rows.iter().map(|r| r.target).collect();
    let pos = |t: NameTargetId| targets.iter().position(|x| *x == t).unwrap();
    assert!(pos(NameTargetId::Project) < pos(NameTargetId::Installation(0)));
    assert!(pos(NameTargetId::Installation(0)) < pos(NameTargetId::BuildingPart(1)));
    assert!(pos(NameTargetId::BuildingPart(1)) < pos(NameTargetId::BuildingPart(3)));
    assert!(pos(NameTargetId::Area(1)) < pos(NameTargetId::Line(1)));
    assert!(pos(NameTargetId::Line(1)) < pos(NameTargetId::Device(1)));
    assert!(pos(NameTargetId::BuildingPart(3)) < pos(NameTargetId::Device(1)));
    assert!(pos(NameTargetId::BuildingPart(3)) < pos(NameTargetId::Function(1)));
    assert!(pos(NameTargetId::GroupRange(1)) < pos(NameTargetId::GroupRange(2)));
    assert!(pos(NameTargetId::GroupRange(2)) < pos(NameTargetId::GroupAddress(1)));
    assert!(pos(NameTargetId::Function(1)) < pos(NameTargetId::GroupAddress(1)));
    assert!(pos(NameTargetId::Device(1)) < pos(NameTargetId::GroupAddress(1)));

    let token = plan.confirmation_token("test-authority").unwrap();
    let outcome = apply_missing_name_apply(
        &mut state,
        &mut stack,
        &revision,
        &plan,
        "test-authority",
        &token,
        None,
    )
    .expect("session-only apply must succeed");
    assert!(outcome.changed);
    assert_eq!(outcome.native_generation, None);

    // Verify all 9 categories updated.
    assert_eq!(state.project.info.name, "Office Building");
    assert_eq!(state.project.installations[0].name, "Main Installation");
    for id in 1..=11u32 {
        let bp = state.project.installations[0]
            .buildings
            .iter()
            .find(|b| b.id.0 == id)
            .unwrap();
        assert_eq!(bp.name, format!("Space {id}"));
    }
    assert_eq!(
        state.project.installations[0].topology.areas[0].name,
        "Area 1"
    );
    assert_eq!(
        state.project.installations[0].topology.lines[0].name,
        "Line 1.2"
    );
    assert_eq!(
        state.project.devices.get(DeviceId(1)).unwrap().name,
        "Relay 1"
    );
    assert_eq!(
        state.project.installations[0].group_ranges[0].name,
        "Lighting"
    );
    assert_eq!(
        state.project.installations[0].group_ranges[1].name,
        "Switching"
    );
    assert_eq!(
        state.project.installations[0].group_addresses[0].name,
        "Main Light Switch"
    );
    assert_eq!(state.project.functions[0].details.name, "Ceiling Light");

    // Verify non-name fields and opaque/manifest data are untouched.
    assert_eq!(state.opaque, original.opaque);
    assert_eq!(state.manufacturer_refs, original.manufacturer_refs);
    assert_eq!(
        state.project.installations[0].group_addresses[0].address,
        original.project.installations[0].group_addresses[0].address
    );
    assert_eq!(
        state.project.installations[0].group_addresses[0].central,
        original.project.installations[0].group_addresses[0].central
    );
    assert_eq!(
        state.project.installations[0].group_addresses[0].description,
        original.project.installations[0].group_addresses[0].description
    );
    assert_eq!(
        state.project.devices.get(DeviceId(1)).unwrap().address,
        original.project.devices.get(DeviceId(1)).unwrap().address
    );
    assert_eq!(
        state.project.devices.get(DeviceId(1)).unwrap().product_ref,
        original
            .project
            .devices
            .get(DeviceId(1))
            .unwrap()
            .product_ref
    );
    assert_eq!(
        state.project.functions[0].members,
        original.project.functions[0].members
    );

    // Single-step Undo restores exact original blank/whitespace strings.
    let applied = state.clone();
    stack.undo(&mut state.project).unwrap();
    assert_eq!(state, original);
    assert!(!stack.can_undo());

    // Single-step Redo reapplies all 20 names cleanly.
    stack.redo(&mut state.project).unwrap();
    assert_eq!(state, applied);
}

#[test]
fn q5_refuses_existing_non_blank_names_including_bom() {
    let mut state = full_q9_fixture();
    state.project.info.name = "Existing Project".into();
    state.project.devices.get_mut(DeviceId(1)).unwrap().name = "\u{feff}".into();
    let before = state.clone();

    let err_regular = preview_missing_name_apply(
        &state,
        &rev(),
        MissingNameApplyIntent {
            entries: vec![MissingNameEntry {
                target: NameTargetId::Project,
                name: "Overwrite Attempt".into(),
            }],
        },
    )
    .unwrap_err();
    assert_eq!(
        err_regular,
        MissingNameApplyError::TargetNotBlank(NameTargetId::Project)
    );

    let err_bom = preview_missing_name_apply(
        &state,
        &rev(),
        MissingNameApplyIntent {
            entries: vec![MissingNameEntry {
                target: NameTargetId::Device(1),
                name: "Overwrite BOM".into(),
            }],
        },
    )
    .unwrap_err();
    assert_eq!(
        err_bom,
        MissingNameApplyError::TargetNotBlank(NameTargetId::Device(1))
    );
    assert_eq!(state, before);
}

#[test]
fn identity_bounds_and_name_admission_fail_closed() {
    let mut state = full_q9_fixture();
    // Add a duplicate GroupAddressId(99) and a reserved FunctionId(0).
    let dup_ga = GroupAddressEntry {
        id: GroupAddressId(99),
        source: src("GA-99"),
        name: "".into(),
        address: GroupAddress::from_raw(2050),
        central: false,
        unfiltered: false,
        range: None,
        declared_dpt: Override::Absent,
        description: None,
    };
    state.project.installations[0]
        .group_addresses
        .push(dup_ga.clone());
    state.project.installations[0].group_addresses.push(dup_ga);
    state.project.functions.push(Function {
        id: FunctionId(0),
        source: src("F-0"),
        location: BuildingPartId(3),
        details: FunctionDetails::default(),
        members: vec![],
    });

    // Invalid revision (empty incarnation, >128 bytes, generation > MAX_SAFE_INTEGER)
    for bad_rev in [
        NameRecipeRevision {
            project_incarnation: "".into(),
            generation: 1,
        },
        NameRecipeRevision {
            project_incarnation: "a".repeat(129),
            generation: 1,
        },
        NameRecipeRevision {
            project_incarnation: "ok".into(),
            generation: 9_007_199_254_740_992,
        },
    ] {
        let err = preview_missing_name_apply(
            &state,
            &bad_rev,
            MissingNameApplyIntent {
                entries: vec![MissingNameEntry {
                    target: NameTargetId::Project,
                    name: "Valid".into(),
                }],
            },
        )
        .unwrap_err();
        assert_eq!(err, MissingNameApplyError::InvalidRevision);
    }

    // Empty selection
    assert_eq!(
        preview_missing_name_apply(&state, &rev(), MissingNameApplyIntent { entries: vec![] })
            .unwrap_err(),
        MissingNameApplyError::InvalidSelection
    );

    // >256 selection
    let too_many: Vec<_> = (1..=257)
        .map(|i| MissingNameEntry {
            target: NameTargetId::GroupAddress(i),
            name: format!("GA {i}"),
        })
        .collect();
    assert_eq!(
        preview_missing_name_apply(&state, &rev(), MissingNameApplyIntent { entries: too_many })
            .unwrap_err(),
        MissingNameApplyError::InvalidSelection
    );

    // Duplicate target in selection
    assert_eq!(
        preview_missing_name_apply(
            &state,
            &rev(),
            MissingNameApplyIntent {
                entries: vec![
                    MissingNameEntry {
                        target: NameTargetId::Project,
                        name: "A".into()
                    },
                    MissingNameEntry {
                        target: NameTargetId::Project,
                        name: "B".into()
                    },
                ],
            },
        )
        .unwrap_err(),
        MissingNameApplyError::DuplicateSelection(NameTargetId::Project)
    );

    // Missing target
    assert_eq!(
        preview_missing_name_apply(
            &state,
            &rev(),
            MissingNameApplyIntent {
                entries: vec![MissingNameEntry {
                    target: NameTargetId::Device(999),
                    name: "Ghost".into(),
                }],
            },
        )
        .unwrap_err(),
        MissingNameApplyError::MissingTarget(NameTargetId::Device(999))
    );

    // Ambiguous duplicate ID
    assert_eq!(
        preview_missing_name_apply(
            &state,
            &rev(),
            MissingNameApplyIntent {
                entries: vec![MissingNameEntry {
                    target: NameTargetId::GroupAddress(99),
                    name: "Ambiguous".into(),
                }],
            },
        )
        .unwrap_err(),
        MissingNameApplyError::AmbiguousTarget(NameTargetId::GroupAddress(99))
    );

    // Reserved FunctionId(0)
    assert_eq!(
        preview_missing_name_apply(
            &state,
            &rev(),
            MissingNameApplyIntent {
                entries: vec![MissingNameEntry {
                    target: NameTargetId::Function(0),
                    name: "Reserved".into(),
                }],
            },
        )
        .unwrap_err(),
        MissingNameApplyError::ReservedTarget(NameTargetId::Function(0))
    );

    // Invalid names: blank, control char, >1024 chars, Function >255 chars
    assert_eq!(
        preview_missing_name_apply(
            &state,
            &rev(),
            MissingNameApplyIntent {
                entries: vec![MissingNameEntry {
                    target: NameTargetId::Area(1),
                    name: "   ".into(),
                }],
            },
        )
        .unwrap_err(),
        MissingNameApplyError::InvalidName(
            NameTargetId::Area(1),
            knx_core::names::NameError::Blank
        )
    );
    assert_eq!(
        preview_missing_name_apply(
            &state,
            &rev(),
            MissingNameApplyIntent {
                entries: vec![MissingNameEntry {
                    target: NameTargetId::Area(1),
                    name: "Bad\nName".into(),
                }],
            },
        )
        .unwrap_err(),
        MissingNameApplyError::InvalidName(
            NameTargetId::Area(1),
            knx_core::names::NameError::ControlCharacter
        )
    );
    assert_eq!(
        preview_missing_name_apply(
            &state,
            &rev(),
            MissingNameApplyIntent {
                entries: vec![MissingNameEntry {
                    target: NameTargetId::Area(1),
                    name: "x".repeat(1025),
                }],
            },
        )
        .unwrap_err(),
        MissingNameApplyError::InvalidName(
            NameTargetId::Area(1),
            knx_core::names::NameError::TooLong
        )
    );
    assert_eq!(
        preview_missing_name_apply(
            &state,
            &rev(),
            MissingNameApplyIntent {
                entries: vec![MissingNameEntry {
                    target: NameTargetId::Function(1),
                    name: "f".repeat(256),
                }],
            },
        )
        .unwrap_err(),
        MissingNameApplyError::InvalidFunctionName(
            NameTargetId::Function(1),
            FunctionError::TooLong("name")
        )
    );

    // Byte budget exceeded (>1 MiB across <=256 entries, e.g. 256 * 1024 4-byte emoji = 1_048_576 + current)
    let mut big_state = full_q9_fixture();
    big_state.project.installations[0].group_addresses.clear();
    let mut budget_entries = Vec::new();
    let four_byte_name = "🛠".repeat(1024); // 4096 bytes, 1024 codepoints (valid name)
    for i in 1..=256u32 {
        big_state.project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(i),
                source: src(&format!("GA-{i}")),
                name: " ".into(), // +1 byte each -> 256 * 4097 > 1_048_576
                address: GroupAddress::from_raw((2048 + i) as u16),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Override::Absent,
                description: None,
            });
        budget_entries.push(MissingNameEntry {
            target: NameTargetId::GroupAddress(i),
            name: four_byte_name.clone(),
        });
    }
    assert_eq!(
        preview_missing_name_apply(
            &big_state,
            &rev(),
            MissingNameApplyIntent {
                entries: budget_entries,
            },
        )
        .unwrap_err(),
        MissingNameApplyError::Budget
    );
}

#[test]
fn q13_duplicate_name_warnings_allow_exact_names_without_auto_suffix() {
    let mut state = full_q9_fixture();
    // Add existing named sibling GA(10) and two blank sibling GAs (11, 12) in GroupRange(2).
    state.project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id: GroupAddressId(10),
            source: src("GA-10"),
            name: "Ceiling Light".into(),
            address: GroupAddress::from_raw(2050),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(2)),
            declared_dpt: Override::Absent,
            description: None,
        });
    state.project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id: GroupAddressId(11),
            source: src("GA-11"),
            name: "".into(),
            address: GroupAddress::from_raw(2051),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(2)),
            declared_dpt: Override::Absent,
            description: None,
        });
    state.project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id: GroupAddressId(12),
            source: src("GA-12"),
            name: "".into(),
            address: GroupAddress::from_raw(2052),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(2)),
            declared_dpt: Override::Absent,
            description: None,
        });
    state.project.ids.raise_to(
        &IdAllocators::from_counts(1, 1, 1, 1, 2, 12, 11, 0, 0).with_function_counts(1, 1),
    );

    let plan = preview_missing_name_apply(
        &state,
        &rev(),
        MissingNameApplyIntent {
            entries: vec![
                // Collides with existing sibling GA(10)
                MissingNameEntry {
                    target: NameTargetId::GroupAddress(1),
                    name: "Ceiling Light".into(),
                },
                // Cross-page / same-batch duplicate between GA(11) and GA(12)
                MissingNameEntry {
                    target: NameTargetId::GroupAddress(11),
                    name: "Shared Label".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::GroupAddress(12),
                    name: "Shared Label".into(),
                },
            ],
        },
    )
    .expect("Q13 duplicates must be allowed with warnings");

    let warnings = &plan.preview().warnings;
    assert_eq!(warnings.len(), 3);

    let w_existing = warnings
        .iter()
        .find(|w| w.target == NameTargetId::GroupAddress(1))
        .unwrap();
    assert_eq!(w_existing.proposed_name, "Ceiling Light");
    assert_eq!(
        w_existing.conflicts_with,
        vec![DuplicateConflictSource::ExistingSibling(
            NameTargetId::GroupAddress(10)
        )]
    );

    let w_11 = warnings
        .iter()
        .find(|w| w.target == NameTargetId::GroupAddress(11))
        .unwrap();
    assert_eq!(
        w_11.conflicts_with,
        vec![DuplicateConflictSource::BatchSibling(
            NameTargetId::GroupAddress(12)
        )]
    );
    let w_12 = warnings
        .iter()
        .find(|w| w.target == NameTargetId::GroupAddress(12))
        .unwrap();
    assert_eq!(
        w_12.conflicts_with,
        vec![DuplicateConflictSource::BatchSibling(
            NameTargetId::GroupAddress(11)
        )]
    );

    // Apply writes exact names without auto-suffixing.
    let token = plan.confirmation_token("dup-test").unwrap();
    let mut stack = CommandStack::new();
    apply_missing_name_apply(
        &mut state,
        &mut stack,
        &rev(),
        &plan,
        "dup-test",
        &token,
        None,
    )
    .unwrap();
    let ga_name = |id: u32| {
        state.project.installations[0]
            .group_addresses
            .iter()
            .find(|g| g.id.0 == id)
            .unwrap()
            .name
            .as_str()
    };
    assert_eq!(ga_name(1), "Ceiling Light");
    assert_eq!(ga_name(11), "Shared Label");
    assert_eq!(ga_name(12), "Shared Label");
}

#[test]
fn q16_parent_invalidation_and_reevaluation_preserve_snapshot_and_provenance() {
    let state = full_q9_fixture();
    let before = state.clone();

    // Area -> Line -> Device -> GroupAddress chain
    let area_deps = invalidated_dependents(&state.project, &[NameTargetId::Area(1)]);
    assert_eq!(
        area_deps,
        vec![
            NameTargetId::Line(1),
            NameTargetId::Device(1),
            NameTargetId::GroupAddress(1),
        ]
    );

    // GroupRange(1) -> GroupRange(2) -> GroupAddress(1) chain
    let gr_deps = invalidated_dependents(&state.project, &[NameTargetId::GroupRange(1)]);
    assert_eq!(
        gr_deps,
        vec![NameTargetId::GroupRange(2), NameTargetId::GroupAddress(1)]
    );

    // BuildingPart(1) -> BuildingPart(3) -> Device(1), Function(1) -> GroupAddress(1)
    let evidence = vec![MissingNameEvidence::ResolvedProduct {
        device: DeviceId(1),
        product_ref: "M-TEST_P-1".into(),
        program_ref: "M-TEST_HP-1".into(),
        product_text: "Switch Actuator".into(),
        association: TextAssociation::Confirmed,
    }];
    let reeval = reevaluate_affected_proposals(
        &state,
        &evidence,
        &[
            NameTargetId::BuildingPart(1),
            NameTargetId::BuildingPart(3),
            NameTargetId::Function(1),
        ],
        &[
            MissingNameEntry {
                target: NameTargetId::BuildingPart(1),
                name: "House".into(),
            },
            MissingNameEntry {
                target: NameTargetId::BuildingPart(3),
                name: "Kitchen".into(),
            },
            MissingNameEntry {
                target: NameTargetId::Function(1),
                name: "Light".into(),
            },
        ],
    )
    .expect("reevaluation must succeed");

    // Underlying snapshot must be completely untouched.
    assert_eq!(state, before);

    // Device(1) and GroupAddress(1) both recompute from draft parent names with provenance.
    let dev_proposal = reeval
        .proposals
        .iter()
        .find(|p| p.candidate.target == NameTargetId::Device(1))
        .expect("device proposal present");
    assert_eq!(
        dev_proposal.proposed_name.as_deref(),
        Some("House / Kitchen – Switch Actuator")
    );
    assert_eq!(dev_proposal.provenance.origin, NameOrigin::Automatic);
    assert!(!dev_proposal.provenance.sources.is_empty());

    let ga_proposal = reeval
        .proposals
        .iter()
        .find(|p| p.candidate.target == NameTargetId::GroupAddress(1))
        .expect("group address proposal present");
    assert_eq!(
        ga_proposal.proposed_name.as_deref(),
        Some("House / Kitchen – Light – SwitchOnOff")
    );
    assert_eq!(ga_proposal.provenance.origin, NameOrigin::Automatic);
    assert!(!ga_proposal.provenance.sources.is_empty());

    // Proposals are ordered parent-before-child (Device before GroupAddress).
    let dev_pos = reeval
        .proposals
        .iter()
        .position(|p| p.candidate.target == NameTargetId::Device(1))
        .unwrap();
    let ga_pos = reeval
        .proposals
        .iter()
        .position(|p| p.candidate.target == NameTargetId::GroupAddress(1))
        .unwrap();
    assert!(dev_pos < ga_pos);
}

#[test]
fn stale_snapshot_revision_and_confirmation_token_mismatches_refuse_with_zero_mutation() {
    let mut state = full_q9_fixture();
    let mut stack = CommandStack::new();
    let revision = rev();
    let plan = preview_missing_name_apply(
        &state,
        &revision,
        MissingNameApplyIntent {
            entries: vec![MissingNameEntry {
                target: NameTargetId::Project,
                name: "Named Project".into(),
            }],
        },
    )
    .unwrap();
    let token = plan.confirmation_token("auth-a").unwrap();

    // Invalid authority rejected
    assert_eq!(
        plan.confirmation_token("").unwrap_err(),
        MissingNameApplyError::InvalidAuthority
    );
    assert_eq!(
        plan.confirmation_token(&"a".repeat(257)).unwrap_err(),
        MissingNameApplyError::InvalidAuthority
    );

    // Wrong confirmation token
    let snapshot_before = state.clone();
    let stack_before = stack.clone();
    assert_eq!(
        apply_missing_name_apply(
            &mut state,
            &mut stack,
            &revision,
            &plan,
            "auth-a",
            "wrong-token",
            None,
        )
        .unwrap_err(),
        MissingNameApplyError::InvalidConfirmation
    );
    assert_eq!(state, snapshot_before);
    assert_eq!(stack, stack_before);

    // Wrong authority for the same token
    assert_eq!(
        apply_missing_name_apply(&mut state, &mut stack, &revision, &plan, "auth-b", &token, None,)
            .unwrap_err(),
        MissingNameApplyError::InvalidConfirmation
    );
    assert_eq!(state, snapshot_before);
    assert_eq!(stack, stack_before);

    // Revision mismatch
    let stale_rev = NameRecipeRevision {
        project_incarnation: revision.project_incarnation.clone(),
        generation: revision.generation + 1,
    };
    assert_eq!(
        apply_missing_name_apply(
            &mut state, &mut stack, &stale_rev, &plan, "auth-a", &token, None,
        )
        .unwrap_err(),
        MissingNameApplyError::Conflict
    );
    assert_eq!(state, snapshot_before);
    assert_eq!(stack, stack_before);

    // Stale snapshot mismatch
    state.project.info.project_number = Some("concurrent edit".into());
    let mutated_snapshot = state.clone();
    assert_eq!(
        apply_missing_name_apply(&mut state, &mut stack, &revision, &plan, "auth-a", &token, None,)
            .unwrap_err(),
        MissingNameApplyError::Conflict
    );
    assert_eq!(state, mutated_snapshot);
    assert_eq!(stack, stack_before);
}

#[test]
fn q14_native_durable_write_before_publish_roundtrip_and_stale_generation_refusal() {
    let dir = TempDbDir::new("native-roundtrip");
    let db_path = dir.db_path("project.knxdb");

    let mut state = full_q9_fixture();
    let original = state.clone();
    let mut stack = CommandStack::new();
    let conn = knx_store::open_and_migrate(&db_path).unwrap();
    let gen0 = history::save_editor(&conn, &state, &stack, None, true).unwrap();
    drop(conn);

    let revision = rev();
    let plan = preview_missing_name_apply(
        &state,
        &revision,
        MissingNameApplyIntent {
            entries: vec![
                MissingNameEntry {
                    target: NameTargetId::Project,
                    name: "Persisted Project".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::Installation(0),
                    name: "Persisted Inst".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::GroupAddress(1),
                    name: "Persisted GA".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::Function(1),
                    name: "Persisted Function".into(),
                },
            ],
        },
    )
    .unwrap();
    let token = plan.confirmation_token("native-auth").unwrap();

    // 1. Stale history_generation must map to Conflict and leave memory untouched.
    let err = apply_missing_name_apply(
        &mut state,
        &mut stack,
        &revision,
        &plan,
        "native-auth",
        &token,
        Some(NameRecipeNativeTarget {
            path: &db_path,
            history_generation: gen0 + 99,
        }),
    )
    .unwrap_err();
    assert_eq!(err, MissingNameApplyError::Conflict);
    assert_eq!(state, original);
    assert!(!stack.can_undo());

    // 2. Real durable apply writes to SQLite before publishing.
    let outcome = apply_missing_name_apply(
        &mut state,
        &mut stack,
        &revision,
        &plan,
        "native-auth",
        &token,
        Some(NameRecipeNativeTarget {
            path: &db_path,
            history_generation: gen0,
        }),
    )
    .expect("durable apply must succeed");
    assert!(outcome.changed);
    let gen1 = outcome
        .native_generation
        .expect("native generation returned");
    assert!(gen1 > gen0);

    // Reopen from disk and verify persisted names and undo stack.
    let conn = history::open_editor_store(&db_path, false).unwrap();
    let mut loaded = history::load_editor(&conn)
        .unwrap()
        .expect("editor history present");
    assert_eq!(loaded.working.project.info.name, "Persisted Project");
    assert_eq!(
        loaded.working.project.installations[0].name,
        "Persisted Inst"
    );
    assert_eq!(
        loaded.working.project.installations[0].group_addresses[0].name,
        "Persisted GA"
    );
    assert_eq!(
        loaded.working.project.functions[0].details.name,
        "Persisted Function"
    );

    // Undo -> save -> reopen -> verify exact original blank/whitespace strings restored.
    let mut loaded_stack = CommandStack::from_snapshot_states(loaded.undo, loaded.redo);
    loaded_stack.undo(&mut loaded.working.project).unwrap();
    let _gen2 = history::save_editor(&conn, &loaded.working, &loaded_stack, None, false).unwrap();
    drop(conn);

    let conn = history::open_editor_store(&db_path, false).unwrap();
    let after_undo = history::load_editor(&conn)
        .unwrap()
        .expect("history after undo");
    assert_eq!(after_undo.working.project.info.name, "   ");
    assert_eq!(after_undo.working.project.installations[0].name, "\t");
    assert_eq!(
        after_undo.working.project.installations[0].group_addresses[0].name,
        "   "
    );
    assert_eq!(after_undo.working.project.functions[0].details.name, "\t  ");
    assert_eq!(after_undo.working, original);
}

#[test]
fn q14_native_concurrent_sqlite_writes_map_to_conflict_and_corrupt_store_to_persistence() {
    // 1. Concurrent SQLite editor write advancing history_generation -> Conflict.
    let dir = TempDbDir::new("native-conflicts");
    let db_gen = dir.db_path("concurrent-gen.knxdb");
    let mut state = full_q9_fixture();
    let original = state.clone();
    let mut stack = CommandStack::new();
    let conn = knx_store::open_and_migrate(&db_gen).unwrap();
    let gen0 = history::save_editor(&conn, &state, &stack, None, true).unwrap();

    let revision = rev();
    let plan = preview_missing_name_apply(
        &state,
        &revision,
        MissingNameApplyIntent {
            entries: vec![MissingNameEntry {
                target: NameTargetId::Project,
                name: "Apply After Concurrent".into(),
            }],
        },
    )
    .unwrap();
    let token = plan.confirmation_token("conflict-auth").unwrap();

    // Another writer advances the history generation on disk.
    let mut other_state = state.clone();
    let mut other_stack = CommandStack::new();
    other_stack
        .do_command(
            &mut other_state.project,
            Command::RenameArea {
                id: AreaId(1),
                name: "Concurrent Area".into(),
            },
        )
        .unwrap();
    let gen1 = history::save_editor(&conn, &other_state, &other_stack, Some(gen0), false).unwrap();
    assert!(gen1 > gen0);
    drop(conn);

    let err = apply_missing_name_apply(
        &mut state,
        &mut stack,
        &revision,
        &plan,
        "conflict-auth",
        &token,
        Some(NameRecipeNativeTarget {
            path: &db_gen,
            history_generation: gen0,
        }),
    )
    .unwrap_err();
    assert_eq!(err, MissingNameApplyError::Conflict);
    assert_eq!(state, original);
    assert!(!stack.can_undo());

    // 2. Concurrent SQLite generation-0 root save causing working snapshot hash drift -> Conflict.
    let db_gen0 = dir.db_path("concurrent-gen0.knxdb");
    let conn0 = knx_store::open_and_migrate(&db_gen0).unwrap();
    knx_store::save_project_with_passthrough(
        &conn0,
        &state.project,
        &state.opaque,
        &state.manufacturer_refs,
    )
    .unwrap();
    let mut drifted = state.clone();
    drifted.project.info.project_number = Some("concurrent-root-save".into());
    knx_store::save_project_with_passthrough(
        &conn0,
        &drifted.project,
        &drifted.opaque,
        &drifted.manufacturer_refs,
    )
    .unwrap();
    drop(conn0);

    let err_hash = apply_missing_name_apply(
        &mut state,
        &mut stack,
        &revision,
        &plan,
        "conflict-auth",
        &token,
        Some(NameRecipeNativeTarget {
            path: &db_gen0,
            history_generation: 0,
        }),
    )
    .unwrap_err();
    assert_eq!(err_hash, MissingNameApplyError::Conflict);
    assert_eq!(state, original);
    assert!(!stack.can_undo());

    // 3. Genuine persistence error (corrupted SQLite file) -> Persistence(_).
    let db_corrupt = dir.db_path("corrupt.knxdb");
    std::fs::write(&db_corrupt, b"not a sqlite database at all").unwrap();
    let err_persist = apply_missing_name_apply(
        &mut state,
        &mut stack,
        &revision,
        &plan,
        "conflict-auth",
        &token,
        Some(NameRecipeNativeTarget {
            path: &db_corrupt,
            history_generation: 0,
        }),
    )
    .unwrap_err();
    assert!(matches!(err_persist, MissingNameApplyError::Persistence(_)));
    assert_eq!(state, original);
    assert!(!stack.can_undo());
}

#[test]
fn q16_parent_selection_edit_deselection_and_validation_failures() {
    let state = full_q9_fixture();
    let before = state.clone();
    let evidence = vec![MissingNameEvidence::ResolvedProduct {
        device: DeviceId(1),
        product_ref: "M-TEST_P-1".into(),
        program_ref: "M-TEST_HP-1".into(),
        product_text: "Switch Actuator".into(),
        association: TextAssociation::Confirmed,
    }];

    // Empty changed_parents with valid draft_parents returns empty results without mutating.
    let empty_changed = reevaluate_affected_proposals(
        &state,
        &evidence,
        &[],
        &[MissingNameEntry {
            target: NameTargetId::BuildingPart(1),
            name: "House".into(),
        }],
    )
    .expect("empty changed_parents with valid draft_parents succeeds");
    assert!(empty_changed.invalidated_targets.is_empty());
    assert!(empty_changed.proposals.is_empty());
    assert_eq!(state, before);

    // 1. Selection: BuildingPart(1) + BuildingPart(3) selected -> Device(1) gets location path.
    let after_select = reevaluate_affected_proposals(
        &state,
        &evidence,
        &[NameTargetId::BuildingPart(1), NameTargetId::BuildingPart(3)],
        &[
            MissingNameEntry {
                target: NameTargetId::BuildingPart(1),
                name: "House".into(),
            },
            MissingNameEntry {
                target: NameTargetId::BuildingPart(3),
                name: "Kitchen".into(),
            },
        ],
    )
    .unwrap();
    let dev_select = after_select
        .proposals
        .iter()
        .find(|p| p.candidate.target == NameTargetId::Device(1))
        .unwrap();
    assert_eq!(
        dev_select.proposed_name.as_deref(),
        Some("House / Kitchen – Switch Actuator")
    );

    // 2. Edit: BuildingPart(3) edited to "Pantry" -> Device(1) updates.
    let after_edit = reevaluate_affected_proposals(
        &state,
        &evidence,
        &[NameTargetId::BuildingPart(3)],
        &[
            MissingNameEntry {
                target: NameTargetId::BuildingPart(1),
                name: "House".into(),
            },
            MissingNameEntry {
                target: NameTargetId::BuildingPart(3),
                name: "Pantry".into(),
            },
        ],
    )
    .unwrap();
    let dev_edit = after_edit
        .proposals
        .iter()
        .find(|p| p.candidate.target == NameTargetId::Device(1))
        .unwrap();
    assert_eq!(
        dev_edit.proposed_name.as_deref(),
        Some("House / Pantry – Switch Actuator")
    );

    // 3. Partial deselection: BuildingPart(3) deselected while BuildingPart(1) remains active.
    let after_partial_deselect = reevaluate_affected_proposals(
        &state,
        &evidence,
        &[NameTargetId::BuildingPart(3)],
        &[MissingNameEntry {
            target: NameTargetId::BuildingPart(1),
            name: "House".into(),
        }],
    )
    .unwrap();
    let dev_partial = after_partial_deselect
        .proposals
        .iter()
        .find(|p| p.candidate.target == NameTargetId::Device(1))
        .unwrap();
    assert_ne!(
        dev_partial.proposed_name.as_deref(),
        Some("House / Pantry – Switch Actuator")
    );

    // 4. Full deselection: last draft parent deselected (draft_parents empty, changed_parents non-empty).
    let baseline = knx_edit::missing_name_recipes::propose_missing_names(
        &state.project,
        &evidence,
        knx_edit::missing_names::MissingNameScanRequest::unbounded(0, usize::MAX),
    );
    let baseline_dev = baseline
        .items
        .iter()
        .find(|p| p.candidate.target == NameTargetId::Device(1))
        .unwrap();
    let after_full_deselect =
        reevaluate_affected_proposals(&state, &evidence, &[NameTargetId::BuildingPart(1)], &[])
            .unwrap();
    assert!(after_full_deselect
        .invalidated_targets
        .contains(&NameTargetId::Device(1)));
    let dev_reverted = after_full_deselect
        .proposals
        .iter()
        .find(|p| p.candidate.target == NameTargetId::Device(1))
        .unwrap();
    assert_eq!(dev_reverted.proposed_name, baseline_dev.proposed_name);
    assert_eq!(dev_reverted.provenance, baseline_dev.provenance);
    assert_eq!(state, before);

    // 5. Draft-parent validation failures (even when changed_parents is empty).
    let mut non_blank_state = full_q9_fixture();
    non_blank_state.project.installations[0].buildings[0].name = "Existing Building".into();
    assert_eq!(
        reevaluate_affected_proposals(
            &non_blank_state,
            &evidence,
            &[],
            &[MissingNameEntry {
                target: NameTargetId::BuildingPart(1),
                name: "Overwrite".into(),
            }],
        )
        .unwrap_err(),
        MissingNameApplyError::TargetNotBlank(NameTargetId::BuildingPart(1))
    );
    assert_eq!(
        reevaluate_affected_proposals(
            &state,
            &evidence,
            &[],
            &[MissingNameEntry {
                target: NameTargetId::BuildingPart(1),
                name: "Bad\nName".into(),
            }],
        )
        .unwrap_err(),
        MissingNameApplyError::InvalidName(
            NameTargetId::BuildingPart(1),
            knx_core::names::NameError::ControlCharacter,
        )
    );
    assert_eq!(
        reevaluate_affected_proposals(
            &state,
            &evidence,
            &[NameTargetId::BuildingPart(1)],
            &[
                MissingNameEntry {
                    target: NameTargetId::BuildingPart(1),
                    name: "A".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::BuildingPart(1),
                    name: "B".into(),
                },
            ],
        )
        .unwrap_err(),
        MissingNameApplyError::DuplicateSelection(NameTargetId::BuildingPart(1))
    );
    let too_many_drafts: Vec<MissingNameEntry> = (0..=MAX_MISSING_NAME_SELECTION)
        .map(|i| MissingNameEntry {
            target: NameTargetId::BuildingPart((i + 1) as u32),
            name: format!("BP {i}"),
        })
        .collect();
    assert_eq!(
        reevaluate_affected_proposals(&state, &evidence, &[], &too_many_drafts).unwrap_err(),
        MissingNameApplyError::InvalidSelection
    );
}

#[test]
fn preview_and_confirmation_token_are_invariant_under_entry_permutations() {
    let state = full_q9_fixture();
    let revision = rev();
    let order_a = vec![
        MissingNameEntry {
            target: NameTargetId::GroupAddress(1),
            name: "GA One".into(),
        },
        MissingNameEntry {
            target: NameTargetId::Project,
            name: "Project One".into(),
        },
        MissingNameEntry {
            target: NameTargetId::Device(1),
            name: "Device One".into(),
        },
        MissingNameEntry {
            target: NameTargetId::BuildingPart(3),
            name: "Room One".into(),
        },
        MissingNameEntry {
            target: NameTargetId::BuildingPart(1),
            name: "House One".into(),
        },
    ];
    let mut order_b = order_a.clone();
    order_b.reverse();

    let plan_a = preview_missing_name_apply(
        &state,
        &revision,
        MissingNameApplyIntent { entries: order_a },
    )
    .unwrap();
    let plan_b = preview_missing_name_apply(
        &state,
        &revision,
        MissingNameApplyIntent { entries: order_b },
    )
    .unwrap();

    assert_eq!(plan_a.preview(), plan_b.preview());
    assert_eq!(
        plan_a.confirmation_token("perm-auth").unwrap(),
        plan_b.confirmation_token("perm-auth").unwrap()
    );
}

#[test]
fn exact_256_entry_batch_applies_atomically_with_single_undo_redo() {
    let mut state = full_q9_fixture();
    state.project.installations[0].group_addresses.clear();
    let mut entries = Vec::with_capacity(MAX_MISSING_NAME_SELECTION);
    for i in 1..=(MAX_MISSING_NAME_SELECTION as u32) {
        let blank = if i % 2 == 0 { "" } else { "  " };
        state.project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(i),
                source: src(&format!("GA-{i}")),
                name: blank.into(),
                address: GroupAddress::from_raw((2048 + i) as u16),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Override::Absent,
                description: None,
            });
        entries.push(MissingNameEntry {
            target: NameTargetId::GroupAddress(i),
            name: format!("Batch GA {i}"),
        });
    }
    state.project.ids.raise_to(
        &IdAllocators::from_counts(1, 1, 1, 1, 2, 260, 11, 0, 0).with_function_counts(1, 1),
    );

    let original = state.clone();
    let revision = rev();
    let plan = preview_missing_name_apply(&state, &revision, MissingNameApplyIntent { entries })
        .expect("exact 256-entry batch must succeed");
    assert_eq!(plan.preview().changed, MAX_MISSING_NAME_SELECTION);
    assert_eq!(plan.preview().rows.len(), MAX_MISSING_NAME_SELECTION);

    let token = plan.confirmation_token("max-batch").unwrap();
    let mut stack = CommandStack::new();
    let outcome = apply_missing_name_apply(
        &mut state,
        &mut stack,
        &revision,
        &plan,
        "max-batch",
        &token,
        None,
    )
    .unwrap();
    assert!(outcome.changed);
    assert_eq!(stack.history_lengths(), (1, 0));
    for (idx, ga) in state.project.installations[0]
        .group_addresses
        .iter()
        .enumerate()
    {
        assert_eq!(ga.name, format!("Batch GA {}", idx + 1));
    }

    // Single Undo restores all 256 exact original blank strings.
    stack.undo(&mut state.project).unwrap();
    assert_eq!(state, original);
    assert!(!stack.can_undo());
    assert!(stack.can_redo());

    // Single Redo restores all 256 names.
    stack.redo(&mut state.project).unwrap();
    for (idx, ga) in state.project.installations[0]
        .group_addresses
        .iter()
        .enumerate()
    {
        assert_eq!(ga.name, format!("Batch GA {}", idx + 1));
    }
}

#[test]
fn cyclic_building_part_and_group_range_links_terminate_safely() {
    let mut state = full_q9_fixture();
    // Create a 2-cycle in BuildingPart parent links: BP(1) <-> BP(2).
    state.project.installations[0].buildings[0].parent = Some(BuildingPartId(2));
    state.project.installations[0].buildings[1].parent = Some(BuildingPartId(1));
    state.project.installations[0].buildings[1]
        .children
        .push(BuildingPartId(1));
    // Create a 2-cycle in GroupRange parent links: GR(1) <-> GR(2).
    state.project.installations[0].group_ranges[0].parent = Some(GroupRangeId(2));
    state.project.installations[0].group_ranges[1].parent = Some(GroupRangeId(1));
    state.project.installations[0].group_ranges[1]
        .children
        .push(GroupRangeId(1));

    let bp_deps = invalidated_dependents(&state.project, &[NameTargetId::BuildingPart(1)]);
    assert!(bp_deps.contains(&NameTargetId::BuildingPart(2)));
    assert!(bp_deps.contains(&NameTargetId::Device(1)));
    assert!(!bp_deps.contains(&NameTargetId::BuildingPart(1)));

    let gr_deps = invalidated_dependents(&state.project, &[NameTargetId::GroupRange(1)]);
    assert!(gr_deps.contains(&NameTargetId::GroupRange(2)));
    assert!(gr_deps.contains(&NameTargetId::GroupAddress(1)));
    assert!(!gr_deps.contains(&NameTargetId::GroupRange(1)));

    let reeval = reevaluate_affected_proposals(
        &state,
        &[],
        &[NameTargetId::BuildingPart(1), NameTargetId::GroupRange(1)],
        &[
            MissingNameEntry {
                target: NameTargetId::BuildingPart(1),
                name: "Cycle BP".into(),
            },
            MissingNameEntry {
                target: NameTargetId::GroupRange(1),
                name: "Cycle GR".into(),
            },
        ],
    )
    .expect("cyclic reevaluation must terminate cleanly");
    assert!(reeval
        .invalidated_targets
        .contains(&NameTargetId::GroupAddress(1)));

    let plan = preview_missing_name_apply(
        &state,
        &rev(),
        MissingNameApplyIntent {
            entries: vec![
                MissingNameEntry {
                    target: NameTargetId::GroupRange(2),
                    name: "GR Two".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::GroupRange(1),
                    name: "GR One".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::BuildingPart(2),
                    name: "BP Two".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::BuildingPart(1),
                    name: "BP One".into(),
                },
            ],
        },
    )
    .expect("cyclic preview must terminate cleanly");
    assert_eq!(plan.preview().rows.len(), 4);
}

#[test]
fn q13_multi_scope_devices_multi_area_lines_unassigned_and_blank_sibling_exclusion() {
    let mut state = full_q9_fixture();
    // Device(1) is already placed in BOTH BuildingPart(3) and Line(1).
    // Add Device(2) (existing named in BP(3)), Device(3) (existing named in Line(1)),
    // Device(4) (blank sibling in BP(3) & Line(1)), Device(5) & Device(6) (unassigned).
    let make_dev = |id: u32, name: &str| DeviceInstance {
        id: DeviceId(id),
        source: src(&format!("D-{id}")),
        name: name.into(),
        description: None,
        address: Some(IndividualAddress::new(1, 2, id as u8).unwrap()),
        product_ref: "M-TEST_P-1".into(),
        program_ref: "M-TEST_HP-1".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    };
    state.project.devices.insert(make_dev(2, "Shared Actuator"));
    state.project.devices.insert(make_dev(3, "Shared Actuator"));
    state.project.devices.insert(make_dev(4, "   "));
    state
        .project
        .devices
        .insert(make_dev(5, "Unassigned Existing"));
    state.project.devices.insert(make_dev(6, ""));

    state.project.installations[0].buildings[2]
        .devices
        .extend([DeviceId(2), DeviceId(4)]);
    state.project.installations[0].topology.lines[0]
        .devices
        .extend([DeviceId(3), DeviceId(4)]);
    state.project.installations[0]
        .topology
        .unassigned
        .extend([DeviceId(5), DeviceId(6)]);

    // Add a second Area(2) that also owns Line(1), plus an existing Line(2) in Area(2) with same name.
    state.project.installations[0].topology.areas.push(Area {
        id: AreaId(2),
        source: src("A-2"),
        name: "Area Two".into(),
        address: 2,
        completion: CompletionStatus::Undefined,
        lines: vec![LineId(1), LineId(2)],
    });
    state.project.installations[0].topology.lines.push(Line {
        id: LineId(2),
        source: src("L-2"),
        name: "Line Label".into(),
        address: 3,
        medium_ref: "MT-0".into(),
        domain_address: None,
        domain_address_is_checked: None,
        ip_routing_multicast_address: None,
        multicast_ttl: None,
        completion: CompletionStatus::Undefined,
        devices: vec![],
    });

    // Also verify build_dependency_graph includes inst.topology.unassigned.
    let inst_deps = invalidated_dependents(&state.project, &[NameTargetId::Installation(0)]);
    assert!(inst_deps.contains(&NameTargetId::Device(6)));

    let plan = preview_missing_name_apply(
        &state,
        &rev(),
        MissingNameApplyIntent {
            entries: vec![
                MissingNameEntry {
                    target: NameTargetId::Device(1),
                    name: "Shared Actuator".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::Device(6),
                    name: "Unassigned Existing".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::Line(1),
                    name: "Line Label".into(),
                },
            ],
        },
    )
    .unwrap();

    let warnings = &plan.preview().warnings;
    // Device(1) collides in BOTH BuildingPartDevices(3) with Device(2) AND LineDevices(1) with Device(3),
    // and blank sibling Device(4) is NOT reported as a conflict.
    let dev1_bp = warnings
        .iter()
        .find(|w| {
            w.target == NameTargetId::Device(1)
                && w.scope == DuplicateNameScope::BuildingPartDevices { building_part: 3 }
        })
        .expect("BuildingPartDevices conflict expected");
    assert_eq!(
        dev1_bp.conflicts_with,
        vec![DuplicateConflictSource::ExistingSibling(
            NameTargetId::Device(2)
        )]
    );
    let dev1_line = warnings
        .iter()
        .find(|w| {
            w.target == NameTargetId::Device(1)
                && w.scope == DuplicateNameScope::LineDevices { line: 1 }
        })
        .expect("LineDevices conflict expected");
    assert_eq!(
        dev1_line.conflicts_with,
        vec![DuplicateConflictSource::ExistingSibling(
            NameTargetId::Device(3)
        )]
    );

    // Device(6) collides in ProjectUnassignedDevices with Device(5).
    let dev6_unassigned = warnings
        .iter()
        .find(|w| {
            w.target == NameTargetId::Device(6)
                && w.scope == DuplicateNameScope::ProjectUnassignedDevices
        })
        .expect("ProjectUnassignedDevices conflict expected");
    assert_eq!(
        dev6_unassigned.conflicts_with,
        vec![DuplicateConflictSource::ExistingSibling(
            NameTargetId::Device(5)
        )]
    );

    // Line(1) collides in Area(2)'s AreaLines scope with Line(2), proving ALL owning areas are checked.
    let line1_area2 = warnings
        .iter()
        .find(|w| {
            w.target == NameTargetId::Line(1)
                && w.scope
                    == DuplicateNameScope::AreaLines {
                        installation: 0,
                        area: Some(2),
                    }
        })
        .expect("Area(2) AreaLines conflict expected");
    assert_eq!(
        line1_area2.conflicts_with,
        vec![DuplicateConflictSource::ExistingSibling(
            NameTargetId::Line(2)
        )]
    );
}

#[test]
fn parent_order_index_backfills_missing_parent_pointers_from_children_links() {
    let mut state = full_q9_fixture();
    // Clear child.parent on BP(2) and GR(2), leaving only parent.children links.
    state.project.installations[0].buildings[1].parent = None;
    state.project.installations[0].group_ranges[1].parent = None;

    let plan = preview_missing_name_apply(
        &state,
        &rev(),
        MissingNameApplyIntent {
            entries: vec![
                MissingNameEntry {
                    target: NameTargetId::GroupRange(2),
                    name: "Child GR".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::GroupRange(1),
                    name: "Parent GR".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::BuildingPart(2),
                    name: "Child BP".into(),
                },
                MissingNameEntry {
                    target: NameTargetId::BuildingPart(1),
                    name: "Parent BP".into(),
                },
            ],
        },
    )
    .unwrap();

    let ordered: Vec<NameTargetId> = plan.preview().rows.iter().map(|r| r.target).collect();
    assert_eq!(
        ordered,
        vec![
            NameTargetId::BuildingPart(1),
            NameTargetId::BuildingPart(2),
            NameTargetId::GroupRange(1),
            NameTargetId::GroupRange(2),
        ]
    );
}
