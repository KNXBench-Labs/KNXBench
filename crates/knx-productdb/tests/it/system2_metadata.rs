//! System-2 tool metadata is preserved without inventing on-device object five.

use knx_productdb::code::{AbsoluteSegment, LoadProcedure, LoadStep, ProgramCode, TablePlacement};
use knx_productdb::system2::{project_tool_metadata, MetadataError, ProjectedStep};

fn code() -> ProgramCode {
    ProgramCode {
        program_id: "M-0000_A-0001-01-0000".into(),
        mask_version: Some("MV-0025".into()),
        load_procedure_style: Some("ProductProcedure".into()),
        segments: vec![AbsoluteSegment {
            id: "objects".into(),
            address: 0x400,
            size: 8,
            data: Some(vec![2, 0, 0x12, 0x25, 7, 0x34, 0xA5, 8]),
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
                LoadStep::TaskCtrl2 {
                    lsm: 3,
                    callback: 0x1234,
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

#[test]
fn the_exact_pointer_witness_remains_ordered_tool_metadata_not_a_wire_step() {
    let source = code();
    let before = source.clone();
    let view = project_tool_metadata(&source).expect("exact source witness");
    assert_eq!(view.table_address, 0x400);
    assert_eq!(view.steps.len(), 5);
    assert!(matches!(
        view.steps[0],
        ProjectedStep::Retained(LoadStep::Connect)
    ));
    assert!(matches!(
        view.steps[2],
        ProjectedStep::ToolTableReference { address: 0x400 }
    ));
    assert!(matches!(
        view.steps[3],
        ProjectedStep::Retained(LoadStep::LoadCompleted { lsm: 3 })
    ));
    assert_eq!(source, before, "projection never rewrites or drops source");
    assert!(knx_productdb::download_plan::check_program_kind(&source).is_err());
}

#[test]
fn an_additional_task_control_is_ambiguous_even_on_another_machine() {
    let mut source = code();
    source.load_procedures[0].steps.insert(
        1,
        LoadStep::TaskCtrl2 {
            lsm: 2,
            callback: 0,
            address: 0x401,
            segment_0: 0,
            segment_1: 0,
        },
    );
    assert_eq!(
        project_tool_metadata(&source),
        Err(MetadataError::TaskControl)
    );
}

#[test]
fn pointer_disagreement_never_becomes_metadata() {
    let mut source = code();
    source.load_procedures[0].steps[2] = LoadStep::TaskSegment {
        lsm: 5,
        address: 0x401,
    };
    assert_eq!(
        project_tool_metadata(&source),
        Err(MetadataError::PointerMismatch)
    );
    source = code();
    source.com_object_table.as_mut().unwrap().offset = Some(1);
    assert_eq!(
        project_tool_metadata(&source),
        Err(MetadataError::PointerMismatch)
    );
}

#[test]
fn other_object_five_operations_and_unknown_shapes_are_never_skipped() {
    for step in [
        LoadStep::Load { lsm: 5 },
        LoadStep::Unload { lsm: 5 },
        LoadStep::LoadCompleted { lsm: 5 },
        LoadStep::CompareProp {
            object_index: 5,
            property_id: 5,
            data: vec![],
        },
        LoadStep::TaskCtrl1 {
            lsm: 5,
            address: 0x400,
            count: 1,
        },
        LoadStep::Unmodelled {
            name: "LdCtrlTaskSegment".into(),
            attributes: [("LsmIdx".into(), "5".into()), ("Extra".into(), "1".into())].into(),
            has_children: false,
        },
        LoadStep::Unmodelled {
            name: "LdCtrlWriteProp".into(),
            attributes: Default::default(),
            has_children: true,
        },
    ] {
        let mut source = code();
        source.load_procedures[0].steps.push(step);
        let before = source.clone();
        assert_eq!(
            project_tool_metadata(&source),
            Err(MetadataError::UnsupportedStep { index: 5 })
        );
        assert_eq!(source, before);
    }
}

#[test]
fn only_the_exact_family_and_procedure_shape_are_projected() {
    for mask in ["MV-0701", "MV-07B0", "MV-0012", "MV-0026", "MV-0025 "] {
        let mut source = code();
        source.mask_version = Some(mask.into());
        assert_eq!(
            project_tool_metadata(&source),
            Err(MetadataError::UnsupportedProgram)
        );
    }
    for style in [None, Some("DefaultProcedure"), Some("MergedProcedure")] {
        let mut source = code();
        source.load_procedure_style = style.map(str::to_owned);
        assert_eq!(
            project_tool_metadata(&source),
            Err(MetadataError::UnsupportedProgram)
        );
    }
    let mut source = code();
    source.load_procedures[0].merge_id = Some("ap1".into());
    assert_eq!(
        project_tool_metadata(&source),
        Err(MetadataError::ProcedureShape)
    );
    source = code();
    source
        .load_procedures
        .push(source.load_procedures[0].clone());
    assert_eq!(
        project_tool_metadata(&source),
        Err(MetadataError::ProcedureShape)
    );
}

#[test]
fn missing_or_duplicate_references_and_ambiguous_table_placement_fail() {
    let mut source = code();
    source.load_procedures[0].steps.remove(2);
    assert_eq!(
        project_tool_metadata(&source),
        Err(MetadataError::ReferenceCount)
    );
    source = code();
    source.load_procedures[0].steps.push(LoadStep::TaskSegment {
        lsm: 5,
        address: 0x400,
    });
    assert_eq!(
        project_tool_metadata(&source),
        Err(MetadataError::ReferenceCount)
    );
    source = code();
    source.segments.push(source.segments[0].clone());
    assert_eq!(
        project_tool_metadata(&source),
        Err(MetadataError::TablePlacement)
    );
    source = code();
    source.com_object_table.as_mut().unwrap().offset = Some(8);
    assert_eq!(
        project_tool_metadata(&source),
        Err(MetadataError::TablePlacement)
    );
    source = code();
    source.segments[0].address = u32::MAX;
    assert_eq!(
        project_tool_metadata(&source),
        Err(MetadataError::TablePlacement)
    );
}

#[test]
fn reference_after_disconnect_keeps_its_exact_position() {
    let mut source = code();
    let reference = source.load_procedures[0].steps.remove(2);
    source.load_procedures[0].steps.push(reference);
    let view = project_tool_metadata(&source).unwrap();
    assert!(matches!(
        view.steps[3],
        ProjectedStep::Retained(LoadStep::Disconnect)
    ));
    assert!(matches!(
        view.steps[4],
        ProjectedStep::ToolTableReference { address: 0x400 }
    ));
    for mask in ["MV-0020", "MV-0021"] {
        source.mask_version = Some(mask.into());
        assert!(project_tool_metadata(&source).is_ok());
        assert!(knx_productdb::download_plan::check_program_kind(&source).is_err());
    }
}
