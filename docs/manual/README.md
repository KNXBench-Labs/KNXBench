← Previous: [Repository overview](../../README.md)

# KNXBench documentation

Welcome to the KNXBench manual. KNXBench is a Linux-first KNX engineering application: you
can import an ETS project, look at the topology, the buildings, the devices and the group
addresses, edit them, export the group addresses as CSV or the whole project as a readable
document, and watch the bus while it happens. Import is one-way: KNXBench reads a
`.knxproj` and never writes one.

This manual is written as a small book. If you are new here, start at chapter 1 and follow
the **Next** links at the bottom of each page. If you already know what you are looking for,
jump straight in — every chapter also links back here.

## Choose your starting point

| You want to… | Start here | What you will leave with |
| --- | --- | --- |
| Try the app without hardware | [Installation](getting-started/04-installation.md), then [your first project](getting-started/06-first-start.md#tutorial-create-save-and-reopen-your-first-project) | A saved `.knxdb` you can reopen |
| Understand the KNX vocabulary | [KNX in a few minutes](knx-basics/01-knx-in-a-few-minutes.md) | Devices, addresses and DPTs explained without a sales seminar |
| Bring an ETS project | [Projects](user-guide/02-projects.md) | An imported project and a report you can check |
| Configure a small job | [Complete configuration workflow](user-guide/06-configuration-workflow.md) | A device, group link and saved working file; no hardware writes |
| Diagnose bus traffic | [Bus and interfaces](user-guide/07-bus-and-interfaces.md) | A separate, safety-aware guide to actual KNXnet/IP operations |
| Run a server | [Docker Hub image](user-guide/11-web-and-docker.md#ready-made-image-from-docker-hub), then [Web and Docker](user-guide/11-web-and-docker.md) | A ready-made download, storage, authentication, HTTPS and networking configuration |
| Fix a problem | [Troubleshooting](reference/03-troubleshooting.md) or [FAQ](reference/04-faq.md) | Checks to run before opening an issue |
| Contribute code or docs | [Contributing](development/01-contributing.md) | Build instructions, quality gates and project conventions |

**No hardware? No problem.** Creating, importing, editing, saving and reporting
are file workflows. A bus connection is needed only for bus operations.

**Which build does this describe?** The manual follows repository source. The
public AppImage and Docker Hub images are release snapshots and may not contain
later source changes.
[Project status](getting-started/03-project-status.md) explains how to compare them.

## A visual map of the workbench

![The real KNXBench workbench in the porcelain theme, showing the fictional Sample house, project explorer and group-address table](../assets/screenshots/porcelain-group-addresses.png)

The left explorer answers **where**, the centre workspace answers **what**, and
the inspector answers **which properties**. Screenshots use fictional data, not
somebody's actual building. The [media inventory](../assets/README.md) records
capture provenance and how to refresh them.

> **Note**
>
> KNXBench is in **alpha**. It is useful, it is tested, and it is still moving. Keep backups
> of the projects you care about, and read [Known issues](known-issues.md) before you trust
> it with anything expensive.

## Getting started

1. [What is KNXBench?](getting-started/01-what-is-knxbench.md)
2. [Why KNXBench exists](getting-started/02-why-knxbench-exists.md)
3. [Project status](getting-started/03-project-status.md)
4. [Installation](getting-started/04-installation.md)
5. [Linux setup](getting-started/05-linux-setup.md)
6. [First start](getting-started/06-first-start.md)

## KNX basics

Enough KNX to use KNXBench. Not a KNX textbook.

7. [KNX in a few minutes](knx-basics/01-knx-in-a-few-minutes.md)
8. [Topology and individual addresses](knx-basics/02-topology-and-individual-addresses.md)
9. [Group addresses](knx-basics/03-group-addresses.md)
10. [Datapoint types](knx-basics/04-datapoint-types.md)
11. [Products and product databases](knx-basics/05-products-and-product-databases.md)

## User guide

12. [The user interface](user-guide/01-user-interface.md)
13. [Projects: create, open, import, save, export](user-guide/02-projects.md)
14. [Buildings and topology](user-guide/03-buildings-and-topology.md)
15. [Working with group addresses](user-guide/04-group-addresses.md)
16. [Devices and products](user-guide/05-devices-and-products.md)
17. [A complete configuration workflow](user-guide/06-configuration-workflow.md)
18. [Bus monitor and KNXnet/IP](user-guide/07-bus-and-interfaces.md)
19. [Documentation export and project comparison](user-guide/08-reports-and-diff.md)
20. [Settings, themes and languages](user-guide/09-settings-and-appearance.md)
21. [The command line](user-guide/10-command-line.md)
22. [Web and Docker deployment](user-guide/11-web-and-docker.md)
23. [AI agents over MCP](user-guide/12-ai-agents.md)

For support gaps: [step-by-step contribution guide (English)](../contribution-intake/README.md)
/ [Deutsch](../contribution-intake/DEUTSCH.md). No Git/XML/schema expertise is required;
a description without a report ZIP is welcome.

## Reference

24. [Keyboard shortcuts](reference/01-keyboard-shortcuts.md)
25. [Supported and unsupported KNX/ETS functionality](reference/02-supported-and-unsupported.md)
26. [Troubleshooting](reference/03-troubleshooting.md)
27. [FAQ](reference/04-faq.md)

## Status and plans

28. [Known issues](known-issues.md)
29. [Implementation status](implementation-status.md)
30. [Ideas and roadmap](ideas-and-roadmap.md)

## For developers

31. [Contributing](development/01-contributing.md)
32. [Building from source](development/02-building-from-source.md)
33. [Architecture tour](development/03-architecture-tour.md)

For a categorized map of existing suites, source links and privacy safeguards,
see the [test catalogue](../TEST_CATALOGUE.md). It describes test coverage, not
fresh passing results — the distinction is rather important.

## The engineering documents

The chapters above are written for people using KNXBench. The documents below are the
project's own working record: longer, denser, and updated as the code changes. The manual
links into them wherever the detail matters.

- [Architecture](../ARCHITECTURE.md) and the [architecture decision records](../adr/README.md)
- [Data model](../DATA_MODEL.md)
- [Import and export](../IMPORT_EXPORT.md)
- [Compatibility](../COMPATIBILITY.md)
- [Known limitations](../KNOWN_LIMITATIONS.md) and [open work](../OPEN_WORK.md)
- [Implementation status](../IMPLEMENTATION_STATUS.md)
- [Roadmap](../ROADMAP.md)
- [Language packs](../LANGUAGE_PACKS.md)

## Where to ask

KNXBench lives on GitHub: [KNXBench-Labs/KNXBench](https://github.com/KNXBench-Labs/KNXBench).
Bug reports and questions belong in its
[issue tracker](https://github.com/KNXBench-Labs/KNXBench/issues).

Next: [What is KNXBench?](getting-started/01-what-is-knxbench.md) →
