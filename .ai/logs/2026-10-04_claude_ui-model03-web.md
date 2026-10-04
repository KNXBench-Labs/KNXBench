# 2026-10-04 — MODEL-03 web half: device number 0 for evidenced couplers

Agent: Claude, goal-ui.md owner session. Web lock taken for this package only
(`959605a2`), after the commissioning session released it at 17:06.

## Contract (published backend, UA1)

`POST /api/individual-address` (`deviceId`, `address`). For a new device
octet `0` the server uses `SetCouplerIndividualAddress` only when the
installed product has `Hardware/@IsCoupler="true"` (RESEARCH §25); otherwise
the core refuses with "address … ends in 0, reserved for couplers; …". Undo,
redo, line-prefix and uniqueness checks are covered by
`apps/knx-server/tests/coupler_address.rs`.

## Change

`apps/knx-web/src/Inspector.tsx`: the line-bound address editor no longer
refuses `0` before asking the server, because only the server can see the
product database. The server's refusal is shown in the field like any other
address error, and the field returns to its previous value. The now unused
`inspector.address.couplerOnly` message was removed from both catalogues.

## Evidence

- RED first: `Inspector.test.tsx` "submits device number 0 so the server can
  accept an evidenced coupler" and "shows the server's refusal when device
  number 0 is not an evidenced coupler" failed against the old client block;
  the non-device-number test keeps 256, 1.2 and -1.
- RED first: `e2e/coupler-address.e2e.ts`, 4 intercepted Chromium cases
  (en/de × accepted/refused), all failing at the expected write before the fix.
  After the fix they pass with the existing device-editor cases (14/14). Every
  API request is intercepted, and unexpected routes are recorded and asserted
  empty.
- Mutants, each caught by a named assertion and restored byte-exact: client
  block restored; generic text instead of the server refusal; field not
  restored after refusal.
- `tsc` covers `src` only; e2e files are not type-checked, which is the
  repository convention.
- Full gate, attempt 2 on the corrected candidate: web build, fmt, clippy -D warnings, workspace tests 3,145 passed / 0 failed / 177 ignored in 169 blocks with 0 skip markers, four repository gates (headers 459 ok, ceiling 157), tsc, Vitest 1,763/100 files, the complete intercepted Chromium suite 94/94, whitespace; source frozen. Attempt 1 failed only `check-headers`: the new e2e header was 106 columns, the limit is 100. Shortened to 88 and re-gated in full; the failed attempt is kept, not relabelled.

## Boundaries

Offline only, no KNX or bus contact. No backend change. Real coupler products
and ETS behaviour are not exercised; acceptance rests on the product-database
flag and the server tests.
