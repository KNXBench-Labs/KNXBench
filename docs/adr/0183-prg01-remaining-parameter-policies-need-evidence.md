# ADR 0183: Remaining PRG-01 parameter policies wait for evidence, not simulators

Date: 2026-11-21
Status: Accepted
Session: PRG-01-N5

## Context

After the run-state, customer-adjustable, inactive-byte and declared-form
increments, PRG-01's Open list names default-token semantics, editable
Dynamic/Union cases and customer-write rules. Each is a statement about what ETS
or a device does. A simulator reproduces whatever we assume, so a simulator-only
increment would encode a guess.

## Decision

Implement none of them now. Record per item what the repository already does and
what evidence would unblock it ([research](../research/2026-11-21-prg01-n5-remaining-parameter-policy-evidence.md)),
and bind later work to the [scope contract](../contracts/prg01-remaining-parameter-policy-scope.md):
new write semantics need a cited observation or primary-source statement plus a
semantic RED.

## Consequences

PRG-01 stays In progress; no runtime or hardware claim changes. Existing
refusals and fallbacks stay. The cost is that the Open list does not shrink; the
benefit is that no invented ETS behaviour reaches a device write path.
