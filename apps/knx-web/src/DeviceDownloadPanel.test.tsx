/** Verifies the device-download panel: plan first, consent before sending, every block shown. */
// @vitest-environment happy-dom

import { tableColumnSmoke } from "./tableColumnSmoke.fixture";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  planDeviceDownload: vi.fn(),
  startDeviceDownload: vi.fn(),
  pollDeviceDownload: vi.fn(),
  serverVersion: vi.fn(),
  getDeviceDownloadReport: vi.fn(),
  getDeviceReadiness: vi.fn(),
  currentProject: vi.fn(),
}));
const savedReports = vi.hoisted(() => vi.fn());
vi.mock("./localJsonDownload", () => ({ downloadLocalJson: savedReports }));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  errorStatus: (error: unknown) =>
    error && typeof error === "object" && "status" in error
      ? (error as { status: number }).status
      : undefined,
}));

import DeviceDownloadPanel from "./DeviceDownloadPanel";
import { messages as en } from "./messages/en";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceDownloadStatusResponse } from "./api";
import { subscribeAchievementEvents, type AchievementEvent } from "./achievementEvents";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;
let root: Root | undefined;

function project(): ProjectTree {
  const device = (id: number, address: string | null, name: string) => ({
    id,
    name,
    address,
    description: null,
    com_object_count: 0,
  });
  return {
    installations: [
      {
        id: 0,
        name: "Test",
        topology: [
          {
            id: 1,
            name: "Area",
            address: 1,
            lines: [
              {
                id: 2,
                name: "Line",
                address: 1,
                devices: [device(10, "1.1.67", "Push button 2-fold"), device(11, "1.1.3", "Actuator")],
              },
            ],
          },
        ],
        buildings: [],
        unassigned: [device(12, null, "Not yet addressed")],
        group_addresses: [],
        group_ranges: [],
      },
    ],
  } as unknown as ProjectTree;
}

const PLAN = {
  planId: 7,
  address: "1.1.67",
  deviceId: 10,
  deviceName: "Push button 2-fold",
  programId: "M-0083_A-0027-15-0BAC",
  maskVersion: 0x0701,
  manufacturer: 0x0083,
  parameterValues: 3,
  groupLinks: 1,
  segments: [{ id: "M-0083_A-0027-15-0BAC_RS-04-00000", address: 0x4000, size: 1418, written: 1416 }],
  dataOctets: 1416,
  steps: ["connect", "check the mask", "write 4003h"],
  confirmationPhrase: "I confirm download to 1.1.67",
  support: { level: "verified", evidence: "MDT, 2026-09-29; docs/RESEARCH.md §19.8", inferences: [] },
  untestedAcknowledgement: null,
  warnings: [],
};

const LOW_FIRST = "base-image-low-octet-first: the program declares no ParameterByteOrder … (KNOWN_LIMITATIONS §170, ADR-0120)";

const REPORT = {
  report: {
    kind: "knxbench-download-report",
    format: 1,
    knxbenchVersion: "0.1.0-alpha.10+gabc",
    program: { id: "M-0083_A-0027-15-0BAC", name: "Taster Plus", applicationVersion: "15", sourceSha256: "ab".repeat(32), manufacturer: "0083", mask: "0701" },
    scope: "complete",
    support: "untested",
    inferences: [],
    warnings: [LOW_FIRST],
    plan: { steps: 3, dataOctets: 1416, groupLinks: 1 },
    outcome: { completed: true, written: "yes", restart: "acknowledged", stoppedInStep: null },
    parameterValues: { "P-1_R-1": "300" },
  },
  fileName: "knxbench-download-report-M-0083_A-0027-15-0BAC.json",
  summary: "KNXBench download report (format 1)\nProgram: M-0083_A-0027-15-0BAC",
  issueUrl: "https://github.com/KNXBench-Labs/KNXBench/issues/new?template=download-report.yml&program=M-0083_A-0027-15-0BAC",
  mailto: "mailto:issue@knxbench.com?subject=KNXBench%20download%20report",
};

function conflict(): Error {
  const error = new Error("the download has not ended yet") as Error & { status: number };
  error.status = 409;
  return error;
}

function notFound(): Error {
  const error = new Error("no download to a device yet") as Error & { status: number };
  error.status = 404;
  return error;
}

function response(
  status: DeviceDownloadStatusResponse["status"],
  events: DeviceDownloadStatusResponse["events"],
  nextSince: number,
): DeviceDownloadStatusResponse {
  return {
    downloadId: 7,
    address: "1.1.67",
    deviceName: "Push button 2-fold",
    steps: 3,
    dataOctets: 1416,
    status,
    nextSince,
    events,
    backupFile: null,
  };
}

async function flush() {
  await act(async () => {
    for (let i = 0; i < 6; i += 1) await Promise.resolve();
  });
}

async function render(tree: ProjectTree | null = project(), onProjectChanged?: (tree: ProjectTree) => void) {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => {
    root!.render(<DeviceDownloadPanel project={tree} onProjectChanged={onProjectChanged} />);
  });
  await flush();
}

function button(label: string, scope: ParentNode = host!): HTMLButtonElement {
  const found = [...scope.querySelectorAll("button")].find((b) => b.textContent === label);
  if (!found) throw new Error(`button not found: ${label}`);
  return found;
}

// Devices are chosen by stable id; tests name them by their address label.
async function choose(address: string) {
  const select = host!.querySelector<HTMLSelectElement>("select[data-download-device]")!;
  const option = [...select.options].find((o) => o.textContent?.startsWith(`${address} — `));
  if (!option) throw new Error(`no device option for ${address}`);
  await act(async () => {
    select.value = option.value;
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
}

async function setGateway(value: string) {
  const input = host!.querySelector<HTMLInputElement>('.device-download-config input:not([type="radio"])')!;
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  await act(async () => {
    setter.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function showPlan() {
  await choose("1.1.67");
  await setGateway("192.0.2.10:3671");
  await act(async () => button(en["deviceDownload.preparePlan"]).click());
  await flush();
}

function consentDialog(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".programming-consent-dialog");
}

beforeEach(() => {
  vi.useFakeTimers();
  window.localStorage.clear();
  apiMock.pollDeviceDownload.mockReset().mockRejectedValue(notFound());
  apiMock.planDeviceDownload.mockReset().mockResolvedValue(PLAN);
  apiMock.startDeviceDownload.mockReset().mockResolvedValue({ downloadId: 7 });
  apiMock.serverVersion.mockReset().mockResolvedValue({ version: "0.1.0-alpha.1" });
  apiMock.getDeviceDownloadReport.mockReset().mockResolvedValue(REPORT);
  apiMock.getDeviceReadiness.mockReset().mockResolvedValue({ devices: [], counts: {} });
  savedReports.mockReset();
});

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove();
  host = undefined;
  root = undefined;
  vi.useRealTimers();
});

it("shows pre-plan refusal, actionable help and exact local detail without a report", async () => {
  apiMock.getDeviceReadiness.mockResolvedValue({ devices: [{ address: "1.1.67", name: "Private device", programRef: "M-0001_A-0001-01-0000", readiness: "unsupported", category: "byte-order", detail: "PRIVATE /home/customer 1.1.67", steps: null, octets: null }], counts: { unsupported: 1 } });
  await render();
  await choose("1.1.67");
  const help = host!.querySelector<HTMLElement>("[data-support-guidance]")!;
  expect(help).not.toBeNull();
  expect(help.textContent).toContain(en["deviceDownload.readiness.reason.byteOrder"]);
  expect(help.querySelector("[data-next-action=requestSupport]")).not.toBeNull();
  expect(help.querySelector<HTMLTextAreaElement>("textarea")!.value).toContain("PRIVATE /home/customer 1.1.67");
  expect(help.querySelectorAll("a")).toHaveLength(3);
  expect(apiMock.planDeviceDownload).not.toHaveBeenCalled();
  expect(apiMock.getDeviceDownloadReport).not.toHaveBeenCalled();
  expect(apiMock.startDeviceDownload).not.toHaveBeenCalled();
});

it("shows failed preparation guidance even when readiness had no mark", async () => {
  apiMock.planDeviceDownload.mockRejectedValue(Object.assign(new Error("PRIVATE plan detail"), { status: 422, body: { kind: "downloadPreparationRefused", category: "modules" } }));
  await render();
  await showPlan();
  const help = host!.querySelector<HTMLElement>("[data-support-guidance]")!;
  expect(help).not.toBeNull();
  expect(help.querySelector("[data-next-action=requestSupport]")).not.toBeNull();
  expect(help.querySelector<HTMLTextAreaElement>("textarea")!.value).toContain("PRIVATE plan detail");
  expect(host!.querySelector(".device-download-plan")).toBeNull();
  expect(host!.querySelector(".device-download-verify")).toBeNull();
});

it("drops old readiness immediately and ignores a late answer after project replacement", async () => {
  let resolveOld!: (value: unknown) => void;
  apiMock.getDeviceReadiness.mockImplementationOnce(() => new Promise((resolve) => { resolveOld = resolve; }));
  const first = project();
  await render(first);
  await choose("1.1.67");
  await act(async () => root!.render(<DeviceDownloadPanel project={project()} />));
  await flush();
  await act(async () => resolveOld({ devices: [{ address: "1.1.67", readiness: "unsupported", category: "modules", detail: "OLD SECRET" }], counts: {} }));
  await flush();
  expect(host!.textContent).not.toContain("OLD SECRET");
  expect(host!.querySelector("[data-next-action=requestSupport]")).toBeNull();
});

it("does not show a late prepared plan or error for a replaced project", async () => {
  let resolvePlan!: (value: unknown) => void;
  apiMock.planDeviceDownload.mockImplementationOnce(() => new Promise((resolve) => { resolvePlan = resolve; }));
  await render();
  await showPlan();
  await act(async () => root!.render(<DeviceDownloadPanel project={project()} />));
  await act(async () => resolvePlan(PLAN));
  await flush();
  expect(host!.querySelector(".device-download-plan")).toBeNull();
  expect(apiMock.startDeviceDownload).not.toHaveBeenCalled();
});

it("never downgrades compatibility after a transient readiness failure", async () => {
  apiMock.getDeviceReadiness.mockRejectedValue(new Error("PRIVATE service detail"));
  await render();
  await choose("1.1.67");
  const help = host!.querySelector<HTMLElement>("[data-support-guidance]")!;
  expect(help).not.toBeNull();
  expect(help.dataset.supportGuidance).toBe("transient");
  expect(help.querySelector("[data-next-action=retryService]")).not.toBeNull();
  expect(host!.querySelector("option[value='10']")!.textContent).not.toContain("unsupported");
});

it("clears refusal detail on selection change rather than painting the next device", async () => {
  apiMock.planDeviceDownload.mockRejectedValue(Object.assign(new Error("FIRST TARGET SECRET"), { status: 422, body: { kind: "downloadPreparationRefused", category: "modules" } }));
  await render();
  await showPlan();
  expect(host!.textContent).toContain("FIRST TARGET SECRET");
  await choose("1.1.3");
  expect(host!.textContent).not.toContain("FIRST TARGET SECRET");
  expect(host!.querySelector("[data-support-guidance]")).toBeNull();
});

it("says what a download is, and needs a project", async () => {
  await render(null);
  expect(host!.textContent).toContain(en["deviceDownload.title"]);
  expect(host!.textContent).toContain("This is not saving or exporting a file");
  expect(host!.textContent).toContain(en["deviceDownload.projectRequired"]);
  expect(host!.querySelector("select")).toBeNull();
});

it("offers only devices that have an individual address", async () => {
  await render();
  const options = [...host!.querySelectorAll<HTMLOptionElement>("select[data-download-device] option")].map((o) => [o.value, o.textContent]);
  expect(options).toEqual([["", en["deviceDownload.chooseDevice"]], ["11", "1.1.3 — Actuator"], ["10", "1.1.67 — Push button 2-fold"]]);
});

it("shows the plan before anything is sent", async () => {
  await render();
  await showPlan();
  expect(apiMock.planDeviceDownload).toHaveBeenCalledWith("1.1.67");
  const plan = host!.querySelector(".device-download-plan")!;
  expect(plan.textContent).toContain(en["deviceDownload.planNothingSent"]);
  expect(plan.textContent).toContain("M-0083_A-0027-15-0BAC");
  expect(plan.textContent).toContain("mask 0701h, manufacturer 0083h");
  expect(plan.textContent).toContain("3 parameter values, 1 group links");
  expect(plan.textContent).toContain("1416 octets");
  expect(plan.textContent).toContain("4000h");
  expect(apiMock.startDeviceDownload).not.toHaveBeenCalled();
});

it("asks for consent first, and Cancel sends nothing", async () => {
  await render();
  await showPlan();
  await act(async () => button("Download to 1.1.67").click());
  await flush();
  const dialog = consentDialog();
  expect(dialog).not.toBeNull();
  expect(dialog!.textContent).toContain("1.1.67 — Push button 2-fold");
  expect(apiMock.startDeviceDownload).not.toHaveBeenCalled();
  await act(async () => button(en["programmingConsent.cancel"], dialog!).click());
  await flush();
  expect(consentDialog()).toBeNull();
  expect(apiMock.startDeviceDownload).not.toHaveBeenCalled();
});

it("starts with the plan's own id and phrase after consent, then shows every block as it is read back", async () => {
  await render();
  await showPlan();
  apiMock.pollDeviceDownload
    .mockReset()
    .mockResolvedValueOnce(
      response(
        { state: "running" },
        [
          { kind: "stepStarted", number: 1, of: 3, step: "connect" },
          { kind: "stepStarted", number: 2, of: 3, step: "check the mask" },
          { kind: "stepDone", number: 2, observed: "mask 0701h" },
          { kind: "stepStarted", number: 3, of: 3, step: "write 4003h" },
          { kind: "dataWritten", number: 3, address: 0x4003, octets: [0x01, 0xab], written: 2, of: 1416 },
        ],
        5,
      ),
    )
    .mockResolvedValueOnce(
      response(
        { state: "finished", written: "yes", restart: "acknowledged", restartNote: null },
        [{ kind: "dataWritten", number: 3, address: 0x4005, octets: [0xff], written: 1416, of: 1416 }],
        6,
      ),
    );
  await act(async () => button("Download to 1.1.67").click());
  await flush();
  await act(async () => button(en["programmingConsent.confirm"], consentDialog()!).click());
  await flush();

  expect(apiMock.startDeviceDownload).toHaveBeenCalledTimes(1);
  expect(apiMock.startDeviceDownload).toHaveBeenCalledWith(7, "192.0.2.10:3671", "I confirm download to 1.1.67");
  expect(apiMock.pollDeviceDownload).toHaveBeenLastCalledWith(0, 7);
  const progress = host!.querySelector(".device-download-progress")!;
  expect(progress.textContent).toContain("Running: step 3 of 3 · write 4003h");
  expect(progress.textContent).toContain("Octets written and read back 2/1416");
  expect(progress.textContent).toContain("mask 0701h");
  expect(progress.querySelector(".device-download-blocks")!.parentElement!.classList.contains("device-download-blocks-scroll")).toBe(true);
  expect(progress.querySelector(".device-download-blocks")!.textContent).toContain("01 AB");

  await act(async () => vi.advanceTimersByTime(600));
  await flush();
  expect(apiMock.pollDeviceDownload).toHaveBeenLastCalledWith(5, 7);
  const rows = host!.querySelectorAll(".device-download-blocks tbody tr");
  expect(rows).toHaveLength(2);
  expect(rows[1].textContent).toContain("4005h");
  expect(rows[1].textContent).toContain("FF");
  expect(host!.querySelector("[data-written]")!.getAttribute("data-written")).toBe("yes");
  expect(host!.textContent).toContain("Written to the device: yes, 1416 octets");
  expect(host!.textContent).toContain(en["deviceDownload.restartAcknowledged"]);

  // Polling stops once the run has ended.
  const calls = apiMock.pollDeviceDownload.mock.calls.length;
  await act(async () => vi.advanceTimersByTime(3000));
  expect(apiMock.pollDeviceDownload.mock.calls.length).toBe(calls);
});

it("says loudly when the restart was not confirmed", async () => {
  apiMock.pollDeviceDownload.mockReset().mockResolvedValue(
    response(
      { state: "finished", written: "yes", restart: "unconfirmed", restartNote: "no T_ACK for A_Restart" },
      [],
      0,
    ),
  );
  await render();
  expect(host!.textContent).toContain("Restart: NOT confirmed.");
  expect(host!.textContent).toContain("no T_ACK for A_Restart");
  expect(host!.textContent).not.toContain(en["deviceDownload.restartAcknowledged"]);
});

it("says partially and where it stopped when a download fails midway", async () => {
  apiMock.pollDeviceDownload.mockReset().mockResolvedValue(
    response(
      { state: "failed", written: "partially", stoppedInStep: 12, error: "connection lost" },
      [],
      0,
    ),
  );
  await render();
  expect(host!.textContent).toContain("Download stopped in step 12.");
  expect(host!.textContent).toContain("Written to the device: partially");
  expect(host!.textContent).toContain("connection lost");
});

it("drops a shown plan when the project changes", async () => {
  await render();
  await showPlan();
  expect(host!.querySelector(".device-download-plan")).not.toBeNull();
  await act(async () => root!.render(<DeviceDownloadPanel project={project()} />));
  await flush();
  expect(host!.querySelector(".device-download-plan")).toBeNull();
});

it("drops a server-refused stale plan but retains its reason and prepare-again action", async () => {
  await render();
  await showPlan();
  apiMock.startDeviceDownload.mockRejectedValue(Object.assign(new Error("Server revision changed; nothing sent"), { status: 409, body: { kind: "downloadPlanStale" } }));
  await act(async () => button("Download to 1.1.67").click());
  await flush();
  await act(async () => button(en["programmingConsent.confirm"], consentDialog()!).click());
  await flush();
  expect(host!.querySelector(".device-download-plan")).toBeNull();
  expect(host!.querySelector("[data-next-action=prepareAgain]")).not.toBeNull();
  expect(host!.querySelector<HTMLTextAreaElement>("[data-support-guidance] textarea")!.value).toContain("Server revision changed; nothing sent");
});

it("shows the server's refusal and does not claim anything was written", async () => {
  await render();
  await showPlan();
  apiMock.startDeviceDownload.mockRejectedValue(
    new Error("the project changed since the plan was shown; nothing was sent, ask for the plan again"),
  );
  await act(async () => button("Download to 1.1.67").click());
  await flush();
  await act(async () => button(en["programmingConsent.confirm"], consentDialog()!).click());
  await flush();
  expect(host!.querySelector("[role=alert]")!.textContent).toContain("nothing was sent");
  expect(host!.querySelector(".device-download-progress")).toBeNull();
});

it("sends the phrase the server named, never one it builds itself", async () => {
  // The phrase's wording belongs to the server (`required_confirmation_phrase`);
  // a client that rebuilt it would drift the day that wording changes.
  apiMock.planDeviceDownload.mockResolvedValue({ ...PLAN, confirmationPhrase: "server wording for 1.1.67" });
  await render();
  await showPlan();
  await act(async () => button("Download to 1.1.67").click());
  await flush();
  await act(async () => button(en["programmingConsent.confirm"], consentDialog()!).click());
  await flush();
  expect(apiMock.startDeviceDownload).toHaveBeenCalledWith(7, "192.0.2.10:3671", "server wording for 1.1.67");
});

it("warns on an untested plan and requires a separate exact acknowledgement", async () => {
  apiMock.planDeviceDownload.mockResolvedValueOnce({
    ...PLAN,
    support: { level: "untested", evidence: null, inferences: [] },
    untestedAcknowledgement: "I accept an untested download to 1.1.67",
  });
  await render();
  await showPlan();
  expect(host!.textContent).toContain("Not verified on hardware");
  const start = button("Download to 1.1.67");
  expect(start.disabled).toBe(true);
  const input = host!.querySelector<HTMLInputElement>("[data-untested-acknowledgement]")!;
  expect(input).not.toBeNull();
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  await act(async () => {
    setter.call(input, "yes");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(start.disabled).toBe(true);
  await act(async () => {
    setter.call(input, "I accept an untested download to 1.1.67");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(start.disabled).toBe(false);
  await act(async () => start.click());
  await flush();
  await act(async () => button(en["programmingConsent.confirm"], consentDialog()!).click());
  await flush();
  expect(apiMock.startDeviceDownload).toHaveBeenCalledWith(
    7, "192.0.2.10:3671", "I confirm download to 1.1.67",
    "I accept an untested download to 1.1.67",
  );
});

it("shows the backup path during a run and on failure", async () => {
  apiMock.pollDeviceDownload.mockReset().mockResolvedValue({
    ...response({ state: "failed", written: "partially", stoppedInStep: 7, error: "read-back failed" },
      [{ kind: "backupTaken", regions: 2, octets: 32 }], 1),
    backupFile: "device-backups/1.1.67_backup.json",
  });
  await render();
  expect(host!.textContent).toContain("device-backups/1.1.67_backup.json");
  expect(host!.textContent).toContain("32 octets");
});

// KL-142: the scope is chosen before the plan; a partial plan says what it leaves out.
const PARTIAL_PLAN = {
  ...PLAN, planId: 8, partial: true, notWritten: [[0x4100, 12], [0x4200, 3]] as [number, number][],
  confirmationPhrase: "I confirm partial download to 1.1.67",
};

function scopeRadio(label: string): HTMLInputElement {
  const found = [...host!.querySelectorAll<HTMLLabelElement>(".device-download-scope label")]
    .find((candidate) => candidate.textContent === label);
  if (!found) throw new Error(`scope not found: ${label}`);
  return found.querySelector("input")!;
}

async function pickScope(label: string) {
  await act(async () => scopeRadio(label).click());
}

it("offers the four download scopes and the pending-only choice before a plan, complete by default", async () => {
  await render();
  const labels = [...host!.querySelectorAll(".device-download-scope label")].map((label) => label.textContent);
  expect(labels).toEqual([
    en["deviceDownload.scope.complete"], en["deviceDownload.scope.parameters"],
    en["deviceDownload.scope.groupAddresses"], en["deviceDownload.scope.both"], en["deviceDownload.scope.changes"],
  ]);
  expect(scopeRadio(en["deviceDownload.scope.complete"]).checked).toBe(true);
  expect(host!.querySelector(".device-download-scope legend")?.textContent).toBe(en["deviceDownload.scope.legend"]);
});

it("asks for a complete plan without a partial part", async () => {
  await render();
  await showPlan();
  expect(apiMock.planDeviceDownload).toHaveBeenCalledWith("1.1.67");
  expect(host!.querySelector(".device-download-not-written")).toBeNull();
});

it.each([
  ["deviceDownload.scope.parameters", { parameters: true, groupAddresses: false }],
  ["deviceDownload.scope.groupAddresses", { parameters: false, groupAddresses: true }],
  ["deviceDownload.scope.both", { parameters: true, groupAddresses: true }],
] as const)("asks for the %s plan with exactly that partial part", async (key, partial) => {
  apiMock.planDeviceDownload.mockResolvedValue(PARTIAL_PLAN);
  await render();
  await choose("1.1.67");
  await pickScope(en[key]);
  await setGateway("192.0.2.10:3671");
  await act(async () => button(en["deviceDownload.preparePlan"]).click());
  await flush();
  expect(apiMock.planDeviceDownload).toHaveBeenCalledWith("1.1.67", partial);
});

it("shows what a partial plan leaves out before anything is confirmed", async () => {
  apiMock.planDeviceDownload.mockResolvedValue(PARTIAL_PLAN);
  await render();
  await choose("1.1.67");
  await pickScope(en["deviceDownload.scope.parameters"]);
  await setGateway("192.0.2.10:3671");
  await act(async () => button(en["deviceDownload.preparePlan"]).click());
  await flush();
  const plan = host!.querySelector(".device-download-plan")!;
  expect(plan.textContent).toContain(en["deviceDownload.partialScope.parameters"]);
  const omitted = [...plan.querySelectorAll(".device-download-not-written tbody tr")].map((row) => row.textContent);
  expect(omitted).toEqual(["4100h12", "4200h3"]);
  expect(plan.textContent).toContain(en["deviceDownload.notWrittenExplainer"]);
  expect(apiMock.startDeviceDownload).not.toHaveBeenCalled();
  expect(consentDialog()).toBeNull();
});

it("says so when a partial plan skips no further application write", async () => {
  apiMock.planDeviceDownload.mockResolvedValue({ ...PARTIAL_PLAN, notWritten: [] });
  await render();
  await choose("1.1.67");
  await pickScope(en["deviceDownload.scope.groupAddresses"]);
  await setGateway("192.0.2.10:3671");
  await act(async () => button(en["deviceDownload.preparePlan"]).click());
  await flush();
  const plan = host!.querySelector(".device-download-plan")!;
  expect(plan.textContent).toContain(en["deviceDownload.partialScope.groupAddresses"]);
  expect(plan.textContent).toContain(en["deviceDownload.notWrittenNone"]);
  expect(plan.querySelector(".device-download-not-written table")).toBeNull();
});

it("drops a shown plan when the scope changes, so a plan for another scope is never started", async () => {
  await render();
  await showPlan();
  expect(host!.querySelector(".device-download-plan")).not.toBeNull();
  await pickScope(en["deviceDownload.scope.parameters"]);
  expect(host!.querySelector(".device-download-plan")).toBeNull();
});

it("renders the server's refusal of a scope it cannot derive and shows no plan", async () => {
  apiMock.planDeviceDownload.mockRejectedValue(new Error(
    "no partial download to device 1.1.67 prepared: a partial download needs parameters, group addresses or both"));
  await render();
  await choose("1.1.67");
  await pickScope(en["deviceDownload.scope.both"]);
  await act(async () => button(en["deviceDownload.preparePlan"]).click());
  await flush();
  expect(host!.querySelector('[role="alert"]')?.textContent).toContain("no partial download to device 1.1.67 prepared");
  expect(host!.querySelector(".device-download-plan")).toBeNull();
});

it("locks the scope while a plan is being prepared", async () => {
  let finish!: (plan: typeof PLAN) => void;
  apiMock.planDeviceDownload.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
  await render();
  await choose("1.1.67");
  await act(async () => button(en["deviceDownload.preparePlan"]).click());
  expect(scopeRadio(en["deviceDownload.scope.parameters"]).disabled).toBe(true);
  await act(async () => finish(PLAN));
  await flush();
  expect(scopeRadio(en["deviceDownload.scope.parameters"]).disabled).toBe(false);
});

// ADR-0089: only a verified download counts towards achievements.
async function achievementEventsFor(status: DeviceDownloadStatusResponse["status"]): Promise<AchievementEvent[]> {
  const seen: AchievementEvent[] = [];
  const unsubscribe = subscribeAchievementEvents((event) => seen.push(event));
  apiMock.pollDeviceDownload.mockReset().mockResolvedValue(response(status, [], 1));
  try {
    await render();
  } finally {
    unsubscribe();
  }
  return seen;
}

it("reports a download as verified when every block was read back and the restart was acknowledged", async () => {
  const seen = await achievementEventsFor({ state: "finished", written: "yes", restart: "acknowledged", restartNote: null });
  expect(seen).toEqual([{ type: "deviceDownloadVerified", subject: "1.1.67" }]);
});

it("also reports a verified download whose plan has no restart", async () => {
  const seen = await achievementEventsFor({ state: "finished", written: "yes", restart: "notInPlan", restartNote: null });
  expect(seen).toEqual([{ type: "deviceDownloadVerified", subject: "1.1.67" }]);
});

it("does not report a download whose restart was not confirmed, or one that failed", async () => {
  expect(await achievementEventsFor({ state: "finished", written: "yes", restart: "unconfirmed", restartNote: "no T_ACK" })).toEqual([]);
  if (root) await act(async () => root!.unmount());
  host?.remove();
  expect(await achievementEventsFor({ state: "failed", written: "partially", stoppedInStep: 2, error: "timeout" })).toEqual([]);
  if (root) await act(async () => root!.unmount());
  host?.remove();
  expect(await achievementEventsFor({ state: "running" })).toEqual([]);
});

it("names the plan's warnings in plain words and lists its assumptions, before anything is sent", async () => {
  apiMock.planDeviceDownload.mockResolvedValue({
    ...PLAN,
    warnings: [LOW_FIRST, "a-future-rule: something new"],
    support: { ...PLAN.support, inferences: ["union-later-member: UP-33_R-33 is written last (RESEARCH §19.20)"] },
  });
  await render();
  await showPlan();
  const warnings = host!.querySelector(".device-download-warnings")!;
  expect(warnings.textContent).toContain(en["deviceDownload.warnings.intro"]);
  expect([...warnings.querySelectorAll("li[data-plan-warning]")].map((li) => li.getAttribute("data-plan-warning")))
    .toEqual(["base-image-low-octet-first", "a-future-rule"]);
  expect(warnings.querySelector('[data-plan-warning="base-image-low-octet-first"]')!.textContent)
    .toBe(en["deviceDownload.warning.baseImageLowOctetFirst"]);
  expect(warnings.querySelector('[data-plan-warning="a-future-rule"]')!.textContent).toBe("a-future-rule: something new");
  expect(warnings.querySelector("details")!.textContent).toContain("KNOWN_LIMITATIONS §170, ADR-0120");
  const inferences = host!.querySelector(".device-download-inferences")!;
  expect(inferences.textContent).toContain("union-later-member: UP-33_R-33 is written last");
  expect(apiMock.startDeviceDownload).not.toHaveBeenCalled();
});

it("shows neither box for a plan without warnings or assumptions, and the help hint only when untested", async () => {
  await render();
  await showPlan();
  expect(host!.querySelector(".device-download-warnings")).toBeNull();
  expect(host!.querySelector(".device-download-inferences")).toBeNull();
  expect(host!.querySelector("[data-verify-hint]")).toBeNull();
  apiMock.planDeviceDownload.mockResolvedValue({
    ...PLAN,
    support: { level: "untested", evidence: null, inferences: [] },
    untestedAcknowledgement: "I accept an untested download to 1.1.67",
  });
  await act(async () => button(en["deviceDownload.preparePlan"]).click());
  await flush();
  expect(host!.querySelector("[data-verify-hint]")!.textContent).toBe(en["deviceDownload.verify.planHint"]);
});

it("offers no report while a download runs", async () => {
  apiMock.pollDeviceDownload.mockReset().mockResolvedValue(response({ state: "running" }, [], 0));
  await render();
  expect(host!.querySelector(".device-download-progress")).not.toBeNull();
  expect(host!.querySelector(".device-download-verify")).toBeNull();
});

it("after a download, explains the steps and creates the report only on request, with a checklist and both ways to send it", async () => {
  apiMock.pollDeviceDownload.mockReset().mockResolvedValue(
    response({ state: "finished", written: "yes", restart: "acknowledged", restartNote: null }, [], 0),
  );
  await render();
  const verify = host!.querySelector(".device-download-verify")!;
  expect(verify.textContent).toContain(en["deviceDownload.verify.intro.untested"]);
  expect([...verify.querySelectorAll(".device-download-verify-steps li")].map((li) => li.textContent)).toEqual([
    en["deviceDownload.verify.step.check"], en["deviceDownload.verify.step.create"], en["deviceDownload.verify.step.save"],
    en["deviceDownload.verify.step.send"], en["deviceDownload.verify.step.review"],
  ]);
  expect(apiMock.getDeviceDownloadReport).not.toHaveBeenCalled();

  await act(async () => button(en["deviceDownload.verify.create"], verify).click());
  await flush();
  expect(apiMock.getDeviceDownloadReport).toHaveBeenCalledWith(7);
  expect([...verify.querySelectorAll("[data-check]")].map((label) => label.getAttribute("data-check")))
    .toEqual(["groupAddresses", "status", "parameters", "multiOctet"]);
  const box = verify.querySelector<HTMLInputElement>('[data-check="multiOctet"] input')!;
  await act(async () => box.click());
  expect(box.checked).toBe(true);
  expect(verify.textContent).toContain(en["deviceDownload.verify.privacy"]);

  const issue = verify.querySelector<HTMLAnchorElement>("a[data-verify-issue]")!;
  expect(issue.getAttribute("href")).toBe(REPORT.issueUrl);
  expect(issue.getAttribute("target")).toBe("_blank");
  expect(issue.getAttribute("rel")).toBe("noopener noreferrer");
  expect(verify.querySelector("a[data-verify-mail]")!.getAttribute("href")).toBe(REPORT.mailto);
  expect(verify.querySelector(".device-download-verify-summary")!.textContent).toBe(REPORT.summary);

  await act(async () => button(en["deviceDownload.verify.save"], verify).click());
  expect(savedReports).toHaveBeenCalledTimes(1);
  const [contents, name] = savedReports.mock.calls[0] as [string, string];
  expect(name).toBe(REPORT.fileName);
  expect(JSON.parse(contents)).toEqual(REPORT.report);
  expect(contents.endsWith("\n")).toBe(true);
  expect(apiMock.startDeviceDownload).not.toHaveBeenCalled();
});

it("asks to try again while the run is still finishing, and asks to report a failed run too", async () => {
  apiMock.pollDeviceDownload.mockReset().mockResolvedValue(
    response({ state: "failed", written: "partially", stoppedInStep: 12, error: "connection lost" }, [], 0),
  );
  apiMock.getDeviceDownloadReport.mockReset().mockRejectedValueOnce(conflict()).mockResolvedValueOnce({
    ...REPORT,
    report: { ...REPORT.report, outcome: { completed: false, written: "partially", restart: null, stoppedInStep: 12 } },
  });
  await render();
  const verify = host!.querySelector(".device-download-verify")!;
  expect(verify.textContent).toContain(en["deviceDownload.verify.intro.failed"]);
  expect(verify.textContent).not.toContain(en["deviceDownload.verify.step.check"]);
  await act(async () => button(en["deviceDownload.verify.create"], verify).click());
  await flush();
  expect(verify.textContent).toContain(en["deviceDownload.verify.stillFinishing"]);
  await act(async () => button(en["deviceDownload.verify.create"], verify).click());
  await flush();
  expect(verify.querySelector(".device-download-verify-checklist")).toBeNull();
  expect(verify.querySelector("a[data-verify-issue]")).not.toBeNull();
});

it("says why no report is available", async () => {
  apiMock.pollDeviceDownload.mockReset().mockResolvedValue(
    response({ state: "finished", written: "yes", restart: "acknowledged", restartNote: null }, [], 0),
  );
  apiMock.getDeviceDownloadReport.mockReset().mockRejectedValue(new Error("download 7 has been replaced"));
  await render();
  const verify = host!.querySelector(".device-download-verify")!;
  await act(async () => button(en["deviceDownload.verify.create"], verify).click());
  await flush();
  expect(verify.querySelector('[role="alert"]')!.textContent).toBe("No report available: download 7 has been replaced");
});

// ADR-0129: the server settles the programming state when a download that
// wrote ends; the editor reloads the project once per such run.
it("warns when programming state cannot be refreshed without changing the download outcome", async () => {
  apiMock.currentProject.mockReset().mockRejectedValue(new Error("offline"));
  const changed = vi.fn();
  apiMock.pollDeviceDownload.mockReset().mockResolvedValue(
    response({ state: "finished", written: "yes", restart: "acknowledged", restartNote: null }, [], 1));
  await render(project(), changed);
  const warning = host!.querySelector('[data-programming-refresh]');
  expect(warning?.getAttribute("role")).toBe("alert");
  expect(warning?.textContent).toContain("Reload the project");
  expect(warning?.textContent).toContain("offline");
  expect(host!.querySelector('[data-download-state]')?.getAttribute("data-download-state")).toBe("finished");
  expect(host!.querySelector('[data-written]')?.getAttribute("data-written")).toBe("yes");
  expect(changed).not.toHaveBeenCalled();
});

it("reloads the project once after a download that wrote, and not after one that wrote nothing", async () => {
  const reloaded = project();
  apiMock.currentProject.mockReset().mockResolvedValue(reloaded);
  const changed = vi.fn();
  apiMock.pollDeviceDownload.mockReset().mockResolvedValue(
    response({ state: "failed", written: "partially", stoppedInStep: 2, error: "lost" }, [], 1));
  await render(project(), changed);
  expect(apiMock.currentProject).toHaveBeenCalledTimes(1);
  expect(changed).toHaveBeenCalledExactlyOnceWith(reloaded);
  if (root) await act(async () => root!.unmount());
  host?.remove();

  apiMock.currentProject.mockClear(); changed.mockClear();
  apiMock.pollDeviceDownload.mockReset().mockResolvedValue(
    response({ state: "failed", written: "no", stoppedInStep: null, error: "refused" }, [], 1));
  await render(project(), changed);
  expect(apiMock.currentProject).not.toHaveBeenCalled();
  expect(changed).not.toHaveBeenCalled();
});

tableColumnSmoke([["download-unwritten", 2], ["download-segments", 3], ["download-blocks", 4]], async id => {
  apiMock.planDeviceDownload.mockResolvedValue({ ...PLAN, partial: true, notWritten: [[0x4008, 2]] });
  await render(); await showPlan();
  if (id === "download-blocks") {
    apiMock.pollDeviceDownload.mockResolvedValue(response({ state: "running" }, [{ kind: "dataWritten", number: 3, address: 0x4003, octets: [1], written: 1, of: 1416 }], 1));
    await act(async () => button("Download to 1.1.67").click()); await flush();
    await act(async () => button(en["programmingConsent.confirm"], consentDialog()!).click()); await flush();
  }
  return { host: host! };
});
