//! Public synthetic regressions for declared parameter grids; fictional data only.
use knx_productdb::parameter_layout::{
    load, BlockGrid, Cell, GridIssue, GridIssueKind, GridStatus, ParameterLayout,
};
use knx_productdb::{ingest_file, open_and_migrate, Connection};

const PARAMETERS: [&str; 7] = ["P-1", "P-2", "P-3", "P-4", "P-5", "P-6", "P-7"];

fn source(dynamic: &str, module_dynamic: &str) -> String {
    let parameters: String = PARAMETERS
        .iter()
        .map(|id| {
            format!(
                r#"<Parameter Id="{id}" Text="Fictional {id}" ParameterType="PT-1" Value="0"/>"#
            )
        })
        .collect();
    let refs: String = PARAMETERS
        .iter()
        .map(|id| format!(r#"<ParameterRef Id="{id}_R-1" RefId="{id}"/>"#))
        .collect();
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1"><ApplicationPrograms><ApplicationProgram Id="A-1" Name="Synthetic grid" ApplicationVersion="1" MaskVersion="MV-0701"><Static>
<ParameterTypes><ParameterType Id="PT-1"><TypeRestriction SizeInBit="8" Base="Value"><Enumeration Id="E-0" Value="0" Text="Off"/><Enumeration Id="E-1" Value="1" Text="On"/></TypeRestriction></ParameterType></ParameterTypes>
<Parameters>{parameters}</Parameters><ParameterRefs>{refs}</ParameterRefs>
</Static><Dynamic>{dynamic}<Module Id="MOD-1_M-1" RefId="MD-1"/></Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="Fictional module"><Static><Parameters><Parameter Id="MD-1_P-1" Text="Module value" ParameterType="PT-1" Value="0"/></Parameters><ParameterRefs><ParameterRef Id="MD-1_P-1_R-1" RefId="MD-1_P-1"/></ParameterRefs></Static>
<Dynamic>{module_dynamic}</Dynamic></ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

struct Fixture {
    _dir: tempfile::TempDir,
    conn: Connection,
}

fn ingest(dynamic: &str, module_dynamic: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("catalogue.sqlite")).unwrap();
    ingest_file(
        &conn,
        "M-1/A-1.xml",
        source(dynamic, module_dynamic).as_bytes(),
    )
    .unwrap();
    Fixture { _dir: dir, conn }
}

fn grid_of<'a>(layout: &'a ParameterLayout, scope: &str, reference: &str) -> Option<&'a BlockGrid> {
    let group = layout.field(scope, reference).group.expect("grouped field");
    layout.grid(scope, group.node_id)
}

fn node_id(conn: &Connection, element: &str) -> i64 {
    conn.query_row(
        "SELECT node_id FROM dynamic_node WHERE program_id='A-1' AND element_id=?1",
        [element],
        |r| r.get(0),
    )
    .unwrap()
}

const TABLE: &str = r#"<ParameterBlock Id="PB-G" Text="Fictional matrix" Layout="Table">
<Rows><Row Id="PB-G_R-1" Text="Upper"/><Row Id="PB-G_R-2" Name="second" CollapseIfEmpty="true"/></Rows>
<Columns><Column Id="PB-G_C-1" Text="Left" Width="40%"/><Column Id="PB-G_C-2" Width="60%" TextParameterRefId="P-7_R-1"/></Columns>
<ParameterSeparator Id="S-1" Text="Pair caption" Cell="1,1"/>
<ParameterRefRef RefId="P-1_R-1" Cell="1,2"/>
<choose ParamRefId="P-1_R-1"><when test="1"><ParameterRefRef RefId="P-2_R-1" Cell="2,2"/></when></choose>
</ParameterBlock>"#;

#[test]
fn a_declared_table_reports_axes_cells_and_separator_placements_verbatim() {
    let fixture = ingest(TABLE, "");
    let layout = load(&fixture.conn, "A-1").unwrap();
    let grid = grid_of(&layout, "", "P-1_R-1").expect("declared grid");
    assert_eq!(grid.status, GridStatus::Grid);
    assert_eq!(grid.declared_layout.as_deref(), Some("Table"));
    let rows: Vec<_> = grid
        .rows
        .iter()
        .map(|r| {
            (
                r.id.as_str(),
                r.text.as_deref(),
                r.collapse_if_empty.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("PB-G_R-1", Some("Upper"), None),
            ("PB-G_R-2", None, Some("true"))
        ]
    );
    let columns: Vec<_> = grid
        .columns
        .iter()
        .map(|c| {
            (
                c.id.as_str(),
                c.text.as_deref(),
                c.width.as_deref(),
                c.text_parameter_ref_id.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        columns,
        [
            ("PB-G_C-1", Some("Left"), Some("40%"), None),
            ("PB-G_C-2", None, Some("60%"), Some("P-7_R-1"))
        ]
    );
    assert!(grid.issues.is_empty(), "{:?}", grid.issues);
    assert_eq!(
        layout.field("", "P-1_R-1").cell,
        Some(Cell { row: 1, column: 2 })
    );
    assert_eq!(
        layout.field("", "P-2_R-1").cell,
        Some(Cell { row: 2, column: 2 })
    );
    let separator = layout
        .separator("", node_id(&fixture.conn, "S-1"))
        .expect("placed separator");
    assert_eq!(separator.cell, Cell { row: 1, column: 1 });
    assert_eq!(
        separator.block_node,
        layout.field("", "P-1_R-1").group.unwrap().node_id
    );
}

#[test]
fn blocks_without_a_declared_grid_carry_no_geometry() {
    let dynamic = r#"<ParameterBlock Id="PB-1" Text="Plain"><ParameterRefRef RefId="P-1_R-1"/></ParameterBlock>
<ParameterBlock Id="PB-2" Text="Explicit list" Layout="List"><ParameterRefRef RefId="P-2_R-1" Cell="1,1"/></ParameterBlock>"#;
    let fixture = ingest(dynamic, "");
    let layout = load(&fixture.conn, "A-1").unwrap();
    for reference in ["P-1_R-1", "P-2_R-1"] {
        assert!(grid_of(&layout, "", reference).is_none(), "{reference}");
        assert_eq!(layout.field("", reference).cell, None, "{reference}");
    }
}

#[test]
fn an_unknown_layout_token_is_preserved_and_never_rendered_as_a_grid() {
    let dynamic = TABLE.replace(r#"Layout="Table""#, r#"Layout="Carousel""#);
    let fixture = ingest(&dynamic, "");
    let layout = load(&fixture.conn, "A-1").unwrap();
    let grid = grid_of(&layout, "", "P-1_R-1").expect("reported variant");
    assert_eq!(grid.status, GridStatus::UnknownLayout);
    assert_eq!(grid.declared_layout.as_deref(), Some("Carousel"));
    assert_eq!(grid.rows.len(), 2, "declared rows are preserved");
    assert_eq!(grid.columns.len(), 2, "declared columns are preserved");
    assert_eq!(layout.field("", "P-1_R-1").cell, None);
    assert!(layout
        .separator("", node_id(&fixture.conn, "S-1"))
        .is_none());
}

#[test]
fn incomplete_or_contradictory_grid_declarations_are_invalid() {
    for (name, dynamic) in [
        ("no columns", TABLE.replace(r#"<Columns><Column Id="PB-G_C-1" Text="Left" Width="40%"/><Column Id="PB-G_C-2" Width="60%" TextParameterRefId="P-7_R-1"/></Columns>"#, "")),
        ("empty rows", TABLE.replace(r#"<Row Id="PB-G_R-1" Text="Upper"/><Row Id="PB-G_R-2" Name="second" CollapseIfEmpty="true"/>"#, "")),
        ("rows without layout", TABLE.replace(r#" Layout="Table""#, "")),
        ("rows with list layout", TABLE.replace(r#"Layout="Table""#, r#"Layout="List""#)),
    ] {
        let fixture = ingest(&dynamic, "");
        let layout = load(&fixture.conn, "A-1").unwrap();
        let grid = grid_of(&layout, "", "P-1_R-1").unwrap_or_else(|| panic!("{name}: reported"));
        assert_eq!(grid.status, GridStatus::InvalidGrid, "{name}");
        assert_eq!(layout.field("", "P-1_R-1").cell, None, "{name}");
    }
}

#[test]
fn cell_problems_are_named_and_such_fields_keep_no_cell() {
    let dynamic = r#"<ParameterBlock Id="PB-G" Text="Fictional matrix" Layout="Grid">
<Rows><Row Id="PB-G_R-1"/><Row Id="PB-G_R-2"/></Rows><Columns><Column Id="PB-G_C-1" Width="50%"/><Column Id="PB-G_C-2" Width="50%"/></Columns>
<ParameterRefRef RefId="P-1_R-1" Cell="2,2"/>
<ParameterRefRef RefId="P-2_R-1" Cell="3,1"/>
<ParameterRefRef RefId="P-3_R-1" Cell="0,1"/>
<ParameterRefRef RefId="P-4_R-1" Cell="1;1"/>
<ParameterRefRef RefId="P-5_R-1"/>
<choose ParamRefId="P-1_R-1"><when test="0"><ParameterRefRef RefId="P-6_R-1" Cell="1,1"/></when><when test="1"><ParameterRefRef RefId="P-6_R-1" Cell="1,2"/></when></choose>
<choose ParamRefId="P-1_R-1"><when test="0"><ParameterRefRef RefId="P-7_R-1" Cell="2,1"/></when><when test="1"><ParameterRefRef RefId="P-7_R-1" Cell="2,1"/></when></choose>
<Button Id="B-1" Text="Fictional action" Cell="1,1" EventHandler="none"/>
</ParameterBlock>"#;
    let fixture = ingest(dynamic, "");
    let layout = load(&fixture.conn, "A-1").unwrap();
    let grid = grid_of(&layout, "", "P-1_R-1").expect("declared grid");
    assert_eq!(grid.status, GridStatus::Grid);
    assert_eq!(grid.declared_layout.as_deref(), Some("Grid"));
    let issue = |kind, element: &str, reference: &str, cell: Option<&str>| GridIssue {
        kind,
        element: element.to_owned(),
        reference: Some(reference.to_owned()),
        cell: cell.map(str::to_owned),
    };
    assert_eq!(
        grid.issues,
        [
            issue(
                GridIssueKind::CellOutOfRange,
                "ParameterRefRef",
                "P-2_R-1",
                Some("3,1")
            ),
            issue(
                GridIssueKind::CellOutOfRange,
                "ParameterRefRef",
                "P-3_R-1",
                Some("0,1")
            ),
            issue(
                GridIssueKind::CellMalformed,
                "ParameterRefRef",
                "P-4_R-1",
                Some("1;1")
            ),
            issue(
                GridIssueKind::MissingCell,
                "ParameterRefRef",
                "P-5_R-1",
                None
            ),
            issue(
                GridIssueKind::AmbiguousCell,
                "ParameterRefRef",
                "P-6_R-1",
                None
            ),
            issue(GridIssueKind::UnshownContent, "Button", "B-1", Some("1,1")),
        ]
    );
    assert_eq!(
        layout.field("", "P-1_R-1").cell,
        Some(Cell { row: 2, column: 2 })
    );
    assert_eq!(
        layout.field("", "P-7_R-1").cell,
        Some(Cell { row: 2, column: 1 }),
        "repeated equal cell"
    );
    for reference in ["P-2_R-1", "P-3_R-1", "P-4_R-1", "P-5_R-1", "P-6_R-1"] {
        assert_eq!(layout.field("", reference).cell, None, "{reference}");
    }
}

#[test]
fn module_definitions_carry_their_own_grids() {
    let module = r#"<ParameterBlock Id="MD-1_PB-1" Text="Module matrix" Layout="Table"><Rows><Row Id="MD-1_PB-1_R-1"/></Rows><Columns><Column Id="MD-1_PB-1_C-1" Width="100%"/></Columns><ParameterRefRef RefId="MD-1_P-1_R-1" Cell="1,1"/></ParameterBlock>"#;
    let fixture = ingest(
        r#"<ParameterBlock Id="PB-1" Text="Plain"><ParameterRefRef RefId="P-1_R-1"/></ParameterBlock>"#,
        module,
    );
    let layout = load(&fixture.conn, "A-1").unwrap();
    assert!(grid_of(&layout, "", "P-1_R-1").is_none());
    assert_eq!(
        grid_of(&layout, "MD-1", "MD-1_P-1_R-1").unwrap().status,
        GridStatus::Grid
    );
    assert_eq!(
        layout.field("MD-1", "MD-1_P-1_R-1").cell,
        Some(Cell { row: 1, column: 1 })
    );
}

#[test]
fn an_unreadable_source_reports_structural_grids_as_unavailable_not_absent() {
    let fixture = ingest(
        &format!(
            r#"{TABLE}<ParameterBlock Id="PB-1" Text="Plain"><ParameterRefRef RefId="P-3_R-1"/></ParameterBlock>"#
        ),
        "",
    );
    fixture
        .conn
        .execute("UPDATE source_file SET bytes = zeroblob(length(bytes))", [])
        .unwrap();
    let layout = load(&fixture.conn, "A-1").unwrap();
    let grid = grid_of(&layout, "", "P-1_R-1").expect("structural grid stays visible");
    assert_eq!(grid.status, GridStatus::SourceUnavailable);
    assert_eq!(grid.declared_layout, None);
    assert_eq!(layout.field("", "P-1_R-1").cell, None);
    assert!(
        grid_of(&layout, "", "P-3_R-1").is_none(),
        "no structure, no claim"
    );
}
