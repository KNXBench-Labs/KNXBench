# Product-database series for Alpha 14 (Claude, 2026-10-18)

Census `docs/research/2026-10-17-product-database-gap-census.md`, steps 2–4.

- Vendor ZIPs (ADR-0140): wrapper detection = no own `knx_master.xml` and at
  least one `*.knxprod` member (ASCII case-insensitive). One transaction, one
  shared expansion budget, atomic refusal naming the inner package, other
  members listed (path/size/SHA-256/kind), never stored, no nesting.
- The first corpus run refused exactly the 4 wrappers carrying a macOS
  AppleDouble sidecar named `._x.knxprod`; the content rule (RFC 1740 magic
  `00 05 16 07`) fixes that without trusting names.
- Packed `TypeTime` units: layout undocumented locally, so they fall back.
  Collection: 1,456 declarations, 1,004 plain, 452 packed.
- `IndentLevel` and object size: presentation only, no schema change.
- Pitfall: limitation dossiers end in a hash-pinned retained block; a dated
  update goes above `<!-- BEGIN retained catalogue record -->`, never inside it
  (`tools/tests/test_limitations_catalogue.py`).
- Pitfall: a new Rust test file needs a one-sentence first-line `//!` header or
  `xtask check-headers` counts it as headerless against the ceiling.
- Deferred to Alpha 15: size ceilings (step 5), `SuffixText` (product-DB
  column), inline sections, `ShowInComObjectTree` folders, object priority.
