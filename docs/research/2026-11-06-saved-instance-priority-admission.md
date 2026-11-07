# Saved-instance priority admission — 6 November 2026

Status: historical source/design admission. Later implementation, focused
execution and remaining acceptance are in the [delivery record](../status/2026-11-06-saved-instance-priority.md).

## Inspected source and primary evidence

The reviewed source is published base `7cba23ff2`. The parent independently
matched all 41 retained code/document inputs to that Git base and rehashed the
original authorised `Project Schema23 v01.00.00.pdf`:
`8fe9193e701350643be5527644955d23b6933c1c4cc66264a3bb234301c21144`.
Sections 1.1.2.4 and 1.2.5.11–13, original physical/printed pages 7 and 48,
provide the three literal tokens, optional instance attribute and omission
context. Original pages were visually inspected; licensed evidence remains
private. These passages do not specify wire bits, malformed-token execution,
older source schemas or module-instance authority.

## Three concrete implementation hazards

1. `knx-etsproj::map_device_v21` groups original override occurrences. Its
   selected `over: Option<_>` is None both for zero declarations and for an
   ambiguous duplicate. A new capture helper must count occurrences before
   declaring absence; reusing None directly would invent source facts.
2. Both native snapshot admission arrays use CURRENT alongside 12–15. Raising
   CURRENT from16 to17 without retaining explicit16 would reject legitimate
   previous working/undo/redo/version images. The new table must be removed
   only from expected predecessor shapes, not by weakening unknown-shape checks.
3. Semantic snapshot identity deliberately omits empty newer overlays. Merely
   hashing an empty new table or increasing the identity version for it would
   invalidate authentic old baseline/context hashes. Empty new metadata keeps
   the old identity choice exactly; nonempty metadata gains its own identity.

## Bounded positive decision

The [contract](../contracts/saved-instance-priority.md) admits a normalized,
immutable observation from fresh uniquely owned non-module schema23 imports.
It keeps declared raw, attribute omission, whole instance-element omission and
unavailability separate. Exact source-member/owner/locator/bytes/hash joins are
required, including independent original device and override occurrence counts.
The existing whole-source/attribute opaque producer is reused unchanged.

No positive old-file promotion is claimed: native16→17 creates an empty table,
and every previous root or model image becomes explicitly NotCaptured while
its retained opaque/context facts stay intact. This narrowing avoids treating
an old image's missing opaque context as proof of absence. Old native builds
may refuse newly written schema17 files; normal read-only open must not modify
an original file during memory-only upgrade.

The parent additionally requires boxed captured payloads and deterministic
pre-admission budgets over source strings, locators, indexes and cumulative
work. These named application observation limits are not format rules. A budget
withholds captured metadata as ResourceLimit while preserving supported import
behavior and opaque facts; no accepted prefix or value default is permitted.
Projection uses a once-per-detail ownership index rather than repeated full
project scans and labels hashes as captured, not currently reverified.

## Real predecessor and future execution boundary

A separate private producer is assigned to create an object-bearing fixture
using the exact unchanged native16 writer, with real baseline, differing
working state, both stacks and a named version. A current writer followed by
DROP statements or lowered user_version is not the defining witness. The
producer's actual source/input/image/file/executable results must be checked
before its artifact is credited or used in new migration acceptance.

All contract tests, migration results, server projections, generated fixture
typechecks and package acceptance remain pending at this admission. Neither
a source review nor an authored model is delivery. No current source-schema,
wire priority, device programming, hardware, native GUI, release or deployment
support is added by this document.
