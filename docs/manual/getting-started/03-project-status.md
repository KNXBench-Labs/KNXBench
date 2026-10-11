← Previous: [Why KNXBench exists](02-why-knxbench-exists.md) · [Manual index](../README.md)

# Project status

Read this chapter before you trust KNXBench with a project you care about.

## The version number

Every part of KNXBench carries its own version
([ADR-0018](../../adr/0018-program-versions-and-file-headers.md)). Since the
`v0.1.0-alpha.6` release (9 October 2026), application versions have been aligned.
The CLI, desktop, web frontend, server and `knx-mcp` now carry `0.1.0-alpha.9`;
library crates keep their own numbers. The public pre-release is
[`v0.1.0-alpha.9`](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.9), published on 11 October 2026: AppImage, `knx-mcp` and the Docker Hub
image `knxbench/knxbench-server`. See [Installation](04-installation.md).

The current screenshots show that published release, not a moving development
build. Alpha 9 brings named English/German DPT choices, display-only fallback
installation labels, clearer login/version and notification details, and better
import feedback. The device overview and restart-safe project history are also
available. [Release notes](../../releases/alpha9.md) keep the exact scope together.

**Before upgrading:** close the app and independently back up `.knxdb` files and
Docker data. Alpha 9 uses native schema **13**; Alpha 8 and earlier refuse upgraded
files. Returning to an older build requires your pre-upgrade backup. In-file
versions and Undo are not an independent backup. Update `knx-mcp` together with
the application.

The release includes foundations for instruction and specialised parameter
editors, but those new editors are **not mounted in the production workbench**.
Do not mistake a source component for a new tab in your downloaded app.

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
