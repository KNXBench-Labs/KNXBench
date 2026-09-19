/** English UI message catalogue whose keys define MessageKey and every catalogue's key set. */
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
//
// Task 3 (T25) added everything from `toolbar.busMonitor` down: the app
// shell's remaining toolbar buttons, the command palette's own "Search…"
// label (distinct from `toolbar.search`, which carries the `(Ctrl+K)`
// shortcut suffix the palette renders separately), the Dashboard, the
// documentation export button's plural summary line, and every toast
// occasion string from `toastCopy.ts` (18 holiday/late-night jokes plus 7
// error wrappers) — the joke pairing per occasion is preserved key-by-key
// so `toastCopy.ts` and `toast.ts` don't need to know which language is
// active, only which key to ask for.
//
// Task 4 (T25) added everything from `inspector.*` down: `Inspector.tsx`,
// `ParameterPanel.tsx` and `ProjectExplorer.tsx`. `buildingPartKind.*` is
// the one namespace that breaks the per-surface convention on purpose — it
// is the same six-value `BuildingPartType` label shown both in
// `Inspector.tsx`'s `BuildingPartInspector` and in `ProjectExplorer.tsx`'s
// `BuildingItem`/`NewBuildingPartRow`, and translating the same domain word
// two different ways in two files is a bug waiting to happen, not a
// feature of the namespacing convention.
export const messages = {
  "toolbar.newProject": "New project…",
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
  "toolbar.busMonitor": "Bus monitor",
  "toolbar.commands": "Commands… (Ctrl+Shift+P)",
  "settings.uiLanguage": "UI language",
  "language.en": "English",
  "language.de": "Deutsch",

  "command.search": "Search…",

  "dashboard.title": "Project status",
  "dashboard.schemaVersion": "Schema version",
  "dashboard.installations": "Installations",
  "dashboard.areas": "Areas",
  "dashboard.lines": "Lines",
  "dashboard.devices": "Devices",
  "dashboard.devicesUnassigned": " ({count} unassigned)",
  "dashboard.groupAddresses": "Group addresses",
  "dashboard.buildingParts": "Building parts",
  "dashboard.comObjects": "Communication objects",
  "dashboard.importErrors": "Import errors",
  "dashboard.importWarnings": "Import warnings",

  "documentationExport.button": "Export documentation…",
  "documentationExport.summaryNone": "Project documentation exported, no warnings.",
  "documentationExport.summaryWithWarnings.one": "Project documentation exported, {count} warning — see Log.",
  "documentationExport.summaryWithWarnings.other": "Project documentation exported, {count} warnings — see Log.",

  "toast.dismiss": "Dismiss",
  // §67's shape (a translated sentence quoting an untranslated reason)
  // closed for language-pack rejection; this is the same shape here,
  // left standing on purpose (KNOWN_LIMITATIONS.md §66) — disclosed
  // rather than silently presented as a translation gap. Rendered once
  // per error toast, next to the `{msg}` it explains.
  "toast.error.messageIsEnglish": "This message is the server's own text, in English.",

  "toast.error.notAsPlanned": "Well, that didn't go as planned: {msg}",
  "toast.error.busObjects": "The bus objects: {msg}",
  "toast.error.gremlins": "Gremlins in the wiring: {msg}",
  "toast.error.knxSaysNo": "KNX says no: {msg}",
  "toast.error.notToday": "Not today: {msg}",
  "toast.error.houston": "Houston, we have a problem: {msg}",
  "toast.error.hardPass": "That's a hard pass: {msg}",
  // 23 more error wrappers, same register, same `{msg}` contract as the
  // original seven above.
  "toast.error.marvinSigh": "Marvin would sigh, then say: {msg}",
  "toast.error.uncaringUniverse": "Another glorious diagnostic in an uncaring universe: {msg}",
  "toast.error.busSpoken": "The bus has spoken, and it is unimpressed: {msg}",
  "toast.error.brainSizeOfPlanet": "Brain the size of a planet, and still: {msg}",
  "toast.error.dontTalkToMeAboutLife": "Life, don't talk to me about life. Or this: {msg}",
  "toast.error.relayDespair": "Somewhere, a relay clicked in despair: {msg}",
  "toast.error.dungeonKeeperNarrates": "The dungeon keeper narrates your doom: {msg}",
  "toast.error.oldTrick": "Ah, yes. This old trick: {msg}",
  "toast.error.wiringConspires": "The wiring conspires again: {msg}",
  "toast.error.nothingWorks": "Nothing works, and yet the day continues: {msg}",
  "toast.error.minorApocalypse": "A minor apocalypse, KNX-flavored: {msg}",
  "toast.error.topologySighed": "The topology sighed audibly: {msg}",
  "toast.error.hopeNowhere": "Group addresses everywhere, hope nowhere: {msg}",
  "toast.error.triumphOfEntropy": "Yet another triumph of entropy: {msg}",
  "toast.error.telegramBadNews": "The telegram arrived, bearing bad news: {msg}",
  "toast.error.dontPanicWorse": "Don't panic. It's worse than that: {msg}",
  "toast.error.busLineComplaint": "The bus line files its complaint: {msg}",
  "toast.error.loadStateMachineWept": "Somewhere a load state machine wept: {msg}",
  "toast.error.alsoInevitable": "This, too, was inevitable: {msg}",
  "toast.error.gremlinsRegards": "The gremlins send their regards: {msg}",
  "toast.error.mediocrityInErrorForm": "Behold, mediocrity in error form: {msg}",
  "toast.error.universeIndifferent": "The universe remains profoundly indifferent: {msg}",
  "toast.error.filedUnderOfCourse": 'Filed under "of course": {msg}',

  "toast.holiday.newYear.groupAddresses": "Happy New Year! May your group addresses stay unique.",
  "toast.holiday.newYear.sameAddresses": "New year, same group addresses.",
  "toast.holiday.valentine.roses": "Roses are red, buses are twisted pair.",
  "toast.holiday.valentine.favorite": "Be my Valentine, my favorite communication object.",
  "toast.holiday.aprilFools.noBugs": "No bugs today. Probably.",
  "toast.holiday.aprilFools.real": "Everything in this build is 100% real. Trust us.",
  "toast.holiday.halloween.spooky": "Spooky season: even the ghosts use KNX for the lighting.",
  "toast.holiday.halloween.boo": "Boo! Your project is still safe.",
  "toast.holiday.christmasEve.hoho": "Ho ho ho, don't forget to save your project.",
  "toast.holiday.christmasEve.silentNight": "Silent night, wired bright.",
  "toast.holiday.christmasDay.santa": "Merry Christmas! Even Santa needs a group address for the chimney sensor.",
  "toast.holiday.christmasDay.greetings": "Season's greetings from your KNX app.",
  "toast.holiday.newYearsEve.oneMoreSave": "One more save before midnight?",
  "toast.holiday.newYearsEve.seeYou": "See you next year, project file.",
  // 23 more holidays, fixed calendar dates only (`HolidayEntry` has no
  // notion of movable feasts) — a mix of German-calendar and
  // computing-culture dates, two jokes each, same pairing convention as
  // the original seven above.
  "toast.holiday.epiphany.starlight":
    "The three wise men found their way by starlight. Your group addresses could use a similar miracle.",
  "toast.holiday.epiphany.noneArrived": "Epiphany, allegedly. No revelation arrived about the wiring.",
  "toast.holiday.piDay.neverResolves":
    "Pi Day: an infinite, non-repeating reminder that some things never resolve cleanly. Much like your open bugs.",
  "toast.holiday.piDay.percentSolved": "3.14 percent of your problems are solved today. The rest continue as usual.",
  "toast.holiday.backupDay.reminder": "World Backup Day. A gentle, mildly threatening reminder to save your project.",
  "toast.holiday.backupDay.hardWay":
    "Somewhere, someone is learning about backups the hard way. Not you, hopefully.",
  "toast.holiday.earthDay.lightsOff":
    "Earth Day. KNX exists partly so lights turn off when nobody's looking. You're welcome, planet.",
  "toast.holiday.earthDay.energyBill":
    "One day a year the planet gets a toast. Every day, your energy bill gets a group address.",
  "toast.holiday.germanBeerDay.reinheitsgebot":
    "Tag des Deutschen Bieres. The Reinheitsgebot regulated beer from 1516; nobody has ever regulated your group address naming.",
  "toast.holiday.germanBeerDay.rulesNeeded":
    "In 1516 Bavaria decided beer needed rules. Your project could use some too.",
  "toast.holiday.tagDerArbeit.busNoDayOff": "Tag der Arbeit. The bus, notably, does not get the day off.",
  "toast.holiday.tagDerArbeit.lineOnDuty":
    "A holiday for workers everywhere. The KNX line remains, as ever, on duty.",
  "toast.holiday.starWarsDay.fourthBeWithYou":
    "May the fourth be with you. The bus, less mystically, remains twisted pair.",
  "toast.holiday.starWarsDay.sithLord": "Somewhere a Sith lord is also debugging a topology. Solidarity.",
  "toast.holiday.telecomDay.telegramsRegards":
    "World Telecommunication Day. KNX telegrams send their regards, unread as usual.",
  "toast.holiday.telecomDay.busUnimpressed":
    "A whole day honoring telecommunication. The bus remains characteristically unimpressed.",
  "toast.holiday.towelDay.bringOne": "Towel Day. Bring one. It won't fix the wiring, but it helps morale.",
  "toast.holiday.towelDay.mostlyBroken": "Don't panic. Your project is merely mostly broken, not entirely.",
  "toast.holiday.environmentDay.savingPlanet":
    "World Environment Day. Somewhere, a well-configured KNX installation is quietly saving the planet.",
  "toast.holiday.environmentDay.rarelySaysSo":
    "The environment thanks you for the automation. It rarely says so directly.",
  "toast.holiday.summerSolstice.longestDay":
    "Sommersonnenwende. Roughly the longest day, give or take the calendar's own rounding errors, for maximum exposure to unresolved diagnostics.",
  "toast.holiday.summerSolstice.todoList": "The sun barely sets tonight. Neither, it seems, does your to-do list.",
  "toast.holiday.moonLanding.lessComputingPower":
    "On this day humanity landed on the Moon with less computing power than your average KNX gateway. Perspective.",
  "toast.holiday.moonLanding.houstonSmaller": "Houston had a problem once, too. Yours is smaller, and stays on Earth.",
  "toast.holiday.swissNationalDay.chalet":
    "Schweizer Bundesfeiertag. Somewhere, a very precisely wired chalet celebrates on schedule.",
  "toast.holiday.swissNationalDay.punctuality":
    "Switzerland's national day. Your project's punctuality remains a separate matter entirely.",
  "toast.holiday.programmerDay.day256":
    "Tag des Programmierers, the 256th day of the year, give or take a leap. A number chosen by programmers, for programmers, understood by nobody else.",
  "toast.holiday.programmerDay.countedOwnDays":
    "A holiday that exists because programmers counted their own days. Fitting.",
  "toast.holiday.pirateDay.plunderedByNobody":
    "Arrr. Ye group addresses be plundered by nobody, which is, admittedly, the point.",
  "toast.holiday.pirateDay.unpiratical":
    "Talk Like a Pirate Day. The bus telegrams remain resolutely un-piratical.",
  "toast.holiday.germanUnity.twoNetworks":
    "Tag der Deutschen Einheit. Two networks became one in 1990; yours, presumably, was always this way.",
  "toast.holiday.germanUnity.ownAffair":
    "A day celebrating unification. Your group address ranges remain stubbornly their own affair.",
  "toast.holiday.backToTheFuture.noFlyingCars":
    "The future, contrary to prediction, has no flying cars. It does have KNX. Small victories.",
  "toast.holiday.backToTheFuture.noHoverboards":
    "Back to the Future Day. No hoverboards arrived. The bus, at least, is on time.",
  "toast.holiday.allSaintsDay.devicesRemembered":
    "Allerheiligen. A quiet day, in memory of every device that didn't survive commissioning.",
  "toast.holiday.allSaintsDay.addressesBefore":
    "All Saints' Day. Spare a thought for the group addresses that came before this project.",
  "toast.holiday.elevenEleven.karnevalBegins":
    "Elfter im Elften, eleven-eleven. Karneval begins; your project's chaos, notably, never took a season off.",
  "toast.holiday.elevenEleven.foolsOfficially":
    "The fools take over today, officially. The bus was unofficially ahead of them all along.",
  "toast.holiday.computerSecurityDay.knxSecureExists":
    "Computer Security Day. A fine occasion to remember that KNX Secure exists, even where this build doesn't touch it yet.",
  "toast.holiday.computerSecurityDay.trustsEveryone":
    "A day for computer security. The bus, as ever, trusts everyone on it completely.",
  "toast.holiday.nikolaustag.properCommissioning":
    "Nikolaustag. Good devices get commissioned properly; the rest get a stern diagnostic instead of coal.",
  "toast.holiday.nikolaustag.warningsOnList":
    "St. Nicholas checks his list. Your unresolved warnings are, regrettably, still on it.",
  "toast.holiday.winterSolstice.longestNight":
    "Wintersonnenwende, roughly the longest night, the calendar's usual rounding notwithstanding. Plenty of time for the lighting group addresses to earn their keep.",
  "toast.holiday.winterSolstice.backlogSameLength":
    "The shortest day of the year. Somehow, the backlog remains exactly as long.",
  "toast.holiday.boxingDay.importWarningsUnopened":
    "Zweiter Weihnachtstag. The presents are open; the import warnings, less excitingly, remain unopened too.",
  "toast.holiday.boxingDay.readingManualLate":
    "Boxing Day. Somewhere, someone is finally reading the manual. Bit late for that.",

  "toast.lateNight.midnightOil": "Burning the midnight oil? So is your KNX bus.",
  "toast.lateNight.busLineRest": "It's late. Even the bus line needs rest.",
  "toast.lateNight.stillAwake": "Still awake? The group addresses admire your dedication.",
  "toast.lateNight.nightOwl": "Night owl mode engaged.",
  // 26 more late-night lines, same register as the original four.
  "toast.lateNight.busQuiet": "The bus is quiet. You, apparently, are not.",
  "toast.lateNight.relayAwake": "Somewhere, a relay is also awake and equally unimpressed.",
  "toast.lateNight.hourOfRegret": "This is the hour reserved for regret and configuration files.",
  "toast.lateNight.buildingSleeps": "The building sleeps. You do not. Interesting choices.",
  "toast.lateNight.addressesInBed": "Even the group addresses have gone to bed.",
  "toast.lateNight.insomniaAndKnx": "Insomnia and KNX: a time-honored pairing.",
  "toast.lateNight.darkHoursSuitDebugging": "The dark hours suit debugging. Allegedly.",
  "toast.lateNight.reasonableHourElsewhere": "Somewhere it is a reasonable hour. Not here.",
  "toast.lateNight.topologyHoldsBreath": "The topology holds its breath until morning.",
  "toast.lateNight.anotherCommit": "Another commit, another sunrise avoided.",
  "toast.lateNight.wiringDiagramsDontJudge": "The wiring diagrams don't judge. Probably.",
  "toast.lateNight.stillHere": "Still here. So is the bus, technically.",
  "toast.lateNight.linesAreSilent": "The lines are silent. You are the exception.",
  "toast.lateNight.thisIsFine": "This is fine. Everything about this hour is fine.",
  "toast.lateNight.gatewayBlinks": "The gateway blinks patiently into the dark.",
  "toast.lateNight.sleepIsForFewerBugs": "Sleep is for installations with fewer bugs.",
  "toast.lateNight.busMonitorLogsSoul": "The bus monitor logs one more soul who should be resting.",
  "toast.lateNight.telegramDedicationOrDespair":
    "A telegram at this hour means dedication or despair. Possibly both.",
  "toast.lateNight.lightsAreOff": "The lights are off. The universe's judgment, notably, is not looking any better lit.",
  "toast.lateNight.loadStateMachineClockedOut": "Even the load state machine has clocked out.",
  "toast.lateNight.hourBelongsToOwls": "This hour belongs to owls and unresolved diagnostics.",
  "toast.lateNight.projectWaits": "The project waits. It has nowhere else to be.",
  "toast.lateNight.deviceRebootsUnaware": "Somewhere a device reboots, blissfully unaware of the time.",
  "toast.lateNight.darkIsVast": "The dark is vast and, this once, so is your uptime.",
  "toast.lateNight.groupRangesNoOfficeHours": "Group ranges don't keep office hours. Neither, apparently, do you.",
  "toast.lateNight.busLineClocksOut": "The bus line clocks out. You, evidently, do not.",

  // `Inspector.tsx`. `inspector.entity.*` and `inspector.restrictedAction.*`
  // are the two halves `inspector.restrictedToFirstInstallation` composes
  // itself from — see that key's own comment for why it's built this way
  // instead of six near-duplicate sentences.
  "inspector.address": "Address",
  "inspector.description": "Description",
  "inspector.dpt": "DPT",
  "inspector.comFlag.read": "Read",
  "inspector.comFlag.write": "Write",
  "inspector.comFlag.transmit": "Transmit",
  "inspector.comFlag.update": "Update",
  "inspector.comFlag.communication": "Communication",
  "inspector.direction.send": "Send",
  "inspector.direction.receive": "Receive",
  "inspector.unlink": "Unlink",
  "inspector.chooseGroupAddress": "(choose a group address)",
  "inspector.link": "Link",
  "inspector.line": "Line",
  "inspector.unassigned": "(unassigned)",
  "inspector.areaLabel": "Area {address}: {name}",
  "inspector.lineLabel": "Line {address}: {name}",
  "inspector.buildingPart": "Building part",
  "inspector.none": "(none)",
  "inspector.delete": "Delete",
  // The two verb clauses `inspector.restrictedToFirstInstallation`'s
  // `{action}` slot takes — carrying their own verb (`is`/`are`) so the
  // base sentence never has to conjugate around how many verbs it's naming.
  "inspector.restrictedAction.delete": "Delete is",
  "inspector.restrictedAction.renameAndDelete": "Rename and Delete are",
  "inspector.restrictedToFirstInstallation": "{action} only available for {entity} in the first installation.",
  "inspector.entity.devices": "devices",
  "inspector.entity.groupAddresses": "group addresses",
  "inspector.entity.groupRanges": "group ranges",
  "inspector.entity.areas": "areas",
  "inspector.entity.lines": "lines",
  "inspector.entity.buildingParts": "building parts",
  "inspector.communicationObjects": "Communication objects",
  "inspector.unnamed": "(unnamed)",
  "inspector.name": "Name",
  "inspector.lineCount.one": "{count} line",
  "inspector.lineCount.other": "{count} lines",
  "inspector.deviceCount.one": "{count} device",
  "inspector.deviceCount.other": "{count} devices",
  "inspector.childPartCount.one": "{count} child part",
  "inspector.childPartCount.other": "{count} child parts",
  // Project node (KNOWN_LIMITATIONS.md §84) — read-only display, nothing
  // more: no control on this panel restyles the project (that happens, if
  // at all, through `POST /api/project/group-address-style`, which has no
  // UI entry point yet). `explorer.project` is the tree label,
  // `inspector.project` this panel's own heading.
  "explorer.project": "Project",
  "inspector.project": "Project",
  "inspector.groupAddressStyle": "Group address style",

  // Shared between `Inspector.tsx` and `ProjectExplorer.tsx` — see the
  // header comment above for why this one namespace isn't per-surface.
  "buildingPartKind.building": "Building",
  "buildingPartKind.floor": "Floor",
  "buildingPartKind.room": "Room",
  "buildingPartKind.corridor": "Corridor",
  "buildingPartKind.distributionBoard": "Distribution Board",
  "buildingPartKind.buildingPart": "Building Part",

  // `ParameterPanel.tsx`.
  "parameters.deviceScope": "Device",
  "parameters.moduleNumber": "Module #{number}",
  "parameters.sharedReadOnlyCaption":
    "Shared across every instantiation of this module; not editable here — see the diagnostics for why.",
  "parameters.staleValuesHeading": "Stale values ({count})",
  "parameters.staleDescription":
    "These stored values no longer correspond to any parameter in the current application program.",
  "parameters.diagnosticsCount.one": "{count} issue found while evaluating this device's parameters",
  "parameters.diagnosticsCount.other": "{count} issues found while evaluating this device's parameters",
  "parameters.copyDetails": "Copy details",

  // KNOWN_LIMITATIONS.md §66: one key per `ParameterDiagnosticKindDto`
  // variant (`apps/knx-server/src/routes.rs`) — the banner/section
  // headline text `ParameterPanel.tsx`'s `describeParameterDiagnosticMessage`
  // picks between. This is prose read in the normal course of using the
  // parameter editor, so it is translatable; the corresponding
  // `.detail` string never appears here and stays English (see
  // `ParameterDiagnostic.detail`'s doc comment in `api.ts`) — it exists
  // for a bug report, not for reading in German.
  "parameters.diagnostic.parametersUnreadable":
    "Some declared parameters could not be read from the product database and are not shown.",
  "parameters.diagnostic.duplicateUnscopedValue":
    "Two stored values target the same parameter; the later one is ignored.",
  "parameters.diagnostic.duplicateModuleScopedValue":
    "Two stored values target the same module-scoped parameter; the later one is ignored.",
  "parameters.diagnostic.duplicateModuleId":
    "Two or more sections in this program declare the same module id; its fields are read-only.",
  "parameters.diagnostic.noModuleInstanceMatch":
    "No imported module instance matches this module; its fields are read-only.",
  "parameters.diagnostic.ambiguousModuleInstance":
    "Two or more imported module instances share this module; its fields are read-only.",
  "parameters.diagnostic.malformedModuleInstanceId":
    "An imported module instance's identifier has an unexpected shape; this module's fields are read-only.",
  "parameters.diagnostic.noBranchMatched": "A choice did not match any of its options.",
  "parameters.diagnostic.unparsableTest": "A choice's condition could not be understood.",
  "parameters.diagnostic.unresolvedParamRef": "A choice's controlling parameter could not be found.",
  "parameters.diagnostic.nonNumericValue": "A choice's controlling value was not a valid number.",
  "parameters.diagnostic.unexpectedTypeNoneShape": "An unusual choice structure was skipped.",
  "parameters.diagnostic.unrecognizedNode": "An unrecognized program element was skipped.",
  "parameters.diagnostic.moduleDefNotFound": "A module could not be found in this program.",
  "parameters.diagnostic.moduleCycleDetected":
    "A module refers back to one of its own enclosing modules and was not expanded.",
  "parameters.diagnostic.moduleNestingTooDeep":
    "A module is nested deeper than this program will expand.",
  "parameters.diagnostic.moduleExpansionBudgetExhausted":
    "This program's modules are too numerous to fully expand; the rest were skipped.",
  "parameters.diagnostic.missingValue": "A choice's controlling parameter has no value.",
  "parameters.diagnostic.moduleWithoutId":
    "A module instance has no identifier and cannot be matched to stored values.",
  "parameters.diagnostic.moduleArgumentNotBound":
    "A module argument could not be matched to the module's declaration and was ignored.",
  "parameters.diagnostic.unsupportedModuleArgumentKind":
    "A module argument uses a kind this build does not interpret and was ignored.",
  "parameters.diagnostic.unresolvedTextPlaceholder":
    "A text placeholder had no matching module argument and was left as written.",

  "parameters.title": "Parameters",
  "parameters.loading": "Loading parameters…",
  "parameters.noProgram":
    "This device has no resolvable application program; parameters cannot be shown.",
  "parameters.none": "(none)",

  // `ProjectExplorer.tsx`.
  "explorer.addDevice": "+ Add device",
  "explorer.lineLabel": "Line {address}: {name}",
  "explorer.areaLabel": "Area {address}: {name}",
  "explorer.newLinePlaceholder": "New line",
  "explorer.newAreaPlaceholder": "New area",
  "explorer.add": "Add",
  "explorer.newGroupAddressPlaceholder": "New group address",
  "explorer.noRange": "(no range)",
  "explorer.newGroupRangePlaceholder": "New group range",
  "explorer.newMiddleRangePlaceholder": "New middle range",
  "explorer.newBuildingPlaceholder": "New building",
  "explorer.newBuildingPartPlaceholder": "New building part",
  "explorer.buildingLabel": "{name} ({kind})",
  "explorer.topology": "Topology",
  "explorer.buildings": "Buildings",
  "explorer.unassigned": "Unassigned",
  "explorer.groupAddresses": "Group Addresses",
  "explorer.groupRanges": "Group Ranges",
  "explorer.importErrorsCount.one": "{count} import error — data may be missing or incorrect",
  "explorer.importErrorsCount.other": "{count} import errors — data may be missing or incorrect",
  "explorer.importWarningsCount.one": "{count} import warning",
  "explorer.importWarningsCount.other": "{count} import warnings",

  // Task 5 (T25) — the last extraction pass: `LogPanel.tsx`,
  // `BusMonitorPanel.tsx`, `CatalogBrowser.tsx`, `CommandPalette.tsx`,
  // `Search.tsx`, `SettingsPanel.tsx`, `ProjectDiffPanel.tsx`, plus the
  // file-dialog filter name in `GroupAddressCsvButtons.tsx` (and, via
  // `ProjectDiffPanel.tsx`'s own filter, `projectDiff.compareFilterName`).
  // `catalogDiagnostic.*` is the one namespace here that doesn't belong to
  // a single component: it's D4's exception, composing `CreationDiagnostic`
  // sentences in the frontend from the server's structured `kind` instead
  // of translating its ready-made `detail` prose (which stays English by
  // design — see that field's own comment in `api.ts`).
  "logPanel.title": "Session log",
  "logPanel.eyebrow": "Diagnostics",
  "logPanel.severity.error": "Error",
  "logPanel.severity.warning": "Warning",
  "logPanel.severity.info": "Info",
  "logPanel.emptyNoEntries": "No log entries yet.",
  "logPanel.emptyFiltered": "No log entries match the current filters.",
  // Same §66/§67 disclosure as `toast.error.messageIsEnglish`, for this
  // panel's own untranslated fields.
  "logPanel.entryTextIsEnglish": "Message, location and detail are the server's own text, in English.",

  "busMonitor.title": "Bus monitor",
  "busMonitor.eyebrow": "KNXnet/IP · Tunnelling",
  "busMonitor.gatewayLabel": "Gateway address",
  "busMonitor.gatewayLocked":
    "Disconnect the running session before changing the gateway address.",
  "busMonitor.connectNeedsGateway": "Enter a gateway address first.",
  "busMonitor.connect": "Connect",
  "busMonitor.disconnect": "Disconnect",
  "busMonitor.session": "Session {id}",
  "busMonitor.assignedAddress": " — assigned address {address}",
  "busMonitor.closedByGateway": " — closed by gateway",
  "busMonitor.stopSummary.one": "Stopped session {id}: {count} telegram seen, {dropped} dropped.",
  "busMonitor.stopSummary.other": "Stopped session {id}: {count} telegrams seen, {dropped} dropped.",
  "busMonitor.gapNotice.one":
    "{count} telegram could not be kept (buffer capacity or a slow poller) and is missing from this view.",
  "busMonitor.gapNotice.other":
    "{count} telegrams could not be kept (buffer capacity or a slow poller) and are missing from this view.",
  "busMonitor.filterPlaceholder": "Filter by destination or name…",
  "busMonitor.rowTitle": "Click to prefill the send form above with this row's destination",
  "busMonitor.emptyNoTelegrams": "No telegrams yet.",
  "busMonitor.emptyFiltered": "No telegrams match the current filters.",
  "busMonitor.column.seq": "Seq",
  "busMonitor.column.time": "Time",
  "busMonitor.column.source": "Source",
  "busMonitor.column.destination": "Destination",
  "busMonitor.column.service": "Service",
  "busMonitor.column.payload": "Payload",
  "busMonitor.column.decoded": "Decoded",
  // Task 4 (the diagnostic companion). The bus session freezes the
  // project's group-address names and DPTs when it starts and never
  // re-resolves them; these three say so out loud rather than letting a
  // decoded column quietly describe a project that has since changed.
  "busMonitor.contextStale":
    "The project changed after this session started. Decoded values below come from the snapshot taken at connect time, and sending is locked. Reconnect to decode against the current project.",
  "busMonitor.contextUnverified":
    "This window did not start this session, so it cannot confirm that the decoded values match the project open now.",
  "busMonitor.sessionReplaced":
    "The bus session was replaced — now showing session {id}. Rows from the previous session were cleared.",
  "busMonitor.endedElsewhere": "The bus session was ended elsewhere.",

  // `CatalogBrowser.tsx`.
  "catalog.title": "Device catalog",
  "catalog.installing": "Installing product database…",
  "catalog.installLabel": "Install product database",
  "catalog.installReport.status.already": "Already installed",
  "catalog.installReport.status.new": "Installed",
  "catalog.installReport.summary": "{status}: scheme {scheme}, {members}, {unknown}, {conflicts}.",
  "catalog.installReport.membersCount.one": "{count} member",
  "catalog.installReport.membersCount.other": "{count} members",
  "catalog.installReport.unknownCount": "{count} unknown",
  "catalog.installReport.conflictsCount.one": "{count} conflict",
  "catalog.installReport.conflictsCount.other": "{count} conflicts",
  "catalog.allManufacturers": "All manufacturers",
  "catalog.searchPlaceholder": "Search catalog items…",
  "catalog.noMatches": "No matches.",
  "catalog.deviceNamePlaceholder": "Device name",
  "catalog.creating": "Creating…",
  "catalog.create": "Create",
  "catalog.diagnosticsHeading": "Creation diagnostics",
  "catalog.createdWithDiagnostics": "Device created with diagnostics.",
  "catalog.done": "Done",

  // D4 exception — one key per `CreationDiagnostic.kind` (`api.ts`, around
  // line 276), composed from the structured fields instead of the server's
  // own ready-made `detail` string. An unknown future `kind` still falls
  // back to `detail` verbatim in `CatalogBrowser.tsx` — never to a blank
  // line — so this list widening is the only maintenance this exception
  // asks for.
  "catalogDiagnostic.programlessProduct":
    "Product {catalogItemId} has no application program; it was created without communication objects.",
  "catalogDiagnostic.ambiguousDpt": "No DPT could be inferred for {refId}; alternatives: {alternatives}.",
  "catalogDiagnostic.comObjectRefMissing":
    "Communication-object reference {refId} is missing from the installed program.",
  "catalogDiagnostic.programRefMissing": "The installed application program reference {programRef} is missing.",
  "catalogDiagnostic.dynamicOrModuleNotEvaluated":
    "Dynamic and module activation was not evaluated for {programId}; only static product data was seeded.",

  // `Search.tsx`.
  "search.overlayLabel": "Search",
  "search.placeholder": "Search devices, group addresses, building parts…",
  "search.noMatches": "No matches.",
  "search.kind.device": "Devices",
  "search.kind.groupAddress": "Group addresses",
  "search.kind.buildingPart": "Building parts",

  // `CommandPalette.tsx`. `command.shortcutHint.*` translates the palette's
  // `shortcutHint` badges (`commandRegistry.ts` keeps the literal
  // "Ctrl+Z"-style strings — a fixed, non-localized lookup key, same
  // discriminant-vs-label split as `BUILDING_PART_KIND_KEYS` in
  // `Inspector.tsx`) so a German user doesn't see "Strg+K" on the toolbar
  // and "Ctrl+K" on the same palette row.
  "command.overlayLabel": "Command palette",
  "command.placeholder": "Type a command…",
  "command.noMatches": "No matching commands.",
  "command.shortcutHint.undo": "Ctrl+Z",
  "command.shortcutHint.redo": "Ctrl+Shift+Z",
  "command.shortcutHint.search": "Ctrl+K",

  // `SettingsPanel.tsx`. `settings.uiLanguage`/`language.*` already existed
  // (task 2); the rest of the panel's field labels are this task's.
  "settings.title": "Settings",
  "settings.theme": "Theme",
  "settings.motionStyle": "Motion style",
  "settings.motionLevel": "Motion level",
  "settings.productDataLanguage": "Product data language",
  "settings.noProductDatabase": "No product database installed",
  "settings.packageDefault": "Package default",
  "settings.productLanguageOption.one": "{language} ({count} string)",
  "settings.productLanguageOption.other": "{language} ({count} strings)",

  // `SettingsPanel.tsx`'s language-pack manager (T25 task 7) — the only
  // place `languagePack.ts` (task 6) becomes visible: import a pack, export
  // the English catalogue as a translate-me template, export/remove an
  // installed one, and the report a user sees after each import. The
  // report's own wording is always picked between these keys, never
  // assembled from the pack's data; a pack's `name` and the reasons/keys
  // `languagePack.ts` hands back are interpolated as `{placeholders}`,
  // never used to choose a key.
  "languagePack.importLabel": "Import a language pack…",
  "languagePack.exportTemplateButton": "Export English template…",
  "languagePack.exportTemplateHint":
    'A ready-to-translate copy of every string this build knows, so a pack can be written without reading source code. Its tag ("en") must be changed before the result is imported as its own language — as exported, it is shadowed by the built-in English catalogue and would never be used.',
  "languagePack.installedTitle": "Installed language packs",
  "languagePack.noPacksInstalled": "No language packs installed.",
  "languagePack.exportPackButton": "Export…",
  "languagePack.exportPackAriaLabel": "Export {name}",
  "languagePack.removePackButton": "Remove",
  "languagePack.removePackAriaLabel": "Remove {name}",
  "languagePack.importReport.invalidJson": "That file isn't valid JSON.",
  "languagePack.importReport.rejected": "Import rejected: {reason}",
  "languagePack.importReport.grandfatheredHint":
    'Tip: "{oldTag}" is an old-style (grandfathered) tag; its modern registered form is "{modernTag}".',
  "languagePack.importReport.heading": '"{name}" imported.',
  "languagePack.importReport.appliedKeys.one": "{count} string translated.",
  "languagePack.importReport.appliedKeys.other": "{count} strings translated.",
  "languagePack.importReport.missingKeys.one": "{count} string not translated — falls back to English.",
  "languagePack.importReport.missingKeys.other": "{count} strings not translated — fall back to English.",
  "languagePack.importReport.missingKeysSample": "For example: {keys}.",
  "languagePack.importReport.unknownKeys.one":
    "{count} key this build doesn't recognise — the pack may target a different version:",
  "languagePack.importReport.unknownKeys.other":
    "{count} keys this build doesn't recognise — the pack may target a different version:",
  "languagePack.importReport.pluralSupported": "Plural forms are supported for this language.",
  "languagePack.importReport.pluralUnsupported":
    "Plural forms have no data for this language on this system; plural text always uses the general form.",
  "languagePack.importReport.shadowedByBuiltIn":
    'This pack\'s tag ("{tag}") matches a built-in language and will never be used — edit "tag" before activating or sharing it.',

  // §67's fix: one key per `LanguagePackRejectionReason.kind`
  // (`languagePack.ts`), so `SettingsPanel.tsx`'s `describeRejectionReason`
  // can compose the "Import rejected: …" sentence above entirely in the
  // active UI language — the rejection reason used to be a raw English
  // string dropped into an otherwise-translated sentence. Deliberately
  // one key per validation rule rather than one generic "invalid pack"
  // key, so a translator (and a user reading their own mistake) gets the
  // specific field name every time, same as the English original did.
  "languagePack.rejection.notObject": "A language pack must be a JSON object.",
  "languagePack.rejection.formatVersionMissing": '"formatVersion" is required and must be a number.',
  "languagePack.rejection.tagMissing": '"tag" is required and must be a non-empty string.',
  "languagePack.rejection.tagMalformed":
    '"tag" ("{tag}") is not a well-formed BCP 47 tag, e.g. "nl-NL", "tlh" (Klingon), "bar" (Bavarian), or "art-x-sindarin" (a private-use tag for anything unregistered).',
  "languagePack.rejection.nameMissing": '"name" is required and must be a non-empty string.',
  "languagePack.rejection.messagesMissing":
    '"messages" is required and must be an object mapping keys to strings.',
  "languagePack.rejection.messageValueNotString": '"messages.{key}" must be a string, got {valueType}.',
  "languagePack.rejection.englishNameNotString": '"englishName" must be a string when present.',
  "languagePack.rejection.basedOnNotString": '"basedOn" must be a string when present.',
  "languagePack.rejection.packVersionNotString": '"packVersion" must be a string when present.',
  "languagePack.rejection.pluralCategoriesInvalid":
    '"pluralCategories" must be an array of strings when present.',
  "languagePack.rejection.storageFailure":
    "Could not save the change: the browser's storage rejected the write ({detail}).",

  // `ProjectDiffPanel.tsx`. `projectDiff.entityStatus.*` doubles as both the
  // per-table count word ("1 {status}") and the whole-installation status
  // word (`projectDiff.installationStatusLine`'s `{status}`) — the same
  // vocabulary either way, so one namespace instead of two.
  "projectDiff.compareButton": "Compare with…",
  "projectDiff.compareFilterName": "KNXBench project",
  "projectDiff.title": "Comparison result",
  "projectDiff.noDifferences": "No differences found.",
  "projectDiff.close": "Close",
  "projectDiff.projectInfoChanged.one": "Project info: {count} field changed",
  "projectDiff.projectInfoChanged.other": "Project info: {count} fields changed",
  "projectDiff.installationInfoChanged.one": "Installation info: {count} field changed",
  "projectDiff.installationInfoChanged.other": "Installation info: {count} fields changed",
  "projectDiff.installationPrefix": "Installation {id}: ",
  "projectDiff.installationStatusLine": "installation {status}",
  "projectDiff.entityStatus.added": "added",
  "projectDiff.entityStatus.removed": "removed",
  "projectDiff.entityStatus.changed": "changed",
  "projectDiff.entityStatus.ambiguous": "ambiguous",
  "projectDiff.entity.areas": "Areas",
  "projectDiff.entity.lines": "Lines",
  "projectDiff.entity.devices": "Devices",
  "projectDiff.entity.groupRanges": "Group ranges",
  "projectDiff.entity.groupAddresses": "Group addresses",
  "projectDiff.entity.buildings": "Buildings",

  // `GroupAddressCsvButtons.tsx` — controller ruling after the task 3
  // review: only the native file-dialog filter name is in scope here,
  // nothing else in that file.
  "groupAddressCsv.filterName": "Group-address CSV",

  // `App.tsx` (task 5, controller correction): the addendum that sent
  // `filePicker.ts`/`GroupAddressCsvButtons.tsx` into scope mislocated
  // these two strings — they actually live here, as a module-level const
  // (`pickProject`'s own inline filter) and as `EXPORT_FILTER`
  // (`exportProject`). Both are resolved at call time now, same reasoning
  // as `commandRegistry.ts`'s `COMMANDS` in task 3: a module-level
  // `t()` call would freeze the first language forever.
  "app.filterName.etsProject": "ETS project",
  "app.filterName.knxDesktopProject": "knx-desktop project",

  // `DocumentationExportButton.tsx` (task 5, controller correction) —
  // same mislocated string, same fix: the filter name moves from a
  // module-level const into the component, resolved via `t()`.
  "documentationExport.filterName": "HTML document",

  // Task 5 review, round 2: three files the brief never named at all
  // (`BulkActionToolbar.tsx`, `BusComposeForm.tsx`, `FsPicker.tsx`) plus
  // the rest of `GroupAddressCsvButtons.tsx` beyond its filter name, which
  // an earlier controller ruling had put out of scope for the wrong
  // reason. All four were entirely, everyday-reachable English until now.

  // `BulkActionToolbar.tsx`. `count === 1 ? "" : "s"` became a real plural
  // pair — that inline ternary is exactly the case `.one`/`.other` exists
  // for. `bulkAction.moveToLine`/`bulkAction.moveToBuildingPart` reuse the
  // `explorer.areaLabel`/`explorer.lineLabel` keys already seeded for the
  // option list's area/line labels — same tree, same vocabulary.
  "bulkAction.deviceLabel.one": "{count} device selected",
  "bulkAction.deviceLabel.other": "{count} devices selected",
  "bulkAction.groupAddressLabel.one": "{count} group address selected",
  "bulkAction.groupAddressLabel.other": "{count} group addresses selected",
  "bulkAction.delete": "Delete",
  "bulkAction.moveToLine": "Move to line…",
  "bulkAction.unassigned": "(unassigned)",
  "bulkAction.moveToBuildingPart": "Move to building part…",
  "bulkAction.none": "(none)",
  "bulkAction.dismissSelection": "Dismiss selection",

  // `BusComposeForm.tsx` — `BusMonitorPanel.tsx`'s compose/send sibling.
  // `SESSION_CLOSED_MESSAGE`/`NO_DPT_RESOLVED_MESSAGE`/
  // `conflictingDptsMessage()` used to live outside any component at all;
  // all three are now `t()` calls made from inside the component body.
  "busCompose.heading": "Send a value",
  "busCompose.noProjectHint":
    "No project open — no DPT resolves automatically here; type one explicitly.",
  "busCompose.sessionClosedMessage": "This session is closed — sending is disabled.",
  "busCompose.noDptResolvedMessage": "No DPT resolved for this group address — enter one explicitly.",
  "busCompose.conflictingDptsMessage":
    "Conflicting DPTs for this group address: {names} — enter one explicitly.",
  "busCompose.destinationLabel": "Destination",
  "busCompose.dptLabel": "DPT",
  "busCompose.valueLabel": "Value",
  "busCompose.send": "Send",
  "busCompose.liveAction": "Sends to the connected bus. Project Undo cannot reverse this action.",
  "busCompose.sent": "Sent {service}: {payload}",
  "busCompose.contextStaleMessage":
    "The project changed after this bus session started — the DPT would be resolved against the old snapshot, so sending is locked. Reconnect first.",

  // `DiagnosticsCompanion.tsx` and the button in `App.tsx` that opens it.
  "companion.open": "Diagnostics window",
  "companion.title": "Diagnostics",
  "companion.eyebrow": "Companion window · read-only",
  "companion.readOnly":
    "Read-only companion. The project is edited in the main window only; there is no Undo here, and this window shares the main window's bus session rather than opening its own.",
  "companion.backToMain": "Back to main window",
  "companion.noMainWindow": "No main window to return to — this one was opened on its own.",
  "companion.blocked":
    "The diagnostics window was blocked. Allow pop-ups for this page, or keep using the monitor here.",
  "companion.failed":
    "The diagnostics window could not be opened. The monitor and the log stay available here.",

  // `FsPicker.tsx` — the plain-web-build fallback dialog `filePicker.ts`
  // routes to when not running under Tauri. Had no `useTranslate()` call
  // anywhere; this is the file dialog every non-Tauri user actually sees.
  "fsPicker.open": "Open",
  "fsPicker.saveAs": "Save as",
  "fsPicker.filenamePlaceholder": "filename",
  "fsPicker.upload": "Upload…",
  "fsPicker.save": "Save",
  "fsPicker.cancel": "Cancel",

  // `GroupAddressCsvButtons.tsx`, everything past its filter name: the two
  // button labels, and the export/import toast summaries — the latter's
  // `count === 1 ? "" : "s"` ternaries became real `.one`/`.other` pairs,
  // same as `BulkActionToolbar.tsx` above. `importSummarySeeLog` is one
  // fragment among several composed together at the call site (see
  // `importSummary()` in `GroupAddressCsvButtons.tsx`) rather than one
  // full sentence per combination of warnings/ignored-columns, since the
  // component can carry both counts independently and enumerating every
  // combination would multiply the catalogue for no translation benefit —
  // the join punctuation itself (", ") needs no localization.
  "groupAddressCsv.exportButton": "Export group addresses (CSV)…",
  "groupAddressCsv.importButton": "Import group addresses (CSV)…",
  "groupAddressCsv.exportSummaryNone": "Group addresses exported to CSV, no warnings.",
  "groupAddressCsv.exportSummaryWithWarnings.one":
    "Group addresses exported to CSV, {count} warning — see Log.",
  "groupAddressCsv.exportSummaryWithWarnings.other":
    "Group addresses exported to CSV, {count} warnings — see Log.",
  "groupAddressCsv.importSummaryBase":
    "Group addresses imported from CSV: {created} created, {updated} updated, {unchanged} unchanged",
  "groupAddressCsv.importSummaryWarnings.one": "{count} warning",
  "groupAddressCsv.importSummaryWarnings.other": "{count} warnings",
  "groupAddressCsv.importSummaryIgnoredColumns.one": "{count} column ignored",
  "groupAddressCsv.importSummaryIgnoredColumns.other": "{count} columns ignored",
  "groupAddressCsv.importSummarySeeLog": "— see Log.",
  "appearance.accent": "Accent color",
  "appearance.accentUnavailable": "This theme keeps its own accent; the accent setting has no effect here.",
  "appearance.density": "Density",
  "appearance.violet": "Violet",
  "appearance.mint": "Mint",
  "appearance.blue": "Blue",
  "appearance.amber": "Amber",
  "appearance.rose": "Rose",
  "appearance.compact": "Compact",
  "appearance.comfortable": "Comfortable",
  "workbench.overview": "Overview",
  "workbench.buildings": "Buildings",
  "workbench.topology": "Topology",
  "workbench.addresses": "Group addresses",
  "workbench.catalog": "Product catalog",
  "workbench.device": "Device",
  "workbench.unassigned": "Unassigned",
  "workbench.emptyStructure": "No entries yet. Use the project tree to create structure.",
  "workbench.address": "Address",
  "workbench.name": "Name",
  "workbench.file": "File",
  "workbench.navigation": "Navigation",
  "workbench.properties": "Properties",
  "workbench.welcome": "Your KNX workspace",
  "workbench.openHint": "Open a project to explore buildings, topology and group addresses.",
  "workbench.noSelection": "Select an item to inspect or edit its properties.",
  "workbench.importNotices": "Import: {errors} errors · {warnings} warnings",
  "workbench.parameters": "Parameters",
  "workbench.telegram": "Telegram",
  "workbench.selectTelegram": "Select a telegram to inspect its received data.",
  "workbench.noDevices": "No devices assigned to this building part.",

  // The group-address table (stage 4). A group address has no DPT of its
  // own in the KNX model — these labels name what the linked communication
  // objects state, which is why an empty DPT reads "none stated" rather
  // than "unknown", and why a disagreement is labelled a conflict instead
  // of being silently resolved.
  "addressTable.filterLabel": "Filter group addresses",
  "addressTable.filterPlaceholder": "Filter addresses…",
  "addressTable.selectColumn": "Select",
  "addressTable.range": "Range",
  "addressTable.dpt": "DPT",
  "addressTable.links": "Links",
  "addressTable.noDpt": "none stated",
  "addressTable.dptConflict": "conflicting",
  "addressTable.noRange": "(no range)",
  "addressTable.noLinks": "none",
  "addressTable.linkCounts": "{senders} sending · {receivers} receiving",
  "addressTable.linkTotal.one": "{count} link",
  "addressTable.linkTotal.other": "{count} links",
  "addressTable.noMatches": "No group address matches this filter.",
  "addressTable.linksFor": "Links · {address}",
  "addressTable.noLinksYet": "No communication object is linked to this address.",
  "addressTable.participant": "Participant",
  "addressTable.function": "Function",
  "addressTable.direction": "Direction",
  "addressTable.unlinkFrom": "Unlink {object} from {address}",
  // Only reachable if the project links an object whose device is missing —
  // the link is still shown rather than dropped (see `GroupAddressLinkNode`).
  "addressTable.unknownDevice": "Unknown device #{id}",
  "addressTable.unnamedObject": "Unnamed object",

  // T16, the device product identity block in the workspace's third tab
  // (`DeviceIdentity` in `Inspector.tsx`). The four `resolution.*` words
  // and their four `explain.*` sentences are deliberately not
  // interchangeable: "no database is loaded" and "the database does not
  // have it" are different facts about the installation, and collapsing
  // them into one "unknown" is precisely the dishonesty this block exists
  // to avoid. `explain.resolved` has no entry — the catalogue speaks for
  // itself — but `explain.resolvedWithoutCatalog` does, for the shape the
  // generated type permits and the server never sends.
  "deviceIdentity.title": "Product identity",
  // The workspace tab. Deliberately its own key: the tab is a short noun in
  // a strip of three, while `deviceIdentity.title` labels the section itself
  // and `deviceIdentity.more` labels the disclosure inside it.
  "deviceIdentity.tab": "Product data",
  "deviceIdentity.productRef": "Product reference",
  "deviceIdentity.programRef": "Application program reference",
  "deviceIdentity.refNotStated": "not stated in the project",
  "deviceIdentity.resolution.resolved": "From the product database",
  "deviceIdentity.resolution.noDatabase": "No product database",
  "deviceIdentity.resolution.notInDatabase": "Not in the product database",
  "deviceIdentity.resolution.noReference": "No product reference",
  // For a resolution string this build has never heard of — a frontend
  // talking to a newer server. Saying so is the only honest option: borrowing
  // another variant's wording would deny references the user can see.
  "deviceIdentity.resolution.unrecognised": "State not recognised",
  "deviceIdentity.explain.unrecognised":
    "The server reported a product-resolution state this build does not recognise, so this panel cannot say whether the references above were matched to a product. The references are shown exactly as the project states them.",
  "deviceIdentity.explain.noDatabase":
    "No product database is loaded here, so what the project states above cannot be matched to a product. Install the manufacturer's product package to see product, hardware and application program details.",
  "deviceIdentity.explain.notInDatabase":
    "A product database is loaded and does not contain what the project states above. The manufacturer's catalogue for this product is not installed here.",
  "deviceIdentity.explain.noReference":
    "This device states neither a product reference nor an application program reference — it was created without one, or came from an import that carried none.",
  "deviceIdentity.explain.resolvedWithoutCatalog":
    "The product database reported a match but returned no details for it.",
  "deviceIdentity.more": "More product data",
  "deviceIdentity.group.product": "Product",
  "deviceIdentity.group.hardware": "Hardware",
  "deviceIdentity.group.application": "Application program",
  "deviceIdentity.manufacturer": "Manufacturer",
  "deviceIdentity.manufacturerId": "Manufacturer ID",
  "deviceIdentity.productText": "Product name",
  "deviceIdentity.orderNumber": "Order number",
  "deviceIdentity.catalogItemName": "Catalogue item",
  "deviceIdentity.catalogItemNumber": "Catalogue item number",
  "deviceIdentity.hardwareName": "Hardware name",
  "deviceIdentity.hardwareVersion": "Hardware version",
  // The serial number the manufacturer package states for this *hardware
  // type*, not the serial of the unit on the wall — that one cannot be read
  // at all (KNOWN_LIMITATIONS.md §73), so the label must not promise it.
  "deviceIdentity.hardwareSerial": "Hardware serial number",
  "deviceIdentity.applicationName": "Program name",
  "deviceIdentity.applicationNumber": "Program number",
  "deviceIdentity.applicationVersion": "Program version",
  "deviceIdentity.applicationProgramId": "Program ID",
  "deviceIdentity.maskVersion": "Mask version",
  "deviceIdentity.groupEmpty": "The product database holds no values here.",
  "deviceIdentity.omitted.one": "{count} further field is omitted: the product database has no value for it.",
  "deviceIdentity.omitted.other": "{count} further fields are omitted: the product database has no value for them.",

  // The from-scratch project launcher (the only way to a project that
  // never came from a file). `defaultName`/`defaultInstallation` are
  // seeded into the dialog's fields rather than sent as an empty request:
  // `domain.rs`'s `new_project_impl` deliberately refuses to invent a
  // name, on the grounds that a localized default belongs to this
  // catalogue — so here it is.
  "newProject.title": "New project",
  "newProject.intro": "An empty project with one installation. Nothing is written to disk until you save it.",
  "newProject.name": "Project name",
  "newProject.defaultName": "Untitled project",
  "newProject.installation": "Installation name",
  "newProject.defaultInstallation": "Installation 1",
  "newProject.language": "Project language",
  "newProject.languageHint": "The language tag your project texts are stored under, e.g. en or de-DE. Not the language of this interface.",
  "newProject.style": "Group address style",
  "newProject.styleHint": "Pick the one you think in; the project properties can restyle it later.",
  "newProject.style.Free": "Free (0–65535)",
  "newProject.style.TwoLevel": "Two level (main/sub)",
  "newProject.style.ThreeLevel": "Three level (main/middle/sub)",
  "newProject.nameRequired": "A project needs a name.",
  "newProject.languageInvalid": "Not a well-formed language tag. Try en, de, or de-DE.",
  "newProject.create": "Create project",
  "newProject.creating": "Creating…",
  "newProject.cancel": "Cancel",
  "newProject.conflictTitle": "The open project has unsaved changes",
  "newProject.conflictBody": "Creating a new project throws those changes away, and no undo brings them back. Keep editing to save them first, or discard them deliberately.",
  "newProject.conflictDiscard": "Discard changes and create",
  "newProject.conflictKeep": "Keep editing",
} as const;

export type Messages = typeof messages;
export type MessageKey = keyof Messages;
