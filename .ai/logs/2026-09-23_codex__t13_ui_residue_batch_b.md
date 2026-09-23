# T13 UI residue batch B — resumed verification

## Scope

Continues the existing `t13-ui-residue-b` worktree from `36d7bca`, not a
reimplementation of its three reviewed slices. The user authorized resumption
and raised the weekly usage pause threshold from 60% to 95%. The read-only
Codex account preflight returned 67%; no reset credit was consumed.

The batch consists of honest project modification state (§§81/103), active
session group-address context (§91), exact documented space types (§89), and a
build-time theme-role contrast gate (§120). Native store schema stays 9.
Report preview/selection and §60 are not folded into this branch.

## Resume work

- Added the deferred import-baseline regression to `http_project_routes.rs`
  using the existing synthetic `write_minimal_knxproj` fixture. It cannot
  silently skip for lack of private corpus. A dirty prior project is imported
  over; both the POST projection and fresh GET report clean state, history and
  native save-path reset, and edit/undo returns to the imported baseline.
  This test covers existing product behavior; no implementation change was
  needed for it.
- Task 4 extends the existing test-owned theme parser, with no dependency or
  runtime UI code. Its initial implementation evaluates foreground/background,
  foreground/surface and on-accent/accent for base palettes and variations.
  Exact contrast threshold is 4.5:1; unsupported values are errors rather than
  skipped themes.
- Updated the goal's weekly threshold and measured header ceiling (162), and
  corrected the stale `knx-net` freshness checkpoint in §119.

## Initial proof before closing review

The task worker's transcript records six expected contrast failures at
2026-09-23 11:04 before adding the helpers, then 77/77 focused tests after the
implementation. The later controller run independently passed TypeScript and
946 frontend tests in 63 files. A production `npm run build` also passed;
Vite still reports the existing >500 kB chunk warning. Its removal of tracked
`dist/.gitkeep` was repaired without changing application code.

Controller Rust gates:

- `cargo test --workspace --no-fail-fast`: 1,970 passed, 0 failed, 5 ignored,
  92 result blocks.
- `cargo fmt --all --check`, warning-denied workspace Clippy, layering,
  headers and cargo-deny: exit 0.
- Initial anchors: 388 links in 184 Markdown files, none dead.
- Added-line pattern-class scan: no private IPv4, secret assignment or shell
  evaluation findings; no transport, migration or limitation-triage changes.

The old canary assertion (252 lib tests) deliberately stopped the first gate
run. Source enumeration found 253 test attributes. `cargo clean -p knx-net`
then compiled a fresh binary listing and running all 253 tests. The added scan
comparison regression explains the old count; a stale count is not a reason to
trust a binary. Cargo target is `/var/tmp/knxbench-t13-target`, incremental and
profile debug information disabled, two build jobs.

No real KNX gateway, physical LAN or hardware interaction was performed.
Existing transport tests exercise their simulator/loopback paths. Proprietary
reference fixtures, where available, are read only; none is committed.

## Review status

A fresh whole-branch review was dispatched to `claude-opus-5` with high effort.
It includes all changes from merge base `2ffcf82` through the working tree,
not only the resumed theme files. The PATH Claude wrapper tried a mise update
and stalled; the already-installed CLI binary ran successfully instead. The
initial theme implementation used a native Hermes worker (`gpt-6-astra`), an
explicit runtime adaptation, not a claim of an Opus implementation review.

Controller edge probes already reproduced two review targets: unchecked RGB
channels can yield a false-green contrast result, and an unsupported color
behind a token alias loses the terminal offending value in its diagnostic.
Final review findings, RED/GREEN corrections and integration evidence follow
when verified. This checkpoint does not claim T13 is merged or complete.

### Provider exception and numerical references

The Opus CLI ultimately returned only its weekly-quota message, not a review.
The parallel T14 survey hit the same quota. At 2026-09-23 11:59 CEST the user
explicitly approved replacing this run's Opus branch reviews with independent
GPT-6-Astra reviews; goal rule 14 records this bounded exception. Fresh native
review/discovery workers were dispatched. A separate read-only Codex usage
check returned 70%, below the new 95% pause threshold.

W3C source check for the numerical fix:

- [WCAG 2.2 relative luminance](https://www.w3.org/TR/WCAG22/relative-luminance.html)
  gives the current sRGB breakpoint 0.04045, coefficients
  0.2126/0.7152/0.0722 and the 255 normalization; 0.03928 is historical.
- [SC 1.4.3](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)
  specifies 4.5:1 for normal text and explicitly forbids rounding a lower
  computed ratio up to acceptance.

The controller also identified the unrefreshed undo/redo call path as a review
target for §91. Findings and fixes must be verified, not assumed from this note.

## Final review and user-directed pause — 2026-09-23 12:12 CEST

The user instructed: finish the review, then pause. No product code was changed
after that instruction, and no commit, merge, push or T14 continuation occurred.

### Provider provenance correction

The native preliminary review/discovery dispatcher reported `gpt-5.6-luna`.
Calling those helpers GPT-6-Astra in their prompts did not select that model;
the earlier model labels in this log and ledger are not review provenance.
The original theme worker's prompt label alone also is not verification of its
actual runtime model. Those preliminary helpers are no longer live.

The authorized whole-branch review was therefore executed through Codex CLI
with explicit `--model gpt-6-astra`, high reasoning effort and read-only sandbox.
The first invocation failed before a review result due to a disconnected
configured-provider stream (exit 1). An invocation-only `--ignore-user-config`
retry used provider `openai`, logged the requested model/effort/sandbox, completed
the review and exited 0. No persistent CLI or Hermes configuration was changed.
Latest read-only weekly usage was 71%, below 95%; this pause is user-directed.

### Scope and unchanged-tree proof

Base: `2ffcf8267b0e86799bf4ef17bc9f41a049d2e5ea`; HEAD:
`36d7bca6b47aaf09239c18fe9493b9afcb29489e`, including uncommitted changes.
Whole-diff SHA-256 before/after the read-only review was identical:
`b425329b21384666651b07826930630b3c308d526306b75ac3ccff2b4f89b0ac`.
The reviewer covered all four slices and recommended **do not merge**:
**0 Critical, 3 Important, 2 Minor**. Parent checked each finding's exact source
location. The Important scenarios below are statically established, not yet
confirmed by newly written/running regression tests.

### Open findings — fixes deferred until explicit resume

1. **Important — stale save refresh**, `apps/knx-web/src/App.tsx:680–683`:
   a delayed clean GET response after Save/Save As unconditionally replaces a
   newer dirty tree from an intervening completed edit; Quit can lose its
   unsaved-change warning. Bind refresh to current project/update generation
   and reject obsolete responses; add a delayed-GET/intervening-edit regression.
2. **Important — Undo/Redo session context**, `apps/knx-server/src/routes.rs:809`,
   `:2160–2173`: only direct restyle participates in mutation–snapshot–session
   publication. Undo/Redo can restore another project style while the session
   retains the old style, or interleave with publication. Include these history
   paths in the same serialized publication contract. Add fake-session tests
   for Undo, Redo and ordering; do not use physical hardware.
3. **Important — duplicate accent blocks**, `apps/knx-web/src/themeTokens.test.ts:620–623`:
   variation cases are named by `(theme, accent)` then looked up with `.find()`.
   A later duplicate block with white-on-white colors wins in CSS but both test
   cases inspect the first block. Reject duplicate keys and parameterize actual
   variation objects; add an unreadable-second-block regression.
4. **Minor — unchecked numeric ThemeColor**, `apps/knx-web/src/themeTokens.ts:605–608`:
   object inputs bypass the string parser's finite/range validation. Validate
   every channel of both objects, retaining valid fractional values. Four
   current RED cases cover NaN, Infinity, -1 and 256. Current CSS strings are
   protected by the earlier fix; do not overstate this as a current palette bug.
5. **Minor — historical WCAG breakpoint**, `apps/knx-web/src/themeTokens.ts:599–601`:
   replace 0.03928 with referenced 0.04045. The fractional-channel test exposes
   the difference; integer 8-bit palette channels have no value between these
   breakpoints. Luminance coefficients and unrounded 4.5 comparison are correct.

### Test state at pause

- RGB-string overflow/Infinity and missing terminal alias diagnostics were
  first reproduced with three RED tests, then fixed using range checks and
  typed `ThemeColorError`. Focused 94/94 and TypeScript passed afterwards.
- Additional tests were added before the pause instruction. The latest focused
  `npm test -- --configLoader runner src/themeTokens.test.ts` at 12:06 CEST
  returns exit 1: **100 passed, 5 failed, 105 total**. Failures are findings 4/5;
  supported opaque RGB/hex and non-opaque hex tests pass. No fix loop was run
  after the review. Earlier full green gates are historical only.
- Reviewer reported no new finding for five exact space types, strict native
  unknown-kind errors and unchanged schema 9. Its added-line pattern-class
  security scan found no secret/private-IPv4/shell-execution issue; §60 and
  `LIMITATION_TRIAGE.md` remained untouched. No physical KNX/LAN/hardware traffic.
- Raw CLI evidence remains temporarily in active-profile scratch:
  `t13-astra-review-direct.log` and `t13-astra-review-result.md`. The findings
  above are the durable record; do not depend on scratch retention to resume.

The pause above was subsequently superseded by the user's bounded instruction
to finish T13 through merge, then stop before the next task.

## Bounded resumption and Sol review — 2026-09-23

The user changed ordinary review selection to explicitly pinned `gpt-5.6-sol`
with medium/high effort. The strongest model is reserved for the very last
whole-goal review, not T13. `goal.md` records both this policy and the T13-only
execution boundary. Sol-high implementation and read-only review headers were
verified as provider `openai`; no persistent provider configuration changed.

The first fix wave addressed the five findings above. Controller gates all
exited zero: Rust 1,973 passed / 5 ignored; frontend 968 passed; TypeScript,
production build, fmt, warning-denied workspace Clippy, layering, headers,
anchors, cargo-deny and whitespace checks. The build's existing chunk-size
warning remains. These results describe that first-wave tree, not subsequent
edits. The fresh `knx-net` canary remains 253 source attributes/binary tests.

Independent Sol-high review still blocked integration with three Critical,
two Important and one Minor finding:

- Save chose its destination before locking the project, allowing an Open to
  pair the replacement project with the previous file's path.
- New's dirty predicate and replacement were separate transactions, allowing
  an intervening accepted edit to be discarded without the required refusal.
- Open/import/new published project state before opaque and manufacturer
  manifest collections, allowing a concurrent save to mix project data.
- Client generations covered delayed Save GET after edit, not delayed edit
  after Save nor replacement responses arriving after newer edits.
- Style publication refreshed the server context but left the browser's
  session fingerprint stale; source comments and the user manual also claimed
  the context never changed. The manual and §82 now distinguish style refresh
  from other edits and acknowledge T12's existing current-project GET.

The controller confirmed the source paths and started a second bounded
RED→GREEN fix/reverify wave. Runtime snapshot order must remain application
metadata, not persisted KNX project data; replacement must cover every
per-project collection under a coherent lock order. Final evidence follows
only after verification. Live weekly usage at 13:05 CEST was 79%, below 95%.

No commit/merge/push yet. Main's pre-existing local handover is preserved in
the stash named `T13 preserve pre-existing local handover before merge`, with
an additional active-profile scratch binary-diff backup. It must be restored
or explicitly preserved before finishing. No T14 implementation or hardware
activity is authorized.

## Second correction wave — controller verification

The worker fixed the six source/test findings without touching parent-owned
documentation. It reported initial compile-RED (`E0425`, missing test seams)
for the three server transactions; these are not behavioral reproductions of
the original races. Frontend reverse-order regressions did produce two real
assertion failures before their fix, and the bus-rebase case one real failure.

The controller added a stronger public-entry-point Save lock-order regression.
Temporarily restoring the original path-before-project semantics produced the
specific assertion `Save chose its path before locking its project`, exit 101,
0 passed / 1 failed. Restoring the project-first implementation then passed
1/1, exit 0. The test holds the path mutex and observes the leading project
mutex with a bounded yield loop; it releases the blocker and joins before
asserting. No temporary mutant remains.

All eleven full controller gates on the final second-wave tree exited zero:
Rust 1,977 passed / 0 failed / 5 ignored across 92 blocks; frontend 971/971
across 63 files; TypeScript/build/fmt/Clippy/layering/headers/anchors/deny/diff.
The source/binary `knx-net` canary agrees at 253. Source diff SHA-256 before
and during the read-only Sol-high re-review:
`0b25f24efbf6c2d0128703bcf5dae19980b98ec3067b66bff41236fe943efc50`.
The added-line pattern-class scan has no credential/private-IPv4/shell/eval
finding. An overly broad initial scope check flagged `knx-report/render.rs`;
inspection proved it is only the already-committed T13 Task 3 regression for
the five space kinds, with no new report implementation or T14 work.

### Newly reproduced lifetime blocker

Using Node's native TypeScript stripping to execute the real `busContext.ts`,
the controller supplied a synthetic in-memory storage adapter and the existing
test fixture's ProjectTree shape. Publishing a previous-process revision 100
succeeds. Publishing a different project's revision 1 after simulated server
restart returns `false`, retains revision 100 and retains the old fingerprint;
an assertion requiring acceptance exits 1. This is a local synthetic probe,
not fabricated server output; it contacts no server or hardware.

`AppState.project_revision` starts at zero, whereas `ProjectContextRecord` is
persisted in localStorage and lacks a server-instance identity. App's own
revision ref has a related lifetime boundary. The independent re-review is
still running. This additional RED probe is outside the green suite totals;
the branch must not be merged while the lifetime problem is unresolved.

The actual Hermes GoalManager for this session was read through the installed
profile-scoped implementation: the standing goal is already paused (5/20),
not active. Bounded T13 work does not resume it or authorize T14.

## Targeted additional round explicitly authorized

The second independent Sol-high review completed at 13:50 CEST with **0
Critical, 1 Important, 1 Minor**, still BLOCK. The three server atomicity
findings are closed. Snapshot order and context rebasing work within one
process; restart/session-ID reuse remains the Important finding. Minor:
connect-time-only comments/messages remain in BusMonitorPanel, BusComposeForm,
EN/DE catalogues, App and DiagnosticsCompanion.

After reaching the bounded automatic correction limit, the controller asked
instead of silently beginning another round. At 13:52 CEST the user selected:
`Ja, gezielt korrigieren und bis zum sicheren Merge abschließen`. This grants
the targeted incarnation/lifetime fix, refreshed copy, new regression proofs,
independent Sol review and eventual verified merge, not T14 or goal resumption.
The live read-only weekly preflight is 85%, below 95%.

Main's pre-existing local handover was restored byte-for-byte from our temporary
stash and compared with its original binary patch. SHA-256:
`32db3b553f03df4e268dbd04526cc732e1c1e5c6f128492303084eb65528b06e`.
Only then was that temporary stash dropped. Main retains its original local
handover modification; unrelated Headroom notes must remain uncommitted and
preserved during later integration.

## Final lifetime correction and approval

The additional user-authorized round adds a transient OS-random server
incarnation to live project snapshots and Bus Start/Poll/Stop metadata.
Revisions compare only within the same incarnation; browser records retain
retired identities so delayed old-process responses cannot regain authority.
Session identity includes incarnation rather than a reused numeric ID alone.
Legacy/mismatched session data stays unverified; pure/offline projections omit
runtime metadata. EN/DE copy now says last confirmed context publication.

The controller executed the actual busContext.ts with synthetic storage:
A/100 → B/1 accepted, delayed A/101 refused, A's reused numeric session ID not
verified in B. All eleven branch gates passed with Rust 1,977 passed, zero
failed, five ignored (92 blocks); frontend 977/977 in 63 files; TypeScript,
production build, fmt, warning-denied Clippy, layering, headers, anchors,
cargo-deny and diff check. Source/binary knx-net count agrees at 253. Added-line
credential/private-IPv4/shell/eval and forbidden-scope scans returned no findings.

Independent CLI gpt-5.6-sol/high/read-only review exited 0 and approved with
zero Critical/Important/Minor findings. It explicitly closed the restart and
stale-copy findings without reopening the previous atomicity/color corrections.
Source diff SHA256 is
`6beabcc7183b92545d9f07a76a6c6eb8bcdc399e02acad493c2f2457bbd41734`.
Evidence is in active-profile scratch `t13-incarnation-review-result.md` and
`t13-incarnation-final-gates/results.json`. Merged-main verification remains
separate; no T14 implementation or strongest-model whole-goal review occurred.

## Integration — 2026-09-23 14:36 CEST

Correction commit `fef0e52` was merged into main as `7f9c8c4`, with KNXBench's required author/committer email and no co-author trailer. Only handover
and manual-status text conflicted: both histories, main's device-drag status,
and T13's five-space-type status were retained. Product source matches the
approved branch byte-for-byte through Git's empty apps/crates diff.

All eleven gates were repeated on merged main and exited zero. Rust: 1,977
passed / zero failed / five ignored in 92 blocks, knx-net 253; frontend:
977/977 in 63 files; TypeScript and production build; fmt, warning-denied
workspace Clippy, layering, headers, cargo-deny and whitespace. Anchors:
381 links in 195 Markdown files, none dead. The tracked dist/.gitkeep removed
by Vite was restored. Evidence: active-profile scratch/t13-merged-main-gates/.

The standing Hermes goal was independently read as paused, five turns used
of twenty. T14 remains unimplemented and no continuation is authorized.
The user's pre-existing local handover patch is kept out of these commits and
will be restored after the evidence commit, with its exact added bytes checked.
