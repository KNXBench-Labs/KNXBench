# TypeNumber bound admission in image construction

Scope: `ParameterTypes::value` in `crates/knx-productdb/src/image.rs`, the one value
check behind `build_download_image`, `build_system2_image` and `build_system1_image`
(and the experimental builder's fallback, whose retained-source policy keeps its own
stricter check), and through it `byte_order_probe`: a bound that now refuses turns that
parameter into `SkipReason::Refused(..)` instead of a probe candidate, not into an error
for the whole probe. Narrow application admission, **not** a statement about ETS schema
validity or compatibility.

## Rule

- A `TypeNumber` bound (`minInclusive`, `maxInclusive`) that is **absent** constrains
  nothing, as before.
- A bound that is **declared** must be one this builder can interpret: an unsigned
  type reads it as `u64`, a signed type as `i64`, after trimming whitespace. A declared
  bound that does not read (empty, non-numeric, decimal such as `100.0`, scientific
  such as `1e2`, or beyond the type's domain) refuses the value with
  `ImageError::Value`, naming the `ParameterRef`, the attribute, its raw text and the
  number type. Previously such a bound silently vanished and any value that fit the
  field was written.
- Both bounds are read before the range is compared, minimum first, so an unreadable
  bound is never hidden by a range miss or by its readable sibling. If both are
  unreadable the minimum is reported.
- Unsigned types: both `minInclusive` and `maxInclusive` must read as `u64`. A negative
  bound (`-1`, `-0`, or below `i64::MIN`) is refused like any other declared bound that
  is outside `u64`, minimum and maximum alike. The old builder silently dropped every
  unreadable bound, negatives included. This admission rule does not reinterpret an
  unsupported negative minimum as zero. Signed types read `i64`, unchanged.
- The importer keeps the raw bound text verbatim; nothing here rewrites stored data.
  Source defaults are refused through the same check wherever a default is consumed.

## Not claimed

Float, hexadecimal and scientific bounds are not supported. The inspected Project
Schema23 v01.00.00 §1.1.3.19 (p. 30) describes parameter values as decimal strings;
that value rule does not establish the lexical grammar of these bound attributes.
A refused bound is "not interpreted by this builder", not "invalid XML".
Likewise the accepted spelling is whatever Rust's integer `FromStr` reads after trimming:
a leading `+` and leading zeros are admitted. That is implementation behaviour (as before
this change, and as in the experimental source check), not a normative XML lexicon, and it
is not tightened here.

Compatibility question left open for **both** a negative `minInclusive` and a negative
`maxInclusive` on unsigned types: whether real products declare them is unchecked here
(no private corpus read). Such an image used to build and now refuses; the maintained
corpus gate is where that would show. They are not labelled schema-invalid.

Tests: `image::tests::number_bounds` (unit, public `build_download_image`) and the
`system1_image` / `system2_image` integration tests.
