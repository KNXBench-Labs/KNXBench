# S2-05 E15: changed-caption persistence failure — 3 November 2026

**SOFTWARE ACCEPTED — source `f5c65ea99`, main-first `99f7e65dd`.**
Independent review and both complete selected 14-stage package gates passed.
This fixture-only follow-up exercises a named portion of the caption rule's E15
obligation, not all of E15 or S2-05. It changes no production behavior, source
admission, recovery policy, timeout, production database schema or hardware permission.

## Executed scope

Four registered tests in server
`device_system2::tests::mounted_hardware::caption_mounted::persistence`
cross both selector families (standalone and retained members), namespaces
20/23, initially pending/previously programmed claims, unchanged effective
request/valid parameter edit, and failed/working persistence: **32 executed
matrix cells**, eight per registered test. The actual production prepare route
remains retained-only; standalone reuses the existing private fully prepared
source seam before the same mounted start admission and supervised worker.

Every case saves a fictional native project, admits the original start, pauses
after the first real simulated data write, and replaces the catalogue through
its managed owner with a Name-only source change. Strict old-source admission
refuses; working-persistence controls require successful full or selective
credit. No real bus, gateway, device, browser, GUI or private corpus is used by
this focused fixture run.

## Failure and preservation oracle

An ordinary SQLite trigger would fail existing schema admission too early.
Instead the fixture adds a generation-guarded expression index to its saved
history state. The current row remains readable; the next history UPSERT raises
SQLite integer overflow. All handles close before releasing the worker. The
terminal error must be exactly `project history storage: integer overflow`,
not a source, stale-history, permission or schema refusal.

Baselines are captured after fault installation. Failed persistence preserves
the entire live project/context, command stack, undo/redo snapshots, revision
and history generation, plus every typed row and schema object in a consistent
read-only saved-file snapshot. This is logical file preservation, **not SQLite
page/sidecar-byte identity, power-loss or crash durability**. Controls require
exactly one revision/generation advance, persisted working/history state,
unchanged saved root and removal of stale touched history claims.

Repeated status/report reads before and after fault removal do not retry
settlement. A genuinely fresh service/native open recovers exact working and
history content with `notRecorded`; explicit recovered undo/redo still reaches
the original targets after failure. Transfer facts remain Finished/Verified,
with every step complete; original observed bundle bytes and recovery binding
remain unchanged. No actual restore is executed.

## Focused execution and independent review

The parent ran the maintained Cargo library target with filter `caption_mounted`
and two test threads: **11 passed, zero failed, zero ignored**, including the
seven existing caption tests and four new tests above; runtime 198.28 seconds.
The source-bound runner recorded exit zero and unchanged inputs. It held the
shared workspace lease and used an offline loopback-only network namespace,
isolated XDG homes and dropped DAC-bypass capabilities. The actual executed
server test binary is retained with its digest in private evidence.

Independent review found no Critical, Important or Minor findings. Its baseline
and full restored run each passed the four new leaves and all 32 cells. Four
compiled wrong variants cover three defect classes: pre-persistence project/
revision publication, stack-only publication on failure, and committing partial
history changes despite the UPSERT error (plus a supplementary error-arm project
leak). Every variant failed the intended preservation assertion and retained
passing positive and restored controls; no compiler failure is counted as a kill.
The partial-commit variant exposed history-stack deletion while the other 34
tables, saved root, working state, schema and generation stayed unchanged.
Ten source-frozen independent invocations, their actual binaries and raw results
are retained; repeated controls are not additional unique coverage. The isolated
review copy independently bound the fixture/runtime bytes; original feature-base
Git provenance was supplied by the parent, not reconstructed in that snapshot.

## Complete selected acceptance and publication

Source `f5c65ea99` and main-first `99f7e65dd` have identical trees. Both frozen
package gates completed all 14 planned stages with direct exit zero and valid
proofs: formatting, documentation, browser-lane registration, layering, headers,
anchors, ledger, corpus-gate registration, tool tests, desktop cleanup, Clippy,
Rust tests, selected corpus and whitespace. `success`, `complete`,
`planCompleted` and `inputsFrozen` are true; checked user data was untouched
and no user data remained unchecked.

Each gate recorded **1,695 passed and 72 unchanged ignored Rust tests**, from
five actual test binaries and two doctest suites. The selected private corpus
separately passed **51/51 in two targets**. Tool tests ran 517 methods with
six existing opt-in skips. Prerequisite web builds passed; browser-lane
registration and `cargo clean -p knx-desktop` are not browser or native-GUI
acceptance. The gate built/tested its selected targets, not a release artifact.

The first package attempt failed the Chromium tooling check because dropping
all outer capabilities prevented its nested uid mapping. A focused correction
also initially refused the helper's overdeep TMPDIR. These environment refusals
remain separate failed attempts, not product failures or relabelled passes.
The corrected probe preserved nested user mapping while dropping DAC-bypass
capabilities and used the approved scratch root. The exact Chromium test and
both complete gates then passed without any source correction.

Publication was read back on local integration, tracking and live remote
`main` as `99f7e65dd`. The shared root checkout was intentionally not updated.
A documentation-only successor carries these results only with exact
non-documentation input equality and its own complete documentation-delta gate.
Original module-registration/format refusals are setup results, not failed
simulations. Private evidence is retained under
`<profile>/evidence/2026-11-03-s2-caption-persistence/`, including the original
`focused-r1/receipt.json`, `review/review-summary.json`,
`branch-gate-r2/receipt.json`, `integrated/gate-r1/receipt.json`, raw logs,
executed-binary bindings and `PARENT_REVIEW_VALIDATION.json`.

## Unchanged boundaries

The [original bounded acceptance](2026-11-03-s2-caption-settlement-candidate.md)
remains the production rule's authority. General source equivalence, exhaustive
profile/record/caller adversaries, actual changed-source restore, other terminal
lifecycle/failure combinations, races/read counts and large-project measurements
remain open. This is not whole-E15, hardware, release or deployment acceptance.

[Open task](../open-work/S2-05-result-and-settlement.md) ·
[E1–E17 obligations](../design/system2-settlement-source-equivalence.md)
