/** KL-142 in a browser: the download scope is chosen, sent, and its omissions shown first. */
import { expect, test, type Page } from "@playwright/test";

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
  accessKey: "none", support: { level: "verified", evidence: "MDT, 2026-09-29" }, untestedAcknowledgement: null,
  partial, notWritten: partial ? [[0x4100, 12]] : [],
});

async function open(page: Page) {
  const planBodies: unknown[] = [];
  const writes: string[] = [];
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    const json = (body: unknown, status = 200) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status") return json({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return json({ schemaVersion: 1, status: "ok", settings: {} });
    if (path === "/api/project" && method === "GET") return json(tree);
    if (path === "/api/project/new" && method === "POST") return json(tree);
    if (path === "/api/bus/discover") return json({ interfaces: [] });
    if (path === "/api/device-download/plan" && method === "POST") {
      const body = request.postDataJSON() as { address: string; partial?: { parameters: boolean; groupAddresses: boolean } };
      planBodies.push(body);
      if (body.partial && !body.partial.parameters) {
        return json({ error: "no partial download to device 1.1.67 prepared: the complete download allocates no application program task segment" }, 422);
      }
      return json(plan(body.partial !== undefined));
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
  await page.getByRole("navigation", { name: "Bus monitor" }).getByRole("button", { name: "Bus monitor" }).first().click();
  await page.getByRole("button", { name: "Download to device", exact: true }).click();
  await page.getByRole("combobox").filter({ hasText: "1.1.67" }).selectOption("1.1.67");
  return { planBodies, writes };
}

test("a parameters-only scope is sent as such and the plan names what it leaves out, before any consent", async ({ page }) => {
  const { planBodies, writes } = await open(page);
  const scope = page.getByRole("group", { name: "What to write" });
  await expect(scope.getByRole("radio")).toHaveCount(4);
  await expect(scope.getByRole("radio", { name: "Complete download" })).toBeChecked();
  await scope.getByRole("radio", { name: "Parameters only" }).check();
  await page.getByRole("button", { name: "Show what would be written" }).click();
  const shown = page.getByRole("region", { name: "Download plan" });
  await expect(shown).toContainText("Partial download: parameters only.");
  await expect(shown.locator(".device-download-not-written tbody tr")).toHaveText(["4100h12"]);
  expect(planBodies).toEqual([{ address: "1.1.67", partial: { parameters: true, groupAddresses: false } }]);
  await scope.getByRole("radio", { name: "Complete download" }).check();
  await expect(shown).toHaveCount(0);
  await page.getByRole("button", { name: "Show what would be written" }).click();
  await expect(page.getByRole("region", { name: "Download plan" })).not.toContainText("Partial download");
  expect(planBodies[1]).toEqual({ address: "1.1.67" });
  await expect(page.locator(".programming-consent-dialog")).toHaveCount(0);
  expect(writes).toEqual([]);
});

test("a scope the server cannot derive is refused visibly and leaves no plan to start", async ({ page }) => {
  const { planBodies, writes } = await open(page);
  await page.getByRole("group", { name: "What to write" }).getByRole("radio", { name: "Group addresses only" }).check();
  await page.getByRole("button", { name: "Show what would be written" }).click();
  await expect(page.getByRole("alert")).toContainText("no partial download to device 1.1.67 prepared");
  await expect(page.getByRole("region", { name: "Download plan" })).toHaveCount(0);
  expect(planBodies).toEqual([{ address: "1.1.67", partial: { parameters: false, groupAddresses: true } }]);
  expect(writes).toEqual([]);
});
