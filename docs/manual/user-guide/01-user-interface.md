← Previous: [Products and product databases](../knx-basics/05-products-and-product-databases.md) · [Manual index](../README.md)

# The user interface

**Goal:** find the explorer, workspace, inspector, search and command palette.
**Prerequisites:** a running app; open a disposable project to explore populated views.
**Expected result:** you can select an item and find its properties without changing
the project. **Watch out:** selecting is not editing, and closing a tab is not saving.

KNXBench opens as one window with one job: show you a KNX project and let you change
it. There is no ribbon, no floating tool window, and nothing that has to be docked
before it works. This chapter walks around the window once, names every part, and
says what each part is for.

![The KNXBench window with no project open: a header bar, a navigation pane on the
left, a workspace headed "Your KNX workspace" with the cards New project…, Open
KNXBench project and Import ETS project, and a properties pane on the right](../../assets/screenshots/porcelain-welcome.png)

That is the whole window before any project exists. Notice that the navigation pane,
the workspace and the properties pane are already there and already empty — the layout
does not change shape when a project arrives, it only fills up.

On the very first start, and again when the build reaches a new release stage, a
four-page introduction opens over this window first; [First
start](../getting-started/06-first-start.md#the-introduction) describes it.

## The header

The top row, left to right:

- **KNXBench** — the brand, and also a button: it takes you back to the Overview.
- **File** — a drop-down menu with project and export actions. It is
  covered in [Projects](02-projects.md).
- **Undo** and **Redo** — two arrow buttons. They are gray when there is nothing to
  undo or redo. Every edit in KNXBench goes through the same command stack, so undo
  works the same way for a renamed room as for a bulk delete.
- **Search… (Ctrl+K)** — opens the project search. Disabled until a project is open.
- **Commands… (Ctrl+Shift+P)** — opens the command palette.
- **Save** — the one purple button. If the project has never been written to a file,
  Save behaves as Save As and asks you where to put it.
- **?** — opens the help panel. `F1` does the same from anywhere in the main window.
- **The gear** — opens Settings. See
  [Settings, themes and languages](09-settings-and-appearance.md).

The web File menu includes **Export project…** for a local `.knxdb` copy of
the open project. The desktop build instead has native file dialogs and adds
**Quit**; a browser tab cannot close itself. Neither build exports `.knxproj`.

## The strip below the header

A thin bar carries the two pane toggles: **Navigation** on the left, **Properties** on
the right. Each one hides or shows its pane, which is useful on a small screen and
essential on a laptop in a basement.

Two things appear in this bar only when they have something to say:

- **Bulk actions**, when you have more than one device or group address selected. See
  [Buildings and topology](03-buildings-and-topology.md).
- **Import: N errors · M warnings**, when the project was imported and the import
  found something worth telling you about. Clicking it goes to the Overview, where the
  counts are broken down. [Projects](02-projects.md) explains what those numbers mean.

## Active task status text (14 October 2026 source, main-only)

While work is actually running, its status text has a quiet glint that moves
from left to right and pauses before repeating. The words remain readable: this
is an activity cue, not a skeleton placeholder or a progress estimate. Existing
bars and real counts remain. Results, errors, connection states and prompts
waiting for your input do not shimmer. Busy action-button captions are unchanged.

Turn **Settings → Appearance → Motion level → Off** to stop it. Your operating
system's reduced-motion preference also wins; forced-colour mode and printing
keep plain text. This needs a source build after Alpha 11; the published manual
screenshots still show the earlier released UI. See
[Motion settings](09-settings-and-appearance.md#motion-style-and-motion-level).

## The navigation pane

The left pane has three stacked blocks.

**The views**, at the top:

| Button | What it shows |
| --- | --- |
| Overview | Project status: project contents, the import report and technical details |
| Buildings | The building structure as cards, with the devices placed in each part |
| Topology | Areas, lines and the devices in them |
| Devices | The project-wide, filterable/sortable device table; a device click opens its central editor |
| Group addresses | The group-address table |
| Notes | The project's engineering notebook (since Alpha 10) — see [Project notes](#project-notes-since-alpha-10) |
| Product catalog | Opens the catalog overlay, not a view — see [Devices and products](05-devices-and-products.md) |

**The project explorer**, in the middle: the project as a tree — Topology, Buildings,
Group Addresses, Group Ranges, and any unassigned devices. This is where you create
things. Each branch that can hold a new child has an inline row at the bottom of it:
type a name, press Enter or click Add, and the object exists. No dialog opens.

Every branch that can open has a chevron in front of it (source builds after
v0.1.0-alpha.11): it points right while the branch is closed and turns down when it
opens. A closed branch shows how many entries it holds, for example a small **3**
next to a line with three devices. The two buttons above the tree are **Expand
all** and **Collapse all**; Collapse all keeps the installations open and folds
their sections, so a large project shrinks to one screen. Thin lines connect each
node to its children. When you open a branch, its lines grow from the parent
down, and about every twelve seconds a soft pulse runs from the top of the tree
through the rows on screen. Turn that down or off under **Settings → Motion by
category → Structure lines in the explorer**; the lines themselves stay. The
screenshots in this manual still show the released explorer.

**The diagnostics block**, at the bottom: **Bus monitor**, **Log**, **Diagnostics
window** and **Settings**. The first two swap the center workspace for a panel; the
third opens a second window (below); the fourth opens the Settings overlay.
In source builds after Alpha 11 (issue #23), selecting an already-active **Bus
monitor** or **Log** entry keeps that panel open and preserves its current tab,
input or filter. To leave it, choose another view explicitly; clicking the active
entry is no longer a close toggle. Settings and the separate diagnostics window
keep their existing opening behaviour. The screenshots still show the released UI.

![The Overview of an imported project, showing counts for schema version,
installations, areas, lines, devices, group addresses, building parts and
communication objects, with the project tree on the
left](../../assets/screenshots/porcelain-dashboard.png)

The Overview of the fictional sample project the manual's screenshots use: 1 area,
2 lines, 8 devices, 15 group addresses, 8 building parts, 39 communication objects.
The tree on the left shows the same project, opened at its first line.

Alpha 10 (newer than this Alpha 9 screenshot) groups the same numbers
into three cards: **Project contents**, **Import report** and **Technical details**.
The import card states the overall result in words — errors recorded, imported with
warnings, nothing recorded, or *not assessed* when no import report is available —
and every count stays visible, zeros included. A non-zero error or warning count
carries **Show in log**: it opens the [Log](#the-workspace) showing only that
severity's import-report entries, says whether the log still holds as many as were
counted, and offers **Show all log entries** to return. Keyboard users reach it with
Tab and open it with Enter; focus moves to the explanation above the list. A zero is
plain text, because there is nothing to show.

## The workspace

The center is whatever the selected view shows. Selecting a device from the tree,
topology/building diagram, search or a device link switches the center to its
editor, rather than appending it below the previous view. **Devices** stays marked
in the navigation. **Back** returns to the originating app view; **All devices**
opens the project-wide device table. Filter, sort, table scroll and checkbox
selection survive the editor visit. The existing Properties inspector remains
on the right. See [Devices and products](05-devices-and-products.md#the-devices-view).

The Devices view first shipped with Alpha 6 and is included in every later release; the
older Alpha 5 AppImage does not have it.

The Bus monitor and the Log replace the workspace entirely while they are open, and
the properties pane hides itself while they are, because neither of them has a
selection to inspect.

Click a toast’s message action to open **Log**, jump to its exact retained
UI notification and show details. The separate × still only dismisses the toast.

Alpha 10 also has update notifications: **Release details on GitHub** opens
that release page, while the message action still opens the notification record.
**File → About KNXBench…** retains update information and offers manual checking;
**Settings → Updates** controls automatic checks. This is newer than the
Alpha.9 shown in the screenshots. See
[Release notifications](09-settings-and-appearance.md#release-notifications-since-alpha-10).

Click any log message for **Message details**: **Copy message** copies its raw
message and **Copy all details** copies the full JSON record. You can select
the read-only text and use Ctrl+C; if clipboard permission is refused, the
record is selected for manual copying. Escape closes details and returns focus
to the log row. Text is not interpreted as HTML or changed for copying.

![Alpha 9 in Graphite: message details for a fictional project’s theme-change notification, with raw message and full-record copy controls](../../assets/screenshots/graphite-message-details.png)

*This is a browser notification, not a bus telegram or a server-log export. The
message really came from changing the theme; no error was invented for the camera.*

The latest 1000 UI notifications are browser-local, independent of popup expiry
and the server log. Older UI losses are counted; reload ends this history.
A removed target reports that it is unavailable instead of opening a same-text
replacement. **JSON session-log export stays server-only**; UI notifications
are available through the detail copy controls, not silently mixed into that
server-run audit format.

In **Log**, type into **Search session log** to match operation, summary or
detail without changing the severity checkboxes; **Clear search** restores
the unsearched view. The result count compares shown entries with the
retained total, including the explicitly separate UI notifications. Every
entry shows its time in a column of its own (today's entries the time only,
older ones the date too; hover for the full timestamp), then level, source and
message, one line per entry *(source build after Alpha 11; earlier builds have
no time column and rows of uneven height)*. **Export…** opens a small dialog:
choose the **server** entries shown now or all retained server entries, then
**Export JSON** saves `session-log.json` to your computer *(up to Alpha 11:
an **Export scope** list and **Export log (JSON)…** in the toolbar)*. JSON v1 preserves raw timestamp, severity, source/operation,
message/summary, location, detail and diagnostic metadata, including
quotes, line breaks and Unicode; it is not spreadsheet CSV. The export
contains the cap and loss notice even when filtering hides the notice.
This server run keeps up to 16000 entries (up to Alpha 11:
1000, oldest first). When the log is full, info entries are dropped first, then
warnings, then errors, the oldest of each first; the pinned notice says how
many of each are gone. Opening a replacement project or restarting the server
resets the trail. **This is not a complete audit.** The list shows 500 entries
at a time (**Show more** adds the next 500; search and export always cover
all of them). A long message or detail, such as a grouped list of retained
files, is cut to its line with a count of the hidden lines; the entry's details
show all of it. An RTF-like raw-text preview from an import keeps its literal
block under its row. Entries may name
project elements, KNX addresses or local paths; check before sharing the
file. The desktop build opens an OS save dialog; browser export uses a local
file download, not the server's Save As directory.

### Project notes (since Alpha 10)

**Notes** in the navigation pane, or **Notes** in the command palette, opens the
project's engineering notebook: ordered plain-text notes attached to the project
or to one of its parts. It needs an open project and is disabled without one.
This view ships since Alpha 10 and is not in the Alpha 9 screenshots.

- The list sits beside the editor. **Find notes** only filters what you see; it
  never reorders the project's notes.
- **Title** and **Note text** are literal text. Markup such as `<b>` or `**bold**`
  stays exactly as typed and is never rendered.
- **Target** offers the project, its installations, areas, lines, building parts,
  devices, communication objects, group ranges and group addresses, identified by
  their stable IDs. A communication object reads as its number and name followed
  by its device, for example *Communication object: 0 Switching · 1.1.4 Switch
  actuator kitchen*. The server decides which targets are offered and offers only
  ones a note can really be attached to; a note whose stored target is not in the
  list keeps it and shows it as *Stored target not in host catalogue*. If the list
  cannot be loaded, the notebook says so and offers **Load targets again**; the
  notes stay readable and can still be attached to the project.
  *Communication objects as targets: since Alpha 11.*
- **Include in project documentation** stays off unless you tick it.
- **Preview change** shows the exact before and after, naming the action (*Create
  note*, *Change note*, *Remove note*, *Move note*) and whether the note goes into
  the documentation (*Yes*/*No*) in your interface language; nothing changes until
  **Confirm change**. Remove and the two Move buttons are previewed the same way,
  and editing the draft again withdraws the preview.
- After a confirmed change the workbench reloads the project, so Undo/Redo and the
  modified state follow the note like any other edit. Applying a note is not saving
  the file: use Save or Save As as usual.
- If the server refuses (for example because another client changed the notes after
  your preview), the alert appears where you clicked, the notes stay as they were
  and your draft is kept for another try.
- A draft survives switching to another view and saving the project. If the notes
  really changed on the server meanwhile, the draft is kept for comparison and the
  notebook asks you to discard it and reload. Opening or importing another project,
  starting a new one or quitting asks first: **Keep editing** returns to the notes,
  **Discard draft and continue** goes on. In a browser, reloading the page or
  closing the tab with a draft makes the browser ask first, in its own wording
  (it cannot show KNXBench's dialog). In the desktop window a reload (right-click
  › **Reload**) asks the same way, in WebKit's **Stay**/**Leave** dialog; the
  window's close button uses the KNXBench dialog instead. *The browser prompt and
  the translated review names arrived in Alpha 11; the desktop reload prompt
  after Alpha 11.*

**Application help** in the notebook opens the help panel's *The window* topic.

## The properties pane

The right pane inspects whatever is selected and lets you edit it. It says "Select an
item to inspect or edit its properties." when nothing is. What it offers depends on
what is selected: a device has an address, a line, a building part and a description;
a building part and a group range can be renamed; an area or a line can only be
deleted. The chapters that follow say which is which.

## The footer

One line at the bottom: the installation names of the open project (or just
"KNXBench" when nothing is open) on the left, and the version on the right. The
current screenshots in this manual were taken from the published
`v0.1.0-alpha.9` frontend and server (Alpha 10 views are marked where they differ). Porcelain is the light theme; Graphite
shows the same workbench in dark mode. Earlier README videos remain labelled
as historical recordings in the [media inventory](../../assets/README.md).

## Resizing

Both side panes have a drag handle on their inner edge. Drag it, and the pane resizes
within its limits — the navigation pane between 200 and 480 pixels, the properties
pane between 280 and 700.

The handles are also keyboard controls. Tab to one, then:

| Key | Effect |
| --- | --- |
| Left arrow / Right arrow | Move the edge by 16 pixels |
| Home | Jump to the minimum width |
| End | Jump to the maximum width |

With a project open, two horizontal splitters appear inside the navigation pane: one
above the project explorer, one below it. They give height back and forth between the
view buttons, the tree and the diagnostics block. They take arrow keys too — up and
down instead of left and right, in the same 16-pixel steps, with Home and End for the
limits.

> **Tip**
>
> Pane widths are not saved yet. Every start begins at the default layout. If that
> annoys you, it annoys us as well; see [Known issues](../known-issues.md).

## The overlays

Seven things open on top of the window instead of inside it. All of them close with
`Escape` or with their own Close button.

**Search** (`Ctrl+K`) searches the open project — devices, group addresses and
building parts, grouped by kind. It matches as you type, arrow keys move through the
hits, Enter selects one and takes you to it, reopening the containing Project
Explorer branches when necessary.

![The search overlay with the query "light" and eight matching group addresses listed
under a "Group addresses" heading, each with its address](../../assets/screenshots/porcelain-search.png)

Searching the sample project for "light": eight group addresses match, each shown with
its name and its address.

**The command palette** (`Ctrl+Shift+P`) lists every command KNXBench has as a
keyboard-reachable list. There are sixteen: New project, Open project, Open
(.knxdb), Save, Save As, Undo, Redo, Search, Log, Bus monitor, Settings, Diagnostics
window, Product catalog, Show introduction, Achievements, Help. Typing filters the
list by substring.

![The command palette showing all sixteen commands, with Undo and Redo grayed out and
the keyboard hints Ctrl+Z, Ctrl+Shift+Z, Ctrl+K and F1 on the rows that have
them](../../assets/screenshots/porcelain-command-palette.png)

Undo and Redo are gray here because this project has no history yet. The palette shows
each command's shortcut where it has one, which makes it a decent way to learn them.

**Help** (`F1`, or the `?` button) has eleven short topics about this window
and the KNX terms behind it: Getting started, The window, Buildings floors
and rooms, Areas lines and devices, Group addresses, Group address ranges,
Communication object flags, Bus monitor, Import, Keyboard, and What this
does not do. A tip's `?` button opens its own full topic on click or Enter;
F1 while that tip is focused opens the same topic. Elsewhere F1 starts at
**The window** overview. The topic heading names the dialog for a screen
reader and its prose can be reached by keyboard. In the Group addresses
table, the **Range** tip explains nested address containment: ranges are
not datapoint types or payload-value intervals. A validation toast for a
malformed address, DPT or project language also shows the expected syntax
and one example; the original server detail remains available for diagnostics.

![The help panel with its eleven topics listed, among them Group address ranges and
Import, and the "Getting started" topic
open](../../assets/screenshots/porcelain-help-panel.png)

The import topic is named **Import**, not "Import and export", because KNXBench does
not export `.knxproj` ([ADR-0028](../../adr/0028-no-knxproj-export.md)).

The help panel is deliberately short. It answers "what is this thing on my screen",
not "how does KNX work" — that part is this manual's job, starting at
[KNX in a few minutes](../knx-basics/01-knx-in-a-few-minutes.md).

**Settings** (the gear) holds theme, accent color, density, motion style, motion level,
interface language and product-data language. See
[Settings, themes and languages](09-settings-and-appearance.md).

**The product catalog** and **New project** are the two overlays that create something.
They are covered in [Devices and products](05-devices-and-products.md) and
[Projects](02-projects.md).

**About KNXBench** shows version and license information. *(Current source,
14 October 2026:)* **Community Hall of Fame on GitHub** links directly to the
[public community document](../../community/HALL_OF_FAME.md), with everyone outside
the team who reported or discussed an issue, their titles, medals and counters.
The app no longer bundles a snapshot or fetches the board in the background;
only activating the link opens GitHub. Reading the board needs an Internet
connection; opening About does not.
Existing release artifacts are unchanged by this source fix.

**The introduction** (**File → Show introduction…**, or the palette) is the
four-page guide described in [First
start](../getting-started/06-first-start.md#the-introduction).

**Achievements** (**File → Achievements…**, or the palette) lists the
achievements you have unlocked and the ones still ahead. See
[Settings and appearance](09-settings-and-appearance.md#achievements).

## The second window

The **Diagnostics window** button opens a separate browser window (or a second desktop
window) carrying two tabs and nothing else: **Bus monitor** and **Log**. It is
read-only by design. It has no undo, no project editing, no help panel of its own, and
a "Back to main window" button to get you home. In current browser source, that
button navigates the current tab to the editor, even when Diagnostics was opened
directly or its opener has closed; a separate editor tab is not closed. The
currently open server project is displayed again without reopening or
reimporting it; the bus session remains open. Transient editor selection is
not transferred. In the native desktop build, the button still focuses the
existing main window. The same browser return applies to the separate Flow
window. This source fix is not in the published alpha.6 artifacts.

It shares the one bus session with the main window rather than opening a second one,
and it announces as much at the top of the window. Both windows watching one session
is the point: you can keep the monitor on a second screen while you work in the first.

> **Note**
>
> The diagnostics window does not clean up the bus session when you close it. If a
> session is running, stop it from the bus monitor rather than by closing the window.
> [Bus monitor and KNXnet/IP](07-bus-and-interfaces.md) covers sessions properly.

## Keyboard

![Real porcelain-theme search: Ctrl+K opens Search, typing light filters the fictional project results, and Escape closes it](../../assets/workflows/search.gif)

*Search filters project results; it does not scan the KNX bus.*

The shortcuts that work anywhere in the main window:

| Shortcut | Action |
| --- | --- |
| `Ctrl+K` | Search (needs an open project) |
| `Ctrl+Shift+P` | Command palette |
| `F1` | Help |
| `Ctrl+Z` | Undo |
| `Ctrl+Shift+Z` | Redo |
| `Escape` | Close the overlay on top |

Undo and redo deliberately do nothing while the focus is in a text field, a select, or
a dialog — inside a text box, `Ctrl+Z` belongs to the text box.

A fuller list lives in [Keyboard shortcuts](../reference/01-keyboard-shortcuts.md).

[Manual index](../README.md) · Next: [Projects: create, open, import, save, export](02-projects.md) →
