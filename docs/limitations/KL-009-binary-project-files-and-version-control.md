# Binary project files and version control

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-009)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

**Update 2026-10-13 (Goal A1, KL-157).** `knx diff` now opens a native
store read-only and upgrades an older one only in memory, so the Git
textconv/diff-driver recipe no longer changes an older working file. The
retained sentence below about an in-place upgrade describes the earlier state.

<!-- BEGIN retained catalogue record -->

## 9. Project files are not diffable

**Limitation.** A project is a SQLite file, so version control tools cannot
show a meaningful diff of it.

**Cause.** A deliberate trade for transactional, incremental saving and indexed
access ([ADR-0003](../adr/0003-sqlite-project-format.md)).

**Impact.** Projects can be versioned as binaries only. Reviewing what changed
between two versions requires the application.

**Lifted when.** A textual export and import format is added, if a demonstrated
need arises. It is deliberately not built speculatively.

**Update 2026-10-06 (AR15): accepted boundary, with a verified Git recipe.**
Still one SQLite file (ADR-0003); no textual project format exists, and none
is built speculatively. Two ways to review versions were run on a synthetic
repository (the `zipcrypto-minimal` fixture project, one address renamed and
one added through `knx ga-import`, two commits): `knx diff` registered as a
Git external diff driver prints `+ group address 0/0/2` and
`~ group address 0/0/1: name: GA -> Flur Licht`; `git show`/`git log -p` need
`--ext-diff`, and the driver must handle Git's `/dev/null` for added/removed
files, which `knx diff` refuses (Git then aborts the log). A `sqlite3 .dump`
textconv also works, but shows raw rows and counters rather than KNX
entities. `knx diff` reports what it compares, not every stored byte, and it
upgrades an older-schema working file in place (§157). Recipe in the manual:
[Reviewing project versions in Git](../manual/user-guide/08-reports-and-diff.md#reviewing-project-versions-in-git).
<!-- END retained catalogue record -->
