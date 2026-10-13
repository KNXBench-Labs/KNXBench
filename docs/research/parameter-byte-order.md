# Parameter byte order (`ParameterByteOrder`): evidence review (2026-10-13)

Scope: KL-170 only. This extends the
[2026-10-10 public-source pass](schema23-offline-boundaries.md#kl-170-little-endian-parameter-images),
whose web retrieval had failed, with the local KNX specification corpus,
retrievable public schema copies and independent implementations, a much larger
public product corpus and the privately supplied schema-23 project. It
changes no download behaviour. Tags as in [RESEARCH.md](../RESEARCH.md):
**[V]** measured here, **[D]** documented by a cited source, **[A]** inference.
Machine-readable aggregates: [evidence receipt](../evidence/kl170-byte-order-evidence-2026-10-13.json).

## Result

1. **Still no defining source.** No KNX specification, schema copy or
   implementation inspected defines `LittleEndian` per parameter type, or how
   it combines with bit offsets, floats, texts or negative numbers.
2. **Undeclared means BigEndian by schema default** [D]: two independent
   derivations of the project schema give the attribute as optional with
   default `BigEndian` (below). KNXBench already reads an absent attribute that
   way ([V] on one MDT device, `parameter_image`).
3. **Strong but circumstantial evidence for "whole-octet numbers, low octet
   first"** [V]: in 370 LittleEndian-declared public programs the base image
   holds placed multi-octet defaults low octet first, in none high octet first
   except one Siemens System B family; one manufacturer re-declared 32
   identical layouts from `BigEndian` to `LittleEndian`; two independent tools
   implement exactly that rule.
4. **The same evidence undermines itself** [V]: the same manufacturers'
   BigEndian-declared and undeclared `0701h` programs show low-octet-first
   base images too. Base images follow a firmware convention, not the
   declaration, so they cannot show what ETS writes. That also questions the
   high-octet-first writes KNXBench already plans for those programs (KL-170,
   *Related risk*).
5. **Only a device can settle it.** None with a little-endian program is
   available to the project; the owner's own installation has none [V]. The
   read-only probe of [ADR-0119](../adr/0119-read-only-byte-order-probe.md)
   turns one ETS-programmed device into that evidence.

## Sources inspected

**KNX Standard v3.0.0, local licensed corpus (179 PDFs, text extraction).** A
search for byte-order terms finds only unrelated uses (password hashing in
Project Schema23, the S-record format in Load Controls) and one normative
sentence: *KNXnet/IP Device Management* (`03_08_03` v01.07.03), §2.2 *General
remarks*, printed p. 6, requires parameter values in a **KNXnet/IP device's**
download to be presented big-endian, most significant octet first. That
clause is scoped to KNXnet/IP device configuration; it neither defines nor
mentions the product-file attribute. Page verified against the PDF.

**Schema.** A public copy of the `http://knx.org/xml/project/20` XSD (dated
2020, in a CC-BY-4.0 repository) declares `Options/@ParameterByteOrder`
optional, enumeration `BigEndian | LittleEndian`, default `BigEndian`, with no
documentation annotation.[1] The schema-23 model that xknx-editor generates
gives the same default.[3] The official XSD was not obtained here. The same
schema copy also lists `ParameterByteOrder` as a mask-version *resource* name
with flavours `ByteOrder_BigEndian`/`ByteOrder_LittleEndian`; no mask in the
`knx_master.xml` of the private schema-23 project defines such a resource [V].

**Independent implementations (behaviour only; no code taken).** xknx-editor
(GPL-2.0) reverses the octets of a little-endian program's numeric value only
for whole-octet fields of at least two octets, and leaves sub-octet fields,
text and `BinaryValue` enumerations as they are.[2] It calls this what "the
KNX standard requires" without naming a clause, and its tests are unit tests
of its own encoder, with no device or ETS reference [V]. koolenex (MIT) applies
the attribute to byte-aligned whole-octet fields the same way.[5] Its notes
report hardware captures only for big-endian manufacturers and treat the
absent-attribute default as an observation, not a documented rule.[4] Two tools
agreeing is consistency, not a specification.

**Not obtainable.** The ETS SDK API documentation (installed with the SDK
only), KNX Manufacturer Tool help, and the pre-ETS4 (`.vd*`) table layout:
every local `.vd*` sample is password-protected, and no password was guessed.
Whether the pre-ETS4 format carried a byte-order flag is therefore unknown.

## Product corpus measurements [V]

Method: every unique (by content) application-program file was parsed; for
each placed parameter of type `TypeNumber`/`TypeRestriction` with whole-octet
width ≥ 16 bits, bit offset 0 and an integer default, the base image's octets
at its placement were compared with the default in both orders. A value
whose octets read the same both ways (`0`, `0101h`) is not counted. Union
members count at the union's placement plus their own offsets.

| Corpus | Archives | Unique programs | LittleEndian | BigEndian | undeclared |
| --- | --- | --- | --- | --- | --- |
| `OriginalData/ProductDatabases` | 114 | 525 | 3 | 90 | 432 |
| Public vendor downloads ([crawler corpus](../PRODUCT_DATABASE_CORPUS.md)) | 853 | 3,341 | 590 | 1,011 | 1,740 |

LittleEndian programs of the public corpus: ABB Stotz-Kontakt (`M-0002`) 474,
Busch-Jaeger (`M-0007`) 101, Hager (`M-0009`) 8, Berker (`M-0006`) 5, Siemens
(`M-0001`) 2; 479 of them on mask `0701h`, the rest `07B0h`.

**LittleEndian-declared base images.** 40,657 compared fields hold their default
low octet first and 4 high octet first; by program, 370 low-first only, 2
high-first only, 218 without a discriminating field. The 4 are the two
Siemens `07B0h` programs (one family): a 16-bit check constant and a 32-bit
channel allocation, both high octet first. KNXBench does not plan `07B0h`
downloads, but the pair shows a LittleEndian declaration is not uniform
across manufacturers.

**Control: the same manufacturers' other declarations, mask `0701h`.**

| ABB + Busch-Jaeger, `0701h` | Programs | low-first only | high-first only | Fields low / high |
| --- | --- | --- | --- | --- |
| LittleEndian | 468 | 275 | 0 | 3,810 / 0 |
| BigEndian | 418 | 211 | 26 | 3,634 / 95 |
| undeclared | 137 | 53 | 18 | 604 / 18 |

396 of the 418 BigEndian-declared ones are converted pre-ETS4 programs
(`ConvertedFromPreEts4Data`/`PreEts4Style`), and all 418 list every `Options`
attribute at its default value, as a converter would. Control for
coincidence: MDT's programs (big-endian on a real device, almost all
undeclared) give 94 high-first only, 5 low-first only and 3 mixed programs, so
a single low-first program is weak; the ratios above are not.

**Re-declared layouts.** 32 application/version pairs (30 ABB, 2
Busch-Jaeger) exist as a BigEndian release in ETS4-era packages and a
LittleEndian release in ETS5-era packages, e.g. `M-0002_A-A012-17-91D7` →
`-8FEB` and `M-0002_A-A060-14-4F57` → `-06F8`. In all 32 the segment data
(base images) and all parameter placements are identical. [A] The
manufacturer corrected its declaration for an unchanged firmware layout; the
newer one agrees with the base image. Which declaration ETS honoured for the
older release, and whether such devices were ever programmed high octet first,
is unknown.

**Private schema-23 project** (aggregate only): 12 of 32 embedded programs
declare LittleEndian, all mask `0701h`, two manufacturers; 54 devices use them,
all `ParametersLoaded`. None carries a `LoadedImage` (the two devices that do
use other programs), and the 12 base images hold zeros at every discriminating
placement. It adds prevalence, not evidence of encoding. Its add-in data is
vendor ETS-app state with unverified semantics (KL-174) and was not used.

## Probe planning on real programs [V]

The probe planner (`plan_probe`, ADR-0119) was run offline, without a bus:

- **40 small public packages, 42 LittleEndian `0701h` programs, product
  defaults:** 36 plan, at most 202 octets per device; 19 have discriminating
  fields even at defaults (many defaults are `0`). 6 are refused by the shared
  evaluation: a `choose` controlled by a `TypeRestriction Base="BinaryValue"`
  parameter whose default no `when` covers, which the image builder refuses
  for the same reason.
- **Private sample, real project values, 54 LittleEndian devices:** all 54
  plan; 24 have discriminating fields (124 in total, at most 7 per device),
  30 only one-octet controls; at most 78 octets in 29 runs per device. Not
  read: 729 active parameters placed by `Property` (on 11 of the 54 devices), 25
  texts, 32 values that read the same both ways, 16 parameters whose two
  references disagree, 8 `TypeTime`, 2 texts with non-ASCII characters in a
  program declaring no text encoding.

The `Property` count matters beyond the probe: the image builder refuses
`Property` placements whatever the byte order, so lifting KL-170 alone would
not make those 11 devices downloadable. None of the 11 has a discriminating
field either: all 24 devices a probe could settle are among the other 43, so a
probe answer for the manufacturer would not by itself cover those 11. Both
runs gave identical counts.

## Base-image warning (owner decision, 13 October 2026) [V]

Downloads of programs that declare `BigEndian` or nothing but whose own base
image holds every discriminating default low octet first now carry the named
warning `base-image-low-octet-first`
([ADR-0120](../adr/0120-base-image-byte-order-warning.md)). The rule names no
manufacturer. On 929 packages (105 local, 824 public ones under 6 MB) it flags
32 of the 181 programs a memory download plans at product defaults: 23 ABB and
9 Busch-Jaeger `0701h` programs. None of the 93 MDT (the hardware-verified
program included), 10 Siemens or 11 other programs is flagged. Receipt:
[kl170-base-image-warning-2026-10-13.json](../evidence/kl170-base-image-warning-2026-10-13.json).

## What would lift KL-170

A probe of at least one ETS-programmed device per affected manufacturer, with
the device's project imported unchanged: discriminating fields all
low-octet-first, controls agreeing, no "neither" beyond explained drift. Then
a reviewed encoder change limited to what was observed (whole-octet numbers),
with exact boundary tests, keeping fields across octet boundaries, floats,
texts and negative numbers refused until observed too. The same probe of a
BigEndian-declared legacy ABB device answers the related risk.

<!-- Sources generated by the citation ledger; do not hand-renumber. -->

## Sources

[1] https://github.com/Blizzard26/knxTools/blob/b8bd908f485596c7e142bb4e21347430e0ad16b7/src/main/resources/knx.xsd — knxTools: copy of knx_project_20 XSD (CC-BY-4.0 repository)
[2] https://github.com/XKNX/xknx-editor/blob/e3a2216e10563e18589770dbe0a492b9b57d6660/packages/prod/src/xknxeditor/prod/parser_v2/encode.py — xknx-editor parameter encoder (GPL-2.0)
[3] https://github.com/XKNX/xknx-editor/blob/08908490f6bcdc28a6659598f1baa7a8b045aed0/packages/namespaces/src/xknxeditor/namespaces/files/v23/application_program_static_t_options.py — xknx-editor generated schema-23 Options model
[4] https://github.com/jgrahamc/koolenex/blob/a560776c85e978f8f28e9f3dab24740856e769bc/docs/knx-device-write-protocol.md — koolenex: KNX device write protocol notes (MIT)
[5] https://github.com/jgrahamc/koolenex/blob/ef18ed3762f532487cb8d9d8503d21f420cd491a/server/routes/knx-tables.ts — koolenex writeBits/readBits (MIT)
