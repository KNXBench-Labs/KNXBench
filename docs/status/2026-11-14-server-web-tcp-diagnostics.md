# Server/Web TCP device diagnostics

Date: 14 November 2026. Software-accepted on main; not released or deployed.
Loopback and simulator evidence only: no real-gateway interoperability.


## Acceptance

The reviewed source was integrated main-first as `fef417bb9` and publication was
read back exactly. Two independent reviews (Standards, Spec) returned APPROVE with
no blocker. Standards noted one medium finding (a complete success with null actual
transport was admitted; fixed before commit) and three low judgments. Spec noted
two low findings (a dead test setup line, missing TCP cancel test) and ran two
server mutants in an isolated copy (both killed). The package gate passed all 15
selected stages (Rust 1990 passed, 0 failed, 73 ignored; Vitest 5832; 1421 browser
tests; separate corpus 51/51), inputs frozen and user data untouched. The first gate
attempt failed on tool-tests (production spec count 44 vs 45); the fix added the new
spec to the test assertion and passed in the second run.

## Contract

- New strict `POST /api/device-diagnostics/preview/tcp` (`deny_unknown_fields`, same
  body as the legacy preview, binding required). The transport is stored in the job at
  preview time; `start` runs with exactly that transport. Legacy preview, start,
  status, cancel and export routes and bodies are unchanged and stay UDP; an old server
  answers 404/405 for the sibling instead of silently reading over UDP.
- A TCP job opens TCP only: no UDP retry, reconnect or replay. Admission order,
  gateway-busy locks, lease, activity, `stop_admission`, cancel, the 10 s connect
  timeout, the 3 s disconnect and the 900 s / binding freshness are unchanged.
- Views of TCP jobs (and only those) carry `connection`: `requestedTransport`,
  `actualTransport`, `assignedAddress`, `skippedFramesBeforeDisconnect`, sampled from the
  real handle before disconnect, independent of recording, also after a post-connect
  failure (for example peer EOF: `connection-lost`). Preview, refusals and connect
  failures before a tunnel exists report `actualTransport: null` and null facts
  (unavailable, not zero). The type is the comparison route's `ComparisonConnection`,
  shared, not copied.
- Recordings: the TCP route's settings carry `requestedTransport: "tcp"` and a
  `connection` note after the sample. The legacy diagnostics route's recording settings
  are unchanged (no `requestedTransport`, no note), unlike the comparison route.
- Report and export bundles (format 1/2) are unchanged: the test asserts the exported
  bytes of a TCP run equal `bundle_export` of its report alone and the report carries no
  transport facts.
- Web: diagnostics-local UDP/TCP choice (UDP for every freshly opened dialog, changing
  it drops the review, nothing contacted on selection), frozen into the preview. TCP
  calls only the sibling route; 404/405 or missing, mismatched or inconsistent metadata
  is a refusal, never a success and never a UDP retry. UDP calls keep their legacy call
  shape. Metadata admission lives in `diagnosticApi.ts` and the shared
  `tcpConnectionFacts.ts` (also used by comparison), not in `api.ts`.

## Evidence

Retained by the author outside the repository; the parent owns the complete gate and
independent review. The Rust HTTP test drives the real `RealConnector` over loopback TCP
with the package-3 peer (shared by module path, not copied): clean run, skipped frames
before the first reply, and peer EOF during the hardware-type read (four skipped frames,
`connection-lost`). Refusal and control tests use a refusing connector: the legacy
UDP route is only a refused-connect selection control, not a completed UDP read. The
browser lane replays the producer's captured bodies only when
`KNX_DIAGNOSTICS_HTTP_CAPTURE_DIR` is set (the Rust test writes `preview`, `start`,
`complete` and `eof` there); without it, hand-written fallback bodies are used, so
routine runs do not prove producer/client wire agreement.

## Limits

Experimental. Skipped-frame counts are pre-disconnect samples, not telegram loss or
final wire totals; their sampling order relative to disconnect is not pinned by a
discriminating test. While a TCP job runs its `connection` still shows the unopened
facts; actual facts appear when the acquisition ends. No KNX IP Secure. Writes,
programming and restore are out of scope. No real gateway was contacted.
