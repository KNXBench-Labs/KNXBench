# AR01 — runtime verification targets, no phantom worktrees

## Baseline, scope and review

- Agent: codex (GPT-6.1-Sol through Hermes); no delegation or live KNX action.
- Baseline `6f4cef2469d8cd051ce46bcc09ffb55fdef6c68f`, AR00 publication:
  exact HEAD/origin/main equality and published dossier byte comparison passed.
  The clean owned `alpha-queue` checkout and branch have been removed; foreign
  root edits and worktrees remain untouched. Statistics refresh is still
  blocked by the canonical root's foreign report/history synchronization.
- Owned `alpha-gate-scope` changes only repository-verification code/tests and
  focused documentation. No product protocol/domain/DTO/UI source changed,
  no dependency upgrade, release tag or hardware permission.
- Separate in-session complete-diff review found a missing touched-module
  header and graph-root presence that could accept a transitive-only package.
  Both fixed; header ratchet lowered to measured 160. Tiny Cargo fixtures now
  use exact exclusion paths and assert their real metadata membership; an
  initial broad exclusion had not established the intended external shape.
  Corrected fixture is covered by a behaviorally failing membership mutant.
  No remaining blocking finding. This is not an independent-agent review.

## Change and invocation contract

- Gates select exact CWD or leading `--root PATH`, canonicalize it, validate
  the workspace and named member paths and print the chosen target. No baked
  build-root fallback or parent climb. Metadata is pinned to that root.
- Header scan requires non-generated sources in every defined root; anchor
  scan requires docs Markdown; corpus lint requires Rust in apps and crates.
  Missing/read-error/empty roots fail. Intentional nonempty fixture scopes
  work without a production bypass. Zero links in actual documents is valid.
- Layering requires every checked root to be a workspace member and resolved
  node. All scans/graph emit actual coverage, not today's magic counts.
- Existing AppImage options/artifact checks and freeze-fixture storage helper
  retained. Read `docs/VERIFICATION.md` for residuals: caller-selected wrong
  *valid* checkout, concurrent mutations, textual heuristics and pre-AR01
  executables are not magically certified by a nonempty count.

## Actual RED/GREEN evidence

- Initial CLI regression failed behaviorally: unrelated runtime CWD silently
  audited the baked build tree. Missing-source header regression failed;
  corpus/anchor empty-scan regressions failed; missing graph roots and ignored
  misplaced options failed. All pass after implementation.
- Real old AR00 binary, after its owned build tree was deleted, reported
  `headers ok: 0 ... 0 ... 0` and exit 0. New compiled executable with the
  deleted explicit root exited 1 and reported target unavailable.
- Focused mutation sweep: baked-root revert, weak header-empty check, disabled
  anchor-empty check, weak corpus-empty check, removed workspace membership.
  All five compiled and failed named behavioral regressions (exit 101 / failed
  test, not compiler errors); original sources restored byte-identically.
  A sixth extra candidate had no unique anchor and was rejected before any
  write/run; it is not counted as tested or caught.
- Restored candidate: 75 unit + 12 CLI integration tests, zero failures and
  zero ignored; strict `cargo clippy -p xtask --all-targets -- -D warnings`,
  workspace fmt, build and four runtime gates passed.
- Initial final runtime scopes: layering 448 packages, headers 370 valid / 160
  absent / 17 generated (ceiling 160), corpus lint 323 Rust files, anchors 375
  links / 229 Markdown files. Later doc-only gate repetitions can increase
  anchor totals; no count is hardcoded as a gate success condition.
- 180 unique IDs and unchanged priorities/routes match goal/dossier; 110
  headings / 104 residual / 103 classified (5/30/54/14), §105 unclassified.
  Inventory bytes unchanged. KL-130-GATE closed, zoom identity stays open.

## Evidence boundaries / delivery

This is the applicable **xtask/verification** gate, not a full workspace product
suite, private-corpus execution, native platform acceptance or compatibility
certification. Doc/whitespace checks are repeated after handover and any
upstream integration. Publication requires remote ref/artifact readback; no
publication is claimed ahead of that action. Next ready package is AR02.
Canonical-root statistics refresh remains explicitly blocked, not fabricated.
