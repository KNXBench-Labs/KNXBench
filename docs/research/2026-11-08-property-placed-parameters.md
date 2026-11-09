# Property-placed parameters, master resets and declared property descriptions (ETS 6.4)

Date: 2026-11-08. Scope: memory (`0701h`) programs whose parameters live in
interface-object properties (`Parameter/Property`), with `LdCtrlMasterReset`,
`LdCtrlDeclarePropDesc` and parameter-filled `LdCtrlWriteProp Count="…"`.
Decision: [ADR-0161](../adr/0161-property-placed-parameters-and-master-resets.md).

## Method and provenance

As in [the 2026-11-07 note](2026-11-07-ldctrlwriteprop-and-byte-order.md):
ETS 6.4 downloaded an unchanged product program (manufacturer `M-0009`, a
10-channel switch/blind actuator, `ParameterByteOrder="LittleEndian"`,
`DownloadInvisibleParameters="Background"`) to KNXBench's own simulator. The
simulator held every declared property as an element array
(`--device-element-property`), pre-filled with `EEh` in one run and `11h` in
another, so that every octet ETS takes from the device and every octet it
computes itself can be told apart. ETS logs and simulator captures stay
private. This is tool behaviour against a simulator, not device evidence.

## Findings

1. **`[V]` Load sequence.** All 97 steps ran: connect, `LdCtrlWriteProp 5/51`
   (`01`), hardware-type compare, two `LdCtrlMasterReset` (erase codes 6 and
   5, channel 0, `AppliesTo="full,grp"` / `"full,par"`), the usual unloads and
   segment loads, then 36 `LdCtrlDeclarePropDesc` + `LdCtrlWriteProp Count=N`
   pairs (`AppliesTo="full,par"`), `5/51 = 02`, restart, disconnect.
2. **`[V]` Master reset is a function property command.** ETS logged
   "Mapping MasterReset(6) to WriteFunctionProperty(5, 52, 4)" and sent
   `A_FunctionPropertyCommand` object 5, PID 52, data `04h`; for erase code 5
   data `10h`. The simulator's answer without a return code was accepted
   ("04 => 00"). Object 5 is mask `0701h`'s `FirstAppObjectIdx` in the KNX
   master data. The inspected Standard chapters (Resources, Management and
   Configuration Procedures, Application Layer) define no such mapping.
3. **`[V]` The checksum read is an optimisation.** After each reset ETS read the
   description of object 5, PID 53 and logged "Optimize Download: Verify
   AppProperty(0/53): 9DF69E32"; the value is the last four octets of the
   program's `Hardware2Program/@CheckSums`. The simulator did not answer, ETS
   timed out twice and downloaded everything. KNXBench does not optimise and
   does not send this read.
4. **`[V]` A declared description is trusted.** ETS read no description for a
   property declared by `LdCtrlDeclarePropDesc`; it did read one for the
   undeclared `5/51`.
5. **`[V]` Read, merge, write.** Before each parameter write ETS read the
   property's current elements, at most 15 per frame (when a range read was
   refused, it read element 0, the count, and then the elements up to it). It
   then wrote all `Count` elements, again at most 15 per frame.
6. **`[V]` Octets without parameter bits keep the device's value.** In both
   fills every octet no `Parameter/Property` reaches was written back exactly
   as read (`EEh` resp. `11h`); ETS's own log shows `--` for those octets.
7. **`[V]` Octets with parameter bits are ETS's.** Such an octet was written
   identically in both fills: the parameter bits from the value, the other bits
   from an image ETS carries (e.g. `81h` where a one-bit parameter at bit
   offset 7 holds `1`). That image matches the program's code-segment `<Data>`
   at positions the product file does not declare.
8. **`[V]` Numbers are low octet first** (`ParameterByteOrder="LittleEndian"`):
   120 → `78 00`, 150 → `96 00`.
9. **`[V]` Invisible parameters are written with the product default.** All
   blind-channel parameters are invisible while a channel pair is set to
   switching, yet ETS wrote them. With a drive time stored as 200 s and then
   hidden, ETS wrote 120 (`78 00`, the default); visible, it wrote 200 (`C8 00`).
10. **`[V]` `Count` may exceed `MaxElements`.** Ten properties are declared with
    16 elements and written with `Count="17"`; ETS wrote 17 elements. How a real
    device answers element 17 is not known.

## KNXBench against ETS

A KNXBench download of the same program (a tester project's device, its
parameters and group links removed so that the product defaults apply, as in
the ETS project) ran to completion against the simulator with the same `EEh`
fill. Compared with the ETS run:

- the same two function property commands, in the same place;
- the same 37 properties written, each with the same number of elements;
- of 1,146 property octets, 25 differ, every one only in bits no parameter
  owns (finding 7: ETS's undeclared image, KNXBench the device's bits);
- memory: 80 octets differ, all only in the group objects'
  communication-enable bit (KL-145, unchanged by this work); KNXBench also
  writes the table segments' remaining template octets (zeros), which ETS does
  not. Both differences already existed for memory-only programs.

In the tester project of issue #10 nine devices move from `unsupported` to
`untested`. Still refused: `LdCtrlWriteProp` with `AppliesTo` and neither
`Count` nor `InlineData`, or with `Count` and `InlineData`; a `Time` parameter
type. The last two were settled later the same day
([follow-up](2026-11-08-time-parameters-and-inline-property-writes.md)), the
first on 9 November ([ADR-0167](../adr/0167-property-writes-without-a-count.md)).

## Open

- A real device's answer to the function property command, to element reads
  past its count and to element `MaxElements + 1` (finding 10). A tester log of
  an ETS download to a real device would settle them; until then such
  downloads are `untested`.
