# UI U12 ADR-0051 safety preflight (Codex, 2026-09-30)

## Read-only evidence

- `docs/adr/0051-individual-address-write-enable-is-opt-in-debug.md`: accepted decision for a default-off server setting, own per-device phrase, bit-2-only read/modify/write/readback; hardware action has never been tested live.
- `apps/knx-server/src/service_control_routes.rs:64-87,227-260`: the GET/POST setting gate is server-side. POST parses target/gateway, checks `WriteScope::IndividualAddressWriteEnable`, then connects the tunnel and invokes the executor.
- `crates/knx-net/src/commissioning/service_control.rs:183-220`: reads the two-byte value, changes bit 2, writes, checks the readback. It returns `before` on success. Neither this function nor the route persists that before-value or a recovery plan before the write; a process interruption or error after sending can leave the outcome ambiguous.
- Concurrent commissioning handover `.ai/logs/2026-09-30_claude_commissioning-ui-status-handover.md` §4–5 requests auditing pre-write backup for Bit 2 and other UI writes, withholding a UI affordance when the recovery guarantee is not established. This is a safety-review request, not an accepted new ADR; the suitable recovery granularity for this property is still undecided.

## Decision at UI boundary

No frontend setting or device action was added. The existing server route remains default-off. A Settings toggle alone would open the POST endpoint for any HTTP client, so shipping only the toggle would not be a read-only compromise. Do not conflate this pause with changing the accepted ADR or with a claim that a full memory backup is universally required.

**For the commissioning session:** decide and test the pre-write recovery contract for this specific property (including ambiguous outcome and persistent evidence) before this UI package exposes the setting and write action. Hand back an implementable API/safety contract. This is independent of the K13 reset workflow and does not authorize any hardware access.

**For the UI session:** once the server-side safety question is resolved, add the clearly labelled Debug toggle and a target-specific action with its own phrase; only local mocks/simulator in tests. Meanwhile continue the independent monitor control fields and read-only readiness/compare views in goal-ui.md order.

## Verification

Documentation-only package: worktree-built `xtask check-anchors` exited 0 (44 log lines, nonempty link scan) and `git diff --check` exited 0. No application code was edited and no test or live KNX connection was started.
