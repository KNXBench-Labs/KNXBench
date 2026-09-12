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
  "toolbar.busMonitor": "Busmonitor",
  "toolbar.commands": "Befehle… (Strg+Umschalt+P)",
  "settings.uiLanguage": "UI-Sprache",
  // A language names itself, not the currently active UI language — this
  // pair is meant to read identically in both catalogues, not a copy-paste
  // slip that skipped translation.
  "language.en": "English",
  "language.de": "Deutsch",

  // Distinct from `toolbar.search`: the toolbar button carries the
  // "(Ctrl+K)"/"(Strg+K)" shortcut suffix, the palette row does not — its
  // `shortcutHint` badge renders that separately.
  "command.search": "Suchen…",

  "dashboard.title": "Projektstatus",
  "dashboard.schemaVersion": "Schemaversion",
  "dashboard.installations": "Installationen",
  "dashboard.areas": "Bereiche",
  "dashboard.lines": "Linien",
  "dashboard.devices": "Geräte",
  "dashboard.devicesUnassigned": " ({count} nicht zugeordnet)",
  "dashboard.groupAddresses": "Gruppenadressen",
  "dashboard.buildingParts": "Gebäudeteile",
  "dashboard.comObjects": "Kommunikationsobjekte",
  "dashboard.importErrors": "Importfehler",
  "dashboard.importWarnings": "Importwarnungen",

  "documentationExport.button": "Dokumentation exportieren…",
  "documentationExport.summaryNone": "Projektdokumentation exportiert, keine Warnungen.",
  "documentationExport.summaryWithWarnings.one":
    "Projektdokumentation exportiert, {count} Warnung — siehe Protokoll.",
  "documentationExport.summaryWithWarnings.other":
    "Projektdokumentation exportiert, {count} Warnungen — siehe Protokoll.",

  "toast.dismiss": "Schließen",

  "toast.error.notAsPlanned": "Nun, das lief nicht wie geplant: {msg}",
  "toast.error.busObjects": "Der Bus legt Widerspruch ein: {msg}",
  "toast.error.gremlins": "Kobolde in der Verkabelung: {msg}",
  "toast.error.knxSaysNo": "KNX sagt nein: {msg}",
  "toast.error.notToday": "Nicht heute: {msg}",
  "toast.error.houston": "Houston, wir haben ein Problem: {msg}",
  "toast.error.hardPass": "Klares Nein: {msg}",

  "toast.holiday.newYear.groupAddresses": "Frohes neues Jahr! Mögen deine Gruppenadressen einzigartig bleiben.",
  "toast.holiday.newYear.sameAddresses": "Neues Jahr, gleiche Gruppenadressen.",
  "toast.holiday.valentine.roses": "Rosen sind rot, KNX-Busleitungen sind verdrillt.",
  "toast.holiday.valentine.favorite": "Sei mein Valentin, mein liebstes Kommunikationsobjekt.",
  "toast.holiday.aprilFools.noBugs": "Heute keine Bugs. Wahrscheinlich.",
  "toast.holiday.aprilFools.real": "Alles in diesem Build ist zu 100 % echt. Ehrenwort.",
  "toast.holiday.halloween.spooky": "Gruselsaison: Selbst die Geister nutzen KNX für die Beleuchtung.",
  "toast.holiday.halloween.boo": "Buh! Dein Projekt ist trotzdem sicher.",
  "toast.holiday.christmasEve.hoho": "Ho, ho, ho – vergiss nicht, dein Projekt zu speichern.",
  // A nod to "Stille Nacht, heilige Nacht", not a literal rendering of the
  // English pun — "wired bright" has no clean German equivalent that keeps
  // both the carol reference and the KNX joke.
  "toast.holiday.christmasEve.silentNight": "Stille Nacht, verkabelte Nacht.",
  "toast.holiday.christmasDay.santa":
    "Frohe Weihnachten! Sogar der Weihnachtsmann braucht eine Gruppenadresse für den Kaminsensor.",
  "toast.holiday.christmasDay.greetings": "Frohe Festtage von deiner KNX-App.",
  "toast.holiday.newYearsEve.oneMoreSave": "Noch ein Speichern vor Mitternacht?",
  "toast.holiday.newYearsEve.seeYou": "Bis nächstes Jahr, Projektdatei.",

  "toast.lateNight.midnightOil": "Nachtschicht? Dein KNX-Bus macht auch keine Pause.",
  // "Linie" doubles as "line" in the mundane sense and as the KNX topology
  // term (a Line of up to 255 devices under a Line Coupler) — the same
  // double meaning the English "bus line" trades on, preserved rather than
  // flattened into an unambiguous but joke-free word.
  "toast.lateNight.busLineRest": "Es ist spät. Sogar die Buslinie braucht mal Ruhe.",
  "toast.lateNight.stillAwake": "Noch wach? Die Gruppenadressen bewundern deinen Einsatz.",
  "toast.lateNight.nightOwl": "Nachteulen-Modus aktiviert.",
};
