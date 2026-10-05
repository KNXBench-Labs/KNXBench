# DPT format-width audit (AR09, 2026-10-05)

Audit of `crates/knx-core/src/dpt/codec.rs` against the KNX Standard v3.0.0
documents in the local `knx-spec-kb` corpus:

- 03_07_02 Datapoint Types v02.02.01 AS ("DPT-AS"), `Format:` line of each
  main type's section (§§3.1–3.28, 8.3–8.5).
- 03_03_07 Application Layer v02.01.01 AS ("AL-AS"), A_GroupValue_Write PDU,
  Figures 7 and 8 (page 16/17 of 191).
- 03_05_01 Resources v01.10.01 AS, §4.18.6.2.4.1.4 Table 88 "Value Field
  Types" (page 272).

Scope: payload width only, for one representative subtype per main type 1–30
(two for main type 6: V8 and B5N3), excluding the variable-length 24 and 28.
Value ranges, special values and per-subtype semantics are not part of this
pass; KL-61 stays open for them.

Method: `crates/knx-core/tests/dpt_spec_width_audit.rs` holds the widths read
from the spec, not from the codec. For every row it checks that the spec
width is accepted, that one extra octet is refused with `WrongLength` naming
exactly the spec width, and that multi-octet formats refuse one octet less.

## GAP-AR09.1 — 17.001 DPT_SceneNumber was sent in the 6-bit optimised form (fixed)

**Spec.** DPT-AS §3.18 gives the format as "1 octet: r2U6". Resources Table 88
distinguishes object sizes of 1–7 bit from "1 octet" (Value Field Type 7). AL-AS
Figure 8 uses the optimised A_GroupValue_Write PDU, with the data in the APCI
octet, only for values of 6 bits or less; larger values go in their own data
octet (Figure 7).

**Code before.** `encode_scene` returned `GroupValue::Short(n)`, and
`decode_scene` used `require_short(payload, dpt, 6)`. The comment justified
this with the value's six *significant* bits. A write from KNXBench to a scene
number object therefore went out in the optimised form, while the receiving
device has a 1-octet object. A width error was also reported as 6 bit
instead of 8. Decoding still accepted a real 1-octet telegram.

**Fix.** `encode_scene` returns one data octet. `decode_scene` requires
exactly one octet, refuses the inline form with `WrongLength`, and treats a
set reserved bit (bits 7 and 6) as `InvalidData`, as 18.001 already did.
The earlier tests that pinned the inline form were rewritten rather than
kept beside the new ones.

## Verified without finding

All other 28 rows match the spec width: 1 (1 bit), 2 (2), 3 (4), 4–6 (8),
6.020 (8), 7–9 (16), 10–11 (24), 12–15 (32), 16 (112), 18 (8), 19 (64),
20–21 (8), 22 (16), 23 (2), 25–26 (8), 27 (32), 29 (64), 30 (24).
