# Optional KNXnet/IP TCP tunnelling — source admission

Date: 8 November 2026. Scope: the protocol basis for [WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md), not hardware interoperability or complete conformance.

## Sources and method

**[V]** The original authorised local KNX Standard PDFs were independently read
and freshly extracted, with the key original pages rendered and visually checked.
The printed page numbers below were checked against each document's own footer;
for the inspected pages, physical and printed page numbers coincide. Licensed
pages, extracts and raw public responses remain private; this note paraphrases
rules rather than reproducing specification prose.

| Short name | Original document | SHA-256 |
| --- | --- | --- |
| Core | `03_08_02 Core v01.06.02 AS.pdf` | `7c61a3b08e9ecc681e998727661071f8b0799c93d2ea4623f0d03dc95b4ac929` |
| Tunnelling | `03_08_04 Tunnelling v01.07.01 AS.pdf` | `36d336d2ac8008ac81210b8bdf32ba6788b39c28c62220bfe54ab4a50d3601fd` |

This is a bounded source review, not a complete audit of either volume.

**[D]** Public KNX documentation was retrieved independently through the public
support API; HTTP 200 article bodies and their digests were retained. The
successful method was an HTTP GET, not the separate failed extraction attempt.
[Connection options](https://support.knx.org/hc/en-us/articles/360012124579-Connection-options)
mentions heartbeat for UDP and (secure) TCP, but supplies no stream grammar or
precise timers. Its retrieved body SHA-256 is
`235a31c579b43dae33ee0c18c4108f042b91eca3b063bc7dccb48a3f20cfd2ed`.
The explicit TCP/UDP choice in
[Connection Manager (Detailed)](https://support.knx.org/hc/en-us/articles/4402353231762-Connection-Manager-Detailed)
belongs to **EIBlib/IP**, not this KNXnet/IP contract; it is not used as protocol
evidence. Its retrieved body SHA-256 is
`d555e48663f145424c6f6e180ff6d5994bc371b009ad155941668b748beb9b1c`.

## Admitted rules

All rows are **[V]** observations of the identified original clause bodies,
except consequences explicitly marked **[A]**.

| Area | Rule in own words | Original clause / printed page |
| --- | --- | --- |
| Optional selection | UDP remains mandatory; TCP is an optional addition. A connection uses the transport of its request, not an automatic fallback. The client initiates TCP and must not assume several concurrent sockets are supported. | Core §8.4.1 p.43; §8.4.3.1 p.44 |
| Endpoints | A TCP-capable control endpoint uses the same port number for UDP/TCP. Discovery-endpoint traffic remains UDP; the discovery response's UDP control HPAI is not proof of TCP support. | Core §8.6.3.1–.2 p.50; §7.6.2 p.32 |
| TCP HPAI | The only valid IPv4/TCP HPAI is eight bytes `08 02 00 00 00 00 00 00`. It means use the existing stream, not open a callback. Both request HPAIs are TCP Route Back; the successful server data HPAI must match that Route Back type. | Core §8.6.2.1–.2 pp.49–50; §8.4.3.4.3 p.46 |
| Setup | CONNECT still negotiates an inner channel and assigned KNX address. The wire order is control HPAI, data HPAI, CRI. A successful response supplies channel, status, data HPAI and CRD; refusals need not carry the success blocks. CONNECT response budget is 10 s. | Core §5.2 p.12; §7.8.1 Fig.35 p.38; §7.8.2 pp.38–39; Tunnelling §5.4.2–.4 pp.25–26,30 |
| Framing | The ordinary six-byte 1.0 header contains service and total length in network byte order. No stream padding is added. Reads need not align with frames: collect a header and then exactly its declared body. **[A]** Total length below six is structurally invalid for this header. | Core §2.1.2 pp.7–8; §7.1–.2 p.20; §8.4.3.3 pp.45–46 |
| Bounded reception | A malformed header closes the stream. An otherwise well-formed frame exceeding the local receive buffer is drained and ignored, not a reason to close. Unknown services are likewise drained. Skipping must work through total length 65535 without allocating that full length. | Core §6.2–.3 p.15; §8.4.3.3 p.45 |
| Channel and sequence | Keep the server-assigned channel ID; it is not forced to zero. TCP receivers ignore sequence values; a sender may use sequence zero. | Core §8.4.3.4.1 p.46; Tunnelling §2.6.2 p.10; §5.4.5–.6 pp.30–31 |
| No UDP ACK policy | TCP sends no TUNNELLING_ACK and ignores received ones. UDP ACK waits, retransmissions and sequence deduplication do not apply. cEMI confirmations and application replies remain separate and necessary to their workflows. | Tunnelling §2.4–.6.2 pp.9–10 |
| Heartbeat retained | Inner connection monitoring still applies over TCP: 60 s interval, 10 s response budget, three repeats after the original attempt. Wrong-channel responses are not the tunnel's heartbeat. | Core §5.4 p.14; §8.4.3.4.2 p.46 |
| Distinct idle rules | Without an active inner connection/session, byte-idle reaches 10 s and resets on every byte. With an active inner connection, do not close solely because a partial header/body stalls for 10 s. A timeout of the last inner connection closes the outer stream immediately. | Core §8.4.3.2 p.44; §8.4.3.3 pp.45–46 |
| Termination | A received TCP half-close terminates the entire connection immediately and discards pending requests. Normal client teardown closes the inner connection first, then TCP. Answer server DISCONNECT_REQUEST when possible, but never depend on a clean server ordering or a delivered indication. | Core §5.5 p.14; §7.8.5–.6 p.41; §8.4.3.2–.3 pp.44–45 |
| Socket ownership | Inner channel IDs belong to their one TCP stream; a replacement socket cannot reuse a prior channel. Closing TCP closes its inner connections. Multiplexing is permitted by the format, not required for the one-tunnel increment. | Core §8.4.3.5 p.46 |

## Source discrepancies and limits

- Core §7.8.1 prose and Fig.35 disagree on CRI/data-HPAI order. The inspected
  **binary figure** independently supports the existing correct control/data/CRI
  order; no change based on the prose order is admitted.
- Tunnelling §6.1 Fig.15 p.34 has a connection-header length label inconsistent
  with the four fields defined in §5.4.5–.6 pp.30–31. Use the defining four-byte
  structure, not the erroneous example label; do not infer a universal cEMI
  message code from an example caption.
- Core §6.2–.3 p.15 calls for `E_VERSION_NOT_SUPPORTED` handling, but this
  inspection does not establish a universal service-independent negative TCP
  message. Tunnelling forbids a TCP TUNNELLING_ACK. A local fail-closed unsupported-
  version policy is not a claim of complete service-specific error conformance.
- A separate normative graceful-disconnect, TCP dial or write-response timeout
  was not found in the inspected clauses. Any bounded cleanup/write/dial budget
  is a **local product policy**, not an invented KNX timer. CONNECT and heartbeat
  response budgets are independently specified above.
- A completed TCP socket write proves neither physical KNX delivery nor device
  execution. Cancellation after a partial write must abort the stream or preserve
  serialization state; it never authorises automatic reconnect/replay.
- Plain TCP is not KNX IP Secure. No gateway TCP capability, hardware result,
  certification or remedy for S2-04 is inferred from this research.

## Executable acceptance still required

The implementation must exercise the real TCP socket path against loopback
peers: fragmentation/coalescing, exact route-back/channel bytes, drain-and-
continue after oversized/unknown frames, no ACK or duplicate suppression,
heartbeat independent of a stalled partial frame, EOF/half-close, refusal,
matching channel, cancellation, complete cleanup and redacted wire recording.
UDP defaults and existing behaviour remain regression requirements.

The library increment and later CLI/server/Web integration have separate
acceptance boundaries. Source admission alone does not make TCP selectable in
the application or close WP-29.
