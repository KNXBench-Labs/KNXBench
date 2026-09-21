/** Tests the discovery store: one search at a time, and a failure that keeps what it had. */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BusDiscoverResponse } from "./api";

const apiMock = vi.hoisted(() => ({ discoverBusInterfaces: vi.fn() }));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import {
  ensureBusDiscovery,
  readBusDiscoveryForTests,
  resetBusDiscoveryForTests,
  searchBusInterfaces,
} from "./busDiscovery";

const hallway = {
  controlEndpoint: "192.0.2.11:3671",
  individualAddress: "1.1.0",
  friendlyName: "Hallway interface",
  supportsTunnelling: true,
};

beforeEach(() => {
  resetBusDiscoveryForTests();
  apiMock.discoverBusInterfaces.mockResolvedValue({ interfaces: [] });
});

afterEach(() => {
  resetBusDiscoveryForTests();
  vi.clearAllMocks();
});

describe("busDiscovery", () => {
  it("publishes an empty result as a finished search, not a failure", async () => {
    await searchBusInterfaces();
    expect(readBusDiscoveryForTests()).toEqual({ phase: "done", interfaces: [], error: null });
  });

  it("does not start a second search while one is in flight", async () => {
    let release: (response: BusDiscoverResponse) => void = () => {};
    apiMock.discoverBusInterfaces.mockReturnValueOnce(
      new Promise<BusDiscoverResponse>((resolve) => {
        release = resolve;
      }),
    );

    const first = searchBusInterfaces();
    await searchBusInterfaces();
    expect(readBusDiscoveryForTests().phase).toBe("searching");
    expect(apiMock.discoverBusInterfaces).toHaveBeenCalledTimes(1);

    release({ interfaces: [hallway] });
    await first;
    expect(readBusDiscoveryForTests().interfaces).toEqual([hallway]);
  });

  it("keeps the interfaces it already knows when a later search cannot run", async () => {
    apiMock.discoverBusInterfaces.mockResolvedValueOnce({ interfaces: [hallway] });
    await searchBusInterfaces();

    apiMock.discoverBusInterfaces.mockRejectedValueOnce(new Error("multicast went nowhere"));
    await searchBusInterfaces();

    const state = readBusDiscoveryForTests();
    expect(state.phase).toBe("failed");
    expect(state.error).toBe("multicast went nowhere");
    // An interface that answered a minute ago is still the best address
    // this application knows; a search that could not run says nothing
    // about whether that address still works.
    expect(state.interfaces).toEqual([hallway]);
  });

  it("never rejects, whatever the request does", async () => {
    apiMock.discoverBusInterfaces.mockRejectedValue(new Error("boom"));
    await expect(searchBusInterfaces()).resolves.toBeUndefined();
  });

  it("runs the unasked-for search once, however many mounts ask for it", async () => {
    ensureBusDiscovery();
    ensureBusDiscovery();
    await Promise.resolve();
    ensureBusDiscovery();
    expect(apiMock.discoverBusInterfaces).toHaveBeenCalledTimes(1);
  });
});
