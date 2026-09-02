# ADR 0003: SQLite as the native project format

Date: 2026-09-02
Status: Accepted
Session: 1

## Context

`CLAUDE.md` requires a versioned and migratable model, and requires that import
formats do not dictate the internal model.

The reference project holds 514 group addresses, 907 communication object
instances and 1390 parameter instance values (RESEARCH §4.1). Access patterns
are lookups and filtered queries, not whole-document traversal.

Opaque passthrough data (ADR-0006) is binary and can be large.

## Decision

A project is one SQLite file, and that file is the working format. The schema
version lives in SQLite's `user_version` pragma, and upgrades run as an ordered
chain of migrations.

`.knxproj` is import and export only. It never becomes the thing we edit.

A textual export and import format may be added later, for diagnostics and for
version control of the parts that are usefully diffable. It is not built now,
because no need for it has been demonstrated.

## Alternatives considered

**Use `.knxproj` as the working format.** Rejected because the ETS schema would
then be the model — every internal improvement would have to be expressible in
someone else's format — and because there would be nothing to version and
migrate.

**A directory of JSON or TOML files.** Rejected because there is no
transactional save: a crash halfway through writing leaves a project in an
undefined state. The diffability argument in its favour is weaker than it
looks, since the bulk of the data is 22 MB of product references and binary
blobs that no one reads as a diff.

## Consequences

Saves are atomic and incremental rather than full rewrites.

Project files are opaque to version control tools. This is the cost, and the
optional text export is the eventual answer to it.

Each schema version needs a frozen fixture and a migration test, so that a file
written by an older version stays openable.
