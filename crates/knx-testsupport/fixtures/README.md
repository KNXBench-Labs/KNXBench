# knx-testsupport fixtures

## `zipcrypto-minimal.knxproj`

The project of `minimal_knxproj_bytes()` with its project part protected the
way ETS4/ETS5 protect a project: `P-0001/0.xml` and `P-0001/Project.xml`
sit in a nested `P-0001.zip` whose entries are ZipCrypto-encrypted and
deflated. Password: `ar08-Hunter-Secret` (`ZIPCRYPTO_MINIMAL_PASSWORD`), a
public test value. Built on 2026-10-04 with Info-ZIP Zip 3.0, never by this
repository's code (it has no encryption code at all):

```sh
# inner/P-0001/{0.xml,Project.xml} and outer/{P-0001.signature,knx_master.xml,
# M-0001/M-0001_A-1.xml} hold the byte literals from src/lib.rs
cd inner && TZ=UTC touch -t 202610040000 P-0001/0.xml P-0001/Project.xml
zip -q -X -D -P 'ar08-Hunter-Secret' ../outer/P-0001.zip P-0001/0.xml P-0001/Project.xml
cd ../outer && TZ=UTC touch -t 202610040000 knx_master.xml M-0001/M-0001_A-1.xml P-0001.signature P-0001.zip
zip -q -X -D -0 ../zipcrypto-minimal.knxproj P-0001.signature P-0001.zip knx_master.xml M-0001/M-0001_A-1.xml
```

Synthetic, not a real ETS export: it proves the cipher and the import path,
not that every real ETS4/ETS5 protected export has this exact layout.
