# PRG-01-N14: what would "whole-mutation recovery" need, and what is already guaranteed?

Date: 21 November 2026. Baseline: `origin/main` `4c6da4113`. Source and document inspection plus offline unit tests only; no bus, device, ETS VM or manufacturer contact; KNX Secure is excluded. Decision: [ADR-0188](../adr/0188-prg01-whole-mutation-recovery-scope.md); binding scope: [contract](../contracts/prg01-whole-mutation-recovery-scope.md).

## Question

The last unscoped item of the PRG-01 Open list is "whole-mutation recovery contracts". Which parts are already guaranteed, which can a simulator verify now, and which need evidence it cannot supply?

## Inventory

| # | Requirement | Finding in source | Class |
| --- | --- | --- | --- |
| 1 | Restore derivation refuses a backup of another device, product, region shape, load-state set, application, or without a bindable serial; refuses plans that add a guard, change nothing, reset the device or write parameter properties | `restore_plan`/`check_same_shape` in `knx-core` `commissioning/device_backup.rs`; every `RestoreError` variant has a unit test there. [Exemplar binding](../contracts/regular-restore-exemplar-binding.md), [application identity](../contracts/backup-application-identity.md) | (a) |
| 2 | A backup covers every memory region a plan writes | A backup is accepted only when its regions equal `written_regions(plan)` (`OtherShape`) | (a) |
| 3 | A backup of a not-fully-loaded device is not restored | `RestorePrepareError::NotLoaded`, `was_loaded()` | (a) |
| 4 | A changed product procedure is refused | `RestorePrepareError::DifferentProcedure` (compares `plan_steps`) | (a) |
| 5 | Restore refusals reach the operator | One consumer: the `knx` CLI restore prints the refusal. The server and web have no restore path, so there is no second table to drift (unlike download categories, ADR-0187) | (a) |
| 6 | Non-memory mutations: master reset and parameter-property writes | Plans containing them are refused (`changes_unbacked_state`). Fixed product `WriteProperty` steps, load records and restart are **replayed as the plan has them**; the backup holds no property value for them (see row 7) | (a) |
| 7 | Whether a replayed fixed `WriteProperty` is acceptable for a restore | Not stated in any repository source; `partial_memory_download.rs` keeps the product's write per CP §3.9.2.4 but says nothing about restore. Refusing would change behaviour on a guess; allowing is the status quo | (c) |
| 8 | Classification of step kinds into read-only / backed / replayed-uncovered / refused is exhaustive | `changes_unbacked_state` and `changes_device` are `matches!` lists: a new step variant defaults silently to "backed" or "read-only". **Closed in this slice** by a compile-time exhaustive table test | (b) |
| 9 | Recovery after a download that stopped mid-way | The device may be partially written (`changes_device` doc); a backup taken before the write exists and the restore path above applies. A guarantee that restore returns the device to a working state needs a device observation | (c) |
| 10 | Property-only (service-control) recovery | Separate record, explicitly not a device image or restore plan ([contract](../contracts/SERVICE_CONTROL_BACKUP.md)) | (a) |
| 11 | Restoring non-memory state (properties, tables outside written regions, task/PEI state, volatile RAM) | The backup format has no field for it; adding one needs the device's readable property set per mask | (c) |
| 12 | Whole-mutation recovery as an end-to-end claim (device returns to prior function) | Physical effect; a simulator returns what its author chose | (c) |

## Counts

(a) 7 (rows 1–6, 10); (b) 1 (row 8, implemented); (c) 4 (rows 7, 9, 11, 12).

## What is missing for the (c) rows

- Row 7: a primary-source rule (or observed ETS/device behaviour) saying whether a product `LdCtrlWriteProp` is part of the restorable state. Until then the behaviour is only recorded, not changed.
- Rows 9, 11, 12: a device observation of the post-restore state, and a per-mask statement of which properties a backup must capture. Both need real hardware or a manufacturer statement; neither is admitted.

## Method note

Row 8 is a drift pin, test-only, characterization class: it passes on current `main` and cannot be RED because the existing behaviour is what is pinned. Adding a step variant fails to compile until the table is updated.
