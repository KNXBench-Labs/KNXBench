# ADR 0044: An application's download data is read on demand from the stored product file

Date: 2026-09-28
Status: Accepted
Session: 7 (integration / hardening)

## Context

Downloading an application program means writing memory: segment images,
the group tables, and the parameter values laid over them, in the order the
program's load procedure prescribes. `[D]` The KNX Cookbook *Load Controls*
(`02_03_01` v01.00.02, pp. 6–7, Figure 4) describes the tool's side of that
job: the *"default memory image"* shipped with the product is the base, and
the tool *"modifies the default image according to the … project settings,
being: 1) group objects 2) group addresses 3) device parameters"*. The load
controls then say which management procedure writes it.

Everything a download needs is therefore in the application program's XML.
None of it is in the product database's tables (KNOWN_LIMITATIONS §7):

- `Static/Code/AbsoluteSegment` (`Id`, `Address`, `Size`, optional base64
  `Data` and `Mask`);
- `Static/{AddressTable,AssociationTable,ComObjectTable}` placement
  (`CodeSegment`, `Offset`, `MaxEntries`);
- `Static/LoadProcedures/LoadProcedure` and its `LdCtrl*` steps;
- `Static/Options`.

Measured over the 310 application-program files in the private corpus on
this machine (2026-09-28):

- 276 files carry `AbsoluteSegment`.
- 980 segments have `Data` and 305 have `Mask`. Every decoded length equals
  `Size`.
- 303 files carry `LoadProcedures`, all directly under `Static`.
- 19 distinct `LdCtrl*` element kinds occur; `LdCtrlAbsSegment` has 1,486
  occurrences.
- Only `OnError` ever appears as a child element of a step.
- 40 files use `MergedProcedure`, whose steps come from the mask's own
  procedure in `knx_master.xml`, not from the program.

ADR-0011 already keeps every ingested file verbatim in `source_file` and
anticipates this: *"unparsed never means gone — it means the bytes are one
`load_source_file` call away"*.

## Decision

`knx-productdb` gets a module, `code`, that reads an application program's
download data **from its stored source blob on demand**. The lookup goes
`application_program.source_sha256` → `load_source_file` → one streaming
pass over that program's `Static` section. It returns:

- the absolute segments, with `Data` and `Mask` decoded and their lengths
  checked against `Size`;
- the three table placements;
- the load procedures, each with its `MergeId` and its ordered steps;
- the `Options` attributes, verbatim.

Steps the module models become typed values. Any other step, or any
attribute a modelled step does not know, is **kept as a named, unmodelled
step with all its attributes**, never skipped. A caller that is about to
touch hardware must refuse a procedure containing one.

Nothing new is stored and the schema does not change. The source blob stays
the only copy, so there is nothing to backfill, migrate or keep consistent.

What the module does **not** do:

- **Interpret `Mask`.** No KNX PDF defines it (checked: *Project Schema23*
  and Volumes 2, 3 and 8). It is returned as bytes, and a download treats
  the octets it covers as untouched, see below.
- **Expand `LdCtrlMerge`/`MergedProcedure`.** Those programs are reported
  as not downloadable until the mask procedures are read too.

## Alternatives considered

**New tables filled at ingest, with a migration backfilling existing
databases from their blobs.** This is the shape `dynamic_node` took, and it
would make segments and steps queryable in SQL. Rejected for now:

- The only consumer is a download of one program at a time, which needs
  the whole of one program's data, never a cross-program query.
- A schema version plus a backfill is a lot of machinery for that, and a
  second copy of 22 MB-class data that can disagree with the blob.
- If a query need appears, the parser this ADR adds is exactly what the
  backfill would call, so nothing is lost by waiting.

**Parse the XML in `knx-app` or `knx-net`.** Rejected: product-file
grammar belongs in the crate that owns the product files (ADR-0005/0011).
A second XML reader elsewhere would duplicate `xml.rs`'s namespace and
error handling.

**Keep hand-built images per product (the offline analysis that preceded
this ADR).** Rejected outright. CLAUDE.md forbids hard-coding
manufacturer-specific data into the application.

## Consequences

- A download can be assembled from data the database already holds for
  every ingested product, with no re-ingest.
- Every read re-parses one file: a few megabytes at most, once per
  download. That is acceptable until measured otherwise.
- `Mask` stays uninterpreted. The download builder must not change any
  octet a segment's `Mask` marks, and must **refuse** rather than overwrite
  when a desired change hits one. On mask `0701h` the one observed case is
  the device's own individual address in the group address table
  (`AS-4000`, octets 1–2). Those octets must survive untouched anyway, and
  the `[V]` read-back of a real device shows them holding its address.
- Unmodelled steps are loud by construction. They are listed with their
  name and attributes, and a hardware write refuses them by name.
- Tests: the parser is unit-tested on inline XML (malformed base64, length
  mismatch, unknown step, missing placement) and corpus-tested, `#[ignore]`d
  like the other private-corpus tests, against `A-0027-15-0BAC`, whose
  base images are pinned by the octet-exact rebuild of a real device's
  tables.
