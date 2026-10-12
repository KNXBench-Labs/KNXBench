# ADR 0119: A read-only byte-order probe, never an encoder

Date: 2026-10-13
Status: Accepted implementation decision
Session: KL-170 hardware-free package

## Context

KL-170: `build_download_image` refuses every program whose
`Options/@ParameterByteOrder` is not `BigEndian`. The research of
2026-10-13 ([parameter byte order](../research/parameter-byte-order.md))
found no KNX-published per-type definition of the attribute. It found strong
circumstantial evidence that `LittleEndian` means "whole-octet numbers, least
significant octet first": the base images of 370 LittleEndian public programs
hold their multi-octet defaults that way, and only one Siemens System B family
(2 programs) the other way; one manufacturer re-declared 32 identical layouts
from `BigEndian` to `LittleEndian`; and two independent third-party tools
encode it so. The same research shows why that is not enough: the same
manufacturers' BigEndian-declared and undeclared programs show the same
low-octet-first base images, so base images follow a firmware convention and
do not prove what ETS writes. No source covers fields across an octet
boundary, floats, texts or negative numbers in such a program. Only a device
that ETS programmed can settle what ETS writes, and none is available to the
project.

## Decision

KNXBench gets a **probe**, not an encoder.

1. **Read only.** `knx device byte-order-probe` reads, through a
   `ManagementSession::read_only` session with no access key, exactly the
   octets of the fields it classifies (`knx_net`'s `read_memory_runs`). It
   checks mask and manufacturer before the first memory read and reads no
   load state. It has no write path, takes no confirmation phrase, and is
   bounded to `MAX_PROBE_OCTETS` (4096).
2. **Same plan as a download.** The fields come from
   `knx_productdb::byte_order_probe::plan_probe`, which shares the image
   builder's activation (`image::activate`), placement (`field_of`), union
   rule and value checks. It does not refuse a program for a declared order or
   for one field it cannot read; every parameter it leaves out is named with
   its reason.
3. **Two hypotheses, where defined.** `knx_core::commissioning::byte_order_evidence`
   classifies whole-octet fields of at least 16 bits as high- or low-octet-first,
   palindromic values as non-discriminating, and fields inside one octet as
   controls (MSB-first per Schema23 `BitOffset_t`). A field across an octet
   boundary that is not whole octets is not classified: no arrangement is
   defined for it.
4. **Evidence, not a rule.** The classifier is decode-only and is not
   reachable from `parameter_image` or the image builder. A probe result
   changes nothing a download writes. Lifting KL-170 stays a separate,
   reviewed decision that cites probe output from ETS-programmed devices.

## Alternatives considered

- **Implement little-endian writing now** for whole-octet numbers, as the two
  third-party tools do. Rejected: it would write manufacturer-specific values
  on corpus inference alone, the "octet swap on suspicion" KL-170 forbids,
  and the corpus itself contradicts the inference for some programs.
- **Implement it behind a flag or an `--accept-untested` phrase.** Rejected
  for now: an untested download is accepted for documented encodings whose
  device behaviour is unconfirmed, not for an encoding that has no source. It
  would also put an unverified encoder on the write path.
- **Read whole segments.** Rejected: a real parameter segment reaches 24 KB
  (about 2000 bus reads). The fields' octets are enough.
- **Compare a hypothetical little-endian image with `knx device compare`.**
  Rejected: it needs the whole image, which other refusals (floats, negative
  values) block for many of these programs, and it would put a hypothetical
  encoder next to the real one.

## Consequences

- Anyone with an ETS-programmed device can produce the missing evidence in one
  read-only command, for any declared order. It equally checks the
  BigEndian-declared legacy programs whose base images look little-endian.
- The probe trusts that ETS last programmed the device with the project's
  values. Controls and "neither" counts show how far that holds; the verdict
  is reported next to them, never instead of them.
- What the image builder's evaluation cannot decide, the probe cannot either:
  measured on 42 public LittleEndian `0701h` programs at product defaults, 36
  plan (at most 202 octets) and 6 are refused because a `choose` is controlled
  by a `TypeRestriction Base="BinaryValue"` parameter that no `when` covers.
  The refusal names the parameter; with a project's own values the branch may
  be decided.
- Two small helpers of the verified compare path were factored out and are
  shared (`check_identity`, `read_region`); their tests run unchanged. The
  simulator gains `short_memory_answers`, a knob that is no device behaviour
  and exists only to prove that a short memory answer is refused, by the probe
  and by every backup read alike (no earlier test covered that refusal).
- No server, web or MCP surface: the probe is an engineering tool, not an
  installer workflow. A UI would need its own decision.
