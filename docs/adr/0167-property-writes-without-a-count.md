# ADR 0167: Property writes without a count follow the declared size

Date: 2026-11-09
Status: Accepted

## Context

A two-channel Hager switch/blind actuator in the tester project of issue #10
ends its load procedure with a parameter-filled `LdCtrlWriteProp` that carries
neither `Count` nor `InlineData`; the preceding `LdCtrlDeclarePropDesc`
declares `MaxElements="1"`. ADR-0161 and ADR-0162 cover writes with `Count`, so
this one stayed refused as an untranslated step.

ETS 6.4, downloading the unchanged program to KNXBench's own simulator with two
device fills and two parameter values, showed what it sends
([research note](../research/2026-11-08-property-writes-without-a-count.md)).

## Decision

1. `LdCtrlWriteProp` with `Verify="false"` and neither `Count` nor
   `InlineData` is read as a parameter-filled write without a count.
2. The planner writes the declared `MaxElements` from index 1, without reading
   the device: the parameters' bits, and zero in every bit no parameter owns
   (`owned = FFh`). The plan names `declared-size-without-count`.
3. It is refused when the property holds no parameter, when no description was
   declared, or when parameters lie past the declared elements.

## Alternatives considered

- **Read and merge as with `Count`.** ETS read nothing and wrote zero where the
  device held `EEh` or `11h`; merging would keep bits ETS overwrites.
- **Write one element regardless of the declaration.** Matches this program
  only by coincidence; the declaration is the size the procedure states.

## Consequences

The tester's device 0.2.67 plans and moves from `unsupported` to `untested`;
no other device of that project changes. KNXBench downloads of it to the
simulator wrote 11/52 exactly as ETS did in both runs (`00` and `80`) and read
nothing, with either device fill. The size rule rests on one program whose
declaration and parameters both fit one element: the declaration is the only
size the procedure states, so it is used, and the refusals keep everything
else out. The zero fill was likewise seen in one octet only (one one-bit
parameter beside seven unowned bits); ETS writes its own image into unowned
bits of other properties (ADR-0161 finding 7), so zero here is an observation
of this property, matching the `InlineData` fill of ADR-0162. These downloads
stay untested until a device confirms them.
