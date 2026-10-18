# ADR 0126: Positional text placeholders are filled from the text parameter, in the evaluation, never in a UI

Date: 2026-10-14
Status: Accepted
Session: Goal B14 (`docs/goals/unassigned-workbench-hardening.md`), issue #17

## Context

Issue #17 showed `{{0:Page 1}}` as a channel heading and `Page 1 - {{0:Icon 1}}`
as object names. The Manufacturer Tool documentation defines this placeholder:
ETS fills `{{0}}`/`{{0:def}}` with the value of the text parameter the host's
`TextParameterRefId` names, and shows `def` when that value is empty. The corpus
agrees on every point it can check (all references name `TypeText` parameters;
every `{{0…}}` host carries one or inherits its text from an object that does).
Product defaults of those parameters are often not empty, and ETS projects store a
device's value only when it differs. Evidence:
[research, 2026-10-14](../research/product-database.md#2026-10-14--positional-text-placeholders-0-goal-b14-issue-17).

The product database did not keep `TextParameterRefId`: `Channel`,
`ParameterBlock` and `ParameterSeparator` left it in `extra` (an audit trail that
is not re-parseable, design D2) and `ComObjectRef` reported it as an unknown
attribute only (ADR-0052 kept it that way on purpose). The named `{{Name}}`
substitution already ran in the evaluation, but it stopped at the first `}}`, so
`{{0:Button {{ChNo}}}}` stayed raw in module instances.

## Decision

1. **The reference is modelled (product database v23).** `dynamic_node` and
   `com_object_ref` gain `text_parameter_ref_id`, verbatim, `NULL` when absent.
   `TextParameterRefId` on these four hosts is no longer an unknown attribute.
   The migration re-derives the column from each stored program blob whose bytes
   still match their key, each in its own savepoint, and retires the four hosts'
   unknown-attribute rows from the stored install reports as ADR-0052 did for
   `Channel/@Number`. A blob that cannot be re-read keeps its rows and gets a
   `TextParameterRefBackfillError` record. `ParameterRef/@TextParameterRefId` stays
   unmodelled and reported: the documentation allows it, the corpus has none.
2. **One resolver, in the product layer.** `knx_productdb::dynamic::positional`
   is a pure function from a template and the text parameter's value to the
   display text plus the source of what was inserted. It fills index `0` only;
   any other index, an unclosed or stray brace and a host without a reference stay
   verbatim. The value is the device's stored one, otherwise the product default;
   an empty stored value counts as empty.
3. **Where it runs.** The device evaluation that the parameter panel and the
   device's communication objects already share supplies the values, and one
   product-database helper (`DeviceTexts`) applies the rule. Channel headings are
   filled where the server already builds them from that evaluation;
   product-layer object names are filled by one `knx-app` function after the
   translation overlay, for the device detail and the documentation report alike.
   Project-owned names are never touched. The parameter workspace added later
   exposes block and separator titles; issue #26 applies the same resolver to
   those dynamic hosts (including separator grid labels), retaining fill provenance.
   Unreadable captions get UI-only labels, never guessed parameter values.
4. **Named arguments inside a default** are substituted before the positional
   pass, one nesting level deep, which is all the corpus uses.

## Alternatives considered

- **Strip braces in the web app** (`{{0:Page 1}}` → `Page 1`). Looks right until
  a user has named the channel; then it shows the wrong text, and it would repeat
  the rule in every surface. Rejected by the issue's own triage.
- **Follow xknxproject** and fall back to `def` whenever the project stores no
  value. Ignores a non-empty product default, which the documentation's "value of
  the text parameter" includes; 283 of 654 referenced parameters in the corpus
  have one.
- **Read the reference from `extra` or the blob at query time.** `extra` is not
  re-parseable by design, and parsing a program blob per device view is slow and
  a second parser.

## Consequences

- Product database v23 is a migration with a backfill; older KNXBench builds
  refuse a v23 database as from the future, as for every version before.
- Group-address member lists, the Log and CSV exports read the project
  projection, which has no product database, and keep showing the stored
  template. The contract lists them.
- Tests: resolver cases (value, product default, empty, missing, other indices,
  malformed, nested arguments, Unicode, budgets), ingest and migration (columns,
  retired report rows, damaged blobs), and the device-detail and documentation
  surfaces with user names left alone.
