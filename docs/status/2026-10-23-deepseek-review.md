# DeepSeek-era changes: review of 22–23 October 2026

## Verdict and scope

**Changes requested before production catalogue-job wiring.** Five code defects
were reproduced with seven adversarial tests against the published implementation
(test-only additions, no implementation fixes). A separate documentation failure
is present at the reviewed tip.

Reviewed revision: `e6369faa5085013c7ce5411c9d525c4a673fd4cf`, fetched and checked
against live `origin/main` on 23 October 2026. The approximate 24-hour scope starts
after `d261c09ce`: CatalogueOwner integration, transitive installer cancellation,
job admission/DTO/spool/artifact/lifecycle primitives, binding/identity/orchestration,
System-2 full-table identity and consent-gated hardware paths, and the Vite/Vitest
update. The selected main checkout was stale; review and probes used isolated
checkouts. Unmerged route-wiring work was not adopted or changed.

This is an in-session source and executable regression review, not a new KNX-spec,
private-corpus, browser-E2E, hardware or release acceptance. Existing owner-approved
best-effort hardware risk in ADR-0152 is not itself classified as a new bug.

## Findings

### R1 — P1: apply releases its validated writer reservation before live DML

**Locations at the reviewed revision:**
`crates/knx-productdb/src/managed_catalogue.rs:594–602, 651–720`;
`capture_stamp` explicitly rolls its transaction back at `423–427`.

Apply compares the fresh stamp with the shown stamp, releases that reservation,
opens/copies S, closes S and only then starts a different W IMMEDIATE transaction.
An ordinary external SQLite writer can publish in this gap. The later install-report
identity comparison does not cover unrelated catalogue rows and cannot replace the
full original catalogue stamp.

**Executed witness:** an external connection obtains a RESERVED writer transaction
after stamp validation, updates an unrelated manufacturer row and commits as S
closes. The external row change is read back. Apply still returns `Committed`.
`review_apply_refuses_external_commit_after_stamp_check` fails with
`Committed != NotCommitted`. Channels coordinate the interleaving; no timing sleep
or real catalogue is used.

**Required correction:** compare the shown full stamp under the same W IMMEDIATE
reservation that remains held through copy/recheck, live DML and settlement. Preserve
the specified W/S/D/B lock order and external-writer detection; do not merely widen
the install report or take another unlocked sample.

### R2 — P1: failed rollback still permits recovery-backup disposal

**Locations:** `crates/knx-productdb/src/managed_catalogue.rs:744–795, 900–903`.

Installer failure, preview-identity failure and report mismatch inspect a `clean`
boolean for writer health, but return `rollback: Explicit` and `artifacts: Dispose`
even when rollback did not end the transaction. Quarantining W while telling its
caller to discard recovery is contradictory and violates recovery retention.

**Executed fault-injection witness:** a test-only SQLite authorizer is installed
after stamp acquisition. It refuses installer INSERTs and the following ROLLBACK.
The native connection is confirmed still in a transaction and health is quarantined,
but settlement instructs `Dispose`. `review_failed_rollback_must_retain_recovery_artifact`
fails with `Dispose != Retain`. This is deliberate native-error injection, not a
claim that a real filesystem failure was observed.

**Required correction:** propagate actual rollback disposition on every early-return
branch. Unresolved settlement must retain recovery and quarantine; never report an
explicitly successful rollback when that operation failed. Keep no-COMMIT evidence
separate from successful rollback evidence.

### R3 — P2: drain loses authoritative ownership while awaiting

**Location:** `apps/knx-server/src/catalogue_jobs/owner.rs:65–83`.

`drain` removes the only tracked JoinHandle into its local future. A concurrent
drain sees an empty slot and returns `nothingRunning / workOutstanding=false`.
Dropping/aborting the first drain future drops and detaches its worker handle;
subsequent drains make the same false claim. This is a lifecycle integration blocker,
not a demonstrated production shutdown failure: the owner is still unwired.

**Executed witnesses:** both
`review_overlapping_drain_must_not_report_no_work` and
`review_dropped_drain_must_retain_the_running_worker` fail. The worker is held by a
Notify and the test observes handle transfer before invoking the second drain.

**Required correction:** retain authoritative in-flight/draining state and make the
join ownership cancellation-safe. Serialize overlapping drains or return a truthful
outstanding state; do not abort a blocking installer as a shortcut.

### R4 — P2: apply does not compare the retained policy identity

**Location:** `crates/knx-productdb/src/managed_catalogue.rs:580–602`.

The method compares current source limits with current operation limits and compares
the source identity with the shown source. It never compares `shown.policy` with
the current policy, nor validates `shown.version`. Consequently a Standard preview
can be applied with Large source admission and a Large operation policy when the
small fixture happens to produce the same report.

**Executed witness:** `review_apply_refuses_policy_changed_since_preview` obtains a
real Standard preview, switches both current inputs to Large and observes
`Committed != NotCommitted`.

**Required correction:** refuse a retained binding-version or effective-policy
mismatch before any copy or live work; add both change directions as regressions.

### R5 — P2: status assembly bypasses nested preview/result validation

**Locations:** `apps/knx-server/src/catalogue_jobs/dto.rs:1033–1041`;
`apps/knx-server/src/catalogue_jobs/status.rs:82–108`.

`Status::validate` invokes source/profile/commit/cleanup/backup/apply validators,
but not `Preview::validate` or `JobResult::validate`. `assemble_status`, which
claims to admit the complete status, therefore accepts nested values that their
own validators reject.

**Executed witnesses:** a valid succeeded fixture first passes assembly. Setting
`preview.growth_cliff_possible=false`, or replacing the result artifact kind with
`Preview`, is independently rejected by the leaf validator but accepted by actual
`assemble_status`. Both `review_status_assembly_refuses_invalid_nested_preview` and
`review_status_assembly_refuses_wrong_result_artifact_kind` fail.

**Required correction:** validate all present nested objects at the authoritative
assembly boundary. Audit phase/null/lease rules too; this review reproduced the two
named nested cases, not exhaustive protocol validation.

### R6 — P2: current documentation gate is broken by a later deletion

**Introducing commit:** `e6369faa5`, 23 October 2026, 08:18 CEST; Git records author
`KNXBench Labs` and committer `GitHub`. Do not attribute this later commit to the
DeepSeek implementation merely because it is at the review tip.

`docs/support/issue-22-up258-evidence-request.md` was deleted while six callers
still reference it: IMPLEMENTATION_STATUS, OPEN_WORK, ROADMAP, KL-007, and two
16 October research documents. `python3 tools/check_documentation.py` actually
exits 1 and reports those six missing targets.

**Required correction:** restore the document or deliberately retarget its callers,
preserving the relevant historical evidence. Do not disable link checks.

## Verification actually executed

On the unmodified reviewed implementation:

- `cargo test --offline --locked -p knx-productdb`: exit 0; 591 library and
  752 integration passes, plus example/doc executions. The raw runner accounts
  for 1,371 passed executions and 32 ignored executions including its example lanes.
- `cargo test --offline --locked -p knx-net -p knx-app -p knx-cli -p knx-server`:
  exit 0; 2,586 passed executions, 128 ignored.
- Combined **3,957 successful Rust test executions**, not a claim of 3,957 distinct
  tests or a complete workspace gate. Private-corpus lanes were not enabled.
- All-target Clippy for those five packages with `-D warnings`: exit 0.
- `cargo fmt --all -- --check`: exit 0.
- Fresh locked `npm ci --ignore-scripts --no-audit --no-fund`; actually installed
  Vite 8.3.5 and Vitest 5.0.5. The stale checkout's older installed dependencies
  were not reused as evidence for the bump.
- Vitest JSON: `success=true`, **287 files / 4,189 passed / 0 failed**.
- `npm run build` (`tsc && vite build`): exit 0, 376 transformed modules.
- Documentation check: exit 1, six missing targets as described above.

On the separate test-only probe checkout:

- `cargo test --offline --locked -p knx-productdb --lib '::review_' -- --nocapture`:
  three intended assertion failures, no ignored or setup failures.
- The corresponding `knx-server` command: four intended assertion failures.
- These **seven failures reproduce five code defects**. They are not failures
  of the existing unchanged test suite and are not repaired by this review.

## Boundaries and follow-up

- Catalogue job HTTP routes/worker/desktop lifetime wiring are not delivered on
  the reviewed main. Fix R1–R5 before exposing those capabilities. Previously
  documented copy-witness, growth-limit and commit-barrier deferrals remain open;
  this review does not silently accept them.
- The System-2 path is now user-reachable through CLI and remains explicitly
  best-effort. Existing admission/refusal/simulator tests pass, but no real
  device, bus or gateway was contacted, and no ETS-equivalence or physical recovery
  acceptance is granted here.
- Source-traced follow-ups, not separately reproduced runtime bugs: CLI preview
  still says no real transport is enabled (`device_system2_validation.rs:138`),
  current architecture/open-work prose retains blanket hardware refusals, and the
  ADR's user-selectable external-ETS versus KNXBench recovery choice has no CLI
  selector. Reconcile these with the actual shipped consent-gated path.
- No implementation fixes, commits, pushes, deployment or release were performed.
  The selected checkout's pre-existing modifications and untracked documents remain
  untouched apart from the required additive handover and this review record.
- Private raw logs, exact probe sources, the complete test-only patch and retained
  executed binaries are archived under
  `<profile>/evidence/2026-10-23-deepseek-review/`. Public tracked prose contains no
  machine paths, device data or private source extracts.
