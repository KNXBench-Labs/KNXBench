# Telegram-flow evidence and capacity

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-154)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 154. The telegram-flow view is checked and measured in Chromium only

**Update 2026-10-07 — readability and source windows.**
[ADR-0085](../adr/0085-readable-flow-and-source-windows.md) and
[Flow §23](../TELEGRAM_FLOW_VISUALIZATION.md#23-readable-area-source-windows-and-project-links-2026-10-07)
supersede the fixed box, reserved Inspector and activity-clustering description.
Responsive sizing, closable/maximized details, switchable auto-fit and shared
source-window history are implemented. Long visual labels are ellipsized, with
full text retained in the Inspector/accessibility path. Sampled curve-crossing
and foreign-footprint avoidance tests supplement the busy-hub bounding-box guard;
**arbitrary graphs can still cross**, edge labels can still meet other paths,
and fitted dense graphs can make text small. Detailed obstacle routing is reduced
with a visible note beyond 80 nodes or 250 edges. No recorded edge is hidden.

Secondary windows require BroadcastChannel and the same source lifetime. They
retain the last map, mark it non-live and disable links when the source is lost;
no persisted history or automatic rebinding after source reload. Source capture
and context warnings remain visible. Full peer snapshots are not load-certified
for many simultaneous windows or hours of retained history. Project links require
a main-editor-bound scope; diagnostic-only sources cannot establish it. Replacing
a project with identical participant facts can leave old generation links
unavailable until a new interpretation/session rather than risk reused ids.

Current evidence: full Chromium offline suite and unit/build gates; native
capability compilation plus adapter outcome tests, **not** a real WebKitGTK or
Orca run and not new live-bus evidence. Fresh production-load samples (one each):
232 nodes / 239 edges, motion, 200 telegrams/s → main-thread share 0.523, marker
lag max 33 ms; 502 nodes / 2,490 edges, Motion Off, 1,000/s → share 0.335, lag
327 ms. Existing large-map Motion Off guidance remains. Measurements below
are the historical U20/U21/AR21 receipts, not new measurements of this renderer.

<a id="154-the-telegram-flow-view-is-checked-in-chromium-only-load-figures-follow-in-u21-part-c"></a>

<a id="154-the-telegram-flow-view-is-static-and-checked-in-chromium-only"></a>

**Status.** Introduced by U20 (2026-10-04); motion added by U21 parts A and B;
local reheat and hub separation added by the AR21 corrections (2026-10-05,
TELEGRAM_FLOW_VISUALIZATION §14).
The bus monitor's **Flow** view (`TelegramFlowView.tsx`, reducer
`flowModel.ts`, animator `flowAnimator.ts`; rules in
TELEGRAM_FLOW_VISUALIZATION §11 and §12) is fed by the monitor's own poll
loop and the AR20 snapshot route. It is read-only.

**What remains.**

- **Configured, not received.** Lines to devices are project configuration.
  Values at configured members are inferred from the group address, not read
  back. This is the intended meaning and is stated in the view and the guide.
- **One path per pair.** Several group addresses between the same two nodes
  share one curved path; the label names two and counts the rest, and the
  Inspector lists all.
- **Load (measured, one machine).** On a Ryzen 7 5800X with headless
  Chromium and the production build, motion keeps the main thread about 21 %
  busy at 10 telegrams/s and about 69 % in a 200/s burst over 230 nodes;
  Motion Off brings the burst to about 6 %. Values appear 10–20 ms after their
  poll answer, up to about 150 ms in the burst with motion. Above 160
  simultaneous pulses, telegrams are counted instead of drawn, and the view
  says so. Slower hardware and the packaged WebKitGTK app were not measured
  (docs/design/2026-10-04-telegram-flow-u21/).
- **The §7 starting load saturates motion (measured, accepted envelope
  pending AR21).** With 500 devices, ~2,500 lines and ~1,000 telegrams/s the
  main thread stays above 90 % busy with motion on, with long tasks up to
  about 0.33 s, even after the layout has come to rest; the cost is painting
  the large SVG, not layout. Values then appear up to about 0.4 s after their
  poll answer (up to ~1.9 s while the first layout settles). Motion Off brings
  the same load to about 30 % (one sample on a busy host). There is no
  automatic switch to Motion Off.
- **Readability is approximated.** Neighbours keep clear of a node's circle,
  name and value lines using an estimated text width of about twelve
  characters; longer names can still touch, edge labels can cross node text,
  and value lines may reach below the lower edge of the drawing area.
- **Evidence.** Unit tests (happy-dom, fake scheduler) and Chromium e2e with
  intercepted synthetic traffic (`e2e/telegram-flow.e2e.ts`,
  `e2e/telegram-flow-motion.e2e.ts`, frames and intervals counted in the
  page; `e2e/telegram-flow-hub.e2e.ts`, rendered boxes of a settled busy
  hub). No real-bus evidence, no native WebKitGTK run, no screen-reader
  check. Values and lines are hidden from assistive technology on purpose;
  the Inspector is the accessible path, and it has not been tried with Orca.

**Update 2026-10-05 (AR21 acceptance).** Measured again with motion on: a map
growing to 500 nodes and ~2,500 lines keeps the main thread 88 % busy at only
2 telegrams/s, with values up to 1.1 s late, so the saturation above depends
on map size rather than on 1,000 telegrams/s. The accepted Alpha envelope is
motion for small and medium maps and Motion Off for several hundred nodes or
the §7 load; there is still no automatic switch. A 900 s small session stays
light (17 % busy, 5–7 ms lag) but its heap rose from 5.4 to 6.2 MiB without a
visible plateau; a leak is neither shown nor excluded over hours. The
reduced-rendering note now also counts telegrams the full map or the 2,048-event
drawing buffer could not draw (AR21 findings 6 and 7). Details:
[TELEGRAM_FLOW_VISUALIZATION §22](../TELEGRAM_FLOW_VISUALIZATION.md#22-ar21-rerun-of-findings-6-and-7-and-acceptance-alpha-2026-10-05).
<!-- END retained catalogue record -->
