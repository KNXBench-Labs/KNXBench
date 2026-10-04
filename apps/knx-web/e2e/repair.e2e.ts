/** MODEL-02: ambiguous placements are repaired by an explicit keep choice, API intercepted. */
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";

const twin = { id: 42, name: "Twin device", address: null, description: null, com_object_count: 0 };
const sharedLine = { id: 21, name: "Shared line", address: 1, devices: [] as typeof twin[] };

function project(installation: Partial<ProjectTree["installations"][number]>): ProjectTree {
  return {
    schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "repair-fixture", snapshot_revision: 1,
    installations: [{ id: 1, name: "Main", topology: [], buildings: [], unassigned: [], group_addresses: [],
      group_ranges: [], ...installation }],
  };
}

async function serve(page: Page, initial: ProjectTree, repaired: ProjectTree) {
  const writes: Array<{ path: string; body: unknown }> = [];
  const unexpected: string[] = [];
  let current = initial;
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    let body: unknown;
    if (path === "/api/project" && method === "GET") {
      body = current;
    } else if (path === "/api/device/42" && method === "GET") {
      body = { id: 42, name: "Twin device", address: null, description: null, com_objects: [],
        product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" } };
    } else if (path.startsWith("/api/repair/") && method === "POST") {
      writes.push({ path, body: request.postDataJSON() });
      current = { ...repaired, can_undo: true, is_modified: true };
      body = current;
    } else {
      unexpected.push(`${method} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
    }
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  });
  await page.goto("/e2e/installations-fixture.html");
  return { writes, unexpected };
}

test("keeps a line listed by two areas under the chosen area", async ({ page }) => {
  const { writes, unexpected } = await serve(page,
    project({ topology: [{ id: 20, name: "Area two", address: 2, lines: [sharedLine] },
      { id: 23, name: "Area three", address: 3, lines: [sharedLine] }] }),
    project({ topology: [{ id: 20, name: "Area two", address: 2, lines: [] },
      { id: 23, name: "Area three", address: 3, lines: [sharedLine] }] }));
  const lines = page.locator("button.tree-label", { hasText: "Line 1: Shared line" });
  await expect(lines).toHaveCount(2);
  await lines.first().click();
  await expect(page.getByRole("alert")).toContainText("This structure ID occurs more than once");
  const keep = page.getByRole("button", { name: "Keep under this area" });
  await expect(keep).toHaveCount(2);
  await keep.nth(1).click();
  await expect(lines).toHaveCount(1);
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(writes).toEqual([{ path: "/api/repair/line-owner", body: { lineId: 21, keepAreaId: 23 } }]);
  expect(unexpected).toEqual([]);
});

test("keeps the unassigned placement of a device listed twice", async ({ page }) => {
  const { writes, unexpected } = await serve(page,
    project({ topology: [{ id: 20, name: "Area two", address: 2, lines: [{ ...sharedLine, devices: [twin] }] }],
      unassigned: [twin] }),
    project({ topology: [{ id: 20, name: "Area two", address: 2, lines: [sharedLine] }], unassigned: [twin] }));
  const devices = page.locator("button.tree-label", { hasText: "Twin device" });
  await expect(devices).toHaveCount(2);
  await devices.first().click();
  await expect(page.getByRole("heading", { name: "Placement conflict" })).toBeVisible();
  const keep = page.getByRole("button", { name: "Keep this placement" });
  await expect(keep).toHaveCount(2);
  await keep.nth(1).click();
  await expect(devices).toHaveCount(1);
  await expect(page.getByRole("heading", { name: "Placement conflict" })).toHaveCount(0);
  expect(writes).toEqual([{ path: "/api/repair/device-placement", body: { deviceId: 42, keepUnassignedInstallationId: 1 } }]);
  expect(unexpected).toEqual([]);
});
