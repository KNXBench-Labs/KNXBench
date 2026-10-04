# IMPLEMENTATION_STATUS.md

## 2026-10-04 — AR14D D4: RESEARCH split by topic

- `docs/RESEARCH.md` (6,759 lines) is now an index of 181 lines: the evidence
  tags, a table that maps every section to its file, §1 and §9–§12, and the
  sources. The rest moved verbatim to five topic files under `docs/research/`:
  project format (§2, §3, §5–§7), product data (§4, §18, scheme 23),
  KNXnet/IP and bus (§8), commissioning (§19, §22–§24 and the download-coverage
  entries) and features/UI (§13–§17, §20, §21, §25, U5/U6).
- Section numbers stay global, so every textual `RESEARCH §N` reference still
  holds. New findings go into the topic file; §26 is next.
- 14 inbound anchor links rewritten; check-anchors 433 links / 270 files, none
  dead. A script compared the old file with the union of the new ones: no
  original line is missing; only link targets changed. Documentation only.

## 2026-10-04 — AR14D D3: `xtask check-ledger` guards the single status record

- New repository gate `cargo run -p xtask -- check-ledger`
  (`xtask/src/ledger.rs`, ADR-0076), wired into CI next to the anchor gate and
  listed in [VERIFICATION](VERIFICATION.md) and the contributing guide. It
  fails on: a duplicate ID, an unknown status/owner/priority word, a row
  without seven cells, a snapshot table that is not exactly 180 rows, a count
  line that does not match the rows (it prints the expected line), a `KL-n`
  without a `KNOWN_LIMITATIONS` heading, and any table row in `docs/` or the
  root Markdown that pairs a ledger ID with a status word. `docs/archive/` and
  `docs/history/` are exempt; prose that mentions a status word is not a row.
- Evidence: ten unit tests written first against a stub (9 RED, the exemption
  case trivially green), then GREEN; 7/7 behavioural mutants caught; a
  real-repository negative control failed with both seeded problems named.
  xtask 85 + 12 tests, strict Clippy and fmt green; headers 463/157; the real
  ledger passes with 185 rows. The workspace product suite was not run: no
  crate other than `xtask` changed.

## 2026-10-04 — AR14D D2: one source-ID ledger instead of six status tables

- [docs/status/LEDGER.md](status/LEDGER.md) is the status of record for the
  180 snapshot IDs and the five post-snapshot IDs
  ([ADR-0076](adr/0076-one-ledger-is-the-status-of-record.md)). Columns: ID,
  priority, owner, route, status, owner disposition, evidence. Counts are part
  of the file.
- Moved there: the routing table and owner checkpoint tables of
  `alpha-release-goal.md` §7–§8, the per-ID and post-snapshot tables of
  `ALPHA_READINESS` with their count lines, and the priority/disposition
  columns of `COMMISSIONING_ALPHA_LEDGER`. The evidence documents keep their
  evidence and link to the ledger. A script compared the old files with the
  ledger and found every moved cell present.
- Reconciliation: 24 rows differed between the goal and `ALPHA_READINESS`;
  owner checkpoints won where they had written the goal table. Six statuses
  were corrected on evidence: `DATA-02`, `KL-42` (AR04 published `216c673e`),
  `KL-149`, `KL-150`, `KL-152` (AR06P delivered) to `DONE`, `KL-151` to
  `IN_PROGRESS`. Three stale routes (`MODEL-01`, `MODEL-02`, `UX-01`) point to
  the `goal-ui.md` owner again. All cases are listed in the ledger.
- AR06 is `DONE_SCOPED` as a package but mapped none of its eight rows; they
  stay `TODO` until their owner records per-ID dispositions.
- check-anchors 400 links / 265 files, none dead. Documentation only.

## 2026-10-04 — AR14D D1: status documents slimmed, consolidation planned before AR15

- Retired and dated documents moved verbatim to [docs/archive](archive/README.md):
  `goal.md`, `OFFENE_PUNKTE.md`, `PROJECT_ANALYSIS_2026-09-15.md`. This log
  keeps everything since the last October entry; the 12.5k-line September tail
  moved verbatim to [docs/history](history/IMPLEMENTATION_STATUS_2026-09.md).
  The handover `.ai/CURRENT_STATE.md` keeps its newest entries (77 at the
  cut); the 470 older ones moved verbatim to `.ai/archive/`.
- Only relative links changed, two cross-file anchors were re-pointed;
  check-anchors 393 links / 262 files, none dead. A plain-file link check finds
  no new dead link (17 pre-existing ones in old plans and `../CLA.md` are
  unchanged and not part of this package).
- The harder steps are planned as package AR14D D2–D5 in
  [alpha-release-goal.md](../alpha-release-goal.md#ar14d--consolidate-status-tracking-before-ar15)
  before AR15: one source-ID ledger with an ADR and an `xtask` check, a topic
  split of `RESEARCH.md`, resolved `KNOWN_LIMITATIONS` bodies moved to history
  behind stable stubs. They run under a new status-docs lock.
- Documentation only; no code, test or behaviour change.

## 2026-10-04 — MODEL-04 web half: catalog address allocation and unique names

- The catalog dialog has an *Options* group with two unchecked checkboxes:
  **Assign free addresses on the line** and **Keep names unique**. Address
  allocation is disabled, with an explanation, when no target line is
  selected. The client sends `allocateAddresses` / `uniqueNames` only when
  they are `true`, so a request without them keeps its pre-MODEL-04 body and
  replay fingerprint. A safe retry (DATA-03) resends the options unchanged.
- The preview states the allocation rule (.0, used and excluded addresses
  skipped; too few free means nothing is created) and that existing names are
  skipped. After a batch, every created device lists its allocated address.
  The server's refusal (`line 1.1 has 2 free device addresses, 3 requested`)
  appears as an ordinary error without a retry offer.
- Evidence: 4 new `CatalogBrowser.test.tsx` cases and 1 new `api.test.ts` case,
  written RED first. 4 intercepted Chromium cases in
  `e2e/catalog-allocation.e2e.ts` (en/de) all fail against the old client.
  4 guard mutants are caught. The test file's `beforeEach` now resets the
  `createDevice`/`currentProject` mocks, because queued `…Once` answers from a
  test that stopped early leaked into later tests.

## 2026-10-04 — DATA-03 web half: a lost catalog batch can be retried safely

- Each catalog submit now sends one `requestId` (ADR-0069). When the response
  is lost or the server answers 5xx, the batch counts as unconfirmed for any
  quantity, including one device. **Retry safely** resends the identical
  request after `GET /api/project` confirms the same `server_incarnation`, and
  a `replayed: true` answer counts as success. A restarted server, an unknown
  incarnation or a server that cannot be reached gets no new request: the user
  is told to inspect the project, or the retry stays offered until the server
  answers. No request with a new id is ever sent for an unconfirmed batch.
- Evidence: six new `CatalogBrowser.test.tsx` cases, five of them written RED
  first (the sixth guards the unchanged fail-closed case); four
  intercepted Chromium cases in `e2e/catalog-retry.e2e.ts` (en/de, network
  loss then replay, and restart). All four fail against the previous
  component. Four guard mutants are caught: retry despite a restart, a new id
  on retry, a retry without known identity, a single device not treated as
  unconfirmed.

## 2026-10-04 — MODEL-03 web half: device number 0 for evidenced couplers

- The individual-address editor no longer refuses device number `0` itself. It
  submits `.0`, and the server decides: an evidenced coupler (product database
  `Hardware/@IsCoupler="true"`) gets it through `SetCouplerIndividualAddress`
  with undo; anything else is refused, and the refusal text is shown in the
  field, which restores its previous value. The unused client-side refusal
  message is gone from both catalogues.
- Evidence: two new `Inspector.test.tsx` cases written RED first; four new
  intercepted Chromium cases in `e2e/coupler-address.e2e.ts` (en/de, accepted
  and refused), with the existing device-editor browser cases still passing;
  three behavioural mutants caught (client block restored, generic error text,
  field not restored). Backend tests from UA1 are unchanged.

## 2026-10-04 — Bounded History Web closure

Actual merged2057f86b is publicly accepted9/9: ordinary workspace3145/0/177
across169 result blocks, Web1761/100 files, intercepted Chromium8,50 isolated
behavioral guard controls, build/typecheck/strict-Clippy/fmt and four nonempty
policy gates. The [delivery receipt](evidence/commission-history-web-delivery-offline-2026-10-04.json)
retains exact source binding and scope. The separate private68/0/0 sweep stays
bound to60d6a85f, with an unchanged selected-package source delta; it is not
retagged as a new2057f86b run. Counts are not added across overlapping snapshots.

History Web reservation is released by this closure; Git publication/readback
is recorded separately in the handover. Caller production remains local WIP
(Shared-App7, CLI admission2, Service-Control8 passing); CLI download/restore,
broader sessions/clients and offline recovery remain open. No whole-goal,
independent-review, native-package or hardware approval is implied.


## 2026-10-04 — Bounded History Web adoption accepted offline

Current integrated code9fe69116, upstream4bc90aab (including public topology
repair), passed9/9 required stages on frozen1037 public/279 Web inputs.
Workspace3079/0/176 over160 result blocks; ignored176 were not executed.
Vitest1761/100 files, intercepted Chromium8, production build/strict Clippy/fmt
and all four intended-root repository checks passed. Fifty compiled guard
mutants failed at their named semantic assertions in isolated public copies;
canonical source never mutated. Separate tsc error-control detected/restored.
See [acceptance receipt](evidence/commission-history-web-offline-2026-10-04.json).
In-session review has no blocking finding, not independent approval. Exact14
Web paths remain byte-identical to the earlier parked candidate; dependency
versions unchanged except program alpha4. Earlier failed browser/import-graph
and header attempts remain rejected historical evidence, not erased receipts.

The shared authenticated GET path exposes independent device/restart/cleanup/
backup/intent meaning, strict whole-page admission, start-order cursor paging,
refresh-from-start and stale-response refusal. No write/retry/restore control,
raw private error, payload, host path or new bus contact follows. Publication
and Web-reservation release are recorded separately in the handover. Broader
caller/client and recovery/abort/restore software remains open; Global
Commissioning stays partial and owning Alpha IDsIN_PROGRESS. Excluded external
hardware/power-loss/vendor/ETS experiments remain user notices, not queued work.


## 2026-10-04 — Commissioning History Web candidate, not whole-track acceptance

- Published Web reservation7234dd00 remains owned by the commissioning session.
  Local History tab uses the existing diagnostics parent/authenticated GET and
  strict payload-free format2 admission, with EN/DE evidence and scope notices.
- Earlier local candidate: Chromium8/0/0, Web1751 tests in100 files, production
  build0. Initial browser attribution and transitive-import tripwire failures
  are retained; they are not retroactively green. Network-free validation adds
  no companion API call or project-mutating path.
- Current review fixes refuse calendar/hour normalization and classify invalid
  response JSON without exposing parser details. Semantic REDs are retained;
  read/property-write and whole-page refusal matrices extend coverage.
- Renewed pre-header-fix candidate: Chromium8, Web1755/100 files, ordinary
  workspace3028/0/176, fmt/Clippy/layering0; header gate refused two overlong
  new test headers, and anchors/corpus-policy did not start. Corrected headers;
  latest focused35/4 files and tsc0 include strict-effect stale success/error/
  loading and cross-page identity probes. Web alpha4 manifest/lock change only
  the application version, not dependencies (ADR0018).
- Renewed candidate/guard/workspace gates, integration and publication remain
  pending. See [History contract](COMMISSIONING_ACTIVITY_HISTORY.md#web-history-candidate--2026-10-04).
  Broader callers/long sessions and offline recovery remain separate open work;
  excluded hardware/power-loss/vendor/ETS experiments remain user notices.

## 2026-10-04 — Desktop shell recovers a terminated web process (§133)

- **§133, terminated web process.** `apps/knx-desktop/src-tauri/src/web_process.rs`
  observes WebKit's `web-process-terminated`, reloads the page (the unsaved
  project lives in the embedded server, so nothing is lost) at most three
  times in 60 s, then answers the user's explicit close itself: it closes when
  nothing is unsaved and asks a native GTK question first when the server
  still holds edits (`AppState::has_unsaved_changes`, the same predicate as the
  published `is_modified`). Eight shell tests and one server test were written
  RED first; nine guard mutants each fail a named test. A native run of the
  real binary in a loopback-only namespace reproduced the original bug on the
  baseline and showed the fix reloading, closing cleanly, and asking (once,
  again after dismissal) before discarding unsaved edits.
  `webkit2gtk` becomes a direct Linux-only dependency of `knx-desktop`, at the
  version already resolved through `wry` (one new lockfile edge, no new crate).
  A web process that hangs without terminating remains open (§133). The user
  placed dead-WebView evidence outside the Alpha scope on 2026-10-04; this
  narrows that accepted boundary and is not an Alpha requirement.

## 2026-10-04 — AR13: privacy, authentication and provenance (Claude session)

- Debug report: `report.md` (also the GitHub issue body) names every class
  that survives redaction and says `bus-telegrams.json` keeps values (text
  included) and timestamps. One synthetic fixture per class through every
  input channel pins the redaction (KL-106).
- Auth: the guard test now covers every declared route (97 method/path
  pairs) instead of seven samples (KL-22).
- New `crates/knx-build-stamp` replaces both identical build scripts;
  `KNX_REQUIRE_CLEAN_TREE=1` makes a release build refuse a modified or
  unconfirmed tree (KL-65, ADR-0018 amendment).

## 2026-10-04 — AR14: offline bus/CLI contracts (Claude session)

- CLI `bus monitor`/`route-monitor`/`bus write` honour the `--project`
  group-address style (three-level only without a project, and for
  `route-send`). New `knx_core::resolve_project_group_address_names` lists
  every installation's distinct name for a raw address; CLI and server
  Group Monitor share it instead of last-read-wins (KL-29, KL-62 item 13).
- `RoutingClient` clears `IP_MULTICAST_ALL` on Linux: a default-group
  client no longer receives a custom group's telegrams from another client
  on the same host (KL-31; measured over loopback before and after).
- New pinning tests for scan identity (DD0 only), negative-confirm window,
  timeout trade-off, no `Indeterminate` retry, four reconciliation refusals,
  and the three-level monitor→write round trip. 16/16 mutants caught.
- Ledger: KL-29 `DONE`, KL-31 `BLOCKED_EXTERNAL` (real custom-group run),
  ten rows `ACCEPTED_BOUNDARY`; dossier in ALPHA_READINESS.

## 2026-10-04 — UA10: CSV group-address exchange per installation (MODEL-01)

- `knx-csv`: `plan_import_into(project, parsed, Option<InstallationId>)` and
  `export_group_addresses_from(…) -> Result<_, UnknownInstallation>`; the old
  functions delegate with `None` (first installation). Rows match, ranges
  resolve and range-less creates land only in the chosen installation.
- Server: optional `installationId` on `POST /api/group-addresses/csv-import`
  and `/csv-export`; unknown installation → 400, nothing written. The
  destructive-preview confirmation token now also binds the installation.
- Test `apps/knx-server/tests/csv_installation_scope.rs`: RED 0/4 (the
  server silently ignored `installationId` and imported into the first
  installation), GREEN 4/4, 5/5 mutants caught.
- CLI: `--installation <id>` on `knx ga-export`/`ga-import`, token bound to
  it (unchanged without the flag). `apps/knx-cli/tests/cli_ga_csv_installations.rs`:
  RED 3/4 (`unknown flag`), GREEN 4/4, 2/2 mutants. The web CSV buttons still
  use the first installation (web half, handed over).

## 2026-10-04 — UA7/UA8: no silent loss between import, memory and `.knxdb`

- [ADR-0073](adr/0073-imported-elements-keep-their-own-ids.md) (`knx-etsproj`):
  elements with a repeated ETS `@Id` keep distinct internal ids
  (`id_table::IdTable`); references to a repeated id are reported as
  `MapProblemDetail::AmbiguousReference`; schema ≥21 short `Links` ids
  resolve within the device's own installation. Previously both duplicates
  shared one id (one lost on save), and a device could be linked silently to
  another installation's group address.
- [ADR-0074](adr/0074-native-save-is-exact-or-refused.md) (`knx-store`):
  `representable::check_representable` runs before every save; duplicate ids,
  orphaned lines, foreign line references, parent/child mismatches and a
  device twice in one part are refused with `StoreError::Unrepresentable`.
  A probe showed seven of these states saving "successfully" and reopening
  different.
- Tests: `crates/knx-store/tests/lossless_save.rs` (6),
  `crates/knx-etsproj/tests/links_installation_scope.rs` (2), a new case in
  `crates/knx-etsproj/tests/malformed_input.rs`, `id_table` unit test;
  10/10 guard mutants caught. Corpus import output unchanged (no repeated ids,
  one installation each).
- Corpus save/reopen equality now covers all three reference projects
  (schemas 11, 21, 23; previously ETS4 only, from when schema 23 was refused):
  `knx-store/tests/reference_project.rs`, 3/3 green with `--ignored`.

## AR06T exact23 bounded import — delivered (2026-10-04, `aadd8820`)

Local candidate on `9d719a4f`: exact product namespace23 admission with the
existing scheme21 strict member-namespace/qualified-attribute boundary and
package-scoped opaque-field evidence. No schema/budget/runtime/UI change.
Independently reconciled public evidence: six native GREENs, ProductDB
637 passed/0 failed/25 ignored (29 blocks), strict Clippy and two real CLI builds.
Real private2: two atomic namespace refusals become two retained/queryable
installs; exact opaque reporting/reopen replay and original853 rehash verified.
The actual Release CLI Full853 comparison completed on this frozen Source713
candidate: 690 existing installs have equal table contents, 161 refusals remain
diagnostically equal and atomic, and two exact23 packages become installs
(baseline 690/candidate 692). All 1382 installed archive copies were checked
byte-exact; all 853 originals were rehashed afterward. Aggregate receipt,
public/private prerequisite commitments and both release-binary hashes were
reconciled without reopening the private corpus. No raw/item records persisted.
Caller acceptance now adds four source-bound baseline REDs and four fresh-target
CLI/HTTP GREENs: measured opaque facts, exact archive/member retention, replay,
wire privacy/catalog discovery and seeded-database refusal integrity. Current
native six remain GREEN with scanner-owned master evidence and exact path/count
checks. Six compiled behavioral controls cover namespace/qualified guards,
master evidence, member late-depth admission, unresearched24 and generic scope.
Earlier parser-owned-field survivors and verifier refusals are retained; member
scan wiring is proved by the real late-depth test, not field presence alone.
Before integration, only three test files differed from Full853 Source713.
Current upstream adds a read-only coupler-query helper and its test; all other
productdb source files and the whole CLI entrypoint are byte-identical, and the
helper is only called by the owner's server domain path, not import/ingest.
The original Full853 remains its producer-bound private result, not a new
current-source private matrix. Current Source770 on `a346fa30` has actual public
22-stage acceptance: 161 Rust blocks, 3089 passed/0 failed/176 ignored;
ProductDB638/0/25; Web1739; Chromium82; strict Clippy/build/dependency/docs gates;
17 freshly generated binding pairs byte-identical to the previously controlled
closed lexical proof (not a new generic TS parser or rerun of its controls).
The first broad attempt is a retained zero-stage namespace-proof PermissionError;
separate correctly parent-bound isolated retry passed. Separate integrated
in-session review has no blocking product finding, not independent-model approval.
Published as `aadd88204de154cfcf5c1638310831a0a316dd86`: live/fetched refs and17 owned blobs exact. Final actual integrated10 repeats Rust3089/0/176,Web1739,Chromium82 after preserving a story-only owner update; all10 commands0. Its post-stage old-binary hash assertion remains rejected: CLI build.rs correctly stamps the new Git HEAD. Current archived release version/hash independently verified. Exact23 is bounded
import/storage/report/replay support, not full manufacturer or bus/runtime
compatibility. Scheme10, KL153 and the Alpha goal remain open. Decision:
`adr/0072-product-scheme23-namespace-gate.md`.
See `PRODUCT_SCHEME_23_RESEARCH.md` for scope and refused-verifier provenance.

## 2026-10-04 — UA5: explicit topology repair (MODEL-02, core/store/server half)

- [ADR-0071](adr/0071-ambiguous-topology-is-repaired-explicitly.md):
  `Command::RepairDevicePlacement { device, keep: DevicePlacementSlot }` and
  `Command::RepairLineOwner { line, keep }` keep the named existing placement
  and remove every other occurrence, in one undo step with exact-order undo
  (`RestoreDevicePlacements` / `RestoreLineOwners`). Refused when nothing is
  ambiguous, when the kept slot is not current, or across installations.
- **Data-integrity fix:** `.knxdb` save silently collapsed a multiply placed
  device or multiply owned line to the last written placement (schema holds
  one). Save now refuses with `StoreError::AmbiguousTopology` before writing.
- Server: `POST /api/repair/device-placement`, `POST /api/repair/line-owner`.
- Tests: `crates/knx-core/tests/topology_repair.rs` (RED: did not compile —
  no repair API; GREEN 8/8), `apps/knx-server/tests/topology_repair_routes.rs`
  (2), two store tests in `crates/knx-store/tests/command_persistence.rs`
  (the first draft expected save/reopen to keep the ambiguity and exposed the
  lossy save). 10/10 guard mutants caught.
- Not covered: duplicate-id renumbering, building-part/group-range placement
  repair, web UI choice of the kept placement (Web lock). MODEL-02 stays
  `IN_PROGRESS` until the UI half lands.

## 2026-10-04 — UA4: every installation editable in the core (MODEL-01, core/server half)

- [ADR-0070](adr/0070-commands-act-in-the-owning-installation.md): `knx-core` commands no longer assume `installations[0]`: id-addressed
  commands resolve the owning installation (ambiguous ids refused), Delete/
  Restore pairs carry the installation, parameter rows are edited where they
  live (else in the device's installation), and root creates
  (`CreateArea`, `CreateGroupRange`, `CreateBuildingPart`, range-less
  `CreateGroupAddress`) take `installation: Option<InstallationId>` with the
  first installation as default. Nothing connects two installations:
  cross-installation device/line/part/range/link moves are refused with
  `CommandError::CrossInstallation`. New `Command::RenameInstallation`.
- Server: `PATCH /api/installations/{id}` and optional `installationId` on
  the four root create routes.
- Tests: `crates/knx-core/tests/multi_installation.rs` (RED: 5 of 7
  behaviour tests failed with first-installation `NotFound` errors before the
  change), `apps/knx-server/tests/multi_installation_routes.rs`; one older
  core test that pinned first-installation parameter semantics now pins the
  in-place edit. Eight guard mutants caught.
- CSV group-address import originally still created in the first installation
  (since UA10 the server routes take `installationId`); the web
  UI half (installation rename, choose installation for root creates) waits
  for the Web lock. MODEL-01 stays `IN_PROGRESS` until then.

## 2026-10-04 — UA3: opt-in address allocation and unique names (MODEL-04, server half)

- `POST /api/devices` accepts `allocateAddresses` (needs `lineId`) and
  `uniqueNames`, both default off. Allocation uses the new pure
  `knx_core::free_line_addresses` (lowest free octet 1–255, skips 0, every
  project address and the exclusion list; refuses ambiguous lines and short
  supply) and adds one `SetIndividualAddress` per device to the same batch, so
  the core validates every address and one undo removes everything. Items carry
  their `address`. Both flags are part of the DATA-03 replay fingerprint.
- Tests: `apps/knx-server/tests/catalog_allocation.rs` (RED 1/4 → GREEN 4/4),
  `allocation::tests` (5), `allocated_batches_map_both_children_of_an_item_to_that_item`.
  Eight guard mutants caught (one survivor found a missing two-area test, added).
- The web catalog toggles wait for the Web lock; MODEL-04 stays `IN_PROGRESS`.

## 2026-10-04 — UA2: catalog batch replay token (DATA-03, server half)

- `POST /api/devices` accepts an optional `requestId`; a committed ID with
  identical content replays its recorded outcome (`replayed: true`) without
  applying again, other content under the same ID is refused, failed requests
  are not recorded ([ADR-0069](adr/0069-catalog-batch-request-replay-token.md)).
- Tests: `apps/knx-server/tests/catalog_request_replay.rs` (6/6 RED before:
  the resend created a second batch; GREEN after) and the ledger unit tests in
  `catalog_requests.rs`. Five guard mutants caught.
- The web client half (send a per-action ID, offer a safe retry) waits for the
  Web lock; DATA-03 stays `IN_PROGRESS` until then.

## 2026-10-04 — UA1: coupler `.0` with manufacturer evidence; KL-127 closed as known gap

- MODEL-03 backend: a device whose product's hardware has `IsCoupler` true in
  the product database may take device octet 0 on its line
  (`Command::SetCouplerIndividualAddress`, `CouplerEvidence`,
  `knx_productdb::query::product_hardware_is_coupler`). Line prefix and
  uniqueness stay enforced; undo/redo round-trips. Everything else keeps the
  existing refusal. Evidence and tests: [RESEARCH §25](research/features-and-ui.md#25-ua1-coupler-0-evidence-and-siteground-samples-2026-10-04).
- The web editor half (offer `.0` for an evidenced coupler) waits for the Web
  lock; MODEL-03 stays `IN_PROGRESS` until then.
- KL-127: no independent `Ground` sample found; closed for the Alpha as a
  known gap on the user's instruction.

## 2026-10-04 — Commissioning validation scope and requested continuation

- User removed new real-hardware, power-loss, vendor and ETS validation from
  the commissioning completion goal. This is an accepted evidence boundary,
  not a hardware/compatibility/recovery claim or pending operator work.
- [User notices](manual/known-issues.md#commissioning-validation-boundary) now
  disclose it. Existing receipts and runtime safety/refusal gates are unchanged.
- User requested implementation of broader caller/long-session coverage,
  Web/client adoption and offline recovery/abort/restore contracts. These are
  pending implementation, not newly accepted tests. SAFE-03/AUDIT-01 remains
  partial; ADR0067 remains Proposed until software contracts and owner admission
  are verified. No new hardware operation is authorized.

## 2026-10-04 — Project-evolution story, first private version (companion, not product)

- New, independent `story/` companion ([README](../story/README.md),
  [ADR-0068](adr/0068-project-evolution-story-is-a-static-offline-companion.md)).
  No product code, KNX domain, project file or bus is touched; the engineering
  feature backlog is unchanged.
- Source-backed edition: 36 development steps on 8 strands, 46 typed relations,
  8 chapters, 6 disclosed gaps, baseline `origin/main` `75ad9650`. Earliest
  surviving prompt 2026-09-02 13:50 CEST; the strategy document's origin is not
  in any available source.
- Stdlib Python tool: schema validation, pattern privacy scan, private-provenance
  traceability and leak refusal, deterministic layout, immutable candidates with
  diff and review checklist, loopback-only preview server, exact-digest approval
  check, and a `publish` command that always refuses.
- Verified: 49 unit tests (with five guard mutations each caught), and 41
  Playwright/Chromium checks per candidate on 1440×900 and 390×844, including
  live cancellation of running growth by *Motion off* and by OS reduced motion,
  hostile-text rendering, no-JavaScript reading and zero CSP violations.
- Not done: publication, hosting, final fonts, non-Chromium browsers, real
  screen readers, cloud-session coverage.
- Follow-up the same day: the user reviewed `2026-10-04.2` without changes, and
  the built pages are now versioned in `story/previews/` (`build --preview`),
  guarded by a rebuild-equality test (53 unit tests).
- Story motion update (user request, site only, no content change): scrolling
  back retracts later steps and refocuses the current chapter; looping signal
  pulses travel each visible connection (paused off screen); random headline
  letters roll through in place (Web Animations API, accessible names kept).
  All of this stops with *Motion off* and reduced motion. Previews of `.1`–`.3`
  were rebuilt from unchanged candidates. Browser check: 46/46; two
  realistic mutants (no retreat; swaps ignoring motion) are each caught.
- Narrator edition `2026-10-04.3` (user request): the story is told by a gloomy
  AI narrator in homage to Marvin. Optional, schema-enforced `edition.narrator`
  (hero, aside label, mandatory disclosure); editions without it render
  byte-identically (`.1`/`.2` previews unchanged). Only narration and asides
  changed; one new step and two relations record the request. 60 unit tests,
  41/41 browser checks.

## AR06S KL153 scheme23 bounded research — 2026-10-04

No production code changes. Base575a2d1d,source712 frozen; official Schema23
v01.00.00 (2024-03-01) verified/hashed/cited,project-only manufacturer-semantic
boundary documented in PRODUCT_SCHEME_23_RESEARCH.md. Offline census853 hashes,
852 master XML/one BadZipFile refusal,2 complete scheme23 packages/8 XML (2
each Master/Catalog/Hardware/ApplicationProgram). No cross-document namespace
mismatch,foreign elements or qualified attributes observed;4 positive/3
negative controls. Fresh unchanged ProductDB631/0/25 in28 blocks,strict Clippy
and Release CLI passed. Real original-name CLI2 atomic namespace refusals,all
853 originals independently rehashed after probe,temp0/no raw or item records.
Namespace registry and typed/runtime compatibility unchanged; dedicated master
language23 evidence is not package admission. KL153/151/AR06P/AR07/Alpha remain
open. Research doc5/in-session/private-delta review GREEN; research published
as87df5d82a126384da904fc47277b0bd0101bf198 with exact live/fetched/eight
blobs. Closing bookkeeping/own hygiene remain separate,not namespace admission.

## AR06R KL-152 coupled-budget delivery — 2026-10-04

Original853 identity/source80a5500d/706 inputs verified. Four size-admitted
scheme14 packages/16 XML documents select two item refusals. Actual bounded
scratch Release/Release pair: baseline2 atomic refusals, observer2 installs;
max802433 items/155281510 estimated bytes, peak134552KiB RSS/max2.985s.
Archive bytes/original853/empty refusal tables verified; private temp0.
Rejected first verifier attempt stays rejected; source-derived archive-table
correction has three public controls. See PRODUCT_DATABASE_CORPUS.md for scope.
Candidate coupled ceilings1048576/256MiB preserve depth1024/ZIP/namespace limits,
all-or-nothing scan and full retained data. Two public registered REDs and six
new boundary/late-failure tests. Latest integrated7f57abbb public18 accepted:
Rust3028/0/176 in153 blocks,Web1739,Chromium82 inventory/pass; source712,
all18 logs and actual Release binary independently exact. Earlier ed03cb85
public18/private2/binding17 receipts keep their original producer identities.
Six subsequent owner Rust changes invalidated earlier440-input equivalence;
a fresh full853 Release/Release comparison on7f57abbb is now independently
reconciled:688 unchanged table-count installs,163 unchanged normalized refusals,
2 budget admissions→690 installs. All853 original hashes independently
rechecked descriptor-relatively/no-follow; retained archives/atomic refusals,
both binaries/source/logs verified,private temp0. No raw/item vectors persisted.
First latest18-command attempt had a rejected rebuild-verifier contract,not
a test failure:CLI does not depend on server; corrected guard and actual18
retry pass. Failed receipts remain separate. Docs-only commissioningbb62ae57
was preserved as20a3c4cd with source712 unchanged. Code and acceptance docs
published as2b2a267f7873137ccbf3d0a5052a541a76573d59; live/fetched refs and
all9 outgoing blobs exact. Four own runtime directories removed; closing
metadata/final two targets/runner/worktree cleanup are tracked separately.
KL152 scoped correction is delivered. No UI/DTO/schema,
bus, manufacturer runtime or ETS compatibility expansion. KL151/153 and Alpha
remain open; KL151 research closing80a5500d delivered/cleaned.

Broad attempt1 is RED on three retained-source classification regressions;
release CLI unstarted. Dedicated master-language work/input limits were
implicitly shared with the scheme scanner. Candidate separates them and keeps
their original64MiB/262144 values, including retained-source preflight. Failed
logs/inputs are preserved; corrected real-source and integrated gates now pass.

## Commissioning lifecycle: integrated offline implementation accepted — 2026-10-04

Candidate `a8c9342c` and upstream `75ad9650` are integrated and published as
`1c5dec07`, with exact fetched/live `main` readback. Same-merged-source scoped
**125/0/0** (Store94/OneShot18/ten offline workers/API3) and required ordinary
**3022 passed/0 failed/176 ignored** across153 blocks are independently
reconciled. The ordinary ignored scope is not accepted and overlaps scoped125;
History13 and the one25-case leaf are included, not additional tests. Web pinned
offline build/typecheck passed; Vitest1739 tests/98 files passed. All26 command
stages used unchanged source32/public Rust-Cargo369/tracked Web inputs and
actual dual leases, with initially fresh build target/unchanged private original
commitment/no skip/no private raw saved. The first worker's stale inner receipt
namespace refused before Cargo/input discovery; corrected17-phase continuation
and initial same-source positive3 are separately bound, not rewritten evidence.

Recovery-first/start-before-contact, intent/terminal monotonicity, independent
cleanup uncertainty, strict malformed receipt refusal and version evolution are
implemented at the documented bounded scope. Five semantic mutations below
remain pre-integration evidence; integrated runtime was actually renewed.
Review is in-session; broader owner admission/full SAFE-03/AUDIT-01 remains
`PARTIAL_BACKEND`, ADR0067 Proposed. Web/manual-client adoption, other callers,
hardware/device recovery/crash/power-loss/ETS evidence are not claimed. The
permanent [receipt](evidence/commission-download-lifecycle-offline-2026-10-04.json)
and [history contract](COMMISSIONING_ACTIVITY_HISTORY.md) preserve provenance.
Older entries below are historical snapshots, including their former pending
integration notices; they do not override this current scoped acceptance.

## Commissioning lifecycle candidate: offline closure — 2026-10-04

The local SAFE-03/AUDIT-01 candidate on `iaw-commission-download-lifecycle`
passed125 distinct registered tests/0 failed/0 ignored on the current declared
source32/HEAD: full Store94 (History13 included), OneShot18 (one25-case malformed
download matrix included), ten actual offline worker leaves and three public
history API leaves. All16 stages used actual outer-alpha/workspace leases;
worker/history API rebuilds and all-target server/store Clippy passed. The
ten workers retain the original unchanged input commitment, no skip and no raw
private output. The source32 scope is explicit, not every repository file;
reviewed Rust diffs and owned-path inventory are checked separately.

Five actual production-source controls compiled and failed at their intended
semantic assertions: repeated result, late intent, swallowed migration COMMIT
error, admitted unknown download fields, admitted zero session. Originals were
restored byte-for-byte in finally under the actual leases; the canonical public
Store94/OneShot18 bookend112 passed and is not added to the final-chain total.
The newly added matrix is test-only: malformed second rows refuse the whole
page without overwriting, latch unavailable and prevent a fresh download;
bytes/rows/identity/cursor/sidecar absence are checked after refusal.

This is scoped local offline closure, not integrated delivery or complete
SAFE-03/AUDIT-01. The established four-kind coverage declaration remains partial;
the bounded local download candidate does not erase its broader residue.
ADR-0067 remains Proposed. Review is in-session, not independent whole-goal
approval. Hardware/vendor/ETS compatibility, power-loss/crash restoration,
wider commissioning journalling, integrated acceptance and publication are not
claimed. Earlier sections below retain their historical source-scoped counts.
See `COMMISSIONING_ACTIVITY_HISTORY.md` and `KNOWN_LIMITATIONS.md`.

## Commissioning lifecycle storage candidate — 2026-10-03 08:14 CEST

SAFE-03/AUDIT-01 continues offline on `iaw-commission-download-lifecycle`.
Known history format1 is admitted read-only first and upgraded transactionally
to2, preserving opaque documents, identity keys, cursor gaps and AUTOINCREMENT.
SQLite EXTRA synchronization is configured after admission, before transactions;
this is storage policy, not universal power-loss or hardware recovery evidence.
The exact current storage source passed **91/0/0** library tests, including
nonempty old-schema retention, busy-write migration failure, foreign-schema
refusal through both openers, hot-journal and WAL preservation.

Proposed ADR0066 and the manual contract record this local candidate. The queued
metadata regression subsequently compiled and failed on the expected missing
reader support; its minimal parser passed13 focused tests. A second compiled
RED exposed prior-incarnation pending cleanup remaining falsely pending. The
corrected reader passed14/0/0, preserves witnessed device/restart results and
original stored documents while projecting interrupted cleanup as unknown.
The separate publisher RED compiled and failed on accepting missing download
evidence. That publisher snapshot passed the complete focused metadata/OneShotLog
group15/0/0 with an actual server rebuild before the subsequent caller changes. Existing
history HTTP admission/version/empty/refusal regressions pass3/0/0; this does
not test nonempty download serialization or worker receipts.
The initial actual caller tracer bullet compiled its expected missing-start RED,
then passed one selected offline HTTP simulator test with a server rebuild,
both shared leases and two unchanged original inputs. Its start row is durable
before fake tunnel acquisition, outside the volatile one-shot ring. This is
not complete worker journalling. Unproved intent/result/cleanup hooks were
removed before the next test-first terminal slice; the candidate currently
retained minimum start ownership and conservative drop before the next RED.
The nonempty joined-worker HTTP receipt regression subsequently compiled and
failed on the expected missing terminal record (0/1/0,15 filtered). Only then
were intent/result/cleanup hooks added in two production paths; test and
collector were unchanged; actual terminal GREEN now passes one selected offline
simulator HTTP test with a real server rebuild, both leases and unchanged inputs.
This is happy-path worker/receipt evidence only. Subsequent scoped regressions
passed OneShotLog15/0/0, worker8/0/0 and start-before-contact1/0/0 on that source.
A terminal-replacement preservation probe subsequently compiled and failed as
expected: a second result could overwrite the first. The minimum prior-state
guard now refuses before mutation without marking storage unavailable; its
complete OneShotLog GREEN passes16/0/0 and new-source terminal HTTP regression
passes1/0/0 with actual server rebuilds, both leases and unchanged private inputs.
The late-intent-after-terminal sibling probe compiled and failed on reopening
possible-send evidence after a failed/no outcome. Only afterward a prior-state
refusal was added before intent mutation/persistence; its complete OneShotLog,
worker and selected terminal HTTP GREEN/regressions passed17/0/0,8/0/0 and1/0/0
on that candidate, with exact source manifests and unchanged private inputs.
A test-only actual keeper/worker admission-refusal probe follows: synthetic
sidecar after durable start, retained backup, zero device-changing sends, cleanup,
unchanged metadata DB/marker and refusal of further starts. Its actual run is
not yet behaviorally measured: first attempt failed instrumentation compilation
before running tests. A missed second simulator constructor was corrected;
compiler-only check then separate fault/positive retries remain pending.
Production paths unchanged; no authentic WAL/crash/power-loss claim.
The next compiler-only stage passed; its named runtime probe failed because the
fixture installed only a synthetic marker with a still-admitted rollback header.
That rejected premise is retained separately; it does not establish keeper failure.
Corrected test-owned header2/2 plus marker now defines the intended refusal, with
post-injection bytes as preservation baseline. Actual corrected runtime pending;
rollback-header orphan-sidecar policy remains unresolved separate coverage.
Corrected actual runtime and positive default-disabled-control terminal test now
each passed1/0/0 with exact retained named outcomes, compiler-only fresh target,
both lease pairs and32 captured source/build-input paths. This proves the narrow
simulated admission-refusal keeper/cleanup behavior, not real WAL/crash/recovery.
Public synthetic orphan-sidecar refusal test actually compiled and failed on
admitting the first marker. Only afterward a minimum no-follow presence refusal
was added before sentinel/SQLite opening; whole-source delta verified. Complete
history/OneShotLog and actual-worker GREEN/regressions passed11/0/0,17/0/0 and
two named1/0/0 on the guard candidate, including24 regular-marker combinations
in one storage leaf. Next test-only Unix directory/live-link/dangling-link
coverage completed18 combinations; its actual12-leaf history run passed12/0/0
on its own32-path snapshot. The next test-only actual keeper backup-save-failure
scope must prove no device-changing sends, no false retained-backup/intent claim,
cleanup and a durable failed/no row. Its compiler passed, named runtime failed
without a retained public assertion location; failure evidence is preserved.
Diagnostic-only23 closed keys locate wrong test enum expectation:available instead
of actual configured. Preceding no-write/no-backup/cleanup checks passed; only test
literal corrected, production unchanged. Corrected full case and positive-worker
regression passed1/0/0 each after an actual rebuild on their own frozen32-path
snapshot, dual leases/originals unchanged/raw discarded. The test-owned backup
directory blocker proves zero mutations/no retained backup, failed/no worker,
one cleanup and successful history query with actual durable failed/no/backup=false/
intent=false/cleanup row. Configured alone is not health proof; not all keeper faults.
Metadata-error/path-race/remaining lifecycle fault and full release gates remain
open; no authentic WAL/crash/hardware or universal restoration claim.
Next scoped evidence captures exact selected public test/outcome before discarding
private raw output and freezes Cargo manifests/lock/config presence as well.
Earlier private receipts retain their narrower exact-command/result-block scope;
they are not retroactively named-execution or omitted-build-input proof.
The Proposed lifecycle decision is now ADR-0067; upstream ADR-0066 is another
owner's accepted decision and is not replaced by this package.
Intent/terminal/fault acceptance,
mutations, review and full integrated acceptance/publication remain **PENDING**.
Next test-only actual-worker cleanup returned-error/panic cases require persisted
result before disconnect, retained backup and stable witnessed result/timestamp/
identity/sequence independently of cleanup metadata. Default-disabled local probes,
exact named private scopes and closed diagnostic controls prepared; actual compiler/
runtime/default-control results pending, production unchanged. First compiler
passed but returned-error test failed on a fixture row/API projection mismatch:
sequence/incarnation were omitted from the captured document. Negative evidence
preserved; test snapshot corrected from actual row fields, comparisons retained,
corrected error/panic/default scopes passed1/0/0 each after actual rebuild on
their own frozen32-path snapshot/every dual lease/originals unchanged/raw discarded.
Actual worker results/backup/timestamp/identity/sequence survive adapter returned
error or panic; cleanup separately returnedError/unknown, reservation released,
one adapter cleanup and no history-query bus contact. Production unchanged,
initial fixture failure retained; storage-recording refusal/abort faults still open.
Next test-only cleanup-recording refusal injects synthetic unsupported header after
persisted device result, then returns adapter OK. New case requires preserved
worker result/backup, unavailable history/refused subsequent start before contact,
preserved injected baseline/no sidecars. Compiler/new case/error/panic/default
regressions passed1/0/0 each (4 actual leaves) after rebuild/frozen32/every dual lease/
originals unchanged/raw discarded on new source. Device result/backup preserved,
history unavailable/new write503 before contact/refused bytes exact/no sidecars;
production unchanged. Pending-cleanup interruption/terminal storage/midwrite faults
and broader gates remain open; no protocol or real-WAL/crash/restoration claim.
Next synchronous actual-worker pending-cleanup shutdown test is prepared with a
default-disabled adapter hold/Notify signal and fully owned current-thread runtime,
then separate observer. Result/backup/identity/timestamp preservation and unknown
cleanup/released reservation now passed1/0/0, alongside storage/error/panic/default
1/0/0 each (5 leaves) after actual rebuild on own frozen32/every dual lease/originals
unchanged/raw discarded. Real spawned worker yielded in adapter cleanup, full
runtime drop before observer; backup/result/timestamp/identity preserved, cleanup
unknown/reservation released/no extra frames. No public cancel API or crash proof;
terminal-recording refusal/midwrite interruption/mutants/full gates remain open.
Actual terminal-recording refusal test is now prepared: consumed synthetic metadata
fault after simulated Restart send, durable running intent before fault, actual
worker result/backup preserved/unavailable history/new-start refusal before contact/
refused bytes no sidecars. Compiler/new case and shutdown/cleanup-storage/error/
panic/default each1/0/0, Store12/0/0 and OneShot17/0/0 passed after actual integration
rebuild:35 leaves on own frozen32/every dual lease/originals unchanged/raw absent.
Actual terminal recorder refuses after restart, durable running intent observed
first; worker result/backup retained, history unavailable/new-start503 before contact/
refused bytes unchanged/no sidecars. Production unchanged, no protocol change or
claim that unavailable terminal receipt was persisted. Midwrite interruption,
negative mutants/version-fault closure/review/full integrated delivery remain open.
Midwrite interruption actual-worker regression is prepared with default-disabled
send-future hold after simulated MemoryWrite/durable running intent and full owned
runtime shutdown. It requires backup/intent retained, live failed/partially versus
unknown nullable durable extent/restart, cleanup unknown/zero adapter-return claims,
same identity and no added frames/retry/restore. Compiler/new case/six private
controls each1/0/0/public Store12/0/0/OneShot17/0/0 passed after actual integration
rebuild:36 leaves on own frozen32/every dual lease/originals unchanged/raw absent.
Actual dropped worker retains backup/intent, live failed/partially versus durable
unknown/null written/restart/unknown cleanup/same identity; zero adapter-return
claim and no added frames. Production unchanged by test-only extensions, no hard
kill/power-loss/rollback/restoration proof. All-target server/store Clippy next;
negative source mutants/version-fault closure/full review/integrated delivery open.
All-target server/store Clippy returned101 on two capability-heavy eight-argument
constructor/worker signatures. Only scoped too_many_arguments annotations/reason
comments added (all executable statement bytes exactly unchanged); no broad lint
disable or parameter-bag abstraction. Failed lint receipt preserved; new all-target
Clippy/compiler/seven worker/Store/OneShot regressions PENDING on frozen source.
Next lint attempt returned101/await_holding_lock in the header-refusal fixture;
later ten phases did not start. Existing explicit drop already preceded awaits;
the synchronous snapshot assertion now has lexical lock scope, all checks retained
and no lint suppression/production change. New all-target Clippy/compiler/eight
worker controls (including owning header-refusal case)/Store/OneShot PENDING.
That complete corrected source passed all-target server/store Clippy and actual
integration rebuild, eight real-worker cases plus Store12/OneShot17:37/0/0 on
frozen32/every dual lease/unchanged originals/raw absent. Both earlier lint failures
remain separate negative evidence, no full SAFE03 close. Next public synthetic
legacy-upgrade reader-blocked finalization test adds actual dirty-journal witness,
main/version/row/counter preservation and explicit-reader-release retry; production
unchanged. New compiler/focused/all-target lint/Store/OneShot results PENDING.
New case now passed with actual dirty-journal witness; compile0/all-target Clippy0,
Store13/OneShot17:30 distinct public tests0 failed, separately executed focused leaf
included rather than double counted. Exact frozen32/every dual lease/current source;
no historical private-worker borrowing. Three production-source negative guard/
commit-error controls planned, not executed; full SAFE03 close still pending.
All three controls now compiled and failed at the intended exact named public
semantic assertion. Original production bytes restored in finally under actual
dual leases; canonical Store13/OneShot17 bookend30/0/0. Fresh complete current-
source twelve-phase lint/build/eight-worker/public-suite chain PENDING, expected
38 distinct tests; no historical borrowing or full SAFE03/parent close.
That restored current-source chain passed actual all-target lint/integration build,
eight privacy-closed actual workers plus Store13/OneShot17:38 distinct/0 failed/
0 ignored, all32/chain/every dual lease/unchanged originals/raw absent. Three
public production controls killed/restored on same source. Complete branch review
and actual three-case public HTTP history version projection gate still pending;
PARTIAL_BACKEND, no integrated delivery/hardware/power-loss/full SAFE03 claim.
Three actual public HTTP history API cases now passed3/0/0 after actual build:
format2/partial/persistent/no path, bounds before creation, foreign/future3
unchanged. Same exact source32 current total41 distinct/0 failed/0 ignored.
Two additional current-source start-before-contact/backup-save-failure worker
scopes pending; full branch review/closure/integrated delivery still pending.
Those two actual selected workers now passed1/0/0 each with identical source32,
every actual lease/same unchanged original commitment/no raw retained. Current
43 distinct/0 failed/0 ignored accepted 2026-10-04, no historical borrowing.
Wider storage-library public unit gate/full review/closure/delivery pending.
No complete recovery, Web adoption, hardware/vendor/ETS or release claim.

Fresh upstream `c07e6403` changed only owner statistics and was fast-forwarded
into this owned checkout; no owned source overlap or ADR-number collision.
Later fresh upstream `5ca570a0` includes productdb scalar-copy admission and
its ADR0065; own unpublished lifecycle ADR was renumbered0066. No direct
commissioning-source overlap, but eventual integration must retest planning.
Root/statistics and the ui-theme-management/U17 Web surface remain untouched.
## CRT authorized integration/publication follow-up

- User authorized commit/main integration/push. Feature16c9d774688c2e7e9a0c0b6bf436c788a7ce79e1
  integrated onto upstream5540dcac as3d03aea5ee2905a98450ee193bdc1246d0f93c99.
  Complete upstream handover archive and both status prefixes preserved exactly.
- Actual merged-result gates: Web1739/98, build,12 production/16 reference browser
  groups, Clippy workspace/all-targets with warnings denied, Rust3009 passed,
  166 ignored across153 result blocks, fmt and4 repository gates. All executed
  stages exit0; ignored/private/hardware/native/Orca scopes remain unclaimed.
- Retained merged receipt/closure log and fresh native-frame screenshot. Source
  unchanged after the accepted merge; closure changes are documentation/evidence.
  Current refs and latest handover establish the final publication state.
- Shared dirty/stale root main remains deliberately unsynchronized; no foreign
  product/stats/research/UI-plan edits were staged or overwritten. Earlier local-
  only/no-commit/no-merge entries below describe the original delivery, not this
  authorized follow-up. Separate semantic-role/acceptance limitations remain.

## 2026-10-03 — Productive CRT interactions (local feature worktree)

- Implemented the user-requested animations in real App/ProjectExplorer/
  GroupAddressTable, not only the offline study. Independent motion style `crt`
  is admitted by both the actual pre-mount bootstrap and runtime registry.
- Standard: 250ms ease-out fill, inert clipped light, bounded activation glow.
  Subtle: existing 120ms fill only. Off/OS reduced motion cancel running feedback.
  A disposable per-workbench controller handles timers/observers and retirement;
  existing delegated keyboard, native selection and bulk-selection rail remain.
- Manual Save/Save As glows only on its existing request path, after cancellation/
  stale-snapshot checks; autosave is silent. The glow does not certify success.
  Browser refusal assertions verify error reporting; source review confirms the
  existing dirty-state/error path is unchanged, not a real persistence test.
- Reviewed final Web **1739 tests / 98 files**, TypeScript/Vite build and **12**
  actual-app Chromium groups pass. Receipt records zero page errors, unexpected
  requests and real backend requests; two Save requests are intercepted synthetic
  refusals. The native-frame screenshot has been visually inspected.
- Added controller/CSS/bootstrap regressions and a reproducible real-App browser
  verifier with a synthetic project and intercepted file-picker/open/settings/
  discovery/Save flow. No actual project/settings/backend/KNX write or new
  dependency. No WebKitGTK/Orca/full-WCAG or independent-review claim.
- In-session review removed redundant keyboard handlers in favor of the already
  delegated production implementations; transient feedback is presentation-only.
  Updated ADR-0022, CRT/theme guide, roadmap, limitations and handover/log.
- Worktree: /mnt/daten-i/Sourcecode/KNXBench.worktrees/crt-interactions-20261003,
  branch feat/crt-interactions-20261003, base 3337e4ef. Changes remain local and
  uncommitted; no feature push/main merge/root product synchronization. Native
  acceptance, exact selection/Save-only roles and broader component styling
  remain separate. Own test-runtime cleanup is recorded in the task handover.

## 2026-10-03 — CRT 1.1 branch publication

- User-authorized source commit879c69b2f11c825c1d4f10b5409e5a5148f06e67 is
  published on design-retro-green-crt-20261003; exact local/fetched/live refs
  matched at20:19 CEST. No main merge or dirty-root product synchronization.
- Fresh Web1712/96 files and TypeScript/Vite build pass; separate in-session
  review/static scan found no blocking issue. Existing16-group browser receipt
  covers unchanged source. Author/committer github@knxbench.com; no co-author.
- The following local design notes are historical; palette/study and proposed
  production limitations are unchanged. This acknowledgment changes Markdown only.

## 2026-10-03 — CRT 1.1 reference-image development (local)

- Evolved the existing design from the user reference: softer phosphor ink,
  green-black surfaces, fine green rules, mint action gradients and compact
  chrome. Same theme ID, palette version1.1.0, unchanged safe v1 token contract.
- Replaced the study's layout-button rows with twelve native table rows and
  independent checkbox/address-button controls. Added a bounded leading light,
  violet Save flare and shared activation guard; cancellation handles user Off,
  OS reduced motion, scroll and resize. All data and actions remain synthetic.
- Ten theme/study tests pass (new revision observed RED before implementation);
  focused329, complete Web1712/96 files and TypeScript/Vite build pass.
- Retained reproducible `design/verify-crt-reference.mjs` and JSON receipt:16
  Chromium groups, including actual manager refused/confirmed same-ID replacement,
  exact reload/export and mid-effect cancellation. One guarded mock settings write,
  zero page errors/unexpected requests. Screenshot samples a paused native frame.
- Updated [design guide](DESIGN_RETRO_GREEN_CRT.md) with actual palette behavior,
  reproduction and specific token/component/motion proposals. No production
  component, protocol, core, dependency or storage change; no native/Orca/WCAG claim,
  commit, push or root product synchronization. Earlier 1.0 results below are
  historical; retained current design artifacts are now1.1.

## 2026-10-03 — Modern Retro Green CRT design artifacts (local)

- Added an importable complete theme-pack v1 palette: black `#050505`, neon
  green `#39ff14`, installed JetBrains Mono throughout, small radii, green
  elevation/hover-shadow tokens and distinct error/warning colors.
- Added a self-contained interactive HTML design study with embedded licensed
  JetBrains Mono, purple Save, left-to-right fill, activation glow, filtering,
  keyboard navigation and user/OS motion guards. This is a standalone reference,
  not React/Tailwind/Tauri component integration or actual project operations.
- The production v1 pack cannot separate Save from the shared accent, set exact
  `#003300` row fill, inject animation rules or add input focus-shadow consumers.
  These remain explicitly proposed component/versioned-token changes in
  [the design guide](DESIGN_RETRO_GREEN_CRT.md); no theme-contract weakening,
  dependency, domain, protocol, persistence or production-component change.
- Seven palette tests observed RED, then GREEN. Intermediate test-harness
  environment/URL issues were corrected without changing runtime code. Focused
  suite: 326/326; full Web: 1,709/1,709 in 96 files; TypeScript/Vite build passes.
- Ten actual Chromium verification groups pass: real manager import preview,
  conditional acknowledged apply, reload/all-token paint and UI export, plus
  six standalone-design checks including keyboard activation, actual 250ms fill,
  Off/reduced-motion and 390px layout. Settings were intercepted/synthetic;
  one mock conditional write, zero unexpected requests or page errors.
- Local isolated branch `design-retro-green-crt-20261003` at baseline `e7f9db8e`;
  no root synchronization, commit/push, native/Orca/full-WCAG or hardware claim.

## AR06Q KL-151 resource research accepted — 2026-10-03 22:02 UTC

Base5540dcac/source706 unchanged. Secure853 declared-size census selects15:
7 member-only/5 total-only/3 both. Scratch-only fixed member256MiB/expanded4GiB
variant (compressed256MiB, same grammar/evidence) admits14 and refuses1 namespace;
baseline15 size refusals remain atomic. Fresh Release profiles,22 build/test
commands/16 hostile ZIP controls/six helper controls verified. Peak789976KiB
RSS, max221.55s/7556988928-byte DB;638.04s/21970833408 DB bytes total. All originals
independently rehashed, retained archives checked, private temporary data gone,
closed aggregate-only output. First zero-pair namespace-guard PermissionError
retained as infrastructure rejection; corrected isolated retry is the accepted
measurement. No production limit/code change or general compatibility claim.
KL151 remains open pending bounded cap/streaming/caller safety decision; direct
HTTP catalog install/product mutex needs owner-aware latency acceptance before
a global raise. KL152/153 and wider Alpha remain open. KL1505540dcac fully
delivered and owned checkout/branches/targets/scaffolds cleaned.

## AR06P KL-150 scoped code delivered — 2026-10-03 23:05 CEST

Actual integrated cbe9952f passed fresh complete public16 and the authorized
private same-profile baseline RED/candidate GREEN. Source706 is identical to
the full853 Release CLI pair (installed687→688, one constraint refusal admitted,
all other outcomes unchanged; originals rehashed, no raw/private item output).
Code published as1b215d51 after adopting a foreign stats-only commit byte-exact;
fresh doc5 gates and live/fetched equality verified. Fourteen owned build/
snapshot/browser directories removed; final metadata/checkout hygiene remains.
KL150 fresh-install storage/key-scoping is lifted, not nested runtime/allocation/
parameter-write semantics or automatic old-catalog repair. AR06P/AR07/full
Alpha remain open; KL151/152 require measurements, KL153 bounded research.

## AR06P KL-150 full candidate verification — 2026-10-03 22:35 CEST

Owned candidate a2aa4b7 is committed, source706 exact; not published yet.
Fresh complete public16 retry: Rust153 blocks/3009-0-166, compiled ignored166,
Web1702, Chromium inventory/pass82, shadow bindings17 significant-token exact
with eight controls; strict workspace Clippy/build/deny/format and all audits.
First broad eight commands exited0; the added binding helper's pre-v7 absolute
directory API assumption rejected verification, not product tests or semantics.
Original evidence retained and whole16 rerun, never fixture-origin weakening.
Release pair12 separately reconciled: exact committed old-parser named RED,
candidate storage7 GREEN, CLI5 GREEN each; both profiles Release, binaries/logs
hash-exact. Full853 original-filename CLI pair accepted and independently
rehashed/reconciled: installed687→688, one database-constraint refusal→installed;
all other categories unchanged, no installed→refused regression. Every side has
a fresh private DB; successful archive bytes/source blob digests verified and
four catalog/package tables empty on refusal. Original manifest/all inputs
unchanged, temporary copies/DBs removed, only aggregate receipts survive.
Fresh current-upstream integrated acceptance, final metadata/publication/readback
and owned cleanup remain pending; KL150 remains unchecked. R-MODULE-04 runtime,
new limit/grammar claims and automatic repair of old stored rows are not included.

## AR06P KL-150 lexical storage candidate — 2026-10-03 20:56 CEST

The nested-definition UNIQUE-constraint bug has an exact synthetic package RED.
The smallest private parser scope stack preserves enclosing definition identity
and argument position, including Empty and deeper/sibling scopes. No public
API, schema/version, evaluator, UI, protocol or limit change. Nine focused
ordinary tests and ProductDB625/0/25 pass; compiled ignored inventory25 and
strict Clippy/format/whitespace verified. Three separately compiled mutants
have five named failures; a fourth exact committed pre-fix parser has its own
public RED. Canonical706 source/config inputs remain unchanged during gates.
One authorized offline private nested package passes only with the candidate:
same-profile baseline RED/candidate GREEN, independent lexical/stored scope
counts agree, all original ZIP member bytes match retained blobs, retry passes,
original package/manifest unchanged, zero private temporary directories and
no private raw/item records. This is not a new full853 or115/113 matrix run.
Separate in-session review has no blocking finding, not independent-model
approval. Fresh broad/current-upstream acceptance, full853 CLI measurement,
publication/readback and owned cleanup remain open; KL150 is not lifted.
R-MODULE-04 runtime/allocation semantics and automatic repair of old admitted
mis-scoped rows remain outside this storage correction. KL149 closing metadata
cb5781c7 is published/live/fetched byte-verified and its own final target,
seventeen runner/config scaffolds, checkout and branch have been removed.

## AR06P KL-149 scoped delivery checked — 2026-10-03 18:45 CEST

Only CLI exact-extension dispatch now uses ASCII case-independent comparison.
Five synthetic CLI regressions cover fresh uppercase/mixed install, retained
byte/idempotent retry, uppercase project routing, eight legacy refusals and
three lookalikes. No parser/schema/legacy/UI admission added. Original RED and
seven-command expanded GREEN are verified; two separate compiled snapshot
mutants yield four named behavioral failures, canonical never mutated.
Fresh candidate public16 accepted: Rust3000/0/165 (151 blocks), Web1702,
Chromium72 in a network namespace retaining exact4173 safety guards,17 equal
bindings and700 frozen inputs. Both port/origin rejections retained.
Release-pair nine stages accepted: pre-fix committed producer two named REDs,
candidate5/0/0, exact same release profile and source inputs. Incomplete debug
measurement deliberately interrupted after56 pairs; zero private temporaries,
no final accepted result, receipt retained. Full offline original-filename853
release measurement proc_1fcfbf5cdfa3 is independently accepted:644→687
installed, Hager/Berker2→45;43 new installs and three still explicit refusals.
All originals, retained successful blobs, source700 and binary hashes verified;
refused package/source_file/product tables empty, no private raw/item records
or temporary private directories. This is unpinned measurement, not a CI or
semantic compatibility claim. Normal current-U18 e7f9db8e integration dd5a350c
passes NEW actual16 independently: Rust3000/0/165/151 blocks, Web1702,
Chromium82 inventory/pass,17 equal bindings/704 committed-exact inputs.
All285 CLI Rust/build/test inputs equal the privately measured producer; that
receipt retains its original run identity. Delivery362fec24 published/live/
fetched read back at refs0/0, source704 and ten owned artifacts exact, complete
owner history retained. Five acceptance Markdown gates passed.16 completed
own build/snapshot/shadow/XDG directories removed, all aggregate/public logs
retained. Closing metadata/final target/checkout/branch cleanup remain; broader
AR06P/AR07/Alpha open, next KL150 scoping RED; package limits unchanged.

## AR07 checked outside-walk scoped delivery — 2026-10-03 13:54 CEST

Scoped delivery 0b8ec935362d06642a81bafcbbb74824236c39b8 was pushed/fetched/live-read back at the recorded
checkpoint: refs/trees equal0/0,699 actual-gated inputs and7 acceptance documents
exact. Actual0369a56a's22 accepted commands and final five nonempty/root-explicit
doc/whitespace gates retain their run scope; later delta is Markdown only.
Twelve completed own build/snapshot/shadow/XDG directories removed after process
checks; originals/foreign/root unchanged. Closing metadata gates/readback and
clean checkout/branch/scaffolding removal remain, not new policy acceptance.
AR06P KL-149 is the next ready package; broader AR07/Alpha/UI-native/ETS remain
open. Published statistics-owner artifact preserved, not a local statistics refresh.

## AR07 checked text overlay actual acceptance — 2026-10-03 13:32 CEST

Source a065ad94 / actual0369a56a, proc_39eb1bd6a2f3:22/22 accepted,
16 fresh public+3 compiled selected inventories+3 private runs. Rust2995/0/165
(150 blocks), Web1702, intercepted Chromium72,17 equal bindings,699 exact
committed inputs. Selected private6+2+1 cases pass;420 originals unchanged,
no private raw logs, transient link removed. Five real compiled behavior mutants,
separate in-session review, not independent-model/native/ETS approval.
ADR0066 accepts only checked scoped FunctionText/channel-copy admission and
whole existing400 refusal; loaded project/product bytes preserved, no new DTO.
Legacy SDK/unscoped/general metadata/query/serializer/RSS and UI-specific
language/refresh snapshot/localization remain explicit separate boundaries.
Post-gate e2a40268 Markdown-only AR06P/owner history merged7bb0ee72, source699
unchanged. Acceptance docs/publication/readback/own cleanup PENDING. Next ready
queue is new AR06P KL-149 then coordinated nested parser KL-150; broader AR07
matrix/full Alpha remain open. U18 Web reservation untouched.

## AR07 outside-walk candidate Broad accepted — 2026-10-03 12:45 CEST

Public candidate fourteen-command evidence reconciled independently in-session:
13 verified reused commands plus1 new, Workspace2995/0/165 (150 result blocks),
Web1665, Chromium61,17 semantically equal shadow bindings,619 unchanged inputs.
Four runner-only rejections retained; final actual inventory/type/placeholder
checks follow source/Git facts, without changing production or audit limits.
Seven-command focused GREEN and five compiled behavioral mutants remain valid.
Separate complete code/test review has no blocker, not an independent-model
verdict. Current upstream c6b5a240 contains U17 Web changes; candidate evidence
is not integrated acceptance. ADR0066 remains Proposed pending fresh merged
public/private-safe compatibility gates, acceptance/docs/publication/readback.
UI retains U18 ownership/lock; full AR07, semantics matrix and Alpha stay open.

## AR07 remaining public audit inventory correction — 2026-10-03 12:27 CEST

Continuation proc_b03d8af2339a rejected only for own erroneous >400 source-count
assertion: both new audits exit0, anchors376 links/247 MD files, corpus340 Rust
files. Source-backed independent census apps105/crates235 (xtask excluded by
that scanner) matches340 exactly; predicate corrected to actual inventory, not
another magic minimum. No production/619-input change. First and continuation
rejections remain preserved, not GREEN. Thirteen actual commands passed; final
whitespace/17-shadow-binding/source checks dispatched proc_173bac7ae3af, queued
under both shared leases, final PENDING. Complete candidate acceptance must say
13 verified reused+1 new, not fresh14 or actual merged/private/owner evidence.
Reviewed commit/integration/applicable private-safe gates/docs/push/readback/
cleanup and full AR07/Alpha remain open.

## AR07 public broad runner rejection — 2026-10-03 12:14 CEST

First proc_3d0b6f570fda retained rejected solely for header-output parser mismatch,
not Rust/Clippy/header-audit failure. Eleven actual commands exit0 independently
reconciled; workspace2995/0/165 over150 result blocks, Web1665/Chromium61,
strict Clippy/build/dependency/fmt/layering and headers414/157 ceiling157/17
nonempty intended-root audit. Header source template in xtask:330-339 verified,
parser corrected with2 public positives/9 negatives; no production/source change.
Canonical619 inputs/HEAD5ca570a0 exact, raw logs hash-pinned for continuation.
Remaining3 commands/final17-shadow-binding checks dispatched proc_b03d8af2339a
under shared leases; complete broad still PENDING, not a fresh14-stage run or
integration acceptance. Foreground queue timeout at420s produced no stage/summary
and no own survivor; retained separately, corrected to notified background launch.
No foreign process/lock bypass/private output. Review/ADR0066 still candidate;
actual integration/applicable private-safe gates/docs/push/readback/cleanup follow.

## AR07 outside-walk current public acceptance — 2026-10-03 11:47 CEST

Current candidate remains uncommitted/ADR0066 Proposed. Actual expanded
proc_3ceb5030b9ba independently accepted7/7 public stages: complete HTTP2/0/0,
ProductDB363/0/0, DynamicTree66/0/6, Server214/0/2, strict Clippy/fmt/whitespace,
619 current source/config inputs byte-equal. Five compiled behavioral mutants
caught; each compile/inventory0 and exact named Rust101/0-1-0: independent
projection/raw admission, cached copy, checked scoped caller and per-object
reset. No compiler/zero-selection/timeout/OOM substitute, no private input.
Canonical never mutated; isolated snapshots/fresh per-snapshot Cargo targets.
Separate in-session full source/test review: zero blocking code findings,
loaded-project wording clarified (not native save/reopen atomicity evidence);
no independent-model review claimed. Test harness isolates XDG default opener.
Fourteen-step fresh-target ordinary workspace/Web/intercepted Chromium/binding/
dependency-policy/repository-audit broad gate dispatched proc_3d0b6f570fda,
PID2863725 under both common locks; final verdict PENDING, not accepted branch
or merged/corpus/ETS/native/owner UI evidence. Source619 frozen and index empty;
only explicit Markdown notice delta allowed. After broad: actual integration
and applicable private-safe selected gates, fresh docs, publication/readback
and cleanup; entire AR07/Alpha still open. No bus/vendor/subagents/quota checks.

## AR07 outside-walk text refusal — candidate only, 2026-10-03 11:23 CEST

ADR0066 stays Proposed. Public HTTP RED independently accepted compile0 and
Rust101/0-1-0: active ordinary detail first, then unexpected HTTP200 versus400,
nonempty project and product-file equality before failure; no timeout/OOM.
Candidate uses one substitution scanner, independent sticky TextProjectionBudget,
pre-admission raw/lookup/UTF8 output, per-device sharing and cached-text copy
admission. Two production consumers now propagate checked Results through the
existing domain/device GET400 error envelope. No DTO/UI/persistence/grammar,
parameter authority or manufacturer semantics change; legacy String helper stays
unmetered, other loaders/metadata/serializer/RSS are not bounded by this policy.
First public GREEN7/7 accepted HTTP1, ProductDB355/0/0, Dynamic66/0/6,
Server213/0/2, strict Clippy/fmt/whitespace and618 source/config inputs. Later
public Core363/0/0 exercised eight new checked-text unit regressions; its HTTP
selection still excluded the new cached-copy case. Current619-input candidate
adds scoped FunctionText unchanged-activation oracle and HTTP exact2-vs3 cache
copies/whole-error/privacy/project-product equality. Expanded GREEN then five
isolated compiled mutants dispatched proc_3ceb5030b9ba: PENDING, not an accepted
mutation/full/merged verdict. Snapshot preparation's symlink rejection and stale
count/preflight assumptions retained as orchestration failures, not test REDs.
Fresh fetched remote and own base5ca570a0 equal; U17 Web lock remains held by
ui-theme-management, not touched. No bus/vendor/private inputs or new model
review. Full review, broad/actual integration, doc gates, publication/readback,
cleanup and complete AR07/Alpha remain pending.

## 2026-10-03 — Product install tested against 853 public manufacturer downloads (test only)

No code changed. The release `knx products ingest` from `c6b5a240` ran over
853 files that the separate `knxprod-crawler` tool downloaded from Siemens,
ABB/Busch-Jaeger, Hager/Berker and MDT, into one fresh shared database
(content order). 644 installed (608 new, 36 byte-identical). 147 were refused
by namespace (scheme 10: 145, scheme 23: 2), 46 hit a case-sensitive CLI
extension check, 13 hit the size limits, and 1 each hit the evidence item
limit, a database constraint crash (nested `ModuleDef`s) and an invalid ZIP
(a PDF named `.knxprod`, correctly refused). `products verify` reported 0
mismatches. Coverage: 51 of 1,167 programs are plannable for download.
Database size 13.2 GiB for 1.25 GiB of input. Details and method:
PRODUCT_DATABASE_CORPUS §Public crawler corpus run. New limitations:
KNOWN_LIMITATIONS §149–§153. The run is evidence, not a pinned gate; the
downloaded files are not part of the repository.

## 2026-10-03 — U17 Appearance manager (delivered/read back as 4d9073ca)

Settings → Appearance now manages immutable builtins and admitted installed
packs with origin/version/saved status, import/export/recovery, explicit Apply/
Cancel, content-bound replacement/removal questions and System reset. The root
runtime is the sole visual owner; the Debug report reads without acquiring a
second DOM lease. Live EN/DE outcomes and all actual admission kinds are typed;
accent controls expose only variations the displayed palette can apply.

31 actual parent cases and five root cases cover stale/late/out-of-order intake,
cross-client contents/selection, repeated Apply, definitive 409 and uncertain
500 reconciliation without write replay, close during acknowledgment, exact Blob
roundtrip/recovery and independent preference preservation. Both manager and
original selector report server success separately from local cache failure.
Latest scoped 74 and TypeScript pass without stderr. Six behavioral guard
controls were caught/restored; the new diagnostic module's TS2322 inclusion
canary was caught and restored. Final frontend candidate: Web1,702/95 files,
Chromium69 without skip/flaky/failure, build/types and262 frozen inputs unchanged.
Eight behavioral controls were caught and restored. Full branch13/13 repository commands pass: Rust2940/0/164,17 equal bindings
and693 inputs unchanged; actual-merged acceptance/publication remain pending.
U17-R1 acceptance audit found missing explicit manager-browser rejection/reset/
HTTP500 gestures. Added three fully intercepted cases:11 manager browser cases
and three further compiled controls pass/restored, no production change. Earlier
full69-browser source predates this test-only delta; renewed actual gates pending.
Full legacy Web suite emits fixture stderr; no warning-free whole-suite claim.
Native WebKitGTK/Orca/global-alpha/ETS and U18 acceptance remain separate/open.

The candidate-only statements above are superseded by actual chain
proc_cdcd42b97d47 on f16f1e40:23/23 commands passed,17 repository and six offline
inventory/execution commands; Web1702, Chromium72 (zero failed/skipped/flaky),
Rust2984/0/165 across149 blocks, compiled ignored inventory165,17 equal bindings,
697 unchanged source/config inputs. Six selected offline suites execute27 private
cases plus one115-instance/113-unique matrix case;420 private files unchanged,
temporary corpus link removed. Earlier cancelled/interrupted runs are not
acceptance. Integration with upstream5ca570a0 changes only Markdown and preserves
complete owner histories/gated source; document gates/publication/readback are
tracked in the current handover. U18-R1 remains IMPORTANT: representative
editor/inspector/table/dialog/diagnostic state coverage across palettes is not
established by the current generic root-switch fixture. U17 management acceptance
does not close U18, native/Orca, general WCAG, ETS or global-alpha release.

Publication4d9073ca092198b23fbb52411cd68edc7fab8c4a verified: owned HEAD,
fetched main and live main equal; full Git trees equal, all697 gated source/config
inputs and25 owned artifacts exact remotely, zero outgoing commits. Subsequent
changes from gated f16f1e40 are Markdown only; four nonempty intended-root
documentation audits and whitespace passed. U18 stays open with U18-R1.

## AR07 bounded scalar-copy policy delivered — 2026-10-03 09:59 CEST

Source/actual acceptance published and fetched/live read back at
cbc6b0b238fbeef2da41c4d38208850574c68760: owned HEAD/fetched/live main equal,
divergence0/0; all617 actual-gated code/config inputs and7 acceptance documents
exact remotely. Canonical dirty root/main/statistics unchanged. Actual9f512ab3
continuation remains20 accepted stages with5 verified public commands reused,
13 commands+2 checks new, workspace2984/0/165, Web1665/Chromium61, selected
private Dynamic6/0/0 and offline SimTunnel14/0/0;17 equal bindings and420
unchanged originals including103 archives. Four fresh-target nonempty
intended-root acceptance-document audits and whitespace passed; zero code delta.
ADR0065 accepted only for this bounded Core/HTTP scalar-content admission,
not external String-only projections or full AR07/ETS/Alpha semantics. Initial
header/classifier rejections and compiled mutant/restoration receipts retained.
Six completed owned build targets actually removed after process checks;
compact machine-readable evidence retained. Closing receipt-only metadata
gates/publication and clean owned checkout/branch/scaffolding removal follow.
Whole AR07/Alpha, ISSUE-08 checked-result consumer contract, UI diagnostics/
identity adoption, native/hardware/vendor/general allocation/RSS remain open.

## AR07 scalar-copy actual integration accepted — 2026-10-03 09:52 CEST

Reviewed source2e7a41c3 conventionally merged with published c07e6403 at
9f512ab3e302014d1b4c3d2af33cd33712a052ad; upstream ADR0064/owner histories
preserved, own scalar policy ADR0065. Actual proc_3ea5d4e7afa4 exit1 is retained:
private Dynamic Rust6/0/0 exit0 but closed skip classifier rejected two output
signals. Private raw lines were not persisted; do not claim recovered text.
Source-backed public regression proves bool-valued PackageInstallReport.skipped
and registered libtest-prefix cases; corrected classifier5 positives/7 negatives
pass, unknown or missing-data signals still rejected. No production change.
Continuation proc_8dd254bb5c75 exit0 independently accepted20 stages: five public
commands reused from the exact same committed617-input tree (not a wholly fresh
20-command run),13 commands and2 final checks newly executed. Workspace2984/0/165
over149 blocks, Web1665, intercepted Chromium61; newly selected private Dynamic
6/0/0 and in-memory offline SimTunnel Download14/0/0, zero unknown skip signals.
All617 current/committed source/config hashes and17 shadow bindings equal;
420 originals including103 product archives unchanged across the complete new
private window. Hashing420 files does not mean parsing420 files. Strict
Clippy/build/fmt/dependency and four nonempty intended-root audits pass.
Earlier candidate/header failure and three compiled omission mutants/restoration
remain scoped to their actual runs, not relabelled. Separate in-session review,
not an independent-model verdict. Acceptance-doc gates/publication/readback
pending. Broader AR07/Alpha, external String-only ISSUE-08 projections,
UI diagnostics/identity adoption, native/ETS and general allocation/RSS/latency
remain open. No live bus/vendor code or private raw logs; UI owner untouched.

## AR07 scalar-copy audit — three valid public REDs (2026-10-03)

The preceding bounded module-provenance source and final receipt14e2eb9a are
published/read back; clean own checkout/two ancestor-confirmed branches and
build scaffolding actually removed, foreign dirty root untouched. New clean
alpha-text-output starts from14e2eb9a, not the older canonical root checkout.
Read-only audit confirms unmetered binding/label scalar copies. Initial ancestor
lookup hypothesis rejected: ModuleScope::argument deliberately searches only its
own vector, already charged correctly; do not invent inheritance or ancestor fees.
ADR-0065 proposes scalar cost admission only. proc_49808cde0743 compiled0 and
executed four assertions; independently accept three valid copy REDs101/0-1-0
(8,000,002/8,408,071/8,393,618 bytes versus proposed4,000,000), no timeout/OOM.
Fourth expected ancestor search incorrectly;4096 unresolved names prove the
non-inheritance rule, now a positive regression, not a correctness RED.
Six production/consumer files frozen unchanged during RED. Additional UTF-8
exact-cost unit101/0-1-0 and positive non-inheritance baseline1/0/0 verified.
Candidate core now admits binding bytes, raw length before reservation/scan and
whole output slices before copy using existing quota/marker; no ancestor or API
change. Public Core GREEN proc_c73947a5b170 independently accepted5/5: library
355/0/0, DynamicTree66/0/6 (private ignores not executed), strict Clippy/fmt/
whitespace and frozen inputs; five consumer sources unchanged. Added new-cause
HTTP regression shares existing full-prefix authority/atomic project+source
checks with inert-work case; proc_8ee608d8a643 independently5/5, named1/0/0 and
complete39/0/0, strict server/PDB Clippy/fmt/whitespace/frozen inputs. Compiled
copy-guard omission/restoration proc_98b4f3f512f1 independently accepted3 compiled
mutants caught by4 actual assertions, canonical evaluator/all9 source hashes
restored. Candidate-only broad proc_2b136248f7f9 rejected at actual headers
158>157 after workspace2963/0/164 and other runtime stages passed. Fixed only
new test doc separator per ADR0018, not ceiling; fix gate410 valid/157 absent
passes. First rejected receipt/logs retained; retry proc_230c3f7e4db9 independently
passed13/13, workspace2963/0/164 across148 blocks, Web1665,615 frozen inputs and
17 unchanged shadow bindings. Separate in-session review has no blocking finding,
not independent-model approval. Upstream owns ADR0064; copy policy renamed0065;
no actual-source delivery or full
AR07 acceptance yet. Outside-walk String-only ISSUE-08 projection remains
coordinated owner work; no Web lock, caller or generated binding changed.

## Commissioning bounded history source delivered — 2026-10-03 07:10 CEST

Source and gated acceptance published/read back at
`cde52ebc1e9f9125f5b94096a6b6c001a3269bd4`: local/fetched/live main equal,
all23 contributed artifacts and617 actual-gated inputs exact remotely. Actual
66dca279 acceptance below remains18/18, workspace2978/0/165, Web1665 in93 files,
offline Download14/0/0 and Dynamic6/0/0,17 bindings and108 scoped originals
unchanged. Four fresh-target acceptance-document audits/whitespace passed.

Four completed owned build targets actually removed after live-process/lease
checks;105 compact JSON receipts, including rejected/survivor evidence, retained
outside the repository. Closing receipt-only doc gate/publication and clean own
checkout/scaffolding removal follow separately; canonical root/statistics belong
to their owner. SAFE-03/AUDIT-01 long-session work remains next, not complete.
Web/native/hardware/vendor/ETS/release boundaries are unchanged.

## Commissioning actual integrated backend accepted — 2026-10-03 06:59 CEST

Actual merge `66dca2793fcaf50c2149d73c90364a4ae727cd06` integrates reviewed runtime
source `0fc483c2` and published parent `14e2eb9a`. Exact18-step receipts and
committed-source hashes independently reconciled after process exit0: workspace
2978/0/165 over149 result blocks, Web1665 in93 files, selected offline in-process
Download14/0/0 and Dynamic6/0/0 with no unknown skips and108 scoped originals
unchanged. All617 frozen code/config inputs and17 shadow bindings equal.
Strict Clippy/build/fmt/dependency policy and four nonempty intended-root audits
pass. Final metadata gates and publication/readback are pending, not source tests.

Complete owner handover/status histories remain preserved; ADR0064 does not
overwrite published0062/0063. Two rejected receipt-verifier assumptions (Web
file count and optimized xtask root suffix) were corrected from raw evidence
without replaying the gate. Historical candidate entries below are superseded
for the current integration, not silently relabelled. SAFE-03/AUDIT-01 remain
partial: long-session durable intent/terminal/version evolution is next offline
work. Web/global consumer, independent hardware/vendor evidence and controller
release stay open; no Chromium/native/ETS/device-success proof follows.

## Commissioning durable metadata / admission candidate — 2026-10-03

Separate version-1 activity storage and bounded authenticated history API cover
four one-shot kinds; seven untracked kinds and volatile long-session data remain
explicit. Identity/state validation, interruption projection, sticky failure,
serialized first admission and pre-tunnel write refusal are offline-tested.
Metadata is not a recovery image or device-success proof.

Late review reproduced foreign hot-journal recovery before format refusal.
Corrected admission is read-only before the writable opener; a raw SQLite
header guard also refuses WAL before sidecar creation. Two synthetic RED/GREEN
regressions protect both openers and original main/journal/WAL bytes. Current
21 compiled behavioral mutants fail as expected, with exact source restoration.
An earlier 18-step run (workspace 2948/0/165, Web 1559, private download 14/0/0,
Dynamic 6/0/0) describes the pre-admission-fix source only. Corrected integrated
gates and publication are PENDING, not implied by focused tests.

Corrected candidate is now independently accepted at 18/18: workspace
2950/0/165, Web 1559 in 89 files, explicitly selected private download 14/0/0
and Dynamic 6/0/0, zero unknown skips and 108 originals unchanged. All 608
frozen code/config inputs and 17 shadow bindings match; reviewed runtime diff
is unchanged. Integrated Float-guard coexistence and publication remain
PENDING. This does not close long-session, Web, hardware or release scope.

The [42-ID ledger](COMMISSIONING_ALPHA_LEDGER.md) matches the readiness inventory
without omissions, duplicates or extras; every row retains its fallback and
unblock condition. [API contract](COMMISSIONING_ACTIVITY_HISTORY.md), ADR-0064
and the direct Profile audit document the bounded support. Long-session intent,
Web adoption, independent hardware/vendor evidence and release remain open.

## AR07 module-scope backend leaf delivered — 2026-10-03 05:48 CEST

Published/read back source/acceptance3a8b66f422bda73c9999fb214f2459c79ecea76b;
local/fetched/live refs equal0/0 at that checkpoint, all11 owned artifacts and
624 actual-gated inputs exact remotely. Source d51dd6c7, actual3711c4f7, the
independently accepted20/20 and three compiled mutants/restoration below remain
the evidence. Only owned Markdown changed after the actual gate, with four
nonempty intended-root acceptance audits and whitespace check passing.

Completed owned targets/raw scaffolding removed after live-process checks;
minimal aggregate receipts retained. Final receipt-only publication and clean
checkout/two ancestor-confirmed branch removal follow separately. ADR-0063's
bounded backend identity is delivered, not UI consumer/diagnostic association,
genuine nested manufacturer, external substitution or whole AR07/Alpha/ETS
acceptance. Canonical root/statistics remain untouched with their owner.

## AR07 module-scope provenance — actual integration accepted (2026-10-03 05:38 CEST)

Reviewed source d51dd6c7 is conventionally integrated as3711c4f7 with published
e9707794, the exact candidate tree and complete inherited owner handover.
proc_f27315f3cd8c's actual20/20 is independently accepted: ordinary Rust2957/0/164
over148 result blocks; Web1665 and existing intercepted Chromium61; explicit
private Dynamic6/0/0 and in-memory SimTunnel HTTP13/0/0 with no genuine/unknown
skip and no private raw logs. All624 committed code/config inputs and17 shadow
bindings exact;420 original files, including103 product archives, unchanged.
The broader hash inventory is not a claim that every file was parsed. Strict
Clippy/build/fmt/dependency and four nonempty intended-root audits pass.

Three compiled mutants/restoration and bounded public RED/GREEN below remain
their narrower evidence. First receipt verifier's `Checking`-only Clippy display
assertion rejected separately: actual fresh output says `Compiling knx-server`;
corrected evidence check, no source change or gate replay. ADR-0063's bounded
backend projection contract accepted; publication/readback and cleanup pending.
Manual UI scope matching stays with U17; no new UI, real nested manufacturer,
storage/ETS parity, independent-model or full AR07/Alpha acceptance claimed.

## AR07 module-scope provenance — public GREEN, not delivered (2026-10-03)

At `e9707794`, public nameless nested paths retained distinct Core chains and
server sections but serialized identical HTTP scopes: compilation 0, behavioral
Rust 101 / 0 passed / 1 failed / 0 ignored. Production hashes unchanged during
RED. The additive ADR-0063 backend candidate now supplies `nodeChain` from the
existing Core accessor on section and diagnostic scopes, without changing
write authority, scoped values, domain/storage schemas or legacy fields.

Public gate independently reconciled: parameter HTTP 38/0/0, server library
205/0/2, selected public ProductDB identity suites 72/0/6; strict two-crate
Clippy, fmt/whitespace pass and 624 code/config inputs unchanged. Three compiled
wire-omission, innermost-only and order-reversal mutants each fail behaviorally
with 101 / 0-1-0, restored sources and final HTTP/mapper GREEN verified. Repeated
tests are subsets, not additional coverage. Separate in-session contract/security
review found no blocking issue, not independent-model approval.

Actual integrated gates/publication remain pending. UI manual `ModuleScope` and
`sameScope()` adoption remains with the U17 Web-lock owner; no Web or binding
change. Genuine nested products, broader AR07/Alpha and ETS parity stay open.

## AR07 bounded work admission delivered — 2026-10-03 04:10 CEST

Published/read back2704f8e29b6ca2d53c022c468f116b526109c678; local/live/fetched
refs equal0/0, all14 owned artifacts and624 actual-gated code/config inputs exact
remotely. Source8f47c13b, actual merged7ae116a1 accepted20/20 as below, five compiled
mutants/restoration; doc-only acceptance gates pass and code delta is zero.
Own build/script/raw-log scaffolding removed after active-process checks, minimal
aggregate receipts/source-only next-audit retained. Final receipt-only publication
and completed clean checkout/branch removal remain the closing steps.

ADR-0062's bounded work policy is delivered, not complete AR07/Alpha, localized
new-token UI, byte/RSS/latency/general-depth, external-substitution or native/ETS
acceptance. Other owners' complete histories and source remain intact.

## AR07 work admission — actual merge accepted, delivery pending (2026-10-03)

Actual7ae116a1 integrates reviewed budget8f47c13b with published0889c102 U16/U17,
without source conflict or lost owner history/artifacts. proc_096e63a3429e exited0
and all20 stages independently pass: workspace2953/0/164 over148 result blocks,
Web1665, intercepted Chromium61, explicit private Dynamic6/0/0 and offline
SimTunnel HTTP13/0/0 with no genuine skips or private raw logs. All103 product
archives/108 original fixtures unchanged,624 frozen code/config hashes equal
exact committed blobs,17 shadow bindings equal; strict build/lints/dependency
and four nonempty intended-root audits pass. Projection42 is a workspace subset.

Five compiled behavioral mutations/restoration and public RED/GREEN precede
this merged acceptance. Both initial lock timeouts remain rejected infrastructure
attempts before compiler/source start, not behavioral evidence. Delivery/readback
remain pending. Manual new-token/localization adoption stays UI-owned; broader
AR07/Alpha, byte/RSS/latency/general-depth and native/ETS guarantees stay open.

## AR07 work-admission candidate — public GREEN, not delivered (2026-10-03)

The owned `alpha-parameter-budget` candidate adds shared evaluator work
admission and explicit truncation reporting; incomplete parameter-panel prefixes
are read-only. Original and sibling public REDs, then a compiled HTTP 200/400
write-authority RED, precede their respective fixes. ProductDB Library 354/0/0,
DynamicTree 62/0/6, full parameter HTTP 35/0/0 and server Library 204/0/2 pass;
targeted repeats are subsets. Strict ProductDB/server Clippy, formatting,
whitespace, source freeze, nonempty-project refusal and original-source
retention checks pass. No private or live-hardware acceptance is inferred.

[ADR-0062](adr/0062-dynamic-evaluation-work-admission.md) remains Proposed:
five compiled behavioral mutations and exact source restoration are independently
accepted at proc_43b041004d63; first lock-only timeout rejected separately.
Actual candidate proc_cd67fb854490 exited0 and all20 stages independently pass:
workspace2944/0/164, Web1559, intercepted Chromium61, private Dynamic6/0/0 and
offline SimTunnel HTTP13/0/0,103/108 originals unchanged,616 frozen inputs,
17 equal shadow bindings and strict build/lints/nonempty intended-root gates.
This is restored candidate GREEN, not current-upstream acceptance. Fresh
origin/main0889c102 U16/U17 must be preserved/integrated and the combined tree
gated before publication. UI token/localization adoption belongs to its owner.
The work cap is not a complete byte/RSS/latency/general-depth guarantee; broader
AR07 and Alpha remain incomplete. See [the bounded evidence](PARAMETER_SEMANTICS_BOUNDARY.md).

## 2026-10-03 — U16 acknowledged theme persistence (foundation delivered as 1f94808d)

- Implemented strict bounded UTF-8/BOM import, validated deterministic export,
  detached/frozen content-bound theme plans, explicit replacement consent and
  coupled active removal/System selection through the existing settings queue.
- Optional key-scoped conditional PUT plus capability version 1 retains the
  opaque settings-file schema. Unknown preferences/raw entries are preserved;
  incompatible, pending and uncertain authority disarms mutations. Network
  ambiguity rereads without replay; cache failure is reported independently.
- Five-file focused suite: 144 passed; TypeScript passed. Thirty-one restored
  behavioral controls and TS2322 canary pass; two initial survivors required
  stronger export/status assertions. Separate self-review fixed aggregate
  selection/runtime disagreement with an observed RED/GREEN regression.
- Raw recovery exports browser-observed theme JSON only, not a byte-exact file.
  Latest frozen candidate passed 16 checks: Web 1,665, intercepted Chromium 61,
  ordinary Rust 2,925 / zero failed / 164 ignored. Eight conditional HTTP cases,
  seven compiled server guard controls (including the initially surviving lock
  case), five extra ordinary queue controls and all-source restoration verified.
  Actual merge 36e922bc passed 22/22 commands: Web 1,665, intercepted Chromium
  61, ordinary Rust 2,940 / zero failed / 164 ignored across 148 result blocks.
  Six explicitly selected offline suites/11 private cases passed, including the
  115-instance/113-unique product matrix. This is narrower than U15's historical
  twenty-case scope. All 622 protected inputs and 420 private files unchanged;
  all 17 bindings equal. Published 1f94808d5d9985b38fcf85021403bb4b05fea3e7;
  exact live/fetched remote ref, all 20 owned artifact blobs and 622 gated inputs
  read back equal. Receipt-only metadata/owned cleanup are tracked separately.
  U17 production manager/diagnostics/preview and U18 closing review
  remain open; no independent/native/Orca/ETS/alpha-release approval implied.



## AR07 scoped Float delivery — 2026-10-03 00:04 CEST

Scoped Float guard published/read back as da3bc9472610341a0d56bb13a6cfc016bb33eb2d.
Local/live/fetched refs equal, divergence0/0, eight owned source/receipt/doc
artifacts byte-exact. Actual-gated sourcebc5999c1 is unchanged by the doc-only
receipt. Four completed owned build/shadow directories removed; accepted and
rejected aggregate evidence plus next read-only audit retained. Shared dirty
root/U16 untouched; integrated root statistics remain with their owner.
Broader AR07 and Alpha remain incomplete; next bounded diagnostic/inert-work
budget proof is not yet executed, no new resource/semantic support claimed.


## AR07 Float actual merged checkpoint accepted — 2026-10-02 23:55 CEST

Actual integration bc5999c1 (source4514076b + published commissioningc9f77d7b),
proc_d4c3a0b0b43f exit0, independently reconciled20/20: workspace2931/0/164,
Web1559, intercepted Chromium61, private Dynamic6/0/0 and offline injected
SimTunnel HTTP13/0/0, zero unknown/genuine skips. All103 original product
archive and108 total original fixture identities/hashes unchanged; no private
raw logs. All615 source/config inputs frozen, fresh changed crates compiled,
17 shadow bindings equal, strict Clippy/build/fmt/deny/nonempty root gates pass.
Projection42 is a workspace subset, not additional passes. Both complete
handover/status owners preserved. Publication/readback still pending.

Broader AR07 remains open. Read-only source trace identifies a possible
general-diagnostic fan-out and inert-node/binding traversal budgeting gap:
activations_recorded(:1217) excludes ordinary diagnostics, diagnose(:1323)
pushes without admission, walk(:1463/:1532) can revisit inert nodes per
expansion, and bind_arguments(:1619) is a sibling work path. No behavioral
RED, new safety policy or production fix has run; do not label this reproduced
or accepted. Follow-up must cover the resource class, keep opaque sources and
explicit truncation/uncertainty, and use bounded public TDD before changes.
Nested ScopeKey/panel grouping and duplicate-module authority are source-traced;
full ModuleScopeDto ancestor provenance and UI/localization remain separate.


## AR07 Float owned final checkpoint accepted — 2026-10-02 23:09 CEST

proc_536a6eb16342 exit0 independently reconciled18/18 on owned guard/test delta
from d62baef4. Workspace2926/0/164, Web1559, intercepted Chromium61, selected
private Dynamic6/0/0, zero genuine/unknown skips, all103 original identities/
hashes unchanged, no private raw logs. All615 source/config inputs frozen,
17 bindings equal; two compiled min/max mutants caught unit+HTTP and canonical
hashes restored. Separate in-session source review no blocking bounded finding.

Current upstream commissioning c9f77d7b changes code outside the guard; scoped
commit, preservation/integration and actual merged-source gates/private offline
owner witnesses remain pending before push/readback. Broader AR07 budget/
module/provenance/vendor-inert audit and typed/localized UI/native/ETS parity
remain open, not closed by this leaf. Previous pending eighteen-stage receipt
is historical, superseded only for the owned pre-integration checkpoint.


## AR07 Float public baseline + guard mutation checkpoint — 2026-10-02 22:34 CEST

Corrected proc_e87d7afdd30d independently reconciled8/8: workspace2926/0/164,
strict workspace Clippy/build/fmt, real npm install/build, intended-root nonempty
anchors and whitespace. All615 source/config hashes frozen, 17 shadow bindings
equal. First baseline remains rejected/archived for absent Tauri frontend
resource before any tests. Both independently compiling finite min/max guard
mutants caught at unit and HTTP (4 observations); all615 canonical hashes
restored. Separate in-session source review no blocking bounded findings.

Final eighteen-stage checkpoint proc_536a6eb16342 / PID826351 dispatched with
classifier preflight, exact ignored inventory, six opt-in private Dynamic tests
(raw stdout discarded), source/input hash before/after and full public/UI/
repository/dependency gates. Actual candidate remains uncommitted on d62baef4,
frozen by source hashes/status, no mutation until receipt. No acceptance inferred
before reconciliation. Final integrated/private/UI checks and publication pending;
this leaf does not close broader AR07 budgets/module/provenance/vendor-inert
audit or typed/localized unsupported token/native/ETS parity. Earlier pending
public-baseline/mutation entries are historical, superseded by this measured
receipt; no private acceptance inferred from ordinary164 ignored tests.


## AR07 remaining audit / synthetic validation RED dispatched — 2026-10-02 21:32 CEST

Controller final receipt d62baef4 published/read back, old owned worktree/branch
and 13 completed build/shadow/review entries removed. Active broader-AR07
aggregate/public/failed evidence retained. Fresh alpha-parameter-audit starts
from that published checkpoint; no shared-root/U16 edits.

Existing Float validator allowed finite input under a NaN lower declaration.
Actual synthetic RED Rust101/0-1-0 accepted from proc_63fdb03535b1; minimal
min/max finite checks now in place. Targeted Float6/0/0, full HTTP34/0/0
independently verified, with ten metadata cases, nonempty project equality,
byte-exact retained source and independent sibling edits. Separate in-session
bounded producer review has no blocking findings; test-only format corrected.
First frozen baseline proc_370267152e1a rejected before tests: missing Tauri
frontend resource ../../knx-web/dist, workspace101/wrapper1. All615 source/config
hashes preserved; failed attempt archived. Corrected retry proc_e87d7afdd30d
pending with real npm ci/build prerequisites before the public workspace/strict
Clippy/build/fmt/root-explicit anchors/whitespace stages. No private inputs.
Delayed RED notification matches existing accepted witness, not a new run.
Mutations/restoration,
remaining budget/module/provenance audit and full integrated gates still pending.
Do not mutate frozen source or treat targeted GREEN as publication acceptance.
Official Rust finite-predicate source HTTP200 confirmed; no new KNX format/
encoding or private corpus claim. Remaining budget/module identity/validation
audit and UI token adoption stay open.


## Commissioning recovery: backup before Verify Mode — 2026-10-02

The opt-in service-control procedure now establishes its management connection
without a property write, reads PID_SERVICE_CONTROL and exactly one
PID_DEVICE_CONTROL octet, and persists both originals before setting Verify
Mode or changing bit 2. Recovery format 2 adds the original PID 14 octet;
legacy format 1 files are not rewritten or treated as complete setup recovery.
Wrong-scope authorisation refuses before any frame (including cleanup
Disconnect). An already-correct value performs no backup or property write.
Four simulator disconnect transitions clear only the Verify Mode bit, not
unrelated Device Control bits. CLI plan output lists the setup write explicitly.

Offline evidence: knx-net 425/0/0, backup 4/0/0, CLI 8/0/0, HTTP 12/0/0;
nine compiled behavioral mutants caught and restored. Full workspace gate
2929/0/164, Web 1559, strict Clippy/build/types/fmt/deny and nonempty repository
gates pass; 17 shadow bindings are semantically equal. This is a bounded
safety fix, not whole-device recovery, durable audit history, private-corpus
coverage, a new hardware run or complete Alpha acceptance. Details and final
publication state are in the commissioning recovery log.

## AR07 bounded controller integrated checkpoint — 2026-10-02 20:56 CEST

Source 00f23758 and integrated 03f18c95 (published U15/theme parent fe02deeb)
accepted after proc_dea67da354fd exit 0, independent 17/17 reconciliation:
workspace 2924/0/164, Web 1559, intercepted-API Chromium fixtures 61, selected
private Dynamic 6/0/0 with zero genuine/unknown skips. All 103 originals
unchanged, 606 source/config inputs frozen, six mutation-protected sources
unchanged, 17 shadow bindings semantically equal under CI policy. Full frontend
install/build, strict workspace Clippy/build, root-explicit nonempty repository
gates and dependency/format/whitespace checks pass. Projection42 is subset.

Two documentation conflicts preserved both owners and complete upstream
handover suffix; no source conflicts or manual Web/binding edits. Remote
publication/readback confirmed at 2d9aaeb8: live/fetched refs equal local HEAD
and all seven owned receipt/document artifacts byte-exact. Broader AR07 audit and typed/
localized UI token adoption remain open under U16 ownership; no blanket private
opaque-data, native desktop or ETS compatibility claim. Earlier pre-merge
pending notes describe their own dated receipts, now superseded in this scope.


## AR07 controller broad candidate verified — 2026-10-02 20:14 CEST

Corrected proc_ed20715c68e5 independently reconciled: 13/13, workspace
2,924/0/164, Web 1,357, six selected private Dynamic tests 6/0/0 with no genuine
skip signals; intentional reinstall/duplicate metadata classified separately.
103 original archive identity/hash entries unchanged, private raw stdout
discarded, all 596 source/config inputs frozen and controller mutation hashes
unchanged. Seventeen shadow bindings semantically match CI policy without Web
writes. Read-only in-session controller review: no blocking findings.

First broad classifier-only failure archived, not retrospectively accepted.
Fresh origin/main fe02deeb includes U15/theme work; integrate and re-gate that
actual candidate before publication. Broader AR07 audit and localized/typed
unsupportedControlKind UI adoption remain pending (U16 Web lock owned by
ui-theme-storage). This supersedes the earlier no-private/no-workspace receipt
status; no full ETS, native UI or all-opaque-data compatibility claim.


## AR07 bounded controller candidate — 2026-10-02 18:43 CEST

Public baseline reconciled at 417/0/6; its initial count-only failure is
archived. Stored Text controller RED/GREEN and ADR-0061 establish a conservative
Number/Restriction guard with the separately corpus-derived None exception
unchanged. Known other kinds receive UnsupportedControlKind; no matching/default
activation, bounded skipped-ref reports, preserved raw source, scoped diagnostics
and Undetermined projection. No domain/store schema or per-kind validator change.

Controller gate proc_b1e47be31700 independently reconciled at 16/16: public
ProductDB/server 1,118/0/57, five stored-kind cases, two server unit cases, HTTP
hidden-field refusal/independent sibling edit, six compiled behavioral mutants,
exact source restoration, strict two-crate Clippy/frontend build/fmt/whitespace.
593 source/config inputs frozen; Web/generated bindings untouched. The first
controller gate rejected a compile-only mutant and restored sources; not green
evidence. No private corpus, whole-workspace/integration or publication receipt
yet. The unsupportedControlKind warning/English fallback is on the backend wire;
manual Web union/catalogue adoption remains with UI, not completed localization.
See [parameter boundary](PARAMETER_SEMANTICS_BOUNDARY.md). AR07 broader audit open.

## 2026-10-02 17:14 CEST — AR07 offline source discovery; baseline pending

- AR06 final delivery receipt `0c3d6a8a` published/read back; task-owned AR06
  worktree/ancestor-confirmed branch and 333 scratch entries cleaned. No shared
  root/corpus changes. New isolated AR07 checkout starts from that receipt.
- [Parameter boundary](PARAMETER_SEMANTICS_BOUNDARY.md) records fresh local
  Condition_t evidence (printed/PDF page 30/64), current resolver/evaluator
  source path and already-delivered channel-label work. A stored Text/Float/
  unknown controller is categorized Comparable today; targeted behavioral
  regression and diagnostic/contract decision precede any production fix.
- Seven-step public/synthetic baseline `proc_4e0a97d00922` dispatched using both
  shared gate locks, fresh target/shadow output and a source freeze. Private
  cases remain ignored. Dispatch is not passed evidence or AR07 completion.
- No UI/domain-storage change, Repeat/Allocator engine, vendor execution or
  live KNX action. External retrieval failures are logged, not proof that
  primary manufacturer evidence does not exist.
## 2026-10-02 — U15 theme runtime foundation (delivered, extension still open)

- Implemented bounded duplicate-aware JSON admission, exact v1/token contracts,
  safe complete value grammars and existing unrounded base/accent contrast.
  Settings/cache revalidation retains diagnostics without rewriting raw data.
- Added reversible DOM property ownership and existing theme/OS/accent/cache
  integration; implicit fallback no longer overwrites stored selection.
- Observed RED/GREEN covers parser, limits, cache, real hook and DOM lifecycle.
  Review fixes pass six files/206 tests and TypeScript; 37 unit controls plus
  one intercepted Chromium control caught, every temporary source restored.
  Deliberate new-file TS2322 detected. Earlier complete Web/9 Chromium passes
  precede the last review fixes and are not final candidate acceptance.
- Frontend alpha.3 changes version metadata only; no added/upgraded dependency,
  KNX Core, protocol, manufacturer model or backend format change.
- Candidate acceptance proc_56f26c5c70d5 passed all 15 steps: Web 1,559,
  intercepted Chromium 61, Rust 146 result blocks / 2,890 passed / zero failed /
  163 ignored; no missing-corpus markers. All 614 protected source/configuration
  fingerprints unchanged; strict lint/type/build/bindings/deny and repository
  gates pass. Combined acceptance proc_32a1aab25020 repeats all ordinary gates
  on de1bf652 and passes 22/22 steps: Web 1,559, intercepted Chromium 61, Rust
  2,916 / zero failed / 164 ignored / 146 result blocks, all twenty selected
  private offline cases and the pinned 115-instance matrix. All 614 protected
  files unchanged; complete upstream preserved. Published as 9d1ae19d;
  fetched/live ref, full tree, all 21 owned artifacts and zero outgoing commits
  verified. Closing receipt/owned cleanup follows; U16–U18 stay open.
- U16 durable
  transactions/file export, U17 production management/preview and U18 closing
  review remain open. [THEME_PACKS](THEME_PACKS.md) defines the exact boundary;
  no independent/native/Orca/full WCAG or release approval is inferred.

## 2026-10-02 16:59 CEST — AR06 scoped delivery verified

- Source f8b6f27e and integrated gates on 1404f39b published in 95e6bcb0;
  fetch plus live refs/heads/main readback matched local HEAD, divergence 0/0.
  Subsequent UI theme integration e98a0b58 and receipt changes are Markdown only
  versus the gated source. Final doc gates: 376 links / 237 Markdown files,
  389 well-formed headers, 157 at ceiling, 17 generated skipped; whitespace clean.
- Actual combined acceptance: 24/24 steps, 2,916 Rust passes / zero failures /
  164 ignored / 146 blocks; 1,357 Web unit and 52 intercepted Chromium passes;
  eighteen selected private offline cases and six raw cases without skips;
  595 source/config inputs frozen. Both complete upstream archives retained.
- AR06 scoped conservative implementation and four verification items are DONE.
  Genuine independent module/schema samples remain BLOCKED_EXTERNAL: three
  existing exports are two installations, not a complete compatibility matrix.
  Unknown legacy formats are refused, not implemented; raw labels are not
  tested Secure/ETS abilities; empty ProductDB native reopen is not enrichment.
- No full ETS/XSD/descendant-QName or durable-report claim, independent external
  approval, live KNX access or shared-root synchronization. AR05 stays unchanged.
  Task-owned scratch/worktree cleanup follows; aggregate evidence is durable
  here and in the import boundary contract. AR07 offline research is next.

## 2026-10-02 16:56 CEST — AR06 integrated receipt (publication pending)

- proc_8211942fb620 exited 0; all 24 expected steps/raw logs independently
  reconciled on merge 1404f39be3fc629788da158c4e43f459da5f2594, tree
  5ce951c8e5ae918b1b4e5bc23eb2ddf3000a8b09; 595 tracked source/config inputs frozen.
- Workspace: 2,916 passed, zero failed, 164 ignored, 146 result blocks. Web:
  1,357 unit and 52 intercepted Chromium fixture tests passed. Eighteen selected
  private offline cases and six existing raw catalogue/producer cases ran without
  skips. Strict workspace Clippy/build, format, dependency policy, shadow binding
  semantic comparison and all four nonempty intended-root repository gates pass.
- Headers: 389 well-formed, 157 at the existing ceiling, 17 generated skipped;
  anchors: 376 links / 235 Markdown files; layering: 448 resolved packages.
- In-session whole-diff review closed both Important findings with RED/GREEN and
  compiled behavioral mutants: destination opening before master admission and
  an unsaved domain seed in refusal evidence. Not independent external approval.
- New upstream e98a0b58 contains two documentation-only UI theme commits. Preserve
  their complete handover/status archives, then rerun doc gates before publication.
  This receipt supersedes preceding pending combined-gate wording, not the scopes
  of historical receipts. No shared-root edits, original/corpus pin changes,
  full ETS/XSD/descendant validation, durable-report or live KNX approval.
- Scoped implementation/verification is complete; actual publication is pending.
  Three genuine exports still represent two installations; independent module/
  unimplemented-schema samples remain BLOCKED_EXTERNAL, not waived or compatible.

## 2026-10-02 — AR06 whole-diff review checkpoint (offline; 15:52 CEST)

- Corrected-CLI proc_129928b4f8dc exited 0; all eighteen steps/logs reconciled:
  workspace 2,910 passed / zero failed / 164 ignored / 146 result blocks;
  eighteen selected private cases and six existing raw tests ran without skips.
  Fifteen source paths frozen (fourteen changed plus protected unchanged mapper).
  Strict workspace Clippy/build and nonempty intended-root repository gates pass.
- Separate in-session review of all fourteen changed Rust diffs found and closed
  two Important findings: product DB opening before unsupported-master admission,
  and a refusal test's unsaved domain seed. The first has missing-target/sentinel
  regressions and a caught premature-open compiled mutant. The second reproduced
  `NotSaved`, now saves a nonempty project and checks whole-model equality after
  each refusal; a compiled destructive-store mutation is caught. Sources restored
  byte-exactly. Corrected ordinary app service: four passed, zero failed, three
  ignored; changed-app strict Clippy, format and whitespace pass. First seed
  orchestrator's wrong expected test count is archived, not a product failure.
- No remaining blocking in-session code finding within the conservative contract;
  not independent external approval. Final strengthened-fixture candidate
  proc_0c225d9b5f0e exited 0; all eighteen steps reconciled at 16:14 CEST:
  2,910 workspace passes / zero failed / 164 ignored, eighteen selected private
  and six raw cases without skips; fifteen protected sources frozen. Current
  upstream has additional UI/monitor source changes; combined gates/publication
  remain pending, not inferred from candidate acceptance.
  AR06 remains IN_PROGRESS; genuine independent sample gaps stay BLOCKED_EXTERNAL.
  No full ETS parity, report persistence, standalone-package-wide atomicity,
  descendant/QName/XSD validation, UI/foreign-root or live KNX claim.

## 2026-10-02 — AR06 mapping/native and raw-field checkpoint (offline; 14:34 CEST)

- Renewed proc_2296901dfdb3 exited 0; six compiled behavioral mapping mutants
  killed and source restoration equals owned HEAD. Eight step logs reconciled;
  four fixture/test files frozen. Three-crate tests: 170 passed, zero failed,
  76 ignored, 31 blocks; strict Clippy, format and whitespace pass. First Clippy
  failure is archived; no production allowance or mapping change was needed.
- Exact DefaultLine cases cover both mapper generations/project tables 11/21/23;
  repeated device-local refs retain distinct owners, flags/text and links, and
  malformed refs do not erase later objects. Whole-model/opaque native reopen
  exercises 24 synthetic combinations with/without an empty product DB.
- Eight explicitly selected private offline tests passed without skips, including
  actual enrichment and the exact existing schema-21 empty DefaultLine finding.
  Three existing exports still represent only two documented installations.
- Audited raw attribute/producer/catalogue boundaries and reran six existing
  catalogue/producer tests, all passed. [Contract](IMPORT_BOUNDARY_CONTRACT.md)
  records the sample matrix, normalized-versus-byte retention, optional producer
  extraction and transient-report limits. No Secure capability, new real schema,
  complete corpus or independent module sample is inferred.
- AR06 IN_PROGRESS: complete feature review, renewed changed-candidate workspace
  gates, integration/publication remain pending. Missing genuine samples remain
  BLOCKED_EXTERNAL; upstream UI changes are not yet integrated/certified here.

## 2026-10-02 — AR06 changed legacy candidate wider receipt (offline; 13:16 CEST)

- Exact registry handle `proc_11bf8a0ab11f` exited 0; all twelve expected steps
  and raw log counts reconciled. Workspace: 2,904 passed, zero failed, 163
  ignored, 146 result blocks. Strict workspace all-target Clippy, build,
  formatting/whitespace and four intended-root repository gates pass.
- Nonempty gate scope: 380 valid headers, 376 links / 233 Markdown files,
  448 resolved packages and 332 Rust corpus-source files. Thirteen named Rust
  files remained unchanged; no tracked Web/binding delta.
- Ten explicitly selected private offline tests ran, none skipped: eight CLI
  import cases, readable-product installation and real VD2 refusal. Not a whole
  corpus-matrix run, new independent ETS/schema sample, password or bus claim.
- Current artifacts are `ar06-legacy-wide-summary.json`, twelve named logs and
  aggregate verified receipt. Older counts belong to historical candidates.
  AR06 remains IN_PROGRESS: raw-field/sample/mapping and integrated delivery
  remain open. [Contract](IMPORT_BOUNDARY_CONTRACT.md).

## 2026-10-02 — AR06 legacy and CLI master checkpoint (offline; 13:02 CEST)

- Typed filename-only refusal covers VD3–VD5/PR3–PR5 and mixed-case inputs;
  VD2 stays unsupported. CLI preflight prevents destination creation/migration
  before refusal. Direct ProductDB VD2 hash/length behavior remains unchanged;
  CLI VD2 now uses the same early filename diagnostic as the other legacy names.
- Library/application/CLI regressions verify no legacy admission through modern
  bytes or known-hash retries, seeded database/WAL/source integrity, untouched
  reports and full native project/opaque equality after reopen. No legacy
  grammar, cipher, conversion or compatibility claim is introduced.
- Caller RED reproduced silent unsupported-master acceptance in products-only
  CLI ingest. Typed refusal now precedes every manufacturer write; two root
  variants preserve existing database bytes and prevent new hardware/DPT rows.
- Five compiled behavioral mutants killed and all three production sources
  restored byte-exactly. Renewed five-crate gates: 899 passed / zero failed /
  119 ignored / 70 blocks, strict Clippy/format/whitespace pass; tracked Web
  delta empty. Ignored fixtures are not corpus execution. Wider/private gates,
  sample/raw-field/mapping evidence and publication remain pending; AR06 stays
  IN_PROGRESS. [Contract](IMPORT_BOUNDARY_CONTRACT.md).

## 2026-10-02 — AR06 changed master candidate wider receipt (offline; 10:54 CEST)

- Exact proc_7228729035d2 completion: exit 0; all nine expected step exits and
  actual workspace results verified: 2,898 passed / zero failed / 163 ignored /
  146 result blocks. Strict workspace all-target Clippy/build, format/whitespace
  and four nonempty intended-root repository gates pass; seven Rust files frozen.
- Production files still equal restored mutation baselines; no tracked Web or
  binding delta. No ignored private fixture executed, no native report
  persistence or direct ProductDB/standalone compatibility is inferred.
- Master diagnostic/admission/native checkpoint below is backed by the current
  wider candidate, not the historical root receipt. Remaining raw-field,
  private-corpus/sample, legacy and DefaultLine/device-local AR06 rows stay open.
  AR06 IN_PROGRESS; no publication, release or live-bus claim. Contract:
  [IMPORT_BOUNDARY_CONTRACT](IMPORT_BOUNDARY_CONTRACT.md).

## 2026-10-02 — AR06 master metadata/admission/native checkpoint (offline; 10:42 CEST)

- Replaced suppressed optional-master detection errors with a typed finding
  forwarded through ImportOutcome. Fixed-label unsupported reporting withholds
  source values; missing master, valid comparison and container read failures
  remain distinct. No domain/storage/wire-shape or project-table expansion.
- Cross-layer RED proved a foreign master could still write a typed DPT row.
  Application now retains its opaque bytes without shared typed master ingest;
  canonical positive control preserves the existing valid typed path.
- Native regression verifies both product-DB modes, semantic `.knxdb` reopen,
  all opaque entries/master bytes/hash and unchanged synthetic input. This does
  not newly persist the transient report or certify descendant namespaces,
  standalone packages, direct ProductDB ingest or real independent ETS samples.
- Final three-crate gates: 162 passed / zero failed / 75 ignored / 31 blocks;
  strict Clippy/format/whitespace pass. Three compiled behavioral mutants caught
  and production sources restored byte-exactly. Renewed workspace/corpus/sample,
  legacy and DefaultLine/device-local requirements remain pending. AR06 remains
  IN_PROGRESS with its four full checklist items open; no release/bus claim.
  Contract: [IMPORT_BOUNDARY_CONTRACT](IMPORT_BOUNDARY_CONTRACT.md).

## 2026-10-02 — AR06 root candidate wider-gate receipt (offline; 10:04 CEST)

- Renewed proc_c5ad6d2638d7 exited 0; all nine expected steps and log counts
  read back independently: workspace 2,892 passed / zero failed / 163 ignored /
  146 result blocks. Workspace strict all-target Clippy/build, format/whitespace
  and four intended-root repository gates pass; five owned source files frozen.
- Fresh desktop initially lacked the built frontend resource. The first wider
  run exited 101 before tests; real pinned frontend build corrected the
  prerequisite. Its delayed notification is not a new candidate failure.
  No tracked Web source/binding delta or UI ownership change.
- Nonempty repository coverage verified; corpus source lint and ignored suites
  are not private-corpus execution. Remaining master diagnostic/lexeme, legacy,
  DefaultLine/device-local, native/sample and whole-feature requirements remain
  open. AR06 IN_PROGRESS; no publication, ETS parity, alpha-release or bus claim.
  Contract: [IMPORT_BOUNDARY_CONTRACT](IMPORT_BOUNDARY_CONTRACT.md).

## 2026-10-02 — AR06 root import-boundary checkpoint (offline; 08:51 CEST)

- Read the user-provided KNX Standard v3.0.0 PDFs as local primary evidence;
  schema-23 root/namespace/source-string findings are scoped in
  [IMPORT_BOUNDARY_CONTRACT](IMPORT_BOUNDARY_CONTRACT.md), not copied as licensed
  text or treated as an authoritative XSD/legacy grammar.
- Detection now verifies root QName and exact namespace identity, decodes XML
  attribute values without lossy UTF-8 replacement, refuses malformed attributes,
  and checks metadata/topology namespace agreement before table-based parsing.
  Synthetic full-pipeline positive/negative cases cover each boundary.
- Application-service regression seeds project and shared product databases;
  all six namespace refusals preserve main/WAL bytes and typed diagnostics,
  with/without the shared product DB. Two compiled behavioral mutants fail;
  production bytes restored exactly before fresh green tests/lint.
- Three focused crates: 155 passed / zero failed / 75 ignored / 31 result blocks;
  strict all-target Clippy, formatting and whitespace pass. No private-corpus,
  native-reopen, full-workspace, CLI/HTTP/UI or external compatibility acceptance
  is inferred. Master metadata diagnostic suppression and remaining AR06 source
  routes still need work. AR06 remains IN_PROGRESS; no alpha-release/bus claim.

## 2026-10-02 — U14 theme-pack contract (implementation pending)

- [THEME_PACKS](THEME_PACKS.md) and
  [ADR-0060](adr/0060-versioned-declarative-theme-packs.md) resolve the complete
  versioned JSON/token grammar, primary-source security evidence, limits,
  exact contrast roles, durable conditional settings and preview contracts.
- Existing theme/settings/language code was inspected; five primary-source
  bodies and their literal evidence were retrieved/verified. Separate
  in-session contract review findings were fixed before acceptance.
- U14 closes contract/research only. U15–U18 still need actual parser/runtime,
  persistence, UI, mutation and integrated-product gates; no built-in behavior,
  KNX Core/API or manufacturer format was changed.
- Fresh-target doc anchors and citation/whitespace checks pass. Original UI
  implementation, alpha dispositions, native/Orca/hardware boundaries remain.

## 2026-10-02 — Keyboard/modal/help-tip alpha follow-up (offline candidate)

- Search, Command Palette and Catalog Browser keep active keyboard rows visible
  without moving combobox focus; catalog highlight is distinct from selection.
- Shared modal background isolation is document-local, preserves prior attributes,
  handles nested/out-of-order close and dynamic DOM, and restores final focus.
  Initial/Tab-wrap focus filtering excludes hidden/inert/aria-hidden ancestors.
- HelpTip separates its permanent local description from a decorative painted
  portal; fixed viewport bounds/placement account for zoom and avoid clipping.
  Resize/scroll listeners and portals are cleaned on close/unmount.
- Seventeen new mock-only Chromium cases pass, including actual accessibility
  tree descriptions/background exclusion. Eighteen behavioral controls and a
  TypeScript new-file canary are caught with exact restoration. An initial
  early-release survivor prompted an immediate pre-observer assertion; the
  original whole-Web failure was the exact companion graph's missing new DOM
  helper, corrected without changing its API/project-mutation assertions.
- Separate in-session review fixes are complete; nine focused files / 117 tests
  and TypeScript pass. Renewed twelve-step proc_870fe2d19835 passes: Web 84 files /
  1,357 tests, 52 intercepted Chromium cases, Rust 146 result blocks / 2,890
  passed / zero failed / 163 ignored; 576-source freeze. First header failure
  stays failed; purpose/SPDX ordering corrected and measured ceiling lowered
  to 157 without relaxing it. Frontend alpha.2 changes no dependencies.
  All twelve gates repeated on integrated source as proc_490a156044df with the
  same counts/source freeze. Published 2e57f8e5, exact remote/tree/all 28 artifacts
  and zero outgoing commits verified. Narrow delivery receipt and owned cleanup
  follow; actual native and external prerequisites remain open.
  No native/Orca/network/sample/domain, hardware or alpha-release acceptance.

## 2026-10-02 — KL-82 authoritative monitor context candidate (offline)

- Added a read-only comparison of actual bus-session interpretation against
  current server project style/names/resolved DPTs; unavailable/busy/poisoned
  snapshots never round up to current. Monitor HTTP fields are additive and
  disclose neither project names nor host paths.
- UI uses only server evidence to establish freshness/project presence and
  blocks compose for missing/malformed/legacy evidence and poll failures.
  Local browser records are invalidation hints, not authoritative proof.
- Paused polls check context/status without rows or cursor advancement; Resume
  retains its cursor. Generations/session incarnation reject late replies and
  obsolete reattach errors. Original captured-row interpretation stays intact.
- Focused Rust/UI and monitored mocked Chromium tests pass. Eleven behavioral
  mutations fail as assertions and restore exact source. Separate in-session
  review findings are fixed; no independent external review is claimed.
- Twelve coordinated candidate gates pass: Web 83 files / 1,343 tests, 35 mocked
  Chromium cases; Rust 146 result blocks / 2,890 passed / zero failed / 163
  ignored; source freeze 570. Missing hint CSS failed the first gate, was fixed
  without weakening tests and its removal detected. Final comment clarification
  is non-runtime; repeated integrated twelve-step acceptance proc_f5cf67729adf
  passes with the same counts/source freeze. Source 8ceacf49 and exact remote
  ref/tree/twenty artifacts were verified, with zero outgoing commits.
  Keyboard/modal/help-tip contracts follow separately. This is not a hardware
  authorization, transaction-bound write guarantee or alpha release.

## 2026-10-02 — UI-owned alpha-readiness follow-ups (offline)

- Audited all 24 `goal-ui.md` rows in the alpha-readiness inventory against
  current source and retained contracts; U0–U13 were already closed and were
  not reimplemented. Per-ID evidence: [UI_ALPHA_READINESS](UI_ALPHA_READINESS.md).
- Catalog picker now offers only KNX product/ZIP packages; the project inspector
  exposes the existing undoable group-address-style command and consumes its
  authoritative response, with unknown/refused/pending cases covered.
- Main/companion settings periodically reread the authoritative server record
  and recheck on focus/visibility. Write generations and cancellation protect
  queued/failed edits, unknown keys and deletion intent; reads do not write back.
- Discovery retains raw Device Info across network client, HTTP projection and
  labelled, expandable UI details; absent adapter metadata is explicitly
  unavailable. No codec, CLI default format, identity or write authority changes.
- Eleven final coordinated gates pass: Web 83 files / 1,331 tests; Rust
  146 result blocks / 2,885 passed / zero failed / 163 ignored; type/build,
  strict Clippy/fmt and repository gates. Ten behavioral negative controls
  rejected realistic regressions with exact source restoration. Six scoped
  Chromium metadata cases pass; the first CSS-guard failure remains recorded.
- Repaired the default browser harness to serve fixture HTML on isolated Vite
  without a KNX backend/API proxy. All 33 cases pass, including fully intercepted
  full-app creation for each style. Configuration regressions and a rejected
  proxy-restoration mutant cover isolation. The first old-harness attempt was
  not purely mocked; its possible automatic read-only discovery is explicitly
  qualified in the owner receipt, never presented as live acceptance.
- Native Gtk/WebKitGTK 2.52.6 static-fixture geometry passed nine X11 width/zoom
  cases. It is not full Tauri, Orca, native-dialog or live-discovery acceptance.
  Existing invisible-tooltip overflow at 640 px is independently recorded,
  not introduced by metadata and not repaired by removing its AT description.
- Integrated source publication `6c16fe5a` and exact remote/tree readback verified;
  the eleven gates reran on that integrated source. KL-82/keyboard contracts
  and missing independent native/sample evidence remain open. No alpha release,
  full ETS compatibility, bus access or hardware write is authorized.

## 2026-10-02 — AR05 corrected-candidate acceptance (offline; 06:43 CEST)

- Shared master Languages evidence, byte-only v18 -> v19 migration and explicit
  rebuilding are fully gated without normalized-master replay or changes to
  historical installation snapshots. DPT collision/provenance audit and bounded
  Dynamic reporting retain their documented semantic limitations.
- Separate in-session review findings are closed, including repaired issue
  markers, protected census output, exact wrapper comparison and declared ZIP
  entry limits before construction. Compiled guard mutants fail behaviorally;
  sources are restored byte-exactly. No independent external approval is claimed.
- Final proc_f9f87cee4329 exited 0: all 20 expected steps, workspace 2,884 passed /
  zero failed / 163 ignored / 146 result blocks, ProductDB 585 / zero failed /
  24 ignored, Web 1,312. Strict Clippy, build/bindings/dependencies and intended-root
  repository gates pass. Each private matrix/real upgrade/census case executed
  with zero failure/ignored/skip markers; aggregate shapes and 595 guarded sources
  match. The intervening failed delivery remains a failed historical attempt.
- Technical acceptance and scoped implementation/audit publication verified at
  `04900fbc35b2daec5e766a32f99c263a04700e0e`; exact remote and all 25 owned
  artifacts match. Closing receipt `65b91777` published/read back; both owned
  checkouts/branch and task scratch cleaned with originals/foreign root preserved.
  AR06 source discovery has started, without new implementation/runtime acceptance.
  KL-86's source-winner provenance
  remains open, not a release waiver. No alpha-release, ETS parity or bus claim.
  Contract: [MANUFACTURER_REPORT_CONTRACT](MANUFACTURER_REPORT_CONTRACT.md).

## 2026-10-02 — AR05 master evidence and report-boundary checkpoint (offline)

- Shared pure master Languages evidence reports unconsumed attributes rather
  than declaring metadata interpreted. Current ingest, explicit rebuild and
  product-database v18 -> v19 call the same logic; no normalized master replay,
  retained-byte mutation or rewriting of historical installation snapshots.
- Missing/malformed/identity-invalid sources remain named, oversized unclassified
  sources explicitly unexamined. SQL failures are atomic, including deferred
  commit failure at final savepoint release; caller-owned transactions survive
  failed nested rebuilds. New deferred-commit test reproduced two retained rows
  before the correction, then passed with zero and released ownership.
- DPT package regressions distinguish first normalized values, retained losing
  declarations, collision counts and orphan semantic drops. Winner provenance
  remains limited (KL-86 not closed). Dynamic diagnostics use the existing mixed
  active/skipped budget and one truncation marker, not exhaustive enumeration.
- Post-mutation ProductDB gate: 25 result blocks / 578 passed / zero failed /
  23 ignored / zero skip markers. Warning-denied all-target ProductDB Clippy
  passed, changed crate compiled/checked. Two compiled behavioral mutants caught;
  production source hashes restored exactly. The ignored private tests are not
  counted as acceptance.
- Exact-scope shape census: 115 instances / 113 packages / 67 master blobs,
  canonical TranslationUnit RefId/Version each 1,870 occurrences on distinct
  masters. Scoped matrix, real retained-byte upgrade and renewed census passed
  after independent full-value/evidence reconciliation; all new rows are exactly
  TranslationUnit RefId/Version. Permanent corpus assertions and reviewed pins
  distinguish instance/package/source units; pre-assertion capture hooks removed.
- Final review reproduced stale issue markers after byte repair. Two new
  behavioral regressions now pass, including identity-invalid opaque data and a
  false-master intermediate. All nine rebuild tests and strict all-target
  ProductDB Clippy pass; cleanup-disabled mutant caught. Full integrated gates
  with renewed corpus acceptance, final sign-off and publication remain pending.
  AR05 IN_PROGRESS,
  AR06 not started; no release/ETS parity, UI or live-bus claim. Contract:
  [MANUFACTURER_REPORT_CONTRACT](MANUFACTURER_REPORT_CONTRACT.md).

## 2026-10-02 — AR04 storage fallback publication (offline; UTC)

- Exported `sync_after_command` delegates to the existing transactional full
  project writer for every command; no successful unsupported incremental arm.
  Production save paths/signatures, schemas and dependencies are unchanged.
- Behavioral parameter RED/GREEN; nine focused regressions and three new
  file-backed tests compare reopened native models, retained allocator marks,
  structural batch/history, sibling order, opaque/manifest preservation and
  late SQL failure. Three compiled mutants caught; all source hashes restored.
- Separate in-session review closed redundant fixture normalization and added
  explicit unchanged-post-apply-memory evidence on save failure. Not an
  independent external review or 50-variant runtime certification.
- Complete coordinated gate exited 0: 142 Rust result blocks, 2,859 passed,
  zero failed / 161 ignored / zero skip markers; private store 2/2; Web 1,312.
  Strict Clippy, typecheck/build, semantic bindings, dependency policy and
  repository gates green. 574 guarded source files unchanged; changed store
  compilation/check verified. Headers 373 valid / 159 absent / 17 generated;
  corpus policy 325 Rust files, target explicitly this candidate.
- KL-42/DATA-02/AR04 delivered as `216c673e7c32a4bd82a308e06544a4fd239d7b3f`;
  exact remote ref and source/contract artifact equality, author/committer and
  no-co-author policy verified. Closing documentation/owned cleanup follows.
  Contract
  [STORAGE_COMMAND_CONTRACT](STORAGE_COMMAND_CONTRACT.md), receipt
  `.ai/logs/2026-10-01_codex_alpha-storage-contract.md`. Triage is 110 headings,
  seven historical/resolved, 103 residual, 102 classified (5/30/54/13).
- No UI/editor, live KNX, parked ADR-0039 phase, import format or multi-user scope
  change. Canonical-root statistics remains foreign-owner blocked.

## 2026-10-01 — AR03 enforcement audit; AR02 publication receipt (UTC)

- Published AR02 `e691bc1318d0785289f8132378a0f26c9a829b27`: exact remote ref
  and artifact/tree matched the staged/gated candidate. Required author and
  committer verified, no co-author trailer. Owned checkout/branch, build
  targets and scratch removed; foreign root/reports left untouched.
- Docs-only audit [ADR0039_ENFORCEMENT_AUDIT](ADR0039_ENFORCEMENT_AUDIT.md)
  distinguishes completed phases 1–2 from the six remaining direct live
  allocator calls, single catalog-create assignment and post-command enrichment.
  The old nine-bypass narrative is historical, not current source evidence.
- Proposed three separate phases, IDs-only sealing and a disclosed heuristic
  gate. Activation unanswered: `KL-129`/AR03 remain `WAITING_DECISION`; not a
  user-approved deferral, new runtime test or implementation. Continue AR04
  storage guarantees independently. Canonical-root statistics stays blocked.
  Receipt `.ai/logs/2026-10-01_codex_alpha-command-audit.md`.

## 2026-10-01 — AR02 general ID exhaustion (offline; UTC receipt)

- All nine core allocators use checked addition and return `IdAllocationError`;
  `u32::MAX` is valid, repeat refusal is stable and never changes counters.
  All production callers propagate errors: ETS mapper/import, CSV planning,
  structural creation, parameter insertion and offline scan reconciliation.
  Existing parameter edits need no allocation. No native schema or DTO change.
- Nine behavioral REDs; final entities/counters save/reopen equality; CSV and
  reconciliation failure atomicity, maximum reservation rollback and monotonic
  undo/redo; actual CLI dry-run/write exit 2 and HTTP unchanged-project refusals.
  Mapper tests seed counters rather than constructing billions of entities.
  Three realistic compiled mutants caught; originals hash-checked/restored.
- Separate in-session review closed a direct mapper fixture compilation gap
  and an ADR-0018 header gap. The final stable-source gate passes: 141 Rust
  result blocks / 2,855 passed / zero failed / 161 ignored; 77 explicitly run
  private offline corpus/roundtrip tests / zero failed/ignored/skip markers;
  Web 1,312; strict workspace Clippy, dependency policy, typecheck/build,
  repository headers/anchors/layering/corpus and patch checks. Headers:
  371 valid / 160 absent / 34 generated; corpus policy 324 Rust sources.
- `DATA-01` and AR02 closed at this bounded contract; catalog preflight, U13
  fixes and all numbered limitation identities/counts preserved. This does
  not activate parked ADR-0039 phases 3–5, prove native GUI/live bus/ETS parity
  or replace independent whole-product review. AR03 activation was asked but
  no answer arrived; continue its bounded audit and then independent AR04.
  Canonical-root statistics refresh remains foreign-owner blocked.
  Receipt `.ai/logs/2026-10-01_codex_alpha-id-exhaustion.md`.

## 2026-10-01 — AR01 runtime gate target and coverage (offline)

- Repository gates use the exact runtime root (CWD or leading `--root PATH`),
  validate the Cargo workspace/members, print their canonical target and pin
  dependency metadata to it. They no longer use the build checkout as a root.
- Headers, anchors and corpus-gates fail on missing/empty required scan roots;
  generated-only headers do not qualify. Layering refuses absent or merely
  transitive policy roots and reports actual graph coverage. Existing scan
  heuristics/exclusions and AppImage artifact checks remain intact. See
  [verification invocation/coverage contract](VERIFICATION.md).
- Actual old binary after removal of its owned build checkout: exit 0 over
  zero sources; rebuilt code refuses the deleted target. RED/GREEN tests and
  five behavioral guard mutations verify runtime targeting and empty-scan /
  membership refusal. Sources restored byte-for-byte; compilation failures
  are not counted as killed mutants. In-session review findings fixed.
- Relevant gates: 75 unit + 12 CLI integration tests, zero failures/ignored;
  strict xtask Clippy, workspace fmt, layering (448 packages), headers
  (370 valid / 160 absent / 17 generated), corpus-gates (323 Rust files),
  anchors and patch checks. Lowered the header ratchet to measured 160 after
  adding a header to the touched layering module. No new dependency or product API.
- KL-130-GATE resolved with its original heading retained; KL-130-ZOOM remains
  open. Current triage: 110 headings, 104 residual / 103 classified (5/30/54/14),
  §105 unclassified. The 180-ID inventory is untouched. No full workspace/
  product, private-corpus, native-screen-reader or hardware acceptance claim.
  Canonical-root statistics refresh remains blocked by foreign report work.

## 2026-10-01 — AR00 alpha queue and decision provenance (offline)

- User started `alpha-release-goal.md`; its AR queue is the sole executor of
  overlapping non-UI/non-commissioning work. Historical `goal.md` dispatch
  text no longer redispatches resolved streaming, file gestures or U13 review.
- `ALPHA_READINESS.md` accounts for all 180 original IDs/priorities/routes,
  owner receipts, accepted decisions, exact missing inputs and safe fallbacks.
  The dated `OFFENE_PUNKTE.md` remains byte-identical. U13's original independent
  changes-required verdict and later fix-review provenance remain intact.
- Triage mechanically distinguishes 110 numbered headings from 105 residual
  boundaries; five solved/clarification rows are excluded, 104 are classified
  (5/30/55/14), §105 remains wire-evidence-only. Both §130 meanings and nine
  unnumbered section aliases are stable; no historic anchor is renumbered.
- Existing implementation/tests support KL-18/23/24 and ISSUE-04 closure;
  this package inspected them, not reran the whole product suite. Three
  technical rules (KL-70/88/134) lack evidenced user release-scope acceptance
  and remain waiting for decision; unscheduled FUTURE-05 is later work.
- Fresh worktree-built anchors: 375 links / 228 Markdown files, none dead;
  the 180-ID/owner/priority/triage validator and patch checks passed. No source,
  Web lock, private corpus, hardware action or release permission changed.
  Canonical-root statistics refresh remains blocked on foreign local report
  work and safe synchronization by its owner; no fabricated refresh is claimed.

## 2026-10-01 — Pre-write backup directory chains (offline)

- Both application backup writers now request directory-entry synchronization
  for the absolute supplied path and the resolved target's ancestor chains,
  after syncing file bytes and verifying readback. Shared spellings are visited
  once, in deterministic child-to-parent traversal order. Symlink and `..`
  components remain covered; relative paths are anchored without an empty
  ancestor. A directory or path-resolution error refuses the backup receipt.
- Public signatures, JSON versions, owner-only/create-new file protection,
  backup coverage and hardware write gates are unchanged. A failed sync can
  leave a file on disk, but that file is not returned as a durable receipt.
- Four initial RED failures reproduced leaf-only sync and missing-parent-error
  acceptance in both writers. A separate alias RED exposed the resolved-target
  branch. Sixteen backup-focused tests pass, including nine new regressions;
  leaf-only and canonical-only mutations fail their tests and are restored.
- In-session review caught an empty path becoming an implicit CWD receipt.
  Its additional RED refusal test now passes with explicit empty-path rejection,
  retaining the previous fail-closed behavior instead of silently normalizing it.
- This is an OS synchronization request, not a power-loss experiment,
  descriptor-pinned path confinement or whole-device recovery evidence.
- Delivery gates passed: 139 workspace suites / 2,818 passed / zero failures /
  161 ignored / zero `SKIP:`; explicit simulator CLI backup/restore 1/1 and
  private HTTP download fixtures 13/13. Strict workspace Clippy checked the
  changed application crate; fmt, Web-resource build, layering, headers,
  anchors (375 links / 226 Markdown files), corpus-gates and diff passed.
  Temporary corpus links were removed. No live bus or write authorization.

## 2026-10-01 — U13 closed: independent-review fixes (offline)

- The operator explicitly accepts the recovered independent GPT-6.1-Sol
  review instead of unavailable Claude; its original verdict was changes
  required, not approval. The implementation session's separate diff review
  is not presented as a new independent whole-track review.
- Device selection clears previous detail immediately. The Properties
  Inspector and centre workspace both reject mismatched detail IDs, so a
  pending/failed request or contradictory response cannot expose another
  device's editors. Existing out-of-order request checks are retained.
- Parameter loading also keys on the accepted authoritative project snapshot.
  Undo/Redo and other published commands refresh values without requiring a
  device/language switch; older overlapping GET responses remain rejected.
- Autosave scheduling invalidates the obsolete effect closure. A save that
  completes after disable/unmount/cadence change cannot arm a timer or replace
  the current cycle's timer; failed saves still report their failure.
- ISSUE-12 gains actual bounded UDP loopback roundtrip/no-response coverage
  through the production discovery exchange, extracted privately without
  changing public API, protocol, destination, HPAI selection or timeout.
  This is not multicast/native Search proof. RESEARCH §20.1 and limitation
  §79 retain the real-network boundary; no live bus/firewall operation.
- Integrated gates after concurrent backup-safety changes: 139 Rust suites /
  2,820 passed / zero failed / 161 ignored / zero `SKIP:` or skipping markers;
  Web 82 files / 1,312 tests; all six local Chromium mock suites (30 tests);
  TypeScript/build, strict workspace/all-targets Clippy, fmt, layering, headers,
  anchors, corpus-gates and diff checks green. Fresh task-owned target and
  read-only corpus link/environment prevented stale-root or silent-skip gates.
- Both ISSUE-12 boxes are ticked with exact test/boundary evidence; U0–U13
  are complete and this package's final handover releases the Web lock.
  Regression/mutation evidence, original nonmatching mutation-filter mistake
  and final outcomes are in `.ai/logs/2026-10-01_codex_ui-u13-fixes.md`.
  Native WebKitGTK/real screen-reader and global alpha/whole-goal acceptance
  remain outside this completion claim; hardware recovery/write gates stay
  fail-closed and no productive device operation occurred.


## 2026-10-01 — Download worker outcome and tunnel reservation (offline)

- Polling a finished server worker without a terminal device result now reports
  `failed`, rather than leaving a dead task `running`. Before the plan's first
  mutation boundary it reports `written:no`; afterward `partially` is explicitly
  conservative: any attempted write's outcome and connection cleanup are unknown.
  Existing events, backup path and already witnessed terminal results survive.
- The server keeps the download's tunnel reservation until the worker actually
  ends, including the await after a terminal device result. Cancelling a `join`
  wait no longer detaches the worker or releases that reservation.
- Eight synthetic offline regressions cover panic, task abort, silent return,
  read-only versus mutation progress, preserved backup/restart evidence and
  delayed cleanup. Initial RED tests reproduced four bugs; the additional
  cancelled-join regression was also RED before its fix. Four temporary guard
  mutations failed their corresponding tests and were restored.
- The expanded private HTTP-fixture run exposed a stale test assumption:
  group-address partial support for its original program is already verified.
  The acknowledgement-gate test now chooses an installed plannable program
  without shipped evidence and uses a fresh default-only project. It still
  rejects missing/wrong/other-target acknowledgements with zero tunnel calls;
  the exact acknowledgement opens one simulated tunnel. No evidence was
  downgraded and no sibling-program hardware success is claimed.
- Gates: full workspace 139 suites / 2,809 passed / zero failures / zero
  `SKIP:`; all 13 explicitly ignored simulator HTTP-download fixtures passed;
  strict workspace Clippy, fmt, Web resource build, layering/headers/anchors/
  corpus-gates and diff check passed. The first expanded fixture run was
  12 passed / 1 stale-assumption failure, corrected before this green run.
- This is server-lifetime outcome bookkeeping, not a cancel API, automatic
  restore, guaranteed disconnect or durable process-crash recovery. Public
  write gates, wire shape, domain model and Web sources are unchanged.

## 2026-10-01 — Failed memory download closes its management connection

- The shared memory executor now attempts one best-effort `T_Disconnect`
  whenever execution returns an error with a still-open management connection.
  CLI/server use this common executor; closing their outer IP tunnel alone
  previously left some pre-write property-read or partial-write errors without
  this cleanup. Already-disconnected failures do not send a second disconnect.
- Two RED tests reproduced the open-connection leak before and after the
  mutation boundary. They now pass across plain/observed/backed-up entries;
  a temporary bypass mutation fails both again. The 45-test executor suite
  also checks invalid-plan refusal sends nothing, failed backup persistence
  sends only one disconnect, and partial failure neither retries nor restores.
- The original error and caller-owned backup remain intact. This is connection
  cleanup, not rollback, proof of device recovery or cancellation safety:
  dropping an executing future or terminating the process cannot run this
  awaited cleanup. No hardware, key, gateway or public write availability changed.
- Gates: full workspace 139 suites / 2,801 passed / zero failures / zero
  `SKIP:`; private-fixture simulator backup/restore 1/1; strict workspace
  Clippy, fmt, layering/headers/anchors/corpus-gates and diff check passed.

## 2026-10-01 — Restore refuses incomplete load-state coverage (offline)

- `knx_core::commissioning::device_backup::restore_plan` now compares the
  backup's load-state machine list to the machines in the original download
  plan. Missing, duplicate and extraneous records are refused even if every
  memory region still has the right shape. State order is not a coverage claim.
- RED: two focused core tests failed against the previous permissive behavior;
  GREEN: three focused refusal tests and the wider core/app backup tests passed.
  A temporary bypass mutation made all three refusal tests fail, then was
  removed. The explicitly ignored simulator restore test using the locally
  available private product/project fixtures ran 1/1; dropping one load state
  now returns `OtherLoadStates`, while the complete backup still restores.
- Integrated gates on the isolated worktree: `cargo fmt --all -- --check`,
  strict workspace Clippy, `cargo test --workspace --no-fail-fast` (139 suites,
  2,798 passed, zero failed, zero `SKIP:`), layering, headers, anchors
  (375 links / 225 files), corpus-gates and `git diff --check` all passed.
- This strengthens existing *plan-scoped* K7 recovery files. It does not add
  hardware evidence, a full-device image, an address-write backup or a public
  K6/serial/K13 write path. Their pre-tunnel gates remain closed.

## 2026-10-01 — Active limitations and task backlog reconciled (docs only)

- Removed 38 wholly resolved or withdrawn numbered KNOWN_LIMITATIONS
  entries and one obsolete corpus-test note. Legacy fragment anchors used by
  repository links are retained. The active document has 110 numbered
  headings: 109 classified in LIMITATION_TRIAGE, §105 awaiting on-wire
  evidence; §130 occurs twice. Partially solved entries retain their actual
  remaining boundaries rather than disappearing with the finished work.
- ROADMAP now distinguishes completed Session 0–7 milestones from verified
  product scope and current safety gates. The goal files no longer dispatch
  K1–K19, U0–U12 or completed PDB/Paperclip packages. U13's independent
  review and ISSUE-12's two evidence boxes remain open; the chosen review
  attempt was service-refused before a verdict. ISSUE-04 is evidenced and
  checked. Confirmed public K6/serial/K13 address writes remain pre-tunnel
  refused pending action-specific durable recovery, while `1.1.32` has
  bounded read-only presence/identity evidence only (RESEARCH §24). The new
  Debug UI uses a property-only backup; no hardware or full-device recovery
  claim follows.
- Documentation only: no source code, device, firewall or product data
  changed. Anchor, triage-count and diff checks ran on this isolated tree;
  the native/UI and whole-product test suites were not rerun for the edit.

## 2026-10-01 — K6 alternate candidate: bounded read-only presence

- A dry run scoped `knx bus scan` to exactly `1.1.32`. One live read-only
  DD0 probe on the approved range returned `occupied`, mask `0701h`, exit 0;
  no locally observed competing KNX process/socket, other target, property
  read, access key or hardware write. RESEARCH §24 holds the scope and evidence.
- A **separate** one-target `live_identify` read-only test passed 1/1: the
  manufacturer-ID and hardware-type properties responded, while Device Object
  `PID_PROGRAM_VERSION` returned no elements. The manufacturer agrees with
  MDT local master data; a six-octet hardware-type prefix matches two possible
  local `.01` application-program compare records, one project-labelled.
  This does not identify the exact installed model or full application image.
- The operator replied “go” to the suitability/temporary-address question;
  actual button accessibility, complete recoverable storage and a concrete
  destination/write plan remain unverified. The K6 CLI/HTTP write gate stays
  pre-tunnel fail-closed; the previous `1.1.67` go is not transferable.

## 2026-10-01 — U13 independent review chosen, ISSUE-12 evidence reconciled for review

- The user chose the independent read-only whole-UI-track review by the
  goal.md/Claude session. This is a decision, **not** a completed review,
  approval of ISSUE-12 as out of scope, or a UI-track closeout. The Web lock
  is free. A review brief is in `.ai/logs/2026-10-01_codex_ui-u13-review-brief.md`.
- ISSUE-12's issue-plan U10 note and two unchecked boxes still describe the
  2026-09-28 no-response observation. Later RESEARCH §20.1 records the
  2026-09-29 gateway reply in the host UFW drop log and the user's 2026-09-30
  source-port firewall rule followed by successful unchanged CLI and HTTP
  discovery. KNOWN_LIMITATIONS §79 records both the host fix and Docker
  bridge limitations. No actual wire packet capture was taken and no new
  network/hardware operation was performed in this documentation pass.
  Independent review must assess whether the external fix and existing
  offline tests satisfy the open checkbox wording; until then they remain
  open and the completion condition remains unmet.

## 2026-10-01 — Experimental K6 decision; K7 offline interruption regression

- The operator approves continued K6 investigation **experimentally**, with
  no further manufacturer documents available. This does not prove the exact
  `1.1.67` model or affected storage, nor lift ADR-0059's pre-tunnel
  durable-recovery gate. Confirmed CLI/HTTP K6 writes remain unavailable.
  The active work target is `goal-commission.md`, not the UI goal.
- K7's optional interrupted-download scenario now has a simulator regression:
  the first `0701h` run writes one data region, disconnects before the second,
  and leaves the table `Loading`; a new session on that same simulated device
  repeats the full plan and reaches `Loaded` with both regions read back.
  This is **not** a live interruption, rollback, or verified K6 recovery.

## 2026-10-01 — K6 recovery-gate availability in the Web tab (published)

Published `76fa7e83f2e3a81062f5a1d68f9a8b3c61c2010b` to
`origin/main`; exact remote SHA readback matched before releasing the Web
lock. No public K6 write path was reopened.

- `GET /api/device-address/availability` projects the existing application
  recovery precondition as `{startAvailable, reason}`. It is read-only and
  neither checks device identity nor grants write permission; confirmed
  `POST /api/device-address/start` still applies the same guard independently
  and returns 412 before any tunnel. The simulated HTTP regression compares
  the GET reason to the POST refusal and checks zero connector calls and
  unchanged device state (RED 404 → GREEN 200).
- **Bus monitor → Program address** fetches availability before offering
  consent or a phrase. Unknown, failed or malformed responses fail closed;
  Retry explicitly rechecks the server, and a 412 after an earlier available
  response closes the action again. Historical status/stop rendering remains.
  EN/DE summaries explain durable-recovery requirements while the exact
  server reason stays visible. Six unit regressions and local mocked
  Chromium EN/DE at 360/1440 px (4/4) cover refusal, read failure/retry,
  malformed/contradictory results, preserved status/stop, 412 race, zero
  consent and no start request. Historical
  future-ready workflow tests mock `startAvailable: true`; they do not
  demonstrate an enabled production start.
- The user-guide's old present-tense write instructions were corrected in
  both the Bus tab and CLI chapter. No hardware, gateway, tunnel, key or
  device write was used. This does not lift ADR-0059, KNOWN_LIMITATIONS §116
  or the commissioning goal's live K6 recovery blocker.
- Gates: focused simulated HTTP availability test 1/1 and Web
  panel 16/16; full corpus-backed Rust 139 suites / 2,794 passed / 0 failed /
  161 ignored / 0 `SKIP:`; Web 82 files / 1,303 passed, TypeScript/build,
  strict Clippy/fmt, headers/anchors/layering/corpus gates and diff check.
  Local mocked Chromium K6 4/4, Site 4/4, Service Control 4/4, Device checks
  4/4, monitor 4/4 and ISSUE-09 10/10. No live device was tested.

## 2026-10-01 — ISSUE-04 completion evidence and last-save locale correction (published)

Published `912eb7685012bda0e355759d37d489bcfdefa8e1` to
`origin/main` with exact remote SHA readback before releasing the Web lock.

- The ISSUE-04 plan had five unticked boxes despite the server's saved-baseline
  tracking and the shipped autosave engine/settings. `http_project_routes.rs`
  now explicitly proves an edited project prompts before Save and not after,
  and that undo to the saved baseline is clean while a replacement edit forms
  a dirty branch (redo invalidated; timestamp unchanged). Its existing
  failed-save case proves no dirty/timestamp reset. These tests run without
  a gateway.
- `App.tsx::formatLastSaved` was using the browser's locale rather than the
  selected UI language. `App.test.tsx::uses the selected UI language and
  advances only after a successful save` failed RED on German UI with an
  English browser and passed after passing `useUiLanguage()` to the formatter.
  A rejected Save leaves the status text unchanged; the next accepted clean
  snapshot advances it. `autosaveSettings.test.ts`, `SettingsPanel.test.tsx`
  and nine fake-timer `useAutosave.test.tsx` cases evidence the already-shipped
  settings/countdown contract. No new persistence or protocol path was added.
- The ISSUE-04 plan now uses the actual `is_modified` field name rather than
  its former `is_dirty` shorthand. The older ISSUE-04 status entry remains
  the record for Save-and-continue behavior.
- Gates on this candidate: targeted `http_project_routes` 13 passed / 3
  ignored; full corpus-backed Rust 139 suites / 2,793 passed / 0 failed /
  161 ignored / 0 `SKIP:`; Web 82 files / 1,297 passed, TypeScript no
  diagnostics and Vite build green; mocked browser suites Site 4/4,
  Service Control 4/4, Device checks 4/4, monitor 4/4 and ISSUE-09 10/10.
  Strict Clippy, Rust fmt, headers, anchors, layering, corpus gates and
  diff check passed. ISSUE-12's two discovery fix/loopback boxes remain
  open pending wire/gateway evidence; U13 closing review needs the user's
  decision and is not asserted by this ISSUE-04 package.

## 2026-09-30 — Serial-address writes fail closed pending durable recovery (ADR-0057)

- `POST /api/device-address/by-serial` and the confirmed CLI
  `knx device address-by-serial` now refuse even a possible no-op **before
  opening any tunnel**: HTTP `412 Precondition Failed` / CLI failure. A typed
  confirmation, previous-address read and post-write response are not a
  durable, complete pre-write backup of affected storage. There is no bypass
  flag. Malformed addresses, serials and phrases still fail validation first.
- Read-only `find-serial` and CLI plan remain available; the plan no longer
  suggests that adding `--confirm` is sufficient to write. The MP §2.5
  procedure remains simulator-tested in `knx-net`, but public CLI/HTTP
  write success is deliberately unavailable until an action-specific,
  persisted and verified recovery contract is implemented. `serialAddress`
  stays `untracked` and overall activity coverage `partial`; no success is
  inferred from earlier simulation tests or prior live attempts.
- Simulated HTTP/CLI regressions verify pre-tunnel refusal for valid inputs,
  a possible no-op, and after a separately backed-up service-control bit-2
  change. No Web code, gateway or real device was touched. ADR-0057 and
  RESEARCH §22 distinguish protocol fields from unspecified device storage.

## 2026-09-30 — Phase-aware service-control write activity (ADR-0056)

- `POST /api/device/service-control` now records a bounded
  `serviceControlWrite` one-shot entry only after the existing Debug opt-in,
  target, confirmation, key-plan and gateway-conflict checks. A typed guard
  reports `running`, `noChange`, `notSent`, `effectUnverified`, `verified` or
  cancellation `unknown`. `verified` requires the same-session exact-octet
  property response; a transport error after the durable backup is only
  `effectUnverified`, even when the simulated transport rejected the send.
  `writeEvidence.backupRecorded`/`sendPossible` are flags, not a device receipt.
- The synchronous pre-write callback marks these flags only after the exact
  property's durable backup and readback succeeded. No property octets, mask,
  key, serial, gateway or backup path enter the activity snapshot. The write
  route still requires its separate confirmation and opt-in; telemetry adds
  no write capability. Simulated HTTP tests cover no-op, verified change,
  pre-send backup failure, transport failure both before delivery and after
  a simulated property mutation, and cancellation both before connection and
  at the write send boundary. The original property backup and HTTP response
  remain the recovery record.
- `serialAddress` and `groupWrite` remain `untracked`; `coverage: partial`,
  volatile/evictable history, no global UI integration and no live-hardware
  permission remain explicit. The serial-address HTTP route still needs a
  durable pre-write recovery design. No Web source or physical bus was touched.

## 2026-10-01 — ISSUE-06 Ground-root site affordance (published)

Published `7fd96bf1aef86b07a2d01bec86a52621978003ed` (feature) and
`3822a20c1bf9333eb66ded5303158a9a31470b74` (integrated evidence) to
`origin/main`; the remote SHA matched the latter before the Web lock was
released.

- Buildings overview now offers **Add site / property** for the first
  installation. It fixes the existing `NewBuildingPartRow` to `Ground` and
  calls the same validated `createBuildingPart` route as the generic form.
  Existing **Parent building part** uses `moveBuildingPart` for each building;
  no new type, command, storage migration or installation level was added.
- RED→GREEN `StructureWorkspace.test.tsx` covers a `Ground` root with two
  buildings in one installation, both reparenting commands, unchanged shared
  topology and one device projection per building. A wrong-kind mutation
  fails the test and was restored. Local mocked Chromium `site.e2e.ts`
  passed EN/DE at 360/1440 px (4/4) with no unmocked API call or bus use;
  existing mocked browser suites passed 4/4 Debug, 4/4 Device checks,
  4/4 monitor and 10/10 ISSUE-09. On the rebased tree Web 82 files /
  1,296 tests, TypeScript and build pass; corpus-backed Rust 139 suites /
  2,791 passed / 0 failed / 161 ignored / 0 `SKIP:`, strict Clippy,
  fmt, headers, anchors, layering and corpus gates pass. The existing
  synthetic `site_hierarchy.rs` and native store roundtrip establish owner
  and persistence fidelity; they are not evidence of an ETS Ground export.
  The first-installation-only editor and installation-rename gaps remain.

## 2026-09-30 — Write activity evidence contract (ADR-0056)

- Audited the existing serial-address, service-control and group-write routes
  against their protocol outcomes and recovery evidence. An HTTP 200 can be a
  no-op for the first two; a failed or aborted request does not prove that a
  write was never sent. A group-write payload echo is not a receiver readback.
  The current generic read activity states cannot safely represent these
  distinctions, so at that design stage all three writes were explicitly
  `untracked` and the aggregate stayed `coverage: partial`. The newer
  service-control follow-up above replaces only that route's untracked status.
- Simulated HTTP regressions cover successful-but-no-op serial/property
  requests, the property-specific backup on a real simulated change, and a
  group telegram whose accepted send is not mislabeled as a device receipt.
  No live hardware or Web source was touched. ADR-0056 defines the typed
  write-evidence prerequisites; it does **not** implement that API or grant
  write permission. The serial-address HTTP route has no durable pre-write
  recovery record and needs one before a future live write can be authorized.

## 2026-09-30 — Opt-in service-control read activity only

- After the server-side Debug opt-in, address/key-plan checks and gateway
  conflict checks, `GET /api/device/service-control` now records
  `serviceControlRead` with its validated target before connecting. Completed
  reads finish, transport or property errors fail, and request cancellation
  remains unknown. Refused requests never open a tunnel or enter the ledger.
  The snapshot never contains property octets, mask, project key or host path.
- At that stage `POST /api/device/service-control` was deliberately
  **untracked** pending a write-specific outcome contract. The follow-up
  above now supplies this contract without changing the existing opt-in,
  typed confirmation or durable property-specific pre-write backup/readback
  gate. Simulated HTTP tests covered disabled/refused, completed, failed and
  cancelled reads, and confirmed writes did not masquerade as reads.
  `coverage: partial` and all live hardware restrictions still apply.

## 2026-09-30 — Serial lookup joins the observed one-shot actions

- Read-only `GET /api/device-address/find-serial` now records a per-server
  `serialLookup` action after its validation and conflict checks, before
  connecting. A successful read, including no answering device, is finished;
  a transport failure is failed; dropping the request before a witnessed
  result leaves unknown. The activity target is **null**, not a serial number
  or invented physical address. It also refuses a busy/running scan before
  opening a second tunnel. Simulated HTTP checks cover two finished reads,
  connection failure, in-flight cancellation, malformed input, busy scan,
  missing target data and no device write.
- `untracked` still names serial-address **writes**, service-control and group
  writes. `coverage: partial` and volatile bounded history remain unchanged;
  no UI, hardware or authorization was changed. See ADR-0055.

## 2026-09-30 — First observed one-shot action: read-only device compare

- `POST /api/device-compare` now records its target and server-lifetime action
  ID before connecting; `GET /api/bus/activity` shows running, witnessed
  finished/failed, or unknown after request cancellation. The bounded
  in-memory list declares how many older entries were evicted. No comparison
  bytes, secrets, gateway or host path enter that activity record.
- This is not durable audit and not full one-shot coverage; serial-number,
  service-control and group-write routes remain untracked. ADR-0055 continues
  to require `coverage: partial`. The compare route itself is still read-only;
  no Web source or live KNX device was touched. Simulated HTTP tests cover
  completed, failed, cancelled and pre-tunnel refused comparisons.

## 2026-09-30 — ADR-0051 Debug service-control UI (scoped property action)

Published `ab31ca292f536602e6338692e04a39759566cd1c` (UI) and
`e63adad0a15985bdf50bb1522921c20635ce7322` (integrated evidence) to
`origin/main`; the remote SHA matched the latter before lock release.

- **Settings → Debug · device control** exposes the default-off
  `debugIndividualAddressWriteEnable` flag. Unlike ordinary optimistic
  preferences, this safety setting reads the server record, sends a serialized
  one-key patch, verifies the server's PUT response and reads the record back
  before showing it as enabled. A failed or contradictory response disables
  the control until it can be checked again. The server still refuses both
  service-control routes with `403` unless the saved key is exactly `true`.
- **Bus diagnostics → Debug · service control** does not open a tunnel on
  mount, input or review. An operator must explicitly read an existing
  individual address via an IPv4 gateway, inspect the two original property
  octets and mask, review the bit change, type the separate device-specific
  phrase, then request the write. The response must identify the same target,
  mask, reviewed pre-value and bit-only change; an actual write must report a
  recovery path. A mismatch or absent path withholds success and requires a
  fresh read. Setting the opt-in alone never changes a device or download.
- This UI consumes the K12 same-session property backup and exact readback;
  it does not add a generic write-activity receipt (ADR-0056) or create a
  whole-device image, automatic restore, permission for K13 reset or a live
  hardware go. Tests use mocked HTTP/local Chromium and
  the existing simulator route. See ADR-0051, KNOWN_LIMITATIONS §139 and the
  settings/bus user-guide chapters. Gate evidence: Vitest 82 files / 1,295
  tests, TypeScript/build green; local mocked Chromium EN/DE at 360/1440 px
  4/4 for the Debug action, plus 4/4 Device checks, 4/4 monitor and 10/10
  existing browser checks. Integrated workspace Rust after ADR-0056: 137 suites /
  2,776 passed / 0 failed / 161 ignored / 0 `SKIP:` with the corpus present;
  strict Clippy, fmt and all four xtask gates passed on the feature candidate. Removing the recovery-path or typed
  phrase UI guard made its corresponding test fail, then both guards were
  restored. No device was contacted by these gates.

## 2026-09-30 — Partial read-only bus-activity snapshot (ADR-0055)

- `GET /api/bus/activity` is a guarded, tunnel-free server view of the
  retained device-download, button-programming and line-scan sessions plus
  the monitor session while it has not been stopped.
  Download counts are derived from the same events as its detailed status
  route; no raw blocks, keys, telegrams or host paths appear in the snapshot.
  A held holder lock is reported in `busyLocks`, not mistaken for idle.
- The contract says `coverage: "partial"` and lists one-shot routes without
  retained activity evidence. It is **not** a global action history or proof
  that the gateway is free. UI status bar and one-shot instrumentation remain
  open under the other session's Web lock; no UI file or hardware was touched.
  HTTP tests cover empty/held-lock snapshots, GET-only semantics, simulated
  programming before/after and corpus-backed simulated download progress.

## 2026-09-30 — U12 device checks: offline readiness and explicit read-only comparison

- Added a **Device checks** tab to Bus diagnostics. Its offline `GET
  /api/device-readiness` view retains every device/grade, server counts,
  refusal category/detail, hardware evidence and nullable plan sizes.
  Unknown grades remain visibly unknown; ambiguous duplicate addresses
  stay in the table but cannot become a compare target. This corrects the
  `/api/readiness` shorthand in `goal-ui.md`: the actual mounted path is
  `/api/device-readiness`.
- A separate two-step action calls read-only `POST /api/device-compare` only
  after the operator chooses one uniquely addressed, plannable device,
  supplies a valid gateway and explicitly confirms the bus read. The UI
  requests the complete plan only; it shows the server's byte ranges and
  load states verbatim, withholds results on unexpected write/scope or
  contradictory difference counts, and invalidates stale project results.
  No setting, access key, write action or live gateway was used in this UI
  package. The optional partial API selection remains CLI/API-only.
- Regression: `BusDiagnosticsPanel.test.tsx` was RED before the tab was
  added; `DeviceInspectionPanel.test.tsx` covers readiness, unsupported and
  unknown data, duplicate targets, two-step consent, invalid gateways,
  server-contract mismatches, keyboard-scrollable tables and stale project
  requests. Two deliberate safety-guard mutations failed their tests before
  restoration. Web gate: 80 files / 1,270 tests; TypeScript/build green;
  local mocked Chromium EN/DE at 360/1440 px (4/4 new, 4/4 monitor and
  10/10 existing). Corpus-backed simulator route checks: offline readiness
  3/3, ignored-by-default device-compare 6/6 with no writes. No live bus
  connection was opened. On the rebased merged-equivalent tree including
  ADR-0055, Rust passed 137 suites / 2,769 tests / 0 failures / 160 ignored
  with no `SKIP:`; strict Clippy, fmt and all four xtask checks passed.
  Web passed 80 files / 1,270 tests, TypeScript and build, plus local mocked
  Chromium 4/4 Device checks, 4/4 monitor and 10/10 existing checks.
  See the bus user guide and KNOWN_LIMITATIONS' ADR-0049 section for the
  scoped hardware claims.

## 2026-09-30 — K12 property-specific recovery gate before service-control writes

- `knx-net` invokes a required pre-write callback after reading the exact
  `PID_SERVICE_CONTROL` value in its management session and before the first
  property write. A failed callback refuses without writing. CLI and HTTP
  persist the original two octets, mask and address in a versioned owner-only
  JSON record, read it back and sync file plus directory. The HTTP response
  reports `backupPath`; CLI defaults to `./device-backups/` and accepts
  `--backup-dir`. A no-op writes neither property nor backup.
- Simulator tests cover the pre-write refusal and HTTP/CLI failure paths;
  the HTTP test reads the saved record and checks it against the pre-write
  response. Focused suite and Clippy/check pass (details in the commissioning
  log). No live device was contacted; no UI file or K13 route was changed.
- ADR-0051 and KNOWN_LIMITATIONS §139 define the narrow property-only recovery
  scope and the ambiguous-write/manual-recovery boundary. A full device dump
  is **not** claimed. The UI owner may re-evaluate the Debug action after
  reviewing this contract; the default server gate remains off.

## 2026-09-30 — U12 / §147 received control fields in the web monitor

- The existing additive monitor API field `control` is now typed by the
  frontend. The telegram table and selected-row details show translated
  priority, hop count (including zero), and the observed repeat
  state only where the server supplies a boolean. `null` on other cEMI
  kinds and the closed-session marker, or absence from an older server,
  does not become a false claim. Unknown priority values are shown
  explicitly rather than classified as a known priority.
- `BusMonitorPanel.test.tsx` failed twice before the new column was added;
  the focused cases now cover true, false, null, marker, older response,
  zero hops, unknown priority and DE labels. A mutation that hid the false
  state failed its guard before restoration. Final Web: 78 Vitest files /
  1,258 tests; TypeScript/build green; local mocked Chromium 4/4 new and
  10/10 existing. Corpus-backed Rust: 136 suites / 2,761 passed / 0 failed /
  160 ignored, no `SKIP:`; strict Clippy, fmt, four xtask gates and diff check
  green. No live KNX bus was contacted. See KNOWN_LIMITATIONS §147, the
  bus-monitor manual and `.ai/logs/2026-09-30_codex_ui-monitor-control.md`.

## 2026-09-30 — U12 / ADR-0051 Debug UI paused at the safety boundary

- Read-only review of `service_control_routes.rs` and
  `knx-net/commissioning/service_control.rs`: the existing `POST` rejects an
  off/non-boolean/unreadable debug setting before opening a tunnel, checks a
  device-specific phrase, changes only bit 2 of the two-byte property and
  reads it back. It does **not** persist the pre-write value/recovery plan.
- The commissioning UI/status handover requests a backup audit before
  exposing other UI write paths. Whether a complete device image or a
  smaller property-specific recovery record is appropriate for this bit is
  unresolved. No Debug toggle/action was added in this package; the server
  remains default-off. Next: commissioning owner resolves and tests the
  recovery policy; the UI owner then implements the explicit action.
- Independent, read-only monitor control fields remain next in UI order.

## 2026-09-30 — Commissioning UI/status inventory and handover (no feature shipped)

- Read-only inventory: device download and button-address programming have
  panel-local progress/status only. The global workbench footer shows project,
  save time and version, not bus operations. Partial-download scopes have a
  server contract but no Web selector; K13 address reset has CLI only, **no
  HTTP route**. Serial-address, Debug Bit 2, readiness and device compare
  have HTTP routes but no corresponding Web controls.
- Detailed task and acceptance-test handover:
  `.ai/logs/2026-09-30_claude_commissioning-ui-status-handover.md`.
  **Safety blocker:** K13 must not be exposed as a write-capable HTTP route
  before a complete verified persistent backup of every affected device is
  guaranteed before the first broadcast write. A prototype lacking that
  gate was discarded; no server/UI implementation is claimed here.

## 2026-09-30 — U12 / §146 channel labels (UI follow-up)

- Channel groups keep the evaluated, opaque ownership key. Their heading
  prefers translated `text`, then verbatim `name`, then an explicit generic
  fallback. Where both text and name exist, the raw name is shown separately;
  a present `number` is always shown as text, including non-decimal values
  and `"0"`. Nothing is parsed, combined into `text`, or translated.
  Channel-independent and non-evaluated groups remain distinct.
- `DeviceWorkspace.test.tsx` reproduces the missing labels before the change
  and covers the two field combinations, untranslated textual numbers,
  blank values and groups without owners. The local Chromium fixture checks
  keyboard access and EN/DE rendering at 360 and 1440 px without a KNX
  server or bus. See ADR-0052 and KNOWN_LIMITATIONS §146.
- After the separate diff review and its three fixes: focused Web tests
  21/21; full Web suite 78/78 files, 1,256/1,256 tests; TypeScript/Web
  build and local mock Chromium 10/10 green. Corpus-backed Rust workspace
  136 suites, 2,761 passed, 0 failed, 160 ignored, 0 `SKIP:`; strict
  Clippy, rustfmt, diff check and all four `xtask` gates passed. These
  checks do not assert a real bus connection or hardware compatibility.
- The reviewed feature is published on `main` as `e8a3c56f`; the remote
  ref matched when read back. The publishing-equivalent tree repeated the
  complete Web, corpus-backed Rust, Clippy and repository gates. The web
  lock was released in the UI closeout handover.

## 2026-09-30 — Contributor License Agreement removed again (ADR-0054)

- At the user's decision, KNXBench stays `AGPL-3.0-or-later` with no CLA.
  `CLA.md` and `.github/pull_request_template.md` are removed; README, FAQ
  and the contributing guide have their earlier text back.
- ADR-0053 is superseded by ADR-0054, which records two unreviewed UrhG
  findings against the CLA (§ 40 written form for future works, § 32
  remuneration). KNOWN_LIMITATIONS §148 is withdrawn.
- No outside contribution was ever made under the CLA.

## 2026-09-30 — Contributor License Agreement (ADR-0053)

- The license stays `AGPL-3.0-or-later`. New [`CLA.md`](../CLA.md): each
  contributor grants the maintainer a non-exclusive license, including the
  right to license proprietary terms, so KNXBench can be dual-licensed. The
  contributor keeps the copyright. The maintainer promises the contribution
  stays available under the AGPL (fallback clause).
- Agreed by a fixed sentence and a checkbox in the new
  `.github/pull_request_template.md`. Checked in review; no bot.
- README, FAQ and contributing guide explain it: commercial use is allowed,
  a closed product needs a commercial license. The guide's old "no CLA"
  sentence is gone.
- Not reviewed by a lawyer; no organization agreement, no commercial license
  text yet (KNOWN_LIMITATIONS §148).

## 2026-09-30 — Partial download of parameters and group addresses verified on `1.1.67`

- Live (user "starte mit teildownload"): `knx device download 1.1.67
  --partial both`, 24 steps, 1416 octets each read back, all three parts
  `Loaded`, dump byte-identical; backup matches the pre-dump in every octet
  (RESEARCH §19.17, KL §142).
- `verified_downloads.json` gains `partial-both`; the shipped-evidence test
  now requires all four scopes (RED before the data change). Every download
  scope of `M-0083_A-0027-15-0BAC` is verified on hardware.

## 2026-10-01 — K6 public button-programming paths require durable recovery

- `knx device program-address` plan mode stays offline, but confirmed public
  CLI programming now refuses before runtime/tunnel opening. HTTP
  `POST /api/device-address/start` validates its address and phrase, then
  returns `412` before taking a lock or opening a tunnel. Neither route
  durably backs up the actually pressed device's complete affected storage
  today, so a phrase or an old application dump cannot authorize the write
  (ADR-0059; RESEARCH §24). The previous `1.1.67` live round trip remains
  historical evidence, not a reusable write allowance.
- A local UDP CLI regression and simulated HTTP regression check no tunnel or
  datagram on a valid confirmed request; direct simulated sessions retain
  status/stop, activity and exclusion tests without reopening public writes.
  No Web source or hardware was changed; the Web tab can display a phrase but
  cannot perform a confirmed write until the recovery design is implemented.

## Older entries

Entries dated up to 2026-09-30 that sat below the last October entry live in
[the September 2026 history](history/IMPLEMENTATION_STATUS_2026-09.md),
archived verbatim. New entries keep going at the top of this file.
