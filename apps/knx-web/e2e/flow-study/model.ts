/** U19 study model: session-local flow graph, value slots, sender window and pulse bundling. */
// Study code for the telegram-flow contract (docs/TELEGRAM_FLOW_VISUALIZATION.md).
// It runs on synthetic events only and is not wired to the monitor feed; U20
// owns the productive reducer. Pure data, no DOM, no timers: callers pass the
// monotonic time explicitly, so expiry and windows are deterministic.

export const VALUE_TTL_MS = 7_000;
export const WINDOW_MS = 60_000;
export const MAX_BADGES = 3;

export type StudyService = "GroupValueWrite" | "GroupValueRead" | "GroupValueResponse";

/** One admitted observation. `targets` are configured project participants,
 * never measured receivers; an empty list means the group is unresolved. */
export interface StudyEvent {
  seq: number;
  observedAtMs: number;
  source: string;
  ga: string;
  service: StudyService;
  value?: string;
  targets: string[];
}

export interface Slot {
  ga: string;
  value: string;
  seq: number;
  observedAtMs: number;
  source: string;
  /** True when the device is a configured target, not the observed sender. */
  inferred: boolean;
}

export interface EdgeState {
  from: string;
  to: string;
  count: number;
  lastAtMs: number;
  gas: Map<string, number>;
}

export interface FlowLimits {
  maxNodes: number;
  maxEdges: number;
}

export interface FlowState {
  limits: FlowLimits;
  lastSeq: number;
  nodes: Set<string>;
  edges: Map<string, EdgeState>;
  /** Value slots indexed per device, then per group address. */
  slots: Map<string, Map<string, Slot>>;
  slotCount: number;
  sends: Map<string, number[]>;
  leader: string | null;
  overflow: { refusedNodes: number; refusedEdges: number };
}

export const DEFAULT_LIMITS: FlowLimits = { maxNodes: 1_000, maxEdges: 5_000 };

export function createFlowState(limits: FlowLimits = DEFAULT_LIMITS): FlowState {
  return {
    limits, lastSeq: -Infinity, nodes: new Set(), edges: new Map(), slots: new Map(), slotCount: 0, sends: new Map(),
    leader: null, overflow: { refusedNodes: 0, refusedEdges: 0 },
  };
}

export const groupNode = (ga: string) => `GA:${ga}`;
const edgeKey = (from: string, to: string) => `${from}→${to}`;

function ensureNode(state: FlowState, id: string): boolean {
  if (state.nodes.has(id)) return true;
  if (state.nodes.size >= state.limits.maxNodes) {
    state.overflow.refusedNodes += 1;
    return false;
  }
  state.nodes.add(id);
  return true;
}

function touchEdge(state: FlowState, from: string, to: string, event: StudyEvent): void {
  const key = edgeKey(from, to);
  let edge = state.edges.get(key);
  if (!edge) {
    if (state.edges.size >= state.limits.maxEdges || !ensureNode(state, from) || !ensureNode(state, to)) {
      state.overflow.refusedEdges += 1;
      return;
    }
    edge = { from, to, count: 0, lastAtMs: event.observedAtMs, gas: new Map() };
    state.edges.set(key, edge);
  }
  edge.count += 1;
  edge.lastAtMs = Math.max(edge.lastAtMs, event.observedAtMs);
  edge.gas.set(event.ga, (edge.gas.get(event.ga) ?? 0) + 1);
}

function setSlot(state: FlowState, device: string, event: StudyEvent, inferred: boolean): void {
  if (event.value === undefined) return;
  let own = state.slots.get(device);
  if (!own) { own = new Map(); state.slots.set(device, own); }
  if (!own.has(event.ga)) state.slotCount += 1;
  own.set(event.ga, {
    ga: event.ga, value: event.value, seq: event.seq, observedAtMs: event.observedAtMs, source: event.source, inferred,
  });
}

/** Admits one observation. Sequences at or below the high-water mark are a
 * repeated or stale delivery and change nothing. */
export function admit(state: FlowState, event: StudyEvent): FlowState {
  if (event.seq <= state.lastSeq) return state;
  state.lastSeq = event.seq;
  const sends = state.sends.get(event.source) ?? [];
  sends.push(event.observedAtMs);
  state.sends.set(event.source, sends);
  const targets = event.targets.length > 0 ? event.targets : [groupNode(event.ga)];
  ensureNode(state, event.source);
  for (const target of targets) touchEdge(state, event.source, target, event);
  // Reads carry no value and must not renew a deadline; Write/Response
  // update the sender and every configured target immediately.
  if (event.service !== "GroupValueRead") {
    setSlot(state, event.source, event, false);
    for (const target of event.targets) setSlot(state, target, event, true);
  }
  return state;
}

/** Current value slots of a device, newest first, at most MAX_BADGES. */
export function badges(state: FlowState, device: string, nowMs: number): { current: Slot[]; overflow: number } {
  const live: Slot[] = [];
  for (const slot of state.slots.get(device)?.values() ?? []) {
    if (nowMs - slot.observedAtMs < VALUE_TTL_MS) live.push(slot);
  }
  live.sort((a, b) => b.observedAtMs - a.observedAtMs || b.seq - a.seq);
  return { current: live.slice(0, MAX_BADGES), overflow: Math.max(0, live.length - MAX_BADGES) };
}

/** Drops slots that expired; values are never shown past their deadline. */
export function expire(state: FlowState, nowMs: number): void {
  for (const [device, own] of state.slots) {
    for (const [ga, slot] of own) {
      if (nowMs - slot.observedAtMs >= VALUE_TTL_MS) { own.delete(ga); state.slotCount -= 1; }
    }
    if (own.size === 0) state.slots.delete(device);
  }
}

/** The most active observed sender in the rolling window. Exact ties keep the
 * current leader; otherwise the lowest identity wins; no traffic, no leader. */
export function currentLeader(state: FlowState, nowMs: number): string | null {
  const counts = new Map<string, number>();
  for (const [source, times] of state.sends) {
    const recent = times.filter((at) => at > nowMs - WINDOW_MS);
    state.sends.set(source, recent);
    if (recent.length > 0) counts.set(source, recent.length);
  }
  if (counts.size === 0) { state.leader = null; return null; }
  const best = Math.max(...counts.values());
  if (state.leader !== null && counts.get(state.leader) === best) return state.leader;
  state.leader = [...counts].filter(([, count]) => count === best).map(([id]) => id).sort()[0];
  return state.leader;
}

export interface Pulse {
  from: string;
  to: string;
  ga: string;
  count: number;
  value?: string;
  seq: number;
}

/** Bundles the pulses of one render interval per directed pair and group
 * address, keeping how many observations each bundle represents. */
export function coalescePulses(events: readonly StudyEvent[]): Pulse[] {
  const bundles = new Map<string, Pulse>();
  for (const event of events) {
    const targets = event.targets.length > 0 ? event.targets : [groupNode(event.ga)];
    for (const to of targets) {
      const key = `${event.source}\u0000${to}\u0000${event.ga}`;
      const bundle = bundles.get(key);
      if (!bundle) {
        bundles.set(key, { from: event.source, to, ga: event.ga, count: 1, value: event.value, seq: event.seq });
      } else {
        bundle.count += 1;
        if (event.seq > bundle.seq) { bundle.seq = event.seq; bundle.value = event.value ?? bundle.value; }
      }
    }
  }
  return [...bundles.values()];
}
