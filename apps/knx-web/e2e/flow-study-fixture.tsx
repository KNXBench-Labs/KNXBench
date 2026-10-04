/** U19 study fixture: synthetic telegram flow in native SVG with a keyboard and static fallback. */
import { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { FlowStudyEngine, type StudyOptions } from "./flow-study/engine";
import { badges, currentLeader } from "./flow-study/model";
import "../src/styles.css";

const params = new URLSearchParams(location.search);
const num = (key: string, fallback: number) => Number(params.get(key) ?? fallback);
const options: StudyOptions = {
  devices: num("devices", 14), edges: num("edges", 22), rate: num("rate", 4), seed: num("seed", 1),
  edgeLabels: params.get("labels") !== "off", width: num("width", 900), height: num("height", 620),
  renderer: params.get("renderer") === "canvas" ? "canvas" : "svg",
};
if (params.get("theme")) document.documentElement.setAttribute("data-theme", params.get("theme")!);
if (params.get("motion")) document.documentElement.setAttribute("data-motion-level", params.get("motion")!);

const css = `
.flow-study { display: grid; grid-template-columns: minmax(0, 1fr) 300px; gap: 12px; padding: 12px; color: var(--knx-foreground); background: var(--knx-bg); min-height: 100vh; box-sizing: border-box; }
.flow-stage { position: relative; background: var(--knx-surface); border: 1px solid var(--knx-border); border-radius: var(--knx-radius-card); width: max-content; }
.flow-stage canvas { position: absolute; inset: 0; pointer-events: none; }
.flow-stage svg { position: relative; display: block; }
.flow-edge { fill: none; stroke: var(--knx-accent); stroke-width: 1.4; }
.flow-edge-label, .flow-node-label, .flow-badge { paint-order: stroke; stroke: var(--knx-surface); stroke-width: 3px; stroke-linejoin: round; }
.flow-edge-label { fill: var(--knx-muted); font: 10px var(--knx-font-mono); text-anchor: middle; }
.flow-node circle { fill: var(--knx-surface); stroke: var(--knx-foreground); stroke-width: 1.5; }
.flow-node-group circle { stroke: var(--knx-warning-color); stroke-dasharray: 3 2; }
.flow-node.is-leader circle { stroke: var(--knx-accent); stroke-width: 3.5; }
.flow-node.is-selected circle { fill: var(--knx-accent); }
.flow-node-label { fill: var(--knx-foreground); font: 11px var(--knx-font-mono); text-anchor: middle; }
.flow-badge { fill: var(--knx-success-color); font: 10px var(--knx-font-mono); text-anchor: middle; }
.flow-pulse { fill: var(--knx-accent); }
#flow-arrow path { fill: var(--knx-muted); }
.flow-synthetic { border: 1px dashed var(--knx-warning-color); padding: 4px 8px; display: inline-block; }
.flow-list { max-height: 260px; overflow: auto; padding: 0; list-style: none; }
.flow-list button[aria-pressed="true"] { outline: 2px solid var(--knx-accent); }
`;

function Study() {
  const svgRef = useRef<SVGSVGElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const engineRef = useRef<FlowStudyEngine | null>(null);
  const [, setTick] = useState(0);
  const [frozen, setFrozen] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  useEffect(() => {
    let last = 0;
    const engine = new FlowStudyEngine(svgRef.current!, options, () => {
      // The panel refreshes at most four times a second, never per telegram.
      const now = performance.now();
      if (now - last > 250) { last = now; setTick((n) => n + 1); }
    }, options.renderer === "canvas" ? canvasRef.current : null);
    engineRef.current = engine;
    // `?freeze=after` freezes the layout once it has spread, for measurements
    // of traffic, values and pulses over resting geometry.
    if (params.get("freeze") === "after") setTimeout(() => { engine.frozen = true; setFrozen(true); }, 2500);
    (window as unknown as { __flowStudy: FlowStudyEngine }).__flowStudy = engine;
    engine.start();
    return () => engine.stop();
  }, []);
  const engine = engineRef.current;
  const now = performance.now();
  const nodes = engine ? [...engine.state.nodes].sort() : [];
  const inspected = engine && selected ? badges(engine.state, selected, now) : null;
  const outgoing = engine && selected ? [...engine.state.edges.values()].filter((edge) => edge.from === selected) : [];
  return (
    <main className="flow-study">
      <section aria-label="Telegram flow study">
        <p className="flow-synthetic"><strong>Synthetic study</strong> — generated telegrams, not live bus data.</p>
        <div className="flow-stage">
        {options.renderer === "canvas" && <canvas ref={canvasRef} width={options.width} height={options.height} aria-hidden="true" />}
        <svg ref={svgRef} width={options.width} height={options.height} role="img"
          aria-label="Synthetic telegram flow map; use the device list for keyboard access">
          <defs>
            <marker id="flow-arrow" viewBox="0 0 10 10" refX="16" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
              <path d="M0,0 L10,5 L0,10 z" />
            </marker>
          </defs>
        </svg>
        </div>
      </section>
      <aside aria-label="Flow inspector">
        <button type="button" aria-pressed={frozen} onClick={() => {
          const next = !frozen; setFrozen(next); if (engine) engine.frozen = next;
        }}>Freeze layout</button>
        <p>Activity leader (60 s window): <strong data-testid="leader">{engine ? currentLeader(engine.state, now) ?? "none" : "—"}</strong></p>
        <p data-testid="counters">Admitted {engine?.metrics.admitted ?? 0} · refused nodes {engine?.state.overflow.refusedNodes ?? 0} · bundled {engine?.metrics.pulsesBundled ?? 0}</p>
        <h2>Devices</h2>
        <ul className="flow-list" aria-label="Devices and group nodes">
          {nodes.slice(0, 200).map((id) => (
            <li key={id}><button type="button" aria-pressed={selected === id} onClick={() => {
              setSelected(id); engine?.select(id);
            }}>{id}</button></li>
          ))}
          {nodes.length > 200 && <li>+{nodes.length - 200} more (study list is capped)</li>}
        </ul>
        {selected && (
          <section aria-label="Selected device" data-testid="inspector">
            <h2>{selected}</h2>
            <h3>Current values</h3>
            <ul>{inspected?.current.map((slot) => (
              <li key={slot.ga}>{slot.ga}: {slot.value} {slot.inferred ? "(configured target, not confirmed)" : "(observed sender)"}</li>
            ))}</ul>
            {inspected && inspected.overflow > 0 && <p>{inspected.overflow} more current values</p>}
            <h3>Sends to</h3>
            <ul>{outgoing.map((edge) => <li key={edge.to}>{edge.to} via {[...edge.gas.keys()].join(", ")} ({edge.count})</li>)}</ul>
          </section>
        )}
      </aside>
      <style>{css}</style>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<Study />);
