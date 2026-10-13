# Product schema and namespace coverage

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-011)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 11. `.knxprod` support is evidenced for schemes 11, 12, 13, 14, 20, and exact-namespace 21

**Update 2026-10-10 (evidence only).** A privately supplied ETS6 project embeds
every application program in exact namespace 23. These include programs ETS
converted from pre-ETS4 sources (`PreEts4Style`, `Legacy*` options) and use
`Script`, `ParameterCalculation`, `ParameterValidation`, `Allocator`, `Repeat`,
`Union`, `Rename`, `Assign`, `Message`, `BinaryData` and image-property load
procedures, but no nested `ModuleDef`s and no duplicate ids within one program
file. This widens the evidence for the exact-23 path (§153) beyond standalone
`.knxprod` files. No status change; it does not replace the application-program
XSD (`IMPORT-06`).

**Current reading, 2026-10-08:** accepted namespaces are 10/11/12/13/14/20
and exact 21/23. Unsupported namespaces, encrypted modern packages and `.vd2`
stay refused; legacy VD3/VD4 offline import has its own L1–L3 path (§128).
Older namespace/blocker statements below are dated history, not the current
admission table. Parser/storage admission is not complete device semantics.

**Update 2026-10-05 (AR06 row reconciliation).** Exact-namespace scheme 23
now also has bounded standalone import/storage/report/replay support; see
[§153](../KNOWN_LIMITATIONS.md#153-master-data-schemes-10-and-23-are-refused-for-standalone-knxprod)
and COMPATIBILITY.md. Statements below that list only 11/12/13/14/20/21 predate
that package. Schemes 15–19/22 still need evidence before broader admission;
the old external-blocker assignment is superseded by the current owner ledger.

**Update 2026-10-05 (ADR-0083).** Exact-namespace scheme 10 is admitted too,
on corpus vocabulary evidence (146 public packages, no name outside scheme
11); see [§153](../KNOWN_LIMITATIONS.md#153-product-scheme10-is-refused-exact23-support-is-bounded).
The accepted set is now 10, 11, 12, 13, 14, 20 and exact 21/23.

**Limitation.** Manufacturer product files in the `.knxprod` container are
readable, as a standalone package independent of any `.knxproj`, for master
data schemes 11, 12, 13, 14, 20 and exact-namespace 21
(`knx_productdb::install_package`). Scheme-21 acceptance is evidenced by
synthetic fixtures and the passing read-only corpus matrix; its additional
fields remain retained/report-only. Schemes 15-19, 22, and any scheme not
listed here remain unmeasured
as a *standalone package* — they can still reach the product database bundled
inside a `.knxproj` that already contains them (see Impact below;
`knx_productdb::ingest_file` performs no scheme/namespace gating). `.vd2`,
a pre-2013 ETS2-era legacy container (SFX/`.vd_`-style, not
the same ZIP/XML family as `.knxprod`/`.knxproj` at all — confirmed by
inspection, it has no `knx_master.xml`), is explicitly and permanently
rejected: `PackageError::LegacyVd2 { sha256, len }` → `"legacy .vd2 product
data is unsupported (sha256 <64 hex chars>, <len> bytes)"`, identified by
filename suffix alone — the archive is never opened as a ZIP, never
decrypted, never parsed. As of 2026-09-11 its bytes are hashed (bounded by
the same `MAX_PACKAGE_SIZE` guard every package is subject to) so the
rejection carries the archive's hash and length as evidence; before that
date the filename check reported no evidence at all. This is a named,
external blocker (a genuinely different, undocumented legacy format), not
an untested general failure.

ZIP member-name encoding is no longer an incidental blocker. The installer
reads the central-directory encoding flag, requires valid UTF-8 when bit 11 is
set, and otherwise follows the ZIP-defined CP437 path. Local/central identity
must agree, and Unicode-path overrides require a supported version, matching
CRC and valid UTF-8 before every path and duplicate check
([ADR-0034](../adr/0034-zip-member-names-follow-declared-encoding.md)).
The eleven-package aggregate that previously stopped at the raw-name UTF-8
check now installs in the local gated regression. Only its count and aggregate
identity commitment are public. This removes that one container-level rejection only; it does not claim lossless
interpretation of manufacturer semantics.

**Measured update, 2026-09-24.** The first read-only compatibility-matrix run
covers 115 package instances (113 unique hashes): scheme 11 (48), scheme 12
(1), scheme 13 (4), scheme 14 (3), scheme 20 (56), and scheme 21 (3). It
records 115 isolated installs and, in deterministic shared order, 113 installs
plus 2 exact-byte deduplications. The final gate passes, including its pinned
23,347 isolated unknown rows, ten scheme-21 field frequencies and aggregate
identity/outcome commitment. These measurements show parser/persistence behavior,
not complete manufacturer semantics.
Public identity is aggregate. Its hash-ordered per-instance records contain
only ordinal, scheme and `{status}` for success/deduplication or
`{status, category}` for rejection; detailed per-instance report-count vectors
stay inside the aggregate commitment. No individual hash, private source name,
or manufacturer identity is published.
This replaces the earlier assumption that no standalone samples existed for
those schemes. Scheme-12/13/14 support is backed by representative synthetic
persistence/rollback tests and the eight real packages, but it does not
establish complete interpretation of all possible files. In particular, observed load-procedure and separator metadata remains
explicitly retained and reported rather than treated as executable semantics.
The targeted scheme-12/14 evidence pass rejects application-program XML deeper
than 1,024 nested elements so a size-bounded but adversarial document cannot
turn path tracking into unbounded memory growth. It also rejects more than
262,144 scanned element/attribute items and 64 MiB of cumulative rendered
path, value, and expanded namespace-name evidence, bounding reconciliation
memory independently of the 64-MiB XML-member limit.
The relational parsers still key attributes by local name, but collisions are
deterministic: an unqualified attribute takes precedence over any qualified
same-local-name attribute in either document order. The qualified values remain
separate namespace-qualified unknown evidence; they do not overwrite the KNX
value or create an order-dependent plain-name report.

Scheme-21 package XML is deliberately narrower than schemes 12/14 for foreign
extensions: every typed member must use the exact scheme-21 element namespace
and unqualified attributes. Other element namespaces or qualified attributes
cause a clear import error and no published package rows. The domain readers
dispatch on local names, so accepting those extensions could otherwise turn
foreign content into typed KNX rows. The three observed scheme-21 packages use
only the canonical namespace and unqualified attributes. Evidence for the
observed scheme-21 fields is retained and reported; it does not implement
load-procedure execution, RF/coupler behavior, or access-policy enforcement.

Note the scope: this is about standalone `.knxprod` *product packages*
(`knx products ingest`, `POST /api/catalog/install`,
`CatalogBrowser.tsx`'s install picker). Full `.knxproj` *project* import
has evidenced coverage at schema 11/20/21/23 (see
[COMPATIBILITY.md](../COMPATIBILITY.md) §2/§3); schema-20 project import rests on
three public ETS 5.7 samples since 2026-10-12 (§177), separately from this
row's scheme-20 `.knxprod` package support.

**Cause.** `.knxprod` is the same XML family as `.knxproj` (both root at
`knx_master.xml`, `http://knx.org/xml/project/{scheme}`, per *Project
Schema23 v01.00.00* §4.2.2-§4.2.3's MasterData/`M-iiii` layout); the earlier
assumption that all schemes ≥ 12 needed a still-unresolved encryption layer
(RESEARCH §10) has been disproven for schemes 11, 12, 13, 14, and 20
specifically. The
observed scheme-13 grammar adds no element or attribute names relative to the
measured scheme-11/20 union, and its four real packages pass isolated and shared
installation. Scheme-12/14-specific load-procedure and separator fields are
retained and reported with aggregate corpus counts, not interpreted as executable
semantics. The passing private matrix shows three scheme-21 packages install in isolation
and shared order under the exact namespace allowlist. Schemes 15-19/22 remain
unmeasured.

**Impact.** A manufacturer's standalone `.knxprod` at scheme 11, 12, 13, 14,
or 20 can be installed directly via `knx products ingest`, the HTTP endpoint,
or `CatalogBrowser.tsx`'s install picker, without needing a `.knxproj` that
bundles it. Scheme 21 is accepted only for the exact observed namespace and
has synthetic and observed parser/persistence evidence; its corpus gate passes.
A `.knxprod` at any other scheme, or a legacy `.vd2`, still has to reach the
product database another way (in practice, from a `.knxproj` that already
contains the application programs it references) or not at all for `.vd2`.

**Lifted when.** Scheme 21's exact-namespace parser/persistence acceptance is
already evidenced synthetically and by the passing corpus gate; remaining work
needs fixture-backed semantic coverage. For schemes
15-19/22, a sample is still required before making any claim. For `.vd2`:
never — it is a structurally different, pre-standard legacy container, not a
variant of the current format needing decryption.

Schemes 12-14 no longer belong to this unsupported set. Their acceptance does
not broaden any neighboring namespace and does not alter the permanent `.vd2`
decision.

**Superseded in part, 2026-09-23.** The current product-database completion
goal put evidenced scheme expansion back in scope. The permanent `.vd2`
decision remains unchanged; no scheme is enabled merely because a sample now
exists.
<!-- END retained catalogue record -->
