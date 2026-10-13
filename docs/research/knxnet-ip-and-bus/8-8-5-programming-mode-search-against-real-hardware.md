# 8.8.5 Programming-mode search against real hardware (2026-09-26)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="885-programming-mode-search-against-real-hardware-2026-09-26"></a>
<!-- BEGIN preserved record-1-036 -->
#### 8.8.5 Programming-mode search against real hardware (2026-09-26)

First live use of MP §2.2's `NM_IndividualAddress_Read` in this project, run
read-only against the real installation's gateway on operator request while a
device sat in Programming Mode. Deliverable:
`crates/knx-net/tests/live_programming_mode.rs`, `#[ignore]`d like every other
live test, run with `KNX_GATEWAY` set.

What was sent: one connectionless broadcast `A_IndividualAddress_Read`, waited
out for the full `INDIVIDUAL_ADDRESS_READ_TIMEOUT` (3 s, never cut short by an
early answer), then a connection-oriented read-only identification pass against
the single address that answered. Every session was
`ManagementSession::read_only` with `AuthorisationPlan::Skip`, so no write path
and no `A_Authorize_Request` existed to use.

Observations, reported exactly as measured:

- **One responder, one frame: `1.0.71`.** `device_count() == 1` and
  `frame_count() == 1`, so no Layer-2 repetition needed discarding on this run.
- **Mask version `0701h`** — System B, the same mask the nine `1.1.24`–`1.1.32`
  devices reported in §8.8.2.
- **`PID_MANUFACTURER_ID` = `00 83`.** Resolved against local product data, not
  from memory: `knx_master.xml` in the corpus declares
  `<Manufacturer Id="M-0083" KnxManufacturerId="131" Name="MDT technologies">`,
  and 0x0083 = 131 **[V]**.
- **`PID_HARDWARE_TYPE` = `00 00 00 00 01 27`**, six octets, matching
  `PDT_GENERIC_06` (§8.6.4).
- **`PID_PROGRAM_VERSION` refused with zero elements** on the Device Object —
  the same "does not exist, or insufficient access level" answer §8.8.2 already
  recorded for `1.1.24`. Not a new failure mode, and not diagnosed further here.

**The address was not the expected one.** The operator expected a device on
line `1.1`; the responder is on line `1.0`. This is exactly why a
programming-mode search is a *search*: the broadcast reports whoever answers,
and the device's current address is an observation, not an assumption.

**`PID_HARDWARE_TYPE` is not a product identifier on its own, and it is not
`Hardware/@SerialNumber`.** Searching the local corpus for the observed
`000000000127` matched only `LdCtrlCompareProp` `ObjIdx="0" PropId="78"`
`InlineData` in `MDT_KP_BE_01_Push_Button_V15a.knxprod`, whose products are the
`BE-TA55*.01` generation; the `SerialNumber` attributes in that same file are
36–39 and 361–391, i.e. unrelated numbers. The operator named a `BE-TA55P2.G1`,
and no `.G1` order number exists anywhere in the local MDT corpus (only
`BE-TA55P2.01`, `BE-TA55P2.02` and `RF-TA55P2.01`). **So the corpus match is
consistent with an MDT 2-fold push button and is not proof of the exact model or
generation** — identifying a product from `PID_HARDWARE_TYPE` alone is an open
question, not a solved lookup. `PID_ORDER_INFO` was not read on this pass.

**Write status unchanged.** Assigning a new individual address was requested and
**not performed**: `ManagementSession::authorised` refuses at construction
whenever either the transport or the authorisation is hardware
(`SessionError::NotASimulator`), per design spec §15 and §13 R11. This is now
proven rather than asserted, with no gateway required, by
`crates/knx-net/tests/hardware_write_is_refused.rs` — a correctly confirmed
hardware `WriteAuthorisation` plus a default (`Hardware`) transport yields the
refusal before any frame can be constructed.


<!-- END preserved record-1-036 -->
