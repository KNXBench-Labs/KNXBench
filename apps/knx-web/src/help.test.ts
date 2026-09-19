/** Tests the help system's rules: which key opens it, when it moves, and what a topic holds. */
import { describe, expect, it } from "vitest";
import {
  DEFAULT_HELP_TOPIC_ID,
  HELP_STANDARD_NOTE_KEY,
  HELP_TOPICS,
  helpTipAnimates,
  helpTopicParagraphs,
  opensHelp,
} from "./help";
import { messages as enMessages } from "./messages/en";
import { messages as deMessages } from "./messages/de";

type KeyEvent = Parameters<typeof opensHelp>[0];

function key(overrides: Partial<KeyEvent> = {}): KeyEvent {
  return { key: "F1", ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...overrides };
}

describe("opensHelp", () => {
  it("accepts plain F1", () => {
    expect(opensHelp(key())).toBe(true);
  });

  // Each modifier separately, because a predicate that checks three of the
  // four reads exactly like one that checks all four until the fourth one
  // is pressed.
  it.each(["ctrlKey", "metaKey", "altKey", "shiftKey"] as const)(
    "refuses F1 held with %s, which belongs to the browser or the desktop",
    (modifier) => {
      expect(opensHelp(key({ [modifier]: true }))).toBe(false);
    },
  );

  it.each(["F2", "F11", "f1", "Escape", "?", "Enter"])("refuses %s", (k) => {
    expect(opensHelp(key({ key: k }))).toBe(false);
  });
});

describe("helpTipAnimates", () => {
  const media = (matches: boolean) => ({ matchMedia: () => ({ matches }) }) as unknown as Window;
  const root = (level: string | null) => ({ getAttribute: () => level }) as unknown as Element;

  it("animates when neither switch is thrown", () => {
    expect(helpTipAnimates(media(false), root(null))).toBe(true);
  });

  it("does not animate when the application's own motion level is off", () => {
    expect(helpTipAnimates(media(false), root("off"))).toBe(false);
  });

  it("does not animate when the system asks for reduced motion", () => {
    expect(helpTipAnimates(media(true), root("standard"))).toBe(false);
  });

  it("still animates at a motion level that is not off", () => {
    expect(helpTipAnimates(media(false), root("subtle"))).toBe(true);
  });

  it("treats a runtime without matchMedia as one that never asked", () => {
    expect(helpTipAnimates({} as unknown as Window, root(null))).toBe(true);
  });
});

describe("HELP_TOPICS", () => {
  it("has unique ids and opens on the first of them", () => {
    const ids = HELP_TOPICS.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(DEFAULT_HELP_TOPIC_ID).toBe(ids[0]);
  });

  it("gives every topic a title and at least one paragraph", () => {
    for (const topic of HELP_TOPICS) {
      expect(topic.bodyKeys.length).toBeGreaterThan(0);
      expect(enMessages[topic.titleKey].length).toBeGreaterThan(0);
    }
  });

  // `satisfies readonly MessageKey[]` already makes a bad key a compile
  // error; this asserts the other half — that the catalogue entry behind a
  // real key is not an empty string in either language, which the type
  // cannot see.
  it("has non-empty English and German text behind every paragraph key", () => {
    for (const topic of HELP_TOPICS) {
      for (const bodyKey of topic.bodyKeys) {
        expect(enMessages[bodyKey].trim().length).toBeGreaterThan(0);
        expect(deMessages[bodyKey].trim().length).toBeGreaterThan(0);
      }
    }
  });

  it("appends the shared standard note to exactly the topics that opt in", () => {
    for (const topic of HELP_TOPICS) {
      const paragraphs = helpTopicParagraphs(topic.id);
      expect(paragraphs).toBeDefined();
      if (topic.standardNote) {
        expect(paragraphs).toEqual([...topic.bodyKeys, HELP_STANDARD_NOTE_KEY]);
      } else {
        expect(paragraphs).toEqual([...topic.bodyKeys]);
        expect(paragraphs).not.toContain(HELP_STANDARD_NOTE_KEY);
      }
    }
  });

  it("carries the standard note on the topics that explain a KNX concept", () => {
    const noted = HELP_TOPICS.filter((t) => t.standardNote).map((t) => t.id);
    expect(noted).toEqual(["buildings", "topology", "groupAddresses", "comObjectFlags"]);
  });

  it("has no paragraphs for an unknown topic id", () => {
    expect(helpTopicParagraphs("no-such-topic")).toBeUndefined();
  });

  // Help text is the one place a compatibility or certification claim
  // would do real damage, because it is the text a user believes. No test
  // can check that a sentence is *true* — ADR-0024 says so outright — but
  // it can pin which keys are allowed to touch those two words at all, so
  // that a new help string mentioning either fails here and has to be
  // read by someone before it ships. Both current mentions are denials.
  const VETTED_CLAIM_KEYS = ["help.topic.importExport.p3", "help.topic.limits.p1"];

  it("mentions certification or compatibility only in the vetted English keys", () => {
    const found = Object.entries(enMessages)
      .filter(([k]) => k.startsWith("help."))
      .filter(([, v]) => /certifi|compatib/i.test(v))
      .map(([k]) => k)
      .sort();
    expect(found).toEqual([...VETTED_CLAIM_KEYS].sort());
    expect(enMessages["help.topic.limits.p1"]).toMatch(/not made by, endorsed by or certified by/);
  });

  it("mentions certification or compatibility only in the vetted German keys", () => {
    const found = Object.entries(deMessages)
      .filter(([k]) => k.startsWith("help."))
      .filter(([, v]) => /zertifiz|kompatib/i.test(v))
      .map(([k]) => k)
      .sort();
    expect(found).toEqual([...VETTED_CLAIM_KEYS].sort());
    expect(deMessages["help.topic.limits.p1"]).toMatch(/weder unterstützt noch zertifiziert/);
  });
});
