//! Storage tests for the `ApplicationProgram/Dynamic` tree (T18 Task 1,
//! design D2-D4). These assert the tree lands losslessly, one row per
//! element, in document order — nothing here evaluates `@test`.
//!
//! T18 Task 2's evaluator unit tests (hand-built trees, no database) and
//! migration-backfill/corpus-evaluation tests live further down this same
//! file, after the storage tests above.

use std::collections::HashMap;
use std::io::{Cursor, Write};

use rusqlite::Connection;
use zip::write::SimpleFileOptions;

use knx_productdb::dynamic::{
    evaluate, load_program_trees, ActiveRef, ControlKind, Diagnostic, DynamicNode, DynamicTree,
    ModuleScope, Op, ProgramTrees, ScopedDiagnostic, Test, ValueMap,
};

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

/// One `ApplicationProgram` with a `Dynamic` tree exercising every element
/// kind the plan's step 1 lists, plus an unmodelled attribute on a known
/// kind (`Channel/@Mystery`) and one unknown element kind (`Weird`) nested
/// under a `default="true"` branch.
const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <ApplicationPrograms>
        <ApplicationProgram Id="M-006A_A-0001-22-26C0-O0079" Name="Presence" ApplicationVersion="22"
                            MaskVersion="MV-0701">
          <Static>
            <ParameterRefs>
              <ParameterRef Id="P-1_R-1" RefId="P-1" DisplayOrder="1" Tag="1" />
            </ParameterRefs>
          </Static>
          <Dynamic>
            <Channel Id="CH-1" Text="Channel A" Mystery="42">
              <ParameterBlock Id="PB-1" Name="Block1">
                <choose ParamRefId="P-1_R-1">
                  <when test="1">
                    <ParameterRefRef RefId="P-1_R-1" />
                    <ComObjectRefRef RefId="O-1_R-1" />
                  </when>
                  <when default="true">
                    <Weird Foo="bar" />
                  </when>
                </choose>
              </ParameterBlock>
            </Channel>
          </Dynamic>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

fn node(
    conn: &Connection,
    program_id: &str,
    module_def_id: &str,
    node_id: i64,
) -> (Option<i64>, i64, String, Option<String>, Option<String>) {
    conn.query_row(
        "SELECT parent_id, position, kind, element_id, ref_id
         FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ?2 AND node_id = ?3",
        (program_id, module_def_id, node_id),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )
    .unwrap()
}

#[test]
fn every_element_of_the_dynamic_tree_gets_one_row_in_document_order() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
    let pid = "M-006A_A-0001-22-26C0-O0079";

    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ''",
            [pid],
            |r| r.get(0),
        )
        .unwrap();
    // Dynamic, Channel, ParameterBlock, choose, when, ParameterRefRef,
    // ComObjectRefRef, when, Weird = 9 elements.
    assert_eq!(count, 9);

    let (parent, pos, kind, ..) = node(&conn, pid, "", 0);
    assert_eq!((parent, pos, kind.as_str()), (None, 0, "Dynamic"));

    let (parent, pos, kind, ..) = node(&conn, pid, "", 1);
    assert_eq!((parent, pos, kind.as_str()), (Some(0), 0, "Channel"));

    let (parent, pos, kind, ..) = node(&conn, pid, "", 2);
    assert_eq!((parent, pos, kind.as_str()), (Some(1), 0, "ParameterBlock"));

    let (parent, pos, kind, _, ref_id) = node(&conn, pid, "", 3);
    assert_eq!((parent, pos, kind.as_str()), (Some(2), 0, "choose"));
    assert_eq!(ref_id.as_deref(), Some("P-1_R-1"));

    let (parent, pos, kind, _, _) = node(&conn, pid, "", 4);
    assert_eq!((parent, pos, kind.as_str()), (Some(3), 0, "when"));
    let test: Option<String> = conn
        .query_row(
            "SELECT test FROM dynamic_node WHERE program_id = ?1 AND module_def_id = '' AND node_id = 4",
            [pid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(test.as_deref(), Some("1"));

    let (parent, pos, kind, _, ref_id) = node(&conn, pid, "", 5);
    assert_eq!(
        (parent, pos, kind.as_str()),
        (Some(4), 0, "ParameterRefRef")
    );
    assert_eq!(ref_id.as_deref(), Some("P-1_R-1"));

    let (parent, pos, kind, _, ref_id) = node(&conn, pid, "", 6);
    assert_eq!(
        (parent, pos, kind.as_str()),
        (Some(4), 1, "ComObjectRefRef")
    );
    assert_eq!(ref_id.as_deref(), Some("O-1_R-1"));

    let (parent, pos, kind, ..) = node(&conn, pid, "", 7);
    assert_eq!((parent, pos, kind.as_str()), (Some(3), 1, "when"));
    let is_default: Option<i64> = conn
        .query_row(
            "SELECT is_default FROM dynamic_node WHERE program_id = ?1 AND module_def_id = '' AND node_id = 7",
            [pid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(is_default, Some(1));

    let (parent, pos, kind, ..) = node(&conn, pid, "", 8);
    assert_eq!((parent, pos, kind.as_str()), (Some(7), 0, "Weird"));
}

#[test]
fn an_unknown_element_kind_is_stored_under_its_own_literal_name_not_a_bucket() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
    let kind: String = conn
        .query_row(
            "SELECT kind FROM dynamic_node WHERE kind = 'Weird'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(kind, "Weird");
    let bucketed: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE kind = 'Unknown'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(bucketed, 0);
}

#[test]
fn an_unmodelled_attribute_lands_in_extra_and_in_ingest_unknown() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();

    let extra: Option<String> = conn
        .query_row(
            "SELECT extra FROM dynamic_node WHERE kind = 'Channel'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(extra.as_deref(), Some("Mystery=42"));

    let reported: i64 = conn
        .query_row(
            "SELECT occurrences FROM ingest_unknown WHERE name = 'Mystery' AND kind = 'Attribute'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reported, 1);

    // The unknown element's own attribute is unmodelled too (its whole
    // kind is unrecognized), so it is reported and also lands in extra.
    let weird_extra: Option<String> = conn
        .query_row(
            "SELECT extra FROM dynamic_node WHERE kind = 'Weird'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(weird_extra.as_deref(), Some("Foo=bar"));
    let weird_reported: i64 = conn
        .query_row(
            "SELECT occurrences FROM ingest_unknown WHERE name = 'Foo' AND kind = 'Attribute'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(weird_reported, 1);
}

/// A `Name` on `ParameterBlock` is modelled (design D4) so it must not be
/// reported as unknown, even though there is no dedicated column for it —
/// it still lands in `extra` for data-integrity's sake.
#[test]
fn a_modelled_attribute_without_a_dedicated_column_is_kept_but_not_reported() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
    let extra: Option<String> = conn
        .query_row(
            "SELECT extra FROM dynamic_node WHERE kind = 'ParameterBlock'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(extra.as_deref(), Some("Name=Block1"));
    let reported: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE name = 'Name'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reported, 0);
}

/// Fix round 1, finding 1: `@default`'s only recognized spelling is the
/// literal string `"true"`. Any other spelling must not vanish — it stays
/// out of `is_default` (correct already) but must also stay *in* `extra`
/// and be reported like any other unmatched attribute, instead of being
/// silently excluded from both.
const NON_TRUE_DEFAULT: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006B">
      <ApplicationPrograms>
        <ApplicationProgram Id="M-006B_A-0002-22-26C0-O0080" Name="P" ApplicationVersion="22"
                            MaskVersion="MV-0701">
          <Static><ParameterRefs/></Static>
          <Dynamic>
            <Channel Id="CH-1">
              <ParameterBlock Id="PB-1">
                <choose ParamRefId="P-1_R-1">
                  <when default="false" />
                  <when default="1" />
                </choose>
              </ParameterBlock>
            </Channel>
          </Dynamic>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

#[test]
fn a_default_attribute_spelled_other_than_true_is_kept_in_extra_and_reported() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-006B/A.xml", NON_TRUE_DEFAULT.as_bytes()).unwrap();
    let pid = "M-006B_A-0002-22-26C0-O0080";

    // node_id 4 = first <when default="false">, node_id 5 = second
    // <when default="1">; both are children of node_id 3 (<choose>).
    for (node_id, raw) in [(4i64, "false"), (5i64, "1")] {
        let (is_default, extra): (Option<i64>, Option<String>) = conn
            .query_row(
                "SELECT is_default, extra FROM dynamic_node
                 WHERE program_id = ?1 AND module_def_id = '' AND node_id = ?2",
                (pid, node_id),
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            is_default, None,
            "node {node_id}: default={raw:?} is not \"true\""
        );
        assert_eq!(
            extra.as_deref(),
            Some(format!("default={raw}")).as_deref(),
            "node {node_id}: raw default=... must survive in extra"
        );
    }

    let reported: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE name = 'default' AND kind = 'Attribute'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        reported, 1,
        "both non-\"true\" default= spellings collapse into one reported construct"
    );
    let occurrences: i64 = conn
        .query_row(
            "SELECT occurrences FROM ingest_unknown WHERE name = 'default' AND kind = 'Attribute'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        occurrences, 2,
        "once per element carrying a non-\"true\" default="
    );
}

/// Fix round 1, finding 2: a self-closing `<ModuleDef Id="..."/>` (no
/// children of its own) must not leak its `module_def_id` forward onto the
/// program's own `Dynamic` tree that follows it in document order.
const SELF_CLOSING_MODULE_DEF: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer RefId="M-00FB">
<ApplicationPrograms><ApplicationProgram Id="M-00FB_A-0003-10-C071" Name="P" ApplicationVersion="10" MaskVersion="MV-0701">
<Static><ComObjectTable/><ComObjectRefs/></Static>
<ModuleDefs><ModuleDef Id="M-00FB_A-0003-10-C071_MD-1" Name="empty"/></ModuleDefs>
<Dynamic>
  <Channel Id="CH-1" />
</Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

#[test]
fn a_self_closing_module_def_does_not_leak_its_id_onto_the_programs_own_tree() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-00FB/A.xml", SELF_CLOSING_MODULE_DEF.as_bytes()).unwrap();
    let pid = "M-00FB_A-0003-10-C071";

    // The program's own <Dynamic> (Dynamic, Channel = 2 rows) must be keyed
    // by the empty sentinel, not by the self-closing ModuleDef's id.
    let program_rows: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ''",
            [pid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(program_rows, 2);

    let leaked: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node
             WHERE program_id = ?1 AND module_def_id = 'M-00FB_A-0003-10-C071_MD-1'",
            [pid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        leaked, 0,
        "the self-closing ModuleDef has no Dynamic tree of its own to leak into"
    );

    let (_, _, kind, ..) = node(&conn, pid, "", 1);
    assert_eq!(kind, "Channel");
}

const MODULE_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer RefId="M-00FA">
<ApplicationPrograms><ApplicationProgram Id="M-00FA_A-2504-10-C071" Name="P" ApplicationVersion="10" MaskVersion="MV-0701">
<Static><ComObjectTable/><ComObjectRefs/></Static>
<Dynamic>
  <Channel Id="CH-P">
    <choose ParamRefId="P-owner">
      <when test="1"><ParameterRefRef RefId="P-owner" /></when>
    </choose>
  </Channel>
</Dynamic>
<ModuleDefs><ModuleDef Id="M-00FA_A-2504-10-C071_MD-2" Name="module">
<Static>
<ComObjectTable>
  <ComObject Id="M-00FA_A-2504-10-C071_MD-2_O-2-0" Number="0" Text="OnOff" ObjectSize="1 Bit" DatapointType="DPST-1-1" WriteFlag="Enabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="M-00FA_A-2504-10-C071_MD-2_O-2-0_R-1" RefId="M-00FA_A-2504-10-C071_MD-2_O-2-0" />
</ComObjectRefs>
</Static>
<Dynamic>
  <Module Id="MOD-1" RefId="M-00FA_A-2504-10-C071_MD-2" />
</Dynamic>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

#[test]
fn a_module_defs_own_dynamic_tree_is_stored_keyed_by_its_own_id_while_the_program_tree_uses_the_empty_sentinel(
) {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-00FA/A.xml", MODULE_PROGRAM.as_bytes()).unwrap();
    let pid = "M-00FA_A-2504-10-C071";
    let mdid = "M-00FA_A-2504-10-C071_MD-2";

    let program_rows: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ''",
            [pid],
            |r| r.get(0),
        )
        .unwrap();
    // Dynamic, Channel, choose, when, ParameterRefRef = 5.
    assert_eq!(program_rows, 5);

    let module_rows: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ?2",
            [pid, mdid],
            |r| r.get(0),
        )
        .unwrap();
    // Dynamic, Module = 2.
    assert_eq!(module_rows, 2);

    let (module_kind, module_ref): (String, Option<String>) = conn
        .query_row(
            "SELECT kind, ref_id FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ?2 AND node_id = 1",
            [pid, mdid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(module_kind, "Module");
    assert_eq!(module_ref.as_deref(), Some(mdid));
}

/// Plan Task 1 step 6: the real corpus's `choose`/`when` counts must match
/// RESEARCH.md §4.3's table exactly, per archive, and every stored
/// `choose/@ParamRefId` must resolve against `parameter_ref` — 0 dangling,
/// matching the spike's own finding on these same files.
#[test]
fn corpus_choose_and_when_counts_match_research_and_every_choose_resolves() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../OriginalData/ProductDatabases")
        });
    if !root.exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }

    for (name, expected_choose, expected_when) in [
        ("646704-04_ETS4_2012_47_DE_EN.knxprod", 1646i64, 2252i64),
        ("Weinzierl_730_KNX_IP_Interface_ETS4.knxprod", 5, 5),
        ("MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod", 509, 982),
        ("Dummy_Applikation_Secure.knxprod", 0, 0),
    ] {
        let bytes = std::fs::read(root.join(name)).unwrap_or_else(|e| {
            panic!(
                "corpus fixture {name} unavailable: {e}; set KNXBENCH_PRODUCT_CORPUS to \
                 OriginalData/ProductDatabases"
            )
        });
        let (_dir, conn) = db();
        knx_productdb::install_package(&conn, name, &bytes).unwrap();

        let choose: i64 = conn
            .query_row(
                "SELECT count(*) FROM dynamic_node WHERE kind = 'choose'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let when: i64 = conn
            .query_row(
                "SELECT count(*) FROM dynamic_node WHERE kind = 'when'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        eprintln!(
            "corpus {name}: choose={choose} (expected {expected_choose}), when={when} (expected {expected_when})"
        );
        assert_eq!(
            (choose, when),
            (expected_choose, expected_when),
            "{name}: choose/when counts"
        );

        let dangling: i64 = conn
            .query_row(
                "SELECT count(*) FROM dynamic_node c
                 WHERE c.kind = 'choose' AND c.ref_id IS NOT NULL
                   AND NOT EXISTS (
                       SELECT 1 FROM parameter_ref p
                       WHERE p.program_id = c.program_id AND p.id = c.ref_id
                   )",
                [],
                |r| r.get(0),
            )
            .unwrap();
        eprintln!("corpus {name}: dangling choose/@ParamRefId={dangling}");
        assert_eq!(dangling, 0, "{name}: dangling choose/@ParamRefId");
    }

    // The fifth archive is byte-identical to the second and must contribute
    // no new dynamic_node rows: `install_package` recognizes the repeat by
    // whole-package content hash and skips re-parsing outright.
    let bytes_a = std::fs::read(root.join("Weinzierl_730_KNX_IP_Interface_ETS4.knxprod")).unwrap();
    let bytes_b =
        std::fs::read(root.join("Weinzierl_730_KNX_IP_Interface_ETS4_v1.knxprod")).unwrap();
    assert_eq!(
        knx_productdb::sha256_hex(&bytes_a),
        knx_productdb::sha256_hex(&bytes_b),
        "corpus assumption: these two archives are byte-identical"
    );
    let (_dir, conn) = db();
    knx_productdb::install_package(&conn, "a.knxprod", &bytes_a).unwrap();
    let count = |conn: &Connection, kind: &str| -> i64 {
        conn.query_row(
            "SELECT count(*) FROM dynamic_node WHERE kind = ?1",
            [kind],
            |r| r.get(0),
        )
        .unwrap()
    };
    let before = (count(&conn, "choose"), count(&conn, "when"));
    assert_eq!(before, (5, 5));
    let report = knx_productdb::install_package(&conn, "b.knxprod", &bytes_b).unwrap();
    assert!(
        report.skipped,
        "a byte-identical package must be recognized as already installed"
    );
    let after = (count(&conn, "choose"), count(&conn, "when"));
    eprintln!(
        "corpus v1 duplicate: before={before:?} after={after:?} (skipped={})",
        report.skipped
    );
    assert_eq!(
        before, after,
        "the byte-identical fifth archive must add no new rows"
    );
}

fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in members {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers>
    <Manufacturer Id="M-006A" Name="Example"/></Manufacturers></MasterData></KNX>"#;

/// Fix round 1, finding 4: `package::install_package`'s whole-package
/// content-hash dedup only ever catches a byte-identical *package*. Two
/// different packages that happen to carry the same `ApplicationProgram`
/// bytes (a common real shape — the same program shipped inside more than
/// one manufacturer archive) must still reach `parse_dynamic_trees` a
/// second time, via the `parse_existing` path
/// (`ingest_file_in_transaction`), and that second pass must not duplicate
/// rows or collide on `dynamic_node`'s primary key.
#[test]
fn two_different_packages_sharing_one_application_programs_bytes_do_not_duplicate_its_dynamic_tree()
{
    let (_dir, conn) = db();
    let first = archive(&[
        ("knx_master.xml", MASTER),
        ("M-006A/A.xml", PROGRAM.as_bytes()),
    ]);
    // A second, differently-named package with an unrelated extra member —
    // different whole-package sha256, same ApplicationProgram bytes at
    // "M-006A/A.xml" — mirrors `retries_keep_conflicts_and_unknown_paths_-
    // cannot_supply_parsed_rows` in tests/standalone_packages.rs, the
    // Hardware-flavoured precedent for this fixture shape.
    let second = archive(&[
        ("knx_master.xml", MASTER),
        ("M-006A/A.xml", PROGRAM.as_bytes()),
        ("notes.xml", b"<Root/>"),
    ]);

    knx_productdb::install_package(&conn, "first.knxprod", &first).unwrap();
    let pid = "M-006A_A-0001-22-26C0-O0079";
    let before: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ''",
            [pid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(before, 9);

    let report = knx_productdb::install_package(&conn, "second.knxprod", &second).unwrap();
    assert!(
        !report.skipped,
        "the second package's bytes differ from the first's, so it is not itself skipped"
    );

    let after: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ''",
            [pid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        after, before,
        "the shared ApplicationProgram's Dynamic tree must not be duplicated"
    );
}

// ---------------------------------------------------------------------
// T18 Task 2: evaluator unit tests, over hand-built trees. No database.
// ---------------------------------------------------------------------

/// A `DynamicNode` with every field defaulted except the three every test
/// below sets explicitly; `..nd(...)` overrides the rest per call site.
fn nd(node_id: i64, parent_id: Option<i64>, kind: &str) -> DynamicNode {
    DynamicNode {
        node_id,
        parent_id,
        kind: kind.to_string(),
        element_id: None,
        ref_id: None,
        test: None,
        is_default: false,
        control_kind: None,
    }
}

fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// An `ActiveRef` in the program's own (unscoped) tree — the shape every
/// pre-T18-slice-2 unit test's activations had before `Activation` grew a
/// `ModuleScope`.
fn active(id: &str) -> ActiveRef {
    ActiveRef {
        scope: None,
        ref_id: id.to_string(),
    }
}

/// A `ScopedDiagnostic` in the program's own (unscoped) tree.
fn diag(diagnostic: Diagnostic) -> ScopedDiagnostic {
    ScopedDiagnostic {
        scope: None,
        diagnostic,
    }
}

#[test]
fn test_parse_recognizes_all_six_operators_a_single_number_and_a_space_separated_list() {
    assert_eq!(Test::parse("1").unwrap(), Test::Single(1));
    assert_eq!(Test::parse("1 2 3").unwrap(), Test::List(vec![1, 2, 3]));
    assert_eq!(Test::parse("=1").unwrap(), Test::Compare(Op::Eq, 1));
    assert_eq!(Test::parse("!=1").unwrap(), Test::Compare(Op::Ne, 1));
    assert_eq!(Test::parse(">1").unwrap(), Test::Compare(Op::Gt, 1));
    assert_eq!(Test::parse("<1").unwrap(), Test::Compare(Op::Lt, 1));
    assert_eq!(Test::parse(">=1").unwrap(), Test::Compare(Op::Ge, 1));
    assert_eq!(Test::parse("<=1").unwrap(), Test::Compare(Op::Le, 1));
    // The longest-operator-first matching order must not misread `>=`/`<=`
    // as `>`/`<` followed by a malformed remainder.
    assert!(Test::parse(">=").is_err());
    assert!(Test::parse("not-a-number").is_err());
}

/// One `choose` per operator, each fed the value that must match, proving
/// the parsed `Test` actually drives `evaluate`'s branch selection and not
/// just `Test::parse` in isolation.
#[test]
fn every_comparison_operator_selects_its_matching_when_through_evaluate() {
    for (op_str, observed, should_match) in [
        ("=1", "1", true),
        ("=1", "2", false),
        ("!=1", "2", true),
        ("!=1", "1", false),
        (">1", "2", true),
        (">1", "1", false),
        ("<1", "0", true),
        ("<1", "1", false),
        (">=1", "1", true),
        (">=1", "0", false),
        ("<=1", "1", true),
        ("<=1", "2", false),
    ] {
        let tree = DynamicTree::from_nodes(vec![
            DynamicNode {
                control_kind: Some(ControlKind::Comparable),
                ref_id: Some("P".into()),
                ..nd(0, None, "choose")
            },
            DynamicNode {
                test: Some(op_str.to_string()),
                ..nd(1, Some(0), "when")
            },
            DynamicNode {
                ref_id: Some("PRR".into()),
                ..nd(2, Some(1), "ParameterRefRef")
            },
        ]);
        let activation = evaluate(
            &ProgramTrees::single(tree),
            &values(&[("P", observed)]).into(),
        );
        assert_eq!(
            !activation.parameter_refs.is_empty(),
            should_match,
            "op={op_str} observed={observed}"
        );
        if should_match {
            assert!(
                activation.diagnostics.is_empty(),
                "op={op_str}: {:?}",
                activation.diagnostics
            );
        } else {
            // No default branch in this fixture, so a non-matching value
            // falls through to `NoBranchMatched` — the no-match policy
            // applies equally here, it is not a test-harness quirk.
            assert_eq!(
                activation.diagnostics,
                vec![diag(Diagnostic::NoBranchMatched {
                    choose_node: 0,
                    param_ref: Some("P".to_string()),
                    observed_value: observed.to_string(),
                })],
                "op={op_str} observed={observed}"
            );
        }
    }
}

/// `Test::List`: a space-separated list matches any of its members.
#[test]
fn a_space_separated_list_test_matches_any_member() {
    let tree = DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::Comparable),
            ref_id: Some("P".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            test: Some("1 2 3".to_string()),
            ..nd(1, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("PRR".into()),
            ..nd(2, Some(1), "ParameterRefRef")
        },
    ]);
    let activation = evaluate(&ProgramTrees::single(tree), &values(&[("P", "2")]).into());
    assert_eq!(activation.parameter_refs, vec![active("PRR")]);
    assert!(activation.diagnostics.is_empty());
}

/// `@default="true"` covers whatever no `@test` branch matches.
#[test]
fn a_default_when_covers_what_no_test_matches() {
    let tree = DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::Comparable),
            ref_id: Some("P".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            test: Some("1".to_string()),
            ..nd(1, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("MATCH".into()),
            ..nd(2, Some(1), "ParameterRefRef")
        },
        DynamicNode {
            is_default: true,
            ..nd(3, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("FALLBACK".into()),
            ..nd(4, Some(3), "ParameterRefRef")
        },
    ]);
    let activation = evaluate(&ProgramTrees::single(tree), &values(&[("P", "999")]).into());
    assert_eq!(activation.parameter_refs, vec![active("FALLBACK")]);
    assert!(activation.diagnostics.is_empty());
}

/// No `when` matches and there is no default: nothing activates, and
/// `NoBranchMatched` is reported.
#[test]
fn no_matching_branch_and_no_default_activates_nothing_and_is_reported() {
    let tree = DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::Comparable),
            ref_id: Some("P".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            test: Some("1".to_string()),
            ..nd(1, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("MATCH".into()),
            ..nd(2, Some(1), "ParameterRefRef")
        },
    ]);
    let activation = evaluate(&ProgramTrees::single(tree), &values(&[("P", "999")]).into());
    assert!(activation.parameter_refs.is_empty());
    assert!(activation.com_object_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![diag(Diagnostic::NoBranchMatched {
            choose_node: 0,
            param_ref: Some("P".to_string()),
            observed_value: "999".to_string(),
        })]
    );
}

/// A `choose` whose controlling value is missing from the `ValueMap`
/// entirely: `MissingValue`, nothing activated.
#[test]
fn a_missing_controlling_value_is_reported_and_activates_nothing() {
    let tree = DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::Comparable),
            ref_id: Some("P".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            test: Some("1".to_string()),
            ..nd(1, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("MATCH".into()),
            ..nd(2, Some(1), "ParameterRefRef")
        },
    ]);
    let activation = evaluate(&ProgramTrees::single(tree), &values(&[]).into());
    assert!(activation.parameter_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![diag(Diagnostic::MissingValue {
            choose_node: 0,
            param_ref: Some("P".to_string()),
        })]
    );
}

/// A controlling value present but not a legal `Condition_t` number:
/// `NonNumericValue`, nothing activated.
#[test]
fn a_non_numeric_controlling_value_is_reported_and_activates_nothing() {
    let tree = DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::Comparable),
            ref_id: Some("P".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            test: Some("1".to_string()),
            ..nd(1, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("MATCH".into()),
            ..nd(2, Some(1), "ParameterRefRef")
        },
    ]);
    let activation = evaluate(
        &ProgramTrees::single(tree),
        &values(&[("P", "not-a-number")]).into(),
    );
    assert!(activation.parameter_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![diag(Diagnostic::NonNumericValue {
            choose_node: 0,
            param_ref: Some("P".to_string()),
            raw: "not-a-number".to_string(),
        })]
    );
}

/// A `when/@test` that fails to parse is skipped (never matches) but
/// reported, and evaluation continues to later siblings — an unparsable
/// test does not abort the whole `choose`.
#[test]
fn an_unparsable_test_is_reported_and_evaluation_continues_to_later_siblings() {
    let tree = DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::Comparable),
            ref_id: Some("P".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            test: Some("garbage".to_string()),
            ..nd(1, Some(0), "when")
        },
        DynamicNode {
            test: Some("1".to_string()),
            ..nd(2, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("MATCH".into()),
            ..nd(3, Some(2), "ParameterRefRef")
        },
    ]);
    let activation = evaluate(&ProgramTrees::single(tree), &values(&[("P", "1")]).into());
    assert_eq!(activation.parameter_refs, vec![active("MATCH")]);
    assert_eq!(
        activation.diagnostics,
        vec![diag(Diagnostic::UnparsableTest {
            when_node: 1,
            raw: "garbage".to_string(),
        })]
    );
}

/// D9: a `TypeNone`-controlled `choose` whose sole child is `when
/// default="true"` takes that branch with no comparison and no diagnostic.
#[test]
fn a_type_none_choose_with_its_sole_default_branch_activates_it_without_diagnostics() {
    let tree = DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::TypeNone),
            ref_id: Some("P".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            is_default: true,
            ..nd(1, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("ALWAYS".into()),
            ..nd(2, Some(1), "ParameterRefRef")
        },
    ]);
    let activation = evaluate(&ProgramTrees::single(tree), &values(&[]).into());
    assert_eq!(activation.parameter_refs, vec![active("ALWAYS")]);
    assert!(activation.diagnostics.is_empty());
}

/// D9: any other shape under a `TypeNone`-controlled `choose` (here: two
/// children instead of the sole default) is `UnexpectedTypeNoneShape`,
/// nothing activated.
#[test]
fn a_type_none_choose_of_any_other_shape_is_reported_and_activates_nothing() {
    let tree = DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::TypeNone),
            ref_id: Some("P".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            is_default: true,
            ..nd(1, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("A".into()),
            ..nd(2, Some(1), "ParameterRefRef")
        },
        DynamicNode {
            is_default: true,
            ..nd(3, Some(0), "when")
        },
    ]);
    let activation = evaluate(&ProgramTrees::single(tree), &values(&[]).into());
    assert!(activation.parameter_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![diag(Diagnostic::UnexpectedTypeNoneShape { choose_node: 0 })]
    );
}

/// D10: an unrecognized element kind activates nothing, is reported, and
/// its subtree is not descended (a `ParameterRefRef` nested under it must
/// not activate).
#[test]
fn an_unrecognized_element_kind_is_reported_and_its_subtree_is_not_descended() {
    let tree = DynamicTree::from_nodes(vec![
        nd(0, None, "Weird"),
        DynamicNode {
            ref_id: Some("HIDDEN".into()),
            ..nd(1, Some(0), "ParameterRefRef")
        },
    ]);
    let activation = evaluate(&ProgramTrees::single(tree), &values(&[]).into());
    assert!(activation.parameter_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![diag(Diagnostic::UnrecognizedNode {
            node_id: 0,
            kind: "Weird".to_string(),
        })]
    );
}

// The plan's step 9 replaces `a_module_node_is_recognized_but_not_expanded`
// (asserted `ModuleNotExpanded`, a variant this slice removes) with the
// step 8 tests below, which cover every `Module`-expansion outcome
// (expanded-and-scoped, `ModuleDefNotFound`, `NestedModuleNotExpanded`,
// a scoped non-Module diagnostic, and an empty `ModuleDef` tree) more
// precisely than the one test it replaces ever did.

/// D17/AC#4: a `Module` with no `@RefId` at all yields `ModuleDefNotFound`
/// and activates nothing.
#[test]
fn a_module_with_no_ref_id_yields_module_def_not_found() {
    let program = DynamicTree::from_nodes(vec![nd(0, None, "Module")]);
    let activation = evaluate(&ProgramTrees::single(program), &values(&[]).into());
    assert!(activation.parameter_refs.is_empty());
    assert!(activation.com_object_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![diag(Diagnostic::ModuleDefNotFound {
            node_id: 0,
            ref_id: None,
        })]
    );
}

/// D17/AC#4: a `Module` naming a `ModuleDef` with no stored tree for this
/// program also yields `ModuleDefNotFound` — the `modules` map is simply
/// empty here (`ProgramTrees::single`), standing in for "no such scope was
/// ever loaded."
#[test]
fn a_module_naming_an_absent_module_def_yields_module_def_not_found() {
    let program = DynamicTree::from_nodes(vec![DynamicNode {
        ref_id: Some("MD-GHOST".into()),
        ..nd(0, None, "Module")
    }]);
    let activation = evaluate(&ProgramTrees::single(program), &values(&[]).into());
    assert!(activation.parameter_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![diag(Diagnostic::ModuleDefNotFound {
            node_id: 0,
            ref_id: Some("MD-GHOST".to_string()),
        })]
    );
}

/// D15/AC#3: a `Module` inside a `ModuleDef`'s own tree is not expanded —
/// `NestedModuleNotExpanded`, no activations, regardless of whether the
/// nested `Module`'s own `@RefId` would otherwise resolve.
#[test]
fn a_module_inside_a_module_defs_tree_is_reported_not_expanded() {
    let program = DynamicTree::from_nodes(vec![DynamicNode {
        element_id: Some("M-A".into()),
        ref_id: Some("MD-1".into()),
        ..nd(0, None, "Module")
    }]);
    let module_def = DynamicTree::from_nodes(vec![DynamicNode {
        ref_id: Some("MD-1".into()), // even a self-reference is not followed
        ..nd(0, None, "Module")
    }]);
    let trees =
        ProgramTrees::from_parts(program, HashMap::from([("MD-1".to_string(), module_def)]));
    let activation = evaluate(&trees, &values(&[]).into());
    assert!(activation.parameter_refs.is_empty());
    assert!(activation.com_object_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![ScopedDiagnostic {
            scope: Some(ModuleScope {
                module_node: 0,
                module_id: Some("M-A".to_string()),
                module_def_id: "MD-1".to_string(),
            }),
            diagnostic: Diagnostic::NestedModuleNotExpanded {
                node_id: 0,
                ref_id: Some("MD-1".to_string()),
            },
        }]
    );
}

/// D14: a diagnostic raised *inside* a module's expansion (here:
/// `NoBranchMatched`, chosen because it exercises `evaluate_comparable_choose`)
/// comes back carrying the instantiating `Module`'s scope, not `None` —
/// the only thing that makes the node-id collision D14 names survivable.
#[test]
fn a_diagnostic_raised_inside_a_module_carries_that_modules_scope() {
    let program = DynamicTree::from_nodes(vec![DynamicNode {
        element_id: Some("M-A".into()),
        ref_id: Some("MD-1".into()),
        ..nd(0, None, "Module")
    }]);
    let module_def = DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::Comparable),
            ref_id: Some("P".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            test: Some("1".to_string()),
            ..nd(1, Some(0), "when")
        },
    ]);
    let trees =
        ProgramTrees::from_parts(program, HashMap::from([("MD-1".to_string(), module_def)]));
    let activation = evaluate(&trees, &values(&[("P", "999")]).into());
    assert_eq!(
        activation.diagnostics,
        vec![ScopedDiagnostic {
            scope: Some(ModuleScope {
                module_node: 0,
                module_id: Some("M-A".to_string()),
                module_def_id: "MD-1".to_string(),
            }),
            diagnostic: Diagnostic::NoBranchMatched {
                choose_node: 0,
                param_ref: Some("P".to_string()),
                observed_value: "999".to_string(),
            },
        }]
    );
}

/// D17's last line: a `Module` whose `ModuleDef` tree exists but is
/// genuinely empty activates nothing and raises no diagnostic — an empty
/// tree is not itself evidence of anything malformed.
#[test]
fn a_module_with_an_empty_module_def_tree_activates_nothing_and_is_silent() {
    let program = DynamicTree::from_nodes(vec![DynamicNode {
        element_id: Some("M-A".into()),
        ref_id: Some("MD-1".into()),
        ..nd(0, None, "Module")
    }]);
    let module_def = DynamicTree::from_nodes(vec![]);
    let trees =
        ProgramTrees::from_parts(program, HashMap::from([("MD-1".to_string(), module_def)]));
    let activation = evaluate(&trees, &values(&[]).into());
    assert!(activation.parameter_refs.is_empty());
    assert!(activation.com_object_refs.is_empty());
    assert!(activation.diagnostics.is_empty());
}

/// D18: within one module's expansion, a ref reachable through two active
/// branches still appears once — the same first-occurrence dedup rule
/// D11 established, now proven to hold *inside* a `ModuleScope` too.
#[test]
fn within_one_module_scope_a_ref_reachable_twice_is_deduplicated_once() {
    let program = DynamicTree::from_nodes(vec![DynamicNode {
        element_id: Some("M-A".into()),
        ref_id: Some("MD-1".into()),
        ..nd(0, None, "Module")
    }]);
    let module_def = DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        DynamicNode {
            ref_id: Some("SHARED".into()),
            ..nd(1, Some(0), "ParameterRefRef")
        },
        DynamicNode {
            control_kind: Some(ControlKind::TypeNone),
            ref_id: Some("P".into()),
            ..nd(2, Some(0), "choose")
        },
        DynamicNode {
            is_default: true,
            ..nd(3, Some(2), "when")
        },
        DynamicNode {
            ref_id: Some("SHARED".into()),
            ..nd(4, Some(3), "ParameterRefRef")
        },
    ]);
    let trees =
        ProgramTrees::from_parts(program, HashMap::from([("MD-1".to_string(), module_def)]));
    let activation = evaluate(&trees, &values(&[]).into());
    assert!(
        activation.diagnostics.is_empty(),
        "{:?}",
        activation.diagnostics
    );
    assert_eq!(
        activation.parameter_refs,
        vec![ActiveRef {
            scope: Some(ModuleScope {
                module_node: 0,
                module_id: Some("M-A".to_string()),
                module_def_id: "MD-1".to_string(),
            }),
            ref_id: "SHARED".to_string(),
        }]
    );
}

/// D14/D18, AC#2 — the regression this whole slice exists for. Two
/// `Module` elements in the program's own tree instantiate the *same*
/// `ModuleDef`, whose tree declares one `ComObjectRefRef`. Reusing the old
/// flat `HashSet<String>` dedup would collapse both instantiations into a
/// single activation; the fix is a dedup key qualified by the
/// instantiating `Module`'s own `node_id` (`ModuleScope::module_node`).
#[test]
fn two_modules_instantiating_one_module_def_produce_two_scoped_activations() {
    let program = DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        DynamicNode {
            element_id: Some("M-A".into()),
            ref_id: Some("MD-1".into()),
            ..nd(1, Some(0), "Module")
        },
        DynamicNode {
            element_id: Some("M-B".into()),
            ref_id: Some("MD-1".into()),
            ..nd(2, Some(0), "Module")
        },
    ]);
    let module_def = DynamicTree::from_nodes(vec![DynamicNode {
        ref_id: Some("O-1_R-1".into()),
        ..nd(0, None, "ComObjectRefRef")
    }]);
    let trees =
        ProgramTrees::from_parts(program, HashMap::from([("MD-1".to_string(), module_def)]));
    let activation = evaluate(&trees, &values(&[]).into());
    assert!(
        activation.diagnostics.is_empty(),
        "{:?}",
        activation.diagnostics
    );
    assert_eq!(
        activation.com_object_refs,
        vec![
            ActiveRef {
                scope: Some(ModuleScope {
                    module_node: 1,
                    module_id: Some("M-A".to_string()),
                    module_def_id: "MD-1".to_string(),
                }),
                ref_id: "O-1_R-1".to_string(),
            },
            ActiveRef {
                scope: Some(ModuleScope {
                    module_node: 2,
                    module_id: Some("M-B".to_string()),
                    module_def_id: "MD-1".to_string(),
                }),
                ref_id: "O-1_R-1".to_string(),
            },
        ]
    );
}

/// A `ModuleDef` tree whose `choose` is gated on a declared ref shared by
/// every instantiation — the KV shape from E2 (five instantiations, one
/// declared `ParameterRef`, five divergent stored values). Two `Module`
/// elements instantiate it: `M-A` carries its own scoped value that flips
/// the `choose`'s outcome, `M-B` carries none. D36's core claim: `M-A`'s
/// activation reflects its own value, and `M-B` — untouched — still sees
/// the program default, not `M-A`'s value and not a shared one.
fn module_def_gated_on_p() -> DynamicTree {
    DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::Comparable),
            ref_id: Some("P".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            test: Some(">10".to_string()),
            ..nd(1, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("HIGH".into()),
            ..nd(2, Some(1), "ParameterRefRef")
        },
        DynamicNode {
            is_default: true,
            ..nd(3, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("LOW".into()),
            ..nd(4, Some(3), "ParameterRefRef")
        },
    ])
}

fn two_module_a_b_program() -> DynamicTree {
    DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        DynamicNode {
            element_id: Some("M-A".into()),
            ref_id: Some("MD-1".into()),
            ..nd(1, Some(0), "Module")
        },
        DynamicNode {
            element_id: Some("M-B".into()),
            ref_id: Some("MD-1".into()),
            ..nd(2, Some(0), "Module")
        },
    ])
}

#[test]
fn a_scoped_value_wins_for_its_own_instantiation_and_the_other_sees_the_program_default() {
    let program = two_module_a_b_program();
    let module_def = module_def_gated_on_p();
    let trees =
        ProgramTrees::from_parts(program, HashMap::from([("MD-1".to_string(), module_def)]));
    let mut vm: ValueMap = values(&[("P", "5")]).into();
    vm.insert_scoped("M-A".to_string(), "P".to_string(), "20".to_string());
    let activation = evaluate(&trees, &vm);
    assert!(
        activation.diagnostics.is_empty(),
        "{:?}",
        activation.diagnostics
    );
    assert_eq!(
        activation.parameter_refs,
        vec![
            ActiveRef {
                scope: Some(ModuleScope {
                    module_node: 1,
                    module_id: Some("M-A".to_string()),
                    module_def_id: "MD-1".to_string(),
                }),
                ref_id: "HIGH".to_string(),
            },
            ActiveRef {
                scope: Some(ModuleScope {
                    module_node: 2,
                    module_id: Some("M-B".to_string()),
                    module_def_id: "MD-1".to_string(),
                }),
                ref_id: "LOW".to_string(),
            },
        ]
    );
}

// ---------------------------------------------------------------------
// Task 5: the fixture E2 says the repository has never had — a
// module-scoped `ParameterRef` that both holds divergent per-channel
// values *and* controls a `choose` in the same `ModuleDef`. `module_def_
// gated_on_p()` above proves the value side (D36); this proves the other
// half, with ids shaped like a real corpus program instead of the loose
// `"M-A"`/`"M-B"`/`"P"` placeholders those unit tests use elsewhere.
// ---------------------------------------------------------------------

/// Two `Module` instantiations of one `ModuleDef`, id-shaped like KV's own
/// switch actuator [V] (design doc `2026-09-12-module-scoped-editing-
/// design.md` §E1's table): declared `ParameterRef`
/// `M-00FA_A-2504-10-C071_MD-2_P-1_R-1`, program-side `Module/@Id`s
/// `M-00FA_A-2504-10-C071_MD-2_M-4`/`_M-5` — two of the five real KV
/// channels (`M-2`..`M-6`, E2), not the bare `M-<n>` shape an earlier
/// fixture in this repository was corrected away from.
fn kv_shaped_two_module_program() -> DynamicTree {
    DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        DynamicNode {
            element_id: Some("M-00FA_A-2504-10-C071_MD-2_M-4".into()),
            ref_id: Some("M-00FA_A-2504-10-C071_MD-2".into()),
            ..nd(1, Some(0), "Module")
        },
        DynamicNode {
            element_id: Some("M-00FA_A-2504-10-C071_MD-2_M-5".into()),
            ref_id: Some("M-00FA_A-2504-10-C071_MD-2".into()),
            ..nd(2, Some(0), "Module")
        },
    ])
}

/// The `ModuleDef` tree E2 says never existed anywhere in this repository:
/// its `choose` gates on `M-00FA_A-2504-10-C071_MD-2_P-1_R-1` — the exact
/// declared ref [V] E2 measured holding five genuinely different stored
/// values (`17`, `33`, `49`, `32`, `48`) across KV's real `M-2`..`M-6`
/// instantiations — and the high branch also activates a `ComObjectRefRef`
/// the low branch never reaches. [A]: everything below the gate (the
/// `>10` threshold, `_P-2_R-1`/`_P-3_R-1`/`_O-1_R-1`, and the branch
/// structure itself) is this fixture's own invention, not an observed
/// program; KV's real `MD-2` declares no `choose` at all. This is a
/// mirror of the id grammar plus the one combination E2 flags as
/// untested, not a claim that any KNX package looks like this.
fn module_def_gated_on_kv_shaped_scoped_ref() -> DynamicTree {
    DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::Comparable),
            ref_id: Some("M-00FA_A-2504-10-C071_MD-2_P-1_R-1".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            test: Some(">10".to_string()),
            ..nd(1, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("M-00FA_A-2504-10-C071_MD-2_P-2_R-1".into()),
            ..nd(2, Some(1), "ParameterRefRef")
        },
        DynamicNode {
            ref_id: Some("M-00FA_A-2504-10-C071_MD-2_O-1_R-1".into()),
            ..nd(3, Some(1), "ComObjectRefRef")
        },
        DynamicNode {
            is_default: true,
            ..nd(4, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("M-00FA_A-2504-10-C071_MD-2_P-3_R-1".into()),
            ..nd(5, Some(4), "ParameterRefRef")
        },
    ])
}

/// Item 1 of the Task 5 brief: with the scoped values supplied, the two
/// instantiations activate genuinely different sets, named by their exact
/// `ref_id`s — not just different counts. `M-4` carries its own stored
/// value (`20`, `>10`) and selects the high branch's parameter and com
/// object; `M-5`, with no stored row of its own, falls back to the
/// program default (`5`, not `>10`) and selects the low branch's
/// parameter alone.
#[test]
fn two_kv_shaped_instantiations_with_divergent_scoped_values_activate_different_refs() {
    let program = kv_shaped_two_module_program();
    let module_def = module_def_gated_on_kv_shaped_scoped_ref();
    let trees = ProgramTrees::from_parts(
        program,
        HashMap::from([("M-00FA_A-2504-10-C071_MD-2".to_string(), module_def)]),
    );
    let mut vm: ValueMap = values(&[("M-00FA_A-2504-10-C071_MD-2_P-1_R-1", "5")]).into();
    vm.insert_scoped(
        "M-00FA_A-2504-10-C071_MD-2_M-4".to_string(),
        "M-00FA_A-2504-10-C071_MD-2_P-1_R-1".to_string(),
        "20".to_string(),
    );
    let activation = evaluate(&trees, &vm);
    assert!(
        activation.diagnostics.is_empty(),
        "{:?}",
        activation.diagnostics
    );

    let params_for = |module_id: &str| -> Vec<&str> {
        activation
            .parameter_refs
            .iter()
            .filter(|r| r.scope.as_ref().and_then(|s| s.module_id.as_deref()) == Some(module_id))
            .map(|r| r.ref_id.as_str())
            .collect()
    };
    let coms_for = |module_id: &str| -> Vec<&str> {
        activation
            .com_object_refs
            .iter()
            .filter(|r| r.scope.as_ref().and_then(|s| s.module_id.as_deref()) == Some(module_id))
            .map(|r| r.ref_id.as_str())
            .collect()
    };

    assert_eq!(
        params_for("M-00FA_A-2504-10-C071_MD-2_M-4"),
        vec!["M-00FA_A-2504-10-C071_MD-2_P-2_R-1"],
        "M-4's own scoped value (20, >10) selects the high branch"
    );
    assert_eq!(
        params_for("M-00FA_A-2504-10-C071_MD-2_M-5"),
        vec!["M-00FA_A-2504-10-C071_MD-2_P-3_R-1"],
        "M-5, untouched, still sees the program default (5, not >10) and selects the low branch"
    );
    assert_eq!(
        coms_for("M-00FA_A-2504-10-C071_MD-2_M-4"),
        vec!["M-00FA_A-2504-10-C071_MD-2_O-1_R-1"],
        "only the high branch's ComObjectRef activates, and only for M-4"
    );
    assert!(
        coms_for("M-00FA_A-2504-10-C071_MD-2_M-5").is_empty(),
        "the low branch activates no ComObjectRef"
    );
}

/// Item 2 of the Task 5 brief: the explicit contrast, kept as the
/// pre-T18-slice-4 behaviour. With no stored per-channel rows at all —
/// not even one — both instantiations fall back to the identical unscoped
/// program default and therefore agree on the same active set.
#[test]
fn without_any_scoped_rows_kv_shaped_instantiations_agree_on_the_program_default() {
    let program = kv_shaped_two_module_program();
    let module_def = module_def_gated_on_kv_shaped_scoped_ref();
    let trees = ProgramTrees::from_parts(
        program,
        HashMap::from([("M-00FA_A-2504-10-C071_MD-2".to_string(), module_def)]),
    );
    let vm: ValueMap = values(&[("M-00FA_A-2504-10-C071_MD-2_P-1_R-1", "5")]).into();
    let activation = evaluate(&trees, &vm);
    assert!(
        activation.diagnostics.is_empty(),
        "{:?}",
        activation.diagnostics
    );

    let params_for = |module_id: &str| -> Vec<&str> {
        activation
            .parameter_refs
            .iter()
            .filter(|r| r.scope.as_ref().and_then(|s| s.module_id.as_deref()) == Some(module_id))
            .map(|r| r.ref_id.as_str())
            .collect()
    };
    let m4 = params_for("M-00FA_A-2504-10-C071_MD-2_M-4");
    let m5 = params_for("M-00FA_A-2504-10-C071_MD-2_M-5");
    assert_eq!(m4, vec!["M-00FA_A-2504-10-C071_MD-2_P-3_R-1"]);
    assert_eq!(
        m5, m4,
        "no scoped rows anywhere: both instantiations see the identical program default"
    );
    assert!(
        activation.com_object_refs.is_empty(),
        "the default branch activates no ComObjectRef, for either instantiation"
    );
}

/// D36: a scoped value never leaks into a top-level (`scope: None`) read
/// of the same declared ref id.
#[test]
fn a_scoped_value_never_leaks_into_a_top_level_read() {
    let mut vm: ValueMap = values(&[("P", "5")]).into();
    vm.insert_scoped("M-A".to_string(), "P".to_string(), "20".to_string());
    assert_eq!(vm.get(None, "P"), Some("5"));
}

/// D36: a scoped value stored for one `module_id` never answers a lookup
/// under another — the lookup falls back to the program level, not
/// sideways to a sibling instantiation.
#[test]
fn a_scoped_value_for_one_module_id_never_answers_a_lookup_under_another() {
    let mut vm: ValueMap = values(&[("P", "5")]).into();
    vm.insert_scoped("M-A".to_string(), "P".to_string(), "20".to_string());
    let scope_b = ModuleScope {
        module_node: 2,
        module_id: Some("M-B".to_string()),
        module_def_id: "MD-1".to_string(),
    };
    assert_eq!(vm.get(Some(&scope_b), "P"), Some("5"));
}

/// `get_unscoped` ignores scoped values entirely, even when a scoped value
/// for the same ref id exists.
#[test]
fn get_unscoped_ignores_scoped_values_entirely() {
    let mut vm: ValueMap = values(&[("P", "5")]).into();
    vm.insert_scoped("M-A".to_string(), "P".to_string(), "20".to_string());
    assert_eq!(vm.get_unscoped("P"), Some("5"));
    assert_eq!(vm.len_scoped(), 1);
}

/// D37: a `Module` element with no `@Id` is reported exactly once, at its
/// expansion site, and its subtree still evaluates — against the unscoped
/// map, since a nameless instantiation can never be matched to a scoped
/// value.
#[test]
fn module_without_id_is_reported_once_and_its_subtree_still_evaluates_from_the_unscoped_map() {
    let program = DynamicTree::from_nodes(vec![DynamicNode {
        element_id: None,
        ref_id: Some("MD-1".into()),
        ..nd(0, None, "Module")
    }]);
    let module_def = DynamicTree::from_nodes(vec![
        DynamicNode {
            control_kind: Some(ControlKind::Comparable),
            ref_id: Some("P".into()),
            ..nd(0, None, "choose")
        },
        DynamicNode {
            test: Some(">1".to_string()),
            ..nd(1, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("HIT".into()),
            ..nd(2, Some(1), "ParameterRefRef")
        },
    ]);
    let trees =
        ProgramTrees::from_parts(program, HashMap::from([("MD-1".to_string(), module_def)]));
    let vm: ValueMap = values(&[("P", "5")]).into();
    let activation = evaluate(&trees, &vm);
    assert_eq!(
        activation.diagnostics,
        vec![ScopedDiagnostic {
            scope: None,
            diagnostic: Diagnostic::ModuleWithoutId { node_id: 0 },
        }]
    );
    assert_eq!(
        activation.parameter_refs,
        vec![ActiveRef {
            scope: Some(ModuleScope {
                module_node: 0,
                module_id: None,
                module_def_id: "MD-1".to_string(),
            }),
            ref_id: "HIT".to_string(),
        }]
    );
}

/// D11: the same ref, reachable through two different active branches,
/// appears exactly once in the result, at its first (document-order)
/// position.
#[test]
fn a_ref_reachable_through_two_branches_is_deduplicated_by_first_occurrence() {
    let tree = DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        DynamicNode {
            ref_id: Some("SHARED".into()),
            ..nd(1, Some(0), "ParameterRefRef")
        },
        DynamicNode {
            control_kind: Some(ControlKind::TypeNone),
            ref_id: Some("P".into()),
            ..nd(2, Some(0), "choose")
        },
        DynamicNode {
            is_default: true,
            ..nd(3, Some(2), "when")
        },
        DynamicNode {
            ref_id: Some("SHARED".into()),
            ..nd(4, Some(3), "ParameterRefRef")
        },
    ]);
    let activation = evaluate(&ProgramTrees::single(tree), &values(&[]).into());
    assert_eq!(activation.parameter_refs, vec![active("SHARED")]);
    assert!(activation.diagnostics.is_empty());
}

// ---------------------------------------------------------------------
// T18 Task 2: migration backfill (`migrate_v2_to_v3`'s
// `backfill_dynamic_nodes`, design D5).
// ---------------------------------------------------------------------

/// Proves the backfill actually inserts rows and is not a silent no-op:
/// a program's `dynamic_node` rows are wiped, the database rolled back to
/// `user_version = 2` (the table dropped, `application_program` and
/// `source_file` left untouched, exactly as a pre-Task-1 v2 database would
/// look), then `open_and_migrate` reruns `migrate_v2_to_v3` with no
/// re-install — the tree must reappear from the stored blob alone.
///
/// This is the test `program_should_be_skipped`'s doc comment (`dynamic/parse.rs`)
/// says the sha256-matching skip conditions are deliberately *not* keyed on:
/// without that deliberate choice, `application_program.source_sha256`
/// already trivially matches its own blob at this point, and the backfill
/// would find every program pre-skipped and insert nothing.
#[test]
fn migrating_from_v2_backfills_dynamic_node_from_stored_blobs_without_a_reinstall() {
    let (dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
    let pid = "M-006A_A-0001-22-26C0-O0079";

    let before: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ''",
            [pid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        before, 9,
        "sanity: the program's tree is stored before rollback"
    );

    // Roll the schema back to v2: drop only `dynamic_node` (the table
    // Task 1's migration added), leave every other v3 table — including
    // `application_program` and `source_file`, both already populated by
    // `ingest_file` above — alone. This mirrors the existing
    // `migrating_v1_preserves_existing_rows_and_blobs` idiom in
    // `standalone_packages.rs`, one version further along the chain.
    // `translation` is also rolled back to its pre-Task-1 (v0-v1) shape,
    // alongside `dynamic_node`: `db()` already ran the full chain up to
    // v4, so `translation` already has `scope`/`scope_id`, and rerunning
    // `migrate_v3_to_v4`'s rebuild against a table that is already in its
    // own target shape would fail looking for the `program_id` column it
    // expects to migrate away from.
    conn.execute_batch(
        "DROP TABLE dynamic_node;
         DROP INDEX translation_lookup;
         DROP TABLE translation;
         CREATE TABLE translation (
             program_id     TEXT NOT NULL,
             language       TEXT NOT NULL,
             ref_id         TEXT NOT NULL,
             attribute_name TEXT NOT NULL,
             text           TEXT,
             PRIMARY KEY (program_id, language, ref_id, attribute_name)
         ) STRICT;
         CREATE INDEX translation_lookup ON translation (program_id, language, ref_id);
         PRAGMA user_version = 2;",
    )
    .unwrap();
    drop(conn);

    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );

    let after: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ''",
            [pid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        after, 9,
        "the backfill must recreate the tree from the stored blob alone, with no re-install"
    );
}

/// One `source_file` blob deliberately broken (a mismatched end tag, which
/// `quick-xml`'s well-formedness check rejects — confirmed empirically to
/// surface as a genuine `ProductDbError::Xml` from `parse_dynamic_trees`
/// itself, unlike plain truncation, which that function's own event loop
/// treats as ordinary `Eof`, not an error) must not abort the v2-to-v3
/// backfill for every other blob in the same database.
#[test]
fn a_parse_failure_during_the_v2_to_v3_backfill_does_not_abort_the_migration() {
    let (dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
    let good_pid = "M-006A_A-0001-22-26C0-O0079";

    let broken: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-BAD">
      <ApplicationPrograms>
        <ApplicationProgram Id="M-BAD_A-9999-1-0000-O0000" Name="Broken" ApplicationVersion="1"
                            MaskVersion="MV-0701">
          <Static><ParameterRefs/></Static>
          <Dynamic>
            <Channel></Weird>
          </Dynamic>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;
    let broken_sha = knx_productdb::sha256_hex(broken);
    // Inserted directly rather than through `ingest_file`: the ordinary
    // ingest path parses `Dynamic` in the same pass as `Static`, so a file
    // this broken would already fail at install time, before ever reaching
    // this migration. A pre-Task-1 v2 database could only have acquired a
    // blob like this by installing it back when nothing read `Dynamic` at
    // all — plausible history, reproduced directly here rather than
    // through a v3-shaped API that would refuse it.
    conn.execute(
        "INSERT INTO source_file (sha256, source_path, len, bytes) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![broken_sha, "M-BAD/A.xml", broken.len() as i64, broken],
    )
    .unwrap();

    // `translation` is also rolled back to its pre-Task-1 (v0-v1) shape,
    // alongside `dynamic_node`: `db()` already ran the full chain up to
    // v4, so `translation` already has `scope`/`scope_id`, and rerunning
    // `migrate_v3_to_v4`'s rebuild against a table that is already in its
    // own target shape would fail looking for the `program_id` column it
    // expects to migrate away from.
    conn.execute_batch(
        "DROP TABLE dynamic_node;
         DROP INDEX translation_lookup;
         DROP TABLE translation;
         CREATE TABLE translation (
             program_id     TEXT NOT NULL,
             language       TEXT NOT NULL,
             ref_id         TEXT NOT NULL,
             attribute_name TEXT NOT NULL,
             text           TEXT,
             PRIMARY KEY (program_id, language, ref_id, attribute_name)
         ) STRICT;
         CREATE INDEX translation_lookup ON translation (program_id, language, ref_id);
         PRAGMA user_version = 2;",
    )
    .unwrap();
    drop(conn);

    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4,
        "a single blob's parse failure must not abort the migration"
    );

    let good_after: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ''",
            [good_pid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        good_after, 9,
        "the good blob's tree must still be backfilled despite the broken one"
    );

    let recorded: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown
             WHERE source_sha256 = ?1 AND kind = 'DynamicBackfillError'",
            [&broken_sha],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        recorded, 1,
        "the broken blob's failure must be recorded, not silently dropped"
    );

    // Per-file atomicity: `parse_dynamic_trees` inserts rows incrementally
    // as it walks the XML events (`<Channel>` lands before the mismatched
    // `</Weird>` end tag is ever seen), so without an inner boundary around
    // this call the broken program would keep whatever rows it managed to
    // write before the error surfaced. That would contradict this crate's
    // own documented per-file atomicity invariant (`ingest.rs`'s "a parse
    // error partway through leaves the database exactly as it was") for
    // exactly this code path. Every row for the broken program, under any
    // `module_def_id`, must be gone — not just the top-level `''` tree.
    let broken_rows: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id = ?1",
            ["M-BAD_A-9999-1-0000-O0000"],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        broken_rows, 0,
        "a mid-file parse failure must leave zero dynamic_node rows for that program, \
         not the partial prefix parsed before the error"
    );
}

// ---------------------------------------------------------------------
// T18 Task 2: corpus evaluation. Follows the same `KNXBENCH_PRODUCT_CORPUS`
// env-override / loud-skip idiom as
// `corpus_choose_and_when_counts_match_research_and_every_choose_resolves`
// above.
// ---------------------------------------------------------------------

/// Loads and evaluates every `choose` in the real corpus (defaults only —
/// no supplied values) and checks it against RESEARCH.md §4.3's own
/// findings on these same archives: no `when/@test` fails to parse, no
/// `choose/@ParamRefId` fails to resolve, and every `TypeNone`-controlled
/// `choose` has exactly the one-default-`when` shape the spike found in all
/// 604 corpus occurrences.
#[test]
fn corpus_evaluation_matches_research_no_unparsable_tests_no_unresolved_refs_and_type_none_holds() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../OriginalData/ProductDatabases")
        });
    if !root.exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }

    for name in [
        "646704-04_ETS4_2012_47_DE_EN.knxprod",
        "Weinzierl_730_KNX_IP_Interface_ETS4.knxprod",
        "MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod",
        "Dummy_Applikation_Secure.knxprod",
    ] {
        let bytes = std::fs::read(root.join(name)).unwrap_or_else(|e| {
            panic!(
                "corpus fixture {name} unavailable: {e}; set KNXBENCH_PRODUCT_CORPUS to \
                 OriginalData/ProductDatabases"
            )
        });
        let (_dir, conn) = db();
        knx_productdb::install_package(&conn, name, &bytes).unwrap();

        let program_ids: Vec<String> = conn
            .prepare("SELECT DISTINCT program_id FROM dynamic_node WHERE module_def_id = ''")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        let mut unparsable_tests = 0usize;
        let mut unresolved_param_refs = 0usize;
        let mut unexpected_type_none_shapes = 0usize;
        let mut other_diagnostics: Vec<Diagnostic> = Vec::new();
        let mut choose_count = 0usize;
        let mut type_none_choose_count = 0usize;

        for program_id in &program_ids {
            let tree = knx_productdb::dynamic::load_tree(&conn, program_id, "").unwrap();
            let values =
                knx_productdb::dynamic::resolve_values(&conn, program_id, &HashMap::new()).unwrap();
            // `ProgramTrees::single`, not `load_program_trees`: this test's
            // own docstring below states it evaluates only each program's
            // own top-level tree, `Module`/`ModuleDef` deliberately
            // unevaluated here — Task 1 keeps that scope unchanged; Task 2
            // adds the module-expanding corpus coverage.
            let activation = evaluate(&ProgramTrees::single(tree), &values);
            for sd in activation.diagnostics {
                match sd.diagnostic {
                    Diagnostic::UnparsableTest { .. } => unparsable_tests += 1,
                    Diagnostic::UnresolvedParamRef { .. } => unresolved_param_refs += 1,
                    Diagnostic::UnexpectedTypeNoneShape { .. } => unexpected_type_none_shapes += 1,
                    other => other_diagnostics.push(other),
                }
            }

            let rows: i64 = conn
                .query_row(
                    "SELECT count(*) FROM dynamic_node
                     WHERE program_id = ?1 AND module_def_id = '' AND kind = 'choose'",
                    [program_id],
                    |r| r.get(0),
                )
                .unwrap();
            choose_count += rows as usize;

            let type_none_rows: i64 = conn
                .query_row(
                    "SELECT count(*) FROM dynamic_node c
                     JOIN parameter_ref pr ON pr.program_id = c.program_id AND pr.id = c.ref_id
                     JOIN parameter p ON p.program_id = pr.program_id AND p.id = pr.parameter_id
                     JOIN parameter_type pt ON pt.program_id = p.program_id AND pt.id = p.parameter_type_id
                     WHERE c.program_id = ?1 AND c.module_def_id = '' AND c.kind = 'choose'
                       AND pt.kind = 'None'",
                    [program_id],
                    |r| r.get(0),
                )
                .unwrap();
            type_none_choose_count += type_none_rows as usize;
        }

        eprintln!(
            "corpus {name}: choose={choose_count} type_none_choose={type_none_choose_count} \
             unparsable_tests={unparsable_tests} unresolved_param_refs={unresolved_param_refs} \
             unexpected_type_none_shapes={unexpected_type_none_shapes} other_diagnostics={}",
            other_diagnostics.len()
        );

        assert_eq!(
            unparsable_tests, 0,
            "{name}: every when/@test in the researched corpus is a legal Condition_t"
        );
        assert_eq!(
            unresolved_param_refs, 0,
            "{name}: every choose/@ParamRefId in the researched corpus resolves"
        );
        assert_eq!(
            unexpected_type_none_shapes, 0,
            "{name}: RESEARCH.md §4.3 found the sole-default-when shape in all \
             604 corpus TypeNone occurrences"
        );

        // Design AC#3 / plan Task 2 step 7: not just "0 UnparsableTest", but
        // that every stored `when` lands in the *right* `Condition_t` shape
        // bucket (RESEARCH.md §4.3's table: `SINGLE_INTEGER`,
        // `DEFAULT_ATTR(true)`, `SPACE_LIST_OF_INTEGERS`, `OP_NUMBER`).
        // Classified straight from the stored `test`/`is_default` columns,
        // independently of `evaluate`'s own diagnostics (which, in *this*
        // test, only walk each program's own `module_def_id = ''` tree —
        // this test still calls `evaluate` with `ProgramTrees::single`, not
        // `load_program_trees`, so any `Module` node here yields
        // `ModuleDefNotFound` rather than being expanded; the corpus test
        // that does expand `Module` into its `ModuleDef` tree is
        // `corpus_module_expansion_resolves_every_prod3_module_and_grows_activation_counts`,
        // below). §4.3's own per-archive `when` totals (2252/5/982/0, the
        // same ones `corpus_choose_and_when_counts_match_research_and_every_choose_resolves`
        // already proves) count *every* stored `when` regardless of
        // `module_def_id`, so this counts the same way — over the whole
        // freshly-installed, single-archive database, not just the
        // evaluated top-level trees — to actually be comparable to that
        // table. This also independently re-verifies "0 UnparsableTest"
        // over rows `evaluate` never reaches (`ModuleDef` trees): any
        // stored `test` that fails `Test::parse` here panics on the spot.
        let mut single_integer = 0usize;
        let mut default_attr_true = 0usize;
        let mut space_list = 0usize;
        let mut op_number = 0usize;
        let mut when_total = 0usize;

        let mut stmt = conn
            .prepare("SELECT test, is_default FROM dynamic_node WHERE kind = 'when'")
            .unwrap();
        let rows: Vec<(Option<String>, Option<i64>)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        drop(stmt);

        for (test, is_default) in rows {
            when_total += 1;
            if is_default == Some(1) {
                default_attr_true += 1;
                continue;
            }
            match test.as_deref().map(Test::parse) {
                Some(Ok(Test::Single(_))) => single_integer += 1,
                Some(Ok(Test::List(_))) => space_list += 1,
                Some(Ok(Test::Compare(_, _))) => op_number += 1,
                other => panic!(
                    "{name}: when row with test={test:?} is_default={is_default:?} \
                     classified as {other:?} — every stored @test must be a legal Condition_t"
                ),
            }
        }

        eprintln!(
            "corpus {name}: when_total={when_total} SINGLE_INTEGER={single_integer} \
             DEFAULT_ATTR(true)={default_attr_true} SPACE_LIST_OF_INTEGERS={space_list} \
             OP_NUMBER={op_number}"
        );

        // RESEARCH.md §4.3 states directly, whole-corpus: SINGLE_INTEGER
        // 19138, DEFAULT_ATTR(true) 3417, SPACE_LIST_OF_INTEGERS 62,
        // OP_NUMBER(>) 13 — "all 13 in one prod3 application program". It
        // does not publish a per-archive breakdown of the first three
        // shapes, only the per-archive `choose`/`when` totals (the
        // "Corpus evidence table") and that `prod3` is the *only* archive
        // with any `OP_NUMBER` at all. Those are the facts this asserts
        // directly; the per-archive SINGLE_INTEGER/DEFAULT_ATTR/SPACE_LIST
        // split itself is not written down anywhere in the document (the
        // whole-corpus totals sum in seven archives, three of which —
        // `ez4`, `ez630`, `kv25` — are `.knxproj` samples outside this
        // four-archive `.knxprod`-only corpus test), so it is measured
        // here from the real corpus rather than copied, then locked in and
        // cross-checked against every constraint §4.3 *does* state:
        assert_eq!(
            single_integer + default_attr_true + space_list + op_number,
            when_total,
            "{name}: every when classifies into exactly one shape"
        );
        match name {
            "646704-04_ETS4_2012_47_DE_EN.knxprod" => {
                assert_eq!(
                    when_total, 2252,
                    "{name}: matches the evidence table's when count"
                );
                assert_eq!(
                    op_number, 0,
                    "{name}: §4.3 states all 13 OP_NUMBER instances belong to prod3 alone"
                );
                assert_eq!(single_integer, 1669, "{name}: measured shape split");
                assert_eq!(default_attr_true, 583, "{name}: measured shape split");
                assert_eq!(space_list, 0, "{name}: measured shape split");
            }
            "Weinzierl_730_KNX_IP_Interface_ETS4.knxprod" => {
                assert_eq!(
                    when_total, 5,
                    "{name}: matches the evidence table's when count"
                );
                assert_eq!(
                    op_number, 0,
                    "{name}: §4.3 states all 13 OP_NUMBER instances belong to prod3 alone"
                );
                assert_eq!(single_integer, 1, "{name}: measured shape split");
                assert_eq!(default_attr_true, 4, "{name}: measured shape split");
                assert_eq!(space_list, 0, "{name}: measured shape split");
            }
            "MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod" => {
                assert_eq!(
                    when_total, 982,
                    "{name}: matches the evidence table's when count"
                );
                assert_eq!(
                    op_number, 13,
                    "{name}: §4.3 states all 13 corpus-wide OP_NUMBER instances are in prod3"
                );
                assert_eq!(single_integer, 907, "{name}: measured shape split");
                assert_eq!(default_attr_true, 0, "{name}: measured shape split");
                assert_eq!(
                    space_list, 62,
                    "{name}: measured shape split — also equal to the whole-corpus \
                     SPACE_LIST_OF_INTEGERS total (§4.3: 62), so all corpus-wide \
                     instances of this shape are in prod3 too, same as OP_NUMBER"
                );
            }
            "Dummy_Applikation_Secure.knxprod" => {
                assert_eq!(
                    when_total, 0,
                    "{name}: genuinely empty Static/Dynamic tree per §4.3"
                );
                assert_eq!(single_integer, 0, "{name}: measured shape split");
                assert_eq!(default_attr_true, 0, "{name}: measured shape split");
                assert_eq!(space_list, 0, "{name}: measured shape split");
                assert_eq!(op_number, 0, "{name}: measured shape split");
            }
            other => panic!("unexpected corpus archive {other} in this test's own list"),
        }
    }
}

// ---------------------------------------------------------------------
// T18 Task 2 continued: corpus `Module` expansion. Same
// `KNXBENCH_PRODUCT_CORPUS` env-override / loud-skip idiom as the two tests
// above. Expected numbers below are not fitted to what `evaluate` prints:
// they come from `/home/knxbench/.claude/jobs/8098e9e6/tmp/derive_module_counts.py`,
// an independent, from-scratch reimplementation of this same evaluator
// algorithm in Python, driven straight off the raw `ApplicationProgram` XML
// (no sqlite, no knx_productdb). See the Task 2 report for its full output
// and the corpus-derivation method.
//
// RESEARCH.md §4.4 Q7's distribution table lists seven module-bearing
// application programs: three in `prod3`, four in `kv25`. Only `prod3`
// (`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`) is reachable here —
// `kv25` is a `.knxproj` demo project under `OriginalData/DemoProjects/`,
// not one of the four `.knxprod` archives these corpus tests install. The
// other three archives (`prod1`, `prod2`, `prod4`) contain zero `ModuleDef`
// and zero `Module` and are this test's module-free control group.
// ---------------------------------------------------------------------

/// AC#6: every `prod3` `Module/@RefId` resolves (zero `ModuleDefNotFound`),
/// nesting never occurs (zero `NestedModuleNotExpanded`), and expansion
/// strictly grows each program's activation count over the program-tree-only
/// baseline, by the exact amounts derived independently in Python.
///
/// The corpus's own default parameter values only ever steer every one of
/// prod3's per-channel "operating mode" `choose`s onto its first `ModuleDef`
/// (`..._MD-1`); the sibling `Module`s naming `MD-2`/`MD-3`/`MD-4` structurally
/// exist (`corpus_choose_and_when_counts_match_research_and_every_choose_resolves`
/// already pins 44/28/14 total `Module` rows per program) but sit on branches
/// the defaults never select, so `evaluate` never even reaches them — this is
/// why "distinct `ModuleScope`s" below (12/8/4) is smaller than the raw
/// `Module` row count (44/28/14): the former counts instantiations `evaluate`
/// actually walks under real default values, the latter counts every stored
/// `Module` element regardless of reachability. Both are real, independently
/// derived numbers; they are not expected to agree, and the difference is the
/// finding, not a bug in either count (see the Task 2 report).
#[test]
fn corpus_module_expansion_resolves_every_prod3_module_and_grows_activation_counts() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../OriginalData/ProductDatabases")
        });
    if !root.exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }

    let name = "MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod";
    let bytes = std::fs::read(root.join(name)).unwrap_or_else(|e| {
        panic!(
            "corpus fixture {name} unavailable: {e}; set KNXBENCH_PRODUCT_CORPUS to \
             OriginalData/ProductDatabases"
        )
    });
    let (_dir, conn) = db();
    knx_productdb::install_package(&conn, name, &bytes).unwrap();

    let program_ids: Vec<String> = conn
        .prepare("SELECT DISTINCT program_id FROM dynamic_node WHERE module_def_id = ''")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        program_ids.len(),
        3,
        "{name}: three module-bearing application programs, per §4.4 Q7"
    );

    for program_id in &program_ids {
        let single_tree = knx_productdb::dynamic::load_tree(&conn, program_id, "").unwrap();
        let values =
            knx_productdb::dynamic::resolve_values(&conn, program_id, &HashMap::new()).unwrap();
        let single = evaluate(&ProgramTrees::single(single_tree), &values);
        let single_total = single.parameter_refs.len() + single.com_object_refs.len();

        let trees = load_program_trees(&conn, program_id).unwrap();
        let full = evaluate(&trees, &values);
        let full_total = full.parameter_refs.len() + full.com_object_refs.len();

        let module_def_not_found = full
            .diagnostics
            .iter()
            .filter(|sd| matches!(sd.diagnostic, Diagnostic::ModuleDefNotFound { .. }))
            .count();
        let nested_not_expanded = full
            .diagnostics
            .iter()
            .filter(|sd| matches!(sd.diagnostic, Diagnostic::NestedModuleNotExpanded { .. }))
            .count();
        // Task 1's review nominated this loop for the check E2 already
        // states as fact: `Module/@Id` is present on 102/102 corpus
        // elements. A `ModuleWithoutId` here would mean this build can no
        // longer match a stored per-channel value to its channel on a
        // program the corpus actually ships — worth failing loudly on,
        // not just leaving to go unnoticed by the two counts above.
        let module_without_id = full
            .diagnostics
            .iter()
            .filter(|sd| matches!(sd.diagnostic, Diagnostic::ModuleWithoutId { .. }))
            .count();

        let mut scope_nodes: std::collections::HashSet<i64> = std::collections::HashSet::new();
        for r in full
            .parameter_refs
            .iter()
            .chain(full.com_object_refs.iter())
        {
            if let Some(scope) = &r.scope {
                scope_nodes.insert(scope.module_node);
            }
        }
        for sd in &full.diagnostics {
            if let Some(scope) = &sd.scope {
                scope_nodes.insert(scope.module_node);
            }
        }
        let distinct_scopes = scope_nodes.len();

        eprintln!(
            "corpus {name} program {program_id}: single_total={single_total} \
             full_total={full_total} module_def_not_found={module_def_not_found} \
             nested_not_expanded={nested_not_expanded} module_without_id={module_without_id} \
             distinct_scopes={distinct_scopes}"
        );

        assert_eq!(
            module_def_not_found, 0,
            "{program_id}: AC#6 — every Module/@RefId in the corpus resolves"
        );
        assert_eq!(
            nested_not_expanded, 0,
            "{program_id}: AC#6 — §4.4 Q6's zero-nesting finding, enforced as a regression"
        );
        assert_eq!(
            module_without_id, 0,
            "{program_id}: E2 — Module/@Id present on 102/102 corpus elements; \
             a regression here would silently strand this program's stored \
             per-channel values"
        );
        assert!(
            full_total > single_total,
            "{program_id}: expansion must strictly grow the activation count \
             (single={single_total}, full={full_total})"
        );

        let (expected_single, expected_full, expected_scopes) = match program_id.as_str() {
            "M-0083_A-0317-31-7DC6" => (22, 382, 12),
            "M-0083_A-0318-31-DB39" => (18, 258, 8),
            "M-0083_A-0319-31-587B" => (14, 134, 4),
            other => panic!("unexpected prod3 program_id {other} in this test's own list"),
        };
        assert_eq!(
            single_total, expected_single,
            "{program_id}: program-tree-only activation count, independently derived"
        );
        assert_eq!(
            full_total, expected_full,
            "{program_id}: fully-expanded activation count, independently derived"
        );
        assert_eq!(
            distinct_scopes, expected_scopes,
            "{program_id}: distinct ModuleScopes — the number of Module rows this \
             program's own default parameter values actually cause evaluate to walk \
             (not the raw stored-row count; see this test's own doc comment)"
        );
    }
}

/// AC#7: a module-free program's activation counts are unaffected by this
/// slice. `prod1` (`646704-04_ETS4_2012_47_DE_EN.knxprod`) has zero
/// `ModuleDef` and zero `Module` (§4.4 Q7), so `load_program_trees` loads no
/// module scopes at all and `evaluate` over it must behave identically to
/// `ProgramTrees::single` — which is also what slice 1's own `evaluate(&tree,
/// ...)` did before this slice existed.
#[test]
fn corpus_module_expansion_leaves_a_module_free_program_unchanged() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../OriginalData/ProductDatabases")
        });
    if !root.exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }

    let name = "646704-04_ETS4_2012_47_DE_EN.knxprod";
    let bytes = std::fs::read(root.join(name)).unwrap_or_else(|e| {
        panic!(
            "corpus fixture {name} unavailable: {e}; set KNXBENCH_PRODUCT_CORPUS to \
             OriginalData/ProductDatabases"
        )
    });
    let (_dir, conn) = db();
    knx_productdb::install_package(&conn, name, &bytes).unwrap();

    let program_ids: Vec<String> = conn
        .prepare("SELECT DISTINCT program_id FROM dynamic_node WHERE module_def_id = ''")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(program_ids.len(), 1, "{name}: one application program");
    let program_id = &program_ids[0];

    let module_def_ids: Vec<String> = conn
        .prepare("SELECT DISTINCT module_def_id FROM dynamic_node WHERE program_id = ?1 AND module_def_id != ''")
        .unwrap()
        .query_map([program_id], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(
        module_def_ids.is_empty(),
        "{name}: control group must be module-free, found {module_def_ids:?}"
    );

    let single_tree = knx_productdb::dynamic::load_tree(&conn, program_id, "").unwrap();
    let values =
        knx_productdb::dynamic::resolve_values(&conn, program_id, &HashMap::new()).unwrap();
    let single = evaluate(&ProgramTrees::single(single_tree), &values);
    let single_total = single.parameter_refs.len() + single.com_object_refs.len();

    let trees = load_program_trees(&conn, program_id).unwrap();
    let full = evaluate(&trees, &values);
    let full_total = full.parameter_refs.len() + full.com_object_refs.len();

    let no_branch_matched = |diags: &[ScopedDiagnostic]| {
        diags
            .iter()
            .filter(|sd| matches!(sd.diagnostic, Diagnostic::NoBranchMatched { .. }))
            .count()
    };

    eprintln!(
        "corpus {name} program {program_id}: single_total={single_total} full_total={full_total} \
         single_no_branch_matched={} full_no_branch_matched={}",
        no_branch_matched(&single.diagnostics),
        no_branch_matched(&full.diagnostics)
    );

    assert_eq!(
        single_total, 145,
        "{program_id}: program-tree-only activation count, independently derived"
    );
    assert_eq!(
        full_total, 145,
        "{program_id}: AC#7 — module expansion must not change a module-free program's count"
    );
    assert_eq!(
        single_total, full_total,
        "{program_id}: identical because there is nothing to expand"
    );
    assert_eq!(
        no_branch_matched(&single.diagnostics),
        24,
        "{program_id}: program-tree-only NoBranchMatched count, independently derived"
    );
    assert_eq!(
        no_branch_matched(&full.diagnostics),
        24,
        "{program_id}: AC#7 — diagnostics are unaffected too"
    );
}
