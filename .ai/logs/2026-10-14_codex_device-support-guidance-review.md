# Device-support guidance — reconciled self-review and acceptance

Session: `d03c0319b8c1`. Review is **self-review**; no independent reviewer,
subagent, hardware/ETS/manufacturer execution, external submission or release.
Goal B's published B15 release authorizes only the finite S1–S4 download/support
surface. B5/B16 and the pending B14 product-database work remain separate.

## Findings and closure

- **IMPORTANT — stale plan retention**, `DeviceDownloadPanel.tsx`, start refusal:
  a server `downloadPlanStale` refusal retained the old plan, inviting another
  invalid attempt. Regression `drops a server-refused stale plan but retains its
  reason and prepare-again action` failed at the exact retained-plan assertion
  (37 pass / 1 fail); clearing the plan/acknowledgement while retaining guidance
  made the focused five-file suite pass (86/0).
- **IMPORTANT — typed API response test**, `device_download_routes.rs` tests:
  an old plain-422 equality assertion contradicted the additive error envelope.
  Keep its exact error/status, assert the category/kind, and additionally cover
  procedure/module categories. Actual prior RED/GREEN evidence is retained in
  task-owned local logs; no production guard is weakened.
- **MINOR — byte-order wording**, dedicated EN/DE messages: the category covers
  every unsupported declaration, not only LittleEndian. Do not assert an unknown
  declaration is LittleEndian. Message expectations updated with the same wording.
- **IMPORTANT — configuration-grid placement**, `DownloadSupportNotice.css`:
  the first production screenshot showed the notice in a narrow grid cell.
  The feature-scoped notice now spans all columns. Added browser geometry
  assertion against the actual form's available width; its post-fix acceptance
  is pending in the frozen full gate, not implied by the earlier four passes.
- **IMPORTANT — practical manual route**, download chapter: expose the extra
  untested acknowledgement and unresolved executable-extension boundary, add
  pre-plan reporting instructions, and remove the blanket complete-scope retry
  suggestion for a partial-plan refusal. FAQ/troubleshooting point to that chapter.

## Source/security pass

Reviewed additive preparation/start error envelopes and unchanged refusal status/
messages; shared application category mapping; late project/device/scope responses
and consent; current readiness snapshots and B15 status/retry/handoff lifecycle;
no new executor or admission route. Refused plans cannot start through direct API.
Reporting targets are fixed application-owned URLs with zero project-derived
prefill. Raw error text is escaped local text; clipboard use is explicit and has
manual-selection fallback. The retained post-run verification workflow is separate.
Added-line scan found zero dynamic-execution, unsafe-HTML or password-literal
patterns. Private-looking test sentinels are fictional, not source credentials.
All original server/identity/key/freshness/backup/readback/exclusivity/stop and
ordinary/extra-untested confirmation checks remain authoritative.

## Actual acceptance so far

- Focused Vitest: **86 passed / 0 failed**, five files, reconciled candidate.
- Earlier production-build link-click browser check: **4 passed**, EN/DE ×
  1440/400, Graphite/Porcelain; fixed URLs, intercepted public targets and blocked
  mail dispatch, no plan/run/report. Its screenshots precede the grid-width fix.
- Documentation checker after practical chapter changes: **580 Markdown files,
  4837 local targets, 59 images, 33 chapters; zero errors**; external URLs not checked.
- Offline audit: **two executed tests / zero failed**, identical recursively
  privacy-checked aggregates: 105 packages, 467 programs, 1 existing Verified,
  98 Untested, 368 refused. All 467 executable dependencies unassessed.
  Public aggregate receipt and bounded F1–F5 source-backed follow-ups are in docs.
  Neither aggregate equality nor simulator tests prove input custody/hardware support.

## First complete attempt and corrective acceptance

The frozen `branch-gate-20261014-0238` attempt is **not accepted**: Rust 101
(one file-mode precondition failure), Chromium 505 pass / eight failed inherited
selector assertions, final whitespace 2 and frozen-input check false because
corpus tests regenerated eight bindings with trailing spaces. All other stages
passed, including 145/145 private tests in 32 targets and the four production
link-click checks. The complete failed receipt is retained locally.

- Namespace correction is external to product code: keep UID 1000, bring up
  loopback, then drop bounding/inheritable/ambient capabilities. Actual probe
  showed effective/bounding masks zero and protected write denied; all eight
  `name_recipe_writer` tests then passed without changing the test or its guard.
- Bindings were compared with `--ignore-space-at-eol` and their exact generated
  copies retained before restoring only tracked trailing-whitespace drift. The
  corrected runner repeats that comparison after corpus tests, rejects other
  drift, preserves a start fingerprint and refuses overwriting earlier attempts.
- `gateway-refusal.e2e.ts` now asserts the new notice's exact localized reason,
  exact support action, three links and geometry; monitor-stop/no-download
  assertions stay unchanged. The eight old selector failures are their RED.
- **IMPORTANT — invalid-address reason:** recognized `downloadInvalidAddress`
  used the correct action but a generic unknown reason. EN/DE regressions failed
  on that exact reason; the notice now gives an explicit target-address reason.
- **IMPORTANT — delayed clipboard response:** a copy completing after the
  displayed details changed could incorrectly report success/failure and select
  the next target's field. EN/DE delayed success/failure tests reproduced it.
  Copy feedback is now bound to copied text; manual selection only occurs for
  the same displayed text. All six added tests failed for those exact defects
  (six old tests passed); focused GREEN is **92 tests, five files, zero failures**,
  plus the feature's real TypeScript check exit zero.
- Inspected all four retained production screenshots after the grid fix:
  EN/DE at 400 Graphite and 1440 Porcelain. All are full-width, readable and
  unclipped; German narrow links wrap normally. These are intercepted-browser
  screenshots, not hardware/mail evidence.

A corrected complete branch gate is next; no failed first attempt is relabelled
as passed, and no publication has occurred.

## Pending delivery boundary

Run the complete mechanically frozen Rust/web/typecheck/browser/documentation/
repository/corpus gates, inspect corrected screenshots, commit only the reviewed
scope as KNXBench <github@knxbench.com>, reconcile newest upstream owners, verify
an isolated merged artifact, then normal push with exact remote readback. Update
handover/status with actual gate receipts only. The source candidate is not yet
published or released; no current working UI or device result is inferred from
this review alone. Preserve unrelated root journals and research branches.

## 2026-10-14 04:27 — corrected branch acceptance

Complete corrected attempt exited zero, all 35 stages zero and `inputs_frozen=true`;
current candidate's computed fingerprint equals the retained start fingerprint.
Rust 4411/0, 200 ignored (opt-in audit included); Vitest 3460/0, 247 files;
Chromium 513/0; focused browser 8/0; production download/report/scope/B15 specs
33/0; corpus 145/145, 32 targets. Python 122, one skipped; 15 tsconfig checks,
Clippy/build/bindings/final-binding comparison/documentation/xtask all zero.
Final binding copies were compared before only trailing-space drift restoration.
Current production DE 400 Graphite and EN 1440 Porcelain screenshots reopened:
reason/action and three links, privacy disclosure and local-copy field readable,
full configuration width, no clipping. No hardware/mail evidence follows.
Fresh main remains `00f73602`; its B15 release is explicit, other web surfaces
and B14 v23 remain Goal B-owned. Commits and isolated integration acceptance
are next; branch success alone is not a merged-result or push claim.

## 2026-10-14 07:17 — final merged runtime acceptance

`6bf86976` (includes published Goal B B14 and B2) passes all 35 stages with
frozen inputs: Rust 4442/0/200 ignored; Vitest 3504/0, 249 files; Chromium
522/0; production download/report/scope/B15 33/0; focused browser 8/0; corpus
145/145 in 32 targets; Python 122, one skipped; all 15 tsconfig checks,
Clippy/build/bindings/final binding/docs/xtask/whitespace zero. Exact current
tracked input hashes equal the retained immutable manifest. Final production
EN 1440 Porcelain and DE 400 Graphite screenshots reopened and inspected:
readable full-width notice, reason/action, three links and local/private-copy
controls, no clipping. Self-review only; no independent/hardware/mail claim.

The earlier integrated attempt's corpus disk exhaustion remains rejected. Its
unchanged-source recovery separately passed 145/145 before B2 was merged; the
new complete gate is a distinct execution, not rewritten old evidence.
Prior aggregate-audit counts are not relabelled as a new B14/B2 census.

Remaining delivery actions are documentation-only closure checks, normal push
and exact remote readback, then verified owned cleanup. All device-family
follow-ups remain outside this finite goal; no new goal/release/deploy starts.

Documentation-only closure passes documentation checker, Python tool suite
(122 executed, one skip), anchors and ledger; whitespace zero. Every non-doc
tracked blob matches the immutable accepted runtime manifest. Normal push and
exact remote readback are the remaining publication boundary.
