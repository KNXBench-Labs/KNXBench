# DPT numeric range audit (AR09, 2026-10-05)

Second AR09 pass, following [the format-width audit](2026-10-05-dpt-format-widths.md).
Source: 03_07_02 Datapoint Types v02.02.01 AS ("DPT-AS"), datapoint-type
tables of §§3.5–3.15 and 3.28 (local `knx-spec-kb`, extracts in scratch only).

Method: a throwaway probe encoded and decoded the printed minimum, maximum and
one step beyond each limit through the public codec for 5.001, 5.003, 5.004,
5.010, 6.001, 7.001–7.005, 7.012, 8.001, 8.003, 8.004, 8.010, 8.011, 9.001,
9.004, 9.027, 9.029, 12.001, 13.001, 13.002, 13.100, 14.000 and 29.010. The
probe was not committed; the codec's own unit tests already pin these limits.

## Format ranges: no defect

Every main-type range (U8, V8, U16, V16, F16, U32, V32, F32, V64) is accepted
at its printed limits and refused one step beyond, with `OutOfRange`. Scaled
types 5.001 (0–100 %), 5.003 (0–360°) and 13.002 (0.0001 m³/h) map to the
expected raw values. The two documented sentinel collisions are unchanged:
8.010 327.67 % and 9.029 670 760 are refused because the raw value would be
the 7FFFh invalid-data code (`invalid-sentinel-precedence`).

## Subtype ranges: documented boundary, not a defect

The codec validates the main-type format only. It does not enforce a
subtype's narrowed range: 9.001 accepts −273.01 °C, 9.004 accepts −1 lux and
9.027 accepts −459.7 °F. The module header states this rule, and
KNOWN_LIMITATIONS §61 keeps it as a boundary. AR09 leaves it unchanged,
because enforcing subtype tables is the "unverified subtype table" work the
package excludes.

## GAP-AR09.2 — parameter-only time periods were not disclosed (fixed)

**Spec.** DPT-AS §3.8.3 gives 7.003 and 7.004 a resolution of 10 ms and
100 ms, and 7.006 a resolution of 1 min. Footnote 6 marks all three "Not
allowed for runtime communication", for parameters and diagnostic data only.
§3.9.3 says the same for 8.003, 8.004 and 8.006 (footnote a).

**Code before.** The codec encodes and decodes the raw counter, so "10" for
7.003 means 100 ms on the bus. It had no ruling for these subtypes, so
`encoding_rulings` returned nothing and a caller could not tell before a
write. The wire behaviour itself is not wrong.

**Fix.** These six subtypes now return a new
`time-period-raw-counter-parameter-only` ruling, which discloses both the raw
counter and the parameter-only restriction. The wire encoding is unchanged.
New test: `parameter_only_time_periods_disclose_their_raw_counter`
(RED before, GREEN after).

## Gates

Integrated public16 on 6fbb02c3 independently accepted: 16 exit0, Rust 3248/0/177 in 181 blocks, Web 2001, Chromium 131 plus probe 1, 887 frozen inputs, CLI knx 0.1.0-alpha.4+g6fbb02c3. First attempt refused (workspace 101): `http_bus_monitor` compared whole rows across two polls although `observedAgeMs` is measured per response (AR20); fixed in 6fbb02c3 (test only, 5 ms injected pause: old assertion fails, new passes).
