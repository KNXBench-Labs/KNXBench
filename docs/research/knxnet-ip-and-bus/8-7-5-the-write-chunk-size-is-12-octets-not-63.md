# 8.7.5 The write chunk size is 12 octets, not 63

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="875-the-write-chunk-size-is-12-octets-not-63"></a>
<!-- BEGIN preserved record-1-014 -->
#### 8.7.5 The write chunk size is 12 octets, not 63

`03_03_07` §3.5.3/§3.5.4 specify `A_Memory_Read`/`_Write` for *"between 1 and 63
octets"*. `03_05_02` §3.16 `DM_MemWrite` adds the constraint that actually
governs: *"If the Management Server does not support the L_Data_Extended frame
format, then this maximal size shall be 12 octets."* **[D]**

So the service permits 63 and a large real-device population permits 12. How a
client discovers whether a device supports `L_Data_Extended` is **not stated in
§3.16** — the clause states the consequence, not the discovery mechanism. The
first draft of this subsection concluded from that that the discovery mechanism
was undocumented. **That was wrong, and the fix round of the same day corrected
it: it is documented, in `03_05_01` §4.3.7 `PID_MAX_APDU_LENGTH`.** See §8.7.13.
The default of 12 stands anyway, because it is what the Standard prescribes when
the property is absent and because the failure mode of guessing high is a
partially written memory region that the device may report as successfully
`Loaded`.

Two adjacent facts from the same clause, both design-relevant.

First, **corrected 2026-09-14**: this paragraph used to read *"the read-back is
normative, not a precaution this project invented"*, citing `03_03_07` §3.5.4's
*"The value of the associated memory area shall be explicitly read back after
writing to it."* Quoted with its context, that sentence sits **inside the
active-Verify-Mode paragraph** and constrains the **device**, which is why
`A_Memory_Write.res` carries *"data: the octet(s) read back or no data"* rather
than an echo of the request. The preceding sentence is **[D]** *"With inactive
Verify Mode the remote application process shall not respond."* — so with Verify
Mode off, §3.5.4 obliges nobody to read anything back, least of all the client.
The design spec's read-back rule is therefore **[A] this project's design rule**,
kept because a device that silently drops a write is indistinguishable from one
that took it, and recorded that way in spec §6.2 and §13 **R6**. The rule did not
change; its authority did.

Second, `03_05_02` §3.16's non-verify procedure inserts a *"delay for programming
the memory in the device"* which is **named and never quantified** anywhere
searched — see `GAP-T30-07` in §8.7.15, including the reference chase that failed
to close it. Undocumented.


<!-- END preserved record-1-014 -->
