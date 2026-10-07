# 2026-10-07 — Achievements, package 2 (ADR-0089)

## Scope

The remaining 27 of the 38 achievements approved in the grill-me interview
(the user answered "passt, lets go"). Package 1 (mechanism and 11
achievements) was already on `main` (`5683e133`).

## Built

- **Rules** (`achievementRules.ts`, written red first):
  - `event` gains typed `where` conditions (`eq`, `gte`). The condition
    type is derived from the event union, so a wrong field or value type
    fails `tsc`.
  - New kinds: `steps` (export → re-import), `distinct` (subjects as
    record markers `<id>--<slug>-<fnv1a>`, within the server's id rule)
    and `allOthers`.
  - `threshold` works on any `projectObserved` measure; `localHours` takes
    an optional weekday.
  - The server is unchanged: markers are ordinary progress entries.
- **Measuring** (`achievementObservation.ts`): named group addresses,
  distinct DPT references, rooms at any depth, and devices (on lines plus
  without a line, counted once).
- **18 new event types**, emitted where the server confirmed the outcome.
  The table is in ADR-0089 under "Package 2".
  - Verified commissioning is the server's own statement: a download
    needs `written: "yes"` (every block read back, `device_download.rs`)
    and a restart that was not left `unconfirmed`; address programming
    needs `written: "yes"`.
  - Still no send event.
- **Catalogue:** 27 entries with DE/EN titles invented per language (the
  joke only in the title) and plain descriptions; six new glyphs.

## Reported, not simulated

- **#19** (validation, ≥ 50 devices): there is no project validation in
  the UI. It became `clean-sheet`, an ETS import with at least 50 devices
  whose report has no losses and no notices.
- **#26** (bus diagnosis): there is no bus diagnosis. It became
  `clean-bill`, the offline readiness check where every device can be
  planned.
- **#34** (Friday without writing): "without writing" needs a send
  event, which is forbidden. It became `read-only-friday`, starting the
  bus monitor on a Friday from 15:00.
- **#7** and **#27** were detectable as worded: `ProjectTree.errors`
  counts only genuine losses, and the comparison reports
  `differingOctets`.
- Hidden share: 6 of 38 (16 %), as in the approved catalogue, not the
  25 % from the interview. Noted in KL §164.

## Findings

- **Hidden progress leak (fixed).** The overview would have shown "3 / 10"
  for a locked *hidden* counting achievement. Package 1 had none of those,
  but `error-culture` is one. The test went red first; the fix is
  `!secret`.
- **Diagnostics companion tripwire.** The pinned module graph now contains
  `achievementEvents.ts` (through `BusMonitorPanel`). The reason is
  written next to the list: no imports, no request, and no subscriber in
  that window.
- **The 502 from the package 1 probe is explained.** It is
  `POST /api/bus/discover`, the KNXnet/IP search, which cannot multicast
  in the offline namespace. It is unrelated to achievements.

## Evidence

- Emit-site tests: download (acknowledged and notInPlan count;
  unconfirmed, failed and running do not), address programming (only
  `yes`), readiness (3 of 5 not plannable), comparison (only after the
  consistency checks), error toast (once under StrictMode, not for fun
  toasts), monitor (start, 2 minutes, nothing after disconnect, nothing
  on refusal), CSV (applied vs declined), App (ETS import with counts,
  password flag without the password, `.knxdb` is not an ETS import).
- Mutations: removing either verified condition (download restart,
  address `written`) turns the tests red.
- Real app: debug `knx-server`, fresh data directory,
  Chromium in an offline namespace, the synthetic sample house imported
  through the UI. The server reported errors=0, warnings=0, 8 devices,
  15 GAs (15 named), 4 DPTs and 4 rooms. `achievements.json` then held
  exactly `lossless-move` and progress 15/15/4/4, and no wrong unlocks
  (archaeologist, clean-sheet). Popup and overview were inspected
  visually.

## Gates

- **Gate 1** (pre-rebase candidate on `4c4f3b05`): build, tsc, flow-study,
  theme fixtures and Vitest (2286) pass; fmt, workspace Clippy (362
  crates fresh), knx-server 699/0/45 and five xtask gates pass.
  - Chromium red in all 157 tests, for a reason in the gate script, not
    the code: `TMPDIR` (`…/ach2/gtmp<pid>`) made Chromium's singleton
    socket path exceed the Unix limit ("Socket path too long").
- **Rebase** onto `e99a94e9` (Boarisch/Klingon language packs).
  - The only conflict was IMPLEMENTATION_STATUS: both entries kept,
    newest first, no upstream line lost.
  - The companion pin list and the Settings emit sites merged cleanly.
- **Gate 2** (rebased HEAD `0dd3f996`, short `TMPDIR`):
  - build, tsc, flow-study, theme fixtures: pass; Vitest 137 files /
    2317 tests;
  - Chromium offline 157/157, `achievements.e2e.ts` 3× 6/6;
  - fmt and workspace Clippy pass (target reused; no `.rs`/`Cargo` file
    differs from `4c4f3b05`); knx-server 699/0/45;
  - five xtask gates and `diff --check` pass; `inputs_frozen=1`.
