/** U21: activity layout: bounds, freeze, determinism, leader, cooling (from U19). */
import { describe, expect, it } from "vitest";
import {
  MAX_DISTANCE,
  MAX_STEP,
  MIN_DISTANCE,
  createDynamics,
  ensureDynamicNodes,
  preferredDistance,
  reheat,
  reheatAround,
  step,
  type DynamicEdge,
} from "./flowDynamics";
import { NODE_RADIUS, nodeFootprint, placeNodes } from "./flowLayout";

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

describe("local reheat (§9.3)", () => {
  // Two separate chains far apart; only the second one gets a new edge.
  function settledTwoChains() {
    const left = Array.from({ length: 20 }, (_, i) => `l${i}`);
    const right = Array.from({ length: 20 }, (_, i) => `r${i}`);
    const layout = createDynamics([...left, ...right], box);
    const edges: DynamicEdge[] = [
      ...left.slice(1).map((id, i) => ({ from: left[i], to: id, rate: 2 })),
      ...right.slice(1).map((id, i) => ({ from: right[i], to: id, rate: 2 })),
    ];
    for (let i = 0; i < 600; i += 1) step(layout, edges, { leader: null });
    expect(layout.alpha).toBe(0);
    return { layout, edges, left, right };
  }

  it("heats a new edge's endpoints and their neighbours, and nothing else moves", () => {
    const { layout, edges, left } = settledTwoChains();
    const grown = [...edges, { from: "r0", to: "r10", rate: 9 }];
    const before = new Map([...layout.nodes].map(([id, n]) => [id, [n.x, n.y]]));
    reheatAround(layout, ["r0", "r10"], grown, 0.5);
    const hot = new Set(["r0", "r10", "r1", "r9", "r11"]);
    for (const [id, node] of layout.nodes) expect(node.heat > 0, id).toBe(hot.has(id));
    const moved = new Set<string>();
    for (let i = 0; i < 40; i += 1) for (const id of step(layout, grown, { leader: null })) moved.add(id);
    for (const id of moved) expect(hot.has(id), id).toBe(true);
    for (const id of left) expect([layout.nodes.get(id)!.x, layout.nodes.get(id)!.y]).toEqual(before.get(id));
    expect(Math.hypot(layout.nodes.get("r0")!.x - before.get("r0")![0], layout.nodes.get("r0")!.y - before.get("r0")![1])).toBeGreaterThan(1);
  });

  it("reports no moved node once everything has cooled", () => {
    const { layout, edges } = settledTwoChains();
    expect(step(layout, edges, { leader: null }).size).toBe(0);
  });

  it("lets a settled node gather no momentum from a hot neighbour, so a later reheat starts at rest", () => {
    const { layout, edges } = settledTwoChains();
    // Only r5 is hot; its springs to the settled r4 and r6 must not charge them up.
    layout.nodes.get("r5")!.heat = 0.5;
    layout.alpha = 0.5;
    for (let i = 0; i < 20; i += 1) step(layout, edges, { leader: null });
    for (const id of ["r4", "r6"]) expect([layout.nodes.get(id)!.vx, layout.nodes.get(id)!.vy], id).toEqual([0, 0]);
  });
});

describe("hub readability (§9.3)", () => {
  // A busy hub with 24 active receivers, as in the AR21 screenshots: every
  // pair prefers the shortest distance, which is less than two node radii.
  function settledStar(box = { width: 960, height: 520 }) {
    const receivers = Array.from({ length: 24 }, (_, i) => `r${i}`);
    const layout = createDynamics(["hub", ...receivers], box);
    const edges: DynamicEdge[] = receivers.map((id) => ({ from: "hub", to: id, rate: 20 }));
    const badgeLines = new Map([["hub", 4], ...receivers.map((id) => [id, 1] as [string, number])]);
    for (let i = 0; i < 2_000 && layout.alpha > 0; i += 1) step(layout, edges, { leader: "hub", badgeLines });
    expect(layout.alpha).toBe(0);
    return { layout, badgeLines };
  }

  it("settles a busy star with no node drawn over another", () => {
    const { layout } = settledStar();
    const nodes = [...layout.nodes.values()];
    for (const a of nodes) {
      for (const b of nodes) {
        if (a === b) continue;
        expect(Math.hypot(a.x - b.x, a.y - b.y), `${a.id}–${b.id}`).toBeGreaterThanOrEqual(2 * NODE_RADIUS);
      }
    }
  });

  it("keeps every label and value block of a busy star clear of its neighbours", () => {
    const { layout, badgeLines } = settledStar();
    const box = (id: string) => {
      const node = layout.nodes.get(id)!;
      const f = nodeFootprint(badgeLines.get(id) ?? 0);
      return { left: node.x - f.halfWidth, right: node.x + f.halfWidth, top: node.y - f.top, bottom: node.y + f.bottom };
    };
    const ids = [...layout.nodes.keys()];
    for (const a of ids) {
      for (const b of ids) {
        if (a >= b) continue;
        const p = box(a);
        const q = box(b);
        const overlapX = Math.min(p.right, q.right) - Math.max(p.left, q.left);
        const overlapY = Math.min(p.bottom, q.bottom) - Math.max(p.top, q.top);
        // A pixel of slack: the solver stops once nothing moves visibly.
        expect(Math.min(overlapX, overlapY), `${a}–${b}`).toBeLessThan(1);
      }
    }
  });

  it("keeps every name and circle inside the drawing area", () => {
    const { layout } = settledStar();
    const f = nodeFootprint(0);
    for (const node of layout.nodes.values()) {
      expect(node.y - f.top, node.id).toBeGreaterThanOrEqual(-layout.height / 2);
      expect(node.y + NODE_RADIUS, node.id).toBeLessThanOrEqual(layout.height / 2);
      expect(Math.abs(node.x) + f.halfWidth, node.id).toBeLessThanOrEqual(layout.width / 2);
    }
  });

  it("moves no node further than one step allows, even out of a crowd", () => {
    // Thirty nodes on top of each other: every neighbour pushes, the sum must not jump.
    const ids = Array.from({ length: 30 }, (_, i) => `n${i}`);
    const layout = createDynamics(ids, { width: 960, height: 520 });
    for (const node of layout.nodes.values()) {
      node.x = 0;
      node.y = 0;
    }
    step(layout, [], { leader: null });
    for (const node of layout.nodes.values()) expect(Math.hypot(node.x, node.y), node.id).toBeLessThanOrEqual(2 * MAX_STEP + 1e-9);
  });

  function separation(badgeLines: number): number {
    const layout = createDynamics(["hub", "near"], box);
    layout.nodes.get("hub")!.x = 0;
    layout.nodes.get("hub")!.y = 0;
    layout.nodes.get("near")!.x = 0;
    layout.nodes.get("near")!.y = 30;
    const extents = new Map([["hub", badgeLines]]);
    for (let i = 0; i < 300; i += 1) step(layout, [], { leader: null, badgeLines: extents });
    const a = layout.nodes.get("hub")!;
    const b = layout.nodes.get("near")!;
    return Math.hypot(a.x - b.x, a.y - b.y);
  }

  it("keeps a neighbour below a busy node clear of its value lines", () => {
    // Straight below, the neighbour's name must end up under the last value line.
    expect(separation(4)).toBeGreaterThanOrEqual(nodeFootprint(4).bottom + nodeFootprint(0).top);
    expect(separation(4)).toBeGreaterThan(separation(0));
  });
});
