# WP-11: native real-pointer cells — 28 October 2026

## Result and boundary

The actual Alpha 17 desktop was driven with the **real compositor pointer** at
80%, 100% and 150% application zoom, using fictional data only. Three KL-130
pointer cells now have native evidence, and one product defect was found
and is recorded as open:

| Cell | Result |
| --- | --- |
| Navigation pane separator, pointer drag | Whenever a drag completes, the edge follows the pointer (±100 px → ±100/±101 px) and the announced width changes by pointer/zoom (+100, −125, ±67). **Drags are intermittently cut short** after a drag has created a document text selection. |
| Navigation block height splitter (150%) | First drag correct (60 px → +40). Later drags moved 2–4 px; a selection appeared after each forward drag. |
| Device table column (Name) | 4/4 drags complete and reversible at every scale, no selection. Exact at 100% (+80) and 150% (+53). At 80% the table is stretched to fill the width and announces +111 per 100 layout px (edge +77 for 80 px). This matches the fit distribution and is not classified as a defect. |
| Topology device hover | At all three scales the card geometry stays still, the background highlight is visible, and icon, name, address and object count are not clipped. Checked against crops. |

This record covers bounded cells only. It is **not complete WP-11**, does not
fix anything, and does not lift KL-130. The
[public receipt](../evidence/wp11-pointer-2026-10-28.json) pins the artifact,
fixture, per-drag numbers, crop hashes and exclusions.

## Target and authority

The owner gave an explicit Go on 28 October 2026 for real-cursor WP-11 pointer
cells with fictional data. The target is the retained, authenticated
`KNXBench_0.1.0-alpha.17_amd64.AppImage` (source `a9ea32d72`, SHA-256
`cd1f0cdf…49e1d8a`), restored from the sealed WP-11 archive. The launcher
checks that hash before starting. The fixture is an untouched copy of
`demos/1.0.0/office-building.knxdb` at the target source. No product code,
release or image changed. No bus was contacted and no real project was opened.

## Isolation and method

The app ran in the existing WP-11 sandbox shape: a separate loopback-only
network namespace, a read-only host, a masked real home and projects, and
private HOME/XDG directories. A compositor window rule placed it silently on its
own workspace on a physical output, so it never covered the user's working
window. After every cell the visible workspace, keyboard focus and cursor
position were restored to their saved values.

Pointer input came from a small standard-library Wayland client that uses
`zwlr_virtual_pointer_v1`, with absolute motion on one named output and the
left button. The compositor therefore moved the one real seat cursor, and every
action was checked by reading back the compositor's cursor position. Before pressing, the
script required the sandbox window to be active under the cursor. AT-SPI was
used for reading values, bounds and text selections, and a screen-capture tool captured the
hover crops. No JavaScript injection, mocked HTTP or mocked Tauri was used.

## Found defect (open)

`ResizablePane` (navigation/properties separators) and `PaneSplitter`
(navigation block heights) start a drag on `pointerdown` with pointer capture,
but they neither call `preventDefault()` nor set `user-select: none`. In
WebKitGTK a separator drag can therefore select document text, here the
Project status block. Later separator drags then receive only their first
motion event, if any, and never commit. The stored width keeps the last
committed value.

Exact reproduction from a cleared selection at 100% (pointer ±100 px):
+100 (committed, selection created), −5 (not committed), 0, 0. The last
press cleared the selection. The table column handle is the control. It calls
`preventDefault()` on `pointerdown`, blocks `dragstart`, sets
`user-select: none` and never lost a drag in 12 attempts. Both affected
components are unchanged on `main`.

This finding is attributed from source differences plus native observation.
That the next drag becomes a native drag-and-drop of the selection is a
plausible mechanism, but it was not separately observed. Chromium tests did
not exercise repeated real drags with a live selection.

**Amendment 28 October 2026 (source fix).** `ResizablePane` and
`PaneSplitter` now cancel the primary `pointerdown` default, block
`dragstart`, set `user-select: none` and focus the separator, matching the
control; a primary press also clears an existing selection. This record's observations stay as recorded; they describe the
build before the fix. Whether the fix removes the native loss is not yet
observed: the reproduction above has to be repeated in WebKitGTK.

## Harness observations (not product results)

- The first path entry went to the main window instead of the same-PID file
  chooser. The driver now focuses the exact window address. Afterwards one
  separator was briefly missing from the main window's tree; it reappeared
  once the project was open.
- `Return` in the chooser's location field did not confirm the dialog; the
  native Open action did.
- One run labelled `c1-z80` actually ran at 100% because a stale environment
  variable refused the zoom keys. It is kept only as an interruption
  observation (+40 of +100).
- Under 80% zoom, WebKitGTK reported separator x positions in visual pixels
  but heights in layout pixels. Click targets used x centres only.

## Still open

- Fixing the defect above, followed by a native re-check on a build that
  contains the fix.
- Pointer drag of the Properties pane, other window sizes and themes, Orca,
  Firefox, and vendor-specific GPU/EGL evidence.
- The KL-130 ledger status is unchanged.

Private evidence: `<profile>/evidence/wp11-pointer-20261028/` (cells.jsonl,
actions.jsonl, AT-SPI before/after trees, crops, scripts, launch receipt).
