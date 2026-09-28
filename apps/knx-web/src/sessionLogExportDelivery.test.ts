/** Delivery tests: browser Blob export and native dialog command share one JSON payload. */
// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LogEntry } from "./api";

const mocks = vi.hoisted(() => ({ native: vi.fn(() => false), invoke: vi.fn().mockResolvedValue(true) }));
vi.mock("./filePicker", () => ({ isTauri: mocks.native }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
import { saveSessionLog } from "./sessionLogExport";

const entry: LogEntry = {
  timestamp: "2026-09-10T12:00:00Z", severity: "info", source: "save",
  message: "=user text", location: null, detail: "Grüße\nNext line",
};

afterEach(() => { vi.restoreAllMocks(); mocks.native.mockReturnValue(false); mocks.invoke.mockClear(); });

describe("session log file delivery", () => {
  it("uses the native save dialog command with JSON content and no user-controlled path from JS", async () => {
    mocks.native.mockReturnValue(true);
    expect(await saveSessionLog([entry], [entry], "filtered")).toBe(true);
    expect(mocks.invoke).toHaveBeenCalledWith("save_session_log", { contents: expect.any(String) });
    const payload = mocks.invoke.mock.calls[0][1];
    expect(Object.keys(payload)).toEqual(["contents"]);
    expect(JSON.parse(payload.contents).entries).toEqual([entry]);
  });

  it("downloads UTF-8 JSON locally in the browser without a server-side path", async () => {
    const create = vi.fn((_blob: Blob) => "blob:session-log");
    const revoke = vi.fn();
    Object.defineProperty(URL, "createObjectURL", { configurable: true, value: create });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: revoke });
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
      expect(this.download).toBe("session-log.json");
      expect(this.href).toBe("blob:session-log");
    });
    expect(await saveSessionLog([entry], [entry], "all")).toBe(true);
    expect(create).toHaveBeenCalledOnce();
    expect(create.mock.calls[0][0]).toBeInstanceOf(Blob);
    expect(click).toHaveBeenCalledOnce();
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(document.querySelector('a[download="session-log.json"]')).toBeNull();
  });
});
