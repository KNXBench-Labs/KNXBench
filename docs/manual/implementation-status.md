← Previous: [Known issues](known-issues.md) · [Manual index](README.md)

# Implementation status

This chapter is the honest inventory: what exists, what half exists, and what
does not exist at all, area by area across the whole application.

The version described is `0.1.0-alpha.1`. **No release has been published.**
The version number is where the project starts counting, not a claim that
anything has reached a finish line.

## How to read the tables

| Marker | Meaning |
| --- | --- |
| ✅ Implemented | Verified by an automated test against real data, or by a documented run against a real file, gateway or container |
| 🟡 Partial or experimental | Works for part of its scope, or works but has never been verified against a real-world sample |
| 🚧 In progress | Started, not usable end to end |
| ❌ Not implemented | Does not exist, or exists only somewhere a user cannot reach |

Two rules were applied while filling these tables in. A ✅ needs evidence
that can be pointed at — no marker was awarded for code that looks finished.
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
| Native `.knxdb` project file (SQLite, store schema version 9) | ✅ Implemented | Open, save, save as — [Projects](user-guide/02-projects.md) |
| Schema migration of an older project file, with a refusal rather than a guess when the file is newer than the build | ✅ Implemented | Migration tests per version step — [`docs/DATA_MODEL.md`](../DATA_MODEL.md) |
| Undo and redo across every project edit | ✅ Implemented | One shared undo stack per open project |
| Provenance: every attribute knows which layer it came from, and an override is distinguishable from an inherited value | ✅ Implemented | [`docs/DATA_MODEL.md`](../DATA_MODEL.md), [ADR-0010](../adr/0010-per-attribute-override-representation.md) |
| Downloading the open project through the browser | ✅ Implemented | File → Download project streams a newly serialized `.knxdb`; Save As still writes to the server's directory — [Projects](user-guide/02-projects.md) |
| Automatic backup or version history | ❌ Not implemented | Copy the `.knxdb` file yourself |

## ETS project import

| Capability | Status | Notes |
| --- | --- | --- |
| `.knxproj` at schema 11 (ETS4) and schema 21 | ✅ Implemented | Measured counts on real files — [Supported and unsupported](reference/02-supported-and-unsupported.md) |
| `.knxproj` at schemes 12–14, 20, 22, 23 | 🟡 Partial or experimental | Either documented but never sampled, or sampled in one direction only — [Supported and unsupported](reference/02-supported-and-unsupported.md) |
| Import report with errors, warnings, unsupported constructs and conflicts | ✅ Implemented | Also available as JSON from the command line |
| Preserving unknown attributes, elements and whole container members untouched | ✅ Implemented | 38 container entries stored with the content hash they arrived with |
| ZipCrypto-protected projects (ETS4/ETS5) | 🟡 Partial or experimental | Implemented and tested against synthetic fixtures only |
| AES-protected projects (ETS6) | ❌ Not implemented | Refused by name — [§13](../KNOWN_LIMITATIONS.md#13-password-protected-projects-zipcrypto-ets4ets5-is-decrypted-aes-ets6-is-still-refused) |
| Importing part of a project into an existing one | ❌ Not implemented | Import is whole-file — [`docs/GAP_ANALYSIS_ETS.md`](../GAP_ANALYSIS_ETS.md), row C3 |

## ETS project export

| Capability | Status | Notes |
| --- | --- | --- |
| Writing a `.knxproj` at any schema version | ❌ Not implemented, deliberately | Withdrawn on 2026-09-20 with the writer, the `knx export` subcommand, the HTTP route and the button — [ADR-0028](../adr/0028-no-knxproj-export.md) |

Import is one-way. This table used to have five rows describing a writer that shipped
between 2026-09-10 and 2026-09-20; none of it exists now, and neither do the warnings it
raised. A project that has been imported stays in `.knxdb`.

## Product databases

| Capability | Status | Notes |
| --- | --- | --- |
| Product data ingested from inside a `.knxproj` | ✅ Implemented | The reference project's 4 manufacturers and 12 application programs ingest completely |
| Standalone `.knxprod` at master data scheme 11 or 20 | ✅ Implemented | Four real files, content-addressed, idempotent re-install — [Devices and products](user-guide/05-devices-and-products.md) |
| Standalone `.knxprod` at schemes 12–19, 21, 22 | ❌ Not implemented | No tested sample at those schemes |
| Encrypted `.knxprod` packages, and legacy `.vd2` files | ❌ Not implemented, deliberately | A permanent scope exclusion, refused as a typed error |
| Online catalog update from a manufacturer | ❌ Not implemented | Files are installed by hand — [`docs/GAP_ANALYSIS_ETS.md`](../GAP_ANALYSIS_ETS.md), row C6 |
| Devices configured by a manufacturer plug-in | ❌ Not implemented | No plug-in host exists and none is planned — [§6](../KNOWN_LIMITATIONS.md#6-devices-behind-vendor-plug-in-dlls) |

## Topology and buildings

| Capability | Status | Notes |
| --- | --- | --- |
| Areas, lines and individual addresses: read and edit | ✅ Implemented | [Buildings and topology](user-guide/03-buildings-and-topology.md) |
| Buildings, floors, rooms and building parts: read and edit | ✅ Implemented | 22 building parts in the reference project |
| Creating a project from scratch in the interface | ✅ Implemented | Browser-verified — [§83](../KNOWN_LIMITATIONS.md#83-the-from-scratch-launcher-is-browser-verified--resolved-2026-09-16-goal-task-17) |
| Five additional documented space types kept distinct on import | ✅ Implemented | `Stairway`, `RoomPart`, `Area`, `Ground` and `Segment` survive import and native save/load; synthetic coverage, with the Schema23 vocabulary inconsistency documented in [§89](../KNOWN_LIMITATIONS.md#89-five-documented-spacetype-values-are-coarsened-to-buildingpart-on-import) |
| Moving a device by drag and drop | 🟡 Partial or experimental | Single device → line/building part in the first installation; other structural gestures remain unavailable — [Buildings and topology](user-guide/03-buildings-and-topology.md) |

## Group addresses

| Capability | Status | Notes |
| --- | --- | --- |
| Create, rename, delete, inline edit; duplicate and still-linked validation | ✅ Implemented | [Working with group addresses](user-guide/04-group-addresses.md) |
| Choosing free, two-level or three-level style when the project is created | ✅ Implemented | The choice is made once |
| Changing the style afterwards | ❌ Not implemented | The New project dialog says otherwise; it is wrong — [Known issues](known-issues.md) |
| CSV export and re-import in KNXBench's own format | ✅ Implemented | Every address in the reference project round-trips unchanged |
| CSV interoperability with ETS or `.esf` | ❌ Not implemented | Never claimed, never tested — [§38](../KNOWN_LIMITATIONS.md#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability) |
| Re-addressing, deleting or managing ranges through CSV | ❌ Not implemented | [§39](../KNOWN_LIMITATIONS.md#39-csv-import-never-re-addresses-deletes-or-manages-group-ranges) |

## Communication objects and links

| Capability | Status | Notes |
| --- | --- | --- |
| Viewing every communication object of a device, with its resolved values and their provenance | ✅ Implemented | [Devices and products](user-guide/05-devices-and-products.md) |
| Editing the datapoint type and the description | ✅ Implemented | Both are undoable commands |
| All six communication flags, including Read on init (`I`) | ✅ Implemented | Stored, resolved through the product-data layers, undoable |
| Exporting Read on init into a `.knxproj` | ❌ Not applicable since 2026-09-20 | There is no `.knxproj` export at all — [ADR-0028](../adr/0028-no-knxproj-export.md). The flag is stored and undoable in `.knxdb`; the warning that used to announce its loss is gone with the exporter — [§117](../KNOWN_LIMITATIONS.md#117-read_on_init_flag-is-parsed-and-stored-then-discarded-before-it-reaches-knx-core) |
| Linking and unlinking a communication object to a group address, with a send/receive direction | ✅ Implemented | [Working with group addresses](user-guide/04-group-addresses.md) |

## Parameters

| Capability | Status | Notes |
| --- | --- | --- |
| Reading, storing and displaying parameters from the application program | ✅ Implemented | 1,390 parameter values from the reference project |
| Editing a value, with validation and re-evaluation of the program's dynamic tree | ✅ Implemented | Stored as an undo step |
| Writing a declared but currently hidden parameter | ❌ Not implemented, deliberately | Refused rather than written blind — [§70](../KNOWN_LIMITATIONS.md#70-writing-a-declared-but-not-currently-shown-parameter-is-now-refused) |
| Module arguments and module-scoped parameters | 🟡 Partial or experimental | Repeated instantiation is refused; a module without an `Id` cannot be matched — [§68](../KNOWN_LIMITATIONS.md#68-repeated-module-instantiation-is-refused-not-supported), [§69](../KNOWN_LIMITATIONS.md#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance) |
| Writing a parameter value to a real device | ❌ Not implemented | See *Commissioning*, below |

## Datapoint types

| Capability | Status | Notes |
| --- | --- | --- |
| Encoding and decoding main types 1 to 30, every subtype | ✅ Implemented | [Datapoint types](knx-basics/04-datapoint-types.md) |
| The 200-series LTE/system types | ❌ Not implemented | Out of scope for this version |
| Resolving a group address's type from its linked objects, reporting conflicts instead of guessing | ✅ Implemented | — |
| Explicitly declaring a typed value's input grammar | ✅ Implemented | CLI, HTTP, and web callers can pass the format explicitly; omitted legacy CLI/HTTP fields retain their prior inferred grammar, while new web writes are explicit. Project-judgment encodings are queryable metadata — [§61](../KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) |

## Documentation export

| Capability | Status | Notes |
| --- | --- | --- |
| HTML project documentation | ✅ Implemented | [Reports and comparison](user-guide/08-reports-and-diff.md) |
| Native PDF, print preview, section selection | 🟡 Partial or experimental | No native PDF or frontend controls; the API supports side-effect-free HTML preview and section selection — [§45](../KNOWN_LIMITATIONS.md#45-project-documentation-export-has-no-native-pdf-output), [§49](../KNOWN_LIMITATIONS.md#49-project-documentation-export-has-no-in-application-print-preview), [§50](../KNOWN_LIMITATIONS.md#50-project-documentation-export-has-no-section-selection) |
| Manufacturer, product and program names in the report | 🟡 Partial or experimental | Resolved only from a matching installed product/program pair; raw identifiers and warnings remain the fallback — [§46](../KNOWN_LIMITATIONS.md#46-project-documentation-export-does-not-resolve-manufacturer-product-or-program-names) |
| Parameter values and module arguments in the report | 🟡 Partial or experimental | Raw values are listed; enum labels and names resolve when supported, while unformatted kinds and unsupported module semantics warn — [§47](../KNOWN_LIMITATIONS.md#47-project-documentation-export-does-not-list-parameter-values-or-module-instance-arguments) |
| Parity with an ETS report | ❌ Not implemented | And not measurable here, with no ETS to compare against — [§44](../KNOWN_LIMITATIONS.md#44-project-documentation-export-t13-has-no-ets-report-parity-and-none-can-currently-be-measured) |

## Project comparison

| Capability | Status | Notes |
| --- | --- | --- |
| Comparing two `.knxdb` projects, in the interface and on the command line | ✅ Implemented | [Reports and comparison](user-guide/08-reports-and-diff.md) |
| Showing before and after values for every entity type | 🟡 Partial or experimental | Most types name the changed fields only — [§59](../KNOWN_LIMITATIONS.md#59-project-diffs-text-and-web-renderers-show-which-fields-changed-not-their-beforeafter-values-for-most-entity-types) |
| The web panel's detail level | 🟡 Partial or experimental | Grouped counts — [§60](../KNOWN_LIMITATIONS.md#60-project-diffs-web-panel-shows-grouped-counts-only) |
| Comparing against a raw `.knxproj` | ❌ Not implemented | [§57](../KNOWN_LIMITATIONS.md#57-project-diff-cannot-compare-against-a-raw-knxproj) |
| Merging or applying a difference | ❌ Not implemented | [§55](../KNOWN_LIMITATIONS.md#55-project-diff-cannot-merge-or-apply-a-diff-back-onto-a-project) |
| Three-way comparison, and a non-zero exit code for pipelines | ❌ Not implemented | [§56](../KNOWN_LIMITATIONS.md#56-project-diff-does-not-do-a-three-way-comparison), [§58](../KNOWN_LIMITATIONS.md#58-project-diff-has-no-ci-friendly-exit-nonzero-on-any-difference-flag) |

## Command line

| Capability | Status | Notes |
| --- | --- | --- |
| `knx import` | ✅ Implemented | With `--report-json` for the full import report — [Command line](user-guide/10-command-line.md). `knx export` was removed on 2026-09-20 — [ADR-0028](../adr/0028-no-knxproj-export.md) |
| `knx ga-export`, `knx ga-import` (with `--dry-run`) | ✅ Implemented | — |
| `knx doc-export`, `knx diff` | ✅ Implemented | — |
| `knx products list / ingest / show / verify` | ✅ Implemented | `ingest` accepts a `.knxproj` or a `.knxprod`; a `.vd2` argument is listed but always refused |
| `knx bus discover / monitor / write / route-monitor / route-send / scan` | ✅ Implemented | Some with the limitations under *KNXnet/IP and the bus* |
| Distinct exit codes (`0` success, `1` failure, `2` import produced a project whose report has errors) | ✅ Implemented | `2` is raised by `import` and `ga-import` only |
| Creating a project, or programming a device, from the command line | ❌ Not implemented | Neither command exists |

## KNXnet/IP and the bus

| Capability | Status | Notes |
| --- | --- | --- |
| Gateway discovery | ✅ Implemented | CLI and graphical bus monitor — [Bus monitor and KNXnet/IP](user-guide/07-bus-and-interfaces.md) |
| Tunnelling: connect, monitor, decode | ✅ Implemented | Verified live against one gateway; a 34-minute session read 1,299 telegrams with no drops |
| Routing (multicast) | 🟡 Partial or experimental, command line only | The graphical monitor is tunnelling only — [§62](../KNOWN_LIMITATIONS.md#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-only-its-passive-receive-path-has-real-gateway-evidence) |
| Sending a group value write | ✅ Implemented | The only bus write KNXBench performs; project undo does not cover it |
| Line scan | 🟡 Partial or experimental | CLI and graphical diagnostics; live bus load, cannot identify products or cross couplers — [§72](../KNOWN_LIMITATIONS.md#72-line-scan-t17-an-unthrottled-scan-is-a-live-bus-cost-not-a-theoretical-one--shipped-2026-09-13-still-true) |
| Verification against more than one gateway model | 🟡 Partial or experimental | Exactly one model so far |
| USB and other non-IP interfaces | ❌ Not implemented | KNXnet/IP only |

## Commissioning and device download

| Capability | Status | Notes |
| --- | --- | --- |
| Programming a device — application, parameters, links or individual address | ❌ Not implemented | **For a user this is simply absent:** no button, no route and no command exists anywhere in the interface or the command line, whatever the core library can do internally |
| The load/unload/reset/memory-write procedures inside the core library | 🚧 In progress | Verified against a simulator this project wrote, never against hardware — [§92](../KNOWN_LIMITATIONS.md#92-commissioning-phase-2-is-verified-against-a-simulator-this-project-wrote-and-has-never-addressed-a-device) |
| Read-only verification against a real installation | 🟡 Partial or experimental | Reading device state has been exercised against real hardware; writing has not — [§7](../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked) |

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
| TLS, user accounts, roles, audit trail | ❌ Not implemented | One password, no identities — [§22](../KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback) |
| Two people editing the same project | ❌ Not implemented | One project, one undo stack, no conflict detection — [§63](../KNOWN_LIMITATIONS.md#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all) |

## Desktop application

| Capability | Status | Notes |
| --- | --- | --- |
| Tauri desktop shell with the same workbench and a native file dialog | ✅ Implemented | [Installation](getting-started/04-installation.md) |
| Linux AppImage | 🟡 Partial or experimental | Built and launched on one host, x86-64 only |
| Code signing, auto-update, distribution packages, other architectures | ❌ Not implemented | Build from source, or use the server |
| Windows and macOS builds | ❌ Not implemented | Linux-first, by design |

## Language and accessibility

| Capability | Status | Notes |
| --- | --- | --- |
| English and German interface | ✅ Implemented | [Settings and appearance](user-guide/09-settings-and-appearance.md) |
| Any other interface language | ❌ Not implemented | — |
| Using the translations that come with product data | ❌ Not implemented | Stored on import, never read for the interface — [§37](../KNOWN_LIMITATIONS.md#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12) |
| Keyboard operation with documented shortcuts | ✅ Implemented | [Keyboard shortcuts](reference/01-keyboard-shortcuts.md) |
| Respecting the system's reduced-motion preference | ✅ Implemented | Settings also offers motion style and level; the OS reduced-motion preference takes precedence — [Settings and appearance](user-guide/09-settings-and-appearance.md) |
| Screen-reader support | 🟡 Partial or experimental | Successful project loads now announce a localized status; end-to-end assistive-technology validation is still missing — [Known issues](known-issues.md) |

---

## What this adds up to

KNXBench reads, understands, edits and writes back KNX project data, and it
can watch a bus. It cannot program a device, cannot handle KNX Secure, and
cannot be shared between two people. Those three are not near-misses; they
are absences, and the next chapter is careful about what it does and does not
promise regarding them.

The engineering record behind every row is
[`docs/IMPLEMENTATION_STATUS.md`](../IMPLEMENTATION_STATUS.md), which is a
running log rather than a summary — long, chronological, and written for
maintainers.

[Manual index](README.md) · Next: [Ideas and roadmap](ideas-and-roadmap.md) →
