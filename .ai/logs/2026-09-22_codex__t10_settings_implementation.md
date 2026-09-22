# T10 settings surface implementation

Date: 2026-09-22  
Branch: `t10-settings-surface`  
Base: `9227042` (`main`)

## Delivered

- Typed settings diagnostics cross the settings and session-log APIs and are
  localized by one frontend formatter.
- Pre-hydration settings edits, including deletions, survive GET/adoption and
  are coalesced through the serialized PUT queue.
- One optional `preferredGateway` seeds new Bus Monitor and Line Scan mounts
  without changing an active or user-edited workflow.
- One lossless exclusion module and editor serve Settings and Line Scan;
  invalid legacy values remain visible and block work until removed.
- Settings are grouped as Appearance, Language & data, and Bus & diagnostics.
  Group addresses remain fixed slash notation with no selector.

Product commits:

- `e108458` — typed diagnostics
- `b32b707` — hydration journal
- `54a03f4` — preferred gateway and one-time seeding
- `7746ea3` — shared exclusion editor
- `41772ee` — grouped settings and localized diagnostics

## Rulings and retained boundaries

- `preferredGateway` is unencrypted installation-network metadata included in
  data-directory backups; it is not a credential.
- Settings reads and writes never discover, connect, scan, or otherwise cause
  KNX, multicast, LAN, or hardware traffic.
- Invalid non-blank gateway metadata remains storable; the existing request
  boundary stays the sole endpoint grammar authority.
- Unknown diagnostic tags fall back to the server message. The compatibility
  anchor for the former §122 heading remains because existing docs link it.
- `KNOWN_LIMITATIONS.md` §121 remains open: mounted windows do not receive live
  preference updates from another window.
- No project/entity defaults, server path controls, routing defaults, scan
  timing defaults, generic settings registry, or per-user settings were added.

## Verification

The following commands exited 0 in the branch worktree with
`CARGO_TARGET_DIR=/tmp/knxbench-t10-target`, incremental compilation disabled,
and dev/test debug info disabled:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo run -q -p xtask -- check-layering
cargo run -q -p xtask -- check-headers
cargo run -q -p xtask -- check-anchors
cargo deny check
cd apps/knx-web && npx tsc --noEmit
cd apps/knx-web && npx vitest run
git diff --check
```

Observed totals: Rust 1,948 passed across 92 result blocks with zero failing
blocks; frontend 859 passed across 63 files. Header check reported 194 files
with headers and 162 without, at the ceiling of 162. Anchor check reported 389
links across 178 Markdown files and none dead. Dependency audit completed with
the repository's accepted duplicate-version warnings and exit status 0.

Focused audits found no group-address-notation control, no private-LAN literal
in product additions, and no whitespace errors. All protocol-facing tests used
in-process fakes; no real KNX, multicast, LAN, gateway, or hardware operation
was run.

## Next

Complete the fresh whole-branch review and merged-result gates, then continue
with goal task T11 (bounded native drag-and-drop gestures through existing
validated commands, each with a keyboard equivalent).
