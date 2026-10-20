# Product directory stream integrity (20 October 2026)

## Evidence and scope

This follows the product-reader residue of
[WP-16](../OPEN_WORK.md#wp-16--empty-directory-zip-integrity) and
[KL-159](../limitations/KL-159-project-archive-budgets-and-duplicate-names.md).
The independent product parser continues to use the already checked central
record, exact local payload range and effective decoded name. No ETS-specific
compression behavior is assumed.

## Verified public references

Retrieved directly on 20 October 2026; these are format/API references, not
claims about a proprietary exporter:

- [PKWARE APPNOTE](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT),
  sections 4.3.8 and 4.3.9: descriptors follow compressed data and are not part
  of that stream. Its no-file-data wording for empty entries does not itself
  establish interoperability with compressed empty directories.
- [RFC 1951](https://www.rfc-editor.org/rfc/rfc1951.txt), sections 3.2.3–3.2.4:
  final-block signaling and uncompressed blocks explain why a complete empty
  DEFLATE stream has more than one valid byte representation.
- [flate2 1.1.10 `Decompress`](https://docs.rs/flate2/1.1.10/flate2/struct.Decompress.html):
  the low-level API accepts raw DEFLATE (`new(false)`) and exposes consumed input
  and produced output. `Finish` marks the supplied input as final. A one-byte
  output buffer suffices to disprove the asserted empty payload without
  allocating its expanded contents.

## Deliberate admission policy

The proposed product policy matches the existing project-directory check:
accept a complete empty stream only when it consumes the entire declared
compressed payload. Preserve stored empty directories and multiple valid empty
DEFLATE shapes, including descriptor-bearing forms. Do not mistake a descriptor
for trailing stream data. This is an explicit integrity policy, not a new claim
of full ZIP or ETS compatibility.

Apply it only to unencrypted, DEFLATE-compressed directories declaring no
expanded data, after the existing name, encryption and size checks. Keep the
existing bounded reader and CRC check. Ordinary file stream consumption,
encryption-header semantics, supported methods and archive layout stay outside
this package. Member open/read failures may gain their already validated decoded
name without replacing the underlying technical reason or public error variant.

## Read-only compatibility census

[V] A bounded scanner examined 171 unchanged input files: 170 top ZIP archive
instances (166 distinct byte contents) and 180 readable nested ZIP instances
(151 distinct byte contents). Four additional named nested payloads were not
ZIP containers. The combined ZIP population has 314 distinct byte contents;
per-level distinct counts cannot be added because some contents recur across
levels.

All 260 directory records (166 top-level, 94 nested) were unencrypted, stored,
CRC zero, with no compressed or expanded bytes and no descriptor. None would
be newly rejected by the proposed directory-only rule; no directory probe was
unresolved. No real DEFLATE directory occurred. Alternate empty streams,
descriptors, truncation, hidden output and trailing bytes are therefore covered
only by 35 synthetic scanner controls, all passing. The controller reran the
retained-manifest verifier: all 16 accounting/identity checks passed.

No resource-limit refusal occurred within the scanner's explicit bounds:
256 MiB per top input, 128 MiB per nested payload, nesting depth three,
2 GiB cumulative nested expansion and one byte of directory-probe output.
The inventory also found 14 encrypted non-directory entry occurrences and two
unsafe-path occurrences. These observations do not imply package admission;
ordinary payload CRCs, XML semantics and production installation were not
verified. Detailed private identities and reproducible scanner evidence remain
local; only aggregate technical results are recorded here.

## Acceptance status

[V] Original-production controller replay: 27 named failures and ten positive
controls reproduce the admission/context gaps. The minimal product-only repair
passes those 37 cases plus three multi-chunk stream controls (40 total). The
chunk controls cover a 10,002-byte empty stream, trailing input and a missing
final block; compressed staging remains 8 KiB and expanded probing one byte.
The helper runs after the normal CRC reader and reads raw compressed member
bytes, excluding descriptors. It uses incremental DEFLATE status and consumed
input rather than a canonical byte-pattern comparison.

Regression coverage includes fresh/populated closed-database byte comparisons,
rollback of an earlier new inner package, historical hash retry revalidation,
CP437-decoded paths and reachable method-12 member-open refusals. Existing
error variants and underlying CRC/codec reasons are retained. Ordinary-file
trailing input is explicitly pinned unchanged. No live WAL concurrency proof
or new codec support is claimed. Independent source review accepted with zero
Critical/Important findings. Removing exact-consumption enforcement causes eleven
named failures; removing leaf context causes 29. Production bytes were restored
exactly, then all 40 focused tests and Clippy passed again. Maintained merged
delivery acceptance remains pending.
A structural census alone will not establish successful product installation,
lossless normalization or device programmability. Private input names, hashes
and member details must remain outside the public repository.
