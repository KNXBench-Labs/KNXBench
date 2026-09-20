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
| Detecting a password-protected container and refusing it by name when no password is given | A hand-built container shaped like a protected project (nested `<P-xxxx>.zip`) | IMPORT_EXPORT §2 [V] | `a_password_protected_project_is_detected_and_named` (`knx-etsproj/src/container.rs`) |
| Decrypting a schema < 21 (ETS4/ETS5) password-protected `.knxproj`'s ZipCrypto-encrypted nested payload with its password — a wrong password is reported as a typed error, never a panic or silently wrong bytes; an AES-protected (schema ≥ 21 / ETS6) nested payload is still refused by name, not attempted | ZipCrypto is a 1990s stream cipher with no real security margin — this is read-only support for opening a file the caller already has the password to, not a security feature (see `knx_secure::zipcrypto`'s module docs). Tested against synthetic fixtures built with the independent Info-ZIP `zip` CLI, never by any code path in this repository — the algorithm is fully specified by APPNOTE.TXT and does not vary by writer, so this verifies the algorithm, not one vendor's export quirks. Two caveats on the coverage: both container-level fixtures carry the **Info-ZIP DOS-time** check-byte convention, so the PKZIP CRC-high-byte convention is exercised only at the `knx-secure` unit level, never end to end through a container; and neither has been run against a real password-protected ETS4/ETS5 export (§3). A decrypted entry's decompressed bytes are additionally checked against the entry's declared size and CRC-32, so a wrong password that slips past the one-byte check (about 1 in 128) is still reported as `WrongPassword` rather than handed on as plaintext. The AES refusal reads the entry's **raw on-disk** compression-method field, not the `zip` crate's parsed method, which that crate overwrites with the entry's real underlying method as soon as it parses a WinZip AES extra field (0x9901) | PKWARE APPNOTE.TXT v6.3.3 §6.1.3-§6.1.7 [D]; `knx_secure::zipcrypto` module docs | `the_right_password_decrypts_the_zipcrypto_fixture`, `the_stored_fixture_decrypts_with_the_right_password`, `a_wrong_password_is_a_typed_error_not_a_panic_or_garbage`, `a_wrong_password_that_survives_the_check_byte_is_caught_by_the_entrys_crc`, `an_aes_payload_is_refused_by_name_and_never_blamed_on_the_password`, `a_nested_entry_colliding_with_an_outer_path_is_refused_not_shadowed`, `open_with_password_on_an_unprotected_project_behaves_like_open` (`knx-etsproj/src/container.rs`); `decrypts_the_fixture_with_the_right_password`, `a_wrong_password_is_a_typed_error_not_garbage`, and 5 more (`knx-secure/src/zipcrypto.rs`) |
| Writing schema-11 containers our own reader reads back to a semantically equal model | The reference project, roundtripped | IMPORT_EXPORT §9, ADR-0007 | `roundtrip_model_is_semantically_equal` (`knx-etsproj/tests/roundtrip.rs`) |
| Every opaque byte surviving a roundtrip unchanged | The reference project's 38 container entries | IMPORT_EXPORT §5 | `roundtrip_opaque_bytes_are_hash_identical` (`knx-etsproj/tests/roundtrip.rs`) |
| A second roundtrip changing nothing the first one did not already normalize | The reference project, exported twice | IMPORT_EXPORT §9 | `a_second_roundtrip_changes_nothing_further` (`knx-etsproj/tests/roundtrip.rs`) |
| Cross-checking the import against `xknxproject`'s own reading, on the parts it is not known to lose | The reference project | RESEARCH §7.1 | `group_addresses_agree_with_the_oracle_by_address_and_name`, `devices_agree_with_the_oracle_except_for_the_one_it_loses`, `the_project_metadata_agrees_with_the_oracle`, `our_linked_communication_objects_match_the_oracle_count` (`knx-etsproj/tests/oracle_xknxproject.rs`) |
| Bad input (empty file, missing project part, truncated XML, no namespace, unsupported schema version, invalid address, duplicate id, dangling reference, an oversized declared entry size, 10000 levels of nesting) never panics and always produces a named error or a report entry | Ten hand-built malformed containers | CLAUDE.md's malformed-input testing rule | `knx-etsproj/tests/malformed_input.rs` (10 tests) |
| KNXnet/IP tunnelling | One gateway at `192.0.2.1:3671`, 280 telegrams captured in a 300 s window; `GroupValueWrite`, `GroupValueRead` (30), `GroupValueResponse` (30); all 280 resolved to a named group address from the project | RESEARCH §8.1 [V] | none — a Session 0 research script, not an automated test; Session 6 builds the tested `BusConnection` |
| Ingesting the reference project's manufacturer data completely: 4 manufacturers, 24 source files, 12 application programs, 5,630 `ComObjectRef` rows, 11,311 parameters, 3,846 enumeration values, 48,190 translations (48,057 program-scope, plus 109 catalog and 24 hardware since T32, 2026-09-12) | The same ETS 4.1.8 reference project's `<M-xxxx>/*` entries | IMPORT_EXPORT §10, RESEARCH §4 [V] | `the_reference_projects_manufacturer_data_ingests_completely` (`knx-productdb/tests/golden_reference_products.rs`) |
| Every ingested manufacturer blob verifying against its own content hash | The same manufacturer data | ADR-0011 | `every_blob_verifies_against_its_own_hash` (`knx-productdb/tests/golden_reference_products.rs`) |
| A second ingest of already-known manufacturer files parsing nothing new | The same manufacturer data, ingested twice | ADR-0011, ROADMAP's "skipping existing entries" | `a_second_ingest_of_the_same_files_stores_nothing_new` (`knx-productdb/tests/golden_reference_products.rs`), `a_second_import_into_the_same_product_db_skips_every_file` (`knx-app/tests/product_db.rs`) |
| Export producing byte-identical output whether manufacturer data came from the project's opaque store or the shared product database | The reference project, imported and exported both ways | IMPORT_EXPORT §10, ADR-0011 | `export_is_byte_identical_with_and_without_the_product_database` (`knx-app/tests/product_db.rs`) |
| A project opening and naming its gap when the product database is missing, rather than failing to open or guessing | The reference project, its product database deleted after import | ADR-0005, ADR-0011 | `a_project_opens_and_names_its_gap_when_the_product_database_is_gone` (`knx-app/tests/product_db.rs`) |
| Enriching communication objects from the application program into `Override::Absent` slots only, never into `Empty`/`Malformed`/instance-level values | Hand-built fixtures pinning each of the four `Override` states | ADR-0012, DATA_MODEL §3 | `an_absent_text_is_filled_at_the_program_layer`, `an_empty_instance_attribute_is_never_overwritten`, `an_instance_value_is_never_overwritten` (`knx-productdb/src/enrich.rs`) |
| An ambiguous, space-separated `DatapointType` list filling nothing and being reported rather than guessed | A `ComObjectRef` carrying two alternatives | RESEARCH §4.2, ADR-0012 | `a_datapoint_type_list_fills_nothing_and_is_reported` (`knx-productdb/src/enrich.rs`) |
| Installing a standalone `.knxprod` product package (no accompanying `.knxproj`) at master data scheme 11 or scheme 20, atomically, with content-addressed storage and idempotent re-install | 4 real-world files: 2 at scheme 11 (`646704-04_ETS4_2012_47_DE_EN.knxprod`, `Weinzierl_730_KNX_IP_Interface_ETS4.knxprod`), 2 at scheme 20 (`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`, `Dummy_Applikation_Secure.knxprod`) — the `.knxprod`/`.knxproj` container family per *Project Schema23 v01.00.00* §4.2.2-§4.2.3 (`knx_master.xml` root, `http://knx.org/xml/project/{scheme}`), the "manufacturer product template as input to tool-side configuration" role per *03_01_01 Architecture v03.00.02 AS* §6.2 | `installs_the_readable_corpus` (`knx-productdb/tests/standalone_packages.rs`) |
| Rejecting a malformed or unsafe standalone package (invalid ZIP, encrypted member, path traversal, duplicate member, oversized member, missing `knx_master.xml`, unsupported namespace, a full `.knxproj` project archive, or a legacy `.vd2` file) as a typed error with no rows published | Hand-built malformed archives plus the real `Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2` (a pre-2013 ETS2-era SFX/`.vd_` container, not the same ZIP/XML family). Reverified 2026-09-17 against the restored local 77,265-byte fixture; the focused rejection test and complete `knx-productdb` suite passed. | This plan's spec | `malformed_and_unsupported_packages_leave_no_rows`, `rejects_the_real_legacy_vd2_corpus_file_with_its_hash_and_size` (`knx-productdb/tests/standalone_packages.rs`), `malformed_and_legacy_product_uploads_are_typed_bad_requests` (`knx-server/tests/http_product_install.rs`) |
| Exporting "KNXBench group-address CSV v1" and re-parsing/re-planning it against the same project yields every group address as `unchanged` with no problems, including names containing commas, quotes, and umlauts — this proves the writer and the reader agree with each other, **not** interoperability with ETS's own CSV export (see §4) | The reference project, every one of its group addresses | IMPORT_EXPORT §11 | `exporting_and_replanning_the_reference_project_is_entirely_unchanged` (`knx-app/tests/csv_roundtrip.rs`) |
| Decoding and encoding DPT main types 1, 2, 3, 4, 5, 6 (except `6.020`), 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19 against their wire form, including both sentinel-collision rulings (`8.010`'s printed 327.67% maximum vs. its identical invalid-data code; main type 9's arithmetic maximum at `M=2047,E=15` vs. the same collision) and resolving a group address's DPT by inference over its linked communication objects, conflicts reported not guessed | `03_07_02 Datapoint Types v02.02.01 AS` §§3.1-3.20, excluding the sections in that range for the main types this codec does not implement (20 and above); `docs/RESEARCH.md` §6.1 (194/514 group addresses resolve to no DPT, 110/514 have no linked communication object) | `u32_round_trips_min_max_zero_and_interior`, `v16_percent_maximum_collides_with_the_invalid_sentinel`, `f16_encode_rejects_the_value_that_would_collide_with_the_sentinel`, `a14_worked_example_round_trips_byte_for_byte`, `scene_control_round_trips_min_max_zero_and_interior`, `access_data_matches_example_6`, `date_century_encoding_matches_example_5`, `datetime_round_trips_a_fully_valid_value` (`knx-core/src/dpt/codec.rs`); `one_linked_object_stating_a_dpt_resolves_to_single`, `two_linked_objects_stating_different_dpts_resolve_to_a_sorted_deduplicated_conflict` (`knx-core/src/dpt/resolve.rs`); `dry_run_with_a_project_resolving_to_a_single_dpt_encodes_using_it`, `a_group_address_with_conflicting_dpts_fails_naming_both_and_exits_nonzero` (`apps/knx-cli/tests/cli_bus_dpt.rs`); `encode_promotes_out_of_range_short_instead_of_corrupting_apci` (`knx-net/src/cemi.rs`) |

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
| Password-protected projects, schema < 21 (ZipCrypto), against a real ETS4/ETS5 export | Decryption is implemented and tested against a synthetic, APPNOTE-conformant fixture (§2); never run against a real password-protected ETS4/ETS5 project | Open a real protected ETS4/ETS5 project with its password |
| Password-protected projects, schema ≥ 21 (AES, PBKDF2) | PBKDF2 key derivation verified against the KNX Standard's own test vectors (`knx-secure/src/lib.rs`); container decryption not implemented — `Container::open_with_password` refuses an AES-protected nested payload by name (`ContainerError::UnsupportedEncryption`) rather than attempting it | Verify AES container decryption against a real protected ETS6 project with its password, then implement it the same way ZipCrypto was |
| ETS re-import of a file we export | Untested (risk R9) | Export a project and open it in a real ETS installation; record the result either way |
| KNXnet/IP against other gateway models | One model tested | Test discovery, tunnelling and routing against further gateways |
| Group Monitor GUI (T15) — tunnelling receive, project-name resolution, and DPT decode | Verified [V] on 2026-09-16 against one real gateway and the real schema-23 `Unser Zuhause` project: a 107-second passive session received 65 telegrams, resolved 65/65 destination names, decoded 10 values, and reported zero drops. A preceding 133-second empty-project session received 52 telegrams with zero drops. Re-verified [V] again 2026-09-19 against the same gateway and project: a session already running roughly 30 minutes before this task picked it up, polled it, and stopped it, for a total of 2040 seconds (34 minutes) from first telegram to stop — 1299 telegrams from 20 sources to 61 destinations, 100% names resolved, 209 of 1098 value-bearing telegrams decoded through their DPT (the remaining 889 carry no declared DPT in the project, not a main-type gap), zero drops. That run also confirmed the app's own single-session `409` live (not just against the fake tunnel) and found that the physical gateway itself accepts only one concurrent tunnel connection, ahead of and independent from that guard. No group read/write/response, management request, scan, or `/api/bus/write` call occurred in either run (`KNOWN_LIMITATIONS.md` §62). | Repeat on other gateway models and for sessions longer than 34 minutes. Transmit behavior requires separate explicit hardware authorization and remains unverified. |

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
| Writing a ZipCrypto-protected `.knxproj` | Deliberate, permanent: ZipCrypto is a broken 1990s cipher (`knx_secure::zipcrypto` module docs) kept only to read a file the caller already has the password to. Nothing in this codebase produces one — `knx_secure::zipcrypto` exposes decryption only, no encrypt function exists anywhere in the workspace |

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

## 6. Specification questions resolved against the Standard, not errata

Findings that settle an apparent gap or contradiction by reading further in
the Standard, rather than by concluding the printed text is wrong and
choosing not to follow it — the opposite of §7's list below, which is why
these live here instead of there.

**Q1 (task C18, blocks task C13). Must a System B device implement
`PID_DOWNLOAD_COUNTER` at all before a Partial Download is attempted?** No —
its availability is conditional, and the Standard states the consequence of
its absence explicitly rather than leaving it undefined. RES §4.2.30.1,
p. 41, defines `PID_DOWNLOAD_COUNTER` (`DPT_Value_2_Ucount`, 7.010) as part
of the Coupler Model 2.0's Partial Download support; RES p. 320, in the same
model's Network Management chapter, adds the operative sentence: *"If
PID_DOWNLOAD_COUNTER is not available for the part to be downloaded, then
the MaC shall not perform a Partial Download."* That is a normative
instruction, not an inference — task C13 implements against it directly:
read the Property before attempting a partial download, and fall back to a
full download whenever it is absent or unreadable, rather than treating its
absence as an error.

**Q2 (task C18, settles task C1's tolerant branch). Is CP §3.5.3's
`PID_PROGRAM_VERSION` write to the Group Address/Association/Group Object
Table objects errata, or Standard-permitted?** Permitted, not errata. CP
§3.5.3's Group Address Table variant Nr. 06, p. 54, and Association Table
variant Nr. 06, p. 56 (the Group Object Table variant likewise, pp. 51-52),
each instruct `PropertyWrite(..., PID_PROGRAM_VERSION)` on the table object
itself, while CP §3.5.2's own table steps 08/09/10, pp. 43-44, list no such
write for the same objects, and RES's own per-object property tables are
silent on it too: Table 77 (Group Address Table, p. 238), Table 80
(Association Table, p. 249) and Table 85 (Group Object Table, p. 270) do not
list `PID_PROGRAM_VERSION` among the object's properties; only Table 90
(Application Program 1, p. 288) and Table 91 (Application Program 2, p. 290)
do. That silence is not a prohibition: RES §4.2.13.1.3, p. 34, says outright
that *"the use of Program Version is Profile dependent"* and its *"mandatory
- or optional access rights"* are *"specified in [17]"* (Volume 6 Profiles)
— RES's own Realisation-Type tables were never meant to be the last word on
this Property. Volume 6 Profiles Annex A §A.1.1, p. 133, states the general
rule: *"any Interface Object or any Property in an Interface Object that is
not listed is optional."* Annex A's own per-object tables (A.2.4 Group
Address Table Object, p. 143; A.2.5 Association Table Object, p. 145; A.2.8
Group Object Table Object, p. 149) also do not list PID 13 for any Profile,
System B (masks 07B0h/17B0h) included — so under Annex A's own rule,
`PID_PROGRAM_VERSION` is *optional* on these three object types, not
forbidden. CP §3.5.3's writes are therefore a legitimate use of an optional
Property, and CP §3.5.2's silence is simply the minimal (mandatory-only)
path. The code's behaviour already matches this finding without needing a
change: the write is attempted and a device-side refusal is recorded as an
expected outcome, not a procedure failure (`VersionOutcome::Refused`,
`crates/knx-net/src/commissioning/download.rs`) — see task C1, confirmed by
task C18's audit of `06 Profiles v02.01.01.pdf`
([spec-audits/2026-09-19-cp-3_5_3-partial-download.md](spec-audits/2026-09-19-cp-3_5_3-partial-download.md)).

## 7. KNX Standard errata — printed text this project deliberately does not follow

A specification is not automatically self-consistent just because it is
official. Reading the commissioning procedures of `03_05_03 Configuration
Procedures` ("CP") and `03_05_01 Resources` ("RES") against the code in
`crates/knx-core/src/commissioning` and `crates/knx-net/src/commissioning`
turned up six places where the printed text is internally inconsistent, or
says something the code deliberately does not do — a seventh candidate
(CP §3.5.2/§3.5.3's apparent disagreement on `PID_PROGRAM_VERSION`) turned
out to be Standard-permitted rather than errata and lives in §6 above
instead. None of the six below is a defect report against the KNX
Association — a published standard this size having a handful of
transcription slips is unremarkable. What would be a defect is leaving them
unwritten: an undocumented divergence from a published specification reads
exactly like a bug to the next person who opens the PDF beside this code and
finds a difference. This list is that person's answer.

All page numbers below are PDF page numbers of the cited document, offset
zero — verified per document by its own footer, not assumed.

1. **The three table-variant load procedures read AP2's Memory Control Block,
   not their own.** CP §3.5.3's Group Object Table variant (pp. 51-52), Group
   Address Table variant (pp. 54-55) and Association Table variant (p. 56) all
   print `PropertyRead(ID_ApplicationProgram_2, PID_MCB)` in a row that
   otherwise addresses the part being loaded — every surrounding row in the
   same procedure reads or writes *that part's own* object, not Application
   Program 2's. The code reads each loaded part's own MCB instead
   (`load_one_part`, `crates/knx-net/src/commissioning/download.rs:788` and
   `:817`) — the sensible reading of "read back what you just loaded", and,
   absent this note, an undocumented divergence from the printed text.
2. **AP2 Nr. 11 files a Group Object Table step under the Address Table's own
   row.** "Loading the Address Table" (CP §3.5.3 AP2 Nr. 11, p. 47) reads *"Get
   Group Object Table base pointer"* and *"Set Group Object Table to the
   LoadState 'Loaded'"* — both plainly about the Group Object Table — inside
   the row that loads the Address Table. The same misplacement repeats at AP1
   Nr. 10 (p. 50) and GOT Nr. 09 (p. 52). The code treats each table as its
   own segment and does not carry this cross-wiring into the load order.
3. **Group Address Table Nr. 05 lists `LoadCompleted` as an event inside an
   unload wait.** CP §3.5.3, p. 53: the row waits for the *previous* Group
   Address Table to leave its old state and names `LoadCompleted` among the
   events that wait tolerates. But `LoadCompleted` received while a Load State
   Machine is `Unloaded` is, per RES Table 94, p. 296, `R: Unloaded /
   O: Error` — not a step of unloading anything. The code's unload wait does
   not treat an incoming `LoadCompleted` as expected input.
4. **CP §3.5.4 step 05's cross-reference points at nothing.** CP §3.5.4, p. 57,
   step 05 reads *"refer to the routines of 'Unload Device' in 3.5.1.3"* — but
   CP §3.5.1.3, p. 40, is titled *"Memory architecture"*, and the
   capitalized string `Unload Device` occurs exactly once in the entire
   document, case-sensitive: inside this same dangling reference. A
   case-insensitive search for `unload device` instead matches exactly two
   places, neither of them the reference itself: CP §3.5.2 Nr. 05, p. 42,
   whose row label is the unrelated, lowercase `Unload device` — that
   procedure's own unload step — and CP §3.5.4 Nr. 05, p. 57, whose own row
   label is likewise the lowercase `Unload device`, on the very page that
   carries the dangling reference, naming step 05 itself rather than
   referencing anything. The quoted capitalized reference does not turn up
   in a naive text extraction at all: the source PDF's two-column table
   layout wraps the cell across a line break as `'Unload` / *(other
   columns' text)* / `Device' in 3.5.1.3)`, so `Unload Device` never appears
   as a contiguous string to grep for — confirmed against
   `03_05_03 Configuration Procedures v02.01.01 AS.pdf` with
   `pdftotext -layout`. Recorded here so a future check does not rediscover
   either lowercase occurrence, or the invisible-to-grep reference itself,
   and "correct" this entry.
5. **RES §4.23.2.4.1, p. 297, names a Load Control value as if it were a Load
   State.** It reads *"continue with further access only after load state has
   changed to LoadCompleted"*, but Table 92 (the Load State Machine's state
   list) names the terminal state `Loaded`; `LoadCompleting` is the
   intermediate state on the way there, and `LoadCompleted` (`02h`) is a Load
   Control *value* a client writes to request the transition, never a state a
   device reports back. The code polls for `Loaded`, matching Table 92, not
   the clause's own wording.
6. **CP §3.5.2, §3.5.3 and §3.5.4 each unload and load the five parts in a
   different order, and cannot all three be matched by one implementation.**
   CP §3.5.2 step 05, p. 42, unloads Application Program 2, Application
   Program 1, Group Object Table, Association Table, Address Table. CP §3.5.4
   step 05, p. 57, unloads Address Table, Association Table, Object Table,
   Application Program 2, Application Program 1. CP §3.5.2's own *load* order,
   steps 06-10, pp. 43-44, is Application Program 2, Application Program 1,
   Group Object Table, Address Table, Association Table — a third ordering,
   distinct from both unload orders above. Freeing (and loading) memory is
   order-independent
   as far as the Load State Machine cares, so this is a trace-fidelity
   observation, not a defect, and is explicitly not something the code
   changes iteration order to "fix": the code iterates `plan.parts` in one
   fixed order for all three procedures.

Nothing above authorises implementing or claiming behaviour the KNX Standard
does not specify — see §1's wording policy. It records six places where
following the printed text *literally* would either duplicate a transcription
error or contradict another clause of the same Standard, and where the actual
behaviour was chosen instead.
[KNOWN_LIMITATIONS.md §95](KNOWN_LIMITATIONS.md#95-six-places-where-the-knx-standards-printed-text-must-not-be-followed-literally)
points here.
