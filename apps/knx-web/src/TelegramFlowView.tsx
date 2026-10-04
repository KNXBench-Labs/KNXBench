/** U20: read-only telegram-flow view: observed senders, configured targets, current values. */
// Renders the reducer state from `flowFeed` (docs/TELEGRAM_FLOW_VISUALIZATION.md
// §1, §2, §11). Nothing here polls, connects or writes. Values and edges are
// decoration for sighted users and hidden from assistive technology; every
// fact is reachable as text through keyboard selection and the Inspector,
// so a busy bus does not turn into a stream of announcements.
import { useId, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { flowNow, type FlowFeed } from "./flowFeed";
import { NODE_RADIUS, edgeGeometry, placeNodes } from "./flowLayout";
import {
  allCurrentValues,
  currentBadges,
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

export default function TelegramFlowView({ feed }: { feed: FlowFeed }) {
  const t = useTranslate();
  const markerId = `flow-arrow-${useId().replace(/:/g, "")}`;
  const model = feed.model;
  const [view, setView] = useState<View>(HOME);
  const [selected, setSelected] = useState<string | null>(null);
  const [focused, setFocused] = useState<string | null>(null);
  const nodeRefs = useRef(new Map<string, SVGGElement>());
  const drag = useRef<{ x: number; y: number; view: View } | null>(null);
  const nowMs = flowNow();

  const positions = useMemo(
    () => placeNodes(model ? [...model.nodes.keys()] : []),
    // `version` changes whenever the model changed in place.
    [model, feed.version],
  );
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
        <span className="flow-keys">{t("flow.keys")}</span>
      </div>
      <p className="flow-summary">{t("flow.summary", { nodes: model.nodes.size, edges: model.edges.size })}</p>
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
                  const from = positions.get(edge.from);
                  const to = positions.get(edge.to);
                  if (!from || !to) return null;
                  const geometry = edgeGeometry(from, to);
                  return (
                    <g key={edge.id} className={`flow-edge ${edge.configured ? "flow-edge-configured" : "flow-edge-group"}`}>
                      <path d={geometry.path} markerEnd={`url(#${markerId})`} />
                      <text x={geometry.label.x} y={geometry.label.y} className="flow-edge-label">{edgeLabel(edge)}</text>
                    </g>
                  );
                })}
              </g>
              {order.map((id) => {
                const node = model.nodes.get(id)!;
                const at = positions.get(id)!;
                const badges = currentBadges(model, id, nowMs);
                return (
                  <g
                    key={id}
                    ref={(element) => {
                      if (element) nodeRefs.current.set(id, element);
                      else nodeRefs.current.delete(id);
                    }}
                    className={`flow-node flow-node-${node.kind}${node.ambiguous ? " flow-node-ambiguous" : ""}${selected === id ? " flow-node-selected" : ""}`}
                    transform={`translate(${at.x} ${at.y})`}
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
