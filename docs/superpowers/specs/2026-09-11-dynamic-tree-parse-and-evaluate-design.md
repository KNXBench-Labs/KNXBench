# `Dynamic` tree: parse, store, evaluate (T18, first slice)

## Status and scope

T18 ("parameter interpretation and editor",
[GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md) Tier 5) is the single
largest remaining gap by effort. It stopped being a research problem on
2026-09-11, when the R3 spike documented the `when/@test` value grammar
([RESEARCH.md §4.3](../../RESEARCH.md)). This design covers the **first
slice only**: reading the `ApplicationProgram/Dynamic` tree out of
manufacturer XML, storing it losslessly in the product database, and
evaluating it headlessly into "which parameter refs and which
communication-object refs are active for this set of parameter values".

Explicitly **not** in this slice:

* no parameter editor, no UI of any kind, no HTTP endpoint;
* no writing of parameter values anywhere — the evaluator is a pure
  function of (stored tree, supplied values);
* no `Module`/`ModuleDef` expansion inside the `Dynamic` tree (recorded,
  reported, deliberately not followed — see D10);
* no memory layout, no download, no commissioning;
* no claim of ETS behavioural parity. Nothing here was validated against
  ETS itself, and the no-match rule this design picks (D8) is an
  inference the KNX Standard does not state.

Import is unaffected: it already avoids the `Dynamic` tree entirely by
reading `GroupObjectTree`, ETS's own precomputed answer
([ADR-0014](../../adr/0014-group-object-tree-authoritative-source.md)).
This slice adds a capability; it changes no existing import path.

## Evidence

Everything below rests on [RESEARCH.md §4.3](../../RESEARCH.md) and its
full spike report (`.ai/logs/2026-09-11_claude_r3_dynamic_grammar.md`),
whose numbers come from 34 application programs in 7 archives — 12149
`choose`, 22630 `when`. The three confidence levels used there are used
here too: **[D]** the Standard states it, **[V]** the corpus shows it,
**[A]** inferred beyond both.

Load-bearing facts for this design:

* **[D]** `When_t/@test` has type `Condition_t`
  (`Project Schema23 v01.00.00.md` §1.1.3.18): a single number, a
  space-separated list of numbers, or `op number` with `op` one of
  `= != > < >= <=`. The controlling parameter must be `TypeNumber` or
  `TypeRestriction`; for `TypeRestriction` the comparison uses the
  matching `Enumeration/@Value`.
* **[V]** Four `@test` shapes cover 22630/22630 `when` elements, with
  `@default="true"` as a fifth, mutually exclusive branch selector.
* **[V]** `choose/@ParamRefId` resolves 12149/12149 times, 0 dangling.
* **[V]** 604 `choose` elements are controlled by a `TypeNone`
  parameter, which the Standard's own constraint forbids; all 604 have
  exactly one `when default="true"` child.
* **[V]** 5570 of 8732 default-less `choose` elements have a legal
  enumeration value no `when` covers. "No branch matches" is the common
  case, not a corner case.
* **[V]** `ChannelIndependentBlock` was found mid-spike and exists in no
  schema document available here. The observed element vocabulary is a
  superset-so-far, not a closed grammar.

## Decisions

### D1. The product database owns the tree

`knx-productdb` parses, stores and evaluates. No new crate, and no *new*
crate dependency: nothing here reaches for `knx-store`, `knx-etsproj`,
`knx-app`, the server or the UI. (`knx-productdb` already depended on
`knx-core` before this slice — an earlier draft of this decision said
otherwise, which was simply wrong about the existing `Cargo.toml`. The
direction that matters, and that `xtask`'s `check-layering` enforces, is
that `knx-core` must not reach back down here.) The `Dynamic` tree is
manufacturer data; it belongs beside the `Static` tree rows that it
references. `cargo run -p xtask -- check-layering` must stay clean
without a new exception.

### D2. One row per element, in document order

A new table, `dynamic_node`, in a new product-database schema version:

```sql
CREATE TABLE dynamic_node (
    program_id    TEXT NOT NULL,
    module_def_id TEXT NOT NULL,   -- '' for the ApplicationProgram's own tree
    node_id       INTEGER NOT NULL,-- document order within (program, module_def)
    parent_id     INTEGER,         -- NULL for the root <Dynamic>
    position      INTEGER NOT NULL,-- 0-based index among its parent's children
    kind          TEXT NOT NULL,   -- the XML local name, verbatim
    element_id    TEXT,            -- @Id
    ref_id        TEXT,            -- @RefId / @ParamRefId
    test          TEXT,            -- @test, verbatim, unparsed
    is_default    INTEGER,         -- 1 when @default="true"
    text          TEXT,            -- @Text
    extra         TEXT,            -- every other attribute, "name=value" pairs,
                                   -- sorted by name, newline-separated
    PRIMARY KEY (program_id, module_def_id, node_id)
) STRICT;
CREATE UNIQUE INDEX dynamic_node_sibling
    ON dynamic_node (program_id, module_def_id, parent_id, position);
```

`module_def_id` participates in the key because a `ModuleDef` carries its
own `Dynamic` tree with its own local ids — the same one-level-deeper
repetition of the `ApplicationProgram` shape that
[ADR-0013](../../adr/0013-module-instance-representation.md) already
records. It is `NOT NULL` with an empty-string sentinel rather than
nullable on purpose: SQLite permits NULLs in a non-`INTEGER` `PRIMARY
KEY` and treats each as distinct, which would silently disable the
uniqueness constraint for exactly the common case — the program's own
tree.

### D3. `kind` is the literal element name, never a mapped enum

No `kind = 'Unknown'` bucket, no normalisation. An element this build has
never heard of is stored under its own name and is recognisable later
without re-reading the blob. The evaluator, not the schema, decides what
it can act on. This is the direct consequence of `ChannelIndependentBlock`
having been unknown to this repository until the spike found it.

### D4. Unmodelled attributes are stored *and* reported

Attributes outside the modelled set land in `extra` verbatim, and are
additionally counted into the existing `ingest_unknown` table through the
existing `UnknownCollector`, exactly as the `Static`-tree parsers already
do. Modelled attribute names per element kind:

| Kind | Modelled attributes |
| --- | --- |
| `choose` | `ParamRefId` |
| `when` | `test`, `default`, `Id` |
| `Channel`, `ParameterBlock` | `Id`, `RefId`, `Text`, `Name` |
| `ParameterRefRef`, `ComObjectRefRef` | `RefId` |
| `ParameterSeparator` | `Id`, `Text`, `UIHint` |
| `Module` | `Id`, `RefId` |
| `Assign` | `TargetParamRefRef`, `Value`, `SourceParamRefRef` |
| everything else | none; all attributes go to `extra` |

Nothing is dropped either way: the source blob still holds the file
byte-for-byte ([ADR-0011](../../adr/0011-product-database-storage.md)).

### D5. Schema v3 backfills from stored blobs

The v2 → v3 migration creates `dynamic_node` and then **re-reads every
`source_file` blob with the new `Dynamic`-only parser**, so an existing
product database gains its trees without a re-install. This is the first
migration that runs Rust rather than plain SQL, and it is the payoff
ADR-0011 was designed for: the blob is kept precisely so a later parser
can extract what an earlier one skipped.

Rejected: "bump the version, populate on next install". Installation is
content-hash idempotent (`source_parse_evidence`), so an already-installed
file would never be re-parsed, and every existing database would stay
permanently empty for no visible reason.

### D6. A pure evaluator, with diagnostics as a first-class result

New module `knx-productdb/src/dynamic/` with a parser half and an
evaluator half. The evaluator takes the loaded tree and a value map and
returns activations plus diagnostics:

```rust
pub fn evaluate(tree: &DynamicTree, values: &ValueMap) -> Activation;

pub struct Activation {
    pub parameter_refs: Vec<String>,   // document order, first occurrence wins
    pub com_object_refs: Vec<String>,  // document order, first occurrence wins
    pub diagnostics: Vec<Diagnostic>,
}
```

`ValueMap` is keyed by `ParameterRef` id — the same id
`choose/@ParamRefId` and `ParameterRefRef/@RefId` use, and the same one a
project's `ParameterInstance` carries (`DATA_MODEL.md` §10). A ref with no
supplied value falls back to `parameter_ref.value`, then to
`parameter.value`; if neither exists, that is a diagnostic, not a panic
and not a guess.

Diagnostics are returned, never logged and never swallowed — the same
posture the import report takes. At minimum:
`NoBranchMatched`, `UnparsableTest`, `UnresolvedParamRef`,
`NonNumericValue`, `UnexpectedTypeNoneShape`, `UnrecognizedNode`,
`ModuleNotExpanded`, `MissingValue`.

### D7. `Condition_t` is parsed strictly, per the Standard

```rust
enum Test { Single(i64), List(Vec<i64>), Compare(Op, i64) }
enum Op { Eq, Ne, Gt, Lt, Ge, Le }
```

All six operators are implemented even though the corpus only ever shows
`>` (13 occurrences): they are in the normative grammar, and implementing
only what one corpus happens to use is how the next corpus breaks. A
`@test` that does not parse produces `UnparsableTest` and matches nothing
— it is never treated as true, and never as false-by-silence.

### D8. No matching branch means nothing under that `choose` is active — and it is reported

This is the design decision the spike said had to be made, and it is an
**[A]** inference: no source consulted states it. The conservative
reading is chosen (activate nothing) because the alternative — guessing a
branch — would silently fabricate a device configuration, which this
project's data-integrity rule forbids outright. Every occurrence emits
`NoBranchMatched { choose_node, param_ref, observed_value }` so a caller
can surface it rather than discover an empty dialog.

The same rule covers a `choose` whose controlling value is missing or
non-numeric: nothing activates, a diagnostic is emitted.

### D9. `TypeNone`-controlled `choose` gets its own code path

When the controlling parameter's type is `TypeNone`, the evaluator does
not attempt a comparison at all: it takes the single
`when default="true"` child, which is what all 604 corpus occurrences
look like. Any other shape under a `TypeNone` control emits
`UnexpectedTypeNoneShape` and activates nothing. Forcing this idiom
through the `TypeNumber`/`TypeRestriction` comparison path would mean
implementing behaviour the Standard does not define for a type it says
cannot appear here.

### D10. Unrecognized nodes are opaque and non-activating; `Module` is not expanded

Recognized as transparent containers: `Dynamic`, `ChannelIndependentBlock`,
`Channel`, `ParameterBlock`, `when`. Recognized as activating leaves:
`ParameterRefRef`, `ComObjectRefRef`. Recognized and deliberately inert:
`ParameterSeparator`, `Assign` (presentation and assignment constructs
whose semantics this slice does not model).

Anything else — including a future construct nobody here has seen —
activates nothing, its subtree is not descended, and it produces
`UnrecognizedNode { kind, node_path }`. Descending would mean assuming
the unknown element is a transparent wrapper, which is precisely the
guess that `ChannelIndependentBlock`'s late discovery argues against.

`Module` is recognized but not expanded: its `@RefId` and `NumericArg`
children are stored, and the evaluator reports `ModuleNotExpanded` rather
than following into the `ModuleDef`'s own tree. Module expansion is the
next slice, and pretending otherwise would produce a plausible-looking
but incomplete activation set with no signal that it was incomplete.

### D11. Result ordering is document order, deduplicated by first occurrence

An id may legitimately appear under more than one active branch. The
result keeps the first occurrence's position and drops later duplicates,
so the output is stable and diffable across runs.

## Acceptance criteria

1. Installing the five readable `.knxprod` corpus archives stores a
   `dynamic_node` tree whose per-archive `choose`/`when` counts match
   [RESEARCH.md §4.3](../../RESEARCH.md)'s table exactly: 1646/2252 for
   `646704-04_ETS4_2012_47_DE_EN`, 5/5 for the Weinzierl ETS4 interface,
   509/982 for the MDT switch actuator, 0/0 for
   `Dummy_Applikation_Secure`. Reinstalling
   `Weinzierl_730_KNX_IP_Interface_ETS4` contributes no new rows after
   content-hash deduplication. Counted from the database, not from the parser's own
   bookkeeping.
2. Every `choose` stored from that corpus resolves its `@ParamRefId`
   against the already-stored `parameter_ref` rows — 0 dangling, matching
   the spike's finding on the same files.
3. Every `@test` stored from that corpus parses into `Test` — 0
   `UnparsableTest` diagnostics over the whole corpus — and the shape
   histogram matches §4.3 restricted to these archives.
4. A `TypeNone`-controlled `choose` from the real corpus takes its single
   default branch, with no comparison attempted and no diagnostic.
5. Unit tests over hand-built trees cover: each `Test` shape including
   every one of the six operators; `@default` fallback; no-match
   (`NoBranchMatched`, nothing activated); missing value; non-numeric
   value; an unknown element kind (`UnrecognizedNode`, subtree not
   descended); a `Module` node (`ModuleNotExpanded`).
6. A database created at schema v2 with blobs already stored gains its
   `dynamic_node` rows after migrating to v3, proving D5's backfill.
7. `docs/` reconciled in the same change: `IMPLEMENTATION_STATUS.md`,
   `KNOWN_LIMITATIONS.md` §3, `COMPATIBILITY.md`, `DATA_MODEL.md`,
   `GAP_ANALYSIS_ETS.md` (T18), `ROADMAP.md`, `ARCHITECTURE.md`. The
   limitation "no parameter editor, parameter values retained but not
   interpreted" stays true and must not be downgraded — what changes is
   that an evaluator now exists underneath it.
8. Full gate suite green: `cargo fmt --all --check`,
   `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`, `cargo run -p xtask -- check-layering`,
   `cargo deny check`.

## Non-goals and follow-ups

* **Slice 2 — module expansion.** Follow `Module/@RefId` into the
  `ModuleDef`'s own `Dynamic` tree, binding `NumericArg` values to the
  module's formal `Argument`s.
* **Slice 3 — the editor.** Rendering active parameters, writing values
  back into the project, and honouring `Access` (whose meaning is still
  open, §4.3's third unknown).
* **Not planned here at all:** validating any of this against ETS's own
  rendering, which would require ETS.
