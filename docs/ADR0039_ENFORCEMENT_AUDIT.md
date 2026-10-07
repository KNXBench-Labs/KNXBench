# ADR-0039 enforcement audit and reserved activation

## Scope and revision

Offline source audit, 2026-10-01 UTC, at
`e691bc1318d0785289f8132378a0f26c9a829b27` (published AR02).
Owner: [alpha-release-goal](../alpha-release-goal.md), AR03 / `KL-129`.
[ADR-0039](adr/0039-project-mutation-goes-through-commands.md) is the accepted
design; its historically parked phases 3–5 still require explicit activation.
This audit is not a new implementation, runtime race test or full mutation
proof. Its source references are pinned to this revision, not permanent lines.

## Already implemented: do not repeat the data-loss fix

- Phase 1: `Command::ReserveIds` and insertion uniqueness checks in
  `crates/knx-core/src/command.rs`; load-time counter raising and explicit
  repair report in `crates/knx-store/src/project.rs:378–403`.
- Phase 2: CSV planning emits `ReserveIds`; server CSV apply is bound to
  the planned revision. Offline scan reconciliation also emits `ReserveIds`
  (`apps/knx-server/src/domain.rs:3098`).
- Existing `crates/knx-app/tests/id_allocation_integrity.rs` drives the stale
  CSV interleaving through real planner/commands/native save. AR02's full
  workspace run executes it, not merely a source inspection.
- AR02 adds typed exhaustion refusal to every allocator, including detached
  construction. It verifies final IDs, monotonic reservation, failed batch
  atomicity and native reopen. See [model contract](DATA_MODEL.md) and
  [alpha evidence](ALPHA_READINESS.md#ar02-general-id-exhaustion).

The historical duplicate-ID loss described by ADR-0039 is not an outstanding
unfixed corruption claim. Enforcement of the live mutation rule remains open.

## Current phase-3 surface

The ADR's historical count of eight live allocation calls is no longer the
current shape: U11 catalog creation already allocates device/com-object IDs
from a detached clone, with complete batch preflight. Do not undo that fix.

| Production path | Current remaining boundary at pinned revision |
| --- | --- |
| `domain::create_group_address_impl`, line 2074 | Allocates directly from live `Project.ids`, then applies separately. |
| `domain::create_area_impl`, line 2116 | Same live allocation pattern. |
| `domain::create_line_impl`, line 2170 | Same live allocation pattern. |
| `domain::create_group_range_impl`, line 2261 | Same live allocation pattern. |
| `domain::create_building_part_impl`, line 2352 | Same live allocation pattern. |
| `domain::set_parameter_value_impl`, line 4475 | New values allocate live; existing values correctly reuse their own ID. |
| `domain::create_devices_impl`, lines 2813–2920 | Detached IDs, existing whole-batch preflight and reservation for quantity > 1; single creation still assigns `project.ids` after command success. |
| `domain::create_devices_impl`, line 2936 | Seed enrichment still mutates the live project after the command. |

These are the audited ADR integration paths, not an exhaustive AST proof that
no other alias or mutation exists. `reconcile_scan_impl` allocates from a clone
and is not one of the six direct allocation sites. Detached import construction
and import-time enrichment remain legitimate exemptions.

## Smallest proposed activation: three independent packages

**Phase 3:** Add the locked command-builder form; move the six remaining live
allocations to detached reservation under the applying lock. Move single-device
reservation and seed enrichment into the creation command, preserving U11
catalog batch atomicity, owning installation, diagnostics and session-log
labels. Reuse the import enrichment implementation via a per-instance form,
not a second enrichment algorithm. Preserve existing-parameter edits at
exhaustion. Test invalid-command refusal without ID consumption, plan/apply
interleavings, undo/redo, seed/default equality and explicitly bound corpus
roundtrip. No new UI control or DTO is proposed.

**Phase 4:** Seal only `Project.ids` (`project.rs:230` is still public), adding
the accepted immutable accessor and detached-construction path. No public
mutable accessor, full-project sealing, schema migration or persisted history.
Migrate all actual callers/fixtures and test native construction/roundtrip.

**Phase 5:** Add `xtask check-project-mutation`, absent from the current xtask
sources/dispatch. Use exact file/function construction exemptions and test both
accepted and rejected fixtures, including test-code handling. Integrate with
CI and verification documentation. Describe alias/lexical limitations openly:
this is a source heuristic for the remaining public fields, not type-system
proof or a general Rust borrow-analysis engine.

Reason to unpark: central checked allocation removes exhaustion, but does not
make the architectural mutation rule enforceable or make failed live creations
stop consuming reserved IDs. Completing the accepted narrow design would
remove those inconsistencies without sealing every project field. This reason
is a proposal, not evidence of a newly reproduced data-loss bug.

## Reserved decision and safe continuation

Status: **WAITING_DECISION**. The activation question returned no user response;
empty input is neither approval nor accepted continued deferral. Do not label
AR03 or `KL-129` done. The user can explicitly activate phases 3–5, or explicitly
accept continued deferral with this disclosed boundary. Until then, preserve
phases 1–2 and AR02, change none of the parked API/enforcement surface, and
continue independent AR04 storage-contract work. A decision here does not
authorize hardware, a release tag or any other owner's work.
