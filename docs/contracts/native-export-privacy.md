# Reviewed native export and retained-source privacy

> On main since 2026-10-12 (port of the parked branch; fresh gate in the
> [evidence](../evidence/native-evidence-port-2026-10-12.json)). Local acceptance figures below are historical.

Local implementation contract, 2026-10-11. No domain/schema migration.
Accepted locally on 1921 frozen inputs: owning Rust1305/0/100 ignored,
whole Web2703/0/177 files, built regressions12/0 and actual-server native4/0;
strict quality/build/type/five repository/docs/fmt/whitespace gates pass.
Four final actual-server EN/DE400/1440 frames assessed for readable content-fit
privacy and controls. Self-review; integration/publication remains separate.

## User-facing boundary

The web File-menu **Export project** action opens the existing modal shell rather
than immediately downloading. DE/EN copy says a native file can include original
imported data, private settings, attachments/extension payloads, source metadata
and original reports. Renaming visible labels or compression is not anonymization.
The file is the current snapshot, not a complete version-history/shared-product
catalogue backup or an edited ETS `.knxproj` export. Export does not mark the open
project saved, change original source or automatically upload/publish anything.
Native-shell Save As and existing ordinary save behavior are not changed.

Cancel is initially focused; Cancel/Escape/backdrop do not export. Confirmation
allows one explicit browser-native streaming download with fixed `project.knxdb`.
No large in-memory Blob is created. A missing server/load/revision identity or a
changed authoritative snapshot disables/refuses confirmation instead of silently
rebinding or falling back to the legacy unbound path. A server-side conflict is a
failed browser download, not automatic retry; refresh and review again.

The confirmation fits its content rather than stretching like a search-results
list. Existing focus isolation/dismissal belongs to `Overlay`; only local dialog
styling differs. No remembered blanket privacy consent is stored.

## Wire boundary

`GET /api/project/download` accepts either no query (existing direct clients) or
all three fields `serverIncarnation`, `projectIncarnation`, `snapshotRevision`.
Partial, malformed or unknown query fields refuse. Complete bindings are compared
under the existing project lock before temporary serialization; stale server,
load or revision returns 409 without changing project/context/store/history.
Query-free legacy reads remain authenticated as before. The web confirmation
always supplies a complete reviewed binding; these values are identity, not
credentials or manufacturer/hardware evidence.

The existing temporary-file body adapter streams a newly serialized snapshot
and cleans interrupted/completed bodies. Responses are inert octet-stream
attachments, fixed Content-Disposition, no-store and nosniff. Source names/paths
never choose download destinations. None of this scrubs retained source or
verifies vendor signatures; original reports/opaque bytes remain intact.

## Evidence and exclusions

Focused HTTP tests read downloaded native model/opaque/manufacturer rows and
verify unchanged source state, legacy reads and stale/partial/unknown refusals.
Pure identity, component default-cancel/double-activation/stale-state and actual
App File-menu tests have named RED/GREEN evidence. Actual production server and
built frontend EN/DE400/1440 browser runs import a fictional archive, cancel,
confirm, receive a genuine native download and independently compare its retained
attachment in read-only SQLite; the project tree remains unchanged. Only UI
preferences and automatic bus discovery are intercepted, so no discovery reaches
hardware. Clicked-link observation measures the real URL without rewriting it;
Chromium native downloads need not emit page-scoped request events.

No private corpus replay is needed for this generic privacy gate. No manufacturer
execution, bus operation, native GTK/Orca, encrypted/anonymized export, ETS writer,
main integration, release or deployment acceptance is implied. Self-review only.
