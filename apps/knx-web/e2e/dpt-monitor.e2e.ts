/** Built-workbench DPT names and deliberate sends with strictly fictional API responses. */
import { expect, test } from "@playwright/test";
import { join } from "node:path";
import type { BusTelegramRow, BusMonitorTelegramsResponse } from "../src/api";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { dismissToasts, intercept, openProject } from "./communication-fixture";

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} named monitor DPT and explicit send at ${width}px`, async ({ page }) => {
    const m = language === "en" ? en : de;
    const temperature = language === "en" ? "Temperature (9.001)" : "Temperatur (9.001)";
    const scene = language === "en" ? "Scene number (17.001)" : "Szenennummer (17.001)";
    const foreign: string[] = [];
    await page.route("**/*", route => {
      const url = new URL(route.request().url());
      if (url.hostname === "127.0.0.1") return route.continue();
      foreign.push(url.origin); return route.abort();
    });
    const state = await intercept(page, language, 8, width === 1440 ? "graphite" : "porcelain");
    await page.route("**/api/version", route => route.fulfill({ json: { version: "0.1.0-alpha.9+gfixture" } }));
    const ga = state.tree.installations[0].group_addresses[0];
    ga.dpts = ["DPST-9-1"];
    ga.dpt_detail = { declared: { state: "Value", text: "DPST-9-1" }, linked: ["DPST-9-1"], outcome: "Declared" };
    await page.route("**/api/settings", route => route.fulfill({ json: { schemaVersion: 1, conditionalPatchVersion: 1,
      status: "ok", settings: { uiLanguage: language, theme: width === 1440 ? "graphite" : "porcelain",
        releaseChecksEnabled: false, onboardingGuide: { seenStage: "alpha" }, motionLevel: "off" } } }));
    const rows: BusTelegramRow[] = ["DPST-9-1", "DPST-09-001"].map((dpt, index) => ({
      seq: index + 1, timestamp: "2026-10-12T06:00:00Z", source: "1.1.2", destination: ga.address,
      destinationName: ga.name, service: "GroupValueWrite", rawPayload: "0x0c33", control: null,
      decoded: { kind: "value", dpt, text: "21 °C" },
    }));
    await page.route("**/api/bus/monitor/telegrams?**", route => {
      const since = Number(new URL(route.request().url()).searchParams.get("since") ?? 0);
      const body: BusMonitorTelegramsResponse = { sessionId: 1, serverIncarnation: "devices-browser",
        contextStatus: "current", projectOpen: true, status: "active", nextSince: 2, droppedBefore: 0,
        telegrams: since === 0 ? rows : [] };
      return route.fulfill({ json: body });
    });
    const writes: unknown[] = [];
    await page.route("**/api/bus/write", route => {
      writes.push(route.request().postDataJSON());
      return route.fulfill({ json: { service: "GroupValueWrite", encodedPayload: "0x01",
        decodedEcho: { kind: "value", dpt: "DPST-17-1", text: "1" } } });
    });
    await page.setViewportSize({ width, height: 1000 });
    await openProject(page, language);
    await dismissToasts(page, language);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
    await page.getByRole("combobox").press("Enter");
    const table = page.locator(".bus-monitor-table");
    await expect(table.locator("tbody tr")).toHaveCount(2);
    await table.locator("tbody tr").first().click();
    await expect(page.locator(".telegram-details")).toContainText(temperature);
    await expect(page.locator(".telegram-details")).not.toContainText("DPST-");
    const select = page.locator("select.bus-compose-dpt");
    await expect(page.locator("input.bus-compose-dpt")).toHaveCount(0);
    await expect(select).toHaveValue("DPST-9-1");
    expect(await select.locator('option[value="DPST-9-1"]').textContent()).toBe(temperature);
    await select.focus(); await select.press("Tab");
    expect(writes).toEqual([]);
    await table.locator("tbody tr").last().click();
    await expect(select).toHaveValue("DPST-09-001");
    expect(await select.locator('option[value="DPST-09-001"]').textContent()).toBe(temperature);
    await select.selectOption("DPST-17-1");
    expect(await select.locator('option[value="DPST-17-1"]').textContent()).toBe(scene);
    await page.locator(".bus-compose-value").fill("1");
    expect(writes).toEqual([]);
    await select.scrollIntoViewIfNeeded();
    if (process.env.KNX_DPT_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_DPT_CAPTURE_DIR, `monitor-${language}-${width}.png`) });
    const bounds = await select.evaluate(node => ({ select: node.getBoundingClientRect().width,
      form: node.closest(".bus-compose-form")!.getBoundingClientRect().width }));
    expect(bounds.select, "DPT selection fits the compose form").toBeLessThanOrEqual(bounds.form - 36);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    await page.getByRole("button", { name: m["busCompose.send"], exact: true }).click();
    await expect(page.locator(".bus-compose-sent")).toBeVisible();
    expect(writes).toEqual([{ destination: ga.address, dpt: "DPST-17-1", value: "1", inputFormat: null }]);
    expect(ga.dpts).toEqual(["DPST-9-1"]);
    expect(state.unexpected).toEqual([]); expect(foreign).toEqual([]);
  });
}
