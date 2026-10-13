# ADR 0123: Gateway refusals are structured, and name only what is known

Date: 2026-10-13
Status: Accepted implementation decision
Origin: GitHub issue #10 (reported by thengsty), KL-180

## Context

A tester could not program a device: the bus monitor still held the gateway's
one tunnel, and the download tab learned so only after **Start**, from an
English server sentence ("stop the bus monitor first: the gateway serves one
tunnel"). Refusals were `ApiError { error, kind? }` with free text; the client
could only show that text. A gateway's own refusal surfaced as `status 0x24`.

Verified: the server knows its own sessions (download, address programming,
bus monitor, line scan) and their locks; the activity snapshot (ADR-0055)
already exposes them read-only. The CONNECT_RESPONSE status codes and their
meanings are in Core v01.06.02 §7.8.2 Table 8 (p. 39) and Tunnelling
v01.07.01 §2.2.2 (p. 6, 25h) and §5.4.3.1 Table 11 (p. 26, 29h). Not known: who
else holds a gateway's tunnels; the server cannot see other clients.

## Decision

1. **Structured bodies next to the unchanged text.** `ApiError` gains optional
   `details` (only together with `kind`), flattened into the JSON body. The
   `error` text stays exactly as before, so older clients and scripts keep
   working.
2. **`409 gatewayBusy`** for every tunnel start refused by another KNXBench
   session, with `holder` tagged by `kind`: `busMonitor` (`sessionId`,
   `gateway`, `state`), `lineScan` (`sessionId`, `probed`, `total`),
   `deviceDownload` (`address`, `state`), `addressProgramming` (`address`,
   `state`), or `connecting` (`session`: the lock name of the activity
   snapshot's `busyLocks`) when only a held lock is visible. Nothing about a
   target is guessed.
3. **`502 gatewayRefused`** (`gateway`, raw `gatewayStatus`, `gatewayError`:
   the specification's constant or `null` for a code the specification does not
   define there) and **`502 gatewayTimeout`** (`gateway`) for a failed connect;
   other transport failures stay a plain `502`. `knx_net::core::services::
   connect_refusal` holds the named codes with their sources.
4. **`422 unsupportedByteOrder`** (`address`, `order`) for the KL-170 refusal,
   typed as `ImageError::UnsupportedByteOrder` and graded `byte-order` in
   readiness.
5. **The client words what it can admit.** `gatewayRefusal.ts` admits each body
   strictly (exact keys, valid addresses/endpoints, status range) and words it
   per kind in the UI language through `errorMessage`; anything else keeps the
   server's text. The download tab reads the activity snapshot to say up front
   what holds the gateway.
6. **The live activity snapshot names the bus monitor's gateway** (ADR-0055
   follow-up). History and one-shot records still carry no gateway (ADR-0064).

## Alternatives considered

- **Reword the server text only** (German/English on the server): the server
  has no UI language, and clients could still not tell a busy gateway from a
  broken one.
- **A new error type per route**: more surface, the same information; one
  shared `holder` keeps every tunnel route's answer alike.
- **Naming the other client** (ETS, visualisation): not observable; the
  gateway's status code is the only evidence, so that is what is named.
- **Retrying 25h as Tunnelling §4.3 describes** (management connection, reading
  the additional individual addresses): a separate protocol feature, not
  verified against hardware; the code is named and the tester pointed at the
  tunnel addresses instead.

## Consequences

- Every new tunnel-opening route must use `tunnel_holder::gateway_busy` and
  `gateway_connect_failed`; `http_gateway_busy.rs` and the existing per-route
  `409` tests pin the shape.
- New holder kinds or codes need a client admission and wording, or the client
  falls back to the server text (by design, never a guess).
- The monitor's gateway is now visible to every client that reads the live
  snapshot (same server, logged-in session).
