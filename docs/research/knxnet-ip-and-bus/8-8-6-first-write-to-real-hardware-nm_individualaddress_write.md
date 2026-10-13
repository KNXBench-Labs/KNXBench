# 8.8.6 First write to real hardware — `NM_IndividualAddress_Write` (2026-09-26)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="886-first-write-to-real-hardware--nm_individualaddress_write-2026-09-26"></a>
<!-- BEGIN preserved record-1-037 -->
#### 8.8.6 First write to real hardware — `NM_IndividualAddress_Write` (2026-09-26)

**The first time this project changed a physical device.** An operator
explicitly authorised assigning `1.1.67` to the device found in Programming
Mode by §8.8.5, *"auch als test ob schreiben funktioniert"*. Deliverable:
`crates/knx-net/tests/live_individual_address_write.rs`, `#[ignore]`d and
additionally gated on `KNX_WRITE_NEW_ADDRESS`, so `--ignored` alone cannot
perform it.

**Result: the write worked.** Independently confirmed, by the scan probe rather
than by the procedure vouching for itself — §8.8.3 established that a
`ManagementSession` read cannot be trusted to prove presence or absence:

| | before | after |
|---|---|---|
| `1.0.71` (old) | occupied, mask `0701h` | **vacant** (6005 ms, full timeout) |
| `1.1.67` (new) | **vacant** (6005 ms) | **occupied, mask `0x0701`** (196 ms) |

Identity at the new address is byte-identical to what §8.8.5 measured at the
old one: mask `0701h`, `PID_MANUFACTURER_ID` `00 83`, `PID_HARDWARE_TYPE`
`00 00 00 00 01 27`, `PID_PROGRAM_VERSION` still refused with zero elements. So
it is the same device, readdressed — not a coincidental second device.

**But step 4 failed, and the procedure reported failure.** `individual_address_
write` returned `Err`, and the report is precise about where: `occupancy:
NotOccupied` (step 1 correctly found `1.1.67` free), `wrote: true` (step 3 did
broadcast `A_IndividualAddress_Write`), then step 4 — connect to the new
address, read Device Descriptor Type 0, `A_Restart` — failed with
`SessionError::ConnectionReleased`, i.e. nothing acknowledged the `T_Connect`
and the Transport Layer gave up after its repetitions.

This is **not** a contradiction of the success above, and MP §2.3 anticipates
exactly it. Its exception handling "to 4." says *"If no
A_DeviceDescriptor_Response-PDU is received, than the programming of the
Individual Address may have failed, or the system (Router) is not configured
correctly"* — and, as design spec §4.2 already noted, **the Standard itself
refuses to distinguish those two causes**. The observed facts distinguish them
here: the device does answer at `1.1.67` seconds later, to a fresh tunnel, so
the programming did *not* fail. The most likely reading is that the device was
still settling immediately after adopting its new address, and
`individual_address_write` proceeds from the broadcast write to step 4's
`T_Connect` with **no delay at all** — `SessionTiming::programming_delay` exists
but this procedure never consults it. Not proven: no frame-level capture was
taken, and a router/line-coupler configuration cause is not excluded.

**Consequences, none of them cosmetic:**

1. **The procedure's failure was honest and its report was usable.** A boolean
   "worked/failed" would have been actively misleading here; `wrote: true` plus
   a step number is what made the situation diagnosable. This is evidence for
   the step-record design, not against it.
2. **`individual_address_write` needed a settling allowance before step 4.**
   MP §2.3 gives no value. It was deliberately not fixed in the pass that
   discovered it. **Fixed on 2026-09-27, in simulation only:** step 4 now
   treats one unanswered or released `T_Connect` as possibly still settling,
   waits `SessionTiming::restart_basic_t1` (MP §3.7.1.1.2's `t1`, 1 s) and
   connects once more. A second silence is still the genuine "to 4." failure,
   still with `wrote: true`; a *rejected* connect is not retried at all.
   `t1` is a **borrowed** figure — it is defined for reconnecting after a Basic
   Restart, not after `A_IndividualAddress_Write` — used as the Standard's
   nearest comparable number instead of inventing a constant. The fix is pinned
   by `a_device_still_settling_after_the_write_is_not_a_failure` (watched to
   fail first with `Session { step: 4, wrote: true }`) and its counterpart
   `a_device_silent_after_the_settling_wait_still_fails_step_four` (checked
   against a two-retry mutant, which it catches). **Not yet re-verified
   against the MDT device**: that needs another Programming Mode session, and
   whether 1 s suffices for that device is unmeasured.
3. **A caller cannot currently distinguish "wrote but could not confirm" from
   "did not write"** without inspecting the report's `wrote` field. The error
   type does carry the report, so the information is present; nothing surfaces
   it as a distinct outcome.
4. **Programming Mode switched itself off** after the write: a subsequent
   broadcast read returned zero responders. Consistent with RES §4.26.1's
   autonomous disable, though four minutes had not elapsed, so this looks like
   the device's own post-write behaviour rather than the timeout.

**What was changed to allow this at all.** `knx_core::commissioning::mutation::
hardware_write_is_authorised` is a new allowlist of exactly two scopes,
`IndividualAddressProgramming` and `Restart`. `check_write_target()` now permits
hardware writes only when transport *and* authorisation are both hardware *and*
the scope is allowlisted; simulator behaviour is unchanged, and mixed
hardware/simulator pairs are still refused so a confirmation phrase cannot be
spent on a fixture. `Download`, `Unload` and `ProgrammingModeToggle` remain
refused on hardware even with a correctly typed confirmation phrase —
`crates/knx-net/tests/hardware_write_gate.rs` holds both sides of that down.


<!-- END preserved record-1-037 -->
