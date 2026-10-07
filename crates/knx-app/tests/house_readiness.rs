//! Download readiness of the maintainer's house, device by device.
//!
//! RESEARCH §19.12 measured it by hand, running `knx device download` (plan
//! only) for every address of "Unser Zuhause ets 6.3.0". This test pins that
//! table through `project_readiness`, so the table stays true as the image
//! builder and the load-procedure translation change. A device that starts
//! to plan, or stops, fails here by address and has to be re-documented.
//!
//! `#[ignore]`d: it needs the gitignored `OriginalData/` corpus.

use std::collections::BTreeMap;

use knx_app::download_support::{SupportLevel, UnsupportedCategory};
use knx_app::project_readiness::{project_readiness, DeviceReadiness, ReadinessSummary};

/// The expected code, and for a refusal the category and a phrase of the
/// refusal, per address range (RESEARCH §19.12's table).
fn expected(address: &str) -> (&'static str, Option<(UnsupportedCategory, &'static str)>) {
    let device: u16 = address.rsplit('.').next().unwrap().parse().unwrap();
    match device {
        22 | 23 => (
            "unsupported",
            Some((UnsupportedCategory::NotMemoryMapped, "")),
        ),
        220 => ("excluded", None),
        // 1-9: the machine-5 task segment is AbsCObjSeg, not sent
        // (Cookbook 02_03_01 §2.3). 11-13 and 24/250/253 rest on
        // inferences (ADR-0086, RESEARCH §19.12), checked below.
        1..=21 | 24..=32 | 250 | 253 => ("untested", None),
        _ => panic!("{address} is not in RESEARCH §19.12"),
    }
}

/// The inferences the house's devices rest on.
const MACHINE_5: &str = "machine-5-after-restart: LdCtrlLoad LsmIdx=5 after LdCtrlRestart is not \
     sent: the restart closed the connection and a BIM M112 has no fifth load state machine \
     (RESEARCH §19.12)";
const UNION_13: &str = "union-later-member: M-0083_A-0019-13-B655_UP-33_R-33 is not written: \
     M-0083_A-0019-13-B655_UP-1227_R-1227, later in the parameter tree, holds the union at \
     M-0083_A-0019-13-B655_AS-4400 offset 1810 bit 0 (RESEARCH §19.12)";

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_house_plans_as_research_19_12_says() {
    assert!(
        knx_testsupport::corpus_available(),
        "OriginalData/ corpus not present (gitignored, local-only)"
    );
    let dir = tempfile::tempdir().unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let project = knx_app::import_ets_project_with(
        &knx_testsupport::reference_ets6_path(),
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap()
    .project;

    // No shipped evidence: none of the house's programs is verified, and
    // the test must not change when the evidence file grows.
    let rows = project_readiness(&products, &project, &BTreeMap::new());
    assert_eq!(rows.len(), 35, "the house has 35 devices");
    let mut wrong = Vec::new();
    for row in &rows {
        let address = row
            .address
            .expect("every house device has an address")
            .to_string();
        let (code, refusal) = expected(&address);
        let found = match &row.readiness {
            DeviceReadiness::Graded(SupportLevel::Unsupported { category, detail }) => {
                format!("unsupported {category}: {detail}")
            }
            other => other.code().to_string(),
        };
        let fits = row.readiness.code() == code
            && refusal.is_none_or(|(category, phrase)| {
                matches!(
                    &row.readiness,
                    DeviceReadiness::Graded(SupportLevel::Unsupported {
                        category: found,
                        detail,
                    }) if *found == category && detail.contains(phrase)
                )
            });
        let inferences: Vec<String> = match &row.readiness {
            DeviceReadiness::Graded(SupportLevel::Untested { inferences, .. }) => {
                inferences.iter().map(ToString::to_string).collect()
            }
            _ => Vec::new(),
        };
        let expected_inferences: Vec<String> = match address.as_str() {
            "1.1.11" | "1.1.12" | "1.1.13" => vec![UNION_13.to_string()],
            "1.1.24" | "1.1.250" | "1.1.253" => vec![MACHINE_5.to_string()],
            _ => Vec::new(),
        };
        if !fits || inferences != expected_inferences {
            wrong.push(format!("{address} {}: {found} {inferences:?}", row.name));
        }
    }
    assert!(
        wrong.is_empty(),
        "not as RESEARCH §19.12 says:\n{}",
        wrong.join("\n")
    );
    let summary = ReadinessSummary::of(&rows);
    assert_eq!(summary.by_code.get("untested"), Some(&32));
    assert_eq!(summary.by_code.get("unsupported"), Some(&2));
    assert_eq!(summary.by_code.get("excluded"), Some(&1));
}
