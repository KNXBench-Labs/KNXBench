← Previous: [Why KNXBench exists](02-why-knxbench-exists.md) · [Manual index](../README.md)

# Project status

Read this chapter before you trust KNXBench with a project you care about.

## The version number

Every part of KNXBench carries its own version
([ADR-0018](../../adr/0018-program-versions-and-file-headers.md)). On 2026-10-06 the CLI,
the desktop shell and the web frontend report `0.1.0-alpha.4`, the standalone server
`0.1.0-alpha.1`. There is no git tag and
no published release. The GitHub Actions workflow that would build and
publish a release AppImage exists but has never been run.

## What "alpha" means here

**KNXBench is Alpha software.** Concretely:

- As of 2026-10-06, `docs/KNOWN_LIMITATIONS.md` has 120 numbered headings,
  including eleven resolved or signpost entries. Of the 109 residual
  boundaries, five are K1 (critical); the
  [detailed triage](../../LIMITATION_TRIAGE.md) is the authority, not a count of
  headings interpreted as defects.
- The web/Docker server's protection is one shared password and one session
  cookie. No user accounts, no roles, no audit trail, and no TLS of its own.
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
