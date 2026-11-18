//! `nested-block-cell-passive-v1`: a program-owned nested `ParameterBlock`
//! placed by its own `@Cell` in a parent grid binds by structure and shows
//! its literal `@Text` as a passive cell caption. Display only.
use knx_productdb::parameter_layout::{load, NestedBinding};
use knx_productdb::{ingest_file, open_and_migrate, Connection};

fn source(dynamic: &str) -> String {
    source_with_extra("", dynamic)
}

fn source_with_extra(extra: &str, dynamic: &str) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-N"><ApplicationPrograms><ApplicationProgram Id="A-N" Name="Nested fixture" ApplicationVersion="1" MaskVersion="MV-091A"><Static>
<ParameterTypes><ParameterType Id="PT-1"><TypeRestriction SizeInBit="8" Base="Value"><Enumeration Id="E-0" Value="0" Text="Off"/><Enumeration Id="E-1" Value="1" Text="On"/></TypeRestriction></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P-1" Text="Mode" ParameterType="PT-1" Value="0"/></Parameters>
<ParameterRefs><ParameterRef Id="P-1_R-1" RefId="P-1"/></ParameterRefs>
</Static>{extra}<Dynamic>{dynamic}</Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn ingest(xml: &str) -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("catalogue.sqlite")).unwrap();
    ingest_file(&conn, "M-N/A-N.xml", xml.as_bytes()).unwrap();
    (dir, conn)
}

/// Stored program-owned `ParameterBlock` node ids in document order.
fn blocks(conn: &Connection) -> Vec<i64> {
    let mut stmt = conn
        .prepare(
            "SELECT node_id FROM dynamic_node WHERE program_id='A-N' AND module_def_id='' AND kind='ParameterBlock' ORDER BY node_id",
        )
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

const NESTED: &str = r#"<ParameterBlock Id="B" Layout="Grid"><Rows><Row Id="R"/></Rows><Columns><Column Id="C" Width="100%"/></Columns><ParameterRefRef RefId="P-1_R-1" Cell="1,1"/><ParameterBlock Id="N" Cell="1,1" Text="Nested caption"/></ParameterBlock>"#;

#[test]
fn a_nested_block_cell_binds_and_shows_its_literal_text() {
    let (_dir, conn) = ingest(&source(NESTED));
    let layout = load(&conn, "A-N").unwrap();
    assert_eq!(layout.nested_binding(), NestedBinding::Bound);
    let ids = blocks(&conn);
    let nested = ids[1];
    let cell = layout.nested_cell("", nested).expect("bound nested cell");
    assert_eq!(cell.element_id, "N");
    assert_eq!(cell.text.as_deref(), Some("Nested caption"));
    assert_eq!((cell.cell.row, cell.cell.column), (1, 1));
    assert_eq!(cell.group.element_id.as_deref(), Some("B"));
    // The parent block itself has no cell of its own.
    assert!(layout.nested_cell("", ids[0]).is_none());
}

#[test]
fn a_nested_block_without_cell_or_with_bad_cell_binds_nothing() {
    for dynamic in [
        // No Cell attribute: no candidate at all.
        NESTED.replace(
            r#" Cell="1,1" Text="Nested caption""#,
            r#" Text="Nested caption""#,
        ),
        // Out-of-range and malformed cells: candidate exists but binds no cell.
        NESTED.replace(
            r#"Cell="1,1" Text="Nested caption"/>"#,
            r#"Cell="9,9" Text="Nested caption"/>"#,
        ),
        NESTED.replace(
            r#"Cell="1,1" Text="Nested caption"/>"#,
            r#"Cell="x,1" Text="Nested caption"/>"#,
        ),
    ] {
        let (_dir, conn) = ingest(&source(&dynamic));
        let layout = load(&conn, "A-N").unwrap();
        assert_eq!(layout.nested_binding(), NestedBinding::Bound);
        assert!(blocks(&conn)
            .iter()
            .all(|node| layout.nested_cell("", *node).is_none()));
    }
}

#[test]
fn placeholder_or_blank_text_binds_the_cell_but_shows_no_caption() {
    for text in [
        r##"Text="{{arg}}""##,
        r##"Text="  ""##,
        r####"Text="#""####,
        "",
    ] {
        let nested = if text.is_empty() {
            NESTED.replace(r#" Text="Nested caption""#, "")
        } else {
            NESTED.replace(r#"Text="Nested caption""#, text)
        };
        let (_dir, conn) = ingest(&source(&nested));
        let layout = load(&conn, "A-N").unwrap();
        let cell = layout
            .nested_cell("", blocks(&conn)[1])
            .expect("cell binds regardless of caption");
        assert!(cell.text.is_none(), "text {text:?} must show no caption");
    }
}

#[test]
fn a_nested_block_in_a_list_block_binds_no_cell() {
    let list = r#"<ParameterBlock Id="B" Layout="List"><ParameterBlock Id="N" Cell="1,1" Text="Nested caption"><ParameterRefRef RefId="P-1_R-1"/></ParameterBlock></ParameterBlock>"#;
    let (_dir, conn) = ingest(&source(list));
    let layout = load(&conn, "A-N").unwrap();
    assert!(blocks(&conn)
        .iter()
        .all(|node| layout.nested_cell("", *node).is_none()));
}

#[test]
fn a_module_scoped_nested_block_is_never_a_candidate() {
    let module_defs = r#"<ModuleDefs><ModuleDef Id="MD-1" Name="Mod"><Dynamic><ParameterBlock Id="MB" Layout="Grid"><Rows><Row Id="MR"/></Rows><Columns><Column Id="MC" Width="100%"/></Columns><ParameterBlock Id="MN" Cell="1,1" Text="Module nested"/></ParameterBlock></Dynamic></ModuleDef></ModuleDefs>"#;
    let (_dir, conn) = ingest(&source_with_extra(module_defs, NESTED));
    let layout = load(&conn, "A-N").unwrap();
    assert_eq!(layout.nested_binding(), NestedBinding::Bound);
    // Only the program-owned nested block binds.
    let ids = blocks(&conn);
    let bound: Vec<i64> = ids
        .iter()
        .copied()
        .filter(|node| layout.nested_cell("", *node).is_some())
        .collect();
    assert_eq!(bound.len(), 1);
    assert_eq!(layout.nested_cell("", bound[0]).unwrap().element_id, "N");
}
