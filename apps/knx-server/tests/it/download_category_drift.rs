//! Pins that the web consumers name exactly the server's download refusal categories.

use knx_app::download_support::UnsupportedCategory;
use std::collections::BTreeSet;

const READINESS: &str = include_str!("../../../knx-web/src/downloadReadiness.ts");
const GUIDANCE: &str = include_str!("../../../knx-web/src/downloadSupportGuidance.ts");

/// Quoted or bare keys of the object literal that follows `marker`, up to its closing `};`.
fn keys(source: &str, marker: &str) -> BTreeSet<String> {
    let body = source.split_once(marker).expect("marker present").1;
    let body = body.split_once("\n};").expect("object end").0;
    let body = body.split_once('{').expect("object start").1;
    body.split(',')
        .filter_map(|entry| entry.split_once(':'))
        .map(|(key, _)| key.trim().trim_matches('"').to_owned())
        .filter(|key| !key.is_empty())
        .collect()
}

#[test]
fn web_reason_and_action_tables_match_the_server_categories_exactly() {
    let server: BTreeSet<String> = UnsupportedCategory::ALL
        .iter()
        .map(|category| category.code().to_owned())
        .collect();
    let reasons = keys(READINESS, "const CATEGORY_REASON");
    let actions = keys(GUIDANCE, "const ACTIONS");
    assert_eq!(
        reasons, server,
        "downloadReadiness.ts CATEGORY_REASON drifted"
    );
    assert_eq!(
        actions, server,
        "downloadSupportGuidance.ts ACTIONS drifted"
    );
}

#[test]
fn the_extractor_reads_the_real_tables_not_nothing() {
    assert!(keys(READINESS, "const CATEGORY_REASON").contains("customer-adjustable-parameters"));
    assert!(keys(GUIDANCE, "const ACTIONS").contains("modules"));
}
