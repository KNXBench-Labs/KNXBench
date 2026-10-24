# Restart snapshot

Updated: 2026-10-24 23:58 CEST. Operational snapshot, not an append-only journal.

## Current assignment and authority

- Owner asked (23:45) to work off the next 5–7 open tasks, Codex included. Six tasks in five packages are claimed below (Claude session coordinates; workers are Codex and Claude subagents).
- No real bus/device operations, vendor contact, Ollama/model downloads, Fable/Astra or release/deployment authorised. The globally excluded device remains completely off-limits.
- Each package: own worktree off `origin/main`, independent review, `tools/run_package_gate.py`, main-first merge, push after green. Nothing is merged yet.

## Claimed packages (2026-10-24, all in progress, not merged)

| Package | Branch / worktree `<sources>/knxbench-<name>` | Worker |
| --- | --- | --- |
| Gate hygiene: `test_web_gate` marker race + surviving Playwright `webServer` after SIGTERM | `fix/gate-hygiene-20261024` / `gate-hygiene-20261024` | Codex |
| Diagnostic reader cross-checks the identity stop reason (UP258 MINOR gap) | `fix/diag-stop-reason-20261024` / `diag-stop-reason-20261024` | Codex |
| CLI `device diagnostics --device <area.line.device>` | `feat/diag-device-address-20261024` / `diag-device-address-20261024` | Claude |
| Matrix: one common, persisted object-column width (UI-05 follow-up) | `feat/matrix-object-width-20261024` / `matrix-object-width-20261024` | Claude |
| Inspector Diagnostics tab: *Analyse device* + *Send report* (existing dialogs, device preselected; owner decision 23:55) | `feat/device-diagnostics-entry-20261024` / `device-diagnostics-entry-20261024` | Claude |

- Worker briefs and logs: `<profile-scratch>/batch-20261024/`.
- WP-16 AES extra-field equality was considered and deliberately not claimed: AES entries are refused outright, so it changes no admission.
- Next: review each worker result, gate, merge main-first, push, then release its row here.

## Saved-project-folder grouping: delivered, closeout owned separately

- Source delivered on main as `e6f227c4`; delivery record `da36b77b9` and [owner log](logs/2026-10-24_codex_wp06-folder-grouping.md).
- WP-06 grouping claim released. Owner still owns result-only documentation acceptance and compact evidence/cleanup; completion of that closeout has not been independently established here.
- Next for that owner: reconcile the closeout, without product-test replay or reopening delivered folder grouping.

## Remaining boundaries and preservation

- Family mapping, codecs, manufacturer folders/modules, import witnesses, priority schema and native/hardware acceptance retain their named prerequisites in maintained docs.
- WinZip AES source is already documented in the 20 October research; do not claim blanket source absence. Source availability is not verified AES support.
- Preserve selected-root private/untracked files, the separate Alpha 16 release worktree and all other owners' active evidence.
- Detailed old post-16 closeout: [owner log](logs/2026-10-24_codex_post16-closeout.md); private evidence `<profile>/release-evidence/post16-20261024`.

## Handover policy and evidence

- Read/update checkpoint rules: `AGENTS.md`. Replace relevant state, preserve other tracks, aim for at most 50 lines.
- Latest pre-compaction journal: [archive](archive/CURRENT_STATE_2026-10-24_final-pre-compact.md). [Transition notice](logs/2026-10-24_handover-notice.md) has no claimed acknowledgement.
- Repo docs own project status; external vault holds durable knowledge; Hermes sessions hold conversation history.
