/** Functions against the real server: create, member, stale refusal, undo, save and reopen. */
import { expect, test, type Page } from "@playwright/test";
import { copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";

const data = process.env.KNX_RENAME_NATIVE_DATA!;
const demo = process.env.KNX_RENAME_NATIVE_DEMO!;
if (!data || !demo || process.env.KNX_RENAME_ISOLATED !== "1") throw new Error("Run only through the isolated verification runner.");
async function openCopy(page: Page, m: typeof en | typeof de) {
  await page.getByRole("button", { name: m["workbench.openNativeTitle"], exact: true }).click();
  await page.getByRole("dialog", { name: m["fsPicker.open"], exact: true }).getByRole("button", { name: "functions-demo.knxdb", exact: true }).click();
  await expect(page.locator(".welcome-card")).toHaveCount(0);
}
/** The Kitchen's Functions disclosure in Properties, opened. */
async function kitchenFunctions(page: Page, m: typeof en | typeof de) {
  const explorer = page.locator(".project-explorer");
  if (!await explorer.isVisible()) {
    await page.locator(".workbench-panel-controls").getByRole("button", { name: m["workbench.navigation"], exact: true }).click();
  }
  await explorer.locator("button.tree-label", { hasText: /^Kitchen \(/ }).first().click();
  const inspector = page.locator(".workbench-pane-right .inspector");
  if (!await inspector.isVisible()) {
    await page.locator(".workbench-panel-controls").getByRole("button", { name: m["workbench.properties"], exact: true }).click();
  }
  await expect(inspector.getByRole("heading", { name: "Kitchen", exact: true })).toBeVisible();
  const panel = inspector.locator("details.functions-panel");
  await panel.locator("summary").click();
  return panel;
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} built app/native server Functions at ${width}px`, async ({ page, request }) => {
    test.setTimeout(150_000);
    const m = language === "en" ? en : de;
    const reset = await request.post("/api/project/new", { data: { name: "Isolated reset", discardChanges: true } }); expect(reset.ok()).toBe(true);
    mkdirSync(data, { recursive: true }); copyFileSync(demo, join(data, "functions-demo.knxdb"));
    const settings = await request.put("/api/settings", { data: { settings: {
      uiLanguage: language, theme: language === "en" ? "graphite" : "porcelain", motionLevel: "off", uiScale: 1,
      autosaveEnabled: false, achievementsEnabled: false, onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.6" },
    } } }); expect(settings.ok()).toBe(true);
    const errors: string[] = []; const writes: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("request", (r) => { if (!["GET", "HEAD"].includes(r.method())) writes.push(`${r.method()} ${new URL(r.url()).pathname}`); });
    await page.setViewportSize({ width, height: 900 }); await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/"); await openCopy(page, m);
    let panel = await kitchenFunctions(page, m);
    await expect(panel).toContainText(m["functions.none"]);

    const create = panel.locator("form.functions-details").last();
    await create.getByLabel(m["functions.name"], { exact: true }).fill("Kitchen light 🛠");
    await create.getByRole("button", { name: m["functions.createButton"], exact: true }).click();
    const item = panel.locator("[data-function-id]").first();
    await expect(item).toContainText("Kitchen light 🛠");
    await expect(item).toContainText("KB-Function-");
    await expect(page.getByRole("button", { name: m["toolbar.undo"], exact: true })).toBeEnabled();

    const picker = item.getByRole("combobox", { name: m["gaPicker.label"], exact: true });
    await picker.click(); await picker.press("ArrowDown"); await picker.press("Enter");
    await item.getByRole("button", { name: m["functions.addMember"], exact: true }).click();
    await expect(item.locator("[data-member-id]")).toHaveCount(1);

    // A second client writes while this edit is on its way, so the real API must refuse the stale binding.
    let raced = false;
    await page.route("**/api/project/functions/edit", async (route) => {
      if (!raced) {
        raced = true;
        const current = await (await request.get("/api/project/functions")).json();
        const external = await request.post("/api/project/functions/edit", { data: { serverIncarnation: current.serverIncarnation,
          projectIncarnation: current.projectIncarnation, snapshotRevision: current.snapshotRevision,
          action: { kind: "create", location: current.functions[0].location.buildingPartId, details: { name: "External blind" } } } });
        expect(external.ok()).toBe(true);
      }
      await route.continue();
    });
    await item.getByRole("button", { name: m["functions.editDetails"], exact: true }).click();
    const details = item.locator("form.functions-details");
    await details.locator("label", { hasText: m["functions.completionLabel"] }).locator("select").selectOption("Tested");
    await details.getByRole("button", { name: m["functions.applyDetails"], exact: true }).click();
    await expect(panel.locator(".functions-notice[role=alert]")).toContainText(m["functions.stale"]);
    await expect(details.locator("label", { hasText: m["functions.completionLabel"] }).locator("select")).toHaveValue("Tested");
    await panel.getByRole("button", { name: m["functions.reload"], exact: true }).click();
    await expect(panel.locator("[data-function-id]")).toHaveCount(2);
    expect(raced).toBe(true);
    await page.unroute("**/api/project/functions/edit");
    await details.getByRole("button", { name: m["functions.applyDetails"], exact: true }).click();
    await expect(item.locator("summary .provenance-badge")).toHaveText(m["functions.completion.Tested"]);

    // Nothing inside the panel may stick out of it; the pane would clip it silently.
    const clipped = await panel.evaluate((root) => {
      const edge = root.getBoundingClientRect().right + 1;
      return [...root.querySelectorAll<HTMLElement>("*")].filter((el) => el.getClientRects().length && el.getBoundingClientRect().right > edge)
        .map((el) => `${el.tagName.toLowerCase()}.${el.className}`).slice(0, 8);
    });
    expect(clipped).toEqual([]);
    await item.scrollIntoViewIfNeeded();
    if (process.env.KNX_RENAME_CAPTURE) await page.screenshot({ path: join(process.env.KNX_RENAME_CAPTURE, `functions-${language}-${width}.png`) });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);

    // Undo and Redo are the project's own steps; the panel follows them.
    await page.locator(".workbench-toolbar").getByRole("button", { name: m["toolbar.undo"], exact: true }).click();
    await expect(item.locator("summary .provenance-badge")).toHaveText(m["functions.completion.Undefined"]);
    await page.locator(".workbench-toolbar").getByRole("button", { name: m["toolbar.redo"], exact: true }).click();
    await expect(item.locator("summary .provenance-badge")).toHaveText(m["functions.completion.Tested"]);

    await page.locator('[data-crt-surface="save"]').click();
    await expect.poll(async () => (await (await request.get("/api/project")).json()).is_modified).toBe(false);
    const saved = await (await request.get("/api/project/functions")).json();
    await page.reload(); await openCopy(page, m);
    const reopened = await (await request.get("/api/project/functions")).json();
    expect(reopened.functions).toEqual(saved.functions);
    panel = await kitchenFunctions(page, m);
    await expect(panel.locator("[data-function-id]")).toHaveCount(2);
    await expect(panel.locator("[data-function-id]").first().locator("[data-member-id]")).toHaveCount(1);
    // Same rule as rename.native.ts: discovery is a search; no bus write, download or address write may start.
    expect(writes.filter((w) => /\/api\/(?:bus\/(?:write|monitor\/start|monitor\/stop)|device-download\/(?:start|plan)|device-address\/start)/.test(w))).toEqual([]);
    expect(errors).toEqual([]);
  });
}
