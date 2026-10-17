# Parameter byte-order download boundary

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-170)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->


## 170. Little-endian parameter byte order is refused for download images

**Evidence review 2026-10-13; boundary unchanged, read-only probe added.**
[Parameter byte order](../research/parameter-byte-order.md) extends the
[2026-10-10 public-source pass](../research/schema23-offline-boundaries.md) with
the local KNX specification corpus, schema copies, two independent
implementations, 3,341 public product programs and the private schema-23
sample. Still no source defines `LittleEndian` per parameter type. Base images
of 370 LittleEndian programs hold whole-octet defaults low octet first (one
Siemens System B family the other way), and one manufacturer re-declared 32
unchanged layouts from `BigEndian` to `LittleEndian`. But the same
manufacturers' BigEndian-declared and undeclared programs show the same
low-octet-first base images, so product files cannot tell what ETS writes.
[ADR-0119](../adr/0119-read-only-byte-order-probe.md) adds
`knx device byte-order-probe`: it reads (never writes) an ETS-programmed
device and reports per field whether it holds the project's values high or
low octet first. No encoder change follows from it.

**Limitation.** `knx_productdb::image::build_download_image` refuses every
program whose `Options/@ParameterByteOrder` is anything but `BigEndian`
(`ParameterByteOrder "LittleEndian": numbers are written high octet first`). An
undeclared byte order is treated as big-endian, the schema's default value
(two independent schema derivations, research above). Offline readiness marks
such devices `unsupported`, since 2026-10-13 with their own category
`byte-order` (before: `image-structure`); the refusal is the typed
`ImageError::UnsupportedByteOrder` with the same text, and the web download tab
names the reason in plain words before anything is prepared (§180). The pinned
download coverage (467 programs) has no program in `byte-order`: none of its
counts moved with the split. Until this entry the boundary lived only in the
module comment of `crates/knx-productdb/src/image.rs`.

**Prevalence.** Local product corpus (`OriginalData/ProductDatabases`, 114
archives, 525 unique application-program files by content hash, measured
2026-10-10 and again 2026-10-13): `LittleEndian` 3, `BigEndian` 90, undeclared
432. Public vendor downloads (853 archives, 3,341 unique programs): 590
`LittleEndian` (ABB 474, Busch-Jaeger 101, Hager 8, Berker 5, Siemens 2; 479 on
mask `0701h`). Private schema-23 sample: 12 of 32 programs, used by 54 devices,
most of its offline refusals, so a real installation can be hit far more often
than the local corpus suggests.

**Cause.** The image encoder writes multi-octet numbers high octet first
(`knx_core::commissioning::parameter_image`). No source available here defines
little-endian placement per parameter type (integers, floats, bit-offset
unions, fields that cross octet boundaries); the application-program XSD and
the ETS SDK documentation are not available (`IMPORT-06`).

**Related risk (not a refusal).** The base images of 211 BigEndian-declared
and 53 undeclared ABB/Busch-Jaeger `0701h` programs hold their defaults low
octet first, against 26 and 18 high octet first. KNXBench plans those
programs high octet first, as declared, and they are untested downloads
(`--accept-untested`). If ETS honours their declaration, nothing is wrong; if
those devices actually read low octet first, an untested download would write
wrong multi-octet values. The probe answers this for any such device too.
Since 2026-10-13 such a download carries the named warning
`base-image-low-octet-first` in its plan (CLI and server; owner decision,
[ADR-0120](../adr/0120-base-image-byte-order-warning.md)): a generic rule on the
program's own base image, which in the measured corpus flags 32 of 181
memory-downloadable programs, all ABB/Busch-Jaeger `0701h`, and no MDT,
Siemens or other program. The web download tab shows it in the plan's
**Before you write** box and asks for multi-octet checks in the tester
checklist (ADR-0121).

**Impact.** No offline image, compare plan or download for little-endian
programs. Import, display and parameter editing are unaffected.

**Lifted when.** Probe output from at least one ETS-programmed device per
affected manufacturer, with the device's project imported unchanged, shows
every discriminating field low octet first and the controls agreeing; then a
reviewed encoder change limited to what was observed (whole-octet numbers)
with exact signed/unsigned/multi-octet/boundary tests, keeping fields across
octet boundaries, floats, texts and negative values refused until observed
as well. Swapping octets on suspicion is not a fix. Ledger `KL-170`.
<!-- END retained catalogue record -->

## Re-check 17 October 2026: no new primary source

The local specification collection is unchanged since 7 October; no new private
file declares `ParameterByteOrder`; the xknx-editor encoder's byte-order path is
unchanged since the cited commit. The lift condition above stands
([review](../status/2026-10-17-ledger-residue-review.md#phase-2--evidence-re-check-no-new-primary-source)).
