//! Tests for `resolve_scoped_values` (WP-37): decomposition of module-qualified
//! write ids into `(Module/@Id, ParameterRef id)` baseline keys.

use std::collections::BTreeMap;

use super::resolve_scoped_values;

fn supplied(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn module_qualified_keys_decompose_into_module_ref_pairs() {
    let scoped = resolve_scoped_values(&supplied(&[
        ("P-1_M-2_MI-3_R-7", "on"),
        ("P-1_M-2_MI-4_R-7", "off"),
        ("P-1_R-1", "unscoped stays out"),
    ]));
    assert_eq!(
        scoped,
        BTreeMap::from([
            (("P-1_M-2_MI-3".to_string(), "R-7".to_string()), "on".to_string()),
            (("P-1_M-2_MI-4".to_string(), "R-7".to_string()), "off".to_string()),
        ])
    );
}

#[test]
fn rightmost_marker_wins_like_the_write_side_decomposer() {
    let scoped = resolve_scoped_values(&supplied(&[("P_M-1_MI-1_P_M-2_MI-2_TAIL", "v")]));
    assert_eq!(
        scoped.keys().next().unwrap().clone(),
        ("P_M-1_MI-1_P_M-2_MI-2".to_string(), "TAIL".to_string())
    );
}

#[test]
fn malformed_or_plain_keys_are_skipped_never_guessed() {
    let scoped = resolve_scoped_values(&supplied(&[
        ("P-1_M-2_MI-_R-1", "no digits"),
        ("P-1_M-2_R-1", "no MI marker"),
        ("P-1_R-1", "plain"),
        ("", "empty"),
    ]));
    assert!(scoped.is_empty());
}

