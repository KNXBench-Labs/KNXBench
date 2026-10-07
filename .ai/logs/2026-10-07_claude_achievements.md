# 2026-10-07 — Achievements, package 1 (ADR-0089)

## Request and decisions

The user wanted Steam-like achievements (30 to 40) to bring some fun into
routine engineering work. A grill-me interview settled the following:

- **Audience:** every user, on by default. Off means off.
- **Content:** fun and serious themes mixed. Bus and commissioning count
  only as verified results, never volume.
- **Storage:** a separate `achievements.json`, never in a project.
- **Detection:** in the frontend.
- **Presentation:** a Steam popup plus an overview. No sound.
- **Rarity:** no telemetry; tiers instead of rarity.
- **Mix:** about 25 % hidden and 20 % with progress.
- **Retroactivity:** achievements based on project state look at the open
  project; action-based ones count from now on.
- **Names:** per language.
- **Delivery:** two packages, 38 achievements in total (catalogue in the
  interview).
- **Branch:** `feat/achievements` in its own worktree.

## What was built

- **Server** (`apps/knx-server`):
  - `achievements.rs`: a grow-only record with merge (earliest unlock,
    highest counter), quarantine, refusal of newer files, a reset that
    moves the file aside, and size/format limits.
  - `achievement_routes.rs`: `GET /api/achievements`, `POST .../record`,
    `POST .../reset`, behind the guard, under `achievements_lock`.
  - `data_file.rs`: the atomic write and the move-aside step, extracted
    from `settings.rs` (pure refactor; settings tests green).
- **Web** (`apps/knx-web/src`):
  - catalogue, rules, tracker (with admission of the server's answer),
    event channel, preference, hook, Konami matcher;
  - the `achievement` toast kind, the overview dialog and the settings
    section;
  - emit sites in `App.tsx` (project created/opened/saved, autosave, undo,
    project observed), `CommandPalette`, `OnboardingGuide` and
    `SettingsPanel` (theme, UI language);
  - a File menu entry (hidden while off) and the palette command
    `open-achievements`.
- **11 achievements:** welcome-site, foundation, palette-pro, dark-side,
  polyglot, seatbelt, time-traveller, mega-site, night-shift (H),
  christmas-elf (H), konami (H, legendary).

## Findings along the way

- **Palette bug, found in the real browser.** Enter in the command palette
  was not consumed. A command that opens a dialog whose first focusable
  control is a button (the achievements overview: Close) got the same
  Enter's activation in Chromium, so the dialog closed as it opened. The
  introduction escaped only because it mounts later. Fix:
  `preventDefault()` in `CommandPalette.tsx`. Regression tests: a Vitest
  test pins `defaultPrevented`, and an e2e spec opens the overview through
  the palette. Both fail with the fix reverted.
- **Tests that reached the network.** Under happy-dom, `fetch` in
  `App.test.tsx` reached whatever listens on `localhost:3000` (a foreign
  process on this host answered 200). This was already true for the
  Settings panel's `/api/settings` reads. `App.test.tsx` now stubs `fetch`
  (refused, like an unreachable server) and fails on any URL other than
  the two known self-fetches.
- **Admission.** The tracker now checks the server's answer
  (`admitAchievementsResponse`). A foreign 200 makes it unavailable
  instead of "ready".
- **Dialog UX.** Initial focus on Close scrolled the list to the end. Now
  only the list scrolls. The reset button in Settings is not a stretched
  primary button.
- **Fixtures.** Four e2e specs that mount the whole app answer the new
  route through `e2e/achievements-fixture.ts`. Their strict
  "unexpected request" ledgers stay strict.

## Evidence

- Review: self-review in session (no subagents by user rule). Findings F1
  to F3 fixed with regressions. F4 and F5 accepted as minor (Konami
  listener stays installed while off but `report` ignores it; the language
  switch inside the introduction does not count).
- Visual check in a real server and Chromium in an offline namespace,
  with only `lo`, `XDG_DATA_HOME` in scratch and a fresh data directory:
  - welcome-site and konami popups appear;
  - the palette opens the overview, and it stays open;
  - `achievements.json` on disk matches the unlocks;
  - Porcelain, Graphite and Retro-Green CRT were inspected;
  - a theme change through the API does not count as "dark-side".
- Mutation checks:
  - the tracker's retry guard: two tests go red;
  - the palette's `preventDefault`: both the Vitest test and the e2e spec
    go red.
- **Gate 1** (pre-rebase candidate `ed171ed2`, fresh target directory,
  shared gate leases held):
  - npm build, tsc, flow-study and theme-fixtures: pass;
  - Vitest 133 files / 2243 tests;
  - Chromium (offline namespace) 157 passed, `e2e/achievements.e2e.ts`
    repeated 3× 6/6;
  - fmt, workspace Clippy `-D warnings` (362 crates compiled fresh) and
    knx-server 688/0/45: pass;
  - five xtask gates and `diff --check`: pass.
  - `inputs_frozen=0` is a docs-only delta made during the run (manual
    palette count, File-menu line). Rechecked in gate 2.
- **Rebase** onto `origin/main` `f822e86c`, which brought upstream's
  SIGTERM package into `knx-server` and KL §163.
  - My limitation was renumbered to §164.
  - Conflicts in IMPLEMENTATION_STATUS (newest first, both kept) and
    KNOWN_LIMITATIONS (§163 then §164). No upstream line was lost
    (checked line by line).
- **Gate 2** (rebased HEAD `5683e133`, fresh target directory):
  - tsc, flow-study and theme-fixtures: pass; Vitest 2243;
  - fmt, workspace Clippy, knx-server 699/0/45 (includes upstream's
    signal tests): pass;
  - five xtask gates and `diff --check`: pass; `inputs_frozen=1`.
  - Chromium carried over from gate 1, because no file under
    `apps/knx-web` differs between the two candidates (upstream changed
    none).
- **Manual screenshots** regenerated with a release `knx-server` in an
  offline namespace.
  - This first needed `KNX_TLS=off` for the login instance (separate
    commit; the breakage came from ADR-0088).
  - `porcelain-command-palette.png` and `porcelain-file-menu.png` were
    replaced: they now show "Achievements…".
  - The others differ only by the footer version (alpha.4 → alpha.5),
    random loading flavour text or content that is not mine. They were
    left unchanged. No achievement popup appears in any screenshot.
