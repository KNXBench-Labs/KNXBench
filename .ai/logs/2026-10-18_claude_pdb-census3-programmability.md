# 2026-10-18 — Claude — product-database census 3 (programmability)

Docs only. 19 new files in the owner's product-database folder, measured with
the release build of `482f0fcd`: declaration scanner and two depth probes,
production `knx products ingest` (standard, then large profile), legacy
`inspect-legacy`/`import-legacy`, and `knx products coverage` per throwaway
database. Originals byte-identical afterwards (SHA-256).

Result in `docs/research/2026-10-18-product-database-programmability-census.md`:
177 new programs, 3 build a download plan (all `0701h`), 0 verified. New gaps
G10–G16; the two data-integrity ones (G11 legacy upper-case tables read as
empty, G10 refusal naming the wrong cause) are the suggested next package.

Pitfalls:
- `.etsapp` files carry an `AddInManifest.xml` with `ApplicationProgram`
  elements that only name programs; count only KNX-namespace elements.
- A `BinaryValue` oracle that follows only `Parameters/Parameter` misses Union
  members; follow `Union/Memory` + `Parameter/@Offset`.
- The first reading ("BinaryValue blocks 71 programs") was wrong as a lever:
  the parameters are property-placed (PID 78), so the next refusal would be the
  placement. Always resolve the refused parameter's placement before naming a
  fix.
- `inspect-legacy` exits 0 on a file it reads as "0 products"; a census that
  checks exit codes only misses silent emptiness. Read the product count.
- `os.wait4` peak RSS of a forked child includes the parent's memory at fork
  time; publish small values as upper bounds.
- The session Python kernel can lose state between calls; persist parsed
  results to the evidence folder and reload.

Evidence (scanner, probes, logs, JSON): agent evidence folder
`pdb-census3-20261018`, not in Git.
