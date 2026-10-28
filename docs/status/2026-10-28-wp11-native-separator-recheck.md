# WP-11: native separator selection-fix re-check — 28 October 2026

## Result and boundary

**The bounded native reproduction now passes:** 12 cells / 42 separator drags
on actual Wayland/WebKitGTK, at 80%, 100% and 150% application zoom. Repeated
navigation-pane drags commit, including after a real pointer selection of text
elsewhere. The navigation block's height splitter also completes repeated and
preselected-text drags at all three scales. No post-drag document text selection
or interrupted drag was observed in these attempts.

This is a source-bound check of the drag-after-selection trigger from the
[previous native defect](2026-10-28-wp11-native-pointer-cells.md#found-defect-open),
not full WP-11 acceptance, KL-130 closure, or an AppImage packaging result. The
[public receipt](../evidence/wp11-separator-recheck-2026-10-28.json) contains every
measured drag and the artifact/fixture/crop hashes. No runtime source changed.

## Target, authority and isolation

The owner renewed consent on 28 October 2026 for the bounded real-cursor native
re-check with isolated fictional data. A fresh release-mode native desktop was
built at source `1a4f5a9708532977375c461d3fc3f23add21c15b`, which contains selection
fix `1822a9756`. This is **not** the old published Alpha 17 AppImage. The source
still displays `0.1.0-alpha.17`; the source and artifact hashes, not that label,
identify the tested build:

- Native executable SHA-256:
  `bf51ea26b2fcecd4cb8325edf50e7de1793d888a9e925265824381bcd955042e`.
- Source-built external frontend manifest SHA-256:
  `4050577689ef8179a75a4d3f7f330b81f7d94cb78c59980ad9214520b18e2c2a`.
- Untouched fictional office demo SHA-256:
  `ddda4fd55e077f00bc34d5b0ca02dcd3c6d2d54da52f836ac7eea7e4f244dc2a`.

The build held the repository's actual shared execution leases. Offline
`npm ci`, the frontend production build and offline `cargo build --release -p
knx-desktop` all exited 0, with a separate target directory. The executed
`/proc` binary hash matched the build hash; the frontend was copied with it into
the native release resource layout.

The app ran with a whitelisted environment, private HOME/XDG state, a read-only
host, masked home/project trees, and a separate loopback-only network namespace.
The compositor reported a native, non-XWayland window. Real projects, the user
catalogue, bus, VM and external peers were not accessed. No WebKit sandbox-disable
option was set; this run does not independently attest WebKit's internal
subprocess sandbox. Mesa/EGL warnings occurred, but the native window rendered
and completed the cells; this is not vendor-specific GPU acceptance.

## Executed cells

Each scale used a clean-selection sequence and a second sequence immediately
after selecting the overview's `Project status` heading with the real pointer.
The selected heading text was `Project statu`; AT-SPI exposes the same selection
through ancestor text objects too, not three independent user selections.

| Separator | Pointer sequence per scale | 80% announced deltas | 100% announced deltas | 150% announced deltas |
| --- | --- | --- | --- | --- |
| Navigation width, clean | +100, −100, +100, −100, −5, +5 | +125, −125, +125, −125, −6, +6 | +100, −100, +100, −100, −5, +5 | +67, −67, +67, −67, −3, +3 |
| Navigation width, preselected text | +100, −100 | +125, −125 | +100, −100 | +67, −67 |
| Navigation block height, clean | +60, −60, +60, −60 | +75, −75, +75, −75 | +60, −60, +60, −60 | +40, −40, +40, −40 |
| Navigation block height, preselected text | +60, −60 | +75, −75 | +60, −60 | +40, −40 |

All 42 edge displacements equal the requested pointer displacement. Integer
compositor cursor readback is exact in 41 drags; the 150% small −5 request reads
−4, while its edge moves −5 and layout width changes −3. The cursor-readback rounding occurs only in that small 150% drag. These
sequences do not replay the exact original +100 then −5 sequence with a live
selection before the small return drag. Width values are rounded layout
pixels, so they scale as expected. Every width's settings-file readback matches
its announced value; inverse drags return to the starting value. Block height is
component state, **not a persisted preference**; its measured return is verified,
without claiming a settings write.

All six preselection cells have a non-empty native text selection before the
first separator press. Every separator drag ends with zero reported selections.
Pointer input uses the compositor virtual-pointer protocol, with the exact
sandbox required as the active window and sole window on its workspace before
pressing. AT-SPI reads the
bounds, values, focus and selections. There is no injected JavaScript, mock
Tauri runtime or Chromium substitute.

## Focus appearance after pointer drag

Every tested separator remains AT-SPI-focused after release and after moving
the pointer away without clicking. Native screenshots show a visible accent
outline/line at all three scales. For each of the 12 cells, the pointer-away crop
is byte-identical to a crop after native Tab / Shift+Tab keyboard refocus. The
contact sheet (all 12 pairs) and two representative native crops were visually
inspected; retained pixel profiles independently show the accent strokes.

This is an **observed appearance**, not proof of a desired keyboard-only ring
policy: the source uses `:focus-visible` for the outline and
`:hover, :focus-visible, :active` for the accent band; pointer focus is requested
explicitly by the fix. AT-SPI focus alone cannot establish a DOM pseudo-class.
No absent ring was inferred from pointer modality, and no assertion is made
that a pointer drag ought to suppress the ring. Earlier native keyboard input
in the same session can influence WebKit focus-visible heuristics; there is no
non-focused/non-hovered image control. Product intent for that policy remains
separate from this native observation.

## Restoration, teardown and harness limits

Before each input cell, the active user window, cursor and output workspaces
were recorded. All **20** seat-restoration receipts verify exact active-window
address, cursor position and all output workspace identities after the cell,
including the aborted setup guard. The ordinary native Close action removed
the owned window/process; both WebKit children were also absent, and the
launcher exited 0. The test's runtime compositor placement rules remain until
configuration reload; no shared compositor reload was performed.

Setup needed a correction: the chooser is titled `Open File`, not the assumed
project-open label. The exact-window guard refused typing, and the seat was
restored. A subsequent path entry needed native Return/autocomplete followed
by the chooser's own Open action. An initial capture-helper filename shadowed
a Python standard-library module and failed before seat input; it was renamed.
These are retained harness observations, not product failures.

Still untested: Properties-pane dragging, other sizes/themes, Orca, Firefox,
AppImage packaging/startup and vendor-specific GPU/EGL behavior. No ledger row
is closed. Private evidence:
`<profile>/evidence/alpha18-followups-20261028/native/` (build/launch receipts,
raw per-cell trees and settings, real-pointer readings, selections, focus crops,
verified summary, isolation and restoration/closure receipts).
