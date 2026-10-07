← Previous: [Web and Docker deployment](../user-guide/11-web-and-docker.md) · [Manual index](../README.md)

# Keyboard shortcuts

This chapter lists every shortcut the KNXBench frontend actually binds, read straight out
of `apps/knx-web/src`. If a key combination isn't here, it isn't wired to anything —
KNXBench doesn't have a hidden shortcut you have to guess.

The **command palette** (`Ctrl+Shift+P`) is also a live, filterable list of these — see
[The user interface](../user-guide/01-user-interface.md) — which makes it a reasonable way
to check a shortcut without leaving the keyboard.

> **Note**
>
> "`Ctrl`" below also means `Cmd` — the code checks both `ctrlKey` and `metaKey` for every
> shortcut, so the same keys work if you ever run the web build in a browser on a
> non-Linux machine. KNXBench is Linux-first, not Linux-only for the browser build.

## Global shortcuts

These work anywhere in the main window, with no field focused:

| Shortcut | Action | Notes |
| --- | --- | --- |
| `F1` | Open Help | Plain `F1` only — `Ctrl+F1`, `Alt+F1` and `Shift+F1` are left alone, since several Linux desktops bind those themselves |
| `Ctrl+K` | Open Search | Only while a project is open |
| `Ctrl+Shift+P` | Open the command palette | Always available |
| `Ctrl+Z` | Undo | Disabled (does nothing) while a text field, a select, or a dialog has focus — inside a text box, `Ctrl+Z` belongs to the text box |
| `Ctrl+Shift+Z` | Redo | Same field-focus exception as Undo |
| `Escape` | Close whatever is on top | See [Escape, in order](#escape-in-order) below |

Eleven more actions are reachable only through the command palette or a button, with no
shortcut of their own: New project, Open project, Open (`.knxdb`), Save, Save As, Log, Bus
monitor, Settings, the Diagnostics window, the Product catalog, and Show introduction. All
fifteen palette commands, with the shortcuts that exist, are visible at a glance in the
command palette itself:

![The command palette listing all fifteen commands as a filterable list, with Undo and
Redo shown disabled and the keyboard hints Ctrl+Z, Ctrl+Shift+Z, Ctrl+K and F1 next to the
rows that have them](../../assets/screenshots/porcelain-command-palette.png)

**Save** has no dedicated shortcut. It's a header button and a palette command, not a key
combination — there is no `Ctrl+S` in this application today.

## Moving around without a mouse

KNXBench is built so that nothing behind a mouse-only interaction is actually
mouse-only — a few things just need `Tab` first.

**The project tree, in the navigation pane.** Once a tree item has focus:

| Key | Effect |
| --- | --- |
| `ArrowUp` / `ArrowDown` | Move to the previous / next visible row |
| `ArrowRight` | Expand a collapsed branch |
| `ArrowLeft` | Collapse an expanded branch |
| `Home` / `End` | Jump to the first / last visible row |

**The device workspace tabs** (Communication objects, Parameters, Product data) form a
proper tab strip: focus the active tab, then `ArrowLeft` / `ArrowRight` moves between the
three, wrapping at the ends. `Home` and `End` jump to the first and last tab.

**The left-pane view buttons** (Overview, Buildings, Topology, Group addresses, Product
catalog) and the group-address table's rows are plain, individually focusable controls —
`Tab` moves between them one at a time, but there is no arrow-key roving selection on
either. Use the tree for arrow-key navigation between structural items.

**Search, the command palette, and the product catalog's search field** share the same
list behavior once open:

| Key | Effect |
| --- | --- |
| `ArrowUp` / `ArrowDown` | Move the highlight up or down the result list |
| `Enter` | Activate the highlighted result |
| `Escape` | Close the overlay |

![The search overlay with the query "light" and eight matching group addresses listed
underneath, reachable the same way by arrow keys and Enter](../../assets/screenshots/porcelain-search.png)

**The File menu is keyboard-reachable even though it looks like a mouse dropdown.** It's
a native HTML `<details>`/`<summary>` element: `Tab` to it, `Enter` or `Space` opens it,
`Tab` moves through its items, and `Escape` closes it and returns focus to the menu
button. Clicking anywhere outside it also closes it.

**Resizing panes is a keyboard operation too**, not just a drag:

| Where | Keys | Step |
| --- | --- | --- |
| The navigation pane and the properties pane (their inner-edge handle, once focused with `Tab`) | `ArrowLeft` / `ArrowRight`, `Home`, `End` | 16 pixels per press, `Home`/`End` jump to the pane's minimum/maximum width |
| The two horizontal splitters inside the navigation pane (only visible with a project open) | `ArrowUp` / `ArrowDown`, `Home`, `End` | Same 16-pixel step, `Home`/`End` jump to the block's minimum/maximum height |

See [Resizing](../user-guide/01-user-interface.md#resizing) for what the panes actually
contain and their pixel limits.

## Escape, in order

`Escape` doesn't always mean the same thing — it closes whatever is nearest to you first:

1. A help tip bubble, if one is open.
2. The dialog or overlay it's inside of (Search, the command palette, the catalog
   browser, Settings, Help, About, New project, the file picker, the quit confirmation).
3. The open File menu.
4. A multi-selection built with `Ctrl`-click or `Shift`-click in the tree or the
   group-address table, if nothing above applies.

Every overlay in KNXBench is built on one shared shell that also traps `Tab` inside it
while it's open — `Tab` from the last focusable control wraps back to the first, and
`Shift+Tab` from the first wraps to the last, so keyboard focus can't slip out behind a
modal dialog by accident. Closing an overlay returns focus to whatever opened it.

## What is genuinely mouse-only

To be honest about the gaps rather than imply a keyboard path that doesn't exist:

- **Building a multi-selection is `Ctrl`-click / `Shift`-click only.** There is no
  keyboard equivalent for selecting several devices or group addresses at once for the
  bulk-action bar; `Escape` can clear a multi-selection once it exists, but nothing
  creates one from the keyboard.
- **Every drag-and-drop gesture has a keyboard path.** Dragging a device onto a line or
  building part does what the select fields in the properties pane do, and dropping a
  group address on a communication object's link row does what that row's Link button
  does (see [Buildings and topology](../user-guide/03-buildings-and-topology.md) and
  [Devices and products](../user-guide/05-devices-and-products.md)).
- **List entries inside overlays and the group-address table have no arrow-key roving**
  beyond the highlight behavior described above; navigate them with `Tab` like any other
  set of buttons.

## Web build versus desktop build

The shortcuts above are identical in both builds — they live in `apps/knx-web/src`, which
both builds share. Two things differ because of what surrounds that shared code, not
because of the shortcuts themselves:

- **Quit has no keyboard shortcut in either build**, and the File menu's Quit item only
  exists at all in the desktop (Tauri) build — a browser tab has no way to close itself,
  so the web build's File menu simply has no Quit entry. In the desktop build, the
  window manager's own close button quits the whole application the same way Quit does,
  not just the window.
- **The Open/Save file picker is KNXBench's own in-browser dialog in the web build**
  (shown below) and your desktop's native file dialog in the Tauri build. The native
  dialog's own keyboard behavior is your desktop environment's, not KNXBench's, and is
  outside this manual's scope.

![KNXBench's own Open dialog in the web build, listing a directory with one project file,
an Upload row, and a Cancel button](../../assets/screenshots/porcelain-open-project-dialog.png)

[Manual index](../README.md) · Next: [Supported and unsupported KNX/ETS functionality](02-supported-and-unsupported.md) →
