# WP-16: empty ZIP directories must really be empty (Claude, 2026-10-19)

## Request

Alpha 15 package WP-16 (`docs/OPEN_WORK.md`): close KL-159's round-4 corner.
An empty directory record whose two headers agree on a nonzero CRC, or on a
deflate method with no complete stream, was accepted.

## Reproduction (RED before any production edit)

Synthetic archives from the hand-written writer in
`crates/knx-etsproj/tests/decoded_member_names.rs`, outer archive and
protected nested payload. On the unchanged code 5 of 9 new tests failed:

- stored empty directory, CRC `0xdeadbeef`: imported;
- deflated `03 00` directory, CRC 1 (also with Java's bit-3 descriptor): imported;
- deflated directory with no bytes: imported;
- truncated streams (`03`, `01 00 00 ff`): refused, but as "carries data";
- method 12 with no bytes: refused later by `zip` as an unsupported method.

The new `knx-app` test (seeded project and product database) failed as well:
the damaged archives imported.

## Writer shapes measured

A current JDK, run locally in scratch: `jar` writes directories stored with
the UTF-8 flag (the first with a `0xCAFE` extra field); `ZipOutputStream`
writes them deflated (`03 00`), flags `0x0808`, zeros locally and a signed
descriptor (CRC 0, sizes 2/0). Both shapes are control tests. `unzip -t`
reports the nonzero CRC and the missing stream as errors (it accepts the
truncated stream and bytes behind the stream).

## Change

`local_record::check_directory` replaces `directory_carries_data` and returns
the reason: declared size → "carries data"; stored with bytes → "carries
data"; deflate → complete empty stream that ends at the stored bytes, else
"carries data" (output or bytes behind) or "no complete compressed stream";
other methods with no bytes → "no complete compressed stream"; then CRC must
be 0. The call site maps the reason to `InconsistentRecord { path, reason }`.
The check still runs only after the layout check, in both layers.

## Consumers checked

Every project-container consumer opens through `Container::open`,
`open_outer` or `open_with_password`, which share `check_decoded_names`. File
members already refused bad CRC/no stream when read (pinned by a test). The
product-package reader (`knx-productdb`) reads every member through `zip`:
CRC/no-stream/truncated directories are refused there (without the member
name); a deflated directory with bytes behind its `03 00` stream installs.
Not fixed here (different reader, N12 class), recorded as open.

New open finding: a deflated *file* with bytes behind its stream end imports
in both layers of the project reader; the bytes belong to no member.

## Verification

Before the owner pause: fmt, clippy, crate tests, 6 killed mutants
(byte-exact restore) and a directory census (222 directory records in 231
archives, all stored, empty, CRC 0).

After the rebase onto the test-speed package (tests now in `tests/it/`):
focused modules green (`decoded_member_names` 62 passed, `import_service`
5 passed, 3 ignored corpus cases), xtask anchors, headers (ceiling
unchanged), layering, ledger and corpus gates clean, and the complete
private-corpus run: 155 of 155 passed in 9 targets. The change-scoped
package gate ran on the final commit; its receipt is in the package report.
Its first run failed `tool-tests`: the KL-159 update had been appended
inside the frozen retained catalogue record (hashed by the preservation
receipt). It now is a dated section above the record, which is byte-exact
to `origin/main` again.
