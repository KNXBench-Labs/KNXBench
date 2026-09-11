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
| Reading and writing `.knxproj` schema 21 | KNX Association `KV v2.5` demo project (ETS 5.7), 4 devices, module-based application programs (ADR-0013), zero unknown-construct entries | this plan's spec + ADR-0013/ADR-0014 | `importing_the_kv_schema_21_project_succeeds_with_zero_unknown_constructs`, `a_schema_21_export_reimports_to_an_equal_domain_model` (`crates/knx-etsproj/src/lib.rs`, `.../export/schema21.rs`) |
| Reading `.knxproj` schema 23 | The ETS6 re-export of the reference project — module-based application-program handling is *inferred* from the schema-21 sample, not independently evidenced (no module-using schema-23 sample exists) — no round-trip claim | RESEARCH §2.4/§3.3 + this plan's scope decision | `importing_the_ets6_schema_23_project_succeeds_but_carries_no_round_trip_claim` |
| Detecting a password-protected container and refusing it by name, rather than attempting decryption | A hand-built container shaped like a protected project (nested `<P-xxxx>.zip`) | IMPORT_EXPORT §2 [D for the decryption schemes; V for detection] | `a_password_protected_project_is_detected_and_named` (`knx-etsproj/src/container.rs`) |
| Writing schema-11 containers our own reader reads back to a semantically equal model | The reference project, roundtripped | IMPORT_EXPORT §9, ADR-0007 | `roundtrip_model_is_semantically_equal` (`knx-etsproj/tests/roundtrip.rs`) |
| Every opaque byte surviving a roundtrip unchanged | The reference project's 38 container entries | IMPORT_EXPORT §5 | `roundtrip_opaque_bytes_are_hash_identical` (`knx-etsproj/tests/roundtrip.rs`) |
| A second roundtrip changing nothing the first one did not already normalize | The reference project, exported twice | IMPORT_EXPORT §9 | `a_second_roundtrip_changes_nothing_further` (`knx-etsproj/tests/roundtrip.rs`) |
| Cross-checking the import against `xknxproject`'s own reading, on the parts it is not known to lose | The reference project | RESEARCH §7.1 | `group_addresses_agree_with_the_oracle_by_address_and_name`, `devices_agree_with_the_oracle_except_for_the_one_it_loses`, `the_project_metadata_agrees_with_the_oracle`, `our_linked_communication_objects_match_the_oracle_count` (`knx-etsproj/tests/oracle_xknxproject.rs`) |
| Bad input (empty file, missing project part, truncated XML, no namespace, unsupported schema version, invalid address, duplicate id, dangling reference, an oversized declared entry size, 10000 levels of nesting) never panics and always produces a named error or a report entry | Ten hand-built malformed containers | CLAUDE.md's malformed-input testing rule | `knx-etsproj/tests/malformed_input.rs` (10 tests) |
| KNXnet/IP tunnelling | One gateway at `192.0.2.1:3671`, 280 telegrams captured in a 300 s window; `GroupValueWrite`, `GroupValueRead` (30), `GroupValueResponse` (30); all 280 resolved to a named group address from the project | RESEARCH §8.1 [V] | none — a Session 0 research script, not an automated test; Session 6 builds the tested `BusConnection` |
| Ingesting the reference project's manufacturer data completely: 4 manufacturers, 24 source files, 12 application programs, 5,630 `ComObjectRef` rows, 11,311 parameters, 3,846 enumeration values, 48,057 translations | The same ETS 4.1.8 reference project's `<M-xxxx>/*` entries | IMPORT_EXPORT §10, RESEARCH §4 [V] | `the_reference_projects_manufacturer_data_ingests_completely` (`knx-productdb/tests/golden_reference_products.rs`) |
| Every ingested manufacturer blob verifying against its own content hash | The same manufacturer data | ADR-0011 | `every_blob_verifies_against_its_own_hash` (`knx-productdb/tests/golden_reference_products.rs`) |
| A second ingest of already-known manufacturer files parsing nothing new | The same manufacturer data, ingested twice | ADR-0011, ROADMAP's "skipping existing entries" | `a_second_ingest_of_the_same_files_stores_nothing_new` (`knx-productdb/tests/golden_reference_products.rs`), `a_second_import_into_the_same_product_db_skips_every_file` (`knx-app/tests/product_db.rs`) |
| Export producing byte-identical output whether manufacturer data came from the project's opaque store or the shared product database | The reference project, imported and exported both ways | IMPORT_EXPORT §10, ADR-0011 | `export_is_byte_identical_with_and_without_the_product_database` (`knx-app/tests/product_db.rs`) |
| A project opening and naming its gap when the product database is missing, rather than failing to open or guessing | The reference project, its product database deleted after import | ADR-0005, ADR-0011 | `a_project_opens_and_names_its_gap_when_the_product_database_is_gone` (`knx-app/tests/product_db.rs`) |
| Enriching communication objects from the application program into `Override::Absent` slots only, never into `Empty`/`Malformed`/instance-level values | Hand-built fixtures pinning each of the four `Override` states | ADR-0012, DATA_MODEL §3 | `an_absent_text_is_filled_at_the_program_layer`, `an_empty_instance_attribute_is_never_overwritten`, `an_instance_value_is_never_overwritten` (`knx-productdb/src/enrich.rs`) |
| An ambiguous, space-separated `DatapointType` list filling nothing and being reported rather than guessed | A `ComObjectRef` carrying two alternatives | RESEARCH §4.2, ADR-0012 | `a_datapoint_type_list_fills_nothing_and_is_reported` (`knx-productdb/src/enrich.rs`) |
| Installing a standalone `.knxprod` product package (no accompanying `.knxproj`) at master data scheme 11 or scheme 20, atomically, with content-addressed storage and idempotent re-install | 5 real-world files: 3 at scheme 11 (`646704-04_ETS4_2012_47_DE_EN.knxprod`, `Weinzierl_730_KNX_IP_Interface_ETS4.knxprod`, `Weinzierl_730_KNX_IP_Interface_ETS4_v1.knxprod`), 2 at scheme 20 (`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`, `Dummy_Applikation_Secure.knxprod`) — the `.knxprod`/`.knxproj` container family per *Project Schema23 v01.00.00* §4.2.2-§4.2.3 (`knx_master.xml` root, `http://knx.org/xml/project/{scheme}`), the "manufacturer product template as input to tool-side configuration" role per *03_01_01 Architecture v03.00.02 AS* §6.2 | `installs_the_readable_corpus` (`knx-productdb/tests/standalone_packages.rs`) |
| Rejecting a malformed or unsafe standalone package (invalid ZIP, encrypted member, path traversal, duplicate member, oversized member, missing `knx_master.xml`, unsupported namespace, a full `.knxproj` project archive, or a legacy `.vd2` file) as a typed error with no rows published | Hand-built malformed archives plus the real `Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2` (a pre-2013 ETS2-era SFX/`.vd_` container, not the same ZIP/XML family) | This plan's spec | `malformed_and_unsupported_packages_leave_no_rows` (`knx-productdb/tests/standalone_packages.rs`), `malformed_and_legacy_product_uploads_are_typed_bad_requests` (`knx-server/tests/http_product_install.rs`) |
| Exporting "KNXBench group-address CSV v1" and re-parsing/re-planning it against the same project yields every group address as `unchanged` with no problems, including names containing commas, quotes, and umlauts — this proves the writer and the reader agree with each other, **not** interoperability with ETS's own CSV export (see §4) | The reference project, every one of its group addresses | IMPORT_EXPORT §11 | `exporting_and_replanning_the_reference_project_is_entirely_unchanged` (`knx-app/tests/csv_roundtrip.rs`) |
| Decoding and encoding DPT main types 1, 2, 3, 5, 6 (except `6.020`), 7, 8, 9, 12, 13, 14, 16, 17, 18 against their wire form, including both sentinel-collision rulings (`8.010`'s printed 327.67% maximum vs. its identical invalid-data code; main type 9's arithmetic maximum at `M=2047,E=15` vs. the same collision) and resolving a group address's DPT by inference over its linked communication objects, conflicts reported not guessed | `03_07_02 Datapoint Types v02.02.01 AS` §§3.1-3.19, excluding the sections in that range for the main types this slice does not implement (4, 10, 11, 15); `docs/RESEARCH.md` §6.1 (194/514 group addresses resolve to no DPT, 110/514 have no linked communication object) | `u32_round_trips_min_max_zero_and_interior`, `v16_percent_maximum_collides_with_the_invalid_sentinel`, `f16_encode_rejects_the_value_that_would_collide_with_the_sentinel`, `a14_worked_example_round_trips_byte_for_byte`, `scene_control_round_trips_min_max_zero_and_interior` (`knx-core/src/dpt/codec.rs`); `one_linked_object_stating_a_dpt_resolves_to_single`, `two_linked_objects_stating_different_dpts_resolve_to_a_sorted_deduplicated_conflict` (`knx-core/src/dpt/resolve.rs`); `dry_run_with_a_project_resolving_to_a_single_dpt_encodes_using_it`, `a_group_address_with_conflicting_dpts_fails_naming_both_and_exits_nonzero` (`apps/knx-cli/tests/cli_bus_dpt.rs`); `encode_promotes_out_of_range_short_instead_of_corrupting_apci` (`knx-net/src/cemi.rs`) |

Every export this application produces is unsigned — see §3, "ETS re-import
of a file we export."

## 3. Expected but unverified

| Item | Status | What would move it to verified |
| --- | --- | --- |
| Schema 12 (ETS 4) | Documented only [D] | Import a real ETS 4 project of that schema and reconcile the unknown-construct report to empty |
| Schema 13, 14 (ETS 5 up to 5.6) | Documented only [D] | Same, with an ETS 5 sample |
| Schema 20 (ETS 5.7) **`.knxproj` project** import | Documented only [D] — distinct from §2's now-verified standalone `.knxprod` scheme-20 *product-package* install; a full ETS 5.7 project has not been imported | Same, with an ETS 5.7 sample |
| Schema 22 (ETS 6.x, early) | Documented only [D] | Same, with an ETS 6.0–6.2 sample |
| Schema 23 (ETS 6.3.7959.0) | Container and content model diffed against schema 11 [V] (RESEARCH §2.4/§3.3); importer built, but module handling inferred not evidenced | Obtain an independent, module-using schema-23 sample and reconcile its unknown-construct report to empty |
| Password-protected projects, schema < 21 (ZipCrypto) | Code path derived from `xknxproject` source [V], never executed here | Open a real protected ETS4/ETS5 project with its password |
| Password-protected projects, schema ≥ 21 (AES, PBKDF2) | As above | Open a real protected ETS6 project with its password |
| ETS re-import of a file we export | Untested (risk R9) | Export a project and open it in a real ETS installation; record the result either way |
| KNXnet/IP against other gateway models | One model tested | Test discovery, tunnelling and routing against further gateways |
| Group Monitor GUI (T15) — starting a tunnelling session from `apps/knx-web`/`apps/knx-desktop`, watching telegrams decode live in a table, and sending a group write back through it | Code and tests only [D] — every test in `apps/knx-server/src/bus.rs` and its `tests/http_bus_monitor.rs`/`http_bus_write.rs` drives a `FakeConnector`/`FakeTunnel`, never a real socket; nothing in this GUI has been run against a physical KNX installation in this branch (`KNOWN_LIMITATIONS.md` §62) | Start a session against a real gateway, watch real telegrams decode and a real write go out, and record the result here |

An unverified row is not a promise. Until it is verified, the honest statement
is that we expect it to work and have not shown that it does.

## 4. Not supported

| Item | Reason |
| --- | --- |
| Devices whose configuration depends on a vendor plug-in DLL | The behaviour lives in the binary; it is preserved and reported, never executed (RESEARCH §7, risk R5) |
| Commissioning and device download | Not implemented yet, and — per the user's 2026-09-11 ruling — not permanently excluded either: required, blocked. The generic load/unload/reset/memory-write procedures are documented in the KNX Standard (RESEARCH §8.4, R5 spike); a product-specific `Legacy*` compatibility-flag matrix and vendor DLLs remain undocumented outside ETS tooling and are why it hasn't started, alongside bricking risk on real hardware (RESEARCH §8.3/§8.4) ([KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)) |
| KNX Secure | No sample key material to verify against; the subsystem exists but stays empty (RESEARCH §9) |
| Direct `.knxprod` import for master data scheme ≥ 12, **except schemes 11 and 20** (§2, standalone package install) | Schemes 12-19, 21, 22 have no standalone sample tested yet; `.vd2` is a distinct pre-2013 legacy container, permanently unsupported, not an encryption question |
| Device parameter editing | The `@test` value grammar is now documented (risk R3, RESEARCH §4.3); the `Dynamic` tree's structural grammar is corpus-observed only. `knx-productdb` now parses, stores and evaluates the tree headlessly, including expanding a `Module` node into its `ModuleDef`'s own tree (T18 slices 1 and 2, both 2026-09-11), but nothing wires that evaluation into a UI, and no parameter value is ever written — parameter values are preserved but not editable |
| ETS's own "Export Group Addresses" CSV/Excel format, `.esf` (OPC export) | No sample of either exists in this repository or in the KNX Standard v3.0.0 corpus, and neither is a KNX Association standard. KNXBench instead defines and documents its own format, "KNXBench group-address CSV v1" (IMPORT_EXPORT §11) — never presented as ETS-compatible. If a genuine ETS CSV sample is obtained, adding a matching column profile to the importer is the stated upgrade path |

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
