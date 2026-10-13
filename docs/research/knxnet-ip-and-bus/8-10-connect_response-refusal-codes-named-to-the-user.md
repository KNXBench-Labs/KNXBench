# 8.10 CONNECT_RESPONSE refusal codes named to the user (issue #10, 2026-10-13)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="810-connect_response-refusal-codes-named-to-the-user-issue-10-2026-10-13"></a>
<!-- BEGIN preserved record-1-039 -->
### 8.10 CONNECT_RESPONSE refusal codes named to the user (issue #10, 2026-10-13)

Read from the local specification PDFs (`pdftotext` text checked against the
printed page footers), for naming a gateway's refusal of a tunnel connect
(`knx_net::core::services::connect_refusal`, ADR-0123):

| Code | Constant | Source | Meaning (summary) |
| --- | --- | --- | --- |
| 22h | `E_CONNECTION_TYPE` | Core v01.06.02 §7.8.2 Table 8, p. 39/60 | connection type not supported |
| 23h | `E_CONNECTION_OPTION` | same | a requested connection option not supported |
| 24h | `E_NO_MORE_CONNECTIONS` | same | maximum number of concurrent connections already occupied |
| 25h | `E_NO_MORE_UNIQUE_CONNECTIONS` | Tunnelling v01.07.01 §2.2.2, p. 6/35 | a tunnel is free, but no unique tunnelling address for it |
| 28h | `E_AUTHORISATION_ERROR` | Core Table 8 (V. 2, extended CRI) | client not authorised for the requested IA |
| 29h | `E_TUNNELLING_LAYER` | Tunnelling §5.4.3.1 Table 11, p. 26/35 | requested tunnelling layer not supported |
| 2Dh | `E_NO_TUNNELLING_ADDRESS` | Core Table 8 (V. 2, extended CRI) | requested IA is not a tunnelling IA |
| 2Eh | `E_CONNECTION_IN_USE` | Core Table 8 (V. 2, extended CRI) | requested IA already in use |

Notes: Table 11 is titled *Tunnelling CONNECT_ACK error codes* and sits in the
Basic CRI section of the connect request; it is read here as the tunnelling
CONNECT_RESPONSE status. 25h is mandatory for servers with more than one
tunnel (§2.2.2); §4.3 prescribes a client procedure for it (management
connection, read `PID_ADDITIONAL_INDIVIDUAL_ADDRESS`, retry), which KNXBench
does not implement. 21h, 26h and 27h are CONNECTIONSTATE_RESPONSE codes
(Core Table 9), not connect refusals, and are not named. Only 24h has been seen from a real gateway
(KNOWN_LIMITATIONS §62); the rest are synthetic tests only.

<!-- END preserved record-1-039 -->
