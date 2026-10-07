/** Checks readable layout, camera bounds and cross-document value age without hardware. */
import { describe, expect, it } from "vitest";
import { readableLayout, fitFlowCamera, cloneFlowAtOrigin, routeFlowEdge } from "./flowPresentation";
import { createFlowModel, currentBadges } from "./flowModel";
import { pointOnEdge, edgeGeometry, placeNodes, type Point } from "./flowLayout";

const box = { width: 1000, height: 600 };
describe("flow presentation contracts", () => {
  it("routes a curved edge around a foreign node's complete label/value footprint", () => {
    const obstacle = { x: 0, y: 60, halfWidth: 60, top: 48, bottom: 80 };
    const curve = routeFlowEdge({ x: -300, y: 0 }, { x: 300, y: 0 }, [obstacle]);
    for (let i = 1; i < 40; i++) {
      const p = pointOnEdge(curve, i / 40);
      expect(Math.abs(p.x) < 60 && p.y > 12 && p.y < 140).toBe(false);
    }
  });
  it("uses label widths and separate component lanes instead of clamping a crowd into a small box", () => {
    const nodes = Array.from({ length: 50 }, (_, i) => ({ id: String(i), label: "A very long device name " + i }));
    const at = readableLayout(nodes, [], box);
    expect(at.size).toBe(50);
    for (const a of nodes) for (const b of nodes) {
      if (a.id >= b.id) continue;
      const p = at.get(a.id)!; const q = at.get(b.id)!;
      expect(Math.abs(p.x - q.x) >= 230 || Math.abs(p.y - q.y) >= 150).toBe(true);
    }
    expect(Math.max(...[...at.values()].map(p => p.y)) - Math.min(...[...at.values()].map(p => p.y))).toBeGreaterThan(520);
  });
  it("keeps a large fan-out compact instead of shrinking a single extremely tall column", () => {
    const nodes = Array.from({ length: 25 }, (_, i) => ({ id: String(i), label: `Device ${i}` }));
    const edges = nodes.slice(1).map(n => ({ from: "0", to: n.id }));
    const at = [...readableLayout(nodes, edges, { width: 1400, height: 800 }).values()];
    const width = Math.max(...at.map(p => p.x)) - Math.min(...at.map(p => p.x));
    const height = Math.max(...at.map(p => p.y)) - Math.min(...at.map(p => p.y));
    expect(Math.max(width / height, height / width)).toBeLessThan(2.5);
  });
  it("reduces actual curved-edge crossings on a six-device communication cycle", () => {
    const nodes = ["a", "b", "c", "d", "e", "f"].map(id => ({ id, label: id }));
    const edges = [["a", "d"], ["b", "e"], ["c", "f"], ["a", "e"], ["b", "f"], ["c", "d"]].map(([from, to]) => ({ from, to }));
    const cross = (a: Point, b: Point, c: Point, d: Point) => {
      const side = (p: Point, q: Point, r: Point) => (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
      return side(a, b, c) * side(a, b, d) < 0 && side(c, d, a) * side(c, d, b) < 0;
    };
    const count = (positions: Map<string, Point>) => {
      let crossings = 0;
      const curves = edges.map(e => Array.from({ length: 25 }, (_, i) => pointOnEdge(edgeGeometry(positions.get(e.from)!, positions.get(e.to)!), i / 24)));
      edges.forEach((a, i) => edges.slice(i + 1).forEach((b, offset) => {
        if (a.from === b.from || a.from === b.to || a.to === b.from || a.to === b.to) return;
        const one = curves[i]; const two = curves[i + offset + 1];
        if (one.slice(1).some((p, k) => two.slice(1).some((q, l) => cross(one[k], p, two[l], q)))) crossings++;
      }));
      return crossings;
    };
    const before = count(placeNodes(nodes.map(n => n.id)));
    const after = count(readableLayout(nodes, edges, box));
    expect(before).toBeGreaterThan(0);
    expect(after).toBeLessThan(before);
  });
  it("places a bipartite crossing fixture in neighbour order", () => {
    const nodes = ["a", "b", "c", "d"].map(id => ({ id, label: id }));
    const edges = [{ from: "a", to: "d" }, { from: "b", to: "c" }, { from: "a", to: "c" }];
    const at = readableLayout(nodes, edges, box);
    // The tree-shaped graph can be drawn without crossings or shared node positions.
    expect(new Set([...at.values()].map(p => `${p.x}:${p.y}`)).size).toBe(4);
    expect(readableLayout(nodes, edges, box)).toEqual(at);
  });
  it("fits complete label footprints, including an off-centre graph", () => {
    const camera = fitFlowCamera([{ x: 1400, y: 800, halfWidth: 160, top: 48, bottom: 110 }], box);
    expect(camera.zoom).toBeGreaterThan(0);
    expect(Math.abs(1400 * camera.zoom + camera.x)).toBeLessThan(box.width / 2);
    expect(Math.abs(800 * camera.zoom + camera.y)).toBeLessThan(box.height / 2);
  });
  it("preserves expired values and evidence when a later window has a different monotonic origin", () => {
    const model = createFlowModel({ serverIncarnation: "fixture", sessionId: 1 });
    model.nodes.set("d:1", { id: "d:1", kind: "device", label: "Switch", deviceId: 1 });
    model.slots.set("d:1", new Map([[1, { gaRaw: 1, gaLabel: "0/0/1", value: "On", seq: 1,
      observedAtMs: 1000, deadlineMs: 8000, generation: "1", origin: "source", sourceNode: "d:1", sourceLabel: "Switch" }]]));
    model.slotCount = 1;
    const copy = cloneFlowAtOrigin(model, 100000, 109000);
    expect(copy.nodes).toEqual(model.nodes);
    expect(copy.slots.get("d:1")!.get(1)!.deadlineMs).toBe(-1000);
    expect(currentBadges(copy, "d:1", 0).current).toHaveLength(0);
    expect(model.slots.get("d:1")!.get(1)!.deadlineMs).toBe(8000);
  });
});
