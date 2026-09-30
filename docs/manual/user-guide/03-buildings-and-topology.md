← Previous: [Projects: create, open, import, save, export](02-projects.md) · [Manual index](../README.md)

# Buildings and topology

A KNX project describes the same set of devices twice, from two directions.

**Topology** is the electrical truth: which bus line a device is physically wired to.
It nests as **area → line → device**, and it is where a device's **individual
address** comes from — the `1.0.1`-style number that identifies exactly one device on
the bus. See
[Topology and individual addresses](../knx-basics/02-topology-and-individual-addresses.md)
for the KNX rules behind it.

**Buildings** is the human truth: which floor, which room, which cabinet. A building
part can be a Building, a Floor, a Room, a Corridor, a Distribution Board, or a
generic Building Part, and parts nest as deeply as you like.

A device lives in one line and, optionally, in one building part. The two placements
are independent: moving a device to another room does not rewire it, and moving it to
another line does not move it out of its room.

## The two views

![The Topology view showing area 0 with an empty line 0.0, and area 1 with line 1.0
holding four devices, each with a communication-object
count](../../assets/screenshots/porcelain-topology.png)

The Topology view draws each area as a block, each line inside it, and each device as
a tile with its individual address and its number of communication objects. The `+`
button on a line header adds a device to that line. An empty line is drawn as an empty
line, not hidden.

![The Buildings view with a single card named "KV v2.5 - demo", tagged Building, and a
"+ Device" button in the header](../../assets/screenshots/porcelain-buildings.png)

The Buildings view draws building parts as nested cards with their devices inside.
Clicking a card focuses it: the view then shows that part's devices as a table and its
child parts below, with a back link to the overview.

Both views offer **Add** disclosures in the main workspace as well as creation rows in
the project explorer. Select an area to reveal its **Add line** action; select a
building part to reveal **Add building part inside …**. A selected area's, line's
or building part's editor appears in the workspace and in Properties on the right;
both use the same validated project command, not separate edits.

## Creating structure

Create from a disclosure in the main workspace or from the corresponding inline
row in the project explorer. No dialog is needed for structure; devices use
the product catalog.

| To create | Where | Fields |
| --- | --- | --- |
| Area | **Add area** in Topology, or under Topology in the explorer | Address (a number), name |
| Line | Select an area, then **Add line in …**, or create under that area in the explorer | Address (a number), name, medium reference (for example `MT-0`) |
| Device | The "+ Add device" button under a line, or `+` on a line in Topology | Opens the product catalog — see [Devices and products](05-devices-and-products.md) |
| Building part | **Add building part at installation root** or select a part and choose **Add building part inside …**; the explorer has the same two levels of action | Kind, name |

Open the disclosure with Enter or Space, type the fields and press Enter in a
field or click **Add**. A refused duplicate address or invalid parent appears
beside the form; nothing is created. The parent is explicit in each action:
creating at the installation root does not silently choose the selected part.

> **Note**
>
> Creation and structure edits are available only for the first installation.
> Later installations remain visible and savable but cannot be mutated by these
> commands. Imported duplicate IDs are not used to guess a parent.

## Editing

Select something, and the properties pane on the right shows what can be changed.
Areas, lines and building parts also have a contextual editor in the centre
workspace; changing either copy uses the same undoable command.

**A device** has an address field, a line, a building part, a description, and a
Delete button.

![The properties pane for device 1.0.1, showing Delete, Address 1.0.1, Line "Line 0:",
Building part "(none)" and an empty Description
field](../../assets/screenshots/porcelain-device-inspector.png)

Areas, lines and building parts can be renamed. Device renaming is still not
available. Text fields commit when they lose focus, and Enter is a shortcut
for that. A rejected value snaps back and shows the reason.

### Individual addresses

For a device on an unambiguous line, the area.line prefix comes from that line;
only the device number (1–255) is editable. A device not assigned to a line
instead offers the complete address. Duplicate addresses, an out-of-line
prefix and newly assigning device number `0` are refused. Existing imported
mismatches or `.0` values are displayed intact so they can be repaired
explicitly. See [Devices and products](05-devices-and-products.md#address-and-line)
for the address editor and ambiguous-import cases.

### Moving a device

The **Line** select moves a device to another line, or to Unassigned. The **Building
part** select moves it to another part, or to none. Both act immediately, and both are
one undo step.

Selecting several devices — checkboxes in the group-address table, or `Ctrl`-click and
`Shift`-click in the tree — puts a bulk action bar in the strip below the header, with
a move-to-line select, a move-to-building-part select, and a delete button.

> **Warning**
>
> Moving a device never changes its individual address. An addressed device
> whose address does not match the destination line is refused: clear its
> address first, move it, then assign an address under the target prefix.
> Undo restores the original project placement, not a bus state.

In the Project Explorer you can also drag an eligible **single device** onto a
line or building part in the first installation. The drop uses the same
validated, undoable move as the Inspector selects; it does not change the
individual address. The Inspector selects remain the keyboard-accessible way
to perform either move. Group-address links need an explicit send/receive
direction and cannot be created by dragging.

### Moving lines and building parts

Select a **line** to see its current **Assigned area** and choose another area
from the native select, in the centre workspace or Properties. The line keeps
its own numeric address and every device keeps its individual address; the
move is refused if the target already has that line number or any placed
device's address would not match the new area.line prefix. An orphaned imported
line can be attached explicitly, but a line with multiple owners or duplicate
references cannot be moved by guessing. Undo restores the previous area and
sibling position, including an original imported orphan placement.

Select a **building part** and choose its parent, or **installation root**,
with the parent select. The part's children and devices stay attached. Cycles,
ambiguous imported references and unknown parents are refused; changing parent
is one undoable step, and undo restores its former sibling order. These selects
work with a keyboard. Existing device drag/drop remains a separate gesture;
structure reparenting uses these explicit controls, not an unverified drag target.

If an imported structure ID occurs more than once in a project, KNXBench
disables its rename, move and delete controls instead of guessing which
installation you meant. Correct duplicate identities at the source; no
automatic renumbering or cross-installation structure editor exists here.

## Deleting

Delete never cascades. KNXBench refuses to delete anything that still holds something
else:

| Deleting | Is refused while |
| --- | --- |
| An area | It still has lines |
| A line | It still has devices |
| A device | Any of its communication objects is still linked to a group address |
| A building part | It still has child parts or devices in it |
| A group range | It still has subranges or group addresses assigned to it |

Empty it first, then delete it. If an imported child points at a parent that
does not list it, the parent cannot be deleted until that inconsistency is
repaired; deleting it would leave an orphan. Undo restores the original
sibling order as well as the deleted structure.

> **Warning**
>
> Deleting does not ask for confirmation, in the properties pane or in the bulk action
> bar. Undo (`Ctrl+Z`) is the safety net, and it works for deletes — but the undo
> history lives in the running application, not in the project file, and opening or
> importing a project clears it. If the deletion mattered, save before and after.

[Manual index](../README.md) · Next: [Working with group addresses](04-group-addresses.md) →
