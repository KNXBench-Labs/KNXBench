# Regular restore exemplar binding: bounded software acceptance

Observed: 29 October 2026.

## Accepted runtime scope

Main-first runtime integration `0ddfe49533ab159c427f9e2324afdcd1ebf82ccf`
merges implementation `d12a79f16b270f2d2b6ff339e8978a0581746d88` on the
source-reviewed contract/reservation `f5427e629`. Its runtime tree equals the
accepted implementation tree. Later result-documentation changes are not
relabelled as the runtime that was built or exercised.

The [contract](../contracts/regular-restore-exemplar-binding.md) adds an observed
Device Object PID 11 serial to regular pre-write backups. Public core restore
derivation inserts a typed exemplar precondition immediately before the first
mutation; every regular memory executor enforces it, including the restore's
own second backup observation. A replacement with the same address, product
and application must not receive the captured memory merely because those
other identity fields match.

This is a necessary observed-serial guard, not an absolute physical-identity or
whole-device recovery proof. Normal backed-up downloads may record unavailable
serial evidence without fabricating it; that evidence does not permit restore.
Unbacked downloads, comparison and preview do not add a serial observation.
Existing application, product, procedure, load-state, family, authorisation,
stop/commit, retry and wait gates remain in force.

## Compatibility and consumer boundary

- Backup format 3 adds required nullable `exemplar_serial`; `prior_application`
  stays required and nullable. Missing, duplicate, malformed, smuggled and
  unknown fields/versions are refused, not silently discarded.
- Formats 1/2 remain readable but are no longer restorable, **including partial
  backups**. Format 3 without a bindable serial, including all-zero/all-FF
  observations, cannot restore. No legacy bypass or downgrade is promised.
- Canonical source-procedure identity excludes only the typed synthetic guard,
  not genuine source `CompareProperty` steps. A backup made during a matching
  restore can still be restored under its own captured evidence.
- CLI restore uses the guarded core plan; CLI and HTTP backed-up downloads
  produce format 3. HTTP has no restore route. New serial evidence is confined
  to the private backup, not added to history, HTTP DTOs or reports. New
  guard/error/progress presentation is serial-free; this is not a universal
  no-leak promise for existing arbitrary payload dumps or opt-in traces.
- Legitimate second-observation failures retain their existing payload-free
  vocabulary: `propertyMismatch` / `identityMismatch` and `malformedProperty` /
  `malformedResponse` may occur in `backupBeforeStep` only with zero read-back
  write octets. Rust persistence and web parsing admit exactly those pairs.
  Durable history reopen and reports must not lose the optional cause.

## Executed acceptance

- Source contract reviewed independently and corrected before implementation;
  [primary-source facts](../research/2026-10-29-regular-restore-exemplar.md)
  remain distinct from the project's conservative replay policy.
- Original unsafe-replay semantic RED was executed before the guard GREEN.
  Additional real consumer tests later exposed a durable-history cause-loss
  defect; its semantic RED and targeted Rust/web GREEN are retained separately.
  Compiler, API-formation, provider-refusal and interrupted namespace-setup
  attempts are not semantic-RED or acceptance credit.
- Independent whole-feature review required stronger consumer proofs and
  bounded presentation claims. Fresh correction/whole-feature review accepted
  the final candidate with zero findings and closed all ten prior findings.
  The reviewer input's 3,887 tracked file contents/modes were authenticated;
  this is that review snapshot's inventory, not a count of later documentation.
- The maintained change-scoped branch gate at `d12a79f1` and main-first gate at
  `0ddfe4953` each passed **17/17 selected stages**, with completed plans, frozen
  inputs, valid per-stage proofs and neither touched nor unchecked user data.
- Integrated execution includes web build, owning fixture TypeScript checks,
  Vitest and functional/exclusive browser lanes (**948 = 881 + 67**), Clippy,
  **7,252 Rust tests passed / 0 failed / 238 ignored** across 49 binaries,
  62 binding tests, and **155/155 selected corpus checks**. Focused, binding,
  corpus and package populations overlap and are not summed into unique
  coverage. Ignored tests and unresolved historical opt-ins are not claimed run.
- An actual server binary was built from runtime integration `0ddfe4953`.
  Its version command returned `knx-server 0.1.0-alpha.18+g0ddfe4953` with exit 0.
  In a non-root, capability-free user/network namespace with loopback only and
  process-scoped HOME/XDG isolation, `/healthz` and `/api/version` returned 200;
  the index and its JavaScript were byte-equal to the built web frontend.
  An empty `/api/device-download/status` returned the expected 404. SIGTERM
  returned 0, the process was reaped and its listener closed. Source stayed
  unchanged and the checked real user-data paths were untouched and checked.
  This is startup/static-serving/status/shutdown evidence, not a device run.

Detailed original attempts, review inputs, execution receipts and licensed or
private-corpus provenance stay private. No licensed specification extract,
manufacturer package payload or real installation address is published here.

## Not closed

No bus, real device, analysis VM, deployment, release tag or release asset was
used or changed. The Alpha 18 release is not relabelled by this development build.
Matching observations do not prove absolute exemplar continuity, installation
function, all mutation effects or recovery from every interruption. Other
runtime layouts, memory encoding, inactive/Union/customer-write semantics and
historical opt-in fixture/category provenance remain independent requirements.
**PRG-01 and KL-007 remain open**; see the
[maintained task](../open-work/PRG-01-programming-gaps.md).
