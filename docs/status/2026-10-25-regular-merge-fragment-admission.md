# Regular MergeId fragment admission — 25 October 2026

## Scope

The regular memory planner conservatively refuses a declared `MergeId` fragment
beside exactly one main `ProductProcedure` load procedure
([contract](../contracts/regular-merge-fragment-admission.md)). This intentionally
narrows an earlier positive unit-test contract. It does not establish a KNX
format prohibition, fragment execution semantics or an incorrect device effect.

The first fragment is reported with its full-list, one-based source position and
raw ID, including empty or whitespace IDs. Kind and identity checks and
`ProcedureCount(0/2)` retain priority; fragment refusal precedes Connect/step
inspection. The application explicitly reports `procedure-shape` through its
real offline preparation. Shared read-only comparison, source freshness and
restore planning inherit this conservative compatibility restriction; their
barriers are not weakened.

## Executed focused evidence

- The original public ingest → image → planner `Result` regression compiled
  against the unchanged API and failed semantically on the old implementation.
  The retained binary was independently replayed: five intended failing methods
  and four passing controls. This is not a compile-failure RED or a hardware
  witness. An earlier fixture-control placeholder failure is kept separately
  and is not credited as that RED.
- Final ProductDB fragment selection: 10 passed. Actual application offline
  entry points: two passed. The explicit category unit also passed.
- Final source-retention characterization checks exact stored XML bytes after
  ingest, image construction and both refusals, plus raw fragment IDs, ordered
  known steps, unknown-step attributes and a reload from storage.
- The fragment-free control pins the complete literal plan and payload. Count,
  identity and failure-priority controls cover both planner entry points.
- Independent source review found no code blocker. Its discussion of plausible
  test mutants is static reasoning, not executed production-mutation evidence.
- The local same-base differential and source/input/executable integrity checks
  completed separately. Private input identities, measurements and fingerprints
  are not publication material here.

## Integrated acceptance and publication

Pending: the maintained package gate, its selected corpus scope, final
all-target Clippy on the corrected test bytes and publication readback.
Focused owning checks are not relabelled as a full workspace or package gate.

## Existing opt-in pin debt

The full-product opt-in coverage test already fails on its historical input-scope
pin ([KL-006](../limitations/KL-006-devices-dependent-on-vendor-plug-ins.md)). It is
not repinned here and is not included in a claim that every ignored test passes.
The programming-correctness track owns the attribution follow-up, with a review
checkpoint on **26 October 2026**, recorded in
[PRG-01](../open-work/PRG-01-programming-gaps.md#open). Any pin maintenance is a
separate, explicitly evidenced package; expectations must not be derived from
the observed output to make the test tautological.

## Retained boundaries

No parser, image, storage, verified registry, lockfile, dependencies, System-2,
UI or recovery policy changed. No new family, physical operation, deployment or
manufacturer contact. Secure is excluded. This slice does not complete PRG-01,
parameter policies, device baselines, mid-procedure stop or whole-mutation
recovery.
