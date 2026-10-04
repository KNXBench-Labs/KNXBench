# KNXBench goal — everything still open, minus commissioning

**Execution routing (2026-10-01, AR00):** the user has started
[`alpha-release-goal.md`](../../alpha-release-goal.md). Its AR queue is the sole
executor for overlapping non-UI/non-commissioning work; this file is retained
as historical scope and decision evidence, not a parallel dispatch queue.
Current per-ID evidence and unresolved decisions are in
[`docs/ALPHA_READINESS.md`](../ALPHA_READINESS.md).

Current open work outside commissioning and the UI-owned track. The old
Paperclip takeover, T01–T17 and PDB-1–PDB-11 have been completed or retired;
§12 names the remaining acceptance and decision points. Consult current
source/tests, `docs/IMPLEMENTATION_STATUS.md` and the top of
`.ai/CURRENT_STATE.md` before acting; this is not a snapshot of `main`.
The dated history is available in Git and the implementation log. No alpha
release tag is authorized.

---

## 0. What this goal deliberately excludes

This file excludes commissioning and every real-device write; those are
owned by [`goal-commission.md`](../../goal-commission.md). UI work and the Web lock
are owned by [`goal-ui.md`](../../goal-ui.md). See §12.3 for current handover.

Commissioning has a narrow live-verified `070nh` path on one MDT device,
not a general device-support claim. No approval from that track transfers to
this one. This goal does not open a socket that writes, does not "prepare" a
live run under another name, and does not alter commissioning-owned
limitations. In particular `1.1.220` is an alarm panel: never read, write or
scan it. Historic read permissions on `1.1.24`–`1.1.32` were operation-specific,
not blanket permission for new experiments.


---

## 1. Operating rules

1. Start every work cycle by reading `docs/IMPLEMENTATION_STATUS.md`,
   `docs/KNOWN_LIMITATIONS.md`, `docs/LIMITATION_TRIAGE.md`,
   `docs/ROADMAP.md`, `docs/GAP_ANALYSIS_ETS.md` and the source of truth for
   the item at hand. Re-measure counts rather than quoting them; several have
   drifted and were only caught by counting again.
2. Maintain the repository's `.ai/` handover: read the newest entry first,
   append an accurate newest-first state entry at the end of a task and log
   significant blocks. A dated entry is evidence, not a permanent work lock.
3. Work from the highest-risk correctness, data-integrity, compatibility or
   user-visible gap downward. Prefer a small coherent vertical slice over
   broad speculative work.
4. Follow `AGENTS.md` and `CLAUDE.md` exactly. Respect architecture
   boundaries; keep external-format handling lossless and honestly reported.
5. Create an isolated git worktree before implementing anything
   (`/mnt/daten-i/Sourcecode/KNXBench.worktrees/<branch>`). Never work in the
   main checkout. When a session is stopped, check its worktrees for orphaned
   subagent edits before committing there.
6. For every functional change add focused regression coverage. Run the full
   gate set before declaring an item complete, and judge by exit status, never
   by a summary line:
   `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace --no-fail-fast`, `cargo run -p xtask -- check-layering`,
   `cargo run -p xtask -- check-headers`, `cargo run -p xtask -- check-anchors`,
   `cargo deny check`, plus `npx tsc --noEmit` and `npx vitest run` in
   `apps/knx-web` when any file under it was touched.
   `ABSENT_CEILING` is 162 with zero slack (verified 2026-09-23) — a new headerless file fails the
   gate. Markdown is not counted at all.
7. **Freshness check.** The former `ntfs3` mount once served a stale binary
   from a current fingerprint (documented in the dated implementation log).
   The working copy
   is on ext4 since 2026-09-28, so that hazard is lifted, but a green gate is
   still not proof on its own: confirm the log shows the changed crate being
   compiled (a shared target directory can replay a cached result), and settle
   a disputed result with a fresh `CARGO_TARGET_DIR`.
8. Do not silently downgrade a limitation. If an item depends on unavailable
   samples, specifications, credentials, hardware or a user decision, record
   the exact blocker and move to another actionable item.
9. Do not claim KNX certification or full ETS compatibility. Say
   "KNX-compatible".
10. Commit messages in Marvin's gloomy register — accurate facts, resigned
    tone, no exact clone. **No `Co-Authored-By` trailer, ever**, regardless of
    what any session directive says. Author is `github@knxbench.com`.
11. Push normally. Never wait on or gate a merge behind GitHub Actions.
12. A sweep for one known literal is not a sweep. Grep by pattern class (RFC
    1918 ranges, not one remembered address). The repository's history was
    already rewritten twice for this; do not put it back.
13. The user monitors quota directly (2026-09-29); do not pause at package
    boundaries to check or ask about it. An explicit user pause holds until
    their explicit "go". Hardware authorization remains operation-specific.
14. Review code and evidence at the scope warranted by risk. The final
    whole-goal review remains separate from routine branch review and occurs
    only after its completion prerequisites; never present a self-review as
    independent verification.
15. **Refresh project statistics after every completed task** (user amendment
    2026-09-24). Once a task has passed review and verification and is integrated
    into `main`, but before starting the next task, run
    `python /mnt/daten-i/Sourcecode/ai-stats.py` from the root checkout
    `/mnt/daten-i/Sourcecode/KNXBench`. Verify that it exits successfully and
    updates the tracked `stats.md`; include that refresh in the completed task's
    bookkeeping commit or in a dedicated immediately-following stats commit.
    Never run it from an isolated feature worktree, because the Git statistics
    must describe the integrated project rather than a temporary branch.

---

## 2. Data-integrity and format work

T01–T07 and PDB-1–PDB-11 have landed; do not run their original
implementation checklists again. Verified boundaries, rather than new work,
remain in [COMPATIBILITY](../COMPATIBILITY.md),
[KNOWN_LIMITATIONS](../KNOWN_LIMITATIONS.md) (§1, §2, §3, §11–§13,
§61, §85, §92) and [PRODUCT_DATABASE_CORPUS](../PRODUCT_DATABASE_CORPUS.md).
The user's accepted exclusions are in §6; the open command-invariant finding
(ADR-0039 phases 3–5) stays in §8. Any new manufacturer scheme or hardware
claim needs independent evidence and a separate scoped decision.

---

## 3. User-visible residue outside the UI goal

The diagnostics UI, scan reconciliation, settings, drag/drop, discovery,
project creation and the closed search/accessibility bugs have shipped. The
remaining user-facing backlog is in [`goal-ui.md`](../../goal-ui.md), not in old
T08–T13 checklists. AR00 verified the formerly listed §23/§24 assertions
against the existing implementation and regression tests: downloads stream
bounded chunks while retaining temporary SQLite serialization, and the
browser picker uploads multiple selected/dropped files sequentially. Opening
a project is still an individual choice. Neither is a new implementation task.

---

## 4. Reporting, diff and CSV residue

CT-1/CT-2/CT-6 closed the originally scheduled report-preview and diff-UI
work. The twelve DIN-26 decisions in §6 explicitly accept PDF, full report
prose, diff correlation/merge and CSV range/spreadsheet limitations. ETS
parity is unverified; it is not an implicit feature target.

**Not yet explicitly accepted or implemented:** §40 (editable CSV-derived
columns and Description/Comment fields) and the residual §60 large-diff
paging boundary. Review these with the user before marking this section
complete; neither was in DIN-26's accepted twelve. Product-dependent report
names and incomplete parameter semantics (§46/§47) remain documented
evidence limits, not claims that a new renderer can recover absent data.

---

## 5. Priority 4 — platform, packaging and the manual

- **D12, user manual acceptance — after UI U13.** A manual already exists in
  [`docs/manual/`](../manual/README.md). It still needs the ADR-0024
  location decision, screenshot decision and claim-by-claim verification on
  the finished UI; do not describe it as unwritten.
- **Alpha release — user decision.** An x86_64 AppImage has been built and
  inspected (ADR-0021), but no v1 alpha tag/release was authorized. Decide
  whether to tag only after the remaining evidence and manual review; state
  the tested Linux boundary rather than claiming broad distribution support.

---

## 6. Accepted out of scope — do not start, do not fold in

Each of these is a recorded decision, not an oversight. Reopening one costs
the run its credibility.

- **T19 — KNX Secure** (Data Secure, IP Secure, `.knxkeys`). Deferred
  2026-09-11; needs sample key material and a secured installation. §8, §26.
- **T20 — the `Functions` domain concept.** Deferred 2026-09-11 until the new
  KNX specification documentation is available; needs its own ADR first.
- **T22 — multi-user / concurrent editing.** Parked, not a v1.0.0 must-have.
  §63. Note that the diagnostics companion window already depends on this
  boundary (§82).
- **T21's spatial canvas.** ADR-0019: the building model stays topological, no
  entity carries a position in v1.0.0. A later `FloorPlan`/`Placement` layer
  needs its own ADR and store schema 7; neither exists.
- **E4's eighteen 200-series LTE/system DPT main types.** Accepted out of
  scope 2026-09-20: nothing in `knx-net`/`knx-core` speaks LTE addressing, so
  the codecs would be decoration. Main types 1-30 are covered with no gaps.
  §90 also stands: DPT main type 46 never existed, it was a count.
- **§68/§69/§71 — module handling.** Accepted as documented boundaries
  2026-09-20. §69 is not fixable at all (a synthesized `Module/@Id` would be
  an invention presented as data).
- **§13's AES half** — blocked on a real ETS6 AES-protected sample, not on a
  decision. ZipCrypto already works. A synthesized sample proves nothing.
- **§1 / project-schema evidence** — `.knxproj` schemas 12–14, 20 and 22,
  and an independent module-using schema-23 installation, still rest on no
  independent project evidence; product-package schemes are a
  different format boundary. Do not use the new `.knxprod` samples to claim
  `.knxproj` compatibility.
- **`.vd2` and unobserved `.knxprod` schemes 15-19/22** remain out of scope.
  The user reopened the now-evidenced standalone product schemes 12/13/14/21
  on 2026-09-23 through §2.8; no further sample-hunting is needed for those.
- **ETS re-import of KNXBench-written projects** — dropped as a goal by
  ADR-0015; `.knxproj` export itself was withdrawn (ADR-0028).
- **§6 — devices behind manufacturer plug-in DLLs.** No verified semantics
  exist across that boundary; do not invent them.
- **A plugin API** — ADR-0025: extension stays data-shaped (language packs,
  product databases, CSV, the headless CLI). §107. Its tripwires are
  greppable; if one fires, that is a signal to revisit, not to build.
- **Online device-catalog update (C6) and an ETS-App-style ecosystem (F4)** —
  no well-formed task exists for either.
- **A mobile app and non-Linux desktop support** — new-platform work, premature
  while the Linux-first desktop is unfinished.
- **The project logo** — the user is handling it. Do not start it and do not
  fold it into packaging.
- **Twelve documented boundaries from DIN-26** — accepted 2026-09-27 by the
  user as permanent documented boundaries, no work scheduled
  ([decision](../superpowers/plans/2026-09-26-din26-oos-board-decision.md),
  `e0c1f37`): §45 native PDF, §48 the full report prose catalogue,
  §52/§53/§54 diff correlation, §55 applying a diff, §56 three-way compare,
  §39 CSV ranges and renaming, §41 spreadsheet transforms, §12's remaining
  manufacturer gaps, §85 signature verification, §2 the missing XSD. Each
  KNOWN_LIMITATIONS entry stays open as the public record; no ETS parity is
  claimed. These accepted limits are not implementation tasks.

---

## 7. Research requiring a new decision

Research for MCP/natural-language interaction, task automation, project
notes and a "who talks to whom" view is recorded in `docs/RESEARCH.md`
§13/§14/§16 and ADR-0031. None is a scheduled v1 implementation. The
legacy `.vd`/`.pr` design prerequisite is in
[`docs/VD4_PRODUCT_DATABASE_IMPORT.md`](../VD4_PRODUCT_DATABASE_IMPORT.md)
and the reviewed format spec; modern `.knxprod` is a different format.
Do not dispatch completed research as implementation without a new scope
and data-integrity decision.

## 8. Parked structural finding

ADR-0039 phases 1–2 closed duplicate-id data loss. Its phases 3–5 are not
implemented: `Project` fields remain publicly mutable, so "all mutation
goes through `Command::apply`" is still a review convention, not a type
invariant (KNOWN_LIMITATIONS §129). Revisit when changing the core public
surface. LIMITATION_TRIAGE was recounted alongside the limitation cleanup;
recount it by command after any future entry change.

---

## 9. Delivery and review

Work incrementally in isolated worktrees; no KNXBench subagents (user decision
2026-09-28). Check each diff, test and documentation claim before integration.
Do a distinct whole-branch review; fix findings and repeat relevant gates.
The final whole-goal review is separate and only begins after all actionable
items have been delivered or explicitly accepted as boundaries. A green
focused test or self-review is not an independent acceptance verdict.

---

## 10. Completion condition

Finish only when:

- `docs/IMPLEMENTATION_STATUS.md`, `docs/ROADMAP.md`,
  `docs/GAP_ANALYSIS_ETS.md`, `docs/KNOWN_LIMITATIONS.md` and
  `docs/LIMITATION_TRIAGE.md` are reconciled with each other and with the
  code;
- every actionable item in sections 2-5 and 8 has proof of completion, or a
  recorded, non-actionable external blocker;
- all nine gates pass, with the freshness check from rule 7 done;
- every remaining exception carries the user's explicit out-of-scope
  acceptance;
- and the final whole-goal review has run over the finished whole and its findings
  are resolved. It is the last thing that happens, not a formality on the way
  out — if it opens something, the goal is not done.

Report the completed work, the verification evidence, the remaining external
blockers, and the next required user decision, if any. Commissioning stays
where section 0 left it: excluded from this run; its verified narrow hardware
path and unresolved safety boundaries belong to `goal-commission.md`.

## 11. User-reported UX and workflow issues

The thirteen original reports have an evidence checklist in
[`docs/superpowers/plans/2026-09-21-user-reported-issues.md`](../superpowers/plans/2026-09-21-user-reported-issues.md).
Delivered slices are not pending tasks. `goal-ui.md` owns the remaining UI
checks; ISSUE-04's saved-baseline and locale-aware timestamp rows are already
tested and ticked (§12.2). New domain or compatibility questions remain investigation-first.

---

## 12. Current completion and handover

Sources for this section:

- The status map from the Paperclip DIN-3 plan, which Steve Smith checked against git and the docs on 2026-09-26 at `main` `7b64496`.
- The eight agent handoffs in `docs/paperclip-shutdown/` (uncommitted) and their summary, `docs/paperclip-shutdown/STATUS.md`.
- A git check of every Paperclip worktree on 2026-09-27.

**Historical observation, 2026-09-27:** `main` was at `7b64496` for the
Paperclip shutdown check; Paperclip merged nothing and pushed nothing in
that check. This is not the current repository revision or a dispatch order.

"Board" in the Paperclip sources means the user. A Board approval listed below is therefore a user decision. Do not ask for it again.

### 12.1 Completed foundations

T01–T17 and PDB-1–PDB-11, the Paperclip DIN-9/10/11/4/16/12/26
takeovers and the dated UI delivery through U13 are already integrated.
DIN-3 was rejected, not an item to merge. Evidence is in
`docs/IMPLEMENTATION_STATUS.md`, ADR-0038/0039 and Git history. Do not
repeat the former takeover and verification instructions. DIN-26's twelve
accepted limits are recorded in §6.

### 12.2 Work still requiring a decision or delivery

1. §3's former streaming/file-picker tasks (§23/§24) are resolved; §4's
   CSV-derived columns and large-diff paging (§40/§60) still require a scoped
   decision, not an invented acceptance.
2. `goal-ui.md` U0–U13's bounded slices are delivered at `dfa0cc79` with
   publication receipt `8a51b74d`. The user accepted the independent
   GPT-6.1-Sol review in place of unavailable Claude; its original changes-
   required verdict and the later in-session fix review keep their provenance.
   Both ISSUE-12 boxes are ticked with host-firewall and offline UDP unicast
   loopback evidence, not a wire capture/full multicast roundtrip. Native
   WebKitGTK Search and real-screen-reader verification remain disclosed.
3. ISSUE-04's saved-baseline and locale-aware last-save acceptance rows are
   now tested and ticked; do not dispatch them again.
4. The manual's location/screenshots/claim-by-claim acceptance (AR16), local
   candidate (AR17), final integrated gates and independent whole-product
   review (AR18), then the user's release decision (AR19). No release tag
   has been authorized; the release decision is not a review prerequisite.
5. Commissioning K6/serial/K13 confirmed address writes currently refuse
   before any tunnel, pending verified device-specific durable recovery;
   `1.1.32` has bounded read-only presence/partial identity evidence only;
   the exact model, installed application and recovery remain unverified.
   ADR-0039 phases 3–5
   remain parked (§8); AES project support remains artifact-blocked (§6).

### 12.3 Parallel tracks and handover

`goal-ui.md` owns the remaining Web work and the live Web lock protocol.
`goal-commission.md` owns programming and every bus mutation; no prior go
transfers to a different operation. This goal does not edit `apps/knx-web`
without its lock and does not merge, clean or rebase another session's
worktree. For current ownership and in-flight work, read the top of
`.ai/CURRENT_STATE.md`, not a historical status paragraph.
