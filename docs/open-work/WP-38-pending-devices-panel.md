# WP-38 — Pending-devices panel and batch download (#42)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **step 1 implemented** (11 November 2026, main-only); **step 2 open**
(needs an ADR before any code). Depends on WP-34 copy.

## Packages

1. Read-only side panel listing every device with pending changes, what is
   pending, and a jump to its download dialog.
2. Sequential “download all changes” is a multi-device bus write: design and
   ADR first (per-device readiness before start, one batch consent, stop at the
   first failure, per-device results), then implementation with simulator
   tests only unless hardware is separately authorised.

## Step 1 as implemented (11 November 2026)

A **Pending programming** block (German **Ausstehende Programmierung**) in the
navigation pane, between the project explorer and Diagnostics, as the triage
reply promised: read-only, no bus access.

- **Heading:** `Pending programming ({count})` / `Ausstehende Programmierung
  ({count})`, the number of devices with at least one pending part. The count
  is visible while the block is folded; the block starts folded like every
  navigation block (Alpha 14 "closed by default"). Fold state is local, as for
  the other blocks; it is not saved.
- **Rows:** individual address (monospace), name and the Devices table's
  Adr/App/Par/GA chips (`createProgrammingChips`). The order and the filter are
  the Devices table's own (`visibleDeviceRows` with `pendingOnly` and the
  address sort), so devices without an address come first, and 1.1.10 follows
  1.1.9. The device open in the editor is marked (`aria-current`).
- **Click / Enter** on a row selects the device through the explorer's own
  selection path (`selectEntity`): the device editor opens.
- **Download…** / **Programmieren…** calls the same hand-off as the device
  editor's **Download to device…** (`DownloadNavigationProvider` →
  `openDownload` → `DownloadHandoff`): the Download to device tab opens with
  the device preselected; no plan, start or connection. Its accessible name
  names the device (`Download to {device}…` / `{device} programmieren…`). On a
  narrow window (≤ 650 px) the navigation closes so the tab is visible. A
  device without an individual address shows the button disabled with the
  existing reason text.
- **Unknown state:** devices whose `pending_programming` the server does not
  report are never listed or guessed; one line counts them
  (`Programming state not reported for {count} device(s).` /
  `Programmierstand für {count} Gerät(e) nicht gemeldet.`).
- **Empty:** `Nothing pending` / `Nichts ausstehend`.
- **Large projects:** the summary is one pass plus one sort per tree snapshot
  and language; the list mounts only while the block is open
  (`NavigationSection lazy`), renders the first 50 rows with
  `Showing 50 of {count}` / `50 von {count} angezeigt` and **Show all** /
  **Alle anzeigen**, and scrolls inside its own box.

Tests: `PendingProgrammingPanel.test.tsx` (listing, order, chips, empty,
unknown, selection, hand-off, missing address, cap, German),
`NavigationSection.test.tsx` (lazy mount), and en/de browser cases at 1440 and
400 px in `e2e/pending-programming.e2e.ts` (production `playwright.devices.config.ts`
and the development fixture config).

## Exact sidebar copy

| Key (under `pendingPanel`) | English | German |
|---|---|---|
| `title` | `Pending programming ({count})` | `Ausstehende Programmierung ({count})` |
| `listLabel` | `Devices with pending programming` | `Geräte mit ausstehender Programmierung` |
| `empty` | `Nothing pending` | `Nichts ausstehend` |
| `download` | `Download…` | `Programmieren…` |
| `downloadDevice` | `Download to {device}…` | `{device} programmieren…` |
| `showing` | `Showing {shown} of {count}` | `{shown} von {count} angezeigt` |
| `showAll` | `Show all` | `Alle anzeigen` |
| `unknown.one` | `Programming state not reported for {count} device.` | `Programmierstand für {count} Gerät nicht gemeldet.` |
| `unknown.other` | `Programming state not reported for {count} devices.` | `Programmierstand für {count} Geräte nicht gemeldet.` |

`{device}` is the individual address followed by the name, or just the name
when the device has no address. The missing-address reason reuses
`devices.downloadNeedsAddress` unchanged. German copy is neutral, with no
formal address to the user.

## Step 2 (not started)

No batch start button exists. Downloading every pending device in sequence is
a multi-device bus write; it needs an ADR first (see package 2 above) and then
simulator-only tests unless hardware is separately authorised.
