/** U20: the real monitor panel draws intercepted synthetic traffic as a flow, read-only. */
import { expect, test } from "@playwright/test";
import { fakeServer, node, openFlow, telegram } from "./telegram-flow-server";

test("draws senders and configured members, opens the Inspector from the keyboard and writes nothing", async ({ page }) => {
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  await expect(node(page, "Switch")).toBeVisible();
  await expect(node(page, "Dimmer").locator(".flow-badge-inferred")).toHaveText("◇ 1/0/1 On");
  await expect(node(page, "Blind")).toHaveCount(0);
  await node(page, "Dimmer").focus();
  await page.keyboard.press("Enter");
  const inspector = page.locator(".flow-inspector");
  await expect(inspector.locator("h3")).toHaveText("Dimmer");
  await expect(inspector).toContainText("configured member (inferred from the project)");
  await expect(inspector).toContainText("From Switch · configured in the project");
  await page.keyboard.press("ArrowRight");
  await expect(node(page, "Switch")).toBeFocused();
  expect(server.unexpected).toEqual([]);
  expect(server.snapshots).toEqual(["1:1"]);
});

test("expires a value seven seconds after it was observed and keeps the map", async ({ page }) => {
  await page.clock.install();
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  const badge = node(page, "Dimmer").locator(".flow-badge");
  await expect(badge).toHaveText("◇ 1/0/1 On");
  await page.clock.runFor(6_000);
  await expect(badge).toHaveCount(1);
  await page.clock.runFor(1_500);
  await expect(badge).toHaveCount(0);
  await expect(node(page, "Dimmer")).toBeVisible();
});

test("a read neither sets nor renews a value", async ({ page }) => {
  await page.clock.install();
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  const badge = node(page, "Dimmer").locator(".flow-badge");
  await expect(badge).toHaveText("◇ 1/0/1 On");
  await page.clock.runFor(5_000);
  server.rows.push(telegram(2, "1", "", "GroupValueRead"));
  await page.clock.runFor(1_000);
  await page.clock.runFor(1_500);
  await expect(badge).toHaveCount(0);
});

test("resolves each telegram with its own generation after a project change", async ({ page }) => {
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  await expect(node(page, "Dimmer")).toBeVisible();
  server.generation = "2";
  server.rows.push(telegram(2, "2", "Off"));
  await expect(node(page, "Blind").locator(".flow-badge-inferred")).toHaveText("◇ 1/0/1 Off");
  await expect(node(page, "Dimmer")).toBeVisible();
  await node(page, "Dimmer").click();
  await expect(page.locator(".flow-inspector")).toContainText("1/0/1: observed 1× (project evidence from interpretation 1)");
  expect(server.snapshots).toEqual(["1:1", "1:2"]);
});

test("starts a new map when another session answers", async ({ page }) => {
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  await expect(node(page, "Switch")).toBeVisible();
  server.sessionId = 2;
  server.rows = [];
  await expect(page.getByText("No group telegrams observed in this session yet.")).toBeVisible();
  await expect(node(page, "Switch")).toHaveCount(0);
});

test("fits a narrow window in German", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 800 });
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "Ein"));
  await openFlow(page, "de");
  await expect(node(page, "Dimmer").locator(".flow-badge-inferred")).toHaveText("◇ 1/0/1 Ein");
  await expect(page.getByRole("button", { name: "Vergrößern" })).toBeVisible();
  const layout = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
  expect(layout.document, JSON.stringify(layout)).toBeLessThanOrEqual(layout.viewport);
});

test("draws lines and text in the theme's colours, and follows a theme change", async ({ page }) => {
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page, "en", "porcelain");
  const edge = page.locator(".flow-edge path").first();
  await expect(edge).toBeVisible();
  const read = () => page.evaluate(() => {
    const path = document.querySelector(".flow-edge path")!;
    const muted = getComputedStyle(document.documentElement).getPropertyValue("--knx-muted").trim();
    const probe = document.createElement("span");
    probe.style.color = muted;
    document.body.append(probe);
    const expected = getComputedStyle(probe).color;
    probe.remove();
    return { stroke: getComputedStyle(path).stroke, expected, halo: getComputedStyle(document.querySelector(".flow-badge")!).paintOrder };
  });
  const light = await read();
  expect(light.stroke).toBe(light.expected);
  expect(light.halo).toContain("stroke");
  await page.evaluate(() => { document.documentElement.dataset.theme = "graphite"; });
  const dark = await read();
  expect(dark.stroke).toBe(dark.expected);
  expect(dark.stroke).not.toBe(light.stroke);
});
