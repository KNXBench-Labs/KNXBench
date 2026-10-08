← Previous: [Manual index](../README.md) · [Repository overview](../../../README.md)

# What is KNXBench?

KNXBench is a Linux-first KNX engineering application. It is an independent,
open alternative to ETS for working with KNX projects — its own domain model,
its own `.knxproj` parser, its own SQLite-based project format, its own
product database, and its own KNXnet/IP implementation. It is
**KNX-compatible**, not KNX-certified, and it does not claim full ETS
compatibility.

If you have never heard of KNX, group addresses, or ETS, that is fine.
[KNX in a few minutes](../knx-basics/01-knx-in-a-few-minutes.md) explains the
basics before you touch the software. This chapter only tells you what
KNXBench is for.

## Who it is for

Anyone who works with a KNX installation and wants to do that work on Linux:
electricians, integrators, and hobbyists who plan, document, or review a
building's KNX configuration and who have grown tired of running Windows in
a virtual machine just for one program.

## What it does today

- **Imports ETS projects.** Open a `.knxproj` archive exported from ETS and
  KNXBench reads it, reports any warnings or errors it found on the way, and
  turns it into an editable project.
- **Edits topology, buildings, devices, and group addresses.** Areas, lines,
  devices, individual addresses, group addresses, and the links between a
  device's communication objects and those group addresses are all visible
  and editable.
- **Stores projects natively.** KNXBench saves and reopens projects in its
  own `.knxdb` format — a versioned SQLite file that is the supported,
  lossless working format, and the only format KNXBench writes a project to.
  Import is one-way: there is no `.knxproj` export
  ([ADR-0028](../../adr/0028-no-knxproj-export.md)).
- **Ships a product database.** Manufacturer, product, and application
  program data can be installed from `.knxprod` packages or ingested from an
  imported project, and used to enrich a device's communication objects. See
  [Products and product databases](../knx-basics/05-products-and-product-databases.md).
- **Exports documentation.** A project can be turned into a single
  self-contained HTML report.
- **Compares projects.** Two `.knxdb` files can be diffed to see what
  changed.
- **Exports and imports group addresses as CSV.** A simple, KNXBench-defined
  CSV format for bulk group-address work.
- **Watches the bus.** A bus monitor connects to a KNXnet/IP gateway over a
  tunnelling connection and shows telegrams as they pass, plus a form to send
  a single group value.
- **Comes with a command-line tool.** The `knx` binary covers import, export,
  product-database management, and KNXnet/IP operations — including
  operations, such as routing, that are not yet in the graphical
  interface.

## What it is not

Equally important is what KNXBench does not do, at least not yet:

- It is **not KNX-certified**, and running it does not require or imply
  certification.
- It does not claim **full ETS compatibility**. Some ETS project schemas and
  vendor-specific data are supported only where real test material exists to
  verify it against; see
  [Supported and unsupported KNX/ETS functionality](../reference/02-supported-and-unsupported.md).
- It is not yet a **commissioning** tool. A device download works within a
  very narrow verified scope (one device so far), individual addresses cannot
  be programmed at the moment, and most devices still need another tool; see
  [Downloading to a device](../user-guide/07-bus-and-interfaces.md#downloading-to-a-device).
- It does not implement **KNX IP Secure**. Only plain KNXnet/IP installations
  are supported.

> **Note**
>
> KNXBench is in **alpha**. Before installing, read
> [Project status](03-project-status.md), says exactly what that means before
> you install anything.

## Where to go next

If you want the short version of why this project exists at all, continue to
[Why KNXBench exists](02-why-knxbench-exists.md). If you just want to install
it, skip ahead to [Installation](04-installation.md).

[Manual index](../README.md) · Next: [Why KNXBench exists](02-why-knxbench-exists.md) →
