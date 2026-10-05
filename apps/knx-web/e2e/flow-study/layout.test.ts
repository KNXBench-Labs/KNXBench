/** U19 study: bounded activity layout — freeze, determinism, distance bounds, leader centring. */
import { describe, expect, it } from "vitest";
import { MAX_DISTANCE, MIN_DISTANCE, createLayout, preferredDistance, reheat, step, type LayoutEdge } from "./layout";

const box = { width: 1000, height: 800 };

describe("preferred distance", () => {
  it("shrinks with activity and stays within its bounds", () => {
    const quiet = preferredDistance(0);
    const busy = preferredDistance(5);
    const flooded = preferredDistance(10_000);
    expect(quiet).toBe(MAX_DISTANCE);
    expect(busy).toBeLessThan(quiet);
    expect(flooded).toBe(MIN_DISTANCE);
    expect(MIN_DISTANCE).toBeGreaterThan(0);
  });
});

describe("layout step", () => {
  it("is deterministic for the same seed and inputs", () => {
    const edges: LayoutEdge[] = [{ from: "a", to: "b", rate: 1 }, { from: "b", to: "c", rate: 0 }];
    const run = () => {
      const layout = createLayout(["a", "b", "c"], box, 7);
      for (let i = 0; i < 50; i += 1) step(layout, edges, { leader: "a" });
      return [...layout.nodes.values()].map((node) => [node.x, node.y]);
    };
    expect(run()).toEqual(run());
  });

  it("moves nothing while frozen", () => {
    const layout = createLayout(["a", "b"], box, 3);
    const before = [...layout.nodes.values()].map((node) => ({ ...node }));
    for (let i = 0; i < 20; i += 1) step(layout, [{ from: "a", to: "b", rate: 9 }], { leader: "a", frozen: true });
    expect([...layout.nodes.values()]).toEqual(before);
  });

  it("pulls busy pairs closer than quiet ones and the leader towards the centre", () => {
    const layout = createLayout(["lead", "busy", "x", "quiet"], box, 11);
    const edges: LayoutEdge[] = [{ from: "lead", to: "busy", rate: 8 }, { from: "x", to: "quiet", rate: 0 }];
    for (let i = 0; i < 400; i += 1) step(layout, edges, { leader: "lead" });
    const at = (id: string) => layout.nodes.get(id)!;
    const dist = (a: string, b: string) => Math.hypot(at(a).x - at(b).x, at(a).y - at(b).y);
    expect(dist("lead", "busy")).toBeLessThan(dist("x", "quiet"));
    const fromCentre = (id: string) => Math.hypot(at(id).x - box.width / 2, at(id).y - box.height / 2);
    expect(fromCentre("lead")).toBeLessThan(Math.min(fromCentre("x"), fromCentre("quiet")));
  });

  it("stays finite and inside the box for 500 devices and 2,500 edges", () => {
    const ids = Array.from({ length: 500 }, (_, i) => `d${i}`);
    const edges: LayoutEdge[] = Array.from({ length: 2500 }, (_, i) => ({
      from: ids[(i * 7) % 500], to: ids[(i * 13 + 1) % 500], rate: i % 5,
    }));
    const layout = createLayout(ids, box, 5);
    for (let i = 0; i < 60; i += 1) step(layout, edges, { leader: "d0" });
    for (const node of layout.nodes.values()) {
      expect(Number.isFinite(node.x) && Number.isFinite(node.y)).toBe(true);
      expect(node.x).toBeGreaterThanOrEqual(0);
      expect(node.x).toBeLessThanOrEqual(box.width);
      expect(node.y).toBeGreaterThanOrEqual(0);
      expect(node.y).toBeLessThanOrEqual(box.height);
    }
  });
});

describe("spread", () => {
  it("uses the available area: a small map is not packed into the centre", () => {
    const ids = Array.from({ length: 14 }, (_, i) => `n${i}`);
    const edges: LayoutEdge[] = ids.slice(1).map((id, i) => ({ from: ids[i], to: id, rate: 1 }));
    const layout = createLayout(ids, { width: 900, height: 620 }, 1);
    for (let i = 0; i < 300; i += 1) step(layout, edges, { leader: "n0" });
    const nodes = [...layout.nodes.values()];
    const nearest = nodes.map((a) => Math.min(...nodes.filter((b) => b !== a).map((b) => Math.hypot(a.x - b.x, a.y - b.y))));
    expect(Math.min(...nearest)).toBeGreaterThan(45);
    const xs = nodes.map((node) => node.x);
    expect(Math.max(...xs) - Math.min(...xs)).toBeGreaterThan(900 * 0.45);
  });
});

describe("cooling", () => {
  function movement(layout: ReturnType<typeof createLayout>, edges: LayoutEdge[], steps: number): number {
    const before = new Map([...layout.nodes].map(([id, node]) => [id, [node.x, node.y]]));
    for (let i = 0; i < steps; i += 1) step(layout, edges, { leader: "d0" });
    let total = 0;
    for (const [id, node] of layout.nodes) total += Math.hypot(node.x - before.get(id)![0], node.y - before.get(id)![1]);
    return total;
  }

  it("comes to rest without new structure even while activity rates keep changing", () => {
    const ids = Array.from({ length: 60 }, (_, i) => `d${i}`);
    const layout = createLayout(ids, box, 2);
    let edges: LayoutEdge[] = ids.slice(1).map((id, i) => ({ from: ids[i], to: id, rate: 1 }));
    movement(layout, edges, 400);
    edges = edges.map((edge, i) => ({ ...edge, rate: (i * 3) % 7 }));
    expect(movement(layout, edges, 30)).toBeLessThan(1);
  });

  it("moves again after a reheat, e.g. when a node or edge appears", () => {
    const ids = Array.from({ length: 30 }, (_, i) => `d${i}`);
    const layout = createLayout(ids, box, 4);
    const edges: LayoutEdge[] = ids.slice(1).map((id, i) => ({ from: ids[i], to: id, rate: 2 }));
    movement(layout, edges, 400);
    reheat(layout, 0.5);
    expect(movement(layout, [...edges, { from: "d0", to: "d29", rate: 9 }], 30)).toBeGreaterThan(5);
  });
});
