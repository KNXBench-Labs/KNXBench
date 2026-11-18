//! `inline-section-flatten-v1`: an `Inline` block's fields and separators
//! land in the outer block's group, and its own label is bound to that group
//! for a row-view heading. The inline block itself is never a group.
use knx_productdb::parameter_layout::load;
use knx_productdb::{ingest_file, open_and_migrate, Connection};

fn source(dynamic: &str) -> String {
    format!(
        r##"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-I"><ApplicationPrograms><ApplicationProgram Id="A-I" Name="Inline fixture" ApplicationVersion="1" MaskVersion="MV-091A"><Static>
<ParameterTypes><ParameterType Id="PT-1"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="9"/></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P-1" Text="Value one" ParameterType="PT-1" Value="1"/><Parameter Id="P-2" Text="Value two" ParameterType="PT-1" Value="2"/></Parameters>
<ParameterRefs><ParameterRef Id="P-1_R" RefId="P-1"/><ParameterRef Id="P-2_R" RefId="P-2"/></ParameterRefs>
</Static><Dynamic>{dynamic}</Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"##
    )
}

fn ingest(xml: &str) -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("catalogue.sqlite")).unwrap();
    ingest_file(&conn, "M-I/A-I.xml", xml.as_bytes()).unwrap();
    (dir, conn)
}

fn node_id(conn: &Connection, id: &str) -> i64 {
    conn.query_row(
        "SELECT node_id FROM dynamic_node WHERE program_id='A-I' AND module_def_id='' AND kind='ParameterBlock' AND element_id=?1",
        [id],
        |r| r.get(0),
    )
    .unwrap()
}

const INLINE: &str = r#"<ParameterBlock Id="OUTER" Text="Outer page" Layout="Grid"><Rows><Row Id="OR"/></Rows><Columns><Column Id="OC" Width="100%"/></Columns><ParameterRefRef RefId="P-1_R" Cell="1,1"/><ParameterBlock Id="INNER" Inline="true" Text="Inline section"><ParameterRefRef RefId="P-2_R" Cell="1,1"/><ParameterSeparator Id="S" Text="Inline note"/></ParameterBlock></ParameterBlock>"#;

#[test]
fn an_inline_blocks_children_belong_to_the_outer_group() {
    let (_dir, conn) = ingest(&source(INLINE));
    let layout = load(&conn, "A-I").unwrap();
    // Both fields carry the outer block as their group, not the inline one.
    for reference in ["P-1_R", "P-2_R"] {
        let group = layout
            .field("", reference)
            .group
            .expect("field has a group");
        assert_eq!(group.element_id.as_deref(), Some("OUTER"), "{reference}");
    }
    // The inline block's own label is bound to the outer group as a heading.
    let inner = node_id(&conn, "INNER");
    let heading = layout
        .inline_label_group("", inner)
        .expect("inline heading bound");
    assert_eq!(heading.element_id.as_deref(), Some("OUTER"));
    // The inline block is not a group for anything: no field resolves to it.
    assert!(layout
        .inline_label_group("", node_id(&conn, "OUTER"))
        .is_none());
}

#[test]
fn a_non_inline_nested_block_still_owns_its_children() {
    let dynamic = INLINE.replace(r#" Inline="true""#, "");
    let (_dir, conn) = ingest(&source(&dynamic));
    let layout = load(&conn, "A-I").unwrap();
    let group = layout.field("", "P-2_R").group.expect("group");
    assert_eq!(group.element_id.as_deref(), Some("INNER"));
    // And the nested block has no inline heading binding.
    assert!(layout
        .inline_label_group("", node_id(&conn, "INNER"))
        .is_none());
}

#[test]
fn an_inline_block_without_a_non_inline_ancestor_binds_no_heading() {
    let dynamic = r#"<ParameterBlock Id="SOLO" Inline="true" Text="Solo inline"><ParameterRefRef RefId="P-1_R"/></ParameterBlock>"#;
    let (_dir, conn) = ingest(&source(dynamic));
    let layout = load(&conn, "A-I").unwrap();
    assert!(layout
        .inline_label_group("", node_id(&conn, "SOLO"))
        .is_none());
    // Its field is ungrouped rather than owned by the inline block.
    assert!(layout.field("", "P-1_R").group.is_none());
}

#[test]
fn an_inline_spelling_other_than_true_stays_unmodelled() {
    for spelling in ["false", "0", "yes"] {
        let dynamic = INLINE.replace(r#"Inline="true""#, &format!(r#"Inline="{spelling}""#));
        let (_dir, conn) = ingest(&source(&dynamic));
        let layout = load(&conn, "A-I").unwrap();
        // Not inline: the nested block owns its child again, no heading binding.
        let group = layout.field("", "P-2_R").group.expect("group");
        assert_eq!(group.element_id.as_deref(), Some("INNER"), "{spelling}");
        assert!(layout
            .inline_label_group("", node_id(&conn, "INNER"))
            .is_none());
    }
}
