← Previous: [Devices and products](05-devices-and-products.md) · [Manual index](../README.md)

# A complete configuration workflow

The previous five chapters described the workbench one surface at a time. This one
walks a small job from the first click to the last file, in order, so you can see how
the pieces fit. It uses the same demo project the screenshots come from: an imported
ETS project called "KV v2.5 - demo".

The job: add a presence detector to an existing line, give it an address, put it in a
room, and wire its switching object to a new group address — then get the result back
out of KNXBench.

Each step links to the chapter that explains it properly. If something surprises you,
follow the link rather than guessing.

## 1. Get the project in

Start with **File → Open project…** for a `.knxproj`, or **Open (.knxdb)…** for a
project you have already worked on in KNXBench.

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

The demo project has two areas: area 0 with an empty line, and area 1 with line 1.0
holding four devices. Its buildings side is one building part, so the Buildings view is
quiet.

See [Buildings and topology](03-buildings-and-topology.md).

## 4. Add the device

Select the line you want the device on — line 1.0 in this example — then click
**+ Add device** under it in the project explorer, or the `+` on the line in the
Topology view.

In the catalog, filter by manufacturer, search for the product, click the row, check
the name, and press Enter.

Read the creation diagnostics if any appear. At least one always does: the device is
created with every communication object its application program declares, because
parameter-driven activation is not evaluated at creation time. That is normal, and it
means the object list you see is a superset of what a configured device would expose.

See [Devices and products](05-devices-and-products.md).

## 5. Give it an individual address

A new device has none. Select it, and in the properties pane type the address in
`area.line.device` form — `1.0.5` for the fifth device on line 1.0.

KNXBench refuses a duplicate address. It does not check that the address matches the
line the device sits in, so read what you typed once more.

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
and import it back. The import never deletes, never re-addresses, and refuses the whole
file if any row is wrong.

See [Working with group addresses](04-group-addresses.md).

## 10. Save

**File → Save** writes the `.knxdb`. There is no autosave, and the first Save of a
project that has no file yet becomes a Save As.

`.knxdb` is the format to keep. It is KNXBench's own working format, it holds
everything, and nothing is lost on the way in or out.

See [Projects](02-projects.md).

## 11. Hand the work on

Three exits, for three different purposes:

| You want | Use |
| --- | --- |
| To keep the project | **File → Save** — the `.knxdb` keeps everything |
| A list of group addresses for a spreadsheet | **Export group addresses (CSV)…** |
| Something to read, print or archive | The documentation export |

There is no `.knxproj` export. It existed until 2026-09-20 and was withdrawn
([ADR-0028](../../adr/0028-no-knxproj-export.md)): the archives it wrote were unsigned,
no real ETS installation was ever available here to confirm they opened, and a file
KNXBench cannot promise ETS will read is not one it should write. Import is one-way;
keep the `.knxdb`, and keep the `.knxproj` you imported if a colleague needs ETS's own
format.

For the documentation export and for comparing two projects, see
[Documentation export and project comparison](08-reports-and-diff.md).

## 12. Where this workflow stops

Everything above happens in files. The last step a KNX installation actually needs —
loading the configuration into the devices on the wall — is not part of KNXBench.

KNXBench does not commission devices. There is no download, no programming-mode
handling, and no way to transfer an application program or a parameter set to real
hardware, in the interface or on the command line. The protocol work exists in the
core library and has been driven against a simulator this project wrote; it has never
addressed a real device, and it is not wired to any button.

What KNXBench can do on a live bus is watch it and send single group values from the
command line, which is a diagnostic tool, not commissioning. See
[Bus monitor and KNXnet/IP](07-bus-and-interfaces.md).

So the honest end of this workflow is: configure here, save the `.knxdb`, export a
`.knxproj`, and commission with a tool that can. That is the gap between KNXBench and a
complete replacement for ETS, and it is the one the project is still working on.

[Manual index](../README.md) · Next: [Bus monitor and KNXnet/IP](07-bus-and-interfaces.md) →
