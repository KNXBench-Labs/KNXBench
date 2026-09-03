# Product Database Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `knx-productdb` — a shared, content-hashed SQLite database that stores manufacturer files both as raw blobs and as parsed tables, ingests them during `.knxproj` import, and fills the `Program`/`ProgramRef` layers of the domain model from them.

**Architecture:** `knx-productdb` owns its parser and its store and depends only on `knx-core` (plus `rusqlite`, `quick-xml`, `sha2`). `knx-etsproj` hands manufacturer files out of the container as a separate list instead of burying them in the opaque store; `knx-app` — the only crate that sees every side — routes them into the product database, writes a manifest into the project store, and reassembles them for export. Enrichment is a separate pass over the already-mapped `knx_core::Project`.

**Tech Stack:** Rust 1.98.0, `rusqlite` 0.40 (bundled SQLite), `quick-xml` 0.42, `sha2` 0.10, `tempfile` 3 for tests.

**Spec:** [`docs/superpowers/specs/2026-09-03-product-database-design.md`](../specs/2026-09-03-product-database-design.md)

## Global Constraints

- Rust 1.98.0 (`rust-toolchain.toml`), edition 2021. Never add a dependency that is not already in the workspace `[workspace.dependencies]` table.
- Every SQLite table is declared `STRICT`.
- `knx-productdb` must reach neither `knx-etsproj` nor `knx-store`. Task 1 makes this a `check-layering` rule; every later task must keep it passing.
- `knx-core` performs no IO and reaches none of `serde_json`, `quick-xml`, `rusqlite`, `tokio`. Unchanged by this plan.
- Everything CI runs must pass locally before each commit: `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p xtask -- check-layering`, `cargo deny check`.
- Commits: focused, conventional-commit subject, **no `Co-Authored-By` trailer** (CLAUDE.md), authored as `github@knxbench.com`.
- Documentation and code comments in English. User-facing wording is "KNX-compatible", never "ETS compatible".
- Never silently discard manufacturer data. Anything unparsed survives as the `source_file` blob; anything unrecognized is counted into `ingest_unknown`.
- The product database's `v1` migration is extended in place across Tasks 1–8 of this plan, because no user data exists yet. **After Task 17 lands, `v1` is frozen**: any later change needs a `v2` migration function.
- Numbers quoted in golden tests must be *measured during implementation*, never guessed. Where this plan writes `<measured>`, run the code, read the number, put it in the test, and state it in the commit message.

---

## File Structure

**Created:**

| Path | Responsibility |
| --- | --- |
| `crates/knx-productdb/src/lib.rs` | Public API surface and module wiring |
| `crates/knx-productdb/src/migration.rs` | `products.sqlite` schema and its own migration chain |
| `crates/knx-productdb/src/blob.rs` | `source_file` blob store: hash, insert, load, verify |
| `crates/knx-productdb/src/xml.rs` | Streaming helpers over `quick-xml`: attribute lookup, subtree skip, unknown counting |
| `crates/knx-productdb/src/report.rs` | `IngestReport`, `UnknownConstruct`, `IdConflict`, `BlobMismatch` |
| `crates/knx-productdb/src/parse/catalog.rs` | `Catalog.xml` → `catalog_section`, `catalog_item` |
| `crates/knx-productdb/src/parse/hardware.rs` | `Hardware.xml` → `hardware`, `product`, `hardware2program` |
| `crates/knx-productdb/src/parse/program.rs` | Application program → head, parameter types, parameters, refs |
| `crates/knx-productdb/src/parse/comobject.rs` | `ComObjectTable`/`ComObjectRefs` → `com_object`, `com_object_ref` |
| `crates/knx-productdb/src/parse/translation.rs` | `Languages` → `translation` |
| `crates/knx-productdb/src/parse/master.rs` | `knx_master.xml` → `manufacturer`, `datapoint_type` |
| `crates/knx-productdb/src/ingest.rs` | Per-file orchestration: hash skip, classify, transaction, conflicts |
| `crates/knx-productdb/src/query.rs` | Read-side: `resolve_program`, `com_object_view`, listings |
| `crates/knx-productdb/src/enrich.rs` | `enrich(&mut Project, &Connection)` and `EnrichmentReport` |
| `crates/knx-productdb/tests/golden_reference_products.rs` | Ingest of the committed reference project, measured counts |
| `crates/knx-productdb/tests/malformed_input.rs` | Truncated XML, unknown elements, id conflicts |
| `crates/knx-store/fixtures/v3-empty.sqlite` | Frozen migration fixture |
| `crates/knx-app/tests/product_db.rs` | Wiring, export equality, degradation |
| `docs/adr/0011-product-database-storage.md` | Blobs *and* parsed tables; content hash as identity |
| `docs/adr/0012-enrichment-into-absent-slots.md` | Why enrichment never touches `Empty`/`Malformed`/`Value(Instance)` |

**Modified:**

| Path | Change |
| --- | --- |
| `crates/knx-productdb/Cargo.toml` | Add `rusqlite`, `quick-xml`, `sha2`, dev `tempfile` |
| `xtask/src/main.rs:31-75` | Third layering root |
| `crates/knx-etsproj/src/opaque.rs` | `collect_container_entries` splits manufacturer files out |
| `crates/knx-etsproj/src/lib.rs` | `ManufacturerFile`, `ImportOutcome.manufacturer` |
| `crates/knx-etsproj/src/export/mod.rs:26-40` | Warning variants |
| `crates/knx-store/src/migration.rs` | Schema v3 |
| `crates/knx-store/src/lib.rs`, new `manifest.rs` | Manifest table access |
| `crates/knx-core/src/project.rs:19` | `CURRENT_SCHEMA_VERSION = 3` |
| `crates/knx-app/src/import.rs`, new `export.rs` | Wiring and export assembly |
| `apps/knx-cli/src/main.rs` | `--product-db`, `--no-product-db`, `knx products` |
| `docs/*` | See Task 17 |

---

### Task 1: Crate skeleton, migration chain, layering rule

**Files:**
- Modify: `crates/knx-productdb/Cargo.toml`
- Create: `crates/knx-productdb/src/migration.rs`
- Modify: `crates/knx-productdb/src/lib.rs`
- Modify: `xtask/src/main.rs:31-75`

**Interfaces:**
- Consumes: nothing.
- Produces: `knx_productdb::CURRENT_PRODUCTDB_VERSION: i64`, `knx_productdb::open_and_migrate(&Path) -> Result<Connection, ProductDbError>`, `knx_productdb::ProductDbError`, `knx_productdb::default_path() -> Option<PathBuf>`, and the re-export `knx_productdb::Connection`.

- [ ] **Step 1: Write the failing test**

Append to `crates/knx-productdb/src/migration.rs` (created in this step, test module first — the file will not compile yet, which is the point):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_file_migrates_to_current_version() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_PRODUCTDB_VERSION);
        let marker: String = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'created_by'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(marker, "knx-productdb");
    }

    #[test]
    fn reopening_an_already_migrated_file_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        open_and_migrate(&path).unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_PRODUCTDB_VERSION);
    }

    #[test]
    fn a_file_from_a_newer_version_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", CURRENT_PRODUCTDB_VERSION + 1)
                .unwrap();
        }
        assert!(matches!(
            open_and_migrate(&path),
            Err(ProductDbError::FutureVersion { .. })
        ));
    }

    #[test]
    fn default_path_sits_under_the_xdg_data_directory() {
        // `default_path` reads the environment; assert its shape, not a
        // machine-specific absolute path.
        let p = default_path().expect("HOME or XDG_DATA_HOME is set in CI");
        assert!(p.ends_with("knx/products.sqlite"), "{}", p.display());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-productdb`
Expected: FAIL — `cannot find function open_and_migrate`, `cannot find crate tempfile`.

- [ ] **Step 3: Write minimal implementation**

`crates/knx-productdb/Cargo.toml`:

```toml
[dependencies]
knx-core.workspace = true
rusqlite.workspace = true
quick-xml.workspace = true
sha2.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

`crates/knx-productdb/src/migration.rs` (above the test module):

```rust
//! `products.sqlite`'s own schema and migration chain, keyed off SQLite's
//! `user_version` pragma.
//!
//! Deliberately not `knx-store`'s chain: a project-schema bump must not
//! force a product-database migration, or the other way round (ADR-0005,
//! ADR-0011). The two databases have different lifetimes — a project file
//! is per project, this one is shared across all of them.

use std::fmt;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

/// The product-database schema version this build writes.
pub const CURRENT_PRODUCTDB_VERSION: i64 = 1;

#[derive(Debug)]
pub enum ProductDbError {
    Sqlite(rusqlite::Error),
    Xml { source_path: String, cause: String },
    FutureVersion { found: i64, supported: i64 },
}

impl std::error::Error for ProductDbError {}

impl fmt::Display for ProductDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProductDbError::Sqlite(e) => write!(f, "{e}"),
            ProductDbError::Xml { source_path, cause } => {
                write!(f, "{source_path}: {cause}")
            }
            ProductDbError::FutureVersion { found, supported } => write!(
                f,
                "product database is version {found}, this build supports up to {supported} — no downgrade path exists"
            ),
        }
    }
}

impl From<rusqlite::Error> for ProductDbError {
    fn from(e: rusqlite::Error) -> Self {
        ProductDbError::Sqlite(e)
    }
}

/// v0 -> v1. Extended in place while this plan runs, because the database
/// has no released state yet; frozen once the plan's last task lands.
fn migrate_v0_to_v1(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
         INSERT INTO schema_meta (key, value) VALUES ('created_by', 'knx-productdb');",
    )?;
    Ok(())
}

type Migration = fn(&Connection) -> Result<(), ProductDbError>;

fn migrations() -> Vec<Migration> {
    vec![migrate_v0_to_v1]
}

/// Opens (creating if absent) the product database at `path`, runs every
/// pending migration in order, and returns the connection at
/// `CURRENT_PRODUCTDB_VERSION`. Creates the parent directory, since the
/// default path lives under a data directory the user may not have yet.
pub fn open_and_migrate(path: &Path) -> Result<Connection, ProductDbError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| ProductDbError::Xml {
                source_path: parent.display().to_string(),
                cause: e.to_string(),
            })?;
        }
    }
    let conn = Connection::open(path)?;
    let found: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if found > CURRENT_PRODUCTDB_VERSION {
        return Err(ProductDbError::FutureVersion {
            found,
            supported: CURRENT_PRODUCTDB_VERSION,
        });
    }
    for migration in &migrations()[found as usize..CURRENT_PRODUCTDB_VERSION as usize] {
        migration(&conn)?;
    }
    if found < CURRENT_PRODUCTDB_VERSION {
        conn.pragma_update(None, "user_version", CURRENT_PRODUCTDB_VERSION)?;
    }
    Ok(conn)
}

/// `$XDG_DATA_HOME/knx/products.sqlite`, falling back to
/// `$HOME/.local/share/knx/products.sqlite`. `None` when neither variable
/// is set, which the caller reports rather than guessing a location.
pub fn default_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir).join("knx").join("products.sqlite"));
    }
    let home = std::env::var_os("HOME").filter(|v| !v.is_empty())?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("knx")
            .join("products.sqlite"),
    )
}
```

`crates/knx-productdb/src/lib.rs`:

```rust
//! Product database: manufacturer, hardware, application program and version
//! data in a separate SQLite file shared across projects (ADR-0005).
//!
//! Every ingested file is kept twice: verbatim as a blob keyed by its
//! SHA-256, and as parsed rows. The blob is the integrity guarantee — the
//! parser may not understand a construct, but nothing is ever lost.

pub mod migration;

pub use migration::{
    default_path, open_and_migrate, ProductDbError, CURRENT_PRODUCTDB_VERSION,
};
/// Re-exported so callers name the connection type through this crate
/// rather than depending on `rusqlite` directly.
pub use rusqlite::Connection;
```

`xtask/src/main.rs`, inside `check_layering()` after the `knx-etsproj` block:

```rust
    // knx-productdb owns its own parser and its own store (ADR-0011). It
    // must not reach the project-import crate or the project store: a
    // .knxprod ingest added later must not have to travel through the
    // .knxproj importer, and product data must stay separable from project
    // files (ADR-0005).
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-productdb",
        &["knx-etsproj", "knx-store"],
    ));
```

and extend the success message:

```rust
        println!(
            "layering ok: knx-core reaches none of {:?}; knx-etsproj does not reach knx-store; \
             knx-productdb reaches neither knx-etsproj nor knx-store",
            layering::CORE_FORBIDDEN
        );
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-productdb && cargo run -p xtask -- check-layering`
Expected: 4 tests pass; layering prints the extended "layering ok" line.

- [ ] **Step 5: Prove the new layering rule fails on a violation**

Temporarily add `knx-store.workspace = true` to `crates/knx-productdb/Cargo.toml`, run `cargo run -p xtask -- check-layering`, confirm it reports `knx-productdb reaches forbidden package knx-store`, then revert the Cargo.toml edit and re-run to confirm it passes again. A gate that has never been observed failing is not a gate.

- [ ] **Step 6: Commit**

```bash
git add crates/knx-productdb xtask/src/main.rs
git commit -m "feat(productdb): schema-version migration chain and layering rule"
```

---

### Task 2: The blob store

**Files:**
- Create: `crates/knx-productdb/src/blob.rs`
- Modify: `crates/knx-productdb/src/migration.rs` (extend `migrate_v0_to_v1`)
- Modify: `crates/knx-productdb/src/lib.rs`

**Interfaces:**
- Consumes: `open_and_migrate`, `ProductDbError` (Task 1).
- Produces: `sha256_hex(&[u8]) -> String`, `store_source_file(&Connection, &SourceFile) -> Result<bool, ProductDbError>` (`true` when it was newly stored, `false` when the hash was already present), `has_source_file(&Connection, &str) -> Result<bool, ProductDbError>`, `load_source_file(&Connection, &str) -> Result<Option<Vec<u8>>, ProductDbError>`, `verify(&Connection) -> Result<Vec<BlobMismatch>, ProductDbError>`, `struct SourceFile { source_path: String, manufacturer_id: Option<String>, bytes: Vec<u8> }`, `struct BlobMismatch { sha256: String, source_path: String, actual_sha256: String }`.

- [ ] **Step 1: Write the failing test**

`crates/knx-productdb/src/blob.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    fn file(path: &str, bytes: &[u8]) -> SourceFile {
        SourceFile {
            source_path: path.to_string(),
            manufacturer_id: Some("M-0083".into()),
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn a_blob_round_trips_through_sqlite_unchanged() {
        let (_dir, conn) = db();
        let f = file("M-0083/Catalog.xml", b"<KNX/>");
        let sha = sha256_hex(&f.bytes);
        assert!(store_source_file(&conn, &f).unwrap());
        assert_eq!(load_source_file(&conn, &sha).unwrap(), Some(b"<KNX/>".to_vec()));
    }

    #[test]
    fn storing_the_same_bytes_twice_stores_one_row() {
        let (_dir, conn) = db();
        let f = file("M-0083/Catalog.xml", b"<KNX/>");
        assert!(store_source_file(&conn, &f).unwrap());
        assert!(!store_source_file(&conn, &f).unwrap());
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM source_file", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn the_same_bytes_under_a_different_path_still_store_one_row() {
        // Two projects carry the identical application program at the same
        // path; a third could carry it renamed. Identity is the content
        // hash, never the path (ADR-0011).
        let (_dir, conn) = db();
        assert!(store_source_file(&conn, &file("M-0083/A.xml", b"same")).unwrap());
        assert!(!store_source_file(&conn, &file("M-0083/B.xml", b"same")).unwrap());
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM source_file", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn a_missing_hash_loads_as_none() {
        let (_dir, conn) = db();
        assert_eq!(load_source_file(&conn, "deadbeef").unwrap(), None);
        assert!(!has_source_file(&conn, "deadbeef").unwrap());
    }

    #[test]
    fn verify_reports_a_blob_whose_bytes_no_longer_match_its_key() {
        let (_dir, conn) = db();
        store_source_file(&conn, &file("M-0083/Catalog.xml", b"<KNX/>")).unwrap();
        assert_eq!(verify(&conn).unwrap(), vec![]);
        conn.execute("UPDATE source_file SET bytes = ?1", [&b"tampered"[..]])
            .unwrap();
        let bad = verify(&conn).unwrap();
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].source_path, "M-0083/Catalog.xml");
        assert_eq!(bad[0].actual_sha256, sha256_hex(b"tampered"));
    }

    #[test]
    fn a_binary_blob_survives_byte_for_byte() {
        let (_dir, conn) = db();
        let bytes: Vec<u8> = (0u8..=255).cycle().take(4096).collect();
        let f = file("M-0008/Baggages/econEts3.dll", &bytes);
        let sha = sha256_hex(&bytes);
        store_source_file(&conn, &f).unwrap();
        assert_eq!(load_source_file(&conn, &sha).unwrap(), Some(bytes));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-productdb blob`
Expected: FAIL — `cannot find function store_source_file` and friends.

- [ ] **Step 3: Write minimal implementation**

Extend `migrate_v0_to_v1` in `migration.rs` with:

```sql
CREATE TABLE source_file (
    sha256          TEXT PRIMARY KEY,
    source_path     TEXT NOT NULL,
    manufacturer_id TEXT,
    len             INTEGER NOT NULL,
    bytes           BLOB NOT NULL
) STRICT;
CREATE INDEX source_file_manufacturer ON source_file (manufacturer_id);
```

`crates/knx-productdb/src/blob.rs`:

```rust
//! The `source_file` table: every ingested manufacturer file kept verbatim,
//! keyed by the SHA-256 of its bytes.
//!
//! This is the integrity anchor of the whole crate. The parsers below it
//! model what they understand; anything they do not understand — the
//! `Dynamic` subtree, a vendor's `Legacy*` option, a construct from a
//! manufacturer we have never seen — is still here, byte for byte, and the
//! exporter reads it back from here (IMPORT_EXPORT §10).

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::ProductDbError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub source_path: String,
    pub manufacturer_id: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobMismatch {
    pub sha256: String,
    pub source_path: String,
    pub actual_sha256: String,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Stores `file` unless its hash is already present. Returns whether a row
/// was written, which is what tells the ingest whether it still has to
/// parse the file.
pub fn store_source_file(conn: &Connection, file: &SourceFile) -> Result<bool, ProductDbError> {
    let sha = sha256_hex(&file.bytes);
    if has_source_file(conn, &sha)? {
        return Ok(false);
    }
    conn.execute(
        "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            sha,
            file.source_path,
            file.manufacturer_id,
            file.bytes.len() as i64,
            file.bytes
        ],
    )?;
    Ok(true)
}

pub fn has_source_file(conn: &Connection, sha256: &str) -> Result<bool, ProductDbError> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM source_file WHERE sha256 = ?1",
            [sha256],
            |r| r.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}

pub fn load_source_file(
    conn: &Connection,
    sha256: &str,
) -> Result<Option<Vec<u8>>, ProductDbError> {
    let bytes: Option<Vec<u8>> = conn
        .query_row(
            "SELECT bytes FROM source_file WHERE sha256 = ?1",
            [sha256],
            |r| r.get(0),
        )
        .optional()?;
    Ok(bytes)
}

/// Re-hashes every blob and reports the rows whose bytes no longer match
/// their key. Backs `knx products verify`.
pub fn verify(conn: &Connection) -> Result<Vec<BlobMismatch>, ProductDbError> {
    let mut stmt = conn.prepare("SELECT sha256, source_path, bytes FROM source_file")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Vec<u8>>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (sha256, source_path, bytes) = row?;
        let actual = sha256_hex(&bytes);
        if actual != sha256 {
            out.push(BlobMismatch {
                sha256,
                source_path,
                actual_sha256: actual,
            });
        }
    }
    Ok(out)
}
```

Add to `lib.rs`:

```rust
pub mod blob;

pub use blob::{
    has_source_file, load_source_file, sha256_hex, store_source_file, verify, BlobMismatch,
    SourceFile,
};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-productdb`
Expected: PASS (10 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-productdb
git commit -m "feat(productdb): content-hashed blob store for manufacturer files"
```

---

### Task 3: XML streaming helpers and the ingest report

**Files:**
- Create: `crates/knx-productdb/src/xml.rs`
- Create: `crates/knx-productdb/src/report.rs`
- Modify: `crates/knx-productdb/src/migration.rs` (extend `migrate_v0_to_v1`)
- Modify: `crates/knx-productdb/src/lib.rs`

**Interfaces:**
- Consumes: `ProductDbError` (Task 1).
- Produces: `xml::Attrs` (owned attribute map with `get(&self, name: &str) -> Option<&str>`), `xml::attrs(&BytesStart, source_path: &str) -> Result<Attrs, ProductDbError>`, `xml::local_name(&BytesStart) -> String`, `xml::skip_subtree(&mut Reader<&[u8]>, name: &[u8], source_path: &str) -> Result<(), ProductDbError>`, `report::UnknownConstruct { source_sha256, program_id, xpath, kind, name, occurrences, sample }`, `report::UnknownKind { Element, Attribute }`, `report::IdConflict { table, id, kept_sha256, other_sha256 }`, `report::UnknownCollector` with `element(&mut self, xpath, name)`, `attribute(&mut self, xpath, name, sample)` and `into_vec(self)`, `report::insert_unknown(&Connection, source_sha256, &[UnknownConstruct]) -> Result<(), ProductDbError>`, and `report::insert_conflicts(&Connection, &[IdConflict]) -> Result<(), ProductDbError>`, which writes each conflict as an `ingest_unknown` row with `kind = 'IdConflict'`.

- [ ] **Step 1: Write the failing test**

`crates/knx-productdb/src/xml.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::events::Event;
    use quick_xml::Reader;

    #[test]
    fn attributes_are_read_by_local_name_regardless_of_namespace_prefix() {
        let mut r = Reader::from_reader(
            br#"<Hardware Id="M-006A_H-1" SerialNumber="EM12102" BusCurrent="1.5e+001"/>"#
                .as_ref(),
        );
        let mut buf = Vec::new();
        let start = match r.read_event_into(&mut buf).unwrap() {
            Event::Empty(e) => e,
            other => panic!("expected an empty element, got {other:?}"),
        };
        let a = attrs(&start, "t.xml").unwrap();
        assert_eq!(a.get("Id"), Some("M-006A_H-1"));
        assert_eq!(a.get("SerialNumber"), Some("EM12102"));
        assert_eq!(a.get("Missing"), None);
    }

    #[test]
    fn a_skipped_subtree_leaves_the_reader_after_its_end_tag() {
        let xml = br#"<Root><Dynamic><Channel><Block/></Channel></Dynamic><After Id="x"/></Root>"#;
        let mut r = Reader::from_reader(xml.as_ref());
        let mut buf = Vec::new();
        // <Root>
        r.read_event_into(&mut buf).unwrap();
        buf.clear();
        // <Dynamic>
        match r.read_event_into(&mut buf).unwrap() {
            Event::Start(e) => skip_subtree(&mut r, e.name().as_ref(), "t.xml").unwrap(),
            other => panic!("expected <Dynamic>, got {other:?}"),
        }
        buf.clear();
        match r.read_event_into(&mut buf).unwrap() {
            Event::Empty(e) => assert_eq!(local_name(&e), "After"),
            other => panic!("expected <After/>, got {other:?}"),
        }
    }

    #[test]
    fn a_nested_element_of_the_same_name_does_not_end_the_skip_early() {
        let xml = br#"<Root><choose><choose><when/></choose></choose><After Id="x"/></Root>"#;
        let mut r = Reader::from_reader(xml.as_ref());
        let mut buf = Vec::new();
        r.read_event_into(&mut buf).unwrap();
        buf.clear();
        match r.read_event_into(&mut buf).unwrap() {
            Event::Start(e) => skip_subtree(&mut r, e.name().as_ref(), "t.xml").unwrap(),
            other => panic!("expected <choose>, got {other:?}"),
        }
        buf.clear();
        match r.read_event_into(&mut buf).unwrap() {
            Event::Empty(e) => assert_eq!(local_name(&e), "After"),
            other => panic!("expected <After/>, got {other:?}"),
        }
    }

    #[test]
    fn truncated_xml_is_an_error_naming_its_source_path() {
        let mut r = Reader::from_reader(br#"<Root><Dynamic>"#.as_ref());
        let mut buf = Vec::new();
        r.read_event_into(&mut buf).unwrap();
        buf.clear();
        let e = match r.read_event_into(&mut buf).unwrap() {
            Event::Start(e) => e,
            other => panic!("expected <Dynamic>, got {other:?}"),
        };
        let err = skip_subtree(&mut r, e.name().as_ref(), "M-0083/A.xml").unwrap_err();
        assert!(format!("{err}").contains("M-0083/A.xml"));
    }
}
```

`crates/knx-productdb/src/report.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    #[test]
    fn repeated_unknowns_collapse_into_one_counted_row() {
        let mut c = UnknownCollector::default();
        c.attribute("/KNX/ManufacturerData/Manufacturer/Hardware", "Fancy", "1");
        c.attribute("/KNX/ManufacturerData/Manufacturer/Hardware", "Fancy", "2");
        c.element("/KNX/ManufacturerData/Manufacturer", "Extension");
        let v = c.into_vec();
        assert_eq!(v.len(), 2);
        let fancy = v.iter().find(|u| u.name == "Fancy").unwrap();
        assert_eq!(fancy.occurrences, 2);
        assert_eq!(fancy.kind, UnknownKind::Attribute);
        // The first sample is kept, so the value is from the first sighting.
        assert_eq!(fancy.sample.as_deref(), Some("1"));
    }

    #[test]
    fn unknown_rows_persist_with_their_source_hash() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let mut c = UnknownCollector::default();
        c.element("/KNX/ManufacturerData", "Whatsit");
        insert_unknown(&conn, "abc123", &c.into_vec()).unwrap();
        let (name, occurrences): (String, i64) = conn
            .query_row(
                "SELECT name, occurrences FROM ingest_unknown WHERE source_sha256 = 'abc123'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, "Whatsit");
        assert_eq!(occurrences, 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-productdb`
Expected: FAIL — modules `xml` and `report` do not exist.

- [ ] **Step 3: Write minimal implementation**

Extend `migrate_v0_to_v1`:

```sql
CREATE TABLE ingest_unknown (
    id            INTEGER PRIMARY KEY,
    source_sha256 TEXT NOT NULL,
    program_id    TEXT,
    xpath         TEXT NOT NULL,
    kind          TEXT NOT NULL,
    name          TEXT NOT NULL,
    occurrences   INTEGER NOT NULL,
    sample        TEXT
) STRICT;
CREATE INDEX ingest_unknown_source ON ingest_unknown (source_sha256);
```

`crates/knx-productdb/src/xml.rs`:

```rust
//! Streaming helpers over `quick-xml`, shared by every parser in this crate.
//!
//! Deliberately small: this crate stores each file whole as a blob, so its
//! parsers do not need `knx-etsproj`'s retained-fragment machinery — the
//! bytes are the retention mechanism (ADR-0011). What is needed is exactly
//! attribute lookup, subtree skipping (the `Dynamic` tree, RESEARCH §4.1),
//! and errors that name the file they came from.

use std::collections::BTreeMap;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::ProductDbError;

/// One element's attributes, owned and decoded, keyed by local name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attrs(BTreeMap<String, String>);

impl Attrs {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// Every attribute name present, so a parser can report the ones it
    /// does not know instead of dropping them.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

pub fn local_name(e: &BytesStart) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}

pub fn attrs(e: &BytesStart, source_path: &str) -> Result<Attrs, ProductDbError> {
    let mut map = BTreeMap::new();
    for attr in e.attributes() {
        let attr = attr.map_err(|err| ProductDbError::Xml {
            source_path: source_path.to_string(),
            cause: err.to_string(),
        })?;
        let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).into_owned();
        let value = attr
            .unescape_value()
            .map_err(|err| ProductDbError::Xml {
                source_path: source_path.to_string(),
                cause: err.to_string(),
            })?
            .into_owned();
        map.insert(key, value);
    }
    Ok(Attrs(map))
}

/// Consumes events until the element named `name` closes, counting nested
/// elements of the same name so a `<choose>` inside a `<choose>` does not
/// end the skip early.
pub fn skip_subtree(
    reader: &mut Reader<&[u8]>,
    name: &[u8],
    source_path: &str,
) -> Result<(), ProductDbError> {
    let mut depth = 1usize;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == name => depth += 1,
            Ok(Event::End(e)) if e.name().as_ref() == name => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            Ok(Event::Eof) => {
                return Err(ProductDbError::Xml {
                    source_path: source_path.to_string(),
                    cause: format!(
                        "unexpected end of file while skipping <{}>",
                        String::from_utf8_lossy(name)
                    ),
                })
            }
            Ok(_) => {}
            Err(e) => {
                return Err(ProductDbError::Xml {
                    source_path: source_path.to_string(),
                    cause: e.to_string(),
                })
            }
        }
    }
}
```

`crates/knx-productdb/src/report.rs`:

```rust
//! What an ingest has to say for itself: unknown constructs, id conflicts,
//! and the counts a caller prints.
//!
//! Same rule as `knx-etsproj`'s `ImportReport`: an unrecognized element or
//! attribute is counted and reported, never silently dropped and never
//! fatal on its own (CLAUDE.md's data-integrity rule).

use std::collections::BTreeMap;

use rusqlite::{params, Connection};

use crate::ProductDbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownKind {
    Element,
    Attribute,
}

impl UnknownKind {
    fn as_str(self) -> &'static str {
        match self {
            UnknownKind::Element => "Element",
            UnknownKind::Attribute => "Attribute",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownConstruct {
    pub xpath: String,
    pub kind: UnknownKind,
    pub name: String,
    pub occurrences: u32,
    pub sample: Option<String>,
}

/// An id that appeared in two files with different content hashes. The
/// first ingest's rows are kept; this records that the second existed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdConflict {
    pub table: String,
    pub id: String,
    pub kept_sha256: String,
    pub other_sha256: String,
}

#[derive(Debug, Clone, Default)]
pub struct UnknownCollector {
    seen: BTreeMap<(String, &'static str, String), UnknownConstruct>,
}

impl UnknownCollector {
    pub fn element(&mut self, xpath: &str, name: &str) {
        self.record(xpath, UnknownKind::Element, name, None);
    }

    pub fn attribute(&mut self, xpath: &str, name: &str, sample: &str) {
        self.record(xpath, UnknownKind::Attribute, name, Some(sample));
    }

    fn record(&mut self, xpath: &str, kind: UnknownKind, name: &str, sample: Option<&str>) {
        let key = (xpath.to_string(), kind.as_str(), name.to_string());
        self.seen
            .entry(key)
            .and_modify(|u| u.occurrences += 1)
            .or_insert_with(|| UnknownConstruct {
                xpath: xpath.to_string(),
                kind,
                name: name.to_string(),
                occurrences: 1,
                sample: sample.map(str::to_string),
            });
    }

    pub fn into_vec(self) -> Vec<UnknownConstruct> {
        self.seen.into_values().collect()
    }
}

pub fn insert_unknown(
    conn: &Connection,
    source_sha256: &str,
    unknown: &[UnknownConstruct],
) -> Result<(), ProductDbError> {
    let mut stmt = conn.prepare(
        "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for u in unknown {
        stmt.execute(params![
            source_sha256,
            Option::<String>::None,
            u.xpath,
            u.kind.as_str(),
            u.name,
            u.occurrences as i64,
            u.sample,
        ])?;
    }
    Ok(())
}

/// Records an id collision as an `ingest_unknown` row with
/// `kind = 'IdConflict'`, so both hashes and the winning row stay on record
/// without a table of its own (spec §5).
pub fn insert_conflicts(
    conn: &Connection,
    conflicts: &[IdConflict],
) -> Result<(), ProductDbError> {
    let mut stmt = conn.prepare(
        "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
         VALUES (?1, NULL, ?2, 'IdConflict', ?3, 1, ?4)",
    )?;
    for c in conflicts {
        stmt.execute(params![c.kept_sha256, c.table, c.id, c.other_sha256])?;
    }
    Ok(())
}
```

Add both modules and their re-exports to `lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-productdb`
Expected: PASS (16 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-productdb
git commit -m "feat(productdb): xml streaming helpers and the ingest report"
```

---

### Task 4: Catalog and hardware

**Files:**
- Create: `crates/knx-productdb/src/parse/mod.rs`
- Create: `crates/knx-productdb/src/parse/catalog.rs`
- Create: `crates/knx-productdb/src/parse/hardware.rs`
- Modify: `crates/knx-productdb/src/migration.rs` (extend `migrate_v0_to_v1`)
- Modify: `crates/knx-productdb/src/lib.rs`

**Interfaces:**
- Consumes: `xml::{attrs, local_name, skip_subtree}`, `report::UnknownCollector`, `ProductDbError`.
- Produces: `parse::catalog::ingest_catalog(&Connection, source_sha256: &str, source_path: &str, bytes: &[u8]) -> Result<Vec<UnknownConstruct>, ProductDbError>` and `parse::hardware::ingest_hardware(&Connection, source_sha256: &str, source_path: &str, bytes: &[u8]) -> Result<Vec<UnknownConstruct>, ProductDbError>`. Both are idempotent per id via `INSERT OR IGNORE` plus a conflict check.

- [ ] **Step 1: Write the failing test**

`crates/knx-productdb/src/parse/hardware.rs` test module (catalog gets the mirror-image test; write both):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    const HARDWARE: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <Hardware>
        <Hardware Id="M-006A_H-EM12102-6-O0079" Name="Präsenzmelder" SerialNumber="EM12102"
                  VersionNumber="6" BusCurrent="1.5000000e+001" HasIndividualAddress="1"
                  HasApplicationProgram="1" IsPowerSupply="0" IsCoupler="0" IsIPEnabled="0"
                  OriginalManufacturer="M-0079">
          <Products>
            <Product Id="M-006A_H-EM12102-6-O0079_P-N000520" Text="Präsenzmelder"
                     OrderNumber="N000520" IsRailMounted="0" DefaultLanguage="de-DE"
                     Hash="x2MHcIV+s36yBfIVtjnD38UVuOo=">
              <RegistrationInfo RegistrationStatus="Unregistered" />
            </Product>
          </Products>
          <Hardware2Programs>
            <Hardware2Program Id="M-006A_H-EM12102-6-O0079_HP-0001-22-26C0-O0079"
                              MediumTypes="MT-0" Hash="VD7KKaEG0iE5BAyQVg4gHPpNxaQ=">
              <ApplicationProgramRef RefId="M-006A_A-0001-22-26C0-O0079" />
              <RegistrationInfo RegistrationNumber="0001/22" RegistrationStatus="Registered" />
            </Hardware2Program>
          </Hardware2Programs>
        </Hardware>
      </Hardware>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn hardware_product_and_program_link_are_stored() {
        let (_dir, conn) = db();
        ingest_hardware(&conn, "sha-1", "M-006A/Hardware.xml", HARDWARE).unwrap();

        let (name, serial, bus_current, coupler): (String, String, String, i64) = conn
            .query_row(
                "SELECT name, serial_number, bus_current, is_coupler FROM hardware WHERE id = ?1",
                ["M-006A_H-EM12102-6-O0079"],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(name, "Präsenzmelder");
        assert_eq!(serial, "EM12102");
        // Stored verbatim: the source writes it in scientific notation and
        // we do not reinterpret manufacturer values (CLAUDE.md).
        assert_eq!(bus_current, "1.5000000e+001");
        assert_eq!(coupler, 0);

        let order: String = conn
            .query_row(
                "SELECT order_number FROM product WHERE hardware_id = ?1",
                ["M-006A_H-EM12102-6-O0079"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(order, "N000520");

        let (program_ref, media, status): (String, String, String) = conn
            .query_row(
                "SELECT application_program_ref, medium_types, registration_status
                 FROM hardware2program WHERE id = ?1",
                ["M-006A_H-EM12102-6-O0079_HP-0001-22-26C0-O0079"],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(program_ref, "M-006A_A-0001-22-26C0-O0079");
        assert_eq!(media, "MT-0");
        assert_eq!(status, "Registered");
    }

    #[test]
    fn the_manufacturer_row_is_created_from_the_ref_id() {
        let (_dir, conn) = db();
        ingest_hardware(&conn, "sha-1", "M-006A/Hardware.xml", HARDWARE).unwrap();
        let id: String = conn
            .query_row("SELECT id FROM manufacturer", [], |r| r.get(0))
            .unwrap();
        assert_eq!(id, "M-006A");
    }

    #[test]
    fn ingesting_the_same_hardware_twice_does_not_duplicate_rows() {
        let (_dir, conn) = db();
        ingest_hardware(&conn, "sha-1", "M-006A/Hardware.xml", HARDWARE).unwrap();
        ingest_hardware(&conn, "sha-1", "M-006A/Hardware.xml", HARDWARE).unwrap();
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM hardware", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn an_unknown_attribute_is_reported_and_the_rest_still_lands() {
        let (_dir, conn) = db();
        let xml = String::from_utf8(HARDWARE.to_vec())
            .unwrap()
            .replace("IsCoupler=\"0\"", "IsCoupler=\"0\" FancyNewFlag=\"7\"");
        let unknown = ingest_hardware(&conn, "sha-2", "M-006A/Hardware.xml", xml.as_bytes()).unwrap();
        assert!(unknown.iter().any(|u| u.name == "FancyNewFlag"));
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM hardware", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn truncated_hardware_xml_is_an_error_naming_the_file() {
        let (_dir, conn) = db();
        let truncated = &HARDWARE[..HARDWARE.len() / 2];
        let err = ingest_hardware(&conn, "sha-3", "M-006A/Hardware.xml", truncated).unwrap_err();
        assert!(format!("{err}").contains("M-006A/Hardware.xml"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-productdb hardware`
Expected: FAIL — `ingest_hardware` does not exist.

- [ ] **Step 3: Write minimal implementation**

Extend `migrate_v0_to_v1`:

```sql
CREATE TABLE manufacturer (
    id   TEXT PRIMARY KEY,
    name TEXT
) STRICT;

CREATE TABLE catalog_section (
    id                  TEXT PRIMARY KEY,
    manufacturer_id     TEXT NOT NULL,
    parent_id           TEXT,
    name                TEXT,
    number              TEXT,
    visible_description TEXT,
    default_language    TEXT,
    source_sha256       TEXT NOT NULL
) STRICT;

CREATE TABLE catalog_item (
    id                       TEXT PRIMARY KEY,
    manufacturer_id          TEXT NOT NULL,
    section_id               TEXT NOT NULL,
    name                     TEXT,
    number                   TEXT,
    visible_description      TEXT,
    product_ref_id           TEXT,
    hardware2program_ref_id  TEXT,
    default_language         TEXT,
    source_sha256            TEXT NOT NULL
) STRICT;
CREATE INDEX catalog_item_section ON catalog_item (section_id);

CREATE TABLE hardware (
    id                      TEXT PRIMARY KEY,
    manufacturer_id         TEXT NOT NULL,
    name                    TEXT,
    serial_number           TEXT,
    version_number          TEXT,
    bus_current             TEXT,
    has_individual_address  INTEGER,
    has_application_program INTEGER,
    is_accessory            INTEGER,
    is_coupler              INTEGER,
    is_power_supply         INTEGER,
    is_ip_enabled           INTEGER,
    is_power_line_repeater  INTEGER,
    original_manufacturer   TEXT,
    source_sha256           TEXT NOT NULL
) STRICT;

CREATE TABLE product (
    id                    TEXT PRIMARY KEY,
    manufacturer_id       TEXT NOT NULL,
    hardware_id           TEXT NOT NULL,
    text                  TEXT,
    order_number          TEXT,
    is_rail_mounted       INTEGER,
    width_in_millimeter   TEXT,
    default_language      TEXT,
    hash                  TEXT,
    registration_status   TEXT,
    source_sha256         TEXT NOT NULL
) STRICT;
CREATE INDEX product_hardware ON product (hardware_id);

CREATE TABLE hardware2program (
    id                      TEXT PRIMARY KEY,
    manufacturer_id         TEXT NOT NULL,
    hardware_id             TEXT NOT NULL,
    application_program_ref TEXT,
    medium_types            TEXT,
    hash                    TEXT,
    registration_number     TEXT,
    registration_status     TEXT,
    registration_signature  TEXT,
    source_sha256           TEXT NOT NULL
) STRICT;
CREATE INDEX hardware2program_program ON hardware2program (application_program_ref);
```

`crates/knx-productdb/src/parse/mod.rs`:

```rust
//! The parsers, one module per manufacturer file kind. Each takes the
//! file's bytes and its content hash, writes rows, and returns the
//! constructs it did not recognize. None of them fails the ingest because
//! of one unknown element or attribute.

pub mod catalog;
pub mod hardware;

use crate::report::UnknownCollector;
use crate::xml::Attrs;

/// Reports every attribute on `a` that is not in `known`, so an unmodelled
/// manufacturer attribute is visible rather than lost.
pub(crate) fn report_unknown_attrs(
    collector: &mut UnknownCollector,
    xpath: &str,
    a: &Attrs,
    known: &[&str],
) {
    for name in a.names() {
        if !known.contains(&name) {
            let sample = a.get(name).unwrap_or_default();
            collector.attribute(xpath, name, sample);
        }
    }
}

/// `"1"`/`"0"` as SQLite integers; anything else stays `None` rather than
/// being guessed into a boolean.
pub(crate) fn bool_flag(a: &Attrs, name: &str) -> Option<i64> {
    match a.get(name) {
        Some("1") => Some(1),
        Some("0") => Some(0),
        _ => None,
    }
}
```

`crates/knx-productdb/src/parse/hardware.rs` — stream the document, tracking the current `Hardware` id so `Product` and `Hardware2Program` rows can reference it:

```rust
//! `Hardware.xml`: `Hardware` → `Products`/`Product` and
//! `Hardware2Programs`/`Hardware2Program` (RESEARCH §4).
//!
//! `DeviceInstance.product_ref` resolves into `product`, and
//! `DeviceInstance.program_ref` into `hardware2program`, whose
//! `application_program_ref` is the bridge to the application program.

use quick_xml::events::Event;
use quick_xml::Reader;
use rusqlite::{params, Connection};

use super::{bool_flag, report_unknown_attrs};
use crate::report::{UnknownCollector, UnknownConstruct};
use crate::xml::{attrs, local_name};
use crate::ProductDbError;

const HARDWARE_ATTRS: &[&str] = &[
    "Id", "Name", "SerialNumber", "VersionNumber", "BusCurrent", "HasIndividualAddress",
    "HasApplicationProgram", "HasApplicationProgram2", "IsAccessory", "IsCoupler",
    "IsPowerSupply", "IsIPEnabled", "IsPowerLineRepeater", "IsPowerLineSignalFilter",
    "IsChoke", "IsCable", "OriginalManufacturer", "NonRegRelevantDataVersion",
];

const PRODUCT_ATTRS: &[&str] = &[
    "Id", "Text", "OrderNumber", "IsRailMounted", "WidthInMillimeter", "DefaultLanguage",
    "Hash", "NonRegRelevantDataVersion", "VisibleDescription",
];

const H2P_ATTRS: &[&str] = &["Id", "MediumTypes", "Hash", "NonRegRelevantDataVersion"];

pub fn ingest_hardware(
    conn: &Connection,
    source_sha256: &str,
    source_path: &str,
    bytes: &[u8],
) -> Result<Vec<UnknownConstruct>, ProductDbError> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut unknown = UnknownCollector::default();
    let mut manufacturer_id = String::new();
    let mut hardware_id = String::new();
    let mut h2p_id = String::new();

    loop {
        buf.clear();
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| ProductDbError::Xml {
                source_path: source_path.to_string(),
                cause: e.to_string(),
            })?;
        match event {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) => {
                let name = local_name(&e);
                let a = attrs(&e, source_path)?;
                match name.as_str() {
                    "Manufacturer" => {
                        manufacturer_id = a.get("RefId").unwrap_or_default().to_string();
                        conn.execute(
                            "INSERT OR IGNORE INTO manufacturer (id, name) VALUES (?1, NULL)",
                            [&manufacturer_id],
                        )?;
                    }
                    // The outer <Hardware> is the collection, the inner one
                    // the entity: only the one carrying an @Id is a device.
                    "Hardware" if a.get("Id").is_some() => {
                        hardware_id = a.get("Id").unwrap_or_default().to_string();
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/ManufacturerData/Manufacturer/Hardware/Hardware",
                            &a,
                            HARDWARE_ATTRS,
                        );
                        conn.execute(
                            "INSERT OR IGNORE INTO hardware
                             (id, manufacturer_id, name, serial_number, version_number,
                              bus_current, has_individual_address, has_application_program,
                              is_accessory, is_coupler, is_power_supply, is_ip_enabled,
                              is_power_line_repeater, original_manufacturer, source_sha256)
                             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                            params![
                                hardware_id,
                                manufacturer_id,
                                a.get("Name"),
                                a.get("SerialNumber"),
                                a.get("VersionNumber"),
                                a.get("BusCurrent"),
                                bool_flag(&a, "HasIndividualAddress"),
                                bool_flag(&a, "HasApplicationProgram"),
                                bool_flag(&a, "IsAccessory"),
                                bool_flag(&a, "IsCoupler"),
                                bool_flag(&a, "IsPowerSupply"),
                                bool_flag(&a, "IsIPEnabled"),
                                bool_flag(&a, "IsPowerLineRepeater"),
                                a.get("OriginalManufacturer"),
                                source_sha256,
                            ],
                        )?;
                    }
                    "Product" => {
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/ManufacturerData/Manufacturer/Hardware/Hardware/Products/Product",
                            &a,
                            PRODUCT_ATTRS,
                        );
                        conn.execute(
                            "INSERT OR IGNORE INTO product
                             (id, manufacturer_id, hardware_id, text, order_number,
                              is_rail_mounted, width_in_millimeter, default_language, hash,
                              registration_status, source_sha256)
                             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,NULL,?10)",
                            params![
                                a.get("Id"),
                                manufacturer_id,
                                hardware_id,
                                a.get("Text"),
                                a.get("OrderNumber"),
                                bool_flag(&a, "IsRailMounted"),
                                a.get("WidthInMillimeter"),
                                a.get("DefaultLanguage"),
                                a.get("Hash"),
                                source_sha256,
                            ],
                        )?;
                    }
                    "Hardware2Program" => {
                        h2p_id = a.get("Id").unwrap_or_default().to_string();
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/ManufacturerData/Manufacturer/Hardware/Hardware/Hardware2Programs/Hardware2Program",
                            &a,
                            H2P_ATTRS,
                        );
                        conn.execute(
                            "INSERT OR IGNORE INTO hardware2program
                             (id, manufacturer_id, hardware_id, application_program_ref,
                              medium_types, hash, registration_number, registration_status,
                              registration_signature, source_sha256)
                             VALUES (?1,?2,?3,NULL,?4,?5,NULL,NULL,NULL,?6)",
                            params![
                                h2p_id,
                                manufacturer_id,
                                hardware_id,
                                a.get("MediumTypes"),
                                a.get("Hash"),
                                source_sha256,
                            ],
                        )?;
                    }
                    "ApplicationProgramRef" => {
                        conn.execute(
                            "UPDATE hardware2program SET application_program_ref = ?1 WHERE id = ?2",
                            params![a.get("RefId"), h2p_id],
                        )?;
                    }
                    // RegistrationInfo appears under both Product and
                    // Hardware2Program; the last id seen decides which.
                    "RegistrationInfo" => {
                        conn.execute(
                            "UPDATE hardware2program
                             SET registration_number = ?1, registration_status = ?2,
                                 registration_signature = ?3
                             WHERE id = ?4",
                            params![
                                a.get("RegistrationNumber"),
                                a.get("RegistrationStatus"),
                                a.get("RegistrationSignature"),
                                h2p_id,
                            ],
                        )?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    Ok(unknown.into_vec())
}
```

Write `catalog.rs` the same way: track a stack of `CatalogSection` ids for `parent_id`, insert `catalog_item` rows with `product_ref_id` and `hardware2program_ref_id`, report unknown attributes against
`&["Id", "Name", "Number", "VisibleDescription", "DefaultLanguage", "NonRegRelevantDataVersion"]` for sections and that list plus `&["ProductRefId", "Hardware2ProgramRefId"]` for items. Its tests mirror the hardware tests: one nested section is stored with the right `parent_id`, an item resolves to its product and program refs, a second ingest adds no rows.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-productdb`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/knx-productdb
git commit -m "feat(productdb): ingest catalog and hardware data"
```

---

### Task 5: Application program head, parameter types and parameters

**Files:**
- Create: `crates/knx-productdb/src/parse/program.rs`
- Modify: `crates/knx-productdb/src/migration.rs`, `crates/knx-productdb/src/parse/mod.rs`

**Interfaces:**
- Consumes: Task 3 and 4 helpers.
- Produces: `parse::program::ingest_program(&Connection, source_sha256: &str, source_path: &str, bytes: &[u8]) -> Result<ProgramIngest, ProductDbError>` with `pub struct ProgramIngest { pub program_id: String, pub unknown: Vec<UnknownConstruct>, pub conflicts: Vec<IdConflict> }`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    const PROGRAM: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <ApplicationPrograms>
        <ApplicationProgram Id="M-006A_A-0001-22-26C0-O0079" Name="Presence" ApplicationNumber="1"
                            ApplicationVersion="22" ProgramType="ApplicationProgram"
                            MaskVersion="MV-0701" PeiType="0" LoadProcedureStyle="ProductProcedure"
                            DefaultLanguage="de-DE" Hash="Dyd1CfXJEmqKsHKWZeApTA==">
          <Static>
            <ParameterTypes>
              <ParameterType Id="PT-Base" Name="baseOfSignal">
                <TypeRestriction Base="Value" SizeInBit="8">
                  <Enumeration Id="PT-Base_EN-0" Text="0.0 V" Value="0" />
                  <Enumeration Id="PT-Base_EN-5" Text="0.5 V" Value="5" DisplayOrder="1" />
                </TypeRestriction>
              </ParameterType>
              <ParameterType Id="PT-Num" Name="delay">
                <TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" />
              </ParameterType>
            </ParameterTypes>
            <Parameters>
              <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num"
                         Access="ReadWrite" Value="7">
                <Memory CodeSegment="AS-40F4" Offset="116" BitOffset="0" />
              </Parameter>
              <Union SizeInBit="8">
                <Memory CodeSegment="AS-40F4" Offset="160" BitOffset="0" />
                <Parameter Id="P-2" Name="Mode" Text="Mode" ParameterType="PT-Base"
                           Access="ReadWrite" Value="0" />
              </Union>
            </Parameters>
            <ParameterRefs>
              <ParameterRef Id="P-1_R-1" RefId="P-1" DisplayOrder="1000" Tag="1" />
            </ParameterRefs>
          </Static>
          <Dynamic>
            <Channel Id="CH-1">
              <choose ParamRefId="P-1_R-1">
                <when test="1"><ParameterRefRef RefId="P-1_R-1" /></when>
              </choose>
            </Channel>
          </Dynamic>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn the_program_head_is_stored() {
        let (_dir, conn) = db();
        let out = ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        assert_eq!(out.program_id, "M-006A_A-0001-22-26C0-O0079");
        let (number, version, mask): (String, String, String) = conn
            .query_row(
                "SELECT application_number, application_version, mask_version
                 FROM application_program WHERE id = ?1",
                ["M-006A_A-0001-22-26C0-O0079"],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((number.as_str(), version.as_str(), mask.as_str()), ("1", "22", "MV-0701"));
    }

    #[test]
    fn a_restriction_type_stores_its_enumeration_values() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let kind: String = conn
            .query_row(
                "SELECT kind FROM parameter_type WHERE id = 'PT-Base'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kind, "Restriction");
        let values: Vec<(String, String)> = conn
            .prepare("SELECT value, text FROM parameter_type_enum WHERE parameter_type_id = 'PT-Base' ORDER BY value")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            values,
            vec![("0".to_string(), "0.0 V".to_string()), ("5".to_string(), "0.5 V".to_string())]
        );
    }

    #[test]
    fn a_number_type_stores_its_bounds_and_size() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let (kind, min, max, size): (String, String, String, i64) = conn
            .query_row(
                "SELECT kind, min_inclusive, max_inclusive, size_in_bit
                 FROM parameter_type WHERE id = 'PT-Num'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!((kind.as_str(), min.as_str(), max.as_str(), size), ("Number", "0", "255", 8));
    }

    #[test]
    fn a_parameter_stores_its_memory_layout() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let (segment, offset, bit): (String, i64, i64) = conn
            .query_row(
                "SELECT code_segment, offset, bit_offset FROM parameter WHERE id = 'P-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((segment.as_str(), offset, bit), ("AS-40F4", 116, 0));
    }

    #[test]
    fn a_union_member_carries_the_unions_size_and_memory() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let (union_size, segment, offset): (i64, String, i64) = conn
            .query_row(
                "SELECT union_size_in_bit, code_segment, offset FROM parameter WHERE id = 'P-2'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((union_size, segment.as_str(), offset), (8, "AS-40F4", 160));
    }

    #[test]
    fn a_parameter_ref_points_back_at_its_parameter() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let (param, order): (String, i64) = conn
            .query_row(
                "SELECT parameter_id, display_order FROM parameter_ref WHERE id = 'P-1_R-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((param.as_str(), order), ("P-1", 1000));
    }

    #[test]
    fn the_dynamic_subtree_is_skipped_and_stores_nothing() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        // Nothing from <Dynamic> reaches any table: no channel, no choose,
        // no when. It survives as the source_file blob instead (ADR-0011).
        let params: i64 = conn
            .query_row("SELECT count(*) FROM parameter", [], |r| r.get(0))
            .unwrap();
        assert_eq!(params, 2);
        let refs: i64 = conn
            .query_row("SELECT count(*) FROM parameter_ref", [], |r| r.get(0))
            .unwrap();
        assert_eq!(refs, 1);
    }

    #[test]
    fn the_same_id_from_a_different_hash_is_a_recorded_conflict_not_an_overwrite() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let changed = String::from_utf8(PROGRAM.to_vec())
            .unwrap()
            .replace("Name=\"Presence\"", "Name=\"Presence v2\"");
        let out = ingest_program(&conn, "sha-2", "M-006A/A.xml", changed.as_bytes()).unwrap();
        let name: String = conn
            .query_row(
                "SELECT name FROM application_program WHERE id = ?1",
                ["M-006A_A-0001-22-26C0-O0079"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "Presence", "the first ingest's rows win");
        assert!(out
            .conflicts
            .iter()
            .any(|c| c.id == "M-006A_A-0001-22-26C0-O0079" && c.other_sha256 == "sha-2"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-productdb program`
Expected: FAIL — `ingest_program` does not exist.

- [ ] **Step 3: Write minimal implementation**

Extend `migrate_v0_to_v1`:

```sql
CREATE TABLE application_program (
    id                  TEXT PRIMARY KEY,
    manufacturer_id     TEXT NOT NULL,
    name                TEXT,
    application_number  TEXT,
    application_version TEXT,
    program_type        TEXT,
    mask_version        TEXT,
    pei_type            TEXT,
    load_procedure_style TEXT,
    default_language    TEXT,
    hash                TEXT,
    linkable            INTEGER,
    original_manufacturer TEXT,
    source_sha256       TEXT NOT NULL
) STRICT;

CREATE TABLE parameter_type (
    program_id     TEXT NOT NULL,
    id             TEXT NOT NULL,
    name           TEXT,
    kind           TEXT NOT NULL,
    size_in_bit    INTEGER,
    base           TEXT,
    min_inclusive  TEXT,
    max_inclusive  TEXT,
    number_type    TEXT,
    PRIMARY KEY (program_id, id)
) STRICT;

CREATE TABLE parameter_type_enum (
    program_id        TEXT NOT NULL,
    parameter_type_id TEXT NOT NULL,
    id                TEXT NOT NULL,
    value             TEXT,
    text              TEXT,
    display_order     INTEGER,
    PRIMARY KEY (program_id, id)
) STRICT;
CREATE INDEX parameter_type_enum_type ON parameter_type_enum (program_id, parameter_type_id);

CREATE TABLE parameter (
    program_id        TEXT NOT NULL,
    id                TEXT NOT NULL,
    name              TEXT,
    text              TEXT,
    parameter_type_id TEXT,
    access            TEXT,
    value             TEXT,
    suffix            TEXT,
    code_segment      TEXT,
    offset            INTEGER,
    bit_offset        INTEGER,
    union_id          INTEGER,
    union_size_in_bit INTEGER,
    PRIMARY KEY (program_id, id)
) STRICT;

CREATE TABLE parameter_ref (
    program_id    TEXT NOT NULL,
    id            TEXT NOT NULL,
    parameter_id  TEXT NOT NULL,
    display_order INTEGER,
    tag           TEXT,
    text          TEXT,
    value         TEXT,
    PRIMARY KEY (program_id, id)
) STRICT;
CREATE INDEX parameter_ref_program ON parameter_ref (program_id);
```

`parse/program.rs` streams the document with this shape:

```rust
//! The application program's `Static` tree: head, parameter types,
//! parameters (including `Union` members and their `Memory` layout) and
//! parameter refs.
//!
//! `Dynamic` is skipped deliberately (spec §1): its `choose`/`when`
//! visibility program depends on the `when/@test` grammar, which is
//! unresearched (RESEARCH R3). The bytes survive in `source_file`, so
//! nothing is lost by not modelling it yet.

use quick_xml::events::Event;
use quick_xml::Reader;
use rusqlite::{params, Connection, OptionalExtension};

use super::report_unknown_attrs;
use crate::report::{IdConflict, UnknownCollector, UnknownConstruct};
use crate::xml::{attrs, local_name, skip_subtree};
use crate::ProductDbError;

pub struct ProgramIngest {
    pub program_id: String,
    pub unknown: Vec<UnknownConstruct>,
    pub conflicts: Vec<IdConflict>,
}

pub fn ingest_program(
    conn: &Connection,
    source_sha256: &str,
    source_path: &str,
    bytes: &[u8],
) -> Result<ProgramIngest, ProductDbError> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut unknown = UnknownCollector::default();
    let mut conflicts = Vec::new();
    let mut manufacturer_id = String::new();
    let mut program_id = String::new();
    // Set while inside <Union>: its size, and the Memory element that
    // belongs to the union rather than to a single parameter.
    let mut union_seq: i64 = 0;
    let mut current_union: Option<(i64, Option<i64>, Option<String>, Option<i64>, Option<i64>)> = None;
    let mut current_parameter_type: Option<String> = None;
    let mut current_parameter: Option<String> = None;
    let mut already_present = false;

    loop {
        buf.clear();
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| ProductDbError::Xml {
                source_path: source_path.to_string(),
                cause: e.to_string(),
            })?;
        match event {
            Event::Eof => break,
            Event::Start(e) if local_name(&e) == "Dynamic" => {
                skip_subtree(&mut reader, e.name().as_ref(), source_path)?;
            }
            Event::Start(e) | Event::Empty(e) => {
                let name = local_name(&e);
                let a = attrs(&e, source_path)?;
                match name.as_str() {
                    "Manufacturer" => {
                        manufacturer_id = a.get("RefId").unwrap_or_default().to_string();
                        conn.execute(
                            "INSERT OR IGNORE INTO manufacturer (id, name) VALUES (?1, NULL)",
                            [&manufacturer_id],
                        )?;
                    }
                    "ApplicationProgram" => {
                        program_id = a.get("Id").unwrap_or_default().to_string();
                        let existing: Option<String> = conn
                            .query_row(
                                "SELECT source_sha256 FROM application_program WHERE id = ?1",
                                [&program_id],
                                |r| r.get(0),
                            )
                            .optional()?;
                        if let Some(kept) = existing {
                            already_present = true;
                            if kept != source_sha256 {
                                conflicts.push(IdConflict {
                                    table: "application_program".into(),
                                    id: program_id.clone(),
                                    kept_sha256: kept,
                                    other_sha256: source_sha256.to_string(),
                                });
                            }
                        } else {
                            report_unknown_attrs(
                                &mut unknown,
                                "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram",
                                &a,
                                PROGRAM_ATTRS,
                            );
                            conn.execute(
                                "INSERT INTO application_program
                                 (id, manufacturer_id, name, application_number,
                                  application_version, program_type, mask_version, pei_type,
                                  load_procedure_style, default_language, hash, linkable,
                                  original_manufacturer, source_sha256)
                                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                                params![
                                    program_id,
                                    manufacturer_id,
                                    a.get("Name"),
                                    a.get("ApplicationNumber"),
                                    a.get("ApplicationVersion"),
                                    a.get("ProgramType"),
                                    a.get("MaskVersion"),
                                    a.get("PeiType"),
                                    a.get("LoadProcedureStyle"),
                                    a.get("DefaultLanguage"),
                                    a.get("Hash"),
                                    bool_flag(&a, "Linkable"),
                                    a.get("OriginalManufacturer"),
                                    source_sha256,
                                ],
                            )?;
                        }
                    }
                    // ... ParameterType / TypeRestriction / TypeNumber /
                    // Enumeration / Parameter / Union / Memory / ParameterRef
                    _ => {}
                }
            }
            Event::End(e) if local_name(&e) == "Union" => current_union = None,
            _ => {}
        }
    }

    Ok(ProgramIngest {
        program_id,
        unknown: unknown.into_vec(),
        conflicts,
    })
}
```

Fill the elided arms following these rules, each of which a test above pins down:

- `ParameterType`: remember its id; `kind` is decided by the child that follows — `TypeRestriction` → `"Restriction"` (also storing `base`, `size_in_bit`), `TypeNumber` → `"Number"` (`min_inclusive`, `max_inclusive`, `size_in_bit`, `number_type` from `@Type`), `TypeText` → `"Text"`, `TypeNone` → `"None"`, `TypeFloat` → `"Float"`, `TypeIPAddress` → `"IPAddress"`, `TypePicture` → `"Picture"`, `TypeRawData` → `"Raw"`, anything else → `"Other"` plus an `unknown.element(...)` entry.
- `Enumeration`: one row per value, keyed `(program_id, id)`, with `parameter_type_id` from the enclosing `ParameterType`.
- `Union`: increment `union_seq`, store `@SizeInBit`; a `Memory` inside the union but before any `Parameter` belongs to the union and is copied onto every member parameter.
- `Parameter`: row with `union_id`/`union_size_in_bit` when inside a union, otherwise both `NULL`; a following `Memory` sets `code_segment`, `offset`, `bit_offset` on that parameter.
- `ParameterRef`: `parameter_id` from `@RefId`; nullable `text`/`value` are the per-variant overrides.
- When `already_present` is true, skip every insert: the first ingest's rows win, and the conflict — if the hash differs — is recorded.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-productdb program`
Expected: PASS (8 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-productdb
git commit -m "feat(productdb): ingest application program head, parameter types and parameters"
```

---

### Task 6: Communication objects

**Files:**
- Create: `crates/knx-productdb/src/parse/comobject.rs`
- Modify: `crates/knx-productdb/src/migration.rs`, `crates/knx-productdb/src/parse/mod.rs`, `crates/knx-productdb/src/parse/program.rs`

**Interfaces:**
- Consumes: Task 5's streaming loop, which calls into this module for the `ComObjectTable`/`ComObjectRefs` subtrees.
- Produces: `parse::comobject::insert_com_object(&Connection, program_id: &str, a: &Attrs) -> Result<(), ProductDbError>` and `parse::comobject::insert_com_object_ref(&Connection, program_id: &str, a: &Attrs) -> Result<(), ProductDbError>`, plus `parse::comobject::COM_OBJECT_ATTRS`/`COM_OBJECT_REF_ATTRS` for the unknown-attribute check.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;
    use crate::parse::program::ingest_program;

    const PROGRAM: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <ApplicationPrograms>
        <ApplicationProgram Id="A-1" Name="P" ApplicationNumber="1" ApplicationVersion="22"
                            MaskVersion="MV-0701">
          <Static>
            <ComObjectTable>
              <ComObject Id="A-1_O-0" Name="Ausgang - Licht" Text="Ausgang - Licht"
                         FunctionText="Dimmen absolut" Number="0" ObjectSize="1 Byte"
                         Priority="Low" ReadFlag="Enabled" WriteFlag="Disabled"
                         CommunicationFlag="Enabled" TransmitFlag="Enabled"
                         UpdateFlag="Disabled" ReadOnInitFlag="Disabled" VisibleDescription="" />
              <ComObject Id="A-1_O-1" Name="Schalten" Text="Schalten" Number="1"
                         ObjectSize="1 Bit" DatapointType="DPST-1-1" WriteFlag="Enabled" />
            </ComObjectTable>
            <ComObjectRefs>
              <ComObjectRef Id="A-1_O-1_R-10003" RefId="A-1_O-1" Tag="10003" />
              <ComObjectRef Id="A-1_O-0_R-10004" RefId="A-1_O-0" Text="Ausgang - Dimmen"
                            DatapointType="DPST-9-21 DPST-9-1" WriteFlag="Enabled" />
            </ComObjectRefs>
          </Static>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn com_objects_store_number_texts_size_and_flags() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let (number, text, function, size, read, write): (i64, String, String, String, String, String) =
            conn.query_row(
                "SELECT number, text, function_text, object_size, read_flag, write_flag
                 FROM com_object WHERE id = 'A-1_O-0'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .unwrap();
        assert_eq!(number, 0);
        assert_eq!(text, "Ausgang - Licht");
        assert_eq!(function, "Dimmen absolut");
        assert_eq!(size, "1 Byte");
        assert_eq!((read.as_str(), write.as_str()), ("Enabled", "Disabled"));
    }

    #[test]
    fn a_com_object_ref_without_overrides_stores_nulls_not_defaults() {
        // A ProgramRef-layer row states only what the variant overrides.
        // Copying the program's own values down would make every attribute
        // look overridden, which is exactly the distinction DATA_MODEL §3
        // exists to keep.
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let (com, text, dpt): (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT com_object_id, text, dpt_list FROM com_object_ref WHERE id = 'A-1_O-1_R-10003'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(com, "A-1_O-1");
        assert_eq!(text, None);
        assert_eq!(dpt, None);
    }

    #[test]
    fn a_datapoint_type_list_is_stored_verbatim() {
        // RESEARCH §4.2: the attribute can hold several space-separated
        // alternatives. Stored as written; deciding between them is the
        // enrichment's problem, and it refuses to guess (Task 11).
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let dpt: String = conn
            .query_row(
                "SELECT dpt_list FROM com_object_ref WHERE id = 'A-1_O-0_R-10004'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(dpt, "DPST-9-21 DPST-9-1");
    }

    #[test]
    fn a_program_level_datapoint_type_is_stored_on_the_com_object() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let dpt: String = conn
            .query_row("SELECT dpt_list FROM com_object WHERE id = 'A-1_O-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(dpt, "DPST-1-1");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-productdb comobject`
Expected: FAIL — table `com_object` does not exist.

- [ ] **Step 3: Write minimal implementation**

Extend `migrate_v0_to_v1`:

```sql
CREATE TABLE com_object (
    program_id          TEXT NOT NULL,
    id                  TEXT NOT NULL,
    number              INTEGER,
    name                TEXT,
    text                TEXT,
    function_text       TEXT,
    visible_description TEXT,
    object_size         TEXT,
    priority            TEXT,
    dpt_list            TEXT,
    read_flag           TEXT,
    write_flag          TEXT,
    transmit_flag       TEXT,
    update_flag         TEXT,
    communication_flag  TEXT,
    read_on_init_flag   TEXT,
    PRIMARY KEY (program_id, id)
) STRICT;

CREATE TABLE com_object_ref (
    program_id          TEXT NOT NULL,
    id                  TEXT NOT NULL,
    com_object_id       TEXT NOT NULL,
    tag                 TEXT,
    text                TEXT,
    function_text       TEXT,
    visible_description TEXT,
    object_size         TEXT,
    priority            TEXT,
    dpt_list            TEXT,
    read_flag           TEXT,
    write_flag          TEXT,
    transmit_flag       TEXT,
    update_flag         TEXT,
    communication_flag  TEXT,
    read_on_init_flag   TEXT,
    PRIMARY KEY (program_id, id)
) STRICT;
CREATE INDEX com_object_ref_program ON com_object_ref (program_id);
CREATE INDEX com_object_ref_object ON com_object_ref (program_id, com_object_id);
```

`parse/comobject.rs` holds the two insert functions and the attribute lists; `parse/program.rs` gains two match arms calling them (`"ComObject"` and `"ComObjectRef"`), passing the current `program_id`. Flags are stored as the source's own `"Enabled"`/`"Disabled"` strings — the conversion to `bool` happens once, in enrichment, where `knx-core`'s types are in scope.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-productdb`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/knx-productdb
git commit -m "feat(productdb): ingest communication objects and their refs"
```

---

### Task 7: Translations

**Files:**
- Create: `crates/knx-productdb/src/parse/translation.rs`
- Modify: `crates/knx-productdb/src/migration.rs`, `crates/knx-productdb/src/parse/program.rs`

**Interfaces:**
- Consumes: Task 5's loop.
- Produces: `parse::translation::insert_translations(&Connection, program_id: &str, language: &str, ref_id: &str, attribute_name: &str, text: &str) -> Result<(), ProductDbError>`, and a `Languages` arm in `ingest_program`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;
    use crate::parse::program::ingest_program;

    const PROGRAM: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <ApplicationPrograms>
        <ApplicationProgram Id="A-1" Name="P" ApplicationVersion="22" MaskVersion="MV-0701">
          <Static>
            <ComObjectTable>
              <ComObject Id="A-1_O-0" Number="0" Text="Ausgang" ObjectSize="1 Bit" />
            </ComObjectTable>
          </Static>
          <Languages>
            <Language Identifier="en-US">
              <TranslationUnit RefId="A-1">
                <TranslationElement RefId="A-1_O-0">
                  <Translation AttributeName="Text" Text="Output" />
                  <Translation AttributeName="FunctionText" Text="Switch" />
                </TranslationElement>
              </TranslationUnit>
            </Language>
            <Language Identifier="de-DE">
              <TranslationUnit RefId="A-1">
                <TranslationElement RefId="A-1_O-0">
                  <Translation AttributeName="Text" Text="Ausgang" />
                </TranslationElement>
              </TranslationUnit>
            </Language>
          </Languages>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn a_translation_is_keyed_by_language_ref_and_attribute() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let text: String = conn
            .query_row(
                "SELECT text FROM translation
                 WHERE program_id = 'A-1' AND language = 'en-US'
                   AND ref_id = 'A-1_O-0' AND attribute_name = 'Text'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(text, "Output");
    }

    #[test]
    fn the_same_element_translates_into_several_languages() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM).unwrap();
        let rows: i64 = conn
            .query_row(
                "SELECT count(*) FROM translation WHERE ref_id = 'A-1_O-0'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 3);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-productdb translation`
Expected: FAIL — table `translation` does not exist.

- [ ] **Step 3: Write minimal implementation**

Extend `migrate_v0_to_v1`:

```sql
CREATE TABLE translation (
    program_id     TEXT NOT NULL,
    language       TEXT NOT NULL,
    ref_id         TEXT NOT NULL,
    attribute_name TEXT NOT NULL,
    text           TEXT,
    PRIMARY KEY (program_id, language, ref_id, attribute_name)
) STRICT;
CREATE INDEX translation_lookup ON translation (program_id, language, ref_id);
```

In `ingest_program`, track `current_language` on `Language/@Identifier` and `current_translation_ref` on `TranslationElement/@RefId`, then insert one row per `Translation` with `INSERT OR IGNORE`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-productdb`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/knx-productdb
git commit -m "feat(productdb): ingest application program translations"
```

---

### Task 8: Master data

**Files:**
- Create: `crates/knx-productdb/src/parse/master.rs`
- Modify: `crates/knx-productdb/src/migration.rs`, `crates/knx-productdb/src/parse/mod.rs`, `crates/knx-productdb/src/lib.rs`

**Interfaces:**
- Consumes: Task 3 helpers.
- Produces: `ingest_master_data(&Connection, bytes: &[u8]) -> Result<Vec<UnknownConstruct>, ProductDbError>`, filling `manufacturer.name` and the `datapoint_type` table.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    const MASTER: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0001" Name="Siemens" />
      <Manufacturer Id="M-0083" Name="MDT technologies" />
    </Manufacturers>
    <DatapointTypes>
      <DatapointType Id="DPT-1" Number="1" Name="1.xxx" Text="1-bit">
        <DatapointSubtypes>
          <DatapointSubtype Id="DPST-1-1" Number="1" Name="DPT_Switch" Text="switch" />
        </DatapointSubtypes>
      </DatapointType>
    </DatapointTypes>
  </MasterData>
</KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn manufacturer_names_are_filled_in() {
        let (_dir, conn) = db();
        ingest_master_data(&conn, MASTER).unwrap();
        let name: String = conn
            .query_row("SELECT name FROM manufacturer WHERE id = 'M-0083'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "MDT technologies");
    }

    #[test]
    fn a_manufacturer_seen_during_ingest_first_gets_its_name_later() {
        // Hardware.xml creates the row with a NULL name (Task 4); master
        // data fills it in whichever order the two arrive.
        let (_dir, conn) = db();
        conn.execute("INSERT INTO manufacturer (id, name) VALUES ('M-0083', NULL)", [])
            .unwrap();
        ingest_master_data(&conn, MASTER).unwrap();
        let name: String = conn
            .query_row("SELECT name FROM manufacturer WHERE id = 'M-0083'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "MDT technologies");
    }

    #[test]
    fn datapoint_main_and_subtypes_are_stored_with_their_numbers() {
        let (_dir, conn) = db();
        ingest_master_data(&conn, MASTER).unwrap();
        let (main, sub, name): (i64, Option<i64>, String) = conn
            .query_row(
                "SELECT main, sub, name FROM datapoint_type WHERE id = 'DPST-1-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((main, sub, name.as_str()), (1, Some(1), "DPT_Switch"));
        let main_only: (i64, Option<i64>) = conn
            .query_row("SELECT main, sub FROM datapoint_type WHERE id = 'DPT-1'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(main_only, (1, None));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-productdb master`
Expected: FAIL — `ingest_master_data` does not exist.

- [ ] **Step 3: Write minimal implementation**

Extend `migrate_v0_to_v1`:

```sql
CREATE TABLE datapoint_type (
    id   TEXT PRIMARY KEY,
    main INTEGER NOT NULL,
    sub  INTEGER,
    name TEXT,
    text TEXT
) STRICT;
```

`parse/master.rs` streams `MasterData`, handling `Manufacturer` with
`INSERT INTO manufacturer (id, name) VALUES (?1, ?2) ON CONFLICT(id) DO UPDATE SET name = excluded.name`,
and `DatapointType`/`DatapointSubtype`, remembering the current main type's number so a subtype row gets both numbers.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-productdb`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/knx-productdb
git commit -m "feat(productdb): ingest manufacturer names and datapoint types from master data"
```

---

### Task 9: Ingest orchestration

**Files:**
- Create: `crates/knx-productdb/src/ingest.rs`
- Modify: `crates/knx-productdb/src/lib.rs`

**Interfaces:**
- Consumes: everything from Tasks 2–8.
- Produces: `ingest_file(&Connection, source_path: &str, bytes: &[u8]) -> Result<IngestOutcome, ProductDbError>`, `pub enum IngestOutcome { Ingested { sha256: String, kind: FileKind, unknown: usize, conflicts: Vec<IdConflict> }, Skipped { sha256: String } }`, `pub enum FileKind { Catalog, Hardware, ApplicationProgram, Baggages, Baggage, Unrecognized }`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    const HARDWARE: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1"><Products>
<Product Id="H-1_P-1" Text="X" OrderNumber="N1" /></Products></Hardware></Hardware>
</Manufacturer></ManufacturerData></KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn a_file_is_classified_by_its_content_not_its_name() {
        let (_dir, conn) = db();
        let out = ingest_file(&conn, "M-006A/Whatever.xml", HARDWARE).unwrap();
        assert!(matches!(
            out,
            IngestOutcome::Ingested { kind: FileKind::Hardware, .. }
        ));
    }

    #[test]
    fn the_second_ingest_of_the_same_bytes_is_skipped_without_parsing() {
        let (_dir, conn) = db();
        assert!(matches!(
            ingest_file(&conn, "M-006A/Hardware.xml", HARDWARE).unwrap(),
            IngestOutcome::Ingested { .. }
        ));
        assert!(matches!(
            ingest_file(&conn, "M-006A/Hardware.xml", HARDWARE).unwrap(),
            IngestOutcome::Skipped { .. }
        ));
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM hardware", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn a_baggage_blob_is_stored_without_being_parsed() {
        let (_dir, conn) = db();
        let dll = b"MZ\x90\x00binary".to_vec();
        let out = ingest_file(&conn, "M-0008/Baggages/econEts3.dll", &dll).unwrap();
        let sha = match out {
            IngestOutcome::Ingested { sha256, kind: FileKind::Baggage, .. } => sha256,
            other => panic!("expected an ingested baggage blob, got {other:?}"),
        };
        assert_eq!(crate::load_source_file(&conn, &sha).unwrap(), Some(dll));
    }

    #[test]
    fn a_failing_parse_leaves_no_partial_rows_and_no_blob() {
        // One transaction per file: a truncated program must not leave the
        // database holding half of it, or the content-hash skip would then
        // consider that half complete for ever.
        let (_dir, conn) = db();
        let truncated = &HARDWARE[..HARDWARE.len() / 2];
        assert!(ingest_file(&conn, "M-006A/Hardware.xml", truncated).is_err());
        let blobs: i64 = conn
            .query_row("SELECT count(*) FROM source_file", [], |r| r.get(0))
            .unwrap();
        let hardware: i64 = conn
            .query_row("SELECT count(*) FROM hardware", [], |r| r.get(0))
            .unwrap();
        assert_eq!((blobs, hardware), (0, 0));
    }

    #[test]
    fn an_unrecognized_xml_file_is_still_stored_as_a_blob() {
        let (_dir, conn) = db();
        let xml = br#"<?xml version="1.0"?><KNX><SomethingNew/></KNX>"#;
        let out = ingest_file(&conn, "M-006A/New.xml", xml).unwrap();
        assert!(matches!(
            out,
            IngestOutcome::Ingested { kind: FileKind::Unrecognized, .. }
        ));
        assert_eq!(
            crate::load_source_file(&conn, &crate::sha256_hex(xml)).unwrap(),
            Some(xml.to_vec())
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-productdb ingest`
Expected: FAIL — `ingest_file` does not exist.

- [ ] **Step 3: Write minimal implementation**

```rust
//! Per-file ingest: hash, skip-or-parse, one transaction, classification.
//!
//! The content hash is the identity (ADR-0011). A file already in
//! `source_file` is skipped without being parsed at all — which is what
//! makes importing a second project that uses the same devices cheap
//! instead of costing another 22 MB of parsing (RESEARCH §4.1).

use rusqlite::Connection;

use crate::blob::{has_source_file, sha256_hex, store_source_file, SourceFile};
use crate::parse::{catalog, hardware, program};
use crate::report::{insert_conflicts, insert_unknown, IdConflict};
use crate::ProductDbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Catalog,
    Hardware,
    ApplicationProgram,
    Baggages,
    Baggage,
    Unrecognized,
}

#[derive(Debug)]
pub enum IngestOutcome {
    Ingested {
        sha256: String,
        kind: FileKind,
        unknown: usize,
        conflicts: Vec<IdConflict>,
    },
    Skipped {
        sha256: String,
    },
}

/// Ingests one manufacturer file. Everything this function writes — the
/// blob, the parsed rows, the unknown-construct rows — happens in one
/// transaction, so a parse error partway through leaves the database
/// exactly as it was rather than storing a blob whose rows never landed.
pub fn ingest_file(
    conn: &Connection,
    source_path: &str,
    bytes: &[u8],
) -> Result<IngestOutcome, ProductDbError> {
    let sha256 = sha256_hex(bytes);
    if has_source_file(conn, &sha256)? {
        return Ok(IngestOutcome::Skipped { sha256 });
    }

    let tx = conn.unchecked_transaction()?;
    let manufacturer_id = source_path
        .split('/')
        .next()
        .filter(|top| top.starts_with("M-"))
        .map(str::to_string);
    store_source_file(
        &tx,
        &SourceFile {
            source_path: source_path.to_string(),
            manufacturer_id,
            bytes: bytes.to_vec(),
        },
    )?;

    let kind = classify(bytes);
    let (unknown, conflicts) = match kind {
        FileKind::Catalog => (
            catalog::ingest_catalog(&tx, &sha256, source_path, bytes)?,
            Vec::new(),
        ),
        FileKind::Hardware => (
            hardware::ingest_hardware(&tx, &sha256, source_path, bytes)?,
            Vec::new(),
        ),
        FileKind::ApplicationProgram => {
            let out = program::ingest_program(&tx, &sha256, source_path, bytes)?;
            (out.unknown, out.conflicts)
        }
        // Baggages.xml lists the blobs; the blobs themselves and anything
        // unrecognized are stored and not parsed.
        FileKind::Baggages | FileKind::Baggage | FileKind::Unrecognized => (Vec::new(), Vec::new()),
    };

    insert_unknown(&tx, &sha256, &unknown)?;
    insert_conflicts(&tx, &conflicts)?;
    tx.commit()?;

    Ok(IngestOutcome::Ingested {
        sha256,
        kind,
        unknown: unknown.len(),
        conflicts,
    })
}

/// Classifies by the first recognized element inside `ManufacturerData`,
/// not by file name: the name is a convention, the content is the fact.
/// A `Baggages/` blob is not XML at all, so it is recognized by its bytes.
fn classify(bytes: &[u8]) -> FileKind {
    let text = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    if text.iter().find(|b| !b.is_ascii_whitespace()) != Some(&b'<') {
        return FileKind::Baggage;
    }

    let mut reader = quick_xml::Reader::from_reader(text);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e))
            | Ok(quick_xml::events::Event::Empty(e)) => {
                match crate::xml::local_name(&e).as_str() {
                    "Catalog" => return FileKind::Catalog,
                    "Hardware" => return FileKind::Hardware,
                    "ApplicationPrograms" => return FileKind::ApplicationProgram,
                    "Baggages" => return FileKind::Baggages,
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::Eof) | Err(_) => return FileKind::Unrecognized,
            Ok(_) => {}
        }
    }
}
```

Note the classification is content-based on purpose: `M-006A/Whatever.xml` holding a `Hardware` tree is hardware data, and a manufacturer who renames a file does not thereby hide it from the ingest.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-productdb`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/knx-productdb
git commit -m "feat(productdb): per-file ingest with content-hash skip and one transaction per file"
```

---

### Task 10: The read side

**Files:**
- Create: `crates/knx-productdb/src/query.rs`
- Modify: `crates/knx-productdb/src/lib.rs`

**Interfaces:**
- Consumes: the tables of Tasks 4–8.
- Produces:
  - `query::resolve_program(&Connection, hardware2program_id: &str) -> Result<Option<String>, ProductDbError>` — the application program id a device's `Hardware2ProgramRefId` leads to.
  - `query::com_object_view(&Connection, program_id: &str, com_object_ref_id: &str) -> Result<Option<ComObjectView>, ProductDbError>`.
  - `ComObjectView`, one merged view of the `ComObject` and `ComObjectRef` rows, carrying a value **and its layer** for every attribute the enrichment writes:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComObjectView {
    pub number: Option<i64>,
    pub text: Option<String>,
    pub text_layer: ValueLayer,
    pub function_text: Option<String>,
    pub function_text_layer: ValueLayer,
    pub visible_description: Option<String>,
    pub description_layer: ValueLayer,
    pub object_size: Option<String>,
    pub object_size_layer: ValueLayer,
    pub priority: Option<String>,
    pub dpt_list: Option<String>,
    pub dpt_layer: ValueLayer,
    pub read: Option<String>,
    pub read_layer: ValueLayer,
    pub write: Option<String>,
    pub write_layer: ValueLayer,
    pub transmit: Option<String>,
    pub transmit_layer: ValueLayer,
    pub update: Option<String>,
    pub update_layer: ValueLayer,
    pub communication: Option<String>,
    pub communication_layer: ValueLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueLayer {
    Program,
    ProgramRef,
}
```

    A layer field is meaningless when its value is `None`; it says which row supplied the value when there is one. `pick` (below) produces both together, so the two can never drift apart.
  - `query::manufacturers(&Connection) -> Result<Vec<(String, Option<String>)>, ProductDbError>` and `query::programs(&Connection, manufacturer: Option<&str>) -> Result<Vec<ProgramRow>, ProductDbError>` with `pub struct ProgramRow { pub id: String, pub manufacturer_id: String, pub name: Option<String>, pub application_number: Option<String>, pub application_version: Option<String>, pub mask_version: Option<String> }` — the listings `knx products` prints.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;
    use crate::parse::{hardware::ingest_hardware, program::ingest_program};

    const HARDWARE: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

    const PROGRAM: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationNumber="1"
  ApplicationVersion="22" MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Schalten" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" WriteFlag="Enabled" ReadFlag="Disabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
  <ComObjectRef Id="A-1_O-1_R-2" RefId="A-1_O-1" Text="Dimmen" DatapointType="DPST-3-7" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_hardware(&conn, "sha-h", "M-006A/Hardware.xml", HARDWARE).unwrap();
        ingest_program(&conn, "sha-p", "M-006A/A.xml", PROGRAM).unwrap();
        (dir, conn)
    }

    #[test]
    fn a_hardware2program_id_resolves_to_its_application_program() {
        let (_dir, conn) = db();
        assert_eq!(resolve_program(&conn, "H-1_HP-1").unwrap(), Some("A-1".into()));
        assert_eq!(resolve_program(&conn, "H-9_HP-9").unwrap(), None);
    }

    #[test]
    fn a_ref_without_overrides_shows_the_program_layer_values() {
        let (_dir, conn) = db();
        let v = com_object_view(&conn, "A-1", "A-1_O-1_R-1").unwrap().unwrap();
        assert_eq!(v.text.as_deref(), Some("Schalten"));
        assert_eq!(v.text_layer, ValueLayer::Program);
        assert_eq!(v.dpt_list.as_deref(), Some("DPST-1-1"));
        assert_eq!(v.dpt_layer, ValueLayer::Program);
        assert_eq!(v.number, Some(1));
        assert_eq!(v.write.as_deref(), Some("Enabled"));
    }

    #[test]
    fn a_ref_with_overrides_shows_the_program_ref_layer_for_those_values_only() {
        let (_dir, conn) = db();
        let v = com_object_view(&conn, "A-1", "A-1_O-1_R-2").unwrap().unwrap();
        assert_eq!(v.text.as_deref(), Some("Dimmen"));
        assert_eq!(v.text_layer, ValueLayer::ProgramRef);
        assert_eq!(v.dpt_list.as_deref(), Some("DPST-3-7"));
        assert_eq!(v.dpt_layer, ValueLayer::ProgramRef);
        // Not overridden: still the program's own value and layer.
        assert_eq!(v.object_size.as_deref(), Some("1 Bit"));
        assert_eq!(v.write.as_deref(), Some("Enabled"));
    }

    #[test]
    fn an_unknown_ref_id_is_none_not_an_error() {
        let (_dir, conn) = db();
        assert!(com_object_view(&conn, "A-1", "A-1_O-9_R-9").unwrap().is_none());
    }

    #[test]
    fn listings_return_what_the_cli_prints() {
        let (_dir, conn) = db();
        assert_eq!(manufacturers(&conn).unwrap(), vec![("M-006A".to_string(), None)]);
        let programs = programs(&conn, Some("M-006A")).unwrap();
        assert_eq!(programs.len(), 1);
        assert_eq!(programs[0].id, "A-1");
        assert_eq!(programs[0].application_version.as_deref(), Some("22"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-productdb query`
Expected: FAIL — module `query` does not exist.

- [ ] **Step 3: Write minimal implementation**

`com_object_view` runs one join and folds the two rows per attribute:

```rust
//! The read side: resolving a device's program reference, and the merged
//! `ComObject` + `ComObjectRef` view the enrichment consumes.
//!
//! The merge keeps the layer that supplied each value. That is the whole
//! point of the override chain (DATA_MODEL §3): a value without its layer
//! cannot be written back correctly, so this view never returns one.

use rusqlite::{Connection, OptionalExtension};

use crate::ProductDbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueLayer {
    Program,
    ProgramRef,
}

/// Picks the `ComObjectRef` value when it has one, otherwise the
/// `ComObject`'s, reporting which layer won.
fn pick(program: Option<String>, program_ref: Option<String>) -> (Option<String>, ValueLayer) {
    match program_ref {
        Some(v) => (Some(v), ValueLayer::ProgramRef),
        None => (program, ValueLayer::Program),
    }
}

pub fn resolve_program(
    conn: &Connection,
    hardware2program_id: &str,
) -> Result<Option<String>, ProductDbError> {
    let id: Option<String> = conn
        .query_row(
            "SELECT ap.id
             FROM hardware2program h2p
             JOIN application_program ap ON ap.id = h2p.application_program_ref
             WHERE h2p.id = ?1",
            [hardware2program_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(id)
}
```

`com_object_view` selects `co.*` and `cor.*` for the given ref id, then builds the struct through `pick` per attribute. `manufacturers` and `programs` are plain `SELECT`s ordered by id.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-productdb`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/knx-productdb
git commit -m "feat(productdb): resolve program references and merge the two program layers"
```

---

### Task 11: Enrichment

**Files:**
- Create: `crates/knx-productdb/src/enrich.rs`
- Modify: `crates/knx-productdb/src/lib.rs`

**Interfaces:**
- Consumes: `query::{resolve_program, com_object_view, ComObjectView, ValueLayer}`; `knx_core::{Project, Override, Resolved, Layer, Text, DptRef, ObjectSize}`.
- Produces: `enrich(&mut Project, &Connection) -> Result<EnrichmentReport, ProductDbError>`, `pub struct EnrichmentReport { pub available: bool, pub devices_resolved: usize, pub com_objects_enriched: usize, pub issues: Vec<EnrichmentIssue> }`, `pub enum EnrichmentIssue { ProgramMissing { device_ets_id: String, program_ref: String }, ComObjectRefMissing { device_ets_id: String, ref_id: String }, AmbiguousDpt { ref_id: String, alternatives: Vec<String> } }`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;
    use crate::parse::{hardware::ingest_hardware, program::ingest_program};
    use knx_core::{
        ComObjectInstance, DeviceInstance, Language, Layer, Override, Project, ResolvedFlags,
        SourceRef,
    };

    // Same fixtures as Task 10, plus a ref whose DatapointType is a list.
    const HARDWARE: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

    const PROGRAM: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="22"
  MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Schalten" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" WriteFlag="Enabled" />
  <ComObject Id="A-1_O-2" Number="2" Text="Wert" ObjectSize="2 Bytes" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
  <ComObjectRef Id="A-1_O-2_R-1" RefId="A-1_O-2" DatapointType="DPST-9-21 DPST-9-1" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_hardware(&conn, "sha-h", "M-006A/Hardware.xml", HARDWARE).unwrap();
        ingest_program(&conn, "sha-p", "M-006A/A.xml", PROGRAM).unwrap();
        (dir, conn)
    }

    fn source(ets_id: &str) -> SourceRef {
        SourceRef {
            path: "P-0001/0.xml".into(),
            ets_id: ets_id.into(),
        }
    }

    /// One device with one communication object whose `text`, `dpt` and
    /// `flags` are all `Absent` at instance level.
    fn project_with(com_ref_id: &str, dpt: Override<knx_core::DptRef>) -> Project {
        let mut p = Project::new(Language("de-DE".into()));
        let device_id = p.ids.next_device_id();
        let com_id = p.ids.next_com_object_instance_id();
        p.devices.insert(DeviceInstance {
            id: device_id,
            source: source("P-0001-0_DI-1"),
            name: "D".into(),
            description: None,
            address: None,
            product_ref: "H-1_P-1".into(),
            program_ref: "H-1_HP-1".into(),
            commissioning: Default::default(),
            visibility_calculated: false,
            com_objects: vec![com_id],
            binary_data: vec![],
        });
        p.devices.insert_com_object(ComObjectInstance {
            id: com_id,
            source: source(com_ref_id),
            device: device_id,
            number: 1,
            text: Override::Absent,
            description: Override::Absent,
            dpt,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
        });
        p
    }

    #[test]
    fn an_absent_text_is_filled_at_the_program_layer() {
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        let report = enrich(&mut p, &conn).unwrap();
        assert!(report.available);
        assert_eq!(report.com_objects_enriched, 1);
        let com = p.devices.com_object(knx_core::ComObjectInstanceId(1)).unwrap();
        match &com.text {
            Override::Value(r) => {
                assert_eq!(r.layer, Layer::Program);
                assert_eq!(p.strings.text(&r.value, p.strings.default_language()), Some("Schalten"));
            }
            other => panic!("expected a program-layer text, got {other:?}"),
        }
    }

    #[test]
    fn an_absent_datapoint_type_is_filled_and_a_size_is_set() {
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        enrich(&mut p, &conn).unwrap();
        let com = p.devices.com_object(knx_core::ComObjectInstanceId(1)).unwrap();
        match &com.dpt {
            Override::Value(r) => {
                assert_eq!(r.layer, Layer::Program);
                assert_eq!(r.value, knx_core::DptRef { main: 1, sub: Some(1) });
            }
            other => panic!("expected a program-layer dpt, got {other:?}"),
        }
        assert_eq!(
            com.size.map(|s| s.value),
            Some(knx_core::ObjectSize::Bit(1))
        );
    }

    #[test]
    fn an_empty_instance_attribute_is_never_overwritten() {
        // 497 of the reference project's communication objects carry
        // DatapointType="". Replacing that with a program value would make
        // the exporter write the attribute as absent instead of empty,
        // changing the file (ADR-0012).
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-1_R-1", Override::Empty);
        enrich(&mut p, &conn).unwrap();
        let com = p.devices.com_object(knx_core::ComObjectInstanceId(1)).unwrap();
        assert_eq!(com.dpt, Override::Empty);
    }

    #[test]
    fn an_instance_value_is_never_overwritten() {
        let (_dir, conn) = db();
        let instance = Override::Value(knx_core::Resolved {
            value: knx_core::DptRef { main: 5, sub: Some(1) },
            layer: Layer::Instance,
        });
        let mut p = project_with("A-1_O-1_R-1", instance.clone());
        enrich(&mut p, &conn).unwrap();
        let com = p.devices.com_object(knx_core::ComObjectInstanceId(1)).unwrap();
        assert_eq!(com.dpt, instance);
    }

    #[test]
    fn a_datapoint_type_list_fills_nothing_and_is_reported() {
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-2_R-1", Override::Absent);
        let report = enrich(&mut p, &conn).unwrap();
        let com = p.devices.com_object(knx_core::ComObjectInstanceId(1)).unwrap();
        assert_eq!(com.dpt, Override::Absent, "an ambiguous list is never guessed");
        assert!(report.issues.iter().any(|i| matches!(
            i,
            EnrichmentIssue::AmbiguousDpt { alternatives, .. } if alternatives.len() == 2
        )));
    }

    #[test]
    fn a_missing_program_is_reported_per_device_and_leaves_the_model_alone() {
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        p.devices
            .get_mut(knx_core::DeviceId(1))
            .unwrap()
            .program_ref = "H-9_HP-9".into();
        let report = enrich(&mut p, &conn).unwrap();
        assert_eq!(report.com_objects_enriched, 0);
        assert!(matches!(
            report.issues.as_slice(),
            [EnrichmentIssue::ProgramMissing { program_ref, .. }] if program_ref == "H-9_HP-9"
        ));
        let com = p.devices.com_object(knx_core::ComObjectInstanceId(1)).unwrap();
        assert_eq!(com.text, Override::Absent);
    }

    #[test]
    fn an_empty_database_reports_unavailable_and_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("empty.sqlite")).unwrap();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        let report = enrich(&mut p, &conn).unwrap();
        assert!(!report.available);
        assert_eq!(report.com_objects_enriched, 0);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-productdb enrich`
Expected: FAIL — `enrich` does not exist.

- [ ] **Step 3: Write minimal implementation**

```rust
//! Filling the `Program` and `ProgramRef` layers of an already-mapped
//! project from the product database (ADR-0012).
//!
//! Two rules, both load-bearing:
//!
//! 1. Only `Override::Absent` slots are filled. `Empty`, `Malformed` and
//!    `Value(Instance)` are what the project file actually said, and the
//!    exporter reproduces them; overwriting one would change the file.
//! 2. Nothing is guessed. A datapoint type stated as a list of
//!    alternatives (RESEARCH §4.2) fills nothing and is reported.
//!
//! Values written here carry `Layer::Program` or `Layer::ProgramRef`,
//! neither of which `Layer::is_exported()` accepts, so enrichment can
//! never leak into an export.

use knx_core::{
    ComObjectInstanceId, DeviceId, DptRef, Layer, ObjectSize, Override, Project, Resolved, Text,
};
use rusqlite::Connection;

use crate::query::{com_object_view, resolve_program, ComObjectView, ValueLayer};
use crate::ProductDbError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnrichmentIssue {
    ProgramMissing {
        device_ets_id: String,
        program_ref: String,
    },
    ComObjectRefMissing {
        device_ets_id: String,
        ref_id: String,
    },
    AmbiguousDpt {
        ref_id: String,
        alternatives: Vec<String>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnrichmentReport {
    /// False when the database holds no application program at all — the
    /// "product database missing" state ADR-0005 requires to be ordinary,
    /// not an error path.
    pub available: bool,
    pub devices_resolved: usize,
    pub com_objects_enriched: usize,
    pub issues: Vec<EnrichmentIssue>,
}

pub fn enrich(
    project: &mut Project,
    conn: &Connection,
) -> Result<EnrichmentReport, ProductDbError> {
    let mut report = EnrichmentReport {
        available: has_any_program(conn)?,
        ..EnrichmentReport::default()
    };
    if !report.available {
        return Ok(report);
    }

    // Collected first, so the mutable pass below borrows nothing else.
    let devices: Vec<(DeviceId, String, String, Vec<(ComObjectInstanceId, String)>)> = project
        .devices
        .iter()
        .map(|d| {
            let coms = d
                .com_objects
                .iter()
                .filter_map(|&id| project.devices.com_object(id).map(|c| (id, c.source.ets_id.clone())))
                .collect();
            (d.id, d.source.ets_id.clone(), d.program_ref.clone(), coms)
        })
        .collect();

    for (_device, device_ets_id, program_ref, coms) in devices {
        let Some(program_id) = resolve_program(conn, &program_ref)? else {
            report.issues.push(EnrichmentIssue::ProgramMissing {
                device_ets_id,
                program_ref,
            });
            continue;
        };
        report.devices_resolved += 1;
        for (com_id, ref_id) in coms {
            let Some(view) = com_object_view(conn, &program_id, &ref_id)? else {
                report.issues.push(EnrichmentIssue::ComObjectRefMissing {
                    device_ets_id: device_ets_id.clone(),
                    ref_id,
                });
                continue;
            };
            if apply(project, com_id, &ref_id, &view, &mut report.issues) {
                report.com_objects_enriched += 1;
            }
        }
    }
    Ok(report)
}
```

`apply` does the per-attribute work, and every write in it goes through
`fill_absent`, which is the single place ADR-0012's rule lives:

```rust
/// Writes `value` only into an `Override::Absent` slot. Every other state
/// — `Empty`, `Malformed`, `Value` — is what the project file said, and
/// the exporter reproduces it; overwriting one would change the file.
fn fill_absent<T>(slot: &mut Override<T>, value: T, layer: Layer) -> bool {
    if matches!(slot, Override::Absent) {
        *slot = Override::Value(Resolved { value, layer });
        return true;
    }
    false
}

fn layer_of(layer: ValueLayer) -> Layer {
    match layer {
        ValueLayer::Program => Layer::Program,
        ValueLayer::ProgramRef => Layer::ProgramRef,
    }
}

fn apply(
    project: &mut Project,
    com_id: ComObjectInstanceId,
    ref_id: &str,
    view: &ComObjectView,
    issues: &mut Vec<EnrichmentIssue>,
) -> bool {
    // The datapoint type is decided before the mutable borrow, because
    // refusing an ambiguous list is a report entry, not a write.
    let dpt = match view.dpt_list.as_deref() {
        None => None,
        Some(list) => {
            let alternatives: Vec<&str> = list.split_whitespace().collect();
            match alternatives.as_slice() {
                [] => None,
                [one] => match DptRef::parse(one) {
                    Ok(dpt) => Some(dpt),
                    Err(_) => {
                        issues.push(EnrichmentIssue::AmbiguousDpt {
                            ref_id: ref_id.to_string(),
                            alternatives: vec![(*one).to_string()],
                        });
                        None
                    }
                },
                many => {
                    // RESEARCH §4.2: a list of acceptable alternatives.
                    // Which one applies cannot be decided from one sample,
                    // and guessing would be invented compatibility.
                    issues.push(EnrichmentIssue::AmbiguousDpt {
                        ref_id: ref_id.to_string(),
                        alternatives: many.iter().map(|s| (*s).to_string()).collect(),
                    });
                    None
                }
            }
        }
    };

    let text = view
        .text
        .as_ref()
        .map(|t| (Text::Literal(t.clone()), layer_of(view.text_layer)));
    let description = view
        .visible_description
        .as_ref()
        .map(|t| (Text::Literal(t.clone()), layer_of(view.description_layer)));
    let size = view.object_size.as_deref().and_then(parse_object_size);

    let Some(com) = project.devices.com_object_mut(com_id) else {
        return false;
    };
    let mut changed = false;
    if let Some((value, layer)) = text {
        changed |= fill_absent(&mut com.text, value, layer);
    }
    if let Some((value, layer)) = description {
        changed |= fill_absent(&mut com.description, value, layer);
    }
    if let Some(dpt) = dpt {
        changed |= fill_absent(&mut com.dpt, dpt, layer_of(view.dpt_layer));
    }
    for (slot, stated, layer) in [
        (&mut com.flags.read, view.read.as_deref(), view.read_layer),
        (&mut com.flags.write, view.write.as_deref(), view.write_layer),
        (&mut com.flags.transmit, view.transmit.as_deref(), view.transmit_layer),
        (&mut com.flags.update, view.update.as_deref(), view.update_layer),
        (
            &mut com.flags.communication,
            view.communication.as_deref(),
            view.communication_layer,
        ),
    ] {
        // "Enabled"/"Disabled" are the only two values the source uses;
        // anything else stays absent rather than becoming a guessed false.
        let flag = match stated {
            Some("Enabled") => Some(true),
            Some("Disabled") => Some(false),
            _ => None,
        };
        if let Some(flag) = flag {
            changed |= fill_absent(slot, flag, layer_of(layer));
        }
    }
    if com.size.is_none() {
        if let Some((value, layer)) = size.map(|s| (s, layer_of(view.object_size_layer))) {
            com.size = Some(Resolved { value, layer });
            changed = true;
        }
    }
    changed
}

/// `"1 Bit"` -> `Bit(1)`, `"14 Bytes"` -> `Byte(14)`. An unrecognized
/// spelling yields `None`; the size stays unknown rather than wrong.
fn parse_object_size(text: &str) -> Option<ObjectSize> {
    let (number, unit) = text.split_once(' ')?;
    let n: u16 = number.parse().ok()?;
    match unit {
        "Bit" | "Bits" => u8::try_from(n).ok().map(ObjectSize::Bit),
        "Byte" | "Bytes" => Some(ObjectSize::Byte(n)),
        _ => None,
    }
}
```

The rules that code implements, restated so a reviewer can check it against the tests:

- `text` / `description`: when the slot is `Override::Absent` and the view has a value, insert the string into `project.strings` and store `Override::Value(Resolved { value: Text::Literal(s), layer })` with the view's layer.
- `dpt`: split the view's `dpt_list` on whitespace. Exactly one alternative that parses as a `DptRef` → fill. More than one → push `EnrichmentIssue::AmbiguousDpt` and fill nothing. One that does not parse → leave `Absent` and report it as ambiguous with a single alternative, never `Malformed` (that variant means the *project file* said something unparsable).
- `flags`: map `"Enabled"` → `true`, `"Disabled"` → `false`, anything else → leave `Absent`; fill each of the five independently, `Absent` slots only.
- `size`: parse `"1 Bit"` → `ObjectSize::Bit(1)`, `"14 Bytes"` → `ObjectSize::Byte(14)`; set only when `size` is `None`.

`has_any_program` is `SELECT 1 FROM application_program LIMIT 1`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-productdb`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/knx-productdb
git commit -m "feat(productdb): enrich communication objects from the application program"
```

---

### Task 12: `knx-etsproj` hands manufacturer files out separately

**Files:**
- Modify: `crates/knx-etsproj/src/opaque.rs:60-115` and its test module
- Modify: `crates/knx-etsproj/src/lib.rs:32-56` and the `import_knxproj_bytes` body
- Modify: `crates/knx-etsproj/src/export/mod.rs:26-40`
- Modify: `crates/knx-etsproj/tests/roundtrip.rs`, `crates/knx-etsproj/tests/golden_reference_project.rs` (call sites only)

**Interfaces:**
- Consumes: nothing new.
- Produces: `pub struct ManufacturerFile { pub source_path: String, pub bytes: Vec<u8>, pub sha256: String, pub kind: OpaqueKind }`, `pub struct CollectedEntries { pub opaque: Vec<OpaqueEntry>, pub manufacturer: Vec<ManufacturerFile> }`, `collect_container_entries(...) -> Result<CollectedEntries, ContainerError>`, `ImportOutcome.manufacturer: Vec<ManufacturerFile>`, and the export warnings `ManufacturerDataFromProductDb { entries: usize }` and `MissingManufacturerData { source_path: String, sha256: String }`.

- [ ] **Step 1: Write the failing test**

Replace the first test in `crates/knx-etsproj/src/opaque.rs`'s test module and add two:

```rust
    #[test]
    fn manufacturer_files_are_handed_out_separately_from_opaque_entries() {
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let collected =
            collect_container_entries(&mut c, &["P-0512/0.xml", "P-0512/Project.xml"]).unwrap();
        // 36 entries less the two regenerated ones, split into the
        // manufacturer files (which now go to the product database) and
        // everything else (which stays in the project's opaque store).
        assert_eq!(collected.opaque.len() + collected.manufacturer.len(), 36);
        assert!(collected
            .manufacturer
            .iter()
            .all(|m| m.source_path.starts_with("M-")));
        assert!(collected
            .opaque
            .iter()
            .all(|e| !matches!(e.kind, OpaqueKind::ManufacturerData | OpaqueKind::Baggage)));
    }

    #[test]
    fn manufacturer_signatures_stay_in_the_opaque_store() {
        // M-0008.signature signs a container state, not a product; leaving
        // it here keeps the export path for signatures unchanged (spec §3).
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let collected = collect_container_entries(&mut c, &[]).unwrap();
        assert!(collected
            .opaque
            .iter()
            .any(|e| e.source_path == "M-0008.signature"));
        assert!(!collected
            .manufacturer
            .iter()
            .any(|m| m.source_path.ends_with(".signature")));
    }

    #[test]
    fn every_manufacturer_file_carries_the_hash_of_its_own_bytes() {
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let collected = collect_container_entries(&mut c, &[]).unwrap();
        assert!(collected
            .manufacturer
            .iter()
            .all(|m| m.sha256 == sha256_hex(&m.bytes)));
        // The vendor DLL travels with the manufacturer data, byte for byte.
        let dll = collected
            .manufacturer
            .iter()
            .find(|m| m.source_path.ends_with("econEts3.dll"))
            .unwrap();
        assert_eq!(dll.bytes.len(), 641536);
        assert_eq!(dll.kind, OpaqueKind::Baggage);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-etsproj opaque`
Expected: FAIL — `CollectedEntries` does not exist; the old `entries.len()` test no longer compiles.

- [ ] **Step 3: Write minimal implementation**

In `opaque.rs`, change the return type and route by `classify`:

```rust
/// A manufacturer file on its way to the product database (ADR-0005).
/// Same bytes and same hash as an `OpaqueEntry` would have carried — this
/// type exists so the destination is visible in the type system rather
/// than decided by a `match` in the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManufacturerFile {
    pub source_path: String,
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub kind: OpaqueKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollectedEntries {
    pub opaque: Vec<OpaqueEntry>,
    pub manufacturer: Vec<ManufacturerFile>,
}
```

and in the loop, `OpaqueKind::ManufacturerData | OpaqueKind::Baggage` push a `ManufacturerFile`, everything else an `OpaqueEntry`.

In `lib.rs`, add `manufacturer: Vec<ManufacturerFile>` to `ImportOutcome`, fill it from `CollectedEntries`, and extend the hand-written `Debug` impl with `.field("manufacturer", &format!("{} files", self.manufacturer.len()))` — never the bytes.

In `export/mod.rs`, rename the warning variant and add the new one:

```rust
    /// Manufacturer data was written from the shared product database
    /// rather than from the project file (ADR-0005). Always present when
    /// the export carried any.
    ManufacturerDataFromProductDb {
        entries: usize,
    },
    /// The project's manifest named a manufacturer file the product
    /// database could not supply. The container is written without it, and
    /// this says which one — never a silently incomplete archive.
    MissingManufacturerData {
        source_path: String,
        sha256: String,
    },
```

`export_knxproj` keeps counting `OpaqueKind::ManufacturerData` entries for the first warning; `MissingManufacturerData` is raised by `knx-app` (Task 14), which is the layer that knows the manifest.

- [ ] **Step 4: Run the whole crate's tests**

Run: `cargo test -p knx-etsproj`
Expected: PASS after updating the call sites in `tests/roundtrip.rs` and `tests/golden_reference_project.rs` — where they passed `outcome.opaque` to `export_knxproj`, they now pass a vector built from `outcome.opaque` plus each `ManufacturerFile` converted back into an `OpaqueEntry` (a two-line helper in each test's support module, mirroring what Task 14 does for real).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-etsproj
git commit -m "feat(etsproj): hand manufacturer files out separately from opaque entries"
```

---

### Task 13: `knx-store` schema v3 and the project manifest

**Files:**
- Create: `crates/knx-store/src/manifest.rs`
- Create: `crates/knx-store/fixtures/v3-empty.sqlite`
- Modify: `crates/knx-store/src/migration.rs:12`, its `migrations()` and its tests
- Modify: `crates/knx-store/src/lib.rs`
- Modify: `crates/knx-core/src/project.rs:19`

**Interfaces:**
- Consumes: nothing new.
- Produces: `knx_store::ManufacturerRef { source_path: String, sha256: String, len: i64, kind: String }`, `insert_manufacturer_refs(&Connection, &[ManufacturerRef]) -> Result<usize, rusqlite::Error>`, `load_manufacturer_refs(&Connection) -> Result<Vec<ManufacturerRef>, rusqlite::Error>`, `CURRENT_SCHEMA_VERSION = 3` in both `knx-store` and `knx-core`.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn a_fresh_file_migrates_to_version_three_and_has_the_manifest_table() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 3);
        assert_eq!(crate::manifest::load_manufacturer_refs(&conn).unwrap(), vec![]);
    }

    #[test]
    fn the_frozen_v2_fixture_migrates_forward_to_v3() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v2.sqlite");
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v2-empty.sqlite"),
            &path,
        )
        .unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 3);
        // The v2 opaque table survives the migration with its data intact.
        assert_eq!(crate::opaque::load_opaque(&conn).unwrap(), vec![]);
    }

    #[test]
    fn the_frozen_v3_fixture_still_opens() {
        let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v3-empty.sqlite");
        let conn = open_and_migrate(Path::new(fixture)).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 3);
    }
```

and in `manifest.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    fn refs() -> Vec<ManufacturerRef> {
        vec![
            ManufacturerRef {
                source_path: "M-0083/Catalog.xml".into(),
                sha256: "aa".into(),
                len: 21155,
                kind: "ManufacturerData".into(),
            },
            ManufacturerRef {
                source_path: "M-0008/Baggages/econEts3.dll".into(),
                sha256: "bb".into(),
                len: 641536,
                kind: "Baggage".into(),
            },
        ]
    }

    #[test]
    fn the_manifest_round_trips_in_insertion_order() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        assert_eq!(insert_manufacturer_refs(&conn, &refs()).unwrap(), 2);
        assert_eq!(load_manufacturer_refs(&conn).unwrap(), refs());
    }

    #[test]
    fn the_manifest_records_a_file_the_project_no_longer_carries() {
        // The point of the manifest: a project can still name what it was
        // imported with, even when the product database is gone (spec §6).
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        insert_manufacturer_refs(&conn, &refs()).unwrap();
        let loaded = load_manufacturer_refs(&conn).unwrap();
        assert_eq!(loaded[1].len, 641536);
        assert_eq!(loaded[1].sha256, "bb");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-store`
Expected: FAIL — `user_version` is 2, `manifest` module missing.

- [ ] **Step 3: Write minimal implementation**

`migration.rs`: `CURRENT_SCHEMA_VERSION = 3`, a third migration appended to `migrations()`:

```rust
/// v2 -> v3: the manufacturer manifest. Manufacturer data itself now lives
/// in the shared product database (ADR-0005); this table is what lets a
/// project name the files it was imported with even when that database is
/// absent — a nameable gap instead of silent loss.
fn migrate_v2_to_v3(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch(
        "CREATE TABLE manufacturer_ref (
             id          INTEGER PRIMARY KEY,
             source_path TEXT NOT NULL,
             sha256      TEXT NOT NULL,
             len         INTEGER NOT NULL,
             kind        TEXT NOT NULL
         ) STRICT;
         CREATE INDEX manufacturer_ref_sha256 ON manufacturer_ref (sha256);",
    )?;
    Ok(())
}
```

`manifest.rs` mirrors `opaque.rs`: one transaction, one prepared statement, `ORDER BY id` on load.

`knx-core/src/project.rs:19`: `pub const CURRENT_SCHEMA_VERSION: u32 = 3;` — the lockstep the same line already documents.

- [ ] **Step 4: Freeze the v3 fixture and run the tests**

```bash
cargo run -p xtask -- freeze-fixture crates/knx-store/fixtures/v3-empty.sqlite
cargo test -p knx-store -p knx-core
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/knx-store crates/knx-core
git commit -m "feat(store): schema v3 with the manufacturer manifest table"
```

---

### Task 14: `knx-app` wiring and export assembly

**Files:**
- Modify: `crates/knx-app/src/import.rs`
- Create: `crates/knx-app/src/export.rs`
- Modify: `crates/knx-app/src/lib.rs`, `crates/knx-app/Cargo.toml`
- Create: `crates/knx-app/tests/product_db.rs`

**Interfaces:**
- Consumes: `knx_etsproj::{ImportOutcome, ManufacturerFile, export::export_knxproj}`, `knx_store::{insert_opaque, insert_manufacturer_refs, load_manufacturer_refs}`, `knx_productdb::{ingest_file, ingest_master_data, enrich, load_source_file}`.
- Produces:
  - `pub struct ImportOptions<'a> { pub product_db: Option<&'a knx_productdb::Connection> }`
  - `import_ets_project(path, conn)` (unchanged signature, now delegating with `ImportOptions { product_db: None }`)
  - `import_ets_project_with(path, conn, options) -> Result<ImportedProject, AppError>`
  - `ImportedProject { project, report, opaque_entries, manufacturer_ingested: usize, manufacturer_skipped: usize, enrichment: Option<knx_productdb::EnrichmentReport> }`
  - `export::export_ets_project(&Project, &knx_store::Connection, Option<&knx_productdb::Connection>) -> Result<knx_etsproj::export::ExportOutcome, AppError>`, re-exported from `lib.rs` as `knx_app::export_ets_project` — which is the name the tests and the CLI use

- [ ] **Step 1: Write the failing test**

`crates/knx-app/tests/product_db.rs`:

```rust
use std::path::{Path, PathBuf};

fn reference_project_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("Unser Zuhause ets4 - 2025-12-15.knxproj")
}

#[test]
fn importing_with_a_product_db_ingests_manufacturer_files_and_keeps_them_out_of_the_opaque_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();

    let imported = knx_app::import_ets_project_with(
        &reference_project_path(),
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();

    assert!(imported.manufacturer_ingested > 0);
    let manifest = knx_store::load_manufacturer_refs(&store).unwrap();
    assert_eq!(manifest.len(), imported.manufacturer_ingested + imported.manufacturer_skipped);
    let opaque = knx_store::load_opaque(&store).unwrap();
    assert!(
        opaque.iter().all(|e| e.kind != "ManufacturerData" && e.kind != "Baggage"),
        "manufacturer data no longer lives in the project"
    );
}

#[test]
fn a_second_import_into_the_same_product_db_skips_every_file() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let first_store = knx_store::open_and_migrate(&dir.path().join("a.knxdb")).unwrap();
    let second_store = knx_store::open_and_migrate(&dir.path().join("b.knxdb")).unwrap();

    let first = knx_app::import_ets_project_with(
        &reference_project_path(),
        &first_store,
        knx_app::ImportOptions { product_db: Some(&products) },
    )
    .unwrap();
    let second = knx_app::import_ets_project_with(
        &reference_project_path(),
        &second_store,
        knx_app::ImportOptions { product_db: Some(&products) },
    )
    .unwrap();

    assert!(first.manufacturer_ingested > 0);
    assert_eq!(second.manufacturer_ingested, 0);
    assert_eq!(second.manufacturer_skipped, first.manufacturer_ingested);
}

#[test]
fn export_is_byte_identical_with_and_without_the_product_database() {
    // The proof that routing manufacturer data through the product
    // database changes nothing about what we write back (spec §9).
    let dir = tempfile::tempdir().unwrap();
    let with_store = knx_store::open_and_migrate(&dir.path().join("with.knxdb")).unwrap();
    let without_store = knx_store::open_and_migrate(&dir.path().join("without.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();

    let with = knx_app::import_ets_project_with(
        &reference_project_path(),
        &with_store,
        knx_app::ImportOptions { product_db: Some(&products) },
    )
    .unwrap();
    let without = knx_app::import_ets_project_with(
        &reference_project_path(),
        &without_store,
        knx_app::ImportOptions { product_db: None },
    )
    .unwrap();

    let a = knx_app::export_ets_project(&with.project, &with_store, Some(&products)).unwrap();
    let b = knx_app::export_ets_project(&without.project, &without_store, None).unwrap();
    assert_eq!(a.bytes, b.bytes);
}

#[test]
fn a_project_opens_and_names_its_gap_when_the_product_database_is_gone() {
    let dir = tempfile::tempdir().unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products_path = dir.path().join("products.sqlite");
    {
        let products = knx_productdb::open_and_migrate(&products_path).unwrap();
        knx_app::import_ets_project_with(
            &reference_project_path(),
            &store,
            knx_app::ImportOptions { product_db: Some(&products) },
        )
        .unwrap();
    }
    std::fs::remove_file(&products_path).unwrap();

    // Exporting without the product database: every manifest entry is
    // named as missing, and the export still happens.
    let outcome = knx_app::export_ets_project(
        &knx_app::import_ets_project_with(
            &reference_project_path(),
            &knx_store::open_and_migrate(&dir.path().join("q.knxdb")).unwrap(),
            knx_app::ImportOptions { product_db: None },
        )
        .unwrap()
        .project,
        &store,
        None,
    )
    .unwrap();

    let missing = outcome
        .warnings
        .iter()
        .filter(|w| matches!(w, knx_etsproj::export::ExportWarning::MissingManufacturerData { .. }))
        .count();
    assert_eq!(missing, knx_store::load_manufacturer_refs(&store).unwrap().len());
    assert!(!outcome.bytes.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-app --test product_db`
Expected: FAIL — `import_ets_project_with` does not exist; `knx-app` has no `knx-productdb` dependency.

- [ ] **Step 3: Write minimal implementation**

`crates/knx-app/Cargo.toml` gains `knx-productdb.workspace = true`.

`import.rs`:

```rust
/// What a caller wants done with manufacturer data. `None` runs exactly
/// the Session 3 path: the files go into the project's opaque store and
/// nothing is enriched. That path stays supported, and stays tested,
/// because a user who does not want a shared database must still get a
/// complete, exportable project.
#[derive(Default)]
pub struct ImportOptions<'a> {
    pub product_db: Option<&'a knx_productdb::Connection>,
}

pub fn import_ets_project_with(
    path: &Path,
    conn: &Connection,
    options: ImportOptions<'_>,
) -> Result<ImportedProject, AppError> {
    let mut outcome = knx_etsproj::import_knxproj(path)?;

    // The manifest is written whichever way the manufacturer files are
    // stored: it describes what the project was imported with, not where
    // the bytes ended up.
    let manifest: Vec<ManufacturerRef> = outcome
        .manufacturer
        .iter()
        .map(|m| ManufacturerRef {
            source_path: m.source_path.clone(),
            sha256: m.sha256.clone(),
            len: m.bytes.len() as i64,
            kind: format!("{:?}", m.kind),
        })
        .collect();

    let mut stored: Vec<StoredOpaqueEntry> = outcome.opaque.iter().map(to_stored).collect();
    let mut ingested = 0usize;
    let mut skipped = 0usize;
    let mut enrichment = None;

    match options.product_db {
        Some(products) => {
            for file in &outcome.manufacturer {
                match knx_productdb::ingest_file(products, &file.source_path, &file.bytes)? {
                    knx_productdb::IngestOutcome::Ingested { .. } => ingested += 1,
                    knx_productdb::IngestOutcome::Skipped { .. } => skipped += 1,
                }
            }
            if let Some(master) = outcome
                .opaque
                .iter()
                .find(|e| e.kind == knx_etsproj::opaque::OpaqueKind::MasterData)
            {
                knx_productdb::ingest_master_data(products, &master.bytes)?;
            }
            enrichment = Some(knx_productdb::enrich(&mut outcome.project, products)?);
        }
        None => stored.extend(outcome.manufacturer.iter().map(manufacturer_to_stored)),
    }

    let opaque_entries = stored.len();
    insert_opaque(conn, &stored)?;
    insert_manufacturer_refs(conn, &manifest)?;

    Ok(ImportedProject {
        project: outcome.project,
        report: outcome.report,
        opaque_entries,
        manufacturer_ingested: ingested,
        manufacturer_skipped: skipped,
        enrichment,
    })
}
```

`AppError` gains a `ProductDb(knx_productdb::ProductDbError)` variant with its `From` impl and `Display` arm.

`export.rs` assembles the entry list and raises the missing-file warnings:

```rust
/// Rebuilds the full opaque-entry list — stored entries plus every
/// manufacturer file fetched back out of the product database — and hands
/// it to `knx-etsproj`'s writer, whose signature does not change.
pub fn export_ets_project(
    project: &knx_core::Project,
    conn: &Connection,
    products: Option<&knx_productdb::Connection>,
) -> Result<ExportOutcome, AppError> {
    let mut entries: Vec<OpaqueEntry> = load_opaque(conn)?.iter().map(from_stored).collect();
    let mut missing = Vec::new();

    for reference in load_manufacturer_refs(conn)? {
        let bytes = match products {
            Some(products) => knx_productdb::load_source_file(products, &reference.sha256)?,
            None => None,
        };
        match bytes {
            Some(bytes) => entries.push(OpaqueEntry {
                source_path: reference.source_path,
                xpath: String::new(),
                kind: kind_from_str(&reference.kind),
                name: String::new(),
                bytes,
                sha256: reference.sha256,
            }),
            None => missing.push(ExportWarning::MissingManufacturerData {
                source_path: reference.source_path,
                sha256: reference.sha256,
            }),
        }
    }

    let mut outcome = knx_etsproj::export::export_knxproj(project, &entries)?;
    outcome.warnings.extend(missing);
    Ok(outcome)
}
```

`from_stored` is the inverse of the existing `to_stored`; `kind_from_str` maps the stored label back to its `OpaqueKind`, defaulting to `ContainerEntry` for a label this build does not know — a forward-compatibility rule, not a silent drop, since the bytes are written either way.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-app`
Expected: PASS (4 integration tests plus the existing `import_service.rs` suite).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-app
git commit -m "feat(app): route manufacturer data through the product database on import and export"
```

---

### Task 15: CLI

**Files:**
- Modify: `apps/knx-cli/src/main.rs`, `apps/knx-cli/Cargo.toml`
- Modify: `apps/knx-cli/tests/cli_import.rs`

**Interfaces:**
- Consumes: `knx_app::{import_ets_project_with, ImportOptions}`, `knx_productdb::{open_and_migrate, default_path, verify, query::{manufacturers, programs}}`.
- Produces: the `--product-db` / `--no-product-db` flags and the `knx products` subcommand.

- [ ] **Step 1: Write the failing test**

Add to `apps/knx-cli/tests/cli_import.rs`:

```rust
#[test]
fn import_with_a_product_db_reports_what_it_ingested() {
    let dir = tempfile::tempdir().unwrap();
    let products = dir.path().join("products.sqlite");
    let out = run_knx(&[
        "import",
        reference_project_path().to_str().unwrap(),
        "--product-db",
        products.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("manufacturer file"), "{stdout}");
    assert!(products.exists());
}

#[test]
fn import_with_no_product_db_keeps_manufacturer_data_in_the_project() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("p.knxdb");
    let out = run_knx(&[
        "import",
        reference_project_path().to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
        "--no-product-db",
    ]);
    assert_eq!(out.status.code(), Some(0));
    let conn = knx_store::open_and_migrate(&store).unwrap();
    assert!(knx_store::load_opaque(&conn)
        .unwrap()
        .iter()
        .any(|e| e.kind == "ManufacturerData"));
}

#[test]
fn product_db_and_no_product_db_together_are_a_usage_error() {
    let out = run_knx(&[
        "import",
        reference_project_path().to_str().unwrap(),
        "--product-db",
        "/tmp/x.sqlite",
        "--no-product-db",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr).unwrap().contains("--no-product-db"));
}

#[test]
fn products_list_prints_what_was_ingested() {
    let dir = tempfile::tempdir().unwrap();
    let products = dir.path().join("products.sqlite");
    run_knx(&[
        "products",
        "ingest",
        reference_project_path().to_str().unwrap(),
        "--product-db",
        products.to_str().unwrap(),
    ]);
    let out = run_knx(&["products", "list", "--product-db", products.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("M-0083"), "{stdout}");
}

#[test]
fn products_verify_is_clean_after_an_ingest() {
    let dir = tempfile::tempdir().unwrap();
    let products = dir.path().join("products.sqlite");
    run_knx(&[
        "products",
        "ingest",
        reference_project_path().to_str().unwrap(),
        "--product-db",
        products.to_str().unwrap(),
    ]);
    let out = run_knx(&["products", "verify", "--product-db", products.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout).unwrap().contains("0 mismatch"));
}
```

(`run_knx` and `reference_project_path` already exist in that test file; reuse them.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-cli`
Expected: FAIL — unknown flag `--product-db`, unknown command `products`.

- [ ] **Step 3: Write minimal implementation**

Extend `USAGE`, `ImportArgs` (`product_db: Option<String>`, `no_product_db: bool`) and `parse_import_args`, rejecting the combination of both flags. Resolve the path: explicit `--product-db`, else `knx_productdb::default_path()`, else a usage error naming `XDG_DATA_HOME`. Open with `knx_productdb::open_and_migrate`, then call `knx_app::import_ets_project_with`.

Extend `print_summary` with one line, printed only when a product database was used:

```rust
    if let Some(enrichment) = &imported.enrichment {
        println!(
            "  {} manufacturer file(s) ingested, {} already known",
            imported.manufacturer_ingested, imported.manufacturer_skipped
        );
        println!(
            "  {} communication object(s) enriched from {} application program(s), {} issue(s)",
            enrichment.com_objects_enriched,
            enrichment.devices_resolved,
            enrichment.issues.len()
        );
    }
```

Add `run_products` with the four subcommands: `list` prints manufacturers and their programs; `ingest` opens the `.knxproj` through `knx_etsproj::import_knxproj` and feeds `outcome.manufacturer` to `knx_productdb::ingest_file`; `show <program-id>` prints the program head plus its communication-object and parameter counts; `verify` prints `"<n> mismatch(es)"` and exits 1 when `n > 0`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-cli`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/knx-cli
git commit -m "feat(cli): --product-db on import and the knx products subcommand"
```

---

### Task 16: Golden, malformed and oracle suites

**Files:**
- Create: `crates/knx-productdb/tests/golden_reference_products.rs`
- Create: `crates/knx-productdb/tests/malformed_input.rs`
- Modify: `crates/knx-productdb/Cargo.toml` (dev-dependency `zip` for reading the reference container without depending on `knx-etsproj`)

**Interfaces:**
- Consumes: the whole crate's public API.
- Produces: measured counts that later sessions can regress against.

- [ ] **Step 1: Write the failing test**

`tests/golden_reference_products.rs`:

```rust
//! Ingest of the committed ETS4 reference project's manufacturer data.
//!
//! Every number here was measured by running the ingest, never guessed. A
//! change to any of them is either a parser bug or a deliberate change
//! that must be explained in the commit that makes it.

use std::path::{Path, PathBuf};

fn reference_project_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("Unser Zuhause ets4 - 2025-12-15.knxproj")
}

/// Reads the container directly with `zip`, so this crate's tests stay
/// independent of `knx-etsproj` exactly as its code is (ADR-0011).
fn manufacturer_files() -> Vec<(String, Vec<u8>)> {
    let file = std::fs::File::open(reference_project_path()).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut out = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let name = entry.name().to_string();
        if name.starts_with("M-") && !name.ends_with(".signature") {
            let mut bytes = Vec::new();
            std::io::copy(&mut entry, &mut bytes).unwrap();
            out.push((name, bytes));
        }
    }
    out
}

fn ingest_all() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    for (path, bytes) in manufacturer_files() {
        knx_productdb::ingest_file(&conn, &path, &bytes).unwrap();
    }
    (dir, conn)
}

fn count(conn: &knx_productdb::Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn the_reference_projects_manufacturer_data_ingests_completely() {
    let (_dir, conn) = ingest_all();
    assert_eq!(count(&conn, "SELECT count(*) FROM manufacturer"), 4);
    assert_eq!(count(&conn, "SELECT count(*) FROM source_file"), 24); // <measured>
    assert_eq!(count(&conn, "SELECT count(*) FROM application_program"), 12);
    assert_eq!(count(&conn, "SELECT count(*) FROM com_object"), 0); // <measured>
    assert_eq!(count(&conn, "SELECT count(*) FROM com_object_ref"), 0); // <measured>
    assert_eq!(count(&conn, "SELECT count(*) FROM parameter"), 0); // <measured>
    assert_eq!(count(&conn, "SELECT count(*) FROM parameter_type_enum"), 0); // <measured>
    assert_eq!(count(&conn, "SELECT count(*) FROM translation"), 0); // <measured>
}

#[test]
fn every_blob_verifies_against_its_own_hash() {
    let (_dir, conn) = ingest_all();
    assert_eq!(knx_productdb::verify(&conn).unwrap(), vec![]);
}

#[test]
fn a_second_ingest_of_the_same_files_stores_nothing_new() {
    let (_dir, conn) = ingest_all();
    let before = count(&conn, "SELECT count(*) FROM source_file");
    for (path, bytes) in manufacturer_files() {
        assert!(matches!(
            knx_productdb::ingest_file(&conn, &path, &bytes).unwrap(),
            knx_productdb::IngestOutcome::Skipped { .. }
        ));
    }
    assert_eq!(count(&conn, "SELECT count(*) FROM source_file"), before);
}

#[test]
fn the_unknown_construct_table_is_a_short_list_not_a_flood() {
    // Not zero — this is one manufacturer sample of four vendors, and an
    // unmodelled attribute is expected. What matters is that it is
    // reported and bounded, and that every one of them is on record.
    let (_dir, conn) = ingest_all();
    let distinct = count(&conn, "SELECT count(*) FROM ingest_unknown");
    assert!(distinct < 200, "{distinct} distinct unknown constructs");
}
```

`tests/malformed_input.rs`:

```rust
//! Malformed and hostile manufacturer input. None of it may panic, and
//! none of it may leave the database half-written.

fn db() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

const PROGRAM: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1"
 MaskVersion="MV-0010"><Static><ComObjectTable>
<ComObject Id="A-1_O-0" Number="0" Text="T" ObjectSize="1 Bit" />
</ComObjectTable></Static></ApplicationProgram></ApplicationPrograms>
</Manufacturer></ManufacturerData></KNX>"#;

#[test]
fn a_truncated_program_leaves_nothing_behind() {
    let (_dir, conn) = db();
    assert!(knx_productdb::ingest_file(&conn, "M-0001/A.xml", &PROGRAM[..PROGRAM.len() / 2]).is_err());
    let rows: i64 = conn
        .query_row("SELECT count(*) FROM source_file", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn an_empty_file_is_stored_and_classified_as_unrecognized() {
    let (_dir, conn) = db();
    let out = knx_productdb::ingest_file(&conn, "M-0001/Empty.xml", b"").unwrap();
    assert!(matches!(
        out,
        knx_productdb::IngestOutcome::Ingested { kind: knx_productdb::FileKind::Unrecognized, .. }
    ));
}

#[test]
fn deeply_nested_xml_does_not_blow_the_stack() {
    // The parsers are iterative, not recursive; 10,000 levels is the same
    // depth knx-etsproj's own malformed suite uses.
    let (_dir, conn) = db();
    let mut xml = String::from(r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData>"#);
    for _ in 0..10_000 {
        xml.push_str("<Nested>");
    }
    for _ in 0..10_000 {
        xml.push_str("</Nested>");
    }
    xml.push_str("</ManufacturerData></KNX>");
    let _ = knx_productdb::ingest_file(&conn, "M-0001/Deep.xml", xml.as_bytes());
}

#[test]
fn the_same_program_id_from_two_different_files_keeps_the_first_and_records_the_conflict() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-0001/A.xml", PROGRAM).unwrap();
    let changed = String::from_utf8(PROGRAM.to_vec())
        .unwrap()
        .replace("Name=\"P\"", "Name=\"P2\"");
    knx_productdb::ingest_file(&conn, "M-0001/A2.xml", changed.as_bytes()).unwrap();
    let name: String = conn
        .query_row("SELECT name FROM application_program WHERE id = 'A-1'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(name, "P");
    let conflicts: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind = 'IdConflict'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(conflicts, 1);
}
```

- [ ] **Step 2: Run the golden test to read the real numbers**

Run: `cargo test -p knx-productdb --test golden_reference_products -- --nocapture`
Expected: FAIL on the `<measured>` assertions. Read each actual number from the failure output, put it in the test, and delete the `// <measured>` marker.

- [ ] **Step 3: Re-run and confirm every count is now asserted**

Run: `cargo test -p knx-productdb`
Expected: PASS.

- [ ] **Step 4: Add the oracle comparison**

Add to `tests/golden_reference_products.rs` a test that reads `devices.json` / `project_dump.json` at the workspace root (the committed `xknxproject` output) and compares communication-object texts and datapoint types for the programs both tools read, skipping the fields RESEARCH §7.1 records as lossy. `xknxproject` is GPL-2.0-only: it is a committed *output file* being read here, never a dependency — `cargo deny check` must still pass unchanged.

- [ ] **Step 5: Commit**

```bash
git add crates/knx-productdb
git commit -m "test(productdb): golden ingest counts, malformed input and the xknxproject oracle"
```

---

### Task 17: Documentation

**Files:**
- Create: `docs/adr/0011-product-database-storage.md`, `docs/adr/0012-enrichment-into-absent-slots.md`
- Modify: `docs/adr/README.md`, `docs/IMPORT_EXPORT.md` §10, `docs/KNOWN_LIMITATIONS.md` §12, `docs/ARCHITECTURE.md`, `docs/DATA_MODEL.md` §3, `docs/COMPATIBILITY.md`, `docs/IMPLEMENTATION_STATUS.md`, `docs/ROADMAP.md`

**Interfaces:**
- Consumes: the measured numbers from Task 16 and the test names from Tasks 1–16.
- Produces: documentation that matches the code, which is the only kind this repository keeps.

- [ ] **Step 1: Write the two ADRs**

`0011-product-database-storage.md` — context: 22 MB of manufacturer data per project, duplicated per project today (KNOWN_LIMITATIONS §12); decision: a separate SQLite database storing each file *both* as a content-hashed blob and as parsed tables, with the blob as the integrity guarantee and the content hash as identity; alternatives considered: parsed-only (loses the export path and everything unmodelled), blob-only (no resolution possible), bytes left in the project (ADR-0005 half-implemented); consequences: export reconstructs container entries from the database, a missing database is a nameable gap, the `Dynamic` tree can stay unparsed without data loss.

`0012-enrichment-into-absent-slots.md` — context: `Override<T>` has one slot per attribute and 497 reference-project communication objects carry `DatapointType=""`; decision: enrichment fills only `Override::Absent`; alternatives: overwrite (changes the exported file), extend `Override<T>` into a layer stack (domain-model change with a migration, deferred); consequences: program values for `Empty` slots stay queryable in the product database and are not in the model, recorded as a limitation.

Add both to `docs/adr/README.md`'s index.

- [ ] **Step 2: Update the reference documents**

- `IMPORT_EXPORT.md` §10: rewrite from "the target design" to what now happens, describing the manifest, the export reconstruction, and `MissingManufacturerData`.
- `KNOWN_LIMITATIONS.md` §12: lifted for communication objects; restate what remains — parameter *interpretation* (the `Dynamic` tree and `when/@test`), program values behind `Empty` instance slots, ambiguous DPT lists — each with its lift condition.
- `ARCHITECTURE.md`: `knx-productdb`'s responsibility in §3, the third `check-layering` root in §4.
- `DATA_MODEL.md` §3: the `Program`/`ProgramRef` layers are populated by `knx_productdb::enrich`, and by what rule.
- `COMPATIBILITY.md`: new verified rows, each naming its test — ingest completeness, blob hash equality, export equality with and without the database, degradation.
- `IMPLEMENTATION_STATUS.md`: Session 4 marked done for the product database; what was deliberately deferred (entity persistence, the `when/@test` spike, `Dynamic`, `.knxprod`, schema 23) named as such.
- `ROADMAP.md`: Session 4's remaining deliverables listed as their own next cycles.

- [ ] **Step 3: Check every number and test name you wrote**

Run: `cargo test --workspace 2>&1 | tail -40` and confirm each test name quoted in `COMPATIBILITY.md` exists and passes. A compatibility claim naming a test that does not exist is worse than no claim.

- [ ] **Step 4: Run the full gate**

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- check-layering
cargo deny check
```
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add docs
git commit -m "docs(session4): product database, its two ADRs, and the lifted limitation"
```
