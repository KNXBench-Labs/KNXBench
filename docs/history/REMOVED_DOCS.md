# Removed documentation

On 2026-10-08 the user decided to take these paths out of the working tree.
They are **not lost**: every file is unchanged in Git at commit
`6a1ba6ae5d54b8308326e55a28ee7beaa5e280a5` (the last `main` commit that
contains them). Links in maintained docs that pointed into them were rewritten
to pinned GitHub URLs at that commit, so they still open the exact old file.

```sh
git show 6a1ba6ae5d54:<path>                 # read one file
git ls-tree -r --name-only 6a1ba6ae5d54 docs/superpowers   # list a folder
git restore --source=6a1ba6ae5d54 -- <path>  # bring a file back
```

Web: `https://github.com/KNXBench-Labs/KNXBench/tree/6a1ba6ae5d54/<path>`.

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
