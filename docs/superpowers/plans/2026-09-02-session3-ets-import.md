# Session 3 — ETS Project Import Implementation Plan

> **For agentic workers:** Executed inline by the controller in the same
> session (Agent-tool subagent dispatch is blocked by the auto-mode permission
> classifier in this repository — see the project memory
> `subagent-dispatch-blocked`). No fresh-context subagents are dispatched; the
> controller implements each task directly, runs the full gate after each task,
> and commits per task. Because the controller holds full context already,
> per-task "briefs" are the task sections below, read directly rather than
> extracted into separate files. Steps use checkbox (`- [ ]`) syntax for
> tracking.

**Goal.** Read the reference `.knxproj` into the `knx-core` model through the
six-stage pipeline of [IMPORT_EXPORT.md](../../IMPORT_EXPORT.md), preserve
everything the model cannot hold in an opaque passthrough store, report what
happened, and write the project back out.

**Architecture.** Six stages in `knx-etsproj`, each with its own error type and
no knowledge of the next: `container` → `detect` → `parse` → `validate` →
`map` → `report`. `parse` produces a `SourceDocument` whose fields are raw
strings shaped like the XML, so parsing is testable without a domain model;
`map` is the only stage that names `knx_core`, so a schema change stops there.
Export is the mirror: model plus opaque store back to a ZIP. `knx-store` gains
the opaque table at schema version 2.

**Tech Stack.** Rust 1.98.0. New third-party dependencies, all in
`knx-etsproj` and `knx-store` — never in `knx-core`:

- `zip 8.6` (MIT), `default-features = false`, `features = ["deflate-flate2"]`
  — ZIP reading and writing with the pure-Rust miniz_oxide deflate backend.
- `quick-xml 0.42` (MIT) — streaming XML reader and writer, no DOM.
- `sha2 0.10` (MIT OR Apache-2.0) — SHA-256 over opaque entries.
- `serde 1` + `serde_json 1` (MIT OR Apache-2.0) — `ImportReport` JSON export.
- `chrono` (already a workspace dependency) — ETS timestamp parsing.

**Spec:** [docs/IMPORT_EXPORT.md](../../IMPORT_EXPORT.md) (primary),
[docs/DATA_MODEL.md](../../DATA_MODEL.md),
[docs/RESEARCH.md](../../RESEARCH.md) §2–§7,
[ADR-0004](../../adr/0004-provenance-model.md),
[ADR-0006](../../adr/0006-opaque-passthrough-store.md),
[ADR-0007](../../adr/0007-roundtrip-fidelity.md).

## Global Constraints

- `knx-core` reaches none of `serde_json`, `quick-xml`, `rusqlite`, `tokio`.
  Enforced by `cargo run -p xtask -- check-layering`. New dependencies go in
  `knx-etsproj`, `knx-store` or `knx-app`, never in `knx-core`.
- No licence outside `deny.toml`'s allowlist enters the graph. Enforced by
  `cargo deny check`.
- Crate dependency direction stays `knx-app → knx-etsproj → knx-core` and
  `knx-app → knx-store → knx-core`. `knx-etsproj` must not depend on
  `knx-store`: the importer produces opaque entries as plain values, and the
  application layer is what persists them.
- Parsing streams. No stage may load a whole application-program XML file into
  a DOM; the largest single entry in the reference container is 5.7 MB and the
  container unpacks to 22 MB.
- Silent discarding is not permitted anywhere in this pipeline. Every piece of
  source information ends up in the model, in the opaque store, or in the
  report — a piece that lands in none of the three is a bug, not a report
  entry.
- Nothing derived from the product database and nothing marked
  `Layer::Inferred` is ever exported. Export decides purely on
  `Layer::is_exported()`.
- Every export is unsigned and says so. There is no code path that produces an
  export without that warning.
- Vendor binaries are copied, never loaded and never interpreted.
- Tests use the two committed reference projects at the repository root. No
  test requires network access, a KNX gateway, or Python.

**The gate, run after every task:**

```bash
cargo build --workspace \
  && cargo test --workspace \
  && cargo fmt --all --check \
  && cargo clippy --workspace --all-targets -- -D warnings \
  && cargo run -p xtask -- check-layering \
  && cargo deny check
```

## Evidence corrections found while planning this session

Two measurements in `RESEARCH.md` and `DATA_MODEL.md` are misleading, and the
model design below depends on the corrected reading. Both were re-measured
against `P-0512/0.xml` in the ETS4 reference export while writing this plan.

1. **497 of the 758 `ComObjectInstanceRef/@DatapointType` attributes are the
   empty string.** The attribute is present and carries no value. Only 261
   instances state an actual datapoint type. Likewise 82 of the 691
   `@Description` attributes are empty. "758 instances override the datapoint
   type" is therefore true only in the sense that the attribute is written; it
   is not true that 758 instances state a type. A model that collapses
   "attribute absent" and "attribute present but empty" into `None` cannot
   write the file back as it was read, which is why `Override<T>` in Task 1
   has three states rather than two.

2. **`ParameterInstanceRef/@RefId` comes in two forms.** 1174 of the 1390
   values have the shape `<app>_P-<n>_R-<n>`; the remaining 216 have
   `<app>_UP-<n>_R-<n>`, a union parameter — the instance form of the `Union`
   elements RESEARCH §4.1 counted in the application programs. A parser that
   requires `_P-` rejects 216 valid values.

Both get written into `docs/RESEARCH.md` and `docs/DATA_MODEL.md` in Task 23.

## File structure

| File | Responsibility |
| --- | --- |
| `crates/knx-core/src/provenance.rs` | *modify* — add `Override<T>`, the three-state source attribute |
| `crates/knx-core/src/string_table.rs` | *modify* — add `Text`, literal or localized |
| `crates/knx-core/src/flags.rs` | *modify* — add `ResolvedFlags`, per-flag resolution |
| `crates/knx-core/src/device.rs` | *modify* — `ComObjectInstance` per-attribute overrides; `BinaryDataRef` |
| `crates/knx-core/src/project.rs` | *modify* — `ProjectInfo` on `Project` |
| `crates/knx-core/src/topology.rs` | *modify* — `Line` medium configuration |
| `crates/knx-core/src/group.rs` | *modify* — `GroupRange` nesting |
| `crates/knx-core/src/parameter.rs` | *modify* — `ParameterInstance` owning device |
| `crates/knx-etsproj/src/container.rs` | Stage 1 — ZIP inventory, case-insensitive lookup, protection detection |
| `crates/knx-etsproj/src/detect.rs` | Stage 2 — schema version from the default namespace |
| `crates/knx-etsproj/src/source.rs` | The `SourceDocument`: source-shaped, all-string, no `knx_core` |
| `crates/knx-etsproj/src/known.rs` | Known element and attribute tables per schema version |
| `crates/knx-etsproj/src/parse/mod.rs` | Stage 3 dispatch, shared reader helpers, unknown-construct recording |
| `crates/knx-etsproj/src/parse/installation.rs` | `0.xml` reader for schema 11 |
| `crates/knx-etsproj/src/parse/project_info.rs` | `Project.xml` reader for schema 11 |
| `crates/knx-etsproj/src/values.rs` | Attribute value conversions: booleans, timestamps, RefId decomposition |
| `crates/knx-etsproj/src/validate.rs` | Stage 4 — reference resolution, duplicates, conflicts |
| `crates/knx-etsproj/src/map.rs` | Stage 5 — `SourceDocument` → `knx_core::Project` |
| `crates/knx-etsproj/src/infer.rs` | Datapoint type inference from linked objects, and DPT conflicts |
| `crates/knx-etsproj/src/opaque.rs` | `OpaqueEntry`, `OpaqueKind`, SHA-256 |
| `crates/knx-etsproj/src/report.rs` | Stage 6 — `ImportReport` and its JSON form |
| `crates/knx-etsproj/src/compare.rs` | The declared semantic-equality relation |
| `crates/knx-etsproj/src/export/mod.rs` | Container writer, unsigned warning |
| `crates/knx-etsproj/src/export/schema11.rs` | `0.xml` and `Project.xml` writers |
| `crates/knx-etsproj/src/lib.rs` | Public API: `import_knxproj`, `export_knxproj`, re-exports |
| `crates/knx-etsproj/tests/golden_reference_project.rs` | The counts from ROADMAP Session 3 |
| `crates/knx-etsproj/tests/oracle_xknxproject.rs` | Cross-check against `project_dump.json` |
| `crates/knx-etsproj/tests/roundtrip.rs` | The three fidelity guarantees |
| `crates/knx-etsproj/tests/malformed_input.rs` | Truncated, empty, non-ZIP, wrong-namespace, duplicate-id inputs |
| `crates/knx-etsproj/tests/support/mod.rs` | Shared test fixtures: reference project paths, in-memory container builders |
| `crates/knx-store/src/opaque.rs` | Opaque table access |
| `crates/knx-store/src/migration.rs` | *modify* — v1 → v2 adds the opaque table |
| `crates/knx-store/src/lib.rs` | *modify* — re-export the opaque API and `rusqlite::Connection` |
| `crates/knx-app/src/import.rs` | Import a `.knxproj` and persist its opaque entries |
| `crates/knx-app/tests/import_service.rs` | The import service persists opaque entries transactionally |
| `apps/knx-cli/src/main.rs` | *modify* — `knx import <file>` |
| `apps/knx-cli/tests/cli_import.rs` | `knx import` output, exit codes and report JSON |

## Shared test fixtures

The reference projects are already committed at the workspace root and are the
only inputs the tests use. There is no network access, no gateway, and no
Python in any test.

| Fixture | Path, relative to the workspace root |
| --- | --- |
| ETS 4.1.8 reference project, schema 11 | `Unser Zuhause ets4 - 2025-12-15.knxproj` |
| ETS 6.3.0 reference project, schema 23 | `Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj` |
| `xknxproject` oracle dump of the schema-11 project | `project_dump.json` |

The file names carry export dates and will change if the projects are
re-exported. The helper below therefore fails with a message naming the missing
file rather than with a bare `No such file or directory` from twelve call
sites.

`crates/knx-etsproj/tests/support/mod.rs` is created in Task 15, the first task
with an integration test, and every later test file starts with `mod support;
use support::*;`. Its full contents:

```rust
//! Shared fixtures for the knx-etsproj integration tests.

use std::io::Write;
use std::path::{Path, PathBuf};

/// The workspace root, two levels above this crate.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives at <root>/crates/<name>")
        .to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    let path = workspace_root().join(name);
    assert!(
        path.is_file(),
        "reference fixture {name:?} is missing from the workspace root; \
         if the reference project was re-exported, update support/mod.rs"
    );
    path
}

/// The ETS 4.1.8 reference project: schema 11, 36 devices, 514 group addresses.
pub fn reference_ets4_path() -> PathBuf {
    fixture("Unser Zuhause ets4 - 2025-12-15.knxproj")
}

/// The ETS 6.3.0 reference project: schema 23, which this session detects and
/// refuses by name rather than misparsing through the schema-11 table.
pub fn reference_ets6_path() -> PathBuf {
    fixture("Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj")
}

/// The `xknxproject` oracle dump of the schema-11 project.
pub fn oracle_dump_path() -> PathBuf {
    fixture("project_dump.json")
}

/// A ZIP archive built in memory from explicit entries, stored uncompressed so
/// the bytes on disk are the bytes given here.
pub fn zip_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// A minimal but structurally valid schema-11 installation document.
pub const MINIMAL: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="4.1.8">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="" BCUKey="4294967295">
        <Topology>
          <Area Id="P-0001-0_A-1" Address="1" Name="Area">
            <Line Id="P-0001-0_L-1" Address="1" Name="Line" MediumTypeRefId="MT-0">
              <DeviceInstance Id="P-0001-0_DI-1" Address="1" Name="Device"
                              Hardware2ProgramRefId="M-0001_H-1_HP-1"/>
            </Line>
          </Area>
        </Topology>
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" RangeStart="1" RangeEnd="2047" Name="Main">
              <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" DatapointType="DPST-1-1"/>
            </GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>
"#;

/// Wraps an installation document in a container shaped like a `.knxproj`:
/// `P-0001/0.xml`, `P-0001/Project.xml`, and the `knx_master.xml` marker.
pub fn knxproj_with_installation(installation: &[u8]) -> Vec<u8> {
    zip_with_entries(&[
        ("knx_master.xml", br#"<KNX xmlns="http://knx.org/xml/project/11"/>"#),
        ("P-0001/0.xml", installation),
        ("P-0001/Project.xml", MINIMAL_PROJECT_XML),
    ])
}

pub const MINIMAL_PROJECT_XML: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <Project Id="P-0001">
    <ProjectInformation Name="Minimal" GroupAddressStyle="ThreeLevel"
                        CompletionStatus="Editing"/>
  </Project>
</KNX>
"#;

/// `MINIMAL` with the `DeviceInstance` opening tag replaced, for tests that
/// need one malformed attribute and everything else intact.
pub fn minimal_xml_with(device_open_tag: &str) -> String {
    let text = String::from_utf8(MINIMAL.to_vec()).unwrap();
    let start = text.find("<DeviceInstance").expect("MINIMAL has a DeviceInstance");
    let end = text[start..].find("/>").expect("self-closing") + start;
    format!("{}{}{}", &text[..start], device_open_tag, &text[end..])
}

/// A container whose group range holds two group addresses sharing one `Id`.
pub fn knxproj_with_duplicate_ga_id() -> Vec<u8> {
    let text = String::from_utf8(MINIMAL.to_vec()).unwrap().replace(
        r#"<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" DatapointType="DPST-1-1"/>"#,
        concat!(
            r#"<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="A" DatapointType="DPST-1-1"/>"#,
            r#"<GroupAddress Id="P-0001-0_GA-1" Address="2" Name="B" DatapointType="DPST-1-1"/>"#,
        ),
    );
    knxproj_with_installation(text.as_bytes())
}

/// A container whose only communication object links to a group address that
/// does not exist.
pub fn knxproj_with_dangling_link() -> Vec<u8> {
    let text = String::from_utf8(MINIMAL.to_vec()).unwrap().replace(
        r#"Hardware2ProgramRefId="M-0001_H-1_HP-1"/>"#,
        concat!(
            r#"Hardware2ProgramRefId="M-0001_H-1_HP-1">"#,
            r#"<ComObjectInstanceRefs>"#,
            r#"<ComObjectInstanceRef RefId="M-0001_A-1_O-1_R-1" DatapointType="DPST-1-1">"#,
            r#"<Connectors><Send GroupAddressRefId="P-0001-0_GA-404"/></Connectors>"#,
            r#"</ComObjectInstanceRef>"#,
            r#"</ComObjectInstanceRefs>"#,
            r#"</DeviceInstance>"#,
        ),
    );
    knxproj_with_installation(text.as_bytes())
}

/// A ZIP whose local header claims a huge uncompressed size for one entry.
/// The bytes are small; only the declared size is large. Used to prove the
/// container refuses the entry before allocating for it.
pub fn zip_with_declared_size(name: &str, declared: u64) -> Vec<u8> {
    let mut bytes = zip_with_entries(&[(name, b"x")]);
    patch_declared_uncompressed_size(&mut bytes, declared);
    bytes
}

/// Rewrites the uncompressed-size field in the local header and the central
/// directory entry. Both must agree or the archive is rejected as corrupt
/// before the guard under test is reached.
fn patch_declared_uncompressed_size(bytes: &mut [u8], declared: u64) {
    let value = u32::try_from(declared).unwrap_or(u32::MAX).to_le_bytes();
    let local = 0x0403_4b50u32.to_le_bytes();
    let central = 0x0201_4b50u32.to_le_bytes();
    for offset in 0..bytes.len().saturating_sub(4) {
        if bytes[offset..offset + 4] == local {
            bytes[offset + 22..offset + 26].copy_from_slice(&value);
        } else if bytes[offset..offset + 4] == central {
            bytes[offset + 24..offset + 28].copy_from_slice(&value);
        }
    }
}

/// Runs the built `knx` binary with the given arguments.
pub fn run_cli(args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .expect("the knx binary is built by the test harness")
}
```

`run_cli` lives in `apps/knx-cli/tests/cli_import.rs` rather than in this
module, because `CARGO_BIN_EXE_knx` is only defined for integration tests of
the crate that declares the binary. It is listed here so the two copies stay in
agreement.

`zip_with_entries` also exists as an in-module helper in `container.rs`'s unit
tests (Task 3); integration tests cannot reach it, which is why it is repeated
here rather than exported from the crate's public API for the benefit of tests.

---

### Task 1: `knx-core` — per-attribute overrides on communication objects

The override chain operates per attribute, not per object. A schema-11
`ComObjectInstanceRef` may state `ReadFlag` and nothing else, may state
`DatapointType=""`, and never states `ObjectSize` at all. The current
`ComObjectInstance` — `text: Resolved<LocalizedString>`,
`flags: Resolved<ComFlags>`, `size: Resolved<ObjectSize>` — cannot hold any of
those three shapes, so the importer cannot be written against it.

**Files:**
- Modify: `crates/knx-core/src/provenance.rs` — add `Override<T>`
- Modify: `crates/knx-core/src/string_table.rs` — add `Text`
- Modify: `crates/knx-core/src/flags.rs` — add `ResolvedFlags`
- Modify: `crates/knx-core/src/device.rs` — `ComObjectInstance` fields
- Modify: `crates/knx-core/src/lib.rs` — re-exports
- Modify: `crates/knx-core/src/command.rs` — the `SetComObjectDpt` /
  `RestoreComObjectDpt` handlers and their tests now read and write
  `Override<DptRef>`
- Test: in-module `#[cfg(test)]` in each modified file

**Interfaces:**
- Produces: `Override<T>` with variants `Absent`, `Empty`, `Value(Resolved<T>)`
  and methods `value(&self) -> Option<&Resolved<T>>`,
  `layer(&self) -> Option<Layer>`, `is_present(&self) -> bool`;
  `Text` with variants `Literal(String)` and `Localized(LocalizedString)`;
  `ResolvedFlags { read, write, transmit, update, communication }`, each
  `Override<bool>`, plus `ResolvedFlags::none()`;
  `ComObjectInstance { text: Override<Text>, description: Override<Text>,
  dpt: Override<DptRef>, flags: ResolvedFlags, size: Option<Resolved<ObjectSize>> }`.
  Task 10 (`map.rs`) constructs these; Task 18 (`export/schema11.rs`) reads them.

- [ ] **Step 1: Write failing tests for `Override<T>`**

Add to `crates/knx-core/src/provenance.rs`:

```rust
#[test]
fn absent_and_empty_are_distinct_and_neither_carries_a_value() {
    let absent: Override<u8> = Override::Absent;
    let empty: Override<u8> = Override::Empty;
    assert_ne!(absent, empty);
    assert!(absent.value().is_none());
    assert!(empty.value().is_none());
    assert!(!absent.is_present());
    assert!(empty.is_present());
}

#[test]
fn a_value_override_reports_its_layer() {
    let o = Override::Value(Resolved {
        value: 7u8,
        layer: Layer::Instance,
    });
    assert_eq!(o.layer(), Some(Layer::Instance));
    assert_eq!(o.value().map(|r| r.value), Some(7));
    assert!(o.is_present());
}
```

- [ ] **Step 2: Run the tests and watch them fail**

Run: `cargo test -p knx-core provenance`
Expected: FAIL, `cannot find type Override in this scope`.

- [ ] **Step 3: Implement `Override<T>`**

Add to `crates/knx-core/src/provenance.rs`:

```rust
/// A source attribute in one of its three real states.
///
/// ETS distinguishes an attribute it never wrote from an attribute it wrote
/// empty: 497 of the reference project's 907 `ComObjectInstanceRef` elements
/// carry `DatapointType=""`, and 82 of its `Description` attributes are
/// likewise empty. Collapsing both into `None` loses that distinction, and
/// export then writes a file that differs from the one that was read.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Override<T> {
    /// The attribute was not present in the source.
    Absent,
    /// The attribute was present and its value was the empty string.
    Empty,
    /// The attribute was present and carried a value.
    Value(Resolved<T>),
}

impl<T> Override<T> {
    pub fn value(&self) -> Option<&Resolved<T>> {
        match self {
            Override::Value(r) => Some(r),
            _ => None,
        }
    }

    pub fn layer(&self) -> Option<Layer> {
        self.value().map(|r| r.layer)
    }

    /// Whether the attribute appeared in the source at all, empty or not.
    pub fn is_present(&self) -> bool {
        !matches!(self, Override::Absent)
    }
}

impl<T> Default for Override<T> {
    fn default() -> Self {
        Override::Absent
    }
}
```

- [ ] **Step 4: Run the tests and watch them pass**

Run: `cargo test -p knx-core provenance`
Expected: PASS.

- [ ] **Step 5: Write a failing test for `Text`**

Add to `crates/knx-core/src/string_table.rs`:

```rust
#[test]
fn literal_text_resolves_without_the_table_localized_text_does_not() {
    let table = StringTable::new(Language("en".into()));
    let literal = Text::Literal("LICHT_AN_AUS_EG_GARDEROBE".into());
    assert_eq!(
        table.text(&literal, &Language("de".into())),
        Some("LICHT_AN_AUS_EG_GARDEROBE")
    );
    let localized = Text::Localized(LocalizedString(TranslationKey("k1".into())));
    assert_eq!(table.text(&localized, &Language("de".into())), None);
}
```

- [ ] **Step 6: Run it and watch it fail**

Run: `cargo test -p knx-core string_table`
Expected: FAIL, `cannot find type Text in this scope`.

- [ ] **Step 7: Implement `Text` and `StringTable::text`**

Add to `crates/knx-core/src/string_table.rs`:

```rust
/// A display string that is either literal or a handle into the string table.
///
/// Instance-level overrides in `0.xml` are literal — ETS writes the text into
/// the project file and keeps no translation for it. Program-level defaults
/// are localized, resolved through `TranslationUnit` trees in the application
/// program. One field has to hold both.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Text {
    Literal(String),
    Localized(LocalizedString),
}

impl StringTable {
    /// Resolves either form of text for `language`. A literal is returned as
    /// it stands; a handle is resolved through the table.
    pub fn text<'a>(&'a self, text: &'a Text, language: &Language) -> Option<&'a str> {
        match text {
            Text::Literal(s) => Some(s.as_str()),
            Text::Localized(handle) => self.resolve(handle, language),
        }
    }
}
```

- [ ] **Step 8: Run it and watch it pass**

Run: `cargo test -p knx-core string_table`
Expected: PASS.

- [ ] **Step 9: Write a failing test for `ResolvedFlags`**

Add to `crates/knx-core/src/flags.rs`:

```rust
#[test]
fn a_partial_flag_override_leaves_the_other_flags_absent() {
    let flags = ResolvedFlags {
        read: Override::Value(Resolved {
            value: true,
            layer: Layer::Instance,
        }),
        ..ResolvedFlags::none()
    };
    assert_eq!(flags.read.value().map(|r| r.value), Some(true));
    assert!(!flags.write.is_present());
    assert!(!flags.communication.is_present());
}
```

- [ ] **Step 10: Run it and watch it fail**

Run: `cargo test -p knx-core flags`
Expected: FAIL, `cannot find type ResolvedFlags in this scope`.

- [ ] **Step 11: Implement `ResolvedFlags`**

Add to `crates/knx-core/src/flags.rs`:

```rust
use crate::provenance::Override;

/// The five communication flags, each resolved independently.
///
/// The override chain works per attribute: a `ComObjectInstanceRef` in the
/// reference project sets `ReadFlag` 39 times, `UpdateFlag` 30, `TransmitFlag`
/// 27, `WriteFlag` 18 and `CommunicationFlag` 8 — never all five together. A
/// single `Resolved<ComFlags>` would have to invent the four it was not told
/// about. `ComFlags` remains the fully resolved five-flag view, produced once
/// the product database supplies the program-level defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ResolvedFlags {
    pub read: Override<bool>,
    pub write: Override<bool>,
    pub transmit: Override<bool>,
    pub update: Override<bool>,
    pub communication: Override<bool>,
}

impl ResolvedFlags {
    /// No flag stated at any layer.
    pub fn none() -> Self {
        Self::default()
    }
}
```

- [ ] **Step 12: Run it and watch it pass**

Run: `cargo test -p knx-core flags`
Expected: PASS.

- [ ] **Step 13: Change `ComObjectInstance` and add `BinaryDataRef`**

In `crates/knx-core/src/device.rs`, replace the five override fields and add
the binary-data reference list to `DeviceInstance`:

```rust
/// A reference from a device to one of the opaque blobs in
/// `<P-xxxx>/BinaryData/<guid>.dat`. The bytes live in the opaque store; only
/// the reference is modelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryDataRef {
    /// `BinaryData/@Id` — the GUID that names the `.dat` entry.
    pub id: String,
    /// `BinaryData/@Name`.
    pub name: String,
}

pub struct DeviceInstance {
    // ... existing fields unchanged ...
    pub binary_data: Vec<BinaryDataRef>,
}

pub struct ComObjectInstance {
    pub id: ComObjectInstanceId,
    pub source: SourceRef,
    pub device: DeviceId,
    /// From `_O-<n>` in the source `RefId`.
    pub number: u16,
    pub text: Override<Text>,
    pub description: Override<Text>,
    pub dpt: Override<DptRef>,
    pub flags: ResolvedFlags,
    /// Never stated at instance level in schema 11; filled from the
    /// application program once the product database exists.
    pub size: Option<Resolved<ObjectSize>>,
    pub is_active: bool,
    pub links: Vec<GroupLink>,
}
```

Update the two existing tests in that module to construct the new shape, and
add one that locks the empty-versus-absent distinction:

```rust
#[test]
fn an_empty_datapoint_type_attribute_is_not_the_same_as_an_absent_one() {
    let mut com = com_object_instance_fixture();
    com.dpt = Override::Empty;
    assert!(com.dpt.value().is_none());
    assert!(com.dpt.is_present());
    com.dpt = Override::Absent;
    assert!(!com.dpt.is_present());
}
```

- [ ] **Step 14: Fix `command.rs` for the new field types**

`Command::SetComObjectDpt` and `RestoreComObjectDpt` currently read and write
`Option<Resolved<DptRef>>`. Change their payloads to `Override<DptRef>` so that
undoing a change to an instance that carried `DatapointType=""` restores the
empty attribute rather than deleting it. Update the module's tests to match.

- [ ] **Step 15: Re-export from `lib.rs`**

```rust
pub use device::{BinaryDataRef, ComObjectInstance, DeviceInstance};
pub use flags::{ComFlags, Direction, GroupLink, ObjectSize, ResolvedFlags};
pub use provenance::{Layer, Override, Resolved};
pub use string_table::{Language, LocalizedString, StringTable, Text, TranslationKey};
```

- [ ] **Step 16: Run the gate**

Run the full gate command from Global Constraints.
Expected: all six commands pass.

- [ ] **Step 17: Commit**

```bash
git add crates/knx-core/src
git commit -m "knx-core: resolve communication object attributes per attribute

A ComObjectInstanceRef states some attributes and not others, and states
DatapointType empty on 497 of the reference project's 907 instances. Override<T>
keeps absent, empty and valued distinct so export can write back what was read."
```

---

### Task 2: `knx-core` — the entity gaps schema 11 exposes

Five pieces of `0.xml` and `Project.xml` have nowhere to go in the current
model. Adding them now is cheaper than discovering it inside the mapper.

**Files:**
- Modify: `crates/knx-core/src/project.rs` — `ProjectInfo`
- Modify: `crates/knx-core/src/topology.rs` — `Line` medium configuration
- Modify: `crates/knx-core/src/group.rs` — `GroupRange` nesting
- Modify: `crates/knx-core/src/parameter.rs` — owning device
- Modify: `crates/knx-core/src/lib.rs` — re-exports
- Test: in-module `#[cfg(test)]`

**Interfaces:**
- Produces: `ProjectInfo` (constructed by Task 10, read by Task 18);
  `Line::{domain_address, domain_address_is_checked, ip_routing_multicast_address, multicast_ttl}`;
  `GroupRange::{parent, children}`; `ParameterInstance::device`.

- [ ] **Step 1: Write a failing test for `ProjectInfo`**

Add to `crates/knx-core/src/project.rs`:

```rust
#[test]
fn a_project_carries_its_group_address_style_and_identity() {
    let p = Project::new(Language("de-DE".into()));
    assert_eq!(p.info.group_address_style, GroupAddressStyle::ThreeLevel);
    assert!(p.info.project_id.is_empty());
    assert!(p.info.name.is_empty());
}
```

- [ ] **Step 2: Run it and watch it fail**

Run: `cargo test -p knx-core project`
Expected: FAIL, `no field info on type Project`.

- [ ] **Step 3: Implement `ProjectInfo`**

Add to `crates/knx-core/src/project.rs`:

```rust
/// `Project/@Id` plus the `ProjectInformation` element.
///
/// `group_address_style` is here rather than on `GroupAddress` because it is a
/// project-wide rendering choice: the 16-bit value never changes, only how it
/// is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    /// `Project/@Id`, e.g. `"P-0512"`. The container's project part is named
    /// after it.
    pub project_id: String,
    /// `ProjectInformation/@Name`.
    pub name: String,
    /// `ProjectInformation/@ProjectId` — the user-facing project number, a
    /// different thing from `project_id` despite the attribute name.
    pub project_number: Option<String>,
    pub group_address_style: GroupAddressStyle,
    pub completion: CompletionStatus,
    pub last_modified: Option<DateTime<Utc>>,
    pub project_start: Option<DateTime<Utc>>,
}

impl Default for ProjectInfo {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            name: String::new(),
            project_number: None,
            // ETS's own default, and the style of both reference exports.
            group_address_style: GroupAddressStyle::ThreeLevel,
            completion: CompletionStatus::Undefined,
            last_modified: None,
            project_start: None,
        }
    }
}
```

and give `Project` an `info: ProjectInfo` field, initialised to
`ProjectInfo::default()` in `Project::new`.

`ProjectInformation`'s remaining attributes — `ProjectTracingLevel`,
`Hide16BitGroupsFromLegacyPlugins`, and schema 23's `LastUsedPuid` and `Guid` —
are ETS tool state with no meaning in this model. They are retained as opaque
attributes by Task 12, not modelled here.

- [ ] **Step 4: Run it and watch it pass**

Run: `cargo test -p knx-core project`
Expected: PASS.

- [ ] **Step 5: Write a failing test for line medium configuration**

Add to `crates/knx-core/src/topology.rs`:

```rust
#[test]
fn a_line_carries_its_medium_configuration() {
    let line = Line {
        id: LineId(1),
        source: source(),
        name: "Hauptlinie".into(),
        address: 1,
        medium_ref: "MT-0".into(),
        domain_address: Some("0".into()),
        domain_address_is_checked: Some(false),
        ip_routing_multicast_address: Some(Ipv4Addr::new(224, 0, 23, 12)),
        multicast_ttl: Some(16),
        completion: CompletionStatus::Accepted,
        devices: vec![],
    };
    assert_eq!(line.multicast_ttl, Some(16));
}
```

- [ ] **Step 6: Run it and watch it fail**

Run: `cargo test -p knx-core topology`
Expected: FAIL, `struct Line has no field named domain_address`.

- [ ] **Step 7: Add the fields**

```rust
pub struct Line {
    // ... existing fields ...
    /// `DomainAddress` — powerline and RF domain address, uninterpreted:
    /// its width and encoding depend on the medium.
    pub domain_address: Option<String>,
    pub domain_address_is_checked: Option<bool>,
    pub ip_routing_multicast_address: Option<Ipv4Addr>,
    pub multicast_ttl: Option<u8>,
}
```

`BusAccess`, the free-form ETS interface-driver configuration nested under
`Line`, is deliberately not modelled: RESEARCH §3.1 and DATA_MODEL §4 record it
as opaque tool configuration. Task 12 retains it verbatim.

- [ ] **Step 8: Run it and watch it pass**

Run: `cargo test -p knx-core topology`
Expected: PASS.

- [ ] **Step 9: Write a failing test for group range nesting**

Add to `crates/knx-core/src/group.rs`:

```rust
#[test]
fn group_ranges_nest_two_levels_deep() {
    let main = GroupRange {
        id: GroupRangeId(1),
        source: source(),
        name: "Licht".into(),
        start: GroupAddress::from_raw(2048),
        end: GroupAddress::from_raw(4095),
        parent: None,
        children: vec![GroupRangeId(2)],
    };
    let middle = GroupRange {
        id: GroupRangeId(2),
        source: source(),
        name: "Licht - An/Aus".into(),
        start: GroupAddress::from_raw(2048),
        end: GroupAddress::from_raw(2303),
        parent: Some(main.id),
        children: vec![],
    };
    assert!(main.contains(middle.start));
    assert_eq!(middle.parent, Some(GroupRangeId(1)));
}
```

- [ ] **Step 10: Run it and watch it fail**

Run: `cargo test -p knx-core group`
Expected: FAIL, `struct GroupRange has no field named parent`.

- [ ] **Step 11: Add `parent` and `children` to `GroupRange`**

The reference project nests `GroupRanges/GroupRange/GroupRange/GroupAddress`:
7 main ranges, 28 middle ranges, 514 addresses. Flattening the two levels
loses the tree the user built.

- [ ] **Step 12: Run it and watch it pass**

Run: `cargo test -p knx-core group`
Expected: PASS.

- [ ] **Step 13: Give `ParameterInstance` its owning device**

`ParameterInstanceRef` is nested under `DeviceInstance`; `Installation`
holds a flat `Vec<ParameterInstance>`. Without a device field the association
is lost on import, which is silent discarding.

```rust
pub struct ParameterInstance {
    pub id: ParameterInstanceId,
    pub device: DeviceId,
    /// `RefId`, verbatim. Two forms occur: `<app>_P-<n>_R-<n>` for a plain
    /// parameter and `<app>_UP-<n>_R-<n>` for a union parameter, 1174 and 216
    /// respectively in the reference project. Neither is interpreted here.
    pub source: SourceRef,
    /// `Value`, uninterpreted.
    pub raw: String,
}
```

Add a test asserting a union-parameter `RefId` is stored verbatim:

```rust
#[test]
fn a_union_parameter_ref_id_is_stored_verbatim() {
    let p = ParameterInstance {
        id: ParameterInstanceId(1),
        device: DeviceId(3),
        source: SourceRef {
            path: "P-0512/0.xml".into(),
            ets_id: "M-0083_A-0026-15-7565_UP-411_R-411".into(),
        },
        raw: "1".into(),
    };
    assert!(p.source.ets_id.contains("_UP-"));
}
```

- [ ] **Step 14: Run the gate**

Run the full gate command.
Expected: all six commands pass.

- [ ] **Step 15: Commit**

```bash
git add crates/knx-core/src
git commit -m "knx-core: model project information, line media config, group range nesting

Adds the schema-11 entities the model had nowhere to put: ProjectInformation,
the line's domain and multicast configuration, the two-level group range tree,
and the device a parameter instance belongs to."
```

---

### Task 3: Stage 1 — the container

**Files:**
- Create: `crates/knx-etsproj/src/container.rs`
- Modify: `crates/knx-etsproj/src/lib.rs`, `crates/knx-etsproj/Cargo.toml`
- Test: in-module `#[cfg(test)]`

**Interfaces:**
- Produces:
  ```rust
  pub struct Container { /* private: zip archive over Cursor<Vec<u8>> */ }
  pub struct EntryInfo { pub path: String, pub size: u64 }
  impl Container {
      pub fn open(bytes: Vec<u8>) -> Result<Self, ContainerError>;
      pub fn entries(&self) -> &[EntryInfo];
      pub fn read(&mut self, path: &str) -> Result<Vec<u8>, ContainerError>;
      /// Case-insensitive over the inventory; `Project.xml` and `project.xml`
      /// both resolve.
      pub fn find(&self, path: &str) -> Option<&EntryInfo>;
      /// The `P-xxxx` part, recovered from the `P-*.signature` entry name.
      pub fn project_part(&self) -> Result<&str, ContainerError>;
  }
  pub enum ContainerError {
      NotAZip(String),
      EntryNotFound(String),
      Read { path: String, cause: String },
      NoProjectPart,
      PasswordProtected { nested_entry: String },
  }
  ```
  Task 4 calls `read` and `project_part`; Task 12 walks `entries`; Task 19
  writes a container back.

- [ ] **Step 1: Add the dependencies**

In `crates/knx-etsproj/Cargo.toml`:

```toml
[dependencies]
knx-core.workspace = true
zip.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

and in the workspace root `Cargo.toml` under `[workspace.dependencies]`:

```toml
zip = { version = "8.6", default-features = false, features = ["deflate-flate2"] }
```

- [ ] **Step 2: Write failing tests**

```rust
#[test]
fn the_reference_project_inventory_has_thirty_eight_entries() {
    let mut c = Container::open(reference_ets4_bytes()).unwrap();
    assert_eq!(c.entries().len(), 38);
    assert!(c.read("knx_master.xml").unwrap().starts_with(b"\xef\xbb\xbf<?xml"));
}

#[test]
fn entry_lookup_is_case_insensitive() {
    let c = Container::open(reference_ets4_bytes()).unwrap();
    assert_eq!(
        c.find("p-0512/project.xml").map(|e| e.path.as_str()),
        Some("P-0512/Project.xml")
    );
}

#[test]
fn the_project_part_comes_from_the_signature_filename() {
    let c = Container::open(reference_ets4_bytes()).unwrap();
    assert_eq!(c.project_part().unwrap(), "P-0512");
}

#[test]
fn a_file_that_is_not_a_zip_is_rejected_with_its_own_error() {
    assert!(matches!(
        Container::open(b"not a zip at all".to_vec()),
        Err(ContainerError::NotAZip(_))
    ));
}

#[test]
fn a_password_protected_project_is_detected_and_named() {
    // A container whose only project-part entry is `P-0512.zip` is the
    // nested-payload form ETS writes for a protected project.
    let bytes = zip_with_entries(&[("P-0512.signature", b"x"), ("P-0512.zip", b"PK\x03\x04")]);
    assert!(matches!(
        Container::open(bytes),
        Err(ContainerError::PasswordProtected { .. })
    ));
}
```

`reference_ets4_bytes()` reads
`concat!(env!("CARGO_MANIFEST_DIR"), "/../../Unser Zuhause ets4 - 2025-12-15.knxproj")`.
`zip_with_entries` builds a small archive in memory with `zip::ZipWriter`.

- [ ] **Step 3: Run them and watch them fail**

Run: `cargo test -p knx-etsproj container`
Expected: FAIL, `cannot find function Container::open`.

- [ ] **Step 4: Implement `container.rs`**

`open` builds `zip::ZipArchive` over a `Cursor<Vec<u8>>`, snapshots the
inventory into `Vec<EntryInfo>` (skipping directory entries, which ETS6 writes
and ETS4 does not), then checks for the protected form: an entry named
`<project part>.zip` at the archive root. `find` compares
`eq_ignore_ascii_case`. `project_part` scans for an entry matching
`P-*.signature` and returns its stem. `read` resolves through `find` so that
callers never have to know the generation's filename casing.

Password-protected projects are detected and refused, not decrypted. Both
decryption schemes of IMPORT_EXPORT §2 are unverified against a real protected
project, and shipping an untested decryption path would claim support this
repository cannot demonstrate. The error names the nested entry so the user
knows why.

- [ ] **Step 5: Run them and watch them pass**

Run: `cargo test -p knx-etsproj container`
Expected: PASS, 5 tests.

- [ ] **Step 6: Run the gate and commit**

```bash
git add crates/knx-etsproj Cargo.toml Cargo.lock
git commit -m "knx-etsproj: unpack the .knxproj container

Case-insensitive entry lookup, project part from the signature filename, and
detection of the password-protected nested-payload form, which is reported
rather than decrypted."
```

---

### Task 4: Stage 2 — schema detection

**Files:**
- Create: `crates/knx-etsproj/src/detect.rs`
- Modify: `crates/knx-etsproj/src/lib.rs`, `crates/knx-etsproj/Cargo.toml`

**Interfaces:**
- Consumes: `Container::read`, `Container::project_part` from Task 3.
- Produces:
  ```rust
  pub struct SchemaVersion(pub u32);
  pub struct Detected {
      pub version: SchemaVersion,
      pub namespace: String,
      pub created_by: Option<String>,   // KNX/@CreatedBy
      pub tool_version: Option<String>, // KNX/@ToolVersion
  }
  pub fn detect(container: &mut Container) -> Result<Detected, DetectError>;
  pub enum DetectError {
      Container(ContainerError),
      NoDefaultNamespace { entry: String },
      UnparsableNamespace { entry: String, namespace: String },
  }
  ```
  Task 5 dispatches on `SchemaVersion`; Task 14 reports `Detected`.

- [ ] **Step 1: Add `quick-xml`**

Workspace: `quick-xml = "0.42"`. Crate: `quick-xml.workspace = true`.

- [ ] **Step 2: Write failing tests**

```rust
#[test]
fn the_ets4_reference_project_is_schema_eleven() {
    let mut c = Container::open(reference_ets4_bytes()).unwrap();
    let d = detect(&mut c).unwrap();
    assert_eq!(d.version, SchemaVersion(11));
    assert_eq!(d.namespace, "http://knx.org/xml/project/11");
    assert_eq!(d.created_by.as_deref(), Some("ETS4"));
    assert_eq!(d.tool_version.as_deref(), Some("ETS 4.1.8 (Build 3614)"));
}

#[test]
fn the_ets6_reference_project_is_schema_twenty_three() {
    let mut c = Container::open(reference_ets6_bytes()).unwrap();
    assert_eq!(detect(&mut c).unwrap().version, SchemaVersion(23));
}

#[test]
fn a_namespace_without_a_trailing_integer_is_an_error_not_a_guess() {
    let xml = br#"<KNX xmlns="http://knx.org/xml/project/eleven"/>"#;
    assert!(matches!(
        version_from_root(xml, "0.xml"),
        Err(DetectError::UnparsableNamespace { .. })
    ));
}

#[test]
fn a_document_with_no_default_namespace_is_an_error() {
    let xml = br#"<KNX CreatedBy="ETS4"/>"#;
    assert!(matches!(
        version_from_root(xml, "0.xml"),
        Err(DetectError::NoDefaultNamespace { .. })
    ));
}
```

- [ ] **Step 3: Run them and watch them fail**

Run: `cargo test -p knx-etsproj detect`
Expected: FAIL, `cannot find function detect`.

- [ ] **Step 4: Implement `detect.rs`**

Read `<project part>/0.xml`, pull the root element's `xmlns` attribute with a
`quick_xml::Reader` stopping at the first `Start` event, take the trailing
path segment after the last `/` and parse it as `u32`. Never infer from a
filename or from the entry layout. `created_by` and `tool_version` come from
the same element.

IMPORT_EXPORT §3 notes that `xknxproject` reads the version from
`knx_master.xml` instead, and that a disagreement between the two is a report
entry. Implement `version_from_master(container)` alongside, and have `detect`
record a `namespace_disagreement` field when the two differ. The reference
projects agree; the check exists so that a future sample that does not agree
says so.

- [ ] **Step 5: Run them and watch them pass**

Run: `cargo test -p knx-etsproj detect`
Expected: PASS, 4 tests.

- [ ] **Step 6: Run the gate and commit**

```bash
git add crates/knx-etsproj Cargo.toml Cargo.lock
git commit -m "knx-etsproj: detect the schema version from the default namespace

Read from the project part, never assumed and never inferred from a filename.
Both reference projects are covered: schema 11 and schema 23."
```

---

### Task 5: The `SourceDocument` and the known-element tables

**Files:**
- Create: `crates/knx-etsproj/src/source.rs`
- Create: `crates/knx-etsproj/src/known.rs`

**Interfaces:**
- Produces: the types below. Task 6 fills them; Tasks 9 and 10 read them.

Every attribute is a `String` — exactly the bytes that were in the file. The
source layer performs no conversion at all, so the parser can be tested against
raw XML with no domain model in sight, and so `map` is the single place a
conversion can fail.

```rust
pub struct SourceDocument {
    pub schema_version: u32,
    pub created_by: Option<String>,
    pub tool_version: Option<String>,
    pub project_id: String,
    pub info: SourceProjectInfo,
    pub installations: Vec<SourceInstallation>,
}

pub struct SourceProjectInfo {
    pub name: Option<String>,
    pub project_number: Option<String>,
    pub group_address_style: Option<String>,
    pub completion_status: Option<String>,
    pub last_modified: Option<String>,
    pub project_start: Option<String>,
    pub other: Vec<RetainedAttribute>,
}

pub struct SourceInstallation {
    pub installation_id: Option<String>,
    pub name: Option<String>,
    pub default_line: Option<String>,
    pub ip_routing_multicast_address: Option<String>,
    pub completion_status: Option<String>,
    pub areas: Vec<SourceArea>,
    pub unassigned_devices: Vec<SourceDevice>,
    pub buildings: Vec<SourceBuildingPart>,
    pub group_ranges: Vec<SourceGroupRange>,
    pub other: Vec<RetainedAttribute>,
}

pub struct SourceArea {
    pub id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub completion_status: Option<String>,
    pub lines: Vec<SourceLine>,
    pub other: Vec<RetainedAttribute>,
}

pub struct SourceLine {
    pub id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub medium_type_ref_id: Option<String>,
    pub domain_address: Option<String>,
    pub domain_address_is_checked: Option<String>,
    pub ip_routing_multicast_address: Option<String>,
    pub multicast_ttl: Option<String>,
    pub completion_status: Option<String>,
    pub devices: Vec<SourceDevice>,
    /// `BusAccess`, retained verbatim — ETS interface-driver configuration.
    pub bus_access: Option<RetainedElement>,
    pub other: Vec<RetainedAttribute>,
}

pub struct SourceDevice {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub address: Option<String>,
    pub product_ref_id: Option<String>,
    pub hardware2program_ref_id: Option<String>,
    pub last_modified: Option<String>,
    pub last_download: Option<String>,
    pub completion_status: Option<String>,
    pub individual_address_loaded: Option<String>,
    pub application_program_loaded: Option<String>,
    pub parameters_loaded: Option<String>,
    pub communication_part_loaded: Option<String>,
    pub medium_config_loaded: Option<String>,
    pub visibility_calculated: Option<String>,
    pub broken: Option<String>,
    pub parameters: Vec<SourceParameterInstance>,
    pub com_objects: Vec<SourceComObjectInstance>,
    pub binary_data: Vec<SourceBinaryDataRef>,
    pub other: Vec<RetainedAttribute>,
}

pub struct SourceParameterInstance {
    pub ref_id: String,
    pub value: Option<String>,
}

pub struct SourceComObjectInstance {
    pub ref_id: String,
    pub is_active: Option<String>,
    pub datapoint_type: Option<String>,
    pub text: Option<String>,
    pub description: Option<String>,
    pub read_flag: Option<String>,
    pub write_flag: Option<String>,
    pub transmit_flag: Option<String>,
    pub update_flag: Option<String>,
    pub communication_flag: Option<String>,
    pub sends: Vec<String>,    // Connectors/Send/@GroupAddressRefId
    pub receives: Vec<String>, // Connectors/Receive/@GroupAddressRefId
    pub other: Vec<RetainedAttribute>,
}

pub struct SourceBinaryDataRef { pub id: String, pub name: Option<String> }

pub struct SourceBuildingPart {
    pub id: String,
    pub name: Option<String>,
    pub number: Option<String>,
    pub kind: Option<String>,
    pub default_line: Option<String>,
    pub completion_status: Option<String>,
    pub children: Vec<SourceBuildingPart>,
    pub device_refs: Vec<String>,
    pub other: Vec<RetainedAttribute>,
}

pub struct SourceGroupRange {
    pub id: String,
    pub name: Option<String>,
    pub range_start: Option<String>,
    pub range_end: Option<String>,
    pub children: Vec<SourceGroupRange>,
    pub addresses: Vec<SourceGroupAddress>,
    pub other: Vec<RetainedAttribute>,
}

pub struct SourceGroupAddress {
    pub id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub central: Option<String>,
    pub unfiltered: Option<String>,
    pub other: Vec<RetainedAttribute>,
}

/// An attribute the known-element table for this schema version does not
/// list. Kept with its value so that neither the value nor the fact that it
/// was unknown is lost.
pub struct RetainedAttribute {
    pub xpath: String,
    pub name: String,
    pub value: String,
}

/// An element the known-element table does not list, or one that is known but
/// deliberately not modelled, kept as the raw bytes it occupied in the source.
pub struct RetainedElement {
    pub xpath: String,
    pub name: String,
    pub raw: Vec<u8>,
}
```

`known.rs` holds the per-version tables the tolerant parser checks against:

```rust
pub struct KnownElement {
    /// Absolute element path, e.g.
    /// `/KNX/Project/Installations/Installation/Topology/Area/Line`.
    pub path: &'static str,
    pub attributes: &'static [&'static str],
}

pub struct KnownSchema {
    pub version: u32,
    pub elements: &'static [KnownElement],
}

pub fn known_schema(version: u32) -> Option<&'static KnownSchema>;
pub const SCHEMA_11: KnownSchema = /* ... */;
```

`SCHEMA_11`'s table is transcribed from the reference project's measured
inventory, 27 element paths in total: `KNX` (`CreatedBy`, `ToolVersion`),
`Project` (`Id`), `ProjectInformation` (`Name`, `LastModified`, `ProjectStart`,
`ProjectId`, `ProjectTracingLevel`, `Hide16BitGroupsFromLegacyPlugins`,
`GroupAddressStyle`, `CompletionStatus`), `Installations`, `Installation`
(`InstallationId`, `Name`, `BCUKey`, `DefaultLine`,
`IPRoutingMulticastAddress`, `SplitType`, `CompletionStatus`), `Topology`,
`Area` (`Id`, `Name`, `Address`, `CompletionStatus`), `Line` (`Id`, `Name`,
`Address`, `MediumTypeRefId`, `DomainAddress`, `DomainAddressIsChecked`,
`CompletionStatus`, `IPRoutingMulticastAddress`, `MulticastTTL`), `BusAccess`
(`Name`, `Edi`, `Parameter`), `DeviceInstance` (the 16 measured attributes),
`ParameterInstanceRefs`, `ParameterInstanceRef` (`RefId`, `Value`),
`ComObjectInstanceRefs`, `ComObjectInstanceRef` (the 10 measured attributes),
`Connectors`, `Send` (`GroupAddressRefId`), `Receive` (`GroupAddressRefId`),
`BinaryData` (`Id`, `Name`, and the container form with none),
`UnassignedDevices`, `Buildings`, `BuildingPart` (`Id`, `Name`, `Number`,
`Type`, `DefaultLine`, `CompletionStatus`), `DeviceInstanceRef` (`RefId`),
`GroupAddresses`, `GroupRanges`, `GroupRange` (`Id`, `Name`, `RangeStart`,
`RangeEnd`), `GroupAddress` (`Id`, `Name`, `Address`, `Central`,
`Unfiltered`).

The table is evidence, not schema: no authoritative XSD is available
(RESEARCH §2.2), so it lists what has been seen and the parser reports
everything else rather than failing on it.

- [ ] **Step 1: Write failing tests for the known-element table**

```rust
#[test]
fn schema_eleven_knows_the_measured_element_paths() {
    let s = known_schema(11).unwrap();
    let line = s.elements.iter()
        .find(|e| e.path == "/KNX/Project/Installations/Installation/Topology/Area/Line")
        .unwrap();
    assert!(line.attributes.contains(&"MediumTypeRefId"));
    assert!(line.attributes.contains(&"MulticastTTL"));
    assert!(!line.attributes.contains(&"Puid")); // schema 23 only
}

#[test]
fn an_unknown_schema_version_has_no_table() {
    assert!(known_schema(23).is_none());
}

#[test]
fn every_known_path_is_absolute_and_unique() {
    let s = known_schema(11).unwrap();
    let mut paths: Vec<_> = s.elements.iter().map(|e| e.path).collect();
    let before = paths.len();
    paths.sort_unstable();
    paths.dedup();
    assert_eq!(paths.len(), before, "duplicate element path in SCHEMA_11");
    assert!(s.elements.iter().all(|e| e.path.starts_with("/KNX")));
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-etsproj known`
Expected: FAIL, `cannot find function known_schema`.

- [ ] **Step 3: Write `source.rs` and `known.rs`**

`known_schema(23)` returning `None` is deliberate for this session: the
schema-23 differences of RESEARCH §3.3 are load-bearing — `RefId` loses its
prefix, links move to attributes, `GroupObjectTree` becomes the authoritative
object list, booleans change spelling — and a table that pretended otherwise
would produce silently wrong data. Schema 23 import reports "no known-element
table for this schema version" and stops; that is the honest state until
schema 23 is implemented.

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-etsproj known`
Expected: PASS, 3 tests.

- [ ] **Step 5: Run the gate and commit**

```bash
git add crates/knx-etsproj/src
git commit -m "knx-etsproj: add the source document and the schema-11 known-element table

The source layer holds raw strings shaped like the XML so that parsing is
testable without a domain model. The known-element table is transcribed from
the reference project's measured inventory: no authoritative XSD exists."
```

---

### Task 6: Stage 3 — the tolerant streaming parser for `0.xml`

**Files:**
- Create: `crates/knx-etsproj/src/parse/mod.rs`
- Create: `crates/knx-etsproj/src/parse/installation.rs`

**Interfaces:**
- Consumes: `SourceDocument` types (Task 5), `known_schema` (Task 5).
- Produces:
  ```rust
  pub struct ParseOutput {
      pub document: SourceDocument,
      pub unknown: Vec<UnknownConstruct>,
      pub retained_elements: Vec<RetainedElement>,
  }
  pub struct UnknownConstruct {
      pub source_path: String,   // container entry, e.g. "P-0512/0.xml"
      pub xpath: String,         // "/KNX/Project/.../DeviceInstance"
      pub kind: UnknownKind,     // Element | Attribute
      pub name: String,
      pub occurrences: u32,
      /// One example value, for an attribute.
      pub sample: Option<String>,
  }
  pub fn parse_installation(
      bytes: &[u8],
      source_path: &str,
      schema: &KnownSchema,
  ) -> Result<ParseOutput, ParseError>;
  pub enum ParseError {
      Xml { source_path: String, position: u64, cause: String },
      UnsupportedSchemaVersion(u32),
      MissingRequiredAttribute { xpath: String, name: String },
  }
  ```
  Task 9 validates `ParseOutput`; Task 10 maps it; Task 14 reports `unknown`.

The reader is a `quick_xml::Reader` over the byte slice with a reused buffer.
It maintains an explicit path stack of element names, so the current absolute
path is available at every event without a DOM. For each `Start`/`Empty`
event:

1. Build the absolute path from the stack.
2. Look it up in the `KnownSchema` table.
3. If the path is unknown, record an `UnknownConstruct` of kind `Element`,
   capture the element's raw byte span from `reader.buffer_position()` before
   and after skipping its subtree, push it into `retained_elements`, and
   continue. The document is not abandoned.
4. If the path is known, walk its attributes. Each attribute not in the table's
   list becomes an `UnknownConstruct` of kind `Attribute` and is also pushed
   into the owning source struct's `other: Vec<RetainedAttribute>`, so the
   value survives as well as the fact.
5. Assign the known attributes to the source struct's fields verbatim.

`UnknownConstruct`s are aggregated by `(source_path, xpath, kind, name)` with
an occurrence count, so a project that repeats an unknown attribute 900 times
produces one report line, not 900.

- [ ] **Step 1: Write failing tests against hand-written XML**

```rust
const MINIMAL: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="P-0001">
    <Installations>
      <Installation InstallationId="0" Name="" DefaultLine="P-0001-0_L-2" CompletionStatus="Undefined">
        <Topology>
          <Area Id="P-0001-0_A-1" Name="A" Address="1" CompletionStatus="Undefined">
            <Line Id="P-0001-0_L-2" Name="L" Address="1" MediumTypeRefId="MT-0" CompletionStatus="Accepted">
              <DeviceInstance Id="P-0001-0_DI-1" Name="D" ProductRefId="M-0001_H-1_P-1"
                              Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1"
                              LastModified="2023-07-14T11:55:33" CompletionStatus="FinishedDesign"
                              IndividualAddressLoaded="1" ApplicationProgramLoaded="1"
                              ParametersLoaded="1" CommunicationPartLoaded="1"
                              MediumConfigLoaded="1" IsCommunicationObjectVisibilityCalculated="1"
                              Broken="0">
                <ComObjectInstanceRefs>
                  <ComObjectInstanceRef RefId="M-0001_A-1_O-0_R-1" DatapointType="" IsActive="1">
                    <Connectors><Send GroupAddressRefId="P-0001-0_GA-1" /></Connectors>
                  </ComObjectInstanceRef>
                </ComObjectInstanceRefs>
              </DeviceInstance>
            </Line>
          </Area>
        </Topology>
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" Name="Licht" RangeStart="1" RangeEnd="255">
              <GroupRange Id="P-0001-0_GR-2" Name="An/Aus" RangeStart="1" RangeEnd="127">
                <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />
              </GroupRange>
            </GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

#[test]
fn a_minimal_document_parses_into_the_source_shape() {
    let out = parse_installation(MINIMAL, "P-0001/0.xml", known_schema(11).unwrap()).unwrap();
    let inst = &out.document.installations[0];
    assert_eq!(out.document.project_id, "P-0001");
    assert_eq!(inst.areas[0].lines[0].devices.len(), 1);
    let com = &inst.areas[0].lines[0].devices[0].com_objects[0];
    assert_eq!(com.sends, vec!["P-0001-0_GA-1"]);
    assert!(com.receives.is_empty());
    assert_eq!(inst.group_ranges[0].children[0].addresses[0].id, "P-0001-0_GA-1");
    assert!(out.unknown.is_empty());
}

#[test]
fn an_empty_attribute_value_is_kept_as_an_empty_string_not_dropped() {
    let out = parse_installation(MINIMAL, "P-0001/0.xml", known_schema(11).unwrap()).unwrap();
    let com = &out.document.installations[0].areas[0].lines[0].devices[0].com_objects[0];
    assert_eq!(com.datapoint_type.as_deref(), Some(""));
    assert_eq!(com.text, None);
}

#[test]
fn an_unknown_attribute_is_reported_with_its_value_and_not_fatal() {
    let xml = String::from_utf8(MINIMAL.to_vec())
        .unwrap()
        .replace(r#"Address="1" CompletionStatus="Accepted""#,
                 r#"Address="1" CompletionStatus="Accepted" Puid="42""#);
    let out = parse_installation(xml.as_bytes(), "P-0001/0.xml", known_schema(11).unwrap()).unwrap();
    let u = out.unknown.iter().find(|u| u.name == "Puid").unwrap();
    assert_eq!(u.kind, UnknownKind::Attribute);
    assert_eq!(u.xpath, "/KNX/Project/Installations/Installation/Topology/Area/Line");
    assert_eq!(u.sample.as_deref(), Some("42"));
    let line = &out.document.installations[0].areas[0].lines[0];
    assert!(line.other.iter().any(|a| a.name == "Puid" && a.value == "42"));
}

#[test]
fn an_unknown_element_is_retained_verbatim_and_reported() {
    let xml = String::from_utf8(MINIMAL.to_vec())
        .unwrap()
        .replace("<Topology>", "<Security SequenceNumber=\"7\"/><Topology>");
    let out = parse_installation(xml.as_bytes(), "P-0001/0.xml", known_schema(11).unwrap()).unwrap();
    let u = out.unknown.iter().find(|u| u.name == "Security").unwrap();
    assert_eq!(u.kind, UnknownKind::Element);
    let kept = out.retained_elements.iter().find(|e| e.name == "Security").unwrap();
    assert_eq!(kept.raw, br#"<Security SequenceNumber="7"/>"#);
}

#[test]
fn repeated_unknown_attributes_aggregate_into_one_report_line() {
    let xml = String::from_utf8(MINIMAL.to_vec()).unwrap().replace(
        r#"<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />"#,
        r#"<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" Puid="1" />
           <GroupAddress Id="P-0001-0_GA-2" Address="2" Name="GB" Puid="2" />"#,
    );
    let out = parse_installation(xml.as_bytes(), "P-0001/0.xml", known_schema(11).unwrap()).unwrap();
    let u = out.unknown.iter().find(|u| u.name == "Puid").unwrap();
    assert_eq!(u.occurrences, 2);
}

#[test]
fn truncated_xml_is_a_parse_error_carrying_its_byte_position() {
    let truncated = &MINIMAL[..MINIMAL.len() / 2];
    assert!(matches!(
        parse_installation(truncated, "P-0001/0.xml", known_schema(11).unwrap()),
        Err(ParseError::Xml { .. })
    ));
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-etsproj parse`
Expected: FAIL, `cannot find function parse_installation`.

- [ ] **Step 3: Implement `parse/mod.rs` and `parse/installation.rs`**

`mod.rs` holds the path stack, the unknown-construct aggregator, the raw-span
capture helper, and an `attr_map` helper that decodes a `BytesStart`'s
attributes into `Vec<(String, String)>` with entity references expanded.
`installation.rs` holds the element dispatch: one `match` on the absolute path,
each arm filling the corresponding source struct. Sub-elements are appended to
the innermost open struct, tracked with a small stack of in-progress values.

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-etsproj parse`
Expected: PASS, 6 tests.

- [ ] **Step 5: Add a test that the real reference project parses**

```rust
#[test]
fn the_reference_project_parses_with_no_unknown_constructs() {
    let mut c = Container::open(reference_ets4_bytes()).unwrap();
    let bytes = c.read("P-0512/0.xml").unwrap();
    let out = parse_installation(&bytes, "P-0512/0.xml", known_schema(11).unwrap()).unwrap();
    assert_eq!(
        out.unknown, vec![],
        "the schema-11 table was transcribed from this very project; \
         anything unknown here is a gap in the table"
    );
    let inst = &out.document.installations[0];
    let devices: usize = inst.areas.iter().flat_map(|a| &a.lines).map(|l| l.devices.len()).sum();
    assert_eq!(devices + inst.unassigned_devices.len(), 36);
}
```

- [ ] **Step 6: Run the gate and commit**

```bash
git add crates/knx-etsproj/src
git commit -m "knx-etsproj: tolerant streaming parser for schema-11 0.xml

Unknown elements are retained verbatim and unknown attributes keep their
values; both are aggregated into report entries rather than treated as errors.
The reference project parses with no unknown constructs."
```

---

### Task 7: Stage 3 — the `Project.xml` parser

**Files:**
- Create: `crates/knx-etsproj/src/parse/project_info.rs`
- Modify: `crates/knx-etsproj/src/parse/mod.rs`

**Interfaces:**
- Consumes: `SourceProjectInfo` (Task 5), the same unknown-construct machinery.
- Produces:
  ```rust
  pub fn parse_project_info(
      bytes: &[u8],
      source_path: &str,
      schema: &KnownSchema,
  ) -> Result<(SourceProjectInfo, Vec<UnknownConstruct>), ParseError>;
  ```
  Task 15 merges the result into the `SourceDocument`.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn the_reference_project_information_is_read_verbatim() {
    let mut c = Container::open(reference_ets4_bytes()).unwrap();
    let bytes = c.read("P-0512/Project.xml").unwrap();
    let (info, unknown) =
        parse_project_info(&bytes, "P-0512/Project.xml", known_schema(11).unwrap()).unwrap();
    assert_eq!(info.name.as_deref(), Some("Unser Zuhause"));
    assert_eq!(info.group_address_style.as_deref(), Some("ThreeLevel"));
    assert_eq!(info.completion_status.as_deref(), Some("Editing"));
    assert_eq!(info.last_modified.as_deref(), Some("2025-12-15T07:07:12"));
    assert_eq!(unknown, vec![]);
}

#[test]
fn tool_state_attributes_are_retained_rather_than_modelled() {
    let mut c = Container::open(reference_ets4_bytes()).unwrap();
    let bytes = c.read("P-0512/Project.xml").unwrap();
    let (info, _) =
        parse_project_info(&bytes, "P-0512/Project.xml", known_schema(11).unwrap()).unwrap();
    assert!(info.other.iter().any(|a| a.name == "ProjectTracingLevel"));
    assert!(info
        .other
        .iter()
        .any(|a| a.name == "Hide16BitGroupsFromLegacyPlugins" && a.value == "1"));
}
```

The second test states the rule: a known attribute that the model deliberately
does not carry is still retained, so export writes it back. Known-but-not-
modelled and unknown are different things — only the second is a report entry.

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-etsproj project_info`
Expected: FAIL, `cannot find function parse_project_info`.

- [ ] **Step 3: Implement `parse/project_info.rs`**

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-etsproj project_info`
Expected: PASS, 2 tests.

- [ ] **Step 5: Run the gate and commit**

```bash
git add crates/knx-etsproj/src
git commit -m "knx-etsproj: parse Project.xml

Known attributes the model deliberately does not carry are retained for export
rather than reported as unknown."
```

---

### Task 8: Value conversions

**Files:**
- Create: `crates/knx-etsproj/src/values.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn parse_bool(s: &str) -> Result<bool, ValueError>;
  pub fn parse_timestamp(s: &str) -> Result<DateTime<Utc>, ValueError>;
  pub fn parse_u8(s: &str, field: &'static str) -> Result<u8, ValueError>;
  pub fn parse_u16(s: &str, field: &'static str) -> Result<u16, ValueError>;
  pub fn parse_completion_status(s: &str) -> Result<CompletionStatus, ValueError>;
  pub fn parse_building_part_type(s: &str) -> Result<BuildingPartType, ValueError>;
  /// The `_O-<n>` object number inside a schema-11 compound `RefId`.
  pub fn com_object_number(ref_id: &str) -> Result<u16, ValueError>;
  /// The application program part of a compound `RefId`, e.g.
  /// `M-006A_A-0001-22-26C0-O0079`.
  pub fn application_program_ref(ref_id: &str) -> Result<&str, ValueError>;
  pub enum ValueError {
      NotABoolean(String),
      NotATimestamp(String),
      NotAnInteger { field: &'static str, value: String },
      UnknownEnumValue { kind: &'static str, value: String },
      MalformedRefId(String),
  }
  ```
  Task 10 calls all of these; failures become `ImportError` entries in the
  report rather than aborting the import.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn both_boolean_spellings_parse() {
    // Schema 11 writes "1"/"0"; schema 23 writes "true"/"false" (RESEARCH §3.3).
    assert!(parse_bool("1").unwrap());
    assert!(!parse_bool("0").unwrap());
    assert!(parse_bool("true").unwrap());
    assert!(!parse_bool("false").unwrap());
    assert!(parse_bool("True").unwrap());
    assert!(matches!(parse_bool("yes"), Err(ValueError::NotABoolean(_))));
    assert!(matches!(parse_bool(""), Err(ValueError::NotABoolean(_))));
}

#[test]
fn both_timestamp_spellings_parse() {
    // ETS4 writes a naive local timestamp, ETS6 an RFC 3339 instant.
    let naive = parse_timestamp("2023-07-14T11:55:33").unwrap();
    let rfc = parse_timestamp("2025-12-17T10:12:14.3525475Z").unwrap();
    assert_eq!(naive.to_rfc3339(), "2023-07-14T11:55:33+00:00");
    assert_eq!(rfc.date_naive().to_string(), "2025-12-17");
    assert!(matches!(parse_timestamp("14.07.2023"), Err(ValueError::NotATimestamp(_))));
}

#[test]
fn a_compound_ref_id_yields_its_object_number_and_program() {
    let r = "M-006A_A-0001-22-26C0-O0079_O-0_R-10001";
    assert_eq!(com_object_number(r).unwrap(), 0);
    assert_eq!(application_program_ref(r).unwrap(), "M-006A_A-0001-22-26C0-O0079");
    // The program id itself contains "-O0079"; only the "_O-<n>_R-" tail counts.
    assert_eq!(com_object_number("M-0083_A-0019-16-ECA7_O-59_R-149").unwrap(), 59);
}

#[test]
fn a_ref_id_without_the_object_tail_is_an_error() {
    assert!(matches!(
        com_object_number("M-0083_A-0019-16-ECA7"),
        Err(ValueError::MalformedRefId(_))
    ));
    // A parameter RefId is not a communication object RefId.
    assert!(com_object_number("M-0083_A-0026-15-7565_UP-411_R-411").is_err());
}

#[test]
fn completion_status_and_building_part_type_cover_the_observed_values() {
    assert_eq!(parse_completion_status("FinishedDesign").unwrap(),
               CompletionStatus::FinishedDesign);
    assert_eq!(parse_building_part_type("DistributionBoard").unwrap(),
               BuildingPartType::DistributionBoard);
    assert!(matches!(
        parse_building_part_type("Cupboard"),
        Err(ValueError::UnknownEnumValue { kind: "BuildingPart/@Type", .. })
    ));
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-etsproj values`
Expected: FAIL, `cannot find function parse_bool`.

- [ ] **Step 3: Implement `values.rs`**

`com_object_number` splits on `_`, requires the last segment to start with
`R-` and the one before it to match `O-<digits>` exactly, and parses that.
`parse_timestamp` tries `DateTime::parse_from_rfc3339` first, then
`NaiveDateTime::parse_from_str` with `"%Y-%m-%dT%H:%M:%S"`, treating the naive
form as UTC and saying so in the doc comment — ETS4 writes local time with no
offset, so this is a recorded assumption, not a fact.

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-etsproj values`
Expected: PASS, 5 tests.

- [ ] **Step 5: Run the gate and commit**

```bash
git add crates/knx-etsproj/src
git commit -m "knx-etsproj: attribute value conversions

Accepts both boolean spellings and both timestamp forms so that the same
converter serves schema 11 and schema 23. RefId decomposition keeps the
application program id, whose own text can contain an O-token, out of the
object number."
```

---

### Task 9: Stage 4 — validation

**Files:**
- Create: `crates/knx-etsproj/src/validate.rs`

**Interfaces:**
- Consumes: `SourceDocument` (Task 5).
- Produces:
  ```rust
  pub struct ValidationOutput {
      pub errors: Vec<SourceProblem>,
      pub warnings: Vec<SourceProblem>,
  }
  pub struct SourceProblem { pub xpath: String, pub id: Option<String>, pub detail: ProblemDetail }
  pub enum ProblemDetail {
      DuplicateId { kind: &'static str, id: String },
      DanglingReference { kind: &'static str, from: String, to: String },
      DuplicateIndividualAddress { address: String, devices: Vec<String> },
      DuplicateGroupAddress { address: String, ids: Vec<String> },
      GroupAddressOutsideItsRange { id: String, address: String, range: String },
      MissingRequiredAttribute { name: &'static str },
  }
  pub fn validate(document: &SourceDocument) -> ValidationOutput;
  ```
  Task 14 folds `errors` and `warnings` into the `ImportReport`.

Validation is structural and reports; it never modifies the document and never
aborts the import. A project with a dangling reference still opens — the
reference is reported and the link is dropped by the mapper, which is itself a
report entry.

The distinction: an **error** is data the mapper cannot use (a dangling
`GroupAddressRefId`, a duplicate `Id`); a **warning** is data the mapper can
use but that is suspicious (two devices on the same individual address, a group
address outside the range that contains it).

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn the_reference_project_validates_clean() {
    let doc = reference_source_document();
    let out = validate(&doc);
    assert_eq!(out.errors, vec![]);
    assert_eq!(out.warnings, vec![]);
}

#[test]
fn a_dangling_group_address_reference_is_an_error_not_a_panic() {
    let mut doc = minimal_source_document();
    doc.installations[0].areas[0].lines[0].devices[0].com_objects[0].sends =
        vec!["P-0001-0_GA-999".into()];
    let out = validate(&doc);
    assert!(matches!(
        out.errors[0].detail,
        ProblemDetail::DanglingReference { kind: "GroupAddressRefId", .. }
    ));
}

#[test]
fn two_devices_on_one_individual_address_is_a_warning() {
    let mut doc = minimal_source_document();
    let line = &mut doc.installations[0].areas[0].lines[0];
    let mut clone = line.devices[0].clone();
    clone.id = "P-0001-0_DI-2".into();
    line.devices.push(clone);
    let out = validate(&doc);
    assert!(matches!(
        out.warnings[0].detail,
        ProblemDetail::DuplicateIndividualAddress { .. }
    ));
    assert_eq!(out.errors, vec![]);
}

#[test]
fn a_duplicate_element_id_is_an_error() {
    let mut doc = minimal_source_document();
    let inst = &mut doc.installations[0];
    let mut clone = inst.group_ranges[0].children[0].addresses[0].clone();
    inst.group_ranges[0].children[0].addresses.push(clone);
    let out = validate(&doc);
    assert!(matches!(
        out.errors[0].detail,
        ProblemDetail::DuplicateId { kind: "GroupAddress", .. }
    ));
}

#[test]
fn an_address_outside_its_enclosing_range_is_a_warning() {
    let mut doc = minimal_source_document();
    doc.installations[0].group_ranges[0].children[0].addresses[0].address = Some("9999".into());
    let out = validate(&doc);
    assert!(matches!(
        out.warnings[0].detail,
        ProblemDetail::GroupAddressOutsideItsRange { .. }
    ));
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-etsproj validate`
Expected: FAIL, `cannot find function validate`.

- [ ] **Step 3: Implement `validate.rs`**

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-etsproj validate`
Expected: PASS, 5 tests.

- [ ] **Step 5: Run the gate and commit**

```bash
git add crates/knx-etsproj/src
git commit -m "knx-etsproj: validate the source document

Structural checks and reference resolution that report rather than abort. The
reference project validates clean."
```

---

### Task 10: Stage 5 — the mapper

**Files:**
- Create: `crates/knx-etsproj/src/map.rs`

**Interfaces:**
- Consumes: `SourceDocument` (Task 5), `values.rs` (Task 8),
  `ValidationOutput` (Task 9), `knx_core` (Tasks 1 and 2).
- Produces:
  ```rust
  pub struct MapOutput {
      pub project: knx_core::Project,
      pub retained: Vec<RetainedAttribute>,  // known-but-not-modelled, for export
      pub problems: Vec<MapProblem>,
      pub counts: EntityCounts,
  }
  pub struct MapProblem { pub xpath: String, pub detail: MapProblemDetail }
  pub enum MapProblemDetail {
      Value(ValueError),
      UnresolvedReference { kind: &'static str, target: String },
      DroppedLink { com_object: String, group_address: String },
  }
  pub struct EntityCounts { /* read and mapped, per entity type */ }
  pub fn map(document: &SourceDocument, source_path: &str) -> MapOutput;
  ```
  Task 11 infers datapoint types on the result; Task 14 reports `counts` and
  `problems`; Task 18 exports `project` and `retained`.

Mapping rules, each of which is a test below:

- Every value read from `0.xml` that lands on a `Resolved<T>` or `Override<T>`
  carries `Layer::Instance`. Nothing else can be true: the product database
  does not exist yet, so no `Program` value is available to override.
- An attribute present with an empty value becomes `Override::Empty`, an absent
  attribute `Override::Absent`. This is where the 497 empty `DatapointType`
  attributes are kept distinct from the 149 absent ones.
- A device's individual address is composed from its enclosing `Area/@Address`,
  `Line/@Address` and its own `@Address`. A device with no `@Address` — the one
  unassigned device — gets `None`, not a fabricated zero.
- Devices under `UnassignedDevices` go into `Topology::unassigned` and are
  owned by `Devices` exactly like the others. Both counts include them.
- A `ValueError` never aborts the mapping. The field is left unset, the problem
  is recorded, and the rest of the entity still maps.
- `Installation/@BCUKey`, `@SplitType`, `ProjectInformation`'s tool-state
  attributes, and `BusAccess` are known but not modelled: they go to
  `retained`, and from there to the opaque store.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn every_imported_value_carries_the_instance_layer() {
    let out = map(&reference_source_document(), "P-0512/0.xml");
    let com = out.project.devices
        .iter()
        .flat_map(|d| d.com_objects.iter())
        .filter_map(|id| out.project.devices.com_object(*id))
        .find(|c| c.dpt.value().is_some())
        .unwrap();
    assert_eq!(com.dpt.layer(), Some(Layer::Instance));
    assert!(com.dpt.layer().unwrap().is_exported());
}

#[test]
fn an_empty_datapoint_type_maps_to_empty_and_an_absent_one_to_absent() {
    let out = map(&reference_source_document(), "P-0512/0.xml");
    let mut empty = 0usize;
    let mut absent = 0usize;
    let mut valued = 0usize;
    for id in out.project.devices.iter().flat_map(|d| d.com_objects.clone()) {
        match out.project.devices.com_object(id).unwrap().dpt {
            Override::Empty => empty += 1,
            Override::Absent => absent += 1,
            Override::Value(_) => valued += 1,
        }
    }
    assert_eq!(empty, 497);
    assert_eq!(valued, 261);
    assert_eq!(absent, 149);
    assert_eq!(empty + valued + absent, 907);
}

#[test]
fn a_device_address_is_composed_from_its_area_and_line() {
    let out = map(&reference_source_document(), "P-0512/0.xml");
    let d = out.project.devices.iter().find(|d| d.name == "PM - Gardrobe").unwrap();
    assert_eq!(d.address.unwrap().to_string(), "1.1.1");
}

#[test]
fn the_unassigned_device_keeps_no_address_and_is_still_owned() {
    let out = map(&reference_source_document(), "P-0512/0.xml");
    let installation = &out.project.installations[0];
    assert_eq!(installation.topology.unassigned.len(), 1);
    let id = installation.topology.unassigned[0];
    assert!(out.project.devices.get(id).unwrap().address.is_none());
    assert_eq!(out.project.devices.iter().count(), 36);
}

#[test]
fn a_bad_attribute_value_is_reported_and_the_rest_of_the_entity_still_maps() {
    let mut doc = minimal_source_document();
    doc.installations[0].areas[0].lines[0].devices[0].last_modified = Some("not a date".into());
    let out = map(&doc, "P-0001/0.xml");
    assert!(matches!(out.problems[0].detail, MapProblemDetail::Value(_)));
    let d = out.project.devices.iter().next().unwrap();
    assert_eq!(d.name, "D");
    assert!(d.commissioning.last_modified.is_none());
}

#[test]
fn known_but_unmodelled_attributes_are_retained_for_export() {
    let out = map(&reference_source_document(), "P-0512/0.xml");
    assert!(out.retained.iter().any(|a| a.name == "BCUKey" && a.value == "4294967295"));
    assert!(out.retained.iter().any(|a| a.name == "SplitType"));
}

#[test]
fn links_keep_their_direction() {
    let out = map(&reference_source_document(), "P-0512/0.xml");
    let (send, receive) = out.project.devices
        .iter()
        .flat_map(|d| d.com_objects.clone())
        .filter_map(|id| out.project.devices.com_object(id))
        .flat_map(|c| c.links.iter())
        .fold((0usize, 0usize), |(s, r), l| match l.direction {
            Direction::Send => (s + 1, r),
            Direction::Receive => (s, r + 1),
        });
    assert_eq!((send, receive), (569, 27));
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-etsproj map`
Expected: FAIL, `cannot find function map`.

- [ ] **Step 3: Implement `map.rs`**

Two passes. The first allocates ids for group ranges, group addresses,
areas, lines, building parts and devices, and builds
`BTreeMap<String, XxxId>` lookup tables keyed by the ETS id. The second maps
the entities, resolving `RefId` strings through those tables. Two passes rather
than one because `Connectors/Send` may reference a group address defined later
in the document.

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-etsproj map`
Expected: PASS, 7 tests.

- [ ] **Step 5: Run the gate and commit**

```bash
git add crates/knx-etsproj/src
git commit -m "knx-etsproj: map the source document into the core model

Every imported value carries Layer::Instance. Empty and absent attributes stay
distinct: 497 empty, 261 valued and 149 absent DatapointType attributes across
the reference project's 907 communication object instances."
```

---

### Task 11: Datapoint type inference and conflicts

**Files:**
- Create: `crates/knx-etsproj/src/infer.rs`

**Interfaces:**
- Consumes: `MapOutput` (Task 10).
- Produces:
  ```rust
  pub struct InferenceOutput {
      pub inferred: Vec<InferredValue>,
      pub conflicts: Vec<Conflict>,
  }
  pub struct InferredValue { pub group_address: GroupAddressId, pub dpt: DptRef, pub from: Vec<ComObjectInstanceId> }
  pub struct Conflict { pub group_address: GroupAddressId, pub candidates: Vec<(DptRef, Vec<ComObjectInstanceId>)> }
  pub fn infer_group_address_dpts(project: &Project) -> InferenceOutput;
  ```
  Task 14 reports both; nothing else consumes them, and in particular export
  never does.

IMPORT_EXPORT §7: where a group address has no datapoint type of its own but
its linked communication objects agree on one, the derived value is used for
display and marked `Layer::Inferred`. Where linked objects disagree, that is a
`Conflict`: the address keeps no datapoint type, and there is no majority vote
and no first-wins rule.

`GroupAddressEntry` has no datapoint type field by design (DATA_MODEL §9) — the
datapoint type is a property of the linked objects, not of the address — so
inference produces a side table rather than mutating the model. That also makes
the "never exported" rule structural: there is nothing on the entity for export
to write.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn an_address_whose_linked_objects_agree_gets_an_inferred_type() {
    let project = project_with_two_links(DptRef { main: 1, sub: Some(1) },
                                         DptRef { main: 1, sub: Some(1) });
    let out = infer_group_address_dpts(&project);
    assert_eq!(out.inferred.len(), 1);
    assert_eq!(out.inferred[0].dpt, DptRef { main: 1, sub: Some(1) });
    assert_eq!(out.inferred[0].from.len(), 2);
    assert_eq!(out.conflicts, vec![]);
}

#[test]
fn an_address_whose_linked_objects_disagree_is_a_conflict_with_no_winner() {
    let project = project_with_two_links(DptRef { main: 1, sub: Some(1) },
                                         DptRef { main: 5, sub: Some(1) });
    let out = infer_group_address_dpts(&project);
    assert_eq!(out.inferred, vec![]);
    assert_eq!(out.conflicts.len(), 1);
    assert_eq!(out.conflicts[0].candidates.len(), 2);
}

#[test]
fn an_orphan_address_infers_nothing_and_is_not_a_conflict() {
    let project = project_with_orphan_address();
    let out = infer_group_address_dpts(&project);
    assert_eq!(out.inferred, vec![]);
    assert_eq!(out.conflicts, vec![]);
}

#[test]
fn the_reference_project_has_no_datapoint_type_conflicts() {
    let out = infer_group_address_dpts(&reference_project());
    assert_eq!(out.conflicts, vec![], "conflicts are not present in this sample");
    // 110 of 514 addresses have no linked object at all (RESEARCH §6.1), so at
    // most 404 can be inferred, and only those whose links state a type are.
    assert!(out.inferred.len() <= 404);
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-etsproj infer`
Expected: FAIL, `cannot find function infer_group_address_dpts`.

- [ ] **Step 3: Implement `infer.rs`**

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-etsproj infer`
Expected: PASS, 4 tests.

- [ ] **Step 5: Run the gate and commit**

```bash
git add crates/knx-etsproj/src
git commit -m "knx-etsproj: infer group address datapoint types and detect conflicts

Agreement among linked objects yields an inferred value; disagreement yields a
conflict with no winner. Inference lives in a side table, so export cannot
write it back by accident."
```

---

### Task 12: The opaque passthrough store — the values

**Files:**
- Create: `crates/knx-etsproj/src/opaque.rs`

**Interfaces:**
- Consumes: `Container` (Task 3), `RetainedAttribute` / `RetainedElement`
  (Task 5), `MapOutput::retained` (Task 10).
- Produces:
  ```rust
  pub struct OpaqueEntry {
      pub source_path: String,
      /// Empty for a whole container entry; an XPath for a retained fragment.
      pub xpath: String,
      pub kind: OpaqueKind,
      /// Attribute name for `RetainedAttribute`, element name for
      /// `RetainedElement`, empty for a container entry.
      pub name: String,
      pub bytes: Vec<u8>,
      pub sha256: String,
  }
  pub enum OpaqueKind {
      ContainerEntry,      // any entry we do not regenerate
      ManufacturerData,    // <M-xxxx>/*, moves to the product database in Session 4
      Baggage,             // <M-xxxx>/Baggages/*  — never executed
      BinaryData,          // <P-xxxx>/BinaryData/*.dat
      ExtraData,           // <P-xxxx>/ExtraData/*.azp, *.rbg
      Signature,           // *.signature — cannot be regenerated
      MasterData,          // knx_master.xml
      RetainedAttribute,   // known or unknown attribute the model does not carry
      RetainedElement,     // element the model does not carry, raw bytes
  }
  pub fn collect_container_entries(
      container: &mut Container,
      regenerated: &[&str],
  ) -> Result<Vec<OpaqueEntry>, ContainerError>;
  pub fn from_retained_attribute(source_path: &str, a: &RetainedAttribute) -> OpaqueEntry;
  pub fn from_retained_element(source_path: &str, e: &RetainedElement) -> OpaqueEntry;
  pub fn sha256_hex(bytes: &[u8]) -> String;
  ```
  Task 13 persists these; Task 19 writes them back; Task 20 hashes them again.

This extends the four-column shape declared in IMPORT_EXPORT §5 with `xpath`
and `name`, because an entry is not always a whole file: a retained attribute
needs to say which element it belongs to and what it was called. Task 23
amends the document.

`ManufacturerData` is a Session 3 arrangement, not the destination. ADR-0005
and IMPORT_EXPORT §10 put manufacturer data in the shared product database,
stored once rather than once per project. That database arrives in Session 4.
Until it does, keeping the manufacturer entries in the opaque store is what
lets export write a complete container instead of one ETS could not read.
Task 23 records this as a limitation with the condition that lifts it.

- [ ] **Step 1: Add `sha2`**

Workspace: `sha2 = "0.10"`. Crate: `sha2.workspace = true`.

- [ ] **Step 2: Write failing tests**

```rust
#[test]
fn every_container_entry_except_the_regenerated_ones_is_collected() {
    let mut c = Container::open(reference_ets4_bytes()).unwrap();
    let entries = collect_container_entries(
        &mut c,
        &["P-0512/0.xml", "P-0512/Project.xml"],
    ).unwrap();
    assert_eq!(entries.len(), 36); // 38 archive entries less the two we rewrite
    assert!(entries.iter().all(|e| !e.sha256.is_empty()));
}

#[test]
fn entries_are_classified_by_where_they_sit_in_the_container() {
    let mut c = Container::open(reference_ets4_bytes()).unwrap();
    let entries = collect_container_entries(&mut c, &[]).unwrap();
    let kind = |p: &str| entries.iter().find(|e| e.source_path == p).unwrap().kind;
    assert_eq!(kind("knx_master.xml"), OpaqueKind::MasterData);
    assert_eq!(kind("P-0512.signature"), OpaqueKind::Signature);
    assert_eq!(kind("M-0008/Baggages/econEts3.dll"), OpaqueKind::Baggage);
    assert_eq!(kind("M-0008/Catalog.xml"), OpaqueKind::ManufacturerData);
    assert_eq!(
        kind("P-0512/BinaryData/2868e24a-9fc3-4099-82f0-3f535bde8fc1.dat"),
        OpaqueKind::BinaryData
    );
    assert_eq!(kind("P-0512/ExtraData/20001.azp"), OpaqueKind::ExtraData);
}

#[test]
fn the_baggage_dll_is_copied_byte_for_byte() {
    let mut c = Container::open(reference_ets4_bytes()).unwrap();
    let entries = collect_container_entries(&mut c, &[]).unwrap();
    let dll = entries.iter().find(|e| e.source_path.ends_with("econEts3.dll")).unwrap();
    assert_eq!(dll.bytes.len(), 641536);
    assert_eq!(&dll.bytes[..2], b"MZ"); // a PE image, and we do nothing with it
    assert_eq!(dll.sha256, sha256_hex(&dll.bytes));
}

#[test]
fn a_retained_attribute_carries_its_element_path_and_name() {
    let a = RetainedAttribute {
        xpath: "/KNX/Project/Installations/Installation".into(),
        name: "BCUKey".into(),
        value: "4294967295".into(),
    };
    let e = from_retained_attribute("P-0512/0.xml", &a);
    assert_eq!(e.kind, OpaqueKind::RetainedAttribute);
    assert_eq!(e.name, "BCUKey");
    assert_eq!(e.bytes, b"4294967295");
    assert_eq!(e.sha256, sha256_hex(b"4294967295"));
}
```

- [ ] **Step 3: Run them and watch them fail**

Run: `cargo test -p knx-etsproj opaque`
Expected: FAIL, `cannot find function collect_container_entries`.

- [ ] **Step 4: Implement `opaque.rs`**

- [ ] **Step 5: Run them and watch them pass**

Run: `cargo test -p knx-etsproj opaque`
Expected: PASS, 4 tests.

- [ ] **Step 6: Run the gate and commit**

```bash
git add crates/knx-etsproj Cargo.toml Cargo.lock
git commit -m "knx-etsproj: collect opaque entries with their hashes

Everything the model does not carry is kept as bytes with a SHA-256: container
entries, vendor baggage, signatures, and the attributes and elements the model
deliberately does not model. Nothing is executed and nothing is interpreted."
```

---

### Task 13: `knx-store` — the opaque table at schema version 2

**Files:**
- Create: `crates/knx-store/src/opaque.rs`
- Modify: `crates/knx-store/src/migration.rs`, `crates/knx-store/src/lib.rs`
- Create: `crates/knx-store/fixtures/v2-empty.sqlite`

**Interfaces:**
- Produces:
  ```rust
  pub struct StoredOpaqueEntry {
      pub source_path: String,
      pub xpath: String,
      pub kind: String,
      pub name: String,
      pub bytes: Vec<u8>,
      pub sha256: String,
  }
  pub fn insert_opaque(conn: &Connection, entries: &[StoredOpaqueEntry]) -> Result<usize, rusqlite::Error>;
  pub fn load_opaque(conn: &Connection) -> Result<Vec<StoredOpaqueEntry>, rusqlite::Error>;
  ```
  Task 22 calls both.

`knx-store` does not depend on `knx-etsproj`: the conversion between
`OpaqueEntry` and `StoredOpaqueEntry` happens in `knx-app`, which is the only
crate that knows both. Keeping the dependency out is what stops the storage
schema from being defined by the import format.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn a_fresh_file_migrates_to_version_two_and_has_the_opaque_table() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(v, 2);
    assert_eq!(load_opaque(&conn).unwrap(), vec![]);
}

#[test]
fn opaque_bytes_survive_a_round_trip_through_sqlite_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    let entry = StoredOpaqueEntry {
        source_path: "M-0008/Baggages/econEts3.dll".into(),
        xpath: String::new(),
        kind: "Baggage".into(),
        name: String::new(),
        bytes: vec![0x4d, 0x5a, 0x00, 0xff, 0x00],
        sha256: "abc".into(),
    };
    insert_opaque(&conn, std::slice::from_ref(&entry)).unwrap();
    assert_eq!(load_opaque(&conn).unwrap(), vec![entry]);
}

#[test]
fn the_frozen_v1_fixture_migrates_forward_to_v2() {
    // Copied, not opened in place: a migration test must not mutate its fixture.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v1.sqlite");
    std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v1-empty.sqlite"), &path).unwrap();
    let conn = open_and_migrate(&path).unwrap();
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(v, 2);
    assert_eq!(load_opaque(&conn).unwrap(), vec![]);
}

#[test]
fn the_frozen_v2_fixture_still_opens() {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v2-empty.sqlite");
    let conn = open_and_migrate(std::path::Path::new(fixture)).unwrap();
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(v, 2);
}
```

The existing test `the_frozen_v1_fixture_still_opens` opens the fixture in
place and now migrates it, which would rewrite a committed file. Change it to
copy first, the same way the new test does.

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-store`
Expected: FAIL, `cannot find function load_opaque`, and the version assertion
fails at 1.

- [ ] **Step 3: Implement the migration and `opaque.rs`**

```sql
CREATE TABLE opaque_entry (
    id          INTEGER PRIMARY KEY,
    source_path TEXT NOT NULL,
    xpath       TEXT NOT NULL,
    kind        TEXT NOT NULL,
    name        TEXT NOT NULL,
    bytes       BLOB NOT NULL,
    sha256      TEXT NOT NULL
) STRICT;
CREATE INDEX opaque_entry_source_path ON opaque_entry (source_path);
```

Raise `CURRENT_SCHEMA_VERSION` to 2 in both `knx-store` and `knx-core`, and
append `migrate_v1_to_v2` to the chain. `insert_opaque` runs inside one
transaction with a prepared statement — 36 entries totalling 22 MB go in at
once, and a partial insert would leave a project file that cannot be exported.

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-store`
Expected: PASS.

- [ ] **Step 5: Freeze the v2 fixture**

```bash
cargo test -p knx-store fresh_file_migrates_to_version_two -- --nocapture
# then generate the fixture deterministically:
cargo run -p xtask -- freeze-fixture crates/knx-store/fixtures/v2-empty.sqlite
```

Add the `freeze-fixture` subcommand to `xtask` if it does not exist: it calls
`open_and_migrate` on the given path, then `VACUUM`s so the file is minimal and
reproducible.

- [ ] **Step 6: Run the gate and commit**

```bash
git add crates/knx-store xtask/src crates/knx-core/src/project.rs
git commit -m "knx-store: add the opaque passthrough table at schema version 2

One transaction per import: a partial insert would leave a project file that
cannot be exported. knx-store stays free of any dependency on knx-etsproj."
```

---

### Task 14: Stage 6 — the import report

**Files:**
- Create: `crates/knx-etsproj/src/report.rs`

**Interfaces:**
- Consumes: `Detected` (Task 4), `ParseOutput::unknown` (Task 6),
  `ValidationOutput` (Task 9), `MapOutput` (Task 10), `InferenceOutput`
  (Task 11), `OpaqueEntry` (Task 12).
- Produces:
  ```rust
  pub struct ImportReport {
      pub source: SourceInfo,
      pub counts: EntityCounts,
      pub unknown: Vec<UnknownConstruct>,
      pub opaque: Vec<OpaqueSummary>,
      pub inferred: Vec<InferredValue>,
      pub conflicts: Vec<Conflict>,
      pub unsupported: Vec<UnsupportedFeature>,
      pub errors: Vec<ImportError>,
  }
  pub struct SourceInfo {
      pub file_name: String,
      pub file_size: u64,
      pub schema_version: u32,
      pub namespace: String,
      pub created_by: Option<String>,
      pub tool_version: Option<String>,
      pub namespace_disagreement: Option<String>,
  }
  pub struct EntityCounts { pub rows: Vec<EntityCount> }
  pub struct EntityCount { pub entity: String, pub read: u32, pub mapped: u32 }
  /// Bytes are not repeated in the report; the store holds them.
  pub struct OpaqueSummary { pub source_path: String, pub kind: String, pub size: u64, pub sha256: String, pub reason: String }
  pub struct UnsupportedFeature { pub what: String, pub consequence: String }
  impl ImportReport {
      pub fn to_json(&self) -> String;
      pub fn has_losses(&self) -> bool;
  }
  ```
  Task 15 assembles it; Task 22 prints and exports it.

The report is a deliverable, not a log: it is structured, serialisable, and its
`counts` carry read and mapped figures per entity type so a discrepancy is a
number rather than a suspicion.

`unsupported` is where the baggage rule surfaces: a device whose manufacturer
ships a `Baggages/*.dll` gets an `UnsupportedFeature` saying that part of its
configuration behaviour lives inside a vendor binary this application will
never execute, and that the device is therefore not fully editable here.

- [ ] **Step 1: Add `serde` and `serde_json`**

Workspace: `serde = { version = "1", features = ["derive"] }` and
`serde_json = "1"`. Crate: both, in `knx-etsproj` only.

- [ ] **Step 2: Write failing tests**

```rust
#[test]
fn counts_carry_both_read_and_mapped_figures() {
    let r = reference_report();
    let devices = r.counts.rows.iter().find(|c| c.entity == "DeviceInstance").unwrap();
    assert_eq!(devices.read, 36);
    assert_eq!(devices.mapped, 36);
}

#[test]
fn a_report_with_no_losses_says_so() {
    let r = reference_report();
    assert_eq!(r.errors, vec![]);
    assert_eq!(r.unknown, vec![]);
    assert!(!r.has_losses());
}

#[test]
fn the_json_form_round_trips_and_names_every_section() {
    let json = reference_report().to_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    for key in ["source", "counts", "unknown", "opaque", "inferred",
                "conflicts", "unsupported", "errors"] {
        assert!(v.get(key).is_some(), "missing report section {key}");
    }
    assert_eq!(v["source"]["schema_version"], 11);
}

#[test]
fn vendor_baggage_is_reported_as_unsupported() {
    let r = reference_report();
    let u = r.unsupported.iter().find(|u| u.what.contains("econEts3.dll")).unwrap();
    assert!(u.consequence.contains("not executed"));
}

#[test]
fn the_opaque_summary_does_not_repeat_the_bytes() {
    let json = reference_report().to_json();
    // 22 MB of manufacturer data must not end up inside a JSON report.
    assert!(json.len() < 512 * 1024);
}
```

- [ ] **Step 3: Run them and watch them fail**

Run: `cargo test -p knx-etsproj report`
Expected: FAIL, `cannot find type ImportReport`.

- [ ] **Step 4: Implement `report.rs`**

- [ ] **Step 5: Run them and watch them pass**

Run: `cargo test -p knx-etsproj report`
Expected: PASS, 5 tests.

- [ ] **Step 6: Run the gate and commit**

```bash
git add crates/knx-etsproj Cargo.toml Cargo.lock
git commit -m "knx-etsproj: the import report

Structured, serialisable, and carrying read-versus-mapped counts per entity
type. Opaque bytes stay in the store; the report summarises them."
```

---

### Task 15: The import orchestration

**Files:**
- Modify: `crates/knx-etsproj/src/lib.rs`

**Interfaces:**
- Consumes: every stage.
- Produces:
  ```rust
  pub struct ImportOutcome {
      pub project: knx_core::Project,
      pub opaque: Vec<OpaqueEntry>,
      pub report: ImportReport,
  }
  pub fn import_knxproj(path: &std::path::Path) -> Result<ImportOutcome, ImportFailure>;
  pub fn import_knxproj_bytes(bytes: Vec<u8>, file_name: &str) -> Result<ImportOutcome, ImportFailure>;
  pub enum ImportFailure {
      Io(std::io::Error),
      Container(ContainerError),
      Detect(DetectError),
      Parse(ParseError),
      NoKnownSchemaTable { version: u32 },
  }
  ```
  Tasks 16, 17, 20, 21 and 22 all call `import_knxproj`.

`Err` is reserved for a container that cannot be opened at all, a schema
version with no known-element table, or XML that cannot be read. Everything
else — a dangling reference, an unparsable timestamp, an unknown attribute — is
a report entry, because a project that is partly readable should open partly
rather than not at all.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn importing_the_reference_project_succeeds_with_a_clean_report() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    assert_eq!(out.report.source.schema_version, 11);
    assert_eq!(out.report.errors, vec![]);
    assert_eq!(out.project.info.name, "Unser Zuhause");
    assert_eq!(out.project.info.group_address_style, GroupAddressStyle::ThreeLevel);
}

#[test]
fn importing_the_ets6_project_fails_with_a_named_reason_not_wrong_data() {
    let err = import_knxproj(reference_ets6_path()).unwrap_err();
    assert!(matches!(err, ImportFailure::NoKnownSchemaTable { version: 23 }));
}

#[test]
fn the_opaque_entries_cover_every_container_entry_we_do_not_regenerate() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    assert_eq!(out.opaque.iter().filter(|e| e.xpath.is_empty()).count(), 36);
}
```

The second test is the honest outcome for schema 23 today. RESEARCH §3.3 lists
four load-bearing differences — `RefId` loses its prefix, links move to
attributes, `GroupObjectTree` becomes the authoritative object list, booleans
change spelling — and importing schema 23 through the schema-11 table would
undercount communication objects by about 24 per cent and read every boolean
flag as false. Failing with a named reason is better than producing that.

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-etsproj import`
Expected: FAIL, `cannot find function import_knxproj`.

- [ ] **Step 3: Implement the orchestration in `lib.rs`**

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-etsproj import`
Expected: PASS, 3 tests.

- [ ] **Step 5: Run the gate and commit**

```bash
git add crates/knx-etsproj/src
git commit -m "knx-etsproj: wire the six stages into import_knxproj

Err is reserved for a container that cannot be read at all. A schema version
with no known-element table fails by name rather than producing wrong data."
```

---

### Task 16: The golden test

**Files:**
- Create: `crates/knx-etsproj/tests/golden_reference_project.rs`

**Interfaces:**
- Consumes: `import_knxproj` (Task 15).

Every count in this test comes from ROADMAP Session 3 and RESEARCH §3, and each
was re-measured against the file while this plan was written. The test is the
regression net for the whole pipeline: if any stage starts dropping things, one
of these numbers moves.

- [ ] **Step 1: Write the test**

```rust
#[test]
fn the_reference_project_imports_with_the_measured_counts() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let p = &out.project;
    let inst = &p.installations[0];

    assert_eq!(p.installations.len(), 1);
    assert_eq!(inst.topology.areas.len(), 1);
    assert_eq!(inst.topology.lines.len(), 1);

    // 35 devices on the line plus the one unassigned device xknxproject loses.
    assert_eq!(p.devices.iter().count(), 36);
    assert_eq!(inst.topology.unassigned.len(), 1);

    assert_eq!(inst.group_ranges.len(), 35);
    assert_eq!(inst.group_ranges.iter().filter(|r| r.parent.is_none()).count(), 7);
    assert_eq!(inst.group_addresses.len(), 514);
    assert_eq!(inst.group_addresses.iter().filter(|g| g.central).count(), 2);
    assert_eq!(inst.group_addresses.iter().filter(|g| g.unfiltered).count(), 2);

    assert_eq!(inst.buildings.len(), 22);
    assert_eq!(
        inst.buildings.iter().filter(|b| b.kind == BuildingPartType::Room).count(),
        14
    );
    assert_eq!(inst.buildings.iter().map(|b| b.devices.len()).sum::<usize>(), 29);

    assert_eq!(inst.parameters.len(), 1390);
    assert_eq!(
        inst.parameters.iter().filter(|p| p.source.ets_id.contains("_UP-")).count(),
        216
    );

    let coms: Vec<_> = p.devices.iter()
        .flat_map(|d| d.com_objects.clone())
        .filter_map(|id| p.devices.com_object(id))
        .collect();
    assert_eq!(coms.len(), 907);

    let (send, receive) = coms.iter().flat_map(|c| c.links.iter())
        .fold((0, 0), |(s, r), l| match l.direction {
            Direction::Send => (s + 1, r),
            Direction::Receive => (s, r + 1),
        });
    assert_eq!((send, receive), (569, 27));

    assert_eq!(coms.iter().filter(|c| matches!(c.dpt, Override::Value(_))).count(), 261);
    assert_eq!(coms.iter().filter(|c| matches!(c.dpt, Override::Empty)).count(), 497);
    assert_eq!(coms.iter().filter(|c| c.description.is_present()).count(), 691);
    assert_eq!(coms.iter().filter(|c| c.text.is_present()).count(), 121);
    assert_eq!(coms.iter().filter(|c| c.flags.read.is_present()).count(), 39);
    assert_eq!(coms.iter().filter(|c| c.flags.update.is_present()).count(), 30);
    assert_eq!(coms.iter().filter(|c| c.flags.transmit.is_present()).count(), 27);
    assert_eq!(coms.iter().filter(|c| c.flags.write.is_present()).count(), 18);
    assert_eq!(coms.iter().filter(|c| c.flags.communication.is_present()).count(), 8);

    assert_eq!(p.devices.iter().flat_map(|d| d.binary_data.iter()).count(), 3);
}

#[test]
fn nothing_in_the_reference_project_is_unknown_or_lost() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    assert_eq!(out.report.unknown, vec![]);
    assert_eq!(out.report.errors, vec![]);
    assert!(!out.report.has_losses());
    for row in &out.report.counts.rows {
        assert_eq!(row.read, row.mapped, "{} lost entities in mapping", row.entity);
    }
}
```

- [ ] **Step 2: Run it**

Run: `cargo test -p knx-etsproj --test golden_reference_project`
Expected: PASS. If a count is off, the pipeline is wrong, not the number —
every one of these was measured against the file.

- [ ] **Step 3: Run the gate and commit**

```bash
git add crates/knx-etsproj/tests
git commit -m "knx-etsproj: golden test over the reference project

Locks every measured count from RESEARCH §3, including the 36th device
xknxproject loses and the 216 union parameters."
```

---

### Task 17: The oracle comparison

**Files:**
- Create: `crates/knx-etsproj/tests/oracle_xknxproject.rs`

**Interfaces:**
- Consumes: `import_knxproj` (Task 15), the committed `project_dump.json`.

`xknxproject` is a good reader and a lossy one (RESEARCH §7.1). It is used here
as a second opinion on the parts it does read, and the parts it is known to
lose are asserted as differences rather than ignored — so that if a future
version of the oracle starts reading them, the test says so.

- [ ] **Step 1: Write the test**

```rust
mod support;
use support::*;

fn oracle() -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(oracle_dump_path()).unwrap()).unwrap()
}

#[test]
fn group_addresses_agree_with_the_oracle_by_address_and_name() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let oracle = oracle();
    let theirs = oracle["group_addresses"].as_object().unwrap();
    assert_eq!(theirs.len(), 514);

    let style = out.project.info.group_address_style;
    for ga in &out.project.installations[0].group_addresses {
        let key = ga.address.format(style);
        let entry = theirs.get(&key).unwrap_or_else(|| panic!("oracle lacks {key}"));
        assert_eq!(entry["name"].as_str().unwrap(), ga.name);
    }
}

#[test]
fn devices_agree_with_the_oracle_except_for_the_one_it_loses() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let oracle = oracle();
    let theirs = oracle["devices"].as_object().unwrap();
    assert_eq!(theirs.len(), 35);
    assert_eq!(out.project.devices.iter().count(), 36);

    let addressed: Vec<_> = out.project.devices.iter().filter(|d| d.address.is_some()).collect();
    assert_eq!(addressed.len(), 35);
    for d in addressed {
        let key = d.address.unwrap().to_string();
        assert!(theirs.contains_key(&key), "oracle lacks device {key}");
    }
}

#[test]
fn the_oracle_still_loses_exactly_what_research_measured() {
    // If any of these starts holding data, RESEARCH §7.1 needs updating and
    // the comparison above can be widened.
    let oracle = oracle();
    assert!(oracle.get("parameters").is_none());
    assert_eq!(oracle["functions"].as_object().map(|f| f.len()), Some(0));

    // The oracle exposes only communication objects that carry a group
    // address link: 570 of the 907 instances in the file. The 337 unlinked
    // instances are exactly the ones a parameter editor needs.
    assert_eq!(oracle["communication_objects"].as_object().unwrap().len(), 570);

    // Group ranges are exposed as the 7 main ranges; nesting is not.
    assert_eq!(oracle["group_ranges"].as_object().unwrap().len(), 7);
}

#[test]
fn the_project_metadata_agrees_with_the_oracle() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let oracle = oracle();
    assert_eq!(oracle["info"]["schema_version"].as_str().unwrap(), "11");
    assert_eq!(oracle["info"]["project_id"].as_str().unwrap(), out.project.info.project_id);
    assert_eq!(oracle["info"]["name"].as_str().unwrap(), out.project.info.name);
    assert_eq!(oracle["info"]["group_address_style"].as_str().unwrap(), "ThreeLevel");
}

#[test]
fn our_linked_communication_objects_match_the_oracle_count() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let linked = out.project.devices.iter()
        .flat_map(|d| d.com_objects.clone())
        .filter_map(|id| out.project.devices.com_object(id))
        .filter(|c| !c.links.is_empty())
        .count();
    assert_eq!(linked, 570);
}
```

- [ ] **Step 2: Run it**

Run: `cargo test -p knx-etsproj --test oracle_xknxproject`
Expected: PASS, 5 tests.

- [ ] **Step 3: Run the gate and commit**

```bash
git add crates/knx-etsproj/tests
git commit -m "knx-etsproj: cross-check the import against xknxproject

Compares only what the oracle is not known to lose, and asserts the known
losses so that a change in the oracle is visible rather than silent."
```

---

### Task 18: Export — the schema-11 XML writers

**Files:**
- Create: `crates/knx-etsproj/src/export/mod.rs` (module declaration only in
  this task)
- Create: `crates/knx-etsproj/src/export/schema11.rs`

**Interfaces:**
- Consumes: `knx_core::Project` (Tasks 1, 2), `OpaqueEntry` (Task 12).
- Produces:
  ```rust
  pub fn write_installation_xml(project: &Project, opaque: &[OpaqueEntry]) -> Result<Vec<u8>, ExportError>;
  pub fn write_project_xml(project: &Project, opaque: &[OpaqueEntry]) -> Result<Vec<u8>, ExportError>;
  pub enum ExportError {
      Xml(String),
      MissingProjectId,
      UnsupportedSchemaVersion(u32),
  }
  ```
  Task 19 packs the results into a container.

Writing rules:

- Attribute values come from the model when the model carries them, and from
  the retained opaque attributes — matched on `(xpath, name)` — when it does
  not. That is how `BCUKey`, `SplitType`, `ProjectTracingLevel` and `BusAccess`
  come back.
- An `Override::Value` is written when `Layer::is_exported()` is true for its
  layer, `Override::Empty` is written as an empty attribute, and
  `Override::Absent` writes no attribute at all.
- Nothing marked `Layer::Inferred`, `Layer::Program` or `Layer::ProgramRef` is
  ever written. There is no branch that could.
- Element and attribute order follows the source order of schema 11, because a
  reader that depends on order should not be given a reason to fail. Byte
  equality is still not attempted or claimed.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn an_exported_installation_is_well_formed_and_carries_the_namespace() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let xml = write_installation_xml(&out.project, &out.opaque).unwrap();
    let text = String::from_utf8(xml).unwrap();
    assert!(text.contains(r#"xmlns="http://knx.org/xml/project/11""#));
    assert!(text.contains(r#"<Project Id="P-0512">"#));
    // It parses back with our own parser, which is the only reader we control.
    parse_installation(text.as_bytes(), "P-0512/0.xml", known_schema(11).unwrap()).unwrap();
}

#[test]
fn an_empty_override_is_written_as_an_empty_attribute() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let xml = String::from_utf8(write_installation_xml(&out.project, &out.opaque).unwrap()).unwrap();
    assert_eq!(xml.matches(r#"DatapointType="""#).count(), 497);
}

#[test]
fn retained_attributes_come_back_on_their_own_elements() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let xml = String::from_utf8(write_installation_xml(&out.project, &out.opaque).unwrap()).unwrap();
    assert!(xml.contains(r#"BCUKey="4294967295""#));
    assert!(xml.contains(r#"Name='GATEWAY-NAME';IpAddr='192.0.2.1'"#));
}

#[test]
fn an_inferred_value_is_never_written() {
    let mut out = import_knxproj(reference_ets4_path()).unwrap();
    // Force an inferred datapoint type onto an instance that had none.
    let id = *out.project.devices.iter().next().unwrap().com_objects.first().unwrap();
    out.project.devices.com_object_mut(id).unwrap().dpt = Override::Value(Resolved {
        value: DptRef { main: 99, sub: Some(99) },
        layer: Layer::Inferred,
    });
    let xml = String::from_utf8(write_installation_xml(&out.project, &out.opaque).unwrap()).unwrap();
    assert!(!xml.contains("DPST-99-99"));
}

#[test]
fn the_project_xml_carries_the_group_address_style() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let xml = String::from_utf8(write_project_xml(&out.project, &out.opaque).unwrap()).unwrap();
    assert!(xml.contains(r#"GroupAddressStyle="ThreeLevel""#));
    assert!(xml.contains(r#"Name="Unser Zuhause""#));
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-etsproj export`
Expected: FAIL, `cannot find function write_installation_xml`.

- [ ] **Step 3: Implement `export/schema11.rs`**

`quick_xml::Writer` over a `Vec<u8>`, indented two spaces to match ETS's own
output. A `retained_lookup: BTreeMap<(&str, &str), &str>` built once from the
opaque entries of kind `RetainedAttribute` keyed by `(xpath, name)`.

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-etsproj export`
Expected: PASS, 5 tests.

- [ ] **Step 5: Run the gate and commit**

```bash
git add crates/knx-etsproj/src
git commit -m "knx-etsproj: write schema-11 0.xml and Project.xml

Only Instance and UserEdit layers reach the file; empty overrides come back as
empty attributes, and retained attributes come back on their own elements."
```

---

### Task 19: Export — the container

**Files:**
- Modify: `crates/knx-etsproj/src/export/mod.rs`
- Modify: `crates/knx-etsproj/src/lib.rs`

**Interfaces:**
- Consumes: Task 18's writers, `OpaqueEntry` (Task 12).
- Produces:
  ```rust
  pub struct ExportOutcome { pub bytes: Vec<u8>, pub warnings: Vec<ExportWarning> }
  pub enum ExportWarning {
      /// Always present. Every export this application produces is unsigned.
      Unsigned { detail: String },
      /// A signature entry was copied through unchanged; it no longer matches
      /// the content it signs.
      StaleSignature { source_path: String },
      ManufacturerDataFromOpaqueStore { entries: usize },
  }
  pub fn export_knxproj(project: &Project, opaque: &[OpaqueEntry]) -> Result<ExportOutcome, ExportError>;
  ```
  Task 20 round-trips through it; Task 22 exposes it on the CLI.

`ExportWarning::Unsigned` is constructed unconditionally at the top of
`export_knxproj`, before anything can fail, so no code path produces an export
without it. Signatures are RSA over manufacturer and project data and cannot be
regenerated without KNX signing keys; copying the old ones through keeps the
container's shape but they no longer match, and `StaleSignature` says so per
entry.

Whether ETS re-imports an unsigned third-party file is untested (risk R9). The
warning text says that in as many words, and stays until someone has tried it
against a real ETS installation.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn every_export_is_unsigned_and_says_so() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let exported = export_knxproj(&out.project, &out.opaque).unwrap();
    let unsigned = exported.warnings.iter()
        .find(|w| matches!(w, ExportWarning::Unsigned { .. }))
        .expect("no export may be produced without the unsigned warning");
    let ExportWarning::Unsigned { detail } = unsigned else { unreachable!() };
    assert!(detail.contains("untested"));
}

#[test]
fn the_exported_container_holds_every_entry_the_source_had() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let exported = export_knxproj(&out.project, &out.opaque).unwrap();
    let container = Container::open(exported.bytes).unwrap();
    assert_eq!(container.entries().len(), 38);
    assert!(container.find("P-0512/0.xml").is_some());
    assert!(container.find("M-0008/Baggages/econEts3.dll").is_some());
}

#[test]
fn copied_signatures_are_reported_as_stale() {
    let out = import_knxproj(reference_ets4_path()).unwrap();
    let exported = export_knxproj(&out.project, &out.opaque).unwrap();
    let stale = exported.warnings.iter()
        .filter(|w| matches!(w, ExportWarning::StaleSignature { .. }))
        .count();
    assert_eq!(stale, 5); // four manufacturer signatures and one project signature
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-etsproj export`
Expected: FAIL, `cannot find function export_knxproj`.

- [ ] **Step 3: Implement `export/mod.rs`**

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-etsproj export`
Expected: PASS, 8 tests in the module.

- [ ] **Step 5: Run the gate and commit**

```bash
git add crates/knx-etsproj/src
git commit -m "knx-etsproj: write a .knxproj container back out

The unsigned warning is constructed before anything can fail, so no code path
produces an export without it. Copied signatures are reported stale per entry."
```

---

### Task 20: The three roundtrip guarantees

**Files:**
- Create: `crates/knx-etsproj/src/compare.rs`
- Create: `crates/knx-etsproj/tests/roundtrip.rs`

**Interfaces:**
- Produces:
  ```rust
  /// The declared semantic-equality relation of ADR-0007 and IMPORT_EXPORT §9.
  ///
  /// Ordering is normalized and synthetic ids are excluded; everything else is
  /// compared. Changing this type changes what fidelity means in this
  /// repository, which is why it is a declared type and not a bag of ad-hoc
  /// assertions inside a test.
  #[derive(Debug, PartialEq, Eq)]
  pub struct SemanticProject {
      pub info: SemanticInfo,
      /// Keyed by installation index, which is stable: installations are
      /// written in source order and read back in the same order.
      pub installations: Vec<SemanticInstallation>,
  }

  #[derive(Debug, PartialEq, Eq)]
  pub struct SemanticInfo {
      pub project_id: String,
      pub name: String,
      pub project_number: Option<String>,
      pub group_address_style: GroupAddressStyle,
      pub completion: CompletionStatus,
  }

  #[derive(Debug, PartialEq, Eq)]
  pub struct SemanticInstallation {
      /// Sorted by `ets_id`. Every collection below is likewise sorted, so
      /// ordering differences are normalized away and content differences
      /// are not.
      pub areas: Vec<(String, u8, String)>,
      pub lines: Vec<SemanticLine>,
      pub devices: Vec<SemanticDevice>,
      pub group_ranges: Vec<SemanticGroupRange>,
      pub group_addresses: Vec<SemanticGroupAddress>,
      pub buildings: Vec<SemanticBuildingPart>,
      pub parameters: Vec<SemanticParameter>,
  }

  #[derive(Debug, PartialEq, Eq)]
  pub struct SemanticDevice {
      pub ets_id: String,
      pub name: String,
      pub address: Option<String>,
      pub product_ref: Option<String>,
      pub program_ref: Option<String>,
      pub commissioning: CommissioningState,
      pub binary_data: Vec<String>,
      pub com_objects: Vec<SemanticComObject>,
  }

  #[derive(Debug, PartialEq, Eq)]
  pub struct SemanticComObject {
      pub number: u16,
      /// `None` where the attribute was absent, `Some("")` where it was
      /// present and empty — the distinction Task 1 exists to preserve.
      pub text: Option<String>,
      pub description: Option<String>,
      pub dpt: Option<String>,
      /// Read, write, transmit, update, communication, in that order.
      pub flags: [Option<bool>; 5],
      /// `(group address ets_id, direction)`, sorted.
      pub links: Vec<(String, Direction)>,
  }

  pub fn semantic_view(project: &Project) -> SemanticProject;
  pub fn describe_difference(a: &SemanticProject, b: &SemanticProject) -> Option<String>;
  ```

  `SemanticLine`, `SemanticGroupRange`, `SemanticGroupAddress`,
  `SemanticBuildingPart` and `SemanticParameter` follow the same shape: the
  `ets_id`, every modelled attribute rendered as `Option<String>` or its own
  enum, and no synthetic identifier. `Override::Absent` renders as `None` and
  `Override::Empty` as `Some(String::new())`.

`semantic_view` keys every entity by its `SourceRef::ets_id`, sorts every
collection by that key, drops all `*Id` values, and keeps: project info; per
installation the topology, buildings, group ranges and addresses, parameters;
per device the identity, address, commissioning state, binary data references;
per communication object the number, the four overrides, the flag set, and the
links rendered as `(group address ets_id, direction)` pairs sorted.

`describe_difference` returns the first difference as text rather than a bare
`false`, because a failing roundtrip test that says only "not equal" costs an
hour.

- [ ] **Step 1: Write the tests**

```rust
#[test]
fn roundtrip_model_is_semantically_equal() {
    let first = import_knxproj(reference_ets4_path()).unwrap();
    let exported = export_knxproj(&first.project, &first.opaque).unwrap();
    let second = import_knxproj_bytes(exported.bytes, "roundtrip.knxproj").unwrap();

    let a = semantic_view(&first.project);
    let b = semantic_view(&second.project);
    if let Some(diff) = describe_difference(&a, &b) {
        panic!("roundtrip changed the model: {diff}");
    }
}

#[test]
fn roundtrip_opaque_bytes_are_hash_identical() {
    let first = import_knxproj(reference_ets4_path()).unwrap();
    let exported = export_knxproj(&first.project, &first.opaque).unwrap();
    let second = import_knxproj_bytes(exported.bytes, "roundtrip.knxproj").unwrap();

    let hashes = |entries: &[OpaqueEntry]| {
        let mut v: Vec<_> = entries.iter()
            .map(|e| (e.source_path.clone(), e.xpath.clone(), e.name.clone(), e.sha256.clone()))
            .collect();
        v.sort();
        v
    };
    assert_eq!(hashes(&first.opaque), hashes(&second.opaque));
}

#[test]
fn export_is_unsigned_and_reports_it() {
    let first = import_knxproj(reference_ets4_path()).unwrap();
    let exported = export_knxproj(&first.project, &first.opaque).unwrap();
    assert!(exported.warnings.iter().any(|w| matches!(w, ExportWarning::Unsigned { .. })));

    // And the file itself carries no signature we produced: the entries that
    // are there were copied, and are reported stale.
    let stale: Vec<_> = exported.warnings.iter()
        .filter_map(|w| match w {
            ExportWarning::StaleSignature { source_path } => Some(source_path.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(stale.len(), 5);
}

#[test]
fn a_second_roundtrip_changes_nothing_further() {
    // Convergence: if the first roundtrip normalizes something, the second
    // must not normalize it again. A pipeline that keeps changing the file is
    // not a roundtrip.
    let first = import_knxproj(reference_ets4_path()).unwrap();
    let once = export_knxproj(&first.project, &first.opaque).unwrap().bytes;
    let mid = import_knxproj_bytes(once.clone(), "once.knxproj").unwrap();
    let twice = export_knxproj(&mid.project, &mid.opaque).unwrap().bytes;

    let a = Container::open(once).unwrap();
    let b = Container::open(twice).unwrap();
    let names = |c: &Container| {
        let mut v: Vec<_> = c.entries().iter().map(|e| e.path.clone()).collect();
        v.sort();
        v
    };
    assert_eq!(names(&a), names(&b));
    let mut a = a;
    let mut b = b;
    assert_eq!(a.read("P-0512/0.xml").unwrap(), b.read("P-0512/0.xml").unwrap());
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-etsproj --test roundtrip`
Expected: FAIL, `cannot find function semantic_view`.

- [ ] **Step 3: Implement `compare.rs`**

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-etsproj --test roundtrip`
Expected: PASS, 4 tests. Byte equality of the whole container is neither
attempted nor asserted; the second-roundtrip test asserts convergence of the
regenerated `0.xml`, which is a weaker and achievable claim.

- [ ] **Step 5: Run the gate and commit**

```bash
git add crates/knx-etsproj/src crates/knx-etsproj/tests
git commit -m "knx-etsproj: the three roundtrip fidelity guarantees

Semantic equality under a declared comparison relation, hash-identical opaque
bytes, and an export that is unsigned and says so. Plus convergence: a second
roundtrip changes nothing further."
```

---

### Task 21: The malformed-input suite

**Files:**
- Create: `crates/knx-etsproj/tests/malformed_input.rs`

The rule under test is that bad input produces a named error or a report entry,
never a panic, never an unwrap on absent data, and never silently wrong data.

- [ ] **Step 1: Write the tests**

```rust
#[test]
fn an_empty_file_is_not_a_zip() {
    assert!(matches!(
        import_knxproj_bytes(vec![], "empty.knxproj"),
        Err(ImportFailure::Container(ContainerError::NotAZip(_)))
    ));
}

#[test]
fn a_zip_with_no_project_part_is_named_as_such() {
    let bytes = zip_with_entries(&[("knx_master.xml", b"<KNX/>")]);
    assert!(matches!(
        import_knxproj_bytes(bytes, "no-project.knxproj"),
        Err(ImportFailure::Container(ContainerError::NoProjectPart))
    ));
}

#[test]
fn a_truncated_installation_document_reports_its_position() {
    let bytes = knxproj_with_installation(&MINIMAL[..MINIMAL.len() / 2]);
    match import_knxproj_bytes(bytes, "truncated.knxproj") {
        Err(ImportFailure::Parse(ParseError::Xml { position, .. })) => assert!(position > 0),
        other => panic!("expected a parse error with a position, got {other:?}"),
    }
}

#[test]
fn a_document_with_no_namespace_is_rejected_rather_than_assumed_to_be_schema_eleven() {
    let bytes = knxproj_with_installation(br#"<KNX CreatedBy="ETS4"><Project Id="P-0001"/></KNX>"#);
    assert!(matches!(
        import_knxproj_bytes(bytes, "no-ns.knxproj"),
        Err(ImportFailure::Detect(DetectError::NoDefaultNamespace { .. }))
    ));
}

#[test]
fn an_unsupported_schema_version_is_named_and_not_guessed_at() {
    let bytes = knxproj_with_installation(
        br#"<KNX xmlns="http://knx.org/xml/project/14"><Project Id="P-0001"/></KNX>"#,
    );
    assert!(matches!(
        import_knxproj_bytes(bytes, "v14.knxproj"),
        Err(ImportFailure::NoKnownSchemaTable { version: 14 })
    ));
}

#[test]
fn an_invalid_individual_address_is_reported_and_the_device_still_imports() {
    let xml = minimal_xml_with(r#"<DeviceInstance Id="P-0001-0_DI-1" Name="D" Address="999""#);
    let out = import_knxproj_bytes(knxproj_with_installation(xml.as_bytes()), "bad-addr.knxproj").unwrap();
    assert!(out.report.errors.iter().any(|e| e.detail.contains("Address")));
    assert_eq!(out.project.devices.iter().count(), 1);
    assert!(out.project.devices.iter().next().unwrap().address.is_none());
}

#[test]
fn a_duplicate_group_address_id_is_reported_and_both_entries_survive() {
    let out = import_knxproj_bytes(knxproj_with_duplicate_ga_id(), "dupe.knxproj").unwrap();
    assert!(out.report.errors.iter().any(|e| e.detail.contains("duplicate")));
    assert_eq!(out.project.installations[0].group_addresses.len(), 2);
}

#[test]
fn a_dangling_group_address_reference_drops_the_link_and_reports_it() {
    let out = import_knxproj_bytes(knxproj_with_dangling_link(), "dangling.knxproj").unwrap();
    assert!(out.report.errors.iter().any(|e| e.detail.contains("GroupAddressRefId")));
    let coms: Vec<_> = out.project.devices.iter()
        .flat_map(|d| d.com_objects.clone())
        .filter_map(|id| out.project.devices.com_object(id))
        .collect();
    assert!(coms.iter().all(|c| c.links.is_empty()));
}

#[test]
fn a_zip_bomb_shaped_entry_does_not_exhaust_memory() {
    // A single entry declaring a huge uncompressed size must be refused before
    // it is read into a Vec, not after.
    let bytes = zip_with_declared_size("P-0001/0.xml", 4 * 1024 * 1024 * 1024);
    assert!(import_knxproj_bytes(bytes, "bomb.knxproj").is_err());
}

#[test]
fn a_deeply_nested_document_does_not_overflow_the_stack() {
    // The parser is iterative with an explicit stack; 10000 levels must be an
    // error, not a crash.
    let mut xml = String::from(r#"<KNX xmlns="http://knx.org/xml/project/11">"#);
    for _ in 0..10_000 {
        xml.push_str("<GroupRange>");
    }
    let bytes = knxproj_with_installation(xml.as_bytes());
    assert!(import_knxproj_bytes(bytes, "deep.knxproj").is_err());
}
```

`Container::read` gains a size guard for the zip-bomb test: an entry whose
declared uncompressed size exceeds a documented limit is refused with
`ContainerError::Read` before allocation. The limit is 64 MB, chosen because
the largest entry in either reference project is 5.7 MB and the whole
uncompressed container is 22 MB.

- [ ] **Step 2: Run them and watch them fail, then implement the guards**

Run: `cargo test -p knx-etsproj --test malformed_input`
Expected: FAIL on the guard tests until `Container::read`'s size check and the
parser's depth limit exist.

- [ ] **Step 3: Run them and watch them pass**

Run: `cargo test -p knx-etsproj --test malformed_input`
Expected: PASS, 10 tests.

- [ ] **Step 4: Run the gate and commit**

```bash
git add crates/knx-etsproj
git commit -m "knx-etsproj: malformed input suite

Bad input produces a named error or a report entry, never a panic and never
silently wrong data. Adds an uncompressed-size guard and a nesting depth limit."
```

---

### Task 22: The application service and the CLI

**Files:**
- Create: `crates/knx-app/src/import.rs`
- Modify: `crates/knx-app/src/lib.rs`
- Modify: `crates/knx-app/Cargo.toml`
- Modify: `apps/knx-cli/src/main.rs`
- Modify: `apps/knx-cli/Cargo.toml`

**Interfaces:**
- Consumes: `import_knxproj`, `export_knxproj`, `ImportReport` (Tasks 14, 15,
  19), `insert_opaque`, `load_opaque`, `StoredOpaqueEntry`, `open_and_migrate`
  (Task 13).
- Produces:
  ```rust
  // knx-app
  pub struct ImportedProject {
      pub project: knx_core::Project,
      pub report: knx_etsproj::ImportReport,
      pub opaque_entries: usize,
  }

  #[derive(Debug)]
  pub enum AppError {
      Import(knx_etsproj::ImportFailure),
      Store(knx_store::MigrationError),
      Sql(rusqlite::Error),
  }

  /// Imports a `.knxproj` and persists its opaque entries into an already
  /// migrated connection. The caller owns the connection, so the CLI decides
  /// whether it is a file or in memory.
  pub fn import_ets_project(
      path: &Path,
      conn: &Connection,
  ) -> Result<ImportedProject, AppError>;
  ```

  `knx-store` gains `pub use rusqlite::{Connection, Error as SqlError};` so
  that `knx-app` names the connection type through the storage crate rather
  than depending on `rusqlite` directly. The conversion from
  `knx_etsproj::OpaqueEntry` to `knx_store::StoredOpaqueEntry` lives in
  `knx-app/src/import.rs`, which is the only place both types are visible.

`knx-app` is the only crate that sees both `knx-etsproj` and `knx-store`, which
is what keeps `knx-etsproj` free of a storage dependency. `check-layering`
enforces it.

The CLI gains one subcommand:

```
knx import <file.knxproj> [--store <path.knxdb>] [--report-json <path.json>]
```

Human-readable output goes to stdout, the exit code is 0 when the import
produced a project and 1 when it did not, and `--report-json` writes the
serialized `ImportReport` for machine consumption. Warnings never change the
exit code — an import that reports 138 unsupported constructs still succeeded.

- [ ] **Step 1: Write failing tests**

`knx-app`'s tests cannot reach `knx-etsproj`'s `tests/support` module — a
crate's integration tests are private to it — so they repeat the four-line
path helper rather than a public test-only API being added to `knx-etsproj`
for their benefit:

```rust
// crates/knx-app/tests/import_service.rs
fn reference_ets4_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().and_then(Path::parent).unwrap()
        .join("Unser Zuhause ets4 - 2025-12-15.knxproj")
}

fn migrated_store() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    (dir, conn)
}

#[test]
fn importing_persists_the_opaque_entries() {
    let (_dir, conn) = migrated_store();
    let imported = import_ets_project(&reference_ets4_path(), &conn).unwrap();
    assert_eq!(imported.project.installations.len(), 1);
    assert!(imported.opaque_entries > 0);
    assert_eq!(load_opaque(&conn).unwrap().len(), imported.opaque_entries);
}

#[test]
fn the_persisted_bytes_are_the_bytes_that_were_read() {
    let (_dir, conn) = migrated_store();
    import_ets_project(&reference_ets4_path(), &conn).unwrap();
    let stored = load_opaque(&conn).unwrap();
    let dll = stored.iter()
        .find(|e| e.source_path.ends_with("econEts3.dll"))
        .expect("the baggage entry is persisted, not executed");
    assert_eq!(sha256_hex(&dll.bytes), dll.sha256);
}

#[test]
fn a_failed_import_leaves_the_store_untouched() {
    let (_dir, conn) = migrated_store();
    assert!(import_ets_project(Path::new("/nonexistent.knxproj"), &conn).is_err());
    assert_eq!(load_opaque(&conn).unwrap().len(), 0);
}
```

```rust
// apps/knx-cli/tests/cli_import.rs — uses std::process::Command on the built binary
#[test]
fn the_cli_reports_counts_and_exits_zero() {
    let out = run_cli(&["import", reference_ets4_path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("36 devices"));
    assert!(text.contains("514 group addresses"));
    assert!(text.contains("unsupported"));
}

#[test]
fn the_cli_writes_a_machine_readable_report() {
    let dir = tempdir().unwrap();
    let report = dir.path().join("report.json");
    let out = run_cli(&[
        "import", reference_ets4_path().to_str().unwrap(),
        "--report-json", report.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    assert_eq!(json["source"]["schema_version"], 11);
    assert!(json["unsupported"].as_array().unwrap().len() > 0);
}

#[test]
fn the_cli_exits_nonzero_on_a_file_it_cannot_read() {
    let out = run_cli(&["import", "/nonexistent.knxproj"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr).unwrap().contains("nonexistent"));
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p knx-app -p knx-cli`
Expected: FAIL, `cannot find function import_ets_project`.

- [ ] **Step 3: Implement the service and the subcommand**

`insert_opaque` runs the whole batch inside one `conn.unchecked_transaction()`
so the last test's guarantee is the database's and not the caller's: an import
that fails at any stage writes nothing, because the import completes in full
before the first row is inserted.

`import_ets_project` calls `import_knxproj`, maps each `OpaqueEntry` to a
`StoredOpaqueEntry` with `kind` rendered as the `OpaqueKind` variant name, and
inserts them. It never executes, decompresses further, or interprets any
retained bytes.

- [ ] **Step 4: Run them and watch them pass**

Run: `cargo test -p knx-app -p knx-cli`
Expected: PASS, 7 tests.

- [ ] **Step 5: Run the full gate**

```bash
cargo build --workspace \
  && cargo test --workspace \
  && cargo fmt --all --check \
  && cargo clippy --workspace --all-targets -- -D warnings \
  && cargo run -p xtask -- check-layering \
  && cargo deny check
```

`check-layering` must confirm that `knx-etsproj` still has no path to
`knx-store`.

- [ ] **Step 6: Commit**

```bash
git add crates/knx-app apps/knx-cli
git commit -m "knx-app: import service and knx import subcommand

knx-app is the only crate that sees both the importer and the store, which
keeps knx-etsproj free of a storage dependency. Warnings do not change the
exit code."
```

---

### Task 23: Documentation

**Files:**
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/COMPATIBILITY.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`
- Modify: `docs/IMPORT_EXPORT.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/DATA_MODEL.md`
- Modify: `docs/ROADMAP.md`
- Create: `docs/adr/0010-per-attribute-override-representation.md`
  (0008 and 0009 are taken by `0008-key-material-isolation.md` and
  `0009-ui-boundary.md`)
- Modify: `docs/adr/README.md` — the index gains the new record

Documentation is a deliverable of this session, not a postscript. Each item
below is a specific edit with its reason.

- [ ] **Step 1: Correct the two evidence findings in the research and model documents**

`RESEARCH.md` currently records 758 `DatapointType` attributes on
`ComObjectInstanceRef` without distinguishing empty ones. Amend it to: 758
occurrences, of which 497 are the empty string and 261 carry a value. The empty
string is ETS's way of saying "the program's datapoint type is deliberately
cleared here", and it is not the same as the attribute being absent.

`DATA_MODEL.md` §3 describes the override chain as resolving per object. Amend
it to per attribute, with the resolved value carrying its own layer, and point
at ADR-0010.

Add to `RESEARCH.md`: `ParameterInstanceRef/@RefId` references two parameter
kinds in the reference project — 1174 plain parameters (`_P-`) and 216 union
parameters (`_UP-`). A parser that matches only `_P-` silently drops 15.5% of
the parameter values.

Add to `RESEARCH.md`: the communication object number is the `O-<digits>`
segment that immediately precedes the final `R-<digits>` segment of
`ComObjectInstanceRef/@RefId`, not the first `O-` token in the string — an
application program identifier can itself contain one. Verified against all 907
reference RefIds: 0 mismatches.

- [ ] **Step 2: Write ADR-0010**

Title: "Overrides are represented per attribute with an explicit empty state".

Context: the ETS override chain is `ComObject` → `ComObjectRef` →
`ComObjectInstanceRef`, and each attribute overrides independently. An
attribute can be absent, present-and-empty, or present-with-a-value, and those
three are distinguishable in the file and must stay distinguishable in the
model or export loses 497 attributes in one reference project alone.

Decision: `Override<T>` with `Absent`, `Empty`, `Value(Resolved<T>)`, one per
overridable attribute, and `ResolvedFlags` holding five independent
`Override<bool>`.

Consequences: the model is wordier; export becomes a total function of the
model with no "was this really absent?" guesswork; the UI can show which layer
a value came from per field, which is the behavior ETS users expect from the
properties inspector.

Alternatives rejected: `Option<Resolved<T>>` (cannot express `Empty`);
a side table of raw attributes (splits the truth into two places).

- [ ] **Step 3: Update `IMPORT_EXPORT.md`**

§5's fidelity table gains a column stating, per construct, whether it is
modeled, retained opaquely, or reported unsupported — the column now has real
values instead of intentions, because the code exists.

§9 gains the exact roundtrip guarantee wording matching Task 20's three tests,
including the explicit statement that byte equality is not attempted.

Add the container size guard (64 MB per entry) and the nesting depth limit from
Task 21 to the parsing section, with their measured justification.

- [ ] **Step 4: Update `COMPATIBILITY.md`**

State what has actually been verified, in these terms and no stronger:

- Reads schema 11 (`.knxproj` written by ETS 4.1.8) — verified against one
  reference project.
- Detects schema 23 and refuses it by name — verified.
- Detects password-protected containers and refuses them by name; decryption is
  not implemented.
- Writes schema 11 containers that our own reader reads back to a semantically
  equal model — verified.
- Whether ETS reads a container we wrote: **untested**. No claim either way.

The last line stays until someone runs it against a real ETS installation and
records the result here.

- [ ] **Step 5: Update `KNOWN_LIMITATIONS.md`**

- Schema 23 is detected and refused; the schema-23 known-element table is
  Session 4 work.
- Exports are unsigned; signatures cannot be regenerated without KNX signing
  keys. Copied signatures are stale and reported per entry.
- Manufacturer application programs live in the opaque store this session, so
  parameter semantics (type, range, unit, translations) are not yet available;
  they move into the product database in Session 4.
- Only one reference project per schema version has been tested. The
  known-element table for schema 11 is derived from it and from schema-11
  documentation, not from an authoritative XSD, which is not public.
- Password-protected projects are refused, not decrypted.

- [ ] **Step 6: Update `IMPLEMENTATION_STATUS.md` and `ROADMAP.md`**

Mark Session 3 complete with the measured counts from Task 16's golden test.

Correct the pre-existing status line that says entity tables arrive in Session
3: ROADMAP Session 3 lists only the opaque store, and this session ships only
the opaque store. Full SQLite entity persistence moves to Session 4 alongside
the product database, and the status document should say that rather than
leaving a promise the code does not keep.

- [ ] **Step 7: Verify every claim in the documents against the tests**

For each verified claim in `COMPATIBILITY.md`, name the test that verifies it
in the document itself. A compatibility claim with no test behind it is exactly
the thing `CLAUDE.md` forbids.

- [ ] **Step 8: Run the full gate and commit**

```bash
cargo build --workspace \
  && cargo test --workspace \
  && cargo fmt --all --check \
  && cargo clippy --workspace --all-targets -- -D warnings \
  && cargo run -p xtask -- check-layering \
  && cargo deny check
```

```bash
git add docs
git commit -m "docs: record Session 3 import results, limitations and ADR-0010

Corrects two measurement claims found while implementing: 497 of the 758
DatapointType overrides are empty strings, and parameter references use two
kinds. Compatibility claims name the test that verifies them."
```

---

## Definition of Done

Session 3 is complete when all of the following hold:

1. `cargo build --workspace` succeeds on the pinned toolchain (1.98.0).
2. `cargo test --workspace` passes, including the golden test, the oracle
   comparison, the three roundtrip guarantees and the malformed-input suite.
3. `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D
   warnings` are clean.
4. `cargo run -p xtask -- check-layering` passes, confirming `knx-core` reaches
   no infrastructure crate and `knx-etsproj` reaches no storage crate.
5. `cargo deny check` passes with no new licence exceptions.
6. `knx import <reference>.knxproj` prints the measured counts and exits 0.
7. Every claim in `COMPATIBILITY.md` names the test that verifies it.
8. No construct present in the reference project is silently absent from both
   the model and the opaque store — asserted by the golden test's
   accounted-for check.
