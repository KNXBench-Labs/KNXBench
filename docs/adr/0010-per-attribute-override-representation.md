# ADR 0010: Overrides are represented per attribute with an explicit empty state

Date: 2026-09-02
Status: Accepted
Session: 3

## Context

ADR-0004 established `Resolved<T> { value, layer }` and the `Layer` enum:
a value's provenance travels with it, since `ComObject` → `ComObjectRef` →
`ComObjectInstanceRef` resolve through three source layers and export
needs to know which layer a value came from to decide whether to write it
back. That decision stands unchanged.

What Session 2's own sketch of `ComObjectInstance` left unresolved is *how
many* of these resolved values one object carries, and what happens when
an attribute the override chain covers was never stated at all. Measured
against the reference project's 907 `ComObjectInstanceRef` elements during
Session 3 implementation (Task 6/8/10):

- 758 carry a `DatapointType` attribute; 149 do not.
- Of the 758, 497 are the empty string (`DatapointType=""`) and 261 carry
  an actual value.
- The five flag attributes (`ReadFlag`, `WriteFlag`, `TransmitFlag`,
  `UpdateFlag`, `CommunicationFlag`) are each stated independently: 39,
  18, 27, 30 and 8 times respectively, never all five together on the same
  instance.

Two distinct facts are visible in the source data and neither may be
collapsed into the other:

1. **Presence is per attribute, not per object.** One `ComObjectInstanceRef`
   can state `DatapointType` while leaving `Text` unstated. A single
   `Resolved<T>` (or a single `Layer`) per object cannot express "this
   attribute is at the instance layer, that one still resolves from the
   application program."
2. **Empty is not absent.** ETS writes `DatapointType=""` to mean "the
   program's datapoint type is deliberately cleared here" — a real,
   present, empty value — distinct from the attribute never having been
   written at all. Collapsing both into `None` (or into `Resolved<T>`'s
   absence) loses 497 attributes' worth of that distinction in this one
   reference project alone, and export then writes a file that
   demonstrably differs from the one that was read: a device whose
   `DatapointType` ETS explicitly cleared would come back with
   `DatapointType` never written, which is a different, incorrect
   statement about the device.

`CLAUDE.md` forbids silently discarding information. An `Option<Resolved<T>>`
per attribute would still conflate these two states; a model that only
distinguishes "has a resolved value" from "does not" cannot represent what
ETS itself distinguishes.

## Decision

Every overridable attribute is `Override<T>`, one per attribute, not one
`Resolved<T>` per object:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Override<T> {
    /// The attribute was not present in the source.
    #[default]
    Absent,
    /// The attribute was present and its value was the empty string.
    Empty,
    /// The attribute was present and carried a value.
    Value(Resolved<T>),
    /// The attribute was present, non-empty, and could not be parsed into
    /// `T`. The raw text is kept verbatim.
    Malformed(String),
}
```

`ComObjectInstance::text`/`description`/`dpt` are each `Override<T>`
independently; `ResolvedFlags` holds five independent `Override<bool>`,
one per flag, since the reference project's own data shows they are never
stated together. `Resolved<T>` and `Layer` are unchanged from ADR-0004 —
this ADR wraps them per attribute, it does not replace what they mean.

Export is a total function of this shape, with no separate bookkeeping to
consult: `Override::Absent` writes no attribute, `Override::Empty` writes
an empty one, `Override::Malformed(raw)` writes `raw` back exactly as it
was read, and `Override::Value(r)` writes `r.value` only if
`r.layer.is_exported()` is true — the same rule ADR-0004 already
established, now applied per attribute instead of per object.

### Amendment (post-Session-3 review): the `Malformed` state

The original three states covered every value the importer could *parse*.
A present value it could **not** parse (a `DatapointType` that is not a
DPT, a flag spelled neither `Enabled` nor `Disabled`) was collapsed into
`Absent`: the problem was reported, but export then wrote no attribute at
all, so a file that came in with an unreadable value went out having
silently lost it. That contradicts CLAUDE.md's "never silently discard
information — preserve it where technically possible", and preserving it
*is* technically possible: keep the raw text.

`Override::Malformed(String)` is therefore the fourth state.
`is_present()` is true for it, `value()` is `None` (there is no `T` to
hand out), and `malformed()` returns the raw text. The importer must still
record a problem when it produces one — the model carries the data, the
report carries the diagnosis; neither substitutes for the other.

This applies only to fields modelled as `Override<T>`. A field modelled as
a bare value or an `Option<T>` has nowhere to keep raw text, so an
unparsable value there still falls back to the type's default with the
problem reported — a known asymmetry, not an oversight.

## Alternatives considered

**`Option<Resolved<T>>` per attribute.** Closer than a single
`Resolved<T>` per object — it does separate attributes — but still cannot
express `Empty` distinctly from `Absent`: both would have to map to
`None`, which is exactly the 497-attribute loss this ADR exists to avoid.

**A side table of raw, unparsed attribute strings kept alongside the typed
model.** Splits the truth into two places: the typed field says one thing,
the raw string might say another, and every reader has to decide which one
to trust. `Override<T>` keeps the presence state and the resolved value
in the same field, so there is exactly one place to look.

**Represent emptiness with a sentinel value of `T` itself** (e.g. an empty
`DptRef` or a magic string). Type-specific, error-prone (a legitimate
value could collide with the sentinel), and does not generalize to `bool`
flags at all — there is no "empty bool" to reuse as a sentinel.

## Consequences

The model is wordier: every overridable field is a three-state enum
instead of a plain value or a two-state `Option`. This is the direct cost
of the 497-vs-149 distinction being real information, not incidental
detail.

Export becomes a total function of the model with no "was this really
absent?" guesswork — Task 18's writer has exactly one rule per attribute
(`Absent` → nothing, `Empty` → `attr=""`, exported `Value` → `attr="..."`)
and no special case for datapoint type versus any other overridable
attribute.

The UI (Session 5) can show which layer a value came from per field, which
is the behavior ETS users expect from the properties inspector — a field
resolved from the application program looks different from one the
project itself overrides, attribute by attribute rather than object by
object.

The semantic-equality comparison (`compare.rs`, ADR-0007) inherits the
same states for text and datapoint type (`Option<String>` with
`None`/`Some("")`/`Some(raw or value)`) — `Override<bool>`'s comparison
narrows to a plain `Option<bool>`, since a boolean genuinely has no third
representable state; `Override::Empty`, `Override::Malformed` and
`Override::Absent` all collapse to `None` there specifically, a
documented, narrow exception to this ADR's own rule, verified harmless
against the reference project (none of its five flag attributes is ever
empty or unparsable). Export writes all four states back correctly
regardless; only this comparison cannot tell them apart.
