//! AR07: stored controller declarations, not numeric-looking values, bound choose semantics.
//!
//! Synthetic source only; no vendor data or claim of application-XSD validity.

use std::collections::HashMap;

use knx_productdb::dynamic::{evaluate, load_program_trees, Diagnostic, ValueMap};
use rusqlite::Connection;

const PROGRAM_ID: &str = "M-00FA_A-0009-10-ABCD";
const SOURCE_PATH: &str = "M-00FA/control.xml";

fn program(controller_type: &str) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/23">
<ManufacturerData><Manufacturer RefId="M-00FA"><ApplicationPrograms>
<ApplicationProgram Id="{PROGRAM_ID}" Name="Synthetic controls" ApplicationVersion="16" MaskVersion="MV-0701">
<Static><ParameterTypes>
  <ParameterType Id="PT-Control" Name="controller">{controller_type}</ParameterType>
  <ParameterType Id="PT-Output" Name="output"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255" /></ParameterType>
</ParameterTypes><Parameters>
  <Parameter Id="P-Control" Name="controller" ParameterType="PT-Control" Value="1" />
  <Parameter Id="P-Match" Name="matching" ParameterType="PT-Output" Value="0" />
  <Parameter Id="P-Default" Name="default" ParameterType="PT-Output" Value="0" />
  <Parameter Id="P-Outside" Name="outside" ParameterType="PT-Output" Value="0" />
</Parameters><ParameterRefs>
  <ParameterRef Id="P-Control_R-1" RefId="P-Control" />
  <ParameterRef Id="P-Match_R-1" RefId="P-Match" />
  <ParameterRef Id="P-Default_R-1" RefId="P-Default" />
  <ParameterRef Id="P-Outside_R-1" RefId="P-Outside" />
</ParameterRefs></Static>
<Dynamic>
  <choose ParamRefId="P-Control_R-1">
    <when test="1"><ParameterRefRef RefId="P-Match_R-1" /><ComObjectRefRef RefId="O-Match_R-1" /></when>
    <when default="true"><ParameterRefRef RefId="P-Default_R-1" /><ComObjectRefRef RefId="O-Default_R-1" /></when>
  </choose>
  <ParameterRefRef RefId="P-Outside_R-1" /><ComObjectRefRef RefId="O-Outside_R-1" />
</Dynamic></ApplicationProgram>
</ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn install(source: &str) -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, SOURCE_PATH, source.as_bytes()).unwrap();
    (dir, conn)
}

fn values(value: &str) -> ValueMap {
    HashMap::from([("P-Control_R-1".to_string(), value.to_string())]).into()
}

#[test]
fn unsupported_stored_controller_kinds_never_activate_a_choice_or_its_default() {
    for (type_xml, expected_kind) in [
        ("<TypeText SizeInBit=\"8\" />", "Text"),
        ("<TypeFloat Encoding=\"DPT 9\" />", "Float"),
        ("<TypeTime SizeInBit=\"8\" />", "Time"),
        ("<TypeIPAddress />", "IPAddress"),
        ("<TypePicture />", "Picture"),
        ("<TypeRawData />", "Raw"),
        ("<TypeColor Space=\"RGB\" />", "Color"),
        ("<TypeFutureControl />", "Other"),
    ] {
        let source = program(type_xml);
        let (dir, conn) = install(&source);
        let stored_kind: String = conn
            .query_row(
                "SELECT kind FROM parameter_type WHERE program_id = ?1 AND id = 'PT-Control'",
                [PROGRAM_ID],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_kind, expected_kind);
        let trees = load_program_trees(&conn, PROGRAM_ID).unwrap();
        for value in ["1", "0"] {
            let activation = evaluate(&trees, &values(value));
            assert_eq!(activation.diagnostics.len(), 5);
            assert!(activation.diagnostics[0].scope.is_none());
            assert!(matches!(
                &activation.diagnostics[0].diagnostic,
                Diagnostic::UnsupportedControlKind { param_ref: Some(reference), kind, .. }
                    if reference == "P-Control_R-1" && kind == expected_kind
            ));
            let parameters: Vec<_> = activation
                .parameter_refs
                .iter()
                .map(|r| r.ref_id.as_str())
                .collect();
            let objects: Vec<_> = activation
                .com_object_refs
                .iter()
                .map(|r| r.ref_id.as_str())
                .collect();
            assert_eq!(
                parameters,
                ["P-Outside_R-1"],
                "controller kind {expected_kind}"
            );
            assert_eq!(objects, ["O-Outside_R-1"]);
            assert!(activation
                .diagnostics
                .iter()
                .any(|d| d.diagnostic.may_hide_refs()));
            assert!(activation.diagnostics.iter().all(|d| !matches!(
                d.diagnostic,
                Diagnostic::UnresolvedParamRef { .. } | Diagnostic::NonNumericValue { .. }
            )));
            let skipped: Vec<_> = activation
                .diagnostics
                .iter()
                .filter_map(|d| match &d.diagnostic {
                    Diagnostic::RefBelowSkippedNode { ref_id, .. } => ref_id.as_deref(),
                    _ => None,
                })
                .collect();
            assert_eq!(
                skipped,
                [
                    "P-Match_R-1",
                    "O-Match_R-1",
                    "P-Default_R-1",
                    "O-Default_R-1"
                ]
            );
        }
        drop(conn);
        let reopened =
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let original = knx_productdb::load_source_file(
            &reopened,
            &knx_productdb::sha256_hex(source.as_bytes()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(original, source.as_bytes());
    }
}

#[test]
fn number_and_restriction_controllers_keep_their_matching_and_default_behavior() {
    for type_xml in [
        "<TypeNumber SizeInBit=\"8\" Type=\"unsignedInt\" minInclusive=\"0\" maxInclusive=\"255\" />",
        "<TypeRestriction Base=\"Value\" SizeInBit=\"8\"><Enumeration Id=\"EN-0\" Value=\"0\" Text=\"Off\" /><Enumeration Id=\"EN-1\" Value=\"1\" Text=\"On\" /></TypeRestriction>",
    ] {
        let (_dir, conn) = install(&program(type_xml));
        let trees = load_program_trees(&conn, PROGRAM_ID).unwrap();
        for (value, parameter, object) in [
            ("1", "P-Match_R-1", "O-Match_R-1"),
            ("0", "P-Default_R-1", "O-Default_R-1"),
        ] {
            let activation = evaluate(&trees, &values(value));
            assert_eq!(activation.parameter_refs.iter().map(|r| r.ref_id.as_str()).collect::<Vec<_>>(), [parameter, "P-Outside_R-1"]);
            assert_eq!(activation.com_object_refs.iter().map(|r| r.ref_id.as_str()).collect::<Vec<_>>(), [object, "O-Outside_R-1"]);
            assert!(activation.diagnostics.is_empty());
        }
    }
}

#[test]
fn a_missing_controller_declaration_remains_distinct_from_an_unsupported_kind() {
    let source = program("<TypeText SizeInBit=\"8\" />").replace(
        "<choose ParamRefId=\"P-Control_R-1\">",
        "<choose ParamRefId=\"P-Missing_R-1\">",
    );
    let (_dir, conn) = install(&source);
    let activation = evaluate(
        &load_program_trees(&conn, PROGRAM_ID).unwrap(),
        &values("1"),
    );
    assert!(matches!(
        activation.diagnostics[0].diagnostic,
        Diagnostic::UnresolvedParamRef { .. }
    ));
    assert_eq!(activation.parameter_refs.len(), 1);
    assert_eq!(activation.parameter_refs[0].ref_id, "P-Outside_R-1");
    assert_eq!(activation.com_object_refs.len(), 1);
}

#[test]
fn an_unsupported_choice_without_descendant_refs_still_marks_activation_uncertain() {
    let mut source = program("<TypeText SizeInBit=\"8\" />");
    for reference in [
        "<ParameterRefRef RefId=\"P-Match_R-1\" />",
        "<ComObjectRefRef RefId=\"O-Match_R-1\" />",
        "<ParameterRefRef RefId=\"P-Default_R-1\" />",
        "<ComObjectRefRef RefId=\"O-Default_R-1\" />",
    ] {
        source = source.replace(reference, "");
    }
    let (_dir, conn) = install(&source);
    let activation = evaluate(
        &load_program_trees(&conn, PROGRAM_ID).unwrap(),
        &values("1"),
    );
    assert_eq!(activation.diagnostics.len(), 1);
    assert!(matches!(
        activation.diagnostics[0].diagnostic,
        Diagnostic::UnsupportedControlKind { .. }
    ));
    assert!(activation.diagnostics[0].diagnostic.may_hide_refs());
}

#[test]
fn nested_unsupported_controllers_keep_both_full_module_ancestor_chains() {
    let base = program("<TypeText SizeInBit=\"8\" />");
    let (before, tail) = base.split_once("<Dynamic>").unwrap();
    let (body, after) = tail.split_once("</Dynamic>").unwrap();
    let source = format!(
        r#"{before}<Dynamic>
<Module Id="Outer_M-1" RefId="MD-Outer" />
<Module Id="Outer_M-2" RefId="MD-Outer" />
</Dynamic><ModuleDefs>
<ModuleDef Id="MD-Outer" Name="outer"><Dynamic>
<Module Id="Inner_M-1" RefId="MD-Inner" />
</Dynamic></ModuleDef>
<ModuleDef Id="MD-Inner" Name="inner"><Dynamic>{body}</Dynamic></ModuleDef>
</ModuleDefs>{after}"#
    );
    let (_dir, conn) = install(&source);
    let activation = evaluate(
        &load_program_trees(&conn, PROGRAM_ID).unwrap(),
        &values("1"),
    );
    let unsupported: Vec<_> = activation
        .diagnostics
        .iter()
        .filter(|d| matches!(d.diagnostic, Diagnostic::UnsupportedControlKind { .. }))
        .collect();
    assert_eq!(unsupported.len(), 2);
    let chains: Vec<_> = unsupported
        .iter()
        .map(|d| d.scope.as_ref().unwrap().node_chain())
        .collect();
    assert_eq!(chains[0].len(), 2);
    assert_eq!(chains[1].len(), 2);
    assert_ne!(chains[0], chains[1]);
    assert!(unsupported.iter().all(|d| {
        matches!(&d.diagnostic, Diagnostic::UnsupportedControlKind { kind, .. } if kind == "Text")
    }));
    assert_eq!(activation.parameter_refs.len(), 2);
    assert!(activation
        .parameter_refs
        .iter()
        .all(|r| r.ref_id == "P-Outside_R-1"));
    assert_eq!(activation.com_object_refs.len(), 2);
    assert!(activation
        .com_object_refs
        .iter()
        .all(|r| r.ref_id == "O-Outside_R-1"));
    assert_eq!(
        activation
            .diagnostics
            .iter()
            .filter(|d| matches!(d.diagnostic, Diagnostic::RefBelowSkippedNode { .. }))
            .count(),
        8
    );
}
