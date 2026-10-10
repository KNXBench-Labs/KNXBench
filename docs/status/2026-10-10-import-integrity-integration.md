# Import-integrity source integration — 2026-10-10

## Scope and tested revision

Owner-authorized source integration follows the earlier local acceptance.
Feature `d8346579` repairs archive member identity, recursive object trees,
unassigned placement, metadata retention/provenance, bounded parser observations,
manufacturer diagnostic replay, neutral baggage classification and completion
vocabulary. [Contract](../IMPORT_INTEGRITY.md),
[ADR-0107](../adr/0107-import-source-integrity.md).

Full integration acceptance ran on `367405f1`, including the current upstream
privacy and secure-capability witnesses. Subsequent upstream changes were only
Markdown; the concurrent test catalogue and complete owner status entries are
preserved. Runtime, dependency, frontend and test inputs remain byte-identical
through documentation reconciliation. Final repository/documentation checks
and exact normal-push ref readback are separate publication conditions.

## Executed checks

All 23 merged-source gate stages passed:

- Rust workspace tests: **3,893 passed, 0 failed, 188 ignored**; complete workspace
  build, including desktop compilation. Ignored tests are not measured coverage.
- Frontend: **2,595 passed in 163 files**; offline install/build and both extra
  TypeScript configurations passed. This is component/API proof, not native
  accessibility or fresh packaged-GUI acceptance.
- Explicit private original-service test: **1 passed, 0 ignored**, with production
  import, idempotent repeat, native Save/Reopen/Re-Save and unchanged input.
- Explicit private RefId test target: **3 passed, 0 ignored**, including its
  configured independent-project witness and its synthetic controls.
- Selected offline reference-corpus groups (ordinary plus ignored tests within
  each target): parser **36**, store **3**, application **7**, CLI **23** passed;
  all had zero failures/ignored results. These groups overlap the ordinary
  workspace test inventory and must not be added into a distinct-test total.
- Warnings-denied workspace/all-target Clippy; all five nonempty repository
  gates; documentation checker, Python tests, formatting and whitespace passed.
- Actual merged-source production CLI original import: exit **0**. Independent
  bounded ZIP/XML-to-native/product-row comparisons verify source identity,
  placement, parameters, modules/arguments, object order/ownership, overrides,
  links, building references, metadata and retained member payloads. Product
  verification and separate entire-corrected-model offline readiness ran without
  bus/network access. No private project quantities or original/member hashes
  are published here.

## Failed attempt and corrected corpus contract

The initial integration attempt completed broad tests/build and private checks,
then exposed an old exact unknown-metadata assertion in
`the_ets6_project_reports_no_malformed_com_object_ref_ids`. Current upstream
passes that older assertion on the same reference input. The feature deliberately
recognizes documented `Name`/`CompletionStatus` slots, so its previous unknown
inventory no longer applies.

The correction in `367405f1` pins the exact remaining vocabulary and preserves
all loss, occurrence, sample, malformed-ref and retained-attribute assertions.
It additionally compares both original project XMLs against independent indexed
ZIP reads. Removing those retained XML payloads causes that named assertion to
fail at runtime. The original failed attempt is not counted as success; the
entire integration gate was repeated afterward.

## Native/model boundary and remaining work

Native/model12 is a scalar completion-vocabulary barrier on the current v11
upstream: no new entities/reference fields or native tables. Selective import's
reference closure is unchanged and its explicit guard follows the current model.
Unknown stored completion values fail closed; historical missing source cannot
be invented by an identity migration. Parallel, still-unmerged model changes
must reconcile this version before later integration. Current published alpha.7
artifacts remain unchanged.

Inactive object overrides are reported and survive native persistence through
byte-exact retained source. They are not silently activated; applying an override
after reactivation still requires documented semantics. Source observations are
lexical measurements, not complete typed acceptance. Reports/session logs are
not a new durable native report feature; retained source makes observations
independently reconstructable.

Channel/additional-address/IP modelling and parameter/loader execution follow-ups
are separately research-assessed, not implemented support. Single-sample bias,
unverified schemas, full ETS semantics/export/reimport, signatures, vendor
execution and hardware commissioning remain bounded. Self-review only; no
subagent, release, deployment or hardware action. Detailed input-bound acceptance
and rejected attempts stay local and Git-ignored, outside this repository proof.
