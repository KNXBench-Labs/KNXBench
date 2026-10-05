# AR10 language markers in the parameter panel (goal-ui owner, 2026-10-05)

Worktree `ui-ar10` on `origin/main` `52def305` (web lock taken there),
released by this commit. Consumer of Alpha's AR10 slice 2a (`b6a94c24`):
`sourceLanguage`, `textLanguage`, `nameLanguage`, `enumOptions[].language`.

## Presentation decision

- Markers only with a product language selected: under *Package default* the
  package's own text is exactly what was asked for.
- Only the label actually shown counts (`text` before `name`, the panel's
  existing rule). `null` answering language → badge "Untranslated
  (<sourceLanguage>)"; with no declared `DefaultLanguage` just "Untranslated"
  (never guessed). A translated label with an untranslated option label →
  "Options untranslated (…)". Value-only options and id-only labels are not
  translation gaps. Badge `title` names the selected language.
- One panel line counts affected fields across all sections, folded
  `Access=None` fields included; none when the count is 0.
- en/de strings; manual 09 "Product data language"; KNOWN_LIMITATIONS AR10
  note; IMPLEMENTATION_STATUS. `alpha-release-goal.md`'s "UI owner: decide how
  the markers are shown" is Alpha's file and left for Alpha to tick.

## Evidence

- RED: 3 of 5 new cases failed on the old panel; the two absence cases
  (no product language; fully translated) are pinned by mutants.
- Mutants 10/10 (first sweep 9/10: value-only option counted survived → added
  case G, rerun killed).
- `api.ts` fields required (wire always sends them); typed fixtures updated;
  `tsc -b` reported the missing fields before they were added.
- Gate (leases 7/8/9, inputs frozen, head `52def305`): build, flow-study,
  theme-fixtures 0; Vitest 2,029 / 116; Chromium 134; check-layering,
  check-headers (155 / ceiling 155), check-anchors 549, check-ledger 188,
  check-corpus-gates all 0; diff-check clean.

No server change. No hardware, no bus, no KNX socket.
