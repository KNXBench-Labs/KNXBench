/** Browser and native delivery of the same loss-aware bus-monitor capture. */
// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import type { BusTelegramRow } from "./api";

const mocks = vi.hoisted(() => ({ native: vi.fn(() => false), invoke: vi.fn().mockResolvedValue(true) }));
vi.mock("./filePicker", () => ({ isTauri: mocks.native }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
import { saveBusCapture } from "./busMonitorCapture";

const row: BusTelegramRow = {
  seq: 1, timestamp: "2026-09-29T01:00:00Z", source: "1.1.5", destination: "1/2/3",
  destinationName: "=HYPERLINK(\"x\")", service: "GroupValueWrite", rawPayload: "0x01 (6-bit)",
  decoded: { kind: "value", dpt: "DPST-1-1", text: "On" },
};
const provenance = {
  sessionId: 3, serverIncarnation: "test-process", status: "closed" as const,
  serverDroppedBefore: 2, clientPrunedCount: 0, exportedAt: "2026-09-29T01:10:00Z",
};

afterEach(() => {
  vi.restoreAllMocks();
  mocks.native.mockReturnValue(false);
  mocks.invoke.mockReset().mockResolvedValue(true);
});

describe("bus capture file delivery", () => {
  it("uses a native save dialog command with contents only, not a browser-selected server path", async () => {
    mocks.native.mockReturnValue(true);
    expect(await saveBusCapture([row], provenance)).toBe(true);
    expect(mocks.invoke).toHaveBeenCalledWith("save_bus_monitor_capture", { contents: expect.any(String) });
    const payload = mocks.invoke.mock.calls[0][1];
    expect(Object.keys(payload)).toEqual(["contents"]);
    expect(JSON.parse(payload.contents).rows).toEqual([row]);
    expect(JSON.parse(payload.contents).serverDroppedBefore).toBe(2);
  });

  it("propagates native cancellation and write failures without inventing success", async () => {
    mocks.native.mockReturnValue(true);
    mocks.invoke.mockResolvedValueOnce(false).mockRejectedValueOnce(new Error("disk full"));
    expect(await saveBusCapture([row], provenance)).toBe(false);
    await expect(saveBusCapture([row], provenance)).rejects.toThrow("disk full");
    expect(mocks.invoke).toHaveBeenCalledTimes(2);
  });

  it("refuses a capture larger than 16 MiB before opening a dialog or downloading", async () => {
    mocks.native.mockReturnValue(true);
    const oversized = { ...row, rawPayload: "x".repeat(16 * 1024 * 1024) };
    await expect(saveBusCapture([oversized], provenance)).rejects.toThrow(/16 MiB/);
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("downloads UTF-8 JSON locally in the browser without a server-side path", async () => {
    const create = vi.fn((_blob: Blob) => "blob:bus-capture");
    Object.defineProperty(URL, "createObjectURL", { configurable: true, value: create });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: vi.fn() });
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
      expect(this.download).toBe("bus-monitor-capture.json");
      expect(this.href).toBe("blob:bus-capture");
    });
    expect(await saveBusCapture([row], provenance)).toBe(true);
    expect(create).toHaveBeenCalledOnce();
    expect(create.mock.calls[0][0]).toBeInstanceOf(Blob);
    expect(click).toHaveBeenCalledOnce();
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(document.querySelector('a[download="bus-monitor-capture.json"]')).toBeNull();
  });
});
