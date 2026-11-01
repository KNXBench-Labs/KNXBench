# Manufacturer distribution inspection (report format 4)

Contract for `knx products inspect-distribution` and
`knx_productdb::distribution::inspect_distribution`. Added for OpenKNX-style
manufacturer distributions: plain manufacturer XML, or a ZIP release holding
such XML next to scripts, help and baggage files. Format 4 is format 3 plus
`sourceCandidates` and `sourceCandidateTotals`; format 3 is format 2 plus
`dependencyEvidence` and the `dependenciesNotExecuted` limitation. None of them
has been in a release yet; only format 4 is emitted. A format-3 report lacks the
required `sourceCandidates` and `sourceCandidateTotals` fields (a format-2 report
also `dependencyEvidence`) and
fails to deserialize, rather than reading as a source without candidates or
declarations. Format 1 never left a feature branch and is not accepted or
emitted.

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
  range only (`preservedContent`), never echoed or executed. Declared scripts,
  calculations, validations, button handlers and Assign are listed in
  `dependencyEvidence`; none is run, resolved to a function or applied.
- **No repair.** `sourceCandidates` compares declared TypeNumber values and
  Union member intervals arithmetically. Nothing is corrected, initialized,
  chosen or written, and a candidate is not a schema, ETS or device verdict.

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

  Only the attributes listed in `target_kind` are followed: among others
  `Assign` `TargetParamRefRef`/`SourceParamRefRef` and the `ParameterRefRef`
  members of calculation sides and validations (exact `Static` paths only).
  On those member nodes every allowlisted attribute is followed, not only
  `RefId`, exactly as on other modelled nodes.
  Others, such as Argument, Rename and BinaryData references, are not.
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
- `dependencyEvidence[]`: declared dependency facts in member and element
  order (see below).
- `sourceCandidates[]`: TypeNumber value and Union interval candidates in
  member and element order (see below).
- `sourceCandidateTotals[]`: every checked occurrence counted by application,
  kind and outcome, with how many of them have their own row; in member order,
  then application element order, then kind and outcome in their declared
  enumeration order.

## Dependency evidence

Each row has a `kind`, its own `source`, `applicationSource`,
`manufacturerSource` and, inside a ModuleDef, `moduleDefinitionSource`. Only
elements in the document namespace on these exact paths become rows; anything
else, such as an `Assign` in a foreign namespace, stays in `unknowns[]`.
Static declarations that become rows (Script, calculations, validations and
their member wrappers) are still listed in `unknowns[]` too, because no typed
reader models them; a row there is an inventory entry, not a missed fact.

| Kind | Source | Kept |
| --- | --- | --- |
| `script` | `Static/Script` of an application or ModuleDef | `content`: byte range, size and SHA-256 of the raw text between the tags (CDATA delimiters and comments included). Not the per-token `preservedContent` hashes. A self-closing `<Script/>` has a zero-length range at the end of its tag. |
| `parameterCalculation` | `Static/ParameterCalculations/ParameterCalculation` | `Id`, `LRTransformationFunc`, `RLTransformationFunc`; `LParameters`/`RParameters` members as `calculationLeft`/`calculationRight` endpoints |
| `parameterValidation` | `Static/ParameterValidations/ParameterValidation` | `Id`, `ValidationFunc`; `Parameters/ParameterRefRef` members as `validationMember` endpoints |
| `buttonHandler` | every Dynamic `Button` | `Id`, `Text`, `EventHandler`, `EventHandlerOnline` |
| `assign` | every Dynamic `Assign` | `TargetParamRefRef`, `SourceParamRefRef`, `Value`, `assignmentForm` |

Fields:

- `attributes[]`: the fixed names of the kind, each with `presence`
  (`absent`, `presentEmpty`, `presentValue`; whitespace is a value) and the
  XML-decoded value.
- `memberLists[]`: each calculation side or validation wrapper occurrence;
  `presentEmpty` when it holds no member, `absent` (no source) when there is
  none.
- `endpoints[]`: role, owning element, attribute, presence, raw reference,
  `referenceState` and candidates, resolved exactly like `references[]` in the
  manufacturer, application and nearest ModuleDef scope. Duplicates stay
  `ambiguous` with every candidate. An absent attribute has no state, an empty
  one is `dangling`. Nothing is borrowed from another ZIP member.
- `assignmentForm`: `sourceReference`, `literalValue`,
  `conflictingDeclarations` (source and `Value` both declared; no precedence
  is chosen) or `missingSourceAndValue`. A literal `Value` is not a write.
  The form follows attribute presence: an empty `SourceParamRefRef` still
  counts as a source reference, and its endpoint is `dangling`.
- `functionResolution`: `notAttempted` for calculations, validations and
  handlers. Function names stay verbatim strings.
- `graphCycle` (Assign only; never a runtime loop). Each Assign is a graph
  node between every candidate of its source and of its target:
  - `declaredGraphCycle`: source and target resolve uniquely and the Assign
    lies on a cycle of such unique edges. A self-assignment counts.
  - `cycleNotDetermined`: the Assign lies on a cycle only when ambiguous
    candidates are included. Which duplicate is meant is not decided, so no
    cycle is claimed and none is ruled out.
  - `noDeclaredCycle`: source and target resolve uniquely and no Assign edge,
    ambiguous ones included, can close a loop through it.
  - `notApplicable`: other rows, Assigns with an absent or dangling source or
    target, and ambiguous Assigns on no possible cycle.
- `graphCycleAnchor`: the first Assign in element order of the strongly
  connected component that decided the state, so rows of one cycle share it.
  For `declaredGraphCycle` that component is taken from unique edges only, for
  `cycleNotDetermined` from all candidate edges. A declared cycle nested inside
  a larger possible loop therefore keeps its own anchor, distinct from the
  anchor of the undetermined rows around it. `null` for other states.
- Cycle bookkeeping holds every Assign source and target candidate twice (once
  per graph). It stays bounded because the references pass charges the same
  candidates against the reference ceiling (see Limits) before the graph is
  built.
- `disposition`: always `retainedNotExecuted`.
- `impactScope`: `declaredEndpointsOnly` for Assign;
  `applicationImpactUnknown` for scripts, calculations, validations and
  handlers, whose code may read or change any parameter. Neither says that
  other parameters are unaffected.
- `conditionContext`: `applicability` is `notEvaluated`; for Assign and
  Button it lists the enclosing `choose`/`when` occurrences, outermost first.
  `activeParameterRefs` holds no Assign or Button, so a nearby active
  reference says nothing about whether they are reachable.

Calculation sides are labels, not input/output directions, triggers or an
order, and L×R pairs are not expanded. `EventHandlerOnline` (for example
`ConnectionOriented`) is an online-mode declaration, not a second function or
a connection. Validations are declared, not run: there is no valid/invalid
result. The native write-authority recorder is not called.

## Source candidates

Each row has a `kind`, `severity` (always `sourceCandidate`), its own
`source`, `applicationSource`, `manufacturerSource`, inside a ModuleDef
`moduleDefinitionSource`, `initialization` (always `notPerformed`),
`activity` and `dependencyContext`. A row describes declarations under a
named KNXBench model. It is not a runtime error, and nothing is corrected.

**Rows and totals.** The report does not carry a row-detail field; the detail
is chosen by the caller. By default (`DistributionLimits::candidate_rows` is
`Findings`, the only mode of the CLI) a value check whose `comparison` is
`within` and an interval check whose `comparison` is `withinExtent` get no row,
unless the member declares an explicit zero width (`zeroWidthDeclared`), which
is always listed. Every other outcome is listed, including absent, empty,
invalid, type-unavailable, unbounded and resource-limit outcomes, and so is
every Union summary.
`sourceCandidateTotals` counts every checked occurrence either way: one entry
per application, `kind` and outcome (`valueComparison` for value kinds,
`intervalComparison` for members), with `occurrences` and `rows` (how many are
listed). Nothing is checked less; only the listing is shorter. Library callers
may set `DistributionLimits::candidate_rows` to `all` to list every
occurrence. The default exists because real OpenKNX releases declare up to
about 88,000 such occurrences, about 94 % of them inside their bounds, and
listing all of them pushed six of 37 releases over the serialized-report
ceiling, which refuses the whole inspection.

| Kind | Source | Check |
| --- | --- | --- |
| `parameterValue` | each `Parameter` whose type is a `TypeNumber` or cannot be determined | `valueCheck` of its `Value` |
| `parameterRefValue` | each `ParameterRef` that declares `Value` | `valueCheck` of that override, against the type of the uniquely resolved Parameter |
| `unionMemberInterval` | each `Parameter` directly in `Static/Parameters/Union`, of any type kind | `intervalCheck` |
| `unionIntervalSummary` | each such `Union` with at least one member | `unionSummary` |

- A ParameterRef without `Value` gets no row: it is not an override
  occurrence. The value it inherits stays in `parameterFallbacks`, whose
  `source` is the ref; a `parameterValue` row's `source` is the Parameter
  that owns the `Value`.
- A Parameter whose type resolves uniquely to another kind (`TypeFloat`,
  `TypeTime`, `TypeRestriction`, `TypeText`, ...) gets no value row. It is
  neither checked nor counted as TypeNumber.
- `parameter` (`ParameterRef/@RefId`) and `parameterType`
  (`Parameter/@ParameterType`) are resolved like `references[]`: in the
  nearest ModuleDef or the application, a ModuleDef type also in the
  enclosing application (`basis`), duplicates `ambiguous` with every
  candidate.

### Value check (`finite-decimal-comparison-v1`)

- `typeShape`: `typeNumber`; `parameterUnavailable` (the ref's Parameter is
  missing or ambiguous); `typeReferenceUnavailable` (type attribute absent,
  dangling or ambiguous); `noTypeElement` or `multipleTypeElements` (the
  ParameterType has no or several `Type*` children). Only `typeNumber` is
  compared; every other shape gives `typeUnavailable`.
- `value`, `minimum`, `maximum`: the row source's `Value` and the
  TypeNumber's `minInclusive`/`maxInclusive`. Each keeps the attribute name,
  presence and XML-decoded value (not the byte-exact spelling; the element's
  byte range is the raw source), a `class` and a `normalized` reading.
  Classes: `absent`, `empty`, `finiteDecimal`, `notFiniteDecimal`,
  `exceedsLexicalLimit`.
- Grammar: optional sign, digits with an optional fraction (`1.` and `.5`
  are accepted), optional exponent (`e` or `E`, optional sign, digits).
  Surrounding XML whitespace is ignored, as for `xs:decimal`. Anything else,
  including `NaN`, `INF` and inner spaces, is `notFiniteDecimal`. A value
  longer than 128 bytes or with an exponent beyond ±1,000,000 is
  `exceedsLexicalLimit`: refused work, not a source error.
- `normalized` is `[-]<digits>e<exponent>` without leading or trailing zeros
  (`0002` gives `2e0`, `5.000E+002` gives `5e2`), or `0`.
- The comparison is exact: no binary floating point and no narrowing to 64
  bits. Bounds are closed. `belowMinimum` and `aboveMaximum` are `outside`,
  `notOutside`, `unboundedOnThatSide` (bound absent) or `unavailable` (value
  or bound not a finite decimal, reversed range, or no TypeNumber).
- `comparison`, first match wins: `typeUnavailable`; from the value
  `valueAbsent`, `valueEmpty`, `notFiniteDecimal`, `lexicalResourceLimit`;
  `invalidDeclaredRange` (both bounds valid, minimum above maximum, so no
  inside or outside claim); `below`; `above`; `lexicalResourceLimit` or
  `boundInvalid` (a present bound is unusable and its side is not compared,
  which is not the same as unbounded); `unbounded` (no bound declared, so
  there is no range assurance); `within`.
- `typeAttributes`: the TypeNumber's `Type` and `SizeInBit`, as declared. No
  unit, factor, DPT, enumeration, script transform or width encodability is
  applied or checked.

### Interval check (`union-member-relative-bits-v1`)

- `startBit = 8 * Offset + BitOffset` and `endBit = startBit + width`,
  compared as the half-open `[startBit, endBit)` with `[0, extent)`. `width`
  is the `SizeInBit` of the member type's unique `Type*` element, `extent`
  is `Union/@SizeInBit`. The row is `outsideExtent` when `startBit < 0` or
  `endBit > extent`, otherwise `withinExtent`.
- **Strict-explicit.** An omitted `Offset` or `BitOffset` gives no interval
  (`offsetAbsent`, `bitOffsetAbsent`); no zero is substituted. An explicit
  `0` participates. A missing width is `widthUnknown`, never zero, and no DPT
  or native width is inferred. An explicit width `0` is a known empty
  interval with `zeroWidthDeclared`, not an encoding assurance.
- Integers are parsed exactly (optional sign and digits, surrounding XML
  whitespace ignored) into signed 64 bits. Class `absent`, `empty`,
  `integer`, `notInteger` or `exceedsLimit` (over 128 bytes or 64 bits). A
  negative width or extent is invalid (`widthInvalid`, `extentInvalid`); a
  negative offset is a valid input that lands outside. Overflow is
  `arithmeticLimit`. With an unknown or invalid extent the interval is still
  reported (`extentUnknown`, `extentInvalid`). Non-byte-aligned placements
  are inputs, not errors. The checks run in this order: offset, bit offset,
  width, arithmetic, extent.
- `unionPlacement` lists the Union's `Memory`/`Property` children as context.
  Their offsets are never added: the interval is member-relative, not an
  address. No alternative is chosen, and nothing is padded, relocated or
  written.

The formula is a KNXBench diagnostic model, not an official schema binding.
Bit numbering, the physical origin and runtime effects remain unverified.

### Union summary

- `memberDeclarations`, `knownIntervals`, `unavailableIntervals`,
  `unknownWidthMembers` and `outsideMembers` count member declarations.
- `distinctIntervals`, sorted by start and end, give each distinct known
  interval once with the number of members sharing it.
- `overlappingDistinctPairs` is the exact number of overlapping pairs of
  distinct non-empty intervals. At most 16 `overlapSamples` (index pairs into
  `distinctIntervals`) are listed, with `overlapSamplesTruncated`. No pair
  matrix is built.
- Union members are alternatives. Overlap is a relation between
  declarations, not a simultaneous write and not an error.

### Activity and dependency context

- `activity.state` is `notEvaluated` (reason `dynamicNotAttempted`),
  `observedInEvaluatedSubset` or `unavailable`. There is no known-inactive
  state.
- Observations come from the same application's `activeParameterRefs`: the
  declaring ParameterRef (the row's own ref, or each ref that resolves
  uniquely to the Parameter), its module-instance scope chain and
  `input: sourceFallbacks`. A ModuleDef definition stays one row; each
  instance is one observation.
- `unavailable` reasons: `notObservedInEvaluatedSubset` (the subset is
  incomplete, so this says nothing about activity), `noUniqueParameterRef`
  and `summaryRow`.
- `dependencyContext.declarations` lists dependency-evidence endpoints that
  name the row's ParameterRefs (kind, role, declaring element and ref). No
  value is calculated. `impactScope` is `applicationImpactUnknown` when the
  application declares any script, calculation, validation or handler, and
  `declaredEndpointsOnly` otherwise; neither is a non-impact guarantee.

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

- Dependency rows and their attributes, member lists, endpoints, candidates
  and conditions count as output rows. Each endpoint and each of its
  candidates also counts against the reference ceiling.
- Source-candidate rows and their reference candidates, type attributes,
  Union placements, distinct intervals, overlap samples, activity
  observations with their scope chains and dependency declarations count as
  output rows. Each `parameter` and `parameterType` reference and its
  candidates also counts against the reference ceiling.
- The XML working set and the serialized report are charged separately, and
  both are charged while the report is built. A ZIP keeps an exact running
  report size instead of re-serializing it after every member. The final
  report is measured again.
- Any `resourceLimit` refuses the whole inspection. A structural limit in one
  ZIP member therefore refuses the ZIP, not just that member.

## Verified behaviour

- Library tests cover combined and split sources, modules, BOM coordinates,
  malformed and refused inputs, inclusive limits, the source-map resolver,
  Dynamic unknowns, lexical type scope, fallback origins, exact ZIP report
  bytes, dependency evidence (presence, scope, ambiguity, cycles, record
  and reference budgets) and source candidates (the readiness vectors, exact
  decimal edges, lexical caps, strict intervals, Union summaries, activity,
  ZIP members and budgets). CLI tests compare the CLI output byte for byte with the library and
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
  not part of the repository. These figures are from format 2; the run was
  not repeated with dependency evidence or source candidates.

Related: [KL-187](../limitations/KL-187-distribution-inspection.md).
