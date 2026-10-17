# Closed-by-default content disclosures

## Scope

Owner request: collapsible content should start closed by default. This is a web
presentation change only; no project/domain, storage, format, protocol or device
admission change. Navigation/Properties pane visibility is a separate layout
control and is unchanged. Remembered group-address view openings and explicit
search/selection/range reveals remain respected.

Implementation is isolated in `fix/collapsed-defaults-20261017`, based on
`c94f4208`. Preserve the dirty selected checkout and parallel object-function work.

## Changes

- Navigation content blocks and explorer branches start closed.
- Explicit Expand all still opens lazily mounted descendants.
- Native parameter/report/matrix/Function/evidence disclosures start closed.
- Manufacturer parameter groups track explicit openings, so groups introduced
  during a refresh also start closed without replacing mounted drafts.
- Regression tests cover defaults, manual opening, retained drafts, refresh,
  remembered address view state and deliberate reveal. Existing browser setup
  now explicitly opens the content it exercises.

## Review

Separate in-session self-review (no delegated/independent-agent approval).
Important findings addressed: lazy descendant expansion, newly refreshed group
state, visible-tab browser setup, and stable locator enumeration while toggling.
No secret assignment, dynamic execution or unsafe-HTML candidates in added lines.
Only existing public source and synthetic fixtures used; no private findings.

## Verification checkpoint

- Named REDs observed for navigation, explorer, parameter sections, address-tree
  defaults and native disclosure inventory before implementation.
- Complete component suite: 3635 passed, 0 failed (265 files).
- Production build and all 20 derived TypeScript configs pass.
- Documentation checker and tools tests pass.
- Focused browser attempt: 29 passes followed by two stale matrix-setup failures;
  corrected explicit column-picker opening and compact-tree assertions.
- Initial broad exploration intentionally superseded, not accepted. A later broad
  attempt cancelled while queued with zero executed stages behind the foreign
  object-function gate. Original logs retained privately.
- First actual complete web gate failed honestly with frozen inputs: component
  3635/0, docs/tools/build/20 typechecks pass; browser560/578pass,18fail,
  no skips/flakes. Production modes and repository tail did not execute.
- Classified failures: range-create4, structure-heading8 and UX-feedback4 still
  needed explicit parent opening; splitter2 exposed a genuine zero-height initial
  measurement now that its target starts hidden. Existing assertions retained.
- Named splitter drag/keyboard REDs (112 vs280px,88 vs176px) observed before fix.
  ResizeObserver plus interaction-time measurement correct first input and ARIA;
  final12splitter tests pass, including observer disconnect and zero-height fold.
- Focused correction run: 19/19 browser cases passed (all 18 previous failures
  plus one sibling), 0 skipped/flaky.
- Final complete web gate (63/63 stages, inputs frozen, identical source
  fingerprint before/after and at commit time): component, docs, tools, build,
  20 typechecks pass; browser 578/578 passed, 0 skipped/unexpected/flaky.
  `origin/main` did not move from base `c94f4208`, so the merge tree equals the
  gated tree. Earlier failed and partial runs stay retained, never relabelled.

No deployment, bus operation, manufacturer contact, schema/dependency change,
private-corpus/Rust-runtime acceptance or release is claimed.
