# IMPLEMENTATION_STATUS.md

## 2026-10-10 — Import expansion: combined-source integration accepted

Owner-authorized commit/merge/push follows the completed local package.
[Separate integration acceptance](status/2026-10-10-import-expansion-integration.md):
19 stages green; Rust3856/0, Web2592/0, Python54/0, Chromium236/236, private
registered corpus143/143 and four fresh real-API built-server browser cases.
Source inputs are unchanged through acceptance; final metadata readback and
exact remote-ref verification guard publication. IMPORT-02 remains externally
blocked; existing alpha.7 artifacts and hardware boundaries are unchanged.
Self-review only; original local evidence below is historical.


## Alpha.7 published and externally verified — 2026-10-10

`v0.1.0-alpha.7` is a public GitHub prerelease at frozen commit
`839d5afd90559f1435d77a05ece3bbce2de3f2cf`; the annotated tag will not move.
Both tag-triggered packaging workflows passed. Exactly three downloaded assets
(AppImage, standalone MCP and SHA256SUMS) match GitHub SHA-256 digests/sizes;
the checksum file passes and MCP executes with the expected build identity.
AppImage runtime extraction passes; no fresh packaged GUI/Orca acceptance.
Docker Hub version `0.1.0-alpha.7` and `latest` resolve to the same multiarch
index, with real amd64/arm64 manifests. Anonymous digest pull verifies exact
version/revision labels; the published image passes actual HTTPS/authentication/
project-save/reopen smoke. Both #1/#2 remain OPEN. Earlier failed registry
attempts and the pre-tag candidate verdict below remain dated history.
[Verified asset/registry publication receipt](evidence/release-alpha7-2026-10-10.json).
Native schema11 upgrade backups and all import/device/proxy limits still apply.

## Alpha.7 release candidate — 2026-10-10

All changes since Alpha.6 are packaged in version `0.1.0-alpha.7`: durable
project history/schema11, communication-object table, rename support, bounded
offline AP1 diagnostics, legacy improvements, TypeNone correctness, header badge
and Docker upload/browser-return fixes. Back up native files before migration;
Alpha.6 refuses the new store schema. Fresh accepted local18-stage gate:
Rust3831/0/185 ignored, Web2583/0/162 files, Chromium236/0, real-server5/5.
Separate private controls146/0; password-dependent legacy tests not freshly
selected. Runtime/dependency inputs remain equal through CI-only corrections.
AppImage dry run and both native Docker architecture build/smoke jobs passed;
an additional anonymous local AMD64 container smoke passed. Five workflow
regressions/full47 Python controls pass. No new hardware/whole-ETS claim.
[Source-bound release evidence](evidence/release-alpha7-2026-10-10.json).
Publication/assets/registry readback is the next explicit gate, not inferred
from those dry runs. Both reported issues remain open.

This is a **newest-first delivery history**, not the live feature inventory.
Older dates, failures, counts and compatibility milestones remain historical.
For current source capabilities use [manual implementation status](manual/implementation-status.md);
for practical boundaries use [known issues](manual/known-issues.md), and for
formal owner dispositions use [the ledger](status/LEDGER.md). A scoped delivery
does not erase a broader platform/device limitation or imply a new release.

## 2026-10-09 — Docker uploads and companion return: source fixes

- GitHub #1: file uploads now stream through a **256 MiB** per-file ceiling
  plus 16 KiB multipart allowance. Full-field staging/no-clobber and explicit 413
  preserve existing uploads; exact-limit/oversize/malformed regressions pass.
  Importer ZIP expansion budgets remain unchanged; no reporter archive supplied.
- GitHub #2: Diagnostics/Flow explicit browser return navigates this tab and
  read-only resumes the currently open server project/saved-file authority.
  Ordinary startup, native main-webview focus and source-bound Flow selection
  remain unchanged. A separate browser editor/its transient selection is not moved.
- Final **18-stage frozen-source** gate: ordinary Rust **3831 passed / 0 failed /
  185 ignored**; frontend **2583 passed / 0 failed in 162 files**; intercepted
  Chromium **236/236**; builds/three extra type configurations/Clippy;
  five nonempty explicit-root repository gates, documentation/tools/whitespace.
  No explicitly selected private-corpus sweep or new native/hardware acceptance.
- Sealed production server/frontend proof **5/5**: only lo; own HOME/XDG,
  catalogue and project paths; actual 115 MiB upload/SHA-256 equality and
  EN 1440 / DE 400 return/visible installation/unchanged projection. Screenshots
  inspected. Original negative controls catch admission, disabled return,
  focus-only URL failure and missing project resume. Self-review only.
- Initial smoke attempts inherited the normal catalogue opener; no pre-run
  hash exists and host-catalogue integrity is not claimed. Final isolated runtime
  acceptance is separate. Failed/canceled/superseded attempts remain recorded.
- [Contract](contracts/DOCKER_UPLOAD_AND_RETURN.md) and
  [receipt](evidence/docker-import-diagnostics-2026-10-09.json) bind scope/limits.
  Source fixes ready for review; **not in alpha.6 artifacts**.
  No new release, deployment, actual bus operation or full ETS compatibility.

## 2026-10-10 — Selective imports: original local acceptance

Historical evidence below describes the pre-publication package. The later
owner-authorized [integration acceptance](status/2026-10-10-import-expansion-integration.md)
is separate; original local counts are not presented as merge results.

- Uncommitted/unpublished owned worktree based on `b042048b`; devices or whole
  lines merge into the open project through shared application/HTTP/CLI logic.
  Explicit preview/retention consent, reference closure, conflicts/staleness,
  one-step native Undo/Redo and retained source evidence are verified.
- Fresh owning Rust **1331/0/97 ignored**, frontend **2577/0 in163 files**, CVEXC
  Python **7/0**, warnings-denied Clippy/formatting and production build pass.
  Private registered corpus and repository closure are recorded by stage in the
  [receipt](evidence/import-expansion-local-2026-10-10.json), not inferred here.
- Fresh isolated production Chromium: **four cases** (en/de,1440/400px), actual
  source picker/inspection/preview/consent/apply and exact Undo; **eight frames**
  inspected. Localized unnamed-installation labels preserve names and IDs.
  Backend diagnostic notes remain raw English. No API interception or real bus.
- CLI serial lookup now reads the recovered working snapshot read-only; saved
  root data cannot conceal imported source-identity ambiguity. Named RED/GREEN
  and whole owning suites pass; invalid setup/harness attempts remain separate.
- Authorized restore-export harness is ready, but real historical exports were
  not supplied. CVEXC analysis observes declarations without enabling runtime
  comparison/download rules or claiming signature/default/precedence knowledge.
- [Verification](status/2026-10-10-import-expansion-verification.md),
  [contract](SELECTIVE_IMPORT.md), [ADR0106](adr/0106-selective-project-import.md)
  and manual carry the local boundary. Self-review only; no main/release/native
  accessibility/full-ETS/hardware claim or publication authorization.

## 2026-10-09 — Communication-object table: final names/AP1/logo candidate

- Final integrated source `58e2852f` retains published device/project
  names, offline AP1 analysis and header logo. The earlier rename-only19-stage
  candidate below is historical and superseded, not borrowed as acceptance.
- All **21 fresh stages pass** with source inputs frozen: frontend **2568/0 in162
  files**, production build/four type checks; owning core/projection/server/app/
  productdb/CLI Rust **2737/0/118 ignored**, warnings-denied Clippy; five explicit-
  root repository gates, documentation and whitespace. This is not full-workspace
  or native/WebKitGTK/ETS/hardware certification.
- Both explicit full105 Release private-corpus regressions pass; only aggregate
  counters leave private evidence. Production CO Chromium **19/19**, complete
  intercepted Chromium **227/227**, isolated real-server/demo-copy **12/12** pass.
- A prior source stopped on two immediate Porcelain contrast reads during the
  shared120ms button background transition. A diagnostic confirmed interpolation;
  final tests wait actual animation completion, preserving the same contrast
  assertions without sleeps/retries or a product-code workaround.
- Fresh ProjectStats records the clean `58e2852f` source and its real
  collection timestamp. Original local receipt/images remain unchanged; final
  [publication receipt](evidence/communication-objects-publication-2026-10-09.json)
  binds this exact source. Normal main push/readback remains a separate runtime
  completion check. Candidate `2e759b77` was normally pushed and verified
  against fetched/live main at 2026-10-09 19:37 CEST; this docs-only closure retains
  the exact accepted source. No force push, release, deployment or bus operation.

## 2026-10-09 — Communication-object table meets the rename desk (main delivery)

- Owner authorized commit/main integration/push. Feature `ddd4d2cc`, initial
  integration `cd28d704`, upstream name-workflow reconciliation `92ec5407`.
  Four documentation conflicts retained both packages; table ADR renumbered to
  **0102**, upstream name ADR-0101 preserved. Existing Name fields/F2/context menus
  coexist with the device object table. Original local receipt/images stay exact.
- Fresh combined19-stage gate: **2558 frontend passed / 0 failed in161 files**,
  production build/four type checks; **1527 owning Rust passed / 0 failed /45 ignored**
  (core/projection/server ordinary tests, not a full-workspace/private-corpus run),
  warnings-denied owning Clippy/server build. Production Chromium **19/19**,
  full intercepted Chromium **227/227**, actual isolated real-server/demo-copy
  **12/12**, only lo/no interception/unchanged snapshot/original demo.
- All five fresh checkout-bound repository gates/docs/whitespace pass; source
  manifest1004 frozen throughout. First integration attempt refused two unchanged
  monitor-test5000ms timeouts; diagnostic71/0 and a complete maxWorkers2 run retain
  original timeouts. A queued old-source retry was stopped before execution when
  upstream changed; it is not counted as acceptance.
- Fresh ProjectStats describes the clean combined `92ec5407` merge,
  not the stale shared root; canonical preferences/report are preserved. Source
  self-review only. [Publication receipt](evidence/communication-objects-publication-2026-10-09.json)
  and [contract](COMMUNICATION_OBJECT_TABLE.md) record exact commits, measured
  scope and remote readback. No release, deployment, real bus or native/ETS claim.

## 2026-10-09 — Communication objects get headings, not a guessing game (local)

- Owner-approved Q1–Q12 / local implementation go: explicit device CO table,
  channel/flat views, separate Name/Function/DPT/GA/Status columns, search and
  AND-combined evaluated-status/link/effective-DPT family/exact/missing filters.
- Stable original/numeric/natural sorts, match counts, channel reveal/reset and
  UI-only state lifetime. Open editors preserve dirty drafts/errors/pending through
  view/channel/refresh changes; nonmatching editors remain marked exceptions. One
  in-flight admission and delayed-result invalidation reuse existing core/API edits.
- Final source-frozen gate: frontend **2537 passed / 0 failed, 157 files**;
  build/four type configurations; production Chromium **19** and full intercepted
  Chromium **227**, all green. Fresh owned server/xtask build, all five nonempty
  explicit-root repository gates, docs/manual navigation and whitespace pass.
  Separate real production server/demo-copy proof **12/12**, only lo/no API
  interception, unchanged project snapshot and original demo digest.
- Named RED controls covered absent UI/derivation, dirty refresh, removal notice,
  filter reapplication, selected-view contrast and 167px stretched provenance
  badges. Final screenshot review: badges now compact; toolbar/headings readable;
  narrow controls wrap and table columns remain available through local scroll.
- Synthetic 1,000-object production action-to-visible sample: render 2052ms,
  flat view 2908ms, filter 458ms, sort 377ms; includes driver/layout overhead,
  one sample on this host, not a guarantee or 1,000 expanded editors.
- Initial real runtime mistakenly inherited XDG and opened the existing local
  v22 catalogue; missing scanner column refused installation. Package remains
  absent and main-file mtime predates run; no pre-run byte hash/repair claim.
  Corrected final runtime fully isolates HOME/XDG/project data. Rejected harness
  attempts and original chain exit1 remain disclosed, not relabelled as acceptance.
- [Contract](COMMUNICATION_OBJECT_TABLE.md), ADR-0102,
  [aggregate receipt](evidence/communication-objects-2026-10-09.json), architecture,
  manual, roadmap and limitations synchronized. Self-review only; no subagents.
  **Uncommitted/local only**, not main/alpha.6/deployed; core/storage/API/protocol
  and dependencies unchanged. No real-bus/native/Orca/full-ETS acceptance.

## 2026-10-09 — Header logo: the K gets a proper badge

- Replaced the workbench header's letter K with the cropped, transparent
  symbol from the owner-supplied PNG. Local bundled asset, original gradient
  and aspect ratio; adjacent KNXBench name and overview navigation retained.
- Login screen, project/domain data, protocols and dependencies unchanged.
  [Asset derivation](assets/README.md#application-header-mark).
- Named component regression reproduced RED before implementation. Final
  frontend suite: 2545 passed in 159 files; TypeScript/production build pass.
  Built Chromium: six Graphite/Porcelain/LCARS cases at 1440/400 px, resolved
  themes, loaded image, contained aspect ratio and zero document overflow.
  API calls intercepted, including discovery; no server or hardware operations.
- Feature `3858b394` integrated no-ff as `d58cf56f`, then reconciled with
  current main in `a17c08d3` without changing the original logo implementation.
  Actual integrated gate: frontend 2554/0 in 160 files, production build,
  theme/flow types, intercepted Chromium 208/0, six built logo cases,
  five nonempty repository gates, documentation and whitespace pass.
  [Publication evidence](evidence/header-logo-publication-2026-10-09.json).
  No release or deployment.

## 2026-10-09 — Offline AP1 integration keeps its bus pass offline

Feature38d8eb45 integrated with current native-history/name-editing main;
actual merge `d639c5f8` fully re-gated: Rust3827/0/185
ignored, web2553/0/160 files/build/four typechecks, Chromium208/0, both explicit
full105 Release regressions1/0/0, four real built runtime cases/eight verified
reduced ZIPs. Source freeze1144, original/readiness invariants, Clippy/four builds/
bindings/five repository gates/docs pass. Fresh integrated ProjectStats included.
Main push explicitly authorized; see [delivery/readback](status/2026-10-09-offline-ap1-verification.md).
No new executable plan, live support, release/deployment/hardware or root sync.
Earlier local-only receipt below stays dated evidence; review remains self-review.

## 2026-10-09 — Offline AP1 evidence: order without a bus ticket

- Local, unpublished first `MV-07B0` `Load/ap1` diagnostic slice, ADR-0098.
  Package-bound retained sources, exact namespace/canonical master path,
  ambiguity/hash refusals, ordered unknown steps and separate unplaced
  declarations. No executable plan, new live eligibility or hardware Verified.
- Existing support-gap UI/API/CLI and preview/consented ZIP/manual maintainer
  handoff reused. Detailed local identities/values are removed from reduced
  reports; fixed issue-code counts remain shareable. No automatic transmission.
- Real corpus corrected an invented direct-mask synthetic wrapper: canonical
  `HawkConfigurationData` fixtures now exercise the real path. Full-105 original
  regression: 75 selected IDs/results, 69 partial / 6 source-limit unavailable,
  **zero expanded real sequences or complete plans**. Original hashes and
  467-program readiness (1 Verified / 98 Untested) unchanged; counts pinned.
- Final Rust ProductDB/App/server/CLI 1945/0/118 ignored; fmt, warning-denied
  focused all-target Clippy, builds, five nonempty repository gates and docs pass.
  Both explicit ignored Release corpus regressions 1/0/0 each. Web 2494/0 in
  154 files and production build pass, bound by unchanged web inputs.
- Real built unmocked EN/DE, 1440/400px, dark/light loopback-only browser workflow
  passed; eight downloaded reduced ZIPs byte/hash-checked, including unknown-step
  metrics with no source values. Representative screenshots inspected. XML
  declaration, namespace-amplification and attribute-boundary RED controls closed.
- Self-review only; no full-workspace/native/accessibility/hardware acceptance,
  commit, main integration, push, release or deployment. Source/configuration
  frozen. [Contract](OFFLINE_PROCEDURE_RESOLUTION.md),
  [receipt](evidence/offline-ap1-resolution-2026-10-09.json).

## 2026-10-09 — Names can change; identities keep their day job

- Project-local device and individual GA names: editor/Properties, F2 and
  context menu, every installation, exact Unicode and duplicate display names.
- Dedicated name-only core/API path leaves IDs, addresses, flags/DPTs, links,
  placement and refs alone. Context/revision guards, failed drafts, pending
  ownership and read-only lost-response reconciliation prevent blind overwrites.
- Exact native undo/redo and reopen use existing history, no schema migration.
- Local candidate: Rust 1950/0/75 ignored; frontend 2544/0/159 files;
  intercepted Chromium 208 and real built server/UI four DE/EN wide/narrow
  cases pass. Clippy/server build/desktop compile/types/tools 42/five gates/docs
  pass. Self-review only; not native/Orca/private-corpus/hardware acceptance.
- [Contract](contracts/project-name-editing.md), ADR-0101 and
  [verification](status/2026-10-09-project-name-editing-verification.md).
  Owner Go authorizes main integration/push after gates; no release/deployment.
- Integrated candidate `f309612b`: full Rust workspace 3790/0/184 ignored,
  frontend 2544/0/159 files/build/types, Clippy and all five repository gates,
  docs/tools green. Exact merge-tree equivalence retains the final four native
  and 208 browser executions without mislabelling them as fresh merge runs.


## 2026-10-09 — Native history remembers restarts, not hardware writes

- Owner-authorized isolated HISTORY-01/HISTORY-02 package: native v11 durable
  working snapshot and undo/redo, separately saved baseline, named/save/safety
  versions, explicit revision/incarnation/generation-bound restore/delete/clear.
  Opaque bytes/manufacturer references retained; shared context counted once.
- Core stays independent of UI/SQLite. Immediate transactions and read-only
  admission preserve source/visible state on refusal; foreign schema objects,
  corrupt/future envelopes and limits refuse instead of silently trimming.
  Global line order now persists independently of area sibling order.
- DE/EN File/command-palette history panel with persistence disclosure, version
  search, explicit confirmation, safe cancellation and stale-response guards.
- Self-reviewed local candidate: Rust 3762/0/182 ignored; Vitest 2521/154 files;
  Chromium 208; fmt/Clippy/build/bindings/five repository gates/docs and tools 42
  pass. Private reference 3/3 and selected native corpus 5/5; actual built server
  and production workbench pass two SIGKILL/reopens and nine assertions offline.
  Two final footer wording fixes were re-gated; all Rust/configuration unchanged,
  desktop rebuilt and real smoke repeated. No independent-review claim.
- [Requirement matrix and boundaries](status/2026-10-09-project-history-verification.md),
  [aggregate receipt](evidence/project-history-2026-10-09.json), ADR-0100 and
  [manual](manual/user-guide/02-projects.md). Source commit recorded in closure.
  Same-file versions are not independent backups; native accessibility, power
  loss, hardware/ETS recovery and packaged release are not established.
  Original feature receipt is local; user subsequently authorized main integration.
- Integrated candidate `b81c17cf`: Rust 3781/0/184 ignored, Vitest 2523/155 files,
  Chromium 208, fmt/Clippy/four builds/bindings/five gates/docs/tools 42 green;
  reference 3/3, selected native 5/5 and real production crash/reopen assertions
  9 green again. Upstream histories/catalogs preserved; fresh ProjectStats on
  clean merged source. Final publication in CURRENT_STATE, no release,
  deployment or hardware operation.


## 2026-10-09 — Bathroom button: loaded, checked, put back (RESEARCH §19.21)

- Maintainer go for `1.1.14`, MDT fourfold button
  `M-0083_A-0026-15-3591`, mask `0701h`. Complete project-program
  download: 25 steps, 1562 octets read back, three Loaded parts,
  restart unconfirmed. Separate read-only baseline and post-dump cover
  1570 octets including physical address and six load-state bytes.
- User-operated light check: main and mirror-light on/off telegrams on
  configured groups, plus extra blind telegrams (physical movement not
  inferred). Own monitor stopped; restore from the pre-write backup,
  1562 octets read back, independent final dump identical to pre-dump
  at all 1570 selected octets. No full-device/RAM rollback claimed.
- Third shipped evidence program, `complete` only; scoped unit test RED
  before entry, GREEN afterwards; a `partial-parameters` evidence mutant
  is rejected by the named scope assertion and restored. House readiness
  11 verified / 21 untested / 2 unsupported / 1 excluded; only `1.1.14`
  was tested, `1.1.15` inherits the program grade, not live verification.
- Self-review: evidence/tests/docs only, no protocol, schema, UI or executor
  changes. Raw configuration/backups stay gitignored. RESEARCH §19.21,
  §7/§136 limitations, manual scope synchronized.
- Initial scoped gate: app/CLI/network Rust 839 passed / 0 failed / 56
  ignored, fmt, focused all-target Clippy, all five nonempty fresh-target
  xtask checks, whitespace exit 0. Release `house_readiness` and historical
  pinned-103 `download_coverage_corpus`: each 1 passed / 0 failed.
- Integrated acceptance on `44654e04`, with the parallel full-corpus
  package: app/CLI/network Rust 839 passed / 0 failed / 56 ignored,
  fmt, focused all-target Clippy, all five nonempty fresh-target xtask
  checks, documentation and whitespace pass. Release full-105
  `download_coverage_corpus` and `house_readiness`: each 1 passed / 0 failed;
  all 105 originals and integrated source hashes unchanged. Receipt:
  `docs/evidence/live-button-114-2026-10-09.json`.
  No frontend/browser/full-workspace or partial-download claim.

## 2026-10-09 — Download coverage main integration: the whole suitcase, again

- Owner follow-up authorizes merge, commit and push. Feature `bd778821` merged
  without conflicts into fetched main `b3ee55a0` as `d12bce07` (no-ff); complete
  merged tree equals the accepted feature. The original local-only receipt
  remains unchanged; delivery evidence is separate.
- Fresh gate on the actual merge: full Release `download_coverage_corpus`
  1 passed / 0 failed / 0 ignored, all 105 original packages unchanged,
  467 programs (1 verified / 98 untested). knx-app 159/0/27 ignored,
  ZIP boundaries 13/0, fmt, focused Clippy, documentation, all five fresh
  nonempty checkout-built xtask gates and whitespace pass.
- Self-review, no subagents. Test/docs only; no default-limit, production,
  UI, server, hardware, tag, release or deployment change. Shared root and
  parallel sessions remain untouched except the scoped handover update.
- Receipt: `docs/evidence/download-coverage-publication-2026-10-09.json`;
  accepted main delivery `bf023312` read back with local/fetched/live refs equal,
  feature ancestry and exact receipt bytes verified. Final closure is docs-only.

## 2026-10-09 — Full download coverage stops tripping over the big ZIP (KL §151)

- The explicitly invoked ignored `download_coverage_corpus` test now uses the
  existing bounded `PackageLimits::LARGE` (ADR-0082). Every one of the current
  105 product ZIPs installs; no vendor exception or catch-and-continue. Standard
  production/web bounds remain unchanged. The original full Release baseline
  reproduces the typed size refusal; the large-profile measurement exposes the
  expected stale coverage pins before they are deliberately updated.
- Full final Release corpus test: 1 passed / 0 failed / 0 ignored. 467 programs:
  1 verified, 98 untested; refusal counts are pinned in RESEARCH §19.20.
  All original inputs rehashed unchanged. Per-program private diagnostics removed;
  only aggregate coverage is printed. Offline/default-value planning is not new
  hardware evidence, ETS equivalence or project-specific compatibility.
- Gate: knx-app 159/0/27 ignored, ZIP boundaries 13/0; fmt, focused Clippy,
  all five nonempty checkout-built xtask gates, documentation and whitespace pass.
  No full workspace, frontend, server or bus run. Self-review, no subagents.
- Receipt: `docs/evidence/download-coverage-large-2026-10-09.json`;
  research index/topic, KL §151 and `.ai/` handover updated. Local focused delivery;
  merge, push and release are not part of this test-fix request.

## 2026-10-09 — The attic presence detector goes live and comes back unchanged (RESEARCH §19.19)

- First live download of a second program: the house project's
  configuration to Eibmarkt presence detector `1.1.8`
  (`M-006A_A-0001-22-617E-O0079`, mask `0701h`) with the maintainer's go.
  Compare before (23 of 530 octets differ: 21 flag octets, §19.13, and the
  brightness threshold at `4194h`), pre-write backup, 25 steps / 530 octets
  read back, three parts `Loaded`, restart unconfirmed as on `1.1.67`;
  independent compare after 40 s clean; bus monitor saw `1.1.8 -> 2/0/35`
  switch on and off with the maintainer in the attic; restore from the
  backup, after which the compare lists the same 22 runs as before.
- `crates/knx-app/data/verified_downloads.json` gains the program with scope
  `complete` (unit test `the_shipped_evidence_names_the_presence_detector_complete_only`,
  red before the entry). Readiness of the house: 9 verified (`1.1.1`–`1.1.9`),
  23 untested, 2 unsupported, 1 excluded (was 32 untested). The legacy
  `.vd4` program (L4) was not downloaded and stays untested.
- Docs: RESEARCH §19.19, KNOWN_LIMITATIONS §7/§136, manual status,
  supported/unsupported and known issues.
- Found on the way, not fixed (corpus owner): `download_coverage_corpus`
  fails on the full private corpus since the 2026-10-08 Siemens files
  (`Siemens_HVAC_…ETS5_ETS6.knxprod` refused by the ZIP size limit, KL
  §151). With the 103 pinned packages it passes
  unchanged (1 verified there: the detector program is not in that corpus).
- Gate (`RUST=1` web-package gate on `59f4df15`, inputs frozen): build,
  tsc, flow study, theme fixtures, Vitest 2485/153 files, Chromium 202,
  fmt, Clippy, Rust 3746 passed / 0 failed / 184 ignored, five xtask checks,
  `diff --check`: all exit 0. Ignored corpus tests run separately:
  `house_readiness` green; `download_coverage_corpus` green on the 103
  pinned packages, red on the full corpus as described above.

## 2026-10-09 — Legacy programs learn to download, on paper (ADR-0094, L4)

- `knx_productdb::legacy::legacy_program_code` reads a program's
  `s19_block` rows into `ProgramCode` (segments with base image and mask,
  one load procedure, address/association/group object table placements);
  `code::load_program_code` uses it for legacy programs, with identity and
  parameter placements from the database (absolute addresses into the
  holding segment, unions on one shared placement). `CodeError::
  LegacyProgram` is gone; `CodeError::Legacy { cause }` names the row.
  Image builder, planner, readiness and executor are unchanged.
- Rules measured against ETS's conversions (research note *L4*): control
  code = record's first octet, `(LsmIdx << 4) | event`; records checked
  against their columns (allocation records carry the end address); mask
  `01h` reads `FFh`; TaskCtrl1 from the record; table limits from
  `ADDRESS_TAB_SIZE`/`ASSOCTAB_SIZE`; task-segment identity from the program
  row (the nine house devices report it, not the record's). Merged-procedure
  rows, control code `05h`, unknown segment types and record-less Compare
  Property/TaskCtrl1 stay named unmodelled steps.
- L2 correction found by the oracle: `PARAMETER_ADDRESS` 0 is "no memory".
  It had merged unrelated parameters into one cell; the `.vd5` now publishes
  41,817 parameters (was 38,453), the `.vd3` 452 (was 302).
- Acceptance (ignored, `knx-app/tests/legacy_download_oracle.rs`): N000520's
  code equals ETS 6.3's conversion; the house's nine presence detectors plan
  identically apart from `4196h`–`4197h` (L2 deviation 3, named); nine
  Siemens `070nh` programs yield ETS4's code apart from four unmapped
  `string` parameters. `legacy_corpus` and `legacy_oracle` green with the new
  pins.
- Fixture `src-vd-program` carries a 20-step procedure; MARVIN plans end to
  end (`legacy_publish.rs`), a damaged record is refused by `BLOCK_ID`, and
  the server's readiness grades the legacy device `untested` (or names the
  negative signed value it will not write). Copy in CLI/web says untested
  instead of "not supported". Mutation sweep 14/14 killed.
- Not done: no live legacy download (needs a device go); payload re-parsed
  per code load.
- Gate (on `5dd021bf`, fresh target, offline, inputs frozen): Vitest 2485
  tests in 153 files, build, tsc, flow-study and theme fixtures; cargo fmt,
  Clippy `-D warnings`; Rust workspace 3745 passed / 0 failed / 184
  ignored; all five xtask checks; `git diff --check`. Intercepted Chromium:
  201/202 in the gate, the miss a 30 s timeout in
  `diff-virtual.e2e.ts` (2,000-step scroll; no web code it touches
  changed); rerun offline right after: that spec ×3 12/12 and the full
  suite 202/202. The ignored corpus tests (`legacy_download_oracle`,
  `legacy_corpus`, `legacy_oracle`, release) ran green before the gate.
  Rebased onto `45546575` (docs and workflow only). In-session self-review
  only.

## 2026-10-09 — The 173 MB `.vd5` gets measured, then let in (ADR-0094, VD5)

- Layout: a legacy file has exactly one EX-IM member and may carry others
  (the real Siemens `.vd5` is an installer tree with three mask images).
  Others are listed in `LegacyContainer::other_members` /
  `LegacyInspection::other_members`, never read, kept in the stored
  original, and reported as `unread-member` on every publication (also on a
  repeated one; not stored as `legacy_diagnostic`). Two EX-IM members are
  refused. The layout rule now requires all member records to tile the file
  from offset 0 to the central directory.
- Bounds from measurement: file 128 MiB, payload 256 MiB; EX-IM defaults
  value 64 MiB, 2,097,152 continuations, 4 M rows, 24 M values. The parsed
  document is dropped before the publish transaction (peak 1,580 → 1,426
  MiB for the real file); the inflate reservation is capped by deflate's
  maximum ratio.
- Real `.vd5` (release build): inspect 2.1 s / 492 MiB; import 22–38 s /
  1,426 MiB; through the built web app and a release server 22.1 s, server
  peak 1.46 GB. 88 programs, 129 catalog items, 71,467 parameter refs,
  55,381 object refs, 288,413 translations; all 88 programs evaluate with
  only `NoBranchMatched`. Not mapped yet: 1,515 parameters of atomic types
  3 (`string`) and 5 (`long enum`), reported. First charset evidence: ten
  0x80–0x9F bytes read as text only in Windows-1252.
- Web: the legacy import report folds its notes by kind (`<details>` with
  counts), so 1,775 notes stay readable; none is dropped. CLI says "import
  notes" and lists other members in `inspect-legacy`.
- Tests: installer-tree fixture `marvin-installer.vd5` (built by
  `build_fixtures.py installer`), container layout/bounds/limits tests,
  knx-app and server installer-tree imports, `LegacyInstallReport.test.tsx`,
  the ignored corpus pins for the `.vd5` (inspect and publish). Mutation
  sweep 10/10 named (layout, single-member, unread report on both paths,
  both bounds, three parser limits, inspection).
- Gate (on `9305b0e4`, fresh target, offline, inputs frozen): Vitest 2485
  tests in 153 files, build, tsc, flow-study and theme fixtures; intercepted
  Chromium 202; cargo fmt, Clippy `-D warnings`; Rust workspace 3734 passed
  / 0 failed / 182 ignored; all five xtask checks; `git diff --check`. The
  ignored corpus tests ran green beforehand (`legacy_corpus`, release).
  Rebased onto `a1fcc8f2` (docs only). In-session self-review only.
## 2026-10-09 — Docker Hub gets its name badge

- Owner request: populate the empty description of
  `knxbench/knxbench-server` with README/manual-style engineering humour.
  Dedicated English overview in `apps/knx-server/DOCKERHUB.md`: features,
  real-app GIF, pull/run quick start, persistence, tags/platforms, HTTPS,
  network scope, safe updates and honest alpha/ETS compatibility limits.
- Separate metadata-only workflow publishes short description and overview
  on scoped `main` pushes or manual dispatch; pinned Node-24 action,
  explicit byte-limit checks and anonymous exact-source readback.
  Optional description-only token preserves the release credential's scope.
- Local verification: actionlint syntax, fresh-target `xtask check-anchors`,
  documentation/33-chapter navigation and staged whitespace pass; all 10
  overview links/media targets return HTTP 200. An isolated probe of the
  published alpha.6 image served HTTPS health/frontend, used a persistent
  named volume and loopback-only port, logged its certificate fingerprint
  and stopped with exit 0. Probe container/volume removed. Self-review only;
  no full application suite or real-bus test for this metadata-only package.
- Published on `main` as `a9459b95`; automatic
  [sync run 37896835083](https://github.com/KNXBench-Labs/KNXBench/actions/runs/37896835083)
  passed with the existing credential. Anonymous API readback equals both
  committed texts exactly; `latest` and `0.1.0-alpha.6` digests unchanged.
  Public Chromium rendering: seven content checks, GIF loaded, introduction
  and quick-start screenshots visually inspected. Docker Hub's third-party
  telemetry and anonymous profile errors are excluded, not product failures.
  [Acceptance receipt](evidence/dockerhub-description-2026-10-09.json).
- No app, image, release tag, running user container, project or bus changes.
  [ADR-0097](adr/0097-docker-hub-release-image.md) and the
  [Docker manual](manual/user-guide/11-web-and-docker.md#the-docker-hub-overview)
  document the maintained source and credential boundary.

## 2026-10-09 — `v0.1.0-alpha.6`: first release built by CI, first one on Docker Hub

- Owner go for the release. Preparation: `fe7e0e26` makes the tag
  release job create pre-releases (`--prerelease`, matching title) for tags
  with a pre-release part; `042509c6` bumps desktop, web (package + lock), CLI,
  server (from `alpha.2`) and `knx-mcp` (from `alpha.1`) to `0.1.0-alpha.6`.
- Gate on `042509c6` (root worktree): fmt, workspace Clippy `-D warnings`,
  Rust 3726 passed / 0 failed / 182 ignored, Vitest 2480/152 files,
  `tsc` + Vite build, all five xtask checks, tools unittests,
  `check_documentation.py`, diff check. Dry runs on the same commit:
  [AppImage 37891579110](https://github.com/KNXBench-Labs/KNXBench/actions/runs/37891579110)
  (first CI run of that workflow: build, version check, Xvfb and headless
  Weston start, `knx-mcp`) and
  [Docker 37891581261](https://github.com/KNXBench-Labs/KNXBench/actions/runs/37891581261)
  (amd64 + arm64 smoke tests, `knx-server 0.1.0-alpha.6+g042509c`).
- Tag `v0.1.0-alpha.6` on `042509c6`, annotated, KNXBench tagger. `main` had
  meanwhile received the `TypeNone` spacer fix (`7f1efbb2`); it was not gated
  with the release and is **not** in alpha.6.
- [Release](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.6)
  (pre-release, hand-written notes): `KNXBench_0.1.0-alpha.6_amd64.AppImage`,
  `knx-mcp-x86_64-linux`, `SHA256SUMS`; downloaded again, `sha256sum -c`
  OK, `knx-mcp --version` = `0.1.0-alpha.6+g042509c`
  ([run 37893327233](https://github.com/KNXBench-Labs/KNXBench/actions/runs/37893327233)).
- Docker Hub ([run 37893327218](https://github.com/KNXBench-Labs/KNXBench/actions/runs/37893327218)):
  public `knxbench/knxbench-server:0.1.0-alpha.6` and `:latest`, one index
  `sha256:dfacb646…6703` with amd64 + arm64 (plus attestations). An anonymous
  pull of the amd64 image passed `smoke-test.sh`.
- Docs moved from "alpha.5 is current" to alpha.6 (README, installation,
  project status, manual status, known issues, Docker/AI/Devices chapters,
  FAQ); AppImage recipes use `sha256sum -c --ignore-missing`, because
  `SHA256SUMS` now also lists `knx-mcp`. Not tested: the alpha.6 AppImage on
  a real desktop session and the arm64 image on a Raspberry Pi.

## 2026-10-09 — TypeNone rows stop pretending to be input fields (KL §128)

- Kind `None` (ETS `TypeNone`, legacy atomic type 0) has no value. The
  parameter panel (`apps/knx-server/src/domain.rs`, `carries_value`) now
  reports such a field as `editable: false` with no `writeEtsId` and adds no
  warning, because a row without a value is not a refusal. Writes stay
  refused by name, unchanged.
- Web: `ParameterPanel.tsx` draws a kind-`None` row as a heading (its `text`
  only, never the internal name) or, with empty text, as blank space
  (`aria-hidden`). The check is by kind, so even a server that still claims
  the row is editable gets no input. An empty spacer never counts as
  untranslated.
- Real data: the Eibmarkt `.vd4` (N000520) through a local server and the
  built web app, offline: 71 fields, 11 kind `None`, 0 editable; the page
  shows 2 headings (`###`, `2`), 9 spacers and 60 inputs (71 − 11).
- Tests: new `http_parameter_type_none.rs` (ETS `TypeNone`, 2 tests), the
  legacy heading in `http_legacy_device.rs`, three Vitest cases. Realistic
  reverts of the server rule and of the web branch each turn named tests red.
- Gate on `579d5071` (fresh target, offline browser namespace, inputs
  frozen): Vitest 2483/152 files, build, tsc, flow-study and theme fixtures;
  intercepted Chromium 202; cargo fmt, workspace Clippy `-D warnings`; Rust
  workspace 3728 passed / 0 failed / 182 ignored; all five xtask checks;
  `git diff --check`. In-session self-review only.

## 2026-10-09 — Pages catches the Node-24 bus; offline demos get a signpost

- Owner approved commit/push/public website verification. Pages actions:
  checkout v7, configure-pages v6, upload-pages-artifact v5; deploy-pages v5
  already uses Node 24. Official resolved action metadata plus the upload's
  pinned Node-24 subaction checked. Hidden-file inclusion retains `.nojekyll`.
- Root/DE/EN Demos navigation and home/residential/office downloads, combined
  ZIP, English guide and checksum links. Frozen 1.0.0 ZIPs remain byte-exact;
  fictional/offline scope, catalogue setup, reset copies/auto-save and retained
  candidate labels disclosed. Anonymous downloads: four ZIPs plus checksum
  file, all 200 and exact repository bytes; guide 200.
- IPv6 HTTPS GET via four Globalping probes (DE/NL/US/GB): 200 with authorized
  TLS, two distinct Pages IPv6 addresses. Direct local IPv6 is unavailable
  (no public route), not a site defect. IPv4 HTTP→HTTPS remains 301.
- Local gate: website 22 / story 67 unittest, preview/release builds,
  Chromium 444 named checks across seven recipes (134 demo handoffs per mode),
  all five fresh-target repository checks and whitespace. EN desktop / DE
  mobile choices visually inspected. Browser socket startup refusal and
  mobile CSS grid regression were corrected before the final full pass.
- [Evidence](evidence/pages-demos-2026-10-09.json), [website contract](WEBSITE.md)
  and demo index updated. In-session self-review, no application/core/package
  modification or new native/ETS/hardware acceptance. Published source
  `77ab6e97`; Pages run `37890928052` build/deploy successful. All 34 live
  files plus manifest exact; live Chromium 134 checks pass. Four additional
  post-deployment IPv6 probes return 200/valid TLS and the new Demos navigation.
  Concurrent Docker changes preserved by rebase; only docs conflicts, source
  invariant verified and affected gates rerun. Closure metadata is docs-only.

## 2026-10-09 — Release tags publish the server image to Docker Hub

- Owner request: releases publish the container automatically. New
  `.github/workflows/docker-release.yml` ([ADR-0097](adr/0097-docker-hub-release-image.md)):
  on a `v*` tag, native `ubuntu-24.04` / `ubuntu-24.04-arm` runners build
  `apps/knx-server/Dockerfile`, run the smoke test against each image, push by
  digest (SBOM + provenance) and merge one manifest list
  `knxbench/knxbench-server:<version>` + `latest`, verified to contain exactly
  `linux/amd64` and `linux/arm64`. Manual dispatch is a build-and-test dry run.
  Tag runs need `DOCKERHUB_USERNAME` (variable) and `DOCKERHUB_TOKEN` (secret)
  and stop before building without them.
- `apps/knx-server/scripts/smoke-test.sh` repaired: since ADR-0088 a password
  means HTTPS, and the script still spoke HTTP (`307`, failing at the first
  401 check). It now uses HTTPS with `-k`, accepts `KNXBENCH_IMAGE` and
  `KNXBENCH_SMOKE_PORT`, checks `--version`, and hands the root-owned `/data`
  back before cleanup.
- Evidence: actionlint (with shellcheck) clean for the new workflow,
  shellcheck clean for the script, smoke test passed against a local amd64
  build of `0553f43c` (`knx-server 0.1.0-alpha.2+g0553f43c`). Dry-run
  dispatch on `cc5b1b9c` ([run 37890172997](https://github.com/KNXBench-Labs/KNXBench/actions/runs/37890172997)): both platform jobs
  built natively and passed the smoke test (arm64 on `aarch64`); the push and
  publish steps were skipped as designed. Nothing was pushed to Docker Hub;
  the tag path's tag/annotation argument assembly was checked locally with
  sample metadata. Boundaries in KNOWN_LIMITATIONS §168.

## 2026-10-09 — Three local packages move into main (toast parity, Devices navigation, community demos)

- Owner go: integrate the remaining local packages into `main`. Toast parity
  (`f8dbde0a`, `7832b42d`) fast-forwarded; Devices navigation and community
  demos 1.0.0 committed in their worktrees and cherry-picked onto it. Doc
  conflicts (ARCHITECTURE, IMPLEMENTATION_STATUS, KNOWN_LIMITATIONS) resolved
  by keeping both sides. A scratch path containing the account name was
  replaced by `<hermes-scratch>` in the demo delivery receipt before commit.
- Integrated gate on the merged tree `e46e5ce1` (fresh target, offline browser
  namespace, inputs frozen): Vitest 2480/152 files, build, tsc, flow-study and
  theme fixtures; intercepted Chromium 202; production Devices config 13;
  toast spec x3 42; cargo fmt, workspace Clippy `-D warnings`; Rust workspace
  3726 passed / 0 failed / 182 ignored; tools unittest 42; all five xtask
  checks; `git diff --check`. Docs-only status edits followed (this entry,
  OPEN_WORK, KL, ARCHITECTURE, manual status, demos index). The demo UI walk
  and real-server Devices checks are the packages' retained evidence, not
  rerun here. No release, deployment, private corpus or hardware activity.

## 2026-10-09 — Toast parity published on its own branch

- Owner-requested commit/push published feature `f8dbde0a` to
  `feature/toast-parity-20261008` only. All 15 owned file blobs, GitHub commit
  identity and local/tracking/live feature refs verified; KNXBench author and
  committer, no co-author. Main/root integration is not part of this request.
- Fresh focused toast tests: 45 passed; production frontend build, all five
  fresh-target repository checks and whitespace pass. Accepted source/test
  hashes unchanged. The 2,422/189/54 full and repeated test counts below remain
  dated 2026-10-08 evidence, not a newly executed full acceptance run.
- [Publication receipt](evidence/toast-parity-publication-2026-10-09.json).
  Published but unmerged; retained worktree/build for separately approved
  integration. No release, deployment, native/AT/private-corpus or hardware run.

## 2026-10-08 — Toasts now keep the same nine-second appointment

- Standard status and error toasts now reuse achievement card framing,
  message typography and entry/exit animations. All default to nine seconds;
  manual dismissal uses the same exit lifecycle. Errors keep red text/stripes,
  alert semantics and conditional English-server-text disclosure. No fake
  achievement badges or unlock labels on ordinary messages.
- Shared cards wrap long unbroken text and stay inside narrow viewports.
  Motion Off snaps out through zero-duration animation completion; OS reduced
  motion disables animation, with the existing invisible-card cleanup timer.
  Error replacement/clearing remains ID-bound and does not affect other kinds.
- Local isolated worktree `toast-parity-20261008`, base `115b19f6`:
  2,422 Vitest tests across 149 files, 189 offline Chromium cases and 54 focused
  repeat cases pass. Production build, three type checks, all five fresh-target
  repository gates and whitespace pass. Actual production workbench toast
  stacks inspected in Graphite/English at 1440px and Porcelain/German at 400px.
- [Acceptance receipt](evidence/toast-parity-2026-10-08.json), ADR-0089 amendment,
  manual guide and inventory synchronized. In-session self-review, not an
  independent review. Local/uncommitted/unpublished; no root sync, deployment,
  Rust/private-corpus rerun, native accessibility or hardware acceptance.

## 2026-10-08 — Devices gets a seat in the navigation (local source package)

- Owner-approved Q1–Q12 implemented locally on `115b19f6`, in the isolated
  `feature/devices-navigation-20261008` worktree. No commit/push, main/root
  synchronization, release or deployment. The released alpha.5 AppImage is unchanged.
- Devices/palette/Overview navigation opens a project-wide filterable/sortable
  canonical list, including unplaced devices; central editor with app-local Back
  reuses existing tabs/inspector and preserves list filter/sort/scroll/selection.
  Scoped links require explicit current IDs or unique individual-address matches.
- Read-only snapshot-bound GET `/api/devices` resolves each product reference
  once. Product lookup failures preserve canonical identities with explicit
  unavailable metadata; product identity does not establish application compatibility.
  Monitor links do not stop polling/reconnect or trigger hardware-operation forms.
- Final tests: frontend 2473 passed / 0 failed (152 files); server/projection
  781 passed / 0 failed / 45 ignored. Clippy, fmt, server/frontend builds,
  theme/Flow/new Devices fixture type checks pass. Production-preview Chromium
  13/13 and full intercepted Chromium suite 188/188 pass; the latter includes
  those feature scenarios and is not an additional disjoint feature count.
- Isolated actual-server HTTP + production-browser sample smoke confirms eight
  fictional canonical devices, resolved products, unchanged read-only snapshot,
  central editor/Back and no page exceptions. Screenshots visibly inspected;
  long building paths use the existing bounded select and the wide table scrolls
  locally. Initial browser failures and smoke-harness selector/onboarding mistakes
  remain recorded, not relabelled as successful attempts.
- [Contract](DEVICE_NAVIGATION.md), [ADR-0096](adr/0096-devices-navigation-and-catalogue-batch.md)
  and [receipt](evidence/devices-navigation-2026-10-08.json) record scope and evidence.
  In-session self-review, not independent-agent review. No hardware/private-corpus/
  full workspace/native WebKitGTK/Orca or complete accessibility acceptance.

## 2026-10-08 — Post-rebase community links verified

- Audited public main `608a204bf28aa9df83413aa5ccb65241c32478aa`; current app,
  EN/DE guides, native issue form and contact config match exact remote bytes.
  No runtime link adjustment required. Anonymous main/guide/form/website reads
  succeed; signed-out issue handoff preserves native template/version through
  login. Retired repo returns 404; no active hyperlink to it in scanned tracked
  app/site/form/docs sources (historical name mentions retained).
- Focused contribution UI/API tests: 10 passed, 0 failed/pending. Production
  build, theme-fixture types and flow-study types passed. Real Chromium:
  32 checks, 8 explicitly synthetic intercepted requests, no external requests
  or browser exceptions; preview/consent/download precede the manual handoff,
  and only template/version are URL-prefilled. Existing Vite chunk warning stays.
- ADR-0091's current decision now names public main; earlier plan is explicitly
  historical. Evidence: `docs/community-evidence/main-repository-link-verification.json`.
- No deployment, logged-in GitHub submission, attachment upload, private corpus,
  native runtime or hardware acceptance claimed; mailbox still unverified.

## 2026-10-08 — Three English community demos: the buildings are fictional, the checks are not

- Local owner-review candidate: Single-Family Home 32 devices/105 GAs,
  six-flat Multi-Unit Residential Building 101/339, three-floor Office Building
  157/507. Native `.knxdb`, shared original fictional `.knxprod`, English
  tours/five exercises, source/licence, versioned individual/combined ZIPs and
  SHA256SUMS. [Download index](../demos/README.md), [contract](COMMUNITY_DEMO_PROJECTS.md).
- Tooling-only typed core/store writer and separate catalogue declarations;
  no production core/API/UI/schema/dependency change or ETS output. Clean
  catalogue install reports 0 unknown/conflicts; repeated import deduplicates.
- Real server/production frontend/Chromium in a loopback-only namespace:
  966 named checks, all 290 device programme/parameter views, 15 UI exercises;
  acknowledged description/parameter/group/link edits and save-as/reopen.
  Whole baseline save/load equality and whole practice-state equality against
  the four intended core commands are separately verified; only the legitimate
  save timestamp is normalized. Nine actual baseline screenshots retained.
- Gates: affected `knx-app --all-targets` 162 passed/25 ignored; tooling 42,
  example Clippy and workspace fmt passed; fresh-target server and production
  frontend build passed. Repository/documentation/final-package checks and
  exact byte identities are in [the receipt](evidence/community-demos/delivery-verification.json).
  Local self-review only; no private-corpus ignored, full-workspace, native,
  ETS/XSD, hardware or deployment acceptance.
- UI realities are explicit: description editing replaces unavailable device
  renaming; practice DPT is inferred after linking, not edited on the address;
  retain untouched originals because native edits can auto-save. Topology and
  device programmes are illustrative, have no firmware and must never be
  downloaded to real hardware. No isolation/filter/electrical-design claim.
- No commit, push, release or website change: the owner reserved a separate
  publication go after inspecting the local artifacts. Tested app base remains
  `33db32e9`; subsequent main commits through `115b19f6` are documentation-only,
  and the other Devices-navigation worktree is preserved, not incorporated.

## 2026-10-08 — Known issues and implementation status: fewer fossils wearing “current” badges

- Extended the documentation audit across all 40 original user-facing issue
  topics, the whole manual status chapter and the complete technical
  heading/anchor inventory. Added seven existing boundaries to the manual,
  not seven new bugs; [source/evidence audit](status/2026-10-08-known-issues-status-audit.md).
- Corrected non-program translation loss, blanket legacy refusal, history/Web
  handoffs, CLI diff exit codes, native loopback exposure and AppImage source/
  old-asset distinctions. Source-bound evidence is separate from current
  hardware/native/advisory verification; historical log results stay intact.
- Narrowed the wizard contract to previewed names/addresses, layered provenance
  to modelled values and green status markers to their actual evidence scope.
  Sibling tutorials/troubleshooting now say the same thing. Program-DPT fallback
  UI consumption and exact product namespace admission are no longer “missing”.
- Public address writes remain fail-closed; recovery/history are bounded, and
  interrupted device writes, shared preferences, MCP snapshots and private
  contribution evidence have explicit practical notices. Owner ledger unchanged.
- Focused tests: Web 270, HTTP 56, CLI diff 8, read-only store 9, projection 2,
  translation-install 1; tooling/build/documentation gates recorded in
  [the receipt](evidence/known-issues-status-audit-2026-10-08.json). No new hardware,
  private-corpus, full native or advisory run. Documentation-only, local edits;
  existing README owner's combined-delivery handoff remains separate.

## 2026-10-08 — Ideas and roadmap audit: stop putting shipped features back in the queue

- Checked all twelve original root-local `ideas.md` entries against `608a204b`,
  code, actual UI/API consumers, tests, ADRs and the owner ledger. The ignored
  idea file is updated locally; the [source/test audit](status/2026-10-08-ideas-roadmap-audit.md)
  preserves its assessment in maintained documentation.
- Corrected stale Flow/MCP/humour/theme/schema-23 claims, L3 legacy web import,
  commissioning history consumers, historical alpha closeout and public
  website/story/intake state. Kept research, missing functionality, accepted
  scope and missing validation distinct; no new dates or scope decisions.
- Updated roadmap/open-work summaries, manual ideas/status/compatibility and
  related historical VD/website entry points. Preserved stable manual navigation,
  published launch records and prior real-app captures. Owner ledger unchanged.
- Focused verification: 255 web tests, 18 MCP tests, 10 HTTP tests and 38 tooling
  tests pass. Five repository gates and documentation/navigation checks verify
  the local result; [receipt](evidence/ideas-roadmap-audit-2026-10-08.json).
- Documentation only: no application behaviour authored, deployment, hardware,
  private-corpus or broader native/accessibility acceptance. Local edits remain
  uncommitted/unpublished in `docs-refresh-20261008`.

## 2026-10-08 — Website goes live on GitHub Pages (knxbench.com)

- **Why:** owner go for publication; DNS at Host Europe was set up by the
  owner (apex A/AAAA, `www` CNAME, verification TXT; checked at both
  authoritative nameservers).
- **What:** `website/build.py --release` builds the public variant (no
  preview banner/launch note/`noindex`, open `robots.txt`, `CNAME`, privacy
  pages naming GitHub Pages and its documented IP logging); the story's new
  published variant (`storytool build --approval`) renders only when the
  approval record matches the exact edition, otherwise the build refuses and
  writes nothing. `.github/workflows/pages.yml` tests, builds and deploys only
  the built directory. Preview builds are byte-identical except the
  info pages' feedback link, which now points to the main repository's guide;
  the contact text no longer names the interim contributions repository.
- **Gate:** website 20 unit tests, story 67 (4 new; a gate-bypass mutant is
  caught by name), Chromium offline: 154 preview checks unchanged plus 22
  release checks, zero unexpected requests/errors; doc gates.
- **Limits:** privacy text is drafted from GitHub's documentation for owner
  review, not legally certified; no WebKit/screen-reader check.

## 2026-10-08 — Documentation refresh: clearer routes, real visuals, fewer time machines

- README is an entry point with public AppImage and browser/source installation
  paths; the 33-chapter manual keeps its stable URLs and has checked Previous/Next
  navigation, explicit tutorial prerequisites/results/caveats and a no-hardware
  create/save/reopen exercise. New [docs hub](README.md) and
  [maintenance guide](DOCUMENTATION.md) separate user instructions from engineering records.
- Stale release/private-repository wording, HTTPS, autosave/backups, protected
  project input, CSV actions, DPT subtype claims, wizard paths and parameter tabs
  were corrected against source and maintained evidence. Parallel website and
  community-evidence deliveries and their published link repairs are preserved.
- **32 real Porcelain screenshots** and **three short workflow GIFs** were
  captured from freshly built source `4c84ab9f`, a real server, fictional data and
  an isolated loopback-only namespace. [Capture/provenance guide](assets/README.md)
  and [media manifest](assets/media-manifest.json) explain regeneration and gaps.
- Verified: 38 tooling tests including 12 new documentation regressions, 19
  focused language tests, two complete capture tests with real project/device
  operations, frontend TypeScript/Vite and server builds, five xtask gates and
  whitespace check. The documented HTTPS health probe ran twice on a disposable
  native server; this is not a new Docker replacement or hardware run.
- [Source-bound validation receipt](evidence/documentation-refresh-2026-10-08/validation.json)
  scopes the evidence. No application behaviour authored, no new compatibility
  claim, no full workspace/native accessibility run, and no publication implied.


## 2026-10-08 — Community evidence lands, intake moves into the public main repository (ADR-0091)

- **Why:** Codex built the read-only community-evidence package and its
  beginner guide on 2026-10-08 in the shared root checkout and never committed
  them; the root sync after the public launch parked them on a local branch.
  With the main repository public, ADR-0091's launch consolidation applies too.
- **What it is:** own-instance analysis of `.knxproj`/`.knxprod` on a
  disposable product database (production parsers, encounter reports, offline
  evaluation/download preparation, a bounded structural inventory for refused
  namespaces), version-1 evidence ZIPs in three disclosure tiers (reduced
  report, explicitly selected context, private original with separate consent)
  with exact previews and preview-bound export. Reachable through File →
  Analyze support gaps… (EN/DE six-step help), `POST /api/contributions/
  {analyze,preview,export}` behind the session guard (one worker, body limit)
  and `knx contribution analyze|preview|export` (export never overwrites). No
  user project/product mutation, no bus access, no upload.
- **Consolidated:** the issue form is now `.github/ISSUE_TEMPLATE/analysis.yml`
  in this repository (blank issues stay enabled), the EN/DE guides stay in
  `docs/contribution-intake/`, and the app, guides, form and the website's
  support link point here instead of the interim `KNXBench-Contributions`
  (which had no issues; deleted by the owner the same day after the move
  was confirmed, Git bundle kept in the local backups).
- **Left out on purpose:** the parked branch's LCARS and parameter-workspace
  files (already on `main` in newer form or archived), its older doc drafts
  and the per-step receipt files.
- **Gate (worktree, fresh target):** clippy `-D warnings`, fmt; Rust workspace
  3,721 passed / 0 failed / 182 ignored (224 result blocks); Vitest 2,415 / 0
  in 149 files; web build and both fixture type checks; Playwright 175 passed;
  five xtask gates. The package has no corpus or ignored tests. Cargo added the
  new `quick-xml` dependency line to `Cargo.lock` during the run (committed).
  In-session review only.
- **Not verified:** the `contribute@knxbench.com` mailbox, native WebKitGTK/Orca,
  live hardware. Contract: [COMMUNITY_EVIDENCE.md](COMMUNITY_EVIDENCE.md).

## 2026-10-08 — Marketing website lands in the repository (ADR-0095)

- **Why:** the DE/EN marketing companion for knxbench.com was built by Codex
  on 2026-10-08 in the shared root checkout and never committed; the root sync
  after the public launch parked it on a local branch. This package delivers it.
- **What it is:** `website/`, a static site independent of the app, core and
  server: curated DE/EN pages (English at `/`), minimal JS, Story-style rolling
  headlines with live reduced-motion/pause handling, a stdlib-only
  deterministic builder, a loopback-only preview server, hash-inventoried
  self-hosted media (three real-app clips with DE/EN captions, theme
  screenshots, OFL fonts) and the pinned existing story. Contract:
  [WEBSITE.md](WEBSITE.md).
- **Changed on delivery:** ADR renumbered 0094 → 0095 (number taken by the
  legacy EX-IM ADR meanwhile); the archived manual-acceptance link pinned; the
  launch note now says repository and downloads are public and the site is
  still a preview (DE/EN content and the browser recipe's assertion); the five
  historical per-step receipt files dropped (they bound earlier local
  candidates). Launch gate 2 (public entry points) is done: repository,
  manual, Docker guide, releases page, the alpha.5 AppImage and the
  contributions repository answered signed out. The imprint's name and postal
  address are committed on purpose (owner decision 2026-10-08).
- **Not done:** no deployment, no Pages workflow, `build.py --release` still
  refuses. Open launch gates: deployment go and release-mode design, exact
  story edition approval, host-specific privacy page, domain/DNS/HTTPS.

## 2026-10-08 — Public launch: history purged of third-party files, repository public

- **Why:** the user decided to make the repository public. An audit of every
  reachable object (secrets, tokens, private keys, personal identity, private
  corpus files, large blobs) found no secret and no identity, but two deleted
  third-party payloads still in the history: `az-and-sensor-data/` (ETS-plugin
  data of an alarm panel, manufacturer firmware) and a third-party demo
  project.
- **What:** both paths were removed from the whole history with
  `git filter-repo`; the final tree of `main` is byte-identical. The
  repository was recreated under the same name from the purged history, the
  pre-release `v0.1.0-alpha.5` was republished on the rewritten tag, and
  the repository is public since 2026-10-08 (checked anonymously: web, API,
  release asset, clone). The pre-purge repository stays private under
  a different name as a rollback until the user removes it.
- **Hash bridge:** [`docs/history/COMMIT_MAP_2026-10-08-public.txt`](history/COMMIT_MAP_2026-10-08-public.txt),
  details and branch rules in
  [known limitation §162](KNOWN_LIMITATIONS.md#162-commit-hashes-cited-before-2026-10-07-refer-to-the-rewritten-history).

## 2026-10-08 — Legacy VD files L3: web upload, password dialog, one remembered password (ADR-0094)

- **Why:** L2 made `.vd3`–`.vd5` programs usable offline, but only through
  the CLI. L3 brings the import to the web app and the server, together with
  the one remembered password the user approved in the grilling (Q3/Q12).
- **What:**
  - `knx_app::legacy::RememberedPassword` (`legacy/remembered.rs`) keeps one
    password in a plain file, `$XDG_CONFIG_HOME/knx/legacy-vd-password`
    (else `$HOME/.config/knx/…`), with mode 0600 in a directory created
    0700. It is written atomically, with a temporary file per call. A file
    group or others may read, or one with an empty first line, is refused
    by name.
  - `open_with_password_policy` decides which password opens a file. A
    non-empty given password wins; otherwise the remembered one is tried;
    otherwise `PasswordRequired`. Nothing else is ever tried. A remembered
    password that does not fit is its own refusal.
  - Server:
    - `POST /api/catalog/install-legacy` takes multipart `file`, optional
      `password` and `remember`. Refusals are `422` kinds:
      `legacyPasswordRequired`, `legacyWrongPassword`,
      `legacyRememberedPasswordDoesNotFit`,
      `legacyRememberedPasswordUnusable`.
    - `POST /api/catalog/install` names a legacy product database (by
      `.vd3`–`.vd5` name or by content) as `422 legacyProductDatabase`.
    - `GET`/`DELETE /api/legacy-password` report and forget the password,
      never returning its value.
    - The password is remembered only after a successful import.
  - CLI: `knx products legacy-password set|forget|status` and
    `import-legacy --remember`. Without a password, `import-legacy` tries
    the remembered one.
  - Web:
    - `ProductInstallControl` serves the catalog and the device wizard.
      Their file pickers now accept `.vd3`–`.vd5`.
    - A renamed legacy file follows the `legacyProductDatabase` refusal to
      the legacy route.
    - `LegacyPasswordDialog` (with "Remember") and `LegacyInstallReportView`
      are new. The report ends with an "offline only, no download yet" note.
    - A Settings section shows whether a password is remembered and can
      forget it. It is injected into `SettingsPanel` like the achievement
      tracker.
    - en and de copy.
  - CSS fix: `.device-wizard-step .settings-field input { width: 100% }`
    stretched the dialog's "Remember" checkbox to 486 px, because the
    nested Overlay renders inside the wizard step. Checkboxes are now
    excluded. Measured in Chromium and pinned by the e2e spec.
- **Real data:**
  - The real `.vd4` was uploaded to a locally started `knx-server` with
    temporary XDG and data directories. It asked for its password, then
    published 2 programs, 334 parameters, 520 refs, 56 object refs and
    10,428 translations, matching the corpus pins.
  - A second upload used the remembered password and was skipped as
    already imported. The file was 0600 in a 0700 directory. The password
    was in neither the server log nor the data directories.
  - In the web app, N000520 was placed with 28 objects, and the umlauts
    rendered correctly. Switching "Objekttyp für Ausgang - Licht" to
    "Dimmen absolut" swapped "Objektwert für EIN/AUS" from on/off to
    percentage fields (100 %, 0 %).
- **Found, not fixed here** (KNOWN_LIMITATIONS §128): untyped spacer
  parameters (kind `None`, ETS `TypeNone`, e.g. `d_space`) are reported as
  `editable`. The web app therefore shows empty text fields, and the server
  refuses any write by name. The same panel path serves ETS `TypeNone`
  parameters, so this is queued as its own fix.
- **Review (in-session, before fixing):** three minor findings, all fixed
  test-first:
  - Concurrent stores shared one temporary file name. The new test was red
    4 of 4 times and is green after the fix.
  - An empty-first-line file was treated as "none remembered".
  - The ADR named only the server binary, not the desktop app.
- **Mutation sweep:** **19/19**, each killed by a named test. It covers
  Rust (policy, file modes, server, CLI), TypeScript and the CSS fix (e2e).
- **Gate:** on the rebased feature head `ec4a2b07` (the new history after the
  8 October rewrite), under both gate locks with inputs frozen:
  - Web: build, `tsc`, flow-study and theme-fixture checks; Vitest **147
    files, 2,402 tests**; Chromium **175 passed**; the legacy spec repeated
    3× (15/15).
  - Rust: `cargo fmt`, workspace clippy `-D warnings`; workspace tests
    **3,697 passed, 0 failed, 182 ignored**.
  - Corpus: `legacy_corpus` plus `legacy_oracle` with
    `--include-ignored` 4/4.
  - All five xtask gates and `git diff --check` pass.
  - Attempt 1 failed only to compile: a full `AppState` literal in
    `http_load_progress.rs` lacked the new field (fixed there and in
    `http_settings_conditional.rs`).

## 2026-10-08 — Gap analysis, cloud tooling and the memory index retire

- User decision: removed `docs/GAP_ANALYSIS_ETS.md`, `docs/Issues.md`, `.serena/`,
  `tools/cloud/` (and its `SessionStart` hook in `.claude/settings.json`) and
  `tools/agent_memory_sync.py` with its test. 40 links pinned to `aa0ff14ff536`;
  [REMOVED_DOCS](history/REMOVED_DOCS.md) has the second-round table.
- The shared memory index is decommissioned, not just unversioned: the systemd
  user timer is disabled and removed, the marker blocks are uninstalled, and
  `PROJECT_CONTEXT` no longer ranks a generated index. Backups outside the repo.
- Gates: five xtask checks green, remaining `tools/tests` unittest suite OK.

## 2026-10-08 — Archive, design studies and superpowers leave the tree

- User decision: removed `docs/archive/`, `docs/design/`, `docs/design-studies/`,
  `docs/superpowers/` (187 files) and the root `IDEA.md` and development-strategy
  note. All stay in Git at `138403ed6084`;
  [REMOVED_DOCS](history/REMOVED_DOCS.md) lists them and how to read them back.
- 208 Markdown links into them now point to pinned GitHub URLs at that commit;
  115 backticked goal paths became plain goal names. Source comments that cited
  the archived goals were reworded; the telegram-flow load study writes to
  `docs/evidence/telegram-flow-u21/`.
- Plain dead-link scan: 1 (pre-existing `../CLA.md`), down from 17 because the
  old dead links lived in the removed plans. Docs and comments only.

## 2026-10-08 — Goal audit and docs/root cleanup

- Audit of every goal against `origin/main` `b34afcc8`: `alpha-release-goal.md`
  (93/100 boxes; the 7 open are user-deferred `KL-129`/`KL-135` or the optional
  AR14B live run), `goal-ui.md` (closed 2026-10-06) and `goal-commission.md`
  (K1–K19 delivered, hardware validation out of scope by decision) are finished.
  The ledger has no `TODO`/`IN_PROGRESS`/`WAITING_*`/`BLOCKED_*` rows.
- New [OPEN_WORK](OPEN_WORK.md): running tracks, root-only unpublished work
  (community evidence, website, an ADR-0094 number collision), deferred
  decisions and the 23 `LATER` rows in plain words.
- Moved with `git mv`, links rewritten by script: the three goals, nine alpha
  dossiers and the AR18 review to [archive/alpha-0.1/](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/README.md);
  root `stats.md`/`compare.md` there as dated snapshots; ten behavioural
  contracts to [contracts/](contracts/README.md); `CLOUD_SESSIONS.md` and the
  delivered `PROJECT_EVOLUTION_GOAL.md` to `archive/`. Four code comments and
  `tools/cloud/setup-env.sh` follow the new paths.
- Plain dead-link scan unchanged (17 pre-existing, none new); `check-anchors`
  694 links/none dead; `check-ledger` 191 rows.
- Manual implementation-status page: server version corrected to `alpha.2`.
- Docs-only; no code behaviour, ledger row or status changed.

## 2026-10-08 — Clean parameter editor and dedicated inspection tabs

- Owner request: add Diagnostics and Manufacturer fields beside Product data;
  evaluation prose no longer repeats above/inside the normal parameter editor.
- Repeated diagnostics grouped by kind/severity/exact scope/fallback message,
  with occurrence counts, concise no-branch explanation, every original detail
  inspectable/copyable, and unmatched stored values retained in Diagnostics.
- Evaluated Access Read/None fields move to inspection-only Manufacturer fields
  with access-specific reasons; malformed editable/write flags cannot enable them.
  Other read-only fields remain in Parameters. One shared read model/no tab refetch.
- Test-first workspace RED/GREEN; integrated root **2312 frontend tests /
  139 files**, build and both fixture type gates pass. Actual Chromium:
  **278 checks**, EN/DE × 400/1440px × Porcelain/Graphite/LCARS CSS, exact 24
  synthetic intercepted GET/POST requests, no errors/unexpected/external calls.
- All five repository gates pass on a fresh xtask with explicit root/nonzero
  scope (layering, headers, anchors, ledger, corpus source lint); diff check passes.
- UI-only; no server/DTO/core/storage/protocol change or broader compatibility
  claim. Source self-review, not independent approval. Native accessibility and
  full static manufacturer inventory remain outside scope.
- [Contract](PARAMETER_WORKSPACE.md),
  [source/test/browser receipt](parameter-workspace/verification.json).
  The original receipt is historical local-source evidence. The owner authorized
  merge/commit/push separately; merged-source delivery evidence is recorded in
  `docs/parameter-workspace/delivery-verification.json`. No Docker activation
  or hardware contact is part of this delivery.
- Delivery: main publication `38a48d39` verified by exact remote
  readback. Final merged frontend **2388 tests / 144 files**, full Chromium suite
  **170 tests**, retained verifier **278 checks**, build/types and five gates pass.
  Subsequent closure metadata leaves every tracked frontend byte unchanged.

## 2026-10-08 — Legacy VD files: secret-class values withheld from the stored payload (ADR-0094)

- **Why:** L2 stored the decrypted payload byte for byte. The accepted
  design (2026-09-26, §6.4 and decision B-3, confirmed as Q10 in the
  grilling) requires that the non-empty values of secret-class columns
  (`*PASSWORD*`, any case) are blanked in the stored copy and reported by
  count only. L1/L2 had missed it; found while preparing L3.
- **What:**
  - The EX-IM parser now records each value's source range, continuation
    lines included (`ExImTable::source_range`).
  - `knx_productdb::legacy::withhold_secret_values` builds the stored copy.
    Each non-empty secret value becomes an empty value line, and every other
    byte stays. A payload the grammar refuses is refused here as well.
  - `publish_legacy` stores, parses and keys that copy; its digest is the
    payload identity and namespace. It reports `secret-withheld` with table,
    column and count, never a value.
- **Caveat** (KNOWN_LIMITATIONS §128): the original file is still stored
  verbatim (Q10). That is safe when it is encrypted, since the password is
  never kept, but an unencrypted original keeps such values readable.
- **Real files:** neither `EIBMARKT.VD3` nor the Eibmarkt `.vd4` has a
  non-empty secret value. Their corpus pins and the N000520 oracle are
  unchanged (3/3, 1/1, the same three named deviations).
- **Tests:** `legacy_secrets.rs` 5, publish +1, app +1. The program fixture
  gains a `device` table with an invented `DEVICE_BCU_PASSWORD` over a
  continuation line. Mutation sweep **8/8**, each killed by a named test.
- **Gate:** `fmt`, clippy `-D warnings` for `knx-productdb`, `knx-app`,
  `knx-cli`, `knx-server` and `knx-mcp`, all five xtask gates and
  `git diff --check` pass, with inputs frozen. Tests: `knx-productdb`
  **762 passed, 0 failed, 25 ignored**; before the last test-only addition,
  the four crates together passed **1,830, 0 failed, 115 ignored**.
  Corpus: legacy 3/3, oracle 1/1. The `.knxprod` path is untouched, so the
  product matrix was not rerun.
- **Still open:** `inspect-legacy` does not yet list declared secret-class
  columns (the design's "declared / non-empty in n rows" line); it prints no
  value of them either.

## 2026-10-08 — Legacy VD files: programs imported for offline use (L2, ADR-0094)

- **What:** every application program of a legacy ETS3 `.vd3`/`.vd4`/`.vd5`
  product database can be imported into the product database and then used
  like any other: catalog entry, placement, parameters with visibility and
  translations, communication objects linked to group addresses. Download
  stays out (L4).
- **knx-productdb::legacy:**
  - `map_legacy_database` turns the parsed payload into plain rows. The
    rules are measured against ETS 6.3's conversion and recorded in
    `docs/research/legacy-vd-mapping.md`: grouping by memory cell, unions,
    enumerations, default/access/text overrides, the dynamic tree from
    `PAR_PARAMETER_ID`/`PARENT_PARM_VALUE`, five LCIDs, the catalog from
    functional entities.
  - Ids live in an `LX<sha8>` namespace inside ETS's own segment
    (`M-1092_A-LX…-300`). A namespace owned by another payload is refused
    by name.
  - `publish_legacy` writes everything in one transaction, idempotent per
    payload digest. Provenance goes to **schema v22** (`legacy_source`,
    `legacy_source_file`, `legacy_program`, `legacy_diagnostic`, additive).
    `write_authority_recorded` is set, since EX-IM has no calculations.
  - Value escapes `\'`, `\r`, `\n`, `\\` are decoded; unknown ones are
    counted. Unmapped tables, skipped rows (summed per table and reason),
    orphan or conflicting translations, unknown access levels and parent
    chains deeper than 64 are reported, never dropped.
  - A group member's translation is left out only where it adds nothing;
    a member that overrides the text keeps every translation.
- **Download refuses** a legacy program as `CodeError::LegacyProgram`.
- **knx-app:** `import_legacy_file`. **CLI:** `knx products import-legacy
  <file> [--product-db] [--password-stdin | --password-file]`. It decrypts
  before it opens the product database, so a wrong or missing password
  leaves no file behind.
- **Acceptance (private corpus, ignored tests):**
  - N000520 from the `.vd4` against ETS's conversion through KNXBench's
    evaluator: 260 parameter refs, 28 object refs, 3,535 translations,
    36 visibility cases. Three named deviations: 5008 access, one extra
    en-US program-name translation, and 5008's placement.
  - Both real files publish with pinned counts (VD3: 3 programs, 1,366
    translations; VD4: 2 programs, 10,428 translations) and evaluate with
    `NoBranchMatched` as the only evaluator diagnostic, the same kind ETS's
    own conversion shows.
- **Synthetic tests:** mapping 17, publish 8, CLI import 5, HTTP 1 (catalog
  listing, placement, parameter edit that switches the visible branch and
  the object activation, group link). The program fixture is new
  (`marvin-program*.vd4`, "Improbability Drive").
- **Found on the way:** the product-DB rewind fixtures did not drop the new
  v22 tables. That broke 36 migration tests across 11 files, all with
  `table legacy_source already exists`. `v20_rewind::drop_v22_objects` now
  runs in every rewind.
- **Mutation sweep:** 20/20 realistic reverts fail a named test. The first
  sweep left one survivor (text overrides always set); its test now pins
  the ref texts.
- **Self-review** (in-session, not independent) found the CLI's DB-before-
  decrypt order (fixed test-first), the unnamed namespace collision (fixed
  test-first) and an untested catalog listing (test added, no code change).
- **Gate:** on `12ba7c68` (rebased on `bcb23598`), under both gate
  locks, inputs frozen (empty diff at start and end):
  - Web build, fmt and clippy `-D warnings` (workspace) pass; all five xtask
    gates pass; `git diff --check` is clean.
  - Workspace tests: **3,663 passed, 0 failed, 182 ignored** (217 result
    blocks).
  - Corpus: legacy corpus 3/3 and oracle 1/1; `standalone_packages` ignored
    3/3; `legacy_member_names_corpus` 1/1.
  - Product matrix (release): the first run was red on the aggregate
    commitment only. It now counts the four new, empty v22 tables. With them
    left out, the v16-shaped projection proved unchanged, so the commitment
    was re-pinned (`7b558cdd…` → `541d0afc…`) and each table pinned at 0.
    The rerun passes 1/1, and clippy for `knx-productdb` passes again.
- **Not done:** server/web upload with password dialog and the remembered
  password (L3); DPTs (`EIB_DATA_TYPE_CODE` is unmeasured); download (L4).
  The visual web check moves to L3, where the upload makes it reachable.
  The first real `.vd5` (Siemens, Nov 2016) is refused: 173 MB payload,
  four members (KNOWN_LIMITATIONS §128); it needs its own package.

## 2026-10-08 — Calm LCARS ambient source published and activated

- Tested merge **37e7f642** is published on main and stamped into matching Docker
  client/server image; feature **490eed98**, keyboard-test correction **8ccc6958**.
  Parallel wizard/achievement/localization/legacy main source retained.
- Frozen complete app/crate source: **2375 frontend / 143 files**, **170 Chromium**,
  **54 native production-workbench assertions**, targeted Rust/synthetic legacy,
  build/types and five repository gates. In-session self-review only.
- Isolated HTTPS/auth/PID1 probe: all **89 frontend assets** match exercised build;
  both handlers installed, graceful stop **0 in 0.27s**; probe/volume removed.
- Fresh authenticated pre-stop: no modified project/download/address session.
  Live **3b65051a**: HTTPS/version/login/all-89-assets/runtime/bind/env/hostname/
  restart/log/TLS-fingerprint readback accepted. Checksum equality is scoped to
  **4 explicitly inventoried noncredential files**, not all database sidecars.
- Saved LCARS/Standard preference retained; vault login succeeds and both native
  ambient clocks advance. Actual `.25s` token gives **12.5s/20s**; 10s/16s and
  18s/24s are reference-token periods, not forced overrides. No user setting changed.
- Rollback **knxbench-pre-ambient-20261008** and earlier LCARS rollback retained.
  Receipt: `design-studies/lcars/ambient-deployment-verification.json`. No tag,
  hardware request, native WebKitGTK/Orca/full accessibility certification.


## 2026-10-08 — Calm LCARS ambient animation follow-up (integrated publication accepted)

- **Final merge with main `5d979361`:** **2,375 frontend / 143 files**, **170 Chromium**, **54 native workbench assertions**, **83 targeted Rust tests** including synthetic legacy container/grammar/password/CLI controls, build/types/five repo gates on frozen full app/crate source. Both wizards, achievement timing and latest localization/legacy inspection preserved. Proof: `design-studies/lcars/ambient-merged-verification.json`; earlier receipts remain source-specific history. No private-corpus or hardware claim. Image/probe/live acceptance pending.


- **Latest publication candidate `a642eaf9` preserves both delivered wizards and achievement timing:** **2,373 frontend tests / 142 files**, **170 un-retried Chromium**, **54 actual production-workbench assertions**, **27 targeted Rust tests** (13 seed, 5 HTTP seed, 3 catalog requests, 6 HTTP device-wizard), build/types and five repository gates pass on frozen input. Source/build/phase evidence: `design-studies/lcars/ambient-publication-verification.json`. Earlier f5/e3 receipts are historical, not reused as current acceptance. Image/probe/live readback remains pending.


- User explicitly requested a subtle, living idle presentation after approving
  the deployed theme. Exactly two decorative CSS loops: segmented-header
  opacity (10s) and small K-emblem apricot/lavender colour (16s). Subtle slows
  them to 18s/24s and reduces amplitude. No blinking engineering content,
  geometry changes, fake activity/success, JavaScript timer, API/core change
  or new dependency/persisted setting.
- Existing Motion level Off, OS reduction and theme removal cancel live effects
  to static paint. Saved Off and OS reduction also prevent cold-start effects;
  Subtle survives reload. Imported-format palettes cannot gain the loops.
- RED: three new boundary assertions failed before CSS; GREEN: **2,332 frontend
  tests / 138 files**, **158 Chromium tests**, build/type/flow/five repository
  gates passed. Actual production-workbench CLI: **54 named assertions**, no
  unexpected requests/browser errors; real clock progression, amplitude,
  cancellation, independent density, Save As refusal and small layouts verified.
- ADR-0092 amended for the explicit decorative exception. Original standalone
  study and prior delivery/deployment receipts remain historical and unchanged.
  New source/build/evidence binding: `design-studies/lcars/ambient-verification.json`.
- Source self-review only; native accessibility/Firefox not certified. Local
  follow-up complete; user subsequently authorized commit/push/redeployment with
  "go". Saved/no-programming state verified via authenticated read-only status;
  publication and container acceptance pending.

- Fresh combined wizard + ambient gate on `e3e7641b`: **2,358 frontend tests / 140 files**, **162 Chromium tests**, **54 actual production-workbench assertions**, build/types and five repository gates; inputs frozen. Targeted seed regression: **13 app + 5 HTTP tests**.
- First combined Chromium attempt caught an inherited dynamic-first-node test target: selected 9.1.10 but asserted newly arrived 9.1.1. Stable accessible identity and explicit focus retain real Enter/current-value checks; 20 repeated controls and complete un-retried suite pass. Separate focused test correction; no product-code workaround.
- Current-source/build/phase receipts: `design-studies/lcars/ambient-integrated-verification.json`; original 2332/158/54 receipt stays historical. Image/probe/live acceptance is still pending; no bus operation.

## 2026-10-08 — Legacy VD files: read-only inspection (L1, ADR-0094)

- **Requested** in a recorded grill-me interview
  (`.ai/logs/2026-10-08_claude_legacy-vd-grilling.md`, Q1–Q18). The goal is
  offline parameterisation of devices that ship only as ETS3 `.vd3`/`.vd4`
  files; download is a later, separate package. The 2026-09-26 design is
  accepted with amendments.
- **New in `knx-productdb::legacy`**:
  - Content detection of one-member `ets.vd_`/`ets2.vd_`/`ets.pr_`
    containers through the existing package ZIP validator.
  - Observed-layout, encryption and method checks.
  - Bounded inflate with CRC-32.
  - A strict, bounded EX-IM grammar. Raw bytes are kept, Windows-1252 is
    labelled as an assumption, and unknowns become diagnostics.
  - `install_package` refuses a legacy container under any name as
    `PackageError::LegacyExIm`.
- **Decryption lives in `knx_app::legacy`**, with a user-supplied password
  and `knx-secure`'s single ZipCrypto implementation. A new `check-layering`
  rule forbids `knx-productdb → knx-secure`, dev edges included. That edge
  had made `knx-mcp` link key material, which ADR-0090 forbids, and the gate
  caught it.
- **CLI:** `knx products inspect-legacy <file> --password-stdin |
  --password-file <path>` writes nothing. It refuses argv passwords, empty
  passwords and unbounded password files.
- **Real files:** `EIBMARKT.VD3` (37 tables, 4,214 rows), the Eibmarkt
  `.vd4` (37 tables, 14,734 rows) and the MDT `.pr5` (16 tables, 12 rows)
  all read with **zero diagnostics**. Pinned hashes match. The real password
  occurs in no tracked or untracked file; a planted canary proves the scan
  works.
- **Tests:**
  - 20 grammar, 16 container, 7 password, 9 CLI and 2 ignored corpus tests,
    on synthetic "Marvin Test" fixtures (Info-ZIP `zip` + `zipcloak`).
  - Mutation sweep: **25/25** realistic guard reverts fail a named test.
    The first sweep left two survivors; their tests now also assert the
    refusal reason.
- **Gate on `d01cc58d`** (rebased on `e96bfb5d`):
  - Web build, fmt and clippy `-D warnings` pass; all five xtask gates pass;
    `git diff --check` is clean.
  - Corpus tests: `knx-app` legacy corpus 2/2, `legacy_member_names_corpus`
    1/1, `standalone_packages` ignored 3/3, and the product matrix (release)
    1/1. Every matrix pin is unchanged: no modern package behaves
    differently.
  - Workspace tests: **3,618 passed, 0 failed, 180 ignored** (211 result blocks, exit 0,
  HEAD unchanged during the run). The first attempt died before any
    test ran (`ld` killed by signal 9). It is kept, and the step was rerun
    with `-j 2`.
- **Re-gate after the second rebase** (onto `a642eaf9`, the add-device
  wizard; it changed only `knx-server` and the web app, which build
  against the changed crates): `3d8eef7f` passes fmt, clippy `-D warnings`
  (whole workspace) and all five xtask gates. `knx-server`, `knx-app`,
  `knx-productdb` and `knx-cli` tests: **1,786 passed, 0 failed, 113
  ignored**.
- **ADR number:** 0093 was taken upstream meanwhile (wizards), so this is
  ADR-0094.
- **Not done:** product-database import (L2) and server/web upload (L3).
  `knx products ingest`/`.knxproj` keep their filename refusal for
  `.vd*`/`.pr*`.

## 2026-10-08 — German UI uses the informal du-form throughout

- User decision: the German catalogue addresses the reader as "du". 53
  formal places in `apps/knx-web/src/messages/de.ts` rewritten by hand
  (imperatives, "wenn Sie …", "Ihr/Ihre/Ihnen …"); the pronoun "Sie"
  (she/it/they) is unchanged. The convention is recorded in the catalogue's
  header comment.
- New guard `i18n.duForm.test.ts` refuses formal address in the German
  catalogue; negative control: it flags the previous catalogue.
- Verified on the frozen candidate (`inputs_frozen=1`, base `a642eaf9`): web
  build, `tsc`, flow-study/theme-fixture checks, **2,372 Vitest tests / 143
  files**, **170 intercepted Chromium tests**, five xtask checks and
  `git diff --check`. No Rust changed.

## 2026-10-08 — Add-device wizard with server preview and placement (ADR-0093)

- New modal **Add device** wizard (`DeviceWizard.tsx`): product (search the
  catalog or install a `.knxprod`), placement (installation only when there
  are several, line, building part), name/quantity/addresses, review, result.
  Entry points: explorer `+ Add device` rows under every line, under every
  installation's **Unassigned** bucket and under every room; **Add device…**
  in the command palette (aimed at the selected line or building part);
  **Add devices now** after a new project; **Add with wizard…** in the catalog
  (starts at placement). Parameters and group links stay in the device panel.
- `POST /api/devices` takes optional `installationId`, `buildingPartId` and
  `expected`. Placement runs as `MoveDeviceToBuildingPart` inside the same
  `Batch` (one undo step). Line, installation and building part must agree on
  one installation (`domain::resolve_catalog_installation`); unknown or
  disagreeing targets are a `400` before any ID is reserved. All three fields
  are part of the ADR-0069 replay fingerprint.
- New read-only `POST /api/devices/preview`: plans the very batch a create
  would apply and runs it, plus enrichment, on a copy of the project; no IDs
  reserved, no undo entry, no log. A create whose `expected` names/addresses
  differ from what it would now produce is refused with `409`
  `catalogPreviewStale`; the wizard refreshes the preview and asks again.
- The catalog's install report and creation-diagnostic wording moved to
  `CatalogInstallReport.tsx`, shared by catalog and wizard. KNOWN_LIMITATIONS
  U12 updated (a line-less device can now name its installation), new §167
  (what the stale check covers).
- Verified: gate on the frozen candidate (`inputs_frozen=1`, base `e96bfb5d`): web build, `tsc`, flow-study/theme-fixture checks, **2,370 Vitest tests / 142 files**, **170 intercepted Chromium tests** plus `device-wizard.e2e.ts` and `new-project.e2e.ts` repeated ×3 (**39 passed**), `cargo fmt`, `clippy -D warnings`, workspace Rust tests **3,571 passed, 0 failed** (178 ignored), layering/anchors/ledger/corpus-gate checks and `git diff --check`. `check-headers` refused two new first-line headers over 100 columns; only those two comment lines were shortened afterwards and `check-headers`, `diff --check`, `tsc` and the wizard's Vitest file were rerun green.

## 2026-10-08 — Achievement popups linger longer and leave animated (ADR-0089)

- At the user's request: an achievement popup now stays **9 s** (was 6 s)
  and then **slides out** instead of vanishing; × uses the same exit.
- `useToasts` marks the popup `leaving`; `ToastStack` removes it on the
  `animationend` of `knx-achievement-out` (twice the entry duration, motion
  easing). Without motion the leaving toast is invisible at once and a 1 s
  fallback removes it. Error and fun toasts are unchanged.
- Tests: hook timing (9 s, leaving, fallback, early removal), stack
  (`toast--leaving`, only the exit animation's end counts), and Chromium:
  the exit animation runs and removes the popup in under 800 ms; with
  reduced motion the popup is at once invisible and removed by the
  fallback. Removing the CSS exit rule turns the browser test red.

## 2026-10-08 — New-project wizard with an atomic starting structure (ADR-0093)

- **New project…** is now a five-step wizard (project, topology, building,
  group structure, review). Topology pre-fills area 1 / line 1.1. The building
  step edits buildings, floors, rooms and distribution boards, with a quick
  floor fill. The group step edits main and middle groups and offers two
  data-file presets (`apps/knx-web/presets/group-structure/`, admitted
  strictly). Free style skips the group step. **Create project** works from
  every step and Enter on the first step keeps the old fast path. After
  creation a "created, not yet saved" page offers **Add devices now** (opens
  the catalog on the first line for now; the add-device wizard is the next
  package).
- `POST /api/project/new` accepts an optional `seed`; unknown fields are
  refused. `knx_app::project_seed` applies it through the ordinary core create
  commands to the not-yet-installed replacement, on a private copy and before
  the unsaved-changes guard. A refused seed (`422`, kind `projectSeedInvalid`,
  wire path in the message) replaces nothing. An accepted one arrives with the
  project, without undo entries. Cap: 2,000 nodes. Limitation:
  [§166](KNOWN_LIMITATIONS.md#166-the-new-project-wizard-seeds-one-installation-from-a-fixed-preset-vocabulary).
- Verified on the frozen candidate (`inputs_frozen=1`): web build, `tsc`,
  flow-study/theme-fixture type checks, **2,355 Vitest tests / 140 files**,
  **162 intercepted Chromium tests** plus `new-project.e2e.ts` repeated ×3
  (**21 passed**), `cargo fmt`, workspace Rust tests **3,564 passed, 0 failed,
  178 ignored**, five xtask gates, `git diff --check`. The first Clippy step
  exited 255 with an empty log. The same inputs then passed
  `clippy --workspace --all-targets -D warnings` twice, the second time
  re-checking `knx-app`, `knx-server` and `knx-desktop`. The manual screenshot
  `porcelain-new-project.png` was retaken against a release server. The
  existing manual spec times out earlier in `dismissToasts`, so the shot came
  from a temporary copy with a tolerant dismiss; that copy was deleted.
- Not verified: native WebKitGTK, screen-reader output.

## 2026-10-08 — LCARS published and server deployed

- User explicitly requested commit, push and deployment after the local UI
  acceptance. Product commit `3817e6bb` published on main atop
  current MCP work; LCARS decision is **ADR-0092**, not the MCP's ADR-0090.
  Only the 32 owned application/study/docs paths were committed. KNXBench
  author/committer, no co-author; live and fetched remote refs read back.
- Integrated gate: **2,329 frontend tests / 138 files**, **158 intercepted
  Chromium tests**, TypeScript/Vite build, theme-fixture/flow-study type checks,
  five repository gates, diff check and verifier syntax pass on frozen input.
  An initial browser attempt aborted before assertions because its temporary
  socket path was too long; corrected only the harness, retained the failed
  attempt and reused four successful exact-input prerequisites.
- Docker image built from the committed product tree and verified first in a
  private bridge probe. PID 1 handles SIGINT/SIGTERM; probe stopped with exit
  **0 in 0.362 seconds**. No live bus request.
- Live `knxbench` server is up on HTTPS port **8484**; binary reports
  `knx-server 0.1.0-alpha.2+g3817e6bb`. All **89** served frontend files byte-match the
  gated production build. Authentication, hostname, host network, original
  environment and `/data` bind preserved; original TLS fingerprint unchanged.
  All **8** selected existing project/settings files retain their hashes.
- Previous stopped container retained as `knxbench-pre-lcars-20261008`, old
  image tagged `knxbench-server:pre-lcars-20261008` for rollback. Normal image
  tag now points to the verified new image. Sessions are memory-only, so a
  fresh login is expected. No release tag, native/whole-app accessibility or
  hardware-operation claim.
- The older dirty shared root was not reset, stashed or committed wholesale;
  concurrent community/evidence work is excluded and preserved. Durable
  source/gate/runtime evidence: [deployment receipt](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/design-studies/lcars/deployment-verification.json).

## 2026-10-08 — Approved LCARS theme integrated locally

**Delivery follow-up:** user authorized commit, push and server deployment.
Integrated on current main with existing MCP work preserved; LCARS decision
renumbered to ADR-0092 (0090 is MCP; 0091 is reserved by the intake workstream).
Publication/deployment results are recorded separately after actual verification.

- Visually approved study integrated into the actual application as the optional
  **LCARS** Theme dropdown entry. Complete dark/warm palette, existing fonts,
  recognizable elbow/segmented framing and rounded navigation; quiet native
  tables, explorer, inspector and diagnostics retain their existing workflows.
- [ADR-0092](adr/0092-lcars-built-in-presentation.md): controlled built-in
  `data-presentation` geometry, not an executable/importable v1 pack. Existing
  palette-token/contrast gates and acknowledged theme persistence retained;
  no new dependencies, schema, API/core/protocol or imported-pack capability.
- Density remains independent; finite Standard navigation feedback and Subtle
  transitions use existing motion settings. Off/OS reduction cancels live effects.
  Refused Save As remains an error/unsaved state, never decorative success.
- Root delivery verified: **2,286 frontend tests / 136 files**, TypeScript/Vite
  production build and fixture type check pass. Root build's **89 files** are
  byte-identical to the browser-exercised candidate build. Complete intercepted
  browser suite **158 passed**; real production-workbench CLI proof **33 named
  assertions**, with no unexpected requests/errors. Actual UI scale 1.5 and
  720×620/480×900 viewports exercised; screenshots visually inspected.
- Five repository gates pass on both the explicitly selected isolated candidate
  and delivered root;
  source self-review/static added-line scan performed, no independent reviewer.
  Documentation/manual/status and the study's superseded approval note updated.
- Local source delivery only: **no commit, push, deployment, Docker rebuild,
  release or live bus operation**. Native WebKitGTK/Orca, Firefox, native browser
  zoom and whole-app accessibility acceptance remain unverified. Details:
  [LCARS guide](DESIGN_LCARS.md), `design-studies/lcars/application-verification.json`.

## 2026-10-07 — LCARS interactive offline study; visual approval pending

**Update 2026-10-08:** visual approval received and production integration
delivered locally; see the newer entry above. The evidence below remains the
historical, standalone study scope.

- User approved the modernized LCARS brief and explicitly started the offline
  study. Delivered self-contained `docs/design-studies/lcars/index.html`,
  embedded existing fonts/OFL notices, a plain CLI browser verifier, exact-source
  receipt and screenshots. This is **not a shipped application theme or an
  importable v1 pack**; production integration requires separate visual approval.
- Native address table and independent marking/activation, project-room filters,
  retained form drafts, validated/duplicate-safe local edits, inspector controls,
  density/motion/neutral comparison and explicitly memory-only save simulation
  with progress/success/error/cancellation and newer-edit protection.
- Verified in system Chromium: **50 browser assertions**, no external page
  requests or console/script errors; embedded JavaScript syntax passed. Actual
  running effects stop on motion-off or OS reduction; no idle animation loop.
  Small-window table-space regression is covered. The 720px layout represents
  a 200%-zoom-sized layout viewport, not a native browser-chrome zoom test.
- No application/core/server source or dependencies changed. No live project,
  bus, Docker, deployment or release touched. Native WebKitGTK/Orca and full
  accessibility acceptance remain unverified. Details and source fingerprints:
  [study README](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/design-studies/lcars/README.md) and `verification.json`.

## 2026-10-08 — MCP: parameter visibility, paging and two real clients (ADR-0090 amendment)

- **Shared evaluation:** the parameter panel's stored-value evaluation
  (`evaluate_device`, `take_digits`, `decompose_module_qualified`,
  `resolve_mi_authority`) moved verbatim from `apps/knx-server/src/domain.rs`
  into `knx_productdb::device_evaluation`. Its two duplicate findings are now
  data (`EvaluationFinding`); the server maps them back to its DTOs with the
  unchanged wording. Server tests 699/0/45 unchanged.
- **Classification:** `DeviceEvaluation::value_status` → `Active`,
  `Inactive`, `Stale`, `Unknown`; `traversal_complete()` turns a missing
  activation after a budget stop into `Unknown` (ADR-0062). New
  `scoped_ids` maps a stored module-scoped id to its module and declared
  ref. 3 tests in `crates/knx-productdb/tests/device_evaluation.rs`.
- **`knx-mcp` visibility:** `get_device` values and `explain_parameter`
  carry `visibility` + `visibilityReason`; a declared id without a stored
  value reports `activeInModules` when only modules activate it;
  module-scoped values show their module's declaration. `notEvaluated`
  reasons: `noProductDatabase`, `programNotInstalled`,
  `noApplicationProgram`, `noDynamicTree`. Access is not applied, and
  `inactive` says nothing about download (`DownloadInvisibleParameters`).
  `schemaVersion` 2.
- **`knx-mcp` paging:** `get_device` pages communication objects and values
  separately (default 50), narrows with `linkedOnly`/`parameterVisibility`,
  omits `null` object members; totals and `visibilityCounts` cover
  everything. Largest real response 202 KB → 25.6 KB (median 11.8 KB).
- **`knx-mcp` fix:** `get_device` no longer leaks the projection's
  `resolution: NoDatabase` placeholder; it resolves the product like the
  server (`Resolved` + catalogue, `NotInDatabase`, `NoDatabase`).
- **Tests:** 16 tool tests, 2 stdio, 6 unit (+ argument-schema case);
  mutation sweep 11/11 killed after two added tests (repeated row, module
  declaration).
- **Gate (2026-10-08):** web build, fmt, diff-check, workspace Clippy,
  workspace tests 3,546 passed / 0 failed / 178 ignored; corpus tests through
  the moved evaluation: knx-server `com_object_activation_corpus`,
  `open_reference_project`, `http_project_routes` 5/5 plus 2 lib tests,
  knx-productdb `dynamic_tree` + `parameter_views_corpus` 7/7 (release,
  product corpus); xtask layering, anchors, ledger, corpus-gates green,
  headers green after a comment-only fix to the new test file's header.
  (The 4,376 recorded for the 2026-10-07 package counted knx-store and
  knx-productdb twice; the workspace figure then was about 3,545.)
- **Live probe:** copies of `project123.knxdb`, `project_migrated.knxdb` and
  the imported KV demo with a copy of the installed product database: all
  devices evaluated, 1,262 `active` + 81 `inactive` of 1,343 values per real
  project, KV 9/9 module-scoped `active`, product `Resolved` 36/36. All
  copies byte-identical.
- **Real clients (2026-10-08):** Claude Code 2.1.289 (`--mcp-config`,
  `--strict-mcp-config`, only `mcp__knx__*` allowed) and Hermes (temporary
  `HERMES_HOME`, `hermes mcp test`, `hermes chat -t knx`). Both answered a
  visibility question correctly from the saved data and refused a parameter
  write; Claude Code answered the 577-object device by paging
  (verified against the database: 0 links, 0 values). Findings fixed on the
  way: the unbounded `get_device` (Hermes spilled 49 KB to a file), the
  `resolution` placeholder, and an unbacked "does not take effect" claim for
  inactive values.
- **Side effect noticed and reverted:** importing the KV demo with `knx
  import` rewrote 14 manufacturer names in the installed product database
  from the project's embedded master data (documented behaviour: the last
  ingested `knx_master.xml` wins). The file was restored byte-exact from the
  pre-import copy.
- **Docs:** KL §165 retitled and rewritten, ADR-0090 amendment, manual
  chapter 23 (visibility, paging, client notes), agent skill, ROADMAP,
  ARCHITECTURE, manual status row.

## 2026-10-07 — Read-only MCP server and agent skill (ADR-0090)

- **User decisions (grill-me):** power users bring their own agent; v1 is a
  read-only MCP server plus an agent skill, **no chatbox**; bus and hardware
  excluded absolutely; changes only as group-address CSV proposals a person
  applies; privacy by documentation plus hard exclusions; saved files named
  at launch, stdio only; `rmcp`; eight tools; experimental with
  `schemaVersion`; separate release binary.
- **Storage:** `knx_store::open_existing_read_only` and
  `knx_productdb::open_read_only` open with `SQLITE_OPEN_READ_ONLY` +
  `query_only`, refuse what the migrating openers refuse, create nothing,
  and migrate an **older** file only as an in-memory copy (SQLite backup
  API; rusqlite `backup` feature, no new crate). Byte-identity tests for
  current, older, newer, foreign, empty, missing and non-SQLite files
  (`read_only_open.rs` in both crates, 9 + 5 tests). Both wait up to 5 s
  for a writer's lock (explicit `busy_timeout`, pinned by a test that holds
  `BEGIN EXCLUSIVE`), and the in-memory copy runs in one backup step that
  pauses instead of spinning on a busy source. The product database's
  migration loop is shared (`migrate_from`), behaviour unchanged.
- **`apps/knx-mcp`:** `args` (aliases, no path ever reaches a tool),
  `workspace` (snapshots reloaded when length/mtime change; a vanished file
  is an error), `tools` (`project_summary`, `search`, `get_device`,
  `get_group_address`, `find_issues`, `diff_projects`, `explain_parameter`,
  `validate_ga_csv`), `issues` (11 structural checks), `diff_render` (flat
  change list), `server` (rmcp glue; all tools `readOnlyHint`, arguments
  `deny_unknown_fields`, work on the blocking pool). Every response:
  `schemaVersion`, `experimental`, `dataNotice`, `source`, `result`.
- **Enforcement:** `check-layering` gains a production-graph rule: knx-mcp
  reaches none of knx-net, knx-server, knx-secure, knx-etsproj, axum,
  hyper, reqwest (negative control: adding knx-net fails the gate).
- **Tests:** knx-mcp 6 unit + 13 tool + 2 real-binary stdio tests
  (handshake, eight tools, refusals, clean exit, startup failure).
  Mutation sweep: 8/8 guard mutants killed by the intended tests
  (read-write open ×2, missing nothing-saved check, no reload, accepted
  unknown arguments, inverted severity filter, disabled duplicate check,
  zero busy timeout).
  Live probe against copies of two real projects (one schema v9) with the
  installed product database: decoded parameters, smuggled `path` refused,
  every file byte-identical.
- **Delivery:** `linux-appimage.yml` builds `knx-mcp-x86_64-linux` and
  publishes it with a `SHA256SUMS` beside the AppImage on tags (not yet
  exercised by a tag run). Skill: `integrations/agent-skill/knxbench/SKILL.md`.
  Manual chapter 23 "AI agents over MCP".
- **Limitations:** KL §165 (saved state only, visibility not evaluated,
  experimental shapes, client coverage).

## 2026-10-07 — Achievements, package 2: the other 27 (ADR-0089)

- **Catalogue complete: 38.** Import and integrity (lossless-move,
  archaeologist, spot-the-difference, spreadsheet-whisperer, documented,
  open-sesame, clean-sheet), structure (name-giver 100, dpt-sommelier 10,
  master-builder 10 rooms, drag-racer 25, assembly-line ≥ 50), read-only
  bus (first-contact, fly-on-the-wire 60 min, census, chain-of-custody,
  light-show ≥ 100, clean-bill, trust-but-verify), verified commissioning
  (right-address, first-download, commissioner: 10 different addresses),
  bug-hunter, the hidden read-only-friday, green-phosphor and
  error-culture, and bus-master (all others).
- **Rules:** `event` gains typed `where` conditions; new kinds `steps`,
  `distinct` (subject markers in the record, within the server's id rule)
  and `allOthers`; `threshold` works on any project measure;
  `localHours` takes an optional weekday. The server is unchanged.
- **Events:** 18 new event types, emitted where the outcome is confirmed
  (table in ADR-0089). Downloads count only with every block read back and
  the restart not left unconfirmed; address programming only when the
  device answers at the new address. Still no send event.
- **Changed triggers, reported:** #19, #26 and #34 were not detectable as
  worded and were replaced (ADR-0089, KL §164).
- **Found and fixed:** the overview would have shown a progress bar for a
  locked *hidden* achievement with a goal (`error-culture`), giving it
  away. It now shows none (regression test).
- **Tests:** rule kinds (red first), project measurement, catalogue
  invariants (38, no `--` in ids, one `allOthers` and it comes last),
  emit sites (download, address programming, readiness/compare, error
  toast under StrictMode, monitor start/minutes/disconnect, CSV applied
  vs declined, ETS import incl. password). Mutation checks on the
  verified conditions.

## 2026-10-07 — Boarisch and Klingonisch/Klingon join the language picker

- Shipped playful `bar` and `tlh` packs, with 335 curated UI messages each.
  Available without import; typed partial catalogues preserve placeholders and
  use the existing direct English fallback. Detailed technical/safety notices
  intentionally stay English, disclosed in Settings. No linguistic accuracy claim.
- Picker labels are fixed self-names: English, Deutsch, Boarisch and exactly
  Klingonisch/Klingon. Settings, New Project and the first-run guide agree.
  Imported languages retain their own names; an imported replacement of a
  shipped fun pack keeps its data/name on export, appears once, and removal
  restores the shipped pack. Missing replacement keys never merge from it.
- Product-data language options show autonyms and the exact regional tag
  (e.g. Deutsch (de-DE)); unknown/malformed/runtime-unsupported names fall
  back to the raw tag. Option values, project data and storage schema unchanged.
- Verified in the integrated root: Vitest **2,193 passed / 127 files**,
  `npm run build` (TypeScript and Vite), `git diff --check`. Real Chromium
  on the offline existing Settings fixture: all four languages switch, names
  remain fixed, `tlh` persists across reload, foreign settings retained,
  no page errors or unhandled/API-external requests. Self-review only.
- No Rust changes, production-server restart, Docker rebuild, hardware
  interaction, commit or push. Native WebKitGTK not exercised in this package.
- Details: [Language packs](LANGUAGE_PACKS.md), manual settings chapter,
  KNOWN_LIMITATIONS §66; handover receipt `.ai/logs/2026-10-07_codex_fantasy-language-packs.md`.

## 2026-10-07 — Achievements, package 1: mechanism and 11 achievements (ADR-0089)

- **User decisions (grill session):**
  - **Who and default:** every user, on by default; off means nothing is
    counted and nothing is shown.
  - **Content:** light-hearted and serious themes mixed. Bus and
    commissioning only as verified results, never telegram volume.
  - **Storage:** separate `achievements.json`, never in a project.
  - **Where it runs:** detection in the frontend.
  - **Presentation:** Steam-style popups (no sound) and an overview dialog.
  - **Rarity:** no telemetry; tiers instead of rarity percentages.
  - **Mix and names:** about 25 % hidden and 20 % with progress; names per
    language.
  - **Delivery:** two packages, 38 achievements in total.
- **Server:** `achievements.rs` keeps a record that only grows. Unlocks keep
  their earliest time, counters their highest value, and unknown ids and
  members are kept. A newer file is refused untouched and writes to it get
  409. A damaged file is moved aside. A reset moves the record aside
  instead of deleting it. Ids, timestamps, counters and sizes are checked.
  - Routes: `GET /api/achievements`, `POST /api/achievements/record` and
    `/reset`, behind the guard, under their own lock. Refusals, quarantines
    and resets go to the session log.
  - Atomic write and move-aside are shared with `settings.rs` through
    `data_file.rs` (a refactor; settings tests unchanged and green).
- **Web:**
  - `achievementCatalog.ts` (pure data, 5 rule kinds), `achievementRules.ts`
    (evaluate/merge) and `achievementTracker.ts`. The tracker buffers
    events until the record has loaded, keeps failed saves in an outbox
    without a retry loop, and turns read-only on `refusedNewer`/409.
  - `achievementEvents.ts` is the channel; it has no event type for
    sending to the bus. `useAchievements` and `konami.ts` handle the hook
    and the Konami code (ignored in text fields).
  - New `achievement` toast kind (at most 2 popups plus a summary,
    animation behind the motion guard), `AchievementsDialog`, and an
    Achievements section in Settings (switch, confirmed reset).
  - Entries in the File menu (hidden while off) and the command palette
    (`open-achievements`). Ten new outline glyphs. DE/EN strings.
- **Achievements in this package:** welcome-site, foundation, palette-pro
  (25), dark-side, polyglot, seatbelt (successful autosave), time-traveller
  (100 undos), mega-site (≥ 1000 GAs), and the hidden night-shift (save
  at 2–4 am), christmas-elf and konami.
- **Limitations:** KL §164 (per installation, UI only, a concurrent
  increment can be lost, hidden only in the UI).
- **Next:** package 2, the remaining 27 achievements from the interview
  catalogue.

## 2026-10-07 — `knx-server` leaves on SIGTERM instead of being killed

- **Finding:** the binary installed no signal handler. As PID 1 in the
  container the kernel discarded SIGTERM, so every `docker stop` waited
  10 s and ended in SIGKILL (exit 137; reproduced with the image from
  4cc0ec05, `SigCgt` of PID 1 without SIGTERM). A running bus monitor's
  tunnel stayed occupied on the gateway until its heartbeat timeout.
- **Behaviour:** `apps/knx-server/src/graceful_stop.rs`. SIGTERM/SIGINT
  stop accepting connections (`axum::serve(..).with_graceful_shutdown`,
  plain and TLS listener); requests in flight get 5 s (`STOP_GRACE`), a
  second signal ends the wait; then the bus monitor is stopped and a line
  scan cancelled (`release_bus`, 2 s), exit 0. After a forced end the
  process exits via `process::exit` because dropping the Tokio runtime
  would wait for `spawn_blocking` work without a limit. Device download
  and address programming are not waited for (KL §163).
- **Also fixed:** `.dockerignore` sent `data/` (the usual bind mount with
  user projects and the root-owned `0700` TLS directory), `OriginalData/`
  and other private local data into the build context; with the TLS
  directory present `docker build` failed with `permission denied`. The
  manual's update recipe used `docker rm -f` (SIGKILL) and now stops first.
- **Verified:** 7 unit tests (paused clock: drained, grace expired, second
  signal, server error, no signal, bus monitor tunnel disconnected via
  `FakeTunnel`, idle bus) and 4 tests against the real binary
  (`tests/signal_stop.rs`: SIGTERM with an idle keep-alive connection
  < 3 s, SIGINT, stalled request cut off after ≈5 s with exit 0, second
  signal). `knx-server` 674 passed / 0 failed / 45 ignored, Clippy
  `-D warnings`, fmt, five repository gates. Docker image rebuilt: PID 1
  `SigCgt` now includes SIGINT/SIGTERM; `docker stop` 181 ms (HTTP) and
  306 ms (HTTPS with password), exit 0 instead of 137.
- **Not verified:** release of a real gateway tunnel on stop (fake tunnel
  only; no hardware run).

## 2026-10-07 — `knx-server` speaks HTTPS by itself (ADR-0088)

- **User decisions (grill session):** protect browser ↔ `knx-server`
  only; LAN/VPN, not internet; desktop shell untouched; TLS built in
  rather than a Caddy proxy (host networking would leave the plain port
  open beside a proxy); self-signed by default, own PEM files optional;
  expired own certificate starts with a warning, broken files refuse.
- **Behaviour:** `KNX_TLS` `auto` (default: HTTPS exactly when a password
  is set or files are named) / `on` / `off` (loud warning on a networked
  server). `KNX_TLS_CERT` + `KNX_TLS_KEY` for own files; `KNX_TLS_SAN` adds
  names to the generated certificate. Generated certificate: ECDSA P-256,
  SANs localhost/127.0.0.1/::1/host name/extras, EKU serverAuth, 825 days
  (Apple's ceiling), kept in `KNX_DATA_DIR/.knxbench-tls` (`0700`/`0600`),
  renewed at a start within 30 days of expiry or when names change, SHA-256
  fingerprint in the banner. Plain HTTP on the TLS port gets a `307` to
  `https://`. Handshakes run in their own tasks with a 10 s limit. Session
  cookie is always `Secure` over HTTPS. `paths.rs` refuses the TLS
  directory for every route (relative, absolute, symlink); `/api/fs/list`
  hides it.
- **Dependencies:** `rustls` (ring provider, no aws-lc: its OpenSSL licence
  is not allowed by `deny.toml`), `tokio-rustls`, `rcgen`, `yasna`, `time`.
  `cargo deny check`: advisories, bans, licenses, sources ok.
- **Verified:** `knx-server` tests 663 passed / 0 failed / 45 ignored,
  including `tests/https_listener.rs` (real rustls client: trusted
  handshake, uncovered name refused, same-port redirect, silent clients do
  not block, rejecting client does not break the listener). Clippy
  `-D warnings`, fmt, five repository gates. Smoke test of the real binary:
  banner fingerprint equals `openssl x509 -fingerprint -sha256`, SANs/EKU/
  `CA:FALSE` as designed, curl with `--cacert` 200, without trust exit 60,
  plain HTTP 307, login cookie `Secure`, reserved directory refused by
  fs-list and save-as, restart reuses the certificate, `KNX_TLS=off`
  warnings, broken PEM refuses to start, no password stays loopback HTTP.
- **Container and browser (same day):** image rebuilt from `4cc0ec05`, the
  local `knxbench` container (host network, port 8484, password set)
  recreated with its previous configuration. It generated its certificate
  (SANs include the host name), `http://127.0.0.1:8484/` redirected in
  Chromium to an `NET::ERR_CERT_AUTHORITY_INVALID` warning (authority, not
  name), the certificate Chromium received has the same SHA-256 as the
  container log, login over HTTPS set `knx_session` with `Secure`,
  `HttpOnly`, `SameSite=Strict`, a 35-device project opened through the
  file picker (which does not list `.knxbench-tls`), zero non-HTTPS
  resource requests, `isSecureContext` true.
- **Not verified:** Firefox, Apple devices, a provided (non-generated)
  certificate in a browser. Limitations: KL §22.

## 2026-10-07 — One name everywhere: history rewritten, app identifier changed, alpha.5

- **User decision:** no personal identity may remain in the repository, its
  history, its artifacts or the agent instructions; everything is `KNXBench`.
  The repository lives at <https://github.com/KNXBench-Labs/KNXBench>
  (private), recreated from the rewritten history.
- **History rewrite** with `git filter-repo` over all 2,261 commits, every
  local branch, the three stashes and the tag: every author, committer and
  tagger is `KNXBench <github@knxbench.com>`; the personal account name, the
  personal address, the full name, the home-directory user name and the old
  data-folder identifier were replaced in every text file version and every
  commit message. A scan of all reachable objects found no remaining hit; no
  binary file contained one, so no binary was touched. The old to new hash
  mapping is [docs/history/COMMIT_MAP_2026-10-07.txt](history/COMMIT_MAP_2026-10-07.txt)
  ([known limitation §162](KNOWN_LIMITATIONS.md#162-commit-hashes-cited-before-2026-10-07-refer-to-the-rewritten-history)).
  A verified bundle of the pre-rewrite repository stays outside the
  repository as a local, unpublished backup.
- **Desktop app identifier** is now `com.knxbench.knxbench-labs`
  ([ADR-0087](adr/0087-desktop-app-identifier.md)); the data folder moves with
  it. No automatic migration ([§161](KNOWN_LIMITATIONS.md#161-alpha5-does-not-pick-up-the-alpha4-data-folder));
  the single existing installation was copied by hand and checked with `diff -r`.
- **Versions:** CLI, desktop shell and web frontend `0.1.0-alpha.5`. The
  `v0.1.0-alpha.4` tag and pre-release were withdrawn; `v0.1.0-alpha.5`
  replaces them ([ALPHA_FINAL_GATES §14](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md#14-alpha5-identity-rewrite-and-replacement-pre-release)).
- **Gate on the tagged commit** `aca70fd7` (fresh build directory, offline):
  Rust 3,430 passed / 0 failed / 178 ignored, Vitest 2,162, Playwright 155,
  private corpus 143 in 31 targets, Clippy, fmt, `cargo deny`, `npm audit`,
  the five repository gates and `check-appimage` green. The AppImage was
  built with remapped build paths and scanned: no old identity string. Started
  once natively on Wayland. Self-review; no independent review of alpha.5.
- **Published:** the [`v0.1.0-alpha.5` pre-release](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.5)
  (AppImage SHA-256 `8f69064f…0e2ce2`, `SHA256SUMS`), downloaded back and
  verified byte-identical.
- `knxprod-crawler` is a separate private repository; its dead link was removed.
- On the recreated repository the **"CI" workflow is disabled** for now (user
  decision); "Linux AppImage" and Dependabot stay active. Re-enable CI with
  `gh workflow enable CI` before relying on push checks again.
- Agent instructions (`AGENTS.md`/`CLAUDE.md`), cloud-session rules and the
  session-start script commit as `KNXBench <github@knxbench.com>`.

## 2026-10-07 — Branch consolidation integrates the post-alpha AppImage launcher

- User-authorized consolidation: retain `main`, integrate the complete KL-158
  packaging source from `318955d189fb`, and retire the other remote branches
  only after a verified full-branch Git bundle. Old Flow/native/stash WIP is
  preserved outside GitHub, not replayed over newer main functionality.
- Documentation conflicts preserve both current DPT/inference/Flow records
  and the launcher records. Build prerequisites now state Python 3 and the
  tested Tauri CLI pin. No Rust/npm dependency, domain/storage/schema or bus
  behavior was added by this integration.
- Fresh merged-candidate checks: Rust **3430 passed / 0 failed / 178 ignored**
  in 194 result blocks, workspace all-target Clippy `-D warnings`, fmt,
  launcher **19**, Web **2162**, frontend build. Ordinary workspace execution
  was loopback-only/offline; ignored corpus suites were not promoted to passes.
- Real Tauri AppImage build and package gate passed. Actual generated GTK
  runtime hook contains the owned display policy; development artifact SHA-256
  `c1cc776963f09117344b66b61832c281a9e7da1584febe0ae9b124938de8d910`. It names the earlier base plus
  dirty candidate state, not a clean tagged release.
- **Not rerun:** fresh native X11/Wayland startup, browser e2e, screen readers,
  GPU portability and private corpus. Earlier native receipts remain historical.
  No release/tag/asset replacement, deployment or KNX contact.
- Consolidation/deletion receipt: `.ai/logs/2026-10-07_codex_remote-branch-prune.md`.

## 2026-10-07 — DPT inventory, honest boundaries and rounding that stays in bounds

- Audited the authorized DPT-AS v02.02.01 original: 251 extracted pages, 250
  matched printed footers, 280 contents entries, 454 numbered IDs/103 main
  numbers. Public audit matrices distinguish 305 format-coded IDs from 149
  explicitly unsupported IDs; format coverage is not full subtype conformance.
- RED/GREEN numeric fixes: 8.010 cannot round to invalid-data 7FFFh; F16 and
  scaled V32 reject out-of-range engineering inputs before quantization.
- Core `validate_group_write_dpt` is shared by HTTP/CLI generic writers. Nine
  explicit parameter-only subtypes are refused for explicit/project-resolved
  DPTs and explicit/legacy input formats, before sending. Parameter/diagnostic
  encoding remains available; unverifiable FB exceptions fail closed.
- Corrected §61’s blanket structured-DPT/LTE rationale and charset support
  overclaim. Recorded 249.600’s overview omission and conflicting width label.
- Tests/evidence: `dpt_spec_document_inventory`, `dpt_spec_semantics_audit`,
  `http_bus_write`, `cli_bus_dpt`; integrated-root Rust gate 3418 passed / 0
  failed / 178 ignored (191 blocks), workspace all-target Clippy `-D warnings`,
  CLI/server debug build and five xtask audits passed. Fmt and whitespace
  checks passed. Wider gate/provenance and local integration in
  `.ai/logs/2026-10-07_codex_dpt-document-audit.md`. Self-review only; no Web,
  native, ignored-corpus or hardware acceptance. Published by explicit user
  request as `eb0abc6a8e75c614a489af82ab9ac810b5ffb740`; exact live main ref verified.
- Docs: [full inventory/scoped review](spec-audits/2026-10-07-dpt-document-audit.md),
  KL-61, Compatibility, Roadmap. No UI/core-storage schema change, dependency,
  bus contact, deployment, release, full ETS claim or automatic scope expansion.

## 2026-10-07 — Evidence of record, and the house downloads 32 of 35 devices

- ADR-0086 (maintainer decision): the KNX specification, product databases and
  project files are enough evidence; where they are silent but a working
  solution exists, it ships as a named, disclosed **inference**. Supersedes
  ADR-0048 decision 5 (a real device per family). Safety gates unchanged: a plan
  resting on an inference is always Untested and needs the acknowledgement.
- `knx_productdb::inference::Inference` travels image → plan
  (`plan_memory_download_with_inferences`) → `PreparedDownload::inferences` →
  `SupportLevel::Untested { inferences }`; shown by `knx device readiness`,
  `knx products coverage`, `knx device download` (under the UNTESTED line),
  `/api/device-readiness` (`detail`) and `/api/device-download/plan`
  (`support.inferences`).
- Inference `union-later-member`: of two active members of one union that share
  bits, the later one in the parameter tree is written (matches the octet ETS
  left on 1.1.11). A non-union overlap is still refused, now naming both
  references (`ImageError::Overlap`).
- Documented, no inference: `LdCtrlTaskCtrl1` (MP §3.31.2 segment type 4,
  `task_control_1`) and the machine-5 task segment, which the KNX Cookbook
  *Load Controls* (`02_03_01` §2.3) names `AbsCObjSeg`, "not transmitted on the
  bus". Inference `machine-5-after-restart`: a machine-5 event after the final
  restart is not sent. A machine-5 event before it is still refused.
- Group objects (RESEARCH §19.18): `Priority="Alert"` written as urgent
  (inference `alert-is-urgent`, a 17 731-object census of the corpus's base
  images); an enabled `ReadOnInitFlag` — product or project instance, now
  carried as `FlagOverrides::read_on_init` — is disclosed as not representable
  on `070nh` (`read-on-init-not-on-070n`, Resources NOTE 85). `High` and floats
  stay refused (contradicting product data).
- House project: 32 untested, 2 unsupported (`MV-0012`), 1 excluded (was 17/17/1).
  Product corpus at defaults: 1 verified + 89 untested of 246 programs (was
  1 + 77; RESEARCH §19.12, §19.18).
- Gates: unit tests per change (RED first), `house_readiness` and
  `download_coverage_corpus` against the root `OriginalData/` corpus, Rust tests
  of knx-productdb/knx-app/knx-server/knx-cli, clippy `-D warnings`, fmt, xtask.
  No bus contact; no new download has run on hardware.

## 2026-10-07 — Flow gets room to breathe, a window and actual project links

- User-approved grill-me scope: larger measured canvas/growing world; Inspector
  space only on selection with Close; maximized view with Escape/keyboard boundary;
  dedicated browser/Tauri Flow role; initially-on, switchable auto zoom with manual
  override and one-shot Show all.
- Readability-first graph ranks/components and compact fan-out replace activity
  clustering in the real view. Stronger movement is allowed by the user. Batched
  rearrangement, explicit Rearrange and Freeze preserve data/pulse semantics.
  Bounded curve avoidance uses label/value footprints; pulses follow the same
  geometry. Dense-mode reduction is visible, with no hidden recorded edges.
- Source monitor/model remains mounted across navigation and diagnostic/table
  switches; hidden animation/timers stop. A Flow window adopts the already collected
  source graph through BroadcastChannel, not another poll/tunnel. Times/expiry and
  send-time freshness are converted between document origins; repeated snapshots
  do not replay old events. Source loss retains a clearly non-live map and disables
  links. Capture loss/context/error/end diagnostics are copied too.
- Device, group-node and GA detail-row links reveal the exact entity in the main
  editor. Scope/generation, unique installation/entity/address and current server
  revision/incarnation guards refuse ambiguity, deletion/reused ids, external
  replacement and replacement during navigation. No second editing workspace.
- Regression discoveries fixed: SVG intrinsic-size/ResizeObserver feedback could
  grow the area continuously and make nodes unclickable; cloned models replayed
  old pulse events; UI scopes must not require secure-context-only randomUUID on
  LAN HTTP. Existing UUID-formatted load-client tokens remain unchanged.
- Gates: Vitest **2,162/2,162** (125 files), Web build, Chromium **155/155** in a
  loopback-only namespace, native `cargo check -p knx-desktop` and native adapter
  outcome tests. Production-load samples and boundaries: Flow §23 / KL §154.
  Self-review, not an independent review or release acceptance. No live bus/hardware.
- Docs: ADR-0085, Architecture, Flow §23, KL §154, Roadmap and user guide §7.
  No project/product migration or new dependency. Real WebKitGTK/Orca and a new
  packaged release remain unverified/separate; the existing Alpha tag is untouched.

## 2026-10-07 — The README hero does something, and Docker updates in one go

- README hero is now `docs/assets/readme/hero-add-device.gif` (900 × 467,
  56.4 s / 7.9 MB at 10 fps, dark Graphite theme; per step the camera
  glides for 1.6 s, the next click target is framed for 1.1 s, the result
  holds for 1.8 s): the real app against a real `knx-server` and
  the fictional sample house adds a push button from the catalog with a free
  address and links its first object to `0/0/2`. The camera zooms into each
  action in post-production. Regenerate with `playwright.readme-hero.config.ts`
  (command in `e2e/readme-hero.shots.ts`). The illustrated SVG hero and
  `tools/readme_hero_svg.py` are gone; the flow-view GIF stays further down.
- Manual: [Updating in one go](manual/user-guide/11-web-and-docker.md#updating-in-one-go)
  — pull, build with the commit stamped in, replace the container with
  `--network host`, wait for `/healthz`. Run twice against real Docker (fresh
  start and replacing a running container; `data/` survived). Installation
  §b links to it instead of repeating a bridge-only recipe.

## 2026-10-07 — KL-158: the AppImage can choose Wayland without unpacking its suitcase

- Packaging-only follow-up, requested after the alpha.4 release: the GTK hook
  no longer unconditionally overrides `GDK_BACKEND`. Wayland session hints
  select `wayland,x11`; explicit backend/renderer settings are preserved.
- Tauri's verified before-bundle/local-tool extension points prepare a pinned,
  checksum-validated GTK deploy source and inline the owned display policy;
  no global cache patch, Rust/npm dependency or domain/API/schema change.
- Actual development AppImage built on `b54cd5a5` + local launcher patch;
  source/artifact hashes and bounded platform evidence in
  [APPIMAGE_LAUNCHER](APPIMAGE_LAUNCHER.md#local-acceptance-receipt--2026-10-07).
  The parallel owner's published first-run guide is included, not reverted.
- Tests: launcher 19, eight isolated mutants caught, xtask 97; package check;
  direct-image native Wayland/X11, fallback and refusal controls; private
  Weston framebuffer inspected. Both workflow startup bodies replayed locally.
- Full Python tools suite: 56 tests, one unrelated existing `CLAUDE.md`
  shared-memory-marker contract failure, reproduced in the untouched root.
- Docs: ADR-0021 amendment, KL §158, troubleshooting, roadmap, ledger/matrix.
  **Historical local receipt before source integration**; fresh integration
  checks are recorded in the newer entry above. The existing tag/asset stays
  unchanged. No hardware/corpus or broader native-UI/GPU compatibility claim.

## 2026-10-07 — A first-run guide says what this build is before it says what to click

- New four-page introduction (ADR-0084): what KNXBench is and which release
  stage the running build is in (with an English/German switch), what works
  and what does not yet, where to start, and where Help and the debug report
  are. It opens by itself **once per release stage**, only over an empty
  workbench with no other dialog, and only when the server has acknowledged
  the settings record; every way out counts as seen. It gates nothing — the
  programming consent and `WriteAuthorisation` stay the safety boundary.
- Seen state: `onboardingGuide: { seenStage, version }` in `settings.json`,
  per installation (KL §160, `KL-160` accepted by the user).
- Its task buttons run the command palette's commands. Two commands were
  added: `open-catalog` and `show-introduction` (also **File → Show
  introduction…**). The palette now has fifteen entries.
- Corrected on the way: the Help topic "What this does not do" denied the
  ETS4/5 password dialog and device download (both stale); `knx-server`
  reported `0.1.0-alpha.1` in the `v0.1.0-alpha.4` release and is now
  `0.1.0-alpha.2` (ADR-0018 catch-up bump; programs still version
  independently, so no "all versions equal" check was added).
- Two layout faults in the first draft were caught in its own screenshot and
  now have e2e guards with negative controls: the chosen language button was
  accent-on-accent (invisible), and the card stretched to the overlay cap.
- Manual: First start describes the introduction (new screenshot); The user
  interface, Projects and Keyboard shortcuts updated; the command palette,
  File menu and (already stale) New project screenshots retaken.
- Gates: Vitest 2134/2134 (120 files), Chromium 146 passed (offline), the
  guide's spec ×3 12/12, `knx-server` tests 623/0/45 ignored, Clippy
  `-p knx-server` clean, xtask layering/headers/anchors/ledger/corpus-gates
  ok, manual screenshot run 2/2 in a loopback-only namespace. The header
  check first failed on one 101-column test header (comment-only fix, then
  rechecked).
- Not verified: WebKitGTK in the desktop shell, screen readers. `knx-web` and
  `knx-desktop` stay at `0.1.0-alpha.4`; the release that ships this bumps
  them together (`check-appimage` ties them).

## 2026-10-07 — `v0.1.0-alpha.4`: the first alpha is tagged and pre-released

- AR19, the user's decision: annotated tag `v0.1.0-alpha.4` on `514c0c54`
  (product code `2254eed0`, AR18 `READY`).
- A [GitHub pre-release](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.4) in the private repository, with the AppImage
  (SHA-256 `138444b4…c3fc`) and `SHA256SUMS`.
- Assets downloaded back and verified byte-identical
  ([ALPHA_FINAL_GATES §13](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md#13-ar19-release-decision-and-publication)).
- `RELEASE-04` is `DONE`. The installation chapter points to the
  pre-release.

## 2026-10-07 — AR18 recorded `READY`: the alpha candidate is release-ready, pending the user's decision

- Re-check round 4 (a fresh Codex session) returned **`READY`**
  ([verdict](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/review/2026-10-07-alpha-recheck-round4.md)). It tested 181
  fictional archives across CLI and both servers, plus seven archives from
  real writers. No blocking finding remains.
- Candidate `b8724d66`, product code `2254eed0`. AppImage
  `KNXBench_0.1.0-alpha.4_amd64.AppImage`, SHA-256 `138444b4…c3fc`.
- Gates: Rust 3402/0/178, Vitest 2076, Chromium 142, corpus 143/143.
- N14 (MINOR): an empty directory's payload is not checksummed. Disclosed
  in KL §159 and left for post-Alpha hardening.
- Record: [ALPHA_FINAL_GATES §12](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md#12-ar18-outcome-ready).
  `RELEASE-01`/`RELEASE-02` are `DONE`.
- Next: AR19, the user's release, tag and publication decision. Nothing is
  tagged automatically.

## 2026-10-06 — Round 3's condition: every part of a record must agree

- Re-check round 3: `READY_WITH_CONDITIONS`
  ([verdict](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/review/2026-10-06-alpha-recheck-round3.md)).
- N11: each local header must agree with its central record (flags, method,
  CRC, sizes, data descriptor, Unicode Path), and the records must tile the
  archive up to the central directory.
- N12: an empty directory written deflated (Java, `jar`) imports.
- N13: the last stale `500` in `ALPHA_CANDIDATE` corrected.
- Gate on `2254eed0` green: Rust 3402/0/178, corpus 143/143
  ([ALPHA_FINAL_GATES §11](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md#11-ar18-re-check-round-3-condition-r6-fixed-and-re-gated)).
- Next: re-check round 4.

## 2026-10-06 — A README that leads with the alpha and shows the bus thinking

- Root `README.md` rewritten: alpha warning first, verified features as bold
  statements, Docker quick start, links instead of limitation lists (those
  live in the manual's implementation status and known issues).
- Hero `docs/assets/readme/bus-nervous-system.svg`: an *illustration* of the
  flow view (SMIL motion, CRT in dark mode, green ink on porcelain in light
  mode), generated by `python3 tools/readme_hero_svg.py <out.svg>`.
- `docs/assets/readme/telegram-flow.gif` (2.75 MB, 11.7 s): the real
  `BusMonitorPanel` flow tab under the bundled CRT theme, fed synthetic
  traffic of the fictional manual sample house. Regenerate with
  `README_FLOW_GIF=<out.gif> npx playwright test -c playwright.readme.config.ts`
  in `apps/knx-web` (not part of the normal e2e suite; needs `ffmpeg`).

## 2026-10-06 — Agent internals back on stage

- `502dae60` reverted at the user's request: `.ai/`, the root goal and note
  files, the agent-memory tooling, `docs/PROJECT_CONTEXT.md`, the cloud-session
  kit and the 54 links are tracked and linked again, the SessionStart hook is
  back. The README rewrite stays.

## 2026-10-06 — Round 2's condition: a record's parts must agree

- Re-check round 2: `READY_WITH_CONDITIONS`
  ([verdict](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/review/2026-10-06-alpha-recheck-round2.md)).
- N7: directory records that carry data, or that are named like a file, are
  refused.
- N9: local header names must match the central ones.
- N8: a missing import file answers `422`.
- N10: two stale statements corrected.
- Gate on `754a66dd` green: Rust 3367/0/178, corpus 143/143
  ([ALPHA_FINAL_GATES §10](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md#10-ar18-re-check-round-2-condition-r4-fixed-and-re-gated)).
- Next: re-check round 3.

## 2026-10-06 — The re-check's conditions: the reader's names and the payload's budget

- Independent re-check: `READY_WITH_CONDITIONS`
  ([verdict](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/review/2026-10-06-alpha-conditions-recheck.md)).
- N1: members are judged on the names the `zip` reader decodes, too (Unicode
  Path, CP437 against UTF-8).
- N2: a protected payload is counted before it is unpacked.
- N3: `device restore` records a never-opened tunnel as failed.
- N4: `AppState::new` opens no product database.
- N5: a bad import file answers `422 projectNotImportable`, non-ZIP included.
- N6 (web reload) is handed to the UI owner and disclosed in KL §82.
- Gate on `3ede4817` green; Rust 3359/0/178, corpus 143/143
  ([ALPHA_FINAL_GATES §9](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md#9-ar18-re-check-conditions-r1r3-fixed-and-re-gated)).
- Next: re-check round 2.

## 2026-10-06 — The minor findings M1–M9 fixed as well

- M1: readers (*Open*, `doc-export`, `ga-export`, `diff`, compare, the
  serial lookup) never create a file, and refuse a foreign SQLite file or a
  store without a saved project untouched (`422 projectNotOpenable`). A
  failed `knx import` leaves `--store` alone.
- M2: Save writes everything in one transaction.
- M3: a second project part gets a report line.
- M6: a download whose tunnel never opened is recorded as `failed`,
  `written: no`.
- M7: `tools/run_corpus_tests.py` runs all corpus tests.
- M8: test defaults no longer touch the developer's product database.
- M9: `source-map-js` 1.2.2.
- 17 of 17 mutants killed. Gate on `faa3955f`: Rust 3350/0/178, Vitest 2076,
  Chromium 142, corpus 143/143, AppImage
  ([ALPHA_FINAL_GATES §8](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md#8-ar18-minor-findings-m1m9-fixed-and-re-gated)).
- The UI owner's follow-up `5d648560` is part of the candidate.
- Next: the independent re-check ([brief](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/review/AR18_RECHECK_BRIEF.md)).

## 2026-10-06 — UI follow-ups from AR16 and KL-61: honest style hint, styled type outcome

- New project dialog: the group-address style hint no longer says the style
  cannot be changed after creation; it points to the Project node, which
  restyles as one undoable step since 2026-10-02. The manual's known-issues
  entry for the stale hint is removed.
- Group-address Inspector: the KL-61 outcome sentence (`.dpt-outcome`, added
  by Alpha in `4b9e913e` without a rule) is now a muted 12 px note; a size
  conflict keeps the warning colour (compound selector).
- Tests: Vitest case rewritten RED-first (EN/DE, old wording absent); Chromium
  `ga-type-detail.e2e.ts` 3 (muted note; conflict warning in porcelain and
  graphite) RED before the rule; 4/4 CSS mutants killed; Vitest 2,071 / 117,
  `tsc -b` 0.

## 2026-10-06 — The reviewer's four conditions, met in code rather than waived

- Independent AR18 review: `READY_WITH_CONDITIONS`, with no CRITICAL
  finding; four IMPORTANT ones
  ([verdict](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/review/2026-10-06-alpha-independent-review.md)).
- F1: Open and Import refuse to replace unsaved edits. The server answers
  `409` with kind `projectUnsavedChanges` and checks twice; the web asks
  *Cancel / Discard changes and open / Save and open*.
- F2: duplicate or case-colliding archive members are refused by name, also
  inside a protected project's payload.
- F3: members are read only up to their declared size, with at most 512 MiB
  per archive (KNOWN_LIMITATIONS §159).
- F4: `knx import --store` refuses an existing file unless `--replace` is
  given.
- M4 (stray backslash in the wrong-password message) and M5 (limitation
  counts) are fixed.
- 16 of 16 mutants were killed. Gate on `64badb99`: Rust 3331/0/177, Vitest
  2076, Chromium 139, corpus 142/0. AppImage rebuilt
  ([ALPHA_FINAL_GATES §7](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md#7-ar18-conditions-c1c5-fixed-and-re-gated)).
- Next: the independent re-check of the fixes
  ([brief](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/review/AR18_RECHECK_BRIEF.md)).

## 2026-10-06 — The last red corpus pair turns green, and UI-04 gets its signature

- `http_device_compare` harness: built on `AppState::new(dir)` like the
  download tests, so its downloads find an activity history (test-only, by
  user decision). Corpus pair 8/0; revert mutant brings back both 503
  failures; `knx-server` 617/0/44; the review candidate's corpus count is
  142/142.
- `UI-04` accepted by the user as a disclosed Alpha boundary (ledger
  `ACCEPTED_BOUNDARY` / `USER_ACCEPTED`); no snapshot row remains
  `IN_PROGRESS`. Only the independent review stands before the AR18 verdict.

## 2026-10-06 — AR18 gates: the whole candidate goes through the scanner

- Full §5 gate on the clean revision `4b9e913e`, one script, all leases,
  offline: Rust 3317 passed / 0 failed / 177 ignored, Vitest 2071, Chromium
  139, fmt, clippy, `cargo deny`, the five `xtask` checks and `check-appimage`
  all green ([ALPHA_FINAL_GATES](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md)).
- Private corpus: 142 `OriginalData` tests selected from the source by script;
  140 pass, the known `http_device_compare` pair fails (stale test harness
  without an activity-history store; the server fails closed). Commissioning
  owner.
- AppImage built with `--remap-path-prefix`: no builder home path left in the
  binaries; offline Wayland start and API steps as in AR17.
- The independent review brief is [review/AR18_REVIEW_BRIEF.md](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/review/AR18_REVIEW_BRIEF.md);
  the review itself is a fresh Claude session the user starts.

## 2026-10-06 — KL-61: a group address finally says what it declares, not just what it ends up with

- Projection: `GroupAddressNode.dpt_detail` (optional on the wire) carries the
  stored declaration (`Absent`/`Empty`/`Value`/`Malformed` with text), the
  linked objects' types and the outcome, mirrored one to one from
  `knx_core::GroupAddressTypeOutcome`; `dpts` is unchanged and both come from
  one weighing. Bindings regenerated (four new types).
- Web: the group-address Inspector shows **Declared on the address**,
  **Linked objects state** and one outcome sentence (en/de); a size conflict
  is marked as a conflict. Trees without the detail show nothing extra.
- Tests: two projection tests (all six declaration states and four outcomes,
  the unlifted-store case) and seven Inspector tests; mutation sweep 8/8
  killed (four Rust, four Web), originals byte-compared.
- Manual: the group-address chapter and a known issue were stale since
  ADR-0078/T07 and are corrected.

## 2026-10-06 — The user signs the boundary list; AR18 gets a reviewer

- Nine rows accepted as disclosed Alpha boundaries (ledger `ACCEPTED_BOUNDARY`,
  disposition `USER_ACCEPTED`): `KL-1`, `KL-11`, `KL-125`, `KL-31`, `PDB-01`,
  `R-DYNAMIC-01`, `R-MODULE-03`, `R-MODULE-04`, `KL-158`. No `BLOCKED_EXTERNAL`
  row remains.
- AR18's independent review: a fresh Claude session started by the user from a
  written brief. `KL-61`'s declared-versus-linked display comes first.

## 2026-10-06 — AR16 done: the manual meets the app it describes, sentence by sentence

- Claim-by-claim pass by script and live probes: all 95 `knx` invocations,
  bold labels, env vars, routes, versions and every "not yet" sentence checked
  against the running application ([MANUAL_ACCEPTANCE](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/MANUAL_ACCEPTANCE.md)).
- Stale claims fixed: group-address style is changeable (Project node,
  undoable), autosave exists, drag and drop links group addresses, the export
  dialog has section choices, `knx diff --exit-code` exists, translations are
  used, per-program versions, device writes verified on one device, ADR-0078
  in KNX basics, workflow chapter on the sample house.
- Handed to the UI owner: the New project dialog's style hint still claims
  the style cannot be changed.
- AR16 DONE: the UI owner's closure receipt (`84bc32c3`) verified against the
  ledger and the issue plan; the manual is accepted at its tested scope, with
  the user-owned exceptions listed in MANUAL_ACCEPTANCE. `RELEASE-03` DONE.
## 2026-10-06 — UI-04 Web half: live bus activity; KL-61 binding wording

- New Bus tools tab *Live activity* (`BusActivityLive.tsx`, `liveActivity.ts`)
  reads `GET /api/bus/activity` every 2 s while open and visible: sessions with
  state/progress, short operations and eviction count, busy locks as
  "operation unknown", history-storage state (`unavailable` as alert),
  untracked kinds, server-restart note; partial/volatile wording throughout.
  Admission refuses a whole snapshot with unknown fields, states, locks or
  out-of-range counts; one-shot records reuse the history admission so the two
  readers cannot drift. A failed poll clears the last snapshot.
- *Activity history* reloads its first window every 3 s while it shows a
  running operation; after paging further it points to "Refresh from
  beginning" instead of collapsing the loaded pages.
- No global status-bar indicator, by owner decision (KNOWN_LIMITATIONS,
  "Web adoption, 2026-10-06").
- KL-61: `GroupAddressNode.dpts`' doc comment now describes the effective type
  (ADR-0078) and is regenerated into the binding; the declared-versus-linked
  detail needs a projection field first.
- Tests: 17 live + 3 history Vitest cases (RED first), 16/16 mutants; Chromium
  `activity-live.e2e.ts` 5 (EN/DE, 360/1440 px, poll/restart/storage), RED on
  the previous `BusDiagnosticsPanel` (negative control, byte-exact restore).
  Manual 07 gains "Live activity and activity history".

## 2026-10-06 — AR16 slice 1: the manual gets new photos of a house that does not exist

- All 21 manual screenshots regenerated from the real application (release
  `knx-server`, production frontend, offline namespace) with a fictional
  "Sample house" built by `tools/manual_sample_project.py` (unit-tested).
  Reproducible via `apps/knx-web/playwright.manual.config.ts`; recipe in the
  contributing chapter.
- Every screenshot passage re-read; stale claims fixed (the bus monitor's
  button is **Search**, start-up gateway search stated, welcome cards, help
  topics, catalog rows).
- Checklist [MANUAL_ACCEPTANCE](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/MANUAL_ACCEPTANCE.md); the claim-by-claim pass
  over chapters without screenshots is still open.

## 2026-10-06 — AR17: a real AppImage, weighed and sniffed (offline)

- Local candidate `KNXBench_0.1.0-alpha.4_amd64.AppImage` from `6b9b6818`
  (clean tree), SHA-256 `4c778104…80ef2`, `xtask check-appimage` ok. Record:
  [ALPHA_CANDIDATE](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_CANDIDATE.md).
- Offline smoke in a loopback-only namespace: launch, new/import/save/reopen,
  clear errors, no non-loopback socket — under X11 (private Xvfb, unmodified
  AppImage) and native Wayland.
- Fixed on the way: the desktop crate still said `0.1.0-alpha.1` while the
  bundled web frontend was `alpha.4`; the validator refused it.
- New `KL-158`: the AppImage forces X11; Wayland-only sessions need the
  documented workaround. Privacy checklist item 6: local builds carry the
  builder's home path.
- AR16: the user decided the manual stays on GitHub, with screenshots.

## 2026-10-06 — AR15 done: one page that says what the Alpha is, and what it is not yet

- New [ALPHA_SCOPE_MATRIX](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_SCOPE_MATRIX.md): capabilities by evidence
  level (verified / simulator-only / externally blocked / accepted / later),
  deployment, import and hardware boundaries, ledger totals (189 rows), and a
  release disposition for each of the 13 rows not yet accepted — nothing
  waived; the BLOCKED_EXTERNAL rows are *proposed* as disclosed boundaries
  for the user's AR19 decision.
- ALPHA_READINESS carries the AR15 recount. AR15 is DONE as a dossier; the
  final acceptance remains AR18/AR19.

## 2026-10-06 — AR15 slice 2: counting what is left, and saying "10" out loud

- LIMITATION_TRIAGE recounted by script: 119 heading lines = 108 limitations
  + 11 signposts; 107 rated (K1 5, K2 30, K3 58, K4 14), §105 unrated.
  New: §151/153/154/155/157; resolved to signposts: §149/150/152/156.
- Scheme set 10–14, 20, exact 21/23 now stated consistently (COMPATIBILITY,
  GAP_ANALYSIS_ETS, ROADMAP, KL §11, PRODUCT_DATABASE_CORPUS, manual).
- Store schema 10 / ProductDB 21 in the architecture tour and implementation
  status; ARCHITECTURE §8 and COMPATIBILITY point commissioning at KL §7.

## 2026-10-06 — AR15 slice 1: the manual stops saying "never" about downloads

- `DOC-03` DONE: README, getting started, workflow §12, command line, FAQ,
  known issues, reference and implementation status no longer claim that no
  download command exists; they now state the verified scope (one device,
  address programming refused under ADR-0058) and link the bus chapter. The
  workflow no longer suggests exporting a `.knxproj`; two resolved roadmap
  suggestions moved to a "resolved" note. Counts checked against code.
- `KL-9` ACCEPTED_BOUNDARY: `knx diff` as a Git external diff driver was run
  on a synthetic repository; the manual recipe handles Git's `/dev/null` for
  added/removed files and says `--ext-diff` is needed for log/show.
- `KL-16` ACCEPTED_BOUNDARY: advisories ok offline (DB 2026-10-03), GTK4
  migration PRs still open, Tauri 3 still alpha.
- `KL-46` ACCEPTED_BOUNDARY: rechecked against the named tests.

## 2026-10-06 — KL-157: a project upgrade is now all or nothing

- Found by AR15 while checking storage claims: `knx_store::migration::migrate`
  ran each upgrade step in autocommit mode. A failure (or a killed process)
  after the first step kept it while `user_version` stayed old, so every
  later open re-ran that step and failed — a project that never opens again.
- Fix: all pending steps plus the version bump in one `BEGIN IMMEDIATE`
  transaction, the pattern `knx_productdb::open_and_migrate` already used.
- Test `a_failed_upgrade_rolls_back_every_step_and_the_file_stays_reopenable`
  (v8 file, failing v9->v10 step): RED before, GREEN after; a mutant without
  the transaction fails it. The in-place upgrade without a copy is now a
  documented boundary (KNOWN_LIMITATIONS §157, manual known issues, DATA_MODEL).

## 2026-10-05 — KL-153: ABB's ETS4 shelf gets its library card (scheme 10, ADR-0083)

- Exact namespace `http://knx.org/xml/project/10` is admitted through the
  strict member validation of 21/23 (no foreign elements, no qualified
  attributes); generic scheme-11 readers and unknown reporting.
- Evidence: census of 146 packages / 1,391 XML members — no element,
  attribute or parent/child pair outside scheme 11's observed vocabulary.
- Tests: `knx-productdb/tests/scheme10.rs` (4 tests), three compiled mutants;
  ProductDB/CLI/server/xtask 1,589 passed / 0 failed; private product matrix
  unchanged (pin holds, no scheme-10 file in its scope).
- Release, 853 public files, fresh DB each: 692 → 837 installed (standard),
  852 with `--allow-large-package`. AR06P is DONE.

## 2026-10-05 — AR21: the nervous system passes its physical (with a doctor's note for big maps)

- Findings 6 and 7 verified fixed; all seven AR21 findings closed.
- Measured: 900 s small session light (17 % busy, 5–7 ms value lag, heap
  5.4→6.2 MiB, no plateau shown); a map growing to 500 nodes / ~2,500 lines
  is 88 % busy at only 2 telegrams/s — motion cost follows map size.
- `FLOW-01` DONE for the Alpha on that envelope; manual, known issues,
  §154, architecture and roadmap updated. Receipt for AR18:
  TELEGRAM_FLOW_VISUALIZATION §13–§22.

## 2026-10-05 — KL-151: Siemens fits through the door, if you hold it open (ADR-0082)

- `knx_productdb::PackageLimits` (`STANDARD` 64 MiB / 256 MiB, `LARGE`
  256 MiB / 4 GiB) and `install_package_with_limits`; `install_package` and
  the HTTP catalog route keep `STANDARD`.
- `knx products ingest --allow-large-package` opts in; a size refusal without
  it names the flag. Refused for `.knxproj` input.
- Measured (release CLI, 15 public packages the standard profile refuses):
  14 installed and verified, 1 scheme-10 refusal; worst 760.5 MiB RSS, 256 s,
  7.18 GiB DB ([measurement](contracts/PRODUCT_ZIP_LARGE_PROFILE.md)).
## 2026-10-05 — KL-37: catalogue, product block and DPT text say when they fell back

- New `src/languageFallback.tsx`: `fellBack(text, answered)` and
  `LanguageFallbackBadge` — the parameter panel's badge, now shared; its
  message keys moved from `parameters.untranslated.*` to `untranslated.*`, and
  the tooltip says "the original text" because it no longer speaks only of
  programs.
- `CatalogItem` gains `nameLanguage`, `visibleDescriptionLanguage`,
  `sourceLanguage`; the catalogue list marks name and description.
- Inspector: product text, catalogue name and application name carry their
  `*_language` / `*_source_language` markers; a communication object's DPT text
  is marked without a source. Only with a product language selected.
- Ledger `KL-37` → ACCEPTED_BOUNDARY (residue accepted in §37 by Alpha's slice
  3; the waiting UI half delivered). Manual 09 updated.

## 2026-10-05 — Telegram-flow note counts refused and ring-dropped telegrams (AR21 findings 6, 7)

- `flowModel`: `FlowEvent.complete` (false when the sender or any recipient
  was refused at the node limit); a refused sender's telegram becomes a
  lineless event without send times; `counters.eventsRecorded`.
- `flowAnimator`: incomplete events count as not (completely) drawn — never as
  bundled — while their remaining lines are drawn; events pushed out of the
  ring before a sync are counted as not drawn (not while hidden, not ones
  already drawn); the event baseline resets when the view gets a new model,
  so a new session's telegrams pulse again (found while fixing finding 7).
- Details and the §20 stall decision: TELEGRAM_FLOW_VISUALIZATION §21. Awaiting
  the AR21 rerun.

## 2026-10-05 — AR10 slice 3: the German report finally says "Schalten"

- New `knx_app::com_object_language` holds the communication-object
  translation rule (product layers only, only on a translation hit); the
  device-detail route and the documentation report both use it.
- `knx_report::ReportDeviceData::com_object_texts` carries the caller's
  translated texts; the renderer shows them in place of the project text.
- Ledger: `KL-66` ACCEPTED_BOUNDARY, `KL-37` WAITING_OWNER; AR10 DONE_SCOPED.

## 2026-10-05 — AR10 slice 2b: catalogue, product block and master data stop pretending to be bilingual

- `catalog_items`, `device_product` and the master readers name the stored
  language that answered (`de` → `de-DE`) or `None` for the package's own
  text, plus the declared `DefaultLanguage` of each source element.
- Wire: catalogue `nameLanguage`/`visibleDescriptionLanguage`/
  `sourceLanguage`; device-detail `product.catalog.*_language` (optional,
  ts-rs bindings regenerated); com-object `dpt_text_language`.
- `KL-64` ACCEPTED_BOUNDARY; `KL-37` IN_PROGRESS (report com-object text,
  UI consumption of the markers).
## 2026-10-05 — Parameter panel marks labels that fell back to the program's own text

- Consumes AR10 slice 2a (`b6a94c24`): `api.ts` gains `sourceLanguage`,
  `textLanguage`, `nameLanguage` and `enumOptions[].language`.
- With a product language selected, a field whose *shown* label (`text` before
  `name`) has no answering language is badged "Untranslated (en-US)" — or just
  "Untranslated" when the program declares no `DefaultLanguage`, never
  guessed; a translated label with untranslated option labels is badged
  "Options untranslated". Value-only options and id-only labels are not
  translation gaps. A panel line counts affected fields (folded `Access=None`
  fields included). *Package default* marks nothing. English and German.
- Tests: 5 new `ParameterPanel.test.tsx` cases (3 RED first; the two absence
  cases are pinned by mutants); 10/10 mutants. No server change. Catalogue,
  device-product and master surfaces wait for AR10 slice 2b.

## 2026-10-05 — AR10 slice 2a: the parameter panel admits when it is speaking the package's language

- `knx_productdb::query::parameter_views` reports, per text/name/enum label,
  the stored language that answered (`de` → `de-DE`) or `None` for the
  package's own text; new `program_default_language` reads the stored but
  previously unread `ApplicationProgram/@DefaultLanguage`.
- Parameter panel DTO: `sourceLanguage`, `textLanguage`, `nameLanguage`,
  `enumOptions[].language` (additive; values and fallback rule unchanged).
- Tests: one query test, one HTTP test; UI owner decides presentation.

## 2026-10-05 — AR10 slice 1: where every language comes from (and where none does)

- Trace of every language source and translated-text reader:
  `docs/research/backend-localization-paths.md`. Findings: the project schema
  has no project language; `ApplicationProgram/@DefaultLanguage` is stored
  but read by nothing; only communication-object overlays tell the caller
  whether a translation answered — parameter, catalogue, device-product and
  master overlays fall back silently.
- `KL-14` accepted as a boundary, pinned by `knx-etsproj/tests/project_language.rs`
  (a device's `InitialValueLanguage` is reported, never promoted; imports
  leave the string table empty). No production code changed.
- Next: expose the overlay fallback on the silent surfaces (`KL-37`/`KL-64`).

## 2026-10-05 — KL-156: the install report finally admits what it skipped (ADR-0081)

- `Parameter`/`ParameterRef` report every attribute outside their stored
  columns (`SuffixText`, `InitialValue`, `LegacyPatchAlways`, a union
  member's `Offset`/`BitOffset`, `ParameterRef/@Name`, …), through one helper
  shared by ingest and the new **schema v21** backfill. Nothing is newly
  interpreted.
- `migrate_v20_to_v21` adds the rows from the retained program blobs (once
  per package that parsed the blob, the v16 rule), re-derives measured
  install reports and `package.unknown_count`, and names a damaged blob
  (`ParameterAttributeBackfillError`, report `unavailable`).
- Evidence: `parameter_attribute_unknowns.rs` (8 tests, 6 compiled mutants
  caught), workspace 3,289/0/177, corpus-gated suites, a corpus probe in which
  all 102 `OriginalData` packages migrated from a v20 rewind equal a fresh
  install table for table, and a matrix re-pin whose five changed aggregates
  an independent Python recount predicts exactly.
- ADR-0080 marked Accepted (its gates were green at merge).

## 2026-10-05 — Parameter panel adopts ADR-0080's write-authority reasons

- `parameterAccessReadOnly`, `manufacturerCalculation` and
  `writeAuthorityUnavailable` — and the older `unsupportedControlKind`
  (ADR-0061) and `evaluationWorkBudgetExhausted` (ADR-0062), which also only
  had the server's English fallback — join Web's `ParameterDiagnosticKind`
  union and the English/German catalogues.
- UI owner's presentation decision (ADR-0080 "UI presentation"): fields whose
  effective access is `None` are folded per section behind a counting button
  (`aria-expanded`), shown read-only on request, never dropped; `Read` fields
  stay visible and read-only.
- Follow-on fix: a disabled field's caption said "Shared across every
  instantiation of this module" for every refused field; since ADR-0080
  device-level fields are refused too, so they now read "Not editable here —
  see the warnings for why." Module-scoped fields keep the shared caption.
- Tests: 9 new `ParameterPanel.test.tsx` cases (5 translations in German, fold
  with count/toggle/read-only, singular, no fold without `None`, caption by
  scope), 8 RED first; 8/8 mutants. No server change.
## 2026-10-05 — KL-142: the download tab asks what to write

The Download to device tab offers *What to write* before the plan: Complete
download (default), Parameters only, Group addresses only, Parameters and group
addresses. A partial choice sends `partial: { parameters, groupAddresses }` to
`POST /api/device-download/plan` (a complete one sends no `partial` member); the
plan then names the partial scope, the device check before writing, and every
`notWritten` application write (or that none is skipped), before the consent
dialog. A refusal (422) is shown without a plan; changing the scope drops a
shown plan; the radios lock while planning or downloading. Start, consent and
phrase are unchanged — the server re-derives the same partial plan from
`planId`. Tests: 10 new Vitest cases (panel + API body, RED first), 8/8
mutants, `e2e/device-download-scope.e2e.ts` (2, RED on the previous panel).
Mocked/intercepted only; no device contact. The comparison view still
compares the complete plan only.

## 2026-10-05 — AR07: the manufacturer's "hands off" now means hands off (ADR-0080)

- **Census.** Read-only, aggregate-only scan of 3,599 distinct application
  programs (`OriginalData` + the public crawler download): no `Dynamic`
  kind outside the evaluator's set besides the four ADR-0041 names; every
  `when/@test` is a `Condition_t` integer form; 1,199 allocator bindings,
  116,799 `ParameterCalculation`s (JavaScript and VBScript), 373 `Repeat`s.
  Table in PARAMETER_SEMANTICS_BOUNDARY.
- **Write authority.** *Project Schema23* §1.1.2.1 makes `Access` a user
  right. ProductDB schema v20 stores `ParameterRef/@Access`, indexes
  `ParameterCalculation` members and marks recorded programs; the panel
  refuses writes for effective access other than `ReadWrite`, for both sides
  of a calculation, and for unrecorded programs (fail closed), with three
  new warnings. Imported values are untouched.
- **Found:** `KL-156` — unstored `Parameter`/`ParameterRef` attributes are
  not reported (probe-confirmed). Next alpha package.
- **UI owner:** adopt `parameterAccessReadOnly`, `manufacturerCalculation`,
  `writeAuthorityUnavailable`; decide how `access: "None"` fields look.
## 2026-10-05 — Telegram flow: AR21 finding 5 corrected

A telegram whose targets were all refused at the model's node limit carries
`to: []` and has no line; the reduced-rendering note counted it as drawn
bundled. `queuePulses` now counts it as not drawn and marks the rendering
reduced (TELEGRAM_FLOW_VISUALIZATION §18; §16's "no third case" annotated).
Two `flowAnimator` tests at a one-node limit, RED on the previous code; four
mutants killed. `FLOW-01` stays IN_PROGRESS until the AR21 rerun.
## 2026-10-05 — Tunnelling through Docker's bridge network (AR14B, `1fd1664a`)

- `knx_net::TunnelReturnPath::RouteBack` sends the all-zero UDP Route Back HPAI
  (Core v01.06.02 AS §8.6.2.2) in the `CONNECT_REQUEST` (control and data
  endpoint), every `CONNECTIONSTATE_REQUEST` and the `DISCONNECT_REQUEST`;
  the gateway answers to the packet's source, which survives Docker's NAT.
- `knx-server` enables it with `KNX_TUNNEL_ROUTE_BACK=1`; the default keeps the
  own address. The CLI is unchanged.
- Evidence: four focused tests (RED first), 7/7 compiled mutants, workspace
  3,273/0/177. Not verified: a live tunnel from a bridge container; other
  gateways' Route Back support (KNOWN_LIMITATIONS §155).
## 2026-10-05 — Themes: one dropdown, CRT shipped, storage location shown (ADR-0079)

User decision: the preview cards in Settings › Appearance go ("Dropdown
reicht"), Modern Retro Green CRT ships with the application, the pane states
where themes are stored, Neon Grid and Bitcoin DeFi are removed.

- **Dropdown only.** `ThemePackManager` no longer lists themes or previews
  them; the `ThemePreview` plumbing (App → SettingsPanel → `useThemeId`) is
  gone and `SettingsPanel` takes `manageThemes` instead. An import is admitted
  as before and then installed *and selected* in one conditional write;
  replacing an installed id still asks first. Export/Remove act on the selected
  pack (Remove only for installed packs). Diagnostics and recovery export stay.
- **Shipped CRT.** `src/bundledThemes.ts` admits
  `themes/modern-retro-green-crt.knx-theme.json` through `parseThemePackText`
  at build time. Selecting it stores only `theme`; it is never copied into
  `uiThemePacks`. An installed pack with the same id wins and is listed once.
- **Storage location** stated in the pane (en/de): `settings.json`
  (`uiThemePacks`, `theme`) in the data folder — desktop
  `~/.local/share/com.knxbench.knxbench-labs`, server `KNX_DATA_DIR` (Docker
  `/data`). No API returns a host path (unchanged rule).
- **Neon Grid / Bitcoin DeFi removed** from registry, stylesheet and the
  `index.html` bootstrap list. A saved choice of either is kept, shown as
  System and reported as an unavailable selection.
- Tests: new `themeSettings.test.tsx` (10; written first, 8 of the first 9
  RED on the previous code — the shadowing case passes trivially without a
  shipped pack),
  `ThemePackManager.integration.test.tsx` rewritten (23; preview-only cases
  dropped, every write/consent/conflict/500/cache/late-file/out-of-order
  guarantee kept), `themePreview.test.tsx` removed with its feature,
  `e2e/theme-manager.e2e.ts` rewritten (12, incl. CRT selection + cold reload
  in Chromium). 12 code mutants killed.

## 2026-10-05 — Left-column splitters work again with a project loaded

- User report: with a project open, the separators between the navigation
  block, the Project Explorer and the diagnostics block did not resize
  anything. Cause: the explorer had `flex: 1 1 auto`, so its whole tree
  height entered the column's flex calculation and the column shrank both
  blocks — including any height a splitter set — to make room (with 40
  installations the navigation block was squeezed to ~37 px before anyone
  touched it). The happy-dom unit tests could not see layout.
- Fix (CSS only): the explorer takes `flex: 1 1 0` (only the space left over)
  with a 72 px minimum matching the splitters' own minimum.
- New Chromium test `e2e/workbench-splitters.e2e.ts` (full app, intercepted
  API, 40 installations): drag down/up and keyboard change the rendered height
  by exactly the dragged distance, and both blocks at maximum leave the
  explorer visible. RED on the previous CSS (both directions), and with a
  0 px explorer minimum (third case).

## 2026-10-05 — AR06 rows reconciled: seven TODOs find their place

- Status only, no product code. Named evidence rerun on `2f6f20b0` with the
  OriginalData corpus linked: 258 passed / 0 failed / 0 ignored in 11 blocks
  across knx-etsproj, knx-productdb, knx-cli, knx-app and knx-server.
- `KL-128` → `DONE` (legacy filename refusal fixed in 95e6bcb0);
  `KL-11`, `KL-125` → `BLOCKED_EXTERNAL` (missing scheme 15–19/22 samples,
  missing independent schema-23 project); `IMPORT-06`, `KL-15`, `PDB-08`,
  `PDB-10` → `ACCEPTED_BOUNDARY` with their lifting conditions unchanged.
- KNOWN_LIMITATIONS §11 gains the scheme-23 pointer it was missing; §128
  notes that its misnaming is fixed.

## 2026-10-05 — AR21 finding 4: the flow note stops counting heads twice

- The reduced-rendering note in the flow view counted lines as telegrams
  (one per recipient) and also counted refused telegrams as bundled. It now
  counts each telegram once: drawn bundled, or not (completely) drawn
  (TELEGRAM_FLOW_VISUALIZATION §16). Tests with two receivers per telegram and
  the en/de wording fail on the previous code; six mutants killed.
- `FLOW-01` stays `IN_PROGRESS` until the next AR21 rerun.

## 2026-10-05 — AR09: a group address finally says what it carries (ADR 0078)

- `GroupAddressEntry::declared_dpt: Override<DptRef>` holds schema-21+
  `GroupAddress/@DatapointType` (absent/empty/value/malformed with exact
  text and a map report entry); nothing is left as an opaque row.
- `resolve_group_address_type` weighs it against the linked objects:
  `Declared`, `DeclaredDiffersFromLinked` (same width), `SizeConflict`
  (Schema23 §1.2.7 size rule broken → conflict, nothing decoded or written),
  `Unverifiable`, `DeclarationNotLifted`, `Inferred`. Width from the new
  `format_width_bits`, pinned against the DPT-AS width audit.
  `resolve_group_address_dpt` stays inference-only.
- Consumers on the effective type: project DPT map (bus monitor decode,
  bus write, flow, CLI), projection `dpts`, CSV export column (new
  size-difference warning), project diff and import-compare views.
- Store schema v10: `group_address.dpt_state/value/layer`,
  `project_info.unlifted_group_address_dpt_declarations`; `migrate_v9_to_v10`
  lifts keyed opaque rows (schema ≥21 only) and counts unkeyed ones.
- Tests: core resolver (10), width table, importer (2), store migration (3)
  and round trips (2), projection, CSV, diff, bus-monitor decode; 9 compiled
  mutants killed. Integrated public16 on b7d7927e (code 5a2249d3 merged with 595d8d2e) independently accepted: 16 exit0, Rust 3269/0/177 in 182 blocks, Web 2013, Chromium 132 plus probe 1, 893 frozen inputs, CLI knx 0.1.0-alpha.4+gb7d7927e; corpus --include-ignored 1267/0/0 (7 crates); 9/9 mutants killed. KL-61 DONE.
  Web display of the declared/linked detail and the binding
  doc comment are handed to the UI owner (web lock).

## 2026-10-05 — AR21 corrections: the flow map learns to sit still

- Reheat is local and per node (§9.3): a new device, a new pair, a leader
  change, a changed activity class or a taller value block heats only the
  nodes involved and their direct neighbours; settled nodes keep their exact
  position and are not rewritten. Cooling follows the clock, so slow frames
  do not prolong the busy phase.
- Hub readability (§9.3): neighbours are moved out of a node's circle, name
  and value lines (`flowLayout.nodeFootprint`); circles and names stay inside
  the drawing area. New Chromium test `e2e/telegram-flow-hub.e2e.ts` measures
  a settled busy hub; it and 12 new or changed unit tests fail against the
  previous sources.
- §7 starting load (500 devices, ~2,500 lines, ~1,000 telegrams/s), 60 s,
  motion on: long-task time 61.9 s → 22.2 s, frames at the 30 fps cap, but
  the main thread stays above 0.9 (paint-bound). Recorded as the motion-on
  envelope in KNOWN_LIMITATIONS §154 and the guide; Motion Off is the
  recommendation for such buses. Details: TELEGRAM_FLOW_VISUALIZATION §14.
- `FLOW-01` stays `IN_PROGRESS` until the AR21 rerun.

## 2026-10-05 — AR09: parameter-only time periods say so before a write

- Numeric range pass over main types 5–9, 12–14 and 29 against DPT-AS: format
  limits correct, no wire change. Subtype ranges remain unenforced (documented
  §61 boundary). 7.003/7.004/7.006 and 8.003/8.004/8.006 (raw counter in
  10 ms/100 ms/1 min, "not allowed for runtime communication") now return the
  `time-period-raw-counter-parameter-only` ruling. Audit:
  docs/spec-audits/2026-10-05-dpt-numeric-ranges.md. Integrated public16 on 6fbb02c3 independently accepted: 16 exit0, Rust 3248/0/177 in 181 blocks, Web 2001, Chromium 131 plus probe 1, 887 frozen inputs, CLI knx 0.1.0-alpha.4+g6fbb02c3. First attempt refused (workspace 101): `http_bus_monitor` compared whole rows across two polls although `observedAgeMs` is measured per response (AR20); fixed in 6fbb02c3 (test only, 5 ms injected pause: old assertion fails, new passes).
  KL-61 stays IN_PROGRESS
  for the GA-declared DPT versus linked-object question.

## 2026-10-05 — AR09: 17.001 scene numbers travel in their own octet

- Format-width audit of DPT main types 1–30 against DPT-AS (new
  `knx-core/tests/dpt_spec_width_audit.rs`, widths read from the spec).
  One defect: 17.001 DPT_SceneNumber ("1 octet: r2U6") was encoded in the
  6-bit optimised A_GroupValue_Write form and reported a 6-bit width. It is now
  a 1-octet payload; the inline form and set reserved bits are refused.
  Pinned old tests rewritten; two compiled mutants (inline encode, reserved-bit
  check removed) caught. Audit: docs/spec-audits/2026-10-05-dpt-format-widths.md.
  Integrated public16 on f3b4fd34 independently accepted: 16 exit0, Rust 3247/0/177 in 181 blocks, Web 2001, Chromium 131 plus probe 1, 887 frozen inputs, CLI knx 0.1.0-alpha.4+gf3b4fd34.
  KL-61 is IN_PROGRESS: ranges/special values and GA-declared DPTs remain.

## 2026-10-05 — AR06Y: CLI refuses oversized product packages before reading them

- `knx products ingest` checked the 256 MiB package bound only after reading the
  whole file and opening/creating the product DB. It now refuses by file length
  first (typed `product ZIP size limit exceeded: <path>`), reads at most bound+1,
  and opens the DB only for admitted input. Productive limits unchanged; new
  public `MAX_PACKAGE_INPUT_BYTES` mirrors the existing private bound.
- Four real-binary sparse-file tests (+1 byte, 64 GiB, seeded DB byte-identical,
  exact limit reaches ZIP validation); RED shown, two compiled mutants killed.
  Integrated public16 on 0533230b independently accepted: 16 exit0, Rust 3244/0/177 in 180 blocks, Web 2001, Chromium 131 plus probe 1, 886 frozen inputs, CLI knx 0.1.0-alpha.4+g0533230b. Three earlier attempts refused on Web tests (two group-address-drag race failures, one vitest worker crash), retained.
  Details: PRODUCT_ZIP_RAW_INPUT_CONTRACTS.md. KL-151 remains IN_PROGRESS.

## 2026-10-05 — AR06X: actual raw-input guard contracts (local native evidence)

- Actual256MiB and +1-byte nonzero malformed buffers: typed inclusive/error-
  fidelity and all-application-table values/BLOBs plus seed archive preserved.
  Native raw2/full8 and two compiled exact semantic controls independently
  checked on hashed working-tree inputs; productive parser/constants unchanged.
- First control wrapper1 was a failure-inventory verifier false refusal; actual
  metadata0/compile0/named native101 retained and accepted without replay.
  Upper control ran separately. GNUtime127 and interrupted outer scoped-gate
  timeout remain infrastructure observations, not product regression or passes.
- Scoped retry hit a real header-width violation (retained, fixed). Integrated public16 on merge f867741e (leaf adf7ff29 + owner 036b46a6) independently accepted: 16 exit0, Rust 3240/0/177 in 179 blocks, Web 2001, Chromium 131 plus separate probe 1, 885 frozen inputs, CLI knx 0.1.0-alpha.4+gf867741e.
  This is not valid-large import/caller/streaming/resource-policy or Alpha
  acceptance. Authoritative KL151 remains in status/LEDGER.md; evidence in
  PRODUCT_ZIP_RAW_INPUT_CONTRACTS.md.

## 2026-10-05 — Commissioning callers published; Web halves handed over

- Commissioning CLI (`0.1.0-alpha.4`, history format 2) and server callers record
  durable activity history: download, restore, service-control read/write,
  compare and serial lookup. History/input aliases (identical path, parent link,
  unresolved leaf link) are refused before any store is created.
- Integrated gates on `ae567d00`: workspace 3238/0/177, Vitest 2001, Chromium 131,
  selected offline private 68/0, stamped release build
  ([receipt](evidence/commission-integrated-gates-ae567d00-2026-10-05.json)).
- User decisions: the Web partial-scope selector and history adoption go to the
  UI owner ([handoff](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/COMMISSIONING_ALPHA_LEDGER.md#handoff-to-the-ui-owner-2026-10-05));
  no reset route/UI (accepted boundary, KNOWN_LIMITATIONS §140).
- The serial-lookup entry below described a local candidate; it is part of this
  published state. Metadata is not recovery; no power-loss guarantee.

## 2026-10-05 — CLI serial-lookup read lifecycle (local candidate, not integrated/published)

- Both `device find-serial` directions accept optional `--activity-history`.
  Explicit unavailable history refuses before adapter acquisition; startup metadata
  precedes the first loopback tunnel request. Format-2 `serialLookup` keeps its
  required null address and omits serial, gateway, path and payload.
- Four compiled/listed/named runtime RED phases preceded implementation and correction.
  Five-stage branch gate accepted fmt, strict CLI all-target Clippy, selected
  regressions **119/0/8 across three result blocks**, actual alpha.3 version smoke,
  and whitespace. New CLI read tests **7/0/0** cover both directions, interruption,
  privacy, evidence-preserving refusal, duplicate flags, terminal connection failure
  and usage. Existing service-control reads now also refuse explicit unavailable
  history before adapter acquisition rather than silently continuing unjournaled.
  [Source-bound branch receipt](evidence/cli-read-lifecycle-2026-10-05.json).
- Three compiled/listed/intended-runtime source controls caught no-match, omitted
  pending-start and service-admission regressions; canonical bytes restored and
  reader **7/0/0** recompiled afterward.
- Offline Rust workspace **3225/0/177 across178 blocks** passed after an actual
  frontend dependency/build prerequisite; frozen Source/Web hashes unchanged. This
  cached-target branch gate is not latest-main integration, private-corpus, browser
  or independent acceptance. No-match completion is finished metadata while
  preserving the existing no-answer output and nonzero CLI exit.
  No Web/protocol code or hardware changed. Integration with the moving upstream,
  publication, further readers/long sessions and broader recovery remain pending;
  ledger status/ownership is unchanged. See the [caller contract](contracts/COMMISSIONING_ACTIVITY_HISTORY.md).

## 2026-10-05 — Service-control recovery-record admission (local candidate)

- [Property-only record contract](contracts/SERVICE_CONTROL_BACKUP.md): exact format-2
  semantic admission, explicit unsupported/unknown-field errors, all original
  bits and lowercase spelling retained; invalid records refused before filesystem
  effects. App advances to alpha.3, recovery format stays 2.
- Fresh five-stage local gate passed backup7/0/0, App98/0/21, fmt,
  strict Clippy and whitespace. Thirteen source controls each compiled, registered
  one test, failed at its named runtime assertion and restored canonical bytes;
  all four batches reran backup7/0/0. [Snapshot receipt](evidence/service-control-backup-validation-2026-10-05.json).
- Integrated workspace/publication, wider abort/restore/long sessions and client
  surfaces remain open. No independent acceptance, hardware/ETS parity or
  guaranteed power-loss recovery is implied. Ledger statuses are unchanged.

## 2026-10-04 — Shared commissioning Caller candidate (integration pending)

- ADR-0075 moves the unchanged format-2 activity engine to `knx-app`, with a
  server adapter and tracked CLI download/restore/service-control writes.
  Confirmed writes require an explicitly supplied persistent history. Original
  backups precede durable possible-send intent; intent failure sends no mutation.
  Written, restart and cleanup outcomes remain separate. History is not recovery.
- App/CLI program versions advance to0.1.0-alpha.2. The current typed consent,
  unsupported masks/device-recovery refusals and never-contact list are unchanged.
- The candidate78aae8ae passed its local frozen fmt, strict Clippy and three
  package gate853/0/85. After current-main integration, the public seeded-history
  regression covers18 exact/symlink/Unix-hardlink variants across primary,
  product and operator-key inputs for both callers; admission9/0/0. Two
  additional compiled parent-wiring controls failed at their intended runtime
  assertions; canonical bytes were restored and the target passed again.
- Actual merged-workspace acceptance/publication remains pending. The two
  earlier local result scopes are not a new workspace total. Further read
  callers, long sessions, client adoption and offline original-property recovery
  remain open. No hardware/ETS/vendor/power-loss validation is claimed or made
  an operator completion blocker. Status-of-record: [ledger](status/LEDGER.md).

## 2026-10-04 — AR06W: bounded declared-byte admission contracts

- Four public synthetic native contracts pass4/0/0, old count2 filtered: exact/
  one-over64MiB member and256MiB advertised total, with nonempty seeded full-
  value rollback checks. Exact declarations reach a precisely named decoded-
  size mismatch; over declarations fail at the named typed resource boundary.
- Physical fixtures <8KiB, not successful real64/256MiB payloads or memory
  benchmarks. Production parser/constants unchanged. Baseline40/0/3 and121
  inputs/log/binary identities verified separately. Four fresh compiled lower/
  upper controls accepted with named semantic REDs; original snapshot compile
  refusal retained. Scoped package/docs9 passed: ProductDB 644/0/25 in 31 blocks;
  includes native4/count2, not additive. Fresh integrated public16 accepted
  on2e188226: Rust3200/0/177/175, Web2001, Chromium131 plus separate probe1;
  871 source hashes/logs and revision-stamped CLI checked. Final-docs6 passed;
  declaration-only slice published/readback425f3407. Raw input/caller/resource
  acceptance still open;
  scope in PRODUCT_ZIP_DECLARED_SIZE_CONTRACTS.md.

## 2026-10-05 — AR21 review: the flow view goes back for one more lap

- The alpha session reviewed the integrated telegram-flow receipt (U19, AR20,
  U20, U21 up to `fb40a99a`) and reran the gates itself: fmt, clippy
  `-D warnings` and workspace tests without `knx-desktop` (3,184 / 0 / 177),
  web build, tsc, `check:flow-study`, Vitest 2,001, Chromium 130 / 131.
  The productive path (one monitor poll loop, one snapshot per generation,
  no writes) and the §7 scenario tests hold.
- Not accepted yet. Two §9.3 binding requirements for U21 (local reheat,
  badge-height separation) are neither met nor recorded as deviations, and a
  probe at the §7 starting load (500 devices, ~1,000 edges, ~985 telegrams/s)
  keeps the main thread 98 % busy with motion on (13.6 % with motion off).
  `group-address-drag.e2e.ts` is flaky. Details:
  TELEGRAM_FLOW_VISUALIZATION §13. `FLOW-01` stays `IN_PROGRESS`.

## 2026-10-04 — U21 part C: measured, then made lighter

- Production load study (`e2e/flow-load.load.ts`, `playwright.load.config.ts`,
  `vite.study.config.ts`): dense burst of 200 telegrams/s over 230 nodes with
  motion on and off, and a 3-minute session at 10/s. It records main-thread
  share, frames, long tasks, marker lag and heap after GC. The data path is
  cheap (6 % with motion off); motion is the cost, and the profile showed it to
  be mostly native SVG painting. Drawing is now capped at ~30 fps: 0.35 → 0.21
  main thread in the session, 0.89 → 0.69 in the burst, no long task left; values
  appear 10–20 ms after their poll, up to ~150 ms in the burst. The heap
  plateaus. Figures and method: docs/design/2026-10-04-telegram-flow-u21/.
- A test written for a nudge optimisation instead exposed a real defect:
  distances did not follow activity after the first settle. Activity classes
  now reheat on a class change (RED first; 2 mutants). The sending ring is
  written only on change.
- The measurement itself had to be repaired twice. An init-script Motion
  attribute is lost on parse, so motion was not actually off and an e2e test
  passed without testing anything; the fixture now takes `motion` like the
  app's bootstrap, and the test checks the attribute and counts frames. Marker
  values were also replaced by same-batch traffic. Only the production run is
  published.

## 2026-10-04 — U21 parts A and B: the flow view moves, and stops when asked

- Reducer: 60 s sender window and leader (fan-out counts once; exact tie
  keeps the leader), edge activity, bounded ring of fresh events.
  `flowDynamics.ts` promotes the U19 layout (centred, seeded from the stable
  hex slots). `flowAnimator.ts` runs solver and pulses through an injected
  scheduler: it bundles more than 24 events per batch with their count,
  counts beyond 160 pulses instead of drawing them, requests frames only
  while needed and nudges only on real change. `flowMotion.ts` follows the
  Motion setting and the OS reduce preference, including changes mid-run.
  The view adds Freeze (geometry only), the leader label, a reduced-rendering
  note, the sending ring, fading (10 s, then 60 s down to 0.35) and a 1 Hz
  refresh that is skipped while hidden. Rules: TELEGRAM_FLOW_VISUALIZATION
  §12. Load figures follow in part C.
- Evidence: RED first for the reducer (7), motion (2), dynamics (9, ported
  from U19 plus seeding and growth), animator (11, fake scheduler) and
  fade/curve (3). View tests (4) were written after the code and covered by 6
  mutants. Chromium `e2e/telegram-flow-motion.e2e.ts` (5) counts frames and
  intervals in the page. Motion Off and OS reduce mid-flight leave 0 frames
  in a second of live traffic. Freeze holds positions while a new sender
  appears. Leaving the tab clears both animator timers. With motion off,
  markers stay and values expire. The U20 view fails 4/5. Mutants: reducer
  8/8, animator 12/12, view 6/6, browser wiring 5/5.

## 2026-10-04 — U20 part 2: the bus monitor gets a Flow view

- The bus monitor now has **Telegrams | Flow** tabs. The flow view
  (`TelegramFlowView.tsx`) is fed by the monitor's own poll loop through
  `flowFeed.ts` (one model per session, one snapshot fetch per generation,
  one expiry timer); it opens nothing, polls nothing and writes nothing.
  Senders, configured members (solid, "configured, not received") and
  unresolved group addresses (box, dashed) sit on a static hex layout. Up to
  three current values per node, with ◇ on inferred member values, 7 s from
  observation. An HTML Inspector lists values, connections and per-object
  flags from the row's own generation. Keyboard: roving tab stop in name
  order, Enter selects, Shift+arrows pans, +/− zoom, 0 resets. Theme
  variables only; nothing is announced per telegram. en/de. Guide: "The flow
  view" in 07-bus-and-interfaces; residue KNOWN_LIMITATIONS §154.
- Evidence: feed 6, layout 5, view 7 (written after the component, so RED was
  shown against a stub: 7/7 failed), panel integration 8 (incl. reattach
  without revived values, new session, loss notice) and model additions
  (edge evidence, re-addressed device). The Chromium e2e
  `e2e/telegram-flow.e2e.ts` (7 cases, real panel, intercepted synthetic
  traffic, `page.clock` for expiry, live theme switch) fails 6/6 against the
  previous panel. Mutants: reducer 23/23, view 11/11, panel 7/7 (two
  survivors exposed missing reattach/new-session tests), feed 1/1. One feed
  guard was removed as equivalent: a late reply can only reach its own model.
  Screenshots in docs/design/2026-10-04-telegram-flow-u20/ were inspected;
  they showed invisible lines in an unthemed fixture, arrowheads under text
  and an overflowing flag table, all fixed.

## 2026-10-04 — U20 part 1: the telegram-flow reducer and wire validation

- `apps/knx-web/src/flowWire.ts` validates the AR20 snapshot and the new row
  fields (widths, canonical generation, all six flags, known names) and
  refuses everything else. `flowModel.ts` is the pure, session-keyed reducer:
  sequence dedupe and ordering, a bounded queue per unknown generation,
  resolution of every row against its own generation only, exact/ambiguous/
  unresolved/raw sources, configured targets from active members, value slots
  with 7 s from observation time, at most three badges, and bounded growth
  with counters. `api.ts` gains the additive fields and `fetchFlowSnapshot`.
  Rules: TELEGRAM_FLOW_VISUALIZATION §11. Nothing renders it yet (part 2).
- Evidence: RED first for wire (24), reducer (26) and API (1). 23 guard
  mutants were run: 21 caught at once, and the two survivors exposed test gaps,
  which are now closed. The in-session review found stale node evidence across
  generations (`Object.assign` kept old candidates); this was fixed RED-first.
  The companion's import-graph guard lists `flowWire.ts` with its isolation proof.

## 2026-10-04 — KL-60: the diff view's long tables filter and scroll instead of paging

- `ProjectDiffDetails.tsx`: a table with more than 20 entries gets a
  search field (key and name), status toggle buttons, a live match count
  and a bounded scroll viewport rendering only rows near the visible area.
  Up to 20 entries a table stays a plain list. "Show more" and
  `DIFF_PAGE_SIZE` are gone. Window math lives in `virtualWindow.ts`
  (pure, clamped, overscan). The filter `filterEntries` is in
  `projectDiffView.ts`. Row heights are measured (re-measured via
  `ResizeObserver`); rows above the view that grow shift `scrollTop`; a
  list at its end stays there; End/Home jump instantly. Opened nested
  tables survive their row scrolling out of the window (per-mount memory
  keyed by table id). Escape in a typed filter clears it instead of
  closing the report, a defect the in-session review found before the gate.
- Evidence: RED first for the window (7 cases incl. a reachability
  property), filter (3), panel (6) and table memory (2). Chromium
  `e2e/diff-virtual.e2e.ts` on a synthetic 3,300-entry table with variable
  row heights: under 120 DOM rows, every position reached by scrolling,
  End/Home, anchor stability while scrolling up, filters. It fails 4/4
  against the previous list. 14 mutants are caught (5 only in Chromium);
  one memory mutant exposed an ineffective `useMemo([report])`, now
  replaced by per-mount state with the remount contract tested. Backend
  diff API unchanged.

## 2026-10-04 — AR08 Web half: the project-password dialog

- Importing a ZipCrypto-protected ETS4/ETS5 project in the Web UI no longer
  ends in an error. On `422` `projectPasswordRequired` the app opens a
  dialog naming the file. The field is masked and `autocomplete=off`, with a
  note that the password is not stored. The app then retries **the same
  import** with the password in that one request; on `projectPasswordWrong`
  it asks again. Cancel ends quietly: no project, no toast, no failure
  banner. Opening a `.knxdb` never asks. `projectPassword.ts` classifies the
  refusal; the password lives only in the dialog field and the retry closure.
- Evidence: 13 Vitest cases (api body, classification, dialog en/de, three
  App flows), written RED first. `e2e/project-password.e2e.ts` runs the real
  app with an intercepted API (required → wrong → right, and cancel) and
  checks every browser request: the password appears only in import bodies,
  and in neither `localStorage` nor `sessionStorage`. The e2e fails against
  the previous app. 7 guard mutants are caught; one survived at first because
  the unit test looked for a toast class that does not exist, and now checks
  the visible text instead.

## 2026-10-04 — AR20: the telegram-flow backend contract

- Monitor rows gain `sourceRaw`, `destinationRaw`, server-monotonic
  `observedAgeMs` and the `flowGeneration` they were decoded with; the poll
  carries the current `flowGeneration`. Counters stay JavaScript-safe
  (refusal and saturation at 2^53 − 1).
- New read-only `GET /api/bus/monitor/flow-snapshot`: configured devices,
  group members (Send/Receive, activation, six nullable flags), diagnostics
  and truncation counts, bound to session and generation.
- The session context comparison now covers devices, links, flags and
  activation, so such edits show as `stale`.
- Contract: [TELEGRAM_FLOW_VISUALIZATION §10](TELEGRAM_FLOW_VISUALIZATION.md#10-ar20-delivered-contract-alpha-2026-10-04).
  Next: U20/U21 (UI), AR21.

## 2026-10-04 — U19: telegram-flow study measured, AR20 handoff written

- A visibly synthetic native-SVG study (`apps/knx-web/e2e/flow-study/`) covers
  the flow semantics: per-slot values with a 7-second lifetime, reads without
  a value, a sequence high-water mark, at most 3 badges, a sender-only 60-second
  leader with tie rule, capacity refusal, pulse bundling, and a bounded,
  cooling layout. 23 Vitest cases, 4 Chromium checks (map, keyboard Inspector,
  freeze, motion off) and 11 guard mutants, all caught. One survivor was
  initially equivalent; a test for a read row that carries a value made it
  catchable.
- Measurements (Chromium 152, Ryzen 7 5800X, under load from other sessions):
  small and mid maps run at 60 fps. The target load (500 / 2,500 / 1,000 per
  s) runs at 16.7 ms per frame with resting geometry, including pulses and live
  values, but 67–100 ms while every edge moves. Canvas 2D was never better and
  five times slower at rest, so the decision is native SVG without a dependency;
  U21 must reheat locally and bundle pulses over their lifetime.
- The exact AR20 proposal covers raw addresses, server-monotonic
  `observedAgeMs`, a `flowGeneration` covering links, flags and activation,
  and a bounded `flow-snapshot` route. It is in
  [TELEGRAM_FLOW_VISUALIZATION §9](TELEGRAM_FLOW_VISUALIZATION.md#9-u19-resolution-goal-ui-owner-2026-10-04),
  with an ADR-0077 addendum. Nothing is wired to the monitor feed; U20 needs
  AR20 first.

## 2026-10-04 — AR08: password-protected projects reach the importer

- A ZipCrypto (ETS4/ETS5) protected `.knxproj` now imports through
  `knx_etsproj::import_knxproj_with`, `knx_app::import_ets_project_with_password`,
  `knx import --password-stdin` and `POST /api/project/import` (`password`
  field; `422` `projectPasswordRequired`/`projectPasswordWrong`). The
  existing reader is reused; no new cipher code.
- `ProjectPassword` redacts its `Debug`; redaction is asserted across
  report, errors, CLI output, saved files, session log, load progress and
  project tree. A decrypted import adds an `unsupported` report entry for
  the lost protection.
- Fix found on the way: a wrong password that passed the check byte surfaced
  as "corrupt deflate stream"; it is now `WrongPassword`.
- Open: Web password dialog (UI handoff), real ETS4/ETS5 sample, AES.
  Details: [ALPHA_READINESS](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_READINESS.md#ar08-password-import-entry-paths-2026-10-04).

## 2026-10-04 — AR06V: existing product-ZIP count contracts verified

- Existing4096-entry inclusive install/replay and4097-entry seeded all-table
  refusal contracts added as65d7cf5c. Native2 and two compiled cap controls
  accepted; no production cap or grammar change.
- Actual latest source6c3080d9 accepted all16 commands: Rust3157/0/177 across170
  blocks, Web1835, Chromium108 plus separate probe1;806 source/config/log
  hashes and current CLI verified. Same-root accepted cache/native120
  unchanged, no fresh-target claim or private/hardware test passes.
- [PRODUCT_ZIP_COUNT_CONTRACTS](contracts/PRODUCT_ZIP_COUNT_CONTRACTS.md) retains original
  failures, exact checkpoint identities and bounds. Source-identical owner
  documentation adopted; final docs6 passed and main5adccdb0/source806 readback
  verified. Ledger resource policy remains unresolved; no whole Alpha acceptance.

## 2026-10-04 — Telegram-flow nervous-system Alpha scope approved (planning only)

The user confirmed the focused interview and authorized the owning sessions to
implement [the flow contract](TELEGRAM_FLOW_VISUALIZATION.md). U19–U21 own design,
shared-monitor UI, values/layout/pulses and UI acceptance; AR20 owns the read-only
session-bound semantic contract; AR21 adopts integrated Alpha evidence. These
are open packages, not shipped code, a benchmark or a new hardware authorization.
Values update immediately and expire after 7 seconds, independently of pulse
arrival; configured target badges are not confirmed device state. Quiet observed
edges remain visible for the session; themes, freeze and motion policy apply.
[ADR-0077](adr/0077-session-local-telegram-flow-view.md) narrowly supersedes the
older no-animation-first recommendation without introducing physical coordinates
or a persistent history store. The new feature is required before final Alpha
readiness; prior U/UA/AR/K delivery receipts and accepted boundaries stay intact.


## 2026-10-04 — AR13 hand-over: the debug-report dialog names every kept telegram field

- The opt-in warning for `bus-telegrams.json` (`debugReport.privacyTelegrams`,
  en/de) now says what `report.md` says: the file is not redacted and keeps
  addresses, group-address names, every telegram's value (text values
  included) and its timestamp, which together can show when the installation
  was in use. Before, it named addresses and names only.
- Evidence: a content test written RED first checks each kept part in both
  catalogues (the existing test only compared the dialog with the catalogue
  string). Full Vitest 1,835 in 101 files. With this, every web item from the
  2026-10-04 UI owner handoff is delivered.

## 2026-10-04 — UX-01: drag a group address onto a communication object

- Group addresses in the Project Explorer are drag sources. They carry one
  decimal id under `application/x-knxbench-group-address-id`
  (`groupAddressDrag.ts`), with `effectAllowed: link`.
- The drop target is a communication object's link row in the device
  workspace, the row of the keyboard path. During dragover only the type is
  checked, because the browser protects the data then. On drop the id is
  parsed strictly and must be one this device may link (its installation's,
  MODEL-01); otherwise the row reports it locally and sends nothing. A valid
  drop calls the unchanged `POST /api/group-links` once, in the direction
  shown in the row. That is the answer to GAP_ANALYSIS_ETS B10's objection:
  there is no hidden default direction. A server refusal (for example an
  existing link) appears in the row.
- Evidence: 9 Vitest cases written RED first (payload module, Explorer source,
  drop target), plus one guard (foreign drags) that already held. Full Vitest
  1,833 in 101 files. `e2e/group-address-drag.e2e.ts` drives a real HTML5
  drag in Chromium from the Explorer to the link row (direction Receive, and
  a server refusal); both cases fail against the previous sources. 5 guard
  mutants are caught. The e2e needs a viewport that shows source and target
  together: Chromium drops a pending drag when the page scrolls while the
  button is held.

## 2026-10-04 — MODEL-02 web half: ambiguous placements are repaired by an explicit choice

- The device Inspector shows a **Placement conflict** when the topology lists
  a device more than once: twice in one line, on two lines, on a line and
  unassigned, or across installations. Every current slot gets **Keep this
  placement** (`POST /api/repair/device-placement` with `keepLineId` or
  `keepUnassignedInstallationId`). A line the projection shows under two
  areas does not turn its devices into conflicts.
- A line listed by several areas of one installation keeps the duplicate-id
  alert and adds **Keep under this area** per area
  (`POST /api/repair/line-owner`). The button appears only when every
  occurrence is the same line; two different lines sharing an id, or an id in
  two installations, stay without a repair, as before.
- Each repair is one undoable server step; a refusal stays on screen with
  the server's reason. `treeUtils.devicePlacementSlots` lists the distinct
  slots with their counts.
- Evidence: 9 Vitest cases written RED first, plus 3 guards that already
  held. Two more line-owner tests (different lines sharing an id; an id also
  in another installation) were added after their guard mutants survived the
  first mutation run. Full Vitest 1,817 in 100 files. `e2e/repair.e2e.ts`
  performs both repairs in Chromium against the real Inspector; both fail
  against the previous Inspector. 7 guard mutants caught.

## 2026-10-04 — MODEL-01 web half, part 2: every installation is edited in place

- The Inspector no longer gates on the first installation. Edit, delete,
  rename and move of areas, lines, building parts, group ranges, group
  addresses and devices are offered for every entity owned by exactly one
  installation. Every move or link list offers only that installation's
  targets: a device's lines and building parts, a line's areas, a part's or
  range's parents, and the group addresses a communication object can link
  to. A device placed nowhere may link to any group address, mirroring
  `Command::LinkComObject`. An id owned by no single installation stays
  read-only, with the reworded message "… only available for … that belong to
  exactly one installation."
- `findDeviceLine` / `findDeviceBuildingPart` (previously
  `…InFirstInstallation`) look inside the device's own installation.
- The bulk toolbar moves a device selection only when one installation places
  every selected device; otherwise it says so and still offers delete.
- With several installations the group-address CSV buttons carry an
  installation choice. Export, import preview and import confirmation name the
  same installation (the server binds it into the confirmation token). With
  one installation the requests are unchanged.
- Evidence: 14 new Vitest cases written RED first, plus one guard. Five older
  Inspector tests that asserted the first-installation-only rule now assert
  the owning-installation rule, and one `findDeviceBuildingPart` fixture
  gained a topology placement. Full Vitest 1,803 in 100 files. A second
  `e2e/installations.e2e.ts` case renames a line of installation 2 through
  the Inspector; it fails against the previous Inspector. 7 guard mutants
  caught. MODEL-01 is `DONE` in `docs/status/LEDGER.md`.

## 2026-10-04 — MODEL-01 web half, part 1: every installation can be built and named

- The Project Explorer and the structure workspace offer their create rows in
  **every** installation, not just the first. Root creates (area, main group
  range, root building part or site, range-less group address) send the
  `installationId` of the installation they are typed into. A child (line,
  middle range, nested part, address in a range) goes to its parent's
  installation and appears only where that parent lives (ADR-0070). "Add
  device" works on the lines of every installation. The unassigned row stays
  in the first installation only, because the catalog route carries no
  installation and the server puts an unassigned device there.
- Device drag and drop works inside each installation and never between two
  of them. The drop handler re-checks that source and target share one
  installation, because a drop event arrives even when the target refused
  the dragover. A device placed in two installations has no owner and cannot
  be dragged.
- The project node in the Inspector lists every installation with its own
  name field (`PATCH /api/installations/{id}`, one undo step each); a
  refused rename restores the name and shows the server's reason.
- `treeUtils` gains `owningInstallation` and `deviceInstallations` /
  `deviceInstallation`, which mirror the core's `owning_installation` and
  `device_installation`: absent or ambiguous never falls back to the first
  installation. Device ownership is computed in one linear pass per render.
- Evidence: 17 new or changed Vitest cases written RED first, plus one guard
  that already held before the change. Adapted to the new rule: the old
  "only first-installation drag sources" test, and the StructureWorkspace
  expectations that the second installation has no create rows. Full Vitest
  1,790/100 files before the review fixes. `e2e/installations.e2e.ts` uses a
  new fixture with the real Explorer and project Inspector to create a root
  area in installation 2 and rename it; it fails against the previous
  Explorer. 7 guard mutants are caught. One equivalent mutant (a redundant
  section check on the line row) led to removing that check. `site.e2e.ts`
  now expects the explicit `installationId` on its root create. Full gate
  (second attempt; the first failed only on that pinned body): Rust 3,145 / 0
  / 177, Vitest 1,790 in 100 files, Chromium 103/103. Part 2
  (Inspector edit gates and dropdowns, bulk toolbar, CSV installation choice)
  follows under the same Web lock.

## 2026-10-04 — AR14D D5: resolved limitation bodies to history, goal-ui status to its dossier

- Five `KNOWN_LIMITATIONS` entries whose own status line says resolved (§18,
  §23, §24, §42, §130 gate) keep heading, number and status line as a stub;
  their historical descriptions (163 lines) moved verbatim to
  [history/KNOWN_LIMITATIONS_resolved.md](history/KNOWN_LIMITATIONS_resolved.md).
  §90 and §95 stay in place: they are pointers, not resolved defects. The
  heading count (115) and every fragment link are unchanged; a script proved
  no original line is missing.
- `goal-ui.md` (owner agreed 18:30): its *Where things stand* narrative moved
  verbatim to [UI_ALPHA_READINESS](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/UI_ALPHA_READINESS.md#owner-status-history)
  and the section now links to the ledger; the `goal.md` ownership row points
  to `alpha-release-goal.md` and the archived file. The five IDs it named all
  have ledger rows.
- Open: `goal-commission.md`, which waits for its owner's agreement.
- The status-docs lock is released with this delivery. Documentation only.

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

- Retired and dated documents moved verbatim to [docs/archive](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/README.md):
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
  [alpha-release-goal.md](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/alpha-release-goal.md#ar14d--consolidate-status-tracking-before-ar15)
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
  pending. See [History contract](contracts/COMMISSIONING_ACTIVITY_HISTORY.md#web-history-candidate--2026-10-04).
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
- Follow-up 2026-10-05: the browser check runs in Chromium and Firefox (46/46
  each); WebKit is blocked on this host by missing Ubuntu libraries. Privacy
  review locations now name records by id (`events[alpha-backlog].aside`).
- Follow-up 2026-10-08: editions `2026-10-08.1` (ETS wording: "comparable
  functionality", not "mirrors") and `2026-10-08.2` (no remarks about AI usage
  limits). Only `2026-10-08.2` is kept in the tree.
- Follow-up 2026-10-08: edition `2026-10-08.3` moves the cutoff to `origin/main`
  `138403ed` (2,295 commits), remaps every commit reference through the
  7 October commit map, and adds 13 steps (4–8 October) plus a ninth chapter.
  The browser check now reads the chapter count and the bus steps from the
  edition; date labels yield to step labels that reach into the date column.
- Follow-up 2026-10-08: after the partial history rewrite (KNOWN_LIMITATIONS
  §162), edition `2026-10-08.4` replaces `.3` with identical content except the
  remapped cutoff hash and two evidence hashes; only `.4` is kept.
- 2026-10-08: the owner approved `2026-10-08.4` for publication
  (`story/approvals/2026-10-08.4.json`, `release-check` eligible; new
  `test_approvals.py` fails on a stale record). `publish` still refuses: the
  public page variant and hosting are not designed.
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
and [history contract](contracts/COMMISSIONING_ACTIVITY_HISTORY.md) preserve provenance.
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

The [42-ID ledger](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/COMMISSIONING_ALPHA_LEDGER.md) matches the readiness inventory
without omissions, duplicates or extras; every row retains its fallback and
unblock condition. [API contract](contracts/COMMISSIONING_ACTIVITY_HISTORY.md), ADR-0064
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
AR07 and Alpha remain incomplete. See [the bounded evidence](contracts/PARAMETER_SEMANTICS_BOUNDARY.md).

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
See [parameter boundary](contracts/PARAMETER_SEMANTICS_BOUNDARY.md). AR07 broader audit open.

## 2026-10-02 17:14 CEST — AR07 offline source discovery; baseline pending

- AR06 final delivery receipt `0c3d6a8a` published/read back; task-owned AR06
  worktree/ancestor-confirmed branch and 333 scratch entries cleaned. No shared
  root/corpus changes. New isolated AR07 checkout starts from that receipt.
- [Parameter boundary](contracts/PARAMETER_SEMANTICS_BOUNDARY.md) records fresh local
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
  catalogue/producer tests, all passed. [Contract](contracts/IMPORT_BOUNDARY_CONTRACT.md)
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
  remain open. [Contract](contracts/IMPORT_BOUNDARY_CONTRACT.md).

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
  IN_PROGRESS. [Contract](contracts/IMPORT_BOUNDARY_CONTRACT.md).

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
  [IMPORT_BOUNDARY_CONTRACT](contracts/IMPORT_BOUNDARY_CONTRACT.md).

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
  Contract: [IMPORT_BOUNDARY_CONTRACT](contracts/IMPORT_BOUNDARY_CONTRACT.md).

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
  Contract: [IMPORT_BOUNDARY_CONTRACT](contracts/IMPORT_BOUNDARY_CONTRACT.md).

## 2026-10-02 — AR06 root import-boundary checkpoint (offline; 08:51 CEST)

- Read the user-provided KNX Standard v3.0.0 PDFs as local primary evidence;
  schema-23 root/namespace/source-string findings are scoped in
  [IMPORT_BOUNDARY_CONTRACT](contracts/IMPORT_BOUNDARY_CONTRACT.md), not copied as licensed
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
  not reimplemented. Per-ID evidence: [UI_ALPHA_READINESS](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/UI_ALPHA_READINESS.md).
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
  Contract: [MANUFACTURER_REPORT_CONTRACT](contracts/MANUFACTURER_REPORT_CONTRACT.md).

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
  [MANUFACTURER_REPORT_CONTRACT](contracts/MANUFACTURER_REPORT_CONTRACT.md).

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
  [STORAGE_COMMAND_CONTRACT](contracts/STORAGE_COMMAND_CONTRACT.md), receipt
  `.ai/logs/2026-10-01_codex_alpha-storage-contract.md`. Triage is 110 headings,
  seven historical/resolved, 103 residual, 102 classified (5/30/54/13).
- No UI/editor, live KNX, parked ADR-0039 phase, import format or multi-user scope
  change. Canonical-root statistics remains foreign-owner blocked.

## 2026-10-01 — AR03 enforcement audit; AR02 publication receipt (UTC)

- Published AR02 `e691bc1318d0785289f8132378a0f26c9a829b27`: exact remote ref
  and artifact/tree matched the staged/gated candidate. Required author and
  committer verified, no co-author trailer. Owned checkout/branch, build
  targets and scratch removed; foreign root/reports left untouched.
- Docs-only audit [ADR0039_ENFORCEMENT_AUDIT](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ADR0039_ENFORCEMENT_AUDIT.md)
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

- The license stays `AGPL-3.0-or-later`. Then-new `CLA.md`
  ([historical decision](adr/0053-contributions-come-with-a-license-grant-for-dual-licensing.md)): each
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
