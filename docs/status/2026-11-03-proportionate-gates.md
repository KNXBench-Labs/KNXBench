# Proportionate gate selection — 3 November 2026

## Published contract

Main-first **`703ec2904`** is published. Local, tracking and live `main` were
read back equal after the push. The planner, runner, bounded data-fact helper,
tests and [ADR-0144](../adr/0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md)
now select work by the actual changed inputs rather than treating every web
asset or embedded declaration as a functional KNX change.

- App-shell HTML and proven font-only dependency changes retain the complete
  web/default-browser checks. Their production-bundle selection can use the
  representative dialogs suite instead of all production suites.
- Font proof compares the dependency projections and relevant reader identities
  on both revisions. Unproven dependency, reader or format changes fall back to
  the conservative plan; this is not a package-path whitelist.
- Exactly bounded shipped-theme metadata selects an explicit nonzero
  `diagnostic-sections-tests` Rust stage. Functional or unsupported changes keep
  the ordinary dependent-package and corpus selection.
- Normal Rust selection supersedes the narrow diagnostic stage. Mixed plans
  execute Rust/bindings before diagnostics and then corpus, in the same order as
  the recorded stage contract.
- Unknown/mixed broad changes and `--full` retain conservative acceptance. This
  does not change release coverage, global leases, lane exclusivity or worker
  budgets. See [verification](../VERIFICATION.md).

## Review and regression evidence

The initial independent review found two concrete defects: mixed diagnostic
stage order disagreed with the finalizer, and Python JSON byte auto-detection
accepted encodings outside the pinned UTF-8 reader contract. Both were closed
with assertion-level regressions before the corrected candidate was approved.
The JSON proof now rejects unsupported encodings/BOMs, unsupported values and
non-regular old Git blobs instead of granting a narrow projection.

The corrected policy inventory passed **84/84** cases. Eight semantic mutation
controls failed at assertions with the same inventory and no setup errors;
restored sources passed again byte-for-byte. Original author, intermediate and
corrected inventories are separate evidence, not interchangeable counts.

A later integration exposed a test-fixture mismatch: the real CLI canonicalized
its root while the expected mock call retained a symlinked temporary path.
The test-only correction **`6593f481e`** canonicalizes the fixture without
weakening planner/cwd/scratch assertions or changing production resolution.
A new POSIX regression forces an inner symlink even with canonical outer temp
storage. It failed before the correction and passed afterward. The resulting
**85/85** cases passed with both canonical and symlinked temporary parents;
separate static review approved the exact source and diff hashes. The complete
tool suite ran **517 cases: 511 passed, six opt-in checks skipped** (one pinned
GTK-plugin-source check and five existing-native-helper observer checks).
Those optional checks are not claimed as executed.

## Source-bound acceptance composition

The original main-first candidate **`490b6d60e`** completed the full **10/10**
package stages with frozen inputs and valid proofs. Its measured results remain
identified as that source:

| Executed scope on `490b6d60e` | Result |
| --- | --- |
| Workspace Rust test stage | 8,064 passed, 247 ignored, 56 binaries |
| Binding-test stage | 62 passed, zero ignored, two binaries; diff check passed |
| Selected private corpus | 155/155 passed |
| Full web gate | 89/89 stages; default browser 1,009 functional + 69 exclusive |
| Full-gate supporting checks | formatting, desktop clean, Clippy, website and story stages passed |
| Separate actual diagnostic namespace | 15/15 Rust tests passed |

The separate diagnostic run is overlapping execution, not another set of unique
workspace tests. Ignored Rust tests are not counted as passed.

Upstream WP-27/WP-28 UI changes then required fresh main-first integration.
An alternate Git index composed the accepted source, the reviewed fixture patch
and the exact upstream binary patch. Its tree matched **`703ec2904`** exactly;
a deliberately mismatching composition was rejected. Production gate logic,
Rust, bindings and corpus inputs stayed unchanged. Only the reviewed fixture
changed under `tools/`. This permits carrying the original full results while
executing the maintained changed-input plan; it is not a second full Rust or
corpus run on the published successor.

The successor's **11/11** package stages and nested **42/42** web stages passed,
with frozen inputs, valid stage proofs and no touched or unchecked user data:

| Executed scope on `703ec2904` | Result |
| --- | --- |
| Build and maintained derived TypeScript configurations | passed |
| Full Vitest suite | 5,056/5,056 passed |
| Selected default Chromium, 73 spec files | 884 functional + 46 exclusive passed |
| Production devices | 70 functional + 1 exclusive passed |
| Production group addresses | 29 functional + 1 exclusive passed |
| Production table columns | 164 functional + 4 exclusive passed |
| Tool tests | 511 passed, six explicit opt-in skips |
| Supporting package checks | formatting, docs, lanes, layering, headers, anchors, ledger, corpus policy and clean diff passed |

Compiled browser inventories and once-only execution agree. No skipped, flaky,
retried or unexpected browser results are admitted. Corpus-policy validation
in this delta is not corpus execution. Focused and complete runs overlap and
must not be summed as unique coverage.

## Measured scope effect and retained limits

Replaying the actual Rail change selects **12 rather than 15 package stages**.
Its shared dialog-test changes still select all **16 production suites**;
the representative dialogs optimization is not applied blindly to the whole
mixed change. This is a plan comparison, not a measured wall-time speedup or a
retroactive reduction of Rail's already completed acceptance.

The first successor attempt stopped at two fixture assertions before any
browser stage and remains unaccepted. An intentionally deep-temp full-tool
attempt was also rejected by existing short browser-temp-root admission; its
positive focused phases are retained separately. Interrupted and observer-only
failures are not relabeled as product passes. Raw commands, exits, inventories,
review bindings, composition controls and original receipts remain private.

The shared root checkout was not updated by this delivery. Browser API/device
responses are fictional and intercepted. No real bus/device, native
WebKitGTK/Firefox/accessibility matrix, release or deployment acceptance is
claimed. Other [GATE-01 follow-ups](../open-work/GATE-01-test-gate-follow-ups.md)
remain open.
