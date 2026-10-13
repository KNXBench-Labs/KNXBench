# Individual-address reset boundary

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-140)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## §140 The individual-address reset needs the pressed devices named, and its restart is unconfirmed

**User decision 2026-10-05.** No HTTP route or UI for the reset will be built:
the missing reset UI is an accepted, safely refused unsupported boundary, shown
to users in the [manual](../manual/known-issues.md#there-is-no-address-reset-in-the-app).
Reopen only on a new decision.

**Historical live run (2026-09-30, K13 on `1.1.67`).** MP §2.18
`NM_IndividualAddress_Reset` is implemented in
`knx_net::commissioning::individual_address_reset`: broadcast
`A_IndividualAddress_Write` `FFFFh`; `T_Connect`, Basic Restart and
`T_Disconnect` to `FFFFh` without waiting on any confirmation (the
procedure's own rule); broadcast read until nobody answers. The CLI is
`knx device reset-address <a.l.d>... --gateway <host:port> --confirm "I
confirm individual-address reset to 15.15.255"`. There is no HTTP route or
UI yet.

**Current public CLI policy (ADR-0058).** Confirmed CLI resets fail closed
*before opening a tunnel* until every affected device has a complete,
durable, verified pre-write recovery record. The old go is not standing
permission. The offline plan and protocol simulator remain; no HTTP/UI
route exists. `WriteScope::IndividualAddressReset` remains in the protocol
allowlist, but its phrase is not a backup. The caller-named-device guard
requires exactly that set on the first broadcast read, with no excluded
address; it cannot establish storage coverage. MP §2.18 itself resets
whoever is pressed.

**The restart is not evaluated.** On `1.1.67` the device ignored it: the
LED stayed on at `15.15.255` although nobody answered the closing read. The
CLI says the restart is unconfirmed and that a lit LED means programming
mode is still on; the remedy is `knx device program-address` (which gave
`1.1.67` back) or a button press.

**Ours, not MP §2.18's.** The expected-device guard; a first read before
anything is written; a cap of 3 rounds that names the devices still
answering; MP §2.3's 1 s read window.

**Not covered.** Several devices at once on hardware (simulator only); a bus
monitor trace of the sequence. **No HTTP/UI reset:** the earlier live run
used a separately created persistent backup, but the public CLI did not
durably verify all manufacturer-specific affected storage before sending
its broadcast. That historical dump cannot establish general reset recovery.
Do not reopen CLI or expose HTTP reset until an action-specific complete
pre-write backup and abort/recovery contract is verified (ADR-0058); see
`.ai/logs/2026-09-30_claude_commissioning-ui-status-handover.md`.
<!-- END retained catalogue record -->
