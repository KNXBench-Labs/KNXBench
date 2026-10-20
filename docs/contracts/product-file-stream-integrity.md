# Product file compressed-stream integrity

## Decision (20 October 2026)

Each admitted unencrypted DEFLATE file in a product package or its outer wrapper
must contain exactly one complete stream consuming its entire validated
compressed byte extent. Refuse whole trailing bytes, including all-zero trailers
and a concatenated second stream, with the existing `InvalidZip` error and the
validated decoded member path. Preserve the underlying technical cause and,
for an inner package, its outer package context.

This is KNXBench's explicit integrity policy, not a claim that every ZIP producer
uses canonical compressed bytes. Preserve valid alternate block encodings,
multiblock streams, final-byte alignment bits and signed/unsigned descriptors.
Descriptors lie outside the checked compressed payload. Do not change supported
methods, filename decoding, encryption, layout checks, expansion limits or
project-container behavior.

## Evidence behind the decision

[Source research and scoped census](../research/2026-10-20-product-file-stream-integrity.md)
separate input readability from importer compatibility. All 12,772 measured
unencrypted DEFLATE file instances use exact extents; 14 encrypted entries remain
unmeasured. Synthetic production characterization establishes the currently
permissive behavior and original-archive retention separately. Those baseline
assertions are deliberately superseded by the admission policy, not silently
renamed as prior regressions. Retain their original source and run evidence
privately. No compatibility guarantee follows for unmeasured inputs.

## Smallest implementation boundary

Keep the normal bounded decoder and its CRC/size/error behavior. Extend the
existing secondary directory-stream check to admitted DEFLATE files only,
using fixed input/output buffers and checked counters. Require stream end,
exact compressed consumption and output length equal to the already checked
normal decode. Do not accumulate a second expanded copy. Stop with a named
refusal on excess output or non-progress/incomplete stream.

A second decode adds bounded CPU work; measure candidate and immutable baseline
with the same synthetic workload before delivery, and disclose that cost rather
than replacing the established decoder speculatively. Existing later read passes
remain unchanged over immutable archive bytes. No new abstraction or dependency
is needed.

## Integrity and regression requirements

- Before production changes, named refusal tests must fail against the baseline
  for zero/nonzero trailers and concatenation, descriptors, empty/nonempty
  ordinary files and multiple input/output chunks.
- Exercise standalone and outer-wrapper/inner-package paths, fresh/populated
  databases and rollback after an earlier newly installed inner package.
- Revalidate historically accepted hash-identical packages before source-name
  writes; do not use a retry shortcut to bypass the stricter boundary.
- Valid file payloads, raw archives, member hashes, reports and idempotent retries
  retain their existing behavior. Ordinary directory refusal variants stay intact.
- The existing forty directory-package tests are unchanged except for the single
  ordinary-file trailing-policy acceptance pin, whose changed requirement is
  explicitly identified and reproduced as RED. Characterization acceptance
  cases become named refusal regressions; positive/malformed controls remain.
- Require restored-source GREEN, focused Clippy, independent review and the
  maintained change-scoped committed-head gate with corpus coverage. Do not
  equate focused probes with full-workspace, browser, hardware or release proof.

## Delivery status

Policy recorded before implementation. Production changes and acceptance pending.
