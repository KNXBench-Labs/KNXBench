← Previous: [Why KNXBench exists](02-why-knxbench-exists.md) · [Manual index](../README.md)

# Project status

Read this chapter before you trust KNXBench with a project you care about.

## The version number

Every part of KNXBench carries its own version
([ADR-0018](../../adr/0018-program-versions-and-file-headers.md)). Since the
`v0.1.0-alpha.6` release (9 October 2026), application versions have been aligned.
Alpha 15 aligns the CLI, desktop, web frontend, server and `knx-mcp` at
`0.1.0-alpha.15`; library crates keep their own numbers. Its
[release scope](../../releases/alpha15.md) uses source cutoff `21b9d46f`.
[Installation](04-installation.md) names the intended Alpha 15 artifacts.

Alpha 15 adds readable captions (#26), device-empty topology filtering (#25),
catalogue identity/cleanup and explicit catalogue-open errors; File-menu and
control feedback, resizable/zoom-aware tables and panes, storage metrics and
monitor DPT reasons; differences-first comparison marks, System labels and
complete response-local module identity. Product data gains typed placements,
unknown-row repair and `SuffixText` units. The new offline System-2 preview and
CLI-only declared-memory diagnostic profile do not unlock programming.
[Release notes](../../releases/alpha15.md) keep the exact boundaries together.
The manual and README screenshots still show the published Alpha 14 server and
frontend; media was not refreshed for this source update.

**Before upgrading:** close the app and independently back up `.knxdb` files and
all Docker data, including the product database. Native/model **16** is unchanged;
the product database migrates **24 → 26 on writable open**. Read-only CLI catalogue
readers do not migrate its on-disk file. Older builds refuse product26 by their version check
(source-established, not an executed downgrade test). Alpha 11 and earlier refuse
native16; opening new Alpha 15 saves in Alpha 14 is **not verified**, particularly
with the larger storage bounds.
Returning means restoring independent copies, not pressing Undo. Update `knx-mcp`
with the application.

Colour/picture value codecs and Site grouping are not part of this release;
Functions are now project-editing tools, not device programming.

**Since Alpha 15:** file-size bounds follow the storage ceiling instead of the
older MiB limits ([ADR-0142](../../adr/0142-file-size-limits-follow-the-storage-ceiling.md)),
and legacy `.vd5` files with uppercase table names import instead of reading as
empty. Older Alpha 14 artifacts keep their old bounds. V23–v25 column-promotion
residues remain open; v26 repairs its own suffix promotion, not every earlier
backfill. Larger bounds and stored property/binary attributes add no write admission.

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
