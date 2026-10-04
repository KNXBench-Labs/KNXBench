# Known limitations

## Accepted commissioning validation boundary — user decision 2026-10-04

New real-hardware, power-loss, vendor and ETS validation is not required to
complete the commissioning goal: the user explicitly removed these experiments
because they cannot provide them. Their absence remains disclosed in
[KNXBench user notices](manual/known-issues.md#commissioning-validation-boundary)
and is not a hardware/ETS compatibility or recovery guarantee. Existing scoped
evidence and historical limitations stay intact. Broader caller coverage,
Web/client adoption and offline recovery/abort/restore contracts remain open;
original-value backup, authorization and fail-closed runtime gates are unchanged.
This decision does not mark SAFE-03/AUDIT-01 or the Alpha release complete.

## CRT motion is browser-verified application behavior, not theme-pack v2

The CRT interaction follow-up implements real App/ProjectExplorer/
GroupAddressTable feedback through a separately selected motion style. Palette
v1 remains declarative and unchanged; importing it cannot enable motion. Exact
selection `#003300`, Save-only purple, additional input focus glow and all other
component redesign remain outside this animation change. Native table markup,
bulk-selection rail and existing delegated keyboard handlers are retained.

Standard admits bounded light/flash; Subtle admits fill only. Off and OS reduced
motion cancel active CRT effects. Scroll, resize, zoom/preference change, anchor
retirement, drag start, window blur and disposal retire transient feedback.
Save glow is a request-activation cue, never a success indicator; a failed Save
still reports its error and keeps the project dirty. Autosave remains silent.

Evidence uses only intercepted synthetic Chromium requests; there is no native
WebKitGTK, Orca, full ARIA tree/grid, every-state contrast or WCAG acceptance claim.
The native-frame screenshot uses paused test time; normal expiry/cancellation is
checked separately. Authorized feature16c9d774 is integrated onto main as3d03aea5,
with repeated frontend/browser/workspace gates. Publication is established by
current refs/closure handover, not a palette import. The ordinary workspace gate
passed3009 tests and left166 ignored/private/hardware tests unexecuted; this does
not certify those scopes. See [CRT guide](DESIGN_RETRO_GREEN_CRT.md).

<a id="theme-pack-management-is-a-local-candidate-not-release-acceptance"></a>
## Verified theme pack management is not whole-extension release acceptance

U15 admission/runtime is delivered. U16 delivers file transport as 1f94808d,
acknowledged conditional installation/removal/selection and structured errors;
the 16-check candidate and actual-merged 22-check gates passed. Source publication
and exact remote artifact readback are verified. U17 Appearance management/
diagnostics/preview passes actual merged23-command acceptance on f16f1e40:
Web1702, Chromium72, Rust2984/0/165,27 explicitly selected private offline cases
plus the115-instance matrix,697 unchanged source/config inputs and420 unchanged
private files. Publication/readback is tracked in the current handover.

Unknown raw entries are retained, not repaired. Recovery exports only observed
browser JSON theme scope, not original lexical bytes, duplicate names or numeric
precision; retain the original server file for original-file recovery. Recovery
refuses output over 1 MiB or nonserializable data without deleting/truncating it.
Explicit stored null cannot express an absence-based conditional precondition
and disarms theme edits. Conditional comparison is protected by one server's
settings lock, not a multiprocess filesystem CAS. Older servers without the
conditional capability are not silently trusted.

Admission targets serialized JSON/settings data, not isolation from already
privileged hostile JavaScript proxies. Contrast covers the documented three
role pairs and accents, not every component/WCAG criterion. Local intercepted
Chromium/self-review is not native WebKitGTK/Orca, independent approval or
release acceptance. U15 publication remains9d1ae19d. U18-R1 is closed by
actual GroupAddressTable/Inspector/Overlay/diagnostic states over five built-ins,
System light/dark and two admitted user palettes:10/10 representative cases,
three named rendered-style failures, full Chromium82 and Web1702 on the
actual-integrated703-input stand1660911b. The coupled24-command acceptance
includes28 selected offline cases (matrix included);421 private files unchanged.
This closes the fixture-coverage finding, not native/Orca/general WCAG or
whole-application Alpha/ETS acceptance. Publication/readback is in the handover.

Preview cancellation changes presentation only. Once a guarded write is
dispatched, closing Settings cannot cancel its server operation; the UI states
this and disables false-undo Cancel while awaiting acknowledgment. A definitive
409 retains the last confirmed settings until the existing refresh sees a peer;
an ambiguous 500 may require a read, never a silent write retry. Recovery exports
currently observed data (possibly cache), not a fresh server-file backup. A local
cache-write failure after server acknowledgment is separately reported.
Older standalone builtin callers retain their ordinary callback; the actual
managed Appearance selector uses guarded acknowledgment for all theme choices.

## Backup directory synchronization is not a disk-loss or confinement proof

The device-memory and service-control backup writers sync the supplied and
resolved directory chains after file synchronization and readback. Errors from
any ancestor refuse a receipt, including when a filesystem or parent directory
does not support the requested synchronization; do not silently ignore those
errors. Failed attempts can leave owner-only diagnostic files on disk and
must not be mistaken for successful recovery receipts.

Directory paths must remain stable during the operation. This path-based
implementation does not pin directory descriptors, defend against concurrent
symlink replacement/renaming or changes to process CWD, or prove actual
power-loss behavior of the filesystem, controller and drive. It does not widen
plan-scoped memory coverage or establish recovery for address programming,
serial address writes, address reset or master reset. No hardware gate has
been opened by this offline storage change.

## Partial commissioning bus-activity snapshot (ADR-0055)

`GET /api/bus/activity` is a read-only **partial**, server-lifetime snapshot,
not an audit log or proof of device mutation. It reports retained download,
button-programming, monitor and line-scan sessions plus bounded one-shot
read and service-control-write records. Short actions can be missed by polling;
an empty session list does not prove the gateway is idle. Held locks need not
identify an operation or target. The one-shot ring can evict records and
reports `oneShotDropped`; restart loses them. Serial-address and group writes
do not supply durable activity receipts. A `serviceControlWrite` entry
distinguishes no-change, not-sent, verified, unverified and unknown outcomes;
its backup/send flags do not prove transmission or a whole-device recovery.
No property octets, keys or host backup paths are disclosed. The global Web
status bar and per-action history remain open (ADR-0055/0056); simulator
evidence is not a live bus check.

**Durable backend candidate, 2026-10-03 (ADR-0064).** A separate versioned
SQLite history now retains four one-shot operation kinds across ring eviction
and restart; seven kinds remain explicitly untracked. Previous-incarnation
running rows become interrupted/unknown, never success. Storage/metadata
refusal is visible and latches unavailable history; download and Debug-write
starts refuse before tunnelling when history cannot be admitted. Property
intent persistence still follows the PID 8/PID 14 recovery and precedes the
first property write. This is metadata only, not a recovery image, complete
download journal or a shipped global UI. Whole-candidate acceptance and
publication remain pending. The exact consumer contract and residue are in
[COMMISSIONING_ACTIVITY_HISTORY](COMMISSIONING_ACTIVITY_HISTORY.md) and
[COMMISSIONING_ALPHA_LEDGER](COMMISSIONING_ALPHA_LEDGER.md).

**Admission correction, 2026-10-03.** Late synthetic review reproduced SQLite
hot-journal rollback before a foreign-format refusal. Corrected openers first
admit existing identity/schema read-only; WAL is explicitly unsupported and
header-refused before SQLite can create shared-memory sidecars. Hot-journal and
WAL RED/GREEN cases preserve main/journal/WAL bytes; 21 current-source compiled
behavioral mutants fail as expected and restoration is exact. Preserve failed
storage and its sidecars for explicit repair, not automatic rollback/conversion.
Hostile concurrent path replacement is not covered. Earlier full gate receipts
belong to their older source; corrected integrated acceptance is still pending.

## U12 structure editor scope (ISSUE-05)

Area/line renames and line, building-part and group-range reparenting are
local, undoable project commands. The centre workspace reuses the same
creation forms and Inspector commands as the project explorer and Properties.
All structure mutations still target **only the first installation**; later
installations remain visible and savable. The centre cannot select an orphaned
line that has no projected area, although the explicit line-move command can
attach such an imported line by ID. Installation renaming is also still
absent (§127).

A line move changes its area relationship, **not** its numeric line address
or any device's individual address. It refuses a destination with a duplicate
line number or an addressed device outside the target area.line prefix.
Duplicate line/area IDs, multiple owners and multiply placed devices are
not guessed away. Area, line, building-part and group-range IDs repeated in
one or more installations disable structural editing in the Inspector; direct
commands refuse ambiguous IDs too. The UI does not repair or renumber
imported duplicate identities automatically. Building-part moves refuse new
cycles and ambiguous parent references. Group-range moves additionally
require destination-span containment and no sibling overlap. The undo-only
placement commands retain original sibling positions and may restore
pre-existing imported invalid relationships; they do not permit a new invalid
forward move. Deleting a non-empty area, line, building part or group range is
non-cascading. Deletes refuse inconsistent parent/child references that would
leave an unlisted dependent orphan, and their undo restores the original
flat-list and parent-child positions rather than appending.

Native `.knxdb` save/reopen is tested for these changes through the existing
whole-project save path. Several structure commands still have no incremental
store-sync update; a bare incremental-sync call is **not** a substitute for
saving the project. No full ETS edit/round-trip parity is claimed. The
first-installation UI was tested with mocked local API responses and Chromium;
native WebKitGTK drag/drop, screen-reader operation and live bus behavior are
not covered. Existing device-to-line/device-to-building drag/drop remains
bound to its already validated commands; structural moves use labelled native
select controls, not a new drag gesture.

## U11 device editor scope (ISSUE-09)

A device placed in a line shows its line-derived area.line prefix and lets the
user change only the device octet (1–255). New `.0` assignments are refused
unless the product database classifies the device's hardware as a coupler
(`Hardware/@IsCoupler`, RESEARCH §25): the server then uses
`SetCouplerIndividualAddress`. The web editor does not offer `.0` yet, and a
device whose product is not installed stays refused. Imported `.0` addresses
remain intact, including after undo. An imported
address with a line prefix mismatch is shown for repair, not silently
rewritten. Until repaired, moving that device to another line is refused; a
move never auto-allocates an address. Unassigned devices retain the full
address editor. Duplicate and out-of-line addresses fail in the core, not
just in the browser. If a device appears in multiple topology placements
(two lines, twice on one line, both a line and the unassigned list, or
twice unassigned), or
its line belongs to two areas, the editor refuses to choose a prefix and
stays disabled until the topology is repaired outside this editor; imported
values are preserved. Clearing an address through the core remains an
undoable repair step even when placement is malformed.

The existing line-move command and its dropdown operate on the **first
installation** only. A later installation may display and edit its
line-relative address, but moving its device between lines is not yet offered.
This is a known topology-command boundary, not evidence that its line is
missing. Likewise, new group links currently target the first installation's
group-address list. Do not infer full multi-installation editing from the
address display.

“Send + Receive” and “unlink both” are one atomic project edit and one undo
step. If either direction is already linked when adding both, the operation
fails and leaves all links unchanged; if either is missing when unlinking
both, even the **ordering** of other links stays intact. A successful unlink
can be undone without moving an interleaved or imported duplicate link.
Individual directions remain editable separately. An older server that only
accepts Send/Receive rejects `Both`;
the client reports that refusal and does not fall back to two non-atomic
requests. Flag labels expand the standard letters in English and German,
but changing flags or links here is an offline project edit, **not** a KNX
device download. Chromium layout checks cover 360, 640 and 1440 px with long
names/addresses in both languages against a local Vite/mock fixture; native
WebKitGTK, hardware behavior and screen-reader announcements remain unverified.

## U11 catalog batch scope (ISSUE-07)

The product catalog can request 1–32 devices in one project command/undo
step. Generated names use the entered base followed by a one-based index for
multiple devices; the application does not deduplicate against pre-existing
names. A selected line places the devices in that topology line, but **no
physical addresses are allocated** (including when no device octets remain).
Addresses must be assigned separately and validated by the address editor.
This is a local project edit, not a KNX download or ETS-compatibility claim.

An older server may ignore the additive `quantity` field and return a legacy
single-device response. The web client then refreshes the returned project,
warns that the requested batch was not confirmed, and does not retry. It
cannot roll back a partial change already made by a different server. For a
current server, quantity/name/line refusal happens before IDs are reserved;
the core `Batch` command rolls back project changes if one child fails.
If a response is lost after the request was sent or the server responds with
an internal error, the client cannot know whether the batch was committed;
it blocks a blind retry and directs the user to inspect or reload the project.
Successful batch responses carry per-device diagnostics; a late core batch
failure carries the zero-based child command index and its typed cause, which
the catalog maps to the one-based device number (the reservation is child 0).

Catalog creation checks the available `u32` device and communication-object
ID ranges for the full requested quantity before calling the core allocator.
AR02 also makes all nine shared core allocators fallible: `u32::MAX` is the
last valid ID, and later allocation returns `IdAllocationError` without
changing a counter. General creation, CSV planning, offline reconciliation
and ETS mapping propagate refusal rather than wrapping or panicking. CSV and
reconciliation discard a refused detached batch; editing an existing parameter
does not allocate a replacement ID. Maximum-ID native roundtrip and monotonic
undo/redo regressions are in `knx-app/tests/id_exhaustion.rs`. This does not
activate parked ADR-0039 phases 3–5 or claim every failed single edit leaves
its successfully reserved high-water mark unchanged.

Headless Chromium checked the catalog at 1440×900 and 400×700 against a
local Vite page with mock catalog data, not a real manufacturer package. A
native WebKitGTK run and real screen-reader announcements are not verified.

## Commissioning readiness and pre-write backup boundary (ADR-0049)

Offline coverage of 103 locally installed product packages found 55/181
`0701h`/`0705h` application programs plannable with product **defaults** and
no group links; 126 were refused with reasons. A plan is not a verified
hardware download, and a one-device live run does not validate every
configuration or device revision. `verified` is scoped to application *and*
download type; `untested` needs a second exact acknowledgement. Other masks
remain refused. The catalogue is a sample, not an all-device support list.

The automatic backup is read **after** the target identity check but before
the first mutation, and records only memory regions the plan is about to
overwrite plus affected load states. It is **not a complete device memory
dump**, and does not preserve properties, keys held outside those regions,
masked/unwritten memory or a non-Loaded load state. The JSON can contain
private device configuration and must be protected and not attached to
diagnostics. Only a backup with all parts Loaded and a product procedure
that rederives the same steps and memory shape can become a restore plan.
Backup creation and restore were confirmed live once, on one device and
program (MDT `1.1.67`, `0701h`, `A-0027-15-0BAC`, complete download; RESEARCH
top entry, 2026-09-29): backup before the first write, restore
byte-identical over 180 dump lines; the parameters-only partial download
likewise (backup of `4400h`, K7 parameters written, restore to a
byte-identical dump). The backup/restore path for a group-address partial plan, other devices
and the refuse-before-write path on a failing backup remain simulator-only;
none can be promised as a universal rollback. The existing manual read-only
baseline required by `goal-commission.md` before live tests is separate and
still required when live tests are authorised again.

**Device-checks UI boundary (2026-09-30).** Readiness uses only the open
project's installed product database; `verified` applies to one application
and download operation, not general device compatibility. Comparison uses
the complete plan only (the CLI/API partial option is not shown there),
requires an explicit read-only tunnel confirmation and may be refused by a
read-protected device. It is neither a backup nor proof that a later write
will work. Web evidence is mocked/EN-DE and server evidence simulated, not a
live test of this view.

## PDB-9 parameter and Dynamic coverage boundary

Since schema v15 ([ADR-0041](adr/0041-unmodelled-kinds-and-dynamic-nodes-are-named-never-hidden.md))
every `ParameterType` kind observed in the corpus is stored as its own kind,
and a reference below a Dynamic node refused for a structural reason is named
by `RefBelowSkippedNode`. What is still not modelled, each reported rather
than dropped:

- **Repeat expansion.** `Repeat` (16 in the corpus) is `UnrecognizedNode`;
  its `Module` is named by `RefBelowSkippedNode` but not expanded, so the
  repeated module's parameters are not offered for editing. Expansion needs
  the repeat count (`@Count` or a controlling `@ParameterRefId`) and
  project-side instance matching.
- **Renames and buttons.** `ParameterBlockRename` (270), `Rename` (56) and
  `Button` (20) are `UnrecognizedNode`: a conditional rename is not applied
  to the displayed label, and a button's `@EventHandler` script is not run.
- **Calculations and allocators.** `ParameterCalculation` (1,236 in 91
  programs) and `Allocator` (94 in 14) are reported unknown elements with
  their attributes, and the program bytes are retained; their
  transformation scripts and allocation ranges are not evaluated.
- **Display-only type attributes.** `UIHint`, `Increment`, `DisplayFactor`,
  `DisplayFormat`, `Pattern`, `Encoding`, `AddressType`, `TypeTime/@Unit`,
  `TypeColor/@Space`, `TypePicture/@RefId`/`@HorizontalAlignment` and
  `TypeRawData/@MaxSize` are reported with a sample, not stored in columns:
  the editor does not use them yet (e.g. no slider, no hh:mm:ss duration
  input, no colour picker, no picture display).
- **`Color`, `Picture`, `Raw` values** get only a non-empty, XML-safe check;
  the Project Schema documents no value encoding for them.
- **Not every absent reference is enumerated.** The branches of a `choose`
  refused for a value-dependent reason (`MissingValue`, `NonNumericValue`,
  `NoBranchMatched`) are conditionally hidden by design — the `choose` is
  named, its branches' references are not listed. A `Module` that is not
  expanded (`ModuleDefNotFound`, cycle, depth or expansion budget) is named
  by its own diagnostic, but its `ModuleDef`'s references are not listed.
  `RefBelowSkippedNode` reports share the `MAX_MODULE_ACTIVATIONS` budget;
  past it, output is truncated with one budget diagnostic.
- **Install reports stay historical.** `package_install_unknown` rows
  written before v15 keep their `Element TypeTime`/`TypeColor` entries; the
  v14→v15 backfill corrects `ingest_unknown` (the per-blob report), not the
  install-time snapshot — the same rule as the v12→v13 precedent
  (COMPATIBILITY: "Historical encounter reports remain historical").

## PDB-3 report history and coverage boundary

Product install facts are measured only for installs carrying the schema-v12
encounter ledger. Older installs deliberately render as unavailable rather
than as zero. The projection reports unsupported constructs and diagnostics,
but does not interpret every vendor construct, verify signatures, or claim ETS
parity. PDB-8 (schema v14) reports uninterpreted element subtrees inside
supported master sections (`Format`, `PublicKeys`, `OrderNumberFormattingScript`)
but gives none of them typed storage: DPT bit layouts, manufacturer public keys
and order-number scripts are retained bytes plus a diagnostic, not queryable
data. Section-level master data (`MaskVersions` with its resources and access
rights, `InterfaceObjectTypes`, …) likewise stays reported per section, not per
element, and untyped until a commissioning feature proves which parts it needs.
AR05's v19 candidate reports unconsumed attributes inside the master `Languages`
branch through a shared byte-only pass; consumed translation keys/text are not
mislabelled unknown. TranslationUnit RefId/Version remain uninterpreted, not
typed metadata. The bounded, exact-scope census observes 1,870 Version
occurrences across 67 distinct master blobs; the historical 1,928 total used
a different scope/unit and is not this gate's pin. Current source-derived
evidence is separate from immutable historical install reports. The scoped
matrix and real upgrade pass after independently reconciling the +230 isolated /
+226 shared keys; no normalized values or prior unknown evidence were lost.
Repair-path regressions also preserve unexamined status for identity-invalid
bytes while clearing stale exact-owner markers after authentic recovery.
Renewed integrated acceptance of that last correction and publication remain
pending; see [the AR05 contract](MANUFACTURER_REPORT_CONTRACT.md). Persisted subtree
diagnostics are validated for shape, not re-derived from the blob. PDB-10 (schema v16, ADR-0042) inventories baggage: every `Baggages.xml` declaration typed as raw lexemes and resolved exactly to its member, every payload classified by content, nested ZIPs measured from their directory only. See §134 for what it deliberately does not do.

Each entry states the limitation, its cause, what it costs the user, and the
condition under which it would be lifted. Nothing here is a defect to be fixed
by trying harder — these are consequences of evidence we do not have or of
decisions recorded in [docs/adr/](adr/).

The companion document is [COMPATIBILITY.md](COMPATIBILITY.md), which states
what is verified. Nothing may appear as verified there and as a limitation
here.

## PDB-7 catalogue metadata are source strings, not capabilities

**Limitation.** The eight application-program catalogue attributes persisted
at product-database schema v13 — `IsSecureEnabled`,
`MaxSecurityGroupKeyTableEntries`, `MaxSecurityIndividualAddressEntries`,
`MaxSecurityP2PKeyTableEntries`, `MaxTunnelingUserEntries`, `MaxUserEntries`,
`MinEtsVersion` and `ReplacesVersions` — are stored and shown exactly as the
manufacturer wrote them. They are never parsed into booleans, capacities,
version ranges or replacement graphs.

**Cause.** No public schema defines their value space, and the private
115-instance corpus shows non-uniform shapes: `MinEtsVersion` appears both as
dotted-numeric and as other forms, `ReplacesVersions` both as an unsigned
decimal and as other forms. Guessing a type would be an invention, and
`IsSecureEnabled="true"` in a catalogue file is a vendor claim about a
product, not a verified property of the device on the bus.

**Cost.** A user cannot filter by "devices supporting KNX Data Secure", sort
by real ETS version, or follow `ReplacesVersions` as links. The strings must
be read by a human.

**Lifted when.** A normative value-space definition, or a large enough
multi-manufacturer corpus, justifies a typed projection alongside — never
instead of — the retained source string. Interpreting `IsSecureEnabled` as a
security capability would additionally require the runtime KNX Secure work
tracked in limitation 8.

**Related.** Stricter XML well-formedness is now enforced for every recognized
application-program document, on direct ingest as well as on package install
and v12→v13 backfill. A file whose comments, entities, attributes, prolog or
element nesting violate XML 1.0 is rejected with a typed error instead of
being partially interpreted. All 115 corpus instances pass this check; a
hypothetical vendor file that ETS tolerates but XML forbids would now be
refused rather than half-read.

## Closed entries and historical links

Resolved/withdrawn limitations are no longer listed below. Their evidence remains
in Git history and the dated [implementation log](IMPLEMENTATION_STATUS.md);
the following empty anchors keep old documentation links from breaking. They
are **not** open issues. The numbers are intentionally not reused.

<a id="10-project-licence--resolved-2026-09-16"></a> <a id="10-the-project-licence-is-not-decided"></a> <a id="103-unsaved-is-inferred-from-the-undo-stack-not-a-real-dirty-flag"></a> <a id="103-unsaved-is-inferred-from-the-undo-stack-not-a-real-dirty-flag--resolved"></a> <a id="11-knxprod-files-for-master-data-scheme--12-cannot-be-imported-directly"></a> <a id="117-read_on_init_flag-is-parsed-and-stored-then-discarded-before-it-reaches-knx-core"></a> <a id="118-a-succeeded-project-load-announces-nothing-to-a-screen-reader"></a> <a id="118-a-succeeded-project-load-announces-nothing-to-a-screen-reader--resolved"></a> <a id="119-on-this-machines-ntfs3-mount-cargo-has-rebuilt-from-a-stale-fingerprint--a-green-gate-is-not-evidence-by-itself"></a> <a id="120-nothing-checks-that-a-theme-is-legible"></a> <a id="120-nothing-checks-that-a-theme-is-legible--resolved-by-the-role-pair-contrast-gate"></a> <a id="122-resolved-settings-file-diagnostics-follow-the-ui-language"></a> <a id="122-settings-file-notices-reach-the-user-in-english-only"></a> <a id="123-resolved-group-addresses-no-longer-use-dotted-display-notation"></a> <a id="131-seventy-two-corpus-gates-repo-wide-still-pass-when-the-corpus-is-absent"></a> <a id="132-the-window-managers-close-button-quits-the-desktop-app-without-the-unsaved-changes-prompt"></a> <a id="147-a-received-telegrams-priority-repeat-flag-and-hop-count-are-not-kept--lifted-2026-09-30"></a> <a id="148-the-contributor-license-agreement-is-not-reviewed-by-a-lawyer-and-nothing-enforces-it--withdrawn-2026-09-30"></a> <a id="17-deleting-a-group-address-can-leave-a-dangling-grouplink--resolved"></a> <a id="19-a-search-result-inside-a-collapsed-tree-branch-is-not-revealed"></a> <a id="19-a-search-result-inside-a-collapsed-tree-branch-is-not-revealed--resolved-2026-09-22-t12"></a> <a id="21-a-ui-created-group-address-without-a-range-is-still-dropped-on-export--partially-resolved"></a> <a id="21-resolved-export-refuses-a-group-address-without-a-range"></a> <a id="22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback"></a> <a id="22-the-webdocker-deployment-target-has-no-authentication"></a> <a id="25-resolved-the-web-package-and-docker-frontend-stage-use-node-22"></a> <a id="27-tunnelclient-heartbeat-retry-has-a-narrow-race-condition--resolved"></a> <a id="28-tunnelclient-subscribers-receive-no-signal-when-the-tunnel-closes--resolved"></a> <a id="30-apiprojectdownload-has-no-frontend-caller"></a> <a id="30-apiprojectdownload-has-no-frontend-caller--resolved-2026-09-22"></a> <a id="32-routing_busy-is-logged-not-honored-by-routingclient--resolved"></a> <a id="33-routingclients-round-trip-test-transmitted-on-the-physical-lan-not-on-loopback--resolved-2026-09-20"></a> <a id="34-schema-21-export-drops-a-handful-of-known-but-unmapped-per-deviceper-line-attributes--resolved-2026-09-20"></a> <a id="35-device-creation-enrichmentissues-are-silently-dropped--resolved-2026-09-10"></a> <a id="4-round-trips-are-semantic-not-byte-exact"></a> <a id="4-round-trips-are-semantic-not-byte-exact--closed-2026-09-20-export-withdrawn"></a> <a id="49-project-documentation-export-has-an-in-application-preview-and-print-action"></a> <a id="49-project-documentation-export-has-no-in-application-print-preview"></a> <a id="5-exports-are-unsigned-and-ets-acceptance-is-untested"></a> <a id="5-exports-are-unsigned-and-ets-acceptance-is-untested--closed-2026-09-20-export-withdrawn"></a> <a id="50-project-documentation-export-has-no-section-selection"></a> <a id="50-project-documentation-export-has-section-selection-in-the-web-ui"></a> <a id="57-project-diff-cannot-compare-against-a-raw-knxproj"></a> <a id="57-raw-knxproj-comparison-is-available-on-the-cli-and-in-the-web-ui"></a> <a id="58-project-diff-has-an-opt-in-ci-exit-code-contract"></a> <a id="58-project-diff-has-no-ci-friendly-exit-nonzero-on-any-difference-flag"></a> <a id="59-project-diff-exposes-beforeafter-values-but-the-web-panel-does-not-render-them-yet"></a> <a id="59-project-diff-shows-beforeafter-values-in-cli-api-and-web-panel"></a> <a id="59-project-diffs-text-and-web-renderers-show-which-fields-changed-not-their-beforeafter-values-for-most-entity-types"></a> <a id="60-project-diffs-web-panel-shows-grouped-counts-only"></a> <a id="67-a-rejected-language-packs-own-reason-was-shown-untranslated-inside-a-translated-sentence--resolved-2026-09-14-t14"></a> <a id="80-a-project-can-be-created-from-scratch-in-the-ui--resolved-2026-09-16-goal-task-17"></a> <a id="81-new_project_impl-refuses-on-can-undo-not-on-is-dirty"></a> <a id="81-new_project_impl-refuses-on-can-undo-not-on-is-dirty--resolved"></a> <a id="83-the-from-scratch-launcher-is-browser-verified--resolved-2026-09-16-goal-task-17"></a> <a id="84-a-projects-group-address-style-can-be-chosen-and-afterwards-never-seen--resolved-2026-09-14-t4"></a> <a id="89-five-documented-spacetype-values-are-coarsened-to-buildingpart-on-import"></a> <a id="89-five-documented-spacetype-values-are-coarsened-to-buildingpart-on-import--resolved-2026-09-22"></a> <a id="91-a-running-bus-session-keeps-rendering-group-addresses-in-the-style-the-project-had-when-it-started"></a> <a id="91-a-running-bus-session-keeps-rendering-group-addresses-in-the-style-the-project-had-when-it-started--resolved"></a> <a id="92-commissioning-phase-2-is-verified-against-a-simulator-this-project-wrote-and-has-never-addressed-a-device"></a> <a id="96-a-browser-that-loses-the-import-response-cannot-get-the-project-back-without-reloading"></a>

## 1. Single-sample bias

**Limitation.** Everything verified about the `.knxproj` format comes from two
installations: the "Unser Zuhause" project (schema 11, ETS 4.1.8, and schema
23, ETS 6.3.7959.0 — the same project exported twice, risk R1) and, since
Session 7 (2026-09-06), the KNX Association "KV v2.5" demo project (schema
21, ETS 5.7 — a genuinely different installation).

**Cause.** Independent ETS5/ETS6 sample projects remain scarce. The second
export of the ETS4 reference project (RESEARCH §2.4/§3.3) confirms the
schema-11→23 format diff for one installation; the KV demo project (RESEARCH
§2.5/§3.4) independently confirms most of that same diff already exists at
schema 21, on unrelated data. Together they say nothing about schema 12, 13,
14, 20 or 22, and nothing about a differently-structured schema-23 project
(e.g. one using `Functions`, KNX Secure, or multiple areas/lines for real).

**Impact.** Schema 21 is implemented and round-trip verified against one
sample (the KV project) — a first import of a *structurally different*
schema-21 project (e.g. one with `Functions`, multiple areas/lines for real,
or a `GroupObjectTree` shape this session never saw) will still likely
produce unknown-construct entries. Schema 23's module-based handling
specifically remains *inferred, not evidenced* — it reuses schema 21's
measured element/attribute set by inference (`known.rs`'s `SCHEMA_23`
table, commented as such), with no independent module-using schema-23
sample to confirm the inference. Schema 12/13/14/20/22 remain fully
undocumented-by-evidence, unaffected by this work.

**Implemented (schema 21/23 import support).** Schema 21 import and export
shipped, round-trip verified on one sample (`knx-etsproj`'s
`importing_the_kv_schema_21_project_succeeds_with_zero_unknown_constructs`).
Schema 23 import shipped, with no round-trip claim; its module handling is
flagged as inferred both here and in `ImportReport.unsupported` at runtime.
[ADR-0013](adr/0013-module-instance-representation.md) and
[ADR-0014](adr/0014-group-object-tree-authoritative-source.md) record the
design decisions this rests on.

**Lifted when.** A second, independent, module-using schema-21 or schema-23
sample project has been imported and its unknown-construct report
reconciled to empty — this would upgrade schema 23's module handling from
inferred to evidenced, and schema 21's claim from one-sample to
cross-validated. Schema 12/13/14/20/22 still need their own first sample
each, unrelated to this upgrade.

## 2. No authoritative XSD is publicly available

**Limitation.** Imports are validated structurally, not against a schema (risk
R2). There *is* a validation stage — `knx_etsproj::validate`, stage 4 of the
import, between parse and mapping, exactly where `CLAUDE.md`'s data-flow
prescribes it — but it enforces a hand-built list of structural rules, not
conformance to anything authoritative, and that list is deliberately bounded by
what the installed corpus can attest.

**Cause.** The official schemas ship with the Manufacturer Tool via the KNX
GitLab account, which requires KNX membership **[D]**. Without them there is no
document to check a file *against*, so every rule the importer applies has to be
derived from real files and argued for one at a time.

**Impact.** We still cannot tell "this file is invalid" from "this file uses
something we do not know", and a malformed file is read as far as it parses,
with the rest reported rather than rejected. That is the tolerant-parser
half, and it is unchanged. What is narrower than this entry used to claim is
the validation half. As of **T06 (2026-09-21)** stage 4 checks:

* duplicate `@Id` across `Area`, `Line`, `DeviceInstance`, `BinaryData`,
  `GroupRange`, `GroupAddress` and `BuildingPart` — an error, because
  `map.rs` keys each of those by that string and a repeat silently collapses
  two entities into one identity;
* dangling communication-object → group-address links, in **both** spellings:
  schema 11's `Connectors/Send|Receive/@GroupAddressRefId` and schema ≥21's
  flat `Links` attribute of short ids — an error;
* two devices on one individual address, two group addresses on one address,
  and a group address outside its enclosing `GroupRange`'s bounds — warnings,
  since the mapper can use all three.

Validation never modifies the document and never aborts an import: a file that
parses still imports, and every problem above is a report entry.

**How bounded "bounded by the corpus" is.** All three corpus projects validate
clean **[V]** — 0 errors and 0 warnings each, for "Unser Zuhause" at schema 11
and schema 23 and for the KNX Association "KV v2.5" demo at schema 21. The
checks are therefore motivated by measured *coverage*, not by measured
violations, and two measurements from T06 are worth keeping:

* Before T06, stage 4 resolved 596 of the corpus's 1,218 communication-object
  links **[V]** — every one of the 622 written in schema ≥21's `Links` form was
  invisible to it, so for any project a current ETS writes, the stage performed
  no referential check whatsoever. It now resolves both forms.
* `ModuleInstance/@Id` is *not* checked for uniqueness, and that is a
  measurement rather than an omission: the KV project carries 32 of them of
  which only 16 are distinct **[V]**, because three of its four devices run one
  application program and repeat its module-instance ids verbatim. The id is
  device-scoped. Checking it would invent 16 errors about a valid file.

Two classes are deliberately left to other stages so that each has exactly one
reporting channel. `Installation/@DefaultLine`, `BuildingPart/@DefaultLine` and
`BuildingPart`'s `DeviceInstanceRef`s are reported by the mapper as
`MapProblemDetail::UnresolvedReference` — the corpus does contain an
unresolvable case (the KV project writes `DefaultLine=""` **[V]**), and it is
already reported. Anything requiring the manufacturer/application-program
database is reported by `knx_productdb::enrich`: `knx-etsproj` does not depend
on `knx-productdb`, stage 4 is a pure function of one `SourceDocument`, and
product resolution is a property of the machine's installed database rather
than of the file.

**Lifted when.** Authoritative schemas become available to the project. Note
that the tolerant parser would still be worth keeping — it is what turns a new
schema version into a report instead of a crash — and so would the structural
checks: an XSD says a `Links` attribute is a string of the right shape, not
that the group address it names exists.

## 3. Device parameters are preserved but not interpreted

**AR07 validation candidate (2026-10-02).** Nonfinite Float bounds are rejected
before comparison rather than letting a NaN declaration bypass an inclusive
limit. Synthetic RED101 reproduced; targeted Float6/0/0 and HTTP34/0/0 pass,
with ten lower/upper metadata cases, nonempty project equality, retained raw
source and independent sibling edits. Finite-value parsing policy and stored
lexemes unchanged. Separate in-session review has no blocking bounded findings;
public baseline8/8, workspace2926/0/164, 615 frozen inputs and 17 shadow bindings
equal. Both compiled min/max guard mutants caught at unit/HTTP, canonical source
restored. Frozen owned checkpoint18/18, workspace2926/0/164, Web1559 and
Chromium mock61 pass; six selected private Dynamic tests6/0/0 without genuine
skips and all103 original identities/hashes unchanged. No private raw logs.
Actual commissioning/Float mergebc5999c1 passes20/20, workspace2931/0/164,
Web1559/Chromium61, private Dynamic6/0/0 + offline SimTunnel HTTP13/0/0,
all108 originals unchanged/615 frozen inputs/17 equal bindings. Scoped guard
publishedda3bc947, local/live/fetched refs equal0/0 and owned bytes read back;
broader resource budgeting/provenance audit stays open;
no Float encoding, complete type-semantics, native SQL/WAL or ETS claim.

**Limitation.** All 1390 `ParameterInstanceRef` values in the reference project
are imported, stored and exported unchanged. As of **T18 slice 4
(2026-09-12)** a module-scoped (per-channel) value is not just read and
displayed correctly but also *editable*, whenever its section has exactly
one authoritative `ModuleInstance` (risk R3, now closed for that case — see
below for exactly what still is not).

**Cause.** Parameter visibility and semantics are driven by the `Dynamic`
tree. Its `choose`/`when` *value* grammar (`@test`) is now documented
(RESEARCH §4.3, Session 4 spike, 2026-09-11); the tree's *structural*
grammar (`Channel`, `ParameterBlock`, `choose`, `When_t`,
`ChannelIndependentBlock`) remains corpus-observed only. **T18's first
slice (2026-09-11)** parses and stores that tree losslessly in
`knx-productdb` (schema v3, `dynamic_node` table, backfilled into existing
databases) and adds a pure, headless evaluator
(`knx_productdb::dynamic::evaluate`) that turns a stored tree plus a
parameter-value map into the active `ParameterRef`/`ComObjectRef` sets,
with every unmatched, missing, unresolved or unrecognized case reported as
a diagnostic rather than guessed. **T18 slice 2 (2026-09-11)** taught the
evaluator to follow a `Module` node into its referenced `ModuleDef`'s own
stored tree (`knx_productdb::dynamic::{evaluate, load_program_trees,
ProgramTrees}`), so a modular application program's active
`ParameterRef`/`ComObjectRef` set is complete rather than truncated at the
module boundary. Every activation and diagnostic is qualified by a
`ModuleScope` naming the instantiating `Module`, so N sibling `Module`s
instantiating one `ModuleDef` produce N separate results, not one
collapsed into another. `Diagnostic::ModuleNotExpanded` no longer exists;
`ModuleDefNotFound` (no `@RefId`, or the named `ModuleDef` has no stored
tree) and `NestedModuleNotExpanded` (nesting rejected by policy, one level
only, design D15) replace it. Confirmed as a corpus regression: zero
`ModuleDefNotFound` and zero `NestedModuleNotExpanded` across the four
installed `.knxprod` archives; for `prod3`'s three module-bearing programs
specifically, activation totals grow from 22/18/14 (program tree only) to
382/258/134 (expanded). **This is scoped to `prod3` only** — RESEARCH
§4.4 Q7 lists seven module-bearing programs, but the other four live in
the `kv25` demo `.knxproj`, which these corpus tests do not install;
nothing here is a claim about `kv25`. (`Diagnostic::NestedModuleNotExpanded`
itself no longer exists as of task 11 below — nesting is now expanded, not
refused by policy; this paragraph is kept as the dated historical record
of what slice 2 shipped.)

**T18 slice 3 (2026-09-11)** wires the evaluator into a real editor:
`GET`/`POST /api/device/{id}/parameters` (`apps/knx-server`, DTOs and
decisions D20-D26,
[design](superpowers/specs/2026-09-11-parameter-editor-design.md)) plus a
web panel (`apps/knx-web/src/ParameterPanel.tsx`). A write to a top-level
field goes through `knx_core::Command::SetParameterValue` (undo/redo via
`RestoreParameterValue`), is validated against the program's declared
`parameter_ref`/`parameter`/`parameter_type` chain before the command is
even built, and the same response carries the evaluator's freshly
recomputed activation set — no second `GET` needed to see which other
fields or communication objects became active.

**Correction against the corpus, this revision.** An earlier draft of
this document (and of the design that preceded it) assumed
`ParameterInstance` had "nowhere to store" a per-channel value. That was
wrong, and re-measuring changes what this limitation says: ETS already
encodes the `Module` instantiation *inside* the stored `ets_id` string
itself (`<Module/@Id>_MI-<k>_<declared ParameterRef's own suffix>`), and
`ParameterInstance` (`crates/knx-store`, keyed by `(device, ets_id)`,
unmodified by this slice) already stores exactly that shape in our own
corpus today — the KV v2.5 demo project's `ParameterInstance` table holds
5 distinct values (17, 33, 49, 32, 48) for the single declared
`ParameterRef` `M-00FA_A-2504-10-C071_MD-2_P-1_R-1`, one per `Module`
instantiation. This slice's decomposition (D21) recognizes that shape and
D22/D23 surface each value in its own per-channel section, so a
module-scoped value now *reads and displays correctly* — it is not a
storage gap that happens to be unaddressed; it never was one.

**T18 slice 4, module-scoped editing (2026-09-12,
[design](superpowers/specs/2026-09-12-module-scoped-editing-design.md),
D35-D43) closes the write side (a) and the `choose`-evaluation side (b)
below name.** `knx-productdb`'s `ValueMap` gained a scope dimension
(D35/D36): a value stored for one `Module` instantiation is visible to
that instantiation only, falls back to the program default, and never
leaks to a sibling. `knx_core::ModuleInstance` now retains the project's
verbatim `ModuleInstance/@Id` (`instance_ets_id`, D38, store schema 6,
[DATA_MODEL.md §11](DATA_MODEL.md#11-versioning-and-migration)), which the
server uses to reconstruct the exact id a write must target — never
guessed, per section (D39). `apps/knx-web/src/ParameterPanel.tsx` writes
that server-named id instead of the declared one (D43).

**What remains limited, restated accurately rather than smoothed over:**

- **(a) Module-scoped fields are editable when, and only when, their
  section has exactly one authoritative `ModuleInstance` (D38-D39).**
  Still read-only, each for its own reported reason: two or more
  `ModuleInstance`s sharing one `RefId` — genuinely repeated
  instantiation, told apart only by guessing — [§68](#68-repeated-module-instantiation-is-refused-not-supported);
  a `Module` with no `@Id` at all — [§69](#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance);
  and a project imported before store schema 6, whose every
  `ModuleInstance` has `instance_ets_id == ""` until re-imported —
  [§71](#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with).
  Separately, the *write-validation* rule narrowed while closing this: a
  field must now be a currently-shown panel field with a non-null write
  target, not merely a declared id — [§70](#70-writing-a-declared-but-not-currently-shown-parameter-is-now-refused).
- **(b) A module-scoped value now reaches `ValueMap` (D35/D36/D42), so a
  `choose` controlled by a module-scoped parameter evaluates against
  *that channel's own* stored value.** Two sibling channels holding
  different values for the same declared parameter can therefore show
  genuinely different active field sets — proven, not only designed:
  `crates/knx-productdb/tests/dynamic_tree.rs` and
  `apps/knx-server/tests/http_parameter_panel.rs` each carry a fixture
  where two channels' `choose` picks a different branch (T18 slice 4,
  Task 5). **Design D16 (all instantiations of one `ModuleDef` evaluate
  against identical parameter values) is closed in its general form** — it
  now holds only when the channels' own stored values agree, or none
  exist; that was always the narrower, read-side meaning slice 3 gave it,
  and slice 4 makes it true on the write/activation side too, for the
  case D38-D39 can authorize.
- **Nested modules are now expanded, bounded, with cycle detection
  (goal.md T18, task 11, 2026-09-14, design D44-D46,
  [design addendum](superpowers/specs/2026-09-11-module-expansion-design.md#addendum-goalmd-t18-task-11-d15-superseded--bounded-recursive-expansion)).**
  `Diagnostic::NestedModuleNotExpanded` (design D15, one-level policy) no
  longer exists. A `Module` found inside an already-expanded `ModuleDef`'s
  own tree is now walked recursively, up to
  `evaluate::MAX_MODULE_NESTING_DEPTH = 16` (**[A]**, no Standard source
  states a bound — see the constant's own doc comment). A `ModuleDef`
  that, directly or through intermediate `ModuleDef`s, names itself again
  is refused as `Diagnostic::ModuleCycleDetected`, checked by scanning the
  whole ancestor chain, not just the immediate parent; a non-cyclic chain
  past the bound is refused as `Diagnostic::ModuleNestingTooDeep`. A third
  bound, `evaluate::MAX_MODULE_EXPANSIONS = 100_000` (added fix round 1,
  2026-09-14), caps *total* `Module` expansions per `evaluate` call — the
  depth bound alone only refuses one chain going too deep, not many
  shallow chains multiplying by fan-out, which a measured probe showed
  can reach millions of activations and gigabytes of memory well within
  the depth bound; exceeding it is refused as
  `Diagnostic::ModuleExpansionBudgetExhausted`. None of the three cases
  panics, loops, or truncates silently. **Measured against the
  installed corpus (two independent ways, RESEARCH.md §4.4 addendum): zero
  products nest a `Module` inside a `ModuleDef`'s own tree** — the
  capability is proven only by synthetic unit tests, not by a real
  product, and would need re-verification against a corpus sample that
  actually nests before being trusted on one.
  - **Unattested scoped-value collision risk.** `ValueMap`'s scoped lookup
    key is qualified by the *innermost* `module_id` (per-instantiation
    values, design D35/D36), not the full ancestor chain. By analogy with
    RESEARCH.md §4.4 Q5's finding for single-level `ModuleDef`s (inner
    `@Id`s are reused verbatim across sibling instantiations), two
    different outer instantiations of a nested `ModuleDef` whose inner
    `Module/@Id` happens to repeat verbatim could theoretically collide on
    scoped values. This is unattested — the corpus has zero nesting to
    check it against — and is called out here rather than silently
    assumed safe.
  - **Parameter HTTP scope identity — bounded backend correction accepted
    (2026-10-03).** The
    original 2026-09-14 finding was that correctly separated nameless nested
    sections could have identical `moduleNode`/`moduleId`/`moduleDefId`
    HTTP scopes, causing client-side diagnostic misattribution. AR07's
    public synthetic RED reproduced exactly that wire collision on
    `e9707794`, without changing Core or the section grouping.
    [ADR-0063](adr/0063-parameter-scopes-preserve-evaluation-identity.md)
    now adds response-local, outermost-first `nodeChain` through the existing
    Core accessor. Public HTTP 38/0/0, server library 205/0/2 and three
    compiled behavioral mutations/restoration verify the candidate backend
    contract. Actual3711c4f7 passes20/20 independently: ordinary Rust2957/0/164,
    Web1665/Chromium61, selected private Dynamic6/0/0 and in-memory SimTunnel13/0/0;
    all624 committed inputs,17 bindings and420 originals unchanged. Publication
    of source/acceptance3a8b66f4 is read back with all11 owned artifacts and624
    inputs exact. The420-file hash inventory is not420 parsed test cases. This is not
    a durable ETS identity, a write target or a change to scoped-value rules.
    **UI residue remains:** the UI owner's manual `ModuleScope` interface and
    `ParameterPanel.tsx`'s legacy `sameScope()` still require adoption and
    regression evidence; the backend field alone does not fix their matching.
    Real nested manufacturer evidence remains absent from the historical
    corpus measurement, not established by these synthetic cases.
  - **Fixed in fix round 1 (2026-09-14): two nesting chains sharing a
    `module_node` no longer collide into one section.** Before this fix,
    `apps/knx-server`'s parameter-panel grouping keyed sections on the
    flat `Option<i64>` `module_node` D18 used, even after `ScopeKey`
    itself was widened to the full `Vec<i64>` ancestor chain. Because
    `dynamic_node.node_id` resets per `(program_id, module_def_id)` tree,
    two distinct nesting chains can reuse the same `module_node` at the
    same depth under different ancestors — this branch's own test
    `two_nesting_chains_sharing_a_module_node_do_not_merge_into_one_section`
    (`apps/knx-server/tests/http_parameter_panel.rs`) constructs exactly
    that pair. The two sections merged, the S4
    duplicate-module-id backstop could not fire (it counts sections, and
    the collision happened a step earlier), and `write_ets_id`
    reconstruction could point the losing channel's fields at the
    winner's module instance. This was a genuine collision, observable in
    code, not merely a truncation — `section_order`/`sections_by_key` now
    key on `Option<Vec<i64>>` (`ModuleScope::node_chain()`, made `pub` for
    this), the same key `evaluate`'s own dedup already used. Also
    unattested against real data — the corpus has zero nesting to trigger
    it — but no longer latent in the code either.
  - **`Module` arguments are no longer inert** (task 12, 2026-09-14); see
    the narrowed entry below.
- **Argument values (`NumericArg`/`TextArg`) are interpreted for text
  substitution and for nothing else** (narrowed 2026-09-14, goal-completion
  task 12, design D47-D51). What changed: a `ModuleDef`'s declared
  arguments are stored (`module_def_argument`), a `Module`'s bindings get a
  `dynamic_node.value` column instead of the unparseable `extra` blob, and
  `{{Name}}` placeholders in an activated `Channel`/`ParameterBlock`/
  `ParameterSeparator` `@Text` are resolved against the instantiating
  `Module`'s bindings, so two instantiations of one `ModuleDef` now
  evaluate to two different labels
  (`Activation::labels`; closed by
  `an_argument_value_changes_the_evaluated_label_of_each_module_instantiation`
  and, on real corpus data, `corpus_argument_measurement_task_12`, both in
  `crates/knx-productdb/tests/dynamic_tree.rs`).
  **What remains unimplemented, precisely:** the *memory-allocation* facet
  — `Argument/@Allocates` is stored and not interpreted, and the two
  constructs that consume a numeric argument, `Memory/@BaseOffset` (705
  corpus occurrences `[V]`) and `ComObject/@BaseNumber` (157 `[V]`), are
  `Static`-side and unmodelled, so a module's parameters and communication
  objects still carry their `ModuleDef`-local memory placement and object
  numbers rather than their per-instance ones. The purely numeric
  placeholder family (`{{0}}`, 948 corpus occurrences `[V]`, tied to
  `TextParameterRefId`) is also not substituted — it is left verbatim and
  is deliberately *not* reported as an unresolved argument, because it
  never was one. `choose` still never branches on an argument (RESEARCH
  §4.4 Q3), so activation sets are unchanged by any of this.
- **`AllocatorRef`** (`ModuleDefArgType_t`'s third argument-type facet)
  stays unattested and unimplemented — **0 occurrences** across every
  readable member of every archive under `OriginalData/ProductDatabases`
  and `OriginalData/DemoProjects` (`[V]`, re-measured 2026-09-14 and
  re-measured on every run by `corpus_argument_measurement_task_12`), and
  0 hits in either KNX specification knowledge base
  (`knx_spec_kb_programming.sqlite`, 2,207 facts / 27 PDFs;
  `knx_spec_kb_full179_clean.sqlite`, 16,536 facts / 177 PDFs) across
  `content`, `title`, `keywords` and `evidenceText`. Changed in task 12
  only in that it is now **reported** rather than ignored: a declaration
  spelling it produces `Diagnostic::UnsupportedModuleArgumentKind` per
  instantiation instead of passing unremarked. Nothing about its semantics
  is guessed at.
- **`Access` has no attested correlation and is not used for write
  gating.** RESEARCH §4.3 found no usable correlation for `Access`
  (`Access="None"` alongside a `Memory` child came back roughly 50/50 in
  the corpus, `Visible` never observed at all); the editor shows `access`
  verbatim and never uses it to block, hide or grey out a write.
- **T18 slice 5 (2026-09-14) gives `Float`, `Text` and `IPAddress` real
  validation; `Picture` and `Raw` stay on the non-empty-string floor,
  documented as a gap rather than guessed shut.** `Float` now rejects
  non-finite input and anything outside the program's own
  `minInclusive`/`maxInclusive` (`TypeFloat`'s own attributes,
  corpus-observed: MDT `M-0083_A-0317-31-7DC6_PT-2ByteFloatTemp` carries
  `<TypeFloat Encoding="DPT 9" minInclusive="-100" maxInclusive="200"/>`)
  when the program declares bounds, and just finiteness when it does not.
  What `Float` deliberately does *not* enforce is `Value_t`'s own stored
  encoding for a `TypeFloat` — scientific notation with 16 significant
  digits and a three-digit exponent, the shape `value.ToString("E15", ...)`
  produces (*Project Schema23 v01.00.00* §1.1.3.19, p. 30/64) **[D]**.
  That is the wire format, not what a person types into a form field, and
  this design stores the raw string verbatim rather than reformatting it.
  So any finite number in ordinary decimal or scientific notation is
  accepted, and the E15 question stays open — the same kind of deliberate
  narrowing as the IPv6 decision below, in the opposite direction:
  `IPAddress` accepts less than reality allows, `Float` accepts more than
  the wire format specifies. Also unenforced: `TypeFloat/@Increment`
  (corpus-observed alongside `minInclusive`/`maxInclusive` on four of six
  distinct `TypeFloat` shapes, e.g. `Increment="0.1"`), a real acceptance
  constraint — with `minInclusive="1"` `maxInclusive="120"`
  `Increment="0.1"`, the validator accepts `1.05` today. T18 fix round 1
  (2026-09-14) made `Increment` visible — `insert_parameter_type` now
  reports it as an unmodelled attribute rather than dropping it with no
  record at all — but visibility is not enforcement; the value is still
  read nowhere and the bound stays unchecked.
  `Text` now rejects a value whose UTF-8 byte length exceeds the
  program's own `SizeInBit` (`TypeText`'s own attribute, corpus-observed:
  the same program's `<TypeText SizeInBit="240"/>` and
  `SizeInBit="640"`) divided by 8, when declared; a `TypeText` with no
  `SizeInBit` — which happens — gets no length cap rather than a
  fabricated one. Both reuse `parameter_type`'s existing
  `min_inclusive`/`max_inclusive`/`size_in_bit` columns; no schema
  change, no change to how the value is stored (the raw string the user
  typed is still what gets written verbatim into the command and the
  exported `Value` attribute). The byte-length check is a proxy, not the
  real rule, and it is a strict one: UTF-8 costs two bytes per umlaut, so
  a 30-character German label — the norm in this corpus, not an edge
  case — can be rejected by a `SizeInBit="240"` field (30 bytes) that ETS
  itself would accept, since ETS's own storage is not attested to be
  UTF-8-per-character (T18 fix round 1, item 5; the check itself is
  intentionally left as-is — conservative-direction-only, per its own
  comment in `apps/knx-server/src/domain.rs`).
  `IPAddress` accepts both IPv4 and IPv6, on schema evidence: KNX Project
  Schema23 v01.00.00 §1.1.3.19 (`simpleType Value_t`) documents
  `TypeIPAddress` as "IPv4 addresses: decimal dotted notation" and
  "IPv6 addresses: eight groups of four hexadecimal digits, separated by
  colons, e.g. 2001:0db8:85a3:0000:0000:8a2e:0370:7334" — both forms, in
  the same sentence, with a worked IPv6 example `[D]`. IPv4 is validated
  with `std::net::Ipv4Addr::from_str`, which matches the schema's own
  `Ipv4Address_t` restriction pattern (§1.1.3.21) closely enough
  (rejects leading zeroes and out-of-range octets, same as the pattern
  would). IPv6 is deliberately checked against only the schema's literal
  documented shape — eight colon-separated groups of exactly four hex
  digits — not the fuller RFC 4291 grammar `std::net::Ipv6Addr` would
  accept (compression via `::`, elided leading zeroes, embedded IPv4
  tails). The schema names one form; accepting a wider one would be
  inventing a rule the Standard does not state, so the compressed forms
  are rejected even though a real IPv6 address may use them — narrower
  than necessary is the defensible choice here, not the complete one.
  `Picture`, `Raw` and (PDB-9, ADR-0041) `Color` get no format check
  beyond non-empty, because no format exists to check against:
  `Value_t`'s encoding table (§1.1.3.19) lists `TypeNone`, `TypeText`,
  `TypeNumber`, `TypeFloat`, `TypeRestriction`, `TypeTime`, `TypeDate`,
  `TypeIPAddress` and `TypeAllocatorRefId` — `TypePicture`, `TypeRawData`
  and `TypeColor` are absent from it entirely `[D]`. `Time` (PDB-9) is
  validated like `Number`, exactly as `Value_t` says ("Same as
  TypeNumber"). *Correction (2026-09-27, PDB-9):* an earlier, smaller sweep
  reported zero `<TypePicture>`/`<TypeRawData>` elements (why it missed
  them is not established `[A]`); the PDB-9 read-only scan of 304 distinct
  programs finds
  `TypePicture` 1,118, `TypeColor` 115 and `TypeRawData` 3 times. The
  conclusion stands — those are declarations, not a documented value
  encoding — and neither knx-spec-kb knowledge base nor
  xknxproject's own source turned up a documented encoding. The schema
  does use `xs:base64Binary` for other binary attributes elsewhere
  (`SerialNumber`, `LoadedImage`, `PasswordHash`) `[D]`, which was
  considered as a stand-in rule for Picture/Raw and rejected: that
  convention is attested for those specific attributes, not for these
  two parameter kinds, and guessing it across would risk rejecting a
  legitimate value on a rule invented rather than found. What both kinds
  do gain is a reject on any character outside XML 1.0's `Char`
  production (`[V]`: this workspace's pinned `quick-xml 0.42.0` does not
  filter or escape such characters on write, confirmed by a standalone
  test), applied to every kind's fallback path too — a value containing
  a raw control character would otherwise corrupt the exported
  `.knxproj` rather than merely being semantically unchecked.

**Impact.** Device configuration for a top-level field, and now for a
module-scoped field with exactly one authoritative instance, can be done
here, with the evaluator's own diagnostics surfaced in the same response —
including for a channel whose own value flips a `choose`. Editing still has
to happen in ETS for a genuinely repeated module (two or more
`ModuleInstance`s sharing one `RefId`), for a `Module` with no `@Id`, and
for a project that has not yet been re-imported since store schema 6
(§§68-71).

**Lifted when.** For [§68](#68-repeated-module-instantiation-is-refused-not-supported):
RESEARCH.md's sharpest unknown #1 (what a `ModuleInstance/@RepeatIndex`
above `"1"` means) would have to be settled first — inventing the key on
present evidence is exactly what D40 refuses to do. For
[§69](#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance): only
the application program itself can supply the missing `@Id`; nothing here
can invent one. [§71](#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with)
lifts itself, per row, the moment the project is re-imported. The
no-match-branch policy the evaluator implements (nothing under an
unmatched `choose` is active) is itself an inference (RESEARCH §4.3,
finding 2), not a documented rule — noted here, not hidden, and unaffected
by this slice.

## 6. Devices behind vendor plug-in DLLs

**Limitation.** Devices whose configuration depends on a vendor plug-in DLL
cannot be configured by this application (risk R5).

**Cause.** The configuration logic lives inside a Windows PE binary shipped in
the project file — for example `econEts3.dll` (641 KB) and `FastDownload.dll`
(160 KB) in the reference project [V].

**Impact.** Affected devices are detected, marked read-only, and reported in
the import report's `unsupported` list. Their data round-trips unchanged; it
simply cannot be edited here.

**Lifted when.** Never by us. Executing vendor binaries is not something this
application does, on any platform.

<a id="7-commissioning-and-device-download-are-required-but-blocked"></a>
## 7. Commissioning: a verified `070nh` memory path, not general device support

**Scope (ADR-0048/0049, verified 2026-09-29/30).** `knx device download`
and the Web download tab, plus individual-address programming, are real
product commands. They require the identified target, a plan, a
device-specific confirmation phrase and exclusive gateway access. Complete
and all three partial download scopes were exercised on one MDT `1.1.67`
(`0701h`, `A-0027-15-0BAC`) with read-back; an address change and return,
and an address reset with guarded recovery, were also exercised there.
A successful byte read-back does **not** confirm the closing Basic Restart:
that device does not acknowledge it; the result is explicitly
`RestartOutcome::Unconfirmed`. Other devices, application versions and
masks have not acquired this evidence. The property-based `Downloader`
remains simulator-only. Full traces and the chronological corrections are
in [RESEARCH §19](RESEARCH.md) and the
[implementation log](IMPLEMENTATION_STATUS.md), not an open task list here.

**What still limits a download.** The product-database coverage probe could
plan 55 of 181 `0701h`/`0705h` application programs at their defaults from
103 packages; a plan is not a live verification. Other masks and unsupported
procedures are refused by name. `Mask` semantics and parts of the
BIM-M112 memory-procedure interpretation remain inference, identified as
`[A]` in RESEARCH §19.1–§19.3. `LdCtrlMerge`/`MergedProcedure`, unsupported
procedure steps, property-placed/unaligned parameter values, unsupported
parameter encodings and types, ambiguous object priority/ReadOnInit and
other unproved image shapes are refused rather than guessed.
`PID_GROUP_RESPONSER_TABLE`, `A_Key_Write`, some master resets and RF writes
have separate boundaries below (§110, §112, §141, §143–§144).

**Backup is bounded.** Before a permitted memory download's first mutation,
the executor persists the overwritten memory regions and affected load
states. A complete and parameters-only backup/restore roundtrip ran on
`1.1.67`; failing-backup refusal and group-address-partial restore remain
simulator-only. This is not a full image and cannot recover property values,
keys outside those regions, or arbitrary non-Loaded states. See the
commissioning-readiness section above and [ADR-0049](adr/0049-download-readiness-is-per-plan-and-backups-are-pre-write.md).
Since 2026-10-01, a restore also refuses missing, duplicate or extraneous
load-state machine records relative to its download plan, even when the
saved memory-region lengths still match. This is offline-tested validation
of the existing bounded backup, not evidence of complete address-write
recovery or support for previously untested devices.
Returned executor errors now close a still-open device management connection
best-effort (2026-10-01, simulator-tested), preserving the original failure
and existing backup rather than automatically restoring or retrying. This
does not prove remote receipt of disconnect and does not cover a dropped
future, process termination or a power loss during the run.
Server polling now recognises a finished worker with no terminal result as
failed, preserving its progress and backup path. `written:partially` after a
mutation step starts is a conservative unknown outcome, not proof that any
telegram was sent or that device recovery succeeded. An already witnessed
device result survives a later cleanup panic, but does not prove disconnect.
The tunnel reservation remains held while cleanup runs, even if a `join`
wait is cancelled. These offline-tested checks last only for this server
process; there is no new cancel route, automatic recovery or crash journal.

**Live safety and compatibility remain narrow.** `1.1.220` is an excluded
alarm panel. No prior approval carries over to a new target, write scope or
experiment. The gateway allows one tunnel; a refused plan sends nothing.
No general ETS commissioning parity, every-vendor download claim, automatic
rollback or RF hardware support is established by one device.

## 8. KNX Secure is not implemented

**Limitation.** No Data Secure, no IP Secure, no keyring handling (RESEARCH
§9).

**Cause.** No sample key material was available in Session 0, so nothing about
it could be verified.

**Impact.** Secured installations cannot be fully represented or monitored.

**Lifted when.** Sample material and a real secured installation are available.
The isolation boundary already exists — `knx-secure` is a separate crate with
no dependency on `knx-core` — precisely so that this can be built without
retrofitting secret handling into the model
([ADR-0008](adr/0008-key-material-isolation.md)).

**Update, 2026-09-11.** Asked whether T19 (KNX Secure) should wait until
hardware/sample key material exist, the user answered "raus erstmal, aber
als limitation dokumentieren" — deferred for now, but document it as a
limitation. This entry already does; nothing here is rejected, only deferred
behind the precondition above. See [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s
**T19** for the tracked task.

## 9. Project files are not diffable

**Limitation.** A project is a SQLite file, so version control tools cannot
show a meaningful diff of it.

**Cause.** A deliberate trade for transactional, incremental saving and indexed
access ([ADR-0003](adr/0003-sqlite-project-format.md)).

**Impact.** Projects can be versioned as binaries only. Reviewing what changed
between two versions requires the application.

**Lifted when.** A textual export and import format is added, if a demonstrated
need arises. It is deliberately not built speculatively.

## 11. `.knxprod` support is evidenced for schemes 11, 12, 13, 14, 20, and exact-namespace 21

**Limitation.** Manufacturer product files in the `.knxprod` container are
readable, as a standalone package independent of any `.knxproj`, for master
data schemes 11, 12, 13, 14, 20 and exact-namespace 21
(`knx_productdb::install_package`). Scheme-21 acceptance is evidenced by
synthetic fixtures and the passing read-only corpus matrix; its additional
fields remain retained/report-only. Schemes 15-19, 22, and any scheme not
listed here remain unmeasured
as a *standalone package* — they can still reach the product database bundled
inside a `.knxproj` that already contains them (see Impact below;
`knx_productdb::ingest_file` performs no scheme/namespace gating). `.vd2`,
a pre-2013 ETS2-era legacy container (SFX/`.vd_`-style, not
the same ZIP/XML family as `.knxprod`/`.knxproj` at all — confirmed by
inspection, it has no `knx_master.xml`), is explicitly and permanently
rejected: `PackageError::LegacyVd2 { sha256, len }` → `"legacy .vd2 product
data is unsupported (sha256 <64 hex chars>, <len> bytes)"`, identified by
filename suffix alone — the archive is never opened as a ZIP, never
decrypted, never parsed. As of 2026-09-11 its bytes are hashed (bounded by
the same `MAX_PACKAGE_SIZE` guard every package is subject to) so the
rejection carries the archive's hash and length as evidence; before that
date the filename check reported no evidence at all. This is a named,
external blocker (a genuinely different, undocumented legacy format), not
an untested general failure.

ZIP member-name encoding is no longer an incidental blocker. The installer
reads the central-directory encoding flag, requires valid UTF-8 when bit 11 is
set, and otherwise follows the ZIP-defined CP437 path. Local/central identity
must agree, and Unicode-path overrides require a supported version, matching
CRC and valid UTF-8 before every path and duplicate check
([ADR-0034](adr/0034-zip-member-names-follow-declared-encoding.md)).
The eleven-package aggregate that previously stopped at the raw-name UTF-8
check now installs in the local gated regression. Only its count and aggregate
identity commitment are public. This removes that one container-level rejection only; it does not claim lossless
interpretation of manufacturer semantics.

**Measured update, 2026-09-24.** The first read-only compatibility-matrix run
covers 115 package instances (113 unique hashes): scheme 11 (48), scheme 12
(1), scheme 13 (4), scheme 14 (3), scheme 20 (56), and scheme 21 (3). It
records 115 isolated installs and, in deterministic shared order, 113 installs
plus 2 exact-byte deduplications. The final gate passes, including its pinned
23,347 isolated unknown rows, ten scheme-21 field frequencies and aggregate
identity/outcome commitment. These measurements show parser/persistence behavior,
not complete manufacturer semantics.
Public identity is aggregate. Its hash-ordered per-instance records contain
only ordinal, scheme and `{status}` for success/deduplication or
`{status, category}` for rejection; detailed per-instance report-count vectors
stay inside the aggregate commitment. No individual hash, private source name,
or manufacturer identity is published.
This replaces the earlier assumption that no standalone samples existed for
those schemes. Scheme-12/13/14 support is backed by representative synthetic
persistence/rollback tests and the eight real packages, but it does not
establish complete interpretation of all possible files. In particular, observed load-procedure and separator metadata remains
explicitly retained and reported rather than treated as executable semantics.
The targeted scheme-12/14 evidence pass rejects application-program XML deeper
than 1,024 nested elements so a size-bounded but adversarial document cannot
turn path tracking into unbounded memory growth. It also rejects more than
262,144 scanned element/attribute items and 64 MiB of cumulative rendered
path, value, and expanded namespace-name evidence, bounding reconciliation
memory independently of the 64-MiB XML-member limit.
The relational parsers still key attributes by local name, but collisions are
deterministic: an unqualified attribute takes precedence over any qualified
same-local-name attribute in either document order. The qualified values remain
separate namespace-qualified unknown evidence; they do not overwrite the KNX
value or create an order-dependent plain-name report.

Scheme-21 package XML is deliberately narrower than schemes 12/14 for foreign
extensions: every typed member must use the exact scheme-21 element namespace
and unqualified attributes. Other element namespaces or qualified attributes
cause a clear import error and no published package rows. The domain readers
dispatch on local names, so accepting those extensions could otherwise turn
foreign content into typed KNX rows. The three observed scheme-21 packages use
only the canonical namespace and unqualified attributes. Evidence for the
observed scheme-21 fields is retained and reported; it does not implement
load-procedure execution, RF/coupler behavior, or access-policy enforcement.

Note the scope: this is about standalone `.knxprod` *product packages*
(`knx products ingest`, `POST /api/catalog/install`,
`CatalogBrowser.tsx`'s install picker). Full `.knxproj` *project* import
still only has evidenced coverage at schema 11/21/23 (see
[COMPATIBILITY.md](COMPATIBILITY.md) §2/§3) — a `.knxproj` at schema 20 is
still an "expected but unverified" claim, not the same thing as this row's
now-verified scheme-20 `.knxprod` package support.

**Cause.** `.knxprod` is the same XML family as `.knxproj` (both root at
`knx_master.xml`, `http://knx.org/xml/project/{scheme}`, per *Project
Schema23 v01.00.00* §4.2.2-§4.2.3's MasterData/`M-iiii` layout); the earlier
assumption that all schemes ≥ 12 needed a still-unresolved encryption layer
(RESEARCH §10) has been disproven for schemes 11, 12, 13, 14, and 20
specifically. The
observed scheme-13 grammar adds no element or attribute names relative to the
measured scheme-11/20 union, and its four real packages pass isolated and shared
installation. Scheme-12/14-specific load-procedure and separator fields are
retained and reported with aggregate corpus counts, not interpreted as executable
semantics. The passing private matrix shows three scheme-21 packages install in isolation
and shared order under the exact namespace allowlist. Schemes 15-19/22 remain
unmeasured.

**Impact.** A manufacturer's standalone `.knxprod` at scheme 11, 12, 13, 14,
or 20 can be installed directly via `knx products ingest`, the HTTP endpoint,
or `CatalogBrowser.tsx`'s install picker, without needing a `.knxproj` that
bundles it. Scheme 21 is accepted only for the exact observed namespace and
has synthetic and observed parser/persistence evidence; its corpus gate passes.
A `.knxprod` at any other scheme, or a legacy `.vd2`, still has to reach the
product database another way (in practice, from a `.knxproj` that already
contains the application programs it references) or not at all for `.vd2`.

**Lifted when.** Scheme 21's exact-namespace parser/persistence acceptance is
already evidenced synthetically and by the passing corpus gate; remaining work
needs fixture-backed semantic coverage. For schemes
15-19/22, a sample is still required before making any claim. For `.vd2`:
never — it is a structurally different, pre-standard legacy container, not a
variant of the current format needing decryption.

Schemes 12-14 no longer belong to this unsupported set. Their acceptance does
not broaden any neighboring namespace and does not alter the permanent `.vd2`
decision.

**Superseded in part, 2026-09-23.** The current product-database completion
goal put evidenced scheme expansion back in scope. The permanent `.vd2`
decision remains unchanged; no scheme is enabled merely because a sample now
exists.

## 12. Manufacturer data resolution — one of three gaps closed (2026-09-20)

**Lifted (Session 4) for communication-object defaults.** `ProductRefId`
and `Hardware2ProgramRefId` now resolve: the shared product database
([ADR-0005](adr/0005-separate-product-database.md),
[ADR-0011](adr/0011-product-database-storage.md)) ingests `<M-xxxx>/*`
once, keyed by content hash, and `knx_productdb::enrich` fills a
communication object's `text`, `description`, `dpt`, all six flags and
`size`
from the application program wherever the instance itself left the slot
`Absent` (IMPORT_EXPORT §10). `ComObjectInstance` values now carry
`Program`/`ProgramRef` in addition to `Instance` where the source project
did not itself state a value.

Three gaps were named after that lift. Re-measured against the reference
project (goal-completion Task 4, 2026-09-20; every figure below is exact,
reproduced by `crates/knx-app/tests/enrichment_gap_measurement.rs`): one is
now closed, one was already smaller than its own headline made it sound
and is tracked precisely elsewhere, and one is deferred with its blocker
named plainly.

**Closed: a program value behind an `Empty` instance slot is no longer
invisible ([ADR-0027](adr/0027-program-defaults-side-table.md)).** 497 of
the reference project's 907 `ComObjectInstanceRef` elements carry
`DatapointType=""` — present, explicitly cleared, not unstated — spanning
23 of 36 devices; 82 more carry an empty `VisibleDescription`, and `text`
is never empty (a program always states one). Enrichment still never
writes into an `Empty` slot (ADR-0012's export-fidelity rule is unchanged
and re-verified by its own tests), but the program's own value behind that
slot — 122 of the 497 dpt cases and all 82 of the description cases
resolve to exactly one value; the remaining 375 dpt cases genuinely have
nothing behind them either — now lands in `Devices::program_defaults`, a
side table `knx-store` persists (`com_object_program_default`, schema 9)
and the exporter never reads. `ProgramDefaults` and its map replace
ADR-0012's originally-rejected "extend `Override<T>` into a layer stack"
alternative with an additive side table instead — see ADR-0027 for why.
**What this does not yet do:** no UI reads `program_defaults`; the value
is model- and store-level only, a decision recorded in ADR-0027 rather
than silently left undone.

**Narrower than it read: parameter interpretation.** The paragraph this
section used to carry duplicated [§3](#3-device-parameters-are-preserved-but-not-interpreted),
which is the authoritative, continuously-updated account and had already
moved past what this section still said. In short, as of §3's latest
entry: top-level parameter fields are read and writable (T18 slice 3);
module-scoped fields are also now read and writable, but only when their
section has exactly one authoritative `ModuleInstance` (T18 slice 4) — the
residue is three named, individually-tracked cases
([§68](#68-repeated-module-instantiation-is-refused-not-supported),
[§69](#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance),
[§71](#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with)),
not one undifferentiated "module-scoped editing" gap. Nested modules,
module arguments (text substitution only) and `AllocatorRef` reporting
have each moved since this section was last written too — §3 is where
all of that lives now; this section stops duplicating it and points there
instead, so the two cannot drift apart silently again.

**Deferred: an ambiguous, space-separated `DatapointType` list still fills
nothing.** `ComObjectRef/@DatapointType` can hold several acceptable
alternatives (RESEARCH §4.2, e.g. `"DPST-9-21 DPST-9-1"`). Enrichment
refuses to guess between them; it records `EnrichmentIssue::AmbiguousDpt`
and leaves the slot as it was — and must keep doing exactly that, per
CLAUDE.md's data-integrity rule: picking one alternative silently would be
invented compatibility presented as data. Re-measured: 107 issues raised
in total, but 85 of those land on a slot the instance already stated its
own value for (informational noise — nothing was ever going to be filled
there regardless), 0 land on `Empty`, and only 22 land on a genuinely
`Absent` slot that actually stays unfilled because of the ambiguity,
spread across 11 devices; 51 distinct `ComObjectRef`s carry an ambiguous
list in total. **Blocker.** The brief for this task floated one way to
close it — surface `AmbiguousDpt` to the user as a choice, rather than
resolve it automatically — but no mechanism to present or persist that
choice exists anywhere in the stack today: `EnrichmentReport` reaches the
CLI's summary line and, for device creation only, `apps/knx-server`'s
`CreationDiagnostic` ([§35](#35-device-creation-enrichmentissues-are-silently-dropped--resolved-2026-09-10)),
but a project-level *import*'s `EnrichmentReport` is not surfaced to
`apps/knx-web` at all, and there is no domain concept of a user-recorded
DPT choice to write it into even if it were. Building that (a new
surfacing path, a place to store the choice, a UI) is a design of its own
scope — deliberately not absorbed into this task, the same call §3 makes
for module-scoped editing's own residue. Given the narrow measured impact
(22 slots, 11 devices, of 907 communication objects), it did not rank
ahead of gap 2. *Lifted when* that design exists; re-guessing from a
linked group address's own datapoint type, floated in an earlier draft of
this section, was not re-considered this session and is not assumed to be
the answer.

**Impact.** A project opens completely and round-trips its manufacturer
data byte-for-byte. Communication-object defaults resolve where the
instance did not override them, and now also where it explicitly cleared
them and a program value exists — visible in the model, not written into
the exported file either way. A top-level or single-instance module-scoped
parameter value can be read and written from the parameter editor. An
ambiguous DPT list is visible in the product database and in
`EnrichmentReport`, but still fills nothing, and nothing in the current
stack lets a user resolve it by hand.

**Lifted when.** Gap 2 is closed. For the parameter-interpretation
residue, see §3's own "Lifted when". For the ambiguous-DPT gap, see the
blocker above.

## 13. Password-protected projects: ZipCrypto (ETS4/ETS5) is decrypted, AES (ETS6) is still refused

**Limitation.** A `.knxproj` whose project part is nested as `<P-xxxx>.zip`
(IMPORT_EXPORT §2) can now be opened two ways:

* `Container::open` (no password) still refuses it by name
  (`ContainerError::PasswordProtected`), unchanged from before.
* `Container::open_with_password(bytes, password)` decrypts a schema < 21
  (ETS4/ETS5) project protected with **ZipCrypto**, given the right
  password. A schema ≥ 21 (ETS6) project protected with **AES** is still
  refused by name (`ContainerError::UnsupportedEncryption`), not
  attempted — that half of this limitation is unchanged.

Read only, both ways: nothing in this repository writes a ZipCrypto- or
AES-protected entry. `knx_secure::zipcrypto` exposes a `decrypt` function
and nothing that encrypts.

**What ZipCrypto is, plainly.** It is PKWARE's "Traditional Encryption",
specified in APPNOTE.TXT §6.0/§6.1 [D] — a three-key stream cipher from
1990, with a 1-in-256 false-accept rate on its own password check — about
1 in 128 as this repository uses it, since it tries both published
check-byte conventions — and a
known-plaintext attack (Biham & Kocher, 1994) that recovers the key from
a modest amount of known output, no brute force required. It is not
security by any current standard; the KNX Standard specifying it for
ETS4/ETS5 does not make it one. This code exists only to read a file
whose password the caller already has — see `knx-secure/src/zipcrypto.rs`'s
module docs for the full account, including both check-byte conventions
(PKZIP's own vs. Info-ZIP's streamed-entry variant) this implementation
has to try, because the `zip` crate's public API does not expose which
one a given entry used.

**Cause / evidence, updated.**

* **The AES/PBKDF2 key derivation (schema ≥ 21, ETS6+)** is specified in
  the KNX Standard itself, with the Standard's own test vectors: *The KNX
  Standard v3.0.0*, *Project Schema23 v01.00.00*, clause 4.2.4 "Password
  protection", p.64/64. `crates/knx-secure` implements exactly that
  derivation (`derive_knxproj_zip_password`) and its tests assert the
  exact Base64 output of all three of the clause's published vectors
  (`"a"`, `"test"`, and the non-ASCII third vector, recovered by
  rendering the source PDF page directly since every text-extraction
  path fails on it — see that test's own comment for the full account)
  — `[D]` (cited clause and page) and `[V]` (byte-exact). **Container
  decryption for this scheme is still not implemented** — AES needs the
  `zip` crate's `aes-crypto` feature (not enabled in this workspace) and,
  more importantly, a real AES-protected ETS6 project to verify against;
  neither exists here yet, so `Container::open_with_password` refuses an
  AES-protected nested payload by name rather than attempting it.
* **ZipCrypto (schema < 21, ETS4/ETS5) is now implemented and tested.**
  `knx-secure/src/zipcrypto.rs` hand-implements the cipher directly
  against APPNOTE.TXT v6.3.3 §6.1.3-§6.1.7 [D] (quoted verbatim in that
  module's docs, fetched 2026-09-14), rather than relying on the `zip`
  crate's own (also-writes-capable) decryption internals.
  `knx-etsproj`'s `Container::open_with_password` wires it into the
  container: it walks the nested payload's raw entries, decrypts each
  ZipCrypto-protected one, and decompresses the result (Stored or
  Deflated; a third method is reported, not guessed at). Tested against
  two synthetic container fixtures, both password `hunter2knx` and both
  generated with the independent Info-ZIP `zip` CLI — never by any code
  path in this repository: `crates/knx-etsproj/fixtures/zipcrypto-protected.knxproj`
  (nested entries Deflated) and `.../zipcrypto-stored.knxproj`, whose
  nested entry is Stored so that the CRC-32 gate below is reachable at
  all — against Deflated bytes a false accept fails to inflate and is
  reported before any CRC is compared. Plus a crypto-primitive-level
  fixture in `knx-secure/fixtures/`. All are
  synthetic, not extracted from a real ETS project, and that is an
  acceptable substitute *here*: ZipCrypto is a fully specified algorithm
  (APPNOTE.TXT), not an ETS-specific quirk, so a fixture built with a
  standard, independent tool exercises the same cipher a real ETS4/ETS5
  export would use. **What this does not verify:** whether a real
  ETS4/ETS5 export's nested payload matches this fixture's shape in every
  detail (entry layout, compression choices, check-byte convention in
  practice) — see §3 of `docs/COMPATIBILITY.md`, still listed as
  unverified against a real protected export.
* **A wrong password** is reported as `ContainerError::WrongPassword`,
  never a panic and never silently-wrong plaintext. ZipCrypto's own check
  byte only rules out 255/256 wrong passwords per convention, and this
  implementation tries both published conventions, so it lets roughly 1
  wrong password in 128 through — but the check byte is not the last
  gate. Every decrypted entry's decompressed bytes are checked against
  the entry's own declared size and CRC-32 from the ZIP central
  directory, and a mismatch there is reported as `WrongPassword` too,
  because after a check byte has already passed that is what it almost
  certainly is. A wrong password would have to survive a 1-in-128 check
  byte *and* forge a 32-bit CRC to be silently accepted. That residual is
  the ceiling ZipCrypto's design imposes, stated here rather than left
  implicit.

**Impact.** The *container layer* can now decrypt a ZipCrypto-protected
(ETS4/ETS5) project given its password — verified against synthetic
fixtures, not a real export. **No import path reaches it yet.**
`knx_etsproj::import` still calls `Container::open`, which refuses a
protected project outright; there is no CLI flag, HTTP route or UI field
that carries a password, and wiring one through is deliberately out of
this change's scope. Stage 1 of a six-stage pipeline can open a protected
project; the pipeline cannot.

**A decrypted project also has no roundtrip claim.** The opaque
passthrough store (ADR-0006) snapshots `Container::entries()` and reads
every entry back through `Container::read`, which cannot tell a decrypted
entry from one that was never encrypted — and the original ZipCrypto
ciphertext is not kept anywhere once decryption has run. A protected
project exported through that store would come back out *unprotected*.
`Container::was_decrypted()` exists so the import stage that eventually
wires a password through can see this coming and report it; nothing calls
it yet, because nothing yet decrypts anything outside the tests.

An AES-protected (ETS6) project still cannot be imported at all — same as
before this change, and for the same reason: refusing cleanly beats a
decryption path nobody has run against a real encrypted file. The refusal
now reads the entry's *raw on-disk* compression-method field to recognise
AES, because the `zip` crate overwrites its own parsed method with the
entry's real underlying one the moment it sees a WinZip AES extra field
(0x9901) — a check against the parsed value never fires, and the fall-
through blames the user's perfectly correct password instead.

**Lifted when.**

* ZipCrypto decryption's remaining gap — verification against a real
  ETS4/ETS5 protected export — is lifted when such a sample becomes
  available and the unknown-construct/reconciliation report comes back
  clean against it.
* AES container decryption is lifted when the `aes-crypto` feature is
  enabled, the decryption path is implemented the same way ZipCrypto's
  was, and a real password-protected ETS6 project is available to verify
  it against.
* The import pipeline's inability to reach the decryption it now owns is
  lifted when a password reaches `import()` — and, with it, an
  `ImportReport` entry for a decrypted project, so the roundtrip gap
  above is reported rather than discovered.

## 14. The project's default language is a placeholder

**Limitation.** Every imported project is created with
`Language("en")` as its `StringTable`'s default language, regardless of the
language the project was actually authored in.

**Cause.** A schema-11 project file carries no project-wide language tag:
`DefaultLanguage` belongs to an application program's `RegistrationInfo`
(RESEARCH §4.1), not to `ProjectInformation`. Session 3 imports no
application program, so there is nothing in the imported data to derive a
real value from, and inventing one from, say, the project name would be a
guess presented as a fact.

**Impact.** Nothing observable in Session 3: instance-level `@Text` and
`@Description` are literal, not translated, so every `Text` this importer
produces is `Text::Literal` and resolves without consulting the table at
all. The default language only starts to matter once localized program
text exists. Export and semantic comparison both ask the project's own
`StringTable::default_language()` rather than naming a language
themselves, so when a real value arrives there is exactly one place that
sets it.

**Lifted when.** Session 4 ingests application programs and their
`RegistrationInfo`, giving the importer a measured language to set instead
of a placeholder.

## 15. Unparsable values survive only on `Override` fields

**Limitation.** A present attribute whose value cannot be parsed keeps its
raw text — and is written back verbatim on export — only where the field
is modelled as `Override<T>` (`Override::Malformed`, ADR-0010's
amendment). On a field modelled as a bare value or an `Option<T>`, an
unparsable value falls back to the type's default and only the report
records what the source actually said.

**Cause.** `Override<T>` exists to carry an attribute's presence state, so
a fourth state costs nothing structurally. A bare `u8` or an
`Option<DateTime<Utc>>` has nowhere to put a string, and widening every
such field would push presence bookkeeping into parts of the model that do
not otherwise need it.

**Impact.** For the affected fields (timestamps, individual addresses,
numeric ids, enums such as `CompletionStatus`) a malformed source value is
reported but not written back: the export is a correct file that differs
from the original in exactly that attribute. No such value occurs in the
reference project — the case is reachable only with hand-broken input.

**Lifted when.** A real project is found that carries unparsable values on
those fields, making the added model surface worth its cost. Until then
the asymmetry is deliberate, documented, and reported at import time.

<a id="16-tauri-v2s-linux-backend-depends-on-archived-gtk3-bindings"></a>

## 16. Tauri v2 remains on GTK3; former maintenance advisories are resolved

**Resolved premise (verified 2026-09-22).** The locked desktop shell uses
`tauri` 2.11.5 and GTK3, but those bindings are no longer archived. RustSec
withdrew RUSTSEC-2024-0411 through RUSTSEC-2024-0420 on 2026-08-14 after the
`gtk3-rs` repository was unarchived and development resumed. **[D]** The
withdrawal and reason are recorded in each advisory, for example
[RUSTSEC-2024-0415](https://rustsec.org/advisories/RUSTSEC-2024-0415.html).
Before this edit, `cargo deny check advisories` exited 0 but reported the ten
old ignore entries as unmatched. After their removal it exits 0 with
`advisories ok`. **[V]** Keeping withdrawn suppressions would conceal future
policy drift rather than reduce risk.

**Remaining exposure.** Six unrelated transitive maintenance notices remain:
RUSTSEC-2024-0370 for `proc-macro-error`, plus RUSTSEC-2025-0075, -0080,
-0081, -0098 and -0100 for `unic-*` crates pulled through
`urlpattern`/`tauri-utils`. They are not reported vulnerabilities, have no
patched release, and each acceptance in `deny.toml` carries its review date
and dependency reason. **[V]** `proc-macro-error` is reached through
`glib-macros` while compiling the desktop, so exploiting an unknown defect
there would require influence over the build or macro input; it is not linked
as a runtime request handler. The five `unic-*` crates are reached through
`urlpattern` in Tauri's remote-URL matching for capability/IPC permissions.
Exercising an unknown defect there would first require attacker-controlled
remote-URL or capability pattern data;
KNXBench's desktop opens its own local server and exposes no plugin or
arbitrary-site surface. **[V]** These are boundaries, not proof that an
unknown defect cannot exist. The concrete accepted cost is code whose
maintainers no longer promise fixes, not a known exploit chain.

**Decision.** Keep Tauri 2 for this alpha. Tauri 3.0.0-alpha.2 was published
on 2026-09-21, while Tauri's normal Wry Linux GTK4/WebKitGTK 6 migration
([tauri#14684](https://github.com/tauri-apps/tauri/pull/14684)) and Wry's own
migration ([wry#1767](https://github.com/tauri-apps/wry/pull/1767)) remain open
as of 2026-09-22. **[D]** The separate experimental CEF runtime's
[3.0.0-alpha.2 release](https://github.com/tauri-apps/tauri/releases/tag/tauri-runtime-cef-v3.0.0-alpha.2)
lists its GTK4 dependencies. **[D]** Adopting that runtime and distributing
Chromium would be a platform migration, not a maintenance-warning fix. **[A]**
Re-evaluate after the Wry GTK4 work ships in a stable Tauri release. New,
unaccepted advisories block `cargo deny`; obsolete ignore entries are reported
as warnings and must be removed during the dependency review.

<details>
<summary>Historical finding before the RustSec withdrawals</summary>

**Limitation.** The desktop shell's Linux runtime depends on `tauri` 2.11,
which pulls in the archived gtk-rs GTK3 bindings; `cargo deny check` flags
16 upstream "unmaintained" notices in the advisory database, all of which
must be suppressed in `deny.toml` to build.

**Cause.** The gtk-rs project archived its GTK3 bindings repository in 2024.
The bindings are not vulnerabilities — every advisory explicitly states "no
safe upgrade is available" — but they are no longer maintained upstream.
Tauri's own GTK4 migration is in progress and not yet shipped.

**Impact.** Each `tauri` or `tauri-*` dependency bump requires manual
re-check of the 16 suppressed IDs: RUSTSEC-2024-0370, -0411 through -0420
(minus one gap), and RUSTSEC-2025-0075, -0080, -0081, -0098, -0100. As
Tauri moves to GTK4, some or all of these may disappear from the advisory
database. Until then, the `deny.toml` ignore list is permanent infrastructure.

**Lifted when.** Tauri v3 or a later `tauri` 2.x release ships its GTK4
backend and becomes the default on Linux.

</details>

## 18. `open_project` does not clear the previous `.knxdb` `store_path`

**Resolved; source reconciliation 2026-10-01 (AR00).** The text below is
historical, not the current behavior. `apps/knx-server/src/domain.rs::open_project`
passes `None` to `replace_project_state`; its shared replacement transaction
sets `store_path` together with the project, clean baseline, opaque data and
manufacturer references under the project lock. The desktop delegates to that
domain. `http_project_routes::importing_replaces_a_dirty_project_with_a_clean_baseline`
asserts the imported snapshot has no store path. Do not redispatch the old fix.

**Limitation.** `AppState.store_path` (the `.knxdb` file a subsequent plain
`save_project` writes to) is only ever set by `save_project_as` and
`open_native_project`. The Tauri `open_project` command — ETS `.knxproj`
import — loads a fresh in-memory project but never touches `store_path`.
If a `.knxdb` was open and the user then imports a `.knxproj`, `store_path`
still points at that old `.knxdb` file.

**Cause.** `open_project` and `open_native_project` were added in
different cycles (`.knxproj` import predates the native `.knxdb` format)
and were never made to share a single "what file, if any, backs the
in-memory project" invariant.

**Impact.** None reachable through the current UI: `apps/knx-web/src/
App.tsx` resets its own `hasStorePath` flag to `false` on ETS import, so
"Save" always falls back to "Save As…" in that state. But the backend has
no equivalent guard — `save_project` just writes wherever `store_path`
points, with no check that the loaded project actually originated from
that path — so a future UI change that calls `save_project` without first
re-deriving `hasStorePath` from a real backend query could silently
overwrite the old `.knxdb` with the newly-imported project's data.

**Lifted when.** Either `open_project` clears `store_path` to `None`, or
`save_project` verifies the in-memory project actually originated from
`store_path` before writing.

**Related (2026-09-10, T10).** `export_project` used to be a second
consumer of a stale `store_path`: it re-opened `store_path` off disk to
read the opaque passthrough table and manufacturer manifest, so the same
stale-pointer scenario above could attach one project's opaque/manifest
data to a different project's export. Closed for that one code path by
reading `AppState.opaque`/`AppState.manufacturer_refs` (the live,
in-memory copies) instead of re-opening the file — see
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s C4 row. The underlying gap
above (`store_path` itself can point at the wrong file) is unchanged.

## 20. Command palette and search share overlay CSS and an accessibility gap — partially resolved

**Resolved (2026-09-12, T31)** — for the shell and the keyboard defect;
not for accessibility conformance in general, which is not a thing this
entry can ever claim closed by fiat. `apps/knx-web/src/Overlay.tsx` is
now the one component behind
`.search-overlay`/`.search-panel`, and all four former hand-rolled
copies — `Search.tsx`, `CommandPalette.tsx`, `CatalogBrowser.tsx` (T2)
and `SettingsPanel.tsx` (T27) — render it instead of their own overlay
divs. It owns `role="dialog"`/`aria-modal="true"` on the panel,
backdrop-click and single-`keydown`-on-the-panel `Escape` dismissal
(replacing `SettingsPanel.tsx`'s old `window` listener outright),
initial focus (`initialFocusRef`, else the first focusable descendant,
else the panel itself via `tabIndex={-1}`), a `Tab`/`Shift+Tab` focus
trap, and focus restoration to whatever had focus before the dialog
opened. `overlayShell.test.ts` enforces the "one shell" half
structurally: it scans every `.tsx` file under `apps/knx-web/src` with
`node:fs` and fails, naming the offender, if the literal `search-overlay`
appears anywhere outside `Overlay.tsx` — so a fifth hand-rolled copy
cannot slip in by copy-paste the way the second, third and fourth did.

The keyboard defect that made this a user-facing bug rather than a
cosmetic one is also fixed: `CatalogBrowser.tsx`'s result rows, `<li
onClick>` with no `tabIndex`, no key handler and no role, gained
`ArrowDown`/`ArrowUp` highlight movement (stopping, not wrapping, at the
ends, matching `Search.tsx`) and `Enter`-to-pick, matching what clicking
a row already did — pre-filling the device-name field, not creating the
device, which still needs the name field's own `Enter` or the Create
button. A keyboard-only user can now reach and choose a catalog item;
previously they could not reach the list at all.

Listbox semantics are now applied uniformly across all three
list-bearing overlays: each text input is `role="combobox"` with
`aria-expanded`/`aria-controls`/`aria-activedescendant`; each `<ul>` is
`role="listbox"`; each row is `role="option"` with `aria-selected` and a
stable id — so `CommandPalette.tsx`'s `aria-disabled="true"` now sits on
a row that carries a role for it to qualify, and the highlighted row in
every list is announced via `aria-activedescendant` rather than not at
all. `Search.tsx`'s kind groups became `role="group"`/`aria-label`
wrappers around `role="presentation"` `<ul>`s, with the visible
`.search-group-label` marked `aria-hidden="true"` since the group's
`aria-label` already says the same thing.

**Keyboard alpha follow-up (2026-10-02, delivered at 2e57f8e5).** Search, Command Palette
and Catalog Browser now scroll the active option with nearest-edge alignment
without moving combobox focus. Catalog keyboard highlight remains distinct
from a picked product and never creates a device by itself. The shared shell
leases background `inert`/`aria-hidden` state per document, restores prior
attribute values, excludes newly added background content and handles nested
or out-of-order modal closes. Initial/Tab-wrap filtering skips controls below
hidden/inert/aria-hidden ancestors. Final close restores the original connected,
non-inert focus target. List logic still stays outside `Overlay.tsx`.

HelpTip keeps a permanent local, visually clipped description and paints a
separate decorative body portal. Fixed placement escapes clipped/transformed
ancestors, bounds the popup to the viewport at application zoom, repositions
on resize/captured scroll and cleans up listeners. Closed painted bubbles no
longer extend the document at their old trigger coordinates.

Focused tests and 17 fully intercepted Chromium cases cover list visibility,
modal focus/background accessibility-tree exclusion, nested close, and tooltip
geometry/description in DE/EN at 360/640/1440 px and 100/150% zoom. Eighteen
behavioral controls are rejected with exact source restoration. All twelve
candidate/integrated gates pass: Web 1,357, Chromium 52, Rust 2,890 / zero failed /
163 ignored, 576-source freeze. Exact source ref/tree/all 28 artifacts were read
back after publication; package log records failed predecessors separately.

**Still open.** No whole-application focus-visible styling audit across every
theme, full Tauri/native interaction acceptance, or actual screenreader run.
Chromium DOM/accessibility-tree evidence does not prove what NVDA, JAWS, Orca
or VoiceOver announces. No WCAG conformance, universal virtual-cursor guarantee
or alpha-release exception is claimed. Design:
`docs/superpowers/specs/2026-09-12-modal-overlay-shell-design.md`.

**U8 update (2026-09-29).** The shared shell now offers opt-in viewport-bounded
pointer and keyboard resizing. Settings and Debug report use it; the other
overlays retain their existing sizes. Focus containment, Escape and backdrop
dismissal are covered by regression tests, including a pointer drag that ends
on the backdrop. Headless Chromium exercised the Settings layout, arrow-key
resize and internal scroll at 640 px, including 150% application zoom; the
zoomed dialog's rendered dimensions must be converted back to layout pixels
before a keyboard step. Native WebKitGTK sizing and real screen reader
announcements remain unverified; this is not an accessibility audit.

**Originally.** Four components shared `styles.css`'s
`.search-overlay`/`.search-panel` shape with no shared component behind
it, three of them duplicating the whole modal shape by hand (overlay
div, click-outside `stopPropagation` panel, autofocused input, `Escape`
handling); no result list anywhere carried `role="listbox"`/`role="option"`;
`CommandPalette.tsx`'s disabled rows carried `aria-disabled="true"` with
nothing backing it; and `CatalogBrowser.tsx`'s rows had no keyboard path
into the list at all.

<a id="22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback--resolved-2026-09-20"></a>
## 22. One shared server password, no per-user rights or internet-security claim

**Resolved 2026-09-20** ([ADR-0026](adr/0026-server-authentication-or-loopback.md)).
The heading and the anchor above are kept so existing links still resolve;
what follows is what is true now, including the parts of the old
limitation that survived.

**What exists.** `apps/knx-server` has a password login, an in-memory
session table and one middleware layer over every route under `/api/`.
`POST /api/auth/login` takes `{"password": "..."}` and sets an
`HttpOnly; SameSite=Strict; Path=/` session cookie; `POST /api/auth/logout`
invalidates it; `GET /api/auth/status` reports `{"required", "authenticated"}`
so a frontend knows whether a login screen belongs on the screen at all.
The credential is PBKDF2-HMAC-SHA256 at 600 000 iterations (OWASP's 2023
floor) over a 16-byte random salt, stored in a self-describing PHC-style
string, and verified in constant time. Sessions time out after 12 hours
idle, refreshed on use, and do not survive a restart.

**The rule that makes it mandatory.** Authentication is optional at the
type level — `knx_server::app()` still builds an unguarded router, which
is what the Tauri desktop shell uses and why the desktop deliberately has
no login — but the standalone binary will not put an unguarded router on
the network. `bind_address(auth_required)` returns `0.0.0.0` with a
password configured and `127.0.0.1` without one, and it is the only
expression of a listening address in `main.rs`. An operator who sets
neither `KNX_AUTH_PASSWORD_HASH` nor `KNX_AUTH_PASSWORD` gets a loopback-
only server and an unmissable startup line saying so.

**Unauthenticated by design, and only these:** `/healthz` (a liveness
probe that needs a password is not a liveness probe), the static frontend
assets (they are the login screen), and the three `/api/auth/*` routes
above. Everything else under `/api/` — including `/api/version`, the
KNX bus routes, and `/api/fs/*`, which browses the host filesystem under
`KNX_DATA_DIR` (confined there by `paths::resolve_in_data_dir`, not by the
login) and was the sharpest edge of this limitation — answers `401` with the usual
`{"error": ...}` body to a caller with no valid session.

**What is still a limitation, and belongs to the deployer.**

- **No TLS.** Unchanged from the original entry. Over plain HTTP the
  password crosses the network in the clear in the login body, and the
  session cookie crosses it in the clear on every later request. A
  compromised host on the same LAN can read and replay that cookie. Put a
  TLS-terminating reverse proxy in front of anything that matters, and set
  `KNX_AUTH_COOKIE_SECURE=1` when you do — the cookie is not marked
  `Secure` by default, because on plain HTTP a `Secure` cookie is simply
  never sent back.
- **One password, no accounts, no roles, no audit trail.** Anyone holding
  the password can do everything, including writing to the KNX bus.
  Nothing records who did what, because there is no "who". Authentication
  here is not multi-user support: see
  [§63](#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all),
  which is unchanged — two browsers with valid sessions still share one
  project and one undo stack.
- **Brute-force resistance is a delay, not a lockout, and it resets on
  restart.** A failed login costs the caller 250 ms × the number of
  failures since the last success, capped at two seconds. The counter is
  in memory and process-wide. A lockout is deliberately absent: locking
  out the only operator of a single-operator server is a denial of service
  against its owner.
- **`KNX_AUTH_PASSWORD` is the weaker configuration.** A plaintext
  password in the environment is readable in `/proc/<pid>/environ`, in
  `docker inspect` and in shell history. `knx-server --hash-password`
  reads a password from stdin and prints a hash for
  `KNX_AUTH_PASSWORD_HASH`; that is the configuration to prefer. The
  server says as much at startup when it sees the plaintext form.
- **No CSRF tokens.** `SameSite=Strict` on the session cookie is the whole
  defence. It is a browser behaviour, not a server-side check.

**Not claimed.** This is not a statement that the server is safe to expose
to the internet. It is safe to expose to a network you have thought about,
over a transport you have secured yourself.

## 23. `/api/project/download` buffers the whole `.knxdb` file in memory

**Status, reconciled 2026-10-01 (AR00): resolved for whole-file buffering.**
The historical heading is retained for fragment links. Temporary SQLite
serialization remains; it is not a whole-file response buffer.

**Resolution.** The route freshly serializes the current in-memory project,
including unsaved edits, opaque entries, and manufacturer references, into
one temporary SQLite file. `tower_http::services::ServeFile` streams that
file in bounded 64 KiB chunks instead of copying it into a whole-file
`Vec<u8>`. Content type and attachment filename remain unchanged.

The response body owns the temporary path until it is dropped, including
after the HTTP response is split into its headers and body. Both completed
and abandoned bodies remove their temporary file; response extensions alone
would not guarantee this lifetime.

**Proof.** Unit tests consume a file larger than three small test chunks,
require multiple non-empty frames bounded by the configured chunk size,
compare every byte, and verify cleanup after completed and abandoned
downloads. The HTTP regression downloads an unsaved project and opens the
result as a KNX store, preserving its latest edit, installation, opaque
entries, and manufacturer references. Serialization still creates one
temporary SQLite file before streaming begins.

## 24. `FsPicker` has no drag-and-drop or multi-select

**Status, reconciled 2026-10-01 (AR00): resolved for multi-file upload/drop.**
The historical heading remains for fragment links. Single-project selection
is intentional and does not make the implemented upload gestures absent.

**Final-review hardening, 2026-09-22.** Duplicate basenames are explicit 409
conflicts, including pre-existing upload files; no destination is overwritten.
The earlier successful count and filename/error remain visible on partial
failure. Closing/selecting/unmounting stops the remaining queue. A reopened
picker waits for the previous in-flight request, which may still finish on the
server; closing is not a rollback of that request.

**Resolution, 2026-09-22.** The browser picker still returns one
`Promise<string | null>` path because project open/import remains a singular
human choice. Its local file input now accepts multiple files, and its upload
label accepts native file drops. A shared routine sends each file to the
existing one-file `/api/fs/upload` route sequentially, refreshes the
`uploads` listing after successful requests, and announces a completed batch
as a polite status. It never auto-selects an uploaded project.

**Failure handling.** The first failed request stops that batch, keeps earlier
successful uploads intact, and names the failed file plus server error and
completed count. It does not announce batch success. During protected-mode
dragover the picker inspects only `DataTransfer.types`; it reads dropped files
only at drop time.

## 26. `BusConnection` does not yet support KNX IP Secure

**Limitation.** `crates/knx-net`'s `BusConnection` trait implements
tunnelling and discovery: `discover` multicasts a `SEARCH_REQUEST`
(original form only, not Core v2's `SEARCH_REQUEST_EXTENDED`) and
collects `SEARCH_RESPONSE`s; `connect_tunnel` opens a tunnel to a
gateway by known IP; `subscribe` receives telegrams; `TunnelClient::send`
writes one (`GroupValueWrite` or any other `ApplicationService`, no DPT
interpretation — raw bytes only, same scope cut as the receive side).
`connect_routing` (Cycle 4) sends/receives unconfirmed `ROUTING_INDICATION`
frames over the standard multicast group — no custom multicast address
override, and `ROUTING_BUSY` is decoded and logged but never used to
throttle sends (see the two new limitation entries below). Secure-protocol
paths remain unimplemented. A reader should not assume the trait is
feature-complete because it compiles.

**Cause.** Session 6 Cycle 1 delivered read-only tunnelling as the
foundation for bus monitoring; Cycle 2 added sending; Cycle 3 added
discovery. Cycle 4 added routing. Secure protocols are out of v1 scope, handled by the isolated `knx-secure` crate.

**Impact.** A real KNX installation's gateways can be found on the LAN
without a known IP once `discover()` sends a valid discovery HPAI (a
final-review fix: the discovery socket must stay unconnected to receive
unicast `SEARCH_RESPONSE`s from any gateway, so a naive `local_addr()`
read off it reported the invalid `0.0.0.0:<port>` — see the fix commit
for the resolved-IP/real-port workaround), then handed by control
endpoint to `connect_tunnel` for monitoring/actuation by group address
over a tunnel. Live-hardware verification of the full discover-then-connect
flow was left for the user to run, same as Cycle 2's `send` — this sandbox has no real KNXnet/IP gateway to discover. KNX IP Secure
remains unreachable regardless; routing (unencrypted multicast) is
reachable as of Cycle 4. Discovery inside the `knx-server` Docker container
needs `--network host`: the server and web UI now call `discover`, so §79
applies to the shipped image.

**Lifted when.** Shelved indefinitely as of 2026-09-06 — no fixed
session or cycle owns it. Plain tunnelling/routing covers the common
case; IP Secure only matters for secure-only gateways or installations
with it explicitly enabled. Revisit on demand (a real gateway needing
it), doing the RESEARCH.md §9 spike first, not speculatively. See
[ROADMAP.md, Session 6](ROADMAP.md).

**Update, 2026-09-11.** Folded into **T19**'s scope (KNX Secure = Data
Secure + IP Secure + keyring). The user's 2026-09-11 ruling on T19 —
deferred, documented as a limitation, not rejected — applies here too; this
2026-09-06 shelving decision and T19's ruling stand together, not as two
separate calls.

## 29. `apps/knx-cli bus monitor` has formatting limitations

**Limitation.** The `knx bus monitor` subcommand, added in Session 6 Cycle 1,
always formats group addresses as three-level (e.g. `1/2/3`) regardless of
the project's configured style, and merges group-address names from all
installations into one flat namespace (last-seen wins on collision).

**Cause.** Deliberate scope decision for Cycle 1: the tool is built for
dev/smoke-testing use against the reference project, which has one installation
and uses three-level addressing throughout. Generalizing to multi-installation
projects and honouring the configured style requires mapping infrastructure
not needed for this cycle's verification workflow.

**Impact.** A project with multiple installations or a non-three-level
group-address style will see misformatted or incorrectly-merged names in the
monitor output. Data is not lost — telegrams still resolve by address internally
— only the human-readable label is approximate.

**Lifted when.** A future cycle adds full formatting respect and per-installation
name resolution, either bundled into a general bus-monitor redesign or as a
targeted enhancement to the CLI subcommand.

**Update, 2026-09-11 (T29).** `bus monitor --project <path>` now also
resolves and prints a DPT-decoded value for each telegram, via
`resolve_project_group_address_dpts` (§61 below has the full accounting of
what that resolution covers and does not). That resolution takes a
different position on the exact problem this limitation already describes:
where a group address's *name* is merged across installations with
last-seen-wins on collision, a group address's *DPT* is merged across
installations by reporting a `Conflict` and refusing to pick one. Two
answers to the same shape of problem inside the same subcommand, arrived at
in different sessions. This entry is left as-is rather than silently
rewritten to match the newer, stricter behaviour — a future cycle that
reconciles the two should treat that as its own decision, not an
accidental side effect of a DPT codec landing.

**Update, 2026-09-11 (T15).** The same hardcoding exists on the *write*
side, not just the display side this entry originally described:
`apps/knx-cli/src/main.rs`'s `bus write`/`route write` parse a
caller-typed destination with `knx_core::GroupAddress::parse(&str,
knx_core::GroupAddressStyle::ThreeLevel)` — the style is a literal, never
the open project's own `info.group_address_style`. T15's own `/write`
route (`apps/knx-server/src/bus_routes.rs`) had the identical bug and was
fixed to parse in the session's project's actual style (commit
`b540264`); the CLI's copy was deliberately left as-is, since fixing it
was not this branch's scope and CLAUDE.md asks that unrelated changes not
ride along with a feature branch. See
[§62](#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-only-its-passive-receive-path-has-real-gateway-evidence)
item 13 for the full account.

<a id="31-knxnetip-routing-has-no-custom-multicast-address-override--resolved-routing-half"></a>
## 31. Routing multicast override exists; discovery and real custom-group traffic unverified

**Resolved (2026-09-13, E6, branch `e6-routing-multicast`).**
`BusConnection` gained `connect_routing_to_group(own_address, group)`
alongside the unchanged `connect_routing(own_address)` — both funnel
through one `RoutingClient::connect_to_group`, so the default and the
override cannot silently drift apart; `connect_routing` is now exactly
`connect_to_group` called with `ROUTING_MULTICAST`'s own address.
`group` is validated as an IPv4 multicast address
(`Ipv4Addr::is_multicast()`, 224.0.0.0/4) before any socket call; a
non-multicast address fails fast with `BusError::NotMulticast`, naming
the rejected address, instead of a bare OS error several calls into
`join_multicast_v4`. `apps/knx-cli`'s `route-monitor` and `route-send`
both gained `--multicast-group <addr>`: an accepted CLI surface takes a
bare IPv4 address, never `address:port`, because the port is not this
override's to choose (see below); omitted, both join the standard group
exactly as before, byte for byte.

**What the Standard permits — R1, checked against the PDF, not just the
Markdown extraction.** Core v01.06.02 AS §8.5.2.2 **[D]** (p. 48): the
Routing Multicast Address "shall be derived from the... System Setup
Multicast Address by adding an offset", default zero; separate
installations sharing an IP network, or exceeding roughly 180 KNX
Subnetworks, "shall use different" Routing Multicast Addresses (Routing
v01.05.02 AS §2.3.2 **[D]**, p. 9). Routing v01.05.02 AS §2.3.1 **[D]**
(p. 9) fixes the *port*, not the address: "every installation shall use
the same IP multicast address and port... port number 3671 is
registered at [IANA] for this purpose" — which is why the override takes
only an address. Neither document states a maximum offset or any range
narrower than "any IPv4 multicast address"; the 180-Subnetwork figure is
guidance for *when* to deviate, not a constraint the code can enforce on
*what value* is chosen. Both citations were re-checked with `pdftotext`
against the original PDF, word for word, precisely because a numeric
claim is the kind the Markdown extraction has mis-rendered before —
this file's own §61/§62 entries note DPT `10.001`'s Day column
truncating at "7 =" in the extraction, which is how an earlier draft
wrongly called a documented range "undocumented". No such truncation,
table, or bit layout is involved here: §8.5.2.2 and §2.3.1/§2.3.2 are
plain prose in both the Markdown and the PDF, word for word. Per the
permissive-reading rule, the implementation validates the full
224.0.0.0/4 range rather than inventing a narrower one **[A]**.

**What remains open.** `DISCOVERY_MULTICAST` is untouched and still
hardcoded to `224.0.23.12:3671` — `discover()` has its own design
question (a different gap row) and E6's brief explicitly scoped this to
routing only. And, stated plainly because compiling is not the same as
working: **this override has never been run against a real installation
using a non-default group** — every test in `client.rs` either runs on
loopback or rejects an address before any socket call; none of it proves
a second KNXnet/IP router on the wire actually receives anything sent to
a custom group.

**Originally.** `RoutingClient::connect_routing` always joined the
standard KNXnet/IP System Setup Multicast Address, `224.0.23.12:3671`
(Routing v01.05.02 AS §2.3.1); no CLI flag or API parameter selected a
different group. Session 6 Cycle 4's design spec deliberately hardcoded
it, the same call as Cycle 3's discovery multicast address — no
environment at the time needed a non-default group.

<a id="36-session-log-t11-the-log-tab-was-unreachable-without-an-open-project-and-had-no-growth-cap--resolved-2026-09-10"></a>
## 36. Session-log export is a bounded retained window, not a lifetime audit

**U4 export boundary (2026-09-28 UTC).** The Log view now searches the
retained entries locally and exports either all retained entries or the
matching view as `knxbench-session-log` JSON v1. The export carries the
1000-entry cap, the pinned warning's dropped count (or `null` when its
wording is unfamiliar), and the raw warning message even if filtered out.
It cannot reconstruct entries evicted by the cap, a replaced project or a
server restart: this is **not** a lifetime audit. Exported raw entries can
contain project names, KNX addresses, filesystem locations and diagnostic
metadata. Browser export stays local; the native dialog writes atomically
but refuses a document above 16 MiB without replacing a previous file.
The desktop file chooser still needs interactive Linux validation across
supported environments; no complete desktop compatibility claim follows
from command/build tests alone (ADR-0047, `LogPanel.test.tsx`,
`sessionLogExport.test.ts`, `session_log_export_tests`).

**Resolved.** Two independent fixes: Part A in one commit, Part B in two —
its drop counter needed a follow-up correction, described at the end of
Part B below.

Part A: `apps/knx-web/src/App.tsx`'s "Log" toolbar button is
unconditionally enabled, and the `.workspace` slot now renders whenever
`tree` *or* `logOpen` is truthy, rather than `tree` alone —
`ProjectExplorer` still genuinely needs a project and stays gated on
`tree`, but `LogPanel` does not, so with no project open the Log tab is
the only thing in that area. With a project open, nothing changes: the
Log tab still takes the same slot it always did, and closing it returns
to Inspector/Dashboard as before. `LogPanel`'s `tree` prop is now
`ProjectTree | null`; it stays a `useEffect` dependency (so the panel
still refetches after a successful operation), and `refreshKey` —
bumped by `App.tsx`'s `reportError()` on every failed operation — is
untouched.

Part B: `apps/knx-server/src/session_log.rs` gained a documented
`MAX_ENTRIES: usize = 1000` const (not a bare literal at a call site).
Past it, `SessionLog::push` evicts the oldest real entries and pins a
synthetic `Severity::Warning`/`source: "log"` entry at index 0 naming
how many real entries have been dropped so far, refreshed on every
subsequent drop — CLAUDE.md's "never silently discard information" rule
applies to the log itself, not just to import data. That entry is never
itself dropped or duplicated, and it counts against the cap, so
`entries().len()` never exceeds 1000. `dropped` counts real entries
actually removed: the push that first exceeds the cap removes two (the
oldest real entry, plus one more to make room for the synthetic entry
itself), and every push after that while still over capacity removes
one more — an earlier draft of this counter tracked overflowing calls
instead of removed entries and read one low from the first drop
onward, caught before merge and fixed to match what actually happened
to the data. `reset()` clears the dropped count along with everything
else, so a freshly opened project starts with a genuinely empty log.
`GET /api/log`'s wire shape (`Vec<LogEntry>`, a bare JSON array) is
unchanged, so T12's own `session_log::from_csv_import_report` writer
and the existing `apps/knx-server/tests/http_log_route.rs` integration
tests needed no changes.

New tests: 6 in `session_log.rs`'s own `#[cfg(test)]` module (under the
cap, exactly at the cap, one past it — names 2 dropped — well past it —
cap + 250, names 251 dropped — reset-after-a-drop, and an invariant
test pinning "dropped named in the synthetic entry plus real entries
retained equals total pushes" at two different overflow sizes), plus a
new `apps/knx-web/src/App.test.tsx` (the first App-level test in this
project: reachable with no project open, unchanged behaviour with one
open). Gates: `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace`, `cargo run -p
xtask -- check-layering`, `npx tsc --noEmit`, `npm test -- --run`
(139/139), `npm run build` all clean.

**Originally.** `apps/knx-web/src/App.tsx`'s "Log" toolbar button was
`disabled={!tree}`, and the whole `.workspace` div — the only place
`LogPanel` rendered — was itself gated on `tree` being non-null. But
`GET /api/log` deliberately worked with no project open (`routes.rs`
returned `200 []`, not `404`), specifically so a failed import with
nothing open yet still left an inspectable trail. Separately,
`SessionLog` had no cap on how many entries it accumulated, and
`LogPanel` refetched and re-serialized the whole log on every
`tree`/`refreshKey` change while the tab was open.

Both gaps traced to the same cause, per the plan's own text: the Log
tab's UI slot was scoped to "a project is open" from the start, since
every other panel in that slot (Inspector, Dashboard, Project Explorer)
needs one; a log entry cap was never in the design spec's stated
surface. Neither gap was caught until T11's final whole-branch review,
and both were parked as a follow-up rather than fixed in the
final-review-fix round that closed the rest of that review's findings.

Impact while open: the single highest-value scenario for this feature —
"my import just failed and no project is open, why?" — produced a
correct error entry on the server that the UI could not show. Unbounded
growth was never a problem at the usage levels this feature actually
saw (a single server process, one project at a time, log never
persisted), but nothing stopped it from becoming one over a very long
session.

<a id="37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12"></a>
## 37. Translations reach selected surfaces, not every imported text or UI output

**Resolved.** Parameter text, parameter-ref text, and enum option labels
are read from the `translation` table at exactly one surface: the device
parameter panel. `knx-productdb`'s `parameter_views(conn, program_id,
language)` and `parameter_type_enum_options` take an `Option<&str>`
language and, when set, overlay the requested language's `Text`,
`FunctionText`, `SuffixText`, `VisibleDescription`, and `Name` rows over
the package's own untranslated attribute before `pick()` runs —
`ValueLayer`'s meaning is unaffected. `apps/knx-server` exposes `GET
/api/product-languages` (the database-wide language list, `200 []` with
no product database installed) and an optional `?language=` on both the
GET and the POST of `/api/device/{id}/parameters`. `apps/knx-web`
persists the chosen language as a per-user setting
(`productLanguage.ts`, `knx-desktop:product-language` in
`localStorage`, default `null` meaning "package default"), surfaces it
as a "Product data language" select in the Settings panel, and
`ParameterPanel` sends it on every load and every write.

**Also resolved, 2026-09-12 (T33).** Communication-object text is read
too now, at a second, narrower surface. `knx-productdb`'s
`com_object_view(conn, program_id, com_object_ref_id, language)` gained
the same `Option<&str>` overlay `parameter_views` already had, applied
before `pick()`: a `ComObject`-scope translation is keyed by the
`ComObject`'s own id, a `ComObjectRef`-scope one by the `ComObjectRef`'s
id, and exactly `Text`, `FunctionText` and `VisibleDescription` are ever
overlaid on `ComObjectView` — the same three attributes, nothing new.
Of those three, only `Text` and `VisibleDescription` go anywhere:
`apps/knx-server`'s `GET /api/device/{id}?language=` reads `view.text`
and `view.visible_description` and overwrites
`ComObjectNode::name`/`description`, but **only** where the stored
`Override<Text>`'s layer is `Layer::Program` or `Layer::ProgramRef` —
values the product database itself supplied. **Narrowed further on
2026-09-12 (T34, finding M6):** that layer condition is necessary but no
longer sufficient. The overwrite also requires `view.text_translated` /
`view.visible_description_translated`, so an overlay *miss* — a requested
language with no `translation` row for that attribute — now leaves the
project's own resolved text standing instead of replacing it with the
product database's untranslated column. `view.function_text` is
computed and then discarded at that call site: `ComObjectNode`
(`crates/knx-projection/src/lib.rs`) has no field to hold it, `enrich()`'s
`apply()` (`crates/knx-productdb/src/enrich.rs`) never stores it into a
project either. `knx-report` now accepts an English/German report language
and caller-composed product data, but its communication-object path
(`crates/knx-report/src/render.rs`) still renders `ComObjectNode::name`/
`description` straight through `build_device_detail`, which takes no
report language. Communication-object text and description therefore remain
language-insensitive even though report chrome and selected product data are
language-aware. `FunctionText` is still unread at every
surface, the same honest status this section already gives `SuffixText`
below. `Layer::Instance`, `Layer::Inferred` and `Layer::UserEdit` are
project-authored (the first and third are exported to `.knxproj`) and
are shown back verbatim regardless of the selected language, never
translated. `apps/knx-web`'s Inspector sends the persisted
product-language setting on every device-detail fetch and refetches when
it changes mid-selection, guarded against an older language's response
landing after a newer one's.

The Inspector's own description editor
(`apps/knx-web/src/Inspector.tsx`'s `ComObjectDescriptionField`, around
lines 112-125) seeds its input from that same displayed value, which
with a language selected is the translated `VisibleDescription`. Saving
it issues `Command::SetComObjectDescription`, which writes
`Layer::UserEdit` — and `UserEdit` values are among the layers exported
to `.knxproj`. So a translated string can become project data, but only
through an explicit save; this is not new behaviour (the same field
pre-filled from untranslated text before T33) and not a bug, just the
one place display and storage meet.

Re-measured, not assumed, on the same package §64 uses:
`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`'s application program
`M-0083_A-0317-31-7DC6` carries 53 `ComObject`-scope `Text` and 50
`ComObject`-scope `FunctionText` translations per language, across all
five declared languages (`de-DE`, `en-US`, `fr-FR`, `es-ES`, `it-IT`) —
already ingested since T26/T32, now finally read (`Text` and
`VisibleDescription` only, per the `FunctionText` correction above). That
same package carries zero `ComObjectRef`-scope `Text` and 39
`ComObjectRef`-scope `FunctionText` translations per language — a
per-package figure, not the whole corpus. Re-measured directly against
the installed database for this pass
(`~/.local/share/knx/products.sqlite`, `sqlite3`, joining `translation`
against `com_object_ref` by `(scope_id, ref_id) = (program_id, id)`, of
12 installed application programs total): 3,781 `ComObjectRef`-scope
`Text` rows spanning 8 programs and 692 `ComObjectRef`-scope
`VisibleDescription` rows spanning 6. A handful translate a
`ComObjectRef` that declares no structural text of its own — e.g. program
`M-006A_A-0001-22-26C0-O0079`, ref `_O-0_R-10001`, whose fr-FR `Text` row
reads "sortie - Lumière" while that `ComObjectRef`'s own `Text` column is
empty and only its parent `ComObject` supplies "Ausgang - Licht" —
so the overlay resolves at the `ProgramRef` layer where the untranslated
value would otherwise have come from `Program`. Whether ETS treats a
`ComObjectRef`-scope translation of an attribute the `ComObjectRef`
itself never declared the same way is unattested; nothing here claims it
does.

**Narrowed again, backend only (2026-09-13, D10 slice 1, branch
`d10-master-translations`).** The locale-prefix gap named a few
paragraphs below — a stored `de` not matching a package's `de-DE` rows
— is closed at the query layer: `crates/knx-productdb/src/query.rs`'s
`best_matching_language` now lives in exactly one place and is the
resolution step behind every overlay in the file, old and new alike
(`translation_overlay`, `catalog_overlay`, `master_text_overlay`, and
anything built on top of them), with its own dedicated unit tests —
exact match preferred over a prefix match, `de` matches `de-DE`, and a
hypothetical `deX` does not, proving the match requires the `-`
separator rather than a bare string prefix. No caller sends a bare
primary-language tag yet: `apps/knx-web`'s language pickers populate
their options from the exact tags a package actually stored, so nothing
today exercises the new prefix path end to end. This closes the backend
half of the gap, not the full round trip — a frontend that let a user
type or detect a bare `de` would light the rest of it up for free. See
[§64](#64-languages-blocks-outside-an-application-program-are-discarded-on-import)
for this same slice's other two pieces: a reader for `Master`-scope
`DPST-*`/`DPT-*` translations, and per-scope translation counts in the
import report.

**Still open.** Device creation and `enrich()` still bake untranslated
text into the project file — deliberately: translating there would make
the *stored project* depend on a display setting, the same integrity
line T26 was careful not to cross for parameter values, and T33 did not
cross it either. The UI chrome itself, tracked separately as **T25**, is
no longer hard-coded English — it shipped 2026-09-12, see the T25 entry
in [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s Tier 6 — but this closes
only the *chrome* half of D10; the data half's own residue below is
unaffected. `knx_core::string_table`'s `StringTable`
still has no resolver anywhere except `build_device_detail`'s own
`project.strings.default_language()` call (`crates/knx-projection/src/lib.rs`)
— a fixed default, not a user choice — so `LocalizedString` resolution
against the *selected* product language does not exist; the com-object
overlay above works entirely by substituting `knx-productdb` text before
it reaches that call, not by teaching the string table anything. A
project's own `Language` field is still the placeholder `"en"` both
importers hand `Project::new`, and remains unread by anything. `Value`
translations are deliberately never applied, for the identical
stored-data-integrity reason: a parameter's value is a key written into
the project file, not display text. `parameter.suffix` is stored but
displayed nowhere, so all 879 `SuffixText` rows measured for this
slice's design spec remain unread. The backend query layer now matches a
stored `de` selection against a package's `de-DE` rows (D10 slice 1,
above), but no caller exploits it and there is still no
`navigator.language` detection — the setting defaults to "package
default" and stays there until a user picks explicitly.

**Lifted when.** Partially, 2026-09-12: first the parameter panel (T26's
first slice), then communication-object text (T33, same day), then the
UI chrome itself (T25, same day — see its own entry in
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s Tier 6, closing more of gap
**D10**). What remains is a later T26/T33 follow-up: the project's own
`Language` field, and `StringTable`/`LocalizedString` resolution against
a user-selected language — neither touched by any slice so far; every
overlay added is `knx-productdb`-side only. The ingestion gap T26's first
slice deliberately did not fix has since been closed by T32 (2026-09-12):
`Catalog.xml`, `Hardware.xml` and `knx_master.xml` translations are
ingested, and the catalog browser reads the catalog-scope ones. What is
ingested but still read by nothing — hardware and master text — is
recorded in
[§64](#64-languages-blocks-outside-an-application-program-are-discarded-on-import).

## 38. Group-address CSV export/import (T12) has no verified ETS interoperability

**Limitation.** "KNXBench group-address CSV v1" (`crates/knx-csv`,
[IMPORT_EXPORT.md §11](IMPORT_EXPORT.md#11-group-address-csv-exchange)) is
a format this project defines and documents itself. It is not, and cannot
currently be shown to be, compatible with ETS's own "Export Group
Addresses" CSV feature, or with the legacy `.esf`/OPC export format.

**Cause.** No sample of either format exists anywhere in this repository,
and searching all 179 documents of the extracted KNX Standard v3.0.0
corpus for `csv`, `esf`, `OPC export`, and group-address-export
terminology turned up nothing but two incidental prose hits (a
data-security test report and an RF application note) — there is no
standardized group-address exchange text format at all. Group-address CSV
export is an ETS *application* feature, not something the KNX Association
specifies, so there is nothing to read except a real file, and none has
been obtained.

**Impact.** A file exported by KNXBench is not guaranteed to open sensibly
in ETS, and a CSV exported from ETS is not guaranteed to import cleanly
here — the importer is column-name-driven and separator-detecting
specifically so a foreign file has a *fair chance*, but that is a design
mitigation, not a tested claim. Nothing in the UI, CLI output, or this
documentation set may say "ETS CSV" or imply interoperability, and none of
it does.

**Lifted when.** A genuine ETS-produced group-address CSV export is
obtained. At that point, adding a second, ETS-shaped column profile to
`crates/knx-csv`'s reader is the stated upgrade path — header matching is
already isolated in one function (`map_headers`), so the profile would go
there rather than spreading through the parser, though that function is a
hard-coded `match` and would itself have to be edited. `.esf`
import is a separate, larger undertaking (writing a parser against
remembered syntax with no sample to check it against is exactly what
CLAUDE.md's "do not invent technical facts" forbids) and would need its
own task, gated the same way on first obtaining a real file.

<a id="39-csv-import-never-re-addresses-deletes-or-manages-group-ranges"></a>
## 39. CSV import does not create or rename group ranges

**Limitation.** Explicit readdressing and deletion are implemented, but CSV
still does not create, rename, resize, or delete group ranges. New and moved
addresses are assigned to the innermost existing range whose bounds contain
their final address, or left without a range when none does. Removing a row
from a CSV still means nothing; deletion requires `Action=delete`. A
readdress target must be unoccupied in the pre-import project, so swaps and
"delete this target, then move into it" combinations are rejected rather
than made order-dependent.

**Cause.** Range CRUD is separately modelled structure
(`Command::CreateGroupRange`/`RenameGroupRange`), not a property of one
address row. Inferring range mutations from repeated `MainGroup` or
`MiddleGroup` text would introduce ordering, boundary, rename and conflict
ambiguities. Those columns therefore remain derived/read-only.

**Impact.** Bulk address creation, rename, flag edits, stable-id readdressing
and unreferenced deletion are supported. A spreadsheet cannot reshape the
group-range hierarchy; that still uses the dedicated project editing
commands. Delete is refused while communication-object links remain.

**Lifted when.** A dedicated, versioned range-exchange contract defines
stable range identity, hierarchy and bounds without guessing from names.

<a id="40-csv-export-only-columns-are-never-applied-on-import-and-there-are-no-descriptioncomment-columns"></a>
## 40. CSV derived columns are read-only, and there are no `Description`/`Comment` columns

**Limitation.** `DatapointType (read-only)`, `MainGroup (read-only)`, and
`MiddleGroup (read-only)` appear in an exported CSV as derived context, but
are never editable project values. Import validates them against the current
project; a changed value is rejected explicitly and nothing is applied. The
legacy unsuffixed headers remain readable with identical semantics.
Separately, the CSV format has no
`Description` or `Comment` column in either direction, even though the
`.knxproj` schema itself defines `GroupAddress/@Description` and
`@Comment` attributes.

**Cause.** A group address in this domain model (`GroupAddressEntry`,
`crates/knx-core/src/group.rs`) carries no datapoint type at all — a DPT
belongs to the communication objects linked to the address, several of
which may legitimately disagree, so there is no single value a CSV row
could write back onto the address itself. `MainGroup`/`MiddleGroup` name a
*containing* group range, which is structure, not a field of the address,
so writing one back would mean silently moving the address between ranges
from a rename-focused editor. `Description`/`Comment` are simply not
modelled anywhere in `GroupAddressEntry` yet — the CSV cannot round-trip a
field the domain model does not have.

**Impact.** A user who edits one of these derived cells gets a row-level
read-only error rather than a false success; an unchanged export imports as
a tested no-op. There is no way to bulk-set or bulk-view a description or
comment for a group address via CSV, because there is nowhere in the
project for it to live yet.

**Lifted when.** Making `MainGroup`/`MiddleGroup` editable is tied to the
dedicated range-exchange contract named in §39. `Description`/`Comment`
becoming available is tied to `GroupAddressEntry` gaining those fields in
the domain model — no task currently schedules either.

<a id="41-a-csv-file-saved-from-excel-under-a-german-locale-may-still-surprise-a-user"></a>
## 41. German-locale separators are supported; unverified spreadsheet transformations remain

**Limitation.** The importer auto-detects `,` and `;` as the field
separator per file, specifically because Excel's own CSV export/import
behavior depends on the OS list separator setting: under a German
(or otherwise comma-decimal) locale, Excel writes `;`-separated CSV and
expects `;` back on open, while under an English locale it uses `,`. Both
are accepted here. What is not handled is everything else Excel can do
to a file beyond the separator — most notably re-saving with an unsupported
encoding or altering address/boolean cells during a manual edit. RFC 4180
quoting, embedded separators/quotes/newlines, UTF-8 BOM, CRLF/LF and mixed
line endings all have direct regression tests; they are not part of the
remaining limitation.

**Cause.** The separator auto-detection in `crates/knx-csv/src/read.rs`
covers the one Excel behavior this project could concretely name and test
against (`parses_the_same_file_semicolon_separated`). Excel's broader
locale-dependent quirks are not enumerated anywhere in this codebase or
its research, and guessing at more of them without a concrete failing
sample would be exactly the kind of unverified assumption CLAUDE.md rules
out.

**Impact.** The known German-locale separator difference is covered and an
unchanged export/import round trip is tested. A user who hand-edits an exported
file in Excel and hits an import error on a cell Excel silently reformatted
should not assume the importer is broken — it is a known category of risk
with this specific tool, not a claim that every Excel edit is safe.

**Lifted when.** A concrete Excel-induced parse failure is reported with a
reproducing file, at which point it becomes a specific, testable case
rather than a general caution.

## 42. `command_sync.rs`'s module doc overstates its own role — pre-existing, not introduced by T12

**Resolved by AR04.** The exported compatibility helper explicitly uses the
existing transactional whole-project-save fallback. Its former successful no-op
arms and row-only persistence claims are removed; adjacent low-level helper
comments and tests no longer imply a live incremental engine. Current production
save paths remain complete saves and do not call this helper.

**Evidence.** A previously unsupported parameter edit failed behaviorally before
the fix. File-backed regressions verify nested structural batches, exact reopened
models/high-water marks, undo/redo, middle-sibling order, unchanged opaque and
manufacturer rows, and a late SQL failure retaining the prior durable state.
Three compiled behavioral mutants were caught and all touched source hashes
restored. See [the storage contract](STORAGE_COMMAND_CONTRACT.md) and the AR04
receipt in `.ai/logs/2026-10-01_codex_alpha-storage-contract.md` for final gates
and publication; a planned gate is not a passing result.

**Retained boundary, not this former defect.** No incremental-performance claim,
automatic in-memory/history rollback, combined opaque/model transaction or new
multi-user conflict policy is introduced. Use `save_project_if_unchanged` for
the existing expected-state contract. UI/editor and commissioning scopes remain
with their original owners. This heading/fragment is a historical waypoint.

<a id="43-animations-have-no-in-app-switch-only-the-os-reduced-motion-preference"></a>
## 43. Animation controls exist; some motion surfaces remain outside their guard

**Limitation.** Resolved for the two axes T27 (2026-09-12) shipped,
enforced structurally rather than by convention, and still limited in
five specific, deliberate ways below.

`apps/knx-web/src/motion.ts` exposes two independent, persisted settings —
a **level** (`off`/`subtle`/`standard`, default `standard`, driving
`--knx-transition-duration`: `0ms`/`120ms`/`250ms`) and a **style**
(`apple`/`glitch`, displayed as "Smooth"/"Glitch", default `apple`,
driving `--knx-motion-easing`: a cubic-bezier ease or `steps(4, end)`).
Both live as two `<select>`s in the gear-button `SettingsPanel.tsx`
alongside the theme picker, persist to `localStorage`
(`knx-desktop:motion-level`, `knx-desktop:motion-style`), and are applied
as `data-motion-level`/`data-motion-style` on `<html>` both by
`useMotion()` after React mounts and by a pre-mount bootstrap script in
`index.html` (so there is no flash of default motion before the first
render — the script hard-codes the same id lists as `motion.ts`,
cross-commented in both files as a duplication to keep in sync by hand).

The OS-wins rule is structural, not conventional: every
`transition:`/`animation:` declaration in `styles.css` sits inside a
`@media (prefers-reduced-motion: no-preference)` block, none uses
`!important`, and no `.ts`/`.tsx` file calls `window.matchMedia` at all —
so `prefers-reduced-motion: reduce` cannot be overridden from inside the
application, even by mistake. `motionGuard.test.ts` turns that rule into a
test: a brace-counting checker over `styles.css`'s text fails the suite if
any `transition:`/`animation:` declaration sits outside a
`no-preference` block, or uses a literal duration instead of
`var(--knx-transition-duration)`.

What remains limited, on purpose:

- **No per-category control.** One duration and one easing curve apply to
  the whole application. A user who wants the Group Monitor's new-row
  highlight still but the hover transitions live has no way to say so.
- **The guard reads `styles.css` only.** An inline `style={{ transition:
  ... }}` in a component, a second CSS file, or a stylesheet inside a
  future dependency would all escape it entirely.
- **The guard matches the `transition:`/`animation:` shorthands only.** A
  longhand — `animation-duration: 300ms`, `transition-delay: 400ms` — is
  not inspected and would pass. Widening the pattern to longhands would
  false-positive on `transition-property`, which carries no duration at
  all; closing the hole honestly needs a CSS value parser, and the slice
  forbids the dependency. Recorded rather than fixed, on purpose.
- **The test environment does not run CSS animations.** No test anywhere
  asserts that anything actually moves; the tests assert that the right
  class and the right attribute are applied (including
  `BusMonitorPanel.test.tsx`'s new-row highlight tests), and the
  stylesheet is trusted to do the rest. Nobody has verified the visual
  result in a browser.
- **The guard has to read the stylesheet from disk with `node:fs`, and the
  tidier-looking alternative silently disarms it.** Replacing the read
  with Vite's `import css from "./styles.css?raw"` type-checks, runs, and
  passes — against the **empty string**, because Vitest does not process
  CSS. This was tried on this branch and caught by injecting a literal
  `200ms` into `styles.css` and watching the suite stay green regardless.
  A future author tidying that import away would remove the guard without
  removing the test. The `node:fs` import in turn needed
  `apps/knx-web/src/node-builtins.d.ts` (added mid-slice, commit
  `056b4a0`), because `npm run build` is `tsc && vite build` over
  `include: ["src"]` and `@types/node` is deliberately not a dependency —
  without it, `npm run test` (Vitest) was passing on a `node:fs` import
  that broke the production build outright, caught once on this branch
  before it shipped anywhere.

**Cause.** The regression this section used to describe — cycle 11's
`off`/`subtle`/`standard` setting in `ThemePanel.tsx`, deleted without
replacement by cycle 13's theme rewrite — is fixed. What remains above is
scope, decided rather than missed: T27's design chose two orthogonal axes
(level, style) over a per-category switch because the 2026-09-10 style
memo asked for two independent visual directions, not finer-grained
animation targeting (see `ROADMAP.md`'s "Cross-cutting — Motion and
animation"); and the guard was built as a text checker over one known
file rather than a real CSS parser, because the slice's no-new-dependency
rule rules out pulling one in just for this.

**Impact.** Materially smaller than before T27. A user bothered by motion
can turn the level to `off`; a user who finds the default merely too much
can use `subtle`; a user with an opinion about *how* things move, not just
how fast, can pick between the two shipped styles independently of
intensity. What a user still cannot do is quiet one feature while keeping
another animated, and what nobody can do is rely on the test suite alone
to prove that nothing outside a `no-preference` block moves — the guard's
blind spots above are real, just narrow today because `styles.css` is
still the only stylesheet and every current declaration is a compliant
shorthand.

**Lifted when.** Partially lifted, 2026-09-12 (T27, closing
`GAP_ANALYSIS_ETS.md` gap D11): the in-app switch exists, is two axes
wide, persists across sessions, and is structurally bound to
`prefers-reduced-motion: reduce` always winning. The rule this section
used to ask for — every future animation switchable through it — is now
enforced by `motionGuard.test.ts` rather than by prose, and T15's Group
Monitor table (the first animated feature that shipped without the
switch) has been retrofitted with a guarded new-row highlight
(`BusMonitorPanel.tsx`). What is left open, and would need its own
design work rather than a bugfix: (1) a per-category control, if a future
feature ever needs quieter motion in one area while another stays
animated — not requested yet; and (2) a guard with real CSS-parser-backed
coverage (longhands, non-`styles.css` sources, inline `style=` motion) —
deliberately deferred, since it needs a dependency this slice was not
scoped to add.

## 44. Project documentation export (T13) has no ETS report parity, and none can currently be measured

**Limitation.** `crates/knx-report`'s HTML document
([IMPORT_EXPORT.md §12](IMPORT_EXPORT.md#12-project-documentation-export))
is KNXBench's own document. It is not, and cannot currently be shown to
be, similar in content or layout to any report ETS's own printing feature
produces.

**Cause.** No ETS-produced report sample — no PDF, no printout, no
exported document of any kind — exists anywhere in this repository, and
`docs/RESEARCH.md` has no section describing ETS's report layout. This is
the same evidence gap [§38](#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability)
records for T12's CSV format: printing/reporting is an ETS *application*
feature, not something the KNX Association standardizes, so there is
nothing to read except a real sample, and none has been obtained.

**Impact.** Nothing in the UI, CLI output, or this documentation set may
say "ETS report" or imply compatibility with one, and none of it does. A
user expecting the document to resemble an ETS printout in section order,
wording, or completeness has no basis for that expectation from anything
KNXBench ships.

**Lifted when.** A genuine ETS-produced report sample (PDF or printed
export) is obtained. At that point a content-set comparison becomes
possible for the first time; whether that motivates layout changes is a
separate decision to make once evidence exists.

## 45. Project documentation export has no native PDF output

**Limitation.** `crates/knx-report` produces HTML only. There is no Rust
PDF renderer anywhere in this workspace, and none is planned.

**Cause.** A deliberate scope decision
(`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
§2, §9): every modern browser already prints to PDF, the document ships
`@media print` rules for exactly that, and a Rust PDF-rendering dependency
would be a large addition serving a button the operating system already
provides. CLAUDE.md: avoid unnecessary dependencies.

**Current state (2026-09-23, T14).** The self-contained document retains its
print-specific stylesheet and the server now exposes the exact HTML through
`POST /api/project/documentation-preview`, so a browser can preview and invoke
its native print/PDF path without a second renderer. The Rust crate still does
not generate PDF bytes.

**Impact.** Headless automation that cannot drive a browser still has no
`knx doc-export ... --pdf` path. Native PDF remains deliberately omitted; the
browser print path is the supported route.

**Lifted when.** Open. No task currently proposes a native PDF renderer —
recorded here as a boundary of the feature, not a gap awaiting a fix.

<a id="46-project-documentation-export-does-not-resolve-manufacturer-product-or-program-names"></a>
## 46. Project documentation export resolves names only with installed product data — partially resolved 2026-09-23 (T14)

**Current state.** Server and CLI callers now resolve manufacturer, product and
application-program names through `knx_productdb::query::device_product` and
pass only display data into pure `knx-report`. Raw `product_ref` and
`program_ref` remain beside the names as provenance. The query accepts a
program link only when its `hardware_id` equals the product's hardware; a
mismatch cannot contribute another product's program name or parameters and is
reported with both raw references. When the database is absent, a reference
does not resolve, or a joined name is absent or whitespace-only, the available
raw reference is used as the cell value and the renderer returns a
`ReportWarning`; the cell is never silently blank.

**Cause.** `crates/knx-report` depends only on `knx-core`, `knx-projection`,
and `chrono` (`xtask check-layering` enforces this, the same rule
`knx-csv` is held to). Resolving those identifiers to a human-readable
name requires querying `knx-productdb`, a separate, independently
versioned database this crate must not reach.

**Impact.** Reports made without the matching installed product package cannot
invent human-readable names. They remain complete but carry explicit warnings.

**Lifted when.** The data-dependent part cannot be lifted globally: product
packages are optional. The architectural gap is closed; missing external data
is now an explicit per-device report condition.

<a id="47-project-documentation-export-does-not-list-parameter-values-or-module-instance-arguments"></a>
## 47. Project documentation export lists parameter values and module-instance arguments — partially resolved 2026-09-23 (T14)

**Current state.** The Devices section lists stored parameter values and
module-instance arguments in deterministic project order. With matching
product data, parameter names, translated enum display text and module argument
names are added. Raw identifiers and raw values are always printed. Blank
parameter/module-argument names fall back to those identifiers and warn.

**Cause.** Both remain raw in `knx-core`; the outer `knx-app` composition
layer now joins what `knx-productdb` can prove and passes display-only rows
into `knx-report`. This preserves the renderer's dependency boundary.

**Remaining limitation.** A module-qualified parameter whose stored identifier
cannot be matched to a declaration, or a parameter kind without a meaningful
display formatter, is shown raw with both an inline explanation and a
`ReportWarning`. A restriction value missing from its declared enumeration is
handled the same way. Raw module argument values are not semantically
interpreted; `AllocatorRef` and unknown argument kinds remain unsupported, as
do allocation metadata and repeat semantics. The generated document's
mandatory limits section states this boundary.

**Lifted when.** Additional verified parameter/module semantics exist for the
remaining warned cases. T14 deliberately does not invent them.

<a id="48-project-documentation-export-renders-in-one-language-only"></a>
## 48. Project documentation export has English/German chrome but not a complete prose catalogue — partially resolved 2026-09-23 (T14)

**Current state.** `ReportOptions::language` selects English or German, sets
the HTML `lang` attribute, localizes the document title and primary navigation
and asks product-database queries for the same locale. The HTTP preview/export
API accepts `en`, `en-US`, `de`, or `de-DE`.

**Cause.** Product strings can reuse the existing language-aware queries, but
report-owned prose is not part of the web message catalogue and must remain
available to CLI callers. T14 added the two supported built-in locales without
coupling the crate to frontend language packs.

**Impact.** English and German reports use localized primary chrome and
product strings. Remaining detailed English prose can still appear in either.

**Remaining limitation.** Detailed table labels, enum/debug values and several
diagnostic sentences are still English. No third language or external report
language pack exists. The web documentation dialog (CT-2) requests the
report in the UI language (`de` for German, `en` otherwise); a separate report
language selector is still absent.

## 51. Project diff (T14) has no ETS-comparison parity, and none can currently be measured

**Limitation.** `crates/knx-diff`'s output — a "KNXBench project diff" —
is KNXBench's own comparison. It is not, and cannot currently be shown to
be, similar in matching rules, content, or presentation to whatever
ETS's own project-compare feature produces.

**Cause.** No ETS-produced comparison output — no screenshot, no exported
report, no printed diff — exists anywhere in this repository, the same
evidence gap [§44](#44-project-documentation-export-t13-has-no-ets-report-parity-and-none-can-currently-be-measured)
records for T13's HTML report and [§38](#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability)
records for T12's CSV format: project comparison is an ETS *application*
feature, not something the KNX Association standardizes, so there is
nothing to read except a real sample, and none has been obtained.

**Impact.** Nothing in the UI, CLI output, or this documentation set may
say "ETS compare" or imply compatibility with it, and none of it does —
`knx-diff`'s own module doc and its design spec (§1) state this
explicitly. A user expecting the diff to match what ETS's own compare
screen would show — which entities it matches, which fields it compares,
how it presents a rename — has no basis for that expectation from
anything KNXBench ships.

**Lifted when.** A genuine ETS-produced comparison sample is obtained. At
that point a content-set comparison becomes possible for the first time;
whether that motivates changes to the matching rules or the rendered
output is a separate decision to make once evidence exists.

## 52. Project diff cannot correlate a device with no individual address and no matching `ets_id`

**Limitation.** A device's natural key
(`docs/superpowers/specs/2026-09-10-project-diff-design.md` §3.4) is its
individual `address`, and only when `Some`. A device with no individual
address relies entirely on an `ets_id` match; if that also fails to line
up between the two projects being compared, `diff_projects` cannot
correlate the two at all — the device surfaces as an unrelated `removed`
on one side and `added` on the other, never as a match with field
changes.

**Cause.** A deliberate scope decision (design spec §3.4, §9): there is
no stronger identity to fall back on. Guessing would risk a false match
between two genuinely different devices, which CLAUDE.md's
never-silently-discard/never-guess posture rules out.

**Impact.** Two saves that differ only in, say, a description edit on an
address-less device can be reported as one device removed and a
different device added, obscuring what was actually a single edit.

**Measured 2026-09-23 (T15).** All three local corpus projects contain
zero devices lacking both an address and an ETS id. The matcher regression
also establishes that two empty ETS ids are *not* an id match: they remain
unrelated unless a real natural key resolves them.

The aggregate-only local measurement is intentionally ignored in ordinary CI.
With the gitignored `OriginalData/` corpus present, reproduce it with
`cargo test -p knx-app --test diff_correlation_measurement -- --ignored --nocapture`.

**Lifted when.** Open. No stronger per-device identity exists in the
domain model today. The zero corpus count ranks this below observed gaps;
it does not justify inventing a fallback or claiming the case impossible.

## 53. Project diff can collide two same-named sibling building parts

**Limitation.** A building part's natural key is the path of names from
the root (design spec §3.4). Two siblings under the same matched parent
that share a name produce the identical path and therefore collide under
the ambiguity rule (design spec §3.3 step 3): both are reported as
individual `added`/`removed` entries, plus one `AmbiguityNote`, rather
than matched to each other.

**Cause.** The same limitation `knx-etsproj::compare`'s
`semantic_building_part` already accepts for its own single-parent-hop
identity (design spec §3.4's own note): a name-based key has no way to
distinguish same-named siblings, and building parts carry no other
stable identity once their `ets_id`s also fail to correlate.

**Impact.** Renaming, or otherwise editing, one of two same-named sibling
building parts between two saves can render as an ambiguous add/remove
pair instead of a clean field change.

**Measured 2026-09-23 (T15).** Across the ETS4, ETS6, and independent
Schema-21 corpus projects, there are zero duplicate `(parent, name)` sibling
groups. This remains a supported ambiguity path, not evidence that the
shape cannot occur in another project.

**Lifted when.** Open. Recorded as a boundary of the path-based key, not
a bug awaiting a fix; the available corpus gives it no implementation
priority over observed comparison problems.

## 54. Project diff does not detect an ETS re-import's regenerated `RefId`s as "the same project"

**Limitation.** ETS may regenerate `RefId` strings on a fresh re-import
of a `.knxproj` it has seen before. `diff_projects` has no special case
for this: if the natural key also does not line up for a given entity, a
re-import can present as widespread adds/removes rather than "nothing
changed" or "one field changed".

**Cause.** Design spec §3.2, §9: no special-case re-import detection is
built. The corpus test in `crates/knx-app/tests/project_diff.rs`
demonstrates the property that *does* hold — two independent imports of
the *same* `.knxproj`, by this repository's own importer, produce an
empty diff, because this importer's own `RefId` mapping is stable
run-to-run. Whether ETS's own `RefId` regeneration would break that
stability is untested — no such case has been observed in this
repository's corpus. T15 additionally compared the ETS4 and ETS6 re-exports:
among unique device-address, building-path, and group-address natural keys,
zero shared keys carried changed ETS ids. That measures the available
re-export pair but still provides no regenerated-id case to design against.

**Impact.** A `.knxdb` re-created from a re-exported `.knxproj` whose
`RefId`s changed may compare as a large, misleading set of adds/removes
against the original `.knxdb`, even where nothing meaningful changed.

**Lifted when.** Open. Would need either a documented, stable KNX
`RefId`-regeneration rule to compensate for, or a demonstrated real-world
case to design against; neither exists yet.

## 55. Project diff cannot merge or apply a diff back onto a project

**Limitation.** `diff_projects` computes and shows what changed; it does
not turn a `ProjectDiff` back into a `Command` sequence that could replay
one project's changes onto another.

**Cause.** Applying a two-way observation is a merge engine, not a renderer
extension. It needs conflict semantics, revision checks, inverse commands
for every applicable field, and a policy for additions/removals and
ambiguities. Some fields (including `product_ref`/`program_ref`) have no
command today. A partial implementation could silently corrupt project
data, so T15 deliberately does not build one.

**Impact.** Reviewing a diff and then manually re-applying the same
edits to another project remains a manual, error-prone step; there is no
"apply this change" control anywhere in the diff panel.

**Lifted when.** Open. Requires a separately designed, atomic merge plan
with complete conflict and undo semantics; no task currently supplies one.

## 56. Project diff does not do a three-way comparison

**Limitation.** `diff_projects` takes exactly two projects. There is no
common-ancestor-aware three-way comparison the way a VCS merge does one.

**Cause.** Nothing in this codebase tracks project ancestry or a common
base. Three-way display could be added independently, but any useful merge
action also depends on §55's unresolved conflict/atomic-apply machinery.

**Impact.** Reconciling two independently edited copies of the same
original project has no tool support beyond running the two-way diff
twice, once against each candidate.

**Lifted when.** Open. Requires an explicit base-project contract first;
merge behavior additionally waits for §55. T15 does not pretend that
running two unrelated two-way comparisons creates a three-way result.

## 60. Project diff's web panel lists entities, but pages large tables

**Status.** Largely lifted (CT-1, 2026-09-28). `ProjectDiffPanel.tsx`
keeps the grouped-count summary lines and adds, below them, one
collapsed-by-default disclosure per non-empty table and installation
(`ProjectDiffDetails.tsx`, projection in `projectDiffView.ts`). Expanded,
a table lists each added, removed, changed and ambiguous entity by its
natural key (area `1`, line `1.2`, device address plus ETS id, group
address in the user's notation plus ETS id, group-range bounds, building
path). A changed entity shows how it was matched (ETS id or natural key)
and its before/after table; an ambiguous one shows its candidate counts
on each side; a changed device nests its communication-object and
parameter tables the same way.

**Accessibility.** Disclosures are native buttons with `aria-expanded`/
`aria-controls`, so Enter and Space come from the platform. Every status
is spelled out as a word and prefixed by a symbol (`+`, `−`, `~`, `?`);
colour only reinforces it. English and German catalogues cover every new
string. Escape still closes the report first, as before.

**What remains.**

- **Paging, not virtualisation.** A table renders 50 rows, then a
  "Show more" control reveals 50 more at a time and moves focus to the
  first new row. A user who expands a table of thousands and keeps
  clicking will eventually render all of them in the DOM.
- **No search or filter** inside the diff view, and no jump from a diff
  row to the entity in the Project Explorer.
- **Keyboard activation is verified structurally in Vitest.** happy-dom
  does not synthesize a button's Enter/Space activation, so the tests
  reproduce the browser's rule (an uncancelled Enter/Space keydown on a
  focused `<button>` clicks it). No Playwright run covers the panel yet,
  and no screen reader was used to check it.
- The panel never applies or merges a diff (§55) and has no three-way
  mode (§56). Raw `.knxproj` inputs are accepted since CT-6 (§57).

<a id="61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard"></a>

## 61. The DPT codec covers thirty main types with explicit input formats and disclosed encoding rulings

**Limitation.** `crates/knx-core/src/dpt/codec.rs` (2026-09-11, T29;
extended 2026-09-13 and 2026-09-14, E4, twice) can decode and encode main types **1
through 30 inclusive, with no gaps** — thirty main types, counted from
`codec.rs`'s own `decode`/`encode` match arms
(`grep -cE '^        [0-9]+ => decode_' crates/knx-core/src/dpt/codec.rs`
→ `30`). Main type 31 and everything above it returns
`DptCodecError::UnsupportedDpt` unconditionally; nothing about those is
guessed. The eighteen 200-series LTE/system types are an **accepted scope
decision**, not an open codec backlog: neither `knx-core` nor `knx-net`
implements LTE addressing, so standalone value codecs would not form a usable
system.

**2026-09-21 (T07): the bus-facing encoder no longer infers the input
grammar.** `encode(dpt, input, DptInputFormat)` requires the caller to declare
`Canonical`, `Decimal`, `Hexadecimal`, `Binary`, or `Text`. Radix prefixes are
not format declarations and are rejected by this path. CLI callers can pass
`--input-format`; HTTP callers can send `inputFormat`; the web client sends a
deterministic DPT-family choice. If an older CLI or HTTP caller omits the field,
that compatibility boundary deliberately calls `encode_inferred_format` so
fixed-width binary bit sets retain their previous wire value. The web form
exposes the same compatibility mode visibly as `Auto` and lets the user select
each explicit grammar; it never labels arbitrary text with a guessed format.
`default_input_format(dpt)` remains a deterministic recommendation for callers
that deliberately want one; it depends only on DPT identity and never on value
text.

The public `encoding_rulings(dpt)` API returns stable identifiers, the exact
Standard context, and KNXBench's decision for every affected encoding below.
That makes the distinction observable before a write instead of burying it in
this document. The inventory is: `scaled-angle-linear-mapping` (5.003),
`status-mode-format-range` (6.020), `invalid-sentinel-precedence` (8.010 and
9.*), `scene-number-is-wire-value` (17.*, 18.*, 26.*),
`datetime-src-is-reserved` and `datetime-invalid-fields-keep-width-only`
(19.*), `format-level-validation-only` (20.*, 21.*, 22.*, 23.*, 25.*, 27.*,
30.*), `strict-null-termination` (24.*, 28.*), and
`signed64-range-typo-corrected` (29.*). This is metadata only: T07 changed no
wire encoding.

Against the ETS master data that number reads differently, and the
difference is worth stating plainly because it has been misread before.
`knx_master.xml` defines 46 *main types* (`docs/RESEARCH.md` §5) — 46 is a
count, not an identifier, and there is no "main type 46": its 46 ids are
`DPT-1` through `DPT-23`, then `DPT-25`, `DPT-26`, `DPT-27`, `DPT-29`,
`DPT-30`, then eighteen LTE/system types in the 200-series (`DPT-206`,
`DPT-217`, `DPT-219`, `DPT-222`, `DPT-229`, `DPT-230`, `DPT-232`,
`DPT-234`, `DPT-235`, `DPT-237`, `DPT-238`, `DPT-240`, `DPT-241`,
`DPT-244`, `DPT-245`, `DPT-249`, `DPT-250`, `DPT-251`). So of those 46 the
codec now covers **28** — every one below 200 — and the eighteen
200-series types remain unsupported. The other two main types the codec
implements, **24** and **28**, are defined in DPT-AS (§3.24, §3.27) but are
absent from that master-data file, which is exactly the asymmetry
`docs/RESEARCH.md` §5 warns about: the master file's catalogue is a
property of *that file*, not of the Standard.
(`docs/IMPLEMENTATION_STATUS.md`'s T29 entry says "fourteen" — the true
count as of T29's date, 2026-09-11; the E4 rounds of 2026-09-13 and
2026-09-14 added
the rest. This heading states the current total, re-measured, not the
count at any one task's snapshot in time.)

**No subtype-level exclusion remains inside an implemented main type.**
`6.020 DPT_Status_Mode3` used to be one: its wire layout (`B5N3` — five
status bits plus a one-hot three-bit mode field, DPT-AS §3.7) fits none of
`DptValue`'s pre-existing shapes, so it returned `UnsupportedDpt` rather
than being misread as the plain signed 8-bit integer the rest of main type
6 is. The second E4 round (2026-09-14) gave it its own
`DptValue::StatusMode3` variant, so every subtype of every implemented
main type now decodes. No subtype-level exclusion inside an implemented
main type is known any more; if one is found, it belongs in this
paragraph.

**2026-09-13 (E4): main types 4, 10, 11, 15, and 19 added, each with one
Standard-reading judgment call recorded here rather than silently
decided.** `4.*` (`A8`, DPT-AS §3.4) reuses the ASCII/ISO-8859-1
charset-selection logic main type 16 already had (`char_set_is_ascii`),
rather than a second implementation of the same rule; unlike `16`, `4`
gets no bare-main-type default, because the Standard does not print one.
`10.001` (time of day + day of week, DPT-AS §3.11, page 41) represents
day-of-week `0` as "no day" (`Option::None`) rather than as Monday — DPT-AS
§3.11's own Day column prints this directly: `1 = Monday ... 7 = Sunday`,
`0 = no day`, range `[0...7]` ([D], not inferred; the Markdown extraction of
that table truncates at "7 =", which is how an earlier draft of this note
mis-called it undocumented). Only the storage shape is this codec's own
choice ([A]) — the Standard names the code, not a Rust type — and the round
trip is exact (`None` only ever decodes from and encodes back to raw `0`).
`11.001` (date, DPT-AS §3.12) resolves the
two-digit year octet by the century-window rule DPT-AS §3.12 EXAMPLE 5
states directly: a raw value `>= 90` means `1900 +` raw (covering
1990-1999), otherwise `2000 +` raw (covering 2000-2089) — printed in the
Standard, not inferred. `15.*` (access data, DPT-AS §3.16) packs six BCD
digits plus four flag/index bits across four octets with no reserved bits
at all in this format (all 32 bits carry meaning); a BCD nibble above 9 is
rejected as `InvalidData` rather than accepted as a non-decimal digit,
since §3.16 defines the code as decimal. `19.001` (date and time, DPT-AS
§3.20) surfaces a genuine contradiction inside the Standard's own octet 1
diagram: the field-*names* row gives `SRC` (synchronisation source
reliability) bit 6, but the bit-*encoding* row directly beneath it marks
that same bit `r` (reserved), and Note 15 sides with the encoding row,
stating plainly that the seven non-`CLQ` bits of that octet are reserved
and must be zero — confirmed by rendering the source PDF page directly
(page 50) rather than trusting the Markdown extraction, which is
column-misaligned and cannot settle the question on its own. This codec
follows the encoding row and Note 15 ([A], a ruling between the diagram's
two contradictory rows, not a case of "no bit exists"), so
`DptValue::DateTime` has no `src` field; every other named flag in the
octet-2/octet-1 tables (fault,
working day, working-day-unknown, year/date/day-of-week/time-invalid,
summer time, externally-synchronized, `CLQ`) has one, so no bit this codec
*can* assign meaning to is silently dropped. Range checks on Month/Day
(octets 7-6) and Hour/Minute/Second (octets 5-3) are enforced only when
the corresponding invalid-flag says the field is valid; when a flag marks
a field "not valid," only its bit width is checked, not its documented
range, on the reasoning that a clock with no date or no time reading may
legitimately zero- or garbage-fill that octet and rejecting such a
telegram would invent a stricter rule than §3.20 states for exactly that
case. All five new types follow the same reserved-bit policy already used
by main types 1-18: a reserved bit set to anything but zero is
`DptCodecError::InvalidData`, not silently ignored or masked off.

**2026-09-14 (E4, second round): main types 20-30 and `6.020` added, with
one ruling that spans seven of them and four that are local.** The
spanning ruling first, because it is the one most likely to surprise:
**for a main type whose format is a bare enumeration or a bare bit set,
this codec validates the format and not the subtype's own table.**
([A] — a judgment call about which table governs a bare-enumeration/
bit-set format; no clause dictates it.) That covers main type 20 (`N8`,
DPT-AS §3.21), 21 (`Z8`/`B8`, §3.22), 22 (`B16`, §4.5/§8.3), 23 (`N2`,
§3.23/§4.6), 25 (`U4U4`, §8.4), 27 (`B32`, §3.26) and 30 (`B24`, §8.5).
Concretely: an enumeration code a subtype
calls "reserved" still decodes to its raw code, a bit a subtype's table
calls "reserved, set 0" still survives into the decoded value, and a field
a subtype narrows (25.1000's `[0 … 3]` inside the format's `[0 … 15]`) is
not re-narrowed. Three reasons, in order of weight: (1) Main type 20 alone
has **sixty-eight** subtypes whose tables are scattered over six clauses
of DPT-AS — §3.21 (20.001-20.022, sixteen), §4.3 (20.100-20.122,
nineteen), §6.3 (20.600-20.613, fourteen), §7.1 (20.801-20.804, four),
§8.1 (20.1000-20.1005, six) and §9.5 (20.1200-20.1209, nine) — keyed by
application domain; transcribing them into a codec means every
transcription slip silently *rejects* a legal bus value, the worst failure
direction available here. (Counted, not estimated:
`pdftotext -layout "03_07_02 Datapoint Types v02.02.01 AS.pdf" - | grep -oE
'\b20\.[0-9]{3,4}\b' | sort -u | wc -l` → 68. An earlier draft of this
paragraph said "of the order of eighty" and named chapter 10 as a seventh
location; chapter 10 is "Datapoint types for weather encoding" and holds
main types 273 and 274 only, no main type 20 subtype at all.)
(2) Their wording is not uniform: 20.001 and
20.003 say "not used; reserved", 20.002 says "reserved, shall not be
used", 22.100 says "reserved" with "default 0", 21.001 says "reserved, set
0" — three different strengths of prohibition that cannot be collapsed
into one check honestly ([D], the four quoted phrases). (3) In one
case the Standard contradicts itself outright: 22.100 `DPT_StatusDHWC`'s
encoding row (§4.5.1) reads
`0 0 0 0 0 0 0 B BBBBBBBB`, defining bit 8 as a `B`, while the data-field
table printed directly beneath the same diagram lists bits "8 to 15" as
"reserved", "default 0" — read verbatim off page 126 of the source PDF
(`pdftotext -layout "03_07_02 Datapoint Types v02.02.01 AS.pdf"`, which
preserves both the encoding row and the table beneath it), not from a
Markdown extraction ([D], this whole contradiction). §4.5.2's own
Encoding paragraph points the same way without settling the bit:
"depending on the usage of
this DPT in a given Datapoint, some bit-fields may be unused and set to
'0' by the sender and will be ignored by the receiver." ([D]) Nothing is
discarded by this ruling — every bit and every code reaches the caller in
`DptValue::Enum` / `DptValue::BitSet` / `DptValue::DoubleNibble` — so a
caller that *does* know its subtype can apply the table itself. The
commissioning design's `DPT_ErrorClass_System` (20.011) enumeration
(`docs/superpowers/specs/2026-09-13-commissioning-download-design.md`
§5.6, `[0 to 18]`) is exactly such a caller and remains correct at that
layer; it was checked against §3.21 during this slice and matches.

The four local rulings. (a) **`6.020`'s mode field is enforced, unlike a
subtype table** ([A]), because §3.7's Range row states `f = {001b,010b,100b}`
for the *format* itself ([D]) — so a mode field of `000b`, `011b`, `101b`,
`110b` or `111b` is `InvalidData`. Its five status bits keep the
Standard's own inverted polarity ("0 = set, 1 = clear") rather than being
normalised, and print as binary digits for that reason. (b) **Main types
24 and 28 (`A[n]`, NUL-terminated, DPT-AS §3.24 and §3.27) reject a
payload without a terminating `00h`, and a payload with an interior
`00h`.** ([A]) Neither section says what a receiver should do with such a
payload; every tolerant reading is a guess about a non-conforming sender's
intent (cutting at the first `00h` assumes the remainder is padding;
appending a terminator assumes one was lost), so the codec refuses rather
than invents. The same rule makes an embedded `U+0000` unencodable, which
is a real gap for main type 28: §3.27's Range row includes `U+000000`, but
the format cannot transmit it unambiguously. This codec still imposes no
maximum encoded length of its own (§3.24/§3.27's own "cut to the maximum
supported length" rule for an over-long *incoming* string is a receiver
concern, not this codec's), but the layer that owns the APDU budget now
exists and enforces it: `knx_net::cemi::encode_l_data` refuses an NPDU
whose length does not fit the one-octet `L` field with `CemiError::
NpduTooLong` rather than emitting a frame whose `L` octet silently
wrapped **[V]** (fixed 2026-09-14, T5 fix round 1, finding I1). (c) **Main type
29's printed
range is a typo, and this codec follows the datapoint-type rows instead of
the format block.** ([A]) §3.28.1's Range row reads "SignedValue = [9 223 372
036 854 775 808 to 9 223 372 036 854 775 807]" ([D]) — the lower bound has lost
its minus sign, and as printed the range is empty and cannot fit 64 bits
anyway. The 29.010/29.011/29.012 rows in the same clause print
"-9 223 372 036 854 775 808 Wh to 9 223 372 036 854 775 807 Wh", exactly
`i64`'s domain, and that is what is implemented. (d) **Main type 26
carries the wire scene number undecorated** ([A]), like main types 17 and 18 —
see the scene-number paragraph below; §3.25 NOTE 16 is this type's own
note rather than a borrowed one, and the answer does not change.

Payload shapes for the new types follow the AL-AS §3.1.2/§3.1.3 six-bit
inline threshold with no exceptions: main type 23 (2 significant bits) is
the only one of the eleven that travels inline as `GroupValue::Short`;
main type 26 has 7 significant bits and therefore always occupies its own
octet, the same reasoning main type 18 already used. Reserved bits that
*are* format-level — main type 26's bit 7 — are checked and a set bit is
`InvalidData`, consistent with main types 1-19.

There is no DPT main type 46, and this slice was briefed to implement one
— see [§90](#90-there-is-no-dpt-main-type-46-46-is-a-count-of-main-types-in-one-ets-master-data-file) for the search that established that and why the
number was plausible enough to survive into a brief.

**Resolution is inference, not a stated fact.** `resolve_group_address_dpt`
and `resolve_project_group_address_dpts`
(`crates/knx-core/src/dpt/resolve.rs`) derive a group address's DPT by
scanning every communication object linked to it and reading `dpt.value()`
off each — a group address does not carry its own type in this domain
model (except see the next paragraph). Per `docs/RESEARCH.md` §6.1, this
inference is genuinely incomplete: **194 of 514 group addresses (38%)**
in the `Unser Zuhause` reference project resolve to no DPT at all, and
**110 of 514 (21%)** have no linked communication object at all to infer
from. A conflicting set of linked DPTs is reported as
`GroupAddressDpt::Conflict` and never resolved down to one guess — RESEARCH
§6.1's rule 3.

**`GroupAddress/@DatapointType` exists at schema ≥ 21 and is preserved but
not modelled.** ETS versions that write schema 21 or later can state a
group address's DPT directly on the `GroupAddress` element itself, instead
of requiring inference from a linked communication object. This importer
preserves that attribute (opaque passthrough, ADR-0006) but does not read
it into the domain model or consult it for resolution — resolution is
inference-only, as above, even on a project where the group address said
its own type all along. Measured directly against the fixture projects:
`KV v2.5 - demo.knxproj` (schema 21) carries the attribute on **13 of 13**
group addresses; neither `Unser Zuhause` export (schema 11, and the
schema-23 re-export of the same installation) carries it on **any of
514**.

**Two sentinel collisions the Standard does not resolve, where the codec
picked one reading and says so.** `8.010 DPT_Percent_V16`'s printed maximum
(327.67%) and its printed invalid-data code are the identical 16-bit value
(`0x7FFF`); the codec honours the invalid-data sentinel unconditionally, so
`8.010`'s practical maximum is **327.66%**, one step below the number
DPT-AS itself prints. Main type 9 (F16, floating point) has the same
collision at its arithmetic ceiling: `M = 2047, E = 15` is bit-identical to
`0x7FFF`, the reserved invalid-data code, so `encode` rejects that one
value and the family's usable maximum is **670433.28** (at `M = 2046, E =
15`) times the subtype's unit — which is exactly the figure DPT-AS itself
prints for the family, while application note AN188 §4 prints the larger
**670760.96** by not accounting for the collision. Neither collision is
settled by the Standard; both entries record which reading this codec
ships and why.

**Scene numbers are carried at wire value; no display offset is applied.**
DPT-AS §3.19 NOTE 9, attached to `18.001 DPT_SceneControl`, recommends
*displaying* a scene number with an offset of +1 (§3.25 NOTE 16 makes the
same recommendation for `26.001 DPT_SceneInfo`, implemented since the
second E4 round). No equivalent note exists for `17.001 DPT_SceneNumber`
in §3.18. The codec applies no +1 to main type 17, 18 or 26: a
decoded value means the octet it came from, not a display convention layered
on top of it. Any UI presenting a scene number to a human owns that +1
itself — applying it a second time here would make the wire value and the
displayed value silently disagree. (NOTE 9 itself is absent from this
corpus's Markdown extraction of the Standard; it was confirmed to exist
against the source PDF. A previous round of this work briefly asserted §3.19
carried no such note — that assertion was wrong and has been corrected.)

**`DPT-16`'s fixed 14-octet field has no length indicator.** A string
containing an interior NUL byte followed by further content is not
representable: decode strips a trailing run of `0x00` as padding (DPT-AS
§3.17: "unused trailing octets... shall be set to NULL"), because nothing
in the Standard's definition of this type provides an escape sequence or a
length prefix that would let interior NUL survive. This is a recorded gap,
not a rule invented to paper over it.

**A payload with bits set above a short type's significant width is
rejected, not masked.** Where a `GroupValue::Short` carries more bits than
its DPT's definition assigns meaning to, the codec returns
`DptCodecError::InvalidData` rather than silently discarding the
out-of-range bits — a device sending such a telegram gets it printed raw,
with the rejection reason, instead of a decoded value that quietly hides
what the device actually sent.

**`bus monitor`/`bus write` only decode/encode when they have a DPT to work
with.** `bus monitor` decodes only when given `--project <path>` — the DPT
comes from resolving the project's linked communication objects, and there
is nowhere else to get it from; without the flag, the monitor prints
exactly what it printed before this slice. `bus write` needs either
`--project` (to resolve one) or an explicit `--dpt <DPST-m-s>`. Its input
grammar is explicit through `--input-format`; omission deliberately selects the
named legacy compatibility parser so existing writes keep their bytes.

**Subtype wording and units beyond the scaled subtypes are not modelled.**
The codec does not consult `knx_master.xml`'s DPT catalogue, so it has no
source for a subtype's displayed unit beyond what a scaled subtype's own
arithmetic already implies (e.g. `%`, `°C`), and no source for enumeration
wording (`up`/`down`, `open`/`close`, and similar per-subtype vocabulary).
A decoded `DptValue` is a typed number, boolean, or string — not a
formatted, unit-labelled, human-worded string.

**No decoded value has been verified against real hardware.** Every test in
this slice checks the codec against the Standard's own stated encodings
(round-trip tests, boundary tests, the two sentinel rulings above) — not
against a telegram a real KNX device actually produced. That is a narrower
claim than "matches what real devices send," and this entry exists so the
difference is not lost.

**Cause.** The remaining limitations are scope decisions and documented
Standard ambiguities (design spec
`docs/superpowers/specs/2026-09-11-dpt-codec-design.md`, decisions E4-D1
through E4-D9): implement main types the Standard extraction documents
unambiguously and the reference corpus needs, leave the rest
`UnsupportedDpt` rather than guess, and record every place the Standard
itself is ambiguous or self-contradictory rather than resolve it silently.

**Impact.** A user working with a group address whose DPT falls outside
the thirty implemented main types, or whose linked communication objects
disagree, or who has none at all, sees `bus monitor` fall back to the
pre-T29 raw output for that address. A user relying on `8.010`'s printed
327.67% maximum, or AN188's 670760.96 figure for main type 9, will see this
codec's numbers differ by one step, deliberately. Code preparing a write can
query `encoding_rulings` and present the relevant judgment before sending.

**Lifted when.** The accepted LTE/system scope changes, a future slice consults
`knx_master.xml` for units and enumeration wording, or reads
`GroupAddress/@DatapointType` directly for schema ≥ 21 projects instead of
inferring from linked communication objects alone. The input-format inference
part is lifted by T07; the explicitly named compatibility helper remains
opt-in.

## 62. The Group Monitor GUI (T15) is tunnelling-only, single-session, client-filtered, and only its passive receive path has real-gateway evidence

**Limitation.** T15 (2026-09-11, design spec
`docs/superpowers/specs/2026-09-11-group-monitor-design.md`) gives
`apps/knx-server`/`apps/knx-web` a live telegram table and a compose/send
form. What it ships is narrower than "a Group Monitor," in the following
ways, all deliberate and all recorded here per that design's own §7:

1. **Tunnelling only.** `GatewayConnector`/`BusTunnel`
   (`apps/knx-server/src/bus.rs`) expose only the two operations a
   monitor session needs from a `TunnelClient` — nothing reaches
   `RoutingClient`. `route-monitor` stays CLI-only.
2. **No auto-reconnect.** A gateway-side disconnect (`TunnelEvent::Closed`
   or the broadcast channel closing) marks the session `closed` and stops
   the drain task; nothing reopens the tunnel automatically. The user
   restarts explicitly.
3. **No live re-resolution of the DPT map.** The group-address/DPT map is
   computed once, from the project open in `AppState` at
   `BusSession::start`, and cached for the session's life. Editing the
   project (renaming a group address, changing a DPT override) while a
   session is running does not change already-decoded rows, and new rows
   keep using the start-of-session snapshot until the session is
   restarted — inherited from `apps/knx-cli bus monitor`'s existing
   behaviour (§29 below), more likely to surprise a GUI user who can edit
   and monitor in the same window.
4. **One session per server process.** `AppState.bus_session:
   Mutex<Option<BusSession>>` holds at most one; a second
   `POST /api/bus/monitor/start` while one is active is `409 Conflict`,
   naming the existing session, never a silent second connection to the
   gateway.
5. **No persistence of the telegram buffer.** It is purely in-memory,
   capped at `MAX_TELEGRAMS = 5000`; stopping a session and starting a
   new one begins a fresh buffer and a fresh sequence counter at 0. A
   server restart loses whatever was buffered.
6. **No server-side filtering.** `GET /api/bus/monitor/telegrams` always
   returns everything from `since` forward; the text filter over
   destination/name and the service-type checkboxes
   (`apps/knx-web/src/BusMonitorPanel.tsx`) apply only to what the
   browser already fetched. This is nothing like ETS's own Group Monitor
   filter (multiple simultaneous criteria, sender/receiver-specific,
   saved filter sets) — it is a visibility toggle over an already-fetched
   table, not a query language.
7. **`Destination::Individual` frames are not rendered as rows.** The row
   model (`destinationName`, DPT resolution) assumes a group address;
   an individually-addressed frame reaching this path is dropped before
   becoming a row — not counted against `droppedBefore`, since this is a
   declared scope exclusion, not a loss (`bus::tests::individual_addressed_frames_are_not_rendered_as_rows`).
8. **DPT/enumeration coverage.** Inherited unchanged from
   [§61](#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) —
   this slice does not touch the codec. §61 is not edited, reworded, or
   superseded by this entry; it still fully applies to every decoded
   value the GUI shows.
9. **Passive receive has one real-gateway verification; transmit paths do
   not.** On 2026-09-16 the production `POST /api/bus/monitor/start` →
   `BusSession::start` → `RealConnector` path opened a tunnel to a
   user-supplied gateway on the installation LAN. A 133-second session with
   the server's empty project received 52 group telegrams and stopped with
   `droppedCount = 0`. A second 107-second session, after importing the real
   schema-23 `Unser Zuhause` reference project into that same dedicated
   server, received 65 telegrams from 9 source addresses to 21 group
   destinations: all 65 destination names resolved against the project, 10
   values decoded through their DPT, no conflict or decode error was observed,
   and `droppedCount` again remained 0. Both sessions stayed `active` until an
   explicit successful stop; the gateway assigned a tunnel address and no
   disconnect or reconnect occurred. The run only called monitor start, poll,
   and stop. It sent no group read, write, response, management request, or
   scan; `/api/bus/write` was never called. The private gateway address and
   observed bus addresses are deliberately not stored in the repository.
   This is evidence for the passive tunnelling receive and project-resolution
   path on one gateway model, not for routing, transmit behavior, reconnect,
   another gateway, or long-running stability. The exact procedure is recorded
   in `.ai/logs/2026-09-16_codex_group_monitor_reverify.md`.

   **Re-verified independently 2026-09-16 → 2026-09-19 (Task 16, second
   pass, a restatement against new measurement — the run surfaced no
   defect, so nothing here needed fixing).** The measured session was not
   started fresh by this task: it was already running, against the same
   real gateway with the real schema-23 `Unser Zuhause` project open, for
   roughly 30 minutes before this task picked it up mid-flight, polled it,
   and issued the stop. From first telegram to stop, the session had been
   open for 2040 seconds (34 minutes), well past the earlier two- and
   one-minute runs. Result: 1299 telegrams from 20 source addresses to 61
   group destinations, `droppedCount = 0`, all 1299 destination names
   resolved against the project (201 `GroupValueRead` carrying no
   value to decode, against 201 `GroupValueResponse` and 897
   `GroupValueWrite` that do), and 209 of the 1098 value-bearing telegrams (897 +
   201) decoded through their DPT. The other 889 came back `Unresolved`
   — traced to `GroupAddressContext::decode` (`apps/knx-server/src/
   bus.rs`): that variant fires when `self.dpts.get(&ga.raw())` is `None`
   or `GroupAddressDpt::None`, i.e. the project declares no DPT for that
   group address at all. It is not a main-type gap: an address using a
   main type outside §61's implemented thirty would decode through
   `decode_single` into `DecodedValue::Error`, a different kind, and none
   appeared in this run. So roughly 81% of the value-bearing
   telegrams in this run were addressed to group addresses carrying no
   declared DPT — a share of telegrams, not of addresses: 61 destinations
   produced those 1098 telegrams, and no address-level count was measured,
   so nothing here says what fraction of the project's group addresses
   lack a DPT. A fact about this project's data either way, not about
   §61's codec coverage, and §61 is not touched by this entry. This also exercised, for the first time, the item-4
   single-session guard against a live gateway rather than only against
   `FakeConnector` in unit tests: a second `POST /api/bus/monitor/start`
   issued to the *same* server process while the first tunnel was open was
   refused with `409` and the existing session's id, exactly as item 4
   describes. A related, previously undocumented fact surfaced by
   accident: a *second, independent* `knx-server` process attempting its
   own tunnel to the same physical gateway while the first tunnel was open
   was refused by the gateway itself — KNXnet/IP `CONNECT_RESPONSE` status
   `0x24` (`E_NO_MORE_CONNECTIONS`) — before our own single-session guard
   ever ran. The gateway used for this verification accepts exactly one
   concurrent tunnel connection; two separate `knx-server` instances (or a
   `knx-server` and a `knx-cli bus monitor` run) pointed at it will collide
   at the hardware, not just inside this application. No group read, write,
   response, management request, scan, or `/api/bus/write` call was made in
   either session; the gateway address is deliberately not stored here.
   Of §62's original four headline claims (tunnelling-only, single-session,
   client-filtered, "never verified against a real gateway"), two now have
   live evidence from this pass specifically: single-session, from the
   `409` above, and "never verified", now false twice over (2026-09-16 and
   2026-09-19). The other two — tunnelling-only and client-filtered —
   remain confirmed by code inspection, not by this run: it touched no
   routing code and the poll route still takes only `since`, but it never
   tried to exercise routing or server-side filtering, so it is consistent
   with those claims rather than a live test of them. What remains
   unverified is unchanged: routing, transmit behavior, reconnect after a
   mid-session failure, other
   gateway models, and sessions longer than 34 minutes.
10. **No KNX certification or ETS-parity claim.** This is a monitor/write
    table, not a certified diagnostic tool, and not a claim of matching
    ETS's Group Monitor feature-for-feature — see item 6 above for
    exactly where the filtering falls short.

Three further limitations, ruled during this cycle's review and not in
the design document's own §7:

11. **The browser keeps every polled row for the life of a session, with
    no cap.** `apps/knx-web/src/BusMonitorPanel.tsx`'s poll handler does
    `setRows((previous) => [...previous, ...response.telegrams])` on every
    tick, and only ever resets on a fresh `connect()`. The server's own
    buffer is capped and honestly reports what it evicted
    (`droppedBefore`); the browser's row list is not. This was a
    deliberate choice, not an oversight: capping it client-side would
    need the browser to make its own eviction decisions on top of the
    server's, and a client-side gap notice that could disagree with the
    server's `droppedBefore` accounting is worse than the memory growth —
    two independent "what did we lose" answers in one UI is exactly the
    kind of silent-disagreement risk CLAUDE.md's "never silently discard
    information" rule is trying to prevent, applied here to *honesty about
    loss* rather than to loss itself. A long session against a busy
    installation will grow the browser tab's memory without bound; there
    is no cap and no warning about this specific growth today.
12. **The `/write` round trip is verified for two of three
    group-address styles.** `POST /api/bus/write` parses `destination` in
    the open project's own configured `GroupAddressStyle` (fixed
    2026-09-11, commit `b540264`, after `/write` was found hardcoding
    `ThreeLevel` regardless of the project). A regression test,
    `a_non_three_level_projects_telegram_destination_round_trips_through_write`
    (`apps/knx-server/tests/http_bus_write.rs`), drives the full
    `/telegrams` (a telegram arrives, is rendered) → `/write` (the
    rendered string round-trips back through `/write`) path for `Free`
    and `TwoLevel` styles. `ThreeLevel` — the project default — is
    exercised by a different test
    (`write_with_an_explicit_dpt_sends_the_encoded_value_through_the_open_tunnel`)
    that calls `/write` directly with a hand-typed `"0/0/1"` destination;
    it proves the same parse path accepts three-level addresses, but not
    the full receive-then-echo-back round trip the other two styles get.
13. **`apps/knx-cli` has the same group-address-style bug this branch
    fixed on the server, left alone on purpose.** `apps/knx-cli/src/
    main.rs`'s `bus write`/`route write` still parse a destination with
    `knx_core::GroupAddressStyle::ThreeLevel` hardcoded (e.g. lines 1666,
    1897), regardless of the open project's own style — the identical bug
    `b540264` fixed in `knx-server`. It was deliberately not fixed here:
    CLAUDE.md's "do not perform unrelated refactors while implementing a
    feature" argues against reaching into a sibling binary mid-branch for
    a bug this branch's own scope did not require touching. See
    [§29](#29-appsknx-cli-bus-monitor-has-formatting-limitations)'s
    2026-09-11 (T15) update for the record.

**Cause.** Scope decisions for this slice, argued in the design document's
§3/§7 and in this cycle's own review; items 11-13 were found and ruled on
during review, after the design document was written.

**Impact.** A user gets a live, DPT-decoded telegram table and a
send-from-the-table form for one tunnelled gateway at a time, with a
client-side text/service filter. The passive receive and project-resolution
path now has bounded real-installation observations from two dates, the
longer one running 34 minutes, but the send form still has only fake-tunnel
coverage. This remains neither a certified diagnostic tool nor ETS's Group
Monitor and — for a very long browser session — is not bounded in memory
the way the server side already is.

**Lifted when.** Future slices add routing support, auto-reconnect, live DPT
re-resolution, multi-session support, server-side filtering, a client-side row
cap with its own honestly-reported gap notice, a full-round-trip test (and,
ideally, a fix) for the CLI's `ThreeLevel` hardcoding, and separately authorized
real-installation evidence for transmit behavior and longer-running stability.

## 63. `knx-server` has no multi-user/concurrent-edit support — one shared project, one shared undo stack, no conflict detection at all

**Limitation.** `apps/knx-server`'s web/Docker deployment target holds
exactly one project in one process-wide `AppState`, constructed once and
shared by every connected browser for the life of the process:
`Arc::new(knx_server::AppState::new(data_dir))`
(`apps/knx-server/src/main.rs:54`), `pub type SharedState = Arc<AppState>`
(`apps/knx-server/src/lib.rs:42`), handed to the router with
`.with_state(state)` (`apps/knx-server/src/lib.rs:153`). There is no
per-session and no per-connection project state. Since 2026-09-20 a
middleware *does* read a session cookie out of a request
([§22](#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback),
[ADR-0026](adr/0026-server-authentication-or-loopback.md)) — but every
valid session was opened with the same single password, so a session
identifies a browser rather than a person, and nothing downstream of the
guard ever sees which session a request arrived on. The server still
cannot tell two operators apart. Verified concrete consequences:

1. **A second client's undo can undo the first client's command.**
   `command_stack: Mutex<knx_core::CommandStack>`
   (`apps/knx-server/src/domain.rs:48`) is one stack for the whole
   process; `undo_impl`/`redo_impl` (`apps/knx-server/src/domain.rs:2008-2027`)
   pop/replay whatever is on top of it without regard to which client
   pushed it there. Nothing associates a stack entry with the client that
   created it.
2. **No write route carries any optimistic-concurrency check.** No ETag,
   `If-Match`, version/revision counter, or "expected current value"
   field exists on any route in `apps/knx-server/src/routes.rs`,
   `domain.rs`, or `fs_routes.rs` — every command-applying function
   (`apply`, `apps/knx-server/src/domain.rs:1126-1142`; `undo_impl`/
   `redo_impl`, `:2008-2027`; `save_project`/`save_project_as`, `:546-588`)
   reads and mutates the shared state unconditionally, with no way for a
   client to say "only if nothing changed since I last looked."
3. **No client is told the project changed underneath it.** There is no
   `WebSocket` or `EventSource` anywhere in `apps/knx-web`; the only
   `setInterval` call in the whole frontend (verified with `grep -rn
   setInterval apps/knx-web/src`, one hit, no test-file matches) is
   inside `BusMonitorPanel.tsx`'s telegram-polling `useEffect`, calling
   `poll()` on `POLL_INTERVAL_MS` — line 386 as of this writing, but the
   line number is not the citation to trust: this exact line has drifted
   twice before while the fact underneath it held, so re-run the grep
   above rather than trust either number. It polls bus telegrams, not
   project state. A browser's view of the project tree only updates
   from the response to its own request — it never learns about another
   client's edit, undo, redo, or save except by the user manually
   reopening the project.
4. **File-level save is plain last-writer-wins, silently.**
   `save_project`/`save_project_as` (`apps/knx-server/src/domain.rs:546-588`)
   both funnel into `knx_store::save_project`
   (`crates/knx-store/src/project.rs:72`), which unconditionally
   `DELETE`s every row of every project table and reinserts the current
   in-memory project inside one transaction (`crates/knx-store/src/project.rs:93-97`)
   — no check against what is currently on disk, no file lock. Two
   clients saving the same `.knxdb` path (via `store_path`,
   `apps/knx-server/src/domain.rs:32`) end with whichever transaction
   commits last silently discarding the other's work; neither client is
   warned.

**What is protected.** `apply`, `undo_impl`, and `redo_impl` each take the
same `state.project`/`state.command_stack` locks for the full duration of
one command (`apps/knx-server/src/domain.rs:1132-1134` for `apply`,
`:2008-2027` for `undo_impl`/`redo_impl`), so
two simultaneous requests cannot interleave into a torn or corrupted
in-memory `Project` — one command always finishes before the next one
starts. That is a real, verified guarantee of memory-level consistency
for a single command. It does not protect a user's mental model of the
project, a browser's now-stale view of the tree, the one shared undo/redo
history, or a `.knxdb` file from last-writer-wins.

This is a limitation of the web/Docker deployment target specifically,
where `main.rs` binds `0.0.0.0` and any number of browsers can reach the
one process. The Tauri desktop shell constructs the identical
`Arc<knx_server::AppState>` type — `state: Arc<knx_server::AppState>`
and `Arc::new(knx_server::AppState::new(data_dir))`
(`apps/knx-desktop/src-tauri/src/lib.rs:31,57`) — so it shares this
limitation's state *shape*, not a different design. What makes it
single-user in practice is deployment, not architecture: its embedded
server binds `127.0.0.1` for exactly one locally-spawned webview window
(`apps/knx-desktop/src-tauri/src/lib.rs:60-73`), so no second, remote
client can ever reach it.

**Cause.** `knx-server`'s state model (one project, one `Mutex`-guarded
`AppState`) was built for a single open project per process, the
assumption the desktop app started from; the web/Docker deployment (see
[§22](#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback)'s
original design spec) reused it as-is. Session isolation, locking, or merge
logic were never added. ADR-0026 has since added authentication, which was
the prerequisite named here — but deliberately as one shared password with
no user model, so it supplies a session without supplying an identity, and
this limitation is exactly as true after it as before.

**Impact.** A `knx-server` deployment reached by more than one person at
once has no conflict detection, merge, or locking: one person's undo can
remove someone else's change, one person's save can silently overwrite
another's, and neither browser shows any sign that the other exists or
that a change came from outside its own actions.

**Lifted when.** **T22** (multi-user/concurrent-edit support for
`knx-server`) is designed and implemented. Per its own backlog entry
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md#f-non-functional--operational-gaps)),
it "needs its own design (locking vs. merge vs. last-writer-wins, and
what 'conflict' even means for a `Command`-based undo model)" — that
design question is unresolved, and this limitation stands until it is
answered and built.

## 64. `Languages` blocks outside an application program are discarded on import

**Resolved for ingestion (2026-09-12, T32); reading closed for every
entity family this project's corpus has found a `Master`-scope
translation for (2026-09-14, T13) — the residue below is what is left.**
The heading is kept verbatim because five documents link to its anchor;
read the status here, not in the title.

**Ingested now.** `translation` was widened in schema v4 to `(scope,
scope_id, language, ref_id, attribute_name)`
(`crates/knx-productdb/src/migration.rs`), with `''` as the master-scope
`scope_id` sentinel — `knx_master.xml` has no owning element, and SQLite
treats NULLs in a non-`INTEGER` primary key as pairwise distinct, so the
one thing a sentinel is needed for is the one thing NULL will not do. A
single `ingest_translations` pass (`parse/translation.rs`) now reads the
`Languages` block of `Catalog.xml` and `Hardware.xml` (keyed by
`Manufacturer/@RefId`) and of `knx_master.xml` (`FileKind::MasterData`,
master sentinel). `parse/program.rs` keeps its own inline handling
unchanged. A v3→v4 backfill replays the blobs already stored, so a
database installed before this slice does not stay translation-less;
a blob that fails to parse records itself into `ingest_unknown` as a
`TranslationBackfillError` and the migration continues.

Re-measured on the same package this section first cited,
`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`, by installing it with
`knx products ingest` and counting `translation` rows per scope:

| scope | rows | distinct languages |
|---|---|---|
| `Catalog` | 40 | 5 (`de-DE`, `en-US`, `es-ES`, `fr-FR`, `it-IT`) |
| `Hardware` | 30 | 5 (same five) |
| `Master` | 1635 | 18 |
| `Program` | 18546 | 5 |
| **total** | **20251** | |

The 1705 rows this section was opened for — 40 + 30 + 1635 — are in the
database. The golden corpus assertion moved with them: 48,190 rows for
the reference project (48,057 program + 109 catalog + 24 hardware),
re-measured rather than predicted.

**Correction to the earlier figure.** The table previously published here
said `knx_master.xml` carried 1635 translations in **24** languages. The
row count was right; the language count was not. The file's single
`<Languages>` block holds those 1635 translations across **18**
languages. The file declares 42 `<Language>` elements in total, and the
other 24 sit in a separate `<MasterData><ProductLanguages>` block — a
catalogue of language identifiers with no translations attached to them
at all. The earlier number came from grepping the whole file instead of
the block. Measured wrongly here first, corrected here now.

**Still open, narrowed once (2026-09-13, T16).** Ingestion is no longer
the gap; reading is. Two surfaces now read these rows: the catalog
browser, whose item `Name` and `VisibleDescription` are overlaid by
`query::catalog_items(conn, …, language)` behind `GET
/api/catalog/items?language=` (T32 Task 4), and — new — the device
detail panel's product/hardware block, whose `product.text` (`Product`
scope `Hardware`) is overlaid by the new `query::device_product(conn, …,
language)` behind `GET /api/device/{id}?language=` (T16, branch
`t16-device-product`), which also overlays `catalog_item.name` (same
join `catalog_items` already uses) and `application_program.name`
(`Program` scope) for the same response. `Hardware`-scope translation
rows therefore have a reader now, but the claim only narrows, it does
not close: `Master`-scope translations — the entire shared KNX
vocabulary of `knx_master.xml` — are still stored, queryable, and read
by nothing. A second, orthogonal observation, empirically
checked rather than assumed while building T16 and re-checked in review:
across nine `Hardware.xml` files from seven manufacturers — the five
packages under `OriginalData/ProductDatabases/` plus the manufacturer
packages inside the two reference ETS exports, which are one installation
exported from ETS 4 and ETS 6 rather than two independent ones — no
`Hardware.xml` places a `Hardware/@Id` inside a
`TranslationElement/@RefId`; every one of them is a `Product/@Id`. So
`hardware.name` has nothing to read rather than a missing reader, in
every package seen so far. This is a statement about the corpus, not
about the format: the schema does not forbid a `Hardware`-keyed
translation row, and one package carrying one would overturn it. And no
translated string is ever allowed to become a stored identifier —
`query::catalog_item`, the single-row lookup device creation uses, is
deliberately untranslated.

**Narrowed further, still not closed (2026-09-13, D10 slice 1, branch
`d10-master-translations`).** Three of this section's own open items
move. First, the import report now does state how many translations a
package contributed: `InstallReport`/`IngestOutcome` gained a
`TranslationCounts { program, catalog, hardware, master }`, counted from
`INSERT OR IGNORE`'s own affected-row count — a repeated `Translation`
element that the parser walks past but SQLite ignores as a duplicate key
contributes nothing to the count, exactly as `unknown_count` already
counted writes rather than sightings. `knx-cli`'s `install` output
prints the total and the per-scope breakdown. A package already
installed under the previous schema (v4) reports zero for all four
counters on a retried install rather than a guess — re-deriving the true
figure would mean re-parsing bytes this migration has no access to, so
it names the gap instead of inventing a number; a package installed
from this slice onward always gets its real count. Second,
locale-prefix matching is no longer absent — see the correction to §37
cross-referenced there; it is implemented once, in `query.rs`'s
`best_matching_language`, and reused by every overlay this function has
ever had plus the new one described next, but nothing in `apps/knx-web`
sends a bare primary-language tag yet, so the backend half closes and
the round trip does not. Third, `query::datapoint_types`/
`query::datapoint_type` is a new `Master`-scope reader — the first one —
for `datapoint_type` rows (measured non-empty on installation across all
five sampled packages: 383, 354, 234, 234 and 234 rows), overlaying a
`Master`-scope, `Text`-attribute translation onto each row's `text` when
one resolves for the requested language. It is deliberately narrow: it
only ever has rows for the `RefId` families `datapoint_type` itself
holds data for (`DPST-*`, `DPT-*`). Two of the five sampled packages'
`knx_master.xml` also carry `Master`-scope translations for `FT-*`
(function types), `SU-*` (space usages) and `FP-*_DR-*`
(functional-profile/datapoint pairs) — confirmed independently in both
(`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a`: DPST-\*=328, DPT-\*=47,
FP-\*\_DR-\*=738, FT-\*=180, SU-\*=342 of 1635 master rows;
`Dummy_Applikation_Secure`: DPST-\*=314, DPT-\*=45, FP-\*\_DR-\*=697,
FT-\*=170, SU-\*=323 of 1549) — but `parse/master.rs` parses none of
`FunctionType`/`FunctionPoint`/`SpaceUsage`: there is no table for those
`RefId`s to join against, so no reader, this one included, can surface
them. The other three sampled packages' `knx_master.xml` predates that
scheme and carries only `DPST-*`/`DPT-*` master translations, nothing
this gap touches. A translated function-type or space-usage name stays
unavailable until a later slice gives those constructs their own
tables — tracked here, not silently narrowed out of this section's
claim.

**Closed, 2026-09-14 (T13, branch `d10-language-data`).** The residue
above is what this slice closes: `FunctionType`, `FunctionPoint` and
`SpaceUsage` each get a table now (`function_type`, `function_point`,
`space_usage`; schema v9 → v10, `migrate_v9_to_v10`), filled by
`parse/master.rs`'s `ingest_master_data` the same `INSERT OR IGNORE` way
`datapoint_type` already was. `query.rs` gained `function_types`/
`function_type`, `function_points` (scoped to one `function_type_id`)
and `space_usages`/`space_usage`, each overlaying `text` from a
`Master`-scope translation through the same `master_text_overlay` every
other Master reader here already used — that function never filtered by
`RefId` prefix, so the translations these three families needed were
already sitting in `translation` since T32; only the join target was
missing. A v9 database is backfilled the same way a v3 one was for T32:
its `knx_master.xml` blob is replayed through `ingest_master_data` inside
the migration (`a_v9_database_backfills_function_and_space_usage_rows_and_their_translations`,
`migration.rs`), so a database that already existed before this slice
does not stay short these three tables' worth of data. An end-to-end
test through `install_package`
(`hardware_and_master_scope_translations_survive_install_with_their_text_intact`,
`tests/standalone_packages.rs`) reads a planted `Hardware`-scope and a
planted `Master`-scope translation's actual text back out of `translation`
after a real package install, not merely a row count. Neither reader
gained an HTTP route or a UI element — surfacing stops exactly where
`datapoint_types` already stopped (no route in
`apps/knx-server/src/routes.rs`), per this project's "surface only as far
as existing machinery already reaches" rule; a caller inside the backend
can call these functions today, nothing outside it can yet.

**Residue restated, not claimed closed.** Three things this slice does
not touch, stated plainly rather than left implicit: first, the other
eight `MasterData` child sections `parse/master.rs`'s own module doc
names (`DatapointRoles`, `InterfaceObjectTypes`,
`InterfaceObjectProperties`, `PropertyDataTypes`, `MediumTypes`,
`MaskVersions`, `FunctionalBlocks`, `ProductLanguages`) stay unparsed;
none of them carried a `Master`-scope translation in any of the five
sampled packages, but that is a corpus observation, not a schema
guarantee, and a package that did translate one would have that
translation's row sit in `translation` unread by anything, exactly as
`FunctionType`/`SpaceUsage` did before this slice. Second, the
`function_type`/`function_point`/`space_usage` tables inherit
`datapoint_type`'s uncounted-collision gap outright — see §86's residue,
extended 2026-09-14 to name them — a second package's `knx_master.xml`
drops its restated rows with nothing recording that it happened. Third,
`apps/knx-web` still sends no bare primary-language tag (§37's own open
item, unchanged by this slice): the backend-side locale-prefix matching
these new readers reuse has had a caller-reachable surface since D10
slice 1, and still has none from the frontend.

**Lifted when.** Ingestion: lifted 2026-09-12 (T32, branch
`t32-shared-translations`). The `Hardware`-scope half of the reading
residue: lifted 2026-09-13 (T16, branch `t16-device-product`). The
`Master`-scope residue for `datapoint_type`, translation-count
reporting, and backend locale-prefix matching: lifted 2026-09-13 (D10
slice 1, branch `d10-master-translations`). `FunctionType`/
`FunctionPoint`/`SpaceUsage`: lifted 2026-09-14 (T13, branch
`d10-language-data`). This section's own residue (other `MasterData`
sections, collision counting, frontend locale tags) stays open; see
**D10** in [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) and §37's own
"still open" list. Not scheduled.

## 65. `--version` names a commit, never a working tree

`knx --version` and `knx-server --version` print
`<name> 0.1.0-alpha.1+g<short-sha>` ([ADR-0018 §2](adr/0018-program-versions-and-file-headers.md)).
The sha is the `HEAD` commit at the time cargo last ran that binary's
`build.rs`. It says nothing about whether the tree was clean: uncommitted
edits, staged or not, are invisible, and there is no `.dirty` marker,
because cargo re-runs a build script for files it has been told to
watch and has no notion of "anything changed anywhere". A build from a
modified tree therefore reports the last commit's sha with a straight
face. Cost: someone bisecting from a `--version` string is looking at
that commit *plus whatever was uncommitted at build time*. Lifted if: a
build ever runs `git status --porcelain` and accepts that the marker can
then be stale in the other direction (a `dirty` stamp that outlives the
edits, until the script next happens to re-run) — a trade this project
has not taken.

Two ways the sha could have been *wrong* rather than merely incomplete
were found in review (2026-09-12) and are handled. After `git pack-refs`
— routine under `git gc --auto` — the loose branch file disappears, and a
watch on it alone went stale: every later commit was invisible to
`--version` until `HEAD` itself moved. The `HEAD` reflog is watched too
now; it is appended on every commit, checkout and reset, packed or not.
And a source tree unpacked inside an unrelated repository was stamped
with *that* repository's commit; `git rev-parse --show-toplevel` must now
equal the workspace root, canonicalized, or nothing is emitted. Without
git at all, or with `.git` excluded (the Docker build), the metadata is
simply absent — `knx 0.1.0-alpha.1` — unless `KNX_BUILD_SHA` is passed
in. Absence is the intended failure direction; a false number is the one
this section, and the ADR, exist to rule out.

<a id="66-server-composed-prose-and-the-documentation-export-are-not-translated-by-any-ui-language-or-pack--partially-resolved-2026-09-14-t14"></a>
<a id="66-server-composed-prose-and-documentation-are-only-partly-localized--partially-resolved-2026-09-23-t14"></a>
## 66. Server-composed prose is only partly localized

**Resolved, one surface.** `ParameterDiagnostic.message` — the parameter
panel's own diagnostic headline — now follows the `CreationDiagnostic`
pattern this section's own "Lifted when" paragraph named as the way
across this boundary. `ParameterDiagnosticDto`
(`apps/knx-server/src/routes.rs`) gained a `kind` field
(`ParameterDiagnosticKindDto`, 22 variants — 15 of them mirror, one for
one, the 15 cases of `pub enum Diagnostic`
(`crates/knx-productdb/src/dynamic/evaluate.rs:756`); the other 7 —
`ParametersUnreadable` through `MalformedModuleInstanceId` — have no
`Diagnostic` counterpart at all and are constructed directly in
`apps/knx-server/src/domain.rs`, where the read fails before a
`Diagnostic` could even be produced); `diagnostic_kind_and_message` now
returns `(kind, message)` instead of just `message` for the 15 mirrored
cases, and all eight `ParameterDiagnosticDto` construction sites carry
a `kind`. `.message` itself is unchanged text — still English, still
composed server-side — but it is now redundant: `ParameterPanel.tsx`'s
`describeParameterDiagnosticMessage` looks `kind` up in
`PARAMETER_DIAGNOSTIC_MESSAGE_KEYS` (22 entries, one per
`parameters.diagnostic.*` catalogue key in `messages/en.ts`/`de.ts`) and
renders the catalogue's translation, falling back to the raw `.message`
only for a `kind` this build's frontend doesn't recognise — the same
"trust the wire shape, degrade to English for the unknown case"
contract `CreationDiagnostic.kind` already documents. This was cheap
specifically because every one of the 22 messages is a fixed sentence
with no interpolated value of its own (`ParametersUnreadable` through
`UnresolvedTextPlaceholder`, spanning both `domain.rs`'s own
construction sites and its `diagnostic_kind_and_message` match); every
dynamic value — ids, node numbers, counts — already lived in `.detail`,
never in `.message`, so tagging the closed set cost one enum and a
lookup table, not a server-side templating layer. Test:
`apps/knx-web/src/ParameterPanel.test.tsx`, "§66: a diagnostic's message
is translated with the UI language; its detail stays English".

**The boundary rule (apply this to the next string you add).** A
server-composed string crossing into `apps/knx-web` is translatable
work, not a given, and the question to ask before wiring it into the
catalogue is: *does a user read this during normal use of the feature,
and is its content a closed, enumerable set (or safely decomposable
into a translated template plus untranslated data slots, `toast.ts`'s
`{msg}` style)?* If yes, give the Rust side a `kind` tag (or reuse an
existing enum's discriminant) the way `ParameterDiagnostic.message` and
`CreationDiagnostic` (T25 Task 5) both do, and translate the *kind*
client-side — never the composed sentence itself, and never by trying
to parse or pattern-match English prose back into a language. If no —
the text is developer-facing (read for a bug report, not during the
task), or it is arbitrary/unbounded (a panic message, a raw debug
formatter, a validator's free-form detail) — it stays English, on
purpose, and this document is where that decision is recorded so the
next person doesn't have to re-litigate it per string.

There is a third case, and it is not the same as either answer above:
the string passes the "yes" test — user-facing, closed and enumerable —
but the component composing it architecturally cannot reach a
catalogue at all (no injected dependency, no language parameter on its
entry point, a layering rule that forbids the dependency a catalogue
would ride in on). That string also stays English, but for a reason
that has nothing to do with being arbitrary or unbounded, and filing it
under that clause teaches the wrong lesson to whoever reads this
document next. It stays English *until someone gives the component a
catalogue to inject* — which is itself the follow-up task, not a
reason to leave the boundary undecided.

**Ruled out under the first two buckets — developer-facing, or
arbitrary/unbounded — and why (still open, not scheduled):**

- `ParameterDiagnostic.detail` (`apps/knx-web/src/api.ts`) — kept
  English by design, not merely untranslated. It is `format!("{:?}",
  diagnostic)` (`domain.rs`), a Rust `Debug` dump behind
  `ParameterPanel.tsx`'s "Copy details" button, meant to be pasted into
  a bug report or read by whoever wrote `domain.rs`, not by an end user
  during normal use — arbitrary, not a closed set, and nobody reads it
  in German. `api.ts`'s doc comment on the field says so; the test
  above asserts `.detail`'s content (`"NoBranchMatched { choose_node:
  4821 }"`) reaches the clipboard byte for byte through the "Copy
  details" button's handler, untranslated — not merely that it is
  absent from the banner's rendered text, which it always would be
  regardless of translation, since `.detail` is never printed there in
  the first place (fix round 1, Q3).
- `LogPanel.tsx`'s `entry.message`/`entry.location`/`entry.detail` — the
  session log is diagnostic output for whoever is debugging a bus
  session, the textbook case of "log text nobody reads in German" this
  task's brief named explicitly. Only the row's *severity* label
  (`SEVERITY_LABEL_KEYS`) is UI chrome and was already translated before
  this task. **Fix round 2 (B4):** this is §67's shape again — German
  chrome over an English message, with nothing telling the reader it is
  deliberate — so the panel now says so where the reader actually is,
  not just here: a translated `logPanel.entryTextIsEnglish` line
  (`"Message, location and detail are the server's own text, in
  English."`) renders under the header whenever there is at least one
  entry. Disclosure only; `entry.message`/`location`/`detail` are
  exactly as untranslated as before.
- API error strings surfaced in toasts (`api.errorMessage`,
  `toast.ts`'s `humorizeError`, `BusMonitorPanel.tsx`'s
  `stopSummary.warning` fed by `BusSessionSummary::drain_panic` in
  `apps/knx-server/src/bus_routes.rs`) — these are not a closed,
  enumerable set the way `ParameterDiagnostic.message` and
  `LanguagePackRejectionReason` (§67) are; they're every `Result::Err`
  path across the whole Rust backend, funnelled through one generic
  `Error.message` unwrap at the HTTP boundary. Closing this would mean
  auditing and tagging every fallible call site server-wide — a
  cross-cutting rewrite well beyond one task, explicitly out of scope
  per this task's brief ("do not perform unrelated refactors"). The
  wrapper sentence around the message is already translated
  (`toast.error.*`); only the substituted server text is not. **Fix
  round 2 (B4):** every error toast now says so — a translated
  `toast.error.messageIsEnglish` line (`"This message is the server's
  own text, in English."`) renders under the wrapper sentence, `{msg}`
  and all. Disclosure only; the substituted text itself is unchanged.

**The documentation-export case changed on 2026-09-23, but remains open.**
`ReportOptions::language` now injects an English/German choice and localizes
the document title, primary navigation, selected headings and mandatory
limits; `knx-app` requests product strings in the same locale. Detailed table
labels, debug/enum values and several diagnostic sentences remain English.
The report has its own small built-in language choice, not access to the web
message catalogue or imported packs, so a pack cannot add a third report
language. Completing this requires extending the renderer's injected prose
catalogue while retaining its enforced independence from `knx-productdb` and
frontend code. See
[§48](#48-project-documentation-export-renders-in-one-language-only).

**Cause (original, still true except for `ParameterDiagnostic.message` and
the report's built-in primary EN/DE chrome).** The remaining strings are
composed by Rust crates and cross the HTTP boundary as opaque text, not as a
message key plus parameters. A frontend catalogue can only translate a key it
was given; a language pack can only override a key this build already defines.
Neither mechanism has anywhere to attach to a string it never sees structured.

**Impact.** A user running any UI language — German, an imported pack,
or an invented one — now sees a translated parameter-diagnostic
headline, but still sees English `.detail` text (by design), English log
entries (by design), and English error-toast bodies inside translated
wrappers. A report explicitly requested in German has German primary chrome
and product strings but still contains the detailed English text above; an
imported pack cannot add a report language. The gap is smaller, not closed.

**Lifted when.** Partially done, 2026-09-14 (T14): the
`ParameterDiagnostic.message` surface closed, using the exact mechanism
this section previously said would be needed. On 2026-09-23 the report gained
an injected EN/DE choice and localized primary chrome, but not a complete
catalogue or language-pack integration. The remaining surfaces above stay
open for the stated reasons; none is scheduled.

## 68. Repeated module instantiation is refused, not supported

**Limitation.** When two or more `ModuleInstance` elements in a project
share one `RefId` — a genuinely repeated module, i.e. its `MI-` component
would need to exceed `1` to tell the copies apart — that module's fields
stay read-only, with a diagnostic ("Two or more imported module instances
share this module; its fields are read-only.") naming the shared `RefId`
and every claiming `instance_ets_id`.

**Not the same case, and not refused:** a **lone** `ModuleInstance` whose
own `@Id` happens to end `MI-2` or higher is accepted and writable — D39
rule 2 asks only "does exactly one `ModuleInstance` match this module?",
not "does its `MI-` digit equal `1`?". Refusing a project's own `MI-2`
would mean guessing that it must be wrong, which is precisely what D38
exists to avoid (see design D40, corrected in this revision — it used to
say the opposite).

**Cause.** `ValueMap`'s scoped key is `(module_id, ref_id)`, with no `MI-`
dimension, because a program-side `Module` node carries no repeat-index
concept at all — the evaluator has nothing to key sibling channels by.
Two `ModuleInstance`s instantiating one `Module` therefore cannot be told
apart on the read side, and this slice does not pretend otherwise
(design D40).

**Impact.** 0/32 `ModuleInstance` elements in the corpus exercise this —
nothing observed regresses. A device that genuinely has repeated
instantiation falls back entirely to the pre-T18-slice-4 behaviour for
that module: displayed, not writable, evaluated against the program
default in every copy.

**Also recorded here, cosmetic and deliberately left as-is:** when the
two-or-more claiming instances have *different* `RefId`s, the diagnostic's
detail string says `"RefId '{X}' matches module '{module_id}' …"` —
singular, naming only the first (`apps/knx-server/src/domain.rs:2308-2334`,
`MiAuthority::Ambiguous`). It already names every claiming
`instance_ets_id` in the same sentence, which is the information a user
needs; making the `RefId` clause itself plural would touch the
`MiAuthority::Ambiguous` variant's shape, its one construction site, and
the format string — more than a one-line fix, so left for a future pass
rather than done here.

**Task 12 (2026-09-14), and why it does not lift this.** Argument
interpretation now tells two *program-side* instantiations of one
`ModuleDef` apart — `MOD-A` and `MOD-B` produce different labels because
they bind different values. This limitation is about the *project* side:
two `ModuleInstance` elements claiming one `Module`. `ValueMap`'s scoped
key is still `(module_id, ref_id)` with no `MI-` dimension, because the
thing that is missing is a repeat index in the project file's authority,
not a way to distinguish `Module` nodes. Unchanged, in full.

**Lifted when.** RESEARCH.md's sharpest unknown #1 (what
`ModuleInstance/@RepeatIndex`'s embedded `MI-<k>` component means, and
whether/how it legitimately exceeds `1`,
[docs/RESEARCH.md §4.4](RESEARCH.md#44-modulemoduledef-expansion-semantics--r4-spike-session-4-2026-09-11))
would have to be settled — by a normative worked example or a hand-built
multi-repeat fixture — before a scoped key that tells repeated copies
apart could be designed without inventing one.

## 69. A `Module` with no `@Id` cannot be matched to a project instance

**Limitation.** `Module/@Id` is an optional XML attribute
(`ModuleScope::module_id: Option<String>`). When a program instantiates a
`Module` with no `@Id`, that instantiation's stored per-channel values are
unreachable — there is nothing to decompose an `MI-` authority against —
so the section evaluates against the program default and stays read-only,
with a diagnostic: "A module instance has no identifier and cannot be
matched to stored values." (`Diagnostic::ModuleWithoutId`,
`apps/knx-server/src/domain.rs:1827-1829`).

**Cause.** Design D37: `Module/@Id` is present on 102/102 `<Module>`
elements across the corpus's seven module-bearing program files (E2), but
nothing in the Standard extraction guarantees that for a package not yet
seen. The diagnostic is emitted per instantiation, unconditionally — a
`Module` this slice cannot name is worth reporting even when no stored
value would have applied to it.

**Impact.** Unexercised in the corpus today (0/102). If it ever fires,
that `Module`'s channel behaves exactly as every module-scoped channel did
before T18 slice 4: displayed where a value happens to already resolve,
never editable.

**Task 12 (2026-09-14): unaffected, and marginally better reported.** An
`@Id`-less `Module` still cannot be matched to a project instance — an
argument binding names an `Argument`, not the `Module` carrying it, so it
supplies no identity. It does now produce a *different-looking* section:
its labels are substituted like any other instantiation's, so a nameless
module is at least distinguishable on screen from its siblings even while
remaining unmatchable and read-only. Nothing about the matching rule
changed.

**Lifted when.** Never by invention — a synthesised id would be a
fabricated identifier that looks like project data and matches nothing
real (D37's own reasoning). Only the manufacturer's own application
program, carrying its own `@Id`, lifts this.

## 70. Writing a declared-but-not-currently-shown parameter is now refused

**Limitation.** Before T18 slice 4, a `POST` naming a parameter's declared
`ets_id` was accepted even when that field was not currently active/shown
(old D24 check 2: "this check does not require the parameter to be
currently active"). This slice narrows that: a write is now accepted only
if the named id is the `ets_id` of an unscoped field the just-assembled
panel currently shows, or the `write_ets_id` of an editable module-scoped
field in that same panel (design D43). Naming a bare declared id that the
program has but the current panel does not currently show is now rejected
with `"is declared by this program but not currently active"`
(`apps/knx-server/src/domain.rs:2578`).

**Cause.** D43 replaces the old two-check validation with one rule: "the
panel is the single authority on what is writable." Accepting a bare
declared id unconditionally would mean guessing whether that id names a
top-level field or a module-scoped one — `knx-productdb`'s `parameter_ref`
table carries no `module_def_id` column
(`crates/knx-productdb/src/migration.rs:201-210`) to answer that question
without inference, and inferring scope is exactly what this slice's own
constraint forbids (D38's rationale, applied here to the read side of the
same question).

**Impact.** Narrow: a client that wrote to a hidden-but-declared top-level
field (one sitting behind a currently-unmatched `choose` branch) before
this slice can no longer do so directly — it must wait until that field is
shown, i.e. until the `choose` that gates it resolves to a matching
branch. No corpus-observed workflow depends on writing a hidden field
sight-unseen; `Access` itself has no attested write-gating correlation
either ([§3](#3-device-parameters-are-preserved-but-not-interpreted)).

**Task 12 (2026-09-14): unaffected.** The task added a
`module_def_argument` table, not a scope marker on `parameter_ref`. A
declared parameter id still carries no scope of its own, so the panel
remains the single authority on what is writable, exactly as D43 has it.

**Lifted when.** Would need `parameter_ref` (or a sibling table) to carry
a `module_def_id` or equivalent scope marker, so the server could resolve
a bare id's scope without first evaluating the tree it belongs to. Not
scheduled.

## 71. A project imported before store schema 6 has no module-instance ids to write with

**Limitation.** `migrate_v5_to_v6` (design D38) cannot invent a
`ModuleInstance`'s own `@Id` for rows that predate the migration; every
such row's `instance_ets_id` becomes `''` ([DATA_MODEL.md §11](DATA_MODEL.md#11-versioning-and-migration)).
A user opening a `.knxdb` last saved before this slice therefore sees
every module-scoped section read-only, with the same "no imported
`ModuleInstance` matches" family of diagnostics a genuinely missing
authority produces — nothing in the diagnostic text distinguishes "this
project predates schema 6" from "this project has no matching instance at
all."

**Cause.** The id can only come from the project file's own
`<ModuleInstance Id="…">` attribute (D38). Before this slice, the import
path read that attribute (`installation_v21.rs:764-769`) but discarded it
after using it only as a local wiring key (`map.rs:1038-1049`, design E5)
— so a project imported under an earlier binary never had the id to carry
into the store, and the migration has nothing to backfill it from.

**Impact.** User-visible, and easy to mistake for a bug: a user who has
not re-imported since upgrading sees read-only channels with no reason
that names "schema version" or "re-import" specifically — only the
generic no-authority diagnostic.

**Task 12 (2026-09-14): unaffected, and a different database.** This
limitation is about the *project* store (`.knxdb`, schema 6); task 12
migrated the *product* database (`products.sqlite`, v10 -> v11), which is
a separate file with a separate version chain. The product-db migration
re-derives everything it needs from stored `source_file` bytes and so
needs no re-install; this one still needs a re-import, for the reason
below.

**Lifted when.** Automatically, the moment the project's source
`.knxproj` is re-imported (not merely re-opened) — re-import re-parses
`ModuleInstance/@Id` from the file and repopulates the column for every
row.

## 72. Line-scan (T17): an unthrottled scan is a live-bus cost, not a theoretical one — shipped 2026-09-13, still true

**Limitation.** An unthrottled `knx bus scan` of a full line takes tens
of minutes and holds a tunnelling connection open, connecting and
disconnecting, for the whole run. This is not a bug to fix; it is the
documented, measured cost of the KNX Standard's own
`NM_IndividualAddress_Check` procedure, and it is why `bus scan` ships
with pacing (`--pause-ms`, default 100 ms) and an exclusion list
(`--exclude`) rather than a single "scan everything, fast" button.

**Cause.** Two independent, additive costs, both **[D]**/**[V]**, not
implementation slack: (1) each vacant address costs one Transport Layer
connection timeout, fixed by the Standard at 6 s
(`03_03_04 Transport Layer v01.02.03 AS`, clause 4, page 16 of 38 — see
[RESEARCH.md §8.5, Finding 1](RESEARCH.md#85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13)
for the correction of an earlier, wrong attribution of this cost to a
client library's own policy choice); and (2) a real line is mostly
vacant addresses, not mostly occupied ones, so the expensive case
dominates the total, not the cheap one.

**Impact.** Two measurements exist, from two different points in time and
two different implementations, and this entry keeps both distinguished
rather than merging them into one number:

* **2026-09-12, pre-implementation, `xknx`** (full line, one
  installation, one gateway, 200 ms pause, zero probe errors): 254
  addresses probed, 35 occupied, 219 vacant. Occupied probes 13.6-6016.5
  ms (median 121.1 ms); vacant probes 6275.9-6323.8 ms (median 6279.9
  ms). Summed: **1 385.75 s ≈ 23.1 minutes** for the whole line.
* **2026-09-13, this repository's own shipped binary**, on real
  hardware: a five-address run of consecutive vacant addresses cost
  6006 ms each, 30431 ms measured against a 30430 ms prediction — the
  per-address cost from the 2026-09-12 measurement reproduces almost
  exactly under this implementation's own `ProbePolicy::default()`. No
  full-line run has been repeated against this implementation; the
  23.1-minute figure above is the best full-line estimate available and
  is carried forward, not re-derived.

For the duration of a full-line scan, it competes with whatever else
needs that line's bandwidth, including genuinely safety-relevant devices
that share it. This is why `bus scan`'s default timeout is anchored to
the Standard's own 6 s connection timeout rather than shortened for
speed — see [KNOWN_LIMITATIONS.md §75](#75-a-shorter---timeout-ms-is-a-real-option-but-not-the-default--a-slow-but-present-device-can-look-vacant) —
and why an exclusion list is honoured by construction in the domain
layer (`crates/knx-core/src/scan.rs`), not as a UI checkbox someone can
forget to tick.

**Lifted when.** It is not fully liftable — the Standard sets the 6 s
figure, not this implementation — but the exposure shrinks as scans move
from "whole line" to "known range plus known exclusions" in normal use,
and if a future task adds concurrent probing across independent
tunnelling connections (explicitly out of scope for T17, see the brief
for this section) the wall-clock cost, though not the per-address bus
cost, would fall.

## 73. A line scan cannot learn product identity, manufacturer, or serial number

**Limitation.** `knx bus scan` reports an address occupied or vacant and,
when occupied, the responding device's Mask Version — nothing more. It
cannot say which product is installed, who made it, or its serial
number.

**Cause.** `NM_IndividualAddress_Check`'s only application-layer step is
`A_DeviceDescriptor_Read` with `descriptor_type = 0`, which returns DD0,
the Mask Version — **[D]** *"Identification of an implementation, for
operation like download, memory_write … In particular, the Mask Version
is read through a dedicated Application Layer service by the S-Mode
Management Client (ETS) to conclude on the Configuration Profile of the
device and on possible further discovery and configuration steps"*
(`03_01_02 Glossary v01.05.03 AS.md:236`). A Mask Version identifies an
implementation family/coupler-medium class, not a product: **[D]**
`06_02_01 Coupler Model 2.0 v01.01.01 AS` §1.5.2 notes many different
coupler products deliberately share one Mask Version. Product identity,
manufacturer, and serial number need a separate, additional
connection-oriented read after the scan step — e.g. `A_PropertyValue_Read`
on the Device Object (`object_index = 0`), `PID_SERIAL_NUMBER` (PID 11) —
**[D]** `03_05_03 Configuration Procedures v02.01.01 AS.md:4797` and
`03_06_03 EMI_IMI v01.04.02 AS.md:5074`. That step is not part of
`NM_IndividualAddress_Check` and was explicitly out of scope for T17; it
is T16's territory (device-catalog/product identity work).

**Impact.** A scan's occupied/vacant list, and its Mask Version per
occupied address, cannot by itself populate a topology view with product
identity or resolve which manufacturer's device answered. A user
reconciling a scan against a project still needs a second signal, or a
manual lookup, to identify an undocumented device.

**Lifted when.** T16 or a successor adds a `A_PropertyValue_Read` follow-up
step per occupied address; whether every Mask Version a scan might
encounter even supports Property services, versus only Memory-based
access as some older masks do, is unverified and would need checking
before that step could be relied on unconditionally.

## 74. A line scan cannot distinguish a busy-but-present device from an absent one

**Limitation.** If a device's Layer 2 acknowledge for the scan's
`T_Connect` comes back negative — which includes a Standard-compliant
BUSY response — the scan treats it exactly like no acknowledge arriving
at all: both exhaust `vacant_confirmations` and are reported `Vacant`. A
busy-but-present device and an address nobody occupies produce the same
report.

**Cause.** `deadline_verdict` (`crates/knx-net/src/scan.rs`) only resolves
a *positive* connect confirm to a distinct outcome, `OccupiedSilent` — a
device that acknowledged at Layer 2 but never produced an Application
Layer answer (test `a_positive_l2_confirm_with_no_application_answer_
is_occupied_but_silent`, `crates/knx-net/src/scan.rs`). A *negative*
connect confirm, or no confirm at all, both fall through unresolved and,
after the last confirmation pass, become `Vacant` (test
`a_negative_l2_confirm_is_vacant_like_total_silence`). **[D]**
`03_02_02 Communication Medium TP1 v01.03.03 AS` §2.4.2: a device *may*
send BUSY if it expects to be able to process frames again starting
100 ms after the frame that triggered it, and *shall not* send BUSY
otherwise — so a negative confirm can be a real, Standard-compliant
answer from a present device, and this scan has no way to tell that
answer apart from nothing arriving. This spike's own 2026-09-13
five-address live run observed neither a negative confirm nor an
`OccupiedSilent` result among its five vacant addresses (a one-sample
fact about that run, not evidence either case is rare or cannot occur;
see
[RESEARCH.md §8.5 Finding 4](RESEARCH.md#85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13)).

`OccupiedSilent`'s own citation rests on a hedge in the Standard's own
text, not a certainty: **[D]** `03_06_03 EMI_IMI v01.04.02 AS` §4.1.5.3.4
says the confirmation "is **normally** generated after receiving this
immediate acknowledge" — normally, not always, is the word the whole
distinction between `Occupied` and `OccupiedSilent` rests on.

**Impact.** This is a documented limit of the mechanism itself, not a gap
in this implementation's reading of it, and no scanner built on
`NM_IndividualAddress_Check` alone can resolve it. A user reading a scan
report needs to know that `Vacant` means "no positive evidence of
occupancy", never "certainly no device here" — a legitimately busy device
is one concrete way that gap gets filled.

**Lifted when.** Never, by this mechanism alone — §76's negative-confirm
fast path is a separate question (scan speed, not disambiguation) and
would not resolve this either. A second, independent signal (a different
management procedure, or a manual check) would be needed to fully
disambiguate a negative confirm from true absence.

## 75. A shorter `--timeout-ms` is a real option, but not the default — a slow-but-present device can look vacant

**Limitation.** `bus scan --timeout-ms` accepts values below the 6000 ms
default, but doing so trades correctness for speed: a device that would
have answered slowly is reported vacant instead.

**Cause.** **[V]** the 2026-09-12 pre-implementation `xknx` full-line
measurement's occupied-probe round trips ranged 13.6-6016.5 ms (35
occupied addresses of 254 probed, median 121.1 ms). A 1000 ms timeout,
plausible-looking because most occupied addresses in that run answered
well under a second, would have reported the slowest observed present
device as vacant. Shortening the timeout does
not distinguish a slow device from an absent one; it only moves the
threshold at which the scan starts misreporting one as the other.

**Impact.** An operator who shortens `--timeout-ms` to speed up a scan on
an installation with any slow-but-present devices will see false
`Vacant` results, silently, with no separate signal to flag them as
suspect.

**Lifted when.** Not by more code — this is a real trade-off inherent to
the mechanism, not a bug. It stays a documented, explicit, opt-in choice
via `--timeout-ms`, and the shipped default stays anchored to the
Standard's own connection timeout for exactly this reason.

## 76. A negative Layer 2 confirm's fast path was deliberately not built; `Indeterminate` does not retry

**Limitation.** Two related shortcuts a faster or more thorough scan
implementation might take were considered and deliberately not taken.
First, a scan does not fast-path on a negative `L_Data.con`
(acknowledgement/confirmation failure) to conclude "vacant" sooner than
waiting out the full connection timeout. Second, when a probe's evidence
is ambiguous because the tunnel's broadcast event channel lagged and
dropped one or more frames during the probe window — possibly including
the very descriptor response, disconnect, or connection confirm that
would have settled the verdict — the scan reports `Indeterminate` for
that address and moves on — it does not retry the probe.

**Cause.** Both are documented, in-code decisions
(`crates/knx-net/src/scan.rs`), not oversights. The negative-confirm fast
path rests on an assumption this spike could not verify against the
corpus: that a negative `L_Data.con` for this specific exchange reliably
means "nobody there" rather than some other transient Layer 2 condition;
building a fast path on an unverified assumption risks quietly turning a
present-but-momentarily-noisy device into a false `Vacant`, which is the
one failure direction this whole feature exists to avoid. `Indeterminate`
not retrying is a matching decision on the evidence-honesty side: a
lagged channel is reported as exactly what it is, an inconclusive read,
rather than silently retried and folded into whatever the retry happens
to produce — retrying would make `Indeterminate` disappear from a report
without actually resolving the ambiguity that produced it.

**Impact.** A scan is measurably slower than a maximally aggressive
implementation would be, and an installation whose channel lags often
will see more `Indeterminate` results than a retry-based scanner would
report as something more decisive-looking (and less trustworthy).

**Lifted when.** The negative-confirm fast path could be added once the
underlying assumption is verified — directly against hardware behaviour
across more than one gateway/device combination, or against corpus text
this spike did not find. Retrying `Indeterminate` is a considered
trade-off, not a gap, and would need a positive reason (a demonstrated,
common cause of transient lag worth papering over) before revisiting it.

## 77. A line scan covers one line at a time; it does not cross couplers

**Limitation.** `bus scan` scans one line, reached through one
tunnelling gateway, per invocation. It does not discover or traverse line
or backbone couplers to scan other lines in the same installation
automatically.

**Cause.** Explicitly out of scope for T17 (see the task brief for this
work): scanning across couplers, or scanning more than one line per
invocation, was never attempted, and no concurrency between probes was
built either — probing stays sequential, one outstanding request per
tunnelling connection, for the same evidence-honesty reasons as
§76.

**Impact.** An installation with more than one line needs one `bus scan`
invocation per line, with the operator supplying each line's own
gateway/area/line addressing by hand; there is no "scan the whole
installation" command.

**Lifted when.** A future task adds coupler-aware, multi-line scanning —
not scheduled as part of T17 or its immediate successors.

## 78. A line scan reports other KNXnet/IP tunnelling endpoints as occupied devices

**Limitation.** `bus scan` excludes exactly one non-bus address: the
tunnelling connection assigned to the scan itself. Any other KNXnet/IP
tunnelling endpoint sharing the same gateway — another client's tunnel,
or an endpoint answering from the gateway's IP side generally — is
reported `Occupied`, indistinguishable from a real twisted-pair device.

**Cause.** `probe_address` short-circuits to `SelfAddress` only when
`addr == transport.assigned_address()` (`crates/knx-net/src/scan.rs:298-300`);
no other exclusion exists. **[V]**
[RESEARCH.md §8.5 Finding 2](RESEARCH.md#85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13)
found a candidate signal in three samples on one gateway: two tunnelling
endpoints answered in 13.6 ms and 14.0 ms, roughly an order of magnitude
faster than the 100-150 ms a real bus device typically needs. That
sub-20 ms heuristic was deliberately not built: three samples on one
gateway with one client implementation is **[A]**, not evidence it
generalises, and a genuinely fast device or a slower IP path on a
different gateway could break it. Building an exclusion on an unverified
timing gap risks the opposite of this feature's purpose: quietly
mislabeling a real device as not-a-device.

**Impact.** A scan report can include phantom "devices" that are actually
other tunnelling clients or the gateway's own IP-side presence, at
whatever individual address the gateway happened to assign them. An
operator reconciling a scan against a project needs to recognise and
manually exclude these; the same measurement that established this saw
three such endpoints among 35 occupied addresses on one gateway.

**Lifted when.** Only after the sub-20 ms heuristic, or a more reliable
signal, is verified against more than one gateway/client combination —
not scheduled as part of T17.

<a id="79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry"></a>
## 79. Discovery needs reachable multicast and permitted unicast replies

`KnxNetIpClient::discover()` sends `SEARCH_REQUEST` to
`224.0.23.12:3671` and awaits a unicast response. Docker's default bridge
does not provide the required multicast path; the documented Linux
deployment uses host networking when discovery is needed. A manually entered
numeric IPv4 endpoint remains the fallback and can work over unicast even
when multicast discovery cannot. Host routing, firewall and gateway support
still matter with host networking.

**Host cause resolved (2026-09-30, RESEARCH §20.1).** The host `ufw` had
dropped the gateway's unicast UDP reply from *source* port 3671. The user
allowed that source port from the LAN; `knx bus discover` and
`POST /api/bus/discover` then both found the gateway. A rule for the
multicast destination alone was insufficient. No protocol or AppImage
workaround was needed; an unpackaged server and the AppImage had sent the
same request. The CLI and Web empty-result hints still name host firewall
and container networking as plausible causes on other installations.

**Remaining verification boundary.** A native WebKitGTK click on Search
was not observed. The successful CLI/HTTP test on this host is not proof of
all container networks or multi-homed LANs, and a missing gateway remains
a network diagnostic, not proof that the KNX device is absent.

**Offline coverage, 2026-10-01 (U13).** Two non-skipping UDP loopback tests
exercise the same private discovery exchange as production: advertised HPAI
return address, service/body filtering, duplicate endpoints, multiple reply
sources, gateway projection and no-reply deadline. See RESEARCH §20.1 and
`client::tests::discovery_loopback_roundtrip_uses_advertised_hpai_and_filters_datagrams`.
They bind/send only on `127.0.0.1`; they do not verify multicast routing,
host-firewall policy, AppImage/WebKitGTK Search or physical gateway behavior.
ISSUE-12 can close its offline-test acceptance without lifting this network
verification boundary or claiming a new protocol fix.

## 82. The diagnostics companion's stale lock sees one browser profile's own windows, and nothing else

**Status (2026-10-02 scoped delivery).** The browser-only proof described
below is superseded by the delivered authoritative polling
contract. The server compares the active session's real `GroupAddressContext`
with the current project's interpretation (style, names and resolved DPTs).
`contextStatus`, nullable `projectOpen` and `serverIncarnation` are returned by
the same monitor poll. Busy/poisoned/unavailable evidence is not freshness;
legacy/malformed responses or poll failure lock the new UI's compose form.
Pause performs context-only reads without rows or cursor advancement. Delayed
replies cannot erase a newer invalidation or replacement session. Browser
records cannot establish a verified state. Backend/UI regressions and bounded
mocked browser evidence and twelve complete candidate gates pass; integrated
acceptance repeated and source `8ceacf49` published with exact remote/tree
readback. Scoped comparison is delivered, not the remaining boundaries. See
[owner evidence](UI_ALPHA_READINESS.md).

**Remaining boundary.** This is a point-in-time interpretation comparison, not
a complete project identity/version, collaboration/push channel or atomic
write-context token. Existing captured rows keep their old decoded values; a
later edit can occur before another poll or send. Parameter/device changes are
checked only insofar as they change the interpreter's actual DPT/name/style
snapshot. Real native/live-bus verification is not inferred from fixtures.
An older server without this evidence remains readable but cannot enable Send
in the new monitor UI. No hardware permission or source-wide alpha waiver.

**Historical explanation (before this follow-up).** The following describes
the former local-storage-only mechanism and its known cases, not the new
implementation's source of freshness evidence.

**Limitation.** The second-window diagnostics companion (T-UI-06) locks
itself when the project changes under a running bus session. That lock is
decided entirely from two `localStorage` records written by the windows of
one browser profile (`apps/knx-web/src/busContext.ts`). It therefore
detects only edits made in a window that shares that storage. Four cases
it cannot see, and what each one costs:

1. **Another client edits the project.** A second browser, a private
   window, another machine, or `curl` against the same server changes a
   group address's name or DPT without changing its style. No record in
   this profile's `localStorage` moves, so the companion can keep reporting
   `synced` while the running session's `GroupAddressContext`
   (`apps/knx-server/src/bus.rs`) describes an older project. T13 is the
   explicit exception: a successful style change, including Undo/Redo that
   changes the style, refreshes the complete server-side context. That does
   not introduce general cross-client project synchronization.
2. **The project record outlives the server.** `localStorage` survives a
   server restart; the server's in-memory project does not. The companion
   can therefore believe a project is open (`projectContextKnown()`) when
   the server holds none. The only cost is a suppressed hint on the
   compose form: it stops explaining that no DPT will resolve
   automatically. The session half of this self-corrects — the first poll
   after the restart gets a `404` and the panel detaches, clears the
   session record and says the session ended elsewhere.
3. **A project opened in a window that later reloads.** The project record
   is published from the live tree in `App.tsx`; a reloaded window has no
   tree until the user opens a project again, so it publishes nothing and
   the previous record stands until it does. A stale-but-identical
   fingerprint is the harmless case; a project *closed* and a different
   one opened elsewhere is case 1 again.
4. **Two sessions in one profile, one of them unrecorded.** If a session
   is started by something that does not write the record — another
   client, or a direct `POST /api/bus/monitor/start` — the companion
   reports `unverified` rather than `synced` or `stale`: it says it cannot
   confirm the decoded values, and leaves sending enabled. That is
   deliberate (nothing observed says the snapshot is wrong), but it is
   weaker than a real answer.

**Cause.** There is no project-change push channel and the companion does
not poll current project state. [§63 point 3](#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all)
documents the missing cross-client synchronization. Since T12,
`GET /api/project` does return the current authoritative tree without a
mutation; it is used for explicit recovery and save refresh, not continuous
companion synchronization. The older claim that no such GET exists is
obsolete. The companion still relies on its sibling window's local record.

**Impact.** The lock is a guard against the common case — one user, one
browser, editing in one window while watching in another — not a
guarantee. In the multi-client situations of §63 it is silent, and a
silent lock looks the same as a verified-fresh one. The consequence is the
one the feature exists to prevent: a decoded column, and a write's
resolved DPT, describing a project the server has since changed. Writes
land on real hardware and project Undo cannot reverse them (`busCompose.liveAction`).

**Lifted when.** A verified push or polling contract compares current
project state with the actual session context across clients and restarts.
The existing read-only project GET is a prerequisite, not proof that this
contract exists; no continuous companion refresh is implemented by T13.

**Platform note.** Both platforms were exercised on 2026-09-13: the
companion route renders in headless Chromium against the Vite dev server,
and the Tauri desktop shell opens it as a real second native window,
focuses rather than duplicates it on a second invocation, and returns
focus to the main window. What was *not* exercised anywhere is a live bus
session — no KNX hardware was touched, so the lock's behaviour is proven
by tests (`busContext.test.ts`, `BusMonitorPanel.test.tsx`), not by a
running gateway.

**What the fingerprint cannot distinguish, even for edits it does see.**
The four cases above are all "the lock never hears about the edit". These
three are the other axis: the edit happens in this very window, and the
fingerprint still does not move.

5. **A parameter edit's fingerprint never moves — by design, not by gap.**
   `publishProjectContext` runs from an effect on `App.tsx`'s `tree` state,
   so it fires only when something hands the client a fresh `ProjectTree`.
   Until T3 (2026-09-13), `api.setParameterValue` never did: it answers
   with a `ParameterPanelDto`, and `ParameterPanel` was mounted with no
   channel back to `tree` at all, so the effect never fired and nothing
   republished — a real publish hole, not just a fingerprint quirk.
   `DeviceWorkspace` (`Inspector.tsx`) now closes that channel: once a
   field commits, it overlays `can_undo: true, can_redo: false` onto the
   `tree` it already holds — exact, not a guess, since
   `CommandStack::do_command` always pushes onto `undo` and clears `redo`
   — and hands that to `onApplied`, so `setTree` runs and the effect fires
   on every parameter edit, same as any other command.
   What still does not move, and never will without a further change, is
   the *fingerprint value itself*: `fingerprintProjectContext`
   (`busContext.ts`) deliberately excludes parameters, so the republish
   above writes the same fingerprint under a fresh `at` — indistinguishable
   from renaming a device, which has always behaved exactly this way. This
   is harmless **only** because no parameter value reaches a decode today:
   `Command::SetParameterValue` writes `installation.parameters` and
   nothing else, while `resolve_group_address_dpt` reads com-object links
   and resolved DPTs and nothing else, and `GroupAddressNode.dpts` — the
   third fingerprint input — is produced by the same `group_address_dpt_from`
   rule over the same com objects. The two sets do not intersect. The day a
   parameter can influence a com object's DPT, links or activity, this turns
   into a silent false `synced`; `resolve_group_address_dpt`'s doc comment
   carries that warning at the place that would have to change. **[V]**
6. **The digest is 32 bits.** `fnv1a` in `busContext.ts` returns a 32-bit
   FNV-1a value, so two genuinely different projects collide by accident with
   probability about 2^-32 per comparison, and `synced` means "almost
   certainly unchanged", never "provably unchanged" **[D]**. FNV-1a is also
   not collision-resistant, so a *deliberately* crafted project could be made
   to collide **[D]**. Neither is defended against: the lock is a
   decoding-staleness hint, not a security boundary, and the cost of a miss
   is a mislabelled telegram rather than a bad write.
7. **The field separators are non-printing, and not impossible in a name.**
   The pre-hash string separates the three per-address fields with U+0001 and
   successive addresses with U+0002 **[V]**. That is what stops the obvious
   ambiguity — address `1/1/1` named `0Foo` against address `1/1/10` named
   `Foo`, which without a separator flatten to the same bytes; both that pair
   and the record-boundary equivalent are pinned in `busContext.test.ts`.
   What survives is a group address *name* that itself contains U+0001 or
   U+0002. No supported import can produce one: `.knxproj` is XML, and XML 1.0
   section 2.2's `Char` production admits no C0 control except tab, LF and
   CR **[D]**. Nothing else in the product writes such a name today, and no
   keyboard types one **[A]**. It is recorded rather than encoded away
   because a length-prefixed alternative would invalidate every stored
   fingerprint — every live session would read `unverified` once — to close
   a case nothing can currently reach.

   *Historical note, because it cost two reviews.* Items 5 and 6 were found
   by review; a third finding from the same round — "the fingerprint
   concatenates without a separator, so an ordinary rename produces a
   constructible false `synced`" — was **wrong**. The separators were
   already there and had been since the feature landed, but they were
   written as literal U+0001/U+0002 bytes, which no terminal and no diff
   renders, so two successive readers saw a bare concatenation. They are now
   written as escape sequences instead: same bytes, same fingerprints,
   visible to the next reader. The check that settles it is a search of
   `apps/knx-web/src/busContext.ts` for literal C0 bytes, which should find
   none. **[V]**

## 85. A `.signature` package member is stored with role `Signature`, never verified

**Limitation.** `install_package` (`crates/knx-productdb/src/package.rs`)
recognises any ZIP member whose path ends in `.signature`, records it in
`package_member` with `role = 'Signature'`, and stores its bytes verbatim
in `source_file` — the same treatment `notes.txt` gets under
`role = 'Unrecognized'`. No code path anywhere in this crate, in
`apps/knx-server` or in `apps/knx-web` reads a `'Signature'`-role member
back out to check it against a key, a hash, or anything else **[V]**
(`grep -rn '"Signature"' crates/knx-productdb apps` finds exactly one
writer — `package.rs` — and two readers that only forward the string for
display: `apps/knx-server/src/routes.rs`'s `CatalogInstallMemberDto`,
which since the display fix below qualifies the text rather than passing it
through verbatim, and `apps/knx-web/src/api.ts`'s matching TypeScript type,
which still passes it through untouched). Neither reader opens the file. A row that said `Signature` looked, to anyone reading the
install report, like something was signed and checked. Nothing was — see
"Lifted when" below for the display fix task 05 (T05) shipped for that.

Every `.knxprod` file in the local corpus (`OriginalData/ProductDatabases/`,
copied to a scratch directory for inspection, never modified in place)
carries exactly one such member, and every one observed is 175 bytes: a
UTF-8 byte-order mark followed by about 172 base64 characters with no line
terminator — decoding to roughly 129 raw bytes, the size of a single
RSA-1024 signature **[V]** (`file` and a byte count against the extracted
member). That last interpretation — that it *is* an RSA-1024 signature —
remains this report's own inference from the byte count, not a confirmed
algorithm **[A]**; T05 found nothing that raises or lowers that confidence
and did not re-derive it.

**Cause.** Verifying the member requires two things this project does not
have: the `.signature` file's own format/algorithm/canonicalization, and
the manufacturer's public key to check it against. Neither exists in any
corpus this project can reach, checked two ways:

- The accessible KNX Standard corpus
  (`/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0/`)
  was searched by direct text grep for `.signature`, `knxprod`, and
  `signature` generally. It documents a *different* concept under the same
  word: a "registration signature" is a value ETS/the Manufacturer Tool
  computes over registration-relevant XML data so that a later change to
  that data can be detected on an XML→DB→XML round trip — Project
  Schema23 §1.1.3.18/.19 **[D]** and the Certification Manual's
  import-checks section, which warns that changing registration-relevant
  data invalidates "the signature in the registration data" **[D]**
  (`05 KNX Certification of Products - Procedure v01.07.09 AS.md:1542`).
  That is a content-integrity checksum stored as an XML attribute
  (`hardware.rs`'s own `RegistrationSignature`, discussed below), not a
  detached cryptographic signature file.
- T05 additionally queried both of the project's queryable KNX spec
  knowledge bases (SQLite, curated facts with per-fact evidence, distinct
  from the raw-text grep above) via
  `knx-spec-kb/scripts/05_knowledge_base_v1.py --query`, against
  `knx_spec_kb_programming.sqlite` (27 programming-scoped PDFs plus
  figures) and `knx_spec_kb_full179_clean.sqlite` (177 PDFs, text only)
  **[V]**. Search terms: `digital signature`, `package signature`,
  `manufacturer key`, `public key`, `certificate`, `code signing`,
  `knxprod`, `signing key`, `key distribution`, `product database
  signature`, `.signature`, `RSA`, `detached signature`, `signature
  file`, `registration signature`, `manufacturer signing`, `product
  package integrity`. The programming base returned zero hits for every
  term except `knxprod` (an unrelated MT3→MT4 conversion note) and
  `.signature`/`signature` (an unrelated AES-CBC-MAC mode name, KNXnet/IP
  Secure). The full base's `public key`/`certificate`/`manufacturer key`
  hits are all KNX IoT Secure / KNXnet-IP Secure material — SPAKE2+
  session keys, device X.509 certificates (`LDevID`/`IDevID`) — a
  different security domain (device authentication on the bus) from
  signing a manufacturer's product package file. `registration
  signature` and `XML signature` returned zero hits in the full base even
  though the concept exists in Project Schema23 and the Certification
  Manual **[V]**: those documents' `.signature`/`registration` facts were
  not extracted into that base's curated fact table, which is a gap in
  the knowledge base's extraction, not evidence the Standard is silent —
  the raw-text grep above is what actually found that content. Recorded
  here so neither search is repeated expecting a different answer.

Nothing in either search names a `.signature` package member, its format,
its algorithm, or a manufacturer key distribution mechanism. Building real
verification without that specification — or the manufacturer's public
key, which this project does not have either way — would be guessing at a
proprietary scheme, which is out of scope and worse than doing nothing.

`hardware.rs`'s own `RegistrationSignature` attribute (stored into
`hardware2program.registration_signature`, never read back by any query,
DTO, or UI in this codebase **[V]**, `grep -rn registration_signature
crates apps`) is the same gap in a different member — with one difference
worth naming: unlike the package `.signature`, its defining document
*is* in the accessible corpus (Project Schema23, cited above), so what
verifying it would take is at least namable — parsing the
registration-relevant XML subset the Schema defines, recomputing the
checksum by whatever algorithm ETS/the Manufacturer Tool uses (not
specified in the excerpt found), and comparing. That algorithm was not
located, so this is also presently a dead end, not a task with a known
shape **[D]**.

**Impact.** A `'Signature'`-role member is cosmetic. Installing a package
with a corrupted, empty, or entirely fabricated `.signature` member
succeeds identically to installing one with a genuine one — pinned by
`signature_members_are_stored_verbatim_and_never_verified`
(`crates/knx-productdb/tests/standalone_packages.rs`). Nothing downstream
currently treats the role as a trust signal, so today's blast radius is a
misleading label rather than a bypassed check — but that is exactly the
kind of guarantee a future feature could be built on by mistake, reading
`role == "Signature"` and concluding a package was authenticated.

**Lifted when.** Verification proper is lifted when the `.signature`
format is obtained from KNX Association documentation this project does
not currently have access to and implemented deliberately against it (a
separate task, not a drive-by addition to ingestion). The display half is
already done, T05, 2026-09-20: `install_package` still writes the bare
`role = 'Signature'` to `package_member` — the stored value is a stable
domain identifier other code and its tests key on, and changing it was
out of scope — but `apps/knx-server/src/routes.rs`'s
`CatalogInstallMemberDto` now renders it as `"Signature (stored, not
verified)"` before it ever reaches JSON, pinned by
`a_signature_members_role_is_qualified_as_unverified_in_the_install_report`
(`apps/knx-server/tests/http_product_install.rs`). `apps/knx-web`'s
`CatalogBrowser` install report additionally names the count of such
members in its own sentence, in both UI languages it has
(`catalog.installReport.unverifiedSignature`, English and German), so a
person reading the one place this report is actually shown cannot come
away thinking a signature was checked.

## 86. Duplicate identifiers inside one file — recorded for normalized product identifiers; DPT provenance remains limited

**Original limitation (as filed).** `first_winner` — one copy, in
`crates/knx-productdb/src/parse/mod.rs`, called by `parse/hardware.rs` and
`parse/catalog.rs` since the two byte-identical copies were merged on
2026-09-13, plus the inline equivalent for `application_program` in
`crates/knx-productdb/src/parse/program.rs` — recorded an `IdConflict` only
when an id it had already seen belonged to a *different* file — its only
test was `kept != source_sha256`, comparing the existing row's stored
`source_sha256` against the `source_sha256` the current parse call was
handed. Because one `ingest_hardware`/`ingest_catalog` call always passes
the same `source_sha256` for every element in that file, two `Hardware` (or
`Product`, `Hardware2Program`, `CatalogSection`, `CatalogItem`,
`ApplicationProgram`) elements sharing an `@Id` **inside the same file**
always compared equal and never reached the `IdConflict` branch: the second
element was dropped, first-writer-wins, with nothing recorded anywhere.

**Initially fixed for `first_winner`'s two callers.** `first_winner`
(`crates/knx-productdb/src/parse/mod.rs`) now takes an extra
`seen_this_call: &mut HashMap<(String, String), u32>` parameter, freshly
created once per `ingest_hardware`/`ingest_catalog` call and threaded
through every `first_winner` invocation made while parsing that one file.
Each call increments the count for `(table, id)` and reports it back as
`occurrence`. A conflict is now recorded when *either* `kept !=
source_sha256` (the original cross-file check, unchanged) *or* `occurrence
> 1` (new: this exact id has already been seen earlier in this same parse
call). `source_sha256` itself, and its meaning as file provenance
elsewhere (idempotent re-parse detection, translation backfill), is
untouched — this is additive, not a reinterpretation of an existing column.

The new key costs one field, not a schema rewrite: `IdConflict`
(`crates/knx-productdb/src/report.rs`) gained `pub occurrence: u32` (`1`
means "first sighting this call, so any conflict is the old cross-file
kind"; `>1` means "the Nth same-file sighting"). It is persisted by
reusing the existing `ingest_unknown.occurrences` column (previously
always written as the literal `1` for `IdConflict` rows) and, for
installed-package reports, a new `package_conflict.occurrence` column
(schema v6 → v7, `migrate_v6_to_v7`, `DEFAULT 1` for rows written before
this change). First-writer-wins behaviour is unchanged: the second element
in a same-file collision is still not stored as a row, but the fact that
it existed and lost is now visible in the report, satisfying CLAUDE.md's
"never silently discard information" for this path.
Was pinned, now proven fixed **[V]**, by
`two_hardware_elements_sharing_an_id_in_one_file_record_the_collision`
(`parse/hardware.rs`) and
`two_catalog_items_sharing_an_id_in_one_file_record_the_collision`
(`parse/catalog.rs`) — both are the exact same synthetic same-file-duplicate
input as their now-retired `..._conflict_silently` predecessors, with the
assertion flipped from "conflicts is empty" to "one conflict, occurrence
2"; run against the pre-fix code both would fail (and did, verbatim,
before this fix, since they are literally the old pinning tests renamed
and re-asserted).

**Closed for `application_program`, 2026-09-17.**
`ingest_program` now uses the same `first_winner` helper and a fresh
`seen_this_call` map for each source file. Two `ApplicationProgram` elements
with the same `@Id` in one file retain the first declaration and record an
`IdConflict` with `occurrence = 2`; the existing cross-file behavior remains
unchanged. The regression test
`two_application_programs_sharing_an_id_in_one_file_record_the_collision`
failed against the old inline logic and passes with the shared helper.

**Residue: `datapoint_type` still has no declaration provenance**, even
though collisions are counted: `knx_master.xml`'s `DatapointType`/
`DatapointSubtype` elements are written with a bare `INSERT OR IGNORE`
(`crates/knx-productdb/src/parse/master.rs`) into `datapoint_type`, whose
primary key is `id` alone with no `source_sha256` column to compare
against in the first place.

**AR05 audit, 2026-10-02 (candidate, offline).** New package regressions verify
same-file main/subtype collisions: four semantic declarations read, two stored,
two dropped, first normalized values unchanged, full master bytes retained and
historical facts stable after reopen/retry. An orphan subtype is read/dropped
but does not increment the legacy collision counter. These distinguish semantic
loss from retained bytes; they do not invent winner provenance or lift this
residue. Existing normalized product identity reporting is unchanged. Contract:
[MANUFACTURER_REPORT_CONTRACT](MANUFACTURER_REPORT_CONTRACT.md).

**Measured against the real corpus.** Every `.knxprod` file under
`OriginalData/ProductDatabases/` was copied to a scratch directory outside
the repository (never modified in place) and installed with
`knx_productdb::install_package` into one shared database, in the order
`ls` returns them, via a throwaway test gated on `KNXBENCH_PRODUCT_CORPUS`
— command: `KNXBENCH_PRODUCT_CORPUS=<scratch dir> cargo test -p
knx-productdb --test tmp_collision_probe -- --nocapture`, deleted after
this measurement, not part of this commit **[V]**.

- `first_winner`-tracked tables (`hardware`, `product`, `hardware2program`,
  `catalog_item`, `application_program`): **0** cross-file `IdConflict`s
  across the corpus's 4 distinct packages. The corpus is small — one real
  vendor package, one test-fixture package and two near-duplicate
  fixtures — and no two files declare overlapping manufacturer/hardware
  ids, so this measures "never observed here", not "cannot happen"; the
  same-file case (now closed for `hardware`/`catalog_item`/etc., see
  above) is demonstrated by a synthetic test instead because no real file
  in this corpus happens to contain one either.
- `datapoint_type` (the untracked path): **routine, not rare.** Every
  package's `knx_master.xml` restates the *entire* KNX-standard DPT
  catalogue rather than only the DPTs its own products use. Installing the
  4 distinct packages in sequence: package 1 declares 234 `DatapointType`/
  `DatapointSubtype` elements and the table grows by 234 (nothing to
  collide with yet); package 2 declares 354 and the table grows by only
  120 (234 silently dropped); package 3 declares 383 and the table grows
  by 29 (354 dropped); package 4 declares 234 and the table grows by 0
  (all 234 dropped). Total: **822 silent, uncounted drops across 4
  packages**, every one of them after the first hitting the collision on
  effectively its whole DPT declaration.

**Fixed, in a small and contained way.** The `datapoint_type` path had no
counter at all, so one was added: `MasterIngest::dropped_datapoint_types`
counts every `INSERT OR IGNORE` that changed zero rows, `InstallReport`
carries the sum as `dropped_datapoint_types` (persisted in a new
`package.dropped_datapoint_type_count` column, schema v6,
`migrate_v5_to_v6`), and `knx products ingest` in `apps/knx-cli` prints it
alongside the existing conflict count. This is a
count of drops, not a full `IdConflict` — `datapoint_type` still has no
`source_sha256` to build one from, so it cannot say *which* file's id won,
only that one lost.

**Remaining limitation — `datapoint_type` provenance.** The current counter
shows how many declarations collided, but the `datapoint_type` table has no
`source_sha256` column. Reports therefore cannot identify which file supplied
the retained declaration. This needs a deliberate schema migration and remains
separate from the now-complete same-file detection for every `first_winner`
caller.

**Residue grows, 2026-09-14 (T13, branch `d10-language-data`).**
`function_type`, `function_point` and `space_usage` (new tables, schema
v9 → v10, closing §64's own residue) are filled by `parse/master.rs` the
same bare `INSERT OR IGNORE` way `datapoint_type` already was — no
`source_sha256` column, no occurrence counter, same restated-whole-catalogue
collision shape a second `knx_master.xml` produces. Not measured
separately against the corpus the way `datapoint_type` was above; the
shape of the gap is identical, so it is stated rather than re-argued. A
future fix for `datapoint_type`'s residue should cover these three tables
in the same pass rather than leaving them a second time. The same three
element families also inherit `datapoint_type`'s other silent-discard
shape: a `FunctionType`, `FunctionPoint` or `SpaceUsage` with no `@Id`
attribute at all binds `NULL` into a `TEXT PRIMARY KEY` column, and
`INSERT OR IGNORE` drops that row with no error, no counter and no
`ingest_unknown` entry — exactly as an id-less `DatapointType` already
did before this slice, and not a new gap this slice introduces, only one
it extends to three more tables. An id-less `FunctionType` compounds the
loss: it orphans every `FunctionPoint` nested inside it too, and those
are then silently dropped a second time by the parentless-`FunctionPoint`
guard at `parse/master.rs:213` (T13 fix round 2).

---

## 87. A parse fix does not reach rows that were already ingested, and only a migration can go back for them

**Fixed for `linkable`, on 2026-09-14, by product-database schema v8, and for
`parameter_type`'s `Float`/`Text` bounds, the same day, by schema v9**
([ADR-0020](adr/0020-migrations-may-rederive-from-stored-bytes.md)). The
general shape of the defect is not fixed and cannot be, so this section stays
— rewritten to describe the class rather than either instance.

**Limitation.** Every parsed row in `products.sqlite` is a derived value, and
the derivation happens exactly once: at ingest. Three independent
short-circuits then make sure it never happens again for the same bytes —
`install_package` returns `skipped: true` on a package whose sha256 is already
on record (`crates/knx-productdb/src/package.rs`), `ingest_file` skips a blob
already in `source_parse_evidence` (`ingest.rs`), and `ingest_program` sets
`already_present` when a program id is already in the table
(`parse/program.rs`). They are all correct, and they are why a parse-layer fix
shipped today changes nothing about a database ingested yesterday. The bytes
are never lost — every member's XML is in `source_file`, which is the whole
point of ADR-0011 — but nothing re-reads them of its own accord.

**Consequence.** After any fix to the parse layer, a row that the old code
derived wrongly keeps the old answer, and a reader who checks whether the fix
worked by querying an existing database concludes that it did not. There is no
marker on a row saying which build derived it, so a stale value and a current
one are indistinguishable by inspection.

**Workaround.** Ingest into a fresh database, which re-derives everything
from the packages. Measured on the repository corpus (five `.knxprod`, three
`.knxproj`, debug build): 17.0 s for a 128 MB database. This needs the
original files, which the blob store exists precisely so a user need not keep
— see ADR-0020's alternatives for why that is a fallback rather than the
answer.

**The one instance that was repaired, and how.** `bool_flag`
(`crates/knx-productdb/src/parse/mod.rs`) accepted only `"1"` and `"0"` until
2026-09-13, so every word-spelled `Linkable` was stored as `NULL`
**[V]**. Not only in schema-20/21 packages, as this section previously
claimed: the spelling belongs to the tool that wrote the file, and
`grep -o 'Linkable="[^"]*"'` over the corpus finds word form in
`project/11`, `20` and `21` archives and numeric form in `project/11` and `23`
ones. `migrate_v7_to_v8` re-reads `ApplicationProgram/@Linkable` out of each
affected blob and fills the column, only where it is `NULL`, only for the row
that blob's bytes produced, and retires the now-false "attribute not
understood" `ingest_unknown` row it supersedes. Measured: 34 of 34 corpus
programs refilled in 1.14 s, with values identical to a fresh ingest (27
false, 7 true); a v6 database with nothing to fill opens in 0.038 s, the same
as one already at v8.

**The second instance, and how it differs.** T18 slice 5 (2026-09-13) gave
`parameter_type` real bounds — `TypeFloat/@minInclusive`/`@maxInclusive` and
`TypeText/@SizeInBit` — but, same as `Linkable` before it, only for rows
ingested from that day forward; every program ingested earlier keeps
`min_inclusive`/`max_inclusive`/`size_in_bit` `NULL` regardless of what its
own `source_file` blob actually says. `migrate_v8_to_v9` re-reads those three
attributes out of each affected blob and fills the columns, only where
`Float`'s pair is `NULL` together or `Text`'s size is `NULL`, only for the row
that blob's bytes produced (`backfill_parameter_type_bounds` in
`crates/knx-productdb/src/migration.rs`, mirroring `backfill_linkable`'s
shape exactly — `parameter_type` has no `source_sha256` of its own, so the
scoping check joins through `application_program`, which does). One respect
in which it does not mirror `linkable`: there was no stale `ingest_unknown`
row to retire, because the pre-T18 parser never asked for these attributes at
all — it neither read them nor rejected them, so nothing was ever reported.
A separate, unrelated fix landing the same day
(`insert_parameter_type` now calls `report_unknown_attrs` on `TypeFloat`'s
children) means a freshly-ingested row and a v9-backfilled row still differ
in one way this migration does not close: the fresh row also gets
`ingest_unknown` entries for `Encoding`/`Increment`/`DisplayFormat`, and the
backfilled one does not. Named here rather than silently left different.

This is the class's **second** occurrence, not its third.

**Lifted when.** For the class: never entirely, by construction — a
derivation that already ran cannot know it should run again. What is missing
is only the *detection*, and ADR-0020 records the shape of it (a parse
generation recorded per `source_file` row, reported by `knx products verify`),
deliberately not built for a single column. Reach for it if the class turns up
a third time — two is not yet that threshold. For an individual instance: a
v-next backfill migration, under ADR-0020's rule — permitted when the value
is a pure function of bytes the database already holds, forbidden when it
depended on the install event.

## 88. A manufacturer's display name is last-writer-wins, and that is on purpose

**Limitation.** `ingest_master_data`'s `"Manufacturer"` arm
(`crates/knx-productdb/src/parse/master.rs`) writes `name` with `INSERT ...
ON CONFLICT(id) DO UPDATE SET name = excluded.name` — whichever
`knx_master.xml` is ingested last overwrites the name every earlier one
wrote. Every other id-collision path this crate has is first-writer-wins
instead, plus a recorded `IdConflict` when the losing row's file differs:
`first_winner` in `parse/mod.rs`, shared by `hardware.rs` and `catalog.rs`
since 2026-09-13 and by `program.rs` since 2026-09-17 (§86). Since PDB-11
(schema v17, §135) every such candidate element is also recorded with its
digest; manufacturer names are master data and are not. Manufacturer names
update silently and take the opposite side.

**Cause.** `hardware.rs` and `catalog.rs` can each create a manufacturer row
stub (`id`, `name = NULL`) before any `knx_master.xml` naming it has been
ingested — order between the two is not guaranteed. `first_winner` semantics
applied literally would make the first arrival "win", `NULL`-name stub
included, and the name would then never get filled in by a later,
better-informed `knx_master.xml`. The existing test
`a_manufacturer_seen_during_ingest_first_gets_its_name_later` pins the
required outcome: a `NULL` stub inserted first still ends up with a real name
after `ingest_master_data` runs, in either arrival order. That stub is a
constraint on any rule chosen here, not an argument for this one — a
first-writer-wins variant can satisfy it, and the Ruling below names the one
that does and says why it lost anyway.

**Measured against the real corpus.** Swept 69 real `knx_master.xml` files
(pattern search across the filesystem, not one remembered path) for
manufacturer ids whose declared `Name` differs between files. Of 832 distinct
manufacturer ids seen, 52 have more than one `Name` on record — real ETS
rebrandings, not typos. Quoted exactly as the corpus spells them, in no
particular order, because the corpus offers no way to order them: `M-0007`
is either `"Busch-Jaeger Elektro"` or `"ABB AG - BUSCH-JAEGER"`, `M-003D`
either `"WAGO Kontakttechnik"` or `"WAGO GmbH & Co.KG"`, `M-0085` either
`"Video-Star"` or `"GVS"`, and 49 further ids are the same shape **[V]**.
Which spelling is the newer one is not stated anywhere this ingest can read:
no file carries a timestamp or version marker inside the `Manufacturer`
element itself that would let it tell "the newer file" from "the one that
merely happened to be read second" — file mtimes and ingest order are the
only signal available, and mtimes are not part of the KNX master-data
grammar, so they are not read at all. Every id above has exactly two
spellings on record; no id in this corpus has three.

**Consequence.** Ingesting an old package after a new one silently reverts a
manufacturer's display name to its old spelling. There is no `IdConflict` and
no `ingest_unknown` row, because this was never a data-loss path in the sense
those exist for (`kept_sha256`/`other_sha256`) — no id-scoped row is ever
dropped, only overwritten, and every overwrite has the exact same
justification: some later file's opinion of the correct spelling.

**Ruling, 2026-09-13, with its reasoning corrected 2026-09-13.** Aligning
this with `first_winner` was considered and rejected, and the conclusion
stands — but not on the argument first written down here, which claimed
first-writer-wins *must* strand the `NULL` stub. It need not.
`ON CONFLICT(id) DO UPDATE SET name = COALESCE(name, excluded.name)`
satisfies both `a_manufacturer_seen_during_ingest_first_gets_its_name_later`
and first-writer-wins for real names, in one statement, and was the option
this entry should have named and did not.

What actually decides it is that `COALESCE` does not buy what it looks like
it buys. It does not remove the order-dependence, it relocates it: the
*first real* spelling wins forever instead of the last one, and which
spelling that is still depends on which file the ingest opened first — with
52 ids in this corpus carrying two spellings apiece, that is the same coin,
flipped earlier. It then removes the only repair the user has: once a real
name is in the row, no later `knx_master.xml` can correct it, so shipping a
package that renames a manufacturer would have no effect on an existing
database and no report to say so. A manufacturer's display name is not a
fact fixed at first sight the way a hardware id or a catalog item is — ABB
really did rename Busch-Jaeger's `knx_master.xml` entry, and the two
spellings sit side by side in this corpus with nothing to rank them. Given
two order-dependent rules and no recency signal, the one that lets newly
ingested master data have an opinion is the more useful, and it is one
statement rather than one statement with a hidden third state. That is the
whole of the case; it is a preference with a reason, not a proof.
`manufacturer_names_are_filled_in` and
`a_manufacturer_seen_during_ingest_first_gets_its_name_later` lock the stub
behaviour in; the characterization test
`a_later_ingested_master_file_updates_the_name_the_earlier_one_wrote` pins
the last-writer-wins case explicitly, order-dependence named in the test's
own name so nobody mistakes it for an invariant.

**Lifted when.** Never, unless `knx_master.xml` grows a field this crate can
use to actually rank two spellings by recency (a schema/edition attribute
would do it) — at which point "last ingested" could become "provably newer",
and this section would describe that instead.

**A second scan order, not covered above (2026-09-14, T13 fix round 2).**
Everything above is about first *ingest* order. The v9→v10 backfill
(`backfill_function_and_space_data`, `migration.rs`) replays this same
last-writer-wins `Manufacturer` write over every blob a database already
holds, ordered by `source_file.rowid` — the order distinct blobs were
first *written* to that table, which is not always the order they were
first *ingested*. `store_source_file` (`blob.rs`) returns `false` without
inserting a row when a blob's sha256 is already on record, but
`install_package` (`package.rs`) calls `ingest_master_data` on that blob
regardless, so a master blob installed by two different packages is
ingested twice but occupies one rowid — the backfill then replays it once,
at its *first* install's position, which can differ from its *last*
install's position (the one whose names actually won under last-writer-wins
at real install time). Measured **[V]**: installed Weinzierl 730 ETS4, then
MDT KP AMI/AMS 03, then the Weinzierl archive repacked with one XML comment
appended to `M-00C5/Catalog.xml` (package hash differs, `knx_master.xml`
byte-identical to the first install). Rolled back to `user_version = 9`,
dropped the three v10 tables, reopened through `open_and_migrate`: **21 of
799** manufacturer display names changed relative to the pre-rollback
database — `M-0002` from `ABB` to `ABB AG - STOTZ-KONTAKT`, `M-0007` from
`Busch-Jaeger Elektro` to `ABB AG - BUSCH-JAEGER`, `M-000A` from
`INSTA ELEKTRO` to `Insta GmbH`; `translation`, `datapoint_type` and
`ingest_unknown` counts were unchanged. `ORDER BY rowid` is still the right
order to hold — it is the only order `source_file` actually records — but
it reproduces first-install's own answer only when no master blob in the
database was ever installed by more than one package; the third install
above was constructed specifically to violate that, to make the residual
measurable rather than asserted.

## 90. There is no DPT main type 46; 46 is a *count* of main types in one ETS master-data file

**Limitation.** Not a limitation of the code — a limitation of a number that
has been circulating through this repository's own planning documents, and
that reached a task brief as a requirement. `goal.md` §3 row **E4** briefed
the DPT-codec work as "main types 20, 21-30 and the rest of the 46
`knx_master.xml` main types", and task 5's brief shortened that to "main
types 20-30 and 46". There is nothing numbered 46 to implement. Numbers 88
and 89 are claimed by two other branches in flight alongside this one; this
entry is 90 for that reason and no other.

**Where 46 actually comes from.** `docs/RESEARCH.md` §5 measured the DPT
catalogue of one specific master-data file — the one inside
`OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj` — and
found 46 `DatapointType` elements. That file's 46 main-type ids are `DPT-1`
through `DPT-23`, then `DPT-25`, `DPT-26`, `DPT-27`, `DPT-29`, `DPT-30`,
then eighteen LTE/system types in the 200-series (`DPT-206`, `DPT-217`,
`DPT-219`, `DPT-222`, `DPT-229`, `DPT-230`, `DPT-232`, `DPT-234`,
`DPT-235`, `DPT-237`, `DPT-238`, `DPT-240`, `DPT-241`, `DPT-244`,
`DPT-245`, `DPT-249`, `DPT-250`, `DPT-251`) — re-measured on 2026-09-14
with

```sh
unzip -p "OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj" \
  knx_master.xml | grep -oE 'DatapointType Id="DPT-[0-9]+"' \
  | sed 's/.*DPT-//;s/"//' | sort -n
```

RESEARCH.md §5 already warns, in so many words, that this is "a property of
*this one* master-data file, not a fixed constant". The warning was right
and was read past anyway: the other `knx_master.xml` copies under
`OriginalData/` carry 29, 49 and 55 main types, and the two other demo
projects carry 57 and 63. A count that changes with the ETS vintage cannot
be an identifier.

**What the Standard says.** DPT-AS (`03_07_02 Datapoint Types v02.02.01
AS`) §2's overview table enumerates main types 1 to 31 and then jumps
straight to the 200-series; nothing between 32 and 199 exists. The nearest
thing to a "46" in that document is *clause* §3.46 "Datatypes A8A8A8A8",
whose datapoint type is `231.001 DPT_Locale_ASCII` — a clause number, not a
main type. Searched explicitly so the next reader does not have to:
`46.001` against `knowledge_base/knx_spec_kb_full179_clean.sqlite` (177
PDFs) returns exactly one hit, and it is a Connection Code table row from
`03_07_03 Standardized Identifier Tables v01.04.01 AS` —
`46 | 2Eh CC_FanSpeed | 5.001` — where 46 is a connection-code number and
the datapoint type is 5.001. `DPST-46` returns `[]`.

```sh
cd /mnt/daten-i/Sourcecode/knx-spec-kb && .venv/bin/python \
  scripts/05_knowledge_base_v1.py -o knowledge_base/knx_spec_kb_full179_clean.sqlite \
  --query "46.001" --limit 3
```

**Consequence.** No semantics were invented to fill the gap, and
`codec.rs`'s `a_main_type_above_the_implemented_range_is_unsupported_not_a_panic`
test pins main type 46 among the numbers that must return
`DptCodecError::UnsupportedDpt` rather than anything cleverer. Main type
**31** *does* exist (31.101 `DPT_PB_Action_HVAC_Extended`, DPT-AS §4.7.1)
and is deliberately not implemented: its own entry reads "This DPT shall
not be used for runtime communication. This DPT shall only be used for
encoding Parameter values in CH_PB_HVAC_Mode_1", so a group-value codec has
no honest use for it. It is also absent from every `knx_master.xml` in the
repository.

**Lifted when.** Never, as stated — there is nothing to lift. This entry
exists so the number stops being re-derived. If a future brief asks for
"main type 46" again, it means "the remaining main types in some
`knx_master.xml`", and the right first step is to measure the file in front
of you.
## 92. Commissioning phase 2 is verified against a simulator this project wrote; one device has been written since

**Corrected 2026-09-28.** This entry used to say the code *"has never
addressed a device"* and that *"no frame produced by this code has ever
left the machine"*. Both stopped being true on 2026-09-26. What has been
observed on real hardware so far, all on `1.1.67` (MDT, mask `0701h`):

- `[V]` individual-address write (RESEARCH §8.8.6);
- `[V]` memory download, restart request and load-state machine over
  memory (RESEARCH §19.4). The Transport Layer met a lost `T_ACK` and a
  repeated answer, and those cases are now handled;
- `[V]` cEMI framing, APCI packing and 3 s acknowledge timing, end to end
  against a real device on those paths.

Everything else below still holds unchanged for the property-based
procedures (complete, one-part and partial download, unload, recovery).
Those have run only against the simulator.

**Limitation.** The download protocol of
[docs/superpowers/specs/2026-09-13-commissioning-download-design.md](superpowers/specs/2026-09-13-commissioning-download-design.md)
is implemented and tested — 247 tests across `knx-core::commissioning` and
`knx-net::commissioning` (131 in `knx-core`, 116 in `knx-net`; **re-measured
2026-09-20**, correcting the "153" this section previously stated — count
with `cargo test -p knx-core commissioning:: -- --list` and
`cargo test -p knx-net commissioning:: -- --list`, summing each run's
"N tests" line), all of them against
`crates/knx-net/src/commissioning/simulator.rs`. **No frame produced by this
code has ever left the machine.** Every statement the implementation makes
about device behaviour is "what the Standard says a Management Client sends,
plus an unobserved delta", and the delta is unmeasured. Nothing here is a
claim of KNX certification, and nothing here is a claim of ETS
compatibility, verified or otherwise: no ETS-produced download capture
exists in this repository to compare against. Entry 92 and not 91 because
91 is claimed by another branch in flight.

**Cause, in the parts that matter separately.**

- *The simulator answers decoded services, not octets.* It consumes
  `ApplicationService` values and produces `ApplicationService` values. So
  it exercises the procedures, the Load State Machine, the chunking and the
  read-back rules, and it does **not** exercise APCI bit packing on the
  wire, cEMI framing, or a device that answers with a malformed frame. The
  encode/decode side has its own tests in `crates/knx-net/src/cemi.rs`;
  the two have never been run against each other end to end, because doing
  that needs a bus.
- *Its timings are immediate.* A real device takes time; the simulator
  answers within the same task. The §5.5 wait loop's poll interval, its 30 s
  ceiling and the reconnect path are all tested, but with a simulator that
  is told to be slow or to drop the connection, never with a device that
  simply is. The 3 s acknowledge time-out and `max_rep_count = 3` of TL
  clause 4 are implemented and untimed.
- *The programming delay is this project's number.* `SessionTiming`'s
  500 ms default between a memory write and the verification read of §6.2
  exists because **MP §3.16 never quantifies it** — it says a delay is
  needed and stops. 500 ms is an invention of this project, marked `[A]` in
  the source, and the first real device may need more or may need none.
- *Two failure shapes are asserted only because the simulator was asked to
  produce them.* §10.8's silent drop of a `PID_LOAD_STATE_CONTROL` write
  (an access level high enough to read and too low to write) and §10.5's
  "no protected areas" device are configuration flags. Both are documented
  device behaviours; neither has been observed here.
- *No hardware write path is reachable at all.* By ruling, not by accident:
  §2.3's `WriteAuthorisation` can be constructed for a simulator target or
  by an operator-confirmed constructor naming one concrete device, and
  `ManagementTransport::target_kind()` answers `Hardware` for the real
  tunnelling transport, which the session refuses. The refusal is tested;
  the write it refuses is therefore also untested.

**Consequence.** A user cannot download to a device with this code, and
should not be told that the protocol "works" — only that it is complete and
self-consistent against the clauses it cites. The next honest step is
phase 3's **read-only** observation inside `1.1.24`–`1.1.32`: read each
loadable part's `PID_LOAD_STATE_CONTROL`, `PID_ERROR_CODE`,
`PID_DEVICE_CONTROL`, `PID_OBJECT_INDEX`, Device Descriptor Type 0 and
`PID_MANUFACTURER_ID`, and compare the shapes against the design document.
Deviations are expected. `1.1.220` is an alarm panel and is excluded
structurally (`crates/knx-core/src/address.rs`), which phase 3 does not get
to relax.

**Lifted when.** Partially, by phase 3's read-only observation: it can
confirm or refute the property shapes, the mask version, the APDU length
rule of §6.4 and the load states of a real device, and each deviation it
finds is a finding rather than a fix. Fully, never by testing alone — a
download that has been observed to succeed on one manufacturer's device is
evidence about that device. This entry narrows with each observed device
and does not close.
*Narrowed 2026-09-28* by one device and two operations (see the correction
at the top).

<a id="93-knx-cores-declarative-procedure-model-still-writes-pid_program_version-unconditionally-for-every-part--parked-deferred-to-task-c11"></a>

## 93. `knx-core`'s declarative procedure model still writes `PID_PROGRAM_VERSION` unconditionally, for every part — PARKED, unowned

**Limitation.** `crates/knx-core/src/commissioning/procedure.rs`'s
`load_one_part()` renders CP §3.5.2 step 06 as a fixed, five-step-plus-two
list, dry-runnable without a bus (spec §11.2's *"a procedure model, not a
script"*). Its step 5 is unconditional:
`step(5, "set the version", "PropertyWrite PID_PROGRAM_VERSION",
StepEffect::Write)`, for every loadable part, every time. Task C1 taught the
*executed* procedure in `crates/knx-net/src/commissioning/download.rs` to
know better: RES Table 90 (p. 288) and Table 91 (p. 290) give
`PID_PROGRAM_VERSION` to Application Program 1 and 2 only, and RES Table 77
(p. 238), Table 80 (p. 249) and Table 85 (p. 270) do not give it to the
Group Address Table, the Association Table or the Group Object Table —
`download.rs`'s `PartKind::has_program_version()` is that fact, and
`load_one_part` (the `knx-net` function, not the `knx-core` step-list
builder that shares its name) now writes the property unconditionally only
for the two kinds that have it. The declarative model in `knx-core` was not
told. It still renders step 5 as a `Write` for a part it cannot know is a
table, so a dry run of `procedure::load_one_part()` and a real
`download::load_one_part()` no longer agree on what step 5 does for three
of the five loadable parts.

**Cause.** `PartKind` lives in `knx-net`, one layer above `knx-core` in this
project's dependency direction (`knx-core` is the domain the rest of the
crates depend on, not the reverse — see `CLAUDE.md`'s architecture section).
Teaching `procedure::load_one_part()` about `PartKind` would make `knx-core`
name a `knx-net` type, which points that dependency backwards. Whether the
fix is moving `PartKind` down into `knx-core`, parameterising the step list
by an existing `knx-core` concept, or something else is a design decision,
not a bug fix, and task C1's brief explicitly scoped it out: *"do not build
`PartKind::ALL`... those are parked to C11 and C12 on purpose, because
building them now means guessing their shape twice."* This entry records
the resulting model disagreement as a **finding, deliberately not fixed
inside C1** (its audit number there is F8).

**Consequence.** Nothing that reads `download.rs`'s trace is wrong — the
executed procedure is the one with the correct, per-part-kind behaviour,
and its `VersionOutcome`/step-5 label say so accurately. What is wrong is
trusting `knx-core::commissioning::procedure::load_one_part()` alone, without
cross-checking `download.rs`, to describe what step 5 does for a table
part: the declarative model currently overstates it, still showing an
unconditional write where the real one is conditional or tolerant of a
refusal.

**Lifted when.** Corrected 2026-09-20: task C11 merged without touching
this — its actual scope was CP §3.5.3's five partial-download step-list
variants (`crates/knx-core/src/commissioning/partial_download_variant.rs`),
a different finding entirely, and task C12 (which followed it, wiring the
sequencer to that variant table) didn't touch it either.
`procedure.rs`'s `load_one_part()` step 5 is still the unconditional
`PropertyWrite PID_PROGRAM_VERSION` this entry originally described (see
that function, currently around lines 372-373). No task currently owns
the dependency-direction decision between `knx-core` and `knx-net` for
`PartKind` that this fix needs — it is unowned, not merely unscheduled.
Lifted when a task is opened for it and makes that call, or when
`knx-core` gains its own concept for "does this part kind carry a program
version" that `PartKind` could be expressed in terms of instead.

**Kept parked, 2026-09-28 (goal-commission K8).** User decision 2026-09-28,
asked for each K8 entry to fix it or accept it: *"das was am sinnvollsten
ist"* (whatever makes the most sense), i.e. the recommendation. Parked stays
parked: the v1 path (K9) uses `run_memory_download`, not the declarative
procedure model, so this does not reach a device. It reopens with any
feature that runs `procedure.rs` against hardware.

## 95. Six places where the KNX Standard's printed text must not be followed literally

**Limitation.** None — this is a pointer, not a cost. `03_05_03 Configuration
Procedures` ("CP") and `03_05_01 Resources` ("RES") contain six places where
the printed text either contradicts itself or contradicts its own surrounding
rows, and the commissioning code in `crates/knx-core/src/commissioning` and
`crates/knx-net/src/commissioning` deliberately does not follow the literal
text there. A seventh candidate — CP §3.5.2 and §3.5.3 seemingly giving
opposite answers on whether `PID_PROGRAM_VERSION` is written to the three
table objects — turned out on task C18's audit of Volume 6 Profiles to be a
legitimate use of a Property Annex A marks optional there, not errata; it is
recorded as a resolved specification question in
[COMPATIBILITY.md §6](COMPATIBILITY.md#6-specification-questions-resolved-against-the-standard-not-errata)
instead of here. The full list of the remaining six, each item carrying its
clause and its PDF page — three of the table-variant load procedures reading
Application Program 2's Memory Control Block where every neighbouring row
reads its own; a Group Object Table step filed under the Address Table's
load row (and repeated for AP1 and GOT); a `LoadCompleted` event listed
inside an unload wait, where RES Table 94 marks that transition optional
rather than a step of unloading; a cross-reference to a section that does
not contain what it is sent to find; RES §4.23.2.4.1 naming a Load Control
*value* (`LoadCompleted`, `02h`) as if it were the terminal Load *state*
(`Loaded`, per Table 92); and three mutually incompatible part-unload/load
orderings across CP §3.5.2 and §3.5.4 — lives in
[COMPATIBILITY.md §7](COMPATIBILITY.md#7-knx-standard-errata--printed-text-this-project-deliberately-does-not-follow),
next to the other compatibility claims and their evidence, rather than here.

**Cause.** A specification this size is not internally consistent by default,
and Configuration Procedures in particular reads like several authors' work
stitched together — the disagreements above are exactly the shape that
produces (misplaced rows, a stale cross-reference, a state/value name
collision), not the shape a single careless reading would produce.

**Impact.** None on a user. Every item above was already the code's actual
behaviour before this entry existed; what changed 2026-09-19 is that the
divergence is now written down next to its clause, so a future reviewer
comparing this code to the printed Standard finds the explanation instead of
mistaking correct behaviour for a bug.

**Lifted when.** It already is; this entry (and its cross-reference) is the
lifting. It would only need revisiting if a later KNX Standard erratum or
edition corrects one of the six clauses, at which point the corresponding
list item names which one no longer applies.

## 97. Progress is a phase label far more often than it is a percentage

**Limitation.** Of the seventeen phases a load reports, exactly two carry a
`completed`/`total` pair: reading the container entries the exporter cannot
regenerate, and ingesting manufacturer files. Every other phase — including
`parseTopology`, which is the longest one in the maintainer's reference
project — shows an indeterminate indicator and its name. The bar does not
fill smoothly from 0 % to 100 %, because for most of a load there is no
honest number to fill it with.

**Cause.** A percentage needs a total that is known before the work starts.
Topology parsing is a streaming XML pass with no element count in hand;
enrichment visits whatever the product database happens to resolve. The
alternatives — elapsed time, compressed size, the phase ordinal — are all
forbidden by ADR-0023 for the same reason: they would report something
other than progress while looking exactly like progress.

**Consequence.** Users see "Parsing the topology" with a moving indicator
rather than "43 %". This is the intended trade, not an unfinished feature.

**Lifted when.** A phase gains a total that is genuinely known in advance —
counting topology elements in a cheap first pass would be one way, and
would have to pay for itself in measured time before it is worth it.

## 98. Almost nobody will ever see the second flavour message

**Limitation.** The load banner carries fifty rotating flavour lines
(`loadProgress.flavour.01` … `.50`, English and German), shown one at a time
beside the truthful phase label and swapped every 1 800 ms. The measured
release load of the maintainer's reference project is **114 ms**, and the
debug build's is roughly 1.2 s. At that interval a release load shows
**one** message and a debug load shows one or two. The list is written for
slow loads and large projects; a normal import will never cycle it.

**Cause.** The interval is deliberately slower than the 250 ms progress
poll, because the phase label is the news and a joke changing faster than
the news competes with it. Shortening the interval to make the list visible
would trade a real reading problem for an imaginary entertainment one.

**Consequence.** Fifty strings exist, translated twice, and the overwhelming
majority of loads render exactly one of them. They are not dead
code — every one is reachable, and the shuffle picks a different opener each
load — but nobody should expect the list to be seen as a list.

**Not a progress indicator.** Worth stating next to entry 97: the flavour
line is decoration. It is `aria-hidden`, it is excluded from the banner's
polite live region, its timer advances nothing but its own text, and it is
not rendered at all once a load has failed. Under
`prefers-reduced-motion: reduce` or the application's own "Motion: off" it
freezes on its first entry rather than disappearing.

**Lifted when.** Nothing lifts this. It is what the feature is.

## 99. `MemoryControlBlock`'s access-nibble order is inferred, not spec-stated

**Limitation.** RES §4.2.27, Table 12, p. 39, lays out `PID_MCB_TABLE`'s
eight-octet element as Segment Size 1 (4 octets), CRC Control Byte (1
octet), Read Access 1 and Write Access 1 sharing one octet as two 4-bit
nibbles, then CRC (2 octets). The table names which nibble is Read Access 1
and which is Write Access 1 by column position only; it does not state
which nibble is high and which is low, the way some other split-byte
KNX fields (e.g. `PDT_UNSIGNED_CHAR` sub-fields elsewhere in RES) do. This
project's `crates/knx-core/src/commissioning/mcb.rs` reads Read Access 1 as
the high nibble and Write Access 1 as the low nibble, matching the table's
left-to-right column order — a reasonable convention, but an inferred one,
not a cited one. Neither `PID_MCB_TABLE` nor a load-verify path in Task C2's
scope (CP §3.5.3) reads or acts on these two fields; only the CRC (octets
6-7) and the CRC Control Byte's bit 0 (RES §4.2.27.1.1, Table 13, p. 39) are
compared, so this order has no effect on any current comparison outcome.

**Cause.** The printed table's ruling groups Read Access 1 and Write Access
1 visually into one narrow column, and the corresponding text run gives no
bit-numbered breakdown the way Table 13 does for the CRC Control Byte's
individual bits. Nothing else in RES §4.2.27 cross-references this nibble
order either.

**Impact.** None measured. `MemoryControlBlock::read_access` and
`::write_access` are populated and round-trip correctly under this
project's own convention, but a device or a second implementation that
reads the nibbles the other way round would silently disagree with this
project's labelling of which access level applies to which side — while
still agreeing on every octet's raw bit pattern, since nothing is
reordered, only relabelled.

**Lifted when.** A KNX-certified reference implementation, a conformance
tool, or explicit bit-numbered spec text for Table 12 (matching Table 13's
style) settles the nibble order, or a real device's read/write-access
behaviour is observed to disagree with this project's current labelling.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
The nibble order stays inferred and labelled as such; the "Lifted when"
condition above still applies, and a device that disagrees reopens it.
## 100. Help prose lives in the message catalogue, one paragraph per key

**Limitation.** T28's help text (ADR-0024) is stored the same way every other
user-facing string is: as entries in `apps/knx-web/src/messages/en.ts` and
`messages/de.ts`. One key holds one paragraph of plain text. There is no
Markdown, no rich text, no per-topic file, and no separate help store. The
`help.*` keys alone are 50 of them (51 with `toolbar.help`), and they are the
longest strings in either catalogue by a wide margin: adding them grew
`en.ts` by 16% in bytes while adding 7% of its keys (50 of 684).

**Cause.** A deliberate trade, made in ADR-0024 and recorded here rather than
rediscovered later. The catalogue is the one storage mechanism where a missing
German string is a *compile* error, because `de.ts` is typed as
`Record<MessageKey, string>`. Any second mechanism — Markdown files, JSON, a
help-only catalogue — would have to re-earn that guarantee, and until it did,
an untranslated help topic would ship silently. Sentence-shaped prose in a
TypeScript object literal is the price of that check.

**Consequence.** Three concrete costs, none of them fatal, all of them real.
(a) Reflowing a topic — merging two paragraphs, splitting one — is a key
rename in two files plus the `bodyKeys` list in `help.ts`, not an edit to a
paragraph. (b) No translation tooling sees this text: no translation memory,
no fuzzy matching, no `.po` round trip, so a future third language is a
manual rewrite of every paragraph rather than a diff against the last one.
(c) The catalogue is now doing two jobs — labels and prose — and prose is by
volume the larger. If a third language arrives, or if help grows past roughly
double its current size, the right answer is probably a dedicated help store
*that keeps the compile-time completeness check*; designing that is not this
task's work, and doing it speculatively would have shipped a second mechanism
with no evidence that the first one was inadequate.

**Not a rendering limitation.** Plain text is also a security choice, not only
a storage one: help paragraphs are rendered as text nodes, never through
`dangerouslySetInnerHTML`, so a translated string cannot introduce markup. A
help topic that genuinely needs a list or a table is a signal that the topic
is too long for a panel, not a signal that the catalogue needs a parser.

**Lifted when.** A third UI language lands, or the help corpus roughly
doubles — whichever comes first. Either is enough evidence to design a help
store properly; neither has happened.

## 101. RES §4.23.2.4.1's "once more" attempt can roughly triple a load-state wait's worst-case latency

**Limitation.** C5 (`wait_for_load_state`, `crates/knx-net/src/commissioning.rs`)
correctly grants exactly one extra poll past `max_transition`, per RES
§4.23.2.4.1's *"once more when the maximum transition time has passed"*. The
flag that gates it latches *after* the read that discovers the deadline has
passed and *before* the loop's next `tokio::time::sleep`, so the extra
"attempt" is a complete loop body: a full `poll_interval` sleep, an optional
`reconnect()`, and a full `read_load_state` — which, since Task C4, retries
up to `MAX_TRANSMISSIONS` (4, not 3) times before giving up, at
`response_timeout` (`ACKNOWLEDGE_TIMEOUT` = 3 s) each. With
`SessionTiming::default()` (`connection_timeout` 6 s = `CONNECTION_TIMEOUT`,
`response_timeout` 3 s, `poll_interval` 3 s, `max_transition` 30 s) and
`AuthorisationPlan::Skip`, the worst case for that one extra attempt —
device stays in a state Table 94 permits silence in, so `NoAnswer` is
swallowed rather than returned — is:

```
poll_interval sleep (3 s) + read_load_state (4 × 3 s = 12 s) = 15 s
```

with no reconnect needed, since the connection never dropped. The read that
*first notices* the deadline has passed can itself have taken up to 12 s
(the same 4-attempt ladder, if that poll also went unanswered), so a caller
who budgeted `max_transition = 30 s` can see this call block up to
`30 + 12 (last pre-deadline read) + 15 (the one extra attempt) = 57 s` in the
worst case, against a pre-C5-fix bound (C4 already applied) of
`30 + 12 = 42 s`. If the extra attempt's `reconnect()` is also needed (the
connection *did* drop), `reconnect()` re-runs `connect()`, adding up to
`connection_timeout` (6 s) for `AuthorisationPlan::Skip`; the total for that
case is `30 + 12 + 3 + 6 + 12 = 63 s`. `AuthorisationPlan::WithKey` is worse
still: `reconnect()` → `connect()` also calls `authorise()` (`exchange`-based,
so up to another `MAX_TRANSMISSIONS × response_timeout` = 12 s) and, when a
session scope is set, `assert_verify_mode()` (at least one more such
exchange, possibly a read-modify-write pair) — each one an additional
worst-case delay this document does not attempt to total exactly, since
`assert_verify_mode`'s own exchange count was not fully enumerated for this
entry.

**Cause.** The Standard authorises the one extra attempt but does not bound
what an "attempt" is allowed to cost, and this project's attempts are not
free: C4 (Task C4, same review cycle) independently widened every exhausted
exchange from 9 s (`MAX_REP_COUNT` = 3 attempts × 3 s) to 12 s
(`MAX_TRANSMISSIONS` = 4 × 3 s), which compounds with C5's extra attempt
rather than being independent of it.

**Impact.** Latency only, not correctness: `SessionError::TransitionTimedOut`'s
`waited` field reports the true elapsed time, not `max_transition`, so
nothing misreports how long the call actually took. A commissioning UI that
shows a progress indicator keyed to `max_transition` (30 s by default) can
appear to hang for up to roughly twice that, worse still under
`AuthorisationPlan::WithKey`, before the call returns.

**Lifted when.** A UI surface actually renders `SessionTiming::max_transition`
as a hard progress bound (none does yet, per entry 97) — at which point
either the bound must be widened to reflect the true worst case, or the
extra attempt's own cost must be capped independently of `MAX_TRANSMISSIONS`
and `response_timeout`.

**Status 2026-09-28 (K5).** Still open, now with a bound on where it shows.
The first UI that runs a download, the **Download to device** tab
(`DeviceDownloadPanel`), renders no time bound at all: its progress is
steps started out of the plan's steps, and octets read back out of the
plan's octets, both counted from events the executor actually reported. A
load-state wait that runs the extra attempt therefore shows as one step
that takes longer. Nothing claims to finish at `max_transition`, so nothing
appears to hang against a promise. The limitation still applies to any
future surface that shows a countdown.

**Deferred to hardware, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
Only a real device can show whether the extra attempt matters; no surface
renders the bound (K5 status above). Measured opportunistically in the K7
live session, otherwise accepted as is. **K7, 2026-09-29: nothing to
measure.** In all three live downloads each load-state wait was answered
at once (`Loading` → `Loaded`, no extra attempt; RESEARCH §19). Accepted
as is.
## 102. The write echo's decode-failure branch has no known real trigger

**Limitation.** `POST /api/bus/write`'s `decodedEcho` (task 27) carries a
`kind: "error"` branch for when the bytes just sent fail to decode against
the DPT they were just encoded with. No shipped DPT has been found that can
actually reach it through the public API — the branch exists in the type and
in `decode_single`'s dispatch, but no test drives it end to end, because no
input was found that makes it fire honestly.

**Cause.** `crates/knx-core/src/dpt/codec.rs`'s codec is symmetric by
construction. Roughly fifteen main-type `encode`/`decode` pairs were audited
for task 27 looking for one where `encode` accepts a value `decode` then
rejects (main types 1, 5, 6, 9, 10, 11, 15, 16, 19, 20, 21/22/27/30, 24, 28).
Every validation that could produce such an asymmetry turned out to be either
skipped identically on both sides, or enforced by one function both
directions call (`char_set_is_ascii` for main 16, `mode3_code_is_assigned`
for main 6.020). Nothing in the audit suggests this was an accident to fix;
it reads as a design property worth keeping, not a gap.

**Consequence.** The error branch is exercised only indirectly: at the unit
level, by handing `decode_single` hand-crafted bytes no `encode` call would
ever produce (as the pre-existing telegram-decode tests already do for the
bus monitor), never by a write that round-trips through `knx_core::encode`
then `knx_core::decode` inside the same request. If a future DPT — or a
future edit to an existing one — introduces a real asymmetry, the write
handler will surface it correctly (response stays `200`, the mismatch travels
as text) without any code change, but no regression test will notice the day
that asymmetry appears, because none exists to break.

**Lifted when.** An encode-succeeds/decode-fails case is found or introduced
for some DPT, giving a server test something real to drive the branch with.
Until then, adding one anyway would assert nothing the codec's own contract
does not already guarantee some other way.
## 104. A device that goes offline mid-`LoadCompleting` now costs a full reconnect per quiet poll

**Limitation.** Since C19, a connected request whose four transmissions
(`MAX_TRANSMISSIONS`, TL §3, p. 15) all go unacknowledged releases the
Transport Layer connection, per TL §3.9, p. 15 and the state machine's `E18`
→ `A6` cell (§5.2 p. 19, §5.4.1's table p. 22, §5.3 p. 19).
`wait_for_load_state` tolerates that release while the device's last reported
state is one RES Table 94 permits silence in — but the next poll may no
longer reuse the connection, so it re-establishes first. That
re-establishment is `connect()` in full: `T_Connect` and its confirmation,
`A_Authorize_Request` — and, on a write-capable session, the Verify Mode
read-modify-write as well, which is gated on the session holding a write
authorisation and not on which `AuthorisationPlan` it uses. One quiet poll
therefore costs one whole reconnect.

**Cause.** The Standard says the connection is gone and the Standard says to
carry on polling; it does not say the polls become cheaper. RES §4.23.2.4.1
asks the MaC to *"try to re-establish the connection periodically during the
maximum transition time"*, and RES's NOTE 86 expects exactly this device:
*"A device may be offline during state LoadCompleting."* The previous
behaviour was cheaper only because it was wrong — it kept polling a
connection the peer had already forgotten, and read whatever came back as if
it meant something.

**Impact.** Latency and bus traffic, not correctness, and it compounds with
entry 101's accounting of the same wait loop. With `SessionTiming::default()`
each quiet poll is `4 × 3 s = 12 s` of unacknowledged transmissions plus the
re-establishment (up to `connection_timeout` = 6 s for the `T_Connect`
confirmation alone, more with a key), where before C19 it was the 12 s and
nothing else. With the default `max_transition` of 30 s that is room for
roughly one or two such polls before the wait gives up, not an open-ended
series — the cost is a slower failure, not a longer one. Against a device that never comes back the wait still ends in
`SessionError::TransitionTimedOut` with the last state read, as it did
before; the failed re-establishments are tolerated rather than reported,
because a single failure is not what *"periodically"* means.

**Not a regression in what is reported.** A re-establishment that fails for a
reason other than the connection failing to come up — a refused property, a
mismatched read-back, a Verify Mode the device will not take — is still
returned to the caller unchanged (`reestablishment_may_be_retried` in
`crates/knx-net/src/commissioning.rs` lists exactly the four
connection-shaped errors it swallows, and since C19's second fix round each
of the four has a test that fails when it is removed from that list).
`SessionError::Lagged` — raised when a session falls behind the event
broadcast channel it reads from (`crates/knx-net/src/commissioning.rs`) — is
the deliberate fifth case `reestablishment_may_be_retried` does not match:
falling behind a broadcast channel says nothing about whether the Transport
Layer connection itself is still there, so retrying on it would be a guess.
That exclusion is reasoned about in the match arm's shape, not tested; no
test in `commissioning.rs` constructs a `Lagged` error at all, so nothing
would fail today if a future edit folded it into the retryable set by
mistake.

**Lifted when.** A measurement on real hardware says the reconnect cost
matters. The cheaper alternative — keeping a released connection and hoping
the peer still honours it — is not available: it is the defect C19 fixed.

**Deferred to hardware, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
The lifting condition is itself a hardware measurement; taken along in the
K7 live session if the device goes quiet mid-`LoadCompleting`, otherwise
accepted as is. **K7, 2026-09-29: it did not go quiet.** No
`LoadCompleting` silence and no mid-download drop in three live downloads
(RESEARCH §19). Accepted as is.


<a id="105-transport-layer-control-frames-go-out-at-low-priority-not-system"></a>
## 105. System-priority control frames sent; on-wire priority unmeasured

Transport Layer `T_CONNECT`/`T_DISCONNECT` requests now request SYSTEM
priority and acknowledgement (Ctrl1 `0xB2`); `T_ACK`/`T_NAK` use SYSTEM
priority without the ack bit (`0xB0`). Data traffic remains unchanged.
This is spec-grounded in TL §3.7/§3.8/§5.3 and verified by
`control_frames_request_system_priority_and_data_frames_stay_low`. A live
MDT `1.1.67` accepted subsequent download sessions, but no independent
bus trace established whether the priority survived tunnelling onto TP1.
The residual is *wire-level evidence*, not the old low-priority encoder bug.

## 106. The debug report redacts four pattern classes, and nothing else

**Limitation.** The debug-report bundle (T29,
`apps/knx-server/src/debug_report.rs`) replaces exactly four things in
`report.md`, `environment.json` and `log.json`: any IPv4 dotted quad, any
IPv6 literal, the user's home-directory prefix, and the machine's hostname.
Anything else identifying that reaches those files travels with them — a MAC
address, a device serial number, a project file name sitting outside the home
directory, a hostname the machine does not report, or whatever the user types
into the description field beyond those four shapes.

**KNX addresses are not one of the four classes.** `log.json` is on by
default and can name group addresses and imported element names, because
`session_log.rs` puts `conflict.group_address`, `unknown.name` and
`unknown.sample` into its messages verbatim. That is deliberate: a debug log
stripped of the address the conflict is about cannot diagnose the conflict.
The dialog says so in the user's language — "with IP addresses removed", not
"with addresses removed" — and the privacy paragraph states plainly that KNX
addresses and project names are never replaced anywhere.

**Three knowable failure modes inside the four classes.** An IPv6 literal that
follows a word character with no separator at all (`peer2001:db8::1`) is not
redacted: the boundary rule that keeps `knx_core::Project` from being read as
a compressed address cannot tell that case from a Rust path. One separating
colon *is* handled (`peer:2001:db8::1`), two are not (`peer::2001:db8::1`
reads as a path). And the home-directory prefix is matched textually with a
word-boundary check on its right-hand side only, so `/home/knxbench-old` is
still rewritten to `~-old` when `$HOME` is `/home/knxbench` — over-redaction
that garbles a path rather than a leak, and the far more common
`/home/andrea` case is left alone.

A third is the mirror image of the second fix round's IPv4 change. The scan
now slides a four-group window across a whole run of digits and dots rather
than requiring the run to split into exactly four groups, which is what
closes a typo'd fifth octet or a glued extra group (`192.0.2.1.5`,
`5.192.0.2.1`) that used to survive intact. But a bare, unlabelled number
with five or more dot-separated parts is not distinguishable from an address
by shape alone, and a genuine version string in that shape (`1.2.3.4.5` with
nothing in front of it) is over-redacted the same way a real address would
be caught — this pass picks the side that protects the user's data. A
version string glued to a leading letter (`v1.2.3.4`) is unaffected: the
boundary rule still refuses it. This project's own version string never
takes the bare five-part shape (`0.1.0-alpha.1[+g<sha>]` has three digit
groups before the first non-digit), so the residue does not touch anything
this application prints; it would only bite a third party's version string
quoted verbatim into the report with no letter in front of it.

**Cause.** Deliberate, and scoped that way by the brief: redaction is by
pattern class rather than by a list of known values, and each class has to be
a shape that can be recognised without guessing. A dotted quad and an IPv6
literal have grammars; "an identifier that matters to this user" does not. A
broader filter would either miss things anyway or start mangling ordinary
text — the IPv6 pass already has to refuse `knx_core::Project`, which a
parser will happily read as a compressed address. Two classes also have a
knowable failure mode: `hostname()` reads `/proc/sys/kernel/hostname`,
`/etc/hostname` and then the environment, and a process where none of those
answer simply redacts one class fewer; `HOME` unset does the same for paths.

**Consequence.** The bundle is safer than an unfiltered log but is not
anonymous, and nothing in the product claims it is. The dialog names the
three redacted files, says that KNX addresses and project names survive in
all of them, and names what `bus-telegrams.json` carries: the individual and
group addresses of the installation plus the group address names the open
project knows for them ("Kitchen ceiling light"). Without those a telegram
dump says nothing, which is why they stay and why the file is opt-in. The
zip is written locally and shown to the user before anything is shared. The
GitHub path opens a prefilled issue page in the browser and stops there: no
token, no credential, no `POST` from the application, and no upload anywhere.

**Lifted when.** Nothing here is waiting on a fix. If a further class is ever
worth adding — MAC addresses are the obvious candidate — it goes in as
another shape-recognising pass next to the existing four, with the same
requirement that it name what it removes rather than silently blanking text.

## 107. There is no plugin API — a third party cannot add a format, a protocol, a report template or a UI panel without forking

**Limitation.** KNXBench loads no extensions of any kind: no dynamically
loaded library, no WebAssembly module, no script, no out-of-process plugin
protocol. A third party who wants a new import or export format, a new bus
protocol adapter, a different documentation template, or an extra panel or
command in the user interface has exactly one route — fork the repository,
add a workspace crate, and rebuild. Nothing can be dropped into a directory
and picked up at start-up.

Four things *are* extensible without compiling anything, and they are the
supported story rather than a consolation prize: **language packs** (a JSON
file, [LANGUAGE_PACKS.md](LANGUAGE_PACKS.md)), **product databases**
(imported at runtime, [ADR-0005](adr/0005-separate-product-database.md)),
**group-address CSV** ([IMPORT_EXPORT.md §11](IMPORT_EXPORT.md)) and the
headless **`knx` CLI**, which anything that can run a process can drive.
What cannot be added this way is behaviour.

`knx-server`'s `/api/*` routes are **not** a public interface either. They
exist for this application's own frontend, they change whenever it needs them
to, and they are documented nowhere as a contract. Code written against them
will break without notice.

**Cause.** Decided in [ADR-0025](adr/0025-extension-is-data-not-code.md) on
the evidence in [PLUGIN_FEASIBILITY.md](PLUGIN_FEASIBILITY.md), and the cause
is that there is nothing to expose. The whole 16-crate workspace contains
eight traits; six are single-implementer or test seams, one is private, and
the two that are dynamically dispatched say in their own doc comments that
their second implementer is a test fake. There is no importer, exporter or
template trait anywhere, and every candidate seam has exactly one
implementation — so a plugin interface would have to be generalised from a
sample of one, which is the speculative abstraction CLAUDE.md's rules
forbid. Three further facts close the route independently: all 16 manifests
are `publish = false`, so nothing can depend on `knx-core`;
`xtask check-layering` is a hand-written allowlist of crate names built from
`cargo metadata`, so a third-party crate is invisible to the one mechanical
architecture gate this project has; and `knx-core` depends on `chrono` alone,
with no `serde`, so `Command` has no serialisable form for any boundary to
carry.

Data integrity is the part that would be hardest to fix rather than merely
tedious. `Project`'s six fields are all `pub`
(`crates/knx-core/src/project.rs:181-186`), so the rule that every mutation goes
through `Command::apply` — with its validation, its typed errors and its
inverse for undo — is held by review, not by the type system. `Layer`
([ADR-0004](adr/0004-provenance-model.md)) has five variants and none of them
means "a plugin did this", so a plugin write would either forge `UserEdit`
provenance, telling the user they made a change they did not, or force a
sixth variant through the persistence schema and its migration chain. And a
plugin that rewrote a project without carrying the opaque passthrough store
([ADR-0006](adr/0006-opaque-passthrough-store.md)) would break roundtrip
fidelity **silently**, because no validator can catch an absence.

The AGPL licence is deliberately *not* the cause. Copyleft hosts sustain
large third-party ecosystems elsewhere; what AGPL rules out is *proprietary*
in-process addons, and even that with decreasing certainty as the boundary
moves from a dynamically linked library to WebAssembly to a separate process.
See PLUGIN_FEASIBILITY.md §3, which states the licence mechanics and is
explicit that it is not legal advice.

**Consequence.** An extender needs a Rust toolchain, a build, and the
discipline of rebasing onto upstream. That is a real barrier and this entry
does not pretend otherwise. There is also no answer at all for someone who
wants a closed-source addon — though under the licence analysis above that
answer would have been "no" or "unsettled" for every in-process mechanism
anyway, so less is foreclosed than it looks. In exchange, nothing a third
party ships can corrupt a project file, crash the application, forge
provenance, or silently lose the data the opaque store exists to preserve.

**Lifted when.** Four conditions, all of which are worth reaching for other
reasons: the `Command` layer is complete and has a serialisable form — the
same blocker [ROADMAP.md](ROADMAP.md) already records for MCP capabilities
and for the in-app LLM surface; §22 (no authentication on `knx-server`) is
answered; §63 (one shared project, one shared undo stack, no conflict
detection) is answered; and at least two concrete third-party extensions
exist that the four data surfaces above genuinely cannot express. If a code
seam is then wanted, ADR-0025's recommended shape is an out-of-process helper
over a documented protocol. Before any of that, the cheap falsifying test is
to write a *second* implementation of one seam as an ordinary workspace crate
and see whether a shared trait falls out of it — if one does, this entry and
its ADR are wrong and should be revised.


## 108. MP §2.3 contradicts itself about an occupied IA_new, and this project follows the exception text

**Limitation.** `NM_IndividualAddress_Write`'s own body text and its own
exception-handling paragraph disagree about what happens when the address
being assigned already answers on the bus, and `individual_address_write()`
in `crates/knx-core/src/commissioning/procedure.rs` picks a side rather than
implementing a step the clause itself does not resolve.

**Cause.** `[D]` MP §2.3, p. 14, step 1's body text: *"if A_Disconnect-PDU is
received then IA_new shall be regarded as occupied; end procedure."* That
reads as unconditional: any Disconnect in place of a Device Descriptor
response stops the write. But `[D]` the same clause's own exception
handling, p. 15, *"to 1.:"*, describes exactly that situation — *"If an
A_Disconnect-PDU is received instead of an A_DeviceDescriptor_Response-PDU
… The Management Client shall continue with the Management Procedure in
every case."* — and flatly contradicts the body text it is annotating.

The same paragraph's *"to 2.:"* exception then narrows occupancy generally,
not just the Disconnect case: an answer at IA_new only stops the procedure
if it comes from a device other than the one step 2 finds in Programming
Mode. *"[D]"* p. 15: *"A device with the Individual Address IA_new to be
assigned exists, and it is the one that is in Programming Mode. ⇒ The
Management Client shall continue with the Management Procedure."* That is
exactly the case step 3's own guard exists for — *"set Individual Address
if IA_new != IA_current"* (p. 15) is meaningless if step 1 already stopped
the procedure on the grounds that IA_new answered at all, which is what a
device being re-programmed to its own current address always does.

**Ruling.** This project implements the exception text over the body text:
it is the more specific statement, it is the later one in reading order,
and it is the only reading under which the re-assignment guard in step 3
can ever be reached. Occupancy detected via `A_Disconnect-PDU`, or via a
`A_DeviceDescriptor_Response-PDU` from a device other than the one in
Programming Mode, is surfaced to the operator as a finding rather than
enforced as a silent stop; the stricter body-text reading would refuse a
legal re-programming that the exception text explicitly allows.

**Impact.** A stricter reading of MP §2.3 would refuse to continue past
step 1 whenever *anything* answers at IA_new, including the device already
being re-programmed to its current address. This project's reading instead
lets that case through and reports the ambiguous ones (Disconnect, or an
occupant that has not yet been identified against Programming Mode) as
findings. Cost if this ruling is wrong: a re-programming attempt proceeds
where a stricter reading would have refused it outright — recoverable
(nothing here writes before step 3's own guard), and visible to the
operator via the reported finding, not silently swallowed.

**Lifted when.** Nothing here is waiting on a fix; the clause itself is
what disagrees with itself. This entry closes if a future edition of MP
§2.3 removes the contradiction, or if `docs/RESEARCH.md`'s knowledge-base
audit turns up an erratum for v02.01.02 AS that resolves it.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
The contradiction is in the clause itself; this project keeps following the
exception text, and a later edition or erratum reopens it.

## 109. Two loadable parts of the same `PartKind` have no defined relative order, so `DownloadPlan::new` refuses them both

**Limitation.** `[C8]` `DownloadPlan::new`
(`crates/knx-net/src/commissioning/download.rs`) now enforces the download
order CP §3.5.2 Nr. 06-10 and CP §3.5.3 AP2 Nr. 08-12 both carry by row
position — Application Program 2, Application Program 1, the Group Object
Table, the Group Address Table, then the Association Table — by requiring
each part's `PartKind` to be strictly greater than the part immediately
before it (`PartKind`'s `Ord`, not `>=`). That means a plan with two parts
of the *same* kind — two Association Tables at two different object
indices, say — is refused with `PlanError::OutOfOrder`, reporting the
second as following an equal, not a lesser, predecessor. Neither CP §3.5.2
nor CP §3.5.3 lists more than one row per kind, so there is no row position
to place a second instance against; a plan like that falls outside what
either table describes at all, not merely outside the order they give.

**Cause.** The two normative tables assume exactly one Interface Object of
each of the five kinds per device, which is the ordinary case RES's Object
Index scheme was built around. Whether the Standard permits a device with,
for instance, two Association Table objects — and if so, in what order a
Management Client would download them — was not found stated anywhere in
either knowledge base consulted for this task (the programming-focused and
the full 179-document corpus). Refusing rather than guessing an order is
this project's own inference, made in the direction CLAUDE.md's priority
order requires (Correctness and Data Integrity ahead of Compatibility): a
wrong guess here would feed `Downloader::partial_download`'s escalation
slice (`parts[position..]`) the same silently-wrong target set this task
exists to prevent.

**Impact.** None observed against the simulator or any product data this
project has imported: every fixture and every ETS project seen so far
carries at most one Interface Object per `PartKind`. A device or a product
database entry that genuinely needs two objects of the same kind in one
download plan cannot be represented today; `DownloadPlan::new` refuses it
outright rather than downloading it in an arbitrary or caller-supplied
order.

**Lifted when.** Spec text (a clause, a table row, or a confirmed erratum)
states a relative order for two same-kind objects, or a real product
database entry is found that requires more than one object of the same
`PartKind` in a single download — at which point the order for that case
can be added deliberately, cited, and tested, rather than inferred here.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
Refusing both parts stays the behaviour; no order is invented. A cited order
or a real product that needs it reopens this.

## 110. `PID_GROUP_RESPONSER_TABLE` stays unimplemented on every medium

**Limitation.** `[C11]` CP §3.5.3's Application Program 2, Application
Program 1 and Group Object Table variants each carry a step that writes the
Group Address Table segment and, within it, a group responser table via
`PID_GROUP_RESPONSER_TABLE` — footnoted in each case as PL110-only (CP
§3.5.3 footnotes 8, 9 and 10, pp. 47, 50, 52). RES §4.16.8.2.5, p. 239 is
explicit both ways: *"This Property is mandatory for PL110 devices. For all
other media this Property shall not be implemented."* This project targets
TP1, RF and IP; it does not implement `PID_GROUP_RESPONSER_TABLE`, and the
three affected step lists in `crates/knx-core/src/commissioning/
partial_download_variant.rs` say so inline rather than modelling the
property's write as if it applied everywhere.

**Cause.** Deliberate scope decision, not a gap found by accident. RES's own
text makes implementing this property on TP1/RF/IP a Standard violation, not
merely unnecessary, so "not yet done" would be the wrong description.

**Impact.** None on TP1, RF or IP — the medium this project's simulator and
every product fixture seen so far use. A plan that targets a genuine PL110
device must be *declined* outright rather than run with this half of CP
§3.5.3 AP2 Nr. 11 (and its GOT/AP1 siblings) silently missing: an operator
who thinks a PL110 download finished would otherwise be wrong about a
Standard-mandated property no code here ever touched. No such decline
exists in the sequencer yet, because no PL110 support exists yet for it to
guard; this entry is the refusal recorded ahead of the code, per CLAUDE.md's
"never silently discard information."

**Lifted when.** PL110 support is scoped as its own task, at which point
`PID_GROUP_RESPONSER_TABLE` gets a real implementation and this entry
becomes a completed cross-reference instead of a limitation, or a plan
targeting a PL110 device grows an explicit decline in the sequencer and this
entry's "no decline exists yet" clause is struck.

## 111. CP §3.5.4 step 07 (unload the individual address) stays unimplemented

**Limitation.** `[C11]` CP §3.5.4's Individual Address unload procedure has
a step that unloads the individual address itself, which — per its own
mechanism, a broadcast write via the device's `PID_LOAD_STATE_CONTROL` — is
the one write that removes the property a Management Client would need to
address that same device again afterward. This project does not implement
that step: nothing in `crates/knx-core/src/commissioning` or
`crates/knx-net/src/commissioning` issues it, and none of the five variants
this task adds walks it either.

**Cause.** Deliberate refusal, not an oversight. Making a device
unaddressable by the tool that is supposed to be commissioning it is not a
failure mode this application accepts as a side effect of a scripted
procedure — task C6 is what makes the *name* of "unload" honest about the
other four parts it does cover, rather than silently implying a fifth.

**Impact.** None on the parts this project unloads (individual address
excluded). A caller asking for CP §3.5.4's full procedure by that name would
get the four parts other than the individual address; there is no code path
that would remove a device's address without an operator taking a
separate, explicit action outside this procedure.

**Lifted when.** A concrete, reviewed use case needs this project to make a
device unaddressable on purpose (a factory-reset-style workflow, say), at
which point step 07 gets its own guarded implementation, cited against CP
§3.5.4, rather than riding in as one more step of a procedure named for
something else.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
No use case for making a device unaddressable exists; step 07 stays
unimplemented until one does.

## 112. A download plan that needs `A_Key_Write` is refused outright, not carried out

**Limitation.** CP §3.5.2 Nr. 11, p. 44, and CP §3.5.3 AP2 Nr. 13, p. 47,
both read *"Set access keys as required"* — a step this project's download
sequencer now models honestly with `AccessKeyDeclaration`
(`crates/knx-core/src/commissioning/authorisation.rs`): a plan declares
either `NoneRequired` or `Required(Vec<AccessKeyAssignment>)`. A plan
declaring `NoneRequired` completes and the step is reported empty, which is
correct. A plan declaring `Required` is refused with
`DownloadError::AccessKeysNotSupported` at the step, because
`A_Key_Write` has no encoder: `crates/knx-net/src/cemi.rs`'s
`key_write_has_an_apci_but_no_encoder` test documents exactly this —
the APCI constant exists, the frame variant does not.

**Cause.** `A_Key_Write` was out of scope for commissioning phase 2 (design
spec §10.7), and encoding it needs its own frame-format decision (the
payload shape, and how a "delete" key — `FFFFFFFFh` — is represented,
since `AccessKey::new` deliberately rejects that value as the free-access
sentinel rather than a key). Building that encoder was not this task's job;
this task's job was to stop a plan that needs it from silently completing
without it, which is what the refusal now does.

**Impact.** Every download this project can currently run must declare
`AccessKeyDeclaration::NoneRequired`, i.e. must leave every access level at
its existing key. A commissioning workflow that also wants to set or
change a device's key as part of the same download cannot do so through
this sequencer today; the operator must do that separately, by whatever
means already exists outside this application, or wait for `A_Key_Write`
to be built. No device is left on a key the operator believes was changed:
the whole download is refused rather than partially honoured.

**Lifted when.** `A_Key_Write` gets an encoder in `cemi.rs` and
`ManagementSession` gains a way to send it — at which point
`modify_access_keys` (`crates/knx-net/src/commissioning/download.rs`) can
carry out a `Required` declaration instead of refusing it.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
A plan that needs `A_Key_Write` keeps being refused by name; no access key
is ever guessed. A project that needs keys (together with KNX Secure, §8)
reopens this.

## 113. An escalation only reloads the segments a shortened plan actually carries

**Limitation.** `[C12]` `Downloader::partial_download`
(`crates/knx-net/src/commissioning/download.rs`) now cites the correct CP
§3.5.3 step number for every escalated reload (`PartialDownloadVariant`,
`[C11]`), including one a shortened plan happens not to carry: a Group
Address Table reload in an Application Program 2 escalation is always
numbered Nr. 11, whether or not Application Program 1 or the Group Object
Table are also in this plan. What it does not do is reload a segment the
plan never listed at all. CP §3.5.3 AP2 Nr. 07, p. 46 — *"unload all the
following segments"* — reads as if it always means all four, because the
clause assumes a device with one Interface Object of each kind and a
download that touches all of them; this project's `DownloadPlan` may
legitimately carry fewer (`[C8]`, entry 109), and the escalation only ever
walks `self.plan.parts[position..]` — the segments *this plan* was told
about, in order, not a fixed set of four names.

**Cause.** A partial download's plan is built by the caller for the one
part being replaced (and whatever it chooses to also carry for a possible
escalation); this project has no independent source of "every segment this
device actually has" to reload one the caller never mentioned, and
reloading data the plan does not carry would mean writing something no
part of this call ever validated. Between under-reloading and inventing a
payload to write, this project reports the smaller, honest set.

**Impact.** A partial-download plan that carries only the target part (the
common case in the test suite and, so far, in every fixture built from
product data) never escalates at all in practice beyond the target's own
retry — there is nothing after `position` to reload. A caller that wants
the full CP §3.5.3 escalation behaviour must build the plan with every
segment after the target already in it, in download order; if it leaves
one out, that segment's reload — and its step number in the trace — simply
does not happen, silently, from this module's point of view (the caller
made the choice; this module cannot tell a deliberate omission from an
oversight).

**Lifted when.** A caller-facing planner exists that always fills a
partial-download plan with every segment CP §3.5.3 says an escalation may
need, sourced from the device's actual Interface Object list rather than
left to each call site to remember.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
Partial downloads are not offered by any surface (the v1 path, K9, is a full
application download), so the shortened-plan case has no caller; it reopens
with the first partial-download feature.

## 114. The Download Counter refusal is this project's own conservative rule, not a System B obligation

**Limitation.** `[C13]` `Downloader::partial_download`
(`crates/knx-net/src/commissioning/download.rs`) now reads
`PID_DOWNLOAD_COUNTER` from the Device Object before its first write and
refuses the partial download — returning `DownloadError::
DownloadCounterChanged` or `DownloadError::DownloadCounterUnavailable`
rather than proceeding — for two of `DownloadCounterCheck`'s five outcomes.
**Both consequences are Coupler Model 2.0's own, not System B's.**
CP §3.12.5, p. 100 (refusal on change) is written for Coupler Model 2.0's
own download procedure (CP §3.12.4, Filter Table and Router Object,
neither of which this project implements as a distinct procedure).
RES §5.3.2.2, p. 320 (refusal on absence) sits in RES §5.3 *"Resources for
Coupler Model 2.0"*, not System B — a full-text search of RES for *"shall
not perform a Partial Download"* finds this one occurrence, nowhere else.
**There is no System B clause requiring either refusal**; an earlier
revision of this entry and of `docs/IMPLEMENTATION_STATUS.md` said
otherwise and was wrong, corrected in review round 1. This module applies
both Coupler-Model-2.0-native consequences to the one generic CP §3.5.3
five-variant procedure it actually runs, for every part kind, rather than
building a second, Coupler-Model-2.0-specific download procedure that
CP §3.12 alone would otherwise call for — and, for the profiles this
project actually targets (System B in practice), that is now this
project's own conservative ruling, not the Standard's.

**Why keep it anyway.** Without a comparable Download Counter, the MaC
cannot establish that the device is untouched since the last
configuration it performed. CP §3.12.5's and RES §5.3.2.2's answer to that
uncertainty — refuse the partial download, run a complete one instead —
is the safer of the two available answers, and this project ranks data
integrity above the convenience of a partial download. That ruling does
not change; only its citation does.

**The practical consequence.** `[C18]` established that PID 30 is
unlisted — hence optional — for the S-Mode End-device Device Object,
Volume 6 Annex A A.2.3, pp. 138-140, which covers System B's masks 07B0h
and 17B0h along with every other S-Mode End-device system. **A conformant
System B device with no Download Counter will be refused every partial
download it is ever asked for, and will always get a complete one
instead.** That is not a bug in this project or a defect in the device —
it is what applying a Coupler Model 2.0 rule to a System B device,
conservatively, on purpose, actually does to it.

**A second simplification, same direction.** RES §5.3.2.2, p. 320 refuses
on absence *"of the part to be downloaded"* — per part, not per device —
and RES §4.2.30.1, p. 41 allows a downloadable part its own Download
Counter instance distinct from the Device Object's, or none at all. This
module's *absent* check reads only the Device Object's instance
(`OI = 0`), the same instance CP §3.12.4/3.12.5 read for the *changed*
check (which RES §4.2.30.3, p. 42 backs: an unchanged Device Object
instance lets the client conclude no other instance changed either).
Nothing backs doing the same simplification for *absence*. It is recorded
here rather than fixed by a per-part-instance lookup, because it errs
toward refusing more often — the safe direction — and because per-part
instance mapping (which object index owns which part's Download Counter)
is a larger change than this task's scope.

**One more case, deliberately not a refusal.** `DownloadCounterCheck::
NoStoredCounter` — this MaC never called `DownloadPlan::
with_stored_download_counter` for this plan — proceeds rather than
refusing. CP §3.12.5's comparison needs a value to compare against; with
none stored, there is nothing to have changed from, so nothing in either
clause is violated by proceeding. It is a gap in this application's own
bookkeeping, not a fourth Standard outcome, and treating it as a refusal
would invent an obligation neither clause states.

**Cause.** Nothing in this codebase distinguishes a System B device from a
Coupler Model 2.0 device at plan-building time (masks 07B0h/17B0h vs.
2920h, `[C13]`'s own research), and CP §3.5.3 itself, the clause this
sequencer transcribes step-for-step, "imposes nothing" about the Download
Counter at all — the obligation is bolted on from two clauses written for
a different procedure than the one this project actually runs. Building a
genuine second procedure for Filter Table/Router Object downloads was not
this task's job; refusing an unsafe partial download was.

**Impact.** A target this project cannot yet tell is Coupler Model 2.0
gets Coupler Model 2.0's own refusal behaviour applied to it regardless —
which for a System B device with no Download Counter (the common,
conformant case, per `[C18]`) means partial download is unavailable in
practice. Nothing here "falls back": `partial_download` returns `Err` and
this project has no caller that automatically retries as a complete
download — the caller must run one itself. A real Coupler Model 2.0
Filter Table download (CP §3.12.4's own steps, which this project has
never implemented) is also not being run under the name CP §3.12 gives it.

**Lifted when.** A device-profile model exists that can tell a Coupler
Model 2.0 target from a System B one at plan-building time, so the
Coupler Model 2.0 consequences apply only where CP §3.12 actually asks
for them; and CP §3.12.4's Filter Table/Router Object procedure is
implemented as its own `Procedure`, distinct from the CP §3.5.3 one this
module runs today. Per-part Download Counter instance mapping, for the
second simplification above, can be lifted independently of either.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
The conservative Download Counter refusal stays; a device-profile model that
can tell Coupler Model 2.0 from System B reopens it.

## 115. `MasterResetResponse::recovery_wait` and `SessionTiming::restart_basic_t1` compute durations nobody waits on yet

**Current status, 2026-10-03.** The blanket title and original account below
are historical, not a description of current callers. The typed
`master_reset()` procedure in `commissioning/master_reset.rs` waits on
`probe.recovery_wait(&timing)` and the erasing answer's recovery wait, then
performs a bounded mask-read retry at the expected address. The
individual-address procedure also uses `restart_basic_t1` for its documented
reconnect wait. These are caller-specific implementations; they do not add a
generic failed-service restart/retry contract or authorize erasing hardware.
The public reset recovery gate remains in force. Retain this heading/anchor
for historical inbound links; the original "no reader" sentences are
superseded by this status.

**Limitation.** `[C15]` `ManagementSession::restart_master_reset`
(`crates/knx-net/src/commissioning.rs`) sends the confirmed Master Reset
and decodes the device's `A_Restart_Response` into a `MasterResetResponse`,
whose `recovery_wait` method correctly computes MP §3.7.1.2.2, p. 81's
floor — the greater of the reported Process Time and this session's own
`SessionTiming::restart_responsive_again` — because *"the process time is
thus a minimal time for the MaC to wait, not a maximal time."* Its sibling
field `SessionTiming::restart_basic_t1` (MP §3.7.1.1.2, p. 79, Figure 19)
is the same shape of value for the *other* restart type: the earliest
point at which a caller may expect a Basic Restart to have already
succeeded. Both are documented on the methods that produce or return them
and neither is read by any method's own logic — `restart_basic` and
`restart_master_reset` return as soon as `disconnect_after_restart`'s
mandatory wait elapses, without consulting either value.

Nothing in this crate calls `recovery_wait` and then waits, and nothing
computes or waits out `restart_basic_t1` either. Both are missing pieces
of the same absent behaviour: MP §3.7.1.2.2, p. 81's next clause — *"call
the failed service one last time"* after the Process Time expires, before
declaring the Configuration Procedure failed — is an **obligation this
project has not implemented**, not merely a timing value kept in reserve.
There is, at the time of writing, no Configuration Procedure in this
project that runs a service, hits a failure, restarts the device to
recover, and then needs to retry that same service once the device is
responsive again; MP §2.3's own restart step (step 4) restarts the device
as the *last* thing a procedure does, with nothing afterwards to retry.

**Cause.** Building a generic "wait, then call this one more time" retry
combinator with no real caller to attach it to would be exactly the kind
of speculative abstraction this project's own engineering rules warn
against, and an untested combinator is also exactly the kind of
unverifiable behavioural claim this commissioning plan keeps finding and
removing elsewhere. `recovery_wait` and `restart_basic_t1` were written as
small, directly tested pure values instead, so that the arithmetic MP
§3.7.1.2.2 and §3.7.1.1.2 require is proven correct today, and wiring
either to an actual wait-and-retry becomes a caller's problem once a
caller exists.

**Impact.** A future Configuration Procedure that restarts a device via
Basic Restart or Master Reset mid-procedure and needs to resume afterwards
must itself wait out `restart_basic_t1` or `recovery_wait` and perform the
"one last time" retry MP §3.7.1.2.2 requires — none of that happens
automatically today, and no code path in this crate implements that retry
obligation at all. The timing values themselves are correct and tested
(`recovery_wait_never_goes_below_the_configured_floor`,
`default_restart_timings_match_mp_section_3_7_verbatim`); only the
waiting and the retry are absent.

**Updated, 2026-09-27.** `restart_basic_t1` now has one reader:
`individual_address_write`'s step 4 waits it out once before retrying a
`T_Connect` to a freshly addressed device (§7, "Three limitations this
first write exposed", item 1). That use is a **borrowing**, not the MP
§3.7.1.1.2 obligation this section describes — the retry happens *before*
the restart, not after it — so the "call the failed service one last time
after a restart" behaviour is still unimplemented, and `recovery_wait`
still has no reader.

**Lifted when.** A Configuration Procedure exists in this project whose
recovery from a Basic Restart or Master Reset needs more than "restart,
then let a fresh session reconnect" — at which point that procedure's own
retry loop, not a speculative one built ahead of it, waits out the
relevant timing value and implements MP §3.7.1.2.2's "one last time" retry.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
The live K2 traces showed the fresh-session reconnect is enough on the v1
device (about 38 s unreachable, then readable); no procedure needs the
computed waits yet.

## 116. `NM_IndividualAddress_Write` does not loop for the operator, and reads one Transport Layer release as MP §2.3 never quite says

**Limitation.** `[C16]` `individual_address_write()`
(`crates/knx-net/src/commissioning/individual_address_write.rs`) diverges
from MP §2.3 in two places, both narrow, both deliberate.

*Step 2 does not repeat.* The Standard's step 2, p. 14, reads *"2. wait
until device is in Programming Mode: repeat until one
A_IndividualAddress_Response-PDU is received … end repeat"*, and p. 13
states the same obligation in prose: *"The procedure shall wait until
exactly one device is in Programming Mode."* This implementation
broadcasts once. On nobody, or on several, it returns
`IndividualAddressWriteError::Count` to its caller rather than
re-broadcasting until the count comes right.

*A released connection is read as an `A_Disconnect-PDU`.* p. 14 rules that
*"if A_Disconnect-PDU is received then IA_new shall be regarded as
occupied"*, and p. 15's "to 1." explains the two devices that behave that
way. Step 1 reports `Occupancy::OccupiedAfterDisconnect` for that case —
and also when this client's own Transport Layer releases the connection
because nothing acknowledged four transmissions (TL §5.4.1, p. 22,
transition `E18` in `OPEN_WAIT`, action `A6`). No `A_Disconnect-PDU` was
received in that second case; the local Transport Layer synthesised the
indication.

**Cause.** What step 2's `repeat` waits for is a human walking to a device
and pressing a button. A library function cannot wait for that on the
thread that called it, and this project builds no procedure-level retry
loops — the Standard's own General Exception handling, MP §3.1, p. 68, is
*"In general if an error is detected, the download shall be interrupted
and an error-message shall be raised."*, which is the opposite of a retry
obligation. Returning the count to the operator, who is the one who has to
go and press the button, puts the loop where the only actor that can close
it lives. The Standard's own footnote 2) on p. 15 points the same way:
*"The user of the Management Client should get an information in how many
devices are Programming Mode is active (none or more than one)."*

The occupancy reading has a smaller cause: MP §2.3 enumerates three
outcomes for step 1 — a response, a received `A_Disconnect-PDU`, and
silence until the time-out — and a device that acknowledges nothing at all
fits none of them exactly. It is not the silence arm, because that arm's
*"If no A_DeviceDescriptor_Response-PDU is received after time-out"*
describes a connection that is still open when the time-out expires, which
this one is not. Of the two remaining arms, "occupied" is the one that
does not risk handing `IA_new` to a second device.

**Impact.** Neither divergence changes a stop/continue decision the
Standard specifies. The absent `repeat` makes a zero-or-several count an
error the caller sees instead of a wait the caller cannot see; a caller
that wants the Standard's behaviour calls the function again, and any UI
that drives this has to tell the operator what to do anyway. The occupancy
reading affects only which of two occupied labels a report carries, since
`KNOWN_LIMITATIONS.md` §108's stop/continue comparison is on addresses and
not on labels — an unacknowledged `IA_new` that turns out to be the
Programming Mode device itself still continues, and one that turns out to
be somebody else still stops.

**Lifted when.** The `repeat` is lifted by whatever owns the operator
dialogue — a UI loop that re-runs step 2 while showing "press the
programming button on exactly one device" implements the Standard's wait
in the only place it can be implemented, and this function stays the
single-shot primitive underneath it.

**Status 2026-09-28 (K6): the `repeat` half is lifted for the command line,
simulator-verified.** `knx_net::commissioning::programming_button_wait`
repeats the broadcast count (one 1 s window per round, a 1 s pause between
rounds) until exactly one device answers, the operator stops it, or `--wait`
runs out. Every round goes to an observer; `knx device program-address`
prints the count and "press" / "release all but one" whenever it changes
(MP §2.3 footnote 2). The wait runs before step 1 rather than between steps
1 and 2. That changes no decision: step 1 only reads, and
`individual_address_write` still does its own step 2 count and its re-count
before the write. A button released after the wait is caught there (test
`a_button_released_after_the_wait_is_caught_by_step_two`).
`individual_address_write` itself is unchanged, and still single-shot.
**UI half, same day:** the web's **Program address** tab and
`/api/device-address/*` (ADR-0046) run the same loop, show the prompt
whenever the count changes, and can stop the wait (never the procedure).
**Live, 2026-09-29 (K6 [W]):** `1.1.67` → `1.1.68` on a real MDT push
button. The user pressed the button in round 37. Steps 1–3 ran, step 4
connected to `1.1.68` and read the device there, and only the closing Basic
Restart went unacknowledged, as on every restart of this device (RESEARCH
§19). The procedure reported that as a step 4 failure, *"did not answer at
the new address"*, which was wrong. A read-only scan afterwards found
`1.1.68` occupied and `1.1.67` vacant, and the programming LED was off, so
the restart had happened. **Fixed the same day:** step 4's restart silence
is now `AddressRestart::Unconfirmed` inside a successful report, as in the
download path. Only a restart refused before sending is still a step 4
failure. The CLI prints `restart: NOT confirmed`, and the server's
`finished` status carries `restartConfirmed: false`; the web panel does not
show that field yet (web lock held by the UI session). Verified live on the
fixed build: `1.1.68` → `1.1.67` ended `finished`, exit 0, restart NOT
confirmed, and the device answered at `1.1.67` afterwards (RESEARCH §19). The
occupancy-reading half of this entry is unchanged. The occupancy reading is revisited if
`docs/RESEARCH.md`'s knowledge-base audit turns up spec text or an erratum
that rules on a Transport Layer release at step 1.

**Current availability (2026-10-01, ADR-0059).** The historical live K6
round trip does not establish complete durable recovery for a later
button-selected address write. Public confirmed CLI/HTTP starts now fail
before opening a tunnel, pending verified device-specific affected storage,
durable pre-send backup/readback, abort plan and a fresh go. A diagnostic
dump of selected memory or a property-only backup is not proof of that
coverage. The Web tab uses the read-only availability endpoint to disable
Program before consent on blocked/unknown status and again after a 412;
the server POST remains the independent fail-closed authority. Simulated
protocol exercises do not reopen the production write path (RESEARCH §24).

**Not a limitation any more.** An earlier draft of this section claimed MP
§2.3 *"does not consider a `T_Connect` refusal, or a connection that opens
and then answers nothing at all"*. The clause considers both, on p. 14,
and rules the opposite way from the code that was written against that
claim: *"if negative A_Connect.Lcon ⇒ IA_new is not occupied"* and *"If no
A_DeviceDescriptor_Response-PDU is received after time-out ⇒ IA_new is not
occupied"*. Both now follow the text, and the two `Occupancy` variants
invented to hold them are gone.

## 121. Two open windows do not see each other's preference changes until one reloads

**2026-10-02 UI-owner implementation update.** The historical reload-only
description below is superseded at frontend scope: authenticated main and
companion windows now reread the authoritative settings record every five
seconds while visible and on focus/visibility. Reads never write it back.
Queued/failed local patches, unknown keys and deletion intent are preserved;
write-generation and cancellation guards prevent stale reads from overriding
new edits or reviving a stopped timer. Focused regressions, realistic negative
controls and the complete offline gates pass. There is still no instantaneous
push or general shared-project synchronization; native/AT workflows are not
accepted by these tests. Publication remains pending. See
[UI_ALPHA_READINESS](UI_ALPHA_READINESS.md). The historical title and inventory
identity remain stable for incoming links and the separate alpha owner.

**Historical description before this follow-up:**

**Limitation.** Since [ADR-0029](adr/0029-application-settings-file.md),
preferences live in one `settings.json` in the server's data directory and
every window reads it at load. A window that changes a preference writes it
to the file; a second window already open — a browser tab, the diagnostics
companion, the desktop shell next to a browser — keeps showing the value it
read when it loaded, until it reloads or something else makes it re-read.
Changing the theme in one window does not repaint the other.

**Cause.** There is no push channel from the server for this, and no
polling. `apps/knx-web/src/settingsStore.ts` reads the record once, from
`initSettings()`, and every later change it hears about is one it made
itself. The one thing that *is* protected is key loss: `PUT /api/settings`
is a patch and `AppState::settings_lock` serializes the read-modify-write,
so the second window's next write cannot flatten the first window's change
back to what it last read. Only the second window's *view* goes stale, and
only of a key the other window touched.

**Impact.** Small and self-correcting. The `localStorage` arrangement this
replaced had the same staleness within one browser (a `storage` event would
have fixed it and none was listened for) and a worse version across front
ends, where the two never agreed at all. The window that is stale is by
definition not the one the user is changing preferences in.

**Lifted when.** There is a reason to add a notification channel. A
`storage` event on the cache key would fix the same-browser case cheaply and
would still leave the browser-versus-desktop case open, which is the case
worth solving; both wait for a server-side change feed, which nothing else
needs yet.

## 124. The interface search shows four facts about an interface; the protocol carries more

**2026-10-02 UI-owner implementation update.** The narrowing described below
has been removed: `DiscoveredGateway` retains optional decoded Device Info and
the HTTP/UI layers project its medium/status octets, project-installation ID,
serial, routing multicast and MAC without reinterpretation. Missing adapter
metadata is explicitly unavailable, never fabricated zeros. Focused HTTP and
actual unicast-loopback tests, raw-value UI regressions, realistic mutants and
the complete offline gates pass; six additional intercepted-browser cases
verify the disclosure itself. This does not accept native/live Search (§79),
infer device identity/capabilities or grant a write. Publication is pending.
See [UI_ALPHA_READINESS](UI_ALPHA_READINESS.md). The historical title/inventory
identity remains stable; the following cause is historical, not current code.

**Historical description before this follow-up:**

**Limitation.** `POST /api/bus/discover` (`apps/knx-server/src/bus_routes.rs`)
and the panel above it report four things per interface: control endpoint,
individual address, friendly name, and whether tunnelling is among the
service families it advertises. A `SEARCH_RESPONSE`'s Device Info DIB
carries more than that — KNX medium, device status (programming mode), the
project-installation identifier, the KNX serial number, the routing
multicast address, and the MAC address — and `knx-net` decodes every one of
them into `knx_net::core::dib::DeviceInfo`. None of them reach the user.

**Cause.** `KnxNetIpClient::discover()` narrows `DeviceInfo` to the
four-field `DiscoveredGateway` before it returns, so the HTTP route never
sees the rest. This is not a loss the route could avoid by mapping more
carefully: the fields are gone one layer below it. Widening
`DiscoveredGateway` is a change to the protocol crate's public type, which
T25 deliberately did not make — its own boundary was that the search must
not touch `knx-net`'s encoding or decoding.

**Impact.** Small but real, and it grows with the size of the installation.
Two interfaces from the same manufacturer with the same default friendly
name are told apart today only by their addresses; the serial number is the
fact that would distinguish them, and it was decoded and dropped. An
interface sitting in programming mode is likewise invisible here, though
the byte that says so arrived. Nothing is silently wrong — the four
reported fields are accurate — the extra facts are simply not offered.

**Lifted when.** `DiscoveredGateway` carries the `DeviceInfo` it was built
from (or the fields worth keeping), the route maps them, and the panel
shows the ones a user can act on. A day's work in the protocol crate, its
tests, and one DTO — worth doing on the day someone has two identical
interfaces on one network and no way to tell which is which.

**Related.** [§79](#79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry)
is the other half of what discovery cannot promise: an empty result is a
result, and on a container without host networking it is the *only*
possible result. The UI says so in its own words rather than presenting an
empty list as a verdict about the installation.
## 125. ETS 6's device-local communication-object ids are read from a single project's evidence

**AR06 scoped delivery, 2026-10-02 16:59 CEST.** Published 95e6bcb0 after integrated
24-step receipt and documentation-only upstream reconciliation; 2,916 Rust,
1,357 Web and 52 intercepted-browser passes, eighteen selected private/six raw
cases without skips. This supersedes historical pending-delivery wording below,
not the independent-sample lifting condition or any compatibility boundary.

**AR06 scoped checkpoint, 2026-10-02 14:34 CEST.** New synthetic regressions
distinguish repeated RefIds by owning device, preserve instance overrides/group
links and report malformed refs without losing later objects. Existing real
object/enrichment tests and the exact schema-21 empty DefaultLine diagnostic
passed explicitly; native synthetic reopen covers 24 combinations, with/without
an empty product DB. Six compiled negative controls were caught and the mapper
was restored byte-exactly. This adds regression evidence, not a second
independent schema-23 sample or new mapper semantics. The raw-field/sample
matrix in [IMPORT_BOUNDARY_CONTRACT.md](IMPORT_BOUNDARY_CONTRACT.md) distinguishes
the two documented installations and records that invalid DefaultLine tokens
are transient diagnostics, not newly durable native source fields. This
limitation and its independent-sample lifting condition remain in force;
whole-feature/integrated delivery is still pending.

**What changed first.** Until T29 the ETS 6.3.0 reference project (schema 23)
reported **867** `MapProblem::Value(MalformedRefId(..))` over **310** distinct
ids, and mapped every one of its 867 communication objects to object number
`0`. The cause was a shape, not corruption: every id in that project's
`GroupObjectTree/@GroupObjectInstances` is the two-segment, device-local form
`O-<n>_R-<m>`, while `values.rs::split_object_tail` required the
three-segment, fully-qualified schema-11 form
`<program>_O-<n>_R-<m>` **[V]**. Both halves of the pipeline now read the
device-local form — `values::device_local_com_object_number` on the import
side and `knx_productdb::com_object_lookup_id` on the enrichment side — so
the 867 map errors are gone and all 867 objects enrich, up from 0.

**The limitation.** That rule was measured against **one** schema-23 project.
The measurement itself is exhaustive over that project and left nothing
uncounted: prefixing each of the 867 ids with its own device's resolved
application program (`DeviceInstance/@Hardware2ProgramRefId` →
`M-<n>/Hardware.xml`'s `Hardware2Program` → `ApplicationProgramRef/@RefId`)
names an existing `ComObjectRef/@Id` **867 times out of 867**, never more than
one candidate, and the `ComObject/@Number` behind each agrees with the id's
own `O-<n>` digits in all 867 cases, across all 310 distinct ids and all 35
devices **[V]**. What is *not* evidenced is that every ETS 6 export writes
this form. One sample has already proved the wrong thing about schema ≥21
once — ADR-0014's claim that instance-level flags do not occur at schema ≥21
was measured against `KV v2.5 - demo.knxproj` alone and this same project
disproved it with 119 of them **[D]**.

**Why that is safe rather than merely likely.** The reader is strict and the
residue is reported, not guessed. `device_local_com_object_number` accepts
exactly one underscore, `O-` then digits, `R-` then digits; anything else —
including a schema-11 parameter ref (`…_UP-411_R-411`) and a module id
(`MD-…`) — still falls through to `MalformedRefId` and still reaches the user
through the import report. On the enrichment side the program prefix is
always the program *that device* resolved to, looked up per device, so a
misread id produces a reported `ComObjectRefMissing`, never another device's
communication object attached to this one (the §34 ruling, on the import
side) **[V]**.

**Residue a user can still hit on this very project.**

- `report.has_losses()` is still `true` for the schema-23 reference project,
  now for a smaller and entirely different reason: **9** unknown constructs,
  five attributes in `P-0512/0.xml` (`Space/@CompletionStatus`,
  `Area/@Name`, `Line/@CompletionStatus`, `Line/@Name`,
  `DeviceInstance/@CompletionStatus`) and four in `P-0512/Project.xml`
  (`ProjectInformation`'s `CompletionStatus`, `ProjectId`,
  `ProjectTracingLevel`, `Hide16BitGroupsFromLegacyPlugins`). All nine are
  preserved as retained attributes and each is reported with its own xpath,
  name, occurrence count and a sample value, so the user is told what and
  where. Map errors on that project are now **0** **[V]**.
- Enrichment reports **107** `AmbiguousDpt` issues on this project, which
  only became visible once the lookups started hitting. Those are the
  deliberate refusal to pick one datapoint type out of a list of stated
  alternatives (RESEARCH §4.2), not a lookup failure **[V]**.
- A device whose application program is not in the product database cannot
  have its ids qualified at all, since the prefix *is* the program. That is
  the ordinary manufacturer-data gap and is reported per device as
  `ProgramMissing`, exactly as before **[A]**.

**Not affected.** The schema-11 (ETS4) and schema-21 (KV demo) corpus
projects map and enrich identically to before: 907 and 75 communication
objects, 0 and 1 import errors, and 107 and 0 enrichment issues. The ETS4
issues are its already-pinned `AmbiguousDpt` alternatives, not lookup
failures. These counts are covered by
`crates/knx-etsproj/tests/device_local_com_object_refs.rs`,
`crates/knx-app/tests/enrichment_gap_measurement.rs`, and
`crates/knx-app/tests/ets6_device_local_enrichment.rs` **[V]**.

**Lifted when.** A second, independently produced schema-23 project is in the
corpus and its `GroupObjectTree` ids are counted the same way. This is the
same evidence `COMPATIBILITY.md` already wants for schema 23's module
handling, and one sample would settle both.

## 126. line-scan reconciliation acts on occupancy evidence, not device identity

**Boundary.** A completed line scan can establish that an address answered,
did not answer within the selected policy, or was not examined. It does not
identify a manufacturer, product, application program, serial number, or the
reason for silence. T09 therefore creates an explicitly product/program-less
device for a selected unexpected response and never enriches it by inference.
Likewise, a missing project device is deleted only after the user explicitly
selects it; the UI states that silence is not proof of absence.

Excluded project addresses and the scanner's own address are shown as
unexamined and have no action. The
server recomputes the comparison against the current project before applying
anything, requires a completed matching scan session, and refuses a missing
address that does not resolve to exactly one project device. It also refuses
removal while building placement, parameter, or module-instance data still
references the device. Unexpected devices use a matching line across all
installations, falling back to the first installation's unassigned list. One
batch and one undo cover all selected findings; an empty selection changes
neither project nor undo history. Project-tree updates invalidate stale UI
selections. No reconciliation action sends KNX traffic **[V]**.

**Lifted when.** Identity may be attached only when a separately verified
protocol procedure or explicit user selection supplies it. A scan response by
itself never becomes product evidence.

## 127. A site over several buildings rests on schema text and synthetic tests, not on an ETS sample

**Boundary.** [ADR-0038](adr/0038-site-is-a-ground-root-space.md) represents a
site/property that groups several buildings on one KNX infrastructure as a
`Ground` space at the root of one installation's building structure, with
`Building` children. The only authority is *Project Schema23 v01.00.00*
§1.2.6.3 ("Space elements directly below Locations_t will nromally have Type
"Area" or "Building" or “Ground”"). None of the three reference projects
contains a `Ground` space, more than one root space, or more than one
installation (ADR-0038 E3). The import, native round-trip and command
behaviour is covered only by synthetic tests (`knx-etsproj/tests/site_hierarchy.rs`,
`knx-store` and `knx-core` unit tests). How ETS 5/6 displays or restricts a
`Ground` root is unverified.

An undocumented `Space/@Type` such as `Site` is **not** read as `Ground`. It
stays an `UnknownEnumValue` map problem with the existing reported
`BuildingPart` fallback (§89), and a test pins that.

The Buildings workspace now offers **Add site / property**, a fixed `Ground`
root in the first installation; existing buildings can move beneath it.
Mocked UI tests cover two buildings under one root without duplicated
devices. This does not supply independent ETS `Ground` export evidence or
make later installations editable.

Found on the way and not addressed: no command renames an `Installation`
after creation. `Installation.name` comes only from `NewProjectDialog` or
import. That matters once a user splits separate infrastructures into
separate installations. Larger than that: **no command edits any
installation but the first.** Every `Command` applies to `installations[0]`
(`knx-core` `command.rs`; `CreateBuildingPart` uses
`installations.first_mut()`), so a second installation brought in by import
is kept and saved but cannot be edited. ADR-0038's "separate infrastructures
are separate installations" is therefore a representation KNXBench can hold,
not yet a workflow it offers.

**Alpha disposition (2026-10-04).** A renewed search found no `Ground`
sample in the corpus or in eight public xknxproject fixtures (RESEARCH §25).
On the user's instruction this stays a known gap and is closed for the Alpha
(`KL-127` → `ACCEPTED_BOUNDARY`); it is not evidence that ETS accepts the
ADR-0038 shape.

**Lifted when.** A real ETS export containing a `Ground` root (ideally with
several `Building` children, or several installations) is added to the
corpus and imports with the ADR-0038 shape. The installation-rename and
first-installation-only gaps are tracked separately, for ISSUE-05/B10
triage.

## 128. Legacy `.vd3`–`.vd5` and `.pr3`–`.pr5` files are refused, and the refusal misnames the format

**Limitation.** KNXBench cannot install a legacy ETS3-era product database
(`.vd3`–`.vd5`) or a legacy project export (`.pr3`–`.pr5`). Both are ZIP
archives with one ZipCrypto-encrypted member (`ets.vd_` or `ets.pr_`) that
holds a textual `EX-IM` payload, not a `.knxprod`. The supplied MDT
`MDT_VD_VisuControl.pr5` is refused atomically on both paths, but under the
wrong name. `knx products ingest` falls through to the project importer and
reports "not a zip archive". The same bytes renamed `.knxprod` report
"encrypted product ZIP member" **[V]** (2026-09-26, `7b64496`, fresh scratch
product DB, 0 rows written).

**Cause.** No legacy path exists. The CLI routes by suffix (`.knxprod`/`.vd2`
only), and `install_package` has no content detector for `EX-IM` containers.
The `EX-IM` grammar is not described anywhere in *The KNX Standard* v3.0.0
(0 hits across all 179 extracted documents) **[V]**, so every rule has to be
derived from observed files.

**Impact.** Users with only a legacy file must convert it with the official
tooling (ETS6 imports `.vd*` directly; `KnxCvNext.exe` or the Manufacturer
Tool produce a `.knxprod`). The supplied `.pr5` would gain little from a
direct importer anyway: its `application_program` table has zero rows, so even
a perfect import yields a catalogue entry with no parameters and no
communication objects **[V]**.

**Lifted when.** The design
[2026-09-26-legacy-vd-pr-product-import-design.md](superpowers/specs/2026-09-26-legacy-vd-pr-product-import-design.md)
has an independent review verdict and Board approval of its decisions B-1 to
B-6. After that, its slices are implemented one at a time: L1 is named
detection and refusal without decryption. The user-supplied-password,
no-embedded-password and GPL-provenance constraints of
[VD4_PRODUCT_DATABASE_IMPORT.md](VD4_PRODUCT_DATABASE_IMPORT.md) stay binding.
The permanent `.vd2` decision in
[§11](#11-knxprod-support-is-evidenced-for-schemes-11-12-13-14-20-and-exact-namespace-21)
is unaffected.

## 129. A stale id-allocator snapshot can duplicate ids, and saving then drops one entity

**Status.** Open, but the data-loss path is closed: phases 1–2 of ADR-0039
landed on 2026-09-27. A colliding id is refused, and no caller rewinds the
counters any more. AR02 also refuses allocator exhaustion. Structural
phases 3–5 remain `WAITING_DECISION`; an unanswered activation prompt is
neither approval nor accepted continued deferral. See the pinned
[AR03 audit](ADR0039_ENFORCEMENT_AUDIT.md) at `e691bc13`.

**Historical limitation (before phases 1–2, at `7b64496`).**
`Command::SetIdAllocators` replaces the id counters
absolutely, and no `Create*` command refuses an id that is already in use.
A caller that snapshots the allocator, releases the project lock and applies
later can therefore lower the high-water mark and insert a second entity
with an existing id. `save_project` upserts by id (`ON CONFLICT(id) DO
UPDATE`), so on save one of the two entities silently disappears.

**Historical cause (at `7b64496`).** `Project`'s six fields are `pub`, so ARCHITECTURE §6's "every
mutation is a `Command`" is held by review, not by the type system
(goal.md §8.4, F-T30-1). Eight live server handlers advance `project.ids`
outside any command, and `create_device_impl` enriches the live project
after `CreateDevice` was applied. Those nine points are safe today only
because they increment the live counter directly. The non-destructive
group-address CSV import plans under one lock acquisition and applies under
another without a revision check (`apps/knx-server/src/domain.rs:965-1025`
at `7b64496`), so it is exposed.

**Historical evidence.** A scratch probe against `knx-core`, `knx-csv` and `knx-store`
at `7b64496` interleaved `create_area_impl`'s and
`create_group_address_impl`'s command sequence between `plan_import` and the
apply. It produced two group addresses with id 1 and an area counter of 0
while area 1 existed. `save_project` returned `Ok`, and after reload only
one of the two group addresses was left **[V]** for the library path. The
race has not been reproduced over HTTP; it is reachable by construction on
tokio's multi-threaded runtime.

**Historical cost.** Silent loss of a user's group address (or another entity) on save,
with no diagnostic. It needs two concurrent edits against one project, so
it is rare with one user and one browser tab.

**Partly mitigated (ADR-0039 phase 1, DIN-11, 2026-09-27).** Every
id-inserting command (`CreateDevice` and its com objects, `CreateArea`,
`CreateLine`, `CreateGroupRange`, `CreateGroupAddress`,
`CreateBuildingPart`, a new-instance `SetParameterValue`) now refuses an id
already in use with `CommandError::IdInUse`, so the appendix interleaving
ends in a typed refusal and a `Batch` rollback instead of a silent loss on
save (`crates/knx-app/tests/id_allocation_integrity.rs`, which fails with the
check removed **[V]**). `Command::ReserveIds` exists, and `load_project`
raises a stored counter below an id in use and reports it
(`load_project_reporting`; the server logs a warning on open, CLI
`ga-import` prints it). **Phase 2:** the CSV planner and scan reconciliation
emit the never-rewinding `ReserveIds` instead of `SetIdAllocators`. Every
applied CSV plan is bound to the revision it was planned against, so an edit
in between refuses it with "import or preview again" before it gets to the
id backstop (`domain.rs::a_csv_plan_is_refused_when_the_project_changed_after_planning`,
red without the binding **[V]**). Undoing an import or scan apply keeps the
counters' high-water mark, so an undone stable id such as `KB-GA-n` is never
reissued. Current remaining phases 3–5: six server allocation paths still
advance live `project.ids`; catalog creation already allocates from a clone,
but single creation assigns that clone after command success and seed
enrichment still runs after the command. The ID field is still public and no
mutation-source gate enforces the rule. These are architectural enforcement
boundaries, not a newly demonstrated silent-loss regression.

**Lifted when.** [ADR-0039](adr/0039-project-mutation-goes-through-commands.md)
(Accepted) phases 1–2 are already merged. This residual entry closes only
after explicit activation and verified phases 3–5 remove the current live
bypasses, seal the agreed ID surface and add `check-project-mutation`, or
after the user explicitly accepts the disclosed continued deferral. Neither
decision is inferred here; until then this entry stays open.

## §130 A gate binary can verify a directory that no longer exists

**Status.** Resolved by AR01 (2026-10-01); historical heading/anchor retained.

Before AR01, `xtask`'s checks derived their repository root at **compile time** from
`env!("CARGO_MANIFEST_DIR")` (`xtask/src/main.rs:41`, `:234`, `:283`), not from
the working directory at run time. A cached `xtask` binary built inside a
different worktree therefore keeps checking *that* worktree's path. When the
worktree is deleted, `check-anchors` fails with `cannot read .../DIN-3`, while
`check-headers` reports `0 files with a well-formed header ... 0 without one`
and still **exits 0** — a gate that inspected nothing and called it success.

**Evidence.** After the `din-3-goal-migration` worktree was removed,
`strings target/debug/xtask` still contained
`/mnt/daten-i/Sourcecode/.paperclip-worktrees/KNXBench/DIN-3`. `git worktree
prune` did not help (the path is in the binary, not in git metadata), and
`touch xtask/src/main.rs && cargo build -p xtask` did **not** rebuild it on
this ntfs3 mount. A build with a fresh `CARGO_TARGET_DIR` produced a binary
carrying `/mnt/daten-i/Sourcecode/KNXBench`, after which the same three gates
reported real magnitudes: anchors **382 links / 214 files**, headers **215**,
layering ok, all exit 0 **[V]**.

**Cost.** Any documentation gate run from a stale binary is worthless but
looks green. This is the skip-vs-pass failure of §129's corpus tests one layer
up: exit code 0 is not evidence that work happened.

**Resolution.** Current gate binaries select the exact runtime workspace root
from CWD or an explicit leading `--root PATH`, validate its Cargo workspace and
named members, and print that canonical target. They never climb to a parent
or fall back to the build tree. Required source/documentation scan roots must
be nonempty; layering requires every checked policy root as a workspace member
and resolved node. Corpus-gate output now includes actual Rust-file coverage.
See [verification targets](VERIFICATION.md) and `xtask/tests/gate_scope.rs`.

AR01 reproduced the old success over zero sources after deleting its own build
worktree, then verified new deleted-target refusal, valid/wrong targets,
intentional fixtures and behavioral guard mutations. A pre-AR01 executable
still has the old bug and must be rebuilt. The caller must still verify that
the emitted target/revision is the intended candidate; nonempty coverage does
not certify completeness or concurrent-tree stability. No native UI or bus
claim is added; the separate zoom entry numbered 130 remains open.

## §133 A dead webview cannot be closed with the window manager's close button

**Status.** Open (documented 2026-09-27; follows from the §132 fix, read
from the pinned `tauri` 2.11.5 sources, not reproduced).

**Limitation.** Since §132, the main window's frontend listens for
`tauri://close-requested`. While such a JS listener is registered, `tauri`
2.11.5 calls `prevent_close()` on every `CloseRequested`
(`manager/window.rs` `on_window_event`), and only the frontend's handler can
then `destroy()` the window. If the WebKit web process has crashed or hangs
for good, × and Alt+F4 do nothing. Rust never drops a JS listener on its own
when a page crashes or reloads, so a stale registration keeps blocking. The
same holds briefly after a reload until `App` mounts again, for example while
a login screen is shown.

**Cost.** The user has to end the process some other way (the compositor's
kill binding, `kill`, a task manager). Unsaved edits are lost exactly as they
would have been before §132; the new cost is that the window will not close
at all.

**Why it is this way.** The obvious mitigation, Rust destroying the window
when the frontend does not answer within N seconds, would close a merely
*busy* frontend with unsaved edits and no question asked. That is the silent
loss §132 removed. Data integrity outranks convenience here.

**Lifted when.** The shell can tell a dead web process from a busy one. For
example, it could observe a WebKit web-process-terminated signal and then
drop the stale listener or destroy the window. Either way this needs a
test, or at least a manual reproduction on a real window manager.

## §134 Baggage is inventoried, not interpreted

**Limitation.** Since PDB-10 (schema v16, ADR-0042) every baggage
declaration is typed and resolved and every payload is classified, but
nothing acts on it: `InstallOnImport`, `TargetPath` destinations and
`FileInfo` timestamps are carried as raw lexemes, icons are not rendered,
manuals not opened, plug-in DLLs/MSIs not run, nested ZIP entries not
decompressed. Media classes cover the formats the corpus contains (PNG,
JPEG, GIF, BMP, PDF, ZIP, PE, OLE2, XML); anything else is `unknown`, not
guessed. A nested ZIP's expanded size is what its directory *declares*, not a
measured decompression. Installing still holds each member whole in memory
(≤ 64 MiB each; a 54.8 MB member measured 4.2× peak RSS growth). Baggage
*references* from application programs
(`Static/Extension/Baggage/@RefId`, 935 distinct in the corpus) are reported
as unknown constructs, not resolved against the inventory; whether the single
`Hardware/Product/Baggages` element is reported has not been probed. When a
`Baggages.xml` is both ingested standalone and carried in a package, a fresh
install records its index unknowns under that blob once per path while a v15
upgrade records them once, so unknown *row* counts (not the inventory or
report) can differ in that shape. The package corpus alone cannot produce
it; a project import followed by installing the same package can.

**Cost.** The user sees what a manufacturer ships and which files are
undeclared, but cannot preview them in KNXBench, and a package that relies on
a baggage file being installed on import gets no such installation.

**Why it is this way.** The Project Schema specifies only that each
`Baggage` is an external file; the corpus spells `InstallOnImport` as both
`false` and `0`. Acting on undocumented semantics, or opening untrusted
vendor payloads, would trade integrity and safety for convenience.

**Lifted when.** A specified source defines the declaration attributes, or a
concrete feature (icon display, manual links) needs a payload and brings its
own sandboxed viewer with tests.

## §135 Package identity is recorded, not decided

**Limitation.** Since PDB-11 (schema v17, ADR-0043) every element of the six
package-content kinds (`catalog_section`, `catalog_item`, `hardware`,
`product`, `hardware2program`, `application_program`) in every parsed member
blob is recorded with an element digest, and `knx products identity` names
the winner, every candidate, their packages and whether each equals the
winner. The stored row is still the **first installed** one: installing the
same packages in another order stores other values for the ids whose
elements differ. KNXBench shows this; it does not choose.

- The digest is conservative. Equal digests mean equal as the parsers read
  it: the element's subtree plus the context its stored row takes from
  outside (manufacturer, parent section, parent hardware). Different digests
  can come from differences without meaning (a different namespace prefix, a
  changed `Hash` attribute, re-wrapped text). Whitespace-only text and
  namespace URIs are not compared, as the parsers ignore them too.
- Translations are not compared. A program's `Languages`, and catalogue and
  hardware translations, live outside the element.
- Families (`ApplicationNumber`) and `ReplacesVersions` links rest on an
  unofficial public copy of the project schema plus corpus agreement, not on a
  KNX-published XSD. A value that does not parse is shown raw with the reason
  and nothing is linked; a listed version with no installed program means
  *not installed*, not *does not exist*. KNXBench implements no upgrade
  behaviour behind `ReplacesVersions`.
- Order-number lookup is exact-string per manufacturer. An order number is
  never an identity and never merges products.
- Master data is not covered: `manufacturer.name` stays last-writer-wins
  (§88) and `datapoint_type` first-wins without provenance (§86).
- A blob the scan cannot read (an entity the parsers ignore, or retained
  bytes that no longer match their hash after an upgrade), or whose rows
  from before v17 disagree with the scan, is recorded `unavailable` with its
  reason and listed as unmeasured, not guessed. It is not rescanned later.
- Server and web have no identity, family or order-number views; the
  library and CLI do.

**Cost.** A user who installs overlapping packages sees which package
supplied each value and which offer different ones, but cannot yet pick a
different winner.

**Why it is this way.** No normative ordering of package content exists:
`ToolVersion` is a free-form producer string, the namespace is a format
version, and any hash order would be arbitrary and silently change existing
databases (ADR-0043 §2).

**Lifted when.** A specified ordering of package content exists, or a later
UI slice lets the user pick a winner per id.

<a id="136-mask-0701h-bim-m112-devices-cannot-receive-an-application-download"></a>

<a id="136-mask-0701h-bim-m112-devices-can-receive-an-application-download--lifted-the-restart-stays-unconfirmed"></a>
## §136 `0701h` download verified once; restart remains unconfirmed

The earlier claim that mask `0701h` could not be downloaded is withdrawn.
A memory download of `A-0027-15-0BAC` and a functional group-telegram check
on MDT `1.1.67` succeeded with a device-specific go (RESEARCH §19.4).
CLI/Web commands now expose this memory path, and partial scopes have their
own bounded evidence (§7/§142). The closing Basic Restart on this device
remains unacknowledged: `RestartOutcome::Unconfirmed` must not be presented
as a verified restart or a failed image read-back. This single product/device
is not evidence for other masks or revisions.

## §138 A device's access key comes from the project or a key file, and nothing checks it live

**Status (2026-09-29, K11).** A download authorises with a key when one is
given, and with none otherwise. Implemented and tested in the simulator,
the CLI and the HTTP routes. **Not tested on hardware**: the test device
(`1.1.67`) has no key set, and setting one would be a write the goal does
not need.

**Where the key comes from.**

- **The project**: `Installation/@BCUKey` (Project Schema 23 p. 38, *"The
  key used to lock devices supporting authentication"*), read from the
  retained attributes of the imported project. The default `4294967295`
  (`FFFFFFFFh`) means no key. All three corpus projects carry the default.
- **The operator**: `knx device download --key-file <path>` (decimal or
  `0x` hex). There is no `--key` flag: argv is readable by every user
  through `ps`. The HTTP API takes no key at all.
- **Neither**: the free access level. A device that refuses it gets a hint
  naming the missing key as one possible cause. **No key is ever guessed.**
  A key file wins over the project; two installations with different keys
  are refused before anything opens, because a guess between them would
  be a guess.

**What a key does on the bus.**

- Mask `070nh` (BIM M112) runs MP §3.5.2 `DM_Authorize2_RCo`: the free key
  `FFFFFFFFh` first, the project's key only if the free level is not the
  highest. The diagram on p. 76 nests the key request inside *"If the free
  access level is not the highest level"*. Before this, the code sent the
  key whenever the free level was 0 as well; the test that pinned that
  reading was rewritten against the diagram.
- Other masks run MP §3.5.1: one `A_Authorize_Request` with the key.
- The key level is checked against the profile's 16 levels (Profiles
  Table 4.2 p. 37).

**Why no live test.** A locked `0701h` device would be needed. Locking one
means `A_Key_Write` (MP §3.6 `DM_SetKey`), which KNXBench does not
implement and the goal does not ask for. The tests use the simulator with
the free level at 3, a key at level 1 and writes needing level 2.

**Web UI.** The server sends `accessKey` in the plan, an `authorised`
event and a `hint` on failure. `DeviceDownloadPanel` does not show them yet
(the web lock belonged to the parallel UI session). It ignores the unknown
event kind, so nothing breaks.

**Lifted when.** A download to a device locked with a known key succeeds
live, the trace shows two `A_Authorize_Request`s, and the key appears in
no log or output.

<a id="139-a-device-can-be-addressed-by-its-serial-number-and-nothing-has-done-it-live"></a>
## §139 Serial-number address write was ignored by the tested device

MP §2.4 serial-number read was verified on MDT `1.1.67`. The MP §2.5
address write was sent and ignored: `1.1.68` remained free. The user also
authorized a guarded `PID_SERVICE_CONTROL` bit-2 set/read-back/clear; the
write remained ignored. Correcting the four address broadcasts to SYSTEM
priority did not change that outcome (RESEARCH §19.15). The CLI reports
"sent, not confirmed"; do not represent the simulator implementation as
a successful live write. For this device the programming-button procedure
(MP §2.3) is the historically verified route. Public confirmed serial
writes now fail closed before a tunnel (ADR-0057): an action-specific
durable pre-write backup of all affected storage is unavailable. That
gate is separate from device support and from the property-only backup
below. A different identified device or manufacturer statement would
address support, but not waive recovery or fresh authorisation.

The Debug bit-2 server route remains default-off. ADR-0051 now requires a
durable, versioned pre-write record of the original two property octets,
device address and mask. CLI and HTTP persist, read back and sync the record
and directory before any bit-2 change; backup failure refuses the write and
a no-op writes neither file nor property. HTTP reports `backupPath`; CLI
uses `./device-backups/` or `--backup-dir`. Simulator/route/CLI coverage
exists, but this *new recovery gate* has not run on hardware. It is a
property-specific manual recovery aid, not a full device image or automatic
restoration of manufacturer side effects. After an ambiguous write, inspect
the device and backup rather than retrying blind. The Settings Debug toggle
and explicit service-control tab now expose this narrow action; the toggle
reads the server setting back, and the panel requires an explicit read and
typed phrase before sending a write. A returned property result is not a
whole-device receipt; bounded `serviceControlWrite` activity is not a
durable audit trail (ADR-0055/0056). The new UI/backup gate is covered only
by local mock/simulator tests, **not a new live hardware run**.

**Recovery ordering correction, 2026-10-02.** The original property backup
was taken after connection setup had already asserted Verify Mode. The
service-control path now postpones that setup write, validates the original
PID_DEVICE_CONTROL byte, and durably keeps both original properties in
format 2 before any property write. Legacy format 1 is preserved, not silently
filled with an invented byte. No-op and wrong-scope calls do not write;
malformed widths and failed backups refuse. This does not close whole-device
recovery, durable history or hardware-support residues (ADR-0051 amendment).

<a id="146-a-channel-without-text-has-no-name-of-its-own-and-some-activations-are-undetermined"></a>
## §146 Channel labels are shown; undetermined activation and missing DPT remain

**Bounded text-refusal policy accepted (2026-10-03 13:32 CEST, ADR-0066).**
Scoped FunctionText substitution and channel-text rendering/cached copies share
one4,000,000-unit content/work context. Overflow refuses the whole DeviceDetail
through existing HTTP400, no clipped output or changed activation/write authority.
Actual0369a56a/22 commands passes: Rust2995/0/165, Web1702/Chromium72,
selected private9,699 exact inputs/17 equal bindings/420 unchanged originals.
No new generated/UI contract; dedicated language/refresh error/snapshot behavior
is UI-owner verification. Legacy String SDK helper, unscoped/raw metadata/query/
serializer allocations and total RSS remain outside this bound. Full AR07/ETS
and the newly found AR06P nested-definition sample are not accepted by this gate.
Final five document checks passed; scoped delivery 0b8ec935
was published/fetched/live-read back,699 inputs/7 docs exact at that checkpoint.
Whole AR07/Alpha and dedicated UI consumer acceptance remain open. See
[decision](adr/0066-outside-walk-text-refusal.md).

**AR07 candidate (2026-10-02, ADR-0061).** A resolved controller kind outside
Number/Restriction (with the existing None exception retained) now emits
UnsupportedControlKind, skips/names branch refs and marks activation potentially
hidden. Public ingest/load, nested-scope, source-reopen, HTTP hidden-field refusal
and Undetermined projection checks pass; six behavioral mutants are detected.
This is a 1,118/0/57 public two-crate candidate, not new whole-corpus counts or
ETS parity. The warning token/English fallback is on the backend wire; Web's
manual union/localized catalogue adoption remains with UI, not completed here.
The corrected broad candidate is independently verified at 13/13: workspace
2,924/0/164, Web 1,357 and six selected private Dynamic tests 6/0/0; all 103
original archive hashes unchanged, no genuine skips or private raw logs. This
selection is not blanket validation of every archive/opaque construct. Upstream
UI ancestry integrated at 03f18c95 and independently accepted: 17/17, workspace
2,924/0/164, Web 1,559, Chromium fixtures 61, selected private 6/0/0 and
seventeen shadow bindings equal. Published/read back at 2d9aaeb8;
full AR07, typed/localized token adoption and earlier corpus figures retain
their distinct dated scope. See [parameter boundary](PARAMETER_SEMANTICS_BOUNDARY.md).

**Status (2026-09-29, ISSUE-08 P2, ADR-0050).**

- **Channel names.** `ComObjectChannel::text` is the element's `@Text`, and
  `None` when that is empty. In the corpus, 24 of 29 `Channel` elements
  have an empty `@Text`, including every one in the two house exports.
  **Data half lifted (2026-09-30, schema v18, ADR-0052):** they do state
  `@Name` and `@Number`, and `dynamic_node` now stores both verbatim.
  `ComObjectChannel` carries them as `name` and `number` (`None` when
  absent or empty), never composed into `text` and never translated, since
  no product translates `@Name`. `@Number` is text: 5 of 1,268 corpus
  channels hold a value that is not a number. What the standard means by
  either attribute is not documented in the schema text available here;
  KNXBench shows them as written. The UI now uses `@Name` as the heading
  when `@Text` is absent and shows `@Number` as text, without composing or
  translating either value.
- **`Undetermined`.** The evaluation cannot decide in these cases:
  - two module instances with one `RefId` (D40);
  - a scoped activation that no imported instance owns;
  - a value or definition the evaluation could not use.
  Objects in these cases stay `Undetermined`. None occur in the corpus
  (0 of 1,849).
- **Stale module rows.** A stored module row whose `_MI-` digits disagree
  with the imported instance is set aside as stale. Its parameter then runs
  on the program default. `Active` is not downgraded for it, the same as in
  the parameter panel, which evaluates identically. There are 0 stale rows
  in the corpus.
- **Typed UI boundary (2026-09-30, U12/ISSUE-08 UI half, published on main).**
  The previously skipped activation, owner channel, program DPT, translated
  DPT text and function text now have generated TypeScript bindings. The
  device editor groups by the supplied opaque key, shows all four evaluated
  states plus the stored claim, and leaves unmatched objects inspectable.
  **§146 channel-label follow-up (2026-09-30):** the UI shows `name` and
  `number` without changing the translated `text`: it uses `name` as the
  heading only if `text` is absent, shows a separate name fact when text is
  present, and always shows a present number as an uninterpreted string.
  The channel-label gap is lifted. Missing and multi-choice DPTs still have
  no chosen identifier; genuinely undetermined activations remain visible.
- **No DPT to show (P3).** 495 ETS4 and 456 ETS 6.3.0 objects have neither
  a stated DPT nor a program default. For 473 and 434 of them the product
  states none. For 22 each it states a list of several (`DatapointType`
  with more than one id) and neither the project nor the product picks
  one. KNXBench shows no DPT for them rather than choosing from the list.

**Channel-label condition lifted (2026-09-30).** The UI displays both
`name` and `number` when present (`DeviceWorkspace.test.tsx` and local
EN/DE browser fixture); the data half is ADR-0052. The remaining
`Undetermined`/missing-DPT conditions above stay explicit, not guessed.

<a id="145-instance-level-flag-overrides-are-not-written-into-the-group-object-table"></a>
## §145 Instance flags are written; unlinked active objects can still differ

Instance-level flag overrides (`Layer::Instance`/`Layer::UserEdit`) are
written into active group-object table entries; malformed or empty flags
are refused, not replaced with product defaults. The corpus regression
checks 12 overridden, linked objects against read-only device octets
(RESEARCH §19.13). No KNXBench download to those house devices has run, so
this is image/read-back comparison rather than hardware download evidence.

**Remaining difference.** ETS clears the communication-enable bit for an
active object without a group-address link; KNXBench leaves it set. With no
association that object cannot send or receive either way. Neither program
behaviour nor download parity for unlinked objects is asserted.

## §144 RF device configuration exists in the simulator only

**Status (2026-09-29, K17).** CP §3.6 and §3.7 in
`knx_core::commissioning::rf_configuration` (DD2, the group-address
calculation, the `PID_OBJECTLINK`/`PID_PARAMETER` payloads) and
`knx_net::commissioning::rf_configuration` (`DMP_Connect_RCl`, MP §2.6
after the link sequence, Write/Read Parameter, Write Object Link,
InfoReport collection), plus the three function-property PDUs of AL
§3.4.7 in `knx_net::cemi`. No KNX-RF device has been on this bus:
`WriteScope::RfConfiguration` is refused on hardware, and there is no CLI or
HTTP route.

- **Group-address calculation needs channel definitions from outside.**
  CP §3.7.2.3 numbers group addresses from `0001h` along the E-Mode
  Channels' object lists, which Volume 7 defines per application.
  KNXBench holds no such table; the caller passes the object counts, and
  an unknown code is refused. A Channel Info with more than one instance
  is refused too: the only example in CP has one of each, and how several
  are numbered is not shown.
- **MP §2.6 starts after the link sequence.** Its opening
  `CC_Config_Link` exchange is PB-Mode (Easy mode, a later goal).
- **The InfoReport's serial number is not surfaced.** MP §3.2.7: on RF the
  frame carries the sender's KNX Serial Number in the 'RF medium
  information'. `LDataFrame` does not carry that yet, so the report names
  the source address (`05FFh` for every unidirectional device) and DD2.
- **Only the Device Object's two function properties.**
  `PID_OBJECTADDRESS`, `PID_OBJECTLINK`'s read iterator, the E-Mode
  Channel objects and BiBat (CP §3.8) are not implemented.
- **Duty cycle** (CP §3.7.5.5.1.4, 1 % on 868 MHz): not tracked.

**Lifted when** an RF device and an RF-capable interface are available and
the user approves a run.

## §143 RF domain addresses exist in the simulator only; there is no RF device

**Status (2026-09-29, K16).** The six domain-address PDUs of AL
§3.3.3–§3.3.7, the system broadcast (cEMI Ctrl1 SB), the cEMI 'RF medium
information', and MP §2.7, §2.8, §2.9, §2.10 and §2.12 are implemented in
`knx_net::cemi` and `knx_net::commissioning::domain_address`, against a
simulated RF device. **No KNX-RF (or PL110) device has ever been on this
project's bus**, so none of it has met hardware:
`WriteScope::DomainAddressProgramming` is refused on hardware, and the
read procedures have no CLI or HTTP route.

- **Not implemented, with reasons.** MP §2.11 is *"not yet specified"* in
  the PDF. §2.13 (`…_Secure_Write`) needs KNX Data Security. §2.14
  `A_DomainAddressSelective_Read` is two-octet (PL110) only (AL NOTE 6).
  The KNX IP forms of `A_DomainAddressSerialNumber_*` (4 and 21 octets,
  AL Figures 31/32) decode as `Other` with every octet kept.
- **§2.12 steps 1 and 4** switch a KNXnet/IP router's system-broadcast
  routing mode. There is no router in the simulator; the caller owns them.
- **Time-outs are the caller's.** §2.12 borrows KNX IP's 1 s / 60 s for
  its verify loop; this code runs `SERIAL_WRITE_VERIFY_ROUNDS` = 3 rounds
  of the session's response time-out, which is KNXBench's choice.
- **§2.12's own verify cannot see an RF domain address.** It reads back
  with `A_IndividualAddressSerialNumber_Read`, whose response has two
  domain octets. KNXBench also reads `A_DomainAddressSerialNumber_Read`
  and compares; that second read is ours.
- **§2.10 does not read the domain address back.** Its report says
  *wrote*, never *verified*.
- **RF frames over KNXnet/IP.** `encode_l_data_rf` writes the eight-octet
  'RF medium information' (with LFN). The same PDF's own example on p. 74
  draws seven octets without LFN; both are decoded. Which one an RF
  interface expects is not known without one.
- **The cEMI decoder now reads SB.** A group frame to `0000h` with SB
  clear decodes as `Destination::SystemBroadcast`. Before K16 the decoder
  ignored SB and showed such a frame as a plain broadcast to `0/0/0`. Every
  frame KNXBench itself sent had SB set and still does, except the new
  system-broadcast services.

**Lifted when** an RF device and an RF-capable interface are available and
the user approves a run.

<a id="142-the-mask-070nh-partial-download-is-tested-in-the-simulator-only"></a>
## §142 Partial download: one verified device, product and UI boundaries remain

**Verified scope (2026-09-29/30; RESEARCH §19.15/§19.17).** On the MDT
`1.1.67`, the `parameters`, `group-addresses` and `both` partial scopes
completed with the same option-C image: respectively 394, 1022 and 1416
octets read back, with the relevant load states `Loaded` and byte-identical
post-run dumps. One earlier group-address run was interrupted by an external
shell timeout; the complete download restored it before a successful retry.
The history and recovery evidence are retained in RESEARCH, not an open
"simulator-only" limitation. Its closing restart remains unconfirmed (§136).

`knx device download --partial
parameters|group-addresses|both` and the `partial` field of
`POST /api/device-download/plan` derive CP §3.9.2.4's partial download
from the complete plan (`knx_core::commissioning::partial_memory_download`).
The partial plan is a subset of the complete one: no application unload, no
application allocation, tables only when group addresses are selected.

- **Hardware evidence is narrow.** All three partial scopes ran only on
  `1.1.67` with the same option-C image; other programs and changed target
  configurations remain unverified. `WriteScope::Download` still requires
  the device-specific phrase, a pre-run baseline and a restoration plan.
- **Two checks are KNXBench's, not the Standard's.** Before the first
  write, the device must report the plan's application in
  `PID_PROGRAM_VERSION` (object 3), and every part the complete plan loads
  must be `Loaded`. Otherwise nothing is written and the message says to
  run the complete download. Object index 3 for the application program is
  the documented default (`ObjectIndex::APPLICATION_PROGRAM`), and it is
  what `1.1.67` answered to.
- **Non-EEPROM application data is not written.** CP rule 3 ignores it. The
  plan lists every such write as "not written"; the MDT products in the
  corpus have none.
- **`AppliesTo` and `LegacyAllowPartialDownloadIfAp2Mismatch` are not
  interpreted.** Only two of 203 `070n` programs carry `AppliesTo`, both
  `full,par`, and no PDF read defines the option.
- **The web UI does not offer it yet.** The route accepts it; the panel
  belongs to the UI session.

## §141 Master Reset erases in the simulator only; hardware keeps its configuration

**Status (2026-09-29, K14).** MP §3.7.1.2 Tables 4 and 5 are typed
(`knx_core::commissioning::master_reset`). A request is only built with an
Erase Code a client may send (`01h`–`08h`) and a Channel Number the code
allows. `knx_net::commissioning::master_reset` runs MP §3.7.3
`DM_Restart_RCo`: probe, request, the 6 s disconnect wait, then the device
is looked for where it should be, once more after the Process Time.

**The support check is ours.** MP §3.7.3 requires verifying Master Reset
support first and names no method. We send a Confirmed Restart (`01h`,
erases nothing) and go on only on a positive `A_Restart_Response`.
Footnote 11 warns that old devices may just restart; that is what the probe
risks, and all it risks.

**Refused on hardware.** Erase Codes `02h`–`08h` need the new
`WriteScope::MasterReset`, which `hardware_write_is_authorised` refuses.
**Fixed along the way:** `restart_master_reset` sent every Erase Code under
`WriteScope::Restart`, which hardware permits, so a restart phrase could
have carried a Factory Reset. The scope now follows the code; a test and a
mutant hold it. `01h` stays a restart.

**What the simulator models.** A Basic Restart on every code
(programming mode off); the address to `FFFFh` for `02h`/`03h`; RES's
download-counter table; reserved codes answered with `02h`. What a Factory
Reset erases beyond that is *"implementation dependent"* and not modelled.

**Not decided.** `0701h` lists Master Reset as optional (Profiles Table
4.2). Whether `1.1.67` supports it is unknown, and a live Factory Reset
would need the option-C re-download afterwards (goal-commission K14).

**Lifted when.** An operator asks for a named code on a named device; the
scope joins the allowlist with a test; the bus monitor shows the probe,
the request, the answer and the device back where the table says.

## §140 The individual-address reset needs the pressed devices named, and its restart is unconfirmed

**Historical live run (2026-09-30, K13 on `1.1.67`).** MP §2.18
`NM_IndividualAddress_Reset` is implemented in
`knx_net::commissioning::individual_address_reset`: broadcast
`A_IndividualAddress_Write` `FFFFh`; `T_Connect`, Basic Restart and
`T_Disconnect` to `FFFFh` without waiting on any confirmation (the
procedure's own rule); broadcast read until nobody answers. The CLI is
`knx device reset-address <a.l.d>... --gateway <host:port> --confirm "I
confirm individual-address reset to 15.15.255"`. There is no HTTP route or
UI yet.

**Current public CLI policy (ADR-0058).** Confirmed CLI resets fail closed
*before opening a tunnel* until every affected device has a complete,
durable, verified pre-write recovery record. The old go is not standing
permission. The offline plan and protocol simulator remain; no HTTP/UI
route exists. `WriteScope::IndividualAddressReset` remains in the protocol
allowlist, but its phrase is not a backup. The caller-named-device guard
requires exactly that set on the first broadcast read, with no excluded
address; it cannot establish storage coverage. MP §2.18 itself resets
whoever is pressed.

**The restart is not evaluated.** On `1.1.67` the device ignored it: the
LED stayed on at `15.15.255` although nobody answered the closing read. The
CLI says the restart is unconfirmed and that a lit LED means programming
mode is still on; the remedy is `knx device program-address` (which gave
`1.1.67` back) or a button press.

**Ours, not MP §2.18's.** The expected-device guard; a first read before
anything is written; a cap of 3 rounds that names the devices still
answering; MP §2.3's 1 s read window.

**Not covered.** Several devices at once on hardware (simulator only); a bus
monitor trace of the sequence. **No HTTP/UI reset:** the earlier live run
used a separately created persistent backup, but the public CLI did not
durably verify all manufacturer-specific affected storage before sending
its broadcast. That historical dump cannot establish general reset recovery.
Do not reopen CLI or expose HTTP reset until an action-specific complete
pre-write backup and abort/recovery contract is verified (ADR-0058); see
`.ai/logs/2026-09-30_claude_commissioning-ui-status-handover.md`.

## 130. Application zoom is browser-verified, not native WebKitGTK-verified

**Limitation.** Whole-interface zoom is intentionally bounded to 80–150% in
10% steps; pane widths are bounded as documented in the user manual. The
interaction and responsive stack were exercised in system Chromium, not in a
running Tauri/WebKitGTK window. No desktop-rendering parity is claimed.

**Cause.** `zoom` participates in browser layout, so a change of rendering
engine can expose different viewport or font metrics. Headless browser tests
cannot prove native focus, resize or accessibility behaviour.

**Cost.** A Linux desktop installation may need a layout adjustment beyond
the Chromium evidence. The server still retains the raw, versioned settings
record; an out-of-range saved value is visually clamped rather than silently
overwritten or discarded.

**Lifted when.** A GUI-capable Linux run checks zoom shortcuts, pointer and
keyboard pane resize, hide/show and restart, and topology-device hover at
narrow, default and enlarged scales. No KNX hardware traffic is required.

## 137. A bus-monitor JSON capture is a retained window, not a complete trace

**Limitation.** The panel keeps at most 1000 telegram rows. Pause suspends
client polling, not the server's finite ring: a gateway burst may overwrite
older rows before Resume. Continued monitoring may also evict rows already
seen in the panel. The export records `serverDroppedBefore` and
`clientPrunedCount` separately, but cannot recover either category of lost
telegram. Statistics cover retained real rows only, not all traffic or
filtered table rows. Browser and desktop exports refuse a document over
16 MiB. Native WebKitGTK layout and save-dialog interaction have not been
exercised for this feature.

**Cause.** Both the server ring and browser capture are deliberately
bounded; a local snapshot cannot synthesize data that was never retained.
The native writer validates a bounded JSON document and writes atomically,
but automated tests cannot prove the running desktop dialog on every Linux
window system.

**Cost.** Use the exported JSON for a diagnostic window, not a full audit or
permanent bus history. The file contains source/destination addresses and
payloads; handle it as private installation data. A long paused period can
produce a visible server gap even though the session never disconnected.

**Lifted when.** Full-history capture would require a separately specified,
resource-bounded streaming/storage workflow and its own privacy policy; it
is not an extension of this local snapshot. A GUI-capable Linux desktop
check can independently verify the dialog and responsive layout. Live-bus
acceptance remains separate from the mock/simulator evidence here.

## Download lifecycle history admission — SAFE-03 / AUDIT-01 evidence boundary

**Limitation.** The bounded lifecycle is integrated and published as `1c5dec07`,
not full SAFE-03/AUDIT-01. Actual merged-source renewed125/0/0, ordinary3022/0
(with176 ignored/unexecuted), Web typecheck/build and Vitest1739/98 passed.
All26 command stages kept source32/public Rust-Cargo369/tracked Web unchanged,
actual leases, initially fresh target and original private commitments/no raw.
Three positive prerequisite phases precede a retained namespace setup refusal
(exit74 before input discovery/Cargo); corrected17-phase continuation is a
separate exact dispatch. This refuses overwriting historical receipts, not a
product test failure. In-session integrated review is not independent approval.
See the [permanent receipt](evidence/commission-download-lifecycle-offline-2026-10-04.json).
`PARTIAL_BACKEND`/proposed ADR0067 and broader owner/hardware/ETS/crash/recovery/
caller/Web-adoption limitations remain. Ordinary/scoped counts overlap; ignored
tests were not accepted. Earlier local controls/counts below are historical,
not retroactive mutations on the merged tree. Earlier integrated/publication
pending notices below are superseded only at this bounded package scope.

Pre-integration candidate acceptance 2026-10-04 was125 distinct/0 failed/0 ignored:
full Store94 (including History13), OneShot18 (including one25-case malformed
download leaf), ten actual offline workers and three public HTTP history cases.
All16 phases used the declared source32/HEAD/each actual dual lease; all-target
server/store Clippy and actual worker/API integration rebuilds passed with the
unchanged original input commitment/no private raw retained. The declared scope
is not every repository file; reviewed Rust diffs/owner inventory are separate.
Five production-source controls for repeated terminal result, late send intent,
swallowed migration COMMIT error, unknown download fields and zero session each
compiled then failed at the intended named assertion; original bytes restored
in finally while leased and canonical public112 bookend passed (not added again).
The synthetic malformed persisted download matrix refuses rather than returns
a partial page, latches unavailable, refuses new downloads and preserves bytes,
rows/identity/cursor/sidecar absence. In-session review is not independent goal
approval. The nonempty legacy read-lock case witnesses dirty
metadata journal/refused finalization and original main/version/docs/keys/counter
preservation. This is local bounded evidence, not full crash/power-loss restoration,
integrated release or full SAFE03/AUDIT01 closure. The wider storage-unit gate is
now green; integrated review/acceptance and authorized publication remain pending.
Earlier source43/widened source124 and snapshots below are
historical and not added to current totals or retroactively broadened.
Its bounded offline guard acceptance is30/0/0 on one source snapshot:11 storage,
17 OneShotLog and two named simulated HTTP tests. The24 empty/nonempty regular
marker combinations are one storage test, not additional Rust leaves. Directory,
live/broken-link coverage subsequently passed12/0/0 on its own test-only snapshot,
including18 entry combinations in a second storage leaf. On a separate corrected
test-only snapshot, actual keeper directory-blocker failure and positive terminal
worker passed1/0/0 each after rebuild/frozen32/dual leases/originals unchanged/raw
discarded. The blocker proves no mutation/no retained backup, failed/no worker,
one cleanup and successful history query with actual durable failed/no/no-backup/
no-intent/cleanup row. The initial wrong test enum expectation and diagnostic101
are preserved; configured alone is not health proof and production was unchanged.
This bounded blocker is not acceptance for every keeper failure or restoration.

Actual-worker error/panic, pending-cleanup runtime shutdown, synthetic terminal-
recording and cleanup-recording refusal plus default worker have now each passed
1/0/0 after rebuild on the final owned frozen32 snapshot, alongside Store12/0/0 and
OneShot17/0/0 (35 actual leaves/every dual lease/originals unchanged/raw discarded).
Witnessed result/backup survives cleanup faults; pending cleanup interruption
records unknown cleanup without rewriting terminal identity/timestamp. Recording
refusal latches unavailable history/new-start503 before contact and preserves the
injected baseline/no sidecars; unavailable history does not prove terminal storage.
Production unchanged by these test-only extensions, no crash or hardware guarantee.
Subsequent actual midwrite future-drop case plus those six worker controls and
Store12/OneShot17 passed36/0/0 on a new frozen32 source after integration rebuild/
every dual lease/originals unchanged/raw discarded. Backup/intent retained; live
failed/partially is distinct from durable unknown/null written/restart/unknown
cleanup, identity preserved/no added frame/no successful disconnect claim. This
does not prove hard-kill/power-loss survival, rollback or complete restoration;
negative source mutants/version-fault closure/full integrated review remain open.
That older snapshot's three scoped public production controls and reader-blocked
metadata finalization boundary are now tested as above; broader version/fault/
restart closure and full integrated review are still not claimed.

**Policy and cost.** This owned rollback/DELETE history refuses an existing -wal,
-shm or -journal entry before creating a main sentinel or opening SQLite, even
when the main header is1/1 or absent. Unknown entries are not automatically
deleted, truncated, followed or recovered. Store availability can therefore fail
closed on leftover files; deleting them blindly is not a supported remedy.

**Not proved.** Metadata permission-error and hostile concurrent path-replacement
branches, full backup contents/restoration, all terminal/cleanup/abort/restart faults,
crash/power-loss durability, hardware/vendor/ETS compatibility and final integrated
release acceptance. The simulated header2/2 fault is not an authentic WAL capture.
See ADR-0067 and the maintained commissioning status/owner ledger for disposition.
## §149 `knx products ingest` matches the `.knxprod` extension case-sensitively

**Scoped resolution published362fec24 (2026-10-03):** The scoped AR06P CLI correction
uses exact ASCII case-independent extension comparison, with five synthetic CLI
regressions, four named compiled mutation failures, public workspace/build
gates and same-release-profile baseline RED/candidate GREEN verification.
`.KNXPROJ` remains project import; case-independent pre-destination legacy
refusal remains unchanged. Full unpinned853 original-filename measurement
is independently accepted on the same release profile:644→687 installs,
Hager/Berker2→45;43 upper-case packages admitted, two ZIP-limit and one
evidence-item refusal remain explicit/atomic. Originals and retained successful
blobs verified unchanged; no private raw/item logs. Normal U18 merge dd5a350c
is independently actual16 accepted: Rust3000/0/165, Web1702, Chromium82,
17 bindings/704 committed-exact inputs. All285 CLI build/test inputs equal
the measured producer. Delivery362fec24 live/fetched/local refs equal0/0,
source704 and ten owned artifacts byte-exact; five acceptance Markdown gates
pass. This lifts only the case-sensitive CLI dispatch limitation, not §150–153
or full product/ETS/commissioning compatibility.

**Historical baseline observed 2026-10-03** (PRODUCT_DATABASE_CORPUS §Public crawler corpus run).
`run_products_ingest` (`apps/knx-cli/src/main.rs`) routes a file to
`knx_productdb::install_package` only when its extension is exactly
`knxprod` or `vd2`. Hager and Berker publish their product databases as
`*.KNXPROD`. Those 46 files fall through to the `.knxproj` project importer
and fail with the misleading `no P-*.signature entry; cannot determine the
project part`. The same bytes under a lowercase name install 43 of 46 (the
other 3 hit §151/§152).

- Only the CLI is affected. The server install route calls
  `install_package` directly with no extension switch, and the web file
  picker's `accept=".knxprod"` matches case-insensitively per HTML.
- `install_package` itself already compares legacy extensions
  case-insensitively (`.vd3`–`.vd5`, `.pr3`–`.pr5`), so the two entry
  points disagree.
- **Historical workaround (older CLI builds):** rename or symlink to lowercase `.knxprod`.
- **Lifted in the scoped delivery above:** exact ASCII case-independent routing
  with uppercase/mixed-case, project negative-control, legacy-refusal, retained
  byte/idempotence regressions and independently compiled routing mutants.

## §150 A product with nested `ModuleDef`s crashes the install with a database constraint error

**Scoped storage fix delivered 2026-10-03 23:05 CEST.** Actual integrated
cbe9952f passed fresh public16 and the same authorized private baseline RED/
candidate GREEN; code published as1b215d51, fetched/live refs equal. An incoming
foreign stats-only commit was retained byte-exact, source706 unchanged, five
fresh publication doc gates pass. Full853 measured installed687→688 with one
constraint refusal→installed and unchanged other categories; all original
hashes independently checked. Fourteen owned build/snapshot/browser directories
removed; closing metadata and final checkout hygiene pending. Lift only this
fresh-install storage/key-scoping bug. R-MODULE-04 runtime/allocation/parameter
semantics, new limits/grammar, full vendor/ETS compatibility and automatic repair
of already-admitted mis-scoped catalog rows remain outside this fix.

**Full offline/public candidate evidence accepted 2026-10-03 22:35 CEST;
integrated acceptance/publication/cleanup pending, not yet lifted.** Commit
a2aa4b7 has fresh complete public16: Rust153 blocks/3009-0-166, ignored
inventory166, Web1702, Chromium82 and shadow bindings17 token-exact. The first
binding helper API rejection is retained separately, not a product/test failure.
Fresh same-profile Release pair12 confirms old-parser named storage RED,
candidate7 GREEN and CLI5 controls GREEN on both sides. Full853 original-name
CLI pair independently reconciled: baseline687 installed/candidate688, exactly
one database-constraint refusal becomes installed;147 namespace,15 ZIP-limit,
two evidence-item-limit and one invalid-ZIP refusals unchanged. No installed-
to-refused regression. Original manifest/all853 files independently rehashed,
successful package bytes/source-blob hashes verified, four catalog/package
tables empty after refusal, temporary inputs/databases gone; aggregate-only
receipts, no private item/raw output. No new limits or runtime semantics.

**Storage candidate verified 2026-10-03 20:56 CEST; not yet delivered/lifted.**
Synthetic named UNIQUE-constraint RED confirms that a single mutable definition
identity loses its enclosing scope at an inner End; Empty also clears it.
The candidate stores lexical definition identity and argument position on a
stack, preserving existing keys, source ownership and atomic transactions.
Nine public focused tests and ProductDB625/0/25 pass with strict Clippy;
three compiled mutants have five named failures, plus an exact committed
pre-fix baseline RED. One authorized private nested-package pair is baseline
RED/candidate GREEN, with independent lexical/stored scope-count agreement,
original-ZIP/retained-member equality, successful retry and unchanged originals.
No private raw/item records survive. Full853 CLI measurement, broad/current-
upstream gates, publication/readback and cleanup remain pending. This does not
claim R-MODULE-04 runtime/allocation semantics, new nested diagnostic XPath
fidelity or automatic repair of previously admitted mis-scoped catalog rows.
The existing idempotence policy still skips stored trees; eligible retained-
source replay is synthetically verified, not a new database migration.

**Observed 2026-10-03.** MDT `RF-TAL55Bx0x-01S_MDT_KP_V12.knxprod`
(scheme 20, program `M-0083_A-00F2-12-05C4`) fails in a shared and in a
fresh database with `UNIQUE constraint failed: dynamic_node.program_id,
dynamic_node.module_def_id, dynamic_node.node_id`. The whole package is
rolled back, so no partial rows are written, but nothing of it can be used.

- The program declares 19 `ModuleDef`s, 11 of them nested inside another
  `ModuleDef` (depth 2). It is the only one of 97 crawled MDT packages with
  nested `ModuleDef`s, and every other MDT package installs.
- **Hypothesis (unverified, no code changed):** `dynamic/parse.rs` clears
  `module_def_id` at the end of a `ModuleDef` (around line 317). After an
  inner definition ends, the outer one's remaining `Dynamic` content lands
  under the wrong `(program_id, module_def_id)` key, where the per-key
  monotonic `node_id` then collides.
- This contradicts the earlier note that "the installed corpus measures
  zero products that actually nest" (GAP_ANALYSIS_ETS A3). That note
  remains true for `OriginalData`, but real downloadable products do nest. A full scan of the 852 crawled ZIP
 packages found this one package as the *only* nested example, which makes
 it the sole real R-MODULE-04 sample so far.
 - Planned as alpha package AR06P (`alpha-release-goal.md`), P1.
- **Lifted when** nested `ModuleDef`s parse into correctly scoped
  `dynamic_node` rows, proven by a synthetic nested fixture plus this
  package.

## §151 Real manufacturer packages exceed the product-ZIP size limits

**Bounded resource research verified 2026-10-03 22:02 UTC, base5540dcac.**
All853 original hashes/manifest rechecked; declared-size selection15 is seven
member-only, five total-only and three both (eight total/ten member violations).
The extra two member-only cases follow KL149 dispatch admission. A scratch-only
fixed member256MiB/expanded4GiB variant, compressed256MiB and other706 inputs
unchanged, installs14 and explicitly refuses one unsupported namespace;
production baseline refuses15 atomically. Same Release profile, fresh per-side
DBs,16 registered public hostile controls/six helper controls verified. Raised
cohort maxima:789976KiB RSS,221.55s ingest,7556988928-byte DB; total638.04s and
21970833408 DB bytes. Original/retained archive hashes and refusal empty tables
verified, private copies/DBs gone, no private raw/per-item vectors persisted.
This is measured admission, not runtime/ETS compatibility. Production bounds
remain unchanged: a blanket raise is not accepted by these corpus figures alone.
The HTTP catalog route still performs synchronous install while holding the
product DB mutex; CLI timings do not prove HTTP latency/progress/cancellation
acceptance. Reconcile caller ownership/resource guards and add cap-boundary
hostile regressions before choosing a smaller documented bound or streaming.

**Observed 2026-10-03.** `MAX_EXPANDED_SIZE` (256 MiB) and
`MAX_MEMBER_SIZE` (64 MiB) in `knx-productdb/src/package.rs` refuse 13 of
853 crawled files, plus 2 more Hager packages once §149 is worked around.

- **Expanded total > 256 MiB (8):** complete manufacturer bundles, 295 MiB
  to 2,740 MiB expanded. These include Siemens' complete
  `Siemens_HVAC_All_PDB_Oct_2023_ETS5_ETS6.knxprod` (1,006 MiB, which is
  Siemens' *only* current download) and ABB's
  `IBUS_ETS5_{ABB,BJE}_XX_V24-12-20_…` all-products bundles (2,740 and
  1,700 MiB).
- **Single member > 64 MiB (5):** individual ABB device packages whose
  application XML is 65–139 MiB (e.g. `DGS_264511_…`, `6197_46_…`).
- The refusal is clean and correct for the current limits. The limits
  remain a deliberate denial-of-service bound and are not a defect in
  themselves. But Siemens' entire public offering is currently unusable,
  and so are several real single-device packages.
- **Lifted when** the limits are revisited with measured memory/time costs,
  for example a streaming ingest or a per-package opt-in raise, while
  keeping a bound.

## §152 The XML evidence item limit refuses two real packages

**2026-10-04 candidate update (verified, not delivered):** exact original853 hashes and a
bounded raw-item census select the same two cases from four size-admitted
scheme14 packages/16 XML documents. Actual same-Release scratch pair proves
baseline2 atomic item refusals/observer2 installs, max802433 items and155281510
estimated evidence bytes, peak134552KiB RSS/max2.985s. Independent byte64MiB
would still refuse an item-only raise. A coupled1048576-item/256MiB candidate
retains depth1024, ZIP caps, namespaces and all-or-nothing reporting. Two
registered REDs, six new inclusive-boundary/late-no-partial tests and focused
GREEN and latest7f57abbb public18 are verified:Rust3028/0/176,Web1739,
Chromium82,source712/logs/Release binary exact. Earlier ed03cb85 bindings17/eight
negative controls and private2 retain their original producer identities.
Six owner Rust changes invalidated earlier440-input equivalence; fresh full853
on7f57abbb is independently reconciled:688 unchanged table-count installs,
163 unchanged diagnostic refusals,2 item-budget admissions→690 installs.
Both binaries/source/logs,original853 hashes/retained archives/atomic refusals
and private temp0 independently checked. Rejected rebuild-verifier attempt
stays rejected; corrected actual public18 retry passes. Scoped correction
published as2b2a267f7873137ccbf3d0a5052a541a76573d59 with exact live/fetched
refs and nine blobs; docs-only owner closure preserved/source712 unchanged.
Four own runtime directories removed; final closing metadata/hygiene separate. Original master-language64MiB/262144 limits
and retained-source classification are explicitly preserved. No limit removal,
data summary/truncation or compatibility
claim. Detailed producer identities/rejected verifier attempt/scope are in
PRODUCT_DATABASE_CORPUS.md. This measured two-package limitation is resolved;
larger hostile inputs still receive explicit atomic bounded refusals.

**Observed 2026-10-03.** `MAX_EVIDENCE_ITEMS` (262,144,
`knx-productdb/src/parse/scheme_evidence.rs`) refuses ABB
`PS5604-KNX AC500.knxprod` and Hager `PS_TXA664D_V105_T5` (both scheme 14;
the Hager one measured after the §149 workaround) with `XML evidence exceeds
item limit 262144`. Reproduced in a fresh database. The refusal is atomic
and explicit, so no data is lost silently. **Lifted when** the evidence
budget is sized against measured real maxima, or evidence collection
degrades to a counted summary instead of refusing the package.

## §153 Master-data schemes 10 and 23 are refused for standalone `.knxprod`

**Research 2026-10-04,not admission.** Base575a2d1d/source712:bounded offline
census rechecks853 hashes/852 master documents/one explicit scan refusal;two
scheme23 packages have8 complete XML (2 each Master/Catalog/Hardware/
ApplicationProgram),no observed namespace mismatch/foreign elements/qualified
attributes. Fresh ProductDB631/0/25/28 blocks,strict Clippy/Release build passed.
Real unchanged original-name CLI has2 exact atomic namespace refusals;all853
originals independently rehashed after probe,temp0/no item records. Primary
project Schema23 v01.00.00 excludes full manufacturer semantics;retained-language
23 evidence is not typed product acceptance. See PRODUCT_SCHEME_23_RESEARCH.md.
No whitelist/namespace/ZIP/runtime changes and no scheme10 acceptance.

**Observed 2026-10-03.** Of 853 crawled files, 146 use namespace
`http://knx.org/xml/project/10` (145 ABB plus 1 ABB bundle that also hits
§151) and 2 use `…/project/23`: ABB `LKS_43_VD-TP_XX_V1-0_…_Rev_A` and MDT
`SCN-LK001-03S_MDT_KP_V10_ETS6`. Both schemes are refused with
`unsupported product master namespace`, consistent with the documented
accepted set (11, 12, 13, 14, 20, exact 21).

- Scheme 10 is the largest single refusal class. ABB still offers that
  many ETS4-era packages.
- Scheme 23 has so far been named only for `.knxproj` projects
  (COMPATIBILITY.md). It now also appears in current ETS6 product
  downloads, and the corpus documents' "schemes 15–19 and 22 remain
  unmeasured" list did not mention it.
- **Lifted when** each scheme is admitted with grammar evidence like
  schemes 12–14/21 were. Until then, the refusal is the intended behaviour.
