/** U21: the animator stops frames and timers, freezes geometry only, bundles pulses. */
import { describe, expect, it } from "vitest";
import { COALESCE_ABOVE, FRAME_INTERVAL_MS, FlowAnimator, MAX_PULSES, PULSE_MS, type AnimatorScheduler, type DrawnPulse } from "./flowAnimator";
import { admitRows, createFlowModel, provideContext, type FlowModel, type FlowRowInput } from "./flowModel";
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
