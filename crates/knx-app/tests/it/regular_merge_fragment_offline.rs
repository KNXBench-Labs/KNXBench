//! The offline download path reports a regular program's MergeId fragment as a procedure shape.

use knx_app::download_support::{offline_download, offline_plan, UnsupportedCategory};

const PROGRAM: &str = "M-0083_A-0027-15-0BAC";
const MAIN: &str = r#"<LoadProcedure><LdCtrlConnect /><LdCtrlUnload LsmIdx="3" /><LdCtrlLoad LsmIdx="3" /><LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="17408" Size="2" Access="255" MemType="3" SegFlags="128" /><LdCtrlLoadCompleted LsmIdx="3" /><LdCtrlRestart /><LdCtrlDisconnect /></LoadProcedure>"#;

fn installed(procedures: &str) -> (tempfile::TempDir, knx_productdb::Connection) {
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0083"><ApplicationPrograms>
<ApplicationProgram Id="{PROGRAM}" Name="Merge fragment" ApplicationNumber="39" ApplicationVersion="21" PeiType="1" MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="A" Address="17408" Size="2"><Data>qrs=</Data></AbsoluteSegment></Code>
<LoadProcedures>{procedures}</LoadProcedures>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "merge-fragment.xml", xml.as_bytes()).expect("ingests");
    (dir, conn)
}

#[test]
fn a_fragment_makes_the_offline_download_a_procedure_shape_refusal() {
    let fragment = r#"<LoadProcedure MergeId="4"><LdCtrlConnect /></LoadProcedure>"#;
    let (_dir, conn) = installed(&format!("{MAIN}{fragment}"));
    let Err((category, detail)) = offline_download(&conn, PROGRAM) else {
        panic!("a MergeId fragment was planned offline");
    };
    assert_eq!(category, UnsupportedCategory::ProcedureShape);
    assert_eq!(category.code(), "procedure-shape");
    assert!(detail.starts_with("load procedure: "), "{detail}");
    assert!(
        detail.contains("load procedure 2 declares MergeId \"4\""),
        "{detail}"
    );
    assert_eq!(offline_plan(&conn, PROGRAM).unwrap_err().0, category);
}

#[test]
fn the_same_program_without_a_fragment_still_plans_offline() {
    let (_dir, conn) = installed(MAIN);
    let (plan, _) = offline_download(&conn, PROGRAM).expect("plans");
    assert_eq!(plan.steps.len(), 8);
}
