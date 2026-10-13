# Report parameter semantics

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-047)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="47-project-documentation-export-does-not-list-parameter-values-or-module-instance-arguments"></a>
## 47. Project documentation export lists parameter values and module-instance arguments — partially resolved 2026-09-23 (T14)

**Current state.** The Devices section lists stored parameter values and
module-instance arguments in deterministic project order. With matching
product data, parameter names, translated enum display text and module argument
names are added. Raw identifiers and raw values are always printed. Blank
parameter/module-argument names fall back to those identifiers and warn.

**Cause.** Both remain raw in `knx-core`; the outer `knx-app` composition
layer now joins what `knx-productdb` can prove and passes display-only rows
into `knx-report`. This preserves the renderer's dependency boundary.

**Remaining limitation.** A module-qualified parameter whose stored identifier
cannot be matched to a declaration, or a parameter kind without a meaningful
display formatter, is shown raw with both an inline explanation and a
`ReportWarning`. A restriction value missing from its declared enumeration is
handled the same way. Raw module argument values are not semantically
interpreted; `AllocatorRef` and unknown argument kinds remain unsupported, as
do allocation metadata and repeat semantics. The generated document's
mandatory limits section states this boundary.

**Lifted when.** Additional verified parameter/module semantics exist for the
remaining warned cases. T14 deliberately does not invent them.
<!-- END retained catalogue record -->
