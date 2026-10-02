# Manufacturer report omissions — AR05 design and evidence

Status: implemented and complete corrected-candidate acceptance passed;
publication/readback is pending. Base:
`f4b845a3880f4380e66c9d94a218a6dfff16e724`; product database v18 -> v19.

## Baseline facts before implementation (v18)

The source owns separate paths:

| Input/field | Owning code | Current behavior |
| --- | --- | --- |
| package master member | `package.rs` → `parse/master.rs` | stores original bytes, parses master rows, persists unknowns and install facts |
| generic standalone MasterData | `ingest.rs` | opaque bytes only; do not silently turn this into typed ingest |
| project/CLI master | dedicated `ingest_master_data` calls | shared master parsing; callers own persistence/transaction policy |
| Language Identifier | `parse/translation.rs` | consumed into translation key |
| TranslationElement RefId | `parse/translation.rs` | consumed into translation key |
| Translation AttributeName/Text | `parse/translation.rs` | consumed into translation row |
| Languages wrapper attributes, TranslationUnit RefId/Version and extras | no reporting owner in v18 master path | retained original bytes, not represented as master unknown attributes |
| product/hardware/program duplicate IDs | `first_winner` plus identity scan | existing first-winner/conflict/candidate behavior; preserve it |
| master DPT/function/space declarations | `parse/master.rs` | retained bytes, ignored row collisions; no stored winner provenance |
| skipped Dynamic references | `dynamic/evaluate.rs` | non-activating reports under shared activation budget; explicit budget diagnostic already exists |
| historical package install facts | `package.rs`, v12 ledger | original install-time evidence, not a current-parser report |

The absence of a stored column or a consumer must not be disguised by adding
its attribute to a "known" list. In particular, seeing Version in a corpus is
not interpreting its semantics. Report such metadata as retained/uninterpreted;
keep original source bytes. Counts for actual writes are historical and cannot
be reconstructed by parsing today (ADR-0020).

## Primary-source research (checked 2026-10-02)

KNX Association's Project Schema23 description, §2 and §4.2, distinguishes
KNX-administered master definitions from product/project files, and places the
XSD in Manufacturer Tool installations/member-access updates. It provides no
TranslationUnit attribute table in this PDF. This is not evidence for an invented
Version interpretation. Sources:

- `https://support.knx.org/hc/en-us/articles/4408207190674-Project-schema-description`
- `https://support.knx.org/hc/en-us/article_attachments/17389755651474`
- `https://support.knx.org/hc/en-us/articles/4407072790930-ETS6-Master-data-file-is-missing-is-corrupt`

The last source links an official project-23 master download. Availability of
an XML example is not a normative definition of every field or permission to
embed manufacturer/master data in application code. AR05 uses existing authorized
corpus observations and synthetic fixtures, not an additional compatibility claim.

**Important separate compatibility finding:** the first support page, updated
2026-08-27, states that LocationUsage references must be interpreted according to
ToolVersion: through 6.3.8272.0 they refer to internal master data; later versions
use ontology references. This is an AR06 audit input, not a room-type feature
or ETS 6.4 compatibility result delivered by AR05.

SQLite's official foreign-key documentation, §4.2, separately confirms the
commit-time boundary reproduced by the deferred-constraint regression: releasing
a top-level transaction savepoint can fail, while nested release does not own
the caller's eventual commit. Checked 2026-10-02:
`https://www.sqlite.org/foreignkeys.html`.

## Acceptance matrix (baseline plan; execution evidence below)

1. Aggregate-only bounded corpus observation by full ancestor path and expanded
   namespace, separating instances, unique packages and unique master blobs.
2. Behavioral master/package tests for all five Languages element families,
   consumed versus uninterpreted attributes, unknown extras, repeated keys,
   qualified/lookalike namespaces, original bytes and retry persistence.
3. Re-derivation through the same parse-layer evidence logic, never a duplicate
   grammar hidden in a migration; per-blob rollback/error evidence, no mutation
   of historical install snapshots or pre-ledger unavailable markers.
4. Audit master declaration duplicates and missing-ID/parent paths separately
   from already-fixed normalized product conflicts. Do not guess historical
   winners from an arbitrary retained-blob order.
5. Exercise actual Dynamic truncation/budget boundaries; keep omitted conditional
   branches and unexpanded module definitions explicit, without activating them.
6. Exact malformed/late-write/upgrades, focused and full-crate tests, mutation
   probes, explicit private corpus, separate whole-diff review, coordinated full
   gates and remote artifact readback before declaring any inventory row DONE.

The implementation uses a data-only v18 -> v19 migration under ADR-0020.
There is no API-binding regeneration, typed master-language metadata, Dynamic
semantic expansion or new scheme/ETS compatibility claim.

## Implemented contract and verified boundaries

- `parse/master_language.rs` is a pure bounded pass over retained bytes. Only
  exact supported root namespaces and canonical ancestor paths exempt fields
  consumed by translation storage: Language Identifier, TranslationElement
  RefId and Translation AttributeName/Text. TranslationUnit RefId/Version and
  other unconsumed attributes remain named unknowns. Qualified attributes keep
  their expanded namespace keys; opaque branches outside Languages are not
  assigned this scanner's evidence budget or semantic interpretation.
- Current master ingest and v18 -> v19 use the same parse-layer pass. The public
  `rederive_master_language_evidence` operation changes only current unknown
  evidence, not normalized master entities, retained bytes or historical
  installation counts/snapshots. Repeated derivation replaces exact keys rather
  than adding guessed historical totals. A pre-ledger report remains unavailable.
- Malformed/hash-invalid/owned-missing sources have named
  `MasterLanguageEvidenceError` evidence. Oversized unclassified sources have
  `MasterLanguageEvidenceUnexaminedSource`, never an inferred clean result.
  Report counters distinguish failed and unexamined sources from derived keys.
  SQL write **and final savepoint-release** failures roll back the owned work;
  nested execution leaves the caller's outer transaction under caller control.
  Reinspection retires only this pass's exact source/owner issue markers.
  Identity-invalid, bounded repairs of previously flagged non-master sources
  remain unexamined; authentic restored bytes can retire stale failure markers.
  No opaque payload is semantically imported by this classification check.
- Existing normalized product-ID winner/conflict/candidate reporting remains
  unchanged. The DPT audit verifies first-normalized-value behavior, semantic
  read/stored/dropped counts, retained original bytes, reopen and retry. A
  same-file main/subtype collision is not a new chosen value: both losing
  declarations remain in the source bytes. An orphan subtype is a semantic
  drop but not a collision in the legacy dropped-DPT counter. DPT declarations
  still have no source-winner provenance; KL-86's residual is **not closed**.
- Dynamic structural refusals name descendant references within the existing
  shared activation budget. Active refs, labels and skipped-ref diagnostics
  draw on one budget; exactly one budget diagnostic marks truncation. It is
  not an exhaustive inventory after that point. Value-dependent branches and
  unexpanded module definitions stay outside this enumeration. No scripts,
  Repeat semantics or formerly refused node are executed by this work.

## Evidence obtained so far (offline, 2026-10-02)

The bounded shape census at exact Gira/MDT scopes observed 115 instances,
113 unique packages and 67 unique master blobs. On the latter, canonical
TranslationUnit RefId and Version each occur 1,870 times. This scope/dedup unit
differs from the old unrestricted 1,928-occurrence inventory. Neither count
defines Version semantics or proves compatibility outside the observed files.

Synthetic regressions verify exact keys/occurrences/samples, original bytes,
namespace boundaries, late and depth-budget refusal, measured-zero versus
unavailable v18 upgrades, missing/malformed/oversized retained sources and
idempotent re-derivation. Additional DPT tests are in `tests/install_reports.rs`;
the strengthened mixed active/skipped budget assertion is in
`tests/dynamic_tree.rs`. Deferred-commit RED retained two unknown rows before
the rollback correction; GREEN now verifies rollback and released ownership.
Two compiled behavioral mutants (rollback disabled; unconsumed metadata falsely
declared consumed) were caught; original source hashes were restored exactly.

Owning-crate tests and warning-denied Clippy pass at the preceding checkpoint.
The baseline/candidate comparison subsequently found exactly five aggregate
counter deltas plus the commitment: isolated unknown keys +230, shared keys
+226, and both evidence tables +226 rows. An independent exact-path XML census
over retained baseline bytes predicts every new row and occurrence. Full-value
comparison of 34 other table projections is equal, excluding only the legacy
package unknown-count field separately checked against the same census.
All prior unknown values survive; fresh reports and their read/stored counters
change only by those independently predicted findings. Identities, installed
outcomes, source bytes and normalized values are unchanged.

After that reconciliation, pins were updated and the assertion-complete matrix
passed over 115 instances / 113 packages. Permanent assertions distinguish
113 shared rows and 3,125 occurrences per attribute from 67 distinct master
blobs and 1,870 unique-source occurrences. Temporary pre-assertion database and
JSON capture hooks were removed. The reviewed repair-path correction below is
not accepted by this earlier matrix run; final gates must rerun the matrix.

The opt-in `tests/ar05_corpus_upgrade.rs` audit harness ran successfully on real
task-local v18/fresh databases, then passed again after re-pinning. It compares every non-evidence
table by complete ordered value commitments before/after upgrade and explicit
rebuild, then compares current distinct source/key evidence with a fresh corpus
database. Distinct current evidence is compared deliberately: historical
per-install duplicate rows and original install reports need not equal a fresh
v19 install and must not be rewritten to force that equality.

The separate final review reproduced stale owned issue markers after retained
bytes were repaired. Two new regressions compiled and failed behaviorally,
including an opaque source temporarily masquerading as a master. Exact-owner
cleanup now keeps identity-invalid repairs unexamined and replaces a bounded
master's old classification marker with derived evidence or a named failure.
All nine rebuild regressions and strict all-target Clippy pass. A compiled
cleanup-disabled mutant failed both repair regressions; its source was restored
byte-exactly before the subsequent error-marker extension. Full renewed gates,
final review sign-off and publication remain **PENDING**.

### Final harness review correction (2026-10-02 05:55 CEST)

The first complete coordinated acceptance exited 0 (workspace 2,879,
ProductDB 580, Web 1,312; explicit matrix/upgrade/census passed). Subsequent
separate review found two test-harness gaps, not a new product/schema change:
the census's pathname output bypassed corpus filesystem protection, and the
upgrade comparator selected Languages descendants without the wrapper.

Four synthetic regressions first failed behaviorally. Census now uses the
existing matrix publisher, extracted into `tests/corpus_support/output.rs`:
descriptor-held validation, a different output filesystem, stale-result cleanup
before discovery, and owner-only temporary-file/atomic-rename publication.
The upgrade query compares exact wrapper and descendant paths and excludes
lookalike prefixes. Private evidence failures remain value-safe comparisons.

Focused suites pass (upgrade 1, census 3, matrix 9). A compiled wrong-device
mutant fails the new refusal regression; the shared source was restored
byte-exactly, then tests and strict all-target ProductDB Clippy passed again.
Actual corpus pin/shape verification and complete acceptance on the corrected
source remain **PENDING**; earlier successful gates are retained separately.

### Complete corrected-candidate acceptance (2026-10-02 06:43 CEST)

The later census resource-order regression also verifies declared ZIP member
limits before archive construction, through the existing bounded validator.
Its compiled guard-removal mutant fails behaviorally; source restoration is
byte-exact and the restored focused suites pass (upgrade 1, census 4, matrix 9).
The intermediate delivery run failed on a diagnostic expectation in this new
test; that failure remains separately recorded, not relabelled as acceptance.

Final process `proc_f9f87cee4329` exited 0. All 20 expected steps passed:
workspace 2,884 tests / zero failed / 163 ignored / 146 result blocks;
ProductDB 585 / zero failed / 24 ignored; Web 1,312. Strict workspace Clippy,
build, semantic shadow bindings, dependency policy and all four intended-root
repository gates passed. Matrix, real v18 upgrade and shape census each actually
executed their one private case with zero failure/ignored/skip markers. Complete
aggregate shape equality and the 595-file source freeze passed. This supersedes
the pending corrected-candidate acceptance statements above, not their historical
scope or failure records. Separate in-session review has no remaining blocking
finding; this is not an independent external approval.

The four AR05 checklist scopes are verified. KL-86's audit is complete, but
missing DPT source-winner provenance remains a numbered residual, not an accepted
release waiver or an implemented feature. Other unsupported Dynamic semantics and
historical unavailable reports remain as documented. No broader ETS/platform
claim, UI ownership change, bus action, release tag or hosted release follows.
