# Parameter-workspace layout observations

Date: 2026-10-17. Evidence class: bounded, local, read-only **declaration census**;
not application evaluation, screenshot fidelity or manufacturer compatibility.
Source selection: `.knxprod` files recursively under the owner's product-database
collection, with no executable component or private research material used.
Original package bytes were checked before/after and remained unchanged.

## Measured scope

- 106 discovered package files; 105 unique package contents and one duplicate.
- Application-related declarations admitted in 97 packages; seven distinct
  manufacturer reference identities observed. These are not seven fully tested
  manufacturers or 97 accepted application profiles.
- XML-member limit 16 MiB, package limit 256 MiB, 128-package discovery cap,
  20,000-member cap, depth 96, four-million-event XML work cap, total decompression
  budget 1.5 GiB. DTD/entity declarations refused. No archive extraction.
- 27 XML members exceeded the per-member limit and were not parsed. Nested ZIP
  packages, legacy databases and other source subroots are outside this census.
  All counts below apply only to admitted members, not the entire collection.

## Layout-relevant declaration presence

Counts are numbers of distinct unique-content packages in which the named
construct was observed, not counts of rendered or active controls.

| Declaration | Packages |
|---|---:|
| ParameterBlock | 97 |
| Choice/restriction type | 96 |
| Numeric type | 93 |
| Text type | 72 |
| ParameterSeparator | 72 |
| Channel | 70 |
| ChannelIndependentBlock | 62 |
| Module | 26 |
| ParameterBlock caption parameter reference | 20 |
| No-value type | 19 |
| Picture type | 18 |
| Colour type | 14 |
| Time type | 9 |
| ParameterBlock Layout plus Rows/Columns | 8 |
| ParameterRefRef Cell | 8 |
| Repeat | 4 |

## Consequences for this implementation

1. **Use source identity, not naming conventions.** Equal captions do not establish
   equal groups or channels. Keep device/module scopes separate and incomplete
   group metadata visible.
2. **Do not make a universal repeated-channel pivot.** A source group can mix
   choices, numerical/text fields, conditional branches and no-value headings.
   Compact label/value rows accommodate these without guessing column meanings.
3. **Keep editor identity across visibility changes.** Conditional structures and
   revised evaluations can change grouping/visible fields. Navigation and search
   must not become a second value store or silently drop drafts/conflicts.
4. **Treat actual table coordinates as separate source metadata.** Explicit
   Rows/Columns/Cell declarations exist; do not claim they are absent, and do not
   pretend the existing nearest-group DTO already preserves them. A future exact
   grid adapter must retain typed coordinates, span/branch semantics and refuse or
   report ambiguous ownership. This package does not reproduce those layouts.
5. **Keep specialized editors and unsupported diagnostics.** Time/colour/picture
   data is not permission for an unverified picker, duration conversion or
   manufacturer-script execution. Existing checked-editor contracts remain in
   force.

The public fixtures are independently authored and reproduce only structural
classes, not manufacturer texts, identifiers, source fragments or raw values.
Detailed input identities, digests and the bounded scanner are retained privately;
no source packages or private project information are redistributed.

Implementation contract: [source-group parameter workspace](../contracts/parameter-workspace.md).
