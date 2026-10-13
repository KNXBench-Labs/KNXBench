# Devices dependent on vendor plug-ins

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-006)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

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
