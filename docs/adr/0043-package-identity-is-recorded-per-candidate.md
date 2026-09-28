# ADR 0043: Package identity is recorded per candidate, never decided by a new winner rule

Date: 2026-09-28
Status: Accepted
Session: 4 (manufacturer databases), goal item PDB-11
Amends: ADR-0011 (product database storage), ADR-0037 (catalogue metadata) for
`ReplacesVersions`

## Context

Goal item PDB-11 asks the product database to distinguish byte-identical
packages, the same logical id arriving with different bytes, product
families, `ReplacesVersions`, the same order number on a different
program or scheme, and deterministic winner/loser sources. It asks that
content hashes, not filenames, own exact deduplication.

What the code did at schema v16 (verified by reading source and tests):

- A package is keyed by the SHA-256 of its archive bytes (`package.sha256`)
  and every member by the SHA-256 of its bytes (`source_file.sha256`).
  A byte-identical package is revalidated and reported `skipped`. Only the
  first source name is kept; a second name under which the same bytes arrive
  is dropped silently.
- For `catalog_section`, `catalog_item`, `hardware`, `product`,
  `hardware2program` and `application_program`, `first_winner` keeps the
  first installed row per id. A later element with the same id is not
  stored. An `IdConflict` names the kept and the other member blob, but not
  whether the two elements actually differ.
- `ReplacesVersions` is a raw string (ADR-0037). Nothing groups program
  versions, nothing groups products by order number, and `KNX/@CreatedBy` /
  `@ToolVersion` are stored nowhere.

What was measured (read-only, aggregate-only probes of the private corpus on
2026-09-28; 117 package instances, 115 unique; details in
`docs/PRODUCT_DATABASE_CORPUS.md`):

- Every id that appears in more than one unique package appears in member
  files whose bytes differ, so member-hash equality says nothing about the
  element.
- Among those ids, whether the element itself is the same depends on the
  kind: for application programs, 2 of 29 have identical element bytes, but
  19 of 29 have identical *attributes*. Comparing attributes alone would
  call 17 differing programs equal. For hardware 18 of 68 are byte-equal,
  for products 23 of 69, for catalogue sections 1 of 53 (30 share
  attributes only). All 33 `Hardware2Program` and 16 of 17 catalogue items
  are identical.
- In shared install order by package hash, 398 losing elements exist; for
  about a hundred ids, reversing the order would change the stored
  attributes. The stored catalogue is therefore order dependent.
- `ApplicationNumber` equals the hex `A-NNNN` part of the program id in 275
  of 275 programs, and `ApplicationVersion` the `VV` part in 275 of 275.
  Grouping by (manufacturer, `ApplicationNumber`) gives families of size 1
  (183), 2 (40) and 3 (4). No (manufacturer, number, version) has more than
  one program id.
- `ReplacesVersions` is absent on 145 distinct programs, one integer on 55
  and a whitespace-separated integer list on 75. No other lexical form
  occurs. 289 listed versions; 16 name a program that is in the corpus.
- 73 of 304 order numbers appear in more than one package; 44 of those carry
  different program sets there, 18 different schemes. 4 order numbers sit on
  more than one product id.
- `CreatedBy`/`ToolVersion` sit on the root of manufacturer members (never on
  `knx_master.xml`). 4 distinct `CreatedBy` and 25 distinct `ToolVersion`
  values; the namespace does not determine the tool version.

What is specified: KNX Association does not publish its XSDs (RESEARCH §2).
An unofficial public copy of the project/11 schema (`KNXSchema.xsd`,
<https://gist.github.com/7f5c7f80080d92c02933a02d60ea9ab5>) declares
`ApplicationProgram/@ApplicationNumber` as `xs:unsignedShort`,
`@ApplicationVersion` as `xs:unsignedByte`, `@ReplacesVersions` as
`xs:list` of `xs:unsignedByte`, and `KNX/@CreatedBy`/`@ToolVersion` as
optional `xs:string`. All three program attributes are annotated
"registration-relevant". This is secondary evidence; the corpus agrees with
it without exception. Nothing available specifies what a tool must do with
`ReplacesVersions`.

## Decision

### 1. Content hashes keep owning exact deduplication

Unchanged: package and member identity are SHA-256 of bytes. New: every
source name a package arrives under is recorded (`package_source_name`),
including on the `skipped` retry path. That is the only write the retry path
makes. A filename is never used to decide identity.

### 2. The winner rule stays first-installed; every candidate is recorded

No new winner rule. Candidates considered and rejected:

- *Highest `ToolVersion`*: a free-form producer string; no ordering is
  specified, and a newer tool does not make the content more correct.
- *Newest schema namespace*: a format version, not a content version.
- *Lexicographic member hash*: deterministic but arbitrary, and it would
  silently change which values existing databases show.

Instead, the order dependence becomes visible. For every element of the six
identity-bearing kinds in every parsed member blob, a
`source_identity` row records `(source_sha256, table_name, logical_id,
occurrence, digest)`. The rows are a pure function of the blob bytes, so the
recorded set does not depend on install order, even though the winner does.
A query names the winner (the typed row's `source_sha256`), every candidate,
the packages that contain each candidate's blob, and whether each candidate
is identical to the winner.

"Deterministic winner/loser sources" is therefore met as: the same install
sequence always yields the same winner, and every winner and loser is named
by source blob and package. Choosing a winner by rule stays open until a
normative ordering exists (Consequences).

### 3. What "identical" means: the element digest

The digest is SHA-256 over a length-prefixed canonical event stream of the
element, from its start tag to its end tag:

- start and end tags by the qualified name as written;
- attributes sorted by qualified name, with values normalized as every
  parser in this crate reads them (`normalized_value`). Namespace
  declarations count as attributes;
- character content with entity and character references resolved, CDATA
  taken literally and line endings normalized to `\n`. Adjacent pieces are
  coalesced, and a run that is only XML whitespace is dropped. Other text is
  kept exactly;
- comments, processing instructions and the XML declaration are ignored;
- a nested element of another tracked kind (for example `Product` inside
  `Hardware`, `CatalogItem` inside `CatalogSection`) is not hashed into its
  parent. A marker naming its kind and id takes its place, so the parent
  still changes when its set or order of children changes.

Every digest starts with the context the parser stores for the element
from outside it: `Manufacturer/@RefId`, and the parent id where a column
holds one (the enclosing `CatalogSection` for sections and items, the
last `Hardware` with an `Id` for products and `Hardware2Program`). The same
element bytes under another parent therefore report as different, because
the stored row would differ.

Equal digests mean equal as the parsers read it. Different digests mean
something differs, possibly something without meaning (a different
namespace prefix, for example). Two elements count as equal only where
the parsers cannot tell them apart either: whitespace-only text runs are
dropped, and names are compared as written without resolving namespaces.
Beyond the stored context, the comparison covers the element's subtree
only: a program's `Languages` translations are outside the
`ApplicationProgram` element and are not covered.

The whole definition (dispatch, context and canonical form) is versioned
as `IDENTITY_SCANNER`, stored with every scan. A row from another scanner
version fails closed; changing the definition requires a bump and a
rescanning migration.

### 4. The candidate scan is checked against the parsers on every ingest

A separate streaming scan records candidates, because the domain parsers skip
subtrees (`Dynamic`) that the digest must cover. It mirrors the parsers'
dispatch exactly: the same element names, `Hardware` only with an `Id`, and
in program files the `Dynamic` skip and the parameter-type child that is
never dispatched. After each parse of a blob, the ingest checks the scan
against what the parser did: every typed row the blob won and every
`IdConflict` it produced must have its candidate row. On the pass that
parses a blob for the first time, the parser's decisions are complete, so
the check is exact: the candidates with an id must be exactly those rows
and conflicts, nothing more. A mismatch fails the ingest, and the
transaction rolls back. A scan that cannot read the blob
records `unavailable` with its reason in `source_identity_scan` and never
fails the ingest. The same helper serves the migration.

### 5. Producer facts are stored per blob

`source_producer(source_sha256, root_namespace, created_by, tool_version)` is
written for every stored blob whose root element's local name is `KNX`. The
facts are stored as source strings, never compared or ordered, and are not
merged with `MinEtsVersion` or the package scheme: those stay separate facts.
Existing unknown-attribute reporting is unchanged.

### 6. Families, `ReplacesVersions` and order numbers are derived at query time

No stored graph. From the winning `application_program` rows:

- **Family**: programs with the same `manufacturer_id` and the same
  `ApplicationNumber` value, read as `xs:unsignedShort` lexical form
  (collapsed whitespace, optional `+`, digits, at most 65535). A program
  whose number does not parse has no family, and the reason is shown.
- **`ReplacesVersions`**: an `xs:list` of `xs:unsignedByte`, split on XML
  whitespace. If every token parses, each listed version resolves to the
  family members whose `ApplicationVersion` has that value. A version may
  resolve to several members or to none. None means *not installed*, never
  *does not exist*. If any token fails to parse, the raw string is shown and
  nothing is linked. The raw string stays stored and shown (ADR-0037
  continues to hold).
- **Order number**: an exact-string lookup per manufacturer that lists every
  winning product with that order number, its hardware, its programs via
  `Hardware2Program`, and the schemes of the packages containing its source
  blob. An order number is never an identity and never merges products.

### 7. Migration v16 → v17

Creates `package_source_name`, `source_identity`, `source_identity_scan` and
`source_producer`. The backfill:

- seeds `package_source_name` from `package.source_name`;
- scans every blob in `source_parse_evidence` that classifies as `Catalog`,
  `Hardware` or `ApplicationProgram`. These are the blobs the parsers
  actually read; a raw-stored member never becomes a candidate;
- extracts producer facts from every stored blob.

The backfill also runs the agreement check against the historical rows.
A per-blob scan failure, or historical rows that disagree with the scan,
is recorded as `unavailable` with its reason and the upgrade continues, so
a later re-parse of that blob cannot fail on rows an earlier build wrote.
The six identity tables get an index on `source_sha256` for the check. The original install order is not recoverable, so historical
winners and `package_conflict` rows stay exactly as they were.

## Consequences

- A reader can answer "which package supplied this value, which others offer
  a different one, and how do they differ?". Before, the answer was "the
  first one; the others are gone".
- The stored catalogue is still order dependent, and the documentation says
  so with measured numbers. Lifted when a normative ordering of package
  content exists, or the user can pick a winner per id (a later UI slice).
- Families and `ReplacesVersions` links rest on an unofficial schema copy
  plus corpus agreement. A value that does not fit is shown raw and never
  guessed. KNXBench does not implement any upgrade behaviour behind
  `ReplacesVersions`.
- The digest is conservative: syntactic differences without meaning can
  still report "differs". It never reports two different elements as the
  same.
- Server and web do not show identity, family or order-number views in this
  slice; library and CLI do.
- `manufacturer.name` (last `knx_master.xml` wins, KNOWN_LIMITATIONS §88)
  and `datapoint_type` (first wins without provenance, §86) are master data,
  not one of the six package-content kinds. This slice does not change them.
