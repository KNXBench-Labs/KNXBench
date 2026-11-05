/** WP-26: the status-bar bus connection fits, selects without connecting and stops its session. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { dismissToasts as dismissReadyToasts, intercept, openProject } from "./communication-fixture";

const CONNECTED = "192.0.2.10:3671";
const HALL = "192.0.2.20:3671";
// Settings can change the initial English chrome after a late-night toast appears.
async function dismissToasts(page: Page, language: "en" | "de") {
  await expect(page.locator("html")).toHaveAttribute("lang", language);
  await dismissReadyToasts(page, language);
}
const fill = (template: string, params: Record<string, string | number>) =>
  template.replace(/\{(\w+)\}/g, (_, key: string) => String(params[key]));

type Phase = "idle" | "active" | "closed" | "system2" | "failing";

/**
 * The shared strict fixture plus the bus routes this bar touches, registered
 * last so they answer first. Every request is logged; anything else stays in
 * `state.unexpected` (404). Discovery is always intercepted: no multicast.
 */
async function serve(page: Page, language: "en" | "de", theme: string, uiScale?: number) {
  const state = await intercept(page, language, 8, theme);
  const bus = { phase: "idle" as Phase, starts: [] as string[], bound: [] as string[], legacyStops: 0, settingsPuts: 0 };
  await page.route("**/api/settings", (route) => {
    const method = route.request().method();
    if (method === "PUT") {
      bus.settingsPuts += 1;
      return route.fulfill({ contentType: "application/json", body: JSON.stringify({ schemaVersion: 1, status: "ok", settings: {} }) });
    }
    return route.fulfill({ contentType: "application/json", body: JSON.stringify({ schemaVersion: 1, status: "ok",
      settings: { uiLanguage: language, theme, preferredGateway: CONNECTED, ...(uiScale === undefined ? {} : { uiScale }) } }) });
  });
  // Settings reads this on open; not part of this package, answered as absent.
  await page.route("**/api/legacy-password", (route) => route.request().method() === "GET"
    ? route.fulfill({ status: 404, contentType: "application/json", body: "{\"error\":\"not configured\"}" })
    : route.fallback());
  await page.route("**/api/bus/discover", (route) => route.fulfill({ contentType: "application/json", body: JSON.stringify({
    interfaces: [
      { controlEndpoint: HALL, individualAddress: "1.1.0", friendlyName: "Hall interface with a deliberately long friendly name", supportsTunnelling: true },
      { controlEndpoint: "192.0.2.30:3671", individualAddress: "1.0.0", friendlyName: "Line router", supportsTunnelling: false },
    ] }) }));
  await page.route("**/api/bus/activity", (route) => {
    if (bus.phase === "failing") return route.fulfill({ status: 500, contentType: "application/json", body: "{\"error\":\"boom\"}" });
    const sessions = bus.phase === "active" || bus.phase === "closed"
      ? [{ kind: "busMonitor", id: 6, gateway: CONNECTED, state: bus.phase }] : [];
    return route.fulfill({ contentType: "application/json", body: JSON.stringify({
      serverIncarnation: "fixture", coverage: "partial", historyState: "disabled", sessions, oneShot: [], oneShotDropped: 0,
      busyLocks: bus.phase === "system2" ? ["system2Operation"] : [], untracked: [] }) });
  });
  await page.route("**/api/bus/monitor/start", (route) => {
    bus.starts.push(JSON.parse(route.request().postData() ?? "{}").gateway);
    bus.phase = "active";
    return route.fulfill({ contentType: "application/json", body: JSON.stringify({ sessionId: 6, serverIncarnation: "fixture", assignedAddress: "1.1.250" }) });
  });
  await page.route("**/api/bus/monitor/stop-bound**", (route) => {
    bus.bound.push(new URL(route.request().url()).search);
    bus.phase = "idle";
    return route.fulfill({ contentType: "application/json", body: JSON.stringify({ sessionId: 6, serverIncarnation: "fixture", telegramCount: 0, droppedCount: 0 }) });
  });
  await page.route("**/api/bus/monitor/stop", (route) => {
    bus.legacyStops += 1;
    return route.fulfill({ status: 500, contentType: "application/json", body: "{\"error\":\"legacy stop must not be used\"}" });
  });
  return { state, bus };
}

async function assertFits(page: Page, width: number) {
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow, "no horizontal page overflow").toBeLessThanOrEqual(0);
  for (const selector of [".bus-connection-state", ".bus-connection-trigger", ".bus-connection-action"]) {
    const box = (await page.locator(selector).boundingBox())!;
    expect(box.x, selector).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width, selector).toBeLessThanOrEqual(width + 0.5);
  }
}

async function assertPanelFits(page: Page, width: number, height: number) {
  const panel = page.locator(".bus-connection-panel");
  await expect(panel).toBeVisible();
  const box = (await panel.boundingBox())!;
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.y).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(width + 0.5);
  expect(box.y + box.height).toBeLessThanOrEqual(height + 0.5);
  // Not clipped: its own scroll box holds everything it renders.
  const clipped = await panel.evaluate((element) => {
    const workbench = element.closest(".workbench")!.getBoundingClientRect();
    const own = element.getBoundingClientRect();
    return own.top < workbench.top - 0.5 || own.bottom > workbench.bottom + 0.5;
  });
  expect(clipped, "the drop-up stays inside the workbench").toBe(false);
}

/** The control's own centre is hit, not a toast or anything else over it. */
async function assertUnobscured(page: Page, selectors: string[]) {
  // Narrow windows scroll the whole workbench; the bar is then reached at its end.
  await page.locator(".workbench-status").evaluate((bar) => bar.scrollIntoView({ block: "end" }));
  for (const selector of selectors) {
    const hit = await page.locator(selector).evaluate((element) => {
      const box = element.getBoundingClientRect();
      const top = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
      if (top !== null && (top === element || element.contains(top))) return "control";
      const where = `${Math.round(box.left)},${Math.round(box.top)} ${Math.round(box.width)}x${Math.round(box.height)}`;
      return `${top?.className || top?.tagName || "nothing"} at ${where} in ${innerWidth}x${innerHeight}`;
    });
    expect(hit, `${selector} is not covered`).toBe("control");
  }
  const stack = page.locator(".workbench > .toast-stack");
  if (await stack.count() > 0) {
    const stackBottom = await stack.evaluate((element) => element.getBoundingClientRect().bottom);
    const barTop = await page.locator(".workbench-status").evaluate((element) => element.getBoundingClientRect().top);
    expect(stackBottom, "toast stack ends above the status bar").toBeLessThanOrEqual(barTop + 0.5);
  }
}

async function capture(page: Page, name: string) {
  if (!process.env.KNX_BUS_FOOTER_CAPTURE_DIR) return;
  await page.screenshot({ path: join(process.env.KNX_BUS_FOOTER_CAPTURE_DIR, `${name}.png`) });
}

for (const language of ["en", "de"] as const) {
  for (const [width, theme] of [[1440, "porcelain"], [400, "graphite"]] as const) {
    test(`${language} ${width} ${theme}: select, connect once, disconnect bound, all inside the viewport`, async ({ page }) => {
      const m = language === "en" ? en : de;
      const height = 900;
      await page.setViewportSize({ width, height });
      await page.emulateMedia({ reducedMotion: "reduce" });
      const { state, bus } = await serve(page, language, theme);
      await page.goto("/");
      await dismissToasts(page, language);
      const status = page.locator(".bus-connection-state");
      await expect(status).toContainText(m["busConnection.state.disconnected"]);
      await expect(page.locator(".workbench-status > span:last-child")).toHaveText(/^v/);
      await assertFits(page, width);
      await capture(page, `${language}-${width}-disconnected`);

      // Keyboard: open, move in, Escape back to a visibly focused trigger.
      const trigger = page.locator(".bus-connection-trigger");
      await trigger.focus();
      await page.keyboard.press("Enter");
      await assertPanelFits(page, width, height);
      await expect(page.locator(".bus-connection-panel select").first().locator("option")).toHaveText(["KNXnet/IP", "USB/serial TP-UART"]);
      await expect(page.locator(".bus-connection-panel select").first()).toHaveValue("ip");
      const router = page.locator(".bus-connection-panel .bus-discovery-option", { hasText: "Line router" });
      await expect(router).toBeDisabled();
      await expect(router).toContainText(m["busConnection.notTunnelling"]);
      await capture(page, `${language}-${width}-open`);
      await page.keyboard.press("Tab");
      await page.keyboard.press("Tab");
      await page.keyboard.press("Escape");
      await expect(page.locator(".bus-connection-panel")).toHaveCount(0);
      await expect(trigger).toBeFocused();
      expect(await trigger.evaluate((element) => getComputedStyle(element).outlineStyle)).not.toBe("none");
      expect(await page.evaluate(() => document.getAnimations().length), "no running animation under reduced motion").toBe(0);

      // Selection saves the preference only.
      await trigger.click();
      await page.locator(".bus-connection-panel .bus-discovery-option", { hasText: "Hall interface" }).click();
      await expect(trigger).toContainText(HALL);
      await expect.poll(() => bus.settingsPuts).toBeGreaterThan(0);
      expect(bus.starts).toEqual([]);
      await page.mouse.click(width / 2, 20);
      await expect(page.locator(".bus-connection-panel")).toHaveCount(0);

      // One explicit Connect, with the selected endpoint.
      const action = page.locator(".bus-connection-action");
      await action.click();
      await expect(status).toContainText(fill(m["busConnection.state.connected"], { gateway: CONNECTED }));
      expect(bus.starts).toEqual([HALL]);
      await assertFits(page, width);
      await capture(page, `${language}-${width}-connected`);

      // The achievement toast stays: it sits above the bar and must not cover it.
      await assertUnobscured(page, [".bus-connection-trigger", ".bus-connection-action"]);
      // The fixture reports A while B is selected: both stay visible, nothing switches.
      await trigger.click();
      await expect(page.locator(".bus-connection-panel")).toContainText(
        fill(m["busConnection.selectionDiffers"], { connected: CONNECTED, selected: HALL }));
      await assertPanelFits(page, width, height);
      await page.keyboard.press("Escape");

      await action.click();
      await expect(status).toContainText(m["busConnection.state.disconnected"]);
      expect(bus.bound).toEqual(["?sessionId=6&serverIncarnation=fixture"]);
      expect(bus.legacyStops).toBe(0);
      expect(bus.starts).toEqual([HALL]);
      expect(state.unexpected).toEqual([]);
    });
  }

  test(`${language} 400: lost, occupied and unknown never offer Connect falsely`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width: 400, height: 900 });
    const { state, bus } = await serve(page, language, "porcelain");
    bus.phase = "closed";
    await page.goto("/");
    await dismissToasts(page, language);
    const status = page.locator(".bus-connection-state");
    const action = page.locator(".bus-connection-action");
    await expect(status).toContainText(fill(m["busConnection.state.lost"], { gateway: CONNECTED }));
    await expect(action).toHaveText(m["busConnection.disconnect"]);
    await expect(action).toBeEnabled();
    await capture(page, `${language}-400-lost`);
    bus.phase = "system2";
    await page.locator(".bus-connection-trigger").click();
    await expect(status).toContainText(m["busConnection.state.occupied"]);
    await expect(action).toBeDisabled();
    await expect(page.locator(".bus-connection-panel")).toContainText(m["gatewayRefusal.busy.connecting.system2Operation"]);
    await assertPanelFits(page, 400, 900);
    await capture(page, `${language}-400-occupied`);
    bus.phase = "failing";
    await page.locator(".bus-connection-tools button", { hasText: m["busConnection.checkStatus"] }).click();
    await expect(status).toContainText(m["busConnection.state.unknown"]);
    await expect(action).toBeDisabled();
    await assertFits(page, 400);
    await capture(page, `${language}-400-unknown`);
    expect(bus.starts).toEqual([]);
    expect(bus.bound).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });

  test(`${language} 400 zoomed: the drop-up fits at the largest UI scale with a project open, and opens bus settings`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width: 400, height: 760 });
    const { state, bus } = await serve(page, language, "porcelain", 1.5);
    await openProject(page, language);
    await dismissToasts(page, language);
    await expect(page.locator(".workbench-status-saved")).toBeVisible();
    await page.locator(".bus-connection-trigger").click();
    await assertPanelFits(page, 400, 760);
    await assertFits(page, 400);
    await capture(page, `${language}-400-zoom-open`);
    await page.locator(".bus-connection-tools button", { hasText: m["busConnection.settings"] }).click();
    await expect(page.locator("#settings-bus")).toBeFocused();
    expect(bus.starts).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });
}

// WP-26 review R5: the normal achievement toast after Connect is kept on screen,
// and the bar's trigger and Connect/Disconnect stay reachable at narrow widths,
// with and without a project, across UI scales, and after a resize.
for (const language of ["en", "de"] as const) {
  for (const scale of [0.8, 1, 1.5]) {
    for (const project of [false, true]) {
      test(`${language} 400 scale ${scale} ${project ? "project" : "welcome"}: a live toast never covers the bar`, async ({ page }) => {
        const m = language === "en" ? en : de;
        await page.setViewportSize({ width: 400, height: 800 });
        const { state, bus } = await serve(page, language, "porcelain", scale);
        if (project) await openProject(page, language);
        else await page.goto("/");
        const status = page.locator(".bus-connection-state");
        await expect(status).toContainText(m["busConnection.state.disconnected"]);
        await assertUnobscured(page, [".bus-connection-trigger", ".bus-connection-action"]);
        await page.locator(".bus-connection-action").click();
        await expect(status).toContainText(fill(m["busConnection.state.connected"], { gateway: CONNECTED }));
        await expect(page.locator(".workbench > .toast-stack .toast").first()).toBeVisible();
        await assertUnobscured(page, [".bus-connection-trigger", ".bus-connection-action"]);
        await capture(page, `r5-${language}-400-${scale}-${project ? "project" : "welcome"}-connected`);
        await page.locator(".bus-connection-trigger").click();
        await assertPanelFits(page, 400, 800);
        await page.keyboard.press("Escape");
        if (scale === 1) {
          await page.setViewportSize({ width: 340, height: 700 });
          await assertUnobscured(page, [".bus-connection-trigger", ".bus-connection-action"]);
          await page.setViewportSize({ width: 400, height: 800 });
        }
        await expect(page.locator(".workbench > .toast-stack .toast").first()).toBeVisible();
        await page.locator(".bus-connection-action").click();
        await expect(status).toContainText(m["busConnection.state.disconnected"]);
        expect(bus.bound).toEqual(["?sessionId=6&serverIncarnation=fixture"]);
        expect(bus.legacyStops).toBe(0);
        expect(state.unexpected).toEqual([]);
      });
    }
  }
}
