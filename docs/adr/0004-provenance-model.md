# ADR 0004: Provenance and override-chain model

Date: 2026-09-02
Status: Accepted
Session: 1

## Context

A communication object's effective properties resolve through three layers in
the source data: `ComObject` in the application program, `ComObjectRef` per
application variant, and `ComObjectInstanceRef` per device (RESEARCH §3.2).

In the reference project, 758 of 907 communication object instances override
the datapoint type at instance level. An importer that reads only the
application program is therefore wrong for the majority of objects, and an
exporter that has only the resolved value cannot decide what belongs in the
project file and what belongs to the product database.

## Decision

Values that can be overridden are modelled as `Resolved<T> { value, layer }`,
where `Layer` is one of `Program`, `ProgramRef`, `Instance`, `Inferred` or
`UserEdit`.

`Layer::is_exported()` is true for `Instance` and `UserEdit` only. Program-level
values belong to the product database and are not written back; `Inferred`
values are ours — a datapoint type derived from linked objects, for example —
and are never written back as if the user had set them.

Any command that changes a value sets its layer to `UserEdit`.

## Alternatives considered

**Store only the resolved value.** Simplest model, and it cannot export
correctly: there is no way to tell a value that came from the product database
from one the user set to the same thing.

**Store only the raw layers and resolve on every read.** Correct, but the user
interface needs resolved values everywhere, so this turns every read into a
resolution pass and pushes caching into the UI, which is where it least
belongs.

## Consequences

The export rule is a property of the data rather than of bookkeeping kept
alongside it: an exporter asks the value where it came from.

Changing a device's application program reference invalidates the `Program` and
`ProgramRef` layers of everything under it, so re-resolution must be an
explicit, testable operation.

`Resolved<T>` is in `knx-core` from the first commit of the workspace, because
everything else in the model is shaped by it.
