# Offline download expansion: first codec package

Date: 15 October 2026. Scope: [offline expansion contract](offline-download-expansion.md),
P1 foundations. This is not System-2 programming admission or field validation.

## Delivered software

- `knx_productdb::system2::project_tool_metadata` cross-checks the witnessed
  `TaskSegment/5` against the resolved source ComObjectTable and the single
  application-machine TaskCtrl2. Every original source position survives. An
  exact metadata record is distinguished from retained declarations; neither is
  an executable step. Wrong masks/styles, ambiguous procedures/pointers and
  unrelated object-5/unknown operations are refused. Source ProgramCode is
  borrowed, never rewritten or stripped of metadata.
- `knx_core::commissioning::legacy_object_table::rewrite` implements the explicit
  Resources Type-1/Type-2 grammar: two-byte header, three-byte descriptors. It
  preserves source pointers, selector bits, count, inactive type bytes and tail.
  Inactive slots lose only Communication Enable. Active flags/types are explicit
  inputs with duplicate/range validation. Type 1 forces its mandatory bit7 and
  refuses Update Enable instead of discarding it. This format codec does not
  decide that mask0025 uses Type2, nor build a complete product image.
- `knx_core::commissioning::absolute_load_control` supplies pure absolute data,
  stack and task property-record encoders. No object/machine selection, subtype
  admission, socket or recovery claim accompanies an encoded payload. Existing
  System2 mask admission still fails.

No dependencies, database/project schema or public HTTP/CLI/UI programming path
change. Core stays independent of product XML and networking. The only existing
codec adjustment is making its flag-bit helper visible to sibling Core modules;
Easy3 byte output and public API are unchanged.

## Direct original-PDF evidence

Inspected rendered pages from the original PDFs, not a text/knowledge-base
extract:

- `03_05_01 Resources v01.10.01 AS.pdf`, §§4.18.3.1–4.18.4.1 pp262–266.
  Descriptor diagram p262 gives data/config/type as three single bytes. Config
  table p263 requires Type1 bit7=1 (not zero/reserved); p266 makes Type2 bit7
  Update Enable. Thus copying the existing four-byte Easy3 codec would be wrong.
- `03_05_02 Management Procedures v02.01.02 AS.pdf`, §3.31.3.4 pp139–141.
  Absolute property records have ten bytes and no BIM-M112 machine nibble or
  segment-ID byte. **Property memory-attribute bit7 = 0 disables checksum
  control** (p140). The earlier claim that setting this bit disables control was
  a source-reading error; see the [UP258 correction and decision](../research/2026-10-16-up258-production-decision.md#2-property-checksum-bit-a-source-reading-correction).
  `AbsolutePropertySegment.checksum_disabled` must therefore clear bit7 when true.
  The record grammar remains separate, but opposite checksum polarity is not
  established. Raw product `SegFlags` still needs its own source-to-wire mapping.
- Historical tool-only metadata and exact bounded source correlations remain
  labelled inference in [the archive dossier](../research/2026-10-14-system2-archive-mirrors.md).
  This source projection is not a normative universal XML execution mapping.

## Verification and remaining work

Focused test-first receipts cover missing APIs plus actual ambiguous-control and
wrong-header-diagnostic RED assertions, followed by GREEN. Synthetic expected
bytes are independently written, use distinct nonzero words/pointers and pin the
field widths. Bounds, overflow, zero/max counts, duplicate/inactive slots, table
pointers, unknown steps and unchanged public mask refusal are covered. The
existing real System2 product fixture additionally verifies the full ordered
metadata projection without making the procedure executable.

Observed final gate results are recorded in the implementation status/evidence
when run; the existence of this contract is not a green test receipt.

Remaining: source-bound full image/parameter/GA/association generation;
property-to-object and complete product-step translation; correct interpretation
of source allocation flags; indirect allocation/LSM mutation coverage and durable
property+memory recovery; real-executor simulation and later field verification.
BCU1/System300/SystemB/couplers/RF/Secure are not closed by these pure primitives.
