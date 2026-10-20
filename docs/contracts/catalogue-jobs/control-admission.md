# A2 internal catalogue admission kernel

Status: **private A2 kernel delivered on main** (`9ebb7cc6`, 21 October 2026).
Independent corrected-source review accepts I1 closure. Fresh parent checks pass
30 focused tests and 1,087 owning server tests / 53 ignored (overlapping selections).
Feature and main-first integrated gates each pass all 14 selected stages; see the
[delivery record](../../IMPLEMENTATION_STATUS.md#wp-043-a2-admission-kernel--delivered-21-october-2026).
Construction remains test-only; there is no production Ready/apply API or real
binding producer. Refines stage A of [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
and [R1](../catalogue-update-jobs.md#readyapplyexpiry-admission-r1), without enabling
a production job, Ready consent, installer or route. The existing synchronous
APIs and delivered A1 publication primitive remain unchanged.

## Scope and authority

Use one small crate-private `catalogue_job_control` module in `knx-server`.
It owns a single record's short mutex, phase/generation/revision, one tagged
lease, consent/full accepted command, first stop cause and the actual A1 gate.
Do not attach it to AppState, the router, standalone/desktop shutdown or SQL yet.
It is not a manager, queue, outcome ledger, public DTO or artifact validator.
No crate/dependency or generic task framework is needed.

**Construction is test-only until the real binding producer exists.** Keep the
state/transition implementation compiled normally, but provide no non-test
constructor or production instantiation. This deliberately selects the deferred
construction alternative: an arbitrary string/hash, `()`, v1 token or successful
validator must not stand in for the unavailable controlled PreviewBinding.
Tests supply an explicitly synthetic immutable payload `B` containing distinct
source/profile/policy/raw-preview evidence. The kernel retains that exact value
without validation, replacement, `B: Eq`, cloning, serialization or callbacks.
Its type parameter exists only to retain the future real payload, not to admit
plugins. The preseeded synthetic Previewing payload is not the future production
acquisition recipe: a real binding is produced by controlled preview. Later
construction must attach that actual immutable evidence once at guarded Ready
entry, or use a separately reviewed ownership split, while preserving the same
record, gate and counters; it must not fabricate an early binding or hide mutable
replacement inside B. Concrete production construction, complete artifact/source validation
and installation authority require stages B/C and later reviewed integration.

A test-only initial state represents a preview already owned by the same
operation, not Receiving admission. It has an immutable nonsecret record identity
(16-byte incarnation, checked nonzero/non-sentinel admission serial and 16-byte
job identity), retained synthetic payload, phase Previewing, revision/generation
1, a working lease and Open A1 gate. Ready evidence fixes consent id, artifact id
and public SHA-256 as typed fixed-size bytes. There is no setter for record or
payload. A2's nonsecret identity checks are **not capability authentication**:
no capability parsing, comparison or verified-owner boolean is introduced here.
The future authenticated manager must pin the actual record and verify capability
before entering this private kernel; it must not expose the test constructor.
The existing base64 project/bus incarnation is not changed into the future
catalogue protocol's lowercase hex format.

The control-only snapshot has fixed-size scalar fields and no binding, secrets,
strings, file paths, report collections, terminal outcome or claimed worker
settlement. It is not serializable as the full protocol Status. The one gate is
private: future workers may checkpoint, but cannot mutate it independently of
the control lock. No production commit-claim method exists in A2. A clearly
`cfg(test)` under-lock fixture may claim that same gate and mark Committing solely
to exercise late-stop behavior; it is not precommit/COMMIT evidence.

## One ordering and exact replay

All transitions lock the same mutex. No SQL, filesystem, parser, I/O, join,
serialization or arbitrary callback occurs under it. Immutable payload ownership
is outside the mutable phase fields; no transition clones/drops that payload
under the lock. Poison refuses without recovering the inner value.

For apply, freeze this internal refusal precedence:

1. Refuse poisoned/unavailable control or a different nonsecret record identity.
2. If an accepted command exists, compare its **whole** fixed-size body: command
   id, consent id, artifact id, public digest and confirmed flag. Exact replay
   returns the current snapshot before stop, phase, clock or deadline checks;
   any difference is commandConflict. A rejected command is never remembered.
3. Closed admission refuses according to its retained first cause: serverStop
   -> serverStopping, readyExpired -> consentExpired, counterExhausted ->
   counterExhausted; other stopped phases -> invalidPhase. Do not overwrite a
   first stop merely because its old deadline later passes.
4. Require Ready with available consent and its tagged Ready lease. Other phases
   refuse invalidPhase. Phase generation is internal, not a new wire field.
5. Sample the clock **inside** the lock. Check current lease and time health;
   at `now >= deadline`, request readyExpired settlement immediately, even with
   no reaper, and refuse consentExpired. No terminal cancellation is reported.
6. Before ordinary mutation, preflight the checked revision/generation and new
   working deadline/ticks. Failure closes admission and requests counterExhausted
   through the same gate. Then require confirmed=true and exact consent/artifact/
   digest; false is invalidRequest, mismatch is catalogueUpdateStale. No refusal
   here consumes consent or populates an accepted-command record.
7. In one mutation, consume consent, retain the whole command, advance counters,
   replace Ready lease with a fresh two-hour working lease and enter
   WaitingForWriter. Return AcceptedOnce, not installed or spawned. Replay never
   returns another AcceptedOnce, increments counters or renews the lease.

Ready entry requires the exact previous record/phase/generation/tagged lease,
Open gate and open admission; sample time under lock and refuse an already due
preview lease through workingTimeout. It replaces the working lease with a
15-minute Ready lease, not another independently armed timer. A delayed preview
completion after stop/expiry cannot revive the record. This is internal kernel
behavior only; it does not certify real artifacts or grant production consent.

An expiry ticket contains record identity, phase, generation, lease kind and
exact native deadline. Expiry rechecks all of them under the same lock before
sampling fresh time. A mismatch or current early ticket is a read-only no-op.
After apply wins, the old Ready ticket is inert even after its old deadline.
A current due working lease requests workingTimeout, Ready requests readyExpired.
Snapshot, polling, replay and inert expiry do not renew or increment anything.

User cancel and server stop use the same short lock and A1 gate. Server stop
closes this record's admission (manager-wide stop integration is later). Accepted
cancellation enters Settling, spends unavailable/available consent, retains any
accepted command, and latches the first cause. Repeated stop does not rewrite it.
If commit was already claimed, stop returns TooLate, closes admission if needed,
preserves Committing and leaves stopCause absent; checkpoint remains Ok.
No deadline/stop proves worker completion or releases a slot. There are no
terminal/cleanup/result setters in this increment.

## Clock and checked arithmetic

Use a private bounded clock seam returning a monotonic elapsed `Duration` from
one fixed origin; production adapter uses `Instant::now().checked_duration_since`
that origin. Only that adapter and deterministic test clocks implement the
private trait. No caller-sampled request time or user callback is accepted.
Initialization may sample before publishing the not-yet-shared test object;
every time-sensitive transition samples after acquiring its actual mutex.

Compare full native elapsed durations (including sub-millisecond precision).
Use checked duration addition for 900-second Ready and 7,200-second Working
leases; milliseconds are checked u64 display projections of that same origin,
never the comparison authority. Do not round before testing equality. Backward
samples relative to the last actual sample, native duration failure, addition
failure and millisecond conversion overflow close admission and request
counterExhausted settlement. Exact replay/no-op expiry need not read the clock.
Keep an old valid lease while settling a clock fault; do not fabricate a new
deadline. Initialization failure returns an error without an object or consent.
This partial Settling snapshot is not a total, infallible input to full Status:
its retained valid lease can still be Ready, whereas protocol Settling requires
a working lease. Before stage D exposure, reconcile working/settling lease,
checked ticks, shared incarnation origin and the unavailable/fault path under
the same owner, without inventing a deadline, resetting counters or accepting
work followed by a concealed response-construction failure. No full DTO or
clock-dependent cleanup is added to satisfy this unwired increment.
Internal clock observation changes do not change the fixed snapshot/revision.

Own one real control revision and phase generation, both initially 1, from the
start; do not reset or replace them at future DTO integration. Ordinary phase
mutations must leave their next revision and generation **below** `u64::MAX-64`.
Reaching that threshold closes admission and requests counterExhausted instead
of apply/Ready acceptance. The reserved tail is only for settlement/stop changes.
A2 uses at most one first-stop transition and, when needed, one admission-close
transition after a prior claim; idempotent stop/replay does not spend reserve.
All required next values are checked before gate/consent mutation. If even a
reserved increment cannot be represented, mark control unavailable: every later
snapshot/transition refuses rather than publishing changed state at a reused
revision. No wrap, saturating counter, implicit new incarnation or terminal
setter is allowed. Later lifecycle integration must preserve this budget and
add its separately bounded settlement emissions from the full P2 contract.

## Required deterministic controls

Use real kernel methods/mutex and unchanged A1 gate, no SQL/HTTP/installer model.
The proposed twelve groups may become several small named tests:

- Ready entry replaces the preview lease and refuses stale/due/stopped entry.
- Apply immediately before deadline admits once with exact new lease/counters.
- Equality/after refuse without any reaper, including native sub-ms boundaries.
- The actual clock is sampled under the actual mutex after a scheduled lock wait;
  advancing it to equality before release cannot authorize apply. No sleeps.
- Stale tickets independently vary record, phase, generation, kind and deadline;
  current early/equality/due cases preserve exact snapshots or request stop.
- Whole accepted-command replay after stop/old expiry is read-only.
- Alter every accepted-command field; never-accepted rejection leaves consent and
  replay empty. Do not use a command-id-only equality oracle.
- Each nonsecret handle component differs independently; immutable synthetic
  source/profile/policy/raw-preview payload remains exact and unsubstitutable.
  No capability or live binding validation coverage is claimed.
- Both ordered stop/apply schedules exercise the same kernel and latch first cause.
- Ready and working deadlines/cause are separate; repeated stop cannot relabel.
- Test-only actual commit claim gives TooLate, no stopCause and Ok checkpoint;
  cancellation-first cannot subsequently claim the same gate.
- Checked duration/tick/counter failures, inclusive reserve boundary and poisoned
  mutex refuse atomically; no partial consent, wrap, revision reuse or revival.

Join every spawned test worker before outcome assertions and cleanup. Retain
original interface absence separately from compiled semantic REDs, incremental
GREEN baselines, final inventory/executed names and final source hashes. Focused
Cargo verification, independent review, isolated critical mutants and maintained
branch/main-first gates precede A2 delivery. These are requirements, not results.
