# AR10 — backend localization paths (Claude, 2026-10-05)

## Scope
AR10 of `alpha-release-goal.md`: `KL-14`, `KL-37`, `KL-64`, `KL-66`.

## Slices
1. **Trace** (`docs/research/backend-localization-paths.md`): the project schema
   has no language; `InitialValueLanguage` is reported, never promoted
   (`knx-etsproj/tests/project_language.rs`). `KL-14` ACCEPTED_BOUNDARY.
2. **2a – parameter panel**: `ProgramOverlay`/`OverlayHit` carry the stored
   language that answered; `program_default_language`; DTO
   `sourceLanguage`/`textLanguage`/`nameLanguage`/`enumOptions[].language`.
   The UI owner shipped "Untranslated (en-US)" badges (`f5494094`).
3. **2b – catalogue, device product, master**: `overlay_one`,
   `catalog_overlay`, `master_text_overlay` return `OverlayHit`; row structs
   gained `*_language` and `*_source_language` fields; wire fields on
   `/api/catalog/items`, `DeviceProductCatalog` (ts-rs, optional) and
   `ComObjectNode::dpt_text_language`. `KL-64` ACCEPTED_BOUNDARY.
4. **3 – report com-object text**: `knx_app::com_object_language` holds the
   T33/M6 rule (product layers only, only on a translation hit), split into
   `collect` (project) and `translate_com_object_texts` (product DB) so the
   server keeps its "never both locks" discipline. Server device detail and
   `documentation::report_options` both call it;
   `knx_report::ReportDeviceData::com_object_texts` carries the result.
   `KL-66` ACCEPTED_BOUNDARY after re-checking its disclosures; `KL-37`
   WAITING_OWNER (UI shows the 2b markers).

## Pitfalls met
- Generated ts-rs bindings emit trailing spaces; CI compares with
  `--ignore-space-at-eol`, so strip them to keep `git diff --check` clean.
  Regenerating also rewrites unrelated bindings' whitespace — restore those.
- `xtask check-headers` was red on main after earlier pushes of this session
  that ran only anchors/ledger; all five xtask checks now run before a push.
- `/api/device/{id}` com objects need a stated DPT for `dpt_text`; the HTTP
  test sets one explicitly.

## Not claimed
No "fully localized" claim: report labels (§48) and server error bodies (§66)
stay English with in-place disclosure; device creation keeps storing package
text by design.
