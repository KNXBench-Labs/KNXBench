/** Batch review: reasons, deselection, acknowledgements, batch consent, start and polling. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type * as Api from "./api";

const locale = vi.hoisted(() => ({ value: "en" as "en" | "de" }));
vi.mock("./i18n", async (importOriginal) => {
  const original = await importOriginal<typeof import("./i18n")>();
  const translators = { en: original.translateFor.bind(null, "en"), de: original.translateFor.bind(null, "de") };
  return { ...original, useTranslate: () => translators[locale.value] };
});
const consentMock = vi.hoisted(() => ({ requestBatch: vi.fn(), request: vi.fn() }));
vi.mock("./useProgrammingConsent", () => ({
  useProgrammingConsent: () => ({ request: consentMock.request, requestBatch: consentMock.requestBatch, dialog: null }),
}));
const apiMock = vi.hoisted(() => ({
  planDeviceDownloadBatch: vi.fn(),
  startDeviceDownloadBatch: vi.fn(),
  pollDeviceDownloadBatch: vi.fn(),
  stopDeviceDownloadBatch: vi.fn(),
  currentProject: vi.fn(),
  errorStatus: (error: unknown) => (error as { status?: number }).status,
  errorMessage: (error: unknown) => (error as Error).message,
}));
vi.mock("./api", () => apiMock);

import DeviceDownloadBatch from "./DeviceDownloadBatch";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

const PROJECT = { snapshot_revision: 1, server_incarnation: "batch-test", project_incarnation: 1 } as unknown as ProjectTree;

const REVIEW: Api.DeviceDownloadBatchReview = {
  batchId: 40,
  devices: [
    { deviceId: 1, address: null, name: "Loose actuator", status: "notInBatch", reason: { code: "noAddress" } },
    { deviceId: 2, address: "1.1.4", name: "Address pending", status: "notInBatch", reason: { code: "addressPending" } },
    { deviceId: 3, address: "1.1.8", name: "Dimmer", status: "inBatch", planId: 41, programId: "M-0001_A-1", scope: "complete",
      steps: 12, dataOctets: 40, support: "verified", inferences: [], warnings: [],
      confirmationPhrase: "I confirm download to 1.1.8", untestedAcknowledgement: null },
    { deviceId: 4, address: "1.1.9", name: "Blind", status: "inBatch", planId: 42, programId: "M-0001_A-2", scope: "parameters",
      steps: 8, dataOctets: 4, support: "untested", inferences: [], warnings: [],
      confirmationPhrase: "I confirm download to 1.1.9", untestedAcknowledgement: "I accept an untested download to 1.1.9" },
    { deviceId: 5, address: "1.1.10", name: "Old program", status: "notInBatch",
      reason: { code: "unsupported", category: "modules", detail: "parameter in a module instance" } },
  ],
};

const notFound = Object.assign(new Error("no batch download yet"), { status: 404 });

let host: HTMLDivElement; let root: Root;
beforeEach(() => {
  locale.value = "en";
  vi.useFakeTimers({ shouldAdvanceTime: true });
  apiMock.pollDeviceDownloadBatch.mockRejectedValue(notFound);
  apiMock.planDeviceDownloadBatch.mockResolvedValue(REVIEW);
  consentMock.requestBatch.mockResolvedValue(true);
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => {
  await act(async () => root.unmount()); host.remove();
  vi.useRealTimers(); vi.clearAllMocks();
});

async function render(props: Partial<Parameters<typeof DeviceDownloadBatch>[0]> = {}) {
  await act(async () => root.render(<DeviceDownloadBatch project={PROJECT} gateway="192.0.2.10:3671" disabled={false}
    openRequest={null} {...props} />));
}
const q = <T extends Element = HTMLElement>(selector: string) => host.querySelector<T>(selector);
const startButton = () => q<HTMLButtonElement>("[data-batch-start]")!;
async function openReview() {
  await act(async () => q<HTMLButtonElement>("[data-batch-open]")!.click());
}
async function type(input: HTMLInputElement, value: string) {
  await act(async () => {
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    setter.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

it("plans nothing until asked, then lists every pending device with its reason", async () => {
  await render();
  expect(apiMock.planDeviceDownloadBatch).not.toHaveBeenCalled();
  expect(q("[data-batch-open]")!.textContent).toBe(en["deviceDownload.batch.open"]);
  await openReview();
  expect(apiMock.planDeviceDownloadBatch).toHaveBeenCalledTimes(1);
  const rows = [...host.querySelectorAll<HTMLElement>("[data-batch-review] [data-batch-device]")];
  expect(rows.map((row) => row.dataset.batchStatus)).toEqual(["notInBatch", "notInBatch", "inBatch", "inBatch", "notInBatch"]);
  expect(rows[0].textContent).toContain(en["deviceDownload.batch.reason.noAddress"]);
  expect(rows[1].textContent).toContain(en["deviceDownload.batch.reason.addressPending"]);
  expect(rows[4].textContent).toContain("not supported: parameter in a module instance");
  expect(rows[0].querySelector("input[type=checkbox]")).toBeNull();
  expect(apiMock.startDeviceDownloadBatch).not.toHaveBeenCalled();
});

it("keeps start disabled until every selected untested device is acknowledged, or deselected", async () => {
  await render();
  await openReview();
  expect(startButton().disabled).toBe(true);
  expect(q("[data-batch-blocked]")!.textContent).toBe(en["deviceDownload.batch.blocked.acknowledgement"]);
  const ack = q<HTMLInputElement>("[data-batch-device='1.1.9'] [data-batch-ack]")!;
  await type(ack, "I accept an untested download to 1.1.8");
  expect(startButton().disabled).toBe(true);
  await type(ack, "I accept an untested download to 1.1.9");
  expect(startButton().disabled).toBe(false);
  expect(startButton().textContent).toBe(en["deviceDownload.batch.start.other"].replace("{count}", "2"));
  // Deselecting the untested device also unblocks, and hides its prompt.
  await type(ack, "");
  await act(async () => q<HTMLInputElement>("[data-batch-device='1.1.9'] input[type=checkbox]")!.click());
  expect(q("[data-batch-device='1.1.9'] [data-batch-ack]")).toBeNull();
  expect(startButton().disabled).toBe(false);
  expect(startButton().textContent).toBe(en["deviceDownload.batch.start.one"].replace("{count}", "1"));
  // Deselecting every device blocks again.
  await act(async () => q<HTMLInputElement>("[data-batch-device='1.1.8'] input[type=checkbox]")!.click());
  expect(startButton().disabled).toBe(true);
  expect(q("[data-batch-blocked]")!.textContent).toBe(en["deviceDownload.batch.blocked.none"]);
});

it("needs a gateway", async () => {
  await render({ gateway: " " });
  await openReview();
  await type(q<HTMLInputElement>("[data-batch-ack]")!, "I accept an untested download to 1.1.9");
  expect(startButton().disabled).toBe(true);
  expect(q("[data-batch-blocked]")!.textContent).toBe(en["deviceDownload.batch.blocked.gateway"]);
});

it("asks the batch question with the selected devices and sends nothing on a no", async () => {
  consentMock.requestBatch.mockResolvedValue(false);
  await render();
  await openReview();
  await type(q<HTMLInputElement>("[data-batch-ack]")!, "I accept an untested download to 1.1.9");
  await act(async () => startButton().click());
  expect(consentMock.requestBatch).toHaveBeenCalledWith(["1.1.8 — Dimmer", "1.1.9 — Blind"]);
  expect(consentMock.request).not.toHaveBeenCalled();
  expect(apiMock.startDeviceDownloadBatch).not.toHaveBeenCalled();
});

it("starts exactly the selected devices with their phrases and polls a result per device", async () => {
  const onBusyChange = vi.fn();
  const onProjectChanged = vi.fn();
  apiMock.startDeviceDownloadBatch.mockResolvedValue({ batchId: 40 });
  const running: Api.DeviceDownloadBatchStatus = { batchId: 40, state: "running", stopRequested: false, devices: [
    { deviceId: 3, address: "1.1.8", name: "Dimmer", state: "running", downloadId: 41 },
    { deviceId: 4, address: "1.1.9", name: "Blind", state: "waiting", downloadId: 42 },
    { deviceId: 2, address: "1.1.4", name: "Address pending", state: "notInBatch", reason: { code: "addressPending" } },
  ] };
  const ended: Api.DeviceDownloadBatchStatus = { ...running, state: "ended", devices: [
    { deviceId: 3, address: "1.1.8", name: "Dimmer", state: "unconfirmed", downloadId: 41, doubt: "restartUnconfirmed",
      written: "yes", restart: "unconfirmed", restartNote: null },
    { deviceId: 4, address: "1.1.9", name: "Blind", state: "notStarted", downloadId: 42, reason: "earlierDevice" },
    running.devices[2],
  ] };
  apiMock.currentProject.mockResolvedValue(PROJECT);
  await render({ onBusyChange, onProjectChanged });
  await openReview();
  await type(q<HTMLInputElement>("[data-batch-ack]")!, "I accept an untested download to 1.1.9");
  apiMock.pollDeviceDownloadBatch.mockResolvedValueOnce(running).mockResolvedValue(ended);
  await act(async () => startButton().click());
  expect(apiMock.startDeviceDownloadBatch).toHaveBeenCalledWith(40, "192.0.2.10:3671", [
    { address: "1.1.8", confirmation: "I confirm download to 1.1.8" },
    { address: "1.1.9", confirmation: "I confirm download to 1.1.9", acceptUntested: "I accept an untested download to 1.1.9" },
  ]);
  expect(q("[data-batch-run]")!.dataset.batchRun).toBe("running");
  expect(q("[data-batch-run] [role=status]")!.textContent)
    .toContain(en["deviceDownload.batch.running"].replace("{position}", "1").replace("{total}", "2"));
  expect(onBusyChange).toHaveBeenLastCalledWith(true);
  await act(async () => { await vi.advanceTimersByTimeAsync(600); });
  expect(q("[data-batch-run]")!.dataset.batchRun).toBe("ended");
  const states = [...host.querySelectorAll<HTMLElement>("[data-batch-run] [data-batch-device]")];
  expect(states.map((row) => row.dataset.batchState)).toEqual(["unconfirmed", "notStarted", "notInBatch"]);
  expect(states[0].textContent).toContain(en["deviceDownload.batch.state.restartUnconfirmed"]);
  expect(states[1].textContent).toContain(en["deviceDownload.batch.notStarted.earlierDevice"]);
  expect(onProjectChanged).toHaveBeenCalledWith(PROJECT);
  expect(onBusyChange).toHaveBeenLastCalledWith(false);
});

it("stop after this device asks the server once", async () => {
  const running: Api.DeviceDownloadBatchStatus = { batchId: 40, state: "running", stopRequested: false, devices: [
    { deviceId: 3, address: "1.1.8", name: "Dimmer", state: "running", downloadId: 41 },
  ] };
  apiMock.pollDeviceDownloadBatch.mockResolvedValueOnce(running).mockResolvedValue({ ...running, stopRequested: true });
  apiMock.stopDeviceDownloadBatch.mockResolvedValue({ stop: "afterThisDevice" });
  await render();
  const stop = q<HTMLButtonElement>("[data-batch-stop]")!;
  expect(stop.textContent).toBe(en["deviceDownload.batch.stop"]);
  await act(async () => stop.click());
  expect(apiMock.stopDeviceDownloadBatch).toHaveBeenCalledWith(40);
  expect(q<HTMLButtonElement>("[data-batch-stop]")!.disabled).toBe(true);
  expect(q("[data-batch-run] [role=status]")!.textContent).toContain(en["deviceDownload.batch.stopRequested"]);
});

it("a pending-block request waits while another download is busy, then opens the review once", async () => {
  const onOpenHandled = vi.fn();
  await render({ openRequest: 3, onOpenHandled, disabled: true });
  expect(onOpenHandled).not.toHaveBeenCalled();
  expect(apiMock.planDeviceDownloadBatch).not.toHaveBeenCalled();
  await render({ openRequest: 3, onOpenHandled });
  expect(onOpenHandled).toHaveBeenCalledWith(3);
  expect(apiMock.planDeviceDownloadBatch).toHaveBeenCalledTimes(1);
  await render({ openRequest: 3, onOpenHandled });
  expect(apiMock.planDeviceDownloadBatch).toHaveBeenCalledTimes(1);
});

it("speaks German", async () => {
  locale.value = "de";
  await render();
  expect(q("[data-batch-open]")!.textContent).toBe(de["deviceDownload.batch.open"]);
  await openReview();
  expect(host.textContent).toContain(de["deviceDownload.batch.reason.addressPending"]);
  expect(startButton().textContent).toBe(de["deviceDownload.batch.start.other"].replace("{count}", "2"));
});

it("keeps the review through a reload of the same project state and drops it on an edit", async () => {
  await render();
  await openReview();
  expect(q("[data-batch-review]")).not.toBeNull();
  await render({ project: { ...PROJECT } as ProjectTree });
  expect(q("[data-batch-review]")).not.toBeNull();
  await render({ project: { ...PROJECT, snapshot_revision: 2 } as unknown as ProjectTree });
  expect(q("[data-batch-review]")).toBeNull();
});
