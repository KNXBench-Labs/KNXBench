← Previous: [Implementation status](implementation-status.md) · [Manual index](README.md)

# Ideas and roadmap

This chapter is in three parts, and the boundaries between them matter more
than anything inside them:

1. **What is already built** — a short backward look.
2. **Planned** — unfinished or deferred scope, with prerequisites and no dates.
3. **Ideas and experiments** — things someone wrote down. Not commitments.

No future release schedule is promised here. KNXBench is in its `0.1.0-alpha`
series with a public alpha.5 pre-release. This source-level overview was checked
on **8 October 2026**. An item's position in
part 2 does not schedule it or reopen a recorded alpha boundary. This audit
covers source `608a204b`; the released AppImage is a separate snapshot.

> The distinction is boring until the day somebody buys hardware because a
> documentation page sounded confident. So: part 3 is daydreaming, part 2 is
> intent, and only [Implementation status](implementation-status.md) describes
> software that exists.

---

## What is already built

The full inventory is [Implementation status](implementation-status.md), with
finer-grained format detail in
[Supported and unsupported](reference/02-supported-and-unsupported.md). In
one paragraph: KNXBench imports real ETS project samples at schemas 11, 21
and 23, with schema-23 module evidence still bounded (and, since 2026-09-20,
writes none — [ADR-0028](../adr/0028-no-knxproj-export.md)), preserves opaque data
where technically possible and reports unsupported data, stores projects in
its own SQLite format with undo during the open session, ingests product
databases, edits topology, buildings, group addresses,
communication objects, links and parameters, exports HTML documentation,
compares two projects, and monitors a KNX bus over KNXnet/IP.

Several things on the original wish list have already been built and are
therefore not ideas any more: the command palette, search, light and dark
themes, small interface animations, a project status dashboard, gateway
discovery, the in-application help panel, and the lightly humorous tone in
messages and toasts.

### Original wish list: the useful part and the remainder

| Original idea | Available now | Real remainder |
| --- | --- | --- |
| Small animations | Motion level/style settings and reduced-motion handling | Per-category controls; broader native/accessibility evidence |
| MCP | Experimental read-only saved-project tools and agent skill | No agent edits, chatbox or bus control |
| Device discovery | Gateway search, one-line occupancy scan, explicit reconciliation | No product identity from a scan or coupler traversal |
| Repetitive-task automation | Undoable batch primitive, not a macro feature | No templates/recorder/scheduler UI |
| Humour templates | Thirty error wrappers, thirty late-night entries and holiday pairs | Optional extra copy, not an absent mechanism |
| Project notes | Domain shape decided in ADR-0031 | No collection, persistence or note editor |
| Project status | Dashboard counts and diagnostics | No clickable count-to-detail drill-down |
| Animated GA/device connections | Flow view, values, readable layout and a separate window | Session-local only; inferred recipients are not receipt/effect proof |
| Mobile app | Browser workbench is a separate existing option | No dedicated mobile application |
| Other operating systems | Other OSes can reach a running server through a browser | No native Windows/macOS build |
| Themes | Built-ins, System, bundled/importable packs and LCARS | No arbitrary CSS/code plug-ins; native accessibility stays bounded |
| Schema 21/23 import | Project import, exact product/master admission and native save/reopen | Missing independent module-using schema-23 evidence; no ETS project export |

For maintainers, the [source/test audit](../status/2026-10-08-ideas-roadmap-audit.md)
preserves all twelve original entries. A wish list is not a completion
percentage. Some wishes are considerably larger than others.

## Planned

These are real remainders, **not a promised implementation queue**. Some were
explicitly deferred by the user and need a new decision before work resumes.
The maintained list is [Open work](../OPEN_WORK.md); historical alpha goals
are finished at their recorded scope, not proof that every feature exists.

| Item | Where it stands | Waiting on |
| --- | --- | --- |
| Persistent project undo and native versions | Implemented in the local 2026-10-09 source package: native-backed undo/redo, named/save/pre-restore versions and guarded restore. Not in alpha.6; commissioning metadata remains a different history. | [Project history](../PROJECT_HISTORY.md). Independent disaster-backup copies remain necessary. |
| Legacy device download and oversized `.vd5` coverage | Web/CLI offline import, password dialog and one remembered password already exist for the evidenced legacy scope. | Resource measurement before raising legacy limits; separate L4 mapping and recovery evidence. |
| Selective import, diff apply/merge and three-way comparison | Source selected-device/line merge has a shared application planner, File-menu preview/consent and CLI; diff apply/merge and three-way comparison remain separate future work. | [Selective import](../SELECTIVE_IMPORT.md); [source integration acceptance](../status/2026-10-10-import-expansion-integration.md); authorized historical ETS exports and released-artifact acceptance remain separate. |
| Specialized parameter widgets and online manufacturer updates | Generic supported parameter editing and manual product-file installation exist. | Supported type/UIHint evidence and separate catalog/update design. |
| Commissioning more real devices | A device download (application tables and parameters) has been run and read back on one real device after an explicit go-ahead; the property-based procedures are verified against a simulator this project wrote. Address programming is refused until durable recovery exists. | Test hardware and a per-target go-ahead. The only bus available is a house people live in, so every new device, write scope or experiment needs its own explicit approval. |
| Import support for ETS schemas 12, 13, 14, 20 and 22 | The parser handles the schema family; these versions have never been tested against a real file. | One real project file per schema. No sample-hunting is scheduled. |
| Opening AES-protected ETS6 projects | The key derivation already exists in the `knx-secure` crate. | A genuine AES-protected sample. Verifying against a self-made one would only prove the implementation agrees with itself. |
| The `Functions` element in the KNX project model | No project entity/import/storage/UI implementation; master `FunctionType` is not the same thing. | Dedicated ADR and representative project evidence. Deferred, not rejected. |
| Whether Data Secure runtime keys can be read from a project file | An open question with a named home in the `knx-secure` crate; nothing built. | Its own scheduling. See also *KNX Secure*, below. |
| Broader platform and accessibility validation | Chromium/offline UI evidence exists; native WebKitGTK, Orca and wider host coverage remain separate. | Real platform runs, not screenshots interpreted as accessibility proof. |

### Decided against, or parked on purpose

These are not slow-moving plans. They are answers, and they are here so that
nobody waits for them.

- **A plug-in API: no.** Extension stays data-shaped — language packs,
  product databases, group-address CSV and the command-line tool. The
  reasoning is that there is nothing stable to expose yet: every candidate
  seam in the codebase has exactly one implementation, so an interface would
  be generalised from a sample of one. Recorded in
  [ADR-0025](../adr/0025-extension-is-data-not-code.md), with the survey in
  [`docs/PLUGIN_FEASIBILITY.md`](../PLUGIN_FEASIBILITY.md).
- **Spatial floor plans: not in version 1.0.0.** The building model stays
  topological. A later placement layer has had its shape pre-committed so it
  cannot be improvised, but building it needs its own architecture decision
  — [ADR-0019](../adr/0019-building-model-stays-topological.md).
- **DPTs outside implemented main families 1–30:** unsupported at the current
  codec boundary, not a blanket claim that every higher-numbered type is LTE.
  Expansion needs measured device relevance and authoritative subtype evidence
  — [DPT audit](../spec-audits/2026-10-07-dpt-document-audit.md).
- **Legacy `.vd2` files and encrypted `.knxprod` packages: permanently out of
  scope.** Both are refused by name rather than half-attempted.
- **Multi-user editing: parked.** It is not a version 1.0.0 requirement.
  Today one server holds one project with one undo stack, and that is what
  the documentation says everywhere.
- **KNX Secure: deferred.** Not implemented, with no sample key material
  available to verify an implementation against. IP Secure was scoped and
  then shelved indefinitely.

## Ideas and experiments

Nothing in this section is planned. These are entries in the project's own
idea list, and where an entry has a condition attached, it is a technical
dependency somebody noticed — not a queue position.

**Natural-language and MCP interaction.** Researched in
[RESEARCH.md §13](../research/features-and-ui.md#13-natural-language-interaction-and-mcp-prerequisite-audit-2026-09-22-t19).
The read-only part now exists: [AI agents over MCP](user-guide/12-ai-agents.md)
lets an agent query saved projects and check proposed group-address CSVs.
Changing a project through an agent is still not planned. The current
command, authorization, revision and audit boundaries are not sufficient
for mutation, and bus work stays excluded.

**Automating repetitive tasks.** Researched in
[RESEARCH.md §14](../research/features-and-ui.md#14-repetitive-task-automation-and-macro-layer-decision-2026-09-22-t20),
not scheduled. The supported future direction is a parameterised template over
an explicit selection, expanded into a previewed, revision-bound and atomic
`Command::Batch` with one-step undo. Raw command recording, partial mutation,
a script engine and bus-facing macros are not the plan.

**A live "who talks to whom" view is already built.** The session-local,
read-only [flow view](user-guide/07-bus-and-interfaces.md#the-flow-view) shows observed
traffic and labels configured/inferred recipients separately. It is not proof that
each recipient acted on a telegram. A separate window and readability controls
also exist; native accessibility and wider platform evidence remain open.

**A mobile application.** Possible in principle over a KNX IP interface. It
is an entirely new platform, which makes it a much larger project than it
sounds. An idea.

**Support for operating systems other than Linux.** The project is Linux-first
by design, and porting is only sensible once the Linux version is stable. An
idea, with no work done.

**Project notes inside the application.** Somewhere to write down why a
decision was made, attached to the project or a user-facing entity. The domain
shape is now decided in
[ADR-0031](../adr/0031-project-notes-are-a-project-owned-collection.md): a
project-owned collection with typed targets and explicit report opt-in. It is
not implemented.

**More humour in the messages.** The existing toast and error copy should be
expanded only if more variety is wanted. The original thirty-entry error and
late-night lists already exist, along with holiday pairs. This is optional
copy expansion, not a missing feature. A joke must never hide the real error
or weaken a safety warning; the bus is quite capable of comedy without help.

**The project logo already exists.** It is used on the repository front page;
it is no longer a roadmap item.

### Suggestions from this documentation pass

Marked separately because they are exactly that — suggestions written down
while this manual was being checked against the code, not decisions anybody
has taken.

- Removing `.vd2` from the command-line help remains an unscheduled copy fix;
  that format is always refused. Legacy `.vd3`–`.vd5` uses its own supported
  inspection/import path, not `products ingest`.

Already delivered: project and add-device wizards, autosave, parameter-workspace
separation, the web **Export project…** action, LCARS, achievements and playful
language packs. The group-address style **can** be changed on the Project node.
See [Implementation status](implementation-status.md) and
[open work](../OPEN_WORK.md), rather than treating old wish-list entries as promises.

Only the remaining suggestions are unscheduled; the paragraph above lists
delivered features, not a second backlog.

### Delivered outside the original wish list

The repository and alpha.5 download are public. The marketing website and
approved Evolution Story are deployed. Community support-gap analysis and
manual evidence export also exist; the temporary intake repository was
retired. Website maintenance, privacy-text review and unverified mailbox
delivery remain separate from implementation. See [Open work](../OPEN_WORK.md).

---

The project's own roadmap, with the full reasoning and the dependency chains
behind every item above, is [`docs/ROADMAP.md`](../ROADMAP.md).

[Manual index](README.md) · Next: [Contributing](development/01-contributing.md) →
