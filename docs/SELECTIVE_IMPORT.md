# Selective project import

Source implementation against normalized model v12 (completion vocabulary only;
no added entities or references). The original local package
was based on `b042048b`; [integration verification](status/2026-10-10-import-expansion-integration.md)
records the separate combined-source acceptance. This is not a release claim. Architecture: [ADR-0106](adr/0106-selective-project-import.md).
Formal source-ID status belongs to [the ledger](status/LEDGER.md).

## User workflow

With a project open, use **File → Import selected lines/devices…**. Choose an
ETS `.knxproj` source, select its installation and the destination installation,
then tick individual devices or complete lines. A selected line includes its
own devices, not unrelated devices. Search limits the displayed device list to
250 matches; search does not alter the selection.

Preview lists included devices, communication objects, parameters, modules,
group addresses, lines and building parts, together with explicit ID mappings,
reuse notes and the complete source importer report. Approval requires separate
consent to retaining the full source. A changed selection, source or destination
revision invalidates the preview. Existing devices/addresses are not silently
reassigned or overwritten; conflicts refuse the entire operation.

Source parsing uses the existing bounded import pipeline. The source's entire
report remains distinct from selected-entity counts: a mapped device is not a
claim of complete ETS compatibility or executable commissioning support.

## Retention and ownership

The original archive, opaque/member evidence, import report and manufacturer
references remain under a content-addressed `imports/<source-hash>/` context.
Raw ETS identifiers and payloads remain unchanged. Domain IDs are allocated from
the destination's checked high-water marks; all included references are remapped.
Target project identity, installation settings, language and address style stay
owned by the destination. Existing compatible structural elements keep their
names; reuse is reported. Translations that cannot retain their meaning under
the target fallback language are refused, not silently replaced.

Retained attribute rows are classified as `SelectiveImportRetainedAttribute`:
they are evidence from their source, **not global destination settings**. Thus
an imported installation's BCU key cannot become the destination's default key.
Serial lookup uses the complete device SourceRef; duplicate raw ETS identifiers
require explicit context rather than selecting the first source's value.
Manufacturer data are retained, not installed into or substituted for the
separate product database. This import does not certify device/application
version compatibility or widen download eligibility.

**The retained archive can contain unselected and confidential source data.**
Undo removes imported model entities but does not erase append-only source
context. Keep native project copies private, including after undo. This is not
an anonymized or reduced export.

## Atomicity and history

One import is one native snapshot history step, with undo/redo. Admission and
persistent working-state save occur before publishing a changed in-memory
project. The store compares both history generation and the reviewed working
snapshot hash inside its write lock, including generation-zero files changed
through plain save. Refused/stale previews leave the target unchanged. CLI
previews open the destination read-only; a confirmed apply opens the writer only
after input admission. Lost HTTP acknowledgments cause observation, not automatic
replay. Native save/reopen preserves the admitted model/context/history within
existing native snapshot bounds.

## CLI and HTTP

```text
knx import-selection inspect source.knxproj [--password-stdin]
knx import-selection source.knxproj --project target.knxdb \
  --source-installation 0 --target-installation 0 --device 1
knx import-selection source.knxproj --project target.knxdb \
  --source-installation 0 --target-installation 0 --line 1 --confirm <preview-token>
```

Without `--confirm`, the merge command only returns a JSON preview. Obtain IDs
from `inspect`, not from a raw ETS identifier. Passwords travel through stdin,
never an argv option. Confirmation binds the actual source bytes, selection,
reviewed native state and target identity. The GUI and CLI call the same
application service.

The three POST routes are `/api/project/import-selection/source`,
`/api/project/import-selection/preview` and
`/api/project/import-selection/apply`. Apply requires the preview token and
recomputes the plan. Existing full-project replacement import is unchanged.

## Deliberate bounds

- 64 MiB source input, plus the existing container/parser resource ceilings.
- Native model v12 only. The v11→v12 change extends completion vocabulary,
  not entity/reference fields; the dependency closure is unchanged. A further
  model bump must audit new entity/reference fields before widening this explicit
  guard; do not merge Functions/Sites changes without that audit.
- Complete-source mapping/validation errors refuse selective import, even if
  the error is outside the selected subset.
- Native snapshot admission is mandatory; unsupported preservation shapes or
  exhausted IDs refuse without a partial merge.
- Only currently mapped source grammar can be selected. Retained opaque data
  are not promoted to typed domain semantics by selection.
- No ETS internal-store/restore-point parser, ETS project writer, vendor-code
  execution, product-database installation or protocol operation is introduced.

## Regression evidence

Named tests: `crates/knx-app/tests/selective_import.rs` (exact dependency/context,
conflict/stale/selection/ordered-vector/source-metadata tests),
`crates/knx-store/tests/history_compare_and_save.rs`,
`apps/knx-server/tests/http_selective_import.rs`,
`apps/knx-cli/tests/selective_import_cli.rs` and
`apps/knx-web/src/SelectiveImportButton.test.tsx`. Fresh selected owning suites and four production-browser language/viewport
cases are recorded in the [local verification](status/2026-10-10-import-expansion-verification.md)
and [receipt](evidence/import-expansion-local-2026-10-10.json). Repository closure
is recorded there separately; this source is not published or released. Self-review only; no independent or real-bus acceptance.

The separate [authorized export harness](AUTHORIZED_RESTORE_EXPORTS.md) does not
replace unavailable real restore-point exports. Converter-exception observations
are documented in [cvexc research](research/cvexc.md); they do not drive runtime
policies.
