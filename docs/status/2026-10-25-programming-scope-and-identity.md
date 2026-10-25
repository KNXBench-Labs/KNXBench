# Programming scope and application identity acceptance — 25 October 2026

## Delivered source

| Increment | Published revision | Verified boundary |
| --- | --- | --- |
| Exact partial-application response width | `02c437110` | Existing object 3/PID 13 precondition requires five response octets before backup and mutation; complete downloads gain no extra identity precondition |
| Advice from programming marks | `78fd9d780` | Core rule, app preparation, CLI, HTTP and web choose the concrete pending scope, or prepare nothing; explicit scopes remain available |

Both revisions were pushed and their exact remote tips read back. The selected
root checkout was not synchronised; delivery used an isolated integration tree.
The contracts define [identity comparison](../contracts/partial-identity-comparison.md)
and [scope advice](../contracts/device-programming-support-guidance.md).

## Acceptance

The application-identity candidate `afb92302c` passed independent source review
and all 16 selected maintained package-gate stages: 6,440 Rust tests passed
(225 ignored), 62 binding tests and 155/155 selected corpus tests. Main advanced
with documentation only. Main-first `02c437110` preserved the accepted runtime
inputs exactly and passed all 14 stages selected by the retained delta plan,
including its broader web selection. These are original acceptance plus delta
acceptance, not a freshly repeated complete runtime gate on the published SHA.
An observer's incorrect ten-stage expectation was corrected against the retained
fourteen-stage plan; producer receipts and successful executions were not changed.

The scope candidate passed independent whole-feature review and focused
correction reviews. The first integrated browser run passed 799 cases but failed
all twelve layout variants on a stale four-radio assertion: the feature adds a
fifth choice. The correction pins all five option values and keyboard selection
while preserving geometry and no-write assertions. On final `78fd9d780`:

- focused layout acceptance passed all 12 language/width/theme variants;
- all 17 stages selected by the maintained package gate passed;
- its nested web gate passed all 34 selected stages, including 874 default
  browser cases (811 functional, 63 exclusive) and the selected production suite;
- 6,470 Rust tests passed (225 ignored), 62 binding tests and 155/155 selected
  corpus tests passed;
- source fingerprints stayed frozen and both touched-user-data and
  unchecked-user-data lists were empty.

The focused and integrated runs overlap; their test counts are not added into
one purported unique total. Failed attempts remain historical evidence. No test
was skipped or timeout relaxed to resolve the layout failure.

## Remaining boundaries

Project programming marks express intent, not bus evidence. Address and medium
needs remain separate; a partial refusal never silently becomes a full download.
The server binds changes advice to the same snapshot used to re-prepare the
confirmed plan, not to all future flag edits. No new device-family admission,
physical exemplar identity, runtime-function or hardware acceptance is implied.

Download follow-up observation and its consumers are a separate, unmerged
package. Parameter-write policies and whole-mutation recovery remain open in
[PRG-01](../open-work/PRG-01-programming-gaps.md). KNX Secure remains excluded.
Software tests used simulated or isolated transports; no live bus operation,
release, deployment or physical-device test was performed for these increments.
