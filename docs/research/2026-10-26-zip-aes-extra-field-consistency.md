# ZIP WinZip AES extra-field consistency (26 October 2026)

## Question

[WP-16](../open-work/WP-16-zip-encryption-metadata.md) asks how project and
product ZIP admission treats WinZip AES (AE-x) metadata whose parts contradict
each other. Consistent AES entries are already refused as unsupported
encryption; this note is about the metadata *around* that refusal, not about
implementing AES decryption.

## Sources

- PKWARE [APPNOTE 6.3.10](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT),
  retrieved 26 October 2026, SHA-256
  `0b993022a7d320a0bf704e6980bea36fafd17a6066ab994db0a0c16278a50cd6` (byte-identical
  to the copy cited by the [bit-6 note](2026-10-24-zip-bit6-flag-consistency.md)).
  Section 4.4.5 lists method 99 as "AE-x encryption marker"; section 4.6.1 lists
  header ID 0x9901 as "AE-x encryption structure"; Appendix E: E.3 AE-x uses
  bit 0 to indicate encryption, E.4 the method field is set to 99, E.5 the actual
  compression method is stored in the 0x9901 extra field, E.6 version 1 stores
  the CRC, version 2 stores 0.
- WinZip, [AES Encryption Information](https://www.winzip.com/en/support/aes-encryption/),
  retrieved 26 October 2026 (HTML page, SHA-256
  `3a58f169a389c2e23235acab7098d177b08359448fc573106467d7ecabbb08c8`; the page is
  dynamic, so the hash identifies this retrieval only). It states that bit 0
  "must be set to 1 in each AES-encrypted file's local header and central
  directory entry", that method 99 in both headers is "used along with the AES
  extra data field", and that this field "is stored in both the local header and
  central directory entry". The data size is currently 7, "but vendors should
  not assume that it will always remain 7". Directory entries need not be
  encrypted; mixed archives are allowed.
- The `zip` crate as locked in this workspace (8.6.0, `src/read.rs`,
  `src/extra_fields/aex_encryption.rs`, `src/read/readers.rs`,
  `src/read/zip_archive.rs`), read on 26 October 2026. It reads only the
  *central* extra field when building its index. It parses every 0x9901 field
  it finds, independent of method 99 or bit 0, and overwrites the entry's
  compression method with the field's method; the last field wins. A field
  whose length is not 7, whose vendor is not `AE` or whose version/strength is
  unknown fails `ZipArchive::new`. Method 99 without a field also fails there.
  Opening an entry that carries a parsed field but no bit 0 fails later with
  `InvalidPassword`.

The sources are format evidence. They are not evidence about ETS archive
production, and nothing here is tested against a real AES-protected project.

## Measured behaviour before the change

A temporary probe on base `b3028bb4d` built hand-written archives (no
encryption library) and opened them with `Container::open` (outer layer) and
`Container::open_with_password` (nested protected payload):

| Shape | Outer | Nested payload |
| --- | --- | --- |
| bit 0 clear, method 0, field naming DEFLATE | refused, "provided password is incorrect" | **accepted**, stored bytes inflated as DEFLATE |
| bit 0 clear, method 0, field naming Stored | refused, "provided password is incorrect" | **accepted** |
| bit 0 clear, method 99, field | refused, "provided password is incorrect" | **accepted** as plaintext |
| field in local header only | **accepted** | **accepted** |
| field in central record only | refused, "provided password is incorrect" | **accepted** |
| bit 0, method 99, two fields | refused, password required | refused, unsupported AES |
| bit 0, method 0, field | refused, password required | refused, wrong password |
| plaintext directory carrying a field | refused, "provided password is incorrect" | **accepted** |
| consistent AES (bit 0, method 99, one equal field) | refused, password required | refused, unsupported AES |
| bit 0, method 99, no field | refused by `zip` | refused by `zip` |

The nested loop reads plaintext members through `by_index_raw` and decompresses
with the method `zip` reports, which a 0x9901 field rewrites; the raw method
and the encryption flag never enter that decision. A reader that follows the
raw method (or the local headers) sees different bytes from the same archive.
The outer refusals are fail-closed only by accident of `zip`'s password logic
and name no member.

## Decision

At the existing record-admission boundary of every layer (outer and nested
project containers, product packages, wrapper packages and their inner
packages), and before any member codec is opened, refuse by validated member
name:

1. local and central 0x9901 field lists that differ (the same rule as for
   Unicode Path fields);
2. any 0x9901 field on a record whose method is not 99;
3. method 99 with other than exactly one 0x9901 field;
4. method 99 without bit 0.

A record with bit 0, method 99 and exactly one equal field in both headers is
the consistent AE-x shape and keeps its existing unsupported-encryption
refusal. Field *contents* (length, vendor, version, strength) stay with `zip`,
which already refuses malformed ones when opening the archive; a future longer
field (the WinZip note above) stays refused as unsupported. Other unknown
extra fields remain accepted. Legacy EX-IM member handling keeps its own
method-99 refusal and is not widened.

This is stricter than the previous runtime for the shapes marked accepted
above. No real project or product file in the private corpus is expected to
carry such a shape; the package gate's corpus stage is the check of that
expectation, not this note. It passed: 155/155 selected corpus cases in the
integrated gate of `161e3b14b`.

The validated product directory records a conflict but only
`open_checked_archive` refuses it; the legacy EX-IM reader (raw method, its own
method-99 refusal) and the metadata-only baggage census do not consume
member bytes under a field-rewritten method. A pre-existing System-2
preparation path decodes a package member with plain `zip` before installation
admits it; installation still refuses afterwards and nothing is persisted.
That read-before-admission is outside this package.

## Exclusions

AES decryption, strong encryption (bit 6) semantics beyond the delivered bit-6
rule, minimum version-needed-to-extract validation, AE-1/AE-2 CRC rules for
encrypted data, private encrypted-corpus acquisition, password guessing and
live-device operations are excluded. No compatibility, certification or
release claim follows.
