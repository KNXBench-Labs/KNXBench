/** U21: flow motion in Chromium; Motion Off, OS reduce, Freeze and unmount stop it. */
import { expect, test, type Page } from "@playwright/test";
import { fakeServer, node, openFlow, telegram } from "./telegram-flow-server";

// Counts animation frames that actually ran and intervals that are alive, so
// "stopped" means stopped, not merely hidden by CSS.
async function instrument(page: Page) {
  await page.addInitScript(() => {
    const w = window as unknown as { __frames: number; __intervals: Set<number> };
    w.__frames = 0;
    w.__intervals = new Set();
    const raf = window.requestAnimationFrame.bind(window);
    window.requestAnimationFrame = (callback) => raf((time) => { w.__frames += 1; callback(time); });
    const setI = window.setInterval.bind(window);
    const clearI = window.clearInterval.bind(window);
    window.setInterval = ((handler: TimerHandler, ms?: number) => {
      const id = setI(handler, ms);
      w.__intervals.add(id);
      return id;
    }) as typeof window.setInterval;
    window.clearInterval = ((id?: number) => {
      if (id !== undefined) w.__intervals.delete(id);
      clearI(id);
    }) as typeof window.clearInterval;
  });
}

const frames = (page: Page) => page.evaluate(() => (window as unknown as { __frames: number }).__frames);
const intervals = (page: Page) => page.evaluate(() => (window as unknown as { __intervals: Set<number> }).__intervals.size);

/** Pushes a fresh telegram every 150 ms until stopped. Injected rows take
 * the next sequence number too: the fake server's cursor must never skip. */
function feed(server: Awaited<ReturnType<typeof fakeServer>>, start = 1) {
  let seq = start;
  const timer = setInterval(() => { server.rows.push(telegram(seq, "1", `v${seq}`)); seq += 1; }, 150);
  return {
    stop: () => clearInterval(timer),
    last: () => seq - 1,
    inject: (overrides: Partial<ReturnType<typeof telegram>>) => { server.rows.push({ ...telegram(seq, "1", "x"), ...overrides }); seq += 1; },
  };
}

async function framesStopped(page: Page) {
  await page.waitForTimeout(300);
  const before = await frames(page);
  await page.waitForTimeout(1_000);
  return (await frames(page)) - before;
}

test("draws pulses for live traffic and stops every frame when Motion is switched off mid-flight", async ({ page }) => {
  await instrument(page);
  const server = await fakeServer(page);
  const traffic = feed(server);
  await openFlow(page);
  await expect(page.locator(".flow-pulse").first()).toBeAttached();
  await page.evaluate(() => document.documentElement.setAttribute("data-motion-level", "off"));
  await expect(page.locator(".flow-pulse")).toHaveCount(0);
  expect(await framesStopped(page)).toBe(0);
  await expect(page.locator(".flow-pulse")).toHaveCount(0);
  const latest = traffic.last();
  await expect(node(page, "Dimmer").locator(".flow-badge")).not.toHaveText(`◇ 1/0/1 v${latest - 20}`);
  await expect(page.getByText("Motion is off: the layout stays still")).toBeVisible();
  traffic.stop();
});

test("stops when the OS asks to reduce motion mid-flight", async ({ page }) => {
  await instrument(page);
  const server = await fakeServer(page);
  const traffic = feed(server);
  await openFlow(page);
  await expect(page.locator(".flow-pulse").first()).toBeAttached();
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.locator(".flow-pulse")).toHaveCount(0);
  expect(await framesStopped(page)).toBe(0);
  await expect(page.getByRole("button", { name: "Freeze layout" })).toBeDisabled();
  traffic.stop();
});

test("Freeze keeps every node in place while traffic, pulses and values continue", async ({ page }) => {
  const server = await fakeServer(page);
  const traffic = feed(server);
  await openFlow(page);
  await expect(node(page, "Dimmer")).toBeVisible();
  await page.getByRole("button", { name: "Freeze layout" }).click();
  await expect(page.getByRole("button", { name: "Freeze layout" })).toHaveAttribute("aria-pressed", "true");
  const transforms = () => page.locator("g.flow-node").evaluateAll((nodes) => nodes.map((n) => `${n.getAttribute("data-node-id")}=${n.getAttribute("transform")}`).sort());
  const before = await transforms();
  // A new sender would reheat an unfrozen layout and push the others aside.
  traffic.inject({ source: "1.1.9", sourceRaw: 0x1109, destination: "2/1/7", destinationRaw: 0x1107 });
  await expect(page.locator('g.flow-node[data-node-id="ia:4361"]')).toBeAttached();
  await page.waitForTimeout(1_500);
  const after = (await transforms()).filter((entry) => !entry.startsWith("ia:4361=") && !entry.startsWith("g:4359="));
  expect(after).toEqual(before);
  await expect(page.locator(".flow-pulse").first()).toBeAttached();
  const shown = await node(page, "Dimmer").locator(".flow-badge").textContent();
  await expect(node(page, "Dimmer").locator(".flow-badge")).not.toHaveText(shown!);
  traffic.stop();
});

test("leaving the flow tab stops its frames and its refresh timer", async ({ page }) => {
  await instrument(page);
  const server = await fakeServer(page);
  const traffic = feed(server);
  await page.goto("/e2e/telegram-flow-fixture.html?lang=en&theme=porcelain");
  await expect(page.getByRole("tab", { name: "Flow" })).toBeVisible();
  const withoutFlow = await intervals(page);
  await page.getByRole("tab", { name: "Flow" }).click();
  await expect(page.locator(".flow-pulse").first()).toBeAttached();
  // The animator's refresh timer and, with motion on, its nudge timer.
  expect(await intervals(page)).toBe(withoutFlow + 2);
  await page.getByRole("tab", { name: "Telegrams" }).click();
  expect(await framesStopped(page)).toBe(0);
  expect(await intervals(page)).toBe(withoutFlow);
  traffic.stop();
});

test("with motion off from the start, no frame runs under live traffic and markers stay", async ({ page }) => {
  await instrument(page);
  const server = await fakeServer(page);
  const traffic = feed(server);
  await openFlow(page, "en", "porcelain", "off");
  expect(await page.evaluate(() => document.documentElement.getAttribute("data-motion-level"))).toBe("off");
  await expect(node(page, "Dimmer").locator(".flow-badge")).toBeVisible();
  expect(await framesStopped(page)).toBe(0);
  await expect(page.locator(".flow-pulse")).toHaveCount(0);
  await expect(page.locator(".flow-edge path").first()).toHaveAttribute("marker-end", /url\(#flow-arrow-/);
  traffic.stop();
});

test("with motion off, values still expire on time", async ({ page }) => {
  await page.clock.install();
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page, "en", "porcelain", "off");
  expect(await page.evaluate(() => document.documentElement.getAttribute("data-motion-level"))).toBe("off");
  const badge = node(page, "Dimmer").locator(".flow-badge");
  await expect(badge).toHaveText("◇ 1/0/1 On");
  await page.clock.runFor(7_500);
  await expect(badge).toHaveCount(0);
});
