# WP-11 — Native Linux acceptance

**Status:** In progress: bounded native editor, capture and real-pointer cells recorded; Properties has historical and pinned current-source six-cell pointer counterparts at 80/100/150% (3 November), with an explicit software-render profile. Broader current-source parity and the wider matrix remain open. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Record actual native behaviour for named workflows with fictional or
simulator data (no bus contact needed):

- WebKitGTK keyboard, focus, draft and dialog behaviour, including the
  notes, instruction and checked-parameter editors.
- Orca / screen-reader behaviour.
- Wayland save/cancel/overwrite paths and the bus-monitor capture export.
- Application zoom cases on WebKitGTK (KL-130; bounded native controls now
  recorded, remaining pointer/layout/hover matrix open).
- AppImage startup smoke on a newer-Mesa host (issue #6).
- Firefox for the app.

## Consent and scope — 27 October 2026

The owner explicitly authorises desktop control for these bounded native tests
with isolated fictional/simulator data, including keyboard/focus, dialogs, zoom,
save paths and screen-reader checks where available. No bus access or changes
to real projects are authorised. Consent is granted; the first bounded native Alpha 17 slice below is
separate executed evidence, not consent alone. Missing environments remain
explicitly untested.

## First bounded slice recorded — 27 October 2026

The published Alpha 17 AppImage was exercised in an actual isolated native
Wayland/WebKitGTK window. Save/Cancel/Replace, exact note save/reopen before and
after process restart, settings focus wrap/return, 80/150% shortcuts, bounded
keyboard table/pane controls and File submenus have resulting state/file
evidence. See [acceptance and exclusions](../status/2026-10-27-wp11-native-acceptance.md).

At that first cutoff the fictional demo only established checked-editor
`unsupportedXml` refusal and instruction `MissingSource` feedback, not positive
editing. Those historical outcomes remain refusal evidence.

## Positive native editor cells — 27 October 2026

A separate [positive continuation](../status/2026-10-27-wp11-positive-native-editors.md)
uses independently reviewed original fictional source-20 data. It verifies one
declared integer slider's Cancel/Apply/Save/Undo/Redo and native-created-device
plain instruction preview/cancel/confirm/save. Read-only saved-result oracles
and a real native process restart verify slider `7` and exact instruction text.
This is not all checked controls or imported-original instruction editing.
Virtual-key replacement remains unestablished; physical-keyboard comparison
and product-cause attribution are missing.

## Bounded native capture export — 27 October 2026

The [capture continuation](../status/2026-10-27-wp11-native-capture-export.md)
uses one source-bound fictional UDP peer and the actual Alpha 17 backend,
not mocked Tauri/HTTP. Native Export Cancel creates no capture; Save writes
one real received row with JSON-v1 session/loss provenance; overwrite Cancel
preserves exact bytes/mtime and Replace changes only the export timestamp.
Actual heartbeat/disconnect and ordinary native/peer closure are recorded.
The original/replaced JSON and original-source fixture discrimination remain
separate retained evidence. This is one bounded capture cell, not protocol,
hardware, complete bus-history or whole-WP-11 acceptance.

Remaining work includes other controls, imported-original instructions,
virtual-key replacement attribution, keyboard/pointer/layout cells and
vendor-specific EGL/GPU evidence. Orca and Firefox remain untested; their
previously missing environments were not installed by this continuation.
No ledger row is closed.

## Real-pointer drag and hover cells — 28 October 2026

The [pointer continuation](../status/2026-10-28-wp11-native-pointer-cells.md)
drove the real compositor cursor at 80/100/150% with fictional data. Completed
navigation-pane drags follow the pointer and scale correctly. Table column drags
are stable and reversible: exact at 100/150%, fit-stretched at 80%. Topology
device hover keeps its geometry and shows the highlight at all three scales.

**Defect, fixed in source 28 October 2026:** the pane separators and height
splitters did not cancel the `pointerdown` default. In WebKitGTK a drag could
select document text, and later drags then stopped after their first motion
without committing. Both now apply the table column handle's guards: they
cancel the primary `pointerdown` default, block `dragstart`, set
`user-select: none` and take focus; a primary press also clears an existing
selection, as the native press did. Unit tests pin these guards. A Chromium
test checks repeated drags still commit without a selection, but Chromium never
showed the loss, so it also passes without the fix. The
[source-bound native re-check](../status/2026-10-28-wp11-native-separator-recheck.md)
now records 12 cells / 42 real-pointer drags on a fresh build containing the fix,
at 80/100/150%. Repeated ±100 drags and the height splitter complete, including
immediately after selecting text elsewhere; all post-drag selections are empty.
Width settings match each announced value. Pointer-away focus crops show the
accent outline, byte-identical to native keyboard-refocus controls; this is an
observed appearance, not a keyboard-only ring policy assertion. The historical
pre-fix defect remains recorded; the bounded drag-after-selection trigger check
passes. This sequence is not a replay of the exact original +100 then −5
sequence with a live selection before the small return drag.
At that 28 October cutoff, Properties-pane drags, other sizes/themes and the
wider native matrix were still untested; no ledger row was newly closed.

## Properties pointer continuation — 3 November 2026

The [bounded Properties record](../status/2026-11-03-wp11-native-properties.md)
adds six actual Wayland/WebKitGTK cells and 18 separator drags on retained source
`1a4f5a970`, with unchanged executable/frontend/fictional input bytes. At each
80/100/150% scale, repeated drags and a real preselected-text case commit exact
widths to `inspectorPaneWidth`; inverse legs restore 360 and post-drag selections
are empty. All 14 final-run seat receipts and ordinary teardown are verified.
The explicit software-render profile, historical artifact, no Properties restart
test and missing per-zoom image-control matrix remain disclosed. This is not
current-source, AppImage/GPU, Orca/Firefox or whole-WP-11 acceptance; ledger status
is unchanged.

## Current-source Properties counterpart — 3 November 2026

The separate [current-source record](../status/2026-11-03-wp11-current-native-properties.md)
and [measurement](../evidence/wp11-current-properties-native-2026-11-03.json) bind
the same six-cell/18-leg Properties subset to freshly built source `6ecb69427`,
not a later documentation HEAD. Current executable/147-file frontend hashes and
the unchanged fictional fixture are recorded. The first 100% preselected-text
pair passed 360 → 408 → 360 before the other five cells. The parent audit of 36
full before/after native trees and 18 actual settings byte captures matches all
historical step observables; it does not assert full-tree or pixel parity.
All 14 seat restores, ordinary exit and owned process/window/rule/private-runtime
retirement are verified. No production code changed.

The historical-source Properties record above remains intact. Software-render
flags and the host session-D-Bus/AT-SPI exception remain explicit; this is not
AppImage/GPU, X11/XWayland, internal WebKit sandbox/full host-service attestation,
Orca/Firefox, other sizes/themes, boundary clamping, Properties restart or
save-roundtrip acceptance. Earlier non-Properties cells are not promoted to this
source; broader current-source/native coverage and WP-11/KL-130 remain open.
Ledger status is unchanged.

## Done when

Native actions and resulting files/states are recorded for each tested cell;
unsupported environments stay explicit. Chromium, Broadway startup and a mock
Tauri runtime are not substitutes. Not a GTK4 port or new window policy.

## References

[KL-020](../limitations/KL-020-overlay-accessibility-evidence.md) ·
[KL-130](../limitations/KL-130-ZOOM-application-zoom.md) ·
[AppImage launcher](../APPIMAGE_LAUNCHER.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-11--native-linux-acceptance)
