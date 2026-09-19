# Spec audit 1 — CP §3.5.3, the partial download, all five variants

Read-only audit, 2026-09-19, opus, reading
`03_05_03 Configuration Procedures v02.01.01 AS.pdf` and
`03_05_01 Resources v01.10.01 AS.pdf` directly. Transcribed by the controller
because subagents cannot write files.

## Corrections to the brief

**C1. The page offset is 0, not +1.** `pdftotext -f 38` lands on the page whose
footer reads "page 38 of 198". Document page N = PDF page N throughout
`03_05_03`. `03_05_01` also has offset 0, but its footers misprint as
"page N of N+1" — an editorial quirk, not an offset.

**C2. The Download Counter NOTE is not in the §3 area.** It is in §1.3.1.2
"Manual Reset", document page 9, and concerns `Reset to default state`, not
the partial download:

> CP §1.3.1.2, p. 9: *"NOTE 1 Special attention needs to be paid to the
> Download Counter: please refer to PID_DOWNLOAD_COUNTER in the Device Object
> ([05])."*

§3.5.3 never mentions the Download Counter. See Gap 8.

The five-variant table in the brief was **correct in every row**: AP2 01-14,
AP1 01-13, GOT 01-12, Group Address Table 01-11, Association Table 01-08, with
the Nr. 07 escalation lists and the Nr. 06 jump targets as stated. The
Association Table variant's Nr. 07 is "Modifying access keys", so it has no
escalation step at all.

## Gaps

### Gap 1 — one trace numbering for five different step lists — *traceability*

Each variant numbers its own steps. The unload is Nr. 05 in all five; the
escalation is Nr. 07 in four and does not exist in the fifth; the tail is
13/14, 12/13, 11/12, 10/11 and **07/08**.

> CP §3.5.3, Association Table variant, p. 56: `07 Modifying access keys |
> Set access keys as required` — `08 Disconnect | Disconnect via bus`

`download.rs:514-588` has one generic `partial_download(object_index)` with
the step numbers as literals — 5 at :526, 6 at :531, 7 at :546, 13 at :579,
14 at :585 — for every `object_index`. `procedure.rs:393-396` models only
"CP §3.5.3 'application program 2', steps 01-14".

A partial download of the Association Table therefore emits `13 modify access
keys` / `14 disconnect` against a clause whose last step is 08, and
`07 on failed allocation, escalate` against a Nr. 07 that in that clause means
*modifying access keys*. Four of five traces cite numbers the Standard does
not use for that variant; one cites a number that means something else.

### Gap 2 — the escalated reloads 09-12 never appear in the trace — *traceability*

AP2 Nr. 08-12 are five separate numbered rows (pp. 46-47). `download.rs:546-568`
records outer step 7 and then nothing until 13; the reloads go under
`ProcedureKind::LoadOnePart`. The test at `download.rs:1183-1187` pins
`outer_step_numbers() == vec![1, 3, 2, 4, 5, 6, 7, 13, 14]`.

The module header justifies only the Nr. 08 merge on the happy path. On the
escalation path Nr. 08 *and* 09-12 genuinely happen and none are recorded —
against the file's own rule, stated twice (`:490-492`, `:574-578`): *"a step
silently missing from a trace is indistinguishable from a step that was
forgotten."*

### Gap 3 — the escalation unloads the failed part, and the stated reason is wrong — *correctness (minor) + documentation*

Nr. 07 (p. 46) lists AP1, GOT, Address Table, Association Table — not AP2.
Nr. 08 then goes straight back to Loading:

> CP §3.5.3, AP2 variant Nr. 08, p. 46: *"Set Application Program 2 to the
> LoadState 'Loading' — MaC: Set ApplicationProgram_2.LoadControl = Load —
> MaS: Set ApplicationProgram_2.LoadState = Loading"*

That works from the state a failed allocation leaves, and two clauses say so:

> RES §4.23.2.3.3, Table 94, p. 296: row `Start Loading (01h)`, column
> `Loading (02h)` → `Loading`.

> CP §3.5.1.2, p. 40: *"If a Management Client allocates the same segment
> memory more than once, the device shall free the previously allocated memory
> and attempt to reallocate the requested memory size at the original base
> address."*

`download.rs:551-555` unloads `parts[position..]`, the failed part included,
justified at `:548-550` by *"§7.6 says only an unload frees that"*. **The cited
§7.6 says the opposite** — the design spec at
`docs/superpowers/specs/2026-09-13-commissioning-download-design.md:1271-1279`
gives two mechanisms, re-allocation *and* unload, and the comment names one.

Cost: one extra Unload frame per escalation, and the re-allocation loses the
"at the original base address" guarantee — irrelevant under ascending relative
allocation, relevant if a device arranges segments otherwise, which §3.5.1.3
p. 40 explicitly permits. The defect is the misquotation, not the frame.

### Gap 4 — the escalated reload of the target part skips the CRC comparison — *completeness*

Nr. 08 carries the full "Compare CRC checksum" block, identical to Nr. 06:

> CP §3.5.3, AP2 variant Nr. 08, p. 46: *"Compare CRC checksum: MaC:
> PropertyRead(ID_ApplicationProgram_2, PID_MCB) … If the CRC matches, then
> MaC shall use differential download algorithm."*

The *following* segments (Nr. 09-12) have no such block. `download.rs:556-566`
passes `compare_crc = false` for the whole `parts[position..]` slice, target
included; the first attempt passes `true` at `:538`. So `PartOutcome.crc`
reads `NotCompared` for the part that triggered the escalation — correct for
the parts after it, wrong for it.

### Gap 5 — `PID_PROGRAM_VERSION` written to three objects RES does not give it — *correctness, highest value*

**(a) The three table variants do prescribe the write.**

> CP §3.5.3, Group Address Table variant Nr. 06, p. 54: *"Set
> GroupAddressTable.ApplicationVersion — MaC: PropertyWrite(ID_
> GroupAddressTable, PID_PROGRAM_VERSION)"*

> CP §3.5.3, Association Table variant Nr. 06, p. 56: *"Set
> AssociationTable.ApplicationVersion — MaC: PropertyWrite(ID_
> AssociationTable, PID_PROGRAM_VERSION)"*

The GOT variant likewise (pp. 51-52). The **complete** download §3.5.2 does
*not* list these writes for its table steps 08/09/10 (pp. 43-44). The two
clauses disagree.

**(b) RES gives none of those three objects the property.** Table 77 (Group
Address Table, Realisation Type 7, "Used by: System B", p. 238) lists
`PID_OBJECT_TYPE`, `PID_LOAD_STATE_CONTROL`, `PID_TABLE_REFERENCE`,
`PID_TABLE`, `PID_MCB_TABLE`, `PID_ERROR_CODE`, `PID_GROUP_RESPONSER_TABLE`
— no `PID_PROGRAM_VERSION`. Same for Table 80 (Association Table, p. 249) and
Table 85 (Group Object Table, p. 270). Only Tables 90 and 91 (Application
Program 1 and 2, pp. 288, 290) carry `13 = PID_PROGRAM_VERSION`.

`download.rs:801-809` writes it unconditionally in `load_one_part`, for all
five parts and all four procedures. On real hardware the write to a table
object targets a property RES says is not there: `write_property` →
`PropertyRefused` (`commissioning.rs:1570`), aborting **after** the data is
written and **before** `LoadCompleted`, leaving the segment stuck in `Loading`.

The tests do not catch it because the simulator stores any
`(object_index, property_id)` pair (`simulator.rs:606`), making it strictly
more permissive than RES Tables 77/80/85.

### Gap 6 — the MCB layout is documented; the code says it is not, and compares all 8 octets — *correctness + a false "undocumented" claim*

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
> CRC16-CCITT specification."* — parameters on p. 40: width 16, polynomial
> `1021h`, initial value `FFFFh`, input not reflected, output not reflected, no
> output XOR.

`commissioning.rs:1366-1376` states the layout *"is not specified in either
knowledge base (spec §12), so this crate carries it and does not pretend to
parse it"*. That is false. `simulator.rs:57-60` repeats it.
`download.rs:789-793` compares `*stored == current` over all 8 octets.

Three consequences:
1. **Segment Size is octets 0-3.** A partial download allocates before
   comparing (`load_one_part:779-784` precedes the MCB read at `:788`), so any
   change in payload length changes octets 0-3 and the comparison reports
   `Differed` for reasons unrelated to the CRC. The clause says to compare the
   CRC — two octets.
2. The read/write access nibbles (octet 5) pollute the comparison too.
3. **CRC Control Byte bit 0 is ignored.** When it is 1 the device is saying the
   memory may have been changed by the application since the load, so a
   matching CRC proves nothing. Nothing reads that bit.

### Gap 7 — `PID_GROUP_RESPONSER_TABLE` is PL110-only — *out of scope, correctly*

> CP §3.5.3, footnote 8, p. 47: *"Applicable for PL110 devices only."*

It attaches to the AP2 variant's Nr. 11. The AP1 variant carries the identical
sentence as footnote 9 (p. 50), the GOT variant as footnote 10 (p. 52), §3.5.2
step 09 as footnote 7 (p. 44).

> RES §4.16.8.2.5 `PID_GROUP_RESPONSER_TABLE (PID = 53)`, p. 239: *"This
> Property is mandatory for PL110 devices. For all other media this Property
> shall not be implemented."*

Cost of omitting it on TP1/RF/IP: zero. `procedure.rs:299` already records the
PL110 scoping. **No task** — except a refusal: if a plan ever targets a PL110
device, decline rather than silently omit Nr. 11's second half.

### Gap 8 — the Download Counter — *completeness, forward-looking*

§3.5.3 imposes nothing. The obligation is real for Coupler Model 2.0:

> CP §3.12.4, p. 99: *"The MaC shall store the read value of the Download
> Counter in its repository, so that it can use it later for possible
> acceptance of partial download."*

> CP §3.12.5, p. 100: *"If the read value of the Download Counter differs from
> the value that the MaC stored after the preceding configuration, then the MaC
> shall not continue with a partial download, but instead perform a complete
> download as specified in 3.12.4."*

For System B the client duty is advisory:

> RES §4.2.30.3, p. 42: *"The Management Client should firstly read
> PID_DOWNLOAD_COUNTER in the Device Object. If this PID_DOWNLOAD_COUNTER has
> not changed, then it may conclude that no further instance of
> PID_DOWNLOAD_COUNTER in the Management Server has changed value."*

`grep -rn "download_counter\|DOWNLOAD_COUNTER" crates/` returns nothing.

### Gap 9 — the segment order is pinned nowhere — *correctness*

The memory layout is only recommended:

> CP §3.5.1.3, p. 40: *"The memory segments … are recommended to be arranged in
> the device memory in ascending order as described in Figure 5. … Nevertheless
> it shall be possible to arrange the segments in different ways."*

The *download* order is normative but carried entirely by row position —
§3.5.2 Nr. 06-10 and §3.5.3 AP2 Nr. 08-12 both give AP2 → AP1 → GOT → Address
Table → Association Table. There is no free-standing sentence stating it.

`DownloadPlan::new` (`download.rs:133-153`) validates only non-emptiness and
index uniqueness. The doc comment at `:128-132` asserts the order; nothing
enforces it. The escalation slice `parts[position..]` derives its whole target
set from caller-supplied order. A caller handing the parts over in another
order gets a silently wrong escalation set: no error, a plausible report, a
wrong device.

Aside: §3.5.2's *unload* at Nr. 05 (p. 42) runs AP2, AP1, GOT, **Association
Table, Address Table** — the two tables swapped relative to the load order.
Harmless; worth not "fixing".

### Gap 10 — the last segment has no escalation branch, and the code invents one — *correctness (minor) + traceability*

> CP §3.5.3, Association Table variant Nr. 06, p. 56: *"Base Address is a 4
> octet absolute value; if it is zero then allocation was not successful. This
> causes an error message of the MaC to the Installer."*

No `⇒ Continue at Nr. 7`. Failure is terminal and must be reported.
`download.rs:545-568` takes the escalation branch for any position; at the last
position `parts[position..]` is a one-element slice, so the code unloads,
retries the identical allocation, fails again and propagates — a wasted round
trip, `escalated_from` set on a variant that has no escalation, and a trace
line citing a Nr. 07 that means "modifying access keys".

### Gap 11 — errata in §3.5.3 that must not be followed literally — *documentation*

1. **The three table variants read AP2's MCB.** GOT (pp. 51, 52), Group Address
   Table (pp. 54, 55) and Association Table (p. 56) all print
   `PropertyRead(ID_ApplicationProgram_2, PID_MCB)` while every surrounding row
   addresses their own object. The code reads the loaded part's own MCB
   (`load_one_part:788`, `:817`) — the sensible reading, and an undocumented
   divergence from the printed text.
2. **AP2 Nr. 11** ("Loading the Address Table", p. 47) says *"Get Group Object
   Table base pointer"* and *"Set Group Object Table to the LoadState
   'Loaded'"* inside the Address Table row. Same in AP1 Nr. 10 (p. 50) and GOT
   Nr. 09 (p. 52).
3. **Group Address Table Nr. 05** (p. 53) puts a `LoadComplete` inside the
   unload wait. `LoadComplete` from `Unloaded` is `R: Unloaded / O: Error` per
   Table 94.
4. §3.5.2 vs §3.5.3 disagree on the table version writes — Gap 5(a).

### Gap 12 — the simulator picks the optional Table 94 transitions — *cosmetic*

> RES §4.23.2.3.3, Table 94, p. 296: from `Unloaded`, `Load Completed (02h)` →
> `R: Unloaded / O: Error`. Legend: *"R: recommended … O: optional"*.

`simulator.rs:626-635` maps both to `Error` while the comment at `:611-613`
claims these are *"the recommended transition for each event"*. It is the
optional one. Harmless, but the suite exercises only one of two legal device
behaviours and the comment misdescribes which.

## What is already correct — write no tasks for these

- **The escalation target set** is right for every position: `parts[position..]`
  minus the target reproduces each variant's Nr. 07 list exactly.
- **Escalation rather than abort** — the hard part, right, tested
  (`download.rs:1165-1203`) and reported via `escalated_from`.
- **Step 05 unloads only the target part** (`:526-529`), matching all five
  variants and distinguishing §3.5.3 from §3.5.2's Nr. 05.
- **"Read and save CRC checksum" is performed**: `load_one_part` step 7
  (`:816-817`) reads `PID_MCB_TABLE` after `LoadCompleted` and returns it as
  `PartOutcome.mcb`; persistence is left to the caller
  (`LoadablePart::with_stored_mcb`). The consumer is the next partial
  download's Nr. 06 comparison. The ordering is right — the CRC is valid in
  `Loaded` only. Only *what* is compared is wrong (Gap 6).
- **Allocation only in `Loading`**, state checked after the write
  (`commissioning.rs:1614-1622`, `:1658-1687`), catching §3.5.1.2's
  *"In all other states the memory allocation shall be ignored"* silent no-op.
- **Base address zero is a failure** (`commissioning.rs:1360-1362`).
- **The `FFFFh` service split** decided on base + length, whole region on one
  service (`knx-core/src/commissioning/memory.rs:52-56, 192-199`, tested at
  `:411-423`), matching §3.5.1.4 Table 5.
- **The wait loop** (`commissioning.rs:1690-1758`): polls, tolerates
  `LoadCompleting` silence per Table 94's footnote, reconnects mid-transition
  per RES §4.23.2.4.1, distinguishes *never moved* from *still moving*.
- **A matching CRC does not skip the write** (`download.rs:754-761`) — Nr. 05
  already unloaded the part and RES Table 93 declares unloaded data undefined.

## Open questions

**Q1. The differential download algorithm — the gap is much smaller than
`GAP-T30-04` claims.** The design spec says the phrase *"occurs only at
CP §3.5.3 as a name"*. It occurs in four more places, three substantive:

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
> the last time (used with differential download)` and `CheckSums |
> xs:base64Binary | optional | Check sums read from the device the last time
> (used with differential download)`, alongside `DownloadCounter |
> xs:unsignedInt | optional`.

So the trigger, the goal, the required client-side state and the consistency
argument are all documented. What is genuinely absent is the diffing and
chunk-selection strategy — and the Glossary's *"may for instance"* says outright
it is implementation-defined. That is a licence, not a gap.

Searched: `"differential download algorithm"` in both bases (5 results each,
all §3.5.3); `grep -ril "differential download"` over the extracted corpus,
hitting `03_05_03` (14), `3_10_5 KNX IoT Point API` (9, unrelated),
`Project Schema23` (2), `03_05_01` (2), `03_01_02 Glossary` (1). Unresolved:
how ETS chooses chunk boundaries; `LoadedImage` is opaque base64.

**`grep -rn "LoadedImage\|CheckSums\|DownloadCounter" crates/` returns
nothing** — if KNXBench imports an ETS project carrying these three attributes
they are silently dropped, which the data-integrity rule forbids.

**Q2. Whether a System B device must have `PID_DOWNLOAD_COUNTER` at all.**
RES §4.2.30.1 (p. 41) is conditional and defers the ruling to Volume 6
Profiles, Annex A, which was not opened.

**Q3. Why §3.5.3's table variants write `PID_PROGRAM_VERSION` to objects RES
Tables 77/80/85 do not define it on.** No reconciling clause found. Two
readings survive: the CP rows are errata (likely, given Gap 11's pattern), or
RES's per-object tables are non-exhaustive and Volume 6 Annex A permits it.
The same Volume 6 Annex A read closes Q2 and Q3. Until then Gap 5's fix should
be *tolerant*: write on AP1/AP2, and on the three tables treat a refusal as
expected rather than fatal.

**Q4.** `03_05_01` repeats the `PID_DOWNLOAD_COUNTER` heading on p. 320; not
diffed against the p. 41 definition. Low risk, unchecked.

## Recommended task split (the auditor's, not yet ruled on)

Tier 1, independent: **T-A** per-part-kind property writes (Gap 5) — introduce
a `PartKind`, write `PID_PROGRAM_VERSION` only for the two application
programs, and **teach the simulator to refuse properties RES does not list**,
without which the rest proves nothing. **T-C** parse the MCB per Table 12 and
compare the CRC alone, with a fourth outcome for CRC Control Byte bit 0, and
strike the false "undocumented" claims (Gap 6). **T-F** fix or revert the
escalation's justification (Gap 3). **T-G** an errata note in `docs/` (Gaps 11,
12, and the `GAP-T30-04` correction).

Tier 2: **T-B** pin the segment order (Gap 9, depends on T-A). **T-E** CRC
comparison on the escalated reload (Gap 4, depends on T-C).

Tier 3: **T-D** five variants, five numberings (Gaps 1, 2, 10) — split into
T-D1 (model the five step lists in `knx-core`, pure data) and T-D2 (make the
sequencer choose among them). Depends on T-A, benefits from T-B.

Tier 4: **T-H** Download Counter and differential-download state (Gap 8, Q1) —
H1 read and report the counter, H2 preserve `LoadedImage`, `CheckSums` and
`DownloadCounter` on ETS import.

Caveat carried by the auditor: every one of these is provable only against the
simulator, and the simulator is currently more permissive than RES on exactly
the point Gap 5 turns on.
