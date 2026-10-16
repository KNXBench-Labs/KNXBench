# Explicit single-octet parameter substitution

## Scope

The shared legacy image check now distinguishes **explicit direct `Memory`
numeric fields wholly inside one octet** from inferred Union, cross-octet and
multi-octet fields. For the former, a template need not already contain the
parameter default. Apply the existing MSB-first `ParameterImage` writer; do not
add another encoder. This affects the explicit offline System-1 and System-2
image entries, not Easy3 interpretation, public mask admission or transport.

The predicate is `BitOffset <= 7` and `1 <= SizeInBit <= 8 - BitOffset`, with a
numeric value and direct Memory placement. Probe the default through the existing
writer **before** waiving equality, preserving invalid-width, reference and
segment-bound checks. The requested write still passes reserved-table/pointer,
mask and active-overlap checks. Unknown types, negative signed values, Union
arithmetic, multibyte order and direct FixupList remain subject to their existing
refusals. No storage/domain schema or dependency changes.

A separate source-integrity correction adds nonpersisted presence retention for
`LegacyPatchAlways` and an early shared refusal in Easy3/System1/System2. It does
not execute or decode that directive; source bytes and existing unknown reporting
remain intact. Thus the direct-bit correction does not turn an unhandled patch
instruction into a silently incomplete image.

## Evidence and limits

Original licensed PDFs were rendered and visually inspected, including footers:

- **Project Schema23 v01.00.00**, §1.1.3.17, printed pp29–30: BitOffset locates
  the parameter's most significant bit relative to the first memory octet's MSB;
  its range is0–7. This defines the explicit bit position, not Union addition or
  a universal byte-order transform. The schema's image-generation scope exclusion
  remains in force; this is not a new complete manufacturer-image specification.
- **Cookbook Load Controls 02_03_01 v01.00.02**, printed p7 Figure4: download
  preparation modifies a retained default image with project parameters, objects
  and addresses. Requiring template/default equality for an otherwise explicit
  field was an over-strict implementation rule.
- **Cookbook Manufacturer Tool 02_04_01 v01.00.01**, §5.4 pp18–19: the exercise
  describes eight-bit unions and channel byte shifts. It supplies neither the
  missing mid-octet Union arithmetic nor FixupList patch semantics.

The official Define Parameters search result corroborates explicit MSB placement
and describes mutually exclusive Union members. Direct article retrieval returned
HTTP403; its complete body was **not** inspected. A search result is not used to
invent additional rules. Licensed pages, original sources and byte oracles remain
private. Existing source mapping across retained schema generations is an offline
engineering interpretation, not independently measured ETS output or firmware.

## Tests and actual original-source outcomes

A new actual importer/evaluator/image test failed at the original named
`System1 default disagrees with source bits` refusal before correction. Five
fixed byte specimens cover MSB/LSB ends, interior fields, defaults and edits,
unrelated same-octet bits, surrounding octets and immutable source/database state.
Additional cases keep bounds, bad segment references, table/pointer protection,
changed-bit masks, too-wide values and overlapping writes closed. System-2's
former direct-four-bit refusal becomes an independently expected `60h` write.

A separate self-review found the first implementation waived default width
validation as well as equality. A named width3/default8/request3 regression
**returned an image and failed its refusal assertion**. The correction keeps the
probe and waives equality only; that regression then passes. The initial review
finding and both actual RED runs remain separate private evidence.

The initial thirteen-source run advanced two related sources to returned image
candidates. Four full emitted-segment comparisons against private independent
XML/base64/hand-reviewed Dynamic/table oracles passed; their output was not used
to generate the expectations. Those are **historical bounded representations,
not accepted complete real-source images**. Both sources contain an inactive
`Parameter/@LegacyPatchAlways` declaration whose effect was not interpreted.
The apparent candidate successes are withdrawn from current artifact acceptance.
The temporary probe and initial refusal-test failure remain private evidence.

Three new named runtime regressions first proved that Easy3, System1 and System2
all returned images despite an inactive legacy patch declaration. The shared
builder now refuses before evaluation/assembly whenever the code reader observes
an **unqualified** `LegacyPatchAlways` attribute on a direct Static parameter or
Union member, regardless of activation, placement or lexical value. Empty,0,1,
false and true all refuse: this is a conservative unhandled-declaration policy,
not an interpretation of the directive's boolean/default rule. Existing FixupList
and relative-source refusals keep their earlier precedence; the new declaration
guard still runs before parameter evaluation/assembly. Original XML and importer
unknown notices remain unchanged. Nested extension parameters and qualified lookalike attributes do
not masquerade as this declaration. The legacy EX-IM view records no XML
attribute presence; no legacy-format transform is invented.

`ProgramCode.has_legacy_patch_always` is an on-demand, nonpersisted presence bit,
not an interpreted parameter policy or database migration. All three public
image entries share the same guard. Positive no-directive images, each family,
all declaration lexemes, Union members, exact ancestry/qualification and unchanged
database/source state are tested. The existing private refusal test adds a
closed `legacy-patch` outcome; no unused generic oracle framework is retained.

Current configured expectations are **7 FixupList, 2 LegacyPatchAlways,
3 inferred-default-bit and 1 mid-octet Union refusals**. Actual guarded-source
acceptance is recorded after execution. **Zero accepted complete real-source
images, returned guarded candidates, executable source plans or new device/
firmware verifications are claimed.** Full image semantics still require the
exact activation/default patch rule, not another guessed parameter write.

## Remaining stop lines

- **FixupList:** patch width, instruction/offset semantics, byte order, mask
  interaction and exact mask/master binding remain unproven. The36 observed
  correlations do not waive the shared guard. See [fixup integrity](source-fixup-integrity.md).
- **Union and multibyte:** retained defaults disagree or placements need
  unproven arithmetic/order. The direct-field correction does not waive those
  checks. See [System-1 image](offline-download-system1-image.md).
- **Full plans and recovery:** source scope/merge/capture/substitution and all
  memory/property/setup/pointer/allocation recovery remain separate contracts;
  no second sequencer or downloader is introduced.
- **Other families/cross-cutting paths:** the [source stop-line review](../research/2026-10-16-offline-source-stop-lines.md)
  distinguishes concrete semantic prerequisites from later real-device tests.

The earlier pre-legacy-guard branch passed13stages, including ProductDB931/0.
That is historical acceptance, not the final integrity-corrected candidate.
The final guarded runtime passes ProductDB935/0/29ignored in56reports,
all-target Clippy/build and both explicit original-source regressions. The full
13-stage attempt retains its whitespace-only refusal (new log blank EOF); its
first12stages passed with source/Git/external input equality. The earlier guarded
attempt separately failed the original-relative diagnostic-precedence assertion;
production ordering was corrected, not the assertion. Documentation-delta checks,
actual integration/private corpus and publication remain pending.
[Aggregate evidence](../evidence/explicit-source-bits-2026-10-16.json).
No bus, vendor executable, manufacturer contact,
release, tag, deployment or new hardware admission belongs to this package.
