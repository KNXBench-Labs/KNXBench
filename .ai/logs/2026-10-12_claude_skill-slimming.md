# 2026-10-12 — Claude — Hermes skill slimming and developer pitfalls

## Why

The knxbench Hermes profile's self-learned skills had grown by appending one
lesson per patch: `repository-delivery` reached 99,792 characters (208 below
the hard 100k cap) after 306 patches, and eleven more skills sat between 20k
and 45k. Every load paid the full size; long Pitfalls sections mixed topics and
repository facts that Codex never sees.

## What changed outside the repository (profile `knxbench`)

- Snapshot first: `hermes curator backup` →
  `skills/.curator_backups/2026-10-12T14-54-19Z`, plus a scratch copy for the
  coverage check.
- Twelve skills restructured into a short core SKILL.md (procedure, one-line
  rules, reference map, maintenance rule) plus `references/` topic files that
  hold every original entry verbatim. A script checked each original item in
  12-word windows against the new skill directory: 0 missing for all twelve.
  Core sizes before → after: repository-delivery 99.8k → 16.3k,
  autonomous-goal-boundaries 44.7k → 6.7k, project-completion-auditing
  44.3k → 6.2k, knx-live-bus-operations 41.6k → 14.1k (safety rules verbatim),
  compressed-tool-output-recovery 38.4k → 7.5k, import-evidence-integrity
  31.0k → 13.2k, cross-layer-contract-verification 30.8k → 13.5k,
  web-ui-feature-implementation 28.5k → 14.2k, project-readme-authoring
  27.5k → 8.5k, data-integrity-feature-verification 25.8k → 8.3k,
  private-corpus-regression-testing 25.5k → 12.4k, format-corpus-analysis
  20.8k → 14.4k.
- Title moved to the top in two more skills; maintenance rule added to eight
  more skills of 12–20k; five unmanaged skills adopted by the curator,
  `knxbench-release` pinned.

## What changed in the repository

- New `docs/DEVELOPER_PITFALLS.md`: repository traps, each re-checked against
  `origin/main` `ae9c44f4`. Linked from `docs/README.md` and
  `docs/VERIFICATION.md`.

## Discoveries

- The "stale xtask binary audits a deleted worktree" hazard was fixed by
  `dea70329` (2026-10-01): scan tasks resolve their root at runtime. Skill and
  memory notes that still described it as current were corrected with a date.
- CI runs `cargo test --workspace` before the frontend build, so the skill
  claim "build `apps/knx-web` before `cargo test --workspace`" is not supported
  by CI and was left out of the repository document.

## Gates

`python3 -m unittest tools.tests.test_check_documentation` OK;
`python3 tools/check_documentation.py` 275 files, 0 errors;
`cargo run --locked -p xtask -- check-anchors` 575 links, none dead (root =
this worktree); `git diff --check` clean.
