# KL-37 — slice-2b language markers on screen (goal-ui owner, 2026-10-05)

Worktree `ui-kl37` on `origin/main` `6fa10eb8` (web lock taken in `942d2680`,
released by this commit). Consumer of Alpha's AR10 slice 2b (`c4883242`).

## Change

- `src/languageFallback.tsx` (new): `fellBack(text, answered)` — shown text
  whose answering language is `null` or absent — and `LanguageFallbackBadge`
  ("Untranslated (<declared source>)", or "Untranslated" when none is
  declared; "Options …" variant for the parameter panel). The parameter
  panel's own badge moved here; keys `parameters.untranslated.{label,…,title}`
  → `untranslated.*`, tooltip "the original text".
- `CatalogItem`: `nameLanguage`, `visibleDescriptionLanguage`,
  `sourceLanguage`; `CatalogBrowser` marks name and description.
- `Inspector`: product text, catalogue name, application name (each with its
  `*_language` / `*_source_language`); DPT text marked without a source (no
  master default language on this wire). Manufacturer and hardware names have
  no marker on the wire and get none.
- Markers only with a product language selected.
- Docs: KNOWN_LIMITATIONS §37 update, manual 09, IMPLEMENTATION_STATUS; ledger
  `KL-37` WAITING_OWNER → ACCEPTED_BOUNDARY (Alpha's slice 3 accepted the
  backend residue and left only this UI half), snapshot counts
  WAITING_OWNER 4→3, ACCEPTED_BOUNDARY 106→107.

## Evidence

- RED: catalogue and product-block cases failed on the old code; the three
  parameter-panel cases failed on the renamed shared class until wired. The
  absence cases (no product language; translated DPT text) are pinned by
  mutants.
- Mutants 11/11: answered-ignored, absent-text-counted, source-never-named,
  catalog-name-answer-ignored, catalog-description-unmarked,
  catalog-without-language, identity-without-language, identity-source-dropped,
  identity-wrong-source, dpt-answer-ignored, dpt-without-language.
- Gate (leases 7/8/9 after Alpha's KL-151 measurement, inputs frozen, head
  `6fa10eb8`): build, flow-study, theme-fixtures 0; Vitest 2,044 / 116;
  Chromium 134; check-layering, check-headers (155/155), check-anchors 554,
  check-ledger 188, check-corpus-gates 0; diff-check clean.

No server change. No hardware, no bus, no KNX socket.
