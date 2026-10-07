# 2026-10-07 — Claude — first-run guide (ADR-0084)

## Request
User (grill-me interview, German): the app needs a first-run guide that says
what state the app is in (alpha) and names the most important options/tasks.

## Decisions from the interview
Q1 both honesty and quick start, honesty first · Q2 guide, not wizard (only a
language switch) · Q3 once per release stage, reopen manually · Q4 stepped
dialog on Overlay · Q5 non-blocking, no acknowledgement · Q6 four steps
(what/stage, works/not yet, where to start, help) · Q7 hand-written coarse
texts + fix the stale limits help · Q8 any close = seen; unnamed stage never
auto-opens · Q9 settings.json (per installation) · Q10 fix the server version
· Q11 auto-open only: main window, after auth, acknowledged settings, known
stage, no dialog, no project · Q12 buttons run COMMANDS · Q13 en/de toggle ·
Q14 manual shots close the guide + one guide shot · Q15 three commits.

## Deviation (disclosed to the user)
Q10 asked for an xtask check that all program versions agree. ADR-0018 §1
says programs version independently, so that check would contradict an
accepted ADR. Did the ADR-0018 bump-rule catch-up instead: knx-server
0.1.0-alpha.1 → 0.1.0-alpha.2. knx-web/knx-desktop were not bumped (the
release that ships the guide bumps them together; check-appimage ties them).

## Findings while building
- Help `limits.p2` was stale (denied ETS4/5 password dialog and device
  download) — fixed in its own commit.
- `porcelain-new-project.png` was stale before this package (hint text
  changed); retaken.
- Own screenshot showed two faults: pressed language button invisible
  (accent on accent gradient) and the card stretched to max-height because
  `.search-overlay` is a row flexbox without align-items. Fixed with
  `align-self: flex-start` and a neutral/filled toggle; e2e guards added and
  proven by negative controls (control A: background-image ≠ none; control
  B: height 590 vs cap 590).
- A colour guard comparing `color` with `backgroundColor` alone passed on the
  broken CSS (gradient over transparent colour) — strengthened to require a
  solid fill first.

## Commits
- `10928f9b` fix(server): knx-server 0.1.0-alpha.2 (ADR-0018 catch-up)
- `7b61d6f8` fix(help): limits topic no longer denies the ETS4/5 password dialog and device download
- `fdb625ba` feat(web): first-run guide (ADR-0084, KL §160, KL-160)

## Gate (on 7b61d6f8 + uncommitted candidate, inputs_frozen=1)
Vitest 2134/2134 (120 files) · Chromium 146 passed offline · onboarding spec
×3 12/12 · knx-server tests 623/0/45 ignored · clippy -p knx-server -D
warnings clean · `knx-server --version` = 0.1.0-alpha.2+g7b61d6f8 · xtask
layering/anchors/ledger/corpus-gates ok; check-headers failed once (101-col
test header) → comment-only fix, check-headers rerun ok (567/155 ceiling).
Delta between gated tree and fdb625ba: that header line and the
IMPLEMENTATION_STATUS entry (anchors rerun ok, diff --check ok).
Manual screenshots: 2/2 in `unshare -rn` (ifaces = lo), release server built
in a fresh target; changed images compared with `compare -metric AE`.

## Not done / open
- WebKitGTK (desktop shell) and screen readers not verified.
- knx-web/knx-desktop stay 0.1.0-alpha.4 until the next release bumps both.
- F1 / Ctrl+Shift+P while the guide is open stack Help/palette over it (same
  as the About dialog); accepted, not changed.
