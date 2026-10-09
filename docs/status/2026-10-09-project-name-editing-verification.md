# Project-local names: scoped verification, 9 October 2026

## Delivered behavior

Owner interview Q1–Q11 and subsequent Go define single device-instance and
individual group-address names in every installation. Name fields, F2 and
context menu share guarded name-only mutation, exact undo/redo and native
persistence. See [contract](../contracts/project-name-editing.md) and
[ADR-0101](../adr/0101-project-local-device-and-ga-names.md).

Names are not identities: IDs, numeric addresses, DPTs, flags, links, placements,
source refs and product/application names remain unchanged. Admission preserves
valid Unicode exactly; imported exceptions can be restored exactly. Failed
requests retain drafts. Revision/server/successful-load context rejects stale
writes; read-only reconciliation of unplaced canonical devices never auto-writes.
Pending commits cannot be dismissed as an unsent draft; editor generations reject
late replies even after switching away and back to the same entity.

## Executed local candidate gates

[Aggregate source-bound receipt](../evidence/project-name-editing-2026-10-09.json)
records file/log/screenshot hashes and measured counts. The candidate source
freeze is unchanged after acceptance; prose closure is separate.

- Rust: 1,950 passed, zero failed, 75 ignored across core, projection, store,
  server, app, CSV and diff. Ignored tests are **not** passes.
- Frontend: 2,544 passed, zero failed, 159 files; production build, production
  and fixture type checks, theme/flow/history checks pass.
- Clippy with warnings denied, server build and Linux desktop **compile check**
  pass. This is not a packaged native runtime or accessibility claim.
- Five nonempty repository gates pass: layering, headers, anchors, ledger,
  corpus-gate coverage. Documentation and Python tools (42 tests) pass.
- Intercepted Chromium regressions: 208 passed. Native production browser:
  four cases passed (DE/EN × 400/1440px) with actual built server/UI, conflict,
  undo/redo and save/reopen; no API interception. Namespace has only loopback,
  original fictional demo hash is unchanged, owned test server stopped.
- Source/security **self-review**, no independent-review claim. Added-source
  scan found no secret/shell-eval/HTML-injection/unsafe-deserialization matches.

## Actual visual review

Final built-app captures show a compact Name field, visible validation, focus
outline and reachable apply/cancel/refresh controls without dialog clipping at
both reviewed sizes. The narrow controls wrap inside the dialog; the broad
Graphite and narrow Porcelain variants retain the existing visual system.

![German compact rename dialog at 400px, with blank-name validation and all
three actions inside its bounds](../assets/screenshots/name-editing-rename-de-400.png)

![English compact rename dialog at 1440px with visible focus and validation
above apply, cancel and refresh](../assets/screenshots/name-editing-rename-en-1440.png)

## Failed controls and repaired harnesses

Do not count earlier failed runs as acceptance. Two old drag fixtures no longer
fit after adding Name: they now scroll before drag and assert both targets in
view; focused and full suites pass. One native 5-second dialog wait timed out,
while the trace recorded HTTP 200 at 5,139.985ms: the test wait alone is now 15s.
Visual review pinned a failing 630px-height dialog assertion, then a scoped
flex alignment made the short name dialog compact. Final four native cases and
all 208 regression cases passed after that last stylesheet change.

## Boundaries and publication

No bus access, new CLI/MCP write, bulk naming, CSV restructuring or `.knxproj`
export. This does not establish private-corpus, native-shell/Orca, power-loss,
hardware or ETS compatibility. Same-file native history is not an independent
backup; before first Save As it is session-only. Alpha.6 does not bundle this
newer source feature. Owner Go separately authorizes commit/integration/push,
not release/deployment. The final published revision is recorded in handover.
