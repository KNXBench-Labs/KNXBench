# Declared product-folder captions: bounded display contract

## Status and scope

Admitted WP-06 literal, non-module increment on `e53e75417`, after parent
reconciliation of the independent raw-source recount and primary passages.
No implementation or runtime acceptance is claimed.

This adds optional **declared product text** to an already assigned saved folder.
It does not reconstruct current manufacturer placement, evaluate a caption,
resolve modules, change activation, or replace the saved tree. The existing
[saved-folder contract](communication-object-folders.md) remains authoritative.
Primary observations are in the [source study](../research/2026-11-06-product-folder-captions.md).

## Identity and source admission

1. Snapshot the existing saved placements under the established project/opaque
   lock; query product data only after that lock is released. No product lookup
   is needed when no assigned saved Folder path is available.
2. Use the owning device's exact Hardware2Program reference and its exact
   installed ApplicationProgram. No product name, family, caption, suffix search,
   latest-version substitution, or cross-program match is authority.
3. Require current measured, unambiguous source ownership for the selected
   Hardware2Program and ApplicationProgram, including divergent/unmeasured
   candidates. A product/hardware mismatch must not produce a caption. Corroborate
   the selected Product at canonical Hardware/Products/Product under its exact
   Hardware/Manufacturer in its own winning retained member; a measured local-name
   or parser-carried normalized row alone grants no authority. Product and H2P
   winning members may differ and must not substitute for each other.
4. Read the winning application's retained XML with a pre-materialization byte
   bound; re-hash it against its source key. Select exactly one canonical
   `KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram`
   by full ID and owner. No DTD, mixed/foreign ownership or first-descendant match.
5. The first source slice admits project schemas 20 and 23 and retained product
   XML namespace project/20, witnessed below. Other combinations keep their old
   folder display without product-caption authority. This is a bounded support
   policy, not a claim that other schemas have different semantics.
6. Only the selected application's own direct `Dynamic` contributes candidates.
   Exclude ModuleDef/Module expansion, descendants of Repeat, aliases and
   unsupported structural containers. Repeat elsewhere in the program does not
   invalidate an unrelated root declaration.
7. Construct the relative identity only by removing that exact full application
   ID plus its delimiter from a canonical declaration ID. Match the complete
   saved Channel/Folder ancestry, not just the last PB ID. In the product path,
   only declared Channels and ParameterBlocks with unqualified
   `ShowInComObjectTree` explicitly `true` or `1` contribute those nodes.
   Missing, false, qualified or unsupported flag spellings do not grant authority.
8. Duplicate/ambiguous declared identities and paths refuse their caption. Require
   one matching normalized, unscoped ParameterBlock row consistent with the raw
   element ID, Text and TextParameterRefId; never parse `dynamic_node.extra`.
9. Any root Dynamic Rename/ParameterBlockRename leaves this initial reader
   unavailable; those effects are not implemented. A candidate or structural
   ancestor with an unsupported access declaration, including `None`, is not
   admitted; absent/Read/ReadWrite remain eligible. This is conservative display
   admission, not an implementation of parameter access or current visibility.

## Text and provenance

- Admit only a nonblank unqualified ParameterBlock `Text`, at most 4,096 UTF-8
  bytes, without TextParameterRefId or template-looking brace pairs. No
  translation, substitution, rename or default caption inference is added.
- Keep the original source bytes and all existing unknown/unsupported reports.
  This bounded raw-source reader does not promote ShowInComObjectTree into the
  general Dynamic model or alter its diagnostics; no product-DB migration.
- Add a separate optional structured product-caption field to the path node.
  Preserve at least the exact AP ID, full PB ID, source SHA-256 and declaration
  source kind. Never expose local source paths. Missing legacy JSON remains valid.
- The installed winning source is **not** evidence of the original import-time
  product bytes. This field states a current installed-source declaration only.
- Existing saved schema-20 `text` is never overwritten. When that field is
  present, keep its current display choice, including blank-to-ID fallback.
  Otherwise an admitted product declaration may accompany the folder ID. Source
  provenance must be distinguishable in the UI; IDs and the full saved ID path
  always remain visible. Render as plain React text, not HTML.
- Search may add only the product text actually admitted for display on Folder
  nodes. No Channel text, refused placement, or hidden unsupported metadata leak.
- Keys, grouping membership/order, expansion, editor lifetime, activation, saved
  text, project/history and opaque bytes must remain unchanged when captions
  appear, disappear or change. Equal labels never merge distinct saved keys.

## Bounds and failure policy

Bound source bytes/depth/events and candidate identities before allocations grow.
Use bounded identity admission, not the general diagnostic candidate report;
candidate/conflict/holder counts and consumed metadata must be admitted before
materialization, while irrelevant package/path/reason payloads stay unread.
An incomplete capped prefix never grants source authority. Corroborate normalized
unscoped nodes in a bounded pass, not a full table scan per caption candidate.
Reuse the existing text-copy budget through repeated DTO copies, including source
identifiers; a small unique-label map does not bound a large rendered detail.
Missing database/source, ambiguous ownership, malformed input or resource refusal
withholds the optional product overlay, not the original saved placement. An
aggregate copy refusal leaves the complete optional overlay unapplied and emits
a bounded generic server diagnostic; never return a silently truncated caption.
Actual database failures follow the established server error handling.

## Executable acceptance

- Independently reproduce the pinned public witness's canonical identity counts;
  calibrate a real source reader against it before projecting new API/UI fields.
  Do not copy vendor XML into public regression fixtures.
- Synthetic product-reader tests: exact owner/version/full ancestry, duplicate
  paths/IDs, same text on different IDs, same short ID in other programs, bad
  hash/length, missing source, foreign namespace/qualified attributes, aliases,
  Repeat/module paths, flags, templates, access, renames and resource bounds.
- Overlay/projection tests: exact old key/path/status/saved Text for every refusal
  and legacy response; separate product provenance; all-or-nothing copy budget.
- Real HTTP handler: install an independently authored synthetic product, import
  a canonical project, check exact device JSON, native save/reopen and edit/undo,
  retained bytes/hash/history and no-DB/source-miss controls.
- Web unit and intercepted real-browser cases: both source kinds, saved blank
  precedence, inert HTML-looking literal text, search, unchanged opaque keys,
  no unexpected writes, DE/EN and 400/1440 geometry. Run every selecting config.
- Independent review with actual semantic discrimination controls, followed by
  the complete maintained change-selected branch and main-first gates, actual
  isolated server build/version and publication/readback. No live bus, hardware,
  native GUI, release or deployment acceptance follows.
