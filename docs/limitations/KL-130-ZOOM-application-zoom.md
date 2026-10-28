# Application zoom

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-130-zoom)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 130. Application zoom is browser-verified, not native WebKitGTK-verified

**Limitation.** Whole-interface zoom is intentionally bounded to 80–150% in
10% steps; pane widths are bounded as documented in the user manual. The
interaction and responsive stack were exercised in system Chromium, not in a
running Tauri/WebKitGTK window. No desktop-rendering parity is claimed.

**Cause.** `zoom` participates in browser layout, so a change of rendering
engine can expose different viewport or font metrics. Headless browser tests
cannot prove native focus, resize or accessibility behaviour.

**Cost.** A Linux desktop installation may need a layout adjustment beyond
the Chromium evidence. The server still retains the raw, versioned settings
record; an out-of-range saved value is visually clamped rather than silently
overwritten or discarded.

**Lifted when.** A GUI-capable Linux run checks zoom shortcuts, pointer and
keyboard pane resize, hide/show and restart, and topology-device hover at
narrow, default and enlarged scales. No KNX hardware traffic is required.
<!-- END retained catalogue record -->

## Update 18 October 2026: resizable tables under application zoom

Resizable data tables convert DOMRect widths and pointer deltas into CSS
layout pixels by dividing by `--app-ui-scale` (the same rule as `Overlay`),
and keep `clientWidth`/`scrollWidth` and computed-style lengths as they are.
That split matches Chromium's standardized CSS `zoom` (Chrome 128 and later),
where DOMRect and `clientX` are zoomed while `client*`/`scroll*` and computed
lengths are not. It was verified only in system Chromium. WebKitGTK may report
DOMRect differently under `zoom`; if it already divides by the zoom factor,
the tables would correct twice and underestimate word floors at 150%.

**Also lifted only when** the GUI-capable Linux run additionally checks table
column fitting, pointer drag distance and announced widths at 80% and 150%.

**Known open defect (not part of the table fix).** Navigation and inspector
pane drags still add the raw pointer delta to a layout-pixel width, so at 150%
the pane edge moves 1.5 times the pointer distance (0.8 times at 80%). Tracked
in [open work](../history/OPEN_WORK_2026-10-25.md#ui-feedback-packages-18-october-2026).

## Update 19 October 2026: the File menu under application zoom

The File menu's flyout placement is now zoom-aware (UISET2-01). It divides the
panel's and row's DOMRect and the window size (`clientWidth`, `innerHeight`)
by `--app-ui-scale` and compares, caps and shifts in CSS layout pixels only;
its viewport-unit limits (`100vw`, `100dvh`) are divided by the scale in the
stylesheet. A small probe recorded what Chromium 154 reports inside the
root-zoomed document at 80%, 100% and 150%: the window size is never divided,
DOMRect and pointer coordinates are zoomed, computed lengths are not, and a
viewport unit renders at the zoomed size
([table](../contracts/file-menu.md#units-under-zoom)). Browser cases set the
scale through a stored `uiScale` setting and through Ctrl+Plus.

The first level is zoom-aware as well (UISET3-01): the menu measures its
first level in layout pixels and takes the scrolling, inline layout when it
runs past the bottom margin, and moves left when it runs past the right one,
so at 150% **Logout** stays reachable in a 1440 x 601 window (Comfortable
density, a session) and the menu stays on screen in a 660 px window. The two
media queries (650 px wide, 600 px high) still read the window and remain as
a floor; the toolbar's narrow layout follows the window at every scale, so at
80% a 640 px window keeps the inline File menu although it is 800 layout
pixels wide.

This is Chromium-only evidence, like the table rule above. WebKitGTK may
report DOMRect, the window size or the scroll height differently under
`zoom`.

**Also lifted only when** the GUI-capable Linux run additionally opens every
File submenu at 80% and 150% in a mid-width and a short window.

## Update 19 October 2026: pane drags and a setting-driven table case

The navigation and inspector pane drags (`ResizablePane`) and the left
column's block splitters (`PaneSplitter`) now divide the pointer delta, and the
splitter its measured DOMRect height, by `--app-ui-scale`; pane widths are
rounded to whole layout pixels. Keyboard steps are unchanged layout pixels.
This closes the open defect named in the 18 October update. Browser cases at
80%, 100% and 150% set the scale through a stored `uiScale` setting and drag
with a real pointer (`e2e/workbench-splitters.e2e.ts`). One table case
(`e2e/table-column-widths.e2e.ts`, "stored uiScale") drives the scale only
through the setting: a stored 150%, then the zoom shortcuts to 80%, then a
reload. It checks App's inspector stacking at 1200 px, table fit or local
scrolling, intact words and pointer tracking. Chromium-only, like the rules
above.

Evidence: six Vitest cases (`Workbench.test.tsx` four, `PaneSplitter.test.tsx`
two) and the 80% and 150% browser cases were RED before the fix (the
navigation edge ran 12 and 30 visual pixels off the pointer); 100% passed as
the control. Four source mutants each fail a named case: the pane delta, the
splitter delta, the splitter's observed height and its interaction-time
height left unscaled. Rounding the pane width to whole layout pixels is not
discriminated by these scales (60 / 0.8 and 60 / 1.5 are whole numbers).

## Update 27 October 2026: first bounded native zoom controls

The actual published Alpha 17 AppImage now has isolated Wayland/WebKitGTK
input evidence: shortcuts reached 80%/150%, keyboard Right increased table
column/navigation separator values at both scales, navigation hide/show worked
at 150%, and all three File submenus exposed their items at both scales in the
tested window. Stored zoom and navigation width survived process restart.
[Native record and exclusions](../status/2026-10-27-wp11-native-acceptance.md).

This supersedes the blanket absence of native evidence above, not the full
lifting conditions. Pointer-drag distance, complete table fit/announced widths,
hover, further window sizes/themes, visual clipping and actual screen-reader
output remain unverified. Native AT-SPI control/value observations and stored
settings are not a full rendering-parity result; the ledger stays unchanged.

## Update 28 October 2026: native real-pointer drag and hover

The [pointer record](../status/2026-10-28-wp11-native-pointer-cells.md) used
the actual Alpha 17 AppImage and the real compositor cursor at 80%, 100% and
150%. When a navigation-separator drag completes, the edge follows the pointer
and the announced width changes by pointer/zoom. The Devices table's Name column
is exact at 100% and 150% and stable at all three scales. At 80% that table is
stretched to fill the width and announces +111 per 100 layout px, consistent
with fit distribution rather than the double correction feared above.
Topology-device hover keeps the card geometry and content unclipped.

**Defect, fixed in source.** Pane separators and height splitters did not
cancel the `pointerdown` default. In WebKitGTK an earlier drag could leave a
document text selection, and later separator drags then stopped after their
first motion without committing. Since 28 October 2026 they use the column
handle's guards (see [WP-11](../open-work/WP-11-native-linux-acceptance.md));
a native WebKitGTK re-check is still open. The ledger status is unchanged.
Properties-pane drag, further window sizes and themes, and screen-reader output
remain unverified.
