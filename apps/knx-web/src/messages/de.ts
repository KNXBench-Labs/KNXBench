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
  "toolbar.newProject": "Neues Projekt…",
  "toolbar.openProject": "Projekt öffnen…",
  "toolbar.openNativeProject": "Öffnen (.knxdb)…",
  "toolbar.save": "Speichern",
  "toolbar.saveAs": "Speichern unter…",
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
  "toast.error.messageIsEnglish": "Diese Meldung ist der unveränderte Text des Servers, auf Englisch.",

  "toast.error.notAsPlanned": "Nun, das lief nicht wie geplant: {msg}",
  "toast.error.busObjects": "Der Bus legt Widerspruch ein: {msg}",
  "toast.error.gremlins": "Kobolde in der Verkabelung: {msg}",
  "toast.error.knxSaysNo": "KNX sagt nein: {msg}",
  "toast.error.notToday": "Nicht heute: {msg}",
  "toast.error.houston": "Houston, wir haben ein Problem: {msg}",
  "toast.error.hardPass": "Klares Nein: {msg}",
  // 23 more error wrappers. Native German lines in Marvin's/the dungeon
  // keeper narrator's register, not word-for-word translations of the
  // English set — "Filed under 'of course'", for instance, becomes
  // "Abgelegt unter 'na klar'", because a literal filing-cabinet metaphor
  // does not land the same dry tone in German.
  "toast.error.marvinSigh": "Marvin würde seufzen und sagen: {msg}",
  "toast.error.uncaringUniverse": "Eine weitere glorreiche Diagnose in einem gleichgültigen Universum: {msg}",
  "toast.error.busSpoken": "Der Bus hat gesprochen, und er ist nicht beeindruckt: {msg}",
  "toast.error.brainSizeOfPlanet": "Ein Gehirn von der Größe eines Planeten, und trotzdem: {msg}",
  "toast.error.dontTalkToMeAboutLife": "Das Leben. Sprich mir nicht vom Leben. Oder von diesem hier: {msg}",
  "toast.error.relayDespair": "Irgendwo hat ein Relais verzweifelt geklickt: {msg}",
  "toast.error.dungeonKeeperNarrates": "Der dungeon keeper verkündet dein Verhängnis: {msg}",
  "toast.error.oldTrick": "Ach, dieser alte Trick schon wieder: {msg}",
  "toast.error.wiringConspires": "Die Verkabelung verschwört sich erneut: {msg}",
  "toast.error.nothingWorks": "Nichts funktioniert, und dennoch geht der Tag weiter: {msg}",
  "toast.error.minorApocalypse": "Eine kleine Apokalypse, ganz nach KNX-Geschmack: {msg}",
  "toast.error.topologySighed": "Die Topologie hat hörbar geseufzt: {msg}",
  "toast.error.hopeNowhere": "Gruppenadressen überall, Hoffnung nirgends: {msg}",
  "toast.error.triumphOfEntropy": "Ein weiterer Triumph der Entropie: {msg}",
  "toast.error.telegramBadNews": "Das Telegramm kam an und brachte schlechte Nachrichten: {msg}",
  "toast.error.dontPanicWorse": "Keine Panik. Es ist schlimmer als das: {msg}",
  "toast.error.busLineComplaint": "Die Linie reicht ihre Beschwerde ein: {msg}",
  "toast.error.loadStateMachineWept": "Irgendwo hat eine Ladezustandsmaschine geweint: {msg}",
  "toast.error.alsoInevitable": "Auch das war unausweichlich: {msg}",
  "toast.error.gremlinsRegards": "Die Kobolde lassen grüßen: {msg}",
  "toast.error.mediocrityInErrorForm": "Seht her, Mittelmäßigkeit in Fehlerform: {msg}",
  "toast.error.universeIndifferent": "Das Universum bleibt zutiefst gleichgültig: {msg}",
  "toast.error.filedUnderOfCourse": 'Abgelegt unter "na klar": {msg}',

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
  // 23 more holidays, native German wording per occasion, same pairing
  // convention as the original seven above.
  "toast.holiday.epiphany.starlight":
    "Die Heiligen Drei Könige fanden ihren Weg im Sternenlicht. Deinen Gruppenadressen würde ein ähnliches Wunder nicht schaden.",
  "toast.holiday.epiphany.noneArrived": "Heilige Drei Könige, angeblich. Zur Verkabelung ist keiner erschienen.",
  "toast.holiday.piDay.neverResolves":
    "Pi-Tag: eine unendliche, nicht periodische Erinnerung daran, dass sich manche Dinge nie ordentlich auflösen lassen — ganz wie deine offenen Bugs.",
  "toast.holiday.piDay.percentSolved": "3,14 Prozent deiner Probleme sind heute gelöst. Der Rest macht weiter wie gewohnt.",
  "toast.holiday.backupDay.reminder":
    "Weltbackup-Tag. Eine sanfte, leicht bedrohliche Erinnerung, dein Projekt zu speichern.",
  "toast.holiday.backupDay.hardWay":
    "Irgendwo lernt gerade jemand die harte Lektion über Backups. Hoffentlich nicht du.",
  "toast.holiday.earthDay.lightsOff":
    "Tag der Erde. KNX gibt es unter anderem, damit Lichter ausgehen, wenn keiner hinschaut. Gern geschehen, Planet.",
  "toast.holiday.earthDay.energyBill":
    "Einmal im Jahr bekommt der Planet einen Toast. Jeden Tag bekommt deine Stromrechnung eine Gruppenadresse.",
  "toast.holiday.germanBeerDay.reinheitsgebot":
    "Tag des Deutschen Bieres. Das Reinheitsgebot regelte Bier ab 1516; deine Gruppenadressbenennung regelt bis heute niemand.",
  "toast.holiday.germanBeerDay.rulesNeeded":
    "1516 entschied Bayern, dass Bier Regeln braucht. Dein Projekt könnte auch ein paar vertragen.",
  "toast.holiday.tagDerArbeit.busNoDayOff":
    "Tag der Arbeit. Der Bus hat, wie man anmerken muss, keinen freien Tag.",
  "toast.holiday.tagDerArbeit.lineOnDuty":
    "Ein Feiertag für Arbeitende überall. Die KNX-Linie bleibt, wie immer, im Dienst.",
  "toast.holiday.starWarsDay.fourthBeWithYou":
    "Am vierten Mai sei die Macht mit dir. Der Bus bleibt, weniger mystisch, verdrilltes Adernpaar.",
  "toast.holiday.starWarsDay.sithLord": "Irgendwo debuggt auch ein Sith-Lord eine Topologie. Solidarität.",
  "toast.holiday.telecomDay.telegramsRegards":
    "Weltfernmeldetag. Die KNX-Telegramme lassen grüßen, ungelesen wie immer.",
  "toast.holiday.telecomDay.busUnimpressed":
    "Ein ganzer Tag zu Ehren der Telekommunikation. Der Bus bleibt charakteristisch unbeeindruckt.",
  "toast.holiday.towelDay.bringOne":
    "Handtuchtag. Nimm eins mit. Es repariert die Verkabelung nicht, hebt aber die Stimmung.",
  "toast.holiday.towelDay.mostlyBroken": "Keine Panik. Dein Projekt ist nur größtenteils kaputt, nicht vollständig.",
  "toast.holiday.environmentDay.savingPlanet":
    "Weltumwelttag. Irgendwo rettet gerade eine gut konfigurierte KNX-Anlage still und leise den Planeten.",
  "toast.holiday.environmentDay.rarelySaysSo":
    "Die Umwelt dankt dir für die Automatisierung. Nur sagt sie das selten direkt.",
  "toast.holiday.summerSolstice.longestDay":
    "Sommersonnenwende. Ungefähr der längste Tag, kalendarische Rundungsfehler eingeschlossen, für maximale Konfrontation mit ungelösten Diagnosen.",
  "toast.holiday.summerSolstice.todoList":
    "Die Sonne geht heute kaum unter. Deine To-do-Liste offenbar auch nicht.",
  "toast.holiday.moonLanding.lessComputingPower":
    "An diesem Tag landete die Menschheit auf dem Mond, mit weniger Rechenleistung als ein durchschnittliches KNX-Gateway. Zur Einordnung.",
  "toast.holiday.moonLanding.houstonSmaller":
    "Auch Houston hatte einmal ein Problem. Deins ist kleiner und bleibt auf der Erde.",
  "toast.holiday.swissNationalDay.chalet":
    "Schweizer Bundesfeiertag. Irgendwo feiert ein äußerst präzise verkabeltes Chalet pünktlich auf die Minute.",
  "toast.holiday.swissNationalDay.punctuality":
    "Der Schweizer Nationalfeiertag. Die Pünktlichkeit deines Projekts ist davon völlig unberührt.",
  "toast.holiday.programmerDay.day256":
    "Tag des Programmierers, ungefähr der 256. Tag des Jahres, Schaltjahre schieben das etwas. Eine Zahl, gewählt von Programmierern für Programmierer, verstanden von niemand sonst.",
  "toast.holiday.programmerDay.countedOwnDays":
    "Ein Feiertag, der existiert, weil Programmierer ihre eigenen Tage gezählt haben. Passt.",
  "toast.holiday.pirateDay.plunderedByNobody":
    "Arrr. Deine Gruppenadressen wurden von niemandem geplündert, was ja eigentlich der Sinn der Sache ist.",
  "toast.holiday.pirateDay.unpiratical":
    "Tag des Piratensprechens. Die Bus-Telegramme bleiben standhaft unpiratisch.",
  "toast.holiday.germanUnity.twoNetworks":
    "Tag der Deutschen Einheit. 1990 wurden aus zwei Netzen eins; deins war vermutlich schon immer so.",
  "toast.holiday.germanUnity.ownAffair":
    "Ein Tag zur Feier der Einheit. Deine Gruppenadressbereiche bleiben stur ihre eigene Angelegenheit.",
  "toast.holiday.backToTheFuture.noFlyingCars":
    "Die Zukunft enthält, anders als vorhergesagt, keine fliegenden Autos. Sie enthält KNX. Kleine Erfolge.",
  "toast.holiday.backToTheFuture.noHoverboards":
    "Zurück-in-die-Zukunft-Tag. Kein Hoverboard ist angekommen. Der Bus zumindest ist pünktlich.",
  "toast.holiday.allSaintsDay.devicesRemembered":
    "Allerheiligen. Ein stiller Tag im Gedenken an jedes Gerät, das die Inbetriebnahme nicht überlebt hat.",
  "toast.holiday.allSaintsDay.addressesBefore":
    "Allerheiligen. Ein Gedanke an die Gruppenadressen, die es vor diesem Projekt gab.",
  "toast.holiday.elevenEleven.karnevalBegins":
    "Elfter im Elften. Die Karnevalssaison beginnt; das Chaos deines Projekts hat, nebenbei bemerkt, nie Pause gemacht.",
  "toast.holiday.elevenEleven.foolsOfficially":
    "Heute übernehmen offiziell die Narren. Der Bus war ihnen inoffiziell schon die ganze Zeit voraus.",
  "toast.holiday.computerSecurityDay.knxSecureExists":
    "Tag der Computersicherheit. Ein guter Anlass, sich daran zu erinnern, dass es KNX Secure gibt, auch wenn dieser Build es noch nicht anfasst.",
  "toast.holiday.computerSecurityDay.trustsEveryone":
    "Ein Tag für Computersicherheit. Der Bus vertraut nach wie vor völlig jedem, der auf ihm sitzt.",
  "toast.holiday.nikolaustag.properCommissioning":
    "Nikolaustag. Brave Geräte werden ordentlich in Betrieb genommen; der Rest bekommt statt Kohle eine strenge Diagnosemeldung.",
  "toast.holiday.nikolaustag.warningsOnList":
    "Der Nikolaus prüft seine Liste. Deine ungelösten Warnungen stehen, leider, immer noch drauf.",
  "toast.holiday.winterSolstice.longestNight":
    "Wintersonnenwende, ungefähr die längste Nacht, kalendarische Rundung wie üblich. Reichlich Zeit für die Beleuchtungs-Gruppenadressen, sich zu bewähren.",
  "toast.holiday.winterSolstice.backlogSameLength":
    "Der kürzeste Tag des Jahres. Der Rückstand bleibt trotzdem exakt gleich lang.",
  "toast.holiday.boxingDay.importWarningsUnopened":
    "Zweiter Weihnachtstag. Die Geschenke sind ausgepackt; die Importwarnungen, weniger aufregend, immer noch nicht.",
  "toast.holiday.boxingDay.readingManualLate":
    "Der zweite Weihnachtstag. Irgendwo liest gerade endlich jemand das Handbuch. Reichlich spät dafür.",

  "toast.lateNight.midnightOil": "Nachtschicht? Dein KNX-Bus macht auch keine Pause.",
  // "Linie" doubles as "line" in the mundane sense and as the KNX topology
  // term (a Line of up to 255 devices under a Line Coupler) — the same
  // double meaning the English "bus line" trades on. "Buslinie" reads to a
  // German ear as a public-transit bus route, which kills the pun, so we
  // drop the "Bus" prefix and let "Linie" alone carry both meanings.
  "toast.lateNight.busLineRest": "Es ist spät. Auch die Linie braucht mal Pause.",
  "toast.lateNight.stillAwake": "Noch wach? Die Gruppenadressen bewundern deinen Einsatz.",
  "toast.lateNight.nightOwl": "Nachteulen-Modus aktiviert.",
  // 26 more late-night lines, native German wording.
  "toast.lateNight.busQuiet": "Der Bus ist ruhig. Du offenbar nicht.",
  "toast.lateNight.relayAwake": "Irgendwo ist auch ein Relais wach und ebenso unbeeindruckt.",
  "toast.lateNight.hourOfRegret": "Das ist die Stunde für Reue und Konfigurationsdateien.",
  "toast.lateNight.buildingSleeps": "Das Gebäude schläft. Du nicht. Interessante Entscheidungen.",
  "toast.lateNight.addressesInBed": "Sogar die Gruppenadressen sind schlafen gegangen.",
  "toast.lateNight.insomniaAndKnx": "Schlaflosigkeit und KNX: eine altehrwürdige Paarung.",
  "toast.lateNight.darkHoursSuitDebugging": "Die dunklen Stunden eignen sich fürs Debuggen. Angeblich.",
  "toast.lateNight.reasonableHourElsewhere": "Irgendwo ist es eine vernünftige Uhrzeit. Nicht hier.",
  "toast.lateNight.topologyHoldsBreath": "Die Topologie hält den Atem an bis zum Morgen.",
  "toast.lateNight.anotherCommit": "Noch ein Commit, noch ein Sonnenaufgang vermieden.",
  "toast.lateNight.wiringDiagramsDontJudge": "Die Schaltpläne urteilen nicht. Wahrscheinlich.",
  "toast.lateNight.stillHere": "Immer noch da. Der Bus technisch gesehen auch.",
  "toast.lateNight.linesAreSilent": "Die Linien schweigen. Du bist die Ausnahme.",
  "toast.lateNight.thisIsFine": "Alles bestens. Wirklich alles zu dieser Uhrzeit ist bestens.",
  "toast.lateNight.gatewayBlinks": "Das Gateway blinkt geduldig in die Dunkelheit.",
  "toast.lateNight.sleepIsForFewerBugs": "Schlaf ist für Installationen mit weniger Fehlern.",
  "toast.lateNight.busMonitorLogsSoul":
    "Der Busmonitor protokolliert eine weitere Seele, die eigentlich schlafen sollte.",
  "toast.lateNight.telegramDedicationOrDespair":
    "Ein Telegramm zu dieser Stunde bedeutet Hingabe oder Verzweiflung. Möglicherweise beides.",
  "toast.lateNight.lightsAreOff": "Die Lichter sind aus. Das Urteilsvermögen des Universums wirkt davon auch nicht heller.",
  "toast.lateNight.loadStateMachineClockedOut": "Sogar die Ladezustandsmaschine hat Feierabend gemacht.",
  "toast.lateNight.hourBelongsToOwls": "Diese Stunde gehört den Eulen und den ungelösten Diagnosen.",
  "toast.lateNight.projectWaits": "Das Projekt wartet. Es hat sonst nichts vor.",
  "toast.lateNight.deviceRebootsUnaware": "Irgendwo startet gerade ein Gerät neu, das die Uhrzeit nicht kennt.",
  "toast.lateNight.darkIsVast": "Die Dunkelheit ist grenzenlos, und ausnahmsweise ist deine Uptime es auch.",
  "toast.lateNight.groupRangesNoOfficeHours": "Gruppenbereiche kennen keine Bürozeiten. Du offenbar auch nicht.",
  "toast.lateNight.busLineClocksOut": "Die Linie macht Feierabend. Du offenbar nicht.",

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
  "inspector.comFlag.readOnInit": "Lesen bei Initialisierung",
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
  "explorer.project": "Projekt",
  "inspector.project": "Projekt",
  "inspector.groupAddressStyle": "Gruppenadress-Stil",

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

  "parameters.diagnostic.parametersUnreadable":
    "Einige deklarierte Parameter konnten nicht aus der Produktdatenbank gelesen werden und werden nicht angezeigt.",
  "parameters.diagnostic.duplicateUnscopedValue":
    "Zwei gespeicherte Werte beziehen sich auf denselben Parameter; der spätere wird ignoriert.",
  "parameters.diagnostic.duplicateModuleScopedValue":
    "Zwei gespeicherte Werte beziehen sich auf denselben modulgebundenen Parameter; der spätere wird ignoriert.",
  "parameters.diagnostic.duplicateModuleId":
    "Zwei oder mehr Abschnitte dieses Programms deklarieren dieselbe Modul-ID; die Felder dieses Moduls sind schreibgeschützt.",
  "parameters.diagnostic.noModuleInstanceMatch":
    "Keine importierte Modulinstanz passt zu diesem Modul; seine Felder sind schreibgeschützt.",
  "parameters.diagnostic.ambiguousModuleInstance":
    "Zwei oder mehr importierte Modulinstanzen teilen sich dieses Modul; seine Felder sind schreibgeschützt.",
  "parameters.diagnostic.malformedModuleInstanceId":
    "Die Kennung einer importierten Modulinstanz hat eine unerwartete Form; die Felder dieses Moduls sind schreibgeschützt.",
  "parameters.diagnostic.noBranchMatched": "Eine Auswahl passte auf keine ihrer Optionen.",
  "parameters.diagnostic.unparsableTest": "Die Bedingung einer Auswahl konnte nicht verstanden werden.",
  "parameters.diagnostic.unresolvedParamRef":
    "Der steuernde Parameter einer Auswahl konnte nicht gefunden werden.",
  "parameters.diagnostic.nonNumericValue": "Der steuernde Wert einer Auswahl war keine gültige Zahl.",
  "parameters.diagnostic.unexpectedTypeNoneShape": "Eine ungewöhnliche Auswahlstruktur wurde übersprungen.",
  "parameters.diagnostic.unrecognizedNode": "Ein nicht erkanntes Programmelement wurde übersprungen.",
  "parameters.diagnostic.moduleDefNotFound": "Ein Modul konnte in diesem Programm nicht gefunden werden.",
  "parameters.diagnostic.moduleCycleDetected":
    "Ein Modul verweist zurück auf eines seiner eigenen umschließenden Module und wurde nicht expandiert.",
  "parameters.diagnostic.moduleNestingTooDeep":
    "Ein Modul ist tiefer verschachtelt, als dieses Programm expandiert.",
  "parameters.diagnostic.moduleExpansionBudgetExhausted":
    "Die Module dieses Programms sind zu zahlreich, um vollständig expandiert zu werden; der Rest wurde übersprungen.",
  "parameters.diagnostic.missingValue": "Der steuernde Parameter einer Auswahl hat keinen Wert.",
  "parameters.diagnostic.moduleWithoutId":
    "Eine Modulinstanz hat keine Kennung und kann keinen gespeicherten Werten zugeordnet werden.",
  "parameters.diagnostic.moduleArgumentNotBound":
    "Ein Modulargument konnte nicht der Deklaration des Moduls zugeordnet werden und wurde ignoriert.",
  "parameters.diagnostic.unsupportedModuleArgumentKind":
    "Ein Modulargument verwendet eine Art, die diese Version nicht interpretiert, und wurde ignoriert.",
  "parameters.diagnostic.unresolvedTextPlaceholder":
    "Ein Textplatzhalter hatte kein passendes Modulargument und wurde unverändert belassen.",

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
  "logPanel.eyebrow": "Diagnose",
  "logPanel.severity.error": "Fehler",
  "logPanel.severity.warning": "Warnung",
  "logPanel.severity.info": "Info",
  "logPanel.emptyNoEntries": "Noch keine Protokolleinträge.",
  "logPanel.emptyFiltered": "Keine Protokolleinträge entsprechen den aktuellen Filtern.",
  "logPanel.entryTextIsEnglish":
    "Meldung, Ort und Detail sind der unveränderte Text des Servers, auf Englisch.",

  "busMonitor.title": "Bus-Monitor",
  "busMonitor.eyebrow": "KNXnet/IP · Tunneling",
  "busMonitor.gatewayLabel": "Gateway-Adresse",
  "busMonitor.gatewayLocked":
    "Trennen Sie die laufende Sitzung, bevor Sie die Gateway-Adresse ändern.",
  "busMonitor.connectNeedsGateway": "Geben Sie zuerst eine Gateway-Adresse ein.",
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
  "busMonitor.contextStale":
    "Das Projekt hat sich nach dem Start dieser Sitzung geändert. Die Dekodierung unten stammt aus dem Stand vom Verbindungsaufbau, Senden ist gesperrt. Für die Dekodierung gegen das aktuelle Projekt neu verbinden.",
  "busMonitor.contextUnverified":
    "Dieses Fenster hat die Sitzung nicht gestartet und kann daher nicht bestätigen, dass die Dekodierung zum aktuell geöffneten Projekt passt.",
  "busMonitor.sessionReplaced":
    "Die Bus-Sitzung wurde ersetzt — angezeigt wird jetzt Sitzung {id}. Zeilen der vorherigen Sitzung wurden entfernt.",
  "busMonitor.endedElsewhere": "Die Bus-Sitzung wurde an anderer Stelle beendet.",

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
  "catalog.installReport.unverifiedSignature.one":
    "{count} Signatur-Element gespeichert, nicht geprüft — diese Anwendung kann es nicht prüfen.",
  "catalog.installReport.unverifiedSignature.other":
    "{count} Signatur-Elemente gespeichert, nicht geprüft — diese Anwendung kann sie nicht prüfen.",
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

  "languagePack.rejection.notObject": "Ein Sprachpaket muss ein JSON-Objekt sein.",
  "languagePack.rejection.formatVersionMissing": "\"formatVersion\" ist erforderlich und muss eine Zahl sein.",
  "languagePack.rejection.tagMissing": "\"tag\" ist erforderlich und muss eine nicht leere Zeichenkette sein.",
  "languagePack.rejection.tagMalformed":
    "\"tag\" (\"{tag}\") ist kein wohlgeformtes BCP-47-Tag, z. B. \"nl-NL\", \"tlh\" (Klingonisch), \"bar\" (Bairisch) oder \"art-x-sindarin\" (ein privates Tag für alles nicht Registrierte).",
  "languagePack.rejection.nameMissing": "\"name\" ist erforderlich und muss eine nicht leere Zeichenkette sein.",
  "languagePack.rejection.messagesMissing":
    "\"messages\" ist erforderlich und muss ein Objekt sein, das Schlüssel auf Zeichenketten abbildet.",
  "languagePack.rejection.messageValueNotString":
    "\"messages.{key}\" muss eine Zeichenkette sein, ist aber {valueType}.",
  "languagePack.rejection.englishNameNotString": "\"englishName\" muss, falls vorhanden, eine Zeichenkette sein.",
  "languagePack.rejection.basedOnNotString": "\"basedOn\" muss, falls vorhanden, eine Zeichenkette sein.",
  "languagePack.rejection.packVersionNotString": "\"packVersion\" muss, falls vorhanden, eine Zeichenkette sein.",
  "languagePack.rejection.pluralCategoriesInvalid":
    "\"pluralCategories\" muss, falls vorhanden, ein Array aus Zeichenketten sein.",
  "languagePack.rejection.storageFailure":
    "Die Änderung konnte nicht gespeichert werden: der Speicher des Browsers hat den Schreibvorgang abgelehnt ({detail}).",

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
  "busCompose.sentDecoded": "Dekodiert: {text}",
  "busCompose.contextStaleMessage":
    "Das Projekt hat sich nach dem Start dieser Bus-Sitzung geändert — der DPT würde gegen den alten Stand aufgelöst, deshalb ist Senden gesperrt. Zuerst neu verbinden.",

  // `DiagnosticsCompanion.tsx` und der Knopf in `App.tsx`, der es öffnet.
  "companion.open": "Diagnosefenster",
  "companion.title": "Diagnose",
  "companion.eyebrow": "Begleitfenster · nur lesend",
  "companion.readOnly":
    "Nur lesendes Begleitfenster. Das Projekt wird ausschließlich im Hauptfenster bearbeitet; hier gibt es kein Rückgängig, und dieses Fenster teilt sich die Bus-Sitzung des Hauptfensters, statt eine eigene zu öffnen.",
  "companion.backToMain": "Zurück zum Hauptfenster",
  "companion.noMainWindow": "Kein Hauptfenster vorhanden — dieses Fenster wurde eigenständig geöffnet.",
  "companion.blocked":
    "Das Diagnosefenster wurde blockiert. Pop-ups für diese Seite erlauben — oder den Monitor weiter hier benutzen.",
  "companion.failed":
    "Das Diagnosefenster konnte nicht geöffnet werden. Monitor und Protokoll bleiben hier verfügbar.",

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
  "appearance.accentUnavailable": "Dieses Theme behält seinen eigenen Akzent; die Akzenteinstellung hat hier keine Wirkung.",
  "appearance.density": "Dichte",
  "appearance.violet": "Violett",
  "appearance.mint": "Mint",
  "appearance.blue": "Blau",
  "appearance.amber": "Bernstein",
  "appearance.rose": "Rosa",
  "appearance.groupAddressNotation": "Gruppenadress-Schreibweise",
  "appearance.gaNotation.slash": "Schrägstriche — 1/2/3",
  "appearance.gaNotation.dot": "Punkte — 1.2.3",
  "appearance.groupAddressNotationHint":
    "Gilt für jede im Programm angezeigte Gruppenadresse. Physikalische Adressen behalten ihre Punkte, und Projektdateien, Exporte und Bustelegramme verwenden immer Schrägstriche. Eingeben können Sie beide Schreibweisen.",
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

  // Die Gruppenadresstabelle (Etappe 4). Eine Gruppenadresse hat im
  // KNX-Modell keinen eigenen DPT — diese Beschriftungen benennen, was die
  // verknüpften Kommunikationsobjekte angeben. Deshalb "nicht angegeben"
  // statt "unbekannt", und deshalb wird eine Abweichung als Konflikt
  // benannt statt stillschweigend aufgelöst.
  "addressTable.filterLabel": "Gruppenadressen filtern",
  "addressTable.filterPlaceholder": "Adressen filtern…",
  "addressTable.selectColumn": "Auswählen",
  "addressTable.range": "Bereich",
  "addressTable.dpt": "DPT",
  "addressTable.links": "Verknüpfungen",
  "addressTable.noDpt": "nicht angegeben",
  "addressTable.dptConflict": "widersprüchlich",
  "addressTable.noRange": "(kein Bereich)",
  "addressTable.noLinks": "keine",
  "addressTable.linkCounts": "{senders} Senden · {receivers} Empfangen",
  "addressTable.linkTotal.one": "{count} Verknüpfung",
  "addressTable.linkTotal.other": "{count} Verknüpfungen",
  "addressTable.noMatches": "Keine Gruppenadresse passt zu diesem Filter.",
  "addressTable.linksFor": "Verknüpfungen · {address}",
  "addressTable.noLinksYet": "Mit dieser Adresse ist kein Kommunikationsobjekt verknüpft.",
  "addressTable.participant": "Teilnehmer",
  "addressTable.function": "Funktion",
  "addressTable.direction": "Richtung",
  "addressTable.unlinkFrom": "{object} von {address} trennen",
  // Nur erreichbar, wenn das Projekt ein Objekt verknüpft, dessen Gerät
  // fehlt — die Verknüpfung wird trotzdem gezeigt, nicht verworfen (siehe
  // `GroupAddressLinkNode`).
  "addressTable.unknownDevice": "Unbekanntes Gerät #{id}",
  "addressTable.unnamedObject": "Unbenanntes Objekt",

  "deviceIdentity.title": "Produktidentität",
  "deviceIdentity.tab": "Produktdaten",
  "deviceIdentity.productRef": "Produktreferenz",
  "deviceIdentity.programRef": "Applikationsprogramm-Referenz",
  "deviceIdentity.refNotStated": "im Projekt nicht angegeben",
  "deviceIdentity.resolution.resolved": "Aus der Produktdatenbank",
  "deviceIdentity.resolution.noDatabase": "Keine Produktdatenbank",
  "deviceIdentity.resolution.notInDatabase": "Nicht in der Produktdatenbank",
  "deviceIdentity.resolution.noReference": "Keine Produktreferenz",
  // Bewusst nicht "Zustand unbekannt": die englische Fassung vermeidet das
  // Wort "unknown", weil der Zustand dem Server sehr wohl bekannt ist —
  // nur dieser Build kennt ihn nicht. `DeviceWorkspace.test.tsx` prüft
  // beide Kataloge darauf, damit die Formulierung nicht zurückwandert.
  "deviceIdentity.resolution.unrecognised": "Zustand nicht erkannt",
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

  // Siehe en.ts: der Projektassistent für Projekte, die nie aus einer
  // Datei kamen.
  "newProject.title": "Neues Projekt",
  "newProject.intro": "Ein leeres Projekt mit einer Anlage. Bis zum Speichern wird nichts auf die Festplatte geschrieben.",
  "newProject.name": "Projektname",
  "newProject.defaultName": "Unbenanntes Projekt",
  "newProject.installation": "Anlagenname",
  "newProject.defaultInstallation": "Anlage 1",
  "newProject.language": "Projektsprache",
  "newProject.languageHint": "Das Sprachkennzeichen, unter dem die Projekttexte gespeichert werden, z. B. de oder de-DE. Nicht die Sprache dieser Oberfläche.",
  "newProject.style": "Gruppenadressstil",
  "newProject.styleHint": "Wähle den Stil, in dem du denkst; die Projekteigenschaften können ihn später umstellen.",
  "newProject.style.Free": "Frei (0–65535)",
  "newProject.style.TwoLevel": "Zweistufig (Haupt/Unter)",
  "newProject.style.ThreeLevel": "Dreistufig (Haupt/Mittel/Unter)",
  "newProject.nameRequired": "Ein Projekt braucht einen Namen.",
  "newProject.languageInvalid": "Kein wohlgeformtes Sprachkennzeichen. Versuchen Sie en, de oder de-DE.",
  "newProject.create": "Projekt anlegen",
  "newProject.creating": "Wird angelegt…",
  "newProject.cancel": "Abbrechen",
  "newProject.conflictTitle": "Das geöffnete Projekt hat ungespeicherte Änderungen",
  "newProject.conflictBody": "Ein neues Projekt verwirft diese Änderungen, und kein Rückgängig holt sie zurück. Weiter bearbeiten, um sie zuerst zu speichern, oder bewusst verwerfen.",
  "newProject.conflictDiscard": "Änderungen verwerfen und anlegen",
  "newProject.conflictKeep": "Weiter bearbeiten",

  // Siehe `en.ts`: jede Phase benennt einen Schritt, den die Pipeline
  // wirklich ausführt.
  "loadProgress.importing": "{source} wird importiert…",
  "loadProgress.opening": "{source} wird geöffnet…",
  "loadProgress.failed": "{source} konnte nicht geladen werden",
  "loadProgress.failedDuring": "Fehlgeschlagen bei: {phase}",
  "loadProgress.barLabel": "Ladefortschritt",
  "loadProgress.counted": "{completed} von {total}",
  "loadProgress.phase.starting": "Wird gestartet…",
  "loadProgress.phase.openContainer": "Archiv wird geöffnet",
  "loadProgress.phase.detectSchema": "Schemaversion wird erkannt",
  "loadProgress.phase.parseTopology": "Topologie wird gelesen",
  "loadProgress.phase.parseProjectInfo": "Projektinformationen werden gelesen",
  "loadProgress.phase.validate": "Referenzen werden geprüft",
  "loadProgress.phase.map": "Projektmodell wird aufgebaut",
  "loadProgress.phase.inferDatapointTypes": "Datenpunkttypen werden abgeleitet",
  "loadProgress.phase.collectContainerEntries": "Übrige Archiveinträge werden gelesen",
  "loadProgress.phase.ingestManufacturerData": "Herstellerdaten werden eingelesen",
  "loadProgress.phase.ingestMasterData": "KNX-Stammdaten werden eingelesen",
  "loadProgress.phase.enrichFromProductDatabase": "Aus der Produktdatenbank wird ergänzt",
  "loadProgress.phase.persistOpaque": "Durchgereichte Daten werden gespeichert",
  "loadProgress.phase.openStore": "Projektdatei wird geöffnet",
  "loadProgress.phase.loadStoredProject": "Gespeichertes Projekt wird gelesen",
  "loadProgress.phase.loadOpaque": "Durchgereichte Daten werden gelesen",
  "loadProgress.phase.loadManufacturerRefs": "Herstellerreferenzen werden gelesen",
  "loadProgress.phase.buildProjectTree": "Projektbaum wird aufgebaut",

  // Die Sprüche des Ladebanners (Task 26). Mehrere Zeilen sind bewusst
  // nicht wörtlich übersetzt — 07, 14, 23, 31, 45 und 46 fallen sonst im
  // Deutschen flach. Bei 18 bleibt `230 V` exakt stehen, weil die Pointe
  // das Runden ist und nicht die Zahl.
  "loadProgress.flavour.01": "Sortiere Gruppenadressen nach Farbe",
  "loadProgress.flavour.02": "Frage die Geräte nach ihrem Befinden",
  "loadProgress.flavour.03": "Prüfe die Sternenkonstellation",
  "loadProgress.flavour.04": "Zähle die Bustelegramme zweimal",
  "loadProgress.flavour.05": "Verhandle mit dem Linienkoppler",
  "loadProgress.flavour.06": "Poliere die physikalischen Adressen",
  "loadProgress.flavour.07": "Überzeuge einen Aktor davon, ein Sensor zu sein",
  "loadProgress.flavour.08": "Entwirre die Topologie",
  "loadProgress.flavour.09": "Wecke die Präsenzmelder behutsam",
  "loadProgress.flavour.10": "Schätze die Länge der Buslinie mit bloßem Auge",
  "loadProgress.flavour.11": "Suche den verschollenen Datenpunkttyp",
  "loadProgress.flavour.12": "Übersetze ETS in etwas Lesbares",
  "loadProgress.flavour.13": "Bringe die Hersteller in alphabetische Ordnung",
  "loadProgress.flavour.14": "Bitte den Dimmer, die Erwartungen zu dämpfen",
  "loadProgress.flavour.15": "Prüfe, ob 1.1.1 zu Hause ist",
  "loadProgress.flavour.16": "Wärme die verdrillte Doppelader auf",
  "loadProgress.flavour.17": "Lese das Datenblatt, das niemand liest",
  "loadProgress.flavour.18": "Runde 230 V auf etwas Sichereres ab",
  "loadProgress.flavour.19": "Erkläre den Kommunikationsobjekten ihre Flags",
  "loadProgress.flavour.20": "Sortiere die Räume nach Gemütlichkeit",
  "loadProgress.flavour.21": "Warte auf ein Quittungstelegramm",
  "loadProgress.flavour.22": "Frage den Rollladenaktor, wo oben ist",
  "loadProgress.flavour.23": "Ergründe, was der dritte Taster wirklich tut",
  "loadProgress.flavour.24": "Füttere die Parameter",
  "loadProgress.flavour.25": "Prüfe die Prüfsummen, dann noch einmal",
  "loadProgress.flavour.26": "Konsultiere den KNX-Standard, Band für Band",
  "loadProgress.flavour.27": "Schlage eine Fußnote von 2003 nach",
  "loadProgress.flavour.28": "Entschuldige mich bei der Assoziationstabelle",
  "loadProgress.flavour.29": "Suche das Gerät, das nie eingebaut wurde",
  "loadProgress.flavour.30": "Richte die Etagen an der Schwerkraft aus",
  "loadProgress.flavour.31": "Entwickle eine Meinung zu Namenskonventionen",
  "loadProgress.flavour.32": "Spule das Telegramm zurück",
  "loadProgress.flavour.33": "Lasse die Wetterstation raten",
  "loadProgress.flavour.34": "Zähle die Gruppenadressen, die niemand benutzt",
  "loadProgress.flavour.35": "Prüfe, ob das jemand dokumentiert hat",
  "loadProgress.flavour.36": "Bringe dem Binäreingang das Ja bei",
  "loadProgress.flavour.37": "Richte die Hutschiene gerade",
  "loadProgress.flavour.38": "Schaue hinter den Verteilerkasten",
  "loadProgress.flavour.39": "Gleiche zwei Schreibweisen desselben Raums ab",
  "loadProgress.flavour.40": "Messe die Verdrillung nach",
  "loadProgress.flavour.41": "Bitte den Thermostat um Vernunft",
  "loadProgress.flavour.42": "Entscheide, ob der Keller ein Geschoss ist",
  "loadProgress.flavour.43": "Rette einen Kommentar von 2011",
  "loadProgress.flavour.44": "Sortiere die Szenen nach Dramatik",
  "loadProgress.flavour.45": "Gönne dem Gateway einen Moment für sich",
  "loadProgress.flavour.46": "Bestätige, dass die Sonne weiterhin im Osten aufgeht",
  "loadProgress.flavour.47": "Zähle die Geräte, dann noch einmal",
  "loadProgress.flavour.48": "Mache Platz für die Gebäudestruktur",
  "loadProgress.flavour.49": "Lege die unbekannten Attribute sorgfältig ab",
  "loadProgress.flavour.50": "Überrede das Projekt, sich zu öffnen",

  // T23 (ADR-0024). Die Hilfe ist der längste Fließtext dieser Anwendung,
  // und damit die Stelle, an der eine Wort-für-Wort-Übertragung am
  // deutlichsten auffiele. Die Absätze sind daher auf Deutsch geschrieben,
  // nicht übersetzt: gleiche Aussage, gleiche Reihenfolge, eigene Sätze.
  // Fachbegriffe bleiben die der KNX-Praxis (Gruppenadresse,
  // Kommunikationsobjekt, Linie, Bereich), die Flag-Buchstaben bleiben
  // R/W/T/U/C, weil sie auf dem Gerät auch so heißen.
  "toolbar.help": "Hilfe (F1)",
  "help.title": "Hilfe",
  "help.intro":
    "Was die Teile dieses Fensters tun und was die KNX-Begriffe dahinter bedeuten. F1 öffnet das hier von überall im Hauptfenster; das abgetrennte Diagnosefenster hat keine eigene Hilfe.",
  "help.topics": "Themen",
  "help.close": "Schließen",
  "help.standardNote":
    "Das beschreibt, wie KNXBench den Begriff verwendet, nicht wie der KNX Standard ihn definiert — maßgeblich bleibt der Standard.",

  "help.topic.gettingStarted.title": "Erste Schritte",
  "help.topic.gettingStarted.p1":
    "KNXBench öffnet zwei Arten von Datei. Das eigene Format, ein .knxdb-Projekt, liest „Öffnen (.knxdb)…“ und schreibt „Speichern“. Einen ETS-Projektexport, eine .knxproj-Datei, liest „Projekt öffnen…“ und wandelt sie beim Einlesen in ein KNXBench-Projekt um — einmal. Danach lebt das Projekt als .knxdb weiter; KNXBench schreibt keine .knxproj-Dateien.",
  "help.topic.gettingStarted.p2":
    "Ein Import schreibt nie in die importierte Datei zurück. Die .knxproj wird gelesen und bleibt unangetastet; was entsteht, ist ein Projekt im Arbeitsspeicher, und auf die Festplatte kommt es erst, wenn Sie es als .knxdb speichern.",
  "help.topic.gettingStarted.p3":
    "Ohne Ausgangsdatei legt „Neues Projekt…“ ein leeres Projekt mit Name, Projektsprache und Gruppenadressstil an; die Struktur bauen Sie danach im Baum links auf.",

  "help.topic.workbench.title": "Das Fenster",
  "help.topic.workbench.p1":
    "Drei Bereiche. Links die Navigation mit den Ansichten und dem Projektbaum. In der Mitte die gewählte Ansicht. Rechts die Eigenschaften, die das jeweils Ausgewählte anzeigen und bearbeiten lassen.",
  "help.topic.workbench.p2":
    "Die vier Ansichten sind Übersicht, Gebäude, Topologie und Gruppenadressen. Die Übersicht fasst das Projekt zusammen und zeigt, was der letzte Import bemängelt hat. Gebäude und Topologie sind zwei Ordnungen über denselben Geräten. Gruppenadressen ist die Tabelle der Adressen samt ihren Verknüpfungen.",
  "help.topic.workbench.p3":
    "Beide Seitenbereiche lassen sich über die Schaltflächen darüber einklappen und an ihrer inneren Kante in der Breite ziehen. Der Produktkatalog öffnet sich aus der Navigation und fügt Geräte aus einer installierten Herstellerdatenbank ein.",

  "help.topic.buildings.title": "Gebäude, Geschosse und Räume",
  "help.topic.buildings.p1":
    "Die Gebäudestruktur sagt, wo ein Gerät körperlich sitzt — Gebäude, Geschoss, Raum, Verteiler. Sie gibt es, damit man ein Gerät findet, indem man das Haus abgeht, statt sich seine Adresse zu merken.",
  "help.topic.buildings.p2":
    "Hier ist sie ein Baum benannter Teile, die einander enthalten, und keine Zeichnung: keine Koordinaten, keine Grundrisse, keine Positionen. Ein Gerät gehört zu höchstens einem Gebäudeteil, und ein Umhängen ändert nichts an seiner Verdrahtung.",

  "help.topic.topology.title": "Bereiche, Linien und Geräte",
  "help.topic.topology.p1":
    "Die Topologie sagt, wie ein Gerät verdrahtet ist: Bereiche enthalten Linien, Linien enthalten Geräte. Die physikalische Adresse eines Geräts schreibt sich Bereich.Linie.Gerät und ist seine Identität am Bus.",
  "help.topic.topology.p2":
    "Die drei Teile sind unterschiedlich breit. Bereich geht von 0 bis 15, Linie von 0 bis 15, Gerät von 0 bis 255. Eine Adresse außerhalb dieser Bereiche wird abgelehnt und nicht stillschweigend abgeschnitten.",
  "help.topic.topology.p3":
    "Eine physikalische Adresse ist im Projekt eindeutig: zwei Geräte können nicht dieselbe tragen. In der Topologie-Ansicht sehen Sie, an welcher Linie ein Gerät hängt, und dort verschieben Sie es auch.",

  "help.topic.groupAddresses.title": "Gruppenadressen",
  "help.topic.groupAddresses.p1":
    "Eine Gruppenadresse ist kein Gerät. Sie benennt einen gemeinsamen Zustand — das Ein/Aus einer Leuchte, die Position einer Jalousie, den Sollwert eines Raums — auf den beliebig viele Geräte senden oder hören können. Geräte erreichen einander ausschließlich über Gruppenadressen.",
  "help.topic.groupAddresses.p2":
    "Darunter liegt eine einzelne 16-Bit-Zahl. Wie sie geschrieben wird, entscheidet das Projekt als Ganzes: dreistufig (Haupt/Mittel/Unter), zweistufig (Haupt/Unter) oder frei, also schlicht die Zahl. Ein Stilwechsel ändert die Schreibweise, nie die Adresse.",
  "help.topic.groupAddresses.p3":
    "Zu einer Gruppenadresse gehört außerdem ein Datenpunkttyp, der sagt, wie die Bytes auf der Leitung zu lesen sind — 1.001 ist ein Schalter, 5.001 ein Prozentwert. Die Ansicht zeigt den vorhandenen Typ, markiert Adressen ohne Typ und solche, deren Verknüpfungen sich über den Typ uneinig sind.",

  "help.topic.comObjectFlags.title": "Flags der Kommunikationsobjekte",
  "help.topic.comObjectFlags.p1":
    "Ein Kommunikationsobjekt ist ein Ein- oder Ausgang des Applikationsprogramms eines Geräts. Es mit einer Gruppenadresse zu verknüpfen, setzt das Gerät auf diese Adresse. KNXBench bildet dafür sechs Flags ab — in den Eigenschaften stehen sie als R, W, T, U, C und I — und zusammen legen sie fest, was das Objekt dort darf.",
  "help.topic.comObjectFlags.p2":
    "C, Kommunikation, ist der Hauptschalter: ist es aus, nimmt das Objekt am Busverkehr gar nicht teil, und die übrigen fünf haben nichts, woran sie wirken könnten. R, Lesen, lässt das Objekt eine Leseanfrage mit seinem aktuellen Wert beantworten. W, Schreiben, lässt ein eintreffendes Telegramm diesen Wert ändern.",
  "help.topic.comObjectFlags.p3":
    "T, Übertragen, lässt das Objekt von sich aus senden, wenn sein Wert sich ändert — das Flag, das aus einem Sensor einen Sender macht. U, Aktualisieren, lässt es einen Wert übernehmen, den es in der Leseantwort eines anderen Geräts sieht. I, Lesen bei Initialisierung, fragt den Wert der Adresse einmal beim Start am Bus ab, damit das Objekt mit einem echten Wert beginnt und nicht mit einer Annahme. KNXBench speichert die sechs so, wie Sie sie setzen, und beurteilt nicht, welche Kombination zu Ihrem Gerät passt.",

  "help.topic.busMonitor.title": "Busmonitor",
  "help.topic.busMonitor.p1":
    "Der Busmonitor verfolgt den laufenden Verkehr über ein KNXnet/IP-Gateway. Adresse und Port eintragen, verbinden, und die Telegramme erscheinen, sobald sie eintreffen. Mitlesen und Senden sind getrennt: die Tabelle zeigt nur an, und ein Telegramm zu senden ist ein eigener, bewusster Schritt.",
  "help.topic.busMonitor.p2":
    "Tunneling ist der einzige Übertragungsweg. Dieses Fenster sucht das Netz nicht nach Gateways ab und tritt keiner Routing-Multicast-Gruppe bei — deshalb muss die Adresse von Hand eingetragen werden.",
  "help.topic.busMonitor.p3":
    "Telegramme werden gegen das geöffnete Projekt decodiert. Ändert sich dieses Projekt während einer laufenden Sitzung, sagt der Monitor das, statt bereits Decodiertes klammheimlich umzubeschriften — der Hinweis bedeutet, dass die decodierte Spalte die Antwort des älteren Projektstands ist.",

  "help.topic.importExport.title": "Import",
  "help.topic.importExport.p1":
    "Ein Import berichtet, was er vorgefunden hat: Fehler, Warnungen und alles, was er nicht zuordnen konnte. Die Übersicht zeigt die Zahlen, das Protokoll führt sie einzeln auf. Beides lohnt sich zu lesen — sonst sehen ein Import ohne Befund und ein Import, in den niemand geschaut hat, gleich aus.",
  "help.topic.importExport.p2":
    "Daten, die diese Anwendung nicht abbildet, werden nicht weggeworfen. Sie werden entweder unverändert aufbewahrt oder als nicht unterstützt aufgeführt und tauchen im Importbericht auf, statt zwischen Datei und Projekt zu verschwinden.",
  "help.topic.importExport.p3":
    "Der Import läuft nur in eine Richtung. KNXBench liest eine .knxproj und schreibt nie eine, es gibt also keinen Weg über eine Projektdatei zurück zur ETS — ab dem ersten Speichern ist das Projekt eine .knxdb. Was die Anwendung dennoch verlässt: Gruppenadressen lassen sich als CSV ein- und ausgeben, und „Dokumentation exportieren…“ schreibt eine lesbare Beschreibung des Projekts.",

  "help.topic.keyboard.title": "Tastatur",
  "help.topic.keyboard.p1":
    "Strg+K öffnet die Suche über das Projekt. Strg+Umschalt+P öffnet die Befehlspalette, die zeigt, was gerade möglich ist, samt Tastenkürzel. F1 öffnet diese Hilfe — nur F1 allein; ein F1 mit Zusatztaste bleibt dem Browser und der Arbeitsumgebung überlassen.",
  "help.topic.keyboard.p2":
    "Strg+Z macht rückgängig, Strg+Umschalt+Z stellt wieder her, und zwar am Projekt, nicht an dem Text, den Sie gerade tippen: in einem Eingabefeld, einem Dialog oder beim Umbenennen im Baum gilt das Rückgängig des Feldes.",
  "help.topic.keyboard.p3":
    "Esc schließt, was obenauf liegt — zuerst einen offenen Hilfehinweis, dann den Dialog darum herum. Solange ein Dialog offen ist, wandert Tab nur in ihm und kann ihn nicht verlassen; beim Schließen kehrt der Fokus dorthin zurück, wo er herkam.",

  "help.topic.limits.title": "Was diese Anwendung nicht tut",
  "help.topic.limits.p1":
    "KNXBench ist eine unabhängige Anwendung. Sie stammt nicht von der KNX Association, ist von ihr weder unterstützt noch zertifiziert, und sie ist nicht die ETS. Wo sie eine ETS-Datei liest, tut sie das nach eigener Lesart dieser Datei.",
  "help.topic.limits.p2":
    "KNX Secure wird nicht unterstützt. Ein kennwortgeschütztes Projekt lässt sich aus diesem Fenster nicht öffnen — es gibt keine Stelle, an der das Kennwort einzugeben wäre. Geräte über den Bus zu programmieren, ist hier ebenfalls nicht vorgesehen: die Busfunktionen sind Mitlesen und das Senden einzelner Telegramme. Zurück zur ETS führt kein Weg: KNXBench liest eine .knxproj und schreibt nie eine, ein hier importiertes Projekt lässt sich also nicht wieder als ETS-Projektdatei herausgeben.",
  "help.topic.limits.p3":
    "Was vorhanden ist, ist getestet, aber eine Testsuite ist keine Begehung. Bevor Sie sich in einer Anlage auf diese Anwendung verlassen, gleichen Sie ab, was sie Ihnen sagt, mit dem, was die Anlage tut.",

  "help.tip.comFlags.label": "Was die Flags der Kommunikationsobjekte bedeuten",
  "help.tip.comFlags.text":
    "R, W, T, U, C und I legen fest, was dieses Objekt am Bus darf. C ist der Hauptschalter — ist es aus, bewirken die anderen fünf nichts. Die ausführliche Erklärung steht unter F1.",
  "help.tip.addressTable.label": "Was diese Tabelle zeigt",
  "help.tip.addressTable.text":
    "Alle Gruppenadressen des Projekts mit ihrem Datenpunkttyp und den damit verknüpften Kommunikationsobjekten. Das Feld filtert nach Adresse, Name oder Datenpunkttyp. Was eine Gruppenadresse ist, erklärt F1.",
  "help.tip.busGateway.label": "Was als Gateway einzutragen ist",
  "help.tip.busGateway.text":
    "Adresse und Port eines KNXnet/IP-Gateways, etwa 192.0.2.1:3671. Nur Tunneling — diese Anwendung sucht das Netz nicht nach einem Gateway ab.",

  "workbench.resizeNavigation": "Höhe des Navigationsblocks",
  "workbench.resizeDiagnostics": "Höhe des Diagnoseblocks",

  "toolbar.quit": "Beenden",
  "toolbar.about": "Über KNXBench…",
  "toolbar.logout": "Abmelden",

  // T01b / ADR-0026 — der Anmeldebildschirm. Der Eyebrow ist in beiden
  // Katalogen gleich gemeint wie `language.en`/`language.de`: „KNX-compatible“
  // ist die Formulierung, die dieses Projekt überall führt, und wird nicht
  // übersetzt, weil sie eine Aussage über Kompatibilität ist und keine
  // Werbezeile.
  "login.eyebrow": "KNX-compatible · Linux-first",
  "login.intro":
    "Dieser Server ist passwortgeschützt. Geben Sie das Passwort ein, um weiterzuarbeiten.",
  "login.password": "Passwort",
  "login.submit": "Anmelden",
  "login.pending": "Wird angemeldet…",
  "login.rejected": "Dieses Passwort wurde nicht akzeptiert.",
  "login.checking": "Der Server wird gefragt, ob er ein Passwort verlangt…",
  "login.expiredNotice":
    "Ihre Sitzung ist beendet — entweder durch Zeitablauf oder weil der Server neu gestartet wurde. Melden Sie sich erneut an, um weiterzuarbeiten; was auf dem Bildschirm steht, ist erhalten geblieben. Wurde der Server allerdings neu gestartet, ist seine Kopie des Projekts weg und Sie müssen es erneut öffnen.",
  "login.signedOutNotice":
    "Sie sind abgemeldet. Nach erneuter Anmeldung steht die Arbeitsfläche wieder genauso da, wie Sie sie verlassen haben.",
  "login.footnote":
    "Ein Passwort für den ganzen Server, festgelegt beim Start. Benutzerkonten gibt es noch nicht.",

  "quit.title": "Nicht gespeicherte Änderungen",
  "quit.message":
    "Dieses Projekt hat Änderungen, die noch in keiner Datei stehen. Wer jetzt beendet, wirft sie weg.",
  "quit.hint":
    "Brechen Sie ab, speichern Sie mit „Speichern“ oder „Speichern unter…“, und beenden Sie danach.",
  "quit.cancel": "Abbrechen",
  "quit.discard": "Ohne Speichern beenden",

  "about.title": "Über KNXBench",
  "about.version": "Version",
  "about.versionUnknown": "unbekannt — der Server hat nicht geantwortet",
  "about.licence": "Lizenz",
  "about.licenceValue": "AGPL-3.0-or-later",
  "about.independence":
    "KNXBench ist ein unabhängiges Projekt. Es ist von der KNX Association nicht zertifiziert und steht mit ihr in keiner Verbindung.",
  "about.trademark": "ETS ist eine Marke der KNX Association.",
  "about.close": "Schließen",

  // T29 — der Fehlerbericht. Der Dialog ist übersetzt, report.md und der
  // GitHub-Text nicht (beides erzeugt der Server auf Englisch, gelesen von
  // dem, der das Issue bearbeitet).
  "debugReport.button": "Fehlerbericht…",
  "debugReport.title": "Fehlerbericht",
  "debugReport.intro":
    "Sammelt das, was zur Fehlersuche taugt, in einer Zip-Datei auf diesem Rechner. Es wird nichts verschickt: die Datei entsteht dort, wo Sie sie hinlegen, und die GitHub-Schaltfläche öffnet lediglich eine vorausgefüllte Issue-Seite im Browser, die Sie vor dem Absenden lesen.",
  "debugReport.descriptionLabel": "Was ist passiert?",
  "debugReport.descriptionPlaceholder": "Was Sie getan haben, was Sie erwartet haben, was stattdessen geschah.",
  "debugReport.descriptionHint":
    "Kommt in Ihren eigenen Worten in den Bericht. Alles Weitere unten wird automatisch gesammelt.",
  "debugReport.filterName": "Zip-Archiv",
  "debugReport.include.log.label": "Sitzungsprotokoll",
  "debugReport.include.log.hint":
    "Die Einträge aus dem Reiter „Log“, ohne IP-Adressen. Importprobleme werden namentlich protokolliert, deshalb kann diese Datei KNX-Gruppenadressen und Namen importierter Elemente enthalten.",
  "debugReport.include.projectSummary.label": "Projektstatistik",
  "debugReport.include.projectSummary.hint":
    "Wie viele Geräte, Linien und Gruppenadressen das geöffnete Projekt hat. Nur Anzahlen — keine Namen, keine Adressen.",
  "debugReport.include.busTelegrams.label": "Telegramme des Busmonitors",
  "debugReport.include.busTelegrams.hint":
    "Die Telegramme, die gerade im Puffer des Monitors liegen. Sie behalten die physikalischen und Gruppenadressen Ihrer Geräte.",
  "debugReport.contentsTitle": "Was in der Datei stehen wird",
  "debugReport.contents.report": "report.md — Ihre Beschreibung, die Versionen und die Umgebung.",
  "debugReport.contents.environment": "environment.json — dieselben Angaben maschinenlesbar.",
  "debugReport.contents.log":
    "log.json — die Protokolleinträge dieser Sitzung; kann KNX-Adressen und importierte Elemente benennen.",
  "debugReport.contents.projectSummary": "project-summary.json — Anzahlen zum geöffneten Projekt.",
  "debugReport.contents.busTelegrams": "bus-telegrams.json — der Puffer des Busmonitors.",
  "debugReport.privacyRedacted":
    "IP-Adressen, Ihr Home-Verzeichnis und der Name dieses Rechners werden in report.md, environment.json und log.json durch Platzhalter ersetzt. KNX-Adressen und Namen aus Ihrem Projekt werden nirgends ersetzt.",
  "debugReport.privacyTelegrams":
    "bus-telegrams.json behält die physikalischen und Gruppenadressen Ihrer Anlage und, soweit das geöffnete Projekt sie kennt, die Namen der Gruppenadressen — „Küche Deckenlicht“. Ohne diese Angaben sagt ein Telegrammmitschnitt nichts aus, deshalb bleiben sie stehen. Nehmen Sie die Datei nur auf, wenn Sie das weitergeben wollen.",
  "debugReport.save": "Zip speichern…",
  "debugReport.openIssue": "GitHub-Issue öffnen…",
  "debugReport.close": "Schließen",
  "debugReport.busy": "Wird gesammelt…",
  "debugReport.saved.one": "Fehlerbericht gespeichert, {count} Datei.",
  "debugReport.saved.other": "Fehlerbericht gespeichert, {count} Dateien.",
  "debugReport.issueOpened":
    "Im Browser wurde ein vorausgefülltes Issue geöffnet. Abgeschickt ist nichts — lesen Sie es, hängen Sie die Zip-Datei an und senden Sie es selbst ab.",
  "debugReport.issueTruncated":
    "Im Browser wurde ein vorausgefülltes Issue geöffnet. Der Bericht war für einen Link zu lang und wurde gekürzt — speichern Sie bitte zusätzlich die Zip-Datei und hängen Sie sie an.",
  "debugReport.issueTitle": "Fehlerbericht aus KNXBench",
};
