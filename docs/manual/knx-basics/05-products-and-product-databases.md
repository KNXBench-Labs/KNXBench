← Previous: [Datapoint types](04-datapoint-types.md) · [Manual index](../README.md)

# Products and product databases

A device on the bus is a piece of hardware running a program. KNXBench keeps what it
knows about manufacturers, that hardware, and those programs in a separate database from
your project — this chapter is about that database: what it contains, where it comes
from, and what KNXBench can and can't read into it.

## Manufacturer, hardware, product, application program

These four terms describe increasingly specific things, and KNX product data keeps them
genuinely separate:

- A **manufacturer** is who made the thing.
- A piece of **hardware** is a physical device model — an actuator, a sensor, a coupler.
  Whether it's rail-mounted, whether it needs an individual address, whether it's a
  coupler or a power supply are all facts recorded at this level.
- A **product** is a sellable variant of that hardware — its own order number, its own
  catalog text — because one physical design is often sold under more than one order
  number.
- An **application program** is the software that gives a piece of hardware its actual
  behavior: the set of communication objects it exposes and the parameters that
  configure them. The same hardware can potentially run different application programs
  (or versions of one), and that's what actually determines what the device does on the
  bus — the hardware alone tells you comparatively little.

A device in your project is meaningful largely because of the application program behind
it: that's where its list of communication objects comes from, each with its own
datapoint type expectations (previous chapter), and where its configurable parameters
come from.

## Why the catalog matters when you build a project

Without product data, a device you add to a project is a blank slate — you'd have to
invent its communication objects and parameters by hand, which is both tedious and error
prone. With the manufacturer's data installed, adding one of its products gives the
device its real communication objects and parameters straight away. This is also how
importing an existing `.knxproj` benefits from having product data available: it can
enrich the imported devices' communication objects with descriptions, flags and
datapoint types the project data alone didn't state — filling in gaps, never overwriting
a value the project itself already recorded.

## What a `.knxprod` file is

A `.knxprod` file is a manufacturer's product package: a ZIP archive built around a
`knx_master.xml` file (the manufacturer, hardware, product and application program
records) plus the associated application-program XML and any translations. It's the same
XML family, and the same schema numbering, that a `.knxproj` project file uses — which is
why manufacturer data can also travel bundled inside a project archive instead of as a
standalone package, and both routes end up in the same product database.

## What KNXBench can read today

- **Standalone `.knxprod` packages**, installed directly into the product database, at
  master data schemes **11, 12, 13, 14, 20** and exact-namespace **21** — checked
  atomically and safe to re-install (an already-known file is skipped, not duplicated).
  A passing read-only corpus matrix and synthetic tests verify parser/persistence
  behavior for these schemes, not complete manufacturer semantics or ETS parity.
  Schemes 15–19 and 22 are not verified for standalone installation.
- **Manufacturer data bundled inside an imported `.knxproj`**, ingested wherever the
  project container itself can be read — which today means schema 11 fully, schema 21
  fully, and schema 23 for reading (no round-trip claim on that last one; see
  [Projects: create, open, import, save, export](../user-guide/02-projects.md)).
- **An ambiguous datapoint type list** on a communication object reference (a product
  occasionally states more than one acceptable type for one object) is reported, not
  guessed — the same "report, don't guess" rule the previous chapter's conflict handling
  follows.

## What's out of scope, on purpose

Two things are explicitly not supported, and this is a scope decision, not a bug KNXBench
happens to have:

- **Encrypted `.knxprod` packages.** Any encrypted member inside a product ZIP is
  refused outright.
- **Legacy `.vd2` files.** These are a pre-2013, ETS2-era container format, a genuinely
  different file family from the ZIP/XML `.knxprod` packages KNXBench reads — not an
  encryption question, and not something KNXBench will start reading.

Neither of these is "not implemented yet." Both are recognized by name and rejected with
a clear error, rather than failing confusingly partway through.

See [Devices and products](../user-guide/05-devices-and-products.md) for installing a
product database and browsing its catalog inside KNXBench, and
[Supported and unsupported KNX/ETS functionality](../reference/02-supported-and-unsupported.md)
for the full, current scheme-by-scheme picture.

[Manual index](../README.md) · Next: [The user interface](../user-guide/01-user-interface.md) →
