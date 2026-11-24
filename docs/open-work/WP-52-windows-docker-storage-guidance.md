# WP-52 — Windows Docker storage guidance and safe migration

Owner request: 24 November 2026, approved naming-feedback interview decisions
Q3 and Q18.
Status: Planned: awaiting implementation Go
Recorded only, not claimed, no implementation Go.
Blocked by: None (WP-51 and WP-52 are independent).

## Source report and evidence limits

The reporter alone measured approximately 3 minutes with a Windows bind mount
versus approximately 7 seconds with a named volume for a 240 MB database, and
reported approximately 28 MB/s versus 2.7 GB/s. These are attributed observations,
not an independent reproduction, a verified universal root cause, a controlled
benchmark or a performance guarantee. They do not establish that Docker itself
is universally slow or that SQLite/application code needs rewriting. The exact
cause and generality are unverified; no diagnostic investigation is authorised
by this package.

The finished 24 November interview authorises recording this plan only. Source
observations were bounded read-only inspection; the anchors below were checked
against current source at `8ab381831afffc42c25326747bb6775c5ba9e90f`.
No Docker, VM, toolchain, database or runtime operation was performed for intake.
Private report material and storage contents are not reproduced.

## Requested outcome

Improve the existing Windows Docker Desktop guide with a clear, evidence-bounded
explanation of storage choices and a safe, reversible path for an existing
Windows bind-mount installation to move both data and configuration into named
volumes. Do not create a second Windows installation guide. New-installation
advice already uses named volumes; distinguish it from migration of an existing
installation, preserving projects, product database, configuration, secrets,
authentication and runtime options.

## Existing verified base

All links are repository-relative anchors at the recorded base:

- [Generic installation routes Windows readers to the existing guide](../../docs/manual/getting-started/04-installation.md)
  (line 162), explicitly identifying its following commands as a Linux-host example.
- [Existing pinned image tag](../../docs/manual/user-guide/windows-docker-desktop.md#4-download-the-image-and-choose-your-login-password)
  (line 115) and [new-installation checks/volume creation](../../docs/manual/user-guide/windows-docker-desktop.md#5-create-persistent-storage-and-the-container)
  (line 157) already use named volumes; creation arguments (line 176)
  mount `knxbench-data` at `/data` and `knxbench-config` at `/config`, with
  hostname, loopback-published port and authentication settings.
- [Storage inventory and retention](../../docs/manual/user-guide/windows-docker-desktop.md#7-understand-storage-and-day-to-day-starting)
  (line 232) describe projects, product database, TLS material and the separate config-root
  key. Both roots must remain paired; replacing only one is insufficient.
- [Stopped paired backups and restore](../../docs/manual/user-guide/windows-docker-desktop.md#8-back-up-and-restore-both-storage-locations)
  (line 274) already copy the whole roots, preserve originals and use empty new restore
  volumes; the guide explicitly says its file-restore procedure has no Windows
  runtime verification yet. This is a documentation basis, not accepted migration
  execution evidence.
- [Safe recreation/update](../../docs/manual/user-guide/windows-docker-desktop.md#9-update-by-recreating-safely)
  (line 327) preserves custom options and warns that returning to an older image after
  schema upgrades needs matching pre-upgrade backups. Migration must not silently
  become a version upgrade.

## Authoritative owner decisions

These Q numbers belong to the 24 November naming-feedback interview.

| Decision | Binding requirement |
| --- | --- |
| Q3 | An independent, small Docker documentation package; no inferred SQLite code change or performance promise. |
| Q18 | Record two independent packages: the complete naming wizard and this targeted Docker-guide extension with safe migration. Only link WP-50; do not duplicate its native Windows work. |

## Bounded internal stages and actual status

Every stage is **Planned: awaiting implementation Go**. This intake changes no
README, installation manual or operational command recipe.

| Stage | Planned work | Actual status |
| --- | --- | --- |
| 52.1 | Recheck the guide and supported consistent-backup methods; gather official Docker and SQLite evidence for public technical facts at execution time, recording versions, source dates and actual URLs then | Planned: awaiting implementation Go |
| 52.2 | Extend the existing Windows guide with bounded storage advice, attributed measurement limits, prerequisites, paired backup/copy, explicit source/destination guards, validation and rollback; keep generic installation linking this guide | Planned: awaiting implementation Go |
| 52.3 | Review and, only with appropriate execution authorisation, validate the documented procedure using disposable fictional projects/product data/config/auth; record what was actually checked and all runtime limits | Planned: awaiting implementation Go |

### Required migration procedure design

The later guide extension must provide a reviewable procedure with these checks:

1. Inventory the existing installation privately: exact image version/digest,
   source data/config mount identities and types, hostname, internal/published
   ports, TLS/authentication mechanism and custom options. Distinguish a Windows
   host bind path from a container path or named volume. Do not paste inspection
   output, real passwords, hashes or keys into chat or tracked files.
2. Save work, stop/quiesce the server and any other writers, verify shutdown and
   keep them stopped through paired backup and copy. Use an existing supported
   backup method or a verified consistent filesystem copy; never copy just an
   active SQLite main file and assume consistency. Account for all SQLite-related
   state/sidecars as applicable to the verified method, and every file in both
   roots, including hidden files and key/TLS material. Do not invent checkpoint
   commands, change journal modes or rewrite databases as a shortcut.
3. Independently back up **both** original data and configuration outside the
   destination storage before migration. Check success and restorability of the
   matching pair; retain runtime options and the exact original image privately.
   Preserve necessary permissions and protect secrets as the existing guide
   requires. Failure of either backup stops the procedure.
4. Identify source and destination explicitly before each copy. Use unused,
   verified empty destination volumes and a read-only source where supported by
   the chosen copy method. Guard against mistyped/missing source directories,
   implicit empty sources, reversed mounts, source/destination volume swaps,
   same-source-and-destination copies, existing nonempty destinations and
   empty-overwrite. Stop on uncertainty; do not merge unrelated installations.
   Copy complete data and config roots, verify results and retain originals.
5. Recreate using the **same pinned version or digest**, both correct destination
   mounts, the same hostname, internal/published ports, authentication and custom
   settings. Do not fold in an upgrade or reset configuration. All host commands
   must be coherent PowerShell (including error handling and quoting); any shell
   command inside a helper container must be clearly separated and labelled.
   Pin helper image versions/digests too. Keep the existing secret-entry method;
   examples use fictional placeholders and never prescribe sharing a real secret.
6. Reopen and verify projects, product database, configuration, TLS expectations
   and authentication, not just container startup. Validate using disposable
   fictional content, including a saved project reopened after restart, known
   product records, a recognisable config setting and successful/failed login.
   Document exactly which cells were exercised; unexecuted Windows checks remain
   unverified. No real account or bus is needed for these checks.
7. Provide rollback before cutover: stop the replacement, reattach the preserved
   original pair with the original image/options and verify reopening/login.
   Failed validation must not delete or overwrite originals/backups. Do not
   retire original storage until explicit user verification; keep any retention
   choice separate from the copy. If later changes upgrade schemas, rollback
   requires matching pre-upgrade backups, not an older image on migrated files.

The implementer must substantiate technical statements about Docker storage and
SQLite consistency with official Docker/SQLite documentation at execution time.
This plan intentionally provides no guessed external URLs or unverified public
claims. The report's timings may motivate guidance, but cannot be presented as
expected performance on another machine.

## Acceptance matrix (planned, unchecked)

No command or runtime acceptance below has been performed for this intake.

| Accepted | Case | Observable result |
| --- | --- | --- |
| [ ] | Existing guide integration and generic installation link | One Windows Docker guide covers new installations and a distinct existing bind-mount migration; no duplicate guide or claim that named-volume advice was previously absent. |
| [ ] | Reported timings and technical explanation | Numbers remain reporter-attributed with size and evidence limits; no universal cause, guaranteed speedup or “Docker is slow” claim; public technical facts cite checked official Docker/SQLite evidence. |
| [ ] | Command review | Coherent PowerShell host syntax, clearly labelled container shell syntax, pinned images, consistent mounts/ports/hostname/auth/custom options, explicit error stops; no secret in examples or requested public output. |
| [ ] | Backup/copy with fictional paired roots | Writers stopped, complete consistent state copied by supported method, both backups retained and checked; SQLite-related state and hidden config/TLS/key files accounted for, permissions preserved. |
| [ ] | Wrong/missing/empty source, swapped/equal mounts, occupied destination or failed copy | Procedure stops before destructive action; no accidental empty-overwrite, mixing or mutation of originals. |
| [ ] | Disposable fictional migration and restart | Saved project reopens, expected product records/configuration survive, TLS/authentication remain usable with both mounts; record actual environment and limits without claiming an unrun Windows result. |
| [ ] | Validation failure and rollback rehearsal | Original image/options and original data/config pair reopen correctly; originals/backups remain intact throughout, including after failed cutover. |
| [ ] | Version boundaries and documentation review | Storage migration does not silently upgrade; schema rollback needs matching backups; incomplete runtime verification is explicit and no broader support is claimed. |

## Exclusions and coordination

No live Docker/container/VM operation now, database rewrite, performance
investigation, benchmark campaign, native Windows implementation, toolchain/SDK
operation or bus access. Do not edit the README/manual for this intake. No new
Windows-app ticket, support claim or change to foreign package plans.

[WP-51](WP-51-empty-project-object-name-wizard.md) is independently deliverable.
[WP-50](WP-50-windows-desktop.md) tracks the separate native Windows desktop
work; this link adds no Windows support claim or dependency. Coordinate only
with the existing guide's maintainers and storage/backup contract; neither
WP-50 nor the naming wizard is a prerequisite.

## Open decisions

None at owner level: Q3/Q18 are approved; implementation Go remains absent.
Source-derived implementation details still to verify are the supported
consistent copy/backup method for both roots, SQLite-related-state handling,
source/destination guards, exact pinned image/helper commands and the available
Windows validation environment. Gather official evidence and validate the
procedure when authorised; report a real scope blocker instead of inventing
commands, performance results or runtime acceptance.
