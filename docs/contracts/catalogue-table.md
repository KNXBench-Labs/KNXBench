# Catalogue table presentation

WP-24 scope, authorised 26 October 2026: a presentation change over the existing
catalogue response, not a catalogue-data migration or a second selection model.

## Columns and ordering

- Product, Manufacturer, Order number and Application are explicit sortable columns.
  Product retains description, language fallback and exact source/binding identities.
- Initial order is Product ascending; clicking the active header reverses it and
  another header starts ascending. The active direction is visible and exposed
  with `aria-sort`. Natural locale-aware comparison uses the UI language.
- Empty display values have a defined fallback: product/manufacturer identity or
  a missing-value dash. The same normalized text drives display and ordering;
  raw catalogue values and authoring payloads are not normalized or rewritten.
- Missing primary or secondary sort values stay last in both directions; exact
  catalogue id breaks ties ascending. Sorting copies the returned array.
- The current server route maps the entire filtered `catalog_items` result; the
  domain adapter and both translated/untranslated queries add no paging or cap
  (`routes.rs::catalog_items`, `domain.rs::catalog_items_impl`,
  `knx-productdb::query::catalog_items`). Sorting is local and sends no new query.
  This is source tracing, not a large-catalogue performance result.

## Interaction and accessibility

Selection, highlight and create/wizard requests remain bound to exact catalogue
ids, never row positions. Search arrows follow visible sorted order; Enter selects
without creating. Refresh preserves a still-present highlight; a removed identity
may fall back to an available row. Sorting changes neither filters nor data.

The table uses native rows, headings and separate selection/header buttons with
contained scrolling. Search stays a native search field. On search-arrow movement, a polite status region
announces the actual highlighted item and row position; it is not an invented
active-descendant relationship to a plain table row. Re-sorting, refresh and
mouse selection do not re-announce it; its last arrow-move message is replaced
on the next arrow move, rather than acting as a live selection indicator.

Primary reference: [WAI-ARIA 1.2, `aria-activedescendant`](https://www.w3.org/TR/wai-aria-1.2/#aria-activedescendant),
checked against its returned normative conditions on 26 October 2026. The
controlled-textbox/searchbox alternative requires a controlled
widget supporting that property and containing the referenced descendant; a plain
native table is not that widget. This source check does not establish actual
Orca or other native screen-reader behavior.

Existing install, preview, cleanup, retry, quantity and creation boundaries remain
unchanged. Browser acceptance uses fictional loopback fixtures, not vendor files,
real hardware or a live KNX bus. No column-resizing or virtualization claim is made.
