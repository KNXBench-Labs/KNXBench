# Server/Web TCP service control — source candidate

Date: 15 November 2026. Source candidate only; not reviewed, accepted, released or
deployed. Loopback and simulator evidence only: no real-gateway interoperability.

## Contract

- New strict `POST /api/device/service-control/tcp/read` (address, gateway, binding) and
  `POST /api/device/service-control/tcp` (the legacy write body plus `binding`), both
  `deny_unknown_fields`. `binding` is the project snapshot (`serverIncarnation`,
  `projectIncarnation`, `revision`); a stale or foreign one answers 409
  `serviceControlContextChanged` before any I/O. The legacy `GET`/`POST
  /api/device/service-control` routes and bodies are unchanged, stay UDP and carry no
  transport field, so an old server answers 404/405 for the siblings instead of silently
  using UDP.
- Order unchanged, with the Debug setting first: setting (403, before binding and I/O),
  binding, address/gateway/phrase validation, key plan, gateway reservation, activity,
  connect. Locks, lease, activity, `stop_admission`, cancel and timeouts are the legacy
  code path. A TCP operation opens TCP only; no UDP retry, reconnect or replay.
- The TCP answers carry `connection` (`requestedTransport`, `actualTransport`,
  `assignedAddress`, `skippedFramesBeforeDisconnect`), sampled from the real handle after
  the operation and before disconnect, independent of recording. Failures carry it in the
  error body (`kind` `serviceControlFailed` after a tunnel existed; `gatewayConnectFailed`
  with all facts null when none did). Legacy answers and errors stay without it. The type
  is the comparison route's `ComparisonConnection`, shared.
- Recordings: TCP settings carry `requestedTransport: "tcp"` and a `connection` note.
  Legacy-route recordings are unchanged (no `requestedTransport`, no note), as for the
  diagnostics route. Backup files are byte-identical in shape: the test checks they contain
  no transport, address or connection facts.
- Web: a service-control-local UDP/TCP select (UDP for every fresh operation: on mount,
  after project change and after a write; changing it drops the reading and review, nothing
  is contacted), frozen into the review. TCP calls only the new routes through
  `serviceControlApi.ts`; 404/405, a 2xx without the `connection` key, wrong or unopened
  facts on a 2xx, or malformed failure facts are refusals, never a success and never a UDP
  retry. The TCP choice is disabled while no project snapshot exists. Metadata validation is
  `tcpConnectionFacts.ts` (shared), imported by `serviceControlApi.ts`, not by `api.ts`.

## Evidence

Retained by the author outside the repository; the parent owns the complete gate and the
independent review. The Rust HTTP test (`tests/it/http_service_control/tcp.rs`) drives the
real `RealConnector` over loopback TCP with the shared package-3 peer: read then write with
and without skipped frames, and peer EOF during the read (four skipped frames, exact
reason). Refusal and control tests use a refusing connector; the legacy routes are only a
refused-connect selection control, not a completed UDP operation. The browser lane replays
the producer's captured bodies only when `KNX_SERVICE_CONTROL_HTTP_CAPTURE_DIR` is set (the
Rust test writes `read`, `write`, `eof` and `refused-read` there as `{status, body}`);
otherwise hand-written fallback bodies are used, so routine runs do not prove
producer/client wire agreement. `service-control-tcp.e2e.ts` runs in the production
communication lane and the development service-control lane, en/de at 1440 and 400 px.

## Limits

Experimental. Skipped-frame counts are pre-disconnect samples, not telegram loss or final
wire totals; their sampling order relative to disconnect is not pinned by a discriminating
test. A UDP operation reports no facts. The server's checks and the activity history of a
write are those of the legacy route; a TCP write after a transport failure still records
"effect unverified" like UDP. No KNX IP Secure, no programming, no restore. No real gateway
was contacted.
