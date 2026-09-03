# Entity Persistence (knx-store) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `knx-store` can save a full `knx_core::Project` to SQLite and load it back losslessly, and can apply the four `knx-core::command::Command` variants as targeted row updates instead of a full re-save.

**Architecture:** Schema v4 adds one table per `knx-core` entity (normalized: the `Override<T>` chain becomes one row per attribute in `com_object_override`, not wide columns). One `upsert_<entity>`/`load_<entity>` pair per table, grouped into modules matching `knx-core`'s own module split. `save_project` calls every upsert in parent-to-child order inside one transaction; `sync_after_command` calls the same upserts for just the id(s) one `Command` touched, reading the already-mutated `Project`.

**Tech Stack:** Rust, `rusqlite` (already a `knx-store` dependency), SQLite `STRICT` tables, `PRAGMA foreign_keys = ON`.

**Spec:** [docs/superpowers/specs/2026-09-03-knx-entity-persistence-design.md](../specs/2026-09-03-knx-entity-persistence-design.md)

## Global Constraints

- `knx-store` depends only on `knx-core` + `rusqlite` (+ `tempfile` dev-dep) — no new dependency this plan.
- Every SQL table is `STRICT` (matches `opaque_entry`/`manufacturer_ref`/`schema_meta`).
- `PRAGMA foreign_keys = ON` must be set on every connection before any entity-table write (new — `migration.rs` does not set it today).
- Every multi-statement write goes through one transaction (`conn.unchecked_transaction()`, matching `opaque.rs`'s existing pattern) — a partial write must never be observable.
- No new `Stored*` mirror types — every function takes/returns real `knx_core` types directly (`knx-store` already depends on `knx-core`).
- `CURRENT_SCHEMA_VERSION` (both `knx-store::migration` and `knx-core::project`) moves from 3 to 4 in lockstep, per ADR-0003.
- Frozen fixture `crates/knx-store/fixtures/v4-empty.sqlite`, generated the same way as `v1`/`v2`/`v3` (`cargo run -p xtask -- freeze-fixture`, or by copying the pattern in that xtask task — check `xtask/src/` for the exact invocation before Task 2's fixture step).
- Column-name-to-Rust-field mapping for every table is fixed by the spec's DDL block — do not rename columns while implementing; if a name turns out to be wrong, fix the spec first.

---

## Before Task 1: confirm the fixture-freezing command

- [ ] **Step 1: Find how existing fixtures were generated**

```bash
grep -rn "freeze-fixture\|v3-empty\|v2-empty" xtask/src/
```

Read whatever `xtask` subcommand or test helper produced `crates/knx-store/fixtures/v3-empty.sqlite`, so Task 2's fixture step uses the same mechanism rather than inventing a new one.

---

### Task 1: `knx-core` — `PartialEq` derives and `StringTable::iter`

**Files:**
- Modify: `crates/knx-core/src/project.rs` (`Project` struct, `IdAllocators` struct)
- Modify: `crates/knx-core/src/devices.rs` (`Devices` struct)
- Modify: `crates/knx-core/src/string_table.rs` (`StringTable` struct — add `iter`)
- Test: inline `#[cfg(test)]` in `crates/knx-core/src/string_table.rs`

**Interfaces:**
- Produces: `Project: PartialEq`, `Devices: PartialEq`, `StringTable: PartialEq`, `IdAllocators: PartialEq` (all via `#[derive]`, no behavior change). `StringTable::iter(&self) -> impl Iterator<Item = (&TranslationKey, &Language, &str)>`.

- [ ] **Step 1: Write the failing test for `StringTable::iter`**

Add to `crates/knx-core/src/string_table.rs`'s existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn iter_yields_every_entry_regardless_of_order() {
    let mut t = StringTable::new(Language("en".into()));
    t.insert(
        TranslationKey("k1".into()),
        Language("de".into()),
        "Licht".into(),
    );
    t.insert(
        TranslationKey("k2".into()),
        Language("en".into()),
        "Heat".into(),
    );
    let mut seen: Vec<(String, String, String)> = t
        .iter()
        .map(|(k, l, v)| (k.0.clone(), l.0.clone(), v.to_string()))
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec![
            ("k1".to_string(), "de".to_string(), "Licht".to_string()),
            ("k2".to_string(), "en".to_string(), "Heat".to_string()),
        ]
    );
}
```

- [ ] **Step 2: Run it, confirm it fails to compile**

Run: `cargo test -p knx-core string_table:: 2>&1 | tail -20`
Expected: FAIL — `no method named 'iter' found for struct 'StringTable'`

- [ ] **Step 3: Add `iter`, and the four `PartialEq` derives**

In `crates/knx-core/src/string_table.rs`, add to the `impl StringTable` block (the first one, next to `new`/`default_language`/`insert`/`resolve`):

```rust
    /// Every entry as `(key, language, value)`. Iteration order is
    /// unspecified (backed by a `HashMap`) — persistence writes all of it
    /// regardless of order, since string-table entries are looked up by
    /// key, never enumerated positionally (DATA_MODEL §8).
    pub fn iter(&self) -> impl Iterator<Item = (&TranslationKey, &Language, &str)> {
        self.entries.iter().map(|((k, l), v)| (k, l, v.as_str()))
    }
```

Change the `StringTable` struct's derive from `#[derive(Debug)]` to:

```rust
#[derive(Debug, PartialEq)]
pub struct StringTable {
```

In `crates/knx-core/src/devices.rs`, change `Devices`'s derive from `#[derive(Debug, Clone, Default)]` to:

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Devices {
```

In `crates/knx-core/src/project.rs`, change `IdAllocators`'s derive from `#[derive(Debug, Clone, Default)]` to:

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IdAllocators {
```

And give `Project` a derive it currently has none of:

```rust
#[derive(Debug, PartialEq)]
pub struct Project {
```

- [ ] **Step 4: Run tests, confirm everything compiles and passes**

Run: `cargo test -p knx-core 2>&1 | tail -30`
Expected: PASS, including the new `iter_yields_every_entry_regardless_of_order` test. (`PartialEq` derives require every field's type to already implement `PartialEq` — `DeviceInstance`, `ComObjectInstance`, `Installation` already do per their existing derives; `ProjectInfo` already does. If the compiler names a field type that does not, add `PartialEq` to that type's own derive too and note it here before moving on.)

- [ ] **Step 5: Commit**

```bash
git add crates/knx-core/src/project.rs crates/knx-core/src/devices.rs crates/knx-core/src/string_table.rs
git commit -m "feat(knx-core): PartialEq for Project/Devices/StringTable/IdAllocators, StringTable::iter"
```

---

### Task 2: `knx-store` — schema v4 migration, `StoreError`, frozen fixture

**Files:**
- Modify: `crates/knx-store/src/migration.rs` (add `migrate_v3_to_v4`, bump `CURRENT_SCHEMA_VERSION`)
- Modify: `crates/knx-store/src/lib.rs` (add `StoreError`, wire `PRAGMA foreign_keys = ON`)
- Create: `crates/knx-store/fixtures/v4-empty.sqlite` (frozen, generated not hand-written)
- Test: inline in `crates/knx-store/src/migration.rs`

**Interfaces:**
- Produces: `pub enum StoreError { Sqlite(rusqlite::Error), NotSaved }` in `knx_store` (re-exported from `lib.rs`), implementing `std::error::Error`, `Display`, `From<rusqlite::Error>`. `CURRENT_SCHEMA_VERSION: i64 = 4`. Every table from the spec's Schema v4 DDL block exists after migration.
- Consumes: nothing new (extends the existing `migrations()` chain).

- [ ] **Step 1: Write the failing tests**

Add to `crates/knx-store/src/migration.rs`'s `#[cfg(test)] mod tests`:

```rust
#[test]
fn a_fresh_file_migrates_to_version_four_and_has_every_entity_table() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    let v: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 4);
    for table in [
        "project_info",
        "id_allocators",
        "string_table_entry",
        "installation",
        "area",
        "line",
        "building_part",
        "building_part_device",
        "device",
        "binary_data_ref",
        "com_object_instance",
        "com_object_override",
        "group_link",
        "group_range",
        "group_address",
        "parameter_instance",
    ] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "table {table} should exist and be empty");
    }
}

#[test]
fn foreign_keys_are_enforced() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    let fk_on: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fk_on, 1);
}

#[test]
fn the_frozen_v3_fixture_migrates_forward_to_v4() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v3.sqlite");
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v3-empty.sqlite"),
        &path,
    )
    .unwrap();
    let conn = open_and_migrate(&path).unwrap();
    let v: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 4);
    assert_eq!(crate::opaque::load_opaque(&conn).unwrap(), vec![]);
}

#[test]
fn the_frozen_v4_fixture_still_opens() {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v4-empty.sqlite");
    let conn = open_and_migrate(Path::new(fixture)).unwrap();
    let v: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 4);
}
```

- [ ] **Step 2: Run, confirm failure**

Run: `cargo test -p knx-store migration:: 2>&1 | tail -30`
Expected: FAIL — tables don't exist yet, `v4-empty.sqlite` doesn't exist yet, `CURRENT_SCHEMA_VERSION` is still 3.

- [ ] **Step 3: Add `PRAGMA foreign_keys = ON`, bump the version, add the migration**

In `crates/knx-store/src/migration.rs`, change:

```rust
pub const CURRENT_SCHEMA_VERSION: i64 = 4;
```

Add, right after `migrate_v2_to_v3`:

```rust
/// v3 -> v4: every entity table for `knx_core::Project` — the full domain
/// model, not just the opaque/manifest passthrough (ADR-0003; design doc
/// `docs/superpowers/specs/2026-09-03-knx-entity-persistence-design.md`).
/// The `Override<T>` chain is one row per (com object, attribute) in
/// `com_object_override`, not wide columns. `position`/`flat_position`
/// columns preserve every order-sensitive `Vec` the domain model has — see
/// the design doc's "owned-list vs flat-list order" note for which table
/// gets which.
fn migrate_v3_to_v4(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch(
        "CREATE TABLE project_info (
             id                  INTEGER PRIMARY KEY CHECK (id = 0),
             project_id          TEXT NOT NULL,
             name                TEXT NOT NULL,
             project_number      TEXT,
             group_address_style TEXT NOT NULL,
             completion          TEXT NOT NULL,
             last_modified       TEXT,
             project_start       TEXT,
             default_language    TEXT NOT NULL
         ) STRICT;

         CREATE TABLE id_allocators (
             id                  INTEGER PRIMARY KEY CHECK (id = 0),
             device              INTEGER NOT NULL,
             area                INTEGER NOT NULL,
             line                INTEGER NOT NULL,
             com_object_instance INTEGER NOT NULL,
             group_range         INTEGER NOT NULL,
             group_address       INTEGER NOT NULL,
             building_part       INTEGER NOT NULL,
             parameter_instance  INTEGER NOT NULL
         ) STRICT;

         CREATE TABLE string_table_entry (
             key      TEXT NOT NULL,
             language TEXT NOT NULL,
             value    TEXT NOT NULL,
             PRIMARY KEY (key, language)
         ) STRICT;

         CREATE TABLE installation (
             id                INTEGER PRIMARY KEY,
             name              TEXT NOT NULL,
             default_line_id   INTEGER,
             multicast_address TEXT,
             completion        TEXT NOT NULL
         ) STRICT;

         CREATE TABLE area (
             id              INTEGER PRIMARY KEY,
             installation_id INTEGER NOT NULL REFERENCES installation(id),
             position        INTEGER NOT NULL,
             source_path     TEXT NOT NULL,
             source_ets_id   TEXT NOT NULL,
             name            TEXT NOT NULL,
             address         INTEGER NOT NULL,
             completion      TEXT NOT NULL
         ) STRICT;
         CREATE INDEX area_installation_id ON area (installation_id);

         CREATE TABLE line (
             id                           INTEGER PRIMARY KEY,
             area_id                      INTEGER NOT NULL REFERENCES area(id),
             position                     INTEGER NOT NULL,
             source_path                  TEXT NOT NULL,
             source_ets_id                TEXT NOT NULL,
             name                         TEXT NOT NULL,
             address                      INTEGER NOT NULL,
             medium_ref                   TEXT NOT NULL,
             domain_address               TEXT,
             domain_address_is_checked    INTEGER,
             ip_routing_multicast_address TEXT,
             multicast_ttl                INTEGER,
             completion                   TEXT NOT NULL
         ) STRICT;
         CREATE INDEX line_area_id ON line (area_id);

         CREATE TABLE building_part (
             id              INTEGER PRIMARY KEY,
             installation_id INTEGER NOT NULL REFERENCES installation(id),
             parent_id       INTEGER REFERENCES building_part(id),
             position        INTEGER NOT NULL,
             flat_position   INTEGER NOT NULL,
             source_path     TEXT NOT NULL,
             source_ets_id   TEXT NOT NULL,
             name            TEXT NOT NULL,
             number          TEXT,
             kind            TEXT NOT NULL,
             default_line_id INTEGER REFERENCES line(id),
             completion      TEXT NOT NULL
         ) STRICT;
         CREATE INDEX building_part_installation_id ON building_part (installation_id);
         CREATE INDEX building_part_parent_id ON building_part (parent_id);

         CREATE TABLE device (
             id                          INTEGER PRIMARY KEY,
             installation_id             INTEGER NOT NULL REFERENCES installation(id),
             line_id                     INTEGER REFERENCES line(id),
             topology_position           INTEGER NOT NULL,
             source_path                 TEXT NOT NULL,
             source_ets_id               TEXT NOT NULL,
             name                        TEXT NOT NULL,
             description                 TEXT,
             address                     INTEGER,
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
         CREATE INDEX device_installation_id ON device (installation_id);
         CREATE INDEX device_line_id ON device (line_id);

         CREATE TABLE binary_data_ref (
             device_id INTEGER NOT NULL REFERENCES device(id),
             position  INTEGER NOT NULL,
             blob_id   TEXT NOT NULL,
             name      TEXT NOT NULL,
             PRIMARY KEY (device_id, position)
         ) STRICT;

         CREATE TABLE building_part_device (
             building_part_id INTEGER NOT NULL REFERENCES building_part(id),
             device_id        INTEGER NOT NULL REFERENCES device(id),
             position         INTEGER NOT NULL,
             PRIMARY KEY (building_part_id, device_id)
         ) STRICT;
         CREATE INDEX building_part_device_device_id ON building_part_device (device_id);

         CREATE TABLE com_object_instance (
             id            INTEGER PRIMARY KEY,
             device_id     INTEGER NOT NULL REFERENCES device(id),
             position      INTEGER NOT NULL,
             source_path   TEXT NOT NULL,
             source_ets_id TEXT NOT NULL,
             number        INTEGER NOT NULL,
             size_kind     TEXT,
             size_value    INTEGER,
             size_layer    TEXT,
             is_active     INTEGER NOT NULL
         ) STRICT;
         CREATE INDEX com_object_instance_device_id ON com_object_instance (device_id);

         CREATE TABLE com_object_override (
             com_object_instance_id INTEGER NOT NULL REFERENCES com_object_instance(id),
             attr                    TEXT NOT NULL,
             state                   TEXT NOT NULL,
             value                   TEXT,
             text_kind               TEXT,
             layer                   TEXT,
             PRIMARY KEY (com_object_instance_id, attr)
         ) STRICT;

         CREATE TABLE group_range (
             id              INTEGER PRIMARY KEY,
             installation_id INTEGER NOT NULL REFERENCES installation(id),
             parent_id       INTEGER REFERENCES group_range(id),
             position        INTEGER NOT NULL,
             flat_position   INTEGER NOT NULL,
             source_path     TEXT NOT NULL,
             source_ets_id   TEXT NOT NULL,
             name            TEXT NOT NULL,
             range_start     INTEGER NOT NULL,
             range_end       INTEGER NOT NULL
         ) STRICT;
         CREATE INDEX group_range_installation_id ON group_range (installation_id);
         CREATE INDEX group_range_parent_id ON group_range (parent_id);

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
         CREATE INDEX group_address_installation_id ON group_address (installation_id);
         CREATE INDEX group_address_range_id ON group_address (range_id);

         CREATE TABLE group_link (
             com_object_instance_id INTEGER NOT NULL REFERENCES com_object_instance(id),
             group_address_id       INTEGER NOT NULL REFERENCES group_address(id),
             direction              TEXT NOT NULL,
             position               INTEGER NOT NULL,
             PRIMARY KEY (com_object_instance_id, position)
         ) STRICT;
         CREATE INDEX group_link_group_address_id ON group_link (group_address_id);

         CREATE TABLE parameter_instance (
             id            INTEGER PRIMARY KEY,
             device_id     INTEGER NOT NULL REFERENCES device(id),
             position      INTEGER NOT NULL,
             source_path   TEXT NOT NULL,
             source_ets_id TEXT NOT NULL,
             raw           TEXT NOT NULL
         ) STRICT;
         CREATE INDEX parameter_instance_device_id ON parameter_instance (device_id);",
    )?;
    Ok(())
}
```

Update `migrations()`:

```rust
fn migrations() -> Vec<Migration> {
    vec![
        migrate_v0_to_v1,
        migrate_v1_to_v2,
        migrate_v2_to_v3,
        migrate_v3_to_v4,
    ]
}
```

`installation.default_line_id` and `building_part.default_line_id`/`.parent_id` reference `line`/`building_part` but are declared without inline `REFERENCES` above only where the referenced table is defined later in the same `execute_batch` — SQLite resolves `REFERENCES` lazily (no forward-declare problem), so this is fine as written; only `installation.default_line_id` genuinely has no `REFERENCES` clause because `line` rows can be created before their owning installation's default line is known during a future incremental-update path — leave it unconstrained (a plain `INTEGER`, no FK) as the one deliberate exception, and say so with a one-line SQL comment above that column when writing the migration.

Now `PRAGMA foreign_keys = ON`. `rusqlite::Connection::open`/`open_in_memory` do not enable it by default. Change `crates/knx-store/src/migration.rs`'s `migrate` function's two callers — actually simplest: set it once in `open_and_migrate` and `open_and_migrate_in_memory`, right after opening, before `migrate`:

```rust
pub fn open_and_migrate(path: &Path) -> Result<Connection, MigrationError> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}
```

```rust
pub fn open_and_migrate_in_memory() -> Result<Connection, MigrationError> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}
```

- [ ] **Step 4: Add `StoreError` to `lib.rs`**

In `crates/knx-store/src/lib.rs`:

```rust
use std::fmt;

/// Errors from the entity-persistence layer (`project`, `strings`,
/// `topology`, `building`, `devices`, `group`, `parameter`,
/// `command_sync`) — distinct from `MigrationError`, which is only about
/// getting the schema to `CURRENT_SCHEMA_VERSION`.
#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    /// `load_project` was called against a database with no `project_info`
    /// row — it was migrated but never saved. Distinct from an empty
    /// project (`Project::new`), which is a valid in-memory value that has
    /// simply not been persisted yet.
    NotSaved,
}

impl std::error::Error for StoreError {}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Sqlite(e) => write!(f, "{e}"),
            StoreError::NotSaved => write!(f, "no project has been saved to this database yet"),
        }
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Sqlite(e)
    }
}
```

Keep it in `lib.rs` itself (not its own file) — it is a small shared type every later module's functions return, not an entity module of its own.

- [ ] **Step 5: Generate the frozen `v4-empty.sqlite` fixture**

Using whatever mechanism "Before Task 1" found (likely `cargo run -p xtask -- freeze-fixture` with an argument naming the version, or a small throwaway Rust snippet calling `open_and_migrate` against `crates/knx-store/fixtures/v4-empty.sqlite`). Confirm the file is created and is not the same bytes as `v3-empty.sqlite` (`diff` should show a difference — the new tables).

- [ ] **Step 6: Run all tests, confirm green**

Run: `cargo test -p knx-store 2>&1 | tail -40`
Expected: PASS — including the four new tests from Step 1, and every pre-existing migration test (they must still pass unmodified; `the_frozen_v2_fixture_migrates_forward_to_v3` etc. are untouched by this task).

- [ ] **Step 7: Commit**

```bash
git add crates/knx-store/src/migration.rs crates/knx-store/src/lib.rs crates/knx-store/fixtures/v4-empty.sqlite
git commit -m "feat(knx-store): schema v4 — entity tables for knx_core::Project, StoreError"
```

---

### Task 3: `strings.rs` — `StringTable` persistence

**Files:**
- Create: `crates/knx-store/src/strings.rs`
- Modify: `crates/knx-store/src/lib.rs` (`pub mod strings;`)

**Interfaces:**
- Consumes: `knx_core::string_table::{StringTable, Language, TranslationKey}`; `StoreError` (Task 2).
- Produces: `pub fn upsert_string_table(conn: &Connection, table: &StringTable) -> Result<(), StoreError>`, `pub fn load_string_table(conn: &Connection, default_language: Language) -> Result<StringTable, StoreError>`.

- [ ] **Step 1: Write the failing test**

Create `crates/knx-store/src/strings.rs`:

```rust
//! Persistence for `knx_core::StringTable` — the `string_table_entry`
//! table, one row per `(key, language)` pair (DATA_MODEL §8).

use rusqlite::{params, Connection};

use knx_core::string_table::{Language, StringTable, TranslationKey};

use crate::StoreError;

pub fn upsert_string_table(conn: &Connection, table: &StringTable) -> Result<(), StoreError> {
    conn.execute("DELETE FROM string_table_entry", [])?;
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO string_table_entry (key, language, value) VALUES (?1, ?2, ?3)",
        )?;
        for (key, language, value) in table.iter() {
            stmt.execute(params![key.0, language.0, value])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn load_string_table(
    conn: &Connection,
    default_language: Language,
) -> Result<StringTable, StoreError> {
    let mut table = StringTable::new(default_language);
    let mut stmt = conn.prepare("SELECT key, language, value FROM string_table_entry")?;
    let rows = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let language: String = row.get(1)?;
        let value: String = row.get(2)?;
        Ok((key, language, value))
    })?;
    for row in rows {
        let (key, language, value) = row?;
        table.insert(TranslationKey(key), Language(language), value);
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;

    #[test]
    fn a_string_table_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut table = StringTable::new(Language("en".into()));
        table.insert(
            TranslationKey("k1".into()),
            Language("de".into()),
            "Licht".into(),
        );
        table.insert(
            TranslationKey("k1".into()),
            Language("en".into()),
            "Light".into(),
        );
        upsert_string_table(&conn, &table).unwrap();
        let loaded = load_string_table(&conn, Language("en".into())).unwrap();
        assert_eq!(loaded, table);
    }

    #[test]
    fn an_empty_string_table_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let table = StringTable::new(Language("de-DE".into()));
        upsert_string_table(&conn, &table).unwrap();
        let loaded = load_string_table(&conn, Language("de-DE".into())).unwrap();
        assert_eq!(loaded, table);
    }

    #[test]
    fn upsert_replaces_rather_than_accumulates() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut table = StringTable::new(Language("en".into()));
        table.insert(TranslationKey("k1".into()), Language("en".into()), "A".into());
        upsert_string_table(&conn, &table).unwrap();
        table.insert(TranslationKey("k2".into()), Language("en".into()), "B".into());
        upsert_string_table(&conn, &table).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM string_table_entry", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }
}
```

Add `pub mod strings;` to `crates/knx-store/src/lib.rs`.

- [ ] **Step 2: Run, confirm compiles but fails (or passes trivially)**

Run: `cargo test -p knx-store strings:: 2>&1 | tail -30`
Expected: PASS immediately if the implementation above is correct as-written — this task's Step 1 already contains the real implementation, since `StringTable::insert`/`iter` (Task 1) make it a thin wrapper with little room for a meaningfully-failing intermediate state. Confirm all three tests pass; if any fails, fix the SQL/logic above and rerun before proceeding — do not skip ahead with a red test.

- [ ] **Step 3: Commit**

```bash
git add crates/knx-store/src/strings.rs crates/knx-store/src/lib.rs
git commit -m "feat(knx-store): StringTable persistence"
```

---

### Task 4: `topology.rs` — installation, area, line

**Files:**
- Create: `crates/knx-store/src/topology.rs`
- Modify: `crates/knx-store/src/lib.rs` (`pub mod topology;`)

**Interfaces:**
- Consumes: `knx_core::installation::Installation`, `knx_core::topology::{Area, Line}`, `knx_core::ids::{InstallationId, AreaId, LineId, SourceRef}`, `knx_core::commissioning::CompletionStatus`; `StoreError`.
- Produces:
  - `pub fn upsert_installation_row(conn: &Connection, installation: &Installation) -> Result<(), StoreError>` — writes only the `installation` table row (id, name, default_line_id, multicast_address, completion). Does **not** touch topology/buildings/group ranges/addresses/parameters — those are Tasks 6, 7, 8, 9, 10's job, called separately by Task 11's `save_project`.
  - `pub fn upsert_area(conn: &Connection, installation_id: InstallationId, position: i64, area: &Area) -> Result<(), StoreError>`
  - `pub fn upsert_line(conn: &Connection, area_id: AreaId, position: i64, line: &Line) -> Result<(), StoreError>`
  - `pub fn load_installation_rows(conn: &Connection) -> Result<Vec<InstallationRow>, StoreError>` where `pub struct InstallationRow { pub id: InstallationId, pub name: String, pub default_line: Option<LineId>, pub multicast_address: Option<std::net::Ipv4Addr>, pub completion: CompletionStatus }` (a plain data carrier — Task 11 assembles the full `Installation` from this plus Tasks 6/7/8/9/10's loaders).
  - `pub fn load_topology(conn: &Connection, installation_id: InstallationId) -> Result<knx_core::topology::Topology, StoreError>` — loads `area`/`line` rows for this installation ordered by `position`, and the `unassigned` device id list ordered by `topology_position` (reads the `device` table directly by `installation_id`/`line_id IS NULL` — Task 5 owns writing that table, but any module may read a table it does not own, matching how `migration.rs`'s tests already call into `opaque`/`manifest`).
- Also exports two small conversion helpers other tasks reuse: `pub fn completion_to_str(c: CompletionStatus) -> &'static str` and `pub fn completion_from_str(s: &str) -> CompletionStatus` (used by every table with a `completion` column — Tasks 4, 5, 6).

- [ ] **Step 1: Write the failing test**

Create `crates/knx-store/src/topology.rs`:

```rust
//! Persistence for `Installation`'s scalar fields plus its `Topology`
//! (`Area`/`Line`, DATA_MODEL §5). Buildings, group ranges/addresses and
//! parameters — the rest of what `Installation` owns — are other modules'
//! tables, assembled together only in `project::save_project`/
//! `load_project`.

use std::net::Ipv4Addr;

use rusqlite::{params, Connection, OptionalExtension};

use knx_core::commissioning::CompletionStatus;
use knx_core::ids::{AreaId, DeviceId, InstallationId, LineId, SourceRef};
use knx_core::installation::Installation;
use knx_core::topology::{Area, Line, Topology};

use crate::StoreError;

pub fn completion_to_str(c: CompletionStatus) -> &'static str {
    match c {
        CompletionStatus::Undefined => "Undefined",
        CompletionStatus::Editing => "Editing",
        CompletionStatus::FinishedDesign => "FinishedDesign",
        CompletionStatus::Accepted => "Accepted",
    }
}

pub fn completion_from_str(s: &str) -> CompletionStatus {
    match s {
        "Editing" => CompletionStatus::Editing,
        "FinishedDesign" => CompletionStatus::FinishedDesign,
        "Accepted" => CompletionStatus::Accepted,
        _ => CompletionStatus::Undefined,
    }
}

pub fn upsert_installation_row(
    conn: &Connection,
    installation: &Installation,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO installation (id, name, default_line_id, multicast_address, completion)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             default_line_id = excluded.default_line_id,
             multicast_address = excluded.multicast_address,
             completion = excluded.completion",
        params![
            installation.id.0,
            installation.name,
            installation.default_line.map(|l| l.0),
            installation.multicast_address.map(|a| a.to_string()),
            completion_to_str(installation.completion),
        ],
    )?;
    Ok(())
}

pub fn upsert_area(
    conn: &Connection,
    installation_id: InstallationId,
    position: i64,
    area: &Area,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO area
             (id, installation_id, position, source_path, source_ets_id, name, address, completion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET
             installation_id = excluded.installation_id,
             position = excluded.position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             address = excluded.address,
             completion = excluded.completion",
        params![
            area.id.0,
            installation_id.0,
            position,
            area.source.path,
            area.source.ets_id,
            area.name,
            area.address,
            completion_to_str(area.completion),
        ],
    )?;
    Ok(())
}

pub fn upsert_line(
    conn: &Connection,
    area_id: AreaId,
    position: i64,
    line: &Line,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO line
             (id, area_id, position, source_path, source_ets_id, name, address, medium_ref,
              domain_address, domain_address_is_checked, ip_routing_multicast_address,
              multicast_ttl, completion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(id) DO UPDATE SET
             area_id = excluded.area_id,
             position = excluded.position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             address = excluded.address,
             medium_ref = excluded.medium_ref,
             domain_address = excluded.domain_address,
             domain_address_is_checked = excluded.domain_address_is_checked,
             ip_routing_multicast_address = excluded.ip_routing_multicast_address,
             multicast_ttl = excluded.multicast_ttl,
             completion = excluded.completion",
        params![
            line.id.0,
            area_id.0,
            position,
            line.source.path,
            line.source.ets_id,
            line.name,
            line.address,
            line.medium_ref,
            line.domain_address,
            line.domain_address_is_checked,
            line.ip_routing_multicast_address.map(|a| a.to_string()),
            line.multicast_ttl,
            completion_to_str(line.completion),
        ],
    )?;
    Ok(())
}

pub struct InstallationRow {
    pub id: InstallationId,
    pub name: String,
    pub default_line: Option<LineId>,
    pub multicast_address: Option<Ipv4Addr>,
    pub completion: CompletionStatus,
}

pub fn load_installation_rows(conn: &Connection) -> Result<Vec<InstallationRow>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, default_line_id, multicast_address, completion
         FROM installation ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        let id: u8 = row.get(0)?;
        let name: String = row.get(1)?;
        let default_line: Option<u32> = row.get(2)?;
        let multicast_address: Option<String> = row.get(3)?;
        let completion: String = row.get(4)?;
        Ok((id, name, default_line, multicast_address, completion))
    })?;
    rows.map(|r| {
        let (id, name, default_line, multicast_address, completion) = r?;
        Ok(InstallationRow {
            id: InstallationId(id),
            name,
            default_line: default_line.map(LineId),
            multicast_address: multicast_address
                .map(|a| a.parse().expect("stored multicast address is always valid")),
            completion: completion_from_str(&completion),
        })
    })
    .collect()
}

pub fn load_topology(
    conn: &Connection,
    installation_id: InstallationId,
) -> Result<Topology, StoreError> {
    let mut area_stmt = conn.prepare(
        "SELECT id, source_path, source_ets_id, name, address, completion
         FROM area WHERE installation_id = ?1 ORDER BY position",
    )?;
    let areas = area_stmt
        .query_map(params![installation_id.0], |row| {
            let id: u32 = row.get(0)?;
            let source_path: String = row.get(1)?;
            let source_ets_id: String = row.get(2)?;
            let name: String = row.get(3)?;
            let address: u8 = row.get(4)?;
            let completion: String = row.get(5)?;
            Ok((id, source_path, source_ets_id, name, address, completion))
        })?
        .map(|r| {
            let (id, source_path, source_ets_id, name, address, completion) = r?;
            let area_id = AreaId(id);
            let mut line_stmt = conn.prepare(
                "SELECT id FROM line WHERE area_id = ?1 ORDER BY position",
            )?;
            let lines = line_stmt
                .query_map(params![area_id.0], |row| Ok(LineId(row.get(0)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, StoreError>(Area {
                id: area_id,
                source: SourceRef {
                    path: source_path,
                    ets_id: source_ets_id,
                },
                name,
                address,
                completion: completion_from_str(&completion),
                lines,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut line_stmt = conn.prepare(
        "SELECT l.id, l.source_path, l.source_ets_id, l.name, l.address, l.medium_ref,
                l.domain_address, l.domain_address_is_checked, l.ip_routing_multicast_address,
                l.multicast_ttl, l.completion
         FROM line l JOIN area a ON l.area_id = a.id
         WHERE a.installation_id = ?1 ORDER BY a.position, l.position",
    )?;
    let lines = line_stmt
        .query_map(params![installation_id.0], |row| {
            let id: u32 = row.get(0)?;
            let source_path: String = row.get(1)?;
            let source_ets_id: String = row.get(2)?;
            let name: String = row.get(3)?;
            let address: u8 = row.get(4)?;
            let medium_ref: String = row.get(5)?;
            let domain_address: Option<String> = row.get(6)?;
            let domain_address_is_checked: Option<bool> = row.get(7)?;
            let ip_routing_multicast_address: Option<String> = row.get(8)?;
            let multicast_ttl: Option<u8> = row.get(9)?;
            let completion: String = row.get(10)?;
            Ok((
                id,
                source_path,
                source_ets_id,
                name,
                address,
                medium_ref,
                domain_address,
                domain_address_is_checked,
                ip_routing_multicast_address,
                multicast_ttl,
                completion,
            ))
        })?
        .map(|r| {
            let (
                id,
                source_path,
                source_ets_id,
                name,
                address,
                medium_ref,
                domain_address,
                domain_address_is_checked,
                ip_routing_multicast_address,
                multicast_ttl,
                completion,
            ) = r?;
            let line_id = LineId(id);
            let mut dev_stmt = conn.prepare(
                "SELECT id FROM device WHERE line_id = ?1 ORDER BY topology_position",
            )?;
            let devices = dev_stmt
                .query_map(params![line_id.0], |row| Ok(DeviceId(row.get(0)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, StoreError>(Line {
                id: line_id,
                source: SourceRef {
                    path: source_path,
                    ets_id: source_ets_id,
                },
                name,
                address,
                medium_ref,
                domain_address,
                domain_address_is_checked,
                ip_routing_multicast_address: ip_routing_multicast_address
                    .map(|a| a.parse().expect("stored address is always valid")),
                multicast_ttl,
                completion: completion_from_str(&completion),
                devices,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut unassigned_stmt = conn.prepare(
        "SELECT id FROM device WHERE installation_id = ?1 AND line_id IS NULL
         ORDER BY topology_position",
    )?;
    let unassigned = unassigned_stmt
        .query_map(params![installation_id.0], |row| Ok(DeviceId(row.get(0)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Topology {
        areas,
        lines,
        unassigned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use knx_core::commissioning::CompletionStatus;

    fn source() -> SourceRef {
        SourceRef {
            path: "0.xml".into(),
            ets_id: "t".into(),
        }
    }

    fn installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "Haus".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    #[test]
    fn installation_row_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let i = installation();
        upsert_installation_row(&conn, &i).unwrap();
        let rows = load_installation_rows(&conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, i.id);
        assert_eq!(rows[0].name, i.name);
        assert_eq!(rows[0].completion, i.completion);
    }

    #[test]
    fn area_and_line_round_trip_with_topology() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let line = Line {
            id: LineId(1),
            source: source(),
            name: "HL1".into(),
            address: 1,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::FinishedDesign,
            devices: vec![],
        };
        let area = Area {
            id: AreaId(1),
            source: source(),
            name: "A1".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![line.id],
        };
        upsert_area(&conn, InstallationId(0), 0, &area).unwrap();
        upsert_line(&conn, area.id, 0, &line).unwrap();

        let topo = load_topology(&conn, InstallationId(0)).unwrap();
        assert_eq!(topo.areas, vec![area]);
        assert_eq!(topo.lines, vec![line]);
        assert_eq!(topo.unassigned, vec![]);
    }
}
```

Add `pub mod topology;` to `crates/knx-store/src/lib.rs`.

- [ ] **Step 2: Run, confirm pass**

Run: `cargo test -p knx-store topology:: 2>&1 | tail -40`
Expected: PASS. `Line`'s `devices` field requires a `device` row to exist to test `load_topology`'s "assigned device" branch meaningfully with a non-empty list — that case is covered in Task 5's tests instead (once `device` rows can actually be written), not duplicated here; `area_and_line_round_trip_with_topology` above only exercises the empty-`devices`/empty-`unassigned` path, which is enough to prove `upsert_area`/`upsert_line`/`load_topology`'s area/line reconstruction itself.

- [ ] **Step 3: Commit**

```bash
git add crates/knx-store/src/topology.rs crates/knx-store/src/lib.rs
git commit -m "feat(knx-store): installation/area/line persistence"
```

---

### Task 5: `devices.rs` (part A) — device, binary_data_ref

**Files:**
- Create: `crates/knx-store/src/devices.rs`
- Modify: `crates/knx-store/src/lib.rs` (`pub mod devices;`)

**Interfaces:**
- Consumes: `knx_core::device::{DeviceInstance, BinaryDataRef}`, `knx_core::ids::{DeviceId, InstallationId, LineId, SourceRef}`, `knx_core::address::IndividualAddress`, `knx_core::commissioning::CommissioningState`; `topology::{completion_to_str, completion_from_str}` (Task 4); `StoreError`.
- Produces:
  - `pub fn upsert_device(conn: &Connection, installation_id: InstallationId, topology_position: i64, device: &DeviceInstance) -> Result<(), StoreError>` — writes the `device` row and replaces its `binary_data_ref` rows. Does not touch `com_object_instance` (Task 7) or `building_part_device` (Task 6).
  - `pub fn load_device(conn: &Connection, id: DeviceId) -> Result<DeviceInstance, StoreError>` — loads the `device` row and its `binary_data_ref` rows; `com_objects` is filled in by Task 11's orchestration (it belongs to `com_object_instance`, Task 7's table), so this function leaves `com_objects: vec![]` and documents that its caller must fill it in.
  - `pub fn load_all_device_ids(conn: &Connection) -> Result<Vec<DeviceId>, StoreError>` (ordered by `id`, matching `Devices`'s own `BTreeMap` order).

- [ ] **Step 1: Write the failing test**

Create `crates/knx-store/src/devices.rs`:

```rust
//! Persistence for `DeviceInstance` and `ComObjectInstance`
//! (DATA_MODEL §4) — the two entities `knx_core::devices::Devices` owns.
//! This file grows across three plan tasks: device + binary data here,
//! `com_object_instance` + the `Override<T>` codec next, `group_link`
//! last (it needs `group_address` rows to exist first).

use rusqlite::{params, Connection};

use knx_core::address::IndividualAddress;
use knx_core::commissioning::CommissioningState;
use knx_core::device::{BinaryDataRef, DeviceInstance};
use knx_core::ids::{DeviceId, InstallationId, SourceRef};

use crate::topology::{completion_from_str, completion_to_str};
use crate::StoreError;

pub fn upsert_device(
    conn: &Connection,
    installation_id: InstallationId,
    topology_position: i64,
    device: &DeviceInstance,
) -> Result<(), StoreError> {
    let c = &device.commissioning;
    conn.execute(
        "INSERT INTO device
             (id, installation_id, line_id, topology_position, source_path, source_ets_id,
              name, description, address, product_ref, program_ref, completion,
              individual_address_loaded, application_program_loaded, parameters_loaded,
              communication_part_loaded, medium_config_loaded, last_modified, last_download,
              broken, visibility_calculated)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                 ?17, ?18, ?19, ?20)
         ON CONFLICT(id) DO UPDATE SET
             installation_id = excluded.installation_id,
             line_id = excluded.line_id,
             topology_position = excluded.topology_position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             description = excluded.description,
             address = excluded.address,
             product_ref = excluded.product_ref,
             program_ref = excluded.program_ref,
             completion = excluded.completion,
             individual_address_loaded = excluded.individual_address_loaded,
             application_program_loaded = excluded.application_program_loaded,
             parameters_loaded = excluded.parameters_loaded,
             communication_part_loaded = excluded.communication_part_loaded,
             medium_config_loaded = excluded.medium_config_loaded,
             last_modified = excluded.last_modified,
             last_download = excluded.last_download,
             broken = excluded.broken,
             visibility_calculated = excluded.visibility_calculated",
        params![
            device.id.0,
            installation_id.0,
            topology_position,
            device.source.path,
            device.source.ets_id,
            device.name,
            device.description,
            device.address.map(|a| a.raw()),
            device.product_ref,
            device.program_ref,
            completion_to_str(c.completion),
            c.individual_address_loaded,
            c.application_program_loaded,
            c.parameters_loaded,
            c.communication_part_loaded,
            c.medium_config_loaded,
            c.last_modified.map(|d| d.to_rfc3339()),
            c.last_download.map(|d| d.to_rfc3339()),
            c.broken,
            device.visibility_calculated,
        ],
    )?;
    // NOTE: the INSERT above always sets line_id = NULL — placement into a
    // line is Task 4/11's job (topology::upsert_line owns the device's
    // *membership*, not this row's own line_id column directly, since a
    // device can be reassigned between lines independent of its own
    // fields). A later task revisits this: see Task 11's orchestration
    // note on device placement, which sets `device.line_id` with a
    // separate targeted UPDATE once the owning line is known. Until then,
    // `topology::load_topology`'s "unassigned" query would misclassify
    // every device as unassigned — Task 11 must not skip that step.
    conn.execute(
        "DELETE FROM binary_data_ref WHERE device_id = ?1",
        params![device.id.0],
    )?;
    let mut stmt = conn.prepare(
        "INSERT INTO binary_data_ref (device_id, position, blob_id, name) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (i, b) in device.binary_data.iter().enumerate() {
        stmt.execute(params![device.id.0, i as i64, b.id, b.name])?;
    }
    Ok(())
}

/// Sets a device's line placement directly — the one column `upsert_device`
/// deliberately leaves untouched. Called by Task 11's orchestration once
/// the owning `Line`'s id is known (or with `line_id = None` for an
/// unassigned device), and by Task 12's `sync_after_command` for
/// `SetIndividualAddress` (which does not move a device between lines, but
/// re-asserts the same placement is harmless and keeps that call site to
/// one function).
pub fn set_device_line(
    conn: &Connection,
    device_id: DeviceId,
    line_id: Option<knx_core::ids::LineId>,
    topology_position: i64,
) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE device SET line_id = ?1, topology_position = ?2 WHERE id = ?3",
        params![line_id.map(|l| l.0), topology_position, device_id.0],
    )?;
    Ok(())
}

pub fn load_device(conn: &Connection, id: DeviceId) -> Result<DeviceInstance, StoreError> {
    let (
        source_path,
        source_ets_id,
        name,
        description,
        address,
        product_ref,
        program_ref,
        completion,
        individual_address_loaded,
        application_program_loaded,
        parameters_loaded,
        communication_part_loaded,
        medium_config_loaded,
        last_modified,
        last_download,
        broken,
        visibility_calculated,
    ) = conn.query_row(
        "SELECT source_path, source_ets_id, name, description, address, product_ref,
                program_ref, completion, individual_address_loaded, application_program_loaded,
                parameters_loaded, communication_part_loaded, medium_config_loaded,
                last_modified, last_download, broken, visibility_calculated
         FROM device WHERE id = ?1",
        params![id.0],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<u16>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, bool>(8)?,
                row.get::<_, bool>(9)?,
                row.get::<_, bool>(10)?,
                row.get::<_, bool>(11)?,
                row.get::<_, bool>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, bool>(15)?,
                row.get::<_, bool>(16)?,
            ))
        },
    )?;

    let mut bd_stmt = conn.prepare(
        "SELECT blob_id, name FROM binary_data_ref WHERE device_id = ?1 ORDER BY position",
    )?;
    let binary_data = bd_stmt
        .query_map(params![id.0], |row| {
            Ok(BinaryDataRef {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(DeviceInstance {
        id,
        source: SourceRef {
            path: source_path,
            ets_id: source_ets_id,
        },
        name,
        description,
        address: address.map(IndividualAddress::from_raw),
        product_ref,
        program_ref,
        commissioning: CommissioningState {
            completion: completion_from_str(&completion),
            individual_address_loaded,
            application_program_loaded,
            parameters_loaded,
            communication_part_loaded,
            medium_config_loaded,
            last_modified: last_modified.map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .expect("stored timestamp is always valid RFC3339")
                    .with_timezone(&chrono::Utc)
            }),
            last_download: last_download.map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .expect("stored timestamp is always valid RFC3339")
                    .with_timezone(&chrono::Utc)
            }),
            broken,
        },
        visibility_calculated,
        com_objects: vec![],
        binary_data,
    })
}

pub fn load_all_device_ids(conn: &Connection) -> Result<Vec<DeviceId>, StoreError> {
    let mut stmt = conn.prepare("SELECT id FROM device ORDER BY id")?;
    let ids = stmt
        .query_map([], |row| Ok(DeviceId(row.get(0)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use crate::topology::upsert_installation_row;
    use knx_core::commissioning::CompletionStatus;
    use knx_core::installation::Installation;
    use knx_core::topology::Topology;

    fn installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "H".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    fn device() -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(1),
            source: SourceRef {
                path: "0.xml".into(),
                ets_id: "M-1".into(),
            },
            name: "Aktor".into(),
            description: Some("Schaltaktor".into()),
            address: Some(IndividualAddress::new(1, 1, 5).unwrap()),
            product_ref: "P-1".into(),
            program_ref: "H-1".into(),
            commissioning: CommissioningState {
                completion: CompletionStatus::FinishedDesign,
                individual_address_loaded: true,
                application_program_loaded: true,
                parameters_loaded: false,
                communication_part_loaded: true,
                medium_config_loaded: false,
                last_modified: None,
                last_download: None,
                broken: false,
            },
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![
                BinaryDataRef {
                    id: "guid-1".into(),
                    name: "cert.dat".into(),
                },
                BinaryDataRef {
                    id: "guid-2".into(),
                    name: "key.dat".into(),
                },
            ],
        }
    }

    #[test]
    fn a_device_with_binary_data_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let loaded = load_device(&conn, d.id).unwrap();
        assert_eq!(loaded.name, d.name);
        assert_eq!(loaded.binary_data, d.binary_data);
        assert_eq!(loaded.commissioning, d.commissioning);
        assert_eq!(loaded.address, d.address);
    }

    #[test]
    fn a_device_without_an_address_or_description_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let mut d = device();
        d.address = None;
        d.description = None;
        d.binary_data = vec![];
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let loaded = load_device(&conn, d.id).unwrap();
        assert_eq!(loaded.address, None);
        assert_eq!(loaded.description, None);
        assert_eq!(loaded.binary_data, vec![]);
    }

    #[test]
    fn set_device_line_updates_placement_without_touching_other_fields() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        // A line row must exist to satisfy the foreign key.
        conn.execute(
            "INSERT INTO area (id, installation_id, position, source_path, source_ets_id, name, address, completion)
             VALUES (1, 0, 0, 't', 't', 'A', 1, 'FinishedDesign')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO line (id, area_id, position, source_path, source_ets_id, name, address, medium_ref, completion)
             VALUES (1, 1, 0, 't', 't', 'L', 1, 'TP', 'FinishedDesign')",
            [],
        )
        .unwrap();
        set_device_line(&conn, d.id, Some(knx_core::ids::LineId(1)), 0).unwrap();
        let line_id: Option<i64> = conn
            .query_row("SELECT line_id FROM device WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(line_id, Some(1));
        let loaded = load_device(&conn, d.id).unwrap();
        assert_eq!(loaded.name, d.name); // untouched
    }
}
```

Add `pub mod devices;` to `crates/knx-store/src/lib.rs`.

- [ ] **Step 2: Run, confirm pass**

Run: `cargo test -p knx-store devices:: 2>&1 | tail -40`
Expected: PASS. If `chrono`'s `parse_from_rfc3339` signature differs from what's shown (check `crates/knx-core/Cargo.toml`'s `chrono` version and how `knx-etsproj` already parses/formats timestamps — reuse that exact pattern, e.g. `values.rs`, instead of introducing a second RFC3339 convention), align to the existing convention.

- [ ] **Step 3: Commit**

```bash
git add crates/knx-store/src/devices.rs crates/knx-store/src/lib.rs
git commit -m "feat(knx-store): device + binary data persistence"
```

---

### Task 6: `building.rs` — building_part, building_part_device

**Files:**
- Create: `crates/knx-store/src/building.rs`
- Modify: `crates/knx-store/src/lib.rs` (`pub mod building;`)

**Interfaces:**
- Consumes: `knx_core::building::{BuildingPart, BuildingPartType}`, `knx_core::ids::{BuildingPartId, DeviceId, InstallationId, LineId, SourceRef}`; `topology::{completion_to_str, completion_from_str}`; `StoreError`.
- Produces:
  - `pub fn upsert_building_part(conn: &Connection, installation_id: InstallationId, position: i64, flat_position: i64, part: &BuildingPart) -> Result<(), StoreError>` — writes the `building_part` row and replaces its `building_part_device` rows from `part.devices`. Does not resolve `children` (that is a query at load time, not stored data — `children` is derived, never written).
  - `pub fn load_buildings(conn: &Connection, installation_id: InstallationId) -> Result<Vec<BuildingPart>, StoreError>` — every building part for the installation, ordered by `flat_position`, with `children` reconstructed by querying `parent_id` ordered by `position`.

- [ ] **Step 1: Write the failing test**

Create `crates/knx-store/src/building.rs`:

```rust
//! Persistence for `BuildingPart` (DATA_MODEL §5) — a recursive hierarchy
//! stored flat (`flat_position` gives `Installation::buildings`'s order;
//! `parent_id` + `position` rebuilds each part's `children`).

use rusqlite::{params, Connection};

use knx_core::building::{BuildingPart, BuildingPartType};
use knx_core::ids::{BuildingPartId, DeviceId, InstallationId, LineId, SourceRef};

use crate::topology::{completion_from_str, completion_to_str};
use crate::StoreError;

fn kind_to_str(k: BuildingPartType) -> &'static str {
    match k {
        BuildingPartType::Building => "Building",
        BuildingPartType::Floor => "Floor",
        BuildingPartType::Room => "Room",
        BuildingPartType::Corridor => "Corridor",
        BuildingPartType::DistributionBoard => "DistributionBoard",
        BuildingPartType::BuildingPart => "BuildingPart",
    }
}

fn kind_from_str(s: &str) -> BuildingPartType {
    match s {
        "Floor" => BuildingPartType::Floor,
        "Room" => BuildingPartType::Room,
        "Corridor" => BuildingPartType::Corridor,
        "DistributionBoard" => BuildingPartType::DistributionBoard,
        "BuildingPart" => BuildingPartType::BuildingPart,
        _ => BuildingPartType::Building,
    }
}

pub fn upsert_building_part(
    conn: &Connection,
    installation_id: InstallationId,
    position: i64,
    flat_position: i64,
    part: &BuildingPart,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO building_part
             (id, installation_id, parent_id, position, flat_position, source_path,
              source_ets_id, name, number, kind, default_line_id, completion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
             installation_id = excluded.installation_id,
             parent_id = excluded.parent_id,
             position = excluded.position,
             flat_position = excluded.flat_position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             number = excluded.number,
             kind = excluded.kind,
             default_line_id = excluded.default_line_id,
             completion = excluded.completion",
        params![
            part.id.0,
            installation_id.0,
            part.parent.map(|p| p.0),
            position,
            flat_position,
            part.source.path,
            part.source.ets_id,
            part.name,
            part.number,
            kind_to_str(part.kind),
            part.default_line.map(|l| l.0),
            completion_to_str(part.completion),
        ],
    )?;
    conn.execute(
        "DELETE FROM building_part_device WHERE building_part_id = ?1",
        params![part.id.0],
    )?;
    let mut stmt = conn.prepare(
        "INSERT INTO building_part_device (building_part_id, device_id, position)
         VALUES (?1, ?2, ?3)",
    )?;
    for (i, device_id) in part.devices.iter().enumerate() {
        stmt.execute(params![part.id.0, device_id.0, i as i64])?;
    }
    Ok(())
}

pub fn load_buildings(
    conn: &Connection,
    installation_id: InstallationId,
) -> Result<Vec<BuildingPart>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, source_path, source_ets_id, name, number, kind,
                default_line_id, completion
         FROM building_part WHERE installation_id = ?1 ORDER BY flat_position",
    )?;
    let rows = stmt
        .query_map(params![installation_id.0], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, Option<u32>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<u32>>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    rows.into_iter()
        .map(
            |(id, parent_id, source_path, source_ets_id, name, number, kind, default_line_id, completion)| {
                let building_id = BuildingPartId(id);
                let mut children_stmt = conn.prepare(
                    "SELECT id FROM building_part WHERE parent_id = ?1 ORDER BY position",
                )?;
                let children = children_stmt
                    .query_map(params![building_id.0], |row| Ok(BuildingPartId(row.get(0)?)))?
                    .collect::<Result<Vec<_>, _>>()?;
                let mut dev_stmt = conn.prepare(
                    "SELECT device_id FROM building_part_device WHERE building_part_id = ?1
                     ORDER BY position",
                )?;
                let devices = dev_stmt
                    .query_map(params![building_id.0], |row| Ok(DeviceId(row.get(0)?)))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok::<_, StoreError>(BuildingPart {
                    id: building_id,
                    source: SourceRef {
                        path: source_path,
                        ets_id: source_ets_id,
                    },
                    name,
                    number,
                    kind: kind_from_str(&kind),
                    default_line: default_line_id.map(LineId),
                    completion: completion_from_str(&completion),
                    children,
                    devices,
                    parent: parent_id.map(BuildingPartId),
                })
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use crate::topology::upsert_installation_row;
    use knx_core::commissioning::CompletionStatus;
    use knx_core::installation::Installation;
    use knx_core::topology::Topology;

    fn installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "H".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology { areas: vec![], lines: vec![], unassigned: vec![] },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    fn part(id: u32, parent: Option<u32>, kind: BuildingPartType) -> BuildingPart {
        BuildingPart {
            id: BuildingPartId(id),
            source: SourceRef { path: "t".into(), ets_id: "t".into() },
            name: format!("Part {id}"),
            number: None,
            kind,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: vec![],
            devices: vec![],
            parent: parent.map(BuildingPartId),
        }
    }

    #[test]
    fn a_three_level_hierarchy_round_trips_with_correct_children_and_flat_order() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let building = part(1, None, BuildingPartType::Building);
        let floor = part(2, Some(1), BuildingPartType::Floor);
        let mut room = part(3, Some(2), BuildingPartType::Room);
        room.devices = vec![DeviceId(9), DeviceId(7)]; // deliberately non-sorted by id

        // Insert a device row to satisfy building_part_device's FK. Minimal
        // direct SQL, matching the migration test's own style — devices.rs's
        // upsert_device is not yet wired to installation membership here.
        for id in [7, 9] {
            conn.execute(
                &format!(
                    "INSERT INTO device (id, installation_id, line_id, topology_position,
                        source_path, source_ets_id, name, product_ref, program_ref, completion,
                        individual_address_loaded, application_program_loaded, parameters_loaded,
                        communication_part_loaded, medium_config_loaded, broken,
                        visibility_calculated)
                     VALUES ({id}, 0, NULL, 0, 't', 't', 'D', 'P', 'H', 'FinishedDesign',
                             0, 0, 0, 0, 0, 0, 1)"
                ),
                [],
            )
            .unwrap();
        }

        // flat_position: building=0, floor=1, room=2 (pre-order). position:
        // sibling order under each's own parent (each is an only child here, so 0).
        upsert_building_part(&conn, InstallationId(0), 0, 0, &building).unwrap();
        upsert_building_part(&conn, InstallationId(0), 0, 1, &floor).unwrap();
        upsert_building_part(&conn, InstallationId(0), 0, 2, &room).unwrap();

        let loaded = load_buildings(&conn, InstallationId(0)).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].id, BuildingPartId(1));
        assert_eq!(loaded[0].children, vec![BuildingPartId(2)]);
        assert_eq!(loaded[1].children, vec![BuildingPartId(3)]);
        assert_eq!(loaded[2].devices, vec![DeviceId(9), DeviceId(7)]);
        assert_eq!(loaded[2].parent, Some(BuildingPartId(2)));
    }
}
```

Add `pub mod building;` to `crates/knx-store/src/lib.rs`.

- [ ] **Step 2: Run, confirm pass**

Run: `cargo test -p knx-store building:: 2>&1 | tail -40`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/knx-store/src/building.rs crates/knx-store/src/lib.rs
git commit -m "feat(knx-store): building-part hierarchy persistence"
```

---

### Task 7: `devices.rs` (part B) — com_object_instance, the `Override<T>` codec

**Files:**
- Modify: `crates/knx-store/src/devices.rs` (append)

**Interfaces:**
- Consumes: `knx_core::device::ComObjectInstance`, `knx_core::provenance::{Layer, Override, Resolved}`, `knx_core::dpt::DptRef`, `knx_core::flags::{ResolvedFlags, ObjectSize}`, `knx_core::string_table::{Text, LocalizedString, TranslationKey}`, `knx_core::ids::ComObjectInstanceId`.
- Produces:
  - `pub enum Attr { Text, Description, Dpt, Read, Write, Transmit, Update, Communication }` with `as_str`/`from_str`-equivalent free functions `attr_to_str`/`attr_from_str` (used by Task 12's `command_sync` too, so it must be `pub`).
  - `pub fn upsert_com_object_instance(conn: &Connection, device_id: DeviceId, position: i64, com: &ComObjectInstance) -> Result<(), StoreError>` — writes the `com_object_instance` row and all 7 of its `com_object_override` rows. Does not touch `group_link` (Task 9).
  - `pub fn upsert_com_object_override_row(conn: &Connection, com_object_instance_id: ComObjectInstanceId, attr: Attr, ...)` — see below; the exact per-attribute upsert Task 12's `sync_after_command` calls directly for `SetComObjectDpt`/`RestoreComObjectDpt` without rewriting the whole `com_object_instance` row.
  - `pub fn load_com_object_instance(conn: &Connection, id: ComObjectInstanceId) -> Result<ComObjectInstance, StoreError>` — `links` is left `vec![]` (Task 9's job, filled in by Task 11's orchestration).
  - `pub fn load_com_object_ids_for_device(conn: &Connection, device_id: DeviceId) -> Result<Vec<ComObjectInstanceId>, StoreError>` ordered by `position` — this is what `DeviceInstance::com_objects` is filled from.

- [ ] **Step 1: Write the failing test**

Append to `crates/knx-store/src/devices.rs` (imports first — add these to the existing `use` block at the top of the file):

```rust
use knx_core::device::ComObjectInstance;
use knx_core::dpt::DptRef;
use knx_core::flags::{ObjectSize, ResolvedFlags};
use knx_core::ids::ComObjectInstanceId;
use knx_core::provenance::{Layer, Override, Resolved};
use knx_core::string_table::{LocalizedString, Text, TranslationKey};
```

Then append this new code (after `load_all_device_ids` and before the existing `#[cfg(test)] mod tests`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attr {
    Text,
    Description,
    Dpt,
    Read,
    Write,
    Transmit,
    Update,
    Communication,
}

pub fn attr_to_str(a: Attr) -> &'static str {
    match a {
        Attr::Text => "text",
        Attr::Description => "description",
        Attr::Dpt => "dpt",
        Attr::Read => "read",
        Attr::Write => "write",
        Attr::Transmit => "transmit",
        Attr::Update => "update",
        Attr::Communication => "communication",
    }
}

fn layer_to_str(l: Layer) -> &'static str {
    match l {
        Layer::Program => "Program",
        Layer::ProgramRef => "ProgramRef",
        Layer::Instance => "Instance",
        Layer::Inferred => "Inferred",
        Layer::UserEdit => "UserEdit",
    }
}

fn layer_from_str(s: &str) -> Layer {
    match s {
        "ProgramRef" => Layer::ProgramRef,
        "Instance" => Layer::Instance,
        "Inferred" => Layer::Inferred,
        "UserEdit" => Layer::UserEdit,
        _ => Layer::Program,
    }
}

/// One row of `com_object_override`, already string-encoded. Building this
/// is what turns each `Override<T>` field into the four columns the table
/// stores — the encoding side of the codec.
struct OverrideRow {
    state: &'static str,
    value: Option<String>,
    text_kind: Option<&'static str>,
    layer: Option<&'static str>,
}

fn encode_text(t: &Override<Text>) -> OverrideRow {
    match t {
        Override::Absent => OverrideRow { state: "absent", value: None, text_kind: None, layer: None },
        Override::Empty => OverrideRow { state: "empty", value: None, text_kind: None, layer: None },
        Override::Malformed(raw) => OverrideRow {
            state: "malformed",
            value: Some(raw.clone()),
            text_kind: None,
            layer: None,
        },
        Override::Value(Resolved { value, layer }) => {
            let (kind, text) = match value {
                Text::Literal(s) => ("literal", s.clone()),
                Text::Localized(LocalizedString(TranslationKey(k))) => ("localized", k.clone()),
            };
            OverrideRow {
                state: "value",
                value: Some(text),
                text_kind: Some(kind),
                layer: Some(layer_to_str(*layer)),
            }
        }
    }
}

fn decode_text(state: &str, value: Option<String>, text_kind: Option<String>, layer: Option<String>) -> Override<Text> {
    match state {
        "empty" => Override::Empty,
        "malformed" => Override::Malformed(value.expect("malformed state always carries a value")),
        "value" => {
            let text = match text_kind.as_deref() {
                Some("localized") => Text::Localized(LocalizedString(TranslationKey(
                    value.expect("value state always carries a value"),
                ))),
                _ => Text::Literal(value.expect("value state always carries a value")),
            };
            Override::Value(Resolved {
                value: text,
                layer: layer_from_str(&layer.expect("value state always carries a layer")),
            })
        }
        _ => Override::Absent,
    }
}

fn encode_dpt(d: &Override<DptRef>) -> OverrideRow {
    match d {
        Override::Absent => OverrideRow { state: "absent", value: None, text_kind: None, layer: None },
        Override::Empty => OverrideRow { state: "empty", value: None, text_kind: None, layer: None },
        Override::Malformed(raw) => OverrideRow {
            state: "malformed",
            value: Some(raw.clone()),
            text_kind: None,
            layer: None,
        },
        Override::Value(Resolved { value, layer }) => OverrideRow {
            state: "value",
            value: Some(value.to_string()),
            text_kind: None,
            layer: Some(layer_to_str(*layer)),
        },
    }
}

fn decode_dpt(state: &str, value: Option<String>, layer: Option<String>) -> Override<DptRef> {
    match state {
        "empty" => Override::Empty,
        "malformed" => Override::Malformed(value.expect("malformed state always carries a value")),
        "value" => Override::Value(Resolved {
            value: DptRef::parse(&value.expect("value state always carries a value"))
                .expect("stored DptRef text is always valid"),
            layer: layer_from_str(&layer.expect("value state always carries a layer")),
        }),
        _ => Override::Absent,
    }
}

fn encode_bool(b: &Override<bool>) -> OverrideRow {
    match b {
        Override::Absent => OverrideRow { state: "absent", value: None, text_kind: None, layer: None },
        Override::Empty => OverrideRow { state: "empty", value: None, text_kind: None, layer: None },
        Override::Malformed(raw) => OverrideRow {
            state: "malformed",
            value: Some(raw.clone()),
            text_kind: None,
            layer: None,
        },
        Override::Value(Resolved { value, layer }) => OverrideRow {
            state: "value",
            value: Some(if *value { "1".into() } else { "0".into() }),
            text_kind: None,
            layer: Some(layer_to_str(*layer)),
        },
    }
}

fn decode_bool(state: &str, value: Option<String>, layer: Option<String>) -> Override<bool> {
    match state {
        "empty" => Override::Empty,
        "malformed" => Override::Malformed(value.expect("malformed state always carries a value")),
        "value" => Override::Value(Resolved {
            value: value.expect("value state always carries a value") == "1",
            layer: layer_from_str(&layer.expect("value state always carries a layer")),
        }),
        _ => Override::Absent,
    }
}

fn write_override_row(
    conn: &Connection,
    com_object_instance_id: ComObjectInstanceId,
    attr: Attr,
    row: OverrideRow,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO com_object_override
             (com_object_instance_id, attr, state, value, text_kind, layer)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(com_object_instance_id, attr) DO UPDATE SET
             state = excluded.state,
             value = excluded.value,
             text_kind = excluded.text_kind,
             layer = excluded.layer",
        params![
            com_object_instance_id.0,
            attr_to_str(attr),
            row.state,
            row.value,
            row.text_kind,
            row.layer,
        ],
    )?;
    Ok(())
}

/// Writes exactly one `com_object_override` row for `com`'s `dpt` field —
/// the single-attribute upsert `command_sync::sync_after_command` (Task 12)
/// calls for `SetComObjectDpt`/`RestoreComObjectDpt`, instead of rewriting
/// the whole `com_object_instance` row.
pub fn upsert_com_object_dpt_override(
    conn: &Connection,
    com_object_instance_id: ComObjectInstanceId,
    dpt: &Override<DptRef>,
) -> Result<(), StoreError> {
    write_override_row(conn, com_object_instance_id, Attr::Dpt, encode_dpt(dpt))
}

fn size_columns(size: &Option<Resolved<ObjectSize>>) -> (Option<&'static str>, Option<i64>, Option<&'static str>) {
    match size {
        None => (None, None, None),
        Some(Resolved { value, layer }) => {
            let (kind, v) = match value {
                ObjectSize::Bit(n) => ("bit", *n as i64),
                ObjectSize::Byte(n) => ("byte", *n as i64),
            };
            (Some(kind), Some(v), Some(layer_to_str(*layer)))
        }
    }
}

fn decode_size(kind: Option<String>, value: Option<i64>, layer: Option<String>) -> Option<Resolved<ObjectSize>> {
    let kind = kind?;
    let value = value.expect("size_value present whenever size_kind is");
    let layer = layer.expect("size_layer present whenever size_kind is");
    let size = match kind.as_str() {
        "byte" => ObjectSize::Byte(value as u16),
        _ => ObjectSize::Bit(value as u8),
    };
    Some(Resolved {
        value: size,
        layer: layer_from_str(&layer),
    })
}

pub fn upsert_com_object_instance(
    conn: &Connection,
    device_id: DeviceId,
    position: i64,
    com: &ComObjectInstance,
) -> Result<(), StoreError> {
    let (size_kind, size_value, size_layer) = size_columns(&com.size);
    conn.execute(
        "INSERT INTO com_object_instance
             (id, device_id, position, source_path, source_ets_id, number, size_kind,
              size_value, size_layer, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
             device_id = excluded.device_id,
             position = excluded.position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             number = excluded.number,
             size_kind = excluded.size_kind,
             size_value = excluded.size_value,
             size_layer = excluded.size_layer,
             is_active = excluded.is_active",
        params![
            com.id.0,
            device_id.0,
            position,
            com.source.path,
            com.source.ets_id,
            com.number,
            size_kind,
            size_value,
            size_layer,
            com.is_active,
        ],
    )?;

    write_override_row(conn, com.id, Attr::Text, encode_text(&com.text))?;
    write_override_row(conn, com.id, Attr::Description, encode_text(&com.description))?;
    write_override_row(conn, com.id, Attr::Dpt, encode_dpt(&com.dpt))?;
    write_override_row(conn, com.id, Attr::Read, encode_bool(&com.flags.read))?;
    write_override_row(conn, com.id, Attr::Write, encode_bool(&com.flags.write))?;
    write_override_row(conn, com.id, Attr::Transmit, encode_bool(&com.flags.transmit))?;
    write_override_row(conn, com.id, Attr::Update, encode_bool(&com.flags.update))?;
    write_override_row(conn, com.id, Attr::Communication, encode_bool(&com.flags.communication))?;
    Ok(())
}

pub fn load_com_object_instance(
    conn: &Connection,
    id: ComObjectInstanceId,
) -> Result<ComObjectInstance, StoreError> {
    let (source_path, source_ets_id, device_id, number, size_kind, size_value, size_layer, is_active) =
        conn.query_row(
            "SELECT source_path, source_ets_id, device_id, number, size_kind, size_value,
                    size_layer, is_active
             FROM com_object_instance WHERE id = ?1",
            params![id.0],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, u16>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, bool>(7)?,
                ))
            },
        )?;
    let _ = device_id; // not part of ComObjectInstance's own fields beyond `device` below

    let mut stmt = conn.prepare(
        "SELECT attr, state, value, text_kind, layer FROM com_object_override
         WHERE com_object_instance_id = ?1",
    )?;
    let mut text = Override::Absent;
    let mut description = Override::Absent;
    let mut dpt = Override::Absent;
    let mut flags = ResolvedFlags::none();
    let rows = stmt
        .query_map(params![id.0], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (attr, state, value, text_kind, layer) in rows {
        match attr.as_str() {
            "text" => text = decode_text(&state, value, text_kind, layer),
            "description" => description = decode_text(&state, value, text_kind, layer),
            "dpt" => dpt = decode_dpt(&state, value, layer),
            "read" => flags.read = decode_bool(&state, value, layer),
            "write" => flags.write = decode_bool(&state, value, layer),
            "transmit" => flags.transmit = decode_bool(&state, value, layer),
            "update" => flags.update = decode_bool(&state, value, layer),
            "communication" => flags.communication = decode_bool(&state, value, layer),
            other => unreachable!("unknown com_object_override.attr {other:?}"),
        }
    }

    Ok(ComObjectInstance {
        id,
        source: SourceRef {
            path: source_path,
            ets_id: source_ets_id,
        },
        device: DeviceId(device_id),
        number,
        text,
        description,
        dpt,
        flags,
        size: decode_size(size_kind, size_value, size_layer),
        is_active,
        links: vec![],
    })
}

pub fn load_com_object_ids_for_device(
    conn: &Connection,
    device_id: DeviceId,
) -> Result<Vec<ComObjectInstanceId>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id FROM com_object_instance WHERE device_id = ?1 ORDER BY position",
    )?;
    let ids = stmt
        .query_map(params![device_id.0], |row| Ok(ComObjectInstanceId(row.get(0)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}
```

Append these tests to the existing `#[cfg(test)] mod tests` block in `devices.rs`:

```rust
    fn com_object_fixture() -> ComObjectInstance {
        ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: SourceRef { path: "0.xml".into(), ets_id: "M-1_O-0_R-1".into() },
            device: DeviceId(1),
            number: 0,
            text: Override::Value(Resolved { value: Text::Literal("An/Aus".into()), layer: Layer::Instance }),
            description: Override::Absent,
            dpt: Override::Value(Resolved { value: DptRef { main: 1, sub: Some(1) }, layer: Layer::UserEdit }),
            flags: ResolvedFlags {
                read: Override::Value(Resolved { value: true, layer: Layer::Instance }),
                write: Override::Absent,
                transmit: Override::Empty,
                update: Override::Malformed("???".into()),
                communication: Override::Value(Resolved { value: false, layer: Layer::Program }),
            },
            size: Some(Resolved { value: ObjectSize::Bit(1), layer: Layer::Program }),
            is_active: true,
            links: vec![],
        }
    }

    #[test]
    fn a_com_object_instance_round_trips_every_override_state() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let com = com_object_fixture();
        upsert_com_object_instance(&conn, d.id, 0, &com).unwrap();
        let loaded = load_com_object_instance(&conn, com.id).unwrap();
        assert_eq!(loaded.text, com.text);
        assert_eq!(loaded.description, com.description);
        assert_eq!(loaded.dpt, com.dpt);
        assert_eq!(loaded.flags, com.flags);
        assert_eq!(loaded.size, com.size);
        assert_eq!(loaded.is_active, com.is_active);
    }

    #[test]
    fn a_localized_text_override_round_trips_distinct_from_a_literal_one() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let mut com = com_object_fixture();
        com.text = Override::Value(Resolved {
            value: Text::Localized(LocalizedString(TranslationKey("k1".into()))),
            layer: Layer::Program,
        });
        upsert_com_object_instance(&conn, d.id, 0, &com).unwrap();
        let loaded = load_com_object_instance(&conn, com.id).unwrap();
        assert_eq!(loaded.text, com.text);
        assert_ne!(
            loaded.text,
            Override::Value(Resolved { value: Text::Literal("k1".into()), layer: Layer::Program })
        );
    }

    #[test]
    fn upsert_com_object_dpt_override_touches_only_the_dpt_row() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let com = com_object_fixture();
        upsert_com_object_instance(&conn, d.id, 0, &com).unwrap();

        let new_dpt = Override::Value(Resolved { value: DptRef { main: 5, sub: Some(1) }, layer: Layer::UserEdit });
        upsert_com_object_dpt_override(&conn, com.id, &new_dpt).unwrap();

        let loaded = load_com_object_instance(&conn, com.id).unwrap();
        assert_eq!(loaded.dpt, new_dpt);
        assert_eq!(loaded.text, com.text); // untouched by the targeted upsert
        assert_eq!(loaded.flags, com.flags); // untouched
    }
```

- [ ] **Step 2: Run, confirm pass**

Run: `cargo test -p knx-store devices:: 2>&1 | tail -60`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/knx-store/src/devices.rs
git commit -m "feat(knx-store): com-object-instance persistence, Override<T> codec"
```

---

### Task 8: `group.rs` — group_range, group_address

**Files:**
- Create: `crates/knx-store/src/group.rs`
- Modify: `crates/knx-store/src/lib.rs` (`pub mod group;`)

**Interfaces:**
- Consumes: `knx_core::group::{GroupRange, GroupAddressEntry}`, `knx_core::address::GroupAddress`, `knx_core::ids::{GroupRangeId, GroupAddressId, InstallationId, SourceRef}`.
- Produces:
  - `pub fn upsert_group_range(conn: &Connection, installation_id: InstallationId, position: i64, flat_position: i64, range: &GroupRange) -> Result<(), StoreError>`
  - `pub fn load_group_ranges(conn: &Connection, installation_id: InstallationId) -> Result<Vec<GroupRange>, StoreError>` — flat order via `flat_position`, `children` via `parent_id` + `position` (same pattern as Task 6's building parts).
  - `pub fn upsert_group_address(conn: &Connection, installation_id: InstallationId, position: i64, entry: &GroupAddressEntry) -> Result<(), StoreError>` — this is also the function `command_sync::sync_after_command` (Task 12) calls directly for `Command::CreateGroupAddress`.
  - `pub fn load_group_addresses(conn: &Connection, installation_id: InstallationId) -> Result<Vec<GroupAddressEntry>, StoreError>` — ordered by `position`.
  - `pub fn delete_group_address(conn: &Connection, id: GroupAddressId) -> Result<(), StoreError>` — this is what `Command::DeleteGroupAddress` syncs to.

- [ ] **Step 1: Write the failing test**

Create `crates/knx-store/src/group.rs`:

```rust
//! Persistence for `GroupRange` and `GroupAddressEntry` (DATA_MODEL §9).
//! `GroupRange` is stored flat with the same two-ordering pattern as
//! `BuildingPart` (`building.rs`): `flat_position` for `Installation::
//! group_ranges`, `position` (sibling order under `parent_id`) for
//! `GroupRange::children`.

use rusqlite::{params, Connection};

use knx_core::address::GroupAddress;
use knx_core::group::{GroupAddressEntry, GroupRange};
use knx_core::ids::{GroupAddressId, GroupRangeId, InstallationId, SourceRef};

use crate::StoreError;

pub fn upsert_group_range(
    conn: &Connection,
    installation_id: InstallationId,
    position: i64,
    flat_position: i64,
    range: &GroupRange,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO group_range
             (id, installation_id, parent_id, position, flat_position, source_path,
              source_ets_id, name, range_start, range_end)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
             installation_id = excluded.installation_id,
             parent_id = excluded.parent_id,
             position = excluded.position,
             flat_position = excluded.flat_position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             range_start = excluded.range_start,
             range_end = excluded.range_end",
        params![
            range.id.0,
            installation_id.0,
            range.parent.map(|p| p.0),
            position,
            flat_position,
            range.source.path,
            range.source.ets_id,
            range.name,
            range.start.raw(),
            range.end.raw(),
        ],
    )?;
    Ok(())
}

pub fn load_group_ranges(
    conn: &Connection,
    installation_id: InstallationId,
) -> Result<Vec<GroupRange>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, source_path, source_ets_id, name, range_start, range_end
         FROM group_range WHERE installation_id = ?1 ORDER BY flat_position",
    )?;
    let rows = stmt
        .query_map(params![installation_id.0], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, Option<u32>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, u16>(5)?,
                row.get::<_, u16>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    rows.into_iter()
        .map(|(id, parent_id, source_path, source_ets_id, name, start, end)| {
            let range_id = GroupRangeId(id);
            let mut children_stmt =
                conn.prepare("SELECT id FROM group_range WHERE parent_id = ?1 ORDER BY position")?;
            let children = children_stmt
                .query_map(params![range_id.0], |row| Ok(GroupRangeId(row.get(0)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, StoreError>(GroupRange {
                id: range_id,
                source: SourceRef { path: source_path, ets_id: source_ets_id },
                name,
                start: GroupAddress::from_raw(start),
                end: GroupAddress::from_raw(end),
                parent: parent_id.map(GroupRangeId),
                children,
            })
        })
        .collect()
}

pub fn upsert_group_address(
    conn: &Connection,
    installation_id: InstallationId,
    position: i64,
    entry: &GroupAddressEntry,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO group_address
             (id, installation_id, range_id, position, source_path, source_ets_id, name,
              address, central, unfiltered)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
             installation_id = excluded.installation_id,
             range_id = excluded.range_id,
             position = excluded.position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             address = excluded.address,
             central = excluded.central,
             unfiltered = excluded.unfiltered",
        params![
            entry.id.0,
            installation_id.0,
            entry.range.map(|r| r.0),
            position,
            entry.source.path,
            entry.source.ets_id,
            entry.name,
            entry.address.raw(),
            entry.central,
            entry.unfiltered,
        ],
    )?;
    Ok(())
}

pub fn load_group_addresses(
    conn: &Connection,
    installation_id: InstallationId,
) -> Result<Vec<GroupAddressEntry>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, range_id, source_path, source_ets_id, name, address, central, unfiltered
         FROM group_address WHERE installation_id = ?1 ORDER BY position",
    )?;
    let entries = stmt
        .query_map(params![installation_id.0], |row| {
            Ok(GroupAddressEntry {
                id: GroupAddressId(row.get(0)?),
                range: row.get::<_, Option<u32>>(1)?.map(GroupRangeId),
                source: SourceRef {
                    path: row.get(2)?,
                    ets_id: row.get(3)?,
                },
                name: row.get(4)?,
                address: GroupAddress::from_raw(row.get(5)?),
                central: row.get(6)?,
                unfiltered: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

pub fn delete_group_address(conn: &Connection, id: GroupAddressId) -> Result<(), StoreError> {
    conn.execute("DELETE FROM group_link WHERE group_address_id = ?1", params![id.0])?;
    conn.execute("DELETE FROM group_address WHERE id = ?1", params![id.0])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use crate::topology::upsert_installation_row;
    use knx_core::commissioning::CompletionStatus;
    use knx_core::installation::Installation;
    use knx_core::topology::Topology;

    fn installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "H".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology { areas: vec![], lines: vec![], unassigned: vec![] },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    fn source() -> SourceRef {
        SourceRef { path: "t".into(), ets_id: "t".into() }
    }

    #[test]
    fn nested_group_ranges_round_trip_with_correct_children_and_flat_order() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
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
            parent: Some(GroupRangeId(1)),
            children: vec![],
        };
        upsert_group_range(&conn, InstallationId(0), 0, 0, &main).unwrap();
        upsert_group_range(&conn, InstallationId(0), 0, 1, &middle).unwrap();

        let loaded = load_group_ranges(&conn, InstallationId(0)).unwrap();
        assert_eq!(loaded, vec![main, middle]);
    }

    #[test]
    fn a_group_address_round_trips_and_deletes_cleanly() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let entry = GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "Licht EG An/Aus".into(),
            address: GroupAddress::from_raw(2048),
            central: false,
            unfiltered: false,
            range: None,
        };
        upsert_group_address(&conn, InstallationId(0), 0, &entry).unwrap();
        assert_eq!(load_group_addresses(&conn, InstallationId(0)).unwrap(), vec![entry.clone()]);
        delete_group_address(&conn, entry.id).unwrap();
        assert_eq!(load_group_addresses(&conn, InstallationId(0)).unwrap(), vec![]);
    }

    #[test]
    fn a_group_address_without_a_range_is_valid() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let entry = GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "Sonder".into(),
            address: GroupAddress::from_raw(1),
            central: true,
            unfiltered: true,
            range: None,
        };
        upsert_group_address(&conn, InstallationId(0), 0, &entry).unwrap();
        let loaded = load_group_addresses(&conn, InstallationId(0)).unwrap();
        assert_eq!(loaded[0].range, None);
    }
}
```

Add `pub mod group;` to `crates/knx-store/src/lib.rs`.

- [ ] **Step 2: Run, confirm pass**

Run: `cargo test -p knx-store group:: 2>&1 | tail -40`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/knx-store/src/group.rs crates/knx-store/src/lib.rs
git commit -m "feat(knx-store): group range/address persistence"
```

---

### Task 9: `devices.rs` (part C) — group_link

**Files:**
- Modify: `crates/knx-store/src/devices.rs` (append)

**Interfaces:**
- Consumes: `knx_core::flags::{Direction, GroupLink}`, `knx_core::ids::{GroupAddressId, ComObjectInstanceId}`.
- Produces:
  - `pub fn upsert_group_links(conn: &Connection, com_object_instance_id: ComObjectInstanceId, links: &[GroupLink]) -> Result<(), StoreError>` — replaces all of one com object's links.
  - `pub fn load_group_links(conn: &Connection, com_object_instance_id: ComObjectInstanceId) -> Result<Vec<GroupLink>, StoreError>` ordered by `position`.

- [ ] **Step 1: Write the failing test**

Append to `crates/knx-store/src/devices.rs`'s top `use` block:

```rust
use knx_core::flags::{Direction, GroupLink};
use knx_core::ids::GroupAddressId;
```

Append after `load_com_object_ids_for_device`:

```rust
fn direction_to_str(d: Direction) -> &'static str {
    match d {
        Direction::Send => "send",
        Direction::Receive => "receive",
    }
}

fn direction_from_str(s: &str) -> Direction {
    match s {
        "receive" => Direction::Receive,
        _ => Direction::Send,
    }
}

pub fn upsert_group_links(
    conn: &Connection,
    com_object_instance_id: ComObjectInstanceId,
    links: &[GroupLink],
) -> Result<(), StoreError> {
    conn.execute(
        "DELETE FROM group_link WHERE com_object_instance_id = ?1",
        params![com_object_instance_id.0],
    )?;
    let mut stmt = conn.prepare(
        "INSERT INTO group_link (com_object_instance_id, group_address_id, direction, position)
         VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (i, link) in links.iter().enumerate() {
        stmt.execute(params![
            com_object_instance_id.0,
            link.ga.0,
            direction_to_str(link.direction),
            i as i64,
        ])?;
    }
    Ok(())
}

pub fn load_group_links(
    conn: &Connection,
    com_object_instance_id: ComObjectInstanceId,
) -> Result<Vec<GroupLink>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT group_address_id, direction FROM group_link
         WHERE com_object_instance_id = ?1 ORDER BY position",
    )?;
    let links = stmt
        .query_map(params![com_object_instance_id.0], |row| {
            let ga: u32 = row.get(0)?;
            let direction: String = row.get(1)?;
            Ok((ga, direction))
        })?
        .map(|r| {
            let (ga, direction) = r?;
            Ok(GroupLink {
                ga: GroupAddressId(ga),
                direction: direction_from_str(&direction),
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(links)
}
```

Append this test to `devices.rs`'s test module (it needs a `group_address` row, so it also exercises the cross-module dependency on Task 8's table — insert that row directly via SQL, same style as the earlier `set_device_line` test's direct `line`/`area` inserts):

```rust
    #[test]
    fn group_links_round_trip_in_order() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let com = com_object_fixture();
        upsert_com_object_instance(&conn, d.id, 0, &com).unwrap();
        for id in [10, 20] {
            conn.execute(
                &format!(
                    "INSERT INTO group_address (id, installation_id, range_id, position,
                        source_path, source_ets_id, name, address, central, unfiltered)
                     VALUES ({id}, 0, NULL, 0, 't', 't', 'GA', {id}, 0, 0)"
                ),
                [],
            )
            .unwrap();
        }
        let links = vec![
            GroupLink { ga: GroupAddressId(20), direction: Direction::Send },
            GroupLink { ga: GroupAddressId(10), direction: Direction::Receive },
        ];
        upsert_group_links(&conn, com.id, &links).unwrap();
        assert_eq!(load_group_links(&conn, com.id).unwrap(), links);
    }
```

- [ ] **Step 2: Run, confirm pass**

Run: `cargo test -p knx-store devices:: 2>&1 | tail -60`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/knx-store/src/devices.rs
git commit -m "feat(knx-store): group-link persistence"
```

---

### Task 10: `parameter.rs` — parameter_instance

**Files:**
- Create: `crates/knx-store/src/parameter.rs`
- Modify: `crates/knx-store/src/lib.rs` (`pub mod parameter;`)

**Interfaces:**
- Consumes: `knx_core::parameter::ParameterInstance`, `knx_core::ids::{DeviceId, ParameterInstanceId, SourceRef}`.
- Produces: `pub fn upsert_parameter_instance(conn: &Connection, position: i64, p: &ParameterInstance) -> Result<(), StoreError>`, `pub fn load_parameters_for_installation(conn: &Connection, installation_id: InstallationId) -> Result<Vec<ParameterInstance>, StoreError>` — parameters have no `installation_id` column of their own (only `device_id`); this loader joins through `device` to filter by installation, ordered by `parameter_instance.position`.

- [ ] **Step 1: Write the failing test**

Create `crates/knx-store/src/parameter.rs`:

```rust
//! Persistence for `ParameterInstance` — retained but uninterpreted
//! (DATA_MODEL §10). `raw` is never parsed here or anywhere else yet.

use rusqlite::{params, Connection};

use knx_core::ids::{InstallationId, ParameterInstanceId, SourceRef};
use knx_core::parameter::ParameterInstance;

use crate::StoreError;

pub fn upsert_parameter_instance(
    conn: &Connection,
    position: i64,
    p: &ParameterInstance,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO parameter_instance (id, device_id, position, source_path, source_ets_id, raw)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
             device_id = excluded.device_id,
             position = excluded.position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             raw = excluded.raw",
        params![p.id.0, p.device.0, position, p.source.path, p.source.ets_id, p.raw],
    )?;
    Ok(())
}

pub fn load_parameters_for_installation(
    conn: &Connection,
    installation_id: InstallationId,
) -> Result<Vec<ParameterInstance>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.device_id, p.source_path, p.source_ets_id, p.raw
         FROM parameter_instance p JOIN device d ON p.device_id = d.id
         WHERE d.installation_id = ?1 ORDER BY p.position",
    )?;
    let params_ = stmt
        .query_map(params![installation_id.0], |row| {
            Ok(ParameterInstance {
                id: ParameterInstanceId(row.get(0)?),
                device: knx_core::ids::DeviceId(row.get(1)?),
                source: SourceRef {
                    path: row.get(2)?,
                    ets_id: row.get(3)?,
                },
                raw: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(params_)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::upsert_device;
    use crate::open_and_migrate_in_memory;
    use crate::topology::upsert_installation_row;
    use knx_core::commissioning::{CommissioningState, CompletionStatus};
    use knx_core::device::DeviceInstance;
    use knx_core::installation::Installation;
    use knx_core::topology::Topology;

    fn installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "H".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology { areas: vec![], lines: vec![], unassigned: vec![] },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    fn device() -> DeviceInstance {
        DeviceInstance {
            id: knx_core::ids::DeviceId(1),
            source: SourceRef { path: "t".into(), ets_id: "t".into() },
            name: "D".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    #[test]
    fn parameters_round_trip_in_position_order() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();

        let p1 = ParameterInstance {
            id: ParameterInstanceId(1),
            device: d.id,
            source: SourceRef { path: "t".into(), ets_id: "M-1_P-1_R-1".into() },
            raw: "1".into(),
        };
        let p2 = ParameterInstance {
            id: ParameterInstanceId(2),
            device: d.id,
            source: SourceRef { path: "t".into(), ets_id: "M-1_UP-2_R-2".into() },
            raw: "42".into(),
        };
        upsert_parameter_instance(&conn, 0, &p1).unwrap();
        upsert_parameter_instance(&conn, 1, &p2).unwrap();
        assert_eq!(
            load_parameters_for_installation(&conn, InstallationId(0)).unwrap(),
            vec![p1, p2]
        );
    }
}
```

Add `pub mod parameter;` to `crates/knx-store/src/lib.rs`.

- [ ] **Step 2: Run, confirm pass**

Run: `cargo test -p knx-store parameter:: 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/knx-store/src/parameter.rs crates/knx-store/src/lib.rs
git commit -m "feat(knx-store): parameter-instance persistence"
```

---

### Task 11: `project.rs` — save_project / load_project orchestration

**Files:**
- Create: `crates/knx-store/src/project.rs`
- Modify: `crates/knx-store/src/lib.rs` (`pub mod project; pub use project::{save_project, load_project};`)
- Modify: `crates/knx-core/src/project.rs` (`impl IdAllocators` gains eight `peek_*` getters and `from_counts` — see Step 1 below)

**Interfaces:**
- Consumes: every `upsert_*`/`load_*` function from Tasks 3–10, plus `knx_core::project::{Project, ProjectInfo, IdAllocators}`, `knx_core::address::GroupAddressStyle`, `knx_core::string_table::Language`.
- Produces: `pub fn save_project(conn: &Connection, project: &Project) -> Result<(), StoreError>`, `pub fn load_project(conn: &Connection) -> Result<Project, StoreError>`.

- [ ] **Step 1: Write the failing test**

Create `crates/knx-store/src/project.rs`:

```rust
//! Orchestrates every entity module into one `save_project`/`load_project`
//! pair — the only two functions outside this crate that see the whole
//! `knx_core::Project` graph at once (design doc:
//! docs/superpowers/specs/2026-09-03-knx-entity-persistence-design.md).

use rusqlite::{params, Connection};

use knx_core::address::GroupAddressStyle;
use knx_core::ids::InstallationId;
use knx_core::installation::Installation;
use knx_core::project::{IdAllocators, Project, ProjectInfo};
use knx_core::string_table::Language;

use crate::building::{load_buildings, upsert_building_part};
use crate::devices::{
    load_all_device_ids, load_com_object_ids_for_device, load_com_object_instance, load_device,
    load_group_links, set_device_line, upsert_com_object_instance, upsert_device,
    upsert_group_links,
};
use crate::group::{load_group_addresses, load_group_ranges, upsert_group_address, upsert_group_range};
use crate::parameter::{load_parameters_for_installation, upsert_parameter_instance};
use crate::strings::{load_string_table, upsert_string_table};
use crate::topology::{
    completion_from_str, completion_to_str, load_installation_rows, load_topology,
    upsert_area, upsert_installation_row, upsert_line,
};
use crate::StoreError;

fn style_to_str(s: GroupAddressStyle) -> &'static str {
    match s {
        GroupAddressStyle::Free => "Free",
        GroupAddressStyle::TwoLevel => "TwoLevel",
        GroupAddressStyle::ThreeLevel => "ThreeLevel",
    }
}

fn style_from_str(s: &str) -> GroupAddressStyle {
    match s {
        "Free" => GroupAddressStyle::Free,
        "TwoLevel" => GroupAddressStyle::TwoLevel,
        _ => GroupAddressStyle::ThreeLevel,
    }
}

const DELETE_ALL_TABLES: &[&str] = &[
    // Child-to-parent order — matches the reverse of the insert order below.
    "com_object_override",
    "group_link",
    "com_object_instance",
    "binary_data_ref",
    "building_part_device",
    "parameter_instance",
    "group_address",
    "group_range",
    "device",
    "building_part",
    "line",
    "area",
    "installation",
    "string_table_entry",
    "id_allocators",
    "project_info",
];

pub fn save_project(conn: &Connection, project: &Project) -> Result<(), StoreError> {
    let tx = conn.unchecked_transaction()?;

    // `building_part` and `group_range` self-reference via `parent_id`.
    // Under `PRAGMA foreign_keys = ON`, a bulk `DELETE FROM` on a
    // self-referencing table risks the constraint being checked against a
    // row this same statement has not deleted yet (SQLite does not
    // guarantee an all-at-once "no rows left, so nothing to violate"
    // ordering here — only that no *other* table's FK is left dangling).
    // Breaking every self-reference first makes the two DELETEs below
    // unconditionally safe regardless of internal row order.
    tx.execute("UPDATE building_part SET parent_id = NULL", [])?;
    tx.execute("UPDATE group_range SET parent_id = NULL", [])?;

    for table in DELETE_ALL_TABLES {
        tx.execute(&format!("DELETE FROM {table}"), [])?;
    }

    tx.execute(
        "INSERT INTO project_info
             (id, project_id, name, project_number, group_address_style, completion,
              last_modified, project_start, default_language)
         VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            project.info.project_id,
            project.info.name,
            project.info.project_number,
            style_to_str(project.info.group_address_style),
            completion_to_str(project.info.completion),
            project.info.last_modified.map(|d| d.to_rfc3339()),
            project.info.project_start.map(|d| d.to_rfc3339()),
            project.strings.default_language().0,
        ],
    )?;

    tx.execute(
        "INSERT INTO id_allocators
             (id, device, area, line, com_object_instance, group_range, group_address,
              building_part, parameter_instance)
         VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            project.ids.peek_device(),
            project.ids.peek_area(),
            project.ids.peek_line(),
            project.ids.peek_com_object_instance(),
            project.ids.peek_group_range(),
            project.ids.peek_group_address(),
            project.ids.peek_building_part(),
            project.ids.peek_parameter_instance(),
        ],
    )?;

    upsert_string_table(&tx, &project.strings)?;

    for installation in &project.installations {
        upsert_installation_row(&tx, installation)?;

        for (i, area) in installation.topology.areas.iter().enumerate() {
            upsert_area(&tx, installation.id, i as i64, area)?;
            for (j, line_id) in area.lines.iter().enumerate() {
                let line = installation
                    .topology
                    .line(*line_id)
                    .expect("Area::lines only ever names lines that exist in this Topology");
                upsert_line(&tx, area.id, j as i64, line)?;
            }
        }

        // One pass per device placement — line-assigned devices, then
        // unassigned ones — writing the device row and its topology
        // placement together instead of in three separate passes over the
        // same ids.
        for line in &installation.topology.lines {
            for (i, device_id) in line.devices.iter().enumerate() {
                let device = project
                    .devices
                    .get(*device_id)
                    .expect("Line::devices only ever names devices that exist in Devices");
                upsert_device(&tx, installation.id, i as i64, device)?;
                set_device_line(&tx, *device_id, Some(line.id), i as i64)?;
            }
        }
        for (i, device_id) in installation.topology.unassigned.iter().enumerate() {
            let device = project
                .devices
                .get(*device_id)
                .expect("Topology::unassigned only ever names devices that exist in Devices");
            upsert_device(&tx, installation.id, i as i64, device)?;
            set_device_line(&tx, *device_id, None, i as i64)?;
        }

        for (i, flat_position, part) in flatten_buildings(&installation.buildings) {
            upsert_building_part(&tx, installation.id, i, flat_position, part)?;
        }

        for (i, flat_position, range) in flatten_ranges(&installation.group_ranges) {
            upsert_group_range(&tx, installation.id, i, flat_position, range)?;
        }

        for (i, entry) in installation.group_addresses.iter().enumerate() {
            upsert_group_address(&tx, installation.id, i as i64, entry)?;
        }

        for (i, p) in installation.parameters.iter().enumerate() {
            upsert_parameter_instance(&tx, i as i64, p)?;
        }
    }

    // Com-object instances + group links: one pass over every device's
    // `com_objects`, independent of which installation the device belongs
    // to — `com_object_instance` has no `installation_id` column of its
    // own, only `device_id`, so this does not need to be nested inside the
    // installation loop above.
    for device in project.devices.iter() {
        for (j, com_id) in device.com_objects.iter().enumerate() {
            let com = project
                .devices
                .com_object(*com_id)
                .expect("DeviceInstance::com_objects only ever names existing com objects");
            upsert_com_object_instance(&tx, device.id, j as i64, com)?;
            upsert_group_links(&tx, com.id, &com.links)?;
        }
    }

    tx.commit()?;
    Ok(())
}

/// `Installation::buildings`/`::group_ranges` are flat `Vec`s where each
/// element also carries `position` (sibling order under its own
/// `parent`/`parent_id`, used to rebuild `children`) — see the design
/// doc's "owned-list vs flat-list order" note. `flat_position` is simply
/// the element's index in the flat `Vec`; `position` is recomputed here as
/// the 0-based rank among siblings sharing the same parent, in flat-list
/// order (which is the pre-order the importer itself produces — verified
/// by Task 6/8's own hierarchy tests).
fn flatten_buildings(
    buildings: &[knx_core::building::BuildingPart],
) -> Vec<(i64, i64, &knx_core::building::BuildingPart)> {
    sibling_positions(buildings, |p| p.parent.map(|x| x.0))
        .into_iter()
        .enumerate()
        .map(|(flat, (sib, part))| (sib, flat as i64, part))
        .collect()
}

fn flatten_ranges(ranges: &[knx_core::group::GroupRange]) -> Vec<(i64, i64, &knx_core::group::GroupRange)> {
    sibling_positions(ranges, |r| r.parent.map(|x| x.0))
        .into_iter()
        .enumerate()
        .map(|(flat, (sib, range))| (sib, flat as i64, range))
        .collect()
}

/// For each element, its 0-based rank among the elements preceding it (in
/// slice order) that share its `parent_key`.
fn sibling_positions<T>(items: &[T], parent_key: impl Fn(&T) -> Option<u32>) -> Vec<(i64, &T)> {
    let mut counts: std::collections::HashMap<Option<u32>, i64> = std::collections::HashMap::new();
    items
        .iter()
        .map(|item| {
            let key = parent_key(item);
            let count = counts.entry(key).or_insert(0);
            let position = *count;
            *count += 1;
            (position, item)
        })
        .collect()
}

pub fn load_project(conn: &Connection) -> Result<Project, StoreError> {
    let (project_id, name, project_number, style, completion, last_modified, project_start, default_language) =
        conn.query_row(
            "SELECT project_id, name, project_number, group_address_style, completion,
                    last_modified, project_start, default_language
             FROM project_info WHERE id = 0",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional_not_saved()?;

    let strings = load_string_table(conn, Language(default_language))?;

    let ids = conn.query_row(
        "SELECT device, area, line, com_object_instance, group_range, group_address,
                building_part, parameter_instance
         FROM id_allocators WHERE id = 0",
        [],
        |row| {
            Ok(IdAllocators::from_counts(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        },
    )?;

    let mut devices = knx_core::devices::Devices::new();
    for device_id in load_all_device_ids(conn)? {
        let mut device = load_device(conn, device_id)?;
        device.com_objects = load_com_object_ids_for_device(conn, device_id)?;
        for com_id in device.com_objects.clone() {
            let mut com = load_com_object_instance(conn, com_id)?;
            com.links = load_group_links(conn, com_id)?;
            devices.insert_com_object(com);
        }
        devices.insert(device);
    }

    let mut installations = Vec::new();
    for row in load_installation_rows(conn)? {
        let topology = load_topology(conn, row.id)?;
        let buildings = load_buildings(conn, row.id)?;
        let group_ranges = load_group_ranges(conn, row.id)?;
        let group_addresses = load_group_addresses(conn, row.id)?;
        let parameters = load_parameters_for_installation(conn, row.id)?;
        installations.push(Installation {
            id: row.id,
            name: row.name,
            default_line: row.default_line,
            multicast_address: row.multicast_address,
            completion: row.completion,
            topology,
            buildings,
            group_ranges,
            group_addresses,
            parameters,
        });
    }

    Ok(Project {
        schema_version: knx_core::project::CURRENT_SCHEMA_VERSION,
        strings,
        info: ProjectInfo {
            project_id,
            name,
            project_number,
            group_address_style: style_from_str(&style),
            completion: completion_from_str(&completion),
            last_modified: last_modified.map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .expect("stored timestamp is always valid RFC3339")
                    .with_timezone(&chrono::Utc)
            }),
            project_start: project_start.map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .expect("stored timestamp is always valid RFC3339")
                    .with_timezone(&chrono::Utc)
            }),
        },
        installations,
        devices,
        ids,
    })
}

/// Turns `query_row`'s `QueryReturnedNoRows` specifically into
/// `StoreError::NotSaved` — every other `rusqlite::Error` still becomes
/// `StoreError::Sqlite` via the existing `From` impl.
trait OptionalNotSaved<T> {
    fn optional_not_saved(self) -> Result<T, StoreError>;
}

impl<T> OptionalNotSaved<T> for Result<T, rusqlite::Error> {
    fn optional_not_saved(self) -> Result<T, StoreError> {
        match self {
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(StoreError::NotSaved),
            other => other.map_err(StoreError::from),
        }
    }
}
```

This references `IdAllocators::from_counts` and eight `peek_*` getters that do not exist yet — `IdAllocators`'s fields are private and its only public API today is the eight `next_*` mutators. Add these now, as part of this task's own commit, to `crates/knx-core/src/project.rs`'s `impl IdAllocators` block (Task 1 already landed and was reviewed without this addition — do not amend that commit; this task adds `knx-core/src/project.rs` to its own `Modify` list and its own commit instead):

```rust
    pub fn peek_device(&self) -> u32 {
        self.device
    }
    pub fn peek_area(&self) -> u32 {
        self.area
    }
    pub fn peek_line(&self) -> u32 {
        self.line
    }
    pub fn peek_com_object_instance(&self) -> u32 {
        self.com_object_instance
    }
    pub fn peek_group_range(&self) -> u32 {
        self.group_range
    }
    pub fn peek_group_address(&self) -> u32 {
        self.group_address
    }
    pub fn peek_building_part(&self) -> u32 {
        self.building_part
    }
    pub fn peek_parameter_instance(&self) -> u32 {
        self.parameter_instance
    }

    /// Reconstructs an `IdAllocators` at exactly the counts given —
    /// `knx-store::load_project`'s way of restoring allocator state so a
    /// freshly loaded project never reissues an id already in use.
    #[allow(clippy::too_many_arguments)]
    pub fn from_counts(
        device: u32,
        area: u32,
        line: u32,
        com_object_instance: u32,
        group_range: u32,
        group_address: u32,
        building_part: u32,
        parameter_instance: u32,
    ) -> Self {
        Self {
            device,
            area,
            line,
            com_object_instance,
            group_range,
            group_address,
            building_part,
            parameter_instance,
        }
    }
```

Amend Task 1's commit (or add a small follow-up commit now referencing Task 1's file) with this addition before continuing.

Add to `crates/knx-store/src/lib.rs`:

```rust
pub mod project;
pub use project::{load_project, save_project};
```

Now the round-trip tests. Append to `project.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use knx_core::address::GroupAddress;
    use knx_core::building::{BuildingPart, BuildingPartType};
    use knx_core::commissioning::{CommissioningState, CompletionStatus};
    use knx_core::device::DeviceInstance;
    use knx_core::group::{GroupAddressEntry, GroupRange};
    use knx_core::ids::{AreaId, BuildingPartId, DeviceId, GroupAddressId, GroupRangeId, LineId, SourceRef};
    use knx_core::topology::{Area, Line, Topology};

    fn source() -> SourceRef {
        SourceRef { path: "t".into(), ets_id: "t".into() }
    }

    #[test]
    fn an_empty_project_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let project = Project::new(Language("en".into()));
        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded, project);
    }

    #[test]
    fn loading_a_never_saved_database_is_not_saved_not_an_empty_project() {
        let conn = open_and_migrate_in_memory().unwrap();
        assert!(matches!(load_project(&conn), Err(StoreError::NotSaved)));
    }

    #[test]
    fn a_project_with_one_device_and_an_unassigned_one_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(Language("de-DE".into()));
        let line = Line {
            id: LineId(1),
            source: source(),
            name: "HL".into(),
            address: 1,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::FinishedDesign,
            devices: vec![DeviceId(1)],
        };
        let area = Area {
            id: AreaId(1),
            source: source(),
            name: "A1".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![line.id],
        };
        let installation = Installation {
            id: InstallationId(0),
            name: "Haus".into(),
            default_line: Some(line.id),
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![area],
                lines: vec![line],
                unassigned: vec![DeviceId(2)],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        };
        project.installations.push(installation);
        for id in [1, 2] {
            project.devices.insert(DeviceInstance {
                id: DeviceId(id),
                source: source(),
                name: format!("D{id}"),
                description: None,
                address: None,
                product_ref: "P".into(),
                program_ref: "H".into(),
                commissioning: CommissioningState::default(),
                visibility_calculated: true,
                com_objects: vec![],
                binary_data: vec![],
            });
        }

        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded, project);
    }

    #[test]
    fn a_project_with_nested_buildings_and_group_ranges_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(Language("en".into()));
        let building = BuildingPart {
            id: BuildingPartId(1),
            source: source(),
            name: "House".into(),
            number: None,
            kind: BuildingPartType::Building,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: vec![BuildingPartId(2)],
            devices: vec![],
            parent: None,
        };
        let floor = BuildingPart {
            id: BuildingPartId(2),
            source: source(),
            name: "Floor 1".into(),
            number: Some("1".into()),
            kind: BuildingPartType::Floor,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: vec![],
            devices: vec![],
            parent: Some(BuildingPartId(1)),
        };
        let main_range = GroupRange {
            id: GroupRangeId(1),
            source: source(),
            name: "Licht".into(),
            start: GroupAddress::from_raw(0),
            end: GroupAddress::from_raw(2047),
            parent: None,
            children: vec![GroupRangeId(2)],
        };
        let mid_range = GroupRange {
            id: GroupRangeId(2),
            source: source(),
            name: "Licht - An/Aus".into(),
            start: GroupAddress::from_raw(0),
            end: GroupAddress::from_raw(255),
            parent: Some(GroupRangeId(1)),
            children: vec![],
        };
        let ga = GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "EG Licht".into(),
            address: GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(2)),
        };
        project.installations.push(Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology { areas: vec![], lines: vec![], unassigned: vec![] },
            buildings: vec![building, floor],
            group_ranges: vec![main_range, mid_range],
            group_addresses: vec![ga],
            parameters: vec![],
        });

        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded, project);

        // Re-save on top of existing self-referencing rows (`building_part`/
        // `group_range`'s `parent_id`) — the case a single-save round trip
        // above never exercises, and the one `PRAGMA foreign_keys = ON`
        // bulk-delete could violate if the two tables' `parent_id` columns
        // are not neutralized before their rows are deleted.
        save_project(&conn, &project).unwrap();
        let loaded_again = load_project(&conn).unwrap();
        assert_eq!(loaded_again, project);
    }

    #[test]
    fn allocator_state_survives_a_round_trip_and_next_id_does_not_collide() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(Language("en".into()));
        let _ = project.ids.next_device_id(); // DeviceId(1)
        let second = project.ids.next_device_id(); // DeviceId(2)
        save_project(&conn, &project).unwrap();
        let mut loaded = load_project(&conn).unwrap();
        let next = loaded.ids.next_device_id();
        assert_eq!(next, DeviceId(3));
        assert_ne!(next, second);
    }
}
```

- [ ] **Step 2: Run, confirm failure, then fix, then pass**

Run: `cargo test -p knx-store project:: 2>&1 | tail -80`
Expected: first FAIL to compile (missing `IdAllocators::peek_*`/`from_counts` until the Task-1 addition above is made), then PASS once that addition and this file are both in place. Also run the full crate: `cargo test -p knx-store 2>&1 | tail -20` — every earlier task's tests must still be green (this task adds no new tables or columns, only orchestration).

- [ ] **Step 3: Commit**

```bash
git add crates/knx-core/src/project.rs crates/knx-store/src/project.rs crates/knx-store/src/lib.rs
git commit -m "feat(knx-store): save_project/load_project full-project round trip"
```

---

### Task 12: `command_sync.rs` — incremental sync after a `Command`

**Files:**
- Create: `crates/knx-store/src/command_sync.rs`
- Modify: `crates/knx-store/src/lib.rs` (`pub mod command_sync; pub use command_sync::sync_after_command;`)

**Interfaces:**
- Consumes: `knx_core::command::Command`, `knx_core::project::Project`; `devices::{upsert_device, upsert_com_object_dpt_override, set_device_line}` (Tasks 5/7); `group::{upsert_group_address, delete_group_address}` (Task 8).
- Produces: `pub fn sync_after_command(conn: &Connection, installation_id: InstallationId, project: &Project, command: &Command) -> Result<(), StoreError>`. `installation_id` is a required parameter (not derivable from `Command` alone) — the caller (future `knx-app` wiring, out of scope this cycle) knows which installation it is editing.

- [ ] **Step 1: Write the failing test**

Create `crates/knx-store/src/command_sync.rs`:

```rust
//! Incremental persistence: after `Command::apply(&mut project)` succeeds,
//! `sync_after_command` writes only the row(s) that command's own target
//! id(s) name, reading the resulting state out of the already-mutated
//! `project` rather than re-deriving `command.rs`'s own mutation logic
//! (design doc, "Incremental command sync"). Grows as `command.rs` grows —
//! today's four variants are all that exist.

use rusqlite::Connection;

use knx_core::command::Command;
use knx_core::ids::InstallationId;
use knx_core::project::Project;

use crate::devices::{set_device_line, upsert_com_object_dpt_override, upsert_device};
use crate::group::{delete_group_address, upsert_group_address};
use crate::StoreError;

/// Call only after a successful `Command::apply(&mut project)`, passing the
/// resulting `project`. Applies equally to undo/redo, since both replay
/// through this same `Command` enum — a `RestoreComObjectDpt` produced by
/// undoing a `SetComObjectDpt` is itself a `Command`, synced the same way.
///
/// `installation_id` names which installation's topology position bookkeeping
/// applies (`SetIndividualAddress` does not move a device between lines, so
/// its existing line/position is looked up and re-asserted rather than
/// changed).
pub fn sync_after_command(
    conn: &Connection,
    installation_id: InstallationId,
    project: &Project,
    command: &Command,
) -> Result<(), StoreError> {
    let tx = conn.unchecked_transaction()?;
    match command {
        Command::SetIndividualAddress { device, .. } => {
            let d = project
                .devices
                .get(*device)
                .expect("Command::apply already proved this device exists");
            let (line_id, position): (Option<i64>, i64) = tx.query_row(
                "SELECT line_id, topology_position FROM device WHERE id = ?1",
                [d.id.0],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            upsert_device(&tx, installation_id, position, d)?;
            set_device_line(
                &tx,
                d.id,
                line_id.map(|l| knx_core::ids::LineId(l as u32)),
                position,
            )?;
        }
        Command::SetComObjectDpt { com_object, .. }
        | Command::RestoreComObjectDpt { com_object, .. } => {
            let com = project
                .devices
                .com_object(*com_object)
                .expect("Command::apply already proved this com object exists");
            upsert_com_object_dpt_override(&tx, com.id, &com.dpt)?;
        }
        Command::CreateGroupAddress { entry } => {
            let position: i64 = tx
                .query_row(
                    "SELECT COALESCE(MAX(position) + 1, 0) FROM group_address WHERE installation_id = ?1",
                    [installation_id.0],
                    |row| row.get(0),
                )?;
            upsert_group_address(&tx, installation_id, position, entry)?;
        }
        Command::DeleteGroupAddress { id } => {
            delete_group_address(&tx, *id)?;
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use crate::topology::upsert_installation_row;
    use knx_core::address::{GroupAddress, IndividualAddress};
    use knx_core::commissioning::{CommissioningState, CompletionStatus};
    use knx_core::device::DeviceInstance;
    use knx_core::group::GroupAddressEntry;
    use knx_core::ids::{DeviceId, GroupAddressId, SourceRef};
    use knx_core::installation::Installation;
    use knx_core::topology::Topology;

    fn source() -> SourceRef {
        SourceRef { path: "t".into(), ets_id: "t".into() }
    }

    fn installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "H".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology { areas: vec![], lines: vec![], unassigned: vec![] },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    fn project_with_one_unassigned_device() -> Project {
        let mut project = Project::new(knx_core::string_table::Language("en".into()));
        let mut installation = installation();
        installation.topology.unassigned = vec![DeviceId(1)];
        project.installations.push(installation);
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: source(),
            name: "D".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        project
    }

    #[test]
    fn set_individual_address_syncs_only_the_device_row() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_unassigned_device();
        crate::save_project(&conn, &project).unwrap();

        let command = Command::SetIndividualAddress {
            device: DeviceId(1),
            address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
        };
        command.apply(&mut project).unwrap();
        sync_after_command(&conn, InstallationId(0), &project, &command).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        assert_eq!(
            loaded.devices.get(DeviceId(1)).unwrap().address,
            Some(IndividualAddress::new(1, 1, 1).unwrap())
        );
    }

    #[test]
    fn create_and_delete_group_address_sync_incrementally() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(knx_core::string_table::Language("en".into()));
        project.installations.push(installation());
        crate::save_project(&conn, &project).unwrap();

        let ga_id = project.ids.next_group_address_id();
        let entry = GroupAddressEntry {
            id: ga_id,
            source: source(),
            name: "New GA".into(),
            address: GroupAddress::from_raw(5),
            central: false,
            unfiltered: false,
            range: None,
        };
        let create = Command::CreateGroupAddress { entry: entry.clone() };
        create.apply(&mut project).unwrap();
        sync_after_command(&conn, InstallationId(0), &project, &create).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        assert_eq!(loaded.installations[0].group_addresses, vec![entry]);

        let delete = Command::DeleteGroupAddress { id: ga_id };
        delete.apply(&mut project).unwrap();
        sync_after_command(&conn, InstallationId(0), &project, &delete).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        assert_eq!(loaded.installations[0].group_addresses, vec![]);
    }
}
```

Add to `crates/knx-store/src/lib.rs`:

```rust
pub mod command_sync;
pub use command_sync::sync_after_command;
```

- [ ] **Step 2: Run, confirm pass**

Run: `cargo test -p knx-store command_sync:: 2>&1 | tail -60`
Expected: PASS. If `Command::CreateGroupAddress { entry }` does not carry a `Clone`-friendly `GroupAddressEntry` (check: `GroupAddressEntry` derives `Clone` already per `group.rs`), no change needed; if `Command` itself is not `Clone` where the test needs it, the test above does not require cloning `Command`, only `entry`, so this should compile as written.

Also confirm the `SetComObjectDpt`/`RestoreComObjectDpt` sync path with one more test appended to the same module:

```rust
    #[test]
    fn set_com_object_dpt_syncs_only_the_dpt_override_row() {
        use knx_core::device::ComObjectInstance;
        use knx_core::dpt::DptRef;
        use knx_core::flags::ResolvedFlags;
        use knx_core::ids::ComObjectInstanceId;
        use knx_core::provenance::Override;

        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_unassigned_device();
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
        });
        project
            .devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));
        crate::save_project(&conn, &project).unwrap();

        let set = Command::SetComObjectDpt {
            com_object: ComObjectInstanceId(1),
            dpt: Some(DptRef { main: 1, sub: Some(1) }),
        };
        let inverse = set.apply(&mut project).unwrap();
        sync_after_command(&conn, InstallationId(0), &project, &set).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        let com = loaded.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(com.dpt.value().is_some());

        // Undo replays through the same mechanism.
        inverse.apply(&mut project).unwrap();
        sync_after_command(&conn, InstallationId(0), &project, &inverse).unwrap();
        let loaded = crate::load_project(&conn).unwrap();
        let com = loaded.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(*com.dpt.value().map(|r| &r.value).map(|_| ()).map_or(&Override::Absent, |_| &com.dpt), com.dpt);
        assert!(!com.dpt.is_present());
    }
```

If that last assertion reads awkwardly once written out (it does — simplify it before running): replace the final three lines with the direct, obvious check:

```rust
        assert_eq!(com.dpt, Override::Absent);
```

- [ ] **Step 3: Run the full workspace test suite**

Run: `cargo test --workspace 2>&1 | tail -60`
Expected: PASS, including every pre-existing `knx-core`/`knx-etsproj`/`knx-productdb`/`knx-app`/`knx-projection` test untouched by this plan.

- [ ] **Step 4: Run the project-wide gates**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- check-layering
cargo deny check
```

Expected: all pass. `check-layering` in particular must still show `knx-store` is not one of the four enforced roots and has gained no dependency beyond `knx-core`/`rusqlite`.

- [ ] **Step 5: Commit**

```bash
git add crates/knx-store/src/command_sync.rs crates/knx-store/src/lib.rs
git commit -m "feat(knx-store): incremental command-sync for the four existing Command variants"
```

---

### Task 13: Documentation

**Files:**
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/ROADMAP.md`
- Modify: `docs/DATA_MODEL.md` (§11 Versioning and migration — schema v4 note)

**Interfaces:** none — documentation only.

- [ ] **Step 1: Update `IMPLEMENTATION_STATUS.md`**

- Add a row/paragraph for `knx-store`'s new entity persistence: schema v4, every entity table, the `Override<T>` codec, `save_project`/`load_project`, `sync_after_command`.
- Update the test count (sum of every `cargo test --workspace` count after this plan — run it and read the real number, do not guess).
- Move the "Full entity persistence... still not part of any session's shipped deliverables" line out of "known gaps carried forward" (it is now shipped) and replace it with the honest remaining gap: no `knx-app`/desktop wiring yet, and only four fields have an incremental sync path.

- [ ] **Step 2: Update `ROADMAP.md`**

Session 5's entry gains a "cycle 2" paragraph (matching cycle 1's own paragraph style) naming what shipped and what's still open (Inspector, search, command palette, dark/light mode, desktop save UX — unchanged from cycle 1's own "not yet scheduled" list, now joined by "knx-app wiring for save/load").

- [ ] **Step 3: Update `DATA_MODEL.md` §11**

Add a Session 5 amendment paragraph parallel to the existing Session 3/Session 4 amendments in that section, naming schema v4 and pointing at the design doc.

- [ ] **Step 4: Commit**

```bash
git add docs/IMPLEMENTATION_STATUS.md docs/ROADMAP.md docs/DATA_MODEL.md
git commit -m "docs(session5): record entity persistence (cycle 2)"
```

---

## Self-Review Notes

- **Spec coverage:** every table in the spec's Schema v4 block has an upsert/load pair (Tasks 3–10); `save_project`/`load_project` (Task 11) and `sync_after_command` (Task 12) match the spec's two named functions exactly; the `PartialEq`/`iter` additions (Task 1) match the spec's two flagged `knx-core` changes; `PRAGMA foreign_keys = ON` is Task 2. Out-of-scope items (Tauri commands, desktop UX, non-command-covered incremental sync, undo-stack persistence) are not tasked, matching the spec's "Out of scope" section.
- **Ordering columns:** every `position`/`flat_position`/`topology_position` column from the spec's amended "owned-list vs flat-list order" note has a task writing and reading it (Tasks 4, 5, 6, 7, 8, 10).
- **Placeholder scan:** no TBD/TODO, no "similar to Task N" without repeated code, no bare "add error handling" steps — every step above carries the actual SQL/Rust.
- **Type consistency:** function names/signatures introduced in one task (`upsert_device`, `upsert_com_object_dpt_override`, `set_device_line`, `upsert_group_address`, `delete_group_address`, `completion_to_str`/`completion_from_str`) are used with matching names and argument order by every later task that calls them (Tasks 11, 12).
