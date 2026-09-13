# Implemented UI — browser evidence

These are Chromium screenshots of the implemented application, not generated
images. The browser script intercepts every API request with clearly named
example project/device/telegram data. No live backend or KNX hardware is used.
Unexpected API requests and browser errors fail the script.

- `01-porcelain.png`: building/device workspace, Porcelain.
- `02-graphite.png`: group address workspace, Graphite.
- `03-busmonitor.png`: Mint/comfortable monitor with selected telegram details.
- `monitor-1280.png`, `monitor-1920.png`, `monitor-960.png`: responsive layout.
- `checks.json`: results and timestamp of the reproducible browser checks.

Run from the repository root with an installed `playwright-core`:

```sh
PLAYWRIGHT_MODULE=/absolute/path/to/playwright-core/index.mjs \
node apps/knx-web/scripts/workbench-browser-proof.mjs \
docs/design/2026-09-13-codex-ui-proof http://127.0.0.1:1427
```

The checked workflow opens a file and selects a device with keyboard events,
switches device tabs, changes theme/accent/density in Settings, selects a
telegram using Enter, verifies live System light/dark changes, and tests page
overflow at 1920/1280/960 CSS px. 960 CSS px approximates the layout available
at 200% scaling on a 1920 px desktop; it is not native mixed-DPI verification.

Motion evidence: the workflow first emulates reduced motion and verifies a
zero-duration button transition, then restores normal motion, moves the pointer
onto Settings and records the real `transitionend` event. `checks.json` records
the measured background transition duration (120 ms). This proves the examined
hover feedback, not every animation or every assistive-technology workflow.

Remaining proof: large/long data sets, complete mouse/keyboard CRUD and bulk
workflows, native Tauri behavior and safe diagnostic window coordination. The
screenshots prove rendering with fixtures; existing unit and server tests cover
other functionality. This evidence does not mark the overall Goal complete.
