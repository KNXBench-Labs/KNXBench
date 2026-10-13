# Selected-view image export — researched contract (AP2)

Candidate design, not yet accepted or published. Researched 2026-10-10.

## Evidence before implementation

Primary sources:

- CompuServe GIF89a specification, hosted by W3C:
  https://www.w3.org/Graphics/GIF/spec-gif89a.txt
  Sections15–23 and AppendixF: global color table, little-endian dimensions,
  frame control delay in hundredths of a second, image data subblocks up to255
  bytes terminated by0, and little-endian packed LZW codes. A clear code may
  appear at any point, resets code width, and each image ends in EOI.
- WHATWG HTML Canvas serialization/origin-clean rules:
  https://html.spec.whatwg.org/multipage/canvas.html#dom-canvas-toblob
  PNG uses the browser encoder; a tainted canvas refuses serialization rather
  than authorizing cross-origin reads.
- W3C SVG2 viewBox/coordinate rules:
  https://www.w3.org/TR/SVG2/coords.html#ViewBoxAttribute

GIF and Graphics Interchange Format are attributed to CompuServe Incorporated
in this technical/user-facing documentation. No vendor KNX/ETS format involved.

## Selected scope and policy

Only the visible SVG graph viewport, selected explicitly in an export dialog.
Not the page, sidebar, Inspector, raw log, earlier history, or project bundle.
Names/addresses/values visible in the graph may be sensitive: affirmative
checkbox before an explicit download; no automatic upload or persistence.
The dialog's preview is a current viewport snapshot, not a claim of a complete
telegram record. SVG exports are not offered; serialized source is transient.

PNG: one browser-rasterized image, max960x540, aspect ratio preserved.
GIF: user-selected1–30seconds (user clarified max30s), default3seconds,
10 chronological samples/second, max300frames. Visible resolution declared
before Start: max640x360 for1–5s,480x270 for6–15s,320x180 for16–30s;
aspect ratio preserved, deterministic RGB332 palette, no looping extension.
A viewer may override repetition. Reduced
motion is captured as seen (no invented pulses). A visual sample, not a bus
recording. Longer clips reduce dimensions, never discard selected frames.

Limits: one job/window; no background ring/recording; max4000 SVG elements,
1MiB serialized SVG per sample,32MiB output,300 GIF frames, selected duration
plus5s wall deadline (max35s).
One raster/palette frame at a time; retain encoded chunks only, not full RGBA
history. No remote images, foreignObject, scripts, stylesheets, or external URL
references accepted in rasterization. Inline actual computed SVG presentation.

Cancel before download discards partial output. Abort on hidden document,
view/session/project source replacement, unmount, or timeout. No partial GIF
is downloaded. Encoding/raster/download errors are localized and visible;
retry requires a new explicit action. A completed download is not revocable.

## Encoder choice

Small dependency-free GIF89a writer: deterministic RGB332 palette, full frames,
fixed nine-bit literal codes, clear at most every200 pixels (before dictionary
width grows). Larger files than a dictionary compressor; bounded by the stated
caps. No proprietary compression/extension behavior guessed. Actual independent
image decoding and expected frame/duration/pixel checks required before acceptance.

## Animation boundary

Preserve existing feed/reducer/order, count and capacity diagnostics. Smooth
camera transitions only; retain measured32ms draw throttle and all existing
pulse/overflow/hidden-page contracts. No increase in paint work without evidence.
Category/global/OS Off must disable new CSS movement. Measurements/visual
comparisons from synthetic offline graph only; no real bus connection needed.

## Candidate implementation evidence (not browser acceptance)

Focused source/adapter/dialog/theme/motion tests: 174 passed in 9 files;
source, production-fixture/spec typecheck and production bundles passed.
Real-time production CaptureService, compiled with the project Vite SSR
bundler, produced a synthetic-raster control: 300 frames/disposals, 320x180,
19,621,382 bytes. Independent Pillow opened every frame, checked every
chronological pixel sample and summed delays to exactly30,000ms.
This is NOT evidence of native SVG rasterization or browser downloads.

Native production-browser EN/DE 1440/400 download/cancel specs are compiled
under the own preview configuration; actual execution awaits the shared
workspace/browser leases. Camera paint/CPU evidence, real decoded PNG/GIF
and visual inspection remain acceptance blockers. No full-suite pass,
publication or source/ETS/hardware compatibility is inferred.

Whole selected SVG DOM is subject to the4,000-element/source-text/serialized
1MiB budgets even if some nodes are outside its viewport; zooming alone
does not guarantee a smaller DOM. Large/unsafe/late captures refuse visibly,
not truncate. No webfont assets or page stylesheets are embedded; system font
fallbacks can differ. On view unmount cancellation discards the dialog and
partial output; a notice cannot remain in an already-unmounted view.

Native first-case evidence: actual production-built Chromium Flow fixture
EN1440 PNG960x380 and GIF320x127 opened by independent Pillow; GIF300
frames/30000ms/no looping extension, all frames decoded. PNG graph-only
content visually inspected. The declared long-clip tier has tiny labels
at a wide zoom level: zoom in before selection or use PNG for detail.
This is built component evidence, not full-workbench/HTTP-server acceptance.
A native compact-dialog assertion caught inherited flex stretch (358px
empty tail); alignment is scoped to the Flow dialog, not the shared shell.

Native final component cases EN/DE1440/400:4/0 on explicitly rebuilt
production fixtures, including compact-panel RED/GREEN and cancel checks.
Eight native PNG/GIF outputs independently decoded; GIFmax300 frames,
30000ms, no loop extension; shorter clips30frames/3000ms. All decoded.
Light/wide and dark/narrow screenshots inspected; no dialog overflow,
consent/actions visible. Full workbench, whole-web, native smoothing
performance/reference acceptance remain pending. The stale initial
ordinary-build rerun failed compact assertions; it is NOT acceptance.

Expanded native reference admission at55df730f: whole web2674/0 in173 files,
production/type/fixture builds green; browser50/2. Two selected GIFs visibly
refused as `tooSlow` on a busy host rather than skipping frames. This is an
explicit resource-limit boundary, not a timeout bug hidden by retry settings.
The test now surfaces that alert immediately. That run reused an earlier image
folder: its aggregate decoded-file receipt is rejected for final source binding;
only its newly created live GIF is individually useful (30 frames/3000ms,
21 distinct pixel frames). Final acceptance requires an empty new output folder.
Timing/frame/count guards remain unchanged. No performance/throughput promise.
