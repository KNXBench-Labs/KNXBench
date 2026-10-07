/** Proves late-window graph adoption, clock conversion and stale packet refusal without polling. */
import { describe, expect, it } from "vitest";
import { acceptFlowPacket, initialFlowMirror, type FlowPacket } from "./flowChannel";
import { createFlowModel } from "./flowModel";

function packet(revision = 1): Extract<FlowPacket, { type: "snapshot" }> {
  const model = createFlowModel({ serverIncarnation: "a", sessionId: 7 });
  model.nodes.set("d:1", { id: "d:1", kind: "device", label: "Already observed", deviceId: 1 });
  return { type: "snapshot", revision, origin: 100000, sentAt: 9000, model, links: new Set(), paused: false };
}
describe("flow window snapshots", () => {
  it("adopts all accumulated nodes, not merely fresh events", () => {
    const next = acceptFlowPacket(initialFlowMirror(), packet(), 109000, 0);
    expect(next.model?.nodes.get("d:1")?.label).toBe("Already observed");
    expect(next.live).toBe(true);
  });
  it("ignores delayed revisions, including an old session after replacement", () => {
    const old = packet(1); const latest = packet(2); latest.model!.identity.sessionId = 8;
    const state = acceptFlowPacket(initialFlowMirror(), latest, 100000, 0);
    expect(acceptFlowPacket(state, old, 100000, 1)).toBe(state);
    expect(state.model!.identity.sessionId).toBe(8);
  });
  it("a source close retains the graph but removes live and navigation claims", () => {
    const state = acceptFlowPacket(initialFlowMirror(), packet(), 100000, 0);
    const next = acceptFlowPacket(state, { type: "closed" }, 100000, 1);
    expect(next.live).toBe(false); expect(next.model).toBe(state.model); expect(next.links.size).toBe(0);
  });
  it("does not label a delayed old snapshot live just because it arrived now", () => {
    const old = packet(); old.sentAt = 0;
    const next = acceptFlowPacket(initialFlowMirror(), old, 100000, 8000);
    expect(next.model?.nodes.size).toBe(1);
    expect(next.live).toBe(false);
    expect(next.receivedAt).toBe(0);
    expect(next.links.size).toBe(0);
  });
  it("refuses malformed snapshots without replacing current data", () => {
    const state = initialFlowMirror();
    for (const input of [null, {}, { ...packet(), origin: NaN }, { ...packet(), revision: -1 }, { ...packet(), model: {} }]) {
      expect(acceptFlowPacket(state, input, 100000, 0)).toBe(state);
    }
  });
});
