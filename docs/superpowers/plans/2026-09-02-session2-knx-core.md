# Session 2 — KNX Core Implementation Plan

> **For agentic workers:** Executed inline by the controller in the same
> session (Agent-tool subagent dispatch is blocked by the auto-mode permission
> classifier in this repository — see the project memory
> `subagent-dispatch-blocked`). No fresh-context subagents are dispatched;
> the controller implements each task directly, runs
> `cargo build --workspace && cargo test --workspace && cargo fmt --all --check
> && cargo clippy --workspace --all-targets -- -D warnings && cargo run -p
> xtask -- check-layering && cargo deny check` after each task, and commits
> per task. Because the controller holds full context already, per-task
> "briefs" are the task sections below, read directly rather than extracted
> into separate files.

**Goal.** Implement the `knx-core` domain model described in
[docs/DATA_MODEL.md](../../DATA_MODEL.md): entities, typed addresses, DPT
references, override resolution, validation, the string table, and the
command layer with undo/redo — plus the schema-version constant and a
migration-chain skeleton in `knx-store`, per
[docs/ROADMAP.md](../../ROADMAP.md) Session 2.

**Architecture:** One module per concern inside `crates/knx-core/src/`,
re-exported from `lib.rs`. No IO anywhere in `knx-core` (enforced by
`xtask check-layering`). `knx-store` gains its first real content: a
`user_version`-pragma-based migration chain, still schema-empty beyond a
metadata marker — full entity tables are Session 3's job, once the importer
exists to fill them.

**Tech Stack:** Rust, workspace crates `knx-core` (no dependencies) and
`knx-store` (adds `rusqlite 0.40` with the `bundled` feature — MIT-licensed,
matches `deny.toml`'s allowlist, avoids a system SQLite dependency).

**Spec:** [docs/DATA_MODEL.md](../../DATA_MODEL.md) (primary),
[docs/ARCHITECTURE.md](../../ARCHITECTURE.md) §5–6 (override model, command
layer), [ADR-0003](../../adr/0003-sqlite-project-format.md),
[ADR-0004](../../adr/0004-provenance-model.md).

## Global Constraints

- `knx-core` reaches none of `serde_json`, `quick-xml`, `rusqlite`, `tokio` —
  gated by `cargo run -p xtask -- check-layering`.
- No runtime crate depends on a GPL-licensed crate — gated by `cargo deny
  check`; `deny.toml`'s allowlist is authoritative.
- Internal IDs (`DeviceId`, `GroupAddressId`, ...) are synthetic, project-
  unique, never derived from ETS's own id strings (DATA_MODEL §2).
- `SourceRef { path: String, ets_id: String }` is carried by every entity
  that came from import, and is never used as a primary key (DATA_MODEL §2).
- A value that can be overridden is `Resolved<T> { value, layer }`, never a
  bare `T` (DATA_MODEL §3, ADR-0004). Any command that changes such a value
  sets its layer to `Layer::UserEdit`.
- `Devices` is the sole owner of devices; `Topology` and `Buildings` hold
  references only (DATA_MODEL §5).
- A device without a line is valid (`Topology::unassigned`); a group address
  without a DPT is valid; a group address with no linked communication
  object is valid. None of these are validation failures (DATA_MODEL §5, §9).
- Direction (`Send`/`Receive`) on a `GroupLink` is never flattened into an
  undirected association (DATA_MODEL §6).
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D
  warnings` must pass after every task.

---

### Task 1: Module scaffold, `provenance` split

**Files:**
- Create: `crates/knx-core/src/provenance.rs`
- Create: `crates/knx-core/src/ids.rs`
- Modify: `crates/knx-core/src/lib.rs`

**Interfaces:**
- Produces: `provenance::Layer`, `provenance::Resolved<T>` (moved verbatim
  from the current `lib.rs`, behaviour unchanged), re-exported at crate root
  as `knx_core::{Layer, Resolved}`.
- Produces: `ids::SourceRef { pub path: String, pub ets_id: String }`.
- Produces: `ids::{InstallationId, AreaId, LineId, DeviceId, ComObjectInstanceId,
  GroupRangeId, GroupAddressId, BuildingPartId, ParameterInstanceId}`, each a
  `pub struct XId(pub u32)` (except `InstallationId(pub u8)`, which mirrors
  ETS's own small stable installation number rather than a synthetic
  counter — see doc comment), built through one local macro
  `id_type!(name, repr)` to avoid nine hand-written boilerplate blocks. Each
  derives `Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord` and
  implements `Display`.

Steps:
- [ ] Move `Layer`, `Resolved<T>` and their existing test module from
  `lib.rs` into `provenance.rs`, unchanged.
- [ ] Write `ids.rs` with the `id_type!` macro and the nine id types plus
  `SourceRef`.
- [ ] `lib.rs` becomes: crate doc comment (unchanged), `pub mod provenance;`,
  `pub mod ids;`, `pub use provenance::{Layer, Resolved};`, `pub use
  ids::*;`.
- [ ] Add a unit test in `ids.rs`: two `DeviceId` values with different
  `u32`s are not equal; `DeviceId(1).to_string() == "1"`.
- [ ] Run `cargo test -p knx-core` — expect the moved `Layer` test plus the
  new `ids` test to pass.
- [ ] Commit: `knx-core: split provenance and id types into modules`.

---

### Task 2: Typed addresses

**Files:**
- Create: `crates/knx-core/src/address.rs`
- Modify: `crates/knx-core/src/lib.rs` (add `pub mod address;` and re-export)

**Interfaces:**
- Produces:
  ```rust
  pub struct IndividualAddress(u16); // area:4 line:4 device:8, packed as ETS does
  impl IndividualAddress {
      pub fn new(area: u8, line: u8, device: u8) -> Result<Self, AddressError>; // area,line <= 15
      pub fn from_raw(raw: u16) -> Self;
      pub fn raw(self) -> u16;
      pub fn area(self) -> u8;
      pub fn line(self) -> u8;
      pub fn device(self) -> u8;
  }
  impl FromStr for IndividualAddress // "area.line.device"
  impl fmt::Display for IndividualAddress // "area.line.device"

  pub enum GroupAddressStyle { Free, TwoLevel, ThreeLevel }

  pub struct GroupAddress(u16);
  impl GroupAddress {
      pub fn from_raw(raw: u16) -> Self;
      pub fn raw(self) -> u16;
      pub fn parse(s: &str, style: GroupAddressStyle) -> Result<Self, AddressError>;
      pub fn format(self, style: GroupAddressStyle) -> String;
  }

  pub enum AddressError {
      OutOfRange { field: &'static str, value: u32, max: u32 },
      MalformedIndividual(String),
      MalformedGroup(String),
  }
  impl std::error::Error for AddressError {}
  impl fmt::Display for AddressError
  ```
- Consumes: nothing from Task 1 besides the module pattern.
- Produces (for later tasks): `IndividualAddress`, `GroupAddress`,
  `GroupAddressStyle`, `AddressError` — used by `device.rs`, `group.rs`,
  `validation.rs`, `commands.rs`.

Free style is the plain `u16` decimal. Two-level packs `main:5 sub:11`
("main/sub", main 0–31, sub 0–2047). Three-level packs `main:5 middle:3
sub:8` ("main/middle/sub", main 0–31, middle 0–7, sub 0–255). Individual
address packs `area:4 line:4 device:8` ("area.line.device", area and line
0–15, device 0–255).

Steps:
- [ ] Write failing tests first, in a `#[cfg(test)] mod tests` block at the
  bottom of `address.rs`:
  ```rust
  #[test]
  fn individual_address_roundtrips_through_display_and_parse() {
      let a = IndividualAddress::new(1, 2, 3).unwrap();
      assert_eq!(a.to_string(), "1.2.3");
      assert_eq!("1.2.3".parse::<IndividualAddress>().unwrap(), a);
  }

  #[test]
  fn individual_address_rejects_area_above_15() {
      assert!(matches!(
          IndividualAddress::new(16, 0, 0),
          Err(AddressError::OutOfRange { field: "area", .. })
      ));
  }

  #[test]
  fn group_address_free_style_is_plain_decimal() {
      let ga = GroupAddress::parse("1234", GroupAddressStyle::Free).unwrap();
      assert_eq!(ga.raw(), 1234);
      assert_eq!(ga.format(GroupAddressStyle::Free), "1234");
  }

  #[test]
  fn group_address_three_level_roundtrips() {
      let ga = GroupAddress::parse("4/2/100", GroupAddressStyle::ThreeLevel).unwrap();
      assert_eq!(ga.format(GroupAddressStyle::ThreeLevel), "4/2/100");
      // main=4 (bits 15-11), middle=2 (bits 10-8), sub=100 (bits 7-0)
      assert_eq!(ga.raw(), (4u16 << 11) | (2u16 << 8) | 100u16);
  }

  #[test]
  fn group_address_two_level_roundtrips() {
      let ga = GroupAddress::parse("4/612", GroupAddressStyle::TwoLevel).unwrap();
      assert_eq!(ga.format(GroupAddressStyle::TwoLevel), "4/612");
      assert_eq!(ga.raw(), (4u16 << 11) | 612u16);
  }

  #[test]
  fn group_address_rejects_out_of_range_middle() {
      assert!(GroupAddress::parse("1/8/1", GroupAddressStyle::ThreeLevel).is_err());
  }
  ```
- [ ] Run `cargo test -p knx-core address::` — expect FAIL (module does not
  exist yet).
- [ ] Implement `address.rs` to satisfy the tests, using the bit layouts
  documented above.
- [ ] Run `cargo test -p knx-core address::` — expect PASS.
- [ ] Commit: `knx-core: add typed individual and group addresses`.

---

### Task 3: Datapoint type reference

**Files:**
- Create: `crates/knx-core/src/dpt.rs`
- Modify: `crates/knx-core/src/lib.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct DptRef { pub main: u16, pub sub: Option<u16> }
  impl DptRef {
      pub fn parse(s: &str) -> Result<Self, DptParseError>; // "DPST-<main>-<sub>" or "DPT-<main>"
  }
  impl fmt::Display for DptRef // "DPST-1-1" or "DPT-1"
  pub enum DptParseError { Malformed(String) }
  ```
- Consumes: nothing.
- Produces (for later tasks): `DptRef` — used in `device.rs`
  (`ComObjectInstance::dpt: Option<Resolved<DptRef>>`).

Verified against the reference project's raw XML
(`M-0083/M-0083_A-0019-13-A892.xml` etc.): `ComObjectInstanceRef@DatapointType`
values observed are all `DPST-<main>-<sub>`. Note for the record, not part of
this task: `ComObjectRef@DatapointType` (the application-program layer, not
modelled until the product database exists) was observed to hold
space-separated lists of alternatives, e.g. `"DPST-9-21 DPST-9-21"` — a
polymorphic com object's set of acceptable types. `DptRef::parse` parses one
token; list parsing is deferred to whichever session ingests application
programs (`knx-productdb`, Session 4) and must not be assumed solved by this
type.

Steps:
- [ ] Write failing tests:
  ```rust
  #[test]
  fn parses_dpst_with_sub() {
      assert_eq!(DptRef::parse("DPST-1-1").unwrap(), DptRef { main: 1, sub: Some(1) });
  }

  #[test]
  fn parses_dpt_without_sub() {
      assert_eq!(DptRef::parse("DPT-1").unwrap(), DptRef { main: 1, sub: None });
  }

  #[test]
  fn displays_with_and_without_sub() {
      assert_eq!(DptRef { main: 14, sub: Some(19) }.to_string(), "DPST-14-19");
      assert_eq!(DptRef { main: 1, sub: None }.to_string(), "DPT-1");
  }

  #[test]
  fn rejects_malformed_input() {
      assert!(DptRef::parse("bogus").is_err());
      assert!(DptRef::parse("DPST-1-1 DPST-1-2").is_err());
  }
  ```
- [ ] Run tests, expect FAIL.
- [ ] Implement `dpt.rs`.
- [ ] Run tests, expect PASS.
- [ ] Commit: `knx-core: add datapoint type reference`.

---

### Task 4: String table

**Files:**
- Create: `crates/knx-core/src/string_table.rs`
- Modify: `crates/knx-core/src/lib.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct Language(pub String); // e.g. "de-DE"
  pub struct TranslationKey(pub String); // the source id a TranslationUnit entry is keyed by
  pub struct LocalizedString(pub TranslationKey); // a handle, never a bare String

  pub struct StringTable {
      default_language: Language,
      // (key, language) -> text
  }
  impl StringTable {
      pub fn new(default_language: Language) -> Self;
      pub fn insert(&mut self, key: TranslationKey, language: Language, text: String);
      pub fn resolve(&self, handle: &LocalizedString, language: &Language) -> Option<&str>;
      // falls back to default_language if `language` has no entry for the key
  }
  ```
- Consumes: nothing.
- Produces (for later tasks): `LocalizedString`, `StringTable` — used by
  `device.rs` (`ComObjectInstance::text`, `::description`), `project.rs`
  (`Project::strings`).

Steps:
- [ ] Write failing tests:
  ```rust
  #[test]
  fn resolves_exact_language_match() {
      let mut t = StringTable::new(Language("en".into()));
      let key = TranslationKey("k1".into());
      t.insert(key.clone(), Language("de".into()), "Licht".into());
      t.insert(key.clone(), Language("en".into()), "Light".into());
      let h = LocalizedString(key);
      assert_eq!(t.resolve(&h, &Language("de".into())), Some("Licht"));
  }

  #[test]
  fn falls_back_to_default_language_when_missing() {
      let mut t = StringTable::new(Language("en".into()));
      let key = TranslationKey("k1".into());
      t.insert(key.clone(), Language("en".into()), "Light".into());
      let h = LocalizedString(key);
      assert_eq!(t.resolve(&h, &Language("fr".into())), Some("Light"));
  }

  #[test]
  fn returns_none_when_key_entirely_absent() {
      let t = StringTable::new(Language("en".into()));
      let h = LocalizedString(TranslationKey("missing".into()));
      assert_eq!(t.resolve(&h, &Language("en".into())), None);
  }
  ```
- [ ] Give `Language` and `TranslationKey` `Clone, Debug, PartialEq, Eq,
  Hash` so they can be map keys and be cloned in the tests above.
- [ ] Run tests, expect FAIL.
- [ ] Implement using a `HashMap<(TranslationKey, Language), String>`.
- [ ] Run tests, expect PASS.
- [ ] Commit: `knx-core: add localized string table`.

---

### Task 5: Flags, commissioning state, directional links

**Files:**
- Create: `crates/knx-core/src/commissioning.rs`
- Create: `crates/knx-core/src/flags.rs`
- Modify: `crates/knx-core/src/lib.rs`

**Interfaces:**
- Produces (`flags.rs`):
  ```rust
  pub struct ComFlags {
      pub read: bool,
      pub write: bool,
      pub transmit: bool,
      pub update: bool,
      pub communication: bool,
  }
  pub enum ObjectSize { Bit(u8), Byte(u16) } // covers "1 Bit".."1 Byte".."14 Bytes" as observed
  pub enum Direction { Send, Receive }
  pub struct GroupLink { pub ga: GroupAddressId, pub direction: Direction }
  ```
- Produces (`commissioning.rs`):
  ```rust
  pub enum CompletionStatus { Undefined, Editing, FinishedDesign, Accepted }
  pub struct CommissioningState {
      pub completion: CompletionStatus,
      pub individual_address_loaded: bool,
      pub application_program_loaded: bool,
      pub parameters_loaded: bool,
      pub communication_part_loaded: bool,
      pub medium_config_loaded: bool,
      pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
      pub last_download: Option<chrono::DateTime<chrono::Utc>>,
      pub broken: bool,
  }
  impl Default for CommissioningState // all loaded=false, completion=Undefined, broken=false, timestamps=None
  ```
- Consumes: `ids::GroupAddressId` (Task 1).
- Note: this introduces `chrono` as a `knx-core` dependency (`DateTime<Utc>`
  only — no IO, no system clock reads inside the crate). `chrono` is
  MIT/Apache-2.0, already permitted by `deny.toml`; `check-layering`'s
  forbidden list is `serde_json`, `quick-xml`, `rusqlite`, `tokio` only, so
  this does not trip it. Add `chrono = { version = "0.4", default-features
  = false, features = ["std"] }` to `crates/knx-core/Cargo.toml` and to
  `[workspace.dependencies]`.
- Produces (for later tasks): `ComFlags`, `ObjectSize`, `Direction`,
  `GroupLink`, `CompletionStatus`, `CommissioningState` — used by
  `device.rs`.

Steps:
- [ ] Add the `chrono` dependency as specified above (workspace root
  `Cargo.toml` and `crates/knx-core/Cargo.toml`).
- [ ] Write `flags.rs` with the types above; no parsing logic here — flag
  and size string parsing belongs to the importer (Session 3), this task
  only models the resolved shape.
- [ ] Write `commissioning.rs` with the types above and the `Default` impl.
- [ ] Write a unit test in `commissioning.rs`:
  ```rust
  #[test]
  fn default_commissioning_state_is_all_unloaded_and_undefined() {
      let s = CommissioningState::default();
      assert_eq!(s.completion, CompletionStatus::Undefined);
      assert!(!s.individual_address_loaded);
      assert!(!s.broken);
      assert!(s.last_modified.is_none());
  }
  ```
- [ ] Run `cargo test -p knx-core` — expect PASS.
- [ ] Commit: `knx-core: add flags, directional links, commissioning state`.

---

### Task 6: Group ranges and group address entities

**Files:**
- Create: `crates/knx-core/src/group.rs`
- Modify: `crates/knx-core/src/lib.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct GroupRange {
      pub id: GroupRangeId,
      pub source: SourceRef,
      pub name: String,
      pub start: GroupAddress,
      pub end: GroupAddress,       // inclusive
  }
  impl GroupRange {
      pub fn contains(&self, ga: GroupAddress) -> bool; // start.raw() <= ga.raw() <= end.raw()
  }

  pub struct GroupAddressEntry {
      pub id: GroupAddressId,
      pub source: SourceRef,
      pub name: String,
      pub address: GroupAddress,
      pub central: bool,
      pub unfiltered: bool,
      pub range: Option<GroupRangeId>, // the range it was imported under, if any
  }
  ```
  Naming note: `GroupAddress` (Task 2) is the typed 16-bit value with parse
  and format; `GroupAddressEntry` is the project entity that carries one
  as a field, per DATA_MODEL §4/§9 distinguishing the address type from the
  addressable entity. Both names are deliberate and not to be merged.
- Consumes: `GroupAddress` (Task 2), `GroupRangeId`, `GroupAddressId`,
  `SourceRef` (Task 1).
- Produces (for later tasks): `GroupRange`, `GroupAddressEntry` — used by
  `project.rs`, `validation.rs`, `commands.rs`.

Steps:
- [ ] Write failing tests:
  ```rust
  #[test]
  fn range_contains_is_inclusive_on_both_ends() {
      let r = GroupRange {
          id: GroupRangeId(1),
          source: SourceRef { path: "t".into(), ets_id: "t".into() },
          name: "Lighting".into(),
          start: GroupAddress::from_raw(100),
          end: GroupAddress::from_raw(200),
      };
      assert!(r.contains(GroupAddress::from_raw(100)));
      assert!(r.contains(GroupAddress::from_raw(200)));
      assert!(!r.contains(GroupAddress::from_raw(201)));
  }
  ```
- [ ] Run test, expect FAIL.
- [ ] Implement `group.rs`.
- [ ] Run test, expect PASS.
- [ ] Commit: `knx-core: add group range and group address entities`.

---

### Task 7: Building parts and topology

**Files:**
- Create: `crates/knx-core/src/building.rs`
- Create: `crates/knx-core/src/topology.rs`
- Modify: `crates/knx-core/src/lib.rs`

**Interfaces:**
- Produces (`building.rs`):
  ```rust
  pub enum BuildingPartType {
      Building, Floor, Room, Corridor, DistributionBoard, BuildingPart,
  }
  pub struct BuildingPart {
      pub id: BuildingPartId,
      pub source: SourceRef,
      pub name: String,
      pub number: Option<String>,
      pub kind: BuildingPartType,
      pub default_line: Option<LineId>,
      pub completion: CompletionStatus,
      pub children: Vec<BuildingPartId>,
      pub devices: Vec<DeviceId>,      // DeviceInstanceRef — a reference, not ownership
      pub parent: Option<BuildingPartId>,
  }
  ```
- Produces (`topology.rs`):
  ```rust
  pub struct Area {
      pub id: AreaId,
      pub source: SourceRef,
      pub name: String,
      pub address: u8,
      pub completion: CompletionStatus,
      pub lines: Vec<LineId>,
  }
  pub struct Line {
      pub id: LineId,
      pub source: SourceRef,
      pub name: String,
      pub address: u8,
      pub medium_ref: String,          // MediumTypeRefId, opaque product reference
      pub completion: CompletionStatus,
      pub devices: Vec<DeviceId>,      // reference, not ownership
  }
  pub struct Topology {
      pub areas: Vec<Area>,
      pub unassigned: Vec<DeviceId>,   // devices with no line — valid, not an error
  }
  impl Topology {
      pub fn line(&self, id: LineId) -> Option<&Line>;
      pub fn area_of(&self, line: LineId) -> Option<&Area>;
  }
  ```
- Consumes: `CompletionStatus` (Task 5), `AreaId`, `LineId`, `DeviceId`,
  `BuildingPartId`, `SourceRef` (Task 1).
- Produces (for later tasks): `BuildingPart`, `BuildingPartType`, `Area`,
  `Line`, `Topology` — used by `project.rs`.

Steps:
- [ ] Write failing tests:
  ```rust
  // topology.rs
  #[test]
  fn line_lookup_finds_line_inside_its_area() {
      let line = Line {
          id: LineId(1), source: SourceRef { path: "t".into(), ets_id: "t".into() },
          name: "L1".into(), address: 1, medium_ref: "TP".into(),
          completion: CompletionStatus::FinishedDesign, devices: vec![],
      };
      let area = Area {
          id: AreaId(1), source: SourceRef { path: "t".into(), ets_id: "t".into() },
          name: "A1".into(), address: 1, completion: CompletionStatus::FinishedDesign,
          lines: vec![line.id],
      };
      let topo = Topology { areas: vec![area], unassigned: vec![] };
      // line() looks up by scanning; area_of() finds the owning area
      assert!(topo.area_of(LineId(1)).is_none()); // Topology alone doesn't own Line objects yet
  }
  ```
  Note while writing this test: `Topology.areas: Vec<Area>` holds areas, but
  `Line` objects themselves are not stored inside `Area` in the struct above
  (`Area.lines` is `Vec<LineId>`, a reference list, matching DATA_MODEL §5's
  "reference only" rule) — so `Topology` needs its own `lines: Vec<Line>` to
  make `line()`/`area_of()` implementable by lookup. Correct the interface
  before implementing: add `pub lines: Vec<Line>` to `Topology`, and
  implement `area_of` by scanning `areas` for one whose `lines` contains the
  given id. Rewrite the test above to assert `topo.area_of(LineId(1)) ==
  Some(&area)` once `lines` holds the real `Line`. This correction is part
  of this task, not a deviation from it.
- [ ] Write a second test:
  ```rust
  #[test]
  fn unassigned_device_is_valid_topology_state() {
      let topo = Topology { areas: vec![], lines: vec![], unassigned: vec![DeviceId(7)] };
      assert_eq!(topo.unassigned, vec![DeviceId(7)]);
  }
  ```
- [ ] Write a `building.rs` test:
  ```rust
  #[test]
  fn building_part_nests_via_children_not_containment() {
      let floor = BuildingPart {
          id: BuildingPartId(2), source: SourceRef { path: "t".into(), ets_id: "t".into() },
          name: "Floor 1".into(), number: Some("1".into()), kind: BuildingPartType::Floor,
          default_line: None, completion: CompletionStatus::FinishedDesign,
          children: vec![], devices: vec![], parent: Some(BuildingPartId(1)),
      };
      assert_eq!(floor.parent, Some(BuildingPartId(1)));
  }
  ```
- [ ] Run tests, expect FAIL, then implement `building.rs` and
  `topology.rs` (with the `Topology.lines` correction folded in), run again,
  expect PASS.
- [ ] Commit: `knx-core: add building parts and topology`.

---

### Task 8: Devices, communication object instances, parameters

**Files:**
- Create: `crates/knx-core/src/device.rs`
- Create: `crates/knx-core/src/parameter.rs`
- Modify: `crates/knx-core/src/lib.rs`

**Interfaces:**
- Produces (`device.rs`):
  ```rust
  pub struct DeviceInstance {
      pub id: DeviceId,
      pub source: SourceRef,
      pub name: String,
      pub description: Option<String>,
      pub address: Option<IndividualAddress>, // None only transiently; see validation.rs
      pub product_ref: String,          // ProductRefId, opaque product reference
      pub program_ref: String,          // Hardware2ProgramRefId, opaque product reference
      pub commissioning: CommissioningState,
      pub visibility_calculated: bool,  // IsCommunicationObjectVisibilityCalculated
      pub com_objects: Vec<ComObjectInstanceId>,
  }

  pub struct ComObjectInstance {
      pub id: ComObjectInstanceId,
      pub source: SourceRef,
      pub device: DeviceId,
      pub number: u16,                              // from _O-<n>
      pub text: Resolved<LocalizedString>,
      pub description: Option<Resolved<LocalizedString>>,
      pub dpt: Option<Resolved<DptRef>>,
      pub flags: Resolved<ComFlags>,
      pub size: Resolved<ObjectSize>,
      pub is_active: bool,
      pub links: Vec<GroupLink>,
  }
  ```
  (This is the exact shape from DATA_MODEL §4; transcribed, not
  reinterpreted.)
- Produces (`parameter.rs`):
  ```rust
  pub struct ParameterInstance {
      pub id: ParameterInstanceId,
      pub source: SourceRef,   // RefId
      pub raw: String,         // Value, uninterpreted — see DATA_MODEL §10
  }
  ```
- Consumes: `Resolved<T>`, `Layer` (Task 1 re-export), `LocalizedString`
  (Task 4), `DptRef` (Task 3), `ComFlags`, `ObjectSize`, `GroupLink`,
  `CommissioningState` (Task 5), `IndividualAddress` (Task 2), `DeviceId`,
  `ComObjectInstanceId`, `ParameterInstanceId`, `SourceRef` (Task 1).
- Produces (for later tasks): `DeviceInstance`, `ComObjectInstance`,
  `ParameterInstance` — used by `project.rs`, `validation.rs`,
  `commands.rs`.

Steps:
- [ ] Write a failing test in `device.rs` that exercises the override chain
  end to end, grounding DATA_MODEL §3's worked example in code:
  ```rust
  #[test]
  fn instance_layer_dpt_overrides_program_layer_dpt() {
      let program_dpt = Resolved { value: DptRef { main: 1, sub: Some(1) }, layer: Layer::Program };
      let instance_dpt = Resolved { value: DptRef { main: 5, sub: Some(1) }, layer: Layer::Instance };
      // The override chain resolves to the last (highest-precedence) layer present.
      // ComObjectInstance always stores the already-resolved value plus its layer;
      // resolving from raw layers is the importer's job (Session 3). This test
      // documents and locks the precedence a resolver must honour.
      let precedence = |l: Layer| match l {
          Layer::Program => 0,
          Layer::ProgramRef => 1,
          Layer::Instance => 2,
          Layer::Inferred => 3,
          Layer::UserEdit => 4,
      };
      assert!(precedence(instance_dpt.layer) > precedence(program_dpt.layer));
      assert!(!program_dpt.layer.is_exported());
      assert!(instance_dpt.layer.is_exported());
  }

  #[test]
  fn com_object_instance_without_dpt_is_constructible() {
      let com = ComObjectInstance {
          id: ComObjectInstanceId(1),
          source: SourceRef { path: "t".into(), ets_id: "t".into() },
          device: DeviceId(1),
          number: 0,
          text: Resolved { value: LocalizedString(TranslationKey("t".into())), layer: Layer::Program },
          description: None,
          dpt: None,   // a com object instance may have no resolved DPT at all
          flags: Resolved {
              value: ComFlags { read: false, write: true, transmit: false, update: false, communication: true },
              layer: Layer::Program,
          },
          size: Resolved { value: ObjectSize::Bit(1), layer: Layer::Program },
          is_active: true,
          links: vec![],
      };
      assert!(com.dpt.is_none());
  }
  ```
- [ ] Write a failing test in `parameter.rs`:
  ```rust
  #[test]
  fn parameter_instance_holds_raw_value_uninterpreted() {
      let p = ParameterInstance {
          id: ParameterInstanceId(1),
          source: SourceRef { path: "t".into(), ets_id: "M-x_P-1".into() },
          raw: "42".into(),
      };
      assert_eq!(p.raw, "42");
  }
  ```
- [ ] Run tests, expect FAIL (types don't exist).
- [ ] Implement `device.rs` and `parameter.rs`.
- [ ] Run tests, expect PASS.
- [ ] Commit: `knx-core: add device instances, communication object instances, parameters`.

---

### Task 9: Devices map, installation, project root, id allocation

**Files:**
- Create: `crates/knx-core/src/devices.rs`
- Create: `crates/knx-core/src/installation.rs`
- Create: `crates/knx-core/src/project.rs`
- Modify: `crates/knx-core/src/lib.rs`

**Interfaces:**
- Produces (`devices.rs`):
  ```rust
  pub struct Devices {
      by_id: std::collections::BTreeMap<DeviceId, DeviceInstance>,
      com_objects: std::collections::BTreeMap<ComObjectInstanceId, ComObjectInstance>,
  }
  impl Devices {
      pub fn new() -> Self;
      pub fn insert(&mut self, device: DeviceInstance);
      pub fn get(&self, id: DeviceId) -> Option<&DeviceInstance>;
      pub fn get_mut(&mut self, id: DeviceId) -> Option<&mut DeviceInstance>;
      pub fn remove(&mut self, id: DeviceId) -> Option<DeviceInstance>;
      pub fn iter(&self) -> impl Iterator<Item = &DeviceInstance>;
      pub fn insert_com_object(&mut self, com: ComObjectInstance);
      pub fn com_object(&self, id: ComObjectInstanceId) -> Option<&ComObjectInstance>;
      pub fn com_object_mut(&mut self, id: ComObjectInstanceId) -> Option<&mut ComObjectInstance>;
  }
  ```
- Produces (`installation.rs`):
  ```rust
  pub struct Installation {
      pub id: InstallationId,
      pub name: String,
      pub default_line: Option<LineId>,
      pub multicast_address: Option<std::net::Ipv4Addr>, // IPRoutingMulticastAddress
      pub completion: CompletionStatus,
      pub topology: Topology,
      pub buildings: Vec<BuildingPart>,
      pub group_ranges: Vec<GroupRange>,
      pub group_addresses: Vec<GroupAddressEntry>,
      pub parameters: Vec<ParameterInstance>,
  }
  ```
- Produces (`project.rs`):
  ```rust
  pub const CURRENT_SCHEMA_VERSION: u32 = 1;

  #[derive(Default)]
  pub struct IdAllocators {
      device: u32, area: u32, line: u32, com_object_instance: u32,
      group_range: u32, group_address: u32, building_part: u32,
      parameter_instance: u32,
  }
  impl IdAllocators {
      pub fn next_device_id(&mut self) -> DeviceId;
      pub fn next_area_id(&mut self) -> AreaId;
      pub fn next_line_id(&mut self) -> LineId;
      pub fn next_com_object_instance_id(&mut self) -> ComObjectInstanceId;
      pub fn next_group_range_id(&mut self) -> GroupRangeId;
      pub fn next_group_address_id(&mut self) -> GroupAddressId;
      pub fn next_building_part_id(&mut self) -> BuildingPartId;
      pub fn next_parameter_instance_id(&mut self) -> ParameterInstanceId;
  }

  pub struct Project {
      pub schema_version: u32,
      pub strings: StringTable,
      pub installations: Vec<Installation>,
      pub devices: Devices,
      pub ids: IdAllocators,
  }
  impl Project {
      pub fn new(default_language: Language) -> Self; // schema_version = CURRENT_SCHEMA_VERSION
  }
  ```
  Each `next_*_id` increments its counter starting from 1 (0 is never
  allocated, so it stays free for tests to use as an obviously-fake id, as
  several earlier tasks' tests already do).
- Consumes: everything from Tasks 1–8.
- Produces (for later tasks): `Devices`, `Installation`, `Project`,
  `IdAllocators`, `CURRENT_SCHEMA_VERSION` — used by `validation.rs`,
  `commands.rs`, and Session 3's importer.

Steps:
- [ ] Write failing tests in `project.rs`:
  ```rust
  #[test]
  fn id_allocator_starts_at_one_and_increments() {
      let mut ids = IdAllocators::default();
      assert_eq!(ids.next_device_id(), DeviceId(1));
      assert_eq!(ids.next_device_id(), DeviceId(2));
      assert_eq!(ids.next_group_address_id(), GroupAddressId(1));
  }

  #[test]
  fn new_project_carries_current_schema_version() {
      let p = Project::new(Language("en".into()));
      assert_eq!(p.schema_version, CURRENT_SCHEMA_VERSION);
      assert!(p.installations.is_empty());
  }
  ```
- [ ] Write a failing test in `devices.rs`:
  ```rust
  #[test]
  fn devices_is_the_sole_owner_lookup_by_id() {
      let mut d = Devices::new();
      d.insert(DeviceInstance {
          id: DeviceId(1), source: SourceRef { path: "t".into(), ets_id: "t".into() },
          name: "Dev".into(), description: None, address: None,
          product_ref: "P".into(), program_ref: "H".into(),
          commissioning: CommissioningState::default(), visibility_calculated: true,
          com_objects: vec![],
      });
      assert!(d.get(DeviceId(1)).is_some());
      assert!(d.get(DeviceId(2)).is_none());
  }
  ```
- [ ] Run tests, expect FAIL.
- [ ] Implement `devices.rs`, `installation.rs`, `project.rs` in that order
  (each depends only on earlier ones plus Tasks 1–8).
- [ ] Run `cargo test -p knx-core`, expect all PASS including every earlier
  task's tests.
- [ ] Commit: `knx-core: add devices map, installation, and project root`.

---

### Task 10: Validation rules

**Files:**
- Create: `crates/knx-core/src/validation.rs`
- Modify: `crates/knx-core/src/lib.rs`

**Interfaces:**
- Produces:
  ```rust
  pub enum ValidationError {
      DuplicateIndividualAddress { address: IndividualAddress, existing: DeviceId, new: DeviceId },
      GroupAddressOutsideRange { address: GroupAddress, range: GroupRangeId },
      DanglingGroupLink { com_object: ComObjectInstanceId, ga: GroupAddressId },
  }
  impl std::error::Error for ValidationError {}
  impl fmt::Display for ValidationError

  pub fn check_no_duplicate_individual_address(
      devices: &Devices, candidate: DeviceId, address: IndividualAddress,
  ) -> Result<(), ValidationError>;

  pub fn check_group_address_in_range(
      range: &GroupRange, address: GroupAddress,
  ) -> Result<(), ValidationError>;

  pub fn check_group_link_target_exists(
      installation: &Installation, com_object: ComObjectInstanceId, ga: GroupAddressId,
  ) -> Result<(), ValidationError>;
  ```
- Consumes: `Devices`, `Installation` (Task 9), `DeviceId`,
  `ComObjectInstanceId`, `GroupAddressId`, `GroupRangeId` (Task 1),
  `IndividualAddress`, `GroupAddress` (Task 2), `GroupRange` (Task 6).
- Produces (for later tasks): `ValidationError` and the three check
  functions — used by `commands.rs`.

`check_no_duplicate_individual_address` scans `devices.iter()` for any
device other than `candidate` whose `address == Some(address)`.
`check_group_address_in_range` calls `range.contains(address)`.
`check_group_link_target_exists` scans `installation.group_addresses` for an
entry whose `id == ga`.

Steps:
- [ ] Write failing tests:
  ```rust
  #[test]
  fn duplicate_individual_address_is_rejected() {
      let mut devices = Devices::new();
      devices.insert(make_device(DeviceId(1), Some(IndividualAddress::new(1, 1, 1).unwrap())));
      let err = check_no_duplicate_individual_address(
          &devices, DeviceId(2), IndividualAddress::new(1, 1, 1).unwrap(),
      );
      assert!(matches!(err, Err(ValidationError::DuplicateIndividualAddress { .. })));
  }

  #[test]
  fn same_device_reusing_its_own_address_is_not_a_duplicate() {
      let mut devices = Devices::new();
      let addr = IndividualAddress::new(1, 1, 1).unwrap();
      devices.insert(make_device(DeviceId(1), Some(addr)));
      assert!(check_no_duplicate_individual_address(&devices, DeviceId(1), addr).is_ok());
  }

  #[test]
  fn group_address_outside_range_is_rejected() {
      let range = make_range(GroupAddressId::default() /* placeholder, see below */);
      // build the range directly instead:
      let range = GroupRange {
          id: GroupRangeId(1), source: test_source(), name: "R".into(),
          start: GroupAddress::from_raw(0), end: GroupAddress::from_raw(10),
      };
      assert!(check_group_address_in_range(&range, GroupAddress::from_raw(11)).is_err());
      assert!(check_group_address_in_range(&range, GroupAddress::from_raw(5)).is_ok());
  }
  ```
  Drop the first `make_range` placeholder line above — it was scratch work,
  not part of the test; the real test builds `GroupRange` inline as shown
  in the second half. Add a small private test helper `fn make_device(id:
  DeviceId, address: Option<IndividualAddress>) -> DeviceInstance` and `fn
  test_source() -> SourceRef` at the top of the test module to keep the
  three tests above free of repeated boilerplate.
- [ ] Write one more failing test for the dangling-link check:
  ```rust
  #[test]
  fn group_link_to_nonexistent_address_is_dangling() {
      let installation = Installation {
          id: InstallationId(0), name: "I".into(), default_line: None,
          multicast_address: None, completion: CompletionStatus::FinishedDesign,
          topology: Topology { areas: vec![], lines: vec![], unassigned: vec![] },
          buildings: vec![], group_ranges: vec![], group_addresses: vec![],
          parameters: vec![],
      };
      let err = check_group_link_target_exists(&installation, ComObjectInstanceId(1), GroupAddressId(99));
      assert!(matches!(err, Err(ValidationError::DanglingGroupLink { .. })));
  }
  ```
- [ ] Run tests, expect FAIL.
- [ ] Implement `validation.rs`.
- [ ] Run tests, expect PASS.
- [ ] Commit: `knx-core: add validation rules`.

---

### Task 11: Command layer with undo/redo

**Files:**
- Create: `crates/knx-core/src/command.rs`
- Modify: `crates/knx-core/src/lib.rs`

**Interfaces:**
- Produces:
  ```rust
  pub enum Command {
      SetIndividualAddress { device: DeviceId, address: Option<IndividualAddress> },
      SetComObjectDpt { com_object: ComObjectInstanceId, dpt: Option<DptRef> },
      CreateGroupAddress { entry: GroupAddressEntry },   // id pre-allocated by the caller
      DeleteGroupAddress { id: GroupAddressId },
  }

  pub enum CommandError {
      Validation(ValidationError),
      DeviceNotFound(DeviceId),
      ComObjectNotFound(ComObjectInstanceId),
      GroupAddressNotFound(GroupAddressId),
      InstallationNotFound,
  }
  impl std::error::Error for CommandError {}
  impl fmt::Display for CommandError
  impl From<ValidationError> for CommandError

  impl Command {
      /// Applies the command to `project`'s first installation (the model
      /// supports multiple installations; Session 2's commands operate on
      /// `installations[0]`, which is the only one any current test or the
      /// reference project populates — a multi-installation command target
      /// is future work, not silently assumed solved here).
      /// Returns the inverse command on success.
      pub fn apply(&self, project: &mut Project) -> Result<Command, CommandError>;
  }

  pub struct CommandStack {
      undo: Vec<Command>,
      redo: Vec<Command>,
  }
  impl CommandStack {
      pub fn new() -> Self;
      pub fn do_command(&mut self, project: &mut Project, cmd: Command) -> Result<(), CommandError>;
      pub fn undo(&mut self, project: &mut Project) -> Result<(), CommandError>;
      pub fn redo(&mut self, project: &mut Project) -> Result<(), CommandError>;
      pub fn can_undo(&self) -> bool;
      pub fn can_redo(&self) -> bool;
  }
  ```
- Consumes: `Project`, `Devices` (Task 9), `ValidationError` and its check
  functions (Task 10), `GroupAddressEntry` (Task 6), `DeviceId`,
  `ComObjectInstanceId`, `GroupAddressId` (Task 1), `IndividualAddress`
  (Task 2), `DptRef` (Task 3), `Resolved`, `Layer` (Task 1 re-export).

`SetIndividualAddress::apply`: look up the device via
`project.devices.get_mut(device)`, else `CommandError::DeviceNotFound`. If
`address` is `Some`, run `check_no_duplicate_individual_address` against
`project.devices` first (excluding the device itself, since it is the
candidate) and propagate its error via `From`. Record the device's current
`address` as the inverse's `address` field, then set the new one.

`SetComObjectDpt::apply`: look up the com object via
`project.devices.com_object_mut(com_object)`, else
`CommandError::ComObjectNotFound`. Record the current `dpt` (the whole
`Option<Resolved<DptRef>>`, needed to restore both value and layer on
undo — the inverse command only carries the bare `Option<DptRef>`, so wrap
whatever layer it restores to as `Layer::UserEdit` too; a command's inverse
always re-applies through the same command variant, so this is
self-consistent). Set the new value: `Some(Resolved { value: dpt, layer:
Layer::UserEdit })` if `dpt.is_some()`, else `None`.

`CreateGroupAddress::apply`: push `entry.clone()` onto
`project.installations[0].group_addresses`, else
`CommandError::InstallationNotFound` if there is no installation. Inverse is
`DeleteGroupAddress { id: entry.id }`.

`DeleteGroupAddress::apply`: find and remove the entry with the given `id`
from `project.installations[0].group_addresses`, else
`CommandError::GroupAddressNotFound`. Inverse is `CreateGroupAddress {
entry: <the removed entry> }`.

Steps:
- [ ] Add `#[derive(Clone)]` to `GroupAddressEntry` (Task 6) if not already
  present — `CreateGroupAddress`/`DeleteGroupAddress` need to clone entries
  into their inverse commands.
- [ ] Write failing tests:
  ```rust
  fn test_project_with_one_device(address: Option<IndividualAddress>) -> Project {
      let mut p = Project::new(Language("en".into()));
      p.installations.push(Installation {
          id: InstallationId(0), name: "I".into(), default_line: None,
          multicast_address: None, completion: CompletionStatus::FinishedDesign,
          topology: Topology { areas: vec![], lines: vec![], unassigned: vec![] },
          buildings: vec![], group_ranges: vec![], group_addresses: vec![],
          parameters: vec![],
      });
      p.devices.insert(DeviceInstance {
          id: DeviceId(1), source: SourceRef { path: "t".into(), ets_id: "t".into() },
          name: "D".into(), description: None, address,
          product_ref: "P".into(), program_ref: "H".into(),
          commissioning: CommissioningState::default(), visibility_calculated: true,
          com_objects: vec![],
      });
      p
  }

  #[test]
  fn set_individual_address_then_undo_restores_previous_value() {
      let mut project = test_project_with_one_device(None);
      let mut stack = CommandStack::new();
      let addr = IndividualAddress::new(1, 1, 1).unwrap();
      stack.do_command(&mut project, Command::SetIndividualAddress { device: DeviceId(1), address: Some(addr) }).unwrap();
      assert_eq!(project.devices.get(DeviceId(1)).unwrap().address, Some(addr));
      stack.undo(&mut project).unwrap();
      assert_eq!(project.devices.get(DeviceId(1)).unwrap().address, None);
      stack.redo(&mut project).unwrap();
      assert_eq!(project.devices.get(DeviceId(1)).unwrap().address, Some(addr));
  }

  #[test]
  fn set_individual_address_rejects_duplicate() {
      let mut project = test_project_with_one_device(Some(IndividualAddress::new(1, 1, 1).unwrap()));
      project.devices.insert(DeviceInstance {
          id: DeviceId(2), source: SourceRef { path: "t".into(), ets_id: "t".into() },
          name: "D2".into(), description: None, address: None,
          product_ref: "P".into(), program_ref: "H".into(),
          commissioning: CommissioningState::default(), visibility_calculated: true,
          com_objects: vec![],
      });
      let mut stack = CommandStack::new();
      let result = stack.do_command(&mut project, Command::SetIndividualAddress {
          device: DeviceId(2), address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
      });
      assert!(matches!(result, Err(CommandError::Validation(ValidationError::DuplicateIndividualAddress { .. }))));
      // rejected command must not land on the undo stack
      assert!(!stack.can_undo());
  }

  #[test]
  fn create_then_delete_group_address_round_trips_through_undo() {
      let mut project = test_project_with_one_device(None);
      let mut stack = CommandStack::new();
      let entry = GroupAddressEntry {
          id: GroupAddressId(1), source: SourceRef { path: "t".into(), ets_id: "t".into() },
          name: "GA".into(), address: GroupAddress::from_raw(1), central: false,
          unfiltered: false, range: None,
      };
      stack.do_command(&mut project, Command::CreateGroupAddress { entry: entry.clone() }).unwrap();
      assert_eq!(project.installations[0].group_addresses.len(), 1);
      stack.do_command(&mut project, Command::DeleteGroupAddress { id: GroupAddressId(1) }).unwrap();
      assert!(project.installations[0].group_addresses.is_empty());
      stack.undo(&mut project).unwrap(); // undoes the delete -> recreates
      assert_eq!(project.installations[0].group_addresses.len(), 1);
      stack.undo(&mut project).unwrap(); // undoes the create -> empty again
      assert!(project.installations[0].group_addresses.is_empty());
  }

  #[test]
  fn set_com_object_dpt_marks_layer_as_user_edit_and_undoes() {
      let mut project = test_project_with_one_device(None);
      let com = ComObjectInstance {
          id: ComObjectInstanceId(1), source: SourceRef { path: "t".into(), ets_id: "t".into() },
          device: DeviceId(1), number: 0,
          text: Resolved { value: LocalizedString(TranslationKey("t".into())), layer: Layer::Program },
          description: None,
          dpt: Some(Resolved { value: DptRef { main: 1, sub: Some(1) }, layer: Layer::Program }),
          flags: Resolved {
              value: ComFlags { read: true, write: false, transmit: false, update: false, communication: true },
              layer: Layer::Program,
          },
          size: Resolved { value: ObjectSize::Bit(1), layer: Layer::Program },
          is_active: true, links: vec![],
      };
      project.devices.insert_com_object(com);
      let mut stack = CommandStack::new();
      let new_dpt = DptRef { main: 5, sub: Some(1) };
      stack.do_command(&mut project, Command::SetComObjectDpt { com_object: ComObjectInstanceId(1), dpt: Some(new_dpt) }).unwrap();
      let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
      assert_eq!(updated.dpt.as_ref().unwrap().value, new_dpt);
      assert_eq!(updated.dpt.as_ref().unwrap().layer, Layer::UserEdit);
      stack.undo(&mut project).unwrap();
      let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
      assert_eq!(restored.dpt.as_ref().unwrap().value, DptRef { main: 1, sub: Some(1) });
      assert_eq!(restored.dpt.as_ref().unwrap().layer, Layer::Program);
  }
  ```
- [ ] Run tests, expect FAIL.
- [ ] Implement `command.rs` per the interfaces and per-variant behaviour
  described above. `do_command` clears `redo` on a successful new command
  (standard undo/redo semantics); a failed command leaves both stacks
  untouched.
- [ ] Run tests, expect PASS.
- [ ] Run the full workspace check listed in Global Constraints.
- [ ] Commit: `knx-core: add command layer with undo and redo`.

---

### Task 12: `knx-store` migration-chain skeleton

**Files:**
- Modify: `Cargo.toml` (workspace root, add `rusqlite` to
  `[workspace.dependencies]`)
- Modify: `crates/knx-store/Cargo.toml` (add `rusqlite.workspace = true`)
- Create: `crates/knx-store/src/migration.rs`
- Modify: `crates/knx-store/src/lib.rs`
- Create: `crates/knx-store/fixtures/v1-empty.sqlite` (binary fixture,
  committed once)

**Interfaces:**
- Produces:
  ```rust
  pub const CURRENT_SCHEMA_VERSION: i64 = 1; // matches knx_core::project::CURRENT_SCHEMA_VERSION

  pub enum MigrationError {
      Sqlite(rusqlite::Error),
      FutureSchemaVersion { found: i64, supported: i64 },
  }
  impl std::error::Error for MigrationError {}
  impl fmt::Display for MigrationError
  impl From<rusqlite::Error> for MigrationError

  /// Ordered chain, index i migrates user_version i -> i+1.
  /// v0 -> v1 creates a single `schema_meta` marker table; entity tables
  /// arrive with Session 3's importer, once there is data to store.
  fn migrations() -> Vec<fn(&rusqlite::Connection) -> Result<(), MigrationError>>;

  /// Opens (creating if absent) the SQLite file at `path`, runs every
  /// pending migration in order, and returns the connection at
  /// `CURRENT_SCHEMA_VERSION`. Errors if the file's `user_version` is
  /// already ahead of `CURRENT_SCHEMA_VERSION` (no downgrade path, per
  /// ADR-0003).
  pub fn open_and_migrate(path: &std::path::Path) -> Result<rusqlite::Connection, MigrationError>;
  ```
- Consumes: nothing from `knx-core` yet (entity persistence is Session 3);
  this task only proves the version/migration mechanics.

Steps:
- [ ] Add to workspace root `Cargo.toml` under `[workspace.dependencies]`:
  `rusqlite = { version = "0.40", features = ["bundled"] }`.
- [ ] Add `rusqlite.workspace = true` to `crates/knx-store/Cargo.toml`'s
  `[dependencies]`.
- [ ] Write `migration.rs` with the types and functions above. `v0 -> v1`
  runs `CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT
  NULL) STRICT;` then `INSERT INTO schema_meta (key, value) VALUES
  ('created_by', 'knx-store')`. After running all pending migrations,
  `PRAGMA user_version = <CURRENT_SCHEMA_VERSION>;`.
- [ ] Write failing tests in `migration.rs`:
  ```rust
  #[test]
  fn fresh_file_migrates_to_current_version() {
      let dir = tempfile::tempdir().unwrap();
      let path = dir.path().join("p.sqlite");
      let conn = open_and_migrate(&path).unwrap();
      let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
      assert_eq!(version, CURRENT_SCHEMA_VERSION);
      let marker: String = conn.query_row(
          "SELECT value FROM schema_meta WHERE key = 'created_by'", [], |r| r.get(0),
      ).unwrap();
      assert_eq!(marker, "knx-store");
  }

  #[test]
  fn reopening_an_already_migrated_file_is_a_no_op() {
      let dir = tempfile::tempdir().unwrap();
      let path = dir.path().join("p.sqlite");
      open_and_migrate(&path).unwrap();
      let conn = open_and_migrate(&path).unwrap(); // second open must not fail or re-run migrations
      let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
      assert_eq!(version, CURRENT_SCHEMA_VERSION);
  }

  #[test]
  fn a_file_from_a_newer_schema_version_is_rejected() {
      let dir = tempfile::tempdir().unwrap();
      let path = dir.path().join("p.sqlite");
      {
          let conn = rusqlite::Connection::open(&path).unwrap();
          conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1).unwrap();
      }
      assert!(matches!(
          open_and_migrate(&path),
          Err(MigrationError::FutureSchemaVersion { .. })
      ));
  }

  #[test]
  fn the_frozen_v1_fixture_still_opens() {
      let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v1-empty.sqlite");
      let conn = open_and_migrate(std::path::Path::new(fixture)).unwrap();
      let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
      assert_eq!(version, CURRENT_SCHEMA_VERSION);
  }
  ```
- [ ] Add `tempfile` as a dev-dependency: `tempfile.workspace = true` in
  `crates/knx-store/Cargo.toml`'s new `[dev-dependencies]` section, and
  `tempfile = "3"` under `[workspace.dependencies]` (MIT/Apache-2.0,
  already permitted).
- [ ] Run tests except the fixture one, expect FAIL (module/functions don't
  exist).
- [ ] Implement `migration.rs`. Set `pub mod migration;` and `pub use
  migration::{open_and_migrate, MigrationError, CURRENT_SCHEMA_VERSION};`
  in `crates/knx-store/src/lib.rs`, replacing its current bare doc comment
  (keep the doc comment as the module-level doc above the new `pub mod`
  line).
- [ ] Generate the frozen fixture once real code exists: run `cargo test -p
  knx-store fresh_file_migrates_to_current_version -- --nocapture`, then
  separately create `crates/knx-store/fixtures/v1-empty.sqlite` by calling
  `open_and_migrate` against that exact path from a scratch `main.rs` or
  test, and commit the resulting binary file. It must never be regenerated
  after this commit (ADR-0003: "a migration that cannot open its
  predecessor's fixture is a failing test, not a release note").
- [ ] Run `cargo test -p knx-store`, expect all four tests PASS.
- [ ] Commit: `knx-store: add schema-version migration chain skeleton`.

---

### Task 13: Documentation and final workspace check

**Files:**
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/ROADMAP.md`
- Modify: `docs/DATA_MODEL.md`
- Modify: `docs/RESEARCH.md` (append the `ComObjectRef@DatapointType`
  space-separated-list discovery from Task 3 to the relevant section, e.g.
  near §3.2)

Steps:
- [ ] In `docs/DATA_MODEL.md`, change every `*Planned.*` marker for
  sections 2, 4, 5, 6, 7, 8, 9, 11 to `*Implemented.*`, and update section
  3's status line to note the command layer now also lives in `knx-core`.
  Leave section 10 (`*Retained but uninterpreted.*`) as is — the opaque
  store itself is not built this session, only `ParameterInstance`.
- [ ] In `docs/ROADMAP.md`, mark the Session 2 entry condition / deliverable
  list as done, matching what was actually built (call out explicitly that
  the opaque passthrough store is not part of this session's delivery,
  since DATA_MODEL §10 lists it separately from the entities this plan
  covers).
- [ ] In `docs/IMPLEMENTATION_STATUS.md`: update the status table (row 2 to
  **Done**), rewrite the "What exists" table's `knx-core` and `knx-store`
  rows to describe the new module layout, and rewrite "Next session" to
  point at Session 3 (ETS project import) with its entry condition
  (workspace builds, both gates pass, this plan's tests green).
- [ ] In `docs/RESEARCH.md`, add one short paragraph documenting the
  `ComObjectRef@DatapointType` space-separated-list finding from Task 3,
  citing the two example files
  (`M-0083/M-0083_A-0019-13-A892.xml`) and the observed value
  `"DPST-9-21 DPST-9-21"`, and note it is unhandled until `knx-productdb`
  (Session 4) parses application programs.
- [ ] Run the full workspace check from Global Constraints one final time:
  `cargo build --workspace && cargo test --workspace && cargo fmt --all
  --check && cargo clippy --workspace --all-targets -- -D warnings && cargo
  run -p xtask -- check-layering && cargo deny check`. All must pass.
- [ ] Commit: `docs: mark Session 2 (KNX core) complete`.
