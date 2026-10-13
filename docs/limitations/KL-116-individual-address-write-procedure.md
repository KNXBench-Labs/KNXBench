# Individual-address write procedure

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-116)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 116. `NM_IndividualAddress_Write` does not loop for the operator, and reads one Transport Layer release as MP §2.3 never quite says

**Limitation.** `[C16]` `individual_address_write()`
(`crates/knx-net/src/commissioning/individual_address_write.rs`) diverges
from MP §2.3 in two places, both narrow, both deliberate.

*Step 2 does not repeat.* The Standard's step 2, p. 14, reads *"2. wait
until device is in Programming Mode: repeat until one
A_IndividualAddress_Response-PDU is received … end repeat"*, and p. 13
states the same obligation in prose: *"The procedure shall wait until
exactly one device is in Programming Mode."* This implementation
broadcasts once. On nobody, or on several, it returns
`IndividualAddressWriteError::Count` to its caller rather than
re-broadcasting until the count comes right.

*A released connection is read as an `A_Disconnect-PDU`.* p. 14 rules that
*"if A_Disconnect-PDU is received then IA_new shall be regarded as
occupied"*, and p. 15's "to 1." explains the two devices that behave that
way. Step 1 reports `Occupancy::OccupiedAfterDisconnect` for that case —
and also when this client's own Transport Layer releases the connection
because nothing acknowledged four transmissions (TL §5.4.1, p. 22,
transition `E18` in `OPEN_WAIT`, action `A6`). No `A_Disconnect-PDU` was
received in that second case; the local Transport Layer synthesised the
indication.

**Cause.** What step 2's `repeat` waits for is a human walking to a device
and pressing a button. A library function cannot wait for that on the
thread that called it, and this project builds no procedure-level retry
loops — the Standard's own General Exception handling, MP §3.1, p. 68, is
*"In general if an error is detected, the download shall be interrupted
and an error-message shall be raised."*, which is the opposite of a retry
obligation. Returning the count to the operator, who is the one who has to
go and press the button, puts the loop where the only actor that can close
it lives. The Standard's own footnote 2) on p. 15 points the same way:
*"The user of the Management Client should get an information in how many
devices are Programming Mode is active (none or more than one)."*

The occupancy reading has a smaller cause: MP §2.3 enumerates three
outcomes for step 1 — a response, a received `A_Disconnect-PDU`, and
silence until the time-out — and a device that acknowledges nothing at all
fits none of them exactly. It is not the silence arm, because that arm's
*"If no A_DeviceDescriptor_Response-PDU is received after time-out"*
describes a connection that is still open when the time-out expires, which
this one is not. Of the two remaining arms, "occupied" is the one that
does not risk handing `IA_new` to a second device.

**Impact.** Neither divergence changes a stop/continue decision the
Standard specifies. The absent `repeat` makes a zero-or-several count an
error the caller sees instead of a wait the caller cannot see; a caller
that wants the Standard's behaviour calls the function again, and any UI
that drives this has to tell the operator what to do anyway. The occupancy
reading affects only which of two occupied labels a report carries, since
`KNOWN_LIMITATIONS.md` §108's stop/continue comparison is on addresses and
not on labels — an unacknowledged `IA_new` that turns out to be the
Programming Mode device itself still continues, and one that turns out to
be somebody else still stops.

**Lifted when.** The `repeat` is lifted by whatever owns the operator
dialogue — a UI loop that re-runs step 2 while showing "press the
programming button on exactly one device" implements the Standard's wait
in the only place it can be implemented, and this function stays the
single-shot primitive underneath it.

**Status 2026-09-28 (K6): the `repeat` half is lifted for the command line,
simulator-verified.** `knx_net::commissioning::programming_button_wait`
repeats the broadcast count (one 1 s window per round, a 1 s pause between
rounds) until exactly one device answers, the operator stops it, or `--wait`
runs out. Every round goes to an observer; `knx device program-address`
prints the count and "press" / "release all but one" whenever it changes
(MP §2.3 footnote 2). The wait runs before step 1 rather than between steps
1 and 2. That changes no decision: step 1 only reads, and
`individual_address_write` still does its own step 2 count and its re-count
before the write. A button released after the wait is caught there (test
`a_button_released_after_the_wait_is_caught_by_step_two`).
`individual_address_write` itself is unchanged, and still single-shot.
**UI half, same day:** the web's **Program address** tab and
`/api/device-address/*` (ADR-0046) run the same loop, show the prompt
whenever the count changes, and can stop the wait (never the procedure).
**Live, 2026-09-29 (K6 [W]):** `1.1.67` → `1.1.68` on a real MDT push
button. The user pressed the button in round 37. Steps 1–3 ran, step 4
connected to `1.1.68` and read the device there, and only the closing Basic
Restart went unacknowledged, as on every restart of this device (RESEARCH
§19). The procedure reported that as a step 4 failure, *"did not answer at
the new address"*, which was wrong. A read-only scan afterwards found
`1.1.68` occupied and `1.1.67` vacant, and the programming LED was off, so
the restart had happened. **Fixed the same day:** step 4's restart silence
is now `AddressRestart::Unconfirmed` inside a successful report, as in the
download path. Only a restart refused before sending is still a step 4
failure. The CLI prints `restart: NOT confirmed`, and the server's
`finished` status carries `restartConfirmed: false`; the web panel does not
show that field yet (web lock held by the UI session). Verified live on the
fixed build: `1.1.68` → `1.1.67` ended `finished`, exit 0, restart NOT
confirmed, and the device answered at `1.1.67` afterwards (RESEARCH §19). The
occupancy-reading half of this entry is unchanged. The occupancy reading is revisited if
`docs/RESEARCH.md`'s knowledge-base audit turns up spec text or an erratum
that rules on a Transport Layer release at step 1.

**Current availability (2026-10-01, ADR-0059).** The historical live K6
round trip does not establish complete durable recovery for a later
button-selected address write. Public confirmed CLI/HTTP starts now fail
before opening a tunnel, pending verified device-specific affected storage,
durable pre-send backup/readback, abort plan and a fresh go. A diagnostic
dump of selected memory or a property-only backup is not proof of that
coverage. The Web tab uses the read-only availability endpoint to disable
Program before consent on blocked/unknown status and again after a 412;
the server POST remains the independent fail-closed authority. Simulated
protocol exercises do not reopen the production write path (RESEARCH §24).

**Not a limitation any more.** An earlier draft of this section claimed MP
§2.3 *"does not consider a `T_Connect` refusal, or a connection that opens
and then answers nothing at all"*. The clause considers both, on p. 14,
and rules the opposite way from the code that was written against that
claim: *"if negative A_Connect.Lcon ⇒ IA_new is not occupied"* and *"If no
A_DeviceDescriptor_Response-PDU is received after time-out ⇒ IA_new is not
occupied"*. Both now follow the text, and the two `Occupancy` variants
invented to hold them are gone.
<!-- END retained catalogue record -->
