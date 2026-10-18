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
in [open work](../OPEN_WORK.md#ui-feedback-packages-18-october-2026).
