# ADR 0005: Separate, shared product database

Date: 2026-09-02
Status: Accepted
Session: 1

## Context

Manufacturer data dominates a project by size: about 22 MB unpacked for the 12
distinct application programs in the reference project, with one application
program alone carrying 5919 translations (RESEARCH §4.1).

Redistributing manufacturer product data carries a licensing risk that is not
ours to take on the user's behalf (RESEARCH §10), and `CLAUDE.md` forbids
hard-coding manufacturer products into the application.

## Decision

Manufacturer, hardware, application program and version data live in their own
SQLite database, outside the project file and shared across projects. Entries
are keyed by manufacturer, application program and version, with a content
hash.

A project references the product database and stays openable without it. In
that state the application shows the `Instance` layer only and marks the
project as incomplete, rather than refusing to open it or inventing defaults.

## Alternatives considered

**Embed manufacturer data in each project file.** This is what `.knxproj` does.
Rejected because it duplicates tens of megabytes across every project that uses
the same devices, and because it welds a licensing question onto every project
file the user creates.

## Consequences

The licensing separation is structural: product data can be distributed,
imported or withheld independently of the application and of project files.

A missing product database degrades gracefully, which means the incomplete
state is a first-class state in the model and the UI, not an error path.

Ingest, versioning and cache invalidation for the product database need
designing in Session 4.
