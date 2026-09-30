/** Debug opt-in only reflects the persisted server settings, not the browser cache. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import ServiceControlDebugSetting from "./ServiceControlDebugSetting";
import { readPersistedBooleanSetting, setPersistedBooleanSetting } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";

vi.mock("./settingsStore", async (importOriginal) => ({
  ...await importOriginal<typeof import("./settingsStore")>(),
  readPersistedBooleanSetting: vi.fn(),
  setPersistedBooleanSetting: vi.fn(),
}));
const read = vi.mocked(readPersistedBooleanSetting);
const write = vi.mocked(setPersistedBooleanSetting);
let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  read.mockReset();
  write.mockReset();
  resetUiLanguageForTests();
});

async function mount() {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => root.render(<ServiceControlDebugSetting />));
  return root;
}

describe("service control Debug setting", () => {
  it("is off and unavailable until the server confirms the saved preference", async () => {
    let resolve!: (value: boolean) => void;
    read.mockReturnValue(new Promise<boolean>((ready) => { resolve = ready; }));
    const root = await mount();
    const checkbox = host!.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
    expect(checkbox.checked).toBe(false);
    expect(checkbox.disabled).toBe(true);
    await act(async () => resolve(true));
    expect(checkbox.checked).toBe(true);
    expect(checkbox.disabled).toBe(false);
    root.unmount();
  });

  it("does not claim opt-in when persistence fails, then permits an explicit retry", async () => {
    read.mockResolvedValue(false);
    write.mockRejectedValueOnce(new Error("409")).mockResolvedValueOnce(undefined);
    const root = await mount();
    const checkbox = host!.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
    await act(async () => { checkbox.click(); });
    expect(write).toHaveBeenCalledWith("debugIndividualAddressWriteEnable", true);
    expect(checkbox.checked).toBe(false);
    expect(checkbox.disabled).toBe(true);
    expect(host!.querySelector('[role="alert"]')?.textContent).toMatch(/could not|did not confirm/i);
    await act(async () => { host!.querySelector<HTMLButtonElement>(".settings-debug-retry")!.click(); });
    await act(async () => { checkbox.click(); });
    expect(checkbox.checked).toBe(true);
    root.unmount();
  });

  it("fails closed when the server settings cannot be read", async () => {
    read.mockRejectedValue(new Error("offline"));
    const root = await mount();
    expect(host!.querySelector<HTMLInputElement>('input[type="checkbox"]')?.disabled).toBe(true);
    expect(host!.querySelector('[role="alert"]')).not.toBeNull();
    expect(write).not.toHaveBeenCalled();
    root.unmount();
  });

  it("ignores an old read from StrictMode's first effect when a newer read says off", async () => {
    let resolveOld!: (value: boolean) => void;
    read.mockReturnValueOnce(new Promise<boolean>((ready) => { resolveOld = ready; }))
      .mockResolvedValueOnce(false);
    host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(<StrictMode><ServiceControlDebugSetting /></StrictMode>));
    expect(read).toHaveBeenCalledTimes(2);
    const checkbox = host!.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
    expect(checkbox.checked).toBe(false);
    expect(checkbox.disabled).toBe(false);
    await act(async () => resolveOld(true));
    expect(checkbox.checked).toBe(false);
    root.unmount();
  });
});
