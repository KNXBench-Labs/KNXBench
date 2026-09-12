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
} as const;

export type Messages = typeof messages;
export type MessageKey = keyof Messages;
