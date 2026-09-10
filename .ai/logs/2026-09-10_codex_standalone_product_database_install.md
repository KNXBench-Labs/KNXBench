# 2026-09-10 — Task 5: reconcile evidence and compatibility documentation

Plan: `docs/superpowers/plans/2026-09-09-standalone-product-database-install.md`
(Task 5 of 5). Branch: `codex/catalog-creation-diagnostics`. Base for this
task's work: `91193da` (Task 4 handover state), through the fix-loop commits
`e8011fb`/`c957197`/`2f7bbcc`. Executing agent: Claude (filename kept as the
plan's own literal `..._codex_...` name per explicit dispatch instruction,
despite the agent identity).

## What this task changed

Documentation only, per the plan's own file list — no product code behavior
changed except two trivial, zero-risk lint/format fixes surfaced while
running the final gate (see "Gate findings" below), which are style-only and
committed separately.

### Ground truth established before writing anything

Before editing any doc, re-verified every claim against the actual repo
state rather than trusting the design spec or prior session memory:

- **Corpus split.** Read `knx_master.xml`'s `xmlns` directly out of each of
  the 6 files in `OriginalData/ProductDatabases/` with `unzip -p`:
  - Scheme 11 (`http://knx.org/xml/project/11`):
    `646704-04_ETS4_2012_47_DE_EN.knxprod`,
    `Weinzierl_730_KNX_IP_Interface_ETS4.knxprod`,
    `Weinzierl_730_KNX_IP_Interface_ETS4_v1.knxprod`
  - Scheme 20 (`http://knx.org/xml/project/20`):
    `MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`,
    `Dummy_Applikation_Secure.knxprod`
  - `Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2` — confirmed via `unzip -l`/
    `unzip -p` to be a pre-2013 ETS2-era SFX/`.vd_`-style archive with no
    `knx_master.xml` at all; not the same ZIP/XML container family as
    `.knxprod`/`.knxproj`, not merely "the same format but encrypted."
  - Matches the design spec's claim ("three archives at namespace .../11,
    two at .../20") exactly, and matches
    `installs_the_readable_corpus`'s own file list
    (`crates/knx-productdb/tests/standalone_packages.rs`).
- **`.vd2` rejection mechanism.** Read `crates/knx-productdb/src/package.rs`
  directly: `install_package` checks
  `source_name.to_ascii_lowercase().ends_with(".vd2")` and returns
  `PackageError::LegacyVd2` *before* any ZIP parsing, hashing, or size
  computation happens (line ~345, ahead of the ZIP-open call). The
  `Display` impl renders this exact string:
  `"legacy .vd2 product data is unsupported"`. This means the design
  spec's acceptance criterion "the caller receives the archive hash/size
  in the error report where available" is **not implemented** for this
  case — recorded honestly as a gap in IMPLEMENTATION_STATUS.md, not
  fixed (out of scope for a docs-only task).
- **Spec citations verified against primary source, not cited on faith.**
  - `The KNX Standard v3.0.0/Project Schema23 v01.00.00.md` §4/4.1/4.2 (read
    lines ~1600-1703): confirms the `.knxprod`/`.knxproj` extension
    distinction and the "one MasterData in one XML file" +
    `M-iiii/{Catalog,Hardware,...}.xml` per-manufacturer layout — supports
    the design spec's §4.2.2-§4.2.3 citation for the shared container
    family claim.
  - `03 Volume 3 System Specifications/03_01_01 Architecture v03.00.02
    AS.md` §6.2 "System Mode" (read lines ~408-425): confirms the
    "manufacturer-created/maintained product template, imported by
    ETS/an installer, drives configuration" concept (text has PDF-
    extraction garbling but the substance holds) — supports the design
    spec's §6.2 citation for treating a `.knxprod` as a *product
    template* input, not a configured device.
- **`PackageError` variant/message inventory** (for the COMPATIBILITY.md
  rejection row and the CLI/HTTP citation): `LegacyVd2`, `InvalidZip`,
  `Encrypted`, `UnsafeMember`, `DuplicateMember`, `SizeLimit`,
  `MissingMaster`, `UnsupportedNamespace`, `ProjectArchive`,
  `MissingManufacturerData` — all in `crates/knx-productdb/src/package.rs`.
  `malformed_and_legacy_product_uploads_are_typed_bad_requests`
  (`apps/knx-server/tests/http_product_install.rs`) pins the exact HTTP
  strings `"invalid product ZIP"`, `"legacy .vd2 product data is
  unsupported"`, `"encrypted product ZIP member"`.

### Docs edited

- **`docs/GAP_ANALYSIS_ETS.md`.** Reconciled A5 (partially closed: scheme
  11/20 standalone package install verified, distinct from full `.knxproj`
  project import); closed B1/B2/B3/B5/B6/B7 (they already read "Done" in
  the Tier-1/2 task backlog with 2026-09-06/07/08 dates, but the Section B
  table rows still described them as open gaps — table rows now match the
  backlog, each with a one-line "closed, why" plus the original impact
  text kept for historical context); closed/extended D3 (catalog browser
  existed since T2, now also has package install); fixed the stale "T2's
  future browser" phrase in T1's own entry; added a new **T24** task-
  backlog entry for this plan's Tasks 1-4 (standalone package installer +
  honest creation diagnostics), the first new backlog number since T23;
  updated the document's "as of" header to 2026-09-10.
- **`docs/KNOWN_LIMITATIONS.md`.** §11 rewritten: scheme 11/20 standalone
  `.knxprod` package install now works (verified, cited), schemes
  12-19/21/22 remain unread (no standalone sample tested, not "encryption
  unresolved" — that premise was wrong for 11/20 and unproven either way
  for the rest), `.vd2` named as a permanent, structurally-different-
  format blocker rather than an untested general failure, with an
  explicit note distinguishing this row's `.knxprod`-package scope from
  `.knxproj`-project schema coverage (still 11/21/23 only) to avoid
  conflating the two. §35 marked **RESOLVED (2026-09-10)**: original text
  kept below a new "Resolved" summary for context; the stale "T2's future
  catalog-browser UI" lift condition corrected to "Done, 2026-09-10."
- **`docs/ROADMAP.md`.** The "open questions" table's
  `.knxprod encryption for master data scheme ≥ 12` row corrected: the
  encryption premise was wrong for schemes 11/20 specifically, both now
  ingest directly; schemes 12-19/21/22 still have no fixed session,
  `.vd2` is unrelated to encryption at all. Left the Session-4-scoped
  "Deferred" bullet list (a different, clearly historical section)
  untouched — it is dated to that past session, not a live claim, and the
  plan's own consistency-grep expects historical mentions to remain.
- **`docs/COMPATIBILITY.md`.** §2 "Verified today" gained two new rows: the
  standalone package install (citing `installs_the_readable_corpus` and
  the exact 5-file list, plus both verified spec citations) and the
  rejection-path tests (`malformed_and_unsupported_packages_leave_no_rows`,
  `malformed_and_legacy_product_uploads_are_typed_bad_requests`). §3's
  "Schema 20 (ETS 5.7)" row relabelled `.knxproj` **project** import
  specifically, with a parenthetical distinguishing it from the now-
  verified `.knxprod` *package* claim, so a reader doesn't conflate the
  two different scheme-20 claims. §4's blanket "Direct `.knxprod` import
  for master data scheme ≥ 12" row narrowed to carve out the now-tested
  11/20 exception, keeping schemes 12-19/21/22 and `.vd2` honestly listed
  as still unsupported.
- **`docs/IMPLEMENTATION_STATUS.md`.** Appended a full **T24** entry
  (Tasks 1-5 of this plan) at the file's tail, in the same per-task prose
  style as the existing T1-T23 entries: first-winner provenance fix,
  the atomic package installer with its full rejection matrix and the
  6-file corpus result, truthful CLI/HTTP error surfacing, the
  `CreateDeviceResponse`/`CreationDiagnostic` wiring plus
  `CatalogBrowser.tsx`'s install picker, and this reconciliation step
  itself (including the explicitly-recorded, not-fixed hash/size gap for
  `.vd2` error reports, and the exact gate results — see below). Also
  corrected one earlier "remains unsupported" bullet (in an older, not-
  clearly-historical-looking "known gaps" list from the 2026-09-06/07
  cycle) that would otherwise contradict the new KNOWN_LIMITATIONS §11.

### Documentation consistency search (plan's Step 2)

```
rg -n "future catalog-browser|no device catalog browser|scheme 11 is readable|direct.*knxprod.*out of v1" docs
```

Before this task's edits: 2 matches (`docs/ROADMAP.md:472`,
`docs/GAP_ANALYSIS_ETS.md:34`). After all edits above: **0 matches** — both
had their exact matched phrasing rewritten, not just papered over
elsewhere.

A broader manual sweep for `scheme ≥ 12`/`scheme >= 12` afterward found
further mentions in `docs/ARCHITECTURE.md`, `docs/IMPORT_EXPORT.md`, and
several `docs/superpowers/{plans,specs}/*` files (all outside Task 5's
explicit file list) plus one genuinely historical, session-dated
`docs/ROADMAP.md` "Deferred" bullet — left untouched deliberately: the
`superpowers/` files are frozen session artifacts by convention (not
"living" docs CLAUDE.md asks to keep current), and touching
ARCHITECTURE.md/IMPORT_EXPORT.md would be scope creep beyond the plan's
own file list. Flagged here rather than silently ignored.

## Gate findings (Step 4)

Ran, in order, against `HEAD` after the doc edits:

1. `cargo fmt --all --check` — **failed initially**, not one of the two
   pre-flagged known issues. Traced to two leftover-unformatted spots from
   the Task 3/4 fix-loop commits (`e8011fb`, `c957197`):
   `apps/knx-server/src/domain.rs:1123` (a test's multi-line
   `CreationDiagnostic::from_enrichment(...)` call) and
   `apps/knx-server/src/routes.rs:564` (the `DynamicOrModuleNotEvaluated`
   match arm's struct literal). Ran `cargo fmt --all`, diffed the result —
   whitespace/line-wrap only, no logic change — and committed separately
   as `style: cargo fmt leftover reflow in domain.rs/routes.rs`
   (`e56c83a`).
2. `cargo clippy --workspace --all-targets -- -D warnings` — failed on
   exactly the two pre-flagged `clippy::large_enum_variant` errors on
   `knx-etsproj`'s `Frame` enum (`crates/knx-etsproj/src/parse/
   installation.rs:36`, `installation_v21.rs:48`, 744 vs 360 bytes) — left
   untouched as instructed (do not touch that enum; CLAUDE.md requires
   size optimizations to be measurement-driven, not lint-driven).
   Additionally surfaced a **second**, previously-unflagged pre-existing
   issue: `crates/knx-core/src/command.rs` lines 1394/1398/1432,
   `clippy::bool_assert_comparison` (`assert_eq!(x, false/true)` in three
   test assertions inside `SetComObjectFlag` tests, unrelated to the
   `Frame` enum). Judged this trivial and zero-risk (test-only,
   mechanical `assert_eq!(x, false)` → `assert!(!x)` rewrite, same
   semantics) and fixed it, committed separately as
   `style(knx-core): fix bool_assert_comparison clippy lint in flag
   tests` (`1f80064`). Re-ran `cargo clippy -p knx-core --all-targets --
   -D warnings` clean, then re-ran the full workspace clippy and confirmed
   the *only* remaining failure is the two blessed `large_enum_variant`
   errors.
3. `cargo test --workspace` with
   `KNXBENCH_PRODUCT_CORPUS=/mnt/daten-i/Sourcecode/KNXBench/OriginalData/ProductDatabases`
   set — **all green**, exit 0, including
   `installs_the_readable_corpus ... ok`. Every `test result:` line in the
   run shows `0 failed`.
4. `cargo run -p xtask -- check-layering` — clean:
   `layering ok: knx-core reaches none of [...]; knx-etsproj does not
   reach knx-store; knx-productdb reaches neither knx-etsproj nor
   knx-store; knx-projection reaches none of [...]`.
5. `cd apps/knx-web && npm test` — 9 test files, **96/96 tests passed**.
6. `npm run build` — succeeded (`✓ built in 172ms`); restored
   `apps/knx-web/dist/.gitkeep` with `git checkout -- apps/knx-web/dist/.gitkeep`
   afterward, per the documented gotcha.

Net: the full gate chain from the plan's Step 4 is green end to end, with
the only remaining failure being the two explicitly out-of-scope
`large_enum_variant` errors, exactly as expected going in.

## Judgment calls worth flagging explicitly

- Fixed two pre-existing, non-product-code issues (`cargo fmt` drift, one
  `bool_assert_comparison` clippy lint) that surfaced mid-gate rather than
  leaving the gate red. Both are mechanical, test-only or whitespace-only,
  reviewed via `git diff` before committing, and committed as separate
  `style:` commits from the docs commit — not mixed into the docs
  changeset, and not touching the one enum explicitly placed off-limits.
  If this was the wrong call for a documentation-only task, both commits
  are trivially revertible in isolation (`e56c83a`, `1f80064`).
- `docs/GAP_ANALYSIS_ETS.md`'s new **T24** backlog entry is new
  documentation, not called for by name in the plan's file/edit list, but
  is the natural place this project's own convention (every closed gap
  gets a task-backlog entry, per T1-T23) already puts this kind of
  change — added for consistency with the existing document shape, not
  scope creep on the underlying work.
