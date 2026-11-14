//! Exercises source-bound literal axis translations with fictional program data.
use super::*;
use knx_productdb::dynamic::{TextProjectionBudget, MAX_TEXT_PROJECTION_WORK};
use knx_productdb::query::{parameter_grid_axis_captions, GridAxisCaptions};

const AXES: &str = r#"<ParameterBlock Id="AX-B" Text="Axis example" Layout="Table">
<Rows><Row Id="AX-R9" Text="North" CollapseIfEmpty="true"/><Row Id="AX-R2" Text="South"/></Rows>
<Columns><Column Id="AX-C8" Text="Input" Width="37%"/><Column Id="AX-C3" Text="Output" Width="63%"/></Columns>
<ParameterRefRef RefId="P-1_R-1" Cell="2,1"/></ParameterBlock>"#;

fn translation(conn: &Connection, program: &str, id: &str, language: &str, text: &str) {
    conn.execute("INSERT INTO translation(scope,scope_id,ref_id,language,attribute_name,text) VALUES('Program',?1,?2,?3,'Text',?4)",
        rusqlite::params![program, id, language, text]).unwrap();
}

fn project(conn: &Connection, language: Option<&str>) -> GridAxisCaptions {
    parameter_grid_axis_captions(
        conn,
        "A-1",
        &load(conn, "A-1").unwrap(),
        language,
        &mut TextProjectionBudget::default(),
    )
    .unwrap()
}

#[test]
fn own_row_and_column_translation_preserves_declaration_order_and_geometry() {
    let fixture = ingest(AXES, "");
    translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "Nord");
    translation(&fixture.conn, "A-1", "AX-C8", "de-DE", "Eingang");
    translation(&fixture.conn, "A-1", "AX-B", "de-DE", "Not an axis");
    let layout = load(&fixture.conn, "A-1").unwrap();
    let grid = grid_of(&layout, "", "P-1_R-1").unwrap().clone();
    let selected = project(&fixture.conn, Some("de-DE"));
    assert_eq!(
        selected.texts,
        std::collections::HashMap::from([
            ("AX-R9".into(), "Nord".into()),
            ("AX-C8".into(), "Eingang".into())
        ])
    );
    assert!(selected.unsupported.is_empty());
    assert_eq!(
        grid.rows.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
        ["AX-R9", "AX-R2"]
    );
    assert_eq!(
        grid.columns
            .iter()
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>(),
        ["AX-C8", "AX-C3"]
    );
    assert_eq!(grid.rows[0].text.as_deref(), Some("North"));
    assert_eq!(grid.rows[0].collapse_if_empty.as_deref(), Some("true"));
    assert_eq!(grid.columns[0].width.as_deref(), Some("37%"));
    assert_eq!(
        layout.field("", "P-1_R-1").cell,
        Some(Cell { row: 2, column: 1 })
    );
    assert_eq!(
        grid_of(&load(&fixture.conn, "A-1").unwrap(), "", "P-1_R-1"),
        Some(&grid)
    );
    println!("WP05_CELL|pdb|own|complete");
}

#[test]
fn missing_no_requested_and_unknown_languages_keep_source_fallback() {
    let fixture = ingest(AXES, "");
    translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "Nord");
    for language in [None, Some("fr"), Some(""), Some("!!!"), Some("de-DE-extra")] {
        assert!(
            project(&fixture.conn, language).texts.is_empty(),
            "{language:?}"
        );
        println!("WP05_CELL|pdb|fallback|{language:?}");
    }
    let selected = project(&fixture.conn, Some("de-DE"));
    assert!(!selected.texts.contains_key("AX-R2"));
    assert_eq!(
        grid_of(&load(&fixture.conn, "A-1").unwrap(), "", "P-1_R-1")
            .unwrap()
            .rows[1]
            .text
            .as_deref(),
        Some("South")
    );
    println!("WP05_CELL|pdb|fallback|source-tail");
}

#[test]
fn explicit_empty_translation_is_present_while_missing_and_null_are_not() {
    let fixture = ingest(AXES, "");
    translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "");
    fixture.conn.execute_batch("INSERT INTO translation(scope,scope_id,ref_id,language,attribute_name,text) VALUES('Program','A-1','AX-C8','de-DE','Text',NULL)").unwrap();
    let selected = project(&fixture.conn, Some("de-DE"));
    assert_eq!(selected.texts.get("AX-R9").map(String::as_str), Some(""));
    assert!(!selected.texts.contains_key("AX-C8"));
    assert!(!selected.texts.contains_key("AX-R2"));
    println!("WP05_CELL|pdb|empty|complete");
}

#[test]
fn language_policy_is_exact_then_one_way_prefix_with_lexical_tie_break() {
    let fixture = ingest(AXES, "");
    for (language, text) in [
        ("de-AT", "Nord AT"),
        ("de-DE", "Nord DE"),
        ("deX", "Wrong prefix"),
    ] {
        translation(&fixture.conn, "A-1", "AX-R9", language, text);
    }
    assert_eq!(project(&fixture.conn, Some("de")).texts["AX-R9"], "Nord AT");
    assert_eq!(
        project(&fixture.conn, Some("de-DE")).texts["AX-R9"],
        "Nord DE"
    );
    translation(&fixture.conn, "A-1", "AX-R9", "de", "Exact short");
    assert_eq!(
        project(&fixture.conn, Some("de")).texts["AX-R9"],
        "Exact short"
    );
    assert!(project(&fixture.conn, Some("de-CH")).texts.is_empty());
    println!("WP05_CELL|pdb|language|complete");
}

#[test]
fn wrong_axis_program_scope_and_attribute_never_supply_a_caption() {
    let fixture = ingest(AXES, "");
    translation(&fixture.conn, "Other", "AX-R9", "de-DE", "Other program");
    translation(&fixture.conn, "A-1", "AX-B", "de-DE", "Block");
    translation(&fixture.conn, "A-1", "AX-absent", "de-DE", "Neighbor");
    fixture.conn.execute_batch("INSERT INTO translation VALUES('Hardware','A-1','de-DE','AX-R9','Text','Wrong scope'); INSERT INTO translation VALUES('Program','A-1','de-DE','AX-R9','Value','Wrong attribute')").unwrap();
    assert!(project(&fixture.conn, Some("de-DE")).texts.is_empty());
    let layout = load(&fixture.conn, "A-1").unwrap();
    assert!(parameter_grid_axis_captions(
        &fixture.conn,
        "Other",
        &layout,
        Some("de-DE"),
        &mut TextProjectionBudget::default()
    )
    .unwrap()
    .texts
    .is_empty());
    println!("WP05_CELL|pdb|isolation|complete");
}

#[test]
fn ambiguous_axis_identity_is_not_borrowed_between_rows_columns_or_blocks() {
    for dynamic in [
        AXES.replace("AX-C8", "AX-R9"),
        format!("{AXES}<ParameterBlock Id=\"AX-Other\" Text=\"Other\" Layout=\"Table\"><Rows><Row Id=\"AX-R9\" Text=\"Neighbor\"/></Rows><Columns><Column Id=\"AX-Other-C\" Text=\"Other\"/></Columns></ParameterBlock>"),
    ] {
        let fixture = ingest(&dynamic, "");
        translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "Must not leak");
        assert!(project(&fixture.conn, Some("de-DE")).texts.is_empty());
        println!("WP05_CELL|pdb|ambiguous|{dynamic:?}");
    }
}

#[test]
fn source_hash_length_and_stored_structure_drift_refuse_axis_binding() {
    for sql in [
        "UPDATE source_file SET bytes=CAST(replace(CAST(bytes AS TEXT),'North','Other') AS BLOB)",
        "UPDATE source_file SET len=len+1",
        "UPDATE dynamic_node SET position=99 WHERE kind='Row' AND position=0",
        "UPDATE dynamic_node SET element_id='Changed' WHERE kind='ParameterBlock'",
    ] {
        let fixture = ingest(AXES, "");
        translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "Nord");
        assert_eq!(project(&fixture.conn, Some("de-DE")).texts["AX-R9"], "Nord");
        fixture.conn.execute_batch(sql).unwrap();
        assert!(
            project(&fixture.conn, Some("de-DE")).texts.is_empty(),
            "{sql}"
        );
        println!("WP05_CELL|pdb|drift|{sql}");
    }
}

#[test]
fn module_repeat_and_nonliteral_source_axes_keep_originals() {
    for (dynamic, module) in [
        (String::new(), AXES.to_owned()),
        (
            format!("<Repeat Id=\"AX-Repeat\" Count=\"2\">{AXES}</Repeat>"),
            String::new(),
        ),
        (
            AXES.replace("Text=\"North\"", "Text=\"{{0}}\""),
            String::new(),
        ),
        (
            AXES.replace(
                "Text=\"North\"",
                "Text=\"North\" TextParameterRefId=\"P-1_R-1\"",
            ),
            String::new(),
        ),
        (
            AXES.replace("Text=\"North\"", "Text=\"North\" Mystery=\"yes\""),
            String::new(),
        ),
        (
            AXES.replace(
                "Text=\"North\" CollapseIfEmpty=\"true\"/>",
                "Text=\"North\"><Extension/></Row>",
            ),
            String::new(),
        ),
    ] {
        let fixture = ingest(&dynamic, &module);
        translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "Not admitted");
        assert!(project(&fixture.conn, Some("de-DE")).texts.is_empty());
        println!("WP05_CELL|pdb|excluded|{dynamic:?}|{module:?}");
    }
}

#[test]
fn selected_translated_placeholders_are_refused_without_evaluation() {
    for text in ["Channel {{0}}", "{{Argument}}", "{{unclosed", "stray }}"] {
        let fixture = ingest(AXES, "");
        translation(&fixture.conn, "A-1", "AX-R9", "de-DE", text);
        let selected = project(&fixture.conn, Some("de-DE"));
        assert!(selected.texts.is_empty());
        assert_eq!(
            selected.unsupported,
            std::collections::HashSet::from(["AX-R9".into()])
        );
        assert_eq!(
            grid_of(&load(&fixture.conn, "A-1").unwrap(), "", "P-1_R-1")
                .unwrap()
                .rows[0]
                .text
                .as_deref(),
            Some("North")
        );
        println!("WP05_CELL|pdb|placeholder|{text}");
    }
}

// Test-local VM accounting only: no production connection hooks or timing oracle.
struct VmSteps<'a> {
    conn: &'a Connection,
    steps: Box<usize>,
}
impl<'a> VmSteps<'a> {
    fn install(conn: &'a Connection) -> Self {
        unsafe extern "C" fn tick(context: *mut std::ffi::c_void) -> i32 {
            // SAFETY: SQLite calls synchronously; the owned box stays pinned.
            unsafe { *context.cast::<usize>() += 1 };
            0
        }
        let mut guard = Self {
            conn,
            steps: Box::new(0),
        };
        // SAFETY: the guard owns the context until it detaches the callback.
        unsafe {
            rusqlite::ffi::sqlite3_progress_handler(
                conn.handle(),
                1,
                Some(tick),
                (&mut *guard.steps as *mut usize).cast(),
            );
        }
        guard
    }
}
impl Drop for VmSteps<'_> {
    fn drop(&mut self) {
        // SAFETY: unregister before the boxed callback context is freed.
        unsafe {
            rusqlite::ffi::sqlite3_progress_handler(
                self.conn.handle(),
                0,
                None,
                std::ptr::null_mut(),
            );
        }
    }
}

fn measured_axes(
    conn: &Connection,
    layout: &ParameterLayout,
    language: &str,
) -> (GridAxisCaptions, usize) {
    let guard = VmSteps::install(conn);
    let selected = parameter_grid_axis_captions(
        conn,
        "A-1",
        layout,
        Some(language),
        &mut TextProjectionBudget::default(),
    )
    .unwrap();
    let steps = *guard.steps;
    drop(guard);
    (selected, steps)
}

#[test]
fn axis_translation_sql_work_excludes_unrelated_program_rows() {
    // Independently vary normalized row count and payload size, not retained source.
    for rows in [4, 2048] {
        for payload_bytes in [8, 4096] {
            let fixture = ingest(AXES, "");
            translation(&fixture.conn, "A-1", "AX-R9", "de-AT", "Nord AT");
            translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "Nord DE");
            let layout = load(&fixture.conn, "A-1").unwrap();
            let source: Vec<u8> = fixture
                .conn
                .query_row("SELECT bytes FROM source_file", [], |r| r.get(0))
                .unwrap();
            let baseline: Vec<_> = ["de-DE", "de", "fr"]
                .iter()
                .map(|language| measured_axes(&fixture.conn, &layout, language).1)
                .collect();
            let tx = fixture.conn.unchecked_transaction().unwrap();
            for i in 0..rows {
                for language in ["de-AT", "de-DE"] {
                    translation(
                        &tx,
                        "A-1",
                        &format!("Unrelated-{i:05}"),
                        language,
                        &"z".repeat(payload_bytes),
                    );
                }
            }
            tx.commit().unwrap();
            let before = fixture.conn.total_changes();
            for ((language, expected), baseline) in [
                ("de-DE", Some("Nord DE")),
                ("de", Some("Nord AT")),
                ("fr", None),
            ]
            .into_iter()
            .zip(baseline)
            {
                let (selected, steps) = measured_axes(&fixture.conn, &layout, language);
                assert_eq!(selected.texts.get("AX-R9").map(String::as_str), expected);
                assert_eq!(selected.texts.len(), usize::from(expected.is_some()));
                assert!(selected.unsupported.is_empty());
                // B-tree height may change; a row scan/sort cannot fit this fixed allowance.
                assert!(steps <= baseline + 128,
                    "{language}: rows={rows}, payload={payload_bytes}, VM={steps}, baseline={baseline}");
            }
            assert_eq!(fixture.conn.total_changes(), before);
            let after: Vec<u8> = fixture
                .conn
                .query_row("SELECT bytes FROM source_file", [], |r| r.get(0))
                .unwrap();
            assert_eq!(after, source);
            assert_eq!(
                grid_of(&load(&fixture.conn, "A-1").unwrap(), "", "P-1_R-1"),
                grid_of(&layout, "", "P-1_R-1")
            );
            println!("WP05_CELL|pdb|sql-work|{rows}|{payload_bytes}|all-three-languages");
        }
    }
}

#[test]
fn unmatched_axis_language_metadata_exhausts_the_same_sticky_budget() {
    for keys in [3, 2048] {
        let fixture = ingest(AXES, "");
        // Distinct keys belong only to unrelated rows; no axis candidate.
        for i in 0..keys {
            translation(
                &fixture.conn,
                "A-1",
                "Unrelated",
                &format!("zz-{i:05}"),
                "irrelevant",
            );
        }
        let layout = load(&fixture.conn, "A-1").unwrap();
        assert!(project(&fixture.conn, Some("fr")).texts.is_empty());
        let mut budget = TextProjectionBudget::default();
        budget
            .admit_copy(&"x".repeat(MAX_TEXT_PROJECTION_WORK - 9))
            .unwrap();
        let changes = fixture.conn.total_changes();
        let guard = VmSteps::install(&fixture.conn);
        let result =
            parameter_grid_axis_captions(&fixture.conn, "A-1", &layout, Some("fr"), &mut budget);
        let steps = *guard.steps;
        drop(guard);
        assert!(matches!(
            result,
            Err(knx_productdb::query::CaptionTemplateError::Projection(_))
        ));
        assert!(
            steps < 512,
            "metadata admission stops key enumeration: {keys} keys, {steps} steps"
        );
        assert!(budget.copy_text("").is_err());
        assert_eq!(fixture.conn.total_changes(), changes);
        println!("WP05_CELL|pdb|unmatched-budget|{keys}");
    }
}

fn ingest_axis_source(xml: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("catalogue.sqlite")).unwrap();
    ingest_file(&conn, "M-1/A-1.xml", xml.as_bytes()).unwrap();
    Fixture { _dir: dir, conn }
}

fn qualified_owner(xml: &str, name: &str, namespace: &str) -> String {
    let xml = xml.replacen(
        &format!("<{name}>"),
        &format!("<x:{name} xmlns:x=\"{namespace}\">"),
        1,
    );
    let xml = xml.replacen(
        &format!("<{name} "),
        &format!("<x:{name} xmlns:x=\"{namespace}\" "),
        1,
    );
    let xml = xml.replacen(&format!("</{name}>"), &format!("</x:{name}>"), 1);
    assert!(xml.contains(&format!("<x:{name} ")));
    xml
}

#[test]
fn foreign_namespace_owner_ancestors_never_admit_axis_translations() {
    let canonical = source(AXES, "");
    let namespace = "http://knx.org/xml/project/11";
    for name in [
        "KNX",
        "ManufacturerData",
        "Manufacturer",
        "ApplicationPrograms",
        "ApplicationProgram",
    ] {
        // A lexical prefix is not a namespace mismatch: every ancestor has a positive.
        let fixture = ingest_axis_source(&qualified_owner(&canonical, name, namespace));
        translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "Nord");
        assert_eq!(
            project(&fixture.conn, Some("de-DE")).texts["AX-R9"],
            "Nord",
            "{name}"
        );
        println!("WP05_CELL|pdb|ancestor|{name}|canonical");
        for foreign in [
            "urn:fictional:owner",
            "http://knx.org/xml/project/11-lookalike",
        ] {
            let fixture = ingest_axis_source(&qualified_owner(&canonical, name, foreign));
            translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "Must not leak");
            let selected = project(&fixture.conn, Some("de-DE"));
            assert!(selected.texts.is_empty(), "{name}: {foreign}");
            assert!(selected.unsupported.is_empty());
            println!("WP05_CELL|pdb|ancestor|{name}|{foreign}");
        }
    }
    // A foreign wrapper must not regain authority at a canonical owner below it.
    let wrapped = canonical
        .replace(
            "<ApplicationPrograms>",
            "<x:Wrapper xmlns:x=\"urn:fictional:wrapper\"><ApplicationPrograms>",
        )
        .replace(
            "</ApplicationPrograms>",
            "</ApplicationPrograms></x:Wrapper>",
        );
    assert!(wrapped.contains("<x:Wrapper"));
    let fixture = ingest_axis_source(&wrapped);
    translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "Must not leak");
    assert!(project(&fixture.conn, Some("de-DE")).texts.is_empty());
    println!("WP05_CELL|pdb|ancestor|wrapper");

    // Existing direct retained-source policy is root-relative, not a scheme whitelist.
    // Preserve its namespace-free and uniformly unknown-root forms, not package admission.
    for root in ["", "urn:fictional:whole-source"] {
        let xml = canonical.replace(
            &format!("xmlns=\"{namespace}\""),
            &format!("xmlns=\"{root}\""),
        );
        assert!(xml.contains(&format!("xmlns=\"{root}\"")));
        let fixture = ingest_axis_source(&xml);
        translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "Nord");
        assert_eq!(project(&fixture.conn, Some("de-DE")).texts["AX-R9"], "Nord");
        println!("WP05_CELL|pdb|ancestor|root|{root}");
    }
    // A namespace-free ancestor in a canonical source is different from that policy.
    let xml = canonical
        .replace("<ApplicationPrograms>", "<ApplicationPrograms xmlns=\"\">")
        .replace(
            "<ApplicationProgram Id=",
            &format!("<ApplicationProgram xmlns=\"{namespace}\" Id="),
        );
    assert!(xml.contains("<ApplicationPrograms xmlns=\"\">"));
    let fixture = ingest_axis_source(&xml);
    translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "Must not leak");
    assert!(project(&fixture.conn, Some("de-DE")).texts.is_empty());
    println!("WP05_CELL|pdb|ancestor|namespace-free-ancestor");
}

#[test]
fn selected_axis_text_and_metadata_share_the_sticky_utf8_byte_budget() {
    let fixture = ingest(AXES, "");
    translation(&fixture.conn, "A-1", "AX-R9", "de-DE", "é");
    let layout = load(&fixture.conn, "A-1").unwrap();
    // Two IDs, cursor + candidate + selected language, and UTF-8 text: 27.
    let mut budget = TextProjectionBudget::default();
    budget
        .admit_copy(&"x".repeat(MAX_TEXT_PROJECTION_WORK - 27))
        .unwrap();
    assert_eq!(
        parameter_grid_axis_captions(&fixture.conn, "A-1", &layout, Some("de-DE"), &mut budget)
            .unwrap()
            .texts["AX-R9"],
        "é"
    );
    assert!(budget.copy_text("x").is_err());
    assert!(budget.copy_text("").is_err(), "exhaustion stays sticky");
    let mut budget = TextProjectionBudget::default();
    budget
        .admit_copy(&"x".repeat(MAX_TEXT_PROJECTION_WORK - 26))
        .unwrap();
    assert!(parameter_grid_axis_captions(
        &fixture.conn,
        "A-1",
        &layout,
        Some("de-DE"),
        &mut budget
    )
    .is_err());
    println!("WP05_CELL|pdb|utf8-budget|complete");
}
