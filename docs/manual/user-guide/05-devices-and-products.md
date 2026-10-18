← Previous: [Working with group addresses](04-group-addresses.md) · [Manual index](../README.md)

# Devices and products

**Goal:** install product data, add a device and edit its project configuration.
**Prerequisites:** an open project and an authorised product package, or product data
from an imported project. The fictional sample is enough for practice.
**Expected result:** a device in the explorer, with its application objects and parameters.
**Watch out:** installation diagnostics matter; an offline editor does not establish
download compatibility. Legacy VD support is partial and offline-only.

A KNX device is not much use to an engineering tool on its own. What makes it
configurable is its **application program**: the manufacturer's description of what the
device can do, which **communication objects** it exposes, and which parameters change
its behavior. That description comes from a **product database** — a file the
manufacturer publishes, not something KNXBench invents. See
[Products and product databases](../knx-basics/05-products-and-product-databases.md).

This chapter covers the catalog, adding a device, and the device panel's objects,
parameters, product data, diagnostics and inspection-only manufacturer fields.

## The Devices view

Open **Devices** in the left navigation, run **Devices** from the command
palette (Ctrl+Shift+P), or click the device count in Overview. The flat table
shows every canonical device once, including unassigned/unplaced devices.
It separates installation/KNX-line placement from complete building/room paths;
a device listed in both branches is not counted twice.

Filter by name, individual address, description, manufacturer, product, order
number or placement. Click a column heading to sort it; click again to reverse
it. Individual addresses sort numerically. The wide table scrolls locally on
small screens rather than widening the whole page.

Click a row/name/address to open the existing device editor **as the central
view**, with Properties still on the right. **Back** returns to the view you
came from; **All devices** opens the table. Filter, sort, scroll and checkbox
selection are kept during an editor visit, but are not stored in the project.
Checkboxes and Ctrl/Meta/Shift clicks use the existing bulk actions. Shift-click
uses only the filtered/sorted table order. Removed targets are pruned when the
project snapshot changes.

Manufacturer/product/order data is resolved by one read-only batch, not by
opening every device's parameter editor. Missing references, a missing product
database, an uninstalled product and a failed lookup are named distinctly.
**Refresh product data** repeats the read. Untranslated product text carries the
existing language-fallback badge. A failed product query still returns canonical
device identities; an unreachable/malformed batch leaves the current tree rows
visible with an error. Unplaced devices require a successful identity-batch read.
Resolution is not application or download compatibility.

Device links also appear at group-address participants and in their inspector,
monitor source addresses, line-scan results, readiness rows and explicit **Open in
device editor** actions in the download/address/inspection panels. Address-only
links need exactly one current-project match; missing or ambiguous matches remain
plain text with a hint. They do not start a scan, tunnel, download or bus write.
The monitor keeps collecting/polling while its editor visit hides it.

This package does not add URL deep links, browser Back/Forward, switchable table
grouping, free-text log parsing or project-diff links. The separate diagnostic
companion does not become another editing window. See the
[contract and evidence](../../DEVICE_NAVIGATION.md).

![Alpha 9 in Graphite: the filterable Devices overview with fictional products, individual addresses, KNX-line placement and the selected device’s properties](../../assets/screenshots/graphite-devices.png)

The Devices view first shipped with Alpha 6 and remains in every later release. The older
Alpha 5 AppImage does not include it.

## Comparing the project with devices (P4, source build after Alpha 11)

**Included in Alpha 12:** both table and named-parameter comparison packages.
Most earlier manual screenshots retain their stated capture version. The new
website/README stills show the Alpha 12 candidate. This adds no new physical-device
verification; [source acceptance](../../evidence/p4-device-comparison-parameters-2026-10-15.json)
remains separate from release acceptance.

In **Devices**, use **Compare** on a row or **Compare all shown**. The latter
uses exactly the current filtered/sorted list, **not** checkbox selection.
Filtering for a KNX line compares that line's shown devices; changing the
filter after opening review does not change the reviewed list. Choose a
gateway and confirm the **read-only** operation once for the whole list.
Opening the review alone contacts no device.

The dialog shows completed/total and the current device. A device error is
reported and the next device is tried. **Cancel** waits for the current
bounded read to disconnect and then stops; it does not contact the remaining
devices. A tunnel occupied by Bus monitor is explicitly refused: stop that
monitor before trying again. This is a comparison, not a download or backup;
no key, phrase or write permission is sent.

Tags are session-only: **Synced**, **Different**, **Outdated**, **Wrong
application**, **No answer**, and grey **Not comparable** with its reason.
Without an attempted comparison a row has no tag. Click a tag to inspect its
result without contacting the bus. Relevant P3 content edits, undo/redo and
known affected write settlement make old readings outdated; names, DPT labels
and manual programming marks alone do not. Reloading/reopening has no cached
comparison claim. An external tool may change the device without notifying
KNXBench: a tag records a reading, not continuous monitoring.

The result table shows group-address entries and associations, resolving each
side's associations through its own address table. Parameter rows then show
name/reference, bit position/width, **project value** and **device value** for
actually written active fields. Supported numeric fields exclude neighboring
bits; text follows the program's already-supported encoding. Missing/masked,
overwritten or unsupported fields show **Unknown / not read** with a reason,
not a guessed zero or empty string. Negative signed device patterns remain
unknown; negative signed project values still prevent a plan. No enum label or
engineering unit is inferred. Every raw read range remains underneath, always
visible rather than collapsed or filtered. Wrong application is checked before
plan memory is read, using
the existing verified application identity; missing/short/ambiguous identity
is Not comparable, not a guessed mismatch. Unread, masked, malformed or
unattributable fields stay unknown with a reason. Raw bytes are never hidden.
A byte match with an unloaded compared part does not earn Synced. At narrow
widths scroll the comparison table horizontally to reach the device value
and memory columns; long target lists also scroll within their list.

[Exact scope and limits](../../contracts/device-comparison.md),
[device checks](07-bus-and-interfaces.md#device-checks-offline-readiness-then-an-optional-read-only-comparison).

## Programming marks (P3, source build after Alpha 11)

The **Programming** column in Devices shows the parts the project still needs
sent to each device: **Adr** (individual address), **App** (application),
**Par** (parameters) and **GA** (group-address connections). Only pending parts
have chips; an empty cell means none are pending. A muted dash means the server
has not reported this state, not that the device is programmed. **Pending only**
hides devices with no reported pending parts. Click Programming to sort by the
number of pending parts. These controls are included in Alpha 12; the
release screenshots above do not show them.

An amber dot beside a device in either explorer branch means at least one part
is pending. Hover the dot or a chip group for the full part names. Selecting the
device shows all four states under **Programming** in Properties.

The starting state comes from the ETS project's imported loaded flags. Editing
an individual address marks Adr; a parameter value marks Par; linking/unlinking,
changing an object's effective flags or renumbering/deleting a linked group
address marks GA on each affected device. DPT choices, names and descriptions
do not count. A mark is in the edit's same undo step. The application reference
is part of App tracking, but this feature adds no application-swap editor.

A finished full download clears all four parts; a partial download clears only
the selected parts it writes. Programming an individual address clears Adr.
An interrupted write marks the parts it may have touched pending, even if they
were previously clean; parts not reached stay as they were. An unconfirmed
closing restart keeps its separate warning: it does not undo verified memory
readback. Edits made while the operation runs stay pending if they differ from
what was sent. Reassigning the target or opening another project cannot mark a
different project device programmed.

A device write is not an undo step. Undoing an earlier edit after programming
makes its affected part pending rather than claiming the old value is on the
device. Native Save/Open and recovered working history preserve the marks and
undo behavior without a storage migration. Recovered history uses whole
snapshots: after a later device write, undoing one can conservatively show
additional pending parts even where their values did not change. This is not
proof of a device difference. Direct DPT/name edits do not add marks; do not
repeat a physical write merely to tidy this conservative project status.

For work done outside the server workbench (ETS, the separate CLI or another
tool), use **Mark as programmed** or **Mark as not programmed** in Properties.
Each action affects all four parts of this device, is undoable, and leaves a
log entry. It does not contact the bus. If another edit makes the displayed
snapshot stale, marking refuses and explains why; refresh before trying again.

These marks are the **project's belief**, not a live synchronization test or
proof that a device is running. The separate project/device comparison (P4) is
not part of this feature. Existing programming support and recovery refusals
still apply. A failed display refresh warns you to reload the project; do not
repeat a physical write just to refresh its marks. A storage error during
settlement is logged and must be resolved before trusting persisted flags.
[Technical rules and boundaries](../../adr/0129-per-part-pending-programming.md).

## The product catalog

Open it from the navigation pane (**Product catalog**) or from the `+` button on a
line in the Topology view. Both open the same centre workspace; only the target line
differs. The `+ Add device` rows in the project explorer open the
[add-device wizard](#the-add-device-wizard) instead.

![Alpha 9’s Device catalog in the main workspace: an install file picker,
manufacturer filter and search, and four fictional products. The first list row
is highlighted, but no creation form or address preview is open](../../assets/screenshots/porcelain-product-catalog.png)

A row shows `name (number) — description`, as in the screenshot, and the same
product can appear more
than once — one row per catalog item in the database, including
several entries that differ only in a number. KNXBench shows what the database
contains; it does not deduplicate on your behalf.

The manufacturer select and the search field combine: the search runs on the server
over the filtered set, a moment after you stop typing. Arrow keys move through the
results and Enter picks the highlighted one, which fills the name field below — picking
is not creating.

### Installing a product database

The file picker at the top of the workspace takes a `.knxprod` file, the package format
manufacturers publish. *(Since Alpha 14:)* it also takes the ZIP many manufacturers
actually ship, with one or more `.knxprod` packages and manuals inside. The preview
then lists every package in the ZIP and every other file that is **not installed**
(with its size); the packages install together or not at all, each unchanged. A ZIP
whose package is too large for the default bounds is refused with that package's
name; install it with `knx products ingest <file>.zip --allow-large-package`. Installing reports what happened in one line: whether the
package was new or already installed, which scheme it used, and how many members,
unknown entries and conflicts it contained. The catalog list refreshes immediately
afterwards.

### Updating with a preview (since Alpha 10)

Got a newer `.knxprod` from a manufacturer? In Alpha 10 and 11, **Update with
preview…**, under the install picker, checks it first. *(Since Alpha 12:)* there is one picker, and every `.knxprod` it takes, in the catalogue and
in the device wizard alike, is previewed first; an old ETS3 product database still
goes straight to its own import. The command-line equivalent is
[`knx products update`](10-command-line.md#knx-products-update--check-a-newer-product-file-before-installing-it).

- The preview says **Nothing is installed yet.** and lists each application program
  in the file as **New**, **New version** (with the versions you already have) or
  **Already installed**, plus the package's SHA-256, how many products and links it
  adds, and any ID conflicts.
- **Back up and install** first saves a copy of your whole product catalogue next
  to it, then installs. The message names that backup file.
- If anything changed in between (another install, another window), nothing is
  installed and **Preview again** checks the same file afresh.
- Your projects keep the exact program versions they were built with; a new version
  is added beside the old one and no device is switched to it.

Open **Catalogue backups** to see the backups, newest first, with size and date.
**Restore…** asks before it does anything: **Keep the current catalogue** is the
default, and **Restore backup** returns the catalogue to that state, removing what
was installed after it. Devices in your projects keep their references; one whose
program is not in the backup shows no parameters until the program is installed
again.

*(Since Alpha 12:)* **Delete…** removes a backup you no longer need,
after asking: **Keep the backup** is the default and **Delete backup** removes that
file for good; the catalogue in use stays as it is. KNXBench deletes exactly the file
it listed: if a file with that name changed in the meantime, nothing is deleted and
the list is read again. It never deletes backups on its own. (Alpha 10 and 11 cannot
delete backups, and their plain **Install product database** picker installs
directly, without a preview or backup.)

### Old ETS3 product databases (`.vd3`, `.vd4`, `.vd5`)

The same file picker also takes an old ETS3-era product database. Most of these
files are encrypted: KNXBench then asks for the password. KNXBench ships no password
and never guesses one. Tick **Remember this password** and the server keeps that one
password, in a file only its own user can read (`~/.config/knx/legacy-vd-password`,
mode 0600), so later imports need none. A second remembered password replaces the
first. **Settings → Legacy product database password** shows whether one is
remembered and forgets it.

The report lists the application programs, the catalog entries, parameters,
communication objects and translations, and every import note. Devices from such a
file can be placed, parameterised, linked to group addresses and, since October
2026, downloaded: KNXBench builds the download from the file's own load procedure,
checked against ETS's conversions of the same programs. No such download has run on
a real device yet, so the readiness check shows these devices as *untested* and a
download asks you to accept that. Their objects carry no data point type until you
set one. A large
`.vd5` (the measured Siemens database is 67 MB with 88 programs) takes 20–40 seconds
and over 1 GiB of server memory to import; its import notes are grouped by kind, so
the report stays readable with close to 1,800 of them. See
[ADR-0094](../../adr/0094-legacy-exim-product-files.md).

> **Note**
>
> Two limits are worth knowing before you go looking for a file. Legacy `.vd2` product
> data is refused with an explicit message rather than half-read, and a `.knxprod`
> whose contents are encrypted cannot be installed either. See
> [Known issues](../known-issues.md).

Normal project import also ingests the product data carried inside a `.knxproj`.
To ingest that data separately from a project editing session, use the CLI:

```bash
knx products ingest project.knxproj --product-db products.db
```

See [The command line](10-command-line.md).

## The add-device wizard

![The real add-device wizard selecting a product, reviewing placement and an allocated project address, then confirming one created fictional actuator](../../assets/workflows/add-device.gif)

*A project edit only. The clip uses the fictional Sample house and does not talk to hardware.*

The wizard is a guided path to the same result as the catalog, in five steps:
**Product**, **Placement**, **Name and quantity**, **Review** and **Result**.
Open it from:

- a `+ Add device` row in the project explorer: under a line (that line is
  chosen), under **Unassigned** of any installation (that installation, no line),
  or under a room (that room is chosen);
- **Add device…** in the command palette (Ctrl+Shift+P), aimed at the selected line
  or building part if there is one;
- **Add devices now** at the end of the new-project wizard (the new line 1.1);
- **Add with wizard…** in the catalog, which starts at **Placement** with the product
  selected there.

| Step | What you decide |
| --- | --- |
| Product | Search the installed catalog, or install a `.knxprod` package first. An installed package stays installed even if you cancel. |
| Placement | Line and building part, each optional; the installation only when the project has more than one. Line and building part must belong to the same installation. |
| Name and quantity | Name, 1 to 32 devices, **Assign free addresses on the line** (only with a line) and **Keep names unique**. |
| Review | The server computes, on a copy of the project, exactly which names and addresses would be created, with the product diagnostics. Nothing changes yet. |
| Result | What was created. **Open device** selects the first one; **Add more of this product** goes back to the naming step. |

**Create** sends the previewed names and addresses along. If the server would now
compute different names or addresses, for example because another window took an
address, it refuses with nothing created. This does not lock the whole project:
other edits may leave that expectation unchanged, and deleted placements can
cause an ordinary validation refusal. The wizard shows a fresh preview after a stale-name/address refusal to
confirm. Creation and placement are **one undo step**. Parameters and group links are
not part of the wizard; edit them in the device panel afterwards.

![The real wizard's Review step lists Practice actuator at its computed address before the Create device button is pressed](../../assets/screenshots/porcelain-device-wizard-review.png)

Check the computed address, not just the green-looking button. A preview describes
the current project; it does not reserve a bus address or confirm hardware compatibility.

**Cancel** or **Escape** asks before discarding a choice; a second **Escape** keeps
you in the wizard. A lost response is treated like in the catalog: **Retry safely**
resends the same request id while the same server is running and never creates a
duplicate.

The new-project wizard's optional seed is the project's initial structure,
not a separate undo step. Device creation afterwards is undoable; neither
wizard programs physical devices.

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

Two things to decide before creating:

- **Address allocation is optional.** With a target line, **Assign free addresses
  on the line** previews and assigns available project addresses. Without that
  choice the device remains unaddressed. This does not program physical hardware.
  You can also assign an address separately in Properties. See
  [Buildings and topology](03-buildings-and-topology.md).
- **Names can be changed afterwards in current source.** Edit **Name** in the
  device editor or Properties, or focus the device in Explorer/the Devices list
  and press **F2** or choose **Rename** from its context menu. This names the
  project instance, not its product/application, and never programs hardware.
  The change is newer than alpha.6.

### Renaming a device

Enter or leaving the edited Name field applies once; Escape cancels. Names may
repeat and keep intentional spaces/Unicode exactly. New names must not be blank,
contain control characters/line breaks, or exceed 1,024 Unicode code points.
A failed draft stays visible with its error. **Refresh state** reads the current
project without writing; review the retained draft before **Apply name**. Stale
project/revision/name contexts are refused, not silently overwritten. An unchanged
name produces no undo step. Undo restores the exact original name, including
imported exceptions; native save/reopen retains the name and existing history.
See [the naming contract](../../contracts/project-name-editing.md).

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

### When a device's application program cannot be found

The **Parameters** tab says why it cannot show anything:

- **No product database** on this server: nothing can be loaded at all.
- **This project's product data is not installed** (with the number of missing
  files): the project was saved by KNXBench alpha.11 or earlier, and the product
  database it was made with is gone (a rebuilt Docker container, another computer).
  **Restore from original .knxproj…** reads the project's original ETS export and
  installs exactly the manufacturer files whose checksum matches. Files that
  differ are listed and not installed. A password-protected export needs no
  password, because its manufacturer files are not encrypted. The project itself
  does not change. Save it afterwards, and its file carries the programs from then on.
- **The application program is not in the product database**: the device comes
  from a product package that was never installed. Install its `.knxprod` from the
  catalogue.

A project saved with a current build carries its application programs itself and
installs them again when it is opened, so this only affects older files
([ADR-0128](../../adr/0128-native-project-files-carry-their-manufacturer-files.md)).

## The device panel

Select a device and the center of the workbench shows its panel: the name, the
individual address and tabs for objects, parameters, product data, diagnostics
and manufacturer fields. `←` and `→` move between tabs, `Home` and `End`
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

#### Device functions (source builds after Alpha 12)

**Included in Alpha 13:** the device-mode controls, parameter groups and supported
atomic object refresh described here. Unsupported module construction still
refuses the complete edit; this does not add programming support.

**Source builds after Alpha 13:** expand a communication object and use its
manufacturer function dropdown below the **DPT** field. The separate **Device
functions** disclosure is removed. Only active supported enum parameters mapped
to this exact object's declared reference are offered; names and object numbers
are not used to guess a mapping. Module-local or ambiguous mappings are not
inferred. If no supported choice is available, the editor says so and points to
**Parameters**. If the object list is empty, enable the button and select its
operating mode under **Parameters** first.

Select only options the installed product declares. A selection saves immediately
and refreshes the parameter panel and object list. Pending object controls are
disabled. An ordinary refusal restores the last committed choice and shows its
reason; a stale-write conflict keeps the draft for explicit retry. In the
published Alpha 13 build these controls remain in **Device functions** above the
object table; existing screenshots still show released UI.

These are real project parameters, not an editable **Function** caption or a
free-text `FunctionText` override. Only active, supported enum controls structurally
governing communication-object branches are offered here. Other settings remain
under **Parameters**. No manufacturer/product is hard-coded, but unsupported
scripts and formats do not become supported just because a dropdown exists.

When a supported edit activates an object reference missing from the imported
project, KNXBench appends that object. Existing IDs, overrides, group-address
links and inactive instances stay intact. The parameter and new objects share
one **Undo** step and survive native save/reopen. Ambiguous or malformed declared
DPTs remain unset and are reported in Diagnostics, including after a panel
reload, until a known explicit type is supplied. They are never guessed. Missing module-scoped objects
currently refuse the whole edit if their instance/number mapping cannot be
established; no partial parameter change is saved. This edits the offline project,
not the physical button, and does not initiate a download. Ambiguous object
references are likewise reported in Diagnostics and refuse the whole edit; a
short catalogue ID is not blindly expanded into another object. Existing raw
references remain intact.
[Detailed scope](../../contracts/parameter-object-refresh.md).

**Alpha 9:** the DPT field is now a
dropdown, not a free-text input. Each known option shows its number and the name
in the UI language, for example **9.001 – Temperature / Temperatur** or
**17.001 – Scene number / Szenennummer**. Choose a value, then leave the field
or press Enter to apply it. An unknown imported reference stays selected and
unchanged until you explicitly choose a different value. **No project DPT**
clears the project value; if a program default exists, that option names it but
does not save it as your edit. Source badges now belong to the DPT/description
heading and explain their origin, e.g. **Source: application program / Quelle:
Applikationsprogramm**. Catalogue names do not establish device or codec support.
[Scope and evidence](../../contracts/dpt-selection.md).

![Alpha 9 in Graphite: the communication-object editor’s named 1.001 – Switch choice, application-source badge and exact stored DPST-1-1 reference](../../assets/screenshots/graphite-dpt-selection.png)

*The name helps you choose. The stored reference and source badge explain what is
actually in the project; a friendly label is not a new codec.*

**Also available in Alpha 9:** the object
panel now has aligned **No., Name, Function, DPT, Group addresses, Status** headings,
a **By channel / All objects** switch (flat view adds Channel), search and filters.
Click a heading to sort ascending/descending, within channels or globally in flat
view. **Original order** resets sorting; **Reset filters** leaves view/sort alone.

Search covers object number/name/function/description, channel labels, effective
DPT ID/text and every linked GA name/address; slash or dotted GA input works.
Description-only matches show a hint. Status, linked/unlinked and DPT family/exact/
missing filters combine. All states are initially included; unresolved-target
links count as linked. Matching channels open initially, zero-result groups hide,
and counts show matches/total regardless of collapse. Reset restores your original
channel expansion; reapplying a filter reveals matches again.

The open editor stays visible if its object leaves the result, marked **Editing —
outside filter**, without inflating counts. Collapsing a channel hides all of its
objects, open editors included *(since Alpha 12; Alpha 11 and earlier
kept an open editor visible under a collapsed channel)*; expanding it again shows
them as you left them. Inputs/refusal/pending state survive sort/filter/view/channel
changes and collapsing; only committed data drives results. Removing an
edited object announces and closes that editor. Tabs/refresh retain view settings;
device/project change or leaving the editor resets them. No view state is stored
in the project. Editing still saves DPT/description on blur/Enter and flags/links
immediately: clicking a filter from a dirty field can trigger that existing blur
save, but filtering adds no write of its own. Pending edits prevent duplicate
starts; a delayed result cannot republish into a removed editor/another device.

The table scrolls locally, keeping all columns reachable on narrow displays.
[Full contract and evidence](../../COMMUNICATION_OBJECT_TABLE.md). The
[earlier source screenshot](../../evidence/communication-objects/real-demo-editor.png)
remains historical evidence; the workbench views shown here are Alpha 9.

![Alpha 9 in Porcelain: device 1.1.1 with Communication objects selected,
Channel A open and object 0 expanded. The DPT selector shows 1.001 – Switch with
an application-source badge; the original reference remains DPST-1-1. Table
columns scroll within the workspace](../../assets/screenshots/porcelain-device-tab-communication-objects.png)

One collapsible **channel group** per evaluated channel, initially closed. The
application program's evaluated ownership determines grouping; neither the
object's name nor a printed channel number is used to guess it.
An evaluated channel shows its translated product **Text** when present and
its source **Name** as a separate fact. If Text is absent, the source Name
becomes the heading; if both are absent, the group says **Untitled channel**.
The source **Number** appears beside either heading as text (it need not be a
decimal number). Name and Number are shown as supplied, not translated,
reformatted, used as keys, or joined into an invented label.
Some products write a placeholder such as `{{0:Page 1}}` into a heading or an
object name, meaning "the text you typed into this text parameter". KNXBench
fills it in: the device's value of that parameter, otherwise the product's
default, and the placeholder's own text (`Page 1`) when the value is empty.
Hover the heading or name to see the product text and where the inserted text
came from; a placeholder that cannot be filled stays visible, with the reason
in the same tooltip. Names you edited yourself are shown exactly as you wrote
them. The documentation report prints the same filled names; group-address
member lists, the Log and CSV exports still show the stored text
([details](../../contracts/positional-text-templates.md)).
A channel-independent block gets its own group. Objects without evaluated
channel ownership — including inactive, uncertain and not-evaluated objects —
remain visible under **Without evaluated channel**, not silently discarded.
Opening a group and then an object row is possible by mouse or with Enter on
the focused headings. Expanded groups stay open when the same device's detail
refreshes and reset when you select another device.

Each object row shows its number, name, product function text when available,
its numbered/localized datapoint type, original reference and separate master-data
DPT description when available, its activation
state, and linked group addresses. **Active**, **Inactive**, **Undetermined**
(cannot decide from the available product/parameter evidence), and **Not
evaluated** (no usable program evaluation) mean different things. The project's
stored `IsActive` claim remains separate; if it disagrees with an evaluated
active/inactive state, the row says **Stored active** or **Stored inactive**.
No status silently rewrites the imported project. An object whose project DPT
is empty can show a **Program default** DPT for reference; opening the row
still leaves the project DPT unset until you explicitly select one. An unknown
reference stays verbatim; an absent/ambiguous type gets no invented name.

The screenshot above predates the channel-group wrapper and localized DPT
selection. Expanding a row gives you:

| Control | What it does |
| --- | --- |
| DPT | Selects the object's datapoint type by number and localized name; preserves unknown existing values |
| Description | Sets the object's description |
| Flags | Read, Write, Transmit, Update, Communication, Read on init (with standard letters) |
| Group links | One row per directional link, with Unlink; paired links can be unlinked together |
| New link | Pick an address, choose Send, Receive or Send + Receive, then click Link — or drop a group address from the Project Explorer on this row, which links it in the direction the row shows |

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

These six flags are retained in native `.knxdb` project data. There is no
`.knxproj` writer; editing a flag does not program hardware or establish that a
manufacturer's application permits that combination.

This is the only place in the interface where a link between a communication object and
a group address is created. [Working with group addresses](04-group-addresses.md)
describes the view from the other end.

#### Where a value came from

Next to the datapoint type and the description you may see a small badge — `Program`,
`ProgramRef`, `Instance`, `Inferred` or `UserEdit`. It says which layer the shown value
came from: the application program's default, the program's per-variant override, the
value stored in the imported project, something KNXBench derived, or your own edit.

That badge distinguishes imported values, product defaults, derived values and
your edits. It is not a promise of ETS export: there is no `.knxproj` writer.
Save the native project and retain the product database; product defaults and
an explicit user override are different evidence, even when their values match.

The same rule drives **product-data enrichment**, which runs at the end of an import
and once per device created from the catalog: for every communication object, slots the
project left empty are filled from the installed application program. Only genuinely
empty slots. A value the project file stated — even an empty or a malformed one — is
left exactly as it was, rather than replacing preserved source evidence with a guess.
Nothing is guessed: a datapoint type stated as a list of alternatives fills nothing and
is reported instead.

If no product database is installed, the import simply says so and skips this step.

### Parameters

**Source builds after Alpha 13** replace the ordinary tab's long accordion form
with a compact parameter workspace. The screenshot below still shows its stated
older release; this source change does not update a published AppImage/container.

**Areas** lists the declared parameter areas with their entry counts. Like other
folded content, nothing opens until you choose an area; your choice stays open
when the project refreshes, while new areas stay closed. The main pane aligns
parameter labels and values; device and module scopes remain separate. Missing group information is
available under **Other parameters**. Search by parameter name, ID, value or
option caption across all areas. Clearing search returns to your chosen area;
a no-result search does not delete fields or values.

Focus a parameter or use its **Information** button to inspect its original ID,
type, accepted-value origin, access and declared numeric limits. An accepted
value is not an unsaved draft, a device readback or an inferred factory default.
Ordinary ReadWrite badges are omitted from the rows but remain in the information
area. Refusals, conflicts and translation fallbacks remain visible with the field.
Editors retain drafts when navigating/searching; ordinary numeric/text blur-save
can still run as focus leaves a dirty input. On narrow panes the area register
scrolls locally and information moves below the editor. No manufacturer-specific
channel numbers or physical arrangements are inferred.

**Declared grids.** Some product data lays out a parameter area as a table, for
example one row per button pair with left and right columns. When you open such
an area, the same fields appear at the rows and columns the product data
declares, with its row, column and separator captions. A note above the grid
says it comes from the product data. Fields the data does not place in a cell
appear below under **Not placed in the declared grid**; declarations that could
not be placed are listed in a closed **could not be placed** disclosure. Editing
works exactly as in the row view, and switching areas keeps your drafts. Areas
without a declared grid stay in rows with a short note saying so; an unknown or
unreadable layout description also falls back to rows and names the reason.
Search results are always rows. Wide grids scroll sideways inside the area on
narrow windows. Captions appear in the program's own language.
See the [workspace contract](../../contracts/parameter-workspace.md).

The following release-era description explains the retained value/editing rules;
the new workspace replaces ordinary section/block folding with area navigation.

![The device panel with the Parameters tab selected. A Device section holds "Delay
after bus voltage recovery (s)" set to 3, followed by "Operating mode" ("Normally
open") and "Behaviour on bus voltage recovery" ("Keep state") once per channel. The
sample program has no modules, so no module sections appear](../../assets/screenshots/porcelain-device-tab-parameters.png)

The tab loads the device's parameters when you select the device. What you get:

- **Sections.** One for the device itself, plus one per module instantiation —
  a device with twelve channels shows twelve sections, never one merged list.
- **Fields.** A select for a choice parameter, a number field with the program's own
  minimum and maximum for a numeric one, a text field otherwise. **Source builds
  after Alpha 12** save a choice immediately; numeric/text fields still save on
  blur or Enter. An ordinary refusal restores the committed value with its reason;
  a stale-write conflict keeps the draft as explained below. The refused write
  does not change the server state. Alpha 12 and earlier apply ordinary choices
  on blur/Enter too.
- **Parameter groups (source after Alpha 12).** Fields use their nearest declared
  parameter block, with a product caption or a numbered fallback. Fold a block
  to hide all its fields without losing drafts. Missing/ambiguous metadata leaves
  fields ungrouped rather than hiding or assigning them to an invented channel.
  Folding adds no command of its own; moving focus away from a dirty numeric/text
  input can still trigger its ordinary blur-save.
- **Changed elsewhere** *(since Alpha 12)*. Each write names the
  project state and the value you saw. If another window or client changed the
  value first, nothing is written, your value stays in the field and the current
  state is reloaded. Under the field, **Current value** shows the new value;
  **Apply my value** writes yours on top of it, **Discard my value** takes the
  current one. Leaving the field does not resend. Without a complete project
  identity the fields are read-only and say so.
- **Headings and spacers.** A parameter without a value (ETS `TypeNone`, or type 0
  in an old `.vd*` database) is drawn as a bold heading with its text, or as blank
  space when it has none. It never gets an input and cannot be written.
Evaluation notices are no longer printed above the editor or repeated inside
sections. Manufacturer-declared Access Read/None fields are in their own tab.
Actual rejected edits still show their reason next to the edited field.

#### Checked editor (since Alpha 10)

Time, number, colour and picture fields have a **Checked editor** button under the
field. It ships since Alpha 10 and is not in the Alpha 9 screenshots. It
reads the parameter's type straight from the imported product data and only offers
what that data proves:

- **Time** values are a decimal count, exactly as stored (`0010` stays `0010`). The
  declared unit, such as `Seconds`, is shown as written; KNXBench does not convert
  it into a clock or duration.
- **Number** fields the manufacturer marked as a slider get a slider next to the
  input. Moving it only changes your draft.
- **Colour** and **picture** values are shown unchanged with a note that their
  format is not verified. There is no colour picker, no image, and nothing is
  fetched.
- When the product data gives no checked source, the editor says why and shows the
  original value; the plain field remains available.

**Apply value** writes through the same path as the plain field, so validation,
Undo/Redo and Save are unchanged. A refused value keeps your draft and shows the
reason. Saving the project keeps a draft; if the value really changed (Undo,
another client), the editor keeps your draft for comparison until you choose
**Discard draft and reload**. Without a draft, it simply follows the project.
**Back to the plain field** closes it. The button is not offered without a complete
project identity or in **Manufacturer fields**; module sections show a note instead
of an editor.

### Diagnostics

Next to **Product data**, **Diagnostics** counts informational notes separately
from warnings. Repeated messages with the same cause, severity and exact
section scope become one entry with an occurrence count, such as **6 occurrences**.
The explanation for an unmatched selection says the controlling value matched
no option; it does not claim a broken manufacturer program. Unknown severity
is still treated as a warning, and unknown diagnostic kinds keep their message.

Open **Technical details** to inspect every original record, or **Copy details**
to copy all records in that group. Product-language fallback summaries and
**Stale values** (stored values with no matching parameter in the current
program) also live here. Those values are preserved, never silently dropped.

### Manufacturer fields

**Manufacturer fields** lists evaluated fields that the manufacturer explicitly
marked **Access Read** (read-only) or **Access None** (hidden from normal user
access). Both remain inspection-only, with the access-specific reason next to
the field. This replaces the old “Show fields without user access” fold in
Parameters. Other refusal reasons do not automatically make a field a
manufacturer-access restriction; those fields stay in Parameters with a
read-only caption pointing to Diagnostics.

Only already evaluated fields are shown, not every static field in the product
database. Switching these tabs does not reload the device or write a parameter.

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

**What it does not do.** Editing a parameter does not itself talk to a
physical device. It changes the project value and re-evaluates the displayed
program state; writing configuration to hardware is a separate, explicitly
gated device-download operation, never a side effect of this field.

> **Note**
>
> Parameter interpretation is the least finished part of KNXBench, and the project
> documents it in detail rather than glossing over it: section 3 of
> [Known limitations](../../KNOWN_LIMITATIONS.md) is the full account. If a device's
> parameters look wrong, that document is the first place to check before filing a bug.

### Product data

![The device panel with the Product data tab selected. Under PRODUCT IDENTITY a badge
reads "From the product database"; the product reference, the application program
reference, the manufacturer "Example Devices (fictional)", the product name "Switch
actuator 4-fold, DIN rail" and the order number EX-SA4 are listed, and the expanded
"More product data" disclosure adds the PRODUCT, HARDWARE and APPLICATION PROGRAM
groups with manufacturer ID, catalogue item, hardware and program versions and the mask
version](../../assets/screenshots/porcelain-device-tab-product-data.png)

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

### Installation instructions (since Alpha 10)

The sixth tab, **Installation instructions**, shows the installation hints a device
brought along from an imported ETS project, and lets you replace them with your own
text. It ships since Alpha 10 and is not in the Alpha 9 screenshots.

- The imported original is kept as it came in. **Imported original is preserved**
  or **Native override** above the text says which of the two you are looking at.
- Editing covers plain text with paragraphs and **Bold**, **Italic** and
  **Underline**; select text and use the buttons. **Rendered preview** shows the
  result. Markup such as `<script>` is only text and never runs, and nothing is
  fetched from links or embedded resources.
- **Preview change** shows the change; **Apply change** applies it. If the original
  used formatting KNXBench cannot show, Apply asks you to agree to the supported
  subset first; the original stays retained either way.
- **Return to imported original** removes your override.
- A device you added in KNXBench has no imported original: the tab says **Made in
  KNXBench: no imported original** and starts empty. An imported device that came
  without instructions says **Imported without instructions**. In both cases
  **Remove instructions** takes your text away again, and the device has none, as
  before.
- After Apply the workbench reloads the project, so Undo/Redo and the modified state
  follow the edit. Applying is not saving the file: use Save or Save As.
- Saving the project does not disturb a draft. If the instructions really changed on
  the server in the meantime (another client, Undo), the editor keeps your draft for
  comparison and asks you to discard it and reload before a new preview.
- If the server refuses the change, its message appears in the tab, nothing changes
  and your draft stays.
- Without a complete project identity the tab is read-only and says so.
- A draft survives switching tabs. Opening or importing another project, starting a
  new one or quitting asks first, the same as for a [project-notes
  draft](01-user-interface.md#project-notes-since-alpha-10).

Only instructions from an imported ETS project are supported; a device created in
KNXBench, or one whose original cannot be read, says so instead of offering an editor.

### Recorded device state (since Alpha 10)

In the Properties Inspector of a device, **Show recorded device state** lists what
the project believes about the device's loaded parts (individual address,
application program, parameters, communication part, medium configuration) next to
any report the model still holds. It never contacts the device. *No report
retained* does not mean "not loaded"; recorded dates are project metadata, not the
last successful download. Stored bus access and IP settings from ETS are a separate,
read-only view (see *Stored bus access* below).
[Contract](../../contracts/recorded-device-state.md).

## Adopting plug-in data (source builds after Alpha 13)

Some devices are configured in ETS through a manufacturer plug-in. KNXBench
keeps what such a plug-in stored (a `BinaryData` blob and numbered `ExtraData`
tables) byte for byte, but it never runs the plug-in. If you changed a device in
the plug-in and exported the project from ETS again, **Adopt plug-in data…**
copies that one device's stored bytes from the new export into the project you
have open. It is in the device panel header and in the command palette.

1. Optionally enter the source project's password, then **Choose source
   project…**. The list shows each device pair (the same ETS device in both
   projects) with *Can adopt* or *Cannot adopt* and the reason.
2. Pick a pair and **Preview**. Nothing is written yet. The preview names every
   entry it would replace with its size and both SHA-256 digests, lists the
   entries that already match, and warns when the two devices have different
   individual addresses (the bytes may still contain the source address).
3. Confirm that Undo does not reverse this, then **Adopt plug-in data**.

It refuses, naming the reason, unless both devices are the same product with
the same application program, reference the same `BinaryData` and `ExtraData`
entries and link the same group addresses. It never maps one id or table number
onto another, because that would be a guess about the plug-in's format.

**The way back is a project version, not Undo.** The project must be saved as
`.knxdb`. Before anything changes, KNXBench keeps the current state as the
version *Before plug-in data adoption*; restore it in **Project history** to go
back (the dialog has a button for it). The replaced bytes also stay in the
project. Whether the plug-in, ETS or the device accepts the adopted bytes is
**not verified**; KNXBench writes no `.knxproj`. If the project changes while a
preview is shown, the adoption is refused with nothing written: reload and
preview again. [Contract](../../contracts/plugin-data-adoption.md),
[known limitation §186](../../KNOWN_LIMITATIONS.md#catalogue-kl-186).

## Stored bus access

An ETS project can remember how ETS reached the bus for a line (*bus access*) and
a device's IP settings (*IP configuration*). KNXBench keeps both and shows them
read-only: **Stored bus access…** in the command palette lists every record, and
a selected line shows its own under **Stored bus access (from ETS)**.

Addresses, ports and names are hidden at first; you see what kind of value is
there (for example *hidden (IPv4 address, 13 characters)*). **Show values** shows
one record's values; **Hide values** hides them again. Settings such as the
access type stay visible. Each record says which line it belongs to, or why it
belongs to none: an IP configuration never names its device in the stored
project, so it is listed without one. **Source** shows the file, the path inside
it, the size and a SHA-256 checksum.

KNXBench never connects to these addresses and never offers them as a gateway;
you choose your gateway in the bus connection as before
([details](../../contracts/stored-bus-access.md)).

[Manual index](../README.md) · Next: [A complete configuration workflow](06-configuration-workflow.md) →
