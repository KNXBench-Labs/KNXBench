# Structure editor scope

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-structure-editor-scope)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## U12 structure editor scope (ISSUE-05)

Area/line renames and line, building-part and group-range reparenting are
local, undoable project commands. The centre workspace reuses the same
creation forms and Inspector commands as the project explorer and Properties.
Since MODEL-01 (2026-10-04) the core resolves every id-addressed structure
command in the installation that owns the entity, and refuses ids found in
several installations or moves that would connect two installations
(`CommandError::CrossInstallation`). Root creates take an optional
`installationId` (absent = first installation) and `PATCH
/api/installations/{id}` renames an installation. Since MODEL-01 web part 1
(2026-10-04) the Project Explorer and the centre workspace offer creation in
every installation: root creates name their installation, children follow
their parent, devices move by drag and drop only inside one installation,
and the project node renames installations. Since ADR-0093 (2026-10-08)
`POST /api/devices` takes an optional `installationId` and `buildingPartId`:
the add-device wizard and every installation's **Unassigned** row create a
line-less device in the named installation and place it in a building part in
the same undo step. The catalog workspace itself still sends neither, so an
unassigned device created there lands in the first installation as before.
Since part 2 (same day) the Inspector edits, deletes, moves and links an
entity in the installation that owns it, and every move or link list offers
only that installation's targets. The bulk toolbar moves a selection only
when one installation places every selected device. With several
installations the CSV buttons name the installation they read or write;
preview and confirmation name the same one. An entity whose id is not owned
by exactly one installation stays read-only. The centre cannot
select an orphaned line that has no projected area, although the explicit
line-move command can attach such an imported line by ID.

A line move changes its area relationship, **not** its numeric line address
or any device's individual address. It refuses a destination with a duplicate
line number or an addressed device outside the target area.line prefix.
Duplicate line/area IDs, multiple owners and multiply placed devices are
not guessed away. Area, line, building-part and group-range IDs repeated in
one or more installations disable structural editing in the Inspector; direct
commands refuse ambiguous IDs too. Since MODEL-02 (ADR-0071) a multiply
placed device and a line listed by several areas can be repaired
**explicitly**: the user names the placement or area to keep
(`POST /api/repair/device-placement`, `POST /api/repair/line-owner`), every
other occurrence is removed in one undoable step. Since 2026-10-04 the web
Inspector offers that choice: a device listed more than once shows every
current slot with "Keep this placement", and a line listed by several areas of
one installation offers "Keep under this area". The line choice is offered
only when every occurrence is the same line; two different lines sharing an
id, or a line id found in two installations, get no repair button. A device
listed twice by one line counts as a conflict, while a line shown under two
areas does not make its devices conflicts. Nothing is repaired automatically, duplicate ids
are not renumbered, and ambiguous building-part or group-range placement has
no repair command. Since ADR-0074 a native save is exact or refused: duplicate
entity ids, orphaned lines, inconsistent parent/child lists and a device
listed twice in one building part are refused with
`StoreError::Unrepresentable` before any write, where they used to reopen
different. Orphaned lines can be attached with the line move; hierarchy
mismatches and duplicate ids have no in-app repair, so such a project cannot
be saved natively until it is fixed elsewhere. ETS import no longer creates
duplicate internal ids (ADR-0073). `.knxdb` save refuses an ambiguous topology
(`StoreError::AmbiguousTopology`) instead of silently keeping the last
placement, as it did before; a `knx-cli` import producing such a project
therefore fails at save. Building-part moves refuse new
cycles and ambiguous parent references. Group-range moves additionally
require destination-span containment and no sibling overlap. The undo-only
placement commands retain original sibling positions and may restore
pre-existing imported invalid relationships; they do not permit a new invalid
forward move. Deleting a non-empty area, line, building part or group range is
non-cascading. Deletes refuse inconsistent parent/child references that would
leave an unlisted dependent orphan, and their undo restores the original
flat-list and parent-child positions rather than appending.

Native `.knxdb` save/reopen is tested for these changes through the existing
whole-project save path. Several structure commands still have no incremental
store-sync update; a bare incremental-sync call is **not** a substitute for
saving the project. No full ETS edit/round-trip parity is claimed. The
first-installation UI was tested with mocked local API responses and Chromium;
native WebKitGTK drag/drop, screen-reader operation and live bus behavior are
not covered. Existing device-to-line/device-to-building drag/drop remains
bound to its already validated commands; structural moves use labelled native
select controls, not a new drag gesture. Since UX-01 (2026-10-04) a group
address can be dragged from the Project Explorer onto a communication
object's link row. The drop links it once through `POST /api/group-links` in
the direction shown in that row (default Send, the same default as the
keyboard path); an address the device cannot link is refused locally. Verified
in headless Chromium with real HTML5 drag events, not in native WebKitGTK.
<!-- END retained catalogue record -->

## Update 2026-10-27: explicit catalogue installation target

The catalogue workspace no longer silently targets the first installation when
several installations are open and no line is selected. A labelled
**Installation** selector starts without a choice; Create and Enter are blocked
until a current installation is selected. Removing that installation clears the
choice rather than falling back, even if only one installation remains; that
case requires a fresh explicit choice. Lost-response retries retain the original
request and installation instead of rebuilding them from current controls.

With a single installation the request stays unchanged. A selected line decides
its installation; with no open project, creation is disabled. The catalogue
workspace still sends no building part. These controls use the existing
validated, undoable create command and do not add cross-installation transfers,
a new site model or native-platform acceptance.
