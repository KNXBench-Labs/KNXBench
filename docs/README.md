← Previous: [Repository overview](../README.md)

# Documentation hub

**Using KNXBench? Start with the [manual](manual/README.md).** It is a guided book,
with installation, practical workflows, screenshots and a KNX primer. The pages
below are the engineering record behind it, not extra reading before your first click.

## Pick the right shelf

| I need… | Read |
| --- | --- |
| Installation and a first saved project | [Installation](manual/getting-started/04-installation.md) → [First start](manual/getting-started/06-first-start.md) |
| Everyday editing workflows | [User guide](manual/README.md#user-guide), especially the [complete example](manual/user-guide/06-configuration-workflow.md) |
| English offline projects to explore | [Three community demos](../demos/README.md) — local review candidate; [scope and verification](COMMUNITY_DEMO_PROJECTS.md) |
| Configuration or a broken setup | [Web and Docker](manual/user-guide/11-web-and-docker.md), [Settings](manual/user-guide/09-settings-and-appearance.md), [Troubleshooting](manual/reference/03-troubleshooting.md) |
| Supported features and honest boundaries | [User-facing status](manual/implementation-status.md), [Known issues](manual/known-issues.md), [Compatibility](COMPATIBILITY.md) |
| Were the status and issue claims checked? | [Known-issues/status source and evidence audit](status/2026-10-08-known-issues-status-audit.md) |
| Future work, not promises | [Ideas and roadmap](manual/ideas-and-roadmap.md), [Open work](OPEN_WORK.md), [Roadmap](ROADMAP.md) |
| What happened to the original wish list? | [Source/test audit of all twelve ideas](status/2026-10-08-ideas-roadmap-audit.md) |
| A contributor's starting point | [Contributing](manual/development/01-contributing.md), [Build instructions](manual/development/02-building-from-source.md) |
| Which tests exist, and how private inputs are protected | [Test catalogue](TEST_CATALOGUE.md) — categories, source modules and explicit prerequisites |

## Understand the implementation

- [Architecture](ARCHITECTURE.md): domain, application and infrastructure boundaries.
- [Data model](DATA_MODEL.md): projects, topology, addresses and versioned storage.
- [Import/export](IMPORT_EXPORT.md): parsing, validation, preservation and supported outputs.
- [Research index](RESEARCH.md): evidence and explicitly unresolved questions.
- [Architecture decisions](adr/README.md): why the code is shaped this way.
- [Status ledger](status/LEDGER.md): the status of record for tracked work.
- [Implementation log](IMPLEMENTATION_STATUS.md): dated changes and actual gate results.
- [Known limitations](KNOWN_LIMITATIONS.md): detailed behaviour boundaries, including history.

## Help keep the documentation honest

The manual follows **repository source**, not an imaginary all-features-in-one
release. A published AppImage can predate the newest documented feature. Check
[Project status](manual/getting-started/03-project-status.md) before comparing builds.

Screenshots must come from the real application with fictional data. See the
[media inventory and capture guide](assets/README.md). The
[documentation maintenance guide](DOCUMENTATION.md) explains links, navigation
and validation. A green link checker does not make an unsupported feature work.

Next: [Open the KNXBench manual](manual/README.md) →
