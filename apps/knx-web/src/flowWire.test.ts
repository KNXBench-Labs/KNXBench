/** U20: the telegram-flow wire contract (AR20) is validated before anything is drawn. */
import { describe, expect, it } from "vitest";
import { parseFlowSnapshot, rowFlowFacts } from "./flowWire";
import { snapshotJson } from "./flowTestFixtures";

describe("parseFlowSnapshot", () => {
  it("accepts the AR20 snapshot and keeps every stated fact", () => {
    const snapshot = parseFlowSnapshot(snapshotJson());
    expect(snapshot.generation).toBe("1");
    expect(snapshot.devices[0]).toEqual({ deviceId: 1, installationId: 1, name: "Switch", individualAddressRaw: 0x1101 });
    expect(snapshot.groups[0].members[0].flags).toEqual({
      communication: true, read: null, write: false, transmit: true, update: null, readOnInit: null,
    });
    expect(snapshot.truncated).toEqual({ devices: 0, groups: 0, members: 0, diagnostics: 0 });
  });

  it("accepts the empty historical and unavailable answers", () => {
    for (const status of ["historical", "unavailable"]) {
      const snapshot = parseFlowSnapshot(snapshotJson({ status, groupAddressStyle: null, devices: [], groups: [] }));
      expect(snapshot.status).toBe(status);
    }
  });

  it.each([
    ["a device id beyond u32", { devices: [{ deviceId: 2 ** 32, installationId: 1, name: "x", individualAddressRaw: null }] }],
    ["a fractional session id", { sessionId: 1.5 }],
    ["an unsafe session id", { sessionId: 2 ** 53 }],
    ["a group address beyond u16", { groups: [{ ...(snapshotJson().groups as object[])[0], gaRaw: 65536 }] }],
    ["a non-canonical generation", { generation: "01" }],
    ["a numeric generation", { generation: 1 }],
    ["an unknown status", { status: "fresh" }],
    ["an unknown address style", { groupAddressStyle: "FourLevel" }],
    ["a member without flags", {
      groups: [{ ...(snapshotJson().groups as object[])[0], members: [{ deviceId: 1, comObjectId: 1, direction: "Send", active: true }] }],
    }],
    ["an unknown direction", {
      groups: [{
        ...(snapshotJson().groups as object[])[0],
        members: [{
          deviceId: 1, comObjectId: 1, direction: "Both", active: true,
          flags: { communication: true, read: null, write: false, transmit: true, update: null, readOnInit: null },
        }],
      }],
    }],
    ["a missing truncation record", { truncated: undefined }],
    ["a member with a flag left out", {
      groups: [{
        ...(snapshotJson().groups as object[])[0],
        members: [{
          deviceId: 1, comObjectId: 1, direction: "Send", active: true,
          flags: { communication: true, read: null, write: false, transmit: true, update: null },
        }],
      }],
    }],
  ])("refuses %s", (_name, overrides) => {
    expect(() => parseFlowSnapshot(snapshotJson(overrides))).toThrow(/flow snapshot/);
  });
});

describe("rowFlowFacts", () => {
  const base = { sourceRaw: 0x1101, destinationRaw: 0x0801, observedAgeMs: 120, flowGeneration: "3" };

  it("reads the raw identities, age and generation of a row", () => {
    expect(rowFlowFacts(base)).toEqual({ kind: "facts", sourceRaw: 0x1101, destinationRaw: 0x0801, observedAgeMs: 120, flowGeneration: "3" });
  });

  it("keeps an unknown age as unknown", () => {
    expect(rowFlowFacts({ ...base, observedAgeMs: null })).toMatchObject({ kind: "facts", observedAgeMs: null });
  });

  it("names a row from a server without the flow fields as legacy", () => {
    expect(rowFlowFacts({})).toEqual({ kind: "legacy" });
  });

  it("names the session marker, which is not traffic", () => {
    expect(rowFlowFacts({ sourceRaw: null, destinationRaw: null, observedAgeMs: 5, flowGeneration: null })).toEqual({ kind: "marker" });
  });

  it.each([
    ["an out-of-range source", { sourceRaw: 70000 }],
    ["a negative destination", { destinationRaw: -1 }],
    ["a fractional age", { observedAgeMs: 1.5 }],
    ["an unsafe age", { observedAgeMs: 2 ** 53 }],
    ["a malformed generation", { flowGeneration: "1e3" }],
    ["half a marker", { sourceRaw: null }],
  ])("refuses %s as malformed", (_name, overrides) => {
    expect(rowFlowFacts({ ...base, ...overrides })).toEqual({ kind: "malformed" });
  });
});
