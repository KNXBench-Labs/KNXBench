# ADR-0139: Block access and separators reach the parameter tab

Status: accepted for the offline Parameters tab.
Date: 2026-10-17.

## Context

The [product-database gap census](../research/2026-10-17-product-database-gap-census.md)
(G1, G2) found two parameter-tab gaps:

- `Access` on a `ParameterBlock` was not applied. Write authority
  ([ADR-0080](0080-parameter-write-authority.md)) read only the
  parameter's and its reference's own `Access`, so a `ReadWrite` parameter inside a
  `Read` block was offered as writable and a `None` block showed as an ordinary area.
- `ParameterSeparator`s reached only declared grids
  ([ADR-0137](0137-declared-parameter-grids-reach-the-workspace.md)). Headlines,
  information and error texts and rulers in the row view were never shown, and
  `UIHint` was not projected.

Evidence:

- **[D]** Project Schema 23 v01.00.00 §1.1.2.1 `Access_t` (printed page 17):
  "the rights for the ETS user to view and modify parameters", values `None`,
  `Read`, `ReadWrite`. The schema documents the type, not how an element's
  `Access` affects what it contains.
- **[I]** Applying a block's `Access` to everything below it is therefore an
  inference from the type's description, not a documented rule. It is the safe
  direction: it can only refuse a write, never offer one.
- **[V]** A count-only probe over the local collection (17 October 2026, file
  hashes deduplicated, originals unchanged): 254,222 per-program parameter
  references sit below a restricting block in at least one place, 301 of them
  are also referenced from an unrestricted place, and 1 is referenced below
  both `None` and `Read`. No block declares a less restrictive `Access` inside a
  more restrictive one. 402 restricting blocks sit inside a `ModuleDef`, and
  1,903 separators declare `Access="None"` themselves. 213,499 separators lie
  outside a declared grid and 29,785 inside one.
- **[C]** `Access` and `UIHint` on blocks and separators were preserved only in
  the `extra` audit string and reported as unknown attributes.

## Decision

1. **Product DB v24** gives `dynamic_node` typed `access` and `ui_hint` columns,
   following ADR-0126's v23 pattern. The parser fills `access` for
   `ParameterBlock` and `ParameterSeparator`, and `ui_hint` for
   `ParameterSeparator`. These attributes are no longer unknown there.
   `Access` on other hosts (for example `Channel`) stays unmodelled and keeps
   being reported. The migration backfills from the retained package bytes
   ([ADR-0020](0020-migrations-may-rederive-from-stored-bytes.md)) and retires
   the corresponding unknown-attribute rows. Legacy EX-IM programs
   ([ADR-0094](0094-legacy-exim-product-files.md)) have no XML tree to re-read
   and are skipped, not failed.
2. **The evaluator** passes the most restrictive block `Access` down its walk,
   across `Module` boundaries, like the owning channel
   (`ActiveRef::block_access`). The ranking is `None` < `Read` < `ReadWrite`.
   An unknown token restricts like `Read`, as ADR-0080 treats an unknown
   parameter `Access`, and is kept verbatim. A ref activated along several
   paths of one scope keeps the **least** restrictive of them: wherever it is
   shown writable, it is writable (the 301 case above). Activated separators
   are listed in document order (`Activation::separators`) with their position
   among the activated refs. A separator whose label a budget refused is
   omitted, never shown textless.
3. **Write authority** gains one refusal: a field whose `blockAccess` is not
   `null` (and not `ReadWrite`) is read-only, reported as
   `parameterBlockAccessRestricted`. A write to it is refused with `400` and
   changes nothing. The field's own `access` stays its own fact.
4. **HTTP** adds `field.blockAccess` and `section.separators[]`. The latter is
   always serialized and holds `position` (the field index it stands before),
   `group` (keyed like the fields'), `elementId`, `text` (module arguments
   substituted; `null` for a ruler), `uiHint`, its own `access` and
   `blockAccess`, all verbatim. A scope that activated only separators still
   gets its section, and a text-only module section needs no write authority.
5. **The web app** partitions block-restricted fields like field-restricted
   ones (Restricted view, own captions). It shows separators only in the
   Parameters workspace, between the rows of their area: `Headline` as a
   heading, `Information` and `Error` as marked text, `HorizontalRuler` and
   textless separators as a rule, and any other hint as plain text with the
   token kept as data. Separators that are restricted themselves or by their
   block are not shown. Search lists fields only. A declared grid keeps its
   own captions instead of separator rows. A block holding only text is an
   area of its own.

## Consequences

- Fields that were offered as writable inside `Read`/`None` blocks become
  read-only. Their stored values are untouched, and so are downloads, which
  never consulted presentation access.
- Object refs record `blockAccess` too. Nothing consumes it yet; object
  visibility stays as it was.
- `Error` separators are shown when their `when` branch is active, as declared.
  KNXBench does not judge the combination itself.
- Not covered: inline sections, `IndentLevel`, `SuffixText`, separator styling
  inside declared grids, and the browser lane (Vitest and HTTP coverage only).
  The census lists them as later packages.
