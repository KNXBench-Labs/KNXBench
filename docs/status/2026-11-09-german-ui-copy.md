# German UI and device parameter metadata (9 November 2026)

## Scope

The device parameter editor previously rendered the server's parameter kind,
value source and access tokens directly. The surrounding labels were German,
but values such as `Number`, `Restriction`, `Stored`, `ProgramDefault`, `Read`
and `ReadWrite` bypassed the message catalogue.

Known metadata now has typed English/German catalogue keys shared by the
parameter row's access badge and the parameter workspace's information pane.
For example, German shows **Auswahl**, **Im Projekt gespeichert**,
**Programmstandard**, **Nur lesen** and **Lesen und Schreiben**. English keeps
its existing token labels. Original tokens remain in tooltips; unknown/new
metadata, including prototype-like names, remains verbatim rather than being
assigned a guessed meaning.

The German catalogue also replaces leftover English UI wording in project
history, achievements, appearance settings, activity records, CSV summaries and
download guidance. This changes display copy only, not confirmation phrases,
commands or safety prerequisites. The catalogue job field `Kind` correctly
becomes **Art**.

## Data and language boundary

No manufacturer caption, group title, enum option, parameter value or imported
identifier is rewritten. UI language and product-data language remain separate
preferences. The product database's existing translation/fallback rules are
unchanged. Missing manufacturer translations are not repaired by inventing UI
catalogue entries. Technical diagnostic details and unknown tokens stay intact.

The bundled Bavarian and Klingon packs remain intentional partial overlays
with English fallback. Their current catalogues each contain 334 entries;
this package does not claim complete translation of either pack.

## Regression coverage

- `src/parameterMetadata.test.tsx`: all known kind/source/access labels, unknown
  and prototype-like tokens, mounted parameter badge and information pane,
  live UI-language switching, restricted fields, exact manufacturer text/value
  preservation and no parameter writes during presentation/language changes.
- `src/i18n.catalogues.test.ts`: actual built-in catalogue key parity, nonempty
  text, exact placeholder multisets (including singular forms), plural pairs,
  corrected German vocabulary and intentional partial fun-pack coverage.
- `e2e/devices-navigation.e2e.ts`: original-token tooltips with localized
  parameter metadata in English/German at 400/1440 px, retaining the existing
  editor-identity, navigation, mutation and overflow checks.

## Verification history

The first integrated package gate stopped at Vitest: 5,466 tests passed and one
new catalogue-guard test failed. The guard had mistaken four existing
non-plural action/diagnostic keys ending in `.one`/`.other` for incomplete
plural pairs. The correction names those exact keys and adds positive/negative
controls; real missing halves and exact singular placeholder multisets remain
checked. This is a test correction, not a manufacturer-data or runtime change.

The focused four-file regression run then passed all 65 tests. Three deliberate
faults in an isolated source copy were detected by assertions: bypassed metadata
translation, an English German-catalogue value, and a renamed singular count
placeholder. Restoring the exact original source returned all 65 tests to green.
The original failed gate is retained separately; it is not counted as acceptance.

These are software/UI checks using fictional data and intercepted APIs. No bus,
device, native-shell, release or deployment acceptance follows from this change.
