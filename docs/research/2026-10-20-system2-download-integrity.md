# System-2 download preparation: evidence and integrity follow-up

20 October 2026. Scope: generic offline image assembly, not a new executable
System-2 download route. The owner's behavioural research supplied leads about
selection, tables and loading; it supplied no defining implementation, release-
bound ETS byte oracle or reference-tag grammar. No proprietary executable/code
was inspected or copied for this follow-up.

## Evidence classes

Keep three decisions separate: **which references contribute values**, **how
communication tables are encoded**, and **which ordered load operations execute**.
A format definition does not decide an engineering tool's selection policy.
A returned image does not grant executable admission; a passing refusal test is
not a successful product download.

The following original licensed PDF body pages were rendered and visually
inspected, including their printed footers; extracts and rendered pages remain
private. These citations paraphrase, rather than reproduce, the licensed text.

| Source | Independently supported fact | Not established by that source |
| --- | --- | --- |
| `02_04_01 Manufacturer Tool v01.00.01.pdf`, §2.2, printed pp5–6 | Several references can use one parameter with different reference defaults/properties; Union members share memory as alternatives. | Complete inactive/legacy patch selection, numeric tag ordering or overlap winner rules. |
| `03_05_01 Resources v01.10.01 AS.pdf`, §§4.17.3.3.1–4.17.4.3.1, printed pp246–248 | Legacy sending rows use the object's ASAP position; unused TSAP FEh rows can preserve the indexed positions. Type2 checks the stored ASAP. | A source Module's effective base number or a universal mask0025 source translator. |
| Same Resources, §§4.18.3.1–4.18.4.1, printed pp262–265 | The legacy descriptor carries pointer/configuration/type; priority codes are system00, urgent10, normal01, low11; Type2 uses bit7 for Update Enable. | A mapping from all product-source priority labels to those codes. |
| `03_05_02 Management Procedures v02.01.02 AS.pdf`, §3.31.3.4, printed p142 | Data-relative allocation0Bh has separate size, Mode and fill fields. Mode bit0 requests server-side fill; otherwise contents are kept. | A modern XML flag mapping, an implicit client image transfer, or full allocation recovery. |

The existing [System-2 image contract](../contracts/offline-download-system2-image.md)
separately records the original address-table sources, individual-address slot,
family boundaries and preserved source pointers. The existing
[relative-image investigation](2026-10-16-relative-image-integration-gaps.md)
remains authoritative for unresolved source integration; allocation is not
rewritten by this package.

## Implementation inspection and correction

`knx-productdb::image` already resolves values per reference: supplied project
value first, reference default second, parameter default last. It writes only
Dynamic-active, supported unscoped references. Conflicting values for two active
references sharing a parameter refuse; overlapping distinct fields also refuse.
System2 does not borrow Easy3's later-Union-member inference. No numeric-tag
order, inactive-zeroing policy or default-Union selection is added here.

The observed omission was narrower and reproducible: `Static/Options` retained
`DownloadInvisibleParameters`, but `build_system2_image` returned an active-only
image without interpreting that explicit declaration. A synthetic test with an
inactive field containing **A5h**, definition default7, reference default9 and
supplied value33 returned an image despite `Background`. It failed the intended
refusal assertion before the correction.

The System2 entry now refuses **any explicit unqualified**
`DownloadInvisibleParameters` option, naming its exact value, before evaluation
or assembly. This is a **conservative unsupported-policy boundary**, not an
assertion that KNX forbids such declarations or that every declared value has a
known meaning. Empty and unrecognized values refuse too. Qualified extension
attributes and Options nested under an extension do not become instructions.
The absence of this option keeps the pre-existing bounded active-only contract;
it does **not** prove ETS's absent-option policy. FixupList, relative segments
and LegacyPatchAlways retain their earlier refusal precedence. The exact UP258
source still reaches its earlier legacy-patch refusal, not a new success.

No parser/schema/database migration, source retention, import reporting, Core
codec, public mask admission, task identity, allocator or transport changes.
System1 and Easy3 retain their existing policies; this System2 guard is not a
claim that those siblings implement invisible-parameter semantics.

## Integrated regression evidence

The tests use the real synthetic-source importer, evaluator and image builder.
Literal expected bytes are independent of the production encoders:

- A four-descriptor source activates objects3 then1; slots0 and2 remain present,
  with pointers/types retained and only Communication Enable cleared. Group
  addresses sort by their raw16-bit values; a lower-address receiving link does
  not displace object3's selected sending row. Reversing input links gives the
  same complete image; public planning remains refused.
- Two different active references to object1 refuse as a duplicate, without
  database writes.
- A three-bit field uses reference default5 instead of definition default2,
  then project value3. Nonzero neighbouring bits and octets survive; original
  source data and reference attribution remain distinct from generated bytes.
- The explicit inactive-policy regression and extension-lookalike positives
  verify the bounded refusal rather than guessing Background/Default/None bytes.
  Those strings are test inputs, not a claimed exhaustive schema enumeration.

Three controlled source mutants were caught by their named integrated tests:
compacting descriptor slots, disabling duplicate-number admission, and clearing
neighbouring parameter bits. Original Core bytes were restored exactly and the
focused **15-test System2 image suite passed** afterwards. An initial coverage
run had13passes/1failure because the test expected the wrong error wording;
that assertion was corrected to the existing exact diagnostic. It is not
counted as a reproduced production defect. The earlier one-test policy RED and
subsequent GREEN are separate evidence. A failed mutation-harness setup found
no Cargo on its interpreter PATH before any source mutation; the corrected
harness established a green baseline before all three mutations.

The maintained change-scoped gate on main-first merge `76864b51` passed all
14planned stages with frozen inputs and empty userDataTouched/userDataUnchecked:
317tooling tests,2990Rust tests passed/0failed/144ignored, all-target Clippy and
97/97selected offline corpus tests. The exact merge was pushed and local,
tracking and live main equality verified. No full web/browser or full workspace
release gate was claimed; the frontend production build was a separate passed
desktop-input preflight. This is self-reviewed final source, not independent
final-source approval or physical/ETS byte-equivalence acceptance.

The subsequent [UP258 source inventory recheck](2026-10-16-up258-public-package-evidence.md#independent-inventory-recheck-20-october-2026)
confirms the owner's counts and corrects the short-data premise: all four
decoded Data blobs equal their reserved sizes. No padding or length relaxation
is needed for that exact source.

## Remaining questions and next admissible work

1. Obtain independent defining evidence or discriminating release-bound outputs
   for LegacyPatchAlways's presence/default, inactive-reference instance policy,
   Background/default/exclusion masks and default-Union behaviour. A missing
   instance must not be replaced by an invented patch value.
2. Establish numeric reference-tag identity/order before permitting overlapping
   writes. Current conflict refusal is deliberate and safer than guessing which
   reference wins; it is not an ETS-equivalence claim.
3. Admit text/float/date/colour/raw-data codecs individually with bounds,
   rounding, encoding, padding/termination and length-field evidence. DPT wire
   definitions alone do not establish manufacturer-parameter storage encoding.
4. Preserve sparse object indices; module bases need their own scoped mapping
   witness. Product labels such as `High`/`Alert` remain unmapped for System2;
   `Normal` must never default to system00.
5. Table sizes, relocation, task application identity/PEI, ordered source-to-load
   translation and durable recovery still belong to their existing contracts.
   Allocation fill does not itself authorize or require a separate image write.

Functionally valid KNX formats and byte-identical output from a particular ETS
release remain different acceptance goals. No real device, desktop, vendor
contact or release operation belongs to this follow-up.
