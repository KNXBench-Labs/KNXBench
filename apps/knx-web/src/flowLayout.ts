/** U20: stable placement on hex rings and curved, direction-distinct edges for the flow view. */
// U20 places nodes by order of first appearance only: a node never moves
// once placed, so new traffic cannot shuffle the map. Activity-dependent
// distances, damping and the leader belong to U21 (docs/
// TELEGRAM_FLOW_VISUALIZATION.md §5, §9.4).

export const NODE_SPACING = 150;
export const NODE_RADIUS = 22;
const MAX_BEND = 60;

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
  let index = 0;
  const take = (q: number, r: number) => {
    if (index < ids.length) placed.set(ids[index++], axialToPoint(q, r));
  };
  take(0, 0);
  for (let ring = 1; index < ids.length; ring++) {
    let q = -ring;
    let r = ring;
    for (const [dq, dr] of AXIAL_DIRECTIONS) {
      for (let step = 0; step < ring; step++) {
        take(q, r);
        q += dq;
        r += dr;
      }
    }
  }
  return placed;
}

export interface EdgeGeometry {
  path: string;
  start: Point;
  end: Point;
  /** Midpoint of the curve, where the group-address label sits. */
  label: Point;
}

/** A quadratic curve bending to the left of the direction of travel, so the
 * two directions of one pair never share a line. Trimmed at the circles. */
export function edgeGeometry(from: Point, to: Point): EdgeGeometry {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy) || 1;
  const normal = { x: -dy / length, y: dx / length };
  const bend = Math.min(MAX_BEND, 0.2 * length);
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
  return { path: `M${f(start.x)},${f(start.y)} Q${f(control.x)},${f(control.y)} ${f(end.x)},${f(end.y)}`, start, end, label };
}
