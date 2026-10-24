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

Marketing h1/h2/h3 now enter with SessionRadar-style staggered character fades,
vertical movement and a slight 3D turn (650ms per character, up to 24ms stagger,
600ms maximum total stagger). Leaving the inset viewport plays the same timeline
backwards; quick re-entry reverses in place without jumping. Once settled, each
ambient roll still randomly chooses right, left, up or down with an oppositely
offset twin and 720ms timing. Rare single-glyph 180ms glitches remain. Entrance and
ambient motion use separate character layers, preserving layout and readable text.

Only visible headings run ambient effects; a finite reverse entrance may finish
offscreen and then releases its animations. Header Pause/Resume, live OS reduced
motion, hidden-document and pagehide cleanup cancel both layers and restore all
text. The setting is page-local, never persisted. No-JS/missing APIs/animation
failure leave static text; legal/privacy pages stay static. No animation is
needed to read or navigate.

Actual-browser recipes: `tests/verify-headline-reveal.js`,
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

## Alpha 16 release content

`content/de.json` and `content/en.json` hold the release copy; `page.html` holds
the intended `v0.1.0-alpha.16` release and download targets. There is no separate
release-data updater in the maintained site sources: update both locales and
the template together, without changing the normal release workflows.

This refresh is a **prepared pre-release**, not a publication receipt. Local
package acceptance covers amd64; the normal Docker pipeline still targets
amd64 and arm64. The intended GitHub assets follow the Alpha 15 channel:
x86-64 AppImage, `knx-mcp-x86_64-linux` and `SHA256SUMS`. Docker uses the version
tag and `latest` after publication; `latest` includes alphas and does not mean
stable. No standalone-server or local container-archive download is promised.

The smallest offline release-content check is:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s website/tests -p test_build.py -k test_alpha16_release_copy_links_history_and_upgrade_boundaries -v
```

It builds a private static preview and checks localized copy, intended links,
upgrade bounds and retained media labels. It does not fetch GitHub/Docker Hub,
verify public assets/manifests or run a browser. The separate browser recipe
`tests/verify-release-features.js` is updated for this content, not newly run.
After publication, read back release assets/checksums and both Docker platform
manifests/digests before replacing candidate/pending/intended labels in the
README and both site locales. Then verify the deployed website separately.

Alpha 15 remains linked as release history. Alpha 14 media retains its original
release labels and has its own historical section; none is relabelled Alpha 16.

## Media

All retained application media shows **Alpha 14** (`v0.1.0-alpha.14`, commit `8d104e2f`):
the hero, the CRT/LCARS gallery, four feature stills and all three videos. Stills
and the group-details clip come from the real server and production frontend of
the tagged source with the fictional Sample house, in a loopback-only namespace.
The note-decision scene uses a group address created through the real in-range
create action (every sample address is linked, so a delete would rightly be
refused); the vendor-ZIP scene previews the fictional demo package plus a manual
and installs nothing; the diagnostic scene stops at the read preview. Seven PNGs
and the capture manifest (server binary, spec/config and intermediate GIF hashes)
live in `docs/assets/releases/alpha14/`. The public WebPs are lossless and
pixel-equal to those PNGs; `media.json` records every source and output hash.
Earlier Alpha 9/12 capture records stay as history.

Regenerate after building the tagged server (`cargo build -p knx-server`) and
frontend (`npm ci && npm run build` in `apps/knx-web`), under the shared gate
leases (`flock` on the three lock files `tools/run_web_gate.py` names):

```sh
# Stills + group-details.mp4
KNX_SERVER_BIN=<target>/debug/knx-server \
KNX_RELEASE_MEDIA_DIR="$TMPDIR/alpha14-media" \
KNX_RELEASE_MEDIA_ARTIFACT_DIR="$TMPDIR/alpha14-browser-artifacts" \
unshare --user --map-root-user --net sh -c \
  'ip link set lo up && cd apps/knx-web && exec npx playwright test -c playwright.release-media.config.ts'
# project-work clip (GIF; the website MP4 plays it at 2x): same wrapper with
#   README_HERO_GIF=<out.gif> ... -c playwright.readme-hero.config.ts
# bus-flow clip (GIF, fixture server with simulated telegrams): same wrapper with
#   README_FLOW_GIF=<out.gif> ... -c playwright.readme.config.ts
```

The MP4s are H.264/yuv420p, faststart, no audio, cropped to even sizes; posters
are lossless WebPs of a decoded frame. Exact commands and hashes: the manifest.
Current browser recipe for the release section: `tests/verify-release-features.js`.

**Caption rule (owner, 18 October 2026):** captions, alt texts and transcripts
say what is shown (release, theme, fictional project, zoom/speed) and carry no
negative tags such as "no live bus" or "no hardware".
`test_site_copy_and_captions_carry_no_negative_disclaimer_tags` keeps it that way.
Real scope boundaries stay in the status section and the release notes.

Videos do not autoplay or loop. They have native controls, same-host posters,
DE/EN descriptive captions and text walkthroughs. Do not silently swap in an old
clip after a workflow change; regenerate it or scope its caption/provenance.
The font licence file must remain in the artifact. See `media.json` for the exact
source and output hashes. No third-party embeds, external fonts or analytics.
