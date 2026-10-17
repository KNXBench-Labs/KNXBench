# P5 — OPEN_WORK synchronisation (docs only)

Planning only; part of the [17 October package set](README.md). Follow its
shared rules; no code, no web gate (documentation checks only).

**Starter:** `/goal Run package P5 from docs/goals/open-work-2026-10-17/p5-open-work-docs-sync.md on fresh origin/main; follow README.md's rules.`

## Entry conditions

Run first: P2 and P3 edit `docs/OPEN_WORK.md` at their end.

## Scope

1. **KL-185 section.** It says local branches move "via `refs/rewritten/*`";
   the rewritten copies are under `refs/kl185-rewritten/heads/<branch>`
   (`refs/rewritten/` is per-worktree and used by `git rebase -r`). Correct it
   and link [COMMIT_MAP_2026-10-17](../../history/COMMIT_MAP_2026-10-17.txt).
2. **Plug-in data adoption.** Add a dated section for KL-186 (CLI and app
   layer delivered; open: server route + web UI = P2; acceptance by the
   plug-in on the owner's installation).
3. **Unpublished packages.** Add the object-function package (feature
   `99dd6e65`, not on `main`; P1) to section 2 "Parked packages and
   publication follow-up".
4. **Prepared packages.** Link this package set from `OPEN_WORK.md` and from
   `docs/goals/` neighbours where an index exists.
5. **Ledger cross-check.** Regenerate nothing by hand; if any OPEN_WORK count
   disagrees with the ledger's *Counts* section, quote the ledger.

Before writing, re-check each item on fresh `main`; skip items another session
already fixed.

## Done when

`tools/check_documentation.py`, `check-anchors` and `check-ledger` are green on
`main` and the three statements above are true there.
