# Default browser case parallelism assessment — 27 October 2026

Status: assessment complete for the default lane and cross-suite production
concurrency on `448a647a0`; no runtime change admitted. GATE-01 remains open
for its other bounded follow-ups.

## Decision

Keep the default functional lane file-parallel at four workers. Keep production
functional case parallelism ([ADR-0155](../adr/0155-production-functional-case-parallelism.md)),
the exclusive lane, inventories, standalone configs and worker budget unchanged.
The retained run has little scheduling headroom for a further default-lane
policy. No new browser run or measured speedup is claimed by this assessment.

## Retained evidence and independently checked arithmetic

The accepted web run of `3af5ea9e4` contains 925 default cases, partitioned into
856 functional and 69 exclusive cases. Their exact identities reconcile with
the inventory, with one passing result per executed case and no retry or skip.
The inventory names all 93 `.e2e.ts` specs present on the assessment revision;
35 are also selected by production configs, leaving 58 default-only files.
Relevant tracked web app and runner inputs are unchanged between the retained
web run and `448a647a0`; intervening fixture-test and documentation changes do
not turn the old measurement into a new execution.

| Historical functional-lane quantity | Seconds |
|---|---:|
| Sum of individual case durations | 1830.227 |
| Observed Playwright report wall time | 485.816 |
| Arithmetic four-worker work bound, sum divided by four | 457.557 |
| Longest-processing-time-first heuristic, indivisible files | 458.072 |
| Same heuristic, indivisible individual cases | 457.559 |

The heuristic assigns the longest remaining item to the least-loaded worker.
Its schedules are estimates using historical durations, **not mathematical
lower bounds or measured alternative runs**. Only sum divided by four is an
arithmetic lower bound within that fixed-duration model. File versus case
heuristic granularity differs by about 0.5 s. The gap between observed wall time
and the arithmetic work bound is about 28.3 s. It is model headroom, not a
promised saving or an upper bound on real performance: setup costs, module
reloads, Vite startup, contention and per-case durations can all change under
a different schedule. The longest functional file takes 258.6 s in summed case
time, below the four-worker work bound; there is no dominant single-file tail
like the earlier production-suite case.

## Independence review and limits

The read-only audit found no ordering hook in the selected `.e2e.ts` files and
traversed 13 test-side helper modules, two beyond the 11 helpers in the earlier
production review. This is not an exhaustive proof of independence: application
imports outside `e2e/`, aliasing, destructuring, hidden function effects and
scope shadowing remain outside the lexical safety net's full coverage.

`monitor-dpt-context.e2e.ts` constructs its scenarios and then appends one case
at module initialization. The production guard reports that mutation; it is
not evidence of cross-test mutation. A future default-lane opt-in must account
for it honestly rather than aliasing it to evade the scanner or weakening the
guard globally. No such source change is included here.

The historical eight-worker cold-Vite failures recorded in
[GATE-01](../open-work/GATE-01-test-gate-follow-ups.md) remain unresolved. This
assessment neither reproduces nor fixes them and does not increase workers.

## Cross-suite production assessment

Keep production suites sequential. This is a scope decision, not a claim that
all concurrency is unsafe or a measured rejection of a new scheduler.

The retained 16-suite production scope contains 557 suite-qualified executions
and 455.655 s of summed stage wall time. Individual source cases deliberately
appear in more than one suite: 116 file/line/title identities recur across
configs in that run. Per-suite inventory and lane identities reconcile; do not
assume globally disjoint source tests or deduplicate required suite coverage.

The present implementation assumes sequential stages:

- `tools/run_web_gate.py::run_production` gives each functional stage the
  configured four-worker budget, then runs its exclusive stage with one worker.
  Merely dispatching several copies concurrently multiplies the aggregate
  worker budget and lets one suite's functional/build work compete with
  another suite's exclusive measurements. A global budget and exclusive
  barrier are prerequisites, not an incidental loop edit.
- `tools/web_gate_support.py::MeasuredRun.save` uses one shared receipt and
  fixed `receipt.tmp` replacement. Concurrent calls require a single writer
  or separately reconciled stage records; the current writer is not a
  thread-safe interface.
- Each browser stage is launched under its own network namespace by
  `offline()`. Repeated loopback ports are therefore **not** a demonstrated
  cross-suite collision under the current launch path. Per-stage JSON and
  artifact paths are distinct too.
- Most suites read the prebuilt default `dist`; shared read-only consumption
  alone is not a race. Table-columns and UX-feedback build separate output
  directories in their web-server commands. Two lanes of the same such suite
  must not rebuild/empty that suite's output while it is being served; these
  two different suites do not share an output directory.

The original audit's proposed `cpuSeconds / 4` wall-time bound is rejected:
OS CPU consumption is not browser-worker occupancy. Browser processes may be
multithreaded or waiting; four browser workers are not four CPU cores.
No 105-second saving cap, port-remapping requirement, disjoint-global-case
claim or measured overlap benefit is accepted here. The retained timings do
not isolate startup cost or establish performance under overlap. Inventory
uses `--list`; its short recorded duration alone is not proof of web-server
lifecycle semantics.

Reopen only as a separately designed scheduler package with an unchanged
aggregate budget, global exclusive isolation, sound stage provenance and
controlled overlap measurements. This assessment does not create that package
or alter the existing scheduling policy.

## Reopening criteria

Reconsider only with a newer retained run showing a material file-local tail
or a separate explicit priority for case-level scheduling. Before any opt-in,
finish independence review of the actual consumed scope, retain complete
before/after case identities, execute at least two comparable four-worker
runs, preserve exclusive isolation and require a worthwhile measured gain.
An extra default-only subset lane is not admitted merely to create another
server startup and scheduling policy with no demonstrated gain.

Private evidence: `<profile>/evidence/gate01-default-audit-20261027/`, including
the original read-only report, scripts, raw-report locators and independent
parent verification. The original audit's speculative independence wording is
not a compatibility or runtime acceptance claim.
