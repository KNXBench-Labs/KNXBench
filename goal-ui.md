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

## Where things stand (2026-10-01)

U0–U12's UI slices are delivered: host/port discovery fields, catalog and
device/structure editors, channel labels, monitor control, read-only device
checks, Site/Property creation and the ADR-0051 Debug property action.
The Debug route remains default-off and its durable backup is *property-only*;
no new live device check or whole-image recovery follows. K6 confirmed public
address writes now refuse before a tunnel without device-specific durable
recovery (ADR-0059); the Web tab shows this availability rather than asking
for consent prematurely. CLI/HTTP discovery succeeded after the user's
firewall rule; native WebKitGTK Search remains unverified. ISSUE-04's five
acceptance rows have since been verified and ticked. Only ISSUE-12's two
discovery-evidence boxes and U13's **independent read-only review** remain
open on this UI track. The chosen Claude review was attempted but refused by
its service quota on 2026-10-01; that is **not a review verdict**. See
`.ai/logs/2026-10-01_codex_ui-u13-review-blocked.md`. `docs/IMPLEMENTATION_STATUS.md` and the current top of
`.ai/CURRENT_STATE.md` take precedence over this dated snapshot.

---

## 0. Scope: what this goal owns

Remaining work: reconcile ISSUE-12's two acceptance boxes against the actual
host-firewall/discovery evidence without inventing a wire capture or a full
loopback roundtrip, then obtain the chosen independent U13 review and fix
its findings. Native WebKitGTK and real screen-reader checks are verification
gaps, not permission for a KNX device write. Completed ISSUE-01–11/13 slices
and the accepted ADR-0038 are evidenced in
[the issue plan](docs/superpowers/plans/2026-09-21-user-reported-issues.md)
and [IMPLEMENTATION_STATUS](docs/IMPLEMENTATION_STATUS.md); they are not
implementation tasks here. Product/import data belongs to `goal.md`, live
programming and its safety gate to `goal-commission.md`.

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
9. **Quota.** The user monitors quota directly; do not stop at a package
   boundary to ask for a quota reading. A user-requested pause still holds
   until their explicit "go".
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
- **Current holder:** read the newest entry of `.ai/CURRENT_STATE.md` before
  touching `apps/knx-web`. A holder written in this plan is stale as soon as
  another package merges. U13's read-only review needs no Web lock.

---

## 3a. Work packages, in this order

Every package ends with: tests and mutation check, gates, docs, merge, push,
handover, cleanup. **[web]** means the package needs the web lock.

### Completed packages (U0–U12)

U0–U9, U10's host/port UI, U11's catalog/device editor and the bounded U12
surfaces are delivered;
the acceptance tests are in the
[issue plan](docs/superpowers/plans/2026-09-21-user-reported-issues.md) and
[implementation log](docs/IMPLEMENTATION_STATUS.md). The gateway's reply
was initially blocked by the host firewall; after the user's rule change,
CLI and HTTP discovery succeeded (RESEARCH §20.1). Native WebKitGTK Search
remains unverified. Do not redispatch the completed packages.

### U12 — Delivered within verified boundaries [web]

ISSUE-05 structure editing, ISSUE-08 grouping, §146 labels, §147 monitor
control, read-only **Device checks**, ADR-0051 Debug property UI and
ADR-0038 Site/Property actions are published. `GET /api/device-readiness`
is offline; `POST /api/device-compare` uses a confirmed read-only tunnel
and does not prove a later write. The Debug server gate defaults off and
records only the original property octets; it cannot restore an entire
device. The K6 address tab now checks the independent server recovery
precondition and fails closed before consent when unavailable (ADR-0059).
No new hardware run or full ETS site/project evidence was asserted by U12.
The global activity/status bar, partial-scope selector and K13 reset UI
remain commissioning follow-ups with their own safe contracts and Web lock,
not unfinished U12 checkboxes. See the implementation log and commissioning
handover.

### U13 — Close the UI track

The user chose a read-only independent review by the goal.md/Claude session.
An attempt on 2026-10-01 was refused by that service's weekly limit before
any review; no verdict exists. Do not replace this with a self-review or
mark the track complete. The brief is in
`.ai/logs/2026-10-01_codex_ui-u13-review-brief.md`. Once review access
returns (or the user explicitly chooses another independent reviewer):

1. Reconcile ISSUE-12's two open discovery boxes against RESEARCH §20.1,
   host-firewall fix and the actual offline test scope. No wire capture or
   full loopback multicast test was performed; do not tick on inference.
2. Obtain the independent verdict, fix findings and rerun the integration
   gates. Close with per-ISSUE evidence and a handover to `goal.md` (§6).

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
  §12.3 and confirms in its next entry. Typical items:
  - new or renumbered KNOWN_LIMITATIONS entries (for the triage recount);
  - a `stats.md` refresh after each of your merges;
  - findings in product data or import (as findings, not fixes).
- **To the commissioning session:** under **"For the commissioning
  session:"**, for example UI findings in their programming views.
- **From the goal.md session to you:** the chosen U13 independent review
  returns concrete findings and an actual verdict. A refused review request
  is not a verdict; do not close U13 until findings and gates are settled.
