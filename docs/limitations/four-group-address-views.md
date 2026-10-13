# Four group-address views

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-four-group-address-views)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Four group-address presentations (2026-10-12)

The connection matrix lists communication objects that are linked to at least
one group address (reverse links in the project projection), plus every object
of the devices whose object list the user explicitly loads through the canonical
device-detail read (`GET /api/device/{id}`). Devices that are not loaded
contribute only their linked objects, so the matrix is not a whole-project object
inventory unless every device has been loaded; the UI states this scope and
counts loaded, not loaded, loading and unavailable devices. There is no "load all
devices" bulk action. Loaded lists are re-read after every project update (one
request per loaded device) and forgotten on a project switch; a failed read, or a
reply that names a different device, is shown as unavailable and never treated as
absence. A linked object missing from its device's loaded list is reported, not
hidden. An empty cell means "no project link", never "no bus traffic". At most 12
object columns are shown at a time, chosen explicitly; loaded objects are offered,
never auto-selected. The atlas is stored-range navigation, not a topology or
floor-plan map; there
is no geometric zoom/pan, live bus overlay, layout persistence or direct matrix
editing. Large lists draw 500 rows at a time (explicit "show more/all"); this is
a render window, not virtualization, and "show all" on very large projects is
correspondingly slow. Keyboard arrow navigation covers the view's navigable
buttons; no full accessibility certification is claimed. Grouping follows stored
ranges only: a project without stored ranges stays flat (no derived 2-/3-level
tree from address numbers; parked until feedback asks for it).
<!-- END retained catalogue record -->
