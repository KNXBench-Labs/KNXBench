# Relative System-B image integration: source-bound gaps (16 October 2026)

Baseline: published P4d runtime `894f14e0`, documentation closure `1b00ed76`.
This is a bounded original-source investigation, not a new image builder,
executable plan, recovery implementation or device-support claim.

## What the complete constructor does not decide

P4d's [complete object-table constructor](../contracts/offline-systemb-object-table.md#p4d-explicit-complete-construction-16-october-2026)
requires a complete explicitly numbered descriptor set. A direct bounded read of
the same three retained application sources in one original product package
finds sparse declared communication-object numbers in all three. The product
number set is not an explicit dense one-based descriptor array. These are three
selected sources in one package, not independent family coverage.

Neither sorting/compacting those numbers nor padding to the highest number with
invented disabled descriptors is justified by that observation. A correct pure
codec does not provide the missing product-number-to-ASAP mapping, inactive-entry
policy, activation transformation or generated-table size. Existing absolute
image code uses a legacy octet-sized object number in `image.rs::active_object`;
reusing it for these wide source numbers would refuse or lose the required
representation. It must not be widened without a separate System-B mapping.

## Source-level locations are present, not a universal runtime contract

The selected original master has an unqualified `HawkConfigurationData` and a
separate `LegacyVersion`-qualified wrapper. Reading only the explicit unqualified
wrapper, its retained resource declarations name relative-memory table resources
for address, association and object tables, with references 1, 2 and 3, property
7 and start offset zero. Separate pointer resources declare four-octet lengths.

Thus it would be wrong to claim that this selected source has no table-resource
locations at all. These are source declarations tied to that master, not a
newly admitted runtime mapping, proof of all resource flavours, property-array
framing or a general extension of a different profile's procedure. The qualified
wrapper must not be selected by first-descendant lookup or silently merged.

Canonical application options were rechecked at `ApplicationProgram/Static/Options`,
not at the application's direct `Options` child. None of these selected options
establishes the missing numeric byte-order contract. No absence claim is based
on searching the wrong parent.

## Original profile and procedure bodies

Freshly rendered original pages were visually inspected, including their printed
footers. Full licensed text and page images remain private.

- **Profiles v02.01.01**, p13 Table1: the informative overview names `07B0` and
  `17B0` under System B. This is family classification, not modern product XML
  transformation semantics. P28's `57B0` column belongs to KNX IP medium features;
  it is not the expected missing `07B0` download-translation chapter.
- **Profiles v02.01.01**, §4.2.10 p41: System-B memory allocation and its
  implementation-specific range refer to the configuration procedures. The
  clause does not turn source segment offsets into absolute addresses or supply
  product object-number mapping.
- **Configuration Procedures v02.01.01**, §3.9.3 example p75: table allocations
  and relative writes use current generated-table image sizes. They are not
  permission to forward the selected modern source's large numeric write-size
  literals unchanged or infer their substitution from integer width.
- The same document's partial-parameter example §3.9.3.6 p79 captures table
  references via `LdCtrlLoadImageProp` and separately describes allocation without
  filling. It does not define all modern `AppliesTo` tokens or every image-property
  capture/substitution rule. Preserve the existing [profile-scope correction](2026-10-16-systemb-offline-prerequisites.md#direct-original-pdf-inspection-d).

The existing declarative resolver retains unknown controls/attributes and always
keeps execution false (`procedure_resolution.rs::emit`). This investigation does
not remove those diagnostics by treating recognized spelling as a complete rule.

## Exact next integration requirements

1. Establish product object-number-to-ASAP transformation and a complete descriptor
   policy, with independently specified bytes for a sparse real-source witness.
2. Bind generated-table formats, resource locations and sizes to that exact
   source/profile; do not confuse source declarations with device validation.
3. Define canonical scope selection and image-property capture/substitution,
   including real numeric literals and the parameter/image lifecycle.
4. Reuse actual parameter/Dynamic evaluation, the existing sequencer and the
   stateful simulator for independently checked complete source-to-image-to-plan
   acceptance. No generic stub or inferred placeholder completion.
5. Establish whole-mutation recovery before any public hardware write route.

These are semantic/evidence blockers for this slice, not a reason to stop
independent work on other families or require manufacturer approval.

## Bounded remaining-family witness discovery

A calibrated, read-only scan in a network namespace visited all105inventoried
direct `.knxprod` archives, inspected503application prefixes (4,102,807bytes),
and retained at most two complete source witnesses per selected family group.
Eight calibration checks passed; all105original package digests and all seven
retained source digests were rechecked afterward. Full source bytes read total
4,529,535. Nothing was extracted into the application tree or executed.

| Selected group | Complete source witnesses | Observed direct source shape | Next disposition |
| --- | --- | --- | --- |
| System1 | 2 | `0012`, `DefaultProcedure`, one absolute segment each | Investigate source-bound Type1 image assembly using the existing legacy codec; default procedure execution/recovery remains separate. |
| System300 | 1 | `0300`, `ProductProcedure`, a USB-interface application with only a direct Disconnect control | This is not a representative complete programmable-application procedure; do not infer a family download from the mask or control count. |
| Other System7 | 0 | No selected `5705` header observed | Exact source witness remains missing from this bounded direct-member view; not proof of general absence. |
| Coupler/router | 2 | `091A`, merged procedure, absolute segment and property comparison/memory-write fragments | Investigate default/merge and IP/filter semantics separately from ordinary application images. |
| RF | 2 | `2705`, absolute segments and explicit product control names | Inspect RF-specific profile/image/control contracts before reusing TP algorithms. |

An initial research-reader attempt decoded the complete fixed-size prefix as a
UTF-8 string and refused one prefix ending inside a multibyte character, despite
a complete earlier application header. The original result is retained. Feeding
bytes incrementally, with an additional calibration for a split character,
removed that harness refusal; all seven selected witnesses stayed byte-identical.
No product code or input changed.

This is discovery, not a compatibility census: only the explicit mask allowlist
was selected, nested archives were not examined, and
only up to two source bodies per selected group were retained. A control name
or apparently short sequence establishes neither missing nor executable device
behaviour. Raw identifiers, XML, archive hashes and licensed pages remain private.
[Aggregate source evidence](../evidence/offline-source-integration-2026-10-16.json).
