# Alpha16 B10 managed-apply bridge

Status: implemented controlled slice, focused runtime checks pass; independent
review and combined production acceptance remain separate. This document is the
callable cross-crate contract, not a
claim that all WP-04.3 or B08/B10 acceptance is complete.

## ProductDB controlled API

`knx_productdb::managed_apply` explicitly exposes controlled operations without
exporting a transaction, connection, arbitrary SQL closure, private orchestration
trait or a receipt-success setter. Existing quiet APIs/errors remain unchanged.

```rust
AdmittedSource::admit(name: &str, bytes: &[u8], policy: ControlledPolicy)
    -> Result<AdmittedSource<'_>, ControlledError>;
ApplyWorkspace::create(directory: PathBuf) -> std::io::Result<ApplyWorkspace>;
ManagedCatalogue::prepare_controlled(
    &mut self, source: AdmittedSource<'_>, gate: &PublicationGate,
    workspace: &mut ApplyWorkspace,
) -> Result<PreparedPreview, PreviewError>;
ManagedCatalogue::execute_controlled(
    &mut self, source: AdmittedSource<'_>, shown: &PreparedPreview,
    control: &dyn CommitArbitration, workspace: &mut ApplyWorkspace,
    receipt: &mut ApplyReceipt, publisher: &mut dyn RecoveryPublisher,
) -> Result<ApplyExecution, ApplyBridgeError>;
```

Workspace creation exclusively creates one private subdirectory beneath a
caller-owned root. Preview and apply use **distinct** workspaces. It owns fixed
D/B lifecycle inventory, retains failed handles, exposes `has_recovery()`, and
never automatically unlinks recovery data. The caller must retain it across
ordinary Rust unwind. It is not the P8 durable server artifact inventory.

`PreparedPreview` contains the immutable actual rehearsal/binding. It is not
client-constructible. `PreparedPreview::install()` borrows the complete immutable
raw rehearsal result for the runtime's independent artifact admission/projection.
The binding version is explicitly **3**: actual observed
W `trusted_schema`/`recursive_triggers` boolean values now participate in consent
equality, before BEGIN and under the original reservation; neither value is
silently repaired. The five previously fixed settings remain required.

`CommitArbitration` exposes only `gate()` and `claim_commit()`. The latter returns
an opaque non-Clone `CommitClaim` minted by the unique successful CAS on that
same gate. ProductDB consumes/checks its gate identity before its one native W
COMMIT. This is a control-only hook, never arbitrary SQLite/write authority.

## Server callable bridge

The source file is `apps/knx-server/src/catalogue_jobs/controlled_apply.rs`;
its module is mounted from **`catalogue_job_control`**, deliberately without
editing the runtime worker's `catalogue_jobs/mod.rs` ownership.

```rust
type ManagedApplyControl<C> = Control<OnceLock<PreparedPreview>, C>;
Control::new(identity: Identity, binding: B, clock: C) -> Result<Control<B,C>, Error>;
controlled_apply::prepare<C: Clock>(
    owner: &CatalogueOwner, source: AdmittedSource<'_>,
    control: &ManagedApplyControl<C>, workspace: &mut ApplyWorkspace,
) -> Result<(), BridgeError>;
controlled_apply::apply<C: Clock>(
    owner: &CatalogueOwner, source: AdmittedSource<'_>,
    control: &ManagedApplyControl<C>, workspace: &mut ApplyWorkspace,
    receipt: &mut ApplyReceipt, publisher: &mut dyn RecoveryPublisher,
) -> Result<ApplyExecution, BridgeError>;
```

Create `Control::new(identity, OnceLock::new(), MonotonicClock { origin })` once
per job. `prepare` binds the actual preview once on the same operation gate.
Only after all independent artifact/consent requirements complete may the
runtime call `enter_ready(ticket, ReadyEvidence)`. `apply(identity, Command)`
atomically consumes that consent; only the first accepted command schedules a
worker. `controlled_apply::apply` consumes the bound preview, not a substituted
binding. All these types/methods are server-crate internal.

Owner acquisition is `try_lock`: Busy means no writer was acquired. A supervisor
may wait/check stop/deadline without holding control; neither bridge spawns a
task. Once W is owned, the only nesting is W -> short control. Final claim checks
accepted phase, samples monotonic time under control, checks reserve counters,
and claims the same gate used by cancel/stop/exact-ticket expiry. No SQLite runs
under control. No native interruption hook is added.

## Receipt, unwind and poison

Construct `ApplyReceipt::default()` and both workspace owners **outside** the
worker/response unwind boundary. `execute_controlled` catches ordinary unwind
while the managed owner is still borrowed, then metadata-only moves its exact
attempt-key trace into that outer receipt, before server result projection.
No poisoned lock is cleared or extracted to read SQL/evidence.

`receipt.witness()` is metadata only; `witness.key()` / `witness.trace()` expose
fixed-size identity/trace. `receipt.take(exact_key)` is the explicit consuming
handoff; a wrong key preserves it. An occupied inner or outer receipt refuses a
new apply rather than replacing old evidence. Dropping `ApplyExecution` does
not consume the receipt. Native OK/autocommit=true is recorded before callbacks,
restoration or inventory, and remains committed even after later cleanup panic.
The managed guard remains armed through the final inventory projection. Panic
before W COMMIT is not proof of rollback: Ignore leaves it quarantined/unresolved.

`ApplyExecution` retains the original typed ordinary settlement and exposes
commit/rollback/health/artifacts/failed/cleanup_failed/panicked projections.
`failure_kind()` and `PreviewError::kind()` expose controlled diagnostic categories,
not HTTP mappings; raw internal exceptions must never be serialized to clients.
Recovery B stays independently owned for committed/unknown, including poison.
An outer response panic may poison the server mutex but cannot erase a receipt
already handed out. Metadata handoff does not repair health or permit retry.

## Implemented versus independent boundaries

This slice supplies actual managed apply, native evidence and server same-gate
commit arbitration. It does **not** implement HTTP authentication/DTO assembly,
spool/result caps, durable artifact/sidecar inventory, recovery publication,
reclamation, worker/join/drain ownership, UI or shutdown. The original B10-only commit retained a private B and lacked the P8 pre-live
barrier. The subsequent runtime extension closes THAT dependency, not all WP-04.3:
`execute_controlled` and the server apply bridge now require a `RecoveryPublisher`.
Its allocated-FD callback runs before SQLite opens D/B. After actual copy
validation, inspected S settlement and D rehearsal close, orchestration invokes
publication before its first W DML. `ValidatedRecovery::publish_to` performs
no-replace rename and file+both-directory sync; it alone constructs the opaque
non-Clone `PublishedRecovery` receipt checked against the exact destination id.
A backup-publication refusal explicitly rolls back the unchanged reservation.
The same required publisher also receives the complete actual live install and
checked actual W page-byte extent through `admit_result`, before COMMIT claim.
A report-admission refusal takes inspected rollback; it cannot commit without
its complete admitted report. The server adapter requires an explicit
`JobResultAdmission` implementation, not a default no-op report sink. A moved copy with a
failed acknowledgement remains in its native owner with its actual new path.

`JobRecoveryPublisher` supplies the real server adapter and bounded durable
inventory acknowledgement; it publishes in the existing listed-backup grammar.
`ApplyWorkspace::in_owned_directory` and `prepare_controlled_owned` /
`controlled_apply::prepare_owned` connect that same leased job workspace to
initial-preview allocation. `native_handles_closed` and
`release_closed_ownership` permit release only after inspected native close;
failed handles remain retained. Quiet APIs are unchanged. The subsequent
production composition in `alpha16-runtime.md` now supplies the retained
session/status/full-record adapter and authenticated, path-fenced common app
mount. Its source wiring and compile checks are not combined runtime/UI
acceptance and do not retroactively widen the focused B10 evidence below.

No durable outcome database, generic task registry, automatic restore/retry,
process-loss/power-loss/panic-abort/double-panic guarantee or global quota claim
is introduced. Wider native race matrices and final combined acceptance remain
parent-owned; minimal executed cases must not be called exhaustive B10 closure.

## Focused execution boundary

The final affected-package run compiles ProductDB and the actual server bridge:
10 selected ProductDB methods and 3 selected server methods pass, without owning
suites, mutation sweeps, benchmarks or package/integration gates. The server
native method exercises five channel-ordered schedules: user cancel, server stop,
working deadline without reaper, exact-ticket expiry, and commit-wins/late stop.
Native COMMIT/rollback counts and independently read installed rows distinguish
the winners. All worker threads join before assertions.

ProductDB controls cover immediate and final-inventory panic after native COMMIT,
precommit unwind after scratch publication, public bridge receipt handoff through
its caught unwind, wrong-gate claim refusal, lost result/pending/wrong-key/exact
handoff, exhaustion, variable-setting drift and denied initial/reserved getters.
The outer server receipt and B remain owned after a deliberate post-handoff panic
poisons the owner mutex; metadata transfer does not clear poison or allow SQL.

Additional retained-trace assertions were added to existing Busy/non-Busy/denied
rollback and growth-restoration controls. Those existing controls are left for
the parent's single combined acceptance, not claimed as newly re-executed here.
