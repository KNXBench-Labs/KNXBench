/** UX-01: a group address dragged onto a communication object is linked once, API intercepted. */
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";

const device = { id: 42, name: "Kitchen actuator", address: null, description: null, com_object_count: 1 };
const comObject = {
  id: 7, number: 1, name: "Switch output", dpt: "DPST-1-1", dpt_layer: null, description: null,
  description_layer: null, is_active: true, activation: "NotEvaluated", channel: null, program_dpt: null,
  dpt_text: null, function_text: null, read: false, write: true, transmit: false, update: false,
  communication: true, read_on_init: false, links: [] as unknown[],
};

function project(): ProjectTree {
  return {
    schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "ga-drag-fixture", snapshot_revision: 1,
    installations: [{ id: 1, name: "Main", buildings: [], unassigned: [], group_ranges: [],
      topology: [{ id: 2, name: "Area", address: 1, lines: [{ id: 3, name: "Line", address: 1, devices: [device] }] }],
      group_addresses: [{ id: 9, name: "Kitchen lights", address: "1/2/3", range: null, dpts: [], links: [] }] }],
  };
}

async function serve(page: Page, linkStatus: number) {
  const links: unknown[] = [];
  const unexpected: string[] = [];
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    let body: unknown;
    if (path === "/api/project" && method === "GET") {
      body = project();
    } else if (path === "/api/device/42" && method === "GET") {
      body = { id: 42, name: device.name, address: null, description: null, com_objects: [comObject],
        product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" } };
    } else if (path === "/api/group-links" && method === "POST") {
      links.push(request.postDataJSON());
      if (linkStatus !== 200) {
        return route.fulfill({ status: linkStatus, contentType: "application/json", body: '{"error":"link already exists"}' });
      }
      body = { ...project(), can_undo: true, is_modified: true };
    } else {
      unexpected.push(`${method} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
    }
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  });
  // Source and target must both be on screen: Chromium drops a pending drag
  // when the page scrolls while the mouse button is held.
  await page.setViewportSize({ width: 1280, height: 1200 });
  await page.goto("/e2e/installations-fixture.html?workspace");
  await page.locator("button.tree-label", { hasText: "Kitchen actuator" }).click();
  for (const summary of await page.locator(".com-object-groups summary").all()) await summary.click();
  const row = page.locator(".group-link-list .tree-new-row");
  await expect(row).toBeVisible();
  await expect(row).toBeInViewport();
  return { links, unexpected, row };
}

test("links a group address dropped on a communication object, once, in the row's direction", async ({ page }) => {
  const { links, unexpected, row } = await serve(page, 200);
  await row.locator("select").nth(1).selectOption("Receive");
  await page.locator("button.tree-label", { hasText: "1/2/3 Kitchen lights" }).dragTo(row);
  await expect.poll(() => links.length).toBe(1);
  expect(links).toEqual([{ comObjectId: 7, gaId: 9, direction: "Receive" }]);
  expect(unexpected.filter((entry) => !entry.startsWith("GET /api/device/42/parameters"))).toEqual([]);
});

test("shows the server refusal of a dropped link", async ({ page }) => {
  const { links, row } = await serve(page, 400);
  await page.locator("button.tree-label", { hasText: "1/2/3 Kitchen lights" }).dragTo(row);
  await expect(row.locator(".field-error")).toHaveText("link already exists");
  expect(links).toHaveLength(1);
});
