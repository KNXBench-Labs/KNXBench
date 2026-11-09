← Previous: [Documentation maintenance](../DOCUMENTATION.md)

# Visual assets: real application, fictional building

The current manual stills show the published **v0.1.0-alpha.19**, in light
**Porcelain** (`porcelain`) and dark **Graphite** (`graphite`). The optional
CRT/LCARS stills also show that release. Chromium runs the production frontend
against the real server extracted from the published Docker image, not from
a design mockup, moving development build or invented API responses.
All device names, rooms and products come from the generated **Sample house**
or the first-project **Practice house** exercise. No customer project or live
bus capture belongs in this directory.

## Manual refresh for Alpha 19 (1 November 2026)

All manual stills and the three workflow GIFs were recaptured from the published
`v0.1.0-alpha.19` image (index digest `sha256:a8c48252…`, tagged commit
`0b827abee`): its `knx-server` and production frontend, the fictional Sample
house and Practice house, in an isolated loopback-only namespace. New are the
**Report problem…** stills (`porcelain-report-problem-*.png`: the File menu entry
and the wizard's four steps) and the Settings page in the center workspace. The
diagnostic-export and report stills come from the tagged source's diagnostic
simulator and a production frontend built from identical application source: a real read of a fictional
device, a real wizard preview and a real ZIP download whose contents are checked
for the operation's `recording.jsonl`. The capture specs start the browser clock
on a neutral weekday so no date-dependent startup joke reaches a still, then
return to the real time. Exact files, sizes and hashes are in the
[manifest](media-manifest.json).

## Website highlight media (9 November 2026)

Ten PNGs and the [capture manifest](releases/alpha20/website/media-manifest.json) are in
`releases/alpha20/website/`. They show the `v0.1.0-alpha.20` tag (`1d3c7d43`): production
frontend, actual `knx-server` and the fictional Sample house, captured in an isolated
loopback-only namespace with no application response replaced. Scenes as below, plus an
ETS-style group-address CSV imported in Windows-1252 (descriptions on four addresses, one
new address with its datapoint type); its still shows the new address with description and
declared type. The re-recorded group-details clip comes from the same run; the project-work
and bus-flow clips remain the Alpha 14 recordings below.

## Website highlight media (27 October 2026)

*Superseded on the website by the Alpha 20 captures above; the files stay as history.*


Nine PNGs and the [capture manifest](releases/alpha17/website/media-manifest.json) are in
`releases/alpha17/website/`. They show the `v0.1.0-alpha.17` tag (`a9ea32d7`): production
frontend, actual `knx-server` and the fictional Sample house, captured in an isolated
loopback-only namespace with no application response replaced. Scenes: group addresses
(Graphite, LCARS, green CRT), the imported project's status page, a named version in
project history, the add-device wizard review (cancelled, nothing created), the parameter
workspace, a vendor-ZIP catalogue job stopped at its bound preview (nothing installed) and
the Ctrl+K search. The re-recorded group-details clip comes from the same run; the
project-work and bus-flow clips remain the Alpha 14 recordings below.

Reproduction: `apps/knx-web/e2e/release-media.shots.ts` and its config; see
[website media instructions](../../website/README.md#media).

## Alpha 14 website media (18 October 2026)

*Historical: `release-media.shots.ts` now captures the scenes above; the Alpha 14
manifest binds the earlier version of the spec by hash.*

Seven PNGs and the [capture manifest](releases/alpha14/media-manifest.json) are in
`releases/alpha14/`. They show the tagged Alpha 14 source (`8d104e2f`): production
frontend, actual `knx-server` and the fictional Sample house, captured in an
isolated loopback-only namespace with no application response replaced. Scenes:
group addresses (Graphite, LCARS, green CRT), the parameter workspace, the note
decision for a newly created, unlinked address, a vendor-ZIP preview of the
fictional demo package (nothing installed) and the diagnostic export up to its
read preview (start/export requests asserted absent). The website's three MP4s
come from the same tagged source; their recorders, hashes and encoding are named
in the manifest. README media are unchanged by this refresh.

Reproduction: `apps/knx-web/e2e/release-media.shots.ts`, `readme-hero.shots.ts`,
`readme-flow.shots.ts` and their configs; see
[website media instructions](../../website/README.md#media).

## Alpha 12 website and README stills (15 October 2026)

The ten current release-candidate PNGs are in [releases/alpha12](releases/alpha12/media-manifest.json).
They use the integrated Alpha 12 production frontend and actual server, a
fictional Sample house and one native device created through the real wizard.
All capture operations ran in an isolated loopback-only namespace. No application
API responses, device reads or programming successes were fabricated; no live bus
was contacted. The schema-11 sample's imported instruction source remains refused;
the instruction still deliberately shows a native-created device instead.

Seven website WebPs preserve the PNG pixels exactly. README uses the new light
addresses/parameters/topology stills and current theme gallery. Earlier wizard/
Flow videos and most manual screenshots retain their original dates and scope.
The final ten frames were visually reviewed; one scrolled support-preview frame
is evidence, while the readable initial form is the public gallery still.

Reproduction: `apps/knx-web/e2e/release-media.shots.ts`,
`playwright.release-media.config.ts` and `tsconfig.release-media.json`; see
[website media instructions](../../website/README.md#media). The manifest binds
application source, actual binary, spec/config hashes, dimensions and one passing
capture test. This does not certify native WebKitGTK, screen readers or hardware.

## Application header mark

The workbench header uses the text-free symbol in
`apps/knx-web/src/assets/knxbench-mark.png`, derived from the owner-supplied
`KNXBench-logo-small.png` here (source SHA-256
`9534e3835923c53a2bcef13826927cc18edd0e5f6765a0414df752c79ffce027`).
Both supplied logo variants include a wordmark; the PNG symbol was cropped to
`(72, 37, 424, 271)` (right/bottom exclusive), producing a 352 × 234 image.
Its near-black matte was removed for light/dark header use; the original
files remain unchanged. Edge alpha is `clamp((max(R,G,B) - 10) / 50, 0, 1)`;
RGB is unmatted against `(10, 10, 10)` and clamped to byte range.
The app bundles the derived image locally, fits it into 44 × 34 CSS pixels
without stretching, and keeps the adjacent KNXBench label and overview link.
The Alpha 9 login captures show the release logo and frontend version. Earlier
README recordings remain historical media; replacing stills does not date them anew.

## Current manual media

| Asset | What it teaches | Capture |
| --- | --- | --- |
| `screenshots/porcelain-*.png` | Light workspace, dialogs, project/device wizards, search, the Settings page, monitor, reopened practice project, history and the four group-address presentations | Published Alpha 19; refreshed 1 November 2026 |
| `screenshots/graphite-*.png` | Dark device overview, object/DPT and parameter editor, group addresses, installation properties, history, message details and login | Published Alpha 19; real server, same fictional project |
| `screenshots/user-modern-retro-green-crt-group-addresses.png`, `screenshots/lcars-group-addresses.png` | Optional themes used by the website gallery | Published Alpha 19; no live bus |
| `screenshots/porcelain-report-problem-*.png` | [Report problem wizard](../manual/user-guide/08-reports-and-diff.md#report-problem): File menu entry, problem type, contents, preview, save and send | `v0.1.0-alpha.19` tagged source: production frontend, real HTTP handlers and the regular KNX simulator on loopback (fictional device), real ZIP download. Reproduce with `apps/knx-web/e2e/diagnostic-guide.shots.ts` and `playwright.diagnostic-guide.config.ts` (`KNX_DIAG_GUIDE_OUT`) |
| [New project](workflows/new-project.gif) | Details → optional structure → review → creation; the result explicitly says **not saved yet** | Real UI actions, 960 × 600, 10 fps |
| `screenshots/porcelain-diagnostic-export-*.png` | Step-by-step [diagnostic export guide](../manual/user-guide/08-reports-and-diff.md#send-diagnostic-data): File menu, device and interface, preview and read, result with **Report problem…** | `v0.1.0-alpha.19` tagged source, 1 November 2026; production frontend, real HTTP handlers and the regular KNX simulator on loopback (fictional device, labelled *SIMULATED*), no live bus. Same spec as the report stills |
| [Add device](workflows/add-device.gif) | Explorer action → product → placement → address preview → confirmed creation | Real project mutation, fictional actuator; no hardware access |
| [Search](workflows/search.gif) | Ctrl+K → type `light` → filtered results → Escape | Search only; it does not connect to a gateway |
| [Telegram flow](readme/telegram-flow.gif) | Session-local traffic animation under the CRT theme | Earlier real component capture with **synthetic traffic**; not real-bus evidence |
| [Earlier catalog shortcut](readme/hero-add-device.gif) | Earlier dark-theme add/link demonstration through the catalog workspace | Retained historical media, not the current wizard recording |

[media-manifest.json](media-manifest.json) records exact dimensions, durations,
byte sizes and SHA-256 hashes for the refreshed screenshots and workflow GIFs.
The source snapshot and exercised checks are recorded in the documentation
validation evidence linked from [the implementation log](../IMPLEMENTATION_STATUS.md).
The Alpha 19 refresh covers **59 screenshots** (46 from the main capture, four
group-address presentations, four diagnostic-export and five Report problem
stills) and **three workflow GIFs**. Workspace PNGs are full captures at 1× scale:
1440 × 900 normally, 1440 × 1080 for history, the expanded DPT field and message
details; the diagnostic and report stills are element captures of their dialog. The extra height keeps the history notices
and DPT explanation visible; it is not a painted or redesigned interface. Wide
device/object tables can still scroll within their own pane. GIFs are scaled for
GitHub; static screenshots preserve readable details. Website WebPs are lossless
encodings of these actual PNG pixels.

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

For these exact **release** captures, use the published image rather than the
source build above. Extracting a stopped container does not start the application:

```bash
artifact="$(mktemp -d "${TMPDIR:-/tmp}/knx-alpha19.XXXXXX")"
container="$(docker create --network none knxbench/knxbench-server:0.1.0-alpha.19)"
docker cp "$container:/usr/local/bin/knx-server" "$artifact/knx-server"
docker cp "$container:/app/frontend" "$artifact/frontend"
docker rm -v "$container"
export KNX_SERVER_BIN="$artifact/knx-server"
export MANUAL_FRONTEND_DIR="$artifact/frontend"
cd apps/knx-web
unshare --user --map-root-user --net sh -c \
  'ip link set lo up && exec "$@"' sh \
  npx playwright test -c playwright.manual.config.ts
unshare --user --map-root-user --net sh -c \
  'ip link set lo up && exec "$@"' sh \
  npx playwright test -c playwright.manual-group-addresses.config.ts
```

Run this alternative from the repository root, on Linux matching the extracted
server architecture. `MANUAL_FRONTEND_DIR` overrides `dist/`; without it the config
keeps the normal source-build route. Record the image revision/digest and binary
hash in the manifest. The captures here bind to release source
`0b827abee222c459ce068cbce83a941691a16805` and published image index digest
`sha256:a8c48252786f029518096fa77fe4518987685bce73d5a832e950bc5a544f01ba`.

The diagnostic-export and Report problem stills need the regular KNX simulator,
which the image does not contain. Build it from the tag, and build the production
frontend (`npm run build` in `apps/knx-web`) from a checkout whose application
source matches the tag; for this refresh `git diff v0.1.0-alpha.19 -- apps crates`
was empty apart from the capture specs:

```bash
git worktree add --detach ../knxbench-tag v0.1.0-alpha.19
(cd ../knxbench-tag && cargo build -p knx-server --example diagnostic_simulator)
cd apps/knx-web
unshare --user --map-root-user --net sh -c \
  'ip link set lo up && exec "$@"' sh \
  env KNX_DIAG_EVIDENCE="$(mktemp -d)" KNX_DIAG_GUIDE_OUT=../../docs/assets/screenshots \
  KNX_DIAG_SIM_BIN=../../../knxbench-tag/target/debug/examples/diagnostic_simulator \
  npx playwright test -c playwright.diagnostic-guide.config.ts
```
Remove only your extraction directory after verifying the output and stopping the
owned processes; leave existing application containers alone.

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
- [Group-address presentations spec](../../apps/knx-web/e2e/manual-group-address-views.shots.ts)
- [Diagnostic export and Report problem spec](../../apps/knx-web/e2e/diagnostic-guide.shots.ts)
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
