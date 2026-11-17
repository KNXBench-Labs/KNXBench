# ADR 0177: Grid-axis captions resolve their declared text parameter

Date: 2026-11-17
Status: Accepted implementation direction (owner Go "Hier das go", 2026-11-17); branch WIP, not software-accepted or released
Session: WP-05 clean-room
Scope: WP-05 first slice — parameter-driven `Row`/`Column` captions

## Context

ADR-0137 brings declared grids to the workspace but leaves axis
`TextParameterRefId` as raw metadata: the alpha-16 contract explicitly marks
"Declared row/column `TextParameterRefId`" as "explicit unsupported fallback; do
not borrow a neighbouring host's text value", and KL-003 lists
parameter-driven captions as not reproduced. The axis `Text` itself is
already projected (source literal plus program-owned `Text` translations,
`parameter_grid_axis_captions`); 622 of 1,513 rows and 2,951 of 3,163 columns
in the census have no `Text` at all, so a caption driven by a text parameter is
the only visible heading for those axes.

Evidence for the semantics:

- **[D]** KNX Association, Manufacturer Tool help *Dynamic text change*
  (support.knx.org article 360000160080): ETS inserts the value of a text
  parameter chosen with `TextParameterRefId` into a host's text via
  `{{0}}`/`{{0:def}}`; `def` shows when the value is empty. The article names
  `Channel`, `ParameterBlock`, `ParameterSeparator`, `ParameterRef` and
  `ComObjectRef` — not `Row`/`Column`.
- **[D]** ETS v5.7.3 release notes (support.knx.org article 360009162540):
  "Support for dynamic update of TextParameterRef labels in table cells" and
  "XML default language texts are not used in table row- and column headlines"
  (fixed in v5.7.7, article 4409114300178). So ETS does render
  `TextParameterRef`-driven labels in table row/column headlines.
- **[D]** Public schema-20 XSD (see
  [parameter-workspace-layout.md](../research/2026-10-17-parameter-workspace-layout.md)):
  `Rows/Row` and `Columns/Column` carry `TextParameterRefId` alongside `Text`.
- **[A]** The exact mechanism on an axis — whether the axis `Text` carries
  `{{0}}` filled from the referenced parameter, or whether the parameter value
  replaces the caption outright when `Text` is absent — is **not documented**
  in the local corpus. The census measured zero axis `Text` values containing
  `{{` placeholders, so the corpus cannot distinguish the two readings.
  Assumption, labeled: an axis caption is the referenced text parameter's
  value when the axis declares `TextParameterRefId` (stored value, otherwise
  product default, otherwise empty), independent of the axis's own `Text`;
  this matches "TextParameterRef labels in table cells/headlines" and the
  observed absence of `{{0}}` in axis texts. The existing positional
  `{{0}}`/`{{0:def}}` resolver (ADR-0126) is reused per axis `Text` only if a
  future evidence source shows placeholders there.

## Decision

1. **One resolver, reused.** `Row`/`Column` captions with a declared
   `TextParameterRefId` resolve through the existing ADR-0126 machinery:
   `TextParameters`/`DeviceTexts` in the program scope (axes are literal
   program-owned declarations, never inside a module or `Repeat`), value =
   device stored value, else product default, else empty string. The resolved
   text replaces the axis caption; the original `Text` stays on the wire as
   metadata. No neighbouring host's value is ever borrowed.
2. **Wire (additive, versioned).** `ParameterGroupLayoutDto` stays
   `version: 1`; `rows[]`/`columns[]` gain an optional `textFill`
   (`TextFill`, same as separator labels) describing the fill's source. The
   existing `text` field carries the resolved caption; `textParameterRefId`
   stays verbatim. No version bump: version-1 consumers that ignore the new
   field keep rendering `text`.
3. **Fallbacks stay explicit.** A reference naming no `ParameterRef`, a
   non-text parameter, or an undecidable scope leaves the axis's own `Text`
   (or none) and reports `textFill.source` as `unknownParameter` /
   `notTextParameter` / `undecided`; `noReference` never occurs here (the
   attribute is present by construction). The frontend shows the same
   provenance tooltip as separator labels.
4. **Budget unchanged.** All fills charge the one request-wide sticky
   `TextProjectionBudget`; exhaustion refuses the panel, never truncates.
5. **Translated axis captions stay as delivered.** A program-owned `Text`
   translation still overlays the literal `Text` first (alpha-16 rule);
   the parameter-driven fill applies to the result only when the axis
   declares `TextParameterRefId`, and a translated template containing
   placeholders remains the existing unsupported fallback.

## Consequences

- The WP-05 open item "Parameter-driven row/column captions
  (`TextParameterRefId`)" is addressed for program-owned literal axes; module
  and `Repeat` axes remain out of scope as before.
- No product-database schema change: axis metadata is re-read from the
  retained source, values come from the existing `parameter_ref`/
  `parameter` tables and the device evaluation.
- Undefined cases (placeholder in axis text, module-scoped axis) stay
  visible as unsupported rather than guessed, per the WP-05 done-when rule.
