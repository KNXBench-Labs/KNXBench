# System-2 source-image assembly: offline contract

Status: integrated and published as `79156a5d` on 16 October 2026; exact local,
tracking and live remote main equality verified. Fourteen branch stages and
fifteen actual merged stages pass: Rust4623/0/207ignored, Vitest3626/263files,
workspace Clippy/build, actual frontend build, tooling/five repository checks and
the explicit Release real-source refusal regression. Merged acceptance includes
isolated corpus152/152 in33targets and2892unchanged source paths. This is not a
successful whole-Siemens image or a new executable device path. [Receipt](../evidence/offline-download-system2-image-2026-10-16.json).

## Explicit inactive-policy boundary (20 October 2026 source)

The System2 entry now refuses an explicitly declared unqualified
`Static/Options/@DownloadInvisibleParameters`, naming its value, before
parameter evaluation or image assembly. The source reader already retained the
option; the assembler previously ignored it. This is a conservative
unsupported-policy refusal, **not implementation of Background, default or
exclusion semantics**. Empty/unknown values refuse too; extension lookalikes
are not instructions. Existing FixupList/relative/legacy-patch precedence stays.
Without the option the prior bounded active-only contract is unchanged, not
asserted equivalent to ETS's absent-option behaviour. Easy3/System1 policies,
public mask admission and all load/recovery paths remain unchanged.

[Evidence, integrated byte tests and remaining semantics](../research/2026-10-20-system2-download-integrity.md).
The integrated tests pin sparse descriptor indices, sorted address-to-association
mapping, duplicate active-reference refusal, reference/project value precedence
and preservation of nonzero neighbouring parameter bits. They strengthen
existing codecs rather than introducing a new table layout or overlap policy.
Historical acceptance counts below remain bound to their original revisions.

## Scope

`knx_productdb::image::build_system2_image` is an explicit offline entry point.
It reuses the production product database, parameter validation and Dynamic tree
resolver; no second parameter evaluator or manufacturer-specific product branch
is introduced. The existing public image/download route is unchanged. A returned
`DownloadImage` still fails the memory planner's System2 admission check.

Inputs retain the exact product source and request. The System2 path requires the
P1 source-bound metadata witness; the output is always marked with the
`system2-source-image` inference. This is not a normative universal XML mapping,
an executable plan or hardware verification. Scope is source interpretation and
deterministic image generation for engineering/review/testing.

A retained direct FixupList now also [refuses before image assembly](source-fixup-integrity.md);
source XML preservation is not evidence that its transformations were applied.

## Separate table grammars

- Core's legacy descriptor codec uses the two-byte header and three-byte Type2
  descriptor, preserving source pointer/selector/inactive type/tail bytes.
- A legacy association builder reserves the sending row at index ASAP. A row
  containing TSAP FEh is unused, including objects without an explicitly selected
  sending address. Receive-only links follow those indexed rows deterministically.
  No receive-only link is silently chosen as sending.
- Reserved rows count against the association capacity. FEh cannot simultaneously
  be used as a real TSAP; the common address builder is therefore bounded to253
  distinct group addresses for this grammar. Descriptor count remains explicit,
  with bounds, duplicate-link and multiple-sender refusals.
- Existing Easy3 address/association bytes and callers are not changed.

Direct original-PDF evidence: `03_05_01 Resources v01.10.01 AS.pdf`,
§§4.16.3.1/4.16.4.1 printed pp223/226 (GrAT Type1/2),
§§4.17.3.3.1/4.17.3.4.1/4.17.4.3.1 pp246–248 (indexed sending, stored ASAP,
FEh unused), §§4.18.3–4.18.4 pp261–266 (descriptor grammar).
All were read as rendered original PDF pages, not extracts. Historical Type2
GrAT placement is fixed at0116h even though a property pointer is described;
this is not silently generalised into a complete mask0025 execution contract.

## Data integrity and parameter evidence

The System2 path checks unique source segments, 16-bit bounds, data/mask lengths,
absolute-region overlap, complete table placement/capacity, and overlapping
reserved table regions. Parameters cannot overwrite any reserved table region.
Unsupported placements/encodings/flags are named refusals, not dropped content.

Existing placement/type validation is reused, but System2-specific source checks
live in a private `image/legacy_source.rs` module, shared through an explicit
family argument with the bounded System-1 entry. An explicitly placed numeric
field wholly within one octet has no byte-order ambiguity and may replace its
template bits; it need not already equal its source default. Default-width and
bounds validation remain mandatory. [The later bitfield correction](offline-explicit-parameter-bits.md)
extends the original whole-octet exception without waiving inferred placements.
The original Cookbook Load Controls
`02_03_01` v01.00.02 p7 explicitly describes modifying the default memory image
with project parameters. A real source witness contains a different template
byte at an explicitly placed8-bit field; requiring equality there was an
over-strict implementation rule, not a normative requirement. A named regression
reproduced that refusal before the narrow correction.

For inferred Union-member/bitfield/multibyte placements, the independently retained
source-default value must encode to its original source bits. Overlapping active
writes refuse rather than borrowing the070nh later-member-wins inference.
A symmetric default is not evidence of byte order: an edited multi-byte number
without an explicit BigEndian declaration needs a discriminating default; an
unaligned or endian-ambiguous case refuses. Existing text/type validation remains
in force. Source-default corroboration is a conservative boundary on inferred
forms, not a universal KNX requirement or proof of firmware behaviour.

The actual Siemens source contains unqualified `LegacyPatchAlways` declarations;
the [later shared integrity guard](offline-explicit-parameter-bits.md) now refuses
before parameter evaluation. Historically it reached a16-bit field whose template
is zero and whose declared default is nonzero, without ParameterByteOrder.
That missing encoding witness remains unresolved, but is no longer the current
first refusal. The opt-in digest-pinned public-package test asserts the exact
legacy-patch refusal and unchanged public mask admission. Neither outcome is a
successful whole-Siemens image. Earlier discovery tests expecting
full image success remain failed evidence, not relabelled passes. The successful
synthetic source assembly proves only its explicitly covered grammar.

ReadOnInit and priorities beyond absent/Low refuse in this path instead of using
the070nh loss/priority inference. Source masks remain carried and checked; their
interpretation is disclosed as an offline inference, never field acceptance.

## Acceptance requirements

Independent synthetic bytes must distinguish table widths, sending-slot indices,
parameter bits and preserved pointers. Cover sparse/no-sender objects, capacity
including unused rows, FEh collision, malformed segments, overlap, masks,
contradictory defaults, ambiguous encoding, unsupported input and unchanged public
admission. Keep the exact real-product source test opt-in, read-only and isolated.
A real fixture refusal is a remaining evidence contract, not grounds to weaken a
guard or mark the package universally supported.

This does not complete source-to-property procedure planning, allocation side
effects, identity-bound durable property+memory recovery, simulation of the whole
executor or later field validation. Those remain next packages under the
[expansion contract](offline-download-expansion.md).
