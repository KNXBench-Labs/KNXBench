# Removed documentation

On 2026-10-08 the user decided to take these paths out of the working tree.
They are **not lost**: every file is unchanged in Git at commit
`bca2d3336b9692e7a6ece18804d187ecdda6968f` (the last `main` commit that
contains them; it was `138403ed6084` until the public-launch history purge and
`6a1ba6ae5d54` before the same-day identity rewrite, see
[known limitation §162](../KNOWN_LIMITATIONS.md#162-commit-hashes-cited-before-2026-10-07-refer-to-the-rewritten-history)). Links in maintained docs that pointed into them were rewritten
to pinned GitHub URLs at that commit, so they still open the exact old file.

```sh
git show bca2d3336b96:<path>                 # read one file
git ls-tree -r --name-only bca2d3336b96 docs/superpowers   # list a folder
git restore --source=bca2d3336b96 -- <path>  # bring a file back
```

Web: `https://github.com/KNXBench-Labs/KNXBench/tree/bca2d3336b96/<path>`.

| Removed path | Files | What it held |
| --- | ---: | --- |
| `docs/archive/` | 28 | Finished goals (`goal.md`, `alpha-release-goal.md`, `goal-ui.md`, `goal-commission.md`), the alpha evidence dossiers (`ALPHA_READINESS`, `UI_ALPHA_READINESS`, `COMMISSIONING_ALPHA_LEDGER`, `ALPHA_SCOPE_MATRIX`, `ALPHA_FINAL_GATES`, `ALPHA_CANDIDATE`, `LIMITATION_TRIAGE`, `MANUAL_ACCEPTANCE`, `ADR0039_ENFORCEMENT_AUDIT`), the AR18 review rounds, the frozen 180-ID inventory `OFFENE_PUNKTE.md` (source of the ledger's snapshot rows), `CLOUD_SESSIONS.md`, `PROJECT_EVOLUTION_GOAL.md`, the 2026-09-15 project analysis and two dated root snapshots (former `stats.md`, `compare.md`) |
| `docs/superpowers/` | 97 | Session design specs and implementation plans (2026-09-02 onwards). Many ADRs, tests and source comments still name these files as their design origin |
| `docs/design/` | 32 | Codex UI concept/proof (2026-09-13) and the telegram-flow U19–U21 studies with their measurements |
| `docs/design-studies/` | 30 | The LCARS study, its verification scripts, receipts and screenshots |
| `KNX ETS Alternative – Claude Code Development Strategy.md` (root) | 1 | The original development-strategy prompt |
| `IDEA.md` (root) | 1 | The one-line original idea |

## What still names them

- **Source comments and tests** (about 40 files) cite `docs/superpowers/…` specs
  as design origin, and a few cite the commissioning goal's package numbers
  (K2, K19). Read those names as Git paths at the commit above.
- **Story editions** (`story/candidates/`, `story/content/edition.json`) carry
  them as evidence references with source `src-git`, i.e. as Git provenance.
  Editions are frozen, so they were not edited.
- **`.ai/` handovers and logs** and `docs/history/` are historical and keep
  their old wording; links in `docs/history/` were pinned like the rest.
- The telegram-flow load study (`apps/knx-web/e2e/flow-load.load.ts`) now
  writes new measurements to `docs/evidence/telegram-flow-u21/`; the 2026-10-04
  figures stay at the commit above.
- `xtask check-ledger` still skips any `archive/` or `history/` directory; that
  rule is harmless without the folder.

## Second round, 2026-10-08 (at `a584007fc05a`; `aa0ff14ff536` before the public-launch purge, `62a54e70cbc4` before the identity rewrite)

Also removed on the user's request; readable with
`git show a584007fc05a:<path>`, links pinned to that commit.

| Removed path | What it held |
| --- | --- |
| `docs/GAP_ANALYSIS_ETS.md` | Feature-by-feature gap analysis against ETS (rows B1–D12). Ledger evidence, limitations and source comments (`GAP_ANALYSIS_ETS.md B9`, `C2`, `C3` …) still cite its row IDs; current claims live in [COMPATIBILITY](../COMPATIBILITY.md) |
| `docs/Issues.md` | The user's empty intake note for observations |
| `.serena/` | Tracked `.gitignore` of the Serena MCP config; the local folder moved to `KNXBench.backups/root-cleanup-2026-10-08/serena` |
| `tools/agent_memory_sync.py`, `tools/tests/test_agent_memory_sync.py` | Generator of the shared agent-memory index `.agent-memory/`. Decommissioned: systemd user timer `knxbench-memory-sync` disabled and removed, marker blocks removed from the root `MEMORY.md` and the Hermes `SOUL.md` by its own `uninstall-*` commands; installed copy, unit files, the old index and pre-change copies are in `KNXBench.backups/memory-sync-removal-2026-10-08` |
| `tools/cloud/` | Claude Code cloud-session hook, setup script and rules (cloud sessions paused since 2026-09-28); the `SessionStart` hook in `.claude/settings.json` was removed with it |
