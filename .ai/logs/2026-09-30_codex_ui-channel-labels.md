# UI U12 §146 — channel-label delivery (Codex, 2026-09-30)

## Scope and source

- `goal-ui.md` U12 §146; ADR-0052 (`name`/`number` are optional strings, with non-decimal numbers) and `docs/KNOWN_LIMITATIONS.md` §146.
- Display only existing typed projection fields in `DeviceWorkspace`; no product-database, project, server or hardware mutation.
- Worktree: `ui-channel-labels`; web lock acquired on `main` as `29f98ae2` and read back from remote.

## RED / focused implementation evidence

- New `DeviceWorkspace.test.tsx` regressions failed before implementation: 2 failed / 19 passed (missing source name and number).
- After change: 21/21 focused tests; TypeScript exit 0; local mock Chromium EN/DE × 360/1440 px 10/10; Web build exit 0; full Vitest 78 files / 1,256 passed.
- Removing the `name` fallback temporarily made the focused regression fail (exit 1); original source restored byte-for-byte.

## Separate whole-diff review (before fixes)

- **IMPORTANT** `docs/manual/user-guide/05-devices-and-products.md:162-169`: previous sentence ended in `A`; the new channel explanation produced the dangling `A An evaluated channel`. Fix the grammar rather than ship broken manual text.
- **MINOR** `docs/IMPLEMENTATION_STATUS.md:13`: says blank values are tested but the new test uses only `null`; exercise empty strings explicitly or narrow the claim.
- **MINOR** `apps/knx-web/src/DeviceWorkspace.test.tsx:180`: redundant `Raw name` text-content assertion after the stronger `summary > strong` assertion; remove it.
- Other reviewed paths: source field boundaries, opaque group key and order, metadata as text rather than parsed number, EN/DE localization of labels only, null/independent/unassigned cases, keyboard disclosure and no network bus calls. No other blocking finding.

## Gates and delivery

- Fixed the manual's dangling article, covered empty-string channel fields and removed the redundant assertion. Focused Web test 21/21, TypeScript/build green, full Web 78 files / 1,256 tests, local mock Chromium 10/10, rustfmt and diff check green.
- After review fixes, corpus-backed Rust workspace: 136 suites / 2,761 passed / 0 failed / 160 ignored / 0 `SKIP:`. Strict Clippy and `xtask` check-layering, check-headers, check-anchors, check-corpus-gates exit 0; dedicated target compiled the candidate's crates. No live tunnel or device write.
- Published the reviewed feature `e8a3c56f2a4c35e40c4148c5479c8b94f17282a4` to `origin/main`; readback matched. The publication-equivalent tree passed a second Web build, 78 files / 1,256 tests, 10/10 local mock browser cases, Rust 136 suites / 2,761 passed / 0 failed / 160 ignored / 0 `SKIP:`, Clippy, fmt, diff and all four xtask gates. No live KNX connection or device write. The web lock is released in the closeout handover.
