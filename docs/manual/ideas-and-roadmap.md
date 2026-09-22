← Previous: [Implementation status](implementation-status.md) · [Manual index](README.md)

# Ideas and roadmap

This chapter is in three parts, and the boundaries between them matter more
than anything inside them:

1. **What is already built** — a short backward look.
2. **Planned** — work the project's roadmap commits to, none of it finished,
   none of it dated.
3. **Ideas and experiments** — things someone wrote down. Not commitments.

There are no dates anywhere in this chapter, and no release schedule exists.
KNXBench is at `0.1.0-alpha.1` with nothing published. An item's position in
part 2 says it is intended; it does not say when, or that it will happen at
all.

> The distinction is boring until the day somebody buys hardware because a
> documentation page sounded confident. So: part 3 is daydreaming, part 2 is
> intent, and only [Implementation status](implementation-status.md) describes
> software that exists.

---

## What is already built

The full inventory is [Implementation status](implementation-status.md), with
finer-grained format detail in
[Supported and unsupported](reference/02-supported-and-unsupported.md). In
one paragraph: KNXBench imports ETS project files at the two schemas it has
real samples for (and, since 2026-09-20, writes none —
[ADR-0028](../adr/0028-no-knxproj-export.md)), keeps everything it does not understand
rather than dropping it, stores projects in its own SQLite format with full
undo, ingests product databases, edits topology, buildings, group addresses,
communication objects, links and parameters, exports HTML documentation,
compares two projects, and monitors a KNX bus over KNXnet/IP.

Several things on the original wish list have already been built and are
therefore not ideas any more: the command palette, search, light and dark
themes, small interface animations, a project status dashboard, gateway
discovery, the in-application help panel, and the lightly humorous tone in
messages and toasts.

## Planned

Everything below is on the roadmap, unfinished, and waiting on something
specific. The "waiting on" column is the honest part — most of these are not
waiting on somebody finding time.

| Item | Where it stands | Waiting on |
| --- | --- | --- |
| Commissioning a real device | All six procedures are implemented in the core library and verified against a simulator this project wrote. Read-only runs against a real installation have happened twice. | Dedicated test hardware. Writing to a live installation was ruled out of scope by the maintainer until a test bench exists — the only bus available is a house people live in. The first real write will need its own explicit go-ahead. |
| Import support for ETS schemas 12, 13, 14, 20 and 22 | The parser handles the schema family; these versions have never been tested against a real file. | One real project file per schema. No sample-hunting is scheduled. |
| Opening AES-protected ETS6 projects | The key derivation already exists in the `knx-secure` crate. | A genuine AES-protected sample. Verifying against a self-made one would only prove the implementation agrees with itself. |
| The `Functions` element in the KNX project model | Absent from every reference sample, so there is nothing to model against. | New KNX specification documentation. Deferred by decision, explicitly not rejected. |
| Whether Data Secure runtime keys can be read from a project file | An open question with a named home in the `knx-secure` crate; nothing built. | Its own scheduling. See also *KNX Secure*, below. |
| End-user documentation | The in-application help panel shipped. The written manual half stayed open — this document is that half being written. | Nothing. It is in progress. |

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
- **The 200-series LTE and system datapoint types: out of scope for 1.0.0.**
  The application cannot speak LTE addressing at all, so codecs for those
  types would improve a coverage table and nothing else.
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
[RESEARCH.md §13](../RESEARCH.md#13-natural-language-interaction-and-mcp-prerequisite-audit-2026-09-22-t19),
not scheduled. The current command, authorization, revision and audit
boundaries are not sufficient for mutation. Any later start is bounded reads
and typed proposals with exact human approval, never raw commands or bus work.

**Automating repetitive tasks.** Researched in
[RESEARCH.md §14](../RESEARCH.md#14-repetitive-task-automation-and-macro-layer-decision-2026-09-22-t20),
not scheduled. The supported future direction is a parameterised template over
an explicit selection, expanded into a previewed, revision-bound and atomic
`Command::Batch` with one-step undo. Raw command recording, partial mutation,
a script engine and bus-facing macros are not the plan.

**A live "who talks to whom" view.** Research now narrows this to an
evidence-labelled flow for one selected bus-monitor telegram, not a topology
canvas that pretends configured recipients were observed. See
[RESEARCH §16](../RESEARCH.md#16-who-talks-to-whom-flow-view-decision-2026-09-22).
It is designed neither implemented nor scheduled.

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
expanded considerably — the note in the idea list asks for at least thirty
different sentences per case, in the spirit of Dungeon Keeper II or Marvin
from *The Hitchhiker's Guide to the Galaxy*. Purely cosmetic, entirely
optional, and the only idea on this page that can never corrupt a project
file.

**A project logo.** In progress, in the sense that somebody is thinking about
it.

### Suggestions from this documentation pass

Marked separately because they are exactly that — suggestions written down
while this manual was being checked against the code, not decisions anybody
has taken.

- A correction to the New project dialog, whose hint promises that the
  group-address style can be changed later. It cannot.
- A way to download the project file from the browser, since the server
  already has the route and nothing calls it.
- Removing `.vd2` from the file pickers and command-line help that still
  offer it, given that it is always refused.

None of these are scheduled. They are listed here so they are not lost, and
because a manual that spots a defect and says nothing is not much of a
manual.

---

The project's own roadmap, with the full reasoning and the dependency chains
behind every item above, is [`docs/ROADMAP.md`](../ROADMAP.md).

[Manual index](README.md) · Next: [Contributing](development/01-contributing.md) →
