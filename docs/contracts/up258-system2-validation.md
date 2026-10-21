# UP258D12 bounded System-2 validation

## Scope and status

**STRONG HYPOTHESIS / SIMULATOR TESTED.** Application
`M-0001_A-9205-04-9270`, exact public package and source digests in
[`up258d12-920504-v1.json`](../../data/system2-validation/up258d12-920504-v1.json).
One complete configuration: product defaults, no group links, no project
parameter/flag overrides. This is a complete source-derived candidate, not a
selected handful of load events, ETS equivalence or verified firmware support.

[ADR-0149](../adr/0149-source-bound-system2-validation.md) defines the separation.
[Independent source observations](../research/2026-10-20-system2-validation-rules.md)
are distinct from the candidate hypotheses below. Owner-supplied research was a
lead, not implementation text or defining authority.

## Pipeline and admission

1. Ingest the pinned public `.knxprod` into a separate versioned ProductDB.
2. `knx device system2-prepare` reads that database without migration or writes.
   The dedicated experimental builder requires all six named policies: numeric
   byte order, bit/Union offsets, legacy patch/reference precedence, invisible
   defaults/Union overlay, one-octet BinaryValue restriction payloads, and
   no-Dynamic-occurrence reference activation. Missing/unrecognized policy,
   mismatched digest/namespace or unsupported required source data refuses.
   Production evaluation/resolution/table generation is reused; ordinary image
   and mask admission remain unchanged. Every source parameter has a coverage
   disposition; virtual/inactive alternatives are not silently confused with
   missing data.
3. Translate all 21 source positions in order, retaining source indices. Four
   absolute allocations reserve 1,514 bytes; computed writes cover 760 bytes.
   The remaining 754 bytes are not written: 338 masked (no source write) plus
   416 octets of declared address/association-table capacity beyond the computed
   tables. The address/association segments write only the computed table, not
   the template's writable capacity. The TaskSegment with tool-only LSM5 is
   retained as metadata, not emitted to a guessed interface object. TaskCtrl2 is
   not dropped. Declared allocation counts, zero flags/checksum interpretation,
   table-zero identity and computed-table-only writes are explicit bounded
   hypotheses, not general XML rules.
4. The existing management-session operation loop executes normalized controls
   and verified writes. A complete source plan is bound to target, application,
   profile version/hash and source hash. No resume from a guessed later step.
5. Before mutation, acquire the old complete allocation intervals and exact write
   regions plus identity/load-state/pointer evidence. App persists private
   immutable, linked checkpoints with exclusive ownership, fsync/readback and
   strict reopening validation. Verify Mode setup is inside that barrier.
6. Failures/cancellation retain operation/source positions and mutation
   uncertainty; no automatic retry or restore. An unfinished reopened journal
   is interrupted. Explicit, separately confirmed recovery replays the complete
   old same-application/procedure/shape with the captured old write bytes and
   verifies the full allocation result, including masked/unwritten bytes.
   A failed/interrupted restore is not automatically retried.

Final recovery must also re-establish the captured five-octet application identity;
its absence at the start of interrupted recovery is tolerated, but absence or
mismatch after replay is a durable RestoreFailed result, never Restored. Every
hardware-kind call to either experimental Net entry refuses before a session or
callback, including complete-write and task-only plans. Source/profile metadata,
a no-op journal callback and ordinary Download authorization confer no admission.

This recovery does **not** read arbitrary allocator/task commands from PID5 or
promise recovery of a different application/allocation history. Same-shape
preconditions are a declared simulator premise. Complete physical rollback of
indirect firmware effects remains unresolved.

## Executable offline validation

Build from the checkout, obtain the public `UP258_DB12__920504d_ETS5.knxprod`
identified in the [public-package evidence](../research/2026-10-16-up258-public-package-evidence.md),
and run:

```sh
cargo build --offline --locked -p knx-cli
python3 tools/validate_up258_system2.py \
  --package /path/to/UP258_DB12__920504d_ETS5.knxprod \
  --knx target/debug/knx \
  --output ./up258-validation-private
```

Use a new output directory. The workflow verifies the package digest before
creating it, isolates data/config homes, then performs ingest → preparation →
preview → simulator execution → recovery in a second CLI process. All commands
are simulator-only; no gateway or bus connection is constructed. `1.1.25` and
serial `SIM...` describe synthetic fixture identity, not a real installation.
The fresh recovery CLI device is explicitly synthetic, not evidence that an
actual device or process retained physical state.

The private directory retains stage exits, full local logs, prepared bytes and
operation-specific recovery. Only review/share `report.json` and
`recovery-report.json`; do not post the operation directory, bundle or raw logs.
Nothing uploads automatically. The reports intentionally exclude target/serial,
raw diagnostics, parameters, memory contents and credentials.

For the integrated same-device oracle and deterministic crash/cancel cuts:

```sh
KNXBENCH_PUBLIC_UP258=/path/to/UP258_DB12__920504d_ETS5.knxprod \
  cargo test --offline --locked -p knx-app --test it \
  system2_validation_preparation:: -- --include-ignored
```

The opt-in source test is separate from the ordinary synthetic tests; the package
is not vendored. It installs through production code, prepares through the same
App entry, seeds an independent OLD typed fixture and retains that same device
through fresh disk reopening. It compares all modeled persistent bytes,
allocation ownership/attributes, pointers, task identities and TaskCtrl2 words.
It covers successful full download/recovery, one-shot partial-memory failure,
source-bound journal cuts after Unload/allocation and partial data, cooperative
cancellation, and the destructive-Unload counter-hypothesis. A journal cut is a
deterministic process-loss boundary simulation, not a real process/device crash.
Additional App/Net tests cover malformed/rebound/corrupt backups, persistence
failure, refused confirmation, identity mismatch and interrupted/failed restore.

## Acceptance checkpoint — 20 October 2026

The integrated candidate at `1d761840` passes the maintained 10-stage package
gate, including the 69-stage full web gate, unchanged generated bindings,
Clippy and 155/155 explicitly invoked private corpus tests. The ordinary Rust
run reports 5,578 passes with 222 ignored; it is not presented as all ignored
hardware/source tests having run. The separate exact-source App selection is
8/8 and the experimental-image selection is 16/16, both with zero ignored.
The rebuilt CLI executes this document's workflow against the pinned public
package: `finished`, then `restored` from the durable operation in a second CLI
process. The explicit physical refusal test returns the expected nonzero exit
before any socket or operation directory. Complete receipts and before/after
input manifests agree; protected user data is unchanged. These are software
results only, not confirmation of the candidate's real-device hypotheses.

The first complete-gate attempt stopped on an existing test-fixture lease
collision. The isolated-fixture correction was independently reviewed and the
entire chain then reran successfully; the first failure remains recorded.

## Open-point dispositions (21 October 2026)

Each remaining open point is either pinned by a defining source or recorded here
as still-blocked with the exact missing evidence. The
[source audit](../research/2026-10-21-up258-masked-byte-and-recovery-rules.md)
carries the per-clause citations; nothing in this section is private analysis
data.

| # | Open point | Disposition |
| --- | --- | --- |
| 1 | Masked-byte preservation / reconstruction (338 bytes) | **Blocked.** The standard defines only logical invalidation on Unload (Management Procedures `03_05_02` v02.01.02 §3.31.3.1, printed p. 138: all data declared invalid) and clears only the PID 7 pointer (Resources `03_05_01` v01.10.01 §4.2.7, printed p. 31). The only "keep existing memory contents unchanged" clause belongs to the relative data allocation (subtype `0Bh`), which mask `0025h` does not use (Profiles `06` v02.01.01 §A.2.4.1, printed p. 144). No clause states physical byte retention or reconstruction. Missing evidence: a firmware-level or destructive-hardware observation defining whether/how the 338 bytes survive. The offline destructive-Unload test already models loss and asserts refusal, never "restored". |
| 2 | Pointer/property widths and resource transforms | **Partially established; transform blocked.** `PID_TABLE_REFERENCE` is a four-octet read-only absolute pointer, `PDT_UNSIGNED_LONG` (Resources §4.2.7, printed p. 31) — `[D]`-cited. The master's "PID 7, Length 2, `Ptr_StandardMemory`" and the `SegFlags` / `AllocExtraByte` XML→wire transforms are defined in the master XML schema / Manufacturer Tool documentation, which is not in the authorized corpus. Missing evidence: that defining schema. No truncation, padding or template-matching inference. |
| 3 | Whole-mutation recovery | **Offline exercised; physical read-back blocked.** PID 5 returns only the current load state (Management Procedures §3.31.3.5, printed p. 142); PID 7 only the pointer (Resources §4.2.7); Verify Mode Control's used-by list omits mask `0025h` (Resources §4.2.14.7, printed pp. 35–36). Success / partial-failure / interrupted cases are already exercised offline. Missing evidence: a standard read-back for the masked bytes and firmware-internal state. |
| 4 | Parameter-encoding confirmation | **Six hypotheses, all still-unresolved.** Each rests on historical Schema10 prose or a revision-bound namespace-20 mirror, not a defining document for this converted legacy application. Listed as unresolved in the [rule sources](../research/2026-10-20-system2-validation-rules.md). |
| 5 | Hardware-validation adapter | **Gated on 1–4** (unchanged). Separate approval does not bypass the technical prerequisites. |
| 6 | Physical validation | **Gated, last** (unchanged). One target, one plan, fresh consent, exclusive gateway, readback plus the operator's functional presence/brightness check. |

The physical-admission refusal remains in force until points 1–4 are resolved and
a separately approved hardware run actually succeeds.

## Physical validation is NOT enabled

Do not connect a bus or substitute a real device identity in the bundle.
`--gateway`, even with both exact confirmations, refuses before network setup.
338 reserved bytes have no source write, and real Unload/reallocation retention
has not been defined. The counter-hypothesis intentionally destroys them; both
NEW completion and attempted recovery then fail full-allocation verification,
never claiming restored. Hardware absence is not the software development
blocker; the missing physical preservation/reconstruction rule is.

A separately approved future hardware-validation adapter must first resolve
that rule and actual pointer/property widths, acquire observed target binding,
join existing activity/gateway admission, and persist operation-specific evidence
before any write. Approval alone must not bypass these technical prerequisites.
No complete System-2/ETS/device programming status is closed by this contract.
