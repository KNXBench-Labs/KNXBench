# WP-07 — Module-instance authority

**Status:** Blocked: evidence. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

**Evidence route:** clean-room observation is admitted by the owner (8 November 2026; [OWN-01](OWN-01-owner-decisions.md#decisions-recorded--8-november-2026), [rules](../RESEARCH.md#evidence-routes)). The full independently counted mapping witness is not yet accepted. The bounded source-only classification below does not supply missing/duplicate/nested/repeated authority or public ETS-observation provenance.

**Mapping witness (17 November 2026):** the admitted mapping contract is now backed by a real, independently counted witness over eight ETS exports of one fictional project plus two modular corpus projects (42 + 32 + 75 module instances; 292 counted argument values, all matching `Start + Allocates·(k−1)`; RepeatIndex positions counted 149/149): [witness](../research/2026-11-17-wp07-module-mapping-witness.md). Acceptance of that witness, and with it the unblocking of the first deliverable, remains with the owner; the download-image arithmetic and bus behavior stay out of scope.

## Task

Establish `ModuleDef`/`ModuleInstance`, repeat and number-offset identity
(KL-068, PDB-01) for module-scoped consumers and for constructing newly
activated module objects. Caption matching is no substitute.

## Current state

Read-only own instance values and unscoped object creation are delivered. A
parameter write that would activate a module object not already represented
by an imported instance is refused as a whole; the project stays unchanged.

**WP07-S1 (14 November 2026; ships in Alpha 22):** an imported module
object with unresolved own instance authority projects `Undetermined` without
a channel after evaluation, rather than accidentally taking the genuine
unscoped `Inactive` path. `NotEvaluated` without an evaluable program and the
existing unscoped/resolved controls stay unchanged. Corrected source
`8873e0150f` has retained focused 17 unit + 9 HTTP passes and independent F1
closure. The main-first slice is software-accepted and published as `e31f0e062`
after the complete selected gate14/14 (Rust2090passed/73ignored, separate
corpus51/51; frozen inputs and checked user data untouched). No deployment or
native UI claim.
The read-only overlay adds no persisted owner, numbering or write authority;
constructed missing/foreign-owner states are not malformed-store load/reopen
evidence. [Epochs, controls and limits](../status/2026-11-14-wp07-unresolved-module-owner.md).

## First deliverable

One admitted mapping contract backed by a real, independently counted
witness, with missing, duplicate, nested and repeated cases.

## Done when

Evaluation, parameter writes and object reconciliation reuse that authority;
ambiguous mappings refuse the whole edit without spending IDs or changing
native history. Identity-keyed module comparison follows the mapping.

## References

[ADR-0108](../adr/0108-read-only-module-instance-values.md) ·
[object-refresh contract](../contracts/parameter-object-refresh.md) ·
[KL-068](../limitations/KL-068-repeated-module-activation.md) ·
[source stop lines](../research/2026-10-16-offline-source-stop-lines.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-07--module-instance-authority)
