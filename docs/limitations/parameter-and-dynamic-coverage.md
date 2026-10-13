# Parameter and Dynamic coverage

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-parameter-and-dynamic-coverage)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## PDB-9 parameter and Dynamic coverage boundary

**Update 2026-10-10 (two observations, no change).** From programs embedded in
a privately supplied project. **Grid layout:** some `ParameterBlock`s use
`Layout` with `Rows`/`Columns` and `Cell` positions. The evaluator treats
`Rows`/`Columns` as recognized-inert and reports `Cell` as an unknown
attribute, so such fields render as a list, not a grid. **Value forms:**
project values of `TypeColor` parameters occur in more than one lexical form
(plain digits and hex-like strings without prefix), `TypeFloat` values as
integer and decimal, `TypeTime` values as integer. `PDB-05`'s
non-empty/XML-safe floor stays. No encoding is inferred from these occurrences
and raw values are never normalized.

Since schema v15 ([ADR-0041](../adr/0041-unmodelled-kinds-and-dynamic-nodes-are-named-never-hidden.md))
every `ParameterType` kind observed in the corpus is stored as its own kind,
and a reference below a Dynamic node refused for a structural reason is named
by `RefBelowSkippedNode`. What is still not modelled, each reported rather
than dropped:

- **Repeat expansion.** `Repeat` (16 in the corpus) is `UnrecognizedNode`;
  its `Module` is named by `RefBelowSkippedNode` but not expanded, so the
  repeated module's parameters are not offered for editing. Expansion needs
  the repeat count (`@Count` or a controlling `@ParameterRefId`) and
  project-side instance matching.
- **Renames and buttons.** `ParameterBlockRename` (270), `Rename` (56) and
  `Button` (20) are `UnrecognizedNode`: a conditional rename is not applied
  to the displayed label, and a button's `@EventHandler` script is not run.
- **Calculations and allocators.** `ParameterCalculation` (1,236 in 91
  programs) and `Allocator` (94 in 14) are reported unknown elements with
  their attributes, and the program bytes are retained; their
  transformation scripts and allocation ranges are not evaluated. Since
  ADR-0080 (2026-10-05) every `ParameterRef` a calculation names on either
  side is read-only (`manufacturerCalculation` warning): editing the input
  without running the script would leave the derived values stale. The
  wider 3,599-program census has 116,799 calculations in 809 programs
  (JavaScript and VBScript), so this removes many fields from editing — by
  design. All 1,199 corpus `NumericArg/@AllocatorRefId` bindings carry no
  `@Value`; the evaluator reports `ModuleArgumentNotBound` and leaves their
  `{{Name}}` placeholders verbatim. The allocation rule itself is still not
  documented ([census](../contracts/PARAMETER_SEMANTICS_BOUNDARY.md#ar07-corpus-census-and-write-authority--2026-10-05)).
- **Display-only type attributes.** `UIHint`, `Increment`, `DisplayFactor`,
  `DisplayFormat`, `Pattern`, `Encoding`, `AddressType`, `TypeTime/@Unit`,
  `TypeColor/@Space`, `TypePicture/@RefId`/`@HorizontalAlignment` and
  `TypeRawData/@MaxSize` are reported with a sample, not stored in columns:
  the ordinary parameter rows do not use them. Since 12 October 2026 an
  opt-in **Checked editor** in the device editor's Parameters tab
  ([host](../contracts/parameter-editor-host.md#workbench-host-12-october-2026))
  reads the retained source for Time (decimal count, `Unit` shown
  uninterpreted, no hh:mm:ss conversion) and integer `UIHint="Slider"` Number
  fields. Colour/Picture get only an unsupported-codec original-value view:
  no colour picker, no picture display. PDB-04 stays open.
- **`Color`, `Picture`, `Raw` values** get only a non-empty, XML-safe check;
  the Project Schema documents no value encoding for them.
- **Not every absent reference is enumerated.** The branches of a `choose`
  refused for a value-dependent reason (`MissingValue`, `NonNumericValue`,
  `NoBranchMatched`) are conditionally hidden by design — the `choose` is
  named, its branches' references are not listed. A `Module` that is not
  expanded (`ModuleDefNotFound`, cycle, depth or expansion budget) is named
  by its own diagnostic, but its `ModuleDef`'s references are not listed.
  `RefBelowSkippedNode` reports share the `MAX_MODULE_ACTIVATIONS` budget;
  past it, output is truncated with one budget diagnostic.
- **Install reports stay historical.** `package_install_unknown` rows
  written before v15 keep their `Element TypeTime`/`TypeColor` entries; the
  v14→v15 backfill corrects `ingest_unknown` (the per-blob report), not the
  install-time snapshot — the same rule as the v12→v13 precedent
  (COMPATIBILITY: "Historical encounter reports remain historical").
<!-- END retained catalogue record -->
