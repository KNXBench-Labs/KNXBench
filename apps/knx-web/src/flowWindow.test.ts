/** Verifies dedicated window URLs, focus-only reopening and failure reporting. */
// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
const native = vi.hoisted(() => ({ existing: vi.fn(), created: vi.fn(), outcome: "tauri://created" }));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  WebviewWindow: class {
    static getByLabel = native.existing;
    constructor(label: string, options: unknown) { native.created(label, options); }
    once(event: string, handler: () => void) {
      if (event === native.outcome) handler();
      return Promise.resolve(() => {});
    }
  },
}));
import { flowWindowUrl, isFlowWindow, openFlowWindow, resetFlowWindowForTests } from "./flowWindow";
afterEach(() => { vi.restoreAllMocks(); vi.clearAllMocks(); resetFlowWindowForTests(); delete (window as unknown as Record<string, unknown>).__TAURI__; });
describe("dedicated flow windows", () => {
  it("selects only the Flow role and passes the exact source lifetime", () => {
    expect(isFlowWindow("?view=flow&source=owner-1")).toBe(true);
    expect(isFlowWindow("?view=diagnostics")).toBe(false);
    expect(flowWindowUrl("http://localhost:1420/?old=1#x", "owner-1")).toBe("http://localhost:1420/?view=flow&source=owner-1");
    expect(() => flowWindowUrl("http://localhost:1420/", "../bad")).toThrow();
  });
  it("creates the native read-only role and waits for the actual creation event", async () => {
    (window as unknown as Record<string, unknown>).__TAURI__ = {};
    native.existing.mockResolvedValue(null); native.outcome = "tauri://created";
    expect(await openFlowWindow(location.href, "owner-1")).toBe("opened");
    expect(native.created).toHaveBeenCalledWith("flow-owner-1", expect.objectContaining({ width: 1440, height: 900, url: flowWindowUrl(location.href, "owner-1") }));
  });
  it("focuses the existing native source window without creating a duplicate", async () => {
    (window as unknown as Record<string, unknown>).__TAURI__ = {};
    const focus = vi.fn().mockResolvedValue(undefined); native.existing.mockResolvedValue({ setFocus: focus });
    expect(await openFlowWindow(location.href, "owner-1")).toBe("focused");
    expect(focus).toHaveBeenCalledOnce(); expect(native.created).not.toHaveBeenCalled();
  });
  it("reports a native creation refusal, not an optimistic success", async () => {
    (window as unknown as Record<string, unknown>).__TAURI__ = {};
    native.existing.mockResolvedValue(null); native.outcome = "tauri://error";
    expect(await openFlowWindow(location.href, "owner-1")).toBe("failed");
  });
  it("reports popup blocking", async () => {
    vi.spyOn(window, "open").mockReturnValue(null);
    expect(await openFlowWindow(location.href, "owner-1")).toBe("blocked");
  });
  it("focuses a repeated open without navigating or reloading the window", async () => {
    const focus = vi.fn(); const fake = { closed: false, focus } as unknown as Window;
    const open = vi.spyOn(window, "open").mockReturnValue(fake);
    expect(await openFlowWindow(location.href, "owner-1")).toBe("opened");
    expect(await openFlowWindow(location.href, "owner-1")).toBe("focused");
    expect(open).toHaveBeenCalledTimes(1); expect(focus).toHaveBeenCalledTimes(2);
  });
});
