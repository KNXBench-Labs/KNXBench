//! Owns the corpus fixture paths so no crate hard-codes the maintainer's export filenames.

use std::path::{Path, PathBuf};

/// The workspace root, resolved from *this crate's own* `CARGO_MANIFEST_DIR`
/// (`<root>/crates/knx-testsupport`) rather than the caller's. `env!` expands
/// where it is written, so this is always this crate's manifest directory no
/// matter who calls — which is the point: consumers live under both
/// `crates/<name>` and `apps/<name>`, and a version that walked up from the
/// caller's manifest would have to know which. Two parents up from
/// `crates/knx-testsupport` reaches the root; if this crate ever moves, this
/// is the one line that must move with it.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("knx-testsupport lives at <root>/crates/knx-testsupport")
        .to_path_buf()
}

fn overridable(env_var: &str, default_relative: &str) -> PathBuf {
    match std::env::var_os(env_var) {
        Some(value) => PathBuf::from(value),
        None => workspace_root().join(default_relative),
    }
}

/// The maintainer's real ETS4 export. Override with `KNXBENCH_REFERENCE_PROJECT`
/// to point at your own corpus — the golden tests assert counts measured
/// against the default fixture, so they will fail against a substitute; that
/// is expected, not a bug.
pub fn reference_ets4_path() -> PathBuf {
    overridable(
        "KNXBENCH_REFERENCE_PROJECT",
        "OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj",
    )
}

/// The same installation, re-exported from ETS 6.3.0. Override with
/// `KNXBENCH_REFERENCE_PROJECT_ETS6`; same golden-test caveat as
/// [`reference_ets4_path`].
pub fn reference_ets6_path() -> PathBuf {
    overridable(
        "KNXBENCH_REFERENCE_PROJECT_ETS6",
        "OriginalData/DemoProjects/Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj",
    )
}

/// A second, independent schema-21 installation (KNX Association demo
/// project). Override with `KNXBENCH_REFERENCE_PROJECT_SCHEMA21`; same
/// golden-test caveat as [`reference_ets4_path`].
pub fn reference_kv_schema21_path() -> PathBuf {
    overridable(
        "KNXBENCH_REFERENCE_PROJECT_SCHEMA21",
        "OriginalData/DemoProjects/KV v2.5 - demo.knxproj",
    )
}

/// True when the gitignored `OriginalData/` fixture corpus is present
/// locally. It holds the maintainer's own real KNX installation and
/// manufacturer files — never committed, so CI (and any contributor
/// without a copy) has none of it. Every test that needs the corpus must
/// check this first and skip, not panic, or CI is permanently red.
pub fn corpus_available() -> bool {
    reference_ets4_path().exists()
}
