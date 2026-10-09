← Previous: [Known issues](known-issues.md) · [Manual index](README.md)

# Implementation status

This chapter is the honest inventory: what exists, what half exists, and what
does not exist at all, area by area across the whole application.

This chapter describes source checked on **8 October 2026**; the application
components were bumped to `0.1.0-alpha.6` on 9 October. The public
[`v0.1.0-alpha.6` pre-release](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.6)
is a separate build snapshot, not a promise that every newer source feature is bundled.
The version number is where the project starts counting, not a claim that
anything has reached a finish line.

The [ideas/roadmap audit](../status/2026-10-08-ideas-roadmap-audit.md) checked
source `608a204b`, actual UI callers and recorded boundaries. An accepted
alpha boundary is not implemented functionality or an active work package.

## New on main: native project history

The user-authorized 2026-10-09 package implements restart-safe native undo/redo,
named and automatic saved-state versions, and confirmed restore with a safety
version. Unsaved native work remains marked unsaved while its recovery journal
is durable. Before first Save As history is session-only. Versions are inside
the same file, not independent disaster backups. Native schema v11 needs a new
build; this source feature is **not** in the published alpha.6 snapshot.
[Contract](../PROJECT_HISTORY.md) · [verification](../status/2026-10-09-project-history-verification.md).
Native/Orca, physical power-loss, hardware/ETS and release evidence remain separate.

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
| Native `.knxdb` project file (SQLite, store schema version 11 in the local history package; alpha.6 uses v10) | ✅ Implemented | Open, save, save as — [Projects](user-guide/02-projects.md) |
| Schema migration of an older project file, with a refusal rather than a guess when the file is newer than the build | ✅ Implemented | Migration tests per version step — [`docs/DATA_MODEL.md`](../DATA_MODEL.md) |
| Undo and redo across project edits | ✅ Implemented | Native-backed reversible edits survive restart with undo/redo; before first Save As it is session-local. Cannot undo bus writes — [history contract](../PROJECT_HISTORY.md) (local package, not alpha.6) |
| New-project wizard with an optional starting structure (areas/lines, building tree, main/middle group ranges, two presets) | ✅ Implemented | Applied with the new project in one step or refused without replacing anything — [Projects](user-guide/02-projects.md), [ADR-0093](../adr/0093-wizards-are-views-over-existing-commands.md) |
| Add-device wizard: product, placement (installation, line, building part), server preview of names and addresses, confirmed create | ✅ Implemented | Creation/placement are one undo step. Changed computed names/addresses cause stale-preview refusal; this is not a whole-project revision lock — [Devices and products](user-guide/05-devices-and-products.md#the-add-device-wizard), [ADR-0093](../adr/0093-wizards-are-views-over-existing-commands.md) |
| Provenance for modelled layered values | ✅ Implemented | `Override` distinguishes absent/empty/malformed/value, and `Resolved` carries the source layer. Not every ordinary project attribute is layered — [Data model](../DATA_MODEL.md), [ADR-0010](../adr/0010-per-attribute-override-representation.md) |
| Exporting the open project through the browser | ✅ Implemented | File → Export project… streams a newly serialized `.knxdb`; Save As still writes to the server's directory — [Projects](user-guide/02-projects.md) |
| Autosave for a project that already has a file | ✅ Implemented | Configurable interval, cancellation notice and reported failures — [Projects](user-guide/02-projects.md#saving) |
| Native project versions and persistent undo history | ✅ Local source package | Named/save/pre-restore versions, revision-bound confirmed restore, bounded atomic journal — [Projects](user-guide/02-projects.md#project-history-restart-safe-undo-and-versions). Not in alpha.6; keep independent copies against file/disk loss |

## ETS project import

| Capability | Status | Notes |
| --- | --- | --- |
| `.knxproj` at schema 11 (ETS4) and schema 21 | ✅ Implemented | Measured counts on real files — [Supported and unsupported](reference/02-supported-and-unsupported.md) |
| `.knxproj` at schema 23 | 🟡 Bounded evidence | Real reference import and native `.knxdb` roundtrip exist; module handling is inferred from schema 21 without an independent module-using schema-23 sample. No ETS roundtrip — [Supported and unsupported](reference/02-supported-and-unsupported.md) |
| `.knxproj` at schemas 12–14, 20, 22 | 🟡 Unverified sample coverage | Parser-family admission is not verification against real project files at those schemas — [Supported and unsupported](reference/02-supported-and-unsupported.md) |
| Import report with errors, warnings, unsupported constructs and conflicts | ✅ Implemented | Also available as JSON from the command line |
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

| Capability | Status | Notes |
| --- | --- | --- |
| Product data ingested from inside a `.knxproj` | ✅ Implemented | The reference project's 4 manufacturers and 12 application programs ingest completely |
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
| Moving a device by drag and drop | 🟡 Partial or experimental | Single device → line/building part within its own installation; a group address dropped on a link row links it; other structural gestures remain unavailable — [Buildings and topology](user-guide/03-buildings-and-topology.md) |

## Group addresses

| Capability | Status | Notes |
| --- | --- | --- |
| Create, rename, delete, inline edit; duplicate and still-linked validation | ✅ Implemented | [Working with group addresses](user-guide/04-group-addresses.md) |
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
