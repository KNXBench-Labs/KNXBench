# 19.9 The `procedure-style` refusals, and signed and text parameter values (2026-09-29)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="199-the-procedure-style-refusals-and-signed-and-text-parameter-values-2026-09-29"></a>
<!-- BEGIN preserved record-2-018 -->
### 19.9 The `procedure-style` refusals, and signed and text parameter values (2026-09-29)

Question: can the 41 programs refused as `procedure-style` be planned, and
what else keeps `070nh` programs from planning? Method: the direct PDFs
under `knx-spec-kb/sources/` (no extracted text), the 103 local `.knxprod`
packages, and throwaway scripts that compare each parameter's `Value` with
the octets its product's own base image holds at its `Memory` placement.

- **[V] `procedure-style` was the wrong name for all 41.** Every
  `MergedProcedure`/`DefaultProcedure` program in the corpus has a mask the
  memory planner never handles: `MV-07B0` 27, `MV-0912` 7, `MV-0012` 2,
  `MV-091A` 2, `MV-2920` 2, `MV-0001` 1. None is a `070nh` program. A new
  load procedure style would not have planned one of them; their mask
  would still refuse them. `plan_memory_download` now asks the mask first,
  so they count as `not-memory-mapped` (65, with the 24 `MV-2705`
  `ProductProcedure` programs that already did), which is what an operator
  can act on. `procedure-style` remains for a `070nh` program with another
  style; the corpus has none.
- **[V] What the PDFs say about parameter values.** *Project Schema23*
  §1.1.3.19 (`Value_t`) fixes how a value is **spelled in the XML**: a
  `TypeNumber` as a decimal string, a `TypeFloat` as C#'s `"E15"` notation
  (`-?\d\.\d{15}E[+-]\d{3}`), a `TypeText` as the text itself. A scan of
  every PDF under `sources/` for `signedInt`, `TypeText`,
  `TextParameterEncoding` and `ParameterByteOrder` found only Schema23 and
  the Opternus cookbook's `unsignedInt` example. **No PDF says how a signed,
  float or text parameter is laid out in device memory.**
- **[V] What the products' own base images show** (`070nh` programs only;
  a base image need not hold the default, so only a match is evidence and
  a mismatch against a zero field is none):
  - *Signed `TypeNumber`.* 213 fields (135 of 8 bits, 78 of 16) hold their
    non-negative default high octet first; none holds it low octet first.
    36 16-bit fields hold another non-zero value in either order (e.g. `40`
    for a default of `20`), so the base is not always the default. The one
    negative default sits over a zero field. At or above zero every sign
    representation gives the same bits, so such a value is **written**; a
    negative one is **refused**, because nothing shows its form.
  - *`TypeText`.* 262 fields hold their default as its characters' octets,
    first character first, zero-filled to the field; 8 of them fill the
    field exactly, with no terminator. The six non-ASCII defaults
    (`"Zurück"`) are no exact match (the base holds a leading space), but
    hold `ü` as `FCh`, the ISO-8859-1/-15 octet, not UTF-8's `C3h BCh`.
    Of the `070nh` programs, `MV-0705` ones declare `TextParameterEncoding`
    `iso-8859-1` (12) or `iso-8859-15` (12); `utf-8` appears only on
    `MV-07B0` (3). Text is now
    **written** in the declared one of the two; without a declaration only
    ASCII, which both share; `utf-8` or another name is refused.
  - *`TypeFloat`.* 48 `DPT 9` fields match that encoding, every one at
    zero, which proves nothing about the format. The 15 non-zero ones
    contradict it: a default of `500` sits as `F4h 01h`, the 16-bit
    integer 500 low octet first, not DPT 9's `2E1Ah`. The 16
    `IEEE-754 Single` fields are all zero. Floats stay **refused**, now for
    their type (`image-structure`) instead of as "not an unsigned number"
    (`parameter-value`).
  - *`ParameterByteOrder`.* Declared by 2 `MV-0705` programs, both
    `BigEndian`. Any other declared order is now refused, since
    `parameter_image` writes high octet first.
- **[V] Result** (`crates/knx-app/tests/download_coverage_corpus.rs`,
  release build, 125 s): 246 programs, **1 verified, 69 untested** (was
  54), 176 unsupported: `not-memory-mapped` 65, `parameter-evaluation` 59,
  `image-structure` 41 (19 `TypeFloat`, 10 `ReadOnInitFlag` enabled, 9
  placed by `Property`, 3 priority `High`), `parameter-value` 9 (6 refs of
  one parameter with different values, 3 values outside their
  enumeration), `unmodelled-step` 2 (`LdCtrlTaskCtrl1`). Within `MV-0701`
  35/40 plan, within `MV-0705` 35/141. The 15 new plans are **untested**:
  no hardware has seen a signed or text parameter written by KNXBench.
- **[A] Not concluded.** A negative signed value is very likely two's
  complement (every ETS-era device CPU is), but "likely" is not a source;
  it waits for a product image or a device that shows one. Same for
  floats: the `F4h 01h` example suggests some products store a float
  parameter as a scaled integer, which is a manufacturer choice the
  product data does not declare.


<!-- END preserved record-2-018 -->
