/** Frozen Alpha-16 transport: no permissive or private-field fallbacks. */
import { afterEach, expect, it, vi } from "vitest";
import * as system2 from "./system2DownloadApi";
import { subscribeSessionExpired } from "./session";

import { preview, status } from "./system2DownloadTestFixtures";
function reply(value: unknown, code = 200) {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify(value), { status: code })));
}
afterEach(() => vi.unstubAllGlobals());
it("prepares only retained configured sources by stable device id", async () => {
  reply(preview); expect(await system2.prepareSystem2(10)).toEqual(preview);
  expect(fetch).toHaveBeenCalledWith("/api/device-system2/prepare", expect.objectContaining({ method: "POST", body: '{"deviceId":10,"sourceMode":"retainedSources"}' }));
});
it.each([
  { ...preview, backupPath: "/private" }, { ...preview, schemaVersion: 2 }, { ...preview, preparationId: "A".repeat(32) },
  { ...preview, binding: { ...preview.binding, projectRevision: "01" } },
  { ...preview, binding: { ...preview.binding, projectRevision: "18446744073709551616" } },
  { ...preview, address: "01.1.67" }, { ...preview, parameterValues: -1 }, { ...preview, steps: [42] },
  { ...preview, binding: { ...preview.binding, serial: "private" } },
])("refuses invalid preview %#", async (value) => {
  reply(value); await expect(system2.prepareSystem2(10)).rejects.toThrow("System-2");
});
it.each([{ ...status, state: "success" }, { ...status, completedSteps: 3, totalSteps: 2 }, { ...status, totalSteps: undefined }, { ...status, memory: [] }])("refuses invalid status %#", async (value) => {
  reply(value); await expect(system2.getSystem2Status()).rejects.toThrow("System-2");
});
it("preserves null status and all explicit mutation fields with no retries", async () => {
  reply(null); expect(await system2.getSystem2Status()).toBeNull();
  const start = { preparationId: preview.preparationId, gateway: "192.0.2.1:3671", confirmation: "a", hypothesisConfirmation: "b", downloadConfirmation: "c", releaseConfirmation: "d", recoveryTrack: "externalETS" as const };
  reply(status); await system2.startSystem2(start);
  expect(fetch).toHaveBeenCalledWith("/api/device-system2/start", expect.objectContaining({ body: JSON.stringify(start) }));
  reply(status); await system2.cancelSystem2(status.operationId);
  expect(fetch).toHaveBeenCalledWith("/api/device-system2/cancel", expect.objectContaining({ body: JSON.stringify({ operationId: status.operationId }) }));
  const recovery = { operationId: status.operationId, gateway: "192.0.2.1:3671", recoveryConfirmation: "original", downloadConfirmation: "download", releaseConfirmation: "release" };
  reply(status); await system2.recoverSystem2(recovery);
  expect(fetch).toHaveBeenCalledWith("/api/device-system2/recover", expect.objectContaining({ body: JSON.stringify(recovery) }));
  vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("lost")));
  await expect(system2.startSystem2(start)).rejects.toThrow("lost"); expect(fetch).toHaveBeenCalledTimes(1);
});
it("uses the existing session-expiry and typed HTTP error handling", async () => {
  const expired = vi.fn(); const stop = subscribeSessionExpired(expired);
  try { reply({ error: "Session required" }, 401); await expect(system2.getSystem2Status()).rejects.toMatchObject({ status: 401 }); expect(expired).toHaveBeenCalledOnce(); }
  finally { stop(); }
});
