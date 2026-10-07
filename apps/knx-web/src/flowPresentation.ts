/** Builds readable graph lanes, fits their full footprints and translates cross-window clocks. */
import { edgeGeometry, pointOnEdge, hexSlots, nodeFootprint, type EdgeGeometry, type Point, type Footprint } from "./flowLayout";
import type { FlowModel } from "./flowModel";

export interface FlowCamera { zoom: number; x: number; y: number }
export interface FlowBox { width: number; height: number }
export const DISPLAY_LABEL_LIMIT = 36;
export const DETAILED_ROUTING_NODES = 80;
export const DETAILED_ROUTING_EDGES = 250;
export function displayFlowLabel(label: string): string {
  return label.length > DISPLAY_LABEL_LIMIT ? `${label.slice(0, DISPLAY_LABEL_LIMIT - 1)}…` : label;
}
export function labelHalfWidth(label: string): number {
  return Math.max(50, displayFlowLabel(label).length * 3.5);
}

/** Connected components, BFS ranks and alternating barycentre sweeps. A graph
 * grows its world rather than squeezing more nodes into fixed drawing bounds.
 * This is bounded layout work on graph growth, not a per-telegram simulation.
 * Arbitrary/non-planar graphs still have crossings; no edges are removed. */
export function readableLayout(
  nodes: readonly { id: string; label: string }[],
  edges: readonly { from: string; to: string }[],
  box: FlowBox,
): Map<string, Point> {
  const neighbours = new Map(nodes.map(n => [n.id, new Set<string>()]));
  for (const e of edges) {
    if (e.from === e.to || !neighbours.has(e.from) || !neighbours.has(e.to)) continue;
    neighbours.get(e.from)!.add(e.to); neighbours.get(e.to)!.add(e.from);
  }
  const rankOrder = [...neighbours.keys()].sort((a, b) => neighbours.get(b)!.size - neighbours.get(a)!.size || a.localeCompare(b));
  const remaining = new Set(rankOrder);
  const components: { points: Map<string, Point>; width: number; height: number }[] = [];
  const labels = new Map(nodes.map(n => [n.id, n.label]));
  for (const root of rankOrder) {
    if (!remaining.delete(root)) continue;
    const ranks: string[][] = [[root]];
    const rankById = new Map([[root, 0]]);
    for (let rank = 0; rank < ranks.length; rank++) {
      for (const id of ranks[rank]) for (const neighbour of [...neighbours.get(id)!].sort()) {
        if (!remaining.delete(neighbour)) continue;
        const next = rank + 1;
        (ranks[next] ??= []).push(neighbour); rankById.set(neighbour, next);
      }
    }
    for (let sweep = 0; sweep < 6; sweep++) {
      const sequence = ranks.map((_, i) => i);
      if (sweep % 2) sequence.reverse();
      const order = new Map<string, number>();
      ranks.forEach(ids => ids.forEach((id, index) => order.set(id, index)));
      for (const r of sequence) {
        const wanted = r + (sweep % 2 ? 1 : -1);
        const barycentre = (id: string) => {
          const adjacent = [...neighbours.get(id)!].filter(n => rankById.get(n) === wanted);
          return adjacent.length ? adjacent.reduce((sum, n) => sum + order.get(n)!, 0) / adjacent.length : order.get(id)!;
        };
        ranks[r].sort((a, b) => barycentre(a) - barycentre(b) || a.localeCompare(b));
        ranks[r].forEach((id, index) => order.set(id, index));
      }
    }
    const half = Math.max(50, ...[...rankById.keys()].map(id => labelHalfWidth(labels.get(id)!)));
    const column = half * 2 + 120;
    const row = 190;
    const points = new Map<string, Point>();
    const ids = [...rankById.keys()];
    if (ids.length > 8 && neighbours.get(root)!.size === ids.length - 1 && ids.every(id => id === root || neighbours.get(id)!.size === 1)) {
      // A wide star is not a 24-storey column: compact hex rings leave room
      // for values and direct fan-out, with obstacle routing on outer rays.
      hexSlots(ids.length).forEach((p, i) => points.set(ids[i], { x: p.x * Math.max(2, half * 2 / 100), y: p.y * 1.45 }));
      const left = Math.min(...[...points.values()].map(p => p.x));
      const top = Math.min(...[...points.values()].map(p => p.y));
      const right = Math.max(...[...points.values()].map(p => p.x));
      const bottom = Math.max(...[...points.values()].map(p => p.y));
      for (const p of points.values()) { p.x -= left - half; p.y -= top - row / 2; }
      components.push({ points, width: right - left + half * 2 + 100, height: bottom - top + row });
    } else {
      const rows = Math.max(4, Math.ceil(Math.sqrt(ids.length * box.height / Math.max(1, box.width))));
      const height = Math.min(rows, Math.max(...ranks.map(ids => ids.length))) * row;
      let x = half;
      for (const rank of ranks) {
        const columns = Math.ceil(rank.length / rows);
        rank.forEach((id, i) => points.set(id, { x: x + Math.floor(i / rows) * column,
          y: (height - Math.min(rows, rank.length - Math.floor(i / rows) * rows) * row) / 2 + i % rows * row + row / 2 }));
        x += columns * column;
      }
      components.push({ points, width: x - column + half + 100, height });
    }
  }
  // Shelf-pack disconnected components; sensible aspect ratio, unrestricted
  // world height. A late isolated sender cannot land on a clamped boundary.
  const shelfWidth = Math.max(box.width, Math.sqrt(Math.max(1, nodes.length) * 220 * 190 * box.width / Math.max(1, box.height)));
  const result = new Map<string, Point>();
  let x = 0; let y = 0; let shelfHeight = 0; let width = 0;
  for (const c of components) {
    if (x > 0 && x + c.width > shelfWidth) { y += shelfHeight + 80; x = 0; shelfHeight = 0; }
    for (const [id, p] of c.points) result.set(id, { x: p.x + x, y: p.y + y });
    x += c.width + 80; width = Math.max(width, x - 80); shelfHeight = Math.max(shelfHeight, c.height);
  }
  const height = y + shelfHeight;
  for (const p of result.values()) { p.x -= width / 2; p.y -= height / 2; }
  return result;
}

export function fitFlowCamera(
  nodes: readonly (Point & { halfWidth: number; top: number; bottom: number })[],
  box: FlowBox,
): FlowCamera {
  if (!nodes.length) return { zoom: 1, x: 0, y: 0 };
  const left = Math.min(...nodes.map(p => p.x - p.halfWidth));
  const right = Math.max(...nodes.map(p => p.x + p.halfWidth));
  const top = Math.min(...nodes.map(p => p.y - p.top));
  const bottom = Math.max(...nodes.map(p => p.y + p.bottom));
  const zoom = Math.min(1.5, Math.max(0.01, Math.min(Math.max(1, box.width - 80) / Math.max(1, right - left), Math.max(1, box.height - 80) / Math.max(1, bottom - top))));
  return { zoom, x: -(left + right) / 2 * zoom, y: -(top + bottom) / 2 * zoom };
}

/** Tries bounded alternate curves around foreign node/label footprints.
 * Directed reverse routes use a distinct lane multiplier in the renderer.
 * Sampling is best-effort obstacle avoidance, not a planar-graph guarantee. */
export function routeFlowEdge(from: Point, to: Point, obstacles: readonly (Point & Footprint)[], lane = 1): EdgeGeometry {
  let best = edgeGeometry(from, to);
  const score = (curve: EdgeGeometry) => {
    let hits = 0;
    for (let i = 1; i < 24; i++) {
      const p = pointOnEdge(curve, i / 24);
      for (const n of obstacles) if (Math.abs(p.x - n.x) < n.halfWidth + 12 && p.y > n.y - n.top - 12 && p.y < n.y + n.bottom + 12) hits++;
    }
    return hits;
  };
  let bestScore = score(best);
  if (!bestScore) return best;
  for (const bend of [110, -110, 190, -190, 280, -280]) {
    const curve = edgeGeometry(from, to, bend * lane); const hits = score(curve);
    if (hits < bestScore) { best = curve; bestScore = hits; }
    if (!bestScore) break;
  }
  return best;
}

export function flowFootprint(label: string, badgeLines: number) {
  return { ...nodeFootprint(badgeLines), halfWidth: Math.max(labelHalfWidth(label), badgeLines ? 130 : 0) };
}

/** Maps use structured-clone transport, never localStorage/persisted traffic.
 * performance.timeOrigin + performance.now is the common monotonic epoch;
 * document-local deadlines must be shifted, not reset on window adoption. */
export function cloneFlowAtOrigin(model: FlowModel, fromOrigin: number, toOrigin: number): FlowModel {
  const copy = structuredClone(model);
  const shift = fromOrigin - toOrigin;
  for (const entry of copy.pending) if (entry.observedAtMs !== null) entry.observedAtMs += shift;
  for (const edge of copy.edges.values()) {
    edge.lastObservedAtMs += shift;
    edge.recentTimes = edge.recentTimes.map(t => t + shift);
  }
  for (const slots of copy.slots.values()) for (const slot of slots.values()) { slot.observedAtMs += shift; slot.deadlineMs += shift; }
  for (const [id, times] of copy.sendTimes) copy.sendTimes.set(id, times.map(t => t + shift));
  for (const event of copy.events) event.observedAtMs += shift;
  return copy;
}
