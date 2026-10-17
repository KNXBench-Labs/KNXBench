← Previous: [Known issues](known-issues.md) · [Manual index](README.md)

# Implementation status

## Workbench configuration (Alpha 13)

Navigation blocks fold independently; File actions have four labelled groups.
Device functions exposes supported manufacturer mode enums beside communication
objects; Parameters uses declared blocks and retains folded drafts. Supported
parameter edits create missing positively active unscoped objects in one durable,
undoable batch without replacing existing links/overrides. Missing module mappings
refuse, and ambiguous DPTs are not selected automatically. No new hardware or ETS
compatibility is implied. These changes are in Alpha 13, not a
retroactive update to an Alpha 12 AppImage. [Operating instructions](user-guide/05-devices-and-products.md#device-functions-source-builds-after-alpha-12)
and [contract](../contracts/parameter-object-refresh.md).

This chapter is the honest inventory: what exists, what half exists, and what
does not exist at all, area by area across the whole application.

The detailed inventory retains its **8 October 2026** source audit and dated
follow-ups. The current public snapshot is [Alpha 13](../releases/alpha13.md),
published on 16 October 2026. [Source acceptance](../releases/alpha13-acceptance.md)
and [public artifact verification](../evidence/release-alpha13-2026-10-16.json)
pass independently; neither expands device admission.
The version number is where the project starts counting, not a claim that
anything has reached a finish line.

The [ideas/roadmap audit](../status/2026-10-08-ideas-roadmap-audit.md) checked
source `608a204b`, actual UI callers and recorded boundaries. An accepted
alpha boundary is not implemented functionality or an active work package.

<a id="project-versus-devices-p4-main-only-source"></a>

## Project versus devices (P4, Alpha 12)

Devices has explicitly confirmed single/filtered-batch read-only comparison,
progress/cancel, per-device failures and volatile tags. P3 content revisions
invalidate old readings; identity is checked before memory. The 0701 address
and association tables are decoded; all raw read ranges remain visible.
Named parameter rows show name/reference/bit metadata and separate project/device
values for supported numeric and text fields. Unknown, incomplete or overwritten
fields keep an explicit reason and all raw bytes. Negative signed project values
and new image types are not admitted. Both P4 packages are published in source on main (`17cfd5b7`);
combined full51/51 and corpus152/152+16/16 pass after the separate inherited
corpus-prerequisite correction. This is not an Alpha11 artifact update.
[Publication record](../evidence/p4-device-comparison-parameters-2026-10-15.json).
[Use and limits](user-guide/05-devices-and-products.md#comparing-the-project-with-devices-p4-source-build-after-alpha-11),
[contract](../contracts/device-comparison.md).

## Universal device-support evidence (15 October 2026 source)

The File menu's universal support workflow accepts wholly unknown devices,
selected project devices or a manually supplied local individual address.
Offline preview needs no project/catalogue/plan/gateway. Optional fixed read-only
recipes require separate target-specific consent; output is reviewed before a
separate local ZIP save and manual issue/mail handoff. The report distinguishes
unknown, established refusal/support, prior run scope and a user's functional
answer. Installation identifiers, configuration values and raw/vendor assets are
excluded; an optional bounded numeric source projection needs rights confirmation.
This does not enable System-2 programming or promote a device to Verified.
Implementation acceptance/publication is tracked in the delivery index; included in Alpha 12, not retroactively in an Alpha 11 binary.
[Contract](../contracts/universal-device-support.md).

## Diagnostic navigation selection (Alpha 12)

Selecting the active Bus monitor or Log sidebar entry keeps the same workspace,
including its tab/input or filter, rather than returning to the previous project
view. Select another view explicitly to leave. Settings and Diagnostics window
retain their separate overlay/window actions. No protocol or storage change.
[User guide](user-guide/01-user-interface.md#the-navigation-pane),
[contract](../contracts/diagnostic-sidebar-selection.md).

<a id="per-part-programming-marks-15-october-2026-source-main-only"></a>

## Per-part programming marks (Alpha 12)

Devices shows pending-only Adr/App/Par/GA chips, a Pending only filter and an
explorer dot. Properties lists each part and offers undoable, logged manual
marks. The existing loaded flags now follow edits in their same undo step;
server full/partial/address programming settles only the applicable parts,
without adding an undoable hardware operation. Interrupted writes mark attempted
parts pending; edits during a run do not gain false clean claims. Native flags
and recovered undo need no migration. After a later device write, recovered
snapshot undo conservatively invalidates older clean claims and can show extra
pending parts without proving a device difference. This is project belief, not
P4 live-device comparison; CLI/external work needs manual reconciliation.
[Use and limits](user-guide/05-devices-and-products.md#programming-marks-p3-source-build-after-alpha-11),
[ADR-0129](../adr/0129-per-part-pending-programming.md).

<a id="community-hall-of-fame-link-14-october-2026-source-main-only"></a>

## Community Hall of Fame link (Alpha 12)

About links the public GitHub document instead of bundling its JSON snapshot.
The Docker frontend no longer needs `docs/community` in its app-only build
stage. No board request is made until the user activates the link; there is no
local/offline participant list. Repository scoring and award automation are
unchanged. [User guide](user-guide/01-user-interface.md),
[contract](../contracts/community-recognition.md#about-dialog-issue-19-owner-amendment-14-october-2026).

<a id="active-task-status-shimmer-14-october-2026-source-main-only"></a>

## Active task status shimmer (Alpha 12)

Existing active-work text gains a slow monochrome sweep followed by a quiet
pause; real text, counters and progress bars remain. It stops for results,
errors or decisions and respects Motion Off, OS reduced motion, forced colours
and printing. Ordinary connection labels and busy action-button captions are
unchanged. Chromium is the acceptance target; native WebKitGTK/Orca and arbitrary
imported-theme contrast are not certified by this cosmetic change.
[Use and controls](user-guide/01-user-interface.md#active-task-status-text-14-october-2026-source-main-only),
[contract and boundaries](../contracts/task-status-shimmer.md).

<a id="pre-plan-programming-support-help-14-october-2026-source-main-only"></a>

## Pre-plan programming support help (Alpha 12)

The download tab shows a reason and next action before any plan or report exists,
with EN/DE instructions, the support-gap issue form and an unsent email draft.
Technical details remain local and copyable; links contain no project-derived
metadata. B15's pending context and readiness/status retry remain. This is not a
new encoder, device approval or release. [How to report](../contribution-intake/README.md#device-programming-refused-before-a-plan-exists).

## Help verify a device (13 October 2026 source)

The download tab now shows what a plan warns about in plain words, the assumptions it
rests on, and a hint on untested programs. After every download it offers **Help verify
this device**: steps, a checklist, a report file without names or addresses, and the
GitHub issue form or a mail to issue@knxbench.com to send it. Nothing is sent
automatically; a maintainer reviews each report before a program counts as verified.
Included in Alpha.11.

## Steadier notifications, Flow images and motion per category (13 October 2026 source)

Error cards stay until you have seen them, hold still under the pointer and no longer
replace each other; the Explorer's import counts open the matching Log entries. The
Flow view saves a PNG or a GIF clip of up to 30 seconds after asking; toasts,
achievements and the Flow each get their own motion setting that can only reduce
motion; four more achievements. Included in Alpha.11
([notifications](user-guide/09-settings-and-appearance.md#toast-notifications),
[Flow images](user-guide/07-bus-and-interfaces.md#saving-the-flow-as-an-image-since-alpha-11),
[limits](../KNOWN_LIMITATIONS.md#ported-ux-feedback-2026-10-13)).

## Original import reports, retained payloads and read-only project facts (12 October 2026 source)

Native files keep the complete original import report, so a reopened project shows
**Historical import findings** with the original counts instead of "unavailable".
The Overview also lists retained attachments and extension files (inert downloads)
and the modeled project information; **Export project…** asks before a native
download; the Properties Inspector shows a device's recorded load state. All of it is
read-only and contacts no device
([how to use it](user-guide/02-projects.md#review-the-complete-original-import-report),
[limits](../KNOWN_LIMITATIONS.md#ported-native-import-evidence-2026-10-12)). Included
in Alpha.10. Stored bus access and IP settings are shown read-only since Goal B3
(after Alpha 11), endpoints hidden until revealed
([contract](../contracts/stored-bus-access.md)).

## Group-address members and download preparation (12 October 2026 source)

A group address's members — communication-object links, not devices — can now be
added from the address: **Add member…** in all four presentations and in Properties,
choosing device, object and direction, with the existing link rule, duplicate check,
server refusals and one undo step per add. The **Download to device** tab chooses its
gateway from the start-up interface search (one answer filled in, several offered by
name and endpoint, typed addresses kept), the device editor opens it with that device
preselected, and a five-step list names what is still missing. No new protocol,
download capability or compatibility claim
([limits](../KNOWN_LIMITATIONS.md#group-address-members-and-download-preparation-2026-10-12)).
Included in Alpha.10.

## Catalogue updates with a preview (12 October 2026 source)

The catalogue's **Update with preview…** shows what a newer `.knxprod` would add or
change before anything is installed, backs up the product catalogue before
installing, and lists backups you can restore after confirming
([how to use it](user-guide/05-devices-and-products.md#updating-with-a-preview-since-alpha-10),
[limits](../KNOWN_LIMITATIONS.md#catalogue-updates-come-only-from-files-you-supply-2026-10-12)).
There is still no online catalogue: you bring the file. Included in Alpha.10.

## Checked parameter editors (12 October 2026 source)

Time, number, colour and picture parameters have an opt-in **Checked editor** that
reads the type from the imported product data: time values as an exact decimal
count with their declared unit, sliders where the manufacturer declared one, and
colour/picture values unchanged with a note
([how to use it](user-guide/05-devices-and-products.md#checked-editor-since-alpha-10),
[limits](../KNOWN_LIMITATIONS.md#checked-parameter-editors-in-the-device-editor-2026-10-12)).
Included in Alpha.10.

## Notes follow-ups (12 October 2026 source)

Notes can now be attached to communication objects; the target list comes from the
server and offers only targets a note can really use. In a browser, reloading or
closing the tab with an unapplied note or instruction draft asks first, and the
note review names its action in your language
([how to use it](user-guide/01-user-interface.md#project-notes-since-alpha-10),
[limits](../KNOWN_LIMITATIONS.md#project-notes-workbench-host-2026-10-12)).
Included in Alpha.11.

## Installation instructions in the device editor (12 October 2026 source)

The device editor's **Installation instructions** tab shows the hints a device
brought from an imported ETS project and lets you replace them with formatted text
through a preview and an explicit Apply; the imported original is kept. Drafts of
notes and instructions now survive saving the project
([how to use it](user-guide/05-devices-and-products.md#installation-instructions-since-alpha-10),
[limits](../KNOWN_LIMITATIONS.md#installation-instructions-tab-and-save-proof-drafts-2026-10-12)).
Included in Alpha.10. Since Goal B4 (after Alpha 11) devices added in KNXBench get
instructions too, without an invented original.

## Project notes in the workbench (12 October 2026 source)

The **Notes** view opens the project's engineering notebook: literal plain-text
notes on the project or one of its parts, changed only through a reviewed preview
and an explicit confirmation, with Undo/Redo and the usual Save. Drafts survive
view switches; leaving the project with a draft asks first
([how to use it](user-guide/01-user-interface.md#project-notes-since-alpha-10),
[limits](../KNOWN_LIMITATIONS.md#project-notes-workbench-host-2026-10-12)).
Included in Alpha.10.

## Clearer dialogs and an actionable project status (12 October 2026 source)

The unsaved-changes dialogs (Quit, Open/Import, New project) label all three
answers by what they do — **Keep editing**, **Discard …**, **Save and …** — with
explicit colours in every palette and a content-sized panel. A save-first that you
withdraw while it runs no longer opens the other project afterwards. About is a
compact, centred card that keeps version, licence, update notice and legal text.
The Overview's project status uses three labelled cards; non-zero import counts open
the matching session-log entries ([limits](../KNOWN_LIMITATIONS.md#project-status-import-links-and-unsaved-changes-dialogs-2026-10-12)).
Included in Alpha.10.

## Name-first DPTs and monitor selection (12 October 2026 source)

Current source reuses the 454-ID EN/DE display vocabulary for **Name (main.sub)**
in object and group-address views and monitor decoded values. The monitor send
form now offers named DPT choices, with exact current aliases/unknowns retained
and no send on selection. Raw object references are disclosed on demand; domain,
API, project files and codec support are unchanged. Included in Alpha.10.
[Preservation and verification](../contracts/dpt-selection.md).

## Release notifications (11 October 2026 source)

The workbench can announce newer GitHub releases at startup and hourly while
visible, with release details and manual checking in File → About KNXBench.
Settings → Updates disables automatic checks; no project data is transmitted.
Stable builds exclude prereleases; alpha/beta builds include newer prereleases.
No automatic installation or upgrade-compatibility approval is implied. Offline
and rate-limited checks leave editing usable. Included in Alpha.10; Alpha.9 does not have it. [Contract and privacy](../contracts/RELEASE_NOTIFICATIONS.md),
[verified source scope](../evidence/release-notifications-2026-10-11.json).

## Read-only diagnostic export (source after Alpha 13)

One-device identity export in the web UI and CLI, simulator-verified only; no
memory reads and no hardware compatibility claim yet
([contract](../contracts/device-diagnostic-export.md)).

<a id="alpha-12-release-candidate-boundary-15-october-2026"></a>

## Alpha 12 release boundary (15 October 2026)

[Alpha 12](../releases/alpha12.md) includes the delivered post-Alpha 11 work:
portable native product sources, per-part programming marks, read-only comparison,
universal support reports, Functions, revision-bound workbench writes and the
reported UI repairs. Native/model schema **16** and product schema **23** require
independent pre-upgrade project/catalogue backups. Alpha 11 and earlier refuse
upgraded native files. Public AppImage/MCP downloads match their checksums;
Docker version/latest identify the same amd64/arm64 image set and the frozen
source. An isolated HTTPS/auth/save/reopen and authentic predecessor-file
migration probe passes; the user's running container was not replaced. The
live website matches the accepted build, including the new stills and unchanged
Story. AppImage Broadway/embedded-server startup is verified, not FUSE/native
visual/EGL/Orca feature acceptance. No new hardware compatibility is implied.
[Publication verification](../evidence/release-alpha12-2026-10-15.json).

## Alpha.11 release boundary (13 October 2026)

[`v0.1.0-alpha.11`](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.11)
packages frozen source `758a112b` (main cutoff `2a43ad5c`): everything above marked
"Included in Alpha.11", the group-address picker (#8), tree indentation (#9), named
gateway busy/refused states and offline readiness marks in the download tab (#10),
and the read-only byte-order probe with its named download warning (KL-170 stays
LATER: little-endian downloads are still refused). Schema stays **14**; no migration
from Alpha.10. Not exercised in the native WebKitGTK window; #10 was not tried
against a real gateway.
[Release notes](../releases/alpha11.md) · [artifact verification](../evidence/release-alpha11-2026-10-13.json).

## Alpha.10 release boundary (12 October 2026)

[`v0.1.0-alpha.10`](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.10)
packages frozen source `73420e13` (main cutoff `4a0294b8`): everything above marked
"Included in Alpha.10", plus the Mesa AppImage fix (#6) and ETS 5.7 import (#7).
Native/model **schema 14** requires independent native/Docker backups; Alpha.9 and
earlier refuse upgraded files. Functions/Sites, the remaining UX-feedback package
and colour/picture parameter editors are not included.
[Release notes](../releases/alpha10.md) · [artifact verification](../evidence/release-alpha10-2026-10-12.json).

## Alpha.9 release boundary (11 October 2026)

[`v0.1.0-alpha.9`](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.9)
packages frozen source `5b21323e`, including published import/native-history,
DPT, installation navigation, login/message and download-layout repairs.
Native/model **schema13** requires independent native/Docker backups; Alpha.8
and earlier refuse upgraded files. Instruction/parameter components remain
unmounted production-workbench foundations, not advertised usable editors
(the instruction editor was mounted in later source, see above).
[Release notes](../releases/alpha9.md) and [artifact verification](../evidence/release-alpha9-2026-10-11.json)
separate the tag from later `main`, browser fixtures from native UI evidence,
and preservation from ETS/hardware certification. Later parallel/local packages
are not included, and existing containers are not automatically replaced.

## Device-download layout (2026-10-11 local source)

The local layout correction keeps the four download-scope radios compact,
places their labels beside them and separates the scope from the device and
gateway fields. EN/DE production-browser checks cover 400/1440 px in Graphite,
Porcelain and CRT, including keyboard selection. The change is presentation
only; download planning and safety confirmations are unchanged. The source
repair is now published on `main` as merge `0dae7c93`, but is not included
in the existing Alpha.8 release or running containers.
[Source publication](../IMPLEMENTATION_STATUS.md#2026-10-11--device-download-layout-published-on-main);
[initial local acceptance](../IMPLEMENTATION_STATUS.md#2026-10-11--local-device-download-configuration-layout-repair).

## Installation-node navigation (11 October 2026, local source)

Unnamed imported installations now have translated display-only labels with
stable IDs. Their Explorer labels select Properties with a name editor and an
explanation; arrows still toggle the subtree. This reuses the existing installation
rename command and native history/storage, with no new model, schema or bus behavior.
Local acceptance is recorded in [the focused contract](../contracts/installation-node.md);
[verified main source publication](../evidence/installation-node-publication-2026-10-11.json)
supersedes the local-only source boundary, not its historical measurements.
This is not an Alpha.8 release or live deployment claim.

## Alpha.8 release boundary

Alpha.8 packages selected-line/device import, schema-23 source/ZIP integrity
repairs, bounded payload content/unverified-role evidence, own stored module-instance
values in read-only views and the **1024 MiB**
expanded project budget with localized refusals. It requires native/model
**schema12**; back up before upgrading, because Alpha.7 and earlier refuse the
new files. [Release notes](../releases/alpha8.md) and the source-bound publication
[receipt](../evidence/release-alpha8-2026-10-10.json) separate downloaded artifacts from later `main`, synthetic/private
evidence from full ETS semantics, and packaging from hardware certification.
Unmerged Functions/Sites and remaining research work are not included.

## Alpha.7 release boundary

`v0.1.0-alpha.7` bundles the verified changes since Alpha.6: persistent native
history/schema11, object-table UX, rename support, offline AP1 diagnostics,
legacy improvements, the header badge and browser upload/return fixes.
The detailed entries below retain their original source acceptance boundaries;
Alpha.6 still does not contain those later additions. Back up native projects
before upgrading: old builds refuse schema11. No new whole-ETS/hardware claim.

<a id="project-archive-budget-2026-10-10-local-source"></a>

## Project archive budget (2026-10-10 source integration)

The owner-requested source change raises the expanded `.knxproj` archive budget
from 512 MiB to **1024 MiB**. The 64 MiB limit per expanded member and all
archive identity, layout and bounded-read checks remain in place. This does not
raise HTTP upload limits or guarantee peak memory usage. The size refusal is
shown plainly in both the popup and persistent load-failure banner, for example:
“Die Datei ist entpackt mit 1280 MiB zu groß. Das Limit liegt bei 1024 MiB.”
The displayed limit comes from the server, including an older server's 512 MiB
limit. MiB accurately names the binary unit used by the importer, rather than
compressed file size. Unknown technical messages still pass through unchanged.
The verified source change is published on `main`, separately from any new
release/deployment; Alpha.7 artifacts retain their existing limits. Alpha.8
packages the new budget.
[Verification](../IMPLEMENTATION_STATUS.md#2026-10-10--archive-budgetreadable-refusals-main-integration-accepted)
· [Import limits](../IMPORT_EXPORT.md).

## Local login, notification details and larger inputs (2026-10-11)

The isolated login/feedback package reuses the KNXBench logo and shows the built
frontend version before authentication. Toast message actions navigate by exact
local ID into the separately disclosed browser notification history; server/UI
messages have raw details, exact clipboard copying and a selected-text fallback.
Backend JSON export remains v1/server-only. Raw analysis/evidence sources and
streamed file uploads admit 1024 MiB; contribution expanded inventory also
admits 1024 MiB. XML/member/manufacturer/selective-source/public-output safety
budgets are not blanket-raised. This is **local source**, not a released or
deployed image. [Contract](../contracts/LOGIN_FEEDBACK_AND_FILE_LIMITS.md).

## Docker upload and browser return fixes (2026-10-09 source)

The source fixes for GitHub #1/#2 raise the streamed upload file ceiling to
**256 MiB**, with a separate 16 KiB multipart-envelope allowance, without
changing ZIP expansion/import limits. Diagnostics and Flow's explicit browser
return navigate this tab to the editor and resume the current server project
with a read-only request; native return still focuses the existing main window.
A separate browser editor is not closed and transient selection is not copied.
These fixes are **not in the published alpha.6 artifacts**. See the
[behavioral contract](../contracts/DOCKER_UPLOAD_AND_RETURN.md).

## Source integration: selected lines/devices

The 10 October 2026 source package adds a File-menu selector and the matching
CLI for a previewed merge into the open project. Shared application logic maps
required dependencies, refuses conflicts/stale confirmations and creates one
native undo step. Target settings/identity remain owned by the open project.
Retained complete source evidence can include unselected private data and
survives Undo; explicit consent is required. [Source integration acceptance](../status/2026-10-10-import-expansion-integration.md)
records the source separately; Alpha.8 packages the feature without changing the
existing Alpha.7 artifacts. [Workflow](../SELECTIVE_IMPORT.md) ·
[local verification](../status/2026-10-10-import-expansion-verification.md).
The authorized historical-export harness lacks its real export set; cvexc
analysis does not activate device comparison/download rules. Native/Orca and
real ETS/hardware evidence remain unverified.

## Published main source: localized DPT dropdown and field provenance

The 2026-10-11 [display/selection extension](../contracts/dpt-selection.md)
replaces the CO DPT free-text field with a native dropdown containing 454
versioned EN/DE names, resolves primary table/filter/search labels by UI language
and explains provenance directly at the DPT/description heading. Unknown/raw
aliases and main-only values stay exact options; program defaults stay display-only.
Ordinary Web2625/0 in166 files, production build/types, complete intercepted
Chromium252/0 and built Chromium23/0 are verified; native ArrowDown/Enter1/0
also commits exactly the selected canonical DPT. All browser cases are offline.
Owner-authorized no-ff main integration is accepted on actual `e57e3a09ff34`:
Web2648/0, Rust3946/0,
full Chromium274/0 and built23/0.
[Source/remote-boundary receipt](../evidence/dpt-selection-publication-2026-10-11.json).
**Not in Alpha.8 or the live container**; no core/database/API/protocol or
device/codec compatibility extension. Earlier local counts above are historical.

## Communication-object table: accepted main-delivery source

The 2026-10-09 owner-approved local UI package adds explicit column headings,
channel/flat views, search, status/link/effective-DPT filters and stable numeric/
text sorting in a single-device editor. Draft/refusal/pending state survives view
changes; counts exclude open editing exceptions. Core/storage/API/protocol are
unchanged. The owner authorized main publication after fresh combined-source
acceptance; the [delivery receipt](../evidence/communication-objects-publication-2026-10-09.json)
records the publication boundary. This is **not in alpha.6**; it is bundled in Alpha.7. The
[contract and acceptance receipt](../COMMUNICATION_OBJECT_TABLE.md) distinguish
synthetic/built-browser checks from native/Orca, real-bus and ETS compatibility.

## New on main: native project history

The user-authorized 2026-10-09 package implements restart-safe native undo/redo,
named and automatic saved-state versions, and confirmed restore with a safety
version. Unsaved native work remains marked unsaved while its recovery journal
is durable. Before first Save As history is session-only. Versions are inside
the same file, not independent disaster backups. Native schema v11 needs a new
build; this source feature is **not** in the published alpha.6 snapshot.
[Contract](../PROJECT_HISTORY.md) · [verification](../status/2026-10-09-project-history-verification.md).
Native/Orca, physical power-loss and hardware/ETS evidence remain separate;
Alpha.7 is the first release containing this history feature.

## New on main: Devices navigation

The owner-approved Devices list/editor package was implemented in the isolated
`devices-navigation-20261008` package and is on `main` since 2026-10-09; it is
part of the `v0.1.0-alpha.6` release and its Docker Hub image. It adds
the navigation/palette entry, a filterable/sortable canonical device table,
central editor with app-local Back, preserved list state and scoped device links.
Product-only metadata uses a read-only snapshot-bound batch; unavailable data
is named, not inferred. [Contract and verification](../DEVICE_NAVIGATION.md).
The older alpha.5 AppImage does not contain it. Native/Orca/hardware acceptance,
URL/browser history, persistent state, grouping and project-diff/log-prose links
are not part of this package.

## How to read the tables

| Marker | Meaning |
| --- | --- |
| ✅ Implemented | Reachable and backed by named tests or a documented run at the stated scope; the evidence may be synthetic/offline |
| 🟡 Partial or experimental | Works for part of its scope, or works but has never been verified against a real-world sample |
| 🚧 In progress | Started, not usable end to end |
| ❌ Not implemented | Does not exist, or exists only somewhere a user cannot reach |

Two rules were applied while filling these tables in. A ✅ needs evidence
that can be pointed at — no marker was awarded for code that looks finished.
**Evidence is a separate axis:** synthetic tests, real-file/corpus runs,
real-gateway/device runs and native/assistive-technology checks prove different
things. A ✅ never upgrades one into another. Real-data counts below remain
scoped measurements, not universal compatibility or a newly repeated corpus run.
And where the choice was close, the row got 🟡, because "partial" is a
smaller lie than "done" when it turns out to be wrong.

[Supported and unsupported KNX/ETS functionality](reference/02-supported-and-unsupported.md)
carries the same markers at a finer grain for file formats, datapoint types,
the bus and KNX Secure; where this chapter gives one row, that chapter often
gives five, with the measured counts behind them.

---

## Project storage

| Capability | Status | Notes |
| --- | --- | --- |
| Native `.knxdb` project file (SQLite, store schema12 in Alpha.8; Alpha.7 uses v11 and Alpha.6 uses v10) | ✅ Implemented | Open, save, save as — [Projects](user-guide/02-projects.md) |
| Schema migration of an older project file, with a refusal rather than a guess when the file is newer than the build | ✅ Implemented | Migration tests per version step — [`docs/DATA_MODEL.md`](../DATA_MODEL.md) |
| Undo and redo across project edits | ✅ Implemented | Native-backed reversible edits survive restart with undo/redo; before first Save As it is session-local. Cannot undo bus writes — [history contract](../PROJECT_HISTORY.md) (local package, not alpha.6) |
| New-project wizard with an optional starting structure (areas/lines, building tree, main/middle group ranges, two presets) | ✅ Implemented | Applied with the new project in one step or refused without replacing anything — [Projects](user-guide/02-projects.md), [ADR-0093](../adr/0093-wizards-are-views-over-existing-commands.md) |
| Add-device wizard: product, placement (installation, line, building part), server preview of names and addresses, confirmed create | ✅ Implemented | Creation/placement are one undo step. Changed computed names/addresses cause stale-preview refusal; this is not a whole-project revision lock — [Devices and products](user-guide/05-devices-and-products.md#the-add-device-wizard), [ADR-0093](../adr/0093-wizards-are-views-over-existing-commands.md) |
| Provenance for modelled layered values | ✅ Implemented | `Override` distinguishes absent/empty/malformed/value, and `Resolved` carries the source layer. Not every ordinary project attribute is layered — [Data model](../DATA_MODEL.md), [ADR-0010](../adr/0010-per-attribute-override-representation.md) |
| Exporting the open project through the browser | ✅ Implemented | File → Export project… streams a newly serialized `.knxdb`; Save As still writes to the server's directory — [Projects](user-guide/02-projects.md) |
| Autosave for a project that already has a file | ✅ Implemented | Configurable interval, cancellation notice and reported failures — [Projects](user-guide/02-projects.md#saving) |
| Native project versions and persistent undo history | ✅ Included since Alpha 7 | Named/save/pre-restore versions, revision-bound confirmed restore, bounded atomic journal — [Projects](user-guide/02-projects.md#project-history-restart-safe-undo-and-versions). Not in alpha.6; keep independent copies against file/disk loss |

## ETS project import

| Capability | Status | Notes |
| --- | --- | --- |
| `.knxproj` at schema 11 (ETS4) and schema 21 | ✅ Implemented | Measured counts on real files — [Supported and unsupported](reference/02-supported-and-unsupported.md) |
| `.knxproj` at schema 23 | 🟡 Bounded evidence | Reference/private-original import and native `.knxdb` roundtrip/source reconciliation are evidenced; retained-uninterpreted information is not full module semantics. No ETS export roundtrip — [Supported and unsupported](reference/02-supported-and-unsupported.md) |
| `.knxproj` at schemas 12–14, 20, 22 | 🟡 Unverified sample coverage | Parser-family admission is not verification against real project files at those schemas — [Supported and unsupported](reference/02-supported-and-unsupported.md) |
| Import report with errors, warnings, unsupported constructs and conflicts | ✅ Implemented | Also available as JSON from the command line; since the 12 October source the complete original report is kept in native files |
| Preserving opaque source data and reporting unsupported constructs | ✅ Implemented | See the import report and [data-integrity contract](../IMPORT_EXPORT.md); retained bytes do not imply editable semantics |
| ZipCrypto-protected projects (ETS4/ETS5) | 🟡 Partial or experimental | Implemented and tested against synthetic fixtures only |
| AES-protected projects (ETS6) | ❌ Not implemented | Refused by name — [§13](../KNOWN_LIMITATIONS.md#13-password-protected-projects-zipcrypto-ets4ets5-is-decrypted-aes-ets6-is-still-refused) |
| Importing part of a project into an existing one | ❌ Not implemented | Import is whole-file — [`docs/GAP_ANALYSIS_ETS.md`](https://github.com/KNXBench-Labs/KNXBench/blob/a584007fc05a/docs/GAP_ANALYSIS_ETS.md), row C3 |

## ETS project export

| Capability | Status | Notes |
| --- | --- | --- |
| Writing a `.knxproj` at any schema version | ❌ Not implemented, deliberately | Withdrawn on 2026-09-20 with the writer, the `knx export` subcommand, the HTTP route and the button — [ADR-0028](../adr/0028-no-knxproj-export.md) |

Import is one-way. This table used to have five rows describing a writer that shipped
between 2026-09-10 and 2026-09-20; none of it exists now, and neither do the warnings it
raised. A project that has been imported stays in `.knxdb`.

## Product databases

**Support-gap reporting:** source builds include **File → Analyze support gaps…**
for own-instance, read-only analysis and an explicitly previewed evidence ZIP.
It does not change the active project or install into your product database.
See the [contribution guide](../contribution-intake/README.md) and
[evidence contract](../COMMUNITY_EVIDENCE.md). Analysis completion is not
verified ETS/device support, and submission remains manual.

**Local source extension (2026-10-09):** the same analysis displays ordered
`MV-07B0` `Load/ap1` procedure declarations with local provenance and unresolved
steps. It is not a materialized download plan and enables no additional bus
execution. The reduced evidence ZIP retains value-free findings/counts, not the
local source details. [Scope and evidence](../OFFLINE_PROCEDURE_RESOLUTION.md).
Main source delivery is [verified](../status/2026-10-09-offline-ap1-verification.md); this is not a packaged release.

| Capability | Status | Notes |
| --- | --- | --- |
| Product data ingested from inside a `.knxproj` | ✅ Implemented | The reference project's 4 manufacturers and 12 application programs ingest completely |
| A `.knxdb` carries its application programs (store schema 16) | ✅ Included in Alpha 12 | Saved, Save As and the browser export embed the imported manufacturer files (compressed, hash-checked); opening reinstalls missing ones into the product database, so a Docker rebuild or another computer keeps device parameters. An older file names what is missing and offers **Restore from the original .knxproj…** (hash-matched, project untouched, no password needed) — issue #21, [ADR-0128](../adr/0128-native-project-files-carry-their-manufacturer-files.md), [§183](../KNOWN_LIMITATIONS.md#183-a-knxdb-saved-before-store-schema-v16-does-not-carry-its-application-programs) |
| Standalone `.knxprod` at master data scheme 11 or 20 | ✅ Implemented | Four real files, content-addressed, idempotent re-install — [Devices and products](user-guide/05-devices-and-products.md) |
| Legacy ETS3 `.vd3`/`.vd4`/`.vd5` product databases, offline use (web, CLI) | 🟡 Partial or experimental | The real `.vd3` and `.vd4` import completely; the real Siemens `.vd5` (88 programs) imports except its `string`/`long enum` parameters; password dialog and one remembered password; download plans from the file's own load procedure (untested: never run on a device); no DPTs yet — [Devices and products](user-guide/05-devices-and-products.md#old-ets3-product-databases-vd3-vd4-vd5), [ADR-0094](../adr/0094-legacy-exim-product-files.md) |
| Standalone `.knxprod` at schemes 12, 13, 14 and exact-namespace 21 | 🟡 Partial or experimental | Synthetic tests and the passing read-only corpus matrix verify parser/persistence behavior, not full manufacturer semantics |
| Standalone `.knxprod` at exact namespaces 10 and 23 | 🟡 Scope-limited | Strict parser/persistence admission is delivered, not full manufacturer or project-module semantics — [ADR-0083](../adr/0083-admit-exact-product-scheme-10.md), [product namespace admission](../../crates/knx-productdb/src/package.rs) |
| Standalone `.knxprod` at schemes 15–19 and 22 | ❌ Not implemented | No observed standalone sample or verified namespace support |
| Encrypted `.knxprod` packages, and legacy `.vd2` files | ❌ Not implemented, deliberately | A permanent scope exclusion, refused as a typed error |
| Online catalog update from a manufacturer | ❌ Not implemented | Files are installed by hand — [`docs/GAP_ANALYSIS_ETS.md`](https://github.com/KNXBench-Labs/KNXBench/blob/a584007fc05a/docs/GAP_ANALYSIS_ETS.md), row C6 |
| Devices configured by a manufacturer plug-in | ❌ Not implemented | No plug-in host exists and none is planned — [§6](../KNOWN_LIMITATIONS.md#6-devices-behind-vendor-plug-in-dlls) |
| Package/version identity inspection | 🟡 Bounded implementation | Source/candidate identity is recorded; a version pin/selector is not implemented. First-installed normalized rows remain the winner — [§135](../KNOWN_LIMITATIONS.md) |
| Manufacturer signature verification | ❌ Not implemented | Signature members are retained and labelled, not cryptographically verified — [§85](../KNOWN_LIMITATIONS.md) |

## Topology and buildings

| Capability | Status | Notes |
| --- | --- | --- |
| Areas, lines and individual addresses: read and edit | ✅ Implemented | [Buildings and topology](user-guide/03-buildings-and-topology.md) |
| Buildings, floors, rooms and building parts: read and edit | ✅ Implemented | 22 building parts in the reference project |
| Creating a project from scratch in the interface | ✅ Implemented | Browser-verified — [§83](../KNOWN_LIMITATIONS.md#83-the-from-scratch-launcher-is-browser-verified--resolved-2026-09-16-goal-task-17) |
| Five additional documented space types kept distinct on import | ✅ Implemented | `Stairway`, `RoomPart`, `Area`, `Ground` and `Segment` survive import and native save/load; synthetic coverage, with the Schema23 vocabulary inconsistency documented in [§89](../KNOWN_LIMITATIONS.md#89-five-documented-spacetype-values-are-coarsened-to-buildingpart-on-import) |
| Device-instance name editing in editor/Properties and F2/context menu | ✅ Current source, newer than alpha.6 | Names only; no product/address/program change — [naming contract](../contracts/project-name-editing.md) |
| Moving a device by drag and drop | 🟡 Partial or experimental | Single device → line/building part within its own installation; a group address dropped on a link row links it; other structural gestures remain unavailable — [Buildings and topology](user-guide/03-buildings-and-topology.md) |

## Group addresses

| Capability | Status | Notes |
| --- | --- | --- |
| Create/delete and address/reference validation | ✅ Implemented | Numeric duplicate/still-linked rules; display names may repeat; a range heading's **+** creates in that range with the next free address suggested (issue #20, Alpha 12) — [Working with group addresses](user-guide/04-group-addresses.md#creating-an-address-right-in-its-range) |
| Individual group-address name editing through Properties, F2/context menu | ✅ Current source, newer than alpha.6 | Name-only, all installations, guarded revision and exact undo/native history; [naming contract](../contracts/project-name-editing.md) |
| Choosing free, two-level or three-level style when the project is created | ✅ Implemented | Initial display/address structure; the next row describes changing it later |
| Changing the style afterwards | ✅ Implemented | On the Project node in the properties pane, undoable |
| CSV export and re-import in KNXBench's own format | ✅ Implemented | Every address in the reference project round-trips unchanged |
| CSV interoperability with ETS or `.esf` | ❌ Not implemented | Never claimed, never tested — [§38](../KNOWN_LIMITATIONS.md#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability) |
| Explicit, preview-confirmed re-addressing and unreferenced deletion through CSV | ✅ Implemented | Stable ids, directional-link preview, state-bound confirmation |
| Creating or restructuring ranges through CSV | ❌ Not implemented | [§39](../KNOWN_LIMITATIONS.md#39-csv-import-never-re-addresses-deletes-or-manages-group-ranges) |

## Communication objects and links

| Capability | Status | Notes |
| --- | --- | --- |
| Viewing every communication object of a device, with its resolved values and their provenance | ✅ Implemented | [Devices and products](user-guide/05-devices-and-products.md) |
| Editing the datapoint type and the description | ✅ Implemented | Both are undoable commands |
| Filling a product name's or channel heading's `{{0}}` from its text parameter | ✅ Implemented (2026-10-14) | Device detail and documentation report; index `0` only, project-owned names untouched — [contract](../contracts/positional-text-templates.md) |
| All six communication flags, including Read on init (`I`) | ✅ Implemented | Stored, resolved through the product-data layers, undoable |
| Exporting Read on init into a `.knxproj` | ❌ Not applicable since 2026-09-20 | There is no `.knxproj` export at all — [ADR-0028](../adr/0028-no-knxproj-export.md). The flag is stored and undoable in `.knxdb`; the warning that used to announce its loss is gone with the exporter — [§117](../KNOWN_LIMITATIONS.md#117-read_on_init_flag-is-parsed-and-stored-then-discarded-before-it-reaches-knx-core) |
| Linking and unlinking a communication object to a group address, with a send/receive direction | ✅ Implemented | [Working with group addresses](user-guide/04-group-addresses.md) |

## Parameters

**2026-10-08 presentation:** the editor is separated from **Diagnostics** and
inspection-only **Manufacturer fields** (evaluated Access Read/None). Repeated
causes are grouped while every detail and unmatched stored value is retained.
See [Device parameter workspace](../PARAMETER_WORKSPACE.md) for contract and
source-bound browser/unit/build evidence; this does not expand KNX compatibility.

| Capability | Status | Notes |
| --- | --- | --- |
| Reading, storing and displaying parameters from the application program | ✅ Implemented | 1,390 parameter values from the reference project |
| Editing a value, with validation and re-evaluation of the program's dynamic tree | ✅ Implemented | Stored as an undo step |
| Writing a declared but currently hidden parameter | ❌ Not implemented, deliberately | Refused rather than written blind — [§70](../KNOWN_LIMITATIONS.md#70-writing-a-declared-but-not-currently-shown-parameter-is-now-refused) |
| Module arguments and module-scoped parameters | 🟡 Partial or experimental | Repeated instantiation is refused; a module without an `Id` cannot be matched — [§68](../KNOWN_LIMITATIONS.md#68-repeated-module-instantiation-is-refused-not-supported), [§69](../KNOWN_LIMITATIONS.md#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance) |
| Writing a parameter value to a real device | 🟡 Partial or experimental | As part of a device download; verified on one device — see *Commissioning and device download*, below |

## Datapoint types

| Capability | Status | Notes |
| --- | --- | --- |
| Encoding and decoding across DPT main families 1–30 | 🟡 Scope-limited | Main-family coverage is not complete subtype/application semantics — [DPT audit](../spec-audits/2026-10-07-dpt-document-audit.md), [Datapoint types](knx-basics/04-datapoint-types.md) |
| Types outside implemented main families 1–30 | ❌ Not implemented | Includes structured/system types; not every higher-numbered type is LTE. No blanket subtype conformance — [DPT audit](../spec-audits/2026-10-07-dpt-document-audit.md) |
| Resolving a group address's type from its linked objects, reporting conflicts instead of guessing | ✅ Implemented | — |
| Declared versus linked group-address DPT | ✅ Implemented at stated scope | Typed declaration, linked alternatives and conflict/outcome are kept distinct and shown in the Inspector — [ADR-0078](../adr/0078-group-address-declared-dpt.md) |
| Explicitly declaring a typed value's input grammar | ✅ Implemented | CLI, HTTP, and web callers can pass the format explicitly; omitted legacy CLI/HTTP fields retain their prior inferred grammar, while new web writes are explicit. Project-judgment encodings are queryable metadata — [§61](../KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) |

## Documentation export

| Capability | Status | Notes |
| --- | --- | --- |
| HTML project documentation | ✅ Implemented | [Reports and comparison](user-guide/08-reports-and-diff.md) |
| Preview, print and section selection | ✅ Implemented | Sandboxed in-app preview with warnings, print through the browser, section checkboxes shared by preview and export; desktop-webview printing unchecked — [§49](../KNOWN_LIMITATIONS.md#49-project-documentation-export-has-no-in-application-print-preview), [§50](../KNOWN_LIMITATIONS.md#50-project-documentation-export-has-no-section-selection) |
| Native PDF | ❌ Not implemented | Print the preview to PDF instead — [§45](../KNOWN_LIMITATIONS.md#45-project-documentation-export-has-no-native-pdf-output) |
| Manufacturer, product and program names in the report | 🟡 Partial or experimental | Resolved only from a matching installed product/program pair; raw identifiers and warnings remain the fallback — [§46](../KNOWN_LIMITATIONS.md#46-project-documentation-export-does-not-resolve-manufacturer-product-or-program-names) |
| Parameter values and module arguments in the report | 🟡 Partial or experimental | Raw values are listed; enum labels and names resolve when supported, while unformatted kinds and unsupported module semantics warn — [§47](../KNOWN_LIMITATIONS.md#47-project-documentation-export-does-not-list-parameter-values-or-module-instance-arguments) |
| Parity with an ETS report | ❌ Not implemented | And not measurable here, with no ETS to compare against — [§44](../KNOWN_LIMITATIONS.md#44-project-documentation-export-t13-has-no-ets-report-parity-and-none-can-currently-be-measured) |

## Project comparison

| Capability | Status | Notes |
| --- | --- | --- |
| Comparing two `.knxdb` projects, in the interface and on the command line | ✅ Implemented | [Reports and comparison](user-guide/08-reports-and-diff.md) |
| Showing before and after values for every entity type | ✅ Implemented | Field names are shown untranslated — [§59](../KNOWN_LIMITATIONS.md#59-project-diffs-text-and-web-renderers-show-which-fields-changed-not-their-beforeafter-values-for-most-entity-types) |
| The web panel's detail level | ✅ Implemented | Expandable per-entity list; long tables scroll (virtualised) with text and status filter; no search across tables — [§60](../KNOWN_LIMITATIONS.md#60-project-diffs-web-panel-shows-grouped-counts-only) |
| Comparing against a raw `.knxproj`, in the interface and on the command line | ✅ Implemented | Import diagnostics shown; an import with errors is refused — [§57](../KNOWN_LIMITATIONS.md#57-project-diff-cannot-compare-against-a-raw-knxproj) |
| Merging or applying a difference | ❌ Not implemented | [§55](../KNOWN_LIMITATIONS.md#55-project-diff-cannot-merge-or-apply-a-diff-back-onto-a-project) |
| A non-zero exit code for pipelines | ✅ Implemented | `knx diff --exit-code`: 0 equal, 1 different, 2 failure — [Command line](user-guide/10-command-line.md) |
| Three-way comparison | ❌ Not implemented | [§56](../KNOWN_LIMITATIONS.md#56-project-diff-does-not-do-a-three-way-comparison) |

## Command line

| Capability | Status | Notes |
| --- | --- | --- |
| `knx import` | ✅ Implemented | With `--report-json` for the full import report — [Command line](user-guide/10-command-line.md). `knx export` was removed on 2026-09-20 — [ADR-0028](../adr/0028-no-knxproj-export.md) |
| `knx ga-export`, `knx ga-import` (with `--dry-run`) | ✅ Implemented | — |
| `knx doc-export`, `knx diff` | ✅ Implemented | — |
| `knx products list / ingest / show / verify` | ✅ Implemented | `ingest` accepts a `.knxproj` or a `.knxprod`; a `.vd2` argument is listed but always refused |
| `knx bus discover / monitor / write / route-monitor / route-send / scan` | ✅ Implemented | Some with the limitations under *KNXnet/IP and the bus* |
| Command-specific exit codes | ✅ Implemented | Import/GA-import use `2` for report errors; `diff --exit-code` uses 0 equal / 1 different / 2 failure. Read the command's contract, not one global meaning for `2` |
| Creating a project from the command line | ❌ Not implemented | No command exists; create one in the application |
| Writing to a device from the command line | 🟡 Partial or experimental | `knx device download` (three verified programs, each on one device); address programming is refused for now — see *Commissioning and device download*, below |
| `knx-mcp`: read-only questions from an AI agent over MCP | 🟡 Partial or experimental | Eight read tools over saved files; never writes, no bus; parameter visibility from the `Dynamic` tree, access not applied — [AI agents over MCP](user-guide/12-ai-agents.md), [§165](../KNOWN_LIMITATIONS.md#165-the-mcp-adapter-reads-saved-files-only-and-its-visibility-ignores-access) |

## KNXnet/IP and the bus

| Capability | Status | Notes |
| --- | --- | --- |
| Gateway discovery | ✅ Implemented | CLI and graphical bus monitor — [Bus monitor and KNXnet/IP](user-guide/07-bus-and-interfaces.md) |
| Tunnelling: connect, monitor, decode | ✅ Implemented | Verified live against one gateway; a 34-minute session read 1,299 telegrams with no drops |
| Flow view: who talks to whom in the running session | ✅ Implemented | Read-only, session-local; checked in Chromium with synthetic traffic, not on a real bus; Motion Off advised for large installations — [§154](../KNOWN_LIMITATIONS.md#154-the-telegram-flow-view-is-checked-and-measured-in-chromium-only) |
| Routing (multicast) | 🟡 Partial or experimental, command line only | The graphical monitor is tunnelling only — [§62](../KNOWN_LIMITATIONS.md#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-only-its-passive-receive-path-has-real-gateway-evidence) |
| Sending a group value write | 🟡 Implemented, live evidence limited | Tested write paths are reachable; passive receive evidence does not prove every live send/UI/gateway combination. Undo does not reverse a bus write |
| Line scan | 🟡 Partial or experimental | CLI and graphical diagnostics; live bus load, cannot identify products or cross couplers — [§72](../KNOWN_LIMITATIONS.md#72-line-scan-t17-an-unthrottled-scan-is-a-live-bus-cost-not-a-theoretical-one--shipped-2026-09-13-still-true) |
| Verification against more than one gateway model | 🟡 Partial or experimental | Exactly one model so far |
| USB and other non-IP interfaces | ❌ Not implemented | KNXnet/IP only |

## Commissioning and device download

| Capability | Status | Notes |
| --- | --- | --- |
| Downloading a project device's configuration to the device | 🟡 Partial or experimental | `knx device download` and the **Download to device** tab: plan first, confirmation, live progress. Verified in the simulator; real devices: `1.1.67` (MDT, mask `0701h`) via both the command and the tab, and an Eibmarkt presence detector plus the MDT fourfold bathroom button (both mask `0701h`, complete download only, 2026-10-09) via the command, every octet read back — [Bus](user-guide/07-bus-and-interfaces.md#downloading-to-a-device) |
| Programming an individual address | 🟡 Refused safety boundary | The command/tab exist, but confirmed writes fail before connection until device-specific durable recovery exists. Not a working write or an active task; historical live evidence does not grant permission — [web](user-guide/07-bus-and-interfaces.md#programming-an-individual-address), [ADR-0058](../adr/0058-individual-address-reset-requires-durable-recovery.md) |
| The load/unload/reset/memory-write procedures inside the core library | 🟡 Bounded implementation | Three real memory-download devices, all mask `0701h`; property procedures have simulator-only evidence. No generic real-device coverage or new hardware authorization — [§7](../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked), [§92](../KNOWN_LIMITATIONS.md#92-commissioning-phase-2-is-verified-against-a-simulator-this-project-wrote-and-has-never-addressed-a-device) |
| Read-only verification against a real installation | 🟡 Partial or experimental | Reading device state has been exercised against real hardware — [§7](../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked) |
| Live activity and persisted commissioning metadata | ✅ Implemented at bounded scope | Bus → Live/History shows supported activity, partial coverage and untracked categories. Not project undo, a complete traffic journal or a recovery image — [Bus](user-guide/07-bus-and-interfaces.md), [history UI](../../apps/knx-web/src/BusActivityHistory.tsx) |

## KNX Secure

| Capability | Status | Notes |
| --- | --- | --- |
| Data Secure, IP Secure, keyring handling | ❌ Not implemented | No sample key material has been available to verify against; IP Secure was scoped and then shelved indefinitely — [§8](../KNOWN_LIMITATIONS.md#8-knx-secure-is-not-implemented), [§26](../KNOWN_LIMITATIONS.md#26-busconnection-does-not-yet-support-knx-ip-secure) |

## Web interface and Docker

| Capability | Status | Notes |
| --- | --- | --- |
| The full workbench in a browser, served by `knx-server` | ✅ Implemented | [Web and Docker](user-guide/11-web-and-docker.md) |
| Docker image, running from the host network | ✅ Implemented | Measured against the built image |
| Docker image, reached through a published port | ✅ Implemented | Needs a credential, which is also what makes it safe to publish — measured against the built image |
| Password authentication, server side | ✅ Implemented | PBKDF2-HMAC-SHA256, session cookie, loopback-only binding when unset — [ADR-0026](../adr/0026-server-authentication-or-loopback.md) |
| Logging in from the browser | ✅ Implemented | A login card in front of the workbench, driven by `GET /api/auth/status`; verified end to end — [Web and Docker](user-guide/11-web-and-docker.md) |
| HTTPS with self-signed or supplied certificates | ✅ Implemented | Enabled by default with authentication — [Web and Docker](user-guide/11-web-and-docker.md#https), [ADR-0088](../adr/0088-server-terminates-tls-itself.md) |
| User accounts, roles and per-user audit trail | ❌ Not implemented | One password, no identities — [§22](../KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback) |
| Two people editing the same project | ❌ Not implemented | One project, one undo stack, no conflict detection — [§63](../KNOWN_LIMITATIONS.md#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all) |

## Desktop application

| Capability | Status | Notes |
| --- | --- | --- |
| Tauri desktop shell and native file-dialog path | 🟡 Implemented, platform evidence bounded | Shell/dialog code is present and scoped native runs exist, but there is no complete native-dialog/Orca/host-matrix acceptance — [Installation](getting-started/04-installation.md) |
| Linux AppImage | 🟡 Partial or experimental | Built and launched on one host, x86-64 only |
| Owned Wayland/X11 AppImage launcher | 🟡 Source-integrated, one-host evidence | Current/tagged alpha.5 source includes the packaging hook; historical direct-image tests are not a new native run of every released asset/GPU — [Launcher contract](../APPIMAGE_LAUNCHER.md) |
| Code signing, auto-update, distribution packages, other architectures | ❌ Not implemented | Build from source, or use the server |
| Windows and macOS builds | ❌ Not implemented | Linux-first, by design |

## Additional workbench features

| Capability | Status | Notes |
| --- | --- | --- |
| Themes, System selection and managed declarative packs | ✅ Implemented | Porcelain/Graphite/Cupertino/LCARS plus bundled/importable packs; no arbitrary CSS or code — [Settings](user-guide/09-settings-and-appearance.md) |
| Achievements | ✅ Implemented at stated scope | Shipped catalogue and server record; does not authorize device operations — [ADR-0089](../adr/0089-achievements.md) |
| Toast notifications | ✅ Implemented | Status, error and achievement cards share their frame, nine-second duration and motion-aware exit; errors retain alert semantics and server-text disclosure — [Settings](user-guide/09-settings-and-appearance.md#toast-notifications) |
| Humour templates | ✅ Implemented | Thirty error wrappers and thirty late-night entries plus holiday pairs; extra copy is optional |
| Dashboard | 🟡 Partial | Counts/diagnostics work; clickable count-to-detail drill-down does not |
| Project notes | ❌ Not implemented | ADR-0031 defines the shape only; not F1 help or HTML report export |
| Generic repetitive-task automation | ❌ Not implemented | Atomic command batches exist, but no macro recorder, template UI or scheduler |

## Language and accessibility

| Capability | Status | Notes |
| --- | --- | --- |
| English and German interface | ✅ Implemented | [Settings and appearance](user-guide/09-settings-and-appearance.md) |
| Playful Bavarian and Klingon language packs | ✅ Implemented | Shipped packs (`bar`, `tlh`) supplement the English/German catalogues; playful translations, not a technical-language certification — [Settings and appearance](user-guide/09-settings-and-appearance.md#language-packs) |
| Other interface languages | 🟡 Partial or experimental | Importable JSON language packs; completeness depends on the supplied pack — [Settings and appearance](user-guide/09-settings-and-appearance.md#language-packs) |
| Using the translations that come with product data | 🟡 Partial or experimental | **Product data language** in Settings picks the language for parameters, catalogue, product data and DPT texts; a missing translation falls back to the program's own text and is marked — [§37](../KNOWN_LIMITATIONS.md#37-translations-reach-selected-surfaces-not-every-imported-text-or-ui-output) |
| Keyboard operation with documented shortcuts | ✅ Implemented | [Keyboard shortcuts](reference/01-keyboard-shortcuts.md) |
| Respecting the system's reduced-motion preference | ✅ Implemented | Settings also offers motion style and level; the OS reduced-motion preference takes precedence — [Settings and appearance](user-guide/09-settings-and-appearance.md) |
| Screen-reader support | 🟡 Partial or experimental | Successful project loads now announce a localized status; end-to-end assistive-technology validation is still missing — [Known issues](known-issues.md) |

---

## What this adds up to

KNXBench reads, understands, edits and writes back KNX project data, and it
can watch a non-secure bus. “Writes back” means native `.knxdb`, not an ETS
archive; supported editable semantics are narrower than retained source bytes.
It can download a device's configuration only within a very
narrow verified scope (one device so far), cannot handle KNX Secure, and
cannot be shared between two people. The last two are absences, the first is
a beginning, and the next chapter is careful about what it does and does not
promise regarding them.

The engineering record behind every row is
[`docs/IMPLEMENTATION_STATUS.md`](../IMPLEMENTATION_STATUS.md), which is a
running log rather than a summary — long, chronological, and written for
maintainers.

The [known-issues/status audit](../status/2026-10-08-known-issues-status-audit.md)
maps these areas to source consumers and evidence. It does not rewrite dated
historical test results or close another owner's ledger rows.

[Manual index](README.md) · Next: [Ideas and roadmap](ideas-and-roadmap.md) →
