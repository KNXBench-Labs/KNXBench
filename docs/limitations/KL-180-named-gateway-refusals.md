# Named gateway refusals

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-180)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 180. Gateway refusals name KNXBench's own sessions and the gateway's stated reason, nothing more

**Issue #10 (2026-10-13), [ADR-0123](../adr/0123-structured-gateway-refusals.md).**

- **Who holds the gateway.** A start refused because another KNXBench session
  of the same server holds the gateway's one tunnel names that session (`409`,
  `kind: "gatewayBusy"`, `holder`: bus monitor with its gateway, line scan with
  its progress, download, address programming, or only a held lock). Another
  client (ETS, a visualisation, a second KNXBench server) is invisible to the
  server; it appears only as the gateway's own refusal.
- **Why the gateway refused.** A CONNECT_RESPONSE status is named only for
  codes the specification defines there: 22h, 23h, 24h and the extended-CRI
  codes 28h, 2Dh, 2Eh (Core v01.06.02 §7.8.2 Table 8, p. 39), 25h (Tunnelling
  v01.07.01 §2.2.2, p. 6) and 29h (§5.4.3.1 Table 11, p. 26, titled there
  *CONNECT_ACK error codes*). KNXBench sends a basic CRI, so it does not expect
  the extended-CRI codes. Any other code stays a bare number. The reasons shown
  are plain summaries of the table descriptions. 24h was seen on a real
  gateway during the 2026-09-16 → 19 re-verification (§62); every other code
  only in synthetic tests. For 25h, KNXBench does not run the client procedure
  of Tunnelling §4.3 (a management connection reading the additional
  individual addresses); it names the code and points at the tunnel addresses.
  A gateway that does not answer is `gatewayTimeout` with its address.
- **Up-front hint.** The download page reads the volatile, partial activity
  snapshot (ADR-0055) every 3 s. A failed read shows nothing, and a start's own
  refusal stays authoritative. A held lock without a session (a one-shot
  management read, a session still connecting) is named by its lock only.
  There are stop buttons for a bus monitor, a line scan and an address
  programming; the page's own download is shown by the page. The snapshot now
  carries the bus monitor's gateway in the live view only, never in history or
  one-shot records.
- **Readiness marks** come from the offline readiness grade, re-read after
  every project change. Only `unsupported` and `excluded` devices with a unique
  address are marked; a duplicate address is left unmarked. The mark is a hint:
  preparing the download decides. A failed or malformed readiness answer is
  said in the tab, not silently dropped.
- **Byte order** is still refused (§170); only the wording and the category
  changed. The plan's `warnings` (`base-image-low-octet-first`) are still not
  shown in the tab (KL-170, next in the queue).

No real gateway or device was contacted for this package; Chromium only, the
native WebKitGTK window was not exercised. Ledger `KL-180`.
<!-- END retained catalogue record -->
