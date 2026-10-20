# Directory encryption-header investigation (20 October 2026)

## Scope and evidence level

The remaining [WP-16](../OPEN_WORK.md#wp-16--empty-directory-zip-integrity)
question is directory encryption-header validity, not ordinary-file DEFLATE
extent consumption. The latter is delivered under the
[project-file contract](../contracts/project-file-stream-integrity.md).
The research itself changes no production admission rule, cipher, dependency,
credential plumbing or persistence behavior. The subsequent
[admission contract](../contracts/project-encrypted-directory-admission.md)
chooses explicit bit-0 directory refusal; implementation remains pending.

[V] A read-only audit of runtime revision `8ccb99b3` (unchanged through claim
`2a4b60b6`) was independently checked against twelve source/test/manifest file
hashes. The table below records source traces against that revision. A separate
isolated original-runtime characterization and controller replay now confirm
94 named cases. No private encrypted input or real credential was accessed.

## Primary references

[D] Retrieved directly on 20 October 2026:

- [PKWARE APPNOTE 6.3.10](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT),
  revised 1 November 2022: sections 4.3.8–4.3.9 separate encryption headers,
  payload and descriptors. Sections 6.1.3–6.1.6 describe the traditional
  twelve-byte encrypted header and its password-dependent check. Section 4.4.4
  requires bit 0 and extraction version at least 50 when strong-encryption bit 6
  is set. These facts do not establish an ETS encrypted-directory layout or a
  requirement to support one. A descriptor cannot replace an encryption header;
  twelve present bytes alone cannot establish successful password validation.
- [WinZip AES format](https://www.winzip.com/en/support/aes-encryption/): the
  encoded extent includes salt, password-verification bytes, ciphertext and an
  authentication code. Raw method 99 and extra field `0x9901` describe a different
  scheme; the traditional twelve-byte rule must not be applied to it. This is
  format reconnaissance, not implementation or legal clearance for AES support.

[V] Snapshot SHA-256 values: APPNOTE
`0b993022a7d320a0bf704e6980bea36fafd17a6066ab994db0a0c16278a50cd6`;
WinZip HTML
`3a58f169a389c2e23235acab7098d177b08359448fc573106467d7ecabbb08c8`.
The selected cached `zip` 8.6.0 reader sources match their original crate archive,
whose SHA-256 matches `Cargo.lock`. An initial attempt to use an absent local
checksum sidecar provided no integrity verdict; the archive comparison does.

## Independently checked source boundaries

[V] Paths refer to the runtime revision above:

| Boundary | Source observation, not a test verdict |
| --- | --- |
| Project directory preflight | `crates/knx-etsproj/src/container/local_record.rs:146–176` tests raw method, payload and zero CRC without consulting encryption flags. `container.rs:994–995` passes the on-disk byte slice, not decrypted bytes. |
| Outer project inventory | `container.rs:591–595` opens a normal reader before skipping a directory. `zip` 8.6.0 `src/read/zip_archive.rs:560–567` refuses encrypted members when no password is supplied. The password API does not add outer-member decryption. |
| Direct nested project ZIP | `container.rs:390–395` skips raw directory entries before encryption handling at `410–520`. A bit-0 stored-empty record, or bit-0 raw empty-DEFLATE bytes `03 00`, can therefore reach the skip without a cipher-header check if preceding structural checks admit it. The short-header skip is confirmed by the synthetic results below. |
| Genuine encrypted empty directory | Traditional ciphertext is currently subjected to the raw plaintext directory check. Stored encryption-header bytes count as data; DEFLATE ciphertext is interpreted as DEFLATE. Neither establishes encrypted-directory validity. |
| AES metadata | The dependency parses `0x9901` and can replace the effective compression method (`zip` `src/read.rs:680–688`). Directory preflight uses the raw method instead. A nested directory never reaches the ordinary-file named AES refusal at `container.rs:472–478`. |
| Product reader | Independent implementation: `crates/knx-productdb/src/package.rs:2579–2580` refuses `file.encrypted()` before reading directory payloads. The wrapper uses that reader for its outer archive and recognized direct inner packages. It is not the project `Container`. |
| Strong-encryption flag alone | No dedicated semantic bit-6/bit-0 consistency check was found on these paths. A bit-6-only record is not evidence of valid strong encryption. Legacy VD/PR selected-member rules are separate. |

[V] Project local-record checks accept signed/unsigned descriptors, zero or
truthful local values and existing ZIP64 widths. Product descriptor rules are
narrower; project support must not be attributed to product packages. Local and
central flags/raw methods are compared, but only Unicode Path extra-field
bodies receive semantic equality checking. This investigation does not silently
extend those checks to AES or strong-encryption metadata.

## Original-runtime characterization

[V] The standalone harness discovered and executed 94 named tests, all passing
on unchanged runtime `8ccb99b3`. The controller independently rehashed the complete
evidence, replayed the same executable into a separate receipt directory and
required exact per-case result equality. Counts are 94 unique cases, not 188
because of the replay. Source binding covers 2,774 files across 73 dependency
packages, 144 tracked local Rust/manifest files, and 188 frozen fixture/control
files. Registry versions and checksums match the repository lock.

| Original result | Outer cases | Direct nested cases |
| --- | ---: | ---: |
| Accepted | 12 | 28 |
| Directory carries data (`InconsistentRecord`) | 11 | 11 |
| No complete raw directory stream (`InconsistentRecord`) | 8 | 8 |
| Password required (`NotAZip`) | 14 | 0 |
| Wrong password on ordinary encrypted file (`WrongPassword`) | 0 | 2 |

[V] Bit-0 Stored entries with no header and bit-0 raw `03 00` directory payloads
are accepted and skipped only in the direct nested layer, even for the selected
wrong public-fixture password. Independent dependency decryption refuses these
with `UnexpectedEof`. Genuine synthetic traditional Stored/DEFLATE empty
payloads independently decrypt successfully with the correct public credential,
including all four signed/unsigned descriptor × zero/truthful-local shapes;
the production directory preflight instead refuses their raw ciphertext.
Stored one-/eleven-byte controls refuse as directory data in both layers.

[V] AES cases contain valid metadata only, not authenticated ciphertext: raw
method 99 with AE-2/vendor AE/strength 3/underlying Stored and zero or 28 placeholder
bytes. They fail raw preflight; the independent reader refuses AES decryption
because its crypto feature is disabled. Bit-6-only empty Stored/DEFLATE shapes
remain accepted in both layers. Bits 0+6 retain the outer-versus-inner split for
the short-header controls. None establishes strong-encryption support.

[V] Every admitted container reads both exact required XML payloads and excludes
the directory from inventory. Four ordinary-file controls retain exact correct-
password output or typed wrong-password refusal for one selected negative
credential. Neither a check byte nor these controls establishes authentication.
No product-runtime, persistence or real encrypted-project compatibility claim.
No failed fixture correction was needed.

## Decision and remaining delivery work

The [explicit refusal contract](../contracts/project-encrypted-directory-admission.md)
was recorded after original-runtime evidence and before production changes.
It deliberately supersedes fourteen direct-inner acceptance pins and changes
48 existing refusal diagnostics. The other 32 original cases are retained
unchanged. A raw encrypted directory is refused as unsupported rather than
silently skipped or presented as a validated plaintext directory.

Implementation, original-source regression REDs, candidate GREEN, application
preservation checks, independent review and maintained delivery acceptance remain
pending. No directory decryption, new AES, private encrypted-corpus acquisition
or broader strong-encryption metadata policy is included. The earlier maintained
product note is
[product directory stream integrity](2026-10-20-product-directory-stream-integrity.md).
