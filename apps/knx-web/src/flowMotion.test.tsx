/** U21: motion for the flow view follows the app's Motion setting and the OS reduce preference. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { motionAllowed, useFlowMotion } from "./flowMotion";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function fakeMedia(initial: boolean) {
  const listeners = new Set<() => void>();
  const query = {
    matches: initial,
    addEventListener: (_: string, listener: () => void) => listeners.add(listener),
    removeEventListener: (_: string, listener: () => void) => listeners.delete(listener),
  };
  return {
    matchMedia: vi.fn(() => query as unknown as MediaQueryList),
    set(matches: boolean) {
      query.matches = matches;
      for (const listener of listeners) listener();
    },
    listeners,
  };
}

let root: Root | undefined;
let host: HTMLDivElement | undefined;
let seen: boolean[] = [];

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  document.documentElement.removeAttribute("data-motion-level");
  vi.unstubAllGlobals();
  seen = [];
});

function Probe() {
  seen.push(useFlowMotion());
  return null;
}

describe("motionAllowed", () => {
  it("is off when the app's Motion is Off or the OS asks to reduce motion", () => {
    expect(motionAllowed("standard", false)).toBe(true);
    expect(motionAllowed("subtle", false)).toBe(true);
    expect(motionAllowed("off", false)).toBe(false);
    expect(motionAllowed("standard", true)).toBe(false);
    expect(motionAllowed(null, false)).toBe(true);
  });
});

describe("useFlowMotion", () => {
  it("follows both sources while mounted and stops listening on unmount", async () => {
    const media = fakeMedia(false);
    vi.stubGlobal("matchMedia", media.matchMedia);
    document.documentElement.setAttribute("data-motion-level", "standard");
    host = document.createElement("div");
    document.body.appendChild(host);
    root = createRoot(host);
    await act(async () => root!.render(<Probe />));
    expect(seen.at(-1)).toBe(true);
    await act(async () => { document.documentElement.setAttribute("data-motion-level", "off"); await Promise.resolve(); });
    expect(seen.at(-1)).toBe(false);
    await act(async () => { document.documentElement.setAttribute("data-motion-level", "standard"); await Promise.resolve(); });
    expect(seen.at(-1)).toBe(true);
    await act(async () => media.set(true));
    expect(seen.at(-1)).toBe(false);
    await act(async () => root!.unmount());
    root = undefined;
    expect(media.listeners.size).toBe(0);
  });
});
