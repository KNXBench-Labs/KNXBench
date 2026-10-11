/** Startup/background release checks are bounded, cancellable and notification-deduplicated. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { useReleaseUpdates } from "./useReleaseUpdates";
const mocks = vi.hoisted(() => ({ serverVersion: vi.fn(), fetchGitHubReleases: vi.fn(), enabled: true, hydration: "hydrated", acknowledged: true }));
vi.mock("./api", () => ({ serverVersion: mocks.serverVersion }));
vi.mock("./githubReleases", () => ({ fetchGitHubReleases: mocks.fetchGitHubReleases }));
vi.mock("./settingsStore", () => ({ useSettingsState: () => ({ hydration: mocks.hydration }), useSettingsRevision: () => 0, getSetting: () => mocks.enabled, getAcknowledgedSettings: () => ({ ok: mocks.acknowledged }) }));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
let root: Root | undefined, host: HTMLDivElement | undefined;
const notify = vi.fn();
let controls: ReturnType<typeof useReleaseUpdates>;
function Probe() { controls = useReleaseUpdates(notify); return null; }
async function mount() {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => { root!.render(<StrictMode><Probe /></StrictMode>); });
}
afterEach(async () => { if (root) await act(() => root!.unmount()); host?.remove(); root = undefined; host = undefined; vi.restoreAllMocks(); vi.resetAllMocks(); vi.useRealTimers(); mocks.enabled = true; mocks.hydration = "hydrated"; mocks.acknowledged = true; });
const rows = [{ tag_name: "v0.1.0-alpha.10", draft: false, prerelease: true, published_at: "2026-10-11T20:00:00Z" }];
it("checks the running version at startup and announces a newer publication exactly once under StrictMode", async () => {
  mocks.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.9+g1234567" });
  mocks.fetchGitHubReleases.mockResolvedValue(rows);
  await mount();
  expect(notify).toHaveBeenCalledTimes(1);
  expect(controls.status).toBe("available");
  await act(() => controls.check());
  expect(notify).toHaveBeenCalledTimes(1);
});

it("does no network work before settings hydration or when automatic checks are disabled", async () => {
 mocks.enabled = false; await mount(); expect(mocks.serverVersion).not.toHaveBeenCalled(); expect(mocks.fetchGitHubReleases).not.toHaveBeenCalled();
});
it("does no network work while settings are still cached", async () => {
 mocks.hydration = "cached"; await mount(); expect(mocks.serverVersion).not.toHaveBeenCalled();
});
it("rechecks after an hour, not on every focus, and announces each new version once", async () => {
 vi.useFakeTimers(); mocks.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.9" }); mocks.fetchGitHubReleases.mockResolvedValue(rows);
 await mount(); const count = mocks.fetchGitHubReleases.mock.calls.length;
 await act(() => { window.dispatchEvent(new Event("focus")); }); expect(mocks.fetchGitHubReleases).toHaveBeenCalledTimes(count);
 await act(() => vi.advanceTimersByTimeAsync(60 * 60 * 1000 - 1)); expect(mocks.fetchGitHubReleases).toHaveBeenCalledTimes(count);
 await act(() => vi.advanceTimersByTimeAsync(1)); expect(mocks.fetchGitHubReleases).toHaveBeenCalledTimes(count + 1); expect(notify).toHaveBeenCalledTimes(1);
 mocks.fetchGitHubReleases.mockResolvedValue([{ ...rows[0], tag_name: "v0.1.0-alpha.11" }]);
 await act(() => vi.advanceTimersByTimeAsync(60 * 60 * 1000)); expect(notify).toHaveBeenCalledTimes(2);
});
it("does not announce a reply arriving after unmount", async () => {
 mocks.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.9" });
 let resolve!: (rows: unknown[]) => void;
 mocks.fetchGitHubReleases.mockImplementation(() => new Promise(done => { resolve = done; }));
 await mount(); await act(() => root!.unmount()); root = undefined;
 await act(async () => resolve(rows)); expect(notify).not.toHaveBeenCalled();
});
it("cancels the active request when the user disables automatic checks", async () => {
 mocks.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.9" });
 let signal!: AbortSignal, resolve!: (rows: unknown[]) => void;
 mocks.fetchGitHubReleases.mockImplementation((owner: AbortSignal) => { signal = owner; return new Promise(done => { resolve = done; }); });
 await mount(); mocks.enabled = false; await act(() => root!.render(<StrictMode><Probe /></StrictMode>));
 expect(signal.aborted).toBe(true); expect(controls.status).not.toBe("checking"); await act(async () => resolve(rows)); expect(notify).not.toHaveBeenCalled();
});
it("keeps network errors separate from no-newer-release results and allows explicit retry", async () => {
 mocks.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.9" }); mocks.fetchGitHubReleases.mockRejectedValue(new Error("offline"));
 await mount(); expect(controls.status).toBe("unavailable"); expect(notify).not.toHaveBeenCalled();
 mocks.fetchGitHubReleases.mockResolvedValue([]); await act(() => controls.check()); expect(controls.status).toBe("current");
});
it("never guesses a running version from the frontend bundle", async () => {
 mocks.serverVersion.mockResolvedValue({ version: "unknown" }); await mount(); expect(controls.status).toBe("unavailable"); expect(mocks.fetchGitHubReleases).not.toHaveBeenCalled();
});
it("does not overlap explicit checks with a pending automatic request", async () => {
 mocks.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.9" });
 let resolve!: (rows: unknown[]) => void; mocks.fetchGitHubReleases.mockImplementation(() => new Promise(done => { resolve = done; }));
 await mount(); await act(() => controls.check()); expect(mocks.fetchGitHubReleases).toHaveBeenCalledTimes(1);
 await act(async () => resolve(rows)); expect(notify).toHaveBeenCalledTimes(1);
});
it("allows an explicit manual check even when automatic discovery is disabled", async () => {
 mocks.enabled = false; mocks.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.9" }); mocks.fetchGitHubReleases.mockResolvedValue(rows);
 await mount(); await act(() => controls.check()); expect(notify).toHaveBeenCalledTimes(1);
});

it("does not contact GitHub when the settings record is refused or unacknowledged", async () => {
 mocks.acknowledged = false; await mount(); expect(mocks.serverVersion).not.toHaveBeenCalled();
});

it("starts after acknowledgement becomes usable without needing another hydration transition", async () => {
 mocks.acknowledged = false; mocks.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.9" }); mocks.fetchGitHubReleases.mockResolvedValue(rows);
 await mount(); expect(mocks.fetchGitHubReleases).not.toHaveBeenCalled();
 mocks.acknowledged = true; await act(() => root!.render(<StrictMode><Probe /></StrictMode>));
 expect(notify).toHaveBeenCalledTimes(1);
});

it("skips hidden-page checks and checks a due release when the workbench becomes visible", async () => {
 vi.useFakeTimers(); mocks.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.9" }); mocks.fetchGitHubReleases.mockResolvedValue(rows);
 await mount(); const count = mocks.fetchGitHubReleases.mock.calls.length;
 const visibility = vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
 await act(() => vi.advanceTimersByTimeAsync(60 * 60 * 1000)); expect(mocks.fetchGitHubReleases).toHaveBeenCalledTimes(count);
 visibility.mockReturnValue("visible"); await act(async () => document.dispatchEvent(new Event("visibilitychange")));
 expect(mocks.fetchGitHubReleases).toHaveBeenCalledTimes(count + 1); expect(notify).toHaveBeenCalledTimes(1); visibility.mockRestore();
});
