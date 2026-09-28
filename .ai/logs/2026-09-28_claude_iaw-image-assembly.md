# 2026-09-28 — Claude — iaw: download image assembly

**Track:** commissioning `1.1.67` (MDT `A-0027-15-0BAC`, mask `0701h`),
option C: button 1 toggles `2/0/53`, button 2 inactive, everything else
product defaults. **Offline only; nothing written to the device.**

## What was built

- `crates/knx-productdb/src/code.rs`: `ProgramCode::parameters`, which
  gives each `Parameter`'s placement verbatim from the blob:
  - `Memory`;
  - `UnionMember { union, offset, bit_offset }`;
  - `Unmodelled` (for example `Property`), by name and with its
    attributes.

  The database row holds only the union's placement, so a union member's
  own offset was invisible before this.
- `crates/knx-productdb/src/image.rs`: `build_download_image(conn,
  &ImageRequest)` produces a `DownloadImage`, in six steps:
  1. validate the requested values against their types;
  2. `resolve_values` and `evaluate` the `Dynamic` tree;
  3. write every active parameter into the base `Data` via
     `knx_core::commissioning::parameter_image`;
  4. write the GrOT *Easy 3* with the active objects
     (`group_object_table`);
  5. write the GrAT/GrOAT (`group_tables`) at their placements;
  6. check the masks.

  The layer stays clean: `knx-core` knows no XML or SQL, and
  `knx-productdb` already depended on `knx-core`.

## Decisions and why

| Rule | Basis |
|---|---|
| Change only group objects, addresses and parameters | `[D]` Load Controls 02_03_01 pp. 6–7 |
| `BitOffset` counted from the most significant bit | `[D]` Schema23 §1.1.3.17 |
| Union member = union `Memory` + own `Offset`/`BitOffset` | `[V]` device (no PDF covers `Union`); unions that start mid-octet are refused |
| A legal value no `when` covers activates nothing; every other diagnostic refuses | `[V]` 10 such `choose`s in the device's configuration and 0 diffs; illegal values are refused |
| Priority absent or `Low` → `11b`; `High`/`Alert` refused | `[A]` Schema23 and *Resources* name the classes differently |
| An enabled `ReadOnInitFlag` is refused | `[A]` it has no bit in Easy 3 |
| A change to an octet whose mask is not `FFh` is refused, except the IA slot | `[A]`; the corpus has only `00`/`FF` mask octets |

What this does not cover is refused by name and not guessed
(KNOWN_LIMITATIONS §7): modules, `Property` placements, other parameter
types, and a parameter that reaches into the GrOT.

## Evidence

- Corpus test `the_image_rebuilds_the_devices_parameter_segment_octet_for_octet`:
  - `AS-4400` is equal to the device's read-back in all 394 octets;
  - GrAT (11 octets) and GrOAT (9 octets) are equal.
- Corpus test `option_c_…`: exactly eight `AS-4400` octets change, all
  predicted by the Python reference model (`scratch/iaw/mdt/mdt_env.py`).
  - GrAT becomes `02 1143 1035`.
  - GrOAT becomes `01 01 00`.
- 24 unit tests (`image::`), 26 (`code::`); 20/20 mutants caught.
- The review found a real gap. The GrOT overlap check only tested where a
  field *starts*, so a multi-octet field that begins before the table and
  runs into it slipped through. It is now a proper interval check, and a
  test catches the old variant.
- Documentation in the reports was corrected, not taken over. The first
  draft had "28 code tests / 22 mutants / 1 caught only by the corpus";
  the real figures were 26 / 18 / 0.

## Open, in order

1. Load-procedure executor over `LoadStep`, end to end against the
   simulator. It skips masked octets when writing, and refuses an
   `Unmodelled` step by name.
2. Enable `WriteScope::Download` for hardware. Then show the user the
   exact write sequence and get a new "go".
