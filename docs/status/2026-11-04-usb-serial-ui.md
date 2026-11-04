# USB/serial TP-UART server and Web UI candidate

Scope: source implementation for a Linux server using the existing `knx-net`
TP-UART adapter. Main-only after Alpha 19; independent correction review approved,
with package acceptance and publication still pending.
No physical device, installed firmware, real bus, discovery, VM or deployment
acceptance is claimed. The [transport research](../research/2026-11-04-usb-tpuart.md)
remains the protocol evidence; its original CLI package boundary is historical.

## API and ownership

- `POST /api/bus/monitor/start-serial` accepts exactly
  `{"path":"<server-local-tty>","source":"1.1.250","mode":"passive"}` or
  `mode: "active"`. All three fields are required. Unknown/duplicate fields,
  gateway fields, malformed JSON, empty/whitespace/control-character paths,
  noncanonical/invalid/reserved/excluded sources are refused before a connector
  call. The caller must ensure its chosen source is unique on the installation;
  KNXBench neither probes for uniqueness nor assigns an address automatically.
- Non-Linux servers refuse serial starts. The production connector opens the
  existing `TpuartClient`; injected fake connectors exercise server HTTP tests.
  No gateway-string convention, IP fallback, automatic reconnect or uncertain
  send retry exists. The legacy IP start body and route remain unchanged.
- The existing `BusSession`, connector/tunnel seam, nonqueueing start admission
  and process/session-bound stop remain the owners. Start, telegram polls,
  activity and busy-holder responses project `serial: {path,source,mode}` for
  serial sessions; activity/holder `gateway` is absent for serial sessions.
  Serial-start clients require the exact echoed target and source address.
- Group read requires exactly `sessionId`, `serverIncarnation` and `destination`.
  Serial group writes use the distinct `/api/bus/write-bound` route with both
  identity fields plus `destination`, `value` and optional nullable `dpt` and
  `inputFormat`. Both handlers use nonqueueing reservation admission, compare the
  immutable session identity under the same guard and send only on that session.
  Missing/duplicate/extra/invalid fields fail before I/O; an identity mismatch
  returns `409 busSessionChanged`. There is no legacy-route fallback.
- Passive bound group actions return `409 passiveSerial` before tunnel send.
  Legacy unbound `/api/bus/write` remains available for IP sessions and refuses
  serial sessions with `409 busSessionBindingRequired`. Read uses the session's
  group-address style and sends one `GroupValueRead`; writes retain the existing
  codec and validation.
- Stop uses only `/api/bus/monitor/stop-bound?sessionId=…&serverIncarnation=…`.
  Both bound and legacy stops keep the shared reservation through serial
  reset/disconnect completion, including failure. A second stop/start during
  cleanup is refused immediately; legacy stop no longer queues behind a held
  reservation. IP stop retains its release-before-disconnect behavior.
  A serial disconnect error becomes a stop warning about unknown auto-ACK state;
  delivery is never retried. Link confirmation/readbacks are not device-state
  verification.

## UI and stored preferences

Settings, status-bar popover and Bus Monitor expose IP versus USB/serial,
server-local path, caller-selected source and passive/active mode. They share
`preferredBusTransport`, `serialPath`, `serialSource` and `serialMode` through
existing settings storage. Older settings default to IP and passive serial
configuration. `preferredGateway` retains its IP-only meaning for all existing
device/download/scan tools. Selection, settings, focus and polls never connect.

The footer opens serial configuration before its explicit serial Connect.
The active warning discloses automatic ACK of the local individual address AND
ALL group telegrams before connection. Passive sends are visibly disabled.
Observed targets remain distinct from edited preferences. Missing/legacy serial
routes fail without using the IP route. EN/DE copy uses the same contract.

## Verification boundary

Focused acceptance sources cover fake mounted HTTP, pure/API/controller/component
contracts and strict full-app EN/DE browser cases at 1440/400 pixels. The browser
spec belongs to the existing `communication` production lane and has its own
fixture TypeScript project. Execution results are recorded after the focused run;
parent-owned package gates, commit and publication remain separate.

Transparent NCN5130 firmware with 19200 baud 8E1 and end-to-end parity/error
signalling is required; host termios alone proves no installed bridge compatible.
No hardware enumeration, firmware flashing, KNX USB HID, FT1.2, arbitrary USB
serial, extended TP1 frames or serial device-management/download/scan integration.

## Review corrections F1–F4 (new source epoch, acceptance pending)

The independent review found unbound group traffic, early serial cleanup release,
hidden active-start uncertainty and a fresh-reattach provenance gap. The corrected
source uses the bound action contract above, retains the existing shared guard
through serial cleanup, and requires a discriminated action-session prop in the
compose form. Unknown known-serial metadata cannot select legacy IP writes.

`serialConnectFailed` now carries required boolean
`activeAutoAckStateUncertain`: true conservatively for every failed active
connector attempt, false for passive attempts. This is derived from requested
mode, never by parsing adapter text. Exact failure phase is unavailable at the
server seam: the warning means active initialization **may** have enabled ACK and
reset state is unconfirmed, not that ACK was observed. Validation errors before a
connector call and unsupported platforms do not carry that uncertainty flag.
There is no automatic reset, reconnect or retry.

The controller retains the latest admitted serial provenance for an exact
session/process identity across unknown reads. Fresh monitor reattach uses that
observation, never preferences, to keep missing serial metadata unverified and
lock both actions. A different identity never inherits the former target/mode;
a newer controller observation arriving during the first poll fences its late
answer for a different session/process, including a server restart observed with
no active session. An ordinary replacement observed by an
owned poll still follows the existing context/recording lifecycle.
Existing context/recording/action epochs remain the owners of publication.

The browser fixture preserves the parent-corrected exact combobox selectors,
adds strict bound read/write handlers and exact payload assertions, and separates
the existing one-time bootstrap IP discovery from zero additional discovery
caused by serial selection or actions. Parent's earlier browser, mutation and
server smoke evidence belongs to the pre-correction epoch. Correction-focused
execution results are appended after the batched checks; broader acceptance is
still parent-owned.

## Pre-correction focused execution (4 November 2026)

| Check | Observed result |
| --- | --- |
| New mounted serial HTTP tests, fake connector/tunnel only | 7 passed, exit 0 |
| Existing monitor/admission/bound-stop HTTP regression selection | 39 passed, 1 corpus test ignored, exit 0 |
| Existing group-write HTTP regression selection | 18 passed, exit 0 |
| Existing gateway-busy HTTP regression selection | 5 passed, exit 0 |
| Focused UI/API/controller/component selection | 413 passed in 14 files, exit 0 |
| Final UI wording/ownership/refusal + EN/DE catalogue checks | 73 passed in 5 files, exit 0; overlaps the selection above |
| Serial fixture TypeScript project | Exit 0 |
| Existing communication fixture TypeScript project | Exit 0 |
| Web production build | Exit 0; existing bundle-size advisory remains |
| Server build (offline, task-private target) | Exit 0 |
| Rust formatting and diff whitespace checks | Exit 0 |
| Isolated browser setup | Exit 1: `Cannot open netlink socket: Operation not permitted` |

The original author's blocked setup occurred before Playwright execution; no
browser cases or screenshots ran in that sandbox and no isolation was weakened.
The controller's subsequent browser results belong to the source epochs named
below. One existing corpus test was ignored by the monitor selection; that
focused run was not private-corpus acceptance. No hardware or live bus access
occurred. Original failures and raw command exits remain retained.


## Correction-focused execution (4 November 2026)

| Check | Observed result |
| --- | --- |
| Serial mounted HTTP selection, including F1–F3 witnesses | 16 passed, exit 0 |
| Existing monitor/admission/stop HTTP selection | 39 passed, 1 corpus test ignored, exit 0 |
| Legacy IP group-write HTTP selection | 18 passed, exit 0 |
| Gateway-busy HTTP regression selection | 5 passed, exit 0 |
| Final UI/API/controller/components, notation and EN/DE catalogues | 463 passed in 17 files, exit 0 |
| Serial and communication fixture TypeScript projects | Both exit 0 |
| Web production build and server build | Both exit 0; existing Web bundle-size advisory |
| Rust formatting check | Exit 0 |

The initial formatting controller found the shared build lease held and ran no
Cargo command (controller exit 125). A separate bounded lease attempt acquired
it and completed formatting and all selected Rust checks. Raw earlier evidence
was preserved. The author did not retry correction browser setup inside its
restricted sandbox. The controller subsequently exercised the corrected source
under its verified loopback-only namespace, without a less-isolated fallback.
The earlier 24/24 default and 24/24 production browser results, mode-consistency
control and nine-observation server smoke remain pre-correction evidence.

## Parent verification of the correction epoch

- Independent correction source/test review: **APPROVE**, F1–F4 closed; no open
  Critical/Important findings. The one Minor achievement-text issue is corrected
  with transport-neutral EN/DE wording, not a new transport capability.
- Corrected-source default and production browser runs: **24/24 each**, with
  strict API fixtures, exact bound read/write payloads and bootstrap-versus-serial
  discovery assertions. All 16 generated configuration/footer frames were viewed.
  Narrow footer captures initially showed only the current viewport portion;
  executed actions were reachable. The capture successor scrolls the footer
  into view before taking the frame, without weakening functional assertions.
- All **27** maintained derived TypeScript consumer projects passed.
- The exact built corrected server passed **12** isolated HTTP observations and
  clean shutdown (exit 0): strict invalid bodies, passive/active missing-port
  failures with distinct conservative uncertainty, no false session, and bound
  no-session refusals. Serial opens targeted only guaranteed-absent task paths;
  no physical tty was accessed by this smoke.
- Isolated compiled Rust controls/restored controls: **8/8 each**. Ignoring the
  session id failed the stale-A action test; releasing serial cleanup early
  failed all **four** bound/legacy × clean/failed cleanup tests. Both variants
  compiled and failed their intended assertions; product sources stayed intact.
  The first mutation setup refused before Cargo execution because its script
  expected two textually identical guards; the corrected attempt matched both
  actual forms and reused the unchanged prepared copy. This is not a product
  failure or a semantic test result for the refused attempt.
- The earlier UI mode-response mutation separately passed its seven-test
  control/restored selection and failed the targeted mismatched-mode assertion.
  Those tests are filtered selections, not additional whole-suite totals.

The final copy/capture-only successor changes neither transport nor lifecycle
logic. No hardware, live bus, firmware, VM, release or deployment action.

## Package-gate epochs (4 November 2026)

The frozen branch `b72315667` passed every selected package-gate stage (15/15),
with checked, untouched user data. That run passed 1,733 scoped Rust tests
(72 ignored), 51 separate corpus cases, 5,099 UI tests, 1,095 default browser
cases and 92 communication-production browser cases. These are separate lane
counts, not a deduplicated total or whole-workspace/release acceptance.

The first main-first merge `1e54ec529` stopped in default functional browser
acceptance: 1,025 passed and one German report-problem keyboard case failed.
Its retained trace shows Enter with `html lang=en` and no matching commands;
German rendering arrived only afterwards. Later stages did not run, and no
publication followed that failed attempt.

The fixture successor waits for the actual active language, focused query and
exact selected command. The shared palette helper also awaits its localized
menu entry. Production behavior, retry policy, timeouts, keyboard-only input,
consent/save and strict unexpected-request assertions are unchanged. Both
keyboard cases passed five repetitions each with retries disabled; the complete
21-case file passed separately. These focused populations overlap and do not
replace the final integrated gate or make the failed attempt green.

Fresh main-first acceptance, remaining exact-build production consumers, final
screenshot readback and publication are still pending.
