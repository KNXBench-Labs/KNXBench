/** Tests the strict TCP service-control client and its connection-facts admission. */
import { afterEach, describe, expect, it, vi } from "vitest";
import { readServiceControlTcp, ServiceControlTcpRefusal, writeServiceControlTcp } from "./serviceControlApi";

vi.mock("./api", () => ({ noteRefusal: vi.fn() }));
const binding = { serverIncarnation: "fixture", projectIncarnation: 3, revision: 1 };
const opened = { requestedTransport: "tcp", actualTransport: "tcp", assignedAddress: "1.1.250", skippedFramesBeforeDisconnect: 0 };
const unopened = { requestedTransport: "tcp", actualTransport: null, assignedAddress: null, skippedFramesBeforeDisconnect: null };
const reading = { address: "1.1.25", raw: "0000", mask: "07B0", individualAddressWriteEnabled: false };
const reply = (body: unknown, status = 200) => vi.fn().mockResolvedValue(new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } }));
async function refused(promise: Promise<unknown>): Promise<ServiceControlTcpRefusal> {
  try { await promise; } catch (e) { expect(e).toBeInstanceOf(ServiceControlTcpRefusal); return e as ServiceControlTcpRefusal; }
  throw new Error("expected a refusal");
}
afterEach(() => vi.unstubAllGlobals());

describe("explicit TCP service-control client", () => {
  it("calls only the strict sibling routes with the binding in the body", async () => {
    const fetch = reply({ ...reading, connection: opened }); vi.stubGlobal("fetch", fetch);
    await readServiceControlTcp("1.1.25", "192.0.2.10:3671", binding);
    expect(fetch.mock.calls[0][0]).toBe("/api/device/service-control/tcp/read");
    expect(JSON.parse(fetch.mock.calls[0][1].body)).toEqual({ address: "1.1.25", gateway: "192.0.2.10:3671", binding });
    const write = reply({ before: reading, after: "0004", individualAddressWriteEnabled: true, written: true, backupPath: "p", connection: opened }); vi.stubGlobal("fetch", write);
    await writeServiceControlTcp("1.1.25", "192.0.2.10:3671", true, "phrase", binding);
    expect(write.mock.calls[0][0]).toBe("/api/device/service-control/tcp");
    expect(JSON.parse(write.mock.calls[0][1].body)).toEqual({ address: "1.1.25", gateway: "192.0.2.10:3671", enable: true, confirmation: "phrase", binding });
  });
  it("admits zero skipped frames as a fact, distinct from unavailable", async () => {
    vi.stubGlobal("fetch", reply({ ...reading, connection: opened }));
    expect((await readServiceControlTcp("1.1.25", "g", binding)).connection.skippedFramesBeforeDisconnect).toBe(0);
  });
  it("refuses a success whose connection key is truly absent", async () => {
    vi.stubGlobal("fetch", reply(reading));
    expect((await refused(readServiceControlTcp("1.1.25", "g", binding))).status).toBe(0);
  });
  it.each([
    ["unavailable facts claimed as a completed read", unopened],
    ["wrong actual transport", { ...opened, actualTransport: "udp" }],
    ["wrong requested transport", { ...opened, requestedTransport: "udp" }],
    ["zero standing in for unavailable", { ...unopened, skippedFramesBeforeDisconnect: 0 }],
    ["opened without an address", { ...opened, assignedAddress: null }],
    ["negative skipped count", { ...opened, skippedFramesBeforeDisconnect: -1 }],
    ["array", []],
    ["null", null],
  ])("refuses %s on a 2xx", async (_name, connection) => {
    vi.stubGlobal("fetch", reply({ ...reading, connection }));
    expect((await refused(readServiceControlTcp("1.1.25", "g", binding))).status).toBe(0);
  });
  it("keeps valid failure facts and drops malformed ones; 404 stays a 404", async () => {
    vi.stubGlobal("fetch", reply({ error: "broken", connection: { ...opened, skippedFramesBeforeDisconnect: 4 } }, 502));
    const failure = await refused(readServiceControlTcp("1.1.25", "g", binding));
    expect([failure.status, failure.detail, failure.connection?.skippedFramesBeforeDisconnect]).toEqual([502, "broken", 4]);
    vi.stubGlobal("fetch", reply({ error: "broken", connection: { ...opened, actualTransport: "udp" } }, 502));
    expect((await refused(readServiceControlTcp("1.1.25", "g", binding))).connection).toBeNull();
    vi.stubGlobal("fetch", reply({ error: "unsupported" }, 404));
    expect((await refused(writeServiceControlTcp("1.1.25", "g", true, "p", binding))).status).toBe(404);
  });
});
