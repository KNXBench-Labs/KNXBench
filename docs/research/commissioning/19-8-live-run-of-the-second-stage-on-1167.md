# 19.8 Live run of the second stage on `1.1.67` (2026-09-29)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="198-live-run-of-the-second-stage-on-1167-2026-09-29"></a>
<!-- BEGIN preserved record-2-017 -->
### 19.8 Live run of the second stage on `1.1.67` (2026-09-29)

User go: *"Freigabe fuer alle Tasks auf der Testhaedware"*. Raw logs:
`OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-29_{pre-live2,k12-*,k15-*,post-k15}.txt`.

- **Before.** Read-only dump, 180 lines, identical to the last dump after
  K7 (option C still on the device).
- **K12 `[V]`.** `PID_SERIAL_NUMBER` = `0083:7A8213CF` (manufacturer
  `0083h` matches). MP §2.4 broadcast read answers from `1.1.67`. MP §2.5
  write to `1.1.68`: no effect, read-back still from `1.1.67`, scan
  unchanged. `PID_SERVICE_CONTROL` = `0000h`, bit 2 clear, which RES §4.2.8
  says forbids address changes by programming mode *and* serial number;
  the device nonetheless accepts the programming-mode path (K6). Recorded
  as device behaviour (KL §139).
- **K15 `[V]`.** Partial download of the parameters, 11 steps, 394 octets,
  all checks and read-backs passed; restart unacknowledged as usual;
  dump after 40 s identical, load states `01 01 01` (KL §142).
- **Not run, and why.** K13 (MP §2.18 address reset) and K14 (Master
  Reset) stay refused on hardware. K13 resets every device in programming
  mode to `FFFFh` and needs a button press and a re-addressing afterwards;
  K14 on `0701h` cannot pass MP §3.7.3's support check and got no response
  even to Erase Code `01h` before. Opening either scope is a code change to
  the hardware allowlist and asks for its own decision. RF (K16/K17):
  no RF hardware.


<!-- END preserved record-2-017 -->
