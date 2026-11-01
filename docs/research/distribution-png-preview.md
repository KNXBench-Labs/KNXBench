# Bounded distribution PNG previews

## Scope and evidence

OKX-04 uses the already-locked `png` 0.18.1 crate, not a general media framework.
This is KNXBench passive presentation policy, not an ETS rendering or manufacturer
schema claim. See the [owning contract](../contracts/distribution-inspection.md#distribution-resources-and-passive-png-presentation).

Primary API references:

- <https://docs.rs/png/0.18.1/png/struct.Decoder.html>
- <https://docs.rs/png/0.18.1/png/struct.Reader.html>
- <https://docs.rs/png/0.18.1/png/struct.Limits.html>

The crate's internal decoder limit does not include caller output allocation.
The inspector therefore admits encoded bytes, IHDR dimensions, pixel count,
conservative RGBA8 decoded bytes, internal decoder bytes, chunk count, derived
output and cumulative work separately. Text and ICC expansion are disabled before
reading metadata. A constant-memory full chunk/CRC walk rejects APNG and malformed
extents. Both `next_frame` and `finish` must succeed; a first-frame/header success
alone does not authorize a preview. Owned re-encoding uses expanded/stripped pixel
output only, without original text, profile or other metadata.

The implementation tests use authored synthetic PNG/ZIP/XML controls. They check
actual decoded red RGBA pixels after re-encoding, original/derived hashes, exact
inclusive budgets, corrupt CRC/zlib, missing/trailing data, APNG and oversized
headers. The real CLI test requires a derived image, explicit corrupt-image and
metadata-only ZIP fallbacks, escaped hostile strings, inert Script bodies and
unchanged source/profile files. Those are local product regressions, not browser
or whole-corpus proof. A separate offline headless-Chromium proof decoded both
derived PNGs with no external request; all 37 OpenKNX releases still inspect.

Nested help ZIPs reuse `BaggagePayload::measure` and its checked metadata validator.
They are not decompressed, recursively explored, downloaded or rendered. Selected
payloads are read by checked ordinal once after XML composition, not resolved by
filename reopening, suffix search or decoded baggage IDs.
