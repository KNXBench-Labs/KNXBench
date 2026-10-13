# PDF output

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-045)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 45. Project documentation export has no native PDF output

**Limitation.** `crates/knx-report` produces HTML only. There is no Rust
PDF renderer anywhere in this workspace, and none is planned.

**Cause.** A deliberate scope decision
(`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
§2, §9): every modern browser already prints to PDF, the document ships
`@media print` rules for exactly that, and a Rust PDF-rendering dependency
would be a large addition serving a button the operating system already
provides. CLAUDE.md: avoid unnecessary dependencies.

**Current state (2026-09-23, T14).** The self-contained document retains its
print-specific stylesheet and the server now exposes the exact HTML through
`POST /api/project/documentation-preview`, so a browser can preview and invoke
its native print/PDF path without a second renderer. The Rust crate still does
not generate PDF bytes.

**Impact.** Headless automation that cannot drive a browser still has no
`knx doc-export ... --pdf` path. Native PDF remains deliberately omitted; the
browser print path is the supported route.

**Lifted when.** Open. No task currently proposes a native PDF renderer —
recorded here as a boundary of the feature, not a gap awaiting a fix.
<!-- END retained catalogue record -->
