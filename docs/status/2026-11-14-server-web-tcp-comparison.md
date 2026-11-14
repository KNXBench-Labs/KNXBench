# Server/Web TCP device comparison — source candidate

Date: 14 November 2026. Source candidate only; not reviewed, accepted, released or
deployed. Loopback and simulator evidence only: no real-gateway interoperability.

## Contract

- New strict `POST /api/device-compare/tcp` (`deny_unknown_fields`, binding
  required). Legacy `/api/device-compare` keeps its request, response body (no
  `connection` field), admission and UDP behaviour. Its operation recordings gain
  two additive fields, because both routes share the recording helper: a
  `connection` note and `requestedTransport: "udp"` in the settings. Both share one comparison implementation; admission,
  reservation, read recipe and observation lifecycle are unchanged.
- Transport reaches the connector through the existing recorded-transport seam.
  Recording settings carry `requestedTransport` only; actual facts are noted after connect.
- TCP responses and post-connect errors add `connection`: `requestedTransport`,
  `actualTransport`, `assignedAddress`, `skippedFramesBeforeDisconnect`, sampled
  from the real handle before disconnect, independent of recording. Refusals
  before connect report `actualTransport: null` and null facts (unavailable, not zero).
- Web: compare-local choice, UDP default per fresh review, no I/O on selection,
  gateway and transport frozen for the queue. TCP calls only the new route; 404/405
  or missing/mismatched metadata never retries UDP and never counts as success.
  Cancelling a queue does not prove the pending HTTP read stopped (unchanged).

## Evidence

Retained by the author outside the repository; parent owns the complete gate and
independent review. Loopback tests pin literal same/different bytes and the
management/outer disconnects; the browser fixture replays the producer's captured
bodies, but only when `KNX_COMPARE_HTTP_CAPTURE_DIR` is set. UDP controls are refused-handshake selection only, not completed UDP reads.

## Limits

Experimental. Skipped-frame counts are pre-disconnect samples, not telegram loss or
final wire totals. The Rust test writes the producer's HTTP bodies into the directory
named by `KNX_COMPARE_HTTP_CAPTURE_DIR` and the browser lane reads them from there;
without the variable (routine runs) the browser lane uses hand-written fallback bodies,
so routine runs do not prove producer/client wire agreement. Skipped-frame sampling
order relative to disconnect is not pinned by a discriminating test. No KNX IP Secure. Writes, programming and restore are out of scope.
