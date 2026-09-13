//! Shared fixture paths for integration tests. A `tests/support/mod.rs`
//! (not `tests/support.rs`) so Cargo does not treat it as its own test
//! binary — each file that needs it pulls it in with `mod support;`.
//!
//! Not every test file that imports this module uses every function in it;
//! `dead_code` is allowed here rather than per-item, since that is a
//! property of being a shared module, not a sign any one function is
//! actually unused.
#![allow(dead_code)]

use std::path::PathBuf;

use knx_etsproj::opaque::{ManufacturerFile, OpaqueEntry};

/// Reassembles the full entry list `export_knxproj` needs: every opaque
/// entry plus every manufacturer file converted back into an `OpaqueEntry`
/// at its own `source_path`, mirroring what `knx-app` does for real once
/// the product database exists (Task 14).
pub fn all_entries(opaque: &[OpaqueEntry], manufacturer: &[ManufacturerFile]) -> Vec<OpaqueEntry> {
    opaque
        .iter()
        .cloned()
        .chain(manufacturer.iter().map(|m| OpaqueEntry {
            source_path: m.source_path.clone(),
            xpath: String::new(),
            kind: m.kind,
            name: String::new(),
            bytes: m.bytes.clone(),
            sha256: m.sha256.clone(),
        }))
        .collect()
}

pub fn workspace_root() -> PathBuf {
    knx_testsupport::workspace_root()
}

pub fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

pub fn reference_ets6_path() -> PathBuf {
    knx_testsupport::reference_ets6_path()
}

/// The `xknxproject` reference dump used by the oracle comparison (Task 17).
pub fn oracle_dump_path() -> PathBuf {
    workspace_root().join("project_dump.json")
}
