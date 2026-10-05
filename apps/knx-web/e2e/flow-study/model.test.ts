/** U19 study: semantics of the synthetic telegram-flow model, pinned before the renderer. */
import { describe, expect, it } from "vitest";
import {
  MAX_BADGES,
  VALUE_TTL_MS,
  WINDOW_MS,
  admit,
  badges,
  coalescePulses,
  createFlowState,
  currentLeader,
  type FlowLimits,
  type StudyEvent,
} from "./model";

let seq = 0;
function write(source: string, ga: string, value: string, at: number, targets: string[] = []): StudyEvent {
  return { seq: ++seq, observedAtMs: at, source, ga, service: "GroupValueWrite", value, targets };
}
function read(source: string, ga: string, at: number, targets: string[] = []): StudyEvent {
  return { seq: ++seq, observedAtMs: at, source, ga, service: "GroupValueRead", targets };
}

describe("value slots", () => {
  it("shows a written value at the source and every configured target at once", () => {
    const state = createFlowState();
    admit(state, write("1.1.1", "1/1/1", "On", 0, ["1.1.2", "1.1.3"]));
    for (const device of ["1.1.1", "1.1.2", "1.1.3"]) {
      expect(badges(state, device, 0).current.map((slot) => [slot.ga, slot.value])).toEqual([["1/1/1", "On"]]);
    }
    expect(badges(state, "1.1.2", 0).current[0].inferred).toBe(true);
    expect(badges(state, "1.1.1", 0).current[0].inferred).toBe(false);
  });

  it("expires a value exactly 7 seconds after its observation, not before", () => {
    const state = createFlowState();
    admit(state, write("1.1.1", "1/1/1", "On", 1000));
    expect(VALUE_TTL_MS).toBe(7000);
    expect(badges(state, "1.1.1", 1000 + VALUE_TTL_MS - 1).current).toHaveLength(1);
    expect(badges(state, "1.1.1", 1000 + VALUE_TTL_MS).current).toHaveLength(0);
  });

  it("lets a newer value replace the slot and reset its deadline, but never an older one", () => {
    const state = createFlowState();
    const first = write("1.1.1", "1/1/1", "On", 0);
    const second = write("1.1.4", "1/1/1", "Off", 5000);
    admit(state, second);
    admit(state, first);
    expect(badges(state, "1.1.1", 5000).current).toHaveLength(0);
    admit(state, write("1.1.1", "1/1/1", "On", 6000));
    expect(badges(state, "1.1.1", 12_999).current.map((slot) => slot.value)).toEqual(["On"]);
  });

  it("does not let a read invent a value or renew an existing deadline", () => {
    const state = createFlowState();
    admit(state, write("1.1.1", "1/1/1", "On", 0));
    admit(state, read("1.1.9", "1/1/1", 6000, ["1.1.1"]));
    expect(badges(state, "1.1.1", 7000).current).toHaveLength(0);
    expect(badges(state, "1.1.9", 6000).current).toHaveLength(0);
  });

  it("does not let a read set a value even when its row carries one", () => {
    const state = createFlowState();
    admit(state, write("1.1.1", "1/1/1", "On", 0));
    admit(state, { ...read("1.1.9", "1/1/1", 6000, ["1.1.1"]), value: "decoded-by-mistake" });
    expect(badges(state, "1.1.1", 6000).current.map((slot) => slot.value)).toEqual(["On"]);
    expect(badges(state, "1.1.1", 7000).current).toHaveLength(0);
    expect(badges(state, "1.1.9", 6000).current).toHaveLength(0);
  });

  it("keeps one slot per group address, shows at most three and counts the rest", () => {
    const state = createFlowState();
    ["1/1/1", "1/1/2", "1/1/3", "1/1/4", "1/1/5"].forEach((ga, index) => admit(state, write("1.1.1", ga, `v${index}`, index * 10)));
    const shown = badges(state, "1.1.1", 100);
    expect(MAX_BADGES).toBe(3);
    expect(shown.current.map((slot) => slot.ga)).toEqual(["1/1/5", "1/1/4", "1/1/3"]);
    expect(shown.overflow).toBe(2);
  });

  it("ignores a repeated delivery of the same sequence", () => {
    const state = createFlowState();
    const event = write("1.1.1", "1/1/1", "On", 0);
    admit(state, event);
    admit(state, { ...event });
    expect(state.edges.get("1.1.1→GA:1/1/1")?.count).toBe(1);
  });
});

describe("activity leader", () => {
  it("ranks observed senders only; fan-out to configured targets does not count", () => {
    const state = createFlowState();
    admit(state, write("1.1.1", "1/1/1", "On", 0, ["1.1.2", "1.1.3", "1.1.4"]));
    admit(state, write("1.1.5", "1/1/2", "On", 10));
    admit(state, write("1.1.5", "1/1/2", "Off", 20));
    expect(currentLeader(state, 30)).toBe("1.1.5");
  });

  it("keeps the current leader on an exact tie and has none without traffic in the window", () => {
    const state = createFlowState();
    expect(currentLeader(state, 0)).toBeNull();
    admit(state, write("1.1.2", "1/1/1", "On", 0));
    expect(currentLeader(state, 1)).toBe("1.1.2");
    admit(state, write("1.1.1", "1/1/2", "On", 5));
    expect(currentLeader(state, 6)).toBe("1.1.2");
    expect(WINDOW_MS).toBe(60_000);
    expect(currentLeader(state, 5 + WINDOW_MS)).toBeNull();
  });

  it("keeps an edge in the graph after its activity window and value expired", () => {
    const state = createFlowState();
    admit(state, write("1.1.1", "1/1/1", "On", 0, ["1.1.2"]));
    expect(currentLeader(state, 10 * WINDOW_MS)).toBeNull();
    expect([...state.edges.keys()].sort()).toEqual(["1.1.1→1.1.2"]);
  });
});

describe("pulse coalescing", () => {
  it("bundles pulses per directed pair and group address within one interval, keeping count and newest value", () => {
    const events = [1, 2, 3, 4, 5].map((n) => write("1.1.1", "1/1/1", `v${n}`, n, ["1.1.2"]));
    events.push(write("1.1.2", "1/1/1", "back", 6, ["1.1.1"]));
    const pulses = coalescePulses(events);
    expect(pulses.map((pulse) => [pulse.from, pulse.to, pulse.ga, pulse.count, pulse.value])).toEqual([
      ["1.1.1", "1.1.2", "1/1/1", 5, "v5"],
      ["1.1.2", "1.1.1", "1/1/1", 1, "back"],
    ]);
  });
});

describe("capacity", () => {
  it("refuses new graph growth at the limit, keeps every existing edge and counts what it refused", () => {
    const limits: FlowLimits = { maxNodes: 3, maxEdges: 2 };
    const state = createFlowState(limits);
    admit(state, write("1.1.1", "1/1/1", "On", 0, ["1.1.2"]));
    admit(state, write("1.1.1", "1/1/2", "On", 1, ["1.1.3"]));
    admit(state, write("1.1.4", "1/1/3", "On", 2, ["1.1.2"]));
    admit(state, write("1.1.1", "1/1/1", "Off", 3, ["1.1.2"]));
    expect([...state.edges.keys()].sort()).toEqual(["1.1.1→1.1.2", "1.1.1→1.1.3"]);
    expect(state.edges.get("1.1.1→1.1.2")?.count).toBe(2);
    expect(state.overflow).toEqual({ refusedEdges: 1, refusedNodes: 1 });
    // Admission is independent of graph capacity: the refused sender's value
    // still reaches the existing configured target's 1/1/3 slot.
    expect(badges(state, "1.1.2", 3).current.map((slot) => [slot.ga, slot.value])).toEqual([["1/1/1", "Off"], ["1/1/3", "On"]]);
  });
});

describe("slot index", () => {
  it("answers a device's badges from its own slots only, also with thousands of others", () => {
    const state = createFlowState();
    for (let i = 0; i < 3000; i += 1) admit(state, write(`9.1.${i}`, `1/1/${i % 200}`, "x", 10));
    admit(state, write("1.1.1", "2/2/2", "mine", 11));
    expect(badges(state, "1.1.1", 12).current.map((slot) => slot.value)).toEqual(["mine"]);
    expect(state.slotCount).toBe(3001);
  });
});
