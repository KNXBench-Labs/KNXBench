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
    evaluate, ControlKind, Diagnostic, DynamicNode, DynamicTree, Op, Test,
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
        let activation = evaluate(&tree, &values(&[("P", observed)]));
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
                vec![Diagnostic::NoBranchMatched {
                    choose_node: 0,
                    param_ref: Some("P".to_string()),
                    observed_value: observed.to_string(),
                }],
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
    let activation = evaluate(&tree, &values(&[("P", "2")]));
    assert_eq!(activation.parameter_refs, vec!["PRR".to_string()]);
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
    let activation = evaluate(&tree, &values(&[("P", "999")]));
    assert_eq!(activation.parameter_refs, vec!["FALLBACK".to_string()]);
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
    let activation = evaluate(&tree, &values(&[("P", "999")]));
    assert!(activation.parameter_refs.is_empty());
    assert!(activation.com_object_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![Diagnostic::NoBranchMatched {
            choose_node: 0,
            param_ref: Some("P".to_string()),
            observed_value: "999".to_string(),
        }]
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
    let activation = evaluate(&tree, &values(&[]));
    assert!(activation.parameter_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![Diagnostic::MissingValue {
            choose_node: 0,
            param_ref: Some("P".to_string()),
        }]
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
    let activation = evaluate(&tree, &values(&[("P", "not-a-number")]));
    assert!(activation.parameter_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![Diagnostic::NonNumericValue {
            choose_node: 0,
            param_ref: Some("P".to_string()),
            raw: "not-a-number".to_string(),
        }]
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
    let activation = evaluate(&tree, &values(&[("P", "1")]));
    assert_eq!(activation.parameter_refs, vec!["MATCH".to_string()]);
    assert_eq!(
        activation.diagnostics,
        vec![Diagnostic::UnparsableTest {
            when_node: 1,
            raw: "garbage".to_string(),
        }]
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
    let activation = evaluate(&tree, &values(&[]));
    assert_eq!(activation.parameter_refs, vec!["ALWAYS".to_string()]);
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
    let activation = evaluate(&tree, &values(&[]));
    assert!(activation.parameter_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![Diagnostic::UnexpectedTypeNoneShape { choose_node: 0 }]
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
    let activation = evaluate(&tree, &values(&[]));
    assert!(activation.parameter_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![Diagnostic::UnrecognizedNode {
            node_id: 0,
            kind: "Weird".to_string(),
        }]
    );
}

/// D10: a `Module` node is recognized but not expanded.
#[test]
fn a_module_node_is_recognized_but_not_expanded() {
    let tree = DynamicTree::from_nodes(vec![DynamicNode {
        ref_id: Some("MD-1".into()),
        ..nd(0, None, "Module")
    }]);
    let activation = evaluate(&tree, &values(&[]));
    assert!(activation.parameter_refs.is_empty());
    assert!(activation.com_object_refs.is_empty());
    assert_eq!(
        activation.diagnostics,
        vec![Diagnostic::ModuleNotExpanded {
            node_id: 0,
            ref_id: Some("MD-1".to_string()),
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
    let activation = evaluate(&tree, &values(&[]));
    assert_eq!(activation.parameter_refs, vec!["SHARED".to_string()]);
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
    conn.execute_batch("DROP TABLE dynamic_node; PRAGMA user_version = 2;")
        .unwrap();
    drop(conn);

    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3
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

    conn.execute_batch("DROP TABLE dynamic_node; PRAGMA user_version = 2;")
        .unwrap();
    drop(conn);

    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3,
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
            let activation = evaluate(&tree, &values);
            for d in activation.diagnostics {
                match d {
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
    }
}
