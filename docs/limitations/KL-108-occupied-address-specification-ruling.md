# Occupied-address specification ruling

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-108)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->


## 108. MP §2.3 contradicts itself about an occupied IA_new, and this project follows the exception text

**Limitation.** `NM_IndividualAddress_Write`'s own body text and its own
exception-handling paragraph disagree about what happens when the address
being assigned already answers on the bus, and `individual_address_write()`
in `crates/knx-core/src/commissioning/procedure.rs` picks a side rather than
implementing a step the clause itself does not resolve.

**Cause.** `[D]` MP §2.3, p. 14, step 1's body text: *"if A_Disconnect-PDU is
received then IA_new shall be regarded as occupied; end procedure."* That
reads as unconditional: any Disconnect in place of a Device Descriptor
response stops the write. But `[D]` the same clause's own exception
handling, p. 15, *"to 1.:"*, describes exactly that situation — *"If an
A_Disconnect-PDU is received instead of an A_DeviceDescriptor_Response-PDU
… The Management Client shall continue with the Management Procedure in
every case."* — and flatly contradicts the body text it is annotating.

The same paragraph's *"to 2.:"* exception then narrows occupancy generally,
not just the Disconnect case: an answer at IA_new only stops the procedure
if it comes from a device other than the one step 2 finds in Programming
Mode. *"[D]"* p. 15: *"A device with the Individual Address IA_new to be
assigned exists, and it is the one that is in Programming Mode. ⇒ The
Management Client shall continue with the Management Procedure."* That is
exactly the case step 3's own guard exists for — *"set Individual Address
if IA_new != IA_current"* (p. 15) is meaningless if step 1 already stopped
the procedure on the grounds that IA_new answered at all, which is what a
device being re-programmed to its own current address always does.

**Ruling.** This project implements the exception text over the body text:
it is the more specific statement, it is the later one in reading order,
and it is the only reading under which the re-assignment guard in step 3
can ever be reached. Occupancy detected via `A_Disconnect-PDU`, or via a
`A_DeviceDescriptor_Response-PDU` from a device other than the one in
Programming Mode, is surfaced to the operator as a finding rather than
enforced as a silent stop; the stricter body-text reading would refuse a
legal re-programming that the exception text explicitly allows.

**Impact.** A stricter reading of MP §2.3 would refuse to continue past
step 1 whenever *anything* answers at IA_new, including the device already
being re-programmed to its current address. This project's reading instead
lets that case through and reports the ambiguous ones (Disconnect, or an
occupant that has not yet been identified against Programming Mode) as
findings. Cost if this ruling is wrong: a re-programming attempt proceeds
where a stricter reading would have refused it outright — recoverable
(nothing here writes before step 3's own guard), and visible to the
operator via the reported finding, not silently swallowed.

**Lifted when.** Nothing here is waiting on a fix; the clause itself is
what disagrees with itself. This entry closes if a future edition of MP
§2.3 removes the contradiction, or if `docs/RESEARCH.md`'s knowledge-base
audit turns up an erratum for v02.01.02 AS that resolves it.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
The contradiction is in the clause itself; this project keeps following the
exception text, and a later edition or erratum reopens it.
<!-- END retained catalogue record -->
