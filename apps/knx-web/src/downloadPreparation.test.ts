/** Gateway choice, device hand-off and step guide for the download tab. */
import { describe, expect, it } from "vitest";
import type { BusDiscoveredInterface } from "./api";
import type { BusDiscoveryState } from "./busDiscovery";
import type { ProjectTree } from "./bindings/ProjectTree";
import {
  gatewayNote, gatewayOptionLabel, initialGatewayChoice, isFound, pickedGateway, typedGateway, withDiscoveryDefault,
} from "./downloadGateway";
import { downloadHandoff, resolveHandoff } from "./downloadHandoff";
import { currentGuideStep, downloadGuide } from "./downloadGuide";

const iface = (endpoint: string, name = "IP interface", address = "1.1.250"): BusDiscoveredInterface =>
  ({ controlEndpoint: endpoint, friendlyName: name, individualAddress: address, supportsTunnelling: true });
const discovery = (phase: BusDiscoveryState["phase"], interfaces: BusDiscoveredInterface[] = []): BusDiscoveryState =>
  ({ phase, interfaces, error: phase === "failed" ? "no route" : null });

describe("gateway choice", () => {
  it("starts from the preferred gateway, or empty", () => {
    expect(initialGatewayChoice(" 192.0.2.5:3671 ")).toEqual({ value: "192.0.2.5:3671", origin: "preferred" });
    expect(initialGatewayChoice("")).toEqual({ value: "", origin: "none" });
  });

  it("fills in exactly one found gateway, only while nobody chose anything", () => {
    const one = discovery("done", [iface("192.0.2.10:3671")]);
    expect(withDiscoveryDefault({ value: "", origin: "none" }, one)).toEqual({ value: "192.0.2.10:3671", origin: "auto" });
    for (const choice of [initialGatewayChoice("192.0.2.5:3671"), pickedGateway("192.0.2.7:3671"), typedGateway(""), typedGateway("10.0.0.1")]) {
      expect(withDiscoveryDefault(choice, one)).toBe(choice);
    }
  });

  it("does not choose among several, nor from a running or failed search", () => {
    const none = { value: "", origin: "none" } as const;
    expect(withDiscoveryDefault(none, discovery("done", [iface("192.0.2.10:3671"), iface("192.0.2.11:3671")]))).toBe(none);
    expect(withDiscoveryDefault(none, discovery("searching", [iface("192.0.2.10:3671")]))).toBe(none);
    expect(withDiscoveryDefault(none, discovery("failed", [iface("192.0.2.10:3671")]))).toBe(none);
    expect(withDiscoveryDefault(none, discovery("done"))).toBe(none);
  });

  it("calls a preferred gateway found only when the search returned it", () => {
    const preferred = initialGatewayChoice("192.0.2.5:3671");
    expect(gatewayNote(preferred, discovery("done", [iface("192.0.2.10:3671")]))).toBe("preferred");
    expect(gatewayNote(preferred, discovery("idle"))).toBe("preferred");
    expect(gatewayNote(preferred, discovery("done", [iface("192.0.2.5:3671")]))).toBe("preferredFound");
  });

  it("marks a search-derived choice stale when the latest finished search lacks it, keeping the value", () => {
    const picked = pickedGateway("192.0.2.10:3671");
    expect(gatewayNote(picked, discovery("done", [iface("192.0.2.10:3671")]))).toBeNull();
    expect(gatewayNote(picked, discovery("done", [iface("192.0.2.11:3671")]))).toBe("stale");
    expect(gatewayNote(picked, discovery("searching", []))).toBeNull();
    expect(gatewayNote({ value: "192.0.2.10:3671", origin: "auto" }, discovery("done"))).toBe("stale");
    expect(gatewayNote({ value: "192.0.2.10:3671", origin: "auto" }, discovery("done", [iface("192.0.2.10:3671")]))).toBe("auto");
    expect(gatewayNote(typedGateway("10.0.0.1:3671"), discovery("done"))).toBeNull();
  });

  it("tells interfaces with the same name apart by endpoint and interface address", () => {
    expect(gatewayOptionLabel(iface("192.0.2.10:3671", "KNX IP"))).toBe("KNX IP — 192.0.2.10:3671 · 1.1.250");
    expect(gatewayOptionLabel(iface("192.0.2.11:3671", "KNX IP", "1.1.251"))).toBe("KNX IP — 192.0.2.11:3671 · 1.1.251");
    expect(isFound(" 192.0.2.10:3671 ", [iface("192.0.2.10:3671")])).toBe(true);
    expect(isFound("", [iface("192.0.2.10:3671")])).toBe(false);
  });
});

function project(incarnation = 1): ProjectTree {
  const device = (id: number, address: string | null, name: string) => ({ id, name, address, description: null, com_object_count: 0 });
  return {
    schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "srv", project_incarnation: incarnation,
    installations: [{ id: 1, name: "I", buildings: [], group_addresses: [], group_ranges: [],
      topology: [{ id: 2, name: "A", address: 1, lines: [{ id: 3, name: "L", address: 1, devices: [device(10, "1.1.67", "Push button")] }] }],
      unassigned: [device(12, null, "Not yet addressed")] }],
  };
}

describe("device hand-off", () => {
  it("selects by stable id in the same project", () => {
    expect(resolveHandoff(downloadHandoff(project(), 10, 1), project(), "idle"))
      .toEqual({ kind: "selected", deviceId: 10, name: "Push button", address: "1.1.67" });
  });

  it("refuses a request from another project, a missing device or one without an individual address", () => {
    expect(resolveHandoff(downloadHandoff(project(1), 10, 1), project(2), "idle")).toEqual({ kind: "stale" });
    expect(resolveHandoff(downloadHandoff(project(), 10, 1), null, "idle")).toEqual({ kind: "stale" });
    expect(resolveHandoff(downloadHandoff(project(), 99, 1), project(), "idle")).toEqual({ kind: "missing" });
    expect(resolveHandoff(downloadHandoff(project(), 12, 1), project(), "idle")).toEqual({ kind: "noAddress", name: "Not yet addressed" });
  });

  it("never switches devices while busy or while a running download cannot be ruled out", () => {
    expect(resolveHandoff(downloadHandoff(project(), 10, 1), project(), "busy")).toEqual({ kind: "busy" });
    expect(resolveHandoff(downloadHandoff(project(), 10, 1), project(), "unknown")).toEqual({ kind: "unknownStatus" });
  });
});

describe("step guide", () => {
  const plan = { untestedAcknowledgement: null };
  it("names what is missing, in order", () => {
    const steps = downloadGuide({ deviceSelected: false, gateway: "", plan: null, acknowledgement: "" });
    expect(steps.map((step) => [step.id, step.state, step.missing])).toEqual([
      ["device", "open", ["device"]], ["gateway", "open", ["gateway"]], ["scope", "done", []],
      ["plan", "blocked", ["device"]], ["confirm", "blocked", ["plan"]],
    ]);
    expect(currentGuideStep(steps)).toBe("device");
  });

  it("asks for the plan once a device is chosen and leaves confirmation open after it", () => {
    const before = downloadGuide({ deviceSelected: true, gateway: "192.0.2.10:3671", plan: null, acknowledgement: "" });
    expect(before.find((step) => step.id === "plan")).toEqual({ id: "plan", state: "open", missing: ["plan"] });
    expect(currentGuideStep(before)).toBe("plan");
    const ready = downloadGuide({ deviceSelected: true, gateway: "192.0.2.10:3671", plan, acknowledgement: "" });
    expect(ready.find((step) => step.id === "confirm")).toEqual({ id: "confirm", state: "open", missing: [] });
    expect(currentGuideStep(ready)).toBe("confirm");
  });

  it("requires the exact acknowledgement of an untested plan and a gateway before starting", () => {
    const untested = { untestedAcknowledgement: "I accept untested download to 1.1.67" };
    const steps = downloadGuide({ deviceSelected: true, gateway: " ", plan: untested, acknowledgement: "I accept" });
    expect(steps.find((step) => step.id === "confirm")?.missing).toEqual(["gateway", "acknowledgement"]);
    const done = downloadGuide({ deviceSelected: true, gateway: "192.0.2.10:3671", plan: untested, acknowledgement: untested.untestedAcknowledgement });
    expect(done.find((step) => step.id === "confirm")?.missing).toEqual([]);
  });
});
