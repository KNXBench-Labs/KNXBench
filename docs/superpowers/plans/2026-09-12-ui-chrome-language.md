# T25 — Multi-language UI chrome

## Problem

Every word `apps/knx-web` puts on screen is a hard-coded English literal.
A measured scan of the 30 non-test source files (11,300 lines) found ~139
user-visible literals: JSX text, `title`/`aria-label`/`placeholder`
attributes, button labels, empty states and error copy. There is no
message catalogue, no locale detection, no UI-language setting, and no
i18n dependency in `package.json`.

This is the chrome half of gap **D10** and part of **D8**. The data half
shipped across T26 (parameter text), T32 (catalog/hardware/master
translations) and T33 (communication-object text): a user can already read
their *product data* in German while every button, heading and error
around it stays English. That is the gap this plan closes.

German is the second locale: the language of the KNX Association's own
documentation, of the sample projects in `OriginalData/`, and of this
project's users.

## Spec

A user picks a UI language in Settings, or gets one detected from the
browser, and the application's own chrome renders in that language.
Product data keeps obeying its own separate product-language setting —
the two are independent on purpose, because a German-speaking engineer
working with an English-only product package needs the chrome translated
and the package left alone.

Text the server composes as prose stays in the language the server wrote
it in. That boundary is documented, not papered over.

## Decisions this plan makes

**D1 — No i18n library.** ~139 strings, one plural rule, a handful of
interpolations. `Intl.PluralRules` and `Intl.NumberFormat` are in the
platform already. A dependency would buy message extraction tooling we do
not need for a catalogue this size, and `CLAUDE.md` names unnecessary
dependencies as a thing to avoid. Revisit if a third locale or a
translator workflow arrives.

**D2 — Storage mirrors the existing preference modules.** `theme.ts`,
`motion.ts` and `productLanguage.ts` are all the same shape: a
module-level `useSyncExternalStore` value backed by a `localStorage` key,
lazily seeded on first `getSnapshot()` (because `happy-dom` only sets up
`window.localStorage` per test file). `uiLanguage.ts` is the fourth
sibling, key `knx-desktop:ui-language`. No new state-management approach.

**D3 — A missing translation falls back to English, and this is
deliberately the opposite of T26/T33's rule.** Product-data translation
must never fall back to another language, because a user reading a
device's parameter has to know the text came from that package and not
from a guess. UI chrome is the reverse case: the English source string
*is* the message key's own content, so falling back shows real words
instead of a bare key. The asymmetry is intentional and gets one sentence
in the docs so nobody "fixes" it later.

**D4 — Server-composed prose is out of scope and stays English.**
`ParameterDiagnostic` (`message`/`detail`, `apps/knx-web/src/api.ts:898`),
`LogPanel`'s `entry.message`, `Toast`'s API-error messages and
`knx-report`'s generated documentation are all composed in Rust. A
frontend catalogue cannot translate them. `CreationDiagnostic` is the
exception and **is** in scope: it already ships a structured `kind`
(`apps/knx-web/src/api.ts:276-286`) plus its fields, so the frontend can
compose the sentence itself and translate it. The rest becomes a
documented limitation and a follow-on backlog entry, not a silent gap.

**D5 — Domain discriminants are not translatable text.** `Building`,
`Floor`, `Room`, `Corridor`, `DistributionBoard` and friends appear in
`ProjectExplorer.tsx` both as the domain model's type discriminant and as
a rendered label. Only the rendered label is translated; the
discriminant's string value must survive untouched. Any task that
confuses the two has broken the project model.

## Global Constraints

1. **The catalogue is typed.** A message key that does not exist is a
   TypeScript error, not a runtime surprise. `npx tsc --noEmit` is the
   coverage mechanism for "every key exists".
2. **No key is looked up dynamically from an untyped string.** If a call
   site needs to pick between messages, it picks between keys, so the
   type checker can still see all of them.
3. **Locale detection never overrides an explicit choice.** Detection
   seeds the default only when `localStorage` holds nothing.
4. **`de-DE` is complete before the task that adds it is done.** A
   half-translated catalogue that silently falls back to English hides
   its own gaps — the fallback exists for robustness in the field, not as
   a workflow.
5. Accessibility text (`aria-label`, `title`) is translated with the same
   rigour as visible text. A screen-reader user gets the same language as
   everyone else.
6. Every new or edited source file carries a version and a one-sentence
   purpose header, per repository convention (noting that `apps/knx-web`
   has no such header today — follow the file's existing local style
   rather than inventing one for this slice alone).
7. Commit messages in the repository's established gloomy style. Facts
   stay accurate. No `Co-Authored-By` trailer (`CLAUDE.md`).

## Task 1 — the catalogue and the store

**Files:** new `apps/knx-web/src/i18n.ts`, new
`apps/knx-web/src/uiLanguage.ts`, new `apps/knx-web/src/messages/en.ts`,
new `apps/knx-web/src/messages/de.ts`, plus tests.

- `messages/en.ts` exports a `const messages = { ... } as const` object
  of message key to English string. Seed it with ten keys drawn from real
  call sites (`App.tsx`'s toolbar buttons will do) — the remaining keys
  arrive with the extraction tasks. The exported type of this object is
  the key type the rest of the app checks against.
- `messages/de.ts` exports the same keys, typed `Record<MessageKey,
  string>` so a missing German key is a compile error.
- `i18n.ts` exports `useTranslate()` returning a `t(key, params?)`
  function: catalogue lookup for the active UI language, English on a
  miss (D3), and `{name}`-style placeholder substitution. Plurals go
  through `Intl.PluralRules` with a `key.one` / `key.other` convention —
  German and English share the same two categories, but the rules object
  is what makes a third locale possible without rework.
- `uiLanguage.ts` mirrors `productLanguage.ts` exactly: lazy seed,
  `useSyncExternalStore`, `knx-desktop:ui-language`, a
  `resetUiLanguageForTests()` equivalent if that module has one. Default
  when nothing is stored: `navigator.language` matched against the
  available catalogues by primary subtag (`de-AT` → `de`), else `en`.

**Tests:** `t()` returns the active language's string; falls back to
English for a key the German catalogue is missing at runtime (construct
this case by casting, since the type system otherwise forbids it — the
test documents the runtime behaviour D3 promises); placeholder
substitution; both plural branches; detection picks `de` for `de-AT` and
`en` for `fr-FR`; an explicit stored choice beats detection.

**Acceptance:** `npm test` and `npx tsc --noEmit` clean in `apps/knx-web`.

## Task 2 — the Settings control and detection wiring

**Files:** `apps/knx-web/src/SettingsPanel.tsx`, and whichever component
renders the app shell.

Add a UI-language select beside the existing product-data language
control, listing the catalogues that exist and nothing else. Selecting
one writes through `uiLanguage.ts` and every mounted component re-renders
from the shared store — no remount, no prop drilling. Set the document's
`lang` attribute from the active language so the browser and assistive
technology agree with what is on screen.

**Tests:** the select shows the active language; changing it re-renders
an already-mounted component in the new language; `document.documentElement.lang`
follows.

## Tasks 3-5 — extraction

Three tasks, split by file so they never touch the same file, each one
"replace every user-visible literal in these files with a catalogue key,
add the English and German entries, update the tests that assert on the
old literals":

- **Task 3:** `App.tsx`, `commandRegistry.ts`, `Dashboard.tsx`,
  `DocumentationExportButton.tsx`, `Toast.tsx`, `toastCopy.ts`.
  `toastCopy.ts`'s 18 occasion strings come in randomized pairs — the
  German catalogue has to preserve the pairing, and a translated joke
  that stops being a joke is worse than an untranslated one. Translate
  for effect, not word-for-word.
- **Task 4:** `Inspector.tsx` (the largest single concentration, ~29
  literals), `ParameterPanel.tsx`, `ProjectExplorer.tsx`. Inspector
  repeats "Delete is only available for … in the first installation."
  once per entity type — collapse those into one parameterized message.
  `ProjectExplorer.tsx` is where D5 applies: translate the label, never
  the discriminant.
- **Task 5:** `LogPanel.tsx`, `BusMonitorPanel.tsx`, `CatalogBrowser.tsx`,
  `CommandPalette.tsx`, `Search.tsx`, `SettingsPanel.tsx`,
  `ProjectDiffPanel.tsx`. `ProjectDiffPanel`'s count summaries are the
  plural case Task 1 built the rules for. `CatalogBrowser` also renders
  `CreationDiagnostic` — compose those sentences in the frontend from
  `kind` and its fields per D4, and translate them.

Each task runs `npm test` and `npx tsc --noEmit` before reporting.

## Task 6 — documentation reconciliation

**Files:** `docs/GAP_ANALYSIS_ETS.md` (D10, D8, a `T25. Done` entry in
Tier 6), `docs/KNOWN_LIMITATIONS.md`, `docs/IMPLEMENTATION_STATUS.md`,
`docs/ROADMAP.md`.

- Record precisely which surfaces are translated and which are not.
  Server-composed prose stays English (D4) — that is a new documented
  limitation with its own backlog entry, not a footnote.
- D10 closes only if both halves really are closed. The hardware- and
  master-scope translations §64 names are still read by nothing, so say
  what remains.
- Record the measured literal count and the per-file distribution as
  measurement, not recollection. If the implementer's count disagrees
  with this plan's ~139, the implementer's count wins.
- Do not claim the generated documentation export is language-aware. It
  is not.

## Gates before merge

`cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D
warnings`; `cargo test --workspace`; `cargo run -p xtask --
check-layering`; `cargo deny check`; `npm test` and `npx tsc --noEmit` in
`apps/knx-web`.
