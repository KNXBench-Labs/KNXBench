# Group-write activity boundary — read-only audit

- Agent: codex
- Date: 2026-09-30 23:44 CEST
- Scope: commissioning activity follow-up; no route, UI, protocol or hardware changes.

## Verified in repository

- `apps/knx-server/src/bus_routes.rs:908-990`: `POST /api/bus/write` resolves a group address/DPT, encodes a value, awaits `BusSession::send`, then returns an echo decoded from those encoded bytes. That echo is not a receiver readback.
- `apps/knx-server/src/bus.rs:1412-1421`: the session forwards the send to the held tunnel and returns the tunnel result. This path does not establish which physical devices received a group telegram or which storage areas they may affect.
- `apps/knx-server/src/one_shot_activity.rs:115-128,205-226`: `WriteGuard::mark_send_possible` has a documented precondition: the exact pre-write backup must already be durably persisted and read back. The method sets `backupRecorded` and `sendPossible` itself; it does not independently inspect a backup. Its `verified` state cannot be used as a generic transport receipt. The group route has no such backup or receiver-effect witness.
- `goal-commission.md` §5 assigns group-value sends to the `goal.md` session and says this track must hand off findings rather than modify that area. The existing `GET /api/bus/activity` therefore correctly remains partial with `groupWrite` untracked.

## For the goal.md session:

Please decide and document the group-write backup/recovery policy and the route's honest delivery-evidence contract **before** adding group-write activity or changing the send route. A successful `send` or HTTP 200 must not mean receiver effect verified; a failed/cancelled send may have an unknown effect. Never set `backupRecorded`/`sendPossible` by calling the existing write guard without a real verified durable backup. The group address is not a single physical target, so do not invent one or infer complete affected-storage coverage from the address alone. Leave `groupWrite` explicitly untracked until the owner can supply the required proof; maintain the existing live-write safety boundary. No bus access or write was performed in this audit.

## Next commissioning-owned step

Investigate whether K13's address-reset procedure can establish a complete, durable, device-specific pre-write recovery record without guessed keys or undocumented storage assumptions. Do not enable the HTTP reset or perform a live write on this audit's evidence. The serial-address CLI/HTTP fail-closed gate and the parallel Web lock remain in force.
