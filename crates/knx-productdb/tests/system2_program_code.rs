//! Checks System-2 product steps without authorizing any device download.

use knx_core::commissioning::load_control::{task_control_2, TaskControl2};
use knx_productdb::code::{load_program_code, parse_program_code, LoadStep};
use knx_productdb::download_plan::{check_program_kind, DownloadPlanError};
use knx_productdb::{install_package_with_limits, open_and_migrate, PackageLimits};

#[test]
fn xml_zero_based_segment_names_map_to_the_property_words_in_declared_order() {
    let program = "M-0000_A-0001-01-0000";
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20">
      <ManufacturerData><Manufacturer RefId="M-0000"><ApplicationPrograms>
        <ApplicationProgram Id="{program}" MaskVersion="MV-0025" LoadProcedureStyle="ProductProcedure">
          <Static><LoadProcedures><LoadProcedure>
            <LdCtrlTaskCtrl2 LsmIdx="3" Callback="4660" Address="22136" Seg0="39612" Seg1="57072" />
          </LoadProcedure></LoadProcedures></Static>
        </ApplicationProgram>
      </ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    let code = parse_program_code("synthetic.xml", xml.as_bytes(), program)
        .expect("valid XML")
        .expect("application found");
    let LoadStep::TaskCtrl2 {
        lsm,
        callback,
        address,
        segment_0,
        segment_1,
    } = code.load_procedures[0].steps[0]
    else {
        panic!("expected a typed TaskCtrl2");
    };
    assert_eq!(lsm, 3);
    let payload = task_control_2(TaskControl2 {
        callback,
        communication_objects: address,
        segment_1: segment_0,
        segment_2: segment_1,
    });
    assert_eq!(
        payload.octets(),
        &[3, 5, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]
    );
    assert_eq!(
        check_program_kind(&code),
        Err(DownloadPlanError::NotMemoryMapped(Some("MV-0025".into())))
    );
}

/// The source is private/non-redistributable; it is never embedded in the test.
#[test]
#[ignore = "requires OriginalData private Siemens product corpus; no hardware"]
fn siemens_system2_preserves_the_whole_procedure_and_stays_not_executable() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| knx_testsupport::workspace_root().join("OriginalData/ProductDatabases"));
    let file = "Siemens_HVAC_All_PDB_Oct_2023_ETS5_ETS6.knxprod";
    let path = knx_testsupport::find_corpus_file(&root, file)
        .expect("exact private Siemens fixture must exist for this ignored test");
    let bytes = std::fs::read(path).expect("readable private fixture");
    let dir = tempfile::tempdir().expect("isolated directory");
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).expect("isolated database");
    install_package_with_limits(&conn, file, &bytes, PackageLimits::LARGE)
        .expect("bounded package import");
    let program = "M-0001_A-9205-04-9270";
    let code = load_program_code(&conn, program)
        .expect("application source readable")
        .expect("exact application installed");
    assert_eq!(code.mask_version.as_deref(), Some("MV-0025"));
    assert_eq!(
        code.load_procedure_style.as_deref(),
        Some("ProductProcedure")
    );
    assert_eq!(code.load_procedures.len(), 1);
    let procedure = &code.load_procedures[0];
    assert_eq!(procedure.merge_id, None);
    assert_eq!(procedure.steps.len(), 21);
    assert_eq!(procedure.unmodelled().count(), 0);
    let metadata = knx_productdb::system2::project_tool_metadata(&code)
        .expect("exact source-bound tool metadata, not an executable plan");
    assert_eq!(metadata.table_address, 0x0400);
    assert_eq!(metadata.steps.len(), procedure.steps.len());
    assert!(matches!(
        metadata.steps[16],
        knx_productdb::system2::ProjectedStep::ToolTableReference { address: 0x0400 }
    ));
    assert_eq!(procedure.steps[0], LoadStep::Connect);
    assert_eq!(
        procedure.steps[1],
        LoadStep::CompareProp {
            object_index: 0,
            property_id: 78,
            data: vec![0, 1, 0, 0, 0, 0x5B, 0, 0, 0, 0],
        }
    );
    assert_eq!(
        procedure.steps[15],
        LoadStep::TaskCtrl2 {
            lsm: 3,
            callback: 0,
            address: 0x0400,
            segment_0: 0,
            segment_1: 0,
        }
    );
    assert_eq!(
        procedure.steps[16],
        LoadStep::TaskSegment {
            lsm: 5,
            address: 0x0400
        }
    );
    assert_eq!(procedure.steps[20], LoadStep::Disconnect);
    assert!(!procedure.steps.contains(&LoadStep::Restart));
    let LoadStep::TaskCtrl2 {
        callback,
        address,
        segment_0,
        segment_1,
        ..
    } = procedure.steps[15]
    else {
        panic!("TaskCtrl2 must stay in its declared position");
    };
    let payload = task_control_2(TaskControl2 {
        callback,
        communication_objects: address,
        segment_1: segment_0,
        segment_2: segment_1,
    });
    assert_eq!(payload.octets(), &[3, 5, 0, 0, 4, 0, 0, 0, 0, 0]);
    assert_eq!(
        check_program_kind(&code),
        Err(DownloadPlanError::NotMemoryMapped(Some("MV-0025".into())))
    );
}
