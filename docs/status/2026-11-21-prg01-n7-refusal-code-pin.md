# PRG-01-N7: refusal classifiers exhaustive and pinned (2026-11-21)

Slice: [contract](../contracts/download-refusal-reason-codes.md),
[ADR-0185](../adr/0185-download-refusal-classifiers-are-exhaustive.md).

- `plan_category` and `image_category` list every variant; behaviour unchanged.
- Tests: `every_plan_refusal_keeps_its_category_and_code` (all plan variants)
  and `image_refusals_without_payload_keep_their_category`.
  `ImageError::Overlap` and `::Parameter` need heavier payloads and are covered
  only by the exhaustive match plus existing tests.
- Software only: no bus, hardware, corpus re-measurement or consumer change. The
  122/758 baseline is untouched (private inputs not run here). PRG-01 stays open.
