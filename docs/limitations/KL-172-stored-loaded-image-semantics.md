# Stored loaded-image semantics

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-172)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 172. A device's stored loaded image is kept, not decoded

**Public-source research completed 2026-10-10; boundary unchanged.**
[Inspected sources and missing contract](../research/schema23-offline-boundaries.md)
record the public evidence for KL-172. No sufficient manufacturer encoding or
loaded-payload contract was found in the inspected sources; no runtime change,
private payload decoding, byte swap or hardware action follows. The negative
finding is scoped to this search, not a claim that a specification cannot exist.

**Limitation.** ETS6 can write `DeviceInstance/@LoadedImage`/`@CheckSums` and
`Hardware2Program/@LoadedImage`/`@CheckSums`. KNXBench keeps them as known
attributes the model does not carry (`known.rs`) and never decodes or compares
them. In a privately supplied project they occur on a few devices with
merged-procedure programs and on some `Hardware2Program` entries. The value is
Base64 of a raw-deflate stream whose inflated content is binary, in a format no
source available here documents.

**Why it matters.** With a documented format it could serve as an offline
differential reference for KNXBench's image builder on those masks (ADR-0098,
§142), without bus access.

**Boundary.** The content is device configuration and therefore private. It is
never download input and never an authority.

**Lifted when.** A public source documents the format, or a controlled
comparison on the owner's own device (read-back against the stored image of the
owner's own project, under the existing hardware rules) establishes it. Only
aggregate agreement enters the repository. Ledger `KL-172`.
<!-- END retained catalogue record -->
