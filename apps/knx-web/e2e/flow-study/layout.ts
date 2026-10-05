/** U19 study layout: a bounded, damped activity layout without a graph dependency. */
// Evaluated in U19 before any force-layout dependency is chosen. Per tick the
// cost is O(nodes + edges): springs along edges, repulsion only between nodes
// in neighbouring grid cells, a weak pull to the centre and a stronger one for
// the activity leader. Positions live in a disposable copy keyed by node id;
// no domain or wire object is ever handed to the solver.

export const MIN_DISTANCE = 40;
export const MAX_DISTANCE = 220;
/** Observations per window at which a pair reaches its shortest distance. */
export const RATE_SATURATION = 10;
const REPULSION_RADIUS = 60;
const DAMPING = 0.82;
const SPRING = 0.04;
const CENTRE_PULL = 0.002;
const LEADER_PULL = 0.05;
const MAX_STEP = 12;
/** Cooling: forces scale with alpha, which decays each step and stops at rest. */
export const ALPHA_DECAY = 0.985;
export const ALPHA_MIN = 0.005;

export interface LayoutNode { id: string; x: number; y: number; vx: number; vy: number }
export interface LayoutEdge { from: string; to: string; rate: number }
export interface Layout { width: number; height: number; nodes: Map<string, LayoutNode>; alpha: number }

/** Lets a cooled layout move again, e.g. when nodes, edges or the leader change. */
export function reheat(layout: Layout, alpha: number): void {
  layout.alpha = Math.max(layout.alpha, Math.min(1, alpha));
}

/** Distances follow the area each node has: a sparse map spreads out, a
 * dense one tightens, both within fixed bounds. */
export function areaScale(layout: Layout): number {
  const perNode = Math.sqrt((layout.width * layout.height) / Math.max(1, layout.nodes.size));
  return Math.min(2.5, Math.max(0.6, perNode / 110));
}

/** Busier pairs sit closer; quiet ones relax to the maximum, never beyond. */
export function preferredDistance(rate: number): number {
  const share = Math.min(1, Math.max(0, rate) / RATE_SATURATION);
  return MAX_DISTANCE - (MAX_DISTANCE - MIN_DISTANCE) * share;
}

function random(seed: number): () => number {
  let state = seed >>> 0 || 1;
  return () => {
    state ^= state << 13; state >>>= 0;
    state ^= state >>> 17;
    state ^= state << 5; state >>>= 0;
    return state / 0x1_0000_0000;
  };
}

export function createLayout(ids: readonly string[], box: { width: number; height: number }, seed: number): Layout {
  const next = random(seed);
  const nodes = new Map<string, LayoutNode>();
  for (const id of ids) nodes.set(id, { id, x: box.width * (0.2 + 0.6 * next()), y: box.height * (0.2 + 0.6 * next()), vx: 0, vy: 0 });
  return { width: box.width, height: box.height, nodes, alpha: 1 };
}

/** Adds nodes for new ids at a seeded position near the centre; existing
 * nodes keep their positions, so a new event never reshuffles the map. */
export function ensureNodes(layout: Layout, ids: Iterable<string>, seed: number): void {
  const next = random(seed);
  for (const id of ids) {
    if (layout.nodes.has(id)) continue;
    layout.nodes.set(id, { id, x: layout.width * (0.35 + 0.3 * next()), y: layout.height * (0.35 + 0.3 * next()), vx: 0, vy: 0 });
  }
}

/** One damped tick. A frozen layout keeps every position exactly. */
export function step(layout: Layout, edges: readonly LayoutEdge[], options: { leader: string | null; frozen?: boolean }): void {
  if (options.frozen) return;
  if (layout.alpha < ALPHA_MIN) { layout.alpha = 0; return; }
  const { nodes } = layout;
  const alpha = layout.alpha;
  layout.alpha *= ALPHA_DECAY;
  const cx = layout.width / 2;
  const cy = layout.height / 2;
  const scale = areaScale(layout);
  const radius = REPULSION_RADIUS * scale;
  for (const edge of edges) {
    const a = nodes.get(edge.from);
    const b = nodes.get(edge.to);
    if (!a || !b || a === b) continue;
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const distance = Math.hypot(dx, dy) || 0.01;
    const force = (alpha * SPRING * (distance - preferredDistance(edge.rate) * scale)) / distance;
    a.vx += dx * force; a.vy += dy * force;
    b.vx -= dx * force; b.vy -= dy * force;
  }
  const cells = new Map<string, LayoutNode[]>();
  const cellOf = (node: LayoutNode) => `${Math.floor(node.x / radius)}:${Math.floor(node.y / radius)}`;
  for (const node of nodes.values()) {
    const key = cellOf(node);
    const cell = cells.get(key);
    if (cell) cell.push(node); else cells.set(key, [node]);
  }
  for (const node of nodes.values()) {
    const gx = Math.floor(node.x / radius);
    const gy = Math.floor(node.y / radius);
    for (let ox = -1; ox <= 1; ox += 1) {
      for (let oy = -1; oy <= 1; oy += 1) {
        for (const other of cells.get(`${gx + ox}:${gy + oy}`) ?? []) {
          if (other === node) continue;
          const dx = node.x - other.x;
          const dy = node.y - other.y;
          const distance = Math.hypot(dx, dy) || 0.01;
          if (distance >= radius) continue;
          const push = alpha * ((radius - distance) / radius) * 0.5 * scale;
          node.vx += (dx / distance) * push;
          node.vy += (dy / distance) * push;
        }
      }
    }
    const pull = alpha * (node.id === options.leader ? LEADER_PULL : CENTRE_PULL);
    node.vx += (cx - node.x) * pull;
    node.vy += (cy - node.y) * pull;
  }
  for (const node of nodes.values()) {
    node.vx = Math.max(-MAX_STEP, Math.min(MAX_STEP, node.vx * DAMPING));
    node.vy = Math.max(-MAX_STEP, Math.min(MAX_STEP, node.vy * DAMPING));
    node.x = Math.max(0, Math.min(layout.width, node.x + node.vx));
    node.y = Math.max(0, Math.min(layout.height, node.y + node.vy));
  }
}
