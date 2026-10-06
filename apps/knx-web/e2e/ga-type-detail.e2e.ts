/** KL-61 follow-up in Chromium: the type outcome reads as a muted note, a conflict as a warning. */
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";

function project(): ProjectTree {
  return {
    schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "ga-type-fixture", snapshot_revision: 1,
    installations: [{ id: 1, name: "Main", buildings: [], unassigned: [], group_ranges: [], topology: [],
      group_addresses: [
        { id: 9, name: "Dimmer value", address: "1/0/2", range: null, dpts: ["DPST-5-1"], links: [],
          dpt_detail: { declared: { state: "Value", text: "DPST-5-1" }, linked: ["DPST-5-4"], outcome: "DeclaredDiffersFromLinked" } },
        { id: 10, name: "Broken switch", address: "1/0/3", range: null, dpts: ["DPST-1-1", "DPST-5-1"], links: [],
          dpt_detail: { declared: { state: "Value", text: "DPST-1-1" }, linked: ["DPST-5-1"], outcome: "SizeConflict" } },
      ] }],
  };
}

async function open(page: Page, name: string, theme = "porcelain") {
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/project") return route.fulfill({ contentType: "application/json", body: JSON.stringify(project()) });
    return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
  });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto("/e2e/installations-fixture.html?workspace");
  // The fixture sets no theme, and the colour tokens exist only per theme.
  await page.evaluate((id) => { document.documentElement.dataset.theme = id; }, theme);
  expect(await tokenColor(page, "--knx-muted")).not.toBe(await tokenColor(page, "--knx-warning-color"));
  await page.locator("button.tree-label", { hasText: name }).click();
  await expect(page.locator(".dpt-outcome")).toBeVisible();
}

/** The colour a token resolves to here, measured on a probe element. */
async function tokenColor(page: Page, token: string) {
  return page.evaluate((name) => {
    const probe = document.createElement("span");
    probe.style.color = `var(${name})`;
    document.body.appendChild(probe);
    const color = getComputedStyle(probe).color;
    probe.remove();
    return color;
  }, token);
}

test("the outcome sentence is a muted note under the facts, not body text", async ({ page }) => {
  await open(page, "1/0/2 Dimmer value");
  const note = page.locator(".dpt-outcome");
  const style = await note.evaluate((element) => {
    const own = getComputedStyle(element);
    const body = getComputedStyle(element.closest(".inspector")!);
    return { color: own.color, size: parseFloat(own.fontSize), bodySize: parseFloat(body.fontSize) };
  });
  expect(style.color).toBe(await tokenColor(page, "--knx-muted"));
  expect(style.size).toBeLessThan(style.bodySize);
});

for (const theme of ["porcelain", "graphite"]) test(`${theme}: a size conflict keeps the warning colour on the outcome sentence`, async ({ page }) => {
  await open(page, "1/0/3 Broken switch", theme);
  const color = await page.locator(".dpt-outcome").evaluate((element) => getComputedStyle(element).color);
  expect(color).toBe(await tokenColor(page, "--knx-warning-color"));
  expect(color).not.toBe(await tokenColor(page, "--knx-muted"));
});
