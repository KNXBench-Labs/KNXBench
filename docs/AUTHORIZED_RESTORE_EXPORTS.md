# Authorized ETS restore-point export regressions

This local test harness requires **authorized ETS `.knxproj` exports** of two or
more revisions of one project. It never reads ETS's live database or internal
`.restorepoint` files. No authorized restore-point export set was supplied for
this delivery; the real historical regression remains an external prerequisite.
Public synthetic harness tests are not ETS evidence.

## Private setup

Keep the exports and manifest outside Git. The manifest is version 1, with
`authorizedEtsExports: true` and 2–16 `exports`. Each entry has:

- `file`: exact relative `.knxproj` path below the manifest directory;
- `sha256`: 64 lowercase hexadecimal characters, measured from that export;
- `expected`: exact independently reviewed baseline counts.

The required `expected` fields are `schema` (ETS XML namespace version),
`devices`, `groupAddresses`, `communicationObjects`, `parameters`, `modules`,
`unknown`, `opaque`, `unsupported` and `errors`. Missing or unknown fields refuse
the manifest. Do not invent baselines, normalize an invalid hash, substitute
zeroes for unknown values, or automatically rebaseline an importer change.
Require at least two distinct byte revisions and one stable project identity;
different bytes alone do not prove different topology or restore-point behavior.
The test intentionally does not claim revision chronology from filenames.

```text
KNXBENCH_RESTORE_EXPORT_MANIFEST=/private/exports/manifest.json \
  cargo test -p knx-app --test authorized_restore_exports \
  authorized_restore_point_exports_match_private_baselines_and_native_roundtrips \
  -- --ignored --exact --nocapture
```

Set the variable explicitly. Without it an explicitly invoked private test fails;
ordinary public CI reports it **ignored**, not a successful zero-input check.
Only the verified aggregate export count is printed, never private filenames,
individual fingerprints, project names or XML values.

## Read and integrity contract

The manifest is bounded to 1 MiB; each export to 64 MiB, with existing container
and XML ceilings still enforced. All baseline fields are required, export paths
must be distinct, and at least two digests must differ. Exact lexical paths
reject empty components, dot components, traversal, absolute/drive-like paths,
backslashes and NULs. Linux `openat2` holds the manifest directory and opens
regular inputs descriptor-relatively with beneath/no-symlink/no-magiclink
resolution. Leaf and ancestor symlink escapes refuse before import. Unsupported
platform/kernel confinement is a refusal, never an unsafe fallback.

Already captured and hash-verified bytes go through the same application
import/persist path as normal project import into disposable in-memory stores.
The harness checks exact model/report counts, native whole-model save/reload,
retained-byte digests/counts and original input readback. It never writes into
the export directory. The current harness accepts unprotected exports only;
password support or protected real fixtures are not claimed.

Public tests pin authorization, schema/count shape, distinct identities,
missing configuration, forbidden internal-file/path syntax, and leaf/ancestor
symlink confinement. Genuine exported revisions still need to exercise this
entire harness before historical compatibility can be claimed.

## Unblock condition

The project owner supplies an explicitly authorized, private export set made
from the required ETS restore-point revisions and independently reviews its
baselines. Run the named ignored test on the actual gated source, inspect its
aggregate result and retain the private evidence locally. This is separate
from native KNXBench project history and from the
[selective import workflow](SELECTIVE_IMPORT.md).
