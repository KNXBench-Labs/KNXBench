# ADR 0170: Every matching `when` of a `choose` is active

Date: 2026-11-09
Status: Accepted

## Context

The `Dynamic` evaluator took the first `when` whose `@test` matches and
ignored later siblings, with `@default="true"` as the fallback. That rule was
an inference from the absence of overlapping tests in the original research
sample ([product database research](../research/product-database.md), §4.3);
the sample said nothing about what overlapping tests mean.

They are not rare: one manufacturer's push-buttons, glass push-buttons and
logic modules use them routinely (`test="2 3"` next to `test="2"`). In the
tester project of issue #10 one push-button was refused because the second
block's object, linked to a group address, counted as inactive.

ETS 6.4, with two of these programs added to a project and downloaded to
KNXBench's own simulator, showed what it does
([research note](../research/2026-11-09-overlapping-when-tests.md)).

## Decision

1. Every `when` of a `choose` whose test matches the controlling value is
   active, in document order. Their parameters, objects and nested elements
   are all activated; a reference reached twice is activated once.
2. `@default="true"` stays the branch for a value no test matches.
3. No match and no default stays `NoBranchMatched`, as before.
4. The rule covers every test kind. ETS was observed with value lists and
   single values only; overlapping comparisons (`>5` next to `>10`) follow the
   same reading by assumption.

## Alternatives considered

- **Keep the first match.** ETS showed the second block's parameters and
  objects and wrote their values.
- **Refuse overlapping tests.** ETS reads them without complaint; refusing
  would keep devices the observation explains unsupported.

## Consequences

The parameter panel, object activity and the download image follow the same
evaluation. A sibling `when` with an unparsable test after a match is now
read and reported, so objects it might hide become undetermined. Two members
of one union reached through two matching branches follow the existing union
rule (the later member wins and is reported; legacy layouts refuse), and two
refs of one parameter with different values refuse the image
(`ConflictingValues`). In the tester project one more device plans (7 unsupported, 68
untested) and one planning device writes five octets differently, now as ETS
does. The ignored private corpus pins move (see the research note). The rule
rests on two programs of one manufacturer; it stays untested until a device
confirms it.
