/** WP-45 browser contract: compact selects cap at the scalar slot and marquee on hover. */
import { expect, test } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { chooseComView, dismissToasts, intercept, openList, openObjectSettings, openProject } from "./communication-fixture";

for (const language of ["en", "de"] as const) {
  test(`${language} DPT select is capped and hover-marquees on overflow`, async ({ page }) => {
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

    // Package 45.2: hovering a value that overflows adds the marquee class;
    // leaving removes it. (happy-dom cannot measure, the real browser can.)
    const wrapper = field.locator(".select-compact");
    await select.hover();
    const marquee = await wrapper.evaluate((node) => node.classList.contains("is-marquee"));
    const optionText = await select.locator("option:checked").textContent();
    // "Scene number (17.001)" is far wider than 160 px, so the marquee engages.
    expect(optionText!.length).toBeGreaterThan(20);
    expect(marquee, "overflowing value must engage the hover marquee").toBe(true);
    // Leaving (or leaving the field by keyboard) ends the marquee.
    // Pure CSS: once the pointer leaves, the wrapper no longer matches :hover
    // and the animation rule (scoped to .is-marquee) stops applying. Assert
    // the computed animation is none after a real pointer move away.
    // The marquee animation is hover-scoped: once the pointer leaves, the
    // computed animation must be none even though the overflow class remains.
    const box2 = (await select.boundingBox())!;
    await page.mouse.move(box2.x + box2.width + 80, box2.y + 5);
    await expect.poll(() => select.evaluate((node) => getComputedStyle(node).animationName), { timeout: 3000 }).toBe("none");

    // Reduced motion: the animation rule is scoped to no-preference only.
    await page.emulateMedia({ reducedMotion: "reduce" });
    await select.hover();
    const anim = await select.evaluate((node) => getComputedStyle(node).animationName);
    expect(anim).toBe("none");
    await page.emulateMedia({ reducedMotion: "no-preference" });

    // Keyboard path unchanged: native select still operable.
    await select.focus();
    await select.selectOption("DPST-9-1");
    await expect(select).toHaveValue("DPST-9-1");
    expect(state.unexpected).toEqual([]);
  });
}
