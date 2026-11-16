//! knx-devconf integration tests in one binary: one link, one test pool (ADR-0144).

mod demo;
mod expressions;
mod module_load;

use knx_devconf::{LoadError, Module};
use serde_json::{json, Value};

pub fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

pub fn demo() -> Module {
    Module::load(&fixture("demo-panel.devconf.json")).expect("demo module loads")
}

/// The valid demo document after `edit`.
pub fn demo_doc(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut doc: Value = serde_json::from_slice(&fixture("demo-panel.valid.json")).unwrap();
    edit(&mut doc);
    serde_json::to_vec(&doc).unwrap()
}

/// A module with the given document members, rules and exports.
pub fn module(members: Value, rules: Value, exports: Value) -> Result<Module, LoadError> {
    let package = json!({
        "format": "knxbench-device-configuration-module",
        "format_version": 1,
        "id": "test.module",
        "version": "0.1.0",
        "title": "Test",
        "max_document_bytes": 1_048_576,
        "document": { "type": "object", "members": members },
        "rules": rules,
        "exports": exports,
    });
    Module::load(&serde_json::to_vec(&package).unwrap())
}

/// A module of one table `t` (key `id`, members `id` and `v`) with one rule per row.
pub fn row_rule(require: Value) -> Module {
    module(
        json!({ "t": { "type": "table", "key": "id", "min_rows": 0, "max_rows": 4096, "row": { "type": "object", "members": {
            "id": { "type": "int", "min": 0, "max": 100000 },
            "v": { "type": "int", "min": i64::MIN, "max": i64::MAX },
            "s": { "type": "enum", "values": ["a", "b"] } } } } }),
        json!([{ "id": "r", "table": "t", "require": require, "message": "violated" }]),
        json!([]),
    )
    .unwrap_or_else(|e| panic!("module loads: {e}"))
}

/// Rule violations for a document of rows `(id, v, s)`.
pub fn violations(module: &Module, rows: &[(i64, i64, &str)]) -> Vec<String> {
    let rows: Vec<Value> = rows
        .iter()
        .map(|(id, v, s)| json!({ "id": id, "v": v, "s": s }))
        .collect();
    let doc = serde_json::to_vec(&json!({ "t": rows })).unwrap();
    let validation = module.validate(&doc).expect("evaluates");
    validation.findings.iter().map(|f| f.path.clone()).collect()
}
