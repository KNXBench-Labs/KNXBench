//! Conditional block rename: ingest, v33 migration and evaluation of synthetic leaves (WP-05).
//!
//! The program is invented. Shapes mirror the independently reviewed contract only: a literal
//! `ParameterBlockRename`/`Rename` leaf naming its own enclosing block, active under a test.
use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Write};

use knx_productdb::dynamic::{
    evaluate, load_program_trees, resolve_values, Activation, Diagnostic, ProgramTrees,
};
use knx_productdb::{install_package, open_and_migrate, CURRENT_PRODUCTDB_VERSION};
use rusqlite::{params, types::Value, Connection};
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// Selector `P-1` drives block `PB-1` (plain renames); `P-3` drives block `PB-2`, one refusal
/// shape per test value; `MI-1` instantiates a module whose own block carries a rename.
const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-1" Name="Program">
<Static>
<ParameterTypes><ParameterType Id="PT-Num" Name="Num"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255"/></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P-1" Name="One" ParameterType="PT-Num" Text="One" Value="0"/><Parameter Id="P-3" Name="Three" ParameterType="PT-Num" Text="Three" Value="0"/><Parameter Id="P-4" Name="Four" ParameterType="PT-Num" Text="Four" Value="0"/></Parameters>
<ParameterRefs><ParameterRef Id="P-1_R-1" RefId="P-1"/><ParameterRef Id="P-3_R-3" RefId="P-3"/><ParameterRef Id="P-4_R-4" RefId="P-4"/></ParameterRefs>
</Static>
<Dynamic>
<Channel Id="CH-1" Name="A" Number="1" Text="Channel">
<ParameterBlock Id="PB-1" Name="main" Text="Base one">
<ParameterRefRef RefId="P-1_R-1"/>
<choose ParamRefId="P-1_R-1">
<when test="1"><ParameterBlockRename Id="PR-1" RefId="PB-1" Text="fixture_switch_py"/></when>
<when test="2"><ParameterBlockRename Id="PR-2" RefId="PB-1" Text="fixture_switch_pz"/></when>
</choose>
</ParameterBlock>
<ParameterBlock Id="PB-2" Name="shapes" Text="Base two">
<ParameterRefRef RefId="P-3_R-3"/>
<choose ParamRefId="P-3_R-3">
<when test="1"><ParameterBlockRename Id="PR-3" RefId="PB-1" Text="Cross"/></when>
<when test="2"><Rename Id="PR-4" RefId="PB-2" Text="Converted"/></when>
<when test="3"><ParameterBlockRename Id="PR-5" RefId="PB-2" Text="First"/><ParameterBlockRename Id="PR-6" RefId="PB-2" Text="Second"/></when>
<when test="4"><ParameterBlockRename Id="PR-7" RefId="PB-2" Text="Extra" Foo="x"/></when>
<when test="5"><ParameterBlockRename Id="PR-8" RefId="PB-2" Text="Child"><ParameterRefRef RefId="P-1_R-1"/></ParameterBlockRename></when>
<when test="6"><ParameterBlockRename RefId="PB-2" Text="No identity"/></when>
<when test="7"><ParameterBlockRename Id="PR-9" RefId="PB-2" Text="Fill {{0}}"/></when>
<when test="8"><ParameterBlockRename Id="PR-10" RefId="PB-NONE" Text="Missing target"/></when>
<when test="9"><ParameterBlockRename Id="PR-11" RefId="PB-2" Text="Twice"/><ParameterBlockRename Id="PR-11" RefId="PB-2" Text="Twice"/></when>
</choose>
</ParameterBlock>
<choose ParamRefId="P-1_R-1"><when test="3"><ParameterBlockRename Id="PR-OUT" RefId="PB-1" Text="Outside"/></when></choose>
<Module Id="MI-1" RefId="MD-1"/>
</Channel>
</Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="Module"><Dynamic><ParameterBlock Id="MD-1_PB-1" Text="Module base"><ParameterRefRef RefId="P-4_R-4"/><choose ParamRefId="P-4_R-4"><when test="1"><ParameterBlockRename Id="MD-1_PR-1" RefId="MD-1_PB-1" Text="Module renamed"/></when></choose></ParameterBlock></Dynamic></ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const DYNAMIC: &str =
    "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Dynamic";
const MODULE_DYNAMIC: &str = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/ModuleDefs/ModuleDef/Dynamic";

/// Three attributes at three hosts (`ParameterBlockRename`, `Rename`, and the module tree's
/// `ParameterBlockRename`) plus the `Foo` that was never modelled.
const OLD_UNKNOWNS: usize = 3 * 3 + 1;

fn package(program: &[u8]) -> Vec<u8> {
    package_of(&[("M-0001/M-0001_A-1.xml", program)])
}

fn package_of(programs: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let fixed = [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ];
    for &(path, bytes) in fixed.iter().chain(programs) {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn installed() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    install_package(&conn, "synthetic.knxprod", &package(PROGRAM)).unwrap();
    (dir, path)
}

fn rows(conn: &Connection, sql: &str) -> Vec<Vec<String>> {
    let mut stmt = conn.prepare(sql).unwrap();
    let width = stmt.column_count();
    stmt.query_map([], |r| {
        (0..width)
            .map(|i| r.get::<_, Value>(i).map(|v| format!("{v:?}")))
            .collect::<Result<Vec<_>, _>>()
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

/// Everything the v33 step may touch, in a stable order.
fn snapshot(conn: &Connection) -> Vec<Vec<Vec<String>>> {
    [
        "PRAGMA user_version",
        "SELECT * FROM dynamic_node ORDER BY program_id, module_def_id, node_id",
        "SELECT source_sha256, xpath, kind, name, occurrences, sample FROM ingest_unknown
         ORDER BY source_sha256, xpath, kind, name",
        "SELECT sha256, unknown_count FROM package ORDER BY sha256",
        "SELECT * FROM package_install_unknown ORDER BY package_sha256, ordinal",
        "SELECT package_sha256, status, unknown_distinct, unknown_occurrences
         FROM package_install_report ORDER BY package_sha256",
        "SELECT * FROM package_install_count WHERE category = 'unknown_construct'
         ORDER BY package_sha256, disposition",
    ]
    .iter()
    .map(|sql| rows(conn, sql))
    .collect()
}

type Node = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// `(element_id, ref_id, text, extra)` of one leaf.
fn leaf(conn: &Connection, id: &str) -> Node {
    conn.query_row(
        "SELECT element_id, ref_id, text, extra FROM dynamic_node
         WHERE kind IN ('ParameterBlockRename','Rename') AND (element_id = ?1 OR ?1 = '')
         ORDER BY node_id LIMIT 1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .unwrap()
}

fn rename_attribute_unknowns(conn: &Connection) -> Vec<(String, String)> {
    conn.prepare(
        "SELECT xpath, name FROM ingest_unknown
         WHERE kind = 'Attribute' AND (xpath LIKE '%/Dynamic/ParameterBlockRename'
                                    OR xpath LIKE '%/Dynamic/Rename') ORDER BY 1, 2",
    )
    .unwrap()
    .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn some(value: &str) -> Option<String> {
    Some(value.to_string())
}

#[test]
fn ingest_keeps_identity_target_and_text_and_reports_only_what_it_does_not_model() {
    let (_dir, path) = installed();
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        leaf(&conn, "PR-1"),
        (some("PR-1"), some("PB-1"), some("fixture_switch_py"), None)
    );
    assert_eq!(
        leaf(&conn, "PR-4").3,
        None,
        "the converted kind stores the same columns"
    );
    // Source identity survives for a leaf the evaluator refuses.
    assert_eq!(
        leaf(&conn, "PR-7"),
        (some("PR-7"), some("PB-2"), some("Extra"), some("Foo=x"))
    );
    // A missing `@Id` stays NULL, never invented.
    let anonymous: Option<String> = conn
        .query_row(
            "SELECT element_id FROM dynamic_node WHERE text = 'No identity'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(anonymous, None);
    assert_eq!(
        rename_attribute_unknowns(&conn),
        [(format!("{DYNAMIC}/ParameterBlockRename"), "Foo".to_string())]
    );
}

fn trees(conn: &Connection) -> ProgramTrees {
    load_program_trees(conn, "A-1").unwrap()
}

/// Supplied values over the program's own defaults, as every real caller resolves them: a
/// selector left out is at its default, not undecided.
fn with_in(
    conn: &Connection,
    program: &str,
    values: &[(&str, &str)],
) -> knx_productdb::dynamic::ValueMap {
    let supplied = values
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect::<HashMap<_, _>>();
    resolve_values(conn, program, &supplied).unwrap()
}

fn node_of(conn: &Connection, element: &str) -> i64 {
    conn.query_row(
        "SELECT node_id FROM dynamic_node WHERE element_id = ?1 AND module_def_id = ''",
        [element],
        |r| r.get(0),
    )
    .unwrap()
}

fn refused(activation: &Activation, node: i64) -> usize {
    activation
        .diagnostics
        .iter()
        .filter(|d| {
            d.scope.is_none()
                && matches!(d.diagnostic, Diagnostic::UnrecognizedNode { node_id, .. } if node_id == node)
        })
        .count()
}

fn replacement(activation: &Activation, block: i64) -> Option<&str> {
    activation
        .renames
        .iter()
        .find(|r| r.target_node_id == block)
        .map(|r| r.text.as_str())
}

#[test]
fn one_loaded_source_follows_the_selector_back_to_the_base_caption() {
    let (_dir, path) = installed();
    let conn = open_and_migrate(&path).unwrap();
    let trees = trees(&conn);
    let block = node_of(&conn, "PB-1");
    let mut seen = Vec::new();
    // The same loaded trees, a fresh value map per evaluation.
    for selector in ["0", "1", "2", "0", "3", "0"] {
        let activation = evaluate(&trees, &with_in(&conn, "A-1", &[("P-1_R-1", selector)]));
        let base: Vec<_> = activation
            .labels
            .iter()
            .filter(|l| l.node_id == block)
            .map(|l| (l.raw_text.as_str(), l.text.as_str()))
            .collect();
        assert_eq!(base, [("Base one", "Base one")], "base is immutable");
        seen.push(replacement(&activation, block).map(str::to_string));
    }
    assert_eq!(
        seen,
        [
            None,
            some("fixture_switch_py"),
            some("fixture_switch_pz"),
            None,
            None,
            None
        ],
        "the third state's leaf sits outside the block and is refused"
    );
    // Inactive branches leave nothing behind: no renames, no diagnostic for the idle leaves.
    let idle = evaluate(&trees, &with_in(&conn, "A-1", &[]));
    assert!(idle.renames.is_empty());
    assert!(!idle
        .diagnostics
        .iter()
        .any(|d| matches!(d.diagnostic, Diagnostic::UnrecognizedNode { .. })));
}

#[test]
fn every_refused_shape_keeps_the_base_caption_and_stays_reported() {
    let (_dir, path) = installed();
    let conn = open_and_migrate(&path).unwrap();
    let trees = trees(&conn);
    let block = node_of(&conn, "PB-2");
    let outside = node_of(&conn, "PR-OUT");
    let by_text = |text: &str| -> i64 {
        conn.query_row(
            "SELECT node_id FROM dynamic_node WHERE text = ?1 AND module_def_id = ''",
            [text],
            |r| r.get(0),
        )
        .unwrap()
    };
    let selector_one = evaluate(&trees, &with_in(&conn, "A-1", &[("P-1_R-1", "3")]));
    assert_eq!(refused(&selector_one, outside), 1, "outside every block");
    assert!(selector_one.renames.is_empty());
    for (value, leaves) in [
        ("1", vec![node_of(&conn, "PR-3")]),
        ("3", vec![node_of(&conn, "PR-5"), node_of(&conn, "PR-6")]),
        ("4", vec![node_of(&conn, "PR-7")]),
        ("5", vec![node_of(&conn, "PR-8")]),
        ("6", vec![by_text("No identity")]),
        ("7", vec![node_of(&conn, "PR-9")]),
        ("8", vec![node_of(&conn, "PR-10")]),
    ] {
        let activation = evaluate(&trees, &with_in(&conn, "A-1", &[("P-3_R-3", value)]));
        assert_eq!(replacement(&activation, block), None, "selector {value}");
        for leaf in leaves {
            assert_eq!(
                refused(&activation, leaf),
                1,
                "selector {value}, node {leaf}"
            );
        }
        let labels: Vec<_> = activation
            .labels
            .iter()
            .filter(|l| l.node_id == block)
            .map(|l| l.text.as_str())
            .collect();
        assert_eq!(labels, ["Base two"], "selector {value}");
    }
    // The same identity declared twice: neither declaration is trusted.
    let twice = evaluate(&trees, &with_in(&conn, "A-1", &[("P-3_R-3", "9")]));
    assert!(twice.renames.is_empty());
    assert_eq!(
        twice
            .diagnostics
            .iter()
            .filter(|d| matches!(d.diagnostic, Diagnostic::UnrecognizedNode { .. }))
            .count(),
        2
    );
    // The converted kind is accepted on the same terms.
    let converted = evaluate(&trees, &with_in(&conn, "A-1", &[("P-3_R-3", "2")]));
    assert_eq!(replacement(&converted, block), Some("Converted"));
    assert_eq!(converted.renames[0].kind, "Rename");
}

#[test]
fn a_leaf_inside_a_module_expansion_is_refused() {
    let (_dir, path) = installed();
    let conn = open_and_migrate(&path).unwrap();
    let activation = evaluate(&trees(&conn), &with_in(&conn, "A-1", &[("P-4_R-4", "1")]));
    assert!(activation.renames.is_empty());
    assert!(activation.diagnostics.iter().any(|d| d.scope.is_some()
        && matches!(&d.diagnostic, Diagnostic::UnrecognizedNode { kind, .. } if kind == "ParameterBlockRename")));
    // The source leaf is still stored with its identity, for the day it is modelled.
    let stored: Option<String> = conn
        .query_row(
            "SELECT ref_id FROM dynamic_node WHERE element_id = 'MD-1_PR-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stored, some("MD-1_PB-1"));
}

/// Rewinds a v33 database to the state the v32 parser left: the three attributes folded back
/// into `extra`, their columns empty, reported again, and counted again in every report.
fn rewind_to_v32(conn: &Connection) {
    type Key = (String, String, &'static str);
    let mut retired: BTreeMap<Key, (i64, String)> = BTreeMap::new();
    /// `(blob, program, node, kind, in a module tree, [Id, RefId, Text], extra)`.
    type RenameRow = (
        String,
        String,
        i64,
        String,
        bool,
        [Option<String>; 3],
        Option<String>,
    );
    let leaves: Vec<RenameRow> = conn
        .prepare(
            "SELECT a.source_sha256, d.program_id, d.node_id, d.kind, d.module_def_id <> '',
                    d.element_id, d.ref_id, d.text, d.extra, d.module_def_id
             FROM dynamic_node AS d JOIN application_program AS a ON a.id = d.program_id
             WHERE d.kind IN ('ParameterBlockRename','Rename')
             ORDER BY d.program_id, d.module_def_id, d.node_id",
        )
        .unwrap()
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                [r.get(5)?, r.get(6)?, r.get(7)?],
                r.get(8)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for (sha, program, node, kind, module, [id, reference, text], extra) in leaves {
        let mut lines: Vec<String> = extra
            .map(|e| e.split('\n').map(str::to_string).collect())
            .unwrap_or_default();
        for (name, value) in [("Id", id), ("RefId", reference), ("Text", text)] {
            let Some(value) = value else { continue };
            lines.push(format!("{name}={value}"));
            let xpath = if module {
                format!("{MODULE_DYNAMIC}/{kind}")
            } else {
                format!("{DYNAMIC}/{kind}")
            };
            retired
                .entry((sha.clone(), xpath, name))
                .and_modify(|(n, _)| *n += 1)
                .or_insert((1, value));
        }
        lines.sort_by_key(|l| l.split('=').next().unwrap_or_default().to_string());
        conn.execute(
            "UPDATE dynamic_node SET element_id = NULL, ref_id = NULL, text = NULL, extra = ?3
             WHERE program_id = ?1 AND node_id = ?2 AND kind = ?4 AND module_def_id <> '' = ?5",
            params![program, node, lines.join("\n"), kind, module],
        )
        .unwrap();
    }
    for ((sha, xpath, name), (occurrences, sample)) in &retired {
        conn.execute(
            "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
             VALUES (?1, NULL, ?2, 'Attribute', ?3, ?4, ?5)",
            params![sha, xpath, name, occurrences, sample],
        )
        .unwrap();
    }
    let packages: Vec<String> = conn
        .prepare("SELECT package_sha256 FROM package_install_report WHERE status = 'measured'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for package in packages {
        let members: Vec<String> = conn
            .prepare(
                "SELECT source_sha256 FROM package_member
                 WHERE package_sha256 = ?1 AND role = 'ApplicationProgram'",
            )
            .unwrap()
            .query_map([&package], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let mut merged: BTreeMap<(String, &'static str), (i64, String)> = BTreeMap::new();
        let mut distinct = 0i64;
        for ((sha, xpath, name), (occurrences, sample)) in &retired {
            if members.contains(sha) {
                distinct += 1;
                merged
                    .entry((xpath.clone(), name))
                    .and_modify(|(o, _)| *o += occurrences)
                    .or_insert((*occurrences, sample.clone()));
            }
        }
        conn.execute(
            "UPDATE package SET unknown_count = unknown_count + ?2 WHERE sha256 = ?1",
            params![package, distinct],
        )
        .unwrap();
        let mut report: Vec<(String, String, String, i64, Option<String>)> = conn
            .prepare(
                "SELECT xpath, kind, name, occurrences, sample FROM package_install_unknown
                 WHERE package_sha256 = ?1",
            )
            .unwrap()
            .query_map([&package], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for ((xpath, name), (occurrences, sample)) in &merged {
            report.push((
                xpath.clone(),
                "Attribute".into(),
                (*name).into(),
                *occurrences,
                Some(sample.clone()),
            ));
        }
        report.sort_by(|a, b| (&a.0, &a.1, &a.2).cmp(&(&b.0, &b.1, &b.2)));
        conn.execute(
            "DELETE FROM package_install_unknown WHERE package_sha256 = ?1",
            [&package],
        )
        .unwrap();
        for (ordinal, (xpath, kind, name, occurrences, sample)) in report.iter().enumerate() {
            conn.execute(
                "INSERT INTO package_install_unknown VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    package,
                    ordinal as i64,
                    xpath,
                    kind,
                    name,
                    occurrences,
                    sample
                ],
            )
            .unwrap();
        }
        let total: i64 = report.iter().map(|r| r.3).sum();
        conn.execute(
            "UPDATE package_install_report SET unknown_distinct = ?2, unknown_occurrences = ?3
             WHERE package_sha256 = ?1",
            params![package, report.len() as i64, total],
        )
        .unwrap();
        for (disposition, count) in [("read", total), ("stored", report.len() as i64)] {
            conn.execute(
                "UPDATE package_install_count SET count = ?3
                 WHERE package_sha256 = ?1 AND category = 'unknown_construct' AND disposition = ?2",
                params![package, disposition, count],
            )
            .unwrap();
        }
    }
    conn.execute_batch("PRAGMA user_version = 32;").unwrap();
}

#[test]
fn v32_to_v33_reproduces_a_fresh_install_and_is_idempotent() {
    let (_fresh_dir, fresh_path) = installed();
    let fresh = snapshot(&open_and_migrate(&fresh_path).unwrap());

    let (_dir, path) = installed();
    {
        let conn = Connection::open(&path).unwrap();
        rewind_to_v32(&conn);
        // The rewind is genuine: the old shape is back in every place it lived.
        assert_eq!(
            leaf(&conn, "").0,
            None,
            "the columns are empty again in the old store"
        );
        assert_eq!(rename_attribute_unknowns(&conn).len(), OLD_UNKNOWNS);
        assert_ne!(snapshot(&conn), fresh);
    }
    let migrated = snapshot(&open_and_migrate(&path).unwrap());
    assert_eq!(
        migrated[0],
        [[format!("Integer({CURRENT_PRODUCTDB_VERSION})")]]
    );
    assert_eq!(migrated, fresh);
    // The unknown evidence that is not modelled survives, exactly.
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        rename_attribute_unknowns(&conn),
        [(format!("{DYNAMIC}/ParameterBlockRename"), "Foo".to_string())]
    );
    assert_eq!(snapshot(&conn), fresh, "a second open changes nothing");
}

#[test]
fn v32_to_v33_names_a_damaged_blob_and_keeps_its_old_evidence_and_report() {
    let (_dir, path) = installed();
    let program_sha: String;
    {
        let conn = Connection::open(&path).unwrap();
        program_sha = conn
            .query_row(
                "SELECT source_sha256 FROM application_program WHERE id = 'A-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        rewind_to_v32(&conn);
        conn.execute(
            "UPDATE source_file SET bytes = substr(bytes, 1, 200) WHERE sha256 = ?1",
            [&program_sha],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    let failures: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown
             WHERE source_sha256 = ?1 AND kind = 'RenameBackfillError'",
            [&program_sha],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(failures, 1);
    // Nothing was retired for it and the old tree rows are as they were.
    assert_eq!(rename_attribute_unknowns(&conn).len(), OLD_UNKNOWNS);
    assert_eq!(leaf(&conn, "").0, None);
    // The report still describes exactly the rows that stayed, so it is not downgraded.
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, "measured");
}

#[test]
fn a_program_that_lost_its_id_conflict_gains_no_tree_and_keeps_nothing_stale() {
    // A second package declaring the same program id with other bytes loses; both packages
    // must migrate exactly like a fresh install of both.
    let rival = String::from_utf8(PROGRAM.to_vec())
        .unwrap()
        .replace("Name=\"Program\"", "Name=\"Rival\"");
    let build = |dir: &tempfile::TempDir| {
        let path = dir.path().join("products.sqlite");
        let conn = open_and_migrate(&path).unwrap();
        install_package(&conn, "first.knxprod", &package(PROGRAM)).unwrap();
        install_package(&conn, "second.knxprod", &package(rival.as_bytes())).unwrap();
        path
    };
    let fresh_dir = tempfile::tempdir().unwrap();
    let fresh_path = build(&fresh_dir);
    let fresh = snapshot(&open_and_migrate(&fresh_path).unwrap());
    let dir = tempfile::tempdir().unwrap();
    let path = build(&dir);
    rewind_to_v32(&Connection::open(&path).unwrap());
    assert_eq!(snapshot(&open_and_migrate(&path).unwrap()), fresh);
}

/// Held-witness characterisation. Private, opt-in: `KNXBENCH_WP05_WITNESS_XML` names a held
/// application XML and `KNXBENCH_WP05_WITNESS_SHA256` its expected SHA-256. Nothing from the
/// file is printed or asserted as a literal; identities and captions are only compared with
/// each other. Parser success alone is not the point: every rename leaf the file carries must
/// follow its own selector through the public load and evaluate entry points.
#[test]
#[ignore = "private held witness: set KNXBENCH_WP05_WITNESS_XML and KNXBENCH_WP05_WITNESS_SHA256"]
fn held_witness_leaves_follow_their_selectors_without_a_refusal() {
    let (Ok(path), Ok(expected)) = (
        std::env::var("KNXBENCH_WP05_WITNESS_XML"),
        std::env::var("KNXBENCH_WP05_WITNESS_SHA256"),
    ) else {
        panic!("the held witness is not configured");
    };
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(
        knx_productdb::sha256_hex(&bytes),
        expected,
        "not the held witness"
    );
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "witness/A.xml", &bytes).unwrap();
    let program: String = conn
        .query_row("SELECT id FROM application_program", [], |r| r.get(0))
        .unwrap();
    // (leaf node, leaf id, enclosing block id, target id, selector ref, selector test)
    type Leaf = (i64, String, String, String, String, String);
    let leaves: Vec<Leaf> = conn
        .prepare(
            "SELECT l.node_id, l.element_id, b.element_id, l.ref_id, c.ref_id, w.test
             FROM dynamic_node AS l
             JOIN dynamic_node AS w ON w.program_id = l.program_id AND w.node_id = l.parent_id
             JOIN dynamic_node AS c ON c.program_id = w.program_id AND c.node_id = w.parent_id
             JOIN dynamic_node AS b ON b.program_id = c.program_id AND b.node_id = c.parent_id
             WHERE l.program_id = ?1 AND l.kind IN ('ParameterBlockRename','Rename')
               AND l.module_def_id = '' AND w.kind = 'when' AND c.kind = 'choose'
               AND b.kind = 'ParameterBlock'
             ORDER BY l.node_id",
        )
        .unwrap()
        .query_map([&program], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let stored: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id = ?1
               AND kind IN ('ParameterBlockRename','Rename')",
            [&program],
            |r| r.get(0),
        )
        .unwrap();
    assert!(stored > 0, "the witness carries rename leaves");
    assert_eq!(
        leaves.len() as i64,
        stored,
        "every leaf is a literal leaf in a when"
    );
    assert!(
        leaves.iter().all(|l| l.2 == l.3),
        "each names its enclosing block"
    );
    let trees = trees_of(&conn, &program);
    let baseline = evaluate(&trees, &with_in(&conn, &program, &[]));
    assert!(baseline.renames.is_empty(), "the defaults activate no leaf");
    assert!(!baseline.diagnostics.iter().any(|d| matches!(
        &d.diagnostic,
        Diagnostic::UnrecognizedNode { kind, .. } if kind.ends_with("Rename")
    )));
    let mut followed = 0;
    for (node, _, _, _, selector, test) in &leaves {
        let activation = evaluate(
            &trees,
            &with_in(&conn, &program, &[(selector.as_str(), test.as_str())]),
        );
        let active: Vec<_> = activation
            .renames
            .iter()
            .filter(|r| r.node_id == *node)
            .collect();
        assert_eq!(active.len(), 1, "leaf {node} follows its selector");
        let block = active[0].target_node_id;
        let base = baseline.labels.iter().find(|l| l.node_id == block).unwrap();
        assert_ne!(
            active[0].text, base.text,
            "a replacement differs from its base"
        );
        assert!(
            activation
                .labels
                .iter()
                .any(|l| l.node_id == block && l.text == base.text),
            "the base label is untouched"
        );
        assert!(!activation.diagnostics.iter().any(|d| matches!(
            &d.diagnostic,
            Diagnostic::UnrecognizedNode { kind, .. } if kind.ends_with("Rename")
        )));
        followed += 1;
    }
    assert_eq!(followed, leaves.len());
    // The same loaded trees return to the base caption for the default selector value.
    let (_, _, _, _, selector, _) = &leaves[0];
    assert!(evaluate(
        &trees,
        &with_in(&conn, &program, &[(selector.as_str(), "0")])
    )
    .renames
    .is_empty());
}

fn trees_of(conn: &Connection, program: &str) -> ProgramTrees {
    load_program_trees(conn, program).unwrap()
}

#[test]
fn a_cross_target_or_undecided_leaf_keeps_a_valid_leaf_from_winning_its_block() {
    let (_dir, path) = installed();
    let conn = open_and_migrate(&path).unwrap();
    let trees = trees(&conn);
    let block = node_of(&conn, "PB-1");
    // Positive control under real default resolution: the other selector sits at its default.
    let alone = evaluate(&trees, &with_in(&conn, "A-1", &[("P-1_R-1", "1")]));
    assert_eq!(replacement(&alone, block), Some("fixture_switch_py"));
    // The other block's active leaf names this block: neither leaf wins it.
    let crossed = evaluate(
        &trees,
        &with_in(&conn, "A-1", &[("P-1_R-1", "1"), ("P-3_R-3", "1")]),
    );
    assert_eq!(replacement(&crossed, block), None);
    assert_eq!(refused(&crossed, node_of(&conn, "PR-1")), 1);
    assert_eq!(refused(&crossed, node_of(&conn, "PR-3")), 1);
    // An undecidable stored value hides that same leaf: still no winner, base kept.
    let undecided = evaluate(
        &trees,
        &with_in(&conn, "A-1", &[("P-1_R-1", "1"), ("P-3_R-3", "x")]),
    );
    assert_eq!(replacement(&undecided, block), None);
    assert_eq!(refused(&undecided, node_of(&conn, "PR-1")), 1);
    assert!(undecided
        .labels
        .iter()
        .any(|l| l.node_id == block && l.raw_text == "Base one" && l.text == "Base one"));
}

/// The fixture program under a second program id, for a package with two members.
fn second_program() -> String {
    String::from_utf8(PROGRAM.to_vec()).unwrap().replace(
        "ApplicationProgram Id=\"A-1\"",
        "ApplicationProgram Id=\"A-2\"",
    )
}

fn two_member_package() -> Vec<u8> {
    let second = second_program();
    package_of(&[
        ("M-0001/M-0001_A-1.xml", PROGRAM),
        ("M-0001/M-0001_A-2.xml", second.as_bytes()),
    ])
}

fn installed_two() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    install_package(&conn, "pair.knxprod", &two_member_package()).unwrap();
    (dir, path)
}

fn report_status(conn: &Connection) -> String {
    conn.query_row("SELECT status FROM package_install_report", [], |r| {
        r.get(0)
    })
    .unwrap()
}

fn report_failures(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT count(*) FROM ingest_unknown
         WHERE kind = 'InstallReportBackfillError' AND name = 'rename_report_backfill'",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn v32_to_v33_with_two_members_reproduces_a_fresh_install() {
    let (_fresh_dir, fresh_path) = installed_two();
    let fresh = snapshot(&open_and_migrate(&fresh_path).unwrap());
    let (_dir, path) = installed_two();
    rewind_to_v32(&Connection::open(&path).unwrap());
    assert_eq!(
        rename_attribute_unknowns(&Connection::open(&path).unwrap()).len(),
        2 * OLD_UNKNOWNS
    );
    assert_eq!(snapshot(&open_and_migrate(&path).unwrap()), fresh);
    assert_eq!(
        snapshot(&open_and_migrate(&path).unwrap()),
        fresh,
        "idempotent"
    );
}

#[test]
fn v32_to_v33_promotes_the_intact_member_and_names_the_package_whose_other_member_failed() {
    let (_dir, path) = installed_two();
    let (intact, damaged): (String, String);
    {
        let conn = Connection::open(&path).unwrap();
        let sha = |id: &str| -> String {
            conn.query_row(
                "SELECT source_sha256 FROM application_program WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap()
        };
        (intact, damaged) = (sha("A-1"), sha("A-2"));
        rewind_to_v32(&conn);
        // Parsable bytes, but not the bytes this key names.
        let other = second_program().replace("Name=\"Program\"", "Name=\"Altered\"");
        conn.execute(
            "UPDATE source_file SET bytes = ?2 WHERE sha256 = ?1",
            params![damaged, other.as_bytes()],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    let failures = |sha: &str| -> i64 {
        conn.query_row(
            "SELECT count(*) FROM ingest_unknown WHERE source_sha256 = ?1 AND kind = 'RenameBackfillError'",
            [sha],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!((failures(&intact), failures(&damaged)), (0, 1));
    let columns = |program: &str| -> Option<String> {
        conn.query_row(
            "SELECT text FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ''
               AND kind = 'ParameterBlockRename' ORDER BY node_id LIMIT 1",
            [program],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(
        columns("A-1"),
        some("fixture_switch_py"),
        "the intact member is promoted"
    );
    assert_eq!(
        columns("A-2"),
        None,
        "the damaged member keeps its old rows"
    );
    let kept: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE source_sha256 = ?1 AND kind = 'Attribute'
               AND (xpath LIKE '%/ParameterBlockRename' OR xpath LIKE '%/Rename')",
            [&damaged],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(kept, OLD_UNKNOWNS as i64, "its old evidence stays");
    assert_eq!(report_status(&conn), "unavailable");
    assert_eq!(report_failures(&conn), 1);
}

/// A rewound single-package store whose report is then damaged by `damage`.
fn migrate_with_damaged_report(damage: &str) -> Connection {
    let (dir, path) = installed();
    {
        let conn = Connection::open(&path).unwrap();
        rewind_to_v32(&conn);
        assert_eq!(conn.execute(damage, []).unwrap(), 1, "{damage}");
    }
    let conn = open_and_migrate(&path).unwrap();
    std::mem::forget(dir);
    conn
}

#[test]
fn v32_to_v33_does_not_rebuild_an_invalid_historical_report_as_measured() {
    for damage in [
        // The read counter disagrees with the rows it counts.
        "UPDATE package_install_count SET count = count + 1
         WHERE category = 'unknown_construct' AND disposition = 'read'",
        // A mandatory row is missing; absence is not zero.
        "DELETE FROM package_install_count WHERE rowid = (
             SELECT rowid FROM package_install_count
             WHERE count = 0 AND category <> 'unknown_construct' ORDER BY ordinal DESC LIMIT 1)",
    ] {
        let conn = migrate_with_damaged_report(damage);
        assert_eq!(report_status(&conn), "unavailable", "{damage}");
        assert_eq!(report_failures(&conn), 1, "{damage}");
        // The tree itself is promoted; only the report is not vouched for.
        assert_eq!(
            leaf(&conn, "PR-1"),
            (some("PR-1"), some("PB-1"), some("fixture_switch_py"), None)
        );
    }
}

#[test]
fn a_database_error_in_the_v33_report_rewrite_rolls_the_whole_step_back() {
    let (_fresh_dir, fresh_path) = installed();
    let fresh = snapshot(&open_and_migrate(&fresh_path).unwrap());
    let (_dir, path) = installed();
    let before = {
        let conn = Connection::open(&path).unwrap();
        rewind_to_v32(&conn);
        conn.execute_batch(
            "CREATE TRIGGER injected_failure BEFORE INSERT ON package_install_report
             BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();
        snapshot(&conn)
    };
    assert!(
        open_and_migrate(&path).is_err(),
        "a real database error is not a downgrade"
    );
    {
        let conn = Connection::open(&path).unwrap();
        assert_eq!(snapshot(&conn), before, "nothing of the step remains");
        conn.execute_batch("DROP TRIGGER injected_failure;")
            .unwrap();
    }
    assert_eq!(snapshot(&open_and_migrate(&path).unwrap()), fresh);
}

fn fixture_dir() -> std::path::PathBuf {
    std::env::var_os("KNXBENCH_WP05_V32_DIR")
        .expect("KNXBENCH_WP05_V32_DIR names the private fixture directory")
        .into()
}

/// Opt-in step 1 of the genuine old-store check: writes the invented packages a v32 build
/// installs (`pair.knxprod`, two members; `rival.knxprod`, an id-conflict loser of `A-1`).
#[test]
#[ignore = "private fixture producer: set KNXBENCH_WP05_V32_DIR"]
fn write_the_invented_packages_for_an_old_store() {
    let dir = fixture_dir();
    let rival = String::from_utf8(PROGRAM.to_vec())
        .unwrap()
        .replace("Name=\"Program\"", "Name=\"Rival\"");
    for (name, bytes) in [
        ("pair.knxprod", two_member_package()),
        ("rival.knxprod", package(rival.as_bytes())),
    ] {
        // Never replace a package a produced store was built from.
        let mut file = std::fs::File::create_new(dir.join(name)).unwrap();
        file.write_all(&bytes).unwrap();
    }
}

/// Opt-in step 3: `v32.sqlite` in the same directory was written by the v32 build itself from
/// step 1's packages. Each case migrates its own copy; the produced file is never opened.
#[test]
#[ignore = "private genuine v32 store: set KNXBENCH_WP05_V32_DIR"]
fn migrating_the_produced_v32_store_matches_a_fresh_install() {
    let dir = fixture_dir();
    let read = |name: &str| std::fs::read(dir.join(name)).unwrap();
    let (pair, rival) = (read("pair.knxprod"), read("rival.knxprod"));
    let copy = |label: &str| -> (tempfile::TempDir, std::path::PathBuf) {
        let scratch = tempfile::tempdir().unwrap();
        let path = scratch.path().join(format!("{label}.sqlite"));
        std::fs::copy(dir.join("v32.sqlite"), &path).unwrap();
        (scratch, path)
    };
    let fresh = {
        let scratch = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&scratch.path().join("fresh.sqlite")).unwrap();
        install_package(&conn, "pair.knxprod", &pair).unwrap();
        install_package(&conn, "rival.knxprod", &rival).unwrap();
        snapshot(&conn)
    };
    let sha = |bytes: &[u8]| knx_productdb::sha256_hex(bytes);
    let rival_program = rival_member(&rival);

    // The old store as the v32 build left it, asserted before anything migrates.
    let (_keep, intact) = copy("intact");
    {
        let conn = Connection::open(&intact).unwrap();
        assert_eq!(rows(&conn, "PRAGMA user_version"), [["Integer(32)"]]);
        let packages = rows(&conn, "SELECT sha256 FROM package ORDER BY sha256");
        let mut expected = [sha(&pair), sha(&rival)];
        expected.sort();
        assert_eq!(
            packages,
            expected
                .iter()
                .map(|s| vec![format!("Text({s:?})")])
                .collect::<Vec<_>>()
        );
        assert_eq!(leaf(&conn, "").0, None, "v32 kept the attributes in extra");
        // The conflict loser's own unknown rows and its report counted them in v32.
        let loser_unknowns: i64 = conn
            .query_row(
                "SELECT count(*) FROM ingest_unknown WHERE source_sha256 = ?1 AND kind = 'Attribute'
                   AND (xpath LIKE '%/ParameterBlockRename' OR xpath LIKE '%/Rename')",
                [&rival_program],
                |r| r.get(0),
            )
            .unwrap();
        let loser_report: i64 = conn
            .query_row(
                "SELECT coalesce(sum(occurrences), 0) FROM package_install_unknown
                 WHERE package_sha256 = ?1 AND kind = 'Attribute'
                   AND (xpath LIKE '%/ParameterBlockRename' OR xpath LIKE '%/Rename')",
                [sha(&rival)],
                |r| r.get(0),
            )
            .unwrap();
        // Ten distinct rows; 45 occurrences: 13 root leaves (one without `@Id`), the
        // converted leaf and the module's leaf, each with its attributes, plus `Foo`.
        assert_eq!((loser_unknowns, loser_report), (OLD_UNKNOWNS as i64, 45));
    }
    assert_eq!(snapshot(&open_and_migrate(&intact).unwrap()), fresh);
    assert_eq!(
        snapshot(&open_and_migrate(&intact).unwrap()),
        fresh,
        "idempotent"
    );

    // A member replaced by parsable bytes under its old key: only the intact member moves.
    let (_keep, mixed) = copy("mixed");
    {
        let conn = Connection::open(&mixed).unwrap();
        let damaged: String = conn
            .query_row(
                "SELECT source_sha256 FROM application_program WHERE id = 'A-2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let other = second_program().replace("Name=\"Program\"", "Name=\"Altered\"");
        conn.execute(
            "UPDATE source_file SET bytes = ?2 WHERE sha256 = ?1",
            params![damaged, other.as_bytes()],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&mixed).unwrap();
    let status: String = conn
        .query_row(
            "SELECT status FROM package_install_report WHERE package_sha256 = ?1",
            [sha(&pair)],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "unavailable");

    // Two independent historical report defects: neither is rebuilt as measured.
    for damage in [
        "UPDATE package_install_count SET count = count + 1
         WHERE package_sha256 = ?1 AND category = 'unknown_construct' AND disposition = 'read'",
        "DELETE FROM package_install_count WHERE rowid = (
             SELECT rowid FROM package_install_count WHERE package_sha256 = ?1
               AND count = 0 AND category <> 'unknown_construct' ORDER BY ordinal DESC LIMIT 1)",
    ] {
        let (_keep, damaged) = copy("report");
        assert_eq!(
            Connection::open(&damaged)
                .unwrap()
                .execute(damage, [sha(&pair)])
                .unwrap(),
            1
        );
        let conn = open_and_migrate(&damaged).unwrap();
        let status: String = conn
            .query_row(
                "SELECT status FROM package_install_report WHERE package_sha256 = ?1",
                [sha(&pair)],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "unavailable", "{damage}");
    }

    // A real database error rolls the step back, version and rows included.
    let (_keep, failing) = copy("failing");
    let before = {
        let conn = Connection::open(&failing).unwrap();
        conn.execute_batch(
            "CREATE TRIGGER injected_failure BEFORE INSERT ON package_install_report
             BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();
        snapshot(&conn)
    };
    assert!(open_and_migrate(&failing).is_err());
    assert_eq!(snapshot(&Connection::open(&failing).unwrap()), before);
}

/// The source key of the rival package's program member, as the archive stores it.
fn rival_member(rival: &[u8]) -> String {
    let mut archive = zip::ZipArchive::new(Cursor::new(rival)).unwrap();
    let mut member = archive.by_name("M-0001/M-0001_A-1.xml").unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut member, &mut bytes).unwrap();
    knx_productdb::sha256_hex(&bytes)
}
