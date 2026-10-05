/** U21: activity layout: bounds, freeze, determinism, leader, cooling (from U19). */
import { describe, expect, it } from "vitest";
import {
  MAX_DISTANCE,
  MIN_DISTANCE,
  createDynamics,
  ensureDynamicNodes,
  preferredDistance,
  reheat,
  step,
  type DynamicEdge,
} from "./flowDynamics";
import { placeNodes } from "./flowLayout";

const box = { width: 1000, height: 800 };

describe("preferred distance", () => {
  it("shrinks with activity and stays within its bounds", () => {
    expect(preferredDistance(0)).toBe(MAX_DISTANCE);
    expect(preferredDistance(5)).toBeLessThan(MAX_DISTANCE);
    expect(preferredDistance(10_000)).toBe(MIN_DISTANCE);
    expect(MIN_DISTANCE).toBeGreaterThan(0);
  });
});

describe("dynamics", () => {
  it("starts from the static hex positions, so motion Off and On begin alike", () => {
    const ids = ["a", "b", "c", "d"];
    const layout = createDynamics(ids, box);
    const hex = placeNodes(ids);
    for (const id of ids) expect({ x: layout.nodes.get(id)!.x, y: layout.nodes.get(id)!.y }).toEqual(hex.get(id));
  });

  it("is deterministic for the same inputs", () => {
    const edges: DynamicEdge[] = [{ from: "a", to: "b", rate: 1 }, { from: "b", to: "c", rate: 0 }];
    const run = () => {
      const layout = createDynamics(["a", "b", "c"], box);
      for (let i = 0; i < 50; i += 1) step(layout, edges, { leader: "a" });
      return [...layout.nodes.values()].map((node) => [node.x, node.y]);
    };
    expect(run()).toEqual(run());
  });

  it("moves nothing while frozen", () => {
    const layout = createDynamics(["a", "b"], box);
    const before = [...layout.nodes.values()].map((node) => ({ ...node }));
    for (let i = 0; i < 20; i += 1) step(layout, [{ from: "a", to: "b", rate: 9 }], { leader: "a", frozen: true });
    expect([...layout.nodes.values()]).toEqual(before);
  });

  it("pulls busy pairs closer than quiet ones and the leader towards the centre", () => {
    const layout = createDynamics(["x", "quiet", "lead", "busy"], box);
    const edges: DynamicEdge[] = [{ from: "lead", to: "busy", rate: 8 }, { from: "x", to: "quiet", rate: 0 }];
    for (let i = 0; i < 400; i += 1) step(layout, edges, { leader: "lead" });
    const at = (id: string) => layout.nodes.get(id)!;
    const dist = (a: string, b: string) => Math.hypot(at(a).x - at(b).x, at(a).y - at(b).y);
    expect(dist("lead", "busy")).toBeLessThan(dist("x", "quiet"));
    const fromCentre = (id: string) => Math.hypot(at(id).x, at(id).y);
    expect(fromCentre("lead")).toBeLessThan(Math.min(fromCentre("x"), fromCentre("quiet")));
  });

  it("stays finite and inside the box for 500 devices and 2,500 edges", () => {
    const ids = Array.from({ length: 500 }, (_, i) => `d${i}`);
    const edges: DynamicEdge[] = Array.from({ length: 2500 }, (_, i) => ({ from: ids[(i * 7) % 500], to: ids[(i * 13 + 1) % 500], rate: i % 5 }));
    const layout = createDynamics(ids, box);
    for (let i = 0; i < 60; i += 1) step(layout, edges, { leader: "d0" });
    for (const node of layout.nodes.values()) {
      expect(Number.isFinite(node.x) && Number.isFinite(node.y)).toBe(true);
      expect(Math.abs(node.x)).toBeLessThanOrEqual(box.width / 2);
      expect(Math.abs(node.y)).toBeLessThanOrEqual(box.height / 2);
    }
  });

  it("keeps existing positions when nodes are added", () => {
    const layout = createDynamics(["a", "b"], box);
    for (let i = 0; i < 30; i += 1) step(layout, [{ from: "a", to: "b", rate: 3 }], { leader: "a" });
    const before = { ...layout.nodes.get("a")! };
    ensureDynamicNodes(layout, ["a", "b", "c"]);
    expect(layout.nodes.get("a")).toEqual(before);
    expect(layout.nodes.has("c")).toBe(true);
  });
});

describe("cooling", () => {
  function movement(layout: ReturnType<typeof createDynamics>, edges: DynamicEdge[], steps: number): number {
    const before = new Map([...layout.nodes].map(([id, node]) => [id, [node.x, node.y]]));
    for (let i = 0; i < steps; i += 1) step(layout, edges, { leader: "d0" });
    let total = 0;
    for (const [id, node] of layout.nodes) total += Math.hypot(node.x - before.get(id)![0], node.y - before.get(id)![1]);
    return total;
  }

  it("comes to rest without new structure even while activity rates keep changing", () => {
    const ids = Array.from({ length: 60 }, (_, i) => `d${i}`);
    const layout = createDynamics(ids, box);
    let edges: DynamicEdge[] = ids.slice(1).map((id, i) => ({ from: ids[i], to: id, rate: 1 }));
    movement(layout, edges, 400);
    edges = edges.map((edge, i) => ({ ...edge, rate: (i * 3) % 7 }));
    expect(movement(layout, edges, 30)).toBeLessThan(1);
    expect(layout.alpha).toBe(0);
  });

  it("moves again after a reheat, e.g. when a node or edge appears", () => {
    const ids = Array.from({ length: 30 }, (_, i) => `d${i}`);
    const layout = createDynamics(ids, box);
    const edges: DynamicEdge[] = ids.slice(1).map((id, i) => ({ from: ids[i], to: id, rate: 2 }));
    movement(layout, edges, 400);
    reheat(layout, 0.5);
    expect(movement(layout, [...edges, { from: "d0", to: "d29", rate: 9 }], 30)).toBeGreaterThan(5);
  });
});
