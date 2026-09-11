//! Storage tests for the `ApplicationProgram/Dynamic` tree (T18 Task 1,
//! design D2-D4). These assert the tree lands losslessly, one row per
//! element, in document order — nothing here evaluates `@test`.

use rusqlite::Connection;

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
