/** U20: the flow feed — one model per session, one snapshot fetch per generation, timed expiry. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useFlowFeed, type FlowFeed, type FlowIdentity } from "./flowFeed";
import { snapshotJson } from "./flowTestFixtures";
import type { FlowRowInput } from "./flowModel";
import { parseFlowSnapshot, type FlowSnapshot } from "./flowWire";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const SESSION: FlowIdentity = { serverIncarnation: "inc-1", sessionId: 7 };
const SWITCH = 0x1101;
const LIGHT = 0x0801;

function row(seq: number, overrides: Partial<FlowRowInput> = {}): FlowRowInput {
  return {
    seq, service: "GroupValueWrite", source: "1.1.1", destination: "1/0/1", decoded: { kind: "value", text: `v${seq}` },
    sourceRaw: SWITCH, destinationRaw: LIGHT, observedAgeMs: 0, flowGeneration: "1", ...overrides,
  };
}

function snapshot(overrides: Record<string, unknown> = {}): FlowSnapshot {
  return parseFlowSnapshot(snapshotJson({
    devices: [
      { deviceId: 1, installationId: 1, name: "Switch", individualAddressRaw: SWITCH },
      { deviceId: 2, installationId: 1, name: "Dimmer", individualAddressRaw: 0x1102 },
    ],
    groups: [{
      gaRaw: LIGHT, gaId: 10, installationId: 1, name: "Light", dpt: "1.001",
      members: [1, 2].map((deviceId) => ({
        deviceId, comObjectId: deviceId * 100, direction: deviceId === 1 ? "Send" : "Receive", active: true,
        flags: { communication: true, read: null, write: true, transmit: null, update: null, readOnInit: null },
      })),
    }],
    ...overrides,
  }));
}

let host: HTMLDivElement | undefined;
let root: Root | undefined;
let feed: FlowFeed;
let renders = 0;

function Harness({ fetchSnapshot }: { fetchSnapshot: (sessionId: number, generation: string) => Promise<FlowSnapshot> }) {
  feed = useFlowFeed(fetchSnapshot);
  renders += 1;
  return null;
}

async function mount(fetchSnapshot: (sessionId: number, generation: string) => Promise<FlowSnapshot>) {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => root!.render(<Harness fetchSnapshot={fetchSnapshot} />));
}

const settle = () => act(async () => { await vi.advanceTimersByTimeAsync(0); });

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "setInterval", "clearInterval", "Date", "performance"] });
  renders = 0;
});

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  vi.useRealTimers();
});

describe("useFlowFeed", () => {
  it("fetches each new generation once and resolves the rows that waited for it", async () => {
    const fetchSnapshot = vi.fn(async () => snapshot());
    await mount(fetchSnapshot);
    await act(async () => feed.admit(SESSION, [row(1)]));
    await act(async () => feed.admit(SESSION, [row(2)]));
    await settle();
    expect(fetchSnapshot).toHaveBeenCalledExactlyOnceWith(7, "1");
    expect(feed.model!.edges.get("d:1→d:2")!.count).toBe(2);
  });

  it("ignores a snapshot that arrives after the session changed", async () => {
    const answers: ((value: FlowSnapshot) => void)[] = [];
    const fetchSnapshot = vi.fn(() => new Promise<FlowSnapshot>((resolve) => { answers.push(resolve); }));
    await mount(fetchSnapshot);
    await act(async () => feed.admit(SESSION, [row(1)]));
    await settle();
    await act(async () => feed.admit({ serverIncarnation: "inc-1", sessionId: 8 }, [row(1)]));
    await settle();
    const second = feed.model!;
    expect(answers).toHaveLength(2);
    // The first session's late answer must not touch the second session.
    await act(async () => { answers[0](snapshot()); await vi.advanceTimersByTimeAsync(0); });
    expect(feed.model).toBe(second);
    expect(second.identity.sessionId).toBe(8);
    expect(second.edges.size).toBe(0);
    expect(second.pending).toHaveLength(1);
  });

  it("draws the rows raw when the snapshot cannot be fetched or belongs elsewhere", async () => {
    for (const fetchSnapshot of [vi.fn(async () => { throw new Error("offline"); }), vi.fn(async () => snapshot({ sessionId: 99 }))]) {
      await mount(fetchSnapshot);
      await act(async () => feed.admit(SESSION, [row(1)]));
      await settle();
      expect(feed.model!.pending).toHaveLength(0);
      expect(feed.model!.nodes.get("ia:4353")).toMatchObject({ kind: "rawSource", context: "failed" });
      await act(async () => root!.unmount());
      root = undefined;
    }
  });

  it("expires a value at its deadline with one timer and re-renders", async () => {
    await mount(async () => snapshot());
    await act(async () => feed.admit(SESSION, [row(1, { observedAgeMs: 1000 })]));
    await settle();
    // The sender and its one configured target.
    expect(feed.model!.slotCount).toBe(2);
    const before = renders;
    await act(async () => { await vi.advanceTimersByTimeAsync(5_999); });
    expect(feed.model!.slotCount).toBe(2);
    await act(async () => { await vi.advanceTimersByTimeAsync(1); });
    expect(feed.model!.slotCount).toBe(0);
    expect(renders).toBeGreaterThan(before);
  });

  it("starts a new model when the server restarted, even under the same session number", async () => {
    await mount(async () => snapshot());
    await act(async () => feed.admit(SESSION, [row(1)]));
    const first = feed.model;
    await act(async () => feed.admit({ serverIncarnation: "inc-2", sessionId: 7 }, [row(1)]));
    expect(feed.model).not.toBe(first);
    expect(feed.model!.identity.serverIncarnation).toBe("inc-2");
  });

  it("forgets the model on reset", async () => {
    await mount(async () => snapshot());
    await act(async () => feed.admit(SESSION, [row(1)]));
    await act(async () => feed.reset());
    expect(feed.model).toBeNull();
  });
});
