# Protected project import

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-013)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 13. Password-protected projects: ZipCrypto (ETS4/ETS5) is decrypted, AES (ETS6) is still refused

**Limitation.** A `.knxproj` whose project part is nested as `<P-xxxx>.zip`
(IMPORT_EXPORT §2) can now be opened two ways:

* `Container::open` (no password) still refuses it by name
  (`ContainerError::PasswordProtected`), unchanged from before.
* `Container::open_with_password(bytes, password)` decrypts a schema < 21
  (ETS4/ETS5) project protected with **ZipCrypto**, given the right
  password. A schema ≥ 21 (ETS6) project protected with **AES** is still
  refused by name (`ContainerError::UnsupportedEncryption`), not
  attempted — that half of this limitation is unchanged.

Read only, both ways: nothing in this repository writes a ZipCrypto- or
AES-protected entry. `knx_secure::zipcrypto` exposes a `decrypt` function
and nothing that encrypts.

**What ZipCrypto is, plainly.** It is PKWARE's "Traditional Encryption",
specified in APPNOTE.TXT §6.0/§6.1 [D] — a three-key stream cipher from
1990, with a 1-in-256 false-accept rate on its own password check — about
1 in 128 as this repository uses it, since it tries both published
check-byte conventions — and a
known-plaintext attack (Biham & Kocher, 1994) that recovers the key from
a modest amount of known output, no brute force required. It is not
security by any current standard; the KNX Standard specifying it for
ETS4/ETS5 does not make it one. This code exists only to read a file
whose password the caller already has — see `knx-secure/src/zipcrypto.rs`'s
module docs for the full account, including both check-byte conventions
(PKZIP's own vs. Info-ZIP's streamed-entry variant) this implementation
has to try, because the `zip` crate's public API does not expose which
one a given entry used.

**Cause / evidence, updated.**

* **The AES/PBKDF2 key derivation (schema ≥ 21, ETS6+)** is specified in
  the KNX Standard itself, with the Standard's own test vectors: *The KNX
  Standard v3.0.0*, *Project Schema23 v01.00.00*, clause 4.2.4 "Password
  protection", p.64/64. `crates/knx-secure` implements exactly that
  derivation (`derive_knxproj_zip_password`) and its tests assert the
  exact Base64 output of all three of the clause's published vectors
  (`"a"`, `"test"`, and the non-ASCII third vector, recovered by
  rendering the source PDF page directly since every text-extraction
  path fails on it — see that test's own comment for the full account)
  — `[D]` (cited clause and page) and `[V]` (byte-exact). **Container
  decryption for this scheme is still not implemented** — AES needs the
  `zip` crate's `aes-crypto` feature (not enabled in this workspace) and,
  more importantly, a real AES-protected ETS6 project to verify against;
  neither exists here yet, so `Container::open_with_password` refuses an
  AES-protected nested payload by name rather than attempting it.
* **ZipCrypto (schema < 21, ETS4/ETS5) is now implemented and tested.**
  `knx-secure/src/zipcrypto.rs` hand-implements the cipher directly
  against APPNOTE.TXT v6.3.3 §6.1.3-§6.1.7 [D] (quoted verbatim in that
  module's docs, fetched 2026-09-14), rather than relying on the `zip`
  crate's own (also-writes-capable) decryption internals.
  `knx-etsproj`'s `Container::open_with_password` wires it into the
  container: it walks the nested payload's raw entries, decrypts each
  ZipCrypto-protected one, and decompresses the result (Stored or
  Deflated; a third method is reported, not guessed at). Tested against
  two synthetic container fixtures, both password `hunter2knx` and both
  generated with the independent Info-ZIP `zip` CLI — never by any code
  path in this repository: `crates/knx-etsproj/fixtures/zipcrypto-protected.knxproj`
  (nested entries Deflated) and `.../zipcrypto-stored.knxproj`, whose
  nested entry is Stored so that the CRC-32 gate below is reachable at
  all — against Deflated bytes a false accept fails to inflate and is
  reported before any CRC is compared. Plus a crypto-primitive-level
  fixture in `knx-secure/fixtures/`. All are
  synthetic, not extracted from a real ETS project, and that is an
  acceptable substitute *here*: ZipCrypto is a fully specified algorithm
  (APPNOTE.TXT), not an ETS-specific quirk, so a fixture built with a
  standard, independent tool exercises the same cipher a real ETS4/ETS5
  export would use. **What this does not verify:** whether a real
  ETS4/ETS5 export's nested payload matches this fixture's shape in every
  detail (entry layout, compression choices, check-byte convention in
  practice) — see §3 of `docs/COMPATIBILITY.md`, still listed as
  unverified against a real protected export.
* **A wrong password** is reported as `ContainerError::WrongPassword`,
  never a panic and never silently-wrong plaintext. ZipCrypto's own check
  byte only rules out 255/256 wrong passwords per convention, and this
  implementation tries both published conventions, so it lets roughly 1
  wrong password in 128 through — but the check byte is not the last
  gate. Every decrypted entry's decompressed bytes are checked against
  the entry's own declared size and CRC-32 from the ZIP central
  directory, and a mismatch there is reported as `WrongPassword` too,
  because after a check byte has already passed that is what it almost
  certainly is. A wrong password would have to survive a 1-in-128 check
  byte *and* forge a 32-bit CRC to be silently accepted. That residual is
  the ceiling ZipCrypto's design imposes, stated here rather than left
  implicit.

**Impact (updated 2026-10-04, AR08).** A ZipCrypto-protected (ETS4/ETS5)
project now imports end to end through the library, the application
service, `knx import --password-stdin` and `POST /api/project/import`
(optional `password`); see IMPORT_EXPORT.md §2 for the entry paths and
redaction tests. Verified against a synthetic fixture only, not a real
export. Since 2026-10-04 the Web UI asks for the password when the import
answers `422` `projectPasswordRequired` (or asks again on
`projectPasswordWrong`). It retries the same import with the password in
that request only and never stores or logs it; opening a `.knxdb` never
asks. Cancel leaves no project and no error. Still not covered: comparing against a protected file
(`knx diff`, `POST /api/project/diff`) and ingesting products from a
protected project (`knx products ingest`) take no password and refuse such
a file by name.

**A decrypted project keeps no protection, and says so.** The opaque
passthrough store (ADR-0006) snapshots `Container::entries()` and reads
every entry back through `Container::read`, which cannot tell a decrypted
entry from one that was never encrypted, and the original ZipCrypto
ciphertext is not kept. The import report therefore carries an
`unsupported` entry `project password protection (<P-xxxx>.zip, ZipCrypto)`
whenever `Container::decrypted_payload()` is set: the project is kept
unprotected from then on (memory, retained originals, saved `.knxdb`).
There is no `.knxproj` export to re-protect it (ADR-0028).

A wrong password that slips past the check byte and decrypts to an invalid
deflate stream is reported as `WrongPassword`, like a CRC mismatch. The
price: a damaged encrypted entry under the right password reads as a wrong
password too, and the error message says so.

An AES-protected (ETS6) project still cannot be imported at all — same as
before this change, and for the same reason: refusing cleanly beats a
decryption path nobody has run against a real encrypted file. The refusal
now reads the entry's *raw on-disk* compression-method field to recognise
AES, because the `zip` crate overwrites its own parsed method with the
entry's real underlying one the moment it sees a WinZip AES extra field
(0x9901) — a check against the parsed value never fires, and the fall-
through blames the user's perfectly correct password instead.

**Lifted when.**

* ZipCrypto decryption against a real export: **lifted 2026-10-12** for
  the public ETS4 and ETS 5.7 protected samples (xknxproject test
  resources), which decrypt and agree with xknxproject. Before that, every
  real protected export failed: real ETS writes the nested members bare
  (`0.xml`), the synthetic fixture had `P-0001/0.xml`. Nested names are now
  rooted under the project part (§177).
* AES container decryption is lifted when the `aes-crypto` feature is
  enabled, the decryption path is implemented the same way ZipCrypto's
  was, and a real password-protected ETS6 project is available to verify
  it against.
* The pipeline gap is lifted (AR08, 2026-10-04): a password reaches
  `import()` from the library, application, CLI and server, and a
  decrypted import reports its lost protection. The Web password dialog
  remains with the UI owner; `knx diff`/`products ingest` stay
  password-less.
<!-- END retained catalogue record -->
