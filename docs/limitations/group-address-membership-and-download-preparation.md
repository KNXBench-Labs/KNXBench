# Group-address membership and download preparation

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-group-address-membership-and-download-preparation)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Group-address members and download preparation (2026-10-12)

**Members.** Members are added from the address side through the existing
`POST /api/group-links` route (`Command::LinkComObject`); the UI mirrors its rule
(MODEL-01 / ADR-0070) and adds none of its own. The rule is per installation, not
`installations[0]` as an older code comment and the package brief said: a device
placed in one installation may link only to that installation's addresses, a device
placed in none may link to any, one placed in two is refused, and an address that
occurs in several installations gets no picker. The picker lists only devices of the
open project tree; canonical devices outside every placement are not offered there.
The native store refuses to save such an unplaced device anyway, so the unplaced
case exists only in unsaved working state. Datapoint types of object and address are
shown side by side, never judged in the UI; the server's existing outcome appears
after linking. A late reply never changes another address's or project's draft, but
ignoring a reply does not cancel the command: the returned snapshot is still offered
to the workbench, which accepts it only for the same project lifetime. The member
panel appears in both the workspace and Properties, so two forms can be open for the
same address; each has its own pending guard and the server refuses duplicates.

**Download preparation.** The gateway list is the shared start-up interface search;
the tab never searches on its own, only on **Search again**. Exactly one answer is
filled in only while nothing was chosen or typed and no plan is shown. The typed
address is not validated in the tab (the server's existing checks apply). Changing
the gateway now discards a shown plan, as device, scope and project changes already
did; the plan itself is not bound to a gateway on the server. The hand-off from the
device editor selects a device by id in the same project and does nothing else; it
waits for the download status, refuses during a plan request or a running or
connecting download, and re-reads an uncertain status once before refusing. The step
guide reflects checks the panel and server actually make; it is not a compatibility
or programming-mode statement. Covered by synthetic unit, HTTP and production-build
Chromium fixtures only — no real gateway, device or bus was contacted.
<!-- END retained catalogue record -->
