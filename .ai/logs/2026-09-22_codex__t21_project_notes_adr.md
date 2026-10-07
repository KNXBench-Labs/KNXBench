# T21 — project notes ADR

## Outcome

ADR-0031 decides the shape of future in-application project notes without
implementing them:

- one ordered `ProjectNote` collection owned by the `Project` aggregate;
- stable note IDs and typed targets for the project or supported user-facing
  entities;
- a separate native-store table and a future ordered schema migration;
- plain UTF-8 title/body, with no markup, HTML, attachments, or author model;
- explicit target-disposition when deleting an entity, never silent cascade;
- explicit opt-in to `knx-report`, defaulting to private scratch content;
- explicit compatibility loss reporting for any future external format that
  cannot carry notes.

No product code, schema, migration, endpoint, or UI changed.

## Documentation

- `docs/adr/0031-project-notes-are-a-project-owned-collection.md`
- `docs/DATA_MODEL.md` marks the concept decided but not implemented.
- `docs/adr/README.md` indexes the ADR.
- `docs/manual/ideas-and-roadmap.md` links the decision and retains the
  not-implemented status.
- The ignored local `ideas.md` note was updated too; it remains intentionally
  outside Git.

## Verification

- `git diff --check`
- `cargo run -p xtask -- check-anchors` — 386 links across 182 Markdown files,
  none dead.
- Independent review: 0 Critical, 0 Important.
