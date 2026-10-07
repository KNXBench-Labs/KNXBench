/** Shares one source monitor's disposable flow model with read-only same-origin windows. */
import { useEffect, useRef, useState } from "react";
import { createFlowScope } from "./flowIdentity";
import type { FlowFeed, FlowSourceDiagnostics } from "./flowFeed";
import { cloneFlowAtOrigin } from "./flowPresentation";
import { DEFAULT_FLOW_LIMITS, type FlowModel } from "./flowModel";
import { nodeFlowTarget, validFlowTarget, type FlowNavigation, type FlowTarget } from "./flowNavigation";

export const flowTargetKey = (t: FlowTarget) => `${t.generation}:${t.kind}:${t.id}`;
export type FlowPacket = { type: "snapshot"; revision: number; origin: number; sentAt: number; model: FlowModel | null; links: Set<string>; paused: boolean; source?: FlowSourceDiagnostics } | { type: "closed" };
export interface FlowMirror { revision: number; model: FlowModel | null; links: Set<string>; paused: boolean; live: boolean; receivedAt: number; source?: FlowSourceDiagnostics }
export const initialFlowMirror = (): FlowMirror => ({ revision: 0, model: null, links: new Set(), paused: false, live: false, receivedAt: -Infinity });

/** The channel is same-origin and names a single random source lifetime.
 * Reject malformed envelopes/revisions rather than replacing a valid map.
 * It is UI transport, not an API or a bus/project-write authority. */
export function acceptFlowPacket(state: FlowMirror, input: unknown, origin: number, now: number): FlowMirror {
  if (!input || typeof input !== "object") return state;
  const p = input as FlowPacket;
  if (p.type === "closed") return { ...state, live: false, links: new Set() };
  if (p.type !== "snapshot" || !Number.isSafeInteger(p.revision) || p.revision <= state.revision || !Number.isFinite(p.origin) || !Number.isFinite(p.sentAt) || p.sentAt < 0 || !(p.links instanceof Set) || typeof p.paused !== "boolean") return state;
  if (p.source && (!Number.isSafeInteger(p.source.gaps) || p.source.gaps < 0 || !Number.isSafeInteger(p.source.pruned) || p.source.pruned < 0 || !["synced", "stale", "unverified"].includes(p.source.context) || typeof p.source.ended !== "boolean" || (p.source.error !== null && typeof p.source.error !== "string"))) return state;
  const m = p.model;
  if (m !== null && (!m || !(m.nodes instanceof Map) || !(m.edges instanceof Map) || !(m.contexts instanceof Map) || !(m.slots instanceof Map) || !(m.sendTimes instanceof Map) || !Array.isArray(m.pending) || !Array.isArray(m.events) || !m.identity || typeof m.identity.serverIncarnation !== "string" || !Number.isSafeInteger(m.identity.sessionId) || m.nodes.size > DEFAULT_FLOW_LIMITS.maxNodes || m.edges.size > DEFAULT_FLOW_LIMITS.maxEdges || m.slotCount > DEFAULT_FLOW_LIMITS.maxSlots || m.pending.length > DEFAULT_FLOW_LIMITS.maxPending)) return state;
  try {
    const receivedAt = Math.min(now, p.sentAt + p.origin - origin);
    const live = now - receivedAt <= 6000;
    return { revision: p.revision, model: m === null ? null : cloneFlowAtOrigin(m, p.origin, origin), links: live ? new Set(p.links) : new Set(), paused: p.paused, live, receivedAt, source: p.source ? { ...p.source } : undefined };
  } catch { return state; }
}

function targetsOf(model: FlowModel | null): Map<string, FlowTarget> {
  const result = new Map<string, FlowTarget>();
  const add = (target: FlowTarget) => result.set(flowTargetKey(target), target);
  if (!model) return result;
  for (const node of model.nodes.values()) { const target = nodeFlowTarget(node); if (target) add(target); }
  for (const edge of model.edges.values()) for (const [id, group] of edge.groups) add({ kind: "group", id, generation: group.generation });
  for (const slots of model.slots.values()) for (const slot of slots.values()) add({ kind: "group", id: slot.gaRaw, generation: slot.generation });
  return result;
}
const channelName = (id: string) => `knxbench-flow:${id}`;
export const validFlowOwner = (id: string | null): id is string => !!id && /^[a-zA-Z0-9_-]{1,100}$/.test(id);

/** Source lifetime survives view/navigation switches because its monitor
 * stays mounted. Only subscribed windows receive full snapshots; no traffic
 * is persisted, no second polling loop/tunnel is created by this hook. */
export function useFlowOwner(feed: FlowFeed, navigation?: FlowNavigation, paused = false) {
  const [id] = useState(() => createFlowScope());
  const latest = useRef({ feed, navigation, paused }); latest.current = { feed, navigation, paused };
  const sendRef = useRef<(() => void) | null>(null);
  const [available, setAvailable] = useState(false);
  useEffect(() => {
    if (typeof BroadcastChannel === "undefined") return;
    let channel: BroadcastChannel;
    try { channel = new BroadcastChannel(channelName(id)); } catch { return; }
    setAvailable(true);
    let revision = 0;
    let disposed = false;
    const clients = new Map<string, number>();
    const send = () => {
      for (const [client, seen] of clients) if (performance.now() - seen > 10000) clients.delete(client);
      if (!clients.size) return;
      const { feed: current, navigation: nav, paused: isPaused } = latest.current;
      const links = new Set([...targetsOf(current.model)].filter(([, target]) => nav?.available(target)).map(([key]) => key));
      try { channel.postMessage({ type: "snapshot", revision: ++revision, origin: performance.timeOrigin, sentAt: performance.now(), model: current.model, links, paused: isPaused, source: current.source } satisfies FlowPacket); } catch { /* A closed source never claims successful delivery. */ }
    };
    sendRef.current = send;
    channel.onmessage = async ({ data }) => {
      if (!data || typeof data !== "object" || typeof data.client !== "string" || data.client.length > 100) return;
      if (data.type === "request") { clients.set(data.client, performance.now()); send(); }
      if (data.type === "navigate" && validFlowTarget(data.target) && typeof data.request === "string") {
        const { feed: current, navigation: nav } = latest.current;
        const same = current.model && data.identity?.sessionId === current.model.identity.sessionId && data.identity?.serverIncarnation === current.model.identity.serverIncarnation;
        let ok = false;
        try { if (same && nav?.available(data.target)) ok = await nav.open(data.target); } catch { /* Report refusal to the requesting window. */ }
        if (!disposed) channel.postMessage({ type: "navigationResult", client: data.client, request: data.request, ok });
      }
    };
    return () => {
      disposed = true;
      sendRef.current = null;
      channel.postMessage({ type: "closed" } satisfies FlowPacket); channel.close();
    };
  }, [id]);
  useEffect(() => { sendRef.current?.(); }, [feed.version, navigation, paused]);
  return { id, available };
}

export function useFlowMirror(owner: string | null) {
  const [state, setState] = useState(initialFlowMirror);
  const stateRef = useRef(state); stateRef.current = state;
  const pending = useRef(new Map<string, { resolve: (ok: boolean) => void; timer: ReturnType<typeof setTimeout> }>());
  const portRef = useRef<BroadcastChannel | null>(null);
  const [client] = useState(() => createFlowScope());
  useEffect(() => {
    if (!validFlowOwner(owner) || typeof BroadcastChannel === "undefined") return;
    let port: BroadcastChannel;
    try { port = new BroadcastChannel(channelName(owner)); } catch { return; }
    portRef.current = port;
    port.onmessage = ({ data }) => {
      if (data?.type === "navigationResult" && data.client === client) {
        const request = pending.current.get(data.request);
        if (request) { clearTimeout(request.timer); pending.current.delete(data.request); request.resolve(data.ok === true); }
        return;
      }
      setState(current => acceptFlowPacket(current, data, performance.timeOrigin, performance.now()));
    };
    const request = () => {
      port.postMessage({ type: "request", client });
      setState(current => current.live && performance.now() - current.receivedAt > 6000 ? { ...current, live: false, links: new Set() } : current);
    };
    request(); const timer = setInterval(request, 2000);
    return () => {
      clearInterval(timer); port.close(); portRef.current = null;
      for (const p of pending.current.values()) { clearTimeout(p.timer); p.resolve(false); }
      pending.current.clear();
    };
  }, [owner, client]);
  const navigation: FlowNavigation = {
    available: target => state.live && state.links.has(flowTargetKey(target)),
    open: target => new Promise<boolean>(resolve => {
      const port = portRef.current;
      if (!port || !stateRef.current.live || !stateRef.current.links.has(flowTargetKey(target))) { resolve(false); return; }
      const request = createFlowScope();
      const timer = setTimeout(() => { pending.current.delete(request); resolve(false); }, 5000);
      pending.current.set(request, { resolve, timer });
      port.postMessage({ type: "navigate", client, request, target, identity: stateRef.current.model?.identity });
    }),
  };
  const feed = { model: state.model, version: state.revision, source: state.source };
  return { state, feed, navigation };
}
