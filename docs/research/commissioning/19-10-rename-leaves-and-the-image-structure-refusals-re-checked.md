# 19.10 Rename leaves, and the image-structure refusals re-checked (2026-09-29)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="1910-rename-leaves-and-the-image-structure-refusals-re-checked-2026-09-29"></a>
<!-- BEGIN preserved record-2-019 -->
### 19.10 Rename leaves, and the image-structure refusals re-checked (2026-09-29)

Question: which of the 41 `image-structure` refusals and the
`parameter-evaluation` ones can be lifted from a source? Method: direct
PDFs under `knx-spec-kb/sources/`, the 103 local packages, throwaway
scripts.

- **[V] `Rename`/`ParameterBlockRename` retitle, nothing more.** 326 in
  `070nh` programs (270 `ParameterBlockRename`, 56 `Rename`): every one has
  only `Id`, `RefId` and `Text`, no children, sits under a `when`, and its
  `RefId` names a `ParameterBlock`. Neither the Schema23 PDF nor the
  Configuration Procedures PDF mentions either element, so the reading
  rests on that shape. The image holds no titles, so the image builder now
  lets the walk's `UnrecognizedNode` for these two kinds pass. The
  evaluator itself still reports them (the UI does not apply a rename, and
  `rename_button_and_repeat_are_reported_and_a_repeated_module_is_named`
  pins that). A reference *below* one would still be named by
  `RefBelowSkippedNode` and still refuse the image. 8 programs were held
  up only by these: 4 now plan; 4 (`A-008A-25/28`, `A-008B-25/28`) next
  reach a 6-bit field at bit offset 5, which crosses an octet and stays
  refused by the bit writer (§19 item 3).
- **[V] `NoBranchMatched` stays as it is.** The 51 remaining
  `parameter-evaluation` programs are value-dependent diagnostics inside
  **module instances** (e.g. `A-0008-23-8B6E`, `MD-2_P-7_R-9 = 0` with only
  a `when test="1"`). The image builder already accepts a legal uncovered
  value in the program's own tree; inside a module it would still refuse
  the module instance, which it does not image. Module instances are the
  real gap here, not the branch rule.
- **[V] Priority `High` stays refused.** *Resources* §4.18 codes 2
  priority bits in the Type-2 config octet as the transport priorities
  (`system`, `normal`, `urgent`, `low`); the schema's `ComObjectPriority`
  says `Low`/`High`/`Alert`. No PDF maps `High` onto a transport priority,
  and `urgent` vs. `system` is a choice, not a lookup.
- **[V] `ReadOnInitFlag` stays refused.** The Easy-3 config octet in
  *Resources* has no read-on-init bit (§19.2). The products' own base
  descriptors do not help: comparing their config octets with the product
  flags gave no consistent pattern (enabled: 62 descriptors, none matching
  under the assumed layout), so they are not evidence for any placement.
- **[V] Placement by `Property` stays refused.** In the 9 `MV-0705`
  programs it is object index 6, PIDs 31, 58 and 60, written with a
  property write, not a memory write. The download executor writes memory
  only; a property-writing step is a separate, untested capability.
- **[V] The Easy-3 group-object table on `MV-0705`.** *Resources* assigns
  the Easy-3 realisation explicitly to `0701h`; no table row names
  `0705h`. The products agree with its type-octet coding on both masks:
  2349 of 2695 non-zero `MV-0701` descriptors and 9743 of 10694 `MV-0705`
  ones hold the code for their object's single `ObjectSize`; the rest
  differ in ways that look like a base image not holding the default (e.g.
  type `0` for a 4-octet object). This is product evidence, not a PDF
  statement, and the `MV-0705` plans are untested.


<!-- END preserved record-2-019 -->
