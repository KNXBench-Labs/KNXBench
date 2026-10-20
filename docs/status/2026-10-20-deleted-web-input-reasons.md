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

## Verified delivery

Published on `main` as main-first merge `e49d863d` (feature `fdfb0691`).
Independent whole-change review accepted with no findings. The complete gate
on frozen `cd33b0d0` passed 10 parent and 69 web stages: 22 derived configs,
4,136 Vitest cases, 863 default browser cases and all 16 production suites;
5,287 Rust passes (215 ignored) and 155/155 selected corpus cases. Those results
remain bound to that candidate, not relabelled as a fresh integration run.

Upstream directory-integrity changes required a main-first remerge onto
`65d2f001`. The maintained 14-stage delta gate on `e49d863d` passed, including
2,638 Rust passes (175 ignored) and 149/149 selected corpus cases. Exact source
and build/tool identities prove unchanged web/planner inputs; ordered stage
proofs, frozen inputs and empty touched/unchecked-user-data lists were verified.
Local, tracking and live publication tips agreed. This is package delivery,
not release acceptance.

The private evidence archive was verified member-by-member: 443 evidence files
plus one manifest. Only owned worktree, two merged branches and rebuildable
scratch were retired; the selected stale/dirty checkout and foreign research
were preserved. The planner claim is released.
