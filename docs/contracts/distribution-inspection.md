# Manufacturer distribution inspection (report format 2)

Contract for `knx products inspect-distribution` and
`knx_productdb::distribution::inspect_distribution`. Added for OpenKNX-style
manufacturer distributions: plain manufacturer XML, or a ZIP release holding
such XML next to scripts, help and baggage files. Format 2 has not been in a
release yet. Format 1 never left a feature branch and is not accepted or emitted.

## What it is and is not

- **Read-only source inspection.** The command reads one regular file, writes
  JSON to stdout and errors to stderr, and touches nothing else: no product
  database, no output file, no network. Existing typed readers run against a
  derived view in a private in-memory SQLite database that is checked to be
  in memory and dropped afterwards.
- **Not installation.** A distribution without `knx_master.xml` and a
  signature is not a product package. Installing it stays refused
  (`MissingMaster`, `UnsafeMember`); inspection does not change that path.
- **Not complete.** `complete` is always `false`. `limitations` lists what was
  not performed: master data, XSD validation, authenticity, write authority,
  initialization, master dependencies, translations, baggage association and
  registration.
- **No code runs.** Scripts, CDATA and text are preserved as SHA-256 and byte
  range only (`preservedContent`), never echoed or executed.

## Accepted input

- XML whose root is `KNX` in namespace `http://knx.org/xml/project/<n>` with
  `n` in 10, 11, 12, 13, 14, 20, 21, 23. Other roots are `unrecognizedSource`,
  other versions `unsupportedSchema`.
- UTF-8 (with or without byte-order mark) or UTF-16 with a matching
  declaration. A DTD is refused (`dtdForbidden`). Conflicting BOM and
  declaration is `encodingConflict`.
- A ZIP is read through the same checked archive code as product packages
  (CRC, sizes, expansion, collisions). For distributions only, a Windows
  backslash name gets a `/` logical path; raw and decoded names stay in the
  report. Unsafe names, `%`-escapes and Unix special files are refused. Each
  member is reported with ordinal, names, sizes, SHA-256 and a state; an XML
  member that cannot be read keeps its typed error in `members[].error`, and
  the other members are still inspected.

## Report shape

All field names are camelCase. A source reference is a JSON pair
`[memberOrdinal | null, location]`, where `location` is one-based.

- `documents[]`: one per inspected XML document, with namespace, SHA-256, size
  and the **source map**: `names[]` (namespace, local name, raw prefixed name)
  and `locations[]` (parent index, name index, sibling ordinal, start and end
  byte, and `textToken` for text entries). Every original occurrence is stored
  once; rows refer to it.
- `DistributionInspection::resolve_coordinate` turns a reference into a full
  coordinate: element ordinal, half-open byte range in the original member and
  an indexed path such as `/KNX[1]/ManufacturerData[1]/Manufacturer[1]`. The
  path uses local names only and `text()[n]` uses the document-wide text
  ordinal. It is a locator, not an XPath expression. The resolver rejects
  dangling, cyclic, out-of-range and over-deep maps, so it can be used on a
  report read back from disk.
- `sections[]`: `catalog`, `applicationPrograms`, `hardware`, `baggages`, each
  with a pass state (`parsed`, `preserved`, `partiallyEvaluated`,
  `notAttempted`) for that pass only.
- Typed facts: `catalogItems`, `hardware` (products, Hardware2Program links)
  and `applications`.
- `applications[].parameterFallbacks[]`: the raw source value a ParameterRef
  falls back to, with its origin:
  - `parameterRef`: the ref declares `Value`;
  - `parameter`: the uniquely resolved Parameter declares it;
  - `absent`: it resolves uniquely and declares none;
  - `danglingParameter`: there is no target;
  - `ambiguousParameter`: several Parameters share the target identity.

  These are source observations, **not** initialization: Assign, calculations,
  validation and handlers are not applied.
- `applications[].activeParameterRefIds` and `activeParameterRefs`: the subset
  the existing Dynamic evaluator activates from source fallbacks, with the
  module-instance scope chain. `dynamicState` is `partiallyEvaluated` at best.
  Dynamic is not attempted for an application with duplicate declarations, no
  identity, or a failed Static pass.
- `identities[]`: every declared identity occurrence, never deduplicated.
- `references[]`: each reference attribute with state (`resolved`, `dangling`,
  `ambiguous`, `external`), candidates and basis:
  - `originalScope`: the nearest ModuleDef or application;
  - `enclosingApplicationScope`: a ModuleDef `ParameterType` declared only by
    the application;
  - `declaredManufacturerAcrossMembers`: catalogue and hardware joins between
    ZIP members of the same declared manufacturer and namespace. This is not
    identity proof.
  - `externalUnperformed`: mask, medium and DPT references, which are not
    resolved.

  Only the attributes listed in `target_kind` are followed; others, such as
  Argument, Assign, Rename and BinaryData references, are not.
- `unknowns[]`: every element and attribute that this build does not model, by
  source reference. Dynamic verdicts reuse the Dynamic parser's grammar
  (`spec_for`, `attribute_is_known`) and the evaluator's container kinds. A
  Dynamic subtree that is not attempted or fails is listed as unknown, never
  counted as covered. Comments and processing instructions are covered only by
  the document hash and are not listed.
- `diagnostics[]`, e.g. `duplicateIdentity`, `emptyDeclaredIdentity`,
  `missingDeclaredIdentity`, `dynamicEvaluatedSubset`, `dynamicReaderFailed`,
  `ambiguousOrUnparsedApplication`, `masterMemberPresentNotInspected`.
- `readerPasses[]`: each typed-reader pass with original and derived SHA-256,
  derived size and the transformation name `knx-expanded-role-view-v1`.
- `declaredMetadata[]`: `CreatedBy`, `ToolVersion`, version and identity
  attributes as asserted by the source, never verified.

## Limits

`DistributionLimits::default()` is the ceiling. Callers may only lower each
value (`bounded()` takes the minimum), and every limit is inclusive.

| Limit | Default | Why |
| --- | --- | --- |
| input, member | 256 MiB, 128 MiB | whole-file and per-member reads |
| expanded archive | 512 MiB | ZIP expansion |
| elements per document | 500,000 | largest measured real member: 488,283 |
| depth, attributes per element | 128, 128 | structural |
| text bytes | 64 MiB | retained text |
| identity/reference value | 4096 bytes | lookup keys, not payload |
| references, output rows | 4×, 8× the element ceiling | defence in depth |
| derived view, scopes | 128 MiB, 4096 | reader input |
| XML working set | 256 MiB | retained nodes, tokens, attributes |
| serialized report | 256 MiB | exact compact JSON bytes |
| Dynamic copy work | 256 MiB | scalar copies during evaluation |

- The XML working set and the serialized report are charged separately, and
  both are charged while the report is built. A ZIP keeps an exact running
  report size instead of re-serializing it after every member. The final
  report is measured again.
- Any `resourceLimit` refuses the whole inspection. A structural limit in one
  ZIP member therefore refuses the ZIP, not just that member.

## Verified behaviour

- Library tests cover combined and split sources, modules, BOM coordinates,
  malformed and refused inputs, inclusive limits, the source-map resolver,
  Dynamic unknowns, lexical type scope, fallback origins and exact ZIP report
  bytes. CLI tests compare the CLI output byte for byte with the library and
  check that the profile is unchanged. Native package regressions show that
  installation still refuses distributions.
- Real input: 37 public OpenKNX release ZIPs, each inspected with a release
  build in an isolated, loopback-only namespace. All 37 return a partial
  report: 37 catalogue, application and hardware sections, and 35 baggage
  sections (the other two releases declare no baggage). Peak RSS was about
  1.1 GiB, the slowest took 9 s, and the largest report was 174 MiB, about
  68 % of the report cap. Together they list 3,474,618 unknown occurrences.
  About 2.94 million of these are inside Dynamic, mostly `HelpContext`,
  `Row`/`Column` `Name`, `Inline`, `Icon` and `Button` handlers. Installing
  the same 37 archives in a fresh in-memory product database is still refused:
  28 with `MissingMaster`, 9 with `UnsafeMember`. The archives themselves are
  not part of the repository.

Related: [KL-187](../limitations/KL-187-distribution-inspection.md).
