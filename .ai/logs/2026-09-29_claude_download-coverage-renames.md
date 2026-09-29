# 2026-09-29 — Claude: Rename leaves and the image-structure refusals

User go: "leg los" — continue the download-coverage work with the
`image-structure` refusals.

## Findings (sources: direct PDFs under `knx-spec-kb/sources/`, 103 local packages)

- Priority `High`: *Resources* §4.18 codes the transport priorities
  (system/urgent/normal/low); `ComObjectPriority` is Low/High/Alert. No PDF
  maps High. Stays refused.
- `ReadOnInitFlag`: no bit in the Easy-3 config octet; product base
  descriptors show no consistent placement. Stays refused.
- `Property` placement (9 `MV-0705` programs, object index 6, PIDs 31/58/60):
  needs a property write; the executor writes memory only. Stays refused.
- `MV-0705` GrOT: *Resources* names only `0701h` for Easy 3; product type
  octets agree on both masks (documented as product evidence, RESEARCH §19.10).
- `Rename`/`ParameterBlockRename`: 326 empty leaves under `when`, `RefId` a
  `ParameterBlock`. Image builder now tolerates their `UnrecognizedNode`.
  Evaluator unchanged (an existing test pins that it reports them).
- Remaining 51 `parameter-evaluation`: `NoBranchMatched` inside module
  instances; module instances are the real gap.

## Change

- `crates/knx-productdb/src/image.rs`: tolerate the two rename kinds;
  RED→GREEN test `a_rename_does_not_hold_up_the_image_but_a_reference_below_one_does`.
- Corpus pin: 1 verified, 73 untested, `parameter-evaluation` 51,
  `parameter-value` 13 (4 former Rename programs hit a 6-bit field at bit 5).
- Docs: RESEARCH top entry + §19.10, KNOWN_LIMITATIONS, IMPLEMENTATION_STATUS.

An attempt to make the evaluator itself treat renames as inert was reverted:
the existing ADR-0041 test pins that they stay reported, and the UI
genuinely does not apply them.
