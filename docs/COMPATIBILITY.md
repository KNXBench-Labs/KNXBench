# Compatibility

What this application can read, write and talk to — and, in every case, on what
evidence.

The table structure here matters more than today's contents. This document is
expected to change every time a new sample project is imported or a new gateway
is tested; a row moves from "expected but unverified" to "verified" only when
someone has actually run it.

## 1. Wording policy

User-facing text says **"KNX-compatible"**. Never "KNX certified", never "full
ETS compatibility".

Two independent reasons:

- **Trademark.** "KNX" is a registered trademark. An independent tool may
  interoperate, but cannot call itself KNX-certified without membership and
  conformance testing (RESEARCH §10) [D].
- **Vendor plug-in binaries.** Part of some devices' configuration behaviour
  lives inside vendor plug-in DLLs shipped in the project file (RESEARCH §7)
  [V]. No independent tool can reproduce that behaviour, so full ETS
  compatibility is not achievable and must not be claimed.

This applies to the UI, the CLI, the README, error messages and release notes
alike.

## 2. Verified today

Every row below names the test that verifies it — CLAUDE.md forbids a
compatibility claim with no test behind it.

| Capability | Scope | Evidence | Test |
| --- | --- | --- | --- |
| Reading `.knxproj` schema 11 | One real ETS 4.1.8 project: 36 devices (35 addressed + 1 unassigned), 514 group addresses, 907 communication object instances, 569 send / 27 receive links, 1390 parameter values (1174 plain + 216 union), 22 building parts, 3 binary data references | RESEARCH §3 [V] | `the_reference_project_imports_with_the_measured_counts`, `nothing_in_the_reference_project_is_unknown_or_lost` (`knx-etsproj/tests/golden_reference_project.rs`) |
| Reading `.knxproj` schema 23 | The same project re-exported from ETS 6.3.7959.0: 35 devices, 514 group addresses, 691 communication object instances, 1343 parameter values | RESEARCH §2.4/§3.3, reproducible with `tools/inspect_knxproj.py` [V] — format changes vs. schema 11 catalogued there | none — see "detects and refuses" below; no schema-23 importer exists |
| Detecting schema 23 and refusing it by name, rather than misreading it through the schema-11 table | The same ETS6 export | RESEARCH §3.3 (the load-bearing format differences this would misread) | `importing_the_ets6_project_fails_with_a_named_reason_not_wrong_data` (`knx-etsproj/src/lib.rs`) |
| Detecting a password-protected container and refusing it by name, rather than attempting decryption | A hand-built container shaped like a protected project (nested `<P-xxxx>.zip`) | IMPORT_EXPORT §2 [D for the decryption schemes; V for detection] | `a_password_protected_project_is_detected_and_named` (`knx-etsproj/src/container.rs`) |
| Writing schema-11 containers our own reader reads back to a semantically equal model | The reference project, roundtripped | IMPORT_EXPORT §9, ADR-0007 | `roundtrip_model_is_semantically_equal` (`knx-etsproj/tests/roundtrip.rs`) |
| Every opaque byte surviving a roundtrip unchanged | The reference project's 38 container entries | IMPORT_EXPORT §5 | `roundtrip_opaque_bytes_are_hash_identical` (`knx-etsproj/tests/roundtrip.rs`) |
| A second roundtrip changing nothing the first one did not already normalize | The reference project, exported twice | IMPORT_EXPORT §9 | `a_second_roundtrip_changes_nothing_further` (`knx-etsproj/tests/roundtrip.rs`) |
| Cross-checking the import against `xknxproject`'s own reading, on the parts it is not known to lose | The reference project | RESEARCH §7.1 | `group_addresses_agree_with_the_oracle_by_address_and_name`, `devices_agree_with_the_oracle_except_for_the_one_it_loses`, `the_project_metadata_agrees_with_the_oracle`, `our_linked_communication_objects_match_the_oracle_count` (`knx-etsproj/tests/oracle_xknxproject.rs`) |
| Bad input (empty file, missing project part, truncated XML, no namespace, unsupported schema version, invalid address, duplicate id, dangling reference, an oversized declared entry size, 10000 levels of nesting) never panics and always produces a named error or a report entry | Ten hand-built malformed containers | CLAUDE.md's malformed-input testing rule | `knx-etsproj/tests/malformed_input.rs` (10 tests) |
| KNXnet/IP tunnelling | One gateway at `192.0.2.1:3671`, 280 telegrams captured in a 300 s window; `GroupValueWrite`, `GroupValueRead` (30), `GroupValueResponse` (30); all 280 resolved to a named group address from the project | RESEARCH §8.1 [V] | none — a Session 0 research script, not an automated test; Session 6 builds the tested `BusConnection` |

Every export this application produces is unsigned — see §3, "ETS re-import
of a file we export."

## 3. Expected but unverified

| Item | Status | What would move it to verified |
| --- | --- | --- |
| Schema 12 (ETS 4) | Documented only [D] | Import a real ETS 4 project of that schema and reconcile the unknown-construct report to empty |
| Schema 13, 14 (ETS 5 up to 5.6) | Documented only [D] | Same, with an ETS 5 sample |
| Schema 20 (ETS 5.7) | Documented only [D] | Same, with an ETS 5.7 sample |
| Schema 21, 22 (ETS 6.x, early) | Documented only [D] | Same, with an ETS 6.0–6.2 sample |
| Schema 23 (ETS 6.3.7959.0) | Container and content model diffed against schema 11 [V] (RESEARCH §2.4/§3.3); no importer built yet | Build the schema-23-aware importer and reconcile its unknown-construct report to empty |
| Password-protected projects, schema < 21 (ZipCrypto) | Code path derived from `xknxproject` source [V], never executed here | Open a real protected ETS4/ETS5 project with its password |
| Password-protected projects, schema ≥ 21 (AES, PBKDF2) | As above | Open a real protected ETS6 project with its password |
| ETS re-import of a file we export | Untested (risk R9) | Export a project and open it in a real ETS installation; record the result either way |
| KNXnet/IP against other gateway models | One model tested | Test discovery, tunnelling and routing against further gateways |

An unverified row is not a promise. Until it is verified, the honest statement
is that we expect it to work and have not shown that it does.

## 4. Not supported

| Item | Reason |
| --- | --- |
| Devices whose configuration depends on a vendor plug-in DLL | The behaviour lives in the binary; it is preserved and reported, never executed (RESEARCH §7, risk R5) |
| Commissioning and device download | Bricking risk, an undocumented `Legacy*` matrix, vendor DLLs (RESEARCH §8.3) |
| KNX Secure | No sample key material to verify against; the subsystem exists but stays empty (RESEARCH §9) |
| Direct `.knxprod` import for master data scheme ≥ 12 | The encryption layer is unresolved (RESEARCH §10) |
| Device parameter editing | The `Dynamic` tree grammar is unresearched (risk R3); parameter values are preserved but not interpreted |

Each row here has a matching entry in
[KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) stating what it costs the user and
what would lift it.

## 5. How compatibility is reported to the user

Compatibility is not a document the user has to find. Every import produces an
`ImportReport` with three sections that answer it directly
([IMPORT_EXPORT.md](IMPORT_EXPORT.md)):

- **`unknown`** — elements and attributes outside the known-element list for
  the detected schema version, with source path, XPath and frequency. A
  non-empty list on a new schema version is expected, and is the work item for
  supporting it.
- **`unsupported`** — features present in the project that this application
  will not act on, such as a device carrying a plug-in DLL, with the
  consequence stated (that device is read-only).
- **`conflicts`** — contradictions in the source data, such as linked objects
  declaring different datapoint types on one group address. Reported, never
  silently resolved.

Nothing is dropped without appearing in one of these three lists or in the
opaque store. If it is, that is an importer bug.
