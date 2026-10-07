/** U20: read-only telegram-flow view: observed senders, configured targets, current values. */
// Renders the reducer state from `flowFeed` (docs/TELEGRAM_FLOW_VISUALIZATION.md
// §1, §2, §11). Nothing here polls, connects or writes. Values and edges are
// decoration for sighted users and hidden from assistive technology; every
// fact is reachable as text through keyboard selection and the Inspector,
// so a busy bus does not turn into a stream of announcements.
import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { FlowAnimator, type AnimatorScheduler, type AnimatorSink, type DrawnPulse } from "./flowAnimator";
import { type DynamicNode } from "./flowDynamics";
import { readableLayout, fitFlowCamera, flowFootprint, displayFlowLabel, routeFlowEdge, DETAILED_ROUTING_NODES, DETAILED_ROUTING_EDGES } from "./flowPresentation";
import { nodeFlowTarget, type FlowNavigation, type FlowTarget } from "./flowNavigation";
import { useFlowMotion } from "./flowMotion";
import { flowNow, type FlowFeed, type FlowSourceDiagnostics } from "./flowFeed";
import { BADGE_LINE, NODE_RADIUS, TEXT_CLEARANCE, edgeOpacity, pointOnEdge, type Point, type EdgeGeometry } from "./flowLayout";
import {
  allCurrentValues,
  currentBadges,
  currentLeader,
  type FlowEdge,
  type FlowModel,
  type FlowNode,
  type FlowObjectEvidence,
  type ValueSlot,
} from "./flowModel";
import { useTranslate, type Translate } from "./i18n";
import type { MessageKey } from "./messages/en";

const VIEW_WIDTH = 960;
const VIEW_HEIGHT = 520;
const ZOOM_STEP = 1.25;
const MIN_ZOOM = 0.01;
const MAX_ZOOM = 4;
const PAN_STEP = 60;
const EDGE_LABELS = 2;
// TEXT_CLEARANCE (flowLayout) is the distance from the circle to the node
// name (above) and to the first value badge (below): an arrowhead arriving
// straight from above or below ends in this gap instead of under the text's
// halo. The solver keeps neighbours clear of the same footprint.

interface View {
  zoom: number;
  x: number;
  y: number;
}

const HOME: View = { zoom: 1, x: 0, y: 0 };

function kindText(t: Translate, node: FlowNode): string {
  switch (node.kind) {
    case "device":
      return t("flow.kind.device");
    case "unknownDevice":
      return t("flow.kind.unknownDevice", { id: node.deviceId ?? "?" });
    case "ambiguousSource":
      return t("flow.kind.ambiguousSource", {
        address: node.address ?? node.label,
        devices: (node.candidates ?? []).map((id) => `#${id}`).join(", "),
      });
    case "unresolvedSource":
      return t("flow.kind.unresolvedSource", { address: node.address ?? node.label });
    case "rawSource":
      return t(`flow.raw.${node.context ?? "failed"}` as MessageKey);
    case "group":
      return node.ambiguous ? t("flow.kind.groupAmbiguous") : t("flow.kind.group");
  }
}

const ORIGIN_KEYS: Record<ValueSlot["origin"], MessageKey> = {
  source: "flow.origin.source",
  configuredTarget: "flow.origin.configuredTarget",
  group: "flow.origin.group",
};

const FLAG_NAMES = ["communication", "read", "write", "transmit", "update", "readOnInit"] as const;

function flagCell(t: Translate, value: boolean | null) {
  return value === null ? t("flow.flag.unknown") : value ? t("flow.flag.yes") : t("flow.flag.no");
}

// One narrow table per linked object: a caption with object, direction and
// activation, then one row per flag. Six flags side by side did not fit the
// Inspector column.
function ObjectTable({ objects }: { objects: FlowObjectEvidence[] }) {
  const t = useTranslate();
  if (objects.length === 0) return <p className="flow-inspector-note">{t("flow.inspector.noObjects")}</p>;
  return (
    <>
      {objects.map((object) => (
        <table className="flow-flags" key={object.comObjectId}>
          <caption>
            {t("flow.inspector.objectCaption", {
              id: object.comObjectId,
              direction: t(object.direction === "Send" ? "flow.direction.send" : "flow.direction.receive"),
              active: object.active ? t("flow.inspector.isActive") : t("flow.inspector.isInactive"),
            })}
          </caption>
          <tbody>
            {FLAG_NAMES.map((name) => (
              <tr key={name}>
                <th scope="row">{t(`flow.flagName.${name}` as MessageKey)}</th>
                <td>{flagCell(t, object.flags[name])}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ))}
    </>
  );
}

function ProjectLink({ target, navigation, children }: { target: FlowTarget | null; navigation?: FlowNavigation; children: React.ReactNode }) {
  const t = useTranslate();
  const [failed, setFailed] = useState(false);
  const [busy, setBusy] = useState(false);
  const available = !!target && !!navigation?.available(target);
  return <>
    <button type="button" className="flow-project-link" disabled={!available || busy}
      title={available ? undefined : t("flow.linkUnavailable")}
      onClick={async () => {
        if (!target || !navigation) return;
        setBusy(true); setFailed(false);
        try { setFailed(!await navigation.open(target)); } catch { setFailed(true); }
        finally { setBusy(false); }
      }}>{children}</button>
    {failed && <span role="status" className="flow-inspector-note">{t("flow.linkUnavailable")}</span>}
  </>;
}

function Connection({ edge, model, outgoing, navigation }: { edge: FlowEdge; model: FlowModel; outgoing: boolean; navigation?: FlowNavigation }) {
  const t = useTranslate();
  const other = model.nodes.get(outgoing ? edge.to : edge.from);
  return (
    <li>
      <p>
        {outgoing ? t("flow.inspector.to", { node: other?.label ?? "?" }) : t("flow.inspector.from", { node: other?.label ?? "?" })}
        {" · "}
        {edge.configured ? t("flow.inspector.configured") : t("flow.inspector.observedOnGroup")}
      </p>
      <ul>
        {[...edge.groups].map(([gaRaw, group]) => (
          <li key={gaRaw}>
            <p>{t("flow.inspector.groupCount", { ga: group.label, count: group.count, generation: group.generation })}</p>
            <ProjectLink target={{ kind: "group", id: gaRaw, generation: group.generation }} navigation={navigation}>{group.label}</ProjectLink>
            <ObjectTable objects={outgoing ? group.sourceObjects : group.targetObjects} />
          </li>
        ))}
      </ul>
    </li>
  );
}

function Inspector({ model, node, nowMs, navigation, onClose }: {
  model: FlowModel; node: FlowNode; nowMs: number; navigation?: FlowNavigation; onClose: () => void;
}) {
  const t = useTranslate();
  const values = allCurrentValues(model, node.id, nowMs);
  const outgoing = [...model.edges.values()].filter((edge) => edge.from === node.id);
  const incoming = [...model.edges.values()].filter((edge) => edge.to === node.id);
  return (
    <aside className="flow-inspector" aria-labelledby="flow-inspector-title">
      <div className="flow-inspector-heading"><h3 id="flow-inspector-title">{node.label}</h3>
        <button type="button" onClick={onClose}>{t("flow.closeDetails")}</button></div>
      <ProjectLink target={nodeFlowTarget(node)} navigation={navigation}>{t(node.kind === "group" ? "flow.openGroup" : "flow.openDevice")}</ProjectLink>
      <p>{kindText(t, node)}</p>
      {node.address && node.kind === "device" && <p>{t("flow.inspector.observedAddress", { address: node.address })}</p>}
      <h4>{t("flow.inspector.values")}</h4>
      {values.length === 0 ? (
        <p className="flow-inspector-note">{t("flow.inspector.noValues")}</p>
      ) : (
        <table className="flow-values">
          <thead>
            <tr>
              <th scope="col">{t("flow.inspector.groupAddress")}</th>
              <th scope="col">{t("flow.inspector.value")}</th>
              <th scope="col">{t("flow.inspector.origin")}</th>
              <th scope="col">{t("flow.inspector.sender")}</th>
              <th scope="col">{t("flow.inspector.sequence")}</th>
            </tr>
          </thead>
          <tbody>
            {values.map((slot) => (
              <tr key={slot.gaRaw}>
                <th scope="row"><ProjectLink target={{ kind: "group", id: slot.gaRaw, generation: slot.generation }} navigation={navigation}>{slot.gaLabel}</ProjectLink></th>
                <td>{slot.value}</td>
                <td>{t(ORIGIN_KEYS[slot.origin])}</td>
                <td>{slot.sourceLabel}</td>
                <td>{slot.seq}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <h4>{t("flow.inspector.connections")}</h4>
      {outgoing.length + incoming.length === 0 ? (
        <p className="flow-inspector-note">{t("flow.inspector.noConnections")}</p>
      ) : (
        <ul className="flow-connections">
          {outgoing.map((edge) => <Connection key={edge.id} edge={edge} model={model} outgoing navigation={navigation} />)}
          {incoming.map((edge) => <Connection key={edge.id} edge={edge} model={model} outgoing={false} navigation={navigation} />)}
        </ul>
      )}
    </aside>
  );
}

function SourceDiagnostics({ source, closed }: { source?: FlowSourceDiagnostics; closed?: boolean }) {
  const t = useTranslate();
  if (!source) return null;
  const lines: string[] = [];
  if (source.gaps > 0) lines.push(t("busMonitor.gapNotice", { count: source.gaps }));
  if (source.pruned > 0) lines.push(t("busMonitor.capturePruned", { count: source.pruned, capacity: source.capacity ?? "?" }));
  if (source.context === "stale") lines.push(t("busMonitor.contextStale"));
  if (source.context === "unverified") lines.push(t("busMonitor.contextUnverified"));
  if (source.ended && !closed) lines.push(t("flow.diag.closed"));
  if (source.error) lines.push(source.error);
  return lines.length ? <ul className="flow-source-diagnostics">{lines.map((line, i) => <li key={i}>{line}</li>)}</ul> : null;
}

function Diagnostics({ model }: { model: FlowModel }) {
  const t = useTranslate();
  const c = model.counters;
  const raw = [...model.nodes.values()].filter((node) => node.kind === "rawSource").length;
  const lines: string[] = [];
  if (c.refusedNodes + c.refusedEdges + c.refusedSlots > 0) {
    lines.push(t("flow.diag.refused", { nodes: c.refusedNodes, edges: c.refusedEdges, values: c.refusedSlots }));
  }
  if (model.pending.length > 0) lines.push(t("flow.diag.pending", { count: model.pending.length }));
  if (c.pendingOverflow > 0) lines.push(t("flow.diag.pendingOverflow", { count: c.pendingOverflow }));
  if (raw > 0) lines.push(t("flow.diag.raw", { count: raw }));
  if (c.legacy > 0) lines.push(t("flow.diag.legacy", { count: c.legacy }));
  if (c.malformed > 0) lines.push(t("flow.diag.malformed", { count: c.malformed }));
  if (c.unknownAge > 0) lines.push(t("flow.diag.unknownAge", { count: c.unknownAge }));
  const incomplete = [...model.contexts.values()].some((context) => context.kind === "participants" && !context.complete);
  if (incomplete) lines.push(t("flow.diag.truncated"));
  if (model.closed) lines.push(t("flow.diag.closed"));
  if (lines.length === 0) return null;
  return (
    <ul className="flow-diagnostics">
      {lines.map((line) => <li key={line}>{line}</li>)}
    </ul>
  );
}

function edgeLabel(edge: FlowEdge): string {
  const labels = [...edge.groups.values()].map((group) => group.label);
  const shown = labels.slice(0, EDGE_LABELS).join(", ");
  return labels.length > EDGE_LABELS ? `${shown} +${labels.length - EDGE_LABELS}` : shown;
}

const BOX = { width: VIEW_WIDTH, height: VIEW_HEIGHT };
/** A pulse lights its sender for the first part of its way. */
const SENDING_UNTIL = 0.35;

const browserScheduler: AnimatorScheduler = {
  frame: (callback) => requestAnimationFrame(callback),
  cancelFrame: (id) => cancelAnimationFrame(id),
  every: (ms, callback) => window.setInterval(callback, ms),
  cancelEvery: (id) => window.clearInterval(id),
  now: flowNow,
  hidden: () => document.visibilityState === "hidden",
};

interface EdgeElements {
  from: string;
  to: string;
  path: SVGPathElement;
  label: SVGTextElement;
}

function drawPulses(layer: SVGGElement | null, pulses: readonly DrawnPulse[], geometry: (from: string, to: string) => EdgeGeometry | undefined) {
  if (!layer) return;
  while (layer.childNodes.length > pulses.length) layer.lastChild!.remove();
  while (layer.childNodes.length < pulses.length) {
    const group = document.createElementNS("http://www.w3.org/2000/svg", "g");
    group.setAttribute("class", "flow-pulse");
    group.append(document.createElementNS("http://www.w3.org/2000/svg", "circle"), document.createElementNS("http://www.w3.org/2000/svg", "text"));
    layer.append(group);
  }
  pulses.forEach((pulse, index) => {
    const group = layer.childNodes[index] as SVGGElement;
    const curve = geometry(pulse.from, pulse.to);
    if (!curve) {
      group.setAttribute("visibility", "hidden");
      return;
    }
    const at = pointOnEdge(curve, pulse.progress);
    group.removeAttribute("visibility");
    group.setAttribute("transform", `translate(${at.x.toFixed(1)} ${at.y.toFixed(1)})`);
    const circle = group.firstChild as SVGCircleElement;
    circle.setAttribute("r", String(pulse.count > 1 ? 7 : 4.5));
    const text = group.lastChild as SVGTextElement;
    text.textContent = pulse.count > 1 ? `×${pulse.count}` : "";
    text.setAttribute("x", "9");
    text.setAttribute("y", "-6");
  });
}

export default function TelegramFlowView({ feed, navigation, onOpenWindow, active = true }: {
  feed: Pick<FlowFeed, "model" | "version" | "source">; navigation?: FlowNavigation; onOpenWindow?: () => void; active?: boolean;
}) {
  const t = useTranslate();
  const markerId = `flow-arrow-${useId().replace(/:/g, "")}`;
  const model = feed.model;
  const motion = useFlowMotion();
  const [frozen, setFrozen] = useState(false);
  const [maximized, setMaximized] = useState(false);
  const [autoZoom, setAutoZoom] = useState(true);
  const [box, setBox] = useState(BOX);
  const svgRef = useRef<SVGSVGElement | null>(null);
  const sectionRef = useRef<HTMLElement | null>(null);
  const [view, setView] = useState<View>(HOME);
  const [selected, setSelected] = useState<string | null>(null);
  const [focused, setFocused] = useState<string | null>(null);
  // Once a second (while visible) the animator asks for a refresh: fades,
  // the leader label and value expiry move with time, not only with data.
  const [, setTick] = useState(0);
  const nodeRefs = useRef(new Map<string, SVGGElement>());
  const edgeRefs = useRef(new Map<string, EdgeElements>());
  const geometries = useRef(new Map<string, EdgeGeometry>());
  const modelRef = useRef(model); modelRef.current = model;
  const pulseLayer = useRef<SVGGElement | null>(null);
  const sendingNodes = useRef(new Set<string>());
  const drag = useRef<{ x: number; y: number; view: View } | null>(null);
  const animatorRef = useRef<FlowAnimator | null>(null);
  const nowMs = flowNow();

  // Before the animator's first sync, positions come from the same seeding
  // (hex slots, clamped to the box), so nothing jumps when it takes over.
  const seeded = useMemo(
    () => readableLayout(model ? [...model.nodes.values()] : [], model ? [...model.edges.values()] : [], box),
    // `version` changes whenever the model changed in place.
    [model, feed.version, box],
  );
  const position = (id: string): Point | undefined => animatorRef.current?.layout.nodes.get(id) ?? seeded.get(id);

  function geometryFor(fromId: string, toId: string): EdgeGeometry | undefined {
    const key = `${fromId}→${toId}`;
    const cached = geometries.current.get(key); if (cached) return cached;
    const from = position(fromId); const to = position(toId); if (!from || !to) return;
    const current = modelRef.current;
    const obstacles = current && current.nodes.size <= DETAILED_ROUTING_NODES && current.edges.size <= DETAILED_ROUTING_EDGES ? [...current.nodes.values()].flatMap(n => {
      if (n.id === fromId || n.id === toId) return [];
      const p = position(n.id); return p ? [{ ...p, ...flowFootprint(n.label, 4) }] : [];
    }) : [];
    const geometry = routeFlowEdge(from, to, obstacles, fromId < toId ? 1 : 1.25);
    geometries.current.set(key, geometry); return geometry;
  }

  useEffect(() => {
    const sink: AnimatorSink = {
      // Only what moved is rewritten (§9.3): settled nodes and the edges
      // between them keep their attributes untouched.
      positions: (nodes: ReadonlyMap<string, DynamicNode>, moved: ReadonlySet<string>) => {
        if (moved.size) geometries.current.clear();
        for (const id of moved) {
          const element = nodeRefs.current.get(id);
          const node = nodes.get(id);
          if (element && node) element.setAttribute("transform", `translate(${node.x.toFixed(1)} ${node.y.toFixed(1)})`);
        }
        for (const edge of edgeRefs.current.values()) {
          if ((!modelRef.current || modelRef.current.nodes.size > DETAILED_ROUTING_NODES || modelRef.current.edges.size > DETAILED_ROUTING_EDGES) && !moved.has(edge.from) && !moved.has(edge.to)) continue;
          const from = nodes.get(edge.from);
          const to = nodes.get(edge.to);
          if (!from || !to) continue;
          const geometry = geometryFor(edge.from, edge.to)!;
          edge.path.setAttribute("d", geometry.path);
          edge.label.setAttribute("x", geometry.label.x.toFixed(1));
          edge.label.setAttribute("y", geometry.label.y.toFixed(1));
        }
      },
      pulses: (pulses) => {
        drawPulses(pulseLayer.current, pulses, geometryFor);
        // Only changes are written: an attribute write per node and frame
        // invalidated styles the measurement showed as native work.
        const sending = new Set(pulses.filter((pulse) => pulse.progress < SENDING_UNTIL).map((pulse) => pulse.from));
        for (const id of sendingNodes.current) {
          if (!sending.has(id)) nodeRefs.current.get(id)?.removeAttribute("data-sending");
        }
        for (const id of sending) {
          if (!sendingNodes.current.has(id)) nodeRefs.current.get(id)?.setAttribute("data-sending", "true");
        }
        sendingNodes.current = sending;
      },
      refresh: () => setTick((tick) => tick + 1),
    };
    const animator = new FlowAnimator(BOX, browserScheduler, sink);
    animatorRef.current = animator;
    return () => {
      animator.dispose();
      animatorRef.current = null;
    };
  }, []);

  useEffect(() => {
    animatorRef.current?.setVisible(active);
    animatorRef.current?.setMotion(motion && active);
    if (!motion) {
      for (const element of nodeRefs.current.values()) element.removeAttribute("data-sending");
      sendingNodes.current = new Set();
    }
  }, [motion, active]);

  useEffect(() => {
    animatorRef.current?.setFrozen(frozen);
  }, [frozen]);

  useEffect(() => {
    if (model) animatorRef.current?.sync(model);
  }, [model, feed.version]);

  const hasNodes = !!model?.nodes.size;
  const graphShape = model ? `${model.identity.serverIncarnation}:${model.identity.sessionId}:` +
    [...model.nodes.values()].map(n => `${n.id}:${n.label}`).join("|") + ";" + [...model.edges.keys()].join("|") : "";
  useEffect(() => {
    const svg = svgRef.current;
    if (!svg || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => {
      const { width, height } = svg.getBoundingClientRect();
      if (width > 0 && height > 0) setBox(old => old.width === width && old.height === height ? old : { width, height });
    });
    observer.observe(svg); return () => observer.disconnect();
  }, [hasNodes]);

  function fitAll() {
    if (!model) return;
    const at = animatorRef.current?.layout.targets;
    const positions = new Map([...model.nodes.values()].flatMap(n => {
      const p = frozen ? position(n.id) : at?.get(n.id) ?? position(n.id);
      return p ? [[n.id, { ...p, ...flowFootprint(n.label, 4) }] as const] : [];
    }));
    const bounds = [...positions.values()];
    for (const edge of model.edges.values()) {
      const from = positions.get(edge.from); const to = positions.get(edge.to);
      if (!from || !to) continue;
      const obstacles = model.nodes.size <= DETAILED_ROUTING_NODES && model.edges.size <= DETAILED_ROUTING_EDGES ? [...positions].filter(([id]) => id !== edge.from && id !== edge.to).map(([, p]) => p) : [];
      const curve = routeFlowEdge(from, to, obstacles, edge.from < edge.to ? 1 : 1.25);
      const extrema = (a: number, b: number, c: number) => Math.max(0, Math.min(1, (a - b) / (a - 2 * b + c) || 0));
      for (const t of [0, 1, extrema(curve.start.x, curve.control.x, curve.end.x), extrema(curve.start.y, curve.control.y, curve.end.y)]) {
        bounds.push({ ...pointOnEdge(curve, t), halfWidth: 8, top: 8, bottom: 8 });
      }
      bounds.push({ ...curve.label, halfWidth: Math.min(160, edgeLabel(edge).length * 3.5), top: 12, bottom: 5 });
    }
    setView(fitFlowCamera(bounds, box));
  }
  useEffect(() => {
    geometries.current.clear();
    if (model) animatorRef.current?.rearrange(box);
  }, [graphShape, box, frozen]);
  useEffect(() => {
    if (autoZoom) fitAll();
    // Traffic/value expiry does not re-fit; only graph, viewport and camera settings.
  }, [graphShape, box, frozen, autoZoom]);
  useEffect(() => { if (!active) setMaximized(false); }, [active]);
  useEffect(() => {
    const section = sectionRef.current;
    if (!maximized || !section) return;
    const previousFocus = document.activeElement;
    const outside = new Map<HTMLElement, boolean>();
    let branch: HTMLElement = section;
    while (branch.parentElement) {
      for (const sibling of branch.parentElement.children) if (sibling !== branch && sibling instanceof HTMLElement) {
        outside.set(sibling, sibling.inert); sibling.inert = true;
      }
      branch = branch.parentElement;
    }
    const key = (e: globalThis.KeyboardEvent) => {
      if (e.key === "Escape") { setMaximized(false); e.preventDefault(); e.stopPropagation(); }
      if (e.key === "Tab") {
        const controls = [...section.querySelectorAll<HTMLElement>("button:not(:disabled), [tabindex='0'], a[href], input:not(:disabled), select:not(:disabled)")].filter(el => el.getClientRects().length > 0);
        const first = controls[0]; const last = controls[controls.length - 1];
        if (first && (!section.contains(document.activeElement) || (e.shiftKey ? document.activeElement === first : document.activeElement === last))) {
          e.preventDefault(); (e.shiftKey ? last : first).focus();
        }
      }
    };
    document.addEventListener("keydown", key, true);
    return () => {
      document.removeEventListener("keydown", key, true);
      for (const [el, inert] of outside) el.inert = inert;
      if (previousFocus instanceof HTMLElement && document.contains(previousFocus)) previousFocus.focus();
    };
  }, [maximized]);

  const order = useMemo(
    () => (model ? [...model.nodes.values()].sort((a, b) => a.label.localeCompare(b.label) || a.id.localeCompare(b.id)).map((n) => n.id) : []),
    [model, feed.version],
  );

  if (!model || model.nodes.size === 0) {
    return (
      <section ref={sectionRef} role={maximized ? "dialog" : undefined} aria-modal={maximized || undefined} className={`telegram-flow${maximized ? " flow-maximized" : ""}`} aria-label={t("flow.title")}>
        <p className="flow-explain">{t("flow.explain")}</p>
        <p className="flow-empty">{t("flow.empty")}</p>
        <div className="flow-toolbar">
          <button type="button" aria-pressed={maximized} onClick={() => setMaximized(v => !v)}>{t(maximized ? "flow.restore" : "flow.maximize")}</button>
          {onOpenWindow && <button type="button" onClick={onOpenWindow}>{t("flow.openWindow")}</button>}
        </div>
        <SourceDiagnostics source={feed.source} closed={model?.closed} />
        {model && <Diagnostics model={model} />}
      </section>
    );
  }

  const tabStop = focused !== null && model.nodes.has(focused) ? focused : order[0];
  const selectedNode = selected !== null ? model.nodes.get(selected) ?? null : null;
  const leader = currentLeader(model, nowMs);
  const metrics = animatorRef.current?.metrics;

  function zoomBy(factor: number) {
    setAutoZoom(false);
    setView((current) => ({ ...current, zoom: Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, current.zoom * factor)) }));
  }

  function moveFocus(id: string) {
    setFocused(id);
    nodeRefs.current.get(id)?.focus();
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const arrows: Record<string, [number, number]> = { ArrowLeft: [1, 0], ArrowRight: [-1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] };
    if (event.shiftKey && event.key in arrows) {
      setAutoZoom(false);
      const [dx, dy] = arrows[event.key];
      setView((current) => ({ ...current, x: current.x + dx * PAN_STEP, y: current.y + dy * PAN_STEP }));
    } else if (event.key === "+" || event.key === "=") {
      zoomBy(ZOOM_STEP);
    } else if (event.key === "-") {
      zoomBy(1 / ZOOM_STEP);
    } else if (event.key === "0") {
      setAutoZoom(false);
      setView(HOME);
    } else if (event.key in arrows || event.key === "Home" || event.key === "End") {
      const index = order.indexOf(tabStop);
      const next =
        event.key === "Home" ? 0
          : event.key === "End" ? order.length - 1
            : event.key === "ArrowRight" || event.key === "ArrowDown" ? Math.min(order.length - 1, index + 1)
              : Math.max(0, index - 1);
      moveFocus(order[next]);
    } else if ((event.key === "Enter" || event.key === " ") && event.target instanceof SVGGElement) {
      setSelected(tabStop);
    } else {
      return;
    }
    event.preventDefault();
  }

  function onPointerDown(event: PointerEvent<SVGSVGElement>) {
    if (event.target !== event.currentTarget) return;
    setAutoZoom(false);
    drag.current = { x: event.clientX, y: event.clientY, view };
    event.currentTarget.setPointerCapture?.(event.pointerId);
  }

  function onPointerMove(event: PointerEvent<SVGSVGElement>) {
    const start = drag.current;
    if (!start) return;
    const scale = box.width / (event.currentTarget.clientWidth || box.width);
    setView({ ...start.view, x: start.view.x + (event.clientX - start.x) * scale, y: start.view.y + (event.clientY - start.y) * scale });
  }

  return (
    <section ref={sectionRef} role={maximized ? "dialog" : undefined} aria-modal={maximized || undefined} className={`telegram-flow${maximized ? " flow-maximized" : ""}`} aria-label={t("flow.title")}>
      <p className="flow-explain">{t("flow.explain")}</p>
      <div className="flow-toolbar" role="toolbar" aria-label={t("flow.toolbar")}>
        <button type="button" onClick={() => zoomBy(1 / ZOOM_STEP)}>{t("flow.zoomOut")}</button>
        <button type="button" onClick={() => zoomBy(ZOOM_STEP)}>{t("flow.zoomIn")}</button>
        <button type="button" onClick={() => { setAutoZoom(false); setView(HOME); }}>{t("flow.resetView")}</button>
        <button type="button" onClick={fitAll}>{t("flow.fitAll")}</button>
        <button type="button" aria-pressed={autoZoom} onClick={() => setAutoZoom(v => !v)}>{t("flow.autoZoom")}</button>
        <button type="button" disabled={frozen} onClick={() => { animatorRef.current?.rearrange(box); if (autoZoom) fitAll(); }}>{t("flow.rearrange")}</button>
        <button type="button" aria-pressed={maximized} onClick={() => setMaximized(v => !v)}>{t(maximized ? "flow.restore" : "flow.maximize")}</button>
        {onOpenWindow && <button type="button" onClick={onOpenWindow}>{t("flow.openWindow")}</button>}
        <button type="button" aria-pressed={frozen} onClick={() => setFrozen((value) => !value)}>
          {t("flow.freeze")}
        </button>
        <span className="flow-keys">{t("flow.keys")}</span>
      </div>
      {!motion && <p className="flow-motion-off">{t("flow.motionOff")}</p>}
      <p className="flow-summary">{t("flow.summary", { nodes: model.nodes.size, edges: model.edges.size })}</p>
      <p className="flow-leader">
        {leader ? t("flow.leader", { name: model.nodes.get(leader)?.label ?? leader }) : t("flow.noLeader")}
      </p>
      {metrics?.reduced && (
        <p className="flow-reduced">
          {t("flow.reduced", { bundled: metrics.coalescedEvents, dropped: metrics.overCapacityEvents })}
        </p>
      )}
      <Diagnostics model={model} />
      <SourceDiagnostics source={feed.source} closed={model.closed} />
      {(model.nodes.size > DETAILED_ROUTING_NODES || model.edges.size > DETAILED_ROUTING_EDGES) && <p className="flow-layout-note">{t("flow.routingLimited")}</p>}
      <div className={`flow-body${selectedNode ? " flow-body-with-inspector" : ""}`}>
        <div className="flow-canvas" onKeyDown={onKeyDown}>
          <svg
            ref={svgRef}
            className="flow-svg"
            viewBox={`${-box.width / 2} ${-box.height / 2} ${box.width} ${box.height}`}
            role="group"
            aria-label={t("flow.graph")}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={() => { drag.current = null; }}
            onPointerCancel={() => { drag.current = null; }}
          >
            <defs>
              <marker id={markerId} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
                <path d="M0,0 L10,5 L0,10 z" className="flow-arrow" />
              </marker>
            </defs>
            <g transform={`translate(${view.x} ${view.y}) scale(${view.zoom})`} data-zoom={view.zoom}>
              <g aria-hidden="true">
                {[...model.edges.values()].map((edge) => {
                  const from = position(edge.from);
                  const to = position(edge.to);
                  if (!from || !to) return null;
                  const geometry = geometryFor(edge.from, edge.to)!;
                  return (
                    <g
                      key={edge.id}
                      ref={(element) => {
                        if (element) {
                          edgeRefs.current.set(edge.id, {
                            from: edge.from, to: edge.to,
                            path: element.querySelector("path")!, label: element.querySelector("text")!,
                          });
                        } else edgeRefs.current.delete(edge.id);
                      }}
                      className={`flow-edge ${edge.configured ? "flow-edge-configured" : "flow-edge-group"}`}
                      style={{ opacity: edgeOpacity(edge.lastObservedAtMs, nowMs) }}
                      data-edge-id={edge.id}
                    >
                      <path d={geometry.path} markerEnd={`url(#${markerId})`} />
                      <text x={geometry.label.x} y={geometry.label.y} className="flow-edge-label">{edgeLabel(edge)}</text>
                    </g>
                  );
                })}
              </g>
              <g className="flow-pulses" aria-hidden="true" ref={pulseLayer} />
              {order.map((id) => {
                const node = model.nodes.get(id)!;
                const at = position(id)!;
                const badges = currentBadges(model, id, nowMs);
                return (
                  <g
                    key={id}
                    ref={(element) => {
                      if (element) nodeRefs.current.set(id, element);
                      else nodeRefs.current.delete(id);
                    }}
                    className={`flow-node flow-node-${node.kind}${node.ambiguous ? " flow-node-ambiguous" : ""}${selected === id ? " flow-node-selected" : ""}${leader === id ? " flow-node-leader" : ""}`}
                    transform={`translate(${at.x.toFixed(1)} ${at.y.toFixed(1)})`}
                    role="button"
                    tabIndex={id === tabStop ? 0 : -1}
                    aria-pressed={selected === id}
                    aria-label={`${node.label}. ${kindText(t, node)}`}
                    data-node-id={id}
                    onFocus={() => setFocused(id)}
                    onClick={() => { setSelected(id); moveFocus(id); }}
                  >
                    {node.kind === "group" ? (
                      <rect x={-NODE_RADIUS} y={-NODE_RADIUS * 0.7} width={NODE_RADIUS * 2} height={NODE_RADIUS * 1.4} rx="4" />
                    ) : (
                      <circle r={NODE_RADIUS} />
                    )}
                    <text className="flow-node-label" y={-NODE_RADIUS - TEXT_CLEARANCE} aria-hidden="true">{displayFlowLabel(node.label)}</text>
                    <g className="flow-badges" aria-hidden="true">
                      {badges.current.map((slot, index) => (
                        <text
                          key={slot.gaRaw}
                          className={`flow-badge${slot.origin === "configuredTarget" ? " flow-badge-inferred" : ""}`}
                          y={NODE_RADIUS + TEXT_CLEARANCE + 10 + index * BADGE_LINE}
                        >
                          {displayFlowLabel(`${slot.origin === "configuredTarget" ? "◇ " : ""}${slot.gaLabel} ${slot.value}`)}

                        </text>
                      ))}
                      {badges.overflow > 0 && (
                        <text className="flow-badge flow-badge-more" y={NODE_RADIUS + TEXT_CLEARANCE + 10 + badges.current.length * BADGE_LINE}>
                          {t("flow.moreValues", { count: badges.overflow })}
                        </text>
                      )}
                    </g>
                  </g>
                );
              })}
            </g>
          </svg>
          <p className="flow-legend">{t("flow.legend")}</p>
        </div>
        {selectedNode && <Inspector model={model} node={selectedNode} nowMs={nowMs} navigation={navigation} onClose={() => setSelected(null)} />}
      </div>
    </section>
  );
}
