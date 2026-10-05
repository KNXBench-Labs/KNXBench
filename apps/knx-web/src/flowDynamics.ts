/** U21: bounded, damped activity layout for the flow view (promoted from the U19 study). */
// Per step the cost is O(nodes + edges): springs along edges, repulsion only
// between nodes in neighbouring grid cells, a weak pull to the centre and a
// stronger one for the activity leader. Coordinates are centred on 0/0 like
// the view. Nodes start on the stable hex slots (flowLayout.ts), so motion
// Off (no solver) and motion On begin from the same picture. Tuning values are
// the U19 measurements (TELEGRAM_FLOW_VISUALIZATION §9.4), not KNX facts.
import { hexSlots } from "./flowLayout";

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

export interface DynamicNode {
  id: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
}

export interface DynamicEdge {
  from: string;
  to: string;
  rate: number;
}

export interface Dynamics {
  width: number;
  height: number;
  nodes: Map<string, DynamicNode>;
  alpha: number;
}

/** Lets a cooled layout move again, e.g. when nodes, edges or the leader change. */
export function reheat(layout: Dynamics, alpha: number): void {
  layout.alpha = Math.max(layout.alpha, Math.min(1, alpha));
}

/** Distances follow the area each node has, within fixed bounds. */
export function areaScale(layout: Dynamics): number {
  const perNode = Math.sqrt((layout.width * layout.height) / Math.max(1, layout.nodes.size));
  return Math.min(2.5, Math.max(0.6, perNode / 110));
}

/** Busier pairs sit closer; quiet ones relax to the maximum, never beyond. */
export function preferredDistance(rate: number): number {
  const share = Math.min(1, Math.max(0, rate) / RATE_SATURATION);
  return MAX_DISTANCE - (MAX_DISTANCE - MIN_DISTANCE) * share;
}

function clamp(layout: Dynamics, node: DynamicNode): void {
  node.x = Math.max(-layout.width / 2, Math.min(layout.width / 2, node.x));
  node.y = Math.max(-layout.height / 2, Math.min(layout.height / 2, node.y));
}

export function createDynamics(ids: readonly string[], box: { width: number; height: number }): Dynamics {
  const layout: Dynamics = { width: box.width, height: box.height, nodes: new Map(), alpha: 1 };
  ensureDynamicNodes(layout, ids);
  return layout;
}

/** Adds new ids on the next hex slots; existing nodes keep their positions. */
export function ensureDynamicNodes(layout: Dynamics, ids: Iterable<string>): void {
  const fresh = [...ids].filter((id) => !layout.nodes.has(id));
  if (fresh.length === 0) return;
  const slots = hexSlots(layout.nodes.size + fresh.length);
  for (const id of fresh) {
    const slot = slots[layout.nodes.size];
    const node = { id, x: slot.x, y: slot.y, vx: 0, vy: 0 };
    clamp(layout, node);
    layout.nodes.set(id, node);
  }
}

/** One damped step. A frozen layout keeps every position exactly. */
export function step(layout: Dynamics, edges: readonly DynamicEdge[], options: { leader: string | null; frozen?: boolean }): void {
  if (options.frozen) return;
  if (layout.alpha < ALPHA_MIN) {
    layout.alpha = 0;
    return;
  }
  const { nodes } = layout;
  const alpha = layout.alpha;
  layout.alpha *= ALPHA_DECAY;
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
    a.vx += dx * force;
    a.vy += dy * force;
    b.vx -= dx * force;
    b.vy -= dy * force;
  }
  const cells = new Map<string, DynamicNode[]>();
  const cellOf = (node: DynamicNode) => `${Math.floor(node.x / radius)}:${Math.floor(node.y / radius)}`;
  for (const node of nodes.values()) {
    const key = cellOf(node);
    const cell = cells.get(key);
    if (cell) cell.push(node);
    else cells.set(key, [node]);
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
    node.vx -= node.x * pull;
    node.vy -= node.y * pull;
  }
  for (const node of nodes.values()) {
    node.vx = Math.max(-MAX_STEP, Math.min(MAX_STEP, node.vx * DAMPING));
    node.vy = Math.max(-MAX_STEP, Math.min(MAX_STEP, node.vy * DAMPING));
    node.x += node.vx;
    node.y += node.vy;
    clamp(layout, node);
  }
}
