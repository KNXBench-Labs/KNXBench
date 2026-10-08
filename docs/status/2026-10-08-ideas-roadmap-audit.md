# Ideas and roadmap audit — 8 October 2026

## Scope and evidence rules

Checked against source `608a204bf28aa9df83413aa5ccb65241c32478aa` and the
existing documentation refresh in `docs/refresh-20261008`. The original twelve
entries live in the **ignored, root-local** `ideas.md`; that file is absent from
Git and isolated worktrees. This record makes their assessment available in the
repository. It does not create a second source-ID ledger or change owner rows.

- **Implemented at stated scope:** reachable implementation with named tests or
  retained acceptance. It is not a promise of every-device compatibility.
- **Partial:** a useful part exists, but the original idea has a real remainder.
- **Not implemented:** no end-to-end feature; research or an ADR is not delivery.
- **Accepted/deferred boundary:** preserve the recorded decision and refusal.
  It is neither working functionality nor permission to resume development.
- Source delivery, local documentation edits, a released AppImage and public
  website deployment are separate facts.

## Every original idea, checked

| Original idea | Assessment | Source / existing verification | Actual remainder |
| --- | --- | --- | --- |
| Small interface animations | Implemented at stated scope | [Motion](../../apps/knx-web/src/motion.ts), [tests](../../apps/knx-web/src/motion.test.ts), [motion guard](../../apps/knx-web/src/motionGuard.test.ts) | Per-category controls are later scope; native/accessibility coverage stays disclosed. |
| MCP capabilities | Partial: read-only adapter delivered | [MCP tool surface](../../apps/knx-mcp/src/server.rs), [tool tests](../../apps/knx-mcp/tests/tools.rs), [stdio tests](../../apps/knx-mcp/tests/stdio.rs), [ADR-0090](../adr/0090-read-only-mcp-adapter.md) | Agent-driven edits, chatbox and bus work are not supplied. Saved snapshots only; experimental contract. |
| Device discovery | Implemented at stated scope | [Gateway discovery](../../apps/knx-web/src/busDiscovery.ts), [line-scan panel](../../apps/knx-web/src/LineScanPanel.tsx), [bus workspace](../../apps/knx-web/src/BusDiagnosticsPanel.tsx), limitations §72–79/§124 | One-line occupancy is not product identity or coupler traversal; real gateway/platform scope is bounded. |
| Repetitive-task automation | Not implemented | [Atomic command batch](../../crates/knx-core/src/command.rs), [research decision](../research/features-and-ui.md#14-repetitive-task-automation-and-macro-layer-decision-2026-09-22-t20) | No generic macro UI, recorder, template executor or scheduler. A revision-bound preview is still required. The old “only four undo commands” claim is obsolete. |
| Humour templates | Implemented at stated scope | [Copy lists](../../apps/knx-web/src/toastCopy.ts), [toast tests](../../apps/knx-web/src/toast.test.ts) | Thirty error wrappers and thirty late-night entries already exist; a larger catalogue is optional content, not an unbuilt template mechanism. |
| In-project documentation / notes | Not implemented | [ADR-0031](../adr/0031-project-notes-are-a-project-owned-collection.md), [project aggregate](../../crates/knx-core/src/project.rs) | Collection, persistence migration, undoable commands and editor are absent. F1 help and HTML report export are different features. |
| Project status | Partial: dashboard delivered | [Dashboard](../../apps/knx-web/src/Dashboard.tsx), [statistics tests](../../apps/knx-web/src/dashboardStats.test.ts), actual caller in [App](../../apps/knx-web/src/App.tsx) | Counts are display-only; count-to-detail drill-down remains absent. |
| Animated GA/device connections | Implemented at stated scope | [Flow view](../../apps/knx-web/src/TelegramFlowView.tsx), [monitor caller](../../apps/knx-web/src/BusMonitorPanel.tsx), [Flow tests](../../apps/knx-web/src/TelegramFlowView.test.tsx), [accepted scope](../TELEGRAM_FLOW_VISUALIZATION.md), ADR-0077/0085 | Session-local observation; inferred recipients are not proof of receipt or effect. No persistent traffic history or broader native/live acceptance. |
| Mobile application | Not implemented | Linux-first scope in [architecture](../ARCHITECTURE.md) and [manual](../manual/implementation-status.md#desktop-application) | No dedicated mobile application or mobile commissioning workflow. Browser access is not a native app. |
| Other native operating systems | Not implemented | [Desktop status](../manual/implementation-status.md#desktop-application), recorded Linux-first scope in the [ledger](LEDGER.md) | No shipped Windows/macOS native build. Using the web workbench from another OS is already a separate option. |
| Themes | Implemented at stated scope | [Registry](../../apps/knx-web/src/theme.ts), [bundled packs](../../apps/knx-web/src/bundledThemes.ts), [registry tests](../../apps/knx-web/src/theme.test.ts), [tokens](../../apps/knx-web/src/themeTokens.test.ts), [theme-pack contract](../THEME_PACKS.md) | Built-ins are Porcelain/Graphite/Cupertino/LCARS plus System; shipped packs are separate. Neon Grid/Bitcoin DeFi were removed by decision. No arbitrary CSS/code plug-ins. |
| Schema 21/23 import | Partial: imports delivered, evidence bounded | [Compatibility](../COMPATIBILITY.md#2-verified-today), [product namespace admission](../../crates/knx-productdb/src/package.rs), [scheme evidence](../../crates/knx-productdb/src/parse/scheme_evidence.rs), [ADR-0028](../adr/0028-no-knxproj-export.md), ADR-0072/0083 | Independent module-using schema-23 project evidence remains absent. Schema-23 master/product admission is no longer an unimplemented prerequisite. No `.knxproj` writer; native `.knxdb` roundtrip is not ETS roundtrip. |

This is an inventory of unequal ideas, **not a completion percentage**.

## Other roadmap contradictions corrected

| Old wording | Checked implementation / record | Correct interpretation |
| --- | --- | --- |
| “L3 web upload is next” | [Legacy route](../../apps/knx-server/src/routes.rs), [shared install control](../../apps/knx-web/src/ProductInstallControl.tsx), [HTTP tests](../../apps/knx-server/tests/http_legacy_install.rs), [ADR-0094 L3](../adr/0094-legacy-exim-product-files.md#amendment-l3-2026-10-08-upload-password-dialog-one-remembered-password) | Web upload, password dialog and one remembered password exist. Oversized `.vd5` coverage and L4 download remain separate; the legacy 64 MiB bounds were not raised. |
| “Global Web history consumer is open” | [Bus tabs](../../apps/knx-web/src/BusDiagnosticsPanel.tsx), [history UI](../../apps/knx-web/src/BusActivityHistory.tsx), [history tests](../../apps/knx-web/src/BusActivityHistory.test.tsx), [HTTP tests](../../apps/knx-server/tests/http_activity_history.rs), [activity journal](../../crates/knx-app/src/commissioning_activity.rs) | Bounded commissioning metadata is visible. It is not persistent project undo, device recovery or a complete traffic journal; long-session/untracked coverage stays disclosed. |
| “Final alpha review/tag still pending” | [Historical final gates](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b9692e7a6ece18804d187ecdda6968f/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md), release rows in [ledger](LEDGER.md), published `v0.1.0-alpha.5` record | Historical alpha closeout is delivered. A new release requires its own decision/build/gates, not reopening completed alpha goals. |
| “Website deployment is next / repository is private” | Published launch/Pages records in [implementation log](../IMPLEMENTATION_STATUS.md), [website contract](../WEBSITE.md#release-and-deployment-2026-10-08) | Repository is public; website/story are deployed. Redirect/IPv6 checks, workflow runtime maintenance and owner privacy review are separate follow-ups. No deployment performed by this audit. |
| “Schema-23 manufacturer ingestion is missing” | Exact-namespace admission in [package parser](../../crates/knx-productdb/src/package.rs), ADR-0072/0083 | Parser/persistence support exists; this does not prove complete manufacturer semantics or module-based project coverage. |
| “Humour needs its first thirty messages” | Both thirty-entry lists in [toastCopy](../../apps/knx-web/src/toastCopy.ts) | Original requirement delivered; expansion is optional. |
| “Address programming is in progress” | Public fail-closed safety boundary, ADRs 0057–0059 and recorded owner scope | Currently unavailable, not an active implementation task. Reopening needs a decision and device-specific durable recovery. |

## What remains, without inventing a queue

Use [OPEN_WORK](../OPEN_WORK.md) for the maintained list. Relevant groups:

- **Current follow-up / interview:** device-centric navigation; legacy `.vd5`
  resource measurement and subsequent L4 scope; website maintenance and owner
  decisions. A named interview or worktree is not completed functionality.
- **Absent later features:** notes, macros, dashboard drill-down, persistent undo,
  versioned backups, selective import, diff apply/merge/three-way comparison,
  dedicated parameter widgets and manufacturer online-catalog updates.
- **Missing evidence:** independent untested-schema/module-23/protected-project
  samples; native WebKitGTK, Orca and wider host/browser evidence. Previously
  accepted alpha boundaries are not automatically reopened as release blockers.
- **New protocol scope:** KNX Secure, additional media/masks/procedures and
  legacy download. Unsupported device operations stay refused.
- **Recorded non-goals:** `.knxproj` export, `.vd2`, encrypted modern product
  packages, code plug-ins, spatial floor-plan editing and multi-user editing.
  Their individual decision scopes are recorded in the manual and ADRs.

The ledger remains byte-for-byte unchanged. Its older future rows for MCP,
Flow and humour are bookkeeping boundaries to be reconciled by their owners,
not evidence that the delivered narrower features are absent.

## Verification scope

This is a documentation/source audit, not new hardware or private-corpus
acceptance. Focused tests, documentation checks, source-preservation assertions
and the final self-review are recorded in
[the audit receipt](../evidence/ideas-roadmap-audit-2026-10-08.json).
Existing real-app screenshots remain bound to source `4c84ab9f`; application
source did not change between that snapshot and the audit base. Website footage
has its own evidence scope and is not substituted for those captures.
