# Document-language fanout: bounded browser correction — 28 October 2026

This package makes `useUiLanguage()`'s document-language effect idempotent.
It addresses a measured application-side cost discovered during
[GATE-01's 5000-device investigation](../research/2026-10-28-devices-browser-load-sensitivity.md),
not a change to gate policy or the table's rendering strategy.

## Mechanism and change

Every `useTranslate()` consumer subscribes through `useUiLanguage()`. On mount,
and when the selected language changes, each hook instance previously called
`document.documentElement.setAttribute("lang", language)` even if that exact
attribute value was already present. With many translated children this repeats
one global DOM mutation for every subscriber.

The effect now reads the actual attribute and writes only when it differs.
There is no new global cache or observer. Missing attributes, language changes
and an externally changed attribute encountered by a later mounting subscriber
still reach the same final value. English, German and imported-pack tags keep
the existing storage, detection, subscription and translation behavior.

## Test-first and attribution evidence

Four new tests against the real hook and document were all red on the original
implementation. In the 128-reader StrictMode mount, the old code wrote `lang`
256 times; the corrected hook writes once. Tests also cover an already matching
attribute (zero writes), propagation of English/German/imported-pack changes to
all readers, additional subscribers and repair after external attribute changes.
An inverted comparison fails all four tests; the restored control passes all
four. The focused language/translation run passes 35 tests in four files;
TypeScript and the production build pass.

A Chromium CPU sample and renderer timeline separated project creation,
explorer expansion and the table in the production fixture. The original
project-creation phase takes 11.092 s with 9.875 s of CDP `ScriptDuration`.
Native `setAttribute` below the compiled document-language effect accounts for
about 8.987 s of sampled self time. With the value guard, that phase takes
1.913 s with 0.663 s of `ScriptDuration`.

These are one profiled sample per variant, with profiling overhead. Their
phase durations are not a whole-app benchmark. Much of the explorer phase is
`EvaluateScript`/`visitNode`/`visitChild` work consistent with the installed
Playwright DOM snapshotter, rather than layout/paint; it is not a second
application fix proved here. The table still has measurable script and layout
cost. Trace event buckets are nested and must not be added as independent CPU
shares.

## Uninstrumented before/after comparison

Same host and locked dependencies, shared gate leases held in normal order,
one browser worker, zero retries, isolated loopback-only namespaces. Baseline
source was reconstructed from `7e8c4a0ed`; fixed source and both production
bundle manifests were retained. Builds ran serially in the same owned worktree.
The original expanded-explorer spec bytes and all 5000-row, last-device search,
exact API-count and unexpected-request assertions stayed unchanged. The 120 s
case limit, lane membership, trace settings and worker policy stayed unchanged.

| Variant | Per-case times | Result |
|---|---|---|
| Original production, repeat-three | 41.515 / 41.803 / 43.443 s | 3 passed |
| Fixed production, repeat-three | 21.249 / 21.434 / 21.736 s | 3 passed |
| Original development, one sample | 76.283 s | passed |
| Fixed development, one sample | 32.785 s | passed |
| Fixed development, 16 shell busy loops, repeat-three | 105.275 / 99.153 / 104.394 s | 3 passed |

Production median: 41.803 → 21.434 s, **48.73% less whole-case time** in this
bounded series. This includes preparation and Playwright overhead, not a 49%
improvement in table rendering or general application throughput. The table
render attachment is essentially unchanged: 10.316–10.717 s before and
10.363–10.474 s after; filtering stays about 1.1 s. The source change removes
redundant root-language writes, not rows, assertions or explorer expansion.

All 11 before/after executions passed with one result per expected case and
zero retries. All 16 load loops were terminated and waited for. The fixed
source was restored and the original spec hash verified afterward.

## Boundaries and remaining work

The candidate stays below 120 s in this loaded series, but 99–105 s leaves
limited margin. It does not establish arbitrary-load stability or explain the
historical S2-03, cold-Vite, file-menu or signal failures. The earlier three
original dev failures under shell-loop load were a separate series without
shared gate leases; do not represent them as a simultaneous, leased before/after
load comparison. No production-under-CPU-load series was run.

The shared effect's final document-language value is preserved, but there is
no new native WebKitGTK, screen-reader or physical-device acceptance. GATE-01
remains open for residual large-document and historical-failure work. No
virtualization, framework, timeout increase, coverage removal, scheduling
change, backend change or bus contact is included.

## Provenance

Raw samples, renderer timelines, profiled spec variants, original/fixed source
bytes, original/fixed bundle SHA-256 manifests, JSON case reports, exits,
restoration records and mutation controls are retained privately under
`<profile>/evidence/gate01-language-fanout-20261028/`.
The uninstrumented spec SHA-256 remains
`9673428b679e8e9d1c509e4e97a46ebf1f92047597bed6823bf56009988fd79e`.
Runtime: Playwright 1.63.0, Vite 8.3.5, React 19.2.8, Node v26.8.1 and
Chromium 155.0.8059.84. This is source-level browser evidence, not a release,
deployment, hardware or general performance guarantee.
