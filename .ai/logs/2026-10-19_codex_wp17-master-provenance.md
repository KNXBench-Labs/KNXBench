# WP-17 — master identity provenance

## Scope and source

Owner request: execute the next package in `docs/OPEN_WORK.md`. The release
handover selects WP-17 as the next offline-ready Alpha 16 candidate. Started
from `5faed038`; claim published in `041bbd1d`. The selected checkout contains
foreign handover/research work and is deliberately not synchronized.

Contract: [ADR-0147](../../docs/adr/0147-master-identity-provenance.md).
Normalized first-winner values must remain unchanged. Retained bytes determine
current declarations, not historical winning-source/install facts. No bus,
hardware, release, deployment or third-party contact is authorized by this task.

## Evidence obtained

- Core implementation committed as `18d02546` (worker). Controller inspected
  the source, new regression assertions and raw logs: ProductDB 457 library,
  586 integration and 2 doc tests pass, zero failed; 30 integration and one
  doc test remain ignored. ProductDB all-target warning-denied Clippy passed.
  New direct-import, built CLI and HTTP projections each passed one regression.
- The independent pinned-corpus baseline on `041bbd1d` passed 11/11, with
  frozen source and 115 instances / 113 unique packages. Two independent raw
  XML counters predict 67 master sources, 26,110 declarations, 459 winners and
  43,631 added diagnostic rows (43,508 collisions, 123 malformed declarations).
  Candidate matrix execution and migration comparison remain pending.
- Web-only independent review accepted with one LOW coverage finding: explanatory
  description mappings were not asserted per diagnostic row. Exact EN/DE row
  assertions now catch a description-swap mutant at the named semantic assertion;
  original bytes were restored and the focused tests passed again.

- Existing master-parser baseline passed using an isolated data/config home
  and a task-specific Cargo target.
- Web regression: the two new diagnostic kinds lacked readable labels/source
  details. EN/DE component cases failed on their intended missing label, then
  passed with translated descriptions and escaped technical disclosures.
- Focused component/German-copy checks: 38 passed; production TypeScript check
  passed. Full Vitest: 4,136 passed, zero failed, 284 files.
- Initial four Chromium cases passed text/scroll-width assertions, but actual
  screenshots exposed left clipping at 400 px. Stronger bounding-box assertions
  failed in EN and DE at x = -113 px. `min-width: 0; overflow-wrap: anywhere`
  on the report fixes the intrinsic-width expansion; all four EN/DE ×
  400/1440 cases pass and every corrected screenshot was inspected.
- Neutral report heading: EN/DE cases failed against the old unsupported-data
  heading, then 38 focused cases passed with `Source diagnostics` /
  `Quelldiagnosen`. The original heading is inappropriate for an ordinary
  restated-identity collision.
- Documentation local-target/chapter checks passed after the source-build
  architecture/model/upgrade/manual additions. Heading-anchor acceptance remains
  part of the final package gate.

## Review correction round 1

Whole-feature independent review of `9a9cd2e4` rejected six IMPORTANT items:

1. Replay reported complete while leaving contradictory derived structural rows.
2. Persisted package diagnostics did not validate both losing and kept source
   inventories through the authoritative reader.
3. Missing/corrupt winner markers were conflated with explicit historical NULLs.
4. Repeated ancestor paths could amplify retained evidence without a cumulative
   output-byte budget despite depth/item/input limits.
5. A tracked root element generated a double-leading-slash path rejected later.
6. Existing normalized historical winners followed by the first observed v27
   parse lacked its own focused regression (code appears correct, coverage gap).

Correction is test-first and local; prior passing suites are not acceptance of
these fixes. No implementation has been pushed. The corpus initial run is
retained with frozen inputs, 10 passed / 1 failed; only independently predicted
changes may be re-pinned. Controller independently recomputed the complete
baseline/candidate JSON comparison: exactly the predicted three new table counts,
package diagnostic count and aggregate commitment changed; no other fields did.
The ported matrix file matches the worker's recorded hash. Its v16-shaped
projection pin remains explicitly pending; no final green run is claimed. Both
the corrected-core 11/11 run and the 42-table full-value migration comparison
remain outstanding. Final matrix acceptance must use the corrected core.

## Correction evidence

Correction commit `e2f7783c` addresses all six R1 findings. Controller matched
all ten owned source hashes to the worker's tested inventory and inspected
the actual repair, cached inventory validation, marker decoding, byte admission,
root-path and historical first-observation regressions. Raw ProductDB logs show
458 library, 597 integration and 2 doc tests passed, zero failures; 30 integration
and one standalone corpus test remain ignored. A fresh independent review and
corrected-core corpus/migration acceptance are running. Prior passing results
do not replace those pending verdicts. Controller reran the intended `it`
master-provenance selection: all 24 tests passed, including the 11 new review
regressions. An earlier controller invocation named a nonexistent `integration`
target and measured no tests; that setup refusal is retained separately.

Upstream `2ff5b370` adds only the Alpha 16 successor claim. That owner explicitly
waits for this package's published release of migration v27; preserve its entry
on integration and do not absorb its column-promotion repair into WP-17.

## Correction round 2 and corpus evidence

R2 accepted five original findings and the direct marker decoder, but found
ordinary replay still seeded lost winner markers as paired NULLs. Controller
reproduced that exact behavior in `r2-marker-red.log`. Fix `887f024a` keeps
seeding at schema creation only; ordinary replay validates all existing normalized
identities' marker presence and pairing before its first evidence write.
Regression covers both DPT forms and every other family, deletion and both
half-NULL variants, nested rollback, normal replay and explicit historical NULLs.
Full ProductDB and warning-denied Clippy passed again; final independent review
is pending. This is source-evidence recovery correctness, not database authentication.

The `e2f7783c` matrix passed 11/11 with frozen inputs and unchanged originals.
Only the independently predicted table/diagnostic deltas moved; projection pin
was measured rather than guessed. Controlled v26 → v27 migration preserved all
42 unaffected full-table values and all 859 historical diagnostic rows. The
old setup refusal (symlink identity) and 10/11 stale-pin executions remain
separate. These receipts certify `e2f7783c` only; the final-source matrix and
migration proof for `887f024a` are running in a new attempt namespace.

Final independent focused review (`core-review-r3.json`) accepts `887f024a`:
R2-01 and the remaining R1-03 replay route are closed, no new blocking findings.
Controller read the complete verdict and source binding. All original review
findings now have reviewed dispositions; this is code-review acceptance, not a
package-gate, integration or publication verdict.

Final-source acceptance for `887f024a` is now complete: matrix 11/11 with
frozen inputs; controlled migration preserves all 42 unaffected full-table values
and 859 historical diagnostics; originals and inherited receipts stay unchanged.
Controller verified nine artifact hashes, read the raw matrix result and checked
all 42 full-value verdicts. No preceding result was retagged as a new execution.

## Pending acceptance

Maintained change-scoped package gate (all 90 affected default browser specs,
catalogue production lane, seven Rust packages and selected private corpus),
main-first integration, merged acceptance, publication and owned cleanup remain
pending. Gate input preparation includes the justified final matrix pins and
current capability/ledger/limitations summaries; no implementation has been pushed.
Earlier green web results are not whole-package acceptance.
