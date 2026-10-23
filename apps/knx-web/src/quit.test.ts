import { beforeEach, describe, expect, it, vi } from "vitest";

const native = vi.hoisted(() => ({
  close: vi.fn<() => Promise<void>>(),
  destroy: vi.fn<() => Promise<void>>(),
  handler: null as null | ((event: { preventDefault(): void }) => void),
}));
vi.mock("./filePicker", () => ({ isTauri: () => true }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    close: native.close,
    destroy: native.destroy,
    onCloseRequested: async (handler: typeof native.handler) => {
      native.handler = handler;
      return () => { native.handler = null; };
    },
  }),
}));

import { onWindowCloseRequested, quitApp } from "./quit";

function requestNativeClose(): boolean {
  const preventDefault = vi.fn();
  native.handler?.({ preventDefault });
  return preventDefault.mock.calls.length === 0;
}

beforeEach(() => {
  native.close.mockReset().mockImplementation(async () => { requestNativeClose(); });
  native.destroy.mockReset().mockResolvedValue(undefined);
  native.handler = null;
});

describe("desktop quit close request", () => {
  it("routes an approved quit through close without repeating the unsaved prompt", async () => {
    const shouldClose = vi.fn(() => false);
    await onWindowCloseRequested(shouldClose);
    await quitApp();
    expect(native.close).toHaveBeenCalledOnce();
    expect(native.destroy).not.toHaveBeenCalled();
    expect(shouldClose).not.toHaveBeenCalled();
    // Approval is consumed once; an unrelated later window-manager close asks again.
    expect(requestNativeClose()).toBe(false);
    expect(shouldClose).toHaveBeenCalledOnce();
  });

  it("keeps the unsaved-check refusal for an ordinary window close", async () => {
    await onWindowCloseRequested(() => false);
    expect(requestNativeClose()).toBe(false);
    expect(native.destroy).not.toHaveBeenCalled();
  });

  it("clears approval after a failed native close request", async () => {
    await onWindowCloseRequested(() => false);
    native.close.mockRejectedValueOnce(new Error("native close failed"));
    await expect(quitApp()).rejects.toThrow("native close failed");
    expect(requestNativeClose()).toBe(false);
  });
});
