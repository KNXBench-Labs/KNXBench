/** U21: bounded, damped activity layout for the flow view (promoted from the U19 study). */
// Per step the cost is O(nodes + edges): springs along edges, repulsion only
// between nodes in neighbouring grid cells, a weak pull to the centre and a
// stronger one for the activity leader. Coordinates are centred on 0/0 like
// the view. Nodes start on the stable hex slots (flowLayout.ts), so motion
// Off (no solver) and motion On begin from the same picture. Tuning values are
// the U19 measurements (TELEGRAM_FLOW_VISUALIZATION §9.4), not KNX facts.
//
// Heat is per node (§9.3 "reheat locally"): a new node or edge heats only
// itself and its direct neighbours, a step moves hot nodes only, and a cooled
// node keeps its exact position. Settled regions push hot nodes away but are
// not pushed themselves, so they are never rewritten.
import { hexSlots, nodeFootprint, type Footprint } from "./flowLayout";

export const MIN_DISTANCE = 40;
export const MAX_DISTANCE = 220;
/** Observations per window at which a pair reaches its shortest distance. */
export const RATE_SATURATION = 10;
const REPULSION_RADIUS = 60;
/** Pixels kept free between two footprints, so a settled pair does not touch. */
const FOOTPRINT_GAP = 2;
const DAMPING = 0.82;
const SPRING = 0.04;
const CENTRE_PULL = 0.002;
const LEADER_PULL = 0.05;
/** Furthest a node moves in one step, also when a crowd pushes it from all sides. */
export const MAX_STEP = 12;
/** Cooling: forces scale with alpha, which decays each step and stops at rest. */
export const ALPHA_DECAY = 0.985;
export const ALPHA_MIN = 0.005;

export interface DynamicNode {
  id: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  /** This node's own cooling factor; 0 = at rest, not moved by a step. */
  heat: number;
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
  /** The hottest node's heat: 0 once the whole map is at rest. */
  alpha: number;
}

function heatNode(layout: Dynamics, node: DynamicNode, alpha: number): void {
  node.heat = Math.max(node.heat, Math.min(1, alpha));
  layout.alpha = Math.max(layout.alpha, node.heat);
}

/** Heats the whole map, e.g. for a first layout. */
export function reheat(layout: Dynamics, alpha: number): void {
  for (const node of layout.nodes.values()) heatNode(layout, node, alpha);
}

/** Heats `ids` and their direct neighbours along `edges`, nothing else (§9.3). */
export function reheatAround(layout: Dynamics, ids: Iterable<string>, edges: readonly DynamicEdge[], alpha: number): void {
  const centre = new Set(ids);
  if (centre.size === 0) return;
  const heated = new Set(centre);
  for (const edge of edges) {
    if (centre.has(edge.from)) heated.add(edge.to);
    if (centre.has(edge.to)) heated.add(edge.from);
  }
  for (const id of heated) {
    const node = layout.nodes.get(id);
    if (node) heatNode(layout, node, alpha);
  }
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

/** Keeps a node's circle and name inside the drawing area; value lines may
 * still reach below the bottom edge. A box smaller than a footprint centres. */
function clamp(layout: Dynamics, node: DynamicNode): void {
  const f = nodeFootprint(0);
  const within = (value: number, low: number, high: number) => (low > high ? (low + high) / 2 : Math.max(low, Math.min(high, value)));
  node.x = within(node.x, -layout.width / 2 + f.halfWidth, layout.width / 2 - f.halfWidth);
  node.y = within(node.y, -layout.height / 2 + f.top, layout.height / 2 - f.bottom);
}

export function createDynamics(ids: readonly string[], box: { width: number; height: number }): Dynamics {
  const layout: Dynamics = { width: box.width, height: box.height, nodes: new Map(), alpha: 0 };
  ensureDynamicNodes(layout, ids);
  reheat(layout, 1);
  return layout;
}

/** Adds new ids on the next hex slots, at rest; existing nodes keep their positions. */
export function ensureDynamicNodes(layout: Dynamics, ids: Iterable<string>): void {
  const fresh = [...ids].filter((id) => !layout.nodes.has(id));
  if (fresh.length === 0) return;
  const slots = hexSlots(layout.nodes.size + fresh.length);
  for (const id of fresh) {
    const slot = slots[layout.nodes.size];
    const node = { id, x: slot.x, y: slot.y, vx: 0, vy: 0, heat: 0 };
    clamp(layout, node);
    layout.nodes.set(id, node);
  }
}

export interface StepOptions {
  leader: string | null;
  frozen?: boolean;
  /** Value lines shown below a node; neighbours keep clear of them (flowLayout.nodeFootprint). */
  badgeLines?: ReadonlyMap<string, number>;
  /** Heat kept per step; ALPHA_DECAY by default. The animator lowers it when
   * frames come slowly, so cooling follows wall time. */
  cooling?: number;
}

/** One damped step over the hot nodes. Returns the ids it moved; a frozen
 * or cooled layout moves nothing and keeps every position exactly. */
export function step(layout: Dynamics, edges: readonly DynamicEdge[], options: StepOptions): Set<string> {
  const moved = new Set<string>();
  if (options.frozen) return moved;
  const { nodes } = layout;
  const hot: DynamicNode[] = [];
  for (const node of nodes.values()) {
    if (node.heat >= ALPHA_MIN) hot.push(node);
    else if (node.heat !== 0) {
      node.heat = 0;
      node.vx = 0;
      node.vy = 0;
    }
  }
  if (hot.length === 0) {
    layout.alpha = 0;
    return moved;
  }
  const scale = areaScale(layout);
  const radius = REPULSION_RADIUS * scale;
  const footprint = (node: DynamicNode) => nodeFootprint(options.badgeLines?.get(node.id) ?? 0);
  // Cells must hold every pair whose footprints can touch.
  let cellSize = radius;
  for (const node of nodes.values()) {
    const f = footprint(node);
    cellSize = Math.max(cellSize, 2 * f.halfWidth, f.top + f.bottom);
  }
  for (const edge of edges) {
    const a = nodes.get(edge.from);
    const b = nodes.get(edge.to);
    if (!a || !b || a === b) continue;
    if (a.heat < ALPHA_MIN && b.heat < ALPHA_MIN) continue;
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const distance = Math.hypot(dx, dy) || 0.01;
    const force = (SPRING * (distance - preferredDistance(edge.rate) * scale)) / distance;
    // Scaled by each end's own heat: a settled end (heat 0) gains nothing.
    a.vx += dx * force * a.heat;
    a.vy += dy * force * a.heat;
    b.vx -= dx * force * b.heat;
    b.vy -= dy * force * b.heat;
  }
  const cells = new Map<string, DynamicNode[]>();
  for (const node of nodes.values()) {
    const key = `${Math.floor(node.x / cellSize)}:${Math.floor(node.y / cellSize)}`;
    const cell = cells.get(key);
    if (cell) cell.push(node);
    else cells.set(key, [node]);
  }
  for (const node of hot) {
    const gx = Math.floor(node.x / cellSize);
    const gy = Math.floor(node.y / cellSize);
    for (let ox = -1; ox <= 1; ox += 1) {
      for (let oy = -1; oy <= 1; oy += 1) {
        for (const other of cells.get(`${gx + ox}:${gy + oy}`) ?? []) {
          if (other === node) continue;
          const dx = node.x - other.x;
          const dy = node.y - other.y;
          const distance = Math.hypot(dx, dy) || 0.01;
          if (distance >= radius) continue;
          const push = node.heat * ((radius - distance) / radius) * 0.5 * scale;
          node.vx += (dx / distance) * push;
          node.vy += (dy / distance) * push;
        }
      }
    }
    const pull = node.heat * (node.id === options.leader ? LEADER_PULL : CENTRE_PULL);
    node.vx -= node.x * pull;
    node.vy -= node.y * pull;
  }
  let hottest = 0;
  for (const node of hot) {
    node.vx = Math.max(-MAX_STEP, Math.min(MAX_STEP, node.vx * DAMPING));
    node.vy = Math.max(-MAX_STEP, Math.min(MAX_STEP, node.vy * DAMPING));
    node.x += node.vx;
    node.y += node.vy;
    clamp(layout, node);
  }
  for (const node of hot) {
    separate(layout, node, footprint, cells, cellSize);
    moved.add(node.id);
    node.heat *= options.cooling ?? ALPHA_DECAY;
    hottest = Math.max(hottest, node.heat);
  }
  layout.alpha = hottest;
  return moved;
}

/** Moves a hot node out of every neighbour's footprint (§9.3 hub
 * readability): a hard constraint, not a force, so busy pairs that prefer a
 * short distance cannot pull circles, names or value lines over each other.
 * The push points away from the neighbour's centre, so a node pulled towards
 * a settled neighbour slides around it instead of sticking; it goes half the
 * way when the neighbour is hot too. A settled neighbour is never moved. */
function separate(
  layout: Dynamics,
  node: DynamicNode,
  footprint: (node: DynamicNode) => Footprint,
  cells: ReadonlyMap<string, DynamicNode[]>,
  cellSize: number,
): void {
  const own = footprint(node);
  let shiftX = 0;
  let shiftY = 0;
  const gx = Math.floor(node.x / cellSize);
  const gy = Math.floor(node.y / cellSize);
  for (let ox = -1; ox <= 1; ox += 1) {
    for (let oy = -1; oy <= 1; oy += 1) {
      for (const other of cells.get(`${gx + ox}:${gy + oy}`) ?? []) {
        if (other === node) continue;
        const theirs = footprint(other);
        const dx = node.x - other.x;
        const dy = node.y - other.y;
        const overlapX = own.halfWidth + theirs.halfWidth + FOOTPRINT_GAP - Math.abs(dx);
        // Below the neighbour, its value lines meet this node's name; above, the reverse.
        const overlapY = (dy >= 0 ? theirs.bottom + own.top : own.bottom + theirs.top) + FOOTPRINT_GAP - Math.abs(dy);
        if (overlapX <= 0 || overlapY <= 0) continue;
        const distance = Math.hypot(dx, dy);
        // Same centre: a fixed direction by id keeps the result deterministic.
        const ux = distance > 0 ? dx / distance : node.id < other.id ? -1 : 1;
        const uy = distance > 0 ? dy / distance : 0;
        const along = Math.min(
          Math.abs(ux) > 1e-9 ? overlapX / Math.abs(ux) : Infinity,
          Math.abs(uy) > 1e-9 ? overlapY / Math.abs(uy) : Infinity,
        );
        const push = along * (other.heat >= ALPHA_MIN ? 0.5 : 1);
        shiftX += ux * push;
        shiftY += uy * push;
        // Keep the sideways part of the velocity, drop the part into the neighbour.
        const inward = node.vx * ux + node.vy * uy;
        if (inward < 0) {
          node.vx -= inward * ux;
          node.vy -= inward * uy;
        }
      }
    }
  }
  // In a crowd the pushes add up; one step never moves a node further than
  // MAX_STEP, so an overfull map settles instead of jumping around.
  const shift = Math.hypot(shiftX, shiftY);
  const limit = shift > MAX_STEP ? MAX_STEP / shift : 1;
  node.x += shiftX * limit;
  node.y += shiftY * limit;
  clamp(layout, node);
}
