# System-1 source-image assembly: offline contract

Status: integrated and published as `ccfe5b74`; all nineteen branch and twenty
actual merged stages accepted, including isolated corpus152/152 in33targets.
Exact local/tracking/live main equality verified. This result-only closure
retains runtime acceptance at that merge; it does not admit real-source images.
Nine targeted image tests, the owning ProductDB suite and Clippy pass. The
explicit private regression verifies thirteen named refusals in two packages;
this is **zero accepted complete real-source images**, not thirteen supported
applications. [Aggregate evidence](../evidence/offline-system1-image-2026-10-16.json).

## Architecture and scope

`knx_productdb::image::build_system1_image` is an explicit offline library entry
for retained `MV-0012` / `DefaultProcedure` sources only. The mask/style check
bounds this interpretation; it does not add that mask to any supported-device
allowlist. The same actual ProductDB importer, Dynamic resolver, type encoder
and parameter/image assembly serve the existing routes. There is no second
parameter engine, copied manufacturer data or manufacturer-specific branch.

The private layout discriminator selects Core's existing Type-1 descriptor
codec and indexed legacy association grammar. Common source-region and
source-default validation move from `image/system2.rs` to `image/legacy_source.rs`
with an explicit family argument; System-2 diagnostics and behavior remain the
same. Family-specific pointers/refusals live in `image/system1.rs`. Every
returned image includes a `system1-source-image` inference, never the System-2
inference. No Core/UI, transport, dependency or persistence-schema change.

Original Resources `03_05_01` v01.10.01, rendered printed pp223,246,261–266,
were independently inspected for the address-table position, source pointers,
Type-1 descriptor and reserved flag/type fields. These do not establish an
executable default procedure, source FixupList grammar or universal XML mapping.
The earlier [legacy codec contract](offline-download-codecs.md) and
[indexed-association contract](offline-download-system2-image.md#separate-table-grammars)
remain separate byte prerequisites, not hardware evidence.

## Data-integrity boundaries

- Require full absolute source Data, unique segment identities, consistent
  lengths/masks and nonoverlapping 16-bit source regions. Relative or mixed
  input refuses; no partial absolute image is returned.
- Require source address-table placement at0116h. Association and object-table
  placements must equal0100h plus the original octet at0111h/0112h respectively.
  Read those source pointers; do not manufacture or rewrite them. Parameters and
  tables cannot overwrite either pointer resource.
- Validate complete table capacities, reserved-region separation and source
  Type-1 descriptor fields before rewriting. Preserve original header/pointer,
  selector, inactive type and tail bytes according to the existing Core codec.
  A Type-2 encoder is not interchangeable: a deliberate substitution fails the
  independent literal-byte assertion.
- Reuse legacy sending-slot/FEh rules and capacity/refusal checks. Do not choose
  a receive link as sender. Explicit Type-1 Update, unestablished initial-read
  encoding or non-low priority mapping refuses rather than borrowing another
  family's flags.
- Reuse source-default corroboration for inferred Union/bit/multibyte fields;
  direct numeric octets may replace template defaults as already documented.
  Uncorroborated source bits, ambiguous byte order, unsupported types and
  overlapping active placements stay refusals, not endian/union guesses.
- `ProgramCode.has_fixup_list` is a nonpersisted view of direct `Static/FixupList`
  presence, including an empty declaration. Its contents stay in the original
  stored XML; no fixup semantics are inferred. This entry refuses any such
  declaration. P6 moves that refusal into the [shared image boundary](source-fixup-integrity.md)
  so Easy3 and System-2 also refuse; it does not reinterpret similarly named nested extension data.
  The legacy reader initializes the new view false; storage schemas are unchanged.
- Existing mask validation remains in force. Only the existing individual-address
  slot exemption applies; changed masked parameters/tables refuse. Image assembly
  and refusal do not mutate the product database or original source/package.

## Verified evidence and withdrawn interim result

The initial three existing-code measurements only established the old import/
reader/image behavior and public0012 refusal. They were not System-1 images.
Missing-API RED preceded implementation. A later runtime regression proved that
an apparent real-source image had ignored source FixupList; that interim result
is **withdrawn**, retained privately for traceability, and is not compatibility
or acceptance evidence. The strict guard precedes final measurement.

The final bounded thirteen-source run and the durable opt-in regression agree:
seven FixupList refusals, five source-default-bit disagreements and one mid-octet
Union placement refusal. These are the first observed boundary per source,
not an exhaustive list of everything required to support those products.
The original twenty-source discovery was bounded, not a compatibility census.
Source hashes, imported identities, original package hashes and unchanged public
admission are checked. The private manifest is configured explicitly through
`KNXBENCH_SYSTEM1_SOURCE_CASES`; an ordinary skipped test is not real-source proof.

Nine normal tests cover independent literal bytes, actual evaluator integration,
unchanged planner refusal, family/style boundaries, flags/reserved fields,
pointers/capacities/overlap, relative/dataless sources, masks/encoding, source
FixupList refusal and exact source ancestry. A Type-2 substitution mutant fails
the intended assertion; exact source restoration and all nine tests pass.

## Still open

No complete real-product System-1 image, default-procedure source translator,
executable source plan, whole-mutation durable recovery, application/CLI/HTTP/UI
route or field validation is delivered. `build_download_image`, public mask
admission and the memory planner are unchanged; returned offline images remain
non-executable. `FixupList` interpretation and independently discriminating
parameter/Union encoding evidence are next research dependencies, followed by
separate procedure/recovery contracts. This does not implement System300,
System7, coupler, RF, Secure or USB paths. No bus or vendor executable is used.
