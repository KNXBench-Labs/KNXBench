# 8.8.4 Reconciliation against the spec

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="884-reconciliation-against-the-spec"></a>
<!-- BEGIN preserved record-1-035 -->
#### 8.8.4 Reconciliation against the spec

- **`PID_ERROR_CODE`, `PID_DEVICE_CONTROL` and `PID_OBJECT_INDEX`**, named in
  design spec §14's phase 3 checklist, were **not** read against real hardware
  this pass — the properties this test reads are the ones named in this
  task's own dispatch brief (device presence, mask version, descriptor reads,
  property reads, load-state reads). That set is **different from** §14's
  list, not merely narrower than it: three of §14's six were skipped, and two
  it never named — `PID_HARDWARE_TYPE` and `PID_PROGRAM_VERSION` — were read
  as well. Noted as a residual coverage gap in the design spec (§14.1), not
  closed here — a second read-only pass, or an extension of this same test,
  is the natural next step and needs no new safety reasoning to run.
- **`1.1.24`'s access-level refusal on `PID_HARDWARE_TYPE`/`PID_PROGRAM_VERSION`
  while `PID_MANUFACTURER_ID` succeeded, all under `AuthorisationPlan::Skip`**
  is a live confirmation of design spec §10.2, not a contradiction of it:
  *"the maximum access level protected with `FFFFFFFFh`"* is whatever the
  device's own Profile grants an unauthorised client, and §10.2 already
  predicted *"a client that skips authorisation therefore works until it does
  not."* **[V]** This is the first live evidence for that sentence.
- **The mask `0701h` result matches §8.5 Finding 4's prior scan of this same
  installation** (2026-09-13, "nine consecutive occupied addresses... the same
  Mask Version, `0x0701`", addresses unnamed there for the reason given in
  that section). This section names the addresses because they are the
  already-publicly-approved nine from spec §2.2, not because the redaction
  policy changed — §8.5's broader, unapproved inventory stays unnamed.
  Finding 4 did not record which addresses were in its nine, so this is
  corroboration of "this stretch of the line runs the same mask", not a
  re-identification of Finding 4's own sample.
- **Design spec §14's implicit assumption — that phase 3's read-only checklist
  is something `ManagementSession::read_only` can just run — is corrected**
  by §8.8.3 above: presence must be established with the scan probe first.
  design spec §14.1 and §13 R20 carry the correction.

See [KNOWN_LIMITATIONS.md §7](../../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)
for the durable record.


<!-- END preserved record-1-035 -->
