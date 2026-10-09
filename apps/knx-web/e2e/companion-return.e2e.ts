/** Regression coverage for explicit browser return from Diagnostics and Flow. */
import { expect, test, type BrowserContext } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { achievementsFixtureAnswer } from "./achievements-fixture";

async function interceptApi(context: BrowserContext, language = "en") {
  const unexpected: string[] = [];
  const project: ProjectTree = {
    schema_version: 11, errors: 0, warnings: 0, can_undo: false,
    can_redo: false, is_modified: false, group_address_style: "ThreeLevel",
    server_incarnation: "companion-return-fixture", snapshot_revision: 0,
    installations: [],
  };
  await context.route("**/api/**", route => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    let body: unknown;
    if (method === "GET" && path === "/api/auth/status") body = { required: false, authenticated: true };
    else if (method === "GET" && path === "/api/settings") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language } };
    else if (method === "GET" && path === "/api/project") body = project;
    else if (method === "GET" && path === "/api/product-languages") body = [];
    else if (method === "POST" && path === "/api/bus/discover") body = { interfaces: [] };
    else if (method === "GET" && path === "/api/bus/monitor/telegrams") {
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"no bus session"}' });
    } else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else {
      unexpected.push(`${method} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
    }
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  });
  await context.addInitScript(lang => localStorage.setItem("uiLanguage", lang), language);
  return unexpected;
}

for (const language of ["en", "de"] as const) {
  for (const width of [1440, 400]) {
    for (const view of ["diagnostics", "flow"] as const) {
      test(`${language} ${width}px standalone ${view} returns to the editor`, async ({ page, context }) => {
        const unexpected = await interceptApi(context, language);
        await page.setViewportSize({ width, height: 900 });
        await page.goto(`/?view=${view}&source=missing-owner`);
        expect(await page.evaluate(() => window.opener)).toBeNull();
        const back = page.getByRole("button", { name: language === "de" ? "Zurück zum Hauptfenster" : "Back to main window" });
        await expect(back).toBeEnabled();
        const box = await back.boundingBox();
        expect(box).not.toBeNull();
        expect(box!.x).toBeGreaterThanOrEqual(0);
        expect(box!.x + box!.width).toBeLessThanOrEqual(width);
        await back.click();
        await expect(page).toHaveURL("http://127.0.0.1:4173/");
        await expect(page.locator(".companion-shell, .flow-window")).toHaveCount(0);
        await expect(page.locator("main.workbench")).toBeVisible();
        expect(unexpected).toEqual([]);
      });
    }
  }
}

test("Diagnostics with a live opener returns even when opener focus is ignored", async ({ page, context }) => {
  const unexpected = await interceptApi(context);
  await page.goto("/");
  const pending = page.waitForEvent("popup");
  await page.evaluate(() => window.open("/?view=diagnostics", "regression-diagnostics"));
  const popup = await pending;
  await page.evaluate(() => { window.focus = () => {}; });
  expect(await popup.evaluate(() => window.opener !== null && !window.opener.closed)).toBe(true);
  await popup.getByRole("button", { name: "Back to main window" }).click();
  await expect(popup).toHaveURL("http://127.0.0.1:4173/");
  await expect(popup.locator("main.workbench")).toBeVisible();
  expect(unexpected).toEqual([]);
});
