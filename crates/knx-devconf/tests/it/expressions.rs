//! Expression semantics through the public API: null handling, overflow, logic and quantifiers.

use crate::{module, row_rule, violations};
use serde_json::json;

const MAX: i64 = i64::MAX;

fn op(name: &str, args: serde_json::Value) -> serde_json::Value {
    json!({ "op": name, "args": args })
}

fn missing() -> serde_json::Value {
    json!({ "get": [{ "lookup": { "table": "t", "key": 999_999 } }, "v"] })
}

#[test]
fn a_null_result_is_a_violation_and_is_null_detects_it() {
    let m = row_rule(op("eq", json!([missing(), 1])));
    assert_eq!(violations(&m, &[(1, 1, "a")]), ["/t/0"]);
    let m = row_rule(op("is_null", json!([missing()])));
    assert!(violations(&m, &[(1, 1, "a")]).is_empty());
    let m = row_rule(op("not", json!([op("eq", json!([missing(), 1]))])));
    assert_eq!(
        violations(&m, &[(1, 1, "a")]),
        ["/t/0"],
        "not(null) stays null"
    );
}

#[test]
fn and_or_follow_three_valued_logic() {
    let unknown = op("eq", json!([missing(), 1]));
    for (expr, violated) in [
        (
            op("not", json!([op("and", json!([false, unknown.clone()]))])),
            false,
        ),
        (op("or", json!([true, unknown.clone()])), false),
        (op("and", json!([true, unknown.clone()])), true),
        (op("or", json!([false, unknown.clone()])), true),
        (op("and", json!([true, true, true])), false),
    ] {
        let m = row_rule(expr.clone());
        assert_eq!(
            !violations(&m, &[(1, 1, "a")]).is_empty(),
            violated,
            "{expr}"
        );
    }
}

#[test]
fn if_takes_the_else_branch_on_null() {
    let m = row_rule(op(
        "if",
        json!([op("eq", json!([missing(), 1])), false, true]),
    ));
    assert!(violations(&m, &[(1, 1, "a")]).is_empty());
}

#[test]
fn overflow_and_division_by_zero_are_null_and_mod_is_truncated() {
    let m = row_rule(op(
        "is_null",
        json!([op("add", json!([{ "field": "v" }, 1]))]),
    ));
    assert_eq!(violations(&m, &[(1, MAX, "a"), (2, 1, "a")]), ["/t/1"]);
    let m = row_rule(op("is_null", json!([op("neg", json!([{ "field": "v" }]))])));
    assert_eq!(violations(&m, &[(1, i64::MIN, "a"), (2, 5, "a")]), ["/t/1"]);
    let m = row_rule(op(
        "is_null",
        json!([op("mod", json!([7, { "field": "v" }]))]),
    ));
    assert_eq!(violations(&m, &[(1, 0, "a"), (2, 3, "a")]), ["/t/1"]);
    let m = row_rule(op("eq", json!([op("mod", json!([-7, 2])), -1])));
    assert!(violations(&m, &[(1, 0, "a")]).is_empty());
    let m = row_rule(op(
        "eq",
        json!([op("xor", json!([{ "field": "id" }, 1])), { "field": "v" }]),
    ));
    assert_eq!(violations(&m, &[(4, 5, "a"), (5, 5, "a")]), ["/t/1"]);
}

#[test]
fn in_ordinal_and_string_order() {
    let m = row_rule(op("in", json!([{ "field": "s" }, "a"])));
    assert_eq!(violations(&m, &[(1, 0, "a"), (2, 0, "b")]), ["/t/1"]);
    let m = row_rule(op("in", json!([{ "field": "v" }, 1, missing()])));
    assert_eq!(
        violations(&m, &[(1, 1, "a"), (2, 2, "a")]),
        ["/t/1"],
        "no match beside null is null"
    );
    let m = row_rule(op("eq", json!([{ "ordinal": { "field": "s" } }, 1])));
    assert_eq!(violations(&m, &[(1, 0, "a"), (2, 0, "b")]), ["/t/0"]);
    let m = row_rule(op("lt", json!([{ "field": "s" }, "b"])));
    assert_eq!(violations(&m, &[(1, 0, "a"), (2, 0, "b")]), ["/t/1"]);
}

#[test]
fn index_row_and_quantifiers() {
    let m = row_rule(op(
        "eq",
        json!([{ "index": null }, op("sub", json!([{ "field": "id" }, 1]))]),
    ));
    assert_eq!(violations(&m, &[(1, 0, "a"), (3, 0, "a")]), ["/t/1"]);
    let m = row_rule(op(
        "is_null",
        json!([{ "row": { "table": "t", "index": { "field": "v" } } }]),
    ));
    assert_eq!(
        violations(&m, &[(1, 1, "a"), (2, -1, "a")]),
        ["/t/0"],
        "only index 1 exists"
    );
    let count = json!({ "count": { "table": "t", "as": "q", "where": op("eq", json!([{ "field": "v", "of": "q" }, { "field": "v" }])) } });
    let m = row_rule(op("eq", json!([count, 1])));
    assert_eq!(
        violations(&m, &[(1, 7, "a"), (2, 7, "a"), (3, 8, "a")]),
        ["/t/0", "/t/1"]
    );
    let nested = json!({ "all": { "table": "t", "as": "p", "where": { "any": { "table": "t", "as": "q",
        "where": op("eq", json!([{ "field": "v", "of": "q" }, { "field": "id", "of": "p" }])) } } } });
    let m = row_rule(nested);
    assert!(violations(&m, &[(1, 2, "a"), (2, 1, "a")]).is_empty());
    assert_eq!(violations(&m, &[(1, 2, "a"), (2, 3, "a")]).len(), 2);
}

#[test]
fn quantifiers_over_an_empty_table() {
    let members = json!({ "t": { "type": "table", "key": "id", "min_rows": 0, "max_rows": 4, "row": { "type": "object", "members": {
        "id": { "type": "int", "min": 0, "max": 9 } } } } });
    let rule = |require| json!([{ "id": "r", "require": require, "message": "m" }]);
    for (require, valid) in [
        (
            json!({ "all": { "table": "t", "as": "q", "where": false } }),
            true,
        ),
        (
            json!({ "any": { "table": "t", "as": "q", "where": true } }),
            false,
        ),
        (
            op(
                "eq",
                json!([{ "count": { "table": "t", "as": "q", "where": true } }, 0]),
            ),
            true,
        ),
        (op("eq", json!([{ "rows": "t" }, 0])), true),
    ] {
        let m = module(members.clone(), rule(require.clone()), json!([])).unwrap();
        assert_eq!(
            m.validate(br#"{"t":[]}"#).unwrap().is_valid(),
            valid,
            "{require}"
        );
    }
}

#[test]
fn in_follows_three_valued_logic_and_a_null_predicate_counts_as_false() {
    // A match decides even beside null candidates; no match beside one is unknown.
    let m = row_rule(op("in", json!([{ "field": "v" }, missing(), 1])));
    assert_eq!(violations(&m, &[(1, 1, "a"), (2, 2, "a")]), ["/t/1"]);
    let m = row_rule(op(
        "is_null",
        json!([op("in", json!([{ "field": "v" }, missing(), 1]))]),
    ));
    assert_eq!(violations(&m, &[(1, 1, "a"), (2, 2, "a")]), ["/t/0"]);
    let m = row_rule(op("is_null", json!([op("in", json!([missing(), 1]))])));
    assert!(
        violations(&m, &[(1, 1, "a")]).is_empty(),
        "null needle is null"
    );
    let unknown = op("eq", json!([missing(), 1]));
    let count = json!({ "count": { "table": "t", "as": "q", "where": unknown } });
    let m = row_rule(op("eq", json!([count, 0])));
    assert!(violations(&m, &[(1, 1, "a"), (2, 2, "a")]).is_empty());
    let any = json!({ "any": { "table": "t", "as": "q", "where": unknown } });
    let m = row_rule(op("not", json!([any])));
    assert!(violations(&m, &[(1, 1, "a")]).is_empty());
}
