# ADR-0140: Vendor ZIPs of product packages install as a whole

Status: accepted.
Date: 2026-10-18.

## Context

The [product-database gap census](../research/2026-10-17-product-database-gap-census.md)
(G3 and its addendum) found that the usual manufacturer download is not a
`.knxprod` but a plain ZIP holding one or more `.knxprod` packages next to manuals
and other files. Such a ZIP has no `knx_master.xml` of its own, so the installer
refused it as "product ZIP is missing knx_master.xml": 22 of 24 wrappers in the
first collection and both in the addendum. 38 of the 39 inner packages installed
on their own. For one complete manufacturer database there was no install route
at all: under a `.knxprod` name the standard size bounds refuse it, and the CLI
refused `--allow-large-package` for `.zip` names.

Evidence:

- **[V]** A name census over the local collection (18 October 2026, counts only):
  48 top-level ZIPs without a master and without project folders, 33 of which hold
  1 to 10 `.knxprod` members (22 one, 8 two, 1 three, 1 nine, 1 ten). Other members:
  PDF manuals, `.vd5` legacy databases, plug-in DLLs, ETS app packages and four
  inner `.zip` files, none of which holds a product package.
- **[D]** Nothing in the KNX documents defines such a wrapper. It is a
  distribution convention, so KNXBench treats it as a container only and
  interprets nothing in it beyond what a standalone package already gets.

## Decision

1. **Recognition.** A ZIP that passes every package check (structure, identity,
   paths, sizes) but has no `knx_master.xml` and holds at least one file member
   whose name ends in `.knxprod` (ASCII case-insensitive) is a vendor wrapper.
   `install_package` refuses it with `PackageError::WrapperArchive`, which names
   the inner packages, instead of "missing knx_master.xml". A ZIP with neither a
   master nor an inner package is still `MissingMaster`.
2. **Install.** `knx_productdb::install_product_file` installs a package or a
   wrapper. For a wrapper, every inner package is installed byte-exact through the
   ordinary package path (same admission, same checks, same storage), in archive
   order and in **one transaction**. A refused inner package refuses the whole
   wrapper (`PackageError::WrapperMember` names it) and nothing is stored.
3. **Provenance.** Each inner package is recorded under the source name
   `<wrapper name>!/<member path>`. Identical inner bytes are stored once and keep
   every name they arrived under, as for repeated standalone installs.
4. **Other members.** Every other file member is reported with path, size and
   SHA-256 and is **not stored**. A member named like a `.knxprod` whose bytes
   start with the AppleDouble magic `00 05 16 07` (RFC 1740, the `__MACOSX/._name`
   sidecar macOS archivers add for every file) is reported as macOS metadata
   (`kind: appleDouble`) instead of being installed; the decision is by content,
   so a real package under such a name still installs. A wrapper whose every
   `.knxprod` is AppleDouble holds no package and stays `MissingMaster`. The wrapper's own bytes are not stored either.
   The user keeps the download; KNXBench keeps the product packages.
5. **Budgets.** The wrapper layer gets the full package checks of the chosen
   profile. The inner packages then share **one** expansion budget of that
   profile, so a wrapper never expands more than one package of the same profile
   may. No new limit was introduced.
6. **No nesting.** An inner `.knxprod` that is itself a wrapper is refused.
7. **Surfaces.** `knx products ingest` accepts `.zip` (a project archive under a
   `.zip` name still goes to the project import), and `--allow-large-package`
   applies to vendor ZIPs. The previewed catalogue update (CLI and web,
   [ADR-0117](0117-previewed-recoverable-catalogue-updates.md),
   [ADR-0118](0118-catalogue-updates-through-the-web-server.md)) previews and
   applies wrappers: the preview gains `wrapper` (inner packages with hash and
   "already installed", other members), programs span every inner package, and the
   applied result lists one install report per package. The web keeps the standard
   profile ([ADR-0082](0082-large-product-packages-are-a-cli-opt-in.md)); a wrapper
   whose inner package needs the large profile is refused there with a message
   naming the inner package and installs through the CLI flag.

**[V]** The collection holds 4 such AppleDouble sidecars named `*.knxprod`
next to 56 real inner packages. Without rule 4 they refused their wrappers as a
whole ("invalid product ZIP: missing complete end"), blocking the valid package
beside them.

## Consequences

- The most common real download installs in one step, and the complete database
  of the addendum has an install route (`knx products ingest <file>.zip
  --allow-large-package`).
- Manuals and other files inside a wrapper are named but not kept. That is an
  accepted boundary ([KL-134](../KNOWN_LIMITATIONS.md#catalogue-kl-134)), not a
  silent loss: the install report lists each of them with its digest.
- The apply response of `/api/catalog/update/apply` changed from `report` to
  `packages: [{path, report}]` (`path` null for a plain package). The web is the
  only consumer and moved with it.
- A wrapper that holds both a legacy `.vd5` and packages installs the packages and
  reports the `.vd5`; legacy import of the inner database stays a separate step.

## Verification

`crates/knx-productdb/tests/wrapper_packages.rs` (byte-exact inner packages,
names, other members, atomic refusal, nested wrapper, path checks, shared
budget, duplicate inner package, previewed update), `apps/knx-cli/tests/cli_vendor_zip.rs`,
`apps/knx-cli/tests/cli_large_package.rs` (large inner package refused with the
hint, installed with the flag), `apps/knx-server/tests/http_catalogue_update.rs`
(preview and apply over HTTP, named inner refusal) and
`apps/knx-web/src/CatalogPackageUpdate.test.tsx` (EN/DE wrapper preview, per-package
reports).
