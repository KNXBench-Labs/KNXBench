# Cold-start follow-up: requested workers are not concurrent workers

Date: 27 October 2026. Scope: GATE-01 source and compiled-inventory assessment.
No application, runner, deadline, lane or worker-policy change. No browser,
dispatcher, test body, real project or bus operation ran in this assessment.
The historical timeout cause remains **unresolved**; GATE-01 remains open.

## Delivery and package acceptance

Published main-first as `2890ad2d09c5f0cecee9d32cceac42866e18208e` (source-only feature `763d7fbe6`) after independent
review and four reporting corrections confirmed APPROVE. The original branch gate
passed ten stages with 461 registered tool methods (455 passed, six optional skips).
The first integrated candidate `83871cd88` failed the existing SIGHUP fixture's
partial-record reader; that failed receipt is preserved, not reclassified as green.
The separately reviewed test-only correction is published as `bc1b47a9a`, result
`939c1b384`, with its own final fourteen-method module and package acceptance.

This assessment was rebuilt main-first on that accepted correction. Its fresh
integrated gate passed all ten planned stages with frozen inputs and no touched
or unchecked user data: 466 tool methods registered,
460 passed and 6 explicit optional skips.
Local, tracking and live main and all owned target bytes matched after publication.
No application Rust/web/browser/bindings or corpus runtime suite was selected by
the documentation scope. These package checks are separate from the compiled-list
assessment; they are not browser timings or proof of the historical failure cause.
This result-note gate and own evidence retirement are separate operational closeout.

## Historical evidence boundary

The [19 October dossier](2026-10-19-test-suite-speed.md) records three failures
in `File menu holds entries only`, within 327 repeat entries under a requested
worker cap of eight and three repetitions of the untagged
`file-menu-submenus.e2e.ts` cases. Exact historical CLI flags, exits and pass
counts beyond those dossier observations are not established here from a raw
report; this assessment does not promote session recollection into raw evidence.

The original raw log, exact three case/repeat identities and stacks were not
recovered. The related clock/tag-change source `86c949466` is retained, but
is not authenticated as the exact failed worktree. Its lock selects
Playwright 1.63.0 and Vite 8.2.2; the current source selects Playwright 1.63.0
and Vite 8.3.5. A matching Playwright version does not authenticate the
historical app, config, environment, start order or effective concurrency.

The earlier phrase “eight workers loaded the app … at once” was an
inference, not a measured observation. Requested capacity alone does not
establish active worker count or a cold-server cause. A reported click
timeout without its stack also does not distinguish an absent button from
other actionability or startup failures.

## Executed current-source assessment

The source baseline is `f18af565dd0c8528efefbc7f2a0cb8c8f06a743a`. Locked tools
were installed with `npm ci --prefer-offline`. Playwright `--list` compiled
the real current spec/config without running test bodies or starting its
web server or browser. A private read-only reporter observed actual compiled
suite metadata. A second reporter applied the exact installed
`createTestGroups` routine, isolated from its dispatcher, to those suites.
It checked that every listed test ID was assigned exactly once.

| Current selection, repeat each 3 | Requested cap | Listed entries | Groups | Source-model concurrency ceiling |
| --- | --- | --- | --- | --- |
| `File menu holds entries only` | 4 | 27 | 3 × 9 | 3 |
| `File menu holds entries only` | 8 | 27 | 3 × 9 | 3 |
| Whole file, excluding `@exclusive` | 8 | 327 | 3 × 109 | 3 |

The focused selection contains nine distinct case titles. All three groups
are the same spec file, one group per repeat index 0, 1 and 2, with no
parallel parent or all-test hook in the observed focused hierarchy. The
four-cap and eight-cap focused group inventories are exactly equal.
`fullyParallel` is false in the current config. The installed grouping
routine puts ordinary same-file tests with the same worker hash in one
ordered group; repeat indices have distinct worker hashes.

These are **compiled scheduling inputs and an isolated source-function
result**, not measured process utilization, browser execution, timings,
passes or runtime skips. For the **current single implicit project**, with
`fullyParallel: false` and no CLI override, the ceiling is
`min(requested cap, group count)`. The installed runner's `createPhasesTask`
(`runner/index.js:6539-6547`) groups each project and clamps its reported
`actualWorkers` to the worker cap and maximum concurrent groups. The private
reporter groups the root suite, which is equivalent here only because that
suite has one project; this is not a general multi-project scheduling model.
No eight-worker load, speedup or historical three-worker run is claimed.
The machine-readable [record](../evidence/gate01-worker-shape-2026-10-27.json)
binds the consumed inputs and outputs. Private raw observations, reporter
sources and the verifier are retained at
`<profile>/evidence/gate01-cold-start-20261027/`.

## Consequence and remaining admission boundary

Do **not** run the proposed focused four-cap/eight-cap comparison as though
it varied four-way versus eight-way app load. Both compiled scopes have the
same three-group ceiling. Turning on `--fully-parallel` would change the
experiment and is not a reproduction of the retained command or an approved
change to the default functional lane.

The current config waits for Vite's HTTP URL; that is not evidence that the
application has rendered. `serveProject` navigates before `openFile` clicks
File. This identifies where future startup diagnostics would belong, not a
proven race or permission to add warmup, retries or longer deadlines.

A future runtime investigation needs either the missing original failure
identity/provenance or a separately admitted, clearly labelled current-source
failure with its actual grouping and observed startup/actionability error.
Retain existing assertions, zero hidden retries, scrubbed environments,
loopback-only confinement and the established lease order. No browser probe
or speculative implementation is admitted by this report. Cold Vite remains
a hypothesis; other unresolved GATE-01 timeout causes are unchanged.

## Verification and primary references

The corrected evidence reader checked all three group inventories, repeat
indices, unique IDs, derived focused title count, parent modes and all-test
hook absence, exact focused inventory equality and installed source hashes.
The recorded repository input hashes identify baseline `f18af565` blobs, not
all final package blobs: this package edits the two context documents to
qualify the historical claim. Their baseline hashes and the unchanged current
runtime spec/config/lock inputs are verified separately. Five malformed controls
(missing case, duplicate ID, wrong ceiling, wrong repeat and false runtime
claim) were rejected. Those are verifier controls, not product tests.

- [Playwright parallelism](https://playwright.dev/docs/test-parallel): ordinary
  same-file cases are ordered unless parallel mode is enabled.
- [Playwright web server](https://playwright.dev/docs/test-webserver): URL
  readiness is defined through HTTP response status, not app hydration.
- Locally locked Playwright 1.63.0,
  `node_modules/playwright/lib/runner/index.js`, bundled
  `packages/playwright/src/runner/testGroups.ts`, function `createTestGroups`.
  The source and exact extracted-routine digests are in the record.

The two official documentation pages were retrieved on 27 October 2026;
local locked source and the actual compiled suites govern this assessment.
