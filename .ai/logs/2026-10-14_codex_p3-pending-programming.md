# P3 review and implementation continuation — 2026-10-14

## Scope and mapping

Candidate: feature/p3-pending-programming-20261014, core 0f2c1433 plus
uncommitted App/server/projection/web. Owner decisions in
`docs/goals/ui-feedback-p1-p4.md` are fixed; P4 is excluded.

- Store preflight: `crates/knx-store/src/devices.rs:33-79` writes all five
  loaded booleans; the reader restores them. No migration is needed or added.
- Edits: `CommandStack::do_command` is the shared knx-edit/knx-app boundary;
  compare before/after candidate device content and fold marks into its inverse.
- Server download: `device_download_routes::start` prepares full/partial plans;
  `device_download::run` observes the terminal result before status publication.
- Server address programming: `address_programming_routes::start` starts the
  worker; `address_programming::run` settles before status publication. Public
  hardware start remains fail-closed for existing recovery reasons; simulated
  paths are tested. This package does not relax that gate.
- Projection: `knx_projection::DeviceNode.pending_programming` is derived from
  the persisted booleans. The catalog admission helper also validates the
  optional new field. ProjectTree itself has no strict client admission parser.
- CLI: `device_download::execute_with_history` runs the same memory plan but
  the CLI does not own the server's open project or save edits to the input
  project. `device program-address` has no project argument and remains
  fail-closed publicly. CLI/external programming does not silently mutate an
  unrelated server session; manual marking is the explicit reconciliation path.

## Separate in-session whole-feature review (before fixes)

No subagent/independent approval is claimed.

### IMPORTANT R1 — interrupted runs overmark unvisited parts

`crates/knx-app/src/programming_state.rs:109-112` uses every scheduled memory
part, even when failure stopped before those parts were reached. The server
already has the one-based stopped step and typed memory plan. Derive touched
parts from attempted load-control machines/segment writes; include the failing
write because it may have reached the device. Preserve conservatism for
unattributed writes. Test a failure in the first table before application steps,
partial parameter-only work, and read-only prefixes.

### IMPORTANT R2 — address settlement resolves identity too late

`apps/knx-server/src/programming_state.rs:262` resolves only the device that
holds the target address at completion. If the project reassigns that address
while programming waits, a different device could get a false clean claim.
Capture the sole device at start and require that exact identity/address still
match at completion; a missing or initially ambiguous identity must not become
an implicit target later. Add a regression before fixing.

### IMPORTANT R3 — failed UI refresh is swallowed

`DeviceDownloadPanel.tsx` and `AddressProgrammingPanel.tsx` ignore rejection
of `GET /api/project` after writes. A user can therefore keep seeing stale clean
chips without any warning. Surface a localized refresh warning independently
of the physical write outcome; do not reclassify the write or auto-retry it.
Test each panel's rejection path.

### MINOR R4 — missing mutation and durable lifecycle evidence

Focused coverage is present, but explicit mutants for marking/clearing and a
native save/reopen assertion for the new behavior remain to be executed. Add
and retain those results before acceptance, plus the required documentation.

## Performance evidence boundary

The earlier six successful 5000-device runs used development Vite; their times
are diagnostic, not production-performance acceptance. Production attribution
is queued in task-owned scratch, holding the shared leases, with baseline/full/
no-chips/no-dots variants and renderer traces. No unrelated virtualization or
performance refactor is authorized by a noisy single-sample wall-time result.

### IMPORTANT R5 — unexpected worker termination bypasses settlement

`device_download.rs:268-283` synthesizes a failed status after a task panic,
but the new completion callback runs only on the normal return path. Keep the
one-shot callback in shared session state and run it for the existing fallback
terminal status too, before that status can be observed. Prove exactly-once
settlement after a post-write panic without altering the existing tunnel gate.

### IMPORTANT R6 — unconfirmed restart incorrectly blocks verified-memory clearing

`programming_state::download_end` treated a finished, fully read-back write
with an unconfirmed closing restart as a separate unverified download. Existing
worker/CLI semantics explicitly call this a finished write, with a separate
restart warning. The owner's full-download rule must clear verified parts here
too; restart confirmation is not another owner decision or a new compatibility
claim. Keep its warning, remove the extra flag-clearing condition, and test it.

## Closure before the full branch gate

- R1: named RED for the read-only/early-failure boundary; App settlement 16/16
  and all 29 private download HTTP cases pass. Failing writes are included,
  unreached parts excluded, unknown attribution stays conservative.
- R2: named RED confirmed the wrong device acquired a clean address claim;
  capture/finish identity checks now pass all 15 address HTTP cases.
- R3: both refresh-rejection tests were RED; both panels' suite passes 59/59.
- R4: all nine actual mutants have named behavioral failures. The original
  wrapper exited 1 because ANSI controls hid the UI failure from its parser;
  raw output/exit preserved, corrected parser tested with five controls. The
  subsequent eight restored-green/clippy/header/corpus stages all pass. Native
  restart/undo and all 16 distinct flag patterns pass in six HTTP tests.
- R5: post-write task-panic settlement was RED; all 11 worker lifecycle tests
  pass, with exactly-once fallback before terminal status publication.
- R6: restart mapping was RED; finished-memory clearing is now independent of
  the existing restart warning, including the private HTTP regression.
- Full private corpus: 149/149 in 32 targets, network-isolated; no live bus.
- Existing recorded-state view clears itself on snapshot-revision changes;
  it does not retain a conflicting old loaded-flag presentation.

Performance attribution used six production runs, then two separate twelve-run
counterbalanced comparisons (two per state/variant: absent, clean, all pending).
Caching only seventeen immutable cell presentations reduces repeated work and
markup. Deferring those new nodes until the first visible table visit keeps
existing filter/scroll/DOM after later editor visits. Its named regression was
RED then GREEN. In its controlled all-pending comparison, mean RunTask falls
23204.2→16455.1 ms, project open 18209→11449 ms; table open 4542→4551 ms.
Host load/raw traces and restored base witnesses are retained in scratch.
These are synthetic 5000-device limits, not universal throughput, GPU or whole-
gate speedup claims; no virtualization or unrelated table refactor was added.

Final source self-review: Critical 0, Important 0, Minor 0 remaining. Static
review is closed. The complete branch acceptance follows below; merged acceptance
and publication remain PENDING.

## Branch acceptance — 2026-10-15

The preserved initial runner exited4 before tests: generated ts-rs banner108
triggered the handwritten-header preflight. Do not edit generated text. All28
actual bindings regenerated equal (only known end-of-line formatting restored);
canonical directory/banner exception has three live positive/negative controls.
The regular repository header gate remains unchanged and passes. The reusable
skill template now records/cleans its owned short temporary directory on EXIT.

Distinct retry proc_23489c9ea0d0 exited0, all48 stages zero, inputs_frozen=1.
Rust4545 passed/204 opt-ins ignored; Vitest3580 in259files; default browser560;
production319 across15configs; repeated57; Python123/one inherited skip.
Layer/header/anchor/ledger/corpus-gate/docs/diff checks all zero, nonempty scopes.
HEAD0f2c1433729081588ee9a9c1c35e493727280126 and complete2770-file manifest
unchanged. Accepted diff SHA256:
7dd9962809ffe795f21e72560c36d713d947a30fee5db39f5ba53634db3997cb.

Eight actual production PNGs inspected: EN/DE1440/400 table states and both
1440 Inspector states. P3 chips/status/dots match. At400 the table scrolls
locally, so later chips/header need horizontal panning. Existing transient
notifications overlap a lower German Inspector action in some frames; no
permanent P3 overlap claimed. Capture provenance/hashes retained privately.

This accepts the frozen branch candidate, not a future merge or publication.

## Fresh-main integration candidate

Feature19019348cc4e3bd139d147c940a757048787cd0c follows seven zero-exit
documentation-only closure checks with runtime bytes unchanged. Fresh main
19b506a6eb73de4704ec1cd422ade9921285d34c brings the reviewed test tooling
and #22's still-refusing offline prerequisite. Only handover/status conflicted;
resolutions retain the exact complete authoritative handover suffix and both
full delivery sections. Nine protected main runtime/tooling inputs are unchanged.
Devices production config retains the new temp guard and the six P3 cases.
No merged runtime acceptance or publication is claimed at this checkpoint.
