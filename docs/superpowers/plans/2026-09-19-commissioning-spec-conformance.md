# Commissioning: conformance with CP §3.5, MP §2/§3, RES §4 and TL §3/§4

Backlog derived from two read-only specification audits run on 2026-09-19
against the KNX Standard PDFs themselves, not against the knowledge bases:

- `.superpowers/sdd/2026-09-13-goal-completion/spec-audit-1-partial-download.md`
  — CP §3.5.3, all five partial-download variants.
- `.superpowers/sdd/2026-09-13-goal-completion/spec-audit-2-complete-download-ia.md`
  — CP §3.5.2, §3.5.4, `recovery()`, MP §2.3 individual-address programming,
  and the complete table of timing obligations.

Both audits corrected the same thing in their briefs: **document page N is PDF
page N**. Every page number below is a document page.

Short names used throughout:

| Short | Document |
|---|---|
| CP | `03_05_03 Configuration Procedures v02.01.01 AS.pdf` |
| MP | `03_05_02 Management Procedures v02.01.02 AS.pdf` |
| RES | `03_05_01 Resources v01.10.01 AS.pdf` |
| AL | `03_03_07 Application Layer v02.01.01 AS.pdf` |
| TL | `03_03_04 Transport Layer v01.02.03 AS.pdf` |

## Global constraints

1. **No task in this file opens a socket to the live bus.** Everything is
   provable against the simulator and the unit suite. The hardware-safety rules
   in `goal.md` §3 T30 remain in force unchanged: `1.1.220` is an alarm panel
   and is never read, written or scanned, and nothing writes to a real device
   without an explicit user go-ahead naming the device and the operation.
2. **The simulator is currently more permissive than RES on exactly the point
   C1 turns on.** Until C1's simulator half lands, the suite keeps passing for
   the wrong reason. C1's simulator half is not optional.
3. Every behavioural claim in a comment must name its clause **and** its page,
   in the form `CP §3.5.3, p. 46`. Two of the defects below exist only because
   a comment cited a clause that says the opposite.
4. Eight gates, judged by exit status: `cargo fmt --all -- --check`,
   `cargo clippy --workspace --all-targets -j 2 -- -D warnings`,
   `cargo test --workspace --no-fail-fast -j 2`,
   `cargo run -p xtask -- check-layering`, `cargo run -p xtask -- check-headers`,
   `cargo deny check`, plus `npx tsc --noEmit` and `npx vitest run` in
   `apps/knx-web` when web files are touched.
5. `check-headers` ceiling is 168 with zero slack.

## Explicit non-tasks

- **`PID_GROUP_RESPONSER_TABLE` stays unimplemented.** CP §3.5.3 footnote 8,
  p. 47: *"Applicable for PL110 devices only."* RES §4.16.8.2.5, p. 239: *"This
  Property is mandatory for PL110 devices. For all other media this Property
  shall not be implemented."* The cost on TP1/RF/IP is zero. The obligation
  this creates is a refusal, not an implementation: a plan targeting a PL110
  device must be declined rather than silently missing CP §3.5.3 AP2 Nr. 11's
  second half.
- **CP §3.5.4 step 07 (unload the individual address) stays unimplemented.**
  Making a device unaddressable by broadcast is not a thing this application
  does by accident. C6 makes the *name* honest instead.
- **No procedure-level retry loop.** MP §3.1, p. 68 is the whole of the
  Standard's exception handling — *"the download shall be interrupted and an
  error-message shall be raised"* — which is the opposite of a retry
  obligation. `recovery()`'s existing comment is correct and stays.

---

## Tier 1 — independent, no dependencies

### C1 — write `PID_PROGRAM_VERSION` only to the objects that have it

**Severity: correctness. Highest value in this file.** Everything else is a
trace label or a minor timing; this one fails on real hardware.

`load_one_part` (`crates/knx-net/src/commissioning/download.rs:801-809`) writes
`PID_PROGRAM_VERSION` unconditionally, for all five parts, in all four
procedures. RES does not give that property to three of them:

> RES Table 77 — Group Address Table, Realisation Type 7, "Used by: System B",
> p. 238: `PID_OBJECT_TYPE`, `PID_LOAD_STATE_CONTROL`, `PID_TABLE_REFERENCE`,
> `PID_TABLE`, `PID_MCB_TABLE`, `PID_ERROR_CODE`,
> `PID_GROUP_RESPONSER_TABLE`. No `PID_PROGRAM_VERSION`.

Same for RES Table 80 (Association Table, p. 249) and Table 85 (Group Object
Table, p. 270). Only Tables 90 and 91 (Application Program 1 and 2, pp. 288,
290) carry `13 = PID_PROGRAM_VERSION`.

On hardware the write hits a property RES says is absent: `PropertyRefused`
(`commissioning.rs:1570`) **after** the data is written and **before**
`LoadCompleted`, stranding the segment in `Loading`.

The clauses disagree with each other, which is why the fix is tolerant rather
than absolute:

> CP §3.5.3, Group Address Table variant Nr. 06, p. 54: *"Set
> GroupAddressTable.ApplicationVersion — MaC: PropertyWrite(ID_
> GroupAddressTable, PID_PROGRAM_VERSION)"*

> CP §3.5.3, Association Table variant Nr. 06, p. 56: the same for
> `ID_AssociationTable`. The GOT variant likewise, pp. 51-52.

CP §3.5.2's table steps 08/09/10 (pp. 43-44) list **no** such write. Audit 1
could not find a reconciling clause; the question belongs to Volume 6 Profiles
Annex A, which neither audit opened (see C18).

**Do:**
1. Introduce a `PartKind` on `LoadablePart` distinguishing the two application
   programs from the three tables.
2. Write `PID_PROGRAM_VERSION` on the application programs only.
3. On the three tables, if a plan supplies a version, attempt the write and
   treat `PropertyRefused` as an **expected** outcome recorded in the report,
   never as a procedure failure. Both readings of the contradiction then
   produce a device that ends in `Loaded`.
4. **Teach the simulator to refuse properties RES does not list for an object.**
   `simulator.rs:606` currently stores any `(object_index, property_id)` pair,
   which is what hides this defect today. Without this half, C1 proves nothing.

**Acceptance:** a test that a partial download of the Association Table
completes with the version write refused and the segment in `Loaded`; a test
that the simulator refuses `PID_PROGRAM_VERSION` on an object outside Tables 90
and 91; the existing suite still green.

### C2 — parse the MCB and compare the CRC, not all eight octets

**Severity: correctness, plus a false "undocumented" claim in two files.**

`commissioning.rs:1366-1376` states the Memory Control Block's layout *"is not
specified in either knowledge base (spec §12), so this crate carries it and does
not pretend to parse it"*. `simulator.rs:57-60` repeats it. **It is specified:**

> RES §4.2.27, Table 12 — Memory Control Block (MCB) Table, p. 39:
> `Segment Size 1 [4 octets] | CRC Control Byte [1 octet] | Nr. of elements
> Read Access 1 [4 bit] | Write Access 1 [4 bit] | CRC [2 octets]`

> RES §4.2.27.1.1, Table 13 — CRC Control Byte, p. 39: Bit 0 — *"0 : CRC is
> always valid / 1 : Contents of protected memory area may change"*.

> RES §4.2.27.1.2, p. 39: *"The CRC checksum calculation shall provide a higher
> consistency while using differential download. The CRC shall be calculated
> properly by the Management Server (device) on the transition from the load
> state 'Loading' to the load state 'Loaded' … Its value shall be valid in the
> load state 'Loaded' only. The 16 bit CRC shall be calculated according to the
> CRC16-CCITT specification."* — parameters, p. 40: width 16, polynomial
> `1021h`, initial value `FFFFh`, input not reflected, output not reflected, no
> output XOR.

`download.rs:789-793` compares `*stored == current` over all eight octets.
Three consequences:

1. **Segment Size is octets 0-3.** A partial download allocates before it
   compares (`load_one_part:779-784` precedes the MCB read at `:788`), so any
   change in payload length changes octets 0-3 and the comparison reports
   `Differed` for a reason that has nothing to do with the CRC. The clause says
   to compare the CRC — two octets.
2. The read/write access nibbles (octet 5) pollute the comparison too.
3. **CRC Control Byte bit 0 is ignored.** When it is 1 the device is saying the
   protected memory may have changed since the load, so a matching CRC proves
   nothing.

**Do:** a typed `MemoryControlBlock` in `knx-core` parsing Table 12; compare the
two CRC octets alone; add a fourth `CrcComparison` outcome for *"the device says
the CRC cannot be trusted"* when bit 0 is set; delete both "not specified"
comments and cite RES §4.2.27, p. 39 instead.

**Acceptance:** round-trip parse test over Table 12's layout; a test that a
changed segment size with an unchanged CRC reports `Matched`; a test that bit 0
set yields the new outcome regardless of the CRC octets.

### C3 — two wrong facts in the individual-address step descriptions

**Severity: correctness.** Pure data change in `procedure.rs`, no I/O.

**(a) The time-out is 1 s, not 3 s.** `procedure.rs:211-215` says *"wait out the
full 3 s time-out"*.

> MP §2.3, p. 14, step 2's own remark column: *"time-out: 1 s"*

The 3 s belongs to a different procedure — MP §2.2 `NM_IndividualAddress_Read`,
p. 12: *"time-out: 3 s"*. The rest of that sentence is right and stays:

> MP §2.3, p. 15: *"to 2.: The Management Client shall always wait until the
> time-out has elapsed. It shall collect all the responses during this
> time-out."*

> MP §2.2, p. 13: *"If two or more responses with the same Individual Address
> are received, there is more than one device with the same Individual
> Addresses."* / *"The Management Client shall not evaluate Layer-2
> repetitions."*

**(b) Occupancy must not abort the procedure.** `procedure.rs:202-206` says
*"any answer means the address is occupied and the procedure stops"*.

> MP §2.3 exception handling, p. 15: *"to 2.: … • A device with the Individual
> Address IA_new to be assigned exists, but it is not the one that is in
> Programming Mode. ⇒ The Management Client shall not continue with the
> Management Procedure. • A device with the Individual Address IA_new to be
> assigned exists, and it is the one that is in Programming Mode. ⇒ The
> Management Client shall continue with the Management Procedure. • No device
> with the Individual Address IA_new to be assigned exists. ⇒ The Management
> Client shall continue with the Management Procedure."*

> MP §2.3, p. 15: *"to 1.: … The Management Client shall continue with the
> Management Procedure in every case."*

Re-assigning a device to the address it already holds is the middle bullet and
is explicitly legal — which is exactly what step 3 guards with
`if IA_new != IA_current` (p. 15). The flat "stops" makes a legal
re-programming impossible.

**Ruling on the contradiction** (the clause disagrees with itself — body p. 14:
*"if A_Disconnect-PDU is received then IA_new shall be regarded as occupied; end
procedure"*): implement the exception text, which is the more specific and the
later statement, and surface the occupancy to the operator as a finding rather
than as a stop. Cost if wrong: a re-programming attempt that a stricter reading
would have refused; recoverable, and the operator sees it. Record the
contradiction in `docs/KNOWN_LIMITATIONS.md` (C7).

**Acceptance:** the declarative text carries 1 s with its page citation; the
step-1 description distinguishes the three bullets; no behavioural code moves.

### C4 — the transport gives up one repetition early

**Severity: correctness, minor.**

> TL §3, p. 15: *"If no acknowledgement is received or if the acknowledgement is
> a negative acknowledgement, the local Transport Layer shall repeat the
> transmission of the T_DATA_CONNECTED_REQ_PDU up to 3 times with an
> acknowledgment time-out time of 3 s."*

> TL §4, p. 16: `max_rep_count 3; maximum of T_Connect.req repetitions`

`commissioning.rs` `exchange_inner` sets `attempts = 1` on the initial send and
bails at `attempts >= MAX_REP_COUNT`, giving 3 transmissions — the original plus
**2** repetitions. The clause permits 3 repetitions, i.e. 4 transmissions. The
constant is named after the Standard's but used as a transmission cap. A device
that would have answered the third repeat is abandoned as
`SessionError::NoAnswer`.

**Do:** 4 transmissions; rename the constant so it cannot be misread again
(`MAX_TRANSMISSIONS = MAX_REP_COUNT + 1`, or keep `MAX_REP_COUNT` and compare
against `repetitions`, not `attempts`).

**Acceptance:** a test asserting exactly 4 transmissions before `NoAnswer`, and
that an answer arriving on the fourth is accepted.

### C5 — the wait loop skips the "once more" attempt

**Severity: correctness, minor.**

> RES §4.23.2.4.1, p. 297: *"If a before established TL-connection breaks down,
> the MaC shall try to re-establish the connection periodically during the
> maximum transition time and once more when the maximum transition time has
> passed."*

`wait_for_load_state` reconnects inside the loop — citing this very clause — but
returns `TransitionTimedOut` the moment `started.elapsed() >= max_transition`.
The clause asks for exactly one more attempt after the deadline.

**Acceptance:** a test where the simulator answers only after `max_transition`
has elapsed and the procedure still succeeds; a test that a second post-deadline
attempt is *not* made.

### C6 — three source strings that name the wrong authority

**Severity: traceability.** Three one-line edits, one of which is a comment that
says the opposite of what its clause says.

**(a) `recovery()` is the project's own invention.** `procedure.rs:552` reads
`source: "spec §9.1"` and `download.rs:624` opens `/// Spec §9.1, recovery after
an interrupted download.` — both of which read as Standard clauses to anyone
who does not know the internal numbering. **The Standard prescribes no recovery
procedure at all.** Audit 2 searched both knowledge bases: `recovery` → `[]` in
the programming base and two unrelated `BusReturnMod` hits in full179;
`interrupted download` → `[]` / `[]`; `restart procedure download` → `[]` / `[]`
— with `load state`, `download` and `Verify Mode` returning hits as a sanity
check. The whole of the Standard's position is MP §3.1, p. 68, quoted under
"Explicit non-tasks". Fix: `source: "KNXBench design spec §9.1 — no Standard
equivalent"`.

**(b) The unload is a download-unload, not a device-unload.** CP §3.5.4 has
seven steps; the code implements 01-06 and deliberately omits step 07:

> CP §3.5.4, p. 57: `07 Unload IndividualAddress | Set
> SerialNumber_IndividualAddress_Write(FFFFh) via broadcast — Check if
> SerialNumber_InidividualAddress_Read() == FFFFh (via broadcast)`

The omission is right and stays (see "Explicit non-tasks"). What is wrong is
that `ProcedureKind::Unload` and the report say "unload" unqualified, so an
operator reads "unload complete" for a device that still holds its address.
Fix: qualify the reported name.

**(c) The escalation's justifying comment misquotes the project's own spec.**
`download.rs:551-555` unloads `parts[position..]` — the failed part included —
justified at `:548-550` by *"§7.6 says only an unload frees that"*. The cited
§7.6 (`docs/superpowers/specs/2026-09-13-commissioning-download-design.md:
1271-1279`) gives **two** mechanisms, re-allocation *and* unload. The Standard
agrees with the spec, not with the comment:

> CP §3.5.1.2, p. 40: *"If a Management Client allocates the same segment memory
> more than once, the device shall free the previously allocated memory and
> attempt to reallocate the requested memory size at the original base
> address."*

> RES §4.23.2.3.3, Table 94, p. 296: row `Start Loading (01h)`, column
> `Loading (02h)` → `Loading`.

CP §3.5.3 AP2 Nr. 07 (p. 46) lists AP1, GOT, Address Table and Association Table
— not AP2 — and Nr. 08 (p. 46) goes straight back to `Loading`. **Ruling: keep
the extra unload, fix the comment.** The behaviour costs one Unload frame per
escalation and loses the *"at the original base address"* guarantee, which is
irrelevant under ascending relative allocation and relevant only if a device
arranges its segments otherwise — which CP §3.5.1.3, p. 40 explicitly permits.
Cost if wrong: a redundant frame on a path that is already the slow one. The
defect being fixed is the misquotation.

**Acceptance:** no behavioural change; each of the three strings names an
authority that says what the code does.

### C7 — an errata note in `docs/`

**Severity: documentation.** The Standard contains printed text that must not be
followed literally, and one of this project's own gap entries is overstated.
Both belong in `docs/KNOWN_LIMITATIONS.md` (and the errata list may warrant its
own section in `docs/COMPATIBILITY.md`).

**CP §3.5.3 errata:**

1. **The three table variants read AP2's MCB.** The GOT (pp. 51, 52), Group
   Address Table (pp. 54, 55) and Association Table (p. 56) variants all print
   `PropertyRead(ID_ApplicationProgram_2, PID_MCB)` while every surrounding row
   addresses their own object. The code reads the loaded part's own MCB
   (`load_one_part:788`, `:817`) — the sensible reading, and an undocumented
   divergence from the printed text until this note exists.
2. **AP2 Nr. 11** ("Loading the Address Table", p. 47) says *"Get Group Object
   Table base pointer"* and *"Set Group Object Table to the LoadState 'Loaded'"*
   inside the Address Table row. Same in AP1 Nr. 10 (p. 50) and GOT Nr. 09
   (p. 52).
3. **Group Address Table Nr. 05** (p. 53) puts a `LoadComplete` inside the
   unload wait; `LoadComplete` from `Unloaded` is `R: Unloaded / O: Error` per
   RES Table 94.
4. **§3.5.2 and §3.5.3 disagree** on the table version writes — see C1.
5. **CP §3.5.4 step 05's cross-reference is dangling:** *"refer to the routines
   of 'Unload Device' in 3.5.1.3"*, but CP §3.5.1.3 is titled *"Memory
   architecture"*, and the string `Unload Device` occurs exactly once in the
   whole document — inside that reference.
6. **RES §4.23.2.4.1, p. 297** says *"continue with further access only after
   load state has changed to LoadCompleted"*, but Table 92 names the state
   `Loaded`; `LoadCompleting` is the intermediate state and `LoadCompleted` is a
   Load Control **value** (02h), not a state. The code assumes `Loaded`.
7. **Three different segment orders.** CP §3.5.2 step 05 unloads AP2, AP1, GOT,
   Association Table, Address Table; §3.5.4 step 05 unloads Address Table,
   Association Table, Object Table, AP2, AP1; §3.5.2's *load* order 06-10 is
   AP2, AP1, GOT, Address Table, Association Table. The code iterates
   `plan.parts` in one order for all three and cannot match all three clauses.
   Freeing memory is order-independent, so this is a trace-fidelity note, not a
   defect — and explicitly not something to "fix".

**Also in C7: correct `GAP-T30-04`.** It claims the differential download
algorithm *"occurs only at CP §3.5.3 as a name"*. It occurs in four more places,
three of them substantive:

> `03_01_02 Glossary v01.05.03 AS.pdf`, p. 9: *"**Differential Download** —
> Optimisation of the Configuration Procedure in S-Mode, in which only the data
> is downloaded that is assumed to differ between the current contents and the
> intended contents after download. NOTE 3 To this purpose, the Management
> Client may for instance hold a memory image of a preceding download, which it
> compares with a new memory image (new parameters, links…) to decide on which
> data to write in the device."*

> RES §4.2.27.1.2, p. 39: *"The CRC checksum calculation shall provide a higher
> consistency while using differential download."*

> `Project Schema23 v01.00.00.pdf`, `DeviceInstance` attributes, p. 44:
> `LoadedImage | xs:base64Binary | optional | The image loaded into the device
> the last time (used with differential download)`, `CheckSums |
> xs:base64Binary | optional | Check sums read from the device the last time
> (used with differential download)`, and `DownloadCounter | xs:unsignedInt |
> optional`.

The trigger, the goal, the required client-side state and the consistency
argument are all documented. What is genuinely absent is the diffing and
chunk-selection strategy — and the Glossary's *"may for instance"* says outright
that it is implementation-defined. That is a licence, not a gap. Rewrite
`GAP-T30-04` to say so, and keep `GAP-T30-07` (the unquantified programming
delay) open, which audit 2 confirmed: `delay for programming the memory`,
`programming delay` and `memory programming time` all return `[]` in both bases,
and MP §3.16.2, p. 101, footnote 12 says only *"The delay time depends on the
Management Server and on the amount of written octets (see [08])."*

**Also in C7:** `simulator.rs:626-635` maps both `Unloaded`-state events to
`Error` while its comment at `:611-613` claims these are *"the recommended
transition for each event"*. RES Table 94, p. 296 marks that one **optional**
(`R: Unloaded / O: Error`; legend *"R: recommended … O: optional"*). Harmless —
the suite simply exercises one of two legal device behaviours — but the comment
misdescribes which. Fix the comment; leave the behaviour.

---

## Tier 2

### C8 — pin the segment order — *correctness* — depends on C1

The memory *layout* is only a recommendation:

> CP §3.5.1.3, p. 40: *"The memory segments … are recommended to be arranged in
> the device memory in ascending order as described in Figure 5. … Nevertheless
> it shall be possible to arrange the segments in different ways."*

The *download* order is normative but carried entirely by row position — CP
§3.5.2 Nr. 06-10 and §3.5.3 AP2 Nr. 08-12 both give AP2 → AP1 → GOT → Address
Table → Association Table, and no free-standing sentence states it.

`DownloadPlan::new` (`download.rs:133-153`) validates only non-emptiness and
index uniqueness; the doc comment at `:128-132` asserts the order and nothing
enforces it. The escalation slice `parts[position..]` takes its entire target
set from caller-supplied order, so a caller handing the parts over in another
order gets a silently wrong escalation set: no error, a plausible report, a
wrong device.

**Do:** validate the order in `DownloadPlan::new` against `PartKind` (C1's
type), rejecting anything else with a named error.

**Acceptance:** a constructor test per permutation class; the escalation test
from `download.rs:1165-1203` still green.

### C9 — compare the CRC on the escalated reload of the target part — *completeness* — depends on C2

> CP §3.5.3, AP2 variant Nr. 08, p. 46: *"Compare CRC checksum: MaC:
> PropertyRead(ID_ApplicationProgram_2, PID_MCB) … If the CRC matches, then MaC
> shall use differential download algorithm."*

Nr. 08 carries the full comparison block, identical to Nr. 06. The *following*
segments, Nr. 09-12, do not. `download.rs:556-566` passes `compare_crc = false`
for the whole `parts[position..]` slice, the target included, so
`PartOutcome.crc` reads `NotCompared` for the very part that triggered the
escalation — right for the parts after it, wrong for it.

**Acceptance:** a test that an escalated partial download reports a real
`CrcComparison` for the target part and `NotCompared` for the rest.

### C10 — make step 11 / step 13 honest about access keys — *completeness*

The same step appears under two numbers: CP §3.5.2 Nr. 11 (`download.rs:490-495`)
and CP §3.5.3 AP2 Nr. 13 (`download.rs:576-586`). Both call sites change
together.

> CP §3.5.2, p. 44: `11 Modifying access keys | Set access keys as required`

> AL §3.5.8, p. 129: *"The A_Key_Write.req primitive shall be applied by the
> user of Application Layer, to modify or delete the key associated to a certain
> access level in the communication partner."* and *"The current access level
> shall be less or equal to the access level indicated in the A_Key_Write.ind
> primitive, otherwise the remote application process shall return FFh in the
> A_Key_Response-PDU."*

> MP §3.6.1 `DM_SetKey_RCo`, p. 77: *"If the level returned in A_Key_Response is
> not the same as in the A_Key_Write, the operation was not successful. Possibly
> an authorization is required."*

*"as required"* makes an empty step conforming **when the plan requires no
keys**. The defect is that `DownloadPlan` has no field in which keys could be
required, so the report reads identically whether none were needed or some were
silently skipped. A download that leaves a device on the installer's default key
without saying so is a security-relevant silent difference, not a cosmetic one.

**Do:** an explicit access-key declaration on `DownloadPlan` with a
"none required" variant; the report distinguishes *declared none* from *not
implemented*. **This does not require implementing `A_Key_Write`** — a plan that
declares keys is refused with a named error until that encoder exists
(`cemi.rs:2167` currently asserts the APCI constant exists *without* an
encoder).

**Acceptance:** both call sites report the same declaration; a plan declaring
keys is refused, not silently completed.

---

## Tier 3 — the five variants

### C11 — model five step lists, not one — *traceability* — depends on C1, benefits from C8

CP §3.5.3 has **five** partial-download variants and each numbers its own steps.
The unload is Nr. 05 in all five; the escalation is Nr. 07 in four and does not
exist in the fifth; the tails are 13/14, 12/13, 11/12, 10/11 and **07/08**:

| Variant | Steps | Escalation (Nr. 07) targets | Jump target |
|---|---|---|---|
| Application Program 2 | 01-14 | AP1, GOT, Address Table, Association Table | ⇒ 13 |
| Application Program 1 | 01-13 | GOT, Address Table, Association Table | ⇒ 12 |
| Group Object Table | 01-12 | Address Table, Association Table | ⇒ 11 |
| Group Address Table | 01-11 | Association Table | ⇒ 10 |
| Association Table | 01-08 | — (Nr. 07 is *Modifying access keys*) | none |

> CP §3.5.3, Association Table variant, p. 56: `07 Modifying access keys | Set
> access keys as required` — `08 Disconnect | Disconnect via bus`

`procedure.rs:393-396` models only *"CP §3.5.3 'application program 2', steps
01-14"*.

**Do:** the five step lists as pure data in `knx-core`, each carrying its clause
and page. No sequencer changes in this task.

**Acceptance:** a table-driven test asserting all five lists against the table
above, including the Association Table variant's absent escalation.

### C12 — the sequencer chooses the right list — *traceability + correctness* — depends on C11

`partial_download` (`download.rs:514-588`) emits one numbering for every
`object_index`: literal 5 at `:526`, 6 at `:531`, 7 at `:546`, 13 at `:579`, 14
at `:585`. A partial download of the Association Table therefore emits
`13 modify access keys` / `14 disconnect` against a clause whose last step is
08, and `07 on failed allocation, escalate` against a Nr. 07 that in that clause
means *modifying access keys*. Four of five traces cite numbers the Standard
does not use for that variant; one cites a number that means something else.

Two further defects belong here:

**(a) The escalated reloads 09-12 never appear in the trace.** AP2 Nr. 08-12 are
five separate numbered rows (pp. 46-47). `download.rs:546-568` records outer
step 7 and then nothing until 13; the reloads go under
`ProcedureKind::LoadOnePart`, and the test at `:1183-1187` pins
`outer_step_numbers() == vec![1, 3, 2, 4, 5, 6, 7, 13, 14]`. The module header
justifies only the Nr. 08 merge on the happy path; on the escalation path Nr. 08
*and* 09-12 genuinely happen and none are recorded — against the file's own
rule, stated twice (`:490-492`, `:574-578`): *"a step silently missing from a
trace is indistinguishable from a step that was forgotten."*

**(b) The last segment has no escalation branch, and the code invents one.**

> CP §3.5.3, Association Table variant Nr. 06, p. 56: *"Base Address is a 4
> octet absolute value; if it is zero then allocation was not successful. This
> causes an error message of the MaC to the Installer."*

No `⇒ Continue at Nr. 7`. Failure is terminal and must be reported.
`download.rs:545-568` takes the escalation branch at any position; at the last
position `parts[position..]` is a one-element slice, so the code unloads,
retries the identical allocation, fails again and propagates — a wasted round
trip, `escalated_from` set on a variant that has no escalation, and a trace line
citing a Nr. 07 that means something else.

**Acceptance:** a trace test per variant asserting the exact outer step numbers;
a test that the last part's allocation failure is terminal with
`escalated_from == None`; the escalated reloads appear as numbered outer steps.

---

## Tier 4 — state the Standard expects a client to keep

### C13 — read and report the Download Counter — *completeness*

CP §3.5.3 imposes nothing, but the obligation is real and `grep -rn
"download_counter\|DOWNLOAD_COUNTER" crates/` returns nothing today.

> CP §3.12.4, p. 99: *"The MaC shall store the read value of the Download
> Counter in its repository, so that it can use it later for possible acceptance
> of partial download."*

> CP §3.12.5, p. 100: *"If the read value of the Download Counter differs from
> the value that the MaC stored after the preceding configuration, then the MaC
> shall not continue with a partial download, but instead perform a complete
> download as specified in 3.12.4."*

Those two are Coupler Model 2.0. For System B the client duty is advisory:

> RES §4.2.30.3, p. 42: *"The Management Client should firstly read
> PID_DOWNLOAD_COUNTER in the Device Object. If this PID_DOWNLOAD_COUNTER has
> not changed, then it may conclude that no further instance of
> PID_DOWNLOAD_COUNTER in the Management Server has changed value."*

> CP §1.3.1.2, p. 9: *"NOTE 1 Special attention needs to be paid to the Download
> Counter: please refer to PID_DOWNLOAD_COUNTER in the Device Object ([05])."*

**Do:** read it where present, carry it in the report, and — for Coupler Model
2.0 plans — refuse a partial download when it differs from the stored value.
Open: whether a System B device must have the property at all. RES §4.2.30.1,
p. 41 is conditional and defers to Volume 6 Profiles Annex A (see C18).

### C14 — do not drop ETS's differential-download state on import — *data integrity*

> `Project Schema23 v01.00.00.pdf`, p. 44: `LoadedImage`, `CheckSums`,
> `DownloadCounter` on `DeviceInstance`, all optional.

`grep -rn "LoadedImage\|CheckSums\|DownloadCounter" crates/` returns nothing. An
imported ETS project carrying these three attributes loses them silently, which
CLAUDE.md's data-integrity rule forbids outright: *"Never silently discard
information."*

**Do:** preserve all three through import, and report them in the import's
unsupported/preserved section. `LoadedImage` is opaque base64 and stays opaque —
preserving it is not the same as understanding it.

**Acceptance:** a round-trip test over a fixture carrying all three.

---

## Tier 5 — new capability, and the only tasks that could ever touch hardware

### C15 — the `A_Restart` sender — *completeness* — depends on C3

> MP §2.3 "Use", p. 13: *"The procedure shall check if the programming is
> successful and shall deactivate the Programming Mode by executing a restart of
> the device."*

MP §2.3 step 4, p. 15: A_Connect → A_DeviceDescriptor_Read(00h) → response →
*"A_Restart-PDU"* → *"Abort the connection of the client side Transport Layer."*

`ApplicationService::Restart { response, restart_type, data }` exists in
`cemi.rs` with reserved-bit validation and `InvalidRestartType` for type > 1;
`WriteScope::Restart` exists in `mutation.rs`; the only non-test reference to
either is a test at `commissioning.rs:2641`. No session method sends one, so a
device left in Programming Mode after a failed write stays there.

Carry these timings, all from MP §3.7:

| Obligation | Quantity | Source |
|---|---|---|
| MaC may first expect a Basic Restart to have succeeded | t1 = 1 s | MP §3.7.1.1.2, p. 79 |
| MaS responsive again after a Basic Restart | t2 = 5 s | MP §3.7.1.1.2, p. 79 |
| Master Reset Process Time, device-reported, *"a minimal time for the MaC to wait, not a maximal time"* | ≥ 5 s, DPT 7.005 | MP §3.7.1.2.2, pp. 80-81 |
| After the Process Time expires, call the failed service **one last time** before declaring the Configuration Procedure failed | 1 extra attempt | MP §3.7.1.2.2, p. 81 |
| After a restart, issue T_Disconnect regardless, wait 6 s for a possible T_Disconnect, and **do not continue configuration before that time-out elapses** | 6 s | MP §3.7.3 exception (5), p. 90 |

`disconnect()` is best-effort with no wait today; the last row changes that for
the post-restart path only.

This also makes `set_programming_mode` (`commissioning.rs:1817`) reachable — it
is implemented, correct against MP §3.13.2, pp. 94-95, and currently called only
from tests. The implementation needs no change: *"The Programming Mode shall be
realised as 'Programming Mode – Realisation Type 2'"*, A_Memory_Read(60h,1) then
A_Memory_Write(60h,1), *"In the data (DD) bit 0 has to be set according to the
mode. The parity (bit 7) has to be calculated."* — and `prog_mode_write(old,
enable)` does `old XOR 10000001b`, writing only when the bit actually changes.

**Hardware note:** programming mode on/off is the agreed *first* operation
against a real device, and it is fully reversible and visible on the device LED.
It still requires an explicit go-ahead naming the device and the operation. This
task implements and tests it against the simulator only.

### C16 — `NM_IndividualAddress_Write` end to end — *completeness* — depends on C15

Nothing in `crates/` implements the write path. What exists is the declarative
4-step list (`procedure.rs:195`), the codec
(`ApplicationService::IndividualAddressWrite` / `IndividualAddressRead` in
`cemi.rs`), and `WriteScope::IndividualAddressProgramming`, whose only non-test
reference is `procedure.rs:135`. `ManagementSession` has no method that writes an
individual address. Build it against C3's corrected step descriptions.

### C17 — do not advertise a procedure that cannot run — *completeness*

`Procedure::for_kind` offers `IndividualAddressWrite` while no execution path
exists, so a UI enumerating procedures offers one that fails on use. Either gate
it behind a capability flag or mark it declaratively unavailable. Obsolete once
C16 lands; worth doing first if C16 slips.

---

## Research

### C18 — audit `06 Profiles v02.01.01.pdf` for System B

Three open questions all resolve in Volume 6 Profiles, Annex A, which neither
audit opened:

1. Whether a System B device must implement `PID_DOWNLOAD_COUNTER` at all (RES
   §4.2.30.1, p. 41 is conditional and defers there) — blocks part of C13.
2. Why CP §3.5.3's table variants write `PID_PROGRAM_VERSION` to objects RES
   Tables 77/80/85 do not define it on: either the CP rows are errata (likely,
   given C7's pattern) or RES's per-object tables are non-exhaustive and
   Volume 6 permits it — settles C1's tolerant branch one way or the other.
3. Whether any step in CP §3.5.2 or §3.5.4 is mandatory for System B but absent
   here. `System B mandatory` returns `[]` in both knowledge bases, and neither
   clause marks a step optional or profile-conditional except §3.5.2 step 09's
   PL110 footnote.

Also unchecked, low risk: `03_05_01` repeats the `PID_DOWNLOAD_COUNTER` heading
on p. 320 and it was never diffed against the p. 41 definition.

---

## Dependency order

```text
C1 ∥ C2 ∥ C3 ∥ C4 ∥ C5 ∥ C6 ∥ C7        (tier 1, all independent)
        ↓          ↓
      C8 ∥ C9 ∥ C10                      (C8←C1, C9←C2, C10 independent)
        ↓
      C11 → C12                          (five variants, five numberings)
        ↓
   C13 ∥ C14                             (counter and import state)
        ↓
   C15 → C16, with C17 as C16's stopgap
   C18 runs at any time and unblocks part of C13 and all of C1's open question
```

C1 first, always: it is the only entry here that fails on real hardware, and its
simulator half is what makes every later test mean anything.
