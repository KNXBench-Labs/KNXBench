/** System-2 lifecycle across a Download-tab unmount, a deferred and a failed reload. */
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const api = vi.hoisted(() => ({ deviceCatalog: vi.fn(), compareDevice: vi.fn(), currentProject: vi.fn() }));
const s2 = vi.hoisted(() => ({ getSystem2Status: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: (e: unknown) => String(e) }));
vi.mock("./system2DownloadApi", async (importOriginal) => ({ ...await importOriginal<typeof import("./system2DownloadApi")>(), ...s2 }));
vi.mock("./useProgrammingConsent", () => ({ useProgrammingConsent: () => ({ request: vi.fn(), dialog: null }) }));
import DevicesWorkspace from "./DevicesWorkspace";
import System2DownloadPanel from "./System2DownloadPanel";
import System2Lifecycle from "./System2Lifecycle";
import type { ProjectTree } from "./bindings/ProjectTree";
import { resetSystem2StatusForTests, getSystem2Snapshot } from "./system2Status";
import { resetSettingsForTests } from "./settingsStore";
import { status } from "./system2DownloadTestFixtures";
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const tree = (marks: boolean, revision = 1): ProjectTree => ({ schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel", server_incarnation: "server", project_incarnation: 1, snapshot_revision: revision,
  installations: [{ id: 1, name: "House", topology: [], buildings: [], group_addresses: [], group_ranges: [], unassigned: [
    { id: 10, name: "Configured button", address: "1.1.67", description: null, com_object_count: 0,
      pending_programming: { address: false, application: marks, parameters: marks, group_addresses: marks } }] }] });
const base = { ...status, schemaVersion: 4 as const, address: "1.1.67", deviceName: "Configured button" };
const running = { ...base, state: "running" as const, downloadResult: "inProgress" as const };
const failed = { ...base, state: "failed" as const, downloadResult: "transferCompleteVerificationUnfinished" as const, mutationMayHaveStarted: true,
  completedSteps: 26, totalSteps: 27, downloadCompletedSteps: 26, downloadTotalSteps: 27, projectSettlement: "persisted" as const };

let host: HTMLDivElement; let root: Root;
const onTreeUpdate = vi.fn();
const flush = async () => act(async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); });
const tick = async (ms: number) => act(async () => { await vi.advanceTimersByTimeAsync(ms); });
/** The real parent shape: lifecycle owner at the root, a Download panel that a tab switch unmounts, the table. */
function Parent({ initial }: { initial: ProjectTree }) {
  const [panel, setPanel] = useState(true);
  const [current, setCurrent] = useState(initial);
  onTreeUpdate.mockImplementation((next: ProjectTree) => setCurrent(next));
  return <>
    <System2Lifecycle onTreeUpdate={(next) => onTreeUpdate(next)} />
    <button data-switch onClick={() => setPanel((v) => !v)}>switch</button>
    {panel && <System2DownloadPanel project={current} deviceId={10} gateway="192.0.2.1:3671" enabled ordinaryBusy={false} />}
    <DevicesWorkspace tree={current} active selection={null} multiSelection={null} onItemClick={vi.fn()} onCatalogue={vi.fn()} />
  </>;
}
const markers = () => [...host.querySelectorAll(".programming-system2[data-system2-result]")];
const belief = () => host.querySelector("[data-system2-belief]")?.getAttribute("data-system2-belief") ?? null;
const chips = () => host.querySelectorAll(".programming-chip").length;
async function mount() {
  await act(async () => root.render(<Parent initial={tree(false)} />)); await flush();
}
beforeEach(() => {
  vi.useFakeTimers(); window.localStorage.clear(); resetSettingsForTests(); resetSystem2StatusForTests();
  api.deviceCatalog.mockReset(); api.deviceCatalog.mockResolvedValue({ schemaVersion: 1, serverIncarnation: "server", snapshotRevision: 1, devices: [] });
  api.currentProject.mockReset(); onTreeUpdate.mockReset(); s2.getSystem2Status.mockReset();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.useRealTimers(); });

it("sees the terminal end after the Download tab unmounted, shows the pending belief, then the acknowledged reload", async () => {
  s2.getSystem2Status.mockResolvedValue(running);
  await mount();
  expect(markers()).toHaveLength(0);
  await act(async () => host.querySelector<HTMLButtonElement>("[data-switch]")!.click()); await flush();
  expect(host.querySelector("[data-system2-status]")).toBeNull();
  // The run ends while only the table is visible: the application owner's poll notices.
  s2.getSystem2Status.mockResolvedValue(failed);
  let release!: (tree: unknown) => void;
  api.currentProject.mockReturnValue(new Promise((resolve) => { release = resolve; }));
  await tick(1000); await flush();
  expect(belief()).toBe("refreshing");
  expect(markers()).toHaveLength(1);
  expect(api.currentProject).toHaveBeenCalledTimes(1);
  expect(chips()).toBe(0);
  await act(async () => { release(tree(true, 2)); await Promise.resolve(); }); await flush();
  expect(onTreeUpdate).toHaveBeenCalledTimes(1);
  expect(belief()).toBeNull();
  expect(chips()).toBe(3);
  // Terminal now: no more polling.
  const reads = s2.getSystem2Status.mock.calls.length;
  await tick(5000); expect(s2.getSystem2Status.mock.calls.length).toBe(reads);
});
it("a failed reload stays explicitly stale with a visible retry, and one explicit retry acknowledges it", async () => {
  s2.getSystem2Status.mockResolvedValue(failed);
  api.currentProject.mockRejectedValueOnce(new Error("offline"));
  await mount(); await flush();
  expect(belief()).toBe("stale");
  await tick(3000); await flush();
  expect(api.currentProject).toHaveBeenCalledTimes(1);
  api.currentProject.mockResolvedValue(tree(true, 2));
  await act(async () => host.querySelector<HTMLButtonElement>("[data-system2-belief-retry]")!.click()); await flush();
  expect(api.currentProject).toHaveBeenCalledTimes(2);
  expect(belief()).toBeNull(); expect(onTreeUpdate).toHaveBeenCalledTimes(1);
});
it("drops a late reload answer that a newer end superseded", async () => {
  s2.getSystem2Status.mockResolvedValue(running);
  await mount();
  const answers: ((tree: unknown) => void)[] = [];
  api.currentProject.mockImplementation(() => new Promise((resolve) => { answers.push(resolve); }));
  s2.getSystem2Status.mockResolvedValue(failed);
  await tick(1000); await flush();
  expect(answers).toHaveLength(1);
  // A recovery end of the same operation supersedes the pending refresh (panel poll publishes it).
  s2.getSystem2Status.mockResolvedValue({ ...failed, state: "restored", restoreAttempted: true });
  await act(async () => host.querySelector<HTMLButtonElement>("[data-system2-refresh]")!.click()); await flush();
  expect(answers).toHaveLength(2);
  await act(async () => { answers[0](tree(true, 2)); await Promise.resolve(); }); await flush();
  expect(onTreeUpdate).not.toHaveBeenCalled();
  await act(async () => { answers[1](tree(true, 3)); await Promise.resolve(); }); await flush();
  expect(onTreeUpdate).toHaveBeenCalledTimes(1);
  expect(onTreeUpdate.mock.calls[0][0].snapshot_revision).toBe(3);
});
it("an old verified success is withdrawn in the table when the status authority disappears", async () => {
  s2.getSystem2Status.mockResolvedValue({ ...failed, state: "finished", downloadResult: "verified", projectSettlement: "noMutation" });
  await mount(); await flush();
  expect(markers()[0].getAttribute("data-system2-result")).toBe("verified");
  s2.getSystem2Status.mockResolvedValue(null);
  await act(async () => host.querySelector<HTMLButtonElement>("[data-system2-refresh]")!.click()); await flush();
  expect(markers()[0].getAttribute("data-system2-result")).toBe("statusUnavailable");
});
it("never polls while nothing is known to be active", async () => {
  s2.getSystem2Status.mockResolvedValue(null);
  await mount(); const reads = s2.getSystem2Status.mock.calls.length;
  await act(async () => host.querySelector<HTMLButtonElement>("[data-switch]")!.click());
  await tick(10_000);
  expect(s2.getSystem2Status.mock.calls.length).toBe(reads);
  expect(getSystem2Snapshot().belief).toBe("current");
});
