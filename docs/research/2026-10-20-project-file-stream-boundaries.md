# Project ordinary-file stream boundaries (20 October 2026)

## Scope and evidence level

**Source/format audit only; no admission-policy decision or implementation.**
Baseline: `ebdfac5d04286696119d7a0bba58e7333fda80dd`. No application,
test, build, network, bus, importer or database execution, and no corpus reads.
Existing test bodies were inspected, not run. Concurrent characterization work
is not evidence in this note; test references below describe the baseline.

This is the project-container follow-up in
[WP-16](../OPEN_WORK.md#wp-16--empty-directory-zip-integrity), not an extension
of the delivered [product-file policy](../contracts/product-file-stream-integrity.md).
AES remains refused. Directory encryption-header validity is separate open work.
No ZIP-wide or ETS-wide compatibility claim follows.

## Primary-source binding

[Cargo.lock](../../Cargo.lock) pins **zip 8.6.0** and **flate2 1.1.10**.
[Workspace features](../../Cargo.toml) select `deflate-flate2-zlib-rs` for zip
without its defaults; the project crate also directly names flate2. Every
reviewed installed dependency source file was compared byte-for-byte with its
cached published crate member; the cached crate SHA-256 matched Cargo.lock:

| Published crate | Lockfile/package SHA-256 |
| --- | --- |
| zip 8.6.0 | `2d04a6b5381502aa6087c94c669499eb1602eb9c5e8198e534de571f7154809b` |
| flate2 1.1.10 | `6e634e2e0ebac1ee034020da1ca582e17ffe4e0f5e985823721e168928136dcb` |

The format texts below were read from retained **public research references**
retrieved for the earlier product investigation, not from project archives and
not freshly fetched in this audit:

- [PKWARE APPNOTE](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT),
  retained version **6.3.10, FINAL, revised 1 November 2022**:
  §§4.3.8–4.3.9 separate file data from the following descriptor;
  §§4.4.8–4.4.9 include a decryption header in compressed size;
  §§6.1.2–6.1.7 describe traditional encryption of compressed data and its
  twelve-byte header. Retained text SHA-256:
  `0b993022a7d320a0bf704e6980bea36fafd17a6066ab994db0a0c16278a50cd6`.
- [RFC 1951](https://www.rfc-editor.org/rfc/rfc1951.txt), **May 1996**,
  §§3.2.3–3.2.4: BFINAL identifies the final block; stored blocks ignore bits
  up to the next byte boundary. Whole extra bytes, multiple blocks within one
  stream, concatenated streams and ignored bits are not interchangeable.
  Retained text SHA-256:
  `5ebf4b5b7fe1c3a0c0ab9aa3ac8c0f3853a7dc484905e76e03b0b0f301350009`.
- Versioned dependency source/API locators:
  [zip 8.6.0](https://docs.rs/crate/zip/8.6.0/source/src/),
  [flate2 1.1.10](https://docs.rs/crate/flate2/1.1.10/source/src/).
  Findings here are from the checked local published-source copies, not an
  assertion that these web pages were fetched again.

In zip, `ZipArchive::by_index_raw` (`src/read/zip_archive.rs:535–545`) calls
`ZipFileData::find_content` (`src/types.rs:267–275`): it seeks to local data
start and returns `take(compressed_size)`. It neither decrypts nor decompresses
nor CRC-checks. Normal `by_index_with_options` constructs crypto and codec
readers; `make_reader` wraps decoded output in `Crc32Reader`
(`src/read/readers.rs:265–297`). DEFLATE selects
`flate2::bufread::DeflateDecoder` (`src/compression.rs:495–508`).

Both flate2 DEFLATE reader APIs stop after one stream. The `read` variant can
read ahead; source position/remaining underlying bytes are therefore not
consumption proof. Its `total_in()` delegates to the actual decoder counter
(`src/deflate/read.rs:125–134,226–242`). The buffered API documents the same
single-stream boundary (`src/deflate/bufread.rs:128–136`). `zio::read`
(`src/zio.rs:124–169`) distinguishes incomplete input from successful stream
end. `Decompress::{total_in,total_out,decompress}` exposes counters and status
(`src/mem.rs:451–491`) for an explicit bounded check. Decoded EOF and matching
CRC alone do not establish exact compressed consumption.

## Structural extent: what the project reader actually validates

[RawRecord and check_decoded_names](../../crates/knx-etsproj/src/container.rs)
(`780–935`) bind each raw central name, **raw flags and method**, extra fields,
and zip-resolved CRC/sizes to a physical `by_index_raw` member. Strict flagged
UTF-8 and raw/decoded case collisions are checked. Names stay bound to physical
indices, not a second `by_name` interpretation.

[`local_record::check`](../../crates/knx-etsproj/src/container/local_record.rs)
(`83–136`) computes:

```text
local.data   = [local header + raw name + local extra,
                that start + resolved central compressed_size)
local.record = [local header start, local.data end + descriptor length)
```

It compares name/flags/raw method, local CRC/sizes (including ZIP64 resolution)
and Unicode Path fields. With bit 3, the complete local CRC/size tuple may be
all zero or equal the central tuple; a matching descriptor must start at
`local.data.end`. `descriptor_end` (`206–228`) tries signed and unsigned forms,
with wide sizes when the local ZIP64 field exists. Sorted record extents must
tile through the central-directory start. A prefix before the first record is
not ruled out by this tiling check.

Thus a trailer **inside** the agreed compressed extent belongs to `local.data`
structurally; it is not a gap between records. Structural agreement does not
prove that a decoder consumed it. File data ranges are currently discarded
after validation; only directory ranges are retained temporarily for
`check_directory`. Its exact empty-stream validation is already delivered and
must not be broadened into ordinary encrypted-file decoding.

For traditional ZipCrypto, compressed size includes the twelve encrypted
header bytes. `local.data` and `by_index_raw` include them; the descriptor is
outside both. After `zipcrypto::decrypt`, the supplied compressed plaintext is
exactly the raw payload with those twelve bytes removed and the rest decrypted.
Compare DEFLATE consumption with **that plaintext length**, not with the
ciphertext/header-inclusive compressed size. Never feed a descriptor or an
encryption header into a raw DEFLATE validator.

## Read/preparation map

All repository symbols in this table are in
[container.rs](../../crates/knx-etsproj/src/container.rs), except the cipher.

| Path | Preparation and decoder | Existing integrity / failure boundary |
| --- | --- | --- |
| Unencrypted outer ordinary file | `open_raw` validates structure and inventories via `by_index`; `read` (`655–683`) opens the validated index and calls `read_declared` | Lazy payload read; normal zip CRC/codec errors become `Read { path, cause }`. `take(declared + 1)` rejects expansion beyond declaration. It does **not** explicitly reject shorter output or inspect consumed compressed bytes. |
| Outer `<part>.zip` carrying the inner archive | `open_with_password` (`318–354`) directly calls outer `by_index` and `read_declared`, then builds inner `ZipArchive` over decoded bytes | Bypasses `Container::read`; changing that method alone misses this preparation path. Per-entry bound applies first. Inner parse failure names the outer payload; inner structural errors can instead name the inner decoded member. |
| Unencrypted inner ordinary file | After inner identity/layout checks and aggregate-budget admission, `by_index_raw` bytes go to `decompress` (`415–468`, `955–986`) | Stored clones raw bytes; DEFLATE uses `flate2::read::DeflateDecoder.take(declared + 1)`. Caller checks **equal** decoded length and CRC. Errors remain leaf `Read`; no compressed-consumption check. |
| Supported ZipCrypto inner ordinary file | Raw method 99 is refused before cipher use. `zipcrypto::decrypt` removes/checks its header, then the same `decompress` runs (`471–559`) | Cipher header mismatch becomes `WrongPassword { nested_entry }`; truncated header becomes leaf `Read`. Any DEFLATE `decompress` error is deliberately remapped to `WrongPassword`; final length/CRC failure is also `WrongPassword`. Unsupported codecs remain `Read`. |
| Re-read prepared inner file | `read` returns a clone from `decrypted` | No new decoder/cipher run; enforcement must happen during preparation, before insertion. |

The cipher implementation is
[`knx_secure::zipcrypto::decrypt`](../../crates/knx-secure/src/zipcrypto.rs)
(`188–221`): it rejects input shorter than `HEADER_LEN`, accepts either CRC-high
or DOS-time-high check-byte convention and decrypts all remaining bytes. This
is existing legacy compatibility, not authentication. Do not replace it with
zip's own crypto or tighten its check-byte conventions as part of extent work.

`open_with_password` opens the first signature-selected `<part>.zip` only, not
arbitrary recursive ZIP members. Before unpacking, it replaces that opaque
entry in the inventory, combines remaining outer declarations with rooted inner
file declarations, and checks the total budget. Each prepared entry is checked
for rooted collision and per-member size before insertion. Later failure drops
the local container; no partial container is returned. No password still means
`PasswordProtected` when that nested shape exists, even when its members are
plain. Outer encrypted members are not this supported path: `open_raw` calls
`by_index` without a password and can fail with `NotAZip` first.

**Report nuance:** the `decrypted` map also holds *unencrypted* inner files.
`was_decrypted`/`decrypted_payload` therefore describe prepared nested entries,
not a measured count of encrypted members. A plain nested archive opened with
a supplied password can receive the existing protection-loss warning. This is
a baseline reporting distinction, not authorization to change it here.

## Preservation, retries and refusal semantics

### Whole-project import is not product-package storage

[`import_knxproj_bytes_inner`](../../crates/knx-etsproj/src/lib.rs)
(`235–318,450–503`) consumes the archive into `Container`, parses project XML,
then collects **every inventoried file**, including the original topology and
metadata XML, with an empty regenerated list.
[`collect_container_entries_observed`](../../crates/knx-etsproj/src/opaque.rs)
(`109–153`) hashes the **decoded bytes returned by `read`**. No original outer
ZIP, local-record bytes, descriptor, compressed stream or encryption header is
an `ImportOutcome` field. An opened container still has its in-memory outer
archive, but that is not persisted raw-archive preservation.

For the selected nested payload the inventory exposes prepared plaintext files,
not the original `<part>.zip`. Consequently, in ordinary full-project import a
trailer omitted by the decoder is not restored by opaque storage. Additional
unopened ZIP members can themselves be retained as decoded opaque files; that
does not mean their internals were imported or validated recursively.

[`persist_outcome`](../../crates/knx-app/src/import.rs) first receives a completed
`ImportOutcome`, then routes manufacturer files to `ingest_file` or the opaque
fallback and writes decoded opaque rows plus a manifest. Native
[`insert_opaque`](../../crates/knx-store/src/opaque.rs) stores bytes/hash, not raw
ZIP records. Carried manufacturer files likewise represent decoded member bytes
([manufacturer_files.rs](../../crates/knx-store/src/manufacturer_files.rs)).

Contrast the product reader: [`install_admitted_package` and
`read_checked_member`](../../crates/knx-productdb/src/package.rs) retain raw
package archives separately, validate normalized member indexes, and revalidate
a historical package before its retry source-name insertion. A full project
reimport has no corresponding archive-hash admission shortcut or
`package_source_name` update: it re-enters the parser and reconstructs decoded
files. Historical project stores may contain only those decoded files; bytes
never retained cannot be recovered or retrospectively certified.

### There is an explicit archive-retaining exception

[`selective_import::source_from_bytes`](../../crates/knx-app/src/selective_import.rs)
(`143–213`) clones source bytes into the same importer **before** preparing its
`SelectiveImportArchive` under `imports/<source-hash>/source.knxproj`, and also
retains scoped member evidence and a `SelectiveImportReport`. Here the original
archive, including ciphertext and compressed trailers, is retained for the
subsequent selective merge. Do not extend the full-import statement that
ciphertext is gone to this path. A stricter importer would refuse before this
new source is admitted; it must not silently remove or rewrite historical
scoped evidence. [Selective import](../SELECTIVE_IMPORT.md) separately discloses
whole-source privacy/consent. This audit did not exercise merge or native reopen.

### A reader error is not automatically a persisted diagnostic

The full pipeline propagates `ContainerError` as `ImportFailure::Container`,
then `AppError::Import`. A fatal container error produces no successful new
`ImportReport`. The existing ZipCrypto protection-loss `unsupported` row is
emitted only on successful import (`lib.rs:486–494`). Moving an extent failure
outside the encrypted DEFLATE error mapper changes both error type and password
retry behavior: currently a DEFLATE failure is `WrongPassword`, including damaged
ciphertext under the correct password. That choice needs an explicit contract.

The CLI's `write_imported_store` and server's `import_and_project` attach final
primary historical report evidence through
[`import_evidence::capture_primary`](../../crates/knx-app/src/import_evidence.rs).
The raw application service does not itself add that envelope. Historical
`read` validates the envelope/hash and returns the recorded report; missing
legacy evidence is unavailable, not a newly measured zero. Scoped report access
in [`source_reports`](../../crates/knx-app/src/import_evidence/source_reports.rs)
also returns recorded evidence without running the source importer. A new
stream policy must not relabel old successful reports as newly validated.

The server's [`load_failure_kind`](../../apps/knx-server/src/domain.rs)
(`610–638`) distinguishes password-required/wrong from generic
`projectNotImportable`; the CLI offers its password hint only for
`PasswordProtected` ([main.rs](../../apps/knx-cli/src/main.rs):490–510).
No new transport behavior is verified here.

### Mutation guarantees must stay scoped to the caller

A container/read refusal in the full import occurs before
`persist_outcome` starts product ingestion or opaque/manifest writes. A late
opaque member refusal is still before those writes. This is the useful place
to enforce integrity without needing a cross-database rollback mechanism.
It is **not** evidence that all later service failures are atomic: product
files are ingested individually, `insert_opaque` and `insert_manufacturer_refs`
are separate calls, and the service does not wrap both databases in one
transaction. Native `save_project_with_passthrough` does atomically save the
project/opaque/manifest together, but is a distinct boundary.

The CLI stages project rows in memory and opens the target store only after
import succeeds; it can already open/migrate the product database before the
import. Distinguish no new manufacturer ingestion from no database-open side
effect. Byte-preservation tests need an already populated/migrated fixture.

Another consumer is
[`manufacturer_files::restore_from_knxproj`](../../crates/knx-app/src/manufacturer_files.rs)
(`225–257`): it uses `open_outer`, reads only missing manifest members, records
per-member `Read` failures and **continues**, possibly after earlier successful
ingestions. It is not whole-archive atomic restore and does not re-read members
already present. Putting all file decoding into `open_raw` would change its
selection/report behavior as well as lazy container opening.

## Smallest candidate boundaries — not a selected policy

1. Keep name/layout/directory validation and existing output CRC readers.
   For outer reads, a private index-based read helper could run a fixed-buffer
   raw DEFLATE extent check **after** successful bounded decoded reading. Both
   `Container::read` and direct nested-payload acquisition must use it.
   `by_index_raw` already supplies a descriptor-excluding, size-limited view of
   the validated physical member. Do not duplicate the ZIP parser or eagerly
   inflate every ordinary file in `open_raw`.
2. For nested plain and ZipCrypto entries, `decompress` already receives the
   correct compressed plaintext slice. Retaining the existing decoder long
   enough to inspect its `total_in`, or a local bounded status/counter loop,
   is narrower than replacing crypto or adding a second expanded copy. Preserve
   equal-length/CRC checks and encrypted error remapping deliberately. A raw
   ciphertext check before decryption would be incorrect.
3. If a secondary decoder is selected, use fixed staging/output buffers,
   bounded output counters and input-**or**-output progress, including output
   draining after raw EOF. Require complete stream end and byte-exact extent;
   do not require canonical byte encodings or zero alignment bits. Measure its
   extra decode cost on the project paths; product timings are not that proof.

**Unresolved:** one-stream exact-extent refusal versus any explicitly justified
padding policy; the error category for correct-password encrypted trailers;
outer read's historical upper-bound-only size behavior versus inner/product
exact length; restoration's per-member continuation; historical native/scoped
report preservation versus fresh import admission. These are distinct contracts,
not reasons to silently expand this repair.

## Evidence needed before enforcement

- Separately named original-source fixtures for outer ordinary files, the outer
  ZIP-bearing member, plain inner files and actual ZipCrypto inner files. A
  protected-layout fixture with plain entries does not exercise encryption.
  Cover empty/nonempty output, zero/nonzero trailers, a second empty/nonempty
  stream, corrupt/no/truncated stream, correct CRC with shorter declared/actual
  length in both directions, stored controls and unsupported codecs.
- Descriptor controls with and without signature, true/zero local values and
  ZIP64 size forms; encrypted compressed size includes the header while the
  descriptor remains outside. Preserve AES refusal, wrong-password and
  header-truncation distinctions without opening directory-header scope.
- Independent literal/multi-block streams, legal noncanonical encodings and
  alignment bits, multi-chunk input/output, high expansion with output-only
  progress, final draining and a trailer after draining. CRC failure must still
  reach its existing branch rather than being hidden by a new error.
- Real pipeline refusal before any writes for malformed late opaque/manufacturer
  entries as well as XML and nested preparation; fresh/populated project and
  product destinations, missing/existing destination files, with/without product
  DB, closed-file and explicitly scoped sidecar comparisons. Separately pin
  restore's per-member continuation and unaffected already-present members.
- Historically accepted decoded project rows, original primary/scoped reports
  and any retained selective archive: native load/save must preserve them;
  fresh same-byte reimport must reach the new check rather than simulate product
  retry bookkeeping. Successful protected imports must keep redaction and their
  established warning. Do not manufacture unavailable historical raw bytes.
- If compatibility evidence is required to choose a policy, obtain a separately
  authorized, bounded **project** sample scope with encrypted/nested accounting.
  This audit reads no corpus; neither the product census nor existing public
  protected-project test descriptions measure project trailing extents.

Baseline source tests provide starting points, not new results:
[`decoded_member_names.rs`](../../crates/knx-etsproj/tests/it/decoded_member_names.rs)
contains `protected_with` (plain inner files),
`every_member_with_a_data_descriptor_imports`,
`a_protected_payload_with_data_descriptors_opens`,
`zip64_members_with_data_descriptors_import`, and
`empty_file_members_with_a_bad_crc_or_no_stream_are_refused_when_read`.
The [container tests](../../crates/knx-etsproj/src/container.rs) cover stored
ZipCrypto CRC false accepts and raw-method AES refusal.
[Library password tests](../../crates/knx-etsproj/tests/it/password_import.rs)
cover DEFLATE false accepts, rooting and warning/redaction;
[service password tests](../../crates/knx-app/tests/it/password_import.rs) cover
save/reopen and missing/wrong password.
[`a_malformed_empty_directory_refuses_without_changing_existing_databases`](../../crates/knx-app/tests/it/import_service.rs)
is a preservation-fixture pattern, not ordinary-file or encrypted-extent proof.

**Decision:** sufficient source evidence to define candidate enforcement points
and synthetic characterization; **insufficient evidence to call a stricter
project/ZipCrypto admission policy compatibility-safe**. Decide the named
contracts after source-bound original behavior is measured. No implementation,
corpus authority or blanket legacy-store revalidation follows from this audit.

## Subsequent measured behavior and policy decision

The preceding audit is a retained static baseline, not runtime acceptance.
Subsequent original-production characterization passes thirty new tests and all
62 prior owning-module cases, plus thirteen archive-integrity and eight password
cases (113 unique Rust test identities). Controller reran the evidence verifier:
compiled/executed identities, exact original test reconstruction and frozen
production hashes reconcile. Outer, plain nested and actual synthetic ZipCrypto
paths accept trailers/concatenation and expose only the first decoded stream.
Source files stay unchanged; selective raw retention remains a separate caller.

The [project-specific integrity contract](../contracts/project-file-stream-integrity.md)
now chooses one complete stream per validated plaintext compressed extent. It
explicitly preserves encrypted-corruption-to-`WrongPassword`, outer size policy,
lazy reads, selected-member restoration and historical report evidence. It
requires new-source refusal/reopen tests and direct project-path measurement.
No private project extent census or compatibility-safe guarantee is claimed;
implementation and delivery acceptance remain pending.

## Reviewed paths and source identities

Review was bounded to relevant symbols/sections, not full-file semantic audits.
The accompanying source-review receipt records all whole-file hashes, ranges and
whether live files still matched the baseline. Mutable handover and concurrently
extended tests are not used as production identity evidence.

Repository path inventory:

- Context/dependencies: `AGENTS.md`, `.ai/CURRENT_STATE.md`, `Cargo.toml`,
  `Cargo.lock`, `crates/knx-etsproj/Cargo.toml`.
- Documentation: `docs/OPEN_WORK.md`, `docs/IMPORT_EXPORT.md`,
  `docs/COMPATIBILITY.md`, `docs/IMPORT_INTEGRITY.md`, `docs/SELECTIVE_IMPORT.md`,
  `docs/limitations/KL-159-project-archive-budgets-and-duplicate-names.md`,
  `docs/research/2026-10-20-product-file-stream-integrity.md`.
- Project/cipher: `crates/knx-etsproj/src/container.rs`,
  `crates/knx-etsproj/src/container/local_record.rs`,
  `crates/knx-etsproj/src/lib.rs`, `crates/knx-etsproj/src/opaque.rs`,
  `crates/knx-secure/src/zipcrypto.rs`.
- Application: `crates/knx-app/src/import.rs`,
  `crates/knx-app/src/import_evidence.rs`,
  `crates/knx-app/src/import_evidence/source_reports.rs`,
  `crates/knx-app/src/import_payload_evidence.rs` (symbol search only),
  `crates/knx-app/src/selective_import.rs`,
  `crates/knx-app/src/manufacturer_files.rs`.
- Persistence/comparator: `crates/knx-store/src/opaque.rs`,
  `crates/knx-store/src/project.rs`, `crates/knx-store/src/manufacturer_files.rs`,
  `crates/knx-productdb/src/package.rs`.
- Consumers/tests: `apps/knx-cli/src/main.rs`, `apps/knx-server/src/domain.rs`,
  `crates/knx-etsproj/tests/it/decoded_member_names.rs`,
  `crates/knx-etsproj/tests/it/password_import.rs`,
  `crates/knx-app/tests/it/import_service.rs`,
  `crates/knx-app/tests/it/password_import.rs`.

Installed published-source inventory: zip 8.6.0 `src/read/zip_archive.rs`,
`src/read/readers.rs`, `src/types.rs`, `src/crc32.rs`, `src/compression.rs`,
`src/read.rs`; flate2 1.1.10 `src/deflate/read.rs`, `src/deflate/bufread.rs`,
`src/zio.rs`, `src/mem.rs`. Public reference inventory: APPNOTE text, RFC 1951
text and the retained public-retrieval `sources.json`; no archive corpus member
was opened.

Critical repository SHA-256 bindings:

| Path | SHA-256 |
| --- | --- |
| `Cargo.lock` | `76bbf38f8add913ce08376e0ef930856920a8b3bcec636b51a36aed237f34038` |
| `crates/knx-etsproj/src/container.rs` | `f6808c63c9246bb28a19bb72f4ff9bdcaca02ae6bbbaee5bb6c1faf905b2250b` |
| `crates/knx-etsproj/src/container/local_record.rs` | `05fa384402eb2b2e6d0b82cb65699f1d7dcf02e28ede1724e8a51197cba34176` |
| `crates/knx-etsproj/src/lib.rs` | `341b3bf7b91105a75727202a17422446eeb34359142f7d48e173b0a85ef1594f` |
| `crates/knx-etsproj/src/opaque.rs` | `d27f9816cfec9ee7db47517895d7b83c8c85b6c2b1411d7d40e4a9760f14e990` |
| `crates/knx-secure/src/zipcrypto.rs` | `ab27592a04955ef799e05470f6c15ac931185c699987ef4623afb85e43f740d3` |
| `crates/knx-app/src/import.rs` | `65e6ed82d735615d8c50077fee6fd0431313846d3b214f6ac6ba76086bee6e06` |
| `crates/knx-app/src/selective_import.rs` | `47ef5a32f969be9f66ae11c636560038474b4a138c1bad870cdca8698043d59a` |
