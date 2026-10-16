# P4a: System-B object grammar and real-source baseline

Date: 2026-10-16
Agent: codex
Base: 7f9f3672707f616e1415a1ad3b3b21de9af3ace0

## Completed locally

- Verified actual existing AP1 baseline: three selected applications in one private
  package, three partial/non-executable results, 21 source-local diagnostics,
  read-only resolver, unchanged original inputs and matched runtime manifests.
- Saved and removed the temporary `zz_systemb_p4_probe.rs` test before delivery.
- Inspected original Configuration Procedures pp. 40, 44, 72–73 and Resources
  pp. 269–272; documented grammar versus source/image/execution/recovery limits.
- Added pure Core System-B Type-7 / Format-1 object-table rewriting, reusing
  communication flags, transmission priority and the legacy value-type subset.
  Explicit one-based u16 object numbers and initial-read flag; no mask selection.
- Added ten always-on byte/boundary tests. Observed absent-module E0432 RED,
  then 10/10 GREEN. Omitted initial-read mutant failed the intended named
  assertion; exact production bytes were restored and 10/10 passed again.
- Existing Core all-target tests passed during the first review run. Owning-crate
  Clippy then rejected constant-sized `chunks_exact_mut`; changed only that
  iteration syntax to `as_chunks_mut::<2>().0`. The failed attempt is preserved.
- Separate in-session self-review found no critical/important/security issue;
  this is not independent approval and no subagent was used.
- Corrected-candidate preflight1 passed all ten stages with complete matching
  source manifests: Core all-targets 832/0 in15targets, owning-crate all-target
  Clippy, formatting, documentation, five repository checks and whitespace.

## Pending

Full frozen branch acceptance passed all 17 stages on the reviewed candidate:
Rust 4649/0/207 ignored in 339 targets, Vitest 3626/0, 163 tooling tests
(one explicit skip), explicit Release source regression and workspace checks.
All 2908 input modes/content hashes and Git fingerprints match. Only documentation
is updated after that run; runtime source equality is checked before commit.
Feature committed as `feb0bd76`; actual non-FF merge `adb52990` passed all18
stages with the same Rust/Web totals plus isolated corpus152/152 in33targets.
All2908source identities and Git fingerprints match; temporary corpus links were
removed. Published merge read back equal to tracking/live remote main at03:49CEST.
Documentation-only closure and private evidence/owned retirement follow; no
runtime result is retagged as a new run. Selected dirty root stays untouched.
No test result from before the syntax correction is relabelled as a new run.

## Boundaries

No relative product image builder, source scope/substitution translator,
executable product plan, durable whole-mutation recovery, public mask admission,
schema/dependency/UI change, hardware operation or vendor execution is added.
The selected dirty root checkout is intentionally unsynchronized. Private source
XML, original PDF renders and detailed receipts stay outside Git.

Contract: `docs/contracts/offline-systemb-object-table.md`.
Research: `docs/research/2026-10-16-systemb-offline-prerequisites.md`.
Evidence: `docs/evidence/offline-systemb-object-table-2026-10-16.json`.
Private receipts: profile scratch `offline-download-20261015/p4/`.
The broad offline expansion remains active after this bounded prerequisite.
