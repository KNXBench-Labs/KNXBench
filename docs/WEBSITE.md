# KNXBench website: local marketing companion

Status: user-approved implementation, in the repository since 2026-10-08. The
site itself is not deployed yet; the main repository is public.
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

## Before deploying the site

1. Separate go for publication and a reviewed release-mode/deployment design.
2. Done 2026-10-08: the main repository is public; repository, manual, Docker
   guide, releases page, the alpha.5 AppImage and the contributions repository
   answered signed out.
3. Story: on delivery the pin moved from the removed candidate `2026-10-05.1`
   to the current `2026-10-08.4` (digest `347dfcfb…e0a2`), for which the story
   track holds an approval record. Release mode must check that record; its
   commit links still cite pre-purge hashes (KL §162).
4. Review owner-supplied publisher/contact information and finalize host-specific
   privacy information. Do not invent server-log/retention policies or claim
   legal compliance. Public hosting and privacy details remain unfinished.
5. Verify domain ownership, DNS, Pages artifact boundary and HTTPS; remove
   preview notices/noindex only in the authorized release build and rerun checks.

`build.py --release` currently refuses without writing. There is no Pages
workflow, visibility change, public deployment, Docker registry publication or
application-release change in this package.
