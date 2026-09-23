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
The demo project has exactly one building part, which is why this view looks quiet.
Clicking a card focuses it: the view then shows that part's devices as a table and its
child parts below, with a back link to the overview.

Both views are read-and-select surfaces. The structure itself is created in the project
explorer on the left.

## Creating structure

Everything is created inline in the tree. No dialogs, except for devices, which come
from the product catalog.

| To create | Where | Fields |
| --- | --- | --- |
| Area | Under Topology | Address (a number), name |
| Line | Under an area | Address (a number), name, medium (for example `MT-0`) |
| Device | The "+ Add device" button under a line, or `+` on a line in the Topology view | Opens the product catalog — see [Devices and products](05-devices-and-products.md) |
| Building part | Under Buildings, or under an existing part | Kind (Building, Floor, Room, Corridor, Distribution Board, Building Part), name |

Type into the row and press Enter, or click **Add**. If the server refuses — a
duplicate area address, for example — the reason appears next to the row and nothing
is created.

> **Note**
>
> The creation rows only appear under the first installation. Nearly every edit
> command in KNXBench targets the first installation, so offering the rows elsewhere
> would be a promise the application cannot keep. Almost every project has exactly one
> installation, so this rarely comes up.

## Editing

Select something, and the properties pane on the right shows what can be changed.

**A device** has an address field, a line, a building part, a description, and a
Delete button.

![The properties pane for device 1.0.1, showing Delete, Address 1.0.1, Line "Line 0:",
Building part "(none)" and an empty Description
field](../../assets/screenshots/porcelain-device-inspector.png)

Note what is not there: a name field. Devices, areas and lines cannot be renamed in
KNXBench today. Building parts and group ranges can. This is a gap in the command set,
not a hidden menu — see [Known issues](../known-issues.md).

Text fields commit when they lose focus, and Enter is a shortcut for that. A rejected
value snaps back to what it was and shows the reason.

### Individual addresses

The **Address** field takes the usual `area.line.device` form, for example `1.1.5`.
Leaving it empty is allowed; a device without an address shows as Unassigned.

Two things are worth knowing:

- **A new device starts with no individual address.** Creating a device from the
  catalog places it in a line but does not invent an address for it. You assign one.
- **Nothing checks the address against the line it sits in.** KNXBench refuses two
  devices with the same individual address, two areas with the same address, and two
  lines with the same address inside one area. It does not refuse a device addressed
  `1.0.1` sitting in line `2.3`. Real installations sometimes want exactly that during
  a rebuild, and the KNX rule that would forbid it is a rule about the running bus,
  not about the file. Keep an eye on it yourself.

### Moving a device

The **Line** select moves a device to another line, or to Unassigned. The **Building
part** select moves it to another part, or to none. Both act immediately, and both are
one undo step.

Selecting several devices — checkboxes in the group-address table, or `Ctrl`-click and
`Shift`-click in the tree — puts a bulk action bar in the strip below the header, with
a move-to-line select, a move-to-building-part select, and a delete button.

> **Warning**
>
> Moving a device never changes its individual address. A line move and a re-address
> are two separate actions in KNXBench, and doing one does not imply the other. After
> moving a device to a different line, check whether its address still says what you
> mean.

In the Project Explorer you can also drag an eligible **single device** onto a
line or building part in the first installation. The drop uses the same
validated, undoable move as the Inspector selects; it does not change the
individual address. The Inspector selects remain the keyboard-accessible way
to perform either move. Group-address links need an explicit send/receive
direction and cannot be created by dragging.

## Deleting

Delete never cascades. KNXBench refuses to delete anything that still holds something
else:

| Deleting | Is refused while |
| --- | --- |
| An area | It still has lines |
| A line | It still has devices |
| A device | Any of its communication objects is still linked to a group address |
| A building part | It still has child parts or devices in it |

Empty it first, then delete it. The refusal names the reason.

> **Warning**
>
> Deleting does not ask for confirmation, in the properties pane or in the bulk action
> bar. Undo (`Ctrl+Z`) is the safety net, and it works for deletes — but the undo
> history lives in the running application, not in the project file, and opening or
> importing a project clears it. If the deletion mattered, save before and after.

[Manual index](../README.md) · Next: [Working with group addresses](04-group-addresses.md) →
