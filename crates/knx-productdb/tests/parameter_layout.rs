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

#[test]
fn every_separator_declares_its_group_hint_and_own_access() {
    let dynamic = r#"<ParameterSeparator Id="PS-0" Text="Outside"/><ParameterBlock Id="PB-1" Text="General"><ParameterSeparator Id="PS-1" Text="Read this" UIHint="Information"/><ParameterRefRef RefId="P-1_R-1"/><ParameterSeparator Id="PS-2" UIHint="HorizontalRuler" Access="None"/></ParameterBlock>"#;
    let result = layout(&source(dynamic));
    let node = |id: &str| {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("catalogue.sqlite")).unwrap();
        ingest_file(&conn, "M-1/A-1.xml", source(dynamic).as_bytes()).unwrap();
        conn.query_row(
            "SELECT node_id FROM dynamic_node WHERE element_id = ?1",
            [id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
    };
    let outside = result.separator_declaration("", node("PS-0")).unwrap();
    assert_eq!(outside.group, None);
    assert_eq!(
        (outside.ui_hint.as_deref(), outside.access.as_deref()),
        (None, None)
    );
    let info = result.separator_declaration("", node("PS-1")).unwrap();
    assert_eq!(info.element_id.as_deref(), Some("PS-1"));
    assert_eq!(info.ui_hint.as_deref(), Some("Information"));
    assert_eq!(
        info.group.as_ref().unwrap().text.as_deref(),
        Some("General")
    );
    // The same group the block's field gets.
    assert_eq!(info.group, result.field("", "P-1_R-1").group);
    let ruler = result.separator_declaration("", node("PS-2")).unwrap();
    assert_eq!(
        (ruler.ui_hint.as_deref(), ruler.access.as_deref()),
        (Some("HorizontalRuler"), Some("None"))
    );
    // A separator is not a field, and a node that is no separator has none.
    assert!(result.separator_declaration("", 0).is_none());
}

#[test]
fn object_controls_follow_dependencies_without_crossing_unrelated_branches() {
    let result = layout(&source(
        r#"<ParameterRefRef RefId="P-1_R-1"/><choose ParamRefId="P-1_R-1"><when test="1"><ParameterRefRef RefId="P-2_R-1"/></when></choose><choose ParamRefId="P-2_R-1"><when test="1"><ComObjectRefRef RefId="O-left"/><ComObjectRefRef RefId="O-left"/></when></choose><ParameterRefRef RefId="P-L_R-1"/><choose ParamRefId="P-L_R-1"><when test="1"><ComObjectRefRef RefId="O-right"/></when></choose>"#,
    ));
    assert_eq!(
        result.field("", "P-1_R-1").controlled_object_refs,
        vec!["O-left"]
    );
    assert_eq!(
        result.field("", "P-2_R-1").controlled_object_refs,
        vec!["O-left"]
    );
    assert_eq!(
        result.field("", "P-L_R-1").controlled_object_refs,
        vec!["O-right"]
    );
    assert!(result
        .field("missing-module", "P-2_R-1")
        .controlled_object_refs
        .is_empty());
}
