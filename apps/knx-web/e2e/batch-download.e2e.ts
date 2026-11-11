/** Batch download in the browser: review, acknowledge, one batch consent, per-device status. */
// Every `/api/**` request is answered here; anything not listed is recorded as
// unexpected and fails the case, so no bus route can reach a server. The
// consent is remembered for the build's stage: the batch still asks.
import { openExplorerBranches, openNavigationSections } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import type { PendingProgramming } from "../src/bindings/PendingProgramming";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { busActivityFixtureAnswer } from "./bus-activity-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

const VERSION = "0.1.0-alpha.20";
const GATEWAY = "192.0.2.10:3671";
const pending = (address: boolean, application: boolean, parameters: boolean, groupAddresses: boolean): PendingProgramming =>
  ({ address, application, parameters, group_addresses: groupAddresses });

function nodes(settled: boolean): DeviceNode[] {
  const after = (pendingNow: PendingProgramming) => settled ? pending(false, false, false, false) : pendingNow;
  return [
    { id: 1, name: "Dimmer kitchen", address: "1.1.2", description: null, com_object_count: 0, pending_programming: after(pending(false, true, false, false)) },
    { id: 2, name: "Blind living room", address: "1.1.3", description: null, com_object_count: 0, pending_programming: pending(false, false, true, false) },
    { id: 3, name: "Sensor hall", address: "1.1.4", description: null, com_object_count: 0, pending_programming: pending(true, true, true, true) },
  ];
}
function fixtureTree(settled: boolean): ProjectTree {
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "batch-browser", project_incarnation: 3, snapshot_revision: settled ? 2 : 1,
    installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1,
      lines: [{ id: 2, name: "Ground floor", address: 1, devices: nodes(settled) }] }], unassigned: [],
      buildings: [], group_ranges: [], group_addresses: [] }] };
}

const REVIEW = { batchId: 40, devices: [
  { deviceId: 1, address: "1.1.2", name: "Dimmer kitchen", status: "inBatch", planId: 41, programId: "M-00FA_A-1", scope: "complete",
    steps: 12, dataOctets: 40, support: "verified", inferences: [], warnings: [],
    confirmationPhrase: "I confirm download to 1.1.2", untestedAcknowledgement: null },
  { deviceId: 2, address: "1.1.3", name: "Blind living room", status: "inBatch", planId: 42, programId: "M-00FA_A-2", scope: "parameters",
    steps: 8, dataOctets: 4, support: "untested", inferences: [], warnings: [],
    confirmationPhrase: "I confirm download to 1.1.3", untestedAcknowledgement: "I accept an untested download to 1.1.3" },
  { deviceId: 3, address: "1.1.4", name: "Sensor hall", status: "notInBatch", reason: { code: "addressPending" } },
] };
const ROWS = REVIEW.devices.map(({ deviceId, address, name }) => ({ deviceId, address, name }));
const running = { batchId: 40, state: "running", stopRequested: false, devices: [
  { ...ROWS[0], state: "running", downloadId: 41 },
  { ...ROWS[1], state: "waiting", downloadId: 42 },
  { ...ROWS[2], state: "notInBatch", reason: { code: "addressPending" } },
] };
const CHECK = { observation: { status: "attempted", reach: { result: "reached", identity: { result: "match", exemplar: "notVerified",
  loadStates: [], planLoadVerdict: "matchesPlan" } } }, runtime: "unknown" };
const ended = { ...running, state: "ended", devices: [
  { ...ROWS[0], state: "done", downloadId: 41, written: "yes", restart: "acknowledged", postDownloadCheck: CHECK },
  { ...ROWS[1], state: "unconfirmed", downloadId: 42, doubt: "restartUnconfirmed", written: "yes", restart: "unconfirmed",
    restartNote: "no acknowledgement", postDownloadCheck: CHECK },
  running.devices[2],
] };

async function intercept(page: Page, language: "en" | "de") {
  const state = { unexpected: [] as string[], plans: 0, starts: [] as unknown[], statusReads: 0, started: false, settled: false };
  let projectOpen = false;
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok",
      settings: { uiLanguage: language, theme: "porcelain", programmingConsent: { stage: "alpha", version: VERSION } } };
    else if (path === "/api/version" && method === "GET") body = { version: VERSION };
    else if (path === "/api/project" && method === "GET") {
      if (!projectOpen) return fulfillNoProjectOpen(route);
      body = fixtureTree(state.settled);
    } else if (path === "/api/project/new" && method === "POST") { projectOpen = true; body = fixtureTree(false); }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(fixtureTree(state.settled));
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (path === "/api/legacy-password" && method === "GET") body = { available: true, remembered: false, problem: null };
    else if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no operation in this fixture" }; }
    else if (path === "/api/device-system2/status" && method === "GET") body = null;
    else if (path === "/api/bus/monitor/telegrams" && method === "GET") { status = 404; body = { error: "no monitor session is active" }; }
    else if (path === "/api/bus/activity" && method === "GET") body = { serverIncarnation: "batch-browser", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] };
    else if (path === "/api/device-readiness" && method === "GET") body = { devices: [], counts: {} };
    else if (path === "/api/device-download/batch/plan" && method === "POST") { state.plans += 1; body = REVIEW; }
    else if (path === "/api/device-download/batch/start" && method === "POST") {
      state.starts.push(route.request().postDataJSON()); state.started = true; body = { batchId: 40 };
    } else if (path === "/api/device-download/batch/status" && method === "GET") {
      if (!state.started) { status = 404; body = { error: "no batch download yet" }; }
      else {
        state.statusReads += 1;
        body = state.statusReads < 3 ? running : ended;
        if (state.statusReads >= 3) state.settled = true;
      }
    } else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else if (busActivityFixtureAnswer(method, path) !== undefined) body = busActivityFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict batch fixture only" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

async function openProject(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
  await openNavigationSections(page);
  await openExplorerBranches(page);
}

async function noSidewaysOverflow(page: Page, width: number) {
  const panel = page.locator(".device-download-batch");
  const box = (await panel.boundingBox())!;
  expect(box.x).toBeGreaterThanOrEqual(-1);
  expect(box.x + box.width).toBeLessThanOrEqual(width + 1);
  expect(await panel.evaluate((node) => node.scrollWidth - node.clientWidth)).toBeLessThanOrEqual(1);
}

for (const language of ["en", "de"] as const) {
  const m = language === "en" ? en : de;
  for (const width of [1440, 400]) {
    test(`${language} download all from the pending block: review, acknowledge, confirm, result per device at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      const errors: string[] = []; page.on("pageerror", (error) => errors.push(error.message));
      const state = await intercept(page, language); await openProject(page, language);
      await page.addStyleTag({ content: ".toast-stack { display: none !important; }" });
      const all = page.locator(".pending-programming-panel [data-pending-download-all]");
      await all.scrollIntoViewIfNeeded();
      await expect(all).toHaveText(m["pendingPanel.downloadAll"]);
      expect(state.plans).toBe(0);
      await all.click();

      // The download tab opens with the review; nothing was sent.
      await expect(page.locator(".bus-diagnostics-tabs button[aria-current=page]")).toHaveText(m["deviceDownload.title"]);
      const review = page.locator("[data-batch-review]");
      await expect(review).toBeVisible();
      expect(state.plans).toBe(1);
      await expect(review.locator("[data-batch-device]")).toHaveCount(3);
      await expect(review.locator("[data-batch-device='1.1.4']")).toContainText(m["deviceDownload.batch.reason.addressPending"]);
      await expect(review.locator("[data-batch-device='1.1.4'] input[type=checkbox]")).toHaveCount(0);
      const start = review.locator("[data-batch-start]");
      await expect(start).toBeDisabled();
      await page.locator("input[data-download-gateway]").fill(GATEWAY);
      await expect(start).toBeDisabled();
      await expect(review.locator("[data-batch-blocked]")).toHaveText(m["deviceDownload.batch.blocked.acknowledgement"]);
      await review.locator("[data-batch-device='1.1.3'] [data-batch-ack]").fill("I accept an untested download to 1.1.3");
      await expect(start).toBeEnabled();
      await expect(start).toHaveText(m["deviceDownload.batch.start.other"].replace("{count}", "2"));
      await noSidewaysOverflow(page, width);

      // One batch confirmation, shown although the stage's answer is remembered.
      await start.click();
      const dialog = page.locator(".programming-consent-dialog");
      await expect(dialog).toBeVisible();
      await expect(dialog.locator("h2")).toHaveText(m["programmingConsent.batch.title.other"].replace("{count}", "2"));
      await expect(dialog.locator(".programming-consent-batch li")).toHaveText(["1.1.2 — Dimmer kitchen", "1.1.3 — Blind living room"]);
      await expect(dialog.locator(".programming-consent-remember")).toHaveCount(0);
      expect(state.starts).toEqual([]);
      await dialog.getByRole("button", { name: m["programmingConsent.batch.confirm.other"].replace("{count}", "2"), exact: true }).click();

      // The server runs it; the page shows a result per device.
      const run = page.locator("[data-batch-run]");
      await expect(run).toHaveAttribute("data-batch-run", "ended");
      expect(state.starts).toEqual([{ batchId: 40, gateway: GATEWAY, devices: [
        { address: "1.1.2", confirmation: "I confirm download to 1.1.2" },
        { address: "1.1.3", confirmation: "I confirm download to 1.1.3", acceptUntested: "I accept an untested download to 1.1.3" },
      ] }]);
      await expect(run.locator("[data-batch-device]")).toHaveCount(3);
      await expect(run.locator("[data-batch-device='1.1.2']")).toContainText(m["deviceDownload.batch.state.done"]);
      await expect(run.locator("[data-batch-device='1.1.3']")).toContainText(m["deviceDownload.batch.state.restartUnconfirmed"]);
      await expect(run.locator("[role=status]").first()).toHaveText(m["deviceDownload.batch.ended"]);
      await noSidewaysOverflow(page, width);
      // The settled project was reloaded: one device left in the pending count.
      expect(errors).toEqual([]);
      expect(state.unexpected).toEqual([]);
    });
  }
  test(`${language} the download tab's own Download all… only reviews at 1440px`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    const state = await intercept(page, language); await openProject(page, language);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
    await page.getByRole("combobox").press("Enter");
    await page.locator(".bus-diagnostics-tabs").getByRole("button", { name: m["deviceDownload.title"], exact: true }).click();
    const open = page.locator("[data-batch-open]");
    await expect(open).toHaveText(m["deviceDownload.batch.open"]);
    expect(state.plans).toBe(0);
    await open.click();
    await expect(page.locator("[data-batch-review]")).toBeVisible();
    expect(state.plans).toBe(1);
    await page.locator("[data-batch-close]").click();
    await expect(page.locator("[data-batch-review]")).toHaveCount(0);
    expect(state.starts).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });
}
