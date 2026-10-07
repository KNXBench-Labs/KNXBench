/** U20: stable placement on hex rings and curved, direction-distinct edges for the flow view. */
// U20 places nodes by order of first appearance only: a node never moves
// once placed, so new traffic cannot shuffle the map. Activity-dependent
// distances, damping and the leader belong to U21 (docs/
// TELEGRAM_FLOW_VISUALIZATION.md §5, §9.4).

export const NODE_SPACING = 150;
/** U21 fade: full emphasis while active, then down to a resting line. */
export const QUIET_AFTER_MS = 10_000;
export const FADE_MS = 60_000;
export const RESTING_OPACITY = 0.35;
export const NODE_RADIUS = 22;
/** Gap between a node and its name above or its value lines below. */
export const TEXT_CLEARANCE = 16;
/** Height of one value line below a node. */
export const BADGE_LINE = 14;
/** Half the width a name or value line usually takes (11 px monospace, about
 * twelve characters). Longer names may still touch a neighbour's text. */
const TEXT_HALF_WIDTH = 40;
/** Cap height of the 11 px name above a node. */
const LABEL_ASCENT = 10;
/** How far below a value line's baseline its letters reach. */
const BADGE_DESCENT = 4;
const MAX_BEND = 60;

/** The area a node's circle, name and value lines take, relative to its
 * centre: what neighbours have to keep clear of (§9.3 hub readability). */
export interface Footprint {
  halfWidth: number;
  top: number;
  bottom: number;
}

export function nodeFootprint(badgeLines: number): Footprint {
  const top = NODE_RADIUS + TEXT_CLEARANCE + LABEL_ASCENT;
  const bottom = badgeLines > 0 ? NODE_RADIUS + TEXT_CLEARANCE + 10 + (badgeLines - 1) * BADGE_LINE + BADGE_DESCENT : NODE_RADIUS;
  return { halfWidth: Math.max(NODE_RADIUS, TEXT_HALF_WIDTH), top, bottom };
}

export interface Point {
  x: number;
  y: number;
}

const AXIAL_DIRECTIONS: readonly [number, number][] = [[1, 0], [1, -1], [0, -1], [-1, 0], [-1, 1], [0, 1]];

function axialToPoint(q: number, r: number): Point {
  return { x: NODE_SPACING * (q + r / 2), y: NODE_SPACING * (Math.sqrt(3) / 2) * r };
}

/** Positions by order: the centre, then ring after ring of the hex lattice. */
export function placeNodes(ids: readonly string[]): Map<string, Point> {
  const placed = new Map<string, Point>();
  const slots = hexSlots(ids.length);
  ids.forEach((id, index) => placed.set(id, slots[index]));
  return placed;
}

/** The first `count` lattice positions in placement order. */
export function hexSlots(count: number): Point[] {
  const slots: Point[] = [];
  if (count > 0) slots.push(axialToPoint(0, 0));
  for (let ring = 1; slots.length < count; ring++) {
    let q = -ring;
    let r = ring;
    for (const [dq, dr] of AXIAL_DIRECTIONS) {
      for (let step = 0; step < ring; step++) {
        if (slots.length < count) slots.push(axialToPoint(q, r));
        q += dq;
        r += dr;
      }
    }
  }
  return slots;
}

export interface EdgeGeometry {
  path: string;
  start: Point;
  end: Point;
  control: Point;
  /** Midpoint of the curve, where the group-address label sits. */
  label: Point;
}

/** A quadratic curve bending to the left of the direction of travel, so the
 * two directions of one pair never share a line. Trimmed at the circles. */
export function edgeGeometry(from: Point, to: Point, bendOverride?: number): EdgeGeometry {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy) || 1;
  const normal = { x: -dy / length, y: dx / length };
  const bend = bendOverride ?? Math.min(MAX_BEND, 0.2 * length);
  const middle = { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 };
  const control = { x: middle.x + normal.x * bend * 2, y: middle.y + normal.y * bend * 2 };
  const label = { x: middle.x + normal.x * bend, y: middle.y + normal.y * bend };
  const trim = (anchor: Point, toward: Point): Point => {
    const tx = toward.x - anchor.x;
    const ty = toward.y - anchor.y;
    const distance = Math.hypot(tx, ty) || 1;
    // Ends just outside the circle; node text keeps clear of the arrowhead
    // (see `TEXT_CLEARANCE` in TelegramFlowView.tsx).
    const offset = NODE_RADIUS + 3;
    return { x: anchor.x + (tx / distance) * offset, y: anchor.y + (ty / distance) * offset };
  };
  const start = trim(from, control);
  const end = trim(to, control);
  const f = (n: number) => n.toFixed(1);
  return { path: `M${f(start.x)},${f(start.y)} Q${f(control.x)},${f(control.y)} ${f(end.x)},${f(end.y)}`, start, end, control, label };
}

/** A point on the drawn curve: 0 at the start, 1 at the end. */
export function pointOnEdge(geometry: EdgeGeometry, t: number): Point {
  const u = 1 - t;
  const { start, control, end } = geometry;
  return {
    x: u * u * start.x + 2 * u * t * control.x + t * t * end.x,
    y: u * u * start.y + 2 * u * t * control.y + t * t * end.y,
  };
}

/** U21: an edge's emphasis from its last observation; never invisible.
 * Unknown observation time (`-Infinity`) is a resting line, not fresh. */
export function edgeOpacity(lastObservedAtMs: number, nowMs: number): number {
  if (!Number.isFinite(lastObservedAtMs)) return RESTING_OPACITY;
  const quiet = nowMs - lastObservedAtMs;
  if (quiet <= QUIET_AFTER_MS) return 1;
  const share = Math.min(1, (quiet - QUIET_AFTER_MS) / FADE_MS);
  return share >= 1 ? RESTING_OPACITY : 1 - (1 - RESTING_OPACITY) * share;
}
