# WP-08 — TypeDate lexical slice (28 October 2026 source delivery)

Published main-first merge `4beabe531` (code `b5dc92aa8`, feature documentation
`c57ba4147`) on `58375375b`. Local candidate, tracking ref and live `main`
readback matched after publication. This is source delivery, not a release,
native date-editor acceptance or device compatibility claim.

## Delivered contract

- New ingests store `TypeDate` as kind `Date`; edits validate the documented
  `yyyy-mm-dd` spelling and existing Gregorian calendar days. No DPT 11 year
  range is imposed on stored values.
- Product database v29 is data-only. Source re-derivation preserves the first
  program declaration's authority while retaining losing-declaration evidence.
- Original measured install facts must validate before mutation. Missing or
  inconsistent counts never become newly measured zero/current values.
  Conflicting historical samples are made explicitly unavailable rather than
  attributed to a package member without evidence.
- A source whose affected TypeDate evidence differs between targeted and
  extended-package replay is refused atomically. It keeps `Other`, original
  evidence and counter contributions, with a named source failure and affected
  reports unavailable. Independent sources continue. This also applies when
  only one extended package is recorded: an earlier targeted ingest cannot be
  excluded from the shared ledger.
- The download-image codec remains refused by type. The plain text field stays;
  checked native/date-picker behavior, Colour/Picture codecs and broader WP-08
  closure are not delivered.

[Independent source evidence](../research/2026-10-28-typedate-value-form.md) ·
[remaining work](../open-work/WP-08-parameter-value-codecs.md) ·
[B1 schema-admission amendment](../contracts/catalogue-jobs/schema-admission.md#amendment-data-only-product-database-v29-28-october-2026).

## Review and regression evidence

The whole-feature review requested changes. The duplicate-program and two
report-history defects were reproduced with semantic RED tests, then corrected.
Independent correction re-review approved: 64 unique tests and three separately
failing/restored mutants for original-report validation, sample preservation and
first-program authority. These are separate scopes, not additive coverage totals.

A separate mode investigation used actual v28-writer databases and exposed a
false-measured report for identical source bytes in different package modes.
The conservative refusal was tested RED then GREEN. Final Opus review approved
the savepoint/source-failure boundary and executed copies of genuine v28
scheme-21/23 fixtures, invariant controls, reopen comparisons and a 27-variant
mode sweep. Original fixtures remained unchanged. The probe rollback itself was
verified by source reading; idempotent post-processing cannot independently
prove it from final tables alone.

Public fixtures use independently invented date values. The unpublished
investigation lineage is retained privately and is not an ancestor of the
published source branch.

## Maintained acceptance

| Attempt | Candidate | Verdict | Rust passed / failed / ignored | Selected corpus |
| --- | --- | --- | --- | --- |
| Publication branch | `c57ba4147` | 14/14 stages, frozen inputs | 4,476 / 0 / 165 | 97/97 in six targets |
| Main-first merge | `4beabe531` | 14/14 stages, frozen inputs | 4,476 / 0 / 165 | 97/97 in six targets |

Both change-scoped receipts are complete with every selected stage exit zero
and valid execution proof. User-data inventories were checked, with no touched
or unchecked entries. Rust totals are sums of the executed target summaries;
the two runs overlap and are not distinct combined coverage. Each tooling stage
registered 466 tests with six explicit optional skips. The selected corpus tests
ran separately; 165 ordinary Rust cases remained ignored. Web/binding/browser
application suites were not selected by this backend change. The web bundle was
built as a desktop-crate prerequisite; the desktop crate was compiled/exercised
by the Rust stages, not through native UI acceptance. `clean-desktop` itself is
Cargo cache preparation, not a build test.

Preserved unsuccessful attempts: the initial package gate stopped at missing
Node prerequisites; those were installed and built before the publication
candidate gates. A later investigation gate was explicitly interrupted/reaped
on review rejection. The first final-review CLI invocation received no prompt
and ran no review; its successful successor is recorded separately. None of
these attempts is reclassified as accepted evidence.

## Remaining boundaries

WP-08 and PDB-04/PDB-05 retain their existing broader statuses. Inherited v15
Color/Time duplicate-program promotion remains a separate historical repair
follow-up (helper-level witness, not an authentic pre-v15 upgrade proof).
No bus, hardware, deployment, new release or full ETS-equivalence claim.

Private receipts, reviews, synthetic v28 witnesses, source bundle and manifests
are retained under `<profile>/evidence/wp08-typedate-20261028/`. The shared root
checkout was deliberately not synchronized or cleaned; foreign work is preserved.
