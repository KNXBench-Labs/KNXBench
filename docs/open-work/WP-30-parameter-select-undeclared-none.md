# WP-30 — Parameter selection lists offer an undeclared “(keine)” value (#45)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **open, first in the 11 November intake queue.**

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
