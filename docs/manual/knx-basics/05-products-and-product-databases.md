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

## Systems and mask versions, in plain English

A **system** is the device's KNX system-software family: the basic rules for
communicating with and configuring it. Think of it as the device's foundation,
not its lighting or heating application. System numbers are names, not ETS
versions or a quality ranking.

| System | What it means | Example mask versions (TP) | When it appeared |
| --- | --- | --- | --- |
| **System 1 / BCU1** | The original, simpler EIB device family, with relatively few communication objects. | `0010h`, `0012h`, `0013h` | **1991**, according to KNX training material. |
| **System 2 / BCU2** | An expanded family with more communication objects and device-management features than BCU1. | `0020h`, `0021h`, `0025h` | A few years after 1991; exact year not verified. |
| **System 7 / BIM M112** | Another expanded family for devices with more application resources than System 1. | `0700h`, `0701h`, `0705h` | A few years after 1991; exact year not verified. |
| **System B** | Builds on System 7, allowing much larger sets of communication objects and group-address links. | `07B0h` | Documented by **2007**; this is not a verified launch year. |

These families coexist; a newer family does not make an older device useless.
**System 300** is a separate profile (for example `0300h`), while *Coupler*,
*KNXnet/IP router* and *USB interface* describe specialised profiles.

### What does a mask version tell me?

A **mask version** identifies the device's standardised KNX system-software
profile. It is not its individual address, product number or application version.
Its value uses four hexadecimal digits: `0701h`, `0x0701` and the product-data
reference `MV-0701` denote the same value. The `h` means hexadecimal.

For the example **`0701h`**:

- **`0`** at the front means twisted-pair KNX (TP).
- **`701`** identifies the particular profile and revision; the final **`1`**
  is the profile revision. The complete value `0701h` is a recognised System 7
  mask, not “KNX version 7.01”.
- Other first digits include **`1`** for powerline PL110, **`2`** for radio RF
  and **`5`** for KNX IP; not every profile exists on every medium.

Use the full, known mask, not a guessed number pattern. Two devices with the
same mask need not run the same application, and a family label does **not**
guarantee that KNXBench can program them. In the
[Devices view](../user-guide/05-devices-and-products.md#system-column-since-alpha-15),
KNXBench shows the mask declared by the product data, not a live device reading.

[Sources and dating caveats](../../research/2026-10-19-mask-version-system-family.md#plain-language-manual-overview-20-october-2026).

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
  master data schemes **10** (ETS4 era), **11, 12, 13, 14, 20** and exact-namespace
  **21** and **23** — checked atomically and safe to re-install (an already-known file
  is skipped, not duplicated). Corpus measurements and synthetic tests verify
  parser/persistence behavior for these schemes, not complete manufacturer semantics
  or ETS parity. Schemes 15–19, 22 and 24 are refused for standalone installation.
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
