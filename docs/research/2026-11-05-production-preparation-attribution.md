# Production preparation attribution — 5 November 2026

## Bounded result

The expanded 5000-device production fixture's preparation includes substantial
Playwright snapshotter work, not just application rendering. This is a diagnostic
attribution of the recorded 3df fixture, not an application optimization, accepted
gate speedup or explanation of a historical timeout. No product, default trace,
scheduler, timeout, worker, retry or coverage policy changed. GATE-01 stays open
for its other separately listed investigations.

## Inputs and execution

Source checkout: `3df19a7a13ce5fa90c9f3260f6c7ec9246326d14`.
The production bundle was reused from its actual producer
`b3d1a0a7cb1a4e863d574bfc184300be17ac955d`, with frontend source, config,
lockfile and assets unchanged; it was **not rebuilt on 3df**. Build fingerprint:
`a7cbfe8281733b6d832d58b1c63039e687639d4aa94f5002ac0d67c2b49b00c2`.
Runtime: Node `26.7.0`, locked Playwright `1.63.0`, Chromium `155.0.8059.84`.
These are version/package-lock admissions, not whole dependency-content or
browser-binary attestations.

Exactly one unchanged canonical case and two identically instrumented cases ran,
in that order, following one compiled inventory per arm. Each inventory and
runtime selected one exact named case and bound the same case ID within that arm.
All three passed at the original **120 s / one worker / zero retries**. There were
no skipped, flaky or unexpected cases. Strict intercepted APIs, explorer
expansion, all 5000 table DOM rows, last-device filtering, exactly one devices
GET and no unexpected requests remained intact. Literal forward/inverse checks
bind the instrumented copy to the original body; both profiling arms use the
same copy. This is not a full application or package gate.

The three normal shared leases were held. Each invocation used its own
loopback-only namespace, real non-root UID and zero effective capabilities.
The recorded capability bounding set was nonzero (`CapBnd: 000001ffffffffff`);
no zero-bounding-set confinement is claimed. No artificial CPU load was injected; this is a shared-host,
single-observation series, not a controlled quiet-host benchmark. Start/end load
averages are retained per invocation.

## Observed case durations

Seconds; case duration is the Playwright result, not the whole CLI process.
Render/filter are the original late-start attachment, not whole preparation.

| Arm | Case s | Render attachment s | Filter attachment s | DOM frame snapshots |
|---|---:|---:|---:|---:|
| Unchanged canonical control | 21.325 | 10.345 | 1.080 | 72 |
| Profiled original snapshots | 23.085 | 10.717 | 1.125 | 72 |
| Profiled snapshots-off diagnostic | 10.634 | 6.197 | 1.789 | 0 |

The slower diagnostic filter is retained rather than hidden. One sequential
observation per arm does not establish repeatable improvement, statistical
significance or the exact profiling overhead of the canonical/full difference.
Order, cache and uncontrolled host-load effects are not ruled out.

## Phase and stack evidence

Both profiled copies capture CPU profiles, renderer timelines, before/after
Performance metrics and an action-wall attachment for three explicit phases.
Their sampling/trace windows include observer work outside the action clock;
these clocks must not be treated as identical or summed into whole-case CPU.

| Phase | Full action s | Off action s | Full inferred snapshotter-shaped signed weight s | Off same weight s |
|---|---:|---:|---:|---:|
| `project-creation` | 1.890 | 1.107 | 0.395 | 0.000 |
| `explorer-expansion` | 7.797 | 0.566 | 6.917 | 0.000 |
| `devices-table` | 11.843 | 7.987 | 3.332 | 0.000 |

The table uses the independent review's conservative snapshotter-shaped
classification (not every descendant of a `captureSnapshot` frame). Native
children under application bundle ancestors remain app-associated; unresolved
blank URLs and program/idle/GC weights stay separate. The full explorer profile repeatedly contains
`captureSnapshot -> visitNode -> visitChild/visitNode` stacks. No such named
stacks appear in the diagnostic arm. They match the installed Playwright
snapshotter implementation in `node_modules/playwright-core/lib/coreBundle.js`
(SHA-256 `549070af3acabb3efcc4f55bfe6210f9f7c2fcf633cf7eaa59bfe60719969171`).
This is a name/ancestry/source comparison plus the trace-control counterexample,
not byte authentication of injected scripts: Debugger script texts were not
captured. Blank script URLs are not blanket-classified as Playwright. Application
bundle leaf frames are tracked separately by the retained `index-BLoau9oK.js`
URL; native layout/style/paint slices remain separate and overlapping nested
slices are not summed as a CPU total.

CPU weights use raw signed `timeDeltas` in microseconds. **All six profiles have
negative entries**; they are preserved, not clamped, absolutized or dropped.
Per-profile negative magnitudes range from 5.267 to 24.608 ms and their indices
are retained. Therefore these are diagnostic sampled-stack weights, not exact
CPU seconds, utilization or a conservation proof. The cause of the negative
entries is not established. Action clocks and raw node/sample identities are
separate evidence.

Remaining table work is real too: the diagnostic still has application-bundle
frames and renderer layout/style events. The result does not say all cost is
tracing or that the application has no remaining rendering cost.

## What the counterexample changes

Only the diagnostic copy changes the `trace.snapshots` setting; screenshot,
source and other trace options are inherited. Actual archives contain
**72 / 72 / 0** `frame-snapshot` records in canonical/full/off order and screencast
frames in every arm. This proves the intended control actually took effect.
Playwright's DOM snapshot option also controls network capture; it is not a
pure DOM-only switch. Neither the elapsed difference nor the weighted stacks
is a precise decomposition of every tracer mechanism. Snapshots-off is **not**
a coverage-equivalent gate replacement and is not proposed as a new default.

## Setup refusals and preservation

An incompatible copied dependency skeleton was rejected before browser runtime;
locked offline npm installation then passed the runtime identity guard. The
first actual temp admission returned direct exit 2 before any arm: both initially
allowed deep write parents need at least 117 socket-path bytes against the
unchanged 107-byte policy limit. Outer controller success did not credit a test.
A separately recorded parent scope extension allowed one normal
`browser_directory`-managed short directory under the already authorized profile
scratch, without changing its allocator, guard or sandbox. A new runtime epoch
then executed the three cases. Original setup refusals remain historical.

Sources and bundle were unchanged after execution. The managed directory was
removed, its scoped process-marker scan was empty and all three leases closed.
This checks accessible references to that owned directory and direct-child
reaping, not every possible orphan or host process.
No KNX bus/device, native GUI, VM, private-project, release or deployment work
was performed. Raw evidence and the exact private harness are retained separately
from the public research narrative.

The closed runtime/input packet has **200** entries, SHA-256 manifest
`f75512801fd8025ce5e5c3885e320cf687f20e81ea3f2330ab5730a011e56957`.
Durable evidence locator: `<profile>/evidence/gate01-preparation-attribution-20261105/`.
`raw/runtime-01/` and `raw/runtime01-frozen-manifest.json` retain the closed
producer packet; `frozen-inputs/` materializes its 200 authenticated byte
payloads independently of retired source/dependency links. `raw/analysis/` and
`raw/independent-evidence-review/` keep derived readers separate from raw inputs.
Original link identities are recorded separately, not traversed into unrelated
data. Artifact-reader corrections did not rerun any product case.

Independent evidence review: **APPROVE for this bounded runtime evidence only**.
Its three Important reporting limits (composite control, missing injected-script
bytes, signed/mixed-window samples) and one Minor scoped-cleanup limit are
retained above. This verdict is not an optimization or whole-package acceptance.
The public edition contains only authored synthetic-fixture results and
open-source implementation references, not third-party project/vendor data.

## Sources and related records

- Playwright tracing options, consulted 5 November 2026:
  `https://playwright.dev/docs/api/class-tracing#tracing-start`.
- Primary CDP `Profiler.Profile` and `timeDeltas` schema, consulted the same day:
  `https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/json/js_protocol.json`.
- [Original load-sensitivity investigation](2026-10-28-devices-browser-load-sensitivity.md).
- [Earlier language-fanout correction](../status/2026-10-28-document-language-fanout.md).
- [GATE-01 residual scope](../open-work/GATE-01-test-gate-follow-ups.md).
