//! Module packages that cannot be used are refused as a whole, naming the problem.

use crate::{fixture, module};
use knx_devconf::{Module, MAX_MODULE_BYTES};
use serde_json::{json, Value};

fn table() -> Value {
    json!({ "t": { "type": "table", "key": "id", "min_rows": 0, "max_rows": 8, "row": { "type": "object", "members": {
        "id": { "type": "int", "min": 0, "max": 9 },
        "s": { "type": "enum", "values": ["a", "b"] } } } },
        "fixed": { "type": "const", "value": { "x": 1 } } })
}

fn rule(require: Value) -> Value {
    json!([{ "id": "r", "table": "t", "require": require, "message": "m" }])
}

fn refused(members: Value, rules: Value, exports: Value) -> String {
    match module(members, rules, exports) {
        Ok(_) => panic!("module loaded"),
        Err(e) => e.0,
    }
}

#[test]
fn package_envelope_problems() {
    let mut package: Value = serde_json::from_slice(&fixture("demo-panel.devconf.json")).unwrap();
    let load = |p: &Value| Module::load(&serde_json::to_vec(p).unwrap()).unwrap_err().0;
    let mut p = package.clone();
    p["format_version"] = json!(2);
    assert!(load(&p).contains("unsupported version 2"), "{}", load(&p));
    let mut p = package.clone();
    p["format"] = json!("other");
    assert!(load(&p).contains("not a device configuration module"));
    let mut p = package.clone();
    p["extensions"] = json!({});
    assert!(load(&p).contains("unknown member \"extensions\""));
    let mut p = package.clone();
    p["version"] = json!("1.0");
    assert!(load(&p).contains("MAJOR.MINOR.PATCH"));
    package["title"] = json!("x".repeat(MAX_MODULE_BYTES));
    assert!(load(&package).contains("limit is 262144"));
}

#[test]
fn rule_and_type_problems_are_found_at_load() {
    let count =
        |name: &str, inner: Value| json!({ "count": { "table": "t", "as": name, "where": inner } });
    let deep = count("a", count("b", count("c", count("d", json!(true)))));
    let mut nested = json!(true);
    for _ in 0..40 {
        nested = json!({ "op": "not", "args": [nested] });
    }
    let cases = [
        (
            rule(json!({ "op": "eq", "args": [{ "field": "s" }, "c"] })),
            "is not a value of the compared enum",
        ),
        (
            rule(json!({ "op": "add", "args": [{ "field": "s" }, 1] })),
            "expected integer, found string",
        ),
        (
            rule(json!({ "op": "eq", "args": [{ "lookup": { "table": "u", "key": 1 } }, 1] })),
            "no table \"u\"",
        ),
        (
            rule(json!({ "op": "eq", "args": [{ "lookup": { "table": "t", "key": 1 } }, 1] })),
            "cannot compare row with integer",
        ),
        (
            rule(json!({ "op": "eq", "args": [{ "get": [1, "id"] }, 1] })),
            "expected a row, found integer",
        ),
        (
            rule(
                json!({ "op": "ge", "args": [{ "count": { "table": "t", "as": "q", "where": { "any": { "table": "t", "as": "q", "where": true } } } }, 0] }),
            ),
            "already bound row variable \"q\"",
        ),
        (
            rule(json!({ "op": "ge", "args": [deep, 0] })),
            "quantifier nesting exceeds 3",
        ),
        (
            rule(json!({ "op": "eq", "args": [{ "field": "fixed" }, 1] })),
            "has no member \"fixed\"",
        ),
        (
            json!([{ "id": "r", "require": { "op": "eq", "args": [{ "field": "fixed" }, 1] }, "message": "m" }]),
            "fixed array or object",
        ),
        (
            json!([{ "id": "r", "require": { "op": "eq", "args": [{ "depth": { "table": "t", "parent": "id", "root": 0 } }, 1] }, "message": "m" }]),
            "current table only",
        ),
        (rule(json!({ "field": "id" })), "must be boolean"),
        (
            rule(json!({ "op": "nand", "args": [true, true] })),
            "unknown operator",
        ),
        (rule(json!({ "frobnicate": 1 })), "unknown expression form"),
        (
            rule(json!({ "op": "not", "args": [true, false] })),
            "takes 1..1 arguments",
        ),
        (rule(nested), "nesting exceeds 32"),
        (
            json!([{ "id": "r", "require": true, "message": "m" }, { "id": "r", "require": true, "message": "m" }]),
            "repeated rule id",
        ),
        (
            json!([{ "id": "r", "require": true, "message": "m", "field": "nope" }]),
            "no member \"nope\"",
        ),
    ];
    for (rules, expected) in cases {
        let error = refused(table(), rules.clone(), json!([]));
        assert!(
            error.contains(expected),
            "{rules}: expected {expected:?}, got {error:?}"
        );
    }
}

#[test]
fn declaration_problems_are_found_at_load() {
    let row = |members: Value| json!({ "t": { "type": "table", "min_rows": 0, "max_rows": 1, "row": { "type": "object", "members": members } } });
    let cases = [
        (
            row(
                json!({ "o": { "type": "object", "members": { "x": { "type": "int", "min": 0, "max": 1 } } } }),
            ),
            "type \"object\" is not allowed here",
        ),
        (
            row(json!({ "r": { "type": "ref", "table": "t" } })),
            "has no key",
        ),
        (
            row(json!({ "r": { "type": "ref", "table": "u" } })),
            "no table \"u\"",
        ),
        (
            row(json!({ "x": { "type": "int", "min": 2, "max": 1 } })),
            "min exceeds max",
        ),
        (
            row(json!({ "x": { "type": "enum", "values": ["a", "a"] } })),
            "repeated",
        ),
        (
            row(
                json!({ "x": { "type": "text", "charset": "latin1", "min_bytes": 0, "max_bytes": 1 } }),
            ),
            "unsupported charset",
        ),
        (
            row(json!({ "x": { "type": "int", "min": 0, "max": 1, "unit": "s" } })),
            "unknown member \"unit\"",
        ),
        (
            row(json!({ "Bad": { "type": "int", "min": 0, "max": 1 } })),
            "invalid member name",
        ),
        (
            json!({ "t": { "type": "table", "key": "k", "min_rows": 0, "max_rows": 1, "row": { "type": "object", "members": { "x": { "type": "int", "min": 0, "max": 1 } } } } }),
            "no row member \"k\"",
        ),
    ];
    for (members, expected) in cases {
        let error = refused(members.clone(), json!([]), json!([]));
        assert!(
            error.contains(expected),
            "{members}: expected {expected:?}, got {error:?}"
        );
    }
}

#[test]
fn layout_problems_are_found_at_load() {
    let export = |layout: Value| json!([{ "id": "e", "title": "E", "extension": "bin", "max_bytes": 64, "layout": layout }]);
    let crc = json!({ "crc": "crc16-ccitt-false", "encoding": "u16le" });
    let cases = [
        (
            export(json!([{ "text": { "field": "id" }, "size": 4, "pad": "zero" }])),
            "a text expression is required",
        ),
        (
            export(json!([{ "u8": { "field": "s" } }])),
            "an integer expression is required",
        ),
        (export(json!([crc.clone(), crc])), "one crc"),
        (
            export(json!([{ "rows": "t", "layout": [{ "rows": "t", "layout": [] }] }])),
            "cannot be nested",
        ),
        (
            export(json!([{ "rows": "t", "layout": [{ "total_length": "u16le" }] }])),
            "one total_length",
        ),
        (export(json!([{ "ascii": "\u{e9}" }])), "printable ASCII"),
        (export(json!([{ "u64le": 1 }])), "unknown layout item"),
        (
            json!([{ "id": "e", "title": "E", "extension": "BIN", "max_bytes": 64, "layout": [] }]),
            "extension",
        ),
    ];
    let members = json!({ "id": { "type": "int", "min": 0, "max": 9 }, "s": { "type": "enum", "values": ["a"] },
        "t": { "type": "table", "min_rows": 0, "max_rows": 1, "row": { "type": "object", "members": { "x": { "type": "int", "min": 0, "max": 1 } } } } });
    for (exports, expected) in cases {
        let error = refused(members.clone(), json!([]), exports.clone());
        assert!(
            error.contains(expected),
            "{exports}: expected {expected:?}, got {error:?}"
        );
    }
}

#[test]
fn unsupported_format_versions_and_fixed_array_items_are_refused() {
    let package: Value = serde_json::from_slice(&fixture("demo-panel.devconf.json")).unwrap();
    for version in [0, -1, 2] {
        let mut p = package.clone();
        p["format_version"] = json!(version);
        let error = Module::load(&serde_json::to_vec(&p).unwrap())
            .unwrap_err()
            .0;
        assert!(
            error.contains(&format!("unsupported version {version}")),
            "{error}"
        );
    }
    let members = json!({ "a": { "type": "array", "items": { "type": "const", "value": 1 }, "min_len": 0, "max_len": 2 } });
    let error = refused(members, json!([]), json!([]));
    assert!(
        error.contains("type \"const\" is not allowed here"),
        "{error}"
    );
}

#[test]
fn a_type_error_names_the_type_not_a_debug_value() {
    let error = refused(table(), rule(json!({ "field": "s" })), json!([]));
    assert!(
        error.ends_with("a condition must be boolean, found string"),
        "{error}"
    );
}
