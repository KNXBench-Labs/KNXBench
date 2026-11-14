/** Pins local transport review, frozen pending ownership and ordinary failed-read metadata. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import DeviceComparisonDialog from "./DeviceComparisonDialog";
import { useDeviceComparisons, type DeviceComparisons } from "./useDeviceComparisons";
import { ActivityController } from "./activityController";
import type { ProjectTree } from "./bindings/ProjectTree";
import { messages as en } from "./messages/en";
const api = vi.hoisted(() => ({ compareDevice: vi.fn(), compareDeviceTcp: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
vi.mock("./ComparisonAdoptionPanel", () => ({ default: () => null }));
const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel", installations: [], server_incarnation: "dialog",
  project_incarnation: 1, snapshot_revision: 1, comparison_revisions: { 1: 7 } };
const targets = [{ id: 1, name: "Fictional", address: "1.1.1" }];
let controller: DeviceComparisons;
let host: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
let activity: ActivityController;
function Harness() { controller = useDeviceComparisons(tree, activity); return <DeviceComparisonDialog controller={controller} />; }
async function click(name: string) {
  const button = [...host.querySelectorAll("button")].find((button) => button.textContent === name);
  expect(button).toBeDefined(); await act(async () => button!.click());
}
beforeEach(async () => { vi.resetAllMocks(); activity = new ActivityController(); host = document.createElement("div"); document.body.append(host);
  root = createRoot(host); await act(async () => root.render(<Harness />)); await act(async () => controller.open(targets)); });
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });
it("defaults each fresh review to UDP, performs no I/O on selection and freezes the running connection", async () => {
  const select = host.querySelector("select")!; expect(select.value).toBe("udp");
  await act(async () => { select.value = "tcp"; select.dispatchEvent(new Event("change", { bubbles: true })); });
  expect(api.compareDeviceTcp).not.toHaveBeenCalled(); expect(api.compareDevice).not.toHaveBeenCalled();
  await act(async () => controller.close()); await act(async () => controller.open(targets));
  expect(host.querySelector("select")!.value).toBe("udp");
  let reject!: (reason: Error) => void; api.compareDeviceTcp.mockReturnValue(new Promise((_, no) => { reject = no; }));
  let running!: Promise<void>;
  await act(async () => { running = controller.confirm("192.0.2.10:3671", "tcp"); });
  expect(host.querySelector("select")).toBeNull(); expect(host.querySelector(".comparison-gateway")).toBeNull();
  expect(host.textContent).toContain(en["compare.frozenConnection"].replace("{gateway}", "192.0.2.10:3671").replace("{transport}", "TCP"));
  await click(en["deviceChecks.cancel"]); expect(controller.running).toBe(true); expect(controller.stopping).toBe(true);
  const connection = { requestedTransport: "tcp", actualTransport: "tcp", assignedAddress: "1.1.250", skippedFramesBeforeDisconnect: 2 };
  await act(async () => { reject(Object.assign(new Error("Connection released"), { body: { kind: "deviceCompareNoAnswer", connection } })); await running; });
  expect(host.textContent).toContain(en["compare.actualConnection"].replace("{transport}", "TCP").replace("{address}", "1.1.250").replace("{skipped}", "2"));
  expect(host.textContent).toContain("Connection released"); expect(controller.entry(1)?.result).toBeNull();
  expect(api.compareDeviceTcp).toHaveBeenCalledTimes(1); expect(api.compareDevice).not.toHaveBeenCalled();
});
it("exposes unopened unavailable separately from an actual zero sample", async () => {
  api.compareDeviceTcp.mockRejectedValue(Object.assign(new Error("refused"), { body: { kind: "gatewayConnectFailed",
    connection: { requestedTransport: "tcp", actualTransport: null, assignedAddress: null, skippedFramesBeforeDisconnect: null } } }));
  await act(async () => controller.confirm("192.0.2.10:3671", "tcp"));
  expect(host.textContent).toContain(en["compare.connectionUnavailable"]);
  expect(host.textContent).not.toContain("1.1.250"); expect(controller.entry(1)?.connection?.skippedFramesBeforeDisconnect).toBeNull();
});
