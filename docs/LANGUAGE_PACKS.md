# Language packs

This document is for someone who wants to translate the application's user
interface into a language it does not already ship with — a national
language, a regional dialect, or an invented one. It describes a JSON file
format, how to get a starting copy of it, how to import it, and — just as
important — what it cannot translate.

It is not a developer document. For how the catalogue and pack loader are
implemented, read `apps/knx-web/src/i18n.ts` and
`apps/knx-web/src/languagePack.ts` directly; both are commented in detail.

## What "language" means here

Two settings in the application both say "language", and they are
completely independent of each other:

- **UI language** (Settings panel, top field) — what this document is
  about. It controls the text of buttons, labels, panels, toasts and
  dialogs: the application's own chrome.
- **Product data language** (Settings panel, a separate field, added by an
  earlier task) — controls which language a *manufacturer's* parameter
  names, enum labels and communication-object text are shown in, when the
  loaded product database carries a translation for them. A language pack
  has no effect on this at all; see "The honest boundary" below.

The UI language field offers **English** (`en`), **Deutsch** (`de`), and
two included fun-language packs: **Boarisch** (`bar`) and
**Klingonisch/Klingon** (`tlh`). Picker names are fixed self-names rather
than translations in the currently selected interface language. The Klingon
entry deliberately stays exactly `Klingonisch/Klingon`, not `tlhIngan Hol`.

The fun packs contain 335 curated interface strings each, covering the
workbench, settings, project creation, catalog, search, properties, address
table, bus controls and flow view. They are playful partial translations,
not linguistically authoritative or complete translations. Missing messages
and detailed technical/safety notices use the unchanged English fallback;
Settings displays this boundary when either fun language is active.

They ship with the frontend and need no import or settings migration.
Language detection still selects English or German; fun languages are an
explicit choice. They are separate from user-installed packs and cannot be
removed by the pack manager. An imported pack with `bar` or `tlh` replaces
that shipped catalogue, without merging its missing keys with the shipped
copy. Removing the imported replacement reveals the shipped pack again.
The picker still uses the fixed shipped name; exports preserve the imported
pack's actual name, unknown fields and messages. More languages can be added
by importing a pack, as before.

## Getting a starting point: export the template

Settings → the language-pack manager → **"Export English template…"**.
This downloads a JSON file containing every string the application
currently knows, in English, in exactly the shape described below. Start
from this file, not from a blank one — hand-writing the format from
scratch risks a typo in `formatVersion` or `messages`'s shape that gets
the whole pack rejected, and the template guarantees every key this build
recognises is already present with its correct name.

The template's own `tag` is `"en"` and its own `name` is `"English"`.
**Change both before you translate it and before you import it back.**
`"en"` is one of the two built-in languages, and the application always
prefers a built-in catalogue over an installed pack with the same tag —
so a template re-imported with its tag left untouched will sit in the
pack manager doing nothing, silently shadowed by the real English
catalogue. The Settings panel shows a permanent reminder of this next to
the export button, and, separately, warns you after the fact if you ever
do import a pack whose tag collides with a built-in.

## The file format

A language pack is a single JSON object with four required fields and
several optional ones. Real example — a hypothetical Bavarian pack with
two translated strings:

```json
{
  "formatVersion": 1,
  "tag": "bar",
  "name": "Boarisch",
  "englishName": "Bavarian",
  "basedOn": "de",
  "packVersion": "0.1.0",
  "pluralCategories": ["one", "other"],
  "messages": {
    "toolbar.save": "Speichan",
    "toolbar.settings": "Eistejjunga"
  }
}
```

### Required fields

- **`formatVersion`** (number) — which version of this file format the
  pack is written against. The application only reads version `1` today,
  but a pack declaring a *higher* number is still accepted, not rejected
  — see "Forward compatibility" below.
- **`tag`** (non-empty string) — a BCP 47 language tag identifying the
  pack. See "Choosing a tag" below; this is the field with the most rules
  attached to it.
- **`name`** (non-empty string) — how the language names itself. This is
  the text shown in the UI-language dropdown, so write it the way a
  speaker of that language would want to see it (e.g. `"Deutsch"`, not
  `"German"`).
- **`messages`** (object) — a flat map from message key to translated
  string. A key's *name* (`"toolbar.save"`) is defined by this build of
  the application, never by the pack; a pack only supplies the *value*.

### Optional fields

- **`englishName`** (string) — the language's English name, for
  diagnostics only. Never displayed in place of `name`.
- **`basedOn`** (string) — a note to *human readers*, typically another
  BCP 47 tag, recording which existing catalogue you translated from
  (e.g. a Bavarian pack noting `"basedOn": "de"` because you started from
  the German catalogue). **The application never reads this field for
  any purpose.** See "The fallback rule" below — this is worth repeating
  because it is the single most likely thing a translator will assume
  works differently than it does.
- **`packVersion`** (string) — your own version number for your
  translation. Unrelated to `formatVersion`; free-form, not validated.
- **`pluralCategories`** (array of strings) — which plural categories
  (e.g. `["one", "other"]`, or a richer set for languages with more than
  two) your translated strings were written against. Informational only:
  which category is actually selected for a given number is always
  computed at display time from `tag` via the browser's own
  `Intl.PluralRules`, not read from this field.

### Message keys with a plural form

A handful of catalogue keys come in a `.one`/`.other` pair (for example
`documentationExport.summaryWithWarnings.one` and
`...summaryWithWarnings.other`), used for sentences whose wording changes
with a count ("1 warning" vs. "3 warnings"). Translate both halves of
such a pair; if your language's plural system has more categories than
English's two, note it in `pluralCategories`, but be aware — see below —
that the application only ever looks for the specific suffixes this
build's own English catalogue defines (`.one`/`.other`), so extra
categories in a pack's `messages` under a suffix this build never asks
for are simply never looked up.

### Forward compatibility

Any field in the pack's JSON that is not in the list above — whether
because it belongs to a future format version, or because you added your
own bookkeeping field — is preserved unread rather than stripped out.
Re-exporting an installed pack (via its own "Export…" button in the pack
manager) returns exactly what was imported, unknown fields included. The
application only ever *adds* checks on top of the four required fields;
it never removes a field you supplied.

## Choosing a tag

`tag` is validated for **shape**, against the structure BCP 47 (RFC 5646)
defines for a language tag, never against a **registry** of which tags
are officially assigned. This is deliberate: this feature exists partly
so that a dialect or invented language with no ISO code can still be
named coherently.

Concretely, this means:

- A standard two- or three-letter language code with an optional region,
  script or variant is accepted, e.g. `"de"`, `"nl-NL"`, `"pt-BR"`.
- A three-letter code with no country behind it at all is accepted just
  as readily, as long as it is shaped like a language tag — for example
  `"tlh"`, the real ISO 639-2 code for **Klingon**.
- A regional dialect with its own registered subtag is accepted, e.g.
  `"bar"` for **Bavarian**.
- A language with no registered code of any kind should use the
  **private-use** form: a tag starting with `x-`, or a language tag with
  a `-x-` suffix followed by whatever you like, for example
  `"art-x-sindarin"` for a hypothetical Sindarin translation, or a bare
  `"x-mylang"`.

A tag that is not shaped like any of the above — for instance a string
with no resemblance to a language tag at all — is rejected at import
time with a message explaining what a well-formed tag looks like, using
exactly the three examples above (Klingon, Bavarian, Sindarin) as its
own illustration, so the rejection message and this document say the
same thing.

One more shape rule worth knowing: a tag that collides with a *built-in*
UI language (currently `"en"` or `"de"`) is still accepted at import —
the pack is installed — but it can never be *selected*, because the
built-in catalogue for that tag always wins. The pack manager's import
report tells you this explicitly if it happens; see "Getting a starting
point" above for the most common way it happens (forgetting to edit the
exported template's tag).

## What happens to keys that do not match

Every import — successful or not — produces a report shown directly in
the Settings panel:

- **Keys the pack translates that this build also has** are applied.
  Their count is reported as one number.
- **Keys the pack translates that this build has never heard of** are
  *reported, not dropped.* They stay in the pack's stored data (an export
  of the pack later will still contain them) in case a future build of
  the application adds a matching key, but until then they simply have
  nothing to attach to. The full list of these is shown, not just a
  count, since it usually means the pack targets a different version of
  the application and you may want to know exactly which strings are
  affected.
- **Keys this build has that the pack does not translate** fall back to
  English (see below). Only a count and a short sample of their names
  are shown — the full list is deliberately not dumped, since this is
  the normal, expected state of a partial translation, not a problem to
  fix immediately.

A pack that translates 12 of this build's 519 keys is a completely valid,
useful pack. It is installed exactly like a complete one; warnings never
block an import, only inform you about it.

## The fallback rule

**When the active UI language is missing a translation for a given key,
the fallback is always English, and only English.** The chain is exactly
two links long: the active language, then English. Full stop.

This means, explicitly:

- A Bavarian pack (`tag: "bar"`, `basedOn: "de"`) that has not translated
  a given key does **not** fall back to German, even though Bavarian is
  a dialect of German and `basedOn` says so. It falls back to English,
  exactly like every other pack.
- The active language never falls back to any *other installed pack*,
  regardless of similarity, `basedOn` value, or anything else.
- `basedOn` is read by nobody. It exists purely so that a human reading
  the pack file later — you, six months from now, or someone who
  inherits your translation — knows which catalogue you started editing
  from. It has no effect on how the application behaves.

If you are writing a pack for a dialect of a language the application
does not ship in full (German, in the Bavarian example above), be aware
that an untranslated key will show up in **English**, not in the parent
language, however close a relative it is.

### Why UI chrome behaves differently from product data

This project has a second, independent translation surface — the display
of a manufacturer's own parameter names, enum values and
communication-object text, controlled by the separate "Product data
language" setting mentioned earlier. That surface follows the *opposite*
rule: if a translation for the selected language does not exist, it
shows the data's original, untranslated text and does **not** silently
substitute another language.

The two rules look inconsistent side by side, but each is correct for
what it is translating:

- A UI chrome string's English text **is** that message key's actual
  content — it was written in English first, by this application's own
  developers, as the authoritative source text every translation is a
  translation *of*. Falling back to it shows the user real, meaningful
  words instead of a raw key name like `toolbar.save`.
- A manufacturer's parameter text is not "written in English first" by
  anyone associated with this application. It is data that came from a
  specific vendor's product package, in whichever language or languages
  that vendor chose to ship. Silently substituting a different language
  — or worse, a different vendor's or version's English string standing
  in for a missing translation — would misrepresent what the
  manufacturer's package actually says, which matters when that text is
  being used to correctly configure a physical device.

Both behaviours are deliberate. Neither is a bug in the other.

## The honest boundary: what a language pack cannot translate

A language pack, however complete, can only ever supply values for
message keys the application's frontend already asks for. Several things
a user sees are not asked for through this mechanism at all, because they
are composed as plain text on the server and sent to the browser already
finished:

- **A parameter diagnostic's own detail** (the raw text behind the device
  parameter panel's "Copy details" button) — developer-facing by design,
  meant for a bug report, not for reading in your own language. The
  diagnostic's headline sentence above it, once in this same list, closed
  in T14 (2026-09-14): it now carries a `kind` tag your pack's own
  `parameters.diagnostic.*` keys can translate.
- **Session log entries** shown in the Log panel (what happened, when,
  and why).
- **Error messages inside toast notifications** — the joke wrapper around
  an error toast is translated, but the error text it quotes, which comes
  from the server, is not.
- **The generated project documentation export** (the HTML/document
  report produced from an open project) is not language-aware in any
  respect — it has no language setting of its own at all, independent of
  UI language, product data language, or anything else.

Full technical accounting of each of these:
[KNOWN_LIMITATIONS.md §66](KNOWN_LIMITATIONS.md#66-server-composed-prose-and-the-documentation-export-are-not-translated-by-any-ui-language-or-pack--partially-resolved-2026-09-14-t14).

A related instance closed by the same task: if the application *rejects*
your language pack on import (a malformed `tag`, a missing required
field, etc.), the sentence explaining *why* it was rejected is
translated — and, since T14 (2026-09-14), so is the specific reason
quoted inside that sentence, through its own `languagePack.rejection.*`
keys your pack can supply. See
[KNOWN_LIMITATIONS.md §67](KNOWN_LIMITATIONS.md#67-a-rejected-language-packs-own-reason-was-shown-untranslated-inside-a-translated-sentence--resolved-2026-09-14-t14).

Knowing this boundary in advance means that if you spot English text in
an otherwise fully translated screen, you can tell at a glance whether it
is a gap in your pack (a missing `messages` key — check the import
report) or one of the surfaces above that no pack can ever reach (in
which case, it is a known, documented limitation, not something wrong
with your translation).

## Managing installed packs

The Settings panel's language-pack manager lists every installed pack,
each with its own **Export…** button (writes the pack back out exactly as
stored, unknown fields included — useful for sharing a translation with
someone else, or for fixing a mistake like the shadowed-`"en"`-tag trap
above) and **Remove** button. Removing the pack that is currently active
switches the UI language back to English immediately, rather than leaving
the interface pointed at a language that no longer resolves to anything.

Re-importing a pack with a tag that is already installed replaces the
previous copy outright — there is no merge of old and new translations,
so an incremental update should be a full re-export of your work each
time, not a diff.
