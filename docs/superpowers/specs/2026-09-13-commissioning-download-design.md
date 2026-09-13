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
- **Revision, 2026-09-13:** four places where the first draft said "documented,
  but not read for this document" were read and written up, because that
  formulation is unfinished work rather than a finding. New: §5.6
  (`DPT_ErrorClass_System` 20.011, all nineteen values), §8.2 (AN194's
  per-resource reset semantics), §10 (authorisation, rewritten from a paragraph
  into nine subsections that phase 2 can implement from), and PROF's constraints
  on the Load State Machine in §3.4, §5.4, §6.3, §7.3 and §10.5. One item
  previously listed as undocumented turned out to be documented and is corrected
  in place: §6.4/§12 item 5, `L_Data_Extended` discovery via
  `PID_MAX_APDU_LENGTH`. Seven genuine gaps remain (§12).
- **Revision, 2026-09-14 (fix round 2):** ten review findings corrected. The
  largest is §4.4: the `60h` parity computation **is** documented (RES
  §4.26.3.1 and §4.26.3.4.1) and is written down here as a toggle, which
  removes it from the gap list — and removes an accidental protection with it,
  so the safety carry-through is explicit: `DM_ProgMode_Switch`'s write half
  stays inside §2.3's mutation API, stays out of phase 3, and **R11 in §13 is
  now "writing to `60h` at all on a real device"**, not "writing with a guessed
  parity". Also: §6.2's read-back is this project's requirement and not the
  Standard's obligation on the client (§13 R6 restated to match), §10.4's
  `DM_Authorize2_RCo` is profile-scoped and therefore an opt-in extension over
  MP §3.5.1, §8.2's AN194 rows are split per Interface Object, and §12's gaps
  carry stable `GAP-T30-nn` identifiers. **Six genuine gaps remain (§12)**,
  plus one narrowed to a delegation.

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
| **DPT** | `03_07_02 Datapoint Types v02.02.01 AS.pdf` |
| **AN194** | `AN194 v02 Master Reset of Resources AS.pdf` |
| **PROF** | `06 Profiles v02.01.01.pdf` |

Eight documents. Everything protocol-level in this specification comes out of
those eight. The first five carry the procedures; the last three were added in
the 2026-09-13 fix round to close four places where this document had said
"documented, but not read for this document" — a formulation that is unfinished
work rather than a finding, and is therefore no longer present anywhere below.

An extraction artefact worth knowing before anyone re-checks these quotes: in
the `pdftotext` output of **AL**, the heading of §3.5.4 renders as
`..  _ emory_rite-service` while the clause body extracts intact. The body is
what is quoted. Similarly, `docs/RESEARCH.md` §8.4 states of **AL** Table 1
that *"No APCI value can be read out of that safely"* — that is true of the
**Markdown** extraction it was assessing. `pdftotext -layout` on the source PDF
renders Table 1 row by row and legibly, which is why this document can state
APCI encodings at all. See RESEARCH §8.7.

A second artefact, in **PROF**: the mask-version columns of the feature tables
in §4.2 and §5.3 extract misaligned against their headers. Per-mask attribution
read out of those two tables is therefore **not** citable, and this document
does not cite any. Where a per-mask fact is stated below it comes from
PROF Annex A Table 7, whose columns do survive extraction, or from a footnote,
which has no columns to misalign. Row-level facts from §4.2/§5.3 (the set of
values a row contains, and its footnotes) are cited; column-level ones are not.

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
  commissioning, and is enforced at the **lowest layer that knows what an
  individual address is** — the guard sits on the type through which every
  address passes on its way to `knx-net`, not in the commissioning procedures
  and not in the UI, so that a bug in any higher layer cannot route around it.
  Every higher layer inherits the refusal instead of repeating it.
- **A test proves the refusal.** Phase 2 is not complete without a test that
  attempts every kind of access to `1.1.220` — a single read, a single write, a
  plan that contains it, a range that spans it, and a retry list that would
  re-add it — and asserts that each attempt is refused at the guard and
  reported. §14 item 11 is that test. The requirement is the test, not the
  intention: an exclusion with no failing-path test is an exclusion nobody has
  checked.
- A plan that would have contained an excluded address is **not** silently
  shortened. It reports the exclusion, per `CLAUDE.md`'s "never silently
  discard information". Silence here is indistinguishable from the guard not
  running.
- Broadcast operations are the sharp edge. `A_IndividualAddress_Write` is
  **sent by broadcast** and is accepted by whichever device is in programming
  mode (§4). There is therefore **no address filter that can protect `1.1.220`
  from a broadcast**. A broadcast commissioning operation may only be issued
  after the count of devices in programming mode has been established as exactly
  one (§4.2). This is exactly what MP §2.3 requires anyway; §2.1 is the reason it
  is non-negotiable rather than recommended.
- **And that precondition is structural too, by the same trick as the mutation
  API.** A precondition checked with an `if` is a precondition somebody deletes
  during a refactor. So the broadcast entry point does not take a device count
  and does not check one: it takes a **witness value** that is constructible only
  by the completion of §4.2's counting step, and only when that step completed
  with a result of exactly one. Zero responders and two-or-more responders both
  produce a report and no witness. There is consequently no argument the caller
  can synthesise that means "I checked, it was one" — the witness *is* the check,
  carrying the count it observed and the moment it observed it, and the broadcast
  path cannot be typed without one. This is the same construction §2.3 uses for
  the mutation API's authorisation value, applied to the one hazard an address
  filter cannot reach, and it is what §14 item 11's last clause tests. Doing it
  now costs one type; retrofitting it in phase 2 costs every call site.

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
and this specification neither reads nor writes it. It is named here so an
implementer does not mistake it for the LSM: `PID_LOAD_STATE_CONTROL` says
whether the data is valid, `PID_RUN_STATE_CONTROL` says whether the program is
running.

Out of scope is not the same as irrelevant. One clause about the RSM is load-
bearing for §9.3, and is cited there: **[D]** RES §4.24.2.3.4 *"The executable
part can only start if the Load State is Loaded."* That is what turns "this part
is not `Loaded`" into "this device's application is not running", which is the
severity behind §9.3 and behind R8. The clause is cited for its consequence; the
Run State Machine itself stays out of phase 2.

### 3.4 Realisation Types, and which ones are specified

- **LSM Realisation Type 1** — property-based, via `PID_LOAD_STATE_CONTROL`.
  This is what §5 specifies and what the implementation targets.
- **LSM Realisation Type 2** — **[D]** RES §4.23.3: *"This Realisation Type is
  not specified in this version of this document."* The implementation must not
  claim to support it. If a device turns out to need it, that is a finding for
  phase 3, not something to infer.
- **Programming Mode Realisation Type 2** — the memory-mapped form at address
  `60h`, bit 0, with a parity bit 7. **[D]** MP §3.13.2 NOTE: *"This means that
  the state of the Programming Mode is located at memory address 60h."*, and
  **[D]** RES §4.26.3.2: *"The location of curr_prog_mode shall be the memory
  address 0060h."* The layout is Figure 66, but the layout is also stated in the
  prose of RES §4.26.3.1, which is what §4.4 quotes — the figure is corroboration
  and not the source. This Realisation Type **is** specified, including the
  client-side parity operation, and §4.4 uses it read-only.
- **Mask-version-specific LSM access** — MP §3.31.2's `_RCo_Mem` variant is
  restricted to mask `070nh` and uses fixed memory addresses (load control at
  `0104h`, load states at `B6EAh`/`B6EBh`/`B6ECh`/`B6EDh`). Out of scope for
  phase 2; recorded so it is not reinvented.

Realisation Type 1 is **not universally mandatory**. **[D, corpus]** PROF §4.2
and §5.3 list, in the row *"Load State Machine / a. Realisation Type 1"*, both
`M` and `O` values across the mask versions in the table, and in §5.3's coupler
table the value for one column is `Ma`, whose footnote a is quoted in §5.4. The
per-column attribution is not citable from these tables (§1.1's second
extraction artefact), so what this document asserts is only the row-level fact:
there exist profiled masks for which Realisation Type 1 is optional, and
therefore **a device that does not expose `PID_LOAD_STATE_CONTROL` is not
necessarily broken**. Phase 2 reads the property, and on absence reports "this
device does not implement the LSM realisation this application supports" rather
than "this device is faulty". Which masks those are is a per-mask question this
document does not answer.

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

MP §3.13.2's *"has to be calculated"* is not the whole story, and an earlier
version of this document wrongly recorded the parity rule as undocumented. RES
§4.26.3.1 specifies the client-side operation completely, and it is a toggle
rather than a computation:

> **[D]** *"The variable p_parity shall be the parity bit of the complete octet.
> The don't care bits 1 to 6 may have some essential importance for the system
> stack and so the complete range bit 0 to bit 6 may be parity controlled. With
> this also the variable prog_mode shall be covered by the parity check of this
> systems. Even if the value of the don't care bits 1 to 6 is not evaluated, the
> state of the variable p_parity shall always be changed if the variable
> prog_mode is changed. This means that if the value of prog_mode is changed
> from "0" to "1" or from "1" to "0" then the variable p_parity shall be
> inverted."*

And RES §4.26.3.4.1, which states the client's obligation in one sentence:
**[D]** *"The Management Client shall set the Programming Mode via read – modify
– write of the variable curr_prog_mode. The variable prog_mode shall be set to
the value "1" if Programming Mode shall be activated. The variable prog_mode
shall be set to the value "0" if Programming Mode shall be deactivated. The
variable p_parity shall be inverted, if the value of prog_mode is changed."*

So the derivation is:

```text
old = A_Memory_Read(0060h, 1)               # one octet, curr_prog_mode
if bit0(old) == desired_mode: done          # RES §4.26.3.4.2/§4.26.3.4.3 guard
new = old XOR 0b1000_0001                   # invert bit 0 and invert bit 7
A_Memory_Write(0060h, 1, new)               # bits 1..6 carried through unchanged
```

Three properties of that derivation, each of which is the reason it is safe to
write down:

- **Bits 1–6 are preserved, not cleared.** **[D]** RES §4.26.3.1: *"Bit 1 to bit
  6 are the shared bits. They shall be don't care in context of Programming
  Mode. Even if the whole octet is read and written back they shall not be
  changed when activating and deactivating the Programming Mode."* They belong
  to whatever else the mask puts at `0060h`, so a read-modify-write that masks
  them off is a corruption, not a simplification.
- **Odd versus even never has to be decided.** The rule is *invert*, not
  *recompute*, and it is stated as a client obligation. Whether the device's
  stack checks for odd or for even parity is therefore a residual note rather
  than a blocker: starting from a valid octet — **[D]** RES §4.26.3.3 footnote
  95: *"It is assumed, that a proper running system has always a valid setting
  of the variable p_parity."* — a single inversion of bit 0 and bit 7 preserves
  whichever convention the device started in. The convention itself is still not
  stated anywhere this document searched, and does not need to be.
- **The write is skipped when the mode already matches.** RES §4.26.3.4.2 and
  §4.26.3.4.3 both wrap `DMP_ProgModeSwitch_RCo` in `if prog_mode = 0` /
  `if prog_mode = 1`. A no-op toggle would invert the parity bit against an
  unchanged `prog_mode`, which is precisely the invalid octet §4.26.3.3 warns
  about: **[D]** *"The reaction of an invalid parity value is manufacturer
  specific"*, footnote 96 *"Typically the system is restarted if p_parity is
  invalid."*

**None of that makes the write safe to perform, and the policy does not change.**
This is a **write to a live device at a fixed memory address**, addressed by
individual address rather than by button press, at an address that on some masks
shares an octet with system-stack state, where an octet the device considers
invalid may restart it. So:

- `DM_ProgMode_Switch`'s write half stays **inside the mutation API** (§2.3) and
  is therefore unreachable without an explicit authorisation value naming the
  concrete target. It is listed as a non-goal in §15 and stays there.
- It is **out of scope for phase 3** entirely. The read half
  (`A_Memory_Read(60h, 1)`) is the only part phase 3 may exercise, and only
  inside `1.1.24`–`1.1.32`.
- Phase 2 implements the toggle **against the simulator only**, so that the
  derivation above is exercised and the bit-preservation property is tested.

The distinction that matters for §12: "do not write `60h`" is now a **project
risk decision**, not a documentation gap. The Standard tells us how; this project
declines, because §13's R11 outcome is undocumented and the recovery is a site
visit. That is a defensible engineering choice and it is recorded as one — a
refusal we chose is a different object from a refusal the Standard forced on us,
and §12 no longer counts it among the gaps.

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
- **One profile removes one of the optional cells.** **[D, corpus]** PROF §5.3,
  footnote a to the *"Load State Machine / a. Realisation Type 1"* row:
  *"The Load State Machine transition table allows an optional transition from
  state “Loaded” to “Error” in case of an event “Load Completed”. This is not
  allowed for mask version 0912h Couplers. Mask 0912h shall stay in state
  “Loaded” in case of an error."* So in the `Load Completed` / `from Loaded`
  cell above, the `O: Error` alternative is **forbidden** for mask `0912h`
  couplers, which must report `Loaded`. The transition table of §11.2 therefore
  takes the mask version as an input where a cell has alternatives: the
  permitted-outcome set is narrowed per mask, never widened. This is the only
  narrowing of Table 94 that PROF states, and it is the only one this document
  applies — a profile that is silent about a cell leaves Table 94 as it is.

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
**before** unloading, because unloading clears it. Its values are §5.6.

### 5.6 `PID_ERROR_CODE`'s values: `DPT_ErrorClass_System` 20.011

`PID_ERROR_CODE` (PID 28) is `PDT_ENUM8` carrying `DPT_ErrorClass_System`,
DPT ID **20.011**. The enumeration below is **[D]** from **DPT**
(`03_07_02 Datapoint Types v02.02.01 AS`), clause 3 Datapoint Types, ID 20.011,
`Encoding: field1 = ErrorClass_System`, `Range: [0 to 18]`, `Use: FB`, with
footnote 16 *"This encoding is already used in FB Technical Alarm."* Values are
transcribed verbatim from the source PDF, including the ones whose line wrapping
is obvious:

| Value | Meaning, quoted |
| --- | --- |
| 0 | *"no fault"* |
| 1 | *"general device fault (e.g. RAM, EEPROM, UI, watchdog, …)"* |
| 2 | *"communication fault"* |
| 3 | *"configuration fault"* |
| 4 | *"hardware fault"* |
| 5 | *"software fault"* |
| 6 | *"insufficient non volatile memory"* |
| 7 | *"insufficient volatile memory"* |
| 8 | *"memory allocation command with size 0 received"* |
| 9 | *"CRC-error"* |
| 10 | *"watchdog reset detected"* |
| 11 | *"invalid opcode detected"* |
| 12 | *"general protection fault"* |
| 13 | *"maximal table length exceeded"* |
| 14 | *"undefined load command received"* |
| 15 | *"Group Address Table is not sorted"* |
| 16 | *"invalid connection number (TSAP)"* |
| 17 | *"invalid Group Object number (ASAP)"* |
| 18 | *"Group Object Type exceeds (PID_MAX_APDU_LENGTH – 2)"* |
| 19–255 | *"reserved, shall not be used"* |

The encoding is `N8` — a one-octet enumeration, one field. Two consequences for
this specification and one for somebody else:

- **Values 6, 8, 9, 13 and 14 are download diagnostics**, not device faults.
  They tell the client that *its own* request was wrong: too much data for the
  device's memory (6), an allocation with size 0 (8), a checksum mismatch over
  loaded data (9), a table longer than the device allows (13), a load command
  the device does not know (14). §9.2's "state is `Error` after an event write"
  row is resolved by reading this property: 8/13/14 mean the client's request
  was malformed, 6 means the device is too small, 9 means the data arrived
  corrupted. The reporting text a user sees should distinguish them, because
  only 6 is the user's problem.
- **Value 15 is a validity rule the download must obey**, and its companion
  requirement is in **RES**: *"The Group Addresses shall be sorted in ascending
  order with increasing memory locations."* (RES §4.16.3, Group Address Table
  Realisation Type 1, and again for the later Realisation Type). A client that
  writes an unsorted Group Address Table produces a device that reports error 15
  at runtime. Phase 2 sorts the table before writing it and asserts sortedness
  in a test, rather than discovering this from a device.
- **Reusable elsewhere.** The task implementing DPT main type 20 in
  `crates/knx-core/src/dpt/codec.rs` can take this table as the 20.011 arm
  without re-deriving it. Range is `[0 to 18]`; 19–255 must be rejected or
  surfaced as unknown rather than mapped to a name. For contrast, the adjacent
  `20.012 DPT_ErrorClass_HVAC` has `Range: [0 to 4]` with
  *"5 to 255 : reserved, shall not be used"* — the reserved boundary differs per
  sub-type, so it cannot be shared across the main type.

## 6. Memory read and write over the bus

### 6.1 `A_Memory_Read`

**[D]** AL §3.5.3: the service is used *"to read between 1 and 63 octets"* and
*"memory_address shall specify the 16 bit start address"*.

Error handling, quoted from the same clause, because the read rule and the write
rule of §6.2 are **not the same rule** and an earlier version of this document
had the write rule here:

> **[D]** *"If data are to be read from a protected area or from any logical
> address that is not associated to physical memory then in the
> A_Memory_Response-PDU the field number shall be zero and there shall be no
> field data to indicate an error. The same shall apply if only part of the
> memory to be read is protected or physically existing. In addition, the same
> shall apply if the value of the parameter number is greater than Maximum APDU
> Length – 3."*

A failing read therefore **answers**. It answers with `number = 0` and no data
field, and it answers that way for all four causes: protected, unmapped,
partially protected, and over-length. That has a direct consequence for the code:
an over-long `A_Memory_Read` must be handled on the **response** path as a
parsable negative answer, not on the timeout path. Coding a timeout here — which
is what the write rule would suggest — turns a 30-millisecond refusal into a
several-second stall, and worse, reports "device not responding" about a device
that responded correctly.

The service's own footnote qualifies how much any of this can be relied on:
**[D]** AL §3.5.3 footnote 16: *"Existing devices may have a different error
handling."* So `number = 0` is the specified answer, a timeout is a possible real
one, and the implementation treats both as failure while distinguishing them in
the report.

16-bit address means this service alone reaches 64 kB. §7.2 covers what happens
above that.

### 6.2 `A_Memory_Write`

**[D]** AL §3.5.4: the service is used *"to write between 1 octet and 63
octets"*. Its confirmation behaviour depends on Verify Mode: *"The service
shall be a confirmed service if Verify Mode is active, otherwise it shall be an
acknowledged service."*

#### The read-back sentence, scoped correctly

An earlier version of this document quoted AL §3.5.4's read-back sentence as
applying *"independently of Verify Mode"* and as a requirement on the client. It
is neither. The sentence sits inside the active-Verify-Mode paragraph and
constrains the **device**. Quoted with its surrounding sentences, which is the
only way the scope is visible:

> **[D]** *"With inactive Verify Mode the remote application process shall not
> respond. …*
>
> *With active Verify Mode the remote application process shall respond to the
> A_Memory_Write.ind primitive with an A_Memory_Write.res primitive containing
> the requested number of octets of the associated memory area. **The value of
> the associated memory area shall be explicitly read back after writing to
> it.** If the remote application process has a problem, e.g., memory area
> unreachable or protected or an illegal number of octets are requested, then
> the parameter number shall be zero and shall contain no data."*

So what the Standard requires is: **with Verify Mode active, the device must read
its own memory back and return what it read** — which is why `A_Memory_Write.res`
carries *"data: the octet(s) read back or no data"* rather than an echo of what
was sent. With Verify Mode inactive the device does not respond at all, and
nothing in AL §3.5.4 obliges anybody to read anything back.

**[A] Project requirement, and it is this project's and not the Standard's.**
Phase 2 implements **no write path without a client-side verification read**:

- with Verify Mode active, the `A_Memory_Write.res` payload is compared against
  the written octets, and a mismatch is an error (which is also what MP §3.16.3's
  sequence does: *"different or no data received ⇒ error"*);
- with Verify Mode inactive, an explicit `A_Memory_Read` of the same region
  follows the write and is compared — this is MP §3.16.2's own shape, whose
  sequence issues `A_Memory_Read` when `verify = enabled` and otherwise only
  waits out a *"delay for programming the memory in the device"* (§12's remaining
  gap).

The conclusion the earlier text reached is right and R6 stands. Its authority is
different: this is a design rule chosen because R5 and R6 are the worst outcomes
in §13 and because a device that silently drops a write is indistinguishable from
one that took it. It is **not** a Standard obligation on the client, and this
document may not claim it is.

#### Error handling

Quoted from the same clause, because each case needs different handling in code —
and note that every one of these is *ignore*, which is the opposite of §6.1's
read rule:

- Target protected or nonexistent: *"the service indication shall be ignored"* —
  so a write to a protected region looks exactly like a lost frame.
- Partially protected target: *"If only a part of the addressed memory is
  protected or does not exist, then the complete write operation shall fail."* —
  no partial writes; there is no "how far did it get" to recover from.
- **[D]** *"the remote Application Layer shall ignore the A_Memory_Write.ind if
  the value of the parameter "number" is greater than Maximum APDU Length – 3"*,
  and likewise *"if the parameter number is not equal to the number of received
  data octets."*
- A failed write under Verify Mode is reported as `number = 0` with no data:
  **[D]** *"If Verify Mode is active, then in case of a failed write operation
  the field number of the A_Memory_Response-PDU shall be zero and there shall be
  no field data to indicate an error."* With Verify Mode inactive there is no
  response at all, so there is no failure indication either — which is the whole
  argument for §6.3.
- Same qualifier as the read: **[D]** AL §3.5.4 footnote 21: *"Existing devices
  may have a different error-handling."*

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

There is a third case, which is a device that does not have the feature at all.
**[D, corpus]** PROF, footnote 8 to §4.2's *"Verify Mode"* row and footnote 16 to
§5.3's: *"If Verify Mode is not implemented, it shall always be off."* So a
device may legitimately answer a read of `PID_DEVICE_CONTROL` with bit 2 clear
after the client has just set it, and that is conformant behaviour rather than a
write failure. The read-back therefore decides which write path runs — bit 2
set means §6.2's confirmed service, bit 2 clear after an attempt to set it means
the client must run MP §3.16's explicit `DMP_MemWrite_RCo` verify loop instead.
Failing to set Verify Mode is **not** an error to abort on; failing to notice
that it is off is.

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

**How the client establishes it — this is documented, and an earlier draft of
this document wrongly listed it as a gap.** **[D]** RES §4.3.7
`PID_MAX_APDU_LENGTH` (PID 56, `PDT_UNSIGNED_INT`, Datapoint Type: none):
*"The Management Server shall hold in PID_MAX_APDU_LENGTH the maximal supported
APDU-length for Management of the device."*, *"The value of
PID_MAX_APDU_LENGTH may be in the range between 15 and 254"* with footnote 4
*"255 as value for PID_MAX_APDU_LENGTH is an ESCape Code"*. The discovery rule
is §4.3.7.2.1, quoted in full because the whole design rests on it:
*"This Property shall allow a Property based Management Client to use the
maximal APDU-Length to manage the device. A Management Client supporting the
L_Data_Extended-frame has to check this value before starting download. If the
PID_MAX_APDU_LENGTH is not present in the Device Object, then the Management
Client shall manage the device with L_Data_Standard-frames with an APDU-length
of maximal 15 octets."*

That closes the loop arithmetically. Absent property ⇒ 15-octet APDU ⇒ AL
§3.5.4's "ignore if `number > Maximum APDU Length – 3`" ⇒ **12 octets of data**,
which is exactly MP §3.16's number. The 12 is not a folklore constant; it is
15 − 3.

The resulting algorithm, which phase 2 implements once:

```text
read PID_MAX_APDU_LENGTH from the Device Object (PID 56)
  absent          → max write = 12 octets, L_Data_Standard frames only
  present, value v→ max write = min(v, 254) - 3, capped at 63 by AL §3.5.3/§3.5.4
  value 255       → ESCape Code, not a length; treat as "do not use", i.e. 12
never                exceed the value read; never raise it on a successful write
```

Two adjacent rules from the same clause, because they bite on couplers:
**[D]** RES §4.3.7.2.2: *"If this Property is not present in the Device Object,
then the device shall be managed with L_Data_Standard frames."* and
*"If PID_MAX_APDU_LENGTH is solely in the Router Object (see 4.5.9), then the
device shall only support L_Data_Extended-frames for Routing and only
L_Data_Standard-frames for Management."* So the property must be read from the
**Device Object**; a value found in the Router Object says the opposite of what
a careless read would conclude.

What is **not** the mechanism: `PID_EXT_FRAMEFORMAT`. **[D]** RES §4.16.7.2.5
(PID 51): *"This Property is reserved for LTE-Mode devices."* It scopes an
Address Table to an Extended Frame Format value; it says nothing about the
management APDU length. An implementation that reads that property to decide
write sizes has read the wrong one.

Even so, phase 2's **default** stays 12 and is raised only from a value actually
read from the Device Object of the device in front of it, never from a product
database, a mask version or an assumption.

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

#### Which subtypes a device is required to have (PROF Annex A Table 7)

Which of those subtypes a given device supports is **profiled**, not universal.
**[D, corpus]** PROF Annex A.2.4.1, *"Table 7 – Required Load Controls"*, is
indexed by Load Sub-Control type down the side and by mask version across the
top, with `M` mandatory, `O` optional, `n/a` not applicable. This is the one
per-mask table in PROF whose columns survive extraction, so it is citable.
The rows relevant here, with the mask groupings the table's own column bands
give (`System 2`, `System 300`, `System B`):

| Sub-control | Requirement, as Table 7 states it |
| --- | --- |
| `03h` / `00h` Absolute Code/Data Allocation | `M` across the System 2 and System 300 columns and for masks `0701h`/`0705h`; `O` for `0700h`, `07B0h`, `17B0h`, `2010h`, `2110h`; **`n/a` for `57B0h`** |
| `03h` / `01h` Absolute Stack Allocation | `M` only in the System 300 band (`0300h`, `2300h`), with footnote 66; `n/a` for `57B0h`; every other column blank |
| `03h` / `02h` Segment Control Record, `03h` Task Pointer Record, `04h` Task Control Record-1, `05h` Task Control Record-2 | same row pattern as `00h`, including **`n/a` for `57B0h`** |
| `03h` / `0Ah` Relative Allocation | `M` for mask `0300h` |
| `03h` / `0Bh` Data Relative Allocation | `M` for masks `07B0h`, `17B0h` and `57B0h` |
| `01h` Start Loading, `02h` Load Completed, `04h` Unload | listed as `M` for mask `57B0h`; the remaining columns extract as `?` |

One column is not attributed above: Table 7 has fourteen data columns while
thirteen mask versions extract as headings, so the leftmost column's mask cannot
be named from the extraction. Nothing in this document depends on it, and
nothing below claims a value for it.

Footnote 66 to the absolute-allocation rows: *"The Additional Load Controls are
only required if Additional Data shall be downloaded."* And the NOTE under the
table, which decides how the implementation must use it: *"Table 7 specifies
globally for the device which Load Controls shall be supported. Which Load
Controls shall be supported for the management of a specific Resource is
specified in the (Realisation Type of) the Resource in [11]."* — `[11]` being
**RES**. So Table 7 answers "can this device do it at all", and RES answers
"is it the right one for this loadable part". Both have to agree before a load
control is emitted.

Three design rules follow:

1. **The allocation subtype is chosen per mask version, from the device's
   Device Descriptor Type 0, not per project.** §7.2's use of `0Bh` is correct
   for the System B masks this document's CP §3.5.2 walkthrough covers
   (`07B0h`/`17B0h`/`57B0h`), and is *not* transferable to `0300h`, which
   requires `0Ah`, or to the System 2 masks, which require absolute allocation.
2. **`57B0h` is the sharpest case**: absolute allocation and the four record
   subtypes are `n/a`, while `0Bh`, Start Loading, Load Completed and Unload are
   `M`. A client that falls back to absolute allocation when relative allocation
   fails would be sending a device something its profile says is not applicable.
   There is no fallback between allocation styles. Pick by mask, or refuse.
3. **Where Table 7's cell extracts as `?`, this document states nothing.** The
   `?` cells are an extraction artefact of a very wide table, not a documented
   "unknown". Phase 2 re-reads Table 7 from the PDF for the specific mask it
   intends to support, per mask, at the time it supports it. That is a two-minute
   lookup against a device in hand and a guess that cannot be checked otherwise.

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

### 8.2 What a reset does to the resources this document uses (AN194)

**AN194** `AN194 v02 Master Reset of Resources AS` is the application note that
states, per Interface Object and per PID, what a Master Reset does. Its scope,
quoted from §1: *"That document contains some initial requirements on the
contained Resources for Master Reset. These are reviewed in this Application
Note, and extended to all Resources, so that – if there are any requirements –
these are clear and the MaC (ETS) can rely on a certain state after a Master
Reset."* Its tables have **six** columns, of which only **three** are Erase
Codes: `-` Local Reset to default state (no Erase Code), `02h` Reset to default
state, `07h` Reset to default state without IA, `01h` Confirmed Restart, and then
two columns headed `none` — Basic Restart and Power Cycle. Any statement below of
the form "in all six columns" therefore covers three Erase Codes plus a local
reset, a Basic Restart and a Power Cycle, and must not be paraphrased as "by all
six Erase Codes"; there are only three of those in the table.

The vocabulary has to be read before the tables, because "default" means four
different things (AN194 §2.3.2.1, quoted):

| Action | For the Management Client |
| --- | --- |
| *"Not influenced"* | *"The MaC may rely on the fact that the value will not change because of the Master Reset, but may change because of other reasons."* |
| *"recalculate"* | *"The MaC shall read the value before using it."* |
| *"KNX default"* | *"The MaC may assume that the PID has the default value."* — the value itself being *"some default value that is not specified in the KNX Specifications"* |
| *"implementation default"* | *"If the value is given in the product description then the MaC may rely on this; otherwise, the MaC has to read the value."* |
| *"not applicable"* | no action is specified; the typical setting for Function Properties |

And a runtime marker: *"Constant"* means the value never changes for any Erase
Code, *"Runtime"* means *"The MaC has to read out the value from the MaS if it
needs it."*

The rows that matter to this specification:

| PID | `-` / `02h` / `07h` | `01h` Confirmed Restart, Basic Restart, Power Cycle |
| --- | --- | --- |
| 5 `PID_LOAD_STATE_CONTROL` — §2.3.2.3, Object Type 1 Addresstable | *"implementation default"* | *"not influenced"* in all three |
| 28 `PID_ERROR_CODE` — §2.3.2.3 | *"implementation default"* | *"not influenced"* in all three |
| 7 `PID_TABLE_REFERENCE` — §2.3.2.3, Addresstable **only** | *"recalculate"* | *"not influenced"* in all three |
| 7 `PID_TABLE_REFERENCE` — §2.3.2.10, Object Type 9 Group Object Table | *"recalculate"* | *"not influenced"* in all three |
| 7 `PID_TABLE_REFERENCE` — §2.3.2.4 Associationtable, §2.3.2.5 Applicationprogram, §2.3.2.6 Application Program 2 | *"recalculate"* | *"recalculate"* in all three |
| 6 `PID_RUN_STATE_CONTROL` — §2.3.2.5 and §2.3.2.6 | *"recalculate"* | *"recalculate"* in all three |
| 14 `PID_DEVICE_CONTROL` — §2.3.2.2, Object Type 0 Device Object | *"KNX default"* | *"KNX default"* in all three |
| 54 `PID_PROG_MODE` — §2.3.2.2 | *"KNX default"* | *"KNX default"* in all three |
| 56 `PID_MAX_APDU_LENGTH` (CONSTANT value) — §2.3.2.2 | *"not influenced"* | *"not influenced"* in all three |
| 30 `PID_DOWNLOAD_COUNTER` — §2.3.2.2 and §2.3.2.3 | *"recalculate"* | *"not influenced"* in all three |
| 29 `PID_OBJECT_INDEX` (CONSTANT value) — §2.3.2.2 and §2.3.2.3 | *"not influenced"* | *"not influenced"* in all three |

**The per-object rows do not all agree, and one of them was generalised in
error.** An earlier version of this document transcribed `PID_TABLE_REFERENCE` as
*"not influenced"* in the three restart columns and drew a general rule from it.
That is the **Addresstable Object's** row. Five objects carry PID 7, and they
split two to three:

- §2.3.2.3 Addresstable and §2.3.2.10 Group Object Table: `recalculate`
  `recalculate` `recalculate` / `not influenced` `not influenced`
  `not influenced` — a cached base address survives a restart.
- §2.3.2.4 Associationtable, §2.3.2.5 Applicationprogram and §2.3.2.6
  Application Program 2: `recalculate` in **all six columns** — a cached base
  address does not survive anything, restart included.

`PID_RUN_STATE_CONTROL` (PID 6) is `recalculate` in all six columns in the two
Application Program objects as well, which is consistent with the RSM depending
on the LSM (§9.3).

Cross-object agreement therefore has to be checked per PID and not assumed. What
the five objects read for this document **do** agree on:

- `PID_LOAD_STATE_CONTROL` (PID 5): *"implementation default"* for `-`/`02h`/`07h`
  and *"not influenced"* for Confirmed Restart, Basic Restart and Power Cycle, in
  §2.3.2.3, §2.3.2.4, §2.3.2.5, §2.3.2.6 and §2.3.2.10 alike. Design rule 1 below
  therefore holds across every object this specification touches, which is the
  claim that matters most.
- `PID_ERROR_CODE` (PID 28): the same pattern in the same five objects.
- `PID_MCB_TABLE` (PID 27): the same pattern in all five.

**What was still not read.** AN194 tabulates fourteen Interface Object types
(§2.3.2.2 through §2.3.2.14). This document read five of them — Device Object,
Addresstable, Associationtable, Applicationprogram, Application Program 2 and
Group Object Table — chosen because they are the objects the download procedures
of §7 touch. The Router, LTE Address Routing Table, cEMI Server, KNXnet/IP
Parameter, Security Interface and RF Medium Objects were not read. No claim in
this document rests on them, and no claim in this document generalises across
objects any more: where phase 2 needs a reset semantic for a PID in an object not
listed above, it reads that object's own AN194 row, because the
`PID_TABLE_REFERENCE` split above is the evidence that "the other rows probably
match" is not true.

Read as design rules:

1. **Independent confirmation of §9's central fact.** The load state and the
   error code are *"not influenced"* by a Basic Restart, a Confirmed Restart or
   a Power Cycle. RES §4.23.1 says the state is non-volatile; AN194 says no kind
   of restart clears it. An interrupted download is not fixed by turning the
   device off and on again, from two independent sources.
2. **A destructive Erase Code does reset them** — to an *"implementation
   default"*, which per the vocabulary above the client **must read** unless the
   product description states it. So §9.1's recovery cannot be replaced by
   "factory reset and start over" and then assume the parts are `Unloaded`: the
   post-reset load state has to be read, per part, exactly as §9.1 step 2 does.
3. **`PID_DEVICE_CONTROL` is `KNX default` in all six columns**, so Verify Mode
   does not survive a power cycle either. §6.3 already required re-asserting it
   per connection; this extends the same rule across every restart.
4. **`PID_PROG_MODE` is `KNX default` in all six columns** — programming mode is
   off after any restart. §4.3's "programming mode may switch itself off
   underneath you" has a second cause: anything that restarts the device.
5. **`PID_TABLE_REFERENCE` must be re-read after any restart, not merely after a
   reset.** In the Addresstable and Group Object Table it is `recalculate` for
   the three Erase Codes and `not influenced` for the three restarts; in the
   Associationtable and both Application Program objects it is `recalculate` in
   **all six** columns. The rule that holds for every object is therefore the
   stricter one: **re-read after any restart.** A base address cached across a
   reset, a restart or a power cycle is invalid for at least three of the five
   objects this specification touches, and the client does not get to know which
   object it is dealing with cheaply enough for the distinction to be worth
   keeping. §7.2's read-back of `PID_REFERENCE` is mandatory per download, and
   caching it across any restart is forbidden.
6. **`PID_MAX_APDU_LENGTH` is a CONSTANT value, *"not influenced"* by anything**
   — so §6.4's negotiated write length may be cached for the lifetime of the
   device, which is the one thing in this list that may.
7. **`PID_DOWNLOAD_COUNTER` is `recalculate`**, which is to say a reset moves it.
   It is therefore not a reliable "has this device been touched" fingerprint
   across a reset, only within one.

Two limits AN194 sets on what any client may assume:

- **Timing.** *"Basic Restart … as the service parameter process time is not
  available – the MaC cannot rely on any timing when the parameters become
  effective."* versus *"Confirmed Restart … The MaC can rely on the fact that
  these changed parameters are taken at least after the process time, responded
  by the server in the “Confirmed Restart Response”."* This is the second
  independent argument for §8.1 design rule 1: use `01h`, because it is the only
  restart whose completion time the client is allowed to rely on. The quotation
  is from AN194 §2.2.1, whose subject is KNX IP and KNXnet/IP parameters; the
  general point — that only the Confirmed Restart carries a process time — is
  AL §3.4.2.2's, cited in §8 above.
- **"Ex-factory" does not mean "as delivered".** **[D, corpus]** AN194 §2.3.2.1:
  *"Master Reset is the functionality of a MaS to set itself, one or more parts
  of its configuration back to an internal kept state, without these being set
  by the MaC. These internal states are default values that may differ from the
  state of the MaS as delivered. The below defined Erase Codes “ex-factory” shall
  thus be understood as “set back a default state” rather than “set back to the
  delivery state”."* So §8.1's design rule 3 is stronger than it looked: the
  application may not describe Erase Code `02h` to a user as "returns the device
  to how it left the factory". It returns it to an unspecified internal default.
- **A reset device may be running an application.** *"There are no requirements
  on whether or not an application may be running in the MaS after a Master
  Reset: it is possible that the MaS restores a default application. This has
  consequences for the requirements of certain Resources, like Address Table,
  Load State Machine and other."* A device that reports `Loaded` after a factory
  reset is not necessarily lying, and must not be treated as such. This is the
  clearest single reason the application must never infer device state from an
  operation it performed (§11.2's intent-versus-fact split).
- **"Power Cycle" is not a defined operation.** *"The interpretation of this term
  is implementation specific. The implementation defines what a power cycle
  means for a given implementation of a Resource."* Nothing in phase 2 or phase 3
  may depend on it, and a test plan that says "power-cycle the device" has not
  specified a step.

AN194 also carries one flat requirement with no relation to reset semantics:
**[D, corpus]** §2.3.1, on `PID_PL110_PARAM` (PID 73): *"The default value of
this Property shall be FFh."* Noted because it is in the same document, not
because anything here uses it.

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
| State unchanged after an event write | Illegal or unknown event, **or** genuinely no-op (RES §4.23.2.3.2), **or** an access level too low to write the property while high enough to read it (§10.8), **or** the device has no loadable application at all, in which case the property is conformantly read-only and stuck at `Loaded` — **[D]** RES §4.23.2.3.3: *"For devices with a non-loadable application the Property Load Control shall be read-only and shall have the fix value Loaded."* | Assume the event was accepted. The read-only case is not an error at all and must not be reported as one: a device reading `Loaded` that refuses `Start Loading` may be a device with nothing to load |
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
download.

The Standard says what that costs, in the one clause that connects the load state
to whether the device does anything. §3.3 keeps the Run State Machine out of
scope, and it stays out of scope, but its dependency clause is the substantiation
for the claim above: **[D]** RES §4.24.2.3.4 *"Dependencies between Load - and
Run State Machine"*: *"One of the run conditions of the Run State Machine is the
state of the Load State Machine. The executable part can only start if the Load
State is Loaded. For an executable part that cannot be loaded the Run State may
be read-only and may always be in the state Running."*

So "the part is invalid" is not a bookkeeping state. An application part whose LSM
is anything other than `Loaded` **cannot run**, by specification, and the window
below is a window in which the device's application is stopped — not merely a
window in which a table is stale. That is why this section is about transaction
semantics and not about error reporting. (The document does not track or write the
Run State; it cites this clause only to establish the consequence.)

The implementation must therefore:

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

CP §3.5.2 step 03 is "Get access rights — Authorize"; step 11 is "Modifying
access keys — Set access keys as required". This section specifies both, in
enough detail to implement, because phase 2 cannot begin the download sequence
without step 03.

### 10.1 The key and level model

**[D]** AL §3.5.7, `A_Authorize_Request-service`, quoted at length because every
design decision below is in it:

> *"The A_Authorize_Request.req primitive shall be applied by the user of
> Application Layer, to inform the communication partner about the key that
> shall be four octets long and of data type unsigned32. The remote partner
> shall know a number of valid keys and shall be able to associate a valid key
> to an access level. This access level shall be stored as the current access
> level of this partner and shall be sent back in an A_Authorize_Response-PDU.
> Access levels (unsigned8) between 0 (maximum level, i.e., maximum access
> rights) and 3 (minimum level, i.e. minimum access rights) or 0 (maximum level,
> i.e. maximum access rights) and 15 (minimum level, i.e. minimum access rights)
> are allowed."*

So:

- **A key is one 4-octet unsigned32.** Not a string, not a passphrase, not
  derived. `FFFFFFFFh` has a special meaning (§10.2).
- **A level is one octet, and lower is more powerful.** Level 0 is maximum
  rights. An implementation that treats a larger number as "more access" has it
  backwards, and the mistake is silent because both ends still parse.
- **There are two level ranges, 0–3 and 0–15**, and which one applies is a
  device property, not a client choice. **[D, corpus]** PROF §4.2 row
  *"Authorization … nr of access levels"* contains the values `4`, `16` and `O`,
  and PROF §5.3's coupler table contains `4` and `n/a`. Per-column attribution
  is not citable here (§1.1), so the row-level fact is what this document
  asserts: profiled devices exist with 4 levels and with 16. The client must not
  assume 4. It learns the count from the Profile for the device's mask version,
  or treats the returned level as opaque and compares it numerically — which is
  all any of the procedures below actually require.
- **Purpose.** *"The current access level may be used by the remote application
  process to decide whether or not a communication partner is allowed to request
  a certain read or write operation."* — "may", which means a device with no
  protected areas is free to ignore the whole mechanism (§10.5).

### 10.2 What happens when the client does not authorise, or authorises wrongly

The two cases are specified, and neither is a refusal. **[D]** AL §3.5.7,
*"Error and exception handling"*, both bullets quoted:

> *"If the Remote Management supports authorization and if the communication
> partner does not authorize itself, the Remote Management shall select the
> maximum access level protected with FFFFFFFFh as the current access level."*
>
> *"If the Remote Management supports authorization and if the communication
> partner authorizes itself with an invalid key, the Remote Management shall
> select the minimal access level (this is level 3 or level 15) as the current
> access level."*

Read carefully, because it is counter-intuitive in a useful way:

- **Not authorising at all is not an error.** The device grants whatever level is
  associated with the key `FFFFFFFFh` — the "free" level. On many devices that
  is enough to read, and on devices with no protected areas it is level 0
  (§10.5). A client that skips authorisation therefore works until it does not,
  and the failure appears later as an unexplained ignored write.
- **A wrong or unknown key is *worse than not trying*.** It drops the partner to
  the **minimum** level, 3 or 15. There is no negative response, no error code,
  and no way to distinguish "wrong key" from "right key, low level" except by
  comparing the returned level against the free level. This is the reason
  MP §3.5.2 exists (§10.4).
- **Therefore: never send a key the user has not supplied.** Trying a guessed
  key actively reduces the access the client would have had by doing nothing.

### 10.3 Per connection, not per operation

**[D]** AL §3.5.7: *"A current access level shall be valid until the connection
is released or a new key is indicated with the A_Authorize_Request service."*

That is the lifetime, exactly: the Transport Layer connection. It follows that

- authorisation is a **step of the session**, performed once after `DM_Connect`,
  not a parameter of each memory write;
- and every reconnect re-authorises. §5.5's wait loop reconnects during long
  load transitions, and §6.3 already requires re-asserting Verify Mode on each
  reconnect for the same structural reason. **Authorisation joins it: the
  session object owns the key, and `connect()` performs authorise-then-set-
  Verify-Mode as one unit.** A reconnect that forgets to re-authorise silently
  drops to the free level, and per §10.5 the resulting write failures are
  invisible.

**[D]** MP §3.5, `DM_Authorize`, the procedure that wraps it:

> *"This device Management Procedure shall be used to obtain access
> authorization. The authorization shall be executed only when it is required by
> the Management Server. Whether or not a Management Server supports
> authorisation can directly be retrieved from the Device Descriptor Type 0
> (mask version). In [14] it is specified for which Profiles authorisation is
> mandatory. DM_Connect shall be executed before executing this Management
> Procedure."*

Its parameters, verbatim from the same clause: `DM_Authorize (flags, keys)`
where *"flags — All bits are reserved. These shall be set to 0. This shall be
tested by the Management Client."* and *"key — key for authorization"*.

And the sequence, MP §3.5.1 `DMP_Authorize_RCo`, which uses the
connection-oriented communication mode:

```text
if authorization is required (key != FFFF FFFFH)
    MaC → MaS : A_Authorize_Request-PDU (key)
    MaS → MaC : A_Authorize_Response-PDU (key, level)
endif
                       A_Disconnect.ind  ⇒  "error: connection was broken down"
```

Two things to take from that sequence. The guard *"if authorization is required
(key != FFFF FFFFH)"* makes `FFFFFFFFh` the client-side sentinel for "no key
configured" — the procedure is skipped rather than run with a placeholder. And
`[14]` being **PROF**, the "is authorisation required" question is answered from
the device's mask version via its Profile, which §10.5 covers; it is not
answered by trying.

### 10.4 `DM_Authorize2_RCo`, and the profile scoping this document must respect

**[D]** MP §3.5.2. An earlier version of this section called this *"the procedure
to actually implement"* and quoted its Conditions block while omitting the line
immediately above that block. Quoted in full, in the order the clause prints it:

> **[D]** *"Use*
> *• Profiles — System 2, BIM M112*
> *• Conditions — Write access, i.e. modifying memory- or Property contents; A
> key must be available"*

`DM_Authorize2_RCo` is **profile-scoped to System 2 and BIM M112**. The download
this document specifies is CP §3.5.2, **System B**. Those are not the same
profile, and this document may not quietly treat a System 2 / BIM M112 procedure
as the System B default.

**Ruling, and the reason.** `DM_Authorize_RCo` (MP §3.5.1) is the **default**.
It carries no `Use • Profiles` restriction at all — MP §3.5's `Use` block is prose
about when authorisation applies, and §3.5.1 adds only *"This Management Procedure
shall use the connection oriented communication mode."* — so it is the procedure
whose scope demonstrably covers System B. `DM_Authorize2_RCo`'s two-key
comparison is kept as a **defensive extension**, enabled deliberately and never
by default.

Three reasons for that direction rather than the other:

1. **It is the only choice that needs no unsourced claim.** Justifying
   `DM_Authorize2_RCo` on System B would require asserting that a procedure
   scoped to two other profiles applies to a third. Nothing in MP, CP or PROF
   says it does, and §1's rule is that a claim without a source is removed rather
   than softened. Making the unscoped procedure the default costs nothing and
   asserts nothing.
2. **The algorithm is not what protects the client; §10.2 is.** The failure this
   whole section exists to prevent is sending a key that was not supplied by the
   user, because a wrong key yields *less* access than no key (§10.2, R17).
   `DM_Authorize_RCo`'s `key != FFFFFFFFh` guard already refuses to run without a
   real key. The extra exchange in `DM_Authorize2_RCo` buys recovery from one
   specific situation — a key configured in the project, a device that is not
   actually locked with it — which is a convenience, not a safety property.
3. **The extension is cheap and reversible; the claim is not.** A session that
   implements the unscoped procedure can add the second and third exchanges later
   behind a flag. A document that has claimed cross-profile applicability has to
   be corrected, and by then a later task will have copied the claim.

So: phase 2's session runs MP §3.5.1 on connect. The two-key comparison below is
implemented and tested against the simulator, reachable by explicit opt-in, and
documented as **[A]** *"an extension this project chose, scoped by its own source
clause to System 2 and BIM M112 and therefore not run against a System B device
by default"*. If phase 3 meets a device where the free level beats the configured
key's level, that is the finding that would justify turning it on — and it is a
read-only observation, so phase 3 can make it.

The rest of this subsection describes the extension. It is quoted and specified
because the asymmetry it handles is real; the scoping above is what decides when
it runs.

The asymmetry, in the clause's own words:

> *"Opposite to the Management Procedure DM_Authorize_RCo, this Management
> Procedure DM_Authorize2_RCo does not presume that the device has been locked
> with the key that is provided to the procedure. Therefore, it authorizes
> subsequently with the key FFFFFFFFh and with the key client_key and continues
> with the key that gives the maximal access rights."*
>
> NOTE: *"This is the case when the ETS User enters a key to be used to lock the
> devices and uninitialised devices fresh from the factory are used."*

Preconditions, quoted: *"It shall assume that the Management Client has an
access key provided by its user. If the Management Client does not have an access
key, it shall not be executed."* and *"DM_Connect shall be executed before
executing this Management Procedure."* — the `Use • Profiles` and
`Use • Conditions` lines that follow them in the clause are quoted at the top of
this subsection, because the profile line is the one that decides whether this
procedure runs at all.

The algorithm, transcribed from the clause's sequence diagram:

```text
# The MaS has already granted the level associated with FFFFFFFFh, but the
# client does not know which level that is. So it asks.
MaC → MaS : A_Authorize_Request (key = FFFFFFFFh)
MaS → MaC : A_Authorize_Response(level = free_level)

if free_level is not the highest level (lowest numerical value):
    MaC → MaS : A_Authorize_Request (key = client_key)
    MaS → MaC : A_Authorize_Response(level = client_level)

    if client_level > free_level:          # free access was better
        MaC → MaS : A_Authorize_Request (key = FFFFFFFFh)
        MaS → MaC : A_Authorize_Response(level = FFFFFFFFh)
```

The clause's own gloss on the comparison: *"The level obtained now with
access_key is compared to the one obtained for free access. If free access gave
higher access level (lower numerical value), a new authorisation with free access
level is done."*

Three notes for the implementer. First, the `>` in `client_level > free_level` is
numeric and therefore means *worse*; §10.1's inversion is load-bearing here and
is the single most likely place to introduce a bug that only shows up on a locked
device. Second, the last response's level field is printed in the source as
`level = FFFFFFFFh`, which is a four-octet value in a one-octet field — an
evident defect in the diagram, not a protocol rule; the value that matters is
that the free level has been re-selected, and phase 2 must read back whatever
level the device actually returns rather than expect a constant. Third, error
handling is quoted and total: *"Failure of any of the contained Application Layer
Services shall lead to failure of the entire Configuration Procedure."*

That last sentence is this section's interaction with the rest of the document:
a failed authorisation does not degrade the download, it **ends** it. Which is
the safe direction, and it must stay that way — §7.1 step 03 failing means steps
04 onward never run, and since step 05's unload is the first destructive step,
an authorisation failure leaves the device untouched. Any implementation that
"continues without authorisation" converts a clean refusal into §13's R5.

### 10.5 Devices that have no protected areas, and devices that ignore levels

**[D, corpus]** PROF, footnote 10 to §4.2's *"Authorization"* row and footnote 18
to §5.3's:

> *"The support of the A_Authorize- and the A_Keywrite-service does not imply
> that the device itself has access protected areas. If this is not the case, a
> device shall always allow – regardless of the attributed keys – access to the
> highest level (0), including when receiving an illegal key (‘illegal’ in this
> sense meaning another key than any of the keys entered in the key table)."*

So a device answering level 0 to any key at all is conformant, and an
authorisation that "succeeds" proves nothing about whether the device enforces
anything. The client must not report "authenticated" to a user on the strength of
a level-0 response.

Also from PROF: the *"Authorization"* row contains both `O` and `M` values across
the profiled masks, i.e. **there are profiled devices for which authorisation is
mandatory and others for which it is optional**. Row-level fact only (§1.1);
which masks those are is answered from PROF per mask, and MP §3.5 says the same
thing from the other end — *"In [14] it is specified for which Profiles
authorisation is mandatory."*

### 10.6 How an unauthorised operation fails on the wire

This is the part that decides how the implementation reports failures, and it
differs by service family:

| Service family | How insufficient access presents |
| --- | --- |
| Classic property services (`A_PropertyValue_Read/Write`) | No negative response. A read answers with `nr_of_elem = 0`; a write is simply not applied, and the read-back differs. |
| Classic memory services (`A_Memory_Read/Write`) | **[D]** AL §3.5.4: for a protected or nonexistent target *"the service indication shall be ignored"* — indistinguishable from a lost frame (§9.2 row 1). |
| Extended services (`A_PropertyExtValue*`, `A_MemoryExtended_*`, Function Properties) | A return code. **[D]** AL §3.4.5.5, §3.4.8.3, §3.4.9.1 and §3.4.9.2 define `FCh E_ACCESS_DENIED` alongside `FBh E_ACCESS_READ_ONLY`, `F9h E_TEMPORARILY_NOT_AVAILABLE`, `FDh E_ADDRESS_VOID`, `FEh E_DATA_TYPE_CONFLICT`, `F8h E_DATA_VOID` and `FFh E_ERROR`. |

The consequence for §9.2's table: **an insufficiently authorised classic write is
indistinguishable from a lost frame, from a protected region, and from a device
that is not listening.** The implementation therefore cannot diagnose
authorisation from a failed write. It must establish the level *before* writing —
which is exactly what §10.4's procedure returns — and record it in the session,
so that a later failure can be reported as "write failed; the session holds
level N, which may be insufficient" rather than as an unexplained timeout.

### 10.7 Writing keys: `A_Key_Write` and `DM_SetKey`

CP §3.5.2 step 11 ("Set access keys as required") is **[D]** MP §3.6 `DM_SetKey`,
parameters quoted: `DM_SetKey (flags, keys, level)`, *"flags — All bits are
reserved. These shall be set to 0."*, *"key — key for authorization"*,
*"level — level for which the key is to be set"*. Its procedure
`DM_SetKey_RCo` is connection-oriented, uses `A_Key_Write`, and its exception
handling is quoted: *"If the level returned in A_Key_Response is not the same as
in the A_Key_Write, the operation was not successful. Possibly an authorization
is required."*

The service underneath, **[D]** AL §3.5.8 `A_Key_Write-service`:

- *"Every device shall be able to handle exactly one key per access level. The
  number of access levels supported by a device is Profile dependent."* One key
  per level, and Figure 88's association table runs "Key for level 0 → 0" …
  "Key for Level N-1 → N-1", *"None → N (free access)"*.
- Deleting a key: *"If the key indicated in the A_Key_Write.ind primitive is
  FFFFFFFFh, then the corresponding key entry in the association table of keys to
  access levels shall be set to invalid, this is, then there shall be no key
  associated to the corresponding level any more."*
- The privilege rule, which is the trap: *"The current access level shall be less
  or equal to the access level indicated in the A_Key_Write.ind primitive,
  otherwise the remote application process shall return FFh in the
  A_Key_Response-PDU."* You cannot grant a level you do not hold. `FFh` in the
  response is the refusal, and per `DM_SetKey_RCo` above the client detects it by
  comparing the returned level with the requested one.

**Phase 2 does not implement key writing.** It is specified here because it is
CP §3.5.2 step 11 and would otherwise look like an omission, and because a
client that writes keys can lock a device out of its own reach — a `FFFFFFFFh`
written to the level the client uses removes the client's own access. That
belongs behind §2.3's mutation API with its own confirmation, after phase 3, and
not before the user has asked for it.

### 10.8 Interaction with the load procedures

- **Where it sits.** Step 03 of every procedure in §7 (complete download §7.1,
  partial download §7.4, unload §7.5) and step 1 of §9.1's recovery. After
  `DM_Connect`, before the Manufacturer ID check, before any unload. `DM_Connect`
  first is mandatory (MP §3.5, §3.5.2, §3.6, all quoted above).
- **Writing an LSM event can need more privilege than reading one.**
  **[D, corpus]** PROF Annex A's per-object Data Property tables carry a row for
  `PID_LOAD_STATE_CONTROL` (PID = 5) giving per-mask access levels in the notation
  *"read access level"/"write access level"* (PROF Annex A.1.2.1: *"In the
  following, the access levels are noted as “read access level”/”write access
  level”. … EXAMPLE 3/0 means a read access level equal to 3 and a write access
  level equal to 0."*). The values that appear in that row across the profiled
  masks include `3/3`, `3/1`, `3/0`, `15/2`, `15/1`, `3/(3)` and `(3/3)`. So
  reading the load state is generally permitted at the minimum level, while
  **writing a load event can require level 2, 1 or 0** depending on the device.
  Per-column attribution is not citable (§1.1); the row-level fact is: there
  exist profiled devices on which the client can watch the LSM but not drive it.
- **Which means the failure mode is specific and nasty**: pre-flight checks that
  only *read* succeed, the download starts, and the first `Start Loading` write
  is silently not applied (§10.6, classic property service, no negative
  response). The state then reads back unchanged, which §9.2 already tells the
  implementation not to interpret as success. Phase 2 must treat "event written,
  state unchanged, no error code" as a **possible authorisation failure** and say
  so in the report, alongside the other causes.
- **Levels in PROF Annex A are recommendations, not guarantees.** Table 3's
  legend defines the notation in terms of *"recommended default read access
  level"* and *"recommended default write access level"*, with `(n)` meaning the
  property *"may be read-only"* and `x` meaning read-only-or-writable-only-at-
  level-0-or-1. An implementation must therefore not precompute "this mask needs
  level 1"; it authorises for the best level it can obtain (§10.4) and reports
  what it got.

### 10.9 What phase 2 implements

1. A session type that owns an optional key, performs **MP §3.5.1
   `DMP_Authorize_RCo`** on every connect, stores the resulting level, and re-runs
   it on reconnect. §3.5.1 and not §3.5.2, per §10.4's scoping ruling: §3.5.2 is
   `Use • Profiles System 2 / BIM M112` and this document's download is System B.
2. No key ⇒ skip the procedure entirely (MP §3.5.1's `key != FFFFFFFFh` guard),
   and record "free level, unknown value" rather than "authorised".
3. Never send a key not supplied by the user (§10.2).
4. A failed authorisation aborts the whole procedure before any destructive step.
   MP §3.5.2 states this outright — *"Failure of any of the contained Application
   Layer Services shall lead to failure of the entire Configuration Procedure."* —
   and MP §3.5.1 reaches the same place through *"The general exception handling
   shall apply."* plus its own `A_Disconnect.ind ⇒ "error: connection was broken
   down"* annotation. Where the two procedures differ the implementation takes the
   stricter reading, which is abort.
5. MP §3.5.2's two-key comparison, implemented and tested against the simulator
   (§14 item 14), reachable only by explicit opt-in, and **off by default** — it
   is an extension this project chose and not a System B requirement (§10.4).
6. The simulator implements §10.2's two error rules, §10.5's "no protected areas"
   behaviour as a configurable mode, and a mode where reads are permitted and
   `PID_LOAD_STATE_CONTROL` writes are silently dropped — that last one being the
   failure §10.8 predicts and the only way to test for it without hardware.
7. No `A_Key_Write` (§10.7).

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
- `A_Authorize_Request` (`1111010001`) / `A_Authorize_Response` (`1111010010`)
  and, later and behind the mutation API, `A_Key_Write` (`1111010011`) — §10.
  The request carries one `must be 0` octet then four key octets (AL §3.5.7
  Figure 86); the response carries one level octet (Figure 87); `A_Key_Write`
  carries a level octet then four key octets (Figure 89).
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

**Identifiers, and why they exist.** Each item carries a stable id of the form
`GAP-T30-nn`. The ordinal numbering this section used through 2026-09-13 has now
been revised twice — once to withdraw the `L_Data_Extended` item and once to
reclassify the `60h` parity item — and `docs/RESEARCH.md` §8.7.15 and
`docs/KNOWN_LIMITATIONS.md` §7 had renumbered independently, so that "item 6"
meant the parity gap in one file and the MP §3.16 delay in the others. The ids
below are the canonical names, they are used verbatim in all three files, and
they are **never reused or renumbered**. A withdrawn item keeps its id and says
why it was withdrawn, so that anything citing it lands on the correction rather
than on a neighbour. Ordinals in the list are presentation only; cite the id.

The mapping from the ordinals the three files used before this pass, published
once so that older references resolve:

| Id | SPEC §12 ordinal (before) | RESEARCH §8.7.15 / KNOWN_LIMITATIONS §7 ordinal (before) | Status |
| --- | --- | --- | --- |
| `GAP-T30-01` | 1 | 1 | open |
| `GAP-T30-02` | 2 | 2 | open |
| `GAP-T30-03` | 3 | 3 | open |
| `GAP-T30-04` | 4 | 4 | open |
| `GAP-T30-05` | 5 | — (never listed there) | withdrawn 2026-09-13, documented |
| `GAP-T30-06` | 6 | 5 | **reclassified 2026-09-14** — documented; now a project risk decision |
| `GAP-T30-07` | 7 | 6 | open |
| `GAP-T30-08` | 8 | 7 | open |
| `GAP-T30-09` | 9 | — (never listed there) | open, narrowed |
| `GAP-T30-10` | 10 | — (never listed there) | closed 2026-09-13 |

1. **`GAP-T30-01` — the product-specific download matrix.** The *semantics of each individual
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
2. **`GAP-T30-02` — the `LdCtrl*` → load-control mapping, for 13 of 25 kinds.** RESEARCH §8.6
   matched 12 of the 25 `LdCtrl*` element kinds in `knx_master.xml` to
   documented load controls; the other 13 have no documented counterpart, and
   the element names themselves appear nowhere in the Standard corpus. §7.3
   documents the payload layouts for the events the *documented* ones map to.
   **Forbidden:** emitting an `Additional Load Control` for an `LdCtrl*` kind
   whose subtype mapping is not established. Note the specific hazard from
   §7.3: an unsupported allocation request does not return an error, it drives
   the LSM to `Error` (MP §3.31.3 subtype `0Ah`), from which only `Unload`
   escapes. Guessing here costs a configuration, not an error message.
3. **`GAP-T30-03` — vendor DLL involvement in download.** RESEARCH §8.6 established **[V]**
   that no vendor DLL is *required* to reconstruct a download sequence — the
   step list is declarative product data — and that `EtsDownloadPlugin` appears
   on 5 of 35 corpus application programs without supplying the step list. What
   those plugins *do* is undocumented and, being compiled code, is not
   documentable from either knowledge base.
   **Forbidden:** claiming a device with an `EtsDownloadPlugin` is supported.
   The correct behaviour is to detect the hook, report it, and decline. See also
   `KNOWN_LIMITATIONS.md` §6.
4. **`GAP-T30-04` — the "differential download algorithm"** named in CP §3.5.3. The CRC
   comparison that gates it is specified; the algorithm is not.
   **Undocumented in both knowledge bases as searched** (`knx_spec_kb_programming`
   and the 177-PDF text-only base), and in the extracted text of the eight PDFs
   of §1.1, in which the phrase occurs only at CP §3.5.3 as a name.
   **Forbidden:** implementing or claiming differential download. Permitted:
   comparing the CRC and skipping an unchanged part.
5. **`GAP-T30-05`** — ~~**the `L_Data_Extended` discovery mechanism.**~~ **Withdrawn — this is
   documented, and listing it here was an error of this document.** It is
   `PID_MAX_APDU_LENGTH`, RES §4.3.7, quoted in full in §6.4: range 15–254, and
   *"A Management Client supporting the L_Data_Extended-frame has to check this
   value before starting download"*, with absence meaning standard frames and a
   15-octet APDU, which is where MP §3.16's 12 comes from (15 − 3). The item is
   kept at its number rather than removed so that anything citing "§12 item 5"
   lands on the correction instead of on a renumbered neighbour.
   **Still forbidden:** writing more than 12 octets per `A_Memory_Write` on the
   basis of an assumption — but the value is now obtainable, from the Device
   Object of the device in front of you, and only from there (§6.4).
6. **`GAP-T30-06`** — ~~**the programming-mode parity rule** at memory address
   `60h`.~~ **Reclassified 2026-09-14: not a documentation gap. It is documented,
   and this document had cited the clause that documents it while claiming the
   opposite.** RES §4.26.3.1 states the coverage (*"the complete range bit 0 to
   bit 6 may be parity controlled"*) and the client-side operation (*"if the value
   of prog_mode is changed from "0" to "1" or from "1" to "0" then the variable
   p_parity shall be inverted"*), and RES §4.26.3.4.1 restates the latter as a
   client obligation. §4.4 now specifies the derivation: read `0060h`, invert bit 0
   and bit 7, preserve bits 1–6, write back, and skip the write entirely when the
   mode already matches. No parity *convention* — odd or even — has to be known,
   because the rule is invert rather than recompute. The earlier entry here is the
   inverse of the failure §1 is written to prevent: not a guess dressed as a fact,
   but an admitted gap that the cited clause closes, which is worse in one specific
   way — a later task copies it forward as settled.
   The residual unknown is narrow and is **not** counted as a gap: whether a given
   device's stack checks odd or even parity is manufacturer-specific by design —
   **[D]** RES §4.26.3.3: *"The reaction of an invalid parity value is manufacturer
   specific"*, and if no parity checking is used at all *"the value of p_partity
   shall be ignored"*. A per-device property is not a documentation gap.
   **Still forbidden, on different grounds:** writing to `60h` on a real device.
   That is now a **project risk decision** and is recorded as R11 in §13, not as a
   gap here. The Standard tells us how; this project declines because the failure
   mode is a restarted device (RES §4.26.3.3 footnote 96) and the recovery is a
   site visit. §15 keeps `DM_ProgMode_Switch`'s write path as a non-goal, and §4.4
   keeps it inside §2.3's mutation API so that the refusal survives a refactor as
   well as a reading.
7. **`GAP-T30-07` — the unquantified "delay for programming the memory in the
   device"** (MP §3.16.2, non-verify path). Named, never given a value. The gap
   survives this pass; what changes is that the earlier entry called MP §3.16
   *"the only clause that mentions"* it, and it is not.
   **What was actually searched, and found.** The sequence line *"delay for
   programming the memory in the device"* occurs **five** times in MP — §3.16.2
   `DMP_MemWrite_RCo` (footnote 12), §3.16.4 `DMP_MemWrite_LEmi1` (13), §3.19.2
   `DMP_UserMemWrite_RCo` (14), §3.38.2 `DMP_LCSlaveMemWrite_Rco` (16) and §3.41.2
   `DMP_LCExtMemWrite_Rco` (17) — each carrying the same footnote, quoted from
   footnote 12: **[D]** *"The delay time depends on the Management Server and on the
   amount of written octets (see [08])."* (§3.16.4's reads *"shall depend"*; the
   other four read *"depends"*.) So the Standard states the delay's two
   **dependencies** — the device, and the byte count — and gives neither a figure
   nor a formula for either. Five occurrences agreeing is five times the same
   silence, not corroboration of a value.
   **The `(see [08])` pointer was followed**, which the earlier entry had not done.
   MP `[08]` is `Chapter 3/7/2 "Datapoint Types"` — this document's **DPT**. It
   holds nothing relevant: the only delay datapoint in it is `DPT_Time_Delay`
   **20.013**, `Use: FB`, range `[0 to 25]`, an enumeration of Functional Block
   time steps from *"0 : not active"* and *"1 : 1 s"* up to *"25 : 24 h"* with
   *"26 to 255 : reserved, shall not be used"*, appearing as `PART_Time_Delay`,
   `PART_Cycle_Time` and `PART_Prewarning_Delay`. A one-second minimum step for an
   HVAC-style parameter is not a per-octet EEPROM programming delay, and nothing in
   DPT connects 20.013 to `A_Memory_Write`. The pointer resolves to a dead end, and
   saying so is the point: the reference was followed and yields nothing, which is
   a different statement from the reference not having been noticed.
   **Undocumented in both knowledge bases as searched** — `knx_spec_kb_programming`
   (27 programming PDFs, with figures) and the 177-PDF text-only base — and
   undocumented in the `pdftotext -layout` extractions of **MP** and **DPT**.
   **Forbidden:** presenting any chosen default as a specification value. Permitted,
   and what phase 2 does instead: use the Verify-Mode path (MP §3.16.3), where the
   read-back replaces the delay with an observation. The delay only matters on the
   non-verify path, and §6.2's project requirement means this project has no
   non-verify write path without a following read anyway — which converts this gap
   from a blocker into a documented reason for a design choice.
8. **`GAP-T30-08` — LSM Realisation Type 2.** **[D]** RES §4.23.3: *"This Realisation Type is
   not specified in this version of this document."*
   **Forbidden:** claiming support for devices that require it.
9. **`GAP-T30-09` — cross-LSM ordering between loadable parts, as a general
   rule.** CP §3.5.2 gives one concrete order for System B, which this document
   follows.
   **The Standard delegates this requirement to per-device Profiles, and that is
   the fact that makes the rest of this item matter.** **[D]** RES §4.23.2.4.1:
   *"If a device contains more than one Load State Machine, dependencies between
   these Load State Machines have to be defined in the Profiles (see [17]) of these
   devices."* RES `[17]` is `Volume 6 "Profiles"` — this document's **PROF**.
   So the question is not "does the Standard state an ordering rule"; the Standard
   has answered that, with "the Profiles do". The question is whether PROF states
   one.
   **PROF was searched for it, and states none.** Named so that a later reader can
   re-run it rather than trust it:
   - Base `knx_spec_kb_full179_clean.sqlite` (the 177-PDF text-only base), query
     `"Load State Machine"`, limit 5. The only fact row it returns whose source is
     `06 Profiles v02.01.01` has content *"Load State Machine a. Realisation Type
     1"* — a table row label. The queries `"Load State Machine dependencies"` and
     `"load order"` both return `[]`.
   - Base `knx_spec_kb_programming.sqlite` (27 programming PDFs, with figures),
     queries `"Load State Machine dependencies Profiles"`, `"dependencies"` and
     `"ordering"`: `[]`, `[]`, `[]`. `"Load State Machine"` returns RES §4.23.1's
     definition, not a PROF row.
   - `pdftotext -layout` extraction of `06 Profiles v02.01.01.pdf`, swept
     case-insensitively for `dependenc|shall be sorted|order of load|loading
     order|sequence of load`: **one** hit, §9.1.2.6.4 Security Interface Object,
     *"Further dependencies, e.g. accessibility in function of the Security Mode
     …"* — Property accessibility, not load ordering. The six occurrences of
     "Load State Machine" in the same extraction are table rows, §4.2.7.1, §5.3.8.1,
     footnote a of §5.3 (quoted in §5.4) and Annex A's `5.1 Load State Machines`
     support row. None is an ordering.
   The `"shall be sorted"` requirements in the vicinity are RES's, and they concern
   the *contents* of the Group Address Table (§5.6, value 15), not the order in
   which parts are loaded. What PROF does constrain is which Load Controls a mask
   must support (§7.3), which Table 94 cell a mask may not take (§5.4), whether
   authorisation is mandatory (§10.5), whether Realisation Type 1 is mandatory
   (§3.4), and that an unimplemented Verify Mode *"shall always be off"* (§6.3).
   None of those is an ordering.
   **The consequence, and it is stronger than "the Standard is silent".** Because
   RES §4.23.2.4.1 delegates to *"the Profiles … of these devices"*, the absence of
   an ordering rule in `06 Profiles` is **not** the absence of an ordering rule for
   a given device. The per-mask Profile documents — `06_01_33 mask 2705h`,
   `06_01_35 mask 27B0h`, `06_02_42 mask 2920h`, `06_03_31 mask 2311h` and their
   siblings in Volume 6, none of which this document has read — are exactly where
   such a dependency would live, and a device whose Profile states one is
   conformant while this document knows nothing about it. A negative from the
   general Profiles document is therefore evidence about that document only.
   **Forbidden, and now for a better reason:** generalising CP §3.5.2's order to
   masks it does not cover, and treating "Volume 6's general document states no
   ordering" as licence to reorder. Phase 2 follows the step list of the
   Configuration Procedure that matches the device's mask version, in the order
   that procedure prints, and where no procedure matches it declines. It never
   reorders parts on the strength of this negative. Reading the per-mask Profile of
   a device actually in front of us is a phase 3 task, and a finding if it turns out
   to state a dependency.
10. **`GAP-T30-10`** — ~~**`DM_Authorize`'s parameters and the `DPT_ErrorClass_System` values.**~~
    **Closed 2026-09-13.** This item used to list four things as "documented but
    not read for this document". All four were read and folded in:
    `DM_Authorize`/`DM_Authorize2_RCo`/`DM_SetKey` and the services under them
    are §10; `DPT_ErrorClass_System` 20.011's nineteen values are §5.6; AN194's
    per-resource reset semantics are §8.2; PROF's constraints on the LSM are in
    §3.4, §5.4, §6.3, §7.3 and §10.5. The item is kept as a marker because the
    distinction it recorded is the one worth preserving: **"not in the Standard"
    and "not yet read" require completely different responses**, and a document
    that blurs them is the failure mode §1 exists to prevent. Items 1–4 and 6–8
    are the first kind. There are now none of the second kind.

**Six genuine gaps remain:** `GAP-T30-01`, `-02`, `-03`, `-04`, `-07`, `-08`, plus
`GAP-T30-09` as a gap that has been *narrowed to a delegation* — the general
Profiles document states no rule, per-mask Profiles are where one would live, and
none has been read. Counting `-09` as a seventh open gap or as a closed one would
both be wrong, so it is listed separately and the headline number is six.

Two items left the list without being solved, and the distinction is the point of
this section: `GAP-T30-05` was **withdrawn** because it was never a gap, and
`GAP-T30-06` was **reclassified** because it was never a gap either — the
documentation existed and this document had cited it. `GAP-T30-10` is **closed**:
everything it recorded as unread was read. Three different reasons for three
different disappearances, none of them "we implemented a workaround".

Each of the six names which knowledge base or corpus was searched, and
`GAP-T30-07` and `-09` now name the query strings as well, which is what lets a
later reader tell "the Standard is silent" apart from "nobody looked" — and, in
`-09`'s case, apart from "the Standard delegated it somewhere nobody looked".

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
| R6 | Skipping the client-side verification read (§6.2) | Same as R5, reached by a different route: without a read-back, silent corruption is undetectable. **[A]** The authority here is **this project's design rule, not the Standard's**: AL §3.5.4's *"The value of the associated memory area shall be explicitly read back after writing to it."* sits inside the **active-Verify-Mode** paragraph and constrains the **device**, and with Verify Mode inactive AL §3.5.4 obliges nobody to read anything back. The risk is real either way; the obligation to avoid it is self-imposed. | Full re-download |
| R7 | Emitting an `Additional Load Control` with a guessed subtype or size (§7.3, §12 item 2) | **[D]** MP §3.31.3 subtype `0Ah`: an unsupported request drives the LSM to `Error`. §5.4: only `Unload` leaves `Error`, and **[D]** RES Table 93: unload makes the data *"undefined"*. The part is destroyed, not merely failed. | Unload + full re-download |
| R8 | Beginning a download without the full payload resolved | `Start Loading` from `Loaded` immediately invalidates a working configuration (§5.4). If the data is then unavailable, the device stays invalid indefinitely — and the state is non-volatile, so power-cycling does not help. | Full re-download once the data exists |
| R9 | Interrupting a download (network, crash, operator) | Part left in `Loading` or `Error` after restart (**[D]** RES Table 94 `Device Restart` row: from `Loading`, `R: Loading` / `O: Error`), invalid per Table 92, persistent per §4.23.1. | §9.1 recovery |
| R10 | Treating `LoadCompleting` silence as failure and retrying | Writing load events at a device that is *"offline during state LoadCompleting"* (**[D]** RES Table 94 footnote). Per Table 94, `Start Loading` or `Load Completed` from `LoadCompleting` is `R: Error`. A correct device mid-checksum is driven into `Error` by the client's impatience. | Unload + re-download |
| R11 | Writing to `60h` **at all** on a real device (§4.4) | Not a parity risk any more — §4.4 derives `p_parity` from RES §4.26.3.1 and §4.26.3.4.1 and computes it by inversion, so the octet this project would write is correct by construction. The risk is the write itself. `0060h` is a **device-control** address in the `curr_prog_mode` region whose bits 1 to 6 are **[D]** RES §4.26.3.1 *"shared with other functionality"* and therefore manufacturer-specific: a one-octet write there is a read-modify-write of somebody else's state, racing whatever the device's own firmware does to those bits, and a wrong or stale octet is **[D]** RES §4.26.3.3 *"manufacturer specific"* in its reaction — footnote 96: *"Typically the system is restarted if p_parity is invalid."* Knowing how to build the byte removed the accidental protection that ignorance provided, so the prohibition is now an explicit decision rather than a side effect of a gap. | Unknown, which is why the write half of `DM_ProgMode_Switch` stays inside §2.3's mutation API, out of phase 3 entirely, and simulator-only in phase 2 |
| R12 | Polling the load state faster than the spec allows, or not reconnecting | **[D]** RES §4.23.2.4.1 caps the read period at 3 s and requires periodic reconnection. Too-fast polling loads the bus during a download; failing to reconnect makes a legitimate long transition look like a failure, which leads to R10. | Fix the client |
| R13 | Skipping the Manufacturer ID check (CP §3.5.2 step 04) | One manufacturer's application downloaded into another's device. The Standard puts this check before any write for exactly this reason. Outcome is undefined and device-specific. | Unload + correct download, if the device still communicates |
| R14 | Assuming programming mode is still on (§4.3) | The write is ignored (AL §3.2.2, no negative response), the operator believes the address was programmed, and the project's model of the installation diverges from the installation. Silent, and discovered later at the worst moment. | Re-run §4.2 |
| R15 | Recording "downloaded" from intent rather than from a device read (§11.2) | The project claims a device is configured when it is not. Every later decision — group address assignment, diagnostics, the next partial download — is built on a false premise. | Re-read device state |
| R16 | Losing Verify Mode on reconnect (§6.3) | Writes silently stop being confirmed (**[D]** RES §4.2.14.7.4: it is auto-disabled when the TL connection closes), degrading to R5 without any visible change. | Re-assert bit 2 per connect |
| R17 | Sending a guessed or default access key (§10.2) | **[D]** AL §3.5.7: an invalid key makes the device *"select the minimal access level (this is level 3 or level 15)"*. The client ends up with **less** access than if it had never authorised, and with no error to show for it. Every subsequent classic write is then silently ignored (§10.6), which lands on R5. | Re-authorise correctly on a fresh connection |
| R18 | Losing authorisation on reconnect (§10.3) | The level is valid only *"until the connection is released"* (**[D]** AL §3.5.7). After the reconnect that §5.5 requires during a long load transition, the session is at the free level. Writes stop being applied, without a negative response. Indistinguishable from a dead bus. | Re-authorise per connect, as one unit with Verify Mode |
| R19 | Writing a key (`A_Key_Write`, §10.7) | A key written to the level the client itself uses, or `FFFFFFFFh` written to it, removes the client's own access. **[D]** AL §3.5.8's privilege rule means the client may not be able to undo it: *"The current access level shall be less or equal to the access level indicated"*. The device stays manageable only by whoever holds a better key. | Nothing, from this application. Not implemented |

R1 has no recovery row because it has no recovery. That is the argument for
enforcing it in the type system rather than in a review checklist.

R11 is the one row whose *reason* changed during fix round 2 without its verdict
changing. It used to forbid the write because the byte could not be computed.
The byte can now be computed (§4.4), and the write is still forbidden on real
hardware — on the grounds above, and as a project risk decision rather than as a
consequence of a documentation gap. §12 records the same reclassification from
the other side.

## 14. Testing (phase 2, no hardware)

1. **The transition table of §5.4**, exhaustively: 6 states × 6 events, with
   the `R:`/`O:` alternatives both accepted. Pure, no I/O, and it is the test
   that would have caught R7 and R10.
2. **`Error` is a trap**: assert that no event other than `Unload` leaves it.
3. **Chunking** at the 12-octet default and at 63, including a region that
   straddles `FFFFh` so the §6.5 service-selection rule is exercised on
   `base + length`, not on `base`.
4. **Read-back enforcement**: a write path without a verification read must not
   compile, or at minimum must not exist — asserted by the simulator rejecting a
   session that writes without reading back. Both branches of §6.2 are covered:
   Verify Mode active, where the comparison is against `A_Memory_Write.res`, and
   Verify Mode inactive, where an explicit `A_Memory_Read` must follow. This
   enforces *this project's* rule, not a Standard obligation on the client, which
   is exactly why it needs a test rather than a citation.
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
11. **The exclusion guard — the test that proves the refusal (§2.1).** Not one
    test but a set, all asserting refusal at the guard rather than at the call
    site: a single read of `1.1.220` is refused; a single write to `1.1.220` is
    refused; a plan containing `1.1.220` cannot be constructed; a range
    `1.1.200`–`1.1.240` that spans it is refused rather than silently shortened;
    a retry list that would re-add it is refused; and a broadcast operation
    cannot be issued when the programming-mode count is not exactly one —
    asserted as §2.1's witness value being unconstructible for a count of zero
    and for a count of two, not merely as a runtime check returning an error.
    Each refusal is
    also asserted to be *reported*, because a silent refusal and an absent guard
    look identical from outside. This is the test that makes R1 structural
    instead of procedural, and phase 2 is not done without it.
12. **Property-based**: any sequence of events applied to the model LSM never
    reports `Loaded` unless a `Load Completed` from `Loading` succeeded.
13. **The APDU-length rule of §6.4**: property absent ⇒ 12; property present with
    15 ⇒ 12; with 254 ⇒ capped at 63 by the service limit; with 255 ⇒ treated as
    the ESCape Code and not as a length; a value found only in the Router Object
    ⇒ still 12, per RES §4.3.7.2.2.
14. **The authorisation algorithms of §10.3 and §10.4.** MP §3.5.1, the default
    path, against a simulator configured three ways: no key supplied (procedure
    skipped, recorded as "free level, unknown value"); a valid key; and an invalid
    key, where the device drops the partner to the minimum level (§10.2) and the
    client must notice rather than proceed. Then MP §3.5.2's opt-in extension
    against a simulator configured two more ways: free level worse than the client
    key's level, and free level better than it, so the third exchange must happen.
    Plus one test of the scoping ruling itself: with the extension off — the
    default — a System B session must issue exactly **one** `A_Authorize_Request`
    per connection, never the `FFFFFFFFh` probe. A default that quietly runs the
    profile-scoped procedure is the regression this item exists to catch.
    A simulator with no protected areas (always level 0, §10.5) is configured
    for both paths, since a successful authorisation proving nothing is orthogonal
    to which procedure produced it.
15. **Re-authorisation on reconnect**: the simulator drops the connection
    mid-download; the client is asserted to re-authorise *and* to re-assert
    Verify Mode before the next write (§10.3, §6.3).
16. **The silent-authorisation-failure case of §10.8**: a simulator that permits
    reads and silently drops `PID_LOAD_STATE_CONTROL` writes. The client must
    report "state unchanged after an event write; possible causes include
    insufficient access level" and must not report success.
17. **`DPT_ErrorClass_System` round-trip** (§5.6): 0–18 decode to their names,
    19–255 are surfaced as unknown rather than mapped, and the decoder is shared
    with the DPT 20.011 codec rather than duplicated.
18. **The `curr_prog_mode` toggle of §4.4, simulator-only.** Three properties,
    each asserted against a simulator whose `0060h` octet starts with arbitrary
    bits 1 to 6: the toggle preserves those bits exactly; it inverts bit 0 and
    bit 7 together and never *computes* a parity from scratch, so the same
    derivation works for an odd-parity and an even-parity device without the
    convention being known; and it issues **no write at all** when bit 0 already
    equals the requested mode (RES §4.26.3.4.2/§4.26.3.4.3's guards), because a
    no-op toggle would invert the parity against an unchanged `prog_mode` and
    produce the invalid octet RES §4.26.3.3 leaves manufacturer-specific. Plus
    the refusal: the write half must be unreachable without §2.3's authorisation
    value, and unreachable against a non-simulator transport in any phase (§13
    R11, §15).

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
  mask-`070nh` `_RCo_Mem` variant, no Master Reset with a destructive Erase
  Code, no `SerialNumber_IndividualAddress_Write`, and **no
  `A_Key_Write`/`DM_SetKey`** (§10.7) — the authorisation client is read-only
  with respect to the device's key table.
- **No `DM_ProgMode_Switch` write against real hardware, in any phase.** §4.4
  now specifies the write completely, which is a change from the previous
  revision: it used to be un-specifiable because the `p_parity` computation was
  believed undocumented. Specifying it does not unblock it. The write half lives
  behind §2.3's mutation API, is exercised only against phase 2's simulator, and
  is **out of scope for phase 3**. The toggle's *read* half — one octet from
  `0060h` — is read-only and therefore allowed in phase 3, but only inside
  §2.2's approved range. §13 R11 carries the risk; §12 records why it is no
  longer a documentation gap.
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
