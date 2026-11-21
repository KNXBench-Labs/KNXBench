# ADR 0184: Mutation-effects observation stays closed until restart-stability evidence exists

Date: 2026-11-21
Status: Accepted
Session: PRG-01-N6

## Context

PRG-01 still lists "complete mutation effects". The post-download check already
observes reach, identity, load states and a bounded run state, and the transfer
reads back what it wrote. The remaining candidates are a post-restart memory
comparison, further declared download forms and other identity layouts.

## Decision

Add no observation or form now. A post-restart comparison would report a verdict
about octets the device may legitimately rewrite, and no repository source says
which octets those are; the other candidates lack a cited rule
([research](../research/2026-11-21-prg01-n6-mutation-effects-observation.md)).
Bind later work to the [scope contract](../contracts/prg01-mutation-effects-observation-scope.md).

## Consequences

PRG-01 stays In progress; no runtime, history, report or hardware claim changes.
The Open list does not shrink. The benefit: no simulator-authored guess becomes
a "verified" device statement.
