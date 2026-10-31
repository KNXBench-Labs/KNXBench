# Roadmap

## Settings in the center workspace (30 October 2026 source, after Alpha 18)

The existing app settings move from a modal window to a continuous center page,
with section jumps at the top. The left navigation stays usable; Properties is
hidden temporarily without changing its visibility preference. Project selection
and unapplied note/device-instructions drafts survive the detour. The same
settings and acknowledged persistence paths remain in use; there is no new Save
action, storage migration, safety permission or bus operation. This is newer than
the published Alpha 18 artifacts, not a release or deployment.
[Contract](contracts/settings-workspace.md) ·
[User guide](manual/user-guide/09-settings-and-appearance.md#opening-settings).

## DIAG-03 local candidate after Alpha 18 (29 October 2026)

Operation recording and the unified diagnostic bundle remain in progress, not
released. The saved-project opt-in adapter is locally integrated (`f017dc248`),
with saved-root/native-reopen and review/privacy refusal tests. Full private
System-2 comparison export is blocked on credential classification of both byte
sides and all overlapping archive copies; metadata-only withholding is not raw
evidence delivery. Final integrated review/package acceptance and main-first
publication remain open. [Work and evidence boundary](open-work/DIAG-03-unified-diagnostic-bundle.md).

## Alpha 18 pre-release (source cutoff 28 October 2026)

`v0.1.0-alpha.18` is the current pre-release, cut from main at source cutoff
`bb2d8e51e`. It ships the deliveries in the "in Alpha 18" sections below: the issue
#31 result/report/settlement work with the System-2 final-step failure category,
the issue #30 tunnel-cause, persistence and patient-wait follow-ups, S2-03
defaults profile v3, group-address CSV profiles, German CLI reports, the explicit
catalogue installation target, `TypeDate` and legacy long-enum retention (product
database 28 → 30, data-only), and the desktop separator fix. Native/model 16 is
unchanged. No hardware support, recovery guarantee or ETS equivalence follows; the
allocation mismatch of issues #30/#22 is unresolved and both stay open.
[Release scope and boundaries](releases/alpha18.md).
Work merged on main after `bb2d8e51e` belongs to the next alpha.

## Alpha 17 published pre-release (26 October 2026)

`v0.1.0-alpha.17` is the previous published pre-release, cut from main at
`20ab2f7ea`. It shipped every delivery below up to that cutoff: the issue #30
System-2 changes (prestate observations, own-run gateway hint, PID 7 widths and
typed refusal diagnostic), the regular-download correctness and failure-cause
work, Diagnostics-tab entry points, association link adoption, saved object
folders, Flow group names, the version timeline and sortable catalogue columns.
Native/model 16 and product database 28 are unchanged. No hardware support,
recovery guarantee or ETS equivalence follows; issue #30, S2-02 and S2-03 stay
open. [Release scope and boundaries](releases/alpha17.md).
Publication is verified at immutable `a9ea32d72364478b25e158d0b95465af7a4702df`
([public verification record](evidence/release-alpha17-2026-10-26.json)).
Deliveries after `20ab2f7ea` through `bb2d8e51e` ship in Alpha 18.

<a id="system-2-final-step-failure-category-after-alpha-17"></a>

## System-2 final-step failure category in Alpha 18

The unfinished final verification step is split into mismatch, read failure and
write failure with a closed stage, carried through journal, status and report. It names the
operation that stopped, not its cause; the allocation mismatch in #30/#22 and any hardware
evidence stay separate. Ships in Alpha 18: delivered main-first as `309384726`, with reviewed
branch/integration acceptance and a built-server smoke; the later docs-only delta does
not relabel the accepted runtime revision. [Software acceptance](status/2026-10-28-system2-final-step-failure.md#software-acceptance-and-source-delivery) ·
[Contract](contracts/system2-download-result.md#final-step-failure).

<a id="system-2-result-integration-after-alpha-17"></a>

## System-2 result integration in Alpha 18

The implementation connects experimental result/report presentation and
project-belief settlement without merging the two protocol executors. A verified
run needs current source/preparation evidence before clearing marks; uncertain
writes invalidate memory-part beliefs outside undo. Application-level status
ownership and a global stale-mark notice survive navigation. Reports of ended
retained operations require no new bus contact. [Scope and software acceptance](status/2026-10-28-system2-result-integration.md).
The allocation mismatch in #30/#22, per-part source equivalence and physical
acceptance remain separate; no device retry is implied. Ships in Alpha 18.

<a id="legacy-binary-enumeration-retention-after-alpha-17"></a>

## Legacy binary enumeration retention in Alpha 18

[WP-03](open-work/WP-03-legacy-binary-enumerations.md) is implemented (Alpha 18):
validated legacy long-enum bytes reach the existing Base64 column; data-only
productdb v30 backfills authenticated retained payloads without rewriting
historical reports. Invalid/ambiguous inputs and ordinary-enum binary fields
remain explicitly diagnosed. This closes the storage/backfill package, not
legacy image/programming or all-file conversion equivalence. No deployment or bus action follows.

<a id="experimental-system-2-waits-after-alpha-17"></a>

## Experimental System-2 waits in Alpha 18

Alpha 18 selects a [patient wait profile](contracts/system2-patient-waits.md)
for experimental hardware download and explicit recovery: gateway ACK waits
1 → 10 seconds per attempt, post-ACK management answer slices 3 → 15 seconds and
load-state transitions 30 → 120 seconds. Device transport ACK/repetition stays
at three seconds. Ordinary tunnel users, retry counts,
backup and final verification are unchanged. The existing receiver/teardown fix
is included; neither change proves the cause of issue #30 or a successful device
outcome. Published source `3583ab8e0` passed branch/integrated 14-stage gates;
[acceptance and retest boundary](status/2026-10-27-system2-patient-waits.md#acceptance-boundary).
One optional reporter retest has been requested; it needs Alpha 18 or a newer build.

<a id="up258-invisible-parameter-reading-after-alpha-17"></a>

## UP258 invisible-parameter reading in Alpha 18

[S2-03](open-work/S2-03-up258-image-interpretation.md) is delivered as source
`d39668393`: defaults profile v3 shares the configured path's template-retention
reading, admitted only for an explicit `DownloadInvisibleParameters="Background"`;
new physical v1/v2 execution refuses, recovery stays. Observation-supported
hypothesis on simulator evidence; no hardware or ETS-equivalence claim. Ships in Alpha 18.

<a id="server-tunnel-persistence-after-alpha-17"></a>

## Server tunnel persistence in Alpha 18

The ordinary download worker and service-control backup callback now keep
synchronous recovery/journal persistence off tunnel runtime workers on multi-thread
runtimes, following the System-2 fix. Current-thread fallback remains inline. [S2-04](open-work/S2-04-tunnel-loss-after-checkpoint.md) adds the
individual-address settler (preparatory; its route still refuses) and retains field verification; ships in Alpha 18. Consent, backups, stop-token policy and
read-back remain unchanged.

## Docker signing design before the first stable release

The [bounded proposal](research/2026-10-27-docker-image-signing-design.md) records
exact release-workflow identity, signed-index verification, staged promotion and
recovery requirements. This completes design preparation only: owner approval,
implementation and separately authorised registry acceptance remain pending.
No image, workflow, channel rule or release artifact changes; images remain unsigned.

<a id="explicit-catalogue-installation-target-after-alpha-17"></a>

## Explicit catalogue installation target in Alpha 18

Alpha 18 requires an explicit **Installation** choice when creating a
line-less catalogue device in a project with several installations. The existing
create/Undo path is reused, with live target validation and captured retry
identity; the one-installation body and line-owned target are unchanged.
This closes a consumer gap, not the deferred multi-installation/site model or
cross-installation transfers. [Scope and retained boundaries](limitations/structure-editor-scope.md#update-2026-10-27-explicit-catalogue-installation-target).

<a id="cli-report-language-selection-after-alpha-17"></a>

## CLI report language selection in Alpha 18

Alpha 18 allows an explicit EN/DE locale in `knx doc-export`, reusing the
existing renderer. The default stays English and invalid options cannot write
an output. [Command-line workflow](manual/user-guide/08-reports-and-diff.md#exporting-from-the-command-line).
No new language or external-report parity is added.

<a id="group-address-csv-import-profiles-after-alpha-17"></a>

## Group-address CSV import profiles in Alpha 18

Four fixed [name/address column layouts](contracts/group-address-csv-profiles.md)
are supported by the shared CSV import path since Alpha 18. This is an
address-only improvement, not a range/security synchronisation format. Omitted
ranges and security policy are explicit diagnostics; unsupported metadata
refuses the entire import. Native export, native format behaviour, schema and
hardware scope are unchanged. No external-application parity is implied.

## Native acceptance after the Alpha 17 release

Owner consent for isolated [WP-11](open-work/WP-11-native-linux-acceptance.md)
tests is granted (27 October). The first bounded native Wayland/WebKitGTK
slice exercised the published Alpha 17 AppImage with fictional data and no
external network: keyboard/focus, project save/reopen, note drafts and bounded
zoom/menu controls. [Acceptance and exclusions](status/2026-10-27-wp11-native-acceptance.md).
A [positive native continuation](status/2026-10-27-wp11-positive-native-editors.md)
now verifies one declared integer slider and native-created-device plain
instructions, including persistence across actual process restart. A separate
[bounded capture continuation](status/2026-10-27-wp11-native-capture-export.md)
now records actual native Export Cancel/Save/overwrite Cancel/Replace against
one fictional received UDP frame, with exact JSON/loss and ordinary-close
oracles. WP-11 stays open for other controls/imported-original instructions,
virtual-key replacement attribution, remaining keyboard/layout/pointer cells
and unavailable environments; no code, hardware or release capability is added.
The separate confirmed owner decisions are recorded in
[OWN-01](open-work/OWN-01-owner-decisions.md), including MODEL-05's deferral and
retention of the S2-03 communication-bit policy.

## Deliveries up to the Alpha 17 cutoff

Website task WP-21 is delivered and deployed (`b517930e7`), with reviewed
selective typing, static/reduced-motion fallback and exact live artifact/browser
readback ([acceptance](status/2026-10-26-website-headline-typing.md)). Flow task
WP-23 is separately delivered as `903d5d0c1`, with generation-bound unique group
names and unchanged raw identities ([acceptance](status/2026-10-26-flow-group-names.md)).
WP-22 is delivered as `132070176`: the existing native versions now have a
selection-only timeline with unchanged guarded restore semantics
([acceptance](status/2026-10-26-project-version-timeline.md)). WP-24 is delivered
as `8dd6cf72d`: native sortable catalogue columns, stable product identities and
polite keyboard-highlight announcements, without catalogue-data changes
([acceptance](status/2026-10-26-catalogue-table.md)).

System-2 [S2-01](open-work/S2-01-pid7-raw-response.md) is a delivered engineering slice without hardware evidence yet: the private diagnostic
prefix is published as `5f1ad454e` after review and integrated acceptance
([record](status/2026-10-25-system2-pid7-diagnostic.md)). A 26 October field reply
supplies a two-octet value on object 1; independently retrieved public package
metadata declares two/four-octet types. An exact-profile, versioned width model
is published as `54b052885` after review, correction re-review and the complete
15-stage integrated package gate
([record](status/2026-10-26-up258-pid7-width.md),
[research](research/2026-10-26-up258-pid7-width.md)). Objects 2/3 remain
source-declared hypotheses. A typed, closed-allowlist status/report/UI
projection of the refusal (no raw text) is delivered as `536e492bc`; hardware
evidence remains open. Physical recovery evidence and issue #30 remain open
(S2-02/S2-03). [S2-02](open-work/S2-02-gateway-busy-after-failed-runs.md)
has a delivered own-run gateway-hint/refusal-reason diagnostic slice (`aa2f39f1b`).
The reporter's newer run starts without that banner, but the earlier actual
start failure remains unexplained. Neither package establishes hardware support.

Run-scoped [download receipts](contracts/download-settlement-receipt.md) distinguish
preparation, applied or computed settlement, and the separate post-download check.
Independent reviews and the 15-stage integrated gate passed on `3431aabbc`
([acceptance](status/2026-10-25-download-settlement-receipt.md)); final documentation
integration preserves runtime bytes. Complete effects, provenance baselines and
whole-device recovery remain separate open work.


TypeNumber declared-bound admission is fail-closed in image construction, with
unchanged raw source and integer encoding. Published as `87d71d4aa` after review,
correction re-review and all 14 integrated package stages
([acceptance](status/2026-10-25-parameter-number-bounds.md)). No wider numeric
lexicon, Float support or inactive/Union/customer-write policy is established.

The [duplicate image-start guard](contracts/duplicate-image-segment-start.md)
refuses ambiguous allocated data-bearing starts without dropping a payload or
inventing merge precedence. Published as `d73a2413b` after independent review and
all 14 integrated package stages
([acceptance](status/2026-10-25-duplicate-segment-start.md)). Payload-free
allocations remain valid; different-start overlaps and broader families are not admitted.

The [regular prewrite-stop contract](contracts/regular-download-prewrite-stop.md)
is published as `6c9ca2d7d` after independent reviews and all 14 integrated stages
([acceptance](status/2026-10-25-regular-prewrite-stop.md)). Server shutdown can
prevent the first mutation; a committed download still runs unchanged within the
existing bounded drain. Per-step effects, mid-procedure stop and recovery remain open.

## Regular programming correctness — 25 October 2026 source

The [memory transfer budget](contracts/memory-transfer-budget.md) composes a
conservative default hardware-route cap with service-specific header arithmetic.
At a standard frame it sends 12 ordinary or 11 user-memory data octets; real
interface/coupler capacity acquisition and extended-path authority remain open.
Original semantic regression and independent initial/correction reviews passed;
all 16 selected maintained integrated stages passed, published as `1e6229cea3`
([record](status/2026-10-26-memory-transfer-budget.md)). No family is admitted.

The [regular fragment-admission contract](contracts/regular-merge-fragment-admission.md)
conservatively refuses an untranslated declared shape without inventing merge
semantics. Focused semantic regressions, independent review and all 14 selected
maintained integrated stages passed; published as `be2687796`
([acceptance](status/2026-10-25-regular-merge-fragment-admission.md)). Parameter
policies, device baselines and complete mutation/recovery contracts remain
separate; no new family is admitted.

The [partial application identity correction](contracts/partial-identity-comparison.md)
requires the existing object 3/PID 13 precondition to answer exactly five octets
before backup or mutation. Complete downloads acquire no extra precondition.
Independent review, focused simulator checks, integrated acceptance and
publication as `02c437110` are complete. Advice from project programming marks
is also published (`78fd9d780`) across core, app, CLI, HTTP and web; explicit
scopes and partial refusal boundaries remain unchanged
([acceptance](status/2026-10-25-programming-scope-and-identity.md)). These are
bounded increments, not completion of all programming gaps.

The [hardware identity width correction](contracts/hardware-identity-comparison.md)
closes one pre-mutation response-shape gap in the existing download/restore path.
Focused simulator acceptance, independent source review and the maintained
integrated package gate passed; published as `07fe2e67f`
([delivery](status/2026-10-25-hardware-identity-width.md)). Unified target identity, parameter policies and
complete effects/recovery remain separate work; no broader hardware support is
inferred.

The [read-only check after a regular download](contracts/post-download-check.md)
is published as `8b5edad7f` after independent source reviews and maintained
integrated software acceptance, including HTTP corpus and EN/DE browser tests
([acceptance](status/2026-10-25-post-download-check.md)). That original increment
reports reach, identity and load states, without a runtime or physical-exemplar
match. The [29 October property run-state continuation](contracts/post-download-run-state.md)
adds a separately evidenced observation of one validated plan-matching
Applicationprogram object on exact `0701h` / `0705h` masks; it does not establish
installation function, object uniqueness or physical-exemplar identity.
Its [separate software acceptance](status/2026-10-29-property-run-state-observation.md)
records independent reviews, both 17-stage package gates and isolated real-server
smoke. Hardware acceptance and the broader programming gaps remain open.
The [descriptor-qualified exact-0700 increment](contracts/post-download-run-state-0700.md)
is implemented at `5f075cf99`, with original CLI RED → GREEN and independent
static review complete. Both maintained 15-stage gates and actual isolated
build/smoke passed on `0af68438f`
([software acceptance](status/2026-10-30-descriptor-qualified-0700-run-state.md)).
It preserves 0701/0705 traffic/records and does not promote other layouts,
memory encodings or hardware/function claims.

The [backup application guard](contracts/backup-application-identity.md) records
pre-write evidence and refuses a restore under another application's procedure.
Strict format-2 backups retain unavailable evidence explicitly; legacy complete
backups without it remain readable but require an exact executable identity
precondition or refuse restore. Independent reviews and all 16 selected
integrated package stages passed; published as `08db292f3`
([acceptance](status/2026-10-25-backup-application-identity.md)). This necessary
guard does not close complete mutation effects or whole-device recovery.
The [29 October exemplar-binding increment](contracts/regular-restore-exemplar-binding.md)
adds an independently enforced captured-serial precondition and format 3. It
supersedes the historical legacy-partial eligibility above: every format 1/2
backup remains readable but cannot restore. Independent correction/whole-feature
review, both 17-stage package gates and actual isolated server build/smoke passed
on runtime integration `0ddfe4953`
([software acceptance](status/2026-10-29-regular-restore-exemplar.md)). This does
not close broader target identity, parameter policy or whole-mutation recovery.

The [source-freshness settlement guard](contracts/download-settlement-source-freshness.md)
prevents a successful old plan from crediting current memory after product-source
replacement. It reuses original-request preparation and exact selected-plan
comparison, preserving project-edit settlement and journal recovery. Independent
source review and all 14 maintained integrated package stages passed; published
as `fa4c18213` ([acceptance](status/2026-10-25-download-source-freshness.md)). This snapshot does not monitor later source changes or validate the
device's bytes, unselected parts, runtime or complete recovery.

The [memory-region reader correction](contracts/memory-region-read-service.md)
keeps backup/comparison/probe chunks in the region's selected service and within
its wire count limit. Independent review and all 14 integrated package stages
passed; published as `a4a4ebcf1`
([acceptance](status/2026-10-25-memory-region-reads.md)). This does not add interface
or coupler budget discovery, device-family admission or hardware evidence.

The [load-control response correction](contracts/load-control-response-state.md)
uses a state actually reported before an offline completion interval rather than
forgetting that observation. Independent review and all 14 integrated package
stages passed; published as `e4f5e7c6f`
([acceptance](status/2026-10-25-load-control-response.md)). The existing profile-specific
wait/reconnect and three-read memory mechanisms are not replaced by a common
timeout policy; no hardware acceptance or family promotion follows.

The [memory response pairing guard](contracts/memory-response-service-pairing.md)
rejects another service's response without confirming or falsely reporting a
refusal of the requested operation. Reads, active-Verify writes and subsequent
read-back share the rule. Independent review and all 14 integrated package stages
passed; published as `53400189e`
([acceptance](status/2026-10-25-memory-response-pairing.md)). No physical-device
fault, compatibility expansion, recovery or broader programming completion follows.

## Alpha 16 published pre-release — public artifacts and website verified (24 October 2026)

The [current source-bound evidence matrix](goals/alpha16-release-plan-2026-10-23.md#current-acceptance-disposition--24-october-2026)
records accepted bounded configured System-2 UI/HTTP/App/CLI, catalogue jobs,
comparison adoption, catalogue-authored source portability and caption-language
outcomes. Runtime/tests are frozen at `86b4a7fc68b3d3206c00511b14f41e025e33de4d`.
The final mounted recovery reaches all 105 assertions, including exact original
TestHardware state and once-only refusal across restart; no physical recovery or
ETS parity is established. Historical full gate9 remains bound to `14c9214b`;
focused successors are not relabelled or summed into one full-gate result.

All four final local amd64 artifacts and minimal isolated smokes have parent
acceptance, with independent hash/size readback; [identities and smoke limits](releases/alpha16.md#parent-verified-local-amd64-artifact-identities)
are recorded separately from source acceptance. **Local amd64 software/package
acceptance is complete; Alpha 16 is published at immutable
`f2e3ec6e4b227b49dc74fdba4cdadc3ab91e7c3d`.** Both formerly blocked
pinned-input criteria passed (`ok`/exit 0) at documentation HEAD `a9295a52` using
the owning release-mode integration build after explicitly authorized exact
historical restoration: 111 files (one non-package), 115 instances / 113 unique
payloads, unchanged mapping and original/input/archive/source/user/history guards.
The historical 24/26 receipt stays historical; no overlapping total is created.
The owner explicitly limits local preacceptance to amd64; extra local ARM64
preacceptance is deferred and unverified, not a current blocker. Normal release
channels and the standard amd64+arm64 container workflow remain unchanged; both
remote workflows now succeed, with public asset/OCI identities separately verified. The final standalone artifact now passes the
named public synthetic ETS import/save/switch/reopen and authentic committed
native13 upgrade/save16/switch/reopen routes: 23 checks in one attempt, preserved
topology/retained XML, matching common-column rows across 14 predecessor tables,
SQLite integrity OK and unchanged originals/protected data. This is not undo/redo
operation proof or per-package/browser/hardware/ARM64 acceptance.
Parent documentation review is complete. The explicit 24 October bundled
amendment accepts historical Gate9 plus reviewed affected-successor checks/owning
builds and final artifact smokes instead of a same-final-SHA full-gate replay.
It supersedes the remote dry-run criterion with required actual normal final-tag
publication and public verification; no dry run is claimed. Actual owner-authorized
final-tag publication and public artifact/checksum/digest/revision readback are now
complete ([public verification record](evidence/release-alpha16-2026-10-24.json)).
CI-built downloads and Docker amd64/arm64 revisions match the tag; earlier local
artifact hashes retain their own runtime identity. Final Pages deployment/live
website verification passed at docs/site revision `83376509`: all 40 deployed
files and four aliases matched with no exclusions. Handover and evidence
retention are tracked separately. The bounded documentation reconciliation closes no ledger
row or limitation. General performance and >4 GiB
Large-boundary verification are not claimed. Dated roadmap checkpoints below
retain their original scope; this snapshot supersedes their then-open Alpha-16
integration status without restarting delivered work.

## Alpha 16 implementation authorised — 23 October 2026

The owner adopted the [finite release plan](goals/alpha16-release-plan-2026-10-23.md)
and authorised necessary B10 takeover. The required usable outcomes are configured
experimental System-2 download through the UI, complete catalogue jobs, comparison
adoption, catalogue-created project source portability and source-backed layout
slices. Final review and one integrated candidate acceptance follow implementation;
this is not a release or hardware compatibility claim. Conditional evidence-gated
work and explicit exclusions remain in the plan. Earlier dated next-step prose
below records its checkpoint rather than restarting WP-04.1/.2.

## Offline monitoring handoff (source delivered, 21 October 2026)

The owner authorised a bounded [monitoring snapshot export](contracts/monitor-project-export.md):
one current installation, explicit omissions, a hash-bound report and one local
ZIP through shared App logic with CLI/HTTP/web adapters. Independent design
corrections are admitted by ADR-0151. [Source delivery and internal runtime acceptance](IMPLEMENTATION_STATUS.md#monitoring-snapshot-export--source-delivery)
are complete on `4eee65d8`; separate receiving-tool JSON integration remains
unverified. This does not start a
telegram database, CommunicationLog work, live backend coupling or a device/bus
operation. Existing workstream ordering and ownership remain unchanged.

## UP258 System-2 validation candidate (20 October 2026)

The [complete-defaults simulator path](contracts/up258-system2-validation.md)
now joins source image, ordered procedure, existing executor and durable explicit
same-shape recovery. This narrows the offline goal; it does not complete general
System-2/ETS compatibility or lift physical admission. The remaining open points
are each recorded in the [dispositions](contracts/up258-system2-validation.md#open-point-dispositions-21-october-2026):
the 338 masked allocation bytes have no standard-cited survival rule (the
standard only declares the data invalid on Unload), and the resource/pointer
XML→wire transforms await the master XML schema. A separately approved hardware
adapter/validation still follows. No bus access is implied by this roadmap.

## After Alpha 15 — promotion repair source delivered (20 October 2026)

The [earlier promotion repair](contracts/product-promotion-repair.md) is
delivered as `8fa0d76b9fd41f81c4e135c29e54f2a98d516b38`: product DB v28 corrects the bounded
v23–v25 derived columns and targeted unsupported evidence after WP-17/v27.
[Final acceptance and publication](IMPLEMENTATION_STATUS.md#product-column-promotion-repair--source-delivery)
are source-only, not a new release or Alpha15 artifact change. Authentic
reconstruction, unavailable historical attribution and conservative resource
refusals remain explicit; native/model16 and device admission are unchanged.
The [successor sequence](goals/alpha16-successor-sequence-2026-10-19.md#current-state)
now proceeds to **WP-04.1 measurement only**. WP-04.2 parsing, WP-04.3 web
workflow and WP-19 are still separate later packages; WP-04 remains open.
The release-cutoff sections below retain their original scope.

## Project encrypted-directory integrity delivered (20 October 2026)

The [WP-16 follow-up](contracts/project-encrypted-directory-admission.md) is
published on `main` as `cf5b0fb0`: bit-0 project directories explicitly refuse as
unsupported instead of skipping cipher-header checks. Maintained scoped14/14GREEN,
2,638 Rust passes and149/149selected corpus cases; no new cipher, schema or
release artifact. The bit-0 claim is released. The
[24 October bit-6-only consistency amendment](research/2026-10-24-zip-bit6-flag-consistency.md)
is implemented with original-source regressions and focused corrected tests;
independent review accepts with zero Critical/Important findings; package acceptance
is delivered on `main` as `daef61b74` with the verified integrated 14-stage gate
and exact live publication readback. The claim is released. The
[26 October AES metadata amendment](research/2026-10-26-zip-aes-extra-field-consistency.md)
(delivered as `161e3b14b`, reviewed, integrated 14/14 gate green) refuses contradictory WinZip AES metadata by name in every project and product
layer; consistent AES stays unsupported, so this is no new encryption support.

## After Alpha 15 — master provenance delivered (19 October 2026)

[WP-17](history/OPEN_WORK_2026-10-25.md#wp-17--master-identity-provenance) is delivered on `main`
as `d18ce65e`: product DB v27 master declarations and source-addressable
winners/losers, with unchanged normalized rules and explicitly unavailable
historical encounter facts. Core review, final 11/11 corpus matrix, full-value
migration preservation and both branch/merged 15-stage package gates pass.
This is source-only, not an Alpha 15 artifact upgrade or a new
compatibility/programming claim. Its schema/migration claim is released;
the separately claimed v23–v25 column-promotion repair follows v27, not reuse it.

## Alpha 15 release boundary (19 October 2026)

Prepared from cutoff `21b9d46f`, after the Alpha 15 package set landed. The
[scope note](releases/alpha15.md) includes issues #25/#26, explicit unavailable
catalogue errors, catalogue identity/placement cleanup, the UI feedback waves,
differences-first comparison marks, System labels, complete response-local module
identity, zoom-aware panes, product v25/v26, `SuffixText`, larger measured bounds,
uppercase legacy tables and project ZIP directory integrity. Offline System-2
field preview and the CLI-only declared-memory diagnostic profile are investigation
tools, not programming admission. Native/model16 stays; product24 migrates to26
on writable open. Previous-reader project compatibility is unverified; independent
backups are required.

Still open, not implicitly scheduled for Alpha 16: older v23–v25 promotion repairs,
legacy binary-enumeration diagnostics, inline sections, object folders/priority,
complete source image/procedure/recovery and new hardware evidence. `SuffixText`
and the measured size-ceiling work listed as future work at the Alpha 14 cutoff
are included here; historical sections below retain their original boundary.

## Alpha 14 release boundary (18 October 2026)

Owner instruction: Alpha 14 waits for P1 (object-local function choices) and the product-database series, then is prepared and published. Cutoff `a0c2537d` includes both (plus the Docker frontend-stage fix, the native-spec fold fix, two test-race fixes, ux-feedback and knx-net IGMP, and the v24 product-matrix re-pin found by earlier candidates), the parameter workspace/grids/block access, note decisions before deletes, plug-in data adoption, Functions in comparisons and the read-only diagnostic export. Native/model16 is unchanged; the product database moves 23 → 24. [Release scope](releases/alpha14.md) names the boundaries; census step 5 (size ceilings), `SuffixText`, inline sections, object folders and priority move to Alpha 15. Published on 18 October 2026 at immutable `8d104e2f` ([acceptance](releases/alpha14-acceptance.md), [publication record](evidence/release-alpha14-2026-10-18.json)).

## Alpha 13 release boundary (16 October 2026)

Alpha 13 is published at immutable `6105976f`, covering delivered source through `c9924165` without new feature scope, hardware writes or deployment. [Release scope](releases/alpha13.md) documents unchanged schemas and remaining source/procedure/recovery and module-instance blockers. All 59 ordered source stages and public artifact/registry/lifecycle/site checks pass; [publication evidence](evidence/release-alpha13-2026-10-16.json) keeps their boundaries distinct.

## Exact UP258 outcome takes priority (16 October 2026)

For issue #22, complete `M-0001_A-9205-04-9270` image → all 21 source
positions → production execution → whole-mutation recovery before unrelated
family expansion. Current [No-Go decision](research/2026-10-16-up258-production-decision.md)
names external source contracts still needed; one [prepared request](history/REMOVED_DOCS.md#reporter-request-removal-23-october-2026)
separates firmware, bounded comparison, each device operation and publication.
No physical operation, new supported mask or successful programming is claimed.

## Offline-first download expansion (15 October 2026)

A new explicit continuation after the P6 session closure starts a bounded
[System-300 S-mode property-table prerequisite](contracts/offline-system300-property-tables.md).
Published as `af9431cb` after19actual merged stages, including isolated corpus152/152;
earlier branch generated-whitespace reconciliation remains separate. It reuses Type6
bytes, not a SystemB family admission. Discovery/source mapping, property-service
framing, complete loading and whole-mutation recovery remain separate next slices.

Owner authorised implementation without owning the target devices; later field
validation remains separate. [The finite expansion contract](contracts/offline-download-expansion.md)
starts with System2 codecs/metadata, then complete image/planning, existing
SystemB integration, simulation and whole-mutation recovery where source evidence
permits. [P1 codec contract](contracts/offline-download-codecs.md) records delivered
foundations; [P2 source-image contract](contracts/offline-download-system2-image.md)
separates implemented synthetic/source grammar from the real Siemens encoding
refusal. [P3a normalized property-plan simulation](contracts/offline-property-plan-simulation.md)
has been integrated and published as `7632fb1a`; fifteen-stage branch and
sixteen-stage merged acceptance pass, including isolated corpus152/152. It
is not yet a source translator. Source-to-property mapping, restart and durable
property+memory+setup/allocator recovery remain next where exact contracts permit.
Hardware admission and broader family completion are not implied by these foundations. Unknown semantics remain explicit research
blockers, not guessed simulator behaviour.

P4 now separates the [pure System-B Format-1 object codec](contracts/offline-systemb-object-table.md)
from relative-image/sequencer integration. Its local synthetic acceptance and the
three real partial resolver results do not close product mapping or recovery;
all 17 branch and 18 actual merged stages pass, including isolated corpus152/152.
Published as `adb52990` with verified remote equality; the selected dirty root
checkout remains deliberately unsynchronized.

P4b publishes a [relative source view](contracts/offline-relative-source-segments.md)
and prevents absolute images from silently omitting relative input. The three
source-oracle matches are extraction evidence, not completed relative images;
18branch and19actual merged stages pass, including isolated corpus152/152.
Published as `4bfc3780` with verified remote equality. The relative-image and
recovery integration work above remains open.

P4c, published as `c74038cd`, supplies [explicit System-B link tables](contracts/offline-systemb-link-tables.md)
with a distinct wide-index grammar and first-match sender checks. These bytes
are not a generated real-product image or source-to-sequencer integration;
branch/actual merged acceptance passed; the above semantic contracts remain open.

P4d adds [explicit complete object-table construction](contracts/offline-systemb-object-table.md#p4d-explicit-complete-construction-16-october-2026)
without needing a retained table template. Eighteen branch and nineteen actual
merged stages pass, including isolated corpus152/152; published as `894f14e0`
with exact remote equality verified. Source-to-ASAP mapping, generated table placement,
relative parameter images, procedure translation and recovery remain open.

P5 delivers a [bounded System-1 source-image entry](contracts/offline-download-system1-image.md),
not complete real-product support: thirteen final real-source refusals remain
explicit. Next: source-backed FixupList interpretation and discriminating
parameter/Union evidence; only then complete source images and separate default
procedure/recovery contracts. All nineteen workspace branch and twenty actual
merged stages pass, including isolated corpus152/152; published as `ccfe5b74`
with exact main equality read back.

P6, published as `39c88aba`, [closes sibling FixupList omission](contracts/source-fixup-integrity.md)
across existing image consumers. Two named REDs, owning928tests and Clippy pass;
all nineteen branch and twenty final merged stages pass, including isolated
corpus152/152; publication/readback verified. This session closes without starting
a further package.
Authoritative source transformation/no-op semantics
and complete-image/procedure/recovery integration remain next, not waived by
matching source/master words.

## Alpha 12 release boundary (15 October 2026)

The owner requested a release of delivered source, not new feature work. Both P4
packages are integrated; the release candidate includes Functions, portable native
product sources and universal reporting. Media and current reader routes are
refreshed without changing the approved Story. Full exact-candidate gates,
AppImage/Docker no-publish workflow builds and final tag/publication approval
remain distinct. [Release scope](releases/alpha12.md) retains all hardware and
format limits; no new commissioning support or ETA is promised.


## Owner UI feedback P4 (2026-10-15)

P3 and both P4 packages are accepted in source: bound single/filtered comparisons,
session tags, verified identity, decoded address/association tables and named
parameters from actual bit attribution. Combined full51/51 and isolated corpus
152/152+16/16 pass; [publication record](evidence/p4-device-comparison-parameters-2026-10-15.json)
tracks remote delivery separately. No new device support, hardware action or
release is implied.

## Universal device-support intake (15 October 2026 source work)

[Structured offline/optional read-only reporting](contracts/universal-device-support.md)
addresses evidence intake for unknown/unresolved/refused devices, not their
programming implementation. Unknown families, missing licensed source rights,
complete image/table/LSM contracts, durable recovery and actual target-specific
hardware verification remain explicit gates. Collection and manual handoff do
not close issue22, promise support dates or authorize later bus writes.

## Small offline literal naming source slice (2026-10-12)

[Explicit group-address prefix/suffix naming](contracts/offline-name-recipe.md)
uses a private validated application plan, one Core Batch/undo step and confirmed
native working-history persistence, not an automation interpreter. No generated
selection, UI/MCP writer, root Save, bus capability or release follows. Broader
macros and the separately approved AP6/AP7 remain distinct later work; final source
assembly18 and integrated documentation8 stages have passed; source publication/readback
on main `f70183f0` is verified in the [evidence](evidence/offline-name-recipe-2026-10-12.json).


## GitHub release notification source feature (2026-10-11)

Startup/hourly notification, explicit manual checking and persisted opt-out are
implemented at the [bounded notification-only scope](contracts/RELEASE_NOTIFICATIONS.md).
This is not an automatic installer, artifact verifier, new release or running
container replacement. Native interactive verification and any future updater
remain separate tasks; no KNX compatibility or schema scope expands.

## Local source-integrity repair scope (2026-10-10)

The [import integrity contract](IMPORT_INTEGRITY.md) covers the bounded parser/provenance/reporting repairs and scalar completion vocabulary. New device-extension modelling and execution/planner support remain separate research/owner decisions, not automatic follow-ons. Source integration, release and hardware scope are not authorized by local completion.

## Import expansion source integration, not a release (2026-10-10)

Selected devices/lines, a private authorized-export regression harness and a
read-only cvexc analyzer have a source implementation, historical
[local verification](status/2026-10-10-import-expansion-verification.md) and separate
[integration verification](status/2026-10-10-import-expansion-integration.md).
The owner subsequently authorized commit/merge/push on 10 October 2026; no release
or bus action was added. Remaining evidence:
authorized historical ETS exports with reviewed baselines; future-model closure
when the normalized model changes; primary cvexc semantics before any runtime
policy. This does not reopen commissioning or authorize real-bus actions.
Formal per-ID dispositions remain only in [LEDGER](status/LEDGER.md).

**Open work:** everything not yet done, deferred or later scope is listed in
[OPEN_WORK](OPEN_WORK.md) (baseline audit 2026-10-08, kept current by dated
sections). The three alpha goals are
finished; they were removed from the tree on 2026-10-08
([where to find them](history/REMOVED_DOCS.md)).

## Current reading guide — 8 October 2026

Checked against source `608a204b`. The [ideas/roadmap audit](status/2026-10-08-ideas-roadmap-audit.md)
maps all twelve original ideas to actual callers, tests and remaining scope.
Read the sections below as a delivery record, not a fresh implementation queue.

| Already usable at its documented scope | Still absent or separately bounded |
| --- | --- |
| Read-only MCP, Flow and secondary window, dashboard, theme packs/LCARS, motion and humour | Agent mutation, macros, project notes, dashboard drill-down, mobile/native Windows/macOS apps |
| Project/device wizards, autosave, browser project export, parameter tabs, achievements and language packs; persistent native undo/project versions in the local 2026-10-09 package | Independent disaster-backup copies, specialized parameter widgets, version pinning/selection |
| Legacy VD web/CLI offline import with password handling; commissioning activity/history UI | Oversized VD5 coverage, legacy download, complete long-session journalling and device-specific recovery |
| Public alpha.5, repository and website/story publication | A new release is a separate decision; native/accessibility/real-hardware coverage does not expand with publication |

Per-ID dispositions remain only in the [ledger](status/LEDGER.md). An accepted
alpha limitation is not functionality, and a later idea is not a promised date.

## Communication-object table (2026-10-09 source package)

The owner-approved single-device table adds labeled columns, channel/flat views,
search, evaluated-status/link/effective-DPT filters and stable sorting. Open editors
keep drafts/refusals/pending state across presentation changes; no domain/storage/
API/protocol changes. [Contract](COMMUNICATION_OBJECT_TABLE.md), ADR-0102 and its
receipt define the scoped acceptance. Main integration/publication was subsequently
authorized; release/deployment remain separate. This is not in alpha.6. Project-wide CO lists, bulk
editing and persistent view preferences remain outside this package.

## Offline procedure evidence (local source package, 2026-10-09)

The owner-approved first `MV-07B0` `Load/ap1` slice reconstructs declarative
order with local source provenance and explicit unresolved data, through the
existing support-gap UI/API/CLI. Reduced evidence can be manually shared with
the maintainer. [Contract](OFFLINE_PROCEDURE_RESOLUTION.md), ADR-0098.
Main source publication is [verified](status/2026-10-09-offline-ap1-verification.md); packaged release remains separate. Complete plan materialization, conditional
selection, AP2/other masks, legacy/coupler procedures, new media and new live
execution remain unimplemented; this does not close commissioning ledger rows.

## Project-local names (2026-10-09 source package)

Single device instances and individual group addresses can be renamed through
Name fields, F2 and context menus in every installation. Exact reversible names
use native history; guarded requests and retained drafts preserve conflicts.
[Contract](contracts/project-name-editing.md) and
[verification](status/2026-10-09-project-name-editing-verification.md).
Bulk templates, CLI/MCP authoring, `.knxproj` export and hardware effects remain
outside this package. Publication is authorized separately from release/deployment.


## Native project history (local source package, 2026-10-09)

Restart-safe native undo/redo, named/save/pre-restore versions and revision-bound
confirmed restore are implemented in the owned source package
([contract](PROJECT_HISTORY.md), ADR-0100). Admission is bounded and atomic; retained
context is shared without losing opaque data. Before first Save As history is
session-local; same-file versions are not independent disaster backups. Older
builds refuse native v11. Publication, deployment and release remain separate.


## Devices navigation (local source package, 2026-10-08)

The owner-approved project-wide device list, central editor/Back and scoped
links are implemented locally; [contract](DEVICE_NAVIGATION.md), ADR-0096.
Publication and deployment are separate. URL deep links/browser history,
persistent view state, grouping, free-text log parsing and project-diff links
remain later scope. No new commissioning/ETS/hardware support follows.

## Community evidence (2026-10-08)

Own-instance read-only analysis, exact disclosure previews and versioned ZIP
export via UI, API and CLI are in the repository; the support-gap issue form
and EN/DE guides live in this (now public) repository. Attaching and posting
stay the user's own steps; there is no central upload service. The private
mailbox, its handling/retention terms and confirmed delivery are unverified;
originals never go into public issues. New adapter support still needs
validated evidence, tests and review. See [COMMUNITY_EVIDENCE.md](COMMUNITY_EVIDENCE.md).

## Marketing website (2026-10-08)

The calm-dark DE/EN marketing companion is live at knxbench.com (GitHub
Pages, deployed by `.github/workflows/pages.yml` from a release build):
real-app clips, optional CRT/LCARS gallery, the published Evolution Story and
the Docker/AppImage start paths; the full manual stays on GitHub. See
[website contract](WEBSITE.md) and ADR-0095.

## Modern LCARS theme (2026-10-08)

The visually approved study is integrated as an optional built-in dark theme
with controlled workbench geometry, finite navigation feedback and independent
motion/density. [ADR-0092](adr/0092-lcars-built-in-presentation.md) and the
[LCARS guide](DESIGN_LCARS.md) record its verified local scope. Native
WebKitGTK/Orca and complete accessibility validation remain separate; light
mode, sound, arbitrary CSS imports and workflow redesign are outside this
package. The UI change itself did not authorize deployment; the user separately
authorized commit, push and server deployment on 2026-10-08. No release tag.

## Achievements (2026-10-07)

[ADR-0089](adr/0089-achievements.md): achievements in the UI, with a
catalogue in the frontend and a record on the server that only grows.
Both packages are implemented: all 38 achievements from the interview
catalogue. Three triggers were adapted because KNXBench cannot detect them
as worded (ADR-0089). Any new bus or commissioning achievement must reward
a *verified* result, never a volume of writes.

## DPT document inventory and scoped hardening (2026-10-07)

The [DPT document review](spec-audits/2026-10-07-dpt-document-audit.md) supplies
a 454-ID/280-section/251-page inventory, numeric edge fixes and bounded
parameter-only runtime admission. The implemented main-family scope stays
1–30; subtype/FB semantics and structured DPTs remain disclosed boundaries.
Future expansion requires measured installed-device relevance and authoritative
subtype/application evidence, not a blanket LTE classification or an inferred
release blocker. No release or hardware authorization follows.

## Post-Alpha Flow usability follow-up (2026-10-07)

Implemented on the user's explicit go: larger/maximized area, closable Inspector,
dedicated source-bound Flow window, stronger readability-first rearrangement,
project links and switchable auto zoom. See [ADR-0085](adr/0085-readable-flow-and-source-windows.md)
and [Flow follow-up](TELEGRAM_FLOW_VISUALIZATION.md#23-readable-area-source-windows-and-project-links-2026-10-07).
Browser/offline acceptance is recorded there; native WebKitGTK/screen-reader
validation and a new packaged release remain separate. This does not move the
existing Alpha tag or authorize house-bus operations.

## Post-alpha packaging follow-up (2026-10-07)

The user requested the removal of the forced-X11 AppImage launcher boundary.
The [launcher contract](APPIMAGE_LAUNCHER.md) and
[ADR-0021 amendment](adr/0021-appimage-is-the-first-linux-package.md#2026-10-07-amendment-owned-display-policy-kl-158)
keep this at the packaging layer: owned backend selection, caller overrides,
pinned GTK deploy source, X11/native-Wayland verification. Publication state is
in [the ledger](status/LEDGER.md); the existing alpha.4 tag and asset remain
unchanged. No ETS, GPU portability, commissioning or broader native-UI claim
follows from this launcher work.

## Cross-cutting — Telegram-flow nervous system (Alpha addition, 2026-10-04)

Delivered at the evidenced scope described below; broader platform validation
remains separate. The following records the original contract and delivery sequence.
The [contract/research](TELEGRAM_FLOW_VISUALIZATION.md) and
[ADR-0077](adr/0077-session-local-telegram-flow-view.md) define a session-local,
read-only dynamic communication map: directional/group-labelled pulses, immediate
per-group values with 7-second expiry, an observed-sender activity center,
freezable layout and resting edges that never silently vanish. Theme/motion
preferences apply; inferred endpoints stay visibly distinct from receipt proof.
Execution order: UI U19 → alpha AR20 → UI U20/U21 → alpha AR21, then finished
AR15–AR18 acceptance. That original alpha closeout and publication are complete;
another release still needs its own exact decision.
No new commissioning work, floor-plan editor, permanent coordinates or traffic
history. Existing historical milestones and owner work are not reopened.
**Done 2026-10-05:** U19–U21 and AR20 delivered, AR21 accepted the feature for
the Alpha on a recorded envelope (Motion Off for large maps; Chromium only);
see [TELEGRAM_FLOW_VISUALIZATION §22](TELEGRAM_FLOW_VISUALIZATION.md#22-ar21-rerun-of-findings-6-and-7-and-acceptance-alpha-2026-10-05).


## Integrated CRT animation follow-up (2026-10-03)

Application-owned CRT feedback is implemented separately from the completed
declarative theme-pack delivery: real tree/address-table fills, bounded light
and manual Save activation. Existing keyboard/selection paths and the v1 palette
contract are retained. See [CRT guide](DESIGN_RETRO_GREEN_CRT.md) and ADR-0022.
Authorized main integration is3d03aea5 (feature16c9d774) with repeated merged-result
gates; current refs/handover establish publication. Native WebKitGTK/Orca evidence,
Save-only semantic color roles and broader component redesign remain separate; this is not a
reopening of the completed theme-pack format milestone or Alpha/ETS acceptance.

The original Session 0–7 delivery sequence is complete at its evidenced
scope: Sessions 0–6 shipped, and Session 7 includes a first, safety-gated
`070nh` download and a historically verified button-address path on one MDT
device (`1.1.67`). Confirmed public K6/serial/K13 address writes currently
fail closed before any tunnel until action- and device-specific durable
pre-write recovery exists (ADRs 0057–0059). This is **not** full ETS,
manufacturer or KNX hardware coverage.
The following goal links are **historical**, not current work queues. Remaining
work is in [OPEN_WORK](OPEN_WORK.md); the old v1/UI/commissioning decisions are recorded in
[`goal.md`](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/goal.md), [`goal-ui.md`](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/alpha-0.1/goal-ui.md) and
[`goal-commission.md`](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/alpha-0.1/goal-commission.md). Historical milestone detail
remains in [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).

## Session 0 — Technical research

Done. Sources and format boundaries: [RESEARCH](RESEARCH.md).

## Session 1 — Architecture

Done. Layering, domain contracts, migrations and ADRs are in
[ARCHITECTURE](ARCHITECTURE.md) and [DATA_MODEL](DATA_MODEL.md). The historical
`.knxproj` export contract was withdrawn (ADR-0028).

## Session 2 — KNX core

Done. Typed entities and addresses, validation, command/undo model and
versioned storage shipped. ADR-0039 phases 3–5 still leave a public mutation
invariant to be enforced (KNOWN_LIMITATIONS §129).

## Session 3 — ETS project import

Done at the evidenced schema-11/21/23 import boundaries, with reports,
retained opaque data, tests and native `.knxdb` persistence. `.knxproj`
export is withdrawn (ADR-0028). An independent module-using schema-23
project and projects with schemas 12–14, 20 and 22 are still missing;
this is an evidence limit, not an importer success claim (limitations §1).
See [COMPATIBILITY](COMPATIBILITY.md) and [IMPORT_EXPORT](IMPORT_EXPORT.md).

## Session 4 — Manufacturer databases

Done for the implemented versioned product database and the evidenced
`.knxprod` master-data schemas 11, 12, 13, 14, 20 and exact-namespace 21
(since 2026-10-04/05 also exact 23 and exact 10, ADR-0072/ADR-0083).
PDB-1–PDB-11 landed; source blobs and unknowns are retained/reported, not
claimed to be fully interpreted. Legacy `.vd`/`.pr` uses a separate path
(ADR-0094): inspection and CLI offline import landed, followed by L3 web
upload, password dialog and one remembered password on 2026-10-08. Real
VD3/VD4 evidence does not prove the oversized VD5 case; resource measurement
and legacy download (L4) remain separate;
unknown vendor semantics and encrypted packages stay refused. Evidence:
[PRODUCT_DATABASE_CORPUS](PRODUCT_DATABASE_CORPUS.md),
[COMPATIBILITY](COMPATIBILITY.md), [KNOWN_LIMITATIONS](KNOWN_LIMITATIONS.md).

## Session 5 — UI and UX

Tauri/WebKitGTK shell and one React Web UI over the HTTP API shipped with
project explorer, inspector, search, command palette, save/open, Settings,
structure editing, product catalog, diagnostics and scoped device actions.
U0–U12 UI work is delivered within its tested scope: read-only readiness
and device comparison, Site/Property creation, and a default-off Debug
property action with an offline-tested, property-only backup (ADR-0051).
K6's Web tab shows fail-closed availability rather than a working write.
[`goal-ui.md`](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/alpha-0.1/goal-ui.md) is complete through U13 (2026-10-01).
The operator accepted the independent GPT-6.1-Sol review instead of unavailable
Claude; all three P1 findings are fixed and mutation/regression-tested.
ISSUE-12's two evidence boxes reconcile the host-firewall correction and actual
offline unicast-loopback transport tests, without claiming multicast proof.
Final integrated Rust/Web/Chromium/repository gates passed; see the U13 log
and [IMPLEMENTATION_STATUS](IMPLEMENTATION_STATUS.md).
Native WebKitGTK and screen-reader coverage remain bounded (§20, §130).

## Cross-cutting — Theme packs

U14 resolves the declarative, versioned theme-pack contract and security/
negative-fixture matrix in [THEME_PACKS](THEME_PACKS.md) and
[ADR-0060](adr/0060-versioned-declarative-theme-packs.md). This is contract
research, not shipped import/export. U15 runtime admission, cache revalidation
and reversible theme application are delivered as 9d1ae19d with complete
candidate/combined gates and exact remote readback. U16 delivers acknowledged
storage/file roundtrip as 1f94808d (144 focused passes, restored
controls, complete 16-check candidate and 22-check actual-merged gates), with
exact source publication/artifact readback verified. U17 management/preview
passes actual-merged23-command acceptance on f16f1e40: Web1702, Chromium72,
Rust2984/0/165,27 selected private offline cases plus the115-instance matrix,
17 equal bindings and697 source/config inputs/420 private files unchanged.
Publication/readback is recorded in the current handover. Eleven manager browser
flows and31 actual parent/five root cases cover the management contract.
U18 in [goal-ui](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/alpha-0.1/goal-ui.md) passes actual24-command extension acceptance
on1660911b: Web1702, Chromium82, representative10/10, Rust2995/0/165,28
offline cases including the115-instance matrix;703 source/config inputs and421
private files unchanged. Closing self-review settles U18-R1 with real component
states and detected rendered-style controls, not native/Orca/general WCAG proof.
Publication/readback and owned cleanup are tracked in the current handover.
Existing built-ins/System stay
available; no arbitrary CSS, external assets or new alpha release blocker.

## Cross-cutting — Internationalization

English/German UI chrome and importable language packs shipped (T25).
Product-data translation is available at evidenced surfaces, not universally;
project default language, master-scope translation readers and remaining
server prose are bounded by §14, §37, §64 and §66. Do not call this full
localization. See [LANGUAGE_PACKS](LANGUAGE_PACKS.md).

## Cross-cutting — Motion and animation

T27 shipped user-selectable level and Smooth/Glitch/CRT styles. The OS
`prefers-reduced-motion` setting wins, and a style guard covers the
`styles.css` transitions it can measure. Per-category controls for Toasts,
Achievements and Flow followed on 2026-10-13 (ADR-0122); other CSS surfaces are
not claimed (KNOWN_LIMITATIONS §43).

## Cross-cutting — Web/Docker deployment target

One frontend/API serves desktop and Docker. The server now authenticates
LAN clients with one password or refuses non-loopback without one
(ADR-0026); it is **not** an unauthenticated-LAN design. There is no
per-user authorization or concurrent project editing (limitations §22/§63).
Docker's bridge cannot carry discovery multicast by default; host-network
permissions are a deployment decision (limitations §79).

## Cross-cutting — LLM / natural-language interaction

Research complete (RESEARCH §13). **Delivered 2026-10-07 (ADR-0090):** the
narrower read-only scope §13.6 allows — `apps/knx-mcp`, a stdio MCP server
over saved project files (eight read tools, CSV proposals validated but
never applied), plus an agent skill. Since 2026-10-08 it reports parameter
visibility with the parameter panel's own evaluation, now shared in
`knx_productdb::device_evaluation`. No in-app chatbox. A mutation-capable
LLM or MCP surface is still not scheduled and still requires a mature
command/authorization/revision-bound audit contract. Bus and device writes
remain excluded.

## Cross-cutting — Repetitive-task automation

Research complete (RESEARCH §14); no generic macro/script API is scheduled.
A future operation template needs a deterministic revision-bound preview
and one atomic, undoable command batch.

## Cross-cutting — KNX `Functions` domain concept

Schema-23 prerequisite researched (RESEARCH §15), but project entity,
import, storage and UI remain deferred pending a dedicated ADR and samples
for older project schemas. Do not confuse master `FunctionType` data with
project Function instances.

## Cross-cutting — Third-party extension and plugins

No plugin API is planned (ADR-0025). Supported extension surfaces remain
product data, language packs, CSV and the CLI. A code plugin host needs a
proven second implementation and a safe authorization boundary, not a
speculative interface (KNOWN_LIMITATIONS §107).

## Session 6 — KNXnet/IP

Discovery, tunnelling, routing, monitor and scoped group communication
shipped. On this host the gateway's unicast reply was blocked by `ufw`;
after the user's UDP source-port 3671 rule, CLI and HTTP discovery both
found it (RESEARCH §20.1, limitations §79). A native WebKitGTK **Search**
click remains unobserved. Docker bridge multicast remains a separate
deployment boundary. KNX IP Secure is out of scope (§26). Bus traffic and
live-device support are not implied by simulator tests.

## Session 7 — Integration and hardening

Roundtrip/migration suites, large-project performance measurements and the
first x86_64 Linux AppImage are delivered at their tested boundaries.
Commissioning has a safety-gated `070nh` memory download and historical
individual-address evidence on MDT `1.1.67` (ADR-0048; RESEARCH §19).
Public confirmed address writes are now pre-tunnel refused pending verified
durable recovery (ADRs 0057–0059); no prior go transfers to another device.
Other masks, device families and RF hardware remain refused or simulator-only.
The application-download backup is not a universal rollback (ADR-0049).
ADR-0064 adds a separate durable activity-metadata backend and bounded history
API. The Web Live/History consumers are implemented in the Bus workspace;
they disclose partial/untracked coverage. Complete long-session journalling,
universal device recovery and persistent project undo are not supplied. See
[COMMISSIONING_ALPHA_LEDGER](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/alpha-0.1/COMMISSIONING_ALPHA_LEDGER.md)
for all 42 commissioning-routed source IDs, safe fallbacks and exact unblocks.
The historical alpha closeout, final review and public alpha.5 release are
delivered; see [historical final gates](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3717efdfcdccb46fc19bed54c3bab/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md)
and the ledger.
This closes the recorded alpha scope, not every v1 ambition or every KNX
device. Further release decisions and broader evidence remain separate.

## Cross-cutting — In-application help and user documentation

T28 shipped translated in-app tips and F1 help. A separate
[user manual](manual/README.md) exists with stable chapter navigation and
source-checked workflows. The local documentation refresh adds real captures
and automated documentation checks; its commit/publication is still pending,
not an application feature. Broader native capture/accessibility evidence
remains separate. In-app *project* notes are another, unimplemented feature;
their domain shape is already decided in ADR-0031 (F1 help: ADR-0024).

## Open questions and where they land

- Independent `.knxproj` samples (especially module-using schema 23) and an
  AES-protected ETS6 sample are prerequisites for broader claims (§1/§13).
- The public alpha.5 is a separate build snapshot; newer source features need
  a new gated build and release decision. The documentation refresh remains
  local until reviewed and published.
- KNX Secure, legacy download/oversized VD5, `Functions`, multi-user support
  and RF hardware need their own evidence/scope decision. Legacy offline
  import is already delivered. See the historical `goal.md` §6 and
  `goal-commission.md` §3c; none is silently scheduled here.
