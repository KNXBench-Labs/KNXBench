/** KL-60: which rows of a long variable-height list a scroll viewport has to render. */
import { describe, expect, it } from "vitest";
import { virtualWindow } from "./virtualWindow";

const uniform = (count: number, height: number) => Array.from({ length: count }, () => height);

describe("virtualWindow", () => {
  it("renders nothing for an empty list", () => {
    expect(virtualWindow([], 0, 400, 0)).toEqual({ start: 0, end: 0, before: 0, after: 0 });
  });

  it("covers exactly the visible rows at the top", () => {
    expect(virtualWindow(uniform(100, 10), 0, 50, 0)).toEqual({ start: 0, end: 5, before: 0, after: 950 });
  });

  it("starts at the row the viewport cuts into, and pads the rest", () => {
    expect(virtualWindow(uniform(100, 10), 205, 50, 0)).toEqual({ start: 20, end: 26, before: 200, after: 740 });
  });

  it("clamps a scroll position past the end to the last rows", () => {
    expect(virtualWindow(uniform(100, 10), 1_000_000, 50, 0)).toEqual({ start: 95, end: 100, before: 950, after: 0 });
  });

  it("follows measured heights of individual rows", () => {
    expect(virtualWindow([10, 100, 10, 10], 50, 20, 0)).toEqual({ start: 1, end: 2, before: 10, after: 20 });
  });

  it("extends the window by the overscan on both sides", () => {
    expect(virtualWindow(uniform(100, 10), 500, 50, 30)).toEqual({ start: 47, end: 58, before: 470, after: 420 });
  });

  it("keeps every row reachable: the windows of all scroll positions cover the whole list", () => {
    const heights = Array.from({ length: 500 }, (_, i) => 8 + (i % 7) * 13);
    const total = heights.reduce((sum, h) => sum + h, 0);
    const seen = new Set<number>();
    for (let top = 0; top <= total; top += 97) {
      const w = virtualWindow(heights, top, 300, 0);
      expect(w.before + heights.slice(w.start, w.end).reduce((s, h) => s + h, 0) + w.after).toBe(total);
      for (let i = w.start; i < w.end; i++) seen.add(i);
    }
    expect(seen.size).toBe(500);
  });
});
