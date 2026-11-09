/** Guards catalogue keys, placeholders, plurals and corrected German wording. */
import { describe, expect, it } from "vitest";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";
import { messages as bar } from "./messages/bar";
import { messages as tlh } from "./messages/tlh";

type Catalogue = Record<string, string>;

const EN: Catalogue = en;
const DE: Catalogue = de;
const FUN_PACKS: ReadonlyArray<readonly [string, Catalogue]> = [
  ["bar", bar],
  ["tlh", tlh],
];

/** `{name}` placeholders of one message, sorted so only multiplicity and names matter. */
function placeholders(text: string): string[] {
  return (text.match(/\{[A-Za-z0-9_]+\}/g) ?? []).slice().sort();
}

/** Keys whose placeholder names or multiplicities differ from English. */
function placeholderMismatches(catalogue: Catalogue): string[] {
  const bad: string[] = [];
  for (const [key, text] of Object.entries(catalogue)) {
    const source = EN[key];
    if (source === undefined) continue;
    const want = placeholders(source);
    const have = placeholders(text);
    if (want.join("|") !== have.join("|")) bad.push(key);
  }
  return bad;
}

// These existing action/diagnostic keys are not grammatical plural forms.
const NON_PLURAL_KEYS = new Set([
  "compare.one",
  "storedBusAccess.element.other",
  "diagnostics.stopReason.other",
  "deviceDownload.readiness.reason.other",
]);

function pluralBases(catalogue: Catalogue): Set<string> {
  const bases = new Set<string>();
  for (const key of Object.keys(catalogue)) {
    if (NON_PLURAL_KEYS.has(key)) continue;
    const match = /^(.*)\.(one|other)$/.exec(key);
    if (match) bases.add(match[1]);
  }
  return bases;
}

function danglingPluralHalves(catalogue: Catalogue): string[] {
  const bad: string[] = [];
  for (const base of pluralBases(catalogue)) {
    if (!(`${base}.one` in catalogue) || !(`${base}.other` in catalogue)) bad.push(base);
  }
  return bad;
}

describe("catalogue guard controls", () => {
  it("distinguishes action/default keys from inflection while rejecting missing plural halves", () => {
    expect(danglingPluralHalves({ "compare.one": "Compare", "storedBusAccess.element.other": "{element}",
      "diagnostics.stopReason.other": "Other", "deviceDownload.readiness.reason.other": "{category}" })).toEqual([]);
    expect(danglingPluralHalves({ "items.one": "One item" })).toEqual(["items"]);
    expect(danglingPluralHalves({ "items.other": "{count} items" })).toEqual(["items"]);
    expect(danglingPluralHalves({ "items.one": "One item", "items.other": "{count} items" })).toEqual([]);
  });

  it("detects missing, renamed and repeated placeholders in singular forms too", () => {
    const key = "achievements.moreUnlocked.one";
    expect(placeholderMismatches({ [key]: "+{count} weiterer Erfolg freigeschaltet" })).toEqual([]);
    for (const text of ["Ein weiterer Erfolg", "+{total} Erfolg", "{count} {count}"]) {
      expect(placeholderMismatches({ [key]: text })).toEqual([key]);
    }
  });
});

describe("German catalogue parity with English", () => {
  it("has exactly the English key set", () => {
    const enKeys = Object.keys(EN).sort();
    const deKeys = Object.keys(DE).sort();
    expect(deKeys.filter((key) => !(key in EN))).toEqual([]);
    expect(enKeys.filter((key) => !(key in DE))).toEqual([]);
    expect(deKeys.length).toBe(enKeys.length);
  });

  it("has no empty or whitespace-only message", () => {
    expect(Object.entries(EN).filter(([, text]) => text.trim() === "").map(([key]) => key)).toEqual([]);
    expect(Object.entries(DE).filter(([, text]) => text.trim() === "").map(([key]) => key)).toEqual([]);
  });

  it("keeps every placeholder of the English text", () => {
    expect(placeholderMismatches(DE)).toEqual([]);
  });

  it("keeps the same plural pairs as English", () => {
    expect(pluralBases(DE)).toEqual(pluralBases(EN));
    expect(danglingPluralHalves(EN)).toEqual([]);
    expect(danglingPluralHalves(DE)).toEqual([]);
  });

  it("never glues a placeholder to a colon", () => {
    const glued = (catalogue: Catalogue) =>
      Object.entries(catalogue)
        .filter(([, text]) => /:\{[A-Za-z0-9_]+\}/.test(text))
        .map(([key]) => key);
    expect(glued(DE).filter((key) => !glued(EN).includes(key))).toEqual([]);
  });
});

describe.each(FUN_PACKS)("%s fun pack stays a partial overlay of English", (_tag, pack) => {
  it("names only keys English has", () => {
    expect(Object.keys(pack).filter((key) => !(key in EN))).toEqual([]);
  });

  it("has no empty message", () => {
    expect(Object.entries(pack).filter(([, text]) => text.trim() === "").map(([key]) => key)).toEqual([]);
  });

  it("keeps every placeholder of the English text", () => {
    expect(placeholderMismatches(pack)).toEqual([]);
  });

  it("never carries half a plural pair", () => {
    expect(danglingPluralHalves(pack)).toEqual([]);
  });

  it("is deliberately incomplete, so English fallback stays in play", () => {
    expect(Object.keys(pack).length).toBeLessThan(Object.keys(EN).length);
  });
});

describe("corrected German wording", () => {
  const BANNED: ReadonlyArray<readonly [string, RegExp]> = [
    ["Undo/Redo: the toolbar says Rückgängig/Wiederholen", /\b(Undo|Redo)\b/],
    ["Achievement: the German word is Erfolg", /\bAchievements?\b/],
    ["Restore: say Wiederherstellung", /\bRestore\b/],
    ["Recovery-: say Wiederherstellung", /\bRecovery-/],
    ["Inspector: the panel is called Eigenschaften", /\bInspector\b/],
    ["Log: the panel is called Protokoll", /\bLog\b/],
    ["Untested: say ungetestet without changing literal confirmation phrases", /\bUntested\b/],
    ["Loaded-Zustand: say Ladezustand", /\bLoaded-Zustand/],
    ["Theme: the German UI says Design", /\bTheme\b/],
    ["History: say Historie or Verlauf", /\bHistory\b/],
    ["auf English: say auf Englisch", /\bauf English\b/],
    ["dungeon keeper: say Kerkermeister", /\bdungeon keeper\b/],
  ];

  it.each(BANNED)("refuses %s", (_label, pattern) => {
    const hits = Object.entries(DE)
      .filter(([, text]) => pattern.test(text))
      .map(([key]) => key);
    expect(hits).toEqual([]);
  });

  it("calls the history undo and redo steps by the toolbar's words", () => {
    expect(DE["toolbar.undo"]).toBe("Rückgängig");
    expect(DE["toolbar.redo"]).toBe("Wiederholen");
    expect(DE["projectHistory.undo"]).toBe("Rückgängig-Schritte");
    expect(DE["projectHistory.redo"]).toBe("Wiederholen-Schritte");
    expect(DE["projectHistory.clearUndo"]).toBe("Rückgängig und Wiederholen leeren…");
    expect(DE["projectHistory.confirmClear"]).toBe("Rückgängig und Wiederholen leeren");
    expect(DE["projectHistory.clearTitle"]).toBe("Rückgängig- und Wiederholen-Verlauf leeren?");
    expect(DE["projectHistory.timelineCurrentDetail"]).toBe(
      "Keine gespeicherte Version. {undo} Rückgängig- und {redo} Wiederholen-Schritte sind verfügbar; ihre Inhalte und Zeiten werden hier nicht erfasst.",
    );
  });

  it("says Wiederherstellung and Historie for project restore and history", () => {
    expect(DE["projectHistory.reason.pre_restore"]).toBe("Vor einer Projektwiederherstellung");
    expect(DE["projectHistory.decodedHistoryLimit"]).toBe(
      "Grenze für die gesamte dekodierte Historie: {decoded} MiB; geteilter Kontext zählt einmal.",
    );
    expect(DE["projectHistory.backupCaveat"]).toContain("Eine Projektwiederherstellung setzt weder");
    expect(DE["activityHistory.backup"]).toBe("Wiederherstellungsdatensatz gesichert");
    expect(DE["activityHistory.notRecorded"]).toBe("Kein gesicherter Wiederherstellungsdatensatz protokolliert");
  });

  it("calls achievements Erfolge across title, command, settings and toasts", () => {
    expect(DE["achievements.title"]).toBe("Erfolge");
    expect(DE["achievements.command"]).toBe("Erfolge…");
    expect(DE["achievements.unlockedLabel"]).toBe("Erfolg freigeschaltet");
    expect(DE["achievements.moreUnlocked.one"]).toBe("+{count} weiterer Erfolg freigeschaltet");
    expect(DE["achievements.moreUnlocked.other"]).toBe("+{count} weitere Erfolge freigeschaltet");
    expect(DE["achievements.hiddenTitle"]).toBe("Versteckter Erfolg");
    expect(DE["settings.section.achievements"]).toBe("Erfolge");
    expect(DE["settings.achievementsReset"]).toBe("Erfolge zurücksetzen…");
    expect(DE["settings.motionCategory.achievements"]).toBe("Erfolgsmeldungen");
    expect(DE["achievement.bus-master.description"]).toBe("Alle anderen Erfolge freigeschaltet.");
  });

  it("says Design, not Theme, and keeps the Rail theme name", () => {
    expect(DE["settings.theme"]).toBe("Design");
    expect(DE["settings.motionCategory.theme"]).toBe("Design-Effekte (Rail)");
    expect(DE["appearance.accentUnavailable"]).toBe(
      "Dieses Design behält seinen eigenen Akzent; die Akzenteinstellung hat hier keine Wirkung.",
    );
    expect(DE["achievement.dark-side.description"]).toBe("Das Design gewechselt.");
    expect(DE["achievement.green-phosphor.description"]).toBe("Zum Design Modern Retro Green CRT gewechselt.");
  });

  it("fixes the false friend, the loanword fragments and the English fallback hint", () => {
    expect(DE["catalogJob.field.kind"]).toBe("Art");
    expect(DE["system2.finalStep.stage.loadedState"]).toBe(
      "Lesen und Vergleichen des abschließenden Ladezustands jeder Ressource",
    );
    expect(DE["flow.export.selection"]).toBe(
      "Nur der ausgewählte sichtbare Graphbereich. Seitenleiste, Eigenschaften, Protokoll und frühere Historie sind nicht enthalten.",
    );
    expect(DE["toast.error.dungeonKeeperNarrates"]).toBe("Der Kerkermeister verkündet dein Verhängnis: {msg}");
    expect(DE["languagePack.bundledFunHint"]).toContain("erscheinen auf Englisch.");
  });

  it("leaves legitimate shared German and protocol words alone", () => {
    expect(DE["catalogJob.field.hardware"]).toBe("Hardware");
    expect(DE["catalogJob.field.status"]).toBe("Status");
    expect(DE["catalogJob.field.detail"]).toBe("Detail");
    expect(DE["catalogJob.field.sha256"]).toBe("SHA-256");
    expect(DE["language.en"]).toBe("English");
    expect(DE["language.de"]).toBe("Deutsch");
    expect(DE["login.eyebrow"]).toBe("KNX-compatible · Linux-first");
  });
});
