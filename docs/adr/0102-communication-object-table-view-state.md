# ADR 0102: Object editors keep one parent while the table changes its mind

Date: 2026-10-09
Status: Accepted for local implementation; publication separate

## Context

The owner accepted a grouped/flat communication-object table with headings, search,
filters, sorting and an open-editor exception. The old device panel nested object
`details` under channel `details`. React keys preserve identity only among the
same parent's children: moving an object between channel wrappers or flat markup
would recreate its local draft/error/pending state. Existing DPT/description blur
writes and immediate flag/link writes must remain scoped to the original target.

## Decision

Keep pure list derivation in `comObjectView.ts`, table presentation in
`ComObjectTable.tsx`, and reuse existing application/API editing controls from
`Inspector.tsx`. All object components are keyed direct siblings under one `tbody`;
channel headers are also siblings. An object returns summary/editor rows and keeps
its lazily opened editor mounted across presentation changes. View state is local
and resets on device/project/editor lifetime boundaries; no domain storage owns it.

Use one narrowly scoped per-object mutation boundary to admit only one in-flight
edit, disable fields while pending and suppress late publication after unmount.
This does not cancel or roll back an admitted server write, replace backend rules,
or introduce a new wire contract. Refusal messages remain owned by the existing
fields. Dirty inputs survive committed-value refresh; real object removal is
announced rather than reusing its editor for another identity.

[The full behavior and acceptance contract](../COMMUNICATION_OBJECT_TABLE.md)
defines grouping, effective values, filters, stable sorting, counts, editing
exceptions, save gestures and narrow-screen/keyboard behavior. No core, storage,
product database, API schema, KNX protocol or dependency change.

## Alternatives

- Separate grouped/flat component trees: rejected because they reparent editors.
- Lift every editor field into a global cache: unnecessary state mirroring and
  broader lifetime/cleanup obligations for a single-device feature.
- A third-party table/grid framework: no demonstrated need and new lifecycle
  complexity without correcting domain behavior.

## Consequences

Collapsed/filtered object summary rows remain mounted to preserve identity. Editing
forms mount only when first opened. Large-device performance is measured and
reported as synthetic evidence, not unlimited-scale support. Browser accessibility
checks do not establish native/Orca validation. Local implementation and acceptance
remain separate from commit/integration/publication/release/deployment permissions.
