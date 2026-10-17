# Open work after the 0.1 alpha

## Local machine details: Git history decision pending (17 October 2026)

The tracked tree no longer carries local paths, host name or kernel release and
is guarded ([ADR-0135](adr/0135-repository-carries-no-local-machine-details.md)).
Earlier commits and seven commit messages still do
([KL-185](KNOWN_LIMITATIONS.md#catalogue-kl-185)). Next: the owner decides
whether to rewrite history (force-push, new hashes, moved tags) or keep it.

## Read-only diagnostic export: hardware check pending (17 October 2026)

[Implemented and simulator-verified](contracts/device-diagnostic-export.md);
next is one separately authorised read-only run against a named device, then
any memory profile only with its own evidence and decision (ADR-0134).

## System 2 / System B download outcome (17 October 2026)

Owner renewed the usable-download/admission objective. It remains **blocked on
source semantics and full-state recovery**, not merely missing hardware. The
[fresh source-bound recheck](research/2026-10-16-offline-source-stop-lines.md#17-october-renewed-download-outcome-and-fresh-checks)
passed 53 existing tests in seven targets, including the digest-matched public
UP258 package; no production implementation or download admission changed.
Next: defining converted-legacy patch/default/activation evidence for the
System-2 witness, or source/profile-specific complete descriptor and relative
procedure mapping for System B, followed by complete execution/recovery proof.
The discovered official System-B help body could not be recovered; no search
snippet, guessed mapping, vendor contact or physical write substitutes for it.

## Alpha 13 release delivered (16 October 2026)

[Alpha 13](releases/alpha13.md) is published and externally verified. Release
preparation is closed, not a new feature package. MV-0025 programming, new module
construction and complete source/procedure/recovery work below remain open; no
physical operation or user-container update occurred.

## Parameter-driven module object creation (16 October 2026 source)

[Supported unscoped object creation](contracts/parameter-object-refresh.md)
reconciles the object list after ordinary parameter edits. Still open: verified
module-instance/repeat/number-offset construction for a newly activated object
not already represented by an imported instance. The current route refuses that
whole parameter write, preserves the project, and never guesses the mapping.
This is a shared manufacturer-data boundary, not a vendor-specific exception.

## System-300 offline prerequisites (16 October 2026)

[Pure S-mode property-table images](contracts/offline-system300-property-tables.md)
are published as `af9431cb`, with twelve focused tests and19actual merged stages
including isolated corpus152/152. Earlier branch artifact reconciliation remains
separately recorded. Still open: source/application mapping, discovered
object indices/instances and format/capacity, property framing, full load sequence,
parameter handling/rescan, verification and durable whole-mutation recovery.
Shared Type6 bytes do not make System300 an admitted SystemB device.

## Offline System-1 completion (16 October 2026)

Follow-up: [explicit single-octet parameter bits](contracts/offline-explicit-parameter-bits.md)
no longer require template/default equality. The latest same-source measurement
initially had7FixupList/3inferred-default/1Union refusals and2bounded candidates.
Those candidate returns are withdrawn: LegacyPatchAlways refusal closes the
unhandled directive in the strict System1/System2 image APIs. Existing Easy3
semantics/admission are not changed or newly established by this slice. Its
legacy-directive interpretation remains a separately documented open limitation. Current configured
refusals are7FixupList/2legacy-patch/3inferred-default/1Union. No complete semantic
image is accepted. The original P5 distribution below remains historical.

[The bounded source-image candidate](contracts/offline-download-system1-image.md)
passes synthetic/owning tests but accepts no complete real-product source image.
Next are exact FixupList semantics, source-default/Union encoding evidence,
complete source assembly and then default-procedure translation plus durable
whole-mutation recovery. Final thirteen-source refusals are7/5/1 respectively;
they identify first boundaries, not all missing semantics. Public0012 admission
is unchanged. Nineteen candidate and twenty actual merged stages pass; published
as `ccfe5b74` with isolated corpus152/152. This delivers the bounded assembly
infrastructure, not any of the unfinished source/procedure/recovery work above.

## Siemens System-2 download support, issue #22 (14 October 2026)

Owner priority, 16 October: finish the exact UP258 application vertically, not
additional family-wide foundations. [The fresh vertical audit](research/2026-10-16-up258-vertical-path-audit.md)
reproduces the current first transition: six LegacyPatchAlways declarations
with unresolved default/invisible activation semantics. Two configured source
regressions and a literal 21-position oracle do not close that rule. No complete
image/plan/recovery was delivered; defining evidence is required before continuation. The [production decision](research/2026-10-16-up258-production-decision.md)
and [one prepared reporter request](support/issue-22-up258-evidence-request.md)
identify missing encoding/XML/resource and whole-mutation recovery contracts.
The direct-field comparison is proposed evidence, not permission for any device
operation; the request has not been posted. A checksum-codec correction alone
does not make the detector programmable.

Exact product/application identified; [offline prerequisites](contracts/system2-download-prerequisites.md)
add typed TaskCtrl2 parsing and its property-record codec, not programming support.
Open: verified System-2 table/image representation, complete product-procedure and
machine-5 mapping, hardware-comparison semantics, and durable property+memory
recovery before executable planner admission. `MV-0025` stays refused. Firmware
25N03-V04 onward must be confirmed before separately authorised hardware work.
No private project or manufacturer-executable upload is needed for this stage.

## ETS 5.7 project import, issue #7 (12 October 2026)

Schema 20 imports and real ZipCrypto exports decrypt
([KL §177](KNOWN_LIMITATIONS.md#177-ets-57-projects-schema-20-rest-on-three-public-samples-sub-module-object-ids-are-not-decoded)).
Open: the reporter's own project (asked on #6/#7), nested sub-module object ids
(`SM-`, reported with number 0; needs the project-side `SubModule` grammar),
schema 22 (needs an ETS 6.1 sample with devices). Released in Alpha 10; Alpha 11
(13 October 2026) changes nothing here and the reporter was invited to try it.

**13 October 2026:** the reporter's ~80-device project imports on Alpha 10, with
2840 warnings. *Export log (JSON)…* failed in the AppImage: fixed in source
([KL §181](KNOWN_LIMITATIONS.md#181-appimages-up-to-alpha11-cannot-save-the-session-log-or-a-bus-monitor-capture)),
reaches users with the next AppImage. Most warnings were one `import:unsupported` line
per retained manufacturer file (`Baggages/…`); the Log now groups those notices (one
entry listing every file) and counts them once
([delivery](IMPLEMENTATION_STATUS.md#2026-10-13--issue-7-retained-files-stop-drowning-the-import-findings)).
Open: the reporter's log or report (the Docker image exports it today), to see which
genuine findings remain. AppImage downloads now ask where to save instead of
silently replacing files in Downloads
([KL §182](KNOWN_LIMITATIONS.md#182-appimages-up-to-alpha11-save-in-app-downloads-silently-into-downloads-and-replace-same-named-files));
a by-hand check in a real window (X11, 13 October) confirmed the save dialog
for the report download (Cancel and Save) and the log export (Save); the
Wayland backend and the bus-monitor capture export were not driven.
With a product database most of the warnings were unknown attributes in
manufacturer application files; those are grouped too, and the Log now keeps
16000 entries, dropping the least important first
([delivery](IMPLEMENTATION_STATUS.md#2026-10-13--session-log-holds-16000-entries-and-drops-the-least-important-first)).

## AppImage EGL abort, issue #6 (12 October 2026)

Fixed in source: the AppImage no longer bundles `libwayland-client`
([contract](APPIMAGE_LAUNCHER.md#host-provided-libraries-issue-6), KL §176). The
fix shipped in Alpha 10 (12 October) and the issue was closed the same day; the
reporter's Kubuntu 24.04 confirmation never arrived. Still open: a startup smoke
on a newer-Mesa distribution (CI's build-distribution smokes cannot catch this
mismatch).

## Checked parameter editors in the device editor (12 October 2026)

Time/Number/Colour/Picture rows offer a **Checked editor**
([host](contracts/parameter-editor-host.md#workbench-host-12-october-2026)): exact
source evidence verified per field, writes only through the existing route, drafts
kept across Save. Open: module-scope source mapping, verified Colour/Picture value
codecs (PDB-04 stays LATER), adopting the server's revision-bound parameter writes
and installation renames in the web host (Goal B1;
[contract](contracts/revision-bound-writes.md), server side delivered 13 October),
native WebKitGTK/Orca checks.

## Installation instructions in the device editor (12 October 2026)

The instruction editor is the device editor's sixth tab
([host](contracts/instruction-editor-host.md#workbench-host-12-october-2026)):
real tree identity (read-only without one), refresh after every Apply, drafts kept
across tabs and Save, guarded on Open/Import/New/Quit. Devices made in KNXBench get instructions since
Goal B4, without an invented original. Open: other source schemas/encodings, an
ETS export of edited overrides, native WebKitGTK/Orca checks.

## Project notes in the workbench (12 October 2026)

The notes editor is mounted as the **Notes** workbench view
([host](contracts/project-note-editor-host.md#workbench-host-12-october-2026)):
real tree identity, refresh after Apply, drafts kept across views and guarded on
Open/Import/New/Quit. Follow-ups of 12 October (source, after Alpha 10): the
service's target catalogue including communication objects, a browser
reload/tab-close prompt while drafts exist, and EN/DE action names in the review.
Since Goal B2 the desktop webview asks before a reload too (source-based; run in
Chromium and Firefox, not in a native WebKitGTK window). Open: native
WebKitGTK/Orca checks, and FUTURE-03 as a whole (still LATER).

## AP7 catalogue updates (12 October 2026)

`knx products update` previews a manufacturer `.knxprod` against the installed
catalogue, installs it only with the preview token after a backup, and `knx
products restore` reverts ([ADR-0117](adr/0117-previewed-recoverable-catalogue-updates.md)).
The web catalogue has the same flow since 12 October
([ADR-0118](adr/0118-catalogue-updates-through-the-web-server.md)). Open: any
automatic update source (none is documented for third-party tools), moving a
device to a newer application version. Since Goal B5 (14 October) the web deletes a
listed backup after asking and installs every `.knxprod` through the preview.

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
`f70183f0` is verified. Since Goal B7 the web app offers it as **Rename…** for a
group-address selection. It does
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


**Baseline audit:** 2026-10-08, against `origin/main` `608a204b`; the dated
sections on this page were added since, the newest on 12 October 2026. This page is the one
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

## KL-170 evidence and probe (13 October 2026)

[Parameter byte order](research/parameter-byte-order.md) found no defining
source but strong corpus evidence for whole-octet numbers low octet first,
undermined by the same pattern in BigEndian-declared and undeclared programs.
`knx device byte-order-probe` ([ADR-0119](adr/0119-read-only-byte-order-probe.md))
reads an ETS-programmed device without writing. **Open:** probe output from
ETS-programmed LittleEndian devices (none available to the project; issue #10's
reporter has one; asked on 2026-10-13); then a reviewed, observation-limited
encoder change. Owner decision taken: downloads of BigEndian/undeclared programs
whose base images look little-endian carry the named warning
`base-image-low-octet-first` ([ADR-0120](adr/0120-base-image-byte-order-warning.md));
the tester verification flow the owner asked for on 2026-10-13 is done in
both parts ([ADR-0121](adr/0121-tester-download-reports.md)): reports from CLI
and server, the issue form and review procedure, and the download tab's plan
warnings, assumptions and **Help verify this device**. **Open:** the first
real tester report and its review.

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
| Legacy ETS3 product databases (`.vd3`–`.vd5`) | L1 inspection, L2 import/secret withholding, L3 web upload/password handling, the VD5 installer layout with measured bounds and L4 download plans from `s19_block` are on `main` | A first live legacy download (needs the maintainer's device go; none has run). Atomic types 3/5 are mapped (2026-10-13, KL-128); their `BinaryValue`s, and `.knxprod` `Enumeration/@BinaryValue`, are not modeled (the `.knxprod` ingest does not even report it). Merged legacy procedures (`07B0h`) stay refused. | [ADR-0094](adr/0094-legacy-exim-product-files.md), [VD4 import](VD4_PRODUCT_DATABASE_IMPORT.md), KL §128 |
| Website / published Evolution Story | Edition `2026-10-08.4` is approved and deployed with the site; first publication is complete | HTTP→HTTPS (301) and remote IPv6 HTTPS (four probes, 200/TLS valid) verified on 2026-10-09; Pages Node-24 actions and demo links deployed/verified; owner's privacy review and optional WebKit evidence remain separate | [story/](../story/README.md), [WEBSITE](WEBSITE.md), ADR-0068/0095 |
| Web integration of finished leaves | All four packages delivered on 12 October: project notes, installation instructions, checked parameter editors, catalogue updates with backup/restore | None for this track. Colour/Picture keep the visible fallback; PDB-04 stays LATER | Web-lock lines in `.ai/CURRENT_STATE.md`, [parameter host](contracts/parameter-editor-host.md), [catalogue updates](contracts/catalogue-update.md) |
| Parked-work adoption | Package 1 (native-evidence port) delivered on 12 October; package 2 (ux-feedback port) delivered on 13 October | Native-evidence residue check done on 13 October (nothing left, no schema bump); both parked worktrees removed on 13 October after an owner go (branches kept); stale-claim corrections re-checked and the ADR index completed on 13 October. Nothing left in this track | Web-lock lines in `.ai/CURRENT_STATE.md` |
| Group-address views follow-up | Issue #4 closed as completed on 14 October; the reporter may still send feedback | Derived numeric 2-/3-level tree (GA7, ADR first) only if asked for; virtualization (GA8) only on request or a failing measurement | [goal record](goals/group-address-four-views.md) |

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
| `native-evidence-20261011` (local branch; worktree removed 2026-10-13) — residue checked 2026-10-13: nothing left | 7 locally accepted commits: durable native import reports (ADR-0113), CLI/source-report/payload catalogues, export-privacy consent, read-only project metadata and recorded device state | **Delivered 2026-10-12**: ported onto main with a fresh gate ([evidence](evidence/native-evidence-port-2026-10-12.json)). Local branch retired 2026-10-14 after an owner go; original commits kept in the `KNXBench.backups/2026-10-14-hygiene-cleanup/` bundle |
| `feature/ux-feedback-20261010` (local branch; worktree removed 2026-10-13) | 13 locally accepted commits, UX AP1–AP5 | **Delivered 2026-10-13**: AP2/AP3/AP5 and the parts of AP1 main lacked, ported with a fresh gate ([evidence](evidence/ux-feedback-port-2026-10-13.json)); motion ADR lands as ADR-0122; AP4 dropped. Worktree removed after an owner go; local branch retired 2026-10-14, original commits kept in the `KNXBench.backups/2026-10-14-hygiene-cleanup/` bundle |
| `feature/functions-sites-20261009` (local branch only) | KNX Functions (`MODEL-07`) as one unreviewed `wip(functions)` commit `1154b72f` on schema 12 (main is at 14); acceptance attempt 4 failed | Superseded 2026-10-13: Goal A4 rebuilt the backend fresh on schema 15 ([ADR-0124](adr/0124-project-functions.md)); the WIP was read, not merged. Branch and its stash (`Functions source before published History sync`) dropped by the owner 2026-10-14; both kept in the `KNXBench.backups/2026-10-14-hygiene-cleanup/` bundle |
| Initial native-write preflight | Never-compiled draft from `import-lifecycle-20261011` | Archived as a local patch; to be built fresh. Worktree and branch removed |
| `audit-findings-20261011`, `native-import-diagnostics-20261011` | Superseded / already on main | Dropped. Worktrees and branches removed |
| `wip/manufacturer-plugin-research-20261013` (local branch only) | 14 unpublished research notes under `docs/research/` (manufacturer-plugin host/contract results, fast-download wrapper, DLLs on Linux), found in the root checkout; unreviewed, none of them on main | **Kept by owner decision 2026-10-14**; not merged, not pushed |
| `wip/root-residue-20261013`, `feature/device-support-guidance-20261013`, `feature/device-support-guidance-s2-20261013` | Old root drafts; two pre-integration candidates of the support guidance delivered as `e478a1a0` | Retired 2026-10-14 by owner decision: worktrees removed, branches kept only in the `KNXBench.backups/2026-10-14-hygiene-cleanup/` bundle. The S2-only Codex collision log was copied to the root's ignored `.ai/logs/` |

The two other leftover worktrees, `offline-naming-20261011` and
`data-tooling-stats-20261011`, were removed by owner decision after a backup.
The unaccepted numbered device-naming draft (FUTURE-02) survives only in the
local prune backup; see the triage note.

- **Community evidence** (File → Analyze support gaps…) is in the repository,
  and its intake lives here (ADR-0091). The interim
  `KNXBench-Contributions` repository was deleted on 2026-10-08.
- **Marketing website** is in the repository (`website/`, ADR-0095) and
  deployed to knxbench.com by `.github/workflows/pages.yml`
  ([WEBSITE.md](WEBSITE.md)). The owner-revised original-layout phosphor CRT
  design (installation diagram removed) is published and live-verified on
  15October2026; no remaining website publication work for this package.
  See [its publication receipt](evidence/phosphor-crt-publication-2026-10-15.json).

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
- **Model/import:** KNX `Functions` (`MODEL-07`): backend, storage,
  schema-23 import, projection and HTTP API delivered
  ([contract](contracts/project-functions.md)); web workbench delivered
  (Goal B1b); a per-Function diff table, desktop and MCP surfaces later.
  Several installations under one Site (`MODEL-05`): blocked on evidence
  ([ADR-0125](adr/0125-multi-installation-site-is-blocked.md)); ETS
  restore-point revisions as extra private regressions (`IMPORT-02`, requires
  authorized ETS exports and reviewed baselines). Selective lines/devices and
  bounded `knx_cvexc` content analysis have verified local implementation and
  separately authorized [source integration](status/2026-10-10-import-expansion-integration.md);
  neither replaces the missing authorized revision exports. Dedicated
  time/colour/picture/slider parameter editors from Type attributes and UIHints
  (`PDB-04`); independent `.knxproj` samples for untested schemas
  (12/13/14/22; schema 20 rests on public samples, KL §177) and an
  AES-protected ETS6 sample.
- **Comparison/report extensions:** applying/merging diffs, three-way
  comparison and native PDF output remain absent; comparison and browser
  print-to-PDF already exist. These are recorded boundaries, not active work.
- **Features:** project notes (`FUTURE-03`, ADR-0031; Core/native14/diff/report and application service implemented; authenticated HTTP/isolated UI/help and full feature delivery still open),
  repetitive-task automation (`FUTURE-02`), a mutation-capable LLM/MCP surface
  (`FUTURE-01`; the read-only MCP server shipped, ADR-0090), dashboard
  drill-down, mobile app, native ports and manufacturer online-catalog
  updates (`FUTURE-05`). More humour (`FUTURE-08`) means optional expansion:
  the original thirty-entry lists already exist. Motion has global controls and,
  since 2026-10-13, per-category ones for Toasts, Achievements and Flow (ADR-0122).
- **KNX Secure follow-up:** [key provenance research](research/secure-key-provenance.md)
  answers the documented project/keyring routes (`R-SEC-01`). Offline keyring
  admission, a separate credential store, actual protected-project extraction
  and Secure interoperability remain new scope, not runtime support.
- **Tooling/statistics** (`TOOLS-01` to `TOOLS-05`): where the AI statistics
  generator lives, and the measurement gaps it discloses (network traffic, end
  times, unattributed sessions, CSS/shell metrics;
  [contract](contracts/statistics-measurement.md)).
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

- **Alpha12 publication** (15 October 2026): immutable `v0.1.0-alpha.12` on
  `16aefbe6`, complete77/77 source chain and corpus152/152+product16/16 green.
  Public AppImage/MCP checksums, Docker amd64/arm64 version/latest identity,
  anonymous network-none HTTPS/auth/save/reopen and authentic schema10→16 fixture
  migration verified; user container unchanged. AppImage Broadway/embedded API
  startup passes, not native visual/FUSE/EGL acceptance. Live Pages root and all36
  public content files match accepted bytes. Native/model16 and product23 require
  independent pre-upgrade backups. Open: #7/#22 and explicit platform/hardware
  limitations, not release packaging. [Scope](releases/alpha12.md) ·
  [Publication receipt](evidence/release-alpha12-2026-10-15.json).

- **Alpha11 publication** (13 October 2026): tag `v0.1.0-alpha.11` on `758a112b`
  (main cutoff `2a43ad5c`), 44-stage gate and corpus 144/144 green, downloaded
  AppImage/MCP/Docker verification complete. Schema stays 14. Open: the first real
  tester download report, #10's reporter on a real gateway, a native WebKitGTK check
  of the new screens. [Release scope](releases/alpha11.md) · [Receipt](evidence/release-alpha11-2026-10-13.json).
- **Alpha10 publication** (12 October 2026): tag `v0.1.0-alpha.10` on `73420e13`,
  downloaded AppImage/MCP/Docker verification complete. Native/model 14 requires
  pre-upgrade backups. Open: confirmation from the #6 and #7 reporters on their own
  machines/projects. [Release scope](releases/alpha10.md) · [Receipt](evidence/release-alpha10-2026-10-12.json).
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
  The remaining leftovers (three local branches, one merged remote branch, three
  stray folders) were sorted too; see the
  [triage note](status/2026-10-12-parked-work-triage.md#leftover-branches-and-folders-sorted-2026-10-12).
- **Stale documentation claims (fixed 12 October 2026):** the five superseded
  statements listed in the
  [triage note](status/2026-10-12-parked-work-triage.md#leftover-branches-and-folders-sorted-2026-10-12)
  were re-checked against source and reworded.


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
