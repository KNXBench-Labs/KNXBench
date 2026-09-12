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

  "toast.error.notAsPlanned": "Well, that didn't go as planned: {msg}",
  "toast.error.busObjects": "The bus objects: {msg}",
  "toast.error.gremlins": "Gremlins in the wiring: {msg}",
  "toast.error.knxSaysNo": "KNX says no: {msg}",
  "toast.error.notToday": "Not today: {msg}",
  "toast.error.houston": "Houston, we have a problem: {msg}",
  "toast.error.hardPass": "That's a hard pass: {msg}",

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

  "toast.lateNight.midnightOil": "Burning the midnight oil? So is your KNX bus.",
  "toast.lateNight.busLineRest": "It's late. Even the bus line needs rest.",
  "toast.lateNight.stillAwake": "Still awake? The group addresses admire your dedication.",
  "toast.lateNight.nightOwl": "Night owl mode engaged.",

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
    "Shared across every instantiation of this module; read-only in this release.",
  "parameters.staleValuesHeading": "Stale values ({count})",
  "parameters.staleDescription":
    "These stored values no longer correspond to any parameter in the current application program.",
  "parameters.diagnosticsCount.one": "{count} issue found while evaluating this device's parameters",
  "parameters.diagnosticsCount.other": "{count} issues found while evaluating this device's parameters",
  "parameters.copyDetails": "Copy details",
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
  "logPanel.severity.error": "Error",
  "logPanel.severity.warning": "Warning",
  "logPanel.severity.info": "Info",
  "logPanel.emptyNoEntries": "No log entries yet.",
  "logPanel.emptyFiltered": "No log entries match the current filters.",

  "busMonitor.title": "Bus monitor",
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
  "busCompose.sent": "Sent {service}: {payload}",

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
} as const;

export type Messages = typeof messages;
export type MessageKey = keyof Messages;
