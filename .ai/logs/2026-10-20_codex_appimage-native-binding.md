# AppImage native-binding preflight — 20 October 2026

## Scope

Independent release-engineering follow-up from OPEN_WORK. Add only a
post-install Rolldown import in the Linux AppImage workflow and tests executing
that command. No dependency changes, automatic reinstall/retry, product schema,
release/tag operation or assertion that npm's original omission is explained.

## Evidence

- Named test-first RED: missing post-install binding step.
- Workflow imports the installed package before the Tauri build; errors propagate.
- Real installed loader executes successfully. Isolated copies missing native
  dependencies or containing corrupt native libraries refuse the same command.
- Five focused regressions pass together with existing AppImage launcher tests.
- First full focused attempt exposed a test locator matching the job name rather
  than the build step; fixed to match the complete step prefix. Retained as a
  failed test attempt, not product evidence.

## Acceptance boundary

Independent initial review approves with one minor regression-strength finding:
the corrupt fixture must first load intact and name the copied library in its
failure cause. The retained test now does both in the same isolated tree. A
first correction attempt exposed a logical-versus-canonical temporary path
mismatch in the assertion; the actual refusal was correct. The assertion now
uses the canonical copied-library path. Focused suite: 31 passed, one existing
optional skip. Frontend production build succeeds. Correction re-review ACCEPTS
with no remaining findings. Main-first merge `f9cf7a65` passes all 13 maintained
scoped gate stages with frozen inputs: 322 registered tool tests (one existing
skip), 103 Rust tests, Clippy and all repository checks. The selected xtask
corpus is empty, not new corpus evidence. Both user-data guard lists are empty.
Code publication verified local = tracking = live. Two final in-memory failure-
masking controls are caught; no candidate source mutation. No hosted
workflow dispatch, new AppImage artifact or physical desktop acceptance claimed.
The existing Alpha 16 schema owner and exact-UP258 owner retain their scopes.
Raw command receipts are retained privately under the active profile.
