# PRG-01 whole-mutation recovery: scope and refusal rule

Status: **scoping contract plus one test-only pin**, 21 November 2026. Evidence: [research](../research/2026-11-21-prg01-n14-recovery-axis.md); decision: [ADR-0188](../adr/0188-prg01-whole-mutation-recovery-scope.md). Related: [exemplar binding](regular-restore-exemplar-binding.md), [application identity](backup-application-identity.md), [service-control backup](SERVICE_CONTROL_BACKUP.md), [mutation-effects scope](prg01-mutation-effects-observation-scope.md).

## Rules

1. **A restore replays memory regions only.** The backup holds written memory regions, load states, prior application identity and the exemplar serial. It restores nothing else.
2. **Refusal over guess.** A restore is derived only when device, product, region shape, load states, application and serial all match, and the plan has no master reset or parameter-property write. These refusals stay; no simulator-only fixture may relax one.
3. **Replayed-uncovered steps are named, not endorsed.** Load records, fixed product property writes and restart are replayed as the plan has them. This is the current behaviour, pinned by a test; it is not a claim that the prior property state is recovered.
4. **Step classification is exhaustive.** Every download step kind is classified for restore (read-only, backed, replayed-uncovered, refused) by a compile-time exhaustive match in the `device_backup` tests; a new step kind fails to compile until classified.
5. **Admission of any wider recovery** (property or non-memory state in a backup, refusing or allowing replayed property writes by rule, post-restore verification) needs a named artifact: a cited primary-source statement or observation plus a semantic RED, with CLI/history/report consumers in the same package.
6. **Wording.** "KNX-compatible, hardware-untested". Never "certified" or "full ETS compatibility". Recovery of device function after an interrupted or failed download is not claimed; whole-mutation recovery stays OPEN.

## Verification of this scope

Tests `restore_coverage_of_every_step_kind_is_pinned` and `a_fixed_property_write_is_replayed_as_planned_not_restored` in `knx-core` `commissioning/device_backup.rs`; results in the [acceptance record](../status/2026-11-21-prg01-n14-recovery-scope.md).
