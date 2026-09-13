/** German UI message catalogue, typed against MessageKey so a missing key fails compilation. */
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
    "Wird von jeder Instanziierung dieses Moduls gemeinsam genutzt; hier nicht bearbeitbar – siehe die Diagnosen für den Grund.",
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

  "logPanel.title": "Sitzungsprotokoll",
  "logPanel.severity.error": "Fehler",
  "logPanel.severity.warning": "Warnung",
  "logPanel.severity.info": "Info",
  "logPanel.emptyNoEntries": "Noch keine Protokolleinträge.",
  "logPanel.emptyFiltered": "Keine Protokolleinträge entsprechen den aktuellen Filtern.",

  "busMonitor.title": "Bus-Monitor",
  "busMonitor.connect": "Verbinden",
  "busMonitor.disconnect": "Trennen",
  "busMonitor.session": "Sitzung {id}",
  "busMonitor.assignedAddress": " – zugewiesene Adresse {address}",
  "busMonitor.closedByGateway": " – vom Gateway getrennt",
  "busMonitor.stopSummary.one": "Sitzung {id} beendet: {count} Telegramm empfangen, {dropped} verworfen.",
  "busMonitor.stopSummary.other": "Sitzung {id} beendet: {count} Telegramme empfangen, {dropped} verworfen.",
  "busMonitor.gapNotice.one":
    "{count} Telegramm konnte nicht aufbewahrt werden (Puffergröße oder ein zu langsamer Abruf) und fehlt in dieser Ansicht.",
  "busMonitor.gapNotice.other":
    "{count} Telegramme konnten nicht aufbewahrt werden (Puffergröße oder ein zu langsamer Abruf) und fehlen in dieser Ansicht.",
  "busMonitor.filterPlaceholder": "Nach Ziel oder Name filtern…",
  "busMonitor.rowTitle": "Klicken, um das Sendeformular oben mit dem Ziel dieser Zeile vorzubelegen",
  "busMonitor.emptyNoTelegrams": "Noch keine Telegramme.",
  "busMonitor.emptyFiltered": "Keine Telegramme entsprechen den aktuellen Filtern.",
  "busMonitor.column.seq": "Nr.",
  "busMonitor.column.time": "Zeit",
  "busMonitor.column.source": "Quelle",
  "busMonitor.column.destination": "Ziel",
  "busMonitor.column.service": "Dienst",
  "busMonitor.column.payload": "Nutzdaten",
  "busMonitor.column.decoded": "Dekodiert",

  "catalog.title": "Gerätekatalog",
  "catalog.installing": "Produktdatenbank wird installiert…",
  "catalog.installLabel": "Produktdatenbank installieren",
  "catalog.installReport.status.already": "Bereits installiert",
  "catalog.installReport.status.new": "Installiert",
  "catalog.installReport.summary": "{status}: Schema {scheme}, {members}, {unknown}, {conflicts}.",
  "catalog.installReport.membersCount.one": "{count} Mitglied",
  "catalog.installReport.membersCount.other": "{count} Mitglieder",
  "catalog.installReport.unknownCount": "{count} unbekannt",
  "catalog.installReport.conflictsCount.one": "{count} Konflikt",
  "catalog.installReport.conflictsCount.other": "{count} Konflikte",
  "catalog.allManufacturers": "Alle Hersteller",
  "catalog.searchPlaceholder": "Katalogeinträge durchsuchen…",
  "catalog.noMatches": "Keine Treffer.",
  "catalog.deviceNamePlaceholder": "Gerätename",
  "catalog.creating": "Wird erstellt…",
  "catalog.create": "Erstellen",
  "catalog.diagnosticsHeading": "Diagnosen bei der Erstellung",
  "catalog.createdWithDiagnostics": "Gerät mit Diagnosen erstellt.",
  "catalog.done": "Fertig",

  "catalogDiagnostic.programlessProduct":
    "Produkt {catalogItemId} hat kein Anwendungsprogramm; es wurde ohne Kommunikationsobjekte angelegt.",
  "catalogDiagnostic.ambiguousDpt": "Für {refId} konnte kein DPT ermittelt werden; Alternativen: {alternatives}.",
  "catalogDiagnostic.comObjectRefMissing":
    "Der Verweis auf das Kommunikationsobjekt {refId} fehlt im installierten Anwendungsprogramm.",
  "catalogDiagnostic.programRefMissing":
    "Der Verweis auf das installierte Anwendungsprogramm {programRef} fehlt.",
  "catalogDiagnostic.dynamicOrModuleNotEvaluated":
    "Dynamische Aktivierung und Modulaktivierung wurden für {programId} nicht ausgewertet; es wurden nur statische Produktdaten übernommen.",

  "search.overlayLabel": "Suche",
  "search.placeholder": "Geräte, Gruppenadressen, Gebäudeteile durchsuchen…",
  "search.noMatches": "Keine Treffer.",
  "search.kind.device": "Geräte",
  "search.kind.groupAddress": "Gruppenadressen",
  "search.kind.buildingPart": "Gebäudeteile",

  "command.overlayLabel": "Befehlspalette",
  "command.placeholder": "Befehl eingeben…",
  "command.noMatches": "Keine passenden Befehle.",
  "command.shortcutHint.undo": "Strg+Z",
  "command.shortcutHint.redo": "Strg+Umschalt+Z",
  "command.shortcutHint.search": "Strg+K",

  "settings.title": "Einstellungen",
  "settings.theme": "Design",
  "settings.motionStyle": "Bewegungsstil",
  "settings.motionLevel": "Bewegungsstufe",
  "settings.productDataLanguage": "Sprache der Produktdaten",
  "settings.noProductDatabase": "Keine Produktdatenbank installiert",
  "settings.packageDefault": "Paketstandard",
  "settings.productLanguageOption.one": "{language} ({count} Zeichenkette)",
  "settings.productLanguageOption.other": "{language} ({count} Zeichenketten)",

  "languagePack.importLabel": "Sprachpaket importieren…",
  "languagePack.exportTemplateButton": "Englische Vorlage exportieren…",
  "languagePack.exportTemplateHint":
    "Eine übersetzbereite Kopie jeder Zeichenkette, die diese Version kennt, damit ein Sprachpaket ohne Blick in den Quellcode entstehen kann. Das Tag (\"en\") muss vor dem erneuten Import geändert werden — so exportiert wird es von der eingebauten englischen Sprache verdeckt und käme nie zum Einsatz.",
  "languagePack.installedTitle": "Installierte Sprachpakete",
  "languagePack.noPacksInstalled": "Keine Sprachpakete installiert.",
  "languagePack.exportPackButton": "Exportieren…",
  "languagePack.exportPackAriaLabel": "{name} exportieren",
  "languagePack.removePackButton": "Entfernen",
  "languagePack.removePackAriaLabel": "{name} entfernen",
  "languagePack.importReport.invalidJson": "Diese Datei ist kein gültiges JSON.",
  "languagePack.importReport.rejected": "Import abgelehnt: {reason}",
  "languagePack.importReport.grandfatheredHint":
    "Tipp: \"{oldTag}\" ist ein historisches (grandfathered) Tag; die moderne registrierte Form lautet \"{modernTag}\".",
  "languagePack.importReport.heading": "„{name}“ importiert.",
  "languagePack.importReport.appliedKeys.one": "{count} Zeichenkette übersetzt.",
  "languagePack.importReport.appliedKeys.other": "{count} Zeichenketten übersetzt.",
  "languagePack.importReport.missingKeys.one":
    "{count} Zeichenkette nicht übersetzt — fällt auf Englisch zurück.",
  "languagePack.importReport.missingKeys.other":
    "{count} Zeichenketten nicht übersetzt — fallen auf Englisch zurück.",
  "languagePack.importReport.missingKeysSample": "Zum Beispiel: {keys}.",
  "languagePack.importReport.unknownKeys.one":
    "{count} Schlüssel, den diese Version nicht kennt — das Paket zielt vermutlich auf eine andere Version:",
  "languagePack.importReport.unknownKeys.other":
    "{count} Schlüssel, die diese Version nicht kennt — das Paket zielt vermutlich auf eine andere Version:",
  "languagePack.importReport.pluralSupported": "Pluralformen werden für diese Sprache unterstützt.",
  "languagePack.importReport.pluralUnsupported":
    "Für diese Sprache liegen auf diesem System keine Pluraldaten vor; Pluraltexte verwenden immer die allgemeine Form.",
  "languagePack.importReport.shadowedByBuiltIn":
    "Das Tag dieses Pakets (\"{tag}\") entspricht einer eingebauten Sprache und käme nie zum Einsatz — \"tag\" ändern, bevor es aktiviert oder weitergegeben wird.",

  "projectDiff.compareButton": "Vergleichen mit…",
  "projectDiff.compareFilterName": "KNXBench-Projekt",
  "projectDiff.title": "Vergleichsergebnis",
  "projectDiff.noDifferences": "Keine Unterschiede gefunden.",
  "projectDiff.close": "Schließen",
  "projectDiff.projectInfoChanged.one": "Projektinfo: {count} Feld geändert",
  "projectDiff.projectInfoChanged.other": "Projektinfo: {count} Felder geändert",
  "projectDiff.installationInfoChanged.one": "Installationsinfo: {count} Feld geändert",
  "projectDiff.installationInfoChanged.other": "Installationsinfo: {count} Felder geändert",
  "projectDiff.installationPrefix": "Installation {id}: ",
  "projectDiff.installationStatusLine": "Installation {status}",
  "projectDiff.entityStatus.added": "hinzugefügt",
  "projectDiff.entityStatus.removed": "entfernt",
  "projectDiff.entityStatus.changed": "geändert",
  "projectDiff.entityStatus.ambiguous": "mehrdeutig",
  "projectDiff.entity.areas": "Bereiche",
  "projectDiff.entity.lines": "Linien",
  "projectDiff.entity.devices": "Geräte",
  "projectDiff.entity.groupRanges": "Gruppenbereiche",
  "projectDiff.entity.groupAddresses": "Gruppenadressen",
  "projectDiff.entity.buildings": "Gebäude",

  "groupAddressCsv.filterName": "Gruppenadressen-CSV",

  "app.filterName.etsProject": "ETS-Projekt",
  "app.filterName.knxDesktopProject": "knx-desktop-Projekt",

  "documentationExport.filterName": "HTML-Dokument",

  "bulkAction.deviceLabel.one": "{count} Gerät ausgewählt",
  "bulkAction.deviceLabel.other": "{count} Geräte ausgewählt",
  "bulkAction.groupAddressLabel.one": "{count} Gruppenadresse ausgewählt",
  "bulkAction.groupAddressLabel.other": "{count} Gruppenadressen ausgewählt",
  "bulkAction.delete": "Löschen",
  "bulkAction.moveToLine": "Auf Linie verschieben…",
  "bulkAction.unassigned": "(nicht zugewiesen)",
  "bulkAction.moveToBuildingPart": "Auf Gebäudeteil verschieben…",
  "bulkAction.none": "(keiner)",
  "bulkAction.dismissSelection": "Auswahl aufheben",

  "busCompose.heading": "Wert senden",
  "busCompose.noProjectHint":
    "Kein Projekt geöffnet — hier wird kein DPT automatisch aufgelöst; einen explizit eingeben.",
  "busCompose.sessionClosedMessage": "Diese Sitzung ist geschlossen — Senden ist deaktiviert.",
  "busCompose.noDptResolvedMessage": "Für diese Gruppenadresse konnte kein DPT aufgelöst werden — einen explizit eingeben.",
  "busCompose.conflictingDptsMessage":
    "Widersprüchliche DPTs für diese Gruppenadresse: {names} — einen explizit eingeben.",
  "busCompose.destinationLabel": "Ziel",
  "busCompose.dptLabel": "DPT",
  "busCompose.valueLabel": "Wert",
  "busCompose.send": "Senden",
  "busCompose.liveAction": "Sendet auf den verbundenen Bus. Projekt-Rückgängig kann diese Aktion nicht zurücknehmen.",
  "busCompose.sent": "Gesendet {service}: {payload}",

  "fsPicker.open": "Öffnen",
  "fsPicker.saveAs": "Speichern unter",
  "fsPicker.filenamePlaceholder": "Dateiname",
  "fsPicker.upload": "Hochladen…",
  "fsPicker.save": "Speichern",
  "fsPicker.cancel": "Abbrechen",

  "groupAddressCsv.exportButton": "Gruppenadressen exportieren (CSV)…",
  "groupAddressCsv.importButton": "Gruppenadressen importieren (CSV)…",
  "groupAddressCsv.exportSummaryNone": "Gruppenadressen als CSV exportiert, keine Warnungen.",
  "groupAddressCsv.exportSummaryWithWarnings.one":
    "Gruppenadressen als CSV exportiert, {count} Warnung — siehe Log.",
  "groupAddressCsv.exportSummaryWithWarnings.other":
    "Gruppenadressen als CSV exportiert, {count} Warnungen — siehe Log.",
  "groupAddressCsv.importSummaryBase":
    "Gruppenadressen aus CSV importiert: {created} erstellt, {updated} aktualisiert, {unchanged} unverändert",
  "groupAddressCsv.importSummaryWarnings.one": "{count} Warnung",
  "groupAddressCsv.importSummaryWarnings.other": "{count} Warnungen",
  "groupAddressCsv.importSummaryIgnoredColumns.one": "{count} Spalte ignoriert",
  "groupAddressCsv.importSummaryIgnoredColumns.other": "{count} Spalten ignoriert",
  "groupAddressCsv.importSummarySeeLog": "— siehe Log.",
  "appearance.accent": "Akzentfarbe",
  "appearance.density": "Dichte",
  "appearance.violet": "Violett",
  "appearance.mint": "Mint",
  "appearance.blue": "Blau",
  "appearance.amber": "Bernstein",
  "appearance.rose": "Rosa",
  "appearance.compact": "Kompakt",
  "appearance.comfortable": "Komfortabel",
  "workbench.overview": "Übersicht",
  "workbench.buildings": "Gebäude",
  "workbench.topology": "Topologie",
  "workbench.addresses": "Gruppenadressen",
  "workbench.catalog": "Produktkatalog",
  "workbench.device": "Gerät",
  "workbench.unassigned": "Nicht zugeordnet",
  "workbench.emptyStructure": "Noch keine Einträge. Struktur über den Projektbaum anlegen.",
  "workbench.address": "Adresse",
  "workbench.name": "Name",
  "workbench.file": "Datei",
  "workbench.navigation": "Navigation",
  "workbench.properties": "Eigenschaften",
  "workbench.welcome": "Dein KNX-Arbeitsbereich",
  "workbench.openHint": "Projekt öffnen, um Gebäude, Topologie und Gruppenadressen zu bearbeiten.",
  "workbench.noSelection": "Eintrag auswählen, um Eigenschaften zu prüfen oder zu bearbeiten.",
  "workbench.importNotices": "Import: {errors} Fehler · {warnings} Warnungen",
  "workbench.parameters": "Parameter",
  "workbench.telegram": "Telegramm",
  "workbench.selectTelegram": "Telegramm auswählen, um die empfangenen Daten zu prüfen.",
  "workbench.noDevices": "Diesem Gebäudeteil sind keine Geräte zugeordnet.",

  "deviceIdentity.title": "Produktidentität",
  "deviceIdentity.tab": "Produktdaten",
  "deviceIdentity.productRef": "Produktreferenz",
  "deviceIdentity.programRef": "Applikationsprogramm-Referenz",
  "deviceIdentity.refNotStated": "im Projekt nicht angegeben",
  "deviceIdentity.resolution.resolved": "Aus der Produktdatenbank",
  "deviceIdentity.resolution.noDatabase": "Keine Produktdatenbank",
  "deviceIdentity.resolution.notInDatabase": "Nicht in der Produktdatenbank",
  "deviceIdentity.resolution.noReference": "Keine Produktreferenz",
  "deviceIdentity.resolution.unrecognised": "Zustand unbekannt",
  "deviceIdentity.explain.unrecognised":
    "Der Server meldet einen Auflösungszustand, den dieser Build nicht kennt; dieses Feld kann deshalb nicht sagen, ob die Angaben oben einem Produkt zugeordnet wurden. Die Referenzen stehen genau so, wie das Projekt sie angibt.",
  "deviceIdentity.explain.noDatabase":
    "Hier ist keine Produktdatenbank geladen; die Angaben oben lassen sich deshalb keinem Produkt zuordnen. Produktpaket des Herstellers installieren, um Produkt-, Hardware- und Applikationsprogrammdaten zu sehen.",
  "deviceIdentity.explain.notInDatabase":
    "Eine Produktdatenbank ist geladen und enthält die Angaben oben nicht. Der Herstellerkatalog für dieses Produkt ist hier nicht installiert.",
  "deviceIdentity.explain.noReference":
    "Dieses Gerät gibt weder eine Produktreferenz noch eine Applikationsprogramm-Referenz an — es wurde ohne eine solche angelegt oder stammt aus einem Import, der keine mitbrachte.",
  "deviceIdentity.explain.resolvedWithoutCatalog":
    "Die Produktdatenbank meldet einen Treffer, lieferte dazu aber keine Daten.",
  "deviceIdentity.more": "Weitere Produktdaten",
  "deviceIdentity.group.product": "Produkt",
  "deviceIdentity.group.hardware": "Hardware",
  "deviceIdentity.group.application": "Applikationsprogramm",
  "deviceIdentity.manufacturer": "Hersteller",
  "deviceIdentity.manufacturerId": "Hersteller-ID",
  "deviceIdentity.productText": "Produktname",
  "deviceIdentity.orderNumber": "Bestellnummer",
  "deviceIdentity.catalogItemName": "Katalogeintrag",
  "deviceIdentity.catalogItemNumber": "Katalogeintragsnummer",
  "deviceIdentity.hardwareName": "Hardwarename",
  "deviceIdentity.hardwareVersion": "Hardwareversion",
  "deviceIdentity.hardwareSerial": "Hardware-Seriennummer",
  "deviceIdentity.applicationName": "Programmname",
  "deviceIdentity.applicationNumber": "Programmnummer",
  "deviceIdentity.applicationVersion": "Programmversion",
  "deviceIdentity.applicationProgramId": "Programm-ID",
  "deviceIdentity.maskVersion": "Maskenversion",
  "deviceIdentity.groupEmpty": "Die Produktdatenbank enthält hier keine Werte.",
  "deviceIdentity.omitted.one": "{count} weiteres Feld ist ausgeblendet: Die Produktdatenbank hat dafür keinen Wert.",
  "deviceIdentity.omitted.other": "{count} weitere Felder sind ausgeblendet: Die Produktdatenbank hat dafür keine Werte.",
};
