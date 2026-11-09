# Overlapping `when` tests: every match is active (9 November 2026)

Evidence for [ADR-0170](../adr/0170-every-matching-when-is-active.md). Issue
#10's tester project; ETS 6.4.1, German UI, analysis VM, downloads recorded
against KNXBench's own simulator (mask `0705h`). Logs, captures and exports
stay private.

## Question

A `choose` may hold sibling `when` elements whose tests overlap, for example
`test="2 3"` and `test="2"` on the same controller. The evaluator took the
first match only (an inference, [product database research](product-database.md)
§4.3). In the tester project a push-button with such a `choose` was refused:
the second block's object, linked to a group address, counted as inactive.

## Corpus

Overlapping tests sit in 141 `choose` elements of 13 programs of the private
product corpus (two manufacturers) and in 10 programs of the tester project,
all but one from one manufacturer: push-buttons, glass push-buttons, logic
modules.

## Observations

1. **`[V]` Objects of both blocks.** A push-button program (`0094h`), all
   defaults except the second button's function, has
   `choose` "buttons 1/2" with `when test="2 3"` (block of button 1) and
   `when test="2"` (block of button 2). At the default value `2` ETS 6.4 shows
   both blocks and lists the objects of both buttons (0, 1, 10, 12, 20, 21,
   30, 31, 71, 72, 73). KNXBench's every-match evaluation activates exactly
   these eleven; first-match activated only button 1's.
2. **`[V]` Parameters of both blocks reach the image.** A glass push-button
   program (`018Bh`), standby "whole screen" (`2`) and three status lines
   (`2`), has `when test="2"` and `when test="1 2"` siblings for both
   controllers. ETS shows lines 1, 2 and 3 with their status elements and font
   sizes. In ETS's download the octets of line 1 and line 3 hold their values
   (`4C87h` = `00h`, `4C8Fh` = `02h`); first-match left them at the template
   (`FFh`).
3. **`[V]` Whole segments.** The push-button's parameter segments `456Ch` and
   `4800h` equal ETS's octet for octet with every-match (17 differing octets
   with first-match). The logic module of the [float note](2026-11-09-float-parameters.md),
   which has overlapping tests too, stays identical.

## Not covered

- Three octets of the glass push-button (`4E29h`..`4E2Bh`) still differ: ETS
  writes `FFh`, the value an active `Assign` gives these invisible parameters;
  KNXBench writes their template. That is independent of this rule (first
  match differs there too) and stays open.
- Both observations come from one manufacturer's programs.

## Effect

Tester project: one more device plans (7 unsupported, 68 untested); one
planning glass push-button writes five octets differently. The private
product-corpus coverage classification is unchanged program by program
(758 rows identical with and without the rule).
