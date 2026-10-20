# Product ordinary-file DEFLATE extent investigation (20 October 2026)

## Boundary and status

The product-directory repair is delivered on `main` as `9c79634b` (result-only
follow-up `87fea45e`). This is a **separate, evidence-first investigation** of
ordinary file members in the product reader and its wrapper consumers. The
investigation established the explicit
[whole compressed-stream contract](../contracts/product-file-stream-integrity.md)
before implementation. Project-container file streams and encrypted-header
semantics remain outside this claim.

Read the current [WP-16 scope](../OPEN_WORK.md#wp-16--empty-directory-zip-integrity)
and [KL-159](../limitations/KL-159-project-archive-budgets-and-duplicate-names.md)
before treating this record as an implementation instruction.

## Verified source facts

Primary pages retrieved directly on 20 October 2026:

- [PKWARE APPNOTE](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT),
  sections 4.3.8–4.3.9: file data and the optional descriptor have separate
  extents; a descriptor follows the last compressed-data byte. An exact-stream
  check must not consume a descriptor as payload.
- [RFC 1951](https://www.rfc-editor.org/rfc/rfc1951.txt), section 3.2.3: the final
  block indicator terminates the DEFLATE data set. Section 3.2.4 permits ignored
  alignment bits for stored blocks. Whole trailing bytes, alternate valid block
  encodings and unused bits within a consumed byte are distinct questions;
  matching a canonical compressed byte pattern is not a valid integrity test.
- [flate2 1.1.10 buffered DEFLATE decoder](https://docs.rs/flate2/1.1.10/flate2/bufread/struct.DeflateDecoder.html)
  explicitly stops after one stream even when the underlying reader has bytes
  remaining. End-of-decoded-output alone therefore cannot prove full consumption
  of a ZIP member's declared compressed extent.
- [zip 8.6.0 member API](https://docs.rs/zip/8.6.0/zip/read/struct.ZipFile.html)
  exposes declared compressed size, expanded size and CRC independently.

[V] Installed source for the lockfile versions confirms `ZipArchive::by_index_raw`
uses `ZipFileData::find_content`, whose raw reader is limited to the member's
compressed size. The normal compressed reader wraps a buffered decompressor in
a CRC reader. These details identify a possible extent-validation boundary;
they do not establish runtime compatibility or a completed alternative decoder.

## Existing preservation and consumer boundaries

[V] A standalone package retains the original archive bytes and a separate
normalized member index. A wrapper retains its inner packages, while its other
members are reported by path, decoded length and digest, not stored as files.
Thus a trailer absent from decoded bytes is not automatically absent from the
retained standalone archive. Reports must distinguish archive preservation,
normalized-member representation and wrapper-only extras rather than call all
three cases the same data loss.

The shared `read_checked_member` is used by standalone installation and wrapper
validation; later strict-namespace and ingestion passes reread previously
validated members. Historical hash retries revalidate before adding source
names. A stricter admission rule must reach each of those entry points and keep
transaction rollback and idempotent source-name behavior intact.

## Evidence required before choosing a repair

- Independently encoded, separately named original-production controls for
  nonempty and empty ordinary files, nonzero/zero trailers, concatenated streams,
  descriptors, multiple input chunks and valid alternate block structures.
- Fresh/populated database cases and exact raw-archive/member/report assertions,
  including wrapper extras and inner packages. Characterization tests must label
  the existing behavior explicitly; they are not evidence of a fix.
- Bounded read-only ordinary-file census with explicit encrypted/unsupported,
  corrupt, truncated, over-budget and unmeasured categories. Directory findings
  from the previous package cannot stand in for ordinary-file CRC/stream tests.
- A declared admission policy and named counterexamples before implementation.
  A secondary fixed-buffer probe preserves the normal reader but repeats
  decompression; replacing the decoder instead changes a larger trust boundary.
  Measure the actual candidate's resource cost before choosing extra complexity.

## Verified evidence and interpretation

[V] Controller-rerun independent census verification reconciles 15,029 ordinary
file records in 350 ZIP archive instances (314 distinct archive contents) from
171 unchanged inputs. All 12,772 measured unencrypted DEFLATE instances consumed
the complete compressed extent and matched full-output CRC and declared size;
2,243 stored instances also passed CRC/size checks. Fourteen encrypted instances
remain unmeasured. No measured trailer, malformed/truncated stream, size/CRC
mismatch, unsupported method or budget refusal occurred. Directories (260) were
inventoried separately and do not count as ordinary-file evidence.

The scanner streamed 7,372,391,938 expanded bytes using 64 KiB buffers, within
explicit limits of 1 GiB expanded per file, 32 GiB cumulative expanded bytes,
nesting depth three and 128 MiB per nested payload. The independent verifier
rehashed all 171 inputs, 350 archive identities and 15,015 measured compressed
extents, reconciled private rows and rejected count/hash tampering. All 54
independent calibration controls passed. Previous-importer acceptance is
**unmeasured**. The recursive scanner follows named ZIP/product payloads more
broadly than the nonnested runtime wrapper policy; four named decoded non-ZIPs
and encrypted records do not establish importer compatibility.

[V] Twenty-one synthetic original-production characterization cases pass, as
do all forty prior WP16 cases and owning-test-target Clippy. Controller verification
matches every compiled/executed identity and frozen source digest. Ordinary files
accept zero/nonzero trailers and a second stream; normalized members contain
only the first output. Original standalone/inner package archives remain
byte-exact, whereas outer wrappers are not retained under their existing
contract. This is an admission/report boundary, not total package data loss.
Attempt one failed setup before Cargo at exact insertion-range accounting;
attempt two corrected that guard without whitespace normalization. The failed
attempt is retained and credited with no runtime result.

## Decision and current acceptance

The [contract](../contracts/product-file-stream-integrity.md) chooses exactly one
complete stream per admitted unencrypted DEFLATE file, excluding descriptors,
and names the current acceptance pins to supersede explicitly. Fixed-buffer
secondary validation keeps the normal CRC decoder and error variants; measure
the additional decode cost before delivery. Research/census/characterization
are verified; the local implementation passes all 78 focused cases after 25
original-production semantic failures (53 original passes). The controller
reran the focused suite and all-targets Clippy and verified unchanged source
hashes. Thirteen acceptance pins were explicitly superseded; 39 prior directory
cases and nine other characterization controls remain, with seventeen added
cases covering output-only progress, empty files, input/output boundaries and
historical retries. Selections overlap, so those counts are not additive.

### Measured secondary-decode cost

[V] Same optimized executable, deterministic prebuilt inputs, warmups and ten
alternating rounds on one pinned CPU; 80 samples. The controller recomputed all
four medians from the samples and verified the executable identity. The probe
copies final candidate and original reader code into scratch, with fixture
building and preflight outside timing and allocation/decode inside timing.

| Synthetic workload | Original microseconds/read | Candidate microseconds/read | Ratio |
| --- | ---: | ---: | ---: |
| 4 KiB compressible | 2.277 | 4.080 | 1.792 |
| 4 KiB incompressible | 1.233 | 2.263 | 1.836 |
| 4 MiB compressible | 321.582 | 491.687 | 1.529 |
| 4 MiB incompressible | 377.574 | 522.101 | 1.383 |

The extra buffers are fixed (8 KiB input, 64 KiB output), with no second expanded
copy. This is additional bounded decoder work, not a performance improvement or
end-to-end installation benchmark. CPU frequency/thermal isolation and peak RSS
were not measured. An initial probe selected zero tests and was rejected; the
final source-bound run executes exactly one benchmark and retains all samples.
Initial historical fixtures lacked member indexes; corrected fixtures were rerun
against original production before claiming the final 25 semantic failures.

Independent full-diff review accepted with zero Critical/Important findings and
source bindings rechecked. Maintained committed-head delivery gate remains pending.
Private paths, member identities and content hashes stay local; only aggregate
technical outcomes are published here. No new codec, ETS, browser, device or
release compatibility is claimed.
