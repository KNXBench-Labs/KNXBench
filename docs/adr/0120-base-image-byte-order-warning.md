# ADR 0120: A named warning when a big-endian program's base image looks little-endian

Date: 2026-10-13
Status: Accepted implementation decision (owner decision 2026-10-13)
Session: KL-170 follow-up

## Context

The KL-170 research ([parameter byte order](../research/parameter-byte-order.md))
found that many ABB and Busch-Jaeger `0701h` programs declare
`ParameterByteOrder="BigEndian"` or nothing (the schema default, also
`BigEndian`), yet their own base images hold multi-octet defaults low octet
first. KNXBench downloads those programs and writes multi-octet values high
octet first, as declared. Whether those devices read them that way is not known:
no such device has been tested, and the base image is product data, not a
device. A download of such a program is therefore allowed, but its user was not
told that the product data contradicts the declaration
([§170 related risk](../KNOWN_LIMITATIONS.md#170-little-endian-parameter-byte-order-is-refused-for-download-images)).
The owner decided on 2026-10-13 that such downloads stay allowed and carry a
named warning.

## Decision

1. **Same planning, base image instead of a device.**
   `knx_productdb::byte_order_probe::base_image_order` plans the probe of
   ADR-0119 at product defaults (the request's values, links and flag
   overrides are not used) and reads each planned field from its segment's
   `Data` instead of a device. The ADR-0119 classifier gives the tally.
2. **A generic rule, no manufacturer list.**
   `knx_app::device_download::byte_order_warnings` warns with rule
   `base-image-low-octet-first` when the program declares `BigEndian` or
   nothing and **every** discriminating default is held low octet first
   (`Verdict::Consistent(LowFirst)`). High-first, contradictory and silent
   base images give no warning. No manufacturer or mask is named in code.
3. **A check that cannot run is named.** If the base image cannot be checked,
   the plan carries `base-image-order-unchecked` with the reason instead of
   passing over it.
4. **A warning, not a rule.** `PreparedDownload::warnings` is shown before the
   segments in `knx device download` (`WARNING rule: detail (reference)`) and
   as `warnings` in the server's plan response. It does not change the image,
   the support level, the acknowledgement or the write. Unlike an `Inference`
   (ADR-0086) it is not a rule the download applies.

## Alternatives considered

- **Refuse these downloads.** Rejected by the owner: the evidence is a
  suspicion, the programs are widely used, and a refusal would block downloads
  that may be correct.
- **Write them low octet first.** Rejected: that is the unverified encoder
  ADR-0119 rules out, applied against the program's own declaration.
- **Match ABB/Busch-Jaeger `0701h` by name.** Rejected: manufacturer
  knowledge does not belong in application logic, and the generic rule finds
  the same programs (below).
- **Fold it into the support level as an inference.** Rejected: nothing is
  inferred or applied; the download writes exactly what the program declares.

## Consequences

- Measured on 929 packages (105 local, 824 public ones under 6 MB): of the
  181 programs a memory download plans at product defaults, 32 get the
  warning. All are ABB (23) or Busch-Jaeger (9) `0701h` programs; none of the
  93 MDT, 10 Siemens or 11 other programs gets it, including the one
  hardware-verified MDT program. `base-image-order-unchecked` did not occur
  for any memory-loaded program. Receipt:
  [kl170-base-image-warning-2026-10-13.json](../evidence/kl170-base-image-warning-2026-10-13.json).
- The extra check is a second activation at defaults: 11.8 s for all 519
  checks of the measurement, at most 2.0 s for the largest program (release
  build), on top of the image build. Not optimised; nothing measured asks for
  it yet.
- The web download tab does not show `warnings` yet: it showed no inferences
  either, and the web lock was held when this was built. The server field is
  in place for it.
- A tester's confirmation or refutation on hardware can later settle each
  program; the warning names KNOWN_LIMITATIONS §170 so the evidence path is
  findable.
