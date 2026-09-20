/** Tests the help system's rules: which key opens it, when it moves, and what a topic holds. */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
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
  // read by someone before it ships. The one current mention is a denial.
  // `help.topic.importExport.p3` used to be the second: it explained the
  // `.knxproj` export and disclaimed ETS compatibility for it. ADR-0028
  // withdrew the export, so the claim it had to deny no longer exists.
  const VETTED_CLAIM_KEYS = ["help.topic.limits.p1"];
  // Both languages' words over both catalogues, not one each: an English
  // sentence landing in `de.ts` is exactly what a hurried edit produces,
  // and a per-language pattern would wave it through.
  const CLAIM_WORDS = /certifi|compatib|zertifiz|kompatib/i;

  const claimKeys = (catalogue: Record<string, string>) =>
    Object.entries(catalogue)
      .filter(([k]) => k.startsWith("help."))
      .filter(([, v]) => CLAIM_WORDS.test(v))
      .map(([k]) => k)
      .sort();

  it("mentions certification or compatibility only in the vetted English keys", () => {
    expect(claimKeys(enMessages)).toEqual([...VETTED_CLAIM_KEYS].sort());
    expect(enMessages["help.topic.limits.p1"]).toMatch(/not made by, endorsed by or certified by/);
  });

  it("mentions certification or compatibility only in the vetted German keys", () => {
    expect(claimKeys(deMessages)).toEqual([...VETTED_CLAIM_KEYS].sort());
    expect(deMessages["help.topic.limits.p1"]).toMatch(/weder unterstützt noch zertifiziert/);
  });
});

// The tip's behaviour is split across two languages: `HelpTip.tsx` decides
// which classes are on the bubble, and `styles.css` decides what those
// classes mean. Tests that only assert the class name pass happily while the
// rule behind it is deleted — measured, not feared: removing
// `:not(.is-still)` and removing the `.is-open` rule each left all 640
// tests green. `motionGuard.test.ts` cannot see either, since both
// mutations keep the declaration inside the reduced-motion block and keep
// its duration in a custom property. So this reads the stylesheet, the
// same way that guard does.
describe("the help tip's half of the contract that lives in styles.css", () => {
  const css = readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), "styles.css"),
    "utf8",
  );
  const rule = (selector: string): string => {
    const at = css.indexOf(selector + " {");
    expect(at, `no rule for ${selector}`).toBeGreaterThan(-1);
    return css.slice(at + selector.length + 2, css.indexOf("}", at));
  };

  it("makes the open bubble visible, which is the only thing `is-open` does", () => {
    expect(rule(".help-tip-bubble.is-open")).toContain("opacity: 1");
  });

  it("hides the closed bubble without taking it out of the accessibility tree", () => {
    const closed = rule(".help-tip-bubble");
    expect(closed).toContain("opacity: 0");
    expect(closed).toContain("pointer-events: none");
    // ADR-0024 decision 1: both of these would remove the element from the
    // accessibility tree, and `aria-describedby` would resolve to nothing
    // while the bubble is closed.
    expect(closed).not.toContain("visibility: hidden");
    expect(closed).not.toContain("display: none");
  });

  it("exempts a bubble marked still from the transition", () => {
    // `helpTipAnimates` returning false puts `is-still` on the bubble; if
    // no rule keys off that class, the JS half of the motion setting is
    // decoration.
    expect(css).toContain(".help-tip-bubble:not(.is-still) { transition:");
  });
});
