# Session 1: Architecture — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the approved architecture into a buildable Cargo workspace with mechanically enforced layering and license gates, plus the architecture documentation set and ADRs the project will be maintained against.

**Architecture:** A single Cargo workspace splits the system into `knx-core` (pure domain, no IO), service and infrastructure crates around it, and thin application entry points. Two rules that would otherwise erode silently — `knx-core` staying IO-free, and no GPL crate entering the runtime graph — are implemented as tests in an `xtask` binary and in `cargo-deny`, run in CI.

**Tech Stack:** Rust (stable, pinned via `rust-toolchain.toml`), `cargo-metadata`, `cargo-deny`, GitHub Actions. No Tauri, no Node, no domain logic in this session.

**Spec:** [docs/superpowers/specs/2026-09-02-knx-architecture-design.md](../specs/2026-09-02-knx-architecture-design.md) — read it alongside this plan.

---

## Scope note

Spec §3 lists `apps/knx-desktop` (Tauri shell + React UI). It is **not** created in this session. Pulling in Tauri and a Node toolchain before the UI session adds a large dependency surface with nothing to run against, and the license gate is more useful when it guards a graph we actually build on. `docs/ROADMAP.md` records that `knx-desktop` is scaffolded in Session 5.

`knx-core` receives exactly two real types in this session — `Layer` and `Resolved<T>` (spec §5.2). They are the architectural commitment the whole model depends on, and they give the layering gate something real to check. All other crates are empty apart from a crate-level doc comment stating their responsibility. The rest of the domain model is Session 2.

## Global Constraints

Copied verbatim from the spec. Every task inherits these.

- `knx-core` must not reach `serde_json`, `quick-xml`, `rusqlite` or `tokio` in its dependency graph (spec §3.1 rule 1).
- No runtime crate may depend on a GPL-licensed crate (spec §3.1 rule 2, RESEARCH R6). `xknxproject` stays in `.venv`, invoked only by test scripts, never by the Rust build.
- The UI communicates only through Tauri commands into `knx-app`; it has no path to `knx-store` or `knx-etsproj` (spec §3.1 rule 3). Not testable this session — no UI exists — but stated in `ARCHITECTURE.md`.
- Documentation and code are written in English.
- User-facing wording is **"KNX-compatible"**. Never "KNX certified", never "full ETS compatibility".
- Dependency direction: `apps → knx-app → {knx-core, knx-store, knx-etsproj, knx-productdb, knx-net, knx-secure}`, and every infrastructure crate → `knx-core`. Never upward.
- Git commits: focused, concise, no `Co-Authored-By: Claude` trailer (project `CLAUDE.md`).

---

### Task 1: Rust toolchain and workspace skeleton

Rust is not installed on this machine (`cargo: command not found`). This task installs it and lays out the workspace.

**Files:**
- Create: `rust-toolchain.toml`
- Create: `Cargo.toml` (workspace root)
- Create: `crates/knx-core/Cargo.toml`, `crates/knx-core/src/lib.rs`
- Create: `crates/knx-app/Cargo.toml`, `crates/knx-app/src/lib.rs`
- Create: `crates/knx-store/Cargo.toml`, `crates/knx-store/src/lib.rs`
- Create: `crates/knx-etsproj/Cargo.toml`, `crates/knx-etsproj/src/lib.rs`
- Create: `crates/knx-productdb/Cargo.toml`, `crates/knx-productdb/src/lib.rs`
- Create: `crates/knx-net/Cargo.toml`, `crates/knx-net/src/lib.rs`
- Create: `crates/knx-secure/Cargo.toml`, `crates/knx-secure/src/lib.rs`
- Create: `apps/knx-cli/Cargo.toml`, `apps/knx-cli/src/main.rs`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: nothing.
- Produces: workspace members named `knx-core`, `knx-app`, `knx-store`, `knx-etsproj`, `knx-productdb`, `knx-net`, `knx-secure`, `knx-cli`. `knx_core::{Layer, Resolved}` as defined in Step 6. Tasks 2–4 depend on these exact package names.

- [ ] **Step 1: Install the Rust toolchain**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default
source "$HOME/.cargo/env"
rustc --version
cargo --version
```

Expected: both print a version. `--profile default` includes `rustfmt` and `clippy`, which Task 4 needs.

- [ ] **Step 2: Pin the toolchain**

Read the installed version and write it into the pin file, so CI and this machine agree:

```bash
rustc --version   # e.g. "rustc 1.8x.y (abcdefg 2026-xx-xx)"
```

Create `rust-toolchain.toml` using the version number just printed in place of `1.8x.y`:

```toml
[toolchain]
channel = "1.8x.y"
components = ["rustfmt", "clippy"]
profile = "default"
```

- [ ] **Step 3: Create the workspace root**

`Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/knx-core",
    "crates/knx-app",
    "crates/knx-store",
    "crates/knx-etsproj",
    "crates/knx-productdb",
    "crates/knx-net",
    "crates/knx-secure",
    "apps/knx-cli",
]

[workspace.package]
edition = "2021"
license = "AGPL-3.0-or-later"
repository = "https://github.com/KNXBench-Labs/KNX"
rust-version = "1.8x.y"

[workspace.dependencies]
knx-core = { path = "crates/knx-core" }
knx-app = { path = "crates/knx-app" }
knx-store = { path = "crates/knx-store" }
knx-etsproj = { path = "crates/knx-etsproj" }
knx-productdb = { path = "crates/knx-productdb" }
knx-net = { path = "crates/knx-net" }
knx-secure = { path = "crates/knx-secure" }
```

Note on `license`: this is the placeholder the project ships with until the licence is decided. It is recorded as an open question in `docs/KNOWN_LIMITATIONS.md` in Task 10. Whatever it becomes, it must be compatible with the constraint that no GPL crate enters the runtime graph — that constraint is about *incoming* dependencies, not our own licence.

`xtask` is deliberately absent from `members`: it does not exist yet, and a workspace listing a missing member does not build. Task 2 Step 1 adds it.

- [ ] **Step 4: Create the six empty infrastructure and service crates**

For each of `knx-app`, `knx-store`, `knx-etsproj`, `knx-productdb`, `knx-net`, `knx-secure`, create `crates/<name>/Cargo.toml`:

```toml
[package]
name = "knx-app"
version = "0.0.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
publish = false

[dependencies]
knx-core.workspace = true
```

`knx-secure` is the exception: it has **no** `[dependencies]` section at all. It must not depend on `knx-core`, because key material must never reach the project model (spec §10).

And `crates/<name>/src/lib.rs` with only a crate-level doc comment stating the crate's responsibility, taken from spec §3. For example, `crates/knx-app/src/lib.rs`:

```rust
//! Application services: opening and saving projects, commands, undo/redo,
//! search, selection, and reports.
//!
//! This crate orchestrates. It holds no domain rules (those live in
//! `knx-core`) and no storage or format knowledge (those live in
//! `knx-store`, `knx-etsproj` and `knx-productdb`).
```

The other five doc comments:

- `knx-store`: `//! SQLite project storage, schema migrations, and the opaque passthrough store.`
- `knx-etsproj`: `//! Reading and writing `.knxproj`: ZIP container, schema detection, tolerant XML parsing, mapping to and from `knx-core`, and the import report.`
- `knx-productdb`: `//! Product database: manufacturer, hardware, application program and version data in a separate SQLite file shared across projects.`
- `knx-net`: `//! KNXnet/IP: discovery, tunnelling, routing, cEMI and telegrams.`
- `knx-secure`: `//! Isolated key material subsystem. Key material never enters the project model, a report, an export, or a log.`

- [ ] **Step 5: Write a failing test for `Layer` and `Resolved`**

`crates/knx-core/Cargo.toml`:

```toml
[package]
name = "knx-core"
version = "0.0.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
publish = false

[dependencies]
```

`crates/knx-core/src/lib.rs`:

```rust
//! The KNX domain model: entities, addresses, datapoint types, override
//! resolution and validation.
//!
//! This crate performs no IO. It does not parse XML, does not touch SQL, and
//! knows nothing about any file format or the user interface.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_layer_values_are_not_exported() {
        let from_program = Resolved { value: 1u8, layer: Layer::Program };
        let from_instance = Resolved { value: 1u8, layer: Layer::Instance };
        let inferred = Resolved { value: 1u8, layer: Layer::Inferred };
        let edited = Resolved { value: 1u8, layer: Layer::UserEdit };

        assert!(!from_program.layer.is_exported());
        assert!(!inferred.layer.is_exported());
        assert!(from_instance.layer.is_exported());
        assert!(edited.layer.is_exported());
    }
}
```

- [ ] **Step 6: Run the test to verify it fails**

Run: `cargo test -p knx-core`
Expected: FAIL — `cannot find type `Resolved` in this scope`, `cannot find type `Layer` in this scope`.

- [ ] **Step 7: Write the minimal implementation**

Insert above the `#[cfg(test)]` block in `crates/knx-core/src/lib.rs`:

```rust
/// Where a resolved value came from.
///
/// A communication object's effective properties resolve through three layers
/// in the source data (`ComObject` → `ComObjectRef` → `ComObjectInstanceRef`).
/// 758 of 907 instances in the reference project override the datapoint type at
/// instance level, so a model without provenance cannot decide what to write
/// back on export. See the architecture spec, section 5.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Default from the application program's `ComObject`.
    Program,
    /// Per-variant override from the application program's `ComObjectRef`.
    ProgramRef,
    /// Per-device override that was present in the imported project.
    Instance,
    /// Derived by this application, for example a datapoint type taken from
    /// linked communication objects. Never written back as if the user set it.
    Inferred,
    /// Changed in this application.
    UserEdit,
}

impl Layer {
    /// Whether a value carrying this layer is written back to the project file
    /// on export.
    ///
    /// Program-level values belong to the product database and inferred values
    /// are ours, not the user's; neither is exported.
    pub fn is_exported(self) -> bool {
        matches!(self, Layer::Instance | Layer::UserEdit)
    }
}

/// A value together with the layer it was resolved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Resolved<T> {
    pub value: T,
    pub layer: Layer,
}
```

- [ ] **Step 8: Run the test to verify it passes**

Run: `cargo test -p knx-core`
Expected: PASS, 1 test.

- [ ] **Step 9: Create the CLI entry point**

`apps/knx-cli/Cargo.toml`:

```toml
[package]
name = "knx-cli"
version = "0.0.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
publish = false

[[bin]]
name = "knx"
path = "src/main.rs"

[dependencies]
knx-app.workspace = true
```

`apps/knx-cli/src/main.rs`:

```rust
//! Headless entry point. Keeping a real CLI alongside the desktop application
//! is what forces the core to stay free of user-interface dependencies and
//! makes import, roundtrip and regression tests runnable in CI without a
//! display.

fn main() {
    println!("knx {}", env!("CARGO_PKG_VERSION"));
}
```

No subcommands yet. `import`, `report`, `export` and `monitor` arrive with the features they drive.

- [ ] **Step 10: Ignore build output**

Append to `.gitignore`:

```gitignore
/target/
```

- [ ] **Step 11: Verify the whole workspace builds and tests clean**

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: all four succeed. If `cargo fmt --all --check` reports diffs, run `cargo fmt --all` and re-check.

- [ ] **Step 12: Commit**

```bash
git add rust-toolchain.toml Cargo.toml Cargo.lock crates apps .gitignore
git commit -m "feat: add Cargo workspace skeleton with knx-core provenance types"
```

---

### Task 2: Layering gate in `xtask`

Enforces spec §3.1 rule 1 mechanically: `knx-core` must not reach `serde_json`, `quick-xml`, `rusqlite` or `tokio`.

**Files:**
- Create: `xtask/Cargo.toml`, `xtask/src/main.rs`, `xtask/src/layering.rs`
- Modify: `Cargo.toml` (add `"xtask"` to `members`)

**Interfaces:**
- Consumes: the workspace package names from Task 1.
- Produces: binary `xtask` with subcommand `check-layering`; function `layering::forbidden_reachable(graph: &DepGraph, root: &str, forbidden: &[&str]) -> Vec<Violation>`; types `DepGraph { edges: BTreeMap<String, Vec<String>> }` and `Violation { root: String, forbidden: String, path: Vec<String> }`. Task 4 invokes `cargo run -p xtask -- check-layering`.

- [ ] **Step 1: Add `xtask` to the workspace members**

In `Cargo.toml`, add `"xtask",` to the `members` list after `"apps/knx-cli",`.

- [ ] **Step 2: Create the `xtask` package**

`xtask/Cargo.toml`:

```toml
[package]
name = "xtask"
version = "0.0.0"
edition.workspace = true
publish = false

[dependencies]
cargo_metadata = "0.18"
```

- [ ] **Step 3: Write failing tests for the reachability check**

`xtask/src/layering.rs`:

```rust
use std::collections::{BTreeMap, BTreeSet};

/// A package dependency graph, keyed by package name.
#[derive(Debug, Default)]
pub struct DepGraph {
    pub edges: BTreeMap<String, Vec<String>>,
}

/// One forbidden package reachable from a root package.
#[derive(Debug, PartialEq, Eq)]
pub struct Violation {
    pub root: String,
    pub forbidden: String,
    /// The shortest dependency path from root to the forbidden package,
    /// including both ends. Printed so the reader can see how it got in.
    pub path: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(&str, &[&str])]) -> DepGraph {
        DepGraph {
            edges: edges
                .iter()
                .map(|(k, v)| {
                    (k.to_string(), v.iter().map(|s| s.to_string()).collect())
                })
                .collect(),
        }
    }

    #[test]
    fn clean_graph_has_no_violations() {
        let g = graph(&[("knx-core", &["thiserror"]), ("thiserror", &[])]);
        assert_eq!(forbidden_reachable(&g, "knx-core", &["rusqlite"]), vec![]);
    }

    #[test]
    fn direct_forbidden_dependency_is_reported_with_path() {
        let g = graph(&[("knx-core", &["rusqlite"]), ("rusqlite", &[])]);
        let found = forbidden_reachable(&g, "knx-core", &["rusqlite"]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].forbidden, "rusqlite");
        assert_eq!(found[0].path, vec!["knx-core", "rusqlite"]);
    }

    #[test]
    fn transitive_forbidden_dependency_is_reported_with_full_path() {
        let g = graph(&[
            ("knx-core", &["fancy-parser"]),
            ("fancy-parser", &["quick-xml"]),
            ("quick-xml", &[]),
        ]);
        let found = forbidden_reachable(&g, "knx-core", &["quick-xml"]);
        assert_eq!(found[0].path, vec!["knx-core", "fancy-parser", "quick-xml"]);
    }

    #[test]
    fn dependency_cycles_do_not_hang() {
        let g = graph(&[("a", &["b"]), ("b", &["a"])]);
        assert_eq!(forbidden_reachable(&g, "a", &["tokio"]), vec![]);
    }
}
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `cargo test -p xtask`
Expected: FAIL — `cannot find function `forbidden_reachable` in this scope`.

- [ ] **Step 5: Implement the reachability check**

Insert into `xtask/src/layering.rs`, above the test module:

```rust
/// Breadth-first search from `root`, reporting every forbidden package that is
/// reachable, with the shortest path to it.
///
/// Breadth-first rather than depth-first so the reported path is the shortest
/// one, which is the most useful thing to show someone who has to remove it.
pub fn forbidden_reachable(
    graph: &DepGraph,
    root: &str,
    forbidden: &[&str],
) -> Vec<Violation> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut queue: std::collections::VecDeque<Vec<String>> =
        std::collections::VecDeque::new();
    let mut violations = Vec::new();

    seen.insert(root);
    queue.push_back(vec![root.to_string()]);

    while let Some(path) = queue.pop_front() {
        let current = path.last().expect("path is never empty");

        if forbidden.contains(&current.as_str()) && current != root {
            violations.push(Violation {
                root: root.to_string(),
                forbidden: current.clone(),
                path: path.clone(),
            });
            continue;
        }

        let Some(deps) = graph.edges.get(current) else {
            continue;
        };
        for dep in deps {
            if seen.insert(dep.as_str()) {
                let mut next = path.clone();
                next.push(dep.clone());
                queue.push_back(next);
            }
        }
    }

    violations
}
```

The `seen` set is what makes the cycle test pass. Note the borrow of `dep.as_str()` into `seen` while `graph` is alive — both live as long as the function, so this compiles.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p xtask`
Expected: PASS, 4 tests.

- [ ] **Step 7: Build the graph from `cargo metadata` and wire up the binary**

Append to `xtask/src/layering.rs`:

```rust
/// Packages `knx-core` must never reach. Spec section 3.1, rule 1.
pub const CORE_FORBIDDEN: &[&str] = &["serde_json", "quick-xml", "rusqlite", "tokio"];

/// Build the resolved dependency graph of the whole workspace, including
/// transitive third-party dependencies.
pub fn workspace_graph() -> Result<DepGraph, String> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .map_err(|e| format!("cargo metadata failed: {e}"))?;

    let name_of: std::collections::HashMap<_, _> = metadata
        .packages
        .iter()
        .map(|p| (p.id.clone(), p.name.clone()))
        .collect();

    let resolve = metadata
        .resolve
        .ok_or_else(|| "cargo metadata returned no resolve graph".to_string())?;

    let mut edges = BTreeMap::new();
    for node in resolve.nodes {
        let Some(name) = name_of.get(&node.id) else {
            continue;
        };
        let deps = node
            .deps
            .iter()
            .filter_map(|d| name_of.get(&d.pkg).cloned())
            .collect();
        edges.insert(name.clone(), deps);
    }

    Ok(DepGraph { edges })
}
```

`xtask/src/main.rs`:

```rust
//! Repository verification tasks that are too specific for a Cargo lint.
//!
//! These are architectural rules that would otherwise erode silently, so they
//! run in CI rather than living in a document.

mod layering;

use std::process::ExitCode;

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("check-layering") => check_layering(),
        Some(other) => {
            eprintln!("unknown task: {other}");
            eprintln!("available tasks: check-layering");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("usage: cargo run -p xtask -- <task>");
            eprintln!("available tasks: check-layering");
            ExitCode::FAILURE
        }
    }
}

fn check_layering() -> ExitCode {
    let graph = match layering::workspace_graph() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let violations =
        layering::forbidden_reachable(&graph, "knx-core", layering::CORE_FORBIDDEN);

    if violations.is_empty() {
        println!("layering ok: knx-core reaches none of {:?}", layering::CORE_FORBIDDEN);
        return ExitCode::SUCCESS;
    }

    for v in &violations {
        eprintln!(
            "layering violation: {} reaches forbidden package {} via {}",
            v.root,
            v.forbidden,
            v.path.join(" -> ")
        );
    }
    eprintln!(
        "\nknx-core must perform no IO and must not depend on any format, \
         storage or async runtime. See the architecture spec, section 3.1."
    );
    ExitCode::FAILURE
}
```

- [ ] **Step 8: Run the gate against the real workspace**

Run: `cargo run -p xtask -- check-layering`
Expected: `layering ok: knx-core reaches none of ["serde_json", "quick-xml", "rusqlite", "tokio"]`, exit code 0.

- [ ] **Step 9: Prove the gate actually catches a violation**

Temporarily add to `crates/knx-core/Cargo.toml` under `[dependencies]`:

```toml
serde_json = "1"
```

Run: `cargo run -p xtask -- check-layering`
Expected: FAIL, exit code 1, with `layering violation: knx-core reaches forbidden package serde_json via knx-core -> serde_json`.

Then remove that line again and re-run; expected: back to ok. A gate that has never been seen to fail is not known to work.

- [ ] **Step 10: Verify formatting and lints**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected: all succeed.

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml Cargo.lock xtask
git commit -m "feat: add xtask layering gate keeping knx-core IO-free"
```

---

### Task 3: License gate with `cargo-deny`

Enforces spec §3.1 rule 2 — no GPL crate in the runtime graph (RESEARCH R6).

**Files:**
- Create: `deny.toml`

**Interfaces:**
- Consumes: the workspace from Task 1.
- Produces: `cargo deny check` passing. Task 4 runs it in CI.

- [ ] **Step 1: Install `cargo-deny`**

```bash
cargo install --locked cargo-deny
cargo deny --version
```

- [ ] **Step 2: Write the configuration**

`deny.toml`:

```toml
# Licence policy. The hard constraint is RESEARCH R6: no GPL-licensed crate may
# enter the runtime dependency graph. With the version 2 licence engine, any
# licence not listed in `allow` is denied, so the allowlist below is the policy
# — there is no separate deny list to keep in sync.

[licenses]
version = 2
confidence-threshold = 0.9
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "CC0-1.0",
    "MPL-2.0",
]

[bans]
multiple-versions = "warn"
wildcards = "deny"

[advisories]
version = 2
yanked = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

`MPL-2.0` is allowed because it is file-level copyleft and does not reach into our code; GPL and LGPL are absent from the allowlist and therefore denied.

- [ ] **Step 3: Run the gate**

Run: `cargo deny check`
Expected: PASS for `licenses`, `bans`, `advisories` and `sources`. If a transitive dependency of `cargo_metadata` carries a licence not on the list, do **not** widen the list reflexively — read the licence, decide whether it is acceptable, and if it is, add it with a one-line comment saying why.

- [ ] **Step 4: Prove the gate rejects a licence that is not on the allowlist**

A gate that has never been seen to fail is not known to work. Rather than hunting for a GPL crate to add, exercise the same code path deterministically: temporarily remove the `"MIT",` line from the `allow` list.

Run: `cargo deny check licenses`
Expected: FAIL, listing MIT-licensed dependencies as rejected.

Restore the `"MIT",` line and re-run.
Expected: PASS.

This proves the mechanism that would reject GPL: with `version = 2`, any licence absent from `allow` is rejected, and GPL is absent.

- [ ] **Step 5: Commit**

```bash
git add deny.toml
git commit -m "feat: add cargo-deny license gate rejecting GPL dependencies"
```

---

### Task 4: Continuous integration

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `cargo run -p xtask -- check-layering` (Task 2), `deny.toml` (Task 3), `rust-toolchain.toml` (Task 1).
- Produces: a CI workflow that fails on formatting, lints, test failures, layering violations or licence violations.

- [ ] **Step 1: Write the workflow**

`.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build, test, lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          components: rustfmt, clippy

      - name: Check formatting
        run: cargo fmt --all --check

      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: Test
        run: cargo test --workspace

      - name: Layering gate
        run: cargo run -p xtask -- check-layering

  deny:
    name: License and advisory gate
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
        with:
          command: check
```

`setup-rust-toolchain` reads `rust-toolchain.toml`, so the pinned version is used and caching is handled by the action.

- [ ] **Step 2: Verify the workflow steps pass locally**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xtask -- check-layering
cargo deny check
```

Expected: all five succeed. Everything CI runs must be runnable locally with the same command; if a step can only be run on CI, it will be ignored.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: run tests, lints, layering and license gates"
```

---

### Task 5: Architecture decision records

Nine ADRs, plus a template and an index. Each records a decision already made and approved; the value is the *context and consequences*, so that a future session can re-open a decision by re-checking its evidence rather than re-arguing it.

**Files:**
- Create: `docs/adr/README.md`, `docs/adr/template.md`
- Create: `docs/adr/0001-technology-stack.md` … `docs/adr/0009-ui-boundary.md`

**Interfaces:**
- Consumes: the spec.
- Produces: ADR numbers 0001–0009, referenced by `docs/ARCHITECTURE.md` in Task 6.

- [ ] **Step 1: Write the template**

`docs/adr/template.md`:

```markdown
# ADR NNNN: <Title>

Date: YYYY-MM-DD
Status: Proposed | Accepted | Superseded by ADR-NNNN
Session: N

## Context

What forces this decision. Cite evidence — a RESEARCH.md section, a measured
number, a licence, a standard. State what is verified and what is assumed.

## Decision

What we do. One or two paragraphs, in the present tense.

## Alternatives considered

Each with the reason it was not chosen.

## Consequences

What becomes easier, what becomes harder, and what has to be enforced or
tested as a result.
```

- [ ] **Step 2: Write the nine ADRs**

All are `Status: Accepted`, `Date: 2026-09-02`, `Session: 1`. Content for each, following the template:

**0001 — Technology stack: Rust core, Tauri, React, SQLite.**
Context: 22 MB of application program XML per project and 5.7 MB single files (RESEARCH §4.1) demand streaming and indexing; the provenance model needs a type system; `xknxproject` is GPL-2.0-only (RESEARCH §10) and must not be in the runtime graph.
Decision: Rust core, Tauri shell, React + TypeScript UI, SQLite for storage, English for code and docs.
Alternatives: Python + PySide6 (fastest start, continues Session 0, but weak on the product database and drags packaging problems); TypeScript + Electron (one language, but poor at large XML and has no usable KNX stack — KNXnet/IP would have to be written anyway).
Consequences: two languages at the UI boundary; generated bindings needed (ADR-0009); the GPL exposure disappears at the root because no Python KNX library is in the runtime graph.

**0002 — Own `.knxproj` parser; `xknxproject` as a test oracle only.**
Context: measured losses in `xknxproject` (RESEARCH §7.1) — all 1390 parameter values, the unassigned device, `BusAccess`, `BinaryData`, `CompletionStatus`, the five `*Loaded` flags, `Broken`, `BCUKey`, `SplitType`, `Central`/`Unfiltered`, `DefaultLine`, send/receive direction; and no export path at all. Plus GPL-2.0-only.
Decision: write our own reader and writer. `xknxproject` stays in `.venv` as a cross-check oracle for development and tests, never a runtime dependency.
Alternatives: depend on `xknxproject` (forces the whole application to GPL-2.0 and still cannot export); port it (same data losses, plus a derivative-work question).
Consequences: more work up front; enforced by the `cargo-deny` gate and by the fact that no Rust crate can reach it at all.

**0003 — SQLite as the native project format.**
Context: CLAUDE.md requires a versioned, migratable model; import formats must not dictate the model; 500+ group addresses and 900+ communication objects per project need indexed access; opaque data is binary.
Decision: one SQLite file per project as the working format, with `user_version` carrying the schema version and an ordered migration chain. A text export/import format may be added later for diagnostics and version control, once a need is demonstrated.
Alternatives: `.knxproj` as the working format (rejected — the ETS schema would become the model, and versioning would be impossible); a directory of JSON/TOML files (rejected — no transactional save, and a 22 MB product data set is never usefully diffable anyway).
Consequences: saves are atomic and incremental; project files are not diffable; migrations are SQL plus Rust and each version needs a frozen fixture test.

**0004 — Provenance and override-chain model.**
Context: RESEARCH §3.2 — three resolution layers, and 758 of 907 communication object instances override the datapoint type at instance level. An importer reading only the application program is wrong for the majority of objects, and an exporter cannot decide what to write without knowing where a value came from.
Decision: `Resolved<T> { value, layer }` with `Layer ∈ {Program, ProgramRef, Instance, Inferred, UserEdit}`. Only `Instance` and `UserEdit` are exported. Any command that changes a value sets its layer to `UserEdit`.
Alternatives: store only resolved values (cannot export correctly); store only raw layers and resolve on read (correct but makes every read a resolution pass, and the UI needs the resolved value everywhere).
Consequences: the export rule is a property of the data rather than of bookkeeping; re-resolution is required if a device's application program reference changes.

**0005 — Separate, shared product database.**
Context: 22 MB per project of manufacturer data, 12 distinct application programs in one project (RESEARCH §4.1); redistribution of manufacturer product data is a licensing risk (RESEARCH §10); CLAUDE.md forbids hard-coding manufacturer products.
Decision: manufacturer data lives in its own SQLite database outside the project file, shared across projects, keyed by `(manufacturer, application_program, version)` with a hash. Projects reference it and remain openable without it, showing only the `Instance` layer, clearly marked incomplete.
Alternatives: embed manufacturer data per project (duplicates tens of megabytes and entangles licensing with project data).
Consequences: the licensing separation is structural; a missing product database degrades gracefully instead of blocking; ingest and cache invalidation need designing in Session 4.

**0006 — Opaque passthrough store.**
Context: RESEARCH §7 — signatures we cannot regenerate, vendor plug-in DLLs, per-device binary blobs, legacy ETS3 plugin data, `Legacy*` option flags, certification metadata. Plus, with no authoritative XSD available (RESEARCH §2.2, R2), unknown XML constructs are expected rather than exceptional.
Decision: a table `(source_path, kind, bytes, sha256)` in the project file retains everything not modelled, verbatim, including unknown XML fragments. Never executed, never interpreted, written back unchanged on export. If the importer drops something with neither a model representation nor an opaque entry, that is a bug in the importer.
Alternatives: discard unknown data (violates the data integrity requirement); model everything (impossible without the specification and the vendor DLLs).
Consequences: round trips survive constructs we do not understand; the first ETS5/ETS6 import produces a concrete list of unknowns instead of a crash; project files grow by the size of the opaque data.

**0007 — Roundtrip fidelity definition.**
Context: RESEARCH R4 — byte-exact round trips are impossible (signatures, attribute ordering, ETS-internal ids), and R9 — whether ETS re-imports an unsigned third-party file is untested.
Decision: fidelity is defined as three testable guarantees — semantic equality of the model under a declared comparison relation, hash equality of all opaque bytes, and an explicit statement that every export is unsigned. Byte-exactness is never claimed.
Alternatives: aim for byte-exactness (not achievable); leave fidelity undefined (makes the claim untestable and therefore worthless).
Consequences: three concrete test cases; the export path must warn the user about the unsigned result until R9 is settled.

**0008 — Key material isolation.**
Context: RESEARCH §9 — Data Secure runtime keys, tool keys and FDSKs are secrets. None are present in the sample installation, so everything about them is documented rather than verified.
Decision: `knx-secure` is a separate crate with its own storage, created before the feature exists. Key material never enters the project model, an import report, an export, a log, or a default diagnostic dump. `knx-secure` does not even depend on `knx-core`.
Alternatives: add key handling when KNX Secure is implemented (retrofitting isolation is how secrets leak).
Consequences: an empty crate carried for several sessions; a test asserting `knx-secure` types are not serialisable toward report or export paths.

**0009 — UI boundary via generated projections.**
Context: CLAUDE.md requires that the core not depend on the UI and that UI workarounds not substitute for domain fixes. Large tables (514 group addresses, 907 communication objects) must not be shipped whole into a browser.
Decision: Tauri commands expose display-shaped projections (`ProjectTree`, `DeviceList`, `GroupAddressTable`, `Inspector<T>`), generated into TypeScript with `ts-rs` so they cannot drift. The UI sends `Command` values back and never holds a mutable reference to the model. Filtering and pagination happen in Rust.
Alternatives: mirror domain types into TypeScript one-to-one (couples the UI to the model and ships too much data); hand-written bindings (drift silently).
Consequences: a projection layer to maintain; the UI cannot invent state; a build step generating bindings.

- [ ] **Step 3: Write the index**

`docs/adr/README.md`: a one-paragraph explanation of what an ADR is here, a pointer to `template.md`, and a table of the nine ADRs with number, title, status and date.

- [ ] **Step 4: Verify every ADR is complete**

```bash
grep -L "## Consequences" docs/adr/0*.md
grep -rn "TBD\|TODO" docs/adr/
```

Expected: both produce no output.

- [ ] **Step 5: Commit**

```bash
git add docs/adr
git commit -m "docs: add ADRs 0001-0009 for the Session 1 architecture"
```

---

### Task 6: `docs/ARCHITECTURE.md`

**Files:**
- Create: `docs/ARCHITECTURE.md`

**Interfaces:**
- Consumes: spec §§1–4, 7–10; ADRs 0001–0009.
- Produces: the document `DATA_MODEL.md` and `IMPORT_EXPORT.md` refer to for layering.

- [ ] **Step 1: Write the document**

Sections, in order:

1. **Purpose and scope** — what the application is, the v1 target from spec §1, and the four exclusions with their reasons.
2. **Layering** — the UI → application → domain → infrastructure stack, and the statement that the KNX core must not depend on the UI.
3. **Workspace layout** — the crate tree and dependency diagram from spec §3, matching what Task 1 actually created, with the note that `apps/knx-desktop` arrives in Session 5.
4. **Enforced rules** — the three rules of spec §3.1, each naming the mechanism that enforces it (`cargo run -p xtask -- check-layering`, `cargo deny check`, and, for rule 3, a statement that it is not yet mechanically enforced because no UI exists).
5. **Core approach** — the normalized model with per-value provenance, and the two rejected alternatives from spec §4 with their reasons.
6. **Application layer** — commands, undo/redo, where validation lives, transactional incremental saving.
7. **UI boundary** — projections, `ts-rs`, server-side filtering (ADR-0009).
8. **KNXnet/IP** — the `BusConnection` interface, the bus monitor as a consumer, `BusAccess` preserved verbatim and not translated, commissioning out of scope but not blocked.
9. **Key material** — the `knx-secure` rules (ADR-0008).
10. **Test strategy** — the seven levels from spec §11 (unit, golden, oracle, roundtrip, migration, malformed input, license/layering), each with what it covers and which of them exist today. State that the golden numbers are reproducible independently via `tools/inspect_knxproj.py`, which is what makes them an oracle rather than a self-generated expectation.
11. **Decision index** — a table linking each ADR.

Every section that restates a fact from RESEARCH.md cites its section number, so a reader can check the evidence.

- [ ] **Step 2: Verify internal links resolve**

```bash
grep -o "](\.\./[^)]*)\|](\./[^)]*)\|](docs/[^)]*)\|](adr/[^)]*)" docs/ARCHITECTURE.md
```

Check each printed target exists on disk.

- [ ] **Step 3: Commit**

```bash
git add docs/ARCHITECTURE.md
git commit -m "docs: add ARCHITECTURE.md"
```

---

### Task 7: `docs/DATA_MODEL.md`

**Files:**
- Create: `docs/DATA_MODEL.md`

**Interfaces:**
- Consumes: spec §5; `knx_core::{Layer, Resolved}` from Task 1.
- Produces: the reference Session 2 implements against.

- [ ] **Step 1: Write the document**

Sections:

1. **Scope** — this describes the target model; Session 1 implements only `Layer` and `Resolved<T>`, and each section marks whether it is implemented or planned.
2. **Identity** — internal IDs versus `SourceRef`, and why ETS ids are never primary keys.
3. **The override chain** — the three source layers, the `Layer` enum, the export table from spec §5.2, and the number that forces it (758 of 907).
4. **Entities** — Project, Installation, Area, Line, Device, GroupAddress, GroupRange, ComObjectInstance, ParameterInstance, BuildingPart, with the attribute sets actually observed in RESEARCH §3 and a note of which are modelled, which are opaque, and which are deferred.
5. **Two orthogonal hierarchies** — devices owned once, referenced by topology and by buildings; unassigned devices are valid.
6. **Directional links** — `GroupLink` with send/receive, and the 569/27 counts.
7. **Commissioning state** — the struct from spec §5.5 and why it is domain data.
8. **Localized strings** — the string table, the 5919-translation figure, resolution and fallback.
9. **Addresses and datapoint types** — the address types, the group address styles, and the rules that a datapoint type may be absent (194 of 514) and a group address may be unlinked (110 of 514).
10. **Retained but uninterpreted** — parameter values, memory layout, load procedures.
11. **Versioning and migration** — `schema_version`, the ordered migration chain, the frozen-fixture rule.

- [ ] **Step 2: Verify the Rust snippets compile as written**

Extract each `rust` block that claims to be current code and check it against `crates/knx-core/src/lib.rs`. The `Layer` and `Resolved<T>` definitions in the document must be character-identical to the implementation, or the document is already wrong.

- [ ] **Step 3: Commit**

```bash
git add docs/DATA_MODEL.md
git commit -m "docs: add DATA_MODEL.md"
```

---

### Task 8: `docs/IMPORT_EXPORT.md`

**Files:**
- Create: `docs/IMPORT_EXPORT.md`

**Interfaces:**
- Consumes: spec §6; RESEARCH §§2, 6, 7.
- Produces: the contract Session 3 implements.

- [ ] **Step 1: Write the document**

Sections:

1. **Pipeline** — the six stages from spec §6.1, each with its responsibility and its own error type, and the rule that no stage knows the next.
2. **Container handling** — `.knxproj` as ZIP; the observed entry layout from RESEARCH §2.1; the `Project.xml` versus `project.xml` case difference; password protection for schema < 21 (ZipCrypto) and ≥ 21 (AES with the documented PBKDF2 derivation), marked unverified in practice.
3. **Schema detection** — read the trailing integer of the default namespace, never assume; the version-to-ETS-generation table from RESEARCH §2.2 with its `[V]`/`[D]` markers intact.
4. **Tolerant parsing** — known-element lists per schema version; unknown constructs go to the opaque store *and* the report; streaming rather than DOM, with the file-size numbers that force it.
5. **Opaque store** — the table shape, the full content list from RESEARCH §7, and the three rules (never execute, never interpret, write back unchanged).
6. **The import report** — the `ImportReport` struct from spec §6.4, what each field is for, and the rule that silently dropping information is an importer bug.
7. **Inference and conflicts** — datapoint type inference from linked objects, always marked `Inferred` and never exported; conflicts reported rather than resolved.
8. **Export** — what is written from which layer; that every export is unsigned; the user-facing warning that ETS re-import is untested.
9. **Roundtrip fidelity** — the three guarantees, each as a named test.
10. **Product database ingest** — manufacturer data is ingested into the shared database, not copied into the project; projects open without it.

- [ ] **Step 2: Verify no compatibility claim is unqualified**

```bash
grep -n "ETS compatib\|fully compatible\|KNX certified" docs/IMPORT_EXPORT.md
```

Expected: no hit, or only hits inside a sentence that explicitly denies the claim.

- [ ] **Step 3: Commit**

```bash
git add docs/IMPORT_EXPORT.md
git commit -m "docs: add IMPORT_EXPORT.md"
```

---

### Task 9: `docs/COMPATIBILITY.md` and `docs/KNOWN_LIMITATIONS.md`

Two documents, one task: they are two halves of the same statement — what we can do, and what we cannot — and reviewing one without the other invites contradictions.

**Files:**
- Create: `docs/COMPATIBILITY.md`
- Create: `docs/KNOWN_LIMITATIONS.md`

**Interfaces:**
- Consumes: RESEARCH §§2.2, 7, 9, 10, 11; spec §§1, 13.
- Produces: the documents the import report's `unsupported` entries point users to.

- [ ] **Step 1: Write `docs/COMPATIBILITY.md`**

Sections:

1. **Wording policy** — "KNX-compatible" only; never "KNX certified", never "full ETS compatibility", with the trademark and plug-in reasons.
2. **Verified today** — a table of what has actually been tested, currently: schema 11 (ETS 4.1.8) read of one real project, and KNXnet/IP tunnelling against one gateway. Each row names the evidence.
3. **Expected but unverified** — schema 12, 13, 14, 20, 21+; password-protected projects; ETS re-import of our export. Each row states what would have to be done to move it into the verified table.
4. **Not supported** — vendor plug-in DLLs, commissioning, KNX Secure, `.knxprod` for master data scheme ≥ 12.
5. **How compatibility is reported to the user** — the import report's `unknown`, `unsupported` and `conflicts` sections.

The table structure matters more than the current contents: this document is expected to change every time a new sample project is imported.

- [ ] **Step 2: Write `docs/KNOWN_LIMITATIONS.md`**

Each entry states the limitation, its cause, its impact on the user, and the condition under which it would be lifted:

1. Single-sample bias — everything verified is schema 11 from one project (RESEARCH R1).
2. No authoritative XSD is publicly available, so imports are tolerant rather than validating (R2).
3. Device parameters are imported and exported but not interpreted; no parameter editor in v1 (R3).
4. Round trips are semantic, not byte-exact (R4).
5. Exports are unsigned; whether ETS accepts them is untested (R9).
6. Devices whose configuration depends on a vendor plug-in DLL cannot be configured by any independent tool (R5).
7. Commissioning and device download are out of scope (RESEARCH §8.3).
8. KNX Secure is not implemented (RESEARCH §9).
9. Project files are not diffable, being SQLite (ADR-0003).
10. The project licence is not yet decided; the workspace currently declares a placeholder.

- [ ] **Step 3: Check the two documents do not contradict each other**

Read them side by side. Anything listed as "verified" in `COMPATIBILITY.md` must not appear as a limitation, and every "not supported" row must have a corresponding limitation entry.

- [ ] **Step 4: Commit**

```bash
git add docs/COMPATIBILITY.md docs/KNOWN_LIMITATIONS.md
git commit -m "docs: add COMPATIBILITY.md and KNOWN_LIMITATIONS.md"
```

---

### Task 10: `docs/ROADMAP.md` and status update

Closes the session: records where the project now stands and what the next one starts from.

**Files:**
- Create: `docs/ROADMAP.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`

**Interfaces:**
- Consumes: everything produced in Tasks 1–9.
- Produces: the entry point for the Session 2 briefing.

- [ ] **Step 1: Write `docs/ROADMAP.md`**

Per session, state the goal, the deliverables, and the entry condition that must hold before it starts:

- **Session 2 — KNX core.** The domain model of `DATA_MODEL.md`: entities, addresses, datapoint types, override resolution, validation rules, the command layer. Entry condition: the workspace builds and both gates pass.
- **Session 3 — ETS project import.** The pipeline of `IMPORT_EXPORT.md` for schema 11, the opaque store, the import report, the golden test against the reference project's counts. Entry condition: the core model exists. Blocking risk: no ETS5/ETS6 sample project (R1) — acquiring one is a prerequisite for claiming anything beyond schema 11.
- **Session 4 — Manufacturer databases.** Product database schema and ingest, plus the `when/@test` grammar research spike (R3) that the parameter editor depends on.
- **Session 5 — UI/UX.** Tauri shell, React application, the projection layer and `ts-rs` bindings, Project Explorer, inspector, command palette. Entry condition: import produces a model worth displaying.
- **Session 6 — KNXnet/IP.** Discovery, tunnelling, routing, cEMI, the bus monitor resolving telegrams against the open project.
- **Session 7 — Integration and hardening.** Roundtrip and migration test suites, large-project performance measurement, packaging.

Then a short section listing the open questions from spec §13 with the session each lands in.

- [ ] **Step 2: Update `docs/IMPLEMENTATION_STATUS.md`**

- Set Session 1 to **Done**, with links to `ARCHITECTURE.md`, the ADR index and the design spec.
- Replace "There is no application code yet" with an accurate description: a Cargo workspace with eight crates, of which only `knx-core` has content (`Layer`, `Resolved<T>`); two enforced gates; CI.
- Extend the "what exists" table with `Cargo.toml`, `crates/`, `apps/knx-cli/`, `xtask/`, `deny.toml`, `.github/workflows/ci.yml`, and each new document.
- Update the environment section: Rust toolchain version, `cargo-deny`, and the standing note that the Python `.venv` is test-only.
- State the Session 2 entry point.

- [ ] **Step 3: Verify the whole repository is green**

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- check-layering
cargo deny check
grep -rn "TBD\|TODO\|FIXME" docs/ --include="*.md"
```

Expected: the six cargo commands succeed and the grep produces no output.

- [ ] **Step 4: Commit**

```bash
git add docs/ROADMAP.md docs/IMPLEMENTATION_STATUS.md
git commit -m "docs: add ROADMAP.md and record Session 1 as complete"
```

---

## Definition of done

- `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p xtask -- check-layering` and `cargo deny check` all pass.
- Both gates have been observed to fail when deliberately violated (Task 2 Step 9, Task 3 Step 4).
- `docs/` contains `ARCHITECTURE.md`, `DATA_MODEL.md`, `IMPORT_EXPORT.md`, `COMPATIBILITY.md`, `KNOWN_LIMITATIONS.md`, `ROADMAP.md`, and `adr/` with nine ADRs plus a template and index.
- No `TBD`, `TODO` or `FIXME` in `docs/`.
- `IMPLEMENTATION_STATUS.md` describes the repository as it actually is.
