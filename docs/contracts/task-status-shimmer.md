# Active-task status text shimmer

Owner-approved on 14 October 2026. Presentation only, main-only source after
Alpha 11; no release, protocol, schema, API or stored-data change.

## Behaviour

- Keep the real status text readable. Do not replace it with skeleton bars,
  duplicate it for animation, change live-region roles or announce animation frames.
- A narrow, monochrome glint sweeps left to right, then rests completely outside
  the text before restarting. Standard: 2.5-second sweep plus 1-second pause.
  Subtle: approximately 2.57-second sweep plus 1.03-second pause (3.6 seconds
  total), using the existing 120 ms motion clock rather than accelerating it.
- Smooth and Glitch both use a steady linear sweep, never ping-pong or blinking.
  CSS keyframes hold the end position from 5/7 of the cycle until its end.
  The gradient extends beyond the glyphs at both ends, hiding the reset.
- Existing active-state predicates, not text matching or `role=status`, install
  `task-status-text` on the text fragment. Remove it when the operation settles,
  fails or needs a decision/retry. A server-confirmed running download or scan
  may still animate while its measured counters change; those counters do not.
- Preserve real progress bars, totals, errors, warning roles, target identities
  and original status wording. The import heading, count and humour stay static;
  only its current phase shimmers, including the first pending status read.
- Off, OS reduced motion, forced colours and printing show plain readable text.
  Browsers without text background clipping, colour mixing or transparent glyph
  fill support keep the same plain-text fallback. All three capabilities are
  required before applying transparent fill.
- Use opaque mixtures of the existing text tokens, not new palette literals.
  The dimmest ordinary text mixture is 70% foreground + 30% muted; built-in
  base palettes are tested at 4.5:1 on both background and surface. Warning
  fragments inherit their parent's warning ink rather than becoming ordinary ink.

## Bound call sites

The shared stylesheet is `apps/knx-web/src/styles.css`; no component, dependency,
JavaScript timer, observer, shared task registry or domain logic is added.

Existing textual work indicators are bound in project import/open, authentication
checks, device/editor/parameter loading, object-inventory reads, catalogue preview,
project history, rename saving, notes/instructions/functions reads, theme-file
reading/saving, selective/contribution import, Flow export, stored bus-access reads,
Log loading, gateway discovery, device-readiness/status checks, running device
programming, address programming, line scan/comparison, activity reads and release
checking. A pending download handoff animates only while its actual checks run.

## Deliberate boundaries

- Ordinary connection states, retained results, toasts, autosave countdowns,
  safety instructions awaiting the programming button and retry/confirmation
  prompts stay static. A bus stream is not an endlessly running task label.
- Action buttons and file-input captions keep their labels and existing disabled
  semantics; the effect targets status text, not every busy control. The old
  gateway-search button pulse is removed because the status text now carries
  activity feedback. No dedicated adjacent spinner was present in these callers.
- This does not add missing progress messages, estimate completion time or
  certify hardware/ETS compatibility. There is no new setting or migration.
- Arbitrary user-imported themes retain their existing palette authority; this
  does not certify the contrast of every theme file a user can supply.
- Chromium is the browser acceptance target. Native WebKitGTK painting and Orca
  have not been verified for this effect; plain text is the unsupported-clipping
  fallback, not a claim of native screen-reader acceptance.

## Regression evidence

`LoadProgressBanner.test.tsx` pins running/initial versus terminal states without
changing measured progress. `DeviceDownloadPreparation.test.tsx` holds real
component reads pending and proves settled errors and discovery results stop
shimmer. `taskStatusText.test.ts` pins the CSS gates, timings, warning inheritance
and built-in base-palette contrast. `import-status.e2e.ts` checks actual animation
positions across the sweep, pause and next cycle, Off/reduced motion/forced
colours, unchanged text and progress, EN/DE, 1440/400 widths and terminal failure.
`playwright.task-status.config.ts` repeats import and download-preparation tests
against the production bundle. Acceptance counts belong in the delivery record,
not assumptions in this contract.
