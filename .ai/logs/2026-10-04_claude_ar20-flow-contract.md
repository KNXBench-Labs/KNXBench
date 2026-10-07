# 2026-10-04 — Claude — AR20 telegram-flow backend contract

Input: U19 handoff (TELEGRAM_FLOW_VISUALIZATION §9.2). Output: §10 of the
same document (final wire names), consumed next by U20.

## Changes
- `apps/knx-server/src/flow.rs` (new): `FlowParticipants::from_project`
  (devices with installation/name/raw IA, groups with Send/Receive members,
  activation and six nullable flags, diagnostics), `FlowListsDto::bounded`
  with `FLOW_LIMITS` and truncation counts.
- `bus.rs`: `TelegramRow` gains `source_raw`, `destination_raw`,
  `observed_at: Instant`, `flow_generation`; `GroupAddressContext` gains
  `flow` (compared) and `generation` (not compared, manual `PartialEq`);
  `update_group_address_context` keeps/advances the generation;
  `MAX_SAFE_COUNTER` refusal and saturation.
- `bus_routes.rs`: row DTO fields, poll `flowGeneration`, new
  `GET /api/bus/monitor/flow-snapshot`.

## Deviations from the U19 proposal (named in §10)
- `flags` is never `null`; each flag is nullable instead (more exact).
- `active` is a plain boolean (the model has no unknown activation).
- `historical` returns empty lists (the server keeps only the current
  snapshot, as the proposal said).
- Generation advances only when the session context is republished
  (style, undo, redo); other edits show `contextStatus: "stale"`.

## Evidence
RED first: `flow::tests` 6 of 7 failing against a stub, `http_bus_flow`
7 of 8 failing before the implementation. Nine guard mutants killed
(flow not compared, always/never bump, counter limit, age zero, snapshot
ignoring generation, absent flags as false, dangling diagnostic, member
cap). Gates: fmt, clippy -D warnings (workspace without knx-desktop),
knx-server 626 passed / 0 failed / 44 ignored, check-anchors 452/275,
check-ledger 186, check-headers 487/157/17.
