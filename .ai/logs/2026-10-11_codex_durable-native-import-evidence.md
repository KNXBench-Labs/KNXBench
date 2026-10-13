# Durable native import evidence — active continuation

## Current implementation checkpoint (2026-10-11 15:37 CEST)

The older RED-scaffold notes below are historical. The application codec and
server lifecycle are implemented and focused GREEN, including corruption,
identity/version/duplicate/budget and ambiguous-severity refusal. Original JSON
is served as text and exported verbatim; historical counts survive native
restart/edit/Undo/Redo. Existing login guard is actually tested. Generated Web
binding is updated; no desktop frontend binding exists in this source tree.

Built-browser four-case EN/DE400/1440 run actually downloads all1001 synthetic
records; four production frames inspected. Broader owning acceptance is next,
not yet complete. Two earlier queued RED admissions ran no tests; isolated
one-job RED subsequently executed and corrected GREEN. No private replay or
publication/deployment/hardware action in this package.


## Scope and ownership

Separate branch/worktree `fix/native-evidence-20261011` / `native-evidence-20261011`
from accepted `4e0e874d9bf2da2084f0acec70cc610846ebee46`. Do not edit the concurrent
`import-lifecycle-20261011` preflight or `import-lifecycle-integration-20261011`
publication checkout. No public private-project content, schema bump, protocol,
manufacturer execution, release or deployment.

## Current evidence

- Proposed ADR-0112 documents a versioned, bounded application-owned native
  passthrough report envelope; the domain and store remain format-independent.
- Frontend reader/panel/client now load only on explicit action, validate current
  project/server binding, reject late replies and offer a complete local JSON
  download with privacy/historical disclaimers. Preview is explicitly bounded to
  65,536 characters; it is not the full report.
- Focused frontend RED controls fail at the missing reader, missing report button
  and absent client function. Successor reader/panel/API/Dashboard tests and
  production build/typecheck passed. These are synthetic/frontend results, not
  full application or native persistence acceptance.
- Application/report and productive HTTP native lifecycle tests are prepared.
  Application codec remains a deliberate RED scaffold until its focused test
  actually executes. Do not report it implemented or publish its failing tests.
- Rust RED worker `proc_b96e2c6c6cf4` is waiting under the established ordered
  workspace/alpha leases. The earlier 180-second admission returned before any
  command/test ran (empty log); it is resource contention, not a failing RED.
  No gate was unlocked or another worker stopped.

## Next executable steps

1. Read the queued RED log after actual completion; distinguish compilation from
   named semantic failure. Retain exact source-bound outcome.
2. Implement envelope encoding/validation/full report restoration in `knx-app`,
   then complete server import attachment, native re-open provenance/counters and
   authenticated full-report GET. Keep absent legacy evidence unavailable.
3. Add corruption/duplicate/version/identity/budget tests and native refusal
   preservation; run actual synthetic import/save/restart/edit/undo/redo.
4. Integrate historical marker into generated projection/UI without copying the
   concurrent preflight. Run EN/DE production-browser download/layout cases,
   inspect screenshots, update maintained docs/status, self-review, run relevant
   owning suites/Clippy/repository gates, then focused local commit.
5. Coordinate later source integration separately; no main push performed here.

## Local retained evidence

Profile scratch namespace `native-evidence-20261011`: `scope.json`, separate
frontend RED/GREEN/build/type logs and `app-red-queued.log`. Rust target and
frontend dependencies remain active; cleanup must not remove an active worker.
No private project replay has been executed in this package.
