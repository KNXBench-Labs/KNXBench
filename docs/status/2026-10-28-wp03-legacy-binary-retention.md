# WP-03 legacy binary enum retention — source acceptance (28 October 2026)

## Published scope

Main-first source `6dbdaf8b97296266e1d35918ad417da926fe234d` contains
implementation `68f9da61e`, review regressions `895f4f793` and documentation /
corpus expectations `89a3e5134`, based on `0154b8ae9`. Publication readback
matched local integration HEAD, fetched `origin/main` and live `refs/heads/main`.
The shared root checkout was deliberately not synchronized.

Validated legacy atomic-type-5 hex now reaches the existing Base64 binary-value
column without integer coercion or loss of leading zero bytes. Data-only
productdb v30 fills historical NULL values only from authenticated retained
source and matching relevant normalized identities. A refused payload stays
unchanged and receives an append-only finding; other payloads continue.
Historical import reports and diagnostics remain historical. Atomic-type-4
binary/length fields remain uninterpreted and explicitly diagnosed.

[Exact representation, evidence and migration limits](../research/2026-10-28-legacy-enum-binary-mapping.md) ·
[bounded WP-03 completion contract](../open-work/WP-03-legacy-binary-enumerations.md).

## Independent review and discriminating regressions

Independent Opus review found no proven production bug. Three surviving-mutant
test gaps and two documentation findings were corrected; the final correction
review approved with no open findings. The original twelve tests remain intact.
The seventeen-test suite includes valid-width Atomic4 noninterpretation, a
value-only field diagnostic, continued migration after refusal of the first
sorted payload, a parseable forged-source hash refusal and conservative
length-only-zero characterization. Review-only mutant campaigns and their green
controls were retained privately; no source mutant entered the delivery tree.

The separate focused run passed 17/17 with no skips; all-target ProductDB Clippy
passed. A direct ignored private legacy-corpus regression passed after measured
diagnostic expectation updates; prior RED observations remain retained, not
relabeled as successes. Existing model counts and evaluator checks were kept.

## Maintained acceptance

Both frozen branch `89a3e5134` and main-first integrated source `6dbdaf8b9`
passed the complete **selected 14-stage package plan**:

- Formatting, documentation, browser-lane classification, layering, headers,
  anchors, ledger, corpus registration and final diff checks.
- Tooling: 466 registered tests, with six optional tests skipped in each run.
- Fresh web bundle / Desktop build prerequisite, all-target warning-denied
  Clippy and **4,494 Rust tests passed / 0 failed / 165 routinely ignored**, in
  24 result blocks per run.
- Selected private corpus: **97/97** per run.

These are overlapping branch/integration scopes, not additive unique coverage.
Both receipts are complete with all fourteen stage proofs valid, frozen inputs,
and checked, unchanged user-data directories. No browser runtime suite or
bindings regeneration was selected. This is not full-workspace, native desktop,
Orca/WebKit, all-corpus or hardware acceptance. The separate ignored legacy
regression above is not silently included in the 97-test inventory.

## Retained boundaries

No broad conversion equivalence is claimed for unmatched programs or all legacy
files. Atomic4 encoding semantics, damaged-source reconstruction / refused
backfill retry, genuine independently produced v29 migration artifacts and
large-payload migration cost remain outside this acceptance. Re-ingest can add
source provenance aliases without changing mapped rows or diagnostics.

No `source_xml` is invented, no image/commissioning eligibility is widened,
and no bus, VM, user catalogue, device, deployment or release was touched.
A live legacy download remains unverified and requires its own authorization.

Private raw attempts, manifests, independent reviews, mutants, receipts and
publication readbacks remain in task evidence. No private payload is published.
