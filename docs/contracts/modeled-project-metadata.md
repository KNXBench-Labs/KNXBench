# Modeled project metadata — read-only inspection

> On main since 2026-10-12 (port of the parked branch; fresh gate in the
> [evidence](../evidence/native-evidence-port-2026-10-12.json)). Local acceptance figures below are historical.

Local implementation contract, 2026-10-11. No domain/store schema migration,
new source parser, project metadata writer or complete MODEL-07 claim.
Accepted locally on 1933 frozen inputs: owning Rust1311/0/100 ignored,
whole Web2723/0/180 files, built regressions12/0 and real-native export+metadata8/0;
strict quality/build/type/five repository/docs/fmt/whitespace gates pass.
Four final full-panel frames inspected, all labels/caveats readable. Self-review;
main integration/publication and later model evolution remain separate.

## Meaning and scope

The application projects only fields already in `knx_core::ProjectInfo`:
project ID, exact name, optional project number, group-address style, completion,
stored UTC last-modified/start timestamps, stored ETS schema and the modeled
unresolved legacy DPT-declaration count. `scope: normalizedProject` and
`view_version: 1` describe the independent model view, not a raw ETS document.
`model_schema_version` stays separate from `ets_schema_version`.

- `null` project number means absent in the model; `""` is explicitly empty.
  Neither is normalized into the other. Exact accepted strings remain unchanged.
- Completion is recorded project metadata, not new hardware verification,
  programming authorization, a signature certificate or newly evaluated intent.
- UTC dates are normalized stored model values, not original timestamp spelling
  or the application's `last_saved_at` marker. Reading does not stamp a new date.
- Zero legacy-declaration count is the recorded model scalar, not an assessment
  that every original source field/DPT or compatibility condition is understood.
- Additional Guid/version/copyright/contract/trace/task/usage/segment/address
  metadata remains outside this view. It is not guessed from retained fragments.
  Model and source retention/writer boundaries remain independently documented.

## Application and wire admission

The pure application view accepts only a project reference; it cannot acquire a
bus adapter, product database or source parser. It bounds cumulative project-ID/
name/number UTF-8 text to 64 KiB before cloning. Exceeding that limit refuses the
whole view, never a truncated success; original model/context stays unchanged.

`GET /api/project/metadata` uses the existing authenticated project router and
project lock. JSON carries `server_incarnation`, `project_incarnation`,
`snapshot_revision` plus `metadata`; no-store prevents caching private fields.
No-project and over-budget conditions are explicit refusals. Fields include
private names/identifiers/numbers but no installation access keys, network
credentials or arbitrary source-file paths.

Client admission enforces view version/scope, current identity shape, exact
completion/style vocabularies, safe scalar ranges, nullable fields and the same
cumulative UTF-8 text ceiling. Unknown/malformed views are not replaced by zero
or a fabricated empty project. This is not a generic future-model translator.

## UI behavior

The Dashboard's **Show modeled project information** action is explicit; no
metadata read occurs automatically. EN/DE read-only labels explain recorded
versus live/hardware/save-time meaning and privacy before copying/sharing.
Values are literal React text with preserved whitespace and wrapping, not HTML
or executable help. Null and explicitly empty values have distinct labels.
No input/textarea/write route, link execution or automatic external submission.

Replies must match the current server/load/revision. Late or cross-project
responses are suppressed; authoritative revision changes invalidate the view.
An unchanged read-only refresh alone does not invent a metadata edit. Pending
starts are guarded; stale refusal does not mutate saved or working state.

## Evidence and exclusions

Focused app/HTTP/client/component/Dashboard controls cover exact values, null/
empty distinctions, recorded-date/native roundtrips, authentication/refusals,
text limits and source-state equality. The companion import inventory adds only
the pure no-IO/no-mutation validator; API/mutation inventories are unchanged.
Actual production server + built browser EN/DE400/1440 tests create, Save As,
reopen and inspect fictional modeled data, independently compare native file
bytes before/after read, and assert literal script-like names with no executable
DOM nodes. Only UI preferences and discovery are intercepted; discovery never
reaches hardware. Current model/ETS schema numbers remain separate labels.

No private original replay, new hardware/vendor execution, metadata editing,
unmodeled-field migration, full ETS compatibility, native GTK/Orca, main
integration/publication or release/deployment is established. Source self-review.
