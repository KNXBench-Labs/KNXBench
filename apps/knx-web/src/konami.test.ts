/** Unit tests for the Konami-code matcher and its typing-target guard. */
// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import { createKonamiMatcher, installKonamiListener, isTypingTarget, KONAMI_SEQUENCE } from "./konami";

function feed(keys: readonly string[]): number {
  const matcher = createKonamiMatcher();
  return keys.filter((key) => matcher(key)).length;
}

describe("createKonamiMatcher", () => {
  it("matches the full sequence exactly once", () => {
    expect(feed(KONAMI_SEQUENCE)).toBe(1);
  });

  it("does not match a sequence with one key wrong", () => {
    const wrong = [...KONAMI_SEQUENCE];
    wrong[4] = "ArrowRight";
    expect(feed(wrong)).toBe(0);
  });

  it("matches after a stutter of extra leading arrows", () => {
    expect(feed(["ArrowUp", "ArrowUp", ...KONAMI_SEQUENCE])).toBe(1);
  });

  it("accepts B and A with Shift or Caps Lock", () => {
    expect(feed([...KONAMI_SEQUENCE.slice(0, 8), "B", "A"])).toBe(1);
  });

  it("starts over after a match", () => {
    expect(feed([...KONAMI_SEQUENCE, ...KONAMI_SEQUENCE])).toBe(2);
    expect(feed([...KONAMI_SEQUENCE, "b", "a"])).toBe(1);
  });
});

describe("isTypingTarget", () => {
  it("treats text fields, selects and editable content as typing", () => {
    expect(isTypingTarget(document.createElement("input"))).toBe(true);
    expect(isTypingTarget(document.createElement("textarea"))).toBe(true);
    expect(isTypingTarget(document.createElement("select"))).toBe(true);
    const editable = document.createElement("div");
    editable.contentEditable = "true";
    expect(isTypingTarget(editable)).toBe(true);
  });

  it("does not treat buttons, plain elements or the document as typing", () => {
    expect(isTypingTarget(document.createElement("button"))).toBe(false);
    expect(isTypingTarget(document.createElement("div"))).toBe(false);
    expect(isTypingTarget(document.body)).toBe(false);
    expect(isTypingTarget(null)).toBe(false);
  });
});

describe("installKonamiListener", () => {
  function press(target: EventTarget, key: string) {
    target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  }

  it("fires on the sequence typed at the workbench and stops after cleanup", () => {
    let hits = 0;
    const cleanup = installKonamiListener(document, () => hits++);
    for (const key of KONAMI_SEQUENCE) press(document.body, key);
    expect(hits).toBe(1);
    cleanup();
    for (const key of KONAMI_SEQUENCE) press(document.body, key);
    expect(hits).toBe(1);
  });

  it("ignores the sequence typed into an input field", () => {
    let hits = 0;
    const cleanup = installKonamiListener(document, () => hits++);
    const input = document.createElement("input");
    document.body.append(input);
    for (const key of KONAMI_SEQUENCE) press(input, key);
    cleanup();
    input.remove();
    expect(hits).toBe(0);
  });
});
