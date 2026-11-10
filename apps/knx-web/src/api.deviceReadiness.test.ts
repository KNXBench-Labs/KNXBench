/** Pins optional device-id readiness selection without changing the legacy full-project read. */
import { afterEach, expect, it, vi } from "vitest";
import { getDeviceReadiness } from "./api";

afterEach(() => vi.unstubAllGlobals());

it("keeps the default full-project URL and sends an exact optional u32 selector", async () => {
  const body = { devices: [], counts: {} };
  const fetch = vi.fn().mockImplementation(async () => new Response(JSON.stringify(body), { status: 200 }));
  vi.stubGlobal("fetch", fetch);
  expect(await getDeviceReadiness()).toEqual(body);
  expect(await getDeviceReadiness(0)).toEqual(body);
  expect(await getDeviceReadiness(4294967295)).toEqual(body);
  expect(fetch.mock.calls.map(([url]) => url)).toEqual([
    "/api/device-readiness", "/api/device-readiness?deviceId=0", "/api/device-readiness?deviceId=4294967295",
  ]);
});

it("does not replace a missing-device refusal with a full-project request", async () => {
  const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify({ error: "device not found" }), { status: 404 }));
  vi.stubGlobal("fetch", fetch);
  await expect(getDeviceReadiness(99)).rejects.toMatchObject({ status: 404 });
  expect(fetch).toHaveBeenCalledTimes(1);
  expect(fetch.mock.calls[0][0]).toBe("/api/device-readiness?deviceId=99");
});
