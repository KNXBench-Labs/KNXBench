# Report name resolution

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-046)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="46-project-documentation-export-does-not-resolve-manufacturer-product-or-program-names"></a>
## 46. Project documentation export resolves names only with installed product data — partially resolved 2026-09-23 (T14)

**Current state.** Server and CLI callers now resolve manufacturer, product and
application-program names through `knx_productdb::query::device_product` and
pass only display data into pure `knx-report`. Raw `product_ref` and
`program_ref` remain beside the names as provenance. The query accepts a
program link only when its `hardware_id` equals the product's hardware; a
mismatch cannot contribute another product's program name or parameters and is
reported with both raw references. When the database is absent, a reference
does not resolve, or a joined name is absent or whitespace-only, the available
raw reference is used as the cell value and the renderer returns a
`ReportWarning`; the cell is never silently blank.

**Cause.** `crates/knx-report` depends only on `knx-core`, `knx-projection`,
and `chrono` (`xtask check-layering` enforces this, the same rule
`knx-csv` is held to). Resolving those identifiers to a human-readable
name requires querying `knx-productdb`, a separate, independently
versioned database this crate must not reach.

**Impact.** Reports made without the matching installed product package cannot
invent human-readable names. They remain complete but carry explicit warnings.

**Lifted when.** The data-dependent part cannot be lifted globally: product
packages are optional. The architectural gap is closed; missing external data
is now an explicit per-device report condition.

**Update 2026-10-06 (AR15): accepted boundary, rechecked.** Behaviour as
described above, covered by `knx-report`
`unresolved_identity_and_unrenderable_value_warn_and_remain_inline` and
`knx-app` `mismatched_product_and_program_never_cross_attribute_another_devices_program_data`
and `blank_identity_parameter_and_module_argument_names_fall_back_to_raw_references`
(all green in the 2026-10-06 workspace run, 3,311 passed). Since AR10 slice 3
the report also uses the requested product language with the same rule as the
device detail. Names stay data-dependent: no product package, no name.
<!-- END retained catalogue record -->
