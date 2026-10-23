//! Experimental source translation preserves every instruction and every image interval.

use std::collections::BTreeMap;

use knx_core::commissioning::property_download::PropertyOperation as Op;
use knx_core::IndividualAddress;
use knx_productdb::code::{AbsoluteSegment, LoadProcedure, LoadStep, ProgramCode, TablePlacement};
use knx_productdb::image::{DownloadImage, SegmentImage, TableWrite};
use knx_productdb::system2_procedure::{plan_system2_candidate, System2ProcedurePolicy};

fn image() -> DownloadImage {
    let declarations = [
        ("A", 0x0116, 4),
        ("B", 0x0260, 4),
        ("C", 0x0400, 4),
        ("P", 0x3C00, 4),
    ];
    let segment = |(id, address, size): (&str, u32, u32)| AbsoluteSegment {
        id: id.into(),
        address,
        size,
        data: Some(vec![0; size as usize]),
        mask: None,
        other: BTreeMap::new(),
    };
    let allocate = |lsm, address| LoadStep::AbsSegment {
        lsm,
        segment_type: 0,
        address,
        size: 4,
        access: 0x33,
        memory_type: 3,
        flags: 0,
    };
    let code = ProgramCode {
        program_id: "M-1234_A-5678-09-TEST".into(),
        mask_version: Some("MV-0025".into()),
        load_procedure_style: Some("ProductProcedure".into()),
        segments: declarations.into_iter().map(segment).collect(),
        address_table: Some(TablePlacement {
            code_segment: Some("A".into()),
            offset: Some(0),
            max_entries: Some(1),
        }),
        association_table: Some(TablePlacement {
            code_segment: Some("B".into()),
            offset: Some(0),
            max_entries: Some(1),
        }),
        com_object_table: Some(TablePlacement {
            code_segment: Some("C".into()),
            offset: Some(0),
            max_entries: Some(1),
        }),
        program_attributes: BTreeMap::from([
            ("ApplicationNumber".into(), "22136".into()),
            ("ApplicationVersion".into(), "9".into()),
            ("PeiType".into(), "7".into()),
        ]),
        load_procedures: vec![LoadProcedure {
            merge_id: None,
            steps: vec![
                LoadStep::Connect,
                LoadStep::CompareProp {
                    object_index: 0,
                    property_id: 78,
                    data: vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0, 0, 0, 0],
                },
                LoadStep::Unload { lsm: 1 },
                LoadStep::Unload { lsm: 2 },
                LoadStep::Unload { lsm: 3 },
                LoadStep::Load { lsm: 1 },
                allocate(1, 0x0116),
                LoadStep::TaskSegment {
                    lsm: 1,
                    address: 0x0116,
                },
                LoadStep::Load { lsm: 2 },
                allocate(2, 0x0260),
                LoadStep::TaskSegment {
                    lsm: 2,
                    address: 0x0260,
                },
                LoadStep::Load { lsm: 3 },
                allocate(3, 0x0400),
                allocate(3, 0x3C00),
                LoadStep::TaskSegment { lsm: 3, address: 0 },
                LoadStep::TaskCtrl2 {
                    lsm: 3,
                    callback: 0x1234,
                    address: 0x0400,
                    segment_0: 0x3456,
                    segment_1: 0x789A,
                },
                LoadStep::TaskSegment {
                    lsm: 5,
                    address: 0x0400,
                },
                LoadStep::LoadCompleted { lsm: 1 },
                LoadStep::LoadCompleted { lsm: 2 },
                LoadStep::LoadCompleted { lsm: 3 },
                LoadStep::Disconnect,
            ],
        }],
        ..ProgramCode::default()
    };
    DownloadImage {
        code,
        segments: declarations
            .into_iter()
            .enumerate()
            .map(|(i, (id, address, _))| SegmentImage {
                id: id.into(),
                address,
                octets: vec![i as u8 + 1; 4],
                mask: None,
            })
            .collect(),
        parameters: BTreeMap::new(),
        texts: BTreeMap::new(),
        objects: vec![],
        inferences: vec![],
        parameter_fields: vec![],
        table_writes: vec![],
    }
}

fn capacity_image() -> DownloadImage {
    let mut input = image();
    for i in 0..2 {
        input.code.segments[i].size = 8;
        input.code.segments[i].data = Some(vec![0; 8]);
        input.segments[i].octets = vec![0xFE, 1, 2, 3, 4, 5, 0, 0xA5];
    }
    for step in &mut input.code.load_procedures[0].steps {
        if let LoadStep::AbsSegment { address, size, .. } = step {
            if matches!(*address, 0x0116 | 0x0260) {
                *size = 8;
            }
        }
    }
    input.table_writes = vec![
        TableWrite {
            segment_id: "A".into(),
            start: 0,
            end: 3,
        },
        TableWrite {
            segment_id: "B".into(),
            start: 0,
            end: 1,
        },
    ];
    input
}

#[test]
fn capacity_policy_preserves_independent_source_data_and_omits_reserved_tail() {
    let input = capacity_image();
    let plan = plan_system2_candidate(
        &input,
        IndividualAddress::from_raw(0x0119),
        &System2ProcedurePolicy::source_capacity_preserving_independent_data(),
    )
    .unwrap();
    let writes: Vec<_> = plan
        .steps()
        .iter()
        .filter_map(|s| match &s.operation {
            Op::WriteMemory { address, data, .. } if s.source_index == 6 => {
                Some((*address, data.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        writes,
        vec![(0x0116, vec![0xFE, 1, 2]), (0x011B, vec![5, 0, 0xA5])]
    );
    assert_eq!(plan.source_positions(), 21);
}

#[test]
fn capacity_policy_refuses_unpaired_or_malformed_computed_metadata() {
    for case in 0..8 {
        let mut input = capacity_image();
        match case {
            0 => {
                input.table_writes.pop();
            }
            1 => input.table_writes.push(input.table_writes[0].clone()),
            2 => input.table_writes[0].segment_id = "unknown".into(),
            3 => input.table_writes[0].start = 1,
            4 => input.table_writes[0].end = 0,
            5 => input.table_writes[0].end = 6,
            6 => input.table_writes[0].end = usize::MAX,
            7 => input.table_writes[0].start = 4,
            _ => unreachable!(),
        }
        let result = plan_system2_candidate(
            &input,
            IndividualAddress::from_raw(0x0119),
            &System2ProcedurePolicy::source_capacity_preserving_independent_data(),
        );
        assert!(
            result.is_err(),
            "malformed computed metadata case {case} accepted"
        );
        assert!(result
            .unwrap_err()
            .reason
            .contains("computed table metadata"));
    }
}

#[test]
fn capacity_policy_masks_split_computed_and_independent_data_at_nonzero_offset() {
    let mut input = capacity_image();
    input.code.address_table.as_mut().unwrap().offset = Some(2);
    input.table_writes[0].start = 2;
    input.table_writes[0].end = 5;
    input.code.load_procedures[0].steps[7] = LoadStep::TaskSegment {
        lsm: 1,
        address: 0x0118,
    };
    let mask = vec![0xFF, 0, 0xFF, 0, 0xFF, 0xFF, 0xFF, 0xFF];
    input.code.segments[0].mask = Some(mask.clone());
    input.segments[0].mask = Some(mask);
    let result = plan_system2_candidate(
        &input,
        IndividualAddress::from_raw(0x0119),
        &System2ProcedurePolicy::source_capacity_preserving_independent_data(),
    )
    .unwrap();
    let writes: Vec<_> = result
        .steps()
        .iter()
        .filter_map(|s| match &s.operation {
            Op::WriteMemory { address, data, .. } if s.source_index == 6 => {
                Some((*address, data.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        writes,
        [
            (0x0116, vec![0xFE]),
            (0x0118, vec![2]),
            (0x011A, vec![4]),
            (0x011D, vec![0xA5])
        ]
    );
    let allocations: Vec<_> = result
        .steps()
        .iter()
        .filter_map(|s| match &s.operation {
            Op::LoadControl { payload, .. } if s.source_index == 6 => Some(*payload.octets()),
            _ => None,
        })
        .collect();
    assert_eq!(allocations, [[3, 0, 1, 0x16, 0, 8, 0x33, 3, 0, 0]]);
}

#[test]
fn capacity_policy_uses_all_same_segment_spans_independent_of_metadata_order() {
    // Synthetic planner-interval test only; no claim about physical resource sharing.
    let mut input = capacity_image();
    input.code.segments[0].size = 20;
    input.code.segments[0].data = Some(vec![0; 20]);
    input.segments[0].octets = (0..20).collect();
    if let LoadStep::AbsSegment { size, .. } = &mut input.code.load_procedures[0].steps[6] {
        *size = 20;
    }
    input.code.association_table.as_mut().unwrap().code_segment = Some("A".into());
    input.code.association_table.as_mut().unwrap().offset = Some(8);
    input.code.load_procedures[0].steps[10] = LoadStep::TaskSegment {
        lsm: 2,
        address: 0x011E,
    };
    input.table_writes[1] = TableWrite {
        segment_id: "A".into(),
        start: 8,
        end: 9,
    };
    let run = |input: &DownloadImage| {
        plan_system2_candidate(
            input,
            IndividualAddress::from_raw(0x0119),
            &System2ProcedurePolicy::source_capacity_preserving_independent_data(),
        )
        .unwrap()
    };
    let first = run(&input);
    input.table_writes.reverse();
    assert_eq!(first, run(&input));
    let writes: Vec<_> = first
        .steps()
        .iter()
        .filter_map(|s| match &s.operation {
            Op::WriteMemory { address, data, .. } if s.source_index == 6 => {
                Some((*address, data.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        writes,
        [
            (0x0116, vec![0, 1, 2]),
            (0x011B, vec![5, 6, 7, 8]),
            (0x0121, (11..20).collect())
        ]
    );
}

#[test]
fn capacity_policy_refuses_invalid_source_reservations_and_nonbinary_masks() {
    for case in 0..5 {
        let mut input = capacity_image();
        match case {
            0 => input.code.address_table.as_mut().unwrap().max_entries = Some(u32::MAX),
            1 => input.code.address_table.as_mut().unwrap().offset = Some(u32::MAX),
            2 => {
                input.code.association_table.as_mut().unwrap().code_segment = Some("A".into());
            }
            3 => {
                input.code.address_table.as_mut().unwrap().max_entries = None;
            }
            4 => {
                input.code.segments[0].mask = Some(vec![0xFE; 8]);
                input.segments[0].mask = Some(vec![0xFE; 8]);
            }
            _ => unreachable!(),
        }
        assert!(
            plan_system2_candidate(
                &input,
                IndividualAddress::from_raw(0x0119),
                &System2ProcedurePolicy::source_capacity_preserving_independent_data()
            )
            .is_err(),
            "source case {case}"
        );
    }
}

fn plan(
    image: &DownloadImage,
) -> Result<
    knx_core::commissioning::property_download::PropertyDownloadPlan,
    knx_productdb::system2_procedure::System2ProcedureError,
> {
    plan_system2_candidate(
        image,
        IndividualAddress::from_raw(0x0119),
        &System2ProcedurePolicy::declared_counts_zero_flags_full_table_identity(),
    )
}

#[test]
fn complete_source_sequence_expands_allocations_without_losing_positions() {
    let result = plan(&image()).unwrap();
    assert_eq!(result.source_positions(), 21);
    assert_eq!(result.steps().len(), 25);
    let controls: Vec<_> = result
        .steps()
        .iter()
        .filter_map(|s| match &s.operation {
            Op::LoadControl { object, payload } => Some((*object, *payload.octets())),
            _ => None,
        })
        .collect();
    assert!(controls.contains(&(1, [3, 2, 1, 0x16, 7, 0x12, 0x34, 0x56, 0x78, 9])));
    assert!(controls.contains(&(2, [3, 2, 2, 0x60, 7, 0x12, 0x34, 0x56, 0x78, 9])));
    assert!(controls.contains(&(3, [3, 2, 0, 0, 7, 0x12, 0x34, 0x56, 0x78, 9])));
    assert!(controls.contains(&(3, [3, 5, 0x12, 0x34, 4, 0, 0x34, 0x56, 0x78, 0x9A])));
    assert_eq!(
        result
            .steps()
            .iter()
            .filter(|s| matches!(s.operation, Op::ToolMetadata { .. }))
            .count(),
        1
    );
    assert!(!result
        .steps()
        .iter()
        .any(|s| matches!(s.operation, Op::LoadControl { object: 5, .. })));
}

#[test]
fn source_masks_split_writes_without_transmitting_protected_bytes() {
    let mut input = image();
    input.segments[0].mask = Some(vec![0, 0xFF, 0, 0xFF]);
    input.code.segments[0].mask = Some(vec![0, 0xFF, 0, 0xFF]);
    let result = plan(&input).unwrap();
    let writes: Vec<_> = result
        .steps()
        .iter()
        .filter_map(|s| match &s.operation {
            Op::WriteMemory { address, data, .. } if s.source_index == 6 => {
                Some((*address, data.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(writes, vec![(0x0117, vec![1]), (0x0119, vec![1])]);
}

#[test]
fn table_segment_writes_only_the_computed_table_not_the_template_capacity() {
    let mut input = image();
    // Both table segments ship a computed table far smaller than the four
    // writable octets of their template; the plan must write only the table.
    input.table_writes = vec![
        TableWrite {
            segment_id: "A".into(),
            start: 0,
            end: 1,
        },
        TableWrite {
            segment_id: "B".into(),
            start: 0,
            end: 2,
        },
    ];
    let result = plan(&input).unwrap();
    let writes_for = |index: usize| -> Vec<_> {
        result
            .steps()
            .iter()
            .filter_map(|s| match &s.operation {
                Op::WriteMemory { address, data, .. } if s.source_index == index => {
                    Some((*address, data.clone()))
                }
                _ => None,
            })
            .collect()
    };
    assert_eq!(writes_for(6), vec![(0x0116, vec![1])]);
    assert_eq!(writes_for(9), vec![(0x0260, vec![2, 2])]);
}

#[test]
fn table_task_pointer_must_match_its_source_resource_not_merely_any_address() {
    let mut input = image();
    input.code.load_procedures[0].steps[7] = LoadStep::TaskSegment {
        lsm: 1,
        address: 0x3C00,
    };
    assert!(
        plan(&input).is_err(),
        "wrong address-table task pointer was admitted"
    );
}

#[test]
fn source_transforms_cannot_hide_behind_a_preassembled_image() {
    let mut input = image();
    input.code.has_fixup_list = true;
    assert!(
        plan(&input).is_err(),
        "uninterpreted source fixups were admitted"
    );
}

#[test]
fn unsupported_source_and_incomplete_or_duplicate_images_refuse_whole_plan() {
    let mut input = image();
    input.code.load_procedures[0].steps[15] = LoadStep::Restart;
    assert!(plan(&input).is_err());
    let mut input = image();
    input.segments.pop();
    assert!(plan(&input).is_err());
    let mut input = image();
    input.segments.push(input.segments[0].clone());
    assert!(plan(&input).is_err());
    let mut input = image();
    input.segments[0].octets.pop();
    assert!(plan(&input).is_err());
    let mut input = image();
    input.segments[0].mask = Some(vec![0x80; 4]);
    assert!(plan(&input).is_err());
    let mut input = image();
    if let LoadStep::AbsSegment { flags, .. } = &mut input.code.load_procedures[0].steps[6] {
        *flags = 1;
    }
    assert!(plan(&input).is_err());
}
