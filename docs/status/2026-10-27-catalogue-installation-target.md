# Explicit catalogue installation target — 27 October 2026

## Delivered source scope

Main-first merge `a1742023993de900e3f1cd829e5099b84d6b1e54` publishes
feature `8d3ebe69d5461ebb5bd4a9afdda872ca7a1c3b7b`. Local, tracking and live
`main` were read back equal. The selected user checkout remains unchanged;
unrelated local changes and parked worktrees were preserved.

When creating a catalogue device without a target line, a project with several
installations now requires an explicit **Installation** choice. Create and
Enter remain blocked until the choice is current. Removing that target does
not silently choose another one, even if only one installation remains: an
explicit new choice is required. An empty placeholder or a reappearing old
identifier does not confirm it. Switching to a line-owned target and back
starts a fresh installation choice.

An initially single-installation request is unchanged. A selected line still
owns its installation, and no open project blocks creation. Lost-response
retries use the captured original arguments and request identity, not current
controls. Live context is required from the App. This uses the existing
validated create command and Undo path; no server runtime, API or schema was
changed.

## Executed acceptance

- Independent whole-feature, correction, final target-removal and scoped layout
  reviews: **APPROVE** after their findings were addressed.
- Focused final Web acceptance: **257 tests passed**. The 27 TypeScript
  configurations and the web build passed.
- Real in-process HTTP regressions: **eight passed**, including second-
  installation placement, first-installation preservation, Undo/Redo and
  invalid-target refusal without changing the project. IDs consumed by Undo
  remain consumed; the test does not pretend allocation rewinds.
- Focused Chromium acceptance: **11 passed**, including EN/DE keyboard actions,
  captured-retry regression and 360 px layout with real theme tokens and fonts
  in both themes and densities. The component fixtures mock transport; they
  are not browser-to-backend acceptance. The separate HTTP tests exercise the
  actual server and command path.
- Deliberate lost-target logic reversion produced three intended failures;
  restoring the safe implementation passed. A line-switch reset regression
  failed before its correction. Original overflow and hint-placement failures
  were retained; corrected layout assertions passed without relaxing them.
- Maintained change-scoped package gates on both the committed branch and
  main-first merge: **14/14 stages green**, frozen inputs, no touched or
  unchecked user data. The integrated selection passed **1,503 Rust tests**
  with **62 ignored**, and **51/51** selected offline corpus cases.
- Each full Web gate passed **4,568 Vitest tests** and **932 Chromium cases**
  (863 functional, 69 exclusive). Tooling registered 461 tests with six
  declared optional skips. This was not a release-wide or native-platform
  acceptance run.

Source snapshots, the five actual Rust test executables per tree and both
built web bundles were archived with **192 resolved payloads**, each checked
by SHA-256 and streamed byte count. The sixth Rust result summary is rustdoc,
not a sixth retained executable. The feature tree, merged branch and own build
cache were retired after verification. The integration tree is held until its
result-documentation gate and publication complete.

## Retained boundaries

No cross-installation device transfer, new site model, building-part selection
in the catalogue, ID-reuse guarantee, vendor compatibility, native-platform
acceptance or hardware operation follows. Server requests outside this UI keep
their existing API contract. This delivery creates no release or deployment.

[Bounded structure scope](../limitations/structure-editor-scope.md#update-2026-10-27-explicit-catalogue-installation-target)
