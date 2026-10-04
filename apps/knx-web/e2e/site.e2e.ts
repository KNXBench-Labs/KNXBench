/** EN/DE responsive Ground-root flow; every API request is a local mock. */
import { expect, test } from "@playwright/test";
import { initialTree } from "./site-fixture-data";

for (const language of ["en", "de"] as const) {
  for (const width of [360, 1440]) {
    test(`${language} site hierarchy at ${width}px shares one installation and keeps devices once`, async ({ page }) => {
      await page.setViewportSize({ width, height: 840 });
      const current = structuredClone(initialTree);
      const created: unknown[] = [];
      const moved: unknown[] = [];
      const unmocked: string[] = [];
      await page.route("**/api/**", (route) => {
        const url = new URL(route.request().url());
        const method = route.request().method();
        if (url.pathname === "/api/building-parts" && method === "POST") {
          const body = route.request().postDataJSON();
          created.push(body);
          current.installations[0].buildings.unshift({
            id: 20, name: body.name, kind: body.kind, devices: [], children: [],
          });
        } else if (url.pathname === "/api/move-building-part" && method === "POST") {
          const body = route.request().postDataJSON();
          moved.push(body);
          const buildings = current.installations[0].buildings;
          const [part] = buildings.splice(buildings.findIndex((item) => item.id === body.id), 1);
          buildings.find((item) => item.id === body.parentId)!.children.push(part);
        } else {
          unmocked.push(`${method} ${url.pathname}`);
          return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
        }
        return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(current) });
      });
      await page.goto(`/e2e/site-fixture.html?lang=${language}`);
      await expect(page.locator("html")).toHaveAttribute("lang", language);
      const siteForm = page.locator('[data-structure-create="site-root"]');
      await expect(siteForm).toBeVisible();
      await siteForm.locator("summary").click();
      await expect(siteForm.locator(".structure-create-hint")).toContainText(language === "de" ? "Ground" : "Ground root");
      await expect(siteForm.locator("select")).toHaveCount(0);
      await siteForm.locator("input").fill("Campus");
      await siteForm.locator("button").click();
      await expect(page.locator('.building-diagram > .building-diagram-node')).toHaveCount(3);
      for (const [id, name] of [[4, "North building"], [6, "South building"]] as const) {
        await page.locator(".building-diagram .diagram-heading").filter({ hasText: name }).click();
        await page.locator(".structure-context-editor select").selectOption("20");
        await expect(page.locator('.building-diagram > .building-diagram-node')).toHaveCount(id === 4 ? 2 : 1);
      }
      // MODEL-01: a root create names its installation explicitly; this is the
      // same (only) installation the server's default would have chosen.
      expect(created).toEqual([{ name: "Campus", kind: "Ground", installationId: 1 }]);
      expect(moved).toEqual([{ id: 4, parentId: 20 }, { id: 6, parentId: 20 }]);
      expect(unmocked).toEqual([]);
      await expect(page.locator('.building-diagram .diagram-device')).toHaveCount(2);
      expect(current.installations).toHaveLength(1);
      expect(current.installations[0].topology[0].lines[0].devices.map((device) => device.id)).toEqual([9, 10]);
      const layout = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
      expect(layout.document, JSON.stringify(layout)).toBeLessThanOrEqual(layout.viewport);
    });
  }
}
