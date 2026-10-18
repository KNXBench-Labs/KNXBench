← Previous: [Why KNXBench exists](02-why-knxbench-exists.md) · [Manual index](../README.md)

# Project status

Read this chapter before you trust KNXBench with a project you care about.

## The version number

Every part of KNXBench carries its own version
([ADR-0018](../../adr/0018-program-versions-and-file-headers.md)). Since the
`v0.1.0-alpha.6` release (9 October 2026), application versions have been aligned.
The CLI, desktop, web frontend, server and `knx-mcp` carry `0.1.0-alpha.14`;
library crates keep their own numbers. The public [Alpha 14 pre-release](../../releases/alpha14.md)
was published on **18 October 2026**: AppImage, `knx-mcp` and the Docker Hub image
`knxbench/knxbench-server`. [Installation](04-installation.md) names the artifacts;
[source acceptance](../../releases/alpha14-acceptance.md) and
[publication verification](../../evidence/release-alpha14-2026-10-18.json) are separate.

Alpha 14 adds the parameter workspace with declared grids, block access and
separators, function choices below each object, note decisions before deletes,
plug-in data adoption, Functions in comparisons, vendor-ZIP product install and a
read-only diagnostic export. None of it unlocks new devices.
[Release notes](../../releases/alpha14.md) keep the exact boundaries together.
The manual and README screenshots show the published Alpha 14 server and
frontend.

**Before upgrading:** close the app and independently back up `.knxdb` files and
all Docker data, including the product database. Native/model **16** is unchanged;
the product database moves from **23** to **24** on first open, and Alpha 13
refuses a newer product database (read in its source, not executed). Alpha 11 and
earlier refuse native16; opening new Alpha 14 saves in Alpha 13 is not verified.
Returning means restoring independent copies, not pressing Undo. Update `knx-mcp`
with the application.

Colour/picture value codecs and Site grouping are not part of this release;
Functions are now project-editing tools, not device programming.

**Newer than Alpha 14 (source builds only, for now):** the product database moves on
to **25** ([ADR-0141](../../adr/0141-placements-and-enumeration-attributes-reach-columns.md)),
file-size bounds follow the storage ceiling instead of the older MiB limits
([ADR-0142](../../adr/0142-file-size-limits-follow-the-storage-ceiling.md)), and
legacy `.vd5` files with upper-case table names import instead of reading as empty. These arrive in a
release with the next alpha; until then the Alpha 14 bounds apply to the AppImage and
the Docker Hub image.

**Source documentation and a release asset are different snapshots.** Check the
running application's version and build commit before expecting a recently added
feature to appear in an older AppImage. A version label is not a time machine.

## What "alpha" means here

**KNXBench is Alpha software.** Concretely:

- Compatibility varies by project schema, manufacturer data and operation.
  [Known issues](../known-issues.md) explains the user-visible boundaries;
  [the ledger](../../status/LEDGER.md) is the detailed status record. Counting
  historical limitation headings is not a useful bug score.
- The web/Docker server's protection is one shared password and one session
  cookie. No user accounts, no roles and no per-user audit trail. With a password,
  it provides HTTPS by default using a self-signed certificate.
  Without a password it refuses to leave loopback at all.
  [Installation](04-installation.md) has the recipe and the caveats.
- Writing configuration to a real KNX device is limited to a device download
  verified on one device so far; address programming is refused for now and
  most devices still need another commissioning tool.

None of this means KNXBench is unusable — it is under active development and
the test suite is substantial — but it does mean you should treat it the way
you would treat any pre-1.0 engineering tool:

> **Note**
>
> Keep backups of every project you care about, independent of KNXBench's own
> `.knxdb` file. Expect gaps in ETS compatibility, expect UI rough edges, and
> do not point the bus-writing commands at a live installation unless you
> understand exactly what you are sending and to which address.

## Where to check the details

This manual keeps the day-to-day chapters short. Three chapters, written and
maintained separately from this one, carry the full, current detail:

- [Known issues](../known-issues.md) — the reader-facing list of what does
  not work yet or works only partially.
- [Implementation status](../implementation-status.md) — what is implemented,
  what is partial, and what is planned.
- [Ideas and roadmap](../ideas-and-roadmap.md) — where the project is headed.

If you want the underlying engineering record instead of the manual's
summary, the repository's own
[`KNOWN_LIMITATIONS.md`](../../KNOWN_LIMITATIONS.md) and
[`IMPLEMENTATION_STATUS.md`](../../IMPLEMENTATION_STATUS.md) are the source
those chapters draw from.

[Manual index](../README.md) · Next: [Installation](04-installation.md) →
