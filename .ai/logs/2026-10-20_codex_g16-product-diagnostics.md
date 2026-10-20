# G16 diagnostic follow-up — 20 October 2026

## Scope

Bounded WP-03 G16 slice: clearer existing MissingMaster text and accurate
inspection usage including the observed VD1 form. No new variant, parser,
codec, schema, import admission or executable device route.

## Verified implementation

- Named built-CLI inspection-hint RED observed, then GREEN.
- Named library missing-product text RED observed, then GREEN.
- The shared legacy parser also serves import; a new named import-hint RED
  caught inspection formats bleeding into import. Pass a separate file hint
  from each caller; password parsing and admission stay unchanged.
- Focused wrapper, vendor-ZIP, legacy inspection/import and HTTP install/preview
  tests pass. HTTP checks retain the entire response shape/status, existing
  database file bytes, program identity and absence of backups.
- Four no-product shapes leave seeded package bytes/source counts unchanged;
  AppleDouble-only metadata keeps MissingMaster and the clearer text. Empty ZIP
  retains its earlier structural refusal. Initial attempt wrongly expected a
  MissingMaster error for empty ZIP; corrected the test, not production.

## Decisions and limits

Retain the existing typed refusal and compatibility categories; this is display
accuracy, not a second product-discovery mechanism or recursive installer.
Direct and wrapper callers receive the same existing error text. Do not infer
VD1 import authority from inspection; the real-file census is retained evidence,
not newly executed compatibility evidence. Legacy binary enum mapping remains
separate. Initial independent review rejected the blanket no-product wording
because a master-named directory can coexist with a valid inner package. Named
RED now reproduces that case. The corrected text describes the missing standalone
master file; both public APIs keep their existing refusal. Added directory-only
and directory-plus-product controls with seeded preservation assertions. The
review's minor delivery-index coverage overclaim is corrected: CLI pins wording,
library/HTTP verify existing-data preservation. Correction/whole-diff re-review
ACCEPTS with no unresolved findings; all16reviewed file identities verified before
handover-only updates. Revised focused40tests and three-crate Clippy pass. Final
regression bytes produce seven exact named failures against original production
source, including the directory/AppleDouble/CLI/HTTP variants. Maintained merged
gate and publication remain pending.

Raw results remain private under the active profile. No bus/device, desktop,
release/tag, deployment, vendor contact or real user-data operation.
