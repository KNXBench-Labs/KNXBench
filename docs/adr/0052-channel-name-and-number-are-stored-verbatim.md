# ADR 0052: A channel's `@Name` and `@Number` are stored verbatim, and the v18 migration keeps every other unknown row

Date: 2026-09-30
Status: Accepted
Session: goal.md §12.3, KNOWN_LIMITATIONS §146

## Context

ISSUE-08 groups a device's communication objects by the channel element that
owns them (ADR-0050). The group's heading is the channel's translated `@Text`.
In the three corpus projects, 24 of 29 `Channel` elements have an empty
`@Text`, so those groups have no heading at all (KNOWN_LIMITATIONS §146).

`dynamic_node` stores a `Channel`'s `@Id`, `@RefId` and `@Text`. `@Name` is
recognized but lands only in `extra`, which design D2 documents as an audit
trail that is **not** re-parseable. `@Number` is not recognized: every
install reports it as an unknown attribute.

Measured over every application program file under `OriginalData/`
(**[V]**, 2026-09-30; aggregated counts only; a program shipped in several
files counts once per file):

- 1,268 `Channel` elements in 268 program occurrences. Every one carries
  both `@Name` and `@Number`.
- `@Number` is decimal digits on 1,263 elements. The other 5 are not
  numbers: 4 alphabetic values and 1 slash-separated triple.
- Within one tree (a program's own `Dynamic` or one `ModuleDef`'s), `@Number`
  is unique in all 441 trees. `@Name` repeats inside 8 of them. `@Id` is
  unique per program.
- 94 elements have an empty `@Text`, all in a program's own tree, none inside
  a `ModuleDef`. All 94 have a non-empty `@Name` and no `TextParameterRefId`.
  4 elements have an empty `@Name`, each with a non-empty `@Text`.
- 5,463 `TranslationElement`s refer to a `Channel`. All of them translate
  `Text`; none translates `Name`.
- 183 of the 204 `ModuleDef` channels have a `{{…}}` placeholder in `@Text`.
  None has one in `@Name` or `@Number`.

The schema text available here (Project Schema23 v01.00.00) does not define
`@Name` or `@Number` on a channel. So KNXBench knows what the files contain,
but not what the standard means by these attributes.

Since v13, `ingest_unknown` rows under `…/Dynamic/…` come from two writers:
the dynamic pass (`dynamic::parse`) and the scheme evidence reconciliation
(`parse::scheme_evidence`), which adds targeted and namespaced attributes and
subtracts aliases from the parser's rows. The v10→v11 precedent,
`reparse_dynamic_trees`, deletes every `…/Dynamic/…` row and writes back only
what the dynamic pass reports. Reused now, it would silently drop the
reconciliation's rows.

## Decision

1. **Two columns.** `dynamic_node` gains `name TEXT` and `number TEXT`,
   filled for `Channel` only, verbatim. An absent attribute is `NULL`. An
   empty one is `""`, as `text` already stores it. `number` is text, not an
   integer, because 5 corpus values are not numbers. Neither attribute stays
   in `extra`.
2. **`Channel/@Number` is modelled.** It is no longer reported as an unknown
   attribute. `@Icon`, `@TextParameterRefId` and `@HelpContext` stay
   unmodelled and reported. `ParameterBlock/@Name` keeps landing in `extra`,
   because nothing reads it.
3. **Migration v17→v18** (ADR-0020 E1: every value re-derived here is a
   function of `source_file.bytes`). Each `ApplicationProgram` blob whose
   bytes still hash to their key runs inside its own savepoint:
   - The `dynamic_node` and `module_def_argument` rows of the programs the
     blob owns are deleted and re-parsed, as in v11. The parser skips a
     program another blob owns, so a blob that lost every id conflict gains
     no trees.
   - The parser's unknown rows are **discarded**, not written back. The
     recorded rows stay, except the retired `Channel/@Number` ones at the
     two dynamic-pass xpaths (program tree and `ModuleDef` tree), which are
     deleted for every such blob: the dynamic pass reports attributes even
     for a program it does not store. Only the dynamic pass writes those
     rows: the static pass skips `Dynamic`, and the reconciliation only
     writes targeted names and names with a namespace.
   - A blob that fails to re-parse is rolled back, keeps its v17 rows and
     unknown rows, and gets a `ChannelNameBackfillError` record. So does a
     blob whose bytes no longer match their key, or one that a stored
     application-program owner/package member proves was a program but whose
     damaged bytes no longer classify as one. Nothing is guessed from its
     source filename.

   `package.unknown_count` then loses, per `ApplicationProgram` member, the
   distinct retired rows that member had: exactly what installing it added,
   since every package install re-ingests its program members. Each
   measured install report is rewritten without the retired rows and
   re-validated. A report with a member that was not carried forward, or
   whose numbers do not add up, is downgraded to `unavailable` with a
   recorded `InstallReportBackfillError`, as v14 and v16 do.
4. **Projection.** `DynamicNode` and `ChannelOwner` carry `name` and
   `number`. `ComObjectChannel` gets `name` and `number` (`None` when absent
   or empty), shown as written:
   - No label is composed from them. The UI decides how to show a channel
     without `@Text`.
   - Nothing is substituted: no corpus value holds a placeholder.
   - Nothing is translated: no translation exists.

   `ComObjectNode::channel` stays `#[ts(skip)]` until goal-ui.md U12.

## Alternatives considered

- **Read `extra` back.** Rejected: design D2 declares it not re-parseable.
- **Read the attributes on demand from the stored file** (as ADR-0044 does
  for download data). Rejected: every device-detail request would re-parse
  the whole application program, while the evaluator already walks
  `dynamic_node`.
- **`number INTEGER`.** Rejected: 5 corpus values would be lost or refused.
- **Store `@Number` and keep reporting it as unknown.** Rejected: "known"
  means "modelled" in `ElementSpec`, and v11 retired `@Value`'s unknown rows
  the same way.
- **Reuse `reparse_dynamic_trees`.** Rejected: it deletes the scheme
  evidence rows under `…/Dynamic/…`, which the dynamic pass cannot restore.
- **Compose a heading such as "Channel {Number}: {Name}".** Rejected: that is
  an invented label, and wording is the UI's job.

## Consequences

- `CURRENT_PRODUCTDB_VERSION` is 18. An older build refuses the database
  (`FutureVersion`), as for every bump.
- A migrated database must equal a fresh v18 install of the same files on
  `dynamic_node`, `module_def_argument`, `ingest_unknown`, the package
  reports and `package.unknown_count`. The migration tests assert this, and
  the corpus re-ingest check also compared streaming, normalized row
  fingerprints for those tables plus report counts/unknown details: v17→v18
  and fresh v18 matched across 106 retained files, with auto-increment
  evidence ids excluded. The separate opt-in package matrix compares 115
  package instances against the pre-v18 baseline (IMPORT_EXPORT).
- KNOWN_LIMITATIONS §146's data half is lifted. The limitation closes when
  the UI shows both fields (goal-ui.md U12).
