# 2026-10-08 — Parameter workspace presentation

## Request and scope

Move evaluation warnings/notes out of the parameter editor into a tab beside
Product data, group repeated messages with a short reason, and put manufacturer
restricted fields in a separate tab. Implementation is UI-only, local and
uncommitted. No deployment, backend/core/DTO/migration/KNX write change.

## Implementation

- Five existing-pattern ARIA tabs, including keyboard wrapping/Home/End.
- One `useDeviceParameters` state in DeviceWorkspace; three thin views consume
  the same GET/write response, without refetching on tab selection.
- Pure grouping by kind/normalized severity/exact module scope/fallback message;
  original occurrence counts, ordered details, unknown-kind fallback and
  conservative severity retained. Technical details explicitly inspectable and
  all copied; a single copy payload is unchanged.
- Access Read/None fields separated in source order, disabled both at the
  control and write-handler guard, with short access-specific captions.
  Non-access refusals remain in Parameters, not misclassified as manufacturer
  restrictions. Actual action errors remain local.
- Stale stored values and product-language summary moved to Diagnostics,
  not discarded. Cause wording does not invent exact branch conditions or
  blame the manufacturer. Static unevaluated fields remain outside scope.
- Manual/status/architecture/ADR-0080 presentation amendment/Known Limitations
  synchronized. Contract: docs/PARAMETER_WORKSPACE.md.

## Evidence and review

- Actual RED: missing Diagnostics/Manufacturer tabs and repeated inline
  “A choice” text in the old editor. Recorded before production edits.
- Integrated root: 2,312 frontend tests / 139 files, production build and both
  fixture type gates pass. Tests retain normal numeric/module writes,
  refresh/language behavior, stale values, severity/fallback, clipboard records,
  manufacturer-read-only enforcement, and 10,000 repeated diagnostics.
- Actual Chromium CLI: 278 named checks, EN/DE × 400/1440px × shipped
  Porcelain/Graphite/LCARS CSS. Exact 24 synthetic intercepted feature requests
  (12 GET + 12 POST), zero browser errors/unexpected/external requests.
  Shared updates, all six detail records, clipboard unit payload and all five
  keyboard tabs covered. Graphite desktop and LCARS narrow screenshots inspected.
- First browser harness run focused an inactive tab without selecting it;
  corrected the harness to activate/assert that tab before ArrowRight. This
  was not treated as product acceptance or used to change keyboard logic.
  Initial fixture without theme bootstrap was also not visual acceptance;
  complete three-palette run is the accepted proof.
- Fresh xtask build with explicit root: layering 472 packages, headers
  637 well-formed/155 existing headerless/42 generated, anchors 685 links/327
  markdown files, ledger 191 rows, corpus-gate source lint 416 Rust files.
  All five exit 0; diff check green. No actual private-corpus execution claimed.
- Source self-review only, no independent review or native WebKitGTK/Orca/full
  accessibility, broader ETS compatibility or hardware certification.
- Durable source hashes, browser assertion/request ledger, frontend and gate
  receipt: docs/parameter-workspace/verification.json. Nine changed source
  files are byte-equal between exercised candidate and integrated root.

## Delivery boundaries

Root was deliberately older/dirty at e99a94e9. Isolated candidate copied only
existing frontend state, then the reviewed own delta was applied to root;
foreign frontend/docs/website/handover edits were preserved. Root HEAD and
index were not staged/reset/committed. Main sync/publication and container
activation need their own coordinated request and gates; the running instance
is unchanged. Clean only this package's own browser/server/worktree/scratch,
not foreign active previews, worktrees, data or rollback containers.

Closure 2026-10-08 12:14 CEST: own Chromium session and fixture process stopped,
loopback4194 no longer listening; own isolated worktree and named scratch/build
removed after receipt/source hash readback. Nine accepted sources retained,
root HEAD e99a94e9 and unstaged index unchanged; no deployment/publication.
