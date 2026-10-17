# 2026-10-17 — Claude/codex — read-only device diagnostic export

Owner task: offline, read-only, simulator-accepted diagnostic export so a
reporter without ETS can send a maintainer original device replies.

## Shape

- `knx-net::commissioning::diagnostic_export` — closed recipe
  `generic-identity-v1`: connect, DeviceDescriptor type 0, Device Object PID12,
  PID78, disconnect; `ManagementSession::read_only` + `AuthorisationPlan::Skip`.
  New lossless helpers `read_property_raw` / `read_descriptor_raw`; the old
  `read_property` / `read_mask_version` delegate unchanged. Property responses
  now must match the requested **start index** (only in the raw path).
- `knx-app::device_diagnostics` — the one service: target selection, asserted
  identity, loss-aware report, fixed four-member ZIP (create-only, 0600, sync,
  reread + verify), reader for reimport, per-user nonblocking file lease.
  New edge `knx-app → knx-net`.
- `knx-server` `device_diagnostics_routes` — preview / start (202, server-owned
  task) / poll / cancel / export, revision-bound, existing lock order then lease.
  `AppState::diagnostic_lease_dir` lets tests own their lease directory.
  `BusTunnel::diagnostics_are_simulated()` (default false) labels provenance only.
- `knx device diagnostics` CLI, `--simulate` without any network client.
- Web: File menu → Export diagnostic data, strict response admission in
  `diagnosticApi.ts`.

## Things learned the hard way

- The simulator names services `DeviceDescriptorRead`, not `A_DeviceDescriptor_Read`.
- `Tpci::NumberedAck` does not exist; it is `Tpci::Ack { .. }`.
- `tokio::select!` on an already-expired deadline may still poll the read once;
  check the deadline explicitly before each read (unit test pins it).
- Several server tests build `AppState` exhaustively; a new public field needs
  one line in each of them.

## Evidence (simulator only)

Rust focused targets green (net 6+5 + deadline unit, app 5, server 4, cli 2),
full knx-server tests, clippy -D warnings, layering/headers/ledger; web
component suite and build; built shell in Chromium against real handlers and the
regular simulator, EN/DE × 1440/400 px plus stop-with-partial, all downloaded
ZIPs re-verified independently with Python zipfile/hashlib.

## Pre-commit review (owner "Go", same day)

| # | Severity | Finding | Resolution |
|---|---|---|---|
| 1 | CRITICAL | `deviceDiagnostics` not in the durable activity kind list: one run set `history_failed`, blocking every later write/download in that server session | kind admitted; app + HTTP regression tests; mutants killed |
| 2 | CRITICAL | web history admission rejected the whole page ("activity history malformed"), live panel too | kind + EN/DE labels; RED→GREEN |
| 3 | IMPORTANT | known connect failure logged as `unknown` (guard dropped) | finish("failed"); mutant killed |
| 4 | IMPORTANT | HTTP capability token == archived `projectReference` | separate random token; mutant killed |
| 5 | IMPORTANT | strict start-index match leaked into existing `read_property` callers | confined to `read_property_raw` |
| 6 | MINOR | free-text error/stop tokens accepted on reimport | closed token check; mutant killed |
| 7 | MINOR | duplicated header sentences | fixed |

Lesson: HTTP harness had durable history all along; no assertion looked at it
after a run. Any new one-shot activity kind needs the Rust kind list, the web
`HISTORY_KINDS` and labels together.

No hardware run.
