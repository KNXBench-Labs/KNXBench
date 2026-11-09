# Property writes without a count (ETS 6.4)

Date: 2026-11-08. Scope: a parameter-filled `LdCtrlWriteProp` that carries
neither `Count` nor `InlineData`, after an `LdCtrlDeclarePropDesc`, in a mask
`0701h` program whose parameters live in properties.
Decision: [ADR-0167](../adr/0167-property-writes-without-a-count.md). Builds on
[ADR-0161](../adr/0161-property-placed-parameters-and-master-resets.md) and
[ADR-0162](../adr/0162-time-parameters-and-inline-property-writes.md).

## Method and provenance

As in the earlier notes: ETS 6.4 downloaded an unchanged product program
(manufacturer `M-0009`, a two-channel switch/blind actuator,
`ParameterByteOrder="LittleEndian"`, `DownloadInvisibleParameters="Background"`)
to KNXBench's own simulator, which held every declared property as an element
array pre-filled with `EEh` in one run and `11h` in the other. Between the runs
the one parameter in the property in question was changed from 0 to 1. Logs
and captures stay private; this is tool behaviour against a simulator, not
device evidence.

## The program

The procedure ends with `LdCtrlDeclarePropDesc ObjIdx="11" PropId="52"
PropType="PDT_UNSIGNED_CHAR" MaxElements="1"` and
`LdCtrlWriteProp ObjIdx="11" PropId="52" Verify="false" AppliesTo="full,par"`,
without `Count` or `InlineData`. One parameter is placed there: a one-bit
choice at `Offset="0" BitOffset="0"` (the channel pair's function: switching 0,
blinds 1). Every other parameter-filled write of the program carries `Count`.

## Findings

1. **`[V]` The declared `MaxElements` are written.** ETS sent one
   `A_PropertyValue_Write` to 11/52 with one element from index 1.
2. **`[V]` Nothing is read.** ETS sent no read of 11/52, in either run.
3. **`[V]` Bits no parameter owns are zero.** With the device holding `EEh`
   and the parameter at 0, ETS wrote `00`; with the device holding `11h` and
   the parameter at 1, it wrote `80`. The parameter is the most significant
   bit; the other seven bits did not follow the device in either run.

## What stays open

- A write without `Count` to a property that holds no parameter, and one whose
  declaration has more elements than its parameters reach, were not observed.
  KNXBench refuses the first and zero-fills the second as finding 3 suggests.

## KNXBench against ETS

On 9 November KNXBench downloaded the same program from the tester project
(device parameters, links and overrides removed so that the product defaults
apply, as in the ETS project) to the same simulator, once per fill:

- `EEh`, parameter 0: the same two function property commands, the same nine
  properties with the same element counts, and `00` to 11/52 without a read.
  Of 230 property octets 4 differ, each only in bits that KNXBench takes from
  the device (they follow the fill between the two KNXBench runs) and ETS from
  its own image; this is the existing behaviour of writes with `Count`.
- `11h`, parameter 0: again `00` to 11/52, nothing read, so no device bit
  leaks into the property.
- `11h`, parameter 1 (the second ETS run): `80` to 11/52, nothing read, as ETS.
  The same 4 property octets differ only in device-held bits.
- Memory: the written octets differ only in the group objects'
  communication-enable bit (KL-145, unchanged).

In the tester project the device moves from `unsupported` to `untested`; no
other device changes. No device was written; the result is simulator-only.

## A further observation, not acted on

`6/53` and `7/53` are declared with 16 elements and written with `Count="17"`
(finding 10 of the [earlier note](2026-11-08-property-placed-parameters.md)).
The simulator held 16 elements and answered ETS's write of elements 16 and 17
with zero elements, a refusal; ETS went on and reported the download as
complete. KNXBench stops at such an answer. For the KNXBench runs above the
simulator therefore held 17 elements of the same fill. A real device's answer
to element 17 is still unknown, so neither behaviour is changed here.
