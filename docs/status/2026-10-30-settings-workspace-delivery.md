# Settings center workspace — source delivery, 30 October 2026

## Delivered behavior

Settings opens as one continuous page in the center workspace, with section
jumps at the top. Toolbar, sidebar and command palette reach the same page,
including without an open project. Left navigation stays available; Properties
is hidden temporarily without changing its preference. Back reveals the prior
workspace, selected entity and unapplied notes/instructions drafts.

The existing immediate-apply, acknowledged-save, validation and error-feedback
paths remain authoritative. Escape does not dismiss a normal page. Settings-child
confirmations remain modal. Retained comparison dialogs suspend their isolation
while their workspace is hidden and resume their opening order, available focus
and original opener without cancelling their controller or operation. Global
stale-programming-mark feedback and its reload retry stay outside the hidden
content, including while Settings is open.

[Behavior contract](../contracts/settings-workspace.md) ·
[User guide](../manual/user-guide/09-settings-and-appearance.md#opening-settings).

## Source and publication

The main-first runtime merge is `fd1dfa0e41175b9ad792d53fcdc09de779a25e91`,
built on `72db561a459e0e36e55d52a42725ab2ba1d7bef6`. It includes:

- `a6bcc00326ee6bf4bac27ac11a05a99dbe0422be`: center page, state retention,
  section navigation, retained-modal ownership and focus fallback.
- `92007aaf996ba10faf530be92e67e50d685f3532`: restores the unchanged global
  System-2 lifecycle notice to its original direct-center owner.
- `24d61232030a87aeb1f4e2351e79745e8be301c7`: updates existing UX flows to
  assert nonmodal Escape behavior and leave Settings with Back.

After push, local integration HEAD, fetched `origin/main` and the live remote
`main` ref were read back equal to the runtime merge. Later result-only
bookkeeping does not relabel the runtime revision tested below. The shared
working checkout and foreign worktrees were not synchronized or edited.

## Independent review and regression evidence

The original whole-feature review found a retained comparison's hidden modal
lease making Settings and navigation inert. A finding-specific correction audit
closed that reproduction and identified a disabled remembered Cancel control
which could strand resumed focus on the body. The final independent recheck
approved the actual-focus fallback, including its panel fallback and unchanged
valid-focus/default-modal controls. A separate bounded review approved restoration
of the single global lifecycle notice owner. Unchanged reviewed scopes were
carried under exact source hashes rather than repeatedly claimed as new reviews.

Original mounted-App semantic failures preceded production fixes. Independent
component and Chromium witnesses, corrected fixtures and rejected runs remain
separate. The first full default gate found the misplaced global notice in two
existing EN/DE cases. The second reached all functional cases, then found four
exclusive UX cases still expecting the intentionally removed Escape dismissal.
The root owner was restored, and those three assertion sites now explicitly
require Settings to remain visible before using Back. No case, lane tag, timing
bound, strict API rejection or unrelated motion/toast assertion was removed.

## Accepted verification

Both the final feature branch and the actual main-first merge passed the
maintained change-scoped package gate: **11 of 11 selected stages each**.
These are overlapping executions of the same scope, not additional unique tests.

Each accepted run includes:

- Formatting, documentation, browser-lane checks, layering, source headers,
  anchors, ledger, corpus-registration checks, tooling and final diff checks.
- **472 Python tooling tests, with six explicit skips**.
- A nested **33-stage web gate**: production frontend build, all maintained
  application/fixture typechecks, **4,688 Vitest tests**, and all **1,001 default
  Chromium cases** (**932 functional + 69 exclusive**).
- No browser skips, flakes, retries or unexpected outcomes; compiled inventory
  matches the executed lanes. Source inputs stayed frozen, every declared stage
  exited zero with valid proof, and the named user-data guard reported neither
  touched nor unchecked directories.

The focused suites cover EN/DE, wide/narrow windows and supported zoom bounds,
entry/jump focus, immediate settings changes and errors, actual editor/draft
retention, inspector preference restoration, retained comparison state and
foreground isolation, disabled-control resume, global-notice retry and continuing
mocked monitor polling. Browser APIs, including discovery, are intercepted with
fictional data. Failed attempts were preserved, not relabelled as acceptance.

## Boundaries

This is routine change-scoped source acceptance, not a release-full gate. No
application Rust runtime, private corpus, production-browser release configuration,
native WebKitGTK/Tauri or real KNX hardware acceptance is claimed. The selected
Rust tooling checks do not change that boundary. Narrow high-zoom native selects
may abbreviate their option text while retaining accessible labels and controls.

No settings schema, backend protocol, domain mutation or hardware permission
changed. No device/bus access, release, tag movement or deployment was performed.
The published Alpha 18 artifacts and their modal Settings presentation remain
unchanged; the new page requires a newer source build. Released manual screenshots
remain pinned to their original release.
