# 2026-09-16 — Task 16: passive Group Monitor real-gateway verification

## Safety boundary

This task was authorized for passive monitoring only. It sent no
`GroupValueRead`, `GroupValueWrite`, `GroupValueResponse`, management request,
address scan, or other cEMI request. It did not call `/api/bus/write` or any
CLI send/scan command. Individual address `1.1.220` was never targeted,
queried, written, or included in a range.

Opening and maintaining the tunnel necessarily used KNXnet/IP control traffic:
connect, connection-state checks, tunnelling acknowledgements for received
frames, and disconnect. The gateway address was supplied at runtime and is
deliberately omitted from repository files.

## Path exercised

The run used a dedicated source-built `knx-server` on local TCP port 18080:

1. `POST /api/bus/monitor/start` parsed the runtime gateway and called
   `BusSession::start` through `RealConnector`.
2. `BusSession` subscribed to the real tunnel's incoming event receiver.
3. Repeated `GET /api/bus/monitor/telegrams?since=0` calls read only the
   in-memory buffer and counted rows, resolutions, and decode outcomes.
4. `POST /api/bus/monitor/stop` closed each tunnel and returned final telegram
   and drop counts.

The procedure printed aggregate counts only. It did not retain payloads,
gateway coordinates, observed individual addresses, group addresses, or
project group names.

## Measurement 1 — empty project

- Successful start: 2026-09-16 09:05:26 UTC.
- Gateway assigned a tunnel address; HTTP response was 200.
- Session stayed `active` through every poll.
- Successful explicit stop: 2026-09-16 09:07:39 UTC.
- Duration from successful start to stop: 133 seconds.
- Stop response: 52 telegrams, 0 dropped.
- At the 90-second polling checkpoint: 49 rows, 8 distinct sources, 21
  distinct group destinations, 3 service kinds, 0 resolved destination names,
  and 0 decoded values. The zero resolution is expected because this server
  still held its empty startup project.
- No disconnect, reconnect, buffer gap, or warning occurred.

## Measurement 2 — real installation project open

The dedicated server imported the repository's real schema-23 reference
project `OriginalData/DemoProjects/Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj`
through `POST /api/project/import`, then started a fresh monitor session.

- Successful start: 2026-09-16 09:08:33 UTC.
- Gateway assigned a tunnel address; HTTP response was 200.
- Session stayed `active` through every poll.
- Successful explicit stop: 2026-09-16 09:10:20 UTC.
- Duration from successful start to stop: 107 seconds.
- Stop response: 65 telegrams, 0 dropped.
- Final 90-second polling checkpoint: 65 rows from 9 distinct sources to 21
  distinct group destinations.
- Project resolution: 65/65 rows carried a destination name.
- DPT result: 10 decoded values, 0 DPT conflicts, 0 decode errors. The
  remaining rows were valid unresolved service/payload combinations under the
  existing codec/context rules; no claim beyond the counted outcomes is made.
- No disconnect, reconnect, buffer gap, or warning occurred.

## Conclusion and limits

The production tunnelling receive, in-memory buffer, project-name resolution,
DPT-decode, polling, and explicit-stop path worked against one real gateway in
two bounded sessions. This closes only the earlier “never talked to a real
gateway” clause.

It does not verify routing, transmit behavior, active reads, reconnect after a
failure, other gateway models, long-running stability, client-side filtering
depth, multi-session behavior, or browser memory growth. Real transmit testing
would require separate explicit authorization.

## Repository verification

- `cargo fmt --all --check`
- `git diff --check`
- Focused link check confirmed every updated §62 link resolves to the rewritten
  heading. A broad changed-document link scan also found a pre-existing stale
  §11 anchor in `GAP_ANALYSIS_ETS.md`; it is unrelated to Task 16 and was not
  rewritten as part of this verification.
- Focused source audit confirmed the executed endpoint path never calls
  `BusTunnel::send`; that method remains exclusive to `/api/bus/write`.
