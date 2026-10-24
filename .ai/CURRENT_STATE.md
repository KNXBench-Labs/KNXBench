# Restart snapshot

Updated: 2026-10-25 00:22 CEST. Operational snapshot, not an append-only journal.

## Current assignment and authority

- Diagnostic stop-reason package is implemented and independently reviewed with no findings on `fix/diag-stop-reason-20261024`.
- Both archive profiles now cross-check identity stop reasons, exact item shapes and phase-specific failures against real collector behavior.
- RED evidence captured for contradictory rehashed reasons, missing assertions after reads and impossible property shapes.
- GREEN: `knx-app` 662 passed/45 ignored; diagnostic collector 20 passed; server diagnostics 5 passed; clippy, fmt, headers, layering, anchors, docs, local-info and diff checks passed.
- Package commit created. Owner prohibited push, merge, rebase, real bus/device access and the package gate. Next: hand off for coordinator gate/integration.
- No real bus/device operations, vendor contact, Ollama/model downloads, Fable/Astra or release/deployment authorised. The globally excluded device remains completely off-limits.

## Other claimed packages (preserved from upstream coordination)

| Package | Branch / worktree `<sources>/knxbench-<name>` | Worker |
| --- | --- | --- |
| Gate hygiene: `test_web_gate` marker race + surviving Playwright `webServer` after SIGTERM | `fix/gate-hygiene-20261024` / `gate-hygiene-20261024` | Codex |
| CLI `device diagnostics --device <area.line.device>` | `feat/diag-device-address-20261024` / `diag-device-address-20261024` | Claude |
| Matrix: one common, persisted object-column width (UI-05 follow-up) | `feat/matrix-object-width-20261024` / `matrix-object-width-20261024` | Claude |
| Inspector Diagnostics tab: *Analyse device* + *Send report* | `feat/device-diagnostics-entry-20261024` / `device-diagnostics-entry-20261024` | Claude |

## Saved-project-folder grouping: delivered, closeout owned separately

- Source delivered on main as `e6f227c4`; delivery record `da36b77b9` and [owner log](logs/2026-10-24_codex_wp06-folder-grouping.md).
- WP-06 grouping claim released. Owner still owns result-only documentation acceptance and compact evidence/cleanup; do not reopen delivered folder grouping.

## Remaining boundaries and preservation

- Family mapping, codecs, manufacturer folders/modules, import witnesses, priority schema and native/hardware acceptance retain their named prerequisites in maintained docs.
- WinZip AES source is already documented in the 20 October research; source availability is not verified AES support.
- Preserve selected-root private/untracked files, the separate Alpha 16 release worktree and all other owners' active evidence.

## Handover policy and evidence

- Read/update checkpoint rules: `AGENTS.md`. Replace relevant state, preserve other tracks, aim for at most 50 lines.
- Repo docs own project status; external vault holds durable knowledge; Hermes sessions hold conversation history.
