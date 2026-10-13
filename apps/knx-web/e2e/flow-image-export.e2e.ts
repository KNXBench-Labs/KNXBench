/** Real offline browser downloads: selected SVG pixels, bounded GIF and cancel. */
import { expect, test } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { fakeServer, node, openFlow, telegram } from "./telegram-flow-server";

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`flow images ${language} ${width}: selected-view PNG/GIF and cancellation`, async ({ page }) => {
    test.setTimeout(65000);
    const copy = language === "de" ? de : en;
    // Decoded files go to FLOW_EXPORT_OUTPUT when set, else to this test's own output folder.
    const files = process.env.FLOW_EXPORT_OUTPUT ?? test.info().outputPath("flow-export");
    mkdirSync(files, { recursive: true });
    await page.setViewportSize({ width, height: 950 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    const server = await fakeServer(page); server.rows.push(telegram(1, "1", "On"));
    await openFlow(page, language, width === 1440 ? "porcelain" : "graphite", "off");
    await expect(node(page, "Switch")).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-theme", width === 1440 ? "porcelain" : "graphite");
    await expect(page.locator("html")).toHaveAttribute("data-motion-level", "off");
    const downloads: string[] = []; page.on("download", download => downloads.push(download.suggestedFilename()));
    async function open(format: "png" | "gif") {
      await page.getByRole("button", { name: copy[`flow.export.${format}`], exact: true }).click();
      const panel = page.locator(".flow-export-dialog"); await expect(panel).toBeVisible();
      const box = await panel.boundingBox(); expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
      expect(await panel.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1);
      await expect(panel.locator("img")).toBeVisible();
      await expect.poll(() => panel.locator("img").evaluate(el => (el as HTMLImageElement).complete && (el as HTMLImageElement).naturalWidth > 0)).toBe(true);
      if (width === 1440) {
        const footer = await panel.locator("small").boundingBox();
        expect(box!.y + box!.height - (footer!.y + footer!.height), "compact content-height dialog").toBeLessThanOrEqual(48);
      }
      await expect(panel.locator("[data-export-start]")).toBeDisabled();
      await panel.getByRole("checkbox").check(); return panel;
    }
    let panel = await open("png");
    if (process.env.UX_FEEDBACK_SHOTS) await page.screenshot({ path: join(process.env.UX_FEEDBACK_SHOTS, `flow-image-${language}-${width}.png`) });
    let pending = page.waitForEvent("download"); await panel.locator("[data-export-start]").click();
    let download = await pending; expect(download.suggestedFilename()).toBe("knxbench-flow.png");
    await download.saveAs(join(files, `flow-${language}-${width}.png`));
    await panel.getByRole("button", { name: copy["flow.export.close"], exact: true }).click();
    panel = await open("gif");
    const seconds = panel.getByRole("spinbutton"); await seconds.fill("31"); await expect(panel.locator("[data-export-start]")).toBeDisabled();
    const duration = language === "en" && width === 1440 ? 30 : 3;
    await seconds.fill(String(duration));
    pending = page.waitForEvent("download", { timeout: 40000 }); await panel.locator("[data-export-start]").click();
    download = await Promise.race([pending, panel.getByRole("alert").waitFor({ state: "visible", timeout: 40000 })
      .then(async () => { throw new Error(`Visible export refusal: ${await panel.getByRole("alert").innerText()}`); })]);
    await download.saveAs(join(files, `flow-${language}-${width}-${duration}s.gif`));
    await panel.getByRole("button", { name: copy["flow.export.close"], exact: true }).click();
    panel = await open("gif"); await panel.locator("[data-export-start]").click();
    await expect(panel.getByRole("progressbar")).toBeVisible();
    await panel.getByRole("button", { name: copy["flow.export.cancel"], exact: true }).click();
    await expect(panel.getByRole("alert")).toHaveText(copy["flow.export.cancelled"]);
    await page.waitForTimeout(3200); expect(downloads).toEqual(["knxbench-flow.png", "knxbench-flow.gif"]);
    expect(server.unexpected).toEqual([]);
  });
}

test("live GIF preserves all30 chronological samples with changing graph pixels", async ({ page }) => {
  const files = process.env.FLOW_EXPORT_OUTPUT ?? test.info().outputPath("flow-export");
  mkdirSync(files, { recursive: true });
  const server = await fakeServer(page); server.rows.push(telegram(1, "1", "v1"));
  await openFlow(page); await expect(node(page, "Switch")).toBeVisible();
  await page.getByRole("button", { name: en["flow.export.gif"], exact: true }).click();
  const panel = page.locator(".flow-export-dialog");
  await expect.poll(() => panel.locator("img").evaluate(el => (el as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  await panel.getByRole("checkbox").check(); let seq = 1;
  const timer = setInterval(() => { seq++; server.rows.push(telegram(seq, "1", `v${seq}`)); }, 150);
  try {
    const pending = page.waitForEvent("download"); await panel.locator("[data-export-start]").click();
    await (await pending).saveAs(join(files, "flow-live-3s.gif"));
    clearInterval(timer);
    await expect(node(page, "Dimmer").locator(".flow-badge")).toContainText(`v${seq}`);
    expect(server.rows.map(row => row.seq)).toEqual(Array.from({ length: seq }, (_, index) => index + 1));
    expect(server.unexpected).toEqual([]);
  } finally { clearInterval(timer); }
});
