# AR06 import boundary contract

Status: research and behavioral verification in progress; no new compatibility
acceptance. This document distinguishes a project XML namespace from a trailing
number, and project evidence from standalone product-package evidence.

## Primary evidence read before changes

The user-provided local source is `knx-spec-kb/sources/The KNX Standard v3.0.0/`.
Only reference the licensed PDFs; do not commit their bytes or extracted text.

- `03 Volume 3 System Specifications/03_08_10 XML Data Encoding v01.01.01 AS.pdf`,
  printed page 2/2, version 01.01.01, saved 2021-10-08: points to the ETS Help
  Centre's project-schema description. It is a two-page pointer, not the project
  schema or a definition of legacy VD/PR containers.
- `Project Schema23 v01.00.00.pdf`, dated 2024-03-01, printed pages 4–6/64:
  section 1.1 bounds the document to project XML and excludes manufacturer
  configuration/download semantics; section 1.5 gives the exact target namespace
  `http://knx.org/xml/project/23`; section 1.1.1 identifies `KNX` as the XML root
  and `CreatedBy`/`ToolVersion` as optional source strings. Section 2 locates the
  XSD in the Manufacturer Tool/member toolchain, not in this PDF.
  Page numbering was checked against the PDF's page labels and text extraction.
  Automatic whole-document extraction refused an OCR-needed page; bounded
  `pdftotext -layout -f 1 -l 8` extracted the relevant text successfully.
- `02 Volume 2 Cookbook/02_06_01 ETS App Development v01.01.02.pdf` describes
  ETS4 SDK/add-in development, not `.knxproj`/`.knxprod`/legacy import grammar.

This is a dated local primary document, not a claim that it is the latest schema
publication or that an authoritative XSD is available to this application.
Existing schema-11/21 reference measurements remain separate evidence; no
schema-23 rule is silently copied into unsupported project versions.

## Initial source findings

At discovery, `crates/knx-etsproj/src/detect.rs` selected a numeric schema version from
any default namespace ending in an integer. It did not verify the namespace's
KNX identity or root element name. The pipeline then chooses its existing known
element table from that integer. A foreign namespace must not acquire known KNX
semantics merely by ending in `/11`, `/21` or `/23`.

The installed project known tables are only 11, 21 and 23. Readable standalone
product schemes 11/12/13/14/20/21 do not enable project imports at those versions.
Canonical but unsupported project namespaces must keep an explicit
`NoKnownSchemaTable` failure, not be inferred from a producer name or product DB.

## Implemented root boundary — 2026-10-02 08:51 CEST

Detection now requires a `KNX` root QName with its actual prefix/default
namespace bound to the exact canonical project namespace. A foreign prefix
cannot borrow a canonical default namespace. Zero-padded, signed, foreign,
HTTPS-substituted and extra-segment lookalikes are refused rather than rewritten.
Known project tables remain exactly 11/21/23; canonical unsupported versions
still fail with `NoKnownSchemaTable`.

`Project.xml` must declare the same canonical root namespace as `0.xml` before
its metadata is parsed with that table. Root attributes are parsed from the
original bytes, checked for malformed/duplicate declarations, and normalized
as XML values, including character/entity references. Invalid encoding cannot
be silently replaced. `CreatedBy` and `ToolVersion` remain optional, untyped
source strings: XML decoding does not turn them into tested ETS capabilities.
This is semantic XML-value retention, not byte-for-byte retention of a consumed
root attribute's spelling or a new raw-source archive guarantee.

Named detection errors distinguish a missing binding, noncanonical namespace,
unexpected root, malformed root and topology/metadata namespace mismatch.
Full-pipeline synthetic regressions cover negative cases and a genuinely bound
KNX prefix with all elements qualified. The application-service regression
seeds both SQLite databases, then verifies six refusals (three namespace cases
with and without the shared product DB). Project/product main files and WALs
remain byte-identical; diagnostic variants, archive entries and values are
checked exactly. This does not certify native-bundle reopen or session/history
rollback.

Focused restored-source gates: `knx-etsproj`, `knx-app`, `knx-testsupport` —
155 passed, zero failed, 75 ignored, 31 result blocks; strict all-target Clippy,
formatting and whitespace passed. Ignored private tests did not execute and are
not corpus acceptance. Compiled metadata-guard-removal and raw-attribute-value
mutants each failed their targeted assertion (exit 101); production sources
were restored byte-exactly before the final green run.

At that root-only checkpoint, descendant namespace resolution, master metadata
`.ok()` suppression, untyped master/version lexemes, native reopen, legacy
refusal and DefaultLine/device-local cases remained pending. The master follow-up
below closes only its specifically tested diagnostic/admission/native boundary;
retained bytes are never proof of interpreted fields. No whole AR06 checklist
is closed by either checkpoint.

## Reserved-binding review follow-up — 2026-10-02 09:35 CEST

The public W3C Namespaces in XML 1.0 Recommendation (Third Edition), sections
2.3/3, is the primary reference for namespace-name comparison and reserved
`xml`/`xmlns` bindings: `https://www.w3.org/TR/REC-xml-names/`.
Root declarations are checked on normalized XML values, including unused
declarations; reserved namespace rebinding and non-default empty bindings are
named malformed-root failures. Seven synthetic negatives and escaped-valid
reserved URI/allowed `xmlVendor` positives are covered.

The first reserved-binding regression compiled and failed behaviorally (101).
An attempted `NsReader` substitution passed that new regression but failed the
existing escaped-namespace positive (16 passed / one failed): the resolved URI
was still its escaped spelling. It was not accepted as a green candidate.
The implementation keeps original-byte `Reader` plus explicit root-only
normalized binding checks, rather than trusting raw resolver equality.
Renewed three-crate tests/lint pass: 156 passed / zero failed / 75 ignored /
31 result blocks. No complete lexical QName/XML grammar validation or
descendant namespace conformance is claimed.

## Wider root-candidate receipt — 2026-10-02 10:04 CEST

After generating the real frontend build required by desktop compilation,
renewed proc_c5ad6d2638d7 exited 0. All nine explicit gate steps passed:
formatting, whitespace, workspace tests, strict workspace all-target Clippy,
workspace build, headers, anchors, layering and corpus source lint. Actual
workspace results: 2,892 passed / zero failed / 163 ignored / 146 result blocks.
The five owned Rust source files remained unchanged throughout this run;
tracked Web source/binding diff remains empty. Repository gates selected this
exact checkout and reported nonempty coverage.

The first wider run failed before workspace test results because the fresh
checkout lacked frontend build assets; that failure remains recorded separately.
Ignored fixtures and source lint do not establish corpus compatibility. Remaining
master diagnostics, legacy refusal, mapping, native/sample and full AR06
acceptance requirements are not closed by this receipt.

## Master metadata and admission follow-up — 2026-10-02 10:42 CEST

The optional master comparison no longer suppresses failures with `.ok()`.
`Detected::master_metadata_error` and `ImportOutcome::master_metadata_error`
carry the typed adapter finding through the import pipeline. No domain/storage
migration or `SourceInfo`/`ImportError` wire-shape change is involved.

| Observation | Detection/report | Application disposition |
| --- | --- | --- |
| Master absent | No comparison and no unreadable-master finding | No master ingest |
| Readable canonical root | Actual schema comparison; disagreement remains named | Existing typed ingest remains enabled; not an XSD/whole-master compatibility guarantee |
| Present unsupported/unreadable root metadata | Named `unsupported` entry, comparison unavailable, no source-value echo | Keep whole master in project opaque store; do not ingest its contents into shared typed master tables |
| Master container read/CRC failure | Fatal named container error | No successful import or byte-retention claim |

The report uses fixed boundary labels for missing namespace, unparsable schema
namespace, foreign namespace, unexpected root and malformed root attributes or
encoding. It does not print raw namespace/producer/entity values or parser
error strings. `namespace_disagreement: null` alone does not establish agreement;
the explicit unsupported finding carries the unavailable comparison.

The first compiled report regression failed with an empty unsupported list.
Cross-layer review then reproduced a second defect: despite the warning, the
shared product path inserted a synthetic foreign master's DPT row. A typed
application guard now prevents that write instead of parsing report prose.
The same native test covers both product-DB modes, exact report content, zero
foreign-master DPT rows, semantic project equality after closing/reopening a
real `.knxdb`, equality of all opaque entries, original master bytes/hash and
unchanged source archive. A positive canonical-root control verifies the exact
typed DPT row so skipping all masters cannot masquerade as correctness.

Synthetic library tests cover eight unreadable-root variants, a foreign root,
absent/agreed/disagreed masters and a deliberately CRC-corrupted master. These
are not independent ETS samples. Native source-byte retention does not persist
the transient import report: report availability after native reopen remains
its existing separate contract. Producer/version spellings retained in the
master blob are still untyped; Secure capacities, MinEtsVersion and
ReplacesVersions do not become tested capabilities.

Three compiled detection/report/application-guard mutants failed their intended
behavioral assertions (101), not compilation; all production files restored
byte-exactly. Final three-crate gates: 162 passed / zero failed / 75 ignored /
31 result blocks, strict all-target Clippy, format and whitespace pass.
The earlier wider root receipt does not certify this changed candidate. Renewed
workspace gates are recorded below; private corpus/sample matrix, descendant
namespace audit, legacy refusal and DefaultLine/device-local evidence remain
pending. Direct
ProductDB master ingestion and standalone package admission are separate paths;
this application guard does not claim to harden those APIs.

## Changed master candidate wider receipt — 2026-10-02 10:54 CEST

proc_7228729035d2 completed with exit 0. All nine expected steps and workspace
log counts were independently reconciled: 2,898 passed / zero failed / 163
ignored / 146 result blocks. Workspace strict all-target Clippy/build,
format/whitespace and the four explicitly selected, nonempty repository gates
pass. Seven owned Rust sources stayed unchanged throughout the run; production
files still equal their restored mutation baselines. Tracked Web source/binding
delta remains empty. Source lint and ignored fixtures do not prove private
corpus compatibility. Remaining AR06 requirements and independent sample
blockers stay open; no whole-feature acceptance or publication is claimed.

## Legacy filename admission and CLI master refusal — 2026-10-02 13:02 CEST

Known `.vd3`–`.vd5` and `.pr3`–`.pr5` filename extensions now produce a
typed unsupported-legacy failure instead of reaching modern ZIP/encryption
diagnostics. Matching is ASCII-case-insensitive, while the diagnostic preserves
the supplied extension's spelling. `.vd2` remains refused; its direct ProductDB
API retains the existing hash/length error. The CLI now refuses all these named
extensions before reading their contents or opening/migrating destinations, so
its VD2 message is the same payload-free filename refusal, not a content hash.

This deliberately classifies filenames, **not** legacy container contents.
Readable modern bytes under a legacy filename are also refused, including a
known package-hash retry. A renamed legacy file may still reach existing generic
container/encryption failures. No EX-IM parsing, decryption, passwords, migration,
vendor execution, automatic conversion or legacy compatibility is introduced.
The independent format adapters own their typed errors; no new dependency or
format rule is placed in the KNX domain core.

Application coverage exercises every named extension and mixed-case controls
with/without the shared product DB, asserting exact typed propagation, unchanged
input bytes and byte-identical seeded SQLite main files/WALs. CLI subprocess
tests assert exit 1, no success output/report, no newly created destinations and
unchanged existing native/product/report files. Closing/reopening the native
file verifies complete project equality and all opaque entries; the installed
product catalogue and verification remain intact. Direct ProductDB coverage
preserves the entire seeded database, including its installation reports.
Unrelated suffixes and a readable modern project remain positive controls.

2026-10-02 15:52 CEST evidence review: the application fixture initially
persisted opaque/manufacturer data but not the returned domain Project. The
new load-equality precondition failed with `NotSaved`; the fixture now explicitly
saves a nonempty seed before snapshots and checks whole-project equality after
each legacy/namespace refusal. A compiled pre-refusal project-info mutation
fails the byte-preservation assertion. Restored service tests: four passed,
zero failed, three ignored; strict app Clippy/format/whitespace pass. This
strengthens the witness, not the service's save responsibilities or schema.
The prior eighteen-step gate also passed on the preceding fixture tree;
final strengthened-fixture candidate proc_0c225d9b5f0e subsequently exited 0:
all eighteen steps/logs reconciled at 16:14 CEST, 2,910 workspace passes,
zero failures, 164 ignored, eighteen selected private and six raw tests without
skips; fifteen protected source paths frozen. Integrated-current-upstream
acceptance and publication remain pending; candidate evidence is not ETS parity.

Caller review reproduced a separate master bypass: `products ingest` returned
success and ingested manufacturer/master rows despite the adapter's unsupported
master-root finding. This products-only command cannot preserve project opaque
members, so it now refuses that typed finding before any manufacturer write.
The separate 2026-10-02 15:17 CEST caller review found that this still opened
the target database too early. Project parsing and master admission now precede
opening/migrating the product DB. The regression covers a missing target and an
existing synthetic non-database sentinel as well as the seeded valid database:
no fresh database, unchanged sentinel, no WAL/SHM sidecars, fixed payload-free
source refusal, and no success output. A compiled premature-open mutant fails
the no-creation assertion; source was restored byte-exactly. All eight ordinary
CLI integration tests and all eight selected private offline CLI tests pass
without skips; strict CLI Clippy, format and whitespace pass. This does not
certify every standalone-package failure path or process-crash atomicity.
Foreign-namespace and unexpected-root fixtures assert a fixed, payload-free
diagnostic, no new hardware/DPT rows and unchanged existing database bytes.
The application's retain-opaque/skip-typed-master behavior remains different and
unchanged. Neither guard certifies the generic ProductDB XML APIs or all package
root/version semantics.

Compiled REDs established the missing filename admission, premature CLI
destination creation and silent CLI master acceptance. Initial test compilation
errors are not credited as REDs. The first native seed incorrectly reused the
intentional duplicate-ID fixture; the oracle was corrected to unique IDs with
zero import errors, without weakening full project equality or the existing
exit-2 regression. Five compiled behavioral mutants were killed: project bytes,
package filename admission, each CLI preflight and conditional CLI master bypass.
All three production sources were restored byte-exactly.

Final focused restored-source gates: five crates, 899 passed / zero failed /
119 ignored / 70 result blocks; strict all-target Clippy, format and whitespace
pass. Ignored tests did not execute. No tracked Web/binding delta, private corpus
acceptance, renewed workspace acceptance or publication is inferred. Remaining
sample/raw-field/DefaultLine/device-local requirements keep AR06 IN_PROGRESS.
Evidence: `ar06-legacy-restored-summary.json`,
`ar06-legacy-mutations-summary.json` and their named scratch logs.

## Changed legacy candidate wider receipt — 2026-10-02 13:16 CEST

proc_11bf8a0ab11f completed with registry exit 0. The twelve expected steps,
each log's test results and nonempty intended-root repository scope were
independently reconciled: 2,904 workspace passes, zero failures, 163 ignored,
146 result blocks; strict workspace all-target Clippy/build and format/whitespace
passed. Repository scope was 380 valid headers, 376 links across 233 Markdown
files, 448 resolved packages and 332 Rust files in corpus source lint.
Thirteen named Rust sources stayed frozen and the tracked Web/binding delta
remained empty.

Ten additional opt-in offline private cases actually ran without skips: eight
existing CLI import cases, one readable-product installation case and one real
VD2 refusal case. This is deliberately not the whole private corpus matrix or
independent installation/schema evidence; the 163 ordinary ignored cases remain
unexecuted by the workspace command. Original inputs were accessed read-only via
this checkout's owned ignored link. No private identities/payloads, new password
access, bus action or corpus pin change is part of this receipt.

Artifacts: `ar06-legacy-wide-summary.json`, its twelve named logs and aggregate
`ar06-legacy-wide-verified.txt`. This changed-candidate receipt supersedes earlier
workspace counts only for its named scope. Raw-field/sample and
DefaultLine/device-local evidence, whole-feature review, integration and
publication remain pending; all comprehensive AR06 rows stay open.

## Mapping/native receipt and existing sample matrix — 2026-10-02 14:34 CEST

Renewed proc_2296901dfdb3 exited 0. Six compiled behavioral negative controls
were caught: suppressed empty references, guessed first line, missing legacy,
modern or space DefaultLine wiring, and zeroed device-local object numbers.
Each failed its named assertion, not compilation. Mapper restoration equals
the owned HEAD byte-for-byte; four fixture/test files stayed frozen. The eight
step logs were independently reconciled: three-crate tests 170 passed, zero
failed, 76 ignored, 31 result blocks; strict Clippy, format and whitespace pass.
The first attempt's Clippy failure remains archived, not relabelled as green.

Synthetic DefaultLine tests cover absent, valid, empty, dangling and mixed
installation/space references at project tables 11/21/23. A valid reference
selects the stated second line, not a convenient first-line fallback; invalid
tokens produce exact map stage, severity, path, detail and count. Device-local
tests distinguish the owners, instance flag/text layers and send-group links
for repeated RefIds; malformed/overflow references keep an explicit error
placeholder and do not lose later objects. These extend tests of existing
mappers, not their supported schema set or the domain model.

The native test exercises 24 combinations (three versions, four line states,
two product-DB modes), closes/reopens real SQLite stores and compares the entire
model plus every opaque row, synthetic note bytes/hash and unchanged input.
The optional product DB in this mapping fixture is empty; this alone is not
populated-catalogue enrichment evidence. Eight separate, explicitly requested
private offline tests passed without skips: four object/DefaultLine cases, one
sender crosscheck, two enrichment cases and one schema-11 golden case.

| Evidence class | Actual sample relation | Scope, not a compatibility certificate |
| --- | --- | --- |
| Project schema 11 | One existing installation, also exported at schema 23 | Selected real golden/mapping cases plus synthetic DefaultLine/native controls |
| Project schema 21 | One different existing demo installation | Exact empty DefaultLine diagnostic and existing object/enrichment controls; not a second module-using sample |
| Project schema 23 | Reexport of the schema-11 installation, not a third independent installation | Existing local-object/enrichment/sender crosscheck; module inference and reported unknown attributes remain |
| Other canonical project versions | No installed project table outside 11/21/23 | Named NoKnownSchemaTable refusal; standalone product evidence cannot enable a project version |
| Standalone product schemes 11/12/13/14/20/21 | Separate manufacturer-package evidence in AR05 and KNOWN_LIMITATIONS §11 | Earlier selected readable-package test is not this pass's full corpus-matrix execution, project evidence or executable manufacturer semantics |
| VD2 and VD3–VD5/PR3–PR5 | Earlier selected real VD2 refusal plus synthetic named-file controls | Filename refusal only; no parser, decryption, renamed-content detector or legacy compatibility |
| Native .knxdb | Synthetic stores, not independent ETS exports | Tested semantic/opaque reopen; does not persist the transient report or regenerate the original ETS archive |

This is three existing project exports from two documented installations, not
three independent installations. A genuinely independent module-using schema-23
sample and first samples for unimplemented project versions remain
BLOCKED_EXTERNAL. See KNOWN_LIMITATIONS §1/§125. No original source or corpus
pin was modified. Artifacts: ar06-mapping-checks-summary.json, its named logs,
ar06-mapping-verified.txt and ar06-mapping-native.log. Whole-feature review,
renewed workspace/integrated gates and publication remain pending.

## Raw-field boundary audit — 2026-10-02 14:34 CEST

Traced project attribute decoding/retention, optional producer extraction,
product source blobs and catalogue metadata rather than inferring guarantees
from a successful import. Existing catalogue/producer regressions were rerun:
five catalogue tests and one exact producer test passed, zero failures/skips.
These are six existing tests, not newly implemented raw-field functionality.

| Boundary | What survives or fails | Explicit limit |
| --- | --- | --- |
| Unreadable project root or visited modelled attribute | Named XML/detection failure rather than lossy replacement; no successful normalized import | Refusal is not preservation inside a newly created native file; retain the unchanged original input. Opaque skipped subtrees are a separate byte-retention boundary, not a complete XML-validation claim |
| Readable unmodelled project attribute | Retained attribute/fragment and unknown diagnostic | Normalized XML value is not every original entity/quote spelling; no whole-source archive guarantee |
| Project root CreatedBy/ToolVersion | Optional XML-normalized detection/source-report strings | Not ordered/tested ETS versions; AR06 does not add durable root producer fields or transient-report persistence on native reopen |
| Unsupported/unreadable optional master root | Application preserves the whole master blob, reports unavailable comparison and skips typed master ingestion | Products-only CLI refuses instead; direct ProductDB XML APIs have a different boundary |
| Product source XML and raw Secure/MinEtsVersion/ReplacesVersions fields | Whole source_file blob plus nullable source-value catalogue columns, including empty values | Capacities/version labels are not tested Secure/ETS abilities; a family-list derivation is not a compatibility or execution guarantee |
| Optional ProductDB producer extraction | Unprefixed KNX root attributes may supply source_producer facts | No facts row for non-XML/non-KNX/unreadable extraction does not prove the source had no producer; original blob is the evidence |
| Invalid but readable DefaultLine token | Exact unresolved-reference report; normalized LineId remains absent | The transient diagnostic is not persisted by this work; native reopen is not a promise to reconstruct the consumed invalid token |

The audited implementation points are knx-etsproj parse/mod.rs::attr_map and
the two installation readers, knx-productdb xml.rs::attrs,
parse/program.rs catalogue columns and identity.rs producer/family extraction.
Regressions: catalog_metadata.rs and package_identity.rs's exact producer test.
This audit does not close descendant namespace conformance, generic XML API
admission, full historical DPT winner provenance or executable manufacturer
semantics. Those remain explicit boundaries rather than implied abilities.

## Verification scope

- First reproduce and close foreign/lookalike root-namespace admission with a
  synthetic full-pipeline regression. Match namespace identity exactly; do not
  rewrite the input URI or identifiers to make them fit.
- Audit actual root QName, the metadata document's namespace, malformed root
  attributes and producer-string retention separately before claiming complete
  namespace validation. Descendant namespace handling is not certified by a
  root-only guard.
- Verify application refusal atomicity and native persistence independently.
  Library-level refusal alone does not prove a product DB/native-file transaction.
- Legacy VD3–VD5/PR3–PR5 work is named refusal only, not an importer, embedded
  password, EX-IM grammar or vendor-code execution. VD2 remains refused.
- DefaultLine and device-local object mapping now have the scoped synthetic,
  native and selected private evidence above, not whole-feature acceptance.
  Independent missing project samples remain `BLOCKED_EXTERNAL`; a reexport of
  an installation is not an independent installation, and a synthetic sample is
  not real ETS acceptance.

No domain/storage contract, dependency, UI binding or hardware authority changes.
No XSD validation, ETS export/parity, manufacturer semantics or new real schema
compatibility is claimed.
