# Relative source segments and absolute-image refusal

## P4b scope

`knx_productdb::code::ProgramCode::relative_segments` retains source
`ApplicationProgram/Static/Code/RelativeSegment` data separately from absolute
segments. It is a source adapter view, **not** an allocation, product image,
executable sequence, domain/persistence schema change or new family admission.
No dependency, transport, UI or storage migration is introduced.

The [offline expansion](offline-download-expansion.md) and System-B relative
image/sequencer integration remain open. Reading source bytes does not establish
how to construct generated tables, substitute load images or recover mutations.

## Verified baseline and source basis

An offline Release probe used the production importer, value resolver, Dynamic
evaluator, code reader and absolute image builder on the same three applications
in one private package as [P4a's baseline](../research/2026-10-16-systemb-offline-prerequisites.md).
The sources activated 186 parameter references and 12 object references in total.
All 54 diagnostics were `NoBranchMatched`; the current evaluator marks
that class as not hiding references. This measures the evaluator's current
behaviour, not independent validation of every activation or a zero-warning image.

Before this change, the reader returned zero absolute segments and no typed
relative view. Each actual image attempt refused at `ComObjectTable: no
CodeSegment`. That is an observed guard, not proof that table generation is
impossible or that the relative source is malformed. The source contains one
relative segment per selected application and generated table declarations
without an explicit absolute segment/offset placement. No absent location is
filled with zero or borrowed from an Easy-3 product.

The new parser's supported source grammar is corroborated by these exact retained
original XML sources, with independent ElementTree/strict-base64 decoding.
This is **not full KNX XML-schema validation or a complete profile audit**.
The original specification reading and its unresolved merge/substitution and
allocation contracts remain as documented in the linked research; no modern XML
semantics are invented from normative table layouts.

## Source-view contract

- `Id` must be nonempty. `Offset` and `Size` must explicitly fit the supported
  unsigned 32-bit source fields; `LoadStateMachine` must explicitly fit one
  octet. These are this reader's admission bounds, not a claim that every valid
  schema generation has these exact constraints. No missing default is invented.
- Unknown attributes are retained verbatim in `other`. Unknown child elements
  explicitly refuse the view; their original XML remains in the existing
  verbatim source blob. Nothing claims that unknown semantics are executable.
- `Data`/`Mask` use the same base64/whitespace/CDATA and decoded-length checks as
  absolute segments. The extraction is shared, not a parallel decoder.
  Duplicate relative `Data` or `Mask`, child markup in a blob, invalid base64
  and inconsistent declared size refuse explicitly.
- Missing data remains `None`, including a very large declared size; it does
  not allocate a zero-filled buffer. Present empty data is distinct from absence.
  Mask bytes are retained but not interpreted as a write policy.
- IDs must be unique across both known segment kinds throughout the parsed
  program's Code blocks. Duplicate absolute IDs now explicitly refuse too;
  this intentionally tightens ambiguous input, rather than choosing a winner.
- Each segment-kind vector retains its own source order. The separate vectors
  are not an executable interleaved load sequence. `ProgramCode::segment` stays
  absolute-only; relative offsets and load-state-machine indices do not become
  absolute addresses or interface-object indices.
- Legacy source projection explicitly supplies an empty relative vector; no
  relative records are synthesized from legacy absolute data.

## Image integrity

Both `build_download_image` and `build_system2_image` share an early refusal if
any relative source segment is present: `relative source segments require a
relative image builder`. Even mixed absolute/relative input must not produce a
partial absolute image that silently omits the relative bytes.

The new mixed-source regression initially returned an `Ok` image without this
check, demonstrating the omitted-source mechanism. It now refuses for both
entry points without mutating the database. This strengthens a refusal boundary;
it does **not** implement relative images or promote any mask.

## Evidence and remaining work

Ten source-reader tests cover literal metadata/bytes, absent versus empty data,
namespaces/CDATA, kind separation/order, explicit fields, malformed blobs,
unknown children, duplicate identities and correct application/Code ancestry.
The missing-field compile RED was observed before implementation. The image
boundary has its own named runtime RED/GREEN. An initial focused combined command
filtered out the integration tests; its zero-test target is not accepted as
coverage. A subsequent explicit unfiltered target must pass. An initial owning
Clippy invocation rejected a nested conditional; the correction is verified
separately, with the refused attempt retained.

A second network-isolated production probe matched all three relative metadata
records and Data/Mask digests against an independent original-XML oracle;
original packages and the product database remained unchanged. The new explicit
image refusal was also asserted. The temporary calibration test was saved
privately and removed before the full candidate gate. No raw product source,
per-product identifiers, payload hashes or licensed pages are published.

The reusable opt-in `relative_source_corpus` test consumes
`KNXBENCH_RELATIVE_SOURCE_CASES`, a private JSON array of at most 16 cases
(maximum manifest size 64 KiB). Each case explicitly supplies `package`,
`program_id` and `relative_segments`; every expected segment supplies identity,
offset, size, load-state-machine index, other attributes and optional Data/Mask
SHA-256 values. Unknown manifest fields refuse. Expected values must be prepared
independently from the original XML, never generated by the reader under test.
Run the target with `cargo test --offline --release -p knx-productdb --test
relative_source_corpus -- --ignored`; wrap it in the documented network-isolated
environment. Missing explicit configuration is not a passing opt-in test.

[Aggregate evidence](../evidence/offline-relative-source-segments-2026-10-16.json)
separates focused proof, full branch/merged acceptance and publication. A
source-bound relative image builder, generated tables, byte-order/unsupported
parameter semantics, scope/substitution translation and complete durable recovery
still require their own implementation and evidence. The earlier absolute-image
refusal is historical evidence, not relabelled as the corrected candidate's
runtime output.
