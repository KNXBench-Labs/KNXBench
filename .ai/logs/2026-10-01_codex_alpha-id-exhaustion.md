# AR02 — IDs decline to circle back to zero

## Scope and baseline

- Offline alpha work on `alpha-id-exhaustion`, based on published AR01
  `57e9d9798bddb88178d498a77106e1a1cad6f127`.
- Source `DATA-01`; no Web source, protocol behavior, manufacturer schema,
  dependency, secret, private fixture or real KNX operation changed.
- ADR-0039 phases 1–2 and the scoped catalog preflight remain intact.
  Its parked phases 3–5 are not activated by this package.

## Implemented contract

All nine project-local allocators use checked addition and return typed
`IdAllocationError`. Zero is never issued; `u32::MAX` is the valid final ID;
repeat refusal leaves counters unchanged. The Rust API is intentionally
fallible; no native schema or HTTP DTO migration is involved. Every production
caller is in `knx-etsproj/src/map.rs`, `knx-csv/src/plan.rs` or server `domain.rs`.
Mapping returns no partial project on failure and public import propagates
`ImportFailure::Allocation`. Source bytes are untouched. CSV plans and offline
reconciliation use detached counters and do not apply partially planned work.
Existing parameter edits retain their ID even when new allocation is exhausted.

## Requirement / regression matrix

| Contract | Evidence |
| --- | --- |
| All nine final IDs; repeat refusal; no panic or counter change | `knx-core::project::tests::exhausted_*` |
| All nine final entities/counters survive native reopen unchanged | `knx-app/tests/id_exhaustion.rs::all_nine_final_ids_and_exhausted_counters_survive_native_reopen` |
| CSV batch with only one ID left is wholly refused | `id_exhaustion::a_csv_batch_that_needs_one_id_too_many_is_refused_without_consuming_any` |
| Existing edits at exhaustion remain possible | `id_exhaustion::exhaustion_does_not_prevent_an_update_that_needs_no_new_id`; HTTP parameter-panel regression |
| Maximum reservation rollback; maximum undo/redo/reopen | Last two `id_exhaustion` regressions |
| Mapper early ID tables / later communication-object allocation | Seeded synthetic mapper boundary regressions in `knx-etsproj/src/map.rs` |
| CLI dry-run and write both refuse with exit 2 and identical reopened project | `cli_group_address_csv::ga_import_exhaustion_exits_2_without_saving_existing_edits` |
| HTTP structural creation/CSV failure leaves project and history unchanged | New exhaustion regressions in `http_group_address_csv.rs` |
| Offline reconciliation does not apply pending deletion or consume planned IDs | `domain::tests::offline_reconciliation_exhaustion_preserves_deletions_and_allocators` |
| Parameter refusal versus existing edit, native equality | `http_parameter_panel::exhausted_parameter_ids_refuse_new_values_but_allow_existing_edits` |

## RED/GREEN and behavioral negative controls

Initial RED: all nine exhaustion tests fail behaviorally, no compiler error.
After implementation, focused core, CSV, mapper, native, CLI and HTTP tests pass.
Three independently compiled mutants are caught:

- wrapping addition: all nine per-kind regressions fail;
- CSV error downgraded to warning: partial-plan regression fails;
- exhausted new parameter reusing an existing ID: HTTP regression fails.

Mutated files were restored and hash-checked; restored focused tests pass.
Maximum-ID save/reopen is not a new full ETS compatibility claim. Mapper
maximum-counter tests deliberately seed allocators; they do not pretend to
allocate billions of real source entities or bypass ingestion budgets.

## Separate in-session review

Reviewed production behavior separately from test-fixture migrations, against
ADR-0039 and each source counter/consumer. No subagents and no external review
claim. IMPORTANT: whole-workspace compilation exposed the unmigrated direct
mapper call in `golden_reference_project.rs`; corrected its known-success
fixture with `.unwrap()`, not a production panic. MINOR: the new integration
file's second doc line made its first line an old-style paragraph rather than
an ADR-0018 header; inserted the required blank doc line. No ratchet relaxation.
No remaining blocking source finding in the reviewed candidate.

First full gate stops at that compilation error. Next run: 141 Rust result
blocks, 2,855 passed / zero failed / 161 ignored; Web 1,312; strict workspace
Clippy passes, but headers refuse the new file. Default projection binding
outputs are generated during tests; tracked Web bindings remain unchanged.
The source-freeze check now excludes the two existing generated binding
locations, not hand-maintained source. Those failed runs are not final green.

## Final verification / publication

Final gate `proc_fdbf662c1ca0` exited 0, completed before the recorded UTC clock
reading 2026-10-01 22:42 UTC. All 12 expected steps returned zero and hand-maintained
source remained unchanged throughout. 141 Rust result blocks / 2,855 passed /
zero failed / 161 ignored; 77 explicitly executed private corpus/roundtrip
tests across the six selected offline crates, zero failed/ignored/skip markers;
Web 1,312. Strict workspace Clippy, fmt, npm ci/build, headers, anchors, layering,
corpus policy and diff check pass. Separate `npx tsc --noEmit` and
`cargo deny check` exit 0; dependency policy has existing duplicate-version
warnings, not a claim of warning-free dependency analysis. Runtime scope names
the owned checkout: 371 valid headers / 160 absent / 34 generated; 324 Rust
corpus-policy sources; 375 anchors over 229 Markdown files before closing docs.
Tracked bindings are unchanged; generated default projection bindings are not
hand-maintained source. Owned fixture links were removed by the gate's finally.

Additional cold-target verification: `cargo clippy -p knx-core --all-targets
-- -D warnings` exits 0 in separate `ar02-core-proof`; the log explicitly
shows `Checking knx-core` at this owned source path, not a replayed result.
Final ledger check preserves all 180 IDs/priorities in both tables, historical
handover bytes and numbered limitation identity. Closing-doc anchor gate:
375 checked links across 229 Markdown files, zero dead.

AR02/DATA-01 status and model/limitation contracts synchronized. Reserved AR03
activation was offered explicitly; the prompt timed out with no response.
Empty input is not an activation or acceptance of continued deferral.
PENDING delivery: final staged/doc gates, exact remote publication readback
and owned artifact cleanup. Canonical-root statistics refresh stays blocked
by foreign report work. No release tag, hosted artifact, live bus, native GUI,
full compatibility or owner waiver claimed.
