# Standalone manufacturer product database installation

## Status and scope

This design is the first compatibility slice required by `goal.md`. It makes
the five supplied, readable `.knxprod` archives installable into the existing
shared product database and makes that installed catalog usable by the existing
device-catalog browser. It does not claim support for legacy `.vd2`, product
parameter configuration, commissioning, KNX Secure, or generic encrypted or
otherwise undocumented product containers.

The supplied corpus, inspected on 2026-09-09, contains three archives whose
`knx_master.xml` namespace is `http://knx.org/xml/project/11` and two whose
namespace is `http://knx.org/xml/project/20`. All five are ordinary readable
ZIP archives with `knx_master.xml`, `M-xxxx.signature`, and one manufacturer
directory containing `Catalog.xml`, `Hardware.xml`, and application-program
XML. The legacy `.vd2` archive has encrypted entries and cannot be read by the
available ZIP reader without a password.

The exact product-container grammar and `.vd2` encryption/decryption semantics
are not supplied by the KNX Standard corpus. `03_08_10 XML Data Encoding
v01.01.01 AS.md`, lines 9--11, only refers readers to the ETS Help Centre for
the project XML scheme. This design therefore uses the actual corpus plus the
published project-file naming conventions, rather than inventing format or
decryption semantics. `Project Schema23 v01.00.00.md` §4.2.2--§4.2.3 documents
the same one-master-file and `M-iiii/{Catalog,Hardware,...}` partitioning;
`03_01_01 Architecture v03.00.02 AS.md` §6.2 describes a manufacturer product
template as the input to tool-side configuration, not a fully configured
device.

## Decisions

### Product database owns package ingestion

`knx-productdb` gets a public standalone-package ingestion API. It accepts
archive bytes and an original display/source name, inventories ZIP members,
classifies the root namespace of `knx_master.xml`, and returns a structured
install report. It does not depend on `knx-etsproj`, `knx-store`, `knx-app`,
or a UI.

The reader accepts exactly the observed readable shape:

- one root `knx_master.xml` with namespace scheme 11 or 20;
- entries under exactly one or more `M-<four hexadecimal digits>/` roots;
- `Catalog.xml`, `Hardware.xml`, application-program XML, `Baggages.xml`,
  baggage bytes, and manufacturer signatures are preserved as package members.

It rejects with typed, format-specific errors a non-ZIP file, encrypted ZIP
member, traversal/duplicate member name, oversized member, missing master
file, unsupported master scheme, project-part-only archive, and legacy `.vd2`.
An unsupported archive is never relabelled as a malformed ETS project.

### Lossless and atomic storage

The database stores the raw package bytes keyed by SHA-256 and records its
ordered member inventory (path, member SHA-256, size, role). Existing
`source_file` raw blobs remain the identity and lossless backing for parsed
manufacturer entries. The installer validates the complete inventory before
publishing any parsed row and installs every member in one SQLite transaction;
one malformed member leaves no package, blob, parsed row, or report row behind.

Unknown XML and non-modelled baggage/signature data are stored verbatim and are
reported. A repeated identical package is skipped without re-parsing. A package
that carries byte-different data under an existing logical catalog, hardware,
product, H2P, or application-program identity keeps the first accepted row and
records an `IdConflict`; it never silently rewrites the previous row or its
provenance.

### Catalog-to-device boundary

The existing catalog queries and `Command::CreateDevice` remain the only path
to create a domain device. After a successful installation they discover the
new rows without a duplicate catalog representation.

An installed product template is not proof of a configured device. The
`Dynamic` tree and module-instance activation are unimplemented, so creation
returns explicit creation diagnostics whenever it seeds a potential static
communication-object reference rather than a verified configured object.
Missing catalog-to-product, product-to-hardware, H2P-to-program, or
communication-object relations are hard user-visible errors, not a passive
device with empty identifiers. A genuinely programless product remains
creatable and is labelled as such.

The creation response carries the projected tree and structured diagnostics;
the web catalog browser renders them before closing. This replaces the current
silent discard of `EnrichmentIssue`s. It does not make parameter configuration
or Dynamic evaluation supported.

### Entry points

The CLI gains `knx products ingest <file.knxprod|file.vd2>`, retaining its
explicit product-install command and reporting installation details. The HTTP
API gains a multipart product-package install route bounded by the same file
size policy as project import. The web catalog browser gains an "Install
product database" action using the existing filesystem picker and then
refreshes its manufacturer/catalog data. API and UI errors retain the typed
reason verbatim enough for a technician to act on it.

## Acceptance criteria

1. Each supplied readable `.knxprod` installs into an empty database,
   preserves every archive member, stores and verifies raw hashes, and exposes
   at least its actual catalog rows through the existing query API.
2. Reinstalling an identical archive is idempotent. Conflicting logical IDs do
   not change a first-winner normalized row; the conflict is queryable and
   reported.
3. Corrupt, duplicate/path-unsafe, oversized, missing-master, unsupported
   scheme, and encrypted fixtures fail by a named typed error and leave no
   partial normalized state.
4. The supplied `.vd2` is rejected as an encrypted legacy database with no
   claim that it was decrypted, parsed, or installed. Its bytes are not
   silently discarded: the caller receives the archive hash/size in the error
   report where available.
5. A real catalog item from a successfully installed corpus archive can be
   found over HTTP and creates a `Command::CreateDevice` device through the
   normal undo/persistence path. Its product/program references and each
   creation diagnostic are visible to the caller.
6. The browser can install a selected local archive, visibly reports failure
   and creation diagnostics, and refreshes the catalog on success.
7. Compatibility, limitations, roadmap, implementation-status, and gap
   documents describe only the tested scheme-11/scheme-20 corpus result and
   retain Dynamic/module activation and `.vd2` as explicit limitations.

## Non-goals and follow-ups

- No `.vd2` decryption or semantic parsing without verified format material and
  the needed user authority/credentials.
- No claim that all potential communication-object references represent a
  configured runtime device; Dynamic/module evaluation needs its own research
  and design.
- Product package provenance in a newly created project's ETS export manifest
  is a follow-up integration item. Until it exists, the application must state
  that native creation uses local catalog data and must not promise standalone
  package bytes in a later ETS export.
