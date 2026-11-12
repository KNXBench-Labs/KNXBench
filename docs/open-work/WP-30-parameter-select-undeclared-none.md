# WP-30 — Parameter selection lists offer an undeclared “(keine)” value (#45)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
**Closed 12 November 2026** (removed from [OPEN_WORK](../OPEN_WORK.md); issue closed where one exists). Status: **implemented 11 November 2026**: only declared values are choosable; a
missing or undeclared current value shows as a disabled placeholder; the server refusal
of `""` is pinned by `restriction_accepts_only_declared_values`. Independent review APPROVE.

## Observed

Every selection-list parameter in the device parameter editor offers
“(keine)” (`parameters.none`), whether or not the product declares such a
value. Choosing it would try to store a value the device never defined.

## Cause (source)

`apps/knx-web/src/ParameterPanel.tsx` renders `<option value="">` ahead of
`field.enumOptions` for every `Restriction` parameter.

## Requested outcome

- Selection lists offer only the declared enumeration values.
- A placeholder appears only when the current value is missing or not one of
  the declared values; it is not selectable, so the mismatch stays visible.
- The server refuses an undeclared enumeration value (check the existing
  behaviour first; add a test either way).
