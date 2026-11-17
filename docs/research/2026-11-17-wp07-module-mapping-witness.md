# WP-07 — Independently counted module-mapping witness — 17 November 2026

## Evidence and separation

Clean-room observation route admitted by the owner (8 November 2026;
[OWN-01](../open-work/OWN-01-owner-decisions.md#decisions-recorded--8-november-2026),
[rules](../RESEARCH.md#evidence-routes)): ETS exporting fictional projects is an
admitted evidence source for WP-07. This dossier is written in the project's
own words from eight ETS exports of one fictional single-device project
(LUXORliving iON8, application program `M-0048_A-0418-15-66CC`, ModuleDef
`MD-1` with one `Module` `M-1` wrapped in one `Repeat`), plus the two modular
demo projects of the private local corpus (KV demo, private example 001) and
the product-database corpus for structural cross-checks. No vendor code,
product package, original project XML, screenshots or captures enter Git; the
exports and the corpus stay outside the repository. The application-program
XML inside all eight exports is byte-identical (SHA-256 equal across E0–E7)
and its `ModuleDefs` section is byte-identical to the signed manufacturer
package's original application-program XML, so the module definition observed
in the exports is the manufacturer's own definition, not an ETS rewrite.

The KNX specification corpus (179 volumes, v3.0.0) was searched first as the
primary source: `BaseOffset`, `BaseNumber` as module argument names,
`ObjNumberBase`, `ParamOffsBase` and `Allocates` do not occur in any
specification volume; `ModuleDef`/`ModuleInstance` appear only in the project
schema volume (project side). The mapping rules below therefore come from
observed ETS project exports, not from the specification.

## The counted witness

All counts below were produced by re-parsing the exports, not by assertion.

### 1. Argument stride rule (parameter offset base, object number base, channel number)

For every `ModuleInstance` with index `k` (from its `Id` suffix `MI-k`) and
every numeric `Argument` bound to an `Allocator`:

`value == Allocator/@Start + Argument/@Allocates * (k - 1)`

Counted: iON8 exports E0–E7 — 42 module instances, 126 numeric argument values,
**126/126 match** (E0 18/18, E1 18/18, E2 24/24, E3 12/12, E4 18/18, E5 18/18,
E6 0/0, E7 18/18). Private example 001 — 75 instances, **166/166 match**.
KV demo — 96 argument values had no allocator binding (module-level constants),
0 mismatches. Concrete iON8 values: `A-1` (parameter-offset base, allocator
`L-1` ParRange `Start=122`, `Allocates=550`) yields 122, 672, 1222, … ;
`A-2` (object-number base, `L-2` ObjRange `Start=10`, `Allocates=10`) yields
10, 20, 30, … ; `A-3` (channel number, `L-3` ChRange `Start=1`, `Allocates=1`)
yields 1, 2, 3, … . Instance IDs reuse freed indices after delete-and-re-add
(E3→E4), and the argument values follow the reused index, not a global counter.

### 2. RepeatIndex rule

`RepeatIndex = "<a>x<counter> <b>x1"` where `a` is the 1-based element position
of the nearest enclosing `Repeat` (or of the `Module` itself when it is not
inside a `Repeat`) within its enclosing `Dynamic`, `b` is the 1-based element
position of the `Module` itself within the same `Dynamic`, and `counter` is the
instance index `k`. Counted: iON8 **42/42** (all `288x{k} 289x1`), KV demo
**32/32** (single-component `Nx1` for modules not wrapped in `Repeat`), private
example 001 **75/75** (multi-`Repeat` cases like `4127x{k} 4128x1`). The
device's application program is routed via
`DeviceInstance/@Hardware2ProgramRefId` → `M-xxxx/M-xxxx_A-….xml`, because
several application programs in one project can define same-named `ModuleDef`s.

### 3. Memory placement rule

Every `ModuleDef` `Memory` carries `BaseOffset` referencing a `ModuleDef`
`Argument` (the parameter-offset base) plus a relative `Offset`; the absolute
segment address is `A-1 + Offset`. Corpus-wide structural check over 149
product/project files with module definitions: **23 697/23 697** `Memory/@BaseOffset`
and `ComObject/@BaseNumber` references resolve to an argument of the same
`ModuleDef` (0 violations). In the iON8 `ModuleDef` the maximum memory offset
(502) plus field width stays below the allocator stride (550), so instance
blocks cannot overlap by construction; the static `Data` image of the segment
(12 783 bytes) holds all module parameter blocks zeroed (defaults), which is
consistent but cannot falsify placement on its own.

### 4. Communication-object number rule

Every module-scoped `ComObject` carries `BaseNumber` referencing a `ModuleDef`
`Argument`; the instance object number is `A-2 + ComObject/@Number` (local
numbers 0..2 in iON8). Structural resolution counted: iON8 E0 **18/18**, E2
**24/24**, KV demo **200/200**, private example 001 **358/358** — every
module-scoped `ComObject/@BaseNumber` resolves to an argument of the owning
instance (0 unresolved). The numeric `A-2 + local Number` image itself is
produced at download time and is not contained in a project export; that last
arithmetic step is consistent with, but not independently countable from,
offline exports.

## What this witness admits — and what it does not

Admitted mapping contract (observed, independently counted above):

1. Instance numeric arguments follow `Start + Allocates·(k−1)` per bound
   allocator (parameter-offset base, object-number base, channel number).
2. `RepeatIndex` encodes the `Dynamic`-local positions of the repeat anchor
   and the module plus the instance counter.
3. Module memory is placed at `BaseOffset-argument + relative Offset`;
   `ComObject` numbers are based on the `BaseNumber` argument.
4. The application-program definition observed in exports is byte-identical
   to the manufacturer package's (ModuleDefs section), across all eight
   exports.

Not established by this witness: the download-image arithmetic
`A-2 + local Number` on the wire (needs a download against a simulator), bus
behavior, device readback, and any behavior of modules nested inside other
modules beyond the structural references counted above. Missing/duplicate/
nested/repeated *project-side* cases are covered by the counted RepeatIndex
and stride checks (repeated: counters up to 12 in private example 001;
deleted/re-added: E3→E4 index reuse).

## Consequence for WP-07 / WP-37

This supplies the "real, independently counted witness" that WP-07's first
deliverable requires: a mapping contract with counted cases across three
independent projects (8 exports of one fictional project plus 2 corpus
projects), 42 + 32 + 75 module instances and 292 counted argument values, all
matching. WP-37's remaining "module-scoped parameters" item depends on this
authority (see [WP-37](../open-work/WP-37-pending-changes-across-ui.md)); no baseline for
module-scoped rows exists until module downloads exist, which is unchanged by
this dossier.

## References

[OWN-01](../open-work/OWN-01-owner-decisions.md) ·
[RESEARCH evidence routes](../RESEARCH.md#evidence-routes) ·
[WP-07](../open-work/WP-07-module-instance-authority.md) ·
[KL-068](../limitations/KL-068-repeated-module-activation.md) ·
[WP-37](../open-work/WP-37-pending-changes-across-ui.md) ·
[schema 23 activation differential](schema23-activation-differential.md)
