# Alpha 16 configured System-2 HTTP mount

Status, 24 October 2026: production router/lifecycle implemented; final mounted
HTTP/worker/store recovery reaches all 105 original assertions, including exact
original TestHardware state, reopened `Restored` journal and immediate/restart
once-only refusal without further connection/mutation. The
[current release matrix](../goals/alpha16-release-plan-2026-10-23.md#current-acceptance-disposition--24-october-2026)
supersedes prospective combined-execution wording below; dated checks retain
their original scope. Local amd64 software/package acceptance is complete, including
both formerly blocked pinned-input passes; extra local ARM64 preacceptance is
explicitly deferred and unverified, without changing the normal amd64+arm64
publication workflow. Publication Go is conditional on final readiness; parent closeout and
public readback remain pending. TestHardware is not
physical-device, ETS-parity or UI-usability certification.

This mounts the [UI transport](alpha16-system2-ui.md) over the
[shared configured App service](alpha16-system2-configured-download.md), rather
than invoking the CLI or reusing the ordinary memory-download executor.
Existing ordinary complete/partial download semantics are unchanged.

## Exact owner and callable surfaces

`AppState::device_system2: Arc<System2Service>` is constructed once by
`System2Service::new(data_dir: &Path)`. Routes, actual retained task, local status,
cooperative cancellation and process drain use this same instance. `lib.rs`
merges `device_system2::routes()` into the existing guarded router, before the
existing authentication middleware, with no additional CORS/cookie authority.

The existing UI's prepare/start/status/cancel/recover endpoints have exactly the
required camelCase schema-version-1 public responses, including the three
required nullable ORIGINAL recovery acknowledgement fields. There are no server
paths, serial numbers, raw memory or caller-supplied bundles in these DTOs.
`Status` additionally carries `mutationMayHaveStarted`, `prestateDiffers`,
`pointerObservations` and `programVersionObservation` (2026-10-25, see the
[UI contract](alpha16-system2-ui.md)).

The issue #30 diagnostic follow-up adds required nullable `refusal` to the status
shape: a stable API error-kind token (ASCII alphanumeric, 1–64 characters), or
`null`. A worker refusal before a journal exists is no longer reduced to bare
`failed`. This reason is a volatile record of the current in-process attempt;
new runs, recovery attempts and historical selection/reload clear it. No durable
metadata or journal version changes, and no historical cause is inferred after
restart. The client shows it as diagnostic text, not as execution authority.
A structured `4xx` start refusal is shown alongside existing unknown-outcome
handling; malformed/network/server-error replies do not acquire a refusal claim.
The download hint separately recognizes `system2Operation` in the partial
activity snapshot so the worker's own shared guards are not foreign holders
([ADR-0055](../adr/0055-bus-activity-snapshot-is-partial-session-evidence.md)).

JSON bodies are limited to 16 KiB; struct deserialization refuses duplicate and
unknown fields. Source/recovery enums accept string tokens only. IDs are exactly
32 lowercase hexadecimal characters, never normalized. Preview output including
adapter metadata must fit 64 KiB; required risk/plan text is never truncated.
Command contention refuses rather than queueing a later write. Every response
from these routes carries `Cache-Control: no-store`.

Two additive authenticated local-evidence endpoints permit explicit selection:

- `GET /api/device-system2/operations` returns an array of the exact `Status`
  shape, sorted by opaque operation ID, only while no task is running. It refuses
  an invalid/incomplete inventory or a response over 256 KiB; it never returns a
  misleading partial list.
- `POST /api/device-system2/select`, body `{operationId}`, validates the exact
  original metadata/bundle/journal, durably selects it and returns `Status`.
  This is a local selection, not a download/recovery consent or bus operation.
  Unknown/duplicate fields and malformed IDs refuse as on other commands.

The unchanged UI automatically reattaches the durable selected original through
`status` and can explicitly recover it with the original three phrases, even
when no current project exists. Historical selection is available through these
HTTP endpoints; this slice does not add a historical-operation dropdown to the
frontend. `recover` also validates/selects its explicit original operation ID;
never a current project's or latest preview's plan.

## Preparation and actual execution

Preparation runs on a blocking worker under the current project snapshot and a
scoped catalogue read. It calls `prepare_retained_project_hardware_source` with
the stable selected DeviceId, actual parameters/links/flags and an opaque server
operation ID. It retains the owned `ProjectHardwareSource` privately. The server
binding captures server identity plus project incarnation/revision while the
project lock is held. Unsupported, absent or ambiguous sources refuse; there is
no defaults/package fallback or simulator-to-hardware conversion.

Start validates the retained preparation, all four exact phrases and current
source/project binding before admitting its process-owned task. It consumes the
preparation once. The actual worker checks current sources/binding again before
connect, performs the App read-only hardware observation, then reconstructs the
current-project admission witness after observation and rechecks the project
binding immediately before execution. No database/project mutex crosses a
network await. Selecting a recovery track changes no phrase or write authority.
Ordinary remembered programming consent cannot replace these acknowledgements.

The actual worker reserves existing locks in download → programming → monitor →
scan order using nonblocking acquisition; checks each actual holder; takes the
existing shared per-user gateway file lease; and admits persistent activity
history. The request's preliminary guard check is not tunnel ownership: the
retained worker acquires and holds its own exact guards and cross-process lease
through the awaited consuming IP disconnect, including error paths. A competing
session can cause a refusal, never a second tunnel. There is no reconnect loop.

Tunnel provenance determines `ManagementTransport::target_kind`. Observation and
recovery explicitly refuse simulated tunnels. The new bridge forwards only the
read/control services required here and the known memory/property writes; other
application services refuse. A write cannot pass until `OperationStore`'s watch
projection proves the actual operation-specific recovery checkpoint was persisted
and read back. This covers VerifyMode setup, not just the first load/data write.
App/Net retain the existing complete actual-prestate acquisition, file/ancestor
sync, committed-HEAD readback and exact target-identity checks.

Activity uses the existing **deviceDownload** evidence grammar via its real
`DownloadGuard`, not an unregistered kind or service-control-write alias. The
ordinary session-ID allocator supplies only the history reference; it is not the
System-2 task owner or lifecycle authority. The bridge durably records backup/
send-possible evidence immediately before the first mutation, then records the
actual verified/partial/not-sent classification and independent IP cleanup result.
No source defaults or earlier identity read count as a backup.

## Durable selection and once-only recovery

Private `.knxbench-system2` storage holds a stable exclusive service lock,
create-new owner-only operation directories, immutable bounded metadata,
validated observed format-3 bundles, and the actual App `OperationStore` journal.
The selected-ID file is atomically replaced, file/ancestor-synced and read back
**before any bus contact**. Original bundle/store ownership remains in the actual
worker through disconnect, and all original evidence remains on disk afterwards.
No preparation, navigation, HTTP loss or later operation removes it.

Startup opens only an existing root, retains its lease, reads the selected
original and replays its validated exact journal under `OperationStore::open`.
An unfinished journal is `interrupted`, never resumed or re-executed. A descriptor
without an observed bundle/journal is interrupted with no backup; a partially
constructed, corrupt or mismatched bundle/journal refuses instead of inventing
recovery. A broken selection reports `system2SelectionRequired`; it never chooses
a different operation by timestamp or assumes that missing history means rollback.
A valid explicit selection can reattach an intact historical original.

Recovery first locally reopens and validates that exact original and all three
separate acknowledgements. The actual task writes a create-new, synced/read-back
`recovery-command` claim before IP connect. This adapter-level once-only command
claim also survives a crash or connect failure before App's `RestoreStarted`.
Consumed commands never offer recovery phrases again, even if the original
backup is intact; no HTTP retry/reset clears the claim. A claim without a durable
terminal restore is projected as `interrupted`, never as the old download's
`finished` result. This is deliberately more
conservative than retrying an App attempt that had not reached a device. Recovery
rechecks actual identity through the shared executor and never uses current
project defaults. Only full required verification can produce `restored`.

Retention is bounded at 128 operation directories; reaching the bound refuses a
new operation, never evicts recovery. There is no automatic deletion, reclamation,
expiry, retry or restore. Restart admission preflights at most 16,384 journal
entries and 64 MiB aggregate journal bytes before the shared strict replay; each
bundle/checkpoint also retains the App's existing bounded-read rules. Corrupt or
uncommitted evidence needs private operator inspection, not an automatic repair.
Filesystem-owner tampering and arbitrary local filesystem replacement races are
not authenticated/sandboxed by these hashes or private permissions.

The central project/filesystem resolver reserves this root alongside existing
TLS/catalogue roots. The static fallback checks decoded canonical file targets
against the same reserved roots, including symlink aliases and an overlapping
static directory, rather than publishing raw recovery data.

## Standalone and embedded lifecycle

The exported additive API is:

```rust
pub async fn drain_device_system2(
    state: &AppState,
    limit: Duration,
    second_signal: impl Future<Output = ()>,
) -> System2Drain;

pub struct System2Drain {
    pub outcome: System2DrainOutcome,
    pub work_outstanding: bool,
    pub second_signal: bool,
}
```

`close_process_admission` closes bus, catalogue and the exact System-2 control
before any wait. The retained actual task owns `System2OperationLease`, its cancel
token, original store/bundle and tunnel guards. A cancellation request never
claims rollback. The lease settles the original store outcome only after actual
transport/resource settlement; a dropped lease or failed disconnect leaves the
control uncertain and closed.

Standalone waits for ordinary-device, catalogue and System-2 drains concurrently,
with the existing independent bounds and second-signal receivers. System-2 uses
the configured device-write wait, not the catalogue's deadline. Embedded
`drain_owned_work` returns all three independent facts; `EmbeddedClose` adds
`system2: System2Drain`, and `requires_process_exit()` includes outstanding
System-2 work. The desktop already delegates this exact decision to the embedded
handle; no new desktop exit coordinator is needed.

System-2 drain waits for the shared control **and polls the retained actual join
handle in its owner**, cancellation-safely. Timeout/second stop only drops this
wait: it never takes/drops/aborts the live task, store, backup or disconnect.
Standalone's explicit process-exit escape and the embedder's forced-exit decision
retain truthful outstanding/uncertain facts. Process kill/power loss is still a
crash, not cleanup or device-recovery evidence.

A required adjacent fix makes ordinary-write drain inspection use `try_lock` and
classify held admission locks as unknown active work. Previously it could block
before starting its timer on the very locks System-2 holds through teardown,
defeating all three concurrent deadlines. Ordinary writes remain uncancelled.
The only catalogue-source edit removes duplicate identical public reexports
already present in the integrated base; it preserves the actual manager/drain
exports and changes no catalogue runtime behavior.

## Acceptance and residual limits

Minimal regression assertions for strict requests, authenticated mounting,
selected-original restart projection, task retention/deadline composition and
private evidence fencing are added but not executed here, by instruction. No
suite, browser, benchmark, mutation campaign, installation or real bus action was
run. Final combined-candidate acceptance remains the parent integration's job.

All existing [ADR-0152](../adr/0152-gated-hardware-system2-download.md) experimental
best-effort limitations remain: same supported application/source shape only,
no observed allocator-history proof, unresolved masked/unused byte survival and
indirect firmware effects, no whole-device backup or physical recovery guarantee,
no ETS equivalence and no new firmware/vendor compatibility claim.
