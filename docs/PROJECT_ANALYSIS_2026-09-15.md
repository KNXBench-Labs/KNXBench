# KNXBench project analysis — 2026-09-15

## Purpose and scope

This document reviews the repository as an **alpha project still under active
development**. Missing future functionality is therefore not automatically a
defect. The review distinguishes between:

- deliberate scope cuts or evidence-dependent future work;
- risks in behaviour that already exists;
- release and maintenance work that should happen before a broader beta.

The review covered the workspace architecture, the domain and persistence
boundaries, ETS import/export and compatibility evidence, product-data ingest,
the HTTP/Tauri/React surfaces, CI, tests, ADRs, the roadmap, and the documented
limitations. It was a repository review, not a security penetration test and
not a test against a physical KNX installation.

## Executive summary

KNXBench has a stronger foundation than its alpha label suggests. The crate
boundaries are deliberate, the normalized model preserves provenance, unknown
external data has an opaque passthrough path, mutations use commands with
undo/redo, SQLite writes are transactional, and compatibility statements are
usually tied to named evidence. The automated suite is broad and the checked
quality gates pass.

The next work should nevertheless favour **hardening existing behaviour over
adding more surface area**. The most important risks are:

1. opening or importing a project can replace unsaved edits without the guard
   already used by “New project”;
2. the network server accepts unconfined absolute paths because it shares the
   desktop path policy, while binding to all interfaces without authentication;
3. one logical project session is split across several independent mutexes and
   can be observed or saved in a mixed state under concurrent requests;
4. some known import/export paths still discard or rewrite information;
5. the current documentation has drifted enough that multiple source-of-truth
   documents contradict implemented behaviour.

These are normal alpha-stage findings. They become release blockers only when
KNXBench is presented to less controlled users or connected to live hardware.

## What is already working well

### Architecture and ownership

- The Rust crates express meaningful ownership boundaries: `knx-core`,
  `knx-app`, `knx-store`, `knx-etsproj`, `knx-productdb`, `knx-net`, and the
  server/UI adapters are not collapsed into one application crate
  (`docs/ARCHITECTURE.md:54-123`).
- The layering gate checks forbidden dependency paths, including isolation of
  key material (`docs/ARCHITECTURE.md:147-186`, `xtask/src/layering.rs`).
- The frontend uses Rust-generated projection types for its main read models
  (`crates/knx-projection/src/lib.rs`, `apps/knx-web/src/bindings/`).

### Data integrity and compatibility discipline

- Communication-object values retain their source layer, and explicit empty,
  malformed, absent, and present values are distinguished where overrides need
  that information (ADR-0004, ADR-0010, ADR-0012).
- Unmodelled archive members are retained verbatim and round-trip hash checks
  exist (`docs/COMPATIBILITY.md:41-45`).
- Compatibility claims are carefully scoped to specific schemas and samples.
  Unsupported and unverified behaviour is generally named rather than guessed
  (`docs/COMPATIBILITY.md:29-78`).
- The native project store and product database both have versioned migration
  chains and regression coverage.

### Verification culture

- CI runs formatting, Clippy, workspace tests, frontend tests/build, generated
  binding checks, layering/header gates, and `cargo deny`
  (`.github/workflows/ci.yml:11-79`).
- On 2026-09-15 the local gates completed successfully: Rust formatting,
  Clippy, workspace tests, layering, source headers, `cargo deny`, frontend
  tests, and TypeScript type checking. `cargo test --workspace -- --list`
  discovered 1,594 Rust tests; Vitest reported 468 passing tests across 42
  files.

## Prioritized findings and recommendations

Priority meanings:

- **P0** — address before a public beta, shared deployment, or live write use;
- **P1** — v1 hardening with high value, but not necessarily an immediate
  blocker for controlled development;
- **P2** — valuable follow-up after the integrity and evidence baseline is
  stable.

### P0. Protect every project-replacement path from unsaved-data loss

**Finding.** `new_project_impl` refuses to replace a project when the command
stack indicates edits (`apps/knx-server/src/domain.rs:406-445`). The import and
native-open handlers do not perform an equivalent check
(`apps/knx-server/src/routes.rs:344-450`), and the UI calls them directly
without confirmation (`apps/knx-web/src/App.tsx:412-433`). A user can therefore
edit a project and then open/import another one, replacing the in-memory work.

**Recommendation.** Introduce one backend-enforced project-replacement policy
for new/import/open. Track a genuine saved revision rather than using
`can_undo()` as a dirty flag, return `409 Conflict` by default, and require an
explicit discard token/flag for all three operations. The UI confirmation is a
consumer of that rule, not the rule itself. Add HTTP regression tests for
edited, saved, undone-to-savepoint, and explicit-discard cases.

### P0. Separate desktop and network filesystem trust policies

**Finding.** The standalone server binds to `0.0.0.0`
(`apps/knx-server/src/main.rs:32-36`) and has no authentication
(`README.md:65-67`). Project routes accept absolute paths outside
`KNX_DATA_DIR` because the same handlers also serve Tauri’s native file dialog
(`apps/knx-server/src/paths.rs:9-18`, `apps/knx-server/src/paths.rs:49-63`). The
tests explicitly preserve access to `/etc/hostname`
(`apps/knx-server/src/paths.rs:127-131`). Write targets receive the same bypass.
The documented “trusted LAN” model reduces exposure but does not enforce a
filesystem boundary.

**Recommendation.** Make path capability explicit at construction time:

- desktop mode may accept native absolute paths and must remain loopback-only;
- server mode must reject all paths outside `data_dir`, including absolute
  paths, and should bind to loopback by default unless external binding is
  explicitly requested;
- any future shared/network deployment needs authentication and authorization
  designed together with session isolation.

Add mode-specific route tests for reads, save-as, ETS export, CSV/report export,
and project diff. Do not infer trusted origin merely from absolute-vs-relative
syntax.

### P0. Make project-session replacement and snapshots atomic

**Finding.** One project session is stored in separate mutexes for `project`,
`store_path`, opaque entries, manufacturer references, command history, and
import counts (`apps/knx-server/src/domain.rs:27-65`). Open/import/new replace
those fields one lock at a time (`apps/knx-server/src/domain.rs:237-244`,
`apps/knx-server/src/domain.rs:338-346`). Save/download then acquire several of
them independently (`apps/knx-server/src/domain.rs:559-568`,
`apps/knx-server/src/fs_routes.rs:116-130`). Concurrent requests can therefore
observe or persist a new project with old passthrough/manufacturer state, and
multiple lock orders make later deadlocks easier to introduce.

**Recommendation.** Move all per-project fields into one
`ProjectSessionState` protected by one lock. Build imports/loads outside the
lock, then swap the complete state in one critical section. Take one coherent
snapshot before slow saves/exports. Keep the operational log, product database,
and bus transport separate because they have different lifetimes. Add a
deterministic concurrency test that pauses between load and publish and proves
that save/undo/tree reads see either the old or new session, never a mixture.

### P0. Never turn a broken product database into “not installed” silently

**Finding.** `AppState::new` maps every product-database open or migration error
to `None` (`apps/knx-server/src/domain.rs:108-112`). Missing configuration,
permission failure, corruption, and migration failure therefore become the same
state. Downstream UI can honestly report “no database”, but not that an existing
database failed.

**Recommendation.** Represent product database startup as an explicit state,
for example `Absent | Ready(Connection) | Failed(Diagnostic)`, or fail server
startup when an existing configured database cannot be opened. Surface the
status through health/diagnostics and the Settings panel. Test missing file,
read-only path, corrupt SQLite, and migration failure separately.

### P0. Close known silent or lossy external-data paths

This should be a sequence of small owning-layer fixes, not one broad importer
rewrite.

1. **Resolved 2026-09-17 — range-less group addresses.** Schema-11 and
   schema-21 export now return a typed error naming the installation, internal
   ID, and raw address instead of silently omitting a range-less address.
   Native persistence and optional range membership remain unchanged.
2. **Partially resolved 2026-09-17 — product-data identifier collisions.**
   Same-file `ApplicationProgram/@Id` collisions now use the shared occurrence
   tracking and report the second declaration. Datapoint-type inserts still
   report only a drop count, not winning/losing provenance
   ([KNOWN_LIMITATIONS.md §86](KNOWN_LIMITATIONS.md#86-duplicate-identifiers-inside-one-file--recorded-for-normalized-product-identifiers-dpt-provenance-remains-limited)).
   Add source hash/provenance to DPT definitions and distinguish
   byte/semantic-identical repetition from an actual conflict after the active
   D10 migration work is integrated.
3. **Malformed non-override attributes.** Typed fields outside `Override<T>`
   report malformed source text but cannot preserve it on export
   (`docs/KNOWN_LIMITATIONS.md:1125-1148`). Prioritize fields seen in real
   samples; extend raw-value preservation only where evidence warrants it.
4. **Building-space types.** Five documented `Space/@Type` values are
   coarsened and changed on export (`docs/KNOWN_LIMITATIONS.md:4657-4697`).
   Extend the domain enum, store migration, importer, and both exporters as one
   compatibility change when a fixture can pin the behaviour.

### P0. Keep commissioning writes gated until independent evidence exists

**Finding.** Commissioning procedures are now substantial, but phase 2 is
verified against the repository’s own simulator and no produced frame has
addressed a device (`docs/KNOWN_LIMITATIONS.md:4818-4839`). This is excellent
progress, not sufficient evidence for a user-facing write/download promise.

**Recommendation.** Keep destructive commissioning unavailable by default.
Advance through explicit gates: captured frame conformance, read-only tests on
several devices/gateways, bounded single-operation writes on disposable
hardware, interruption/retry tests, recovery documentation, and only then an
opt-in UI. Preserve a hard distinction between “protocol implemented”,
“simulator verified”, “hardware observed”, and “safe for supported use”.

### P1. Expand the compatibility matrix, not the compatibility claim

**Finding.** The available real-project evidence is strong but narrow: schema
11 and 21 have good samples, schema 23 has limited evidence, schemas
12/13/14/20/22 do not (`docs/COMPATIBILITY.md:66-75`). Live KNXnet/IP evidence
covers only a small hardware set, while the Group Monitor GUI is fake-backed.

**Recommendation.** Maintain a fixture/evidence matrix keyed by schema,
producer version, encryption, modules, vendor, and expected unknowns. Add
sanitized or hash-addressed external fixtures where licensing allows. Create a
manual hardware-in-the-loop suite with recorded gateway/device/firmware and raw
captures; it need not run on every CI job. Claims should continue to follow
evidence, never the other way around.

### P1. Generate or mechanically check the complete HTTP contract

**Finding.** Main projections are generated from Rust, but `api.ts` manually
defines many additional response/request shapes (`apps/knx-web/src/api.ts`,
more than 40 exported types/interfaces). The server defines matching DTOs by
hand in large route modules. Current wire tests cover selected contracts, not
the whole surface.

**Recommendation.** Extend the existing `ts-rs` approach to stable HTTP DTOs,
or generate a compact OpenAPI/schema artifact and diff it in CI. Avoid mirroring
domain objects; generate only boundary DTOs. Until generation is complete, add
a contract inventory test ensuring every registered route has a typed client
function and representative serialization test.

### P1. Restore a quiet, trustworthy frontend test signal and add real-browser smoke tests

**Finding.** All 468 Vitest tests pass, but a measured run emitted 801 React
`act(...)` environment warnings. `vitest.config.ts` has no shared setup file
(`apps/knx-web/vitest.config.ts:1-7`), while individual test files configure the
React act environment inconsistently (for example
`apps/knx-web/src/Search.test.tsx:20-26`). A Playwright proof script exists, but
Playwright is not a project dependency or CI gate.

**Recommendation.** Add one shared test setup, set the React act environment
there, then fix the real updates that remain outside `act`. Fail CI on
unexpected console warnings. Add a very small browser suite for the workflows
whose boundaries matter most: import/open, edit + undo/redo, save/reopen, and
product install + parameter edit. Include keyboard/focus and an automated
accessibility smoke check; keep detailed visual design tests outside the core
gate.

### P1. Test the artifacts users actually run

**Finding.** CI builds Rust and the frontend but does not build the Docker image
or a Tauri bundle. The web package requires Node `>=22.12.0`
(`apps/knx-web/package.json:6-8`), while the Docker frontend stage still uses
Node 20 (`apps/knx-server/Dockerfile:3-8`). The mismatch is already documented
but remains in the release path.

**Recommendation.** Align Docker with the declared Node runtime immediately.
Add a container build plus existing smoke test to CI, and add a Linux Tauri
bundle job at least on release candidates. Verify `--version`, static assets,
health, mounted persistence, and a save/reopen cycle against the built
artifacts—not only workspace binaries.

**Resolved 2026-09-17 (Docker portion).** The frontend build stage now uses
Node 22, matching the package engine. CI builds the actual Docker image and
runs a fixture-free health plus native save/reopen smoke test; local runs may
add a real ETS import with `KNXBENCH_REFERENCE_PROJECT`. The separate AppImage
workflow covers the Linux Tauri bundle and its startup contract.

### P1. Re-establish one current, compact source of truth

**Finding.** Documentation quality is high, but accumulated chronology now
obscures current truth:

- `ARCHITECTURE.md` first includes parameter editing in v1 and then lists it as
  excluded (`docs/ARCHITECTURE.md:12-28`);
- `README.md` still says parameter editing is out of scope
  (`README.md:31-37`), although parameter GET/POST routes and UI exist;
- `COMPATIBILITY.md` says parameter values are never editable
  (`docs/COMPATIBILITY.md:80-90`);
- **Resolved 2026-09-16:** KNXBench is licensed under
  `AGPL-3.0-or-later`; `LICENSE`, `README.md`, `Cargo.toml`, and
  `KNOWN_LIMITATIONS.md` now agree;
- `IMPLEMENTATION_STATUS.md` and `KNOWN_LIMITATIONS.md` have grown to roughly
  5,963 and 4,883 lines respectively, with resolved history mixed into current
  status; `.ai/CURRENT_STATE.md` has the same append-only tendency.

**Recommendation.** Keep one short current-status table keyed by capability and
evidence. Move completed chronological entries to a changelog/archive, move
resolved limitations to an archive while preserving anchors or redirects where
needed, and add a documentation consistency checklist to each slice.

**Licence update, 2026-09-16.** The licence work is complete:
`AGPL-3.0-or-later` was selected, the canonical text was added, and the current
licence documentation was reconciled.

### P1. Add crash recovery and a real save-point model

**Finding.** SQLite transactions protect the consistency of a save, but the
in-memory undo history and unsaved edits do not survive a process crash. The
server currently approximates “dirty” with `can_undo()`
(`docs/KNOWN_LIMITATIONS.md:4029-4050`), so a saved project can still look
dirty and an open/import operation can lose edits.

**Recommendation.** First implement the saved-revision marker required by the
P0 project-replacement guard. Then add a bounded recovery journal or periodic
autosave for desktop use, with explicit recovery on next startup and no silent
overwrite of the canonical `.knxdb`. Test crash points and recovery with a
temporary store. Do not mix this with cloud sync or multi-user work.

### P2. Split only the proven maintenance hotspots

**Finding.** Several files now concentrate broad responsibility:
`knx-core/src/command.rs` (~3,738 lines), `knx-server/src/domain.rs` (~3,710),
`knx-productdb/src/query.rs` (~3,167), `knx-cli/src/main.rs` (~2,630),
`knx-server/src/routes.rs` (~2,007), and `knx-web/src/api.ts` (~1,052).
Large size alone is not a defect—`dpt/codec.rs`, for example, is large because
the domain is large—but the session-state and HTTP-contract findings show real
ownership pressure in `domain.rs`, `routes.rs`, and `api.ts`.

**Recommendation.** Refactor only alongside the owning fixes: session lifecycle
for the atomic state change, route DTO modules for generated contracts, and
feature-grouped command modules while retaining one public `Command` enum.
Bracket each extraction with existing tests and layering checks. Avoid a
repository-wide “clean architecture” rewrite.

### P2. Track desktop dependency risk without destabilizing the alpha

**Finding.** Tauri 2 currently brings archived GTK3 Rust bindings and requires
documented advisory exceptions (`docs/KNOWN_LIMITATIONS.md:1150-1168`). There
is no drop-in GTK4 migration in the current stack.

**Recommendation.** Keep the exceptions explicit and reviewed on dependency
updates, generate a software-bill-of-materials for release artifacts, and set a
decision checkpoint before beta. Do not replace Tauri merely to make the
advisory list shorter; migrate when upstream support or a measured operational
problem justifies the cost.

## Suggested execution order

1. **Integrity baseline:** unified dirty/save-point policy; protect open/import;
   atomic `ProjectSessionState`; explicit product-database failure state.
2. **Trust boundary:** split desktop/server path policy, safe bind defaults,
   mode-specific tests, and an explicit decision about network authentication.
3. **Known data-loss closures:** range-less address export, product-data
   collision provenance, then evidence-backed malformed/type preservation.
4. **Build and test signal:** remove React warning noise, add minimal browser
   workflows, align/build Docker, verify a Tauri release bundle.
5. **Compatibility evidence:** broaden schema fixtures and physical gateway
   coverage; keep commissioning write paths gated.
6. **Release hygiene:** compact current-status docs, recovery path, and
   packaging/SBOM. The licence decision and canonical file were completed on
   2026-09-16.
7. **Only then:** multi-user editing, KNX Secure, wider commissioning,
   spatial/floor-plan work, or LLM integrations—each after its missing evidence
   and product decision exists.

## Explicit non-recommendations for the current stage

- Do not chase “full ETS compatibility”; the repository correctly rejects that
  claim and cannot reproduce proprietary vendor plug-in behaviour.
- Do not prioritize spatial coordinates or a floor-plan editor before v1; ADR
  0019 already gives this a coherent post-v1 boundary.
- Do not add KNX Secure without sample key material and interoperability
  evidence.
- Do not optimize large-project performance speculatively; preserve and extend
  the existing performance baselines, then optimize measured regressions.
- Do not build multi-user editing as a small add-on to the current shared
  project/undo stack. It needs identity, authorization, isolation, and conflict
  semantics as one design.

## Verification performed for this analysis

The following commands completed successfully on `main` at commit `06db5ec`:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xtask -- check-layering
cargo run -p xtask -- check-headers
cargo deny check
npm test --prefix apps/knx-web
npx --prefix apps/knx-web tsc -p apps/knx-web/tsconfig.json --noEmit
```

Not performed: physical KNX operations, destructive commissioning, a Docker
image build/smoke run, a Tauri bundle build, or a security penetration test.
Those omissions are material to release confidence and are intentionally not
presented as covered.
