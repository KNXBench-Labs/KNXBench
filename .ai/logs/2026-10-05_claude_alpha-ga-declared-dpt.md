# AR09 / ADR 0078: declared group-address DPT (2026-10-05 11:50 CEST)

Evidence first: Project Schema23 §1.2.7 (`DatapointType knx:IDREF`, "sizes
must match"); KV v2.5 demo 13/13 declared, 0/26 linked instance DPTs.
Self-review found the published ADR's E3 wrong (retained attributes ARE
keyed with `[@Id=…]` since 2026-09-20, `xpath.rs`); corrected in place with a
dated note, migration now lifts keyed rows.

Implementation: field on GroupAddressEntry (72 literal sites, scripted,
`Default::default()`), ProjectInfo counter, resolver + `format_width_bits`,
importer (`override_dpt` gained a `kind`), store v10 + migration in a
savepoint, consumers (project map, projection `dpts`, CSV, diff, compare).

Review findings fixed before gating: project diff and import-compare views
ignored the new field (would have reported "no change" for a changed
declaration) — added. Projection doc comment kept byte-identical because it
is copied into the locked web bindings; a `//` note explains the new meaning.

Mutation sweep 9/9 killed (declaration ignored, width check off,
unverifiable passes, not-lifted ignored, importer drops, migration keeps row,
migration ignores schema, projection linked-only, CSV width warning).
