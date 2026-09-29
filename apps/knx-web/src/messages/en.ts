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
// is the same `BuildingPartType` label shown both in
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
  "toolbar.exportProject": "Export project…",
  "toolbar.undo": "Undo",
  "toolbar.redo": "Redo",
  "toolbar.search": "Search… (Ctrl+K)",
  "toolbar.log": "Log",
  "toolbar.settings": "Settings",
  "toolbar.busMonitor": "Bus monitor",
  "toolbar.commands": "Commands… (Ctrl+Shift+P)",
  // ISSUE-04's status-bar text. `never` covers a session that has not
  // saved yet; the timestamped form uses the browser's own locale
  // formatting (`Intl.DateTimeFormat`), not a hardcoded pattern.
  "statusBar.lastSaved": "Last saved: {time}",
  "statusBar.neverSaved": "Not saved yet",
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
  "documentationExport.dialogTitle": "Project documentation",
  "documentationExport.sectionsLegend": "Sections",
  "documentationExport.alwaysIncluded": "Header, contents and limits are always included.",
  "documentationExport.section.summary": "Summary",
  "documentationExport.section.topology": "Topology",
  "documentationExport.section.buildings": "Buildings",
  "documentationExport.section.groupAddresses": "Group addresses",
  "documentationExport.section.devices": "Devices",
  "documentationExport.previewTitle": "Documentation preview",
  "documentationExport.previewLoading": "Loading preview…",
  "documentationExport.previewError": "Preview failed: {message}",
  "documentationExport.warningsHeading": "Warnings ({count})",
  "documentationExport.warningsNone": "No warnings.",
  "documentationExport.print": "Print…",
  "documentationExport.export": "Export…",
  "documentationExport.close": "Close",

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
  "inspector.comFlag.readOnInit": "Read on init",
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
  "buildingPartKind.stairway": "Stairway",
  "buildingPartKind.roomPart": "Room Part",
  "buildingPartKind.area": "Area",
  "buildingPartKind.ground": "Ground",
  "buildingPartKind.segment": "Segment",

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
  "parameters.diagnostic.refBelowSkippedNode":
    "A parameter, object or module inside a skipped element was not evaluated.",
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
  "dragDrop.movedToLine": "{device} moved to line {line}.",
  "dragDrop.movedToBuildingPart": "{device} moved to {buildingPart}.",
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
  "logPanel.loading": "Loading session log…",
  "logPanel.emptyFiltered": "No log entries match the current filters.",
  "logPanel.search": "Search session log",
  "logPanel.clear": "Clear search",
  "logPanel.count": "{shown} of {total} entries",
  "logPanel.exportScope": "Export scope",
  "logPanel.exportFiltered": "Matching entries",
  "logPanel.exportAll": "All retained entries",
  "logPanel.exportJson": "Export log (JSON)…",
  "logPanel.unknownCount": "unknown",
  "logPanel.retention": "Only this server session's newest 1000 entries are retained; {count} entries dropped. This is not a lifetime audit. Export may include addresses and names.",
  // Same §66/§67 disclosure as `toast.error.messageIsEnglish`, for this
  // panel's own untranslated fields.
  "logPanel.entryTextIsEnglish": "Message, location and detail are the server's own text, in English.",

  "lineScan.eyebrow": "KNXnet/IP · Read-only diagnostics",
  "lineScan.title": "Line scan",
  "lineScan.readOnly": "Reads occupancy evidence only. It never commissions or writes devices.",
  "lineScan.gateway": "Gateway address",
  "lineScan.area": "Area",
  "lineScan.line": "Line",
  "lineScan.firstDevice": "First device",
  "lineScan.lastDevice": "Last device",
  "lineScan.timeout": "Response timeout (ms)",
  "lineScan.pause": "Pause between probes (ms)",
  "lineScan.estimate": "Estimate cost",
  "lineScan.start": "Start read-only scan",
  "lineScan.cancel": "Cancel scan",
  "lineScan.exclusions": "Protected exclusions",
  "lineScan.noExclusions": "No configured exclusions. Add them in settings before scanning.",
  "lineScan.protected": "Protected",
  "lineScan.remove": "Remove exclusion",
  "lineScan.confirmRemoval": "Confirm removal",
  "lineScan.estimateTitle": "Cost preview",
  "lineScan.candidates.one": "{count} candidate address",
  "lineScan.candidates.other": "{count} candidate addresses",
  "lineScan.basis.one": "{timeout} ms timeout × {confirmations} confirmation; {pause} ms pause",
  "lineScan.basis.other": "{timeout} ms timeout × {confirmations} confirmations; {pause} ms pause",
  "lineScan.worstCase": "{duration} response/pacing budget; transport overhead excluded",
  "lineScan.status.running": "Running",
  "lineScan.status.completed": "Completed",
  "lineScan.status.cancelled": "Cancelled",
  "lineScan.status.failed": "Failed",
  "lineScan.address": "Address",
  "lineScan.outcome": "Outcome",
  "lineScan.outcome.occupiedNoMask": "Occupied · descriptor answered, mask not retained",
  "lineScan.outcome.occupiedMask": "Occupied · mask {mask}",
  "lineScan.outcome.occupiedBusy": "Occupied · busy",
  "lineScan.outcome.occupiedSilent": "Occupied · silent",
  "lineScan.outcome.vacant": "Vacant · no answer in window",
  "lineScan.outcome.indeterminate": "Indeterminate · frames lost",
  "lineScan.outcome.selfAddress": "Scanner address · not probed",
  "lineScan.exclusionAddress": "Individual address to exclude",
  "lineScan.addExclusion": "Add protected exclusion",
  "lineScan.invalidExclusion": "Enter a complete dotted individual address (0..15.0..15.0..255).",
  "lineScan.duplicateExclusion": "That individual address is already excluded.",
  "lineScan.invalidLegacyExclusion": "Invalid legacy entry — remove it before scanning",
  "lineScan.invalidExclusionsBlock": "Remove invalid or duplicate exclusions before estimating or starting a scan.",
  "lineScan.diagnosticsTabs": "Bus diagnostics",
  "lineScan.reconciliation.title": "Project reconciliation",
  "lineScan.reconciliation.loading": "Comparing scan evidence with the open project…",
  "lineScan.reconciliation.projectRequired": "Open a project to compare scan evidence.",
  "lineScan.reconciliation.unexpected": "Bus answered, not in project",
  "lineScan.reconciliation.unexpectedHelp": "A response is evidence at this address; it does not prove that the device belongs in this project.",
  "lineScan.reconciliation.missing": "In project, no answer",
  "lineScan.reconciliation.missingHelp": "No response in this scan does not prove that the device is absent.",
  "lineScan.reconciliation.excluded": "In project, not examined",
  "lineScan.reconciliation.excludedHelp": "Excluded addresses were not probed and are never actionable here.",
  "lineScan.reconciliation.none": "None",
  "lineScan.reconciliation.apply": "Apply selected changes",
  "busMonitor.title": "Bus monitor",
  "busMonitor.eyebrow": "KNXnet/IP · Tunnelling",
  "busMonitor.gatewayLabel": "Gateway address",
  "busMonitor.gatewayHost": "Gateway host",
  "busMonitor.gatewayPort": "Gateway port",
  "busMonitor.invalidHost": "Enter only a numeric IPv4 host; put the port in the separate port field. Hostnames and IPv6 are not supported by this tunnel yet.",
  "busMonitor.invalidPort": "Gateway port must be 1–65535.",
  "busMonitor.gatewayLocked":
    "Disconnect the running session before changing the gateway address.",
  "busMonitor.connectNeedsGateway": "Enter a gateway address first.",
  "busMonitor.connect": "Connect",
  "busMonitor.disconnect": "Disconnect",
  "busMonitor.pause": "Pause",
  "busMonitor.resume": "Resume",
  "busMonitor.pausedNotice": "Paused. The server's finite buffer may drop older telegrams; Resume to fetch what remains.",
  "busMonitor.exportCapture": "Export capture",
  "busMonitor.retainedCapture": "Retained capture from a closed session; connect again to start a new capture.",
  "busMonitor.exportNote": "Export contains addresses and payloads. It includes only retained rows; keep the JSON private.",
  "busMonitor.exporting": "Opening save dialog…",
  "busMonitor.rowDetailTitle": "Click to inspect this captured telegram",
  "busMonitor.session": "Session {id}",
  "busMonitor.assignedAddress": " — assigned address {address}",
  "busMonitor.closedByGateway": " — closed by gateway",
  "busMonitor.stopSummary.one": "Stopped session {id}: {count} telegram seen, {dropped} dropped.",
  "busMonitor.stopSummary.other": "Stopped session {id}: {count} telegrams seen, {dropped} dropped.",
  "busMonitor.gapNotice.one":
    "{count} telegram could not be kept (buffer capacity or a slow poller) and is missing from this view.",
  "busMonitor.gapNotice.other":
    "{count} telegrams could not be kept (buffer capacity or a slow poller) and are missing from this view.",
  "busMonitor.capturePruned.one": "{count} older captured row was removed from this view's {capacity}-row limit; the export reports the loss.",
  "busMonitor.capturePruned.other": "{count} older captured rows were removed from this view's {capacity}-row limit; the export reports the loss.",
  "busMonitor.stats.title": "Statistics · retained rows only",
  "busMonitor.stats.scope": "Based on {count} retained telegrams (at most {capacity}); earlier and lost telegrams are excluded.",
  "busMonitor.stats.services": "Services",
  "busMonitor.stats.destinations": "Busiest group addresses",
  "busMonitor.stats.sources": "Talkative sources",
  "busMonitor.filterPlaceholder": "Filter by destination or name…",
  "busMonitor.filterLabel": "Filter by destination or name",
  "busMonitor.tableScrollLabel": "Telegram table; scroll horizontally",
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
  "busMonitor.decode.unresolved": "No DPT assigned",
  "busMonitor.decode.conflict": "Conflicting DPTs",
  "busMonitor.decode.unsupported": "Unsupported DPT",
  "busMonitor.decode.failed": "Decode failed",
  "busMonitor.decode.unknown": "Decode error (reason unknown)",
  // The diagnostic companion compares the project with the bus session's
  // last confirmed whole-context publication; these messages expose a
  // mismatch instead of quietly presenting stale decoding as current.
  "busMonitor.contextStale":
    "The project changed after this session's last confirmed context publication. Decoded values below use that earlier context, and sending is locked. Reconnect to decode against the current project.",
  "busMonitor.contextUnverified":
    "This window did not start this session, so it cannot confirm that the decoded values match the project open now.",
  "busMonitor.sessionReplaced":
    "The bus session was replaced — now showing session {id}. Rows from the previous session were cleared.",
  "busMonitor.endedElsewhere": "The bus session was ended elsewhere.",

  // T25 — the interface search (`busDiscovery.ts`, and the cluster next to
  // the gateway field in `BusMonitorPanel.tsx`). Finding nothing is an
  // ordinary outcome and reads as one: no alert role, no error styling,
  // and the gateway field still takes a typed address either way.
  // `busDiscovery.emptyHint` carries the same two facts the CLI's
  // `DISCOVER_EMPTY_HINT` does (`apps/knx-cli/src/main.rs`) — multicast
  // has to reach this network segment, and a container without host
  // networking is the usual reason it does not — plus the measured
  // host-firewall cause (RESEARCH §20.1).
  "busDiscovery.search": "Search",
  "busDiscovery.searching": "Searching…",
  "busDiscovery.searchLabel": "Search for KNX-compatible IP interfaces",
  "busDiscovery.resultCount.one": "{count} interface answered.",
  "busDiscovery.resultCount.other": "{count} interfaces answered.",
  "busDiscovery.resultsCaption": "Select an interface to fill in the gateway address.",
  "busDiscovery.individualAddressLabel": "Interface address",
  "busDiscovery.tunnelling": "Tunnelling",
  "busDiscovery.empty": "No interfaces answered.",
  "busDiscovery.emptyHint":
    "The search reaches only as far as IP multicast does on this network segment. An empty result can mean no interface answered, or that the search request never left this machine — running inside a container without host networking is a common cause. A firewall on this computer can also drop the answers, which come back as unicast from UDP port 3671: allow incoming UDP from source port 3671 on the local network. Entering the address by hand still works.",
  "busDiscovery.failed":
    "The search could not be run. Entering the gateway address by hand still works.",

  // `CatalogBrowser.tsx`.
  "catalog.title": "Device catalog",
  "catalog.close": "Close catalog",
  "catalog.installing": "Installing product database…",
  "catalog.installLabel": "Install product database",
  "catalog.installReport.status.already": "Already installed",
  "catalog.installReport.status.new": "Installed",
  "catalog.installReport.summary": "{status}: scheme {scheme}, {members}, {unknown}, {conflicts}.",
  "catalog.installReport.heading": "Install report",
  "catalog.installReport.factsMeasured": "Measured install facts",
  "catalog.installReport.category.archiveMember": "Archive member",
  "catalog.installReport.category.product": "Product",
  "catalog.installReport.category.applicationProgram": "Application program",
  "catalog.installReport.category.parameter": "Parameter",
  "catalog.installReport.category.communicationObject": "Communication object",
  "catalog.installReport.category.dynamicNode": "Dynamic node",
  "catalog.installReport.category.module": "Module",
  "catalog.installReport.category.baggageIndex": "Baggage index",
  "catalog.installReport.category.baggage": "Baggage",
  "catalog.installReport.category.unknownConstruct": "Unknown construct",
  "catalog.installReport.category.masterSection": "Master section",
  "catalog.installReport.category.masterSubtree": "Master subtree",
  "catalog.installReport.category.datapointType": "Datapoint type",
  "catalog.installReport.disposition.read": "Read",
  "catalog.installReport.disposition.stored": "Stored",
  "catalog.installReport.disposition.deduplicated": "Deduplicated",
  "catalog.installReport.disposition.retainedButUninterpreted": "Retained but uninterpreted",
  "catalog.installReport.disposition.unsupported": "Unsupported",
  "catalog.installReport.disposition.dropped": "Dropped",
  "catalog.installReport.unknownKind.element": "Element",
  "catalog.installReport.unknownKind.attribute": "Attribute",
  "catalog.installReport.diagnosticKind.unsupportedMasterSection": "Unsupported master section",
  "catalog.installReport.diagnosticKind.unsupportedMasterSubtree": "Uninterpreted master subtree",
  "catalog.installReport.diagnosticKind.unresolvedBaggageDeclaration": "Unresolved baggage declaration",
  "catalog.installReport.diagnosticKind.undeclaredBaggagePayload": "Undeclared baggage file",
  "catalog.installReport.diagnostic.unsupportedMasterSection": "A master-data section was retained but not interpreted.",
  "catalog.installReport.diagnostic.unsupportedMasterSubtree":
    "Part of a supported master-data section was retained but not interpreted.",
  "catalog.installReport.diagnostic.unresolvedBaggageDeclaration":
    "A baggage declaration names no file in the package.",
  "catalog.installReport.diagnostic.undeclaredBaggagePayload":
    "A baggage file was retained, but no declaration names it.",

  "catalog.installReport.factsUnavailable": "Install facts unavailable for this historical install",
  "catalog.installReport.countsHeading": "Encounter counts",
  "catalog.installReport.countRow": "{category} / {disposition}: {count}",
  "catalog.installReport.unknownHeading": "Unknown constructs",
  "catalog.installReport.unknownSummary": "{distinct} distinct, {occurrences} occurrences",
  "catalog.installReport.unknownRow": "{kind} {name} at {xpath} ({occurrences})",
  "catalog.installReport.diagnosticsHeading": "Unsupported diagnostics",
  "catalog.installReport.diagnosticRow": "{kind}: {detail} ({occurrences}) — {archivePath} {xmlPath}",
  "catalog.installReport.signatureCaution": "Signature data is stored but not verified; no ETS parity or signature verification is claimed.",
  "catalog.installReport.membersCount.one": "{count} member",
  "catalog.installReport.membersCount.other": "{count} members",
  "catalog.installReport.unknownCount": "{count} unknown",
  "catalog.installReport.conflictsCount.one": "{count} conflict",
  "catalog.installReport.conflictsCount.other": "{count} conflicts",
  // §85: a `.signature` member is stored, never verified. Say so where a
  // person actually reads it, not just in a bug tracker.
  "catalog.installReport.unverifiedSignature.one":
    "{count} signature member stored, not verified — this application cannot check it.",
  "catalog.installReport.unverifiedSignature.other":
    "{count} signature members stored, not verified — this application cannot check them.",
  "catalog.allManufacturers": "All manufacturers",
  "catalog.searchPlaceholder": "Search catalog items…",
  "catalog.noMatches": "No matches.",
  "catalog.deviceNamePlaceholder": "Device name",
  "catalog.quantity": "Quantity",
  "catalog.quantityInvalid": "Choose 1–32 devices.",
  "catalog.preview": "Devices to create",
  "catalog.addressUnassigned": "Physical addresses remain unassigned; assign them after creation.",
  "catalog.targetLine": "Target line: {lineId}",
  "catalog.noTargetLine": "No line selected; devices will be unassigned.",
  "catalog.createdMany": "Devices created. Review the results below.",
  "catalog.batchUnsupported": "The server did not confirm every requested device. The project was refreshed; inspect it before retrying. No automatic retry was made.",
  "catalog.unconfirmedBatch": "Could not confirm whether the server added the devices. Inspect or reload the project before trying again; no automatic retry was made.",
  "catalog.creationNeedsReview": "Creation needs review. Check the project before continuing.",
  "catalog.itemLabel": "Device {index}: {name}",
  "catalog.noDiagnostics": "No creation warnings.",
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

  // Shared `Overlay.tsx` keyboard resize control.
  "overlay.resizeHandle": "Resize dialog with arrow keys",

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
  "settings.section.appearance": "Appearance",
  "settings.section.languageData": "Language & data",
  "settings.section.busDiagnostics": "Bus & diagnostics",
  "settings.section.autosave": "Autosave",
  "settings.autosaveEnabled": "Autosave",
  "settings.autosaveIntervalMinutes": "Autosave interval (minutes)",
  "settings.preferredGateway": "Preferred KNXnet/IP gateway",
  "settings.preferredGatewayHint": "Seeds a new monitor or scan only; saving it sends no KNX traffic.",
  "settings.programmingConsent": "Programming confirmation",
  "settings.programmingConsentRemembered": "Not asked again for {stage} builds.",
  "settings.programmingConsentAsks": "Asked before every programming operation.",
  "settings.programmingConsentReset": "Ask again",
  "settings.diagnostic.migrated": "Settings migrated from schema {fromVersion} to {toVersion}.",
  "settings.diagnostic.adopted": "Browser preferences were adopted into settings schema {toVersion}.",
  "settings.diagnostic.refusedNewer": "Settings schema {fileVersion} is newer than this build ({currentVersion}); the file was left untouched.",
  "settings.diagnostic.quarantined.unreadable": "Unreadable settings were moved to {movedTo}; defaults are active.",
  "settings.diagnostic.quarantined.invalidJson": "Invalid JSON settings were moved to {movedTo}; defaults are active.",
  "settings.diagnostic.quarantined.notObject": "Settings with an invalid document shape were moved to {movedTo}; defaults are active.",
  "settings.diagnostic.quarantined.missingSchemaVersion": "Settings without a readable schema version were moved to {movedTo}; defaults are active.",
  "settings.diagnostic.quarantined.settingsNotObject": "Settings with an invalid preferences object were moved to {movedTo}; defaults are active.",
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
  "projectDiff.detailsHeading": "Details",
  "projectDiff.tableToggle": "{table} ({count})",
  "projectDiff.projectInfo": "Project info",
  "projectDiff.installationInfo": "Installation info",
  "projectDiff.installationHeading": "Installation {id}",
  "projectDiff.entity.comObjects": "Communication objects",
  "projectDiff.entity.parameters": "Parameters",
  "projectDiff.fieldHeader": "Field",
  "projectDiff.beforeHeader": "Before",
  "projectDiff.afterHeader": "After",
  "projectDiff.matchedBy.etsId": "matched by ETS ID",
  "projectDiff.matchedBy.naturalKey": "matched by natural key",
  "projectDiff.ambiguityCandidates": "{left} candidates before, {right} after",
  "projectDiff.key.unaddressed": "unaddressed",
  "projectDiff.shownOf": "Showing {shown} of {total}.",
  "projectDiff.showMore": "Show more ({count})",
  // CT-6: raw `.knxproj` comparison inputs and their import report.
  "projectDiff.anyFilterName": "KNXBench or ETS project",
  "projectDiff.etsFilterName": "ETS project export",
  "projectDiff.importSummary.one": "ETS import report: {count} diagnostic",
  "projectDiff.importSummary.other": "ETS import report: {count} diagnostics",
  "projectDiff.importErrors.one": "{count} error",
  "projectDiff.importErrors.other": "{count} errors",
  "projectDiff.importWarnings.one": "{count} warning",
  "projectDiff.importWarnings.other": "{count} warnings",
  "projectDiff.importRefused": "Comparison refused: the ETS import reported errors, so the result would not be trustworthy. Fix the export and compare again.",

  // `GroupAddressCsvButtons.tsx` — controller ruling after the task 3
  // review: only the native file-dialog filter name is in scope here,
  // nothing else in that file.
  "groupAddressCsv.filterName": "Group-address CSV",

  // `App.tsx` (task 5, controller correction): the addendum that sent
  // `filePicker.ts`/`GroupAddressCsvButtons.tsx` into scope mislocated
  // these two strings — they actually live here, as a module-level const
  // (`pickProject`'s own inline filter). Both are resolved at call time
  // now, same reasoning as `commandRegistry.ts`'s `COMMANDS` in task 3: a
  // module-level `t()` call would freeze the first language forever.
  // The ETS filter is an *open* filter now: KNXBench reads `.knxproj` and
  // never writes one (ADR-0028).
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
  "busCompose.inputFormatLabel": "Input format",
  "busCompose.inputFormatAuto": "Auto (legacy compatibility)",
  "busCompose.inputFormatCanonical": "Canonical",
  "busCompose.inputFormatDecimal": "Decimal",
  "busCompose.inputFormatHexadecimal": "Hexadecimal",
  "busCompose.inputFormatBinary": "Binary",
  "busCompose.inputFormatText": "Text",
  "busCompose.send": "Send",
  "busCompose.liveAction": "Sends to the connected bus. Project Undo cannot reverse this action.",
  "busCompose.sent": "Sent {service}: {payload}",
  "busCompose.sentDecoded": "Decoded: {text}",
  "busCompose.contextStaleMessage":
    "The project changed after this bus session's last confirmed context publication — the DPT would be resolved against that earlier context, so sending is locked. Reconnect first.",

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
  "fsPicker.uploaded.one": "Uploaded {count} file. Choose one to open.",
  "fsPicker.uploaded.other": "Uploaded {count} files. Choose one to open.",
  "fsPicker.uploadFailed": "Uploaded {uploaded} of {count} files; {file} failed: {error}",
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
    "Group addresses imported from CSV: {created} created, {updated} updated, {readdressed} readdressed, {deleted} deleted, {unchanged} unchanged",
  "groupAddressCsv.confirmDestructive":
    "Apply this CSV plan? It will readdress {readdressed} and delete {deleted} group addresses; {affectedLinks} communication-object links are affected. This exact preview will be rejected if the project or CSV changes.",
  "groupAddressCsv.confirmDestructiveDetail":
    "{action}: raw address {source} → {target}; communication-object ids: {links}",
  "groupAddressCsv.actionReaddress": "Readdress",
  "groupAddressCsv.actionDelete": "Delete",
  "groupAddressCsv.directionSend": "send",
  "groupAddressCsv.directionReceive": "receive",
  "groupAddressCsv.confirmCancelled": "CSV import cancelled; no changes were applied.",
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
  "workbench.openHint": "Start from scratch, continue a KNXBench project, or import an ETS archive.",
  "workbench.newDescription": "Start an empty project and build its structure here.",
  "workbench.openNativeTitle": "Open KNXBench project",
  "workbench.openNativeDescription": "Continue a saved KNXBench .knxdb project.",
  "workbench.importEtsTitle": "Import ETS project",
  "workbench.importEtsDescription": "Read an ETS .knxproj archive. Save your work as .knxdb later.",
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
  "newProject.languageOther": "Another language tag…",
  "newProject.customLanguage": "Custom language tag",
  "newProject.languageHint": "The language tag your project texts are stored under. Choose a listed language or enter another well-formed tag, e.g. de-DE. This does not change the interface language.",
  "newProject.filenameHint": "Save or Save As chooses the .knxdb filename. Project and installation names are not file paths.",
  "newProject.style": "Group address style",
  "newProject.styleHint": "Choose the group-address style now; it cannot currently be changed after creation.",
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
  "newProject.conflictSave": "Save and create",

  // The project-load banner (ADR-0023). Every `loadProgress.phase.*` entry
  // names a stage the import/open pipeline really runs, in pipeline order;
  // a phase the server sends that is missing here renders as its raw wire
  // name rather than as a soothing generic label.
  "loadProgress.importing": "Importing {source}…",
  "loadProgress.opening": "Opening {source}…",
  "loadProgress.succeeded": "Loaded {source}.",
  "loadProgress.recovered": "Recovered the current project.",
  "loadProgress.failed": "Could not load {source}",
  "loadProgress.failedDuring": "Failed during: {phase}",
  "loadProgress.barLabel": "Load progress",
  "loadProgress.counted": "{completed} of {total}",
  "loadProgress.phase.starting": "Starting…",
  "loadProgress.phase.openContainer": "Opening the archive",
  "loadProgress.phase.detectSchema": "Detecting the schema version",
  "loadProgress.phase.parseTopology": "Parsing the topology",
  "loadProgress.phase.parseProjectInfo": "Parsing the project information",
  "loadProgress.phase.validate": "Validating references",
  "loadProgress.phase.map": "Building the project model",
  "loadProgress.phase.inferDatapointTypes": "Inferring datapoint types",
  "loadProgress.phase.collectContainerEntries": "Reading the remaining archive entries",
  "loadProgress.phase.ingestManufacturerData": "Ingesting manufacturer data",
  "loadProgress.phase.ingestMasterData": "Ingesting the KNX master data",
  "loadProgress.phase.enrichFromProductDatabase": "Enriching from the product database",
  "loadProgress.phase.persistOpaque": "Storing passthrough data",
  "loadProgress.phase.openStore": "Opening the project file",
  "loadProgress.phase.loadStoredProject": "Reading the stored project",
  "loadProgress.phase.loadOpaque": "Reading passthrough data",
  "loadProgress.phase.loadManufacturerRefs": "Reading manufacturer references",
  "loadProgress.phase.buildProjectTree": "Building the project tree",

  // The banner's flavour line (Task 26). Decoration, never progress: a
  // rotating joke sits beside the real phase label, never in place of it,
  // and `LoadProgressBanner` hides it from assistive technology. Fifty
  // entries because a slow load deserves a long list; a fast one shows
  // exactly one of them and nobody is any the wiser.
  "loadProgress.flavour.01": "Sorting group addresses by colour",
  "loadProgress.flavour.02": "Asking the devices how they are feeling",
  "loadProgress.flavour.03": "Checking the constellation of the stars",
  "loadProgress.flavour.04": "Counting the bus telegrams twice",
  "loadProgress.flavour.05": "Negotiating with the line coupler",
  "loadProgress.flavour.06": "Polishing the individual addresses",
  "loadProgress.flavour.07": "Convincing an actuator that it is a sensor",
  "loadProgress.flavour.08": "Untangling the topology",
  "loadProgress.flavour.09": "Waking the presence detectors gently",
  "loadProgress.flavour.10": "Estimating the bus cable length by eye",
  "loadProgress.flavour.11": "Looking for the missing datapoint type",
  "loadProgress.flavour.12": "Translating ETS into something readable",
  "loadProgress.flavour.13": "Putting the manufacturers in alphabetical order",
  "loadProgress.flavour.14": "Asking the dimmer to dim expectations",
  "loadProgress.flavour.15": "Checking whether 1.1.1 is home",
  "loadProgress.flavour.16": "Warming up the twisted pair",
  "loadProgress.flavour.17": "Reading the datasheet nobody reads",
  "loadProgress.flavour.18": "Rounding 230 V down to something safer",
  "loadProgress.flavour.19": "Explaining their flags to the communication objects",
  "loadProgress.flavour.20": "Sorting the rooms by cosiness",
  "loadProgress.flavour.21": "Waiting for an acknowledgement telegram",
  "loadProgress.flavour.22": "Asking the blind actuator which way is up",
  "loadProgress.flavour.23": "Working out what the third button really does",
  "loadProgress.flavour.24": "Feeding the parameters",
  "loadProgress.flavour.25": "Checking the checksums, then checking them again",
  "loadProgress.flavour.26": "Consulting the KNX Standard, volume by volume",
  "loadProgress.flavour.27": "Looking up a footnote from 2003",
  "loadProgress.flavour.28": "Apologising to the association table",
  "loadProgress.flavour.29": "Searching for the device that was never installed",
  "loadProgress.flavour.30": "Aligning the floors with gravity",
  "loadProgress.flavour.31": "Having an opinion about naming conventions",
  "loadProgress.flavour.32": "Rewinding the telegram",
  "loadProgress.flavour.33": "Letting the weather station guess",
  "loadProgress.flavour.34": "Counting the group addresses nobody uses",
  "loadProgress.flavour.35": "Checking whether anyone documented this",
  "loadProgress.flavour.36": "Teaching the binary input to say yes",
  "loadProgress.flavour.37": "Straightening the DIN rail",
  "loadProgress.flavour.38": "Looking behind the distribution board",
  "loadProgress.flavour.39": "Reconciling two spellings of the same room",
  "loadProgress.flavour.40": "Measuring the twist in the pair",
  "loadProgress.flavour.41": "Asking the thermostat to be reasonable",
  "loadProgress.flavour.42": "Deciding whether the cellar counts as a floor",
  "loadProgress.flavour.43": "Rescuing a comment somebody wrote in 2011",
  "loadProgress.flavour.44": "Sorting the scenes by drama",
  "loadProgress.flavour.45": "Giving the gateway a moment to itself",
  "loadProgress.flavour.46": "Confirming the sun still rises in the east",
  "loadProgress.flavour.47": "Counting the devices, then counting them again",
  "loadProgress.flavour.48": "Making room for the building structure",
  "loadProgress.flavour.49": "Filing the unknown attributes carefully",
  "loadProgress.flavour.50": "Persuading the project to open",

  // T23 (ADR-0024). The help system's entire vocabulary: the panel's own
  // chrome, one title plus n paragraph keys per topic in `help.ts`'s
  // `HELP_TOPICS`, the shared closing note four concept topics opt into,
  // and a label/text pair per `HelpTip` placement. One key is one
  // paragraph of plain text — no markup, ever, since nothing renders
  // these as HTML and a translated string that did would be an injection
  // hole with a friendly name.
  "toolbar.help": "Help (F1)",
  "help.title": "Help",
  "help.intro":
    "What the parts of this window do, and what the KNX terms behind them mean. F1 opens this from anywhere in the main window; the separate diagnostics window carries no help of its own.",
  "help.topics": "Topics",
  "help.close": "Close",
  "help.standardNote":
    "This explains how KNXBench uses the concept, not how the KNX Standard defines it — the Standard remains the authority.",

  "help.topic.gettingStarted.title": "Getting started",
  "help.topic.gettingStarted.p1":
    "KNXBench opens two kinds of file. Its own format, a .knxdb project, is what “Open (.knxdb)…” reads and what Save writes. An ETS project export, a .knxproj file, is read by “Open project…” and converted into a KNXBench project on the way in — once. From then on the project lives as a .knxdb; KNXBench does not write .knxproj files.",
  "help.topic.gettingStarted.p2":
    "Importing never writes back to the file you imported. The .knxproj is read and left alone; what you get is a project in memory, and it only reaches disk when you save it as a .knxdb.",
  "help.topic.gettingStarted.p3":
    "With no file to start from, “New project…” creates an empty project with a name, a project language and a group address style, and you build the structure from the tree on the left.",

  "help.topic.workbench.title": "The window",
  "help.topic.workbench.p1":
    "Three regions. On the left, the navigation pane with the views and the project tree. In the middle, whichever view you picked. On the right, the Properties inspector, which shows and edits whatever is selected.",
  "help.topic.workbench.p2":
    "The four views are Overview, Buildings, Topology and Group addresses. Overview summarises the project and shows what the last import complained about. Buildings and Topology are two different orders over the same devices. Group addresses is the table of addresses and their links.",
  "help.topic.workbench.p3":
    "Both side panes collapse with the buttons above them and resize by dragging their inner edge. The product catalog opens from the navigation pane and adds devices from an installed manufacturer database.",

  "help.topic.buildings.title": "Buildings, floors and rooms",
  "help.topic.buildings.p1":
    "The building structure says where a device physically is — a building, its floors, the rooms on them, the distribution boards inside. It exists so a device can be found by walking the site instead of by remembering its address.",
  "help.topic.buildings.p2":
    "Here it is a tree of named parts that contain one another, not a drawing: no coordinates, no floor plans, no positions. A device belongs to at most one building part at a time, and moving it changes nothing about its wiring.",

  "help.topic.topology.title": "Areas, lines and devices",
  "help.topic.topology.p1":
    "The topology says how a device is wired: areas hold lines, lines hold devices. A device's individual address is written area.line.device and is its identity on the bus.",
  "help.topic.topology.p2":
    "The three parts are not equally wide. Area runs 0–15, line runs 0–15, device runs 0–255. An address outside those ranges is refused rather than quietly truncated.",
  "help.topic.topology.p3":
    "An individual address is unique within a project: two devices cannot hold the same one. The Topology view is where you see which line a device sits on, and where you move it to another.",

  "help.topic.groupAddresses.title": "Group addresses",
  "help.topic.groupAddresses.p1":
    "A group address is not a device. It names one piece of shared state — a light's on/off, a blind's position, a room's setpoint — that any number of devices may send to or listen for. Devices reach each other only through group addresses.",
  "help.topic.groupAddresses.p2":
    "Underneath it is a single 16-bit number. How it is written is a project-wide choice: three level (main/middle/sub), two level (main/sub), or free, which is just the number. Restyling a project changes the text and never the address.",
  "help.topic.groupAddresses.p3":
    "A group address also carries a datapoint type, which says how to read the bytes on the wire — 1.001 is a switch, 5.001 a percentage. The Group addresses view shows the type it has, marks the ones that have none, and marks the ones whose links disagree about it.",

  "help.topic.groupRanges.title": "Group address ranges",
  "help.topic.groupRanges.p1":
    "A range is a named container for group addresses. Ranges can nest: a child range must fit inside its parent's address interval. The Range column shows the containing path, not a value sent on the bus.",
  "help.topic.groupRanges.p2":
    "The start and end are group-address boundaries in the project's chosen address style. They do not describe a range of datapoint types or payload values; the datapoint type belongs to each address and its links.",

  "help.topic.comObjectFlags.title": "Communication object flags",
  "help.topic.comObjectFlags.p1":
    "A communication object is one input or output of a device's application program. Linking it to a group address is what puts the device on that address. KNXBench models six flags for it — shown as R, W, T, U, C and I in the Properties inspector — and together they decide what the object may do there.",
  "help.topic.comObjectFlags.p2":
    "C, communication, is the master switch: with it off the object takes no part in bus traffic and the other five have nothing to act on. R, read, lets the object answer a read request with its current value. W, write, lets an incoming telegram change that value.",
  "help.topic.comObjectFlags.p3":
    "T, transmit, lets the object send by itself when its value changes — the flag that makes a sensor a sender. U, update, lets it adopt a value it sees in another device's read response. I, read on init, asks the bus for the address's value once at start-up, so the object begins with a real value rather than a guess. KNXBench stores the six as you set them; it does not judge which combination suits your device.",

  "help.topic.busMonitor.title": "Bus monitor",
  "help.topic.busMonitor.p1":
    "The bus monitor watches live traffic through a KNXnet/IP gateway. Type the gateway's address and port, connect, and telegrams appear as they arrive. Watching and sending are separate: the table only displays, and sending a telegram is its own deliberate action.",
  "help.topic.busMonitor.p2":
    "Tunnelling is the only transport. This window does not search the network for gateways and does not join a routing multicast group, which is why the address has to be typed in.",
  "help.topic.busMonitor.p3":
    "Telegrams are decoded against the project that is open. If that project changes while a session runs, the monitor says so instead of silently re-labelling what it decoded earlier — the notice means the decoded column is the older project's answer.",

  "help.topic.importExport.title": "Import",
  "help.topic.importExport.p1":
    "An import reports what it found: errors, warnings, and anything it could not map. Overview shows the counts and the Log lists them. They are worth reading — otherwise an import with nothing to say and an import you never looked at are the same picture.",
  "help.topic.importExport.p2":
    "Data this application does not model is not thrown away. It is either kept as it was read or listed as unsupported, and it turns up in the import report rather than vanishing between the file and the project.",
  "help.topic.importExport.p3":
    "Import runs one way. KNXBench reads a .knxproj and never writes one, so there is no route back to ETS through a project file — the project you work on is a .knxdb from the first save onwards. What does leave the application: group addresses go in and out as CSV, and “Export documentation…” writes a readable description of the project.",

  "help.topic.keyboard.title": "Keyboard",
  "help.topic.keyboard.p1":
    "Ctrl+K opens Search across the project. Ctrl+Shift+P opens the command palette, which lists what is available right now together with each shortcut. F1 opens this help. Plain F1 only — a modified F1 is left to the browser and the desktop.",
  "help.topic.keyboard.p2":
    "Ctrl+Z undoes and Ctrl+Shift+Z redoes, on the project rather than on the text you are typing: inside an input, a dialog or the project tree's rename field, the field's own undo applies instead.",
  "help.topic.keyboard.p3":
    "Escape closes what is on top — an open help tip first, then the dialog around it. While a dialog is open Tab cycles inside it and cannot leave, and closing it returns focus to whatever opened it.",

  "help.topic.limits.title": "What this does not do",
  "help.topic.limits.p1":
    "KNXBench is an independent application. It is not made by, endorsed by or certified by the KNX Association, and it is not ETS. Where it reads an ETS file it does so on its own reading of that file.",
  "help.topic.limits.p2":
    "KNX Secure is not supported. A password-protected project cannot be opened from this window — there is nowhere to type the password. Programming devices over the bus is not offered here either: the bus features are watching traffic and sending single telegrams. There is no way back to ETS: KNXBench reads a .knxproj and never writes one, so a project imported here cannot be handed back as an ETS project file.",
  "help.topic.limits.p3":
    "What is here is tested, but a test suite is not a site survey. Before trusting this application about an installation, check what it tells you against the installation itself.",

  "help.tip.comFlags.label": "What the communication object flags mean",
  "help.tip.comFlags.text":
    "R, W, T, U, C and I decide what this object may do on the bus. C is the master switch — with it off, the other five do nothing. F1 has the full explanation.",
  "help.tip.groupRange.label": "What a group address range means",
  "help.tip.groupRange.text":
    "Ranges form a hierarchy: each child is contained within its parent's group-address interval. This is not a datapoint range. F1 opens the full explanation.",
  "help.tip.addressTable.label": "What this table shows",
  "help.tip.addressTable.text":
    "Every group address in the project, with its datapoint type and the communication objects linked to it. The box filters by address, by name or by datapoint type. F1 explains what a group address is.",
  "help.tip.busGateway.label": "What to enter as the gateway",
  "help.tip.busGateway.text":
    "Enter the numeric IPv4 host and port in separate fields (3671 by default). Search can suggest interfaces, but an empty result never blocks a manual address. This monitor uses tunnelling only.",

  // T28, from the user's own hands-on run. The two separator labels name
  // what moves, not where the handle is: a screen reader announcing
  // "Navigation, separator" would leave a keyboard user guessing which of
  // the two horizontal handles they had landed on.
  "workbench.resizeNavigation": "Height of the navigation block",
  "workbench.resizeDiagnostics": "Height of the diagnostics block",

  "toolbar.quit": "Quit",
  "toolbar.about": "About KNXBench…",
  // T01b / ADR-0026. Shown only where a session exists — never on the
  // desktop shell, never on a server started without a password.
  "toolbar.logout": "Log out",

  // T01b / ADR-0026 — the login screen. It says "this server", not "your
  // account": there are no accounts, only one shared password, and
  // pretending otherwise would be a promise the server does not keep.
  "login.eyebrow": "KNX-compatible · Linux-first",
  "login.intro": "This server is password-protected. Enter the password to carry on.",
  "login.password": "Password",
  "login.submit": "Sign in",
  "login.pending": "Signing in…",
  "login.rejected": "That password was not accepted.",
  "login.checking": "Asking the server whether it wants a password…",
  // The honest half of "preserve unsaved work". What is on screen behind
  // this panel is kept, because the workbench was never unmounted — but the
  // project itself lives in the server's memory, and a server that was
  // restarted has forgotten it. This sentence says both.
  "login.expiredNotice":
    "Your session ended — it either idled out or the server was restarted. Sign in again to carry on; what is on screen has been kept, but if the server was restarted its copy of the project is gone and you will have to open it again.",
  "login.signedOutNotice":
    "You are logged out. Sign in again and the workbench comes back exactly as you left it.",
  "login.footnote":
    "One password for the whole server, set when it was started. There are no user accounts yet.",

  "quit.title": "Unsaved changes",
  "quit.message":
    "This project has edits that are in no file yet. Quitting now throws them away.",
  "quit.hint":
    "“Save and quit” saves first and quits only if that worked.",
  "quit.cancel": "Cancel",
  "quit.discard": "Quit without saving",
  "quit.saving": "Saving…",
  "quit.save": "Save and quit",

  // ISSUE-04's autosave countdown toast and failure notice. `{seconds}`
  // ticks down from `AUTOSAVE_COUNTDOWN_SECONDS`; `cancel` stops this
  // cycle only — the next interval still offers to autosave again.
  "autosave.countdown": "Autosaving in {seconds}s…",
  "autosave.cancel": "Cancel",
  "autosave.failed": "Autosave failed. Your edits are safe but unsaved — save manually when you can.",

  "about.title": "About KNXBench",
  "about.version": "Version",
  "about.versionUnknown": "unknown — the server did not answer",
  "about.licence": "Licence",
  // Not translated and not spelled out: an SPDX identifier is an
  // identifier, and “GNU Affero General Public License, version 3 or
  // later” in prose is the thing people mistype.
  "about.licenceValue": "AGPL-3.0-or-later",
  "about.independence":
    "KNXBench is an independent project. It is not certified by the KNX Association and is not affiliated with it.",
  "about.trademark": "ETS is a trademark of the KNX Association.",
  "about.close": "Close",

  // Programming consent (`ProgrammingConsentDialog.tsx`). Asked before any
  // write that programs a device; names the build's release stage. Group
  // value sends from the bus monitor are not programming and never ask.
  "programmingConsent.title": "Program this device?",
  "programmingConsent.target": "About to program: {target}",
  "programmingConsent.stageLabel": "Release stage",
  "programmingConsent.versionLabel": "Build",
  "programmingConsent.versionUnknown": "unknown — the server did not answer",
  "programmingConsent.stage.alpha": "Alpha",
  "programmingConsent.stage.beta": "Beta",
  "programmingConsent.stage.releaseCandidate": "Release candidate",
  "programmingConsent.stage.stable": "Stable release",
  "programmingConsent.stage.preRelease": "Unnamed pre-release",
  "programmingConsent.stage.unknown": "Unknown",
  "programmingConsent.risk.alpha":
    "This is alpha software. Programming is new, incomplete and has been tested on very few devices. A failed or wrong write can leave a device unusable until it is reprogrammed with other tools.",
  "programmingConsent.risk.beta":
    "This is beta software. Programming is feature-complete but not yet proven across many installations. A failed or wrong write can leave a device unusable until it is reprogrammed.",
  "programmingConsent.risk.releaseCandidate":
    "This is a release candidate. Programming is expected to work but has not been released yet.",
  "programmingConsent.risk.stable":
    "Programming changes the device immediately. A wrong project setting is written as it is.",
  "programmingConsent.risk.preRelease":
    "This build carries a pre-release label KNXBench does not recognise. Treat it as untested.",
  "programmingConsent.risk.unknown":
    "KNXBench could not determine which build is running. Treat it as untested.",
  "programmingConsent.backup":
    "Only continue if you know how to restore this device. KNXBench is not certified by the KNX Association.",
  "programmingConsent.remember": "Don't ask again for {stage} builds",
  "programmingConsent.rememberUnavailable":
    "Because the release stage is not known, this question is asked every time.",
  "programmingConsent.cancel": "Cancel",
  "programmingConsent.confirm": "Program device",

  // T29 — the debug report. The dialog is translated; `report.md` and the
  // GitHub issue body are not (they are server-generated English, read by
  // whoever picks the issue up).
  "debugReport.button": "Debug report…",
  "debugReport.title": "Debug report",
  "debugReport.intro":
    "Collects what is useful for diagnosing a problem into one zip file on this computer. Nothing is sent anywhere: the file is written where you choose it, and the GitHub button only opens a prefilled issue page in your browser for you to read before you post it.",
  "debugReport.descriptionLabel": "What happened?",
  "debugReport.descriptionPlaceholder": "What you did, what you expected, what happened instead.",
  "debugReport.descriptionHint":
    "Goes into the report in your own words. Everything else below is collected automatically.",
  "debugReport.filterName": "Zip archive",
  "debugReport.include.log.label": "Session log",
  "debugReport.include.log.hint":
    "The entries from the Log tab, with IP addresses removed. Import problems are logged by name, so this file can contain KNX group addresses and the names of imported elements.",
  "debugReport.include.projectSummary.label": "Project statistics",
  "debugReport.include.projectSummary.hint":
    "How many devices, lines and group addresses the open project has. Counts only — no names, no addresses.",
  "debugReport.include.busTelegrams.label": "Bus monitor telegrams",
  "debugReport.include.busTelegrams.hint":
    "The telegrams currently in the monitor buffer. These keep the individual and group addresses of your devices.",
  "debugReport.contentsTitle": "What the file will contain",
  "debugReport.contents.report": "report.md — your description, the versions and the environment.",
  "debugReport.contents.environment": "environment.json — the same facts in machine-readable form.",
  "debugReport.contents.log":
    "log.json — this session's log entries; may name KNX addresses and imported elements.",
  "debugReport.contents.projectSummary": "project-summary.json — counts describing the open project.",
  "debugReport.contents.busTelegrams": "bus-telegrams.json — the bus monitor buffer.",
  "debugReport.privacyRedacted":
    "IP addresses, your home directory and this computer's name are replaced by placeholders in report.md, environment.json and log.json. KNX addresses and names taken from your project are not replaced anywhere.",
  "debugReport.privacyTelegrams":
    "bus-telegrams.json keeps the individual and group addresses of your installation and, where the open project knows them, the names of the group addresses — \u201cKitchen ceiling light\u201d. Without those a telegram dump says nothing, which is why they stay. Include the file only if you are willing to share them.",
  "debugReport.save": "Save zip…",
  "debugReport.openIssue": "Open a GitHub issue…",
  "debugReport.close": "Close",
  "debugReport.busy": "Collecting…",
  "debugReport.saved.one": "Debug report saved, {count} file.",
  "debugReport.saved.other": "Debug report saved, {count} files.",
  "debugReport.issueOpened":
    "A prefilled issue was opened in your browser. Nothing has been posted — read it, attach the zip, then submit it yourself.",
  "debugReport.issueTruncated":
    "A prefilled issue was opened in your browser. The report was too long for a link and has been shortened — please save the zip as well and attach it.",
  "debugReport.issueTitle": "Bug report from KNXBench",
  "deviceDownload.eyebrow": "KNXnet/IP · Writes to a device",
  "deviceDownload.title": "Download to device",
  "deviceDownload.explainer": "Writes the open project's configuration into one device over the bus. This is not saving or exporting a file: the project stays as it is, the device changes.",
  "deviceDownload.projectRequired": "Open a project first: the download is prepared from the project's own device, parameters and group links.",
  "deviceDownload.device": "Device",
  "deviceDownload.chooseDevice": "Choose a device with an individual address",
  "deviceDownload.gateway": "Gateway address",
  "deviceDownload.preparePlan": "Show what would be written",
  "deviceDownload.planTitle": "Download plan",
  "deviceDownload.planNothingSent": "Nothing has been sent. This is what the download would write, prepared from the project as it is now; any project edit discards it.",
  "deviceDownload.target": "Device",
  "deviceDownload.program": "Application program",
  "deviceDownload.support": "Hardware evidence",
  "deviceDownload.verified": "Verified on one device:",
  "deviceDownload.untested": "Not verified on hardware. The plan is complete, but this application and download type have not been tested on a real device.",
  "deviceDownload.untestedPrompt": "To proceed, type exactly: {phrase}",
  "deviceDownload.backupBeforeWrite": "Before the first write, KNXBench reads and saves the memory this plan will overwrite. If that backup fails, nothing is written. This does not capture the device's entire memory.",
  "deviceDownload.backupKept": "Pre-write backup kept in {file} on the server. To write it back, use the CLI restore command with this file and the product database.",
  "deviceDownload.backupTaken": "Backup read and kept before the first write: {octets} octets in {regions} regions.",
  "deviceDownload.expectedDevice": "Expected on the bus",
  "deviceDownload.expectedDeviceValue": "mask {mask}h, manufacturer {manufacturer}h — checked before the first write",
  "deviceDownload.fromProject": "From the project",
  "deviceDownload.fromProjectValue": "{values} parameter values, {links} group links",
  "deviceDownload.octets": "Data",
  "deviceDownload.octetsValue": "{count} octets",
  "deviceDownload.segment": "Segment",
  "deviceDownload.address": "Address",
  "deviceDownload.segmentOctets": "Octets written",
  "deviceDownload.steps": "{count} steps",
  "deviceDownload.start": "Download to {address}",
  "deviceDownload.progressTitle": "Download to {address} — {device}",
  "deviceDownload.running": "Running: step {step} of {of} · {name}",
  "deviceDownload.finished": "Download finished.",
  "deviceDownload.failed": "Download stopped in step {step}.",
  "deviceDownload.stepsProgress": "Steps {done}/{of}",
  "deviceDownload.octetsProgress": "Octets written and read back {done}/{of}",
  "deviceDownload.written.yes": "Written to the device: yes, {count} octets, every block read back unchanged.",
  "deviceDownload.written.no": "Written to the device: no. The device was not changed.",
  "deviceDownload.written.partially": "Written to the device: partially. The device may be partially loaded; run the download again.",
  "deviceDownload.restartUnconfirmed": "Restart: NOT confirmed. The data is written and read back, but the device did not acknowledge the restart. Some devices restart without answering; check that it works.",
  "deviceDownload.restartAcknowledged": "Restart: acknowledged by the device.",
  "deviceDownload.blocksCaption": "Data written, each block shown after the device read it back unchanged",
  "deviceDownload.step": "Step",
  "deviceDownload.data": "Data (hex)",
  "deviceDownload.running_total": "Total",
  "addressProgramming.eyebrow": "KNXnet/IP · Writes to a device",
  "addressProgramming.title": "Program address",
  "addressProgramming.explainer": "Gives one device its individual address over the bus: the device whose programming button is pressed. Only the address is written; this is not a download of parameters, and not saving a file.",
  "addressProgramming.newAddress": "New individual address",
  "addressProgramming.gateway": "Gateway address",
  "addressProgramming.waitSeconds": "Wait for the button (seconds)",
  "addressProgramming.waitOutOfRange": "The wait must be between 1 and {max} seconds.",
  "addressProgramming.planTitle": "What happens",
  "addressProgramming.planWait": "Wait up to {seconds} s until exactly one device is in programming mode, asking about every 2 s. Nothing is written while waiting.",
  "addressProgramming.planStep1": "Check whether {address} is already taken; stop if another device holds it.",
  "addressProgramming.planStep2": "Count the devices in programming mode again: exactly one.",
  "addressProgramming.planStep3": "Write {address} to that device (skipped if it already has it).",
  "addressProgramming.planStep4": "Connect to {address}, read it back, restart it: this ends programming mode.",
  "addressProgramming.start": "Program {address}",
  "addressProgramming.consentTarget": "the device in programming mode → {address}",
  "addressProgramming.progressTitle": "Programming {address}",
  "addressProgramming.pressButton": "Press the programming button on the device that should get this address.",
  "addressProgramming.foundOne": "One device is in programming mode ({device}).",
  "addressProgramming.releaseAllButOne": "Several devices are in programming mode ({devices}): release all but one.",
  "addressProgramming.waiting": "Waiting · round {rounds} · gives up after {seconds} s",
  "addressProgramming.stop": "Stop waiting",
  "addressProgramming.programming": "Found {previous}; programming {address} now. This runs to its end and cannot be stopped.",
  "addressProgramming.written.yes": "Address written: yes, {previous} → {address}. The device answered at {address} and was restarted.",
  "addressProgramming.written.noNeed": "Address written: no need, the device already had {address}. It answered and was restarted.",
  "addressProgramming.written.no": "Address written: no. The device was not changed.",
  "addressProgramming.written.unconfirmed": "Address written: yes, but NOT confirmed. The device did not answer at {address} in step {step}. Check it with a read before anything else; recovery is another programming-mode session.",
  "addressProgramming.stopped": "Stopped after {rounds} rounds. Nothing was written.",
  "addressProgramming.round": "Round {number}:",
  "addressProgramming.nobody": "no device in programming mode",
  "addressProgramming.tab": "Program address",
} as const;

export type Messages = typeof messages;
export type MessageKey = keyof Messages;
