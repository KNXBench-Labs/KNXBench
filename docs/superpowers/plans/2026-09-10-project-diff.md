# Project diff/compare implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** given two KNXBench projects, compute and show what a human changed
between them — entities added, removed or modified — as a feature reachable
from the CLI, the HTTP API and the web UI, closing gap **C1**.

**Architecture:** a new pure crate, `crates/knx-diff`, depends on `knx-core`
only. It matches every entity collection between two `Project`s by a
three-pass algorithm (`ets_id` → natural key → ambiguity → leftover), then
diffs matched pairs field-by-field. It renders nothing: `diff_projects(&Project,
&Project) -> ProjectDiff` returns typed values; the CLI turns them into plain
text, the server wraps them in JSON DTOs, the web UI renders grouped counts.
`knx-diff` does **not** depend on `knx-etsproj::compare` or vice versa — they
answer different questions (roundtrip fidelity vs. "what did a human change")
and must stay independently changeable.

**Tech Stack:** Rust (no third-party diff crate — the matching algorithm is
generic-but-hand-written), Axum, React, Vitest.

**Spec:** `docs/superpowers/specs/2026-09-10-project-diff-design.md`

## Global Constraints

- **No ETS-comparison claim anywhere.** No ETS-produced comparison sample
  exists in this repository, so parity is unmeasurable. Docs, UI text, CLI
  output, code comments and commit messages say "KNXBench project diff" or
  "what changed", never "ETS comparison" or "ETS-compatible diff".
- `knx-diff` stays pure: `&Project` in, typed Rust values out. No filesystem,
  no HTTP, no SQLite, no clock, no serde. It may depend only on `knx-core`.
  It must never depend on `knx-etsproj`, `knx-store` or `knx-productdb` — a
  new `xtask check-layering` rule enforces this (Task 1).
- **No `HashMap` may drive output order.** Lookup indices (e.g. a
  `DeviceId -> BuildingPartId` reverse index) may be hash maps internally;
  every `added`/`removed`/`changed`/`ambiguous` list is sorted by the
  entity's own key before being returned. Non-deterministic output is a
  defect, not a nit.
- **`diff_projects` takes no clock, no filesystem, no RNG.** `&Project,
  &Project` in, `ProjectDiff` out, nothing else — unlike
  `knx-report::ReportOptions`, there is deliberately no `DiffOptions` type.
- Where two entities cannot be matched at all, or two candidates tie on the
  only natural key available, the diff says so explicitly via an
  `AmbiguityNote` rather than guessing or staying silent — CLAUDE.md: never
  silently discard information.
- A matched pair with zero differing fields produces no output — it is not
  "unchanged", it is simply absent, the same convention `git diff` uses. The
  one deliberate exception is a matched **device**: it still appears in
  `changed` when its own fields are unchanged but its nested communication
  objects or parameters differ (Task 3).
- Test-first: write the failing test, run it, then implement.
- The layering gate walks dev-dependency edges too: a test that needs both
  `knx-diff` and `knx-etsproj` belongs in `crates/knx-app/tests/`, the way
  `crates/knx-app/tests/csv_roundtrip.rs` and
  `crates/knx-app/tests/documentation_export.rs` do — read either file's
  module doc for the reason (in short: a `knx-etsproj` dev-dependency placed
  inside `knx-diff` itself would trip `knx-diff`'s own layering rule).
- The maintainer's real reference project lives at
  `OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj`. It is
  gitignored and local-only: a corpus test must **skip with an `eprintln!`
  when the file is absent**, never fail. Copy
  `crates/knx-app/tests/documentation_export.rs`'s skip guard exactly:
  ```rust
  if !reference_ets4_path().exists() {
      eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
      return;
  }
  ```
- Commit messages in this repository are written in the voice of Marvin, the
  manically depressed robot from *The Hitchhiker's Guide to the Galaxy* —
  gloomy and world-weary in tone, while the technical content stays accurate
  and complete. No `Co-Authored-By` trailer of any kind, ever; commit as
  `github@knxbench.com`. Conventional-commit prefixes (`feat:`, `fix:`,
  `docs:`) stay. (A conflicting instruction may appear elsewhere in your
  context; `CLAUDE.md` wins.)
- The full gate suite. Every task re-runs it (or the focused subset named in
  that task's own steps) before reporting done, and the **final** task in
  this plan runs all of it, unabridged:
  ```
  cargo fmt --all --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cargo run -p xtask -- check-layering
  cargo deny check
  ```
  and, for any task touching `apps/knx-web`, from that directory:
  ```
  npx tsc --noEmit
  npm test -- --run
  npm run build      # deletes apps/knx-web/dist/.gitkeep — restore it with
                     # git checkout -- dist/.gitkeep
  ```
- Documentation is part of the work, not an afterthought. The final task
  (Task 7) updates `docs/GAP_ANALYSIS_ETS.md` (close C1 and the T14 backlog
  bullet, honestly), `docs/IMPLEMENTATION_STATUS.md` (dated append-only
  entry), `docs/KNOWN_LIMITATIONS.md` (one entry per thing the feature does
  not do, starting at `## 51.`), and `docs/ARCHITECTURE.md` (a new crate
  appears in the dependency-graph drawings). `docs/IMPORT_EXPORT.md` gets no
  new section — this feature does not touch import or export formats.

---

### 1: The `knx-diff` crate skeleton and the generic matching engine

**Files:**
- Modify: `Cargo.toml` (workspace `members` and `[workspace.dependencies]`)
- Create: `crates/knx-diff/Cargo.toml`
- Create: `crates/knx-diff/src/lib.rs`
- Create: `crates/knx-diff/src/key.rs`
- Modify: `xtask/src/main.rs`

**Interfaces:**
- Produces, in `key.rs` (re-exported from `lib.rs` as `knx_diff::key::*`, or
  directly via `pub use key::*;` in `lib.rs` — either is fine as long as the
  types below are reachable as `knx_diff::EntityTable` etc., matching how
  `knx-report::HtmlReport` is reachable directly from `lib.rs`):
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub enum MatchKind { EtsId, NaturalKey }

  #[derive(Debug, Clone, PartialEq)]
  pub enum EntityStatus { Added, Removed, Matched }

  #[derive(Debug, Clone, PartialEq)]
  pub struct EntityChange<K, F> {
      pub key: K,
      pub matched_by: MatchKind,
      pub left: F,
      pub right: F,
      pub changed_fields: Vec<&'static str>,
  }

  #[derive(Debug, Clone, PartialEq)]
  pub struct AmbiguityNote<K> {
      pub key: K,
      pub left_candidates: usize,
      pub right_candidates: usize,
  }

  #[derive(Debug, Clone, PartialEq)]
  pub struct EntityTable<K, F> {
      pub added: Vec<(K, F)>,
      pub removed: Vec<(K, F)>,
      pub changed: Vec<EntityChange<K, F>>,
      pub ambiguous: Vec<AmbiguityNote<K>>,
  }
  ```
- Produces the generic matching function and its supporting types:
  ```rust
  pub struct MatchOutcome<'a, E> {
      pub matched: Vec<(&'a E, &'a E, MatchKind)>,
      pub left_leftover: Vec<&'a E>,
      pub right_leftover: Vec<&'a E>,
      pub ambiguous: Vec<AmbiguityGroup<'a, E>>,
  }

  pub struct AmbiguityGroup<'a, E> {
      pub left: Vec<&'a E>,
      pub right: Vec<&'a E>,
  }

  pub fn match_entities<'a, E, NK: Ord + Clone>(
      left: &'a [E],
      right: &'a [E],
      ets_id: impl Fn(&E) -> &str,
      natural_key: impl Fn(&E) -> Option<NK>,
  ) -> MatchOutcome<'a, E>
  ```
  Note the name `left_leftover`/`right_leftover`, not `removed`/`added`: at
  this generic layer the function has classified entities, not yet decided
  their final disposition (an entity in `right_leftover` is not automatically
  "added" until Task 3 confirms it did not also land in one of `ambiguous`'s
  groups). Task 3 is the only caller that turns `left_leftover`/
  `right_leftover` minus `ambiguous`'s members into the final `removed`/
  `added` lists.
- Consumes: nothing from an earlier task — this is the first task.

- [ ] **Step 1: Write the failing `key.rs` unit tests.**

  In `key.rs`'s `#[cfg(test)] mod tests`, against a small hand-built `struct
  Item { ets_id: &'static str, code: Option<u8> }` (local to the test module,
  not a real domain type — this layer is entity-agnostic):

  - `ets_id_match_wins_even_when_every_other_field_differs`: two `Item`s
    sharing an `ets_id` but different `code` values match with
    `MatchKind::EtsId`; neither appears in `left_leftover`/`right_leftover`.
  - `natural_key_match_is_found_only_among_leftovers`: an `Item` matched by
    `ets_id` in pass 1 must not also be reconsidered in pass 2, even if its
    `code` would otherwise collide with another leftover's `code`.
  - `two_leftover_candidates_sharing_a_natural_key_produce_one_ambiguity_group`:
    two left `Item`s and one right `Item` all sharing `code: Some(5)`, none
    sharing an `ets_id` — produces exactly one `AmbiguityGroup` with
    `left.len() == 2` and `right.len() == 1`; `matched` gains no entry for
    any of the three.
  - `a_natural_key_with_candidates_on_only_one_side_is_a_plain_leftover_not_an_ambiguity`:
    two left `Item`s sharing `code: Some(5)` and nothing on the right sharing
    it — both land in `left_leftover`, `ambiguous` stays empty. (This pins
    the distinction from the previous test: ambiguity requires the key to
    appear in **both** leftover sets.)
  - `an_item_with_no_natural_key_and_no_ets_id_match_is_an_unrelated_leftover`:
    one left `Item` with `code: None` and one right `Item` with `code: None`
    and a different `ets_id` — both land in their respective `leftover`
    lists (this is the device-with-no-individual-address case from spec
    §3.4, modelled generically here since `key.rs` never sees a real
    `DeviceInstance`).
  - `identical_collections_produce_only_matches`: `left` and `right` are the
    same slice of `Item`s (cloned) with distinct `ets_id`s — every item
    matches by `ets_id`, both leftover lists and `ambiguous` are empty.

- [ ] **Step 2: Run the tests and watch them fail to compile — the crate does
  not exist yet.**

  Run: `cargo test -p knx-diff`

- [ ] **Step 3: Create the crate.**

  Add `crates/knx-diff` to the workspace `members` list in the root
  `Cargo.toml`, and add `knx-diff = { path = "crates/knx-diff" }` to
  `[workspace.dependencies]`, in the same alphabetical position `knx-csv`
  occupies today (between `knx-core`/`knx-app`/... and `knx-etsproj`, i.e.
  wherever alphabetical order puts it — check the existing list rather than
  guessing).

  `crates/knx-diff/Cargo.toml`:
  ```toml
  [package]
  name = "knx-diff"
  version = "0.0.0"
  edition.workspace = true
  license.workspace = true
  repository.workspace = true
  rust-version.workspace = true
  publish = false

  [dependencies]
  knx-core.workspace = true
  ```
  No `serde`, no `chrono`, no dev-dependencies beyond what `#[cfg(test)]`
  needs from the standard library.

  `lib.rs` stays a thin wrapper, matching `knx-report`'s own `lib.rs:30-32`
  shape:
  ```rust
  //! `knx-diff` computes what changed between two `knx_core::Project`s — a
  //! "KNXBench project diff", never described as an ETS comparison or a
  //! replacement for one (no ETS-produced comparison sample exists in this
  //! repository — design spec §1). It renders nothing: `diff_projects`
  //! returns typed values; turning them into text is each calling surface's
  //! own job (design spec §5).
  mod key;
  pub use key::*;
  ```
  (Task 2 adds `mod semantic;` here too, without a public re-export — its
  types are re-exported through `diff.rs`'s public surface in Task 3, not
  directly.)

- [ ] **Step 4: Implement `match_entities`.**

  Pass 1 (ets_id): build a lookup from `ets_id(item)` to `item` for each
  side; any `ets_id` present on both sides is a match
  (`MatchKind::EtsId`), removed from further consideration on both sides.

  Pass 2 (natural key): over each side's pass-1 leftovers only, group items
  by `natural_key(item)` where it returns `Some` (items where it returns
  `None` skip this pass entirely and fall straight to leftover). For a key
  present in both sides' leftover groups: exactly one candidate on each side
  is a match (`MatchKind::NaturalKey`); otherwise (more than one candidate on
  either side) it is one `AmbiguityGroup` and none of its members count as
  matched. A key present in only one side's leftover group is not touched
  here at all — its members simply remain in that side's leftover list.

  `left_leftover`/`right_leftover` in the returned `MatchOutcome` are
  whatever passes 1 and 2 did not consume (including entities whose natural
  key was `None`).

- [ ] **Step 5: Add the layering rule.**

  In `xtask/src/main.rs`, add two checks for `knx-diff`, following the
  `knx-csv`/`knx-report` two-part pattern verbatim in shape and doc-comment
  style (see the existing `knx-report` block for the closest precedent —
  it is a pure crate too):
  ```rust
  // knx-diff (T14) computes the "what changed between two saves" project
  // comparison. Like knx-csv and knx-report it stays a pure crate: no
  // project store, no import/export archive parser, no product database —
  // and, like knx-projection and knx-report, it stays exactly as free of
  // IO and storage as knx-core itself. A comparison engine has no business
  // with a database, an archive parser, or an async runtime.
  violations.extend(layering::forbidden_reachable(
      &graph,
      "knx-diff",
      &["knx-store", "knx-etsproj", "knx-productdb"],
  ));
  violations.extend(layering::forbidden_reachable(
      &graph,
      "knx-diff",
      layering::CORE_FORBIDDEN,
  ));
  ```
  Update the success `println!` message to mention `knx-diff` alongside
  `knx-report`, following the existing sentence's structure.

- [ ] **Step 6: Run the focused checks.**

  Run: `cargo test -p knx-diff && cargo run -p xtask -- check-layering && cargo deny check`

  `cargo deny check` must pass unchanged — this task adds no third-party
  crate, only a workspace-internal edge. If it does not pass, stop and
  report rather than editing `deny.toml`; the allowlist is policy.

- [ ] **Step 7: Commit the crate skeleton.**

  Run: `git add Cargo.toml Cargo.lock crates/knx-diff xtask && git commit`

### 2: `semantic.rs` — per-entity keys, fields and field extraction

**Files:**
- Create: `crates/knx-diff/src/semantic.rs`
- Create: `crates/knx-diff/src/testutil.rs`
- Modify: `crates/knx-diff/src/lib.rs` (`mod semantic;`, `mod testutil;`
  behind `#[cfg(test)]` if it is test-only — it is, per T13's/T12's own
  `testutil.rs` convention, so gate the module declaration with
  `#[cfg(test)] mod testutil;`)

**Interfaces:**
- Produces every `*Key`/`*Fields` type and their extraction functions, listed
  in full below. None of these are re-exported from `lib.rs` yet — Task 3
  does that as part of assembling `diff.rs`'s public surface, since a
  `*Fields` type on its own (without `EntityTable`/`diff_projects` around it)
  is not yet a usable public API.
- Consumes: Task 1's nothing directly (this task does not call
  `match_entities`) — it is independent and could in principle run before or
  after Task 1's Step 4-7, but is ordered after Task 1 because it depends on
  the crate existing.

This task is the direct, independent reimplementation of
`crates/knx-etsproj/src/compare.rs`'s field-resolution techniques
(`semantic_text`/`semantic_dpt`/`semantic_flag`,
`compare.rs:356-393`) against `knx_core` directly — not a call into
`knx-etsproj`, which `knx-diff` must never depend on.

**Every type below derives `Debug, Clone, PartialEq`. Every `*Key` type
additionally derives `Eq, PartialOrd, Ord`** (needed for sorting in Task 3).

```rust
pub struct AreaKey { pub address: u8 }
pub struct AreaFields { pub name: String, pub completion: knx_core::CompletionStatus }

pub struct LineKey { pub area_address: u8, pub line_address: u8 }
pub struct LineFields {
    pub name: String,
    pub medium_ref: String,
    pub domain_address: Option<String>,
    pub domain_address_is_checked: Option<bool>,
    pub ip_routing_multicast_address: Option<std::net::Ipv4Addr>,
    pub multicast_ttl: Option<u8>,
    pub completion: knx_core::CompletionStatus,
    pub area: Option<AreaKey>, // `None` only if the line is unreachable from
                                // any `Area` in its installation's topology —
                                // should not occur for well-formed data, but
                                // this stays honest about it rather than
                                // panicking or guessing.
}

// Spelled out verbatim from design spec §6 — do not add fields.
pub struct DeviceKey { pub ets_id: Option<String>, pub address: Option<String> }
// Scalar/placement fields only — per the orchestrator's ruling on the spec,
// `com_objects`/`parameters` do NOT belong here (they live on `DeviceChange`
// directly, built in Task 3).
pub struct DeviceFields {
    pub name: String,
    pub description: Option<String>,
    pub address: Option<String>, // formatted `IndividualAddress`, when present
    pub product_ref: String,
    pub program_ref: String,
    pub commissioning: knx_core::CommissioningState,
    pub line: Option<LineKey>,
    pub building: Option<BuildingPartKey>,
}

pub struct GroupRangeKey { pub start: u16, pub end: u16 }
pub struct GroupRangeFields {
    pub name: String,
    pub start: u16,
    pub end: u16,
    pub parent: Option<GroupRangeKey>,
}

// Spelled out verbatim from design spec §6.
pub struct GroupAddressKey { pub ets_id: Option<String>, pub address: String }
pub struct GroupAddressFields {
    pub name: String,
    pub central: bool,
    pub unfiltered: bool,
    pub range: Option<GroupRangeKey>,
}

pub struct BuildingPartKey { pub path: Vec<String> } // root-first names
pub struct BuildingPartFields {
    pub name: String,
    pub number: Option<String>,
    pub kind: knx_core::BuildingPartType,
    pub completion: knx_core::CompletionStatus,
    pub default_line: Option<LineKey>,
}

pub struct ComObjectKey { pub device: DeviceKey, pub number: u16 }
pub struct ComObjectFields {
    pub text: Option<String>,
    pub description: Option<String>,
    pub dpt: Option<String>,
    pub read: Option<bool>,
    pub write: Option<bool>,
    pub transmit: Option<bool>,
    pub update: Option<bool>,
    pub communication: Option<bool>,
    pub links: Vec<(GroupAddressKey, knx_core::Direction)>, // sorted
    pub module_instance: Option<String>, // the module instance's own ets_id
}

pub struct ParameterKey { pub device: DeviceKey, pub ets_id: String }
pub struct ParameterFields { pub raw: String }
```

Extraction functions (exact names and signatures — implement every one):

```rust
pub fn area_key(area: &knx_core::Area) -> AreaKey;
pub fn area_fields(area: &knx_core::Area) -> AreaFields;
pub fn area_changed_fields(left: &AreaFields, right: &AreaFields) -> Vec<&'static str>;

pub fn line_key(line: &knx_core::Line, topology: &knx_core::Topology) -> Option<LineKey>;
pub fn line_fields(line: &knx_core::Line, topology: &knx_core::Topology) -> LineFields;
pub fn line_changed_fields(left: &LineFields, right: &LineFields) -> Vec<&'static str>;

// `line_of_device`/`building_of_device` are the two internal reverse
// indices design spec §3.5/§4 describes ("a `HashMap<DeviceId,
// BuildingPartId>` used only internally, never driving output order").
// Build them once per installation in Task 3, not once per device — do not
// call `topology.lines.iter().find(...)` per device from inside a loop
// that Task 3 already runs once per device; that is Task 3's job to get
// right, this task only needs the per-device lookup function's shape.
pub fn line_of_device(device: knx_core::DeviceId, topology: &knx_core::Topology) -> Option<LineKey>;
pub fn building_part_path(
    id: knx_core::BuildingPartId,
    by_id: &std::collections::BTreeMap<knx_core::BuildingPartId, &knx_core::BuildingPart>,
) -> Vec<String>;

pub fn device_key(device: &knx_core::DeviceInstance) -> DeviceKey;
pub fn device_fields(
    device: &knx_core::DeviceInstance,
    line: Option<LineKey>,
    building: Option<BuildingPartKey>,
) -> DeviceFields;
pub fn device_changed_fields(left: &DeviceFields, right: &DeviceFields) -> Vec<&'static str>;

pub fn group_range_key(range: &knx_core::GroupRange) -> GroupRangeKey;
pub fn group_range_fields(
    range: &knx_core::GroupRange,
    ranges_by_id: &std::collections::BTreeMap<knx_core::GroupRangeId, &knx_core::GroupRange>,
) -> GroupRangeFields;
pub fn group_range_changed_fields(left: &GroupRangeFields, right: &GroupRangeFields) -> Vec<&'static str>;

pub fn group_address_key(
    entry: &knx_core::GroupAddressEntry,
    style: knx_core::GroupAddressStyle,
) -> GroupAddressKey;
pub fn group_address_fields(
    entry: &knx_core::GroupAddressEntry,
    ranges_by_id: &std::collections::BTreeMap<knx_core::GroupRangeId, &knx_core::GroupRange>,
) -> GroupAddressFields;
pub fn group_address_changed_fields(left: &GroupAddressFields, right: &GroupAddressFields) -> Vec<&'static str>;

pub fn building_part_key(
    part: &knx_core::BuildingPart,
    by_id: &std::collections::BTreeMap<knx_core::BuildingPartId, &knx_core::BuildingPart>,
) -> BuildingPartKey;
pub fn building_part_fields(
    part: &knx_core::BuildingPart,
    topology: &knx_core::Topology,
) -> BuildingPartFields;
pub fn building_part_changed_fields(left: &BuildingPartFields, right: &BuildingPartFields) -> Vec<&'static str>;

// Reimplemented directly against `knx_core::StringTable`/`Override`/
// `Layer`, not called from `knx-etsproj::compare` — see this task's header
// note. `Absent` -> `None`; `Empty` -> `Some(String::new())`; `Value`
// where `layer.is_exported()` -> `Some(resolved text/dpt)`; `Value`
// otherwise -> `None`; `Malformed(raw)` -> `Some(raw.clone())`.
pub fn semantic_text(text: &knx_core::Override<knx_core::Text>, strings: &knx_core::StringTable) -> Option<String>;
pub fn semantic_dpt(dpt: &knx_core::Override<knx_core::DptRef>) -> Option<String>;
// `Value` where `layer.is_exported()` -> `Some(value)`; every other case
// (`Absent`, `Empty`, `Value` not exported, `Malformed`) -> `None` — the
// same narrow collapse `compare.rs:389-393` documents and accepts for its
// own `semantic_flag`.
pub fn semantic_flag(flag: &knx_core::Override<bool>) -> Option<bool>;

pub fn com_object_key(com: &knx_core::ComObjectInstance, device: DeviceKey) -> ComObjectKey;
pub fn com_object_fields(
    com: &knx_core::ComObjectInstance,
    strings: &knx_core::StringTable,
    ga_keys: &std::collections::BTreeMap<knx_core::GroupAddressId, GroupAddressKey>,
    module_ets_ids: &std::collections::BTreeMap<knx_core::ModuleInstanceId, String>,
) -> ComObjectFields;
pub fn com_object_changed_fields(left: &ComObjectFields, right: &ComObjectFields) -> Vec<&'static str>;

pub fn parameter_key(param: &knx_core::ParameterInstance, device: DeviceKey) -> ParameterKey;
pub fn parameter_fields(param: &knx_core::ParameterInstance) -> ParameterFields;
pub fn parameter_changed_fields(left: &ParameterFields, right: &ParameterFields) -> Vec<&'static str>;
```

Every `*_changed_fields` function compares its fields **in the exact order
they are listed above** and pushes each field's own name (the Rust field
identifier as a string literal, e.g. `"name"`, `"multicast_ttl"`,
`"module_instance"`) when `left.<field> != right.<field>`. This fixes
`changed_fields`'s order deterministically without needing a separate sort
step in Task 3.

- [ ] **Step 1: Write `testutil.rs` first.**

  Small builders, in the style of `crates/knx-report/src/testutil.rs`: an
  empty `Project` (`knx_core::Project::new(knx_core::Language("en".into()))`
  plus one pushed `Installation`), and helpers to add an `Area`, a `Line`
  under a given area, a `GroupRange`, a `GroupAddressEntry` inside a given
  range, a `BuildingPart` under a given parent, a `DeviceInstance` on a
  given line (or unassigned), and a `ComObjectInstance`/`ParameterInstance`
  on a given device. Hand-built values only — no corpus, no import.

- [ ] **Step 2: Write the failing `semantic.rs` unit tests.**

  Over `testutil.rs`'s builders:

  - `area_key_uses_the_raw_address`, `line_key_resolves_via_topology_area_of`
    (a line under area `1` with its own address `2` yields
    `LineKey { area_address: 1, line_address: 2 }`), and the equivalent one
    or two tests for every other `*_key` function — each asserting the exact
    field values, not just "is Some".
  - `override_value_at_program_layer_resolves_to_none_in_com_object_fields`:
    an `Override::Value(Resolved { layer: Layer::Program, .. })` text/dpt
    both resolve to `None` — the same rule `compare.rs`'s own tests already
    lock for its types.
  - `override_value_at_instance_or_user_edit_layer_resolves_to_some`: the
    same override at `Layer::Instance` and separately at `Layer::UserEdit`
    both resolve to `Some(..)` with the expected text/dpt string.
  - `override_empty_resolves_to_some_empty_string_for_text_and_dpt_but_none_for_flags`:
    pins the three-way split between `semantic_text`/`semantic_dpt` (Empty
    -> `Some(String::new())`) and `semantic_flag` (Empty -> `None`).
  - `override_malformed_resolves_to_the_raw_string`: `Malformed("garbage
    ets export".into())` resolves to `Some("garbage ets export".into())` for
    both text and dpt.
  - `device_fields_excludes_com_objects_and_parameters`: assert (by
    construction — `DeviceFields` has no such fields, so this is really a
    compile-time property) that `device_fields` returns exactly the eight
    fields listed above; write it as a test that constructs one and matches
    on every field, so a future accidental field addition to `DeviceFields`
    that duplicates `EntityTable` data breaks a named test, not just a
    reviewer's memory of this plan.
  - `com_object_links_are_sorted_by_group_address_key_then_direction`: two
    links to the same group address, one `Send` one `Receive`, plus a link
    to a different, higher-numbered group address — asserts the exact
    output order.
  - one `*_changed_fields_reports_every_differing_field_in_order` test per
    entity type (8 tests total, one per `*Fields` struct above): build a
    `left`/`right` pair differing in two non-adjacent fields (e.g. for
    `LineFields`, `name` and `multicast_ttl`) and assert
    `changed_fields == vec!["name", "multicast_ttl"]` — exact `Vec`
    equality, not just "contains".

- [ ] **Step 3: Run the tests and watch them fail.**

  Run: `cargo test -p knx-diff`

- [ ] **Step 4: Implement `semantic.rs`.**

  `line_of_device` scans `topology.lines` for the one line whose `devices`
  contains the given id; if none does, the device is presumed
  `Topology::unassigned` and the function returns `None`. `building_part_path`
  walks `parent` through `by_id` to the root, collecting names, then
  reverses so the result reads root-first (`["Building A", "Floor 1", "Room
  3"]`) — the same technique `compare.rs:429-459`'s `semantic_building_part`
  already uses for one parent hop, extended here to the full chain.

  Resolve `GroupAddress`/`IndividualAddress` formatting through their own
  `Display`/`format` methods (`address.rs`) — never hand-rolled `x/y/z`
  string building.

- [ ] **Step 5: Run the focused checks.**

  Run: `cargo test -p knx-diff && cargo fmt --all --check && cargo clippy -p knx-diff --all-targets -- -D warnings`

- [ ] **Step 6: Commit `semantic.rs`.**

  Run: `git add crates/knx-diff && git commit`

### 3: `diff.rs` — `diff_projects`, device diffing, and the correctness tests

**Files:**
- Create: `crates/knx-diff/src/diff.rs`
- Modify: `crates/knx-diff/src/lib.rs`

**Interfaces:**
- Produces the crate's full public surface (re-exported from `lib.rs`):
  ```rust
  pub fn diff_projects(left: &knx_core::Project, right: &knx_core::Project) -> ProjectDiff;

  pub struct FieldChange { pub field: &'static str, pub left: String, pub right: String }

  pub struct ProjectDiff {
      pub info_changes: Vec<FieldChange>,
      pub installations: Vec<InstallationDiff>,
  }

  pub struct InstallationDiff {
      pub id: u8,
      pub status: EntityStatus,
      pub field_changes: Vec<FieldChange>, // only populated when status == Matched
      pub areas: EntityTable<AreaKey, AreaFields>,
      pub lines: EntityTable<LineKey, LineFields>,
      pub devices: DeviceTable,
      pub group_ranges: EntityTable<GroupRangeKey, GroupRangeFields>,
      pub group_addresses: EntityTable<GroupAddressKey, GroupAddressFields>,
      pub buildings: EntityTable<BuildingPartKey, BuildingPartFields>,
  }

  // Per the orchestrator's ruling: devices get dedicated types instead of
  // reusing `EntityTable<DeviceKey, DeviceFields>` — nesting `com_objects`/
  // `parameters` inside a `Fields` type used on both `left` and `right` of
  // a generic `EntityChange` would duplicate the nested diff meaninglessly
  // on both sides.
  pub struct DeviceTable {
      pub added: Vec<(DeviceKey, DeviceFields)>,
      pub removed: Vec<(DeviceKey, DeviceFields)>,
      pub changed: Vec<DeviceChange>,
      pub ambiguous: Vec<AmbiguityNote<DeviceKey>>,
  }

  pub struct DeviceChange {
      pub key: DeviceKey,
      pub matched_by: MatchKind,
      pub left: DeviceFields,
      pub right: DeviceFields,
      pub changed_fields: Vec<&'static str>,
      pub com_objects: EntityTable<ComObjectKey, ComObjectFields>,
      pub parameters: EntityTable<ParameterKey, ParameterFields>,
  }
  ```
  All of the above derive `Debug, Clone, PartialEq` (`ProjectDiff`
  transitively, since every field type does — this is what makes the
  determinism test below a one-line `assert_eq!`).

  `lib.rs` re-exports every `semantic::*Key`/`*Fields` type (Task 2) and
  every `diff::*` type as `knx_diff::X` — flat, no `knx_diff::key::X` /
  `knx_diff::semantic::X` nesting in the public surface, matching
  `knx-report`'s own flat re-export shape:
  ```rust
  mod diff;
  mod key;
  mod semantic;
  pub use diff::*;
  pub use key::{AmbiguityNote, EntityChange, EntityStatus, EntityTable, MatchKind};
  pub use semantic::*;
  #[cfg(test)]
  mod testutil;
  ```
- Consumes: Task 1's `match_entities`, `MatchOutcome`, `EntityTable`,
  `EntityChange`, `AmbiguityNote`, `EntityStatus`, `MatchKind`; Task 2's
  every `*Key`/`*Fields` type and extraction function.

**Two rulings this task must implement, not the spec's literal §6 example
(the spec and this plan's Task 2 section already state the type shapes;
this is the behavior, stated again here because this is where it is
implemented):**

1. A `DeviceChange` is emitted when `changed_fields` is non-empty **or**
   either nested table (`com_objects`, `parameters`) is non-empty — not only
   when `changed_fields` is non-empty. A device whose own fields are
   identical but which has one changed communication object must still
   appear in `changed`.
2. An added or removed device's `com_objects`/`parameters` are **not**
   diffed individually — the device's own add/remove already accounts for
   them. Do not populate `com_objects`/`parameters` for `DeviceTable::added`/
   `removed` entries at all; those entries are just `(DeviceKey,
   DeviceFields)` pairs, matching every other entity table's added/removed
   shape.

**How devices, communication objects and parameters are gathered per
installation** (the domain model does not scope `Devices`/
`Installation::parameters` by installation directly — this mirrors
`crates/knx-etsproj/src/compare.rs:186-192`'s own precedent exactly):
a device "belongs" to an installation if its id appears in any of that
installation's `topology.lines[*].devices` or in
`topology.unassigned`. Gather that id list, then `project.devices.get(id)`
for each. **Parameters are bucketed by `device` id from
`Installation::parameters` before matching within a matched device pair —
they are never walked from the device struct**
(`crates/knx-core/src/parameter.rs:9-18`): build a
`HashMap<DeviceId, Vec<&ParameterInstance>>` once per installation by
grouping `installation.parameters`, and look each matched device pair's two
sides up in it. Communication objects, unlike parameters, **are** walked
from the device struct: `device.com_objects` names the
`ComObjectInstanceId`s that belong to it; resolve each through
`project.devices.com_object(id)`.

**Ordering (design spec §4):** every `added`/`removed`/`changed`/
`ambiguous` `Vec` is sorted by its own key's `Ord` before being placed into
the returned structs — `AreaKey`, `LineKey`, `GroupRangeKey`,
`GroupAddressKey`, `BuildingPartKey`, `ComObjectKey`, `ParameterKey` and
`DeviceKey` all derive `Ord` (Task 2) for exactly this. Never rely on
`match_entities`'s own internal iteration order (it is built over `HashMap`
lookups internally and makes no ordering promise).

- [ ] **Step 1: Write the failing `diff_projects` unit tests.**

  Using `testutil.rs` (Task 2) and hand-built `Project`s built directly (not
  through `testutil.rs`'s single-installation helper, where a test needs two
  independent `Project` values to compare):

  - `diff_of_a_project_against_itself_is_empty_at_every_level` — design
    spec §3.7, the design's own correctness anchor. Build one `Project`
    containing at least one of every entity type this design covers
    (an area, a line, a device with an individual address, a device without
    one, a group range, a group address inside it, a building part, a
    module-based device — schema ≥ 21 shape, i.e. one with a
    `module_instance` set on a com object — and a monolithic one, i.e. one
    without). Assert `diff_projects(&p, &p)` has an empty `info_changes`,
    and for the one installation: `status == EntityStatus::Matched`, empty
    `field_changes`, and every `added`/`removed`/`changed`/`ambiguous` list
    empty on `areas`, `lines`, `devices` (`DeviceTable`'s four lists),
    `group_ranges`, `group_addresses` and `buildings`.
  - `running_diff_projects_twice_on_the_same_inputs_produces_equal_results`
    — the determinism test (design spec §4): call `diff_projects(&left,
    &right)` twice on the same two non-trivial `Project`s (reuse the
    project from the previous test plus a second, edited clone) and
    `assert_eq!` the two `ProjectDiff` values.
  - `a_device_with_unchanged_own_fields_but_a_changed_com_object_dpt_still_appears_in_changed`
    — pins ruling 1 exactly as the brief names it: clone a project
    containing one device with one communication object, change only that
    object's `dpt` on the clone (`Command::SetComObjectDpt`, see Step 4
    below), diff the two, and assert: the device appears in
    `devices.changed` (not silently dropped), its `DeviceChange.changed_fields`
    is **empty** (its own fields did not change), and its `com_objects`
    table has exactly one entry in `changed` naming `"dpt"` in that entry's
    `changed_fields`.
  - `an_added_or_removed_devices_nested_tables_are_empty` — pins ruling 2:
    a device present only on the right side (added) has
    `com_objects.added.is_empty()`, `com_objects.removed.is_empty()`,
    `com_objects.changed.is_empty()` and the same for `parameters`, even
    though the device itself carries communication objects and a parameter
    in the fixture.
  - `an_ambiguous_natural_key_never_appears_in_changed` — two devices on
    each side sharing no `ets_id` but colliding on the same individual
    address: both land in `added`/`removed` (not `changed`), and one
    `AmbiguityNote` appears in `devices.ambiguous` with `left_candidates:
    2, right_candidates: 2` (or whatever counts the fixture actually
    produces — assert the real numbers, not a placeholder).
  - `installation_added_and_removed_carry_empty_nested_tables` — a project
    with two installations on the right and one (a subset) on the left: the
    added installation's `areas`/`lines`/`devices`/etc. are all empty,
    following the same "the container's own add/remove accounts for its
    contents" principle as ruling 2, applied one level up (state this
    explicitly in a comment above the implementation, since the spec never
    names this case directly — it is this plan's own extension of ruling 2
    by analogy, not a literal spec requirement).
  - **A genuine two-project test**, `every_entity_kinds_own_change_is_reported_exactly_once_in_the_right_table_with_the_right_changed_fields`
    (design spec §8's "genuine two-project test"): hand-build `left`, clone
    it to make `right`, then apply exactly one change per entity kind to
    `right` — using `knx_core::Command` where one exists for that field, a
    direct struct mutation where it does not:
    - Device description: `Command::SetDeviceDescription { device,
      description: Some("new description".into()) }.apply(&mut right)`.
    - Device `product_ref`: **no `Command` exists** — mutate
      `right.devices.get_mut(device_id).unwrap().product_ref` directly.
    - Communication object DPT: `Command::SetComObjectDpt { com_object,
      dpt: Some(DptRef { main: 1, sub: Some(1) }) }.apply(&mut right)`.
    - Group address rename: `Command::UpdateGroupAddress { id, name:
      "renamed".into(), central: entry.central, unfiltered:
      entry.unfiltered }.apply(&mut right)`.
    - Group range rename: `Command::RenameGroupRange { id, name:
      "renamed".into() }.apply(&mut right)`.
    - Building part rename: `Command::RenameBuildingPart { id, name:
      "renamed".into() }.apply(&mut right)`.
    - Area/line: **no rename `Command` exists for either** — mutate
      `right.installations[0].topology.areas[..].name` /
      `.lines[..].name` directly.
    - Parameter value: **no `Command` exists** — mutate
      `right.installations[0].parameters[..].raw` directly.

    `Command::apply(&self, project: &mut Project) -> Result<Command,
    CommandError>` (`crates/knx-core/src/command.rs:405`) — `.unwrap()` the
    `Result` in the test; a `CommandError` here is a fixture bug, not
    something the test needs to handle gracefully.

    Diff `left` against `right` and assert, for **each** of the eight
    changes above: it appears in exactly the right `EntityTable`/
    `DeviceTable`'s `changed` list (never in `added`+`removed`, never
    missing), and that entry's `changed_fields` names exactly the one field
    that changed (e.g. `vec!["description"]` for the device description
    change, `vec!["product_ref"]` for the direct mutation, `vec!["dpt"]`
    for the communication object). Also assert every *other* entity's
    tables are empty — proof the diff did not report anything beyond the
    eight intentional changes.

- [ ] **Step 2: Run the tests and watch them fail.**

  Run: `cargo test -p knx-diff`

- [ ] **Step 3: Implement `diff_projects` and the device-specific logic.**

  `info_changes`: compare `left.info`/`right.info`'s `name`,
  `project_number`, `group_address_style`, `completion` (never
  `project_id` — comparing two files that are not the same container is the
  entire point of a diff). Every `FieldChange.left`/`.right` is
  `format!("{value:?}")` of the field's own value — a uniform, unambiguous
  representation across eight structurally different field types, rather
  than a bespoke `Display` for each. (This applies to `FieldChange` wherever
  it is produced in this task: `info_changes` and every `InstallationDiff`'s
  `field_changes`.)

  Installations: build `BTreeMap<u8, &Installation>` for each side (using
  the `InstallationId`'s inner `u8`); walk the union of both sides' ids in
  order. An id on only one side is `EntityStatus::Added`/`Removed` with
  every nested table empty (this task's own extension of ruling 2, see the
  test above). An id on both sides is `EntityStatus::Matched`:
  `field_changes` compares `name`, `multicast_address`, `completion`,
  `default_line` (resolved to `Option<LineKey>` via `topology.line(id)` +
  `line_key`); then build every nested `EntityTable`/`DeviceTable` as
  described below.

  Areas/lines/group ranges/group addresses/building parts: call
  `match_entities` with each entity's `ets_id` accessor
  (`|e| e.source.ets_id.as_str()`) and Task 2's `*_key` function as the
  natural-key accessor (already `Option<NK>`-shaped); for each match/leftover
  in the `MatchOutcome`, build `(Key, Fields)`/`EntityChange` using Task 2's
  `*_fields`/`*_changed_fields` functions, then sort every resulting `Vec` by
  key.

  Devices: gather the installation's device ids as described above, run
  `match_entities` the same way (`natural_key` returns
  `device.address.map(|a| a.to_string())`, `None` when the device has no
  individual address — per design spec §3.4, "only when `Some`"). For each
  matched pair, build `DeviceFields` for both sides (resolving `line`/
  `building` via the per-installation reverse indices — build the
  `HashMap<DeviceId, BuildingPartId>` once per installation from
  `installation.buildings[*].devices`, and call `line_of_device` per
  device), compute `changed_fields`, then build `com_objects`/`parameters`
  by running `match_entities` again — scoped to just this one matched
  device pair's own communication objects/parameters (gathered as this
  task's header section describes) — and emit the `DeviceChange` only if
  ruling 1's condition holds. Added/removed devices skip the nested
  `match_entities` calls entirely (ruling 2) — do not compute
  `com_objects`/`parameters` for them at all, not even as empty tables built
  the long way.

  Ambiguous entries in any `MatchOutcome` become one `AmbiguityNote` (using
  whichever `*_key` function applies, called on one representative from
  `group.left` if non-empty else `group.right` — arbitrary but consistent,
  since an `AmbiguityNote`'s `key` is informational, not a matching key
  itself) with `left_candidates: group.left.len()`, `right_candidates:
  group.right.len()`; every member of `group.left` also becomes a `removed`
  entry and every member of `group.right` an `added` entry, in the same
  pass — per design spec §3.3 step 3, an ambiguous pair is reported as
  *both* individual adds/removes *and* one ambiguity note, never only one or
  the other.

- [ ] **Step 4: Run the crate's whole suite plus the workspace gates.**

  Run: `cargo test -p knx-diff && cargo fmt --all --check && cargo clippy -p knx-diff --all-targets -- -D warnings && cargo run -p xtask -- check-layering && cargo deny check`

- [ ] **Step 5: Commit `diff.rs` and the crate's public surface.**

  Run: `git add crates/knx-diff && git commit`

### 4: Server route and DTOs

**Files:**
- Modify: `apps/knx-server/Cargo.toml`
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Create: `apps/knx-server/tests/http_project_diff.rs`

**Interfaces:**
- Produces: `POST /api/project/diff { path }` → a `ProjectDiffDto` JSON body
  (full shape below).
- Produces: `domain::diff_project_impl(state: &AppState, path: &Path) ->
  Result<knx_diff::ProjectDiff, String>`.
- Consumes: Task 3's `diff_projects` and every `knx_diff::*` public type.

**What the two sides are (design spec §7): the server's live, possibly
edited, in-memory project (`left`) against a `.knxdb` file at `path`
(`right`) — not a re-read of `store_path`.** This is deliberate: "what would
Save change", not "compare two files". `path` is resolved with
`resolve_project_path` (`apps/knx-server/src/paths.rs`) — the same function
`import_project`/`open_native_project` use — **never**
`resolve_new_project_path`, which is for write targets that need not exist
yet.

**The existence-check gotcha (design spec §7), stated again because this is
where it must be fixed:** `knx_store::open_and_migrate` creates an empty
SQLite file if `path` does not exist. `resolve_project_path` does **not**
check existence for an absolute path (only relative, `data_dir`-confined
paths go through `canonicalize`, which does). `diff_project_impl` must
check `path.exists()` itself, immediately after resolving, and fail with a
clear message — a typo'd comparison path must never silently become "every
entity in the open project reports as removed".

**Error classification, a call this plan makes because the spec leaves it
open:** unlike `import_project`/`open_native_project` (which read the
*only* project a route establishes, so any failure is an environment
problem → `ApiError::internal`, per `errors.rs`'s own documented 400/500
split), every failure mode here is the caller's to fix relative to a
project that may already be open: "no project open" is an in-memory
precondition, and "comparison file does not exist" is a caller-supplied bad
path, not a corrupt database or a disk failure. Map the whole
`domain::diff_project_impl` result to `ApiError::bad_request`, matching
`export_documentation`'s own uniform-`bad_request` mapping
(`routes.rs:465-480`), not `import_project`'s `internal` mapping. State this
reasoning in a doc comment above the route handler so a future reader does
not "fix" it back to `internal` by analogy with `import_project` without
re-reading why it differs.

DTOs (`routes.rs`, next to the existing `Documentation*Dto` block): `serde`
generics let one pair of DTO types serve every entity table, since
`EntityTable<K, F>`/`EntityChange<K, F>`/`AmbiguityNote<K>` are already
generic in `knx-diff` itself:

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct EntityChangeDto<K: serde::Serialize, F: serde::Serialize> {
    key: K,
    matched_by: MatchKindDto,
    left: F,
    right: F,
    changed_fields: Vec<&'static str>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AmbiguityNoteDto<K: serde::Serialize> {
    key: K,
    left_candidates: usize,
    right_candidates: usize,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct EntityTableDto<K: serde::Serialize, F: serde::Serialize> {
    added: Vec<(K, F)>,
    removed: Vec<(K, F)>,
    changed: Vec<EntityChangeDto<K, F>>,
    ambiguous: Vec<AmbiguityNoteDto<K>>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum MatchKindDto { EtsId, NaturalKey }

impl From<&knx_diff::MatchKind> for MatchKindDto {
    fn from(k: &knx_diff::MatchKind) -> Self {
        match k {
            knx_diff::MatchKind::EtsId => Self::EtsId,
            knx_diff::MatchKind::NaturalKey => Self::NaturalKey,
        }
    }
}
```

Write one `#[derive(serde::Serialize)] #[serde(rename_all = "camelCase")]`
DTO struct per `knx_diff::*Key`/`*Fields` type from Task 2 (`AreaKeyDto`,
`AreaFieldsDto`, `LineKeyDto`, `LineFieldsDto`, ... through `ParameterKeyDto`/
`ParameterFieldsDto` — 16 small structs, field-for-field copies with
`camelCase` serialization, exactly the "hand-written, matching
`#[serde(rename_all = "camelCase")]`" convention `apps/knx-web/src/api.ts`'s
own comments already describe for the Rust→TS side), each with a `From<&
knx_diff::X>` conversion. `EntityTableDto<AreaKeyDto, AreaFieldsDto>` etc.
are then type aliases (`type AreaTableDto = EntityTableDto<AreaKeyDto,
AreaFieldsDto>;`) for the six generic tables; `DeviceTableDto`/
`DeviceChangeDto` are bespoke, non-generic structs mirroring `DeviceTable`/
`DeviceChange` field-for-field (same reasoning as `knx-diff` itself: the two
nested tables do not fit the generic shape). `FieldChangeDto { field:
&'static str, left: String, right: String }`,
`ProjectDiffDto { info_changes: Vec<FieldChangeDto>, installations:
Vec<InstallationDiffDto> }`, `InstallationDiffDto` mirroring
`InstallationDiff` field-for-field, `EntityStatusDto` mirroring
`EntityStatus`.

- [ ] **Step 1: Write the failing route tests.**

  In `http_project_diff.rs`, following `http_documentation_export.rs`'s
  pattern (build small projects directly, write one to a `.knxdb` fixture
  with `knx_store::open_and_migrate`/`knx_store::save_project`, hold the
  other as the server's live project via the same state-construction helper
  `http_documentation_export.rs` uses):

  - a live project identical to the comparison file produces a response
    whose `installations` array has every table empty and `infoChanges`
    empty;
  - a live project with one device description changed relative to the
    comparison file produces a response naming that change (assert the
    JSON body contains the device's changed `description` value, and that
    the corresponding table's `changed` array has exactly one entry with
    `changedFields: ["description"]`);
  - a comparison path that does not exist is rejected with a 400, and — this
    is the existence-check-gotcha regression test — no new `.knxdb` file
    appears at that path afterward (`assert!(!path.exists())`);
  - calling the route with no project open is a 400, not a 500 and not a
    panic.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cargo test -p knx-server --test http_project_diff`

- [ ] **Step 3: Add the dependency and implement.**

  Add `knx-diff.workspace = true` to `apps/knx-server/Cargo.toml`'s
  `[dependencies]`, alongside the existing `knx-report`/`knx-csv` lines.

  `domain::diff_project_impl` follows `export_documentation_impl`'s shape
  (`domain.rs:544-573`): lock `state.project`, resolve/verify `path`, open
  and load the comparison project, call `knx_diff::diff_projects(left,
  &right)`, return the `ProjectDiff`. Nothing here needs the session log —
  a diff mutates nothing and produces no warnings in the `ReportWarning`
  sense (an `AmbiguityNote` is data in the response, not a warning to log
  separately; there is no established convention in this codebase for
  logging read-only comparison output, and inventing one is out of scope for
  this task).

  The route follows `export_documentation` (`routes.rs:465-480`): reuse the
  existing `PathBody` request struct, convert every `knx_diff::*` value to
  its DTO at the boundary, `.map_err(ApiError::bad_request)` per the
  reasoning stated above.

  Register `POST /api/project/diff` in `project_routes()`.

- [ ] **Step 4: Run the focused tests.**

  Run: `cargo test -p knx-server`

- [ ] **Step 5: Commit the HTTP surface.**

  Run: `git add apps/knx-server && git commit`

### 5: CLI subcommand

**Files:**
- Modify: `apps/knx-cli/Cargo.toml`
- Modify: `apps/knx-cli/src/main.rs`
- Create: `apps/knx-cli/tests/cli_project_diff.rs`

**Interfaces:**
- Produces: `knx diff <a.knxdb> <b.knxdb>` — plain text on stdout, exit `0`
  whenever a comparison was successfully produced (a diff with changes is
  not a failed diff, mirroring `run_doc_export`'s reasoning), exit `1` when
  either store could not be opened or loaded.
- Consumes: Task 3's `diff_projects` and every `knx_diff::*` type.

This task shares no file with Task 4 or Task 6 and may run alongside
either.

- [ ] **Step 1: Write the failing CLI tests.**

  Following `cli_documentation_export.rs`'s pattern — drive the built `knx`
  binary with `std::process::Command` against two tiny `.knxdb` files built
  directly through `knx-core`/`knx-store`:

  - `diff_of_two_identical_stores_reports_no_changes`: exit code `0`, stdout
    contains a line stating no differences were found (e.g. "no
    differences"), and contains neither a `+` nor a `-` nor a `~` prefixed
    change line.
  - `diff_of_two_stores_with_one_changed_group_address_name_prints_it`: exit
    code `0`, stdout contains the group address's formatted address, both
    the old and new name, and a `~` prefix.
  - `diff_with_a_missing_first_store_exits_one_and_prints_no_stray_knxdb`:
    the first argument names a nonexistent path; exit code `1`; assert the
    nonexistent path still does not exist after the run (the existence-check
    regression test for the CLI's own new call site, mirroring Task 4's).
  - `diff_with_wrong_argument_count_prints_usage_and_exits_one`.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cargo test -p knx-cli --test cli_project_diff`

- [ ] **Step 3: Add the dependency and implement the subcommand.**

  Add `knx-diff.workspace = true` to `apps/knx-cli/Cargo.toml`'s
  `[dependencies]`, alongside the existing `knx-report`/`knx-csv` lines.

  Add the line `knx diff <a.knxdb> <b.knxdb>` to the `USAGE` const,
  immediately after the `doc-export` line, matching its indentation exactly
  (`\x20     knx diff <a.knxdb> <b.knxdb>\n`).

  `struct DiffArgs { a: String, b: String }`, `fn parse_diff_args(args:
  &[String]) -> Result<DiffArgs, String>` following
  `parse_doc_export_args`'s shape (positional-only, `--` prefix rejected as
  an unknown flag).

  `fn run_diff(args: &[String]) -> ExitCode`: parse args; **check
  `Path::new(&parsed.a).exists()` and `Path::new(&parsed.b).exists()`
  explicitly before calling `knx_store::open_and_migrate` for either** —
  this is the CLI's own new call site for the existence-check gotcha design
  spec §7 names; print `"store not found: {path}"` and exit `1` on either
  missing file, without creating anything. Then open/load both stores
  exactly like `run_doc_export` does (twice), call
  `knx_diff::diff_projects(&project_a, &project_b)`, and print the result
  via `print_diff_report`.

  `fn print_diff_report(diff: &knx_diff::ProjectDiff)`: print `"no
  differences found"` if `diff.info_changes.is_empty() &&
  diff.installations.iter().all(|i| /* every table on it is empty and
  field_changes is empty and status is Matched */)`. Otherwise, print one
  line per `FieldChange` in `info_changes` (`"~ project info: {field}:
  {left} -> {right}"`), then per installation: its `EntityStatus` if not
  `Matched` (`"+ installation {id}"` / `"- installation {id}"`), its own
  `field_changes` the same way, then one block per entity table — `"+ area
  {address}"` / `"- area {address}"` for `added`/`removed`, `"~ area
  {address}: {changed_fields joined by \", \"}"` for `changed`, `"? area
  {address}: {left_candidates} left candidate(s), {right_candidates} right
  candidate(s)"` for `ambiguous`. Repeat the same four-line shape for
  lines, group ranges, group addresses and building parts (using each
  entity's own key's natural, human-readable formatting — e.g. `"{area}.
  {line}"` for a `LineKey`, the joined `path` for a `BuildingPartKey`).
  Devices follow the same `+`/`-`/`~`/`?` shape using `DeviceKey`'s
  `address` (or `ets_id` when `address` is `None`) as the printed
  identifier; a `DeviceChange` additionally prints, indented two spaces,
  one line per changed communication object and parameter from its own
  nested `com_objects`/`parameters` tables — **including when
  `changed_fields` is empty and only a nested table is non-empty**: print
  `"~ device {id}: (own fields unchanged)"` in that case rather than
  nothing, per ruling 1's whole point (do not let the CLI silently drop
  exactly the information the domain layer went out of its way to keep).

- [ ] **Step 4: Run the focused tests.**

  Run: `cargo test -p knx-cli`

- [ ] **Step 5: Commit the CLI.**

  Run: `git add apps/knx-cli && git commit`

### 6: Web button and result panel

**Files:**
- Modify: `apps/knx-web/src/api.ts`
- Modify: `apps/knx-web/src/api.test.ts`
- Create: `apps/knx-web/src/ProjectDiffPanel.tsx`
- Create: `apps/knx-web/src/ProjectDiffPanel.test.tsx`
- Modify: `apps/knx-web/src/App.tsx`

**Interfaces:**
- Produces: `diffProject(path: string): Promise<ProjectDiffReport>` in
  `api.ts`, plus the hand-written `ProjectDiffReport`/`FieldChange`/
  `EntityTable<K,F>` etc. TypeScript interfaces mirroring Task 4's DTOs.
- Consumes: Task 4's route.

This task shares no file with Task 5 and may run alongside it.

- [ ] **Step 1: Write the failing API and component tests.**

  `api.test.ts`: `diffProject` posts `{ path }` to `/api/project/diff` and
  surfaces a 400 body as an error message, matching the file's existing
  conventions (same shape as the `exportDocumentation` test already there).

  `ProjectDiffPanel.test.tsx`, mocking `./api` and `./filePicker` the way
  `DocumentationExportButton.test.tsx` does:
  - a cancelled picker (`pickOpenPath` resolves `null`) calls neither the
    API nor any callback, and the panel stays closed;
  - a successful diff with every table empty renders a "no differences"
    message;
  - a successful diff with changes renders grouped counts — assert the
    rendered output contains, for a fixture with e.g. 2 changed group
    addresses and 1 added device, both a count for group addresses and a
    count for devices (the exact wording is this task's call — a
    reasonable shape is one line per non-empty entity table, e.g. "Devices:
    1 added", "Group addresses: 2 changed" — the test asserts against
    whatever wording is implemented, named precisely, not "renders
    something");
  - a rejected comparison (API throws) surfaces the error via `onError`,
    the same callback shape `DocumentationExportButton` uses;
  - the button is disabled with no project open (`tree === null`).

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cd apps/knx-web && npm test -- --run api.test.ts`

- [ ] **Step 3: Implement the API helper and the component.**

  `api.ts` gains hand-written interfaces mirroring Task 4's DTOs — at
  minimum `FieldChange { field: string; left: string; right: string }`,
  a generic `EntityTable<K, F> { added: [K, F][]; removed: [K, F][];
  changed: EntityChange<K, F>[]; ambiguous: AmbiguityNote<K>[] }`,
  `EntityChange<K, F> { key: K; matchedBy: "etsId" | "naturalKey"; left: F;
  right: F; changedFields: string[] }`, `AmbiguityNote<K> { key: K;
  leftCandidates: number; rightCandidates: number }`, one small `*Key`/
  `*Fields` interface per entity type (following `CsvProblem`'s comment
  convention: cite the Rust struct each mirrors), `DeviceTable`/
  `DeviceChange` as their own non-generic interfaces, and
  `ProjectDiffReport { infoChanges: FieldChange[]; installations:
  InstallationDiff[] }`. `diffProject` follows `exportDocumentation`'s
  one-line shape:
  ```ts
  export function diffProject(path: string): Promise<ProjectDiffReport> {
    return request("/api/project/diff", {
      method: "POST",
      body: JSON.stringify({ path }),
    });
  }
  ```

  `ProjectDiffPanel.tsx` follows `DocumentationExportButton.tsx`'s
  ownership shape (owns its `./api`/`./filePicker` calls, takes `{ tree,
  onError, onClearErrors }` props) but, unlike that button, needs its own
  local `useState` for the fetched `ProjectDiffReport` and an open/closed
  toggle, since the result is a panel of grouped counts to render, not a
  one-line toast — closer in spirit to `LogPanel.tsx`'s self-contained
  render, but triggered by a user-picked file rather than auto-fetching on
  mount. **Uses `pickOpenPath`, not `pickSavePath`** — the user is picking
  an *existing* `.knxdb` file to compare against, not a new write target
  (`DocumentationExportButton.tsx`'s own `pickSavePath` would be the wrong
  primitive here; do not copy it verbatim). Filter:
  `[{ name: "KNXBench project", extensions: ["knxdb"] }]`. The button
  label is "Compare with…". Never "Compare with ETS export" or similar.

  Render, for each non-empty entity table across every installation in the
  response, one summary line naming the table and its `added.length`/
  `removed.length`/`changed.length`/`ambiguous.length` counts (whichever
  are non-zero) — the "grouped counts" design spec §8 asks for; no tree
  view, no inline before/after highlighting (design spec §9 names both as
  explicitly out of scope).

  Render it in `App.tsx`'s existing toolbar row, next to
  `DocumentationExportButton` — no new top-level view, matching how every
  other export/compare-adjacent control already lives in that row.

- [ ] **Step 4: Run the frontend checks.**

  Run: `cd apps/knx-web && npx tsc --noEmit && npm test -- --run && npm run build`

  Restore `apps/knx-web/dist/.gitkeep` if the build removed it:
  `git checkout -- apps/knx-web/dist/.gitkeep`.

- [ ] **Step 5: Commit the UI.**

  Run: `git add apps/knx-web && git commit`

### 7: Corpus proof and documentation

**Files:**
- Create: `crates/knx-app/tests/project_diff.rs`
- Modify: `crates/knx-app/Cargo.toml` (dev-dependency on `knx-diff`)
- Modify: `docs/GAP_ANALYSIS_ETS.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/ROADMAP.md` (only if it names T14 or C1)

**Interfaces:**
- Consumes: Task 3's `diff_projects` (imported via `knx_diff`, a new
  dev-dependency of `knx-app`), `knx_etsproj::import_knxproj` (already a
  dev-dependency of `knx-app`, per `csv_roundtrip.rs`/
  `documentation_export.rs`).

- [ ] **Step 1: Write the corpus-gated integration test.**

  It goes in `crates/knx-app/tests/`, **not** in `crates/knx-diff/tests/`:
  `check-layering` walks dev-dependency edges too, so a `knx-etsproj`
  dev-dependency on `knx-diff` would trip `knx-diff`'s own layering rule
  (Task 1). `knx-app` is the one crate deliberately permitted to see both
  sides — copy `crates/knx-app/tests/documentation_export.rs`'s shape,
  including the identical 4-line `reference_ets4_path()` helper (a crate's
  integration tests cannot reach another crate's `tests/` module, so this
  is independently repeated, the same way `csv_roundtrip.rs` and
  `documentation_export.rs` each repeat it) and the skip guard from Global
  Constraints above.

  `rendering_diff_projects_between_two_independent_imports_of_the_reference_project_is_empty`:
  import `OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj`
  **twice**, independently, into two `Project`s (`p1`, `p2`), call
  `knx_diff::diff_projects(&p1, &p2)`, and assert `info_changes` is empty
  and every installation's every table is empty at every level —
  design spec §3.7's property, run against real, large, ETS-shaped data
  (36 devices, 907 communication objects, 514 group addresses) instead of a
  hand-built fixture. `eprintln!` the device/communication-object/group-
  address counts before asserting, the same "proof the corpus path actually
  ran" pattern `documentation_export.rs` uses.

  Then confirm `cargo run -p xtask -- check-layering` still passes — verify
  it, do not assume it.

- [ ] **Step 2: Close C1 and the T14 backlog entry with evidence, not
  adjectives.**

  In `GAP_ANALYSIS_ETS.md`: rewrite the C1 row and close the T14 backlog
  entry the way T11/T12/T13 were closed — naming the crate, the route, the
  CLI subcommand, the button and the test file names. Every count you write
  down must be re-derived from the code at that moment (`cargo test -p
  knx-diff -- --list`, `grep -c`), never copied from this plan or from a
  subagent's report. State plainly what the diff does and does not compare
  (§9's out-of-scope list: no merge/apply, no three-way, no version history,
  no re-import `RefId`-regeneration detection, no ETS-comparison parity
  claim) rather than a bare "Closed".

- [ ] **Step 3: Record the limitations honestly.**

  New `KNOWN_LIMITATIONS.md` entries, starting at `## 51.`, in the file's
  existing `**Limitation.** / **Cause.** / **Impact.** / **Lifted when.**`
  style, for at least: no ETS-comparison parity and no way to measure one
  (cross-reference §44, the T13 precedent); a device with no individual
  address and no matching `ets_id` cannot be correlated across two projects
  and surfaces as an unrelated add+remove (design spec §3.4/§9); two
  sibling building parts sharing a name under the same matched parent
  collide under the path-based key and trigger the ambiguity path (design
  spec §3.4); a re-imported `.knxproj` whose `RefId`s ETS regenerated is not
  detected as "the same project" (design spec §3.2/§9); no merge/apply of a
  diff back onto a project; no three-way comparison; comparing against a
  raw `.knxproj` is not supported, only two `.knxdb` files (CLI) or the open
  project against one `.knxdb` file (server/web); no CI-friendly "exit
  nonzero on any difference" CLI flag.

- [ ] **Step 4: Update the architecture drawing.**

  In `ARCHITECTURE.md`: add `knx-diff` to the `crates/` listing (one line,
  matching the existing entries' style — pure, `knx-core`-only, computes the
  project comparison) and to both dependency-graph diagrams in §3, as a
  dependency of `knx-server` and of `knx-cli` (alongside `knx-report`).
  **Note in the same edit, as a comment or a short aside, that `knx-csv` and
  `knx-report` are not currently drawn in this file either** — a pre-existing
  inconsistency from T12/T13, not something this task retroactively fixes,
  but worth flagging rather than silently adding a third undocumented
  crate's siblings' omissions go unmentioned.

- [ ] **Step 5: Run every gate.**

  Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo run -p xtask -- check-layering && cargo deny check && cd apps/knx-web && npx tsc --noEmit && npm test -- --run && npm run build`

  Restore `apps/knx-web/dist/.gitkeep` afterwards:
  `git checkout -- apps/knx-web/dist/.gitkeep`.

- [ ] **Step 6: Commit the proof and the documentation.**

  Run: `git add crates/knx-app docs && git commit`

## Plan self-review

- **Spec coverage:** Task 1 the matching engine (§3.3) and the layering rule
  (§6); Task 2 the natural keys, field lists and `Override` resolution
  (§3.4, §3.5, §3.6); Task 3 `diff_projects` itself, the device rulings, the
  ordering discipline (§3.1-§3.4 tie-together, §4) and every correctness
  test named in §8 except the corpus one; Task 4 the server surface (§7);
  Task 5 the CLI (§5, §7); Task 6 the web panel (§5, §8); Task 7 the corpus
  proof (§8) and the documentation (§1, §9).
- **Ordering:** 1 → 2 → 3 (each compiles and tests standalone: 1 needs
  nothing, 2 needs the crate to exist but not 1's `match_entities`, 3 needs
  both). 4, 5 and 6 each need 3 only, touch disjoint files, and may run in
  parallel. 7 needs everything.
- **File conflicts:** `crates/knx-diff/src/lib.rs` is touched by Tasks 1, 2
  and 3 — they run in sequence, never in parallel. `Cargo.toml` (root) is
  touched only by Task 1. No other file appears in two tasks.
- **Type consistency across the Task 2 / Task 3 boundary:** Task 2 produces
  every `*Key`/`*Fields` type and every `*_key`/`*_fields`/`*_changed_fields`
  function; Task 3 is their only consumer, via `EntityTable<K, F>`/
  `DeviceChange`'s fields. `DeviceFields` is used both generically (as
  Task 1's `EntityChange<K, F>`'s `F` would have been, had ruling 1 not
  applied) and directly inside the bespoke `DeviceChange`/`DeviceTable` —
  Task 3's header section states explicitly that `DeviceTable`/
  `DeviceChange` replace `EntityTable<DeviceKey, DeviceFields>` inside
  `InstallationDiff`, so an implementer reading only Task 3 never sees the
  generic form as a live possibility to (re)introduce.
- **Type consistency across the Task 3 / Task 4 boundary:** Task 3 produces
  `ProjectDiff` and every nested type, none `Serialize` (matching
  `knx-report`'s own no-`serde` precedent). Task 4 consumes every one of
  them via hand-written DTOs, generic over `K`/`F` where `knx-diff`'s own
  types are generic (`EntityTable`/`EntityChange`/`AmbiguityNote`) and
  bespoke where `knx-diff`'s are (`DeviceTable`/`DeviceChange`,
  `ProjectDiff`/`InstallationDiff`). This mirrors the crate boundary
  exactly, so the DTO layer cannot silently diverge in shape from what
  Task 3 actually produces.
- **Type consistency across the Task 4 / Task 6 boundary:** Task 4's DTOs
  use `#[serde(rename_all = "camelCase")]` throughout; Task 6's hand-written
  TypeScript interfaces are named to match field-for-field
  (`changedFields`, `leftCandidates`, `matchedBy: "etsId" | "naturalKey"`).
  Task 6's own step 3 states the interface list explicitly rather than
  leaving "mirror the DTOs" implicit, so a Task 6 implementer who never
  reads Task 4's text still has the exact shape.
- **A call the spec left open, restated:** `FieldChange` is used
  (`info_changes`, `InstallationDiff.field_changes`) but never defined in
  the design spec's §6 sketch. This plan defines it as `{ field:
  &'static str, left: String, right: String }` (Task 3) — unlike
  `EntityChange<K, F>`, which carries `changed_fields: Vec<&'static str>`
  *and* the full `left`/`right` `F` structs separately (so it never needs
  to stringify a value into the change record itself), there is no
  parallel "project info fields" or "installation fields" struct anywhere
  in the design to hold `left`/`right` values out-of-band — `FieldChange`
  is the only place those values are carried, so it must hold them itself,
  stringified via `Debug` for a uniform representation across eight
  structurally different field types.
- **A call the spec left open, restated:** which `*Key` types carry an
  `ets_id: Option<String>` alongside their natural key. The design spec's
  §6 spells out `DeviceKey`/`GroupAddressKey` in full (both carry it) and
  says "the rest follow the field lists in §3.4 and the same generic
  shape" without spelling them out. This plan reads that literally: every
  other `*Key` type (Task 2) is exactly its bare natural-key value —
  `AreaKey { address: u8 }`, not `AreaKey { ets_id: Option<String>,
  address: u8 }` — since `EntityChange.matched_by: MatchKind` already
  states how a pair matched, and the six entity types the spec did not
  spell out have no stated need for the extra field.
- **A call the spec left open, restated:** how devices/communication
  objects/parameters are scoped to one installation, given the domain
  model does not partition `Project::devices`/`Installation::parameters`
  by installation directly. Task 3 adopts
  `crates/knx-etsproj/src/compare.rs:186-192`'s own precedent exactly
  (union of `topology.lines[*].devices` and `topology.unassigned`) rather
  than inventing a new rule — the closest sibling precedent, and one this
  design already cites for its text/DPT resolution technique, so reusing
  its device-gathering technique too keeps the two crates' notions of "this
  installation's devices" from silently diverging.
- **A call the spec left open, restated:** whether an added/removed
  *installation*'s nested entity tables stay empty, the way ruling 2
  requires for an added/removed *device*'s nested `com_objects`/
  `parameters`. The spec states ruling 2 (well, the brief's ruling) only
  for devices; Task 3 extends the same "the container's own add/remove
  accounts for its contents" reasoning one level up, by analogy, and says
  so explicitly in both the task text and a required test
  (`installation_added_and_removed_carry_empty_nested_tables`) — flagged
  here again as a plan-level decision, not a literal spec requirement.
- **Known risk:** Task 6's "grouped counts" rendering wording is this
  plan's own choice (Task 6 says so explicitly), not dictated by the spec,
  which only requires that a diff with changes "renders grouped counts" in
  some form. A reviewer who expects specific UI copy will not find it
  mandated here — the test in Task 6 asserts against whatever the
  implementer writes, named precisely once written, not a fixed string this
  plan pre-decided.
- **Known risk:** the `ARCHITECTURE.md` update in Task 7 makes `knx-diff`
  the third pure crate added to the workspace in as many tasks (`knx-csv`,
  `knx-report`, `knx-diff`) while `knx-csv`/`knx-report` remain undrawn in
  that file from their own, earlier tasks. Task 7's Step 4 requires flagging
  this in the edit itself rather than silently repeating the omission a
  third time or silently fixing two other tasks' unfinished documentation
  debt inside this one.
