//! The System-2 declared-segment profile is derived from source data or refused whole.

use knx_core::commissioning::declared_memory::{DeclaredRange, RangeError, MAX_RANGES};
use knx_productdb::code::{AbsoluteSegment, LoadProcedure, LoadStep, ProgramCode, TablePlacement};
use knx_productdb::declared_segments::{derive_declared_segments, DeclaredSegmentsError};
use knx_productdb::system2::MetadataError;

fn segment(lsm: u8, address: u16, size: u16) -> LoadStep {
    LoadStep::AbsSegment {
        lsm,
        segment_type: 0,
        address,
        size,
        access: 0x33,
        memory_type: 3,
        flags: 0,
    }
}

fn compare(data: &[u8]) -> LoadStep {
    LoadStep::CompareProp {
        object_index: 0,
        property_id: 78,
        data: data.to_vec(),
    }
}

/// A synthetic program in the exact shape `project_tool_metadata` accepts.
fn code() -> ProgramCode {
    ProgramCode {
        program_id: "M-0000_A-0001-01-0000".into(),
        mask_version: Some("MV-0025".into()),
        load_procedure_style: Some("ProductProcedure".into()),
        segments: vec![AbsoluteSegment {
            id: "objects".into(),
            address: 0x400,
            size: 64,
            data: None,
            mask: None,
            other: Default::default(),
        }],
        com_object_table: Some(TablePlacement {
            code_segment: Some("objects".into()),
            offset: Some(0),
            max_entries: Some(2),
        }),
        load_procedures: vec![LoadProcedure {
            merge_id: None,
            steps: vec![
                LoadStep::Connect,
                compare(&[0, 1, 0, 0, 0, 0x5B, 0, 0]),
                LoadStep::Unload { lsm: 3 },
                LoadStep::Load { lsm: 3 },
                segment(3, 0x0400, 64),
                segment(3, 0x3C00, 30),
                LoadStep::TaskCtrl2 {
                    lsm: 3,
                    callback: 0,
                    address: 0x400,
                    segment_0: 0,
                    segment_1: 0,
                },
                LoadStep::TaskSegment {
                    lsm: 5,
                    address: 0x400,
                },
                LoadStep::LoadCompleted { lsm: 3 },
                LoadStep::Disconnect,
            ],
        }],
        ..Default::default()
    }
}

fn steps(source: &mut ProgramCode) -> &mut Vec<LoadStep> {
    &mut source.load_procedures[0].steps
}

#[test]
fn every_abs_segment_and_the_pid78_comparison_are_collected_in_source_order() {
    let source = code();
    let before = source.clone();
    let profile = derive_declared_segments(&source).expect("exact System-2 shape");
    assert_eq!(profile.program_id, "M-0000_A-0001-01-0000");
    assert_eq!(profile.mask, 0x0025);
    assert_eq!(
        profile.ranges.ranges(),
        [
            DeclaredRange {
                address: 0x0400,
                length: 64
            },
            DeclaredRange {
                address: 0x3C00,
                length: 30
            }
        ]
    );
    assert_eq!(profile.range_positions, [4, 5]);
    assert_eq!(profile.compare_position, 1);
    assert_eq!(profile.hardware_type, [0, 1, 0, 0, 0, 0x5B, 0, 0]);
    assert_eq!(profile.ranges.total_octets(), 94);
    assert_eq!(source, before, "derivation never rewrites the source");
}

#[test]
fn a_program_outside_the_system2_witness_is_refused_by_the_existing_precondition() {
    let mut source = code();
    source.mask_version = Some("MV-0701".into());
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::Shape(
            MetadataError::UnsupportedProgram
        ))
    );
    let mut source = code();
    source.load_procedures.push(LoadProcedure::default());
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::Shape(MetadataError::ProcedureShape))
    );
}

#[test]
fn a_missing_or_repeated_pid78_comparison_is_refused() {
    let mut source = code();
    steps(&mut source).remove(1);
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::HardwareTypeComparison { count: 0 })
    );
    let mut source = code();
    steps(&mut source).insert(2, compare(&[0, 2]));
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::HardwareTypeComparison { count: 2 })
    );
    // A comparison of another property is not the hardware-type check.
    let mut source = code();
    steps(&mut source)[1] = LoadStep::CompareProp {
        object_index: 0,
        property_id: 12,
        data: vec![0, 1],
    };
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::HardwareTypeComparison { count: 0 })
    );
    let mut source = code();
    steps(&mut source)[1] = compare(&[]);
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::EmptyHardwareType)
    );
}

#[test]
fn unsafe_range_shapes_are_refused_without_a_partial_profile() {
    let mut source = code();
    steps(&mut source)[5] = segment(3, 0x3C00, 0);
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::Ranges(RangeError::ZeroLength {
            index: 1
        }))
    );
    let mut source = code();
    steps(&mut source)[5] = segment(3, 0xFF00, 0x100);
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::Ranges(
            RangeError::OutsideMemoryService { index: 1 }
        ))
    );
    let mut source = code();
    steps(&mut source)[5] = segment(3, 0x0420, 8);
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::Ranges(RangeError::Overlap {
            first: 0,
            second: 1
        }))
    );
    let mut source = code();
    steps(&mut source)[5] = segment(3, 0x3C00, 1985);
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::Ranges(RangeError::TooManyOctets {
            total: 2049
        }))
    );
    let mut source = code();
    for index in 0..MAX_RANGES {
        steps(&mut source).insert(6, segment(3, 0x1000 + 0x10 * index as u16, 1));
    }
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::Ranges(RangeError::TooManyRanges {
            count: MAX_RANGES + 2
        }))
    );
    let mut source = code();
    steps(&mut source).retain(|step| !matches!(step, LoadStep::AbsSegment { .. }));
    assert_eq!(
        derive_declared_segments(&source),
        Err(DeclaredSegmentsError::Ranges(RangeError::Empty))
    );
}
