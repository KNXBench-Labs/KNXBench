//! Offline System-2 field previews never grant memory-read or download admission.

use knx_core::IndividualAddress;
use knx_productdb::byte_order_probe::{plan_probe, preview_system2_probe};
use knx_productdb::image::{build_system2_image, ImageRequest};

const XML: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0000">
<ApplicationPrograms><ApplicationProgram Id="M-0000_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1"
ApplicationVersion="1" MaskVersion="MV-0025" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="OBJ" Address="1024" Size="8" />
<AbsoluteSegment Id="PAR" Address="15360" Size="256" /></Code>
<ParameterTypes>
<ParameterType Id="U16" Name="Number"><TypeNumber SizeInBit="16" Type="unsignedInt" minInclusive="0" maxInclusive="65535" /></ParameterType>
<ParameterType Id="U8" Name="Control"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255" /></ParameterType>
</ParameterTypes><Parameters>
<Parameter Id="P" Name="Time" ParameterType="U16" Value="300"><Memory CodeSegment="PAR" Offset="248" BitOffset="0" /></Parameter>
<Parameter Id="C" Name="Control" ParameterType="U8" Value="0"><Memory CodeSegment="PAR" Offset="250" BitOffset="0" /></Parameter>
</Parameters><ParameterRefs><ParameterRef Id="P_R" RefId="P" /><ParameterRef Id="C_R" RefId="C" /></ParameterRefs>
<ComObjectTable CodeSegment="OBJ" Offset="0" />
<LoadProcedures><LoadProcedure><LdCtrlConnect />
<LdCtrlTaskCtrl2 LsmIdx="3" Callback="0" Address="1024" Seg0="0" Seg1="0" />
<LdCtrlTaskSegment LsmIdx="5" Address="1024" /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>
</Static><Dynamic><Channel Id="CH" Text="Synthetic"><ParameterBlock Id="PB" Text="Block">
<ParameterRefRef RefId="P_R" /><ParameterRefRef RefId="C_R" />
</ParameterBlock></Channel></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn fixture(xml: &str) -> (tempfile::TempDir, knx_productdb::Connection, ImageRequest) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "synthetic.xml", xml.as_bytes()).unwrap();
    let request = ImageRequest {
        program_id: "M-0000_A-0001-01-0000".into(),
        individual_address: IndividualAddress::from_raw(0x1234),
        values: Default::default(),
        links: vec![],
        flag_overrides: Default::default(),
    };
    (dir, conn, request)
}

#[test]
fn selected_source_fields_reuse_evaluation_without_granting_live_probe_admission() {
    let (_dir, conn, request) = fixture(XML);
    let before = conn.total_changes();
    let preview = preview_system2_probe(&conn, &request, &["P_R".into(), "C_R".into()])
        .expect("bounded source preview");
    assert_eq!(preview.program_id, request.program_id);
    assert_eq!(preview.mask.0, 0x0025);
    assert_eq!(preview.manufacturer, 0);
    assert_eq!(preview.fields.len(), 2);
    assert_eq!(preview.fields[0].parameter_ref, "P_R");
    assert_eq!(preview.fields[0].address, 0x3CF8);
    assert_eq!(preview.fields[0].value, 300);
    assert_eq!(preview.fields[0].field.size_in_bit, 16);
    assert_eq!(preview.fields[1].address, 0x3CFA);
    assert_eq!(preview.fields[1].field.size_in_bit, 8);
    assert!(
        plan_probe(&conn, &request).is_err(),
        "live-probe admission unchanged"
    );
    assert_eq!(conn.total_changes(), before);
}

#[test]
fn explicit_selection_is_required_and_never_silently_drops_a_requested_field() {
    let (_dir, conn, request) = fixture(XML);
    for refs in [
        vec![],
        vec!["P_R".into(), "P_R".into()],
        vec!["absent".into()],
    ] {
        assert!(preview_system2_probe(&conn, &request, &refs).is_err());
    }
    let preview = preview_system2_probe(&conn, &request, &["P_R".into()]).unwrap();
    assert_eq!(preview.fields.len(), 1);
    assert!(preview.skipped.iter().any(|s| s.parameter_ref == "C_R"));
}

#[test]
fn symmetric_values_and_unresolved_metadata_do_not_become_empty_success() {
    let (_dir, conn, mut request) = fixture(XML);
    request.values.insert("P_R".into(), "0".into());
    assert!(preview_system2_probe(&conn, &request, &["P_R".into()]).is_err());
    for xml in [
        XML.replace(
            "Address=\"1024\" /><LdCtrlDisconnect",
            "Address=\"1025\" /><LdCtrlDisconnect",
        ),
        XML.replace("<LdCtrlDisconnect />", "<LdCtrlUnknown />"),
        XML.replace("MV-0025", "MV-0701"),
    ] {
        let (_dir, conn, request) = fixture(&xml);
        assert!(preview_system2_probe(&conn, &request, &["P_R".into()]).is_err());
    }
}

#[test]
fn selected_fields_cannot_cross_the_sixteen_bit_address_boundary() {
    let xml = XML.replace("Address=\"15360\"", "Address=\"65535\"");
    let (_dir, conn, request) = fixture(&xml);
    assert!(preview_system2_probe(&conn, &request, &["P_R".into()]).is_err());
    let xml = XML.replace("Address=\"15360\"", "Address=\"65285\"");
    let (_dir, conn, request) = fixture(&xml);
    let preview = preview_system2_probe(&conn, &request, &["C_R".into()]).unwrap();
    assert_eq!(preview.fields[0].address, 0xFFFF);
}

#[test]
fn the_explicit_selection_accepts_eight_distinct_fields_but_not_nine() {
    let parameters = (0..9).map(|n| format!(r#"<Parameter Id="N{n}" Name="N{n}" ParameterType="U8" Value="1"><Memory CodeSegment="PAR" Offset="{n}" BitOffset="0" /></Parameter>"#)).collect::<String>();
    let refs = (0..9)
        .map(|n| format!(r#"<ParameterRef Id="N{n}_R" RefId="N{n}" />"#))
        .collect::<String>();
    let active = (0..9)
        .map(|n| format!(r#"<ParameterRefRef RefId="N{n}_R" />"#))
        .collect::<String>();
    let xml = XML
        .replace("</Parameters>", &format!("{parameters}</Parameters>"))
        .replace("</ParameterRefs>", &format!("{refs}</ParameterRefs>"))
        .replace("</ParameterBlock>", &format!("{active}</ParameterBlock>"));
    let (_dir, conn, request) = fixture(&xml);
    let selected: Vec<_> = (0..8).map(|n| format!("N{n}_R")).collect();
    assert_eq!(
        preview_system2_probe(&conn, &request, &selected)
            .unwrap()
            .fields
            .len(),
        8
    );
    let mut too_many = selected;
    too_many.push("N8_R".into());
    assert!(preview_system2_probe(&conn, &request, &too_many).is_err());
}

#[test]
fn legacy_patch_remains_disclosed_and_the_image_guard_stays_in_place() {
    let xml = XML.replace(
        "ParameterType=\"U16\" Value=\"300\"",
        "ParameterType=\"U16\" Value=\"300\" LegacyPatchAlways=\"1\"",
    );
    let (_dir, conn, request) = fixture(&xml);
    let preview = preview_system2_probe(&conn, &request, &["P_R".into()]).unwrap();
    assert!(preview.has_uninterpreted_legacy_patch);
    let err = build_system2_image(&conn, &request)
        .unwrap_err()
        .to_string();
    assert!(err.contains("LegacyPatchAlways"), "{err}");
    assert!(plan_probe(&conn, &request).is_err());
}
