# 2026-10-17 — Claude — Product-database gap census

Owner request: analyse the product databases newly placed in the local
collection for KNXBench implementation gaps, and check whether further
parameter and communication-object layouts can be derived for the UI.

## What was done

- Read handover, layout census, workspace contract, ADR-0137 context, KL-003,
  KL-151, KL-156, the Dynamic parser's modelled attribute table, the layout
  reader, write-authority code and the object-table DTO.
- Wrote a stdlib scanner in agent scratch (nested ZIPs in memory, streaming XML
  without size cap, DTD refusal, allowlisted enumeration values only) and ran it
  over the whole collection (145 files). New vs old split by program id.
- Built `knx` (release) and ran `knx products ingest` on every new top-level
  file, every inner `.knxprod` (isolated throwaway databases, deleted after),
  the two complete databases with `--allow-large-package`, and
  `inspect-legacy` on every `.vd5` with the owner's stored password file.
- Shape probes: `Inline` parents, `Rename` targets, `IndentLevel` values,
  `ShowInComObjectTree` contents, block `Access` placement, repeated
  `ModuleDef` instances, object references without a block.
- Read Project Schema 23 v01.00.00 (local spec corpus) for `Access_t` (p6) and
  `GroupObjectTree`/`Node_t` (pp51–52). Packed `TypeTime` units, `Inline`,
  `IndentLevel`, `Rename`, `CollapseIfEmpty` are not defined there.
- Originals verified byte-identical after all runs (145/145).

## Findings (see the research dossier)

- Correctness: block/separator `Access` ignored by write authority; separators
  outside grids never shown (not documented before).
- Install: 22 of 24 wrapper ZIPs fail as "missing knx_master.xml"; 38 of 39 inner
  packages install; complete databases need the large profile (web has none);
  one produced a 4.56 GB product database; one ETS3 `.vd5` exceeds the 24M value
  budget.
- 92 of 546 new programs exceed the 8 MiB presentation-reader ceiling.
- Checked time editor treats packed units as decimal counts.
- Derivable layouts with evidence level: P1–P8 (parameters), O1–O4 (objects).

## Not done

No code change. No KL catalogue entry was added (the KL-003 dossier gained a
dated section; catalogue entries come with the fix packages). Scratch data
(scanner, JSON, probe logs) stays in agent scratch and is removed after
delivery.
