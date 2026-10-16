# System-300 S-mode property-table images

Status: implementation claimed by session `cf85f96ab058`, 16 October 2026.
No tests or implementation acceptance are claimed by this initial contract.
This is a pure offline table-image prerequisite, not a device downloader.

## Original-source evidence

The original licensed PDFs were rendered and their bodies, tables and printed
page footers visually inspected. No external implementation is an oracle.

- Configuration Procedures `03_05_03` v02.01.01, §3.4.1.2, printed p38:
  discover interface-object type/instance-to-index mapping; load the S-mode
  address/association tables through properties; discover association format
  from the property description. Application loading can introduce new interface
  objects, requiring another scan. The example does not establish product XML
  translation or fixed object indices.
- Resources `03_05_01` v01.10.01, §4.16.7, printed pp234–236, Tables73–74:
  System-300 Address Table realisation Type6, Object Type1, S-mode frame format0,
  PID_TABLE23/PDT_GENERIC_02 array. Element0 is a two-octet current length;
  actual group-address elements start at1, are sorted and contain no individual
  address. The maximum comes from property description/device capacity, not the
  current length. LTE address format1/PID52 is a different representation.
- Resources §4.17.5, printed pp248–250, Tables80–82: association realisation
  Type6 explicitly covers System300 and SystemB. Format0 has octet TSAP/ASAP;
  format1 has word TSAP/ASAP, high part TSAP first. Indices start at1; sending
  uses the first matching row. Property element0 length is separate from the
  data elements; PID_TABLE23 format must be selected from discovery, not guessed.
- Empty address tables accept all group addresses when loaded (p235); an empty
  association table selects standard ASAP=TSAP mapping (p250). Empty images are
  not a promise that communication is disabled. Association writes require the
  Loading state; valid content is used only in Loaded state (p250).

Licensed renders and extracted locator text remain in private task scratch.
These inspected passages are not an exhaustive System-300 conformance audit.

## Bounded implementation contract

Core will provide an explicit, immutable property-array image for the S-mode
address and association tables. Count/length value and actual elements are
separate; a caller must not send a count-prefixed memory image at element1.
No object index is guessed and no property service is sent by this API.

Reuse the existing `systemb_group_tables` validation and shared Type6 association
byte grammar, including ascending unique addresses and deterministic sender-first
rows. Do not copy a second link encoder. Both builders use explicit big-endian
word serialization; this is a selected codec representation, not target evidence.
The explicit association format, object-count bound and table capacities are
inputs, not inferred from a mask or from the number of linked objects.

Preserve existing conservative refusals: invalid/zero/out-of-table objects,
broadcast engineering links, duplicates, missing or multiple sending intent,
format-width overflow and capacity overflow. Missing sender is an API limitation
without independent transmit-disabled evidence, not a universal KNX prohibition.
Never silently truncate or turn a receive link into the sender.

Tests must pin complete independent literal element/length bytes for both
formats, a sender whose address sorts after a receiver, counts beyond255,
empty/default semantics, deterministic input permutation and negative capacity/
index/duplicate/intent boundaries. A realistic framing or sender-order mutant
must be rejected before acceptance, followed by exact source restoration.

## Explicit non-goals

No product/application source translation, parameter image, automatic family or
format classification, interface discovery implementation, APDU fragmentation,
load-state sequence, property-write execution, simulator-family admission,
durable recovery, CLI/HTTP/UI route or supported-mask widening is delivered by
this image boundary. In particular System300 is not admitted as SystemB merely
because the association grammar is shared. A full family pipeline must discover
resources and frame format, validate the entire plan, perform scoped loading and
verification, and cover direct/indirect mutations durably before real writes.
Existing System1/System2/SystemB/070nh public boundaries remain unchanged.
