# Standalone Product Database Installation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Install the supplied readable manufacturer `.knxprod` archives into
the shared product database and use their catalog to create devices with honest
diagnostics.

**Architecture:** A new `knx-productdb` package reader validates and persists a
complete ZIP package before its existing parser creates normalized rows. CLI,
server, and web remain thin callers of that owning service and existing catalog
creation commands. Package-level evidence and all collision/diagnostic state
stay in the product database.

**Tech Stack:** Rust, `rusqlite`, `zip`, `quick-xml`, Axum multipart, React,
Vitest.

**Spec:** `docs/superpowers/specs/2026-09-09-standalone-product-database-install-design.md`

## Global Constraints

- The public implementation claim is restricted to the tested supplied,
  readable scheme-11 and scheme-20 `.knxprod` corpus.
- Do not decrypt, parse, or infer `.vd2` semantics; report its encrypted legacy
  format explicitly.
- Product package ingestion belongs in `knx-productdb`; it must not depend on
  `knx-etsproj`, `knx-store`, server, or UI crates.
- Store every accepted package/member byte losslessly and validate the complete
  archive before committing normalized data.
- Preserve first-winner logical identities and expose conflicting second inputs;
  never rewrite rows whose provenance belongs to an earlier hash.
- A product template is not a configured device: Dynamic/module activation and
  parameter configuration remain named limitations.
- Use test-first cycles; run the relevant focused test before and after every
  production change.

---

### Task 1: Preserve normalized first-winner provenance

**Files:**
- Modify: `crates/knx-productdb/src/parse/hardware.rs`
- Modify: `crates/knx-productdb/src/parse/catalog.rs`
- Modify: `crates/knx-productdb/src/ingest.rs`
- Modify: `crates/knx-productdb/tests/malformed_input.rs`

**Interfaces:**
- Produces: `IdConflict` records for catalog/hardware/product/H2P collisions,
  returned through `IngestOutcome::Ingested { conflicts, .. }`.
- Preserves: first accepted row and its `source_sha256` for every logical ID.

- [ ] **Step 1: Write failing collision tests.**

  Add an integration test that ingests two byte-different `Hardware.xml`
  fixtures with the same `Hardware2Program/@Id` but different
  `ApplicationProgramRef/@RefId`. Assert that the first reference and source
  hash still resolve, both source blobs exist, and the second result contains an
  `IdConflict`. Add equivalent catalog-item and product collision assertions.

- [ ] **Step 2: Run the new test and verify that it fails because the existing
  H2P update overwrites the first reference.**

  Run: `cargo test -p knx-productdb --test malformed_input conflicting_hardware2program`

- [ ] **Step 3: Make parser inserts conflict-aware.**

  Replace unconditional H2P/registration updates with insert-or-first-winner
  logic. Before ignoring an existing identity, compare the candidate semantic
  fields with the persisted row and collect `IdConflict { id, first_sha256,
  second_sha256, ... }` if they differ. Thread collectors through catalog and
  hardware parsers exactly as `parse/program.rs` does; persist them through
  `report::insert_conflicts` in the same transaction.

- [ ] **Step 4: Run focused tests.**

  Run: `cargo test -p knx-productdb --test malformed_input`

- [ ] **Step 5: Commit the isolated integrity fix.**

  Run: `git add crates/knx-productdb && git commit -m "fix(productdb): preserve first-winner hardware provenance"`

### Task 2: Add an atomic standalone `.knxprod` package service

**Files:**
- Modify: `crates/knx-productdb/Cargo.toml`
- Create: `crates/knx-productdb/src/package.rs`
- Modify: `crates/knx-productdb/src/lib.rs`
- Modify: `crates/knx-productdb/src/migration.rs`
- Modify: `crates/knx-productdb/src/ingest.rs`
- Create: `crates/knx-productdb/tests/standalone_packages.rs`

**Interfaces:**
- Produces: `install_package(conn: &Connection, source_name: &str, bytes:
  &[u8]) -> Result<PackageInstallReport, ProductDbError>`.
- Produces: typed `ProductDbError::Package { source_name, kind, detail,
  sha256, len }` for unsupported/malformed/encrypted package outcomes.
- Persists: package SHA-256 and each ordered member's path, role, byte hash,
  and length in product-db v2 tables.

- [ ] **Step 1: Write failing package acceptance tests.**

  In `standalone_packages.rs`, locate the workspace corpus relative to
  `CARGO_MANIFEST_DIR`. For every `.knxprod`, assert either successful package
  report with its detected scheme and non-empty catalog rows or the exact
  typed unsupported error. Assert the three scheme-11 and two scheme-20
  fixtures install from an empty DB, save every non-directory member, verify
  each member blob hash, and are idempotent on a second install. Add synthetic
  ZIP tests for duplicate names, `../` traversal, missing `knx_master.xml`,
  an oversized declared entry, truncation, and encrypted member; assert zero
  `source_package`, `source_file`, and parsed rows after failure.

- [ ] **Step 2: Run the corpus test and verify it fails because no standalone
  package API exists.**

  Run: `cargo test -p knx-productdb --test standalone_packages installs_the_readable_corpus`

- [ ] **Step 3: Add package metadata migration and bounded inventory reader.**

  Bump `CURRENT_PRODUCTDB_VERSION` to 2 and add a forward v1→v2 migration for
  `source_package` and `source_package_member`. In `package.rs`, open ZIP
  bytes, reject encryption/path ambiguity/unsafe duplicate paths and declared
  members larger than the established 64 MiB limit, require one master XML,
  read its KNX namespace, and accept only schemes 11 and 20. Classify each
  member by its validated `M-xxxx/` path and XML content; retain signatures,
  baggage, and unrecognized member bytes rather than dropping them.

- [ ] **Step 4: Refactor per-file ingest behind one transaction.**

  Extract the existing store/classify/parse body so package installation can
  call it using one transaction for every validated member. Insert package and
  member evidence in that same transaction, ingest master data once, and
  return counts, unknown constructs, conflicts, member inventory, and skipped
  status. Keep `ingest_file` as the existing single-file public compatibility
  wrapper with its present all-or-nothing guarantee.

- [ ] **Step 5: Run package and existing product-db suites.**

  Run: `cargo test -p knx-productdb`

- [ ] **Step 6: Commit the product-container service.**

  Run: `git add crates/knx-productdb && git commit -m "feat(productdb): install standalone knxprod packages"`

### Task 3: Expose truthful CLI and server installation results

**Files:**
- Modify: `apps/knx-cli/src/main.rs`
- Modify: `apps/knx-cli/tests/cli_products.rs` (or create it if the command
  tests live inline)
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Modify: `apps/knx-server/tests/http_device_routes.rs`
- Create: `apps/knx-server/tests/http_product_install.rs`

**Interfaces:**
- Produces: CLI `knx products ingest <file.knxprod|file.vd2>` using
  `knx_productdb::install_package` instead of `knx_etsproj::import_knxproj`.
- Produces: `POST /api/catalog/install` multipart response carrying
  `PackageInstallReport` or a 400 typed reason.
- Consumes: the Task 2 package service only; neither frontend nor server
  reimplements archive interpretation.

- [ ] **Step 1: Write failing CLI/server tests.**

  Assert the CLI invokes package ingest for a supplied scheme-11 corpus file
  and emits its scheme/member/ingested summary. In Axum tests, upload a small
  valid synthetic package and assert a 200 report plus discoverable catalog
  rows; upload the supplied `.vd2` and a malformed ZIP and assert 400 bodies
  name `encrypted legacy database` and the package error respectively.

- [ ] **Step 2: Run the new focused tests and observe the current project-part
  error path.**

  Run: `cargo test -p knx-server --test http_product_install`

- [ ] **Step 3: Implement thin callers and response DTOs.**

  Parse the CLI positional argument as a product package, read bytes with a
  named IO error, call `install_package`, and print report fields without
  claiming generic `.knxprod` support. Add one server domain function that
  locks `AppState.product_db`, calls the same API, and maps `ProductDbError`
  to a non-internal `ApiError::bad_request`. Keep multipart byte limits at or
  below the package reader's bounds.

- [ ] **Step 4: Run focused CLI/server tests.**

  Run: `cargo test -p knx-cli && cargo test -p knx-server --test http_product_install --test http_device_routes`

- [ ] **Step 5: Commit CLI and HTTP exposure.**

  Run: `git add apps/knx-cli apps/knx-server && git commit -m "feat(catalog): install manufacturer product packages"`

### Task 4: Make catalog creation and diagnostics honest in the web flow

**Files:**
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Modify: `apps/knx-server/tests/http_device_routes.rs`
- Modify: `apps/knx-web/src/api.ts`
- Modify: `apps/knx-web/src/CatalogBrowser.tsx`
- Modify: `apps/knx-web/src/api.test.ts`
- Create or modify: `apps/knx-web/src/CatalogBrowser.test.tsx`

**Interfaces:**
- Produces: `CreateDeviceResponse { tree, diagnostics }` where each diagnostic
  has machine-readable kind and user-visible detail.
- Produces: `installProductPackage(file)` web API helper and install report
  rendering in `CatalogBrowser`.
- Preserves: `Command::CreateDevice` and its undo/redo/persistence behavior.

- [ ] **Step 1: Write failing diagnostics and UI tests.**

  Add server tests for a catalog item with a missing H2P/program relation
  (named 400, no device created), a genuinely programless product (success
  with programless diagnostic), and ambiguous DPT enrichment (success with a
  returned diagnostic). Add component tests that mock the package-install and
  catalog endpoints, assert a failed install error remains visible, a success
  reloads manufacturer/item results, and creation diagnostics render before
  close.

- [ ] **Step 2: Run the focused tests and verify current behavior discards
  enrichment issues or returns only a tree.**

  Run: `cargo test -p knx-server --test http_device_routes && cd apps/knx-web && npm test -- CatalogBrowser`

- [ ] **Step 3: Implement diagnostic response and browser install control.**

  Replace the raw tree response only at `POST /api/devices` with a DTO while
  updating its existing frontend caller. Validate the catalog→product→H2P→
  program chain before command application; only a database-evidenced
  programless product bypasses program seeding. Convert every collected
  `EnrichmentIssue` into response diagnostics. In `CatalogBrowser`, use the
  existing filesystem picker, display package errors/report summaries, refresh
  catalog data after successful install, and render creation diagnostics in the
  same modal.

- [ ] **Step 4: Run focused Rust and web checks.**

  Run: `cargo test -p knx-server --test http_device_routes --test http_product_install && cd apps/knx-web && npm test -- CatalogBrowser api.test.ts && npm run build`

- [ ] **Step 5: Commit the end-to-end caller changes.**

  Run: `git add apps/knx-server apps/knx-web && git commit -m "feat(catalog): report product install and creation diagnostics"`

### Task 5: Reconcile evidence and compatibility documentation

**Files:**
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`
- Modify: `docs/ROADMAP.md`
- Modify: `docs/GAP_ANALYSIS_ETS.md`
- Modify: `docs/COMPATIBILITY.md`
- Create: `.ai/logs/YYYY-MM-DD_codex_standalone_product_database_install.md`
- Modify: `.ai/CURRENT_STATE.md`

**Interfaces:**
- Consumes: passing corpus, server, and web tests from Tasks 1--4.
- Produces: an evidence matrix that names each supplied fixture, detected
  scheme, outcome, test, and remaining external blocker.

- [ ] **Step 1: Write documentation assertions into the actual matrix.**

  Record the three scheme-11 and two scheme-20 standalone archives as verified
  only when Task 2 tests pass. Record `.vd2` as an encrypted legacy external
  blocker, not an untested general failure. Replace stale B1/B2/B3/B5/B6/B7/D3
  claims in `GAP_ANALYSIS_ETS.md`, update the T2 reference in limitation #35,
  and correct historic status snapshots/counts from current code.

- [ ] **Step 2: Run documentation consistency searches.**

  Run: `rg -n "future catalog-browser|no device catalog browser|scheme 11 is readable|direct.*knxprod.*out of v1" docs`

  Expected: every remaining match is historical or an explicit, current
  limitation with a lift condition.

- [ ] **Step 3: Record exact proof and remaining boundaries.**

  Cite `Project Schema23 v01.00.00.md` §4.2.2--§4.2.3 and
  `03_01_01 Architecture v03.00.02 AS.md` §6.2, the corpus test names, and
  the unsupported `.vd2` reason. Do not say "KNX certified" or "full ETS
  compatibility".

- [ ] **Step 4: Run final relevant gates.**

  Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo run -p xtask -- check-layering && cd apps/knx-web && npm test && npm run build`

- [ ] **Step 5: Commit reconciliation and handover.**

  Run: `git add docs .ai && git commit -m "docs: record standalone product database compatibility"`

## Plan self-review

- Spec coverage: Task 1 protects pre-existing first-winner integrity; Task 2
  supplies bounded/atomic lossless package ingestion; Task 3 supplies CLI and
  HTTP errors; Task 4 proves catalog-to-device/browser outcomes and diagnostics;
  Task 5 records the evidence matrix and remaining `.vd2`/Dynamic boundaries.
- Placeholder scan: no task relies on unspecified parser semantics; unknown or
  encrypted formats have exact reject/report behavior.
- Type consistency: Tasks 3--4 consume `PackageInstallReport` from Task 2 and
  `CreateDeviceResponse` is introduced and consumed in Task 4 only.
