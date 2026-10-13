/** Real-renderer camera interpolation; no data admission or protocol changes. */
import { expect, test } from "@playwright/test";
import { writeFileSync } from "node:fs";
import { loadavg } from "node:os";
import { fakeServer, node, openFlow, telegram } from "./telegram-flow-server";

for (const mode of ["standard", "category-off", "os-reduce"] as const) {
  test(`camera ${mode}: preserves observed rows and respects motion priorities`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion: mode === "os-reduce" ? "reduce" : "no-preference" });
    const server = await fakeServer(page); server.rows.push(telegram(1, "1", "First"));
    await openFlow(page); await expect(node(page, "Switch")).toBeVisible();
    await page.getByRole("button", { name: "Auto zoom", exact: true }).click();
    if (mode === "category-off") await page.evaluate(() => document.documentElement.setAttribute("data-motion-flow", "off"));
    // Wait for a finite prior auto-fit transition, never freeze RAF to prove motion.
    await page.locator(".flow-camera").evaluate(el => Promise.all(el.getAnimations().map(a => a.finished)));
    if (process.env.FLOW_CAMERA_NEGATIVE) await page.addStyleTag({ content: ".flow-camera { transition: none !important; }" });
    const beforeRows = server.rows.map(row => row.seq);
    const camera = page.locator(".flow-camera");
    const before = await camera.evaluate(el => getComputedStyle(el).transform);
    const loadBefore = loadavg();
    const session = await page.context().newCDPSession(page); await session.send("Performance.enable");
    const metricsBefore = await session.send("Performance.getMetrics");
    await page.getByRole("button", { name: "Zoom in", exact: true }).click();
    const samples = await camera.evaluate(async el => {
      const values: { at: number; transform: string }[] = []; const start = performance.now();
      for (let index = 0; index < 20; index++) {
        await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
        values.push({ at: performance.now() - start, transform: getComputedStyle(el).transform });
      }
      return values;
    });
    await camera.evaluate(el => Promise.all(el.getAnimations().map(a => a.finished)));
    const after = await camera.evaluate(el => getComputedStyle(el).transform);
    expect(after).not.toBe(before);
    const intermediate = samples.filter(sample => sample.transform !== before && sample.transform !== after);
    if (mode === "standard") expect(intermediate.length, "camera must interpolate, not jump").toBeGreaterThan(1);
    else expect(intermediate).toEqual([]);
    expect(server.rows.map(row => row.seq)).toEqual(beforeRows);
    server.rows.push(telegram(2, "1", "Second"));
    await expect(node(page, "Dimmer").locator(".flow-badge")).toContainText("Second");
    expect(server.unexpected).toEqual([]);
    const metricsAfter = await session.send("Performance.getMetrics"); await session.detach();
    const totals = Object.fromEntries(["TaskDuration", "ScriptDuration", "LayoutDuration", "RecalcStyleDuration"].map(name => [name,
      (metricsAfter.metrics.find(m => m.name === name)?.value ?? 0) - (metricsBefore.metrics.find(m => m.name === name)?.value ?? 0)]));
    if (process.env.FLOW_CAMERA_OUT) writeFileSync(`${process.env.FLOW_CAMERA_OUT}/${mode}.json`, JSON.stringify({ mode, loadBefore, loadAfter: loadavg(), before, after, samples, totals }, null, 2));
  });
}
