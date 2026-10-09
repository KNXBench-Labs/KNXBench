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
| `src-vd-program/MARVIN/ets.vd_` | Plaintext product database with one application program (ADR-0094 L2): program 300 "Improbability Drive" of manufacturer 4242 (`M-1092`), pages, grouped and union parameters, two memory-less parameters at address 0, enumerations, conditional visibility, three communication objects, translations including group members, a functional-entity catalog, a twenty-step `s19_block` load procedure with base images and masks (L4), and a `device` table whose secret-class `DEVICE_BCU_PASSWORD` value (invented, over a continuation line) must be withheld |
| `marvin-program-plain.vd4` | `src-vd-program` zipped without encryption |
| `marvin-program.vd4` | `src-vd-program` zipped and encrypted like `marvin-encrypted.vd4`; its plaintext is byte-identical to `marvin-program-plain.vd4`'s |
| `marvin-installer.vd5` | The installer-tree layout of the real Siemens `.vd5` (ADR-0094, VD5): an invented 14-byte mask image `Program Files (x86)/Common Files/MARVIN sc/MASK/mask4242.bin`, then `src-vd-program`'s payload as `Program Files (x86)/Marvin/Database/@PDB/ets.vd_`, both encrypted like `marvin-encrypted.vd4`. The tree is staged in a temporary directory, not committed |

## Rebuilding

```sh
python3 build_fixtures.py           # the L1 fixtures
python3 build_fixtures.py program   # the program fixtures (L2)
python3 build_fixtures.py installer # the installer-tree .vd5 (VD5)
```

The commands are separate so that rebuilding one set leaves the other
sets' random encryption headers, and so its committed digests, unchanged.

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
2efc1e2a6f76c9b9f5247ba5c75bd1731b5120ac5da1c9c51b8e6cc23894cd6b  src-vd-program/MARVIN/ets.vd_
1ec84cc1e92f401d0c897c342cb41198a7dac924f3b657dac744e84a5c113201  marvin-program-plain.vd4
9b29788abfb4be5545342afd7a0537b486264e2d105bb2391e0febb9fa71e34f  marvin-program.vd4
327db431912529831bab64b4622e81136d76e780da3e322d51c943de2571a25b  marvin-installer.vd5
```

`.gitattributes` turns off line-end conversion for every file here and
marks the archives and both plaintext payloads binary. The CRLF line ends
and the trailing space of the `K ` header line are data, so Git must never
rewrite them or report them as whitespace errors.

These fixtures test the cipher, the container rules, the grammar and the
mapping into the product database. They
are not evidence about any vendor's export. That evidence comes from the
ignored corpus test `tests/legacy_corpus.rs`.
