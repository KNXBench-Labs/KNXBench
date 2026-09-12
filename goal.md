# KNXBench completion goal

Use this file as the instruction passed to `/goal`.

Drive KNXBench to a genuinely complete, trustworthy v1 state. Work
autonomously and persist across turns until every actionable task, gap,
regression, implementation-status item, roadmap deliverable, and known
limitation has either been resolved with evidence or explicitly accepted as
out of scope by the user.

## Priority: manufacturer product databases and compatibility corpus

Treat installation of devices from freely downloadable manufacturer product
databases as the highest-priority current compatibility gap. The intended
result is that a user can install a device using its manufacturer-supplied
product database, without requiring that the product data first appeared in
an imported ETS project. Do not claim this capability complete until import,
validation, normalized storage, device creation, user-visible errors, and
focused regression coverage all work end to end.

Use `OriginalData/ProductDatabases/` as the local evidence corpus. It
currently contains five `.knxprod` files spanning ETS2–ETS4-era and newer
product data, plus one legacy `.vd2` database:

- `646704-04_ETS4_2012_47_DE_EN.knxprod`
- `Dummy_Applikation_Secure.knxprod`
- `MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`
- `Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2`
- `Weinzierl_730_KNX_IP_Interface_ETS4.knxprod`
- `Weinzierl_730_KNX_IP_Interface_ETS4_v1.knxprod`

Inspect every container's actual master-data scheme and format before
designing parser, migration, or decryption support. Preserve unsupported data
or report it explicitly—never silently discard it.

Use `OriginalData/DemoProjects/` as the ETS-project compatibility corpus.
Run each available project through detection, import, validation, and the
appropriate export/round-trip tests. The current fixtures are `KV v2.5 -
demo.knxproj`, `Unser Zuhause ets4 - 2025-12-15.knxproj`, and `Unser Zuhause
ets 6.3.0 - 2026-09-02.knxproj`. Record their detected schemas and support
outcomes in the same evidence matrix; never extrapolate support from one
sample.

## KNX Standard reference

For any unclear KNX protocol, data-model, XML, product-data, medium, or
KNXnet/IP behavior, consult the relevant Markdown source in the extracted
KNX Standard v3.0.0 corpus at:

`/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0/`

This corpus has 179 Markdown documents organized by standard volume. Search
only the relevant subfolders and treat the applicable standard text as the
technical verification source before implementing or changing behavior. Record
the exact source file and section in the implementation notes, tests, or
compatibility documentation. If the standard is ambiguous, the corpus lacks
the needed material, or repository evidence conflicts with it, preserve data,
report the ambiguity, and do not invent semantics.

## Operating rules

1. Start every work cycle by reading `.ai/CURRENT_STATE.md`,
   `docs/IMPLEMENTATION_STATUS.md`, `docs/KNOWN_LIMITATIONS.md`,
   `docs/ROADMAP.md`, `docs/GAP_ANALYSIS_ETS.md`, and the relevant source of
   truth for the work item. Reconcile stale or contradictory status entries.
2. Work from the highest-risk correctness, data-integrity, compatibility, or
   user-visible gap down. Prefer a small coherent vertical slice over broad
   speculative work.
3. Follow `AGENTS.md` exactly. Respect the architecture boundaries and keep
   external-format handling lossless and honestly reported.
4. Before modifying behavior, trace the responsible layer, invariants,
   existing tests, and documentation. Do not paper over a domain or
   persistence problem in the UI.
5. For every functional change, add focused regression coverage and run the
   smallest sufficient checks. Run broader repository gates before declaring
   an item complete when they are relevant.
6. Update the applicable status, limitation, roadmap, compatibility, and
   architecture documentation in the same change. Keep `.ai/CURRENT_STATE.md`
   and a dated `.ai/logs/` handover record current.
7. Do not silently downgrade a limitation. If a task depends on unavailable
   samples, specifications, credentials, hardware, or a user decision, record
   the exact blocker and continue with another actionable item. Ask the user
   only for the missing authority or evidence.
8. Do not claim KNX certification or full ETS compatibility. Use only claims
   supported by repository evidence and say "KNX-compatible" where relevant.

## GitHub and VS Code workflow

The implementation session may start from VS Code; keep that workflow usable.
Before each coherent work item, inspect the active branch, worktree status,
and GitHub remote. Preserve unrelated local changes and never reset, discard,
or overwrite them.

Use GitHub as the shared delivery record: keep the branch and commit history
meaningful, relate completed work to the relevant issue/task/gap where one
exists, and prepare a focused pull request with verification evidence once a
coherent item is ready. Fetch remote state before making integration claims.

Act autonomously when GitHub credentials and repository permissions permit:
create focused branches, make small verified checkpoint commits, push them,
open or update pull requests, link and close the corresponding task/issue/gap,
and merge a ready pull request after its relevant checks pass. Use Git as the
safety net: commit before risky or multi-file work, inspect the diff before
each commit, fetch before integration, and leave a clean, recoverable history.
Never force-push, rewrite shared history, bypass a required review/check, or
discard unrelated local changes. If GitHub access is unavailable, continue
locally, record the exact pending GitHub action, and hand it back clearly.

## Parallel, subagent-driven delivery

Use Subagent-Driven Development (SDD) for planned work. Decompose the backlog
into small, testable work items; give each agent a narrow brief, an isolated
worktree when it will edit files, explicit acceptance criteria, and a report
path. Maintain a durable ledger so completed work is never redispatched after
context compaction.

Maximize parallelism only where tasks are genuinely independent: they must not
edit the same files, rely on an unfinished interface from another task, or
need the same mutable environment. With the available four slots, keep at
most three subagents active alongside the coordinator. Good parallel work
includes repository reconnaissance, fixture/schema analysis, independent test
audits, documentation reconciliation, and review. Implementing changes that
share a boundary remains sequential: implement, run focused checks, conduct a
separate specification-and-code review, then resolve findings before the next
dependent task.

Select a subagent model and reasoning effort explicitly for every dispatch;
never inherit the coordinator's defaults.

| Work type | Model | Effort |
| --- | --- | --- |
| Mechanical, fully specified 1–2-file edit; focused test or re-review | `gpt-5.6-luna` | `low` |
| Multi-file implementation, integration, ordinary debugging, or code review | `gpt-5.6-terra` | `medium` |
| Compatibility research, architecture/domain decisions, security/data-integrity work, difficult debugging, or final whole-branch review | `gpt-6-astra` | `high` |

Escalate one tier after a repeated blocker or failed fix round. Batch only
same-shape mechanical edits that share no risk boundary. Before integration,
the coordinator reviews each subagent's diff, test evidence, documentation,
and report; run one final whole-branch review using `gpt-6-astra` at `high`
effort before merging.

## Completion condition

Finish only when the status, roadmap, gap analysis, and known-limitations
documents have been reconciled; all actionable items have proof of completion;
all tests and relevant quality gates pass; and every remaining exception has
the user's explicit out-of-scope acceptance or a documented, non-actionable
external blocker. Report the completed work, verification evidence, remaining
external blockers, and the next required user decision, if any.
