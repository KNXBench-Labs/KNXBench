# Import expansion source integration — 10 October 2026

## Source and authorization

The owner authorized commit, non-fast-forward merge and push after the completed
local implementation. The combined candidate has main parent `550318d3e7faef97a163d37da84ca7fce8ab28f7`
and feature parent `99e054250a98aae0b7268f81728f16000123cd21`. It was assembled and gated in an owned
checkout; the dirty shared root was not synchronized or staged. Three additive
documentation conflicts retained both sections; no code conflict.
The original [local receipt](../evidence/import-expansion-local-2026-10-10.json)
is unchanged historical evidence, not relabelled as a merge run.

## Executed combined-source acceptance

The [integration receipt](../evidence/import-expansion-integration-2026-10-10.json)
binds 1,075 application/source/configuration inputs with commitment
`9b5e1b54cc6415f4228a212c190bee2eaf47960782921a4d3b9f32d0f2c16cc0` (sorted compact JSON mapping). Those inputs remained identical
through acceptance; later closure edits are documentation/handover only.
Tests exercised the merged source tree before the merge commit; build stamps
identify its parent plus dirty state, not a released artifact or the later commit.

- All **19** stages exited zero: production frontend build, formatter,
  warnings-denied workspace/all-target Clippy, whole ordinary Rust workspace,
  Python tools, whole frontend, theme/Flow types, full Chromium fixtures,
  workspace build, selected private corpus, five explicit-root repository gates,
  documentation, working and staged whitespace. Process completed with exit zero.
- Rust: **3,856 passed / 0 failed / 186 ignored** in 237 result blocks.
  Ignored tests are not accepted private/hardware coverage.
- Frontend: **2,592 passed / 0 failed in 163 files** (actual Vitest JSON).
- Python: **54 passed** including the seven CVEXC analyzer regressions.
- Intercepted-API Chromium fixtures: **236/236**.
- Registered private corpus: **143/143 in 31 targets**. Original corpus/oracle
  were linked read-only for the selected run, then links removed. This does not
  supply the missing authorized historical-revision export set.
- Three further TypeScript configurations (history, rename, communication)
  passed separately; theme and Flow configurations belong to the main pipeline.
- Actual workspace-built server/production frontend: **four** fresh Chromium
  contexts (en/de, 1440/400px), public synthetic source, actual picker/source/
  preview/consent/apply APIs, read-only preview and exact installation Undo.
  **Eight** final-build screenshots inspected: readable ID-bearing labels,
  wrapped privacy/consent/mapping text and usable scrollable actions. Measured
  dialog horizontal overflow is zero. Backend diagnostic notes remain English.
  Running executable SHA-256 matched the retained artifact binding.

## Review and excluded attempts

Review is **self-review**, not an independent audit. Local feature application
inputs matched their closed historical review; actual merged UI/routes and
upstream deltas were inspected. No unresolved blocking finding in this scope.
An early whitespace tool call timed out and an initial observer/initialization
attempt failed without a retained full diagnostic; neither is accepted evidence.
The first successful browser run used the test-build server. After workspace
build produced a different binary, a new disposable environment exercised that
exact workspace-built artifact; original successful evidence remains separate.

## Delivery and retained limits

Final explicit-root repository/documentation and both whitespace checks are
required after metadata closure, before normal push; exact remote ref and merge
ancestry are verified locally and recorded in the handover. The shared root's
HEAD, index and foreign content remain protected; its stale local main is not
silently fast-forwarded. This does not refresh the historical ProjectStats report.

**IMPORT-02 remains externally blocked** on authorized same-project revision
exports and independently reviewed baselines. CVEXC declarations do not establish
normative signatures/defaults/precedence or enable runtime device policies.
No new release, deployment, native accessibility acceptance, ETS certification
or real bus action. Existing alpha.7 artifacts are unchanged.
