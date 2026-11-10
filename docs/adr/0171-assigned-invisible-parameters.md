# ADR 0171: Invisible parameters an active `Assign` sets carry the assigned value

Date: 2026-11-09
Status: Accepted

## Context

Under `DownloadInvisibleParameters="Background"` KNXBench writes only active
parameters; an invisible parameter's octets keep the product template
(ADR-0161 covers property placements only). `Assign` elements in an active
branch set hidden parameters; KNXBench does not execute them (ADR-0168), but
ETS does and stores the result in the project.

ETS 6.4, downloading a glass push-button of issue #10's tester project
(mask `0705h`) to KNXBench's own simulator, wrote `FFh` into three octets
whose parameters are invisible and whose template holds `06h`, `08h`, `0Ah`:
an active `Assign` sets them to `255`, and the exported project holds `255`
for them ([research note](../research/2026-11-09-overlapping-when-tests.md)).

## Decision

1. The evaluation result lists the `Assign` elements the final pass reached
   in an active branch (a controller marked inactive activates nothing, so
   nothing below it), and whether each target controls a `choose` the
   evaluation consulted.
2. In a memory (`070nh`) image of a program with
   `DownloadInvisibleParameters="Background"`, an invisible parameter that
   such an unscoped `Assign` sets is written like an active one, with the
   assigned value (a literal, or the value of the source parameter), checked
   against its type. ETS runs the assignment whatever the project held, so a
   project still holding the default is written the same way.
3. KNXBench does not execute `Assign` during evaluation, so the image is
   refused, not guessed, where writing the assigned value could contradict it:
   - an active `Assign` that cannot be read;
   - two assignments that disagree, or a source without a value;
   - a target that controls a `choose` the evaluation consulted, or is the
     source of another active `Assign`, and holds another value (the
     evaluation used the stored one); a `choose` in a branch the evaluation
     never reached steers nothing;
   - a target whose parameter a visible reference holds with another value.
4. A visible target keeps its own value (ADR-0168 compares it). Only plain
   `Memory` placements; union members, property placements, module scopes,
   other builders and other policies keep their previous handling.

## Alternatives considered

- **Execute `Assign` in the evaluator.** Still the general answer for
  controllers (ADR-0168); for the image it is enough to write the value an
  active assignment yields.
- **Require the project to hold every assigned value.** An ETS export does,
  but a device added in KNXBench holds its defaults; five planning programs of
  the private corpus would have been refused although ETS would assign the
  value. The requirement stays only where the value steers the evaluation.

## Consequences

KNXBench's images of three programs equal ETS's parameter segments octet for
octet. In the tester project one planning device writes three octets
differently, as ETS did. The rule rests on one program's observation; it stays
untested until a device confirms it.
