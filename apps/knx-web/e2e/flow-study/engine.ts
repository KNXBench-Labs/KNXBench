/** U19 study engine: one animation-frame loop drives admission, layout, SVG updates and pulses. */
// Measured design, not product code. One requestAnimationFrame loop does
// everything; React never re-renders per telegram or per pulse. Elements are
// created once per node/edge and then updated with setAttribute. Values are
// admitted as soon as an event is due, independently of any pulse.
import { admit, badges, coalescePulses, createFlowState, currentLeader, expire, type FlowState, type StudyEvent } from "./model";
import { createLayout, ensureNodes, reheat, step, type Layout, type LayoutEdge } from "./layout";
import { eventStream, syntheticInstallation } from "./synthetic";

const SVG = "http://www.w3.org/2000/svg";
export const PULSE_MS = 700;
/** Pulse elements drawn at once; more are bundled and counted, never queued. */
export const MAX_PULSES = 160;
/** Above this many events in one frame, pulses are bundled per pair and group. */
export const COALESCE_ABOVE = 24;
export const QUIET_AFTER_MS = 10_000;
export const FADE_MS = 60_000;
export const RESTING_OPACITY = 0.35;
/** A cooled layout gets a gentle nudge this often so activity distances follow traffic. */
export const NUDGE_EVERY_MS = 5_000;
const GROWTH_HEAT = 0.3;
const NUDGE_HEAT = 0.08;

export interface StudyOptions {
  devices: number;
  edges: number;
  rate: number;
  seed: number;
  /** Group-address labels on every edge (small maps) or only on selection. */
  edgeLabels: boolean;
  /** Where edges and pulses are drawn; nodes, labels and selection stay SVG. */
  renderer: "svg" | "canvas";
  width: number;
  height: number;
}

export interface StudyMetrics {
  frames: number;
  scriptMs: number[];
  intervalsMs: number[];
  lagMs: number[];
  admitted: number;
  pulsesDrawn: number;
  pulsesBundled: number;
  pulsesOverCapacity: number;
  backlogPulsesSkipped: number;
  layoutSteps: number;
  /** Per-frame split of the script time. */
  admitMs: number[];
  layoutMs: number[];
  drawMs: number[];
  edgeWrites: number;
}

interface NodeEl { g: SVGGElement; dot: SVGCircleElement; badge: SVGTextElement; lines: SVGTSpanElement[]; text: string }
interface EdgeEl { path: SVGPathElement; label: SVGTextElement | null; key: string }
interface ActivePulse { from: string; to: string; start: number; count: number }

export class FlowStudyEngine {
  readonly state: FlowState = createFlowState();
  readonly layout: Layout;
  readonly metrics: StudyMetrics = {
    frames: 0, scriptMs: [], intervalsMs: [], lagMs: [], admitted: 0, pulsesDrawn: 0, pulsesBundled: 0,
    pulsesOverCapacity: 0, backlogPulsesSkipped: 0, layoutSteps: 0, admitMs: [], layoutMs: [], drawMs: [], edgeWrites: 0,
  };
  frozen = false;
  selected: string | null = null;
  leader: string | null = null;
  private readonly nodeEls = new Map<string, NodeEl>();
  private readonly edgeEls = new Map<string, EdgeEl>();
  private layoutEdges: LayoutEdge[] = [];
  private readonly recent = new Map<string, number>();
  private pulses: ActivePulse[] = [];
  private readonly pulsePool: SVGCircleElement[] = [];
  private readonly stream;
  private raf = 0;
  private lastNow = 0;
  private lastDecay = 0;
  private running = false;
  /** Rounded position last written per node; edges are rewritten only when an end moved. */
  private readonly written = new Map<string, string>();
  /** Frames in a row in which no node moved more than a tenth of a pixel. */
  private stillFrames = 0;
  private lastRelax = 0;
  private knownElements = 0;
  private lastNudge = 0;

  private colours = { edge: "", pulse: "", arrow: "", read: 0 };

  constructor(private readonly svg: SVGSVGElement, private readonly options: StudyOptions,
    private readonly onTick: (engine: FlowStudyEngine) => void = () => {},
    private readonly canvas: HTMLCanvasElement | null = null) {
    const installation = syntheticInstallation(options.seed, options.devices, options.edges);
    this.layout = createLayout([], options, options.seed);
    this.stream = eventStream(installation, options.seed, options.rate, performance.now());
    const pulseLayer = this.layer("flow-pulses");
    for (let i = 0; i < MAX_PULSES; i += 1) {
      const dot = document.createElementNS(SVG, "circle");
      dot.setAttribute("r", "4");
      dot.setAttribute("class", "flow-pulse");
      dot.style.display = "none";
      pulseLayer.append(dot);
      this.pulsePool.push(dot);
    }
  }

  /** Motion Off (application) or reduced motion (OS) stops layout and pulses. */
  motionAllowed(): boolean {
    if (document.documentElement.getAttribute("data-motion-level") === "off") return false;
    return !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  }

  start(): void {
    if (this.running) return;
    this.running = true;
    this.lastNow = performance.now();
    const loop = (now: number) => {
      if (!this.running) return;
      this.frame(now);
      this.raf = requestAnimationFrame(loop);
    };
    this.raf = requestAnimationFrame(loop);
  }

  stop(): void {
    this.running = false;
    cancelAnimationFrame(this.raf);
    this.pulses = [];
    for (const dot of this.pulsePool) dot.style.display = "none";
  }

  select(id: string | null): void { this.selected = id; }

  private layer(name: string): SVGGElement {
    let g = this.svg.querySelector<SVGGElement>(`g.${name}`);
    if (!g) { g = document.createElementNS(SVG, "g"); g.setAttribute("class", name); this.svg.append(g); }
    return g;
  }

  private frame(now: number): void {
    const t0 = performance.now();
    const interval = now - this.lastNow;
    this.lastNow = now;
    this.metrics.frames += 1;
    if (this.metrics.frames > 1) this.metrics.intervalsMs.push(interval);
    // A frame gap longer than a pulse (hidden tab, stall) is not replayed:
    // the backlog is admitted for its values, but its pulses are retired.
    const resumed = interval > 1000;
    if (resumed) this.pulses = [];
    const tAdmit = performance.now();
    const events = this.stream.due(now);
    for (const event of events) admit(this.state, event);
    this.metrics.admitted += events.length;
    // Lag of the oldest event in this batch: how late the slowest one became visible.
    if (events.length > 0) this.metrics.lagMs.push(now - events[0].observedAtMs);
    this.syncElements();
    for (const event of events) this.bump(event);
    const motion = this.motionAllowed();
    if (motion && !resumed) this.queuePulses(events, now);
    else if (resumed) this.metrics.backlogPulsesSkipped += events.length;
    if (now - this.lastDecay >= 1000) {
      for (const [key, value] of this.recent) this.recent.set(key, value * 0.9);
      this.layoutEdges = [...this.edgeEls.values()].map(({ key }) => {
        const edge = this.state.edges.get(key)!;
        return { from: edge.from, to: edge.to, rate: this.recent.get(key) ?? 0 };
      });
      const leader = currentLeader(this.state, now);
      if (leader !== this.leader) reheat(this.layout, GROWTH_HEAT);
      this.leader = leader;
      this.lastDecay = now;
    }
    this.metrics.admitMs.push(performance.now() - tAdmit);
    const tLayout = performance.now();
    const grew = this.nodeEls.size + this.edgeEls.size !== this.knownElements;
    this.knownElements = this.nodeEls.size + this.edgeEls.size;
    if (grew) { this.stillFrames = 0; reheat(this.layout, GROWTH_HEAT); }
    if (now - this.lastNudge >= NUDGE_EVERY_MS) { reheat(this.layout, NUDGE_HEAT); this.lastNudge = now; this.stillFrames = 0; }
    // A settled layout is re-checked once a second instead of every frame.
    const settled = this.stillFrames > 30 && now - this.lastRelax < 1000;
    if (motion && !this.frozen && !settled) {
      step(this.layout, this.layoutEdges, { leader: this.leader });
      this.metrics.layoutSteps += 1;
      if (this.stillFrames > 30) this.lastRelax = now;
    }
    this.metrics.layoutMs.push(performance.now() - tLayout);
    expire(this.state, now);
    const tDraw = performance.now();
    this.draw(now, motion);
    this.metrics.drawMs.push(performance.now() - tDraw);
    this.metrics.scriptMs.push(performance.now() - t0);
    this.onTick(this);
  }

  private bump(event: StudyEvent): void {
    const ends = event.targets.length > 0 ? event.targets : [`GA:${event.ga}`];
    for (const end of ends) {
      const key = `${event.source}→${end}`;
      if (this.recent.has(key) || this.state.edges.has(key)) this.recent.set(key, (this.recent.get(key) ?? 0) + 1);
    }
  }

  private queuePulses(events: StudyEvent[], now: number): void {
    const bundles = events.length > COALESCE_ABOVE ? coalescePulses(events) : events.flatMap((event) =>
      (event.targets.length > 0 ? event.targets : [`GA:${event.ga}`]).map((to) => ({ from: event.source, to, count: 1 })));
    if (events.length > COALESCE_ABOVE) this.metrics.pulsesBundled += events.length;
    for (const bundle of bundles) {
      if (!this.state.edges.has(`${bundle.from}→${bundle.to}`)) continue;
      if (this.pulses.length >= MAX_PULSES) { this.metrics.pulsesOverCapacity += bundle.count; continue; }
      this.pulses.push({ from: bundle.from, to: bundle.to, start: now, count: bundle.count });
    }
  }

  private syncElements(): void {
    ensureNodes(this.layout, this.state.nodes, this.options.seed + this.state.nodes.size);
    const nodeLayer = this.layer("flow-nodes");
    for (const id of this.state.nodes) {
      if (this.nodeEls.has(id)) continue;
      const g = document.createElementNS(SVG, "g");
      g.setAttribute("class", id.startsWith("GA:") ? "flow-node flow-node-group" : "flow-node");
      g.dataset.node = id;
      const dot = document.createElementNS(SVG, "circle");
      dot.setAttribute("r", id.startsWith("GA:") ? "5" : "8");
      const name = document.createElementNS(SVG, "text");
      name.setAttribute("class", "flow-node-label");
      name.setAttribute("y", "-12");
      name.textContent = id.startsWith("GA:") ? id.slice(3) : id;
      const badge = document.createElementNS(SVG, "text");
      badge.setAttribute("class", "flow-badge");
      // One line per current slot, at most three plus an overflow line.
      const lines = Array.from({ length: 4 }, (_, index) => {
        const line = document.createElementNS(SVG, "tspan");
        line.setAttribute("x", "0");
        line.setAttribute("y", String(22 + index * 12));
        badge.append(line);
        return line;
      });
      g.append(dot, name, badge);
      nodeLayer.append(g);
      this.nodeEls.set(id, { g, dot, badge, lines, text: "" });
    }
    const edgeLayer = this.layer("flow-edges");
    for (const [key, edge] of this.state.edges) {
      if (this.edgeEls.has(key)) continue;
      if (this.options.renderer === "canvas") {
        // Canvas mode keeps an element-free record: geometry is drawn, not mounted.
        this.edgeEls.set(key, { path: null as unknown as SVGPathElement, label: null, key });
        continue;
      }
      const path = document.createElementNS(SVG, "path");
      path.setAttribute("class", "flow-edge");
      path.setAttribute("marker-end", "url(#flow-arrow)");
      edgeLayer.append(path);
      let label: SVGTextElement | null = null;
      if (this.options.edgeLabels) {
        label = document.createElementNS(SVG, "text");
        label.setAttribute("class", "flow-edge-label");
        label.textContent = [...edge.gas.keys()].join(", ");
        edgeLayer.append(label);
      }
      this.edgeEls.set(key, { path, label, key });
    }
  }

  /** Theme colours are read from the live tokens, re-read once a second so a
   * theme change reaches the canvas without a reload. */
  private resolveColours(now: number): void {
    if (now - this.colours.read < 1000 && this.colours.edge) return;
    const style = getComputedStyle(document.documentElement);
    this.colours = {
      edge: style.getPropertyValue("--knx-accent").trim() || "currentColor",
      pulse: style.getPropertyValue("--knx-accent").trim() || "currentColor",
      arrow: style.getPropertyValue("--knx-muted").trim() || "currentColor",
      read: now,
    };
  }

  private drawCanvas(now: number, motion: boolean): void {
    const canvas = this.canvas;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    this.resolveColours(now);
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.lineWidth = 1.4;
    ctx.strokeStyle = this.colours.edge;
    ctx.fillStyle = this.colours.arrow;
    // Edges are batched by rounded opacity so the context changes rarely.
    const buckets = new Map<number, ReturnType<FlowStudyEngine["curve"]>[]>();
    for (const [key] of this.edgeEls) {
      const edge = this.state.edges.get(key)!;
      const quiet = now - edge.lastAtMs - QUIET_AFTER_MS;
      const opacity = quiet <= 0 ? 1 : Math.max(RESTING_OPACITY, 1 - (1 - RESTING_OPACITY) * (quiet / FADE_MS));
      const bucket = Math.round(opacity * 20) / 20;
      const c = this.curve(edge.from, edge.to);
      if (!c) continue;
      const list = buckets.get(bucket);
      if (list) list.push(c); else buckets.set(bucket, [c]);
    }
    for (const [alpha, curves] of buckets) {
      ctx.globalAlpha = alpha;
      ctx.beginPath();
      for (const c of curves) { if (!c) continue; ctx.moveTo(c.a.x, c.a.y); ctx.quadraticCurveTo(c.cx, c.cy, c.b.x, c.b.y); }
      ctx.stroke();
      ctx.beginPath();
      for (const c of curves) {
        if (!c) continue;
        // Arrowhead along the curve's end tangent, set back from the node.
        const dx = c.b.x - c.cx; const dy = c.b.y - c.cy; const len = Math.hypot(dx, dy) || 1;
        const ux = dx / len; const uy = dy / len;
        const tipX = c.b.x - ux * 10; const tipY = c.b.y - uy * 10;
        ctx.moveTo(tipX, tipY);
        ctx.lineTo(tipX - ux * 6 - uy * 3, tipY - uy * 6 + ux * 3);
        ctx.lineTo(tipX - ux * 6 + uy * 3, tipY - uy * 6 - ux * 3);
        ctx.closePath();
      }
      ctx.fill();
    }
    ctx.globalAlpha = 1;
    this.pulses = motion ? this.pulses.filter((pulse) => now - pulse.start < PULSE_MS) : [];
    ctx.fillStyle = this.colours.pulse;
    ctx.beginPath();
    for (const pulse of this.pulses) {
      const c = this.curve(pulse.from, pulse.to);
      if (!c) continue;
      const t = (now - pulse.start) / PULSE_MS; const u = 1 - t;
      const x = u * u * c.a.x + 2 * u * t * c.cx + t * t * c.b.x;
      const y = u * u * c.a.y + 2 * u * t * c.cy + t * t * c.b.y;
      const r = pulse.count > 1 ? 6 : 4;
      ctx.moveTo(x + r, y);
      ctx.arc(x, y, r, 0, Math.PI * 2);
      this.metrics.pulsesDrawn += 1;
    }
    ctx.fill();
  }

  /** Quadratic curve bent to the left of travel: reverse traffic bends away. */
  private curve(from: string, to: string) {
    const a = this.layout.nodes.get(from);
    const b = this.layout.nodes.get(to);
    if (!a || !b) return null;
    const mx = (a.x + b.x) / 2; const my = (a.y + b.y) / 2;
    const dx = b.x - a.x; const dy = b.y - a.y;
    return { a, b, cx: mx - dy * 0.18, cy: my + dx * 0.18 };
  }

  private draw(now: number, motion: boolean): void {
    const moved = new Set<string>();
    let maxShift = 0;
    for (const [id, el] of this.nodeEls) {
      const node = this.layout.nodes.get(id);
      if (node) {
        const at = `${node.x.toFixed(1)},${node.y.toFixed(1)}`;
        const before = this.written.get(id);
        if (before !== at) {
          if (before) {
            const [bx, by] = before.split(",").map(Number);
            maxShift = Math.max(maxShift, Math.abs(bx - node.x), Math.abs(by - node.y));
          }
          el.g.setAttribute("transform", `translate(${at})`);
          this.written.set(id, at);
          moved.add(id);
        }
      }
      el.g.classList.toggle("is-leader", id === this.leader);
      el.g.classList.toggle("is-selected", id === this.selected);
      const visible = this.options.edgeLabels || id === this.selected || id === this.leader;
      const shown = visible ? badges(this.state, id, now) : null;
      const rows = shown ? shown.current.map((slot) => `${slot.ga} ${slot.value}${slot.inferred ? " *" : ""}`) : [];
      if (shown && shown.overflow) rows.push(`+${shown.overflow} more`);
      const text = rows.join("\n");
      if (text !== el.text) {
        el.text = text;
        el.lines.forEach((line, index) => { line.textContent = rows[index] ?? ""; });
      }
    }
    this.stillFrames = maxShift < 0.1 ? this.stillFrames + 1 : 0;
    if (this.options.renderer === "canvas") { this.drawCanvas(now, motion); return; }
    for (const [key, el] of this.edgeEls) {
      const edge = this.state.edges.get(key)!;
      const quiet = now - edge.lastAtMs - QUIET_AFTER_MS;
      const opacity = quiet <= 0 ? 1 : Math.max(RESTING_OPACITY, 1 - (1 - RESTING_OPACITY) * (quiet / FADE_MS));
      const shade = opacity.toFixed(2);
      if (el.path.style.opacity !== shade) el.path.style.opacity = shade;
      if (el.label) {
        // Address labels sit on active or selected edges; quiet ones keep
        // theirs in the Inspector instead of piling up on the map.
        const show = quiet <= 0 || edge.from === this.selected || edge.to === this.selected;
        el.label.style.display = show ? "" : "none";
      }
      if (!moved.has(edge.from) && !moved.has(edge.to) && el.path.hasAttribute("d")) continue;
      const c = this.curve(edge.from, edge.to);
      if (!c) continue;
      el.path.setAttribute("d", `M${c.a.x.toFixed(1)},${c.a.y.toFixed(1)}Q${c.cx.toFixed(1)},${c.cy.toFixed(1)} ${c.b.x.toFixed(1)},${c.b.y.toFixed(1)}`);
      this.metrics.edgeWrites += 1;
      if (el.label) el.label.setAttribute("transform", `translate(${(0.25 * c.a.x + 0.5 * c.cx + 0.25 * c.b.x).toFixed(1)},${(0.25 * c.a.y + 0.5 * c.cy + 0.25 * c.b.y).toFixed(1)})`);
    }
    this.pulses = motion ? this.pulses.filter((pulse) => now - pulse.start < PULSE_MS) : [];
    this.pulsePool.forEach((dot, index) => {
      const pulse = this.pulses[index];
      const c = pulse && this.curve(pulse.from, pulse.to);
      if (!pulse || !c) { dot.style.display = "none"; return; }
      const t = (now - pulse.start) / PULSE_MS;
      const u = 1 - t;
      dot.style.display = "";
      dot.setAttribute("cx", (u * u * c.a.x + 2 * u * t * c.cx + t * t * c.b.x).toFixed(1));
      dot.setAttribute("cy", (u * u * c.a.y + 2 * u * t * c.cy + t * t * c.b.y).toFixed(1));
      dot.setAttribute("r", pulse.count > 1 ? "6" : "4");
      this.metrics.pulsesDrawn += 1;
    });
  }
}
