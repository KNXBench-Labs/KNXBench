# Alpha 16: configured experimental System-2 download

Status, 24 October 2026: shared App/CLI implementation and separate mounted UI/HTTP
consumers have bounded software acceptance in the
[current release matrix](../goals/alpha16-release-plan-2026-10-23.md#current-acceptance-disposition--24-october-2026),
including final original-state TestHardware recovery/once-only/restart assertions.
Dated slice evidence below retains its original scope. Local amd64 software/package
acceptance is complete, including both formerly blocked pinned-input passes;
extra local ARM64 preacceptance is explicitly deferred and unverified, without
changing the normal amd64+arm64 publication workflow. Publication Go is conditional on
final readiness; parent closeout and public readback remain pending.
**Experimental, software-tested; hardware outcome unverified.**
No live-device compatibility, ETS equivalence or physical recovery guarantee.

## Admission and source boundary

Only the exact UP258 application `M-0001_A-9205-04-9270` is admitted. Its expected
mask/manufacturer/hardware/application identity comes from the existing versioned
[identity profile](../../data/system2-validation/up258d12-920504-v1.json), not a
new general mask registry. Ordinary download admission is unchanged.

This is a separate preparation starting from a supplied domain `Project` and
selected `DeviceId`, including a new unsaved project. The existing production
Project → request mapping supplies actual parameters, group links and instance
flag overrides; no empty/default request substitutes for the selected device.
The existing project-specific image/procedure policies and their refusals remain.
Source/profile/input/whole-project identity and request/coverage/ordered plan are
freshly reconstructed from retained originals, as specified in the
[offline source contract](project-system2-offline-preparation.md).

The UI convenience entry selects an explicit retained-source closure, with no
standalone-package or defaults fallback. The explicit-profile entry also permits
the existing package selection for CLI users. Missing, ambiguous or unsupported
sources refuse. Retained originals identify the selected winners and permit
reproduction; they do not certify manufacturer authenticity or archive history.

Offline source preparation is **not physical admission**. An async, read-only
identity observation creates a separate format-3 preparation bound to an
operation id, chosen recovery track, target address and actual observed serial,
mask, manufacturer, hardware and application. Zero, erased and synthetic `SIM`
serials refuse. Format-2 project candidates retain their existing simulator-only
grammar; changing `simulated`, using `rebind_hardware`, or stripping the retained
record cannot admit them. There is no public candidate-to-hardware promotion API.

The offline confirmation binds the operation, target and complete source/project
preparation digest, so it can be shown before any bus connection. The subsequent
hardware record additionally binds the observation and recovery track. A journal
binding includes the hardware-record digest, not just the wire-plan digest.
Different project snapshots with an equal plan are not interchangeable.
Hashes detect accidental drift, not malicious rewriting. Server adapters must
retain their own preparations and reject uploaded mutable bundles.

## Callable App interface

All preparation/execution symbols below are exported from
`knx_app::system2_validation_preparation`. `Connection` is the existing ProductDB
connection, `OperationStore` is from `knx_app::property_validation`, and transport
bounds are `T: knx_net::management::ManagementTransport`.

```rust
fn prepare_retained_project_hardware_source(
    conn: &Connection, project: &Project, device: DeviceId,
    operation: &str, recovery: RecoveryTrack,
) -> io::Result<ProjectHardwareSource>;

fn prepare_project_hardware_source(
    conn: &Connection, project: &Project, device: DeviceId,
    profile: &[u8], operation: &str, recovery: RecoveryTrack,
) -> io::Result<ProjectHardwareSource>;

impl ProjectHardwareSource {
    fn preview_summary(&self) -> io::Result<ConfiguredHardwarePreview>;
    fn validate_confirmations(&self, c: &HardwareConfirmations<'_>) -> io::Result<()>;
    fn check_current(&self, conn: &Connection, project: &Project, device: DeviceId)
        -> io::Result<()>;
    fn with_recovery_track(self, recovery: RecoveryTrack) -> Self;
    fn operation(&self) -> &str;
    fn preview(&self) -> &PreparedBundle; // source-only simulator view, NOT admission
}

async fn observe_project_hardware<T>(
    transport: &T, source: ProjectHardwareSource,
    timing: SessionTiming, cancel: &AtomicBool,
) -> io::Result<PreparedBundle>;

fn admit_current_project_hardware(
    conn: &Connection, project: &Project, device: DeviceId, bundle: &PreparedBundle,
) -> io::Result<CurrentProjectHardware>;

impl CurrentProjectHardware {
    fn bundle(&self) -> &PreparedBundle;
}

async fn execute_project_hardware<T>(
    transport: &T, admission: &CurrentProjectHardware, store: &mut OperationStore,
    timing: SessionTiming, confirmations: &HardwareConfirmations<'_>,
    cancel: &AtomicBool,
) -> io::Result<()>;

fn validate_project_hardware_recovery(
    bundle: &PreparedBundle, store: &OperationStore,
    confirmations: &HardwareRecoveryConfirmations<'_>,
) -> io::Result<()>;

async fn restore_project_hardware<T>(
    transport: &T, bundle: &PreparedBundle, store: &mut OperationStore,
    timing: SessionTiming, confirmations: &HardwareRecoveryConfirmations<'_>,
    cancel: &AtomicBool,
) -> io::Result<()>;
```

`RecoveryTrack::{ExternalEts, KnxBenchBestEffort}` names the operator's intended
recovery route. Both capture the same mandatory actual prestate; external ETS is
an advisory alternative, not certified recovery or a backup exemption. Changing
the selection before observation does not grant authority. Either retains the
explicit KNXBench best-effort option; there is no automatic restore.

`HardwareConfirmations` has borrowed `operation`, `hypothesis`, `download`, `risk`
strings. `HardwareRecoveryConfirmations` has `recovery`, `download`, `risk`.
Every supplied phrase is compared exactly; neither a flag nor a remembered general
consent substitutes. `PreparedBundle::{consent_phrase, acceptance_phrase,
restore_phrase}` supply the corresponding operation strings. Download/risk use
the existing Core/Net phrase constructors. The legacy defaults-only hardware
entries refuse configured bundles: configured execute requires the opaque fresh
`CurrentProjectHardware` witness. Restore uses the original immutable evidence,
not the current project's desired image.

`ProjectHardwareSource` is owned and cloneable and contains no database handle.
Build/check/admit on a blocking worker and cross into the async worker with the
owned source/witness. Do not retain a database mutex across network awaits.
The adapter must compare its current project incarnation/revision/selection
before opening the tunnel and again after observation, immediately before
current-project admission/start. It must consume each preparation once, retain
operation ownership across disconnect and honour the existing gateway/bus/activity
admission. App does not create a second gateway or a UI-owned transport.

A safe start sequence is: fresh source check and exact offline confirmations →
existing single-tunnel/activity guards → read-only target observation → current
project/source admission → private prepared evidence and new operation store →
shared async execution. Restore first opens and validates the intact original
operation and confirmations locally, then acquires the same bus guards, then
calls the shared async restore. Never automatically resend after an HTTP loss.

## Preview, status and cancellation

`ConfiguredHardwarePreview` serializes camelCase fields: `address`, `programId`,
`profileId`, `configurationSha256`, `planSha256`, `parameterValues`, `groupLinks`,
`flagOverrides`, `steps`, `dataOctets`, `allocationOctets`, `notWrittenOctets`,
`warnings`, `confirmationPhrase`, `hypothesisPhrase`, `downloadPhrase`,
`releasePhrase`. The three configuration fields are **counts**, not values.
`configurationSha256` identifies the complete source/project preparation, not
merely a projection of effective values. Steps name source positions, operation
kinds, resource/range/count information without exporting memory contents.
The projection refuses over 2,048 operations or 64 KiB serialized output; it does
not truncate required steps or residual-risk warnings. Adapter-owned public ids,
device name and project/server revision/incarnation fields remain separate.
`configured_hardware_preview(&bundle)` projects a validated observed preparation.

`OperationStore::progress()` and `subscribe_progress(&mut self)` expose
`OperationProgress`: `outcome`, `completed_steps`, `total_steps: Option<usize>`,
`backup_available`, `restore_available`, `mutation_possible`, and
`recovery_phrases: Option<OperationRecoveryPhrases>`. The subscription is
a Tokio watch receiver, available before moving the store into its async task.
Only persisted/read-back checkpoints contribute completed progress; channel
polling requires no disk, SQL, bus operation or long-running executor lock.
Total is unknown before durable recovery acquisition unless the adapter already
has the retained plan count. Backup availability means actual operation-specific
evidence exists, **not** guaranteed physical restorability. Once a restore attempt
is consumed, `restore_available` is false even though the original evidence is
retained. Do not offer another recovery phrase for that attempt.
`OperationStore::recovery_phrases()` and the progress projection derive exact
`recovery_phrase`, `download_phrase`, `release_phrase` from the ORIGINAL durable
operation bundle, never from a newer/current preparation. They are absent while
running/restoring and after an attempt is consumed. Adapters can therefore
supply all recovery acknowledgements after navigation, a lost start response or
reopening, without guessing strings or leaking serials/raw memory.

Cancellation is a caller-owned `AtomicBool`; set it and retain/await the operation
future through cooperative settlement. Do not drop/select away a running future
or claim cancellation undid prior writes. Persistence failure poisons the store
and exposes interrupted status instead of inventing completion. Reopening an
unfinished durable journal is interrupted, never automatic resume. Restart
selection, server task retention and public HTTP DTO state are adapter duties.

## Standalone/embedded close and drain

`knx_app::property_validation` exports the shared, cloneable
`System2OperationControl`, not a second transport/executor:

```rust
impl System2OperationControl {
    fn new() -> Self;
    fn try_admit(&self, operation: &str) -> Result<System2OperationLease, System2ControlError>;
    fn close_admission(&self) -> System2ControlSnapshot;
    fn request_cancel(&self, operation: &str) -> Result<System2ControlSnapshot, System2ControlError>;
    fn snapshot(&self) -> System2ControlSnapshot;
    fn subscribe(&self) -> watch::Receiver<System2ControlSnapshot>;
    async fn drain(&self, timeout: Duration) -> System2DrainOutcome;
}
impl System2OperationLease {
    fn cancel_token(&self) -> Arc<AtomicBool>;
    fn settle(self, outcome: LocalOutcome) -> Result<System2ControlSnapshot, System2ControlError>;
}
```

`System2ControlSnapshot` contains `accepting`, `cancellation_requested`, `settlement`.
`System2Settlement` is `Idle`, `Running { operation }`,
`Settled { operation, outcome }`, or `Uncertain { operation }`.
`System2DrainOutcome` distinguishes `Settled(snapshot)`, `TimedOut(snapshot)`,
`Uncertain(snapshot)`; it never treats process exit as a completed download.
`System2ControlError` distinguishes `Closed`, `Busy`, `InvalidOperation`,
`WrongOperation`, `InvalidSettlement`.

Admission/close share one mutex, so close cannot miss a concurrently admitted
task. Move the opaque lease into the actual owned operation task and pass its
cancel token to App execution. The adapter retains the REAL JoinHandle, original
bundle, OperationStore and bus guards through disconnect; this coordinator never
takes or discards those resources. Close all admission before concurrently
waiting for System-2, catalogue and ordinary-device drains. System-2 drain asks
for cooperative cancellation and waits only on a watch receiver. Timeout leaves
the original task/backup owned and active, for the forced-exit decision; it does
not abort, detach, erase evidence or imply rollback.

Call `lease.settle(final_store_outcome)` only after actual task and transport
ownership settle. Running/Restoring are refused as terminal outcomes. Settled
Cancelled/Failed/Interrupted still is not recovery proof. Dropping a lease without
explicit settlement quarantines the control as Uncertain and closes admission,
never falsely successful drain. Closed admission is not silently reopened.
Restart recovery selection still requires the original durable operation; an
in-memory control is not a restart journal or a whole-device backup.

## Operation-local validation reuse

Configured fresh hardware execution creates one private `ValidatedHardwareExecution` after
current-project admission. It owns the fully validated plan and its derived
same-application preconditions, holds a shared borrow of the exact prepared bundle
and an exclusive borrow of the matching unused `OperationStore`, and is consumed
by the internal executor. The bundle cannot be mutated through that borrow; the
store cannot be swapped, resumed or shared with another execution. No witness is
serialized, cloned, stored on a mutable source, or reused across operations.

Construction still calls the complete `PreparedBundle::plan` validator, including
format grammar, retained-source reconstruction and request/plan/coverage
comparison. Preconditions are derived from that same validated plan. Configured
confirmations and fresh-execution step/source/mutation classification reuse this
immutable result instead of reconstructing the retained sources again. Public
`plan` and `preconditions` calls retain their independent validation behavior.
Format-2 candidates remain simulator-only; the format-3 hardware record and exact
operation-store binding are still required for configured hardware execution.

This witness is **not** current-project/source admission, provenance, consent or
a device backup. Current live winners/project checks, adapter binding rechecks,
all exact confirmations, transport provenance, actual target/identity/prestate
reads, cancellation and Net precondition validation remain independent. Recovery
acquisition still passes through the original `RecoveryRecord` validators and
`OperationStore` checkpoint validation, sync and byte readback before **any**
mutation, including VerifyMode. Disk grammar/hash chaining, reopening, original
backup retention and once-only explicit recovery are unchanged; restore does not
use the fresh-execution witness.

Hardware recovery reuses only its already fully validated **original** plan for
operation-start step/source/mutation classification, in a restoring-aware version
of the same internal callback. The shared original bundle borrow, local owned plan
and exclusive original store borrow remain scoped to that single restore call;
there is no recovery cache or new persistent witness. Net still constructs its
old-byte replay independently. Replay changes only write payloads, so the original
step indices, source positions and mutation classes remain applicable. All
pre-restore route/App/store/source/confirmation validation, snapshot reconstruction,
`RestoreStarted` once-only admission and actual transport checks remain independent.
At `RecoveryAcquired`, the callback still reconstructs/validates the supplied
snapshot and compares the complete record to the durable original; it must never
append a new backup during restore. Existing durable pre-mutation checks and
original retention are unchanged.

Source-only recovery regressions reuse the existing retained-source fixture and
count the actual reconstruction seam: plan acquisition remains fully validating,
while operation-start callbacks perform no reconstruction. Independent snapshot
validation is counted separately. Wrong step/order/source, another correctly
rebound operation/store and a structurally valid but different original snapshot
must refuse without changing journal bytes or progress/mutation state. Synthetic
callback transitions also retain once-only admission after reopen; they are not
device execution or recovery-success evidence. These additions are unexecuted.

Source-only fresh-execution regression additions count actual retained-source
reconstruction entries in a test-local synchronous scope covering context construction,
confirmations and the first two journal steps, and check rebound source/plan and
other-operation refusals without a checkpoint change. They are not executed by
this change. The counter does not cover current admission, complete execution or
the intentionally independent recovery validators. This is duplicate-work
reduction, not a measured speedup, established timeout cause, acceptance result or
new hardware/recovery guarantee. No timing budget or protocol policy changes.

## Actual-state backup and recovery limits

The existing Net executor rechecks observed serial, mask, manufacturer, hardware
and the same installed application. Every touched resource must be Loaded with
its source-applicable four-octet pointer. It reads **all declared absolute
allocation intervals**, including masked/unwritten capacity, plus exact direct
write regions and device control. App persists the immutable recovery checkpoint,
syncs it and its committed head and reads bytes back **before VerifyMode setup,
Unload, allocation, task controls or data writes**. A missing read or persistence
failure refuses the next send. Defaults and the earlier identity observation are
not actual-device backups. The original prestate survives failure/cancel/restore.

A full affected-state read is not a whole-device/allocator image. In particular,
current Loaded states and pointers do not reveal historical allocator/task
commands or prove the source procedure equals that history. That remains the
explicit `experimental-same-application-source-shape-not-observed-allocator-history-v1`
recovery basis. A blank device, absent/different installed application or
non-Loaded resource is refused; a newly created project does not relax this.

Recovery replays the complete source procedure with captured old direct-write
bytes. It verifies complete affected allocation intervals, pointers/Loaded state,
application identity and the captured VerifyMode bit before `Restored`. Masked
and unused-capacity bytes are deliberately not guessed or filled. Their survival
across Unload/reallocation and other indirect firmware effects remain unresolved.
A destructive-Unload counter-hypothesis produces `RestoreFailed`, not Restored.
There is no automatic retry, resume or restore, and no raw allocator rollback
claim. Each durable operation admits at most one explicit restore attempt.

These are the existing [ADR-0152](../adr/0152-gated-hardware-system2-download.md)
best-effort release rules, not a new recovery guarantee. The existing
[source audit](../research/2026-10-21-up258-masked-byte-and-recovery-rules.md)
records the primary boundaries: Management Procedures `03_05_02` v02.01.02
§3.31.3.1/§3.31.3.5 (logical invalidation/current state), Resources `03_05_01`
v01.10.01 §4.2.7 (four-octet read-only pointer), and Profiles `06` v02.01.01
§A.2.4.1 (the relative-allocation retention clause is not this absolute procedure).
No new defining manufacturer-schema or private reporter evidence was obtained.

## Procedure reconciliation

No ProductDB/Core codec change was needed. The existing
[`system2_procedure`](../../crates/knx-productdb/src/system2_procedure.rs) already
emits the full application identity on all table/application task segments.
The old zero-table-identity correction is not reimplemented.

The admitted source keeps every instruction position and its exact declared
machine/start. LSM1 and LSM2 task starts must match their declared address-table
and association-table source placement; inconsistent source is refused, not
swapped or patched. LSM3 retains its declared start and TaskCtrl2 fields. LSM5
remains tool metadata, not a guessed fourth wire resource. Unknown required
transforms remain refused. The
[dated observation](../research/2026-10-22-system2-ets-download-procedure.md)
supports full identities and the represented wire payload layouts, but its
narrative task-start labels do not establish a different source-to-machine
remapping rule. Exact task starts follow retained declarations; broader observed
procedure agreement remains unclaimed. The missing master-schema pointer/flag
transform and invisible/legacy parameter authorities are not silently resolved
by this hardware release.

## CLI workflow

Offline preview of the actual selected configuration:

```sh
knx device system2-project --project desired.knxdb --device-id 1 \
  --product-db products.sqlite --operation-id configured-attempt-1 \
  --source-mode retained-sources --recovery-track knxbench-best-effort
```

To explicitly use a package-bound project-v2 profile instead, select
`--source-mode profile --profile project-profile.json`; it is not a fallback.
The preview prints all four execute phrases and a separate read-only observation
phrase. Repeat the same preparation with:

```sh
  --gateway 203.0.113.250:3671 \
  --observe-confirm 'I confirm read-only System-2 observation to 1.1.25' \
  --output prepared-private.json
```

Use the actual displayed address/phrase, not this documentation example. This
stage writes no device data and retains no claim of an actual-state backup.
Preview and deliberately execute the observed preparation through:

```sh
knx device system2-validation --bundle prepared-private.json
knx device system2-validation --bundle prepared-private.json \
  --project desired.knxdb --device-id 1 --product-db products.sqlite \
  --gateway 203.0.113.250:3671 --operation new-private-operation \
  --confirm '<exact configured preparation phrase>' \
  --accept-hypothesis '<exact hypothesis phrase>' \
  --download-confirm '<exact ordinary download phrase>' \
  --release-confirm '<exact best-effort risk phrase>' \
  --activity-history activity.json --report reviewed-summary.json
```

Explicit recovery uses the original bundle and `--recover existing-private-operation`
with `--confirm-recovery '<displayed recovery phrase>'`, the displayed hypothesis,
ordinary download and risk confirmations, gateway and activity history. It does
not require the current project. A failed/missing/corrupt operation refuses before
configured recovery opens a tunnel. Keep bundles, journals and raw logs private;
only review/share the minimal local summary. Nothing uploads automatically.
