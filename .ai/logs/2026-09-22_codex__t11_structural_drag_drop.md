# T11 structural drag-and-drop implementation

Date: 2026-09-22
Branch: `t11-structural-drag-drop`
Base: `9723316`

## Delivered scope

- Device → topology line and device → building part use native HTML drag/drop.
- Both gestures call the existing `moveDeviceToLine` or
  `moveDeviceToBuildingPart` API; no optimistic projection edit exists.
- Only first-installation line/unassigned devices are sources, and only
  first-installation lines/building parts are targets.
- The private MIME payload accepts only a complete safe positive decimal id,
  must match the active drag source, and is revalidated against the current
  tree at drop time.
- Success uses the existing localized status toast; rejection uses the
  existing alert path. Inspector selects remain keyboard equivalents.
- Drag feedback is workbench-scoped, theme-token based, and motion-free.
- Group address → communication object is deliberately absent because
  `LinkComObject` requires an explicit `Send`/`Receive` direction.

## TDD evidence

- Source eligibility/payload: 2 expected failures, then 7 focused passes.
- Line outcomes: 2 expected failures, then 4 focused passes.
- Building-part outcomes: 2 expected failures, then 3 focused passes.
- CSS feedback guard: 1 expected missing-selector failure, then pass.
- App live-region and Inspector keyboard tests exercised already-existing
  production wiring and therefore passed on their first valid run; the ledger
  records both rulings rather than manufacturing failures.

## Verification

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo test --workspace --no-fail-fast`: 1,948 passed, 5 ignored across
  92 result blocks.
- `cargo run -q -p xtask -- check-layering`: pass.
- `cargo run -q -p xtask -- check-headers`: 194 with headers, 162 without,
  ceiling 162; 30 generated skipped.
- `cargo run -q -p xtask -- check-anchors`: 389 links across 180 Markdown
  files, none dead.
- `cargo deny check`: pass.
- `npx tsc --noEmit`: pass.
- `npx vitest run`: 881 tests across 63 files, all pass.
- Focused audit found no `LinkComObject` explorer drag, forbidden installation
  or RFC-1918 address addition, or whitespace error.

The first two Rust attempts under `/tmp/knxbench-t11-target` hit the user's
disk quota during linking; that task-owned target was removed each time. The
successful clean run used `/var/tmp/knxbench-t11-target` with two build jobs.
Unrelated old build caches were not touched.

No KNX, multicast, LAN, gateway, or hardware operation was performed. All
changes are UI projection/event wiring, tests, styles, and documentation.

## Commits before documentation

- `7778c92` — device → line.
- `6e60113` — device → building part.
- `202fa50` — live-region, keyboard-parity, and static feedback coverage.

Fresh whole-branch review and merged-result gates remain the next mandatory
steps.
