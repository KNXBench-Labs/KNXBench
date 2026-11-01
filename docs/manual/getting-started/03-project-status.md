← Previous: [Why KNXBench exists](02-why-knxbench-exists.md) · [Manual index](../README.md)

# Project status

Read this chapter before you trust KNXBench with a project you care about.

## The version number

Every part of KNXBench carries its own version
([ADR-0018](../../adr/0018-program-versions-and-file-headers.md)). Since the
`v0.1.0-alpha.6` release (9 October 2026), application versions have been aligned.
Alpha 19 is a pre-release with source cutoff 1 November 2026 and aligns the CLI, desktop,
web frontend, server and `knx-mcp` at `0.1.0-alpha.19`; library crates keep their
own numbers. Its [release scope](../../releases/alpha19.md) uses source cutoff
`858b0a24d`. [Installation](04-installation.md) names the Alpha 19 artifacts.

Alpha 19 adds `knx products inspect-distribution`, a read-only offline report on
OpenKNX and other manufacturer distributions that installs nothing and runs no
scripts. **Report problem…** opens one wizard that builds a reviewed, consented
diagnostic ZIP from operation recordings, reduced session-log, bus-monitor and
browser sections and other evidence; nothing is uploaded. Regular programming
refuses customer-adjustable declarations, omits only bytes proved inactive, can
report the run state after a download on exact masks and restores only when the
device answers with the serial number captured in the backup. Settings opens as a
center page. None of this is new hardware support, and the allocation mismatch
behind issues #30 and #22 is unresolved. [Release notes](../../releases/alpha19.md)
keep the exact boundaries together.
The manual and README screenshots show the published Alpha 19 server and
frontend, including step-by-step pictures of the new **Report problem…** wizard.

**Before upgrading:** close the app and independently back up `.knxdb` files and
all Docker data, including the product database and device backups. Native/store
and core model **16** are unchanged; the product database moves **30 → 31** on a
writable open, and Alpha 18 refuses it. Opening Alpha 19 files in Alpha 18 is
**not tested**. Alpha 18 also refuses device backups of format 3/4, activity-history
pages with the new support-read kind and System-2 journals with an
allocation-comparison record. Backups of format 1/2 no longer restore. Alpha 17
refuses product30; Alpha 11 and earlier refuse native16. Returning means restoring
independent copies, not pressing Undo. Update `knx-mcp` with the application.

**Earlier releases:** [Alpha 18](../../releases/alpha18.md) added the System-2
run result, final-step category, S2 chip and report, group-address CSV profiles and
German CLI reports. [Alpha 17](../../releases/alpha17.md) added the
UP 258D12 prestate observations and the regular-download checks. [Alpha 16](../../releases/alpha16.md) added cancellable
catalogue jobs, confirmed comparison-to-project adoption, carried original
packages and migrated product26 → 28. [Alpha 15](../../releases/alpha15.md) moved
file-size bounds to the storage ceiling
([ADR-0142](../../adr/0142-file-size-limits-follow-the-storage-ceiling.md)) and
imported legacy `.vd5` files with uppercase table names. V23–v25 column-promotion
residues remain open; larger bounds and stored property/binary attributes add no
write admission.

Colour/picture value codecs and Site grouping are not part of this release;
Functions are project-editing tools, not device programming.

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
