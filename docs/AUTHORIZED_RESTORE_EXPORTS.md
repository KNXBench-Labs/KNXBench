# Authorized ETS restore-point export regressions

This local test harness requires **authorized ETS `.knxproj` exports** of two or
more revisions of one project. It never reads ETS's live database or internal
`.restorepoint` files. Public synthetic harness tests are not ETS evidence;
the first real run is under [Run record](#run-record-8-november-2026).
Since the owner decision of 8 November 2026, ETS exports of fictional projects
made by the project count as authorised exports
([evidence routes](RESEARCH.md#evidence-routes)); they stay private like any
other export set.

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
  cargo test -p knx-app --test it \
  authorized_restore_exports::authorized_restore_point_exports_match_private_baselines_and_native_roundtrips \
  -- --ignored --exact --nocapture
```

Set the variable explicitly. Without it an explicitly invoked private test fails;
ordinary public CI reports it **ignored**, not a successful zero-input check.
Only the verified aggregate export count is printed, never private filenames,
individual fingerprints, project names or XML values.

## Read and integrity contract

The manifest is bounded to 1 MiB; each export to the storage ceiling of
1,000,000,000 bytes (ADR-0142; formerly 64 MiB), with existing container
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
symlink confinement. They do not exercise the harness on genuine exports; the
run record below does, for its stated scope only.

## Run record (8 November 2026)

First real run, under the owner's clean-room route. The project built a small
fictional project in ETS6 6.4.1 (schema 23: one catalogue switch actuator,
default topology) and exported it unprotected three times:

- revision A: one group address linked to one communication object, all
  parameters at their defaults;
- revision B: a second group address linked to a second object, and one
  parameter changed (which makes ETS show one more object);
- revision A′: revision A brought back through ETS's own project archive
  ("create backup in archive", then "restore") and exported again. Its project
  XML equals revision A apart from one added `ArchivedVersion` attribute.

The baselines come from the ETS user interface and from the raw XML, not from
KNXBench. Devices, group addresses, communication objects (as listed by ETS),
parameter instances, modules and the schema were counted that way. The
KNXBench-only diagnostic fields (`unknown`, `opaque`, `unsupported`, `errors`)
were observed once and then checked against the XML: every `opaque` difference
between revisions maps to a concrete retained attribute (one `Puid` per new
group address, one `ChannelId` per new object link, the archive's
`ArchivedVersion`). A separate reviewer recomputed the ETS/XML counts, the
opaque deltas and the file digests from the raw exports; the run record was
written only after that review. Format observations are recorded in
[research](research/2026-11-08-ets6-revision-exports.md).

The named ignored test passed on `ac636f22f` with three verified exports, in a
network-free namespace with scratch XDG directories. The input hashes were the
same before and after. A negative control that changed one expected parameter
count failed as required.

Scope: one ETS version and schema, unprotected exports, one manufacturer
product, one archive restore. This is not evidence for older ETS versions,
other schemas, protected projects or ETS's internal restore-point files, and
those exports and manifests stay private. Further revision sets go through the
same harness.

## Unblock condition

Met on 8 November 2026 for the scope above. For further sets: supply an
explicitly authorized, private export set (since 8 November 2026 this includes
the project's own exports of fictional projects) of distinct revisions of one
project, have its baselines reviewed independently, run the named ignored test
on the actual gated source, inspect its aggregate result and retain the private
evidence locally. This is separate from native KNXBench project history and
from the [selective import workflow](SELECTIVE_IMPORT.md).
