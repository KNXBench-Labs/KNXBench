/** Structure headings keep their icons and geometry on real mouse hover (issue #5). */
import { openNavigationSections } from "./disclosure-fixture";
import { expect, test, type Locator, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

// Fictional Ground/Building/Floor hierarchy matching the reported layout.
const tree: ProjectTree = {
  schema_version: 13, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel",
  server_incarnation: "heading-hover-fixture", snapshot_revision: 1,
  installations: [{
    id: 1, name: "Shared installation", unassigned: [], group_addresses: [], group_ranges: [],
    buildings: [{ id: 4, name: "PT Home", kind: "Ground", devices: [], children: [
      { id: 5, name: "Haus", kind: "Building", devices: [], children: [
        { id: 6, name: "Erdgeschoss", kind: "Floor", devices: [], children: [] },
        { id: 7, name: "Obergeschoss", kind: "Floor", devices: [], children: [] },
      ] },
    ] }],
    topology: [{ id: 2, name: "Area", address: 1, lines: [
      { id: 3, name: "Line", address: 1, devices: [
        { id: 8, name: "Example sensor", address: "1.1.1", description: null, com_object_count: 0 },
      ] },
    ] }],
  }],
};

async function geometry(heading: Locator) {
  return heading.evaluate(async element => {
    await Promise.all(element.getAnimations().map(animation => animation.finished));
    const rect = (node: Element) => {
      const box = node.getBoundingClientRect();
      return { x: box.x, y: box.y, width: box.width, height: box.height };
    };
    const text = [...element.childNodes].find(node => node.nodeType === Node.TEXT_NODE && node.textContent?.trim());
    if (!text) throw new Error("Heading has no label text");
    const range = document.createRange();
    range.selectNodeContents(text);
    const label = range.getBoundingClientRect();
    const style = getComputedStyle(element);
    return {
      button: rect(element),
      icon: element.querySelector("svg") ? rect(element.querySelector("svg")!) : null,
      label: { x: label.x, y: label.y, width: label.width, height: label.height },
      transform: style.transform, shadow: style.boxShadow, background: style.backgroundColor,
    };
  });
}

async function assertStationaryHover(page: Page, heading: Locator) {
  // Let Playwright finish any actionability scrolling before the baseline.
  await heading.hover();
  // Put the pointer outside the target before taking the unhovered baseline.
  await page.mouse.move(0, 0);
  const before = await geometry(heading);
  await heading.hover();
  const after = await geometry(heading);
  expect(after.transform, "Heading must not inherit generic button magnification").toBe("none");
  expect(after.shadow, "Heading must keep the flat row highlight").toBe("none");
  expect(after.button).toEqual(before.button);
  expect(after.icon).toEqual(before.icon);
  expect(after.label).toEqual(before.label);
  expect(after.background, "Hover still highlights the clickable heading").not.toBe(before.background);
  if (after.icon) {
    expect(after.icon.width).toBeGreaterThan(0);
    expect(after.icon.x).toBeGreaterThanOrEqual(after.button.x);
    expect(after.icon.x + after.icon.width).toBeLessThanOrEqual(after.button.x + after.button.width);
    expect(after.icon.y).toBeGreaterThanOrEqual(after.button.y);
    expect(after.icon.y + after.icon.height).toBeLessThanOrEqual(after.button.y + after.button.height);
  }
}

for (const language of ["en", "de"] as const) {
  for (const width of [400, 1440]) {
    for (const theme of ["porcelain", "graphite"] as const) {
      test(`${language} ${theme} structure headings stay put at ${width}px`, async ({ page }, testInfo) => {
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
          else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(tree);
          else if (path === "/api/version" && method === "GET") body = { version: "0.1.0-alpha.9+gfixture" };
          else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
          else if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) body = [];
          else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
          else {
            unexpected.push(`${method} ${path}`);
            return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"strict offline hover fixture"}' });
          }
          return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
        });
        await page.goto("/?view=editor");
        await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
        await openNavigationSections(page);
        const navigation = page.locator(".workbench-navigation");
        await navigation.getByRole("button", { name: m["workbench.buildings"], exact: true }).click();
        const headings = page.locator(".building-diagram .diagram-heading");
        await expect(headings).toHaveCount(4);
        await expect(headings.locator("svg")).toHaveCount(4);
        for (const heading of await headings.all()) await assertStationaryHover(page, heading);
        const root = headings.filter({ hasText: "PT Home" });
        await root.hover();
        await page.locator(".building-diagram").screenshot({
          path: testInfo.outputPath("hovered-building-headings.png"),
        });
        // Selection and keyboard focus remain functional; neither writes project data.
        await root.click();
        // App intentionally scopes the workspace to the clicked building part.
        await expect(page.locator(".structure-workspace .workspace-heading h1")).toHaveText("PT Home");
        await expect(page.locator(".structure-context-editor")).toBeVisible();
        await page.mouse.move(0, 0);
        const house = headings.filter({ hasText: "Haus" });
        await house.focus();
        await expect(house).toBeFocused();
        await page.keyboard.press("Enter");
        await expect(page.locator(".structure-workspace .workspace-heading h1")).toHaveText("Haus");
        await navigation.getByRole("button", { name: m["workbench.topology"], exact: true }).click();
        const topology = page.locator(".topology-area .diagram-heading");
        await expect(topology).toHaveCount(2);
        await expect(topology.locator("svg")).toHaveCount(1);
        for (const heading of await topology.all()) await assertStationaryHover(page, heading);
        const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
        expect(overflow).toBeLessThanOrEqual(1);
        expect(errors).toEqual([]);
        expect(unexpected).toEqual([]);
      });
    }
  }
}
