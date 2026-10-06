# Manual acceptance checklist (AR16)

Dated record of how the user manual (`docs/manual/`) was checked against the
finished application for the Alpha. Source of the task: `RELEASE-03`,
[alpha-release-goal AR16](../alpha-release-goal.md).

## Policy (user decision, 2026-10-06)

- The manual stays in the repository and is read on GitHub. It is not bundled
  into the application; the in-app help stays the short F1 panel
  ([ADR-0024](adr/0024-in-application-help.md)).
- The manual has screenshots. They show the finished application at its tested
  scope and use fictional data only.

## How the screenshots are made

All 21 pictures in `docs/assets/screenshots/` were regenerated on 2026-10-06
from the real application: release `knx-server` and production frontend at
`origin/main` `f7e98896`, Chromium, 1440×900 at 2× scale, Porcelain theme,
English interface, in a loopback-only network namespace. The project is the
fictional "Sample house" from `tools/manual_sample_project.py` (manufacturer
`M-7FF0` "Example Devices (fictional)", 8 devices, 15 group addresses, 39
communication objects); three unit tests pin its content and determinism. The
run is `apps/knx-web/e2e/manual-screenshots.shots.ts` with
`playwright.manual.config.ts`; the regeneration recipe is in
[Contributing](manual/development/01-contributing.md#screenshots-in-this-manual).

Not shown, by design: any live KNX bus, gateway or device (the bus-monitor
picture shows the offline search result), the desktop shell's native dialogs,
and any theme other than Porcelain.

## Checklist — 2026-10-06

| # | Check | Result |
| --- | --- | --- |
| 1 | UI owner's closure | **Open.** `goal-ui.md`: U0–U13 done, U14–U18 delivered, U19–U21 closed with AR21's acceptance (`4459e310`). At 06:39 the owner's closing review reopened two Web residues (`UI-04` live activity, the `KL-61` binding wording) and took the Web lock; the closure receipt follows them. Named verification gaps: native WebKitGTK and real screen-reader checks. |
| 2 | Location and screenshot policy | Recorded above. |
| 3 | Every screenshot reference (26 places in README and 10 manual chapters) | Picture replaced; each alt text and its surrounding sentence re-read against the new picture and corrected. |
| 4 | Claims found stale while doing 3 | Fixed: the bus monitor's button is **Search**, not "Discover gateways" (7 places); the start-up gateway search is now stated; the welcome screen has three cards (New project…, Open KNXBench project, Import ETS project); help has eleven topics; the catalog row shows `name (number) — description`; the Parameters example has no modules. |
| 5 | Counts re-measured in the run | Command palette: 13 commands with the Ctrl+Z, Ctrl+Shift+Z, Ctrl+K and F1 hints. Help: 11 topics. |
| 6 | Remaining chapters, claim by claim | **Open.** Chapters without a screenshot (KNX basics, configuration workflow, reports and diff, command line, reference tables, FAQ) still need a line-by-line pass against the application. AR15 already corrected the download, schema and version statements in them. |

## Acceptance

Not yet accepted. Rows 2–5 are done; rows 1 and 6 are open. Acceptance needs row 6
and stays with the release owner at AR18, and with the user for anything that
remains an exception at AR19.
