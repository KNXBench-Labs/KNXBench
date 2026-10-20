# Open work after the 0.1 alpha

## Reconciled work packages — 18 October 2026

This is the **current planning entry point**. The dated sections below retain
context and delivery history; they are not additional jobs to execute. Planning
here neither reopens an accepted limitation nor authorises implementation,
bus access, desktop control, deployment, a release or third-party contact.

**Evidence cutoff:** `961502d6` on `main`, fetched on 18 October 2026.
The audit inventories all **195 catalogue entries** (including unnumbered
boundaries) and **208 ledger IDs**. The ledger has 67 completed entries,
124 accepted boundaries, 15 later-scope entries and 2 external blockers;
those are unequal records, not a completion percentage or 141 coding tasks.
Per-ID status remains solely in [LEDGER](status/LEDGER.md).
[Audit routing](evidence/open-work-packages-2026-10-18.json) accounts for every
catalogue entry and routes every non-terminal ledger ID once. This is a
repository-evidence audit with targeted source checks, not a fresh KNX/spec,
security, GitHub-issue, hardware or whole-codebase conformance audit.

### Already owned — do not start a duplicate package

- **WP-17 is delivered** on `main` (`d18ce65e`, 19 October): product DB v27
  master provenance; its parsing/provenance/report/migration claim is released.
  The separately claimed Alpha 16 column-promotion repair must follow v27,
  not reuse it. Historical winner/encounter availability remains bounded below.
- **UI feedback:** UI-01–UI-06 and the whole-set corrections are delivered
  ([UI workstream](goals/ui-feedback-2026-10-18.md)); the claim is released.
  Two follow-ups remain unowned (below, under *UI feedback packages*). Its
  package labels are scoped to that goal, not the historical `UI-*` ledger IDs.
- **Exact UP 258D12 / issue #22:** the
  [offline programming goal](goals/up258-programming-2026-10-18.md) owns
  explicit source-field preparation, image/procedure/execution and full-state
  recovery work for its named application. The first diagnostic export from
  a real detector has arrived; this does **not** lift download admission.
  Shared probe/commissioning changes require coordination with that owner.
- **Delivered (19 October 2026):** issues #25/#26 are on `main`: the
  topology diagram hides device-empty ranges (display only) and supported
  parameter-section captions resolve through the existing text resolver with
  readable fallbacks. Remaining caption boundaries (source-language block and
  separator captions, unresolved grid-axis references) are recorded in
  [positional text templates](contracts/positional-text-templates.md); they are
  open WP-05 scope, not claimed by anyone. The pinned audit's
  product-capability evidence is unchanged.
- The [17 October P1–P5](goals/open-work-2026-10-17/README.md) are delivered;
  P6–P8 are waiting/decision buckets, not an implementation queue. Do not
  rescue the object-function feature or rebuild plug-in adoption, note
  disposition, Functions comparison or gate-hygiene fixes again.

### Release-engineering follow-up — Alpha 16

The first Alpha 15 AppImage dry run lacked the optional native Rolldown binding
after `npm ci`; the exact-source successor passed. Why npm omitted the binding
is unestablished. The workflow now imports the installed `rolldown` immediately
after installation, before the expensive Tauri build; a load failure stops the
job. Regression tests execute that exact command against the installed loader
and isolated copies with missing or corrupt native libraries. They also retain
the manual-build/tag-only-publication boundary. This is a bounded build
preflight, not a repair of npm installation, a new AppImage acceptance run or
a reason to move the Alpha 15 tag. **Delivered 20 October 2026** on main-first
merge `f9cf7a65`: independent review accepted; maintained scoped gate 13/13,
frozen inputs, user-data guards clear. Native-binding claim released. See the
[retained attempts](releases/alpha15-acceptance.md#retained-non-green-attempts).

### Recommended package queue

Priority here orders the *remaining work*, not the frozen ledger priorities.
**P1** = integrity/correctness first; **P2** = bounded capability/usability;
**P3** = validation or documentation follow-up. “Offline-ready” means no real
bus/device access is needed; it does not replace the claim/implementation go.
Each package needs its own scoped claim, review, tests and delivery evidence.

| Package | Priority | Readiness / dependency | Bounded outcome |
| --- | --- | --- | --- |
| [WP-01](#wp-01--idempotent-unknown-evidence) | P1 | Delivered 19 October (product DB v26, ADR-0146) | Unknown-evidence rows remain truthful across shared-source re-ingest. |
| [WP-02](#wp-02--suffixtext-end-to-end) | P2 | Delivered 19 October (product DB v26, ADR-0146) | Source suffix text reaches the existing parameter rows without changing stored values. |
| [WP-03](#wp-03--legacy-binary-enumeration-values) | P2 | Mapping evidence first | Legacy binary enum values use the existing typed storage or keep an explicit refusal/report. |
| [WP-04](#wp-04--large-catalogues-measure-then-stream) | P1/P2 | Measurement first; three sequential slices | Measured database growth, bounded procedure parsing, then a cancellable web large-profile workflow. |
| [WP-05](#wp-05--parameter-layout-fidelity) | P2 | Evidence/contract gate; coordinate with UI-05 | One evidenced layout feature at a time, never inferred manufacturer behaviour. |
| [WP-06](#wp-06--communication-object-organisation) | P2 | Folder contract first; priority needs a separate model ADR | Declared object folders and, separately, lossless priority representation. |
| [WP-07](#wp-07--module-instance-authority) | P1 | Defining mapping evidence required | One independently witnessed instance/repeat/number-offset mapping contract. |
| [WP-08](#wp-08--parameter-value-codecs) | P2 | Per-type encoding evidence required | A separately admitted codec/editor per TypeDate, Colour or Picture form. |
| [WP-09](#wp-09--offline-image-evidence-gates) | P1 | Research-bound; exact UP258 remains owned | Discriminating encoding/fill/placement evidence, not another permissive image builder. |
| [WP-10](#wp-10--import-and-model-evidence) | P2 | Samples/grammar/authorisation required | Independently verified import/model cases with honest unsupported-data reports. |
| [WP-11](#wp-11--native-linux-acceptance) | P2 | Separate desktop-control permission | Actual WebKitGTK/Wayland/Orca and AppImage evidence for named workflows. |
| [WP-12](#wp-12--field-evidence-and-owner-acceptance) | P2 | Outside input or operation-specific consent | Reviewed tester/plug-in/diagnostic evidence, without silently expanding compatibility. |
| [WP-13](#wp-13--documentation-drift) | P3 | First slice 19 Oct; limitation-summary reconciliation 20 Oct; wider sweep open | Current summaries agree with delivered behaviour; retained history stays intact. |
| [WP-14](#wp-14--complete-offline-family-outcomes) | P1 | Source contracts first; UP258 excluded | Complete real-source System 1/B/300 image → procedure → execution → recovery slices. |
| [WP-15](#wp-15--complete-module-scope-in-web-consumers) | P1 | Delivered 19 Oct (web adoption); older-server boundary documented | Complete response-local module paths distinguish diagnostics and workspace identities. |
| [WP-16](#wp-16--empty-directory-zip-integrity) | P1 | Delivered 19 Oct; trailing-stream and product-reader follow-ups open | Empty directories have valid CRC and complete compressed-stream representation. |
| [WP-17](#wp-17--master-identity-provenance) | P1 | Delivered 19 Oct; product DB v27, ADR-0147, `d18ce65e` | Master identity winners, collisions and malformed declarations have truthful provenance. |
| [WP-18](#wp-18--device-comparison-shows-what-differs) | P2 | Delivered 19 Oct; no live-device acceptance | Differing bytes are marked in the raw hex rows; the comparison result shows only differences by default, with a toggle for all rows. |
| [WP-19](#wp-19--sync-from-a-device-comparison) | P2 | Contract first; project → device needs bus-write consent (owner request, 19 Oct) | A comparison result offers *project → device* (the existing download) or *device → project* (adopt verified values, one undo step). |
| [WP-20](#wp-20--system-family-in-the-device-list) | P2 | Delivered 19 Oct; a mask read from a device stays a separate session fact | The device list shows each device's system family (System 1, 2, 7 or B) from its mask version. |

#### WP-01 — Idempotent unknown evidence

**Delivered 19 October 2026** ([ADR-0146](adr/0146-unknown-evidence-rows-have-an-identity.md),
product DB v26). Cause, reproduced with synthetic packages: every package
install re-parsed the members it shares with an earlier package (and the
master) and wrote their rows again; a re-parsed losing blob repeated its
`IdConflict` row. A refused install, its retry, a repeat install and a reopen
did not inflate on their own. A row is now a fact about one blob's bytes,
enforced by two unique indexes; `occurrences` is one parse's count. The v26
repair keeps each group's oldest row with the largest count and the oldest
sample; package reports are untouched. On the 115-instance matrix corpus the
re-pin removes exactly the 849 predicted copies (`ingest_unknown` 23,176 →
22,327), and a migrated v25 shared database equals a fresh v26 install.
**Remaining boundary:** a later parse's *different* sample for the same
construct is not stored separately (none measured; the bytes stay retained).
[Finding](#product-database-census-3-18-october-2026) ·
[report implementation](../crates/knx-productdb/src/report.rs).

#### WP-02 — SuffixText end to end

**Delivered 19 October 2026** in the same product DB v26
([ADR-0146, part 2](adr/0146-unknown-evidence-rows-have-an-identity.md#part-2-suffixtext-reaches-the-parameter-row-wp-02)).
`Suffix` and `SuffixText` are separate columns on `parameter`; `parameter_ref`
stores its own `SuffixText`. The ref's declaration wins over the parameter's
(even when empty), each layer translated like `Text`; the field DTO carries
`suffixText`/`suffixTextLanguage` and the web row shows a non-empty suffix after
the value. Values, drafts, writes and write authority are unchanged. The
`SuffixText` report rows retire only on the two modelled hosts; a migrated v25
database equals a fresh install. **Remaining boundaries:** `Suffix` is stored
but not shown (never seen in the collections, no evidenced meaning); legacy
EX-IM programs get no suffix.
[Workspace contract](contracts/parameter-workspace.md#unit-suffix-wp-02-19-october-2026) ·
[source attribute inventory](../crates/knx-productdb/src/parse/program.rs).

**Open (found while closing the WP-02 review, 19 October 2026): earlier column
promotions share the two migration defects v26 fixes.** A probe (not
committed) migrated synthetic fixtures through each earlier step and compared
them with a fresh install:

- *A program declared twice in one file* (fresh ingest keeps the first
  declaration): the v25 placement backfill stores the second declaration's
  `own_offset`/`binary_value`, the v23 backfill its
  `ComObjectRef/@TextParameterRefId`. v24 already keeps the first.
- *Evidence deleted that fresh ingest keeps reporting*: v23 retires
  `Dynamic/ComObjectRef/@TextParameterRefId` and
  `Static/…/ParameterBlock/@TextParameterRefId`, v24
  `Static/…/ParameterBlock/@Access`, v25 `Dynamic/Parameter/@Offset` and
  `@BitOffset`.

Both only affect databases migrated through those steps from v22–v24 with such
inputs; a fresh install is correct. Fix as its own package: one regression per
step (install, rewind with `v20_rewind::drop_v2{3,4,5}_objects`, set
`user_version`, migrate, compare with the fresh snapshot), then a repair step
in the next schema version, since released databases already passed v23–v25.

#### WP-03 — Legacy binary enumeration values

**Scope:** KL-128 legacy `binary_value`, not the `.knxprod` half already
stored since product DB v25 (ADR-0141). Establish the EX-IM field mapping
against a retained legacy witness and its independent conversion; preserve
binary lexemes/widths rather than coercing them to integers.
**G16 diagnostic slice (20 October 2026 source):** the existing `MissingMaster`
refusal explains that the ZIP is not a standalone product package and names
the missing master file; it does not deny that an inner package may be present.
CLI inspection usage names `.vd1`, while import keeps its separate
`.vd3`–`.vd5` hint and explicitly claims no VD1 import support. No archive
admission, error kind or binary-enum mapping changes. Named CLI/library and
both HTTP install/preview regressions exercise the wording; existing positive,
nested/broken-package and structural-refusal cases remain in the focused suite.
**Delivered 20 October 2026** on main-first merge `b13b0ff9`; independent review
accepted and maintained scoped gate14/14passed (2994Rust tests,97/97selected
corpus,322registered tool tests/one existing optional skip), with frozen inputs
and user-data guards clear. G16 claim released; binary mapping stays open.
**Done when:** fresh/re-ingested legacy enumerations expose exact binary
values in the existing column with regression/report tests, or explicitly
name the remaining mapping blocker. G16 diagnostic handling is described above;
no live legacy download or PR/VD2 import.
[Legacy boundary](limitations/KL-128-legacy-product-import-boundaries.md).

#### WP-04 — Large catalogues: measure, then stream

**Three separate deliveries, in order:**

1. Measure per-table/index/source-byte growth and peak memory on a disposable
   catalogue. Explain retained-source duplication and unknown-row growth
   before proposing storage optimisation; WP-01 supplies the evidence policy.
2. Inventory procedure resolver tree/byte/node/depth bounds; replace whole-tree
   parsing only where measurements justify it. Require source-step/order,
   refusal, malformed-input and expanded-budget parity; no procedure semantics
   or wider device admission is invented by streaming.
3. Only then design a web large-profile job with truthful progress,
   cancellation, shared expansion budgets, preview binding and atomic
   install/rollback. CLI large-profile support and raised storage ceilings
   already exist: do not implement them again.

**Done when per slice:** the measured baseline and nonempty parity/atomicity
checks are recorded; web cancellation cannot publish a partial catalogue;
CLI and web consume the same install service. A larger upload limit alone
closes none of these slices.
[ADR-0142](adr/0142-file-size-limits-follow-the-storage-ceiling.md) ·
[KL-151](limitations/KL-151-product-package-size-profiles.md).

#### WP-05 — Parameter layout fidelity

**Scope:** KL-003 declared-grid/row presentation, not the delivered area
register, matrix placement, separators, block access or indentation.
First build a feature/evidence table for inline sections, Table versus Grid,
`CollapseIfEmpty`, `TextParameterRefId`/`Rename`, translated row/column
captions, nested blocks, in-cell buttons and remaining control hints.
**Done when for one admitted feature:** its contract names source identity,
activation/access interactions and unsupported fallback; browser regressions
cover search, drafts, restricted rows, narrow layout and grid/row variants.
Undefined hints stay visible as unsupported rather than guessed. Coordinate
shared styles/table behaviour with the existing UI-05 owner before editing.
The delivered #26 section-caption resolver is the base for this work; reuse it rather than adding a second one.
[Grid ADR](adr/0137-declared-parameter-grids-reach-the-workspace.md) ·
[gap census](research/2026-10-17-product-database-gap-census.md).

#### WP-06 — Communication-object organisation

**Two independent slices:** (a) project `GroupObjectTree` versus manufacturer
`ShowInComObjectTree` folders, with an explicit source/identity contract;
(b) object priority representation across core, persistence, import,
projection and comparison, only after an ADR defines raw/effective provenance.
**Done when:** deterministic folder membership is source-backed; ambiguous,
inactive and module objects do not acquire invented folders; priority survives
migration/save/reopen/undo where editable. Object size and effective flag
columns are already present. Neither display slice changes bus priority or
manufacturer activation rules.
[Object table contract](COMMUNICATION_OBJECT_TABLE.md) ·
[KL-003 census follow-up](limitations/KL-003-parameter-evaluation-and-editing-boundaries.md).

#### WP-07 — Module-instance authority

**Scope:** KL-068/PDB-01, module-scoped PDB-04 consumers and newly activated
module-object construction. Read-only own instance values and unscoped object
creation are delivered. Do not substitute caption matching for `ModuleDef`,
`ModuleInstance`, repeat and number-offset identity.
**First deliverable:** one admitted mapping contract backed by a real,
independently counted witness, with missing/duplicate/nested/repeated cases.
**Done when that slice is implemented:** evaluation, parameter writes and
object reconciliation reuse that authority; ambiguous mappings refuse the
whole edit without spending IDs or changing native history. Identity-keyed
module comparison follows the mapping, not the other way round.
[Instance ADR](adr/0108-read-only-module-instance-values.md) ·
[object-refresh contract](contracts/parameter-object-refresh.md).

#### WP-08 — Parameter value codecs

**Scope:** PDB-04/PDB-05 and census `TypeDate`; separate subpackages for Date,
Colour and Picture, not one “support all widgets” task. Packed `TypeTime`
fallback is deliberate and remains until its own codec is established.
**First deliverable:** documented lexical/native/resource encoding, width,
invalid-value and roundtrip vectors for one exact type/form. **Done when:**
independent vectors and malformed/boundary cases gate the codec, source-backed
checked editor and all image consumers; unverified forms keep their inert
original-value fallback. A DPT bus codec is not a product-parameter codec.
[Admission contract](contracts/parameter-editor-admission.md) ·
[17 October lift conditions](status/2026-10-17-ledger-residue-review.md).

#### WP-09 — Offline image evidence gates

**20 October source-integrity follow-up:** the generic offline System2 entry
refuses an explicit `DownloadInvisibleParameters` declaration rather than
ignoring it. Integrated tests pin existing table-index/value/bit rules.
[Evidence and next admissible questions](research/2026-10-20-system2-download-integrity.md).
This does not close inactive/legacy patch selection, numeric-tag overlap order,
module mapping, per-type encodings, executable translation or recovery; the
exact-UP258 owner retains that application's work.

**Scope:** KL-170 (per-type byte placement), KL-171 (UTF-8 byte capacity,
termination/cut rule), KL-172 (LoadedImage/CheckSums payload), census G10
(hidden PID-78 placement) and G14 (`Data` shorter than `Size`). Record each
as a distinct evidence question; a probe for one does not settle the others.
**Done when for one question:** a defining source or separately approved,
discriminating observation establishes the exact rule, negative controls
reject the plausible alternatives, and a reviewed bounded implementation
retains unsupported cases. Do not write PID78, pad a segment, swap octets or
turn retained compressed bytes into download authority on suspicion.
The active exact-UP258 owner keeps its source/procedure/recovery work; System
1/B/300 expansion still needs separate complete source-to-plan and
whole-mutation recovery contracts, not a new mask allowlist.
[Source stop lines](research/2026-10-16-offline-source-stop-lines.md) ·
[census 3](research/2026-10-18-product-database-programmability-census.md).

#### WP-10 — Import and model evidence

**Independent, input-gated slices:** (a) `IMPORT-02` authorised ETS exports of
distinct revisions plus independently reviewed baselines; (b) `MODEL-05` a
multi-installation export or explicit native-site requirement before model
change; (c) KL-177 project-side `SubModule` grammar plus schema-22/device
witnesses and independent coverage for other untested schemas; (d) real
CSV/ESF/report/compare references (KL-038/044/051) only when supplied.
**Done when per case:** production import/service/native reopen and reports
match the independent baseline, preserve unknowns, and refuse malformed or
unsupported mappings honestly. Existing import/retention harnesses are reused;
synthetic exports do not replace authorised real exports. AES/Secure is
separately deferred, not smuggled into sample collection.
[Authorised-export harness](AUTHORIZED_RESTORE_EXPORTS.md) ·
[Site decision](adr/0125-multi-installation-site-is-blocked.md).

#### WP-11 — Native Linux acceptance

**Scope:** real WebKitGTK keyboard/focus/draft/dialog behaviour, Orca,
Wayland save/cancel/overwrite paths, the unverified capture-export interaction
and an AppImage smoke on a newer-Mesa host. Build a named workflow/platform
matrix and use fictional/simulator data; bus contact is not necessary for the
UI part. This is not a GTK4 port or a new fallback/window-close policy.
**Done when:** actual native actions and resulting files/states are recorded
for each tested cell, with unsupported environments left explicit. Chromium,
Broadway startup and a mock Tauri runtime are not substitutes. Desktop control
needs separate permission; existing owner-deferred evidence is not reopened
as an alpha/release blocker by this plan.
[KL-020](limitations/KL-020-overlay-accessibility-evidence.md) ·
[AppImage launcher](APPIMAGE_LAUNCHER.md).

#### WP-12 — Field evidence and owner acceptance

**Separate cases, not one generic “hardware test”:** KL-184 fully silent
identity reads (zero-element refusals, a device-ended and a rejected connection
are observed since, and the owner reports the house working normally after a
run) and firmware responses;
KL-186 originating plug-in acceptance after adopted bytes (software route is
complete); KL-170 tester probe/report review; optional custom multicast,
bridge Route-Back tunnel and other specifically identified live gaps.
**Done when per case:** the exact build/device/application/operation and
original evidence are verified, outcomes include refusal/unknown, and only
the witnessed boundary is narrowed. Keep private projects, identity ZIPs and
raw device evidence out of Git. Fresh device operations need their own go;
this audit sends no requests and contacts no vendor/association.
[Diagnostic evidence](research/2026-10-18-up258-device-diagnostics.md),
[eleven more devices](research/2026-10-18-device-diagnostics-eleven-devices.md),
[house run](research/2026-10-18-house-device-diagnostics.md) ·
[plug-in contract](contracts/plugin-data-adoption.md) ·
[tester flow](adr/0121-tester-download-reports.md).

#### WP-13 — Documentation drift

**Scope:** reconcile current catalogue summaries/contracts with later dated
updates and actual consumers, without rewriting preserved historical records.
Confirmed examples: installation-name edits are already revision-bound
(`RenameNameField` → `api.renameEntity`); `.knxprod` BinaryValue/placement
columns and larger bounds ship on `main` since v25/ADR-0142; note disposition,
Functions consumers, plug-in web routes and the UI waves are not missing
backend/UI implementations. KL-181's “next AppImage” wording needs the
published-release boundary rather than another ACL fix.
**Done when:** the current summary, detailed boundary and user-facing route
agree; generated limitations come from catalogue sources; links/ledger/local
information guards pass. This planning pass corrects stale open-work prose;
the wider catalogue/manual sweep remains this bounded follow-up. Also reconcile
old lifecycle “publication pending” and property-planner versus memory-partial
wording; final ledger evidence already closes the bounded lifecycle callers.
G16 CLI/refusal-message code belongs to WP-03, not this docs-only slice.
**First slice (19 October 2026):** KL-181 names the shipping AppImage
(alpha.12, verified by tag ancestry) in its catalogue summary, regenerated
overview and a dated dossier section; the FEATURES rows for issues #25/#26 say
`Main-only` (delivered, issues closed); WP-18's queue row says delivered; the
released Alpha 15 claim is removed; the intermittent SIGTERM tool test is
recorded under test gates. The confirmed examples above (rename, `.knxprod`
placement, note disposition, Functions, plug-in routes, UI waves) and the
lifecycle/property-planner wording remain this package's open sweep.
**Limitation-summary reconciliation (20 October 2026):** eight current catalogue
summaries and their dossiers now distinguish delivered caption/rename/profile
behavior, guarded release stamping, actual archive ceilings, scoped stop
admission, Alpha 12 download availability and manifest-scoped portability.
The fourth motion category is documented; preserved history and formal ledger
dispositions are unchanged. The KL-183 manual routes no longer promise that
every new save is self-contained. Other sweep topics above remain open.
**Separate verification follow-up, not an implemented fix:** the KL-183
catalogue-added-application case is source-traced only; exercise catalogue
device create → Save → reopen with an empty product database before claiming
a runtime-confirmed regression or repair.
[Reconciliation scope](status/2026-10-20-limitations-reassessment.md) ·
[Current ledger](status/LEDGER.md) ·
[delivery index](IMPLEMENTATION_STATUS.md).

#### WP-14 — Complete offline family outcomes

**Three separately claimable family packages, not a replacement for the owned
UP258 goal:**

- **System 1:** complete a real-source image only after FixupList,
  LegacyPatchAlways/default/Union encoding is established; then translate the
  entire default procedure and exercise complete restoration.
- **System B:** reuse relative views, Format-1 descriptors/link codecs and AP1
  inspection; establish product-to-ASAP/source-scope/resource placement and
  all source-step substitutions before complete image/execution integration.
- **System 300:** reuse the pure Type6 property tables; first define object
  index/instance/format/capacity discovery, parameter placement and service
  framing, then the entire loading/rescan/recovery sequence.

**Done when per family/witness:** an independent real-source oracle accounts
for every transformation and source step, production planning/execution and
fault-injected restoration cover setup, property, memory and allocation/task
state as applicable. A codec, partial resolver return, memory-only backup or
passing simulator never substitutes for those later layers. Shared System-2
or probe code requires the active UP258 owner's handover/coordination; physical
admission remains closed until its separate target/recovery/consent gate.
[System 1 contract](contracts/offline-download-system1-image.md) ·
[System B gaps](research/2026-10-16-relative-image-integration-gaps.md) ·
[System 300 contract](contracts/offline-system300-property-tables.md).

#### WP-15 — Complete module scope in web consumers

**Scope:** adopt the existing `ModuleScopeDto.nodeChain` in the manual web
`ModuleScope` type, diagnostic association and parameter-workspace identity.
The backend already supplies outermost-first evaluation identity; consumers
currently compare only `moduleNode`, `moduleId` and `moduleDefId`.
**Done when:** nested nameless scopes sharing those legacy fields cannot merge
diagnostics or exchange draft/navigation identity; complete chains and an
explicit older-server fallback have discriminating regression tests. This
response-local identity is not a durable ETS ID, a project module-instance
mapping or additional write authority (WP-07 stays separate). Coordinate shared
web files with any active owner named in `.ai/CURRENT_STATE.md`.
[ADR-0063](adr/0063-parameter-scopes-preserve-evaluation-identity.md) ·
[diagnostic grouping](../apps/knx-web/src/parameterPresentation.ts) ·
[workspace index](../apps/knx-web/src/parameterWorkspace.ts).

**Delivered (19 October 2026):** the web `ModuleScope` carries `nodeChain`;
[`moduleScope.ts`](../apps/knx-web/src/moduleScope.ts) compares complete
chains for diagnostic grouping, workspace section/group/row keys and the
Restricted/Functions section views. Regression tests cover nameless nested
scopes with equal legacy fields, outer-only and order differences, a sibling
that disappears (chosen area and draft stay with their own scope), the tagged
older-server fallback and malformed chains (treated as absent, panel still
loads). **Remaining boundary:** an older server without the field still cannot
tell nested nameless scopes apart; no real nested manufacturer sample exists;
the section caption still shows only the innermost module.

#### WP-16 — Empty-directory ZIP integrity

**Delivered (19 October 2026):**
an empty directory record in a project container, outer or nested, is
refused by name when its CRC-32 is not 0 or when it is compressed without a
complete stream (none, truncated or malformed); stored, `jar`, Java
`ZipOutputStream` and other complete empty deflate shapes still import, and a
refusal leaves existing project and product databases unchanged
([KL-159 update](limitations/KL-159-project-archive-budgets-and-duplicate-names.md)).
**Implemented follow-up (20 October 2026; delivery acceptance pending):** the
product-package reader checks complete, exactly consumed empty DEFLATE directory
streams after its existing bounded CRC reader. Member-open/read refusals include
the validated decoded leaf name. Standalone, outer-wrapper, inner-package and
historical retry regressions preserve database bytes; valid directory shapes
remain accepted. [Evidence and scope](research/2026-10-20-product-directory-stream-integrity.md).

**Remaining, unowned:** a deflated *file* whose compressed bytes continue
behind the end of its stream imports with those bytes in no member (both
layers). Ordinary-file trailing policy and directory encryption-header semantics
require separate packages; neither is changed here.

**Scope:** KL-159's documented zero-size directory corner in project containers,
including nested archives: a nonzero CRC for empty output and a deflated entry
with no complete empty stream. Headers agreeing is not payload integrity.
**Done when:** malformed CRC/no-stream cases refuse the whole import without
changing existing data; valid stored and complete deflated empty directories
(including Java/ETS shapes) keep working; existing name/budget/prefix rules are
unchanged. Reproduce before choosing the minimum fix, and verify all relevant
container consumers rather than adding a special-case UI refusal.
[Remaining corner](limitations/KL-159-project-archive-budgets-and-duplicate-names.md) ·
[directory check](../crates/knx-etsproj/src/container/local_record.rs).

#### WP-17 — Master identity provenance

**Delivered 19 October 2026** on `main` as `d18ce65e`, product DB **v27**
([ADR-0147](adr/0147-master-identity-provenance.md)). All four master families,
including DPT mains/subtypes, retain individually addressable declarations and
fresh winner links; malformed/missing-parent cases and collisions get distinct
reports through direct import, CLI, HTTP and the EN/DE catalogue display.
Migration/replay preserves normalized values and historical reports; unavailable
historical winner/encounter facts are not guessed. Independent review and the
15-stage merged package gate passed, including 863 default browser cases,
18 catalogue production cases and selected corpus 97/97. The separate product
matrix passed 11/11; controlled migration preserves all 42 unaffected full-table
values and 859 historical diagnostics. Claim released. **Remaining boundaries:**
unrecoverable historical encounters/winners, unsupported identifier families and
version-pinning policy; earlier v23–v25 column-promotion repair is separate scope.
Older builds refuse a v27 product database; back up before upgrading.

**Original scope:** KL-086's still-missing provenance for `datapoint_type`,
`function_type`, `function_point` and `space_usage`. Cover same-file/cross-file
collisions and missing IDs/parents in one schema/report contract; retain the
existing normalized winner policy rather than quietly changing it.
**Done when:** winner/losing declarations resolve to retained source bytes;
malformed/orphan declarations are explicitly counted/reported; migration or
replay provides deterministic facts and labels unavailable historical encounter
facts as unavailable. Fresh/replayed databases and reports have independent
regressions. This is distinct from WP-01's duplicate evidence-row bug and from
version-pinning policy. Serialize product-DB migrations with WP-01/02/03.
[KL-086](limitations/KL-086-duplicate-product-identifiers.md) ·
[master parser](../crates/knx-productdb/src/parse/master.rs) ·
[report contract](contracts/MANUFACTURER_REPORT_CONTRACT.md).

WP-18–WP-20 are owner requests of 19 October 2026, added after the pinned
audit. They route no ledger ID and are not part of the
[audit receipt](evidence/open-work-packages-2026-10-18.json).

#### WP-18 — Device comparison shows what differs

**Scope:** owner requests 1 (marking) and 2 (diff only) for the read-only
*Project file versus device* review
([ADR-0130](adr/0130-session-device-comparison.md)). Today
`ComparisonTable` marks only a whole row (`data-different`) and prints raw
ranges as plain strings, although the server response already carries
byte-level `changes` (address plus project and device bytes).
(a) Raw hex rows mark every differing byte on both sides. Unread or unknown
bytes are marked differently from differing bytes, and the marking also works
without colour (shape or weight plus an accessible label or count).
(b) The result display opens with only differing rows. A visible toggle shows
all rows, with a count of the hidden equal rows. Hidden by default are only
rows proven equal: decoded rows whose two sides are known and equal, and raw
ranges whose bytes were all read and are all equal. *Unknown / not read* and
not-decoded rows, and any raw range with a differing or unread byte, stay
visible. The filter changes the display only; the response, the session tags
and the "Synced" claim are unchanged.
**Owner decision (19 October 2026):** requests 1 and 2 mean the result display
of the comparison between a device's state and the project's state, not the
device list in the dialog. The differences-only default is a display choice and
also applies to raw ranges proven equal. That supersedes the
[comparison contract](contracts/device-comparison.md#decoding-and-integrity)'s
"no differences-only filter" sentence: WP-18 amends it in the same delivery
(rule: no filter may hide a differing or unread byte, and the toggle always
reveals every original read range).
**Done when:** marking uses the server's byte-level data or an
equal-length byte comparison, never a new KNX codec in the UI; regressions cover
a single differing byte, adjacent and separate runs, unread bytes, all equal,
no raw range, the filter with unknown rows, an equal raw range hidden and a
raw range with one differing or unread byte kept visible, and the toggle
revealing every original read range; the resizable-table contract
([column widths](contracts/table-column-widths.md)) and the dark and light
themes hold. The comparison stays read-only, and session-only tags are
unchanged.
[Comparison table](../apps/knx-web/src/ComparisonTable.tsx) ·
[manual](manual/user-guide/05-devices-and-products.md#comparing-the-project-with-devices-p4-since-alpha-12).
**Delivered state (19 October 2026, branch `feat/wp18-comparison-diff-20261019`,
not yet merged):** the pure display rules live in
[comparisonDiff.ts](../apps/knx-web/src/comparisonDiff.ts) (byte marks by an
equal-length comparison of the server's hex text, plus the server's
byte-level `changes`, which can only add *differing* marks; proven-equal
filter). `ComparisonTable` renders per-byte marks, row byte counts and the
*Show equal rows (N)* toggle; the Devices dialog and the Device checks
panel both pass the server's `changes`. The
[contract](contracts/device-comparison.md#result-display-wp-18-owner-amendment-19-october-2026)
and ADR-0130 carry the amendment. Verified offline only (Vitest, the
intercepted production-build browser spec in dark/light themes at 1440/400 px);
no real device was read. Remaining boundaries: the Device checks panel's own
differing-ranges table is unchanged; the toggle choice is per mounted table, not persisted;
no forced-colours/high-contrast check beyond the shape/weight marks.

#### WP-19 — Sync from a device comparison

**Scope:** owner request 1 (sync option). From a bound, current comparison
result (not *Outdated*, *Wrong application* or *Not comparable*), offer two
explicit directions:
(a) **Project → device:** hand over to the existing download workflow for
that device, with its admission checks, confirmation phrase, history and
recovery rules. It writes the full validated plan; a differential write
stays later scope (`GAP-T30-04`). No second write path.
(b) **Device → project:** adopt only the values the comparison decoded with
verified attribution (named parameter fields; group-address and association
entries where the project-side mapping is unambiguous) as one undoable
project change. Raw-only, unknown, masked or not-decoded bytes are never
adopted. An ambiguous association-to-object or address mapping refuses the
whole adoption with its reason, instead of applying part of it.
The comparison session itself stays read-only: it admits no phrase, key or
write authorisation, and a sync is always a separate, confirmed operation.
**First deliverable:** an amendment to the
[comparison contract](contracts/device-comparison.md) and ADR-0130: which row
kinds can be adopted, how the comparison revision binds the adoption, what the
preview shows, refusal reasons, and how the adoption appears in undo and
history.
**Done when:** (b) passes offline tests (stale revision refused, mixed
adoptable and unknown rows, one undo step restores the project, adopted values
re-compare as equal on a simulator); (a) reuses the download path unchanged,
including its [support classification and target-bound
acknowledgement](contracts/device-programming-support-guidance.md), and is
accepted on real hardware only with operation-specific consent. Depends on
WP-18 for a readable preview.

#### WP-20 — System family in the device list

**Scope:** owner request 3. The Devices list gets a sortable, filterable
*System* column showing System 1, System 2, System 7 or System B. Other
profiles (for example System 300) keep their own names, and an unknown mask is
shown as *Unknown (mask XXXXh)*, never guessed. The value comes from the mask
version of the device's application program in the product database. A mask
version read from a device is a separate, session-only fact and is never
mixed with the declared one.
**First deliverable:** one central, cited mask-version → family table in the
core or product layer (KNX Resources and Profiles; reuse the existing
commissioning mask profiles where they already cite their source), exposed
through the device DTO. No mapping in the UI.
**Done when:** devices without a product, without an application program, or
with an unknown mask show an explicit state; regressions cover every mapped
family and the unknown case; the column follows the resizable-table contract
and can be filtered like the existing list columns.
[Devices workspace](../apps/knx-web/src/DevicesWorkspace.tsx).
**State (19 October 2026):** delivered. One cited table of 37 rows in
[`knx-core` `system_family`](../crates/knx-core/src/system_family.rs)
([sources and rows](research/2026-10-19-mask-version-system-family.md));
`GET /api/devices` carries a closed `system` state (`noProduct`,
`noApplicationProgram`, `applicationNotInDatabase`, `applicationNotOfProduct`,
`noMaskVersion`, `invalidMask`, `unknownMask`, `family`); the web client
requires it on every row and admits it strictly, and the Devices list shows a
sortable, filterable *System* column in EN/DE. **Remaining boundaries:** the mask read from a device is not shown or
compared; masks the cited tables do not list stay *Unknown* until a cited row is
added. The table reuses the commissioning `MaskVersion` type by import; the
commissioning code itself (UP258 owner) is unchanged.

### Deliberate later scope and owner decisions — not ready-made coding jobs

| Route | IDs / boundary | Reopen condition |
| --- | --- | --- |
| Later automation | `FUTURE-01`, `FUTURE-02` | One named operation beyond the shipped literal rename/MCP writer, with revision/permission/preview/atomic undo/audit contract. No blanket macro VM or in-app agent scope. |
| Later online catalogue | `FUTURE-05` | A public, licence-clear feed usable without vendor approval, then its own provenance/integrity/offline-fallback ADR. File-based web/CLI update and restore are complete. |
| Later media/procedures | `KL-110`, `GAP-T30-04` | Separate Powerline or differential-download goal satisfying its readiness contract; no new family/transport/Secure scope merely to clear a KL row. |
| External statistics | `TOOLS-01`, `TOOLS-02`, `TOOLS-03`, `TOOLS-04`, `TOOLS-05` | Work in the collector's own repository against [its measurement contract](contracts/statistics-measurement.md), not an invented KNXBench collector. |
| Optional copy | `FUTURE-08` | New optional humour copy is requested; the original mechanism/lists already exist. |
| Owner choices | KL-129/135/185; parked research; CI/container/privacy/repository retention | A new explicit decision for allocator enforcement, version pinning, external old-history refs, parked-note publication, CI re-enablement or destructive cleanup. Keep existing decisions; do not operate on containers/backups in a planning task. |
| Wider product scope | Secure/AES/keyring; accounts/collaboration; native ports/mobile; general extension API; diff merge/three-way/native PDF; scan/monitor expansion; GA7/GA8 | Each needs a finite new requirement and architecture/evidence contract. Documented absence, protocol costs and deliberately conservative refusal policies are not unattended bugs. |

Optional hardening is kept distinct from ready work: native hung-webview
recovery design, redacted authorisation/restart status presentation, the
quiet-poll Lagged regression, descriptor-pinned backup paths and profiled
large-device navigation each need a named requirement before assignment.
Existing safe refusals and the accepted operational assumptions stay in force.

**Routing check:** the five later-scope ID groups above plus WP-08/09/10 route
all 17 non-terminal ledger IDs once. Accepted boundaries and completed entries
with narrower residue remain linked through the audit receipt, not converted
into a second status ledger. All 195 catalogue keys have a package/exclusion
route, including resolved and historical entries; a route is not a claim that
the entry is fully fixed.

**Suggested order:** WP-16 and WP-01 first; WP-17 next, then WP-02/WP-03
(serialize shared product-DB migrations). WP-15 can proceed after the UI-owner
handover. WP-04 starts with measurement, then its parser/web slices. Prepare WP-05/06
contracts independently; implement shared UI changes only after UI-05's
handover. WP-07/08/09/10/14 proceed only for their evidenced slices; do not repeat
unproductive source searches without a new lead. WP-11 needs desktop consent;
WP-12 waits on exact evidence/consent. WP-13 can proceed offline in parallel
provided it preserves the active owners' status updates. WP-18 and WP-20 are
independent offline UI packages; WP-19 follows WP-18 and starts with its
contract.

## Test gates: change-scoped by default (19 October 2026)

Delivered ([ADR-0144](adr/0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md),
[dossier](research/2026-10-19-test-suite-speed.md)): routine packages run
`python3 tools/run_package_gate.py`, releases `--full`; browser cases run in
parallel unless tagged `@exclusive`; one Rust integration-test binary per
crate. The 20 October source also includes the existing workbench-splitters
browser regression in `tsconfig.workbench-fixtures.json`, discovered by the
unchanged maintained runner; static coverage only, no application or browser
assertion change. A compiler sentinel is rejected by that config while the
application-only config does not see it; the browser spec is restored byte-exact.
Delivered on main-first `73d02240` with accepted independent review and maintained
scoped gate11/11/web29/29passed (4136Vitest,863default browser cases, all23derived
typechecks; frozen inputs and user-data guards clear). Splitter claim released.
Open, each small and none blocking:

- Production suites run one after another with one worker each (771.7 s in
  the last complete gate; `table-columns` alone 231.3 s). A lane review per
  suite could let tag-split parallel runs back in where they are safe.
- The plan names a deleted web file's specs as "not statically analysable";
  name the deleted path instead (`tools/gate_scope.py`; a planner change, so
  it costs one complete gate).
- The lane guard does not see a clock reached through destructuring, a
  re-bound alias, a getter or a bracket call, and matches member calls by
  name (ADR-0144); no spec uses those forms today.
- Three `file-menu-submenus` cases timed out at eight workers while a cold
  Vite development server started (passed at the gate's four workers and in
  reruns).
- **Closed in the gate (19 October 2026):** the package gate's test stages run
  with scratch `XDG_DATA_HOME`/`XDG_CONFIG_HOME` and the gate fails when the
  real `knx` directories change. Still open: a **direct** `cargo test` inherits
  the real homes, and spawned binaries outside `knx-cli` (server, desktop, MCP)
  were not swept. Four test comments still claim `AppState::default()` looks up
  `default_path()` (`http_device_routes.rs`, `http_product_language.rs`,
  `http_catalog_translation.rs`, a `domain.rs` test); it opens none (AR18 M8)
  ([pitfall](DEVELOPER_PITFALLS.md#gates)).
- `tools.tests.test_browser_tmp_launch`'s
  `test_cli_sigterm_waits_for_child_shutdown_before_cleanup` intermittently
  times out (`child.communicate(timeout=10)`) inside gate `tool-tests` runs
  under load (four gates on 19 October 2026; 60/60 green alone). A real
  forwarder-after-spawn race exists and has a RED test on the parked branch
  `fix/browser-tmp-signal-race-20261019`, but that fix's own gate still hit the
  timeout, so the observed failure is not explained yet; reproduce it in the
  gate's exact shape before shipping a fix.
- A package gate stopped with SIGTERM during its browser stages leaves the
  fixture Vite server (`npm exec vite --config vite.fixtures.config.ts` and its
  `node` child) running, reparented to the user's systemd (seen three times on
  19 October 2026). It holds its port for the next run; end it by PID until
  `run_web_gate.py`'s cleanup reaches Playwright's `webServer` group. Reading `products` commands and
  `device readiness` open the default product database read-only now; `device
  download/compare/restore/byte-order-probe` still use the migrating opener
  (whether they write the catalogue was not audited).

## Prior dated detail — background, not a second queue

The sections below retain the original topic/anchor context. Read current
package readiness and ownership above first; an older “open”, “pending” or
“not released” sentence is not authority to restart a delivered package.

## Catalogue cleanup follow-ups (19 October 2026)

Delivered: identity lines and placement cleanup
([ADR-0145](adr/0145-catalogue-cleanup-removes-placements-only.md)). Not built, by
decision, each a possible later package:

- A persistent "hidden" marker, so a re-install of a *different* package that
  declares the same placements does not bring them back (needs a product-DB
  migration; serialize with the product-DB package queue).
- Usage counts across saved projects; the preview counts only the open project.
- Removing whole application programs or products, with a project scan.
- Provenance and cleanup cost on very large catalogues is bounded by design (chunked
  ID lookups, one pass for token and last-placement check) but not measured on a
  real large catalogue.

## UI feedback packages (18 October 2026)

[UI-01–UI-06](goals/ui-feedback-2026-10-18.md) group the owner's eight retained UI
requests: monitor DPT correctness; compact controls, visible navigation
separators and source-label cleanup; File submenus; storage sizes and counts;
resizable table columns; labelled, bounded topology layout. The owner withdrew
item 3 (“Update We”) as a typo. **Delivered** (owner go, claimed 18 October,
claim released 19 October). **Delivered:** UI-01 (the reported 1/5/6 telegram genuinely has no DPT in the
project; the monitor now names why: no project, address not in project, no
type, or a program default the project clears) and UI-04 (on-disk project and
product database sizes, catalogue counts). **Delivered:** UI-02
(compact controls and separators), UI-03 (File submenus) and UI-06 (labelled,
bounded topology), published as `bc9d73c1` after the frozen 16-stage gate on
`471ca75a` and main-first delta acceptance. **Delivered (19 October):** UI-05
(word-safe resizable tables, density/theme, stable measured floors, independent
fit-budget evidence) together with the corrections from the whole-set review:
24 px row actions in resized tables, tables and File menu under application zoom,
a File menu that stays inside the window and above the toolbar, inline submenus
that ignore hover, taps and clicks that no longer fold the open submenu, and a
first level that scrolls when it does not fit at 150%. Decisions and per-package worker
models are in the
[goal file](goals/ui-feedback-2026-10-18.md#owner-go-and-decisions-18-october-2026-later-the-same-day).

UI-05: all 18 native table surfaces use the width adapter. The
address/object matrix's select/address/name columns resize; only its per-object
columns still need a width policy that avoids project content in preference keys. [Coverage and boundary](contracts/table-column-widths.md).

Open follow-ups found by the UISET review (18 October 2026), outside the table fix:

- **Pane drags ignore application zoom.** Navigation and inspector pane
  resizing adds the raw pointer delta to a layout-pixel width, so at 150% the
  pane edge moves 1.5 times the pointer distance (0.8 times at 80%). Same
  normalisation as the tables (`appUiScale()`), with a real-drag browser case
  per scale. [KL-130](limitations/KL-130-ZOOM-application-zoom.md#update-18-october-2026-resizable-tables-under-application-zoom).
- **Zoomed table cases bypass the scale setting.** The 80%/150% table cases
  load the real app but write `--app-ui-scale` on the root directly while the
  `uiScale` setting stays at 100%. The setting-driven effect in `App`, the
  narrow-window inspector stacking math (`viewportWidth / uiScale`) and the
  startup order with a stored scale are therefore not exercised together with
  the tables (the File menu cases do use a stored `uiScale`). One table case
  through Settings or a stored `uiScale`, including startup remeasurement,
  would close this.

**Both follow-ups delivered (19 October 2026, WP-15 branch):** pane and
block drags normalise by `appUiScale()`; one table case runs through the stored
`uiScale` setting and a reload
([KL-130](limitations/KL-130-ZOOM-application-zoom.md#update-19-october-2026-pane-drags-and-a-setting-driven-table-case)).
Still open: the same cases on WebKitGTK (Chromium-only evidence, KL-130).

## Prepared goal packages (17 October 2026)

The open work of 17 October is cut into
[goal packages P1–P8](goals/open-work-2026-10-17/README.md) with shared rules,
order (P5 → P2 → P3, P4 in parallel, P1 once its entry conditions hold),
collisions and the routing of every non-terminal ledger row. The files reserve
nothing: a package belongs to a session only after it pushes its claim entry to
the handover. P5 (this page's synchronisation), P2 (plug-in data in the web app),
P3 (notes when deleting, below) and P4 (test-gate hygiene: parallel browser lane
re-admitted, symlink-safe corpus-output guard, counted ZIP work bound,
`knx-desktop` kept in the Rust workspace gate) and P1 (object-local function
choices, rescued from the rewritten history) are delivered. Still open from the
set: only the not-goal-ready P6–P8.

## Project notes: closed, with bounded delete paths (17 October 2026)

[Package P3](goals/open-work-2026-10-17/p3-note-delete-disposition.md) closes
`FUTURE-03` ([ADR-0138](adr/0138-notes-get-an-explicit-decision-before-a-delete.md)):
web deletes ask what happens to every affected note (keep on the project, move,
delete; one undo step), and selective import carries source notes along their
remapped targets with an explicit choice for the rest. **Not built, by
decision:** a decision dialog for the CLI, group address CSV delete rows and
scan reconciliation (they refuse naming the notes), and a control for
unselected source notes (no shipped reader produces source notes)
([limitation](limitations/project-notes-workbench.md#update-2026-10-17-deleting-a-notes-target-adr-0138)).

## Product-database gap census (17 October 2026)

The [census of the second product-database collection](research/2026-10-17-product-database-gap-census.md)
(546 new application programs, production installer run on every file) names
open work in this order:

1. ~~**Correctness:** block/separator `Access` and separators outside declared
   grids~~ — done ([ADR-0139](adr/0139-block-access-and-separators-reach-the-parameter-tab.md);
   remaining boundaries in [KL-003](limitations/KL-003-parameter-evaluation-and-editing-boundaries.md#product-database-gap-census-17-october-2026)).
2. ~~**Install:** vendor wrapper ZIPs fail as "missing knx_master.xml"~~ — done
   ([ADR-0140](adr/0140-vendor-zips-of-product-packages-install-as-a-whole.md)):
   CLI and web install every inner package together, other files are listed with
   their digest, and `--allow-large-package` covers vendor ZIPs. **Still open:**
   the large profile stays CLI-only (the web needs progress and cancellation
   first, [KL-151](limitations/KL-151-product-package-size-profiles.md)), and
   database growth (one complete database grew to 4.56 GB) waits for a
   per-table measurement (census step 5).
3. ~~**Editors:** packed `TypeTime` units are offered as a decimal count~~ —
   done: only the six plain units the collection declares keep the count
   control, everything else falls back
   ([contract](contracts/parameter-editor-admission.md#packed-time-units-fall-back-2026-10-18)).
   **Still open:** `TypeDate` (first seen in the addendum) is unmodelled and
   edited as free text; its value format needs evidence before an editor.
4. **Layouts:** ~~`IndentLevel`~~ and ~~object size~~ — done
   ([row indentation](contracts/parameter-workspace.md#row-indentation-census-p4-18-october-2026);
   the object table names the resolved size); ~~`SuffixText`~~ — done (WP-02,
   product DB v26). **Still open:** inline sections, object folders from
   `ShowInComObjectTree` (documented in Project Schema 23), object priority (not
   in the core model); later conditional captions (`Rename`), objects by
   parameter page and identity-keyed module comparison. Each needs its own
   contract first.

## Product-database census 3 (18 October 2026)

[Census 3](research/2026-10-18-product-database-programmability-census.md)
measured a third collection (177 new programs) for programmability and
completeness: 3 build a download plan, none is verified. Delivered the same day
([follow-up](research/2026-10-18-product-database-programmability-census.md#follow-up-delivered-18-october-2026)):
upper-case legacy tables (G11), the placement-first refusal message (G10),
placement and enumeration columns with an accurate unknown report (G13,
ADR-0141), and size bounds at the storage ceiling (G12, ADR-0142). Open:

1. **Research:** what a tool does with a hidden PID-78 parameter (write,
   compare or ignore); KNXBench must not write it without evidence (G10).
   First device-side data point: an MDT `07B0` panel's `HWT_Check` default
   equals its real PID78 reply ([evidence](research/2026-10-18-device-diagnostics-eleven-devices.md#g10-a-hidden-hardware-type-parameter-meets-its-device)).
2. **Evidence first:** the fill rule for segment `Data` shorter than `Size`
   (G14).
3. **Smaller:** configuration kept in ETS apps (G15, KL-174) and two message
   wordings (G16).
4. **Streaming:** procedure resolution still builds a whole-document tree with
   its own bounds (ADR-0142, unchanged on purpose); the legacy importer could
   fill `parameter_type_enum.binary_value` too.
5. **Duplicate unknown rows: fixed 19 October (WP-01, ADR-0146).** A v24
   shared install of the 115-instance matrix corpus held 978 `ingest_unknown`
   rows repeating another row of the same source, program, xpath, kind and
   name (577 key groups in 48 sources). Cause: each package install re-parsed
   the members it shares with earlier packages and wrote their rows again.
   Product DB v26 gives the row an identity and merges existing copies; the
   v25 measurement (843 construct and 6 conflict copies) is gone after the
   re-pin. Report totals were never affected.

## Plug-in data adoption: plug-in acceptance open (17 October 2026)

Adopting a device's stored plug-in bytes between two projects of one lineage is
delivered in the application layer, the CLI (`knx adopt-plugin-data`) and, since
[package P2](goals/open-work-2026-10-17/p2-plugin-data-web.md), the server and
the web app (device panel and command palette;
[contract](contracts/plugin-data-adoption.md#server-and-web-app),
[ADR-0136](adr/0136-plugin-data-adoption-is-audited-opaque-replacement.md) and
its amendment). [KL-186](KNOWN_LIMITATIONS.md#catalogue-kl-186) stays
`ACCEPTED_BOUNDARY`. **Open:** acceptance of adopted bytes by the originating
plug-in, which only the owner can check on their own installation.

## Declared parameter grids: remaining fidelity (17 October 2026)

[Declared grids render](adr/0137-declared-parameter-grids-reach-the-workspace.md)
from source cells. Not yet reproduced: `Table` versus `Grid` visual rules (needs
the referenced schema rules document), `CollapseIfEmpty`, parameter-driven
captions (`TextParameterRefId`), buttons and nested blocks in cells, and
translated row/column captions. Each needs its own evidence before it is shown.

## Local machine details: history rewritten (17 October 2026)

Tree, history, messages and tags are clean and guarded
([ADR-0135](adr/0135-repository-carries-no-local-machine-details.md),
[KL-185](KNOWN_LIMITATIONS.md#catalogue-kl-185)). Only GitHub's two read-only
pull-request refs and the fork still serve pre-rewrite commits. Optional next
step, owner's call: ask GitHub Support to purge them. Local branches from
before the rewrite have rewritten copies under
`refs/kl185-rewritten/heads/<branch>` (move a clean worktree with
`git reset --keep`); newer commits move with `git rebase --onto`, taking both
bases from [COMMIT_MAP_2026-10-17](history/COMMIT_MAP_2026-10-17.txt). Not
`refs/rewritten/`: that namespace is per-worktree and belongs to `git rebase -r`.

## Read-only diagnostic export: 46 hardware runs (18 October 2026)

[Implemented and simulator-verified](contracts/device-diagnostic-export.md);
46 real devices from eight manufacturers
([detector](research/2026-10-18-up258-device-diagnostics.md),
[eleven more](research/2026-10-18-device-diagnostics-eleven-devices.md),
[the maintainer's house](research/2026-10-18-house-device-diagnostics.md)): three refused
PID78 with zero elements, two System-1 sensors ended the connection, one
connection was rejected; the owner reports the house working normally
afterwards. Still open: a fully silent device; any memory profile only with its own evidence and decision (ADR-0134; the one admitted so far is ADR-0143's below).

## Exact Siemens UP 258D12 programming work (18 October 2026)

Owner-authorised offline work through the hardware boundary is tracked in the
[exact-device goal](goals/up258-programming-2026-10-18.md). The bounded
[source-field preview](contracts/system2-source-probe-preview.md) is available;
it is not a memory read or programming unlock. Remaining image/procedure and
recovery dependencies stay explicit; the reporter approves each hardware step.
Stage 0 of the write ladder is implemented offline: the read-only, ProductDB-
derived `system2-declared-segments-v1` diagnostic profile
([ADR-0143](adr/0143-declared-segment-memory-evidence-profile.md), CLI only,
simulator-tested; four independent reviews, the last accepting). Open: the
reporter's consented run on the device, the offline comparison of its result,
a web entry (UI owner), a `--device <area.line.device>` selector for the CLI
(today only the internal `--device-id`, which no surface displays; the issue #22
request uses a `sqlite3` query instead), and one MINOR reader gap: the identity stop reason
(e.g. `identity-mismatch`) is not cross-checked against the identity items, so
a deliberately rewritten and rehashed archive could carry a wrong reason. It
does not change recorded bytes, coverage or what the collector sends;
checksums never claimed to detect deliberate rewriting.

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
codecs (PDB-04 stays LATER), and native WebKitGTK/Orca checks. Revision-bound
parameter writes and installation renames reached the web host with Goal B1a
on 13 October ([contract](contracts/revision-bound-writes.md)); that is not
an open implementation task.

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
WebKitGTK/Orca checks. The broader FUTURE-03 residue was delivered by P3 on
17 October (note disposition and selective-import note mapping, above).

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
later the same day (section above). FUTURE-03 was subsequently completed by
P3 on 17 October; the AP3/AP5/AP6/AP7 remainders and hardware/ETS/report
compatibility are not closed by this earlier leaf.


**Baseline audit:** 2026-10-08, against `origin/main` `608a204b`; the dated
sections on this page were added since. This historical baseline is superseded
by the reconciled package queue above. This page is the one
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
mapping remains open; the workbench host shipped on 12 October and bound
writes on 13 October. PDB-04 remains LATER, not DONE.
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
- Ledger counts on 17 October 2026 after package P3, quoted from the ledger's
  *Counts* section (`check-ledger` green): **Snapshot IDs** (180 rows) — DONE=48,
  BLOCKED_EXTERNAL=2, ACCEPTED_BOUNDARY=118, LATER=12; **Post-snapshot IDs**
  (28 rows) — DONE=19, ACCEPTED_BOUNDARY=6, LATER=3. The two counts below are
  historical and superseded by these.
- Historical ledger counts (its own *Counts* line, `check-ledger` green, after the
  2026-10-09 owner cleanup): snapshot 180 rows — DONE 41, ACCEPTED_BOUNDARY 119,
  LATER 20; post-snapshot 11 rows — DONE 9, ACCEPTED_BOUNDARY 2.
- Handover `.ai/CURRENT_STATE.md` (newest entries per track), worktree list,
  and a path-by-path check of which root files exist on `origin/main`.
- Historical recount, after import expansion: 191 distinct IDs,
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
| Legacy ETS3 product databases (`.vd3`–`.vd5`) | L1 inspection, L2 import/secret withholding, L3 web upload/password handling, the VD5 installer layout with measured bounds and L4 download plans from `s19_block` are on `main` | A first live legacy download (needs the maintainer's device go; none has run). Atomic types 3/5 are mapped (2026-10-13, KL-128); legacy `BinaryValue` mapping remains WP-03. The `.knxprod` enumeration columns and reporting were delivered with v25/ADR-0141 on 18 October. Merged legacy procedures (`07B0h`) stay refused. | [ADR-0094](adr/0094-legacy-exim-product-files.md), [VD4 import](VD4_PRODUCT_DATABASE_IMPORT.md), KL §128 |
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
file-by-file main check, the `merge-tree` conflict maps and the decisions. The
table includes subsequently delivered or retired packages as history; only
the explicitly kept, unpublished research is still parked:

| Package | What it holds | Owner decision |
| --- | --- | --- |
| `native-evidence-20261011` (local branch; worktree removed 2026-10-13) — residue checked 2026-10-13: nothing left | 7 locally accepted commits: durable native import reports (ADR-0113), CLI/source-report/payload catalogues, export-privacy consent, read-only project metadata and recorded device state | **Delivered 2026-10-12**: ported onto main with a fresh gate ([evidence](evidence/native-evidence-port-2026-10-12.json)). Local branch retired 2026-10-14 after an owner go; original commits kept in the `KNXBench.backups/2026-10-14-hygiene-cleanup/` bundle |
| `feature/ux-feedback-20261010` (local branch; worktree removed 2026-10-13) | 13 locally accepted commits, UX AP1–AP5 | **Delivered 2026-10-13**: AP2/AP3/AP5 and the parts of AP1 main lacked, ported with a fresh gate ([evidence](evidence/ux-feedback-port-2026-10-13.json)); motion ADR lands as ADR-0122; AP4 dropped. Worktree removed after an owner go; local branch retired 2026-10-14, original commits kept in the `KNXBench.backups/2026-10-14-hygiene-cleanup/` bundle |
| `feature/functions-sites-20261009` (local branch only) | KNX Functions (`MODEL-07`) as one unreviewed `wip(functions)` commit `1154b72f` on schema 12 (main is at 14); acceptance attempt 4 failed | Superseded 2026-10-13: Goal A4 rebuilt the backend fresh on schema 15 ([ADR-0124](adr/0124-project-functions.md)); the WIP was read, not merged. Branch and its stash (`Functions source before published History sync`) dropped by the owner 2026-10-14; both kept in the `KNXBench.backups/2026-10-14-hygiene-cleanup/` bundle |
| Initial native-write preflight | Never-compiled draft from `import-lifecycle-20261011` | Archived as a local patch; to be built fresh. Worktree and branch removed |
| `audit-findings-20261011`, `native-import-diagnostics-20261011` | Superseded / already on main | Dropped. Worktrees and branches removed |
| `wip/manufacturer-plugin-research-20261013` (local branch only) | 14 unpublished research notes under `docs/research/` (manufacturer-plugin host/contract results, fast-download wrapper, DLLs on Linux), found in the root checkout; unreviewed, none of them on main | **Kept by owner decision 2026-10-14**; not merged, not pushed |
| `wip/root-residue-20261013`, `feature/device-support-guidance-20261013`, `feature/device-support-guidance-s2-20261013` | Old root drafts; two pre-integration candidates of the support guidance delivered as `e478a1a0` | Retired 2026-10-14 by owner decision: worktrees removed, branches kept only in the `KNXBench.backups/2026-10-14-hygiene-cleanup/` bundle. The S2-only Codex collision log was copied to the root's ignored `.ai/logs/` |

**Object-local function choices (17 October 2026): delivered.** Codex's
unpublished feature commit `99dd6e65` was cherry-picked onto the parameter
workspace by [package P1](goals/open-work-2026-10-17/p1-object-function-rescue.md)
and is on `main` ([status](IMPLEMENTATION_STATUS.md#object-local-function-dropdowns-17-october-2026)).
Its two old-history worktrees, their branches and rewritten refs are retired;
the cherry-pick on `main` is the record.

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
- **Model/import:** KNX `Functions` (`MODEL-07`) is `DONE` since 17 October
  (per-Function comparison table, desktop surface via the embedded server,
  read-only MCP `list_functions`; [contract](contracts/project-functions.md)).
  Recorded boundaries only: schemas 20/21, `Implements`/`DefaulGroupRange`, a
  Functions overview outside Properties.
  Several installations under one Site (`MODEL-05`): blocked on evidence
  ([ADR-0125](adr/0125-multi-installation-site-is-blocked.md)); ETS
  restore-point revisions as extra private regressions (`IMPORT-02`, requires
  authorized ETS exports and reviewed baselines). Selective lines/devices and
  bounded `knx_cvexc` content analysis have verified local implementation and
  separately authorized [source integration](status/2026-10-10-import-expansion-integration.md);
  neither replaces the missing authorized revision exports. Colour/Picture
  parameter value codecs and module-scoped parameter mapping (`PDB-04`; checked
  Time/Slider controls and revision-bound writes already ship); independent `.knxproj` samples for untested schemas
  (12/13/14/22; schema 20 rests on public samples, KL §177) and an
  AES-protected ETS6 sample.
- **Comparison/report extensions:** applying/merging diffs, three-way
  comparison and native PDF output remain absent; comparison and browser
  print-to-PDF already exist. These are recorded boundaries, not active work.
- **Features:** project notes (`FUTURE-03`) are complete since 17 October
  (section above); repetitive-task automation beyond the literal rename recipe (`FUTURE-02`);
  agent mutations beyond the opt-in rename writer (`FUTURE-01`; read-only MCP
  ADR-0090, writer ADR-0116); dashboard drill-down, mobile app, native ports and
  an online catalogue source (`FUTURE-05`; file-based updates ship in CLI and
  web). Reconciled on 17 October in the
  [ledger residue review](status/2026-10-17-ledger-residue-review.md), which also
  records the evidence re-check of `KL-170`/`171`/`172`, `PDB-04`, `GAP-T30-04`,
  `KL-110`, `MODEL-05` and `IMPORT-02` (no new primary source; lift conditions
  unchanged). More humour (`FUTURE-08`) means optional expansion:
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
