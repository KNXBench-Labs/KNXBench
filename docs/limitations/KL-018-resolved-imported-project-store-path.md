# Resolved imported-project store path

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-018)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 18. `open_project` does not clear the previous `.knxdb` `store_path`

**Resolved; source reconciliation 2026-10-01 (AR00).** The text below is
historical, not the current behavior. `apps/knx-server/src/domain.rs::open_project`
passes `None` to `replace_project_state`; its shared replacement transaction
sets `store_path` together with the project, clean baseline, opaque data and
manufacturer references under the project lock. The desktop delegates to that
domain. `http_project_routes::importing_replaces_a_dirty_project_with_a_clean_baseline`
asserts the imported snapshot has no store path. Do not redispatch the old fix.

The historical description moved verbatim to [resolved limitations](../history/KNOWN_LIMITATIONS_resolved.md#18-open_project-does-not-clear-the-previous-knxdb-store_path) (AR14D D5).
<!-- END retained catalogue record -->

<a id="earlier-resolved-description"></a>

<!-- BEGIN earlier resolved record -->
## 18. `open_project` does not clear the previous `.knxdb` `store_path`

**Resolved; source reconciliation 2026-10-01 (AR00).** The text below is
historical, not the current behavior. `apps/knx-server/src/domain.rs::open_project`
passes `None` to `replace_project_state`; its shared replacement transaction
sets `store_path` together with the project, clean baseline, opaque data and
manufacturer references under the project lock. The desktop delegates to that
domain. `http_project_routes::importing_replaces_a_dirty_project_with_a_clean_baseline`
asserts the imported snapshot has no store path. Do not redispatch the old fix.

**Limitation.** `AppState.store_path` (the `.knxdb` file a subsequent plain
`save_project` writes to) is only ever set by `save_project_as` and
`open_native_project`. The Tauri `open_project` command — ETS `.knxproj`
import — loads a fresh in-memory project but never touches `store_path`.
If a `.knxdb` was open and the user then imports a `.knxproj`, `store_path`
still points at that old `.knxdb` file.

**Cause.** `open_project` and `open_native_project` were added in
different cycles (`.knxproj` import predates the native `.knxdb` format)
and were never made to share a single "what file, if any, backs the
in-memory project" invariant.

**Impact.** None reachable through the current UI: `apps/knx-web/src/
App.tsx` resets its own `hasStorePath` flag to `false` on ETS import, so
"Save" always falls back to "Save As…" in that state. But the backend has
no equivalent guard — `save_project` just writes wherever `store_path`
points, with no check that the loaded project actually originated from
that path — so a future UI change that calls `save_project` without first
re-deriving `hasStorePath` from a real backend query could silently
overwrite the old `.knxdb` with the newly-imported project's data.

**Lifted when.** Either `open_project` clears `store_path` to `None`, or
`save_project` verifies the in-memory project actually originated from
`store_path` before writing.

**Related (2026-09-10, T10).** `export_project` used to be a second
consumer of a stale `store_path`: it re-opened `store_path` off disk to
read the opaque passthrough table and manufacturer manifest, so the same
stale-pointer scenario above could attach one project's opaque/manifest
data to a different project's export. Closed for that one code path by
reading `AppState.opaque`/`AppState.manufacturer_refs` (the live,
in-memory copies) instead of re-opening the file — see
[GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/994ac08c0328/docs/GAP_ANALYSIS_ETS.md)'s C4 row. The underlying gap
above (`store_path` itself can point at the wrong file) is unchanged.
<!-- END earlier resolved record -->
