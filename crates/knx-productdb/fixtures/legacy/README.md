# Synthetic legacy EX-IM fixtures

Everything in this directory was written for KNXBench. The manufacturer
"Marvin Test", its products, programs and numeric ids are invented, and no
byte comes from a manufacturer database. The public test password is
`marvin-synthetic`. It protects nothing, and it is **not** the password of
any real legacy file.

| File | What it is |
| --- | --- |
| `src-vd/MARVIN/ets.vd_` | Plaintext EX-IM product database (`H virtual_device`): 7 tables, 10 rows, one value wrapped over two `\\` continuation lines, a Windows-1252 `é`, and dash-only values |
| `src-pr/MARVIN/ets.pr_` | Plaintext EX-IM project export (`H project`) with an empty `application_program` table |
| `marvin-encrypted.vd4` | `src-vd` zipped and ZipCrypto-encrypted, using the layout of the real files (flags `0x0001`, no data descriptor, CRC check byte) |
| `marvin-plain.vd4` | `src-vd` zipped without encryption |
| `marvin-project.pr5` | `src-pr` zipped and encrypted like `marvin-encrypted.vd4` |

## Rebuilding

```sh
python3 build_fixtures.py
```

The script writes both plaintexts with CRLF line ends and archives them with
Info-ZIP `zip -X -D` (fixed mtime 2026-10-08 09:00 UTC). It then encrypts
copies with Info-ZIP `zipcloak -O`, which writes the observed layout.
`zip -P` would instead stream the member (bit 3 plus a data descriptor).
`zipcloak` reads the password only from a terminal, so the script feeds it
through a pseudo-terminal. The repository contains no ZipCrypto writer of
its own.

Encryption headers are random, so a rebuild changes the encrypted
archives' digests. The plaintexts and `marvin-plain.vd4` are reproducible.
Digests of the committed files (Info-ZIP Zip 3.0 / ZipCloak 3.0):

```text
c76b3e524cf38bddcc54364c2c0e9cfc4094376802b4ef0d3e46d08e4088c6c9  src-vd/MARVIN/ets.vd_
022b06aa0339e164dbee4642e99f08e8efdd4465bc881d750443f95b3b64388c  src-pr/MARVIN/ets.pr_
b49a7f6cd205adaf6d866f6ce5650e85ef31879a171cf4098079dd6c365ce797  marvin-plain.vd4
54f40f339ba9886584e6accd261f8a6f5e9c92ef29c5b203aec12e54e9a12aaf  marvin-encrypted.vd4
bd73cd8382def079028e6c14cf9deff1ab05a078e24cff73c03eba8eb8ab9991  marvin-project.pr5
```

`.gitattributes` turns off line-end conversion for every file here and
marks the archives and both plaintext payloads binary. The CRLF line ends
and the trailing space of the `K ` header line are data, so Git must never
rewrite them or report them as whitespace errors.

These fixtures test the cipher, the container rules and the grammar. They
are not evidence about any vendor's export. That evidence comes from the
ignored corpus test `tests/legacy_corpus.rs`.
