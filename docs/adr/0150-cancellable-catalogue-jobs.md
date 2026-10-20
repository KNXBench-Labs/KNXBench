# ADR-0150: Owned cancellable catalogue jobs over the existing installer

- **Status:** Accepted for implementation — production/runtime acceptance pending
- **Date:** 2026-10-20
- **Scope:** WP-04.3; no production implementation or runtime acceptance

## Context

The existing synchronous catalogue update rehearses on an in-memory catalogue
copy, and does not own a cancellable server job across upload, preview, consent,
backup, install and shutdown. ProductDB already has a shared transaction-private
installer seam. Replacing the parser or introducing another importer is neither
required nor justified by the accepted WP-04.1/.2 evidence.

An initial independent design review required explicit Ready/apply expiry,
App-root ownership across real wizard unmounts and bounded replay semantics.
Their scoped rereview accepts the revised choices, but that is not approval of
managed ownership/binding, resource/artifact enforcement or exact DTO/lifetime
implementation. Both final independent review halves and their scoped correction
rereviews now accept the exact consolidated design. Original REVISE verdicts
and source-bound evidence are retained; this approval is architecture only.

## Decision

Adopt the [complete contract and readiness record](../contracts/catalogue-jobs/design-readiness.md),
[job/consent/UI contract](../contracts/catalogue-update-jobs.md),
[managed owner and witness](../contracts/catalogue-jobs/managed-binding.md), and
[protocol/resource/artifact/lifetime specification](../contracts/catalogue-jobs/protocol-artifacts.md)
as one versioned implementation contract following independent readiness approval.
The adjacent JSON schemas and cross-field policy specify proposed exact wire
and artifact data, not already-generated or validated production DTOs.

- Reuse one installer/parser/report body with quiet legacy wrappers and typed
  operation-wide Stop/control siblings. No duplicated importer or parser rewrite.
- Require current-schema managed file-backed DELETE-mode catalogue, with explicit
  refused modes/shapes, no job-driven creation/migration or fallback. Startup
  migration remains separate. Preserve legacy public APIs, token v1 and routes.
- Use one managed owner/health/generation/incarnation, same-W external-write
  observation and a full typed chunked current-schema witness. Validate exact
  source/profile/policy/preview under IMMEDIATE before live install.
- Rehearse on owned disk copies from a separate pinned reader, complete and
  publish the before-state backup before live writes, then use one outer live
  transaction and atomic cancellation-versus-commit claim. Explicitly classify
  commit, rollback and unknown outcomes; quarantine uncertain writers.
- Admit named source/page/result/record allocations before their owned growth;
  provide no aggregate disk/RSS or hard cancellation-latency guarantee. Preserve
  complete authoritative evidence with an explicitly complete safe projection.
- Own files through a stable kernel lease/create-new identities, manifest-last
  visibility and no-replace synced backup publication. Quarantine failed cleanup;
  ordinary final recovery backups are not terminal-job garbage.
- Keep status/cancel outside ProductDB, every DTO state strictly admitted, replay
  memory finite and consumed serials unreusable. Keep one tracked worker through
  settlement and the same explicit standalone/desktop/headless drain boundary.
- Place the shared UI controller above both actual catalogue and wizard lifetimes;
  separate selection-local stale paint from catalogue-wide committed invalidation.

## Consequences and validation

Implementation is staged A–F in the readiness record. A gate primitive alone is
not a cancellable installer; a snapshot capability alone cannot grant Ready
consent. Production routes cannot become reachable before transitive controls,
resource/artifact integrity and both process lifetimes are exercised.

The source/checksum-bound five-case SQLite mechanics experiment supports only
its synthetic DELETE-mode backup/reservation/rollback observations. It is not
ProductDB schema, owner/ABA, cancellation, resource, browser, shutdown or crash
acceptance. New production tests, deterministic fault/barrier cases, compatibility
oracles, independent review, maintained gates and publication remain required.
No schema migration, wider format limit, hardware admission, ETS compatibility
claim, completed WP-04.3 or WP-19 follows from approving this architecture.

## Rejected alternatives

- In-memory whole-catalogue preview or live install-and-rollback as rehearsal.
- Separate importer/parser, generic jobs framework or persistent outcome ledger.
- A checked boolean followed by COMMIT, default Drop as rollback evidence, or
  cancel/timeout response presented as proof a worker ended.
- Partial package/source-set fingerprints, unqualified permanent idempotency,
  first-N evidence pretending completeness, or PRAGMA/stat/free-space as total
  disk/RSS quota proof.

## Review status

Both independent halves return DESIGN READY for the corrected contract. The
parent authenticated all 17 documents, 1,958 source inputs, 70 managed authority
inputs, 36 protocol inputs and the bound review artifacts before approval.
The live exact witness/copy-cookie rule, ordinary-close/force distinction and
native-sidecar exception are reviewed design decisions with explicitly future
implementation tests. This accepts the A–F implementation sequence, not a runtime
feature, completed package or permission to expose premature job routes.
ADR-0150 was confirmed unused on fetched upstream before this promotion.
