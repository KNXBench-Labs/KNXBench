/** Pins the status-bar progress of a running device download (issue #43). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ActivitySnapshot, SessionActivity } from "./liveActivity";

const apiMock = vi.hoisted(() => ({ busActivity: vi.fn() }));
vi.mock("./api", async (importOriginal) => ({ ...(await importOriginal<typeof import("./api")>()), ...apiMock }));

import DownloadFooterProgress, { dataPercent } from "./DownloadFooterProgress";
import { refreshBusConnection, resetBusConnectionForTests } from "./busConnection";
import { resetSettingsForTests } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const snapshot = (sessions: SessionActivity[], busyLocks: ActivitySnapshot["busyLocks"] = []): ActivitySnapshot => ({
  serverIncarnation: "process-a", coverage: "partial", historyState: "disabled", sessions, oneShot: [],
  oneShotDropped: 0, busyLocks, untracked: ["groupWrite", "serialAddress"],
});
const download = (state: "running" | "finished" | "failed", completedSteps = 5, writtenOctets = 524): SessionActivity =>
  ({ kind: "deviceDownload", id: 3, address: "1.1.67", state, completedSteps, totalSteps: 14, writtenOctets, totalOctets: 1457 });

let host: HTMLDivElement;
let root: Root;
const onOpen = vi.fn();
const flush = () => act(async () => { for (let i = 0; i < 6; i += 1) await Promise.resolve(); });

async function show(value: ActivitySnapshot) {
  apiMock.busActivity.mockResolvedValue(value);
  await act(async () => { await refreshBusConnection(); });
  await flush();
}

beforeEach(async () => {
  resetBusConnectionForTests();
  resetSettingsForTests();
  localStorage.clear();
  apiMock.busActivity.mockReset();
  onOpen.mockReset();
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root.render(<DownloadFooterProgress onOpen={onOpen} />));
});
afterEach(() => { act(() => root.unmount()); host.remove(); resetBusConnectionForTests(); });

describe("download progress in the status bar", () => {
  it("names device, step and percent of octets while a download runs, and opens its panel", async () => {
    await show(snapshot([download("running")], ["deviceDownload"]));
    // Finished steps and verified data, worded as such: never "step 5" while the panel runs step 6.
    expect(host.querySelector(".download-footer-progress .task-status-text")?.textContent)
      .toBe("Download to 1.1.67: 5 of 14 steps done · 35 % of data written");
    const bar = host.querySelector("progress")!;
    expect([bar.value, bar.max]).toEqual([524, 1457]);
    await act(async () => host.querySelector("button")!.click());
    expect(onOpen).toHaveBeenCalledOnce();
  });

  it("follows the server's counts from poll to poll", async () => {
    await show(snapshot([download("running", 5, 524)], ["deviceDownload"]));
    await show(snapshot([download("running", 13, 1457)], ["deviceDownload"]));
    expect(host.querySelector(".download-footer-progress .task-status-text")?.textContent)
      .toBe("Download to 1.1.67: 13 of 14 steps done · 100 % of data written");
  });

  it("shows nothing when no download runs: finished, failed, idle or another bus user", async () => {
    for (const value of [snapshot([download("finished")]), snapshot([download("failed")]), snapshot([]),
      snapshot([{ kind: "lineScan", id: 4, state: "running", probed: 1, total: 9 }], ["lineScan"])]) {
      await show(value);
      expect(host.querySelector(".download-footer-progress")).toBeNull();
    }
  });

  it("names no download from a held lock alone, without a visible session", async () => {
    // A held lock without a visible session is not a download the footer can name.
    await show(snapshot([], ["deviceDownload"]));
    expect(host.querySelector(".download-footer-progress")).toBeNull();
  });

  it("announces start and end once, not every poll", async () => {
    const live = () => host.querySelector("[aria-live=polite]")?.textContent;
    await show(snapshot([download("running", 5, 524)], ["deviceDownload"]));
    expect(live()).toBe("Download to 1.1.67 started.");
    await show(snapshot([download("running", 9, 1100)], ["deviceDownload"]));
    expect(live()).toBe("Download to 1.1.67 started.");
    await show(snapshot([download("finished", 14, 1457)]));
    expect(live()).toBe("Download to 1.1.67 no longer running; see the Download to device tab for its result.");
  });

  it("announces nothing while the run's state is unknown: hidden window, failed read, own lock alone", async () => {
    const live = () => host.querySelector("[aria-live=polite]")?.textContent;
    await show(snapshot([download("running", 5, 524)], ["deviceDownload"]));
    expect(live()).toBe("Download to 1.1.67 started.");
    apiMock.busActivity.mockRejectedValueOnce(new Error("offline"));
    await act(async () => { await refreshBusConnection(); });
    await flush();
    expect(live()).toBe("Download to 1.1.67 started.");
    await show(snapshot([], ["deviceDownload"]));
    expect(live()).toBe("Download to 1.1.67 started.");
    Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "hidden" });
    await act(async () => { await refreshBusConnection(); });
    Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "visible" });
    await show(snapshot([download("running", 9, 1100)], ["deviceDownload"]));
    expect(live()).toBe("Download to 1.1.67 started.");
    expect(host.querySelector(".download-footer-progress")).not.toBeNull();
  });

  it("names only steps when a plan writes no data, and the percent never exceeds 100", async () => {
    expect(dataPercent({ completedSteps: 3, totalSteps: 4, writtenOctets: 0, totalOctets: 0 })).toBeNull();
    expect(dataPercent({ completedSteps: 1, totalSteps: 1, writtenOctets: 9, totalOctets: 8 })).toBe(100);
    await show(snapshot([{ ...download("running", 3, 0), totalOctets: 0 } as SessionActivity], ["deviceDownload"]));
    expect(host.querySelector(".download-footer-progress .task-status-text")?.textContent).toBe("Download to 1.1.67: 3 of 14 steps done");
    expect(host.querySelector("progress")).toBeNull();
  });

});
