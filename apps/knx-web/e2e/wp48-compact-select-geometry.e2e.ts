/** WP-48.6 browser contract: a compact select reveals its full value without moving the control. */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { chooseComView, dismissToasts, intercept, openList, openObjectSettings, openProject } from "./communication-fixture";

interface Box { x: number; y: number; width: number; height: number }

async function openDptSelect(page: Page, language: "en" | "de", width: number) {
  await page.setViewportSize({ width, height: 1000 });
  const state = await intercept(page, language, 8, "graphite");
  state.objects[0].dpt = "DPST-17-1";
  await openProject(page, language);
  await dismissToasts(page, language);
  await openList(page, language);
  await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
  const panel = page.locator(".com-table-panel");
  await chooseComView(panel, language === "en" ? en : de, "comTable.flat");
  await panel.locator('[data-object-id="1"] summary').click();
  await openObjectSettings(panel, 1);
  const field = panel.locator('[data-editor-id="1"] .dpt-field');
  const select = field.getByRole("combobox", { name: "DPT", exact: true });
  await expect(select).toHaveValue("DPST-17-1");
  return { state, field, select, wrapper: field.locator(".select-compact") };
}

/** Boxes of the select, its overlay and every sibling control around the wrapper, in one read. */
function geometry(wrapper: Locator) {
  return wrapper.evaluate((node) => {
    const rect = (el: Element | null): Box | null => {
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return { x: r.x, y: r.y, width: r.width, height: r.height };
    };
    const select = node.querySelector("select")!;
    const overlay = node.querySelector(".select-compact-reveal");
    const text = overlay?.firstElementChild ?? null;
    const style = getComputedStyle(select);
    return {
      wrapper: rect(node), select: rect(select), overlay: overlay ? { ...rect(overlay)!, display: getComputedStyle(overlay).display } : null,
      text: rect(text), textTransform: text ? getComputedStyle(text).transform : "none",
      textAnimation: text ? getComputedStyle(text).animationName : "none", selectColor: style.color,
      neighbours: [...(node.parentElement?.children ?? [])].filter((el) => el !== node).map((el) => rect(el)),
    };
  });
}

for (const language of ["en", "de"] as const) {
  test(`${language} narrow compact select scrolls its text inside a stationary control`, async ({ page }) => {
    const { state, select, wrapper } = await openDptSelect(page, language, 400);
    await expect(wrapper).toHaveClass(/is-marquee/);
    await page.mouse.move(2, 2);
    const rest = await geometry(wrapper);
    expect(rest.select!.width).toBeLessThanOrEqual(160.5);
    expect(rest.overlay?.display, "overlay hidden while idle").toBe("none");
    const fullText = (await select.locator("option:checked").textContent())!;
    await expect(select).toHaveAttribute("title", fullText);

    await select.hover();
    const hoverBox = (await select.boundingBox())!;
    await expect.poll(async () => (await geometry(wrapper)).textAnimation, { timeout: 5000 }).toBe("knx-select-marquee");

    // While the text travels, the control and its neighbours never move; poll until the
    // animation has settled at its end pause (the tail reached the overlay's right edge).
    const distance = parseFloat(await wrapper.evaluate((node) => (node as HTMLElement).style.getPropertyValue("--select-marquee-distance")));
    expect(distance).toBeGreaterThan(0);
    let travelled = 0;
    await expect.poll(async () => {
      const g = await geometry(wrapper);
      expect(g.select).toEqual(rest.select);
      expect(g.wrapper).toEqual(rest.wrapper);
      expect(g.neighbours).toEqual(rest.neighbours);
      // The overlay covers only the text area: inside the select, arrow zone free.
      expect(g.overlay!.display).toBe("flex");
      expect(g.overlay!.x).toBeGreaterThanOrEqual(g.select!.x);
      expect(g.overlay!.x + g.overlay!.width).toBeLessThanOrEqual(g.select!.x + g.select!.width - 20);
      expect(g.selectColor, "native text hidden while the overlay shows it").toBe("rgba(0, 0, 0, 0)");
      travelled = g.overlay!.x - g.text!.x;
      return travelled >= distance - 0.5;
    }, { timeout: 20_000, intervals: [100] }).toBe(true);
    // The whole label has been shown: the text's tail sits at the overlay's right edge.
    expect(Math.abs(travelled - distance)).toBeLessThanOrEqual(0.5);
    expect(fullText.length).toBeGreaterThan(20);
    expect(await select.boundingBox()).toEqual(hoverBox);

    // Leaving restores the idle state: overlay hidden, native text visible again.
    await page.mouse.move(2, 2);
    await expect.poll(async () => (await geometry(wrapper)).overlay?.display ?? "none", { timeout: 3000 }).toBe("none");
    expect((await geometry(wrapper)).selectColor).not.toBe("rgba(0, 0, 0, 0)");
    expect(state.unexpected).toEqual([]);
  });

  test(`${language} keyboard focus reveals the text, reduced motion and motion off keep it static`, async ({ page }) => {
    const { state, select, wrapper } = await openDptSelect(page, language, 400);
    await page.mouse.move(2, 2);
    const rest = await geometry(wrapper);

    // Keyboard: Tab onto the select (focus-visible) starts the reveal; the box stays put.
    await select.evaluate((node) => { (node as HTMLSelectElement).blur(); });
    await page.keyboard.press("Shift+Tab");
    await select.focus();
    await expect(select).toBeFocused();
    await expect.poll(async () => (await geometry(wrapper)).textAnimation, { timeout: 5000 }).toBe("knx-select-marquee");
    expect((await geometry(wrapper)).select).toEqual(rest.select);
    // Still a working native select by keyboard.
    await page.keyboard.press("Tab");
    await select.focus();

    // OS reduced motion: no overlay, native clipped text plus title only.
    await page.emulateMedia({ reducedMotion: "reduce" });
    await select.hover();
    await expect.poll(async () => (await geometry(wrapper)).overlay?.display ?? "none").toBe("none");
    expect((await geometry(wrapper)).selectColor).not.toBe("rgba(0, 0, 0, 0)");
    await expect(select).toHaveAttribute("title", /\S/);
    await page.emulateMedia({ reducedMotion: "no-preference" });

    // In-app motion level "off" behaves the same.
    await page.evaluate(() => document.documentElement.setAttribute("data-motion-level", "off"));
    await select.hover();
    await expect.poll(async () => (await geometry(wrapper)).overlay?.display ?? "none").toBe("none");
    await page.evaluate(() => document.documentElement.removeAttribute("data-motion-level"));

    // A value that fits drops the overlay at the value-change boundary.
    await page.mouse.move(2, 2);
    await select.selectOption("DPST-9-1");
    await expect(wrapper).not.toHaveClass(/is-marquee/);
    expect((await geometry(wrapper)).overlay).toBeNull();
    expect(state.unexpected).toEqual([]);
  });
}
