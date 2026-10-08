← Previous: [Documentation maintenance](../DOCUMENTATION.md)

# Visual assets: real application, fictional building

The manual uses the application's **Porcelain** theme (registry id `porcelain`).
The pictures come from Chromium running the built frontend against a real,
freshly built `knx-server`, not from a design mockup or invented API responses.
All device names, rooms and products come from the generated **Sample house**
or the first-project **Practice house** exercise. No customer project or live
bus capture belongs in this directory.

## Current manual media

| Asset | What it teaches | Capture |
| --- | --- | --- |
| `screenshots/porcelain-*.png` | Real workspace, dialogs, project/device wizards, parameter tabs, search, settings, monitor and reopened practice project | 1440 × 900, 1× scale; refreshed 8 October 2026 |
| [New project](workflows/new-project.gif) | Details → optional structure → review → creation; the result explicitly says **not saved yet** | Real UI actions, 960 × 600, 10 fps |
| [Add device](workflows/add-device.gif) | Explorer action → product → placement → address preview → confirmed creation | Real project mutation, fictional actuator; no hardware access |
| [Search](workflows/search.gif) | Ctrl+K → type `light` → filtered results → Escape | Search only; it does not connect to a gateway |
| [Telegram flow](readme/telegram-flow.gif) | Session-local traffic animation under the CRT theme | Earlier real component capture with **synthetic traffic**; not real-bus evidence |
| [Earlier catalog shortcut](readme/hero-add-device.gif) | Earlier dark-theme add/link demonstration through the catalog workspace | Retained historical media, not the current wizard recording |

[media-manifest.json](media-manifest.json) records exact dimensions, durations,
byte sizes and SHA-256 hashes for the refreshed screenshots and workflow GIFs.
The source snapshot and exercised checks are recorded in the documentation
validation evidence linked from [the implementation log](../IMPLEMENTATION_STATUS.md).
The PNGs are full workspace captures where surrounding navigation helps explain
placement. GIFs are scaled for GitHub; static screenshots preserve readable details.

## Reproduce the current captures

Prerequisites: the repository's Rust and Node versions, installed frontend
packages, Chromium at `/usr/bin/chromium`, `ffmpeg` on PATH, and Linux `unshare`
with unprivileged user/network namespaces plus `ip` from iproute2.

From the repository root:

```bash
npm ci --prefix apps/knx-web
npm run build --prefix apps/knx-web
cargo build --locked -p knx-server
export KNX_SERVER_BIN="$PWD/target/debug/knx-server"
cd apps/knx-web
unshare --user --map-root-user --net sh -c \
  'ip link set lo up && exec "$@"' sh \
  npx playwright test -c playwright.manual.config.ts
```

The config creates temporary project, settings and product-database directories,
generates the fictional `.knxproj`, and starts two servers **inside the isolated
network namespace**. Only loopback is available; the app's discovery calls cannot
reach the host's LAN. The second server supplies the login-form screenshot without
signing in. No saved credential or real installation is used.

The run writes PNGs to `docs/assets/screenshots/` and GIFs to
`docs/assets/workflows/`. Use `MANUAL_SCREENSHOT_DIR` and `MANUAL_GIF_DIR` to
redirect a review run. The two capture tests also create a device on the real
server and exercise the first-project tutorial's save/reopen path. They are not
part of the normal application E2E suite.

Source:

- [Capture spec](../../apps/knx-web/e2e/manual-screenshots.shots.ts)
- [Capture configuration](../../apps/knx-web/playwright.manual.config.ts)
- [Lossless GIF recorder](../../apps/knx-web/e2e/manual-recording.ts)
- [Fictional project generator](../../tools/manual_sample_project.py)

**If capture fails:** keep the failure screenshot and inspect the accessibility
snapshot before changing locators. A list of `nth` locators can become stale after
a toast is removed; reacquire the first remaining item. Rebuild before blaming the
UI: a stale `dist/` is an excellent source of very convincing old screenshots.
If Chromium reports **Socket path too long**, use a shorter task-owned `TMPDIR`,
or a short symlink to the prescribed scratch directory. Do not disable network
isolation just to make recording easier.

## What is deliberately not photographed here

> [!NOTE]
> **Not captured: native Linux file dialogs.** The browser picker is real but is
> not a GTK/Tauri dialog. To add native media, start a freshly built desktop app
> with disposable user-data/config directories, set English and Porcelain, use a
> fictional `.knxdb`, then capture File → Open and Save As using the host dialog.
> Crop desktop panels and unrelated windows. State the distribution/display backend
> and build commit. Do not relabel a browser picker as a native screenshot.

> [!NOTE]
> **Not captured: real bus activity or commissioning.** The monitor screenshot is
> intentionally disconnected, and the flow GIF's traffic is synthetic. A real-bus
> recording requires separate installation permission, a reviewed/redacted project
> and a narrowly scoped read-only session. Device writes need their own explicit
> target/scope approval and recovery evidence; documentation work grants neither.

Drag-and-drop has written instructions and existing browser tests, but no new GIF
in this package. For a future recording, use the fictional sample and move one
unaddressed device to a line/room in the **same installation**, or drag one group
address onto a communication-object link row. Verify the result on the real server
before encoding. A useful static screenshot beats a ten-second mystery tour.

## Before replacing media

1. Inspect the full-resolution screenshot, not just its filename.
2. Sample GIF frames around each click and inspect pacing, clipping and legibility.
3. Verify the output size and update the manifest after any optimisation.
4. Update the referring alt text and instructions in the same change.
5. Keep current and historical provenance distinct. A screenshot proves that
   screen existed in that build, not full ETS compatibility or accessibility.

Next: [Return to the manual](../manual/README.md) →
