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

- `page.html`, `atlas.html`, `site.css`, `site.js`, `atlas.js`, `headlines.js`, `content/`: curated site sources.
- `assets/`, `media.json`: explicitly inventoried demo media and font licences.
- `story.json`: pinned existing story candidate; not publication permission.
- `build.py`: deterministic stdlib builder, output inventory/hashes and safe
  no-clobber replacement of its own output. Refuses foreign output files.
- `serve.py`: loopback-only private preview; no domain/core/server/API dependency.
- `tests/verify-browser.js`: Playwright CLI function for the actual built pages.
- `dist/`, `output/`: generated artifacts, ignored; never copy source wholesale
  to a Pages artifact. The output contains `.nojekyll` for plain static hosting.

## Phosphor Atlas and motion

The dark-green canvas leads with a fictional installation diagram: solid green
line membership and dashed violet logical group links, never actual wiring or
live traffic. Keyboard/touch selection opens native explanations; without JS the
fragment links and disclosures still work. Mobile has a vertical diagram.

Marketing h1/h2/h3 keep the Story's ambient rolling-glyph treatment, plus rare
180ms single-glyph glitches. One finite path/node reveal introduces the diagram.
Only visible headings animate; OS reduced motion and header Pause/Resume cancel
all effects live, including button transitions. Resuming never replays the diagram.
The setting is page-local and not persisted. No JS/missing APIs retain readable
static text; legal/privacy pages never load the effect. Story sources unchanged.
`tests/verify-headlines.js`, `tests/verify-phosphor.js` and
`tests/verify-phosphor-visual.js` are actual-browser recipes. The redesign remains
a private preview pending separate owner publication approval; Story unchanged.

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

## Media

The hero, CRT/LCARS gallery and four feature stills show the **Alpha 12
candidate**, built from the integrated source, with the real server and fictional
Sample house in a loopback-only namespace. The instruction frame uses a device
created through the real wizard: the old schema-11 sample deliberately has no
supported imported instruction source. No API successes or UI controls were
invented. Ten PNGs (including a separate offline-preview evidence frame) and
source/spec/server hashes live in `docs/assets/releases/alpha12/`.
The seven public WebPs are lossless and pixel-equal to those PNGs; `media.json`
records each source and output hash. The previous Alpha 9 capture record stays
historical, not newly certified.

Regenerate the candidate media after building the real server/frontend:

```sh
KNX_SERVER_BIN="$PWD/target/debug/knx-server" \
KNX_RELEASE_MEDIA_DIR="$TMPDIR/alpha12-media" \
KNX_RELEASE_MEDIA_ARTIFACT_DIR="$TMPDIR/alpha12-browser-artifacts" \
unshare --user --map-root-user --net sh -c \
  'ip link set lo up && cd apps/knx-web && exec npx playwright test -c playwright.release-media.config.ts'
```

Use the repository's shared gate leases before running alongside another session.
The capture spec, dedicated TypeScript config and original PNGs are committed;
failed capture attempts are retained privately, not counted as accepted runs.
Current browser recipe: `tests/verify-release-features.js`.

Videos remain **earlier recordings**, clearly labelled in both languages.
`project-work.mp4` and `bus-flow.mp4` are first-loop transcodes of existing README
recordings, not current-version or performance measurements. The former includes
camera zoom/timing and a half-duration transcode (28.2 s); the latter is the real
monitor component with synthetic traffic. Historical `group-details.mp4` uses a
fully intercepted fictional API fixture. Its checks remain under
`historicalRecording` in `media.json`. Alpha 12 copy names native16/product23,
independent backups, old-reader refusal and actual remaining programming limits.

Videos do not autoplay or loop. They have native controls, same-host posters,
DE/EN descriptive captions and text walkthroughs. Do not silently swap in an old
clip after a workflow change; regenerate or clearly scope its caption/provenance.
The font licence file must remain in the artifact. See `media.json` for the exact
source and output hashes. No third-party embeds, external fonts or analytics.
