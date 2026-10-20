# Project encrypted-directory admission

## Decision — 20 October 2026 (delivery acceptance pending)

Refuse a project directory record carrying ZIP general-purpose bit 0, in both
outer archives and directly opened nested project ZIPs. Report its validated
decoded path and the explicit reason `encrypted directory records are unsupported`.
Keep the existing `ContainerError::InconsistentRecord` admission-error variant;
the reason denotes an unsupported feature, not necessarily an invalid ZIP or a
wrong password. No new cipher, authentication or encrypted-directory support is
introduced. This is an explicit stricter KNXBench policy, not a universal ZIP or
ETS validity rule.

[Primary facts and original behavior](../research/2026-10-20-directory-encryption-header-validity.md)
were recorded and exercised before implementation. A directory's ciphertext
must not be treated as a plaintext empty DEFLATE stream or skipped as though its
encryption header had been checked. Refusal is simpler and more honest than a
minimum-length heuristic that cannot establish header/password validity.

## Enforcement and unchanged boundaries

- Apply the bit-0 refusal in the shared project directory check after existing
  local/central identity, bounds, descriptor and full-record layout validation,
  before raw payload/CRC/codec inspection. Use the already validated decoded path.
- Treat missing, short, genuine synthetic traditional headers and method-99/AES
  metadata consistently as unsupported encrypted directories; do not decrypt,
  validate cipher credentials or add AES handling on this path. Descriptor bytes
  remain outside the recorded compressed extent.
- Preserve plaintext stored/alternate empty-DEFLATE directories, Unicode/CP437
  identity, signed/unsigned descriptors, zero/truthful local values and current
  ZIP64 layout rules. Ordinary files retain lazy reads, supported ZipCrypto,
  missing/wrong-password and named AES refusal conventions.
- The independent product reader's existing encrypted-member refusal is unchanged.
  Product installation/wrapper rollback is not proved by project-container tests.
- Bit 6 without bit 0, AES extra-field semantic equality and encryption metadata
  on ordinary files are separate scope. Retained positive controls for bit-6-only
  empty directories do not endorse valid strong encryption.
- No private encrypted-corpus census or real encrypted-directory compatibility
  claim. Source-bound synthetic refusal proves the declared boundary only.

## Deliberately superseded original behavior

Original runtime `8ccb99b3` passes 94 named characterization cases; the controller
replayed all 94 with exact original receipts and 188 frozen fixture/control files.
The 73-package dependency graph matches the repository lock; 2,774 frozen source
files and 144 tracked local Rust/manifest identities were checked.

Of those cases, 62 describe bit-0 directories. Fourteen nested acceptance pins
will deliberately become refusals: absent Stored header and raw empty-DEFLATE
`03 00`, each with no descriptor, signed/unsigned descriptor and zero/truthful
local values; the two no-descriptor cases with a selected wrong public-fixture
password; and bits 0+6 with absent Stored header or raw `03 00`. Original bytes,
receipts and names remain retained, not rewritten as historical refusals.

The other 48 cases already refuse: 34 raw-preflight `InconsistentRecord` and
14 outer password-required `NotAZip`. Their reason/context deliberately becomes
the explicit unsupported-directory diagnostic. This is a diagnostic change, not
48 newly prevented admissions. Thirty-two other characterization cases remain
unchanged, including plaintext positives/refusals and ordinary encrypted files.
Do not add controller replay counts to unique test coverage.

## Required verification before delivery

1. Named original-source REDs for intended inner admission changes and exact
   diagnostic assertions; independent cipher control for genuine valid fixtures.
2. Candidate GREEN across encrypted shapes and retained plaintext/ordinary-file
   controls; compiled test inventory must equal actual named execution.
3. Application import refusal before project/product persistence on fresh and
   populated migrated destinations, quiescent source/store/sidecar snapshots and
   native reopen. Exercise password entry points and valid nested positive data.
   This does not establish general cross-store atomicity, live WAL or crash proof.
4. Remove the new guard in an isolated mutant, require intended named failures,
   restore exact source bytes and rerun GREEN. Preserve any failed attempts.
5. Independent full-diff review and the maintained committed-head scoped gate,
   valid proofs, frozen inputs and clear user-data guards before publication.
   No browser/full-workspace/release acceptance follows from this package.

## Local implementation evidence

The shared directory check now refuses bit 0 before raw payload/CRC inspection;
caller structural/layout validation remains first. Existing decoded-path
`InconsistentRecord` mapping and ordinary-file crypto behavior are unchanged.
The new core regression module executes 121 named cases: final original-source
RED has 72 intended semantic failures and 49 unchanged controls; the candidate
passes all 121. The owning integration suite passes 404 cases with sixteen
existing ignored cases (focused selection overlaps). All-targets core Clippy
passes. An initial AES ordinary-file control expected the wrong leaf context;
corrected final test bytes were rerun against original production before repair.
The earlier attempt is retained and uncredited.

Sixteen new application tests have eight intended original-source failures and
eight positive controls, through actual application password/import APIs. Four
inner cases previously admitted, four outer cases returned the older password-
required diagnostic. The controller candidate passes all sixteen and the owning
import-service selection (41 passes, three existing ignores; selections overlap),
plus all-targets Clippy for both affected crates. Tests verify fresh/populated,
already migrated and quiescent DELETE-mode stores, exact source/database/sidecar
snapshots, manufacturer/opaque witness readback and native reopen. They do not
prove live WAL, crash recovery or general cross-store atomicity. A receipt-parser
correction was evaluated against retained original logs without changing tests.

The header gate found both new test files lacked the required paragraph break
after their first sentence (157 absent versus the unchanged ceiling of 155).
The comment-only correction now passes the header gate at the unchanged ceiling.
Final-byte original-source controls reproduce the same 72 core/eight application
semantic failures, and parent candidate2 passes the owning suites and both-crate
Clippy again. The only test delta is one blank module-doc separator per new file.

In an isolated snapshot the guard-removal proof passes 121 baseline cases,
reproduces precisely 72 intended failures with 49 controls, then passes all 121
again after byte-exact restoration. Canonical source was never mutated. Initial
archive-mode setup and output-parser failures are retained but uncredited.
The mutation used pre-separator test comments; final-byte original RED and
candidate GREEN separately bind the header-compliant tests.

**Status:** controller core/application/header checks and isolated guard mutation
verified. Independent full-diff review accepted with zero Critical/Important
findings; its sole minor stale implementation-status sentence is corrected.
The parent verified all fifteen reviewed path identities before the docs-only
correction. Maintained committed-head delivery acceptance remains pending.
