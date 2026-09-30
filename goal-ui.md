# KNXBench goal — UI/UX track (user-reported issues), and nothing else

Written 2026-09-28 against `main` at `99e2a6d`. Use this file as the
instruction passed to `/goal` in the **UI session**. That session runs on
**GPT (Codex)**; this file is written to be agent-neutral. Creating the file
does not start a run.

There are now three goal files, and they do not overlap:

| File | Session | Owns |
|---|---|---|
| `goal.md` | goal.md session (Claude) | everything else: data integrity, product database, import, docs hygiene, manual, alpha, final review |
| `goal-commission.md` | commissioning session (Claude) | T30 phase 3: device writes and their own programming UI |
| **`goal-ui.md`** (this file) | **UI session (GPT/Codex)** | the user-reported UX/UI issues of `goal.md` §11, moved here on 2026-09-28 |

§5 below is the exact boundary. §6 describes how work crosses between
sessions.

## Where things stand (verified 2026-09-28)

- **Source of truth for every task:**
  `docs/superpowers/plans/2026-09-21-user-reported-issues.md` (the "issue
  plan"). Each ISSUE there has files, interfaces and checkboxes.
  - Tick a checkbox only when you close it, and name the test that covers
    it.
  - Its *Global Constraints* and *Review Focus* bind every package here.
- **Ready-made briefs:** `docs/CLOUD_SESSIONS.md` §4, CT-7 to CT-10. They
  are more detailed than the plan for ISSUE-13, -10, -01 and -11. Ignore
  their cloud-only mechanics (draft PR, cloud log name, "leave after the
  PR").
- **Done, do not redo:**
  - ISSUE-04 (dirty state, Save-and-quit, autosave): `d135c5e`, `1742423`.
  - The §4 web residues: CT-1 `313489e`, CT-2 `d9ff0db`, CT-6 `826466a`.
- **ADR-0038** (ISSUE-06, a site is a `Ground` root) is merged but still
  `Status: Proposed`. It has never had an independent review.
- **ADR-0039** (project mutation goes through commands): phases 1 and 2 are
  merged (`43f68a0`, `09beee1`). ISSUE-05 is no longer blocked by it.
- **The web lock is currently held by the commissioning session** for
  goal-commission K5, which it announced at 20:44 on 2026-09-28. See §3.

---

## 0. Scope: what this goal owns

1. **ISSUE-01, 02, 03, 05, 07, 09, 10, 11, 12 and 13** of the issue plan,
   including their server and domain halves (batch create, the "send and
   receive" command, 422 details, export routes, the session-log export).
2. **The UI half of ISSUE-08:**
   - render channel groups collapsed by default, and keep a group the user
     expanded while the device stays selected;
   - distinguish inactive from unsupported data visibly;
   - show the resolved names and DPTs the projection delivers.

   The data half belongs to `goal.md` (§5).
3. **ISSUE-06's remaining step:** an independent review of ADR-0038, then
   the user's acceptance, then `Status: Accepted`.
4. **The File-menu rename** handed over by the commissioning session (rule
   R2, `docs/GLOSSARY.md`):
   - `toolbar.downloadProject` ("Download project" / "Projekt
     herunterladen") saves or exports a file. It must say so ("Save
     project…" / "Export project…").
   - "Download" is reserved for writing to a device.
5. **Docs for these features:** IMPLEMENTATION_STATUS entries,
   KNOWN_LIMITATIONS entries the features lift or add, and the user-manual
   chapter describing the feature you changed. The manual's T23 acceptance
   stays with `goal.md`.

---

## 1. Hard rules

1. **Read first.** Read `AGENTS.md` and follow it. `CLAUDE.md` applies
   where it states project facts, even though you are not Claude.
2. **Nothing reaches a KNX device.**
   - No write of any kind to a real device: no group-value send to the real
     bus, no programming, no restart.
   - All UI tests use the simulator, mocks or fakes.
   - Real multicast discovery for ISSUE-12 is read-only. Individual address
     `1.1.220` (an alarm panel) is never read, written or scanned.
   - The gateway accepts exactly one tunnel. Before any live read, make sure
     no other session's monitor, server or CLI holds it.
3. **Data integrity beats convenience.**
   - Save, autosave and bulk creation must be atomic and honestly reported.
   - Product-data fallbacks stay explicit diagnostics; never guess a name,
     DPT, channel or visibility.
   - Undoable project changes go through `Command`/`CommandStack`
     (ADR-0039). A multi-entity change is one batch command.
4. **One path per behaviour.**
   - Preferences go through the versioned settings document
     (`apps/knx-server/src/settings.rs`, `apps/knx-web/src/settingsStore.ts`),
     never directly into browser storage.
   - Existing discovery, filters and Send/Receive links are extended, not
     re-implemented. The issue plan names which ones already exist.
5. **Keyboard and screen reader.** Every new gesture has a keyboard
   equivalent: resize, zoom, drag and drop, filters, contextual help.
6. **Wording.** Claim nothing beyond "KNX-compatible". Claim no ETS
   compatibility and no certification.

## 2. Operating rules

1. **Start of every work cycle.**
   - `git fetch`, then read the top of `.ai/CURRENT_STATE.md`. The newest
     entry is first, and the web lock lives there (§3).
   - Then read this file and the issue plan's section for the next package.
2. **Isolation.**
   - One worktree per package: `/mnt/daten-i/Sourcecode/KNXBench.worktrees/ui-<topic>`
     on branch `ui-<topic>`, created from the current `origin/main`.
   - Never work in the root checkout `/mnt/daten-i/Sourcecode/KNXBench`, which
     belongs to the goal.md session.
   - A fresh worktree has no `node_modules`: run `npm ci` in
     `apps/knx-web` first.
   - Corpus-backed tests (ISSUE-07/08) need the corpus. Link it:
     `ln -s /mnt/daten-i/Sourcecode/KNXBench/OriginalData OriginalData` in
     the worktree root (the link is gitignored), and set
     `KNXBENCH_PRODUCT_CORPUS` to
     `/mnt/daten-i/Sourcecode/KNXBench/OriginalData/ProductDatabases`.
     Without it those tests take their skip path, which proves nothing.
3. **Handover (the `.ai/` protocol in `AGENTS.md`/`CLAUDE.md`).**
   - Put a new entry at the **top** of `.ai/CURRENT_STATE.md` in your branch:
     `**Last Agent:** Codex (UI session)`, a timestamp from `date`, then
     Completed, Pending/Next Steps, and Notes for Codex or Claude.
   - Never delete or rewrite other sessions' entries.
   - Write a log for each package: `.ai/logs/YYYY-MM-DD_codex_ui-<topic>.md`.
   - `.ai/` is gitignored, although `.ai/CURRENT_STATE.md` and the logs are
     tracked. Stage them with `git add -f`. A plain `git add` refuses them,
     and the commit silently goes without them.
4. **Merging (you merge your own branch).**
   - Immediately before the merge, `git fetch` and rebase onto
     `origin/main`, or merge `origin/main` in.
   - For conflicts in `.ai/CURRENT_STATE.md`, `docs/IMPLEMENTATION_STATUS.md`
     and `docs/KNOWN_LIMITATIONS.md`, keep both sides. CURRENT_STATE entries
     go in chronological order.
   - A new KNOWN_LIMITATIONS number is the next free one at merge time.
   - Push after each reviewed, merged and fully green package. Then delete
     your worktree, your branch, and your scratch and build directories.
5. **Gates (run on the branch, then again on the merged result).**
   - Only one workspace `cargo` gate at a time on this machine, across all
     three sessions. Before `cargo test --workspace` or `cargo clippy`,
     check `pgrep -af cargo`. If another session's gate runs, wait.
   - Use a build directory of your own:
     `CARGO_TARGET_DIR=/mnt/daten-i/Sourcecode/KNXBench.worktrees/.target-ui`.
     Delete it when the track ends.
   - **Required gates:**
     - `cargo fmt --all --check`;
     - `cargo clippy --workspace --all-targets -- -D warnings`;
     - `cargo test --workspace --no-fail-fast`;
     - `cargo run -p xtask -- check-layering`, `check-headers`,
       `check-anchors` and `check-corpus-gates`;
     - `git diff --check`;
     - in `apps/knx-web`: `npx tsc --noEmit` and `npx vitest run`.
   - Judge a gate by its exit status **and** the amount of work in its log
     (tests counted, crates compiled). A wrapper's exit 0 alone is not
     evidence.
   - Every new file needs the licence header (`check-headers` has zero
     slack).
6. **Evidence.**
   - Write a failing test first (RED), then the fix.
   - Check every new guard or validation once by mutation: remove it, and
     the test must fail.
   - Tick the issue plan's checkboxes only with the covering test named.
7. **Review.** No subagents.
   - Before each merge, review your own full branch diff against the issue
     plan's *Review Focus* and `AGENTS.md`, in a separate pass after the
     implementation.
   - The closing review of the whole track is described in U13.
8. **Commits.**
   - Author: `KNXBench <github@knxbench.com>`, for example with
     `git -c user.name="KNXBench" -c user.email=github@knxbench.com commit …`.
   - **Never** add a `Co-Authored-By` trailer or any other co-author line.
   - Messages are concise and describe the change; a little humour is
     welcome.
   - Push normally. Never wait on GitHub Actions.
9. **Quota.** At each package boundary, ask the user how much GPT/Codex
   quota is left. Pause before it would run out. A pause holds until the
   user's explicit "go".
10. **Scratch** goes under
    `~/.hermes/profiles/knxbench/cache/scratch/ui/` (or your own
    tool's scratch). Never touch `scratch/iaw/` (commissioning) or other
    sessions' files.
11. **Do not touch:**
    - `docs/paperclip-shutdown/` (untracked, foreign);
    - `ideas.md` (gitignored, never commit it unchecked);
    - worktrees and branches named `iaw-*`;
    - the root checkout.

## 3. The web lock (all sessions)

`apps/knx-web` is shared: `App.tsx`, `styles.css` and the message catalogues
collide on every task. Only one session edits it at a time.

- **Take it.**
  1. `git fetch` and read the top entries of `.ai/CURRENT_STATE.md`. The
     lock is free if the newest lock line says *released*, or if there is no
     lock line at all.
  2. Commit a handover-only entry **directly to `main`** and push it at
     once. Its first line after the header is
     `Web lock: taken by <session> for <package>`.
- **Release it.** The same line with `released`, in the entry that
  announces your merge.
- **Hold it for one package at a time.** Release it between packages, so
  that the commissioning session (K5/K6) can get in.
- **Right now** the commissioning session holds it for K5. Start with the
  packages that do not edit `apps/knx-web`: U0 to U2 below.

---

## 3a. Work packages, in this order

Every package ends with: tests and mutation check, gates, docs, merge, push,
handover, cleanup. **[web]** means the package needs the web lock.

### U0 — Set up (offline, no lock)

1. Confirm `main` is clean and every gate is green on a fresh worktree.
2. Record the baseline counts in your first handover entry: the number of
   workspace tests and the number of vitest tests.

### U1 — ADR-0038 review (ISSUE-06), no lock

1. Review `docs/adr/0038-site-is-a-ground-root-space.md`, its
   characterisation tests and KNOWN_LIMITATIONS §127 independently. The
   scope is in `goal.md` §12.2 item 5: Project Schema23 §1.1.2.3/§1.2.6.3,
   3/10/3 §1.2.3.5, 3/10/4 Table 10, 3/10/2 Table 1.
2. Report the findings to the user and ask for acceptance. Only after the
   user accepts, change the status to `Accepted`.

### U2 — ISSUE-12 diagnosis, and the ISSUE-09 address-editor research, no lock

1. ISSUE-12: find out why the AppImage discovery finds no gateway while a
   manual connection works. Build the AppImage, capture the multicast
   traffic read-only, and compare it with the dev build.
   - Write the result into IMPLEMENTATION_STATUS/KNOWN_LIMITATIONS.
   - Diagnose first, change second (issue plan). The fix comes in U10.
2. ISSUE-09: check in the KNX documentation whether line membership fixes
   the area and line octets of an individual address.
   - Document it with `[D]`/`[V]`/`[A]` labels.
   - Without evidence, the address editor stays as it is.

### U3 — File-menu rename (R2) [web]

"Download project" becomes Save/Export, in `en.ts`, `de.ts` and
`App.test.tsx`. Change `docs/GLOSSARY.md`'s *Open* note to done. This is
small, which makes it a good first web package.

### U4 — ISSUE-13, session log: search and export [web]

Brief CT-7.

### U5 — ISSUE-10, actionable 422 errors and topic help [web]

Brief CT-8. Keep the bus monitor's existing freetext and service filters.

### U6 — ISSUE-01, zoom and remembered pane widths [web]

Brief CT-9.

### U7 — ISSUE-11, bus monitor: pause, export, decoding, statistics [web]

Brief CT-10. Pausing must not discard buffered telegrams or move the server
cursor wrongly.

### U8 — ISSUE-03, resizable dialogs, readable forms, gear icon [web]

### U9 — ISSUE-02, welcome surface and new-project clarity [web]

### U10 — ISSUE-12 fix: separate host and port fields, and the discovery fix from U2 [web]

### U11 — ISSUE-07 (catalog in the main window, quantity, atomic batch) and ISSUE-09 (readable flags, "send and receive", address editor only if U2 proved it) [web]

These are two packages: ISSUE-07 first, then ISSUE-09.

### U12 — ISSUE-05 structure editing, then the ISSUE-08 UI half [web]

- **ISSUE-05** needs U1 accepted. All gestures call the same validated
  commands as buttons and forms.
- **ISSUE-08 UI half** starts only after the goal.md session has announced
  the ISSUE-08 data half as merged (§6). It uses the projection fields that
  session delivers, and invents no name heuristics.
- **Handed over 2026-09-30 (goal.md session), each needs the web lock:**
  1. **§146 channel labels:** show `channel.name` and `channel.number`
     (ADR-0052, schema v18) verbatim, never composed into `text`, never
     translated. Lifts KNOWN_LIMITATIONS §146 fully.
  2. **ADR-0051 Debug toggle:** a clearly marked "Debug" section in
     Settings with the key `debugIndividualAddressWriteEnable` (default
     off, warning text), and a device action calling
     `GET`/`POST /api/device/service-control` with the scope's own phrase
     `I confirm individual-address write enable to <address>`. The server
     already refuses with 403 while the key is not `true`; the UI must not
     be the only gate.
     **Safety pause (2026-09-30):** a read-only audit of the existing POST
     route and its executor found read/bit-only write/readback but no
     persisted pre-write recovery record. The commissioning handover requests
     a backup guarantee for other UI writes before exposing them. Do not
     expose the setting or action until the commissioning/data owner resolves
     and tests that policy; the route already exists and remains default-off.
     This pause does not block the independent monitor/read-only UI items.
  3. **Monitor control fields (§147 consumers):** `/api/bus/monitor/telegrams`
     rows carry `control: {priority, repeated, hopCount}` (`null` on the
     closed-session marker). Show them as a column or tooltip; `repeated`
     is `null` on anything but `L_Data.ind` and must stay unshown then.
  4. **Readiness and device-compare views:** the APIs exist
     (`/api/readiness` `b72a6b6`, `POST /api/device-compare` `76bcce74`);
     no view shows them yet.

### U13 — Close the UI track

1. The user decides the **closing review of the whole track**. The
   suggestion is a read-only cross-review by the goal.md (Claude) session
   over the track's full diff. Fix its findings.
2. Write a closing handover that lists every ISSUE with its evidence, and
   everything left for `goal.md` (§6).

---

## 4. Completion condition

Finish only when:

- U0 to U12 are done with evidence, or the user has explicitly accepted an
  item out of scope;
- every issue-plan checkbox for the owned issues is ticked with a named test,
  or explicitly marked out of scope with the user's acceptance;
- all gates from §2.5 are green on the merged `main`;
- the closing review has run and its findings are fixed;
- the web lock is released.

Then report: what is done, the evidence, and what is left for the goal.md
session.

---

## 5. Boundaries (no overlap)

| Topic | Owner |
|---|---|
| ISSUE-01, 02, 03, 05, 07, 09, 10, 11, 12, 13, with their server and domain halves; the UI half of ISSUE-08; the ADR-0038 review; the File-menu rename | **this goal** |
| ISSUE-08 data half: trace, classification, language-aware name and DPT resolution, active/visible state and channel ownership in the projection, corpus regression counts | `goal.md` |
| Product database, import, schema, PDB-x | `goal.md` |
| `docs/LIMITATION_TRIAGE.md` recount, `stats.md`, `goal.md`, the manual's T23 acceptance, doc hygiene `goal.md` §8, the alpha decision, the final whole-goal review | `goal.md` |
| Device programming: download, individual address, restart. Their UI (K5 download tab, K6 address dialog), their server routes, the server-side consent decision (ADR-0045) | `goal-commission.md` |
| The ADR-0040 consent hook and dialog | built and closed. The commissioning session calls it; this goal does not change it |
| `apps/knx-web` in general | this goal, under the web lock (§3). The commissioning session takes the lock for its own views only |

If a package turns up something from another column, do not do it here.
Hand it over (§6).

## 6. Handover between sessions

- **To the goal.md session:** in your handover entry, under the heading
  **"For the goal.md session:"**. That session adopts items into `goal.md`
  §12.4 and confirms in its next entry. Typical items:
  - new or renumbered KNOWN_LIMITATIONS entries (for the triage recount);
  - a `stats.md` refresh after each of your merges;
  - findings in product data or import (as findings, not fixes).
- **To the commissioning session:** under **"For the commissioning
  session:"**, for example UI findings in their programming views.
- **From the goal.md session to you:**
  - The ISSUE-08 data half arrives as an entry naming the new projection
    fields and the merge commit.
  - Until then, U12's ISSUE-08 part waits.
