//! Offline download coverage across the private product corpus.
//!
//! `#[ignore]`d: it needs the private product corpus (`KNXBENCH_PRODUCT_CORPUS`
//! or the root checkout's `OriginalData/ProductDatabases`). It installs every
//! `.knxprod` there into a fresh database and asks
//! [`knx_app::download_support::coverage`] for every program, then pins the
//! counts. A change to the image builder or the plan translation that moves
//! any of them must re-pin here and say why in `docs/RESEARCH.md`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use knx_app::download_support::{
    coverage, shipped_evidence, CoverageSummary, SupportLevel, UnsupportedCategory,
};

fn corpus_root() -> PathBuf {
    std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| knx_testsupport::workspace_root().join("OriginalData/ProductDatabases"))
}

#[test]
#[ignore = "requires the private product corpus; run with --ignored"]
fn the_corpus_coverage_is_pinned() {
    let root = corpus_root();
    assert!(
        root.is_dir(),
        "product corpus not present at {} (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it",
        root.display()
    );
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    let mut packages = 0;
    for path in knx_testsupport::walk_corpus_files(&root) {
        if path.extension().and_then(|e| e.to_str()) != Some("knxprod") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        knx_productdb::install_package(&conn, &name, &bytes)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        packages += 1;
    }
    assert_eq!(packages, 103, "the corpus changed; re-measure");

    let rows = coverage(&conn, None, &shipped_evidence().unwrap()).unwrap();
    let summary = CoverageSummary::of(&rows);
    // Printed before the pins, so a failing run still shows every count.
    eprintln!("coverage: {summary:?}");
    for row in &rows {
        if let SupportLevel::Unsupported { category, detail } = &row.level {
            eprintln!("refused {} {}: {detail}", category.code(), row.program_id);
        }
    }
    assert_eq!(summary.programs, 246);
    // Signed values at or above zero and text parameters are written since
    // RESEARCH §19.9: 15 more programs plan (54 -> 69). A Rename /
    // ParameterBlockRename leaf no longer holds up the image (§19.10): 4
    // more plan (69 -> 73). A field across an octet boundary is written
    // (§19.11): the other 4 of those programs plan too (73 -> 77).
    // LdCtrlTaskCtrl1 and the machine-5 steps translate (ADR-0086,
    // RESEARCH §19.12): the 2 unmodelled-step programs plan (77 -> 79).
    // An enabled ReadOnInitFlag is disclosed instead of refused (RESEARCH
    // §19.18, Resources NOTE 85): the 10 programs it stopped plan (79 -> 89).
    assert_eq!((summary.verified, summary.untested), (1, 89));
    let unsupported: BTreeMap<&str, usize> = summary
        .unsupported
        .iter()
        .map(|(category, count)| (category.code(), *count))
        .collect();
    assert_eq!(
        unsupported,
        BTreeMap::from([
            // The mask is asked before the procedure style: all 41
            // MergedProcedure/DefaultProcedure programs have a non-memory
            // mask, so none is counted under procedure-style any more.
            ("not-memory-mapped", 65),
            // 8 programs were held up only by Rename leaves; all 8 now
            // plan.
            ("parameter-evaluation", 51),
            // A float default ("5.0E+002") used to fail as "not an
            // unsigned number" here; it is now refused for its type, under
            // image-structure. 6 conflicting refs, 3 enumeration values
            // remain.
            ("parameter-value", 9),
            // 19 TypeFloat, 9 placed by Property, 3 Priority High.
            ("image-structure", 31),
        ])
    );
    assert_eq!(summary.by_mask.get("MV-0701"), Some(&(40, 37)));
    assert_eq!(summary.by_mask.get("MV-0705"), Some(&(141, 53)));

    // Every mask outside 070nh is refused for its mask, and none of them
    // for a reason that sounds like "almost".
    for row in &rows {
        let mask = row.mask_version.as_deref().unwrap_or("");
        if !matches!(mask, "MV-0701" | "MV-0705") {
            assert!(
                matches!(
                    row.level,
                    SupportLevel::Unsupported {
                        category: UnsupportedCategory::NotMemoryMapped,
                        ..
                    }
                ),
                "{} ({mask}): {}",
                row.program_id,
                row.level
            );
        }
    }

    // The one verified program is the push button, with the plan the live
    // runs used.
    let verified: Vec<_> = rows
        .iter()
        .filter(|row| row.level.code() == "verified")
        .collect();
    assert_eq!(verified.len(), 1);
    assert_eq!(verified[0].program_id, "M-0083_A-0027-15-0BAC");
    assert!(matches!(
        verified[0].level,
        SupportLevel::Verified {
            steps: 25,
            octets: 1416,
            ..
        }
    ));
}
