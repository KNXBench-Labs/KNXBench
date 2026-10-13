# 19.1 Download data in the product file, and what the PDFs say about it (2026-09-28)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="191-download-data-in-the-product-file-and-what-the-pdfs-say-about-it-2026-09-28"></a>
<!-- BEGIN preserved record-2-010 -->
### 19.1 Download data in the product file, and what the PDFs say about it (2026-09-28)

Sources: the source PDFs under `knx-spec-kb/sources/` only, read directly.

**`[D]` What a tool writes.** The KNX Cookbook *Load Controls* (`02_03_01`
v01.00.02, pp. 6–7, Figure 4) describes the tool's side of a download. The
product ships a *"default memory image"*, and the tool *"modifies the
default image according to the ETS project settings, being: 1) group
objects 2) group addresses 3) device parameters"*. The load controls then
drive the management procedures. Configuration Procedures (`03_05_03`
v02.01.01 §3.9.3.2, pp. 71–72) maps `LdCtrlConnect`, `LdCtrlUnload`,
`LdCtrlLoad`, `LdCtrlLoadCompleted`, `LdCtrlRestart` and `LdCtrlCompareProp`
onto `DM_*` procedures for the System B mask.

- `LdCtrlConnect` includes `DM_Authorize` with the *project* key.
- The cookbook's worked ADM1 example (pp. 10–12) pairs an `AbsSegment`
  record for each resource with a `TaskSegment` record, between the
  resource's `Load` and `LoadCompleted`.

**`[D]` The allocation record's second field is a length.** MP (`03_05_02`
v02.01.02 §3.31.2, pp. 135–136) writes it as `EEEE - SSSS + 1`. The
cookbook's ADM1 records carry the *end* address (`4000`…`41FE` for a
`01FF`-octet segment). That is ADM1's storage format, not the memory-mapped
record `load_control_memory` builds, which follows MP. The product's
`LdCtrlAbsSegment/@Size` is the length (`Size="513"` for `AS-4000`, whose
`Data` decodes to 513 octets).

**`[V]` Where the data lives.** The following was measured over the 310
application-program files in the private corpus. All of it is in
`ApplicationProgram/Static`:

- `Code/AbsoluteSegment`: 1,492 segments.
  - 980 have base64 `Data` and 305 have `Mask`; every decoded length
    equals `Size`.
  - The only child elements are `Data` and `Mask`.
  - Other attributes: `MemoryType` (605), `UserMemory` (134), `Name` (4).
- `AddressTable`, `AssociationTable`, `ComObjectTable`: placement through
  `CodeSegment`, `Offset` and `MaxEntries`.
- `LoadProcedures/LoadProcedure`:
  - 303 files have them; 379 procedures in total, 116 of which carry a
    `MergeId`.
  - Steps come in 19 `LdCtrl*` kinds. The only child element any step ever
    has is `OnError` (11 times).
  - 7 files put a `choose` inside a procedure.
- Load procedure styles: `ProductProcedure` 263, `MergedProcedure` 40,
  `DefaultProcedure` 7.

`knx_productdb::code` (ADR-0044) reads all of this back from the stored
blob. It parses all 310 files with 0 errors. 59 contain at least one step
it does not model, which a hardware write must refuse by name.

**`[A]` `Mask` has no definition in any PDF.** No KNX PDF read for this
section defines `AbsoluteSegment/Mask`: *Project Schema23* and Volumes 2, 3
and 8. In `A-0027-15-0BAC` the only `Mask` covers `AS-4000` and marks
exactly octets 1–2, which are `00h` in every other position. Those octets
are where the `[V]` read-back of the real device holds its own individual
address (`11 43` = `1.1.67`). Reading the mask as *"octets the tool must not
overwrite"* fits that evidence, and a download treats it that way. It is
still an assumption and is labelled as one.

**`[V]` Correction: MDT does not declare `ParameterByteOrder`.** §19's claim
that mask-`0701h` applications declare `ParameterByteOrder="BigEndian"` was
measured on *project exports*, where ETS materialises every `Options`
attribute. The MDT product file's own `Static/Options` has exactly one
attribute, `LegacyAllowPartialDownloadIfAp2Mismatch="true"`. Only 26 of 310
product files declare `ParameterByteOrder` at all, all `BigEndian`. So
high-octet-first for MDT is **not** declared by the product. It rests on
the project exports' materialised default and on the device read-back of
`1.1.67` (`AS-4400`+260…267). The first two readings are default values,
the next two are the ones the device's shutter configuration selects:

- `P-5002` (default 50) reads `00 32`;
- `P-5003` (default 400) reads `01 90`, where the base data holds `0B B8`
  (3000);
- the union selector at +264 reads `00 02`;
- +266 reads `00 00`.

A little-endian reading would give 12800, 36865 and 512, none of which is
a value these parameters allow. `parameter_image`'s module documentation
carries the same correction.


<!-- END preserved record-2-010 -->
