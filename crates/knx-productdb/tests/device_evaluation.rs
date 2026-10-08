//! Tests the stored-value evaluation shared by the parameter panel and the MCP adapter.
//!
//! `device_evaluation` (ADR-0090) and the `ValueStatus` classification the
//! adapter reports as visibility.

use knx_productdb::device_evaluation::{evaluate_device, EvaluationFinding, ValueStatus};
use rusqlite::Connection;

/// `P-1` (Mode, Restriction 0/1) shows `P-2` when 1 and `P-3` when 0. One
/// `Module` (`MOD-1_M-1`) declares `MOD-1_P-1` (Restriction 0/1), which in
/// turn shows `MOD-1_P-2` only when the module's own value is 1 — so a
/// module-scoped `choose` only resolves after the stored scoped values are
/// fed back (design D42).
const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
  <ParameterType Id="PT-Enum" Name="enum"><TypeRestriction Base="Value" SizeInBit="8">
    <Enumeration Id="PT-Enum_EN-0" Text="Off" Value="0" />
    <Enumeration Id="PT-Enum_EN-1" Text="On" Value="1" />
  </TypeRestriction></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Mode" Text="Mode" ParameterType="PT-Enum" Value="0" />
  <Parameter Id="P-2" Name="Dim" Text="Dim speed" ParameterType="PT-Num" Value="5" />
  <Parameter Id="P-3" Name="Sw" Text="Switch delay" ParameterType="PT-Num" Value="3" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-1_R-1" RefId="P-1" />
  <ParameterRef Id="P-2_R-1" RefId="P-2" />
  <ParameterRef Id="P-3_R-1" RefId="P-3" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="P-1_R-1" />
  <choose ParamRefId="P-1_R-1">
    <when test="1"><ParameterRefRef RefId="P-2_R-1" /></when>
    <when test="0"><ParameterRefRef RefId="P-3_R-1" /></when>
  </choose>
  <Module Id="MOD-1_M-1" RefId="MD-1" />
</Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="module">
<Static>
<Parameters>
  <Parameter Id="MOD-1_P-1" Name="Use" Text="Use channel" ParameterType="PT-Enum" Value="0" />
  <Parameter Id="MOD-1_P-2" Name="Time" Text="Channel time" ParameterType="PT-Num" Value="1" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="MOD-1_P-1_R-1" RefId="MOD-1_P-1" />
  <ParameterRef Id="MOD-1_P-2_R-1" RefId="MOD-1_P-2" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="MOD-1_P-1_R-1" />
  <choose ParamRefId="MOD-1_P-1_R-1">
    <when test="1"><ParameterRefRef RefId="MOD-1_P-2_R-1" /></when>
  </choose>
</Dynamic>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn products() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/program.xml", PROGRAM.as_bytes()).unwrap();
    (dir, conn)
}

fn stored(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(id, raw)| (id.to_string(), raw.to_string()))
        .collect()
}

#[test]
fn a_choose_decides_which_stored_values_are_active() {
    let (_dir, conn) = products();
    let dimming = evaluate_device(
        &conn,
        "A-1",
        stored(&[("P-1_R-1", "1"), ("P-2_R-1", "7"), ("P-3_R-1", "9")]),
        &[],
    )
    .unwrap();
    assert!(dimming.traversal_complete());
    assert_eq!(dimming.value_status("P-1_R-1"), ValueStatus::Active);
    assert_eq!(dimming.value_status("P-2_R-1"), ValueStatus::Active);
    assert_eq!(dimming.value_status("P-3_R-1"), ValueStatus::Inactive);

    // The same values with the mode switched flip the two branches.
    let switching = evaluate_device(
        &conn,
        "A-1",
        stored(&[("P-1_R-1", "0"), ("P-2_R-1", "7"), ("P-3_R-1", "9")]),
        &[],
    )
    .unwrap();
    assert_eq!(switching.value_status("P-2_R-1"), ValueStatus::Inactive);
    assert_eq!(switching.value_status("P-3_R-1"), ValueStatus::Active);
}

#[test]
fn unknown_and_duplicate_values_are_stale_and_the_first_row_counts() {
    let (_dir, conn) = products();
    let evaluation = evaluate_device(
        &conn,
        "A-1",
        stored(&[
            ("P-1_R-1", "1"),
            ("P-1_R-1", "0"),
            ("P-99_R-1", "4"),
            ("MOD-9_M-9_MI-1_P-1_R-1", "1"),
        ]),
        &[],
    )
    .unwrap();
    assert_eq!(evaluation.value_status("P-1_R-1"), ValueStatus::Active);
    assert_eq!(evaluation.value_status("P-99_R-1"), ValueStatus::Stale);
    assert_eq!(
        evaluation.value_status("MOD-9_M-9_MI-1_P-1_R-1"),
        ValueStatus::Stale
    );
    assert_eq!(
        evaluation.value_status("never-stored"),
        ValueStatus::Unknown
    );
    // The first stored "1" won: P-2 (shown for 1) is active.
    assert!(evaluation
        .activation
        .parameter_refs
        .iter()
        .any(|r| r.scope.is_none() && r.ref_id == "P-2_R-1"));
    assert!(evaluation.findings.iter().any(|f| matches!(
        f,
        EvaluationFinding::DuplicateUnscopedValue { ets_id, kept_raw }
            if ets_id == "P-1_R-1" && kept_raw == "1"
    )));
}

#[test]
fn a_module_scoped_value_is_judged_in_its_own_module_after_feedback() {
    let (_dir, conn) = products();
    let on = evaluate_device(
        &conn,
        "A-1",
        stored(&[
            ("MOD-1_M-1_MI-1_P-1_R-1", "1"),
            ("MOD-1_M-1_MI-1_P-2_R-1", "8"),
        ]),
        &[],
    )
    .unwrap();
    assert_eq!(
        on.scoped_ids.get("MOD-1_M-1_MI-1_P-2_R-1"),
        Some(&("MOD-1_M-1".to_string(), "MOD-1_P-2_R-1".to_string()))
    );
    assert_eq!(
        on.value_status("MOD-1_M-1_MI-1_P-1_R-1"),
        ValueStatus::Active
    );
    assert_eq!(
        on.value_status("MOD-1_M-1_MI-1_P-2_R-1"),
        ValueStatus::Active
    );

    let off = evaluate_device(
        &conn,
        "A-1",
        stored(&[
            ("MOD-1_M-1_MI-1_P-1_R-1", "0"),
            ("MOD-1_M-1_MI-1_P-2_R-1", "8"),
        ]),
        &[],
    )
    .unwrap();
    assert_eq!(
        off.value_status("MOD-1_M-1_MI-1_P-2_R-1"),
        ValueStatus::Inactive
    );
}
