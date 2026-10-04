/** U20: session-keyed telegram-flow reducer: identities, targets, value slots and limits. */
// Contract: docs/TELEGRAM_FLOW_VISUALIZATION.md §1, §2 (consequences), §4
// (deterministic presentation state) and §10. Pure data: no DOM, no timers;
// callers pass the monotonic time, so admission, expiry and refusals are
// deterministic and testable with a fake clock.
//
// One model belongs to one monitor session (`serverIncarnation` +
// `sessionId`); another session gets a new model. Every row is resolved with
// the participant context of its own `flowGeneration`, fixed when the server
// decoded it — never with a newer project. Rows whose context is not known
// yet wait in a bounded, sequence-ordered queue.

import { rowFlowFacts, type FlowDevice, type FlowGroup, type FlowMember, type FlowSnapshot } from "./flowWire";

export const VALUE_TTL_MS = 7_000;
export const MAX_BADGES = 3;

export interface FlowModelLimits {
  maxNodes: number;
  maxEdges: number;
  maxSlots: number;
  maxPending: number;
}

export const DEFAULT_FLOW_LIMITS: FlowModelLimits = { maxNodes: 1_000, maxEdges: 5_000, maxSlots: 10_000, maxPending: 5_000 };

/** The fields of a monitor row the flow reads; `BusTelegramRow` satisfies it. */
export interface FlowRowInput {
  seq: number;
  service: string;
  source: string;
  destination: string;
  decoded: { kind: string; text: string } | null;
  sourceRaw?: unknown;
  destinationRaw?: unknown;
  observedAgeMs?: unknown;
  flowGeneration?: unknown;
}

/** Why a row was drawn without participants. */
export type RawReason = "historical" | "unavailable" | "failed" | "pendingOverflow";

export type FlowNodeKind = "device" | "unknownDevice" | "ambiguousSource" | "unresolvedSource" | "rawSource" | "group";

export interface FlowNode {
  id: string;
  kind: FlowNodeKind;
  label: string;
  deviceId?: number;
  /** The observed individual address as the monitor formatted it. */
  address?: string;
  /** Devices holding an ambiguous source address. */
  candidates?: number[];
  /** Several project groups share this raw group address. */
  ambiguous?: boolean;
  /** Set on raw nodes: why no participant could be named. */
  context?: RawReason;
}

/** One linked object as the project states it; configuration, not receipt. */
export type FlowObjectEvidence = Omit<FlowMember, "deviceId">;

/** Per group address on an edge: how often, and which objects link it. */
export interface FlowEdgeGroup {
  label: string;
  count: number;
  /** Generation the object evidence below was read from (the latest row's). */
  generation: string;
  sourceObjects: FlowObjectEvidence[];
  targetObjects: FlowObjectEvidence[];
}

export interface FlowEdge {
  id: string;
  from: string;
  to: string;
  /** The target is a project-configured member, not an observed receiver. */
  configured: boolean;
  count: number;
  lastSeq: number;
  lastObservedAtMs: number;
  /** Raw group address → label, count and linked-object evidence. */
  groups: Map<number, FlowEdgeGroup>;
}

export interface ValueSlot {
  gaRaw: number;
  gaLabel: string;
  value: string;
  seq: number;
  observedAtMs: number;
  deadlineMs: number;
  generation: string;
  /** `configuredTarget`: inferred from project membership, not readback. */
  origin: "source" | "configuredTarget" | "group";
  sourceNode: string;
  sourceLabel: string;
}

interface ParticipantContext {
  kind: "participants";
  complete: boolean;
  devicesById: Map<number, FlowDevice>;
  devicesByAddress: Map<number, number[]>;
  groupsByAddress: Map<number, FlowGroup[]>;
}

export type FlowContext = { kind: "pending" } | ParticipantContext | { kind: "raw"; reason: RawReason };

interface QueuedRow {
  row: FlowRowInput;
  sourceRaw: number;
  destinationRaw: number;
  generation: string;
  /** Monotonic observation time, or null when the age is unknown. */
  observedAtMs: number | null;
}

export interface FlowCounters {
  admitted: number;
  duplicates: number;
  legacy: number;
  malformed: number;
  markers: number;
  reads: number;
  noValue: number;
  unknownAge: number;
  refusedNodes: number;
  refusedEdges: number;
  refusedSlots: number;
  pendingOverflow: number;
}

export interface FlowModel {
  identity: { serverIncarnation: string; sessionId: number };
  limits: FlowModelLimits;
  /** Highest sequence seen, admitted or queued; lower ones are repeats. */
  highWaterSeq: number;
  contexts: Map<string, FlowContext>;
  pending: QueuedRow[];
  nodes: Map<string, FlowNode>;
  edges: Map<string, FlowEdge>;
  slots: Map<string, Map<number, ValueSlot>>;
  slotCount: number;
  closed: boolean;
  counters: FlowCounters;
}

export function createFlowModel(
  identity: { serverIncarnation: string; sessionId: number },
  limits: FlowModelLimits = DEFAULT_FLOW_LIMITS,
): FlowModel {
  return {
    identity,
    limits,
    highWaterSeq: -1,
    contexts: new Map(),
    pending: [],
    nodes: new Map(),
    edges: new Map(),
    slots: new Map(),
    slotCount: 0,
    closed: false,
    counters: {
      admitted: 0, duplicates: 0, legacy: 0, malformed: 0, markers: 0, reads: 0, noValue: 0, unknownAge: 0,
      refusedNodes: 0, refusedEdges: 0, refusedSlots: 0, pendingOverflow: 0,
    },
  };
}

function participants(snapshot: FlowSnapshot): FlowContext {
  if (snapshot.status !== "current") return { kind: "raw", reason: snapshot.status };
  const devicesById = new Map<number, FlowDevice>();
  const devicesByAddress = new Map<number, number[]>();
  for (const device of snapshot.devices) {
    devicesById.set(device.deviceId, device);
    if (device.individualAddressRaw === null) continue;
    const holders = devicesByAddress.get(device.individualAddressRaw) ?? [];
    holders.push(device.deviceId);
    devicesByAddress.set(device.individualAddressRaw, holders);
  }
  for (const holders of devicesByAddress.values()) holders.sort((a, b) => a - b);
  const groupsByAddress = new Map<number, FlowGroup[]>();
  for (const group of snapshot.groups) {
    const same = groupsByAddress.get(group.gaRaw) ?? [];
    same.push(group);
    groupsByAddress.set(group.gaRaw, same);
  }
  const { devices, groups, members } = snapshot.truncated;
  return { kind: "participants", complete: devices + groups + members === 0, devicesById, devicesByAddress, groupsByAddress };
}

/**
 * Supplies the participant context of `generation`: a validated snapshot, or
 * `null` when it could not be fetched (rows are then drawn raw). A snapshot
 * of another session, server or generation is refused (`false`); a context
 * that is already known is kept. Queued rows are admitted in sequence order.
 */
export function provideContext(model: FlowModel, generation: string, snapshot: FlowSnapshot | null, nowMs: number): boolean {
  if (snapshot !== null) {
    const { serverIncarnation, sessionId } = model.identity;
    if (snapshot.serverIncarnation !== serverIncarnation || snapshot.sessionId !== sessionId) return false;
    if (snapshot.generation !== generation) return false;
  }
  const known = model.contexts.get(generation);
  if (known === undefined || known.kind === "pending") {
    model.contexts.set(generation, snapshot === null ? { kind: "raw", reason: "failed" } : participants(snapshot));
  }
  drain(model, nowMs);
  return true;
}

/**
 * Admits one poll batch received at `receivedAtMs` (monotonic). Returns the
 * generations seen for the first time: the caller fetches their snapshot
 * once and hands it to `provideContext`.
 */
export function admitRows(model: FlowModel, rows: readonly FlowRowInput[], receivedAtMs: number): string[] {
  const needed: string[] = [];
  for (const row of [...rows].sort((a, b) => a.seq - b.seq)) {
    if (row.seq <= model.highWaterSeq) {
      model.counters.duplicates += 1;
      continue;
    }
    model.highWaterSeq = row.seq;
    const facts = rowFlowFacts(row);
    if (facts.kind === "legacy") { model.counters.legacy += 1; continue; }
    if (facts.kind === "malformed") { model.counters.malformed += 1; continue; }
    if (facts.kind === "marker") { model.counters.markers += 1; model.closed = true; continue; }
    if (!model.contexts.has(facts.flowGeneration)) {
      model.contexts.set(facts.flowGeneration, { kind: "pending" });
      needed.push(facts.flowGeneration);
    }
    model.pending.push({
      row,
      sourceRaw: facts.sourceRaw,
      destinationRaw: facts.destinationRaw,
      generation: facts.flowGeneration,
      observedAtMs: facts.observedAgeMs === null ? null : receivedAtMs - facts.observedAgeMs,
    });
    // A full queue does not grow: its oldest row is drawn raw now.
    while (model.pending.length > model.limits.maxPending) {
      model.counters.pendingOverflow += 1;
      apply(model, model.pending.shift()!, { kind: "raw", reason: "pendingOverflow" }, receivedAtMs);
    }
  }
  drain(model, receivedAtMs);
  return needed;
}

function drain(model: FlowModel, nowMs: number): void {
  while (model.pending.length > 0) {
    const head = model.pending[0];
    const context = model.contexts.get(head.generation);
    if (context === undefined || context.kind === "pending") return;
    model.pending.shift();
    apply(model, head, context, nowMs);
  }
}

function ensureNode(model: FlowModel, node: FlowNode): boolean {
  const existing = model.nodes.get(node.id);
  if (existing) {
    // The latest admitted evidence describes the node; identity stays. A
    // changed kind replaces every field, so no earlier ambiguity or raw
    // reason outlives the generation that stated it.
    model.nodes.set(node.id, existing.kind === node.kind ? { ...existing, ...node } : { ...node });
    return true;
  }
  if (model.nodes.size >= model.limits.maxNodes) {
    model.counters.refusedNodes += 1;
    return false;
  }
  model.nodes.set(node.id, { ...node });
  return true;
}

function deviceNode(context: ParticipantContext, deviceId: number): FlowNode {
  const device = context.devicesById.get(deviceId);
  return device
    ? { id: `d:${deviceId}`, kind: "device", label: device.name, deviceId }
    : { id: `d:${deviceId}`, kind: "unknownDevice", label: `#${deviceId}`, deviceId };
}

interface Resolution {
  source: FlowNode;
  sourceLabel: string;
  targets: FlowNode[];
  /** The one project group with this address, when there is exactly one. */
  group: FlowGroup | null;
}

function resolve(entry: QueuedRow, context: ParticipantContext | { kind: "raw"; reason: RawReason }): Resolution {
  const { row, sourceRaw, destinationRaw } = entry;
  const groupNode: FlowNode = { id: `g:${destinationRaw}`, kind: "group", label: row.destination, ambiguous: false };
  if (context.kind === "raw") {
    return {
      source: { id: `ia:${sourceRaw}`, kind: "rawSource", label: row.source, address: row.source, context: context.reason },
      sourceLabel: row.source,
      targets: [groupNode],
      group: null,
    };
  }
  const holders = context.devicesByAddress.get(sourceRaw) ?? [];
  let source: FlowNode;
  let sourceLabel = row.source;
  if (holders.length === 1) {
    source = { ...deviceNode(context, holders[0]), address: row.source };
    sourceLabel = `${source.label} (${row.source})`;
  } else if (holders.length > 1) {
    source = { id: `ia:${sourceRaw}`, kind: "ambiguousSource", label: row.source, address: row.source, candidates: [...holders] };
  } else {
    source = { id: `ia:${sourceRaw}`, kind: "unresolvedSource", label: row.source, address: row.source };
  }
  // Configured endpoints: every active member of the one group with this
  // address (Send or Receive), except whatever may have sent the telegram.
  const groups = context.groupsByAddress.get(destinationRaw) ?? [];
  if (groups.length !== 1) return { source, sourceLabel, targets: [{ ...groupNode, ambiguous: groups.length > 1 }], group: null };
  const excluded = new Set(holders);
  const ids = [...new Set(groups[0].members.filter((m) => m.active && !excluded.has(m.deviceId)).map((m) => m.deviceId))];
  ids.sort((a, b) => a - b);
  const targets = ids.map((id) => deviceNode(context, id));
  return { source, sourceLabel, targets: targets.length > 0 ? targets : [{ ...groupNode, ambiguous: false }], group: groups[0] };
}

function objectsOf(group: FlowGroup | null, node: FlowNode): FlowObjectEvidence[] {
  if (group === null || node.deviceId === undefined) return [];
  return group.members
    .filter((m) => m.deviceId === node.deviceId)
    .map(({ comObjectId, direction, active, flags }) => ({ comObjectId, direction, active, flags }));
}

function touchEdge(model: FlowModel, source: FlowNode, to: FlowNode, entry: QueuedRow, group: FlowGroup | null): void {
  const from = source.id;
  const id = `${from}→${to.id}`;
  let edge = model.edges.get(id);
  if (!edge) {
    if (model.edges.size >= model.limits.maxEdges) {
      model.counters.refusedEdges += 1;
      return;
    }
    edge = {
      id, from, to: to.id, configured: to.kind === "device" || to.kind === "unknownDevice",
      count: 0, lastSeq: entry.row.seq, lastObservedAtMs: entry.observedAtMs ?? Number.NEGATIVE_INFINITY, groups: new Map(),
    };
    model.edges.set(id, edge);
  }
  edge.count += 1;
  edge.lastSeq = Math.max(edge.lastSeq, entry.row.seq);
  if (entry.observedAtMs !== null) edge.lastObservedAtMs = Math.max(edge.lastObservedAtMs, entry.observedAtMs);
  const previous = edge.groups.get(entry.destinationRaw);
  edge.groups.set(entry.destinationRaw, {
    label: entry.row.destination,
    count: (previous?.count ?? 0) + 1,
    generation: entry.generation,
    sourceObjects: objectsOf(group, source),
    targetObjects: objectsOf(group, to),
  });
}

function setSlot(model: FlowModel, nodeId: string, slot: ValueSlot): void {
  if (!model.nodes.has(nodeId)) return;
  let own = model.slots.get(nodeId);
  const existing = own?.get(slot.gaRaw);
  if (existing && existing.seq >= slot.seq) return;
  if (!existing && model.slotCount >= model.limits.maxSlots) {
    model.counters.refusedSlots += 1;
    return;
  }
  if (!own) {
    own = new Map();
    model.slots.set(nodeId, own);
  }
  if (!existing) model.slotCount += 1;
  own.set(slot.gaRaw, slot);
}

const VALUE_SERVICES = new Set(["GroupValueWrite", "GroupValueResponse"]);

function apply(model: FlowModel, entry: QueuedRow, context: FlowContext, nowMs: number): void {
  if (context.kind === "pending") throw new Error("flow: a queued row was applied before its context was known");
  model.counters.admitted += 1;
  const { source, sourceLabel, targets, group } = resolve(entry, context);
  const sourceKept = ensureNode(model, source);
  const keptTargets = targets.filter((target) => ensureNode(model, target));
  if (sourceKept) {
    for (const target of keptTargets) touchEdge(model, source, target, entry, group);
  } else if (keptTargets.length > 0) {
    model.counters.refusedEdges += keptTargets.length;
  }

  const { row } = entry;
  if (!VALUE_SERVICES.has(row.service)) {
    if (row.service === "GroupValueRead") model.counters.reads += 1;
    return;
  }
  if (row.decoded?.kind !== "value") {
    model.counters.noValue += 1;
    return;
  }
  if (entry.observedAtMs === null) {
    model.counters.unknownAge += 1;
    return;
  }
  const deadlineMs = entry.observedAtMs + VALUE_TTL_MS;
  // A value that was already past its lifetime when it arrived is history.
  if (deadlineMs <= nowMs) return;
  const base = {
    gaRaw: entry.destinationRaw, gaLabel: row.destination, value: row.decoded.text, seq: row.seq,
    observedAtMs: entry.observedAtMs, deadlineMs, generation: entry.generation, sourceNode: source.id, sourceLabel,
  };
  setSlot(model, source.id, { ...base, origin: "source" });
  for (const target of targets) {
    setSlot(model, target.id, { ...base, origin: target.kind === "group" ? "group" : "configuredTarget" });
  }
}

/** Current values of one node at `nowMs`: newest first, at most MAX_BADGES. */
export function currentBadges(model: FlowModel, nodeId: string, nowMs: number): { current: ValueSlot[]; overflow: number } {
  const live = [...(model.slots.get(nodeId)?.values() ?? [])].filter((slot) => slot.deadlineMs > nowMs);
  live.sort((a, b) => b.observedAtMs - a.observedAtMs || b.seq - a.seq);
  return { current: live.slice(0, MAX_BADGES), overflow: Math.max(0, live.length - MAX_BADGES) };
}

/** Every current value of one node, for the Inspector. */
export function allCurrentValues(model: FlowModel, nodeId: string, nowMs: number): ValueSlot[] {
  const live = [...(model.slots.get(nodeId)?.values() ?? [])].filter((slot) => slot.deadlineMs > nowMs);
  return live.sort((a, b) => b.observedAtMs - a.observedAtMs || b.seq - a.seq);
}

/** Drops every slot whose deadline has passed; a value is never shown past it. */
export function expireSlots(model: FlowModel, nowMs: number): void {
  for (const [nodeId, own] of model.slots) {
    for (const [gaRaw, slot] of own) {
      if (slot.deadlineMs <= nowMs) {
        own.delete(gaRaw);
        model.slotCount -= 1;
      }
    }
    if (own.size === 0) model.slots.delete(nodeId);
  }
}

/** The earliest deadline among current slots, for one re-render timer. */
export function nextExpiryAt(model: FlowModel): number | null {
  let next: number | null = null;
  for (const own of model.slots.values()) {
    for (const slot of own.values()) if (next === null || slot.deadlineMs < next) next = slot.deadlineMs;
  }
  return next;
}
