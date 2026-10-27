# KNXBench marketing companion

A static DE/EN landing page. Product marketing, not a hosted KNX application.
Approved scope: `.ai/logs/2026-10-08_codex_landing-page-grilling.md`.

## Build and preview

```sh
python3 website/build.py
python3 website/serve.py --port 4198
```

Open `http://127.0.0.1:4198/`. The root is the English landing page; `/de/` and
`/en/` are explicit language URLs. The builder needs Python 3.10+ and no package
installation, network or application build. Media is committed and self-hosted.
The server binds loopback only and serves **only** the built directory. Do not
serve the repository root or open this private preview to a network.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s website/tests -v
node --check website/site.js
```

The default is English regardless of browser locale, without redirects or stored
language preferences. Canonical root content points to `/en/`; `/de/` stays German.
Default-language acceptance recipe: `tests/verify-default-language.js`.

## Source and artifact boundary

- `page.html`, `site.css`, `site.js`, `headlines.js`, `content/`: curated site sources.
- `assets/`, `media.json`: explicitly inventoried demo media and font licences.
- `story.json`: pinned existing story candidate; not publication permission.
- `build.py`: deterministic stdlib builder, output inventory/hashes and safe
  no-clobber replacement of its own output. Refuses foreign output files.
- `serve.py`: loopback-only private preview; no domain/core/server/API dependency.
- `tests/verify-browser.js`: Playwright CLI function for the actual built pages.
- `dist/`, `output/`: generated artifacts, ignored; never copy source wholesale
  to a Pages artifact. The output contains `.nojekyll` for plain static hosting.

## Story-style CRT and motion

Original screenshot-first marketing layout, with the approved Story's green
canvas, typography and phosphor-green headings. The added installation diagram
and all its code/labels are removed following owner review. Story source is
unchanged; shared visual principles do not mean loading its application runtime.

Once a heading has typed, each ambient roll randomly chooses right, left, up or down
with an oppositely offset twin and 720ms timing; rare single-glyph 180ms glitches
remain. The earlier SessionRadar-style fade/3D entrance was removed on 27 October 2026.

**Live typing for every heading (27 October 2026).** Every marketing `h1/h2/h3` types its
glyphs one by one (50ms apiece) the first time it becomes visible in a page load, with a
zero-width cursor inline after the last revealed glyph, on whatever line that is. Untyped
glyphs only turn `visibility: hidden`, so the heading keeps its final layout and wrapping.
The cursor blinks four times after the last glyph, then is removed; from then on the
heading takes part in the ambient random-direction rolls and rare glitches again. The
earlier fade/3D entrance and the selective `data-headline-typing` marker are gone. Pause,
live reduced motion, hidden document and pagehide show all text at once and remove the
cursor. Pause and reduced motion settle every heading for the page load, unseen ones
included; a hidden document or pagehide finishes only headings already typing, and unseen
ones still type once the page is visible again. Typing never replays, not on scroll, not
on resume. Without JS a headline is plain static text.

Only visible, settled headings run ambient rolls. Header Pause/Resume and live OS reduced
motion cancel typing and rolls and restore all text; hidden-document and pagehide cleanup
cancel rolls and finish typing already in progress. The setting is page-local, never persisted. No-JS/missing APIs/animation
failure leave static text; legal/privacy pages stay static. No animation is
needed to read or navigate.

Actual-browser recipes: `tests/verify-headline-typing.js`,
`tests/verify-headlines.js`, `tests/verify-phosphor.js`,
`tests/verify-roll-directions.js`. The direction recipe uses controlled RNG values
against real WAAPI animations, not guessed randomness or a fake animation clock.
Owner commit/push approval was received on15October2026; this revision is now
published at knxbench.com with successful Pages and live byte/browser readback.
Default builds remain private previews. See `docs/WEBSITE.md` and its publication
receipt for exact source-bound verification; the approved Story stays unchanged.

## Release build and deployment

```sh
python3 website/build.py --release --output /tmp/knxbench-site
```

The release build drops the preview banner, launch note and `noindex`, opens
`robots.txt`, writes `CNAME` (`knxbench.com`) and renders the story's
published variant, but only when `story/approvals/<edition>.json` matches the
pinned edition exactly; otherwise it refuses and writes nothing.
`.github/workflows/pages.yml` builds and deploys it to GitHub Pages on pushes
to `main` touching `website/` or `story/`. Contract: `docs/WEBSITE.md`. Node-24 actions: checkout v7, configure-pages v6,
upload-pages-artifact v5 (with hidden files included for `.nojekyll`) and
deploy-pages v5. Only the inventoried site is uploaded.

The DE/EN landing pages link the unchanged `demos/1.0.0` ZIPs and checksums
through GitHub, plus the setup guide. These are fictional offline projects,
not a hosted application, ETS export or hardware download. Browser handoff
recipe: `tests/verify-demos.js` (start it from the desired preview/release URL).

The contact pages are labelled Impressum / Legal notice and contain the owner-supplied
name, postal address and contact email. In the release build the privacy pages name the host (GitHub Pages) and its
documented IP logging; the preview keeps the pending note. This is **not** a legal-compliance certification; no legal
identity, phone, tax number or retention policy was invented.
Imprint acceptance recipe: `tests/verify-imprint.js`.

## Highlights content

The landing page lists product highlights, not per-release changes (owner request,
27 October 2026). `#features` holds nine numbered highlights: six with a real-app still
(import, history/undo, wizards, parameters, catalogue, search) and three text cards (bus
and flow view, documents/comparison, AI agents over MCP). Every claim maps to an
implemented row in `docs/FEATURES.md` or the README highlights; anything proven on a single
device or only in simulation stays out. The alpha notice stays in the hero tag and the
status section. Release numbers, upgrade chains and download URLs of a specific release
do not appear in the page; the start section links the releases page. A release therefore
no longer needs a website copy change; refreshing the stills is optional.

`test_highlights_replace_release_specific_sections` and `tests/verify-highlights.js`
guard the structure, the stills and the absence of release numbers.

## Media

The hero, the CRT/LCARS gallery, the six highlight stills and the group-details clip
show **`v0.1.0-alpha.17`** (commit `a9ea32d7`): real server and production frontend of the
tagged source, fictional Sample house, loopback-only namespace. The add-device wizard is
cancelled at its review (nothing created); the vendor-ZIP scene stops at the catalogue
job's bound preview (nothing installed); the sample is saved as `.knxdb` in the scratch
data directory so the history scene can create a named version. Nine PNGs and the capture
manifest live in `docs/assets/releases/alpha17/website/`; the public WebPs are lossless
and pixel-equal to those PNGs. `project-work.mp4` and `bus-flow.mp4` remain Alpha 14
recordings (`alpha14Capture`). `media.json` records every source and output hash;
earlier Alpha 9/12/14 capture records stay as history.

Regenerate after building the tagged server (`cargo build -p knx-server`) and
frontend (`npm ci && npm run build` in `apps/knx-web`), under the shared gate
leases (`flock` on the three lock files `tools/run_web_gate.py` names):

```sh
# Stills + group-details.mp4
KNX_SERVER_BIN=<target>/debug/knx-server \
KNX_RELEASE_MEDIA_DIR="$TMPDIR/website-media" \
KNX_RELEASE_MEDIA_ARTIFACT_DIR="$TMPDIR/website-browser-artifacts" \
unshare --user --map-root-user --net sh -c \
  'ip link set lo up && cd apps/knx-web && exec npx playwright test -c playwright.release-media.config.ts'
# project-work clip (GIF; the website MP4 plays it at 2x): same wrapper with
#   README_HERO_GIF=<out.gif> ... -c playwright.readme-hero.config.ts
# bus-flow clip (GIF, fixture server with simulated telegrams): same wrapper with
#   README_FLOW_GIF=<out.gif> ... -c playwright.readme.config.ts
```

The MP4s are H.264/yuv420p, faststart, no audio, cropped to even sizes; posters
are lossless WebPs of a decoded frame. Exact commands and hashes: the manifest.
Browser recipe for the highlights section: `tests/verify-highlights.js`.

**Caption rule (owner, 18 October 2026):** captions, alt texts and transcripts
say what is shown (theme, fictional project, zoom/speed) and carry no
negative tags such as "no live bus" or "no hardware".
`test_site_copy_and_captions_carry_no_negative_disclaimer_tags` keeps it that way.
Real scope boundaries stay in the status section and the release notes.

Videos do not autoplay or loop. They have native controls, same-host posters,
DE/EN descriptive captions and text walkthroughs. Do not silently swap in an old
clip after a workflow change; regenerate it or scope its caption/provenance.
The font licence file must remain in the artifact. See `media.json` for the exact
source and output hashes. No third-party embeds, external fonts or analytics.
