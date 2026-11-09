# ADR 0168: Assigned and unnamed choose controllers, short segments, mid-octet unions

Date: 2026-11-09
Status: Accepted

## Context

Three devices of the tester project of issue #10 stayed `unsupported` after
ADR-0167. A push-button sensor was refused because a linked object was not
active: its channel hangs on a controller that ADR-0160 counts as inactive,
while an active `Assign`, which KNXBench does not execute, sets it. Once that
was fixed, an unnamed hidden controller conflicted with the shown one on a
union member, and the procedure allocates more octets than the segment holds.
Two binary inputs place union members inside unions that start mid-octet,
which had no attested reading.

ETS 6.4 downloads of the unchanged programs to KNXBench's own simulator, and
the object activity ETS records in the tester's project, settled each point
([research note](../research/2026-11-09-assigned-controllers-and-short-segments.md)).

## Decision

1. **Assigned controllers.** An evaluation pass records every `Assign` it
   reaches in an active branch (target, and source parameter or literal value,
   read strictly from the stored declaration). A controller that such an
   assignment sets is not marked inactive. If every active assignment yields
   the value the evaluator reads for the controller, its `choose` is
   evaluated as it stands. Otherwise, or when the source value is unknown,
   the pass reports `AssignedControllerDiffers` (and an unreadable `Assign` as
   an unrecognized node) and stops marking, so nothing is guessed: a download
   is refused, the parameter panel shows the diagnostic. An `Assign` below a
   controller that the same pass marks inactive is not run and does not count;
   starting from all candidates, the excluded set shrinks to a fixed point.
2. **Unnamed alternative controllers.** A controller that no
   `ParameterRefRef` names anywhere, whose every `choose` is `Comparable`
   without a `default` branch, and whose branches activate a
   `ParameterRefRef` that a branch of a placed controller's `choose` also
   activates, counts as placed while its value is its product default, so
   ADR-0160 applies when it is not active. Holding another value (a stored
   edit, or one an `Assign` set), it is read as before, and an illegal value
   is still reported. Every other unnamed controller (`TypeNone` and
   default-only structure, a controller with no placed competitor, one
   deciding a `choose` inside a `ModuleDef`) and a controller named only
   inside its own `choose` stay read as before.
3. **Short segments.** An allocation longer than its segment writes the
   segment's octets, provided every written parameter field and table lies in
   them; the plan names `allocation-beyond-segment-not-written`.
4. **Mid-octet unions.** A union member's position is the union's offset and
   bit offset plus the member's offset and bit offset, whole octets carried.
   A member so placed that crosses an octet boundary is refused (unobserved),
   and the legacy System-1 and System-2 image layouts keep refusing members
   away from the start of a mid-octet union.

## Alternatives considered

- **Execute `Assign`.** It is the general answer, but assignments chain and
  feed other choices; nothing beyond these programs was observed, and the
  stored values already agree with the assignments ETS ran.
- **Treat every unnamed controller as inactive.** Measured on the tester
  project, it made 13 devices of two other manufacturers refuse links on
  objects ETS keeps (eight of them planned before): their unnamed controllers
  are `TypeNone` or default-only structure.
- **Treat every unnamed `Comparable` controller without a default branch as
  placed.** It matched ETS on the tester project, including two hidden
  push-button channels ETS does not list, but independent review showed it
  skips the value check of hidden selectors: an illegal stored value that was
  refused through `NoBranchMatched` would build an image. Those channels'
  objects stay inactive anyway (their function defaults to "not active"); the
  narrower rule keeps them evaluated as before.
- **Mark competing alternatives whatever their value.** A read-only scan of
  the private corpus found the competing shape in 10 of 1,441 programs,
  including logic and timer programs of other manufacturers. With a stored
  non-default value of such a controller, independent review measured 15
  parameters and three objects silently dropped, and illegal values no
  longer reported. Limiting the rule to the default keeps those programs as
  before wherever a value was chosen.
- **Resolve the union-member conflict by "the shown one wins".** It names the
  outcome, not a rule the walk can apply before conflicts arise, and leaves
  the unnamed channels active that ETS does not show.
- **Write the full allocation, padded.** ETS did not send octets past the
  segment.

## Consequences

The push-button sensor 0.2.49 and the binary inputs 0.2.7, 0.2.63 and 0.2.66
of the tester project plan and move to `untested`; KNXBench's images equal
ETS's in every parameter octet, differing only in KL-145 flags. Active objects
equal ETS's records on all 51 devices that carry them. The rules rest on one
program each and on simulator runs; the downloads stay untested until a device
confirms them, and `Assign` stays unexecuted. An `Assign` targeting a
reference in another module scope than the `choose` it would decide never
matches and falls back to ADR-0160 without a diagnostic, and an `Assign`
inside a `ModuleDef` instantiated below a hidden controller still counts
(refusing rather than guessing); neither was observed. At their defaults, the
competing controllers of the other corpus programs hide at most one
parameter without memory, as review measured on three of them. Controllers
that only assign each other from below their own hidden `choose` are all
marked (the greatest fixed point), as nothing outside them runs those
assignments.
