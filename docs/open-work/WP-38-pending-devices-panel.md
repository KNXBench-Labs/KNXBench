# WP-38 — Pending-devices panel and batch download (#42)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **step 1 implemented** (11 November 2026, main-only); **step 2 designed**
in [ADR-0172](../adr/0172-batch-programming-stops-at-the-first-doubt.md)
(Proposed); its **server part implemented** (WP-38b, 11 November 2026), the UI
part (WP-38c) open. Depends on WP-34 copy.

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

## Step 2 server part (WP-38b, implemented 11 November 2026)

Routes `batch/plan`, `batch/start`, `batch/status` and `batch/stop` under
`/api/device-download/`, as ADR-0172 and its 11 November amendment describe
(`apps/knx-server/src/device_download_batch.rs`, run in
`device_download_batch_run.rs`; the single start's sequence is shared as
`device_download_routes::launch`). Status states per device: `waiting`,
`running`, `done`, `unconfirmed` (with `doubt`), `failed`, `stopped`,
`notStarted` (`earlierDevice`, `stopRequest`, `serverStop`, `runEnded`),
`notInBatch` (with `reason.code`) and `deselected`. Simulator and fake-tunnel
tests only. Open for WP-38c: the **Download all…** / **Alle programmieren…**
review in the pending block and the download tab, batch consent, per-device
acknowledgements, status polling and en/de copy; wording for the new busy
holder.

## Step 2 (designed)

No batch start button exists yet. The design is
[ADR-0172](../adr/0172-batch-programming-stops-at-the-first-doubt.md)
(Proposed, 11 November 2026): a server-held batch over the regular memory
download; devices with a pending or missing individual address, nothing to
download, `unsupported`/`excluded` readiness or the System-2 path are listed as
*not in this batch*, as is a device whose advised-scope plan grades
`unsupported` or allows no post-download check; all plans and phrases prepared
and checked before the reservation is taken; one confirmation naming the devices (shown even with a
remembered stage consent) plus each `untested` device's own acknowledgement;
continue only after a written, restarted and fully matching checked device,
otherwise stop with the rest `notStarted`; *stop after this device*;
a result per device and per-device history. Implementation follows with
simulator-only tests unless hardware is separately authorised.
