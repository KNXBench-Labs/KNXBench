# Document-wide DPT inventory and scoped conformance review (2026-10-07)

## Source, scope and evidence limits

Primary source: **03_07_02 Datapoint Types v02.02.01 AS.pdf** (DPT-AS),
from the authorized local KNX Standard corpus. Source SHA-256:
`72f924a6c793314d1e20573d71f697f07ce76072525b80c53ee0f68e3b7190af`.
The official attachment was also retrieved and has the same SHA-256.[1]

All **251 physical pages** were extracted. The unnumbered cover is followed
by **250 independently matched printed page footers**; cited physical/printed
pages agree. The inventory contains **280 contents-section entries** and
**454 distinct numbered DPT IDs across 103 main numbers**. The overview has
453 IDs; the definition of **249.600** on p192 adds the missing ID. The
numbered-ID inventory is not an inventory of every sentence, footnote or
unnumbered legacy format.[1]

This review maps the whole document, not just one ETS master catalogue. It
combines source-text inventory, targeted table/rule review, previously checked
PDF diagrams, code/caller review and independent regression vectors. It is
**not exhaustive visual inspection of 251 pages or proof of complete subtype,
Functional Block (FB), manufacturer or ETS conformance**. Licensed page text
and rendered images stay in owned private scratch, not Git. No live bus,
product migration, UI changes, deployment, release or publication is included.

## Machine-readable inventories

- [Pages](2026-10-07-dpt-pages.csv): 251 rows; page identity, document role,
  section starts and located DPT IDs. Text inventory is not semantic proof.
- [Sections](2026-10-07-dpt-sections.csv): 280 rows; all contents entries mapped
  to the format/unsupported matrix or explicit general/annex scope.
- [Types](2026-10-07-dpt-types.csv): 454 rows; source locations, format width,
  actual codec boundary, decoder, runtime restriction, partial semantic status,
  KL-61 and test evidence. `source_pages` are located definition/reference
  candidates, not a claim that every listed page is the sole definition.
  `identifier_class` follows the numbering scheme in §1.2, p13; it is **not**
  a substitute for the individual definition's Use column or FB rules.[1]

`dpt_spec_document_inventory` independently exercises all 454 IDs: 305
in-scope IDs have a decoded format specimen and re-encoding through the
existing input grammar; fixed-width IDs reject wrong lengths; all 149
out-of-scope IDs return `UnsupportedDpt` through decode and both encoders.
A format specimen need not satisfy the subtype's narrowed range or enum table.
The CSV is audit/test evidence, not a new runtime catalogue or domain model.

## Coverage by normative area

| Source area | Implementation / tests | Verdict and residue |
| --- | --- | --- |
| §§1–2, p12–25: identifiers, notation, use and overview | `DptRef`; codec payload helpers; document inventory; KL-61 | Metadata/format coverage; numbering does not establish permitted application use. Unknown subtypes of most implemented families remain format-level interpretations, not registered subtype support. |
| §3.1–3.4: B1, B2, B1U3, character | `decode_b1`, `decode_b2`, `decode_b1u3`, `decode_a8`; existing codec/width tests; inventory | Format implemented. Generic display words do not resolve subtype vocabulary. Bare DPT-4 and unrecognized 4/16 charsets are unsupported. |
| §3.5–3.15, p30–45: numeric families | U8/V8/U16/V16/F16/U32/V32/F32 codecs; `dpt_spec_semantics_audit`; existing codec/width tests | Three confirmed numeric defects fixed below. Selected scales/sentinels implemented; all narrowed subtype ranges/units and application constraints are not comprehensively enforced. |
| §§3.11–3.12, 3.16–3.20: time, date, access, strings, scenes, datetime | corresponding codecs; existing unit/width tests; inventory | Format/field checks implemented. Full calendar/FB semantics are not proved. Datetime SRC ruling retained. Scene wire number is not a UI +1 number; Display need not be encoder input. |
| §§3.21–3.29 and unstructured types in §§4, 6–9 | enum/bitset/string/scene-info/V64 codecs; inventory; runtime caller tests | Format-level only for subtype enum/bitset tables. Reserved subtype codes and narrowed masks can still be preserved as raw values, rather than semantically approved. Variable strings retain the strict-NUL ruling. |
| §4: HVAC | implemented unstructured families; unsupported structured IDs; inventory | 31.101 has no codec. Structured HVAC IDs are explicitly unsupported; LTE/addressing and FB dependencies remain outside this package. |
| §5: load management | type/section inventory | Structured format/FB semantics not implemented. No support inferred from a chapter or a matching primitive field. |
| §§6–7: lighting / shutters | unstructured families; explicit unsupported structured IDs | RGB/RGBW and other lighting structures are not generically LTE-only. No structured codec added; actual Use/receiver restrictions still apply. |
| §§8–9: system / metering | implemented primitive families; explicit unsupported structured IDs | Compound masks, units, status, invalidity and companion-DP rules are not supplied by a primitive codec. |
| §10: weather | explicit unsupported numbered types | No weather-structure codec or application conformance claim. |
| Annex A / B | section/page inventory | Non-standard unnumbered HVAC status and legacy unnumbered DALI formats are not assigned invented IDs or codecs. |
| Annex C | section/page inventory | Informative change history, not a new support guarantee. |

All rows retain [Known Limitations §61](../KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-with-explicit-input-formats-and-disclosed-encoding-rulings).
The earlier [width](2026-10-05-dpt-format-widths.md) and
[numeric](2026-10-05-dpt-numeric-ranges.md) audits remain historical **scoped**
evidence; their prose is not a whole-document closure claim.

## GAP-DPT-FULL.1 — structured types were incorrectly labelled LTE-only (corrected)

§1.2 p13 reserves the LTE-only category for **structured HVAC subnumbers
100–499**, not all main numbers 200–299. The actual unsupported inventory has
47 structured HVAC IDs, 57 structured common IDs, 18 lighting IDs, one system
ID, two shutters IDs, 23 metering IDs, plus unstructured 31.101. In particular,
232.600 (§6.6.1 p182) and 251.600 (§6.18 p194) defeat the former blanket
rationale. RGB still has its single-receiver restriction (NOTE 27).[1]

KL-61's rationale is corrected without reopening the recorded unsupported
scope. Neither the 46-main-number historical master catalogue nor this
454-ID Standard inventory measures the user's installed-device coverage.
Any new codec priority requires a separate per-device need/use assessment.

## GAP-DPT-FULL.2 — V16 percent rounding emitted invalid data (fixed)

§3.9.1 p37 gives 8.010 a 0.01% resolution and reserves 7FFFh for invalid data.
An input of **327.665** passed the old engineering-range test, rounded to
32767 and emitted that sentinel. The encoder now checks the **quantized**
count as well and refuses sentinel-producing inputs with `OutOfRange`.[1]

Code: `encode_v16`. Regression:
`percent_v16_rounding_cannot_generate_the_invalid_sentinel` (both public
encoder APIs). Valid endpoints and ordinary in-range rounding remain pinned.
The correction does not silently clamp to the nearest valid value or introduce
an explicit invalid-data sending grammar.

## GAP-DPT-FULL.3 — F16 admitted out-of-format inputs by rounding (fixed)

§3.10 p39 prints the format range **−671088.64 through 670433.28**, independent
of narrower subtype ranges. The old encoder accepted **−671088.65** by rounding
it back to the minimum. It now checks the engineering input before quantization;
values outside either endpoint and nonfinite inputs are refused.[1]

Code: `encode_f16`. Regression:
`f16_inputs_outside_the_printed_format_range_are_not_rounded_back_in`.
This does not enforce, for example, 9.001's temperature lower bound or 9.004's
nonnegative lux range: those remain disclosed subtype-semantic boundaries.

## GAP-DPT-FULL.4 — scaled V32 admitted out-of-range flow by rounding (fixed)

§3.14.1 p43 defines signed V32 and 13.002's **0.0001 m³/h** resolution. Thus its
engineering bounds are −214748.3648 and 214748.3647 (derived from the signed
32-bit bounds and that resolution). The old encoder accepted **−214748.36481**
by rounding back into range. It now checks the input before rounding.[1]

Code: `encode_v32`. Regression:
`flow_rate_inputs_outside_the_scaled_v32_range_are_not_rounded_back_in`.
Both explicit and compatibility encoders retain legal endpoint encoding.

## GAP-DPT-FULL.5 — generic runtime writers allowed parameter-only DPTs (fixed)

Primary restrictions: 7.003/.004/.006 (§3.8.2 footnote 6 p35),
7.013 (§3.8.3 footnote 7 p36), 8.003/.004/.006 (§3.9.2 footnote a p37),
8.012 (§3.9.3 p38) and 20.022 (§3.21 p57). The time-period footnotes allow
an explicitly specified FB exception; a generic writer does not possess
that verification context. These **nine explicit subtypes** are therefore
refused before generic GroupValue writes, not removed from parameter or
diagnostic encoding.[1]

`validate_group_write_dpt` lives in the UI-independent core and is called by
HTTP and CLI bus writers for **explicit and project-resolved** DPTs, with
**explicit and legacy/omitted input formats**. API tests pin HTTP 400 and an
unchanged write history; CLI tests use dry-run and pin refusal before transport.
Core tests pin all nine restrictions, neighbouring allowed examples, bare-main
behaviour and continued parameter codec availability. Bare main types and
explicit raw telegram mode make no subtype claim; they are not a comprehensive
application-use safety validator.

The older time-period ruling's references are corrected to §§3.8.2/3.9.2 and
its wording now retains the FB exception. This is a bounded caller admission
check, not a second codec, LTE stack, blanket FB validator or hardware guarantee.

## GAP-DPT-FULL.6 — comprehensive subtype/application semantics remain open

All 305 in-scope IDs are conservatively classified **partial subtype semantics**,
not 305 fully conforming DPT implementations. Residue includes narrowed numeric
ranges, enum/bitset reserved values and polarity, unit/presentation vocabulary,
Use=FB constraints, companion datapoints, receiver policies, manufacturer
extensions and full calendar/context checks. Existing raw values/telegram data
are not replaced with guessed semantics. These are retained KL-61 boundaries;
the present inventory must not be used to claim they are resolved.

Structured support remains absent for 148 IDs and 31.101 remains absent too.
The implementation scope is unchanged. Future work should start with measured
installed-device relevance, authoritative subtype/FB evidence and one
independently sourced regression family at a time.

## GAP-DPT-FULL.7 — overview omission and ambiguous source tables (recorded)

249.600 appears in §6.16 p192 but not the overview. Its Format line gives a
four-octet label while the six numbered octets, U16/U16/U8/B8 layout and PDT
indicate six octets. The audit records the identity and ambiguity; it does not
implement a guessed width. Existing 19.001 SRC, 22.100 bit 8, 29 signed lower
bound and 5.005 unspecified scaling rulings remain explicit, rather than
being silently harmonized.[1]

## Verification and local delivery

Numeric RED: three intended assertion failures on the original codec, one
valid-input control passed. Runtime RED: two HTTP tests returned 200 instead
of 400; two CLI tests accepted the restricted explicit/project-resolved type.
These are retained negative evidence, not infrastructure/compile failures.
The first inventory run also failed: Display's `scene 0` was incorrectly used
as decimal write input. That **test-oracle error**, not a wire defect, was
corrected without changing scene encoding.

Focused GREEN: document inventory 4/4; semantic regression 5/5;
HTTP bus-write target 18/18; CLI bus-DPT target 12/12. Final integrated-root
Rust gate: **3418 passed, 0 failed, 178 ignored in 191 result blocks**; workspace
all-target Clippy `-D warnings`, CLI/server debug build and all five xtask audits
passed. Loopback-only execution, source/HEAD/index preservation, negative
attempt classification and the self-review receipt are recorded in
`.ai/logs/2026-10-07_codex_dpt-document-audit.md`. Fmt/whitespace checks passed.
Web/native UI and ignored/private/hardware tests are not part of this gate.
No independent review, full ETS compatibility, release acceptance, hardware
verification or publication is implied.

## Sources

[1] https://support.knx.org/hc/en-us/article_attachments/15392631105682 — KNX DPT-AS v02.02.01, 251 pages (local original audited)
