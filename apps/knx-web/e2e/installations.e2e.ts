/** MODEL-01: a later installation is created into and renamed, every API call intercepted. */

import { expect, test, type Page } from "@playwright/test";
import type { InstallationNode } from "../src/bindings/InstallationNode";
import type { ProjectTree } from "../src/bindings/ProjectTree";

function installation(id: number, name: string): InstallationNode {
  return { id, name, topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [] };
}

async function serve(page: Page) {
  const writes: Array<{ method: string; path: string; body: unknown }> = [];
  const unexpected: string[] = [];
  let current: ProjectTree = {
    schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "installations-fixture", snapshot_revision: 1,
    installations: [installation(1, "Main"), installation(2, "Annex")],
  };
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    let body: unknown;
    if (path === "/api/project" && method === "GET") {
      body = current;
    } else if (path === "/api/areas" && method === "POST") {
      const sent = request.postDataJSON() as { name: string; address: number; installationId?: number };
      writes.push({ method, path, body: sent });
      current = { ...current, can_undo: true, is_modified: true, snapshot_revision: 2,
        installations: current.installations.map((inst) => inst.id === sent.installationId
          ? { ...inst, topology: [...inst.topology, { id: 50, name: sent.name, address: sent.address, lines: [] }] }
          : inst) };
      body = current;
    } else if (path === "/api/installations/2" && method === "PATCH") {
      const sent = request.postDataJSON() as { name: string };
      writes.push({ method, path, body: sent });
      current = { ...current, can_undo: true, is_modified: true, snapshot_revision: 3,
        installations: current.installations.map((inst) => inst.id === 2 ? { ...inst, name: sent.name } : inst) };
      body = current;
    } else {
      unexpected.push(`${method} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
    }
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  });
  return { writes, unexpected };
}

/** The Explorer subtree of one installation, found by its own label. */
function installationItem(page: Page, name: string) {
  return page.locator("button.tree-label", { hasText: new RegExp(`^${name}$`) }).locator("xpath=ancestor::li[1]");
}

test("creates a root area in the second installation and renames that installation", async ({ page }) => {
  const { writes, unexpected } = await serve(page);
  await page.goto("/e2e/installations-fixture.html");
  const annex = installationItem(page, "Annex");
  await expect(annex).toHaveCount(1);

  const areaRow = annex.locator("li.tree-new-row", { has: page.locator('input[placeholder="New area"]') });
  await areaRow.getByLabel("Address").fill("5");
  await areaRow.locator('input[placeholder="New area"]').fill("Annex area");
  await areaRow.getByRole("button", { name: "Add" }).click();
  await expect(annex.locator("button.tree-label", { hasText: "Area 5: Annex area" })).toBeVisible();
  await expect(installationItem(page, "Main").locator("button.tree-label", { hasText: "Annex area" })).toHaveCount(0);

  const second = page.getByLabel("Installation name 2");
  await expect(page.getByLabel("Installation name 1")).toHaveValue("Main");
  await expect(second).toHaveValue("Annex");
  await second.fill("Annex hall");
  await second.press("Enter");
  await expect(installationItem(page, "Annex hall")).toHaveCount(1);

  expect(writes).toEqual([
    { method: "POST", path: "/api/areas", body: { name: "Annex area", address: 5, installationId: 2 } },
    { method: "PATCH", path: "/api/installations/2", body: { name: "Annex hall" } },
  ]);
  expect(unexpected).toEqual([]);
});
