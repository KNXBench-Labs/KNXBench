# ADR 0188: Whole-mutation recovery stays bounded to memory regions

Date: 2026-11-21
Status: Accepted
Session: PRG-01-N14

## Context

PRG-01's last unscoped Open item is whole-mutation recovery. Restore derivation already refuses mismatched device, product, shape, load states, application and serial, and refuses plans with master resets or parameter-property writes. Fixed product property writes, load records and restart are replayed as the plan has them, and the step classification relied on non-exhaustive `matches!` lists.

## Decision

Add no recovery capability. Pin the per-step-kind restore coverage with a compile-time exhaustive table and pin that a fixed property write is replayed unchanged. Record that deciding whether to refuse or allow that replay, capturing non-memory state, and verifying post-restore function need evidence the admitted class (simulator/offline) cannot supply ([research](../research/2026-11-21-prg01-n14-recovery-axis.md), [contract](../contracts/prg01-whole-mutation-recovery-scope.md)).

## Consequences

Adding a step kind fails to compile until classified for restore. No runtime behaviour, consumer or hardware claim changes. PRG-01 stays In progress; with N4, N5, N6 and N14 all ending at an evidence wall, further implementation slices on the Open list need new evidence first, not another scoping pass.
