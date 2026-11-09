# `TypeFloat` parameters in a memory image (ETS 6.4)

Date: 2026-11-09. Scope: two devices of the tester project of issue #10 that
stayed `unsupported` because of a `TypeFloat` parameter: a logic module
(program `M-0083_A-004B-20-7B3F`, mask `0705h`, both encodings) and a glass
push-button (program `M-0083_A-018B-30-F0FC`, mask `0705h`, `DPT 9` only).
Decision: [ADR-0169](../adr/0169-float-parameters.md). Supersedes the float
part of [§19.9](commissioning/19-9-the-procedure-style-refusals-and-signed-and-text-parameter.md)
and [§19.18](commissioning/19-18-object-priority-readoninit-and-floats-under-adr-0086.md)
for memory images.

## Sources

- **[D]** *Datapoint Types* `03_07_02` v02.02.01 §3.10 (printed page 39):
  `F16` is `MEEEEMMM MMMMMMMM`, value `0,01 * M * 2^E`, `E` in `0…15`, `M` a
  12-bit two's-complement number in `-2048…2047`, range
  `-671 088,64 … 670 433,28`; `7FFFh` denotes invalid data. The clause does
  not say which of several representations an encoder chooses or how it
  rounds.
- **[D]** *Project Schema23* §1.1.3.19: a `TypeFloat` value is stored in the
  project as text (`E15` scientific notation). The schema does not describe a
  memory layout.
- `IEEE-754 Single` names the 32-bit binary interchange format.

## Why floats were refused

§19.9 and §19.18 compared the product templates (`AbsoluteSegment/Data`) with
the declared defaults: the `DPT 9` fields agree with that encoding only at
zero, and the non-zero ones do not (a default of `500` sits as `F4h 01h`).
Those octets are what the manufacturer's tool put into the template, not what
ETS writes for a parameter: no source said which of the two a device gets.

## Method and provenance

ETS 6.4 downloaded the unchanged logic-module program to KNXBench's own
simulator. The device sat alone in a new fictional project, with four
functions set to float-valued configurations:

| Function | Parameter (type) | Value | ETS octets |
| --- | --- | ---: | --- |
| Universal calculator, input 1 fixed | `Value4ByteFloat` (`IEEE-754 Single`) | 1.3 | `3F A6 66 66` |
| Universal calculator, input 2 fixed | `Value4ByteFloat` (`IEEE-754 Single`) | -0.26 | `BE 85 1E B8` |
| Filter / limiter, minimum | `ValueFloat` (`DPT 9`) | -12.5 | `83 1E` |
| Filter / limiter, maximum | `ValueFloat` (`DPT 9`) | 1000 | `36 1A` |
| Format converter, on threshold | `ConvertFloat` (`DPT 9`) | 2.5 | `00 FA` |
| Format converter, off threshold | `ConvertFloat` (`DPT 9`) | 25000 | `5C C5` |
| Comparator, value | `TempFloat-50-600` (`DPT 9`) | -5.5 | `85 DA` |
| Comparator, hysteresis | `ValueTempHyst` (`DPT 9`) | 1.2 | `00 78` |

The project was exported, and KNXBench imported that export to build its own
image for the same configuration. ETS's memory image comes from its own
download log; the frames come from the simulator capture. Logs, captures and
the export stay private. This is tool behaviour against a simulator, not
device evidence.

## Findings

1. **`[V]` Declared encoding, high octet first.** Every value is written in the
   encoding its type declares, in a program without `ParameterByteOrder`.
   Template octets play no part: the template held zero at these places.
2. **`[V]` `IEEE-754 Single`** is the nearest binary32 value (`1.3` →
   `3FA66666`).
3. **`[V]` `DPT 9` uses the smallest exponent whose mantissa fits**
   (`2.5` → `E = 0`, `M = 250`; `1000` → `E = 6`; `25000` → `E = 11`).
4. **`[V]` `DPT 9` rounds the mantissa to nearest, ties to even.** `25000`
   gives `1220.7` and ETS wrote `1221` (not truncated); `1000` gives exactly
   `1562.5` and ETS wrote `1562` (`361Ah`), not `1563`. One tie was observed.
5. **`[V]` Whole segment.** KNXBench's image of the parameter segment
   (`4800h`, 2 090 octets) equals ETS's octet for octet. In `4400h` the only
   differences are nine KL-145 communication-enable bits; below that are the
   known table representations. A KNXBench download of the same project to
   the simulator completed, left no ETS image octet unwritten, and differed
   from ETS's image only in those nine bits.

## What KNXBench does

`TypeFloat` values with `Encoding="DPT 9"` or `"IEEE-754 Single"` are written
by the memory (`070nh`) image builder as above, after the declared
`minInclusive`/`maxInclusive` check. Refused, each by name: another or no
`Encoding`, a program declaring `ParameterByteOrder="LittleEndian"`, a float
placed in a property, the legacy System-1/2 and experimental builders, a value
that is not a plain decimal number (decimal comma, `NaN`, infinity), and a
`DPT 9` value outside `F16`'s range or encoding as `7FFFh`.

Tester project: 0.2.70 and 0.2.72 move from `unsupported` to `untested`
(8 unsupported, 67 untested); no other device changes. In the private product
corpus six further programs of the same manufacturer plan, all mask `0705h`;
nothing else changes
([§19.22 addendum](commissioning/19-22-product-coverage-characterisation-baseline.md)).

## What stays open

- One program and one manufacturer; the encodings are named by the product
  data, so other programs declaring them are read the same way, but no other
  program was observed.
- The tie rule rests on one value. Floats in a `LittleEndian` program are
  unobserved and refused.
- The templates that disagree with `DPT 9` (§19.9) remain unexplained; for a
  parameter ETS writes they do not matter, for an inactive one KNXBench keeps
  the template like ETS.
- No device has seen a float written by KNXBench.
