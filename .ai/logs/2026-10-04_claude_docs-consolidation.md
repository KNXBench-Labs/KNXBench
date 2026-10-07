# 2026-10-04 — Claude — docs consolidation (AR14D D1)

User question: which status documents are redundant, what should be merged or
split for readability. Decision delegated ("whatever makes most sense"; plan the
complicated steps before AR15).

## Inventory (origin/main d8540b82, before the change)

| File | Lines | Role |
| --- | ---: | --- |
| docs/IMPLEMENTATION_STATUS.md | 15,220 | newest-first delivery log, 293 sections |
| docs/KNOWN_LIMITATIONS.md | 7,628 | 130 numbered headings, 132 files link in |
| docs/RESEARCH.md | 6,759 | 102 headings |
| .ai/CURRENT_STATE.md | 5,485 | handover, 545 entries (82 on 2026-10-04 alone) |
| alpha-release-goal.md | 1,155 | AR packages + 180-ID ledger + receipts |
| docs/ALPHA_READINESS.md | 530 | per-ID evidence and a second per-ID ledger |
| docs/COMMISSIONING_ALPHA_LEDGER.md / UI_ALPHA_READINESS.md | 174 / 233 | track per-ID ledgers |
| docs/LIMITATION_TRIAGE.md | 197 | priority of KL entries |
| OFFENE_PUNKTE.md | 259 | frozen 180-ID snapshot (2026-10-01) |
| goal.md | 336 | retired executor (AR00) |

Per-ID status lives in up to six places; drift already visible (KL-149 `TODO`
in the §8 table, scoped candidate in the AR06P text).

## Delivered (D1)

- `git mv` of goal.md, OFFENE_PUNKTE.md, PROJECT_ANALYSIS_2026-09-15.md to
  docs/archive/ + README. Link rewrite by script over all tracked Markdown
  outside `.ai/` (links resolved relative to each file's old location).
- IMPLEMENTATION_STATUS cut below the last October heading (line 2695 of
  15,221): 12,526 lines to docs/history/IMPLEMENTATION_STATUS_2026-09.md,
  196 relative links rewritten for the new directory. 17 late-recorded
  September entries stay above the cut.
- Handover: entries after "codex (alpha / KL-151 research clean …)" archived
  (470 entries) so every active track's newest entry stays; reconstruction
  assert `kept + archived == original` passed before writing.
- Gates: check-anchors (fresh CARGO_TARGET_DIR) 393 links / 262 files, none
  dead after re-pointing two anchors (THEME_PACKS, 2026-09-21 plan). Plain
  file-link scan: 17 dead links, all pre-existing (old plans, ../CLA.md).
  `git diff --check` clean.

## Not done / planned

AR14D D2–D5 in alpha-release-goal.md: single ledger (ADR first), xtask ledger
check, RESEARCH topic split, KNOWN_LIMITATIONS resolved bodies to history
behind stubs, goal-ui/goal-commission status sections become links (owner
agreement needed). Kept: IDEA.md, docs/Issues.md (user inbox), compare.md,
stats.md/ProjectStats.md (statistics track, not checked for overlap).

## D2–D5 (status-docs lock, 2026-10-04 18:27–18:45)

- **D2** `257dd8c5`: ADR-0076, `docs/status/LEDGER.md` (180 + 5 rows) built by
  script from goal §7–§8, ALPHA_READINESS tables and the commissioning
  disposition column. Cell-presence proof: every moved cell found in the
  ledger. 24 goal/readiness differences; 6 evidence corrections (DATA-02,
  KL-42, KL-149, KL-150, KL-152 → DONE; KL-151 → IN_PROGRESS); 3 routes.
  Found: AR06 is DONE_SCOPED but mapped no row — left TODO, owner to map.
- **D3** `e9400950`: `xtask check-ledger`, 10 tests RED-first (9 RED, 1
  trivially green exemption case), 7/7 mutants killed (one anchor needed a
  retry after rustfmt rewrapped `SKIP_DIRS`), real-repo negative control
  named both seeded problems. CI step + VERIFICATION + contributing guide.
- **D4** `cc2916c6`: RESEARCH split into five topic files; heading parse is
  fence-aware (a `# $S …` line inside §8's code block is not a heading).
  Line multiset proof: 0 original lines missing; 14 inbound anchors moved.
- **D5**: 5 resolved KL bodies (163 lines) to history behind stubs; §90/§95
  kept (pointers, not defects). goal-ui narrative to UI_ALPHA_READINESS with
  owner agreement. goal-commission.md untouched (no owner answer yet).
- Scripts lived in scratch (`ar14d/`), removed after delivery.
