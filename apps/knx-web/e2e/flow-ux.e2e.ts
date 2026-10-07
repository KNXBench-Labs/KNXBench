/** Exercises large responsive flow views, shared late-window history and source loss offline. */
import { expect, test } from "@playwright/test";
import { messages } from "../src/messages/en";
import { fakeServer, node, openFlow, telegram } from "./telegram-flow-server";

test("details consume space only on selection; maximize restores and manual zoom overrides auto-fit", async ({ page }) => {
  await page.setViewportSize({ width: 1500, height: 1000 });
  const server = await fakeServer(page); server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  const graph = page.locator(".flow-svg");
  const initial = await graph.boundingBox(); expect(initial).not.toBeNull();
  await expect.poll(async () => { const a = await graph.boundingBox(); await page.waitForTimeout(300); const b = await graph.boundingBox(); return Math.abs(a!.height - b!.height); }).toBeLessThan(1);
  await expect(page.locator(".flow-inspector")).toHaveCount(0);
  await node(page, "Dimmer").click();
  const selected = await graph.boundingBox(); expect(selected!.width).toBeLessThan(initial!.width - 200);
  await page.getByRole("button", { name: "Close details", exact: true }).click();
  await expect(page.locator(".flow-inspector")).toHaveCount(0);
  await page.getByRole("button", { name: "Maximize view", exact: true }).click();
  await expect(page.locator(".telegram-flow")).toHaveClass(/flow-maximized/);
  const maximized = await graph.boundingBox(); expect(maximized!.height).toBeGreaterThan(initial!.height + 80);
  for (let i = 0; i < 15; i++) {
    await page.keyboard.press("Tab");
    expect(await page.evaluate(() => !!document.activeElement?.closest(".flow-maximized"))).toBe(true);
  }
  await page.getByRole("button", { name: "Zoom in", exact: true }).click();
  await expect(page.getByRole("button", { name: "Auto zoom", exact: true })).toHaveAttribute("aria-pressed", "false");
  const camera = await page.locator("g[data-zoom]").getAttribute("transform");
  server.generation = "2"; server.rows.push(telegram(2, "2", "Off"));
  await expect(node(page, "Blind")).toBeVisible();
  await expect(page.locator("g[data-zoom]")).toHaveAttribute("transform", camera!);
  await page.getByRole("button", { name: "Auto zoom", exact: true }).click();
  await expect(page.getByRole("button", { name: "Auto zoom", exact: true })).toHaveAttribute("aria-pressed", "true");
  await page.keyboard.press("Escape");
  await expect(page.locator(".telegram-flow")).not.toHaveClass(/flow-maximized/);
  expect(server.unexpected).toEqual([]);
});

test("a late Flow window adopts old graph history without reviving a value or polling the bus", async ({ page, context }) => {
  const server = await fakeServer(page); server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  await expect(node(page, "Dimmer").locator(".flow-badge")).toHaveCount(0, { timeout: 10000 });
  const popupPromise = page.waitForEvent("popup");
  await page.getByRole("button", { name: "Open in new window", exact: true }).click();
  const popup = await popupPromise;
  const requests: string[] = [];
  popup.on("request", r => { if (new URL(r.url()).pathname.startsWith("/api/")) requests.push(r.url()); });
  await expect(node(popup, "Dimmer")).toBeVisible();
  await expect(node(popup, "Dimmer").locator(".flow-badge")).toHaveCount(0);
  await popup.getByRole("button", { name: "Zoom in", exact: true }).click();
  const camera = await popup.locator("g[data-zoom]").getAttribute("transform");
  await page.getByRole("button", { name: "Open in new window", exact: true }).click();
  expect(context.pages()).toHaveLength(2);
  await expect(popup.locator("g[data-zoom]")).toHaveAttribute("transform", camera!);
  server.generation = "2"; server.rows.push(telegram(2, "2", "Off"));
  await expect(node(popup, "Blind").locator(".flow-badge")).toHaveText("◇ 1/0/1 Off");
  await expect(node(popup, "Dimmer")).toBeVisible();
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await expect(popup.getByText("The source monitor is paused. New telegrams are not being added.")).toBeVisible();
  expect(requests).toEqual([]);
  await popup.close();
  expect(server.unexpected).toEqual([]);
});

test("capture loss and stale project context remain visible in the dedicated and maximized view", async ({ page }) => {
  const server = await fakeServer(page); server.rows.push(telegram(1, "1", "On"));
  server.droppedBefore = 5; server.contextStatus = "stale";
  await openFlow(page);
  await page.getByRole("button", { name: "Maximize view", exact: true }).click();
  await expect(page.locator(".flow-source-diagnostics")).toContainText(messages["busMonitor.contextStale"]);
  await expect(page.locator(".flow-source-diagnostics")).toContainText("5");
  const pending = page.waitForEvent("popup");
  await page.getByRole("button", { name: "Open in new window", exact: true }).click();
  const popup = await pending;
  await expect(popup.locator(".flow-source-diagnostics")).toContainText(messages["busMonitor.contextStale"]);
  await expect(popup.locator(".flow-source-diagnostics")).toContainText("5");
  await popup.close();
});

test("source loss retains a visibly non-live map", async ({ page }) => {
  const server = await fakeServer(page); server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  const pending = page.waitForEvent("popup");
  await page.getByRole("button", { name: "Open in new window", exact: true }).click();
  const popup = await pending;
  await expect(node(popup, "Switch")).toBeVisible();
  await page.close();
  await expect(popup.getByText("Source window unavailable. This is the last received map; it is not live.")).toBeVisible({ timeout: 12000 });
  await expect(node(popup, "Switch")).toBeVisible();
  await node(popup, "Switch").click();
  await expect(popup.getByRole("button", { name: "Open device in project", exact: true })).toBeDisabled();
  await popup.close();
});

test("flow camera survives table switches and popup blocking is reported", async ({ page }) => {
  const server = await fakeServer(page); server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  await page.getByRole("button", { name: "Zoom in", exact: true }).click();
  const camera = await page.locator("g[data-zoom]").getAttribute("transform");
  await page.getByRole("tab", { name: "Telegrams", exact: true }).click();
  await page.getByRole("tab", { name: "Flow", exact: true }).click();
  await expect(page.locator("g[data-zoom]")).toHaveAttribute("transform", camera!);
  await page.evaluate(() => { window.open = () => null; });
  await page.getByRole("button", { name: "Open in new window", exact: true }).click();
  await expect(page.locator(".field-error")).toContainText(/popup|Pop-up/i);
});
