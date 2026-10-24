# ZIP bit-6-only flag consistency (24 October 2026)

## Evidence and bounded decision

The remaining [WP-16](../OPEN_WORK.md#wp-16--empty-directory-zip-integrity)
question is contradictory ZIP encryption flags, not implementing strong encryption.
The previous [directory investigation](2026-10-20-directory-encryption-header-validity.md)
records bit-6-only project directories as accepted. The
[encrypted-directory policy](../contracts/project-encrypted-directory-admission.md)
explicitly left that question open.

PKWARE [APPNOTE 6.3.10](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT),
section 4.4.4, was retrieved directly on 24 October 2026 (SHA-256
`0b993022a7d320a0bf704e6980bea36fafd17a6066ab994db0a0c16278a50cd6`).
It requires general-purpose
bit 0 when bit 6 is set. It also specifies minimum extraction versions for strong
and AES encryption; this package neither implements those schemes nor claims to
validate all their metadata. The source is format evidence, not evidence of ETS
archive production or cipher interoperability.

**Admitted repair:** refuse an entry with bit 6 set and bit 0 absent at the existing
ZIP admission boundary, for both files and directories. Apply that rule to outer
and directly opened nested project containers, product packages and the product
wrapper's actual archive consumers. Keep existing typed error classes and a named
member diagnostic. Local/central flag disagreement remains a structural refusal;
matching flags alone do not establish valid encryption metadata.

This is stricter than the previous runtime for contradictory flags. In particular,
the four retained `outer_bit6_only_*` / `nested_bit6_only_*` empty-directory
acceptance pins in `decoded_member_names/encrypted_directories.rs` are deliberately
superseded. Preserve their original-source baseline evidence separately from the
new refusal regressions; do not rewrite history as though they always refused.

## Acceptance contract

- Observe final refusal fixtures fail semantically against the unchanged original
  production implementation before changing it. Distinguish unchanged controls
  from newly refused admissions and from changed diagnostics.
- Exercise ordinary files and directories, Stored and DEFLATE shapes, outer and
  nested consumers, and validated member-name context where reachable.
- Keep plain valid archives, existing directory CRC/stream/layout checks and
  ordinary supported ZipCrypto behavior unchanged. Bit 0 plus bit 6 retains its
  existing unsupported-encryption behavior; it is not admitted as a new cipher.
- Exercise actual application/product installation entry points with populated
  destinations and verify refusal does not change source or persisted data.
  Quiescent SQLite byte/readback checks are not crash, live-WAL or general
  cross-store atomicity proof.
- Freeze the corrected source/tests for independent review and the maintained
  change-scoped package gate. Require nonempty intended tests, actual zero exits,
  complete ordered stages, stable inputs and clear user-data guards before delivery.

## Explicit exclusions

AES extra-field semantic equality, minimum-version validation for otherwise
unsupported encryption, strong-encryption implementation, private encrypted-corpus
acquisition, password guessing and live-device operations are excluded. Legacy
EX-IM selected-member handling already has its own bit-6 refusal and is not widened
by this ZIP repair. No new compatibility, certification, hardware or release-artifact
claim follows from this package.

## Execution status

Implementation and focused original-source regression evidence are verified.
The shared project container reports `InconsistentRecord` with the decoded member
name; the product checked-archive boundary reports a named `InvalidZip`, preserving
wrapper and inner-leaf context. Both reject exactly `(flags & 0x0041) == 0x0040`.
The product boundary retains flags from its structurally checked entry rather than
reinterpreting them through a second unvalidated header walk.

The four unchanged historical project-directory positives passed before their
expectations were deliberately superseded. Final test bytes against original
production compiled and produced 33 intended semantic failures with nine passing
controls. Corrected production passes the disjoint selected suites:
`decoded_member_names` (286), `wrapper_packages` (61), and application
`import_service::encrypted_directory` (20). These are 367 selected tests, not an
addition of overlapping focused/owning runs. The controller checked source and
retained evidence hashes, compiled-discovery/execution identities, original
production bindings, unchanged tests, and final source stability.

Actual import/install APIs exercise populated destinations, native reopen,
quiescent database/sidecar bytes and unchanged source bytes; an invalid inner
package also rolls back an earlier wrapper package. Existing plain, bit-0-plus-6
and ordinary ZipCrypto controls pass. Three affected crates pass all-targets
Clippy. Earlier product/application Clippy failures and their enumerate-loop
correction remain retained, not relabelled as original successes. Runtime tests
used isolated homes and a loopback-only network namespace; no hardware was used.

Independent read-only whole-feature review accepts with zero Critical/Important findings;
the controller verified the review artifacts and unchanged source/test identities.
Delivered on `main` as `daef61b74` (feature `9c7d0c122`). The maintained integrated
14-stage change-scoped gate passes: 4,732 Rust tests across 33 binaries (216
ignored), 155/155 selected corpus cases, complete ordered proofs, frozen inputs
and empty touched/unchecked user-data lists. Exact live remote readback agrees.
The claim is released; these are integrated source results, not release artifacts.
Focused tests are not final integrated, release or real encrypted-corpus acceptance.
