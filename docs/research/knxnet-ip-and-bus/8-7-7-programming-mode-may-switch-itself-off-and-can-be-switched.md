# 8.7.7 Programming mode may switch itself off, and can be switched remotely

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="877-programming-mode-may-switch-itself-off-and-can-be-switched-remotely"></a>
<!-- BEGIN preserved record-1-016 -->
#### 8.7.7 Programming mode may switch itself off, and can be switched remotely

`03_05_01` §4.26 specifies an **optional** autonomous inactivation: *"if the
Programming Mode becomes enabled, by any means, the MaS (device) shall start a
time-out timer of 4 minutes … If the timer expires, the MaS shall autonomously
and automatically disable its Programming Mode."* **[D]** Optional means a
client may neither rely on it nor assume its absence, so programming mode must
be re-verified immediately before a broadcast address write rather than once at
the start of a wizard.

`03_05_02` §3.13 `DM_ProgMode_Switch(flags, mode)` switches it remotely —
*"mode 0: switch Programming Mode off, 1: switch Programming Mode on"*,
*"flags — All bits are reserved. These shall be set to 0."* **[D]** Procedure
`DMP_ProgModeSwitch_RCo` implements it as a read-modify-write of one octet at
memory address `60h` (*"the state of the Programming Mode is located at memory
address 60h"*), where *"bit 0 has to be set according to the mode. The parity
(bit 7) has to be calculated."*

**Correction, 2026-09-14.** This section previously ended "The parity
**computation** is not specified beyond that sentence … Undocumented." That was
wrong, and it was wrong while citing the clause that contradicts it. `03_05_01`
§4.26.3.1 specifies the rule, and it is a rule about *change*, not about
absolute value: **[D]** *"if the value of prog_mode is changed from "0" to "1" or
from "1" to "0" then the variable p_parity shall be inverted."* §4.26.3.4.1
repeats it as a client obligation: **[D]** *"The variable p_parity shall be
inverted, if the value of prog_mode is changed."* Location is prose as well as
figure: **[D]** §4.26.3.2 *"The location of curr_prog_mode shall be the memory
address 0060h."*

So the whole octet is `old XOR 0b1000_0001` — invert bit 0, invert bit 7, carry
bits 1 to 6 through untouched, which matters because §4.26.3.1 calls them
*"the shared bits"* of a Resource that *"may share its storage location with
other data on the same memory location inside the device"*, and adds that
*"These other data may be different depending of the mask version of the
device."* (The earlier revision of this paragraph rendered that as
*"can be shared with other functionality"*, which is not a sentence RES
§4.26.3.1 contains; corrected in fix round 3 against `pdftotext -layout` of the
PDF.) Inverting rather than recomputing
means the odd-versus-even convention never has to be known, so the one residual
unknown is not on the critical path. The write must be skipped entirely when
bit 0 already matches the requested mode — §4.26.3.4.2 and §4.26.3.4.3 guard the
procedure with `if prog_mode = 0` / `if prog_mode = 1`, and a no-op toggle would
flip the parity against an unchanged `prog_mode`, whose reaction is **[D]**
§4.26.3.3 *"manufacturer specific"* (footnote 96: *"Typically the system is
restarted if p_parity is invalid."*).

**Second correction, 2026-09-14 (fix round 3, narrowed in round 4): none of the
above is *established* for System B, and it is not excluded either.** `03_05_01` §4.26.3 carries a scoping header between its title and
§4.26.3.1, and the previous revision used the clause without reading it:
**[D]** *"Used by: − Ctrl-Mode fixed DMA − Ctrl-Mode reloc DMA − masks 0012h
0020h, 0021h, 0701h in E-Mode"*. System B is absent — and so is every mask in the
`07B0h`/`17B0h`/`57B0h` family the download spec targets. Absence from that list
is weak evidence on its own, because the list is not exhaustive: `06 Profiles`
§5.5.1.1 profiles S-Mode **couplers** onto *"§4.26.3 “Programming Mode –
Realisation Type 2”"* and couplers are not in the list either. The decisive
evidence is positive and points the other way, in `06 Profiles` §4.4.1.1
(Device Individualisation → Programming Mode → connection oriented), which splits
into two lettered alternatives each naming its profiles: **[D]**
*"a) Realisation Type 1 - Property based • System B • Mask 57B0h"* and
**[D]** *"b) Realisation Type 2 – Memory mapped • System 1 • System 2 • BCU 1
• BCU 2 • BIM M112"*. Both columns covering the target — `System B` and
`Mask 57B0h` — are Realisation Type 1, i.e. `PID_PROGMODE` (`03_05_01` §4.3.5).

That assignment reaches exactly as far as §4.4.1.1's own title, *"connection
oriented"*, which round 4 added after a review caught the over-correction. The
clause next door goes the other way: **[D]** `06 Profiles` §4.4.1.2 *"Programming
Mode – connectionless"* lists, under *"Programming Mode Control"* — where, unlike §4.4.1.1 a) and b), no
*"• via bus:"* sub-bullet appears; the only bullet is *"• via HMI: device selection
and indication of Programming Mode"*, and the two service references follow it
directly — both *"§3.13.2 “DMP_ProgModeSwitch_RCo”"* and *"§4.26.3 “Programming
Mode – Realisation Type 2”"*. The second is reached twice over: **[D]** MP §3.13.2
itself says *"The Programming Mode shall be realised as “Programming Mode –
Realisation Type 2” as specified in [05]."*, which needs no layout interpretation
at all. And §4.4's feature table gives row *"1.b Connectionless"* the value `O`
in the `System B` and `Mask 57B0h` columns **[D]** (`-` in the System 1, System 2,
BCU 1, BCU 2, BIM M112 and Mask 5705h columns; `O` for System 300 and `M` for RF
Bidirectional). So a System B device may optionally implement the connectionless
path, and that path is profiled onto the very clause that describes `0060h`.

Consequences, recorded so no later pass re-derives them: the `60h` read-modify-write
above is correct *for the profiles its source clauses name*; for a System B device
it is **unestablished** — not assigned on the mandatory connection-oriented path,
optionally assigned on the connectionless one — so what the octet at `0060h` holds
on such a device is **[A]** unknown, which is a gap in *interpretation*, not a
proof of absence. The design spec's phase-3 permission for
`A_Memory_Read(60h, 1)` inside `1.1.24`–`1.1.32` is unchanged, because a one-octet
read is harmless; what changed is that the result must be recorded as a raw octet
of unknown meaning rather than as programming-mode state (spec §3.4, §4.4, §15).
This is not a new gap — the phase-3 read establishes the device's own Profile
before anything interprets the octet — and the gap count stays at six. Source:
`pdftotext -layout` of `03_05_01 Resources v01.10.01 AS.pdf` §4.26.3 and of
`06 Profiles v02.01.01.pdf` §4.4 (feature table), §4.4.1.1, §4.4.1.2 and §5.5.1.1.

The design spec still forbids writing to `60h` on real hardware — but now as an
explicit project risk decision (spec §13 **R11**, restated as "writing to `60h`
at all on a real device"), not as a consequence of not knowing how. Knowing how
removed an accidental protection, so the refusal had to be made deliberate:
spec §4.4 keeps the write inside the mutation API, §15 keeps it a non-goal in
every phase, and phase 2 exercises it against the simulator only. Source:
`pdftotext -layout` of `03_05_01 Resources v01.10.01 AS.pdf`, §4.26.3.1 to
§4.26.3.4.3.


<!-- END preserved record-1-016 -->
