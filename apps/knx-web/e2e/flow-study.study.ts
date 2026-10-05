/** U19 measurements: frame time, event lag, memory and model size for the synthetic flow study. */
// Run with: npx playwright test -c playwright.study.config.ts
// Results go to $FLOW_STUDY_OUT (JSON) and screenshots next to it. These are
// measurements of one machine and browser, not a supported throughput claim.
import { writeFileSync } from "node:fs";
import { cpus, loadavg } from "node:os";
import { join } from "node:path";
import { test, type Page } from "@playwright/test";

const OUT = process.env.FLOW_STUDY_OUT ?? "/tmp/flow-study";
const results: Record<string, unknown> = {};

interface Metrics {
  admitMs: number[]; layoutMs: number[]; drawMs: number[]; edgeWrites: number;
  frames: number; scriptMs: number[]; intervalsMs: number[]; lagMs: number[]; admitted: number;
  pulsesBundled: number; pulsesOverCapacity: number; backlogPulsesSkipped: number; layoutSteps: number;
}

function stats(values: number[]) {
  const sorted = [...values].sort((a, b) => a - b);
  const at = (q: number) => sorted.length ? Number(sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))].toFixed(2)) : null;
  return { n: sorted.length, p50: at(0.5), p95: at(0.95), p99: at(0.99), max: at(1) };
}

async function heapMb(page: Page): Promise<number> {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Performance.enable");
  const { metrics } = await cdp.send("Performance.getMetrics");
  await cdp.detach();
  return Number(((metrics.find((m) => m.name === "JSHeapUsedSize")?.value ?? 0) / 1048576).toFixed(1));
}

async function measure(page: Page, name: string, query: string, seconds: number) {
  await page.setViewportSize({ width: 1600, height: 1000 });
  await page.goto(`/e2e/flow-study-fixture.html?${query}`);
  await page.waitForFunction(() => (window as unknown as { __flowStudy?: unknown }).__flowStudy !== undefined);
  // Warm-up: the first seconds create the graph; then measure a steady window.
  await page.waitForTimeout(3000);
  const heapBefore = await heapMb(page);
  const loadBefore = loadavg()[0];
  await page.evaluate(() => {
    const m = (window as unknown as { __flowStudy: { metrics: Metrics } }).__flowStudy.metrics;
    m.scriptMs.length = 0; m.intervalsMs.length = 0; m.lagMs.length = 0;
    m.admitMs.length = 0; m.layoutMs.length = 0; m.drawMs.length = 0; m.edgeWrites = 0;
  });
  await page.waitForTimeout(seconds * 1000);
  const snapshot = await page.evaluate(() => {
    const engine = (window as unknown as { __flowStudy: { metrics: Metrics; state: { nodes: Set<string>; edges: Map<string, unknown>; slotCount: number; overflow: unknown } } }).__flowStudy;
    return { metrics: engine.metrics, nodes: engine.state.nodes.size, edges: engine.state.edges.size,
      slots: engine.state.slotCount, overflow: engine.state.overflow, dom: document.getElementsByTagName("*").length };
  });
  const heapAfter = await heapMb(page);
  const loadAfter = loadavg()[0];
  const m = snapshot.metrics;
  results[name] = {
    query, seconds, load1: { before: Number(loadBefore.toFixed(2)), after: Number(loadAfter.toFixed(2)), cores: cpus().length }, nodes: snapshot.nodes, edges: snapshot.edges, liveSlots: snapshot.slots, overflow: snapshot.overflow,
    domElements: snapshot.dom, admittedTotal: m.admitted, frameIntervalMs: stats(m.intervalsMs), scriptMsPerFrame: stats(m.scriptMs),
    eventLagMs: stats(m.lagMs), admitMs: stats(m.admitMs), layoutMs: stats(m.layoutMs), drawMs: stats(m.drawMs),
    edgeWritesPerFrame: Number((m.edgeWrites / Math.max(1, m.scriptMs.length)).toFixed(1)), pulsesBundledTotal: m.pulsesBundled, pulsesOverCapacityTotal: m.pulsesOverCapacity,
    backlogPulsesSkipped: m.backlogPulsesSkipped, layoutSteps: m.layoutSteps, heapMb: { before: heapBefore, after: heapAfter },
  };
}

test.describe.configure({ mode: "serial" });

test("slice screenshots in a light and a dark theme, with motion and without", async ({ page }) => {
  for (const [theme, motion] of [["porcelain", ""], ["graphite", ""], ["graphite", "&motion=off"], ["graphite", "&renderer=canvas"]] as const) {
    await page.setViewportSize({ width: 1240, height: 680 });
    await page.goto(`/e2e/flow-study-fixture.html?theme=${theme}${motion}`);
    await page.waitForTimeout(6000);
    await page.getByRole("list", { name: "Devices and group nodes" }).getByRole("button").first().click();
    await page.waitForTimeout(400);
    await page.screenshot({ path: join(OUT, `slice-${theme}${motion.replace(/[&=]/g, "-")}.png`) });
  }
});

test("target workload: 500 devices, 2,500 edges, 1,000 events per second", async ({ page }) => {
  await measure(page, "target-svg-full", "devices=500&edges=2500&rate=1000&labels=off&width=1200&height=900", 10);
  await measure(page, "target-svg-motion-off", "devices=500&edges=2500&rate=1000&labels=off&width=1200&height=900&motion=off", 10);
  await measure(page, "target-svg-labels", "devices=500&edges=2500&rate=1000&labels=on&width=1200&height=900", 10);
  await measure(page, "target-svg-frozen-geometry", "devices=500&edges=2500&rate=1000&labels=off&width=1200&height=900&freeze=after", 10);
  await measure(page, "target-canvas-full", "devices=500&edges=2500&rate=1000&labels=off&width=1200&height=900&renderer=canvas", 10);
  await measure(page, "target-canvas-motion-off", "devices=500&edges=2500&rate=1000&labels=off&width=1200&height=900&renderer=canvas&motion=off", 10);
});

test("reference loads: small slice and a mid-size map", async ({ page }) => {
  await measure(page, "slice", "devices=14&edges=22&rate=4", 8);
  await measure(page, "mid-svg", "devices=150&edges=600&rate=100&labels=off&width=1200&height=900", 8);
});

test("long session: unique-edge growth up to the model limit", async ({ page }) => {
  await measure(page, "long-growth", "devices=1000&edges=5000&rate=300&labels=off&width=1200&height=900", 30);
  await measure(page, "long-growth-canvas", "devices=1000&edges=5000&rate=300&labels=off&width=1200&height=900&renderer=canvas", 30);
});

test.afterAll(() => {
  writeFileSync(join(OUT, "flow-study-results.json"), JSON.stringify(results, null, 2));
});
