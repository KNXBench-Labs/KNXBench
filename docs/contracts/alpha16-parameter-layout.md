# Alpha 16 parameter-layout slice

## Scope and status

Status, 24 October 2026: the bounded WP-05/A16-07 caption-language outcome has
source-bound software acceptance in the
[current release matrix](../goals/alpha16-release-plan-2026-10-23.md#current-acceptance-disposition--24-october-2026).
That disposition supersedes prospective candidate-verification wording below;
the original 16-case focused evidence retains its scope. Local amd64 software/package
acceptance is complete, including both formerly blocked pinned-input passes;
the owner deferred extra local ARM64 preacceptance as unverified backlog, without
changing the normal amd64+arm64 publication workflow. Publication Go is
conditional on final readiness; parent closeout and public readback remain pending.
This is not complete manufacturer layout support; excluded subfeatures
and resource/integrity limits below are unchanged.

## Feature/evidence disposition

| Feature | Existing authority and result | Alpha 16 disposition |
| --- | --- | --- |
| Block/separator language overlay | Existing Program-scoped `Text` translations, exact dynamic-node defining tree/node and existing product-language selection | Implemented: requested-language template → module arguments → positional text resolver → existing row/grid DTO construction. |
| Explicitly empty translation | Stored empty `Text` is distinct from missing translation; existing presentation fallback applies | Preserve empty text, never revive the original template by truthiness. |
| Missing language/translation | Existing exact match, otherwise locale-prefix tie-break, otherwise source fallback | Reused without introducing another language resolver. |
| Module-local captions | Existing evaluated node scope and validated stored/default text-parameter authority | Reused; no label-based instance matching. |
| Declared row/column `TextParameterRefId` | Retained metadata is not a dynamic-node host in the existing positional resolver | Remains explicit unsupported fallback; do not borrow a neighbouring host's text value. |
| Table versus Grid, inline/nested/CollapseIfEmpty and in-cell buttons | Existing retained declarations do not alone establish every activation/layout rule | No blanket expansion. Existing delivered grids, access, separators and indentation remain unchanged. |

## Integrity and resource contract

ProductDB reads only `Text` translations for `ParameterBlock` and
`ParameterSeparator`, keyed by defining tree and native node. It shares the
existing language-selection function. Translation `Value` rows are excluded even
if present: display language cannot alter a value or add a project command.
The original dynamic-node text, project parameters, activation and write authority
are unchanged. Text-language metadata for these labels is not added in this
slice; positional fill provenance still describes the actual displayed template.

The same sticky `TextProjectionBudget` covers selected templates, owned query-result
language/tree metadata, module substitution, fill provenance and all existing DTO
copies. Borrowed SQLite text is admitted before a Rust-owned query-result copy.
Temporary lookup-key copies and query engine/allocator metadata are not an
aggregate RSS guarantee. Exhaustion refuses
the panel; no truncation or per-label budget reset is introduced.

## Focused evidence

The original HTTP path fails the new named assertion with the untranslated
`Functions: Ref default` instead of the requested `Funktionen: Ref default`.
After the local correction the real server integration binary runs the entire
16-case `http_parameter_captions` selection: 16 passed, zero failed/ignored.
This includes requested exact/prefix language, non-placeholder separator text,
source/project preservation, no-requested-language/source fallback, existing
stored/empty/default/scoped values, write/undo and budget refusals. It is not a
whole-server, browser, corpus or release gate.

The existing browser renderer consumes the same caption fields; no UI codec or
new mutation route is added. Actual integrated browser/release verification
belongs to the one final candidate campaign.
