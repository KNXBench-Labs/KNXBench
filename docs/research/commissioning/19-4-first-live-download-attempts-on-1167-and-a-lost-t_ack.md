# 19.4 First live download attempts on `1.1.67`, and a lost `T_ACK` (2026-09-28)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="194-first-live-download-attempts-on-1167-and-a-lost-t_ack-2026-09-28"></a>
<!-- BEGIN preserved record-2-013 -->
### 19.4 First live download attempts on `1.1.67`, and a lost `T_ACK` (2026-09-28)

Two runs of the unchanged plan (`apps/knx-cli/tests/live_memory_download.rs`,
gated by `KNX_DOWNLOAD_ADDRESS` and `KNX_DOWNLOAD_CONFIRM`) both stopped
early. Logs: `OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-28_*`
(gitignored).

- **`[V]` Before the run** a fresh read-only dump was byte-identical to the
  morning's backup; load states `B6EAh` = `01 01 01 00`.
- **`[V]` Run 1** stopped at step 13 (the association table's 511 octets):
  one read-back got `T_ACK` and no answer within 3 s. Afterwards the load
  states read `01 02 00 00`, the address table read `02 1143 1035` (option
  C's one group address, `2/0/53`) and the individual address at
  `4001h`–`4002h` was unchanged.
- **`[V]` Run 2** stopped at step 8 the same way.
- **`[V]` The cause, from a frame trace** (read-only, `diag4-frametrace`):
  1. the device answered a read with `T_DATA_CONNECTED seq 1`;
  2. this client's `T_ACK seq 1` has **no `L_Data.con`** in the trace: it
     never reached the bus (a group telegram from another device arrived
     3 ms after the answer);
  3. the device acknowledged the next request (`seq 2`) at once but did not
     answer it;
  4. it repeated the old answer, unchanged, `seq 1`, 6 s after the first
     transmission and then every 3 s.
- **`[D]` That is TL §5.4.1 exactly** (*Transport Layer* v01.02.03, pp.
  17–22). With its answer unacknowledged the device sits in `OPEN_WAIT`:
  it still receives (`E04` → `A2`, acknowledge), repeats its stored frame
  on each acknowledge time-out (`E17` → `A9`) and keeps every newer answer
  behind it (`A11`, *"Don't change order of T_Data_Connected.req
  events"*). The client has to acknowledge the repetition: `E05`
  (`SeqNo_of_PDU == SeqNoRcv - 1`) → `A3`, *"Send an N_Data_Individual.req
  with T_ACK_PDU … sequence = sequence of received message"*, and nothing
  goes to the user.
- **`[V]` The client's two defects.** It gave up on an acknowledged request
  after one 3 s time-out, before the device's first repetition at 6 s. And
  it had no `SeqNoRcv`, so a repetition would have been taken as the answer
  to a new read of the same address.

The fix (`ManagementSession::exchange_inner`, `receive_numbered`):

- **`[D]`** The session keeps `SeqNoRcv` (reset by `A12` on connect) and
  handles `E04`/`E05`/`E06` as `A2`/`A3`/`A4`. A repetition is
  acknowledged and never matched.
- **`[D]`/`[A]`** An acknowledged request waits up to `MAX_TRANSMISSIONS`
  acknowledge time-outs (12 s) for its answer: the length of the device's
  own repetition ladder (`E17`/`E18`). The request itself is still never
  repeated. Using the ladder as the bound is this project's reading; no
  PDF read states a client-side figure for it.
- **`[D]`** The load-state wait loop keeps one time-out per read. RES
  §4.23.2.4.1: *"The period for reading shall not exceed half the
  TL-timeout, i.e. 3 seconds."*
- The simulator reproduces the device (`lost_ack_for_answer`,
  `answer_repeat_after`). Four new tests; seven mutants of the fix, all
  caught.

**`[V]` Run 3, with the fix (16:55–16:58 CEST, `download-run-3`).** Steps
0–22 completed: every unload, write, read-back and load-state transition.
Only step 23, the final `A_Restart` (Basic Restart), failed. The device sent
no `T_ACK` in four transmissions of 3 s each, the session released the
connection, and the executor reported the download as failed. An
independent read-back straight afterwards, on a fresh connection, found:

- all three segments (`4000h` 513, `4201h` 511, `4400h` 394 octets) equal to
  the option-C image, with **0 differing unmasked octets**;
- the individual address octets at `4001h`–`4002h` still `11 43` (`1.1.67`);
- load states `B6EAh`–`B6EDh` at `01 01 01 00`, all Loaded.

**Update 2026-09-28 (K2), spec reading.** Read directly from the MP
v02.01.02 PDF:

- §3.7.1.1.2, p. 78, Figure 19: between `t0` and `t1` *"The MaS may react
  under the pre-reset conditions, may not react at all, or may already react
  according the post-reset conditions"*. A missing `T_ACK` after a Basic
  Restart is inside that range.
- §3.7.1.1.3, p. 80: *"The Application Layer of the Management Server shall
  not confirm the A_Restart-service if a Basic Restart is called; to obtain
  the same result with an AL-confirmation, the Management Client should
  instead call a Master Reset with Erase Code 00h."*
- §3.7.3, p. 89: the reset of the communication system may mean *"that no
  T_Disconnect –frame is sent on the bus"*.

`[D]` So no clause read requires a `T_ACK` for a Basic Restart, and none
forbids leaving it out. KNXBench reports the case as
`RestartOutcome::Unconfirmed` and never repeats the restart. The confirmed
alternative the Standard names, a Master Reset with Erase Code `00h`, is a
different service. Whether mask `0701h` supports it has not been checked,
so it is not used.

**`[V]` Frame trace of a closing restart (K2, 2026-09-28 19:00 CEST,
`1.1.67_MDT-0701_2026-09-28_restart-trace.txt`).** With the user's go,
`run_memory_download_observed` ran the plan `Connect → Restart →
Disconnect` against `1.1.67`. That is a Download-scoped session on mask
`0701h`, exactly like run 3's, so there is no Verify Mode and no memory
write. The same plan was first run in the simulator to list what it sends:
three reads, one `A_Restart` and the disconnects. The trace:

| t (s) | Frame |
|---|---|
| 0.12–0.42 | `T_Connect`, three reads. Every request is acknowledged (`T_ACK` seq 0–2) and answered: mask `0701h`, manufacturer `0083h` |
| 0.46 | `A_Restart` (Basic Restart, seq 3), positive `L_Data.con` |
| 3.45, 6.45, 9.45 | the same frame repeated by TL clause 4, each with a positive `L_Data.con` |
| 0.42 → 38.5 | **nothing at all from `1.1.67`**: no `T_ACK`, no `T_NAK`, no `T_Disconnect` |
| 3.7–36.5 | group telegrams from `1.1.7`, `1.1.10`, `1.1.25`, `1.1.28`, `1.1.220`, `1.1.251` received throughout, so the receive path was working |
| 12.46, 12.48 | two `T_Disconnect`s from the client: TL's `A6` after the fourth time-out, then MP §3.7.3's explicit `DM_Disconnect` |
| 38.5 | fresh read-only connection: answered within 30 ms, mask `0701h`, `4001h` = `11 43`, `B6EAh` = `01 01 01 00` |

What this shows:

- `[V]` The device does not acknowledge a Basic Restart in any of the four
  transmissions. That makes two observations (run 3 step 23, and this
  trace), so for this device it is the normal case, not a fault.
  `RestartOutcome::Unconfirmed` is the outcome to expect from it.
- `[D]` TL §5.4.1, p. 21: a `T_DATA_CONNECTED` in `OPEN_IDLE`/`OPEN_WAIT`
  gets `A2`/`A3`/`A4` (`T_ACK` or `T_NAK`), and in `CLOSED` it gets `A10` (a
  `T_Disconnect` back to the sender). A Transport Layer that is running
  answers every one of the four transmissions somehow. This one answered
  none of them over 9 s.
- `[A]` So the device's Transport Layer was not running for at least 9 s
  after the first transmission, which is how a device that starts
  restarting before it acknowledges looks. That is strong circumstantial
  evidence that the restart happens. It is not proof: nothing on the bus
  marks a restart from the inside.
- `[V]` Afterwards the device is back, with its address, and all three
  load states still `Loaded`. The restart undid nothing.
- Cosmetic, fixed with it: the error read "no T_ACK for T_ACK for A_Restart".
  The three `send_acknowledged` labels now name only the request.

**`[V]` Master Reset, Erase Code `01h` "Confirmed Restart" (K2, 2026-09-28
19:18 CEST, `1.1.67_MDT-0701_2026-09-28_master-reset-01h-trace.txt`).** MP
§3.7.1.2 Table 4 (p. 81) defines Erase Code `01h` as "the Master Reset as a
confirmed alternative to the unconfirmed Basic Restart", with no Resource
reset. MP §3.7.3 (p. 88) requires the client to verify support first. For
mask `0701h` there is no means to do that: Profiles v02.01.01 §4.2 (p. 37)
marks Master Reset `O` (optional) for BIM M112 mask `0701h`, and the
Management Profile that announces it (Resources §4.1.3 Table 8) belongs to
the E-Mode device descriptor. The user gave the go explicitly for this test
device, and the only way left to find out was to try it once, with the
erase code that resets nothing. The probe first ran in the simulator
(`Connect`, Verify-Mode read/write of `PID_DEVICE_CONTROL`, `A_Restart
type 1 [01 00]`, `Disconnect`; no memory write).

| t (s) | Frame |
|---|---|
| 7.56–7.78 | `T_Connect`; `PID_DEVICE_CONTROL` read `00`, written `04`, echoed `04`: Verify Mode active. All `T_ACK`ed |
| 7.82 | `A_Restart`, restart_type 1, Erase Code `01h`, channel `00h` (seq 2) |
| 10.80, 13.80, 16.81 | the same frame repeated by TL clause 4 |
| 7.78 → 45.8 | **nothing from `1.1.67`**: no `T_ACK`, no `A_Restart_Response`, no `T_Disconnect` |
| 19.81, 19.83 | client `T_Disconnect` ×2 |
| 45.8 | read-only check: mask `0701h`, `4001h` = `11 43`, `B6EAh` = `01 01 01 00`, `4400h` = `40 07 00 07 40 4F 00 07`, identical to before |

- `[V]` `1.1.67` sends no `A_Restart_Response` to a Master Reset. It acts
  exactly as it does for a Basic Restart. MP footnote 11 (p. 88) anticipates
  this: *"Existing implementations may not check bit 0 of octet 7 … may
  only perform a Basic Restart if a Master Reset is called"*. NOTE 10's
  guarantee (the response is sent *before* the reset) therefore does not
  hold for this device.
- `[V]` The device kept its address, application (first 8 octets of `4400h`)
  and load states. Erase Code `01h` really reset nothing.
- `[D]` Verify Mode: this Restart-scoped session set it, as `connect()` does
  for every scope except a memory-mapped load. MP §3.7.1.2.3 does not
  exempt it. On a new connection a client must assume it cleared (MP
  §3.29.1), so it has no lasting effect.
- **Consequence:** a confirmed restart does not exist for this device. The
  inside view of the restart ("did it really restart?") cannot be
  established from the bus. `RestartOutcome::Unconfirmed` stays the final
  answer for mask `0701h` MDT devices, and KNXBench offers no Master Reset
  for them.
- `[V]` Side finding from attempt 1 (19:16, kept as
  `…master-reset-01h-attempt1-lost-disconnect.txt`): the read-only
  pre-check's `T_Disconnect` produced no `L_Data.con` in the trace, and
  `1.1.67` answered the next session's first request with `T_NAK` twice,
  then sent `T_Disconnect` itself after ~6 s. So its old connection was
  still open. A second session started straight after a first one is
  therefore not safe. The probe waited 7 s in attempt 2, and that worked.
  Why the disconnect never showed up is open: `ManagementSession::disconnect`
  discards the send error (`let _ =`).

**`[O]` → `[V]` Open: did the device restart?** *(Answered 2026-09-29 by
the K7 function check below: yes; the new image was active without a
power cycle.)* MP §3.7.1.1.3 says the server does
not confirm a Basic Restart at the Application Layer, and MP §3.7.3
exception (5) tells the client to ignore everything the server sends after
`A_Restart` *"except negative TL-confirmations"*. That clause does not say
whether a missing `T_ACK` counts as one. The run took no frame trace. One
possible reading, **not verified**, is that the device restarts before its
Transport Layer acknowledges. Whether the device now runs the option-C
application has to be checked on the device itself (button 1 → `2/0/53`),
not guessed from this log.

**`[V]` Function check after a power cycle (17:08 CEST,
`monitor-after-powercycle`).** The user disconnected the device from the bus
once and reconnected it, then pressed button 1 repeatedly. A read-only bus
monitor recorded 33 telegrams from `1.1.67`:

- all 33 were `GroupValueWrite` to `2/0/53`;
- the values alternate strictly `1, 0, 1, …`, with no value repeated back to
  back;
- there were **no** telegrams to the old associations (`2/1/15`, `2/1/16`,
  `0/4/6`, `0/4/7`) or to any other group address.

Option C is working: button 1 toggles `2/0/53`. The question above stays
open, because the power cycle restarted the device regardless of whether
the `A_Restart` had already done it. What this check shows is that the
downloaded image is correct and a device restart activates it.

**`[V]` K7 live acceptance, both product paths (2026-09-29, 06:11–06:29 CEST).**
With the user's *"go k7"*, `1.1.67` received two downloads built by the
product path from a project, never from the fixed test image:

1. **CLI, `knx device download`, 06:11:50–06:14:34**
   (`1.1.67_MDT-0701_2026-09-29_k7-cli-download.txt`). The project is a
   new one, `KNXBench 1.1.67 K7 switch-by-push off.knxdb` (gitignored):
   the MDT push button from the product catalog, button 1 in mode
   "Switch by push", value **Off** (`UP-5501 = 0`, `UP-5517 = 0`), object 0
   sending to `2/0/53`. Against the fresh read-only dump taken just before
   (`…pre-k7.txt`), its image differs in exactly **4 octets**
   (`4409h`, `440Ah`, `450Bh`, `4510h`); against option C it therefore
   differs, as K7 requires. 25 steps, **1416 octets written, every one read
   back**, all three load states `Loaded`, restart `NOT confirmed` as
   expected for this device. An independent read-back on a fresh
   connection 2 min later (`…post-k7-cli.txt`) found **0 differing
   unmasked octets** against the K7 image.
2. **Web UI, Download to device tab, 06:25:18–06:28:04**
   (screenshots and request log in the session's scratch, not kept). The
   real front end (Vite dev server against a local `knx-server`), driven
   headless with Playwright: File → Open (.knxdb) with option C, Bus
   monitor → Download to device, device `1.1.67`, gateway, "Show what would
   be written", "Download to 1.1.67", the consent dialog, "Program device".
   The browser sent exactly one `start` with the plan's own id and phrase,
   then only polled status. The panel ended with *"Written to the device:
   yes, 1416 octets, every block read back unchanged"* and *"Restart: NOT
   confirmed"*. The independent read-back 45 s later
   (`…post-k7-web.txt`) found **0 differing octets** against option C.

What this shows and what it does not:

- `[V]` Both product paths (CLI and web) write a project's configuration
  into a real mask `0701h` device, byte-exact, and report the unconfirmed
  restart honestly. The fixed test image is no longer the only hardware
  write path.
- `[V]` The binary carried K8's §105 change: every `T_Connect`,
  `T_Disconnect` and `T_ACK` in both runs went out at system priority
  through the real KNXnet/IP interface, and the device acknowledged every
  data request except the closing restart. The interface accepts the control frames;
  whether it keeps the priority bits on the wire was not traced (no bus
  monitor was running).
- `[V]` **Function check (06:51–06:59 CEST).** The K7 project was
  downloaded a third time (`…k7-check-download.txt`: 1416/1416 read back).
  After the ~38 s restart window, a read-only `knx bus monitor` ran for
  81 s (`…k7-check-monitor.txt`, 06:53:31–06:54:54) while the user pressed
  button 1. It recorded **11 telegrams from `1.1.67`, all
  `GroupValueWrite 0` to `2/0/53`**, and none to any other address. Option C
  toggles (`1, 0, 1, …`, the 2026-09-28 power-cycle monitor), so the K7
  configuration was active. The download's own Basic Restart activated it:
  unlike on 2026-09-28, this time there was **no power cycle**. That answers
  the `[O]` question above for this device: the unacknowledged restart does
  restart it. The monitor printed the small-payload value as `0x00 (6-bit)`
  because it ran without `--project`, so no DPT was resolved.
- `[V]` Afterwards option C was downloaded again (06:55,
  `…k7-check-restore-optionC.txt`, 1416/1416), and an independent read-back
  45 s later (`…post-k7-check.txt`) found **0 differing octets** against it.
  `1.1.67` is back where it started.

**`[V]` K6 live, individual address `1.1.67` → `1.1.68` (2026-09-29,
07:12–07:13 CEST, `…k6-to-1.1.68.txt`).** A read-only scan first found
`1.1.67` occupied (mask `0701h`) and `1.1.68` vacant (`…k6-prescan.txt`).
`knx device program-address 1.1.68` waited for the button, and the user
pressed it in round 37, where exactly one device answered (`1.1.67`).
Steps 1–3 ran. Step 4 connected to `1.1.68` and read the device there;
the settling retry (`be91fe3`) was not needed, and the output shows no
second connect. Only the closing Basic Restart went unacknowledged (four
transmissions, TL released the connection), exactly as in every K7 run.
A read-only scan 40 s later (`…k6-postscan-1.txt`) found `1.1.68` occupied
(mask `0701h`) and `1.1.67` vacant, and the user confirmed the programming
LED was **off**. So the restart did end programming mode: a second
observation that this device restarts without acknowledging.

- `[V]` Bug found: the procedure reported the restart silence as MP §2.3
  "to 4." (`FAILED … address written: yes, but NOT confirmed: the device
  did not answer at the new address`), although step 4 had just read the
  device at that address. Fixed with `AddressRestart` (KNOWN_LIMITATIONS
  §116, status 2026-09-29).
- `[O]` The settling retry itself is still not live-verified: this device
  answered at the new address on the first connect.
- `[V]` **The way back with the fix (`6a71162`), 07:33–07:34 CEST
  (`…k6-back-to-1.1.67-try2.txt`).** The first wait (07:28) expired
  unpressed: `gave up after 90 rounds … nothing was written`, exit 1, which
  is the empty path working as designed. On the second wait the user
  pressed in round 16, and `1.1.68` was found. Result: `finished`, `address
  written: yes, 1.1.68 -> 1.1.67; the device answered at 1.1.67`, `restart:
  NOT confirmed (no T_ACK …)`, exit 0. The scan 40 s later
  (`…k6-postscan-back.txt`) found `1.1.67` occupied (mask `0701h`) and
  `1.1.68` vacant. A memory dump (`…post-k6.txt`) matches the post-K7
  dump line for line (180 of 180 lines), so the two address changes left
  the configuration (option C) untouched.
- `[V]` No `LoadCompleting` stall and no mid-download drop occurred, so
  §101 and §104 had nothing to measure in either run.

---


<!-- END preserved record-2-013 -->
