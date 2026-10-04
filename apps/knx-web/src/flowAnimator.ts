/** U21: drives the flow view's motion: solver steps, pulses, nudges; stops when idle or off. */
// No React and no DOM: a scheduler (frames, timers, clock, visibility) and a
// sink (where positions and pulses are drawn) are injected, so tests can
// prove what really stops. Values are never touched here: the reducer owns
// them and admits data immediately; a pulse is illustration only and has no
// write authority (docs/TELEGRAM_FLOW_VISUALIZATION.md §1, §4).
import {
  ALPHA_MIN,
  RATE_SATURATION,
  createDynamics,
  ensureDynamicNodes,
  reheat,
  step,
  type DynamicEdge,
  type DynamicNode,
  type Dynamics,
} from "./flowDynamics";
import { FRESH_EVENT_MS, currentLeader, edgeActivity, type FlowEvent, type FlowModel } from "./flowModel";

export const PULSE_MS = 700;
/** Simultaneous pulse elements; beyond this telegrams are counted, not drawn. */
export const MAX_PULSES = 160;
/** More events than this in one batch are bundled per pair and group address. */
export const COALESCE_ABOVE = 24;
const NUDGE_EVERY_MS = 5_000;
const NUDGE_ALPHA = 0.08;
const GROWTH_ALPHA = 0.3;
const REFRESH_MS = 1_000;

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
  positions(nodes: ReadonlyMap<string, DynamicNode>): void;
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
  /** Telegrams that were drawn as part of a bundle. */
  coalescedEvents: number;
  /** Telegrams that found no free pulse element. */
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
  private edges: DynamicEdge[] = [];
  private edgeSignature = "";
  private leader: string | null = null;
  private model: FlowModel | null = null;

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
    const before = this.layout.nodes.size;
    ensureDynamicNodes(this.layout, model.nodes.keys());
    const edgeCount = this.edges.length;
    this.refreshEdges(now);
    const leader = currentLeader(model, now);
    const leaderChanged = leader !== this.leader;
    this.leader = leader;
    if (this.layout.nodes.size > before || this.edges.length > edgeCount || leaderChanged) reheat(this.layout, GROWTH_ALPHA);

    const fresh = model.events.filter((event) => event.seq > this.lastEventSeq);
    if (fresh.length > 0) this.lastEventSeq = fresh[fresh.length - 1].seq;
    // A hidden page draws nothing, and what happened meanwhile is not
    // replayed on return: only events observed just now become pulses.
    if (this.motion && !this.scheduler.hidden()) {
      this.queuePulses(fresh.filter((event) => event.observedAtMs >= now - FRESH_EVENT_MS), now);
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

  private refreshEdges(now: number): boolean {
    if (!this.model) return false;
    this.edges = [...this.model.edges.values()].map((edge) => ({ from: edge.from, to: edge.to, rate: edgeActivity(edge, now) }));
    const signature = this.edges.map((edge) => `${edge.from}>${edge.to}:${Math.min(edge.rate, RATE_SATURATION)}`).join("|");
    const changed = signature !== this.edgeSignature;
    this.edgeSignature = signature;
    return changed;
  }

  private queuePulses(events: readonly FlowEvent[], now: number): void {
    const bundles = new Map<string, Pulse>();
    const bundle = events.length > COALESCE_ABOVE;
    const add = (pulse: Pulse) => {
      if (this.pulses.length >= MAX_PULSES) {
        this.metrics.overCapacityEvents += pulse.count;
        this.metrics.reduced = true;
        return;
      }
      this.pulses.push(pulse);
    };
    for (const event of events) {
      for (const to of event.to) {
        const pulse = { from: event.from, to, gaLabel: event.gaLabel, count: 1, start: now };
        if (!bundle) {
          add(pulse);
          continue;
        }
        const key = `${event.from}\u0000${to}\u0000${event.gaRaw}`;
        const existing = bundles.get(key);
        if (existing) existing.count += 1;
        else bundles.set(key, pulse);
      }
    }
    if (bundle) {
      this.metrics.coalescedEvents += events.length;
      this.metrics.reduced = true;
      for (const pulse of bundles.values()) add(pulse);
    }
  }

  private startNudges(): void {
    if (this.nudgeId !== null || this.frozen) return;
    // Rates and the leader drift as the window moves; adapt gently, but only
    // when something actually changed, so a steady map comes to rest.
    this.nudgeId = this.scheduler.every(NUDGE_EVERY_MS, () => {
      if (!this.model) return;
      const now = this.scheduler.now();
      const leader = currentLeader(this.model, now);
      const leaderChanged = leader !== this.leader;
      this.leader = leader;
      if (this.refreshEdges(now) || leaderChanged) {
        reheat(this.layout, leaderChanged ? GROWTH_ALPHA : NUDGE_ALPHA);
        this.wake();
      }
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
    this.metrics.frames += 1;
    if (!this.frozen && this.layout.alpha >= ALPHA_MIN) {
      step(this.layout, this.edges, { leader: this.leader });
      this.metrics.steps += 1;
      this.sink.positions(this.layout.nodes);
    } else if (!this.frozen) {
      step(this.layout, this.edges, { leader: this.leader });
    }
    this.pulses = this.pulses.filter((pulse) => now - pulse.start < PULSE_MS);
    this.sink.pulses(this.activePulses());
    this.wake();
  }
}
