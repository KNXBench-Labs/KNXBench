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

## Recent deliveries

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
