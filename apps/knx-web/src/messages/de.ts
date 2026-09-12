// The German UI chrome catalogue. Typed `Record<MessageKey, string>`
// rather than `as const` like `en.ts` — deliberately: this is the
// catalogue the compiler holds to *English's* key set, so an extraction
// task that adds a key to `en.ts` and forgets this file gets a compile
// error here, not a silent runtime fallback discovered by whoever next
// opens the app in German.
//
// The runtime does still fall back to English for a German key missing
// *at runtime* (a corrupted build, a key deleted from here but not from
// `en.ts` via some future refactor that weakens this type) — see
// `i18n.ts`'s `translate()` and its test. That fallback exists precisely
// because this compile-time guarantee is the normal case, not the only
// line of defence.
import type { MessageKey } from "./en";

export const messages: Record<MessageKey, string> = {
  "toolbar.openProject": "Projekt öffnen…",
  "toolbar.openNativeProject": "Öffnen (.knxdb)…",
  "toolbar.save": "Speichern",
  "toolbar.saveAs": "Speichern unter…",
  "toolbar.exportProject": "Nach .knxproj exportieren…",
  "toolbar.undo": "Rückgängig",
  "toolbar.redo": "Wiederholen",
  "toolbar.search": "Suchen… (Strg+K)",
  "toolbar.log": "Protokoll",
  "toolbar.settings": "Einstellungen",
};
