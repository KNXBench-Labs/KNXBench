/** U21: drives the flow view's motion: solver steps, pulses, nudges; stops when idle or off. */
// No React and no DOM: a scheduler (frames, timers, clock, visibility) and a
// sink (where positions and pulses are drawn) are injected, so tests can
// prove what really stops. Values are never touched here: the reducer owns
// them and admits data immediately; a pulse is illustration only and has no
// write authority (docs/TELEGRAM_FLOW_VISUALIZATION.md §1, §4).
import {
  ALPHA_DECAY,
  RATE_SATURATION,
  createDynamics,
  ensureDynamicNodes,
  reheatAround,
  step,
  type DynamicEdge,
  type DynamicNode,
  type Dynamics,
} from "./flowDynamics";
import { FRESH_EVENT_MS, currentBadges, currentLeader, edgeActivity, type FlowEvent, type FlowModel } from "./flowModel";

export const PULSE_MS = 700;
/** At most one drawn frame per interval (~30 fps). The U21 load study found
 * motion's cost to be painting, which grows with every drawn frame. */
export const FRAME_INTERVAL_MS = 32;
/** Simultaneous pulse elements; beyond this telegrams are counted, not drawn. */
export const MAX_PULSES = 160;
/** More events than this in one batch are bundled per pair and group address. */
export const COALESCE_ABOVE = 24;
const NUDGE_EVERY_MS = 5_000;
const NUDGE_ALPHA = 0.08;
const GROWTH_ALPHA = 0.3;
const REFRESH_MS = 1_000;
/** A node is redrawn only once it moved at least this far from where it was
 * last drawn: the §7 load profile showed attribute writes and the native
 * style/paint work they trigger as the main cost, and sub-pixel moves change
 * nothing visible. */
export const MIN_DRAWN_MOVE = 0.5;
/** Longest gap between two solver steps that still counts as cooling time. */
const MAX_COOLING_SPAN_MS = 1_000;

/** Coarse activity classes: a pair's distance adapts when its class
 * changes, not on every observation inside the same class. */
export function activityClass(rate: number): number {
  if (rate <= 0) return 0;
  if (rate <= 2) return 1;
  if (rate <= 5) return 2;
  if (rate < RATE_SATURATION) return 3;
  return 4;
}

export interface AnimatorScheduler {
  frame(callback: (now: number) => void): number;
  cancelFrame(id: number): void;
  every(ms: number, callback: () => void): number;
  cancelEvery(id: number): void;
  now(): number;
  hidden(): boolean;
}

export interface DrawnPulse {
  from: string;
  to: string;
  gaLabel: string;
  /** Telegrams this pulse stands for (1 unless bundled). */
  count: number;
  /** 0 at the sender, 1 at the target. */
  progress: number;
}

export interface AnimatorSink {
  /** `moved` names the nodes this frame moved; everything else is unchanged
   * and need not be rewritten. */
  positions(nodes: ReadonlyMap<string, DynamicNode>, moved: ReadonlySet<string>): void;
  pulses(pulses: readonly DrawnPulse[]): void;
  /** Once a second while visible: fades, leader label and value expiry. */
  refresh(): void;
}

interface Pulse {
  from: string;
  to: string;
  gaLabel: string;
  count: number;
  start: number;
}

export interface AnimatorMetrics {
  frames: number;
  steps: number;
  /** Telegrams drawn completely, as part of a bundle. Each telegram counts once. */
  coalescedEvents: number;
  /** Telegrams of which at least one line found no free pulse element; counted
   * once per telegram, not per line, and never also as bundled. */
  overCapacityEvents: number;
  /** True once bundling or the capacity limit was needed. */
  reduced: boolean;
}

export class FlowAnimator {
  readonly layout: Dynamics;
  readonly metrics: AnimatorMetrics = { frames: 0, steps: 0, coalescedEvents: 0, overCapacityEvents: 0, reduced: false };
  private motion = false;
  private frozen = false;
  private frameId: number | null = null;
  private nudgeId: number | null = null;
  private readonly refreshId: number;
  private pulses: Pulse[] = [];
  private lastEventSeq = -1;
  /** The model the event baseline belongs to, and how many of its events
   * had been recorded at the last sync (AR21 finding 7). */
  private eventSource: FlowModel | null = null;
  private eventsRecordedSeen = 0;
  private edges: DynamicEdge[] = [];
  /** Activity class per directed pair, to see which pairs changed class. */
  private edgeClasses = new Map<string, number>();
  /** Value-badge lines per node (badges plus an overflow line). */
  private badgeLines = new Map<string, number>();
  /** Most badge lines a node has shown; only a new maximum reheats it, so
   * values coming and going do not keep a settled map moving. */
  private badgeHighWater = new Map<string, number>();
  private leader: string | null = null;
  private model: FlowModel | null = null;
  private lastDrawnAt = Number.NEGATIVE_INFINITY;
  /** When the solver last stepped; a longer gap (rest, Freeze) counts as one frame. */
  private lastStepAt = Number.NEGATIVE_INFINITY;
  /** Where each node was last handed to the sink. */
  private drawnAt = new Map<string, { x: number; y: number }>();

  constructor(
    box: { width: number; height: number },
    private readonly scheduler: AnimatorScheduler,
    private readonly sink: AnimatorSink,
  ) {
    this.layout = createDynamics([], box);
    this.refreshId = scheduler.every(REFRESH_MS, () => {
      if (!scheduler.hidden()) sink.refresh();
    });
  }

  get currentLeader(): string | null {
    return this.leader;
  }

  setMotion(allowed: boolean): void {
    if (allowed === this.motion) return;
    this.motion = allowed;
    if (!allowed) {
      this.cancelFrame();
      this.stopNudges();
      this.pulses = [];
      this.sink.pulses([]);
      return;
    }
    this.startNudges();
    this.wake();
  }

  setFrozen(frozen: boolean): void {
    if (frozen === this.frozen) return;
    this.frozen = frozen;
    if (frozen) this.stopNudges();
    else if (this.motion) {
      this.startNudges();
      this.wake();
    }
  }

  /** Takes in the model after a change: new nodes, rates, leader and events. */
  sync(model: FlowModel): void {
    this.model = model;
    const now = this.scheduler.now();
    // Local reheat (§9.3): only what changed, plus its direct neighbours.
    const added = [...model.nodes.keys()].filter((id) => !this.layout.nodes.has(id));
    ensureDynamicNodes(this.layout, added);
    const { grown, reclassed } = this.refreshEdges(now);
    const grownBadges = this.refreshBadges(now);
    reheatAround(this.layout, [...added, ...grown, ...this.leaderMoves(now)], this.edges, GROWTH_ALPHA);
    reheatAround(this.layout, [...reclassed, ...grownBadges], this.edges, NUDGE_ALPHA);

    // A session change brings a new model whose sequence numbers start
    // again; the old session's high mark must not hide its telegrams.
    if (model !== this.eventSource) {
      this.eventSource = model;
      this.lastEventSeq = -1;
      this.eventsRecordedSeen = 0;
    }
    const fresh = model.events.filter((event) => event.seq > this.lastEventSeq);
    if (fresh.length > 0) this.lastEventSeq = fresh[fresh.length - 1].seq;
    // Events recorded since the last sync that are no longer in the ring
    // were pushed out before they could be drawn (AR21 finding 7).
    const unseenDropped = model.counters.eventsRecorded - this.eventsRecordedSeen - fresh.length;
    this.eventsRecordedSeen = model.counters.eventsRecorded;
    // A hidden page draws nothing, and what happened meanwhile is not
    // replayed on return: only events observed just now become pulses.
    if (this.motion && !this.scheduler.hidden()) {
      this.queuePulses(fresh.filter((event) => event.observedAtMs >= now - FRESH_EVENT_MS), now);
      if (unseenDropped > 0) {
        this.metrics.overCapacityEvents += unseenDropped;
        this.metrics.reduced = true;
      }
    }
    this.wake();
  }

  activePulses(): DrawnPulse[] {
    const now = this.scheduler.now();
    return this.pulses.map((pulse) => ({
      from: pulse.from, to: pulse.to, gaLabel: pulse.gaLabel, count: pulse.count,
      progress: Math.min(1, Math.max(0, (now - pulse.start) / PULSE_MS)),
    }));
  }

  dispose(): void {
    this.cancelFrame();
    this.stopNudges();
    this.scheduler.cancelEvery(this.refreshId);
    this.pulses = [];
  }

  /** Rebuilds the solver edges; names the endpoints of new pairs (`grown`)
   * and of pairs whose activity class changed (`reclassed`). */
  private refreshEdges(now: number): { grown: string[]; reclassed: string[] } {
    const grown: string[] = [];
    const reclassed: string[] = [];
    if (!this.model) return { grown, reclassed };
    this.edges = [...this.model.edges.values()].map((edge) => ({ from: edge.from, to: edge.to, rate: edgeActivity(edge, now) }));
    const classes = new Map<string, number>();
    for (const edge of this.edges) {
      const key = `${edge.from}\u0000${edge.to}`;
      const cls = activityClass(edge.rate);
      classes.set(key, cls);
      const previous = this.edgeClasses.get(key);
      if (previous === undefined) grown.push(edge.from, edge.to);
      else if (previous !== cls) reclassed.push(edge.from, edge.to);
    }
    this.edgeClasses = classes;
    return { grown, reclassed };
  }

  /** The old and the new leader when the leader changed, else nothing. */
  private leaderMoves(now: number): string[] {
    if (!this.model) return [];
    const leader = currentLeader(this.model, now);
    if (leader === this.leader) return [];
    const moves = [this.leader, leader].filter((id): id is string => id !== null);
    this.leader = leader;
    return moves;
  }

  /** Updates badge lines per node; names nodes whose badge block reached a new size. */
  private refreshBadges(now: number): string[] {
    const grown: string[] = [];
    if (!this.model) return grown;
    const lines = new Map<string, number>();
    for (const id of this.model.nodes.keys()) {
      const badges = currentBadges(this.model, id, now);
      const count = badges.current.length + (badges.overflow > 0 ? 1 : 0);
      if (count === 0) continue;
      lines.set(id, count);
      if (count > (this.badgeHighWater.get(id) ?? 0)) {
        this.badgeHighWater.set(id, count);
        grown.push(id);
      }
    }
    this.badgeLines = lines;
    return grown;
  }

  private queuePulses(events: readonly FlowEvent[], now: number): void {
    const bundles = new Map<string, Pulse>();
    const bundle = events.length > COALESCE_ABOVE;
    // Each telegram is counted once (AR21 finding 4): as not (completely)
    // drawn when any of its lines found no free pulse, when it has no line
    // at all (AR21 finding 5), or when the model refused its sender or any
    // recipient (finding 6) — else, in a bundled batch, as drawn bundled.
    // Lines per telegram are not telegrams.
    const incomplete = new Set<number>();
    const members = new Map<Pulse, number[]>();
    const add = (pulse: Pulse, telegrams: readonly number[]) => {
      if (this.pulses.length >= MAX_PULSES) {
        for (const index of telegrams) incomplete.add(index);
        this.metrics.reduced = true;
        return;
      }
      this.pulses.push(pulse);
    };
    events.forEach((event, index) => {
      // A refused sender or recipient (AR21 finding 6): what lines remain are
      // drawn, but the telegram is not drawn completely.
      if (!event.complete) {
        incomplete.add(index);
        this.metrics.reduced = true;
      }
      if (event.to.length === 0) {
        incomplete.add(index);
        this.metrics.reduced = true;
        return;
      }
      for (const to of event.to) {
        const pulse = { from: event.from, to, gaLabel: event.gaLabel, count: 1, start: now };
        if (!bundle) {
          add(pulse, [index]);
          continue;
        }
        const key = `${event.from}\u0000${to}\u0000${event.gaRaw}`;
        const existing = bundles.get(key);
        if (existing) {
          existing.count += 1;
          members.get(existing)!.push(index);
        } else {
          bundles.set(key, pulse);
          members.set(pulse, [index]);
        }
      }
    });
    if (bundle) {
      this.metrics.reduced = true;
      for (const pulse of bundles.values()) add(pulse, members.get(pulse)!);
      this.metrics.coalescedEvents += events.length - incomplete.size;
    }
    this.metrics.overCapacityEvents += incomplete.size;
  }

  private startNudges(): void {
    if (this.nudgeId !== null || this.frozen) return;
    // Rates and the leader drift as the window moves; adapt gently, but only
    // when something actually changed, so a steady map comes to rest.
    this.nudgeId = this.scheduler.every(NUDGE_EVERY_MS, () => {
      if (!this.model) return;
      const now = this.scheduler.now();
      const { grown, reclassed } = this.refreshEdges(now);
      const grownBadges = this.refreshBadges(now);
      const before = this.layout.alpha;
      reheatAround(this.layout, [...grown, ...this.leaderMoves(now)], this.edges, GROWTH_ALPHA);
      reheatAround(this.layout, [...reclassed, ...grownBadges], this.edges, NUDGE_ALPHA);
      if (this.layout.alpha > before) this.wake();
    });
  }

  private stopNudges(): void {
    if (this.nudgeId !== null) this.scheduler.cancelEvery(this.nudgeId);
    this.nudgeId = null;
  }

  private cancelFrame(): void {
    if (this.frameId !== null) this.scheduler.cancelFrame(this.frameId);
    this.frameId = null;
  }

  private wake(): void {
    if (!this.motion || this.frameId !== null) return;
    const settling = !this.frozen && this.layout.alpha > 0;
    if (settling || this.pulses.length > 0) this.frameId = this.scheduler.frame((now) => this.frame(now));
  }

  private frame(now: number): void {
    this.frameId = null;
    if (now - this.lastDrawnAt < FRAME_INTERVAL_MS) {
      this.wake();
      return;
    }
    this.lastDrawnAt = now;
    this.metrics.frames += 1;
    if (!this.frozen && this.layout.alpha > 0) {
      // Cooling follows the clock, not the frame count: when frames come
      // slowly (an overloaded page), a reheat still settles in about the
      // same wall time instead of keeping the expensive phase alive longer.
      const elapsed = now - this.lastStepAt <= MAX_COOLING_SPAN_MS ? now - this.lastStepAt : FRAME_INTERVAL_MS;
      this.lastStepAt = now;
      const cooling = ALPHA_DECAY ** Math.max(1, elapsed / FRAME_INTERVAL_MS);
      const moved = step(this.layout, this.edges, { leader: this.leader, badgeLines: this.badgeLines, cooling });
      if (moved.size > 0) this.metrics.steps += 1;
      const visible = new Set<string>();
      for (const id of moved) {
        const node = this.layout.nodes.get(id);
        if (!node) continue;
        const last = this.drawnAt.get(id);
        if (last && Math.abs(node.x - last.x) < MIN_DRAWN_MOVE && Math.abs(node.y - last.y) < MIN_DRAWN_MOVE) continue;
        this.drawnAt.set(id, { x: node.x, y: node.y });
        visible.add(id);
      }
      if (visible.size > 0) this.sink.positions(this.layout.nodes, visible);
    }
    this.pulses = this.pulses.filter((pulse) => now - pulse.start < PULSE_MS);
    this.sink.pulses(this.activePulses());
    this.wake();
  }
}
