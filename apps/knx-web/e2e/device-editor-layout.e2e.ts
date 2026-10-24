/** Checks the real device editor at narrow and wide widths using only a local mocked API. */
import { test, expect, type Page } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";

const translations = {
  en: ["Read", "Write", "Transmit", "Update", "Communication", "Read on init"],
  de: ["Lesen", "Schreiben", "Übertragen", "Aktualisieren", "Kommunikation", "Lesen bei Initialisierung"],
} as const;

for (const language of ["en", "de"] as const) {
  for (const width of [360, 640, 1440]) {
    test(`${language} device editor keeps long labels and addresses visible at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      const writes: unknown[] = [];
      await page.route("**/api/**", (route) => route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({ programId: null, sections: [], stale: [], diagnostics: [] }),
      }));
      await page.route("**/api/individual-address", (route) => {
        writes.push(route.request().postDataJSON());
        return route.fulfill({ contentType: "application/json", body: "{}" });
      });
      await page.goto(`/e2e/device-editor-fixture.html?lang=${language}`);
      await expect(page.locator(".individual-address-field input")).toHaveValue("12");
      await expect(page.locator(".com-object-channel-summary")).toContainText(
        language === "de" ? "Ohne ausgewerteten Kanal" : "Without evaluated channel",
      );
      await page.locator(".com-object-channel-summary").click();
      await page.locator(".com-object-detail summary").click();
      await expect(page.locator(".com-object-flags .flag-name")).toHaveText([...translations[language]]);
      await expect(page.locator(".com-object-flags .flag-code")).toHaveText(["R", "W", "T", "U", "C", "I"]);
      await expect(page.locator(".group-link-row")).toHaveCount(2);
      const layout = await page.evaluate(() => {
        const flags = document.querySelector<HTMLElement>(".com-object-flags")!;
        const clipped = Array.from(document.querySelectorAll<HTMLElement>(
          ".device-tabs button, .com-object-flags label, .com-object-summary strong, .group-link-row, .individual-address-field",
        )).filter((element) => element.scrollWidth > element.clientWidth + 1)
          .map((element) => `${element.className || element.tagName}: ${element.scrollWidth}/${element.clientWidth}`);
        return {
          documentWidth: document.documentElement.scrollWidth,
          viewportWidth: innerWidth,
          flagColumns: getComputedStyle(flags).gridTemplateColumns.split(" ").length,
          clipped,
          offscreen: Array.from(document.querySelectorAll<HTMLElement>("body *"))
            .filter((element) => {
              const rect = element.getBoundingClientRect();
              return !element.closest(".com-table-scroll") && rect.width > 0 && rect.right > innerWidth + 1;
            })
            .slice(0, 12)
            .map((element) => ({ name: `${element.tagName}.${element.className}`, parent: element.parentElement?.className, text: element.textContent?.trim().slice(0, 45), right: Math.ceil(element.getBoundingClientRect().right) })),
        };
      });
      expect(layout.documentWidth, JSON.stringify(layout)).toBeLessThanOrEqual(layout.viewportWidth);
      expect(layout.flagColumns).toBe(2);
      expect(layout.clipped).toEqual([]);
      expect(layout.offscreen).toEqual([]);

      await page.locator(".individual-address-field input").fill("21");
      await page.locator(".individual-address-field input").press("Tab");
      await expect.poll(() => writes.length).toBe(1);
      expect(writes).toEqual([{ deviceId: 42, address: "1.1.21" }]);
    });
  }
}

for (const language of ["en", "de"] as const) {
  for (const width of [360, 1440]) {
    test(`${language} evaluated channel evidence stays keyboard-accessible at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      await page.route("**/api/**", (route) => route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({ programId: "A-MOCK", sections: [], stale: [], diagnostics: [] }),
      }));
      await page.goto(`/e2e/device-editor-fixture.html?lang=${language}&channels=1`);
      const groups = page.locator(".com-object-channel");
      await expect(groups).toHaveCount(3, { timeout: 2_000 });
      await expect(groups.nth(0).locator(".com-object-channel-summary > strong")).toHaveText("Raw manufacturer name");
      await expect(groups.nth(0).locator(".com-object-channel-number")).toHaveText(language === "de" ? "Nummer: A-5" : "Number: A-5");
      await expect(groups.nth(1).locator(".com-object-channel-summary > strong")).toHaveText("Hall outputs");
      await expect(groups.nth(1).locator(".com-object-channel-name")).toHaveText("Name: Manufacturer output");
      await expect(groups.nth(1).locator(".com-object-channel-number")).toHaveText(language === "de" ? "Nummer: 19" : "Number: 19");
      await expect(groups.nth(2).locator(".com-object-channel-summary")).toContainText(
        language === "de" ? "Ohne ausgewerteten Kanal" : "Without evaluated channel",
      );
      expect(await groups.locator("button").evaluateAll(nodes => nodes.every(node => node.getAttribute("aria-expanded") === "false"))).toBe(true);

      await groups.nth(0).locator(".com-object-channel-summary").focus();
      await page.keyboard.press("Enter");
      await expect(groups.nth(0).locator("button")).toHaveAttribute("aria-expanded", "true");
      const programObject = page.locator('[data-object-id="8"]');
      await programObject.locator("summary").click();
      // Name-first DPT presentation (docs/contracts/dpt-selection.md, 2026-10-12
      // amendment): the cell shows "Name (main.sub)"; the exact reference lives
      // in the tooltip, never inline.
      const programDpt = programObject.locator(".com-object-effective-dpt");
      await expect(programDpt.locator(":scope > span")).toHaveText(
        language === "de" ? "Temperatur (9.001)" : "Temperature (9.001)",
      );
      await expect(programDpt).toHaveAttribute("title", "DPST-9-1");
      await expect(programDpt).not.toContainText("DPST");
      await expect(programObject.locator(".com-object-dpt-origin")).toContainText(
        language === "de" ? "Programm-Standardwert" : "Program default",
      );
      // The program's own master text stays separately labelled, untranslated.
      await expect(programDpt.locator(":scope > small:not(.com-object-dpt-origin)")).toHaveText(
        language === "de" ? "Stammdaten: Temperature" : "Master data: Temperature",
      );
      const dptFacts = programObject.locator(
        ".com-object-effective-dpt > span, .com-object-effective-dpt > small:not(.com-object-dpt-origin)",
      );
      await expect(dptFacts).toHaveCount(2); // Name-first label and separately labelled master text.
      const dptTokenLineCounts = await dptFacts.evaluateAll((elements) => elements.flatMap((element) => {
        const node = element.firstChild;
        if (!node || node.nodeType !== Node.TEXT_NODE) throw new Error("Missing DPT fact text");
        const value = node.textContent!;
        // Labels now contain several words. Natural word wrapping is fine;
        // splitting an identifier or any word into character fragments is not.
        return [...value.matchAll(/\S+/g)].map(token => {
          const range = document.createRange();
          range.setStart(node, token.index!);
          range.setEnd(node, token.index! + token[0].length);
          return range.getClientRects().length;
        });
      }));
      expect(dptTokenLineCounts.length).toBeGreaterThan(0);
      expect(dptTokenLineCounts.every(count => count === 1),
        "DPT labels and master-text words must not wrap at character level").toBe(true);

      await groups.nth(1).locator(".com-object-channel-summary").click();
      await expect(page.locator('[data-object-id="7"]')).toContainText("Switching lights");
      const switchDpt = page.locator('[data-object-id="7"] .com-object-effective-dpt');
      await expect(switchDpt.locator(":scope > span")).toHaveText(
        language === "de" ? "Schalten (1.001)" : "Switch (1.001)",
      );
      await expect(switchDpt).toHaveAttribute("title", "DPST-1-1");
      await groups.nth(2).locator(".com-object-channel-summary").click();
      await expect(page.locator("tr[data-object-id][data-activation='Inactive']:visible")).toHaveCount(1);
      await expect(page.locator("tr[data-object-id][data-activation='Undetermined']:visible")).toHaveCount(1);
      await expect(page.locator("tr[data-object-id][data-activation='NotEvaluated']:visible")).toHaveCount(1);
      const widthState = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
      expect(widthState.document, JSON.stringify(widthState)).toBeLessThanOrEqual(widthState.viewport);
    });
  }
}

// Diagnostics-tab entry (24 October 2026): plain buttons that open the existing dialogs with
// this device preselected. Strict routing: opening either dialog must request nothing new.
async function strictParameterRoutes(page: Page): Promise<string[]> {
  const unexpected: string[] = [];
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (request.method() === "GET" && path === "/api/device/42/parameters") {
      return route.fulfill({ contentType: "application/json", body: JSON.stringify({ programId: null, sections: [], stale: [], diagnostics: [] }) });
    }
    unexpected.push(`${request.method()} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "strict diagnostics-entry fixture" }) });
  });
  return unexpected;
}

for (const language of ["en", "de"] as const) {
  for (const [width, theme] of [[360, "graphite"], [1440, "porcelain"]] as const) {
    test(`${language} diagnostics tab opens analysis and report for this device at ${width}px (${theme})`, async ({ page }) => {
      const m = language === "de" ? de : en;
      await page.setViewportSize({ width, height: 800 });
      const unexpected = await strictParameterRoutes(page);
      await page.goto(`/e2e/device-editor-fixture.html?lang=${language}&theme=${theme}`);
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await page.getByRole("tab", { name: m["parameters.diagnosticsTab"], exact: true }).click();
      const panel = page.locator("#device-panel-42-3");
      const analyse = panel.getByRole("button", { name: m["deviceDiagnostics.analyse"], exact: true });
      const report = panel.getByRole("button", { name: m["deviceDiagnostics.report"], exact: true });
      await expect(analyse).toBeEnabled();
      await expect(report).toBeEnabled();
      // Outside the File menu: no menu roles, no menu-entry marker.
      await expect(panel.locator("[role=menuitem], [data-file-menu-item]")).toHaveCount(0);
      for (const button of [analyse, report]) {
        const box = (await button.boundingBox())!;
        expect(box.x).toBeGreaterThanOrEqual(0);
        expect(box.x + box.width).toBeLessThanOrEqual(width + 1);
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
      const capture = process.env.KNX_DIAGNOSTICS_ENTRY_CAPTURE_DIR;
      if (capture) await panel.screenshot({ path: join(capture, `entry-${language}-${width}-${theme}.png`) });

      // Real keyboard path, not programmatic focus(): Chromium shows :focus-visible for a
      // script focus only when the last interaction was a key press, and the tab was clicked.
      // From the selected tab, Tab reaches the focusable tab panel, then the first entry button.
      const focused = () => page.evaluate(() => {
        const active = document.activeElement as HTMLElement | null;
        const style = active ? getComputedStyle(active) : null;
        return { id: active?.id ?? "", analyse: active?.hasAttribute("data-device-analyse") ?? false,
          report: active?.hasAttribute("data-device-report") ?? false, visible: active?.matches(":focus-visible") ?? false,
          outline: style ? `${style.outlineStyle} ${style.outlineWidth}` : "" };
      });
      await page.keyboard.press("Tab");
      expect((await focused()).id).toBe("device-panel-42-3");
      await page.keyboard.press("Tab");
      const onAnalyse = await focused();
      expect(onAnalyse).toMatchObject({ analyse: true, visible: true });
      expect(onAnalyse.outline).not.toMatch(/^none|\b0px$/);
      if (capture) await panel.screenshot({ path: join(capture, `entry-focus-${language}-${width}-${theme}.png`) });
      await page.keyboard.press("Enter");
      const analysis = page.getByRole("dialog", { name: m["diagnostics.title"], exact: true });
      await expect(analysis).toBeVisible();
      await expect(analysis.locator("[data-diagnostics-device]")).toHaveValue("42");
      const dialogBox = (await analysis.boundingBox())!;
      expect(dialogBox.x).toBeGreaterThanOrEqual(0);
      expect(dialogBox.x + dialogBox.width).toBeLessThanOrEqual(width + 1);
      if (capture) await page.screenshot({ path: join(capture, `analysis-${language}-${width}-${theme}.png`) });
      await page.keyboard.press("Escape");
      await expect(analysis).toHaveCount(0);
      expect(await focused()).toMatchObject({ analyse: true, visible: true });

      await page.keyboard.press("Tab");
      const onReport = await focused();
      expect(onReport).toMatchObject({ report: true, visible: true });
      expect(onReport.outline).not.toMatch(/^none|\b0px$/);
      await page.keyboard.press("Space");
      const support = page.getByRole("dialog", { name: m["support.title"], exact: true });
      await expect(support).toBeVisible();
      await expect(support.locator("[data-support-device]")).toHaveValue("42");
      if (capture) await page.screenshot({ path: join(capture, `report-${language}-${width}-${theme}.png`) });
      await support.getByRole("button", { name: m["support.close"], exact: true }).click();
      await expect(support).toHaveCount(0);
      expect(unexpected).toEqual([]);
    });
  }
}
