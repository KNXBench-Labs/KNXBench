/** Hook-level tests: reads follow the preference only, never an unrelated settings revision. */
// @vitest-environment happy-dom
import { StrictMode, act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DeviceNode } from "./bindings/DeviceNode";
import { GA_LAST_DOWNLOAD_KEY, useDeviceLastDownloads } from "./deviceLastDownloads";
import { resetSettingsForTests, settingsRevision, settingsStorage } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";

const api = vi.hoisted(() => ({ deviceDetail: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: (error: unknown) => String(error) }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const devices = [{ id: 1 }] as DeviceNode[];
const snapshot = {};
let host: HTMLDivElement | undefined;
let root: Root | undefined;
let latest: ReturnType<typeof useDeviceLastDownloads> | undefined;

function Probe() { latest = useDeviceLastDownloads(devices, snapshot, "project"); return null; }
const mount = async (strict = false) => {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => { root!.render(strict ? <StrictMode><Probe /></StrictMode> : <Probe />); });
};

beforeEach(() => {
  resetSettingsForTests(); settingsStorage.setItem("uiLanguage", "en"); resetUiLanguageForTests();
  api.deviceDetail.mockImplementation(async (id: number) => ({ id, last_download: undefined }));
});
afterEach(() => { act(() => root?.unmount()); host?.remove(); host = undefined; root = undefined; latest = undefined; vi.clearAllMocks(); });

describe("useDeviceLastDownloads", () => {
  it("does not re-read when an unrelated settings revision is bumped with the preference on", async () => {
    settingsStorage.setItem(GA_LAST_DOWNLOAD_KEY, "true");
    await mount();
    expect(api.deviceDetail).toHaveBeenCalledTimes(1);
    const before = settingsRevision();
    await act(async () => { settingsStorage.setItem("unrelatedPreference", "x"); });
    expect(settingsRevision()).toBeGreaterThan(before);
    await act(async () => { settingsStorage.setItem(GA_LAST_DOWNLOAD_KEY, "true"); }); // same value again
    expect(api.deviceDetail).toHaveBeenCalledTimes(1);
    expect(latest?.current.has(1)).toBe(true);
  });

  it("issues one read per device under StrictMode's doubled effect", async () => {
    settingsStorage.setItem(GA_LAST_DOWNLOAD_KEY, "true");
    await mount(true);
    expect(api.deviceDetail).toHaveBeenCalledTimes(1);
    expect(latest?.current.has(1)).toBe(true);
  });

  it("reads nothing while the preference is off", async () => {
    await mount(true);
    expect(api.deviceDetail).not.toHaveBeenCalled();
  });
});
