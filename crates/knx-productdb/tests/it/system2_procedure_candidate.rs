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

fn plan(
    image: &DownloadImage,
) -> Result<
    knx_core::commissioning::property_download::PropertyDownloadPlan,
    knx_productdb::system2_procedure::System2ProcedureError,
> {
    plan_system2_candidate(
        image,
        IndividualAddress::from_raw(0x0119),
        &System2ProcedurePolicy::declared_counts_zero_flags_zero_table_identity(),
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
    assert!(controls.contains(&(1, [3, 2, 1, 0x16, 0, 0, 0, 0, 0, 0])));
    assert!(controls.contains(&(2, [3, 2, 2, 0x60, 0, 0, 0, 0, 0, 0])));
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
