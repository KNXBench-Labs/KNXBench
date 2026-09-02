# ADR 0007: Roundtrip fidelity definition

Date: 2026-09-02
Status: Accepted
Session: 1

## Context

Byte-exact round trips are not achievable (RESEARCH risk R4). Signatures cannot
be regenerated, attribute ordering is not guaranteed to be stable, and
ETS-internal identifiers are assigned by ETS.

Whether ETS re-imports an unsigned file written by a third party is untested
(RESEARCH risk R9). We have no verified answer and will not claim one.

## Decision

Roundtrip fidelity means three things, each of which is testable:

1. **Semantic equality.** The model imported from an export equals the model
   that produced it, under a comparison relation stated explicitly in
   `docs/IMPORT_EXPORT.md`.
2. **Opaque hash equality.** Every opaque entry comes back with the same
   SHA-256 it went in with.
3. **Exports are unsigned.** This is stated, not worked around.

Byte-exactness is never claimed, and neither is ETS compatibility beyond what
has been tested.

## Alternatives considered

**Aim for byte-exactness.** Not achievable for the reasons above, and pursuing
it would distort the writer to imitate ETS output rather than to be correct.

**Leave fidelity undefined.** Makes the claim untestable, and an untestable
compatibility claim is worse than none.

## Consequences

Three concrete test cases, which can be written before the importer is
finished.

The export path must tell the user the result is unsigned, and keep saying so
until risk R9 is settled by an actual test against ETS.
