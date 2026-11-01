//! `button-literal-passive-v1`: literal Button labels bound to stored nodes by structure.
use std::collections::HashMap;

use knx_productdb::dynamic::{
    evaluate, load_program_trees, resolve_values, Activation, Diagnostic,
};
use knx_productdb::parameter_layout::{load, ButtonBinding, Cell, ParameterLayout};
use knx_productdb::{ingest_file, open_and_migrate, Connection};

const EN: &str = include_str!("../../fixtures/passive-button-literal.en.xml");
const DE: &str = include_str!("../../fixtures/passive-button-literal.de.xml");
const ONLY: &str = include_str!("../../fixtures/passive-button-only.xml");
const B_ONLY: &str = r#"<Button Id="B-ONLY" Text="Fictional source label" EventHandlerOnline="ConnectionOriented" EventHandler="fictionalCallback" EventHandlerParameters="{ &quot;fixture&quot;: 7 }" Cell="1,2"/>"#;

struct Fixture {
    dir: tempfile::TempDir,
    conn: Connection,
}

fn ingest(xml: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("catalogue.sqlite")).unwrap();
    ingest_file(&conn, "M-FIX/A-FIX.xml", xml.as_bytes()).unwrap();
    Fixture { dir, conn }
}

/// Stored program-owned `Button` node ids in document order.
fn buttons(conn: &Connection, program: &str) -> Vec<i64> {
    let mut stmt = conn
        .prepare("SELECT node_id FROM dynamic_node WHERE program_id=?1 AND module_def_id='' AND kind='Button' ORDER BY node_id")
        .unwrap();
    stmt.query_map([program], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// `(element id, text, cell, group block id)` of every bound label, in node order.
fn labels(layout: &ParameterLayout, nodes: &[i64]) -> Vec<(String, String, Cell, String)> {
    nodes
        .iter()
        .filter_map(|node| layout.button("", *node))
        .map(|literal| {
            (
                literal.element_id.clone(),
                literal.text.clone(),
                literal.cell,
                literal.group.element_id.clone().unwrap_or_default(),
            )
        })
        .collect()
}

fn bound(xml: &str) -> Vec<(String, String, Cell, String)> {
    let fixture = ingest(xml);
    let layout = load(&fixture.conn, "A-FIX").unwrap();
    labels(&layout, &buttons(&fixture.conn, "A-FIX"))
}

const C12: Cell = Cell { row: 1, column: 2 };

fn expected_en() -> Vec<(String, String, Cell, String)> {
    vec![
        (
            "B-ONLY".into(),
            "Fictional source label".into(),
            C12,
            "PB-ONLY".into(),
        ),
        (
            "B-MIX".into(),
            "Fictional mixed label".into(),
            C12,
            "PB-MIX".into(),
        ),
    ]
}

fn activation(conn: &Connection, supplied: &[(&str, &str)]) -> Activation {
    let trees = load_program_trees(conn, "A-FIX").unwrap();
    let supplied: HashMap<String, String> = supplied
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let values = resolve_values(conn, "A-FIX", &supplied).unwrap();
    evaluate(&trees, &values)
}

#[test]
fn stored_buttons_keep_null_identity_yet_bind_literal_text_cell_and_nearest_grid() {
    let fixture = ingest(EN);
    let nodes = buttons(&fixture.conn, "A-FIX");
    assert_eq!(nodes.len(), 2);
    for node in &nodes {
        let (id, text): (Option<String>, Option<String>) = fixture
            .conn
            .query_row(
                "SELECT element_id, text FROM dynamic_node WHERE program_id='A-FIX' AND module_def_id='' AND node_id=?1",
                [node],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((id, text), (None, None), "no migration or reimport needed");
    }
    let layout = load(&fixture.conn, "A-FIX").unwrap();
    assert_eq!(layout.button_binding(), ButtonBinding::Bound);
    assert_eq!(labels(&layout, &nodes), expected_en());
    // Nearest block, not the captioned outer page; no text of its own.
    let only = layout.button("", nodes[0]).unwrap();
    assert_eq!(only.group.text, None);
    assert_eq!(
        layout.grid("", only.group.node_id).unwrap().columns.len(),
        2
    );
    // Fields and their cells are untouched.
    assert_eq!(layout.field("", "PR-EDIT").cell, Some(C12));
    assert_eq!(
        layout.field("", "PR-OTHER").cell,
        Some(Cell { row: 2, column: 1 })
    );
    assert_eq!(layout.field("", "PR-MODE").cell, None);
    assert!(layout.button("MD-ANY", nodes[0]).is_none(), "program only");
}

#[test]
fn an_alternate_source_package_keeps_its_own_literals() {
    let texts: Vec<String> = bound(DE).into_iter().map(|label| label.1).collect();
    assert_eq!(
        texts,
        [
            "Fiktive Quellbeschriftung",
            "Fiktive gemischte Beschriftung"
        ]
    );
}

#[test]
fn an_existing_database_reopened_binds_the_same_labels() {
    let fixture = ingest(EN);
    let path = fixture.dir.path().join("catalogue.sqlite");
    drop(fixture.conn);
    let reopened = knx_productdb::open_read_only(&path).unwrap().conn;
    let layout = load(&reopened, "A-FIX").unwrap();
    assert_eq!(labels(&layout, &buttons(&reopened, "A-FIX")), expected_en());
}

#[test]
fn foreign_siblings_count_structurally_and_never_shift_a_label() {
    let xml = EN
        .replace(
            r#"<Rows><Row Id="ONLY-R1"/>"#,
            r#"<x:Note xmlns:x="urn:fixture"><x:Inner/></x:Note><Rows><Row Id="ONLY-R1"/>"#,
        )
        .replace(
            r#"<ParameterSeparator Id="S-MIX""#,
            r#"<x:Button xmlns:x="urn:fixture" Id="B-SHADOW" Text="Shadow" Cell="1,1"/><ParameterSeparator Id="S-MIX""#,
        );
    let fixture = ingest(&xml);
    let layout = load(&fixture.conn, "A-FIX").unwrap();
    let nodes = buttons(&fixture.conn, "A-FIX");
    assert_eq!(nodes.len(), 3, "the foreign Button is a stored node too");
    assert_eq!(labels(&layout, &nodes), expected_en());
    assert!(layout.button("", nodes[1]).is_none(), "foreign shadow");
}

#[test]
fn stored_structure_drift_refuses_every_label() {
    for mutation in [
        "UPDATE dynamic_node SET position=position+100 WHERE program_id='A-FIX' AND module_def_id='' AND kind='Button'",
        "UPDATE dynamic_node SET kind='Separator' WHERE program_id='A-FIX' AND module_def_id='' AND kind='Button' AND node_id=(SELECT MIN(node_id) FROM dynamic_node WHERE kind='Button')",
        "DELETE FROM dynamic_node WHERE program_id='A-FIX' AND module_def_id='' AND kind='Row'",
        // Same shape, but the nearest block's stored Id no longer corroborates the source.
        "UPDATE dynamic_node SET element_id='PB-ELSEWHERE' WHERE program_id='A-FIX' AND module_def_id='' AND element_id='PB-ONLY'",
    ] {
        let fixture = ingest(EN);
        let nodes = buttons(&fixture.conn, "A-FIX");
        fixture.conn.execute(mutation, []).unwrap();
        let layout = load(&fixture.conn, "A-FIX").unwrap();
        assert_eq!(
            layout.button_binding(),
            ButtonBinding::StructuralDrift,
            "{mutation}"
        );
        assert_eq!(labels(&layout, &nodes), [], "{mutation}");
    }
}

#[test]
fn a_changed_source_hash_binds_nothing() {
    let fixture = ingest(EN);
    let nodes = buttons(&fixture.conn, "A-FIX");
    fixture
        .conn
        .execute(
            "UPDATE source_file SET bytes=CAST(replace(CAST(bytes AS TEXT),'Fictional source label','Fictional source lab3l') AS BLOB)",
            [],
        )
        .unwrap();
    let layout = load(&fixture.conn, "A-FIX").unwrap();
    assert_eq!(layout.button_binding(), ButtonBinding::SourceUnavailable);
    assert_eq!(labels(&layout, &nodes), []);
}

#[test]
fn duplicate_ids_and_duplicate_roots_refuse_every_label() {
    let duplicate = EN.replace(r#"Id="B-MIX""#, r#"Id="B-ONLY""#);
    let fixture = ingest(&duplicate);
    let layout = load(&fixture.conn, "A-FIX").unwrap();
    assert_eq!(layout.button_binding(), ButtonBinding::DuplicateButtonId);
    assert_eq!(labels(&layout, &buttons(&fixture.conn, "A-FIX")), []);

    // A namespace-foreign `Dynamic` is still a root to the stored-tree parser,
    // which refuses the second tree outright; the reader's own refusal of a
    // second root is the `button` unit test.
    let shadow_root = EN.replace(
        "</Dynamic>",
        r#"</Dynamic><x:Dynamic xmlns:x="urn:fixture"><x:Button Id="B-X" Text="X" Cell="1,1"/></x:Dynamic>"#,
    );
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("catalogue.sqlite")).unwrap();
    assert!(ingest_file(&conn, "M-FIX/A-FIX.xml", shadow_root.as_bytes()).is_err());
}

#[test]
fn another_program_with_equal_ids_binds_only_its_own_labels() {
    let start = EN.find("<ApplicationProgram ").unwrap();
    let end = EN.find("</ApplicationProgram>").unwrap() + "</ApplicationProgram>".len();
    let other = EN[start..end]
        .replace(r#"Id="A-FIX""#, r#"Id="A-OTHER""#)
        .replace("Fictional source label", "Other program label");
    let xml = format!("{}{}{}", &EN[..end], other, &EN[end..]);
    let fixture = ingest(&xml);
    let own = load(&fixture.conn, "A-FIX").unwrap();
    assert_eq!(
        labels(&own, &buttons(&fixture.conn, "A-FIX")),
        expected_en()
    );
    let theirs = load(&fixture.conn, "A-OTHER").unwrap();
    assert_eq!(
        labels(&theirs, &buttons(&fixture.conn, "A-OTHER"))[0].1,
        "Other program label"
    );
}

#[test]
fn a_self_closing_module_definition_before_the_program_tree_changes_no_scope() {
    let xml = EN.replace(
        "<Dynamic>",
        r#"<ModuleDefs><ModuleDef Id="MD-E"/></ModuleDefs><Dynamic>"#,
    );
    let fixture = ingest(&xml);
    let layout = load(&fixture.conn, "A-FIX").unwrap();
    assert_eq!(layout.button_binding(), ButtonBinding::Bound);
    assert_eq!(
        labels(&layout, &buttons(&fixture.conn, "A-FIX")),
        expected_en()
    );
}

#[test]
fn a_byte_order_mark_and_non_element_siblings_never_shift_numbering() {
    let plain = buttons(&ingest(EN).conn, "A-FIX");
    let bom = format!("\u{FEFF}{EN}");
    let noisy = EN
        .replace(
            "<Dynamic>",
            "<Dynamic><!-- note --><?fixture pi?><![CDATA[ data ]]>",
        )
        .replace(
            r#"<Rows><Row Id="ONLY-R1"/>"#,
            r#"<!-- c --><?fixture p?><![CDATA[x]]><Rows><!-- r --><Row Id="ONLY-R1"/>"#,
        );
    for xml in [bom, noisy] {
        let fixture = ingest(&xml);
        let nodes = buttons(&fixture.conn, "A-FIX");
        assert_eq!(nodes, plain, "stored numbering unchanged");
        let layout = load(&fixture.conn, "A-FIX").unwrap();
        assert_eq!(layout.button_binding(), ButtonBinding::Bound);
        assert_eq!(labels(&layout, &nodes), expected_en());
    }
}

#[test]
fn only_the_exact_literal_form_is_admitted() {
    let variant = |button: &str| ONLY.replace(B_ONLY, button);
    let admitted = |button: &str| bound(&variant(button));
    // Surrounding whitespace on a non-blank text and decoded markup stay literal.
    assert_eq!(
        admitted(r#"<Button Id="B-ONLY" Text=" Spaced label " Cell="1,2"/>"#)[0].1,
        " Spaced label "
    );
    assert_eq!(
        admitted(
            r#"<Button Id="B-ONLY" Text="&lt;b&gt;Fictional&lt;/b&gt; &amp; note" Cell="1,2"/>"#
        )[0]
        .1,
        "<b>Fictional</b> & note"
    );
    assert_eq!(
        admitted(r#"<Button xmlns:y="urn:decl" Id="B-ONLY" Text="Declared" Cell="1,2"/>"#).len(),
        1,
        "a namespace declaration is no behaviour attribute"
    );
    // A character reference survives attribute-value normalization as itself.
    assert_eq!(
        admitted(r#"<Button Id="B-ONLY" Text="Line&#10;two" Cell="1,2"/>"#)[0].1,
        "Line\ntwo"
    );
    // Whitespace, comments and processing instructions are no content.
    assert_eq!(
        admitted("<Button Id=\"B-ONLY\" Text=\"Spaced\" Cell=\"1,2\">\n  <!-- note --><?pi x?> </Button>").len(),
        1
    );
    for refused in [
        r#"<Button Id="B-ONLY" Cell="1,2"/>"#,
        r#"<Button Id="B-ONLY" Text="" Cell="1,2"/>"#,
        r#"<Button Id="B-ONLY" Text="   " Cell="1,2"/>"#,
        r##"<Button Id="B-ONLY" Text="#" Cell="1,2"/>"##,
        r#"<Button Id="B-ONLY" Text="Value {{0}}" Cell="1,2"/>"#,
        r#"<Button Id="B-ONLY" Text="Half }} open" Cell="1,2"/>"#,
        r#"<Button Id="B-ONLY" Text="Param" TextParameterRefId="PR-EDIT" Cell="1,2"/>"#,
        r#"<Button Id="B-ONLY" Text="Local" Access="ReadWrite" Cell="1,2"/>"#,
        r#"<Button Id="B-ONLY" Text="Odd" Flavor="x" Cell="1,2"/>"#,
        r#"<Button xmlns:x="urn:x" Id="B-ONLY" x:Text="Qualified" Cell="1,2"/>"#,
        r#"<Button Text="No id" Cell="1,2"/>"#,
        r#"<Button Id="B-ONLY" Text="No cell"/>"#,
        r#"<Button Id="B-ONLY" Text="Malformed" Cell="1;2"/>"#,
        r#"<Button Id="B-ONLY" Text="Zero" Cell="0,1"/>"#,
        r#"<Button Id="B-ONLY" Text="Outside" Cell="1,3"/>"#,
        r#"<Button Id="B-ONLY" Text="Parent" Cell="1,2"><ParameterRefRef RefId="PR-OTHER"/></Button>"#,
        // Character data is content too: no leaf form.
        r#"<Button Id="B-ONLY" Text="Worded" Cell="1,2">words</Button>"#,
        r#"<Button Id="B-ONLY" Text="Data" Cell="1,2"><![CDATA[data]]></Button>"#,
        r#"<Button Id="B-ONLY" Text="Entity" Cell="1,2">&amp;</Button>"#,
    ] {
        assert_eq!(admitted(refused), [], "{refused}");
    }
}

#[test]
fn nearest_block_ownership_is_structural() {
    // The nearest block of a Button in a list block inside a grid is that list block.
    let list_inside = ONLY.replace(
        B_ONLY,
        &format!(r#"<ParameterBlock Id="PB-LIST" Text="Inner list">{B_ONLY}</ParameterBlock>"#),
    );
    assert_eq!(bound(&list_inside), []);
    // A foreign ancestor is no traversable Dynamic container.
    let foreign = ONLY.replace(
        B_ONLY,
        &format!(r#"<x:Wrap xmlns:x="urn:fixture">{B_ONLY}</x:Wrap>"#),
    );
    assert_eq!(bound(&foreign), []);
    // Not a grid at all: no label.
    let list = ONLY.replace(r#" Layout="Grid""#, "");
    assert_eq!(bound(&list), []);
}

#[test]
fn module_definitions_and_repeat_scopes_are_outside_this_form() {
    let module = ONLY.replace(
        "<Dynamic>",
        r#"<ModuleDefs><ModuleDef Id="MD-1" Name="Fictional module"><Static/><Dynamic><ParameterBlock Id="MD-1_PB" Layout="Grid"><Rows><Row Id="MD-R"/></Rows><Columns><Column Id="MD-C1" Width="50%"/><Column Id="MD-C2" Width="50%"/></Columns><Button Id="MD-1_B" Text="Module label" Cell="1,2"/></ParameterBlock></Dynamic></ModuleDef></ModuleDefs><Dynamic><Module Id="MOD-1_M-1" RefId="MD-1"/>"#,
    );
    let fixture = ingest(&module);
    let layout = load(&fixture.conn, "A-FIX").unwrap();
    let module_buttons: Vec<i64> = fixture
        .conn
        .prepare("SELECT node_id FROM dynamic_node WHERE program_id='A-FIX' AND module_def_id='MD-1' AND kind='Button'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(module_buttons.len(), 1);
    assert!(layout.button("MD-1", module_buttons[0]).is_none());
    assert_eq!(labels(&layout, &buttons(&fixture.conn, "A-FIX")).len(), 1);
    // The walk reaches the module's Button scoped, never as a program Button.
    let active = activation(&fixture.conn, &[]);
    assert!(active
        .buttons
        .iter()
        .any(|button| button.scope.is_some() && button.node_id == module_buttons[0]));

    let repeat = ONLY.replace(B_ONLY, &format!("<Repeat Id=\"RP-1\">{B_ONLY}</Repeat>"));
    let fixture = ingest(&repeat);
    assert!(
        activation(&fixture.conn, &[]).buttons.is_empty(),
        "Repeat is not traversed"
    );
}

#[test]
fn the_existing_walk_records_only_reached_buttons_and_keeps_its_diagnostics() {
    let fixture = ingest(EN);
    let nodes = buttons(&fixture.conn, "A-FIX");
    let active = activation(&fixture.conn, &[]);
    let reached: Vec<i64> = active.buttons.iter().map(|b| b.node_id).collect();
    assert_eq!(reached, nodes, "selector default 1 opens PB-OUTER");
    assert!(active
        .buttons
        .iter()
        .all(|b| b.scope.is_none() && b.block_access.is_none()));
    let refs: Vec<&str> = active
        .parameter_refs
        .iter()
        .map(|r| r.ref_id.as_str())
        .collect();
    assert_eq!(refs, ["PR-MODE", "PR-EDIT", "PR-OTHER"]);
    for node in &nodes {
        assert!(active.diagnostics.iter().any(|d| matches!(
            &d.diagnostic,
            Diagnostic::UnrecognizedNode { node_id, kind } if node_id == node && kind == "Button"
        )));
    }
    let closed = activation(&fixture.conn, &[("PR-MODE", "0")]);
    let reached: Vec<i64> = closed.buttons.iter().map(|b| b.node_id).collect();
    assert_eq!(
        reached,
        [nodes[1]],
        "selector 0 removes the gated Button only"
    );
    assert_eq!(
        closed
            .parameter_refs
            .iter()
            .map(|r| r.ref_id.as_str())
            .collect::<Vec<_>>(),
        refs
    );

    let restricted = ingest(&EN.replace(
        r#"<ParameterBlock Id="PB-OUTER" Text="Fictional outer page">"#,
        r#"<ParameterBlock Id="PB-OUTER" Text="Fictional outer page" Access="Read">"#,
    ));
    let active = activation(&restricted.conn, &[]);
    assert_eq!(active.buttons[0].block_access.as_deref(), Some("Read"));
    assert_eq!(active.buttons[1].block_access, None);
}

#[test]
fn a_button_is_no_container_for_the_walk() {
    let fixture = ingest(&ONLY.replace(
        B_ONLY,
        r#"<Button Id="B-ONLY" Text="Parent" Cell="1,2"><ParameterRefRef RefId="PR-OTHER"/></Button>"#,
    ));
    let active = activation(&fixture.conn, &[]);
    assert_eq!(active.buttons.len(), 1);
    assert!(active.parameter_refs.is_empty());
    assert!(active.diagnostics.iter().any(|d| matches!(
        &d.diagnostic,
        Diagnostic::RefBelowSkippedNode { ref_id: Some(id), .. } if id == "PR-OTHER"
    )));
}

/// `ONLY` with its grid block stripped to a plain block, as some real products declare.
fn gridless() -> String {
    let grid = r#"<ParameterBlock Id="PB-ONLY" Name="Not a label" Inline="true" Layout="Grid">
          <Rows><Row Id="ONLY-R1"/></Rows><Columns><Column Id="ONLY-C1" Width="46%"/><Column Id="ONLY-C2" Width="53%"/></Columns>"#;
    assert!(ONLY.contains(grid));
    ONLY.replace(
        grid,
        r#"<ParameterBlock Id="PB-ONLY" Text="Fictional plain page">"#,
    )
}

const DRIFT: &str = "UPDATE dynamic_node SET position=position+100 WHERE program_id='A-FIX' AND module_def_id='' AND kind='Button'";
const CORRUPT: &str = "UPDATE source_file SET bytes=CAST(replace(CAST(bytes AS TEXT),'Fictional source label','Fictional source lab3l') AS BLOB)";

fn binding_after(xml: &str, mutation: &str) -> ButtonBinding {
    let fixture = ingest(xml);
    assert!(
        fixture.conn.execute(mutation, []).unwrap() > 0,
        "{mutation}"
    );
    let layout = load(&fixture.conn, "A-FIX").unwrap();
    assert_eq!(labels(&layout, &buttons(&fixture.conn, "A-FIX")), []);
    layout.button_binding()
}

#[test]
fn buttons_without_a_grid_block_skip_the_source_read() {
    // No label could exist, so neither drift nor an unreadable source is read or reported.
    assert_eq!(binding_after(&gridless(), DRIFT), ButtonBinding::Bound);
    assert_eq!(binding_after(&gridless(), CORRUPT), ButtonBinding::Bound);
    // The same refusals still apply to a grid Button, also when its grid source is unreadable.
    assert_eq!(binding_after(ONLY, DRIFT), ButtonBinding::StructuralDrift);
    assert_eq!(
        binding_after(ONLY, CORRUPT),
        ButtonBinding::SourceUnavailable
    );
}

#[test]
fn one_grid_button_among_gridless_ones_keeps_the_read() {
    let mixed = gridless().replace(
        "</Dynamic>",
        r#"<ParameterBlock Id="PB-GRID" Layout="Grid"><Rows><Row Id="G-R1"/></Rows><Columns><Column Id="G-C1" Width="50%"/></Columns><Button Id="B-GRID" Text="Fictional grid label" Cell="1,1"/></ParameterBlock></Dynamic>"#,
    );
    let fixture = ingest(&mixed);
    let nodes = buttons(&fixture.conn, "A-FIX");
    let layout = load(&fixture.conn, "A-FIX").unwrap();
    assert_eq!(layout.button_binding(), ButtonBinding::Bound);
    let shown: Vec<String> = labels(&layout, &nodes)
        .into_iter()
        .map(|label| label.1)
        .collect();
    assert_eq!(shown, ["Fictional grid label"]);
    assert_eq!(binding_after(&mixed, DRIFT), ButtonBinding::StructuralDrift);
}

#[test]
fn a_grid_button_below_a_choice_still_reads_and_binds() {
    // The read skip looks through choose/when exactly as admission does.
    let nested = ONLY.replace(
        B_ONLY,
        &format!(r#"<choose ParamRefId="PR-MODE"><when test="1">{B_ONLY}</when></choose>"#),
    );
    assert_ne!(nested, ONLY);
    assert_eq!(bound(&nested).len(), 1);
    assert_eq!(
        binding_after(&nested, DRIFT),
        ButtonBinding::StructuralDrift
    );
}
