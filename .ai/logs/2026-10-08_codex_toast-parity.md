# Standard/achievement toast parity — local implementation

Timestamp: 2026-10-08 22:45 CEST
Agent: codex
Branch: `feature/toast-parity-20261008`
Base: `115b19f65eeebf7df21598681b34e829d2055099`
Delivery: local, uncommitted and unpublished; owned worktree retained.

## Request and implementation

The owner requested standard toasts to behave visually and in duration like
achievement popups, explicitly in a separate worktree. The old queue used
six seconds/immediate removal for status messages and persistent errors.
The existing achievement view/queue is reused: nine-second defaults,
shared card/typography/animations, exit completion and fallback cleanup.
Errors retain their semantic red accent, alert and server-language notice;
achievement badges/labels remain achievement-only. No application-domain,
server, storage, protocol, dependency or achievement-rule changes.

## Verification

- RED/GREEN: standard lifetime, error lifetime and message typography.
- Browser presentation comparisons rejected the old CSS; long strings and
  standard exit lifecycle covered. Harness corrections, not production fixes:
  pause the fake clock for exact boundaries; use the real motion setting and
  distinguish zero-duration Motion Off from OS-disabled animation.
- Full Vitest: 2422 passed / 0 failed / 0 pending, 149 files.
- Full Chromium: 189 passed, isolated loopback namespace/intercepted APIs.
- Toast/achievement specs repeated three times: 54 passed.
- Production build and theme/flow/toast fixture types pass.
- All five fresh-target xtask checks and whitespace pass; all five documentation-only
  closure checks also pass after the manual/status/ADR/handover updates.
- Production workbench screenshots inspected: Graphite EN/1440 and Porcelain
  DE/400; matching frames/stripes, legible wrapping and visible close controls.
  No external/unexpected request or browser exception in these captures.
- In-session self-review: no blocking finding; no independent review claimed.
- Compact source-bound evidence: `docs/evidence/toast-parity-2026-10-08.json`.

## Boundaries and handoff

No commit/push/root sync/deployment or live bus operation authorized or performed.
No Rust workspace, private corpus, native AppImage or assistive-technology rerun.
Root WIP and other active worktrees remain untouched by this task. App.tsx and
message catalogues are unchanged; eventual integration may overlap styles.css,
status/manual documents and handover with the Devices-navigation package.
Task-owned scratch/gate artifacts and browser outputs were cleaned after closure;
local implementation, worktree, build and compact evidence remain for owner inspection.
