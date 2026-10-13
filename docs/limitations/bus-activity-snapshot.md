# Bus-activity snapshot

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-bus-activity-snapshot)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Partial commissioning bus-activity snapshot (ADR-0055)

`GET /api/bus/activity` is a read-only **partial**, server-lifetime snapshot,
not an audit log or proof of device mutation. It reports retained download,
button-programming, monitor and line-scan sessions plus bounded one-shot
read and service-control-write records. Short actions can be missed by polling;
an empty session list does not prove the gateway is idle. Held locks need not
identify an operation or target. The one-shot ring can evict records and
reports `oneShotDropped`; restart loses them. Serial-address and group writes
do not supply durable activity receipts. A `serviceControlWrite` entry
distinguishes no-change, not-sent, verified, unverified and unknown outcomes;
its backup/send flags do not prove transmission or a whole-device recovery.
No property octets, keys or host backup paths are disclosed. The global Web
status bar and per-action history remain open (ADR-0055/0056); simulator
evidence is not a live bus check.

**Web adoption, 2026-10-06 (UI owner, UI-04).** Bus tools → *Live activity*
reads this snapshot every 2 s while the tab is open and the page visible:
held sessions with their state and progress, short operations and the
eviction count, busy locks named as "operation unknown", the history-storage
state (`unavailable` as an alert), the untracked kinds, and a note when the
server incarnation changes. It always says the view is partial and volatile
and that an empty list does not mean an idle bus. A snapshot with an unknown
field, state, lock or out-of-range count is refused as a whole; a failed poll
clears the last snapshot. *Activity history* now reloads its first window on
its own while that window shows a running operation; after paging further it
says how to see the latest state instead. **Not built, by owner decision:** a
global status-bar indicator. Polling the server from every view for a list
whose emptiness proves nothing would add load without adding certainty; the
tab is where the evidence and its limits can be read together. Native
WebKitGTK/Orca acceptance stays an accepted boundary (user decision
2026-10-04).

**Durable backend candidate, 2026-10-03 (ADR-0064).** A separate versioned
SQLite history now retains four one-shot operation kinds across ring eviction
and restart; seven kinds remain explicitly untracked. Previous-incarnation
running rows become interrupted/unknown, never success. Storage/metadata
refusal is visible and latches unavailable history; download and Debug-write
starts refuse before tunnelling when history cannot be admitted. Property
intent persistence still follows the PID 8/PID 14 recovery and precedes the
first property write. This is metadata only, not a recovery image, complete
download journal or a shipped global UI. Whole-candidate acceptance and
publication remain pending. The exact consumer contract and residue are in
[COMMISSIONING_ACTIVITY_HISTORY](../contracts/COMMISSIONING_ACTIVITY_HISTORY.md) and
[COMMISSIONING_ALPHA_LEDGER](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/COMMISSIONING_ALPHA_LEDGER.md).

**Admission correction, 2026-10-03.** Late synthetic review reproduced SQLite
hot-journal rollback before a foreign-format refusal. Corrected openers first
admit existing identity/schema read-only; WAL is explicitly unsupported and
header-refused before SQLite can create shared-memory sidecars. Hot-journal and
WAL RED/GREEN cases preserve main/journal/WAL bytes; 21 current-source compiled
behavioral mutants fail as expected and restoration is exact. Preserve failed
storage and its sidecars for explicit repair, not automatic rollback/conversion.
Hostile concurrent path replacement is not covered. Earlier full gate receipts
belong to their older source; corrected integrated acceptance is still pending.

**Update 2026-10-06 (user decision, AR18):** with the Web half delivered
(`892b9948`), the user accepted the remaining `UI-04` row as a disclosed
Alpha boundary. The snapshot stays partial as described above; no broader
history or live-bus claim follows.
<!-- END retained catalogue record -->
