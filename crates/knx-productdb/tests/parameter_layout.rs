//! Public synthetic regressions for structural parameter grouping; no vendor source.
use knx_productdb::{ingest_file, open_and_migrate, parameter_layout::load};

fn source(dynamic: &str) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1"><ApplicationPrograms><ApplicationProgram Id="A-1" Name="Synthetic" ApplicationVersion="1" MaskVersion="MV-0701"><Static>
<ParameterTypes><ParameterType Id="PT-1"><TypeRestriction SizeInBit="8" Base="Value"><Enumeration Id="E-0" Value="0" Text="Disabled"/><Enumeration Id="E-1" Value="1" Text="Enabled"/></TypeRestriction></ParameterType><ParameterType Id="PT-Label"><TypeNone/></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P-1" Text="Mode" ParameterType="PT-1" Value="0"/><Parameter Id="P-2" Text="Output" ParameterType="PT-1" Value="0"/><Parameter Id="P-L" Text="Canonical channel" ParameterType="PT-Label"/><Parameter Id="P-X" Text="Extension shadow" ParameterType="PT-Label"/></Parameters>
<ParameterRefs><ParameterRef Id="P-1_R-1" RefId="P-1"/><ParameterRef Id="P-2_R-1" RefId="P-2"/><ParameterRef Id="P-L_R-1" RefId="P-L"/><ParameterRef Id="P-X_R-1" RefId="P-X"/></ParameterRefs>
</Static><Dynamic>{dynamic}</Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn layout(xml: &str) -> knx_productdb::parameter_layout::ParameterLayout {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("catalogue.sqlite")).unwrap();
    ingest_file(&conn, "M-1/A-1.xml", xml.as_bytes()).unwrap();
    load(&conn, "A-1").unwrap()
}

#[test]
fn captions_ignore_foreign_prefixed_and_nested_lookalikes() {
    for shadow in [
        r#"<ext:ParameterBlock xmlns:ext="urn:extension" Id="PB-1" ParamRefId="P-X_R-1"/>"#,
        r#"<Extension><ParameterBlock Id="PB-1" ParamRefId="P-X_R-1"/></Extension>"#,
        r#"<ParameterBlock xmlns:ext="urn:extension" ext:Id="PB-1" ext:ParamRefId="P-X_R-1"/>"#,
    ] {
        let dynamic = format!(
            r#"<ParameterBlock Id="PB-1" ParamRefId="P-L_R-1"><ParameterRefRef RefId="P-1_R-1"/>{shadow}</ParameterBlock>"#
        );
        let result = layout(&source(&dynamic));
        assert_eq!(
            result
                .field("", "P-1_R-1")
                .group
                .unwrap()
                .caption_ref
                .as_deref(),
            Some("P-L_R-1"),
            "shadow={shadow}"
        );
    }
}

#[test]
fn nested_choice_controls_and_ambiguous_groups_are_structural() {
    let dynamic = r#"<ParameterBlock Id="PB-1" Text="General"><ParameterRefRef RefId="P-1_R-1"/><choose ParamRefId="P-1_R-1"><when test="1"><ParameterBlock Id="PB-2" Text="Channel"><ParameterRefRef RefId="P-2_R-1"/><choose ParamRefId="P-2_R-1"><when test="1"><ComObjectRefRef RefId="O-1_R-1"/></when></choose></ParameterBlock></when></choose></ParameterBlock><ParameterBlock Id="PB-3" Text="Other"><ParameterRefRef RefId="P-2_R-1"/></ParameterBlock>"#;
    let result = layout(&source(dynamic));
    assert!(result.field("", "P-1_R-1").controls_com_objects);
    assert!(result.field("", "P-2_R-1").controls_com_objects);
    assert_eq!(
        result.field("", "P-1_R-1").group.unwrap().text.as_deref(),
        Some("General")
    );
    assert!(result.field("", "P-2_R-1").group.is_none());
}
