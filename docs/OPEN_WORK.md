# Open work after the 0.1 alpha

## Checked parameter editors in the device editor (12 October 2026)

Time/Number/Colour/Picture rows offer a **Checked editor**
([host](contracts/parameter-editor-host.md#workbench-host-12-october-2026)): exact
source evidence verified per field, writes only through the existing route, drafts
kept across Save. Open: module-scope source mapping, verified Colour/Picture value
codecs (PDB-04 stays LATER), revision-conditional parameter writes, native
WebKitGTK/Orca checks.

## Installation instructions in the device editor (12 October 2026)

The instruction editor is the device editor's sixth tab
([host](contracts/instruction-editor-host.md#workbench-host-12-october-2026)):
real tree identity (read-only without one), refresh after every Apply, drafts kept
across tabs and Save, guarded on Open/Import/New/Quit. Open: natively authored
devices (no imported source), other source schemas/encodings, an ETS export of
edited overrides, native WebKitGTK/Orca checks.

## Project notes in the workbench (12 October 2026)

The notes editor is mounted as the **Notes** workbench view
([host](contracts/project-note-editor-host.md#workbench-host-12-october-2026)):
real tree identity, stable-ID target catalogue, refresh after Apply, drafts kept
across views and guarded on Open/Import/New/Quit. Open: communication objects as
offered targets (the tree does not project all of them), a reload/tab-close guard,
native WebKitGTK/Orca checks, localized action names in the review, and
FUTURE-03 as a whole (still LATER).

## AP7 catalogue updates (12 October 2026)

`knx products update` previews a manufacturer `.knxprod` against the installed
catalogue, installs it only with the preview token after a backup, and `knx
products restore` reverts ([ADR-0117](adr/0117-previewed-recoverable-catalogue-updates.md)).
Open: a web UI for the same flow (Session 1 surface), any automatic update
source (none is documented for third-party tools), and moving a device to a
newer application version.

## AP6 opt-in MCP rename writer (12 October 2026)

[ADR-0116](adr/0116-opt-in-mcp-name-recipe-writer.md): `knx-mcp
--allow-name-recipe <alias>` adds preview/apply tools for literal
group-address renames with one user-approved form elicitation per apply,
re-check after the answer and native working-history persistence. Real-binary
stdio tests cover opt-in, annotations, approve/decline/cancel/disapprove,
missing capability, stale token, concurrent change during confirmation,
invalid input, persistence failure and reopen. Open: proof that a human (not
an auto-accepting client) answered, any further MCP mutation, and an
acceptance run with a real third-party MCP client.

## AP6 offline service dependency prerequisite (12 October 2026)

[ADR-0115](adr/0115-offline-application-edit-boundary.md) locally separates the
existing bounded recipe from App's importer/key graph with a compatible App facade
and strict production dependency/root gates. Focused library/actual CLI/policy and
three reverting guard proofs pass; fresh20-stage broad acceptance passes on2034
frozen sources. Fresh-upstream merge `d6127583` passes22 frozen stages (Rust4129/0,
Web2841/0, both server-default suites5/5); this closure publishes it normally.
The module split itself grants no MCP write; the opt-in writer above builds on it.
AP7 catalogue work, missing AP3 contracts and Session1 host responsibilities remain
independent, not silently reassigned.

## Offline literal naming slice (12 October 2026)

The [bounded application/CLI recipe](contracts/offline-name-recipe.md) is implemented
and has original assembled acceptance plus strengthened native-refusal/incarnation
controls. [Current evidence](evidence/offline-name-recipe-2026-10-12.json) separates
those source snapshots; final assembled18 stages now pass, integration/publication
closure documentation8 passes; source package publication/readback on main
`f70183f0` is verified. It does
not deliver a macro VM/registry, generated selection, MCP/LLM writes or bus rights.
Broader FUTURE-02, AP6 permission/confirmation and AP7 catalogue work remain separate;
missing AP3 value/module contracts remain open (the project-note host followed later, see above).


## Bounded project-note editor leaf (12 October 2026)

The [isolated component](contracts/project-note-editor-host.md) and
[guidance](contracts/project-note-editor-guidance.md) have actual feature40,
full owner22-stage, built browser8 and source-guard-negative acceptance. Five
real confirmed browser mutations and native working/root recovery are separately
source-bound; an empty Save-success parser error remains honestly partial and its
missing saved-root restart was completed read-only, with no replay.
The production mount, host catalogue and navigation refresh were delivered
later the same day (section above); FUTURE-03 remains LATER, not whole AP4. AP3/AP5/AP6/AP7 data-tooling
remainders and hardware/ETS/report compatibility are not closed by this leaf.


**Audit:** 2026-10-08, against `origin/main` `608a204b`. This page is the one
place that answers "what is not done?". It replaces the three finished alpha
goals (removed 2026-10-08, see [REMOVED_DOCS](history/REMOVED_DOCS.md)). It does not change any
status by itself: per-ID status stays in the [ledger](status/LEDGER.md),
behaviour boundaries in [KNOWN_LIMITATIONS](KNOWN_LIMITATIONS.md).

**Freshness note:** [all twelve original ideas and the roadmap](status/2026-10-08-ideas-roadmap-audit.md)
were checked against actual source/callers. Read-only MCP, Flow, L3 legacy web
upload, commissioning history UI, website/story publication and the original
humour templates are delivered; their broader boundaries are not new tasks.
The [known-issues/status follow-up](status/2026-10-08-known-issues-status-audit.md)
also reconciles current source claims, historical limitations and evidence type;
it changes documentation, not owner rows or device behavior.

Short version: **the original alpha goal files are finished at their recorded
scope, not every later feature.** The current ledger includes the owner-approved
source import expansion and its missing authorized-export evidence. Dated audit
counts below remain historical. What remains is
(1) work that is running or waiting right now, (2) parked unpublished packages
and publication follow-up, (3) deferred items with a recorded user decision, and
(4) later scope that never was an alpha task.

## AP3 missing colour/picture contracts (12 October 2026)

[Fresh primary-source/access and module-scope research](research/product-database.md#2026-10-12--ap3-colourpicture-contract-and-module-scope-follow-up)
received public referring/release/module articles, but the linked exact type
definition returns login HTML. Need authorized Color/Picture lexical/native
value/resource contracts and independently audited module-instance mapping.
Explicit inert literal fallbacks stay in place; neither a fallback nor this
research closes PDB-04. Independent AP5/AP6/AP7 continue without gate bypass.

## Bounded AP3 parameter control slice (11 October 2026)

Exact retained source evidence, optional authenticated read-only source HTTP and
isolated EN/DE decimal Time/integer Slider controls have current source-bound
acceptance. [Host contract](contracts/parameter-editor-host.md) and
[receipt](evidence/parameter-editors-2026-10-11.json). Existing validation,
dependencies, native persistence and write authority remain unchanged.
Colour/Picture have an explicit unverified-codec original-value fallback; module
mapping and Session1 shell mount remain open. PDB-04 remains LATER, not DONE.
No physical device, universal codec, release or deployment claim.

## Alpha.8 publication boundary

Alpha.8 packages the published selective-import, import-integrity, read-only
instance-value and archive-budget source changes. It does not complete the
remaining KL-175/146 follow-ups, externally blocked revision-export proof or
unmerged Functions/Sites work. Native/model schema12 requires independent
backups; a release is not a new ETS/hardware claim.
[Release notes](releases/alpha8.md). Dated source-package no-release statements
below remain historical, not a claim that Alpha.8 was never published.

## Public-source research follow-up (10 October 2026)

Package 9's bounded research pass for KL-170, KL-171 and KL-172 is complete in
[the source review](research/schema23-offline-boundaries.md). Implementation
rows stay LATER: per-type endian placement, KNX UTF-8 capacity/termination and
the loaded-image binary format still lack an admitted contract. Existing
refusals/source retention remain unchanged; no hardware follow-up authorized.

## How this was checked

- Historical goal checkboxes, counted before removal: `alpha-release-goal.md` 93
  checked / 7 open, `goal-ui.md` 37 / 0, `goal-commission.md` (no checklist,
  §4 completion condition). The 7 open boxes are all user-deferred or
  optional (section 3).
- Ledger counts (its own *Counts* line, `check-ledger` green, after the
  2026-10-09 owner cleanup): snapshot 180 rows — DONE 41, ACCEPTED_BOUNDARY 119,
  LATER 20; post-snapshot 11 rows — DONE 9, ACCEPTED_BOUNDARY 2.
- Handover `.ai/CURRENT_STATE.md` (newest entries per track), worktree list,
  and a path-by-path check of which root files exist on `origin/main`.
- Current ledger mechanically recounted after import expansion: 191 distinct IDs,
  52 delivered, 121 accepted boundaries, 17 later-scope rows and 1 externally
  blocked row (`IMPORT-02`); no active/running source work. The earlier census
  above remains historical. Every later-scope row is owned by `later`; the
  external restore-export prerequisite belongs to the maintainer/ETS owner.
  These are unequal source IDs, not a product-completion percentage.

## Import expansion source integration (10 October 2026)

Selected-device/line merge into the open project, the authorized revision-export
harness and read-only cvexc declaration analysis have completed local acceptance.
The owner subsequently authorized commit/merge/push; [integration acceptance](status/2026-10-10-import-expansion-integration.md)
is separate from the [historical local record](status/2026-10-10-import-expansion-verification.md).
Remaining: real authorized ETS revision exports with independently reviewed
baselines, future-model closure before model changes, and primary cvexc semantics
before enabling any runtime rule. Synthetic browser/CLI checks are not real
ETS/hardware/native accessibility evidence. No release/deployment or bus action.

## Private schema-23 follow-ups (10 October 2026)

KL-106 output audit and KL-8 secure-capable/not-activated witness are published.
KL-68/PDB-01 own stored instance values are implemented read-only (`d050c7e78b02`,
[ADR-0108](adr/0108-read-only-module-instance-values.md)); no Dynamic consumption,
sibling/default fallback or additional write authority. Repeat mapping and
activation remain an accepted, research-bound limitation. The explicit private
own-value comparison and RefId gate now pass after import-integrity integration
(`734064823e19`); the earlier parser refusal remains historical evidence.
KL-1 owner decision: retained-uninterpreted, byte-exact and explicitly reported
data may count as reconciled, without lifting its semantic limits.

Final KL-68/PDB-01 candidate `f4af614eac30` now passes the full143/143 corpus
across31 targets, eight-crate Rust and Web gates, explicit own-value/RefId gates
and independent four-case browser replay. Four inherited corpus contracts were
corrected test-only (`e74be53a`) after exact fresh-main RED baselines and owner
permission. Published no-ff merge `2e264f6cc6ec`; local/tracking/live equality verified.

Import-integrity is integrated, not a current waiting dependency. Next rest scopes:
KL-1/F01–F04 original project/product source reconciliation now has positive
private proof and the preservation-only owner decision; package4 final gates
are green on `382ab577`, with upstream-only test followup `dd4b8165` also green.
No-ff `992032c32327` published with local/tracking/live equality. KL-173/F06 out-of-tree
save/reopen evidence `01e6be99` now has green final source `4c38af5b` and
full corpus/quality/repo admission; no-ff `4c38af5b` published and read back
at `4c484227` with local/tracking/live equality. KL-174/F08 bounded content/role candidate `41b3d6267731` passes synthetic
app/native/product/HTTP focuses and final source `566afbdd` (Rust2400/0/181,
strict quality/repo/docs, full nonempty corpus/private reconciliation/RefIds);
delivered/read back `68147ba3` with local/tracking/live ref equality. KL-175/F09
bounded raw labels delivered/read back `6ef0b2d8`; interpretation stays separate.
KL-146 read-only diagnostic source is accepted on `f045390fdf7b`; differences
and unsupported scoped sets remain boundaries, not a deferred comparison.
Diagnostic delivered/read back on `70c05fdc58e5`; the scoped review is closed
([receipt](evidence/private-schema23-review-closure-2026-10-11.json)).
KL-170–172 public research package9 is
published; implementation limits remain research-bound. Do not reuse old-base
results for a changed main.

## Import-integrity merged-source acceptance (10 October 2026)

[Integration proof](status/2026-10-10-import-integrity-integration.md) covers
actual original-input production/native checks and independent source/persistence
comparison. At that integration's recorded snapshot, the ledger had 197 source IDs: 54 delivered, 120 accepted
boundaries, 22 later-scope rows and one external prerequisite; these are not a
product-completion percentage. The dated earlier inventories remain historical.
No release or hardware action follows from successful source integration.

## Private schema-23 sample review (10 October 2026)

A read-only review of a privately supplied ETS6 project lifted KNOWN_LIMITATIONS
§125 and recorded six new limitations (§170–§175) plus evidence updates for §1,
§6, §8, §11, §52, §68, §106, §134, §146 and PDB-9. Open follow-ups, none
authorized yet: little-endian and UTF-8 image encoding (needs a documented
source), repeated-module semantics with a `ValueMap` instance dimension (ADR
first), applying inactive-object overrides after reactivation (preservation
and reporting repaired by the import-integrity package),
content-based classification of project user files/add-in data and the
stored-image reference (research only). The redaction audit of retained network
endpoints is done (§106, 2026-10-10: MAC addresses redacted, every other output
withholds the data or names it). The secure-capable-but-not-activated case is
pinned by a synthetic witness (§8): treated exactly like a plain program, no
Secure semantics added. The import-integrity package now has separate
[merged-source acceptance](status/2026-10-10-import-integrity-integration.md):
archive identity, object trees, unassigned placement, metadata provenance and
source/diagnostic preservation are repaired. Retained extensions and offline
execution limits remain research/follow-up work, not wider runtime support.

## 1. Running or waiting right now

| Track | State | Next step | Where |
| --- | --- | --- | --- |
| Legacy ETS3 product databases (`.vd3`–`.vd5`) | L1 inspection, L2 import/secret withholding, L3 web upload/password handling, the VD5 installer layout with measured bounds and L4 download plans from `s19_block` are on `main` | A first live legacy download (needs the maintainer's device go; none has run). Map atomic types 3 (`string`) and 5 (`long enum`) once ETS's conversion is compared (the `.vd5` has 1,515 such parameters; candidate oracle `SIEMENS_KNX_PDB_Nov_2016_ETS4.knxprod`). Merged legacy procedures (`07B0h`) stay refused. | [ADR-0094](adr/0094-legacy-exim-product-files.md), [VD4 import](VD4_PRODUCT_DATABASE_IMPORT.md), KL §128 |
| Website / published Evolution Story | Edition `2026-10-08.4` is approved and deployed with the site; first publication is complete | HTTP→HTTPS (301) and remote IPv6 HTTPS (four probes, 200/TLS valid) verified on 2026-10-09; Pages Node-24 actions and demo links deployed/verified; owner's privacy review and optional WebKit evidence remain separate | [story/](../story/README.md), [WEBSITE](WEBSITE.md), ADR-0068/0095 |
| Web integration of finished leaves | Packages 1 (project notes) and 2 (installation instructions) delivered on 12 October; package 3 (time/integer-slider parameter editors) holds the web lock in worktree `web-params-20261012` (checked 2026-10-12) | Package 4: catalogue updates in the web. Colour/Picture keep the visible fallback; PDB-04 stays LATER | Web-lock lines in `.ai/CURRENT_STATE.md`, [parameter host](contracts/parameter-editor-host.md), [catalogue updates](contracts/catalogue-update.md) |
| Group-address views follow-up | Waiting for the reporter's feedback on issue #4 | Derived numeric 2-/3-level tree (GA7, ADR first) only if asked for; virtualization (GA8) only on request or a failing measurement | [goal record](goals/group-address-four-views.md) |

**Commissioning refusal buckets are an evidence backlog, not a running track.**
The recorded house/corpus snapshot and module-placement, `placed by Property`,
float, priority/value and non-memory-mask gaps remain in
[commissioning research §19.11–19.18](research/commissioning.md). The user's
hardware/vendor/ETS deferral in section 3 still applies; no new write or
experiment is authorized by this documentation audit.

## 2. Parked packages and publication follow-up

**Offline AP1 diagnostics (2026-10-09):** local implementation and fresh-main
integration acceptance are complete; normal main publication is verified.
[Delivery/readback](status/2026-10-09-offline-ap1-verification.md). Not a release.
[Contract](OFFLINE_PROCEDURE_RESOLUTION.md), ADR-0098: 69 partial source-bound
reconstructions / 6 source-limit unavailable from 75 selected real candidates;
no expanded real sequence, complete plan or new live support. Normal main publication is owner-authorized and tracked in the delivery receipt;
later semantic materialization is not implicitly authorized. Existing commissioning ledger dispositions are unchanged.

The root's two formerly parked packages are delivered. The **documentation
refresh and ideas/roadmap audit** were published as `696f25b6` (8 October); the
`docs-refresh-20261008` worktree no longer exists.

**Parked work (triaged 2026-10-12; owner decisions the same day).** The
[parked-work triage](status/2026-10-12-parked-work-triage.md) has the
file-by-file main check, the `merge-tree` conflict maps and the decisions. Nothing
below is on the remote or merged:

| Package | What it holds | Owner decision |
| --- | --- | --- |
| `native-evidence-20261011` (worktree) | 7 locally accepted commits: durable native import reports (ADR-0113), CLI/source-report/payload catalogues, export-privacy consent, read-only project metadata and recorded device state; 10 conflicts | Adopt as one web-lock integration package |
| `ux-feedback-20261010` (worktree) | 13 locally accepted commits, UX AP1–AP5; ADR-0112 number collides with main; 10 conflicts | Adopt AP1/2/3/5; **AP4 dropped** (main's Project status cards cover it) |
| `feature/functions-sites-20261009` (local branch only) | KNX Functions (`MODEL-07`) as one unreviewed `wip(functions)` commit `1154b72f` on schema 12 (main is at 14); acceptance attempt 4 failed | Kept as WIP; a later MODEL-07 goal restarts from it on the current schema |
| Initial native-write preflight | Never-compiled draft from `import-lifecycle-20261011` | Archived as a local patch; to be built fresh. Worktree and branch removed |
| `audit-findings-20261011`, `native-import-diagnostics-20261011` | Superseded / already on main | Dropped. Worktrees and branches removed |

Two further worktrees hold residue that still needs an owner decision:
`offline-naming-20261011` (an unaccepted device-naming design draft) and
`data-tooling-stats-20261011` (a stale, unpublished ProjectStats refresh).

- **Community evidence** (File → Analyze support gaps…) is in the repository,
  and its intake lives here (ADR-0091). The interim
  `KNXBench-Contributions` repository was deleted on 2026-10-08.
- **Marketing website** is in the repository (`website/`, ADR-0095) and
  deployed to knxbench.com by `.github/workflows/pages.yml`
  ([WEBSITE.md](WEBSITE.md)).

Do not work in the root checkout; publish each package from its own worktree
off `origin/main`.

## 3. Deferred by a recorded user decision

These stay open on purpose. Reopen only on a new decision.

| Item | Decision | Ledger |
| --- | --- | --- |
| ADR-0039 phases 3–5 (seal the id allocator, source-mutation gate) | Deferred past the alpha (2026-10-04); phases 1–2 and AR02 are the boundary | `KL-129` ACCEPTED_BOUNDARY |
| Package/application version pinning and a selector | Continued deferral (2026-10-04); first-installed winner with disclosed candidates | `KL-135` ACCEPTED_BOUNDARY |
| Live tunnel from a Docker bridge container (Route Back) | Optional, only with a user go; offline HPAI tests are delivered | `KL-155` DONE (residue in §155) |
| New real-hardware, power-loss, vendor and ETS validation | Removed from commissioning scope (2026-10-04) | many `commission` rows ACCEPTED_BOUNDARY |
| Durable pre-write recovery for K6 button / serial / K13 address writes | Confirmed public address writes fail closed before any tunnel until device-specific recovery exists; every new write needs its own go | ADR-0057–0059, `goal-commission.md` §3 (removed, see REMOVED_DOCS) |
| Reset UI, Web partial-scope selector | Reset UI: accepted refused boundary with a user notice; selector handed to UI and delivered (`KL-142` DONE) | `UI-04` ACCEPTED_BOUNDARY |

## 4. Later scope (never alpha tasks)

Native restart-safe undo and project versions have been delivered in the
user-authorized history package (code `62464763`, [contract](PROJECT_HISTORY.md),
[verification](status/2026-10-09-project-history-verification.md)). They are no
longer future work. Same-file versions still need independent backups; native
accessibility, physical power-loss and hardware/ETS recovery remain separate.

From the ledger's later-scope rows, historical `goal-commission.md` §3c and
the [original-ideas audit](status/2026-10-08-ideas-roadmap-audit.md). Root
`ideas.md` is ignored and is not present in a fresh clone. Each new feature
needs its own scoped goal or ADR when requested; old future-row labels do
not erase subsequently delivered functionality.

- **Commissioning, new media and procedures:** Powerline (PL110/PL132), KNX IP
  device configuration (masks `5705h`/`57B0h`), coupler filter tables
  (`0912h`/`091Ah`), `NM_Router_Scan`/`NM_SubnetworkDevices_Scan`, KNX Data
  Secure (`DM_SecureSync_*`), Easy Modes, USB interface configuration
  (`KL-110`, `GAP-T30-04`). RF stays simulator-only.
- **Model/import:** KNX `Functions` (`MODEL-07`, needs an ADR and samples);
  several installations under one Site (`MODEL-05`, needs an ADR); ETS
  restore-point revisions as extra private regressions (`IMPORT-02`, requires
  authorized ETS exports and reviewed baselines). Selective lines/devices and
  bounded `knx_cvexc` content analysis have verified local implementation and
  separately authorized [source integration](status/2026-10-10-import-expansion-integration.md);
  neither replaces the missing authorized revision exports. Dedicated
  time/colour/picture/slider parameter editors from Type attributes and UIHints
  (`PDB-04`); independent `.knxproj` samples for untested schemas
  (12/13/14/20/22) and an AES-protected ETS6 sample.
- **Comparison/report extensions:** applying/merging diffs, three-way
  comparison and native PDF output remain absent; comparison and browser
  print-to-PDF already exist. These are recorded boundaries, not active work.
- **Features:** project notes (`FUTURE-03`, ADR-0031; Core/native14/diff/report and application service implemented; authenticated HTTP/isolated UI/help and full feature delivery still open),
  repetitive-task automation (`FUTURE-02`), a mutation-capable LLM/MCP surface
  (`FUTURE-01`; the read-only MCP server shipped, ADR-0090), dashboard
  drill-down, mobile app, native ports and manufacturer online-catalog
  updates (`FUTURE-05`). More humour (`FUTURE-08`) means optional expansion:
  the original thirty-entry lists already exist. Per-category motion controls
  remain later scope; global motion controls already work.
- **KNX Secure follow-up:** [key provenance research](research/secure-key-provenance.md)
  answers the documented project/keyring routes (`R-SEC-01`). Offline keyring
  admission, a separate credential store, actual protected-project extraction
  and Secure interoperability remain new scope, not runtime support.
- **Tooling/statistics** (`TOOLS-01` to `TOOLS-05`): where the AI statistics
  generator lives, and the measurement gaps it discloses (network traffic, end
  times, unattributed sessions, CSS/shell metrics).
- **Platform validation:** native WebKitGTK, Orca/screen readers and complete
  accessibility, Firefox for the app; these are disclosed in every UI receipt.

### Ledger owner cleanup (2026-10-09)

On the user's go the ledger rows that lagged the code were reconciled:
`FUTURE-04` now cites the delivered telegram flow (`FLOW-01`, ADR-0077/0085),
`FUTURE-01` names the delivered read-only MCP slice (ADR-0090), `FUTURE-08`
states that only optional copy remains, and the four later-scope rows still
owned by finished sessions (`FUTURE-05`, `KL-110`, `GAP-T30-04`, `KL-43`) moved
to owner `later`. Routes naming removed goal files are kept as history.

## 5. Repository chores

- **Alpha9 publication** (11 October 2026): frozen tag `v0.1.0-alpha.9` and
  downloaded AppImage/MCP/Docker verification are complete. Native/model13
  requires pre-upgrade native/Docker backups. Unmounted editors and later local
  packages remain outside this release; publication is not live deployment.
  [Release scope](releases/alpha9.md) · [Receipt](evidence/release-alpha9-2026-10-11.json).

- **Docker Hub release image** (ADR-0097, KL §168): live since
  `v0.1.0-alpha.6`; Alpha.7 version/latest and both architecture manifests
  externally verified on 2026-10-10, including anonymous HTTPS/save/reopen
  smoke. [Publication evidence](evidence/release-alpha7-2026-10-10.json).
  Open by choice: image signing, Docker Hub description sync, scheduled
  base-image rebuilds, and the `latest` rule at the first stable release.
- **CI** is disabled (`gh workflow disable CI`, 2026-10-07) until the user
  re-enables it.
- **Repository visibility:** `KNXBench-Labs/KNXBench` became public on
  8 October 2026; the alpha.5 AppImage and checksum file are public too.
  Community-evidence/forms consolidation shipped separately; the interim
  intake repository is already retired. Private mailbox verification and the
  owner's retention decision for the private pre-launch rollback repository
  remain separate; this audit performs neither action.
- **Worktree prune (12 October 2026):** 21 linked worktrees whose HEAD was an
  ancestor of `origin/main` `5a33e95c` were removed. Their only residue was a
  handover copy, tool scratch or ignored logs. That residue (diff, untracked and
  ignored work files, sha256 manifest) is backed up locally outside the
  repository. Local branches were left in place. The read-only interview
  scratch `$TMPDIR/devnav-ro` no longer exists. Remaining parked worktrees are
  listed in section 2. After the owner's decisions the same day, four more
  worktrees went (functions kept as a local WIP branch), and the 24 local
  branches of all retired worktrees were deleted after an ancestor-of-main check.


### Schema23 review — scoped follow-up closed

Package7 bounded raw candidates delivered/read back `6ef0b2d8`; final source
`d8cde545` passed public/private acceptance, production build and outgoing privacy.
No RTF parsing/rendering/editing claim; that belongs to the separate tooling owner.
Own cleanup is recorded in the handover. Package8 KL146 owner selected read-only
diagnostic delivery with differences retained and no parity claim. Historical
comparison preserves input/project; diagnostic coverage and agreement now have
separate contracts. Final source `f045390fdf7b` passes full Rust/Web/build/repo/docs,
corpus and private diagnostic/RefId/own-values/reconciliation/raw-evidence gates.
Diagnostic published/read back no-ff `70c05fdc58e5`, outgoing privacy clear;
KL146 activation differences/unsupported scoped sets remain an ACCEPTED_BOUNDARY,
not an owner blocker or a producer-semantic fix. All nine review packages are
delivered within their accepted scopes; exact twelve-ID states and gate/source
publication proof are in the [closure receipt](evidence/private-schema23-review-closure-2026-10-11.json).
No import-integrity/owner blocker remains for this follow-up. Product-wide and
research-bound implementation work in this file is not closed by that receipt.
