← Previous: [Keyboard shortcuts](01-keyboard-shortcuts.md) · [Manual index](../README.md)

# Supported and unsupported KNX/ETS functionality

This chapter is the honest inventory: what KNXBench does today, what it does but has not
proven against a real file or a real gateway, and what it deliberately does not do at
all. The status markers follow one rule throughout — **✅ means a real test or a
documented real-file run stands behind the claim, not that the code merely looks right**.

| Marker | Meaning |
| --- | --- |
| ✅ Implemented | Verified against a real file, a real gateway, or an automated test that exercises real data |
| 🟡 Partial or experimental | Works for part of the scope, or works but is unverified against a real-world sample |
| 🚧 In progress | Started, not usable end to end yet |
| ❌ Not implemented | Does not exist in the code |

For the full evidence behind every row, see
[COMPATIBILITY.md](../../COMPATIBILITY.md), [IMPORT_EXPORT.md](../../IMPORT_EXPORT.md),
and [KNOWN_LIMITATIONS.md](../../KNOWN_LIMITATIONS.md) — this chapter summarizes them,
it does not replace them.

## `.knxproj` import, by master data scheme

A `.knxproj` file states a "master data scheme" version in its XML — a different number
from KNXBench's own native `.knxdb` file format, covered separately below.

| Scheme | ETS version | Status | Evidence |
| --- | --- | --- | --- |
| 11 | ETS 4.1.8 | ✅ Implemented | A real reference project (36 devices, 514 group addresses) imports with measured counts and nothing unknown or lost — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today) |
| 12 | ETS 4 | 🟡 Partial or experimental | Documented in the schema family, no real sample imported yet — [COMPATIBILITY.md §3](../../COMPATIBILITY.md#3-expected-but-unverified) |
| 13, 14 | ETS 5, up to 5.6 | 🟡 Partial or experimental | Same: documented, unverified — [COMPATIBILITY.md §3](../../COMPATIBILITY.md#3-expected-but-unverified) |
| 20 | ETS 5.7 | 🟡 Partial or experimental | A standalone scheme-20 `.knxprod` *product package* installs and is verified; a full scheme-20 `.knxproj` *project* has not been imported — [COMPATIBILITY.md §3](../../COMPATIBILITY.md#3-expected-but-unverified) |
| 21 | ETS 5.7+ | ✅ Implemented | The KNX Association's own demo project imports with zero unknown constructs — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today) |
| 22 | ETS 6.0–6.2 | 🟡 Partial or experimental | Documented only, no sample — [COMPATIBILITY.md §3](../../COMPATIBILITY.md#3-expected-but-unverified) |
| 23 | ETS 6.3.7959.0 | 🟡 Partial or experimental | An ETS6 re-export of the schema-11 reference project imports; module-based application programs are inferred from the schema-21 sample, not independently evidenced at schema 23 — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today) |

> **Note**
>
> "Verified" for schema 21 means one real sample project imports with measured counts.
> It does not mean every project ever produced by that ETS version will import cleanly —
> see [Working with projects](../user-guide/02-projects.md) for what an import report
> tells you when something in your own file falls outside that.

## `.knxproj` export

| Capability | Status | Evidence |
| --- | --- | --- |
| Writing a `.knxproj` in any schema version | ❌ Not implemented, deliberately | Withdrawn on 2026-09-20 with the writer, the CLI subcommand, the HTTP route and the button — [ADR-0028](../../adr/0028-no-knxproj-export.md), [COMPATIBILITY.md §4](../../COMPATIBILITY.md#4-not-supported) |

> **Note**
>
> This section used to list four shipped export capabilities and two open ones. None of
> them exist any more. KNXBench reads a `.knxproj` and never writes one; a project that
> has been imported stays in `.knxdb`. The group-address CSV and the HTML project
> documentation below are KNXBench's own formats and are unaffected.

## Password-protected projects

| Protection | Status | Evidence |
| --- | --- | --- |
| Detecting a password-protected container and naming it before asking for a password | ✅ Implemented | Tested against a hand-built protected container — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today) |
| Decrypting a ZipCrypto-protected project (schema < 21, ETS4/ETS5) with the correct password | 🟡 Partial or experimental | The ZipCrypto algorithm is fully specified and tested against synthetic fixtures; never run against a real password-protected ETS4/ETS5 export — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today), [§3](../../COMPATIBILITY.md#3-expected-but-unverified) |
| Opening an AES-protected project (schema ≥ 21, ETS6) | ❌ Not implemented | Refused by name (`ContainerError::UnsupportedEncryption`), never attempted — [COMPATIBILITY.md §3](../../COMPATIBILITY.md#3-expected-but-unverified) |

## Product database formats

| Format | Status | Evidence |
| --- | --- | --- |
| Standalone `.knxprod` product package, master data scheme 11 or 20 | ✅ Implemented | Four real files installed, content-addressed, idempotent re-install — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today) |
| Standalone `.knxprod`, schemes 12, 13, 14 and exact-namespace 21 | 🟡 Partial or experimental | Synthetic tests and the passing 115-instance read-only corpus matrix verify parser/persistence behavior, not full manufacturer semantics or ETS parity — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today) |
| Standalone `.knxprod`, exact namespaces 10 and 23 | 🟡 Scope-limited | Namespace admission and bounded parser/persistence support exist; this is not every-vendor semantic compatibility — [ADR-0083](../../adr/0083-admit-exact-product-scheme-10.md), [namespace gate](../../../crates/knx-productdb/src/package.rs) |
| Standalone `.knxprod`, schemes 15–19 and 22 | ❌ Not implemented | No observed standalone sample or verified namespace support — [COMPATIBILITY.md §4](../../COMPATIBILITY.md#4-not-supported) |
| Product data ingested from inside a `.knxproj` | ✅ Implemented | The reference project's manufacturer data — 4 manufacturers, 12 application programs — ingests completely — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today) |
| Encrypted `.knxprod` packages | ❌ Not implemented, deliberately | Rejected as a typed error with no rows published; this is a scope exclusion, not a gap to close — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today) |
| Legacy ETS3 `.vd3`/`.vd4`/`.vd5` product databases | 🟡 Offline use | The web catalog and device wizard install them with a password dialog; one password can be remembered. On the CLI, `knx products inspect-legacy` summarises them and `knx products import-legacy` imports them. Catalog, parameters, objects and visibility work offline. Not yet: data point types, download — [ADR-0094](../../adr/0094-legacy-exim-product-files.md) |
| Legacy `.vd2` product data | ❌ Not implemented, deliberately | A distinct pre-2013 ETS2-era container family, not the `.knxprod` ZIP/XML family at all; permanently out of scope — [COMPATIBILITY.md §4](../../COMPATIBILITY.md#4-not-supported) |

> **Note**
>
> Encrypted `.knxprod` and `.vd2` are both listed above as **excluded by decision**, not
> as work items waiting for a test sample. See
> [Products and product databases](../knx-basics/05-products-and-product-databases.md)
> for what a product database actually contains.

## Datapoint type (DPT) coverage

| Item | Status | Evidence |
| --- | --- | --- |
| Decoding and encoding across DPT main families 1–30 | 🟡 Scope-limited | The codec covers those main families, not every subtype's application semantics. Structured types and project-judgment encodings have explicit boundaries — [DPT audit](../../spec-audits/2026-10-07-dpt-document-audit.md), [KNOWN_LIMITATIONS.md §61](../../KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) |
| Types outside implemented main families 1–30, including higher structured/system types | ❌ Not implemented | Do not classify every higher-numbered type as LTE; expansion needs subtype/application evidence — [DPT audit](../../spec-audits/2026-10-07-dpt-document-audit.md) |
| Resolving a group address's datapoint type from its linked communication objects, with conflicts reported rather than guessed | ✅ Implemented | — |

See [Datapoint types](../knx-basics/04-datapoint-types.md) for what a DPT is and how
KNXBench names them.

> **Note**
>
> A supported main family does not prove every subtype, functional block or device
> uses it correctly. Check the [codec evidence](../../spec-audits/2026-10-07-dpt-document-audit.md)
> and the actual object's declaration rather than guessing from payload length.

## Group-address CSV

| Item | Status | Evidence |
| --- | --- | --- |
| Exporting "KNXBench group-address CSV v1" and re-importing it, unchanged | ✅ Implemented | Every group address in the reference project round-trips as `unchanged`, including names with commas, quotes and umlauts — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today) |
| Interoperability with ETS's own CSV/Excel group-address export, or `.esf` | ❌ Not implemented | No sample of either format exists to build against; KNXBench's format is its own, documented, and never presented as ETS-compatible — [COMPATIBILITY.md §4](../../COMPATIBILITY.md#4-not-supported) |
| Explicit, preview-confirmed re-addressing and unreferenced deletion through CSV | ✅ Implemented | Stable ids retain directional links on moves; linked deletes are refused; confirmation is bound to CSV/project state |
| Creating or restructuring group ranges through CSV | ❌ Not implemented | Existing ranges are selected by final address bounds — see [KNOWN_LIMITATIONS.md §39](../../KNOWN_LIMITATIONS.md#39-csv-import-never-re-addresses-deletes-or-manages-group-ranges) |

See [Working with group addresses](../user-guide/04-group-addresses.md) for the CSV
workflow itself.

## Buildings, topology, and structure

| Item | Status | Evidence |
| --- | --- | --- |
| Reading and editing buildings, floors, rooms, and building parts | ✅ Implemented | 22 building parts in the reference project import correctly — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today) |
| Reading and editing areas, lines, and individual addresses | ✅ Implemented | Verified as part of the same reference-project import and the topology editing covered in [Buildings and topology](../user-guide/03-buildings-and-topology.md) |
| Creating a project from scratch in the interface (rather than only importing one) | ✅ Implemented | Browser-verified — [KNOWN_LIMITATIONS.md §83](../../KNOWN_LIMITATIONS.md#83-the-from-scratch-launcher-is-browser-verified--resolved-2026-09-16-goal-task-17) |

## Device parameters

| Item | Status | Evidence |
| --- | --- | --- |
| Reading, storing and displaying a device's parameters from its application program | ✅ Implemented | 1,390 parameter values from the reference project import correctly — [COMPATIBILITY.md §2](../../COMPATIBILITY.md#2-verified-today) |
| Editing a parameter value in the open project, with validation and re-evaluation of the program's dynamic tree | ✅ Implemented | Described in [Devices and products](../user-guide/05-devices-and-products.md); the value is stored as an undo step and validated against the program's declared parameter chain |
| Writing an edited parameter value to a real device | 🟡 Partial or experimental | As part of a device download, verified on one device so far — see *Commissioning* below and [Downloading to a device](../user-guide/07-bus-and-interfaces.md#downloading-to-a-device) |

> **Note**
>
> Editing a value **in the project** and downloading it **to a physical device** are
> different operations. The editor is undoable. Device download is safety-gated and
> verified only within the narrow commissioning scope below; project undo cannot
> undo a hardware write.

## KNX Secure

| Item | Status | Evidence |
| --- | --- | --- |
| Data Secure, IP Secure, keyring handling | ❌ Not implemented | No sample key material has ever been available to verify against — [KNOWN_LIMITATIONS.md §8](../../KNOWN_LIMITATIONS.md#8-knx-secure-is-not-implemented) |

This is deferred behind real sample material becoming available, not abandoned. A
secured installation cannot currently be fully represented or monitored by KNXBench.

## Commissioning and device download

| Item | Status | Evidence |
| --- | --- | --- |
| Downloading a device: application tables and parameters | 🟡 Partial or experimental | `knx device download` and the **Download to device** tab, plan first and confirmed per device; verified on one device (MDT, mask `0701h`) with read-back, others unverified — [Downloading to a device](../user-guide/07-bus-and-interfaces.md#downloading-to-a-device), [KNOWN_LIMITATIONS.md §7](../../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked) |
| Programming an individual address | 🟡 Refused safety boundary | Confirmed starts fail before connection until device-specific durable recovery exists; not a working write or an active task — [ADR-0058](../../adr/0058-individual-address-reset-requires-durable-recovery.md) |
| The generic load/unload/reset/memory-write commissioning procedures | 🟡 Bounded implementation | One real memory-download device; property procedures remain simulator-verified. No generic real-device coverage — [KNOWN_LIMITATIONS.md §7](../../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked), [§92](../../KNOWN_LIMITATIONS.md#92-commissioning-phase-2-is-verified-against-a-simulator-this-project-wrote-and-has-never-addressed-a-device) |

> **Warning**
>
> KNXBench can download one verified device so far and cannot program individual
> addresses at the moment. For any other device you still need a commissioning tool
> that covers it.

## Bus communication

| Item | Status | Evidence |
| --- | --- | --- |
| KNXnet/IP discovery | ✅ Implemented | CLI and graphical bus monitor — [Bus monitor and KNXnet/IP](../user-guide/07-bus-and-interfaces.md) |
| Tunneling: connect, monitor, decode telegrams | ✅ Implemented | Verified live against one real gateway; a 34-minute session read 1,299 telegrams with zero drops and 100% of destination names resolved — [COMPATIBILITY.md §3](../../COMPATIBILITY.md#3-expected-but-unverified) |
| Routing (multicast) | 🟡 Partial or experimental, command line only | `knx bus route-monitor` and `knx bus route-send`; the graphical bus monitor supports tunneling only — [Bus monitor and KNXnet/IP](../user-guide/07-bus-and-interfaces.md) |
| Sending a group-value write from the bus monitor or the command line | ✅ Implemented | A live bus operation, distinct from device download; project Undo does not cover it |
| A line scan (finding which individual addresses answer on a line) | 🟡 Partial or experimental | Live and unthrottled by default; cannot identify a product, distinguish a busy device from an absent one, or cross a coupler — [KNOWN_LIMITATIONS.md §72](../../KNOWN_LIMITATIONS.md#72-line-scan-t17-an-unthrottled-scan-is-a-live-bus-cost-not-a-theoretical-one--shipped-2026-09-13-still-true) |
| KNXnet/IP against multiple gateway models | 🟡 Partial or experimental | Verified against exactly one gateway model so far — [COMPATIBILITY.md §3](../../COMPATIBILITY.md#3-expected-but-unverified) |

See [Bus monitor and KNXnet/IP](../user-guide/07-bus-and-interfaces.md) for the plain
statement of what KNXBench reads from and writes to a bus, and what it never does.

## Multi-user and collaboration

| Item | Status | Evidence |
| --- | --- | --- |
| Multiple browsers editing the same server-hosted project at once | ❌ Not implemented | One process holds exactly one project in memory with one shared undo stack; a second client's undo can undo the first client's command, with no conflict detection at all — [KNOWN_LIMITATIONS.md §63](../../KNOWN_LIMITATIONS.md#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all) |
| Distinguishing one connected user from another | ❌ Not implemented | The server's authentication is one shared password and one session cookie; a valid cookie says "someone knew the password", never who — [KNOWN_LIMITATIONS.md §63](../../KNOWN_LIMITATIONS.md#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all) |

> **Warning**
>
> If you deploy the web/Docker server for more than one person to reach, everyone who
> can reach it shares one project and one undo stack. Treat it as a single-operator tool
> until this changes.

[Manual index](../README.md) · Next: [Troubleshooting](03-troubleshooting.md) →
