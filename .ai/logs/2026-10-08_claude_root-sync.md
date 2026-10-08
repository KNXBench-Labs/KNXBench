# 2026-10-08 — Claude: root checkout synchronized after the public launch

User go ("go") after the question whether the local state needs updating.

- Before: root `main` at `9fd91108` (tree-identical successor of `e99a94e9`),
  62 behind `origin/main`; 36 tracked modifications and 95 untracked files,
  mostly local, never committed Codex work.
- Backup (local, never publish): `/mnt/daten-i/Sourcecode/KNXBench.backups/2026-10-08-root-sync/`
  (`tracked.patch`, reverse-apply checked; `untracked.tgz` with all 95 files;
  `status.txt`; root handover copy; SHA256SUMS).
- Parked on local branches (base `9fd91108`, not pushed, not reviewed, not
  gated), built through a temporary index so the root index never changed;
  127 of 127 parked files hash-equal to the working tree:
  - `wip/website-20261008` (`482e1cc3`, 44 files): `website/`,
    `docs/WEBSITE.md`, `docs/adr/0094-static-marketing-companion.md`.
    ADR number 0094 is taken on main (Legacy VD L3): renumber on delivery.
  - `wip/community-evidence-20261008` (`daeb6015`, 83 files): community
    evidence package (knx-app/CLI/server/web, ADR-0091, intake docs and
    research), the parameter-presentation and LCARS files, older local drafts
    of already published work, and the root handover with 39 entries that are
    not verbatim on main.
- Not parked (only in the tarball): four Playwright CLI scratch snapshots.
- Status compared byte-for-byte with the backup immediately before the reset;
  exact tracked paths restored, exact untracked list removed (no `git clean`),
  then `git merge --ff-only` to the pinned `ea720cc3`. Root clean.
- `.git/info/exclude` (local only) now ignores `website/dist/` and
  `website/output/`, so Codex' preview on :4198 keeps serving (HTTP 200
  after the sync). `OriginalData/` present and ignored.
- `.serena/` became untracked after upstream `203b601b` removed its ignore
  rule while the local directory stayed; added `.serena/` to `.gitignore`.
