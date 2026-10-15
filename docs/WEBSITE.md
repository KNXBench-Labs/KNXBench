# KNXBench website: static marketing companion

Status: user-approved implementation and GitHub Pages deployment on 2026-10-08.
knxbench.com serves the release build and approved Story; the main repository
is public. Default local builds remain restricted previews. HTTP→HTTPS and
remote IPv6 HTTPS were verified on 2026-10-09; Pages actions now use Node 24.
Owner privacy review and optional WebKit evidence remain separate follow-ups.
Decision: [ADR-0095](adr/0095-static-marketing-companion.md).

<a id="phosphor-atlas--private-implementation-15-october-2026"></a>

## Story-style phosphor CRT — private revision (15 October 2026)

Owner review superseded the initial diagram-first Phosphor Atlas concept: **remove
its installation diagram and restore the original marketing layout**, retaining
phosphor CRT character and applying the existing Story's visual language. The owner authorized commit/push of this reviewed revision on15October2026.
The source is being integrated and checked for Pages publication; public delivery
will be recorded only after workflow and live-byte readback. Alpha12 release and
freeze release were verified before implementation.

The screenshot-first hero, section order and responsive grids follow the original
website. The added installation fragment, script, logical-path animation,
translations and diagram-specific CSS/tests are removed, not hidden. The original
small Evolution Story teaser remains part of the original page. No new card kit,
boot sequence or decorative content replaces the removed hero.

The whole canvas uses the Story's green-black `#03150c`, surfaces `#0b2d1c` and
`#0f3824`, reading ink `#eef5e9`, phosphor `#88ffad`, violet `#c1aff2`, amber
`#e7c688` and rule `#316343`. Font stacks now match the Story's Liberation
Sans/Arial and metadata monospace. **All marketing and legal/privacy h1/h2/h3
headings are phosphor green** with controlled Story-style glow. Prose and original
screenshots remain crisp, unfiltered and unchanged. Existing self-hosted font
assets/licences stay inventoried but are not required to mimic the Story's system
font stack. No copied proprietary assets, backend or new dependency.

### Random-direction characters, one motion owner

The existing `headlines.js` owner retains its 720ms rolling-glyph timing and
selects **right, left, up or down independently per roll** with one uniform
`Math.random()` direction choice. The outgoing glyph and its incoming twin move
in opposite offsets on the chosen axis. Vertical rolls clip within their slot;
no reflow, word changes or motion-dependent content. Rare isolated 180ms glitches
remain; no constant flicker or blurred reading text.

The header pause/resume applies to rolls, glitches and button transitions. Live
OS reduced motion always wins; offscreen/hidden/pagehide lifecycle stops ambient
effects. No storage, cookies or new network calls. Without JavaScript or motion
APIs the original readable headings/screenshot remain and the nonfunctional
motion switch stays hidden. Legal/privacy pages remain static; legal text,
approved Story data/style/runtime/pin and application/media are unchanged.

### Acceptance and limits

`website/tests/test_phosphor.py` checks actual atlas removal and the retained
palette/header control. `verify-phosphor.js` covers the restored hierarchy,
computed green headings, responsive DE/EN320–1440, decoded screenshots, crisp
prose, static/no-JS fallback and live CSS cancellation.
`verify-roll-directions.js` exercises all four real WAAPI branches with controlled
random inputs, opposite twins, stable bounds, sampled displaced glyphs and live
keyboard/OS cancellation. Controlled RNG is explicit test instrumentation, not
claimed natural-random distribution evidence. Existing whole-page, headline,
language, imprint and demo recipes remain mandatory.

Exact observed results live in the [revision evidence](evidence/phosphor-crt-revision-2026-10-15.json): website28, Story76, tooling163 (1 existing skip),23 frozen-source stages, Chromium394 preview and416 public-variant checks. Every recipe is admitted against the selected origin and artifact mode; a named regression prevents the legacy fixed-preview-port escape. Both builds are deterministic. Prior diagram
acceptance remains historical, not acceptance of this replacement. No
Firefox/WebKit/Orca/native-screen-reader, hardware or electrical-plan certification.
Self-review is not independent approval. Preview/public-variant builds do not deploy.

## Alpha 12 refresh (15 October 2026)

The existing dark DE/EN design, root language, editorial voice, legal/privacy
copy, no-tracking policy and exact approved Story are unchanged. Hero/theme
stills and four feature cards now use the Alpha 12 candidate's real server and
fictional data: programming marks, in-range creation, native instructions and
universal offline support. Ten source PNGs and seven pixel-equal lossless WebPs
have explicit provenance. Earlier videos stay visibly historical. Current copy
states native16/product23 and independent backups, old-reader refusal and the
unchanged System-2/little-endian/Secure boundaries. Actual website acceptance and
public deployment remain separate from media capture.

## Original brief (8 October; visual direction superseded above)

- Encourage visitors to try the actual KNXBench application, not a hosted demo.
- Product benefits first; Linux is the origin and a native deployment path, not
  the central USP. Docker/browser is the prominent cross-platform entry.
- Calm dark design, existing logo, real app screenshots and demo-project clips.
- CRT/LCARS, language packs and achievements add secondary personality.
- Positive/respectful toward KNX and ETS; no competitor jokes/disparagement.
- Full DE/EN landing pages; English at `/` by owner correction; explicit
  language URLs (`/de/`, `/en/`), no tracking-based detection or locale redirects.
- Existing Evolution Story linked in navigation and a teaser, reused locally.
- One start section with Docker and AppImage paths; full manual remains on GitHub.
- No analytics, marketing cookies, third-party embeds or remote font requests.

## Default language update (2026-10-08)

Owner chose English as the website default. Root is rendered from English content
with root-relative asset paths, English captions and legal links, current EN
switch and canonical `/en/`. German remains explicitly available at `/de/`;
`/en/` remains English. No app-language change, automatic redirect or persistence.
New unit tracer observed RED before the mapping change, then all 17 tests pass.
Fresh Chromium acceptance: 62 landing, 26 imprint, 48 motion and 18 default-language
checks (154 total), zero errors/unexpected requests. No-JS German browser locale
still receives English root.

## Source boundaries

`website/` contains static page/template/content, CSS/minimal JS, stdlib build and
loopback preview scripts, explicit media inventory, tests and retained proof.
It does not import the KNX core, invoke the application server, open user project
storage or expose bus operations. The source tree is never the deployment artifact.

`website/build.py` emits root/DE/EN pages, small preview contact/privacy pages,
self-hosted assets, `.nojekyll`, robots/noindex preview restrictions and the
existing pinned story into an inventoried directory. Repeated builds are byte
identical for the same sources. It refuses source/output symlinks, duplicate
JSON fields, foreign output, unlisted files/directories and edited generated
files. Same-filesystem staging retains old owned output if replacement fails.

Default output is `website/dist/`, ignored by Git. Preview binds loopback only.
See [build and media recipe](../website/README.md).

## Media and evidence

- Existing `docs/assets/readme/hero-add-device.gif`: real app/sample-house
  workflow, edited camera/timing. The MP4 additionally halves duration to 28.2 s;
  this is expressly not a performance measurement or a recording of the newer
  device wizard. The catalog/link workflow remains the footage's scope.
- Existing `telegram-flow.gif`: real monitor component, synthetic traffic,
  optional CRT theme, no live bus; MP4 duration 11.7 s.
- Historical production frontend at `a642eaf9`: group-address
  filtering/inspection in the retained 8.8 s MP4. Fictional API answers were fully
  intercepted; its seven original capture checks remain historical evidence.
  The Graphite/CRT/LCARS stills have since been replaced by Alpha 9 captures.
- **Alpha 9 stills (11 October 2026):** hero, optional-theme gallery and four
  feature cards use the published release frontend/server and fictional project
  in a loopback-only namespace. Lossless WebPs retain the manual PNG pixels.
  Devices, named DPT choices, native history and notification details are shown;
  existing features are not all claimed as newly introduced in this release.
  Older videos are visibly labelled in DE/EN. Schema 13, independent backups,
  Alpha 8 refusal and unmounted editor foundations stay explicit in the copy.
  [Capture provenance](assets/README.md) · [Acceptance](evidence/alpha9-media-2026-10-11.json).
- Files, source provenance, stream probes and hashes: `website/media.json`.
  Videos are H.264/yuv420p, no audio, faststart, native controls, no autoplay/loop,
  same-host posters, DE/EN descriptive caption tracks and text walkthroughs.
- Font binaries are retained with the unmodified Inter and Space Grotesk OFL
  notices. The project's AGPL licence text is included as a separate asset.

Build/browser acceptance comes from the recipes in `website/tests/`, not from
footage. Target checks include 14 stdlib tests, deterministic rebuild,
actual Chromium desktop/mobile/DE/EN/no-JS navigation and all three videos
actually decoding frames with loaded caption cues. Broader app, native runtime,
screen-reader, every-browser or live KNX claims do not follow.

## Contact update (2026-10-08)

Owner supplied `contact@knxbench.com`. Both contact pages now link that exact
address via mailto; no test message was sent and mailbox delivery is not claimed.
The previous missing-email placeholder is removed; legal publisher and hosting
privacy details remain open. Fifteen build tests pass, including the new
RED/GREEN contact regression; live loopback DE/EN pages read back with the link.

## Legal notice update (2026-10-08)

Owner supplied name and postal address; both existing `/de/contact/` and
`/en/contact/` pages now contain the legal notice and exact supplied name/address
plus contact email. Root/DE/EN footer links are labelled Impressum / Legal notice.
Missing-provider placeholders were removed; privacy hosting details remain
preview-only. No identity, country, registration, tax number or phone was invented.
Sixteen stdlib tests pass (new imprint test observed RED then GREEN); fresh
Chromium acceptance: 62 whole-landing and 26 imprint checks, zero errors or
unexpected requests, keyboard/footer/no-JS/320–1440px checks and visual inspection.
This is implementation
evidence, not a legal compliance certification or permission to deploy.

## Story-style rolling headlines (2026-10-08)

Owner requested the same rolling-character treatment as the Evolution Story.
`website/headlines.js` is a scoped adaptation of `story/site/app.js:392–468`
and the four character CSS rules in `story/site/style.css:226–231`. Story source,
candidate content and prepared previews remain untouched: loading the whole
story runtime would require its graph/data controls and violate the companion
boundary. This is not a new story edition or publication approval.

Marketing main h1/h2/h3 only: one/two random character swaps per tick, same
720ms cubic-bezier(.65,0,.35,1), right-exiting glyph/left-entering pseudo-element,
initial 1.2s delay then 2.2–3.8s intervals. Words/text stay unchanged; explicit
line breaks and normalized accessible heading names are retained, with decorative
letter spans hidden from assistive technology. Only visible headings animate.
Live OS reduction, manual pause and hidden/pagehide lifecycle cancel this
module's active effects; no background/offscreen schedule. Resume is supported.
Footer pause/resume control is keyboard-accessible, has aria-pressed and DE/EN
labels, works per page and uses no storage/cookies. No JS or missing motion APIs
leaves static readable content and hides the nonfunctional switch. Legal/privacy
pages are intentionally static. No extra font/framework or network request.

Sixteen stdlib tests pass (tracking guard covers both scripts); fresh root
Chromium acceptance: 62 landing, 26 imprint and 48 heading checks. Actual WAAPI duration/pseudo-element keyframes, paused mid-roll raster,
no heading-box movement, live cancellation, keyboard resume, no-JS/API fallback,
DE/EN 320–1440px and zero console/external errors verified. Visibility state
was explicitly simulated; not proof of real browser suspension. Native screen
readers and other browser engines remain unverified.

## Repository delivery (2026-10-08)

The sources moved from a local working copy into the repository. The per-step
receipt files (`verification.json`, `contact-`, `imprint-`, `headline-`,
`default-language-verification.json`) were dropped: they bound hashes of
earlier local candidates and the docs policy keeps no study receipts in the
tree. The landing pages' launch note now says that repository and downloads
are public and that this site is still a preview. Acceptance on delivery is in
[IMPLEMENTATION_STATUS](IMPLEMENTATION_STATUS.md).

The legal notice's provider name and postal address are committed on purpose
(owner decision 2026-10-08): the deployed site must show them anyway. An
identity scan of the repository finds them in `website/content/*.json` and in
two website tests; that is expected, not a leak.

## Release and deployment (2026-10-08)

Owner go on 2026-10-08. `python3 website/build.py --release --output <dir>`
builds the public variant into an inventoried directory:

- no preview banner, no launch note, no `noindex`; `robots.txt` allows
  crawling; a `CNAME` file names `knxbench.com`;
- privacy pages titled Datenschutz/Privacy state the host (GitHub Pages) and,
  citing GitHub's own documentation, that GitHub logs and stores visitor IP
  addresses for security purposes, with a link to the GitHub Privacy
  Statement. Drafted from that source only; owner review, no legal
  certification, no invented retention policy;
- the story is rendered as the published variant only when
  `story/approvals/<edition>.json` matches the pinned edition exactly
  (`storytool build --approval`); otherwise the release build refuses and
  writes nothing.

`.github/workflows/pages.yml` runs on pushes to `main` that touch `website/`,
`story/` or the workflow (and on manual dispatch): website and story tests,
the release build, then `actions/upload-pages-artifact` and
`actions/deploy-pages` with only the built directory. Pages source is
"GitHub Actions"; the custom domain `knxbench.com` is set in the repository's
Pages settings, `www.knxbench.com` redirects to it.

DNS at the registrar (Host Europe, nameservers `domaincontrol.com`): apex A
`185.199.108–111.153`, AAAA `2606:50c0:8000–8003::153`, `www` CNAME
`knxbench-labs.github.io`, plus the account's
`_github-pages-challenge-KNXBench-Labs` TXT verification record (values from
GitHub's custom-domain documentation; checked at both authoritative servers).

Preview builds are unchanged: banner, `noindex`, `Disallow: /`, no `CNAME`.
Acceptance recipes: the four preview recipes plus `tests/verify-release.js`.

## Node-24 Pages actions, IPv6 and community demo links (2026-10-09)

Owner approved implementation and commit/push/live verification on 2026-10-09.
The workflow uses `actions/checkout@v7`, `actions/configure-pages@v6`,
`actions/upload-pages-artifact@v5` and the existing `actions/deploy-pages@v5`.
Official `action.yml` metadata was read at each resolved commit: checkout,
configure and deploy declare `node24`; the composite Pages upload pins
`actions/upload-artifact` v7, which declares `node24`. No force-runtime switch
is used. Upload v5 excludes dotfiles by default; `include-hidden-files: true`
keeps the inventoried `.nojekyll`, without uploading the source checkout.
[Runtime commits and handoff evidence](evidence/pages-demos-2026-10-09.json).

The root, `/de/` and `/en/` now include a Demos navigation entry and three
project choices: home (32 devices / 105 group addresses), residential
(101 / 339), office (157 / 507). Direct GitHub links download the frozen
individual or combined 1.0.0 ZIPs; setup/exercises and SHA-256 checksums are
linked alongside them. All five anonymous download responses were 200 and
byte-identical to the repository files. No package contents or checksums were
changed, and no application-release/older-AppImage compatibility is claimed.
The pages explain English guides, catalogue installation, untouched reset
copies and auto-save, the retained candidate labels and the prohibition on
loading fictional applications into real devices. No automatically loaded
third-party media or new application/backend functionality.

Local host: no public IPv6 default route, so direct `curl -6` cannot connect;
this is not a site failure. DNS resolves all four GitHub Pages AAAA addresses.
A bounded, unauthenticated Globalping HTTPS GET measurement explicitly set
`ipVersion: 6`: Falkenstein (DE), Amsterdam (NL), Spokane (US) and London (GB)
all returned **200**, with authorized TLS, on two distinct resolved Pages IPv6
addresses. Measurement `2iJor2k3SmrFCDtiN00021HcZ`; this is four geographic
observations, not a test of every advertised address or every client network.
Direct IPv4 HTTP still returns **301** to `https://knxbench.com/`.

Local acceptance: website unittest **22**, story unittest **67**; deterministic
preview/release builds; all five fresh-target repository gates and whitespace.
Chromium preview recipes: landing **62**, imprint **26**, headlines **48**,
default-language **18**, demo handoff **134**; release recipes: public variant
**22**, demo handoff **134**. Download links, 320–1440px layouts, DE/EN/root,
keyboard/mobile/no-JS navigation, static safety/setup text and no automatic
third-party requests checked. EN/1440 and DE/390 demo choices visually inspected.
The first browser launch was refused for a too-long socket path; a subsequent
mobile regression exposed an inherited grid rule squeezing the links, fixed
with a scoped selector before the final full pass. These are retained failure
facts, not counted as acceptance. Self-review only; no new app, ETS, native,
assistive-technology or hardware test. Live deployment verification is recorded separately in the evidence file.

Published source `77ab6e97`; GitHub Pages run `37890928052` built and deployed
successfully with the exact action commits verified above. All **34** live
files (including `.nojekyll`) plus the build manifest are byte-identical to
the accepted release build. Live Chromium adds **134** named demo-handoff
checks, with no page/console error or automatic third-party request. A second
IPv6 HTTPS GET measurement (`2GyiZtgGE1dNfEEeH00021Hct`, DE/NL/US/GB) after
deployment returns 200/authorized TLS and observes the new Demos navigation
in all four bodies. The probe service truncates bodies at 10,000 characters;
this is not full-file verification over IPv6.

The first push was refused after concurrent Docker delivery; the package
was rebased with exact upstream handover/status preservation. All website
and workflow source hashes remained unchanged, and the affected repository,
documentation and website tests passed again. `actions/deploy-pages@v5` still
emits its upstream DEP0040 `punycode` deprecation warning, but no Node-20
runtime warning was observed; deployment succeeds. No workaround in project
code. Privacy review and optional WebKit remain open, not part of this task.

## Checkout history for media provenance

Pages checks the recorded source commit of each current screenshot. Its checkout
therefore fetches full Git history (`fetch-depth: 0`); a depth-one checkout can
contain every image but still lack the commit needed to verify its origin. The
provenance assertion stays mandatory. This changes deployment input availability,
not the approved Story, generated page content or release application sources.
