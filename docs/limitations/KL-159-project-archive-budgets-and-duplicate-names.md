# Project archive budgets and duplicate names

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-159)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

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
