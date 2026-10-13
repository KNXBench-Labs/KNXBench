# Monitor capture retention

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-137)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 137. A bus-monitor JSON capture is a retained window, not a complete trace

**Limitation.** The panel keeps at most 1000 telegram rows. Pause suspends
client polling, not the server's finite ring: a gateway burst may overwrite
older rows before Resume. Continued monitoring may also evict rows already
seen in the panel. The export records `serverDroppedBefore` and
`clientPrunedCount` separately, but cannot recover either category of lost
telegram. Statistics cover retained real rows only, not all traffic or
filtered table rows. Browser and desktop exports refuse a document over
16 MiB. Native WebKitGTK layout and save-dialog interaction have not been
exercised for this feature.

**Cause.** Both the server ring and browser capture are deliberately
bounded; a local snapshot cannot synthesize data that was never retained.
The native writer validates a bounded JSON document and writes atomically,
but automated tests cannot prove the running desktop dialog on every Linux
window system.

**Cost.** Use the exported JSON for a diagnostic window, not a full audit or
permanent bus history. The file contains source/destination addresses and
payloads; handle it as private installation data. A long paused period can
produce a visible server gap even though the session never disconnected.

**Lifted when.** Full-history capture would require a separately specified,
resource-bounded streaming/storage workflow and its own privacy policy; it
is not an extension of this local snapshot. A GUI-capable Linux desktop
check can independently verify the dialog and responsive layout. Live-bus
acceptance remains separate from the mock/simulator evidence here.
<!-- END retained catalogue record -->
