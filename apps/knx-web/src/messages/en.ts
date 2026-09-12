// The English UI chrome catalogue — every button, heading, placeholder and
// error message in `apps/knx-web`, keyed by a dotted identifier. This
// object's own type is the source of truth for which keys exist at all:
// `MessageKey` below is derived from it, and `de.ts`'s `Record<MessageKey,
// string>` annotation means the compiler rejects a German catalogue that
// falls behind this one by even a single key.
//
// Ten keys were seeded here by task 1, lifted verbatim from `App.tsx`'s
// toolbar (its job was the catalogue and the plumbing around it, not the
// extraction). Task 2 added three more for the Settings panel's own
// UI-language control (`settings.uiLanguage`, and one `language.*` entry
// per catalogue this file has a sibling for — see `AVAILABLE_UI_LANGUAGES`
// in `uiLanguage.ts`). The rest arrive with the extraction tasks (T25
// tasks 3-5), each adding its own keys to this object and to `de.ts`.
//
// Plural keys use the `key.one` / `key.other` convention: both are ordinary
// entries here (see `i18n.ts`'s `resolvePluralBase` for how a base like
// `"foo"` is matched against `"foo.one"`/`"foo.other"`), picked apart by
// `Intl.PluralRules` at lookup time rather than by any pluralisation logic
// baked into this file.
export const messages = {
  "toolbar.openProject": "Open project…",
  "toolbar.openNativeProject": "Open (.knxdb)…",
  "toolbar.save": "Save",
  "toolbar.saveAs": "Save As…",
  "toolbar.exportProject": "Export to .knxproj…",
  "toolbar.undo": "Undo",
  "toolbar.redo": "Redo",
  "toolbar.search": "Search… (Ctrl+K)",
  "toolbar.log": "Log",
  "toolbar.settings": "Settings",
  "settings.uiLanguage": "UI language",
  "language.en": "English",
  "language.de": "Deutsch",
} as const;

export type Messages = typeof messages;
export type MessageKey = keyof Messages;
