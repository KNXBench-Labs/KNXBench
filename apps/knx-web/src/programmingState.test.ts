/** Pending-programming parts: order, absence and wire admission (ADR-0129). */
import { describe, expect, it } from "vitest";
import { addressEndChangesProject, downloadEndChangesProject, hasPendingProgramming, isPendingProgramming,
  pendingParts } from "./programmingState";

const state = (address: boolean, application: boolean, parameters: boolean, groupAddresses: boolean) => ({
  pending_programming: { address, application, parameters, group_addresses: groupAddresses },
});

describe("pending programming", () => {
  it("lists pending parts in the Adr · App · Par · GA order", () => {
    expect(pendingParts(state(true, false, true, true))).toEqual(["address", "parameters", "group_addresses"]);
    expect(pendingParts(state(false, true, false, false))).toEqual(["application"]);
    expect(pendingParts(state(false, false, false, false))).toEqual([]);
  });
  it("shows nothing when the server sent no state", () => {
    expect(pendingParts({})).toEqual([]);
    expect(hasPendingProgramming({})).toBe(false);
    expect(hasPendingProgramming(state(false, false, false, true))).toBe(true);
    expect(hasPendingProgramming(state(false, false, false, false))).toBe(false);
  });
  it("admits exactly four booleans", () => {
    expect(isPendingProgramming(state(true, false, false, false).pending_programming)).toBe(true);
    expect(isPendingProgramming(null)).toBe(false);
    expect(isPendingProgramming({ address: true, application: false, parameters: false })).toBe(false);
    expect(isPendingProgramming({ address: "yes", application: false, parameters: false, group_addresses: false })).toBe(false);
    expect(isPendingProgramming({ address: true, application: false, parameters: false, group_addresses: false, medium: true }))
      .toBe(false);
  });
});

describe("which bus-write ends change the project", () => {
  it("is every download end that may have written", () => {
    expect(downloadEndChangesProject({ state: "running" })).toBe(false);
    expect(downloadEndChangesProject({ state: "finished", written: "yes", restart: "acknowledged", restartNote: null })).toBe(true);
    expect(downloadEndChangesProject({ state: "finished", written: "yes", restart: "unconfirmed", restartNote: "x" })).toBe(true);
    expect(downloadEndChangesProject({ state: "failed", written: "partially", stoppedInStep: 4, error: "lost" })).toBe(true);
    expect(downloadEndChangesProject({ state: "failed", written: "no", stoppedInStep: null, error: "refused" })).toBe(false);
  });
  it("is every address programming end that may have written", () => {
    expect(addressEndChangesProject({ state: "waiting", rounds: 1, inProgrammingMode: [] })).toBe(false);
    expect(addressEndChangesProject({ state: "programming", previousAddress: "15.15.255" })).toBe(false);
    expect(addressEndChangesProject({ state: "finished", written: "yes", previousAddress: "15.15.255", wasFree: true })).toBe(true);
    expect(addressEndChangesProject({ state: "finished", written: "noNeed", previousAddress: "1.1.3", wasFree: false })).toBe(true);
    expect(addressEndChangesProject({ state: "failed", written: "unconfirmed", step: 4, error: "silent" })).toBe(true);
    expect(addressEndChangesProject({ state: "failed", written: "no", step: null, error: "nobody" })).toBe(false);
    expect(addressEndChangesProject({ state: "failed", written: "noNeed", step: 4, error: "x" })).toBe(false);
    expect(addressEndChangesProject({ state: "stopped", rounds: 3 })).toBe(false);
  });
});
