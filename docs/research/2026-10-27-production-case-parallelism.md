# Production browser case parallelism — 27 October 2026

Status: implementation and targeted measurements complete; independent review,
final integrated package gate and publication pending.
Decision: [ADR-0155](../adr/0155-production-functional-case-parallelism.md).
Task: [GATE-01](../open-work/GATE-01-test-gate-follow-ups.md).

## Sources and scope

- Official Playwright parallelism documentation, retrieved 27 October 2026:
  <https://playwright.dev/docs/test-parallel>. The runner supports opting into
  case-level parallelism; parallel cases must be independent and cannot rely on
  shared global state. Hooks can execute separately per worker/case group.
- The locally locked Playwright version is 1.63.0. Its installed CLI
  `node node_modules/@playwright/test/cli.js test --help` lists
  `--fully-parallel`; the installed runner is the one exercised below. No
  dependency upgrade is included.
- Review scope: all 35 spec files named by the 16 production configs, plus 11
  recursively imported test-side helpers. Contexts/routes are per test; output
  uses Playwright's per-case output or stripped opt-in capture variables.
  No assertions, input fixtures, tags, retries or suite selection changed.

Private evidence: `<profile>/evidence/gate01-fullypar-20261027/`, including the
original logs, both JSON report trees, closure inventory, exact identity
comparison, CLI help and retrieved official page. Public notes contain no local
machine paths. Timing sums are computed from recorded stage wall times.

## Targeted runs

Both used `run_web_gate.py --profile targeted --specs '' --production` with all
16 production suite names, four workers, and only the functional production
lane receiving `--fully-parallel`. Each receipt reports success, completed plan
and frozen inputs. Each run executed **557 production cases**: 528 functional,
29 exclusive; exact case identities match the prior integrated gate per lane
and suite. No hidden retries, skips or inventory reduction were accepted.

| Suite (inventory + functional + exclusive), seconds | Prior integrated | Run 1 | Run 2 |
|---|---:|---:|---:|
| catalogue | 26.05 | 10.62 | 10.67 |
| communication | 54.55 | 33.64 | 33.64 |
| device-download-support | 29.88 | 19.61 | 19.45 |
| devices | 82.12 | 64.99 | 64.82 |
| dialogs | 17.75 | 9.28 | 9.11 |
| ga-members-download | 24.63 | 10.06 | 10.25 |
| group-addresses | 98.52 | 32.21 | 32.54 |
| instructions | 17.03 | 7.36 | 7.45 |
| notes | 13.39 | 7.36 | 7.36 |
| parameters | 16.99 | 7.46 | 7.64 |
| reload-guard | 6.65 | 5.57 | 5.71 |
| structure | 23.37 | 11.97 | 12.03 |
| table-columns | 152.04 | 124.22 | 123.85 |
| task-status | 24.97 | 11.98 | 12.14 |
| updates | 7.08 | 4.28 | 4.15 |
| ux-feedback | 86.72 | 78.30 | 79.02 |
| **Total from unrounded stage times** | **681.75** | **438.95** | **439.84** |

Baseline: integrated web gate of `745932994`, after the full default suite.
New measurements: candidate based on `eabea810c`, no default specs; load averages
at start about 1.25/1.22/1.13 and 2.73/4.38/3.05. These are single-host
observations with different warmup/load conditions, not a universal speedup or
an estimate of whole-package runtime. Guard hardening and docs followed the
measurements; the final package gate must accept the final inputs separately.

## Regression safety net

`tools/production_lanes.py` is checked by `tools/tests/test_production_lanes.py`
in the package gate's complete tool-test stage. It reuses masking/import
resolution from `browser_lanes.py` and follows named/default/namespace imports,
re-exports, literal dynamic imports and side-effect imports inside `e2e/`, with
a seen-set for cycles. A config that mixes explicit names with unresolvable
globs/expressions is refused. New spec counts intentionally require review of
the owning test's 35-file admission count.

Tests cover synthetic module state/mutations, harmless local state, comments,
strings, indentation, import aliases, recursive/cyclic helper traversal and
unsupported config selection. Additional synthetic controls failed on the
initial scanner before masking, import-closure and selection hardening; they
pass after those changes. Planting `let` in a real spec and in two real helpers
also produced named diagnostics before byte restoration.

This is a **lexical safety net**, not proof of arbitrary JavaScript independence.
Aliases, destructured bindings, describe-local state, hidden function effects,
computed imports, application/external modules and scope shadowing remain
limitations (including possible false positives). A new fixture or spec still
needs review. A direct targeted measurement does not execute the package gate's
tool tests; invoke the scanner separately if using only that entry point.

## Remaining acceptance

Independent whole-feature review, any required fix/re-review, a main-first
integrated package gate, publication readback and evidence-backed cleanup.
Default-lane case parallelism and concurrent production suites remain out of
scope; the earlier eight-worker development cold-start and historical SIGTERM
questions are not answered by these runs.
