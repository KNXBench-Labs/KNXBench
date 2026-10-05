/** U20: stable node placement and curved, direction-distinct edge paths for the flow view. */
import { describe, expect, it } from "vitest";
import { FADE_MS, NODE_SPACING, QUIET_AFTER_MS, RESTING_OPACITY, edgeGeometry, edgeOpacity, placeNodes, pointOnEdge } from "./flowLayout";

const ids = (count: number) => Array.from({ length: count }, (_, i) => `n${i}`);

describe("placeNodes", () => {
  it("keeps every existing position when nodes are added", () => {
    const before = placeNodes(ids(40));
    const after = placeNodes(ids(41));
    for (const [id, position] of before) expect(after.get(id)).toEqual(position);
  });

  it("puts the first node in the centre and keeps nodes apart", () => {
    const placed = [...placeNodes(ids(300)).values()];
    expect(placed[0]).toEqual({ x: 0, y: 0 });
    let closest = Infinity;
    for (let i = 0; i < placed.length; i++) {
      for (let j = i + 1; j < placed.length; j++) {
        closest = Math.min(closest, Math.hypot(placed[i].x - placed[j].x, placed[i].y - placed[j].y));
      }
    }
    expect(closest).toBeGreaterThanOrEqual(NODE_SPACING * 0.9);
  });

  it("is deterministic", () => {
    expect([...placeNodes(ids(25))]).toEqual([...placeNodes(ids(25))]);
  });
});

describe("edgeGeometry", () => {
  const a = { x: 0, y: 0 };
  const b = { x: 300, y: 0 };

  it("bends the two directions of one pair to opposite sides", () => {
    const forward = edgeGeometry(a, b);
    const reverse = edgeGeometry(b, a);
    expect(Math.sign(forward.label.y)).toBe(-Math.sign(reverse.label.y));
    expect(forward.label.y).not.toBe(0);
  });

  it("starts and ends outside the node circles", () => {
    const { start, end } = edgeGeometry(a, b);
    expect(Math.hypot(start.x - a.x, start.y - a.y)).toBeGreaterThan(20);
    expect(Math.hypot(end.x - b.x, end.y - b.y)).toBeGreaterThan(20);
  });
});

describe("edgeOpacity", () => {
  it("stays full while active, then fades over a minute to a readable resting line", () => {
    expect(edgeOpacity(1000, 1000 + QUIET_AFTER_MS)).toBe(1);
    const halfway = edgeOpacity(1000, 1000 + QUIET_AFTER_MS + FADE_MS / 2);
    expect(halfway).toBeLessThan(1);
    expect(halfway).toBeGreaterThan(RESTING_OPACITY);
    expect(edgeOpacity(1000, 1000 + QUIET_AFTER_MS + FADE_MS)).toBe(RESTING_OPACITY);
    expect(edgeOpacity(1000, 1000 + 3_600_000)).toBe(RESTING_OPACITY);
  });

  it("shows an edge of unknown observation time as a resting line, not as fresh", () => {
    expect(edgeOpacity(Number.NEGATIVE_INFINITY, 5000)).toBe(RESTING_OPACITY);
  });
});

describe("pointOnEdge", () => {
  it("runs from the start of the curve to its end", () => {
    const geometry = edgeGeometry({ x: 0, y: 0 }, { x: 300, y: 0 });
    expect(pointOnEdge(geometry, 0)).toEqual(geometry.start);
    expect(pointOnEdge(geometry, 1)).toEqual(geometry.end);
    const middle = pointOnEdge(geometry, 0.5);
    expect(middle.x).toBeCloseTo(geometry.label.x, 5);
    expect(middle.y).toBeCloseTo((geometry.start.y + geometry.end.y) / 4 + geometry.control.y / 2, 5);
  });
});
