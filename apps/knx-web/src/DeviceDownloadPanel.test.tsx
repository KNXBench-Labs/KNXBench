/** Verifies the device-download panel: plan first, consent before sending, every block shown. */
// @vitest-environment happy-dom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  planDeviceDownload: vi.fn(),
  startDeviceDownload: vi.fn(),
  pollDeviceDownload: vi.fn(),
  serverVersion: vi.fn(),
}));

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
  support: { level: "verified", evidence: "MDT, 2026-09-29; docs/RESEARCH.md §19.8" },
  untestedAcknowledgement: null,
};

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

async function render(tree: ProjectTree | null = project()) {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => {
    root!.render(<DeviceDownloadPanel project={tree} />);
  });
  await flush();
}

function button(label: string, scope: ParentNode = host!): HTMLButtonElement {
  const found = [...scope.querySelectorAll("button")].find((b) => b.textContent === label);
  if (!found) throw new Error(`button not found: ${label}`);
  return found;
}

async function choose(address: string) {
  const select = host!.querySelector("select")!;
  await act(async () => {
    select.value = address;
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
});

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove();
  host = undefined;
  root = undefined;
  vi.useRealTimers();
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
  const options = [...host!.querySelectorAll("option")].map((o) => o.value);
  expect(options).toEqual(["", "1.1.3", "1.1.67"]);
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
    support: { level: "untested", evidence: null },
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

it("offers the four download scopes before a plan, complete by default", async () => {
  await render();
  const labels = [...host!.querySelectorAll(".device-download-scope label")].map((label) => label.textContent);
  expect(labels).toEqual([
    en["deviceDownload.scope.complete"], en["deviceDownload.scope.parameters"],
    en["deviceDownload.scope.groupAddresses"], en["deviceDownload.scope.both"],
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
