# ADR-0072: Admit exact product scheme 23 through the existing strict package adapter

- Status: accepted implementation decision; candidate acceptance/publication tracked separately
- Date: 2026-10-04
- Scope: standalone product-package import, not ETS project export or bus/runtime support

## Context

[Bounded research](../PRODUCT_SCHEME_23_RESEARCH.md) records two original
namespace-23 product packages, eight completely scanned XML members, expanded
namespace/full-ancestor observations and unchanged-importer atomic refusals.
The official Project Schema23 document identifies the namespace but explicitly
excludes complete manufacturer grammar and download-image semantics. Neither
that document nor agreeing element names establish complete product semantics.

The existing typed readers dispatch by local name. [ADR-0036](0036-scheme21-namespace-gate.md)
already protects scheme21 package imports with strict typed-member namespace
validation and package-scoped opaque-field evidence. A lone master-allowlist
entry for 23 would bypass that safety boundary.

## Decision

1. Admit only the exact master namespace `http://knx.org/xml/project/23`.
   Keep unresearched namespaces, including 10, 22 and 24, explicitly refused.
2. Reuse the scheme21 validation path for scheme23. Validate all Master,
   Catalog, Hardware, ApplicationProgram and Baggages XML members against the
   package's exact namespace; refuse foreign elements and qualified attributes
   before any typed member write. Preserve the existing archive/member checks.
3. Extend only the package-scoped supplemental evidence scan. Keep full ancestor
   paths and expanded-name identity; retain and report unfamiliar fields without
   inferring enum, default, reference, device or download semantics. Standalone
   XML and project-import evidence policies do not gain scheme23 admission.
4. Keep existing storage schema, parser/ZIP/work/depth ceilings, transactions,
   source-byte retention, replay validation and UI/API vocabulary unchanged.
   Do not introduce a manufacturer-specific adapter or execute vendor content.

## Verification contract

Native tests require known-subset queryability, exact archive/member retention,
persisted replay, exact opaque paths/samples/occurrences, role-specific namespace
refusals, a nonempty saved seed and a valid late post-write rollback. CLI and
HTTP callers must expose the existing measured-facts contract and preserve
seeded data after refusal. Compiler errors are not REDs or killed mutants.

Mutation controls cover namespace-call wiring, qualified-attribute checks,
master evidence, member scan/depth admission, unresearched namespace admission
and generic/package scope separation. A generic parser can already report a
field used by a supplemental scan: presence-only or suffix-only checks are not
proof that the scan is wired. Require a scanner-owned master witness and the
member scanner's actual late-depth refusal, preserving every surviving attempt.

The original same-profile Release Full853 comparison records 690 equal existing
imports, 161 diagnostically equal atomic refusals and two new exact23 imports.
That receipt stays bound to its source/binaries. Test-only strengthening can be
reported separately after verifying production-input byte identity; changed
production inputs need renewed corpus acceptance. Integrated workspace gates,
review, publication and remote readback remain separate completion requirements.

## Consequences and limits

Known fields in the measured subset are queryable; unknown metadata and original
bytes remain available. This is bounded import/storage support, not complete
manufacturer grammar, ETS parity, signature verification, commissioning or
runtime certification. Scheme10 research and resource-policy work remain open.
No storage migration, dependency upgrade, bus operation or Web-source change is
part of this decision.
