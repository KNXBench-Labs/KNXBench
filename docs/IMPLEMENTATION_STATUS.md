# Implementation delivery history

This page is a short delivery index, not the current feature inventory.
Use [current capabilities](manual/implementation-status.md),
[practical boundaries](manual/known-issues.md), and the [ledger](status/LEDGER.md).

Full original entries live in ISO-week files under [history/implementation](history/implementation/README.md).
Dates are parsed per entry, not inferred from their position. Late entries
for an older date keep that date and their original ordering in the archive.

**Maintenance:** prepend new delivery entries here as before. At a later
archival checkpoint move complete entries into their dated period, preserve
fragment aliases and update this index. Do not rewrite previous results.

## Limitation-summary reconciliation (20 October 2026)

Eight current catalogue summaries now match their delivered consumers: supported
parameter captions, guarded release stamping, actual archive ceilings, scoped
shutdown admission, Alpha 12 download availability, manifest-scoped native
portability, CLI versus generic diagnostic profiles and bound web installation
renames. Nine dossiers gain dated current-reading notes, including the fourth
motion category. Manufacturer-file portability is narrowed in the operating
manual too: the additional catalogue-created-device case is source-traced,
not runtime-reproduced or fixed. The generated overview follows the catalogue;
preserved history and every formal ledger disposition remain unchanged.
[Scope, evidence and validation boundary](status/2026-10-20-limitations-reassessment.md).
Documentation only; no new compatibility, hardware or recovery admission.

## WP-17 master provenance — delivered on main (19 October 2026)

Product database **v27** adds separate source/declaration/winner evidence for
DPT mains/subtypes, function types/points and space usage. Normalized winner
rules stay unchanged; losing and malformed declarations have source positions
and distinct diagnostics through direct import, CLI, HTTP and the catalogue
report. Historical winners and encounters are explicitly unavailable, not guessed.
[ADR-0147](adr/0147-master-identity-provenance.md) defines bounded, atomic replay
and the fixed 16 MiB logical evidence allowance; it is not a process-memory limit.

Independent review and correction review accept core `887f024a`. ProductDB:
458 library, 598 integration and 2 doc tests passed; Clippy passed. The final
115-instance matrix passed **11/11** with frozen inputs. Controlled v26 → v27
migration preserves every value in all **42 unaffected tables** and all **859
historical diagnostic rows**; new migrated evidence remains historical rather
than pretending to be fresh observations. Earlier failed attempts stay retained.
The maintained branch gate on `d2807f70` passed all 15 stages, including 4,136
component tests, 863 default browser cases, 18 catalogue production cases,
2,985 Rust tests and 97 selected corpus tests, with frozen inputs and no changes
to real user directories. Main-first merge `7aed9ddd` and its result-only
follow-up `d18ce65e` are published on `main`; the final merged 15-stage gate
passes the same complete scope, including all 863 default browser cases and
18 production cases. Local/tracking/live refs were verified equal after push.
The earlier merged browser-page acquisition failure remains retained (root cause
unestablished); a fresh complete merged run passed without changing product or
test code. Alpha 15 binaries do not contain this source change. Older builds
refuse a v27 product database; keep a backup before upgrading.

## Alpha 15 publication verified — 19 October 2026

`v0.1.0-alpha.15` points to immutable `c3d5b35f`: one complete release gate
**31/31**, frozen inputs, then successful AppImage and dual-architecture Docker
publication. Public checksums/provider digests, binary/OCI revision binding,
isolated authenticated save/reopen, native-10 → 16 migration, headless extracted
AppImage startup and all 37 served website files passed. The owner's running
container was not upgraded. Failed packaging/verifier attempts remain explicit;
see the [acceptance record](releases/alpha15-acceptance.md) and
[publication receipt](evidence/release-alpha15-2026-10-19.json). Native schema
16 and product-database schema 26 retain the documented compatibility boundaries.
No new bus/device, FUSE/Mesa, native visual or screen-reader evidence is claimed.

## Alpha 15 set closed; documentation drift, first slice (19 October 2026)

The Alpha 15 package set is delivered: WP-16, WP-18, WP-15 with the two zoom
follow-ups, WP-20 and WP-01 + WP-02 (product DB v26), plus two packages the
work itself required (gate isolation of the developer's knx directories and
the fit-assertion catalogue race). Not delivered from the claim: the G16 half
of WP-03 (CLI/refusal messages for legacy binary enumerations), which stays
open under WP-03. WP-13's first slice corrects release-relevant drift: KL-181
now names the alpha.12 AppImage as the first fixed one (tag ancestry of
`72d407d1`), generated from the catalogue; FEATURES lists issues #25/#26 as
`Main-only`; the OPEN_WORK queue and claim list match the deliveries; the
intermittent SIGTERM tool test is recorded as open. Docs only.
[OPEN_WORK WP-13](OPEN_WORK.md#wp-13--documentation-drift).

## Parameter units after the value: `SuffixText` in product DB v26 (19 October 2026)

WP-02 ([ADR-0146, part 2](adr/0146-unknown-evidence-rows-have-an-identity.md#part-2-suffixtext-reaches-the-parameter-row-wp-02)).
The product collections declare a parameter's unit text as `SuffixText`; the
database stored only a `Suffix` column the collections never use. The same
schema v26 now stores `SuffixText` on `parameter` and `parameter_ref`
(separately from `Suffix`), resolves it like `Text` (the reference's own
declaration first, each layer translated), and the parameter row shows it right
after the value. It is never part of the value, a draft or a write. The
`SuffixText` report rows retire only on those two hosts.

On the 115-instance matrix corpus an XML census predicted all five moved counts
before the run and the run matched them (unknown report totals −635 / −633,
`ingest_unknown` −627, `package_install_unknown` −217, and both
parameter-attribute pins); the migrated v25 shared database equals a fresh v26
install in all 43 tables.

A cross-vendor review then found three gaps, closed in the same package: a
migration filled the second declaration of a program id repeated in one file
(ingest keeps the first), it retired the `SuffixText` evidence inside `Dynamic`
that ingest keeps reporting, and the checked editor did not show the suffix.
The matrix does not move (only the migration changed) and the migrated v25
database again equals a fresh install.

## Unknown evidence is recorded once per blob: product DB v26 (19 October 2026)

WP-01 ([ADR-0146](adr/0146-unknown-evidence-rows-have-an-identity.md)).
Census 3 had found `ingest_unknown` rows repeating each other. Synthetic
packages reproduced the cause: a package install re-parses the members it
shares with earlier packages, and the master, and wrote their unknown rows
again; a re-parsed losing blob repeated its `IdConflict` row. Product DB v26
gives the row an identity (two unique indexes), writes idempotently and, on
upgrade, merges existing copies into the oldest row with the largest per-parse
count and the oldest sample. Package reports, `package.unknown_count` and
everything a user sees in an install report are unchanged.

On the 115-instance product matrix corpus a census of the v25 shared database
predicted every moved number, and the v26 run matched it exactly:
`ingest_unknown` 23,176 → 22,327, the parameter-attribute pin (1,146, 288,
73,548) → (1,128, 288, 71,634), the master-language pins (113, 67, 3,125) →
(67, 67, 1,870); outcomes and all other counts equal. The v25 shared database
migrated to v26 equals a fresh v26 install in all 43 tables.

## The fit assertion measures the resolved catalogue, not the loading one (19 October 2026)

`table-column-widths.e2e.ts`' independent fit assertion failed twice in package
gates (gate isolation full gate 4, product-DB gate 2) with the same numbers:
Manufacturer and Order number floors 77 at the first measurement, 76 and 71
at the second. Cause: `openList()` returns once the Devices list is visible,
while the product catalogue (`/api/devices`) still loads; until it answers those
cells read *Not resolved*, so under load the first measurement saw another
table than the second. Proof: holding the catalogue response until after the
first measurement fails the old test with exactly those numbers. The spec now
opens the list through `openResolvedList()`, which waits for a resolved product
cell and for the loading status to disappear, in every case that measures the
Devices table (default fit, overflow and drag, overlay, inflated-floor, the zoom
cases, stored scale), and again after both reload paths. Regression: with the catalogue held, the wait must refuse
(`rejects`) instead of letting a measurement start; an empty wait turns it red.
Test-only; no application code, width budget, tolerance or retry changed. This
corrects the earlier reading of gate 4 as a late-font race: the font barrier
added then stays as hardening with its own regression, but it did not explain
that failure.

## System column in the Devices list, from one cited mask table (19 October 2026)

Owner request (WP-20): the Devices list names each device's KNX system family.
One table in `knx-core` (`system_family`, 37 rows) maps the mask version that
the application program **declares** in the product database to System 1,
System 2, System 7, System B or another profile under its own name (System 300,
Coupler, KNXnet/IP router, USB interface, IR decoder, RF uni-/bidirectional).
Every row was checked against the original KNX pages (Resources Table 7,
Profiles Table 1, the mask 2705h/27B0h/2920h profiles); rows with a single
source say so in the code and the
[research note](research/2026-10-19-mask-version-system-family.md).
`0AFDh`/`0AFEh` and unlisted neighbours stay *Unknown (mask XXXXh)*.

`GET /api/devices` adds a closed `system` state per row: no product, no
application program, program not installed, program of other hardware, no mask,
unreadable mask, unknown mask or family. A program link to other hardware is
not called "no application program". The client requires the field on every
row and admits it strictly (a missing or `null` state refuses the batch), and
the new *System* column sorts and filters by the shown label or the mask
(`0701h`), in English and German. A mask read from a device is not used.
[Guide](manual/user-guide/05-devices-and-products.md#system-column-source-builds-after-alpha-14).

## Package gates keep their hands off the developer's knx directories (19 October 2026)

Acceptance follow-up: full gate 4 on `a86cb1f9` stopped at one browser test
(788 functional cases passed). The independent intrinsic-width oracle now
flushes the source table's layout and awaits current font readiness (*later
correction:* the gate-4 failure itself was a catalogue race, see the entry
above). A held-font-response regression fails
without the barrier; the complete table-column spec passes 50/50 with it. No
application code, width tolerance or retry policy changed. Both reviews
accepted; complete package gate green on `72e92e86` (68 web stages, 855
browser cases, Rust 5,023/0, corpus 155/155; no user data touched or
unchecked); delivered in merge `9535e132`.

The same day as the `doc-export` fix, a second gate migrated the owner's fresh
product database (an ETS import made at 14:37 with the Alpha 14 AppImage) to
the unmerged v26 schema: the gate's `rust-tests` stage on the product-DB branch
ran 15:16:06–15:17:57, the file was written at 15:16:33, and that branch still
predated the fix, so its `cli_documentation_export` tests opened the existing
file with the migrating opener. A fix on `main` cannot protect a gate that runs
an older branch's tests, so the runner now isolates them: the package gate
(`tools/run_package_gate.py`) gives its test stages (`tool-tests`, `rust-tests`,
`bindings`, `corpus`, website and story tests; in a complete run also the web
gate's own `tool-tests` step) empty scratch `XDG_DATA_HOME` and
`XDG_CONFIG_HOME` directories, and fails, naming each path, when a file in the
real `knx` data or config directory, or behind a symlink there, is created,
changed or removed during the run (`userDataTouched` in the receipt; size,
mtime, ctime and inode, symlinked directories followed once), or when it cannot
check them (`userDataUnchecked`: an unlistable directory or unsearchable parent,
a link to an ancestor
or to `$HOME`, more than 5,000 entries; an entry vanishing mid-scan is a
change, not a crash). Browser and build
steps keep the real values (fontconfig, pinned pixel widths). Tests:
`tools/tests/test_run_package_gate.py` (directory resolution like the apps,
routing through both runners' real step functions, symlinked database and
directory, same-size rewrite with the old mtime, unlistable directory, entry
removed mid-scan, link to `$HOME`, entry bound, retargeted link, receipt
failure keeping an earlier error); 15 mutants turn them red. Three cross-vendor
review rounds (Codex) found seven gaps (web `tool-tests` step, symlinks,
unlistable directories, an unsearchable parent read as absent, mid-scan
removal, links to `$HOME`, a test tied to the production stage set); all fixed.
The owner parked the migrated file unchanged. Still open: direct `cargo test`
runs ([OPEN_WORK](OPEN_WORK.md#test-gates-change-scoped-by-default-19-october-2026)).
[Pitfall](DEVELOPER_PITFALLS.md#gates).

## A product database the build cannot read is named, not hidden (19 October 2026)

Owner report on `0.1.0-alpha.14+g6d92fda` (AppImage): after an ETS import, product
databases did not install, no error was visible and the catalogue stayed empty.
Cause, reproduced on a copy of the owner's file: the product database was at v26,
written by a gate on the unmerged WP-01/02 branch. Five `knx-cli` tests spawned
`knx doc-export` with the developer's `HOME`, and `doc-export` opened the default
product database with `open_and_migrate`. `AppState::with_user_product_db` then
turned v25's `FutureVersion` refusal into `.ok()`, so server and desktop ran
without products and every route said only "no product database configured".
Fixes: `doc-export` and the reading commands `products list/coverage/verify/
show/identity/family/order-number` and `device readiness` open the database
read-only (`open_read_only`, as the MCP adapter does, ADR-0090); a missing file
reads as an empty in-memory catalogue and is not created (RED: all eight
commands grew an empty, version-0 file to a full schema or created one). The CLI
tests' helpers give `knx` a never-existing `XDG_DATA_HOME` (inotify sweep over
the 185 tests: 5 → 0 opens). `AppState::with_product_db_at` keeps the open error
in `product_db_unavailable`, prints it, logs it as a `product database` error
that survives the log reset on new/open/import, and every refusal (catalogue,
package and legacy install, parameter writes, download preparation, device
readiness, manufacturer-file restore) names it while keeping its status and kind;
without a reason the wording is unchanged (pinned by a test). A location that
cannot be checked is refused instead of read as an empty catalogue. Nine mutants
killed. Cross-vendor review (Codex `gpt-6.1-sol`, high) and its re-review of
the fix round: accept with fixes, every finding fixed test-first. Troubleshooting page, developer pitfall and an OPEN_WORK item
(gate-wide `XDG_DATA_HOME`) record the rest.

## Empty ZIP directories must really be empty, WP-16 (19 October 2026)

Closes the KL-159 round-4 corner. A directory record in a `.knxproj`, outer
archive or protected nested payload, whose CRC-32 is not 0 or which is
compressed without a complete stream (none, truncated or malformed) is now
refused as an inconsistent record that names the directory and the reason.
Both headers agreeing had been enough before. The check runs while the
container opens, so an existing project and product database stay
byte-for-byte unchanged. Stored (ETS6, `jar`) and Java `ZipOutputStream`
directories still import; name, budget, prefix and layout rules are
unchanged. Evidence: 9 new container tests (5 RED first) and one application
test comparing seeded database files and WALs; a private-corpus census found
222 directory records, all stored and empty with CRC 0
([KL-159](limitations/KL-159-project-archive-budgets-and-duplicate-names.md)).
Open: bytes behind the end of a deflated *file's* stream, and the product
reader's equivalent for directories
([WP-16](OPEN_WORK.md#wp-16--empty-directory-zip-integrity)).

## Pane edges follow the pointer at every zoom level (19 October 2026)

The two UISET zoom follow-ups (WP-15 part B). Navigation and inspector pane
drags and the left column's block splitters divide the pointer delta, and the
splitter its measured height, by the application scale, so at 150% a pane edge
no longer runs 1.5 times ahead of the pointer. One table case now drives the
scale only through the stored `uiScale` setting, the zoom shortcuts and a
reload, so App's own scale effect, the narrow-window inspector stacking and a
startup with a stored scale run together with the table measurement
([KL-130](limitations/KL-130-ZOOM-application-zoom.md#update-19-october-2026-pane-drags-and-a-setting-driven-table-case)).
Six Vitest cases and the 80%/150% real-drag browser cases were RED before the
fix; four source mutants each fail a named case. Chromium-only evidence.

## Nested module scopes keep their own identity in the web client (19 October 2026)

WP-15: the web client adopts ADR-0063's `nodeChain`. Two nameless nested
instances of one module definition share `moduleNode`, `moduleId` and
`moduleDefId`. Their diagnostics no longer merge into one group. When one of
them disappears after a parameter change, the chosen area, a typed draft and
an opened Restricted section stay with their own scope. An older server
without the field gets a tagged legacy fallback that can never equal a chained
identity. A malformed chain is treated as absent instead of refusing the panel
([ADR-0063](adr/0063-parameter-scopes-preserve-evaluation-identity.md#web-adoption-wp-15-19-october-2026)).
Seven new Vitest cases were RED before the fix; six source mutants (chain
ignored, clipped, reordered, untagged fallback, shape check removed, position
keys) each fail a named case.

## Device comparison shows what differs — WP-18 (19 October 2026)

Owner requests 1 and 2 for the *Project file versus device* result. Raw hex
rows mark every differing byte on both sides (bold, solid underline) and every
unread or uncomparable byte differently (italic, dotted underline); tooltips and
a per-row byte count make the marks independent of colour. Marks come from an
equal-length comparison of the server's hex bytes plus its byte-level
`changes`, never from a UI codec. The result opens with only rows not proven
equal; *Show equal rows (N)* reveals every row and every original read range.
The Devices dialog and the Device checks panel share the table and both pass
the server's `changes`. Display only: response, session tags and Synced are unchanged. The comparison
contract's "no differences-only filter" sentence is amended (no filter may hide
a differing or unread byte), with a matching ADR-0130 note
([contract](contracts/device-comparison.md#result-display-wp-18-owner-amendment-19-october-2026)).
Offline verification only: Vitest and the intercepted production-build browser
spec in dark/light themes at 1440/400 px; no real device was read.

## Device catalog: identity lines and placement cleanup (19 October 2026)

Owner report: devices appear twice in the Device catalog, with no visible reason;
asked for a cleanup of old entries. Cause: the catalog lists placements
(`catalog_item` rows); several can share a product and application link, and a
newer application version is installed beside the old one by design. Rows now show
application ID, name and version, catalogue section and source file. **Clean up
catalogue** removes only selected placements after a token-bound preview (with a
"last entry for this product and application" warning and the open project's
device count) and a full backup, in one writer reservation; products, programs,
package bytes and all other tables stay byte-identical
([ADR-0145](adr/0145-catalogue-cleanup-removes-placements-only.md),
[contract](contracts/catalogue-update.md#placement-cleanup)). Evidence before the
package gate: product-DB `catalog_update` 9/9 (RED first for every new behaviour),
server `http_catalogue_update` 20/20 (4 new, RED first), Vitest 4,028/4,028 in 282
files, 22 web typechecks, production catalogue browser config 14/14 (4 new cleanup
cases, EN/DE × 1440/400, screenshots inspected), Clippy `-D warnings` for both
crates, headers/layering/anchors. No schema change.

## Faster test gates: change-scoped by default, tagged browser lanes, bundled Rust tests (19 October 2026)

Owner request: far too many tests ran for every change. Three owner decisions,
recorded in [ADR-0144](adr/0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md)
with the measurements in the
[speed dossier](research/2026-10-19-test-suite-speed.md):

- **Complete gate for releases only.** `python3 tools/run_package_gate.py`
  plans a routine package from its changed paths (`tools/gate_scope.py`):
  static checks always, then only the Rust packages, browser specs, production
  suites, corpus packages, bindings and website/story tests the change can
  affect; deleted files still select their readers, and a path without a rule
  selects everything. `--full` is the Alpha, Beta and release gate. Replayed
  over the last 40 commits on `main`, none needed the complete gate.
- **Browser cases run in parallel unless they measure time.** A
  `{ tag: "@exclusive" }` marks real-time cases; `tools/browser_lanes.py`
  refuses an untagged real-time primitive before the build, also through
  helpers, fixtures and methods. The decayed hash admission
  (`browser-lanes.json`) is gone. The parallel lane found two load-dependent
  races in `explorer-structure` (fixed in the spec), and 37 file-menu zoom
  cases left the exclusive lane by pinning a toast-free date instead of
  waiting 300 ms. Production suites keep one worker each.
- **One integration-test binary per crate** (356 → 37) and line-table debug
  information with optimised dependencies: cold test build 206.5 s → 118.0 s,
  rebuild after a `knx-core` change 194.6 s → 10.0 s, test run 467.2 s →
  145.0 s, target directory 71.6 GB → 6.7 GB, same tests passing.

Two independent reviews were closed test-first. The complete gate on the
reviewed candidate passed in 37.4 min of stages (web 1,770.8 s, Rust tests
4,993 passed, corpus 155 of 155); the change-scoped gate for the last guard
fix ran 11 stages and every default spec, but no Rust, corpus or production
stage, in 13.2 min of work.

## Hall of Fame bot comments name only fresh medals (19 October 2026)

Owner bug report: every XP update from the scoreboard bot repeated the
contributor's whole medal list. The board table now is the only place that shows
the collection. A completion note names a medal only when that completed report
reached a resolution milestone (awards ordered by closure time); a PR credit
comment names only the PR and/or merge medal that PR reached (PRs by number,
merges by merge time). Without a milestone there is no medal line, and the
"trophy cabinet is warming up" filler is gone. Report-count medals stay on the
board only. Thresholds, XP, ranks and the snapshot schema are unchanged; the
first refresh edits each maintained PR credit comment once, while posted
completion notes stay as they are
([contract](contracts/community-recognition.md#medals-in-bot-comments-owner-amendment-19-october-2026)).
81 community tests pass; the live workflow run follows the push.

## UI feedback set delivered — UI-05 and whole-set corrections (19 October 2026)

UI-05 (word-safe resizable tables) and the corrections from the
[whole-set review](goals/ui-feedback-2026-10-18.md#delivery-record-1819-october-2026)
are on `main`. Resized-table row actions keep the 24 px target floor of UI-02.
Tables and the File menu convert DOMRect and window readings to CSS layout
pixels under application zoom (80–150%). File submenus share one 220 px minimum
and expand inline where no side keeps it. The open menu paints above the
toolbar. Inline submenus open by click, tap or keyboard only. A press with
mouse, touch or pen no longer folds the open submenu before it lands. The first
level measures itself and scrolls or moves left when it does not fit, so Logout
stays reachable at 150% in a 601 px high window. Presentation only: no project
data, API, schema or manufacturer grid change.

Evidence: the full 16-stage gate on the delivery merge `951a5451` passed (web
receipt complete, success and frozen with 66 of 66 stages; 4,925 Rust tests;
private corpus 155 of 155; clippy, fmt, bindings, layering, headers, anchors,
ledger and 223 tooling tests). Worker runs before it covered 4,000 Vitest cases
and the full default browser lane (842). Every fix round was reviewed by the
other vendor; the last review accepted without findings. Zoom behaviour is
measured in Chromium only, not in WebKitGTK
([KL-130](limitations/KL-130-ZOOM-application-zoom.md)). Open follow-ups: pane
drags under zoom and a table zoom case through the scale setting
([open work](OPEN_WORK.md#ui-feedback-packages-18-october-2026)).

## Read-only declared-segment memory evidence, UP258 stage 0 (18 October 2026)

`knx device diagnostics --profile system2-declared-segments-v1` derives the
memory ranges a System-2 application's load procedure declares from the
product database (UP 258D12: four ranges, 1514 octets), reads them with
`A_Memory_Read` only after the device shows the asserted mask, manufacturer and
hardware type, and saves archive format 2 with per-chunk items and per-range
coverage. Format 1 is unchanged. The plan is opaque (ProductDB-derived only),
and the strict reader re-checks identity admission, identity-item and chunk
order and stop reason
and refuses duplicate ZIP entries from the raw central directory, which it
binds to the `zip` crate's (no comment, same offsets). Every observer
snapshot, including one with a request in flight, is exportable; a PID 78
comparison the archive cannot record (over 64 octets) is refused at
derivation. Simulator,
built-CLI and exact-package tests only; no hardware run, write or web entry.
[ADR-0143](adr/0143-declared-segment-memory-evidence-profile.md),
[contract](contracts/device-diagnostic-export.md#profile-system2-declared-segments-v1-format-2).

## Offline System-2 source-field preview (18 October 2026)

The existing byte-order probe now supports an offline preview and a distinct
bounded System-2 source-selection preview. The same evaluator, parameter checks
and project mapping are reused; no network client is created in preview mode.
The exact Siemens A/B source fields are covered by the public-package regression;
no real read, image, write or recovery route is enabled.
[Contract](contracts/system2-source-probe-preview.md).

## Parameter caption final review evidence — issue #26 (19 October 2026)

Added independent row-only and column-only HTTP caption-budget regressions. Each
asserts the exact admitted axis DTO and whole-panel exhaustion with no project,
revision or history mutation. Each test passed on the already-correct
implementation, then failed with HTTP 200 instead of 400 when only its axis's
copy charge was locally removed. Both charges were restored byte for byte. This
closes missing discriminating evidence, not a newly discovered axis defect.

The conservative delimiter fallback is unchanged: even a resolved value such as
`Kitchen {{North}}` may use the localized label. EN/DE tooltips now describe this
display policy neutrally rather than declaring the result unresolved. Tests cover
stored-value literals and mixed supported/unsupported templates, retained exact
resolver provenance, original text/template tooltips and search. Contracts and
the user guide document the literal-delimiter limitation explicitly.

Focused checks pass: 98 tests in 11 HTTP targets (15 caption cases), 110 UI tests
and 24 i18n tests (JSON reports), production build, application and fixture
TypeScript, owning-server all-target Clippy, formatting and documentation. Four
localized tooltip tests first failed on the old wording. Independent re-review
and the integrated production/package gate remain with the integration owner;
no publication, main merge, bus or hardware action is claimed.

## Parameter caption review corrections — issue #26 (18 October 2026)

Independent review found a post-commit projection refusal, unmetered caption-copy
fan-out, unsearchable localized fallback labels and an overbroad translation
claim. Parameter writes now preflight the exact reconciled command through the
existing stack on a project copy and construct the full response before durable
publication. The request's existing caption budget survives through every DTO
text/provenance copy, including separators and grids, with no ceiling increase.
Search includes the displayed localized fallback beside original source context;
caption translation boundaries are explicit in both contracts.

Named REDs were observed for all four findings. Focused correction checks pass:
96 tests in 11 HTTP targets, 106 web tests (JSON report), production build,
fixture TypeScript, owning-server all-target Clippy, formatting and documentation.
The stronger HTTP refusal case also includes object reconciliation, bound and
unbound callers, revision/history/generation and persisted-byte invariance.
Optional browser rerun: cancelled while queued on the foreign lease, zero cases
executed; not browser acceptance. Independent re-review and integrated full
package/browser acceptance remain pending with the integration owner. No
publication or hardware action is claimed.

## House devices read with the diagnostic export (18 October 2026)

Owner go: the shipped `knx device diagnostics` ran once against all 34
contactable devices of the maintainer's house (`1.1.220` excluded, never
contacted); nothing written. 30 complete reads, one zero-element PID78 refusal,
two `MV-0012` sensors ending the connection after PID12, one rejected
connection; 29 of 29 declared PID78 comparisons match the `[A]` rule
([evidence](research/2026-10-18-house-device-diagnostics.md)). The private gate
`device_reports_corpus` gains a pinned house set (34 archives byte-identical
after re-bundling; dropped and tampered archives red). No production code or
admission change.

## Eleven more real diagnostic exports reviewed (18 October 2026)

Eleven further read-only exports received by email (Hager, Berker, MDT,
Video-Star) were reviewed and recorded in the
[evidence note](research/2026-10-18-device-diagnostics-eleven-devices.md). First
real `refused-ambiguous` replies (two PID78 refusals); eight of eight declared
PID78 comparisons match the `[A]` `property_matches` shape; a hidden
`HWT_Check` default equals its device's reply (G10 data point). New private gate
`knx-app` `device_reports_corpus`: the strict reader accepts all twelve real
archives and the writer reproduces each byte for byte (three negative controls
red). No production code or admission change.
## Parameter section captions — issue #26 (18 October 2026)

The parameter workspace now applies ADR-0126 to block captions and activated
separator text, including separator grid labels. It reuses the device evaluation's
exact source-reference resolver: stored text overrides, then product defaults;
explicitly empty values stay empty and ambiguous references decide nothing.
Additive fill records preserve the original template and source. Navigator,
selected heading and Restricted captions use readable localized fallbacks for
empty, marker-only or unresolved text, with explanatory source tooltips. Source
metadata, device configuration, field identity and existing write/undo paths stay
unchanged. See the [caption contract](contracts/positional-text-templates.md#parameter-workspace-captions-issue-26).

Focused regression evidence: the original raw-caption assertions failed before
the fix; 69 HTTP tests (eight new caption cases), 101 focused web tests,
production build, fixture TypeScript check (with a deliberate failing control)
and touched-server all-target Clippy pass. Integrated full-gate acceptance and
publication remain separate; this entry does not claim either.

## Table geometry at application scale — UISET-01/05 (18 October 2026)

Table fitting, token/icon floors, pointer deltas and ARIA widths now share layout
pixels, preserving browser-local width preferences through application scaling.
Root-style appearance changes refresh geometry as well as token caches. The
overlay browser check temporarily removes background inert during its stacking
probe. Focused unit and browser regressions cover scale endpoints and saved-width
restoration; [contract](contracts/table-column-widths.md). No project data,
manufacturer grid or native-shell compatibility change.

## UI feedback wave 3 integration acceptance (18 October 2026)

UI-02 (controls/separators), UI-03 (File submenus) and UI-06 (labelled,
bounded topology) passed independent package reviews and the additional
read-only acceptance audit with no findings. Frozen candidate `471ca75a`
passed all 16 gate stages: web receipt complete/success/frozen, 4,915 Rust
tests, 153/153 private corpus tests in 33 targets, Clippy, bindings, repository
checks and 223 tooling tests (one existing skip). Main-first merge `13510653`
preserves newer website/research/claim entries. App, core and tooling inputs
are identical to `471ca75a`; documentation/build/type/Vitest/website/tooling
and repository checks are delta acceptance, not another full gate. The first
tooling attempt preceded Node dependency installation and failed three browser
loader tests; the corrected full tooling rerun passed. UI-05 is not part of
this integration and remains in independent review. Native WebKitGTK/Tauri
interaction evidence is still absent; no full native UI parity claim.

## Bus monitor review fixes — UI-01 (18 October 2026)

ADR-0010 decoding policy stays unchanged. A separate Core diagnostic helper
collects program defaults behind cleared instance DPTs (ADR-0027); monitor rows
name them without decoding with them. Captured unresolved reasons now separate
no project, address outside the project, known address without a type, and
cleared program defaults. Server context comparison includes diagnostic changes;
HTTP, diagnostic bundles and EN/DE labels retain the captured evidence. Synthetic
Core/HTTP/panel regressions and default/reduced-motion browser variants cover the
change. [Contract](contracts/dpt-selection.md#monitor-resolution-reasons-ui-01-18-october-2026).

## Bus monitor unresolved reasons — UI-01 (18 October 2026)

Read-only source inspection found a genuinely undeclared linked measurement,
not an omitted resolvable DPT. Resolution remains shared and never infers a type
from names or sizes. HTTP/diagnostic rows now distinguish a session without a
project from a missing DPT; EN/DE labels and unknown legacy reasons follow that
captured evidence. Raw bytes and previous interpretations survive project edits
and explicit future-context refreshes. Synthetic Core, HTTP and panel regressions
cover the production path; [contract](contracts/dpt-selection.md#monitor-resolution-reasons-ui-01-18-october-2026).
No bus/hardware operation or deployment is part of this change.

## UI-04 project status storage metrics (18 October 2026)

The Overview measures the current on-disk size of the open project's native
file (which may include unsaved recovery-journal growth), its separate unsaved
flag, and the active product database including SQLite side files. It counts
manufacturers owning products or programs separately from project devices.
New projects have no invented file size; pending, unavailable, stale and failed
reads are explicit. Response admission binds server, project and revision.
[Contract](contracts/project-status-metrics.md). Focused HTTP and Vitest tests
cover save, Save As, failed save, native open, import, project switch, catalogue
install, empty/unavailable databases, count identity and locale formatting.

## Reversible marketing headline entrances (18 October 2026)

Website-only: every DE/EN marketing h1/h2/h3 gains a SessionRadar-style staggered
650ms fade/vertical/3D entrance. Viewport exit reverses the same timeline;
interrupted re-entry reverses in place without a jump. Existing 720ms
random-direction glyph rolls remain, with separate transform ownership and no
ambient motion during entrance/exit. Motion-off and lifecycle cleanup restore
all text; no-JS/API-failure fallbacks, accessible labels and layout are retained.
No app, core, media, legal/privacy or approved-Story change.

Local acceptance: website unittest 29, Story unittest 76 (one existing skip),
preview Chromium 654 checks in nine recipes and release Chromium 116 checks in
three recipes; deterministic release rebuild, syntax/docs/local-info green.
New recipe `website/tests/verify-headline-reveal.js` covers viewport boundaries,
interrupted reversal, disposal and readable cancellation/fallbacks. Independent
review completed; the owner accepted the print/PDF limitation and authorized
commit/push without a print fix. See [website contract](WEBSITE.md#reversible-headline-entrances-18-october-2026).

## UI-06: labelled object counts, bounded topology cards (18 October 2026)

Presentation only, no API, schema or stored-data change. Owner request 9. The number
on the right of a device tile (Topology, and the same tile in Buildings) is
`DeviceNode.com_object_count`, the length of the device's communication-object list
regardless of `is_active`; it was shown bare. It now reads "N communication object(s)" /
"N Kommunikationsobjekt(e)" through the plural catalogue (review fix: the full unit is
visible, no hover text); on a narrow tile the count sits under the name. The workspace
arrow/Home/End handler no longer lands on the hidden entries of the closed `+ New` menu. Line cards are a grid with a 28rem column ceiling and device rows a
32rem ceiling, so name and count stay a short glance apart on a widescreen; long
names ellipsise with a title. Still cards, not a data table: no column resizing.
[Contract](contracts/topology-metrics.md).

## UI-03: File menu with animated submenus (18 October 2026)

Presentation and keyboard behaviour only, no API, schema, theme-pack or stored-data
change. Owner request 1 of the [UI feedback set](goals/ui-feedback-2026-10-18.md#ui-03--file-menu-with-animated-submenus).
**History & comparison**, **Help & diagnostics** and **Import & export** are now
submenus of File that open to the right (flip left when the whole panel fits there,
shrink at mid widths, expand inline under the row at 650 px and below); New project,
Open, Open (.knxdb) and Save As stay on the first level. Every entry is the original
button with its original handler, so native/web differences, disabled states, unsaved
change guards and dialog lifecycles are untouched; closed submenus stay mounted so an
in-flight import or comparison survives. The menu holds declared entries only: the
project comparison's result moved out of the History panel into its own modal dialog
(shared `Overlay`, opened by **Compare with…**, which closes the menu like every other
entry), with the same file choice, counts, tables and refusal text. File is a WAI-ARIA menu button:
the control carries `aria-haspopup` and `aria-expanded`, entries are declared explicitly
(`FileMenuItem`, `useFileMenuItemProps`; no DOM tagging), each level has one roving tab
stop, ArrowDown/ArrowUp/Enter/Space on File open the menu onto an entry, arrows and
Home/End move, `→`/Enter/Space open a submenu, `←`/Escape close the innermost level,
and Tab or Shift+Tab from any focusable element in the open menu close the whole menu
and move on from File. An action that closes
the menu rests focus on File first, so every dialog returns focus there. Open panels are
re-placed when they grow (`ResizeObserver`); on short windows (600 px and below) the menu
scrolls and submenus expand inline. Hover opens after 120 ms and survives a diagonal
move, and the 140 ms slide is removed by motion level Off and reduced motion.
Correction round (18 October 2026, UISET-02/03): a flyout has one minimum width
(220 px, stated to the stylesheet as `--file-submenu-min-width`); where neither side keeps
it, that submenu expands inline for as long as it is open instead of leaving the viewport.
The open menu now has `z-index: 6` inside the toolbar, so disabled toolbar controls no longer
paint over its first rows; dialogs stay above it. Hover is for flyouts only: an inline submenu
(narrow, short window, or a mid-width one with no room beside its row) opens and folds by
click and keyboard, because its opening moves the rows under a resting mouse and the hover that
followed used to swap the submenu just clicked (and a mouse press on a sibling row no longer folds the open submenu before the click lands); the layout is read from the rendered panel, not
a copied breakpoint. Second correction round (19 October 2026, UISET2-01/02): placement is
zoom-aware; DOMRect and window readings are divided by the application scale, so the 220 px
minimum, gap, margin, width cap and upward shift share CSS layout pixels at 80–150%, and the
menu's viewport-unit limits are divided by the scale too (Chromium-measured units, KL-130). The
room beside a row is taken from the row, not from the panel mid-slide, and the bottom check
follows the capped height. The press guard now covers touch and pen: `mousedown` re-arms it,
because a tap's focus move comes after `pointerup`, and a press abandoned outside the window is
forgotten on `blur`. Third round (19 October 2026, UISET3-01): the first level is measured in
layout pixels too. Past the bottom margin the whole menu scrolls and folds its submenus inline
(at 150%, Comfortable density and a session, **Logout** sat below a 1440 x 601 window the 600 px
window rule calls tall); past the right margin it moves left (a 660 px window at 150%). The
window rules stay as a floor. Not done: type-ahead, native WebKitGTK/Tauri runs. [Contract](contracts/file-menu.md).

## UI-05 appearance relevance (18 October 2026)

Ancestor geometry edits, including virtual-history spacer padding, retain table
measurement caches. Typography/custom-property changes still invalidate them.
Comparison reset owners require field changes, and repeated-table reset labels
visibly name their table kind in EN/DE. The independent browser oracle preserves
asymmetric horizontal insets. [Contract and regression coverage](contracts/table-column-widths.md).

## UI-05 shared reset controls (18 October 2026)

Comparison/history field tables, flow-inspector flag tables and address tables
across installations each use one owner reset above their repeated list.
It restores the shared preference and
remeasures session floors in every mounted instance of that table kind.
Per-column resets retain floors; single-instance controls remain available.
[Contract and regression coverage](contracts/table-column-widths.md).

## UI-05 invalidation and independent fit proof (18 October 2026)

Adds colspan remapping and attribute-only observation of each table's mounted
ancestor chain, preserving targeted ordinary-cell invalidation and ignoring
geometry-only pane edits. Restores surface-specific padding through the existing
CSS cascade. Browser fit checks use independent intrinsic measurement plus
reviewed rendered budgets and negative controls; flow now fits its inspector
with its original padding. Dynamic ancestry changes require remounting.
[Contract and verification boundaries](contracts/table-column-widths.md).

## UI-05 density and stability fixes (18 October 2026)

Restores shared density/theme padding for header and body cells and accurate
EN/DE location, topology and programming headings. Mutation targets invalidate
only affected cells without full-cell serialization; unchanged cells reuse their
measurements. Sort icon slots remain reserved, and measured floors only grow
until table reset/remount. Instances keep independent measured maxima.
Production EN/DE 1440/1920 fit-budget checks preserve local scrolling where the
word-safe floor sum exceeds the available pane; 1440 px cannot fit the tested
device overview or German flat objects with navigation present.
[Exact behavior, measured budgets and tests](contracts/table-column-widths.md).

## UI-05 measured word-safe fitting (18 October 2026)

Replaces the earlier number/DPT fit constants with one measured header/cell
word floor for every adjustable column in the shared adapter, capped at 320 px.
Cached computed-font measurements preserve ordinary words while narrow panes
scroll locally; explicit resizing retains the stored-preference delta model.
Hidden panes defer measurement; unchanged cells reuse their measurements.
Compact device headings retain full accessible names/tooltips, and table cells
avoid redundant padding to preserve the tested wide fitting budgets.
Browser regressions cover EN/DE grouped and flat objects at 360 px and status
at 640 px, the long DE address-tree DPT and monitor decoded label at 400 px.
The unchanged device-editor layout spec and 1440/1920 fixture fitting remain
acceptance checks. [Precise behavior and verification](contracts/table-column-widths.md).

## UI-05 second review fixes (18 October 2026)

All column preferences can shrink below their defaults, while unresized tables
retain pane fitting. Header stacking is isolated per scroll container. Separators
announce measured pixels; active drags commit on unmount and ignore resize keys.
Sort indicators remain outside clamped heading labels. The matrix owns its fixed
object width. The [contract](contracts/table-column-widths.md) records the accepted
one-frame pane-fit settling behaviour. A gate regression fix gives grouped and
flat communication-object DPT columns a 114 px fit floor while preserving their
100 px manual minimum; narrow panes scroll locally. The unchanged EN/DE
device-editor layout spec checks intact DPT/master-text words at 360 px.

## UI-05: browser-local table column widths (18 October 2026)

Local-only presentation change: one width hook and accessible header separator,
UI-02 paint, validated versioned localStorage, pointer/keyboard resizing and
column/table reset. All 18 native table surfaces use the adapter; the matrix's
select/address/name columns resize while its per-object columns remain fixed.
Review fixes restore container-filling defaults, retain view-specific keys,
coalesce drag previews with one final write, and improve accessible names/help. [Contract and complete coverage](contracts/table-column-widths.md).
Focused hook/component/production tests and browser proof cover sortable devices,
comparison and nested virtualised history tables. No API, domain, schema,
parameter-grid, project-write or KNX bus change.

## UI-02: compact button variants, visible navigation separators (18 October 2026)

Presentation only, no API, schema, theme-pack or stored-data change. Owner
requests 5–7 of the [UI feedback set](goals/ui-feedback-2026-10-18.md#ui-02--compact-distinctive-controls-and-visible-navigation).
One shared Controls block in `styles.css` now owns the look of ordinary buttons
(a listed set of tabs, rows, chips and link-like buttons keeps local paint):
secondary by default, `primary-action`, `quiet-action`/`icon-action` and `danger-action`,
compact (32px compact density, 12px padding) with colour-only hover and a small
press scale that motion Off and reduced motion drop. Dialog confirmations name
their primary or destructive role; the inspector's Delete reads as destructive,
and the toolbar's Save no longer turns pale under the pointer.
The navigation splitters draw a hairline at rest and an accent rule on hover,
focus and drag; without a project the sections carry the same rule. The routine
"Source: device instance" badge left the field headings and stays in the DPT and
description technical references and in the data. After two review rounds the
destructive answers found by the sweeps paint as destructive, each with a test:
note-disposition, project-history, plug-in data, Function delete and forced
overwrite, note and instruction removal, armed scan-exclusion removal, line-scan
reconciliation that deletes, both parameter-conflict answers, placement repair,
language-pack removal, preset over a filled group list, forgetting the legacy
password, bulk delete. Help tips, toast dismiss, import-finding links and table
device links have 24px targets; feature editors use the shared variants; a
browser matrix checks the named dialog, toolbar, inspector and table controls'
roles, focus and size and the separators in all five shipped themes.
[Contract](contracts/controls-and-separators.md).

## Device-empty topology ranges (18 October 2026 source, issue #25)

The Topology diagram hides lines without devices and areas without a populated
line. Zero-object devices and unassigned devices remain visible. Empty ranges
remain in the project and explorer, and selecting one there retains its editor
and the header's exact device-add target. No domain, persistence, export or
protocol change. [Contract](contracts/topology-metrics.md#device-empty-ranges-issue-25-source-builds-after-alpha-14).
Focused verification: named pre-fix RED; 22 component tests, 23 fixture browser
cases and eight production heading/hover cases passed. Independent review
accepted the corrected full diff. Integrated package acceptance is separate.

## Manual, README and screenshots after Alpha 14 (18 October 2026)

Docs and media only, no application change. The manual's 46 stills, the four
group-address views and the three workflow GIFs were recaptured from the server
and frontend extracted from the published `knxbench/knxbench-server:0.1.0-alpha.14`
image (OCI revision `8d104e2f`), in a loopback-only namespace, with the fictional
Sample house; [media manifest](assets/media-manifest.json). The two manual capture
specs now open the Alpha 14 closed-by-default navigation blocks, explorer branches,
address ranges and parameter areas the way a reader would, and the login check reads
the expected version from `MANUAL_EXPECTED_VERSION` (default `0.1.0-alpha.14`).
README stills now use these manual captures. Version labels in the manual say
"since Alpha N" instead of "source builds after Alpha N-1"; renamed headings keep
their old anchors. Stale claims fixed: "no version history yet" (README), the
"Alpha 12 uses schema 23" upgrade notes, palette and help-topic counts (22 commands,
13 topics) and the size limits that `main` raised after the tag (now labelled as
source-only, with the Alpha 14 bounds next to them). The website, the media
inventory page and the Alpha 14 release-media set belong to the parallel website
claim of the same day.

## Census-3 fixes and storage-ceiling size bounds (18 October 2026)

Follow-up to [census 3](research/2026-10-18-product-database-programmability-census.md#follow-up-delivered-18-october-2026):

- **G11:** legacy EX-IM tables match case-insensitively, and case-only
  duplicates are refused. Two real `.vd5` import their programs instead of
  reading as empty.
- **G10:** an unwritable placement is named before the value type (70 real
  `0705h` refusals now name the `Property` placement).
- **G13, product DB v25**
  ([ADR-0141](adr/0141-placements-and-enumeration-attributes-reach-columns.md)):
  placement and enumeration attributes reach typed columns, and the unknown
  report retires them and reports what no column stores. v24→v25 migration
  equals a fresh install on six real packages.
- **G12** ([ADR-0142](adr/0142-file-size-limits-follow-the-storage-ceiling.md)):
  every single file or member may reach SQLite's 1,000,000,000-byte value
  limit (the new `knx_core::limits`), and expansions 4 GiB. This covers product
  packages and both profiles, legacy files, editor/grid/master/contribution
  scans, project containers, manufacturer files, selective import, payload
  downloads (server and web client) and project history. The scheme-evidence
  budgets follow too. The ZIP member count is the 16-bit format maximum. All
  seven real packages above the former defaults install with the default
  profile.

## Alpha 14 preparation (18 October 2026)

Owner instruction: wait for P1 and the product-database series, then prepare and
publish Alpha 14. Both prerequisites are on `main` (P1 `4b961475`, series
`bfc0a9c5`); cutoff `a0c2537d` adds the Docker frontend-stage fix found by the
first candidate's dry run, the native-spec fold fix found by the second
candidate's gate, two test-race fixes (ux-feedback, knx-net IGMP snapshot) and
the v24 product-matrix re-pin found by the third (candidates `b08ed794`,
`f59ace9b`, `54010888`, `bb7c896e` and `ec52706e` are retained, not tagged). Versions `0.1.0-alpha.14` in the four app crates
and the web package. Native/model16 unchanged; product database 23 → 24.
Published on 18 October 2026 from the gated commit `8d104e2f` (36/36 gate stages,
both dry runs clean, tag workflows successful): [source acceptance](releases/alpha14-acceptance.md),
[external publication](evidence/release-alpha14-2026-10-18.json) with an isolated
Docker smoke, a native-13 → 16 migration probe and a headless AppImage startup;
[scope and limits](releases/alpha14.md) remain explicit.

## Docker frontend stage sees the shared temp helper (18 October 2026)

The Alpha 14 Docker dry run failed on both architectures: since the shared
Chromium temp helper moved to `tools/browser_tmp.cjs` (17 October), the web
build's typecheck reaches it, and the image's frontend stage copied only
`apps/knx-web`. The stage now mirrors the repository layout and copies the
helper and its declaration; the runtime takes `dist` from the new place.
`tools/tests/test_docker_frontend_context.py` checks every relative import that
leaves the app against the stage's `COPY` lines (positive control on the known
helper; mutants without the helper or its declaration fail). A local
`docker build --target frontend` produced `dist/index.html`. Alpha 13's image
was not affected.

## Packed time units, row indentation and object size (18 October 2026)

Census steps 3 and 4 (first part). **Packed time:** `chooseParameterEditor`
admits the decimal-count control only for the six plain `TypeTime` units the
collection declares (1,004 of 1,456 declarations); packed units (452, 79
programs) and unknown units fall back to `unsupported-codec`, which shows the
original value and offers no input
([contract](contracts/parameter-editor-admission.md#packed-time-units-fall-back-2026-10-18)).
**Indentation:** the grid reader also collects `ParameterRefRef/@IndentLevel`;
a field gets `FieldLayout::indent_level` (DTO `indentLevel`, 1 to 8) only when
it has one block and every occurrence there declares the same valid level; the
workspace indents such rows in the row view
([contract](contracts/parameter-workspace.md#row-indentation-census-p4-18-october-2026)).
**Object size:** `ComObjectNode.size` (optional, `ComObjectSize_t` spelling)
from the size the product data resolves; the object table names it in the DPT
cell. No schema change. Still open: `SuffixText` (needs a product-database
column), inline sections, `ShowInComObjectTree` folders, object priority.
Tests: Vitest `plan` +3, `ParameterEditor` +1, `ParameterWorkspace` +1,
`ComObjectTable` +1; ProductDB `parameter_layout` +3; HTTP `http_parameter_grid`
+1; projection +1. Mutants: packed fallback and three indent mutants killed.

## Vendor ZIPs of product packages (18 October 2026)

Census step 2 ([ADR-0140](adr/0140-vendor-zips-of-product-packages-install-as-a-whole.md)).
A ZIP without its own `knx_master.xml` that holds `*.knxprod` members was refused
as "missing knx_master.xml"; it now installs as a whole. `knx_productdb::
install_product_file` runs the ordinary package path for every inner package,
byte-exact, in one transaction (source name `<zip>!/<member>`); a refused inner
package refuses the ZIP and names itself; inner packages share one expansion
budget of the chosen profile; other members are reported with size, SHA-256 and
kind (`file`, or `appleDouble` for macOS sidecars recognised by RFC 1740 magic)
and are not stored. The installer itself refuses such a ZIP as
`WrapperArchive` naming its packages. Package code was restructured without a
behaviour change: admission, the checked archive view and the member read are
shared functions, and the inner install runs in the caller's transaction.
Surfaces: `knx products ingest` takes `.zip` (a project archive under that name
still reaches the project import) and `--allow-large-package` for vendor ZIPs;
the previewed catalogue update (CLI and web) previews and applies vendor ZIPs
(`preview.wrapper`; the apply response is now `packages: [{path, report}]`); the
web dialog lists inner packages and every file not installed (EN/DE). The web
keeps the standard profile. Real collection: 33 vendor ZIPs, 30 install with the
standard profile, 2 with the large one, 1 correctly refused (backslash paths);
47 inner packages ([census result](research/2026-10-17-product-database-gap-census.md#result-vendor-zips-18-october-2026)).
Tests: ProductDB `wrapper_packages` 13, CLI `cli_vendor_zip` 4 and
`cli_large_package` +1, HTTP `http_catalogue_update` +2, Vitest
`CatalogPackageUpdate` +4. Mutants 6/6 killed (shared budget, case-insensitive
name, other members, wrapper refusal, inner refusal naming, one transaction).

## Object-local function dropdowns (17 October 2026)

Package P1 publishes Codex's unpublished feature `99dd6e65`, cherry-picked onto
the current parameter workspace. The separate *Device functions* disclosure is
gone: an expanded communication object offers the supported manufacturer
choice parameters that control it, directly below DPT, as the same bound row
the Parameters tab uses (revision-bound write, inline errors, Undo). ProductDB's
structural layout now keeps the exact object references each choice reaches
through choice dependencies (`FieldLayout::controlled_object_refs`, next to the
grid `cell`); the server maps them to configured object IDs
(`field.controlledComObjectIds`) through the existing declaration/alias
resolver. Missing, ambiguous and module-local mappings give an empty list and
no control, never a guess; block- or field-restricted parameters (ADR-0139)
stay out. Grids and separators are unchanged: they render the Parameters tab,
the object editor shows only rows. No schema, dependency or Core change.
Codex's source acceptance on the old base is in its
[log](../.ai/logs/2026-10-17_codex_object-function-dropdown.md).
Tests: ProductDB `parameter_layout` +1 (dependency propagation), HTTP
`http_parameter_panel` +1 (no module mapping) and three exact-ID assertions,
Vitest `ComObjectFunctionControls` 5 (block access case added), Devices browser
spec reworked (area register closed on the first visit, control below DPT,
write shown back in the open area; EN/DE × 1440/400). Six mutants killed, two
of them only after the new HTTP test. Package gate on `4b961475` (16 stages, 0
failures, inputs frozen): web 64/64 stages (Vitest 3,732, browser 596),
Clippy `-D warnings`, workspace tests 4,857 passed / 0 failed / 212 ignored,
xtask checks, documentation, tools 220, private corpus 153/153.
[Contract](contracts/parameter-object-refresh.md).

## Test-gate hygiene (17 October 2026)

Package P4 ([goal file](goals/open-work-2026-10-17/p4-gate-hygiene.md)) fixes
four gate defects, each in its own commit and each with a reproduced failure
first:

- **Corpus-output refusal through a symlink (safety bug).** The private-corpus
  output guard compared a resolved output directory with the corpus root as
  the caller spelled it, so a root named through a symlink waved an output
  inside it through. Both sides are now resolved; the regression test names the
  root through a symlink explicitly (independent of the runner's `TMPDIR`). A
  sweep of the other "output must not land in the source" guards
  (`tools/run_web_gate.py`, `website/build.py`, `knx-server` path confinement,
  `tools/documentation_history.py`) found them already comparing resolved paths.
- **Linear time proven by counting.** The hostile nested-ZIP test timed 71 µs of
  work against a one-second budget and failed on a loaded host; a 2 s debugger
  stall inside the window reproduces that. The ZIP validator now meters the
  bytes it inspects (test builds only) and the test bounds that count by the
  input size from 16 KiB to 2 MiB, with a 30 s backstop clock at 2 MiB.
- **`knx-desktop` stays in the workspace gate.** Its Tauri build script needs
  `apps/knx-web/dist`. The [Rust workspace gate](VERIFICATION.md#rust-workspace-gate)
  now builds the web bundle first, CI does the same before Clippy and tests, a
  tools test holds both to that order, and the build script names the missing
  command. No placeholder bundle, no `--exclude`.
- **Parallel browser lane re-admitted.** The lane fingerprint had moved with the
  temp-path tooling of 17 October, so every spec ran in the one-worker lane.
  The [lane review](evidence/browser-lane-review-2026-10-17.md) re-admits the
  same 45 reviewed specs after measured runs.

- **Four `knx-app` fixtures no longer assume a symlink-free `TMPDIR`.** The
  package gate under a symlinked `TMPDIR` found them: two backup-directory
  sync tests expected only the supplied path's chain although `sync_chain`
  also (by contract) persists the resolved chain, the alias test looked for
  the target's parent by its unresolved name, and the authorized-export
  confinement test put its manifest where the harness's no-symlink opener
  refuses it first. Each fixture now anchors on a resolved base; expectations
  and production code are unchanged, and three mutants (no resolved chain, no
  ancestors, a symlink-following export opener) are still caught.

Rust tests now pass with a symlinked `TMPDIR`; the package set's "resolved
`TMPDIR`" rule is removed.

## Block access and manufacturer separators in the parameter tab (17 October 2026)

Fix package 1 of the [product-database gap census](research/2026-10-17-product-database-gap-census.md)
([ADR-0139](adr/0139-block-access-and-separators-reach-the-parameter-tab.md)).
Product DB **v24** stores `Access` (blocks, separators) and `UIHint`
(separators) in typed `dynamic_node` columns, backfilled from the retained
bytes like v23; legacy EX-IM programs are skipped. The evaluator passes the
most restrictive block `Access` down the walk (modules included; a ref reached
along an open path stays writable) and lists activated separators in source
order. The server refuses writes below a `None`/`Read` block
(`parameterBlockAccessRestricted`, `400`, nothing written) and serializes
`field.blockAccess` and `section.separators[]`. The web workspace moves such
fields to the Restricted view with a group caption and shows headlines,
information and error texts and rulers between the rows of their area (EN/DE).
Tests: ProductDB `presentation_attributes` 4 (incl. v23→v24 rewind),
`dynamic_block_access` 5, `parameter_layout` +1, `scheme12_14` now pins the
scheme-12 separator `Access` as preserved instead of reported; HTTP
`http_parameter_block_access` 3; Vitest +13 (full suite 3,726). No browser
case yet. Boundaries:
[KL-003](limitations/KL-003-parameter-evaluation-and-editing-boundaries.md#product-database-gap-census-17-october-2026).

## Notes when deleting and in selective imports (17 October 2026)

Package P3 closes [`FUTURE-03`](status/LEDGER.md#snapshot-ids)
([ADR-0138](adr/0138-notes-get-an-explicit-decision-before-a-delete.md)). The core
names every note a command would strand (`CommandError::NotesOrphaned`: id,
title, target; other note faults read as plain sentences instead of debug
text). `knx_app::note_disposition` previews a whole delete (cascades included)
and plans one choice per stranded note (keep on the project, move to a surviving
element, delete) plus the delete as one `Batch`: one undo step that restores the
element before its notes. The eight entity delete routes answer `409
noteDispositionRequired` with the notes, the removed targets and the snapshot;
`POST /api/project/delete-with-notes` applies the choices bound to that
snapshot (stale `409`, invalid `422`, nothing written). The web app's Inspector
and bulk deletes open **Notes need a decision** (EN/DE, help topic *Notes when
deleting*); the CLI prints the named reason. Selective import carries source
notes along their remapped targets and needs `unselectedNotes`
(`skip`/`attachToProject`) for the rest, with counts in the preview
([admission contract](contracts/project-note-admission.md#delete-disposition-and-selective-import-notes-17-october-2026-adr-0138),
[HTTP](contracts/project-note-http.md#note-aware-deletes-17-october-2026-adr-0138)).
Tests: core notes 20 (6 new or changed), app disposition 9, selective import 24
(6 new or changed), HTTP 7, CLI 1, Vitest 17 new (full suite 3,713), browser 5
(EN/DE × 1440/400 plus cancel). Boundaries:
[limitation](limitations/project-notes-workbench.md#update-2026-10-17-deleting-a-notes-target-adr-0138).

## Plug-in data adoption in the server and the web app (17 October 2026)

Package P2 lifts the first half of [KL-186](KNOWN_LIMITATIONS.md#catalogue-kl-186):
`knx-server` gains `/api/project/plugin-data/{candidates,preview,apply,cancel}`
and the web app a dialog in the device panel header and the command palette,
with the CLI's list → preview → confirm contract
([contract](contracts/plugin-data-adoption.md#server-and-web-app)). Preview is
bound to `serverIncarnation`/`projectIncarnation`/`snapshotRevision`; the server
keeps the one shown plan, apply sends only its token, a stale token or project
is `409` with nothing written. Adoption stays outside Undo: apply first records
the project version *Before plug-in data adoption* (a `.knxdb` is required),
then persists the adopted working state with the undo stack unchanged
([ADR-0136 amendment](adr/0136-plugin-data-adoption-is-audited-opaque-replacement.md)).
Refusals keep a stable code (`RefusalDetail`) so the dialog words all 14 classes
in EN and DE. Tests: app 19, HTTP 12 (four mutants each killed by a named case),
Vitest 17 new (full suite 3,680 before the last two dialog cases), browser 5
(EN/DE × 1440/400 plus Escape-cancel), screenshots inspected; two layout
defects found that way (full-width radio, squeezed refused row) were fixed
test-first. Plug-in, ETS or device acceptance of adopted bytes stays unverified
(owner only). No schema, bus, device or vendor action.
## Declared parameter grids in the Parameters tab (17 October 2026)

Parameter areas whose product data declares a `Table`/`Grid` (`Rows`, `Columns`,
1-based `Cell`) now render as a matrix of the existing editable rows; every
other area keeps the row view with a hint naming why (no grid, unknown
layout/version/status, invalid declaration, unreadable source, server without
layout reports). ProductDB re-reads the hash-checked retained source with the
caption reader's ownership rules (now shared predicates) and its own budget; HTTP
adds versioned `section.groupLayouts` and `field.cell`; the web plans CSS
placement for the unchanged stable rows parent. No schema, Core, write-path or
manufacturer-specific change; nothing is derived from captions.
Focused evidence: ProductDB 949/0 (30 ignored) incl. 7 fictional grid fixtures
and 4 reader unit tests; HTTP grid 3/0 (separator-filter mutant caught); web
68/0 in four files; browser 4/4 EN/DE × 1440/400 (400 px overflow red before
the `position: relative` fix); real-data cross-check on 13 private members equals
the census exactly; release median 85.5 ms per `load` on the largest member.
[ADR-0137](adr/0137-declared-parameter-grids-reach-the-workspace.md),
[contract](contracts/parameter-workspace.md#declared-grids-adr-0137),
[census](research/2026-10-17-parameter-workspace-layout.md#grid-declarations-17-october-2026).

## KNX Functions in comparisons, MCP and the desktop; ledger residue reconciled (17 October 2026)

`MODEL-07` is closed (`236c64ae`): each installation's comparison has a
per-Function table (ETS id, else building-part path + name; ambiguity reported;
members by raw address, so a notation change is no Function change), rendered by
`knx diff`, the HTTP comparison, the web panel and MCP `diff_projects`. MCP gains a
ninth read-only tool, `list_functions`. The desktop surface is the shell's own
embedded server (ARCHITECTURE rule 3); a `knx-desktop` test drives Functions over
its `spawn_server` on loopback. 8/8 targeted mutants killed; package gate in the
[review](status/2026-10-17-ledger-residue-review.md). The same review reconciles
stale ledger Evidence for `FUTURE-01/02/03/05` and `PDB-04` and re-checks
`KL-170/171/172`, `PDB-04`, `GAP-T30-04`, `KL-110`, `MODEL-05` and `IMPORT-02`:
no new public primary source, lift conditions unchanged. No schema, bus, device
or vendor action.

## Whole Git history rewritten without local machine details (17 October 2026)

On the owner's decision (KL-185) every file version, commit and tag message of
`main`, the nine release tags and all local branches were rewritten with
`git filter-repo`; `main` moved from `c7b729fa` to `951553f9`. The audit added
categories the first scrub had missed (CPU model, distribution and desktop
name, an older kernel release, dash-joined path keys, the real gateway
address), so the tree changed in 15 files as well. 2,963 of 3,216 published commits got
new hashes (the 253 earliest kept theirs), none dropped; identities, dates and parents unchanged; every changed file on every
tip equals the rules applied independently; 0 residue in 37,503 objects.
[Receipt](evidence/history-rewrite-2026-10-17.json),
[commit map](history/COMMIT_MAP_2026-10-17.txt). 241 pinned GitHub links moved
to the new commits; 54 frozen records carry a declared redaction. The guard now
also knows the CPU model, distribution, dash-joined paths, Arch kernel releases
and a clone-local deny file. GitHub's pull-request refs and the fork keep the
old commits ([KL-185](KNOWN_LIMITATIONS.md#catalogue-kl-185)).

## Plug-in data adoption: edited editor bytes reach the working project (17 October 2026)

`knx adopt-plugin-data` and `knx_app::plugin_data` copy one device's plug-in
`BinaryData` blob and numbered `ExtraData` table from a source `.knxproj` into an
existing native project, verbatim and never executed. The preview names the exact
replacements and a confirmation token binds source, request and destination;
the plan refuses unless product and application references, blob/table
references and group-address links match. Superseded bytes stay in the project
and a digest-only record is written; rollback is the saved version, not Undo.
Verified: 17 synthetic application tests (with four mutation controls), 4 CLI
tests on the real binary, and on the private ETS6 export pair exactly one device
(SmartSensor EG) is adoptable with exactly its two entries replaced by the
edited archive's bytes. Not verified: that the plug-in, ETS or a device accepts
adopted bytes. No server route or UI yet. [Contract](contracts/plugin-data-adoption.md),
[ADR-0136](adr/0136-plugin-data-adoption-is-audited-opaque-replacement.md),
[KL-186](KNOWN_LIMITATIONS.md#catalogue-kl-186).

## No local machine details in the repository (17 October 2026)

Tracked files no longer name local mount points, checkout or worktree
locations, home or agent-profile directories, the host name or the kernel
release; 56 files now use placeholders (`<sources>`, `<profile>`, `~`,
`<host>`), and test fixtures use `/home/alice`. The Story browser check finds
Playwright through normal module resolution when `PLAYWRIGHT_DIR` is unset.
`tools/check_local_info.py` guards tracked files through the documentation
gate and checks commit messages on request. Eight records pinned by the two
reorganization receipts carry a declared `redaction`. No
application, KNX, dependency or schema behavior changed. Git history is not
rewritten ([KL-185](KNOWN_LIMITATIONS.md#catalogue-kl-185)).
[ADR-0135](adr/0135-repository-carries-no-local-machine-details.md).

## Chromium temporary paths: one allocator, no extra nesting (17 October 2026)

Developer tooling now distinguishes allocation parents from valid private
explicit leaves. Playwright configs, the measured gate and standalone repository
launchers share the Python allocator through one CJS guard; subprocesses reuse
parent-owned leaves and cleanup never deletes caller-owned data. One-off scripts
can use the managed `tools/browser_tmp.py --run` entry rather than another manual
TMPDIR workaround. No application, KNX, dependency or schema behavior changed.
Scoped verification: 198 tool tests (one pre-existing skip), including 14 temp
regressions and actual offline Chromium boundary/deep-root launches; production
build, all 20 derived typechecks and documentation checks. This is test-tooling
acceptance, not a full UI/Rust/corpus/release gate. See the
[contract](contracts/web-test-gate.md#recurring-chromium-socket-path-protection).

## Exact UP258 source recheck; programming still blocked (16 October 2026)

Fresh post-freeze worktree on `07a93451`: existing digest-pinned public source
regression strengthened to independent literal assertions for all 21 positions
and four regions. A second test uses the actual resolver/Dynamic evaluator for
the two prepared P903 configurations and pins unchanged source/database and
LegacyPatchAlways/public-planner refusals. No production change, complete real
image/plan/simulation, new recovery or hardware support. The first missing
legacy patch/default/activation rule remains a defining-evidence blocker.
Local 14-stage scoped acceptance passes: ProductDB 938/0 (30 regular ignored),
explicit public source 2/0, existing simulator 7/0 and memory backup 11/0,
Clippy/build/format/docs/five repository checks. The reused suites are not
UP258 whole-recovery or source-to-plan evidence; no full workspace/web/private
corpus or main merge is claimed.
[Audit and acceptance matrix](research/2026-10-16-up258-vertical-path-audit.md),
[local execution/self-review log](../.ai/logs/2026-10-16_codex_issue22-up258-vertical-audit.md).

## Alpha 13 published and externally verified (16 October 2026)

Immutable `v0.1.0-alpha.13` names `6105976f`; no new feature scope, device admission
or deployment follows. All 59 ordered source stages pass: Rust4724/0/210ignored,
components3632, browser947 in23profiles, software-server native24,
conventional corpus152/152 in33targets and explicit product16/16 in6targets.
The original Firefox setup refusal remains distinct from accepted32original plus
27continuation stages, with complete source/original-input equality.
Actual public downloads pass checksums/provider digests; MCP/AppImage embedded
versions match. Docker version/latest nativeamd64+arm64 indexes match; isolated
HTTPS/auth/save/reopen and authentic native13→16/history-count checks pass.
Headless Broadway API/frontend evidence is not FUSE/EGL/native visual/Orca proof;
portal warnings are retained. Live website36files/media equal accepted build.
Native/model16 and product23 stay unchanged. Selected dirty root and running user
container were not synchronized/replaced. [Scope](releases/alpha13.md),
[source acceptance](releases/alpha13-acceptance.md),
[external publication](evidence/release-alpha13-2026-10-16.json).

## Workbench configuration: integrated source acceptance

The five-part workbench change has targeted Core/API roundtrip and undo coverage,
2,775 owning Rust tests, 3,632 component tests and 911 browser cases across generic
and production profiles. Browser acceptance reconciles preserved partial runs:
failed geometry/helper/copy cases were corrected and re-executed; a release-test
request-completion race was synchronized without changing release behavior.
Previously passing scopes retain their original input identity; the corrected
scope and unmeasured tail were executed separately. This is not an uninterrupted
all-green run, a physical-device test, a release or universal product compatibility.
Actual merge `925d918b` additionally passes 4,724 workspace tests (210 ignored
in the regular run), all 152 selected private-corpus tests in 33 targets, owning
Clippy and all format/documentation/architecture gates. Original corpus inputs
and source snapshots remained unchanged. The integrated built shell also passed
the four new configuration browser cases. Full tracked frontend and tooling
inputs match the accepted feature tree exactly, so web results retain their
original provenance rather than being represented as a new merged execution.
The tested source was published with result-only documentation as `8e121074`;
this is a source delivery, not a new packaged release or deployment.
See [the editing contract](contracts/parameter-object-refresh.md).

## Recent deliveries

- 2026-10-16 — Workbench configuration source delivery: independently foldable
  navigation, consolidated File groups, manufacturer-declared operating-mode
  dropdowns beside objects, parameter blocks, and atomic parameter-driven
  missing-object reconciliation. Actual merged workspace 4,724/0, private corpus
  152/152; component 3,632/0 and reconciled browser 911/0 with original provenance.
  New module-instance construction and unsupported manufacturer scripts remain
  explicit limitations. No physical bus, ETS-export or packaged-release claim.


- 2026-10-16 — Explicit direct-Memory numeric fields entirely inside one octet
  now replace template bits without requiring default equality, reusing the
  existing importer/evaluator/writer for offline System1/System2 images. Named
  equality and default-width REDs preceded their corrections; bounds, masks,
  pointer/table protection, overlap and other source guards remain. Independent
  whole-segment oracles checked two related source candidates in four
  configurations, but those artifact successes are withdrawn: strict System1/
  System2 entries refuse unhandled LegacyPatchAlways, even on inactive parameters.
  The trial all-family guard is not accepted; it changed established Easy3
  source policy and broke original positive golden tests. That expansion is
  withdrawn, those goldens stay unchanged and Easy3 semantics remain unresolved.
  Current configured13-source refusals are7/2/3/1
  (fixup/legacy-patch/inferred-default/Union);zero complete real images. No new
  public admission, executable plan, recovery or firmware evidence.
  [Contract and acceptance boundary](contracts/offline-explicit-parameter-bits.md).
  Scoped branch12/12passes: ProductDB935/0/29ignored in56reports, exact
  public System2 source1/0, Easy3 scope regression and owning all-targetClippy/
  build. Earlier all-family/whitespace/compatibility attempts remain historical
  or superseded, never final acceptance. The actual scoped corpus initially
  returned151/152solely at the historical private-System2 first-refusal assertion;
  its strict exact-diagnostic/presence/DB-invariance correction passes ten focused
  stages without production changes. Actual merge`a22a6602`passes15/15,
  ProductDB935/0/29ignored, three explicit source regressions and the full regular
  private corpus152/152in33targets (release). Source is published with exact
  local/tracking/live readback; earlier151/152and interrupted attempts remain
  separate, not relabelled GREEN. No whole-workspace/web acceptance is claimed;
  [evidence](evidence/explicit-source-bits-2026-10-16.json).

- 2026-10-16 — Community PR credit comments extend the existing notification
  workflow: one maintained comment per eligible PR with current 5/15/25 XP,
  board total/title, counters, XP sources, earned medals and board link.
  Historical PRs are included; reopen/close/merge updates replace the same comment
  without stacking bonuses. Trusted-marker recovery, live target recheck and
  exact POST/PATCH readback prevent spoofed suppression and stale acknowledgements.
  Existing issue awards and schema remain unchanged. Scoped acceptance and live
  workflow verification are recorded in the [contract](contracts/community-recognition.md).
  No application/domain/dependency, bus, hardware or release change.

- 2026-10-16 — Exact UP258/issue #22 continuation: original MP p140 exposes
  inverted checksum-disable encoding in the offline absolute-property codec.
  Named RED preceded the correction, published as `6d26a1e3`. Actual merged
  acceptance passes Core863/0, Net property7/0, exact public package1/0,
  owning all-target Clippy and eight docs/repository/format/whitespace stages.
  The separate private evaluator experiment passes for both proposed values;
  no full workspace/web/private-corpus/hardware run is claimed. [Production decision](research/2026-10-16-up258-production-decision.md)
  inventories all 21 positions and concrete missing encoding/XML/resource/recovery
  rules. One [English reporter request](support/issue-22-up258-evidence-request.md)
  is prepared, not posted. This is not a complete image, production download or
  physical validation; issue #22 remains open.

- 2026-10-16 — System-300 S-mode property-table image adapter published as `af9431cb`: reuse existing
  Type6 link validation/encoding, separate immutable length/element payloads,
  explicit format and capacity. Intended missing-module RED, twelve literal/
  boundary/negative tests and owning Core862/0/0ignored in19targets pass;
  all-target Clippy passes. Framing and sender-order mutants fail the named
  assertion and exact sources are restored. Branch runtime acceptance reconciled:
  Rust4701/0/210ignored in347targets, Vitest3626/263files,17actual stages exit0;
  final whitespace failed solely on12regenerated bindings. Exact initial full
  source/Git restored, corrected whitespace passes; original refusal retained.
  All19actual merged stages pass on `782dbafd`: Rust4701/0/210ignored in347targets,
  Vitest3626/263files, isolated corpus152/152 in33targets;2943source/Git and487
  external commitments match. Publication adds only a foreign handover with full
  runtime equality and seven checks; exact main readback verified. No source
  translator, executable route or hardware admission.
  [Contract](contracts/offline-system300-property-tables.md) ·
  [Evidence](evidence/offline-system300-property-tables-2026-10-16.json).

- 2026-10-16 — Community Hall of Fame includes PR authors, total/open/merged/
  closed-unmerged counts, explicit XP sources and earned-only PR/merge medals.
  Open PRs earn 5 XP, merged PRs 25 total, closed-unmerged PRs 15 total; bonuses
  do not stack. Schema 2 migrates the existing schema-1 completion ledger without
  replaying notifications. Trusted default-branch Actions handles fork PR
  lifecycle events without executing their code. 59 community tests and real
  public API generation pass; scoped delivery acceptance is recorded in the
  [contract](contracts/community-recognition.md) and agent delivery log.
  No application/domain/build dependency, hardware or release change.

- 2026-10-16 — PR #24 public UP 258D12 evidence integrated and published as
  `678078b8` (review correction `b9ee5cf7`): digest-pinned opt-in regression
  checks the exact tool-metadata pointer/source order and unchanged planner/image
  refusals. Research distinguishes representative template observations from
  exhaustive counts, active Union values and actual wire-encoding evidence.
  All eleven scoped integrated stages pass: ProductDB 928/0, 29 ignored in 56
  targets; explicit public-package test 1/0, owning all-target Clippy, format,
  documentation and five repository checks. Three compiled metadata mutations
  are caught and production bytes restored. No production/UI/schema/admission
  change, full-workspace/web/private-corpus rerun, hardware test or release.
  Issue #22 remains open for actual programming support.
  [Dossier](research/2026-10-16-up258-public-package-evidence.md) ·
  [Scoped acceptance](evidence/public-up258-review-2026-10-16.json).

- 2026-10-16 — P6 shared FixupList guard published as `39c88aba` after two named
  runtime REDs: Easy3/System-2 now refuse unhandled direct declarations alongside
  System-1, before image assembly. Positive sources, empty/populated declarations,
  exact ancestry and unchanged DB tests pass; owning ProductDB928/0/28ignored in56
  targets and Clippy pass. All nineteen branch stages pass: Rust4689/0/209ignored
  in345targets, Vitest3626/263files;2934source paths and six selected external/Git
  commitments match. All twenty final merged stages also pass, including isolated
  corpus152/152 in33targets. Exact local/tracking/live main equality verified.
  Earlier incomplete merge attempt retained as interrupted, not accepted. This
  session closes with no further implementation dispatched.
  Seven original sources/36sites/eight function references correlate with the
  retained master but do not define a general transform/no-op rule. No fixup,
  complete real-image, executable or hardware support is claimed.
  [Contract](contracts/source-fixup-integrity.md) ·
  [Evidence](evidence/source-fixup-integrity-2026-10-16.json).

- 2026-10-16 — P5 bounded System-1 image entry published as `ccfe5b74`:0012/DefaultProcedure
  library entry reuses actual evaluation and Core Type-1/indexed association
  codecs, protects original source pointers and refuses unmodeled FixupList.
  Nine targeted tests,926owning ProductDB tests and all-target Clippy pass;
  a Type-2 substitution mutant fails as intended and exact source is restored.
  Explicit thirteen-source/two-package regression verifies named refusals, not
  complete real-product images:7FixupList/5default-bit/1mid-octet-Union. Initial
  omission-success is withdrawn. All nineteen branch stages pass: Rust4687/0/209ignored
  in345targets, Vitest3626/263files;2931source paths and six selected external
  input commitments match. All twenty actual merged stages also pass with the
  same Rust/Web totals and isolated corpus152/152 in33targets. Published exact
  accepted merge with local/tracking/live main equality verified; no new
  executable mask/plan/recovery or device support.
  [Contract](contracts/offline-download-system1-image.md) ·
  [Evidence](evidence/offline-system1-image-2026-10-16.json).

- 2026-10-16 — P4e bounded research completed: original profile/procedure pages and
  three retained application sources in one package rechecked. Selected master
  table resource locations are present, but sparse product object numbers are
  not an explicitly complete descriptor array. No padding, compaction or
  automatic source/profile mapping inferred; no production change or runtime
  acceptance. Network-isolated bounded discovery selected seven source witnesses
  across System1/System300/coupler/RF, not a complete compatibility census;105
  original archive digests verified unchanged. [Integration gaps](research/2026-10-16-relative-image-integration-gaps.md).

- 2026-10-16 — P4d complete explicit System-B object-table construction published as
  `894f14e0`: every descriptor supplied, caller capacity distinct from emitted
  count, no synthetic inactive slots. Six literal/boundary tests pass after
  missing-API RED; capacity-padding mutant rejected, exact source restored.
  Existing ten rewrite tests and owning all-target Clippy pass. All eighteen
  branch stages pass on the unchanged retry candidate: Rust4678/0/208ignored
  in343targets, Vitest3626/263files;2922source and external/Git commitments
  match. First procfs-snapshot test failure retained; no test weakening. All
  nineteen actual merged stages pass with the same Rust/Web totals and isolated
  corpus152/152 in33targets; exact local/tracking/live main equality verified.
  No product image/plan/recovery
  or new hardware support. [Contract](contracts/offline-systemb-object-table.md#p4d-explicit-complete-construction-16-october-2026) ·
  [Evidence](evidence/offline-systemb-complete-table-2026-10-16.json).

- 2026-10-16 — P4c explicit System-B address/association codec integrated and published as `c74038cd`:
  word counts, no individual slot, one-based wide indices, explicit formats0/1
  and preserved first-match sender. Twelve literal/boundary tests pass after
  missing-module RED; a realistic sender-order mutant fails the intended test,
  source restored. Core844/0 in16targets, owning all-target Clippy and format
  pass. Original-body inspection also corrects57B0-versus07B0merge attribution;
  no generated product image, new mask admission or recovery claim. All18frozen
  branch stages pass: Rust4672/0/208ignored in342targets, Vitest3626/263files,
  both explicit source regressions and workspace checks. All2919source identities,
  external oracle/package commitments and Git fingerprints match. All19actual
  merged stages pass, including isolated corpus152/152 in33targets; exact
  local/tracking/live main equality verified. All10preflights and separate self-review
  pass with no critical/important/security finding. [Contract](contracts/offline-systemb-link-tables.md) ·
  [Evidence](evidence/offline-systemb-link-tables-2026-10-16.json).

- 2026-10-16 — P4b relative source view integrated and published as `4bfc3780`: distinct relative
  metadata/Data/Mask, shared source blob decoder, duplicate-ID refusal and an
  explicit absolute-image guard. Missing-field compile RED and mixed-source
  omission runtime RED observed before their fixes; ten reader tests and the
  image guard pass. Three real sources match an independent XML/base64 oracle;
  original packages and product DB remain unchanged. All twelve corrected
  preflights pass: ten reader tests, 444 ProductDB unit tests, the opt-in
  three-source corpus regression, all-target Clippy and repository checks.
  All 18 frozen branch stages pass: Rust4660/0/208ignored, Vitest3626/263files,
  both explicit source regressions and workspace checks. Complete2914path,
  external oracle/package and Git commitments match. All19actual merged stages
  also pass, including isolated corpus152/152 in33targets. Published merge
  local/tracking/live remote equality verified; no relative image, wire plan
  or new admission.
  [Contract](contracts/offline-relative-source-segments.md) ·
  [Evidence](evidence/offline-relative-source-segments-2026-10-16.json).

- 2026-10-16 — P4a System-B prerequisite integrated and published as `adb52990`:
  pure Type-7 /
  Format-1 group-object rewriting, one-based u16 indices, explicit initial read,
  extended value types and preserved source/tail. Ten focused tests pass after
  an observed missing-module RED; an omitted initial-read mutant fails the named
  bit assertion and source restoration is verified. Initial owning-crate Clippy
  rejected constant-sized chunk syntax; corrected Clippy and all 17 frozen
  branch stages pass: Rust 4649/0 (207 ignored), Vitest 3626/0, explicit Release
  source regression, workspace build and checks. All 18 actual merged stages
  also pass, including isolated corpus152/152 in33targets; all2908source identities
  remain frozen and local/tracking/live remote main equality was verified.
  The actual existing resolver baseline on three
  selected real sources returned three partial/non-executable sequences, not
  product support. No image/plan/recovery/public-route promotion.
  [Contract](contracts/offline-systemb-object-table.md) ·
  [Research](research/2026-10-16-systemb-offline-prerequisites.md) ·
  [Evidence](evidence/offline-systemb-object-table-2026-10-16.json).

- 2026-10-16 — P3b **research checkpoint, no production-code change**:
  original Resources§4.2.7 p31 specifies a four-octet table-reference pointer;
  retained master resource Length2 is not treated as a proven wire-width rule.
  A bounded network-isolated two-package source probe observed49procedures and
  264absolute-segment flags;30oversized members remain unexamined. The44machine5
  application witnesses match the prior study exactly. Neither flag polarity,
  parameter byte order nor complete recovery was inferred from the census.
  This System2 translation slice remains evidence-blocked; independent family
  work continues. P3a publication is unchanged.
  [Findings](research/2026-10-16-system2-source-semantics.md) ·
  [Aggregate receipt](evidence/system2-source-semantics-2026-10-16.json).

- 2026-10-16 — Offline expansion P3a **integrated and published** as `7632fb1a`:
  immutable normalized0025 property plans validate source positions, explicit
  object indices, identity ordering, load lifecycle, absolute allocation bounds
  and non-overlapping writes. The concrete-simulator-only executor reuses
  ManagementSession, delays Verify Mode until identity checks, preserves zero-wire
  metadata and reports source-local failures without automatic rollback.
  Focused Core8/8, Net7/7, one hardware-transport compile-refusal example and
  owning-crate all-target Clippy pass. Original relative simulator allocation
  baseline and setup/target guards produced named REDs before their fixes.
  Fifteen-stage branch and sixteen-stage actual merged acceptance pass:
  Rust4639/0/207ignored, Vitest3626/263files, workspace Clippy/build, actual
  frontend build, documentation/tooling/five repository checks and the explicit
  Release real-source refusal regression. Merged acceptance adds isolated
  corpus152/152 in33targets;2899source paths remain unchanged. Local HEAD,
  fetched main and live remote main read back equal to `7632fb1a`.
  The selected dirty root checkout was deliberately not synchronized.
  ProductDB translation, restart, real-source16-bit encoding, complete durable
  recovery and any new public/hardware support remain open.
  [Receipt](evidence/offline-property-plan-simulation-2026-10-16.json) ·
  [Contract](contracts/offline-property-plan-simulation.md).

- 2026-10-16 — Offline expansion P2 **integrated and published** as `79156a5d`:
  legacy ASAP-indexed sending/FE-unused associations and an explicit System2
  source-image library path sharing the production product/Dynamic evaluator.
  Original-PDF evidence distinguishes table grammar and intended template
  modification. Synthetic source assembly covers parameters, objects and links;
  new overflow/default/overlap tests retain honest RED/GREEN. Real Siemens
  whole-image assembly is still refused at a specific uncorroborated16-bit
  source/default field, not claimed compatible. P3 planning/simulation/recovery
  and field validation remain open. Fourteen-stage branch acceptance and actual
  fifteen-stage merged acceptance pass: Rust4623/0/207ignored, Vitest3626/263files,
  workspace Clippy/build, frontend build, tooling/five repository checks and the
  explicit Release real-source refusal regression. Merged acceptance also runs
  isolated corpus152/152 in33targets;2892source paths remain frozen. Local HEAD,
  fetched main and live remote main were read back equal to `79156a5d`.
  The selected dirty root checkout was deliberately not synchronized. [Receipt](evidence/offline-download-system2-image-2026-10-16.json) ·
  [Contract](contracts/offline-download-system2-image.md).

- 2026-10-15 — Offline download P1 **integrated and published** as `ad5f0e49`:
  source-bound System2 tool metadata with complete source order, explicit legacy
  Type1/Type2 descriptor codec and ten-byte absolute property-load encoders.
  Direct original-PDF review pins distinct layouts and opposite checksum-bit
  polarity; named RED/GREEN controls and a restored field-swap mutant prove the
  byte assertions. Actual frozen eleven-stage gate passes: Rust4612/0/207ignored,
  Vitest3626/263files, workspace Clippy/build, tooling and five repository checks;
  explicit Release System2 fixture1/1 passes. Actual merged twelve-stage gate
  repeats the workspace/build/Vitest/repository checks and adds the isolated
  OriginalData corpus152/152 in33targets, with2884path start/end/current equality.
  Exact local/tracking/live main publication read back. Initial missing-dist setup refusal
  retained separately. No executable new family, recovery, hardware or field
  support claim. [Contract](contracts/offline-download-codecs.md) ·
  [Receipt](evidence/offline-download-codecs-2026-10-15.json).

- 2026-10-15 — Original-layout phosphor CRT website **published and verified**.
  Owner-approved revision60d8eb7a integrated asd90b3d4b; Pages38789580428 build/deploy
  success. Merged23/23 source-frozen stages: website28, Story76, tooling163(1skip),
  Chromium394preview+416public-variant checks. Live root+36content files+manifest
  equal accepted bytes, canonical EN/DE/CSS/headline readback equal;124deployed-host
  Chromium checks pass. Story/application/media/legal content unchanged; no product
  release/hardware or selected-root synchronization. In-session self-review only.
  [Publication receipt](evidence/phosphor-crt-publication-2026-10-15.json).

- 2026-10-15 — Owner-revised private website: added installation diagram removed,
  original screenshot-first layout restored, green Story-style headings/CRT palette
  and independently random horizontal/vertical rolling glyphs. Website28/Story76,
  tooling163 (1 existing skip),23 frozen-source stages; actual Chromium394 preview
  and416 public-variant checks with every recipe origin/mode-bound. Earlier
  mixed-origin gate and prior diagram design remain historical/superseded, not
  silently relabelled. Story/application/media/legal text unchanged; no publication.
  [Contract](WEBSITE.md#story-style-phosphor-crt--private-revision-15-october-2026) ·
  [Evidence](evidence/phosphor-crt-revision-2026-10-15.json).

- 2026-10-15 — Phosphor Atlas marketing redesign accepted as a **private preview**,
  not a Pages publication: whole green-dark canvas, fictional TP-line/logical-link
  diagram before real Alpha12 screenshots, keyboard/touch explanations, vertical
  mobile layout and unified live motion cancellation. Existing Story, application,
  media and legal wording unchanged. Website27, Story76, tooling163 (1 existing
  skip), Chromium418 preview +418 public-variant checks, five repository gates and
  deterministic builds pass. Acceptance is a frozen-source execution chain retaining
  the original dependency/capture refusals, not an uninterrupted green run.
  [Contract](WEBSITE.md#phosphor-atlas--private-implementation-15-october-2026) ·
  [Evidence](evidence/phosphor-atlas-2026-10-15.json). Owner publication approval pending.

- 2026-10-15 — **Alpha 12 published and verified**: immutable annotated
  `v0.1.0-alpha.12` → `16aefbe6`, owner-approved public prerelease with hand-written
  notes. Tag workflows38773736127/38773736107 pass. Downloaded AppImage/MCP and
  SHA256SUMS match GitHub digests; MCP/server versions resolve the exact tag.
  AppImage extracts without bundled libwayland-client and its isolated Broadway
  startup serves the actual embedded frontend/API. Registry version/latest share
  one native amd64/arm64 index; anonymous version pull and network-none HTTPS,
  auth401/200, save/reopen plus authentic public schema10→16 migration pass.
  Original fixture and user's container identity/start remain unchanged. Corrected
  Pages38774710136 passes; root and36 public content files match accepted bytes.
  Initial shallow-checkout Pages refusal retained; narrow checkout-history fix
  has website25/Story76 and byte-identical generated output. No FUSE/native visual,
  EGL/Orca, new hardware or broad ETS-compatibility claim; dirty root unsynchronized.
  [Publication receipt](evidence/release-alpha12-2026-10-15.json).

- 2026-10-15 — Owner approved full Alpha 12 publication after complete source
  acceptance. Annotated `v0.1.0-alpha.12` points exactly to `16aefbe6`; the public
  GitHub prerelease and hand-written scope were read back. Tag-triggered build/
  artifact/registry jobs and live-site verification are in progress. Release/docs
  integration preserves every gated application, test, workflow, Story and website
  input; this entry does not claim published artifact verification yet.

- 2026-10-15 — **Alpha 12 source accepted, not published**, frozen candidate
  `16aefbe6`: complete77/77 source-bound chain (66 original successful steps,
  11 actual continuation steps after private Chromium starter correction).
  Vitest3626/263files; fixture Chromium574 plus additional configurations;
  Rust4597/0 with207 ignored; native evidence12 and naming/Functions/recipe4each;
  bindings28; tooling163(one skip), Story76, website24 and actual site492;
  OriginalData152/152 in33targets and product16/16 in6targets. Full2,860path
  mode/content/HEAD freeze verified. Ten app frames and nineteen website frames
  plus thirty page strips self-reviewed, not independent approval. Exact-SHA
  AppImage/MCP and amd64/arm64 Docker dry runs pass with publishing skipped;
  downloaded build artifacts hashed. Owner final tag/publication Go is pending.
  Original failed records remain; no tag, Pages or container cutover.
  [Acceptance dossier](releases/alpha12-acceptance.md).

- 2026-10-15 — Alpha 12 candidate `ffc95e0a` **not accepted**: full gate
  passed58 stages including all three native edit proofs, then a private
  bindings comparator failed to load the locked TypeScript7 API. Corrected
  comparator passes all28 generated paths with literal-preserving token equality
  and seven rejection/format controls. A same-source continuation then found
  the screenshot config missing its explicit central browser-temp guard; the
  original failure remains, not a completed release gate. Added only that import;
  guard5, tooling163 (one skip), native-media types and actual config listing pass.
  Actual guarded-config real-server capture also passes one test/ten frames in
  loopback-only isolation. New full candidate acceptance remains pending.
  No production/model/schema/compatibility claim or publication changed.
  [Tooling refusal evidence](evidence/alpha12-tooling-refusals-2026-10-15.json).

- 2026-10-15 — Alpha 12 full gate **refused**, not release-ready: candidate
  `02d823c4` passed frontend3626, ordinary Rust4597 (207 ignored), all declared
  fixture browsers and native-evidence, then stopped in the native naming proof.
  Its test sought the welcome-only open button while a project was already open.
  Original failure and unchanged source proof are retained; remaining checks did
  not run and no finalizer override is admissible. The three native edit specs
  now use the real, dialog-scoped Commands route: targeted EN/DE wide/narrow
  native naming, Functions and naming-recipe proofs pass12/12, with TypeScript
  and unchanged source/demo checks. This is test-route evidence, not acceptance
  of a new release candidate; a fresh full74-stage candidate gate is pending.
  Existing exact-candidate AppImage/MCP and amd64/arm64 Docker dry runs succeeded
  with publication jobs skipped; they do not certify a future successor SHA.
  [Refusal evidence](evidence/alpha12-gate-refusal-2026-10-15.json).

- 2026-10-15 — Alpha 12 media/docs preparation: ten actual integrated-application
  PNGs (real server, fictional house/native-created device, loopback-only network)
  and seven decoded-pixel-equal lossless website WebPs. Website24, current docs,
  capture TypeScript and public website build pass; actual production-variant
  Chromium492 checks pass across EN/DE,320–1440px,JS/no-JS,motion/navigation/media.
  All ten application frames and nineteen website frames (thirty long-page strips)
  inspected. README/manual/feature/model/import/compatibility upgrade routes now
  agree on native16/product23 and pre-upgrade independent backups. P4 delivered
  source and its exact original acceptance are retained; approved Story unchanged.
  Full exact-candidate gate, no-publish packaging builds and final tag/publication
  approval remain separate and pending. No hardware operation or release claimed.
  [Preparation evidence](evidence/alpha12-media-2026-10-15.json).

- 2026-10-15 — **P4 delivered on main `17cfd5b7`**, normal push and exact
  local/tracking/live readback verified. Both packages, feature519eff4b and
  separate corpus-fixa6cdfd9d are included. Runtime tested mergefacdd62e has
  full51/51, Vitest3626, Chromium574, Rust4597, repeated87 and corpus152/152+
  product16/16; subsequent nine-stage documentation delta passes with complete
  non-documentation path/mode/blob equality. Final EN/DE1440/400 screenshots
  inspected; unsupported parameters remain Unknown alongside raw ranges.
  Alpha12 prerequisite is delivered; this session pauses after owned retirement.
  Shared root intentionally remains untouched/unsynchronized; no hardware,
  new product release, deployment or independent-review claim.
  [Exact acceptance/publication](evidence/p4-device-comparison-parameters-2026-10-15.json).

- 2026-10-15 — **P4 both packages accepted; final combined source `facdd62e`**:
  single/current-filter batch comparisons, session tags/P3 invalidation,
  verified application identity, decoded address/association tables and named
  parameter values from actual image bit attribution; unknowns and all raw bytes
  stay explicit. Full51/51: Vitest3626, Chromium574, Rust4597 and repeated87 pass,
  original15 configurations plus upstream support pass. Isolated corpus152/152
  and product16/16 pass. Seven parameter + seven corpus semantic mutants caught;
  self-review (not independent approval) and EN/DE1440/400 production images pass.
  Inherited prerequisite fix `a6cdfd9d` changes tests/docs only. No production
  admission/schema/limit or device-support extension, hardware access or release.
  Alpha12 journal-only upstream preserves all consumed source bytes; runtime
  acceptance remains bound to `facdd62e`, documentation delta checked separately.
  [Acceptance/publication](evidence/p4-device-comparison-parameters-2026-10-15.json).

- 2026-10-15 — **Corpus-prerequisite correction a6cdfd9d, integrated candidate / unpublished**:
  two failures reproduced on unchanged pre-P4 main. Matrix pins lag ADR-0126's
  v23 modelling, not an importer regression: independent XML counts predict
  exactly465/463/463/157 retired unknown rows, all41 other table counts unchanged.
  The nesting census explicitly accounts for standard size refusals, not a
  larger admission profile; unexpected errors remain red and refusal row changes
  are forbidden. Targeted product16/16, synthetic classifier cases and Clippy
  pass; seven semantic mutants fail as intended and restored-source ordinary
  product887/0 tests pass. Integrated candidate preserves exact previously
  accepted P4 production/frontend bytes; final combined full/corpus gates remain
  pending, rather than borrowing an earlier source receipt.
  [Facts and limits](IMPORT_EXPORT.md#product-corpus-prerequisites--15-october-2026).

- 2026-10-15 — P4 package 2 **implemented; publication blocked by additional corpus checks**: actual
  active parameter writes retain name/reference/segment/offset/bit/width in
  transient DownloadImage metadata. Named project/device values invert existing
  numeric and text writers, checking exact coverage, matching supported masks
  and final-plan equality. Unsupported/negative patterns stay unknown with
  EN/DE reasons and every raw range visible. No schema/device support change.
  Named RED/GREEN, seven restored-source semantic mutants, separate in-session
  review and production EN/DE1440/400 browser cases pass. Full branch49/49:
  Vitest3611, Chromium570, repeated87 and Rust4570 pass. Source-content manifests
  and template input hashes are exact; the outer add-N index-only mismatch is
  reconciled with negative controls, original receipt retained. Actual merge
  `788974e7` passes full51/51 (Vitest3626, Chromium574, Rust4592) and isolated
  OriginalData152/152. Explicit product opt-ins measure14/16: projected matrix
  commitment mismatch and nested-module installer SizeLimit block publication.
  Both test sources and installer are exact pre-P4 upstream bytes. Unmodified
  `bee4f016` reproduces both named failures with identical commitment values
  and SizeLimit member: inherited, not introduced by P4. The additional product
  corpus is still red. No limits/pins changed; no push claimed.
  [Contract](contracts/device-comparison.md), [ADR-0130](adr/0130-session-device-comparison.md).

- 2026-10-15 — Exact-approved story `2026-10-15.4` publication variant accepted
  on integration `bb2961aa`: story76, website24, tooling163 (one inherited skip),
  Chromium46/46 and Firefox46/46; all44 actual public-build frames inspected.
  Four visual contexts confirm all8 release nodes, explicit private/local KB and
  public-data-provision wording, no overflow or outgoing requests. Five explicit-
  root repository checks, reference validation, docs, matching approval and
  release38-file build pass. Complete story/site inputs remain unchanged.
  Published commit `022d85a0` has successful Pages run `38701307431`; live HTTP200
  HTML matches the accepted build byte for byte and carries the exact approved
  digest. No product release, bus operation or root-checkout synchronization.
  [Publication acceptance](evidence/story-publication-2026-10-15.json).

- 2026-10-15 — Story `2026-10-15.4` received explicit exact-digest owner
  publication approval. Approval, website pin and latest-only retirement are
  applied together; prior editions remain in Git. Content digest
  `081d2c4c42b03a0b97ae803ac7f78375e4ca5593f3687453872200aab8398cf8`
  is unchanged from accepted preview. Release-check, integrated build and public
  deployment verification are separate steps; approval alone does not claim
  deployment. [Story](../story/README.md),
  [accepted content](evidence/story-update-2026-10-15-editorial.json).

- 2026-10-15 — Owner-reviewed story correction candidate `2026-10-15.4`:
  specification knowledge base explicitly private/local and not distributed;
  public product databases described only as provided for analysis. Every
  evidenced alpha release has a separate event/atlas node: historical withdrawn
  Alpha4 plus Alpha5–11, distinguishing original/republication dates and CEST.
  Pinned source stays `2d4ec5f9`; 74 events/93 relations/twelve chapters. Three
  editorial regressions proved RED→GREEN, full story76 and Git-reference
  validation pass. Actual frozen .4 Chromium46/46 and Firefox46/46 pass; all44
  main frames plus8 unoccluded-title supplements inspected across both engines
  at desktop/mobile widths, including all eight real release graph labels.
  Website24, five fresh explicit-root repository checks, docs and unchanged
  old-approved website38-file build pass. The new edition correctly refuses
  release-check without approval. [Current acceptance](evidence/story-update-2026-10-15-editorial.json).
  No publication approval, push, Pages cutover, new release or hardware operation.

- 2026-10-15 — Project-evolution story candidate `2026-10-15.2` prepared locally
  for review, not publication. Pinned source `2d4ec5f9`, 07:04:36 CEST; 68 events,
  87 relations and twelve chapters retain the Phosphor Atlas design. New strands
  cover public users, reversible engineering and evidence boundaries, with
  explicit source/release/hardware/design distinctions. Optional read-only
  `storytool validate --repository` checks Git ancestry, cutoff time and file
  references (six test-first regressions). Existing approved `2026-10-08.4` and
  the website pin remain unchanged. Story73, website24 and repository tooling148
  (one existing skip) pass; Chromium and Firefox each pass46/46 browser checks
  with frozen inputs. Four visual contexts/24 original screenshots inspected;
  mobile guided-graph labels remain dense, full text/atlas available. Five fresh
  root-bound repository checks and documentation pass. The unchanged approved
  website release build passes; the new unapproved edition correctly refuses
  release-check. [Aggregate acceptance](evidence/story-update-2026-10-15.json).
  Exact-edition owner publication approval remains pending; no push/deployment
  or new bus operation.
  [Preparation and boundaries](../story/README.md#update-prepared-on-15-october-2026).

- 2026-10-15 — P4 package 1 tables accepted in source after Alpha 11: confirmed
  single/filtered device queue, progress/draining cancellation and session tags;
  P3 content revisions, verified five-byte application identity, bounded 0701
  group-address/association decoding and visible raw read ranges. Nine named
  semantic mutants and restored-source postflight pass; separate self-review
  closes identity/count/freshness and actual-browser contrast/height findings.
  EN/DE production 1440/400 images were visually checked, including horizontal
  device-value access at 400 px. The frozen branch gate passes all 49 stages: Rust 4565 (206 opt-ins
  ignored), Vitest 3601/260 files, default Chromium566, fifteen production
  configs325 and repeated browser75; Python148/one inherited skip. All2802
  source files are unchanged. Fresh-main merge `a9f429a4` passes49/49 frozen stages: Rust4565,
  Vitest3609, default570, production329/15configs, repeats87; the network-
  isolated corpus passes151/151 in33targets. Merge images inspected; later docs/tooling delta passes nine closure checks with runtime unchanged. Named parameter attribution remains package 2; no bus access.
  [Goal](goals/ui-feedback-p1-p4.md), [contract](contracts/device-comparison.md),
  [ADR-0130](adr/0130-session-device-comparison.md).

- 2026-10-15 — Community Hall of Fame: cumulative report medals every ten
  issues and completed-report medals every five, both through 100; earned
  names only in generated documents/data and completion notices. Public goals
  give hints rather than an unearned catalogue. Table order is Place, Bug hunter,
  Rank & title, Medal, XP, Issues, Replies, Resolved. External AI/Bot accounts
  can earn points; the scoreboard bot stays excluded. Explicit notification
  authors are separate from scoring opt-outs. XP/ranks and retained completion
  awards are unchanged; historical medal backfill posts no repeat comments.
  Test-first and separate in-session self-review: 47 community tests pass;
  full tools suite 163 tests, one inherited skip, passes after installing the
  lockfile-pinned frontend dependencies required by a tools test. All five
  fresh-built repository checks and documentation validation pass. Real public
  GitHub generation verifies consistent README/page/JSON, unchanged award ledger
  and unchanged XP. Published source `fe1f4101`; all eleven files pass exact
  GitHub content readback. Manual community workflow `38694177327` succeeds on
  that source, including its 47-test verification and real GitHub reconciliation.
  No UI/domain/storage/bus change or release/deployment.
  [Contract](contracts/community-recognition.md),
  [board and mystery goals](community/HALL_OF_FAME.md#the-next-mystery).

- 2026-10-15 — Universal device-support evidence intake, source after Alpha11.
  Offline reports work without a project/catalogue/plan/gateway; optional fixed
  read-only diagnostics, context rights and ZIP disclosure have separate consents.
  Unknown/withheld/unsupported evidence is explicit; no download unlock, registry
  promotion or real bus operation. Frozen branch acceptance:10/10 stages,
  maintained web61/61, Rust4575/0 (205 opt-ins ignored), dedicated production
  browser12/12 (EN/DE1440/400, three repeats); all8 setup/preview frames inspected.
  Seven named guard mutations were behaviorally caught in an isolated source
  snapshot; restored tests pass and canonical bytes remained unchanged.
  Separate in-session self-review, not independent approval. Fresh-main merge
  `3444186c` passes11/11 stages with frozen inputs: Rust4587/0 (206 opt-ins ignored),
  maintained web62/62, Vitest3624, default browser574, dedicated dialog12/12 and
  private OriginalData151/151 in33 targets, network-isolated. Eight merged images
  inspected. Later story/docs-only integration preserves all application path
  sets, modes and blobs; runtime evidence remains bound to its tested revision.
  Published on main `9b8d176a` with exact local/tracking/live/source readback;
  fifteen frozen post-merge documentation/story/tooling closure stages pass.
  No binary release, deployment or selected-root synchronization.
  Publication/readback is recorded separately in the aggregate evidence.
  [Contract](contracts/universal-device-support.md),
  [ADR0131](adr/0131-universal-device-support.md),
  [aggregate evidence](evidence/universal-support-2026-10-15.json),
  [operating guide](manual/user-guide/08-reports-and-diff.md#universal-device-support-reporting-source-after-alpha-11).

- 2026-10-14 — Issue #23: diagnostic sidebar entries select instead of toggle.
  Re-clicking Bus monitor or Log keeps its panel and local state; choose another
  project view explicitly to leave. Settings/companion actions are unchanged.
  Test-first: eight App regressions reproduce the old active-marker loss;
  corrected App suite 172/172, dedicated spec TypeScript/build zero, production
  browser 4/4 (EN/DE × 1440/400). Frozen branch web/docs gate: 46 stages zero,
  Vitest 3567/0 (256 files), Chromium 558/0, fifteen production configurations
  zero, devices-navigation repeats 51/0; Python 123 (one skip), all compiler,
  documentation and five repository checks pass. First merged result `b9a2decc`
  also passed all 47 stages with the same test counts. Maintained-runner merge
  `2e803fef` passed 60 stages. Final integration `bb529169` with published P3:
  all 60 maintained web/docs stages zero/proof-valid, inputs/build/tools frozen;
  Vitest 3588/0 (259 files), default browser 564/0, production browser 323/0 in
  fifteen configurations, twelve issue-specific production repeats and dedicated
  spec types pass. All eight EN/DE 1440/400 production images inspected. The
  inherited achievement toast can temporarily overlap lower narrow-screen content;
  no horizontal overflow observed. Self-review, not independent approval.
  Published on main `8307ff6d`; issue #23 closed after exact remote/source
  readback. Eight documentation-only closure checks pass with runtime inputs
  unchanged. [Aggregate acceptance](evidence/issue23-navigation-2026-10-14.json).
  [Behaviour and boundaries](contracts/diagnostic-sidebar-selection.md),
  [operating guide](manual/user-guide/01-user-interface.md#the-navigation-pane).
  No API/domain/storage/schema/protocol change, hardware, release or deployment.

- 2026-10-15 — Owner UI feedback P3 delivered in source after Alpha 11 ([goal](goals/ui-feedback-p1-p4.md),
  [ADR-0129](adr/0129-per-part-pending-programming.md)): existing loaded flags
  follow edits in the same undo step and server full/partial/address write
  outcomes outside undo history. The Devices Programming column shows pending
  Adr/App/Par/GA chips and a Pending only filter; explorer rows have a status
  dot, Properties has undoable logged manual marks. DPT/name edits do not count.
  Interrupted writes mark attempted parts, including uncertain writes; a
  changed project/device cannot acquire another run's clean claim. Native
  save/reopen/recovered undo uses the existing fields without a migration.
  Separate self-review, nine named mutant failures, native restart/recovered
  undo and all sixteen loaded-flag patterns are verified; the private corpus
  passes 149/149 in 32 targets, including all 29 download HTTP cases.
  Production attribution keeps the added chip work bounded: seventeen cached
  presentations, mounted only on the first visible table visit and retained
  thereafter. The controlled synthetic 5000-device all-pending comparison
  reduced mean MainThread RunTask from 23.2 to 16.5 seconds without delaying
  table opening; this is not a universal or whole-gate speedup claim.
  The frozen branch gate passes all 48 stages: Rust 4545, Vitest 3580,
  default browser 560, production browser 319 across fifteen configs and
  repeated browser 57; all stage exits zero and all 2770 source files unchanged.
  Native Rust opt-ins remain reported separately (204 ignored); the earlier
  OriginalData sweep passes 149/149. Python runs 123 tests with one inherited skip.
  Production EN/DE screenshots at 1440/400 px are visually inspected. Fresh-main
  merge `f35d9bda` passes all 52 stages on an unchanged 2787-file manifest:
  Rust 4553/0 (205 opt-ins ignored), Vitest 3580/259 files, default browser560,
  fifteen production configs319, repeats57, Python148/one inherited skip and
  OriginalData-marked ignored sweep150/150 in33 targets, network-isolated.
  All compiler/architecture/header/anchor/ledger/corpus/docs/diff checks pass.
  Recovered snapshot undo deliberately invalidates old clean claims
  conservatively; additional pending parts do not prove actual device differences.
  Runtime acceptance, not a hardware test, release or native-WebKit certification.
  [Evidence and boundaries](evidence/p3-pending-programming-2026-10-15.json).
  [Operating guide](manual/user-guide/05-devices-and-products.md#programming-marks-p3-source-build-after-alpha-11).

- 2026-10-14 — Measured web-test tooling: a maintained Linux runner
  replaces ad-hoc web wrappers, retains all default/production cases and derived
  TypeScript configs, removes three exact compiler duplicates and admits only
  unchanged reviewed functional specs to bounded parallel execution. Six equal-case
  1/2/4/4/2/1 runs pass (276 per execution): mean 331.18/195.66/165.15 s, about
  2× on this host for the functional lane only. Default browser union is 554 cases
  (276 functional / 278 exclusive); no assertions, retries, protocol or application
  behavior changed. Twenty runner regressions plus five central-temp regressions
  pass. The shared guard covers all thirty Playwright configs, including direct
  invocations; a real Chromium probe repairs a 245-byte requested TMPDIR to a
  59-byte owned path and verifies cleanup without deleting the caller's directory.
  Three exact typecheck duplicates are removed; one new Node-only tooling config
  is added (19 previous calls → 17 now, all old scopes retained). The earlier
  branch attempt was intentionally stopped on the owner's scope expansion and
  remains unaccepted. Complete expanded-source branch web acceptance passed:
  60 zero/proof-valid stages, inputs/build/tool versions frozen, Vitest 3559/0,
  default browser 554/0, 15 serial production configs 313/0, Python 148 run
  (one existing skip), all typechecks/docs/xtask checks zero. Merged-source
  `5db49560` also passed sixty stages with the same execution counts. Its exclusive
  stage overlapped a failed archive attempt; a separate unchanged 278-case
  exclusive followup passed under leases, with source/build/tools frozen and no
  retries/skips. No whole-gate speedup, Rust/corpus or release certification.
  Publication follows documentation-only closure checks.
  [Contract](contracts/web-test-gate.md),
  [evidence](evidence/web-test-optimization-2026-10-14.json).

- 2026-10-14 — Issue #22 offline prerequisite: exact-shape `LdCtrlTaskCtrl2`
  source parsing and the pure ten-octet property-record encoder, strict unknown
  preservation and unchanged download refusal. This is not System-2 device
  support; image/procedure/recovery contracts remain open. [Contract](contracts/system2-download-prerequisites.md).
  Gate results are recorded in the delivery handover, not inferred from this entry.

- 2026-10-14 — Owner UI feedback P2 ([goal](goals/ui-feedback-p1-p4.md)): the
  project explorer shows what opens. A chevron turns a quarter when a branch
  opens, a closed branch shows how many entries it holds, and Expand all /
  Collapse all sit above the tree (Collapse all keeps installations open).
  Thin structure lines connect every node to its children; they grow below a
  branch the user opens, and about every twelve seconds a pulse runs from the
  root through the rows on screen (`treeLines.ts`; nothing pulses in a hidden
  tab). New motion category "Structure lines" (`motionLines`): inherit grows
  and pulses, subtle only grows, off keeps the lines still; global Off and OS
  reduced motion win in CSS. Measured on a synthetic 2,394-row tree (production build,
  noisy host at load 8): a pulse marked 4 on-screen rows and cost about what
  lines off cost (2 versus 3 long tasks, style 12 ms); the first draft that
  pulsed every row cost 10 long tasks and 759 ms of style work. Expand all
  costs the same with lines on or off (row mounting dominates).
  Tests: `ProjectExplorer.test.tsx`, `treeLines.test.tsx`,
  `motionCategories.test.ts`, `e2e/explorer-structure.e2e.ts`.

- 2026-10-14 — Issue #19: About links the GitHub Hall of Fame document instead
  of bundling its JSON. Snapshot parsing, unused strings/list styles and Vitest's
  external-directory exception removed; public generator and awards unchanged.
  Original Docker frontend failure reproduced; fixed frontend/full precommit
  image and network-none HTTP/asset smoke pass. Self-reviewed branch acceptance:
  31 stages zero/frozen, Vitest 3500/0 in 251 files, Chromium 531/0,
  production dialogs/link 18/0, repeat link checks 12/0, tooling 122 (one skip),
  build/all TypeScript configurations/docs/five xtask checks pass.
  EN/DE production screenshots at 1440/400 inspected. Merged result `655f056e`
  (on `871cc285`, with P1, #20, #21): 31 stages zero/frozen, Vitest 3547/0 in
  255 files, Chromium 546/0, production 18/0, repeats 12/0, tooling 123 (one
  skip); Docker image, network-none link smoke and `smoke-test.sh` pass. Details
  and two superseded merge attempts in the [receipt](evidence/issue19-docker-hall-link-2026-10-14.json).
  No offline board, new package dependency, Rust/core/schema change, hardware,
  release or deployment. [Contract](contracts/community-recognition.md#about-dialog-issue-19-owner-amendment-14-october-2026).

- 2026-10-14 — Issue #21: a `.knxdb` carries its application programs. Store
  schema 16 adds `manufacturer_file` (each manifest entry's bytes, zlib,
  SHA-256 keyed and re-checked on every read, outside the snapshot and the
  editor history); Save, Save As, the browser export and `knx import --store`
  fill it, and opening reinstalls whatever the product database lacks. A file
  saved before v16 names its missing files in the parameter panel and offers
  **Restore from the original .knxproj…** (hash-matched, outer members only so
  no password is needed, project untouched;
  `POST /api/project/manufacturer-files/restore`). The Docker image keeps the
  product database on `/data` (`XDG_DATA_HOME=/data/.knxbench-share`).
  [ADR-0128](adr/0128-native-project-files-carry-their-manufacturer-files.md),
  [§183](KNOWN_LIMITATIONS.md#183-a-knxdb-saved-before-store-schema-v16-does-not-carry-its-application-programs).
  Tests first (server RED before the store module): server 11, store units incl.
  v15→v16, CLI 1, web 2 files, Docker default 1 (RED without the line).
  End-to-end probe (not committed): two real ZipCrypto exports restored without
  password (4/4 and 2/2 devices) and the largest private project (76/76 devices,
  248 files, 18 MiB carried). Branch gate on `ed8edea9`: Rust 4487/0 (200
  ignored), Vitest 3546/0. Merged gate on `8aab7c27` (with #20; published as `ceb54d2f` after a
  rebase over the docs-only `5ed264aa`, its Python, Hall-of-Fame and build checks
  rerun): fmt, Clippy,
  build, 16 typechecks 0, Rust 4487/0 (200 ignored, 324 suites), Vitest 3577/0
  (255 files), Python 123 (1 skip), five xtask checks and the documentation
  check 0, bindings unchanged apart from end-of-line spaces; private corpus
  145/145 in 32 targets.
  Docker, after #19 unblocked the image build: image from `626ee907`,
  `smoke-test.sh` with the ETS 6.3 reference project passed. Container
  scenario (scratch script, not committed): import 35/35 devices resolve their
  program, `products.sqlite` lands at `/data/.knxbench-share/knx/`; a new
  container on the same volume 35/35; a new container with the product
  database deleted as well 35/35 (the `.knxdb` reinstalled its files, the
  database was recreated on `/data`). No hardware, release or deployment.

- 2026-10-14 — Issue #20: every range heading in the group-address tree table
  gets a **+** that opens **New group address in this range**. The suggested
  address is the range's first gap, else the one after its highest, else its
  lowest free one; the whole installation counts, `0/0/0` (broadcast) is never
  offered, and a typed address works too. The server still refuses duplicates
  and out-of-range addresses; the dialog keeps its input on refusal and opens
  the range after creating. Client-only: no API, domain or schema change.
  Tests first (helper RED before the module): helper 23, workspace dialog 8,
  EN/DE browser spec at 1440/400 px in two themes 4. Branch gate on
  `9c871f06`: build and 16 typechecks 0, Vitest 3566/0 (253 files), five
  xtask checks and the documentation check 0. Chromium 540/546 in a full run
  shared with a Rust workspace gate; the six failures (two untouched specs,
  CO table and UX feedback) pass 39/39 rerun alone. No Rust change, hardware,
  release or deployment. [Manual](manual/user-guide/04-group-addresses.md#creating-an-address-right-in-its-range).

- 2026-10-14 — Owner UI feedback P1 ([goal](goals/ui-feedback-p1-p4.md)): toasts
  enter with their exit animation played backwards (same keyframes, length and
  curve; the stack clips the slide instead of flashing a scrollbar). Every web
  editor mount resumes the project the server still holds through the read-only
  `GET /api/project`, announced with a notice that names unsaved changes
  ([ADR-0127](adr/0127-web-workbench-resumes-the-open-project.md)). Save As onto
  an existing file asks first: `knx-app::save_as_with` and
  `POST /api/project/save-as` refuse it (`409 fileExists`, nothing written) until
  `overwrite: true`; the browser asks in a dialog, the desktop relies on its GTK
  dialog's own question (source-verified, Linux only). Browser fixtures now answer
  `GET /api/project` like the real server (400 until a project exists). Tests
  first: RED proofs for entry, overflow, resume (5 by name), resume language,
  overwrite route (2) and dialog (2). Self-review: notice language and a stale
  doc comment fixed test-first. Frozen branch gate on `29573d41` + candidate:
  build and 16 typechecks 0, Vitest 3535/0 (251 files), Chromium 542/0, 15 production
  configs 262/0, repeat ×3 75/0, Rust fmt/clippy 0, Rust 4467/0 (200 ignored),
  Python 122 (1 skip), five xtask checks 0, `inputs_frozen=1`. No hardware,
  corpus, release or deployment.

- 2026-10-14 — Active-task text gets a slow shimmer and a quiet pause:
  readable real text, existing active-state bindings, measured progress retained;
  Off/reduced motion/forced colours/print and unsupported-CSS plain-text fallback.
  Standard 2.5-second sweep + 1-second pause; Subtle 3.6-second cycle.
  Self-review: feature-query fallback and warning-ink specificity fixed test-first.
  Integrated runtime `53046e64`: Vitest 3530/0 (251 files), Chromium 531/0,
  production import/download 27/0, shimmer repeats 12/0; build/all typechecks,
  tooling (122 tests, one skip), docs and five xtask checks zero; inputs frozen.
  All 32 actual stages accepted. Private runner footer falsely rejected positive
  freeze metadata; corrected outside the source candidate, four regression checks
  and independent raw-report aggregation pass (original wrapper exit retained).
  An earlier Toast-hover failure also reproduces on unchanged base; no unrelated
  code/test change, skip, retry or weakened assertion. Final production EN/DE
  wide/narrow light/dark sweep/pause frames inspected. No release/deployment,
  hardware, Rust workspace or private-corpus run.
  [Contract](contracts/task-status-shimmer.md),
  [acceptance receipt](evidence/task-status-shimmer-2026-10-14.json).

- 2026-10-14 — Device-programming support guidance S1–S4: structured refusal
  reasons/actions, EN/DE help before any plan/report, explicit local-copy privacy
  and fixed unsent docs/issue/mail routes; supported untested admission and every
  server guard retained. Corrected branch gate: Rust 4411/0 (200 ignored),
  Vitest 3460/0 in 247 files, Chromium 513/0, production download specs 33/0,
  focused browser 8/0, private corpus 145/145, all typechecks/Clippy/docs/xtask
  zero; inputs frozen (historical branch acceptance).
  Final merged-runtime `6bf86976`: Rust 4442/0 (200 ignored), Vitest 3504/0
  in 249 files, Chromium 522/0, production specs 33/0, corpus 145/145;
  all 35 stages zero with frozen inputs. [Acceptance receipt](evidence/device-support-guidance-acceptance-2026-10-14.json).
  Closure changes documentation only; no release or deployment.
  [Contract](contracts/device-programming-support-guidance.md),
  [bounded offline audit](research/2026-10-14-device-support-guidance-offline-audit.md).
  All audited executable dependencies remain unassessed; no new encoder or
  hardware/mail-delivery evidence.

- 2026-10-13 — Device support guidance S1: additive categorized preparation
  refusals and pure next-action mapping; missing installed application data is
  distinct from configuration errors. Existing safety/untested admission unchanged.
  [Contract](contracts/device-programming-support-guidance.md). Pre-plan UI/reporting
  and audit are pending S2–S4; this entry makes no hardware verification claim.

- [2026-10-13 — Bug Hunters’ Hall of Fame: sixteen titles and a Golden Bus Probe](contracts/community-recognition.md).
  Public README/page/JSON, real GitHub counters, retained completion XP and
  automatic closure recognition. About display is handed to Goal B, not implemented
  here. [Issues #17/#18 triage](status/2026-10-13-issues-17-18-triage.md) accompanies
  the handoff; neither product fix is claimed.

- [Documentation history and research indexes reorganized](history/DOCUMENTATION_REORGANIZATION.md).
- [Alpha 11 publication](releases/alpha11.md).
- [Individual limitations dossiers](limitations/README.md).

## 2026-W42

<a id="2026-10-14--texts-and-status-claims-catch-up"></a>
- [2026-10-14 — Texts and status claims catch up](history/implementation/2026-W42.md#2026-10-14--texts-and-status-claims-catch-up)
<a id="2026-10-14--devices-made-in-knxbench-get-instructions"></a>
- [2026-10-14 — Devices made in KNXBench get instructions](history/implementation/2026-W42.md#2026-10-14--devices-made-in-knxbench-get-instructions)
<a id="2026-10-14--stored-bus-access-read-only-and-hidden-until-asked"></a>
- [2026-10-14 — Stored bus access: read-only, and hidden until asked](history/implementation/2026-W42.md#2026-10-14--stored-bus-access-read-only-and-hidden-until-asked)
<a id="2026-10-14--the-desktop-webview-asks-before-a-reload"></a>
- [2026-10-14 — The desktop webview asks before a reload](history/implementation/2026-W42.md#2026-10-14--the-desktop-webview-asks-before-a-reload)
<a id="2026-10-14--issue-17-page-1-learns-its-page-name"></a>
- [2026-10-14 — Issue #17: `{{0:Page 1}}` learns its page name](history/implementation/2026-W42.md#2026-10-14--issue-17-page-1-learns-its-page-name)
<a id="2026-10-14--about-thanks-the-community-from-a-bundled-board"></a>
- [2026-10-14 — About thanks the community, from a bundled board](history/implementation/2026-W42.md#2026-10-14--about-thanks-the-community-from-a-bundled-board)
<a id="2026-10-14--catalogue-backups-can-go-and-every-install-is-previewed"></a>
- [2026-10-14 — Catalogue backups can go, and every install is previewed](history/implementation/2026-W42.md#2026-10-14--catalogue-backups-can-go-and-every-install-is-previewed)
<a id="2026-10-13--issue-18-the-download-tab-names-the-device-at-once"></a>
- [2026-10-13 — Issue #18: the Download tab names the device at once](history/implementation/2026-W42.md#2026-10-13--issue-18-the-download-tab-names-the-device-at-once)
<a id="2026-10-13--issue-13-the-log-learns-to-tell-time"></a>
- [2026-10-13 — Issue #13: the Log learns to tell time](history/implementation/2026-W42.md#2026-10-13--issue-13-the-log-learns-to-tell-time)
<a id="2026-10-13--goal-b7-rename-several-addresses-literally"></a>
- [2026-10-13 — Goal B7: rename several addresses, literally](history/implementation/2026-W42.md#2026-10-13--goal-b7-rename-several-addresses-literally)
<a id="2026-10-13--kl-164-two-windows-two-steps"></a>
- [2026-10-13 — KL-164: two windows, two steps](history/implementation/2026-W42.md#2026-10-13--kl-164-two-windows-two-steps)
<a id="2026-10-13--issues-12-and-15-one-new-menu-per-page"></a>
- [2026-10-13 — Issues #12 and #15: one + New menu per page](history/implementation/2026-W42.md#2026-10-13--issues-12-and-15-one-new-menu-per-page)
<a id="2026-10-13--issue-11-the-address-tree-opens-compact-and-remembers"></a>
- [2026-10-13 — Issue #11: the address tree opens compact and remembers](history/implementation/2026-W42.md#2026-10-13--issue-11-the-address-tree-opens-compact-and-remembers)
<a id="2026-10-13--goal-b1b-functions-move-into-the-room"></a>
- [2026-10-13 — Goal B1b: Functions move into the room](history/implementation/2026-W42.md#2026-10-13--goal-b1b-functions-move-into-the-room)
<a id="2026-10-13--session-log-holds-16000-entries-and-drops-the-least-important-first"></a>
- [2026-10-13 — Session log holds 16000 entries and drops the least important first](history/implementation/2026-W42.md#2026-10-13--session-log-holds-16000-entries-and-drops-the-least-important-first)
<a id="2026-10-13--issues-14-and-16-buttons-stay-put-collapsed-channels-stay-shut"></a>
- [2026-10-13 — Issues #14 and #16: buttons stay put, collapsed channels stay shut](history/implementation/2026-W42.md#2026-10-13--issues-14-and-16-buttons-stay-put-collapsed-channels-stay-shut)
<a id="2026-10-13--goal-b1a-the-workbench-learns-to-say-changed-elsewhere"></a>
- [2026-10-13 — Goal B1a: the workbench learns to say "changed elsewhere"](history/implementation/2026-W42.md#2026-10-13--goal-b1a-the-workbench-learns-to-say-changed-elsewhere)
<a id="2026-10-13--goal-a6a7-no-new-evidence-and-gates-tied-to-their-plans"></a>
- [2026-10-13 — Goal A6/A7: no new evidence, and gates tied to their plans](history/implementation/2026-W42.md#2026-10-13--goal-a6a7-no-new-evidence-and-gates-tied-to-their-plans)
<a id="2026-10-13--goal-a5-legacy-strings-and-long-enums-mapped-like-ets4"></a>
- [2026-10-13 — Goal A5: legacy strings and long enums, mapped like ETS4](history/implementation/2026-W42.md#2026-10-13--goal-a5-legacy-strings-and-long-enums-mapped-like-ets4)
<a id="2026-10-13--goal-a4-project-functions-and-a-blocked-site-design"></a>
- [2026-10-13 — Goal A4: project Functions, and a site design that waits for evidence](history/implementation/2026-W42.md#2026-10-13--goal-a4-project-functions-and-a-blocked-site-design)
<a id="2026-10-13--goal-a3-a-stop-that-waits-for-the-device-it-is-writing"></a>
- [2026-10-13 — Goal A3: a stop that waits for the device it is writing](history/implementation/2026-W42.md#2026-10-13--goal-a3-a-stop-that-waits-for-the-device-it-is-writing)
<a id="2026-10-13--goal-a2-writes-that-know-which-project-they-mean"></a>
- [2026-10-13 — Goal A2: writes that know which project they mean](history/implementation/2026-W42.md#2026-10-13--goal-a2-writes-that-know-which-project-they-mean)
<a id="2026-10-13--issue-7-follow-up-desktop-downloads-ask-before-they-land"></a>
- [2026-10-13 — Issue #7 follow-up: desktop downloads ask before they land](history/implementation/2026-W42.md#2026-10-13--issue-7-follow-up-desktop-downloads-ask-before-they-land)
<a id="2026-10-13--issue-7-retained-files-stop-drowning-the-import-findings"></a>
- [2026-10-13 — Issue #7: retained files stop drowning the import findings](history/implementation/2026-W42.md#2026-10-13--issue-7-retained-files-stop-drowning-the-import-findings)
<a id="2026-10-13--issue-7-the-appimage-may-save-its-own-logs-again"></a>
- [2026-10-13 — Issue #7: the AppImage may save its own logs again](history/implementation/2026-W42.md#2026-10-13--issue-7-the-appimage-may-save-its-own-logs-again)
<a id="2026-10-13--goal-a1-old-projects-stay-old-until-you-change-them"></a>
- [2026-10-13 — Goal A1: old projects stay old until you change them](history/implementation/2026-W42.md#2026-10-13--goal-a1-old-projects-stay-old-until-you-change-them)
<a id="2026-10-13--alpha-11-published-and-downloaded-artifacts-verified"></a>
- [2026-10-13 — Alpha 11 published and downloaded artifacts verified](history/implementation/2026-W42.md#2026-10-13--alpha-11-published-and-downloaded-artifacts-verified)
<a id="2026-10-13--plug-in-data-pair-test-a-plug-in-edit-touches-only-the-plug-ins-bytes"></a>
- [2026-10-13 — Plug-in data pair test: a plug-in edit touches only the plug-in's bytes](history/implementation/2026-W42.md#2026-10-13--plug-in-data-pair-test-a-plug-in-edit-touches-only-the-plug-ins-bytes)
<a id="2026-10-13--adr-0121-part-b-the-download-tab-asks-testers-for-a-word-and-explains-how"></a>
- [2026-10-13 — ADR-0121 part B: the download tab asks testers for a word, and explains how](history/implementation/2026-W42.md#2026-10-13--adr-0121-part-b-the-download-tab-asks-testers-for-a-word-and-explains-how)
<a id="2026-10-13--issues-8-9-10-a-picker-that-listens-trees-that-indent-a-gateway-that-says-who-has-it"></a>
- [2026-10-13 — Issues #8, #9, #10: a picker that listens, trees that indent, a gateway that says who has it](history/implementation/2026-W42.md#2026-10-13--issues-8-9-10-a-picker-that-listens-trees-that-indent-a-gateway-that-says-who-has-it)
<a id="2026-10-13--individual-limitations-dossiers-and-a-short-status-reading-view"></a>
- [2026-10-13 — Individual limitations dossiers and a short status reading view](history/implementation/2026-W42.md#2026-10-13--individual-limitations-dossiers-and-a-short-status-reading-view)
<a id="2026-10-13--parked-ux-feedback-ported-steady-toasts-flow-images-motion-per-category"></a>
- [2026-10-13 — Parked UX feedback ported: steady toasts, flow images, motion per category](history/implementation/2026-W42.md#2026-10-13--parked-ux-feedback-ported-steady-toasts-flow-images-motion-per-category)
<a id="2026-10-13--tester-download-reports-part-a-of-the-verification-flow"></a>
- [2026-10-13 — Tester download reports (part A of the verification flow)](history/implementation/2026-W42.md#2026-10-13--tester-download-reports-part-a-of-the-verification-flow)
<a id="2026-10-13--kl-170-follow-up-named-base-image-byte-order-warning"></a>
- [2026-10-13 — KL-170 follow-up: named base-image byte-order warning](history/implementation/2026-W42.md#2026-10-13--kl-170-follow-up-named-base-image-byte-order-warning)
<a id="2026-10-13--notes-follow-ups-object-targets-unload-guard-translated-review"></a>
- [2026-10-13 — Notes follow-ups: object targets, unload guard, translated review](history/implementation/2026-W42.md#2026-10-13--notes-follow-ups-object-targets-unload-guard-translated-review)
<a id="2026-10-13--kl-170-byte-order-evidence-review-and-a-read-only-probe"></a>
- [2026-10-13 — KL-170: byte-order evidence review and a read-only probe](history/implementation/2026-W42.md#2026-10-13--kl-170-byte-order-evidence-review-and-a-read-only-probe)
<a id="2026-10-12--alpha-10-published-and-downloaded-artifacts-verified"></a>
- [2026-10-12 — Alpha 10 published and downloaded artifacts verified](history/implementation/2026-W42.md#2026-10-12--alpha-10-published-and-downloaded-artifacts-verified)
<a id="2026-10-12--device-editor-layout-test-follows-the-name-first-dpt-contract-alpha-10-prerequisite"></a>
- [2026-10-12 — Device-editor layout test follows the name-first DPT contract (Alpha 10 prerequisite)](history/implementation/2026-W42.md#2026-10-12--device-editor-layout-test-follows-the-name-first-dpt-contract-alpha-10-prerequisite)
<a id="2026-10-12--native-import-evidence-ported-onto-main"></a>
- [2026-10-12 — Native import evidence ported onto main](history/implementation/2026-W42.md#2026-10-12--native-import-evidence-ported-onto-main)
<a id="2026-10-12--ets-57-projects-import-real-protected-exports-decrypt-issue-7"></a>
- [2026-10-12 — ETS 5.7 projects import; real protected exports decrypt (issue #7)](history/implementation/2026-W42.md#2026-10-12--ets-57-projects-import-real-protected-exports-decrypt-issue-7)
<a id="2026-10-12--appimage-no-longer-ships-libwayland-client-issue-6"></a>
- [2026-10-12 — AppImage no longer ships libwayland-client (issue #6)](history/implementation/2026-W42.md#2026-10-12--appimage-no-longer-ships-libwayland-client-issue-6)
<a id="2026-10-12--catalogue-updates-with-a-preview-in-the-web"></a>
- [2026-10-12 — Catalogue updates with a preview in the web](history/implementation/2026-W42.md#2026-10-12--catalogue-updates-with-a-preview-in-the-web)
<a id="2026-10-12--checked-parameter-editors-in-the-device-editor"></a>
- [2026-10-12 — Checked parameter editors in the device editor](history/implementation/2026-W42.md#2026-10-12--checked-parameter-editors-in-the-device-editor)
<a id="2026-10-12--installation-instructions-in-the-device-editor-drafts-survive-save"></a>
- [2026-10-12 — Installation instructions in the device editor; drafts survive Save](history/implementation/2026-W42.md#2026-10-12--installation-instructions-in-the-device-editor-drafts-survive-save)
<a id="2026-10-12--group-address-members-gateway-choice-device--download-hand-off-step-guide"></a>
- [2026-10-12 — Group-address members, gateway choice, device → download hand-off, step guide](history/implementation/2026-W42.md#2026-10-12--group-address-members-gateway-choice-device--download-hand-off-step-guide)
<a id="2026-10-12--project-notes-in-the-workbench-session-1-host"></a>
- [2026-10-12 — Project notes in the workbench (Session 1 host)](history/implementation/2026-W42.md#2026-10-12--project-notes-in-the-workbench-session-1-host)
<a id="2026-10-12--previewed-recoverable-catalogue-updates-ap7"></a>
- [2026-10-12 — Previewed, recoverable catalogue updates (AP7)](history/implementation/2026-W42.md#2026-10-12--previewed-recoverable-catalogue-updates-ap7)
<a id="2026-10-12--opt-in-mcp-literal-rename-writer-ap6"></a>
- [2026-10-12 — Opt-in MCP literal rename writer (AP6)](history/implementation/2026-W42.md#2026-10-12--opt-in-mcp-literal-rename-writer-ap6)
<a id="2026-10-12--connection-matrix-object-inventory"></a>
- [2026-10-12 — Connection matrix object inventory](history/implementation/2026-W42.md#2026-10-12--connection-matrix-object-inventory)
<a id="2026-10-12--labelled-unsaved-changes-dialogs-compact-about-actionable-project-status"></a>
- [2026-10-12 — Labelled unsaved-changes dialogs, compact About, actionable project status](history/implementation/2026-W42.md#2026-10-12--labelled-unsaved-changes-dialogs-compact-about-actionable-project-status)
<a id="2026-10-12--offline-application-boundary-local-ap6-prerequisite"></a>
- [2026-10-12 — Offline application boundary, local AP6 prerequisite](history/implementation/2026-W42.md#2026-10-12--offline-application-boundary-local-ap6-prerequisite)
<a id="2026-10-12--four-group-address-presentations"></a>
- [2026-10-12 — Four group-address presentations](history/implementation/2026-W42.md#2026-10-12--four-group-address-presentations)
<a id="2026-10-12--port-8484-and-linux-host-network-deployment-defaults"></a>
- [2026-10-12 — Port 8484 and Linux host-network deployment defaults](history/implementation/2026-W42.md#2026-10-12--port-8484-and-linux-host-network-deployment-defaults)
<a id="2026-10-12--bounded-offline-literal-naming-recipe"></a>
- [2026-10-12 — Bounded offline literal naming recipe](history/implementation/2026-W42.md#2026-10-12--bounded-offline-literal-naming-recipe)
<a id="2026-10-12--release-notification-reader-documentation-completed"></a>
- [2026-10-12 — Release notification reader documentation completed](history/implementation/2026-W42.md#2026-10-12--release-notification-reader-documentation-completed)
<a id="2026-10-12--ap3-primary-contract-qualification-not-new-editors"></a>
- [2026-10-12 — AP3 primary contract qualification, not new editors](history/implementation/2026-W42.md#2026-10-12--ap3-primary-contract-qualification-not-new-editors)
<a id="2026-10-12--bounded-project-note-editor-leaf-candidate"></a>
- [2026-10-12 — Bounded project-note editor leaf candidate](history/implementation/2026-W42.md#2026-10-12--bounded-project-note-editor-leaf-candidate)
<a id="2026-10-12--named-dpts-and-bus-monitor-dropdown"></a>
- [2026-10-12 — Named DPTs and bus-monitor dropdown](history/implementation/2026-W42.md#2026-10-12--named-dpts-and-bus-monitor-dropdown)
## 2026-W41

<a id="2026-10-11--github-release-notifications"></a>
- [2026-10-11 — GitHub release notifications](history/implementation/2026-W41.md#2026-10-11--github-release-notifications)
<a id="2026-10-11--notes-authenticated-http-adapter-candidate"></a>
- [2026-10-11 — Notes authenticated HTTP adapter candidate](history/implementation/2026-W41.md#2026-10-11--notes-authenticated-http-adapter-candidate)
<a id="2026-10-11--alpha-9-release-media-and-manualsite-refresh"></a>
- [2026-10-11 — Alpha 9 release media and manual/site refresh](history/implementation/2026-W41.md#2026-10-11--alpha-9-release-media-and-manualsite-refresh)
<a id="2026-10-11--structure-headings-stop-fleeing-the-mouse-issue-5"></a>
- [2026-10-11 — Structure headings stop fleeing the mouse (issue #5)](history/implementation/2026-W41.md#2026-10-11--structure-headings-stop-fleeing-the-mouse-issue-5)
<a id="2026-10-11--ap8-collector-source-publication-verified"></a>
- [2026-10-11 — AP8 collector source publication verified](history/implementation/2026-W41.md#2026-10-11--ap8-collector-source-publication-verified)
<a id="2026-10-11--notes-foundationservice-bounded-integration-acceptance"></a>
- [2026-10-11 — Notes Foundation/service bounded integration acceptance](history/implementation/2026-W41.md#2026-10-11--notes-foundationservice-bounded-integration-acceptance)
<a id="2026-10-11--native-notes-service-candidate-ap4-still-incomplete"></a>
- [2026-10-11 — Native notes service candidate (AP4 still incomplete)](history/implementation/2026-W41.md#2026-10-11--native-notes-service-candidate-ap4-still-incomplete)
<a id="2026-10-11--native-notes-foundation-local-candidate-ap4-incomplete"></a>
- [2026-10-11 — Native notes Foundation local candidate (AP4 incomplete)](history/implementation/2026-W41.md#2026-10-11--native-notes-foundation-local-candidate-ap4-incomplete)
<a id="2026-10-11--ap3-optional-exact-parameter-source-evidence-not-editor-completion"></a>
- [2026-10-11 — AP3 optional exact parameter source evidence (not editor completion)](history/implementation/2026-W41.md#2026-10-11--ap3-optional-exact-parameter-source-evidence-not-editor-completion)
<a id="2026-10-11--native-instruction-foundation-not-the-full-rtf-editor"></a>
- [2026-10-11 — native instruction foundation, not the full RTF editor](history/implementation/2026-W41.md#2026-10-11--native-instruction-foundation-not-the-full-rtf-editor)
<a id="2026-10-11--bounded-installation-instruction-editing-accepted-for-session1-handoff"></a>
- [2026-10-11 — Bounded installation instruction editing accepted for Session1 handoff](history/implementation/2026-W41.md#2026-10-11--bounded-installation-instruction-editing-accepted-for-session1-handoff)
<a id="2026-10-11--importapplication-repair-accepted-on-main-integration"></a>
- [2026-10-11 — import/application repair accepted on main integration](history/implementation/2026-W41.md#2026-10-11--importapplication-repair-accepted-on-main-integration)
<a id="2026-10-11--read-only-recorded-device-state-accepted-locally"></a>
- [2026-10-11 — read-only recorded device state accepted locally](history/implementation/2026-W41.md#2026-10-11--read-only-recorded-device-state-accepted-locally)
<a id="2026-10-11--local-read-only-modeled-project-information"></a>
- [2026-10-11 — local read-only modeled project information](history/implementation/2026-W41.md#2026-10-11--local-read-only-modeled-project-information)
<a id="2026-10-11--local-reviewed-native-export-privacy"></a>
- [2026-10-11 — local reviewed native export privacy](history/implementation/2026-W41.md#2026-10-11--local-reviewed-native-export-privacy)
<a id="2026-10-11--local-retained-payload-catalogue-and-inert-download"></a>
- [2026-10-11 — local retained-payload catalogue and inert download](history/implementation/2026-W41.md#2026-10-11--local-retained-payload-catalogue-and-inert-download)
<a id="2026-10-11--local-selective-source-report-access"></a>
- [2026-10-11 — local selective-source report access](history/implementation/2026-W41.md#2026-10-11--local-selective-source-report-access)
<a id="2026-10-11--local-cli-original-report-capture-follow-up"></a>
- [2026-10-11 — local CLI original-report capture follow-up](history/implementation/2026-W41.md#2026-10-11--local-cli-original-report-capture-follow-up)
<a id="2026-10-11--local-durable-original-import-evidence"></a>
- [2026-10-11 — local durable original import evidence](history/implementation/2026-W41.md#2026-10-11--local-durable-original-import-evidence)
<a id="2026-10-11--local-importapplication-lifecycle-repair"></a>
- [2026-10-11 — local import/application lifecycle repair](history/implementation/2026-W41.md#2026-10-11--local-importapplication-lifecycle-repair)
<a id="2026-10-11--login-notification-details-and-size-limits-accepted-on-main-integration"></a>
- [2026-10-11 — login, notification details and size limits accepted on main integration](history/implementation/2026-W41.md#2026-10-11--login-notification-details-and-size-limits-accepted-on-main-integration)
<a id="2026-10-11--login-identity-actionable-notifications-and-larger-source-admission"></a>
- [2026-10-11 — login identity, actionable notifications and larger source admission](history/implementation/2026-W41.md#2026-10-11--login-identity-actionable-notifications-and-larger-source-admission)
<a id="2026-10-11--dpt-dropdown-published-on-main-after-merged-source-acceptance"></a>
- [2026-10-11 — DPT dropdown published on main after merged-source acceptance](history/implementation/2026-W41.md#2026-10-11--dpt-dropdown-published-on-main-after-merged-source-acceptance)
<a id="2026-10-11--local-ende-dpt-selection-and-field-provenance"></a>
- [2026-10-11 — local EN/DE DPT selection and field provenance](history/implementation/2026-W41.md#2026-10-11--local-ende-dpt-selection-and-field-provenance)
<a id="2026-10-11--installation-node-source-published-on-main"></a>
- [2026-10-11 — installation-node source published on main](history/implementation/2026-W41.md#2026-10-11--installation-node-source-published-on-main)
<a id="2026-10-11--installation-node-main-integration-accepted"></a>
- [2026-10-11 — installation-node main integration accepted](history/implementation/2026-W41.md#2026-10-11--installation-node-main-integration-accepted)
<a id="2026-10-11--import-status-published-on-main"></a>
- [2026-10-11 — import status published on main](history/implementation/2026-W41.md#2026-10-11--import-status-published-on-main)
<a id="2026-10-11--local-import-banner-readability-correction"></a>
- [2026-10-11 — local import-banner readability correction](history/implementation/2026-W41.md#2026-10-11--local-import-banner-readability-correction)
<a id="2026-10-11--device-download-layout-published-on-main"></a>
- [2026-10-11 — device-download layout published on main](history/implementation/2026-W41.md#2026-10-11--device-download-layout-published-on-main)
<a id="2026-10-11--local-device-download-configuration-layout-repair"></a>
- [2026-10-11 — local device-download configuration layout repair](history/implementation/2026-W41.md#2026-10-11--local-device-download-configuration-layout-repair)
<a id="2026-10-11--local-installation-nodes-get-a-nameplate-and-editable-properties"></a>
- [2026-10-11 — local installation nodes get a nameplate and editable Properties](history/implementation/2026-W41.md#2026-10-11--local-installation-nodes-get-a-nameplate-and-editable-properties)
<a id="2026-10-11--local-collector-delivery-and-scoped-measurement-contract"></a>
- [2026-10-11 — local collector delivery and scoped measurement contract](history/implementation/2026-W41.md#2026-10-11--local-collector-delivery-and-scoped-measurement-contract)
<a id="2026-10-11--ap9-hardware-prerequisites-and-plans-no-execution"></a>
- [2026-10-11 — AP9 hardware prerequisites and plans, no execution](history/implementation/2026-W41.md#2026-10-11--ap9-hardware-prerequisites-and-plans-no-execution)
<a id="2026-10-11--kl146-owner-approved-diagnostic-candidate"></a>
- [2026-10-11 — KL146 owner-approved diagnostic candidate](history/implementation/2026-W41.md#2026-10-11--kl146-owner-approved-diagnostic-candidate)
<a id="2026-10-11--bounded-rtf-adapter-foundation-ap2-still-open"></a>
- [2026-10-11 — bounded RTF adapter foundation (AP2 still open)](history/implementation/2026-W41.md#2026-10-11--bounded-rtf-adapter-foundation-ap2-still-open)
<a id="2026-10-10--secure-key-provenance-not-secure-runtime"></a>
- [2026-10-10 — Secure key provenance, not Secure runtime](history/implementation/2026-W41.md#2026-10-10--secure-key-provenance-not-secure-runtime)
<a id="2026-10-10--alpha8-released-and-independently-verified"></a>
- [2026-10-10 — Alpha.8 released and independently verified](history/implementation/2026-W41.md#2026-10-10--alpha8-released-and-independently-verified)
<a id="2026-10-11--kl175-bounded-raw-header-evidence-delivered"></a>
- [2026-10-11 — KL175 bounded raw-header evidence delivered](history/implementation/2026-W41.md#2026-10-11--kl175-bounded-raw-header-evidence-delivered)
<a id="2026-10-10--kl174-bounded-payload-contentrole-evidence"></a>
- [2026-10-10 — KL174 bounded payload content/role evidence](history/implementation/2026-W41.md#2026-10-10--kl174-bounded-payload-contentrole-evidence)
<a id="2026-10-10--kl-173-independent-servicenative-evidence"></a>
- [2026-10-10 — KL-173 independent service/native evidence](history/implementation/2026-W41.md#2026-10-10--kl-173-independent-servicenative-evidence)
<a id="2026-10-10--kl-1-final-source-preservation-acceptance"></a>
- [2026-10-10 — KL-1 final source-preservation acceptance](history/implementation/2026-W41.md#2026-10-10--kl-1-final-source-preservation-acceptance)
<a id="2026-10-10--kl-1-preservation-only-reconciliation-candidate"></a>
- [2026-10-10 — KL-1 preservation-only reconciliation candidate](history/implementation/2026-W41.md#2026-10-10--kl-1-preservation-only-reconciliation-candidate)
<a id="2026-10-10--archive-budgetreadable-refusals-main-integration-accepted"></a>
- [2026-10-10 — Archive budget/readable refusals: main integration accepted](history/implementation/2026-W41.md#2026-10-10--archive-budgetreadable-refusals-main-integration-accepted)
<a id="2026-10-10--import-regression-main-integration-acceptance"></a>
- [2026-10-10 — Import regression main-integration acceptance](history/implementation/2026-W41.md#2026-10-10--import-regression-main-integration-acceptance)
<a id="2026-10-10--kl-68pdb-01-final-integrated-source-admission"></a> <a id="implementation_statusmd"></a>
- [2026-10-10 — KL-68/PDB-01 final integrated source admission](history/implementation/2026-W41.md#2026-10-10--kl-68pdb-01-final-integrated-source-admission)
<a id="2026-10-11--alpha9-published-and-downloaded-artifacts-verified"></a>
- [2026-10-11 — Alpha9 published and downloaded artifacts verified](history/implementation/2026-W41.md#2026-10-11--alpha9-published-and-downloaded-artifacts-verified)
<a id="2026-10-10--project-archive-budget-1024-mib-local-source"></a>
- [2026-10-10 — Project archive budget: 1024 MiB (local source)](history/implementation/2026-W41.md#2026-10-10--project-archive-budget-1024-mib-local-source)
<a id="2026-10-10--kl-68pdb-01-own-stored-module-instance-values-read-only"></a>
- [2026-10-10 — KL-68/PDB-01: own stored module-instance values, read-only](history/implementation/2026-W41.md#2026-10-10--kl-68pdb-01-own-stored-module-instance-values-read-only)
<a id="2026-10-10--kl-170172-public-source-research-no-runtime-change"></a>
- [2026-10-10 — KL-170–172 public-source research (no runtime change)](history/implementation/2026-W41.md#2026-10-10--kl-170172-public-source-research-no-runtime-change)
<a id="2026-10-10--independent-local-revalidation-no-publication"></a>
- [2026-10-10 — Independent local revalidation, no publication](history/implementation/2026-W41.md#2026-10-10--independent-local-revalidation-no-publication)
<a id="2026-10-10--import-integrity-merged-candidate-accepted"></a> <a id="import-integrity-implementation--local-acceptance-2026-10-10"></a>
- [2026-10-10 — Import-integrity merged candidate accepted](history/implementation/2026-W41.md#2026-10-10--import-integrity-merged-candidate-accepted)
<a id="2026-10-10--local-categorized-test-catalogue"></a>
- [2026-10-10 — Local categorized test catalogue](history/implementation/2026-W41.md#2026-10-10--local-categorized-test-catalogue)
<a id="2026-10-10--kl-8-secure-capable-programs-without-secure-pinned"></a>
- [2026-10-10 — KL-8: secure-capable programs without Secure, pinned](history/implementation/2026-W41.md#2026-10-10--kl-8-secure-capable-programs-without-secure-pinned)
<a id="2026-10-10--kl-106-audit-retained-ets-source-data-stays-out-of-shared-outputs"></a>
- [2026-10-10 — KL-106 audit: retained ETS source data stays out of shared outputs](history/implementation/2026-W41.md#2026-10-10--kl-106-audit-retained-ets-source-data-stays-out-of-shared-outputs)
<a id="2026-10-10--private-schema-23-sample-kl-125-lifted-six-limitations-recorded"></a>
- [2026-10-10 — Private schema-23 sample: KL-125 lifted, six limitations recorded](history/implementation/2026-W41.md#2026-10-10--private-schema-23-sample-kl-125-lifted-six-limitations-recorded)
<a id="2026-10-10--import-expansion-combined-source-integration-accepted"></a> <a id="alpha7-published-and-externally-verified--2026-10-10"></a> <a id="alpha7-release-candidate--2026-10-10"></a>
- [2026-10-10 — Import expansion: combined-source integration accepted](history/implementation/2026-W41.md#2026-10-10--import-expansion-combined-source-integration-accepted)
<a id="2026-10-09--docker-uploads-and-companion-return-source-fixes"></a>
- [2026-10-09 — Docker uploads and companion return: source fixes](history/implementation/2026-W41.md#2026-10-09--docker-uploads-and-companion-return-source-fixes)
<a id="2026-10-10--selective-imports-original-local-acceptance"></a>
- [2026-10-10 — Selective imports: original local acceptance](history/implementation/2026-W41.md#2026-10-10--selective-imports-original-local-acceptance)
<a id="2026-10-09--communication-object-table-final-namesap1logo-candidate"></a>
- [2026-10-09 — Communication-object table: final names/AP1/logo candidate](history/implementation/2026-W41.md#2026-10-09--communication-object-table-final-namesap1logo-candidate)
<a id="2026-10-09--communication-object-table-meets-the-rename-desk-main-delivery"></a>
- [2026-10-09 — Communication-object table meets the rename desk (main delivery)](history/implementation/2026-W41.md#2026-10-09--communication-object-table-meets-the-rename-desk-main-delivery)
<a id="2026-10-09--communication-objects-get-headings-not-a-guessing-game-local"></a>
- [2026-10-09 — Communication objects get headings, not a guessing game (local)](history/implementation/2026-W41.md#2026-10-09--communication-objects-get-headings-not-a-guessing-game-local)
<a id="2026-10-09--header-logo-the-k-gets-a-proper-badge"></a>
- [2026-10-09 — Header logo: the K gets a proper badge](history/implementation/2026-W41.md#2026-10-09--header-logo-the-k-gets-a-proper-badge)
<a id="2026-10-09--offline-ap1-integration-keeps-its-bus-pass-offline"></a>
- [2026-10-09 — Offline AP1 integration keeps its bus pass offline](history/implementation/2026-W41.md#2026-10-09--offline-ap1-integration-keeps-its-bus-pass-offline)
<a id="2026-10-09--offline-ap1-evidence-order-without-a-bus-ticket"></a>
- [2026-10-09 — Offline AP1 evidence: order without a bus ticket](history/implementation/2026-W41.md#2026-10-09--offline-ap1-evidence-order-without-a-bus-ticket)
<a id="2026-10-09--names-can-change-identities-keep-their-day-job"></a>
- [2026-10-09 — Names can change; identities keep their day job](history/implementation/2026-W41.md#2026-10-09--names-can-change-identities-keep-their-day-job)
<a id="2026-10-09--native-history-remembers-restarts-not-hardware-writes"></a>
- [2026-10-09 — Native history remembers restarts, not hardware writes](history/implementation/2026-W41.md#2026-10-09--native-history-remembers-restarts-not-hardware-writes)
<a id="2026-10-09--bathroom-button-loaded-checked-put-back-research-1921"></a>
- [2026-10-09 — Bathroom button: loaded, checked, put back (RESEARCH §19.21)](history/implementation/2026-W41.md#2026-10-09--bathroom-button-loaded-checked-put-back-research-1921)
<a id="2026-10-09--download-coverage-main-integration-the-whole-suitcase-again"></a>
- [2026-10-09 — Download coverage main integration: the whole suitcase, again](history/implementation/2026-W41.md#2026-10-09--download-coverage-main-integration-the-whole-suitcase-again)
<a id="2026-10-09--full-download-coverage-stops-tripping-over-the-big-zip-kl-151"></a>
- [2026-10-09 — Full download coverage stops tripping over the big ZIP (KL §151)](history/implementation/2026-W41.md#2026-10-09--full-download-coverage-stops-tripping-over-the-big-zip-kl-151)
<a id="2026-10-09--the-attic-presence-detector-goes-live-and-comes-back-unchanged-research-1919"></a>
- [2026-10-09 — The attic presence detector goes live and comes back unchanged (RESEARCH §19.19)](history/implementation/2026-W41.md#2026-10-09--the-attic-presence-detector-goes-live-and-comes-back-unchanged-research-1919)
<a id="2026-10-09--legacy-programs-learn-to-download-on-paper-adr-0094-l4"></a>
- [2026-10-09 — Legacy programs learn to download, on paper (ADR-0094, L4)](history/implementation/2026-W41.md#2026-10-09--legacy-programs-learn-to-download-on-paper-adr-0094-l4)
<a id="2026-10-09--the-173-mb-vd5-gets-measured-then-let-in-adr-0094-vd5"></a>
- [2026-10-09 — The 173 MB `.vd5` gets measured, then let in (ADR-0094, VD5)](history/implementation/2026-W41.md#2026-10-09--the-173-mb-vd5-gets-measured-then-let-in-adr-0094-vd5)
<a id="2026-10-09--docker-hub-gets-its-name-badge"></a>
- [2026-10-09 — Docker Hub gets its name badge](history/implementation/2026-W41.md#2026-10-09--docker-hub-gets-its-name-badge)
<a id="2026-10-09--v010-alpha6-first-release-built-by-ci-first-one-on-docker-hub"></a>
- [2026-10-09 — `v0.1.0-alpha.6`: first release built by CI, first one on Docker Hub](history/implementation/2026-W41.md#2026-10-09--v010-alpha6-first-release-built-by-ci-first-one-on-docker-hub)
<a id="2026-10-09--typenone-rows-stop-pretending-to-be-input-fields-kl-128"></a>
- [2026-10-09 — TypeNone rows stop pretending to be input fields (KL §128)](history/implementation/2026-W41.md#2026-10-09--typenone-rows-stop-pretending-to-be-input-fields-kl-128)
<a id="2026-10-09--pages-catches-the-node-24-bus-offline-demos-get-a-signpost"></a>
- [2026-10-09 — Pages catches the Node-24 bus; offline demos get a signpost](history/implementation/2026-W41.md#2026-10-09--pages-catches-the-node-24-bus-offline-demos-get-a-signpost)
<a id="2026-10-09--release-tags-publish-the-server-image-to-docker-hub"></a>
- [2026-10-09 — Release tags publish the server image to Docker Hub](history/implementation/2026-W41.md#2026-10-09--release-tags-publish-the-server-image-to-docker-hub)
<a id="2026-10-09--three-local-packages-move-into-main-toast-parity-devices-navigation-community-demos"></a>
- [2026-10-09 — Three local packages move into main (toast parity, Devices navigation, community demos)](history/implementation/2026-W41.md#2026-10-09--three-local-packages-move-into-main-toast-parity-devices-navigation-community-demos)
<a id="2026-10-09--toast-parity-published-on-its-own-branch"></a>
- [2026-10-09 — Toast parity published on its own branch](history/implementation/2026-W41.md#2026-10-09--toast-parity-published-on-its-own-branch)
<a id="2026-10-08--toasts-now-keep-the-same-nine-second-appointment"></a>
- [2026-10-08 — Toasts now keep the same nine-second appointment](history/implementation/2026-W41.md#2026-10-08--toasts-now-keep-the-same-nine-second-appointment)
<a id="2026-10-08--devices-gets-a-seat-in-the-navigation-local-source-package"></a>
- [2026-10-08 — Devices gets a seat in the navigation (local source package)](history/implementation/2026-W41.md#2026-10-08--devices-gets-a-seat-in-the-navigation-local-source-package)
<a id="2026-10-08--post-rebase-community-links-verified"></a>
- [2026-10-08 — Post-rebase community links verified](history/implementation/2026-W41.md#2026-10-08--post-rebase-community-links-verified)
<a id="2026-10-08--three-english-community-demos-the-buildings-are-fictional-the-checks-are-not"></a>
- [2026-10-08 — Three English community demos: the buildings are fictional, the checks are not](history/implementation/2026-W41.md#2026-10-08--three-english-community-demos-the-buildings-are-fictional-the-checks-are-not)
<a id="2026-10-08--known-issues-and-implementation-status-fewer-fossils-wearing-current-badges"></a>
- [2026-10-08 — Known issues and implementation status: fewer fossils wearing “current” badges](history/implementation/2026-W41.md#2026-10-08--known-issues-and-implementation-status-fewer-fossils-wearing-current-badges)
<a id="2026-10-08--ideas-and-roadmap-audit-stop-putting-shipped-features-back-in-the-queue"></a>
- [2026-10-08 — Ideas and roadmap audit: stop putting shipped features back in the queue](history/implementation/2026-W41.md#2026-10-08--ideas-and-roadmap-audit-stop-putting-shipped-features-back-in-the-queue)
<a id="2026-10-08--website-goes-live-on-github-pages-knxbenchcom"></a>
- [2026-10-08 — Website goes live on GitHub Pages (knxbench.com)](history/implementation/2026-W41.md#2026-10-08--website-goes-live-on-github-pages-knxbenchcom)
<a id="2026-10-08--documentation-refresh-clearer-routes-real-visuals-fewer-time-machines"></a>
- [2026-10-08 — Documentation refresh: clearer routes, real visuals, fewer time machines](history/implementation/2026-W41.md#2026-10-08--documentation-refresh-clearer-routes-real-visuals-fewer-time-machines)
<a id="2026-10-08--community-evidence-lands-intake-moves-into-the-public-main-repository-adr-0091"></a>
- [2026-10-08 — Community evidence lands, intake moves into the public main repository (ADR-0091)](history/implementation/2026-W41.md#2026-10-08--community-evidence-lands-intake-moves-into-the-public-main-repository-adr-0091)
<a id="2026-10-08--marketing-website-lands-in-the-repository-adr-0095"></a>
- [2026-10-08 — Marketing website lands in the repository (ADR-0095)](history/implementation/2026-W41.md#2026-10-08--marketing-website-lands-in-the-repository-adr-0095)
<a id="2026-10-08--public-launch-history-purged-of-third-party-files-repository-public"></a>
- [2026-10-08 — Public launch: history purged of third-party files, repository public](history/implementation/2026-W41.md#2026-10-08--public-launch-history-purged-of-third-party-files-repository-public)
<a id="2026-10-08--legacy-vd-files-l3-web-upload-password-dialog-one-remembered-password-adr-0094"></a>
- [2026-10-08 — Legacy VD files L3: web upload, password dialog, one remembered password (ADR-0094)](history/implementation/2026-W41.md#2026-10-08--legacy-vd-files-l3-web-upload-password-dialog-one-remembered-password-adr-0094)
<a id="2026-10-08--gap-analysis-cloud-tooling-and-the-memory-index-retire"></a>
- [2026-10-08 — Gap analysis, cloud tooling and the memory index retire](history/implementation/2026-W41.md#2026-10-08--gap-analysis-cloud-tooling-and-the-memory-index-retire)
<a id="2026-10-08--archive-design-studies-and-superpowers-leave-the-tree"></a>
- [2026-10-08 — Archive, design studies and superpowers leave the tree](history/implementation/2026-W41.md#2026-10-08--archive-design-studies-and-superpowers-leave-the-tree)
<a id="2026-10-08--goal-audit-and-docsroot-cleanup"></a>
- [2026-10-08 — Goal audit and docs/root cleanup](history/implementation/2026-W41.md#2026-10-08--goal-audit-and-docsroot-cleanup)
<a id="2026-10-08--clean-parameter-editor-and-dedicated-inspection-tabs"></a>
- [2026-10-08 — Clean parameter editor and dedicated inspection tabs](history/implementation/2026-W41.md#2026-10-08--clean-parameter-editor-and-dedicated-inspection-tabs)
<a id="2026-10-08--legacy-vd-files-secret-class-values-withheld-from-the-stored-payload-adr-0094"></a>
- [2026-10-08 — Legacy VD files: secret-class values withheld from the stored payload (ADR-0094)](history/implementation/2026-W41.md#2026-10-08--legacy-vd-files-secret-class-values-withheld-from-the-stored-payload-adr-0094)
<a id="2026-10-08--legacy-vd-files-programs-imported-for-offline-use-l2-adr-0094"></a>
- [2026-10-08 — Legacy VD files: programs imported for offline use (L2, ADR-0094)](history/implementation/2026-W41.md#2026-10-08--legacy-vd-files-programs-imported-for-offline-use-l2-adr-0094)
<a id="2026-10-08--calm-lcars-ambient-source-published-and-activated"></a>
- [2026-10-08 — Calm LCARS ambient source published and activated](history/implementation/2026-W41.md#2026-10-08--calm-lcars-ambient-source-published-and-activated)
<a id="2026-10-08--calm-lcars-ambient-animation-follow-up-integrated-publication-accepted"></a>
- [2026-10-08 — Calm LCARS ambient animation follow-up (integrated publication accepted)](history/implementation/2026-W41.md#2026-10-08--calm-lcars-ambient-animation-follow-up-integrated-publication-accepted)
<a id="2026-10-08--legacy-vd-files-read-only-inspection-l1-adr-0094"></a>
- [2026-10-08 — Legacy VD files: read-only inspection (L1, ADR-0094)](history/implementation/2026-W41.md#2026-10-08--legacy-vd-files-read-only-inspection-l1-adr-0094)
<a id="2026-10-08--german-ui-uses-the-informal-du-form-throughout"></a>
- [2026-10-08 — German UI uses the informal du-form throughout](history/implementation/2026-W41.md#2026-10-08--german-ui-uses-the-informal-du-form-throughout)
<a id="2026-10-08--add-device-wizard-with-server-preview-and-placement-adr-0093"></a>
- [2026-10-08 — Add-device wizard with server preview and placement (ADR-0093)](history/implementation/2026-W41.md#2026-10-08--add-device-wizard-with-server-preview-and-placement-adr-0093)
<a id="2026-10-08--achievement-popups-linger-longer-and-leave-animated-adr-0089"></a>
- [2026-10-08 — Achievement popups linger longer and leave animated (ADR-0089)](history/implementation/2026-W41.md#2026-10-08--achievement-popups-linger-longer-and-leave-animated-adr-0089)
<a id="2026-10-08--new-project-wizard-with-an-atomic-starting-structure-adr-0093"></a>
- [2026-10-08 — New-project wizard with an atomic starting structure (ADR-0093)](history/implementation/2026-W41.md#2026-10-08--new-project-wizard-with-an-atomic-starting-structure-adr-0093)
<a id="2026-10-08--lcars-published-and-server-deployed"></a>
- [2026-10-08 — LCARS published and server deployed](history/implementation/2026-W41.md#2026-10-08--lcars-published-and-server-deployed)
<a id="2026-10-08--approved-lcars-theme-integrated-locally"></a>
- [2026-10-08 — Approved LCARS theme integrated locally](history/implementation/2026-W41.md#2026-10-08--approved-lcars-theme-integrated-locally)
<a id="2026-10-07--lcars-interactive-offline-study-visual-approval-pending"></a>
- [2026-10-07 — LCARS interactive offline study; visual approval pending](history/implementation/2026-W41.md#2026-10-07--lcars-interactive-offline-study-visual-approval-pending)
<a id="2026-10-08--mcp-parameter-visibility-paging-and-two-real-clients-adr-0090-amendment"></a>
- [2026-10-08 — MCP: parameter visibility, paging and two real clients (ADR-0090 amendment)](history/implementation/2026-W41.md#2026-10-08--mcp-parameter-visibility-paging-and-two-real-clients-adr-0090-amendment)
<a id="2026-10-07--read-only-mcp-server-and-agent-skill-adr-0090"></a>
- [2026-10-07 — Read-only MCP server and agent skill (ADR-0090)](history/implementation/2026-W41.md#2026-10-07--read-only-mcp-server-and-agent-skill-adr-0090)
<a id="2026-10-07--achievements-package-2-the-other-27-adr-0089"></a>
- [2026-10-07 — Achievements, package 2: the other 27 (ADR-0089)](history/implementation/2026-W41.md#2026-10-07--achievements-package-2-the-other-27-adr-0089)
<a id="2026-10-07--boarisch-and-klingonischklingon-join-the-language-picker"></a>
- [2026-10-07 — Boarisch and Klingonisch/Klingon join the language picker](history/implementation/2026-W41.md#2026-10-07--boarisch-and-klingonischklingon-join-the-language-picker)
<a id="2026-10-07--achievements-package-1-mechanism-and-11-achievements-adr-0089"></a>
- [2026-10-07 — Achievements, package 1: mechanism and 11 achievements (ADR-0089)](history/implementation/2026-W41.md#2026-10-07--achievements-package-1-mechanism-and-11-achievements-adr-0089)
<a id="2026-10-07--knx-server-leaves-on-sigterm-instead-of-being-killed"></a>
- [2026-10-07 — `knx-server` leaves on SIGTERM instead of being killed](history/implementation/2026-W41.md#2026-10-07--knx-server-leaves-on-sigterm-instead-of-being-killed)
<a id="2026-10-07--knx-server-speaks-https-by-itself-adr-0088"></a>
- [2026-10-07 — `knx-server` speaks HTTPS by itself (ADR-0088)](history/implementation/2026-W41.md#2026-10-07--knx-server-speaks-https-by-itself-adr-0088)
<a id="2026-10-07--one-name-everywhere-history-rewritten-app-identifier-changed-alpha5"></a>
- [2026-10-07 — One name everywhere: history rewritten, app identifier changed, alpha.5](history/implementation/2026-W41.md#2026-10-07--one-name-everywhere-history-rewritten-app-identifier-changed-alpha5)
<a id="2026-10-07--branch-consolidation-integrates-the-post-alpha-appimage-launcher"></a>
- [2026-10-07 — Branch consolidation integrates the post-alpha AppImage launcher](history/implementation/2026-W41.md#2026-10-07--branch-consolidation-integrates-the-post-alpha-appimage-launcher)
<a id="2026-10-07--dpt-inventory-honest-boundaries-and-rounding-that-stays-in-bounds"></a>
- [2026-10-07 — DPT inventory, honest boundaries and rounding that stays in bounds](history/implementation/2026-W41.md#2026-10-07--dpt-inventory-honest-boundaries-and-rounding-that-stays-in-bounds)
<a id="2026-10-07--evidence-of-record-and-the-house-downloads-32-of-35-devices"></a>
- [2026-10-07 — Evidence of record, and the house downloads 32 of 35 devices](history/implementation/2026-W41.md#2026-10-07--evidence-of-record-and-the-house-downloads-32-of-35-devices)
<a id="2026-10-07--flow-gets-room-to-breathe-a-window-and-actual-project-links"></a>
- [2026-10-07 — Flow gets room to breathe, a window and actual project links](history/implementation/2026-W41.md#2026-10-07--flow-gets-room-to-breathe-a-window-and-actual-project-links)
<a id="2026-10-07--the-readme-hero-does-something-and-docker-updates-in-one-go"></a>
- [2026-10-07 — The README hero does something, and Docker updates in one go](history/implementation/2026-W41.md#2026-10-07--the-readme-hero-does-something-and-docker-updates-in-one-go)
<a id="2026-10-07--kl-158-the-appimage-can-choose-wayland-without-unpacking-its-suitcase"></a>
- [2026-10-07 — KL-158: the AppImage can choose Wayland without unpacking its suitcase](history/implementation/2026-W41.md#2026-10-07--kl-158-the-appimage-can-choose-wayland-without-unpacking-its-suitcase)
<a id="2026-10-07--a-first-run-guide-says-what-this-build-is-before-it-says-what-to-click"></a>
- [2026-10-07 — A first-run guide says what this build is before it says what to click](history/implementation/2026-W41.md#2026-10-07--a-first-run-guide-says-what-this-build-is-before-it-says-what-to-click)
<a id="2026-10-07--v010-alpha4-the-first-alpha-is-tagged-and-pre-released"></a>
- [2026-10-07 — `v0.1.0-alpha.4`: the first alpha is tagged and pre-released](history/implementation/2026-W41.md#2026-10-07--v010-alpha4-the-first-alpha-is-tagged-and-pre-released)
<a id="2026-10-07--ar18-recorded-ready-the-alpha-candidate-is-release-ready-pending-the-users-decision"></a>
- [2026-10-07 — AR18 recorded `READY`: the alpha candidate is release-ready, pending the user's decision](history/implementation/2026-W41.md#2026-10-07--ar18-recorded-ready-the-alpha-candidate-is-release-ready-pending-the-users-decision)
<a id="2026-10-06--round-3s-condition-every-part-of-a-record-must-agree"></a>
- [2026-10-06 — Round 3's condition: every part of a record must agree](history/implementation/2026-W41.md#2026-10-06--round-3s-condition-every-part-of-a-record-must-agree)
<a id="2026-10-06--a-readme-that-leads-with-the-alpha-and-shows-the-bus-thinking"></a>
- [2026-10-06 — A README that leads with the alpha and shows the bus thinking](history/implementation/2026-W41.md#2026-10-06--a-readme-that-leads-with-the-alpha-and-shows-the-bus-thinking)
<a id="2026-10-06--agent-internals-back-on-stage"></a>
- [2026-10-06 — Agent internals back on stage](history/implementation/2026-W41.md#2026-10-06--agent-internals-back-on-stage)
<a id="2026-10-06--round-2s-condition-a-records-parts-must-agree"></a>
- [2026-10-06 — Round 2's condition: a record's parts must agree](history/implementation/2026-W41.md#2026-10-06--round-2s-condition-a-records-parts-must-agree)
<a id="2026-10-06--the-re-checks-conditions-the-readers-names-and-the-payloads-budget"></a>
- [2026-10-06 — The re-check's conditions: the reader's names and the payload's budget](history/implementation/2026-W41.md#2026-10-06--the-re-checks-conditions-the-readers-names-and-the-payloads-budget)
<a id="2026-10-06--the-minor-findings-m1m9-fixed-as-well"></a>
- [2026-10-06 — The minor findings M1–M9 fixed as well](history/implementation/2026-W41.md#2026-10-06--the-minor-findings-m1m9-fixed-as-well)
<a id="2026-10-06--ui-follow-ups-from-ar16-and-kl-61-honest-style-hint-styled-type-outcome"></a>
- [2026-10-06 — UI follow-ups from AR16 and KL-61: honest style hint, styled type outcome](history/implementation/2026-W41.md#2026-10-06--ui-follow-ups-from-ar16-and-kl-61-honest-style-hint-styled-type-outcome)
<a id="2026-10-06--the-reviewers-four-conditions-met-in-code-rather-than-waived"></a>
- [2026-10-06 — The reviewer's four conditions, met in code rather than waived](history/implementation/2026-W41.md#2026-10-06--the-reviewers-four-conditions-met-in-code-rather-than-waived)
<a id="2026-10-06--the-last-red-corpus-pair-turns-green-and-ui-04-gets-its-signature"></a>
- [2026-10-06 — The last red corpus pair turns green, and UI-04 gets its signature](history/implementation/2026-W41.md#2026-10-06--the-last-red-corpus-pair-turns-green-and-ui-04-gets-its-signature)
<a id="2026-10-06--ar18-gates-the-whole-candidate-goes-through-the-scanner"></a>
- [2026-10-06 — AR18 gates: the whole candidate goes through the scanner](history/implementation/2026-W41.md#2026-10-06--ar18-gates-the-whole-candidate-goes-through-the-scanner)
<a id="2026-10-06--kl-61-a-group-address-finally-says-what-it-declares-not-just-what-it-ends-up-with"></a>
- [2026-10-06 — KL-61: a group address finally says what it declares, not just what it ends up with](history/implementation/2026-W41.md#2026-10-06--kl-61-a-group-address-finally-says-what-it-declares-not-just-what-it-ends-up-with)
<a id="2026-10-06--the-user-signs-the-boundary-list-ar18-gets-a-reviewer"></a>
- [2026-10-06 — The user signs the boundary list; AR18 gets a reviewer](history/implementation/2026-W41.md#2026-10-06--the-user-signs-the-boundary-list-ar18-gets-a-reviewer)
<a id="2026-10-06--ar16-done-the-manual-meets-the-app-it-describes-sentence-by-sentence"></a>
- [2026-10-06 — AR16 done: the manual meets the app it describes, sentence by sentence](history/implementation/2026-W41.md#2026-10-06--ar16-done-the-manual-meets-the-app-it-describes-sentence-by-sentence)
<a id="2026-10-06--ui-04-web-half-live-bus-activity-kl-61-binding-wording"></a>
- [2026-10-06 — UI-04 Web half: live bus activity; KL-61 binding wording](history/implementation/2026-W41.md#2026-10-06--ui-04-web-half-live-bus-activity-kl-61-binding-wording)
<a id="2026-10-06--ar16-slice-1-the-manual-gets-new-photos-of-a-house-that-does-not-exist"></a>
- [2026-10-06 — AR16 slice 1: the manual gets new photos of a house that does not exist](history/implementation/2026-W41.md#2026-10-06--ar16-slice-1-the-manual-gets-new-photos-of-a-house-that-does-not-exist)
<a id="2026-10-06--ar17-a-real-appimage-weighed-and-sniffed-offline"></a>
- [2026-10-06 — AR17: a real AppImage, weighed and sniffed (offline)](history/implementation/2026-W41.md#2026-10-06--ar17-a-real-appimage-weighed-and-sniffed-offline)
<a id="2026-10-06--ar15-done-one-page-that-says-what-the-alpha-is-and-what-it-is-not-yet"></a>
- [2026-10-06 — AR15 done: one page that says what the Alpha is, and what it is not yet](history/implementation/2026-W41.md#2026-10-06--ar15-done-one-page-that-says-what-the-alpha-is-and-what-it-is-not-yet)
<a id="2026-10-06--ar15-slice-2-counting-what-is-left-and-saying-10-out-loud"></a>
- [2026-10-06 — AR15 slice 2: counting what is left, and saying "10" out loud](history/implementation/2026-W41.md#2026-10-06--ar15-slice-2-counting-what-is-left-and-saying-10-out-loud)
<a id="2026-10-06--ar15-slice-1-the-manual-stops-saying-never-about-downloads"></a>
- [2026-10-06 — AR15 slice 1: the manual stops saying "never" about downloads](history/implementation/2026-W41.md#2026-10-06--ar15-slice-1-the-manual-stops-saying-never-about-downloads)
<a id="2026-10-06--kl-157-a-project-upgrade-is-now-all-or-nothing"></a>
- [2026-10-06 — KL-157: a project upgrade is now all or nothing](history/implementation/2026-W41.md#2026-10-06--kl-157-a-project-upgrade-is-now-all-or-nothing)
<a id="2026-10-05--kl-153-abbs-ets4-shelf-gets-its-library-card-scheme-10-adr-0083"></a>
- [2026-10-05 — KL-153: ABB's ETS4 shelf gets its library card (scheme 10, ADR-0083)](history/implementation/2026-W41.md#2026-10-05--kl-153-abbs-ets4-shelf-gets-its-library-card-scheme-10-adr-0083)
<a id="2026-10-05--ar21-the-nervous-system-passes-its-physical-with-a-doctors-note-for-big-maps"></a>
- [2026-10-05 — AR21: the nervous system passes its physical (with a doctor's note for big maps)](history/implementation/2026-W41.md#2026-10-05--ar21-the-nervous-system-passes-its-physical-with-a-doctors-note-for-big-maps)
<a id="2026-10-05--kl-151-siemens-fits-through-the-door-if-you-hold-it-open-adr-0082"></a>
- [2026-10-05 — KL-151: Siemens fits through the door, if you hold it open (ADR-0082)](history/implementation/2026-W41.md#2026-10-05--kl-151-siemens-fits-through-the-door-if-you-hold-it-open-adr-0082)
<a id="2026-10-05--kl-37-catalogue-product-block-and-dpt-text-say-when-they-fell-back"></a>
- [2026-10-05 — KL-37: catalogue, product block and DPT text say when they fell back](history/implementation/2026-W41.md#2026-10-05--kl-37-catalogue-product-block-and-dpt-text-say-when-they-fell-back)
<a id="2026-10-05--telegram-flow-note-counts-refused-and-ring-dropped-telegrams-ar21-findings-6-7"></a>
- [2026-10-05 — Telegram-flow note counts refused and ring-dropped telegrams (AR21 findings 6, 7)](history/implementation/2026-W41.md#2026-10-05--telegram-flow-note-counts-refused-and-ring-dropped-telegrams-ar21-findings-6-7)
<a id="2026-10-05--ar10-slice-3-the-german-report-finally-says-schalten"></a>
- [2026-10-05 — AR10 slice 3: the German report finally says "Schalten"](history/implementation/2026-W41.md#2026-10-05--ar10-slice-3-the-german-report-finally-says-schalten)
<a id="2026-10-05--ar10-slice-2b-catalogue-product-block-and-master-data-stop-pretending-to-be-bilingual"></a>
- [2026-10-05 — AR10 slice 2b: catalogue, product block and master data stop pretending to be bilingual](history/implementation/2026-W41.md#2026-10-05--ar10-slice-2b-catalogue-product-block-and-master-data-stop-pretending-to-be-bilingual)
<a id="2026-10-05--parameter-panel-marks-labels-that-fell-back-to-the-programs-own-text"></a>
- [2026-10-05 — Parameter panel marks labels that fell back to the program's own text](history/implementation/2026-W41.md#2026-10-05--parameter-panel-marks-labels-that-fell-back-to-the-programs-own-text)
<a id="2026-10-05--ar10-slice-2a-the-parameter-panel-admits-when-it-is-speaking-the-packages-language"></a>
- [2026-10-05 — AR10 slice 2a: the parameter panel admits when it is speaking the package's language](history/implementation/2026-W41.md#2026-10-05--ar10-slice-2a-the-parameter-panel-admits-when-it-is-speaking-the-packages-language)
<a id="2026-10-05--ar10-slice-1-where-every-language-comes-from-and-where-none-does"></a>
- [2026-10-05 — AR10 slice 1: where every language comes from (and where none does)](history/implementation/2026-W41.md#2026-10-05--ar10-slice-1-where-every-language-comes-from-and-where-none-does)
<a id="2026-10-05--kl-156-the-install-report-finally-admits-what-it-skipped-adr-0081"></a>
- [2026-10-05 — KL-156: the install report finally admits what it skipped (ADR-0081)](history/implementation/2026-W41.md#2026-10-05--kl-156-the-install-report-finally-admits-what-it-skipped-adr-0081)
<a id="2026-10-05--parameter-panel-adopts-adr-0080s-write-authority-reasons"></a>
- [2026-10-05 — Parameter panel adopts ADR-0080's write-authority reasons](history/implementation/2026-W41.md#2026-10-05--parameter-panel-adopts-adr-0080s-write-authority-reasons)
<a id="2026-10-05--kl-142-the-download-tab-asks-what-to-write"></a>
- [2026-10-05 — KL-142: the download tab asks what to write](history/implementation/2026-W41.md#2026-10-05--kl-142-the-download-tab-asks-what-to-write)
<a id="2026-10-05--ar07-the-manufacturers-hands-off-now-means-hands-off-adr-0080"></a>
- [2026-10-05 — AR07: the manufacturer's "hands off" now means hands off (ADR-0080)](history/implementation/2026-W41.md#2026-10-05--ar07-the-manufacturers-hands-off-now-means-hands-off-adr-0080)
<a id="2026-10-05--telegram-flow-ar21-finding-5-corrected"></a>
- [2026-10-05 — Telegram flow: AR21 finding 5 corrected](history/implementation/2026-W41.md#2026-10-05--telegram-flow-ar21-finding-5-corrected)
<a id="2026-10-05--tunnelling-through-dockers-bridge-network-ar14b-1fd1664a"></a>
- [2026-10-05 — Tunnelling through Docker's bridge network (AR14B, `1fd1664a`)](history/implementation/2026-W41.md#2026-10-05--tunnelling-through-dockers-bridge-network-ar14b-1fd1664a)
<a id="2026-10-05--themes-one-dropdown-crt-shipped-storage-location-shown-adr-0079"></a>
- [2026-10-05 — Themes: one dropdown, CRT shipped, storage location shown (ADR-0079)](history/implementation/2026-W41.md#2026-10-05--themes-one-dropdown-crt-shipped-storage-location-shown-adr-0079)
<a id="2026-10-05--left-column-splitters-work-again-with-a-project-loaded"></a>
- [2026-10-05 — Left-column splitters work again with a project loaded](history/implementation/2026-W41.md#2026-10-05--left-column-splitters-work-again-with-a-project-loaded)
<a id="2026-10-05--ar06-rows-reconciled-seven-todos-find-their-place"></a>
- [2026-10-05 — AR06 rows reconciled: seven TODOs find their place](history/implementation/2026-W41.md#2026-10-05--ar06-rows-reconciled-seven-todos-find-their-place)
<a id="2026-10-05--ar21-finding-4-the-flow-note-stops-counting-heads-twice"></a>
- [2026-10-05 — AR21 finding 4: the flow note stops counting heads twice](history/implementation/2026-W41.md#2026-10-05--ar21-finding-4-the-flow-note-stops-counting-heads-twice)
<a id="2026-10-05--ar09-a-group-address-finally-says-what-it-carries-adr-0078"></a>
- [2026-10-05 — AR09: a group address finally says what it carries (ADR 0078)](history/implementation/2026-W41.md#2026-10-05--ar09-a-group-address-finally-says-what-it-carries-adr-0078)
<a id="2026-10-05--ar21-corrections-the-flow-map-learns-to-sit-still"></a>
- [2026-10-05 — AR21 corrections: the flow map learns to sit still](history/implementation/2026-W41.md#2026-10-05--ar21-corrections-the-flow-map-learns-to-sit-still)
<a id="2026-10-05--ar09-parameter-only-time-periods-say-so-before-a-write"></a>
- [2026-10-05 — AR09: parameter-only time periods say so before a write](history/implementation/2026-W41.md#2026-10-05--ar09-parameter-only-time-periods-say-so-before-a-write)
<a id="2026-10-05--ar09-17001-scene-numbers-travel-in-their-own-octet"></a>
- [2026-10-05 — AR09: 17.001 scene numbers travel in their own octet](history/implementation/2026-W41.md#2026-10-05--ar09-17001-scene-numbers-travel-in-their-own-octet)
<a id="2026-10-05--ar06y-cli-refuses-oversized-product-packages-before-reading-them"></a>
- [2026-10-05 — AR06Y: CLI refuses oversized product packages before reading them](history/implementation/2026-W41.md#2026-10-05--ar06y-cli-refuses-oversized-product-packages-before-reading-them)
<a id="2026-10-05--ar06x-actual-raw-input-guard-contracts-local-native-evidence"></a>
- [2026-10-05 — AR06X: actual raw-input guard contracts (local native evidence)](history/implementation/2026-W41.md#2026-10-05--ar06x-actual-raw-input-guard-contracts-local-native-evidence)
<a id="2026-10-05--commissioning-callers-published-web-halves-handed-over"></a>
- [2026-10-05 — Commissioning callers published; Web halves handed over](history/implementation/2026-W41.md#2026-10-05--commissioning-callers-published-web-halves-handed-over)
<a id="2026-10-05--cli-serial-lookup-read-lifecycle-local-candidate-not-integratedpublished"></a>
- [2026-10-05 — CLI serial-lookup read lifecycle (local candidate, not integrated/published)](history/implementation/2026-W41.md#2026-10-05--cli-serial-lookup-read-lifecycle-local-candidate-not-integratedpublished)
<a id="2026-10-05--service-control-recovery-record-admission-local-candidate"></a>
- [2026-10-05 — Service-control recovery-record admission (local candidate)](history/implementation/2026-W41.md#2026-10-05--service-control-recovery-record-admission-local-candidate)
<a id="2026-10-05--ar21-review-the-flow-view-goes-back-for-one-more-lap"></a>
- [2026-10-05 — AR21 review: the flow view goes back for one more lap](history/implementation/2026-W41.md#2026-10-05--ar21-review-the-flow-view-goes-back-for-one-more-lap)
## 2026-W40

<a id="2026-10-04--shared-commissioning-caller-candidate-integration-pending"></a>
- [2026-10-04 — Shared commissioning Caller candidate (integration pending)](history/implementation/2026-W40.md#2026-10-04--shared-commissioning-caller-candidate-integration-pending)
<a id="2026-10-04--ar06w-bounded-declared-byte-admission-contracts"></a>
- [2026-10-04 — AR06W: bounded declared-byte admission contracts](history/implementation/2026-W40.md#2026-10-04--ar06w-bounded-declared-byte-admission-contracts)
<a id="2026-10-04--u21-part-c-measured-then-made-lighter"></a>
- [2026-10-04 — U21 part C: measured, then made lighter](history/implementation/2026-W40.md#2026-10-04--u21-part-c-measured-then-made-lighter)
<a id="2026-10-04--u21-parts-a-and-b-the-flow-view-moves-and-stops-when-asked"></a>
- [2026-10-04 — U21 parts A and B: the flow view moves, and stops when asked](history/implementation/2026-W40.md#2026-10-04--u21-parts-a-and-b-the-flow-view-moves-and-stops-when-asked)
<a id="2026-10-04--u20-part-2-the-bus-monitor-gets-a-flow-view"></a>
- [2026-10-04 — U20 part 2: the bus monitor gets a Flow view](history/implementation/2026-W40.md#2026-10-04--u20-part-2-the-bus-monitor-gets-a-flow-view)
<a id="2026-10-04--u20-part-1-the-telegram-flow-reducer-and-wire-validation"></a>
- [2026-10-04 — U20 part 1: the telegram-flow reducer and wire validation](history/implementation/2026-W40.md#2026-10-04--u20-part-1-the-telegram-flow-reducer-and-wire-validation)
<a id="2026-10-04--kl-60-the-diff-views-long-tables-filter-and-scroll-instead-of-paging"></a>
- [2026-10-04 — KL-60: the diff view's long tables filter and scroll instead of paging](history/implementation/2026-W40.md#2026-10-04--kl-60-the-diff-views-long-tables-filter-and-scroll-instead-of-paging)
<a id="2026-10-04--ar08-web-half-the-project-password-dialog"></a>
- [2026-10-04 — AR08 Web half: the project-password dialog](history/implementation/2026-W40.md#2026-10-04--ar08-web-half-the-project-password-dialog)
<a id="2026-10-04--ar20-the-telegram-flow-backend-contract"></a>
- [2026-10-04 — AR20: the telegram-flow backend contract](history/implementation/2026-W40.md#2026-10-04--ar20-the-telegram-flow-backend-contract)
<a id="2026-10-04--u19-telegram-flow-study-measured-ar20-handoff-written"></a>
- [2026-10-04 — U19: telegram-flow study measured, AR20 handoff written](history/implementation/2026-W40.md#2026-10-04--u19-telegram-flow-study-measured-ar20-handoff-written)
<a id="2026-10-04--ar08-password-protected-projects-reach-the-importer"></a>
- [2026-10-04 — AR08: password-protected projects reach the importer](history/implementation/2026-W40.md#2026-10-04--ar08-password-protected-projects-reach-the-importer)
<a id="2026-10-04--ar06v-existing-product-zip-count-contracts-verified"></a>
- [2026-10-04 — AR06V: existing product-ZIP count contracts verified](history/implementation/2026-W40.md#2026-10-04--ar06v-existing-product-zip-count-contracts-verified)
<a id="2026-10-04--telegram-flow-nervous-system-alpha-scope-approved-planning-only"></a>
- [2026-10-04 — Telegram-flow nervous-system Alpha scope approved (planning only)](history/implementation/2026-W40.md#2026-10-04--telegram-flow-nervous-system-alpha-scope-approved-planning-only)
<a id="2026-10-04--ar13-hand-over-the-debug-report-dialog-names-every-kept-telegram-field"></a>
- [2026-10-04 — AR13 hand-over: the debug-report dialog names every kept telegram field](history/implementation/2026-W40.md#2026-10-04--ar13-hand-over-the-debug-report-dialog-names-every-kept-telegram-field)
<a id="2026-10-04--ux-01-drag-a-group-address-onto-a-communication-object"></a>
- [2026-10-04 — UX-01: drag a group address onto a communication object](history/implementation/2026-W40.md#2026-10-04--ux-01-drag-a-group-address-onto-a-communication-object)
<a id="2026-10-04--model-02-web-half-ambiguous-placements-are-repaired-by-an-explicit-choice"></a>
- [2026-10-04 — MODEL-02 web half: ambiguous placements are repaired by an explicit choice](history/implementation/2026-W40.md#2026-10-04--model-02-web-half-ambiguous-placements-are-repaired-by-an-explicit-choice)
<a id="2026-10-04--model-01-web-half-part-2-every-installation-is-edited-in-place"></a>
- [2026-10-04 — MODEL-01 web half, part 2: every installation is edited in place](history/implementation/2026-W40.md#2026-10-04--model-01-web-half-part-2-every-installation-is-edited-in-place)
<a id="2026-10-04--model-01-web-half-part-1-every-installation-can-be-built-and-named"></a>
- [2026-10-04 — MODEL-01 web half, part 1: every installation can be built and named](history/implementation/2026-W40.md#2026-10-04--model-01-web-half-part-1-every-installation-can-be-built-and-named)
<a id="2026-10-04--ar14d-d5-resolved-limitation-bodies-to-history-goal-ui-status-to-its-dossier"></a>
- [2026-10-04 — AR14D D5: resolved limitation bodies to history, goal-ui status to its dossier](history/implementation/2026-W40.md#2026-10-04--ar14d-d5-resolved-limitation-bodies-to-history-goal-ui-status-to-its-dossier)
<a id="2026-10-04--ar14d-d4-research-split-by-topic"></a>
- [2026-10-04 — AR14D D4: RESEARCH split by topic](history/implementation/2026-W40.md#2026-10-04--ar14d-d4-research-split-by-topic)
<a id="2026-10-04--ar14d-d3-xtask-check-ledger-guards-the-single-status-record"></a>
- [2026-10-04 — AR14D D3: `xtask check-ledger` guards the single status record](history/implementation/2026-W40.md#2026-10-04--ar14d-d3-xtask-check-ledger-guards-the-single-status-record)
<a id="2026-10-04--ar14d-d2-one-source-id-ledger-instead-of-six-status-tables"></a>
- [2026-10-04 — AR14D D2: one source-ID ledger instead of six status tables](history/implementation/2026-W40.md#2026-10-04--ar14d-d2-one-source-id-ledger-instead-of-six-status-tables)
<a id="2026-10-04--ar14d-d1-status-documents-slimmed-consolidation-planned-before-ar15"></a>
- [2026-10-04 — AR14D D1: status documents slimmed, consolidation planned before AR15](history/implementation/2026-W40.md#2026-10-04--ar14d-d1-status-documents-slimmed-consolidation-planned-before-ar15)
<a id="2026-10-04--model-04-web-half-catalog-address-allocation-and-unique-names"></a>
- [2026-10-04 — MODEL-04 web half: catalog address allocation and unique names](history/implementation/2026-W40.md#2026-10-04--model-04-web-half-catalog-address-allocation-and-unique-names)
<a id="2026-10-04--data-03-web-half-a-lost-catalog-batch-can-be-retried-safely"></a>
- [2026-10-04 — DATA-03 web half: a lost catalog batch can be retried safely](history/implementation/2026-W40.md#2026-10-04--data-03-web-half-a-lost-catalog-batch-can-be-retried-safely)
<a id="2026-10-04--model-03-web-half-device-number-0-for-evidenced-couplers"></a>
- [2026-10-04 — MODEL-03 web half: device number 0 for evidenced couplers](history/implementation/2026-W40.md#2026-10-04--model-03-web-half-device-number-0-for-evidenced-couplers)
<a id="2026-10-04--bounded-history-web-closure"></a>
- [2026-10-04 — Bounded History Web closure](history/implementation/2026-W40.md#2026-10-04--bounded-history-web-closure)
<a id="2026-10-04--bounded-history-web-adoption-accepted-offline"></a>
- [2026-10-04 — Bounded History Web adoption accepted offline](history/implementation/2026-W40.md#2026-10-04--bounded-history-web-adoption-accepted-offline)
<a id="2026-10-04--commissioning-history-web-candidate-not-whole-track-acceptance"></a>
- [2026-10-04 — Commissioning History Web candidate, not whole-track acceptance](history/implementation/2026-W40.md#2026-10-04--commissioning-history-web-candidate-not-whole-track-acceptance)
<a id="2026-10-04--desktop-shell-recovers-a-terminated-web-process-133"></a>
- [2026-10-04 — Desktop shell recovers a terminated web process (§133)](history/implementation/2026-W40.md#2026-10-04--desktop-shell-recovers-a-terminated-web-process-133)
<a id="2026-10-04--ar13-privacy-authentication-and-provenance-claude-session"></a>
- [2026-10-04 — AR13: privacy, authentication and provenance (Claude session)](history/implementation/2026-W40.md#2026-10-04--ar13-privacy-authentication-and-provenance-claude-session)
<a id="2026-10-04--ar14-offline-buscli-contracts-claude-session"></a>
- [2026-10-04 — AR14: offline bus/CLI contracts (Claude session)](history/implementation/2026-W40.md#2026-10-04--ar14-offline-buscli-contracts-claude-session)
<a id="2026-10-04--ua10-csv-group-address-exchange-per-installation-model-01"></a>
- [2026-10-04 — UA10: CSV group-address exchange per installation (MODEL-01)](history/implementation/2026-W40.md#2026-10-04--ua10-csv-group-address-exchange-per-installation-model-01)
<a id="2026-10-04--ua7ua8-no-silent-loss-between-import-memory-and-knxdb"></a> <a id="ar06t-exact23-bounded-import--delivered-2026-10-04-aadd8820"></a>
- [2026-10-04 — UA7/UA8: no silent loss between import, memory and `.knxdb`](history/implementation/2026-W40.md#2026-10-04--ua7ua8-no-silent-loss-between-import-memory-and-knxdb)
<a id="2026-10-04--ua5-explicit-topology-repair-model-02-corestoreserver-half"></a>
- [2026-10-04 — UA5: explicit topology repair (MODEL-02, core/store/server half)](history/implementation/2026-W40.md#2026-10-04--ua5-explicit-topology-repair-model-02-corestoreserver-half)
<a id="2026-10-04--ua4-every-installation-editable-in-the-core-model-01-coreserver-half"></a>
- [2026-10-04 — UA4: every installation editable in the core (MODEL-01, core/server half)](history/implementation/2026-W40.md#2026-10-04--ua4-every-installation-editable-in-the-core-model-01-coreserver-half)
<a id="2026-10-04--ua3-opt-in-address-allocation-and-unique-names-model-04-server-half"></a>
- [2026-10-04 — UA3: opt-in address allocation and unique names (MODEL-04, server half)](history/implementation/2026-W40.md#2026-10-04--ua3-opt-in-address-allocation-and-unique-names-model-04-server-half)
<a id="2026-10-04--ua2-catalog-batch-replay-token-data-03-server-half"></a>
- [2026-10-04 — UA2: catalog batch replay token (DATA-03, server half)](history/implementation/2026-W40.md#2026-10-04--ua2-catalog-batch-replay-token-data-03-server-half)
<a id="2026-10-04--ua1-coupler-0-with-manufacturer-evidence-kl-127-closed-as-known-gap"></a>
- [2026-10-04 — UA1: coupler `.0` with manufacturer evidence; KL-127 closed as known gap](history/implementation/2026-W40.md#2026-10-04--ua1-coupler-0-with-manufacturer-evidence-kl-127-closed-as-known-gap)
<a id="2026-10-04--commissioning-validation-scope-and-requested-continuation"></a>
- [2026-10-04 — Commissioning validation scope and requested continuation](history/implementation/2026-W40.md#2026-10-04--commissioning-validation-scope-and-requested-continuation)
<a id="2026-10-04--project-evolution-story-first-private-version-companion-not-product"></a> <a id="ar06s-kl153-scheme23-bounded-research--2026-10-04"></a> <a id="ar06r-kl-152-coupled-budget-delivery--2026-10-04"></a> <a id="commissioning-lifecycle-integrated-offline-implementation-accepted--2026-10-04"></a> <a id="commissioning-lifecycle-candidate-offline-closure--2026-10-04"></a> <a id="commissioning-lifecycle-storage-candidate--2026-10-03-0814-cest"></a> <a id="crt-authorized-integrationpublication-follow-up"></a>
- [2026-10-04 — Project-evolution story, first private version (companion, not product)](history/implementation/2026-W40.md#2026-10-04--project-evolution-story-first-private-version-companion-not-product)
<a id="2026-10-03--productive-crt-interactions-local-feature-worktree"></a>
- [2026-10-03 — Productive CRT interactions (local feature worktree)](history/implementation/2026-W40.md#2026-10-03--productive-crt-interactions-local-feature-worktree)
<a id="2026-10-03--crt-11-branch-publication"></a>
- [2026-10-03 — CRT 1.1 branch publication](history/implementation/2026-W40.md#2026-10-03--crt-11-branch-publication)
<a id="2026-10-03--crt-11-reference-image-development-local"></a>
- [2026-10-03 — CRT 1.1 reference-image development (local)](history/implementation/2026-W40.md#2026-10-03--crt-11-reference-image-development-local)
<a id="2026-10-03--modern-retro-green-crt-design-artifacts-local"></a> <a id="ar06q-kl-151-resource-research-accepted--2026-10-03-2202-utc"></a> <a id="ar06p-kl-150-scoped-code-delivered--2026-10-03-2305-cest"></a> <a id="ar06p-kl-150-full-candidate-verification--2026-10-03-2235-cest"></a> <a id="ar06p-kl-150-lexical-storage-candidate--2026-10-03-2056-cest"></a> <a id="ar06p-kl-149-scoped-delivery-checked--2026-10-03-1845-cest"></a> <a id="ar07-checked-outside-walk-scoped-delivery--2026-10-03-1354-cest"></a> <a id="ar07-checked-text-overlay-actual-acceptance--2026-10-03-1332-cest"></a> <a id="ar07-outside-walk-candidate-broad-accepted--2026-10-03-1245-cest"></a> <a id="ar07-remaining-public-audit-inventory-correction--2026-10-03-1227-cest"></a> <a id="ar07-public-broad-runner-rejection--2026-10-03-1214-cest"></a> <a id="ar07-outside-walk-current-public-acceptance--2026-10-03-1147-cest"></a> <a id="ar07-outside-walk-text-refusal--candidate-only-2026-10-03-1123-cest"></a>
- [2026-10-03 — Modern Retro Green CRT design artifacts (local)](history/implementation/2026-W40.md#2026-10-03--modern-retro-green-crt-design-artifacts-local)
<a id="2026-10-03--product-install-tested-against-853-public-manufacturer-downloads-test-only"></a>
- [2026-10-03 — Product install tested against 853 public manufacturer downloads (test only)](history/implementation/2026-W40.md#2026-10-03--product-install-tested-against-853-public-manufacturer-downloads-test-only)
<a id="2026-10-03--u17-appearance-manager-deliveredread-back-as-4d9073ca"></a> <a id="ar07-bounded-scalar-copy-policy-delivered--2026-10-03-0959-cest"></a> <a id="ar07-scalar-copy-actual-integration-accepted--2026-10-03-0952-cest"></a> <a id="ar07-scalar-copy-audit--three-valid-public-reds-2026-10-03"></a> <a id="commissioning-bounded-history-source-delivered--2026-10-03-0710-cest"></a> <a id="commissioning-actual-integrated-backend-accepted--2026-10-03-0659-cest"></a> <a id="commissioning-durable-metadata--admission-candidate--2026-10-03"></a> <a id="ar07-module-scope-backend-leaf-delivered--2026-10-03-0548-cest"></a> <a id="ar07-module-scope-provenance--actual-integration-accepted-2026-10-03-0538-cest"></a> <a id="ar07-module-scope-provenance--public-green-not-delivered-2026-10-03"></a> <a id="ar07-bounded-work-admission-delivered--2026-10-03-0410-cest"></a> <a id="ar07-work-admission--actual-merge-accepted-delivery-pending-2026-10-03"></a> <a id="ar07-work-admission-candidate--public-green-not-delivered-2026-10-03"></a>
- [2026-10-03 — U17 Appearance manager (delivered/read back as 4d9073ca)](history/implementation/2026-W40.md#2026-10-03--u17-appearance-manager-deliveredread-back-as-4d9073ca)
<a id="2026-10-03--u16-acknowledged-theme-persistence-foundation-delivered-as-1f94808d"></a> <a id="ar07-scoped-float-delivery--2026-10-03-0004-cest"></a> <a id="ar07-float-actual-merged-checkpoint-accepted--2026-10-02-2355-cest"></a> <a id="ar07-float-owned-final-checkpoint-accepted--2026-10-02-2309-cest"></a> <a id="ar07-float-public-baseline--guard-mutation-checkpoint--2026-10-02-2234-cest"></a> <a id="ar07-remaining-audit--synthetic-validation-red-dispatched--2026-10-02-2132-cest"></a> <a id="commissioning-recovery-backup-before-verify-mode--2026-10-02"></a> <a id="ar07-bounded-controller-integrated-checkpoint--2026-10-02-2056-cest"></a> <a id="ar07-controller-broad-candidate-verified--2026-10-02-2014-cest"></a> <a id="ar07-bounded-controller-candidate--2026-10-02-1843-cest"></a>
- [2026-10-03 — U16 acknowledged theme persistence (foundation delivered as 1f94808d)](history/implementation/2026-W40.md#2026-10-03--u16-acknowledged-theme-persistence-foundation-delivered-as-1f94808d)
<a id="2026-10-02-1714-cest--ar07-offline-source-discovery-baseline-pending"></a>
- [2026-10-02 17:14 CEST — AR07 offline source discovery; baseline pending](history/implementation/2026-W40.md#2026-10-02-1714-cest--ar07-offline-source-discovery-baseline-pending)
<a id="2026-10-02--u15-theme-runtime-foundation-delivered-extension-still-open"></a>
- [2026-10-02 — U15 theme runtime foundation (delivered, extension still open)](history/implementation/2026-W40.md#2026-10-02--u15-theme-runtime-foundation-delivered-extension-still-open)
<a id="2026-10-02-1659-cest--ar06-scoped-delivery-verified"></a>
- [2026-10-02 16:59 CEST — AR06 scoped delivery verified](history/implementation/2026-W40.md#2026-10-02-1659-cest--ar06-scoped-delivery-verified)
<a id="2026-10-02-1656-cest--ar06-integrated-receipt-publication-pending"></a>
- [2026-10-02 16:56 CEST — AR06 integrated receipt (publication pending)](history/implementation/2026-W40.md#2026-10-02-1656-cest--ar06-integrated-receipt-publication-pending)
<a id="2026-10-02--ar06-whole-diff-review-checkpoint-offline-1552-cest"></a>
- [2026-10-02 — AR06 whole-diff review checkpoint (offline; 15:52 CEST)](history/implementation/2026-W40.md#2026-10-02--ar06-whole-diff-review-checkpoint-offline-1552-cest)
<a id="2026-10-02--ar06-mappingnative-and-raw-field-checkpoint-offline-1434-cest"></a>
- [2026-10-02 — AR06 mapping/native and raw-field checkpoint (offline; 14:34 CEST)](history/implementation/2026-W40.md#2026-10-02--ar06-mappingnative-and-raw-field-checkpoint-offline-1434-cest)
<a id="2026-10-02--ar06-changed-legacy-candidate-wider-receipt-offline-1316-cest"></a>
- [2026-10-02 — AR06 changed legacy candidate wider receipt (offline; 13:16 CEST)](history/implementation/2026-W40.md#2026-10-02--ar06-changed-legacy-candidate-wider-receipt-offline-1316-cest)
<a id="2026-10-02--ar06-legacy-and-cli-master-checkpoint-offline-1302-cest"></a>
- [2026-10-02 — AR06 legacy and CLI master checkpoint (offline; 13:02 CEST)](history/implementation/2026-W40.md#2026-10-02--ar06-legacy-and-cli-master-checkpoint-offline-1302-cest)
<a id="2026-10-02--ar06-changed-master-candidate-wider-receipt-offline-1054-cest"></a>
- [2026-10-02 — AR06 changed master candidate wider receipt (offline; 10:54 CEST)](history/implementation/2026-W40.md#2026-10-02--ar06-changed-master-candidate-wider-receipt-offline-1054-cest)
<a id="2026-10-02--ar06-master-metadataadmissionnative-checkpoint-offline-1042-cest"></a>
- [2026-10-02 — AR06 master metadata/admission/native checkpoint (offline; 10:42 CEST)](history/implementation/2026-W40.md#2026-10-02--ar06-master-metadataadmissionnative-checkpoint-offline-1042-cest)
<a id="2026-10-02--ar06-root-candidate-wider-gate-receipt-offline-1004-cest"></a>
- [2026-10-02 — AR06 root candidate wider-gate receipt (offline; 10:04 CEST)](history/implementation/2026-W40.md#2026-10-02--ar06-root-candidate-wider-gate-receipt-offline-1004-cest)
<a id="2026-10-02--ar06-root-import-boundary-checkpoint-offline-0851-cest"></a>
- [2026-10-02 — AR06 root import-boundary checkpoint (offline; 08:51 CEST)](history/implementation/2026-W40.md#2026-10-02--ar06-root-import-boundary-checkpoint-offline-0851-cest)
<a id="2026-10-02--u14-theme-pack-contract-implementation-pending"></a>
- [2026-10-02 — U14 theme-pack contract (implementation pending)](history/implementation/2026-W40.md#2026-10-02--u14-theme-pack-contract-implementation-pending)
<a id="2026-10-02--keyboardmodalhelp-tip-alpha-follow-up-offline-candidate"></a>
- [2026-10-02 — Keyboard/modal/help-tip alpha follow-up (offline candidate)](history/implementation/2026-W40.md#2026-10-02--keyboardmodalhelp-tip-alpha-follow-up-offline-candidate)
<a id="2026-10-02--kl-82-authoritative-monitor-context-candidate-offline"></a>
- [2026-10-02 — KL-82 authoritative monitor context candidate (offline)](history/implementation/2026-W40.md#2026-10-02--kl-82-authoritative-monitor-context-candidate-offline)
<a id="2026-10-02--ui-owned-alpha-readiness-follow-ups-offline"></a>
- [2026-10-02 — UI-owned alpha-readiness follow-ups (offline)](history/implementation/2026-W40.md#2026-10-02--ui-owned-alpha-readiness-follow-ups-offline)
<a id="2026-10-02--ar05-corrected-candidate-acceptance-offline-0643-cest"></a>
- [2026-10-02 — AR05 corrected-candidate acceptance (offline; 06:43 CEST)](history/implementation/2026-W40.md#2026-10-02--ar05-corrected-candidate-acceptance-offline-0643-cest)
<a id="2026-10-02--ar05-master-evidence-and-report-boundary-checkpoint-offline"></a>
- [2026-10-02 — AR05 master evidence and report-boundary checkpoint (offline)](history/implementation/2026-W40.md#2026-10-02--ar05-master-evidence-and-report-boundary-checkpoint-offline)
<a id="2026-10-02--ar04-storage-fallback-publication-offline-utc"></a>
- [2026-10-02 — AR04 storage fallback publication (offline; UTC)](history/implementation/2026-W40.md#2026-10-02--ar04-storage-fallback-publication-offline-utc)
<a id="2026-10-01--ar03-enforcement-audit-ar02-publication-receipt-utc"></a>
- [2026-10-01 — AR03 enforcement audit; AR02 publication receipt (UTC)](history/implementation/2026-W40.md#2026-10-01--ar03-enforcement-audit-ar02-publication-receipt-utc)
<a id="2026-10-01--ar02-general-id-exhaustion-offline-utc-receipt"></a>
- [2026-10-01 — AR02 general ID exhaustion (offline; UTC receipt)](history/implementation/2026-W40.md#2026-10-01--ar02-general-id-exhaustion-offline-utc-receipt)
<a id="2026-10-01--ar01-runtime-gate-target-and-coverage-offline"></a>
- [2026-10-01 — AR01 runtime gate target and coverage (offline)](history/implementation/2026-W40.md#2026-10-01--ar01-runtime-gate-target-and-coverage-offline)
<a id="2026-10-01--ar00-alpha-queue-and-decision-provenance-offline"></a>
- [2026-10-01 — AR00 alpha queue and decision provenance (offline)](history/implementation/2026-W40.md#2026-10-01--ar00-alpha-queue-and-decision-provenance-offline)
<a id="2026-10-01--pre-write-backup-directory-chains-offline"></a>
- [2026-10-01 — Pre-write backup directory chains (offline)](history/implementation/2026-W40.md#2026-10-01--pre-write-backup-directory-chains-offline)
<a id="2026-10-01--u13-closed-independent-review-fixes-offline"></a>
- [2026-10-01 — U13 closed: independent-review fixes (offline)](history/implementation/2026-W40.md#2026-10-01--u13-closed-independent-review-fixes-offline)
<a id="2026-10-01--download-worker-outcome-and-tunnel-reservation-offline"></a>
- [2026-10-01 — Download worker outcome and tunnel reservation (offline)](history/implementation/2026-W40.md#2026-10-01--download-worker-outcome-and-tunnel-reservation-offline)
<a id="2026-10-01--failed-memory-download-closes-its-management-connection"></a>
- [2026-10-01 — Failed memory download closes its management connection](history/implementation/2026-W40.md#2026-10-01--failed-memory-download-closes-its-management-connection)
<a id="2026-10-01--restore-refuses-incomplete-load-state-coverage-offline"></a>
- [2026-10-01 — Restore refuses incomplete load-state coverage (offline)](history/implementation/2026-W40.md#2026-10-01--restore-refuses-incomplete-load-state-coverage-offline)
<a id="2026-10-01--active-limitations-and-task-backlog-reconciled-docs-only"></a>
- [2026-10-01 — Active limitations and task backlog reconciled (docs only)](history/implementation/2026-W40.md#2026-10-01--active-limitations-and-task-backlog-reconciled-docs-only)
<a id="2026-10-01--k6-alternate-candidate-bounded-read-only-presence"></a>
- [2026-10-01 — K6 alternate candidate: bounded read-only presence](history/implementation/2026-W40.md#2026-10-01--k6-alternate-candidate-bounded-read-only-presence)
<a id="2026-10-01--u13-independent-review-chosen-issue-12-evidence-reconciled-for-review"></a>
- [2026-10-01 — U13 independent review chosen, ISSUE-12 evidence reconciled for review](history/implementation/2026-W40.md#2026-10-01--u13-independent-review-chosen-issue-12-evidence-reconciled-for-review)
<a id="2026-10-01--experimental-k6-decision-k7-offline-interruption-regression"></a>
- [2026-10-01 — Experimental K6 decision; K7 offline interruption regression](history/implementation/2026-W40.md#2026-10-01--experimental-k6-decision-k7-offline-interruption-regression)
<a id="2026-10-01--k6-recovery-gate-availability-in-the-web-tab-published"></a>
- [2026-10-01 — K6 recovery-gate availability in the Web tab (published)](history/implementation/2026-W40.md#2026-10-01--k6-recovery-gate-availability-in-the-web-tab-published)
<a id="2026-10-01--issue-04-completion-evidence-and-last-save-locale-correction-published"></a>
- [2026-10-01 — ISSUE-04 completion evidence and last-save locale correction (published)](history/implementation/2026-W40.md#2026-10-01--issue-04-completion-evidence-and-last-save-locale-correction-published)
<a id="2026-09-30--serial-address-writes-fail-closed-pending-durable-recovery-adr-0057"></a>
- [2026-09-30 — Serial-address writes fail closed pending durable recovery (ADR-0057)](history/implementation/2026-W40.md#2026-09-30--serial-address-writes-fail-closed-pending-durable-recovery-adr-0057)
<a id="2026-09-30--phase-aware-service-control-write-activity-adr-0056"></a>
- [2026-09-30 — Phase-aware service-control write activity (ADR-0056)](history/implementation/2026-W40.md#2026-09-30--phase-aware-service-control-write-activity-adr-0056)
<a id="2026-10-01--issue-06-ground-root-site-affordance-published"></a>
- [2026-10-01 — ISSUE-06 Ground-root site affordance (published)](history/implementation/2026-W40.md#2026-10-01--issue-06-ground-root-site-affordance-published)
<a id="2026-09-30--write-activity-evidence-contract-adr-0056"></a>
- [2026-09-30 — Write activity evidence contract (ADR-0056)](history/implementation/2026-W40.md#2026-09-30--write-activity-evidence-contract-adr-0056)
<a id="2026-09-30--opt-in-service-control-read-activity-only"></a>
- [2026-09-30 — Opt-in service-control read activity only](history/implementation/2026-W40.md#2026-09-30--opt-in-service-control-read-activity-only)
<a id="2026-09-30--serial-lookup-joins-the-observed-one-shot-actions"></a>
- [2026-09-30 — Serial lookup joins the observed one-shot actions](history/implementation/2026-W40.md#2026-09-30--serial-lookup-joins-the-observed-one-shot-actions)
<a id="2026-09-30--first-observed-one-shot-action-read-only-device-compare"></a>
- [2026-09-30 — First observed one-shot action: read-only device compare](history/implementation/2026-W40.md#2026-09-30--first-observed-one-shot-action-read-only-device-compare)
<a id="2026-09-30--adr-0051-debug-service-control-ui-scoped-property-action"></a>
- [2026-09-30 — ADR-0051 Debug service-control UI (scoped property action)](history/implementation/2026-W40.md#2026-09-30--adr-0051-debug-service-control-ui-scoped-property-action)
<a id="2026-09-30--partial-read-only-bus-activity-snapshot-adr-0055"></a>
- [2026-09-30 — Partial read-only bus-activity snapshot (ADR-0055)](history/implementation/2026-W40.md#2026-09-30--partial-read-only-bus-activity-snapshot-adr-0055)
<a id="2026-09-30--u12-device-checks-offline-readiness-and-explicit-read-only-comparison"></a>
- [2026-09-30 — U12 device checks: offline readiness and explicit read-only comparison](history/implementation/2026-W40.md#2026-09-30--u12-device-checks-offline-readiness-and-explicit-read-only-comparison)
<a id="2026-09-30--k12-property-specific-recovery-gate-before-service-control-writes"></a>
- [2026-09-30 — K12 property-specific recovery gate before service-control writes](history/implementation/2026-W40.md#2026-09-30--k12-property-specific-recovery-gate-before-service-control-writes)
<a id="2026-09-30--u12--147-received-control-fields-in-the-web-monitor"></a>
- [2026-09-30 — U12 / §147 received control fields in the web monitor](history/implementation/2026-W40.md#2026-09-30--u12--147-received-control-fields-in-the-web-monitor)
<a id="2026-09-30--u12--adr-0051-debug-ui-paused-at-the-safety-boundary"></a>
- [2026-09-30 — U12 / ADR-0051 Debug UI paused at the safety boundary](history/implementation/2026-W40.md#2026-09-30--u12--adr-0051-debug-ui-paused-at-the-safety-boundary)
<a id="2026-09-30--commissioning-uistatus-inventory-and-handover-no-feature-shipped"></a>
- [2026-09-30 — Commissioning UI/status inventory and handover (no feature shipped)](history/implementation/2026-W40.md#2026-09-30--commissioning-uistatus-inventory-and-handover-no-feature-shipped)
<a id="2026-09-30--u12--146-channel-labels-ui-follow-up"></a>
- [2026-09-30 — U12 / §146 channel labels (UI follow-up)](history/implementation/2026-W40.md#2026-09-30--u12--146-channel-labels-ui-follow-up)
<a id="2026-09-30--contributor-license-agreement-removed-again-adr-0054"></a>
- [2026-09-30 — Contributor License Agreement removed again (ADR-0054)](history/implementation/2026-W40.md#2026-09-30--contributor-license-agreement-removed-again-adr-0054)
<a id="2026-09-30--contributor-license-agreement-adr-0053"></a>
- [2026-09-30 — Contributor License Agreement (ADR-0053)](history/implementation/2026-W40.md#2026-09-30--contributor-license-agreement-adr-0053)
<a id="2026-09-30--partial-download-of-parameters-and-group-addresses-verified-on-1167"></a>
- [2026-09-30 — Partial download of parameters and group addresses verified on `1.1.67`](history/implementation/2026-W40.md#2026-09-30--partial-download-of-parameters-and-group-addresses-verified-on-1167)
<a id="2026-10-01--k6-public-button-programming-paths-require-durable-recovery"></a> <a id="older-entries"></a>
- [2026-10-01 — K6 public button-programming paths require durable recovery](history/implementation/2026-W40.md#2026-10-01--k6-public-button-programming-paths-require-durable-recovery)
