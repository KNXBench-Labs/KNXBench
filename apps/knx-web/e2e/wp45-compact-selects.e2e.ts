/** WP-45 browser contract: compact selects cap at the scalar slot and flag overflow. */
import { expect, test } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { chooseComView, dismissToasts, intercept, openList, openObjectSettings, openProject } from "./communication-fixture";

for (const language of ["en", "de"] as const) {
  test(`${language} DPT select is capped and flags overflow`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 1000 });
    const state = await intercept(page, language, 8, "graphite");
    state.objects[0].dpt = "DPST-17-1";
    await openProject(page, language);
    await dismissToasts(page, language);
    await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    await chooseComView(panel, language === "en" ? en : de, "comTable.flat");
    await panel.locator('[data-object-id="1"] summary').click(); await openObjectSettings(panel, 1);
    const field = panel.locator('[data-editor-id="1"] .dpt-field');
    const select = field.getByRole("combobox", { name: "DPT", exact: true });
    await expect(select).toHaveValue("DPST-17-1");

    // Package 45.1: the select sits inside the compact wrapper and never
    // exceeds the shared 10 rem scalar slot (160 CSS-px at 100% UI scale).
    const box = (await select.boundingBox())!;
    expect(box.width).toBeLessThanOrEqual(160.5);
    expect(box.width).toBeGreaterThan(40);
    // Full value stays reachable through the title tooltip.
    await select.hover();
    await expect(select).toHaveAttribute("title", /Scene number|Szenennummer/);

    // Package 45.2 / WP-48.6: an overflowing value is flagged for the reveal
    // overlay; its geometry, reduced-motion and motion-off behaviour are
    // proven in wp48-compact-select-geometry.e2e.ts.
    const wrapper = field.locator(".select-compact");
    const optionText = await select.locator("option:checked").textContent();
    // "Scene number (17.001)" is far wider than 160 px, so the overflow is flagged.
    expect(optionText!.length).toBeGreaterThan(20);
    await expect(wrapper).toHaveClass(/is-marquee/);

    // Keyboard path unchanged: native select still operable.
    await select.focus();
    await select.selectOption("DPST-9-1");
    await expect(select).toHaveValue("DPST-9-1");
    expect(state.unexpected).toEqual([]);
  });
}
