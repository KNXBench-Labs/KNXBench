# 19.18 Object priority, ReadOnInit and floats under ADR-0086 (2026-10-07)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="1918-object-priority-readoninit-and-floats-under-adr-0086-2026-10-07"></a>
<!-- BEGIN preserved record-2-027 -->
### 19.18 Object priority, ReadOnInit and floats under ADR-0086 (2026-10-07)

The three image-structure refusals of §19.10, re-examined with the
specification, the product data and the projects as evidence.

- **`Priority="Alert"` → urgent (`10b`), an inference.** A census of the
  corpus's `070nh` programs (198 with a `ComObjectTable` in a segment with
  `Data`, 17 731 objects) compared each `ComObject`'s `Priority` with the
  priority bits of its descriptor's config octet in the product's own base
  image: `Alert` carries `10b` in 284 of 285 objects (one `11b`); objects
  without a priority carry `11b` in 15 221 of 15 477, `Low` in 1 853 of
  1 935. *Resources* §4.18.3.1.2.1 names `10b` urgent. The image builder
  writes `Alert` as urgent and discloses `image::ALERT_IS_URGENT`.
- **`Priority="High"` stays refused.** The same census gives `00b` 17
  times, `10b` 4 times and `11b` 13 times, and never `01b` (normal): the
  product data contradicts itself, which ADR-0086 decision 4 refuses.
- **`ReadOnInitFlag="Enabled"` is documented as unrepresentable, and the
  loss is disclosed.** `[D]` *Resources* §4.18.6.2.4.1.3, NOTE 85: *"'Value
  Read on Initialisation' is a new feature introduced with System B."* The
  System B descriptor has it as bit 13; the `070nh` (Easy-3/Type-2) config
  octet has the segment selector in that position and no read-on-init bit
  (§19.2). So the image is written exactly as with the flag disabled, and
  `image::READ_ON_INIT_NOT_ON_070N` tells the user the device will not read
  the object after a reset. The project's own `ComObjectInstanceRef`
  `ReadOnInitFlag` now reaches the image request (`FlagOverrides::
  read_on_init`) and decides, over the product's, whether that is said.
  **[V] Corpus effect:** the 10 programs this refused now plan; the corpus
  at its defaults plans 1 verified + 89 untested (was 79), `image-structure`
  41 → 31. No `Alert` object was the first refusal of a program, so that
  inference changes no count today.
- **Floats stay refused.** The product declares the encoding (`TypeFloat
  @Encoding`: `DPT 9` or `IEEE-754 Single`), but the base images contradict
  a DPT 9 reading where they hold a non-zero default (`500` as `F4h 01h`, a
  little-endian integer, §19.9); 57 non-zero `DPT 9` and 16 non-zero
  `IEEE-754 Single` defaults match neither encoding. With
  contradicting sources and no working solution shown, ADR-0086 keeps the
  refusal.


<!-- END preserved record-2-027 -->
