//! Corpus regression for `query::parameter_views`/`parameter_ref_ids`
//! (T18 Task 1, design D22), over the real `prod3` MDT archive already
//! pinned by `tests/dynamic_tree.rs`'s own corpus test.
//!
//! RESEARCH.md §4.4 Q3 cites 208 `ParameterRef` ids for one `ModuleDef`
//! (`M-0083_A-0317-31-7DC6_MD-1`) of this program's four. That number
//! cannot be asserted here: `parameter_ref` carries no `module_def_id`
//! column (`crates/knx-productdb/src/migration.rs`), so it is keyed only
//! `(program_id, id)` — `parameter_views(conn, program_id)` necessarily
//! returns the *whole* program's declared set, the top-level `Static` plus
//! all four `ModuleDef`s' together, undifferentiated. Measured directly
//! against the loaded database with `SELECT COUNT(*) FROM parameter_ref
//! WHERE program_id = ?1`: 543, not 208 — the AP-level total this test
//! asserts.

use std::path::PathBuf;

fn corpus_root() -> PathBuf {
    std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData/ProductDatabases")
        })
}

/// AP-level `parameter_ref` count for `prod3`'s
/// `M-0083_A-0317-31-7DC6` — derived, not assumed, from
/// `SELECT COUNT(*) FROM parameter_ref WHERE program_id = ?1` against the
/// freshly loaded database (see the module doc comment). Not the 208
/// RESEARCH.md §4.4 Q3 cites for the one `ModuleDef` `MD-1` alone.
const PROD3_PROGRAM_ID: &str = "M-0083_A-0317-31-7DC6";
const PROD3_AP_LEVEL_PARAMETER_REF_COUNT: i64 = 543;

#[test]
fn parameter_views_and_parameter_ref_ids_match_the_ap_level_count_on_prod3() {
    let root = corpus_root();
    if !root.exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let name = "MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod";
    let bytes = std::fs::read(root.join(name)).unwrap_or_else(|e| {
        panic!("corpus fixture {name} unavailable: {e}; set KNXBENCH_PRODUCT_CORPUS to OriginalData/ProductDatabases")
    });
    knx_productdb::install_package(&conn, name, &bytes).unwrap();

    // The query and result this report's step 5 records: the raw table
    // count, independent of `parameter_views`'s own joins, is the ground
    // truth both assertions below are checked against.
    let raw_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM parameter_ref WHERE program_id = ?1",
            [PROD3_PROGRAM_ID],
            |r| r.get(0),
        )
        .unwrap();
    eprintln!(
        "corpus {name}: SELECT COUNT(*) FROM parameter_ref WHERE program_id = '{PROD3_PROGRAM_ID}' = {raw_count}"
    );
    assert_eq!(
        raw_count, PROD3_AP_LEVEL_PARAMETER_REF_COUNT,
        "AP-level parameter_ref count for {PROD3_PROGRAM_ID} (RESEARCH.md §4.4 Q3's 208 is one ModuleDef's count, not this)"
    );

    let views = knx_productdb::query::parameter_views(&conn, PROD3_PROGRAM_ID, None).unwrap();
    eprintln!("corpus {name}: parameter_views(..).len() = {}", views.len());
    assert_eq!(
        views.len() as i64,
        PROD3_AP_LEVEL_PARAMETER_REF_COUNT,
        "parameter_views must join every declared parameter_ref, none dropped"
    );

    let ref_ids = knx_productdb::query::parameter_ref_ids(&conn, PROD3_PROGRAM_ID).unwrap();
    eprintln!(
        "corpus {name}: parameter_ref_ids(..).len() = {}",
        ref_ids.len()
    );
    assert_eq!(ref_ids.len() as i64, PROD3_AP_LEVEL_PARAMETER_REF_COUNT);
    for v in &views {
        assert!(
            ref_ids.contains(&v.id),
            "every parameter_views row's id must also be in parameter_ref_ids: {}",
            v.id
        );
    }
}
