/** Pins when the explorer's structure lines pulse, and when they stay still. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TREE_PULSE_INTERVAL_MS, TREE_PULSE_WINDOW_MS, treePulseAllowed, useTreePulse, visibleRowIndexes } from "./treeLines";

function doc(hidden: boolean, lines: string | null) {
  return { hidden, documentElement: { getAttribute: (name: string) => (name === "data-motion-lines" ? lines : null) } } as unknown as Document;
}

describe("treePulseAllowed", () => {
  it("pulses only in a visible tab at standard structure-line motion", () => {
    expect(treePulseAllowed(doc(false, "standard"))).toBe(true);
    expect(treePulseAllowed(doc(true, "standard"))).toBe(false);
    expect(treePulseAllowed(doc(false, "subtle"))).toBe(false);
    expect(treePulseAllowed(doc(false, "off"))).toBe(false);
    expect(treePulseAllowed(doc(false, null))).toBe(false);
  });
});

describe("visibleRowIndexes", () => {
  it("keeps only rows that overlap the visible part of the explorer", () => {
    const rows = [{ top: -40, bottom: -10 }, { top: -10, bottom: 10 }, { top: 10, bottom: 40 }, { top: 290, bottom: 310 }, { top: 300, bottom: 330 }];
    expect(visibleRowIndexes(rows, { top: 0, bottom: 300 })).toEqual([1, 2, 3]);
    expect(visibleRowIndexes([], { top: 0, bottom: 300 })).toEqual([]);
  });
});

describe("useTreePulse", () => {
  let host: HTMLDivElement;
  const seen: boolean[] = [];
  function Probe() {
    const pulsing = useTreePulse();
    seen.push(pulsing);
    return <span data-pulsing={String(pulsing)} />;
  }
  beforeEach(() => {
    vi.useFakeTimers();
    seen.length = 0;
    host = document.createElement("div");
    document.body.append(host);
    document.documentElement.setAttribute("data-motion-lines", "standard");
  });
  afterEach(() => {
    host.remove();
    document.documentElement.removeAttribute("data-motion-lines");
    vi.useRealTimers();
  });
  const state = () => host.querySelector("span")!.getAttribute("data-pulsing");

  it("raises the pulse every interval for one window, and keeps it down in between", async () => {
    const root = createRoot(host);
    await act(async () => root.render(<Probe />));
    expect(state()).toBe("false");
    await act(async () => vi.advanceTimersByTime(TREE_PULSE_INTERVAL_MS - 1));
    expect(state()).toBe("false");
    await act(async () => vi.advanceTimersByTime(1));
    expect(state()).toBe("true");
    await act(async () => vi.advanceTimersByTime(TREE_PULSE_WINDOW_MS));
    expect(state()).toBe("false");
    await act(async () => vi.advanceTimersByTime(TREE_PULSE_INTERVAL_MS - TREE_PULSE_WINDOW_MS));
    expect(state()).toBe("true");
    await act(async () => root.unmount());
  });

  it("stays down while structure-line motion is subtle or off", async () => {
    const root = createRoot(host);
    await act(async () => root.render(<Probe />));
    for (const level of ["subtle", "off"]) {
      document.documentElement.setAttribute("data-motion-lines", level);
      await act(async () => vi.advanceTimersByTime(TREE_PULSE_INTERVAL_MS));
      expect(state()).toBe("false");
    }
    expect(seen.every((pulsing) => !pulsing)).toBe(true);
    await act(async () => root.unmount());
  });

  it("leaves no timer behind once the explorer is gone", async () => {
    const root = createRoot(host);
    await act(async () => root.render(<Probe />));
    await act(async () => root.unmount());
    expect(vi.getTimerCount()).toBe(0);
  });
});
