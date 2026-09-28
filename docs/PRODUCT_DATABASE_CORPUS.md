# Gira and MDT product-database corpus

Investigation date: **2026-09-23**.

This note inventories the local, ignored corpora under
`OriginalData/ProductDatabases/Gira` and
`OriginalData/ProductDatabases/MDT`. It is an implementation-planning aid, not
a compatibility claim. The manufacturer files are not redistributed.

## Method and reproducibility

`tools/inspect_product_corpus.py` scans files and nested ZIP members without
extracting them. It records hashes, ZIP structure, XML namespaces, element and
attribute frequencies, product/program metadata, parameter kinds, dynamic-tree
shapes, languages and read failures as JSON. XML/ZIP members larger than 128 MiB
are skipped and nesting is limited to three levels.

```bash
python3 tools/inspect_product_corpus.py \
  OriginalData/ProductDatabases/Gira \
  OriginalData/ProductDatabases/MDT \
  --output "$TMPDIR/product-corpus.json"
```

The ignored `corpus_compatibility_matrix` integration test installs every
discovered modern package both in isolation and in deterministic content-hash
order. It exercises the real `install_package` and `open_and_migrate` paths and
writes an anonymized JSON result only when all three explicit corpus environment
variables are present. Ordinary test runs skip the private measurement instead
of reporting a false pass.

Important interpretation rules:

- `http://knx.org/xml/project/N` is called **scheme N** here. It is not an ETS
  release number. Creator/tool metadata shows, for example, scheme-11 packages
  written or converted by tools from ETS/MT generations 4 through 6.
- Element totals are corpus observations, not unique domain entities. Exact
  duplicate packages and shared master/baggage data are included.
- Successful installation proves that the current parser accepted and stored
  the tested path. It does not prove lossless interpretation; unknown-report
  entries are quantified separately.

## PDB-3 evidence contract

Product-package compatibility measurements are reported through the schema-v12
install ledger. A measured install exposes deterministic category/disposition
count rows, all six dispositions (`read`, `stored`, `deduplicated`,
`retained-but-uninterpreted`, `unsupported`, `dropped`), unknown constructs
with distinct and occurrence counts, and unsupported diagnostics. Paths in
those diagnostics are archive-relative/XML-relative; host `source_name` is
never part of the server, CLI, or web projection.

A `facts: null` response means the package predates the v12 ledger and its
encounter facts are historically unavailable. It must not be interpreted as a
measured zero. This evidence describes KNXBench importer behavior only: it does
not establish ETS parity and does not verify package signatures. PDB-8 is the
future typed master-data coverage slice. PDB-10 (schema v16, ADR-0042) inventories baggage: every `Baggages.xml` declaration typed as raw lexemes and resolved exactly to its member, every payload classified by content, nested ZIPs measured from their directory only.

Corpus measurements must remain opt-in and confined to an explicitly supplied
local corpus root. No private corpus content, manufacturer identity, or host
path belongs in committed reports.

The PDB-3 matrix remeasurement records legacy `InstallReport::unknown` totals of
22,404 for isolated successful attempts, 22,279 for installed shared-order
attempts, and 22,404 for all successful shared-order attempts. These are counts
of distinct unknown rows in each report, not the new ledger's occurrence totals.
The baseline changed deliberately. The isolated decrement comes from classifying
`Baggages.xml` as unsupported baggage-index evidence instead of a generic
unrecognized package member. The shared-order increases come from retaining
unknown evidence encountered below a declaration whose storage parent is
rejected. These totals are importer evidence, not a claim that the constructs
are understood.

## Inventory

| Corpus | Top-level files | Modern packages | Other legacy files | Schemes |
| --- | ---: | ---: | ---: | --- |
| Gira | 10 | 15 | 0 | 12: 1, 20: 13, 21: 1 |
| MDT | 101 | 100 | 1 encrypted `.pr5` | 11: 48, 13: 4, 14: 3, 20: 43, 21: 2 |
| **Total** | **111** | **115** | **1** | **11: 48, 12: 1, 13: 4, 14: 3, 20: 56, 21: 3** |

All 115 modern packages are readable ZIP files and contain a root
`knx_master.xml`. No malformed ZIP/XML package was found by the inventory
scanner. Gira predominantly ships download ZIPs containing one or more nested
`.knxprod` files; MDT predominantly ships top-level `.knxprod` files. Import
must therefore distinguish a download bundle from an installable package and
must not mistake nesting for the product-package format itself.

Observed modern-package content (including exact duplicates) comprises:

- 310 application-program declarations and 388 product declarations;
- 234,731 parameters, 50,549 communication objects and 148,385 communication-
  object references;
- 8,611 dynamic module instances;
- 620 function types, 2,542 function points and 1,181 space usages;
- 1,728 baggage declarations; and
- up to 24 languages, with modern packages commonly carrying the full language
  set and older scheme-11 packages often carrying only German/English or a
  small Western-European set.

There are exact duplicates which should be deduplicated by content hash, not
filename. Notable examples are Gira's loose and wrapped
`Dummy_Applikation_Secure.knxprod`, and the two MDT files
`MDT_KP_SCN_02 Glass_Room_Temperature_Controller_V12[ a].knxprod`. Shared help,
icon and symbol baggage archives are also repeated across versions/products.
Consequently the 15 Gira package instances represent 14 distinct package
hashes, while the 100 MDT package instances represent 99 distinct hashes.

The packages are not XML-only. Direct package members include PNG/JPG images,
PDF, MSI, extensionless files and nested ZIPs; recursively scanning nested help
and icon bundles also finds thousands of text assets. Representative risk cases
are MDT's opaque `ETS USB Installation.msi` and nested symbol ZIPs. These bytes
must never be executed or blindly extracted. Baggage inventory should report
declared/expanded size, media type, nesting and encryption while retaining the
original bytes.

Large-file behavior is relevant: the largest observed Gira XML member is
24,201,031 bytes, and the largest MDT XML member is 54,803,397 bytes. Corpus
regressions should therefore cover bounded memory and deterministic reporting,
not just tiny synthetic fixtures.

### PDB-10 preflight: baggage index and payload shape (2026-09-28)

A read-only, aggregate-only probe (Python `zipfile`/`ElementTree`, no
extraction to disk, no names or contents printed) over the whole
`OriginalData/ProductDatabases` tree: 117 package instances, 115 distinct
package hashes, each counted once.

- **Index grammar.** 38 `Baggages.xml` files, exactly the path
  `KNX/ManufacturerData/Manufacturer/Baggages/Baggage/FileInfo`, nothing else.
  777 `Baggage` declarations, each with exactly one `FileInfo`.
  `Baggage/@Id`, `@Name` and `@TargetPath` on all 777; `@InstallOnImport` on
  129 with the observed values `true`, `false` and `0` (so boolean *and*
  numeric spellings). `FileInfo/@TimeInfo` on all 777 (ISO date-time);
  `FileInfo/@Version` on only 2. The Project Schema (v3.0.0 §4.2) says only
  that each `Baggage` is stored as an external file; the attribute semantics
  above are corpus observations `[A]`, not specified.
- **Index → payload.** All 777 declarations resolve *exactly* (case-sensitive)
  to the member `M-XXXX/Baggages/<TargetPath>/<Name>`. There are 790
  `Baggages/` members, so 13 payloads have no declaration in their package.
- **Payload media.** Extensions: `.png` 734, `.zip` 37, `.ai` 11, `.jpg` 5,
  `.dll` 1, `.pdf` 1, `.msi` 1. Magic bytes disagree with extensions: PNG 699,
  ZIP 37, **BMP 35** (all named `.png`), PDF 12 (the 11 `.ai` files are PDF),
  JPEG 5, PE executable 1, OLE2 compound file 1. Classification must use
  content, never the extension.
- **Size and encryption.** No member carries the ZIP encryption flag. Deflate
  431, stored 359. Payload size min 205 B, median 2,623 B, p99 223,370 B,
  max 2,111,931 B, total 15,009,507 B.
- **Nesting.** 37 nested ZIPs with 7,144 entries in total, the largest
  expanding to 317,240 bytes; none contains a further ZIP and none has an
  encrypted entry. They stay opaque; the inventory may read their central
  directory for counts/sizes but must not extract them.
- **Reconciled units.** The inventory above (1,728) counts every `Baggage`
  element in every XML of every package instance; this probe counts index
  declarations once per distinct package. Neither is pinned for the other.
  A per-location recount over the whole tree (117 instances) finds 1,730
  `Baggage` elements (1,713 counting each distinct package once): 786 in
  `Baggages.xml` (777 distinct, the declarations above), 943 as
  `ApplicationProgram/Static/Extension/Baggage` references (935 distinct)
  and 1 under `Hardware/Product/Baggages`. The program references point
  at baggage by `RefId`; PDB-10 does not type or resolve them. The program
  parser reports `Extension`, its `Baggage` child and the `RefId` attribute
  as unknown constructs (verified with a throwaway ingest probe), so they
  are reported, not dropped.
  Over the matrix scopes (`Gira`, `MDT`: 115 instances, 113 distinct), an
  independent Python recount gives 37 index members, 776 declarations, 789
  payloads and 13 undeclared payloads, no unresolved declaration and no
  index unknown; the installer's inventory must match those numbers.
- **Largest XML member** in the tree: 54,803,397 bytes (unchanged).

### PDB-11: package identity and versions (2026-09-28)

Read-only, aggregate-only probes (ADR-0043 Context; 117 package instances,
115 unique in the probe's discovery): every id that appears in more than one
unique package does so in member files whose bytes differ. Whether the
*element* differs depends on the kind — application programs 2 of 29
byte-identical but 19 of 29 attribute-identical, hardware 18 of 68, products
23 of 69, catalogue sections 1 of 53, all 33 `Hardware2Program` and 16 of 17
catalogue items identical. `ApplicationNumber`/`ApplicationVersion` match the
program id in 275 of 275 programs; families by (manufacturer, number) have
size 1 (183), 2 (40), 3 (4). `ReplacesVersions` is absent on 145 distinct
programs, one integer on 55, a whitespace-separated list on 75; 16 of 289
listed versions name a program in the corpus. 73 of 304 order numbers appear
in more than one package (44 with different program sets, 18 with different
schemes); 4 sit on more than one product id.

The schema-v17 matrix (115 instances / 113 unique hashes, shared order by
package hash) records, over the shared database:

| kind | candidate rows | distinct ids | ids in >1 blob | ids with differing digests |
|---|---:|---:|---:|---:|
| application_program | 302 | 273 | 29 | 25 |
| catalog_item | 362 | 345 | 17 | 0 |
| catalog_section | 251 | 103 | 53 | 49 |
| hardware | 334 | 255 | 68 | 50 |
| hardware2program | 337 | 298 | 33 | 0 |
| product | 386 | 306 | 69 | 46 |

170 ids diverge in total; all 528 parsed members were `measured`, none
`unavailable`. New tables: `package_source_name` 115, `source_identity`
1,972, `source_identity_scan` 528, `source_producer` 629. The "ids in >1
blob" column equals the Python probe's multi-package id counts exactly. The
differing-digest counts match the probe's whole-subtree canonical comparison
for programs (25; 27 differ byte-wise), hardware (50), products (46),
catalogue items (0; 1 differs byte-wise) and `Hardware2Program` (0). For
catalogue sections the matrix finds 49 where the probe's whole-subtree
comparison finds 52 and its own-content-only comparison 34: the digest
replaces a nested section or item by a marker, so a section whose only
difference lies inside a nested tracked element is reported equal here and
that element's own digest carries the difference. This explanation is
inferred from the two definitions, not separately measured. No install outcome, report total or pre-existing
table count changed.

After the independent review, every digest also covers the context the
stored row takes from outside the element (manufacturer, parent section,
parent hardware). The re-run matrix (979 s) produced the same aggregates
and the same baseline commitment: in this corpus, no id appears with equal
element bytes under different parents or manufacturers.

## Scheme and producer observations

The XML itself records `CreatedBy` and `ToolVersion`. The corpus contains output
from `MT`, `knxconv`, `ETS4` and `ETS5`; observed tool-version strings range from
4.x through 6.4.x. This is stronger evidence than guessing from filenames, but
it still describes the producer, not a guaranteed minimum runtime ETS version.
`ApplicationProgram/@MinEtsVersion`, where present, should be preserved and
reported separately.

Compared with the union of observed schemes 11 and 20:

- scheme 12 adds observed attributes `LdCtrlWriteProp/@AppliesTo`,
  `Property/@Occurrence` and `ParameterSeparator/@HorizontalRuler`;
- scheme 13 introduces no new observed element/attribute names;
- scheme 14 adds `LdCtrlDeclarePropDesc` and its property-description fields;
- scheme 21 also uses `LdCtrlDeclarePropDesc` and adds observed fields including
  variable/null-terminated data, optional resources, access policy,
  RF/coupler capabilities and `ApplicationProgram/@HardwareType`.

This comparison is a prioritization aid, not a proof that namespace widening is
safe. Semantics, cardinality, defaults, identifiers and load procedures still
need fixture-backed validation for each scheme.

## Current KNXBench result

The standalone package installer admits schemes 11, 12, 13, 14, 20 and the
exact scheme-21 namespace. The passing read-only, content-hash-ordered matrix
records all 115 instances: 115 isolated installs and, in deterministic shared
order, 113 installs plus two byte-identical deduplications. The isolated
unknown count is 23,347. Pinned shared totals, ten observed scheme-21 feature
frequencies and an aggregate identity/outcome commitment guard the result.
This is parser/persistence evidence, not ETS parity or commissioning evidence.
No private source identifiers or member values are part of the published matrix.

Scheme-21 synthetic support retains original XML bytes and reports targeted
observations across application-program, hardware, and master branches;
module-definition subtrees were not separately evidenced by synthetic tests. The
typed readers still dispatch by local name; scheme-21 typed members therefore
reject foreign element namespaces and qualified attributes atomically instead
of allowing an extension to create false typed rows. All 13 XML members in the
three observed scheme-21 packages use the canonical namespace and unqualified
attributes. This is explicit parser evidence, not semantic interpretation, ETS
parity, or commissioning support. Project-schema-21 `.knxproj` import is separate.

Scheme-13 support is deliberately narrower than an ETS compatibility claim.
The observed scheme-13 grammar introduced no new element or attribute names
relative to the measured scheme-11/20 union, representative rows are pinned by
a synthetic regression, malformed packages publish no database rows, and the
four real corpus packages pass the full isolated/shared matrix. No authoritative
scheme-13 XSD or independent semantic oracle is available.

Scheme-12/14 support applies the same boundary. The combined parser evidence
ledger records canonical `AppliesTo` (22), `Occurrence` (5), separator
`HorizontalRuler` (5), separator `Access` (2), separator `UIHint` (418), and
`LdCtrlDeclarePropDesc` (14) occurrences across those four packages, including
constructs nested below subtrees a specialized parser does not model. Values and
original XML bytes are retained, but KNXBench does not claim to execute load
procedures or reproduce manufacturer separator layout.

## Feature-shape observations relevant to implementation

### Parameters

Observed direct `ParameterType` children are:

- both corpora: `TypeRestriction`, `TypeNumber`, `TypePicture`, `TypeFloat`,
  `TypeText`, `TypeColor`;
- Gira additionally: `TypeRawData`;
- MDT additionally: `TypeNone`, `TypeIPAddress`, `TypeTime`.

PDB-9 whole-corpus scan (304 distinct programs, read-only, aggregate):
`TypeRestriction` 20,759, `TypeNumber` 4,153, `TypePicture` 1,118,
`TypeFloat` 579, `TypeText` 554, `TypeColor` 115, `TypeNone` 87,
`TypeIPAddress` 19, `TypeTime` 17, `TypeRawData` 3 — all ten typed since
schema v15. Unrecognized Dynamic kinds: `Rows`/`Columns` 4,267 each
(`Row` 9,275, `Column` 18,171; recognized layout since v15),
`ParameterBlockRename` 270, `Rename` 56, `Button` 20, `Repeat` 16 (each
holding exactly one `Module`). Static constructs reported but not
evaluated: `ParameterCalculation` 1,236 in 91 programs, `Allocator` 94 in 14.

Tests for parameter editing and reporting should cover every observed kind,
including raw fallback for kinds without an editor. Enum display text,
translations, ranges, scale/increment, encoding, UI hints and picture/baggage
references are separate concerns and must not be collapsed into one string.

### Dynamic programs and modules

Scheme 20 carries all observed module-heavy packages: 56 packages, 168
programs, 33 module-bearing packages and 8,611 module instances. The corpus is
a strong regression source for nested module expansion, allocator arguments and
conditional visibility.

Dominant dynamic nodes include `choose`, `when`, `ParameterRefRef`,
`ComObjectRefRef`, `Assign`, `ParameterBlock`, separators and modules. Layout
constructs are not rare: Gira has 17,740 `Column` and 9,009 `Row` nodes; MDT
also contains `ParameterBlockRename`, `Rename` and `Button`. Both corpora contain
`Repeat` around module instances. The evaluator currently recognizes only a
small semantic/transparent subset. Future UI work must decide deliberately
which layout nodes are transparent, which carry presentation semantics, and
which are unsupported; silently dropping them would make a parameter editor
look valid while changing the manufacturer's intended interaction model.

### Secure-capable application data

`IsSecureEnabled=true` occurs in 12 Gira package instances and 7 MDT package
instances (one Gira package is an exact loose/wrapped duplicate). These are
useful fixtures for preserving secure application metadata. They do **not**
prove KNX Data Secure commissioning or runtime interoperability; product
metadata support and bus-security implementation are separate layers.

Observed application-program attributes also include
`MaxSecurityGroupKeyTableEntries`, `MaxSecurityIndividualAddressEntries`,
`MaxSecurityP2PKeyTableEntries`, `MaxTunnelingUserEntries`, `MaxUserEntries`,
`MinEtsVersion` and `ReplacesVersions`. PDB-7 adds these eight unqualified
attributes to `PROGRAM_ATTRS`, nullable columns on the winning
`application_program` row, `query::ProgramRow` and `knx products show`.
Values are the parser-decoded XML strings, **not** validated security status,
numeric capacities or interpreted version relationships. Unknown qualified
lookalikes remain reported; original bytes still survive in `source_file`.
The v12→v13 migration rederives values from retained winning source blobs,
leaving historical package encounter reports unchanged.

PDB-7 read-only shape measurement (including duplicate package instances)
found all eight attributes on `ApplicationProgram`: `IsSecureEnabled` 34,
`MaxSecurityGroupKeyTableEntries` 34,
`MaxSecurityIndividualAddressEntries` 32,
`MaxSecurityP2PKeyTableEntries` 27, `MaxTunnelingUserEntries` 6,
`MaxUserEntries` 6, `MinEtsVersion` 310 and `ReplacesVersions` 140 XML
occurrences. A lexical-only shape classification of the same inventory
found `MinEtsVersion` 271 dotted-numeric / 39 other and `ReplacesVersions`
57 unsigned-decimal / 83 other. No source value or identifier is published.
These are source-field frequencies, not device capabilities
or unique program counts. [ADR-0037](adr/0037-product-program-catalog-metadata.md)
sets the source-value and migration boundary. The read-only package inventory
alone does not establish typed persistence, queries or corpus acceptance; those
require their own synthetic and matrix gates below.

The PDB-7 installer remeasurement persisted those eight fields in isolated
winning program rows at respective counts **34, 34, 32, 27, 6, 6, 310, 140**
(in the attribute order above). In deterministic shared-database order,
first-writer-wins produces **33, 33, 31, 27, 5, 5, 273, 130**; a losing
program does not overwrite the winner's metadata. Isolated reported unknowns
fell from 23,347 to **22,758**, exactly 589 recognized source-attribute
occurrences. This reduction does not imply the remaining unknown constructs
are understood. Shared installed-attempt unknowns are **22,642**; counting
successful byte-identical retries gives **22,758**. The matrix pins these
aggregate counts and a new corpus/outcome commitment. None of this verifies
the interpreted meaning of a security flag, capacity or ETS-version expression.

### Master data and load procedures

The packages exercise mask versions, memory/absolute/relative segments,
properties, resources, access rights and many `LdCtrl*` operations. Scheme 14/21
specifically exposes property-description declarations. These are relevant to
commissioning/download planning: a product catalogue can be useful while still
being insufficient to generate a correct device load procedure. Keep catalogue
installation, parameter interpretation and executable commissioning plans as
separate acceptance levels.

The embedded master data also contains `InterfaceObjectType`,
`InterfaceObjectProperty`, `PropertyDataType`, `MediumType`, `MaskVersion`,
`FunctionalBlock`, `DatapointRole`, `PublicKey` and `RSAKeyValue`. The current
typed master-data path intentionally covers only selected manufacturer/DPT/
function/space-usage data. Raw source preservation prevents byte loss, but it
does not provide queryable interface-object, mask, medium, role or security
metadata. Unsupported master sections need either typed storage or explicit
section-level reporting before they can support commissioning decisions.

PDB-8 status (schema v14): whole unsupported sections are reported per
section (`unsupported-master-section`), and every uninterpreted element subtree
*inside* a supported section is reported per occurrence at its canonical path
(`unsupported-master-subtree`). A read-only scan of the 69 distinct masters in
the private corpus found exactly three such roots: `DatapointSubtype/Format`
(12,072 occurrences, 38 masters), `Manufacturer/PublicKeys` (2,528, all 69) and
`Manufacturer/OrderNumberFormattingScript` (75, 33). None has typed storage;
reporting covers them until a feature needs one. Resources and access rights
sit inside `MaskVersions` and are reported per section only. Attributes in the
master `Languages` branch (e.g. `TranslationUnit/@Version`, 1,928 occurrences)
remain unreported; see KNOWN_LIMITATIONS.

## Recommended follow-up tasks

1. **Fix ZIP-name compatibility without weakening path safety.** Capture raw ZIP
   name flags/bytes, implement the intended legacy encoding policy explicitly,
   normalize only after decoding, and then apply traversal/absolute-path/NUL and
   duplicate-name checks. Add synthetic CP437/UTF-8/path-attack fixtures plus
   the corpus matrix. Scheme 21 has since passed its exact-namespace gate;
   keep unobserved schemes rejected until separately evidenced. Do not merely
   remove the rejection.
2. **Deepen scheme-21 evidence and typed coverage.** Scheme 21 is now accepted
   with synthetic evidence for its observed attribute deltas across application,
   hardware, and master branches. Keep those fields retained/report-only until
   fixture-backed semantics and query consumers exist; do not weaken namespace
   or path checks. The three-package subset passed the private matrix; semantic
   interpretation and query consumers remain separate work.
3. **Reduce unknown reports by capability area.** First classify unknowns into
   safe presentation metadata, retained-but-uninterpreted semantics and data
   required for editing/commissioning. Preserve source XML/attributes until a
   typed mapping is proven. Never make the report quieter by discarding data.
4. **Complete parameter-kind and dynamic-layout coverage.** Add corpus-derived,
   synthetic tests for all observed parameter kinds, Rows/Columns,
   rename/button nodes, repeat/module nesting, calculation transformations and
   allocator arguments. Separate evaluation semantics from UI layout.
5. **Expose a safe baggage inventory.** Model baggage references and metadata,
   classify images/documents/installers/nested archives without executing or
   extracting them, and retain all original bytes. Add maximum-size and nested-
   archive regressions using synthetic fixtures plus gated large-corpus tests.
6. **Treat the `.pr5` as legacy VD/PR work.** Reuse the independent EX-IM
   research and security/legal constraints in
   `VD4_PRODUCT_DATABASE_IMPORT.md`; do not feed it through the modern-package
   path or embed a password.
7. **Add catalogue-version UX.** Exact duplicate detection, same-order-number
   version grouping, producer/tool metadata, scheme, language coverage,
   secure-capable marker and replacement metadata are all available in this
   corpus and should be visible before users install or replace a product.
   PDB-11 (schema v17) supplies the data layer and CLI: per-candidate
   element digests, winner/loser naming, source names, producer facts,
   families, `ReplacesVersions` links and order-number lookup. Server/web
   views and a user-chosen winner remain open.

## Compatibility conclusion

The passing read-only matrix records 115 isolated installs, 113 shared installs
and 2 exact-byte deduplications. Exact scheme-21 acceptance is namespace-
constrained, supported by synthetic regressions and this observed subset; the
accepted packages still carry substantial explicitly unknown metadata. Legacy
ZIP filename decoding and schemes 12-14 are evidenced. The next priorities are
(1) remaining scheme-by-scheme parser support with loss accounting and
(2) deeper typed coverage of parameter/dynamic/load-procedure semantics.
That sequence maximizes usable products without pretending that successful
catalogue installation is full ETS or commissioning compatibility.
