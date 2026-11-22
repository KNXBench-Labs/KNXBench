# WP06-S1 — Folder-caption namespace and language (14 November 2026)

## Published bounded runtime; result-only documentation successor

The bounded runtime extends the existing read-only installed-product folder-caption
overlay to same-schema project/23 sources and explicit product language requests.
It is **software-accepted and published on main `8de9487de`** (hash mapping 2026-11-22: alpha-22 history; on current main as `02cb3beca`), on base `041df68a0`.
The parent verified independent Standards/Spec APPROVE, the complete selected
17-stage gate and exact local/tracking/live publication readback. The runtime gate
ended at 13:55:07 CET on 14 November 2026; the parent acceptance aggregate was
recorded at 13:58 CET. This result-only documentation successor was written
awaiting its own independent review, complete planner-selected gate and
publication; it is published on main (`1dab91cfa`) and was reviewed
retrospectively against source on 22 November 2026
([note](#retrospective-source-review--22-november-2026)), without a new gate.
It changes no runtime, bindings or tests and runs no builds, tests, gates, browser,
VM, bus or hardware operations.

Whole [WP-06](../open-work/WP-06-communication-object-organisation.md) remains
evidence-gated. This is a KNXBench display policy, not an ETS compatibility or
reference-tool fallback claim. The historical source-only contract and summaries
below retain their original evidence scopes; fresh runtime results here come from
the admitted parent acceptance aggregate, not reopened private logs or inputs.
Maintained test fixtures are independently fictional; no private-derived
publication examples are introduced here.

## Source and evidence epochs

| Epoch | Identity and admitted result |
| --- | --- |
| Source admission | Contract scoped against `28db298af49e870b654db758c3d0b91baa343ee0` |
| Initial implementation | `90d422a0b33cd7ce3af0587c2be78d4977acf376`, on that base; eleven feature paths including generated types and tests |
| Initial RED | Unchanged-production HTTP regression: 3 failures out of 12, exit 101. Initial ProductDB RED was a compile failure for the missing API, **not behavioural RED** |
| Initial focused GREEN | Parent rechecked retained logs: captions 27/27, shared language rule 7/7, server HTTP 12/12, server unit 5/5; three-crate clippy with warnings denied exited 0. Projection 100/100, Vitest 23/23, tsc exit 0 and fmt/headers/local-info success were author-reported in the admitted parent summary, not fresh execution here |
| Correction candidate | `9b0d937dce07be157fe015a4142368f3173042b7`; delta from `90d422a0b` touches the caption reader, its tests and the server caption adapter |
| F1 original-production RED | 28 passes / 1 failure, exit 101: oversized application source must retain `ResourceLimit`, rather than a false `SourceIdentity`, after product and Hardware2Program ownership checks |
| F2 exact M11 RED | 28 passes / 1 failure, exit 101: a reachable second foreign manufacturer translates the same program; the mutant reaches the intended foreign-visible-text assertion, not an unrelated refusal |
| Corrected focused GREEN | Captions 29/29, shared language rule 7/7, HTTP 12/12, server unit 5/5, each exit 0; retained clippy/fmt/headers exit 0. These are filtered/overlapping lanes, **not additive totals** |
| Independent correction review | `review02`: Standards/Spec APPROVE; F1/F2/F3/F5 CLOSED, no remaining findings in the correction delta. Unchanged-area initial source/test/mutant conclusions and the adjudicated amendments are carried, not a new whole-feature review run |
| Main-first composition (historical source-only epoch) | `789ab6143ed1f032ff90a508aa7c28bdcac15cc5` on fresh main `041df68a0754192c47cf78a1f8296771680dce53` (hash mapping 2026-11-22: alpha-22 history; on current main as `cfdbcf06b`); parent verified all eleven feature paths and the full feature hunks including context, equal after excluding row offsets. No fresh execution or publication in that composition epoch |
| Published runtime acceptance | `8de9487de4d84517479de159353de3748cd7a840` on `041df68a0754192c47cf78a1f8296771680dce53`, carrying integration `789ab6143` / corrected feature `9b0d937dc`; independent Standards/Spec APPROVE (`deleg_816ee240`), complete selected gate 17/17 with ordered exits 0 / valid proofs, exact local/tracking/live readback |

### Fresh integrated runtime results (parent-verified aggregate)

These results belong to the published runtime, not the earlier correction epochs
or the documentation successor:

- Nested web: **34/34 stages**, targeted/default selection of **103 specs**;
  **1,238 browser cases = 1,143 functional + 95 exclusive**. Production configs
  were **not selected**.
- Rust: **6,187 passed / 0 failed / 181 ignored**, across **29 summary rows/binaries**
  in nine selected packages. The separate binding lane is **100 passed / 0 failed /
  0 ignored** across two rows; it **overlaps Rust and is not additive**.
- Separate runner-managed namespace corpus regression: **97/97** in six targets;
  the namespace stage completed and runner-created links were removed. This is
  bounded regression coverage, not full manufacturer/reference/hardware compatibility.
- Tool tests: **563 total = 547 passed + 16 skipped**.
- Inputs/source identities remained frozen; the current fingerprint matched the
  receipt. The parent checked `userDataTouched = []` and `userDataUnchecked = []`,
  and confirmed exact local/tracking/live publication readback. No release,
  deployment or native UI acceptance follows.

The documentation author admits the aggregate and eleven-path source binding only;
no underlying corpus, observer logs, native artifacts or private fixtures were
reopened, and no product execution was repeated.

`fix01/PARENT_VERIFY.json` records successful parent comparisons of all three
correction sources, all six retained logs, retained/committed diff, candidate
identity and clean tree. `review02` independently checked those same three
source and six log hashes. These are admitted retained-evidence summaries;
this author did not reopen the underlying logs or rerun product checks.

The main-first parent summary freezes SHA-256 identities for eleven paths:

- `crates/knx-productdb/src/com_object_folder_captions.rs` and its `tests.rs`;
- `crates/knx-productdb/src/query.rs`;
- `crates/knx-projection/src/lib.rs`;
- `apps/knx-server/src/domain.rs` and `src/com_object_folders.rs`;
- `apps/knx-server/tests/it/http_com_object_folders.rs`;
- `apps/knx-web/src/bindings/ProductFolderCaption.ts`,
  `ProductFolderCaptionLanguage.ts` and `ProductFolderCaptionUntranslated.ts`;
- `apps/knx-web/src/comObjectView.test.ts`.

The documentation handoff records before/after comparisons against those hashes;
no binding regeneration or feature-source edit belongs to this successor.
Main's published WP07 runtime `e31f0e062` (hash mapping 2026-11-22: alpha-22 history; on current main as `da2b12808`) and documentation `041df68a0`, and the
separate E4 delivery, retain their own acceptance scopes; their results are not
WP06 acceptance evidence.

## Implemented read policy and admitted amendments

1. **Namespace admission.** Existing uniform `http://knx.org/xml/project/20`
   sources remain supported. A schema-23 owning project may additionally use
   uniform `http://knx.org/xml/project/23` sources. Mixed/foreign namespaces and
   schema 21 remain refused. Hash/source identity and canonical ownership walks
   stay authoritative; a distinct product source must agree too. F3 restricts
   application preloading to schema 23 and keeps distinct product bytes
   branch-local. F1 preserves the deferred application refusal after product
   and Hardware2Program ownership, rather than selecting a false namespace.
2. **Explicit-language selection.** The reader uses the same verified application
   source's owned `Languages`/`TranslationUnit` text and the existing
   `best_matching_language` helper (`query.rs`, visibility change only).
   Exact identifier wins; otherwise the lexicographically first stored identifier
   starting with the request plus `-` wins. A longer requested identifier does
   not fall back to a shorter stored identifier. No rule is duplicated and
   `query/caption_templates.rs` is unchanged.
3. **Literal default and provenance.** The selected translation must pass the
   existing literal filters: nonblank, bounded text, no template markers or
   text-parameter reference support. An existing normalized row must corroborate
   the selected XML text; a missing row is not disagreement. Explicit requests
   receive `language.status = Translated` with the selected stored identifier,
   or the default declaration with `Untranslated` and `NoTranslation`,
   `UnusableTranslation` or `TranslationDisagrees`. An unusable selected variant
   does not trigger a second language pick.
4. **No language requested.** The optional caption DTO `language` field is
   **absent**, not an explicit untranslated reason. This parent-admitted
   amendment preserves old no-language responses. The two extra generated
   language/reason TypeScript types are an admitted mechanical consequence of
   the single optional provenance field, not an additional feature surface.
5. **Resource refusal (F4 documentation obligation).** An explicit-language
   translation-pass resource refusal withholds the **whole optional caption
   overlay**. It is **not a default-caption fallback**. Reader and repeated DTO
   copy budgets also refuse the whole overlay rather than publish a prefix.
   Saved placement/text remain independent. Existing silent dropping of
   non-resource caption issues may remain; this slice does not widen diagnostics.
6. **Shared borrowed language helper.** F5 centralizes access to the selected
   language row for DTO accounting/mapping without changing extraction semantics.

## Verification seams and conservative limits

Maintained fictional HTTP source covers a schema-23 project with no root object
list, a Folder before a Channel, and no saved node text. Its uniform project/23
product is actually installed and checked before caption reads. Repeated GETs
exercise default, exact/prefix language and conservative fallback responses;
placement/keys, project snapshot, revision, opaque data and history remain
unchanged. The saved-schema-20 control retains saved text, including blank and
whitespace fields, ahead of translated product captions; the currency marker is
unchanged. Client unit source retains saved-text display precedence and grouping.
These are source-bound retained focused results, not new execution in this epoch.

The mixed-namespace HTTP fixture **bypasses the package installer**, which already
rejects that mix, by ingesting individual files. It tests the caption read refusal
for that constructed state; it is not a generic installability witness. A7's
XML/normalized-row disagreement is **unit-only**, not additional HTTP coverage.
The pre-existing multi-program translation-row importer issue is not repaired or
claimed covered end to end by this slice.

Saved text precedence, placement, keys, grouping, currency, history, priority and
stored bytes are unchanged. Captions never feed IDs or keys. No product-database
schema/migration, new persisted data, folder rename/write flow, new source mapping
or recomputed placement is added. Empty folders, nested/module-folder semantics,
templates/text references, wider schemas and reference-tool missing-translation
behaviour remain unknown or unsupported. The exercised Channel→Folder path is
not evidence for nested Folder or module placement. The fresh 97/97 aggregate
establishes only the selected namespace corpus regression above, not full
manufacturer-corpus or reference-tool compatibility. No native UI, real-project,
hardware, bus, release or deployment claim follows.

## Retained admitted inputs and next boundary

Private root: `<profile>/evidence/2026-11-13-wp05-14/`. The earlier source-only
documentation records these historical summaries; they were not reopened by this
result-only successor:

- `scoping/WP06_S1_ADMITTED_CONTRACT.md` — original bounded behaviour contract;
- `implementation-wp06/PARENT_AUTHOR_VERIFY.md` — initial results and amendments;
- `implementation-wp06/fix01/manifest.json` and `fix01/PARENT_VERIFY.json` —
  correction identities, retained RED/GREEN/check results and parent comparisons;
- `implementation-wp06/review02/RESULT.json` — independent delta closure and F4;
- `implementation-wp06/mainfirst01/PARENT_INTEGRATION.json` — eleven frozen source
  identities and full-hunk composition, not integrated execution.

This successor admits the fresh parent result and exact source binding:

- `<profile>/evidence/2026-11-13-wp05-14/implementation-wp06/delivery/PARENT_PACKAGE_ACCEPTANCE.json`
  — parent-verified published runtime acceptance, counts, frozen inputs and readback;
- `<profile>/evidence/2026-11-13-wp05-14/implementation-wp06/delivery-review01/SOURCE-BINDING.json`
  — eleven SHA-256 source identities and the eight admitted documentation paths.

Next (as written on 14 November 2026): independent review of this result-only
documentation successor, its complete planner-selected gate, then separate
publication/readback by the parent. The successor was published as `1dab91cfa`
without those steps being recorded; see the retrospective review below for what
was and was not done.
Source-ID statuses/counts are unchanged. WP-41, E4, WP07, root-local and
foreign-owner stop lines, including WP-12 consent, remain in force.

## Retrospective source review — 22 November 2026

The 14 November text above says this successor awaited independent review, a complete
planner-selected gate and publication. It is published on main as `1dab91cfa`. On
22 November 2026 its runtime claims were compared with source (published runtime and
current `origin/main`). This is a documentation-claim review against source, **not** a
new gate, test run or acceptance: no counts were re-executed and none are added here;
the 14 November results stay parent-verified aggregates.

**Source state.** Of the eleven frozen paths, nine are unchanged on `origin/main` since
the runtime merge `a6b7fda7e` (the caption reader and its tests, `com_object_folders.rs`,
`http_com_object_folders.rs`, the three bindings, `knx-projection/src/lib.rs`,
`comObjectView.test.ts`). `query.rs` changed later only for unrelated device-catalogue
work (`best_matching_language` is untouched) and `apps/knx-server/src/domain.rs` for
WP-05 parameter-caption work; the folder-caption call site is unchanged.

**Claims checked against source (all consistent):**

- Namespaces: `PROJECT_20`/`PROJECT_23` (`crates/knx-productdb/src/com_object_folder_captions.rs:26-28`);
  schemas other than 20/23 refused as `UnsupportedSchema` (`:132-135`); project/23 only when
  `project_schema == 23` and hardware and loaded application are rooted in it (`:246-253`);
  a distinct product source in another namespace is refused by its ownership link (tests
  `mixed_and_foreign_product_namespaces_stay_refused_with_todays_issues`,
  `distinct_product_source_in_another_namespace_than_the_hardware_is_refused`).
- Language: selection uses the shared rule (`localize`, `:379-402`, calling
  `query.rs:113`, now `pub(crate)`): exact wins, otherwise the lowest identifier with the
  prefix `request-`; a longer request does not match a shorter stored identifier. The
  literal filter is nonblank, at most 4096 bytes, no `{{`/`}}` (`:940`), plus
  the `TextParameterRefId` exclusion for the declaration (`:948`). A normalized row that
  disagrees yields `TranslationDisagrees` (`:400`); a missing row does not.
  The three untranslated reasons are `:86-93`.
- No language requested: the DTO `language` is optional and absent
  (`apps/knx-web/src/bindings/ProductFolderCaption.ts`, `com_object_folders.rs:134`).
- Resource refusal: `Unavailable(ResourceLimit)` is only logged
  (`apps/knx-server/src/domain.rs:2949-2953`, `captions` stays unset, no default fallback);
  an exhausted DTO copy budget skips the whole overlay (`:2747-2749`;
  `apply_product_captions` admits every string in two passes before applying any,
  `com_object_folders.rs:71-139`). Other unavailable issues are dropped silently
  (`domain.rs:2954`), as documented.
- Counts that can be read from source: 29 `#[test]` in the caption reader tests (29/29),
  12 in `http_com_object_folders.rs` (12/12), 6 `best_matching_language` unit tests in
  `query.rs`. The 7-test shared-rule lane and server unit 5 were not re-derived.

**Correction found.** `8de9487de` is a documentation-candidate commit with the tree of
the runtime merge `789ab6143`; it is an ancestor of `v0.1.0-alpha.22` but not of
`origin/main`, whose history carries the tree-identical commits `a6b7fda7e` (runtime
merge; tree equals `789ab6143`) and `02cb3beca` (tree equals `8de9487de`). The feature
commits `90d422a0b`/`9b0d937dc` likewise appear on main as `119e17757`/`6cd37fcd6`.
So "published on main `8de9487de`" names the Alpha 22 history identity; on main the same
runtime content is `a6b7fda7e`, and this successor is `1dab91cfa`. The 14 November
sections keep their original hashes as dated history.

**Not established by this review.** The 14 November runtime gate, corpus and acceptance
numbers (17/17, 6,187, 97/97, 563) are quoted from the parent aggregate and were not
repeated. This successor itself never had its own independent review or planner-selected
gate recorded before publication; the retrospective review replaces neither. WP-06
broader slices (templates, rename, empty/nested/module folders, recomputed placement)
stay evidence-gated.
