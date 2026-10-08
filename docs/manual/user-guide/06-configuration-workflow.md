← Previous: [Devices and products](05-devices-and-products.md) · [Manual index](../README.md)

# A complete configuration workflow

The previous five chapters described the workbench one surface at a time. This one
walks a small job from the first click to the last file, in order, so you can see how
the pieces fit. It uses the same project the screenshots come from: the fictional
"Sample house", an ETS-format project built by `tools/manual_sample_project.py` with an
invented manufacturer.

The job: add a presence detector to an existing line, give it an address, put it in a
room, and wire its switching object to a new group address — then get the result back
out of KNXBench.

**Prerequisites:** a running source build, a project with an existing line and room,
and a suitable installed product. For the exact fictional data used here, generate
`sample-house.knxproj` using `python3 tools/manual_sample_project.py <output-path>`
from a repository checkout and import it. Use a disposable copy, not a customer project.
The sample manufacturer and devices are invented. No bus connection is required.

**Expected result:** one additional project device, a group address and an explicit
communication-object link, saved as `.knxdb`. This changes the project file only.

Each step links to the chapter that explains it properly. If something surprises you,
follow the link rather than guessing.

## 1. Get the project in

Start with **Import ETS project** on the welcome screen (or **File → Open project…**)
for a `.knxproj`, or **Open KNXBench project** (**File → Open (.knxdb)…**) for a project
you have already worked on in KNXBench.

Importing a `.knxproj` is the slower path, and the progress banner names the phase it
is in. When it finishes, do not skip the next step.

See [Projects](02-projects.md).

## 2. Read the import report before you change anything

If the header strip shows **Import: N errors · M warnings**, click it. It takes you to
the Overview, where the counts are broken down; the Log holds one entry per finding.

This is worth five minutes. An import finding tells you what KNXBench did not
understand about the file, and that is exactly the part where later surprises come
from. Data KNXBench cannot interpret is preserved as-is and reported, not dropped, but
"preserved" is not the same as "editable".

See [Projects](02-projects.md).

## 3. Look at what you have

Switch between **Topology** and **Buildings** in the navigation pane. Topology shows
the electrical structure — areas, lines, and the devices wired to them. Buildings shows
the same devices arranged by floor and room, when the project says so.

The sample project has one area, "House", with line 1.1 "Ground floor" holding five
devices and line 1.2 "First floor" holding three. Its buildings side has a building
with two floors, rooms and a distribution board, so the Buildings view shows each
device in its room.

See [Buildings and topology](03-buildings-and-topology.md).

## 4. Add the device

Pick the line you want the device on — line 1.1 in this example — and choose
**+ Add device** under it in the project explorer. Use this explorer action for
the wizard; the `+` on a Topology card opens the catalog workspace instead.

The **Add device** wizard opens. Search for the presence detector, select it,
and choose **Next**. On **Placement**, check the installation and line and choose
a room if wanted. On **Name and quantity**, use quantity `1` and enable
**Assign free addresses on the line** if wanted. On **Review**, read the exact
name and address computed by the server, then choose **Create device**.
Choose **Open device** from the result page to continue.

Read the creation diagnostics if any appear. At least one always does: the device is
created with every communication object its application program declares, because
parameter-driven activation is not evaluated at creation time. That is normal, and it
means the object list you see is a superset of what a configured device would expose.
If the computed names or addresses changed after the preview, creation is
refused and a new preview appears. Check it again: the preview is neither a
reservation nor a whole-project revision lock. Other invalid placements can
still cause a normal refusal.

See [Devices and products](05-devices-and-products.md).

## 5. Give it an individual address

A new device has none, unless you ticked **Assign free addresses on the line** in the
wizard. Select it, and in the properties pane type the **Device number**: the area and
line part (`1.1.`) is fixed by the line the device sits on, so `30` gives `1.1.30`.

KNXBench refuses a duplicate address and a device number outside 1–255.

See [Buildings and topology](03-buildings-and-topology.md).

## 6. Put it in a room

In the same properties pane, pick a **Building part**. This is independent of the line:
it changes where the device appears in the Buildings view and nothing else. A device
can sit in a line and in no room at all.

## 7. Create the group address

Presence detection needs something to send to. In the project explorer:

1. Under **Group Ranges**, create a main range if the project has none that fits, and a
   middle range inside it.
2. Under **Group Addresses**, type the address, a name, and select the range.

The range is not cosmetic: a range is where the address belongs in the project's own
structure, and the group-address views and the CSV export both read it.

See [Working with group addresses](04-group-addresses.md).

## 8. Link the communication object

Select the device, open the **Communication objects** tab, and find the object you
want — the summary row shows its number, its name and its datapoint type.

Expand it, then in the link row pick the group address, pick **Send** (the detector
transmits) or **Receive**, and click **Link**. On the actuator's side, the object that
switches the light gets a **Receive** link to the same address. Two objects, one
address, opposite directions: that is a KNX connection.

While you are in this tab, check the datapoint type and the six flags. The DPT decides
how the value is interpreted; a badge next to it says whether the value came from the
product database, from the imported project, or from you.

See [Devices and products](05-devices-and-products.md) and
[Working with group addresses](04-group-addresses.md).

## 9. Check the result in the table

Switch to **Group addresses**. The new address should show its range, its datapoint
type, and a Links column counting one sender and one receiver. Select the row and the
Links panel below names both objects and their devices.

If the DPT column says `conflicting`, two linked objects disagree about the type.
KNXBench will not pick a winner for you; fix it on the device side.

If you have a lot of naming to do, export the table as CSV, edit it in a spreadsheet,
and import it back. Ordinary upsert rows do not delete omitted addresses or move
them. Explicit `readdress` and `delete` actions require a separate confirmed preview;
any invalid row prevents the whole import from being applied.

See [Working with group addresses](04-group-addresses.md).

## 10. Save

**File → Save** writes the `.knxdb`. The first Save of a project that has no file yet
becomes a Save As; after that, autosave (on by default, every five minutes) keeps
writing to the same file while you work.

`.knxdb` is the supported working format. It stores the model and retained opaque
source data. Preserve your original `.knxproj` and independent backups too; an
import report can still identify unsupported semantics.

See [Projects](02-projects.md).

## 11. Hand the work on

Four exits, for four different purposes:

| You want | Use |
| --- | --- |
| To keep the project | **File → Save** — the `.knxdb` keeps everything |
| A list of group addresses for a spreadsheet | **Export group addresses (CSV)…** |
| Something to read, print or archive | The documentation export |
| A `.knxdb` copy on your own computer (web build) | **Export project…** |

There is no `.knxproj` export. It existed until 2026-09-20 and was withdrawn
([ADR-0028](../../adr/0028-no-knxproj-export.md)): the archives it wrote were unsigned,
no real ETS installation was ever available here to confirm they opened, and a file
KNXBench cannot promise ETS will read is not one it should write. Import is one-way;
keep the `.knxdb`, and keep the `.knxproj` you imported if a colleague needs ETS's own
format.

For the documentation export and for comparing two projects, see
[Documentation export and project comparison](08-reports-and-diff.md).

## 12. Where this workflow stops

Everything above happens in files. Loading the configuration into the devices on the
wall is a separate, guarded step, and KNXBench covers it only narrowly so far.

`knx device download` and the **Download to device** tab write a project device's
application tables and parameters. Both show the plan first and write only after a
confirmation for that one device. This has been verified on one device so far; other
devices, application versions and masks have no such evidence, and procedures KNXBench
cannot plan are refused by name rather than guessed. Programming an individual address
is currently blocked until durable recovery exists. The details, including what is
refused, are in [Bus monitor and KNXnet/IP](07-bus-and-interfaces.md#downloading-to-a-device).

Stop this tutorial after saving and checking the file. A hardware download needs
its own supported target, recovery plan and explicit confirmation. There is no
`.knxproj` export to hand this edited project back to ETS; that is a deliberate
format boundary, not the next step of this exercise.

### Common mistakes and final checks

- **Empty catalog:** install product data first, or import the generated sample.
- **Wrong location:** a line is electrical placement; a room is building placement.
  Both must belong to the same installation.
- **DPT conflict:** compare the address's declaration and the linked objects before
  changing a type. Matching payload lengths alone are not sufficient.
- **No hardware change:** editing a device number in Properties does not program
  that address into the physical device. A project edit and a bus write are different jobs.
- **Proof of completion:** reopen the saved `.knxdb` and check the device, address
  and link. If you need a browser-local copy, use **Export project…** as well.

[Manual index](../README.md) · Next: [Bus monitor and KNXnet/IP](07-bus-and-interfaces.md) →
