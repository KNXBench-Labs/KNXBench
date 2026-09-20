← Previous: [Buildings and topology](03-buildings-and-topology.md) · [Manual index](../README.md)

# Working with group addresses

A **group address** is the address a KNX telegram is sent to. It is not a device. It is
closer to a topic: a switch sends to `0/0/1`, a lamp actuator listens on `0/0/1`, and
neither of them knows the other exists. Wiring a switch to a lamp in KNX means giving
their **communication objects** — the data endpoints a device exposes — the same group
address. [Group addresses](../knx-basics/03-group-addresses.md) explains the idea
properly; this chapter is about the table.

## The group-address view

Pick **Group addresses** in the navigation pane.

![The group-address table with columns Address, Name, Range, DPT and Links, listing
addresses 0/0/1 through 0/0/11 of the demo project, with export and import buttons for
CSV above it](../../assets/screenshots/porcelain-group-addresses.png)

Six columns, one row per address:

| Column | What it holds |
| --- | --- |
| (checkbox) | Selects the row for a bulk action |
| Address | The address, in the project's group-address style |
| Name | The address's name |
| Range | The group range that contains it, as `Main / Middle`, or `(no range)` |
| DPT | The datapoint type, derived from the linked communication objects |
| Links | How many communication objects send to it and how many receive from it |

The **DPT** column deserves a note. In the KNX model a group address does not carry a
datapoint type of its own — its linked communication objects do. So KNXBench derives
it: if every linked object agrees, that is the DPT; if none is linked, the column says
`none stated`; if two linked objects disagree, the column says `conflicting` and shows
both. It never picks a winner. See
[Datapoint types](../knx-basics/04-datapoint-types.md).

**The filter field** above the table matches the address, the name and the DPT, case
insensitively. It filters as you type, and selecting a range in the tree scopes the
table to that range and everything nested under it, with a back link to the full list.

Selecting a row shows a **Links** panel underneath the table: one line per linked
communication object, with its device, the object's name and number, the direction,
and an Unlink button.

## Creating and deleting addresses

Group addresses are created in the project explorer, under the **Group Addresses**
branch: type the address (`1/1/1`), a name, optionally pick a range from the select,
and press Enter or click Add. The same restriction as everywhere else applies — the
creation row appears under the first installation only.

Deleting is done from the properties pane, or from the bulk action bar for several
addresses at once. A group address that still has a communication object linked to it
cannot be deleted; unlink first. There is no confirmation dialog, and undo is the way
back.

## Group ranges

A **group range** is the box a set of addresses lives in — the "Lighting" main group
and its "Ground floor" middle group, in ETS terms. Ranges are created under the
**Group Ranges** branch of the tree: start address, end address, name. A range created
directly under the branch is a main range; a range created under a main range is a
middle range. KNXBench offers those two levels, matching what real projects use.

Ranges can be renamed and deleted from the properties pane. A range that still
contains addresses, or still has middle ranges inside it, is not deleted — the refusal
says which.

Ranges matter beyond tidiness for one reason: a group address that belongs to no range
cannot be written to a `.knxproj` file. See
[Projects](02-projects.md).

## Linking a communication object

Links are created from the device side, not from the table:

1. Select the device.
2. In the device panel, open the **Communication objects** tab.
3. Expand the object you want to link.
4. In the link row, pick a group address, pick **Send** or **Receive**, and click
   **Link**.

Unlinking works from either side — the link row on the communication object, or the
Links panel under the selected group address.

The direction is part of the link, not of the address: one object may hold both a send
and a receive link to the same address, and the **Links** column then counts it once
in each direction. [Devices and products](05-devices-and-products.md) covers the
communication object tab, including the six flags.

> **Note**
>
> There is no way to start a link from the group-address table, because choosing a
> communication object there would need a device-and-object picker that does not exist
> yet. Start from the device.

## The group-address style

A project uses one of three styles for the whole project:

| Style | Looks like | Range |
| --- | --- | --- |
| ThreeLevel | `1/2/3` | main / middle / sub |
| TwoLevel | `1/2` | main / sub |
| Free | `2051` | one number |

The style is chosen when the project is created, and it is shown — read-only — on the
Project node in the properties pane. The application has no control for changing it
afterwards. The server has a route for it, but nothing in the interface calls that
route, so treat the choice as made at creation time.

## Group addresses as CSV

Two buttons sit above the table, and the same two are in the File menu: **Export group
addresses (CSV)…** and **Import group addresses (CSV)…**. They exist so you can edit a
few hundred names in a spreadsheet without clicking through a few hundred rows.

> **Note**
>
> This is KNXBench's own format — "KNXBench group-address CSV v1". It is not ETS's
> group-address export, and no sample of ETS's own format exists in this project to
> check against, so nothing here claims the two interoperate.

### The columns

The header row is required, columns are matched by name (case-insensitively,
whitespace ignored), and their order does not matter.

| Column | Export | Import |
| --- | --- | --- |
| `Address` | The address in the project's style | Required. This is the row's identity. |
| `Name` | The name | Required, must not be empty |
| `Central` | `true` / `false` | Optional: `true`/`false`/`1`/`0`/`yes`/`no` |
| `Unfiltered` | `true` / `false` | Same as `Central` |
| `DatapointType` | Derived from the linked objects, empty if they disagree | Read, counted, never applied |
| `MainGroup` | The containing main range's name | Read, counted, never applied |
| `MiddleGroup` | The containing middle range's name | Read, counted, never applied |

The file is written as UTF-8 with a byte-order mark and CRLF line endings, so
spreadsheets open it without mangling umlauts, and it is read back with or without a
BOM, with either line ending, and with `,` or `;` as the separator — whichever your
spreadsheet produced. Unknown columns are reported by name, never silently ignored.

### What import does, and what it refuses to do

Each row ends as one of four outcomes: **created**, **updated**, **unchanged**, or
**error**. A row is an error when the address is missing, unparseable, out of range for
the project's style, or `0`; when the name is missing; when a boolean cell is spelled
in a way the reader does not recognize; or when the same address appears twice in one
file.

**One error anywhere means nothing is applied.** The report names every offending row,
counted the way a spreadsheet counts them, so you can fix the file and try again. A
half-applied import is not a thing that can happen here.

A successful import is a single undo step, however many rows it touched.

Three things import deliberately never does:

- **It never deletes.** An address that exists in the project but not in the file is
  left alone. The CSV is an edit, not a replacement.
- **It never re-addresses.** The address is the match key, so changing an address in
  the spreadsheet creates a second entry rather than moving the first. Delete and
  recreate instead.
- **It never creates group ranges.** A new address is placed in the innermost existing
  range that already contains it; if no range does, the address is created without one
  and the report says so.

There are no description or comment columns in either direction, because the domain
model does not carry those fields for a group address yet.

After an import, a summary appears as a toast: how many were created, updated and
unchanged, plus the number of warnings and ignored columns, and a pointer to the Log,
which holds the detail. If the file was rejected outright, the project is untouched.

The same exchange exists on the command line, including a dry run that prints the
identical report without writing anything — see
[The command line](10-command-line.md). The format's full specification is in
[Import and export](../../IMPORT_EXPORT.md).

[Manual index](../README.md) · Next: [Devices and products](05-devices-and-products.md) →
