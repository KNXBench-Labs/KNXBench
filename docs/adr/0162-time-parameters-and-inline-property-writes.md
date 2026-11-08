# ADR 0162: Time parameters and property writes with inline data as ETS 6.4 sends them

Date: 2026-11-08
Status: Accepted

## Context

A four-channel dimming actuator in the tester project of issue #10 (mask
`0701h`, parameters in properties, ADR-0161) could not be downloaded for two
reasons: its soft-on/soft-off times are `TypeTime` parameters, whose value
encoding KNXBench had never established, and its parameter-filled
`LdCtrlWriteProp Count="n"` steps also carry `InlineData`, a shape ADR-0161
left refused.

ETS 6.4, downloading the unchanged program to KNXBench's own simulator with
two device fills and two time values, showed both
([research note](../research/2026-11-08-time-parameters-and-inline-property-writes.md)).

## Decision

1. **Time values.** A `TypeTime` value is the integer the project stores, a
   count of the declared `Unit`, encoded like an unsigned `TypeNumber`: same
   bounds check, same declared byte order. Admitted units are the plain ones
   the local collection declares (`Hours`, `Minutes`, `Seconds`,
   `HundredMilliseconds`, `TenMilliseconds`, `Milliseconds`); a packed,
   unknown or missing unit is refused by name. The unit is read from the
   retained program source (`ProgramCode::time_units`), because the database
   stores none. The image names the inference `time-as-unit-count`.
2. **Inline data covering the count.** A parameter-filled write whose
   `InlineData` is all zero and at least `Count` octets long is written
   without reading the device: every element no parameter reaches is zero.
   The plan marks every bit as determined (`owned = FFh`).
3. **Shorter inline data.** An all-zero `InlineData` shorter than `Count` is not
   used; the write reads and merges the device's value as in ADR-0161.
4. **No parameter, no write.** A parameter-filled write with `InlineData` whose
   property holds no parameter sends nothing.
5. **Non-zero inline data** is refused by name; it was not observed.
6. **No read when nothing is the device's.** The executor reads a property
   before a parameter-filled write only if some bit is not owned. This also
   removes the reads ETS skipped for fully parameter-covered properties in the
   ADR-0161 runs.

## Alternatives considered

- **Treat time like the editor does.** The parameter editor shows plain units
  as counts and falls back for packed ones; the download now draws the same
  line instead of a narrower one (only `HundredMilliseconds`) or a wider one
  (packed units, whose layout no local document defines).
- **Store the unit in the database.** A new column needs a schema migration
  for one attribute that the retained source already holds; the code reader
  takes it from there, like the load procedures.
- **Always read before writing.** Harmless for a real device, but it adds
  telegrams ETS does not send and, for an element the device lacks, refuses a
  write ETS completes.
- **Write a property without parameters back as read.** ETS did not touch it;
  writing it would change nothing but the bus load.

## Consequences

The dimming actuator plans completely; together with the parameters it shares
with ADR-0161 programs, its download stays `untested` until a device confirms
it. Programs whose `TypeTime` uses a packed unit, and procedures with non-zero
`InlineData` on a parameter-filled write, stay refused with a reason.
