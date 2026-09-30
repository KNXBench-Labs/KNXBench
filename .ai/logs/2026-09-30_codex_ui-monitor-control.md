# U12 / §147 — monitor control-field display (2026-09-30)

## Scope and source contract

- Isolated branch `ui-monitor-control`, worktree `KNXBench.worktrees/ui-monitor-control`, Web lock taken at 18:07 CEST. No real gateway, tunnel, live monitor, device read or write.
- Server `TelegramRowDto::control` in `apps/knx-server/src/bus_routes.rs` already supplies `{ priority, repeated, hopCount }`; it is `null` on a session-closed marker. The source parser in `apps/knx-server/src/bus.rs` supplies `repeated: null` outside `L_Data.ind`. No server or protocol edit was needed.
- Frontend `BusTelegramRow.control` is optional to tolerate an older server omitting the additive field. The table and keyboard-opened details show priority and hop count; true, false and null repeat evidence remain distinct. Unknown priority names are reported as unknown rather than remapped. Absent and null control render a generic no-data mark without asserting a value. Captured JSON serializes the received row unchanged.

## TDD and verification

- Two monitor-unit regressions were RED before rendering, GREEN after. A temporary mutation `repeated != null` → `repeated === true` caused the expected assertion failure for the false state; source restored. The focused monitor, capture and diagnostic style tests passed 56/56 after adjusting the existing horizontal-width guard from 66 to 64 rem and styling new classes.
- Final Web gates: TypeScript diagnostics 0; Vite build exit 0; Vitest 78/78 files, 1,258/1,258 tests. Existing local mocked device-editor Chromium: 10/10. New local mocked monitor Chromium: EN/DE at 360/1440 px, 4/4. The fixture rejects any unrecognized `/api/**` call and made no KNX connection.
- Corpus-backed `cargo test --workspace --no-fail-fast`: 136 suites, 2,761 passed, 0 failed, 160 ignored, zero `SKIP:` lines. Strict Clippy: 151 crates checked, zero errors. `cargo fmt --all -- --check`, `xtask check-layering`, `check-headers` (336 with headers, 161 baseline without), `check-anchors` (403 links across 220 files) and `check-corpus-gates` passed. `git diff --check` passed. Only the task's read-only corpus symlink exists in this isolated worktree.
- Reviewed the full patch and new fixture files against `goal-ui.md` §3a U12 and §2.5; added-line security-pattern scan found no matches. No external subagent used, per goal/UI review rule and user preference.

## Handover

- Documentation updated: `goal-ui.md` U12, `docs/IMPLEMENTATION_STATUS.md`, `docs/KNOWN_LIMITATIONS.md` §147 and `docs/manual/user-guide/07-bus-and-interfaces.md`. §147's UI display gap is closed, without claiming a new live-bus test or full ETS compatibility.
- ADR-0051 Debug toggle/device write remains paused until a cross-route pre-write recovery policy is defined and tested; no part of this package changes that boundary.
- Next UI package after publishing and releasing the lock: U12 readiness and read-only device-compare views. Work in another isolated worktree with a newly acquired Web lock; no production bus activity.
