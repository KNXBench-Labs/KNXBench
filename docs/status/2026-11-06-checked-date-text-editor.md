# WP-08 checked Date text editor — 6 November 2026

## Scope and acceptance at candidate admission

The [bounded contract](../contracts/checked-date-text-editor.md) adds an opt-in
checked text editor for one exact source-authenticated Date declaration. The
existing server calendar validator, revision-bound write route and domain/storage
commands remain authoritative. No native date picker, image codec, schema change,
new year-range policy or physical compatibility is implemented.

The implementation and maintained tests cover exact source admission, Gregorian
century/leap boundaries and four-digit years, invalid historical values, raw
newline paste, explicit Apply, write restrictions, refusals/retry, stale context,
late outcomes and the production ParameterPanel/host adapter join. Generic text
fields remain available when specialized evidence is unavailable.

## Executed focused evidence

- Actual fixture TypeScript command succeeded. The first focused unit invocation
  retained two new fixture/oracle errors: pending-button label lookup and an
  incorrect expectation that missing specialized identity disables the ordinary
  field. Corrected fixtures pass all 244 tests in six selected files. These are
  not original-source production defect REDs.
- Independent whole-feature review: APPROVE, zero Critical/Important/Minor
  findings. Isolated original/restored controls each pass 244 tests. Three
  independently executed semantic mutants fail their intended assertions:
  Gregorian-century handling, exact Encoding admission and Date clipboard
  trimming. These are discrimination checks, not pre-feature regression results.
- The initial supplied review patch was truncated on capture. It was not used
  for acceptance: a complete read-only candidate diff was recovered and all
  23 reviewed material files were authenticated before isolated execution.
  Parent independently checked the raw exits, assertion diagnostics, per-run
  input deltas, recovered-patch digest and unchanged candidate.
- The measured development-browser precheck passed all 34 selected stages,
  including build, 27 fixture TypeScript projects, 5,293 unit tests and all
  40 default-lane cases from `parameter-editors.e2e.ts`. Ordered proofs and
  unchanged source fingerprints were verified. This was an uncommitted targeted
  precheck, not final committed-package acceptance.
- A separate visual-only invocation reused repository leases, offline browser
  launch and the unchanged source. All four EN/DE × 1440/400 Date cells passed;
  eight invalid/applied editor crops were inspected, with no observed clipping
  or overlapping controls. The maintained gate deliberately strips capture
  environment switches, so its green run alone did not supply those pictures.

The browser fixtures use intercepted HTTP and synthetic value/history state.
They do not execute server Date validation or prove native persistence. The
review did not run browsers; parent browser/visual evidence is separate.

## Committed software acceptance and publication

Feature `b19dfb131` was integrated main-first as `76dc954a1` on base `7cba23ff2`.
The feature and integration trees were identical. The maintained change-scoped
package gate ran once on that committed integration, then published it only after
all selected proofs passed and the upstream base still matched. Independent
parent readback confirmed local, tracking and live `main` equality.

All 14 selected stages passed, with the exact ordered plan, valid stage proofs,
matching source fingerprints, and no touched or unchecked user data. The nested
web gate passed all 37 selected stages: build, 27 fixture TypeScript projects,
5,293 unit tests, 959 default-browser cases (913 functional, 46 exclusive), and
40 production-parameters cases. Production/default cases overlap and are not
unique-test totals. The empty production exclusive lane was inventory-backed.

Clippy and 103 Rust tests in two actual xtask test executables passed. No
application-crate Rust suite was selected. Tooling reported 517 tests with six
existing optional skips. The corpus plan was explicitly empty because selected
packages hold no corpus test: no private-corpus execution is claimed. This is
complete selected-package software acceptance, not a full-workspace or release
gate and not closure of the remaining evidence boundaries below.

A later result-only documentation successor must preserve runtime input bytes
and pass its own maintained selected gate; accepted runtime execution remains
bound to `76dc954a1`, not relabelled as a fresh run on that successor.

Private original logs, manifests, failed setup/fixture attempts, independent
review, mutant patches, raw exits and visual captures are retained under
`<profile>/evidence/2026-11-06-wp08-checked-date-editor/` after closeout.

Broader WP-08/PDB-04, Colour/Picture codecs, module mapping, native WebKitGTK,
real Save/Open integration, device/image encoding, ETS, bus/hardware, release and
deployment acceptance remain outside this increment.
