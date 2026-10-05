# KL-153: scheme 10 admission (Claude, 2026-10-05)

## Decision
Admit exact `http://knx.org/xml/project/10` through the strict exact-namespace
member validation (ADR-0083). Same evidence standard as schemes 12–14:
observed vocabulary, fixture tests, release measurement.

## Evidence (aggregate only, public crawler corpus, read in place)
- Structure census: 146 packages, 1,391 XML members; per role 176 element
  kinds, 438 element/attribute pairs, 188 parent/child pairs, none outside
  scheme 11; no foreign namespace, no qualified attribute.
- Value census: 244 closed scheme-11 attributes compared; 33 differ by
  `false` spelling, other numbers, translation-unit version `0`, handler names.
- Release CLI, 853 files, baseline `e6099fb7` vs candidate: 692 → 837
  installed (standard), 852 with the large profile; only transition 145
  unsupported namespace → installed; originals rehashed unchanged.
- Gate: fmt, Clippy (productdb, cli, server), mutants (namespace not admitted,
  lenient path, master-language list) each fail named scheme-10 tests;
  1,589 passed / 0 failed / 89 ignored; private product matrix unchanged.

## Pitfall
The first master-language mutant survived: the test only checked `unknown > 0`.
It now asserts the exact Languages unknown set, so treating scheme 10 as a
foreign namespace (consumed fields reported) fails.
