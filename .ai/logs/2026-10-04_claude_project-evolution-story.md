# 2026-10-04 — Claude: project-evolution story, first private version

Goal: `docs/PROJECT_EVOLUTION_GOAL.md` (user-started `/goal`). Agent: Claude
(claude-opus-5-5 via Hermes). No subagents, no commits, no push, no KNX hardware.

## Baseline and workspace

- Worktree `/mnt/daten-i/Sourcecode/KNXBench.worktrees/project-evolution-story-20261004`,
  branch `story/project-evolution-20261004`, based on fetched `origin/main`
  `75ad9650351f80fd147d683f3a6aeb34ec0dfd64` (root `main` is `307a5970`, 112 behind;
  root left untouched by this session apart from an additive handover pointer).
- Observed, not caused: concurrent Hermes session `20261003_160758_885821` fast-forwarded
  the root `main` from `307a5970` to `75ad9650` at 01:00:20 CEST (reflog). This work never
  ran merge/pull/reset in the root.
- Brought over byte-identically (sha256 verified): goal, brief, Phosphor Atlas HTML and PNG.
  Brief later amended with a dated status update.

## Source archaeology

Read (KNXBench-attributable, local only): Git history, ADRs, docs, `.ai/` handovers;
111 Claude Code transcripts (project folder under old name `KNX`, renamed `KNXBench`,
worktrees, `knx-spec-kb`) and `~/.claude/history.jsonl`; 202 Codex threads and
`~/.codex/history.jsonl`; 81 Hermes `knxbench` sessions plus cwd-filtered default
profile; Paperclip DB backup (issues/agents/goals/projects/companies tables only —
the user asked mid-session to include `~/.claude`, `~/.codex`, `~/.paperclip`).
Not read: cloud sessions, remote host, `.env`/auth/secret tables, ETS installs,
OriginalData corpus.

Key facts: earliest surviving prompt 2026-09-02 13:50 CEST (home ETS project +
bus tools); founding prompt 14:23 CEST ("read the strategy document and get
going"); strategy-document origin unknown; rename KNX→KNXBench 2026-09-05; history
rewritten twice + remote recreated 2026-09-20; Paperclip experiment 09-25..27
(8 roles, 50 issues mostly blocked, paused and shut down); all commits under the
user's identity. Raw extracts contain sensitive material and remain private.

Private ledger: `/mnt/daten-i/Sourcecode/KNXBench.story-private/` (0700/0600):
`raw_prompts.json`, `human_prompts.json`, `provenance.json` (36 events mapped).

## Implementation (`story/`)

- `content/edition.json`: 36 steps, 8 strands, 46 typed relations (8 editorial),
  8 chapters, 6 gaps, 29 excerpts (translation/privacy/shortened labels derived).
- `storytool` (stdlib Python): validate, privacy scan, provenance leak refusal,
  deterministic layout, immutable candidates + diff + REVIEW, build (single HTML,
  CSP hashes, escaped data island), loopback `serve`, `release-check`,
  always-refusing `publish`.
- `site/`: Phosphor Atlas CSS; vanilla JS growing story graph, full view with
  search/strand focus/pan/zoom/keyboard/inspector, text-only mode, motion off +
  live OS reduced-motion cancellation.
- ADR-0068; IMPLEMENTATION_STATUS entry; `story/README.md` (workflow, archaeology,
  limitations).

## Candidates

- `2026-10-04.1` story_sha256 `45274796a16111081df96139e34ffe8cfd8dc15652e864719523c7bd83cc1715` (35 steps).
- `2026-10-04.2` story_sha256 `70cc71ed42495c96aded8de1858bfe79cafbceb442a4bc9146ad2549e789cd34`
  (+ `story-first-preview`, cutoff and coverage updated); diff vs `.1`; `.1` verified
  byte-identical afterwards.

## Verification receipts

- Unit tests: 49/49 OK (`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -t tests`),
  including the real-ledger canary test (not skipped on this machine).
- Negative controls: removing HTML escaping, script-island escaping, the translation
  rule, the stale-approval check and the leak refusal each failed the suite; restored.
- Browser (Playwright 1.63 Chromium): 41/41 for each candidate; desktop 1440×900 and
  mobile 390×844 narrative, keyboard, evidence, search/focus/zoom/drag/keys, text-only
  persistence, no-JS, hostile text, live Motion-off and OS-reduce cancellation
  (running animations > 0 → 0), zero console/CSP errors. Receipts kept in the
  private directory under `receipts/`.
- `serve` answered HTTP 200 on 127.0.0.1 only and was stopped; non-loopback refused (tests).
- `xtask check-anchors` with a fresh target, gate target = this worktree: 382 links /
  252 Markdown files, none dead. `git diff --check` clean; new files have no trailing
  whitespace. No Rust/web product code changed, so product suites were not run.

## Bugs found and fixed during the run

- Privacy scanner skipped all events (key `events` treated as chapter id list) —
  fixed + regression test.
- Programmatic chapter scroll re-showed intermediate chapters and cancelled growth —
  observer now stands down during navigation (browser check caught it).
- Label collisions/date pile-up on mobile and view drifting off the graph — decluttering
  and clamping; `story/.gitignore` patterns were root-relative — fixed.

## Open

Manual content/privacy review of the candidates; publication design and approval;
fonts/licensing; Firefox/WebKit/screen readers; cloud-session coverage; whether commit
references may be public. REVIEW location strings use content indices (`events[33]`),
not ids — cosmetic, left for a later tool version.

## Content review by the user (2026-10-04, after delivery)

Candidate `2026-10-04.2` reviewed; no content changes requested, so no edition `.3`:
- Bus house-rules excerpt: keep the redacted quote plus summary.
- Keep both jokes from internal data ("Head of All", README "must have been drunk").
- Keep commit hashes although the GitHub repository is currently not publicly reachable
  (anonymous fetch 404); the user expects it to become public. Recheck before publishing.
- Translations of the German prompts: accepted as they are.
This is a content review, not a publication approval; `release-check` still refuses.

## Delivery (2026-10-04 07:10 CEST, user go)

Five focused commits (author/committer github@knxbench.com, no co-author), rebased onto
`origin/main` `9d719a4f`. Upstream had taken ADR 0067 (download lifecycle), so this ADR is
now 0068; all references updated. Conflicts in `IMPLEMENTATION_STATUS.md` (newest first,
both kept) and `docs/adr/README.md` (both rows kept) resolved by script; the handover
suffix equals upstream byte for byte. Gates on the rebased tree: unit 49/49, browser 41/41
(`2026-10-04.2`), `check-anchors` 382 links/254 files, `check-headers` within ceiling,
`git diff --check` clean. Pushed from the owned worktree to `main`; root checkout not synced.
