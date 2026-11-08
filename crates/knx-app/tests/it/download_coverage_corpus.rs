//! Offline download coverage across the private product corpus.
//!
//! `#[ignore]`d: it needs the private product corpus (`KNXBENCH_PRODUCT_CORPUS`
//! or the root checkout's `OriginalData/ProductDatabases`). It installs every
//! `.knxprod` there into a fresh database using the bounded LARGE profile
//! (ADR-0082, KL §151; explicit offline opt-in, no package exclusions) and asks
//! [`knx_app::download_support::coverage`] for every program, then pins the
//! counts. A change to the image builder or the plan translation that moves
//! any of them must re-pin here and say why in `docs/RESEARCH.md`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

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
    let mut inventory = Vec::new();
    for path in knx_testsupport::walk_corpus_files(&root) {
        if path.extension().and_then(|e| e.to_str()) != Some("knxprod") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        inventory.push((format!("{:x}", Sha256::digest(&bytes)), bytes.len()));
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        knx_productdb::install_package_with_limits(
            &conn,
            &name,
            &bytes,
            knx_productdb::PackageLimits::LARGE,
        )
        .unwrap_or_else(|_| panic!("package {} failed to install", packages + 1));
        packages += 1;
    }
    // Inventory is checked after coverage is printed, so drift still leaves
    // an aggregate measurement without private program IDs or diagnostics.

    let rows = coverage(&conn, None, &shipped_evidence().unwrap()).unwrap();
    let summary = CoverageSummary::of(&rows);
    // Printed before the pins, so a failing run still shows every count.
    eprintln!("coverage: {summary:?}");
    // Characterisation baseline of 8 November 2026 (RESEARCH §19.22,
    // owner decision in PRG-01): every `.knxprod` the private collection
    // then held, measured twice with identical results and a private input
    // manifest unchanged before and after. It replaces the 9 October pins
    // (105 packages, 467 programs, §19.20), whose input set could not be
    // reconstructed. It characterises the code; it is not an independent
    // oracle. A moved count must be explained in RESEARCH before re-pinning.
    assert_eq!(packages, 122, "the corpus changed; re-measure");
    assert_eq!(
        inventory_commitment(&inventory),
        "3df573f3af8dc37f81e6602b531402443e0c1ead6ba0472d8455109a4aa780a6",
        "package bytes or installation order changed; re-measure before re-pinning"
    );
    assert_eq!(summary.programs, 758);
    // The October code on this corpus planned 1 + 101. Since then: 13 Float
    // refusals disappear because inactive choose branches no longer select
    // their fields (not Float support); 2 conflicting-reference programs
    // likewise plan (ADR-0160). Four previously planning programs declare an ETS
    // download plug-in (KL-006).
    assert_eq!((summary.verified, summary.untested), (1, 112));
    let unsupported: BTreeMap<&str, usize> = summary
        .unsupported
        .iter()
        .map(|(category, count)| (category.code(), *count))
        .collect();
    assert_eq!(
        unsupported,
        BTreeMap::from([
            // Includes 135 programs whose code is unreadable (a
            // RelativeSegment's Data shorter than its Size): their mask is
            // named first, as for readable code.
            ("not-memory-mapped", 507),
            // 25 LittleEndian programs now build their image (KL-170,
            // ADR-0159) and stop at LdCtrlCompareProp; the 3 October ones
            // (LdCtrlWriteProp) are now vendor-download-plugin.
            ("unmodelled-step", 25),
            ("procedure-contents", 1),
            ("parameter-evaluation", 51),
            // 6 conflicting-reference programs left: 2 plan, 4 stop at a
            // non-default object priority (image-structure).
            ("parameter-value", 3),
            // 84 with October code on this corpus: -25 to unmodelled-step,
            // -13 inactive Float fields no longer selected,
            // +4 from parameter-value.
            ("image-structure", 50),
            ("product-data", 1),
            ("vendor-download-plugin", 7),
        ])
    );
    assert_eq!(summary.by_mask.get("MV-0701"), Some(&(103, 42)));
    assert_eq!(summary.by_mask.get("MV-0705"), Some(&(148, 71)));

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
                "a non-memory mask was refused for the wrong category"
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

/// Hash content identities and lengths in installation order, retaining duplicates.
/// The private manifest also names the files; no private names enter Git or output.
fn inventory_commitment(inventory: &[(String, usize)]) -> String {
    let mut hash = Sha256::new();
    for (sha256, bytes) in inventory {
        hash.update(format!("{sha256}\t{bytes}\n"));
    }
    format!("{:x}", hash.finalize())
}

#[test]
fn the_inventory_commitment_binds_content_size_multiplicity_and_order() {
    let original = vec![("a".into(), 1), ("b".into(), 2)];
    let expected = inventory_commitment(&original);
    assert_eq!(expected, inventory_commitment(&original));
    for changed in [
        vec![("c".into(), 1), ("b".into(), 2)],
        vec![("a".into(), 2), ("b".into(), 2)],
        vec![("a".into(), 1), ("b".into(), 2), ("a".into(), 1)],
        vec![("b".into(), 2), ("a".into(), 1)],
    ] {
        assert_ne!(expected, inventory_commitment(&changed));
    }
}
