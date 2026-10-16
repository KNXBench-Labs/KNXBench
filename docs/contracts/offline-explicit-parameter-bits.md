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
`LegacyPatchAlways` and an early refusal in the strict System1/System2 entries. It does
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

Three historical runtime regressions exposed returned images in the presence
of an inactive declaration. An initial all-family refusal was tried but **not
accepted**: the actual regular corpus demonstrated an independent Easy3 policy
change, including established positive image/readiness/executor fixtures. That
trial and its failed/interrupted acceptance remain separate evidence. None of
those positive goldens is weakened or reclassified to make the trial pass.

The final scope is the **two strict offline System1/System2 source-image
entries**, whose new source assembly must not omit the uninterpreted declaration.
They refuse before evaluation/assembly whenever the reader observes an
**unqualified** LegacyPatchAlways attribute on a direct Static parameter or Union
member, regardless of activation, placement or lexical value. Empty,0,1,false
and true all refuse conservatively; no boolean/default/patch rule is invented.
Existing FixupList and relative-source refusals keep their prior precedence.
Nested extension parameters and qualified lookalikes are not this declaration.
Original XML and importer unknown notices remain unchanged.

ProgramCode.has_legacy_patch_always is an on-demand nonpersisted presence bit,
not an interpreted parameter policy or migration. The legacy EX-IM view records
no XML attribute presence. No legacy-format transform is invented.

### Existing Easy3 is a separate unresolved source policy

Easy3 is not affected by the direct-bit correction. Its existing source/inference
policy, positive golden comparisons and executable admission are preserved. A
named public scope regression first fails under the overbroad trial guard, then
requires unchanged emitted segments, retained declaration presence and unchanged
DB state. This is a compatibility/scope test, **not** a semantic interpretation or
new full-image acceptance of LegacyPatchAlways. Source bytes and unknown notices
remain retained; the missing Easy3 directive rule is explicitly open. No equality
correlation is promoted to a no-op rule, and no new mask/device permission follows.

The source-oriented System1 manifest remains7FixupList/2LegacyPatchAlways/
3inferred-default/1mid-octet-Union refusals in13distinct applications/two packages.
The exact public UP258 source also refuses the uninterpreted legacy declaration
before its historical multibyte witness. **Zero new accepted complete real-source
images, complete source plans or device/firmware verifications are claimed.**

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

The final scoped branch passes12stages: ProductDB935/0/29ignored in56reports,
exact public System2 source1/0, the Easy3 scope regression, owning all-target
Clippy/build and all format/docs/repository/whitespace checks. Source/Git/external
inputs remain frozen. The earlier all-family935/0trial is historical: its actual
corpus failures and intentional interruption are not accepted or erased.
Documentation-delta checks and the corrected actual15stage integration including
regular private corpus remain pending. No full workspace/web acceptance is
claimed for this scoped package.
[Aggregate evidence](../evidence/explicit-source-bits-2026-10-16.json).
No bus, vendor executable, manufacturer contact,
release, tag, deployment or new hardware admission belongs to this package.
