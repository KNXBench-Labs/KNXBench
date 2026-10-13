# 19.2 Assembling a download image from the product file (2026-09-28)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="192-assembling-a-download-image-from-the-product-file-2026-09-28"></a>
<!-- BEGIN preserved record-2-011 -->
### 19.2 Assembling a download image from the product file (2026-09-28)

`knx_productdb::image::build_download_image` produces every segment image
of a program from the product file (ADR-0044), the chosen parameter values
and the group address links. What each rule rests on:

- **`[D]` What changes.** The product's default image, modified in its
  group objects, group addresses and parameters: the Cookbook *Load
  Controls* (`02_03_01` v01.00.02, pp. 6–7). Nothing else is written.
- **`[D]` Bit placement.** `BitOffset` is the distance of the value's most
  significant bit from that of the first octet (*Project Schema23*
  §1.1.3.17, pp. 29–30).
- **`[V]` Union members.** A member lies at the union's `Memory` plus its
  own `@Offset`/`@BitOffset`. None of the PDFs read covers `Union`: Schema23
  describes the project side only (`ComObjectInstanceRef_t` and so on), not
  the product's `Static`.
  - `A-0027-15-0BAC` has six members at `Offset=1`, `BitOffset` 5–7.
  - The rule is used only for unions that start on an octet boundary; the
    corpus has 936 unions that start mid-octet, and those are refused.
- **`[V]`/`[A]` Unmatched `choose`.** A `choose` whose value is legal for
  its type but matched by no `when` activates nothing. This is the
  evaluator's existing `[A]` reading (§4.3). The device confirms it:
  evaluating the MDT tree for the device's values gives ten such `choose`s
  (for example `P-3 = 0`, `P-1007 = 1`), and the result matches the device
  octet for octet. Every other diagnostic refuses the image, and so does a
  value the type does not allow.
- **`[A]` Priority.** Schema23's `ComObjectPriority_t` is `Low`/`High`/
  `Alert` (§1.1.2.4), while *Resources* §4.18.3.1.2.1 has `System`/
  `Urgent`/`Normal`/`Low`. No PDF maps one onto the other.
  - The corpus uses `Low` 270 times, `High` 59 and `Alert` once.
  - An absent priority and `Low` are written as `Low` (`11b`), which is
    what the device holds for MDT's absent priorities.
  - `High` and `Alert` are refused.
- **`[A]` `ReadOnInitFlag`.** It has no bit in the Easy-3 config octet
  (*Resources* §4.18.3.1.2.1), so an enabled one is refused, not dropped.
- **`[A]` `Mask`.** The builder refuses a change to any octet whose mask
  octet is not `FFh`, except the address table's individual-address slot.
  The image still carries the mask, so the writer must skip those octets.
  - Corpus: 5,291 `00h` and 145,896 `FFh` mask octets; no other value.

**`[V]` Acceptance, against the device's read-back.** For the device's own
configuration (five non-default values, four links):

- `AS-4400` is equal in all 394 octets;
- the address table's 11 octets and the association table's 9 are equal.

For option C (button 1 toggles `2/0/53`, button 2 inactive), exactly eight
octets of `AS-4400` differ from the device: objects 1 and 18, the button
1/2 function and subfunction, and three parameters behind them. The address
table becomes `02 1143 1035` and the association table `01 01 00`.

The device keeps stale octets behind both tables (`AS-4000`+11/12,
`AS-4201`+10). These differ from the base data. A download writes the
base data there, which a table's length octet makes irrelevant.


<!-- END preserved record-2-011 -->
