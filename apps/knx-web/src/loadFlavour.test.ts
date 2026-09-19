/** Tests the flavour line's catalogue, its injectable shuffle and its reduced-motion rule. */
import { describe, expect, it } from "vitest";
import { FLAVOUR_INTERVAL_MS, FLAVOUR_KEYS, flavourRotates, shuffleFlavourKeys } from "./loadFlavour";
import { messages as enMessages } from "./messages/en";
import { messages as deMessages } from "./messages/de";

describe("FLAVOUR_KEYS", () => {
  it("names fifty distinct keys, which is what the user asked for", () => {
    expect(FLAVOUR_KEYS).toHaveLength(50);
    expect(new Set(FLAVOUR_KEYS).size).toBe(50);
  });

  // `de.ts` is typed against `en.ts`, so a *missing* German key is a
  // compile error long before this runs. What the compiler cannot see is
  // an entry left as an empty string, or an English line copied into the
  // German column and never translated.
  it("has a non-empty English and German line for every one of them", () => {
    for (const key of FLAVOUR_KEYS) {
      expect(enMessages[key], key).toBeTruthy();
      expect(deMessages[key], key).toBeTruthy();
    }
  });

  it("does not leave a German line identical to its English one", () => {
    const untranslated = FLAVOUR_KEYS.filter((key) => deMessages[key] === enMessages[key]);
    expect(untranslated).toEqual([]);
  });

  it("keeps the joke in entry 18 by keeping 230 V exactly", () => {
    expect(enMessages["loadProgress.flavour.18"]).toContain("230 V");
    expect(deMessages["loadProgress.flavour.18"]).toContain("230 V");
  });
});

describe("shuffleFlavourKeys", () => {
  // The whole point of the injected source: the same dice give the same
  // order, every time, in every test. A `Math.random()` inside the
  // component would make this assertion either flaky or meaningless.
  it("is a pure function of the dice it is handed", () => {
    const dice = () => 0;
    expect(shuffleFlavourKeys(dice)).toEqual(shuffleFlavourKeys(dice));
  });

  it("produces the one order a constant zero can produce", () => {
    const order = shuffleFlavourKeys(() => 0);
    // Fisher-Yates with j always 0 is a left rotation by one.
    expect(order[0]).toBe("loadProgress.flavour.02");
    expect(order[1]).toBe("loadProgress.flavour.03");
    expect(order[49]).toBe("loadProgress.flavour.01");
  });

  // The other half of the pair `App.test.tsx` leans on: it pins
  // `Math.random` to 0 for one load and to a value just under 1 for the
  // next, and reads the two opening lines off the banner. Both facts
  // belong here, at the level that owns the arithmetic. A correct,
  // behaviour-preserving rewrite of the shuffle — forward iteration, a
  // different clamp — could leave that App test red with nothing actually
  // broken; this test is where it would be diagnosed.
  it("leaves the order untouched when every draw picks the candidate in place", () => {
    // A draw just under 1 selects `j === i`, so every swap is a no-op and
    // the permutation is the identity. `Math.random` never returns 1;
    // this is as close as a real source gets.
    expect(shuffleFlavourKeys(() => 0.9999999)).toEqual([...FLAVOUR_KEYS]);
    expect(shuffleFlavourKeys(() => 0.9999999)[0]).toBe("loadProgress.flavour.01");
  });

  it("gives two different sources two different orders", () => {
    let n = 0;
    const scripted = shuffleFlavourKeys(() => [0.5, 0.25, 0.9, 0, 0.75][n++ % 5]);
    expect(scripted[0]).toBe("loadProgress.flavour.14");
    expect(scripted).not.toEqual(shuffleFlavourKeys(() => 0));
  });

  it("keeps every key, loses none and invents none", () => {
    expect([...shuffleFlavourKeys(() => 0.42)].sort()).toEqual([...FLAVOUR_KEYS].sort());
  });

  // `Math.random()` never returns 1, but an injected stub can, and an
  // unclamped index would write `undefined` into the list.
  it("survives a source that returns exactly 1", () => {
    const order = shuffleFlavourKeys(() => 1);
    expect(order).toHaveLength(50);
    expect(order.every((key) => typeof key === "string")).toBe(true);
  });

  it("does not hand back the module's own array to be mutated", () => {
    const order = shuffleFlavourKeys(() => 0);
    order[0] = "loadProgress.flavour.50";
    expect(FLAVOUR_KEYS[0]).toBe("loadProgress.flavour.01");
  });
});

describe("flavourRotates", () => {
  const media = (matches: boolean) => ({ matchMedia: () => ({ matches }) }) as unknown as Window;
  const level = (value: string | null) => ({ getAttribute: () => value });

  it("rotates when neither switch asks for stillness", () => {
    expect(flavourRotates(media(false), level("standard"))).toBe(true);
  });

  it("stops for the operating system's prefers-reduced-motion", () => {
    expect(flavourRotates(media(false), level("standard"))).toBe(true);
    expect(flavourRotates(media(true), level("standard"))).toBe(false);
  });

  it("stops for the application's own Motion: off, whatever the OS says", () => {
    expect(flavourRotates(media(false), level("off"))).toBe(false);
  });

  it("treats a runtime without matchMedia as one that never asked", () => {
    expect(flavourRotates({} as unknown as Window, level(null))).toBe(true);
  });
});

describe("FLAVOUR_INTERVAL_MS", () => {
  // Slower than the 250 ms poll on purpose: the phase label is the news,
  // and a joke that changes faster than the news competes with it.
  it("is slower than the load poll, so the phase stays the thing being read", () => {
    expect(FLAVOUR_INTERVAL_MS).toBe(1800);
  });
});
