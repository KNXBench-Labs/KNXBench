# Design: entity persistence for `knx_core::Project` in `knx-store`

Date: 2026-09-03
Session: 5 (UI/UX), cycle 2 of N
Status: Approved

## Context

[ROADMAP.md](../../ROADMAP.md) / [IMPLEMENTATION_STATUS.md](../../IMPLEMENTATION_STATUS.md)
carry this forward from Session 3 and Session 4 in turn: `knx-store` persists
only the opaque passthrough table (schema v2) and the manufacturer manifest
(schema v3). No table for any `knx_core::Project` entity exists. Cycle 1 of
Session 5 (`docs/superpowers/specs/2026-09-03-knx-desktop-shell-design.md`)
explicitly deferred this — "open project" there means import-and-display
only, with an in-memory connection discarded on drop.

This is a hard prerequisite for the desktop app to ever save an edit: without
it, `knx-core`'s command layer (`command.rs`, undo/redo, `SetIndividualAddress`
etc.) can mutate the in-memory `Project` but nothing survives closing the app.
It belongs early in Session 5 per IMPLEMENTATION_STATUS's own "Next session"
note, rather than being deferred a third time.

`knx-store` already depends on `knx-core` (`crates/knx-store/Cargo.toml`).
Persistence works directly with `knx_core` types — no `Stored*` mirror types
per entity. (`StoredOpaqueEntry`/`ManifestRef` mirror `knx-etsproj`'s/
`knx-productdb`'s own row shapes instead, which have no `knx-core` type to
begin with; that pattern does not apply here.)

## Goal (this cycle)

`knx-store` can save a full `knx_core::Project` to SQLite and load it back
losslessly, and can apply the four `knx-core::command::Command` variants that
exist today as targeted single/few-row updates rather than a full re-save.
Desktop UI wiring (a `save_project`/`load_project` Tauri command, a save
dialog) is explicitly out of scope — this cycle is the storage layer only.

## Decisions made during brainstorming

- **Full save is whole-project replace, not diffed.** One transaction:
  delete every entity table's rows, reinsert from the in-memory `Project`.
  Simplest correct approach (CLAUDE.md: prefer the simpler approach absent a
  measured reason not to); a real diffing save is not attempted.
- **Incremental sync piggybacks on the command layer, not a general
  diff engine.** `sync_after_command(conn, &Project, &Command)` reads the
  *already-mutated* `Project` after `Command::apply` succeeded, and re-writes
  only the row(s) the command's own target id(s) name. It does not
  re-derive `command.rs`'s mutation logic (no duplicated business rules) —
  it reads the resulting state back out of `Project` and upserts that.
- **One row-writer per entity, shared between full save and command
  sync.** `save_project` calls `upsert_<entity>` for every entity in the
  project; `sync_after_command` calls the same functions for just the
  touched id(s). This is what keeps the incremental path from drifting out
  of sync with the full-save encoding.
- **The override chain (`Override<T>`, 7 attributes on `ComObjectInstance`)
  is stored as one normalized table**, `com_object_override` — one row per
  `(com_object_instance_id, attr)` — rather than 21+ wide columns on
  `com_object_instance`. This is also what makes `SetComObjectDpt`'s
  incremental sync a single targeted upsert instead of a whole-row rewrite.
- **Honest scope on "incremental":** only the fields the four existing
  `Command` variants touch (device address, one com-object override
  attribute at a time, group address create/delete) get the incremental
  path today. Every other entity (`Installation`, `Area`, `Line`,
  `BuildingPart`, `ParameterInstance`, `StringTable`, the other six
  overridable attributes) has no command yet and is therefore only ever
  written by a full save, until commands for them exist. Documented as a
  known limitation, not silently implied to be complete.
- **`device.installation_id` is stored explicitly**, separate from
  `device.line_id` (nullable). A device with no line is valid
  (`Topology::unassigned`), but `unassigned` is a per-installation list —
  without `installation_id` a line-less device's installation membership
  would not be reconstructible on load.
- **`position` columns wherever a `Vec<T>` or `Vec<Id>`'s order is
  meaningful** in `knx-core` — SQLite gives no ordering guarantee
  otherwise, and losing input order on reload would be a silent data
  change the round-trip test would not even catch unless `PartialEq` is
  order-sensitive (it is: derived `Vec<T>` equality is). Two distinct kinds
  of ordering show up, and some entities need both:
  - **Owned-list order** — the entity has exactly one parent/container, so
    its order can live as a plain column on the entity's own row: `area
    .position` (within `Topology::areas`), `line.position` (within
    `Area::lines`), `device.topology_position` (within `Line::devices`
    when `line_id` is set, or `Topology::unassigned` when it is not — the
    two are mutually exclusive per device, so one column serves both),
    `com_object_instance.position` (within `DeviceInstance::com_objects`),
    `building_part.position` (sibling order under `parent_id`, used to
    rebuild `BuildingPart::children`), `group_range.position` (sibling
    order under `parent_id`, used to rebuild `GroupRange::children`).
  - **Flat-list order** — `Installation::buildings` and
    `Installation::group_ranges` are each a *flat* `Vec` of every node
    regardless of nesting depth, which is a second, independent ordering
    from the sibling order above. `building_part.flat_position` and
    `group_range.flat_position` carry this one. `Installation::
    group_addresses` and `Installation::parameters` are already flat with
    no separate hierarchy, so `group_address.position` and the new
    `parameter_instance.position` need only the one column each.
  - **Exception, reasoned not assumed:** `Project::installations: Vec<
    Installation>` gets no position column — `InstallationId` mirrors
    ETS's own installation number rather than a synthetic counter
    (`ids.rs`'s doc comment), so `ORDER BY id` reproduces ETS's own
    ordering deliberately, not by luck. Every other id used for ordering
    above is `IdAllocators`-issued and synthetic, which is precisely why
    those get real `position` columns instead of relying on id order.
  - `StringTable`'s entries need no position column: its `PartialEq`
    (added above) compares the underlying `HashMap`, which is already
    order-insensitive.
- **`PRAGMA foreign_keys = ON`**, not currently set anywhere in
  `knx-store`. A real relational schema now exists; referential integrity
  should be enforced by SQLite itself (CLAUDE.md priority: data integrity).
- **Small, additive `knx-core` change: derive `PartialEq` on `Project`,
  `Devices`, `StringTable`, `IdAllocators`.** None of the four derives it
  today. Needed for a round-trip test (`save → load → assert Project ==
  Project`) to be possible at all without a hand-rolled comparison helper.
  Derive-only, no behavior change — flagged here explicitly because it
  touches `knx-core`, not because it is risky.
- **`StringTable` also gains a public `iter()`.** Its entries are private
  (`HashMap<(TranslationKey, Language), String>`); persistence cannot save
  what it cannot enumerate. `pub fn iter(&self) -> impl Iterator<Item =
  (&TranslationKey, &Language, &str)>` is additive, no existing behavior
  changes.

## Architecture

```
crates/knx-store/src/
  project.rs        NEW. save_project, load_project — orchestrates every
                     other module below in dependency order. project_info
                     and id_allocators tables live here (both single-row).
  strings.rs         NEW. string_table_entry table.
  topology.rs        NEW. installation, area, line tables.
  building.rs        NEW. building_part, building_part_device tables.
  devices.rs         NEW. device, binary_data_ref, com_object_instance,
                     com_object_override, group_link tables. Also the
                     per-attribute Override<T> <-> row codec.
  group.rs           NEW. group_range, group_address tables.
  parameter.rs       NEW. parameter_instance table.
  command_sync.rs     NEW. sync_after_command(conn, &Project, &Command) —
                     dispatches to the upsert/delete fns above per variant.
  migration.rs       CHANGED. migrate_v3_to_v4 creates every table above;
                     CURRENT_SCHEMA_VERSION -> 4; fixtures/v4-empty.sqlite.
  manifest.rs        unchanged
  opaque.rs          unchanged
  lib.rs             CHANGED. re-exports save_project/load_project/
                     sync_after_command and the module tree above.
```

`knx-core` (`crates/knx-core/src/{project,devices,string_table}.rs`): add
`#[derive(PartialEq)]` (`Project`, `Devices`, `StringTable`) and
`#[derive(PartialEq)]` on `IdAllocators` (`project.rs`, currently only
`Debug, Clone, Default`). No other change.

## Schema v4

```sql
CREATE TABLE project_info (
    id                  INTEGER PRIMARY KEY CHECK (id = 0),
    project_id          TEXT NOT NULL,
    name                TEXT NOT NULL,
    project_number      TEXT,
    group_address_style TEXT NOT NULL,   -- 'Free' | 'TwoLevel' | 'ThreeLevel'
    completion          TEXT NOT NULL,
    last_modified       TEXT,            -- RFC3339
    project_start       TEXT,
    default_language    TEXT NOT NULL    -- StringTable::default_language
) STRICT;

CREATE TABLE id_allocators (
    id                    INTEGER PRIMARY KEY CHECK (id = 0),
    device                INTEGER NOT NULL,
    area                  INTEGER NOT NULL,
    line                  INTEGER NOT NULL,
    com_object_instance   INTEGER NOT NULL,
    group_range           INTEGER NOT NULL,
    group_address         INTEGER NOT NULL,
    building_part         INTEGER NOT NULL,
    parameter_instance    INTEGER NOT NULL
) STRICT;

CREATE TABLE string_table_entry (
    key      TEXT NOT NULL,
    language TEXT NOT NULL,
    value    TEXT NOT NULL,
    PRIMARY KEY (key, language)
) STRICT;

CREATE TABLE installation (
    id                INTEGER PRIMARY KEY,   -- InstallationId (u8)
    name              TEXT NOT NULL,
    default_line_id   INTEGER REFERENCES line(id),
    multicast_address TEXT,
    completion        TEXT NOT NULL
) STRICT;

CREATE TABLE area (
    id              INTEGER PRIMARY KEY,
    installation_id INTEGER NOT NULL REFERENCES installation(id),
    position        INTEGER NOT NULL,   -- order within Topology::areas
    source_path     TEXT NOT NULL,
    source_ets_id   TEXT NOT NULL,
    name            TEXT NOT NULL,
    address         INTEGER NOT NULL,
    completion      TEXT NOT NULL
) STRICT;

CREATE TABLE line (
    id                           INTEGER PRIMARY KEY,
    area_id                      INTEGER NOT NULL REFERENCES area(id),
    position                     INTEGER NOT NULL,   -- order within Area::lines
    source_path                  TEXT NOT NULL,
    source_ets_id                TEXT NOT NULL,
    name                         TEXT NOT NULL,
    address                      INTEGER NOT NULL,
    medium_ref                   TEXT NOT NULL,
    domain_address               TEXT,
    domain_address_is_checked    INTEGER,   -- 0/1, nullable
    ip_routing_multicast_address TEXT,
    multicast_ttl                INTEGER,
    completion                   TEXT NOT NULL
) STRICT;

CREATE TABLE building_part (
    id              INTEGER PRIMARY KEY,
    installation_id INTEGER NOT NULL REFERENCES installation(id),
    parent_id       INTEGER REFERENCES building_part(id),
    position        INTEGER NOT NULL,   -- sibling order under parent_id (rebuilds ::children)
    flat_position   INTEGER NOT NULL,   -- order within Installation::buildings (flat)
    source_path     TEXT NOT NULL,
    source_ets_id   TEXT NOT NULL,
    name            TEXT NOT NULL,
    number          TEXT,
    kind            TEXT NOT NULL,   -- BuildingPartType variant name
    default_line_id INTEGER REFERENCES line(id),
    completion      TEXT NOT NULL
) STRICT;

CREATE TABLE building_part_device (
    building_part_id INTEGER NOT NULL REFERENCES building_part(id),
    device_id        INTEGER NOT NULL REFERENCES device(id),
    position         INTEGER NOT NULL,
    PRIMARY KEY (building_part_id, device_id)
) STRICT;

CREATE TABLE device (
    id                          INTEGER PRIMARY KEY,
    installation_id             INTEGER NOT NULL REFERENCES installation(id),
    line_id                     INTEGER REFERENCES line(id),   -- NULL = unassigned
    topology_position           INTEGER NOT NULL,   -- order within Line::devices
                                                      -- (line_id set) or Topology::unassigned
                                                      -- (line_id NULL) — mutually exclusive
    source_path                 TEXT NOT NULL,
    source_ets_id               TEXT NOT NULL,
    name                        TEXT NOT NULL,
    description                 TEXT,
    address                     INTEGER,   -- IndividualAddress raw u16, nullable
    product_ref                 TEXT NOT NULL,
    program_ref                 TEXT NOT NULL,
    completion                  TEXT NOT NULL,
    individual_address_loaded   INTEGER NOT NULL,
    application_program_loaded  INTEGER NOT NULL,
    parameters_loaded           INTEGER NOT NULL,
    communication_part_loaded   INTEGER NOT NULL,
    medium_config_loaded        INTEGER NOT NULL,
    last_modified               TEXT,
    last_download               TEXT,
    broken                      INTEGER NOT NULL,
    visibility_calculated       INTEGER NOT NULL
) STRICT;

CREATE TABLE binary_data_ref (
    device_id INTEGER NOT NULL REFERENCES device(id),
    position  INTEGER NOT NULL,
    blob_id   TEXT NOT NULL,
    name      TEXT NOT NULL,
    PRIMARY KEY (device_id, position)
) STRICT;

CREATE TABLE com_object_instance (
    id            INTEGER PRIMARY KEY,
    device_id     INTEGER NOT NULL REFERENCES device(id),
    position      INTEGER NOT NULL,   -- order within DeviceInstance::com_objects
    source_path   TEXT NOT NULL,
    source_ets_id TEXT NOT NULL,
    number        INTEGER NOT NULL,
    size_kind     TEXT,      -- 'bit' | 'byte', NULL if size is None
    size_value    INTEGER,
    size_layer    TEXT,
    is_active     INTEGER NOT NULL
) STRICT;

-- One row per overridable attribute. attr in
-- ('text','description','dpt','read','write','transmit','update','communication').
-- state in ('absent','empty','value','malformed'). value/text_kind/layer are
-- only meaningful for state='value' (layer, and text_kind for text/description)
-- or state='malformed' (value holds the raw text).
CREATE TABLE com_object_override (
    com_object_instance_id INTEGER NOT NULL REFERENCES com_object_instance(id),
    attr                    TEXT NOT NULL,
    state                   TEXT NOT NULL,
    value                   TEXT,
    text_kind               TEXT,   -- 'literal' | 'localized', text/description only
    layer                   TEXT,
    PRIMARY KEY (com_object_instance_id, attr)
) STRICT;

CREATE TABLE group_link (
    com_object_instance_id INTEGER NOT NULL REFERENCES com_object_instance(id),
    group_address_id        INTEGER NOT NULL REFERENCES group_address(id),
    direction                TEXT NOT NULL,   -- 'send' | 'receive'
    position                 INTEGER NOT NULL,
    PRIMARY KEY (com_object_instance_id, position)
) STRICT;

CREATE TABLE group_range (
    id              INTEGER PRIMARY KEY,
    installation_id INTEGER NOT NULL REFERENCES installation(id),
    parent_id       INTEGER REFERENCES group_range(id),
    position        INTEGER NOT NULL,   -- sibling order under parent_id (rebuilds ::children)
    flat_position   INTEGER NOT NULL,   -- order within Installation::group_ranges (flat)
    source_path     TEXT NOT NULL,
    source_ets_id   TEXT NOT NULL,
    name            TEXT NOT NULL,
    range_start     INTEGER NOT NULL,
    range_end       INTEGER NOT NULL
) STRICT;

CREATE TABLE group_address (
    id              INTEGER PRIMARY KEY,
    installation_id INTEGER NOT NULL REFERENCES installation(id),
    range_id        INTEGER REFERENCES group_range(id),
    position        INTEGER NOT NULL,
    source_path     TEXT NOT NULL,
    source_ets_id   TEXT NOT NULL,
    name            TEXT NOT NULL,
    address         INTEGER NOT NULL,
    central         INTEGER NOT NULL,
    unfiltered      INTEGER NOT NULL
) STRICT;

CREATE TABLE parameter_instance (
    id            INTEGER PRIMARY KEY,
    device_id     INTEGER NOT NULL REFERENCES device(id),
    position      INTEGER NOT NULL,   -- order within Installation::parameters
    source_path   TEXT NOT NULL,
    source_ets_id TEXT NOT NULL,
    raw           TEXT NOT NULL
) STRICT;
```

Indexes: on every `*_id` foreign-key column used in a `WHERE` by a loader
(`device.line_id`, `device.installation_id`, `com_object_instance.device_id`,
`com_object_override.com_object_instance_id`, `group_link.com_object_instance_id`,
`group_address.range_id`, `building_part.parent_id`, `group_range.parent_id`,
`parameter_instance.device_id`) — same pattern as `opaque_entry_source_path`
and `manufacturer_ref_sha256` already in `migration.rs`.

## Full save / load

`save_project(conn: &Connection, project: &Project) -> Result<(), StoreError>`:
one transaction (`conn.unchecked_transaction()`, matching `opaque.rs`'s
existing pattern). Deletes every table above in child-to-parent order, then
calls each `upsert_*` function once per entity, parent-to-child (`installation`
before `area` before `line` before `device`, etc.), so foreign keys are
always satisfied at insert time under `PRAGMA foreign_keys = ON`.

`load_project(conn: &Connection) -> Result<Project, StoreError>`: reverse —
reads `project_info` and `id_allocators` first (both required, both
single-row; a database with no `project_info` row is a `StoreError`, not a
default-constructed empty project — an empty project is only ever the result
of `Project::new`, never of loading a database that was never saved),
reconstructs `StringTable`, then `Devices` (all devices and com-object
instances, keyed by id), then each `Installation` (topology, buildings, group
ranges/addresses, parameters), resolving id-lists (`Line::devices`,
`BuildingPart::children`/`::devices`, `GroupRange::children`) from the
child-side foreign key plus `position`, ordered by `position`.

## Incremental command sync

```rust
// knx-store/src/command_sync.rs
pub fn sync_after_command(
    conn: &Connection,
    project: &Project,   // already mutated by Command::apply
    command: &Command,
) -> Result<(), StoreError> {
    match command {
        Command::SetIndividualAddress { device, .. } => {
            devices::upsert_device(conn, project.devices.get(*device).unwrap())
        }
        Command::SetComObjectDpt { com_object, .. }
        | Command::RestoreComObjectDpt { com_object, .. } => {
            let com = project.devices.com_object(*com_object).unwrap();
            devices::upsert_com_object_override(conn, com.id, Attr::Dpt, &com.dpt)
        }
        Command::CreateGroupAddress { entry } => group::upsert_group_address(conn, entry),
        Command::DeleteGroupAddress { id } => group::delete_group_address(conn, *id),
    }
}
```

The `.unwrap()`s are deliberate, not sloppy: by the time `sync_after_command`
runs, `command.apply(project)` already succeeded, which is exactly the
guarantee that the target id exists in `project`. A lookup failure here
would mean `Command::apply`'s own postcondition broke, not a normal
runtime error — it stays a panic, matching how `command.rs` itself panics
after its own existence checks (see `SetIndividualAddress`'s
`.get_mut(device).unwrap()` right after the `ok_or` check earlier in the
same function).

Caller contract (documented on the function, enforced by `knx-app` once it
wires this in a later cycle, not by this cycle's tests beyond the function
itself): call `sync_after_command` only after a successful
`Command::apply`, with the resulting `Project`. Applies equally to undo/redo,
since both replay through the same `Command` enum (a `RestoreComObjectDpt`
undo is itself a `Command`, synced the same way).

## Error handling

`StoreError` (new, in `knx-store/src/lib.rs`, alongside the existing
`MigrationError`): wraps `rusqlite::Error`. `save_project`/`load_project`/
`sync_after_command` all return `Result<_, StoreError>`. No partial-write
possibility: `save_project` is one transaction; `sync_after_command`'s
handful of statements per call are wrapped in one transaction each for the
same reason `insert_opaque` already is.

## Testing

- **Migration**: `migrate_v3_to_v4` creates every table; frozen
  `fixtures/v4-empty.sqlite`; the v1/v2/v3 fixtures still migrate forward to
  v4 (extends the existing `the_frozen_v*_fixture_migrates_forward_to_v*`
  pattern one step).
- **Round trip**: import the reference `.knxproj` (already in the repo) via
  `knx_app::import_ets_project`, `save_project`, `load_project`, assert
  `Project == Project` (needs the `knx-core` `PartialEq` derives above).
  Also: an empty project (`Project::new`), a project with an unassigned
  device, a three-level-deep building hierarchy, a two-level group range
  nesting — each its own smaller round-trip test rather than relying on the
  reference project alone to exercise every shape.
- **Override chain**: a `com_object_override` round trip covering all four
  `Override<T>` states (`Absent`, `Empty`, `Value` at each `Layer`,
  `Malformed`) for both a `DptRef` attribute and a `bool` flag attribute,
  plus `Text::Literal` vs `Text::Localized` for `text`/`description`.
- **Command sync**: one test per `Command` variant — apply, sync, reload
  via `load_project`, assert only the touched entity changed (every other
  row byte-for-byte equal to before). Undo (apply the inverse `Command`
  returned by `apply`) round-trips back to the original state through the
  same mechanism.
- **`check-layering`**: no new root — `knx-store` already sits outside the
  four enforced roots (`knx-core`, `knx-etsproj`, `knx-productdb`,
  `knx-projection`) and stays there; this cycle adds no new dependency to
  `knx-store` itself (still just `knx-core` + `rusqlite`).

## Out of scope for this cycle

`save_project`/`load_project` Tauri commands, a desktop save dialog/UX,
`knx-app` wiring that calls `sync_after_command` from real UI actions,
incremental sync for any entity/attribute without a `Command` today
(explicitly listed above), optimistic concurrency / multi-writer conflict
detection, and undo/redo *history* persistence (only the *effect* of an
undo/redo command is persisted — the stack itself stays in-memory, same as
today).
