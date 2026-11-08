# Time parameters and property writes with inline data (ETS 6.4)

Date: 2026-11-08. Scope: `TypeTime` parameter values and parameter-filled
`LdCtrlWriteProp Count="n"` steps that also carry `InlineData`, in a mask
`0701h` program whose parameters live in interface-object properties.
Decision: [ADR-0162](../adr/0162-time-parameters-and-inline-property-writes.md).
Builds on [ADR-0161](../adr/0161-property-placed-parameters-and-master-resets.md)
and [its research note](2026-11-08-property-placed-parameters.md).

## Method and provenance

As in the earlier notes: ETS 6.4 downloaded an unchanged product program
(manufacturer `M-0009`, a four-channel dimming actuator,
`ParameterByteOrder="LittleEndian"`, `DownloadInvisibleParameters="Background"`)
to KNXBench's own simulator. Every property the procedure declares was held as
an element array, pre-filled with `EEh` in one run and `11h` in a second, so
that octets ETS takes from the device can be told from octets it computes. A
third run changed two time parameters; the project was then exported from ETS
to read the stored values. ETS logs, captures and the export stay private. This
is tool behaviour against a simulator, not device evidence.

## The program

- 8 `TypeTime` parameters, `SizeInBit="16"`, `Unit="HundredMilliseconds"`,
  `minInclusive="0"`, `maxInclusive="63000"`, `UIHint="Duration_hhmmss"`,
  each placed in a property (`ObjectIndex` 6..9, `PropertyId` 52, `Offset` 1
  and 4).
- 16 `LdCtrlDeclarePropDesc` + `LdCtrlWriteProp Count="n" InlineData="…"`
  pairs (`AppliesTo="full,par"`): objects 6..9 with PIDs 52, 53, 54
  (`Count` 6, 3, 129) and objects 10/52, 10/53, 11/52 (`Count` 11, 14, 26).
  Every `InlineData` is ten zero octets. Objects 10 and 11 hold no parameter.

## Findings

1. **`[V]` A time value is the count of its unit, written like a number.**
   ETS showed `Andimmzeit` as `hh:mm:ss`. Entered as `01:02:03` and
   `00:10:01`, the exported project stored `Value="37230"` and `Value="6010"`
   (3,723 s and 601 s in tenths of a second), and ETS wrote `6E 91` and
   `7A 17`: the same integers, low octet first, as the program's
   `ParameterByteOrder` prescribes for numbers
   ([ADR-0159](../adr/0159-little-endian-numbers-and-fixed-load-steps.md)).
   The Project Schema likewise gives `TypeTime`'s value encoding as "same as
   `TypeNumber`" (ADR-0041). Only `HundredMilliseconds` was observed.
2. **`[V]` `InlineData` covering `Count` is the value under the parameters.**
   For 6/52 (`Count` 6) and 6/53 (`Count` 3) ETS read nothing from the device
   and wrote zero into every octet no parameter reaches, in both fills
   (`00 00 00 00 00 00`, `0D 08 00`; with the time values set,
   `00 6E 91 00 7A 17`).
3. **`[V]` A shorter `InlineData` is not used.** For 6/54 (`Count` 129, ten
   inline octets) ETS read all 129 elements (15 per frame) and wrote element 1,
   which no parameter reaches, as the device held it (`EEh`, then `11h`); the
   128 parameter octets were its own. This is the ADR-0161 behaviour without
   `InlineData`.
4. **`[V]` A property no parameter reaches is not written.** ETS logged the
   steps for 10/52, 10/53 and 11/52 and sent nothing for them: no read, no
   write.
5. **`[V]` A fully determined value is not read.** Findings 2 and 3 match the
   earlier switch-actuator run, where ETS read a property only when some octet
   was not wholly owned by parameters: ETS reads exactly when it needs a device
   bit.

## What stays open

- Only all-zero `InlineData` was observed, so whether ETS uses non-zero inline
  octets as the base or zero-fills regardless is unknown; KNXBench refuses a
  non-zero `InlineData`.
- Whether the threshold in findings 2 and 3 is the inline length or the frame
  size (15 elements) cannot be told here: the only counts between 10 and 15
  belong to properties without parameters. KNXBench uses the inline length,
  the reading that fits every observation.
- Finding 4 was observed only with `InlineData`; without it every written
  property carried parameters. KNXBench keeps writing such a property back as
  read when no `InlineData` is present.
- A property without parameters whose `InlineData` covers `Count` was not
  observed (all three had a larger `Count`); KNXBench sends nothing for it too,
  the reading that changes nothing on the device.
- Packed units (`PackedSecondsAndMilliseconds` and others) and the other plain
  units were not observed. The plain ones are admitted under the schema's
  "same as `TypeNumber`"; packed, unknown or missing units stay refused.

## KNXBench against ETS

A KNXBench download of the tester project's dimming actuator (its parameters,
group links and object overrides removed, so the product defaults apply as in
the fresh ETS device) ran to completion against the simulator with the same
`EEh` fill: 41 steps, every memory write read back. Compared with the ETS run:

- the same two function property commands, in the same place;
- the same 12 properties written, each with the same number of elements;
  neither tool sent anything for the three properties without parameters, and
  neither read the eight properties filled from `InlineData`;
- of 552 property octets, 256 differ, each only in bits no parameter owns: in
  each channel's PID 54, 64 one-bit parameters at bit offset 7 share their
  octet with seven undeclared bits, which ETS fills from its own image and
  KNXBench keeps as the device holds them (ADR-0161, decision 4); no owned bit
  differs;
- KNXBench reads and writes at most 10 elements per frame where ETS used 15:
  its tunnelled downloads stay within standard frames;
- memory: 32 octets differ, all only in the group objects'
  communication-enable bit (KL-145, unchanged by this work).

In the tester project of issue #10 the dimming actuator moves from
`unsupported` to `untested` (15 unsupported, 60 untested devices).
