# Deleted web inputs have a named plan reason — 20 October 2026

## Bounded behavior

The existing planner now adds one explanation for changed **known web inputs**
that no longer exist: `web: deleted or missing input (selection stays conservative)`.
Values are the same repository-relative changed paths, deduplicated and sorted
by the existing planner. Missing unknown paths still select the complete gate.

This does not change the dependency graph, fallback, selected default/production
suites, budgets, retries, stages or receipt schema. An unresolved or dynamic
closure still does not prove exactly which deleted path caused a spec's fallback;
its existing reason remains unchanged. The extra line names the missing inputs
without inventing that causal mapping. The planner edit itself continues to
require the maintained complete gate (ADR-0144).

## Focused evidence

- Three named original-source failures: a deleted fixture, multiple deleted
  helper/fixture inputs and a deleted global build input lack the new explanation.
  All reach the intended `assertIn` assertion, not setup/import errors.
- All 32 planner tests pass after the two-line change. Five new cases also check
  unchanged selection, repository-relative output, sorting/deduplication,
  existing-input refusal of the missing label and unknown-input full admission.
- Twenty-two synthetic comparisons against immutable original `f6b196a8`
  preserve every selection field and all prior reasons after removing only the
  added explanation. They cover eleven change sets, with and without unrelated
  computed navigation. This is planning parity, not browser/runtime acceptance.
- The first parity-loader attempt lacked Python module file metadata and failed
  before any comparison. It is retained separately and not counted as parity.
- Existing planner tests remain byte-exact around the five added methods.

Separate in-session review and the maintained complete merged gate precede
publication. No independent approval or full acceptance is claimed by the focused
results. Final source-bound gate results belong in the delivery handover.

[Planner](../../tools/gate_scope.py) · [Tests](../../tools/tests/test_gate_scope.py) ·
[ADR-0144](../adr/0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md).
