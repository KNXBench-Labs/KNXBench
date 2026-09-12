# T25 addendum — user-importable language packs

Amends `2026-09-12-ui-chrome-language.md`, requested by the user
2026-09-12 after Task 2 shipped. The base plan assumed exactly two
compile-time catalogues. That assumption is now wrong, and the tasks
below replace it.

## What the user asked for

English and German ship with the application as the built-in defaults.
Beyond those, a user adds a language by **importing a language file** —
`lang-nl-NL.json` for Dutch, and so on. The mechanism is deliberately not
limited to the languages a vendor thought of: Klingon, Elvish, Bavarian
and any other dialect or invented language are legitimate imports, and
the application must not be the thing standing in their way.

## What this changes about the existing design

Task 1 built `UiLanguage` as a closed union derived from
`AVAILABLE_UI_LANGUAGES`, with `catalogs` a compile-time
`Record<UiLanguage, Messages>`. That was the right shape for two
built-in catalogues and is the wrong shape for an open set. The change
is additive, not a rewrite:

- **The two built-ins stay exactly as they are** — compile-time typed,
  `messages/en.ts` still the source of the `MessageKey` union, German
  still required by the type system to be complete. Type safety over the
  shipped catalogues is worth keeping, and imported packs cannot have it
  in any case.
- **The active language becomes an open tag**, not a member of a closed
  union: a built-in id, or the tag of an imported pack.
- **Lookup grows one layer**: imported pack, then English. A pack that
  translates 12 of 150 keys is a legitimate pack — the user gets 12
  translated strings and 138 English ones, not an error and not a bare
  key. This is the base plan's D3 fallback rule, unchanged, now with one
  more layer above it.

**The fallback language is always English. There is no other fallback,
ever** (user ruling, 2026-09-12). The chain is exactly two long: active
language, then English. It never passes through German, never through
another installed pack, and never through a pack's `basedOn` language. A
Bavarian pack missing a key falls to English, not to German, even though
German is the nearer language and the pack was almost certainly written
from it. One fallback target means a missing string is always missing in
the same, predictable way, and a user who sees an English word knows
exactly what that means: nobody has translated it yet.

Consequently `basedOn` in the format below is **metadata for the human
author only** — which catalogue they translated from — and the loader
must never treat it as a fallback target. A future reader will be
tempted; the ADR-grade sentence is here so they do not.

Note the deliberate asymmetry with product data (T26/T33), which forbids
any cross-language fallback: there, a fallback would misrepresent what a
manufacturer's package actually says. Here the English catalogue *is* the
key's own content, so falling back shows real words.

## Which text a language pack covers

Everything that goes through the message catalogue, which after Tasks
3-5 is all of the frontend's own words:

- toolbar and menu labels, command-palette entries, keyboard-shortcut
  hints
- panel headings, field labels, column headers, buttons
- placeholders, empty states, and the frontend's own validation and
  error copy
- toast copy, including the occasion-specific jokes
- composed `CreationDiagnostic` sentences (Task 5 moves those from
  server prose into frontend keys precisely so they become translatable)
- `aria-label` and `title` text — a screen-reader user gets the same
  language as everyone else
- language display names, so an imported pack names itself in its own
  language
- future in-application help text (backlog T28), which will live in the
  same catalogue and therefore be covered by the same packs on the day
  it is written

What a pack deliberately does **not** cover, and why:

- **Prose the server composed** — `ParameterDiagnostic` messages, log
  entries, API errors. These are Rust strings; a frontend pack cannot
  reach them. Documented as a limitation with its own backlog entry, not
  quietly missing.
- **Product data** — device, parameter and communication-object text
  obeys its own separate product-language setting, because it comes from
  the manufacturer's package. A German engineer using an English-only
  product package needs the chrome translated and the package left
  alone. Two settings, on purpose.
- **Domain discriminants, KNX group addresses, DPT ids and individual
  addresses.** `Building` as a model discriminant is not text; a
  translated one breaks the project file.

## Task 6 — the pack format, loader and store

**Files:** `apps/knx-web/src/i18n.ts`, `uiLanguage.ts`, new
`languagePack.ts`, plus tests.

**The file format.** A JSON document, conventionally named
`lang-<tag>.json`:

```json
{
  "formatVersion": 1,
  "tag": "nl-NL",
  "name": "Nederlands",
  "englishName": "Dutch",
  "basedOn": "en",
  "packVersion": "1.0.0",
  "pluralCategories": ["one", "other"],
  "messages": { "toolbar.save": "Opslaan" }
}
```

- `tag` is BCP 47 and that is broader than it looks: Klingon is `tlh`,
  Bavarian is `bar`, and anything without a registered code has the
  private-use form (`art-x-sindarin`). Validate the *shape* of the tag,
  not its membership in a registry — rejecting an unregistered tag would
  be exactly the gatekeeping the user asked us not to do. A tag that is
  not even well-formed is still an error, with a message that says what
  a well-formed one looks like.
- `name` is how the language names itself, and is what the Settings
  select shows. `englishName` is optional and for diagnostics.
- `pluralCategories` is optional. `Intl.PluralRules` knows `nl`; it does
  not know `art-x-sindarin`. When the runtime cannot supply rules for a
  tag, fall back to the `other` category rather than throwing, and say so
  in the import report.
- Unknown top-level fields are preserved, not dropped, so a pack written
  for a later format version survives a round trip through this one.

**Import.** Reads a file the user picks, validates, and reports —
following the repository's standing rule that data is never silently
discarded and never silently accepted either. The report names:

- how many keys were recognised and applied
- **keys in the pack that this build does not know** — listed, not
  dropped. These are the interesting ones: they mean the pack was
  written for a different version of the application.
- **keys this build has that the pack does not translate** — counted,
  with the first few named. Not an error; this is what the English
  fallback exists for.
- whether plural rules were available for the tag

A pack that fails validation is rejected with the reason. A pack that
imports with warnings is usable — warnings are information, not a veto.

**Export.** The counterpart, and the thing that makes authoring possible
at all: export the current English catalogue as a template pack, every
key present, values in English, ready to be translated and re-imported.
Without this, writing a pack means reading our source code. Also allow
exporting an installed pack back out, so a user can share or edit one.

**Storage.** `localStorage`, alongside the existing preference keys
(`knx-desktop:ui-language-packs`). 150 short strings per pack is
kilobytes; the 5 MB budget is not a concern for a realistic number of
packs. A pack survives a reload, and removing one is a first-class
action that falls the UI back to English if the removed pack was active.

**Tests:** a valid pack imports and its strings render; an unknown key is
reported and does not break the import; a missing key falls back to
English; a malformed tag is rejected with a useful message; a fantasy tag
(`art-x-sindarin`) imports and works, with plural rules degrading to
`other`; an exported template re-imports cleanly (round trip); removing
the active pack falls back to English; a pack persists across a store
reset.

## Task 7 — the Settings surface for packs

**Files:** `apps/knx-web/src/SettingsPanel.tsx`, tests.

The existing language select grows to list built-ins and installed packs
together, each named in its own language. Beside it: import a pack,
export a template, export or remove an installed pack. The import report
is shown to the user, not swallowed — an import that applied 12 of 150
keys must say so, or the user will think the feature is broken when 138
strings stay English.

**Tests:** an imported pack appears in the select and can be activated;
removing the active pack falls the UI back to English; the import report
renders its counts.

## Task 8 — documentation

**Files:** `docs/` — the base plan's documentation task absorbs this, plus
a new user-facing document describing the pack format.

The format document is written for a *user* who wants to translate the
application, not for a developer: what the file looks like, how to get a
template, what a tag may be (including that invented languages and
dialects are expressly allowed and how to name them), what happens to
keys that do not match, and the honest boundary — the parts of the screen
a pack cannot translate, namely server-composed prose and product data,
and why.

## Gates

Unchanged from the base plan.
