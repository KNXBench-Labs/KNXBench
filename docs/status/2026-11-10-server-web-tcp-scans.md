# Server/Web TCP line scans — bounded candidate

Date: 10 November 2026. Status: source review closed and focused checks accepted;
integrated package acceptance and publication remain pending. Not part of Alpha 20; no release,
deployment, real-bus operation or gateway interoperability is claimed.

## Scope

The server's recorded line-scan constructor can explicitly select TCP and then
uses the existing DD0 scanner, exclusions, deadlines and six outcome meanings.
The Web scan selector is local to the next scan and starts at UDP regardless of
the monitor preference. No settings/schema, protocol codec, device programming,
download, serial, discovery or routing behavior is widened. Broader WP-29 and
KNX IP Secure remain open.

## Required contracts implemented by the candidate

- `POST /api/bus/scan/start-tcp` takes exactly the complete scan plan, including
  the gateway. The legacy start stays UDP. TCP never retries UDP, and the
  actual tunnel must report TCP before probing. Estimation remains pure.
- Start/results/cancel carry session ID, server incarnation, actual transport and
  `skippedFrames`: measured nonnegative integer for TCP, null for UDP.
- New actions use `cancel-bound`, `comparison-bound` and `reconcile-bound` with
  the exact canonical query pair. Stale pairs fail under the effect guard;
  legacy unbound actions refuse TCP. Reconciliation still uses existing domain
  commands and undo, and never happens automatically.
- Starts and cancellations refuse held reservations immediately. Server-owned
  tasks retain pending connect, cancellation joins and consuming cleanup despite
  a dropped HTTP request. Shutdown admission is rechecked inside the reservation.
- Running includes disconnect cleanup. Results retain probe errors independently
  from required `disconnectConfirmed` and `cleanupError`. Failed cleanup is not
  a confirmed disconnect and does not erase collected evidence.
- Live skipped-frame counts come from the actual tunnel. Terminal counts freeze
  immediately before consuming disconnect; they exclude cleanup traffic, are not
  telegram-loss/occupancy counts and do not export skipped frame bodies.
- Navigation does not cancel server-owned scans. Explicit Cancel while Start is
  pending targets only its eventual returned identity, even after detach. Late
  callbacks cannot publish into a remounted successor; missing metadata locks
  actions rather than guessing legacy UDP. A lost Start reply is not permission
  to retry or invent an identity.

## Independent review and corrections

Both initial source-review axes blocked acceptance. They identified a lost-Cancel
window between Start acknowledgement and its first attached poll, hidden cleanup
uncertainty at the download-page Cancel caller, unsafe status-value coercion,
a successor Start overlapping reconciliation, and locked editing after definite
no-effect Start refusals. Bounded corrections and owning regression schedules
passed independent bounded source closure on both review axes and parent-verified
focused execution. These findings are source-established, not claims of an
original published-source runtime reproduction. Initial blocked reports remain
historical; integrated acceptance is still required.

## Authored verification and remaining acceptance

Owning Rust tests include strict no-I/O admission, actual-transport refusal,
identity/restart isolation, pending connect/drop/shutdown, blocked and failed
cleanup, terminal ordering and exact live/frozen skips. The new nested TCP suite
includes a real HTTP listener through the production app and RealConnector to a
scripted TCP peer with a same-port UDP trap, exact DD0 masks and exclusions,
positive confirmation versus silence, malformed/closed/refused peers and
observed socket EOF. Recording checks require the line-scan operation and no
skipped body export.

Web tests cover strict runtime admission/routes, scan-local choice, pending
Cancel/detach, stale success/error fencing, tab remount and the download page's
second bound cancellation caller. The communication/default browser selectors
include the new EN/DE wide/narrow scan spec. A proving run can supply captured
actual backend start/poll/cancel JSON; authored controls are explicitly not
captured-backend evidence.

Parent independently accepted the source-bound focused correction checks and
actual backend/browser captures. The private receipts retain raw exits and source
hashes, including the separately classified failed harness attempt. The isolated
privacy calibration and maintained change-scoped integrated gate remain pending;
focused evidence alone does not authorize publication. Existing earlier acceptance
records have not been rewritten to credit this successor.

[WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md) ·
[TCP contract](../contracts/tcp-tunnelling.md)
