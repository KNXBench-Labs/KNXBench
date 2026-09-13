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
# Terminal 1 — the dev server. No backend is needed; the script intercepts
# every /api/** request itself.
npm --prefix apps/knx-web run dev

# Terminal 2
PLAYWRIGHT_MODULE=/absolute/path/to/playwright-core/index.mjs \
node apps/knx-web/scripts/workbench-browser-proof.mjs \
docs/design/2026-09-13-codex-ui-proof 'http://[::1]:1420'
```

**Use the IPv6 literal.** `vite.config.ts` sets `port: 1420` with
`strictPort: true` and no `host`, so the dev server binds `[::1]:1420` and
*only* that — there is no IPv4 listener. Confirmed on 2026-09-13 with
`ss -ltn | grep 1420`, which reports one `LISTEN … [::1]:1420`. A
`127.0.0.1` URL is refused outright (`curl` exits 7 with an empty body), and
an empty body is easy to mistake for a server that answered with nothing;
one verification pass lost time to exactly that. Judge the health check by
its exit status, not by its output. The `http://127.0.0.1:1427` shown here
previously matched neither the dev server (1420) nor `vite preview`'s
default (4173), and does not reproduce.

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

## Provenance of the committed images

Every `.png` in this directory was produced by the run recorded in
`checks.json` and has not been regenerated since. The script has moved on:
it now also drives the device inspector's third tab and writes
`01b-porcelain-product-data.png`, which is **not** committed here, and
`01-porcelain.png` correspondingly shows a two-tab inspector where the
application now has three. Compare with
`git log --oneline -- apps/knx-web/scripts/workbench-browser-proof.mjs
docs/design/2026-09-13-codex-ui-proof/`: the images sit at the branch's
base commit, the script two commits later. Treat the screenshots as
evidence of the run that made them, not as a current portrait of the UI,
and re-run the script before citing them as either.

The gateway field in the monitor screenshots shows a fictitious
`192.168.1.10:3671`; it is an input placeholder, no such host exists, and
no gateway was contacted in any run. New material should prefer the
RFC 5737 placeholder `192.0.2.1`, which is what `BusMonitorPanel.tsx`
renders today.
