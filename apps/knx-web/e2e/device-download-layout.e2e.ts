/** Download configuration stays readable without stretching radios or overlapping fields. */
import { expect, test } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { dismissToasts, intercept, openProject } from "./communication-fixture";

for (const language of ["en", "de"] as const) {
  for (const width of [1440, 400]) {
    for (const theme of ["graphite", "porcelain", "user-modern-retro-green-crt"]) {
      test(`${language} ${width} ${theme}: download fields and compact radios do not overlap`, async ({ page }) => {
        const messages = language === "en" ? en : de;
        await page.setViewportSize({ width, height: 900 });
        const state = await intercept(page, language, 8, theme);
        await openProject(page, language);
        await dismissToasts(page, language);
        await page.keyboard.press("Control+Shift+P");
        await page.getByRole("combobox").fill(messages["toolbar.busMonitor"]);
        await page.getByRole("combobox").press("Enter");
        await page.getByRole("button", { name: messages["deviceDownload.title"], exact: true }).click();
        const panel = page.locator(".device-download-panel");
        await expect(panel).toBeVisible();
        await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
        const group = panel.getByRole("group", { name: messages["deviceDownload.scope.legend"] });
        const radios = group.getByRole("radio");
        await expect(radios).toHaveCount(4);
        if (process.env.KNX_DOWNLOAD_CAPTURE_DIR) {
          await panel.screenshot({ path: join(process.env.KNX_DOWNLOAD_CAPTURE_DIR, `${language}-${width}-${theme}.png`) });
        }
        const geometry = await panel.evaluate((element) => {
          const form = element.querySelector(".device-download-config")!;
          const box = (node: Element | Range) => {
            const { left, right, top, bottom, width, height } = node.getBoundingClientRect();
            return { left, right, top, bottom, width, height };
          };
          return {
            panel: box(element), form: box(form), fields: [...form.children].map(box),
            radios: [...form.querySelectorAll('input[type="radio"]')].map((radio) => {
              const label = radio.closest("label")!;
              const text = [...label.childNodes].find((node) => node.nodeType === Node.TEXT_NODE && node.textContent?.trim())!;
              const range = document.createRange();
              range.selectNodeContents(text);
              return { control: box(radio), label: box(label), text: box(range) };
            }),
            overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
          };
        });
        for (const radio of geometry.radios) {
          expect(radio.control.width, "radio must not inherit text-field sizing").toBeLessThanOrEqual(24);
          expect(radio.control.height, "radio must not inherit text-field padding").toBeLessThanOrEqual(24);
          expect(radio.text.left, "label text belongs beside its radio").toBeGreaterThanOrEqual(radio.control.right);
          expect(radio.control.top).toBeGreaterThanOrEqual(radio.label.top);
          expect(radio.control.bottom).toBeLessThanOrEqual(radio.label.bottom + 1);
        }
        for (let i = 0; i < geometry.fields.length; i++) {
          const a = geometry.fields[i];
          expect(a.left).toBeGreaterThanOrEqual(geometry.form.left - 1);
          expect(a.right).toBeLessThanOrEqual(geometry.form.right + 1);
          for (const b of geometry.fields.slice(i + 1)) {
            const overlapX = Math.min(a.right, b.right) - Math.max(a.left, b.left);
            const overlapY = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
            expect(overlapX > 1 && overlapY > 1, "configuration fields must not overlap").toBe(false);
          }
        }
        expect(geometry.panel.left).toBeGreaterThanOrEqual(0);
        expect(geometry.panel.right).toBeLessThanOrEqual(width);
        expect(geometry.overflow).toBeLessThanOrEqual(1);
        await expect(group.locator('input[value="complete"]')).toBeChecked();
        await group.locator('input[value="parameters"]').focus();
        await page.keyboard.press("Space");
        await expect(group.locator('input[value="parameters"]')).toBeChecked();
        await expect(group.locator('input[value="complete"]')).not.toBeChecked();
        await group.locator('label:has(input[value="both"])').click();
        await expect(group.locator('input[value="both"]')).toBeChecked();
        expect(state.requests.filter((request) => request.startsWith("POST /api/device-download/"))).toEqual([]);
        expect(state.unexpected).toEqual([]);
      });
    }
  }
}
