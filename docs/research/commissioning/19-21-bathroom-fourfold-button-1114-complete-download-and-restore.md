# 19.21 Bathroom fourfold button 1.1.14: complete download and restore (2026-10-09)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="1921-bathroom-fourfold-button-1114-complete-download-and-restore-2026-10-09"></a>
<!-- BEGIN preserved record-2-030 -->
### 19.21 Bathroom fourfold button 1.1.14: complete download and restore (2026-10-09)

With the maintainer's go for the proposed test on `1.1.14`, KNXBench's
existing CLI downloaded the house project's configuration to the MDT
fourfold button (mask `0701h`, manufacturer `0083h`, program
`M-0083_A-0026-15-3591`). Source revision `b3ee55a0`, isolated project and
product database imported from the same ETS 6.3 project. No access key;
no changes to the executor or protocol. **[V]**

- **Read-only baseline:** the complete address segment `4000h`–`4200h`,
  association segment `4201h`–`43FFh`, application segment
  `4400h`–`461Bh`, and six load-state octets `B6EAh`–`B6EFh`: 1570 octets.
  All 1562 configuration octets comparable to the 2026-09-29 reading
  match; physical address is `1.1.14`; the three affected parts are Loaded.
- **Fresh compare:** 57 of 1562 planned octets differ, each a group-object
  descriptor config octet. Two are the documented C-bit difference on
  active unlinked objects; 55 are inactive-object config differences
  (§19.13). No parameter, active association or group-address difference.
  This is not proof of a newly changed light configuration: the run tests
  the load sequence with the existing parameter values and links.
- **Download:** 11:10:47–11:14:04 CEST, 25 steps, 1562 memory octets,
  every write read back, three parts Loaded. The pre-write backup is
  identical to the separate baseline at all 1562 covered octets. Basic
  Restart did not receive T_ACK: `RestartOutcome::Unconfirmed`, not a
  confirmed restart. No separate restart or power cycle was performed.
- **Independent post-download readback:** after the restart wait, all 1570
  selected octets equal the expected image and retained physical-address
  and load-state bytes. A fresh CLI compare is clean (exit 0).
- **Functional check:** during the user-operated check, `1.1.14` sent
  `DPST-1-1 on/off` to the main light (`1/0/11`) and mirror light
  (`2/0/43`) between 11:25:48 and 11:25:51 CEST; the user replied
  "Erledigt". The monitor also captured three user-originated blind
  up/down/stop telegrams on the project's configured groups. These extra
  telegrams do not establish physical blind movement; no additional
  software-driven group write was made. No telegram from this device
  addressed an unconfigured group in the retained monitor interval.
- **Restore:** same 25-step product procedure, its own pre-restore backup,
  11:27:39–11:30:49 CEST, 1562 octets read back, three parts Loaded,
  Basic Restart again unconfirmed. After the wait, a separate dump
  equals the pre-dump at all **1570** selected octets (0 differences);
  before/after-restore CLI comparisons are byte-identical. This proves
  restoration of these configuration regions and selected load-state
  bytes, not all device memory, properties or volatile operating state.
- **Evidence scope:** the third shipped program entry, `complete` only.
  House readiness: 11 verified / 21 untested / 2 unsupported / 1 excluded.
  `1.1.15` inherits the program grade, but was not contacted or downloaded;
  its unexplained configuration differences in §19.13 are not resolved.
  UI and partial downloads were not exercised in this run.
- **Private artefacts:** `OriginalData/DeviceBackups/2026-10-09_button-114-live/`
  holds independent dumps, compare logs, download/restore plans and runs,
  both backups, timestamped monitor, activity history and the read-only
  probe source. No raw project configuration is committed.


<!-- END preserved record-2-030 -->
