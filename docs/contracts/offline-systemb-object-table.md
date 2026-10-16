# Offline System-B object-table codec

## Scope and admission

P4 adds `knx_core::commissioning::systemb_object_table`, a pure, explicitly
selected **Group Object Table Realisation Type 7 / Format 1** byte codec.
It is not a product image builder, source translator, download plan or executor.
No mask is newly admitted; the existing public image/download paths are unchanged.
No persistence schema, dependency, transport, CLI, HTTP or UI changes are made.

The [offline expansion](offline-download-expansion.md) remains open. A successful
synthetic codec test does not establish a successful real-product image or device
compatibility. The existing property-based System-B downloader remains
simulator-only, and complete durable recovery remains a separate prerequisite.

## Original-source basis

The original **Resources 03_05_01 v01.10.01** PDF was rendered and inspected:

- §4.18.6, printed/PDF pp. 269–270: System B uses the Type-7 resource;
  `PID_TABLE_REFERENCE` and the table property are separate resources.
- §4.18.6.2.4.1, Table 86, p. 271: a two-octet length and two-octet descriptors;
  descriptor indices / ASAPs run from **1** through the length, not from zero.
- Table 87 and §§4.18.6.2.4.1.2–.3, p. 271: update/transmit/initial-read/write/
  read/communication flags occupy bits 15 through 10; priority occupies bits
  9–8. Initial read is not a legacy memory segment selector. Enabling it can
  produce reads after reset and is therefore an explicit input, never a default.
- §4.18.6.2.4.1.4, Table 88, p. 272: value types include the legacy 0–14
  subset, additional 5/7/9/11/12/13-octet sizes, codes 21–254 for 15–248
  octets, and code 255 for **252** octets. Sizes 249–251 have no code here.

Licensed page images and real product XML stay private. This is a bounded
inspection, not a complete standard/profile-conformance audit. The API explicitly
consumes a big-endian Format-1 table; selecting and locating that representation
for an actual product still requires source evidence. This codec does not infer
parameter byte order from a table representation.

## Behaviour

`rewrite(base, active)` validates the complete input before returning a rewritten
copy. Active numbers are `u16`, one-based and bounded by the declared source
length. Zero, duplicates, out-of-range objects, absent headers and truncated
source tables are explicit structured errors; nothing is clamped or skipped.

For active objects, the descriptor is formed from explicit shared communication
flags/priority, explicit initial-read selection and a System-B value type. The
existing flag/priority encoding and legacy value-type subset are reused, not
reimplemented in a product adapter. Inactive descriptors retain every bit except
communication-enable bit 10, which is cleared. This is the codec's explicit
rewrite policy, not a claim to evaluate product activation.

The original two-octet length, all bytes after the declared descriptors, and the
input buffer are preserved. A zero-length table and an odd-length tail are
supported. No table growth, relocation, allocation, data-pointer synthesis or
resource/object-index mapping is performed. The full `u16` count can be processed;
this is a format bound, not a declaration of any device's capacity.

`ValueType::from_code` accepts every defined octet. `for_bits` returns `None` for
unrepresentable sizes rather than rounding; the 255/252-octet exception is explicit.

## Verification and limits

Ten always-on tests assert literal complete bytes, each independent flag and
priority, inactive-bit preservation, empty/malformed/duplicate/out-of-range
inputs, order independence, the last object of a 65,535-object table, trailing
bytes, all type codes and the unrepresentable size gaps.

The initial missing-module RED was observed. A deliberately omitted initial-read
bit failed the exact named flag assertion (expected high octet 20h, observed 00h);
the production source was restored before subsequent tests. Focused tests pass.
An initial owning-crate Clippy run rejected the constant-sized chunk iterator;
that attempt is retained separately and the implementation now uses the requested
array-chunk API. Current gate/publication status is recorded in
[the aggregate evidence](../evidence/offline-systemb-object-table-2026-10-16.json),
not inferred from this contract.

The [real-source baseline](../research/2026-10-16-systemb-offline-prerequisites.md)
produced three partial, non-executable resolutions. This codec does not change
those results, interpret `AppliesTo`/image-property fragments, assemble relative
parameter images or supply whole-mutation recovery. No live bus operation or
manufacturer executable was used.
