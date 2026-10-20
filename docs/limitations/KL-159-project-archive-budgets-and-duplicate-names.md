# Project archive budgets and duplicate names

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-159)

## Current archive ceilings (20 October 2026 reconciliation)

The retained 1024 MiB total / 64 MiB member figures are historical.
[ADR-0142](../adr/0142-file-size-limits-follow-the-storage-ceiling.md) raised
the project reader to **4 GiB expanded payload in total** and
**1,000,000,000 bytes per member**, following the shared storage ceiling.
Nested payload accounting and declared-size/inflation checks still apply;
these are payload admission limits, not a peak-process-memory guarantee.
Duplicate/case-colliding names and inconsistent ZIP records remain refused.
The empty-directory CRC/complete-stream repair described below is delivered;
its trailing-file-stream and encryption-header follow-ups remain open.
The product-reader follow-up is delivered on `main` (`9c79634b`), with
its bounded acceptance recorded below.

Evidence: [shared limits](../../crates/knx-core/src/limits.rs),
[project container](../../crates/knx-etsproj/src/container.rs),
[archive-budget regressions](../../crates/knx-etsproj/tests/it/archive_member_integrity.rs).
The declaration-budget tests are not evidence of successful arbitrary
4-GiB imports or bounded peak memory.

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Empty directories must hold an empty payload (19 October 2026, WP-16)

The round-4 gap described in the retained record below is closed in the
outer archive and in a nested payload, through the same directory check. A
directory record that declares no content is now refused with
`InconsistentRecord`, naming the directory and the reason, when:

- its CRC-32 is not 0, the CRC-32 of no content (in both headers alike, or
  in a data descriptor that agrees with them);
- it is compressed but holds no complete stream: no bytes at all (with
  any method other than stored), a deflate stream cut short (`03`, or an
  empty stored block missing a byte), or bytes that are no deflate stream.

The refusal happens while the container opens, before anything is parsed or
written, so a project or product database that already exists stays
byte-for-byte unchanged. Still accepted: stored empty directories (ETS6,
`jar` with the UTF-8 flag and its `0xCAFE` extra field), Java
`ZipOutputStream`'s deflated `03 00` with bit 3 and a signed descriptor, and
any complete deflate stream that produces nothing (`01 00 00 ff ff`). Name,
budget, prefix and layout rules are unchanged. File members never had the
gap: they are read, and reading checks CRC and stream.

Evidence: 9 more tests in `decoded_member_names.rs`. Five were RED before
the fix: a nonzero CRC (stored and deflated) and a missing deflate stream
were accepted; a truncated stream was refused only as "carries data", and an
empty bzip2-method record only later, as an unsupported method. One more
test in `knx-app`'s `import_service.rs` seeds a project and product database
and compares their files and WALs after both refusals. A census of the
private corpus found 222 directory records in 231 archives (38 readable
nested payloads, 7,732 records), all stored with CRC 0 and no bytes, so none
of them is refused.

What remains (not part of this fix):

- A deflated *file* whose compressed bytes continue behind the end of its
  deflate stream imports, in both layers; the trailing bytes belong to no
  member and are not retained. The `zip` reader stops at the end of the
  stream. Recorded as open work.
- The product-package reader (`knx-productdb`) reads every member with the
  `zip` reader, so a nonzero CRC, a missing stream or a truncated stream in
  an empty directory is already refused there, though without the member's
  name. A deflated directory with bytes behind its `03 00` stream installs
  there; KNXBench's project reader refuses it (round 3, N12).
- A directory record with the encryption flag and no bytes is not checked
  for an encryption header.

## Product directory follow-up (20 October 2026)

Delivered on `main` as `9c79634b`; independent review accepted and the maintained
14-stage scoped gate passed (3,034 Rust tests, 97/97 selected corpus cases).
Sources stayed frozen and user-data guards stayed clear. The product-directory
claim is released.
`knx-productdb` now checks exact compressed-stream consumption for empty
DEFLATE directories after its existing bounded CRC reader. Fixed buffers avoid
allocating expanded contents for this secondary check. Member-open/read errors
retain `InvalidZip` and the technical reason while adding the validated decoded
leaf name, including reachable unsupported-codec failures. Wrapper errors retain
both outer package context and inner leaf context.

The controller reproduced 27 named RED cases against original production and
then passed those plus ten original positives and three multi-chunk controls.
Tests cover fresh/populated database preservation, earlier-inner-package rollback,
source-name retry integrity, alternate empty encodings and both descriptor forms.
Independent source review accepted (zero Critical/Important); two repair-removal
controls fail eleven and 29 named cases, respectively, then exact restoration
passes all 40 cases and Clippy again. Closed SQLite database bytes are compared;
concurrent WAL behavior is not claimed.

A read-only census found 260 stored empty directory records across 350 readable
ZIP instances; none would be newly rejected. No real DEFLATE directory occurred,
so that behavior is synthetic-test evidence only. The original ordinary-file
trailing-input and encryption-header boundaries above remain separate open work.
[Research, primary sources and evidence boundaries](../research/2026-10-20-product-directory-stream-integrity.md).

## Product ordinary-file follow-up (20 October 2026)

The separate product-only investigation is complete. Controller-verified
characterization demonstrates accepted zero/nonzero trailers and concatenated
streams; normalized member hashes/bytes describe the first stream only. Original
standalone and inner package archives are retained byte-exact, so this is not
total package data loss. Outer wrappers and their non-package extras have a
separate report-only contract. The earlier historical wording about unretained
trailing bytes must not be extended to all product archives.

The bounded ordinary-file census measured 12,772 complete, exact unencrypted
DEFLATE extents and 2,243 stored extents; fourteen encrypted entries remain
unmeasured. No measured trailer or CRC/size/stream error occurred. The 260
directory records were not counted as ordinary-file evidence. Independent
verification/calibration, scope and privacy limits are recorded in the
[research note](../research/2026-10-20-product-file-stream-integrity.md).

The [one-stream policy](../contracts/product-file-stream-integrity.md) explicitly
supersedes the current ordinary-file acceptance pins while preserving valid
alternate block forms, descriptors and error variants. A fixed-buffer secondary
check keeps the existing CRC decoder. It uses 8 KiB input and 64 KiB output
buffers with exact input/output counters and output-only progress handling.
Original production reaches 25 intended refusal failures and 53 passes; the
candidate passes all 78 focused cases and parent-run all-targets Clippy.
Historical retry and outer/inner wrapper rollback compare closed database bytes.
The additional direct-reader decode cost is measured at 1.38–1.84 times baseline
across four synthetic workloads, not end-to-end installation or peak memory.
Independent full-diff review accepted with zero Critical/Important findings.
Delivered on `main` as `29b69f36`: maintained scoped gate 14/14 GREEN,
3,072 Rust tests and 97/97 selected corpus cases, frozen inputs, valid proofs
and clear user-data guards. Publication readback agreed; the product file-stream
claim is released. Project-reader ordinary-file and encryption-header work
remain separate; no full ZIP/ETS compatibility is claimed.

## Project ordinary-file policy (20 October 2026)

The separate [source audit](../research/2026-10-20-project-file-stream-boundaries.md)
identifies normal outer reads, direct nested ZIP acquisition, plain nested files
and supported ZipCrypto-decrypted files as different enforcement boundaries.
Thirty original-production characterizations pass with 83 existing controls
(113 unique Rust identities). They establish trailer/concatenation acceptance,
decoded first-stream output and existing typed CRC/password refusal identities;
they do not establish database preservation or real-project compatibility.

The [project contract](../contracts/project-file-stream-integrity.md) explicitly
chooses exact complete plaintext DEFLATE extents and identifies fifteen acceptance
pins to supersede. ZipCrypto's twelve-byte header and ZIP descriptors are outside
the compared plaintext stream. Existing encrypted-corruption `WrongPassword`
mapping, outer decoded-size policy, lazy reads and restoration's per-member
continuation remain. Full import returns decoded members; selective import has
an additional original-archive/report retention boundary. Historical native and
scoped evidence must not be silently relabeled by the new source-admission rule.

No private project extent census was performed. The local implementation passes
153owning core cases after58original semantic failures; fifteen old acceptance
pins are explicitly superseded. Parent application checks pass26new cases and
the59pass/3ignore owning selection plus all-targetsClippy. Scoped source/closed
DB/sidecar preservation, native reopen, historical selective archives/reports
and restoration continuation are exercised; no live-WAL/crash/crossDB atomicity
claim follows. Source-bound synthetic project-path cost is disclosed in the
[contract](../contracts/project-file-stream-integrity.md#measured-project-path-work).
Independent full-diff review accepted with zero Critical/Important findings;
delivered on `main` as `8ccb99b3`. Maintained scoped gate 14/14 GREEN,
2,500 Rust tests and 149/149 selected corpus cases, frozen inputs, valid proofs
and clear user-data guards; publication readback agrees. Project ordinary-file
claim released. New AES support and directory encryption-header validity stay
separate.

## Directory encryption-header investigation (20 October 2026)

The [source audit](../research/2026-10-20-directory-encryption-header-validity.md)
locates a direct-inner project-directory skip before cipher-header handling.
Outer-project normal opening and the independent product reader have different
refusal paths. Directory preflight currently examines raw bytes, not decrypted
content. Public format facts and 94 original-runtime synthetic cases are
independently verified, including exact controller replay. The
[admission contract](../contracts/project-encrypted-directory-admission.md)
chooses explicit bit-0 project-directory refusal, deliberately superseding
fourteen nested acceptance pins and changing 48 refusal diagnostics. The minimal
shared guard passes parent core404/16ignored and app16new/41owning3ignored checks,
both-crate Clippy and header checks. Isolated mutation reproduces72semantic
failures between121-pass baseline/restored runs. Independent review accepts with
zero Critical/Important findings; minor stale-status wording is corrected.
Maintained delivery remains pending. Application snapshots establish only quiescent early refusal,
not crash/live-WAL/general cross-store atomicity. No new encrypted-directory
support or real-project compatibility claim. Bit-6-only consistency
and AES extra-field semantics remain separate questions.

<!-- BEGIN retained catalogue record -->

<a id="159-a-project-archive-may-unpack-to-at-most-512-mib-and-every-member-name-must-be-unique"></a>

## 159. A project archive may unpack to at most 1024 MiB, and every member name must be unique

**Limitation.** `knx-etsproj` refuses a `.knxproj` whose members together
declare more than 1024 MiB uncompressed (`MAX_ARCHIVE_UNCOMPRESSED`), or one
member of more than 64 MiB, and it reads every member only up to the size its
headers declare: a member that inflates further is refused by name. It also
refuses an archive in which two file members share a name, byte for byte or
differing only in ASCII case, with `DuplicateEntry` naming the second one.

**Why.** The AR18 independent review (2026-10-06, findings F2 and F3) showed
that a 1 MB crafted archive drove an import to 2–3 GB of memory, and that a
duplicate or case-variant member was collapsed or read in place of the other
without a report — while the documentation promised that nothing is dropped
silently. Lookups in the archive are case-insensitive, so either repeat let
one member's bytes stand in for another's.

**Impact.** A genuine project above 1024 MiB is refused with the totals in the
message instead of imported. ETS is not known to write duplicate or case-variant
names; such an archive is refused instead of imported with lost bytes. The total
budget bounds expanded archive payloads, not peak process memory.

**Update 2026-10-10:** the owner requested raising the former 512 MiB total
budget to 1024 MiB (1,073,741,824 bytes). The 64 MiB member limit, bounded reads,
identity/layout checks and cumulative outer/nested budget remain unchanged.
The tester-reported size motivated the increase; no tester archive was supplied
or imported, and no new large-project peak-memory measurement is claimed.
Existing memory-amplification observations below remain historical evidence.

**Lifted when.** A streaming import that keeps members on disk instead of in
memory, or measured evidence justifying a further bounded budget increase.

**Update 2026-10-06 (AR18 re-check N1/N2): identity is judged on the names
the reader uses, and a protected payload is counted before it is unpacked.**
The first fix compared raw name bytes. The `zip` reader, however, decodes
names — CP437 or UTF-8, and an Info-ZIP Unicode Path field may replace the
name entirely — and keys members by the result. So two different raw names
could still become one member, even one standing in for `0.xml`. The
archive, and a protected project's nested payload, are now refused:
- when the reader keeps fewer members than the central directory has
  records (`NameCollision`);
- when two decoded file names are equal ignoring ASCII case.

A protected payload's declared total is checked against the 512 MiB budget
before its first member is unpacked. Before this change, a 12 MB file
unpacked to 8 GB before the check.

Evidence: `crates/knx-etsproj/tests/decoded_member_names.rs` (7 tests on
hand-written archives), RED before the change, and four killed mutants.

What remains: an import inside the budget still peaks at about three times
the declared total in process memory (measured by the re-check: 480 MiB →
1.6 GB). The budget bounds the archive's bytes, not the whole process.

**Update 2026-10-06 (AR18 re-check round 2, N7/N9): a record's parts must
agree.** A Unicode Path field could still turn the real `0.xml` into a
"directory", whose bytes every later step skipped, and another member into
`0.xml`. The following are now refused, in the outer archive and in a
nested payload, with `InconsistentRecord` naming the reason:
- a directory record that carries data;
- a directory named like a file;
- a local header whose name differs from its central record's raw name.

The last one also catches two central records that share one local record.
Empty directory records, as ETS6 writes them, and archives behind a prefix
still import. Evidence: 8 more tests in `decoded_member_names.rs`, and four
killed mutants.

**Update 2026-10-06 (AR18 re-check round 3, N11/N12): every part of a record
must agree, and the records must fill the archive.** Round 2 compared only
the local *name*. The `zip` reader takes every size from the central record
and reads the local header only to skip its name and extra field, so a
central record that claimed 0 bytes hid the local bytes. With a Unicode Path,
the real `0.xml` vanished again and a forgery took its name. Now each local
record is compared with its central record, in the outer archive and in a
nested payload, and refused with `InconsistentRecord` naming the part:
- flags and compression method equal;
- CRC and both sizes equal, zip64 placeholders taken from the local zip64
  field;
- with a data descriptor (bit 3): the local values are zeros (APPNOTE 4.4.4)
  or the central values (Info-ZIP's `zip -e` writes those, and the ZipCrypto
  fixtures were made with it), and a descriptor, with or without its
  signature, follows the data and carries the central CRC and sizes (8-byte
  sizes when the local header has a zip64 field);
- the Info-ZIP Unicode Path fields (0x7075) of both headers are equal, or
  both absent;
- the records follow each other without gap or overlap up to the central
  directory. Without this, twelve zero bytes after an empty record would pass
  for a descriptor, and whatever follows them would belong to no record.

A directory carries data when it declares a size, stores bytes uncompressed,
or holds a deflate stream that inflates to anything or ends before its stored
bytes do. An empty directory written deflated, as Java's `ZipOutputStream`
and `jar` write it, now imports (N12). A directory name ending in `\`,
which `zip` also treats as a directory, is compared with the file names too.

The private corpus has none of these disagreements: 106 archives and 7
unencrypted nested payloads, 1,517 records, no data descriptor, gap, overlap
or prefix. Through the release CLI the fixed code imports the 3 real projects as before (2 exit 0, 1 exit 2 for report errors in its own data, none refused), ingests the same 3 through `knx products ingest` and all 103 `.knxprod`, and leaves every corpus file unchanged. Evidence: 35 more tests in
`decoded_member_names.rs` (21 of the first 46 were RED before the fix), and
27 killed mutants (ALPHA_FINAL_GATES §11).

What remains:
- Bytes in front of the first record are accepted as a prefix
  (self-extractor stubs exist with both relative and absolute offsets), so a
  record nobody references can still sit there. The central directory is
  authoritative (APPNOTE), and every reader the re-check tried follows it; a
  streaming reader would see the extra record.
- Encrypted nested payloads of real projects were not part of the census.

**Update 2026-10-07 (AR18 re-check round 4, N14, MINOR): an empty
directory's payload is not checksummed.** Round 4 found every local/central
disagreement refused. One gap remains: a directory record with zero declared
size whose CRC is not zero, or which declares deflate but carries no stream
at all, is accepted. `unzip -t` reports both as damaged. The records agree
with each other, so no member is hidden or replaced, and no content is
lost. Round 4 judged it not blocking. It is left unfixed for the Alpha,
because any code change would reopen AR18 for another review round. It is a
candidate for the first post-Alpha hardening.
<!-- END retained catalogue record -->
