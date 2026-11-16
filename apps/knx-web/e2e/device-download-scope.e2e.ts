/** KL-142 in a browser: the download scope is chosen, sent, and its omissions shown first. */
import { openNavigationSections } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";

/** Screenshot dir for the #52 offer/plan captures; defaults to the test output. */
const shots = (testInfo: { outputPath: (name: string) => string }, name: string) =>
  process.env.KNX_A1_CLOSURE_SHOTS ? join(process.env.KNX_A1_CLOSURE_SHOTS, name) : testInfo.outputPath(name);

const device = (id: number, address: string, name: string) => ({ id, name, address, description: null, com_object_count: 0 });
const tree = {
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "download-scope", snapshot_revision: 1,
  installations: [{
    id: 1, name: "Installation 1", buildings: [], unassigned: [], group_addresses: [], group_ranges: [],
    topology: [{ id: 2, name: "Area", address: 1, lines: [{ id: 3, name: "Line", address: 1,
      devices: [device(10, "1.1.67", "Push button 2-fold")] }] }],
  }],
};
const plan = (partial: boolean) => ({
  planId: partial ? 8 : 7, address: "1.1.67", deviceId: 10, deviceName: "Push button 2-fold",
  programId: "M-0083_A-0027-15-0BAC", maskVersion: 0x0701, manufacturer: 0x0083,
  parameterValues: 3, groupLinks: 1,
  segments: [{ id: "M-0083_A-0027-15-0BAC_RS-04-00000", address: 0x4000, size: 1418, written: 1416 }],
  dataOctets: 1416, steps: ["connect", "check the mask", "write 4003h"],
  confirmationPhrase: partial ? "I confirm partial download to 1.1.67" : "I confirm download to 1.1.67",
  accessKey: "none", support: { level: "verified", evidence: "MDT, 2026-09-29", inferences: [] }, warnings: [], untestedAcknowledgement: null,
  partial, notWritten: partial ? [[0x4100, 12]] : [],
});

async function open(page: Page, planOverride?: Record<string, unknown>, planError?: { status: number; body: Record<string, unknown> }) {
  const planBodies: unknown[] = [];
  const writes: string[] = [];
  // Like the real server: no project until the test creates one (ADR-0127).
  let projectOpen = false;
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    const json = (body: unknown, status = 200) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status") return json({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return json({ schemaVersion: 1, status: "ok", settings: {} });
    if (path === "/api/project" && method === "GET") return projectOpen ? json(tree) : json({ error: "no project open" }, 400);
    if (path === "/api/project/new" && method === "POST") { projectOpen = true; return json(tree); }
    if (path === "/api/bus/discover") return json({ interfaces: [] });
    if (path === "/api/bus/activity" && method === "GET") return json({ serverIncarnation: "fixture", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] });
    if (path === "/api/device-readiness" && method === "GET") return json({ devices: [], counts: {} });
    if (path === "/api/device-download/plan" && method === "POST") {
      const body = request.postDataJSON() as { address: string; scope?: string; partial?: { parameters: boolean; groupAddresses: boolean } };
      planBodies.push(body);
      if (body.partial && !body.partial.parameters) {
        return json({ error: "no partial download to device 1.1.67 prepared: the complete download allocates no application program task segment" }, 422);
      }
      if (planError && body.scope === "changes") return json(planError.body, planError.status);
      return json({ ...plan(body.partial !== undefined), ...planOverride });
    }
    if (path.startsWith("/api/device-download/") || path.startsWith("/api/bus/")) {
      if (method !== "GET") writes.push(`${method} ${path}`);
      return json({ error: "not in this fixture" }, 404);
    }
    if (method === "GET") return json([]);
    writes.push(`${method} ${path}`);
    return json({ error: "not in this fixture" }, 404);
  });
  await page.setViewportSize({ width: 1400, height: 900 });
  await page.goto("/");
  await page.getByRole("button", { name: "New project…" }).click();
  await page.getByRole("dialog", { name: "New project" }).getByRole("button", { name: "Create project" }).click();
  await page.getByRole("dialog", { name: "Project created" }).getByRole("button", { name: "Done" }).click();
  await openNavigationSections(page);
  await page.getByRole("navigation", { name: "Bus monitor" }).getByRole("button", { name: "Bus monitor" }).first().click();
  await page.getByRole("button", { name: "Download to device", exact: true }).click();
  // Devices are chosen by stable id; the label carries the address.
  await page.locator("select[data-download-device]").selectOption({ label: "1.1.67 — Push button 2-fold" });
  return { planBodies, writes };
}

test("a parameters-only scope is sent as such and the plan names what it leaves out, before any consent", async ({ page }) => {
  const { planBodies, writes } = await open(page);
  const scope = page.getByRole("group", { name: "What to write" });
  await expect(scope.getByRole("radio")).toHaveCount(5);
  await expect(scope.getByRole("radio", { name: "Complete download" })).toBeChecked();
  await scope.getByRole("radio", { name: "Parameters only" }).check();
  await page.getByRole("button", { name: "Prepare download" }).click();
  const shown = page.getByRole("region", { name: "Download plan" });
  await expect(shown).toContainText("Partial download: parameters only.");
  await expect(shown.locator(".device-download-not-written tbody tr")).toHaveText(["4100h12"]);
  expect(planBodies).toEqual([{ address: "1.1.67", partial: { parameters: true, groupAddresses: false } }]);
  await scope.getByRole("radio", { name: "Complete download" }).check();
  await expect(shown).toHaveCount(0);
  await page.getByRole("button", { name: "Prepare download" }).click();
  await expect(page.getByRole("region", { name: "Download plan" })).not.toContainText("Partial download");
  expect(planBodies[1]).toEqual({ address: "1.1.67" });
  await expect(page.locator(".programming-consent-dialog")).toHaveCount(0);
  expect(writes).toEqual([]);
});

test("a scope the server cannot derive is refused visibly and leaves no plan to start", async ({ page }) => {
  const { planBodies, writes } = await open(page);
  await page.getByRole("group", { name: "What to write" }).getByRole("radio", { name: "Group addresses only" }).check();
  await page.getByRole("button", { name: "Prepare download" }).click();
  await expect(page.getByRole("alert")).toContainText("no partial download to device 1.1.67 prepared");
  await expect(page.getByRole("region", { name: "Download plan" })).toHaveCount(0);
  expect(planBodies).toEqual([{ address: "1.1.67", partial: { parameters: false, groupAddresses: true } }]);
  expect(writes).toEqual([]);
});

const NOTHING_PENDING = {
  status: 422,
  body: {
    kind: "downloadNothingPending", error: "nothing pending for 1.1.67",
    scopeAdvice: { basis: "projectFlags", write: "nothing", address: "current", mediumPending: false, fullyProgrammed: true },
  },
};

test("#52: nothing pending offers the explicit complete plan; the offer sends the bare complete request", async ({ page }, testInfo) => {
  const { planBodies, writes } = await open(page, {
    partial: false, scope: "complete", scopeFromChanges: false,
    scopeAdvice: { basis: "projectFlags", write: "nothing", address: "current", mediumPending: false, fullyProgrammed: true },
  }, NOTHING_PENDING);
  const scope = page.getByRole("group", { name: en["deviceDownload.scope.legend"] });
  await scope.getByRole("radio", { name: en["deviceDownload.scope.changes"] }).check();
  await page.getByRole("button", { name: en["deviceDownload.preparePlan"] }).click();
  const offer = page.getByRole("button", { name: en["deviceDownload.prepareComplete"] });
  await expect(offer).toBeVisible();
  await expect(page.locator("[data-scope-nothing]")).toContainText(en["deviceDownload.scopeAdvice.nothing"]);
  await expect(page.getByRole("region", { name: en["deviceDownload.planTitle"] })).toHaveCount(0);
  await page.locator("[data-scope-nothing]").screenshot({ path: shots(testInfo, "offer.png") });
  await offer.click();
  // Exactly the "Complete download" radio's request: no scope, no partial.
  expect(planBodies).toEqual([{ address: "1.1.67", scope: "changes" }, { address: "1.1.67" }]);
  const shown = page.getByRole("region", { name: en["deviceDownload.planTitle"] });
  await expect(shown).toBeVisible();
  // The shown plan is the complete one, not a partial scope text.
  await expect(shown).not.toContainText(en["deviceDownload.partialScope.parameters"]);
  await expect(page.locator("[data-scope-nothing]")).toHaveCount(0);
  await shown.screenshot({ path: shots(testInfo, "plan.png") });
  // Preparing a plan never starts a download.
  expect(writes).toEqual([]);
});

test("#52: a refused complete offer is shown visibly, and switching the scope clears the offer", async ({ page }) => {
  const refused = {
    status: 422,
    body: { kind: "downloadPreparationRefused", error: "no program image for 1.1.67", category: "modules" },
  };
  const { writes } = await open(page);
  const planBodies: unknown[] = [];
  // First the nothing-pending answer, then refuse the bare complete request.
  await page.route("**/api/device-download/plan", async (route) => {
    const body = route.request().postDataJSON() as { scope?: string };
    planBodies.push(body);
    if (body.scope === "changes") {
      await route.fulfill({ status: 422, contentType: "application/json", body: JSON.stringify(NOTHING_PENDING.body) });
    } else {
      await route.fulfill({ status: refused.status, contentType: "application/json", body: JSON.stringify(refused.body) });
    }
  });
  const scope = page.getByRole("group", { name: en["deviceDownload.scope.legend"] });
  await scope.getByRole("radio", { name: en["deviceDownload.scope.changes"] }).check();
  await page.getByRole("button", { name: en["deviceDownload.preparePlan"] }).click();
  await expect(page.locator("[data-scope-nothing]")).toBeVisible();
  await page.getByRole("button", { name: en["deviceDownload.prepareComplete"] }).click();
  // The refusal is visible, no plan and no silent no-op.
  await expect(page.getByRole("alert")).toContainText("no program image for 1.1.67");
  await expect(page.getByRole("region", { name: en["deviceDownload.planTitle"] })).toHaveCount(0);
  expect(planBodies).toEqual([{ address: "1.1.67", scope: "changes" }, { address: "1.1.67" }]);
  expect(writes).toEqual([]);
  // Switching the scope clears the stale offer/result.
  await scope.getByRole("radio", { name: en["deviceDownload.scope.complete"] }).check();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.locator("[data-scope-nothing]")).toHaveCount(0);
});

for (const width of [1400, 400]) {
  test(`#39 at ${width}px: a parameters-only plan warns that a pending group-address change stays unwritten`, async ({ page }) => {
    const { writes } = await open(page, { scope: "parameters", scopeFromChanges: false,
      scopeAdvice: { basis: "projectFlags", write: "both", address: "current", mediumPending: false, fullyProgrammed: false } });
    await page.setViewportSize({ width, height: 900 });
    // A gateway first: changing it later drops a shown plan, and without one the start stays disabled.
    await page.locator("input[data-download-gateway]").fill("192.0.2.10:3671");
    await page.getByRole("group", { name: "What to write" }).getByRole("radio", { name: "Parameters only" }).check();
    await page.getByRole("button", { name: "Prepare download" }).click();
    const warning = page.getByRole("note").filter({ hasText: "This scope does not write every pending change:" });
    await expect(warning).toBeVisible();
    await expect(warning).toContainText("Pending group-address changes are not written by this scope.");
    await expect(warning).not.toContainText("Pending parameter changes");
    const box = await warning.boundingBox();
    expect(box!.x + box!.width).toBeLessThanOrEqual(width);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    // A warning, not a refusal (owner decision): the start stays available.
    await expect(page.getByRole("button", { name: "Download to 1.1.67", exact: true })).toBeEnabled();
    await expect(page.locator(".programming-consent-dialog")).toHaveCount(0);
    expect(writes).toEqual([]);
  });
}
