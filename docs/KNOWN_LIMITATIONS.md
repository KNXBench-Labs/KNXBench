# Known limitations

This is the short current-reading index. Each detail file preserves the
original technical record and dated history. A historical heading is not
a current defect or a new work assignment.

**Status of record:** [LEDGER](status/LEDGER.md). Status below is generated
from that ledger, not edited here. `DONE` closes the recorded work package,
not every compatibility or evidence boundary. Entries without a ledger row
are explicitly untracked; no formal disposition is invented.

For practical user-facing limits see [Known issues](manual/known-issues.md);
for next work see [OPEN_WORK](OPEN_WORK.md).

**Maintenance:** edit the summaries in [catalogue.json](limitations/catalogue.json),
then run `python3 tools/limitations_catalogue.py --write`. The documentation
gate rejects drift. Keep existing IDs and anchors; never reuse a number.

## Topics

- [Import and export](#import-and-export)
- [Manufacturer data and parameters](#manufacturer-data-and-parameters)
- [Project model and storage](#project-model-and-storage)
- [Commissioning and recovery](#commissioning-and-recovery)
- [Bus diagnostics and DPT](#bus-diagnostics-and-dpt)
- [UI and accessibility](#ui-and-accessibility)
- [Platform, deployment and security](#platform-deployment-and-security)
- [Integrations and community](#integrations-and-community)
- [Historical catalogue notes](#historical-catalogue-notes)

## Import and export

<a id="catalogue-kl-001"></a>
<a id="1-single-sample-bias"></a>
### 1. Independent sample coverage

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-1](status/LEDGER.md#snapshot-ids)).
Independent schema-23 evidence narrows sample preservation/module coverage. Other schemas and broader manufacturer/program shapes still lack equivalent independent evidence.
[Details and history](limitations/KL-001-independent-sample-coverage.md)

<a id="catalogue-kl-002"></a>
<a id="2-no-authoritative-xsd-is-publicly-available"></a>
### 2. Structural validation without authoritative XSD

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-2](status/LEDGER.md#snapshot-ids)).
Imports use bounded structural rules, not authoritative XSD conformance. Unknown constructs are not automatically malformed, understood or editable.
[Details and history](limitations/KL-002-structural-validation-without-authoritative-xsd.md)

<a id="catalogue-kl-013"></a>
<a id="13-password-protected-projects-zipcrypto-ets4ets5-is-decrypted-aes-ets6-is-still-refused"></a>
### 13. Protected project import

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-13](status/LEDGER.md#snapshot-ids)).
ZipCrypto (ETS4/ETS5) decrypts: synthetic fixtures always, the public protected ETS4 and ETS 5.7 samples in an opt-in test since 2026-10-12. AES (ETS6) remains refused; knx diff and product ingest take no password.
[Details and history](limitations/KL-013-protected-project-import.md)

<a id="catalogue-kl-014"></a>
<a id="14-the-projects-default-language-is-a-placeholder"></a>
### 14. Imported project default language

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-14](status/LEDGER.md#snapshot-ids)).
Imported projects use an English default, not an inferred authoring language. Product/UI translation choice does not establish project-wide source language.
[Details and history](limitations/KL-014-imported-project-default-language.md)

<a id="catalogue-kl-015"></a>
<a id="15-unparsable-values-survive-only-on-override-fields"></a>
### 15. Malformed values and typed mappings

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-15](status/LEDGER.md#snapshot-ids)).
Override fields explicitly retain malformed values. Bare/optional mappings can fall back while reporting the source; retained bytes are not typed interpretation.
[Details and history](limitations/KL-015-malformed-values-and-typed-mappings.md)

<a id="catalogue-kl-023"></a>
<a id="23-apiprojectdownload-buffers-the-whole-knxdb-file-in-memory"></a>
### 23. Native project download streaming

**Ledger status:** Not separately tracked.
Whole-file response buffering is resolved by streaming. Export still creates a temporary SQLite serialization before responding.
[Details and history](limitations/KL-023-native-project-download-streaming.md)

<a id="catalogue-kl-038"></a>
<a id="38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability"></a>
### 38. CSV interoperability

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-38](status/LEDGER.md#snapshot-ids)).
KNXBench CSV v1 is its own exchange format. Its roundtrip tests do not verify ETS CSV/ESF interoperability.
[Details and history](limitations/KL-038-csv-interoperability.md)

<a id="catalogue-kl-039"></a>
<a id="39-csv-import-never-re-addresses-deletes-or-manages-group-ranges"></a> <a id="39-csv-import-does-not-create-or-rename-group-ranges"></a>
### 39. CSV group-range and address operations

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-39](status/LEDGER.md#snapshot-ids)).
CSV does not edit group ranges; removing rows alone does nothing. Explicit moves/deletes reject occupied targets rather than guess swaps.
[Details and history](limitations/KL-039-csv-group-range-and-address-operations.md)

<a id="catalogue-kl-040"></a>
<a id="40-csv-export-only-columns-are-never-applied-on-import-and-there-are-no-descriptioncomment-columns"></a> <a id="40-csv-derived-columns-are-read-only-and-there-are-no-descriptioncomment-columns"></a>
### 40. CSV read-only and absent columns

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-40](status/LEDGER.md#snapshot-ids)).
Derived DPT/group columns are read-only; changed values are refused. Description/Comment interchange columns are absent.
[Details and history](limitations/KL-040-csv-read-only-and-absent-columns.md)

<a id="catalogue-kl-041"></a>
<a id="41-a-csv-file-saved-from-excel-under-a-german-locale-may-still-surprise-a-user"></a> <a id="41-german-locale-separators-are-supported-unverified-spreadsheet-transformations-remain"></a>
### 41. Spreadsheet transformations

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-41](status/LEDGER.md#snapshot-ids)).
Comma/semicolon separators are supported. Spreadsheet encoding changes or transformed address/boolean cells are not universally accommodated.
[Details and history](limitations/KL-041-spreadsheet-transformations.md)

<a id="catalogue-kl-044"></a>
<a id="44-project-documentation-export-t13-has-no-ets-report-parity-and-none-can-currently-be-measured"></a>
### 44. Project-report parity

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-44](status/LEDGER.md#snapshot-ids)).
HTML documentation is KNXBench's own report. ETS layout/content parity lacks independent reference verification.
[Details and history](limitations/KL-044-project-report-parity.md)

<a id="catalogue-kl-045"></a>
<a id="45-project-documentation-export-has-no-native-pdf-output"></a>
### 45. PDF output

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-45](status/LEDGER.md#snapshot-ids)).
Reports are HTML with print styling and browser print-to-PDF. No native PDF renderer/direct native PDF export is supplied.
[Details and history](limitations/KL-045-pdf-output.md)

<a id="catalogue-kl-046"></a>
<a id="46-project-documentation-export-does-not-resolve-manufacturer-product-or-program-names"></a> <a id="46-project-documentation-export-resolves-names-only-with-installed-product-data--partially-resolved-2026-09-23-t14"></a>
### 46. Report name resolution

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-46](status/LEDGER.md#snapshot-ids)).
Display names require installed matching product data. Unresolved/mismatched references remain raw and reported.
[Details and history](limitations/KL-046-report-name-resolution.md)

<a id="catalogue-kl-047"></a>
<a id="47-project-documentation-export-does-not-list-parameter-values-or-module-instance-arguments"></a> <a id="47-project-documentation-export-lists-parameter-values-and-module-instance-arguments--partially-resolved-2026-09-23-t14"></a>
### 47. Report parameter semantics

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-47](status/LEDGER.md#snapshot-ids)).
Stored values/module arguments are listed with resolvable display names. Raw values are not complete evaluated manufacturer semantics.
[Details and history](limitations/KL-047-report-parameter-semantics.md)

<a id="catalogue-kl-048"></a>
<a id="48-project-documentation-export-renders-in-one-language-only"></a> <a id="48-project-documentation-export-has-englishgerman-chrome-but-not-a-complete-prose-catalogue--partially-resolved-2026-09-23-t14"></a>
### 48. Report languages

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-48](status/LEDGER.md#snapshot-ids)).
English/German chrome, table labels and product overlays exist (labels since Goal B8). Diagnostic sentences and raw values stay English; no third language.
[Details and history](limitations/KL-048-report-languages.md)

<a id="catalogue-kl-125"></a>
<a id="125-ets-6s-device-local-communication-object-ids-are-read-from-a-single-projects-evidence"></a>
### 125. Independent ETS6 object-id evidence

**Ledger status:** `DONE` ([KL-125](status/LEDGER.md#snapshot-ids)).
A second independent schema-23 project meets the original lifting condition. Resolution is not arbitrary schema/program coverage or writable module semantics.
[Details and history](limitations/KL-125-independent-ets6-object-id-evidence.md)

<a id="catalogue-kl-159"></a>
<a id="159-a-project-archive-may-unpack-to-at-most-512-mib-and-every-member-name-must-be-unique"></a> <a id="159-a-project-archive-may-unpack-to-at-most-1024-mib-and-every-member-name-must-be-unique"></a>
### 159. Project archive budgets and duplicate names

**Ledger status:** Not separately tracked.
Expanded projects are capped at 1024 MiB total/64 MiB per member. Excess inflation and duplicate/case-colliding names are refused.
[Details and history](limitations/KL-159-project-archive-budgets-and-duplicate-names.md)

<a id="catalogue-kl-173"></a>
<a id="173-instance-overrides-outside-the-active-object-tree-retained-and-reported"></a>
### 173. Overrides outside the active object tree

**Ledger status:** `DONE` ([KL-173](status/LEDGER.md#post-snapshot-ids)).
Unused overrides are retained/reported; conflicting duplicates are errors. No active objects or program defaults are invented.
[Details and history](limitations/KL-173-overrides-outside-the-active-object-tree.md)

<a id="catalogue-kl-174"></a>
<a id="174-project-user-files-and-add-in-data-are-filed-as-plain-container-entries"></a>
### 174. Project user files and add-in payloads

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-174](status/LEDGER.md#post-snapshot-ids)).
Payloads have bounded descriptive classification but remain opaque data. No unpacking/installation/execution follows.
[Details and history](limitations/KL-174-project-user-files-and-add-in-payloads.md)

<a id="catalogue-kl-177"></a>
<a id="177-ets-57-projects-schema-20-rest-on-three-public-samples-sub-module-object-ids-are-not-decoded"></a>
### 177. ETS 5.7 schema-20 evidence

**Ledger status:** `DONE` ([KL-177](status/LEDGER.md#post-snapshot-ids)).
Import rests on three public samples. Sub-module object ids and unobserved constructs stay unsupported rather than extrapolated.
[Details and history](limitations/KL-177-ets-57-schema-20-evidence.md)

<a id="catalogue-import-integrity-repairs"></a>
<a id="local-integrity-repairs-and-remaining-boundaries-2026-10-10"></a>
### Import integrity repairs

**Ledger status:** Not separately tracked.
Repairs do not guess opaque attributes into typed values. Later durable-report delivery supersedes the old persistence gap; channel, address and IP semantics remain bounded.
[Details and history](limitations/import-integrity-repairs.md)

<a id="catalogue-independent-private-schema-23-review"></a>
<a id="private-schema-23-sample-review-2026-10-10"></a>
### Independent private schema-23 review

**Ledger status:** Not separately tracked.
One independent project widens import evidence, not universal ETS coverage. Generic findings are documented; original data and raw evidence remain private.
[Details and history](limitations/independent-private-schema-23-review.md)

<a id="catalogue-native-import-evidence-and-recorded-device-state"></a>
<a id="ported-native-import-evidence-2026-10-12"></a>
### Native import evidence and recorded device state

**Ledger status:** Not separately tracked.
Stored load states are project records, not bus measurements. Missing historical reports are not reconstructed; retained BusAccess/IP configuration is shown read-only since Goal B3, IPConfig without a device.
[Details and history](limitations/native-import-evidence-and-recorded-device-state.md)

<a id="catalogue-selective-import"></a>
<a id="selective-import-and-evidence-boundaries-2026-10-10-source-integration"></a>
### Selective import

**Ledger status:** `DONE` ([IMPORT-01](status/LEDGER.md#snapshot-ids)).
Selection merges devices or complete lines into an explicit installation. No building-only selector, automatic readdressing, conflict overwrite or automatic catalogue installation.
[Details and history](limitations/selective-import.md)

<a id="catalogue-kl-186"></a>
### 186. Plug-in data adoption is not undoable; adopted bytes are unverified

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-186](status/LEDGER.md#post-snapshot-ids)).
Adopting a plug-in's stored bytes between two projects of one lineage exists in the application layer, the CLI and (since 17 October 2026) the server and web app. The bytes are not decoded, no remapping is attempted, the opaque store is outside command Undo (the way back is a project version recorded before the change), and no plug-in, ETS or device acceptance of the adopted bytes is verified.
[Details and history](limitations/KL-186-plugin-data-adoption.md)

## Manufacturer data and parameters

<a id="catalogue-kl-003"></a>
<a id="3-device-parameters-are-preserved-but-not-interpreted"></a>
### 3. Parameter evaluation and editing boundaries

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-3](status/LEDGER.md#snapshot-ids)).
Supported top-level and single-authoritative module fields are editable. Calculations, repeated expansion, unsupported controls and encodings remain separately limited. Declared Table/Grid parameter blocks render as a matrix; their visual variants, collapse rules, parameter-driven captions and in-cell buttons are not reproduced.
[Details and history](limitations/KL-003-parameter-evaluation-and-editing-boundaries.md)

<a id="catalogue-kl-006"></a>
<a id="6-devices-behind-vendor-plug-in-dlls"></a>
### 6. Devices dependent on vendor plug-ins

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-6](status/LEDGER.md#snapshot-ids)).
Vendor-dependent devices remain preservation-only where configuration requires a plug-in. Project app/add-in payloads are retained, not executed by the product.
[Details and history](limitations/KL-006-devices-dependent-on-vendor-plug-ins.md)

<a id="catalogue-kl-011"></a>
<a id="11-knxprod-support-is-evidenced-for-schemes-11-12-13-14-20-and-exact-namespace-21"></a>
### 11. Product schema and namespace coverage

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-11](status/LEDGER.md#snapshot-ids)).
Namespaces have bounded parser/persistence evidence, including exact 10/23 follow-ups. Admission does not establish arbitrary namespaces or all vendor semantics.
[Details and history](limitations/KL-011-product-schema-and-namespace-coverage.md)

<a id="catalogue-kl-012"></a>
<a id="12-manufacturer-data-resolution--one-of-three-gaps-closed-2026-09-20"></a>
### 12. Manufacturer resolution gaps

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-12](status/LEDGER.md#snapshot-ids)).
Matching product data enriches supported defaults/parameters, not every construct/application combination. Unresolved identities and unsupported semantics stay explicit.
[Details and history](limitations/KL-012-manufacturer-resolution-gaps.md)

<a id="catalogue-kl-064"></a>
<a id="64-languages-blocks-outside-an-application-program-are-discarded-on-import"></a>
### 64. Non-program translation semantics

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-64](status/LEDGER.md#snapshot-ids)).
Program/catalogue/hardware/master translations are ingested. Remaining semantics/consumer/language gaps are not blanket loss of Languages blocks.
[Details and history](limitations/KL-064-non-program-translation-semantics.md)

<a id="catalogue-kl-068"></a>
<a id="68-repeated-module-instantiation-is-refused-not-supported"></a> <a id="68-repeat-activation-stays-unsupported-own-instance-values-are-readable"></a>
### 68. Repeated-module activation

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-68](status/LEDGER.md#snapshot-ids)).
Own instance values are readable without sibling/default substitution. Repeat-driven activation and general repeated-module editing remain unsupported.
[Details and history](limitations/KL-068-repeated-module-activation.md)

<a id="catalogue-kl-069"></a>
<a id="69-a-module-with-no-id-cannot-be-matched-to-a-project-instance"></a>
### 69. Modules without identifiers

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-69](status/LEDGER.md#snapshot-ids)).
A Module without Id cannot match stored instance authority. It stays read-only with a diagnostic rather than an invented identity.
[Details and history](limitations/KL-069-modules-without-identifiers.md)

<a id="catalogue-kl-070"></a>
<a id="70-writing-a-declared-but-not-currently-shown-parameter-is-now-refused"></a>
### 70. Writing inactive parameters

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-70](status/LEDGER.md#snapshot-ids)).
Only currently shown/editable panel fields admit writes. Declared but inactive ids are refused.
[Details and history](limitations/KL-070-writing-inactive-parameters.md)

<a id="catalogue-kl-071"></a>
<a id="71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with"></a>
### 71. Pre-schema-6 module identities

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-71](status/LEDGER.md#snapshot-ids)).
Migration cannot reconstruct missing original ModuleInstance ids. Re-import is needed to recover authority for scoped editing.
[Details and history](limitations/KL-071-pre-schema-6-module-identities.md)

<a id="catalogue-kl-085"></a>
<a id="85-a-signature-package-member-is-stored-with-role-signature-never-verified"></a>
### 85. Unverified package signatures

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-85](status/LEDGER.md#snapshot-ids)).
Signature members are retained/classified, not verified. Storage does not establish authenticity or trusted origin.
[Details and history](limitations/KL-085-unverified-package-signatures.md)

<a id="catalogue-kl-086"></a>
<a id="86-duplicate-identifiers-inside-one-file--recorded-for-normalized-product-identifiers-dpt-provenance-remains-limited"></a>
### 86. Duplicate product identifiers

**Ledger status:** `DONE` ([KL-86](status/LEDGER.md#snapshot-ids)).
Normalized duplicates are reported. DPT provenance and unsupported identifier families retain separate boundaries.
[Details and history](limitations/KL-086-duplicate-product-identifiers.md)

<a id="catalogue-kl-087"></a>
<a id="87-a-parse-fix-does-not-reach-rows-that-were-already-ingested-and-only-a-migration-can-go-back-for-them"></a>
### 87. Re-derivation after parser fixes

**Ledger status:** `DONE` ([KL-87](status/LEDGER.md#snapshot-ids)).
Previously derived rows need explicit migration/backfill or fresh ingest. Parser changes do not automatically repair all installed databases.
[Details and history](limitations/KL-087-re-derivation-after-parser-fixes.md)

<a id="catalogue-kl-088"></a>
<a id="88-a-manufacturers-display-name-is-last-writer-wins-and-that-is-on-purpose"></a>
### 88. Manufacturer display-name winner

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-88](status/LEDGER.md#snapshot-ids)).
Display names deliberately use last-writer-wins. Ingest order can change names independently of first-winner product definitions.
[Details and history](limitations/KL-088-manufacturer-display-name-winner.md)

<a id="catalogue-kl-128"></a>
<a id="128-legacy-vd3vd5-and-pr3pr5-files-are-refused-and-the-refusal-misnames-the-format"></a>
### 128. Legacy product import boundaries

**Ledger status:** `DONE` ([KL-128](status/LEDGER.md#snapshot-ids)).
VD3–VD5 import is delivered. PR/VD2, legacy DPTs, unsupported parameter forms and unverified hardware downloads remain bounded.
[Details and history](limitations/KL-128-legacy-product-import-boundaries.md)

<a id="catalogue-kl-134"></a>
<a id="134-baggage-is-inventoried-not-interpreted"></a>
### 134. Baggage classification versus interpretation

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-134](status/LEDGER.md#snapshot-ids)).
Baggage is inventoried/classified, not installed or executed. Icons/destinations/timestamps remain raw or uninterpreted where documented. Non-package members of a vendor ZIP are listed with size and digest, not stored.
[Details and history](limitations/KL-134-baggage-classification-versus-interpretation.md)

<a id="catalogue-kl-135"></a>
<a id="135-package-identity-is-recorded-not-decided"></a> <a id="136-mask-0701h-bim-m112-devices-cannot-receive-an-application-download"></a> <a id="136-mask-0701h-bim-m112-devices-can-receive-an-application-download--lifted-the-restart-stays-unconfirmed"></a>
### 135. Product and application identity policy

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-135](status/LEDGER.md#snapshot-ids)).
Winner/candidate provenance is reported; first-installed definitions still win. No version selector, pinning or compatibility/replacement policy exists.
[Details and history](limitations/KL-135-product-and-application-identity-policy.md)

<a id="catalogue-kl-146"></a>
<a id="146-a-channel-without-text-has-no-name-of-its-own-and-some-activations-are-undetermined"></a> <a id="146-channel-labels-are-shown-undetermined-activation-and-missing-dpt-remain"></a>
### 146. Object activation and DPT gaps

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-146](status/LEDGER.md#snapshot-ids)).
Labels/bounded evaluation exist. Undetermined activation, unsupported scoped sets and missing DPTs are explicit, not inferred.
[Details and history](limitations/KL-146-object-activation-and-dpt-gaps.md)

<a id="catalogue-kl-149"></a>
<a id="149-knx-products-ingest-matches-the-knxprod-extension-case-sensitively"></a>
### 149. Resolved product filename case handling

**Ledger status:** `DONE` ([KL-149](status/LEDGER.md#post-snapshot-ids)).
ASCII-case-independent extensions resolve the uppercase refusal. Namespace/archive/legacy limits remain.
[Details and history](limitations/KL-149-resolved-product-filename-case-handling.md)

<a id="catalogue-kl-150"></a>
<a id="150-a-product-with-nested-moduledefs-crashes-the-install-with-a-database-constraint-error"></a>
### 150. Resolved nested-module storage

**Ledger status:** `DONE` ([KL-150](status/LEDGER.md#post-snapshot-ids)).
The database-constraint defect is fixed. Successful install does not prove nested Dynamic/editing/download semantics.
[Details and history](limitations/KL-150-resolved-nested-module-storage.md)

<a id="catalogue-kl-151"></a>
<a id="151-real-manufacturer-packages-exceed-the-product-zip-size-limits"></a>
### 151. Product-package size profiles

**Ledger status:** `DONE` ([KL-151](status/LEDGER.md#post-snapshot-ids)).
Explicit bounded profiles admit larger measured bundles; production/web limits remain. Vendor ZIPs share one budget of the chosen profile; the large profile stays CLI-only. Corpus inclusion is not unlimited admission/hardware verification.
[Details and history](limitations/KL-151-product-package-size-profiles.md)

<a id="catalogue-kl-152"></a>
<a id="152-the-xml-evidence-item-limit-refuses-two-real-packages"></a>
### 152. XML evidence budgets

**Ledger status:** `DONE` ([KL-152](status/LEDGER.md#post-snapshot-ids)).
Measured refusals have a bounded delivered remedy, not unlimited XML processing. Coupled byte/item/depth limits and atomic refusal remain.
[Details and history](limitations/KL-152-xml-evidence-budgets.md)

<a id="catalogue-kl-153"></a>
<a id="153-master-data-schemes-10-and-23-are-refused-for-standalone-knxprod"></a> <a id="153-product-scheme10-is-refused-exact23-support-is-bounded"></a>
### 153. Exact product namespaces 10 and 23

**Ledger status:** `DONE` ([KL-153](status/LEDGER.md#post-snapshot-ids)).
Exact 10/23 parsing/persistence is admitted. Historical refusals do not describe current unknown-vendor/other-namespace support.
[Details and history](limitations/KL-153-exact-product-namespaces-10-and-23.md)

<a id="catalogue-kl-156"></a>
<a id="156-unstored-parameterparameterref-attributes-are-not-reported"></a>
### 156. Parameter-attribute reporting

**Ledger status:** `DONE` ([KL-156](status/LEDGER.md#post-snapshot-ids)).
Previously unreported attributes are now retained/reported at scoped ingestion. Reporting grants no typed semantics/editing/image materialization.
[Details and history](limitations/KL-156-parameter-attribute-reporting.md)

<a id="catalogue-ap3-bounded-parameter-controls"></a>
<a id="bounded-ap3-parameter-controls-2026-10-11-pdb-04-remains-partial"></a>
### AP3 bounded parameter controls

**Ledger status:** `LATER` ([PDB-04](status/LEDGER.md#snapshot-ids)).
Time retains counts; sliders need bounded integer evidence. Colour/Picture stay unsupported-codec originals; modules need separate mapping contracts.
[Details and history](limitations/ap3-bounded-parameter-controls.md)

<a id="catalogue-catalogue-batch-creation"></a>
<a id="u11-catalog-batch-scope-issue-07"></a>
### Catalogue batch creation

**Ledger status:** Not separately tracked.
Creation is bounded to 1–32 devices and one undo step. Unique names/address allocation are explicit options, not default deduplication or physical commissioning.
[Details and history](limitations/catalogue-batch-creation.md)

<a id="catalogue-catalogue-metadata-semantics"></a>
<a id="pdb-7-catalogue-metadata-are-source-strings-not-capabilities"></a>
### Catalogue metadata semantics

**Ledger status:** `DONE` ([PDB-07](status/LEDGER.md#snapshot-ids)).
Secure/version/capacity attributes remain manufacturer source strings, not parsed capabilities or compatibility policy.
[Details and history](limitations/catalogue-metadata-semantics.md)

<a id="catalogue-catalogue-updates-from-supplied-files"></a>
<a id="catalogue-updates-come-only-from-files-you-supply-2026-10-12"></a>
### Catalogue updates from supplied files

**Ledger status:** `LATER` ([FUTURE-05](status/LEDGER.md#snapshot-ids)).
Updates use user-supplied files, not the KNX Online Catalog. Full database backups and plain install are separate from the reviewed update/restore workflow.
[Details and history](limitations/catalogue-updates-from-supplied-files.md)

<a id="catalogue-checked-parameter-editors"></a>
<a id="checked-parameter-editors-in-the-device-editor-2026-10-12"></a>
### Checked parameter editors

**Ledger status:** `LATER` ([PDB-04](status/LEDGER.md#snapshot-ids)).
Checked controls are opt-in and bounded; modules fall back, Colour/Picture remain unconverted originals. Writes are revision-bound end to end since Goal B1a.
[Details and history](limitations/checked-parameter-editors.md)

<a id="catalogue-manufacturer-report-history"></a>
<a id="pdb-3-report-history-and-coverage-boundary"></a>
### Manufacturer report history

**Ledger status:** `ACCEPTED_BOUNDARY` ([PDB-03](status/LEDGER.md#snapshot-ids)).
Measured facts require encounter-ledger evidence; older installs are unavailable, not zero. Reports do not interpret all vendor semantics or verify signatures.
[Details and history](limitations/manufacturer-report-history.md)

<a id="catalogue-parameter-and-dynamic-coverage"></a>
<a id="pdb-9-parameter-and-dynamic-coverage-boundary"></a>
### Parameter and Dynamic coverage

**Ledger status:** `ACCEPTED_BOUNDARY` ([PDB-01](status/LEDGER.md#snapshot-ids)).
Unsupported layouts, calculations, activation and encodings remain named limits. Grid declarations can render as lists; retained values do not prove editable semantics.
[Details and history](limitations/parameter-and-dynamic-coverage.md)

## Project model and storage

<a id="catalogue-kl-009"></a>
<a id="9-project-files-are-not-diffable"></a>
### 9. Binary project files and version control

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-9](status/LEDGER.md#snapshot-ids)).
SQLite projects do not yield meaningful textual VCS diffs. Application comparison exists; no independent textual project interchange format is supplied.
[Details and history](limitations/KL-009-binary-project-files-and-version-control.md)

<a id="catalogue-kl-018"></a>
<a id="18-open_project-does-not-clear-the-previous-knxdb-store_path"></a>
### 18. Resolved imported-project store path

**Ledger status:** Not separately tracked.
Project replacement clears the prior native save target. The old stale-path overwrite scenario is resolved and retained historically.
[Details and history](limitations/KL-018-resolved-imported-project-store-path.md)

<a id="catalogue-kl-042"></a>
<a id="42-command_syncrss-module-doc-overstates-its-own-role--pre-existing-not-introduced-by-t12"></a>
### 42. Resolved save-helper claims

**Ledger status:** `DONE` ([KL-42](status/LEDGER.md#snapshot-ids)).
The compatibility helper uses transactional whole-project saving, not a live row-incremental engine. Earlier no-op/persistence overclaims are resolved.
[Details and history](limitations/KL-042-resolved-save-helper-claims.md)

<a id="catalogue-kl-051"></a>
<a id="51-project-diff-t14-has-no-ets-comparison-parity-and-none-can-currently-be-measured"></a>
### 51. Project-comparison parity

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-51](status/LEDGER.md#snapshot-ids)).
KNXBench defines independent comparison rules. Equivalent ETS matching/content behavior is unverified.
[Details and history](limitations/KL-051-project-comparison-parity.md)

<a id="catalogue-kl-052"></a>
<a id="52-project-diff-cannot-correlate-a-device-with-no-individual-address-and-no-matching-ets_id"></a>
### 52. Device matching without stable keys

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-52](status/LEDGER.md#snapshot-ids)).
Devices lacking address/matching ETS id cannot be confidently correlated. Serial/Puid stability is not guessed into a replacement key.
[Details and history](limitations/KL-052-device-matching-without-stable-keys.md)

<a id="catalogue-kl-053"></a>
<a id="53-project-diff-can-collide-two-same-named-sibling-building-parts"></a>
### 53. Duplicate building-part names in comparison

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-53](status/LEDGER.md#snapshot-ids)).
Same-named sibling paths produce ambiguity and adds/removes. The comparer does not arbitrarily match them.
[Details and history](limitations/KL-053-duplicate-building-part-names-in-comparison.md)

<a id="catalogue-kl-054"></a>
<a id="54-project-diff-does-not-detect-an-ets-re-imports-regenerated-refids-as-the-same-project"></a>
### 54. Regenerated ETS identifiers

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-54](status/LEDGER.md#snapshot-ids)).
Regenerated identifiers can appear as adds/removes when natural keys differ. No special re-import identity inference is supplied.
[Details and history](limitations/KL-054-regenerated-ets-identifiers.md)

<a id="catalogue-kl-055"></a>
<a id="55-project-diff-cannot-merge-or-apply-a-diff-back-onto-a-project"></a>
### 55. Applying or merging project diffs

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-55](status/LEDGER.md#snapshot-ids)).
Comparison is read-only, with no atomic diff-to-command merge engine. Conflict/identity/unsupported-field semantics remain prerequisites.
[Details and history](limitations/KL-055-applying-or-merging-project-diffs.md)

<a id="catalogue-kl-056"></a>
<a id="56-project-diff-does-not-do-a-three-way-comparison"></a>
### 56. Three-way comparison

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-56](status/LEDGER.md#snapshot-ids)).
Only two projects are compared. No common-ancestor-aware comparison or merge workflow exists.
[Details and history](limitations/KL-056-three-way-comparison.md)

<a id="catalogue-kl-127"></a>
<a id="127-a-site-over-several-buildings-rests-on-schema-text-and-synthetic-tests-not-on-an-ets-sample"></a>
### 127. Site and building evidence

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-127](status/LEDGER.md#snapshot-ids)).
Ground roots group buildings on one installation based on schema/synthetic evidence. Independent ETS examples and multi-installation Sites remain separate gaps.
[Details and history](limitations/KL-127-site-and-building-evidence.md)

<a id="catalogue-kl-129"></a>
<a id="129-a-stale-id-allocator-snapshot-can-duplicate-ids-and-saving-then-drops-one-entity"></a>
### 129. Allocator safeguards and deferred enforcement

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-129](status/LEDGER.md#snapshot-ids)).
Collision/rewind/exhaustion data-loss paths are closed. Structural enforcement phases 3–5 remain explicitly deferred.
[Details and history](limitations/KL-129-allocator-safeguards-and-deferred-enforcement.md)

<a id="catalogue-kl-157"></a>
<a id="157-opening-an-older-project-upgrades-it-in-place"></a>
### 157. In-place native project migration

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-157](status/LEDGER.md#post-snapshot-ids)).
The first write into an older project upgrades the original in place; opening and every reading command use an in-memory copy. Atomic migration is not backup; older builds refuse newer schemas.
[Details and history](limitations/KL-157-in-place-native-project-migration.md)

<a id="catalogue-kl-166"></a>
<a id="166-the-new-project-wizard-seeds-one-installation-from-a-fixed-preset-vocabulary"></a>
### 166. New-project wizard presets

**Ledger status:** Not separately tracked.
The wizard seeds one installation from bounded fixed presets. It does not model every vocabulary or multi-installation design.
[Details and history](limitations/KL-166-new-project-wizard-presets.md)

<a id="catalogue-kl-167"></a>
<a id="167-the-add-device-wizard-confirms-names-and-addresses-not-the-whole-project"></a>
### 167. Add-device preview freshness

**Ledger status:** Not separately tracked.
Preview confirms names/addresses, not immutable project/catalogue state. Other concurrent changes can still refuse creation.
[Details and history](limitations/KL-167-add-device-preview-freshness.md)

<a id="catalogue-kl-175"></a>
<a id="175-rich-text-attribute-values-are-kept-verbatim-not-rendered"></a> <a id="175-rich-text-header-candidates-are-kept-raw-not-validated-or-rendered"></a>
### 175. Raw rich-text header candidates

**Ledger status:** `DONE` ([KL-175](status/LEDGER.md#post-snapshot-ids)).
Candidates are exposed without rendering/format certification. Bounded instruction editing is separate; no general rich-text catalogue/RTF decoder is claimed.
[Details and history](limitations/KL-175-raw-rich-text-header-candidates.md)

<a id="catalogue-device-editor-address-scope"></a>
<a id="u11-device-editor-scope-issue-09"></a>
### Device editor address scope

**Ledger status:** Not separately tracked.
Device-octet editing is line-bound; new .0 addresses require coupler evidence. Project edits do not program physical addresses or infer missing product identity.
[Details and history](limitations/device-editor-address-scope.md)

<a id="catalogue-group-address-membership-and-download-preparation"></a>
<a id="group-address-members-and-download-preparation-2026-10-12"></a>
### Group-address membership and download preparation

**Ledger status:** Not separately tracked.
Linking follows actual installation ownership. Selection/preparation guide existing routes, not new write permission or hardware verification.
[Details and history](limitations/group-address-membership-and-download-preparation.md)

<a id="catalogue-historical-native-notes-foundation"></a>
<a id="native-notes-foundation-candidate-2026-10-11-ap4-not-delivered-yet"></a>
### Historical native notes foundation

**Ledger status:** `DONE` ([FUTURE-03](status/LEDGER.md#snapshot-ids)).
Local-only/no-editor wording is superseded by service/workbench delivery. Literal text and downgrade limits remain; source notes in a selective import follow their mapping or need an explicit choice (ADR-0138).
[Details and history](limitations/historical-native-notes-foundation.md)

<a id="catalogue-installation-instruction-editing"></a>
<a id="installation-instruction-editor-integration-boundary-2026-10-11"></a>
### Installation instruction editing

**Ledger status:** Not separately tracked.
Editing admits bounded exact-source Schema23 XML and selected text/header forms, and devices made in KNXBench (no invented original, Goal B4). Other schemas, encodings, ambiguous owners and unsupported formats remain explicit refusals.
[Details and history](limitations/installation-instruction-editing.md)

<a id="catalogue-instruction-and-note-drafts-across-save"></a>
<a id="installation-instructions-tab-and-save-proof-drafts-2026-10-12"></a>
### Instruction and note drafts across Save

**Ledger status:** Not separately tracked.
Drafts survive Save when server content is unchanged. Real changes require comparison/discard; reloads ask through the engine's own prompt in the browser and, since Goal B2, the desktop webview (not run natively).
[Details and history](limitations/instruction-and-note-drafts-across-save.md)

<a id="catalogue-notes-editor-integration"></a>
<a id="project-notes-editor-integration-boundary-2026-10-12"></a>
### Notes editor integration

**Ledger status:** Not separately tracked.
The notes leaf is now hosted; its unmounted status is historical. Unknown transport outcomes require reload, and working history is not implicit Save.
[Details and history](limitations/notes-editor-integration.md)

<a id="catalogue-offline-naming-recipes"></a>
<a id="offline-naming-recipe-boundary-2026-10-12"></a>
### Offline naming recipes

**Ledger status:** `LATER` ([FUTURE-02](status/LEDGER.md#snapshot-ids)).
Recipes rename explicitly ordered existing addresses only. Conflicts require re-preview; durable working history is not root Save, templates or bus operation.
[Details and history](limitations/offline-naming-recipes.md)

<a id="catalogue-persistent-native-working-history"></a>
<a id="persistent-native-history-is-bounded-not-an-independent-backup-adr-0100"></a>
### Persistent native working history

**Ledger status:** `DONE` ([HISTORY-01](status/LEDGER.md#snapshot-ids)); `DONE` ([HISTORY-02](status/LEDGER.md#snapshot-ids)).
Edits may be journaled before Save; closing without Save does not erase the journal. Same-database versions are not independent backups or multi-user history.
[Details and history](limitations/persistent-native-working-history.md)

<a id="catalogue-project-local-name-editing"></a>
<a id="project-local-name-editing-is-not-bulk-authoring-adr-0101"></a>
### Project-local name editing

**Ledger status:** Not separately tracked.
Individual names are guarded edits, not bulk templates or inferred names. Imported exceptions stay preserved; offline projections cannot authorize writes.
[Details and history](limitations/project-local-name-editing.md)

<a id="catalogue-project-notes-workbench"></a>
<a id="project-notes-workbench-host-2026-10-12"></a>
### Project notes workbench

**Ledger status:** `DONE` ([FUTURE-03](status/LEDGER.md#snapshot-ids)).
Notes use real project targets; ambiguous identities are withheld. Web deletes ask what happens to affected notes (ADR-0138); CLI, CSV-import and scan deletes refuse with the named notes. Browser prompts, desktop reloads and translated review text retain disclosed platform/language boundaries.
[Details and history](limitations/project-notes-workbench.md)

<a id="catalogue-structure-editor-scope"></a>
<a id="u12-structure-editor-scope-issue-05"></a>
### Structure editor scope

**Ledger status:** Not separately tracked.
Commands respect installation ownership and refuse ambiguous/cross-installation moves. This is not general multi-installation transfer or topology-compatibility validation.
[Details and history](limitations/structure-editor-scope.md)

<a id="catalogue-kl-183"></a>
<a id="183-a-knxdb-saved-before-store-schema-v16-does-not-carry-its-application-programs"></a>
### 183. Project files without their manufacturer files

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-183](status/LEDGER.md#post-snapshot-ids)).
A .knxdb saved before store schema v16 does not carry its application programs; next to an empty product database its devices show no parameters until restored from the original .knxproj. Fixed for new saves.
[Details and history](limitations/KL-183-knxdb-manufacturer-files.md)

## Commissioning and recovery

<a id="catalogue-kl-007"></a>
<a id="7-commissioning-and-device-download-are-required-but-blocked"></a> <a id="7-commissioning-a-verified-070nh-memory-path-not-general-device-support"></a>
### 7. Bounded commissioning support

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-7](status/LEDGER.md#snapshot-ids)).
Memory downloads have scoped evidence, not general device support. Public K6/serial/K13 address writes fail before a tunnel without specific durable recovery. Project/device comparison is a session observation; unknown fields remain explicit beside raw ranges, and relevant edits invalidate tags.
[Details and history](limitations/KL-007-bounded-commissioning-support.md)

<a id="catalogue-kl-092"></a>
<a id="92-commissioning-phase-2-is-verified-against-a-simulator-this-project-wrote-one-device-has-been-written-since"></a>
### 92. Simulator versus hardware evidence

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-92](status/LEDGER.md#snapshot-ids)).
Simulator coverage exceeds recorded device runs. Limited live experiments do not validate every mask/recovery case/vendor program.
[Details and history](limitations/KL-092-simulator-versus-hardware-evidence.md)

<a id="catalogue-kl-093"></a>
<a id="93-knx-cores-declarative-procedure-model-still-writes-pid_program_version-unconditionally-for-every-part--parked-deferred-to-task-c11"></a> <a id="93-knx-cores-declarative-procedure-model-still-writes-pid_program_version-unconditionally-for-every-part--parked-unowned"></a>
### 93. Declarative procedure version writes

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-93](status/LEDGER.md#snapshot-ids)).
The illustrative core model still lists unconditional version writes. It is not the narrower executed sequencer or a universal write script.
[Details and history](limitations/KL-093-declarative-procedure-version-writes.md)

<a id="catalogue-kl-095"></a>
<a id="95-six-places-where-the-knx-standards-printed-text-must-not-be-followed-literally"></a>
### 95. Documented specification contradictions

**Ledger status:** Not separately tracked.
Explicit rulings handle named standard contradictions. They grant no permission to infer unrelated undocumented behavior.
[Details and history](limitations/KL-095-documented-specification-contradictions.md)

<a id="catalogue-kl-099"></a>
<a id="99-memorycontrolblocks-access-nibble-order-is-inferred-not-spec-stated"></a>
### 99. Memory-control access-nibble inference

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-99](status/LEDGER.md#snapshot-ids)).
Nibble order is inferred from table position, not explicit text. This inference is not independent wire-level evidence.
[Details and history](limitations/KL-099-memory-control-access-nibble-inference.md)

<a id="catalogue-kl-101"></a>
<a id="101-res-423241s-once-more-attempt-can-roughly-triple-a-load-state-waits-worst-case-latency"></a>
### 101. Load-state retry latency

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-101](status/LEDGER.md#snapshot-ids)).
The prescribed extra poll can include sleep, reconnect and retries. Waiting can exceed a simple transition-time estimate.
[Details and history](limitations/KL-101-load-state-retry-latency.md)

<a id="catalogue-kl-104"></a>
<a id="104-a-device-that-goes-offline-mid-loadcompleting-now-costs-a-full-reconnect-per-quiet-poll"></a>
### 104. Reconnect cost during quiet completion

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-104](status/LEDGER.md#snapshot-ids)).
Released connections can require a full reconnect for each quiet LoadCompleting poll. Tolerated silence can still be costly.
[Details and history](limitations/KL-104-reconnect-cost-during-quiet-completion.md)

<a id="catalogue-kl-105"></a>
<a id="105-transport-layer-control-frames-go-out-at-low-priority-not-system"></a> <a id="105-system-priority-control-frames-sent-on-wire-priority-unmeasured"></a>
### 105. Control-frame on-wire priority

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-105](status/LEDGER.md#snapshot-ids)).
Constructed frames request SYSTEM priority. No independent TP1 trace proves gateway preservation of that priority.
[Details and history](limitations/KL-105-control-frame-on-wire-priority.md)

<a id="catalogue-kl-108"></a>
<a id="108-mp-23-contradicts-itself-about-an-occupied-ia_new-and-this-project-follows-the-exception-text"></a>
### 108. Occupied-address specification ruling

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-108](status/LEDGER.md#snapshot-ids)).
The procedure follows MP's exception interpretation where wording conflicts. Public address-write recovery restrictions remain.
[Details and history](limitations/KL-108-occupied-address-specification-ruling.md)

<a id="catalogue-kl-109"></a>
<a id="109-two-loadable-parts-of-the-same-partkind-have-no-defined-relative-order-so-downloadplannew-refuses-them-both"></a>
### 109. Repeated download part kinds

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-109](status/LEDGER.md#snapshot-ids)).
Two parts of the same PartKind are refused because order is undefined. Arbitrary segment ordering is not invented.
[Details and history](limitations/KL-109-repeated-download-part-kinds.md)

<a id="catalogue-kl-110"></a>
<a id="110-pid_group_responser_table-stays-unimplemented-on-every-medium"></a>
### 110. PL110 group-responser table

**Ledger status:** `LATER` ([KL-110](status/LEDGER.md#snapshot-ids)).
PID_GROUP_RESPONSER_TABLE is unimplemented. PL110-specific obligations do not imply medium support or applicability elsewhere.
[Details and history](limitations/KL-110-pl110-group-responser-table.md)

<a id="catalogue-kl-111"></a>
<a id="111-cp-354-step-07-unload-the-individual-address-stays-unimplemented"></a>
### 111. Individual-address unload

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-111](status/LEDGER.md#snapshot-ids)).
Destructive address unload is unimplemented. Removing a management address requires separately verified recovery.
[Details and history](limitations/KL-111-individual-address-unload.md)

<a id="catalogue-kl-112"></a>
<a id="112-a-download-plan-that-needs-a_key_write-is-refused-outright-not-carried-out"></a>
### 112. Access-key assignment during download

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-112](status/LEDGER.md#snapshot-ids)).
Required A_Key_Write plans are refused. Using an existing key does not support assigning/changing keys.
[Details and history](limitations/KL-112-access-key-assignment-during-download.md)

<a id="catalogue-kl-113"></a>
<a id="113-an-escalation-only-reloads-the-segments-a-shortened-plan-actually-carries"></a>
### 113. Partial-download escalation scope

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-113](status/LEDGER.md#snapshot-ids)).
Escalation reloads only carried segments. It does not reconstruct omitted parts or imply a complete-device reload.
[Details and history](limitations/KL-113-partial-download-escalation-scope.md)

<a id="catalogue-kl-114"></a>
<a id="114-the-download-counter-refusal-is-this-projects-own-conservative-rule-not-a-system-b-obligation"></a>
### 114. Download-counter refusal policy

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-114](status/LEDGER.md#snapshot-ids)).
Changed/unavailable counters trigger conservative refusal. This is KNXBench policy, not a universal System B obligation.
[Details and history](limitations/KL-114-download-counter-refusal-policy.md)

<a id="catalogue-kl-115"></a>
<a id="115-masterresetresponserecovery_wait-and-sessiontimingrestart_basic_t1-compute-durations-nobody-waits-on-yet"></a>
### 115. Recovery and restart waits

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-115](status/LEDGER.md#snapshot-ids)).
Typed callers honor scoped waits. No generic failed-service retry policy or universal hardware recovery follows.
[Details and history](limitations/KL-115-recovery-and-restart-waits.md)

<a id="catalogue-kl-116"></a>
<a id="116-nm_individualaddress_write-does-not-loop-for-the-operator-and-reads-one-transport-layer-release-as-mp-23-never-quite-says"></a>
### 116. Individual-address write procedure

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-116](status/LEDGER.md#snapshot-ids)).
The procedure does not loop indefinitely for programming mode and uses a disclosed release interpretation. Public writes remain recovery-gated.
[Details and history](limitations/KL-116-individual-address-write-procedure.md)

<a id="catalogue-kl-136"></a>
<a id="136-0701h-download-verified-on-three-programs-restart-remains-unconfirmed"></a>
### 136. 0701h download and restart evidence

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-136](status/LEDGER.md#snapshot-ids)).
Evidence covers identified programs/devices. The named MDT's Basic Restart stays unacknowledged, not a verified restart.
[Details and history](limitations/KL-136-0701h-download-and-restart-evidence.md)

<a id="catalogue-kl-138"></a>
<a id="138-a-devices-access-key-comes-from-the-project-or-a-key-file-and-nothing-checks-it-live"></a>
### 138. Download authorization key provenance

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-138](status/LEDGER.md#snapshot-ids)).
Keys come from project attributes or explicit input. Simulator/routes do not prove live validation of a device key.
[Details and history](limitations/KL-138-download-authorization-key-provenance.md)

<a id="catalogue-kl-139"></a>
<a id="139-a-device-can-be-addressed-by-its-serial-number-and-nothing-has-done-it-live"></a> <a id="139-serial-number-address-write-was-ignored-by-the-tested-device"></a>
### 139. Serial-number address-write evidence

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-139](status/LEDGER.md#snapshot-ids)).
The device ignored the tested write; sending is not confirmation. Public serial writes stay recovery-gated.
[Details and history](limitations/KL-139-serial-number-address-write-evidence.md)

<a id="catalogue-kl-140"></a>
<a id="140-the-individual-address-reset-needs-the-pressed-devices-named-and-its-restart-is-unconfirmed"></a>
### 140. Individual-address reset boundary

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-140](status/LEDGER.md#snapshot-ids)).
HTTP/UI reset is excluded by decision. Historical restart uncertainty remains; public writes fail closed without recovery.
[Details and history](limitations/KL-140-individual-address-reset-boundary.md)

<a id="catalogue-kl-141"></a>
<a id="141-master-reset-erases-in-the-simulator-only-hardware-keeps-its-configuration"></a>
### 141. Master Reset evidence

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-141](status/LEDGER.md#snapshot-ids)).
Simulator erase coverage exists; recorded hardware kept its configuration. Successful physical erase/universal recovery is unproved.
[Details and history](limitations/KL-141-master-reset-evidence.md)

<a id="catalogue-kl-142"></a>
<a id="142-the-mask-070nh-partial-download-is-tested-in-the-simulator-only"></a> <a id="142-partial-download-one-verified-device-product-and-ui-boundaries-remain"></a>
### 142. Partial-download evidence

**Ledger status:** `DONE` ([KL-142](status/LEDGER.md#snapshot-ids)).
UI/backend scopes are delivered with single-device evidence. Other products/images, escalation and recovery remain separately bounded.
[Details and history](limitations/KL-142-partial-download-evidence.md)

<a id="catalogue-kl-143"></a>
<a id="143-rf-domain-addresses-exist-in-the-simulator-only-there-is-no-rf-device"></a>
### 143. RF domain-address scope

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-143](status/LEDGER.md#snapshot-ids)).
Domain-address procedures are simulator-tested only. Hardware writes are refused; broader RF/PL110 support is not implied.
[Details and history](limitations/KL-143-rf-domain-address-scope.md)

<a id="catalogue-kl-144"></a>
<a id="144-rf-device-configuration-exists-in-the-simulator-only"></a>
### 144. RF configuration scope

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-144](status/LEDGER.md#snapshot-ids)).
Configuration is simulator-only, with no supported hardware write/CLI/HTTP admission. No KNX-RF validation is claimed.
[Details and history](limitations/KL-144-rf-configuration-scope.md)

<a id="catalogue-kl-145"></a>
<a id="145-instance-level-flag-overrides-are-not-written-into-the-group-object-table"></a> <a id="145-instance-flags-are-written-unlinked-active-objects-can-still-differ"></a>
### 145. Unlinked active communication objects

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-145](status/LEDGER.md#snapshot-ids)).
Instance overrides reach active tables, but unlinked objects can differ from ETS images. Read-back comparison is not a download witness.
[Details and history](limitations/KL-145-unlinked-active-communication-objects.md)

<a id="catalogue-kl-170"></a>
<a id="170-little-endian-parameter-byte-order-is-refused-for-download-images"></a>
### 170. Parameter byte-order download boundary

**Ledger status:** `LATER` ([KL-170](status/LEDGER.md#post-snapshot-ids)).
Unverified LittleEndian materialization remains refused. Defaults/base-image probes and tester reports define no per-type encoding or new write permission.
[Details and history](limitations/KL-170-parameter-byte-order-download-boundary.md)

<a id="catalogue-kl-171"></a>
<a id="171-utf-8-text-parameter-encoding-is-refused-for-download-images"></a>
### 171. UTF-8 text download encoding

**Ledger status:** `LATER` ([KL-171](status/LEDGER.md#post-snapshot-ids)).
Encoding is refused without a sufficient manufacturer/payload contract. Research does not justify guessed bytes or hardware changes.
[Details and history](limitations/KL-171-utf-8-text-download-encoding.md)

<a id="catalogue-kl-172"></a>
<a id="172-a-devices-stored-loaded-image-is-kept-not-decoded"></a>
### 172. Stored loaded-image semantics

**Ledger status:** `LATER` ([KL-172](status/LEDGER.md#post-snapshot-ids)).
LoadedImage/CheckSums are retained, not decoded/verified as reusable download images. Preservation does not establish encoding/compatibility.
[Details and history](limitations/KL-172-stored-loaded-image-semantics.md)

<a id="catalogue-accepted-commissioning-validation-boundary"></a>
<a id="accepted-commissioning-validation-boundary--user-decision-2026-10-04"></a>
### Accepted commissioning validation boundary

**Ledger status:** Not separately tracked.
New hardware, power-loss, vendor and ETS experiments were removed by user decision. This grants neither new write permission nor compatibility/recovery guarantees.
[Details and history](limitations/accepted-commissioning-validation-boundary.md)

<a id="catalogue-backup-directory-synchronization"></a>
<a id="backup-directory-synchronization-is-not-a-disk-loss-or-confinement-proof"></a>
### Backup directory synchronization

**Ledger status:** Not separately tracked.
Synced directory chains are not disk-loss or confinement proof. Paths must stay stable; failed diagnostic files are not recovery receipts.
[Details and history](limitations/backup-directory-synchronization.md)

<a id="catalogue-download-lifecycle-history"></a>
<a id="download-lifecycle-history-admission--safe-03--audit-01-evidence-boundary"></a>
### Download lifecycle history

**Ledger status:** `ACCEPTED_BOUNDARY` ([SAFE-03](status/LEDGER.md#snapshot-ids)); `DONE` ([AUDIT-01](status/LEDGER.md#snapshot-ids)).
Admission/lifecycle records are bounded, not full audit/traffic/recovery history. Durable metadata does not prove whole-device recovery.
[Details and history](limitations/download-lifecycle-history.md)

<a id="catalogue-download-readiness-and-pre-write-backup"></a>
<a id="commissioning-readiness-and-pre-write-backup-boundary-adr-0049"></a>
### Download readiness and pre-write backup

**Ledger status:** `ACCEPTED_BOUNDARY` ([SAFE-02](status/LEDGER.md#snapshot-ids)).
A plannable default image is not hardware verification. Readiness is application/download-type scoped; backups cover only declared pre-write recovery boundaries.
[Details and history](limitations/download-readiness-and-pre-write-backup.md)

<a id="catalogue-offline-ap1-sequence-inspection"></a>
<a id="offline-ap1-sequence-inspection-is-not-new-download-support-2026-10-09"></a>
### Offline AP1 sequence inspection

**Ledger status:** Not separately tracked.
Only explicit, uniquely associated MV-07B0 Load/ap1 declarations are reconstructed. Unsupported steps stay reported; inspection adds no download/execution capability.
[Details and history](limitations/offline-ap1-sequence-inspection.md)

<a id="catalogue-kl-184"></a>
### 184. Diagnostic export reads identity only

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-184](status/LEDGER.md#post-snapshot-ids)).
The read-only diagnostic export reads descriptor, PID12 and PID78 only; no memory profile is admitted, a zero-element reply cannot separate refusal from absence, and no physical device has been read with it yet.
[Details and history](limitations/KL-184-device-diagnostic-export.md)

## Bus diagnostics and DPT

<a id="catalogue-kl-026"></a>
<a id="26-busconnection-does-not-yet-support-knx-ip-secure"></a>
### 26. KNXnet/IP protocol scope

**Ledger status:** Not separately tracked.
Tunnelling/routing/discovery have bounded implementations. Ordinary connectivity does not imply KNX IP Secure or extended discovery support.
[Details and history](limitations/KL-026-knxnetip-protocol-scope.md)

<a id="catalogue-kl-029"></a>
<a id="29-appsknx-cli-bus-monitor-has-formatting-limitations"></a>
### 29. CLI monitor formatting

**Ledger status:** `DONE` ([KL-29](status/LEDGER.md#snapshot-ids)).
Lifted by AR14: monitor and write use the --project style and show every installation's name. Three-level remains without --project and for route-send; joined names are not escaped.
[Details and history](limitations/KL-029-cli-monitor-formatting.md)

<a id="catalogue-kl-031"></a>
<a id="31-knxnetip-routing-has-no-custom-multicast-address-override--resolved-routing-half"></a> <a id="31-routing-multicast-override-exists-discovery-and-real-custom-group-traffic-unverified"></a>
### 31. Custom routing multicast evidence

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-31](status/LEDGER.md#snapshot-ids)).
Custom routing multicast selection is validated. Discovery/real custom-group traffic do not inherit verification from default paths or offline tests.
[Details and history](limitations/KL-031-custom-routing-multicast-evidence.md)

<a id="catalogue-kl-036"></a>
<a id="36-session-log-t11-the-log-tab-was-unreachable-without-an-open-project-and-had-no-growth-cap--resolved-2026-09-10"></a> <a id="36-session-log-export-is-a-bounded-retained-window-not-a-lifetime-audit"></a>
### 36. Session-log export window

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-36](status/LEDGER.md#snapshot-ids)).
Only retained capped entries/loss counts are exported. Evicted entries, replaced-project history and prior runs cannot be reconstructed; raw logs may identify projects.
[Details and history](limitations/KL-036-session-log-export-window.md)

<a id="catalogue-kl-061"></a>
<a id="61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard"></a> <a id="61-the-dpt-codec-covers-thirty-main-types-with-explicit-input-formats-and-disclosed-encoding-rulings"></a>
### 61. DPT codec coverage

**Ledger status:** `DONE` ([KL-61](status/LEDGER.md#snapshot-ids)).
Main types 1–30 have explicit formats/rulings; subtype/FB semantics remain partial. Unsupported structured types and generic parameter-only writes are not silently admitted.
[Details and history](limitations/KL-061-dpt-codec-coverage.md)

<a id="catalogue-kl-062"></a>
<a id="62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-only-its-passive-receive-path-has-real-gateway-evidence"></a>
### 62. Group Monitor evidence

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-62](status/LEDGER.md#snapshot-ids)).
The GUI is tunnelling-based, single-session and client-filtered. Passive receive evidence does not verify every send/device/routing/history scenario.
[Details and history](limitations/KL-062-group-monitor-evidence.md)

<a id="catalogue-kl-072"></a>
<a id="72-line-scan-t17-an-unthrottled-scan-is-a-live-bus-cost-not-a-theoretical-one--shipped-2026-09-13-still-true"></a>
### 72. Line-scan duration and bus load

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-72](status/LEDGER.md#snapshot-ids)).
Sequential probing takes significant time and holds a tunnel. Pacing/exclusions mitigate cost; there is no instantaneous whole-line scan.
[Details and history](limitations/KL-072-line-scan-duration-and-bus-load.md)

<a id="catalogue-kl-073"></a>
<a id="73-a-line-scan-cannot-learn-product-identity-manufacturer-or-serial-number"></a>
### 73. Line-scan identity limits

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-73](status/LEDGER.md#snapshot-ids)).
Scans establish occupancy/mask version, not product, manufacturer or serial identity. Those facts are not inferred.
[Details and history](limitations/KL-073-line-scan-identity-limits.md)

<a id="catalogue-kl-074"></a>
<a id="74-a-line-scan-cannot-distinguish-a-busy-but-present-device-from-an-absent-one"></a>
### 74. Busy devices and vacant scan results

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-74](status/LEDGER.md#snapshot-ids)).
Negative/missing confirms can yield the same vacant verdict. Busy devices are not reliably distinguished from absent addresses.
[Details and history](limitations/KL-074-busy-devices-and-vacant-scan-results.md)

<a id="catalogue-kl-075"></a>
<a id="75-a-shorter---timeout-ms-is-a-real-option-but-not-the-default--a-slow-but-present-device-can-look-vacant"></a>
### 75. Short scan timeouts

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-75](status/LEDGER.md#snapshot-ids)).
Shorter timeouts trade evidence for speed and can mark slow devices vacant. Silence is not proof of absence.
[Details and history](limitations/KL-075-short-scan-timeouts.md)

<a id="catalogue-kl-076"></a>
<a id="76-a-negative-layer-2-confirms-fast-path-was-deliberately-not-built-indeterminate-does-not-retry"></a>
### 76. Scan shortcuts and ambiguous probes

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-76](status/LEDGER.md#snapshot-ids)).
Negative confirms have no fast vacancy path; dropped-frame ambiguity is not automatically retried. These are explicit scan policies.
[Details and history](limitations/KL-076-scan-shortcuts-and-ambiguous-probes.md)

<a id="catalogue-kl-077"></a>
<a id="77-a-line-scan-covers-one-line-at-a-time-it-does-not-cross-couplers"></a>
### 77. Single-line scan scope

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-77](status/LEDGER.md#snapshot-ids)).
One line is scanned sequentially through one gateway. No automatic coupler traversal or multi-line scan.
[Details and history](limitations/KL-077-single-line-scan-scope.md)

<a id="catalogue-kl-078"></a>
<a id="78-a-line-scan-reports-other-knxnetip-tunnelling-endpoints-as-occupied-devices"></a>
### 78. Gateway endpoints in scan results

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-78](status/LEDGER.md#snapshot-ids)).
Only the scan's own tunnel address is excluded. Other endpoints can appear occupied without identifying physical devices.
[Details and history](limitations/KL-078-gateway-endpoints-in-scan-results.md)

<a id="catalogue-kl-079"></a>
<a id="79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry"></a> <a id="79-discovery-needs-reachable-multicast-and-permitted-unicast-replies"></a>
### 79. Discovery reachability

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-79](status/LEDGER.md#snapshot-ids)).
Discovery needs multicast and permitted unicast replies. Manual endpoints/host networking do not eliminate firewall, routing, capacity or bridge-HPAI constraints.
[Details and history](limitations/KL-079-discovery-reachability.md)

<a id="catalogue-kl-082"></a>
<a id="82-the-diagnostics-companions-stale-lock-sees-one-browser-profiles-own-windows-and-nothing-else"></a>
### 82. Monitor-context freshness

**Ledger status:** `DONE` ([KL-82](status/LEDGER.md#snapshot-ids)).
Server-authoritative polling supersedes the browser-only stale lock. It is not cross-server coordination or complete traffic/audit history.
[Details and history](limitations/KL-082-monitor-context-freshness.md)

<a id="catalogue-kl-090"></a>
<a id="90-there-is-no-dpt-main-type-46-46-is-a-count-of-main-types-in-one-ets-master-data-file"></a>
### 90. Historical DPT count correction

**Ledger status:** Not separately tracked.
46 was a catalogue count, not main type 46. This planning correction creates no codec support or new task.
[Details and history](limitations/KL-090-historical-dpt-count-correction.md)

<a id="catalogue-kl-102"></a>
<a id="102-the-write-echos-decode-failure-branch-has-no-known-real-trigger"></a>
### 102. Write-echo error branch

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-102](status/LEDGER.md#snapshot-ids)).
A typed decode-error branch exists without a known honest end-to-end public-API trigger. Type coverage is not a real-trigger witness.
[Details and history](limitations/KL-102-write-echo-error-branch.md)

<a id="catalogue-kl-124"></a>
<a id="124-the-interface-search-shows-four-facts-about-an-interface-the-protocol-carries-more"></a>
### 124. Discovery metadata display

**Ledger status:** `DONE` ([KL-124](status/LEDGER.md#snapshot-ids)).
Decoded optional metadata is shown; missing fields stay unavailable. Loopback/browser evidence is not real multicast/hardware verification.
[Details and history](limitations/KL-124-discovery-metadata-display.md)

<a id="catalogue-kl-126"></a>
<a id="126-line-scan-reconciliation-acts-on-occupancy-evidence-not-device-identity"></a>
### 126. Scan reconciliation is occupancy-based

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-126](status/LEDGER.md#snapshot-ids)).
Unexpected responses create product-less devices. Silence is not absence; deletions require explicit selection.
[Details and history](limitations/KL-126-scan-reconciliation-is-occupancy-based.md)

<a id="catalogue-kl-137"></a>
<a id="137-a-bus-monitor-json-capture-is-a-retained-window-not-a-complete-trace"></a>
### 137. Monitor capture retention

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-137](status/LEDGER.md#snapshot-ids)).
Captures are retained windows with separate server-loss/client-pruning counts. Pause cannot recover traffic; exports are size-bounded.
[Details and history](limitations/KL-137-monitor-capture-retention.md)

<a id="catalogue-kl-154"></a>
<a id="154-the-telegram-flow-view-is-checked-and-measured-in-chromium-only"></a> <a id="154-the-telegram-flow-view-is-checked-in-chromium-only-load-figures-follow-in-u21-part-c"></a> <a id="154-the-telegram-flow-view-is-static-and-checked-in-chromium-only"></a>
### 154. Telegram-flow evidence and capacity

**Ledger status:** Not separately tracked.
Chromium readability/layout evidence has bounded capacity. Membership/inference is not measured delivery/device state; native/accessibility remains separate.
[Details and history](limitations/KL-154-telegram-flow-evidence-and-capacity.md)

<a id="catalogue-kl-180"></a>
<a id="180-gateway-refusals-name-knxbenchs-own-sessions-and-the-gateways-stated-reason-nothing-more"></a>
### 180. Named gateway refusals

**Ledger status:** `DONE` ([KL-180](status/LEDGER.md#post-snapshot-ids)).
Holder details cover this server's sessions only. Other clients are known through gateway refusal; timeout is not proof of busy.
[Details and history](limitations/KL-180-named-gateway-refusals.md)

<a id="catalogue-bus-activity-snapshot"></a>
<a id="partial-commissioning-bus-activity-snapshot-adr-0055"></a>
### Bus-activity snapshot

**Ledger status:** Not separately tracked.
The partial server-lifetime snapshot is not a complete audit or device-mutation proof. Polling can miss actions; empty sessions do not prove gateway idleness.
[Details and history](limitations/bus-activity-snapshot.md)

<a id="catalogue-name-first-dpt-presentation"></a>
<a id="name-first-dpt-presentation-2026-10-12-source"></a>
### Name-first DPT presentation

**Ledger status:** Not separately tracked.
Names are a display vocabulary, not codec/device support. Unknown/main-only identifiers stay exact; stored data, diagnostics and exports are not rewritten.
[Details and history](limitations/name-first-dpt-presentation.md)

<a id="catalogue-telegram-flow-delivery-and-scope"></a>
<a id="telegram-flow-visualization-is-approved-but-not-implemented"></a> <a id="telegram-flow-visualization-is-implemented-with-bounded-evidence"></a>
### Telegram-flow delivery and scope

**Ledger status:** `DONE` ([FLOW-01](status/LEDGER.md#post-snapshot-ids)).
Flow is implemented; approval-only wording is historical. Configured participants are not measured recipients/device state; Chromium/capacity limits remain in KL-154.
[Details and history](limitations/telegram-flow-delivery-and-scope.md)

## UI and accessibility

<a id="catalogue-kl-020"></a>
<a id="20-command-palette-and-search-share-overlay-css-and-an-accessibility-gap--partially-resolved"></a>
### 20. Overlay accessibility evidence

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-20](status/LEDGER.md#snapshot-ids)).
Shared modal/keyboard behavior is implemented. Scoped fixes do not certify complete accessibility, screen-reader or native WebKitGTK conformance.
[Details and history](limitations/KL-020-overlay-accessibility-evidence.md)

<a id="catalogue-kl-024"></a>
<a id="24-fspicker-has-no-drag-and-drop-or-multi-select"></a>
### 24. File picker upload and selection

**Ledger status:** Not separately tracked.
Multi-file upload/drop is implemented; project selection remains singular. Cancellation is not rollback of an upload already sent.
[Details and history](limitations/KL-024-file-picker-upload-and-selection.md)

<a id="catalogue-kl-037"></a>
<a id="37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12"></a> <a id="37-translations-reach-selected-surfaces-not-every-imported-text-or-ui-output"></a>
### 37. Translation coverage

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-37](status/LEDGER.md#snapshot-ids)).
Overlays reach selected surfaces, not every imported text or UI output. Fallback labels and untranslated technical detail remain visible.
[Details and history](limitations/KL-037-translation-coverage.md)

<a id="catalogue-kl-043"></a>
<a id="43-animations-have-no-in-app-switch-only-the-os-reduced-motion-preference"></a> <a id="43-animation-controls-exist-some-motion-surfaces-remain-outside-their-guard"></a>
### 43. Motion controls and residual surfaces

**Ledger status:** `DONE` ([KL-43](status/LEDGER.md#snapshot-ids)).
Global/per-category controls exist. Recorded surface, lifecycle and native/accessibility limits remain; controls are not an every-effect guarantee.
[Details and history](limitations/KL-043-motion-controls-and-residual-surfaces.md)

<a id="catalogue-kl-060"></a>
<a id="60-project-diffs-web-panel-lists-entities-with-filtered-virtualised-long-tables"></a>
### 60. Project-diff table presentation

**Ledger status:** `DONE` ([KL-60](status/LEDGER.md#snapshot-ids)).
Filtered virtualized entity tables are implemented. Display adds no apply/merge capability or universal matching guarantee.
[Details and history](limitations/KL-060-project-diff-table-presentation.md)

<a id="catalogue-kl-066"></a>
<a id="66-server-composed-prose-and-the-documentation-export-are-not-translated-by-any-ui-language-or-pack--partially-resolved-2026-09-14-t14"></a> <a id="66-server-composed-prose-and-documentation-are-only-partly-localized--partially-resolved-2026-09-23-t14"></a> <a id="66-server-composed-prose-is-only-partly-localized"></a>
### 66. Technical prose localization

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-66](status/LEDGER.md#snapshot-ids)).
Translated diagnostics expose raw English detail on request. This is not complete localization or reconstruction of every controlling source fact.
[Details and history](limitations/KL-066-technical-prose-localization.md)

<a id="catalogue-kl-097"></a>
<a id="97-progress-is-a-phase-label-far-more-often-than-it-is-a-percentage"></a>
### 97. Indeterminate import progress

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-97](status/LEDGER.md#snapshot-ids)).
Most phases expose truthful labels, not percentages. Smooth completion requires totals the implementation does not know.
[Details and history](limitations/KL-097-indeterminate-import-progress.md)

<a id="catalogue-kl-098"></a>
<a id="98-almost-nobody-will-ever-see-the-second-flavour-message"></a>
### 98. Load-banner message timing

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-98](status/LEDGER.md#snapshot-ids)).
Short loads show few rotating messages. Rotation targets long loads and never deliberately delays work.
[Details and history](limitations/KL-098-load-banner-message-timing.md)

<a id="catalogue-kl-100"></a>
<a id="100-help-prose-lives-in-the-message-catalogue-one-paragraph-per-key"></a>
### 100. Help text format

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-100](status/LEDGER.md#snapshot-ids)).
Help consists of plain translation-key paragraphs. No Markdown/rich-text help engine or per-topic help store exists.
[Details and history](limitations/KL-100-help-text-format.md)

<a id="catalogue-kl-121"></a>
<a id="121-two-open-windows-do-not-see-each-others-preference-changes-until-one-reloads"></a>
### 121. Cross-window preference refresh

**Ledger status:** `DONE` ([KL-121](status/LEDGER.md#snapshot-ids)).
Visible windows refresh periodically/on focus. Bounded synchronization is not user isolation or native-platform verification.
[Details and history](limitations/KL-121-cross-window-preference-refresh.md)

<a id="catalogue-kl-130-zoom"></a>
<a id="130-application-zoom-is-browser-verified-not-native-webkitgtk-verified"></a>
### 130. Application zoom

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-130-ZOOM](status/LEDGER.md#snapshot-ids)).
Zoom is bounded to 80–150% in 10% steps. Browser evidence does not prove native focus/resize/accessibility parity.
[Details and history](limitations/KL-130-ZOOM-application-zoom.md)

<a id="catalogue-kl-160"></a>
<a id="160-the-first-run-guide-is-remembered-per-installation-not-per-person"></a>
### 160. Shared first-run guide state

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-160](status/LEDGER.md#post-snapshot-ids)).
Guide state is per installation, not person. Fresh concurrent windows can both show it; dismissal affects all server users.
[Details and history](limitations/KL-160-shared-first-run-guide-state.md)

<a id="catalogue-kl-164"></a>
<a id="164-achievements-are-per-installation-count-only-in-the-ui-and-can-lose-a-concurrent-increment"></a>
### 164. Shared achievement counters

**Ledger status:** Not separately tracked.
Counters are installation-wide and UI-only. Since 13 October 2026 concurrent steps add up (deduplicated over the last 1,024 batches). Hidden catalogue entries are not secrets.
[Details and history](limitations/KL-164-shared-achievement-counters.md)

<a id="catalogue-kl-178"></a>
<a id="178-the-group-address-picker-filters-and-marks-it-does-not-judge"></a>
### 178. Group-address picker matching

**Ledger status:** `DONE` ([KL-178](status/LEDGER.md#post-snapshot-ids)).
Search is substring-per-word with caller order, not fuzzy ranking. DPT marks disclose identities, not device compatibility.
[Details and history](limitations/KL-178-group-address-picker-matching.md)

<a id="catalogue-kl-179"></a>
<a id="179-tree-indentation-is-capped-at-twelve-levels-flat-views-stay-flat"></a>
### 179. Tree indentation limits

**Ledger status:** `DONE` ([KL-179](status/LEDGER.md#post-snapshot-ids)).
Indentation caps at depth 12; flat views stay flat. Measured channel fixes do not establish every grouped-table layout.
[Details and history](limitations/KL-179-tree-indentation-limits.md)

<a id="catalogue-communication-object-table"></a>
<a id="communication-object-table-scope-2026-10-09"></a>
### Communication-object table

**Ledger status:** Not separately tracked.
DPT labels do not establish manufacturer semantics. Table word floors cap at 320 px; wider tokens or explicit shrinking can split words. Wide fitting is verified with demo/fixture content only.
[Details and history](limitations/communication-object-table.md)

<a id="catalogue-crt-motion"></a>
<a id="crt-motion-is-browser-verified-application-behavior-not-theme-pack-v2"></a>
### CRT motion

**Ledger status:** Not separately tracked.
Motion is a separate preference, not executable theme-pack behavior. Save glow marks activation, not success; Chromium fixtures do not prove native persistence/accessibility.
[Details and history](limitations/crt-motion.md)

<a id="catalogue-current-project-device-navigation"></a>
<a id="devices-navigation-local-source-current-project-links-and-bounded-evidence-2026-10-08"></a>
### Current-project device navigation

**Ledger status:** Not separately tracked.
Links resolve unique current-project devices, not physical/historical hardware. Metadata, large-project rendering and native/accessibility evidence remain bounded.
[Details and history](limitations/current-project-device-navigation.md)

<a id="catalogue-four-group-address-views"></a>
<a id="four-group-address-presentations-2026-10-12"></a>
### Four group-address views

**Ledger status:** Not separately tracked.
The matrix includes linked objects plus explicitly loaded device objects, not an automatic whole-project inventory. Views add no hardware model/direct matrix writes.
[Details and history](limitations/four-group-address-views.md)

<a id="catalogue-import-count-links-and-unsaved-change-dialogs"></a>
<a id="project-status-import-links-and-unsaved-changes-dialogs-2026-10-12"></a>
### Import-count links and unsaved-change dialogs

**Ledger status:** Not separately tracked.
Import links show a capped current-run log, which may hold fewer findings than the report. Shared Chromium-tested dialogs do not establish native behavior.
[Details and history](limitations/import-count-links-and-unsaved-change-dialogs.md)

<a id="catalogue-installation-node-editing"></a>
<a id="local-installation-node-ui-boundary-2026-10-11"></a> <a id="recorded-device-state-view-boundary-2026-10-11"></a> <a id="local-modeled-project-information-boundary-2026-10-11"></a> <a id="local-native-export-privacy-boundary-2026-10-11"></a> <a id="local-retained-payload-access-boundary-2026-10-11"></a> <a id="local-durable-import-report-boundary-2026-10-11"></a>
### Installation-node editing

**Ledger status:** Not separately tracked.
Unnamed nodes are readable and renameable; the rename route accepts a revision binding the web editor does not send yet. This is not a general installation-management workflow.
[Details and history](limitations/installation-node-editing.md)

<a id="catalogue-lcars-presentation"></a>
<a id="lcars-presentation-is-chromium-verified-not-an-executable-imported-pack"></a>
### LCARS presentation

**Ledger status:** Not separately tracked.
Built-in framing is not executable imported geometry or lossless v1 export. Native WebKitGTK, Orca and complete WCAG/browser acceptance remain unverified.
[Details and history](limitations/lcars-presentation.md)

<a id="catalogue-ported-ux-feedback-limits"></a>
<a id="ported-ux-feedback-2026-10-13"></a>
### Ported UX feedback limits

**Ledger status:** Not separately tracked.
Toast queues, flow-image exports and motion controls are bounded presentation. Waiting cards can drop; snapshots/notifications do not establish device success or full history.
[Details and history](limitations/ported-ux-feedback-limits.md)

<a id="catalogue-theme-pack-management"></a>
<a id="theme-pack-management-is-a-local-candidate-not-release-acceptance"></a> <a id="verified-theme-pack-management-is-not-whole-extension-release-acceptance"></a>
### Theme pack management

**Ledger status:** Not separately tracked.
Declarative pack management is delivered, not universal extension compatibility. Native/accessibility and broader format capabilities retain their recorded boundaries.
[Details and history](limitations/theme-pack-management.md)

## Platform, deployment and security

<a id="catalogue-kl-008"></a>
<a id="8-knx-secure-is-not-implemented"></a>
### 8. KNX Secure runtime

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-8](status/LEDGER.md#snapshot-ids)).
Research adds no Secure runtime, keyring decoding, AES-project import or credential vault. Retained/native backups may contain sensitive material.
[Details and history](limitations/KL-008-knx-secure-runtime.md)

<a id="catalogue-kl-016"></a>
<a id="16-tauri-v2s-linux-backend-depends-on-archived-gtk3-bindings"></a> <a id="16-tauri-v2-remains-on-gtk3-former-maintenance-advisories-are-resolved"></a>
### 16. GTK3 desktop dependency

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-16](status/LEDGER.md#snapshot-ids)).
The desktop remains GTK3-based. Withdrawn maintenance advisories provide neither a GTK4 port nor broader native-platform validation.
[Details and history](limitations/KL-016-gtk3-desktop-dependency.md)

<a id="catalogue-kl-022"></a>
<a id="22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback--resolved-2026-09-20"></a> <a id="22-one-shared-server-password-no-per-user-rights-or-internet-security-claim"></a>
### 22. Server authentication and exposure

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-22](status/LEDGER.md#snapshot-ids)).
One password is not per-user rights or internet-security certification. Host-network exposure requires trusted LAN/VPN/firewall; unauthenticated listeners remain host-loopback-only.
[Details and history](limitations/KL-022-server-authentication-and-exposure.md)

<a id="catalogue-kl-063"></a>
<a id="63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all"></a>
### 63. Shared server project and undo stack

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-63](status/LEDGER.md#snapshot-ids)).
Clients share one project/undo stack. Feature-specific guards are not general concurrent-edit isolation, accounts or collaboration.
[Details and history](limitations/KL-063-shared-server-project-and-undo-stack.md)

<a id="catalogue-kl-065"></a>
<a id="65---version-names-a-commit-never-a-working-tree"></a>
### 65. Version identity and dirty builds

**Ledger status:** `DONE` ([KL-65](status/LEDGER.md#snapshot-ids)).
Version output identifies the build commit, not uncommitted state. No dirty-tree marker guarantees exact build provenance.
[Details and history](limitations/KL-065-version-identity-and-dirty-builds.md)

<a id="catalogue-kl-106"></a>
<a id="106-the-debug-report-redacts-five-pattern-classes-and-nothing-else"></a>
### 106. Debug-report privacy limits

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-106](status/LEDGER.md#snapshot-ids)).
Redaction covers named patterns only. Source/XML/endpoints can identify projects; reports/native exports are not comprehensive anonymization.
[Details and history](limitations/KL-106-debug-report-privacy-limits.md)

<a id="catalogue-kl-130-gate"></a>
<a id="130-a-gate-binary-can-verify-a-directory-that-no-longer-exists"></a>
### 130. Resolved gate-root validation

**Ledger status:** `DONE` ([KL-130-GATE](status/LEDGER.md#snapshot-ids)).
Runtime workspace-root validation resolves this gate defect. Historical evidence does not certify every unrelated tool's root handling.
[Details and history](limitations/KL-130-GATE-resolved-gate-root-validation.md)

<a id="catalogue-kl-133"></a>
<a id="133-a-dead-webview-cannot-be-closed-with-the-window-managers-close-button"></a>
### 133. Terminated versus hung webviews

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-133](status/LEDGER.md#snapshot-ids)).
Terminated processes can reload; hangs without termination remain a native close/recovery limitation. Chromium cannot settle that case.
[Details and history](limitations/KL-133-terminated-versus-hung-webviews.md)

<a id="catalogue-kl-155"></a>
<a id="155-tunnelling-from-a-container-on-dockers-bridge-network-gets-no-answer"></a>
### 155. Bridge-network tunnel reachability

**Ledger status:** `DONE` ([KL-155](status/LEDGER.md#post-snapshot-ids)).
Route-Back has offline evidence; real bridged tunnels remain unverified. Linux host networking is the workaround, not elimination of multicast/gateway constraints.
[Details and history](limitations/KL-155-bridge-network-tunnel-reachability.md)

<a id="catalogue-kl-158"></a>
<a id="158-the-appimage-starts-only-with-an-x-server"></a>
### 158. AppImage display-backend evidence

**Ledger status:** `DONE` ([KL-158](status/LEDGER.md#post-snapshot-ids)).
Source no longer universally forces X11. Old assets and one-host native startup evidence do not establish every-host compatibility.
[Details and history](limitations/KL-158-appimage-display-backend-evidence.md)

<a id="catalogue-kl-161"></a>
<a id="161-alpha5-does-not-pick-up-the-alpha4-data-folder"></a>
### 161. Older desktop data-folder transition

**Ledger status:** Not separately tracked.
Alpha.5 changed the identifier without automatic alpha.4 data migration. Copy preserved data deliberately; an empty new library is not evidence of deletion.
[Details and history](limitations/KL-161-older-desktop-data-folder-transition.md)

<a id="catalogue-kl-163"></a>
<a id="163-stopping-the-server-does-not-wait-for-a-device-download-or-address-programming"></a>
### 163. Server shutdown and active writes

**Ledger status:** Not separately tracked.
A stop refuses new bus jobs and waits a bounded, configurable time for a running download/address programming; a write still running when that ends, or under SIGKILL, is cut mid-way.
[Details and history](limitations/KL-163-server-shutdown-and-active-writes.md)

<a id="catalogue-kl-168"></a>
<a id="168-the-docker-hub-image-follows-release-tags-only-and-latest-follows-alphas"></a>
### 168. Docker release-image publication

**Ledger status:** Not separately tracked.
Images follow tags; latest may be an alpha. Provenance is not signing; base updates do not follow every source commit.
[Details and history](limitations/KL-168-docker-release-image-publication.md)

<a id="catalogue-kl-169"></a>
<a id="169-browser-uploads-remain-bounded-and-return-navigation-reloads-the-editor"></a>
### 169. Browser upload and navigation limits

**Ledger status:** Not separately tracked.
Raw/member/XML budgets remain; large admission proves neither full scanning nor bounded memory. Return navigation reloads the editor.
[Details and history](limitations/KL-169-browser-upload-and-navigation-limits.md)

<a id="catalogue-kl-176"></a>
<a id="176-appimages-up-to-alpha9-abort-on-hosts-with-newer-mesa-egl"></a>
### 176. AppImage Mesa EGL compatibility

**Ledger status:** `DONE` ([KL-176](status/LEDGER.md#post-snapshot-ids)).
The alpha.9-and-earlier libwayland failure has a scoped launcher fix. One-host repair does not guarantee every Mesa/EGL/display backend.
[Details and history](limitations/KL-176-appimage-mesa-egl-compatibility.md)

<a id="catalogue-kl-181"></a>
<a id="181-appimages-up-to-alpha11-cannot-save-the-session-log-or-a-bus-monitor-capture"></a>
### 181. Desktop export commands

**Ledger status:** `DONE` ([KL-181](status/LEDGER.md#post-snapshot-ids)).
AppImages up to alpha.11 refuse saving the session log and monitor captures; fixed in source, ships with the next AppImage. Web and Docker unaffected.
[Details and history](limitations/KL-181-desktop-export-commands.md)

<a id="catalogue-kl-182"></a>
<a id="182-appimages-up-to-alpha11-save-in-app-downloads-silently-into-downloads-and-replace-same-named-files"></a>
### 182. Desktop silent downloads

**Ledger status:** `DONE` ([KL-182](status/LEDGER.md#post-snapshot-ids)).
AppImages up to alpha.11 save in-app downloads silently into Downloads, replacing same-named files; fixed in source with a save dialog. Web and Docker unaffected.
[Details and history](limitations/KL-182-desktop-silent-downloads.md)

<a id="catalogue-kl-185"></a>
### 185. Git history rewritten; GitHub pull-request refs keep old commits

**Ledger status:** `DONE` ([KL-185](status/LEDGER.md#post-snapshot-ids)).
History, messages and tags were rewritten on 2026-10-17 (map COMMIT_MAP_2026-10-17); GitHub still serves the pre-rewrite commits through two read-only pull-request refs and the fork.
[Details and history](limitations/KL-185-git-history-local-machine-details.md)

## Integrations and community

<a id="catalogue-kl-107"></a>
<a id="107-there-is-no-plugin-api--a-third-party-cannot-add-a-format-a-protocol-a-report-template-or-a-ui-panel-without-forking"></a>
### 107. Extension API scope

**Ledger status:** `ACCEPTED_BOUNDARY` ([KL-107](status/LEDGER.md#snapshot-ids)).
No general third-party format/protocol/UI plug-in API exists. Declarative configuration is not executable runtime extension support.
[Details and history](limitations/KL-107-extension-api-scope.md)

<a id="catalogue-kl-165"></a>
<a id="165-the-mcp-adapter-reads-saved-files-only-and-its-visibility-ignores-access"></a>
### 165. MCP state and visibility boundaries

**Ledger status:** Not separately tracked.
Read tools expose bounded project evidence; visibility is not access authorization. Opt-in literal renames are separate, not general project mutation.
[Details and history](limitations/KL-165-mcp-state-and-visibility-boundaries.md)

<a id="catalogue-community-support-evidence"></a>
<a id="community-evidence-is-bounded-manually-shared-and-not-anonymous-2026-10-08"></a>
### Community support evidence

**Ledger status:** Not separately tracked.
Sharing is manual and potentially identifying; raw originals are not anonymized. Scanning/redaction is bounded; cancellation does not synchronously stop server parsing.
[Details and history](limitations/community-support-evidence.md)

<a id="catalogue-fictional-community-demos"></a>
<a id="community-demos-are-fictional-offline-learning-data-not-building-designs-2026-10-08"></a>
### Fictional community demos

**Ledger status:** Not separately tracked.
Demos are offline learning data with fictional products, not electrical designs or verified ETS/device programs. Never download their programs to hardware.
[Details and history](limitations/fictional-community-demos.md)

<a id="catalogue-mcp-rename-confirmation"></a>
<a id="mcp-renames-rely-on-the-clients-confirmation-form-2026-10-12"></a>
### MCP rename confirmation

**Ledger status:** `LATER` ([FUTURE-01](status/LEDGER.md#snapshot-ids)).
The opt-in writer permits literal group-address renames through client form elicitation. It cannot prove a human answered or establish operator identity.
[Details and history](limitations/mcp-rename-confirmation.md)

<a id="catalogue-release-notifications"></a>
<a id="release-notification-boundary-2026-10-11"></a>
### Release notifications

**Ledger status:** Not separately tracked.
GitHub polling is not instant notification, installation or integrity verification. Offline/rate-limited responses cannot establish the newest release; opt-out cannot recall sent requests.
[Details and history](limitations/release-notifications.md)

<a id="catalogue-website-and-public-story"></a>
<a id="website-on-github-pages-checked-in-chromium-privacy-text-not-legally-reviewed"></a>
### Website and public story

**Ledger status:** Not separately tracked.
Chromium/publication/video evidence does not certify all browsers, accessibility, performance or hardware. Privacy/legal text has no legal-compliance certification.
[Details and history](limitations/website-and-public-story.md)

## Historical catalogue notes

<a id="catalogue-kl-162"></a>
<a id="162-commit-hashes-cited-before-2026-10-07-refer-to-the-rewritten-history"></a>
### 162. Rewritten Git commit identities

**Ledger status:** Not separately tracked.
Pre-7-October citations may reference rewritten hashes. Use the historical map rather than assume evidence is absent.
[Details and history](limitations/KL-162-rewritten-git-commit-identities.md)

<a id="catalogue-alpha8-release-snapshot"></a>
<a id="alpha8-release-snapshot-2026-10-10"></a>
### Alpha.8 release snapshot

**Ledger status:** Not separately tracked.
This dated release record is historical. Preserved-source evidence does not imply complete ETS semantics, export/signing or wider hardware eligibility.
[Details and history](limitations/alpha8-release-snapshot.md)

<a id="catalogue-closed-entries-and-legacy-links"></a>
<a id="closed-entries-and-historical-links"></a> <a id="10-project-licence--resolved-2026-09-16"></a> <a id="10-the-project-licence-is-not-decided"></a> <a id="103-unsaved-is-inferred-from-the-undo-stack-not-a-real-dirty-flag"></a> <a id="103-unsaved-is-inferred-from-the-undo-stack-not-a-real-dirty-flag--resolved"></a> <a id="11-knxprod-files-for-master-data-scheme--12-cannot-be-imported-directly"></a> <a id="117-read_on_init_flag-is-parsed-and-stored-then-discarded-before-it-reaches-knx-core"></a> <a id="118-a-succeeded-project-load-announces-nothing-to-a-screen-reader"></a> <a id="118-a-succeeded-project-load-announces-nothing-to-a-screen-reader--resolved"></a> <a id="119-on-this-machines-ntfs3-mount-cargo-has-rebuilt-from-a-stale-fingerprint--a-green-gate-is-not-evidence-by-itself"></a> <a id="120-nothing-checks-that-a-theme-is-legible"></a> <a id="120-nothing-checks-that-a-theme-is-legible--resolved-by-the-role-pair-contrast-gate"></a> <a id="122-resolved-settings-file-diagnostics-follow-the-ui-language"></a> <a id="122-settings-file-notices-reach-the-user-in-english-only"></a> <a id="123-resolved-group-addresses-no-longer-use-dotted-display-notation"></a> <a id="131-seventy-two-corpus-gates-repo-wide-still-pass-when-the-corpus-is-absent"></a> <a id="132-the-window-managers-close-button-quits-the-desktop-app-without-the-unsaved-changes-prompt"></a> <a id="147-a-received-telegrams-priority-repeat-flag-and-hop-count-are-not-kept--lifted-2026-09-30"></a> <a id="148-the-contributor-license-agreement-is-not-reviewed-by-a-lawyer-and-nothing-enforces-it--withdrawn-2026-09-30"></a> <a id="17-deleting-a-group-address-can-leave-a-dangling-grouplink--resolved"></a> <a id="19-a-search-result-inside-a-collapsed-tree-branch-is-not-revealed"></a> <a id="19-a-search-result-inside-a-collapsed-tree-branch-is-not-revealed--resolved-2026-09-22-t12"></a> <a id="21-a-ui-created-group-address-without-a-range-is-still-dropped-on-export--partially-resolved"></a> <a id="21-resolved-export-refuses-a-group-address-without-a-range"></a> <a id="22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback"></a> <a id="22-the-webdocker-deployment-target-has-no-authentication"></a> <a id="25-resolved-the-web-package-and-docker-frontend-stage-use-node-22"></a> <a id="27-tunnelclient-heartbeat-retry-has-a-narrow-race-condition--resolved"></a> <a id="28-tunnelclient-subscribers-receive-no-signal-when-the-tunnel-closes--resolved"></a> <a id="30-apiprojectdownload-has-no-frontend-caller"></a> <a id="30-apiprojectdownload-has-no-frontend-caller--resolved-2026-09-22"></a> <a id="32-routing_busy-is-logged-not-honored-by-routingclient--resolved"></a> <a id="33-routingclients-round-trip-test-transmitted-on-the-physical-lan-not-on-loopback--resolved-2026-09-20"></a> <a id="34-schema-21-export-drops-a-handful-of-known-but-unmapped-per-deviceper-line-attributes--resolved-2026-09-20"></a> <a id="35-device-creation-enrichmentissues-are-silently-dropped--resolved-2026-09-10"></a> <a id="4-round-trips-are-semantic-not-byte-exact"></a> <a id="4-round-trips-are-semantic-not-byte-exact--closed-2026-09-20-export-withdrawn"></a> <a id="49-project-documentation-export-has-an-in-application-preview-and-print-action"></a> <a id="49-project-documentation-export-has-no-in-application-print-preview"></a> <a id="5-exports-are-unsigned-and-ets-acceptance-is-untested"></a> <a id="5-exports-are-unsigned-and-ets-acceptance-is-untested--closed-2026-09-20-export-withdrawn"></a> <a id="50-project-documentation-export-has-no-section-selection"></a> <a id="50-project-documentation-export-has-section-selection-in-the-web-ui"></a> <a id="57-project-diff-cannot-compare-against-a-raw-knxproj"></a> <a id="57-raw-knxproj-comparison-is-available-on-the-cli-and-in-the-web-ui"></a> <a id="58-project-diff-has-an-opt-in-ci-exit-code-contract"></a> <a id="58-project-diff-has-no-ci-friendly-exit-nonzero-on-any-difference-flag"></a> <a id="59-project-diff-exposes-beforeafter-values-but-the-web-panel-does-not-render-them-yet"></a> <a id="59-project-diff-shows-beforeafter-values-in-cli-api-and-web-panel"></a> <a id="59-project-diffs-text-and-web-renderers-show-which-fields-changed-not-their-beforeafter-values-for-most-entity-types"></a> <a id="60-project-diffs-web-panel-shows-grouped-counts-only"></a> <a id="67-a-rejected-language-packs-own-reason-was-shown-untranslated-inside-a-translated-sentence--resolved-2026-09-14-t14"></a> <a id="80-a-project-can-be-created-from-scratch-in-the-ui--resolved-2026-09-16-goal-task-17"></a> <a id="81-new_project_impl-refuses-on-can-undo-not-on-is-dirty"></a> <a id="81-new_project_impl-refuses-on-can-undo-not-on-is-dirty--resolved"></a> <a id="83-the-from-scratch-launcher-is-browser-verified--resolved-2026-09-16-goal-task-17"></a> <a id="84-a-projects-group-address-style-can-be-chosen-and-afterwards-never-seen--resolved-2026-09-14-t4"></a> <a id="89-five-documented-spacetype-values-are-coarsened-to-buildingpart-on-import"></a> <a id="89-five-documented-spacetype-values-are-coarsened-to-buildingpart-on-import--resolved-2026-09-22"></a> <a id="91-a-running-bus-session-keeps-rendering-group-addresses-in-the-style-the-project-had-when-it-started"></a> <a id="91-a-running-bus-session-keeps-rendering-group-addresses-in-the-style-the-project-had-when-it-started--resolved"></a> <a id="92-commissioning-phase-2-is-verified-against-a-simulator-this-project-wrote-and-has-never-addressed-a-device"></a> <a id="96-a-browser-that-loses-the-import-response-cannot-get-the-project-back-without-reloading"></a>
### Closed entries and legacy links

**Ledger status:** Not separately tracked.
Removed/withdrawn entries retain historical anchors and evidence links. Numbers stay reserved and are not reopened by reorganization.
[Details and history](limitations/closed-entries-and-legacy-links.md)

<a id="catalogue-historical-reading-guide"></a>
<a id="current-reading-guide--8-october-2026"></a>
### Historical reading guide

**Ledger status:** Not separately tracked.
The 8 October audit is a dated interpretation guide, not the current feature inventory or a new work assignment.
[Details and history](limitations/historical-reading-guide.md)
