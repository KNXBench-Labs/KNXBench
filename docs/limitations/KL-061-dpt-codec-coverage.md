# DPT codec coverage

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-061)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard"></a>

## 61. The DPT codec covers thirty main types with explicit input formats and disclosed encoding rulings

**2026-10-07: document-wide inventory, scoped fixes, no full-conformance claim.**
The [DPT document audit](../spec-audits/2026-10-07-dpt-document-audit.md) maps 251
pages, 280 contents entries and 454 numbered IDs: 305 within the implemented
format families and 149 explicitly unsupported. It fixes V16-percent sentinel
rounding, F16/scaled-V32 input bounds and generic runtime admission for nine
explicit parameter-only subtypes (including the FB exception boundary).
Subtype/application semantics remain partial; the older scoped audits do not
prove full Standard or ETS compatibility.

**2026-10-05 (AR09): 17.001 is a 1-octet value on the wire.** A format-width audit against DPT-AS found that 17.001 DPT_SceneNumber ("1 octet: r2U6") was encoded in the 6-bit optimised A_GroupValue_Write form. It now uses its own data octet and refuses the inline form. All other main types matched the spec width. See [the audit](../spec-audits/2026-10-05-dpt-format-widths.md). Ranges and special values are not yet re-audited, so this entry stays open.

**Limitation.** `crates/knx-core/src/dpt/codec.rs` (2026-09-11, T29;
extended 2026-09-13 and 2026-09-14, E4, twice) can decode and encode main types **1
through 30 inclusive, with no gaps** — thirty main types, counted from
`codec.rs`'s own `decode`/`encode` match arms
(`grep -cE '^        [0-9]+ => decode_' crates/knx-core/src/dpt/codec.rs`
→ `30`). Main type 31 and everything above it returns
`DptCodecError::UnsupportedDpt` unconditionally; nothing about those is
guessed. Leaving the eighteen structured 200-series types in the historical
master catalogue unsupported remains an **accepted scope decision**, not an
automatically reopened codec backlog. The former blanket LTE/system rationale
was wrong: DPT-AS §1.2 p13 restricts the LTE-only classification to structured
HVAC subnumbers 100–499; lighting examples 232.600/251.600 are not in it.
Absent LTE addressing is a dependency for that HVAC scope, not a reason that
every structured value codec would be unusable. The full Standard inventory
has 148 unsupported structured IDs plus 31.101; catalogue counts are not
Standard-wide support counts.

**2026-09-21 (T07): the bus-facing encoder no longer infers the input
grammar.** `encode(dpt, input, DptInputFormat)` requires the caller to declare
`Canonical`, `Decimal`, `Hexadecimal`, `Binary`, or `Text`. Radix prefixes are
not format declarations and are rejected by this path. CLI callers can pass
`--input-format`; HTTP callers can send `inputFormat`; the web client sends a
deterministic DPT-family choice. If an older CLI or HTTP caller omits the field,
that compatibility boundary deliberately calls `encode_inferred_format` so
fixed-width binary bit sets retain their previous wire value. The web form
exposes the same compatibility mode visibly as `Auto` and lets the user select
each explicit grammar; it never labels arbitrary text with a guessed format.
`default_input_format(dpt)` remains a deterministic recommendation for callers
that deliberately want one; it depends only on DPT identity and never on value
text.

The public `encoding_rulings(dpt)` API returns stable identifiers, the exact
Standard context, and KNXBench's decision for every affected encoding below.
That makes the distinction observable before a write instead of burying it in
this document. The inventory is: `scaled-angle-linear-mapping` (5.003),
`status-mode-format-range` (6.020), `invalid-sentinel-precedence` (8.010 and
9.*), `scene-number-is-wire-value` (17.*, 18.*, 26.*),
`datetime-src-is-reserved` and `datetime-invalid-fields-keep-width-only`
(19.*), `format-level-validation-only` (20.*, 21.*, 22.*, 23.*, 25.*, 27.*,
30.*), `strict-null-termination` (24.*, 28.*), and
`signed64-range-typo-corrected` (29.*), and since AR09 (2026-10-05)
`time-period-raw-counter-parameter-only` (7.003, 7.004, 7.006, 8.003, 8.004,
8.006: raw counter; DPT-AS §§3.8.2/3.9.2 limit runtime use unless an FB
specification explicitly permits it). The ruling is metadata; the 2026-10-07
`validate_group_write_dpt` guard separately refuses these six plus 7.013,
8.012 and 20.022 in generic HTTP/CLI writes, which cannot verify an FB
exception. Parameter/diagnostic codecs remain usable. This does not constitute
a comprehensive subtype/FB validator; raw mode and bare-main types make no
subtype claim. T07 itself changed no wire encoding.

Against the ETS master data that number reads differently, and the
difference is worth stating plainly because it has been misread before.
`knx_master.xml` defines 46 *main types* (`docs/RESEARCH.md` §5) — 46 is a
count, not an identifier, and there is no "main type 46": its 46 ids are
`DPT-1` through `DPT-23`, then `DPT-25`, `DPT-26`, `DPT-27`, `DPT-29`,
`DPT-30`, then eighteen structured types in the 200-series (`DPT-206`,
`DPT-217`, `DPT-219`, `DPT-222`, `DPT-229`, `DPT-230`, `DPT-232`,
`DPT-234`, `DPT-235`, `DPT-237`, `DPT-238`, `DPT-240`, `DPT-241`,
`DPT-244`, `DPT-245`, `DPT-249`, `DPT-250`, `DPT-251`). So of those 46 the
codec now covers **28** — every one below 200 — and the eighteen
200-series types remain unsupported. The other two main types the codec
implements, **24** and **28**, are defined in DPT-AS (§3.24, §3.27) but are
absent from that master-data file, which is exactly the asymmetry
`docs/RESEARCH.md` §5 warns about: the master file's catalogue is a
property of *that file*, not of the Standard.
(`docs/IMPLEMENTATION_STATUS.md`'s T29 entry says "fourteen" — the true
count as of T29's date, 2026-09-11; the E4 rounds of 2026-09-13 and
2026-09-14 added
the rest. This heading states the current total, re-measured, not the
count at any one task's snapshot in time.)

**All numbered Standard subtypes in implemented families have format codecs,
not full subtype-semantic support.**
`6.020 DPT_Status_Mode3` used to be one: its wire layout (`B5N3` — five
status bits plus a one-hot three-bit mode field, DPT-AS §3.7) fits none of
`DptValue`'s pre-existing shapes, so it returned `UnsupportedDpt` rather
than being misread as the plain signed 8-bit integer the rest of main type
6 is. The second E4 round (2026-09-14) gave it its own
`DptValue::StatusMode3` variant, so every subtype of every implemented
main type in the 454-ID inventory has a format specimen that decodes. This
does not extend to arbitrary charset subnumbers: `char_set_is_ascii` supports
only 4.001/4.002 and 16.000/16.001 (plus the documented bare DPT-16 default);
bare DPT-4 and unknown charset subnumbers return `UnsupportedDpt`. Nor does
a successful format decode certify a subtype range, enum or runtime use.

**2026-09-13 (E4): main types 4, 10, 11, 15, and 19 added, each with one
Standard-reading judgment call recorded here rather than silently
decided.** `4.*` (`A8`, DPT-AS §3.4) reuses the ASCII/ISO-8859-1
charset-selection logic main type 16 already had (`char_set_is_ascii`),
rather than a second implementation of the same rule; unlike `16`, `4`
gets no bare-main-type default, because the Standard does not print one.
`10.001` (time of day + day of week, DPT-AS §3.11, page 41) represents
day-of-week `0` as "no day" (`Option::None`) rather than as Monday — DPT-AS
§3.11's own Day column prints this directly: `1 = Monday ... 7 = Sunday`,
`0 = no day`, range `[0...7]` ([D], not inferred; the Markdown extraction of
that table truncates at "7 =", which is how an earlier draft of this note
mis-called it undocumented). Only the storage shape is this codec's own
choice ([A]) — the Standard names the code, not a Rust type — and the round
trip is exact (`None` only ever decodes from and encodes back to raw `0`).
`11.001` (date, DPT-AS §3.12) resolves the
two-digit year octet by the century-window rule DPT-AS §3.12 EXAMPLE 5
states directly: a raw value `>= 90` means `1900 +` raw (covering
1990-1999), otherwise `2000 +` raw (covering 2000-2089) — printed in the
Standard, not inferred. `15.*` (access data, DPT-AS §3.16) packs six BCD
digits plus four flag/index bits across four octets with no reserved bits
at all in this format (all 32 bits carry meaning); a BCD nibble above 9 is
rejected as `InvalidData` rather than accepted as a non-decimal digit,
since §3.16 defines the code as decimal. `19.001` (date and time, DPT-AS
§3.20) surfaces a genuine contradiction inside the Standard's own octet 1
diagram: the field-*names* row gives `SRC` (synchronisation source
reliability) bit 6, but the bit-*encoding* row directly beneath it marks
that same bit `r` (reserved), and Note 15 sides with the encoding row,
stating plainly that the seven non-`CLQ` bits of that octet are reserved
and must be zero — confirmed by rendering the source PDF page directly
(page 50) rather than trusting the Markdown extraction, which is
column-misaligned and cannot settle the question on its own. This codec
follows the encoding row and Note 15 ([A], a ruling between the diagram's
two contradictory rows, not a case of "no bit exists"), so
`DptValue::DateTime` has no `src` field; every other named flag in the
octet-2/octet-1 tables (fault,
working day, working-day-unknown, year/date/day-of-week/time-invalid,
summer time, externally-synchronized, `CLQ`) has one, so no bit this codec
*can* assign meaning to is silently dropped. Range checks on Month/Day
(octets 7-6) and Hour/Minute/Second (octets 5-3) are enforced only when
the corresponding invalid-flag says the field is valid; when a flag marks
a field "not valid," only its bit width is checked, not its documented
range, on the reasoning that a clock with no date or no time reading may
legitimately zero- or garbage-fill that octet and rejecting such a
telegram would invent a stricter rule than §3.20 states for exactly that
case. All five new types follow the same reserved-bit policy already used
by main types 1-18: a reserved bit set to anything but zero is
`DptCodecError::InvalidData`, not silently ignored or masked off.

**2026-09-14 (E4, second round): main types 20-30 and `6.020` added, with
one ruling that spans seven of them and four that are local.** The
spanning ruling first, because it is the one most likely to surprise:
**for a main type whose format is a bare enumeration or a bare bit set,
this codec validates the format and not the subtype's own table.**
([A] — a judgment call about which table governs a bare-enumeration/
bit-set format; no clause dictates it.) That covers main type 20 (`N8`,
DPT-AS §3.21), 21 (`Z8`/`B8`, §3.22), 22 (`B16`, §4.5/§8.3), 23 (`N2`,
§3.23/§4.6), 25 (`U4U4`, §8.4), 27 (`B32`, §3.26) and 30 (`B24`, §8.5).
Concretely: an enumeration code a subtype
calls "reserved" still decodes to its raw code, a bit a subtype's table
calls "reserved, set 0" still survives into the decoded value, and a field
a subtype narrows (25.1000's `[0 … 3]` inside the format's `[0 … 15]`) is
not re-narrowed. Three reasons, in order of weight: (1) Main type 20 alone
has **sixty-eight** subtypes whose tables are scattered over six clauses
of DPT-AS — §3.21 (20.001-20.022, sixteen), §4.3 (20.100-20.122,
nineteen), §6.3 (20.600-20.613, fourteen), §7.1 (20.801-20.804, four),
§8.1 (20.1000-20.1005, six) and §9.5 (20.1200-20.1209, nine) — keyed by
application domain; transcribing them into a codec means every
transcription slip silently *rejects* a legal bus value, the worst failure
direction available here. (Counted, not estimated:
`pdftotext -layout "03_07_02 Datapoint Types v02.02.01 AS.pdf" - | grep -oE
'\b20\.[0-9]{3,4}\b' | sort -u | wc -l` → 68. An earlier draft of this
paragraph said "of the order of eighty" and named chapter 10 as a seventh
location; chapter 10 is "Datapoint types for weather encoding" and holds
main types 273 and 274 only, no main type 20 subtype at all.)
(2) Their wording is not uniform: 20.001 and
20.003 say "not used; reserved", 20.002 says "reserved, shall not be
used", 22.100 says "reserved" with "default 0", 21.001 says "reserved, set
0" — three different strengths of prohibition that cannot be collapsed
into one check honestly ([D], the four quoted phrases). (3) In one
case the Standard contradicts itself outright: 22.100 `DPT_StatusDHWC`'s
encoding row (§4.5.1) reads
`0 0 0 0 0 0 0 B BBBBBBBB`, defining bit 8 as a `B`, while the data-field
table printed directly beneath the same diagram lists bits "8 to 15" as
"reserved", "default 0" — read verbatim off page 126 of the source PDF
(`pdftotext -layout "03_07_02 Datapoint Types v02.02.01 AS.pdf"`, which
preserves both the encoding row and the table beneath it), not from a
Markdown extraction ([D], this whole contradiction). §4.5.2's own
Encoding paragraph points the same way without settling the bit:
"depending on the usage of
this DPT in a given Datapoint, some bit-fields may be unused and set to
'0' by the sender and will be ignored by the receiver." ([D]) Nothing is
discarded by this ruling — every bit and every code reaches the caller in
`DptValue::Enum` / `DptValue::BitSet` / `DptValue::DoubleNibble` — so a
caller that *does* know its subtype can apply the table itself. The
commissioning design's `DPT_ErrorClass_System` (20.011) enumeration
(`docs/superpowers/specs/2026-09-13-commissioning-download-design.md`
§5.6, `[0 to 18]`) is exactly such a caller and remains correct at that
layer; it was checked against §3.21 during this slice and matches.

The four local rulings. (a) **`6.020`'s mode field is enforced, unlike a
subtype table** ([A]), because §3.7's Range row states `f = {001b,010b,100b}`
for the *format* itself ([D]) — so a mode field of `000b`, `011b`, `101b`,
`110b` or `111b` is `InvalidData`. Its five status bits keep the
Standard's own inverted polarity ("0 = set, 1 = clear") rather than being
normalised, and print as binary digits for that reason. (b) **Main types
24 and 28 (`A[n]`, NUL-terminated, DPT-AS §3.24 and §3.27) reject a
payload without a terminating `00h`, and a payload with an interior
`00h`.** ([A]) Neither section says what a receiver should do with such a
payload; every tolerant reading is a guess about a non-conforming sender's
intent (cutting at the first `00h` assumes the remainder is padding;
appending a terminator assumes one was lost), so the codec refuses rather
than invents. The same rule makes an embedded `U+0000` unencodable, which
is a real gap for main type 28: §3.27's Range row includes `U+000000`, but
the format cannot transmit it unambiguously. This codec still imposes no
maximum encoded length of its own (§3.24/§3.27's own "cut to the maximum
supported length" rule for an over-long *incoming* string is a receiver
concern, not this codec's), but the layer that owns the APDU budget now
exists and enforces it: `knx_net::cemi::encode_l_data` refuses an NPDU
whose length does not fit the one-octet `L` field with `CemiError::
NpduTooLong` rather than emitting a frame whose `L` octet silently
wrapped **[V]** (fixed 2026-09-14, T5 fix round 1, finding I1). (c) **Main type
29's printed
range is a typo, and this codec follows the datapoint-type rows instead of
the format block.** ([A]) §3.28.1's Range row reads "SignedValue = [9 223 372
036 854 775 808 to 9 223 372 036 854 775 807]" ([D]) — the lower bound has lost
its minus sign, and as printed the range is empty and cannot fit 64 bits
anyway. The 29.010/29.011/29.012 rows in the same clause print
"-9 223 372 036 854 775 808 Wh to 9 223 372 036 854 775 807 Wh", exactly
`i64`'s domain, and that is what is implemented. (d) **Main type 26
carries the wire scene number undecorated** ([A]), like main types 17 and 18 —
see the scene-number paragraph below; §3.25 NOTE 16 is this type's own
note rather than a borrowed one, and the answer does not change.

Payload shapes for the new types follow the AL-AS §3.1.2/§3.1.3 six-bit
inline threshold with no exceptions: main type 23 (2 significant bits) is
the only one of the eleven that travels inline as `GroupValue::Short`;
main type 26 has 7 significant bits and therefore always occupies its own
octet, the same reasoning main type 18 already used. Reserved bits that
*are* format-level — main type 26's bit 7 — are checked and a set bit is
`InvalidData`, consistent with main types 1-19.

There is no DPT main type 46, and this slice was briefed to implement one
— see [§90](../KNOWN_LIMITATIONS.md#90-there-is-no-dpt-main-type-46-46-is-a-count-of-main-types-in-one-ets-master-data-file) for the search that established that and why the
number was plausible enough to survive into a brief.

**Resolution is inference, not a stated fact.** `resolve_group_address_dpt`
and `resolve_project_group_address_dpts`
(`crates/knx-core/src/dpt/resolve.rs`) derive a group address's DPT by
scanning every communication object linked to it and reading `dpt.value()`
off each — a group address does not carry its own type in this domain
model (except see the next paragraph). Per `docs/RESEARCH.md` §6.1, this
inference is genuinely incomplete: **194 of 514 group addresses (38%)**
in the `Unser Zuhause` reference project resolve to no DPT at all, and
**110 of 514 (21%)** have no linked communication object at all to infer
from. A conflicting set of linked DPTs is reported as
`GroupAddressDpt::Conflict` and never resolved down to one guess — RESEARCH
§6.1's rule 3.

**`GroupAddress/@DatapointType` (schema ≥ 21) is modelled since AR09
(2026-10-05, [ADR 0078](../adr/0078-group-address-declared-dpt.md)).** ETS
versions that write schema 21 or later can state a group address's DPT on
the `GroupAddress` element itself. Until 2026-10-05 the importer only kept
it as an opaque attribute. It is now `GroupAddressEntry::declared_dpt`
(absent, empty, value, or malformed with its exact text and a report entry),
and `resolve_group_address_type` weighs it against the linked objects: the
declaration applies when the widths match (a same-width subtype difference
stays visible as `DeclaredDiffersFromLinked`), a width difference — which
Project Schema23 §1.2.7 forbids — is a conflict that decodes and writes
nothing, and an unknown width is `Unverifiable`. `resolve_project_group_address_dpts`,
the bus monitor, bus writes, the CLI, the projection's `dpts` and the CSV
export's read-only column all use this effective type; `resolve_group_address_dpt`
stays inference-only. Store schema v10 lifts keyed opaque declarations from
older stores and counts unkeyed ones (pre-2026-09-20 imports) as
`unlifted_group_address_dpt_declarations`, which turns an absent declaration
into `DeclarationNotLifted`. Measured: `KV v2.5 - demo.knxproj` (schema 21)
carries the attribute on **13 of 13** group addresses, and none of its 26
linked objects states an instance DPT; neither `Unser Zuhause` export
carries it on **any of 514**. Still open, for the web owner: the web shows
only the effective type, not the declared-versus-linked detail, and the
generated binding's doc comment for `GroupAddressNode.dpts` still describes
the linked-only rule. *Update 2026-10-06 (UI owner):* the doc comment now
describes the effective type and was regenerated into the binding. The
declared-versus-linked detail cannot be shown yet: `GroupAddressNode` carries
only `dpts`, not the declaration or the `GroupAddressTypeOutcome`, so the
projection must expose them first (backend owner) before the web can render
them. *Update 2026-10-06 (Alpha, user decision before AR18):* lifted. The
projection carries `GroupAddressNode.dpt_detail` (declared state and text,
linked types, outcome) and the group-address Inspector shows **Declared on
the address**, **Linked objects state** and one sentence for the outcome; a
size conflict is marked. The table column still shows only the effective
type.

**Two sentinel collisions the Standard does not resolve, where the codec
picked one reading and says so.** `8.010 DPT_Percent_V16`'s printed maximum
(327.67%) and its printed invalid-data code are the identical 16-bit value
(`0x7FFF`); the codec honours the invalid-data sentinel unconditionally, so
`8.010`'s practical maximum is **327.66%**, one step below the number
DPT-AS itself prints. Main type 9 (F16, floating point) has the same
collision at its arithmetic ceiling: `M = 2047, E = 15` is bit-identical to
`0x7FFF`, the reserved invalid-data code, so `encode` rejects that one
value and the family's usable maximum is **670433.28** (at `M = 2046, E =
15`) times the subtype's unit — which is exactly the figure DPT-AS itself
prints for the family, while application note AN188 §4 prints the larger
**670760.96** by not accounting for the collision. Neither collision is
settled by the Standard; both entries record which reading this codec
ships and why.

**Scene numbers are carried at wire value; no display offset is applied.**
DPT-AS §3.19 NOTE 9, attached to `18.001 DPT_SceneControl`, recommends
*displaying* a scene number with an offset of +1 (§3.25 NOTE 16 makes the
same recommendation for `26.001 DPT_SceneInfo`, implemented since the
second E4 round). No equivalent note exists for `17.001 DPT_SceneNumber`
in §3.18. The codec applies no +1 to main type 17, 18 or 26: a
decoded value means the octet it came from, not a display convention layered
on top of it. Any UI presenting a scene number to a human owns that +1
itself — applying it a second time here would make the wire value and the
displayed value silently disagree. (NOTE 9 itself is absent from this
corpus's Markdown extraction of the Standard; it was confirmed to exist
against the source PDF. A previous round of this work briefly asserted §3.19
carried no such note — that assertion was wrong and has been corrected.)

**`DPT-16`'s fixed 14-octet field has no length indicator.** A string
containing an interior NUL byte followed by further content is not
representable: decode strips a trailing run of `0x00` as padding (DPT-AS
§3.17: "unused trailing octets... shall be set to NULL"), because nothing
in the Standard's definition of this type provides an escape sequence or a
length prefix that would let interior NUL survive. This is a recorded gap,
not a rule invented to paper over it.

**A payload with bits set above a short type's significant width is
rejected, not masked.** Where a `GroupValue::Short` carries more bits than
its DPT's definition assigns meaning to, the codec returns
`DptCodecError::InvalidData` rather than silently discarding the
out-of-range bits — a device sending such a telegram gets it printed raw,
with the rejection reason, instead of a decoded value that quietly hides
what the device actually sent.

**`bus monitor`/`bus write` only decode/encode when they have a DPT to work
with.** `bus monitor` decodes only when given `--project <path>` — the DPT
comes from resolving the project's linked communication objects, and there
is nowhere else to get it from; without the flag, the monitor prints
exactly what it printed before this slice. `bus write` needs either
`--project` (to resolve one) or an explicit `--dpt <DPST-m-s>`. Its input
grammar is explicit through `--input-format`; omission deliberately selects the
named legacy compatibility parser so existing writes keep their bytes.

**Subtype wording and units beyond the scaled subtypes are not modelled.**
The codec does not consult `knx_master.xml`'s DPT catalogue, so it has no
source for a subtype's displayed unit beyond what a scaled subtype's own
arithmetic already implies (e.g. `%`, `°C`), and no source for enumeration
wording (`up`/`down`, `open`/`close`, and similar per-subtype vocabulary).
A decoded `DptValue` is a typed number, boolean, or string — not a
formatted, unit-labelled, human-worded string.

**No decoded value has been verified against real hardware.** Every test in
this slice checks the codec against the Standard's own stated encodings
(round-trip tests, boundary tests, the two sentinel rulings above) — not
against a telegram a real KNX device actually produced. That is a narrower
claim than "matches what real devices send," and this entry exists so the
difference is not lost.

**Cause.** The remaining limitations are scope decisions and documented
Standard ambiguities (design spec
`docs/superpowers/specs/2026-09-11-dpt-codec-design.md`, decisions E4-D1
through E4-D9): implement main types the Standard extraction documents
unambiguously and the reference corpus needs, leave the rest
`UnsupportedDpt` rather than guess, and record every place the Standard
itself is ambiguous or self-contradictory rather than resolve it silently.

**Impact.** A user working with a group address whose DPT falls outside
the thirty implemented main types, or whose linked communication objects
disagree, or who has none at all, sees `bus monitor` fall back to the
pre-T29 raw output for that address. A user relying on `8.010`'s printed
327.67% maximum, or AN188's 670760.96 figure for main type 9, will see this
codec's numbers differ by one step, deliberately. Code preparing a write can
query `encoding_rulings` and present the relevant judgment before sending.

**Lifted when.** The accepted LTE/system scope changes, a future slice consults
`knx_master.xml` for units and enumeration wording, or reads
`GroupAddress/@DatapointType` directly for schema ≥ 21 projects instead of
inferring from linked communication objects alone — **that part is lifted by
AR09 (2026-10-05, [ADR 0078](../adr/0078-group-address-declared-dpt.md)).** The input-format inference
part is lifted by T07; the explicitly named compatibility helper remains
opt-in.
<!-- END retained catalogue record -->
