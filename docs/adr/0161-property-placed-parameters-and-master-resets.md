# ADR 0161: Property-placed parameters and master resets as ETS 6.4 sends them

Date: 2026-11-08
Status: Accepted

## Context

Several mask-`0701h` programs (two manufacturers' switch, blind and push-button
actuators) keep their parameters in interface-object properties
(`Parameter/Property`) instead of memory. Their load procedures carry
`LdCtrlMasterReset` (erase codes 6 and 5, channel 0), `LdCtrlDeclarePropDesc`
and parameter-filled `LdCtrlWriteProp Count="n"`. KNXBench refused all three,
so these devices could not be downloaded.

The KNX Standard does not say how a tool turns these load controls into
telegrams. ETS 6.4, downloading such a program to KNXBench's own simulator,
showed it ([research note](../research/2026-11-08-property-placed-parameters.md)).
The simulator was pre-filled with two different patterns so that every octet
ETS took from the device could be told from the ones it computed.

## Decision

1. **Master reset.** On mask `0701h`, `LdCtrlMasterReset` with erase code 6 or 5
   on channel 0 is sent as `A_FunctionPropertyCommand` to object 5, PID 52,
   with `04h` or `10h`, as ETS logs and sends it. The device must answer return
   code `00h` or none (ETS accepted none). Any other code, channel or mask is
   refused by name.
2. **Declared descriptions.** `LdCtrlDeclarePropDesc` sends nothing; the next
   parameter-filled write of that property relies on it. Only
   `PDT_UNSIGNED_CHAR`, writable, is admitted.
3. **Parameter images.** Under `DownloadInvisibleParameters="Background"`
   every parameter placed in a property is imaged: a visible one with its
   value, an invisible one with its product default (its refs must agree),
   numbers low octet first under `ParameterByteOrder="LittleEndian"`. Each
   image carries a mask of the bits its parameters own. Without that policy a
   visible property parameter stays refused and invisible ones are not
   written.
4. **Writing.** A parameter-filled write reads the device's elements 1..`Count`
   (as ETS: on a refusal, the count in element 0, then the elements up to it),
   merges the owned bits into them and writes `Count` elements in frames of at
   most 15 elements and the negotiated frame size. An element the device does
   not hold is written only when the parameters own all its bits.
5. **Recovery.** A memory backup does not hold property values, so a restore
   and a partial download refuse plans with these steps.

## Alternatives considered

- **Write whole octets like ETS.** ETS fills the other bits of an octet a
  parameter reaches from an image the product file does not declare. KNXBench
  cannot reproduce it from documented data; it keeps the device's bits instead.
  Both agree when the device holds that image, as after the procedure's own
  parameter reset; they never differ in a bit a parameter owns.
- **Use `A_Restart` master reset.** ETS did not; the device's behaviour on a
  standard master reset for these programs is unknown.
- **Read the property description first.** ETS read none for a declared
  property; a real device may not offer one.
- **Skip unchanged properties via `Hardware2Program/@CheckSums`.** ETS reads a
  checksum property to optimise; KNXBench always writes everything.

## Consequences

Nine more devices of the tester project of issue #10 plan completely. A
KNXBench download of a switch actuator to the simulator sent the same two
function property commands and wrote the same 37 properties with the same
element counts as ETS; of 1,146 property octets, 25 differed, all only in bits
no parameter owns. These downloads stay untested until a device confirms them.
`LdCtrlWriteProp` without `Count`, with `Count` and `InlineData`, other erase
codes and other declared types stay refused. (ADR-0162 later admitted `Count`
with all-zero `InlineData` and `TypeTime` values in plain units, and lets the
executor skip the read when every bit is owned; ADR-0167 admitted writes
without `Count` or `InlineData`, sized by the declaration.)
