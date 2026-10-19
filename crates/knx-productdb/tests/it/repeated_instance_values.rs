//! Stored repeated-module values remain distinct without evaluating Repeat.

use knx_core::{DeviceId, ModuleInstance, ModuleInstanceId, SourceRef};
use knx_productdb::device_evaluation::evaluate_device;
use rusqlite::Connection;

use knx_testsupport::REPEATED_INSTANCE_PROGRAM_XML as PROGRAM;

fn products() -> (tempfile::TempDir, Connection) {
    products_with(PROGRAM)
}

fn products_with(program: &str) -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-TEST/program.xml", program.as_bytes()).unwrap();
    (dir, conn)
}

fn instance(k: u32, repeat: &str) -> ModuleInstance {
    ModuleInstance {
        id: ModuleInstanceId(k),
        device: DeviceId(1),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "MD-71_M-93".into(),
        },
        instance_ets_id: format!("MD-71_M-93_MI-{k}"),
        repeat_index: repeat.into(),
        arguments: vec![],
    }
}

#[test]
fn sibling_instance_values_are_not_stale_or_fed_to_the_shared_evaluator() {
    let (_dir, conn) = products();
    let stored = vec![
        ("A-TEST_MD-71_M-93_MI-2_P-83_R-61".into(), "17".into()),
        ("A-TEST_MD-71_M-93_MI-4_P-83_R-61".into(), "23".into()),
    ];
    let evaluation = evaluate_device(
        &conn,
        "A-TEST",
        stored,
        &[instance(2, "71x2"), instance(4, "71x4 93x1")],
    )
    .unwrap();
    assert!(
        evaluation.stale.is_empty(),
        "own stored instance values must not be stale"
    );
    assert!(
        evaluation.validated_scoped.is_empty(),
        "no sibling earns shared write/evaluation authority"
    );
    assert_eq!(evaluation.values.len_scoped(), 0);
    assert_eq!(
        evaluation.values.get_instance(
            "A-TEST_MD-71_M-93",
            "MD-71_M-93_MI-2",
            "A-TEST_MD-71_P-83_R-61"
        ),
        Some("17")
    );
    assert_eq!(
        evaluation.values.get_instance(
            "A-TEST_MD-71_M-93",
            "MD-71_M-93_MI-4",
            "A-TEST_MD-71_P-83_R-61"
        ),
        Some("23")
    );
    assert_eq!(
        evaluation.values.get_instance(
            "A-TEST_MD-71_M-93",
            "MD-71_M-93_MI-1",
            "A-TEST_MD-71_P-83_R-61"
        ),
        None
    );
    assert_eq!(
        evaluation.value_status("A-TEST_MD-71_M-93_MI-2_P-83_R-61"),
        knx_productdb::device_evaluation::ValueStatus::Unknown
    );
    assert!(evaluation
        .activation
        .parameter_refs
        .iter()
        .all(|r| r.scope.is_none()));
}

#[test]
fn duplicate_imported_identity_cannot_feed_a_scoped_value() {
    let direct = PROGRAM
        .replace("<Repeat ParameterRefId=\"P-TEST_R-1\">", "")
        .replace("</Repeat>", "");
    let (_dir, conn) = products_with(&direct);
    let evaluation = evaluate_device(
        &conn,
        "A-TEST",
        vec![("A-TEST_MD-71_M-93_MI-2_P-83_R-61".into(), "17".into())],
        &[instance(2, ""), instance(2, "uninterpreted")],
    )
    .unwrap();
    assert!(
        evaluation.validated_scoped.is_empty(),
        "duplicate identity must not feed shared evaluation"
    );
    assert_eq!(evaluation.stale.len(), 1);
}

#[test]
fn repeat_text_and_controller_defaults_never_select_a_stored_instance() {
    let (_dir, conn) = products();
    for repeat in ["", "71x1", "71x4 93x1", "missing-counter", "71x99"] {
        let evaluation = evaluate_device(
            &conn,
            "A-TEST",
            vec![("A-TEST_MD-71_M-93_MI-99_P-83_R-61".into(), "0017".into())],
            &[instance(99, repeat)],
        )
        .unwrap();
        assert_eq!(
            evaluation.values.get_instance(
                "A-TEST_MD-71_M-93",
                "MD-71_M-93_MI-99",
                "A-TEST_MD-71_P-83_R-61"
            ),
            Some("0017")
        );
        assert_eq!(
            evaluation.values.get_instance(
                "A-TEST_MD-71_M-93",
                "MD-71_M-93_MI-1",
                "A-TEST_MD-71_P-83_R-61"
            ),
            None
        );
        assert!(evaluation.validated_scoped.is_empty());
    }
}

#[test]
fn duplicate_stored_instance_rows_keep_first_and_report_the_later_raw_row() {
    let (_dir, conn) = products();
    let id = "A-TEST_MD-71_M-93_MI-2_P-83_R-61";
    let evaluation = evaluate_device(
        &conn,
        "A-TEST",
        vec![(id.into(), "17".into()), (id.into(), "23".into())],
        &[instance(2, "71x2")],
    )
    .unwrap();
    assert_eq!(
        evaluation.values.get_instance(
            "A-TEST_MD-71_M-93",
            "MD-71_M-93_MI-2",
            "A-TEST_MD-71_P-83_R-61"
        ),
        Some("17")
    );
    assert_eq!(evaluation.stale[0].raw, "23");
    assert!(matches!(
        evaluation.findings[0],
        knx_productdb::device_evaluation::EvaluationFinding::DuplicateModuleScopedValue { .. }
    ));
}

#[test]
fn malformed_instance_and_foreign_module_parameter_never_validate() {
    let (_dir, conn) = products();
    let mut malformed = instance(2, "71x2");
    malformed.instance_ets_id = "MD-71_M-93_MI-2tail".into();
    let evaluation = evaluate_device(
        &conn,
        "A-TEST",
        vec![("A-TEST_MD-71_M-93_MI-2_P-83_R-61".into(), "17".into())],
        &[malformed],
    )
    .unwrap();
    assert!(evaluation.instance_scoped_ids.is_empty());
    assert_eq!(evaluation.stale.len(), 1);
    let other = evaluate_device(
        &conn,
        "A-TEST",
        vec![("A-TEST_MD-72_M-93_MI-2_P-83_R-61".into(), "17".into())],
        &[instance(2, "71x2")],
    )
    .unwrap();
    assert!(other.instance_scoped_ids.is_empty());
    assert_eq!(other.stale.len(), 1);
}

#[test]
fn conflicting_module_declarations_cannot_authorize_instance_evidence() {
    let ambiguous = PROGRAM.replace(
        "</Repeat>",
        "</Repeat><Module Id=\"A-TEST_MD-71_M-93\" RefId=\"A-TEST_MD-72\"/>",
    );
    let (_dir, conn) = products_with(&ambiguous);
    let evaluation = evaluate_device(
        &conn,
        "A-TEST",
        vec![("A-TEST_MD-71_M-93_MI-2_P-83_R-61".into(), "17".into())],
        &[instance(2, "71x2")],
    )
    .unwrap();
    assert!(evaluation.instance_scoped_ids.is_empty());
    assert!(evaluation.validated_scoped.is_empty());
    assert_eq!(evaluation.stale.len(), 1);
}

#[test]
fn legacy_single_authority_keeps_its_declared_definition_alias() {
    let program = PROGRAM
        .replace("<Repeat ParameterRefId=\"P-TEST_R-1\">", "")
        .replace("</Repeat>", "")
        .replace(
            "ModuleDef Id=\"A-TEST_MD-71\"",
            "ModuleDef Id=\"A-TEST_MD-72\"",
        )
        .replace("RefId=\"A-TEST_MD-71\"", "RefId=\"A-TEST_MD-72\"");
    let (_dir, conn) = products_with(&program);
    let evaluation = evaluate_device(
        &conn,
        "A-TEST",
        vec![("A-TEST_MD-71_M-93_MI-2_P-83_R-61".into(), "17".into())],
        &[instance(2, "")],
    )
    .unwrap();
    assert_eq!(
        evaluation
            .validated_scoped
            .get(&("A-TEST_MD-71_M-93".into(), "A-TEST_MD-71_P-83_R-61".into()))
            .map(String::as_str),
        Some("17")
    );
    assert_eq!(
        evaluation.value_status("A-TEST_MD-71_M-93_MI-2_P-83_R-61"),
        knx_productdb::device_evaluation::ValueStatus::Active
    );
}
