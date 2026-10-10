# Data editing and tooling — Session 2 ownership

Authorized 10 October 2026 by the attached nine-package goal. Branch
`feature/data-tooling-20261010`, worktree `data-tooling-20261010`, initial
source `fd6b1cff37b2713361e256282cc2aef46e2cbcb9`.

## Scope and order

1. R-SEC-01: primary-source key-provenance research, no Secure implementation.
2. Bounded RTF display/editing, original preservation and native roundtrips.
3. PDB-04: type-evidenced time/colour/picture/slider parameter editors.
4. FUTURE-03: project notes and separate help, ADR-0031 baseline.
5. FUTURE-02: one previewed, validated, reversible offline repetitive task.
6. FUTURE-01: opt-in confirmed MCP mutations through application services.
7. FUTURE-05: verified-origin catalogue updates, project associations unchanged.
8. TOOLS-01–05: collector provenance and truthful reproducible measurements.
9. KL-110 and GAP-T30-04: separate prerequisites/test plans only, no hardware.

## Exclusive Session 2 surfaces

- New research `docs/research/secure-key-provenance.md` and focused contracts.
- New isolated frontend features under `apps/knx-web/src/features/rtf/`,
  `features/parameter-editors/`, `features/project-notes/`, `features/help/`
  and their tests/locally scoped styles. Exact existing parameter-panel
  integration ownership is checked before any edit.
- Relevant domain/application/persistence/product/API/MCP modules and their
  tests; no unrelated refactor. Exact existing files are named per package
  before changes, after checking other active tracks.
- Generated bindings are Session 2's responsibility only when needed;
  Session 1's manifests/configuration are not edited.
- AI-statistics measurement code in its verified permanent repository and
  `docs/ProjectStats.md`; visual dashboard presentation remains Session 1.

## Protected Session 1 surfaces

`docs/contracts/ux-feedback-ownership.md` in Session 1's worktree defines
its exact paths. Do not edit shared App/shell/navigation/global styles,
EN/DE catalogues, frontend manifests/configuration, notification/log UI,
Flow/image-export/motion, Dashboard, achievements or humour. Supply isolated
components, explicit wire contracts, requested copy and integration tests in
a handoff. Do not regenerate or rewrite an unrelated binding or fixture.

## Shared-state protocol

Each owner edits only its own worktree, branch handover and private logs.
Shared documentation/ledger changes are reconciled serially at reviewed
package integration, preserving foreign entries. Root checkout/index,
Functions/Sites, release and import-validation tracks remain untouched.
Check fresh main and other owners before every shared-file change or push.
No releases, deployments, subagents, quota probes, secret capture or live bus.
A frozen release source remains independent; these packages are not inferred
release blockers or part of that snapshot.

## Acceptance

Small independently reviewed packages; test-first behavioural regressions,
relevant unit/integration/migration/native roundtrip tests and builds. Inspect
isolated UI components in an actual browser. Unknown semantics remain explicit
and preserved. Research completion is not runtime/ETS/hardware acceptance.
Integration/push occurs only at coordinated verified boundaries. Session 2
updates its own handover before any task end and keeps the nine-package goal
open until each authorized deliverable or explicit external prerequisite has
been accounted for.
