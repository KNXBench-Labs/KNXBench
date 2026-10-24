# Devices dependent on vendor plug-ins

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-006)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Declared ETS download plug-ins refuse every download scope (24 October 2026)

**Field finding.** A read-only comparison against a real ETS-programmed device
whose application program declares an ETS download plug-in
(`Static/Extension/@EtsDownloadPlugin` with a `Baggage` DLL, mask `0701h`)
showed that the plan built from the product's base `Data` alone does not match
the device. Its largest planned segment answered `A_Memory_Read` with zero
octets, and the readable part differed from the base image in octets that no
parameter owns. The plug-in evidently writes data that KNXBench cannot
reproduce. Until now readiness graded such a program `untested`. The mandatory
pre-write backup stopped a complete or parameter download before its first
write, but a group-address-only partial download (address and association
tables) would have passed the backup and written.

**Boundary.** A program whose `Static/Extension` carries a non-empty
`EtsDownloadPlugin` attribute is refused for **every** download scope:
complete, parameters, group addresses and both, and a backup restore planned
from the product. The refusal has its own category, `vendor-download-plugin`,
and names this limitation and the declared GUID. It appears in
`knx device readiness`, the `knx device download` plan,
`/api/device-readiness`, the `/api/device-download/plan` refusal
(`downloadPreparationRefused` with `category`), the web download tab and
`knx products coverage`. The rule is the declaration, never a manufacturer or
product list. It is read on demand from the program's retained source XML
(`knx_productdb::code::ProgramCode::download_plugin`); no product-database
schema change was needed. An empty attribute names no plug-in and does not
refuse. Precedence is fixed: a program that is also of a kind no download
translates (mask not loaded through memory, another procedure style) keeps that
more fundamental refusal; the plug-in category covers what would otherwise
plan. In the maintainer's house corpus this moved one device from `untested`
to `vendor-download-plugin`; two plug-in devices keep `not-memory-mapped`. In
the local private product corpus (758 programs when measured), 7 programs
are refused with `vendor-download-plugin`. The pinned coverage test there
(`download_coverage_corpus.rs`) predates the corpus growth and still pins
105 packages / 467 programs; it fails on its package count before this rule
and was not re-pinned here.

**Still allowed.** Read-only `knx device compare` (CLI and server), comparison
adoption into the project, the byte-order probe and other diagnostics prepare
the same plan through the read-only preparation
(`prepare_device_download_for_reading`) and are not refused: reading the device
is how the mismatch was found.

**Not covered.** Programs imported from a legacy EX-IM product database
(ADR-0094) carry plug-in data in a `program_plugin` table that KNXBench does
not map. Such a program is not refused by this rule; its legacy download stays
`untested` with its own acknowledgement. What a plug-in writes, and whether a
download could ever reproduce it, stays unknown
([research §8.6.7](../research/knxnet-ip-and-bus/8-6-is-the-legacy-matrix-reconstructible-or-does-it-need-a-vendor.md#867-what-this-spike-did-not-settle-and-what-would-settle-it),
item 2).

<!-- BEGIN retained catalogue record -->

## 6. Devices behind vendor plug-in DLLs

**Update 2026-10-10.** Vendor code also reaches a project outside
manufacturer baggage: ETS app packages under `P-…/UserFiles/` and add-in data
under `P-…/AddinData/`. Neither is executed or unpacked, but the import report
does not say what they are. See §174.

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
<!-- END retained catalogue record -->
