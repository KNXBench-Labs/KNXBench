/** Topology device rows label their object count and keep name and count close on every width. */
import { openNavigationSections } from "./disclosure-fixture";
import { expect, test } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";

const LONG_NAME = "Bedroom north wall dimming actuator with sixteen channels and a very long label";
const dev = (id: number, name: string, address: string | null, count: number) =>
  ({ id, name, address, description: null, com_object_count: count });
// Synthetic topology: one empty line, two populated lines, one huge count, one long name.
const tree: ProjectTree = {
  schema_version: 13, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel",
  server_incarnation: "topology-metrics-fixture", snapshot_revision: 1,
  installations: [{
    id: 1, name: "Shared installation", group_addresses: [], group_ranges: [], buildings: [],
    unassigned: [dev(30, "Spare", null, 2)],
    topology: [{ id: 2, name: "Main", address: 1, lines: [
      { id: 3, name: "Empty line", address: 1, devices: [] },
      { id: 4, name: "Lights", address: 2, devices: [
        dev(10, "Hall", "1.2.1", 0), dev(11, "Kitchen", "1.2.2", 1), dev(12, LONG_NAME, "1.2.3", 12345),
      ] },
      { id: 5, name: "Blinds", address: 3, devices: [dev(13, "Shutter", "1.3.1", 8)] },
    ] }],
  }],
};

for (const [language, width, theme] of [
  ["en", 1280, "porcelain"], ["en", 1280, "graphite"],
  ["en", 2560, "porcelain"], ["en", 2560, "graphite"],
  ["en", 700, "porcelain"], ["en", 700, "graphite"],
  ["de", 2560, "porcelain"],
] as const) {
  test(`${language} ${theme} topology metrics are labelled and bounded at ${width}px`, async ({ page }, testInfo) => {
    const m = language === "en" ? en : de;
    const unexpected: string[] = [];
    const errors: string[] = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.setViewportSize({ width, height: 1000 });
    await page.route("**/api/**", route => {
      const path = new URL(route.request().url()).pathname;
      const method = route.request().method();
      let body: unknown;
      if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
      else if (path === "/api/settings" && method === "GET") body = {
        schemaVersion: 1, status: "ok", settings: {
          uiLanguage: language, theme, motionLevel: "normal", onboardingGuide: { seenStage: "alpha" },
        },
      };
      else if (path === "/api/project" && method === "GET") body = tree;
      else if (path === "/api/version" && method === "GET") body = { version: "0.1.0-alpha.9+gfixture" };
      else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
      else if (path === "/api/project/stored-bus-access" && method === "GET") body = { records: [] };
      else if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) body = [];
      else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
      else {
        unexpected.push(`${method} ${path}`);
        return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"strict offline topology fixture"}' });
      }
      return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    });
    await page.goto("/?view=editor");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await openNavigationSections(page);
    await page.locator(".workbench-navigation").getByRole("button", { name: m["workbench.topology"], exact: true }).click();
    const rows = page.locator(".topology-line .diagram-device");
    await expect(rows).toHaveCount(5); // four on lines, one unassigned

    // Every figure says what it counts, with the right plural.
    const unit = language === "en"
      ? { zero: "0 objects", one: "1 object", many: "12345 objects" }
      : { zero: "0 Objekte", one: "1 Objekt", many: "12345 Objekte" };
    const metrics = page.locator(".topology-line .device-object-count");
    const plural = language === "en" ? "objects" : "Objekte";
    await expect(metrics).toHaveText([unit.zero, unit.one, unit.many, `8 ${plural}`, `2 ${plural}`]);

    const measured = await rows.evaluateAll(buttons => buttons.map(button => {
      const name = button.querySelector("strong")!;
      const count = button.querySelector(".device-object-count")!;
      const text = document.createRange();
      text.selectNodeContents(name);
      const textBox = text.getBoundingClientRect();
      const row = button.getBoundingClientRect();
      const countBox = count.getBoundingClientRect();
      return {
        name: name.textContent, rowWidth: row.width, rowLeft: row.left, rowRight: row.right,
        gap: countBox.left - Math.min(textBox.right, name.getBoundingClientRect().right),
        countInside: countBox.left >= row.left && countBox.right <= row.right + 0.5,
        ellipsised: name.scrollWidth > name.clientWidth, title: name.getAttribute("title"),
        countOverlapsName: countBox.left < name.getBoundingClientRect().right - 0.5,
      };
    }));
    const rem = 16;
    for (const row of measured) {
      expect(row.countInside, `${row.name}: count inside its row`).toBe(true);
      expect(row.countOverlapsName, `${row.name}: count does not overlap the name`).toBe(false);
      // A row is bounded by the 28rem line card, whatever the viewport.
      expect(row.rowWidth, `${row.name}: row width at ${width}px`).toBeLessThanOrEqual(28 * rem);
      // So the name-to-number distance can never fill a widescreen.
      expect(row.gap, `${row.name}: name-to-number distance at ${width}px`).toBeLessThanOrEqual(28 * rem);
    }
    const long = measured.find(row => row.name === LONG_NAME)!;
    expect(long.title).toBe(LONG_NAME);
    expect(long.ellipsised, "long names are cut with an ellipsis, not pushed over the count").toBe(true);

    // Lines sit beside each other from the left on a wide screen, not stretched across it.
    const lineBoxes = await page.locator(".topology-line").evaluateAll(
      lines => lines.map(line => { const box = line.getBoundingClientRect(); return { left: box.left, right: box.right, top: box.top, width: box.width }; }));
    for (const box of lineBoxes) expect(box.width).toBeLessThanOrEqual(28 * rem + 2);
    if (width >= 2000) {
      const row = lineBoxes.filter(box => Math.abs(box.top - lineBoxes[0].top) < 2);
      expect(row.length, "wide screens place several lines side by side").toBeGreaterThanOrEqual(3);
    }
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
    expect(overflow).toBeLessThanOrEqual(1);

    // Hierarchy, selection and inspector behaviour are untouched.
    const lineHeading = page.locator(".topology-line .diagram-heading").filter({ hasText: "Lights" });
    await lineHeading.click();
    await expect(lineHeading).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator(".structure-context-editor")).toBeVisible();
    await expect(rows).toHaveCount(5);
    await page.locator(".topology-area").first().screenshot({ path: testInfo.outputPath(`topology-${language}-${theme}-${width}.png`) });
    expect(errors).toEqual([]);
    expect(unexpected).toEqual([]);
  });
}
