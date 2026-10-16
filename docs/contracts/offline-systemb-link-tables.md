# Explicit System-B link-table codec

## Scope

`knx_core::commissioning::systemb_group_tables` constructs the explicitly selected
System-B Group Address Table and Group Object Association Table byte grammar.
It does not select a product/profile, discover an association format, build an
application image, place tables, plan/exercise a download or implement recovery.
No existing public route, mask admission, persistence schema or transport changes.

The [offline expansion](offline-download-expansion.md) remains active. The
[relative source reader](offline-relative-source-segments.md) and
[System-B object codec](offline-systemb-object-table.md) are separate prerequisites;
this new codec does not combine them into a real-product download.

## Direct original-PDF evidence

Original **Resources 03_05_01 v01.10.01** page images were freshly rendered and
visually inspected, including body tables and printed footers:

- §4.16.8.2.4, Table78, printed/PDF pp.238–239: Type7 address table, a two-octet
  count and two-octet addresses, ascending, TSAPs starting at1. There is no
  Easy-3 individual-address slot. The table's zero-length device behaviour is
  special; empty bytes must not be described as disabling all communication.
- §4.17.5.2.5, Tables81–82, pp.249–250: association format0 has one-octet TSAP
  and ASAP, format1 has two-octet TSAP and ASAP; each has a two-octet row count.
  Indices start at1. The first matching association determines the sending
  address. A zero-length association table has a device default mapping and is
  not a literal receive-only disable map.
- Format selection belongs to management/property evidence, not a guessed
  maximum object number: Table80 and p249 describe the property-description
  datatype and consistency with the descriptor indication.

The API explicitly produces count-prefixed **big-endian words**. These clauses
establish the bounded field structure and first-match rule, not every actual
product's memory layout or source-to-property representation. No 0701 readback
is promoted to hardware evidence for System B. Property array service framing,
profile binding and byte-order/placement evidence for an actual target remain
separate responsibilities. This is a bounded passage inspection, not full
standard conformance or product support.

Original **Configuration Procedures 03_05_03 v02.01.01** pp.71–75 were also
inspected: the heading §3.9.3 p71 explicitly scopes this merge procedure to
**mask57B0h**. Its §3.9.3.3 p73 defines indices1/2/3/4/5 for address,
association, object, AP1 and AP2 in that specified procedure. Thus a blanket
claim that all such mapping is undocumented would be wrong, but applying it to
the selected07B0sources without their own profile/source binding is unproven. The example p75 requires current generated
image sizes, whereas the retained modern source contains placeholder-like
allocation/write sizes; modern substitution semantics must be established
rather than passing those literals to a memory writer. Neither this codec nor
AP1 source reconstruction interprets those unresolved placeholders.
Licensed page images and exact source vectors remain private.

## API and integrity policy

`build(format, object_count, links, capacity)` uses an explicit format enum,
one-based `u16` object numbers, typed group addresses and the existing shared
`TableCapacity`. Unlike the Easy-3/legacy builders, it accepts no individual
address and never narrows a wide object through `u8` before validation.

It rejects zero/out-of-table objects, high ASAP/TSAP values for format0,
duplicate object/address pairs, multiple requested sending addresses and any
caller or format capacity overflow. No truncation, clamping, deduplication or
receive-link-to-send promotion is used to make a request fit. A repeated address
across distinct objects legitimately gets one TSAP and separate associations.

The bounded engineering API also rejects broadcast links and an object whose
links have **no explicit sender**. These are API policy/refusal boundaries,
not assertions that the specification prohibits every such table. A receiver-only
object may be safe with independent transmit-disabled object evidence, but this
codec has no such input and must not invent a sending association. Unlinked
objects need no reserved rows in this grammar. Empty inputs produce literal
zero-count images only; the caller must understand the server's default semantics.

Unique addresses are sorted ascending. Associations are deterministic by ASAP,
then the explicit sender, then receiving addresses in ascending order. This
preserves first-match sending even when a receiving address sorts lower.
Input order does not change output and the input itself is not modified.
Lengths and indices are checked before output encoding. Format1 supports the
full nonzero `u16` index/count range as a representation bound, **not** a device
capacity claim. Format0 narrows only after its one-octet index checks.

The existing Easy-3 and indexed legacy codecs remain unchanged: their headers,
zero-based ASAPs, individual slot, reserved sending rows/FEh sentinel and
family evidence are not shared by assertion. Common capacity and GroupAddress
types are reused; the incompatible wide grammar has its own link/result/error types.

## Tests and acceptance

Twelve always-on tests use literal complete bytes and independent row decoding:
formats0/1 widths, sender ordering versus ascending address order, input-order
independence, shared addresses, empty output, malformed links, no/multiple sender,
caller capacities, narrow index refusal, full65535counts/indices and65536row
refusal. The absent-module E0432 RED was observed before implementation.
The first GREEN attempt found a diagnostic-format compile error (GroupAddress
has no Display); the error now uses its raw word and that failed attempt is
retained separately. Corrected focused12/12 pass.

A realistic mutant removed sender precedence from row sorting. The named
sender-order assertion failed against an independently decoded expected row list;
the exact production source was restored and all12tests passed again. Core
all-target tests, owning-crate all-target Clippy and formatting passed in that
restored-source review run. Separate self-review/full candidate and actual merge
acceptance are tracked in [aggregate evidence](../evidence/offline-systemb-link-tables-2026-10-16.json),
not inferred from a successful codec test. No real product image/plan, hardware
compatibility or complete recovery result is claimed.
