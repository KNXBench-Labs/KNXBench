# Device editor address scope

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-device-editor-address-scope)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## U11 device editor scope (ISSUE-09)

A device placed in a line shows its line-derived area.line prefix and lets the
user change only the device octet (1–255, or 0 for an evidenced coupler). New `.0` assignments are refused
unless the product database classifies the device's hardware as a coupler
(`Hardware/@IsCoupler`, RESEARCH §25): the server then uses
`SetCouplerIndividualAddress`. Since 2026-10-04 the web editor submits `.0` and
shows the server's refusal otherwise; a device whose product is not installed
stays refused. Imported `.0` addresses
remain intact, including after undo. An imported
address with a line prefix mismatch is shown for repair, not silently
rewritten. Until repaired, moving that device to another line is refused; a
move never auto-allocates an address. Unassigned devices retain the full
address editor. Duplicate and out-of-line addresses fail in the core, not
just in the browser. If a device appears in multiple topology placements
(two lines, twice on one line, both a line and the unassigned list, or
twice unassigned), or
its line belongs to two areas, the editor refuses to choose a prefix and
stays disabled until the topology is repaired (MODEL-02 repair commands,
server/API only so far); imported values are preserved. Clearing an address through the core remains an
undoable repair step even when placement is malformed.

The line-move command and new group links act in the device's own
installation (MODEL-01): a device may move between lines of its installation
and link to that installation's group addresses. Moving a device to a line,
building part or group address of **another** installation is refused, as is
any edit of an id that occurs in several installations. Whether the web
dropdowns list later-installation targets is a UI question still open under
the Web lock.

“Send + Receive” and “unlink both” are one atomic project edit and one undo
step. If either direction is already linked when adding both, the operation
fails and leaves all links unchanged; if either is missing when unlinking
both, even the **ordering** of other links stays intact. A successful unlink
can be undone without moving an interleaved or imported duplicate link.
Individual directions remain editable separately. An older server that only
accepts Send/Receive rejects `Both`;
the client reports that refusal and does not fall back to two non-atomic
requests. Flag labels expand the standard letters in English and German,
but changing flags or links here is an offline project edit, **not** a KNX
device download. Chromium layout checks cover 360, 640 and 1440 px with long
names/addresses in both languages against a local Vite/mock fixture; native
WebKitGTK, hardware behavior and screen-reader announcements remain unverified.
<!-- END retained catalogue record -->
