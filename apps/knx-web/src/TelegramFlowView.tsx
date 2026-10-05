/** U20: read-only telegram-flow view: observed senders, configured targets, current values. */
// Renders the reducer state from `flowFeed` (docs/TELEGRAM_FLOW_VISUALIZATION.md
// §1, §2, §11). Nothing here polls, connects or writes. Values and edges are
// decoration for sighted users and hidden from assistive technology; every
// fact is reachable as text through keyboard selection and the Inspector,
// so a busy bus does not turn into a stream of announcements.
import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { FlowAnimator, type AnimatorScheduler, type AnimatorSink, type DrawnPulse } from "./flowAnimator";
import { createDynamics, type DynamicNode } from "./flowDynamics";
import { useFlowMotion } from "./flowMotion";
import { flowNow, type FlowFeed } from "./flowFeed";
import { NODE_RADIUS, edgeGeometry, edgeOpacity, pointOnEdge, type Point } from "./flowLayout";
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
const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;
const PAN_STEP = 60;
const EDGE_LABELS = 2;
// Distance from the circle to the node name (above) and to the first value
// badge (below): an arrowhead arriving straight from above or below ends in
// this gap instead of under the text's halo.
const TEXT_CLEARANCE = 16;
const BADGE_LINE = 14;

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

function Connection({ edge, model, outgoing }: { edge: FlowEdge; model: FlowModel; outgoing: boolean }) {
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
            <ObjectTable objects={outgoing ? group.sourceObjects : group.targetObjects} />
          </li>
        ))}
      </ul>
    </li>
  );
}

function Inspector({ model, node, nowMs }: { model: FlowModel; node: FlowNode; nowMs: number }) {
  const t = useTranslate();
  const values = allCurrentValues(model, node.id, nowMs);
  const outgoing = [...model.edges.values()].filter((edge) => edge.from === node.id);
  const incoming = [...model.edges.values()].filter((edge) => edge.to === node.id);
  return (
    <aside className="flow-inspector" aria-labelledby="flow-inspector-title">
      <h3 id="flow-inspector-title">{node.label}</h3>
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
                <th scope="row">{slot.gaLabel}</th>
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
          {outgoing.map((edge) => <Connection key={edge.id} edge={edge} model={model} outgoing />)}
          {incoming.map((edge) => <Connection key={edge.id} edge={edge} model={model} outgoing={false} />)}
        </ul>
      )}
    </aside>
  );
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

function drawPulses(layer: SVGGElement | null, pulses: readonly DrawnPulse[], position: (id: string) => Point | undefined) {
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
    const from = position(pulse.from);
    const to = position(pulse.to);
    if (!from || !to) {
      group.setAttribute("visibility", "hidden");
      return;
    }
    const at = pointOnEdge(edgeGeometry(from, to), pulse.progress);
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

export default function TelegramFlowView({ feed }: { feed: FlowFeed }) {
  const t = useTranslate();
  const markerId = `flow-arrow-${useId().replace(/:/g, "")}`;
  const model = feed.model;
  const motion = useFlowMotion();
  const [frozen, setFrozen] = useState(false);
  const [view, setView] = useState<View>(HOME);
  const [selected, setSelected] = useState<string | null>(null);
  const [focused, setFocused] = useState<string | null>(null);
  // Once a second (while visible) the animator asks for a refresh: fades,
  // the leader label and value expiry move with time, not only with data.
  const [, setTick] = useState(0);
  const nodeRefs = useRef(new Map<string, SVGGElement>());
  const edgeRefs = useRef(new Map<string, EdgeElements>());
  const pulseLayer = useRef<SVGGElement | null>(null);
  const sendingNodes = useRef(new Set<string>());
  const drag = useRef<{ x: number; y: number; view: View } | null>(null);
  const animatorRef = useRef<FlowAnimator | null>(null);
  const nowMs = flowNow();

  // Before the animator's first sync, positions come from the same seeding
  // (hex slots, clamped to the box), so nothing jumps when it takes over.
  const seeded = useMemo(
    () => createDynamics(model ? [...model.nodes.keys()] : [], BOX).nodes,
    // `version` changes whenever the model changed in place.
    [model, feed.version],
  );
  const position = (id: string): Point | undefined => animatorRef.current?.layout.nodes.get(id) ?? seeded.get(id);

  useEffect(() => {
    const sink: AnimatorSink = {
      positions: (nodes: ReadonlyMap<string, DynamicNode>) => {
        for (const [id, element] of nodeRefs.current) {
          const node = nodes.get(id);
          if (node) element.setAttribute("transform", `translate(${node.x.toFixed(1)} ${node.y.toFixed(1)})`);
        }
        for (const edge of edgeRefs.current.values()) {
          const from = nodes.get(edge.from);
          const to = nodes.get(edge.to);
          if (!from || !to) continue;
          const geometry = edgeGeometry(from, to);
          edge.path.setAttribute("d", geometry.path);
          edge.label.setAttribute("x", geometry.label.x.toFixed(1));
          edge.label.setAttribute("y", geometry.label.y.toFixed(1));
        }
      },
      pulses: (pulses) => {
        const animator = animatorRef.current;
        drawPulses(pulseLayer.current, pulses, (id) => animator?.layout.nodes.get(id));
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
    animatorRef.current?.setMotion(motion);
    if (!motion) {
      for (const element of nodeRefs.current.values()) element.removeAttribute("data-sending");
      sendingNodes.current = new Set();
    }
  }, [motion]);

  useEffect(() => {
    animatorRef.current?.setFrozen(frozen);
  }, [frozen]);

  useEffect(() => {
    if (model) animatorRef.current?.sync(model);
  }, [model, feed.version]);

  const order = useMemo(
    () => (model ? [...model.nodes.values()].sort((a, b) => a.label.localeCompare(b.label) || a.id.localeCompare(b.id)).map((n) => n.id) : []),
    [model, feed.version],
  );

  if (!model || model.nodes.size === 0) {
    return (
      <section className="telegram-flow" aria-label={t("flow.title")}>
        <p className="flow-explain">{t("flow.explain")}</p>
        <p className="flow-empty">{t("flow.empty")}</p>
        {model && <Diagnostics model={model} />}
      </section>
    );
  }

  const tabStop = focused !== null && model.nodes.has(focused) ? focused : order[0];
  const selectedNode = selected !== null ? model.nodes.get(selected) ?? null : null;
  const leader = currentLeader(model, nowMs);
  const metrics = animatorRef.current?.metrics;

  function zoomBy(factor: number) {
    setView((current) => ({ ...current, zoom: Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, current.zoom * factor)) }));
  }

  function moveFocus(id: string) {
    setFocused(id);
    nodeRefs.current.get(id)?.focus();
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const arrows: Record<string, [number, number]> = { ArrowLeft: [1, 0], ArrowRight: [-1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] };
    if (event.shiftKey && event.key in arrows) {
      const [dx, dy] = arrows[event.key];
      setView((current) => ({ ...current, x: current.x + dx * PAN_STEP, y: current.y + dy * PAN_STEP }));
    } else if (event.key === "+" || event.key === "=") {
      zoomBy(ZOOM_STEP);
    } else if (event.key === "-") {
      zoomBy(1 / ZOOM_STEP);
    } else if (event.key === "0") {
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
    drag.current = { x: event.clientX, y: event.clientY, view };
    event.currentTarget.setPointerCapture?.(event.pointerId);
  }

  function onPointerMove(event: PointerEvent<SVGSVGElement>) {
    const start = drag.current;
    if (!start) return;
    const scale = VIEW_WIDTH / (event.currentTarget.clientWidth || VIEW_WIDTH);
    setView({ ...start.view, x: start.view.x + (event.clientX - start.x) * scale, y: start.view.y + (event.clientY - start.y) * scale });
  }

  return (
    <section className="telegram-flow" aria-label={t("flow.title")}>
      <p className="flow-explain">{t("flow.explain")}</p>
      <div className="flow-toolbar" role="toolbar" aria-label={t("flow.toolbar")}>
        <button type="button" onClick={() => zoomBy(1 / ZOOM_STEP)}>{t("flow.zoomOut")}</button>
        <button type="button" onClick={() => zoomBy(ZOOM_STEP)}>{t("flow.zoomIn")}</button>
        <button type="button" onClick={() => setView(HOME)}>{t("flow.resetView")}</button>
        <button type="button" aria-pressed={frozen} disabled={!motion} onClick={() => setFrozen((value) => !value)}>
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
      <div className="flow-body">
        <div className="flow-canvas" onKeyDown={onKeyDown}>
          <svg
            className="flow-svg"
            viewBox={`${-VIEW_WIDTH / 2} ${-VIEW_HEIGHT / 2} ${VIEW_WIDTH} ${VIEW_HEIGHT}`}
            role="group"
            aria-label={t("flow.graph")}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={() => { drag.current = null; }}
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
                  const geometry = edgeGeometry(from, to);
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
                    <text className="flow-node-label" y={-NODE_RADIUS - TEXT_CLEARANCE} aria-hidden="true">{node.label}</text>
                    <g className="flow-badges" aria-hidden="true">
                      {badges.current.map((slot, index) => (
                        <text
                          key={slot.gaRaw}
                          className={`flow-badge${slot.origin === "configuredTarget" ? " flow-badge-inferred" : ""}`}
                          y={NODE_RADIUS + TEXT_CLEARANCE + 10 + index * BADGE_LINE}
                        >
                          {`${slot.origin === "configuredTarget" ? "◇ " : ""}${slot.gaLabel} ${slot.value}`}
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
        {selectedNode ? (
          <Inspector model={model} node={selectedNode} nowMs={nowMs} />
        ) : (
          <aside className="flow-inspector flow-inspector-empty"><p>{t("flow.inspector.choose")}</p></aside>
        )}
      </div>
    </section>
  );
}
