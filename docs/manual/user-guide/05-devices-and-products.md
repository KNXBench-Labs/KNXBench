← Previous: [Working with group addresses](04-group-addresses.md) · [Manual index](../README.md)

# Devices and products

A KNX device is not much use to an engineering tool on its own. What makes it
configurable is its **application program**: the manufacturer's description of what the
device can do, which **communication objects** it exposes, and which parameters change
its behavior. That description comes from a **product database** — a file the
manufacturer publishes, not something KNXBench invents. See
[Products and product databases](../knx-basics/05-products-and-product-databases.md).

This chapter covers the catalog, adding a device from it, and the three tabs of the
device panel.

## The product catalog

Open it from the navigation pane (**Product catalog**), from the `+ Add device` row
under a line in the project explorer, or from the `+` button on a line in the Topology
view. All three open the same centre workspace; only the target line differs.

![The product catalog in the main workspace: an install file picker, manufacturer
filter, search and result, then a device name, quantity of three, and a preview
of the generated names and unassigned addresses. This is an illustrative local
UI fixture, not an installed manufacturer's data.](../../assets/screenshots/porcelain-product-catalog.png)

In an installed catalog a row can show `name (number) — description` (the
illustrative screenshot has only a name), and the same product can appear more
than once — one row per catalog item in the database, including
several entries that differ only in a number. KNXBench shows what the database
contains; it does not deduplicate on your behalf.

The manufacturer select and the search field combine: the search runs on the server
over the filtered set, a moment after you stop typing. Arrow keys move through the
results and Enter picks the highlighted one, which fills the name field below — picking
is not creating.

### Installing a product database

The file picker at the top of the workspace takes a `.knxprod` file, the package format
manufacturers publish. Installing reports what happened in one line: whether the
package was new or already installed, which scheme it used, and how many members,
unknown entries and conflicts it contained. The catalog list refreshes immediately
afterwards.

> **Note**
>
> Two limits are worth knowing before you go looking for a file. Legacy `.vd2` product
> data is refused with an explicit message rather than half-read, and a `.knxprod`
> whose contents are encrypted cannot be installed either. See
> [Known issues](../known-issues.md).

Product data can also come out of a `.knxproj`, because an ETS project carries the
product data of the devices in it. That route is command-line only:

```bash
knx products ingest project.knxproj --product-db products.db
```

See [The command line](10-command-line.md).

## Adding a device

1. Select the line the device belongs to, so the catalog knows where to put it.
2. Open the catalog.
3. Filter by manufacturer, search, and click the product.
4. Correct the name if you want — it is pre-filled from the catalog entry.
5. Set **Quantity** from 1 to 32 and check the preview. One device keeps the
   entered name; multiple devices are named `<name> 1`, `<name> 2`, etc.
6. Click **Create** (or press Enter in the name field).

The device appears in the line, with one communication object per communication-object
reference its application program declares.

The catalog occupies the centre workspace, not a modal dialog. Switching to a
normal project view and back preserves your filter, search and selection; closing
the catalog resets them. On narrow screens the navigation pane collapses after
you open the catalog so the work area is visible immediately; use **Navigation**
in the toolbar to reopen it. If no line is selected, devices stay unassigned.
For multiple devices the server performs **one atomic action**: a refusal adds
none, and one **Undo** removes the entire batch. Created devices and their
individual diagnostics remain visible for review until you close the catalog.
An older server that ignores the quantity may create only one device: the UI
refreshes the returned project, warns you and **does not automatically retry**.
If a batch request loses its response or the server reports an internal error,
its outcome cannot be confirmed from that response. The catalog asks you to
inspect or reload the project before another attempt rather than offering an
immediate duplicate-producing retry.

Two things it does not have:

- **No individual address.** Placement on a line does not reserve or invent a
  physical address, even if every device slot is already occupied. Assign an
  address separately in the properties pane. See
  [Buildings and topology](03-buildings-and-topology.md).
- **No name change afterwards.** Devices cannot be renamed in KNXBench yet, so the
  name you type in this workspace is the one you keep.

### Creation diagnostics

If anything about the product was less than perfectly clear, the catalog stays open and
lists it under **Creation diagnostics**. The device is created either way — the
diagnostics describe it, they do not block it. You will see at least one of them every
time, and it is the honest one:

```text
Dynamic and module activation was not evaluated for <program>; only static
product data was seeded.
```

In plain words: the device got every communication object its program declares, rather
than only the ones its parameter settings would actually switch on. A real device's
object list depends on its parameters, and that evaluation is not run at creation time.
Expect a fresh device to list more communication objects than the physical product
would show in ETS.

For a batch, each device's warning is labelled with its name/index. Other
diagnostics you may meet: a product with no application program (created without
communication objects), a datapoint type the database states as several alternatives
(filled with none, alternatives listed), and a reference the installed program does not
contain.

## The device panel

Select a device and the center of the workbench shows its panel: the name, the
individual address, and three tabs. `←` and `→` move between them, `Home` and `End`
jump to the first and last.

### Address and line

For a device already in a topology line, the **area.line** prefix is fixed by
that line; edit only the **device number** (1–255) and press Enter or leave the
field to apply the reconstructed address. A device not assigned to a line
instead offers the complete individual address. A duplicate address, a number
outside the range, or a line/prefix mismatch is rejected with a reason. An
imported mismatch is shown so you can repair it rather than being rewritten
without permission. If the imported topology lists one device in several
positions (including a repeated reference on one line, both a line and
the unassigned list, or two unassigned entries), or one line in multiple areas, this address control is disabled;
it does not guess which prefix wins. Repair the topology outside this editor
before moving such a device. Newly assigning device number `0` is not
supported until couplers can be identified reliably; existing imported `.0`
values are kept.

Line moves and address edits are separate actions: changing the line does
**not** renumber the device. To move an addressed device to a different line,
first clear its address, choose the target line, then assign a number under
that line's new prefix. An imported prefix mismatch can likewise be cleared
before moving. A refused move leaves the project unchanged. The line picker
currently supports devices in the first installation only. These controls
edit the project; they do not connect to, program or download a KNX device.

### Communication objects

![The device panel for 1.0.1 with the Communication objects tab selected, below the
Topology view. Object 0 is expanded: DPT `DPST-1-1` with a ProgramRef badge, a
Description field, the six flag checkboxes R W T U C I with T and C ticked, an existing
Send link to 0/0/1 with an Unlink button, and a row for adding the next
link](../../assets/screenshots/porcelain-device-tab-communication-objects.png)

One collapsible row per object, showing its number, its name, its datapoint type and
the group addresses it is linked to. Expanding a row gives you:

| Control | What it does |
| --- | --- |
| DPT | Sets the object's datapoint type |
| Description | Sets the object's description |
| Flags | Read, Write, Transmit, Update, Communication, Read on init (with standard letters) |
| Group links | One row per directional link, with Unlink; paired links can be unlinked together |
| New link | Pick an address, choose Send, Receive or Send + Receive, then click Link |

The six toggles retain their KNX letters — `R`, `W`, `T`, `U`, `C` and `I` —
and now show their full names alongside them. KNXBench stores what you set; it
does not second-guess whether a combination is sensible for the product.
Send + Receive creates **two separate links as one undoable edit**. If either
already exists, neither is added; you may unlink or edit the individual
Send/Receive links afterward. “Unlink both” likewise removes the pair in one
undo step; a refused paired unlink leaves the other links in their original
order, and Undo restores a removed link to its previous position. The
screenshot above predates the expanded flag labels and the paired-link
control; the current controls include both.

Five of them are written back on export. **Read on init (`I`) is not.** No ETS project
file this project has measured states that flag per communication object, so the
exporter leaves it out rather than invent a place for it — and tells you, with an export
warning counting the objects that had it switched on. The value stays in your `.knxdb`;
it just does not survive a trip through `.knxproj`.

This is the only place in the interface where a link between a communication object and
a group address is created. [Working with group addresses](04-group-addresses.md)
describes the view from the other end.

#### Where a value came from

Next to the datapoint type and the description you may see a small badge — `Program`,
`ProgramRef`, `Instance`, `Inferred` or `UserEdit`. It says which layer the shown value
came from: the application program's default, the program's per-variant override, the
value stored in the imported project, something KNXBench derived, or your own edit.

That badge is not decoration. On export, only `Instance` and `UserEdit` values are
written back to the file. Values that came from the product database stay in the
product database, and values KNXBench derived are never written back as though someone
had set them deliberately.

The same rule drives **product-data enrichment**, which runs at the end of an import
and once per device created from the catalog: for every communication object, slots the
project left empty are filled from the installed application program. Only genuinely
empty slots. A value the project file stated — even an empty or a malformed one — is
left exactly as it was, because the exporter has to be able to reproduce the file.
Nothing is guessed: a datapoint type stated as a list of alternatives fills nothing and
is reported instead.

If no product database is installed, the import simply says so and skips this step.

### Parameters

![The device panel with the Parameters tab selected. A Device section holds "Enable room
temperature control" set to "no"; below it two module sections named
M-00FA_A-2504-10-C071_MD-2_M-1 and _M-2, each with a Function select reading "Switching -
No Feedback" and "Dimming - No
Feedback"](../../assets/screenshots/porcelain-device-tab-parameters.png)

The tab loads the device's parameters when you select the device. What you get:

- **Sections.** One for the device itself, plus one per module instantiation —
  a device with twelve channels shows twelve sections, never one merged list.
- **Fields.** A select for a choice parameter, a number field with the program's own
  minimum and maximum for a numeric one, a text field otherwise. Values commit on blur,
  and Enter is a shortcut for that. A rejected value snaps back with the reason.
- **An issues banner**, collapsed, counting what went wrong while evaluating this
  device's program, with a Copy details button per entry.
- **Stale values**, when the project holds parameter values that no longer correspond
  to any parameter in the current program. They are shown separately, never merged into
  the field list and never quietly dropped.

Some fields are read-only, with a caption saying so. That is the honest answer to a
hard case: a module-scoped parameter can only be written when exactly one module
instance in the project is unambiguously its target. When two instances share one
reference, when a module carries no identifier, or when the project was imported by an
older KNXBench version that did not yet store module instance identifiers, the field
stays read-only and the diagnostics say which of those it is.

**What editing a parameter does.** It stores the value in the project, as a single undo
step, after validating it against the program's declared parameter chain. The server
then re-evaluates the program's dynamic tree with the new value, so fields and
communication objects that the new setting switches on or off change in the same
response.

**What it does not do.** It does not talk to any device. Neither the interface nor the
command line can program a device: there is no download command, no route on the
server, and no button. The commissioning protocol itself is being built inside the
KNX core library, but it has only ever been driven against a simulator this project
wrote, never against real hardware. A parameter you change here is a value in a file
until ETS, or some other tool, puts it into the device.

> **Note**
>
> Parameter interpretation is the least finished part of KNXBench, and the project
> documents it in detail rather than glossing over it: section 3 of
> [Known limitations](../../KNOWN_LIMITATIONS.md) is the full account. If a device's
> parameters look wrong, that document is the first place to check before filing a bug.

### Product data

![The device panel with the Product data tab selected. Under PRODUCT IDENTITY a badge
reads "From the product database"; the product reference, the application program
reference, the manufacturer "KNX Association", the product name "KX.tp (D4)" and the
order number are listed, and the expanded "More product data" disclosure below adds
manufacturer ID and catalogue
item](../../assets/screenshots/porcelain-device-tab-product-data.png)

This tab answers one question: which product does the project say this device is, and
did the installed database recognize it? The badge at the top right gives the verdict:

| Badge | Meaning |
| --- | --- |
| From the product database | The references were matched, details below |
| No product database | Nothing is installed here to match against |
| Not in the product database | Something is installed; it does not contain this product |
| No product reference | The project states no references at all |

Below it, the product reference and the application program reference are printed
verbatim in monospace — they are the manufacturer's identifiers, and an engineer
comparing them against a package needs the exact string. Then the manufacturer, the
product name and the order number, with the rest behind a **More product data**
disclosure, grouped as Product, Hardware and Application program: manufacturer ID,
catalog item and its number; hardware name, version and serial number; program name,
number, version, ID and mask version.

Fields the database has no value for are left out rather than printed as dashes, and
the disclosure ends by counting how many were omitted — so you can tell "the database
is silent here" from "this view only shows six fields".

> **Note**
>
> The hardware serial number is the one the manufacturer's package states for that
> hardware type. It is not the serial of the unit on your wall; KNXBench cannot read
> that at all.

The language of product texts follows the **Product data language** setting, separately
from the interface language. See
[Settings, themes and languages](09-settings-and-appearance.md).

[Manual index](../README.md) · Next: [A complete configuration workflow](06-configuration-workflow.md) →
