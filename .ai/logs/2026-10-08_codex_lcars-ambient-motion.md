# LCARS calm ambient follow-up

- User explicitly requested a subtle, living idle presentation after approving
  the deployed theme. Exactly two decorative CSS loops: segmented-header
  opacity (10s) and small K-emblem apricot/lavender colour (16s). Subtle slows
  them to 18s/24s and reduces amplitude. No blinking engineering content,
  geometry changes, fake activity/success, JavaScript timer, API/core change
  or new dependency/persisted setting.
- Existing Motion level Off, OS reduction and theme removal cancel live effects
  to static paint. Saved Off and OS reduction also prevent cold-start effects;
  Subtle survives reload. Imported-format palettes cannot gain the loops.
- RED: three new boundary assertions failed before CSS; GREEN: **2,332 frontend
  tests / 138 files**, **158 Chromium tests**, build/type/flow/five repository
  gates passed. Actual production-workbench CLI: **54 named assertions**, no
  unexpected requests/browser errors; real clock progression, amplitude,
  cancellation, independent density, Save As refusal and small layouts verified.
- ADR-0092 amended for the explicit decorative exception. Original standalone
  study and prior delivery/deployment receipts remain historical and unchanged.
  New source/build/evidence binding: `design-studies/lcars/ambient-verification.json`.
- Source self-review only; native accessibility/Firefox not certified. Local
  follow-up complete; commit/push/redeployment still require authorization.

## Reproduction and boundaries

- Candidate worktree: `/mnt/daten-i/Sourcecode/KNXBench.worktrees/lcars-motion-20261008`,
  from `origin/main` at `f5aff6d586c1e86b636e6bd72bca38dc9683ef16`.
- Gate process `proc_54a8b9f12805` exited 0, ten stages and frozen inputs;
  short TMPDIR avoided the earlier Chromium socket-length setup problem.
- Final verifier restored Subtle after its added Standard cold-start controls;
  all frontend inputs remained byte-exact, final verifier syntax and 54 actual
  workbench assertions passed. This adjustment is stated in the receipt.
- Native CSS endpoint captures show legible K/toolbar, unchanged geometry and
  restrained dimming; real clock progression/cancellation are separate checks.
- No commit/push, live Docker replacement, user data or bus operation performed.
  Existing deployment and rollback retained. Shared root HEAD/index/foreign
  work not synchronized or staged. Future rollout needs explicit permission,
  saved user work and no active hardware programming.

## Local delivery closure

The publication/restart form was cancelled without an answer; this grants no
commit, push or deployment permission. Local source and exercised production
`dist/` remain in the named candidate worktree as the unique deliverable, not
throwaway scratch. Browser/preview are stopped, no active owned process cwd
remains, raw CLI output and scratch (including the isolated Cargo target) were
removed. Permanent source/build-bound receipt and inspected phase captures
remain. Existing live server, rollback, shared root HEAD/index and foreign work
were not changed by this follow-up. Await explicit rollout permission; do not
interpret earlier LCARS deployment approval as approval of this new package.

## Explicit publication/deployment authorization

At 2026-10-08 09:05 CEST, the user gave a new "go" for this package's commit, main push
and Docker deployment. Authenticated read-only checks show project unmodified,
no device download and no address programming session. Recheck just before stop.
Credentials were entered only through the origin-bound vault, never printed.
Existing source/build hashes still match the 2332/158/54 acceptance receipt;
origin/main is still its exact base. Force-stage only this ignored owned log.
Keep foreign community/wizard changes and the shared root index untouched.

## Upstream integration before publication

Origin advanced to e3e7641b4f4adf092214a592204f62d8616fa3ee with the already-published project wizard
(b2a7dc3a + e3e7641b). No old-source commit/push or server change was made.
Own 13-path delta integrated in a fresh publication worktree; all wizard CSS,
status and handover retained. LCARS presentation suffix is byte-identical.
CLI verifier now acknowledges the actual Project created/Done page; no UI
workaround or product/API change. The original ambient receipt stays historical.
Fresh combined source/build, full Chromium, workbench and targeted seed gates
are required before publishing; client/server ship together.

## Fresh combined acceptance

- Full gate proc_fc51ceea415c exited 0 on frozen fingerprint fe669d0b2d6bd68d299eae646ebcbb39ce8f9c630f384dca664494b3443a4d4d. Actual totals: 2358 frontend / 140 files, 162 un-retried Chromium, 13 project-seed + 5 HTTP-seed, five repo gates, build and fixture types. Fresh production-workbench: 54 unique checks with no errors/unexpected requests; actual phase captures visually inspected.
- Earlier combined attempt's standalone keyboard target drift is documented in the separate flow-study log; 20-repeat stable identity control passes without skips/retries/sleeps/mouse substitution. Original ambient evidence remains historical; fresh source/build binding in ambient-integrated-verification.json.
- In-session self-review and static added-line scans accepted; no independent reviewer, native WebKitGTK/Orca or hardware claim. Live container still original; scoped data/cert baseline captured. Publish separate test-fix and ambient commits; then canonical image/probe/safety/swap.

## Final latest-main source acceptance

Origin advanced again to a642eaf98c09cc6ddc6a1820affff6d0fdd4c927 (device wizard/achievement changes). Publication guard aborted before staging/commit/push. Isolated branch fast-forwarded after exact own-delta backup; ambient deltas re-applied, status/handover preserved. Both wizards, App, Toast and server catalog/domain source are exact current-main bytes. Full gate proc_c926963f050d exited 0: 2373/142 frontend, 170 Chromium, 54 actual workbench, 27 targeted Rust tests, five repository/build/type gates and frozen source. Final phase captures visually inspected. Fresh publication receipt distinct from f5/e3 historical evidence. Commit/push/image/probe/live swap still pending at this source checkpoint.
