# KNXBench website: static marketing companion

Status: user-approved implementation and GitHub Pages deployment on 2026-10-08.
knxbench.com serves the release build and approved Story; the main repository
is public. Default local builds remain restricted previews. HTTP→HTTPS and
remote IPv6 HTTPS were verified on 2026-10-09; Pages actions now use Node 24.
Owner privacy review and optional WebKit evidence remain separate follow-ups.
Decision: [ADR-0095](adr/0095-static-marketing-companion.md).

## Agreed brief

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
- Current production frontend at `a642eaf9`: group-address filtering/inspection,
  8.8 s MP4, plus Graphite/CRT/LCARS screenshots. Fictional API answers are fully
  intercepted. Seven capture checks pass with zero escaped/unhandled requests
  or page errors; no live application server or KNX hardware.
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
assistive-technology or hardware test. Live deployment verification follows
publication and is recorded separately in the evidence file.
