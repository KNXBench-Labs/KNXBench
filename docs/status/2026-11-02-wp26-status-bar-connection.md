# WP-26 IP status-bar connection — acceptance, 2 November 2026

## Delivered source

Main-first source `7ea7ff6dd` is published and was read back as the exact local,
tracking and live `main` revision. It includes the separate bound-stop backend,
the shared web controller, the footer control and the migrated monitor panel.
The earlier non-queueing start prerequisite is `038845ef8`.

This delivers the admitted **IP** slice, not USB transport or full WP-26 transport
coverage. Choosing a gateway is passive; Connect and Disconnect remain explicit.
Connecting also buffers bus telegrams. The selected gateway and the actual live
gateway remain separate, and selection never switches an existing connection.

## Source-bound software verification

The main-first selected package gate completed all **15/15** planned stages:
formatting, documentation, browser-lane policy, layering, headers, anchors,
ledger, corpus selection, tool tests, web, clean desktop build, Clippy, Rust,
selected private corpus and clean diff. Inputs stayed frozen; user-data checks
reported neither touched nor unchecked paths. This is a change-scoped package
gate, not an all-package release or native-UI acceptance claim.

| Final main-first execution | Result |
| --- | --- |
| Selected Rust targets (`knx-server`, `knx-desktop`) | 1,684 passed, 0 failed; 72 ignored in this ordinary run |
| Selected corpus lane | 51/51 passed in 2 targets; not an execution of every ignored test |
| Full Vitest suite | 4,976/4,976 passed |
| Default Chromium functional lane | 998 passed, 0 unexpected, skipped or flaky |
| Default Chromium exclusive lane | 69 passed, 0 unexpected, skipped or flaky |
| Production communication | 87 functional + 1 exclusive passed |

The further production configurations selecting changed fixtures are covered by
an additional complete, frozen web run on the **same published source**:

| Production configuration | Functional | Exclusive |
| --- | --- | --- |
| catalogue | 22 | 0 |
| devices | 65 | 1 |
| dialogs | 26 | 0 |
| group-addresses | 21 | 1 |
| instructions | 15 | 0 |
| notes | 13 | 0 |
| parameters | 27 | 0 |
| reload-guard | 2 | 0 |
| structure | 23 | 0 |
| table-columns | 156 | 4 |
| task-status | 27 | 0 |
| updates | 7 | 0 |
| ux-feedback | 34 | 22 |

All runtime cases in these lanes passed without unexpected, skipped or flaky
results. Zero exclusive cases means an inventory-backed empty lane, not executed
coverage. Counts belong to their own executions/configurations and must not be
added into a unique-test total. The repeated web suite is not a new Rust or corpus
run. Together with communication, these are all 14 owning production configurations.

## Review and regression closure

Independent source reviews closed R1–R9 and approved the full 44-file fixture
delta. The owning UI joins use the real controller, panel/footer and context
records with mocked APIs. They cover foreign-lifetime and same-lifetime delayed
answers, old poll absence, exact newer-record preservation, truthful suppressed
completion wording, token lifetime and local-before-context publication order.
Reattaching the exact session created by a pending start can complete without
resetting captured rows/cursor/flow; later polls are withheld in its regression
so they cannot conceal a reset. Isolated wrong variants distinguish that positive
path from the foreign-session veto and capture loss.

EN/DE Chromium evidence includes wide/narrow windows, UI scales 0.8/1/1.5,
welcome/project states, focus/Escape and unobscured controls with a live toast.
The narrow shell can require scrolling to reach the footer; this is not a claim
of a sticky footer or coverage of every theme/native engine.

Earlier failed/partial gates stay historical. The final merged focused replay
passed all nine companion-return cases with the unchanged exact URL assertion;
the main-first default lane also passed. The earlier isolated `?view=editor`
observation was not reproduced; its cause is not established or relabelled as a
proven flaky test. Strict fixtures answer only the declared passive activity GET;
unknown routes/methods and unexpected-request assertions remain intact.

## Remaining boundaries

- The new `/api/bus/monitor/stop-bound` requires the exact process/session identity
  and never falls back to legacy global stop. The download-holder notice still
  uses that legacy stop; this package does not migrate every caller.
- Only observed client ownership can fence delayed publication. An unobserved
  other-window session is not knowable client-side; server binding still prevents
  a bound request from stopping a different session.
- A valid response from a new lifetime before that lifetime has been observed can
  be conservatively left unpublished; refresh exposes status, never retries a
  connection or sends a compensating stop automatically.
- A short lock reader can cause an immediate busy refusal; retry is manual.
  Closed sessions retain their slot until released. Partial activity is not proof
  that the real bus is idle. The panel's gateway field does not follow later
  footer selections.
- No USB, automatic reconnect/retry, new consent dialog, vendor/ETS compatibility
  certification, real-installation test, native WebKitGTK test, release or
  deployment is claimed. Bus behavior used fake connectors and intercepted HTTP.

[Task and USB gap](../open-work/WP-26-status-bar-bus-connection.md) ·
[Current technical limits](../limitations/KL-180-named-gateway-refusals.md#session-bound-stop-and-status-bar-connection-2-november-2026-source).
