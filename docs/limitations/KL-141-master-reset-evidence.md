# Master Reset evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-141)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## §141 Master Reset erases in the simulator only; hardware keeps its configuration

**Status (2026-09-29, K14).** MP §3.7.1.2 Tables 4 and 5 are typed
(`knx_core::commissioning::master_reset`). A request is only built with an
Erase Code a client may send (`01h`–`08h`) and a Channel Number the code
allows. `knx_net::commissioning::master_reset` runs MP §3.7.3
`DM_Restart_RCo`: probe, request, the 6 s disconnect wait, then the device
is looked for where it should be, once more after the Process Time.

**The support check is ours.** MP §3.7.3 requires verifying Master Reset
support first and names no method. We send a Confirmed Restart (`01h`,
erases nothing) and go on only on a positive `A_Restart_Response`.
Footnote 11 warns that old devices may just restart; that is what the probe
risks, and all it risks.

**Refused on hardware.** Erase Codes `02h`–`08h` need the new
`WriteScope::MasterReset`, which `hardware_write_is_authorised` refuses.
**Fixed along the way:** `restart_master_reset` sent every Erase Code under
`WriteScope::Restart`, which hardware permits, so a restart phrase could
have carried a Factory Reset. The scope now follows the code; a test and a
mutant hold it. `01h` stays a restart.

**What the simulator models.** A Basic Restart on every code
(programming mode off); the address to `FFFFh` for `02h`/`03h`; RES's
download-counter table; reserved codes answered with `02h`. What a Factory
Reset erases beyond that is *"implementation dependent"* and not modelled.

**Not decided.** `0701h` lists Master Reset as optional (Profiles Table
4.2). Whether `1.1.67` supports it is unknown, and a live Factory Reset
would need the option-C re-download afterwards (goal-commission K14).

**Lifted when.** An operator asks for a named code on a named device; the
scope joins the allowlist with a test; the bus monitor shows the probe,
the request, the answer and the device back where the table says.
<!-- END retained catalogue record -->
