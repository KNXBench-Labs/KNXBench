# ETS 5.7 schema-20 evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-177)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 177. ETS 5.7 projects (schema 20) rest on three public samples; sub-module object ids are not decoded

**Reported 2026-10-12 (issue #7, from #6).** An ETS 5.7.4 project was refused
with `no known-element table for schema version 20`: schema 20 had no table.

**Change.** Schema 20 imports with the schema-23 element table minus the
`Segment` level (devices directly under `Line`, medium on `Line`), measured on
three public ETS 5.7.1428 projects (research project-format §3.6). The import
report carries an `unsupported` entry naming that evidence boundary. A version
still without a table is refused with a message that names it and the
supported versions. Real ZipCrypto exports name nested members bare
(`0.xml`); they are rooted under the project part, which made the first real
protected ETS4/ETS5 files importable at all (§13).

**Remaining boundary.**

* Evidence is three small public projects. Entries they did not contain
  (`ProjectTraces`, `DeviceInstanceRef`, `BinaryData`, `UnassignedDevices`)
  are inherited from schema 23 by inference. Medium attributes other than
  `MediumTypeRefId` on a schema-20 `Line` (`DomainAddress`, `MulticastTTL`
  and the like, unmeasured) are reported and retained, not modelled.
* **Nested sub-module object ids are not decoded.** `RefId`s with `SM-`
  segments (`MD-4_M-15_MI-1_SM-1_M-1_MI-1-1-1_SM-1_O-3-0_R-1`) are reported
  as `MalformedRefId` errors. The object is still imported with its links,
  flags and verbatim `RefId`, but with object number 0 and no module
  instance. Seen on one device of one public sample (8 objects). Not
  schema-20 specific: the `SubModule` grammar is project-side ETS vocabulary
  (product-database Q6) that no part of KNXBench implements yet.
* Secure material in the XML (`ToolKey`, `Key`, `FDSK`, passwords) is retained
  as uninterpreted source and reported, never modelled (§8).
* Schema 22 (ETS 6.1) stays refused: its public samples contain no device.
* Not verified on the reporter's project or any larger ETS 5.7 installation.

Ledger `KL-177`.
<!-- END retained catalogue record -->
