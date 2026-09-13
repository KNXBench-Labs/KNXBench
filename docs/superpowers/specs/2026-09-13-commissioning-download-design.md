# T30 phase 1 — the specification that has to exist before anything touches a device

- **Date:** 2026-09-13
- **Status:** design, ready for phase 2 (offline implementation against a simulator)
- **Phase:** 1 of 3. **This document is the deliverable.** Phase 1 writes no
  code and performs no bus access. Phase 2 implements against a simulator with
  no hardware attached. Phase 3 verifies **read-only** against real hardware.
  Writing to a real device can destroy it, which is the whole reason the
  specification is a separate phase.
- **Closes:** nothing on its own. It narrows
  [KNOWN_LIMITATIONS.md §7](../../KNOWN_LIMITATIONS.md) from "blocked" to
  "specified, unimplemented", and it is the design input for
  [GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md) row **E1** / backlog task
  **T30**.
- **Scope of the eventual implementation:** a new `knx-core` module for the
  commissioning domain (procedures, state machines, plans), extensions to
  `knx-net` for the Application Layer services it lacks, and a simulator crate
  or test double. No UI in phase 2. No change to `knx-productdb`'s schema is
  required by this document, though §14 names what it would need later.
- **Research record:** [RESEARCH.md §8.7](../../RESEARCH.md) records the
  findings established while writing this, including two corrections to §8.4.

## 1. The rules this document obeys

`CLAUDE.md`: *"Only implement protocol behavior that is technically verified."*
Applied to a document rather than to code, that means:

1. **Every protocol claim carries a citation** — the source PDF name, the
   clause number, and the sentence the claim rests on, quoted. Quotations were
   read with a file-reading tool from `pdftotext -layout` extractions of the
   source PDFs, never copied out of piped shell output, because piped output in
   this environment is silently reworded and a reworded citation is not a
   citation.
2. **Where there is no citation, the text says so, and says where it looked.**
   The phrase used is *"undocumented in \<base\>"*, naming which of the two KNX
   specification knowledge bases (`knx_spec_kb_programming`, 27 programming
   PDFs with figures; the 177-PDF text-only base) or which extracted corpus was
   searched. A gap that is labelled a gap costs an implementer an afternoon. A
   guess dressed as a fact costs a customer a device.
3. **Confidence markers are the ones the rest of `docs/` uses** and are never
   promoted: **[D]** the Standard states it, quoted with its PDF; **[D, corpus]**
   quoted from an extraction because no indexed fact row exists; **[V]** verified
   by observation on this project's own data or hardware; **[A]** inference,
   explicitly reasoned, never dressed as [D].
4. **Nothing in this document has been run against a device.** Every **[D]**
   below is a statement about the Standard, not about any device's behaviour.
   Devices deviate; that is what phase 3 exists to find out.

### 1.1 Documents cited

| Short name used below | Source PDF |
| --- | --- |
| **AL** | `03_03_07 Application Layer v02.01.01 AS.pdf` |
| **RES** | `03_05_01 Resources v01.10.01 AS.pdf` |
| **MP** | `03_05_02 Management Procedures v02.01.02 AS.pdf` |
| **CP** | `03_05_03 Configuration Procedures v02.01.01 AS.pdf` |
| **TL** | `03_03_04 Transport Layer v01.02.03 AS.pdf` |

Five documents. Everything protocol-level in this specification comes out of
those five. Where a claim would have needed a sixth (`06 Profiles`,
`03_07_02 Datapoint Types`, `AN194`), the claim is not made — it is listed in
§12 as a gap instead.

An extraction artefact worth knowing before anyone re-checks these quotes: in
the `pdftotext` output of **AL**, the heading of §3.5.4 renders as
`..  _ emory_rite-service` while the clause body extracts intact. The body is
what is quoted. Similarly, `docs/RESEARCH.md` §8.4 states of **AL** Table 1
that *"No APCI value can be read out of that safely"* — that is true of the
**Markdown** extraction it was assessing. `pdftotext -layout` on the source PDF
renders Table 1 row by row and legibly, which is why this document can state
APCI encodings at all. See RESEARCH §8.7.

## 2. Safety constraints, which are requirements and not warnings

These are stated first, before any protocol, because they constrain the shape
of the code rather than the behaviour of the code. A constraint that can be
satisfied by remembering to check something will eventually not be satisfied.

### 2.1 R-SAFE-1 — `1.1.220` is structurally unreachable

The installation this will eventually run against contains an **alarm panel at
individual address `1.1.220`**. It must never be read, never be written, never
be probed, and never appear inside any address range, scan plan, iteration,
retry list or diagnostic sweep.

This is a design requirement on the implementation, not an operating
instruction. Concretely:

- The commissioning layer takes its targets from a plan type that **cannot
  represent an excluded address**, in the way `knx_core::scan::ScanPlan`
  already does for line scanning: exclusions are applied when the plan is
  constructed, so there is no code path that iterates a raw range. Any new
  plan type in the commissioning module follows the same pattern —
  exclusion-by-construction, not exclusion-by-filter-at-the-call-site.
- The exclusion list lives in **one** place, is shared by scanning and
  commissioning, and is enforced at the lowest layer that has an address: a
  single guard between the commissioning domain and `knx-net`, so that a bug in
  a higher layer cannot route around it.
- A plan that would have contained an excluded address is **not** silently
  shortened. It reports the exclusion, per `CLAUDE.md`'s "never silently
  discard information". Silence here is indistinguishable from the guard not
  running.
- Broadcast operations are the sharp edge. `A_IndividualAddress_Write` is
  **sent by broadcast** and is accepted by whichever device is in programming
  mode (§4). There is therefore **no address filter that can protect `1.1.220`
  from a broadcast**. The only protection is procedural, so it is a hard
  precondition in the code: a broadcast commissioning operation may only be
  issued after the count of devices in programming mode has been established
  as exactly one (§4.2), and the individual-address programming flow refuses to
  run when that count is anything other than one. This is exactly what MP §2.3
  requires anyway; §2.1 is the reason it is non-negotiable rather than
  recommended.

### 2.2 R-SAFE-2 — the approved address range

Individual addresses `1.1.24`–`1.1.32` are approved for **active reads** on the
real installation, via gateway `KNX_GATEWAY`. Nothing else is approved for
anything. Phase 3 is read-only within that range. Any write to a real device
requires a fresh, explicit, specific go-ahead from the user for that write, and
the implementation must not have a mode in which writes happen without one.

### 2.3 R-SAFE-3 — the write/read asymmetry is in the type system

Read operations and write operations do not share an entry point. A "download"
API that takes a boolean `dry_run` will be called with the wrong boolean. The
implementation separates them:

- an inspection API that can only issue the services in §6.1 and §7.1
  (`A_DeviceDescriptor_Read`, `A_Memory_Read`, `A_PropertyValue_Read`,
  `A_IndividualAddress_Read`), and
- a mutation API that is constructed from an explicit authorisation value
  carrying the operator's confirmation and the concrete target, and which
  cannot be constructed by default.

Phase 2's simulator is the only thing the mutation API is pointed at until the
user says otherwise.

### 2.4 R-SAFE-4 — no bus access from the domain layer

Unchanged from `CLAUDE.md`'s architecture rule and restated because
commissioning is where it is most tempting to break: the commissioning domain
in `knx-core` produces and validates **plans and state transitions**; it does
not open sockets. `knx-net` executes. The existing `ScanTransport` trait is the
model — the domain talks to a trait, and the tests talk to a fake.

## 3. The device model the procedures act on

### 3.1 Interface Objects, Properties, and one Load State Machine per loadable part

A device's configurable state is reached through Properties of Interface
Objects. The relevant ones for download:

| PID | Property | Type | Role here |
| --- | --- | --- | --- |
| 5 | `PID_LOAD_STATE_CONTROL` | `PDT_CONTROL` | The Load State Machine: read = state, write = event (§5) |
| 6 | `PID_RUN_STATE_CONTROL` | — | Run State Machine (§3.3) |
| 14 | `PID_DEVICE_CONTROL` | `PDT_BITSET8`, DPT 21.002 | Bit 2 = Verify Mode (§6.3) |
| 28 | `PID_ERROR_CODE` | `PDT_ENUM8`, `DPT_ErrorClass_System` 20.011 | Last error before load state Error (§5.5) |
| 29 | `PID_OBJECT_INDEX` | read-only | The Interface Object's **own** index (§3.2) |
| 51 | `PID_MCB_TABLE` / `PID_MCB` | — | Memory Control Block, carries the CRC used for partial download (§7.4) |

**[D]** RES §4.23.1: *"The Load State Machine shall be able to manage all kinds
of downloadable configuration data including executable code. More than one
Load State Machine is possible in one device. … The state of the Load State
Machine shall be stored in non-volatile memory (e.g. EEPROM, flash)."*

Two consequences the implementation is built around. First, load state is
tracked **per loadable part**, never per device — Application Program 1,
Application Program 2, the Group Object Table, the Address Table and the
Association Table each have their own LSM, as CP §3.5.2's step list shows by
driving each one separately. Second, because the state is non-volatile, an
interrupted download is **still there after a power cycle** (§9).

### 3.2 Correction: `PID_OBJECT_INDEX` is not the addressing mechanism

`docs/RESEARCH.md` §8.4 (Q2/Q5 area, and its PID table row 29) says each LSM is
*"addressed independently by `object_index` (`PID_OBJECT_INDEX`, PID = 29)"*.
That conflates two things and the implementation must not inherit the mistake.

**[D]** RES §4.2.29: `PID_OBJECT_INDEX` is read-only and *"shall contain the
local Object Index of the Interface Object in which it is located"*. It is a
property a device reports **about itself**.

The mechanism that selects which Interface Object — and therefore which LSM —
a property access targets is the **`object_index` field of the property
services themselves** (`A_PropertyValue_Read` / `_Write`, AL §3.4.4), not
PID 29. PID 29 is how you ask an object what its index is; the service field is
how you say which object you mean. RESEARCH §8.7 records the correction.

### 3.3 The Run State Machine is a separate machine

`PID_RUN_STATE_CONTROL` (PID 6) carries the application's run state
(Halted / Running / Ready / Terminated / Starting / Shutting down) with events
NOP / Restart / Stop. It is **not** part of the download sequence in CP §3.5.2
and this specification does not use it. It is named here only so an implementer
does not mistake it for the LSM: `PID_LOAD_STATE_CONTROL` says whether the data
is valid, `PID_RUN_STATE_CONTROL` says whether the program is running.

### 3.4 Realisation Types, and which ones are specified

- **LSM Realisation Type 1** — property-based, via `PID_LOAD_STATE_CONTROL`.
  This is what §5 specifies and what the implementation targets.
- **LSM Realisation Type 2** — **[D]** RES §4.23.3: *"This Realisation Type is
  not specified in this version of this document."* The implementation must not
  claim to support it. If a device turns out to need it, that is a finding for
  phase 3, not something to infer.
- **Programming Mode Realisation Type 2** — the memory-mapped form at address
  `60h`, bit 0, with a parity bit 7 (RES §4.26, Figure 66). **[D]** MP §3.13.2
  NOTE: *"This means that the state of the Programming Mode is located at
  memory address 60h."* This one **is** specified and §4.4 uses it.
- **Mask-version-specific LSM access** — MP §3.31.2's `_RCo_Mem` variant is
  restricted to mask `070nh` and uses fixed memory addresses (load control at
  `0104h`, load states at `B6EAh`/`B6EBh`/`B6ECh`/`B6EDh`). Out of scope for
  phase 2; recorded so it is not reinvented.

## 4. Programming the individual address via the programming button

### 4.1 What the service is, and why it is broadcast

`A_IndividualAddress_Write` is an unconfirmed broadcast service. **[D]** AL
§3.2.2: *"only the device where the button is pressed shall accept the
A_IndividualAddress_Write.ind, others shall ignore it. The way that a product
is set to ´programming mode´ may be manufacturer specific."*

Two things follow directly. The selection of the target is **the operator's
finger on a button**, not an address in the frame — the frame carries only the
new address (AL §3.2.2 Figure 9: APCI octets, then the new address high byte
and low byte). And a device not in programming mode **ignores** the indication;
the Standard specifies no negative response, so silence is not evidence of
anything.

**[D]** AL §3.2.3, Figures 10/11: `A_IndividualAddress_Read` is likewise a
broadcast, and the `A_IndividualAddress_Response` PDU **carries no data**. The
responding device's address arrives as the **source address of the frame**.
Any implementation that looks for the address in the APDU will find nothing.

### 4.2 The procedure, in order (MP §2.3)

MP §2.3 `NM_IndividualAddress_Write` specifies the sequence. It is not "send
the write"; the write is step 3 of 4.

1. **Check that the new address is free.** Probe `IA_new` with
   `A_Connect` / `A_Disconnect` and `A_DeviceDescriptor_Read`. A response means
   the address is occupied and the procedure must stop. (MP §2.3's exception
   handling "to 1." and its note a).)
2. **Establish that exactly one device is in programming mode.** Repeat
   `A_IndividualAddress_Read` until exactly one device answers, time-out 1 s
   per repetition. Zero responders: no device is in programming mode. More than
   one: more than one button is pressed, and the procedure must **not**
   continue. MP §2.3's exception handling "to 2." enumerates the three cases.
3. **Write**, only if `IA_new != IA_current` — `A_IndividualAddress_Write`.
4. **Verify and restart.** `A_Connect` to the new address,
   `A_DeviceDescriptor_Read` with `descriptor_type = 00h`, then `A_Restart`, and
   then abort the client-side Transport Layer connection. **[D]** MP §2.3,
   exception handling "to 4.": if step 4 fails, *"the programming of the
   Individual Address may have failed, or the system (Router) is not configured
   correctly."* — note that the Standard itself refuses to distinguish those two
   causes, so the implementation must report both possibilities rather than
   assert the first.

The counting rule for the read phase is specified separately and is stricter
than it looks. **[D]** MP §2.2 `NM_IndividualAddress_Read`: time-out 3 s, and
*"The Management Client shall always wait until the time-out has elapsed."* —
i.e. the client may not return early on the first response, because the whole
point is to count responders. And **[D]** *"The Management Client shall not
evaluate Layer-2 repetitions."*: a repeated frame is not a second device. An
implementation that counts frames instead of distinct sources will conclude two
devices are in programming mode and refuse a legitimate operation, or worse,
deduplicate wrongly and proceed when two really are.

### 4.3 Programming mode may switch itself off underneath you

**[D]** RES §4.26 specifies an optional autonomous inactivation: *"if the
Programming Mode becomes enabled, by any means, the MaS (device) shall start a
time-out timer of 4 minutes … If the timer expires, the MaS shall autonomously
and automatically disable its Programming Mode."*

It is **optional**, so the implementation may neither rely on the timer nor
assume its absence. Design consequence: programming mode is re-verified
immediately before the broadcast write, not once at the start of a wizard, and
a UI that walks an operator through several screens after the button press must
re-run step 2 of §4.2 before step 3. The four-minute figure is a lower bound on
how long a stale "one device in programming mode" observation stays true, and
on some devices it is no bound at all.

### 4.4 Switching programming mode remotely

MP §3.13 `DM_ProgMode_Switch` exists: `DM_ProgMode_Switch(flags, mode)` with
**[D]** *"mode 0: switch Programming Mode off, 1: switch Programming Mode on"*
and *"flags — All bits are reserved. These shall be set to 0. This shall be
tested by the Management Client."* It requires a `DM_Connect` first, and
procedure `DMP_ProgModeSwitch_RCo` implements it as a read-modify-write of one
octet at memory address `60h`: `A_Memory_Read(60h, 1)`, then
`A_Memory_Write(60h, 1, DD)` where **[D]** *"In the data (DD) bit 0 has to be
set according to the mode. The parity (bit 7) has to be calculated."*

This is a **write to a live device at a fixed memory address**, addressed by
individual address rather than by button press. It is therefore in the mutation
API (§2.3), it is out of scope for phase 3's read-only verification, and the
read half (`A_Memory_Read(60h, 1)`) is the only part phase 3 may exercise — and
only inside `1.1.24`–`1.1.32`. The parity computation is specified as a
requirement but the parity **rule** (which bits it covers, odd or even) is
**undocumented in the two knowledge bases as searched for this document**; RES
§4.26's Figure 66 shows the layout (`prog_mode` bit 0, `p_parity` bit 7) but a
figure is not an algorithm, and figures are the part of the corpus that extracts
worst. Phase 2 must not guess it: read the octet, and if the intended write
cannot be derived without inventing the parity rule, the operation fails with
"undocumented" rather than writing a byte that might be wrong.

## 5. The Load State Machine

### 5.1 Format

**[D]** RES §4.23.2.1: `PID_LOAD_STATE_CONTROL` is `PDT_CONTROL`, and *"The
format shall differ between the read access, as specified in Table 92 and write
access, as specified in Table 93."* Read gives a state; write is an event. The
same PID, two different encodings, depending on direction — an easy thing to
model wrongly as one enum.

### 5.2 States (RES Table 92)

| State | Value | M/O | Remark, quoted |
| --- | --- | --- | --- |
| Unloaded | 0 | M | *"No data is loaded. The data that is associated with the Property Load Control is not valid (e.g.: Address Table is not valid)."* |
| Loaded | 1 | M | *"Valid data is loaded. Only in this state the associated data shall be considered as valid; in all other states the data shall be considered as invalid"* |
| Loading | 2 | M | *"Load process is active."* |
| Error | 3 | M | *"Error in data detected or error during load process."* |
| Unloading | 4 | O | *"Unload process is active"* |
| LoadCompleting | 5 | O | *"Intermediate state between Loading and Loaded, e.g. checksum calculation process is active"* |

The single most load-bearing sentence in this document is *"Only in this state
the associated data shall be considered as valid"*. It is the definition of
"the download worked", and it is the only one the implementation may use. A
device that answers frames is not a device whose configuration is valid.

### 5.3 Events (RES Table 93)

| Event | Value | Reaction, quoted |
| --- | --- | --- |
| No Operation | `00h` | *"Nothing"* |
| Start Loading | `01h` | *"This shall be interpreted as the request to start the loading of the loadable part."* |
| Load Completed | `02h` | *"This shall be interpreted as the request to complete the Loading of the loadable part if this event occurs during the state Loading. Possible checksum shall be calculated, all data shall be declared as valid and the Load State shall change to the state Loaded, if no internal errors occurs and no error in the data is detected. If the transition from Loading to Loaded takes more than 2 seconds, e.g. by complex checksum calculation, the Load State shall first change to intermediate state LoadCompleting and change to Loaded after calculation is finished."* |
| Additional Load Controls | `03h` | *"This Load Event shall be used to transfer additional load information like memory allocation (absolute or relative), application entry points or special run conditions for an application."* |
| Unload | `04h` | *"This Load Event shall be interpreted as the request to unload the loadable part. The loadable data shall be declared as invalid. The data is undefined."* |

**[D]** RES §4.23.2.3.2 on events the device will not act on: *"Illegal
additional events (e.g. invalid values written to the Property) shall be
ignored and shall not lead to a change of state."* and *"The Load State Machine
shall enter the state Error in case of an error. Unknown events shall be
ignored."* So a write that produces no state change is ambiguous between "you
wrote something illegal" and "nothing happened", which is why every event write
in §7 is followed by a **state read**, not by an assumption.

A cross-reference to fix on the way past: RES Table 93's Additional Load
Controls row points at *"clause 3.27 'DM_LoadStateMachineWrite' in [09]"*,
while in the current **MP** document `DM_LoadStateMachineWrite` is **§3.31** and
§3.27 is `DM_InterfaceObjectRead`. Follow the name, not the number.

### 5.4 Transitions (RES Table 94, reproduced)

Legend, quoted: *"I: intermediate state / M: mandatory / O: optional /
R: recommended"*. Where a cell lists `R:` and `O:` alternatives, **[D]** RES
§4.23.2.3.3: *"For events that are client errors, more than one transition is
allowed; the recommended transitions should be implemented but the optional
transitions may be implemented alternatively."* An implementation must
therefore accept **either** outcome in those cells and must not treat the
optional one as a device defect.

| Load Event | from Unloaded (00h) | from Loaded (01h) | from Loading (02h) | from Error (03h) | from Unloading (04h) | from LoadCompleting (05h) |
| --- | --- | --- | --- | --- | --- | --- |
| No Operation (00h) | Unloaded | Loaded | Loading | Error | Unloading | LoadCompleting |
| Start Loading (01h) | Loading | Loading | Loading | Error | Error | R: Error / O: I: LoadCompleting, M: Loaded |
| Load Completed (02h) | R: Unloaded / O: Error | R: Loaded / O: Error | I: LoadCompleting, M: Loaded | Error | Error | R: Error / O: I: LoadCompleting, M: Loaded |
| Additional/Segment Load Controls (03h) | R: Unloaded / O: Error | Error | Loading | Error | Error | R: Error / O: I: LoadCompleting, M: Loaded |
| Unload (04h) | Unloaded | I: Unloading, M: Unloaded | I: Unloading, M: Unloaded | I: Unloading, M: Unloaded | I: Unloading, M: Unloaded | R: I: Unloading, M: Unloaded / O: I: LoadCompleting, M: Loaded |
| Device Restart | Unloaded | Loaded (Error in case of error detection at start-up) | R: Loading / O: Error | Error | Unloaded | R: Unloaded / O: I: LoadCompleting, M: Loaded |

Read off the facts that matter to the implementation:

- **`Error` is a trap for every load event.** From `Error`, NOP, Start Loading,
  Load Completed and Additional Load Controls all leave the state at `Error`.
  The **only** event that leaves `Error` is `Unload (04h)`. A retry loop that
  re-sends Start Loading after an error will loop forever; recovery is unload,
  then start again.
- **`Unload` always terminates at `Unloaded`** from every state except
  `LoadCompleting` (where the device may be busy — see below). That is the
  recovery path, and §9's recovery procedure is built on it.
- **`Start Loading` from `Loaded` goes to `Loading`** — i.e. beginning a
  download on a configured device invalidates its configuration immediately,
  before any data is sent, because `Loading` is not `Loaded` and §5.2 says only
  `Loaded` is valid. There is no "prepare a download and then commit it"
  semantics available. This is the core risk in §13.
- **`LoadCompleting` may not answer at all.** **[D]** Table 94's footnote:
  *"a device may be offline during state LoadCompleting and therefore not react
  to load events from a MaC. Transition from state LoadCompleting to state
  Loaded is an internal event. Therefore it may not be possible to provoke any
  state transition or a device restart from a MaC. Ignore load event: if the
  device may communicate, it shall return unchanged state LoadCompleting"*. A
  non-answering device in this phase is behaving correctly. Treating silence as
  failure and retrying is the wrong response.
- **Unloading is nearly unobservable.** **[D]** RES §4.23.2.3.2: *"The event
  UnloadCompleted is mostly only an internal event. Therefore, a Management
  Client may never observe the state Unloading."* The implementation must
  accept `Unloaded` directly after an unload event and must not wait for
  `Unloading` first. MP §3.31.4 footnote 15 says the same from the procedure
  side: *"The optional state Unloading is not used."*

### 5.5 Timing, and how to wait

**[D]** RES §4.23.2.1: *"The transitions between the states shall be less than
30 seconds. This time shall be the minimum delay during which a MaC shall wait
after sending a load event until interpretation of the load event with the
expected state transition to be failed."*

So 30 s is a **floor on the client's patience**, not a device deadline to
enforce. MP §3.31.4 states the same number operationally: *"Retry read Property
for 30 Seconds"*.

**[D]** RES §4.23.2.4.1 constrains the polling itself: *"The period for reading
shall not exceed half the TL-timeout, i.e. 3 seconds."*, and the same clause
requires re-establishing a broken Transport Layer connection periodically
during the maximum transition time. The 3 s figure is consistent with **TL**
clause 4's connection-oriented parameters as already recorded in RESEARCH §8.5
(connection time-out 6 s, acknowledge time-out 3 s, `max_rep_count` 3) — the
poll interval is half the 6 s connection time-out, which is what keeps the
connection alive while the device is busy.

The resulting wait loop, which phase 2 implements once and reuses everywhere:

```text
write event  → poll PID_LOAD_STATE_CONTROL every ≤3 s
             → for up to 30 s
             → reconnect the TL connection if it has dropped, and keep polling
             → accept LoadCompleting as "still working", not as failure
             → accept a non-answering device in LoadCompleting as normal
             → on expiry: the transition failed; do not assume which side failed
```

On failure, `PID_ERROR_CODE` (PID 28) is the place to look. **[D]** RES §4.2.28:
*"This Property shall indicate the last error that occurred before the load
state 'Error' is set. When the load state changes from 'Error' to a different
state then the Error Code shall be set to '0' (no error)."* Which means: read it
**before** unloading, because unloading clears it. The enum is
`DPT_ErrorClass_System` 20.011; its member values are recorded in RESEARCH
§8.4 Q5 and were not re-verified from `03_07_02 Datapoint Types` for this
document, so the implementation should treat the numeric mapping as [D] from
§8.4 and re-check it against that PDF before rendering names to a user.

## 6. Memory read and write over the bus

### 6.1 `A_Memory_Read`

**[D]** AL §3.5.3: the service is used *"to read between 1 and 63 octets"* and
*"memory_address shall specify the 16 bit start address"*. Error handling in
the same clause: a `number` field of 0 is the failure indication, and the device
shall ignore the request if `number > Maximum APDU Length – 3`.

16-bit address means this service alone reaches 64 kB. §7.2 covers what happens
above that.

### 6.2 `A_Memory_Write`

**[D]** AL §3.5.4: the service is used *"to write between 1 octet and 63
octets"*. Its confirmation behaviour depends on Verify Mode: *"The service
shall be a confirmed service if Verify Mode is active, otherwise it shall be an
acknowledged service."* And, independently of Verify Mode: *"The value of the
associated memory area shall be explicitly read back after writing to it."*

The read-back is **required by the Standard**, not a nicety this project is
adding. Phase 2 implements no write path that lacks it.

Error handling, quoted from the same clause, because each case needs different
handling in code:

- Target protected or nonexistent: *"the service indication shall be ignored"* —
  so a write to a protected region looks exactly like a lost frame.
- Partially protected target: *"the complete write operation shall fail"* — no
  partial writes; there is no "how far did it get" to recover from.
- The request is ignored if `number > Maximum APDU Length – 3`, or if `number`
  does not match the count of octets actually received.
- A failed write under Verify Mode is reported as `number = 0` with no data.

### 6.3 Verify Mode, and its two traps

**[D]** RES §4.2.14, Table 11: `PID_DEVICE_CONTROL` (PID 14, `PDT_BITSET8`,
DPT 21.002) bit 2 is *Verify Mode On*. **[D]** RES §4.2.14.7.3/§4.2.14.7.4: the
default is 0, the client must actively set it, and it is **automatically
disabled when the Transport Layer connection closes**.

Both halves are traps. Verify Mode is off unless you turn it on, so the
"confirmed service" wording of §6.2 does not apply by default. And it dies with
the connection, so a download that reconnects mid-flight — which §5.5 requires
during long transitions — silently loses verification unless the
implementation re-sets bit 2 after every reconnect. The design rule: Verify
Mode is a property of the **session object**, re-asserted on every connect, and
its current value is read back rather than assumed.

MP §3.16's two procedures encode the two modes explicitly:
`DMP_MemWrite_RCoV` sets `PID_DEVICE_CONTROL` bit 2 first;
`DMP_MemWrite_RCo` does not, and instead runs its own verify loop —
`A_Memory_Write`, then `A_Memory_Read`, and **[D]** *"different or no data
received ⇒ error"*; where verification is not requested, it instead inserts a
*"delay for programming the memory in the device"*. That delay is named but not
quantified in MP §3.16, and no figure for it is cited anywhere this document
searched: **undocumented in both knowledge bases as searched**. Phase 2 must
make it a configurable parameter with a conservative default and must not
present the default as a specification value.

### 6.4 The chunk size the implementation must actually use

This is where a plausible reading of the Standard produces a broken
implementation. AL §3.5.3/§3.5.4 say 1–63 octets. **[D]** MP §3.16
`DM_MemWrite` adds: *"If the Management Server does not support the
L_Data_Extended frame format, then this maximal size shall be 12 octets."*

So the service permits 63, and a large population of real devices permits 12.
MP §3.16 also specifies the call's shape: data format code `20h`, and a flags
octet whose bit 0 selects the data location and bit 1 requests verification.

Design rule: the maximum write length is a **per-device negotiated value**,
defaulting to **12**, raised only when `L_Data_Extended` support has been
positively established for that device. Defaulting to 63 and falling back on
error means the first failure mode is a half-written memory region, which §13
rates as the worst outcome in this document.

Whether a given device supports `L_Data_Extended`, and how a client is meant to
discover that, is **not stated in MP §3.16**; the clause states the consequence,
not the discovery mechanism. **Undocumented in both knowledge bases as
searched.** Phase 2 therefore treats 12 as the value and offers no automatic
promotion.

### 6.5 Above 64 kB: `A_UserMemory_*` and `A_MemoryExtended_*`

**[D]** CP §3.5.1.4 with Table 5: `A_Memory_Write` addresses up to 64 kB,
`A_UserMemory_Write` up to 1 MB, and `PID_TABLE_REFERENCE` is limited to 20
bits. CP §3.5.2's own step list makes the selection rule explicit and it is a
straight comparison, not a heuristic: *"if BaseAddress plus allocated memory is
lower than FFFFh then MaC: MemoryWrite(BaseAddress, \<Data\>, Length); if
BaseAddress plus allocated memory is higher than FFFFh then MaC:
UserMemoryWrite(BaseAddress, \<Data\>, Length)"*.

Note the condition is on **base + length**, not on the base alone: a region
that starts below `FFFFh` and ends above it uses the user-memory service. An
implementation that switches on the start address will write the tail of a
segment to the wrong service.

`A_MemoryExtended_*` appears in AL Table 1 and is listed here for completeness;
this document specifies no procedure using it, and none of the cited procedures
in **MP** or **CP** uses it either. Out of scope for phase 2.

### 6.6 The services, by APCI

Read from **AL** Table 1 in a `pdftotext -layout` extraction of the source PDF
(see §1.1 on why this is now citable). Spot-checked against this project's
shipped code: `A_DeviceDescriptor_Read` is `1100000000` = `0x300`, which is
exactly `cemi.rs`'s `0x300 | descriptor_type`. **[D]** for the encodings,
**[V]** for that one cross-check.

The services phase 2 needs, and where they come from:

| Service | Present in `knx-net` today | Notes |
| --- | --- | --- |
| `A_DeviceDescriptor_Read` / `_Response` | yes, typed | `0x300 \| dt` / `0x340 \| dt` |
| `A_IndividualAddress_Read` / `_Response` / `_Write` | no | Broadcast; §4 |
| `A_Memory_Read` / `_Response` / `_Write` | no | §6.1, §6.2 |
| `A_PropertyValue_Read` / `_Response` / `_Write` | no | §5, §7 |
| `A_Restart` (+ response) | no | §8 |
| `A_Authorize_Request` / `_Response` | no | §7.1 step 03 |
| `A_UserMemory_Read` / `_Write` | no | §6.5 |

`ApplicationService::Other { apci, data }` already exists as an escape hatch,
and `Tpci` already models Connect/Disconnect/NumberedData/Ack/Nak. That means
phase 2 adds **typed variants**, not new transport machinery: the frame layer
can already carry every one of these, which is why this specification can be
implemented without touching `client.rs`.

## 7. Application-program download

### 7.1 What a Management Client sends, in what order (CP §3.5.2)

The Standard specifies this as a numbered table of Device Management Procedures
for a Management Client (MaC) driving a Management Server (MaS, i.e. the
device). **ETS is one such Management Client.** The table below is therefore
what the Standard requires of a conforming client; whether ETS deviates from it
in any particular step is **not documented in either knowledge base** and is
not asserted here. This distinction matters because the brief asks "what ETS
sends" and the honest answer is "what the Standard says a client sends, plus an
unobserved delta".

Complete download, System B (CP §3.5.2), steps as specified:

| # | Procedure | What it does |
| --- | --- | --- |
| 01 | Connect | Connect via bus (connection-oriented) |
| 02 | Verifying device version | Read Device Descriptor Type 0 |
| 03 | Get access rights | Authorize |
| 04 | Check Manufacturer ID | *"Check if expected Manufacturer ID == DeviceObject.PID_MANUFACTURER_ID"* |
| 05 | Unload device | Set `LoadControl = Unload` on Application Program 2, Application Program 1, Group Object Table, Association Table, Address Table; then wait until each reports `LoadState = Unloaded` |
| 06 | Loading Application Program 2 | see §7.2 |
| 07 | Loading Application Program 1 | *"(For details of the loading process apply the routines of 'application program 2' accordingly:)"* |
| 08 | Loading the Group Object Table | as 06 |
| 09 | Loading the Address Table | as 06, plus *"Load group responser table via property write"* — footnoted *"Applicable for PL110 devices only."* |
| 10 | Loading the Association-Table | as 06 |
| 11 | Modifying access keys | *"Set access keys as required"* |
| 12 | Disconnect | Disconnect via bus |

Three things to read carefully. Step 04 is a **guard**: the manufacturer ID is
compared before anything is written, which is the Standard's own protection
against downloading one manufacturer's application into another's device.
Step 05 unloads **everything before loading anything** — so a complete download
passes through a state where the device holds no valid configuration at all, by
design, and §13 rates that. And step 09's group responser table is medium-
specific; on TP1 it does not apply.

### 7.2 The inner loop: loading one loadable part (CP §3.5.2 step 06, quoted)

1. **Start.** `MaC: Set ApplicationProgram_2.LoadControl = Load` →
   `MaS: Set ApplicationProgram_2.LoadState = Loading`.
2. **Allocate.** `MaC: Set ApplicationProgram_2.LoadControl = Data Relative
   Allocation`.
3. **Read back the base address.**
   `MaC: PropertyRead(ID_ApplicationProgram_2, PID_REFERENCE)` →
   `MaS: PropertyResponse(...) = BaseAddress`. Quoted: *"Base Address is a 4
   octet absolute value; if it is zero then allocation was not successful. This
   causes an error message of the MaC to the Installer."* Zero is a failure
   value, not an address. The implementation must check it before it computes a
   single write offset.
4. **Write the data** by direct memory access, choosing the service per §6.5's
   `BaseAddress + length` rule, in chunks per §6.4.
5. **Set the version.**
   `MaC: PropertyWrite(ID_ApplicationProgram_2, PID_PROGRAM_VERSION)`.
6. **Complete.** `MaC: Set ApplicationProgram_2.LoadControl = LoadComplete` →
   `MaS: Set ApplicationProgram_2.LoadState = Loaded`, waited for per §5.5.
7. **Store the checksum.** *"Read Property Memory Control Block and save CRC
   checksum."*

Step 7 is not bookkeeping — it is the input to §7.4. A client that skips it can
never do a partial download afterwards.

### 7.3 Allocation: the `Additional Load Controls` payloads

MP §3.31.3's `DM_LoadStateMachineWrite_RCo_IO` specifies the property write as
**exactly 10 octets** to `PID_LOAD_STATE_CONTROL` (property_id 5, start_index
`01h`, nr_of_elem `01h`), with these payloads:

| Event | 10-octet payload |
| --- | --- |
| No Operation | all `00h` |
| Start Loading | `01h` + 9 × `00h` |
| Load Completed | `02h` + 9 × `00h` |
| Unload | `04h` + 9 × `00h` |
| Additional Load Control | `03h` + subtype + 8 octets of subtype-specific fields |

The Additional subtypes and their field layouts **are specified** in MP §3.31.3
— this narrows RESEARCH §8.6's statement that *"the argument encodings are the
part inference cannot supply"*:

| Subtype | Name | Fields |
| --- | --- | --- |
| `00h` | AllocAbsDataSeg | start address, length, access attributes, memory type (1 = Zero page RAM, 2 = RAM, 3 = EEPROM), memory attributes (bit 7 = checksum control) |
| `01h` | AllocAbsStackSeg | as documented in the clause |
| `02h` | AllocAbsTaskSeg | start address, PEI type, application ID (`MM MM TT TT VV`) |
| `03h` | TaskPtr | per clause |
| `04h` | TaskCtrl1 | per clause |
| `05h` | TaskCtrl2 | per clause |
| `0Ah` | Relative Allocation | number of octets requested. **[D]** *"If the requested number of octets is not supported by the Management Server (device) then the Load State Machine of the loadable part shall change to error."* |
| `0Bh` | Data Relative Allocation | 4-octet requested memory size + Mode/fill octet; Mode bit 0 selects keep vs fill |

`0Bh` is the one CP §3.5.2 step 06 uses. Note the consequence attached to
`0Ah`: asking for more memory than the device has does not return an error
code — it moves the LSM to `Error`, from which only `Unload` escapes (§5.4).

What is **not** documented: the mapping from the product-data `LdCtrl*` element
names to these subtypes. RESEARCH §8.6 measured 25 distinct `LdCtrl*` kinds in
`knx_master.xml`, of which 12 could be matched to documented load controls and
13 could not, and the names themselves return zero hits across the extracted
KNX Standard corpus. §12 states what that forbids.

### 7.4 Partial download (CP §3.5.3)

The shape is the same with one insertion and one branch. Steps 01–04 are
identical (Connect, Read Device Descriptor Type 0, Authorize, Check
Manufacturer ID). Then:

- **Unload only the part being replaced** (step 05: Application Program 2
  alone), instead of everything.
- After allocation and the `PID_REFERENCE` read-back, **compare the CRC**:
  `MaC: PropertyRead(ID_ApplicationProgram_2, PID_MCB)` →
  `MaS: PropertyResponse(..., PID_MCB, Data)`, and quoted: *"The current CRC
  shall be responded and shall be compared with the stored CRC. If the CRC
  matches, then MaC shall use differential download algorithm."*
- **If allocation fails** (base address zero), the procedure does not abort —
  it *"⇒ Continue at Nr. 07"*, which is "unload all the following segments" and
  then reload them in order. In other words the Standard's own recovery from a
  failed partial allocation is **escalation to a larger download**, because the
  segments are laid out in ascending memory order and CP §3.5.3's opening note
  says so: *"Due to the ascending order of the memory segments it can be
  necessary to rearrange all or a subset of the segments when modifying one
  segment."*

Two hard consequences for the implementation. A partial download **can turn
into a full download mid-flight**, so the operator-facing progress model must
allow for it and the pre-flight checks must have gathered everything a full
download needs — otherwise the escalation path runs with missing data.
And the *"differential download algorithm"* itself is named and not specified:
**undocumented in both knowledge bases as searched**. Phase 2 therefore
implements the CRC **comparison** and, on a match, may skip the part
legitimately (nothing to change) — but must not claim to implement differential
download, because the algorithm that name refers to is not in the corpus.

### 7.5 Unload (CP §3.5.4)

**[D]** CP §3.5.4: *"The load procedure shall be connection oriented."* Steps:
01 Connect, 02 Read Device Descriptor Type 0, 03 Authorize, 04 Check
Manufacturer ID, 05 set `LoadControl = Unload` on Address Table, Association
Table, Object Table, Application Program 2 and Application Program 1, then
*"Wait until load states == unloaded"*, 06 Disconnect, and finally:

07 **Unload IndividualAddress** — *"Set
SerialNumber_IndividualAddress_Write(FFFFh) via broadcast"*, then *"Check if
SerialNumber_InidividualAddress_Read() == FFFFh (via broadcast)"*.

Step 07 makes a device unaddressable by design, over broadcast, keyed by serial
number. It is the single most dangerous operation described in this document
and §13 treats it accordingly. Phase 2 implements steps 01–06 only. Step 07 is
specified here so that it is recognisable, not so that it is built.

CP §3.5.4 closes with a warning aimed at manufacturers that is also a warning
to clients: **[D]** *"The manufacturer of the Application Program 1 and the
Application Program 2 shall take care that the device cannot reach a critical
state if the load state machine of one or more segments changes from 'loaded'
to a different state."* The Standard is telling us that leaving a segment
un-`Loaded` is a foreseeable condition that devices are required to survive —
required of the manufacturer, which is not the same as guaranteed by the device
in front of you.

### 7.6 Allocation only happens in `Loading`

**[D]** CP §3.5.1.2, Table 4 (Load Controls for System B): *"An allocation (if
necessary) shall occur if a segment is in the state Loading. In all other
states the memory allocation shall be ignored."* A re-allocation frees the
previous allocation and retries at the original base address. Unload frees the
memory and zeroes `PID_TABLE_REFERENCE`.

"Ignored" is the operative word: allocating outside `Loading` produces no error
and no allocation. A sequence that allocates before Start Loading will read a
`PID_REFERENCE` of zero and conclude, correctly but confusingly, that
allocation failed.

## 8. Restart and reset

**[D]** AL §3.4.2.2 specifies `A_Restart` with a `restart_type` field:

- **`restart_type = 0`, Basic Restart** — no further data, **unconfirmed**.
- **`restart_type = 1`, Master Reset** — carries an Erase Code and a Channel
  Number, and is **confirmed**, with the response carrying an Error Code and a
  Process Time.

Reserved bits are not tolerated: **[D]** if they are not 0 the device *"shall
ignore the service totally. (No negative response shall be sent.)"* Defined
Error Codes include *"Unsupported Erase Code"* and *"Invalid Channel Number"*.
And a point that breaks naive session handling: **[D]** *"The Management Server
(device) may or may not break down the Transport Layer connection."* So after a
restart the client must treat its connection as being in an unknown state and
re-establish rather than branch on whether a disconnect arrived.

The Master Reset response is specified exactly (MP §3.7 area): the device
responds with *"the field Response set to 1"*, *"all bits 4 to 1 of the
APCI/ASDU cleared"*, `restart_type = 1`, an Error Code field and a Process Time
field.

### 8.1 Erase Codes (MP Table 4, reproduced)

| Code | Name | Effect, quoted or summarised from MP Table 4 | Channel Number |
| --- | --- | --- | --- |
| `00h` | — | *"These values are reserved."* | — |
| `01h` | Confirmed Restart | *"No Resource value shall be reset. This encoding shall allow using the Master Reset as a confirmed alternative to the unconfirmed Basic Restart."* | fixed `00h` |
| `02h` | Factory Reset | *"This shall reset the device to its ex-factory state. Which Resources are reset and their value after reset are implementation dependent."* | `00h` = all channels; else that channel |
| `03h` | ResetIA | *"The IA shall be reset to the medium specific default IA when this A_Restart is executed."* | fixed `00h` |
| `04h` | ResetAP | *"Application Program Memory shall be reset to the default application when an A_Restart-PDU is received."* | fixed `00h` |
| `05h` | ResetParam | *"Application Parameter Memory shall be reset to its default value when an A_Restart-PDU is received."* | `00h` = all channels; else that channel |
| `06h` | ResetLinks | *"Link information for Group Objects (Group Address Table, Group Object Association Table) shall be reset to default state when an A_Restart-PDU is received."* | `00h` = all channels; else that channel |
| `07h` | Factory Reset without IA | ex-factory state, but *"Opposite to Erase Code 02h, the Individual Address shall not be reset."* | `00h` = all channels; else that channel |
| `08h` | Erase persistently stored application data | *"Persistently stored application data … shall become invalid when an A_Restart-PDU is received."* | `00h` = all channels; else that channel |
| `09h`–`FFh` | — | *"The Management Client shall not use these Erase Codes."* The device shall *"neither execute a Basic Restart nor any Master Reset"* and shall respond with Error Code *"Unsupported Erase Code"*. | not defined |

Design rules that fall straight out of this table:

1. **`01h` Confirmed Restart is the restart the implementation should use**, not
   `restart_type = 0`. Same effect, but it answers, so the client learns whether
   it happened. An unconfirmed restart in the middle of a download leaves the
   client guessing, and §5.4's `Device Restart` row means the guess matters.
2. Codes `02h`, `03h`, `04h`, `07h` are destructive in ways a user cannot undo
   from this application: `03h` and `02h` make the device unaddressable at its
   current address, `04h` discards the application. They belong behind the
   mutation API (§2.3) with a per-operation confirmation naming the specific
   effect, and none of them is implemented in phase 2.
3. *"Which Resources are reset and their value after reset are implementation
   dependent"* means the application **cannot** tell the user what a factory
   reset will do to their device. It must not pretend to. It can only say what
   the Standard says, which is "ex-factory state, contents unspecified".
4. The *"implementation dependent"* wording also means no post-reset state can
   be asserted in a test against a real device; phase 3 must re-read rather
   than assume.

`AN194 Master Reset of Resources` is the application note that elaborates on
which network resources a Factory Reset must touch (MP §3.7.1.2.3.2 points at
the same requirements). It was **not** read for this document; nothing here
depends on it, and anything in phase 2 that would depend on it must read it
first.

## 9. Error handling, and what an interrupted download leaves behind

This is the section an implementer will come back to, so it states the answer
before the reasoning.

**An interrupted download leaves the device with an invalid loadable part,
persistently, and a power cycle does not fix it.** Recovery requires a client.

The chain of cited facts:

1. **State survives power loss.** **[D]** RES §4.23.1: *"The state of the Load
   State Machine shall be stored in non-volatile memory (e.g. EEPROM, flash)."*
2. **Only `Loaded` is valid.** **[D]** RES Table 92: *"Only in this state the
   associated data shall be considered as valid; in all other states the data
   shall be considered as invalid."*
3. **A restart during `Loading` stays in `Loading` or goes to `Error`.**
   **[D]** RES Table 94, `Device Restart` row: from `Loading`, `R: Loading` /
   `O: Error`. Either way, not `Loaded`. A device interrupted mid-download comes
   back up with an invalid part and stays there.
4. **A restart from `Loaded` can still land in `Error`.** Table 94's
   `Device Restart` / `Loaded` cell reads *"Loaded (Error in case of error
   detection at start-up)"* — a device that self-checks at boot may downgrade a
   previously good part.
5. **`Error` only exits via `Unload`.** §5.4. There is no "resume".
6. **`Unload` makes the data explicitly undefined.** **[D]** RES Table 93:
   *"The loadable data shall be declared as invalid. The data is undefined."*
   So recovery is destructive by construction: you cannot inspect what was
   half-written, you can only discard it.

### 9.1 The recovery procedure

```text
1. Connect, read Device Descriptor Type 0, Authorize, check Manufacturer ID.
2. Read PID_LOAD_STATE_CONTROL for every loadable part.        (per-part, §3.1)
3. Read PID_ERROR_CODE *before* unloading anything.            (RES §4.2.28)
4. For each part not in Loaded: write Unload (04h), wait per §5.5,
   accept Unloaded directly without observing Unloading.       (§5.4)
5. Re-run the complete download of §7.1 from step 06.
6. Confirm every part reports Loaded. Anything else is a failed recovery,
   reported as such — not retried in a loop.
```

Step 3 before step 4 is not cosmetic: **[D]** RES §4.2.28 says the error code is
cleared when the state leaves `Error`, so unloading first destroys the only
evidence of what went wrong.

### 9.2 Failures the implementation must distinguish

| Observation | What it can mean | What the code must not do |
| --- | --- | --- |
| No response to `A_Memory_Write` | Lost frame, **or** protected/nonexistent target (AL §3.5.4: *"the service indication shall be ignored"*) | Retry indefinitely as if it were a lost frame |
| No response during `LoadCompleting` | Normal — the device may be offline in this state (RES Table 94 footnote) | Treat as failure |
| State unchanged after an event write | Illegal or unknown event, **or** genuinely no-op (RES §4.23.2.3.2) | Assume the event was accepted |
| `PID_REFERENCE` reads 0 | Allocation failed (CP §3.5.2), **or** allocation was attempted outside `Loading` and ignored (CP §3.5.1.2) | Use 0 as a base address |
| State is `Error` after Start Loading | Requested allocation unsupported (MP §3.31.3 `0Ah`), or a data error | Re-send Start Loading |
| Step 4 of §4.2 fails | *"the programming of the Individual Address may have failed, or the system (Router) is not configured correctly"* (MP §2.3) | Report only the first cause |
| Write succeeded but read-back differs | MP §3.16: *"different or no data received ⇒ error"* | Accept the write |

Every row exists because the two causes need different handling and the wire
cannot distinguish them. Where it cannot, the implementation reports both — per
`CLAUDE.md`'s "do not hide uncertainty".

### 9.3 Transaction semantics: there are none

There is no rollback in any procedure cited in this document. There is no
"stage then commit". `Start Loading` from `Loaded` invalidates the part
immediately (§5.4), and the only way back to a working device is a successful
download. The implementation must therefore:

- treat the interval between the first `Start Loading` and the last confirmed
  `Loaded` as a **window in which the device is non-functional**, and say so in
  the UI before entering it rather than after;
- never begin a download it does not have all the data to finish — the full
  payload for every part it intends to unload must be resolved, in memory and
  validated, before the first event is written;
- persist enough of its own progress that a crash of the **application** does
  not lose track of which part was being loaded, because the device cannot be
  asked "what were we doing" beyond its per-part load state.

## 10. Authorisation

CP §3.5.2 step 03 is "Get access rights — Authorize", using
`A_Authorize_Request`/`A_Authorize_Response` (AL Table 1), specified as a
procedure in MP §3.5 `DM_Authorize`. CP §3.5.2 step 11 is "Modifying access
keys — Set access keys as required".

What this document does **not** specify: which key a client should present, how
a project stores it, what the default key is, or what happens on a failed
authorisation in the middle of the download sequence. Those were not verified
for this document. `DM_Authorize` is cited as existing and as being step 03;
its parameters are **not** reproduced here because they were not read, and
inventing them would be exactly the failure mode §1 forbids. Phase 2's
simulator should accept the procedure and phase 2 should implement it against
what MP §3.5 actually says, read at that time.

AL §3.4.4.2's NOTE 10 on Access Policy read-back failures is relevant to the
same area and is likewise cited rather than paraphrased.

## 11. How this maps onto the existing code

Nothing below is a protocol claim; this section is design, marked **[A]** as a
whole, and is the part a reviewer should push back on freely.

### 11.1 New, in `knx-net`

- Typed `ApplicationService` variants for the services in §6.6. They currently
  have to go through `Other { apci, data }`, which works but puts encoding
  knowledge at every call site. The APCI values are now citable (§6.6), so the
  variants can be added without guessing.
- A connection-oriented session type. `Tpci` already models
  Connect/Disconnect/NumberedData/Ack/Nak; what is missing is the sequence
  numbering, the 3 s acknowledge time-out, the 6 s connection time-out and
  `max_rep_count = 3` behaviour recorded in RESEARCH §8.5 from **TL** clause 4.
  Every procedure in §7 is connection-oriented (**[D]** CP §3.5.4: *"The load
  procedure shall be connection oriented."*), so this is a prerequisite, not an
  optimisation.
- The `ScanTransport` trait is the right shape to extend or sit beside:
  `assigned_address()`, `subscribe()`, `send_frame(destination, transport,
  service)`. A `ManagementTransport` with the same fake-able shape keeps the
  domain testable with no socket.

### 11.2 New, in `knx-core`

- **A procedure model, not a script.** Each cited procedure (§4.2, §7.2, §7.4,
  §7.5, §9.1) becomes a declarative step list that can be *rendered* for the
  user and *dry-run* without a bus. This is the same conclusion RESEARCH §8.6.6
  Slice 0 reached from the product-data side, and the two meet here: the step
  list comes from the procedure spec, the per-product parameters come from
  `LoadProcedureStyle`/`LdCtrl*` data.
- **`LoadState` and `LoadEvent` as separate enums**, because §5.1 says read and
  write use different encodings of the same PID. One enum with a direction flag
  will be misused.
- **A transition table derived from §5.4**, including the alternative
  `R:`/`O:` outcomes, so "did the device do something legal" is a table lookup
  rather than scattered `if`s. This is testable with no hardware at all and
  should be the first test written in phase 2.
- **`CommissioningState` grows**, it is not replaced. The existing type
  (`crates/knx-core/src/commissioning.rs`) already has
  `individual_address_loaded`, `application_program_loaded`,
  `parameters_loaded`, `communication_part_loaded`, `medium_config_loaded`,
  `last_download` and `broken` — which is a per-part model already, and maps
  onto §3.1's per-part LSMs more closely than it looks. What it lacks is the
  distinction between "we believe this is loaded" (project-side intent, what it
  models today, matching the ETS project schema) and "the device reported
  `Loaded`" (device-side fact). Those must be separate fields. Conflating them
  produces an application that tells the user a download succeeded because it
  was asked to succeed.
- **The exclusion guard of §2.1**, shared with `scan::ScanPlan`.

### 11.3 Phase 2's simulator

A device simulator that implements §5.4's table, §6's length limits including
the 12-octet cap, §7.6's "allocation only in `Loading`" rule, and the failure
modes of §9.2 — including the ones that present as silence. It must be able to
be told to fail at a chosen step, because §9 is the half of this specification
that cannot be tested any other way. It is also the only legitimate target of
the mutation API until the user says otherwise (§2.3).

## 12. The boundary: what is not documented, and what that forbids

Everything in this section was searched for in both KNX specification knowledge
bases and, where relevant, in the extracted Standard corpus and the product
corpus of RESEARCH §8.6. These are gaps, not summaries of gaps.

1. **The product-specific download matrix.** The *semantics of each individual
   `Legacy*` flag* are absent. RESEARCH §8.6 measured that all 13 program-level
   `Legacy*` names return **zero hits** across the entire extracted KNX
   Standard corpus, and the Standard acknowledges only the category: **[D]** CP
   §3.4.1.2.1 footnote 6: *"This is an option for the Management Client. For the
   common tool ETS®, this can be controlled via a flag in the database entry
   for the product."* That is the Standard telling us the behaviour is a tool
   setting, without telling us what the setting does.
   **Forbidden:** the implementation may not infer any flag's behaviour from its
   name, may not implement a "legacy" code path keyed on a flag whose meaning is
   unknown, and may not claim ETS-equivalent download for any product whose
   flags it cannot interpret. What it may do is **record** the flags and
   **refuse** to download products that set flags it does not understand —
   refusing is safe, guessing is not.
2. **The `LdCtrl*` → load-control mapping, for 13 of 25 kinds.** RESEARCH §8.6
   matched 12 of the 25 `LdCtrl*` element kinds in `knx_master.xml` to
   documented load controls; the other 13 have no documented counterpart, and
   the element names themselves appear nowhere in the Standard corpus. §7.3
   documents the payload layouts for the events the *documented* ones map to.
   **Forbidden:** emitting an `Additional Load Control` for an `LdCtrl*` kind
   whose subtype mapping is not established. Note the specific hazard from
   §7.3: an unsupported allocation request does not return an error, it drives
   the LSM to `Error` (MP §3.31.3 subtype `0Ah`), from which only `Unload`
   escapes. Guessing here costs a configuration, not an error message.
3. **Vendor DLL involvement in download.** RESEARCH §8.6 established **[V]**
   that no vendor DLL is *required* to reconstruct a download sequence — the
   step list is declarative product data — and that `EtsDownloadPlugin` appears
   on 5 of 35 corpus application programs without supplying the step list. What
   those plugins *do* is undocumented and, being compiled code, is not
   documentable from either knowledge base.
   **Forbidden:** claiming a device with an `EtsDownloadPlugin` is supported.
   The correct behaviour is to detect the hook, report it, and decline. See also
   `KNOWN_LIMITATIONS.md` §6.
4. **The "differential download algorithm"** named in CP §3.5.3. The CRC
   comparison that gates it is specified; the algorithm is not.
   **Forbidden:** implementing or claiming differential download. Permitted:
   comparing the CRC and skipping an unchanged part.
5. **The `L_Data_Extended` discovery mechanism.** MP §3.16 states the
   consequence (12-octet cap without it) but not how a client learns whether a
   device supports it. **Forbidden:** writing more than 12 octets per
   `A_Memory_Write` on the basis of an assumption.
6. **The programming-mode parity rule** at memory address `60h` (§4.4). The bit
   position is documented; the computation is not, beyond *"The parity (bit 7)
   has to be calculated."*
   **Forbidden:** writing to `60h`.
7. **The unquantified "delay for programming the memory in the device"**
   (MP §3.16, non-verify path). Named, never given a value.
   **Forbidden:** presenting any chosen default as a specification value.
8. **LSM Realisation Type 2.** **[D]** RES §4.23.3: *"This Realisation Type is
   not specified in this version of this document."*
   **Forbidden:** claiming support for devices that require it.
9. **Cross-LSM ordering between loadable parts**, as a general rule. CP §3.5.2
   gives one concrete order for System B, which this document follows. The
   general dependency (e.g. Address Table before Association Table as a
   *requirement* rather than as one table's row order) lives in `06 Profiles`
   and the per-mask Configuration Procedures, neither of which was read for
   this document.
   **Forbidden:** generalising CP §3.5.2's order to masks it does not cover.
10. **`DM_Authorize`'s parameters** (§10) and the `DPT_ErrorClass_System`
    20.011 enum values (§5.5) — both exist, both are cited as existing, neither
    was read from its source PDF for this document. They are gaps in *this
    document*, not in the corpus: they are readable, they were simply not read.
    Phase 2 reads them.

The distinction in item 10 is worth preserving in the implementation's own
notes. "Not in the Standard" and "not yet read" require completely different
responses, and a document that blurs them is the failure mode §1 exists to
prevent.

## 13. Risk register: what a wrong implementation does to a real device

Ordered by how bad the outcome is, not by how likely it is. "Recoverable by"
means recoverable using only what this application could implement.

| # | Operation | Getting it wrong does this | Recoverable by |
| --- | --- | --- | --- |
| R1 | Any operation targeting `1.1.220` | Reads or writes to an **alarm panel**. Consequences are outside this application's model entirely — a false alarm, a suppressed alarm, or a panel in an unknown state. No protocol-level recovery is meaningful. | Nothing. This is why §2.1 is structural. |
| R2 | Broadcast `A_IndividualAddress_Write` with more than one device in programming mode | Two or more devices take the same individual address. The line then has an address collision that cannot be resolved by address, because both answer to it. Physical access to each device is required. | Site visit |
| R3 | `A_Restart` Master Reset with Erase Code `02h`/`03h` | Device reset to ex-factory state and/or its IA reset to the medium default. **[D]** MP Table 4: the reset resources *"are implementation dependent"*, so the application cannot state what was lost. The device is no longer at the address the project believes. | Re-commissioning from the programming button, if the project still holds the configuration |
| R4 | `SerialNumber_IndividualAddress_Write(FFFFh)` (CP §3.5.4 step 07) | Device becomes unaddressable by individual address, by broadcast, keyed by serial number. | Knowing the serial number, or a site visit. Not implemented in phase 2 |
| R5 | Writing memory with a chunk size the device does not support (§6.4) | Partial write of a memory region. **[D]** AL §3.5.4: a partially protected target means *"the complete write operation shall fail"*, and an over-long request is *ignored*, so the client may believe it wrote data it did not. Result: a `Loaded` part containing wrong bytes — the worst outcome in this table, because the device reports success. | Full re-download, **if** the corruption is noticed |
| R6 | Skipping the mandatory read-back (§6.2) | Same as R5, reached by a different route: **[D]** AL §3.5.4 requires *"The value of the associated memory area shall be explicitly read back after writing to it."* Without it, silent corruption is undetectable. | Full re-download |
| R7 | Emitting an `Additional Load Control` with a guessed subtype or size (§7.3, §12 item 2) | **[D]** MP §3.31.3 subtype `0Ah`: an unsupported request drives the LSM to `Error`. §5.4: only `Unload` leaves `Error`, and **[D]** RES Table 93: unload makes the data *"undefined"*. The part is destroyed, not merely failed. | Unload + full re-download |
| R8 | Beginning a download without the full payload resolved | `Start Loading` from `Loaded` immediately invalidates a working configuration (§5.4). If the data is then unavailable, the device stays invalid indefinitely — and the state is non-volatile, so power-cycling does not help. | Full re-download once the data exists |
| R9 | Interrupting a download (network, crash, operator) | Part left in `Loading` or `Error` after restart (**[D]** RES Table 94 `Device Restart` row: from `Loading`, `R: Loading` / `O: Error`), invalid per Table 92, persistent per §4.23.1. | §9.1 recovery |
| R10 | Treating `LoadCompleting` silence as failure and retrying | Writing load events at a device that is *"offline during state LoadCompleting"* (**[D]** RES Table 94 footnote). Per Table 94, `Start Loading` or `Load Completed` from `LoadCompleting` is `R: Error`. A correct device mid-checksum is driven into `Error` by the client's impatience. | Unload + re-download |
| R11 | Writing to `60h` with a guessed parity (§4.4) | An out-of-spec byte at the device-control address. The effect is not documented, which is precisely why it must not be done. | Unknown, which is the point |
| R12 | Polling the load state faster than the spec allows, or not reconnecting | **[D]** RES §4.23.2.4.1 caps the read period at 3 s and requires periodic reconnection. Too-fast polling loads the bus during a download; failing to reconnect makes a legitimate long transition look like a failure, which leads to R10. | Fix the client |
| R13 | Skipping the Manufacturer ID check (CP §3.5.2 step 04) | One manufacturer's application downloaded into another's device. The Standard puts this check before any write for exactly this reason. Outcome is undefined and device-specific. | Unload + correct download, if the device still communicates |
| R14 | Assuming programming mode is still on (§4.3) | The write is ignored (AL §3.2.2, no negative response), the operator believes the address was programmed, and the project's model of the installation diverges from the installation. Silent, and discovered later at the worst moment. | Re-run §4.2 |
| R15 | Recording "downloaded" from intent rather than from a device read (§11.2) | The project claims a device is configured when it is not. Every later decision — group address assignment, diagnostics, the next partial download — is built on a false premise. | Re-read device state |
| R16 | Losing Verify Mode on reconnect (§6.3) | Writes silently stop being confirmed (**[D]** RES §4.2.14.7.4: it is auto-disabled when the TL connection closes), degrading to R5 without any visible change. | Re-assert bit 2 per connect |

R1 has no recovery row because it has no recovery. That is the argument for
enforcing it in the type system rather than in a review checklist.

## 14. Testing (phase 2, no hardware)

1. **The transition table of §5.4**, exhaustively: 6 states × 6 events, with
   the `R:`/`O:` alternatives both accepted. Pure, no I/O, and it is the test
   that would have caught R7 and R10.
2. **`Error` is a trap**: assert that no event other than `Unload` leaves it.
3. **Chunking** at the 12-octet default and at 63, including a region that
   straddles `FFFFh` so the §6.5 service-selection rule is exercised on
   `base + length`, not on `base`.
4. **Read-back enforcement**: a write path without a read-back must not compile,
   or at minimum must not exist — asserted by the simulator rejecting a session
   that writes without reading back.
5. **Verify Mode across a reconnect**: the simulator drops the connection, and
   the client is asserted to re-set `PID_DEVICE_CONTROL` bit 2 before the next
   write.
6. **The §4.2 counting rules**: zero responders, one responder, two responders,
   and one responder whose frame is repeated at Layer 2 — the last must count
   as one device (MP §2.2).
7. **Occupied-address refusal**: step 1 of §4.2 finds `IA_new` occupied and the
   procedure stops before any broadcast.
8. **Interruption at every step** of §7.2, each followed by §9.1, asserting the
   device ends `Loaded` and that `PID_ERROR_CODE` was read before the unload.
9. **`PID_REFERENCE` = 0** handled as failure in both its meanings (§9.2).
10. **Partial-download escalation**: a failed allocation continues at step 07
    and completes as a full download (§7.4).
11. **The exclusion guard**: a plan containing `1.1.220` cannot be constructed,
    the attempt is reported rather than silently dropped, and a broadcast
    operation is refused when the programming-mode count is not exactly one.
12. **Property-based**: any sequence of events applied to the model LSM never
    reports `Loaded` unless a `Load Completed` from `Loading` succeeded.

Phase 3, read-only, within `1.1.24`–`1.1.32` only: read every loadable part's
`PID_LOAD_STATE_CONTROL`, `PID_ERROR_CODE`, `PID_DEVICE_CONTROL`,
`PID_OBJECT_INDEX`, Device Descriptor Type 0 and `PID_MANUFACTURER_ID`, and
compare the observed shapes against this document. Deviations are findings, and
the expected outcome is that there will be some.

## 15. Non-goals

- **No code in phase 1.** No bus access in phase 1. This document is the whole
  deliverable.
- **No writes to real hardware in phase 2 or phase 3**, and no write at all
  without a fresh, specific go-ahead naming the device and the operation.
- **No claim of ETS parity.** §7.1 specifies what the Standard requires of a
  Management Client. Whether ETS matches it step for step is unobserved, and
  `CLAUDE.md` forbids claiming compatibility that has not been verified.
- **No KNX Secure.** Secure commissioning needs key handling this project does
  not have (`KNOWN_LIMITATIONS.md` §9). An unsecured procedure against a secured
  device is not in scope and must be detected and refused rather than attempted.
- **No `A_MemoryExtended_*` procedures**, no LSM Realisation Type 2, no
  mask-`070nh` `_RCo_Mem` variant, no `DM_ProgMode_Switch` write path, no
  Master Reset with a destructive Erase Code, no `SerialNumber_
  IndividualAddress_Write`.
- **No product-database schema change** is specified here. RESEARCH §8.6.5's
  parsing addition (storing `Options`, `LoadProcedures` and `LdCtrl*`) remains
  the separate prerequisite it was, and phase 2 depends on it for real product
  data but not for the simulator.
- **No UI.** The procedure model of §11.2 is designed to be renderable, which
  is a different thing from being rendered.

## 16. Gates

Documentation-only change. Run to prove nothing broke: `cargo fmt --all
--check`, `cargo run -p xtask -- check-headers`, `cargo test --workspace
--no-fail-fast`. `cargo clippy`, `cargo deny` and the web gates are skipped —
no Rust source, no manifest and no web code is touched by this commit, so they
have nothing new to judge. ADR-0018 file headers do not apply to Markdown under
`docs/`.
