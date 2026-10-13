# 19.11 Fields across an octet boundary, and why module instances stay refused (2026-09-29)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="1911-fields-across-an-octet-boundary-and-why-module-instances-stay-refused-2026-09-29"></a>
<!-- BEGIN preserved record-2-020 -->
### 19.11 Fields across an octet boundary, and why module instances stay refused (2026-09-29)

Two candidates for the next coverage step were checked against the direct
PDFs and the product files: parameter fields across an octet boundary (the
4 programs §19.10 left refused) and module instances (51
`parameter-evaluation` refusals, every first diagnostic inside a
`ModuleDef`).

**Fields across an octet boundary — written.**

- **[D]** *Project Schema23* §1.1.3.17 `BitOffset_t`: *"The bit offset is
  the distance of the most significant bit of the parameter from the most
  significant bit of the first octet in memory"*, 0–7. The schema says
  where the field *starts*, not how it continues.
- **[D]** *Configuration Procedures* `03_05_03` v02.01.01 §8.5.4
  (pp. 197–198, E-Mode parameter blocks): *"The position of the parameters
  inside the parameter block are not restricted to the boundary of the
  parameters itself"*. Its Example 24/25 figures number one two-octet
  block's bit offsets `0…7` through octet 0 and `8…15` through octet 1,
  and place an 8-bit parameter at `8…15`.
- **[D]** *Resources* `03_05_01` v01.10.01 §4.18.5.2.5 `PID_EXT_GRPOBJREFERENCE` (p. 268): *"Bit
  offset shall start from "left" / MSB"*; bit 0 of the third octet of a
  `U16B8` is *"Bit Offset = 23"*. *Resources* §4.19.2.1 (p. 278) shows
  E-Mode parameter blocks numbered the same way.
- So the Standard numbers bits MSB-first on through consecutive octets. A
  `Memory` field of `SizeInBit` bits at `Offset`/`BitOffset` is written as
  that bit string: its most significant bit at `BitOffset` in octet
  `Offset`, the rest following into the next octet's MSB. Whole-octet and
  in-octet fields are the special cases of the same rule, and the
  existing tests (among them the live `A-0027-15-0BAC` image) still pass.
- **Not shown anywhere:** none of these figures has a *parameter* that
  crosses a boundary (Example 24's 4-bit field sits at `2…5`), and no
  device has read one back. The rule is the Standard's numbering applied,
  and the plans it opens are `Untested`.
- **[V] Corpus effect.** 77 crossing union members in the `070nh`
  programs' own `Static` (all MDT `M-0083`; 20 more inside `ModuleDef`s).
  The 4 programs `A-008A-25`, `A-008A-28`, `A-008B-25`, `A-008B-28` were
  refused only at `UP-290` (6 bits at octet 1600 bit 5) and now plan. No
  other program's outcome changed (refusal lists diffed before/after).
- Refused as before: more than 64 bits, zero width, a bit offset above 7,
  a union member offset inside a union that itself starts mid-octet, and
  any overlap.

**Module instances — measured, not written.**

- **[D]** What the PDFs say about arguments: *Project Schema23*
  §1.1.2.38 `ModuleDefArgType_t` (`Numeric`, `Text`, `AllocatorRef`) and
  `Value_t`'s one line on `TypeAllocatorRefId`. `BaseOffset`,
  `BaseNumber`, `Allocates` and `NumericArg` are **not defined in any
  direct PDF**: a full scan of `sources/` for them finds only four schema
  lines on the argument type and the project-side `ModuleInstance`.
- **[V]** The products use them uniformly: every `ModuleDef`'s `Memory`
  (7594 in the `070nh` programs) carries `BaseOffset="…_A-1"`, the
  `ParamOffsBase` argument, and every instantiating `Module` binds it with
  a `NumericArg` (e.g. `A-0009-32-094E`: 12 modules at `72`, `144`, `216`,
  `288`, `Allocates="72"`). The obvious reading is *octet = argument value
  + `Memory/@Offset`*, and object *number = `ObjNumberBase` +
  `ComObject/@Number`*.
- **[V] That reading does not rebuild the base images.** Checked the way
  §19.9 checked value encodings — a field's product default against the
  product's own `Data` at the computed place:
  - program-own parameters: 1046 match, 392 differ (73 %);
  - module parameters at *argument + offset*: 1604 match, 1447 differ
    (53 %); shifted by one octet either way, 19 % and 34 %; read at
    *another instance's* base, **52 %** — no better than chance between
    instances, because most instances share their defaults;
  - only 26 of 56 modular programs have zero contradictions where an
    instance's range is its own; for 13 873 octets two instances'
    computed fields collide.
  - group objects: `ObjNumberBase + Number` matched the GrOT's descriptor
    for 5033 and differed for 360 objects.
- The placement rule for parameters is therefore **not settled** by the
  products, and no PDF states it. Writing module parameters on it would
  risk the octets of another instance or of the program itself. Module
  instances stay refused (`modules` / `parameter-evaluation`), and the
  decisive evidence would be a device read-back of a modular product.
  The group-object side alone would not open a program: its parameters
  are the reason it is modular.

: AppImage interface discovery and line-relative addresses (2026-09-28)



<!-- END preserved record-2-020 -->
