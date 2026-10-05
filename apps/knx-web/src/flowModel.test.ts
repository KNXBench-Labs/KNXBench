/** U20: the session-keyed flow reducer — identities, configured targets, value slots, limits. */
import { describe, expect, it } from "vitest";
import {
  FRESH_EVENT_MS,
  VALUE_TTL_MS,
  WINDOW_MS,
  admitRows,
  currentLeader,
  edgeActivity,
  createFlowModel,
  currentBadges,
  expireSlots,
  nextExpiryAt,
  provideContext,
  type FlowModel,
  type FlowRowInput,
} from "./flowModel";
import { snapshotJson } from "./flowTestFixtures";
import { parseFlowSnapshot, type FlowSnapshot } from "./flowWire";

const IDENTITY = { serverIncarnation: "inc-1", sessionId: 7 };
const IA = (area: number, line: number, device: number) => (area << 12) | (line << 8) | device;
const GA = (main: number, middle: number, sub: number) => (main << 11) | (middle << 8) | sub;

function member(deviceId: number, overrides: Record<string, unknown> = {}) {
  return {
    deviceId, comObjectId: deviceId * 100, direction: "Receive", active: true,
    flags: { communication: true, read: null, write: true, transmit: null, update: null, readOnInit: null },
    ...overrides,
  };
}

function device(deviceId: number, address: number | null, name = `Device ${deviceId}`) {
  return { deviceId, installationId: 1, name, individualAddressRaw: address };
}

function group(gaRaw: number, members: object[], overrides: Record<string, unknown> = {}) {
  return { gaRaw, gaId: gaRaw + 10_000, installationId: 1, name: `Group ${gaRaw}`, dpt: "1.001", members, ...overrides };
}

function snapshot(generation: string, devices: object[], groups: object[], overrides: Record<string, unknown> = {}): FlowSnapshot {
  return parseFlowSnapshot(snapshotJson({ generation, devices, groups, ...overrides }));
}

// Switch 1.1.1 sends on 1/0/1; actuators 1.1.2 and 1.1.3 are configured
// members; 1.1.4 is linked but its object is inactive.
const LIGHT = GA(1, 0, 1);
const BLIND = GA(1, 0, 2);
const DEVICES = [device(1, IA(1, 1, 1), "Switch"), device(2, IA(1, 1, 2), "Dimmer"), device(3, IA(1, 1, 3)), device(4, IA(1, 1, 4))];
const GROUPS = [
  group(LIGHT, [member(1, { direction: "Send" }), member(2), member(3), member(4, { active: false })]),
  group(BLIND, [member(1, { direction: "Send" }), member(2)]),
];

function row(seq: number, overrides: Partial<FlowRowInput> = {}): FlowRowInput {
  return {
    seq, service: "GroupValueWrite", source: "1.1.1", destination: "1/0/1",
    decoded: { kind: "value", text: `v${seq}` },
    sourceRaw: IA(1, 1, 1), destinationRaw: LIGHT, observedAgeMs: 0, flowGeneration: "1",
    ...overrides,
  };
}

function ready(generation = "1", devices: object[] = DEVICES, groups: object[] = GROUPS): FlowModel {
  const model = createFlowModel(IDENTITY);
  provideContext(model, generation, snapshot(generation, devices, groups), 0);
  return model;
}

const values = (model: FlowModel, node: string, now: number) => currentBadges(model, node, now).current.map((slot) => slot.value);

describe("flow reducer — participants", () => {
  it("connects an exact source to its active configured members, never to itself or an inactive object", () => {
    const model = ready();
    admitRows(model, [row(1)], 1000);
    expect([...model.edges.keys()].sort()).toEqual(["d:1→d:2", "d:1→d:3"]);
    expect(model.nodes.get("d:1")).toMatchObject({ kind: "device", label: "Switch" });
    expect(model.edges.get("d:1→d:2")).toMatchObject({ configured: true, count: 1 });
    expect(model.nodes.has("d:4")).toBe(false);
  });

  it("keeps the linked objects of both ends per group address as Inspector evidence", () => {
    const model = ready();
    admitRows(model, [row(1)], 1000);
    const evidence = model.edges.get("d:1→d:2")!.groups.get(LIGHT)!;
    expect(evidence.sourceObjects.map((o) => [o.comObjectId, o.direction])).toEqual([[100, "Send"]]);
    expect(evidence.targetObjects).toEqual([{
      comObjectId: 200, direction: "Receive", active: true,
      flags: { communication: true, read: null, write: true, transmit: null, update: null, readOnInit: null },
    }]);
    expect(evidence.generation).toBe("1");
    const raw = createFlowModel(IDENTITY);
    provideContext(raw, "1", snapshot("1", [], [], { status: "historical", groupAddressStyle: null }), 0);
    admitRows(raw, [row(1)], 1000);
    expect(raw.edges.get("ia:4353→g:2049")!.groups.get(LIGHT)).toMatchObject({ sourceObjects: [], targetObjects: [] });
  });

  it("names an address held by several devices ambiguous and excludes its candidates from the targets", () => {
    const devices = [...DEVICES, device(5, IA(1, 1, 1), "Twin")];
    const model = ready("1", devices, [group(LIGHT, [member(5), member(2)])]);
    admitRows(model, [row(1)], 1000);
    expect(model.nodes.get("ia:4353")).toMatchObject({ kind: "ambiguousSource", candidates: [1, 5] });
    expect([...model.edges.keys()]).toEqual(["ia:4353→d:2"]);
  });

  it("keeps an unknown sender and an unknown group address as raw nodes", () => {
    const model = ready();
    admitRows(model, [row(1, { sourceRaw: IA(2, 2, 2), source: "2.2.2", destinationRaw: GA(5, 5, 5), destination: "5/5/5" })], 1000);
    expect(model.nodes.get("ia:8706")).toMatchObject({ kind: "unresolvedSource", label: "2.2.2" });
    expect(model.nodes.get("g:11525")).toMatchObject({ kind: "group", label: "5/5/5" });
    expect([...model.edges.keys()]).toEqual(["ia:8706→g:11525"]);
    expect(model.edges.get("ia:8706→g:11525")!.configured).toBe(false);
  });

  it("does not pick one of several groups sharing a raw address", () => {
    const model = ready("1", DEVICES, [group(LIGHT, [member(2)]), group(LIGHT, [member(3)], { gaId: 99, installationId: 2 })]);
    admitRows(model, [row(1)], 1000);
    expect([...model.edges.keys()]).toEqual(["d:1→g:2049"]);
    expect(model.nodes.get("g:2049")).toMatchObject({ kind: "group", ambiguous: true });
  });

  it("keeps a Response on its observed responder instead of reversing the edge", () => {
    const model = ready();
    admitRows(model, [row(1, { service: "GroupValueResponse", source: "1.1.2", sourceRaw: IA(1, 1, 2) })], 1000);
    expect([...model.edges.keys()].sort()).toEqual(["d:2→d:1", "d:2→d:3"]);
  });

  it("resolves each row with its own generation, never with a newer project", () => {
    const model = ready("1");
    admitRows(model, [row(1)], 1000);
    provideContext(model, "2", snapshot("2", DEVICES, [group(LIGHT, [member(1, { direction: "Send" }), member(4)])]), 1000);
    admitRows(model, [row(2, { flowGeneration: "2" })], 1100);
    expect(model.edges.get("d:1→d:2")!.count).toBe(1);
    expect(model.edges.get("d:1→d:4")!.count).toBe(1);
    expect(model.edges.get("d:1→d:3")!.count).toBe(1);
  });

  it("resolves a re-addressed device only with the generation that states its new address", () => {
    const model = ready("1");
    const moved = [device(1, IA(1, 1, 9), "Switch"), ...DEVICES.slice(1)];
    provideContext(model, "2", snapshot("2", moved, GROUPS), 1000);
    admitRows(model, [row(1), row(2, { flowGeneration: "2" }), row(3, { flowGeneration: "2", sourceRaw: IA(1, 1, 9), source: "1.1.9" })], 1000);
    expect(model.edges.get("d:1→d:2")!.count).toBe(2);
    expect(model.nodes.get("ia:4353")).toMatchObject({ kind: "unresolvedSource", label: "1.1.1" });
    expect(model.edges.get("ia:4353→d:2")!.count).toBe(1);
  });

  it("draws historical and no-project rows raw, without configured targets", () => {
    for (const status of ["historical", "unavailable"] as const) {
      const model = createFlowModel(IDENTITY);
      provideContext(model, "1", snapshot("1", [], [], { status, groupAddressStyle: null }), 0);
      admitRows(model, [row(1)], 1000);
      expect(model.nodes.get("ia:4353")).toMatchObject({ kind: "rawSource", context: status });
      expect([...model.edges.keys()]).toEqual(["ia:4353→g:2049"]);
      expect(values(model, "g:2049", 1000)).toEqual(["v1"]);
      expect(values(model, "ia:4353", 1000)).toEqual(["v1"]);
    }
  });

  it("drops evidence that a newer generation no longer states", () => {
    const twin = [...DEVICES, device(5, IA(1, 1, 1), "Twin")];
    const model = ready("1", twin, [group(LIGHT, [member(2)]), group(LIGHT, [member(3)], { gaId: 99 })]);
    admitRows(model, [row(1)], 1000);
    expect(model.nodes.get("ia:4353")).toMatchObject({ kind: "ambiguousSource", candidates: [1, 5] });
    expect(model.nodes.get("g:2049")).toMatchObject({ ambiguous: true });
    provideContext(model, "2", snapshot("2", [], [], { status: "historical", groupAddressStyle: null }), 1000);
    admitRows(model, [row(2, { flowGeneration: "2" })], 1100);
    expect(model.nodes.get("ia:4353")).toEqual({ id: "ia:4353", kind: "rawSource", label: "1.1.1", address: "1.1.1", context: "historical" });
    expect(model.nodes.get("g:2049")).toEqual({ id: "g:2049", kind: "group", label: "1/0/1", ambiguous: false });
  });

  it("says when the participant list was truncated", () => {
    const model = createFlowModel(IDENTITY);
    provideContext(model, "1", snapshot("1", DEVICES, GROUPS, { truncated: { devices: 2, groups: 0, members: 0, diagnostics: 0 } }), 0);
    expect(model.contexts.get("1")).toMatchObject({ kind: "participants", complete: false });
  });

  it("ignores a snapshot of another session, server or generation", () => {
    const model = createFlowModel(IDENTITY);
    expect(provideContext(model, "1", snapshot("1", DEVICES, GROUPS, { sessionId: 8 }), 0)).toBe(false);
    expect(provideContext(model, "1", snapshot("1", DEVICES, GROUPS, { serverIncarnation: "inc-2" }), 0)).toBe(false);
    expect(provideContext(model, "1", snapshot("2", DEVICES, GROUPS), 0)).toBe(false);
    expect(model.contexts.has("1")).toBe(false);
  });
});

describe("flow reducer — admission", () => {
  it("queues rows until their generation is known, asks for it once, and admits in sequence order", () => {
    const model = createFlowModel(IDENTITY);
    expect(admitRows(model, [row(2), row(1)], 1000)).toEqual(["1"]);
    expect(admitRows(model, [row(3)], 1500)).toEqual([]);
    expect(model.edges.size).toBe(0);
    expect(model.pending).toHaveLength(3);
    provideContext(model, "1", snapshot("1", DEVICES, GROUPS), 1600);
    expect(model.pending).toHaveLength(0);
    expect(model.edges.get("d:1→d:2")!.count).toBe(3);
    expect(values(model, "d:2", 1600)).toEqual(["v3"]);
  });

  it("counts a repeated poll delivery once", () => {
    const model = ready();
    admitRows(model, [row(1), row(2)], 1000);
    admitRows(model, [row(1), row(2)], 1100);
    expect(model.edges.get("d:1→d:2")!.count).toBe(2);
    expect(model.counters.duplicates).toBe(2);
  });

  it("orders a batch by sequence, so the newest value wins", () => {
    const model = ready();
    admitRows(model, [row(3), row(2)], 1000);
    expect(values(model, "d:2", 1000)).toEqual(["v3"]);
  });

  it("treats the session marker as the end of traffic, not as traffic", () => {
    const model = ready();
    admitRows(model, [row(1, { service: "SessionClosed", sourceRaw: null, destinationRaw: null, flowGeneration: null })], 1000);
    expect(model.closed).toBe(true);
    expect(model.nodes.size).toBe(0);
  });

  it("counts legacy and malformed rows without drawing them", () => {
    const model = ready();
    admitRows(model, [
      { seq: 1, service: "GroupValueWrite", source: "1.1.1", destination: "1/0/1", decoded: null },
      row(2, { sourceRaw: 70_000 }),
    ], 1000);
    expect(model.counters).toMatchObject({ legacy: 1, malformed: 1 });
    expect(model.nodes.size).toBe(0);
  });

  it("refuses growth past the limits, keeps what exists and still admits values for it", () => {
    const model = createFlowModel(IDENTITY, { maxNodes: 3, maxEdges: 10, maxSlots: 100, maxPending: 100 });
    provideContext(model, "1", snapshot("1", DEVICES, GROUPS), 0);
    admitRows(model, [row(1)], 1000);
    admitRows(model, [row(2, { sourceRaw: IA(3, 3, 3), source: "3.3.3" })], 1000);
    expect(model.nodes.size).toBe(3);
    expect(model.counters.refusedNodes).toBeGreaterThan(0);
    expect(model.edges.has("d:1→d:2")).toBe(true);
    expect(values(model, "d:2", 1000)).toEqual(["v2"]);
  });

  it("admits a full queue raw instead of growing it without bound", () => {
    const model = createFlowModel(IDENTITY, { maxNodes: 100, maxEdges: 100, maxSlots: 100, maxPending: 2 });
    admitRows(model, [row(1), row(2), row(3)], 1000);
    expect(model.pending.length).toBeLessThanOrEqual(2);
    expect(model.counters.pendingOverflow).toBe(1);
    expect(model.nodes.get("ia:4353")).toMatchObject({ kind: "rawSource", context: "pendingOverflow" });
  });
});

describe("flow reducer — values", () => {
  it("updates source and configured targets immediately and marks the targets inferred", () => {
    const model = ready();
    admitRows(model, [row(1)], 1000);
    expect(currentBadges(model, "d:1", 1000).current[0]).toMatchObject({ value: "v1", origin: "source", gaLabel: "1/0/1" });
    expect(currentBadges(model, "d:2", 1000).current[0]).toMatchObject({ value: "v1", origin: "configuredTarget", sourceNode: "d:1" });
  });

  it("expires at exactly seven seconds after the observation, not after arrival", () => {
    const model = ready();
    admitRows(model, [row(1, { observedAgeMs: 200 })], 1000);
    expect(values(model, "d:2", 800 + VALUE_TTL_MS - 1)).toEqual(["v1"]);
    expect(values(model, "d:2", 800 + VALUE_TTL_MS)).toEqual([]);
    expect(nextExpiryAt(model)).toBe(800 + VALUE_TTL_MS);
    expireSlots(model, 800 + VALUE_TTL_MS);
    expect(model.slotCount).toBe(0);
    expect(nextExpiryAt(model)).toBeNull();
  });

  it("does not revive a value that was already older than its lifetime when it arrived", () => {
    const model = ready();
    admitRows(model, [row(1, { observedAgeMs: VALUE_TTL_MS })], 50_000);
    expect(values(model, "d:2", 50_000)).toEqual([]);
    expect(model.slotCount).toBe(0);
    expect(nextExpiryAt(model)).toBeNull();
    expect(model.edges.get("d:1→d:2")!.count).toBe(1);
  });

  it("shows no live value for a row of unknown age", () => {
    const model = ready();
    admitRows(model, [row(1, { observedAgeMs: null })], 1000);
    expect(values(model, "d:2", 1000)).toEqual([]);
    expect(model.counters.unknownAge).toBe(1);
  });

  it("lets a read neither set a value nor renew a deadline", () => {
    const model = ready();
    admitRows(model, [row(1)], 1000);
    admitRows(model, [row(2, { service: "GroupValueRead", decoded: { kind: "value", text: "read" } })], 5000);
    expect(values(model, "d:2", 7999)).toEqual(["v1"]);
    expect(values(model, "d:2", 8000)).toEqual([]);
  });

  it("invents no value from an undecodable or unresolved payload", () => {
    const model = ready();
    admitRows(model, [row(1, { decoded: { kind: "error", text: "bad" } }), row(2, { decoded: { kind: "unresolved", text: "0x01" } })], 1000);
    expect(values(model, "d:2", 1000)).toEqual([]);
    expect(model.edges.get("d:1→d:2")!.count).toBe(2);
  });

  it("keeps one slot per group address and replaces it only with a newer sequence", () => {
    const model = ready();
    admitRows(model, [row(1), row(2, { destinationRaw: BLIND, destination: "1/0/2" })], 1000);
    admitRows(model, [row(3)], 2000);
    expect(currentBadges(model, "d:2", 2000).current.map((slot) => [slot.gaLabel, slot.value])).toEqual([["1/0/1", "v3"], ["1/0/2", "v2"]]);
  });

  it("lets the newest sender win a shared target slot and keeps who sent it", () => {
    const model = ready();
    admitRows(model, [row(1), row(2, { source: "1.1.3", sourceRaw: IA(1, 1, 3) })], 1000);
    expect(currentBadges(model, "d:2", 1000).current[0]).toMatchObject({ value: "v2", sourceNode: "d:3", sourceLabel: "Device 3 (1.1.3)" });
  });

  it("shows at most three current values, newest first, and counts the rest", () => {
    const groups = [0, 1, 2, 3, 4].map((sub) => group(GA(2, 0, sub), [member(1, { direction: "Send" }), member(2)]));
    const model = ready("1", DEVICES, groups);
    admitRows(model, [0, 1, 2, 3, 4].map((sub) => row(sub + 1, { destinationRaw: GA(2, 0, sub), destination: `2/0/${sub}` })), 1000);
    const badges = currentBadges(model, "d:2", 1000);
    expect(badges.current.map((slot) => slot.gaLabel)).toEqual(["2/0/4", "2/0/3", "2/0/2"]);
    expect(badges.overflow).toBe(2);
  });
});

describe("flow reducer — activity window", () => {
  it("names the most active observed sender of the last 60 s, counting fan-out once", () => {
    const model = ready();
    admitRows(model, [row(1), row(2), row(3, { source: "1.1.2", sourceRaw: IA(1, 1, 2) })], 1000);
    expect(currentLeader(model, 1000)).toBe("d:1");
    expect(model.sendTimes.get("d:1")).toHaveLength(2);
  });

  it("keeps the current leader on an exact tie and otherwise picks the lowest identity", () => {
    const model = ready();
    admitRows(model, [row(1, { source: "1.1.3", sourceRaw: IA(1, 1, 3) })], 1000);
    expect(currentLeader(model, 1000)).toBe("d:3");
    admitRows(model, [row(2)], 1100);
    expect(currentLeader(model, 1100)).toBe("d:3");
    const fresh = ready();
    admitRows(fresh, [row(1, { source: "1.1.3", sourceRaw: IA(1, 1, 3) }), row(2)], 1000);
    expect(currentLeader(fresh, 1000)).toBe("d:1");
  });

  it("drops observations at exactly 60 s and has no leader without traffic", () => {
    const model = ready();
    expect(currentLeader(model, 0)).toBeNull();
    admitRows(model, [row(1)], 1000);
    expect(currentLeader(model, 1000 + WINDOW_MS - 1)).toBe("d:1");
    expect(currentLeader(model, 1000 + WINDOW_MS)).toBeNull();
    expect(model.sendTimes.has("d:1")).toBe(false);
  });

  it("does not place rows of unknown age in the window", () => {
    const model = ready();
    admitRows(model, [row(1, { observedAgeMs: null })], 1000);
    expect(currentLeader(model, 1000)).toBeNull();
    expect(edgeActivity(model.edges.get("d:1→d:2")!, 1000)).toBe(0);
  });

  it("counts edge activity within the window", () => {
    const model = ready();
    admitRows(model, [row(1), row(2)], 1000);
    admitRows(model, [row(3)], 30_000);
    const edge = model.edges.get("d:1→d:2")!;
    expect(edgeActivity(edge, 30_000)).toBe(3);
    expect(edgeActivity(edge, 1000 + WINDOW_MS)).toBe(1);
  });

  it("records fresh rows as events for pulses, but not a reattached backlog", () => {
    const model = ready();
    admitRows(model, [row(1, { observedAgeMs: FRESH_EVENT_MS + 1 }), row(2)], 10_000);
    expect(model.events.map((event) => [event.seq, event.from, event.to])).toEqual([[2, "d:1", ["d:2", "d:3"]]]);
    expect(model.events[0]).toMatchObject({ gaRaw: LIGHT, gaLabel: "1/0/1", service: "GroupValueWrite" });
  });

  it("bounds the event ring and counts what it dropped", () => {
    const model = createFlowModel(IDENTITY, { maxNodes: 100, maxEdges: 100, maxSlots: 100, maxPending: 100, maxEvents: 3 });
    provideContext(model, "1", snapshot("1", DEVICES, GROUPS), 0);
    admitRows(model, [1, 2, 3, 4, 5].map((seq) => row(seq)), 1000);
    expect(model.events.map((event) => event.seq)).toEqual([3, 4, 5]);
    expect(model.counters.eventsDropped).toBe(2);
  });

  // AR21 finding 6: whether a telegram can be drawn completely is decided
  // where nodes are refused, so the renderer only has to read it.
  it("marks an event incomplete when a recipient or the sender was refused, and gives a refused sender no send times", () => {
    const model = createFlowModel(IDENTITY, { maxNodes: 2, maxEdges: 100, maxSlots: 100, maxPending: 100 });
    provideContext(model, "1", snapshot("1", DEVICES, GROUPS), 0);
    admitRows(model, [row(1)], 1000);
    expect(model.events.map((event) => [event.seq, event.from, event.to, event.complete])).toEqual([[1, "d:1", ["d:2"], false]]);
    admitRows(model, [row(2, { source: "1.1.3", sourceRaw: 0x1103 })], 1000);
    expect(model.nodes.has("d:3")).toBe(false);
    expect(model.events[1]).toMatchObject({ seq: 2, from: "d:3", to: [], complete: false });
    expect(model.sendTimes.has("d:3")).toBe(false);
    expect(model.counters.eventsRecorded).toBe(2);
  });

  it("marks an event complete when every recipient has a node", () => {
    const model = ready();
    admitRows(model, [row(1)], 1000);
    expect(model.events[0]).toMatchObject({ to: ["d:2", "d:3"], complete: true });
  });
});
