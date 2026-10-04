/** U20: stable node placement and curved, direction-distinct edge paths for the flow view. */
import { describe, expect, it } from "vitest";
import { NODE_SPACING, edgeGeometry, placeNodes } from "./flowLayout";

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
