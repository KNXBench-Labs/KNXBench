/** Production-built DPT dropdown, localized meanings and lossless edit gestures. */
import { expect, test } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { dismissToasts, intercept, openList, openProject } from "./communication-fixture";
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} DPT selection and contextual source at ${width}px`, async ({ page }) => {
    const m = language === "en" ? en : de;
    const temperature = language === "en" ? "Temperature" : "Temperatur";
    const scene = language === "en" ? "Scene number" : "Szenennummer";
    await page.setViewportSize({ width, height: 1000 });
    const state = await intercept(page, language, 8, width === 1440 ? "graphite" : "porcelain");
    // Unknown source data is deliberately not repaired by rendering or focusing.
    state.objects[0].dpt = "DPST-17-1";
    state.objects[1].dpt = "vendor:opaque"; state.objects[1].program_dpt = null;
    await openProject(page, language);
    await dismissToasts(page, language);
    await expect(page.locator(".toast-stack")).toHaveCount(0);
    await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    await panel.getByRole("button", { name: m["comTable.flat"], exact: true }).click();
    await expect(panel.locator('[data-object-id="1"] .com-object-effective-dpt')).toContainText(`${scene} (17.001)`);
    await panel.locator('[data-object-id="1"] summary').click();
    const field = panel.locator('[data-editor-id="1"] .dpt-field');
    const select = field.getByRole("combobox", { name: "DPT", exact: true });
    await expect(field.locator("input")).toHaveCount(0);
    await expect(select).toHaveValue("DPST-17-1");
    await expect(field.locator(".dpt-reference")).not.toHaveAttribute("open", "");
    await expect(field.locator(".dpt-reference small")).not.toBeVisible();
    await field.locator(".dpt-reference summary").click();
    await expect(field.locator(".dpt-reference small")).toContainText("DPST-17-1");
    await field.locator(".dpt-reference summary").click();
    expect(await select.locator('option[value="DPST-9-1"]').textContent()).toBe(`${temperature} (9.001)`);
    await expect(field.locator(".provenance-badge")).toHaveText(m["valueSource.label"].replace("{source}", m["valueSource.program"]));
    const boxes = await field.evaluate(node => {
      const select = node.querySelector("select")!.getBoundingClientRect();
      const scroll = node.closest(".com-table-scroll")!.getBoundingClientRect();
      const badge = node.querySelector(".provenance-badge")!.getBoundingClientRect();
      return { selectWidth: select.width, scrollWidth: scroll.width, badgeBottom: badge.bottom, selectTop: select.top };
    });
    expect(boxes.selectWidth, "DPT dropdown must fit the local table viewport").toBeLessThanOrEqual(boxes.scrollWidth - 16);
    expect(boxes.badgeBottom).toBeLessThanOrEqual(boxes.selectTop);
    const before = JSON.stringify(state.tree);
    await select.focus(); await select.press("Tab");
    expect(state.requests.filter(r => r === "POST /api/com-object-dpt")).toHaveLength(0);
    expect(JSON.stringify(state.tree)).toBe(before);
    if (process.env.KNX_DPT_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_DPT_CAPTURE_DIR, `dpt-${language}-${width}.png`) });
    await select.focus(); await select.selectOption("DPST-9-1"); await select.press("Tab");
    await expect(panel.locator('[data-object-id="1"] .com-object-effective-dpt')).toContainText(`${temperature} (9.001)`);
    expect(state.objects[0].dpt).toBe("DPST-9-1");
    expect(state.requests.filter(r => r === "POST /api/com-object-dpt")).toHaveLength(1);
    await panel.locator('[data-object-id="2"] summary').click();
    const unknown = panel.locator('[data-editor-id="2"] .dpt-field select');
    await expect(unknown).toHaveValue("vendor:opaque");
    await unknown.focus(); await unknown.press("Tab");
    expect(state.objects[1].dpt).toBe("vendor:opaque");
    expect(state.requests.filter(r => r === "POST /api/com-object-dpt")).toHaveLength(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(state.unexpected).toEqual([]);
  });
}
