# 2026-10-12 — Claude — GA members + download preparation (Package B)

Session 9006b65eb121, worktree `KNXBench.worktrees/ga-members-download-9006b65e`,
branch `feature/ga-members-download-9006b65e`, base `origin/main` c634f82b
(contains 3170b556 and Package A 333b2abd); rebased onto the notes package 114b0097
before the final gate (expected conflict only in docs/manual/implementation-status.md, kept both).

## Scope (from the owner's /goal)

1. Group-address members: show, unlink (existing), add device → object → direction
   in the GA Inspector and all four views.
2. Gateway selection in "Download to device" from the existing start-up discovery.
3. Devices → "Download to device" hand-off by stable device id.
4. Five-step guide with prerequisites derived from implemented checks.

## Findings worth keeping

- The brief said `Command::LinkComObject` validates only against `installations[0]`.
  Current core (command.rs) applies MODEL-01/ADR-0070 instead: a device placed in one
  installation links only to that installation's addresses; unplaced devices may
  link anywhere; double placement and addresses in several installations are
  refused. The stale premise came from an outdated comment in `Inspector.tsx`
  (`NewGroupLinkRow`), now corrected. The UI mirrors the real rule via the existing
  `linkableGroupAddresses` (moved to `groupAddressMembers.ts`, Inspector imports it).
- The native store refuses to save a device that no line or unassigned list places
  ("no installation to be saved under"), so the "placed nowhere" link case exists
  only in unsaved working state.
- `GET /api/bus/monitor/telegrams` is read whenever the bus workspace mounts (the
  monitor tab); without a session the server answers 404. Browser fixtures for the
  bus workspace must answer it.
- Gateway changes previously did not discard a shown plan; the brief lists gateway
  among plan-discarding changes, so a gateway change now drops the plan (the server
  plan itself is gateway-independent; documented in KNOWN_LIMITATIONS).

## Design

- `groupAddressMembers.ts` (pure): link-rule mirror, candidates, duplicates (Both is a
  duplicate when either half exists), draft identity = server/project incarnation + GA.
- `GroupAddressMemberAdd.tsx`: explicit form; object reads reuse
  `useDeviceObjectInventory` (generalised from the GA6 matrix inventory hook, matrix
  behaviour unchanged); pending guard + generation like `RenameNameField`; a late
  reply is still handed to App (which accepts only same-lifetime newer snapshots) but
  never touches another address's/project's draft.
- `GroupAddressLinks.tsx`: shared member panel (table in the workspace, compact list in
  Properties) + add form, or the existing "exactly one installation" hint.
- `downloadGateway.ts`, `downloadHandoff.ts`, `downloadGuide.ts` (pure); panel keyed
  by device id; hand-off waits for a status read, refuses busy/connecting/unknown
  (one re-read first), stale context, missing or unaddressed devices.
- `OpenDownloadButton.tsx` + `DownloadNavigationProvider` (workbench only);
  `BusDiagnosticsPanel` consumes each request once (nonce), switches to the tab.

## Evidence (frozen candidate 47a604ba)

- Vitest 2983/0 in 198 files; tsc for every tsconfig; build.
- Production-preview Chromium `playwright.ga-members-download.config.ts` 19/19 in its own
  network namespace under the shared browser-fixture flock; 41 screenshots inspected
  (EN/DE, porcelain/graphite, 1440/400). Visual fixes found that way: blocked guide steps
  repeated the open step's hint (now hints only on the open step); compact members list
  sat flush against the address's own Delete (spacing).
- Full fixture suite 368/4: the four are `device-editor-layout.e2e.ts` (DPST-9-1), identical
  on untouched origin/main b9e72d54 — pre-existing, DPT presentation owner.
- HTTP: `http_group_members` 2/0 (undo per action incl. Both, refusals change nothing,
  native save/reopen keeps new + imported links), `http_edit_routes` 26/0; Clippy
  `knx-server --tests` and fmt clean.
- 14 named mutants killed (receipt `ga9006-mutants.log` in scratch), sources restored.
- xtask layering/headers/anchors/ledger/corpus-gates and docs check pass.

## Pitfalls hit (for the skill)

- A first exploratory full-suite run found `e2e/theme-state-fixture.tsx` still rendering
  `GroupAddressTable` without the new required `tree` prop; `tsconfig.theme-fixtures.json`
  would have caught it. After changing a component's required props, type-check every
  `tsconfig.*.json`, not just the app config.
- Run browser suites the repository way: `flock …/knxbench-browser-fixture.lock unshare -rn`;
  a bare run competes with another session's gate and produces flaky timeouts.
- Changing message copy while a dev-server suite runs makes spec and page load different
  catalogue versions — edit only between runs.

## Coordination

The web lock was taken by claude-web-integration (package 1, notes host) at 13:19,
after this package had started from c634f82b; that session noted the overlap and set
the rule "whoever lands second merges fresh main, keeps both sides, reruns the full
gate". This branch was gated on top of their 114b0097. The notes package landed at
14:17 and the same session took the lock for package 2 at 14:18; by user decision
(land once notes is on main, even under the package-2 lock) the branch was rebased
docs-only onto 31e817b9 and published with code identical to the gated 47a604ba.
