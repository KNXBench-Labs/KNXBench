/** U21: the animator stops frames and timers, freezes geometry only, bundles pulses. */
import { describe, expect, it } from "vitest";
import { COALESCE_ABOVE, FRAME_INTERVAL_MS, FlowAnimator, MAX_PULSES, MIN_DRAWN_MOVE, PULSE_MS, type AnimatorScheduler, type DrawnPulse } from "./flowAnimator";
import { admitRows, createFlowModel, DEFAULT_FLOW_LIMITS, provideContext, type FlowModel, type FlowRowInput } from "./flowModel";
import { snapshotJson } from "./flowTestFixtures";
import { parseFlowSnapshot } from "./flowWire";

class FakeScheduler implements AnimatorScheduler {
  time = 0;
  isHidden = false;
  frames = new Map<number, (now: number) => void>();
  timers = new Map<number, { ms: number; next: number; callback: () => void }>();
  private id = 0;
  frame(callback: (now: number) => void) { this.frames.set(++this.id, callback); return this.id; }
  cancelFrame(id: number) { this.frames.delete(id); }
  every(ms: number, callback: () => void) { this.timers.set(++this.id, { ms, next: this.time + ms, callback }); return this.id; }
  cancelEvery(id: number) { this.timers.delete(id); }
  now() { return this.time; }
  hidden() { return this.isHidden; }
  /** Runs one animation frame `ms` later, if one was requested. */
  tick(ms = 16) {
    this.time += ms;
    for (const timer of this.timers.values()) {
      while (timer.next <= this.time) { timer.next += timer.ms; timer.callback(); }
    }
    const pending = [...this.frames];
    this.frames.clear();
    for (const [, callback] of pending) callback(this.time);
  }
  run(ms: number, frame = 16) { for (let t = 0; t < ms; t += frame) this.tick(frame); }
}

// The identity the snapshot fixture states; a model of another session refuses it.
const SNAPSHOT_SESSION = { serverIncarnation: "inc-1", sessionId: 7 };

const member = (deviceId: number) => ({
  deviceId, comObjectId: deviceId * 100, direction: deviceId === 1 ? "Send" : "Receive", active: true,
  flags: { communication: true, read: null, write: true, transmit: null, update: null, readOnInit: null },
});

function model(targets = [2, 3]): FlowModel {
  const m = createFlowModel(SNAPSHOT_SESSION);
  provideContext(m, "1", parseFlowSnapshot(snapshotJson({
    devices: [1, 2, 3, 4].map((id) => ({ deviceId: id, installationId: 1, name: `D${id}`, individualAddressRaw: 0x1100 + id })),
    groups: [{ gaRaw: 0x0801, gaId: 1, installationId: 1, name: "G", dpt: null, members: [1, ...targets].map(member) }],
  })), 0);
  return m;
}

const row = (seq: number, overrides: Partial<FlowRowInput> = {}): FlowRowInput => ({
  seq, service: "GroupValueWrite", source: "1.1.1", destination: "1/0/1", decoded: { kind: "value", text: "1" },
  sourceRaw: 0x1101, destinationRaw: 0x0801, observedAgeMs: 0, flowGeneration: "1", ...overrides,
});

function setup(motion = true) {
  const scheduler = new FakeScheduler();
  const drawn: { positions: number; pulses: DrawnPulse[][]; refreshes: number } = { positions: 0, pulses: [], refreshes: 0 };
  const animator = new FlowAnimator({ width: 960, height: 520 }, scheduler, {
    positions: () => { drawn.positions += 1; },
    pulses: (pulses) => { drawn.pulses.push([...pulses]); },
    refresh: () => { drawn.refreshes += 1; },
  });
  animator.setMotion(motion);
  return { scheduler, drawn, animator };
}

const snapshotOf = (animator: FlowAnimator) => [...animator.layout.nodes.values()].map((n) => [n.id, n.x, n.y]);

describe("FlowAnimator", () => {
  it("settles the layout and then stops requesting frames", () => {
    const { scheduler, animator, drawn } = setup();
    const m = model();
    admitRows(m, [row(1)], 0);
    animator.sync(m);
    expect(scheduler.frames.size).toBe(1);
    scheduler.run(20_000);
    expect(drawn.positions).toBeGreaterThan(10);
    expect(animator.layout.alpha).toBe(0);
    expect(scheduler.frames.size).toBe(0);
  });

  it("with motion off requests no frame, no nudge timer and draws no pulse", () => {
    const { scheduler, animator, drawn } = setup(false);
    const m = model();
    admitRows(m, [row(1)], 0);
    animator.sync(m);
    const before = snapshotOf(animator);
    scheduler.run(2_000);
    expect(scheduler.frames.size).toBe(0);
    expect([...scheduler.timers.values()].filter((t) => t.ms !== 1_000)).toHaveLength(0);
    expect(snapshotOf(animator)).toEqual(before);
    expect(drawn.pulses.flat()).toHaveLength(0);
  });

  it("stops frames, nudges and pulses at once when motion is switched off mid-animation", () => {
    const { scheduler, animator, drawn } = setup();
    const m = model();
    admitRows(m, [row(1)], 0);
    animator.sync(m);
    scheduler.tick();
    expect(animator.activePulses().length).toBeGreaterThan(0);
    animator.setMotion(false);
    expect(scheduler.frames.size).toBe(0);
    expect([...scheduler.timers.values()].filter((t) => t.ms !== 1_000)).toHaveLength(0);
    expect(animator.activePulses()).toHaveLength(0);
    expect(drawn.pulses.at(-1)).toEqual([]);
    const before = snapshotOf(animator);
    scheduler.run(1_000);
    expect(snapshotOf(animator)).toEqual(before);
  });

  it("freezes geometry only: positions hold, pulses still run and finish", () => {
    const { scheduler, animator } = setup();
    const m = model();
    animator.setFrozen(true);
    admitRows(m, [row(1)], 0);
    animator.sync(m);
    const before = snapshotOf(animator);
    scheduler.tick();
    expect(animator.activePulses().length).toBe(2);
    scheduler.run(PULSE_MS + 100);
    expect(snapshotOf(animator)).toEqual(before);
    expect(animator.activePulses()).toHaveLength(0);
    expect(scheduler.frames.size).toBe(0);
  });

  it("pulses only for fresh events, one per configured target", () => {
    const { scheduler, animator } = setup();
    const m = model();
    scheduler.time = 10_000;
    admitRows(m, [row(1, { observedAgeMs: 5_000 }), row(2)], 10_000);
    animator.sync(m);
    expect(animator.activePulses().map((p) => [p.from, p.to, p.count])).toEqual([["d:1", "d:2", 1], ["d:1", "d:3", 1]]);
  });

  it("bundles a dense batch and says how many telegrams each pulse stands for", () => {
    const { animator } = setup();
    const m = model([2]);
    admitRows(m, Array.from({ length: COALESCE_ABOVE + 6 }, (_, i) => row(i + 1)), 0);
    animator.sync(m);
    expect(animator.activePulses()).toEqual([expect.objectContaining({ from: "d:1", to: "d:2", count: COALESCE_ABOVE + 6 })]);
    expect(animator.metrics).toMatchObject({ coalescedEvents: COALESCE_ABOVE + 6, reduced: true });
  });

  it("caps simultaneous pulses and counts the telegrams it could not show", () => {
    const { animator } = setup();
    const m = createFlowModel(SNAPSHOT_SESSION);
    provideContext(m, "1", parseFlowSnapshot(snapshotJson({ status: "historical", groupAddressStyle: null, devices: [], groups: [] })), 0);
    admitRows(m, Array.from({ length: MAX_PULSES + 40 }, (_, i) => row(i + 1, { destinationRaw: i, destination: `0/0/${i}` })), 0);
    animator.sync(m);
    expect(animator.activePulses()).toHaveLength(MAX_PULSES);
    expect(animator.metrics.overCapacityEvents).toBe(40);
  });

  // AR21 finding 4: one telegram to several recipients is one telegram in the
  // note, not one per line it travels on (TELEGRAM_FLOW_VISUALIZATION §15).
  function fanOut() {
    const m = createFlowModel(SNAPSHOT_SESSION);
    provideContext(m, "1", parseFlowSnapshot(snapshotJson({
      devices: [1, 2, 3].map((id) => ({ deviceId: id, installationId: 1, name: `D${id}`, individualAddressRaw: 0x1100 + id })),
      groups: [
        // One receiver: a single line.
        { gaRaw: 0x0800, gaId: 1, installationId: 1, name: "One", dpt: null, members: [1, 2].map(member) },
        // Two receivers each: two lines per telegram.
        ...Array.from({ length: 100 }, (_, i) => ({
          gaRaw: 0x0900 + i, gaId: 10 + i, installationId: 1, name: `Two ${i}`, dpt: null, members: [1, 2, 3].map(member),
        })),
      ],
    })), 0);
    const to = (seq: number, gaRaw: number) => row(seq, { destinationRaw: gaRaw, destination: `x/${gaRaw}` });
    return { m, to };
  }

  it("counts a bundled telegram to several recipients once, as drawn or as not completely drawn", () => {
    const { animator } = setup();
    const { m, to } = fanOut();
    // 1 + 99 × 2 = 199 lines for 100 telegrams; the 160th line is the first of
    // telegram 81, so telegrams 81–100 are not (completely) drawn. Two more on
    // the last group share its refused bundle and are missing too.
    admitRows(m, [
      to(1, 0x0800),
      ...Array.from({ length: 99 }, (_, i) => to(i + 2, 0x0900 + i)),
      to(101, 0x0900 + 98), to(102, 0x0900 + 98),
    ], 0);
    animator.sync(m);
    expect(animator.activePulses()).toHaveLength(MAX_PULSES);
    expect(animator.metrics.coalescedEvents).toBe(80);
    expect(animator.metrics.overCapacityEvents).toBe(22);
  });

  it("counts an unbundled telegram to several recipients once when it finds no free pulse", () => {
    const { animator } = setup();
    const { m, to } = fanOut();
    let seq = 1;
    // Four batches of 20 telegrams (no bundling) fill all 160 pulses …
    for (let batch = 0; batch < 4; batch += 1) {
      admitRows(m, Array.from({ length: 20 }, () => { seq += 1; return to(seq, 0x0900 + (seq % 100)); }), 0);
      animator.sync(m);
    }
    expect(animator.activePulses()).toHaveLength(MAX_PULSES);
    // … so the ten telegrams of the fifth batch find none: ten, not twenty lines.
    admitRows(m, Array.from({ length: 10 }, () => { seq += 1; return to(seq, 0x0900 + (seq % 100)); }), 0);
    animator.sync(m);
    expect(animator.metrics.overCapacityEvents).toBe(10);
    expect(animator.metrics.coalescedEvents).toBe(0);
  });

  // AR21 finding 5: at the model's node limit every target of a telegram can
  // be refused while its sender is kept; such an event carries `to: []` and
  // has no line to draw (TELEGRAM_FLOW_VISUALIZATION §17).
  function atNodeLimit() {
    const m = createFlowModel(SNAPSHOT_SESSION, { ...DEFAULT_FLOW_LIMITS, maxNodes: 1 });
    provideContext(m, "1", parseFlowSnapshot(snapshotJson({
      devices: [1, 2, 3].map((id) => ({ deviceId: id, installationId: 1, name: `D${id}`, individualAddressRaw: 0x1100 + id })),
      groups: [{ gaRaw: 0x0801, gaId: 1, installationId: 1, name: "G", dpt: null, members: [1, 2, 3].map(member) }],
    })), 0);
    return m;
  }

  it("counts a bundled telegram without any line as not drawn, never as drawn bundled", () => {
    const { animator } = setup();
    const m = atNodeLimit();
    admitRows(m, Array.from({ length: 30 }, (_, i) => row(i + 1)), 0);
    expect(m.counters.refusedNodes).toBeGreaterThan(0);
    animator.sync(m);
    expect(animator.activePulses()).toHaveLength(0);
    expect(animator.metrics.coalescedEvents).toBe(0);
    expect(animator.metrics.overCapacityEvents).toBe(30);
    expect(animator.metrics.reduced).toBe(true);
  });

  it("counts an unbundled telegram without any line as not drawn and marks the rendering reduced", () => {
    const { animator } = setup();
    const m = atNodeLimit();
    admitRows(m, [row(1), row(2)], 0);
    animator.sync(m);
    expect(animator.activePulses()).toHaveLength(0);
    expect(animator.metrics.overCapacityEvents).toBe(2);
    expect(animator.metrics.coalescedEvents).toBe(0);
    expect(animator.metrics.reduced).toBe(true);
  });

  // AR21 finding 6 (TELEGRAM_FLOW_VISUALIZATION §19): at small node limits
  // a telegram can reach only part of its recipients, or come from a sender
  // the model refused. Both are not (completely) drawn.
  function limited(maxNodes: number, limits: Partial<typeof DEFAULT_FLOW_LIMITS> = {}) {
    const m = createFlowModel(SNAPSHOT_SESSION, { ...DEFAULT_FLOW_LIMITS, maxNodes, ...limits });
    provideContext(m, "1", parseFlowSnapshot(snapshotJson({
      devices: [1, 2, 3].map((id) => ({ deviceId: id, installationId: 1, name: `D${id}`, individualAddressRaw: 0x1100 + id })),
      groups: [{ gaRaw: 0x0801, gaId: 1, installationId: 1, name: "G", dpt: null, members: [1, 2, 3].map(member) }],
    })), 0);
    return m;
  }
  const fromD3 = (seq: number) => row(seq, { source: "1.1.3", sourceRaw: 0x1103 });

  it("counts a telegram drawn to only part of its recipients as not completely drawn (case A)", () => {
    const { animator } = setup();
    const m = limited(2);
    admitRows(m, [row(1), row(2)], 0);
    animator.sync(m);
    expect(animator.activePulses().map((p) => [p.from, p.to])).toEqual([["d:1", "d:2"], ["d:1", "d:2"]]);
    expect(animator.metrics).toMatchObject({ overCapacityEvents: 2, coalescedEvents: 0, reduced: true });
  });

  it("counts telegrams from a refused sender as not drawn (case B)", () => {
    const { animator } = setup();
    const m = limited(2);
    admitRows(m, [row(1)], 0);
    animator.sync(m);
    const before = animator.metrics.overCapacityEvents;
    admitRows(m, [fromD3(2), fromD3(3)], 0);
    animator.sync(m);
    expect(m.nodes.has("d:3")).toBe(false);
    expect(animator.metrics.overCapacityEvents - before).toBe(2);
    expect(animator.metrics.reduced).toBe(true);
  });

  it("counts a bundled batch from a refused sender as not drawn, never as bundled (case C)", () => {
    const { animator } = setup();
    const m = limited(2);
    admitRows(m, [row(1)], 0);
    animator.sync(m);
    const before = { ...animator.metrics };
    admitRows(m, Array.from({ length: 30 }, (_, i) => fromD3(i + 2)), 0);
    animator.sync(m);
    expect(animator.metrics.overCapacityEvents - before.overCapacityEvents).toBe(30);
    expect(animator.metrics.coalescedEvents - before.coalescedEvents).toBe(0);
  });

  it("still counts a fully represented telegram as drawn", () => {
    const { animator } = setup();
    const m = model();
    admitRows(m, [row(1), row(2)], 0);
    animator.sync(m);
    expect(animator.metrics).toMatchObject({ overCapacityEvents: 0, coalescedEvents: 0, reduced: false });
  });

  // AR21 finding 7 (§20): the model's event ring (`maxEvents`) can overflow
  // within one poll; telegrams it pushed out before the animator saw them
  // were never drawn and must not vanish from the note.
  it("counts telegrams the event ring dropped before they were drawn", () => {
    const { animator } = setup();
    const m = limited(8, { maxEvents: 10 });
    admitRows(m, Array.from({ length: 25 }, (_, i) => row(i + 1)), 0);
    expect(m.counters.eventsDropped).toBe(15);
    animator.sync(m);
    expect(animator.metrics.overCapacityEvents).toBe(15);
    expect(animator.metrics.coalescedEvents).toBe(0);
    expect(animator.metrics.reduced).toBe(true);
  });

  it("does not count ring overflow of telegrams that were already drawn", () => {
    const { animator } = setup();
    const m = limited(8, { maxEvents: 10 });
    admitRows(m, Array.from({ length: 8 }, (_, i) => row(i + 1)), 0);
    animator.sync(m);
    admitRows(m, Array.from({ length: 8 }, (_, i) => row(i + 9)), 0);
    expect(m.counters.eventsDropped).toBe(6);
    animator.sync(m);
    expect(animator.metrics).toMatchObject({ overCapacityEvents: 0, reduced: false });
  });

  it("does not count ring overflow while the page is hidden, nor on return", () => {
    const { scheduler, animator } = setup();
    const m = limited(8, { maxEvents: 10 });
    scheduler.isHidden = true;
    admitRows(m, Array.from({ length: 25 }, (_, i) => row(i + 1)), 0);
    animator.sync(m);
    scheduler.isHidden = false;
    animator.sync(m);
    expect(animator.metrics).toMatchObject({ overCapacityEvents: 0, reduced: false });
  });

  // A session change gives the view a new model whose sequence numbers start
  // again; the animator must not keep waiting for the old session's high mark.
  it("pulses for a new session's telegrams even when their sequence numbers are lower", () => {
    const { animator } = setup();
    const first = model();
    admitRows(first, Array.from({ length: 5 }, (_, i) => row(i + 40)), 0);
    animator.sync(first);
    const second = model();
    admitRows(second, [row(1)], 0);
    animator.sync(second);
    expect(animator.activePulses().filter((p) => p.progress === 0)).toHaveLength(12);
  });

  it("does not pulse while the page is hidden, nor replay that time on return", () => {
    const { scheduler, animator } = setup();
    const m = model();
    scheduler.isHidden = true;
    admitRows(m, [row(1)], 0);
    animator.sync(m);
    expect(animator.activePulses()).toHaveLength(0);
    scheduler.isHidden = false;
    scheduler.time = 5_000;
    animator.sync(m);
    expect(animator.activePulses()).toHaveLength(0);
  });

  it("does not replay the event history when the view opens later", () => {
    const { scheduler, animator } = setup();
    const m = model();
    admitRows(m, [row(1)], 0);
    scheduler.time = 30_000;
    animator.sync(m);
    expect(animator.activePulses()).toHaveLength(0);
  });

  it("moves again when a node appears after the layout came to rest", () => {
    const { scheduler, animator } = setup(true);
    const m = model([2]);
    admitRows(m, [row(1)], 0);
    animator.sync(m);
    scheduler.run(20_000);
    expect(scheduler.frames.size).toBe(0);
    admitRows(m, [row(2, { source: "1.1.4", sourceRaw: 0x1104 })], scheduler.time);
    animator.sync(m);
    expect(animator.layout.alpha).toBeGreaterThan(0);
    expect(scheduler.frames.size).toBe(1);
  });

  it("a new pair elsewhere moves only itself and its neighbours; settled nodes are not rewritten", () => {
    const scheduler = new FakeScheduler();
    const moved: string[][] = [];
    const animator = new FlowAnimator({ width: 960, height: 520 }, scheduler, {
      positions: (_nodes, ids) => { moved.push([...ids]); },
      pulses: () => {},
      refresh: () => {},
    });
    animator.setMotion(true);
    const m = createFlowModel(SNAPSHOT_SESSION);
    provideContext(m, "1", parseFlowSnapshot(snapshotJson({
      devices: [1, 2, 3, 4, 5].map((id) => ({ deviceId: id, installationId: 1, name: `D${id}`, individualAddressRaw: 0x1100 + id })),
      groups: [
        { gaRaw: 0x0801, gaId: 1, installationId: 1, name: "G1", dpt: null, members: [1, 2, 3].map(member) },
        { gaRaw: 0x0802, gaId: 2, installationId: 1, name: "G2", dpt: null,
          members: [{ ...member(4), direction: "Send" }, member(5)] },
      ],
    })), 0);
    admitRows(m, [row(1), row(2), row(3)], 0);
    animator.sync(m);
    scheduler.run(20_000);
    expect(animator.layout.alpha).toBe(0);
    const settled = new Map([...animator.layout.nodes].map(([id, n]) => [id, [n.x, n.y]]));
    moved.length = 0;
    admitRows(m, [row(4, { source: "1.1.4", sourceRaw: 0x1104, destination: "1/0/2", destinationRaw: 0x0802 })], scheduler.time);
    animator.sync(m);
    expect(animator.currentLeader).toBe("d:1");
    scheduler.run(20_000);
    const everMoved = new Set(moved.flat());
    expect(everMoved.has("d:4")).toBe(true);
    for (const id of ["d:1", "d:2", "d:3"]) {
      expect(everMoved.has(id), id).toBe(false);
      const node = animator.layout.nodes.get(id)!;
      expect([node.x, node.y], id).toEqual(settled.get(id));
    }
  });

  it("redraws a node only after a visible move, so a cooling tail writes nothing", () => {
    const scheduler = new FakeScheduler();
    const reports: { id: string; x: number; y: number }[][] = [];
    const animator = new FlowAnimator({ width: 960, height: 520 }, scheduler, {
      positions: (nodes, ids) => { reports.push([...ids].map((id) => ({ id, x: nodes.get(id)!.x, y: nodes.get(id)!.y }))); },
      pulses: () => {},
      refresh: () => {},
    });
    animator.setMotion(true);
    const m = model();
    admitRows(m, [row(1)], 0);
    animator.sync(m);
    scheduler.run(20_000);
    expect(animator.layout.alpha).toBe(0);
    expect(animator.metrics.steps).toBeGreaterThan(reports.length);
    // Every node ends within the threshold of where it was last drawn.
    const last = new Map<string, { x: number; y: number }>();
    for (const report of reports) for (const entry of report) last.set(entry.id, entry);
    for (const [id, node] of animator.layout.nodes) {
      const drawn = last.get(id)!;
      expect(Math.abs(node.x - drawn.x)).toBeLessThan(MIN_DRAWN_MOVE);
      expect(Math.abs(node.y - drawn.y)).toBeLessThan(MIN_DRAWN_MOVE);
    }
  });

  it("a larger value block nudges its node; values that only come and go leave the map at rest", () => {
    const { scheduler, animator } = setup();
    const m = createFlowModel(SNAPSHOT_SESSION);
    provideContext(m, "1", parseFlowSnapshot(snapshotJson({
      devices: [1, 2, 3].map((id) => ({ deviceId: id, installationId: 1, name: `D${id}`, individualAddressRaw: 0x1100 + id })),
      groups: [0x0801, 0x0802].map((gaRaw, i) => ({
        gaRaw, gaId: i + 1, installationId: 1, name: `G${i + 1}`, dpt: null, members: [1, 2, 3].map(member),
      })),
    })), 0);
    const second = (seq: number) => row(seq, { destination: "1/0/2", destinationRaw: 0x0802 });
    // Six observations put every pair into the 6–9 class; later rows stay in it.
    admitRows(m, [1, 2, 3, 4, 5, 6].map((seq) => row(seq)), 0);
    animator.sync(m);
    scheduler.run(20_000);
    expect(animator.layout.alpha).toBe(0);
    // Two current values at once: the sender's badge block grows past its maximum.
    admitRows(m, [row(7), second(8)], scheduler.time);
    animator.sync(m);
    expect(animator.layout.nodes.get("d:1")!.heat).toBeGreaterThan(0);
    scheduler.run(20_000);
    expect(animator.layout.alpha).toBe(0);
    // Values expired; one comes back: no new maximum, nothing moves.
    admitRows(m, [row(9)], scheduler.time);
    animator.sync(m);
    expect(animator.layout.alpha).toBe(0);
  });

  it("adapts distances when a pair changes its activity class, but not on every flicker", () => {
    const { scheduler, animator } = setup();
    const m = model([2]);
    admitRows(m, [row(1), row(2), row(3)], 0);
    animator.sync(m);
    scheduler.run(12_000);
    expect(animator.layout.alpha).toBe(0);
    const settled = animator.metrics.steps;
    // 3 → 4 observations in the window: same class (3–5), the map stays at rest.
    admitRows(m, [row(4)], scheduler.time);
    animator.sync(m);
    scheduler.run(6_000);
    expect(animator.metrics.steps).toBe(settled);
    // 4 → 7: a new class (6–9), the distances adapt.
    admitRows(m, [row(5), row(6), row(7)], scheduler.time);
    animator.sync(m);
    scheduler.run(1_000);
    expect(animator.metrics.steps).toBeGreaterThan(settled);
  });

  it("comes to rest within the same wall time when frames are slow, so an overloaded view does not stay busy longer", () => {
    const { scheduler, animator } = setup();
    const m = model();
    admitRows(m, [row(1)], 0);
    animator.sync(m);
    // Five frames a second instead of thirty: the cooling follows the clock.
    scheduler.run(15_000, 200);
    expect(animator.layout.alpha).toBe(0);
    expect(scheduler.frames.size).toBe(0);
  });

  it("draws at most about 30 frames a second, because painting cost grows with every frame", () => {
    const { scheduler, animator, drawn } = setup();
    const m = model();
    admitRows(m, [row(1)], 0);
    animator.sync(m);
    scheduler.run(600);
    expect(FRAME_INTERVAL_MS).toBeGreaterThanOrEqual(30);
    expect(drawn.pulses.length).toBeLessThanOrEqual(Math.ceil(600 / FRAME_INTERVAL_MS) + 1);
    expect(drawn.pulses.length).toBeGreaterThan(10);
  });

  it("refreshes once a second while visible and cancels everything on dispose", () => {
    const { scheduler, animator, drawn } = setup();
    scheduler.run(3_000);
    expect(drawn.refreshes).toBe(3);
    scheduler.isHidden = true;
    scheduler.run(2_000);
    expect(drawn.refreshes).toBe(3);
    animator.sync(model());
    animator.dispose();
    expect(scheduler.frames.size).toBe(0);
    expect(scheduler.timers.size).toBe(0);
  });
});
