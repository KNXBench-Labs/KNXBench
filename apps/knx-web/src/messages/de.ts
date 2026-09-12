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
  "toast.holiday.newYearsEve.oneMoreSave": "Noch einmal speichern vor Mitternacht?",
  "toast.holiday.newYearsEve.seeYou": "Bis nächstes Jahr, Projektdatei.",

  "toast.lateNight.midnightOil": "Nachtschicht? Dein KNX-Bus macht auch keine Pause.",
  // "Linie" doubles as "line" in the mundane sense and as the KNX topology
  // term (a Line of up to 255 devices under a Line Coupler) — the same
  // double meaning the English "bus line" trades on. "Buslinie" reads to a
  // German ear as a public-transit bus route, which kills the pun, so we
  // drop the "Bus" prefix and let "Linie" alone carry both meanings.
  "toast.lateNight.busLineRest": "Es ist spät. Auch die Linie braucht mal Pause.",
  "toast.lateNight.stillAwake": "Noch wach? Die Gruppenadressen bewundern deinen Einsatz.",
  "toast.lateNight.nightOwl": "Nachteulen-Modus aktiviert.",

  "inspector.address": "Adresse",
  "inspector.description": "Beschreibung",
  "inspector.dpt": "DPT",
  // The German ETS's own flag names, not a fresh translation of the
  // English ones — "Transmit"/"Update" become "Übertragen"/"Aktualisieren",
  // not a literal "Senden"/"Erneuern" a dictionary would suggest.
  "inspector.comFlag.read": "Lesen",
  "inspector.comFlag.write": "Schreiben",
  "inspector.comFlag.transmit": "Übertragen",
  "inspector.comFlag.update": "Aktualisieren",
  "inspector.comFlag.communication": "Kommunikation",
  "inspector.direction.send": "Senden",
  "inspector.direction.receive": "Empfangen",
  "inspector.unlink": "Trennen",
  "inspector.chooseGroupAddress": "(Gruppenadresse wählen)",
  "inspector.link": "Verknüpfen",
  "inspector.line": "Linie",
  "inspector.unassigned": "(nicht zugeordnet)",
  "inspector.areaLabel": "Bereich {address}: {name}",
  "inspector.lineLabel": "Linie {address}: {name}",
  "inspector.buildingPart": "Gebäudeteil",
  "inspector.none": "(keine)",
  "inspector.delete": "Löschen",
  "inspector.restrictedAction.delete": "Löschen ist",
  "inspector.restrictedAction.renameAndDelete": "Umbenennen und Löschen sind",
  "inspector.restrictedToFirstInstallation":
    "{action} nur für {entity} in der ersten Installation verfügbar.",
  "inspector.entity.devices": "Geräte",
  "inspector.entity.groupAddresses": "Gruppenadressen",
  "inspector.entity.groupRanges": "Gruppenbereiche",
  "inspector.entity.areas": "Bereiche",
  "inspector.entity.lines": "Linien",
  "inspector.entity.buildingParts": "Gebäudeteile",
  "inspector.communicationObjects": "Kommunikationsobjekte",
  "inspector.unnamed": "(unbenannt)",
  "inspector.name": "Name",
  "inspector.lineCount.one": "{count} Linie",
  "inspector.lineCount.other": "{count} Linien",
  "inspector.deviceCount.one": "{count} Gerät",
  "inspector.deviceCount.other": "{count} Geräte",
  "inspector.childPartCount.one": "{count} untergeordneter Gebäudeteil",
  "inspector.childPartCount.other": "{count} untergeordnete Gebäudeteile",

  "buildingPartKind.building": "Gebäude",
  "buildingPartKind.floor": "Etage",
  "buildingPartKind.room": "Raum",
  "buildingPartKind.corridor": "Flur",
  "buildingPartKind.distributionBoard": "Verteiler",
  "buildingPartKind.buildingPart": "Gebäudeteil",

  "parameters.deviceScope": "Gerät",
  "parameters.moduleNumber": "Modul #{number}",
  "parameters.sharedReadOnlyCaption":
    "Wird von jeder Instanziierung dieses Moduls gemeinsam genutzt; in dieser Version nur lesbar.",
  "parameters.staleValuesHeading": "Veraltete Werte ({count})",
  "parameters.staleDescription":
    "Diese gespeicherten Werte entsprechen keinem Parameter des aktuellen Anwendungsprogramms mehr.",
  "parameters.diagnosticsCount.one":
    "{count} Problem beim Auswerten der Parameter dieses Geräts gefunden",
  "parameters.diagnosticsCount.other":
    "{count} Probleme beim Auswerten der Parameter dieses Geräts gefunden",
  "parameters.copyDetails": "Details kopieren",
  "parameters.title": "Parameter",
  "parameters.loading": "Parameter werden geladen…",
  "parameters.noProgram":
    "Für dieses Gerät lässt sich kein Anwendungsprogramm auflösen; Parameter können nicht angezeigt werden.",
  "parameters.none": "(keine)",

  "explorer.addDevice": "+ Gerät hinzufügen",
  "explorer.lineLabel": "Linie {address}: {name}",
  "explorer.areaLabel": "Bereich {address}: {name}",
  "explorer.newLinePlaceholder": "Neue Linie",
  "explorer.newAreaPlaceholder": "Neuer Bereich",
  "explorer.add": "Hinzufügen",
  "explorer.newGroupAddressPlaceholder": "Neue Gruppenadresse",
  "explorer.noRange": "(kein Bereich)",
  "explorer.newGroupRangePlaceholder": "Neuer Gruppenbereich",
  "explorer.newMiddleRangePlaceholder": "Neuer Mittelbereich",
  "explorer.newBuildingPlaceholder": "Neues Gebäude",
  "explorer.newBuildingPartPlaceholder": "Neuer Gebäudeteil",
  "explorer.buildingLabel": "{name} ({kind})",
  "explorer.topology": "Topologie",
  "explorer.buildings": "Gebäude",
  "explorer.unassigned": "Nicht zugeordnet",
  "explorer.groupAddresses": "Gruppenadressen",
  "explorer.groupRanges": "Gruppenbereiche",
  "explorer.importErrorsCount.one": "{count} Importfehler — Daten könnten fehlen oder falsch sein",
  "explorer.importErrorsCount.other": "{count} Importfehler — Daten könnten fehlen oder falsch sein",
  "explorer.importWarningsCount.one": "{count} Importwarnung",
  "explorer.importWarningsCount.other": "{count} Importwarnungen",
};
