← Previous: [Group addresses](03-group-addresses.md) · [Manual index](../README.md)

# Datapoint types

A group address carries some bytes on the wire. A datapoint type (DPT) says what those
bytes mean. Without it, "1" on a group address is meaningless — with a DPT attached, it's
either "on," or "1%," or a fragment of a temperature reading, depending entirely on which
type applies.

## Why the same bits mean different things

KNX telegrams are small, and several very different pieces of information can fit in the
same one or two bytes. A single bit can mean "on/off," "up/down," or "step/no-step" —
the bit is identical either way; only the datapoint type tells you which question it is
answering. This is also exactly why two devices disagreeing about a group address's type
is a real problem and not a formality: the same byte decodes to a different fact
depending on which side you believe.

## The `main.sub` notation

Datapoint types are identified by a main type and, usually, a subtype: main type 1
covers 1-bit values in general, and subtype 1 of that (commonly written `1.001`) narrows
it down to specifically "Switch." A handful of everyday examples:

- `1.001` — Switch: one bit, on or off.
- `5.001` — Percentage: one byte, scaled to 0–100%.
- `9.001` — Temperature: a 2-byte floating-point value, in °C.
- `14.xxx` — a family of 4-byte floating-point values (current, power, and similar
  physical quantities each get their own subtype).

> **Note**
>
> KNXBench's own reference notation for a datapoint type internally looks like
> `DPST-1-1` (or `DPT-1` when only the main type, no subtype, is known) — not the dotted
> `1.001` form used above and in most product literature. Both name the same thing; if
> you see `DPST-9-1` somewhere in KNXBench, that's `9.001`.

## What happens when two devices disagree

If two communication objects link to the same group address and state different
datapoint types, KNXBench does not guess which one is correct. It reports the
disagreement as a conflict — naming every type involved — both when resolving a group
address's type for display and when decoding a live telegram on the bus monitor, where a
conflicting address is shown as unresolved rather than misleadingly decoded one way. See
[Group addresses](03-group-addresses.md) for how that resolution works.

## What KNXBench actually does with DPTs today

Be skeptical of any tool that claims to support "all DPTs" — the KNX Datapoint Types
standard defines a lot of main types, including manufacturer- and application-specific
ones running well past main type 30. Here is what KNXBench's codec honestly covers as of
this writing:

- Encoding and decoding is implemented for main types **1 through 30**, covering
  booleans, control bits, small enumerations and bit sets, unsigned and signed integers
  of several widths, both floating-point families, time and date, access data, scenes,
  strings, and a few structured types like date-time. Each one is unit-tested against its
  wire format.
- A datapoint type outside that range is a named, reported error — `unsupported
  datapoint type` — never a silent guess or a corrupted value.
- Where a value would be out of range for its type, or a payload doesn't match its
  type's expected shape at all, encoding and decoding fail with a specific, typed error
  rather than producing a plausible-looking wrong answer.
- Resolving *which* datapoint type applies to a given group address (as opposed to
  decoding bytes once you already know the type) is the inference described in the
  previous chapter — it uses an address's explicit declaration when applicable,
  otherwise the linked communication objects, never a guess based on the bytes.

Main-family coverage is not a guarantee of every subtype's semantics or functional
block behavior. Structured types and some encodings have narrower documented
boundaries in the [DPT audit](../../spec-audits/2026-10-07-dpt-document-audit.md).
This is not full DPT coverage, and it isn't presented as such. The
[Supported and unsupported KNX/ETS functionality](../reference/02-supported-and-unsupported.md)
reference chapter is the place to check a specific type before you rely on it.

[Manual index](../README.md) · Next: [Products and product databases](05-products-and-product-databases.md) →
