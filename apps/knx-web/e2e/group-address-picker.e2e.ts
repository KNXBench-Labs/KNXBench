/** Issue #8: the link row's searchable group-address picker in the built app. */
import { expect, test } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import type { GroupAddressNode } from "../src/bindings/GroupAddressNode";
import { intercept, openList, openProject } from "./communication-fixture";

const ga = (id: number, address: string, name: string, dpts: string[]): GroupAddressNode =>
  ({ id, address, name, range: null, dpts, links: [] });
const fill = (template: string, values: Record<string, string>) => template.replace(/\{(\w+)\}/g, (_, k: string) => values[k]);

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) for (const theme of ["porcelain", "graphite"]) {
  test(`${language} picker searches, marks DPT differences and links ${theme} ${width}px`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, 8, theme);
    // Object 1 is DPST-1-1 (Switch). One match, one other type, one other subtype, one without a DPT.
    state.tree.installations[0].group_addresses = [ga(500, "1/1/1", "Kitchen light", []),
      ga(501, "1/1/2", "Kitchen dimmer", ["DPST-5-1"]), ga(502, "1/1/3", "Hall switch", ["DPST-1-1"]),
      ga(503, "1/1/4", "Blind up/down", ["DPST-1-8"])];
    const links: unknown[] = [];
    await page.route("**/api/group-links", route => {
      links.push(route.request().postDataJSON());
      return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ ...state.tree, can_undo: true }) });
    });
    await openProject(page, language); await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await panel.getByRole("button", { name: m["comTable.flat"], exact: true }).click();
    const summary = panel.locator('[data-object-id="1"] summary');
    await summary.scrollIntoViewIfNeeded(); await summary.click();
    const editor = panel.locator('[data-editor-id="1"]');
    const picker = editor.getByRole("combobox", { name: m["gaPicker.label"], exact: true });
    await picker.scrollIntoViewIfNeeded(); await picker.click();
    const listbox = editor.getByRole("listbox", { name: m["gaPicker.label"], exact: true });
    await expect(listbox).toBeVisible();
    // Opening brings the whole list into view, also inside the scrolling table.
    await expect(listbox).toBeInViewport({ ratio: 0.98 }); // sub-pixel rounding
    const scrollBox = (await panel.locator(".com-table-scroll").boundingBox())!, listBox = (await listbox.boundingBox())!;
    expect(listBox.y + listBox.height, "list not cut by the table's scroll area").toBeLessThanOrEqual(scrollBox.y + scrollBox.height + 1);
    const options = listbox.getByRole("option");
    await expect(options).toHaveCount(4);
    expect(await options.evaluateAll(nodes => nodes.map(n => [(n as HTMLElement).dataset.gaOption, (n as HTMLElement).dataset.dptAgreement])))
      .toEqual([["500", "unknown"], ["501", "mismatch"], ["502", "match"], ["503", "subtype"]]);
    const switchLabel = language === "en" ? "Switch (1.001)" : "Schalten (1.001)";
    await expect(options.nth(1)).toContainText(fill(m["gaPicker.dptMismatch"], { dpt: switchLabel }));
    await expect(options.nth(3)).toContainText(fill(m["gaPicker.dptSubtype"], { dpt: switchLabel }));
    await expect(options.nth(2)).not.toContainText(fill(m["gaPicker.dptMismatch"], { dpt: switchLabel }));
    // The open list fits the viewport and the page does not scroll sideways.
    const box = (await listbox.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0); expect(box.x + box.width).toBeLessThanOrEqual(width + 1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    if (process.env.KNX_CO_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `picker-${language}-${width}-${theme}.png`) });
    // Typing filters; Enter chooses the highlighted match and closes the list.
    await picker.fill("hall");
    await expect(options).toHaveCount(1);
    await page.keyboard.press("Enter");
    await expect(listbox).toBeHidden();
    await expect(picker).toHaveValue(/1\/1\/3/);
    // Escape closes an open list without leaving the editor.
    await picker.press("ArrowDown"); await expect(listbox).toBeVisible();
    await picker.press("Escape"); await expect(listbox).toBeHidden();
    await expect(editor).toBeVisible();
    await editor.getByRole("button", { name: m["inspector.link"], exact: true }).click();
    await expect.poll(() => links.length).toBe(1);
    expect(links[0]).toMatchObject({ gaId: 502, direction: "Send" });
    await expect(picker).toHaveValue("");
    expect(state.unexpected).toEqual([]);
  });
}
