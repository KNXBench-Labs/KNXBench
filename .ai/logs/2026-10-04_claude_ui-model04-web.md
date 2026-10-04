# 2026-10-04 — MODEL-04 web half: catalog address allocation and unique names

Agent: Claude, goal-ui.md owner session. Web lock taken for this package only
(`0028a58c`).

## Contract (server half, already published)

`POST /api/devices` accepts `allocateAddresses` (needs `lineId`, else 400) and
`uniqueNames`. Both default to `false` and are part of the ADR-0069 replay
fingerprint. Allocation uses `knx_core::free_line_addresses`: lowest free
octets 1–255, skipping .0, used and excluded addresses. A short supply refuses
the whole batch (`line A.L has F free device addresses, R requested`). Each
created item carries `address` (null without allocation).

## Change

- `api.createDevice(..., requestId, options)` with `CatalogCreateOptions`;
  only `true` options are serialised. `CreatedCatalogDevice.address` is typed.
- `CatalogBrowser`: an *Options* fieldset with two unchecked checkboxes. The
  allocation checkbox is disabled, with a hint, without a target line, and the
  submitted value is additionally gated on `lineId !== null`. The preview
  explains both rules. The created-device list shows the allocated address.
  The options are part of the stored request, so a DATA-03 retry resends them
  unchanged.
- Messages en/de: `catalog.optionsLegend`, `catalog.allocateAddresses`,
  `catalog.allocateNeedsLine`, `catalog.uniqueNames`,
  `catalog.addressAllocated`, `catalog.uniqueNamesNote`, `catalog.itemAddress`.

## Evidence

- RED: 8 failing cases. These are 5 new ones, the quantity test's 6th-argument
  expectation, and 2 old tests that failed because queued `mockResolvedValueOnce`
  answers leaked from the stopped RED tests. That leak is fixed in `beforeEach`
  (mock reset), not by changing the old tests.
- GREEN: `CatalogBrowser.test.tsx` 28/28, the `api.test.ts` file green, tsc
  clean.
- `e2e/catalog-allocation.e2e.ts` reuses the DATA-03 fixture page (line 7).
  Its 4 intercepted Chromium cases (en/de) check the POST body, the listed
  addresses and the refusal path. All 4 fail against the previous client
  (negative control).
- Mutants, each caught by a named test, with the sources restored byte-exact:
  allocation sent without a line, checkbox enabled without a line, unique
  names dropped on the wire, address not shown.
- Full gate (first attempt): web build, fmt, clippy -D warnings, workspace tests 3,145 passed / 0 failed / 177 ignored in 169 blocks with 0 skip markers, four repository gates (headers 462 ok, ceiling 157; anchors 393 ok), tsc, Vitest 1,774/100 files, complete intercepted Chromium suite 102/102, whitespace; source frozen.

## Boundaries

A single device without diagnostics still closes the dialog at once; its
address is visible in the project tree. The allocator sees only the project,
not the real bus. Offline only, no KNX or bus contact.
