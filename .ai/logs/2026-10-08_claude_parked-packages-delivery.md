# 2026-10-08 — Claude: parked root packages delivered (website, community evidence) + link repair

User request: deliver the two packages parked during the root sync, with a
parallel docs-refresh job running (worktree `docs-refresh-20261008`, not touched).

## 1. Pinned-link repair (`767c3770`)

Found while reviewing: the public-launch purge rehashed the commits that
`REMOVED_DOCS` pins, so 242 GitHub links to `138403ed6084`/`aa0ff14ff536`
answered 404. Moved to `bca2d3336b96`/`a584007fc05a` in 32 maintained files
(story editions and handovers untouched); 61/61 real targets checked with
`git cat-file -e` at the new commits, samples 200 on GitHub. Doc gates 5/5.

## 2. Website (`7c0f73b5`)

From `wip/website-20261008`: `website/`, `docs/WEBSITE.md`, ADR renumbered
0094 → 0095. Changes: launch note (DE/EN) now says repository/downloads are
public and the site is a preview; story pin `2026-10-05.1` (removed by the
story track) → `2026-10-08.4` (approval record exists; site stays preview);
browser recipe compares the story's chapter count with its own data; archived
MANUAL_ACCEPTANCE link pinned; five per-step receipts dropped. Imprint name
and postal address committed on the owner's explicit decision (documented in
WEBSITE.md/KL). Gates: 17 unit tests, deterministic rebuild (same hash twice),
`--release` refuses (exit 3), four Chromium recipes 62+26+48+18 = 154 checks,
0 unexpected requests/errors, run offline in `unshare -rn` (Codex' preview on
:4198 left alone); launch-note screenshot inspected; doc gates 5/5 before and
after rebase. Launch gate 2 (public entry points) verified signed out.

## 3. Community evidence (`b7059b38`)

From `wip/community-evidence-20261008`, only the contribution package (new
knx-app/CLI/server/web files, 3-way-merged edits to Cargo.toml, lib.rs, CLI
main, server lib, App.tsx/test, en/de messages; ADR-0091, contract, research,
guides). Left out: LCARS/parameter files (on main or archived), older doc
drafts, receipts. ADR-0091 consolidation: form → `.github/ISSUE_TEMPLATE/
analysis.yml` (blank issues enabled), guides in `docs/contribution-intake/`,
app/guides/form/website link the main repo. Interim repo had 0 issues.
Review (in-session): routes behind `require_session`, body limit, one worker;
input/expansion/member/depth bounds; `persist_noclobber`; no network/process
calls; key-material and consent tests present. Gate (fresh target, workspace
lock): clippy -D warnings, fmt, Rust 3,721/0/182 (224 blocks), Vitest
2,415/0 (149 files), web build, both fixture tsc, Playwright 175 passed,
xtask 5/5. `inputs_frozen=0` explained: Cargo added the `quick-xml` lock line
during the run (committed). After rebase only docs/website changed outside the
gated code (diff-proven); website re-gated (17 unit, 154 browser), doc gates
5/5. Form: raw file differs from the proven interim form only in the guide-link
line; `config.yml` contact link parsed by GitHub (GraphQL). Signed-in form
rendering not viewed.

## Not done

Interim `KNXBench-Contributions` retirement (owner decision); website
deployment gates (go/release mode, host privacy, Pages/DNS/HTTPS); story
commit-link remap; mailbox verification.
