# ADR 0169: Float parameters are written in their declared encoding

Date: 2026-11-09
Status: Accepted

## Context

`TypeFloat` parameters were refused in every image (§19.9, §19.18): the
product templates disagree with a `DPT 9` reading wherever they hold a
non-zero default, and no source said what ETS writes instead. Two devices of
the tester project of issue #10 (a logic module and a glass push-button, both
mask `0705h`) stayed `unsupported` for that reason alone.

ETS 6.4, downloading the logic module's unchanged program to KNXBench's own
simulator with eight distinguishable float values in both declared encodings,
showed what it writes
([research note](../research/2026-11-09-float-parameters.md)).

## Decision

1. The memory (`070nh`) image builder writes a `TypeFloat` value in the
   encoding its type declares, as a number of the encoding's width, high
   octet first:
   - `DPT 9`: the KNX `F16` format with the smallest exponent whose mantissa
     fits, the mantissa rounded to nearest with ties to even;
   - `IEEE-754 Single`: the nearest binary32 value.
2. The value must be a plain decimal number within the type's declared
   `minInclusive`/`maxInclusive` and the encoding's range; `7FFFh` (invalid
   data) is never written.
3. Refused by name: any other or a missing `Encoding`, a program declaring
   `ParameterByteOrder="LittleEndian"`, a float placed in a property, and the
   System-1, System-2 and experimental builders.
4. The image comparison decodes float fields in their encoding.

## Alternatives considered

- **Keep refusing.** The template disagreement concerns octets ETS overwrites
  for the parameters it writes; the declared encoding is what ETS used.
- **Round halves away from zero.** ETS wrote `361Ah` for `1000` (mantissa
  `1562.5`), the even neighbour.
- **Admit floats in every layout.** Only a memory image was observed.

## Consequences

The tester's devices 0.2.70 and 0.2.72 plan and move to `untested`; the
KNXBench image of the observed program equals ETS's in every parameter octet
except KL-145 flags, and a KNXBench download to the simulator wrote the same
octets. In the private product corpus six programs of the same manufacturer
that only floats kept from planning plan as well; nothing else changes. The
rule rests on one program and one tie; it stays untested until a device
confirms it.
