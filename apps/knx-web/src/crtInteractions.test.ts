/** Verifies bounded CRT feedback, cancellation and decorative portal clipping. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { installCrtInteractions, requestCrtActivation } from "./crtInteractions";

let scope: HTMLElement;
let button: HTMLButtonElement;
let dispose: () => void;
let media: MediaQueryList;
let onMediaChange: (() => void) | undefined;
const root = document.documentElement;

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "performance"] });
  root.dataset.motionStyle = "crt";
  root.dataset.motionLevel = "standard";
  root.style.setProperty("--knx-transition-duration", "250ms");
  root.style.setProperty("--knx-feedback-duration", "120ms");
  media = {
    matches: false,
    addEventListener: vi.fn((_type, listener) => { onMediaChange = listener as () => void; }),
    removeEventListener: vi.fn(),
  } as unknown as MediaQueryList;
  vi.spyOn(window, "matchMedia").mockReturnValue(media);
  scope = document.createElement("main");
  scope.innerHTML = '<button data-crt-surface="tree" data-crt-activate>Address</button><input type="checkbox">';
  document.body.append(scope);
  button = scope.querySelector("button")!;
  vi.spyOn(button, "getBoundingClientRect").mockReturnValue(new DOMRect(16, 24, 240, 32));
  dispose = installCrtInteractions(scope);
});

afterEach(() => {
  dispose();
  document.body.replaceChildren();
  delete root.dataset.motionLevel;
  delete root.dataset.motionStyle;
  root.removeAttribute("style");
  onMediaChange = undefined;
  vi.restoreAllMocks();
  vi.useRealTimers();
});

const pulse = () => button.hasAttribute("data-crt-flash");
const beam = () => document.querySelector<HTMLElement>(".crt-interaction-light");

it("flashes a native activation without consuming it, then retires feedback", () => {
  const action = vi.fn();
  button.addEventListener("click", action);
  button.click();
  expect(action).toHaveBeenCalledTimes(1);
  expect(pulse()).toBe(true);
  vi.advanceTimersByTime(120);
  expect(pulse()).toBe(false);
});

it("accepts the same presentation-only signal from Save's actual request path", () => {
  button.dataset.crtSurface = "save";
  button.removeAttribute("data-crt-activate");
  button.click();
  expect(pulse()).toBe(false);
  requestCrtActivation(button);
  expect(pulse()).toBe(true);
  expect(beam()).toBeNull();
});

it("bounds repeated flashes without throttling the native action", () => {
  const action = vi.fn();
  button.addEventListener("click", action);
  button.click();
  vi.advanceTimersByTime(130);
  button.click();
  expect(action).toHaveBeenCalledTimes(2);
  expect(pulse()).toBe(false);
  vi.advanceTimersByTime(600);
  button.click();
  expect(pulse()).toBe(true);
});

it("does not flash checkbox changes or disabled controls", () => {
  scope.querySelector<HTMLInputElement>("input")!.click();
  expect(pulse()).toBe(false);
  button.disabled = true;
  requestCrtActivation(button);
  expect(pulse()).toBe(false);
});

it("shows one inert light sweep outside the table and retires it", () => {
  button.dispatchEvent(new MouseEvent("pointerover", { bubbles: true }));
  expect(beam()?.getAttribute("aria-hidden")).toBe("true");
  expect(beam()?.hasAttribute("inert")).toBe(true);
  expect(beam()?.parentElement).toBe(document.body);
  expect(beam()?.style.width).toBe("240px");
  expect(beam()?.querySelector(".crt-beam-core")).not.toBeNull();
  vi.advanceTimersByTime(250);
  expect(beam()).toBeNull();
});

it("aligns the sweep to root UI zoom", () => {
  root.style.setProperty("--app-ui-scale", "2");
  button.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  expect(beam()?.style.left).toBe(`${16 / 2}px`);
  expect(beam()?.style.top).toBe(`${24 / 2}px`);
  expect(beam()?.style.width).toBe(`${240 / 2}px`);
});

it("clips the decorative portal to visible scroll-container bounds", () => {
  const clip = document.createElement("div");
  clip.style.overflowX = "hidden";
  clip.style.overflowY = "auto";
  vi.spyOn(clip, "getBoundingClientRect").mockReturnValue(new DOMRect(50, 30, 80, 10));
  clip.append(button);
  scope.append(clip);
  button.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  expect(beam()?.style.left).toBe("50px");
  expect(beam()?.style.top).toBe("30px");
  expect(beam()?.style.width).toBe("80px");
  expect(beam()?.style.height).toBe("10px");
});

it.each(["off", "subtle"])("cancels already-running effects when level becomes %s", async (level) => {
  button.click();
  button.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  expect(pulse()).toBe(true);
  expect(beam()).not.toBeNull();
  root.dataset.motionLevel = level;
  await vi.advanceTimersByTimeAsync(0);
  expect(pulse()).toBe(false);
  expect(beam()).toBeNull();
});

it("retires an anchor removed by filtering or collapsing", async () => {
  button.click();
  button.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  expect(beam()).not.toBeNull();
  button.remove();
  await vi.advanceTimersByTimeAsync(0);
  expect(pulse()).toBe(false);
  expect(beam()).toBeNull();
});

it.each(["off", "subtle"])("does not start transient effects at level %s", (level) => {
  root.dataset.motionLevel = level;
  button.click();
  button.dispatchEvent(new MouseEvent("pointerover", { bubbles: true }));
  expect(pulse()).toBe(false);
  expect(beam()).toBeNull();
});

it.each(["apple", "glitch", "unknown"])("does not affect style %s", (style) => {
  root.dataset.motionStyle = style;
  button.click();
  button.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  expect(pulse()).toBe(false);
  expect(beam()).toBeNull();
});

it("immediately cancels live feedback when reduced motion changes", () => {
  button.click();
  button.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  expect(pulse()).toBe(true);
  expect(beam()).not.toBeNull();
  Object.defineProperty(media, "matches", { value: true });
  onMediaChange!();
  expect(pulse()).toBe(false);
  expect(beam()).toBeNull();
});

it("does not create effects under an initial reduced-motion preference", () => {
  Object.defineProperty(media, "matches", { value: true });
  button.click();
  expect(pulse()).toBe(false);
});

it("cancels on resize, scroll, drag and loss of focus", () => {
  for (const event of [new Event("resize"), new Event("scroll"), new Event("dragstart")]) {
    vi.advanceTimersByTime(600);
    button.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    expect(beam()).not.toBeNull();
    (event.type === "resize" ? window : scope).dispatchEvent(event);
    expect(beam()).toBeNull();
  }
  vi.advanceTimersByTime(600);
  button.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  expect(beam()).not.toBeNull();
  window.dispatchEvent(new Event("blur"));
  expect(beam()).toBeNull();
});

it("removes feedback, listeners and media subscription on disposal", () => {
  button.click();
  button.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  dispose();
  expect(pulse()).toBe(false);
  expect(beam()).toBeNull();
  expect(media.removeEventListener).toHaveBeenCalled();
  vi.advanceTimersByTime(1000);
  button.click();
  expect(pulse()).toBe(false);
});

describe("duration admission", () => {
  it.each(["0ms", "-10ms", "garbage", "Infinityms"])("rejects %s", (duration) => {
    root.style.setProperty("--knx-transition-duration", duration);
    button.click();
    expect(pulse()).toBe(false);
  });
  it("accepts seconds and caps a corrupted long duration", () => {
    root.style.setProperty("--knx-transition-duration", "0.25s");
    root.style.setProperty("--knx-feedback-duration", "9s");
    button.click();
    expect(pulse()).toBe(true);
    vi.advanceTimersByTime(120);
    expect(pulse()).toBe(false);
  });
});
