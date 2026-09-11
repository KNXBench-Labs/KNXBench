# 2026-09-11 — T29: a DPT codec in `knx-core`

Architecture log for the T29 cycle, branch `t29-dpt-codec`, thirteen
commits from `b6d72f8`, merged to `main` as `db28104`.

## What changed architecturally

Two things, and only the second one moves a boundary.

### 1. A new module tree inside `knx-core`

`crates/knx-core/src/dpt/` gains `codec.rs` and `resolve.rs` alongside the
existing `mod.rs`. Nothing new joined the dependency graph — the codec is
domain logic and lives where the domain lives:

```text
apps/knx-cli ──┐
apps/knx-server┼─→ knx-core::dpt  (codec + resolution)
crates/knx-net ┘
```

`knx-core` still performs no IO, parses no XML and knows nothing about
the UI. The codec takes a `DptRef` and a `GroupValue` and returns a
`DptValue`, or takes a `DptRef` and a `&str` and returns a `GroupValue`.
It reads no files and consults no catalogue — which is also the source of
its largest limitation, recorded as `KNOWN_LIMITATIONS.md` §61.

### 2. `GroupValue` moved down from `knx-net` into `knx-core`

This is the boundary change. `GroupValue` — the payload of a group
telegram, either a ≤6-bit inline `Short` or a `Bytes` sequence — used to
live in `knx-net`, the transport crate. A codec in `knx-core` cannot
depend on the transport crate without inverting the layering, so the type
moved down and is re-exported from `knx-net` at its old path. No call
site changed.

The alternative — a structurally identical `DptPayload` in `knx-core`
plus conversions at the crate boundary — was rejected. Two types with the
same shape and a conversion between them is a place for them to drift,
and the conversion would have had to live somewhere that could see both.
ADR-0016 records this.

The move surfaced a latent bug that had been sitting in `knx-net`'s cEMI
encoder. A `GroupValue::Short(v)` whose value exceeded six bits
overwrote the two APCI bits sharing its octet, silently turning a
`GroupValueWrite` into a different application service and losing the
value. Nothing had ever produced such a value, because the only producer
was a human typing `0` or `1` at the CLI. A codec is a producer. The fix
promotes an out-of-range `Short` to a one-octet `Bytes` instead of
corrupting the APCI, with a regression test
(`encode_promotes_out_of_range_short_instead_of_corrupting_apci` in
`crates/knx-net/src/cemi.rs`).

## What the codec actually covers

Main types 1, 2, 3, 5, 6 (except `6.020 DPT_Status_Mode3`), 7, 8, 9, 12,
13, 14, 16, 17, 18. Everything else in the Standard is not implemented.
`KNOWN_LIMITATIONS.md` §61 is the full accounting and is deliberately
long; it is the honest half of this cycle.

Three places where the Standard does not settle a question and this
project did, each documented at the code and in §61:

- **Main type 9's maximum collides with its own invalid-data code.**
  `M = 2047, E = 15` is `0x7FFF`, which is the reserved invalid-data
  pattern for every 9.xxx subtype. The codec honours the sentinel, so
  encode rejects that one value and the family's usable maximum is
  670 433,28 — which is exactly the bound the datapoint-type document
  prints as the family range. Application note AN188 §4 prints the larger
  670 760,96 by ignoring the collision; AN188 §5 reprints the smaller
  figure.
- **`8.010 DPT_Percent_V16` has the same collision**, so its practical
  maximum is 327.66 %, not the printed 327.67 %.
- **Scene numbers are carried at their wire value.** The +1 display
  recommendation is DPT-AS §3.19 NOTE 9 for `18.001` and §3.25 NOTE 16
  for `26.001`, with no such note for `17.001`. A decoded value must mean
  the octet it came from, so the offset belongs to whatever UI displays
  it — and that UI can only apply it honestly if handed the untouched
  value.

## How a group address gets a DPT

By inference, not by reading. A group address does not state its own
datapoint type in the model; `resolve_group_address_dpt` scans the
communication objects linked to it and collects what they state.
`GroupAddressDpt` is `None`, `Single(DptRef)` or `Conflict(Vec<DptRef>)`,
and a conflict is **reported, never resolved** — picking one would be
guessing at the user's installation.

`GroupAddress/@DatapointType` does exist at schema ≥ 21 and is preserved
on import, but is not modelled. The measured consequence, from
`docs/RESEARCH.md` §6.1: 194 of 514 group addresses (38 %) in the corpus
resolve to no DPT at all, and 110 (21 %) have no linked communication
object to infer from.

## Corpus defects found along the way

Two sections of the KNX Standard v3.0.0 Markdown extraction at
`/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/` are incomplete, both
confirmed against the source PDFs:

- **§3.19 NOTE 9** (the `18.001` scene-number display recommendation) is
  missing from the Markdown entirely. It exists on page 49 of
  `03_07_02 Datapoint Types v02.02.01 AS.pdf`.
- **§3.14.3** (`13.100 DPT_LongDeltaTimeSec`) has its body missing.

Anyone verifying this cycle's citations by grepping the Markdown will
find nothing for the first of those. That is the extraction's gap, not an
invented citation.

## What this does not do

No GUI — that is T15, which consumes this codec. No `knx_master.xml` DPT
catalogue, so no enumeration names and no units beyond what the scaled
subtypes' own arithmetic implies. No decoded value has been verified
against real hardware: every test in this cycle is against the Standard's
own stated encodings, which is a different and weaker claim.
