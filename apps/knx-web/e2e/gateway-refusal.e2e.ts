/** Issue #10: the gateway says who holds it and why it refuses, up front and on a refused start. */
import { expect, test } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { dismissToasts, intercept, openProject } from "./communication-fixture";

const GATEWAY = "192.0.2.10:3671";
type Catalogue = Record<keyof typeof en, string>;
const fill = (template: string, params: Record<string, string | number>) =>
  template.replace(/\{(\w+)\}/g, (_, key: string) => String(params[key]));

const refusals = [
  {
    name: "busy",
    status: 409,
    body: {
      error: "a line scan is running: the gateway serves one tunnel",
      kind: "gatewayBusy",
      holder: { kind: "lineScan", sessionId: 4, probed: 37, total: 120 },
    },
    expected: (m: Catalogue) => fill(m["gatewayRefusal.busy.lineScan"], { probed: 37, total: 120 }),
  },
  {
    name: "refused",
    status: 502,
    body: {
      error: "gateway refused the connection: E_NO_MORE_CONNECTIONS (status 0x24), …",
      kind: "gatewayRefused",
      gateway: GATEWAY,
      gatewayStatus: 0x24,
      gatewayError: "E_NO_MORE_CONNECTIONS",
    },
    expected: (m: Catalogue) => fill(m["gatewayRefusal.refused"], {
      gateway: GATEWAY,
      reason: m["gatewayRefusal.reason.noMoreConnections"],
      constant: "E_NO_MORE_CONNECTIONS",
      status: "0x24",
    }),
  },
] as const;

for (const language of ["en", "de"] as const) {
  for (const width of [1440, 400]) {
    for (const refusal of refusals) {
      test(`${language} ${width} ${refusal.name}: the monitor start names the reason`, async ({ page }) => {
        const messages = language === "en" ? en : de;
        await page.setViewportSize({ width, height: 900 });
        await intercept(page, language, 8, "porcelain");
        // Registered last, so these answer before the fixture's handler: no
        // monitor session to reattach to, and the start is refused.
        await page.route("**/api/bus/monitor/telegrams**", (route) =>
          route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "no session" }) }));
        await page.route("**/api/bus/monitor/start", (route) =>
          route.fulfill({ status: refusal.status, contentType: "application/json", body: JSON.stringify(refusal.body) }));
        await openProject(page, language);
        await dismissToasts(page, language);
        await page.keyboard.press("Control+Shift+P");
        await page.getByRole("combobox").fill(messages["toolbar.busMonitor"]);
        await page.getByRole("combobox").press("Enter");
        await page.getByLabel(messages["busMonitor.gatewayHost"]).fill("192.0.2.10");
        await page.getByLabel(messages["busMonitor.gatewayPort"]).fill("3671");
        await page.getByRole("button", { name: messages["busMonitor.connect"], exact: true }).click();

        const error = page.locator(".bus-monitor-panel .field-error").filter({ hasText: refusal.expected(messages) });
        await expect(error).toHaveText(refusal.expected(messages));
        // The server's English text is replaced, not appended.
        await expect(page.getByText(refusal.body.error)).toHaveCount(0);
        if (process.env.KNX_REFUSAL_CAPTURE_DIR) {
          await page.locator(".bus-monitor-panel").screenshot({
            path: join(process.env.KNX_REFUSAL_CAPTURE_DIR, `${language}-${width}-${refusal.name}.png`),
          });
        }
        const box = (await error.boundingBox())!;
        expect(box.x).toBeGreaterThanOrEqual(0);
        expect(box.x + box.width).toBeLessThanOrEqual(width);
        const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
        expect(overflow, "no horizontal page overflow").toBeLessThanOrEqual(0);
      });
    }
  }
}

for (const language of ["en", "de"] as const) {
  for (const width of [1440, 400]) {
    for (const theme of ["porcelain", "graphite"]) {
      test(`${language} ${width} ${theme}: the download page says up front what holds the gateway and what cannot be programmed`,
        async ({ page }) => {
          const messages = language === "en" ? en : de;
          await page.setViewportSize({ width, height: 900 });
          const state = await intercept(page, language, 8, theme);
          let monitor = true;
          let stops = 0;
          await page.route("**/api/bus/activity", (route) => route.fulfill({ contentType: "application/json", body: JSON.stringify({
            serverIncarnation: "fixture", coverage: "partial", historyState: "disabled", oneShot: [], oneShotDropped: 0, busyLocks: [],
            untracked: [], sessions: monitor ? [{ kind: "busMonitor", id: 6, gateway: GATEWAY, state: "active" }] : [] }) }));
          await page.route("**/api/bus/monitor/stop", (route) => {
            stops += 1; monitor = false;
            return route.fulfill({ contentType: "application/json", body: JSON.stringify({ sessionId: 6, telegramCount: 0 }) });
          });
          await page.route("**/api/device-readiness", (route) => route.fulfill({ contentType: "application/json", body: JSON.stringify({
            counts: {}, devices: [
              { address: "1.1.2", name: "Actuator 1", programRef: "p", readiness: "unsupported", category: "byte-order", detail: "x",
                steps: null, octets: null },
              { address: "1.1.10", name: "Actuator 2", programRef: "p", readiness: "untested", category: null, detail: null, steps: 1, octets: 1 },
            ] }) }));
          await openProject(page, language);
          await dismissToasts(page, language);
          await page.keyboard.press("Control+Shift+P");
          await page.getByRole("combobox").fill(messages["toolbar.busMonitor"]);
          await page.getByRole("combobox").press("Enter");
          await page.getByRole("button", { name: messages["deviceDownload.title"], exact: true }).click();
          const panel = page.locator(".device-download-panel");
          await expect(page.locator("html")).toHaveAttribute("data-theme", theme);

          const holder = panel.locator(".device-download-holder");
          await expect(holder.getByRole("status")).toHaveText(fill(messages["gatewayRefusal.busy.busMonitor"], { gateway: GATEWAY }));
          const mark = messages["deviceDownload.readiness.optionMark"];
          await expect(panel.locator("select[data-download-device] option")).toContainText([`1.1.2 — Actuator 1 (${mark})`]);
          await panel.locator("select[data-download-device]").selectOption({ label: `1.1.2 — Actuator 1 (${mark})` });
          const guidance = panel.locator("[data-support-guidance=unsupported]");
          await expect(guidance.getByRole("status")).toHaveText(messages["deviceDownload.readiness.reason.byteOrder"]);
          await expect(guidance.locator("[data-next-action=requestSupport]")).toHaveText(messages["deviceDownload.help.action.requestSupport"]);
          await expect(guidance.locator("a")).toHaveCount(3);
          await expect(panel.getByRole("button", { name: messages["deviceDownload.preparePlan"], exact: true })).toBeVisible();
          await expect(panel.locator(".device-download-actions-note")).toHaveText(messages["deviceDownload.preparePlanNote"]);
          if (process.env.KNX_REFUSAL_CAPTURE_DIR) {
            await panel.locator(".device-download-config").scrollIntoViewIfNeeded();
            await page.screenshot({ path: join(process.env.KNX_REFUSAL_CAPTURE_DIR, `download-${language}-${width}-${theme}.png`) });
          }
          // The closed select shows the whole label, mark included.
          const select = panel.locator("select[data-download-device]");
          expect(await select.evaluate((element) => element.scrollWidth <= element.clientWidth + 1)).toBe(true);
          for (const element of [holder, guidance]) {
            const box = (await element.boundingBox())!;
            expect(box.x).toBeGreaterThanOrEqual(0);
            expect(box.x + box.width).toBeLessThanOrEqual(width);
          }
          const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
          expect(overflow, "no horizontal page overflow").toBeLessThanOrEqual(0);

          await holder.getByRole("button", { name: messages["deviceDownload.holder.stopMonitor"] }).click();
          await expect(holder).toHaveCount(0);
          expect(stops).toBe(1);
          // The monitor's gateway is kept for the download.
          await expect(panel.locator("input[data-download-gateway]")).toHaveValue(GATEWAY);
          expect(state.requests.filter((request) => request.startsWith("POST /api/device-download/"))).toEqual([]);
          expect(state.unexpected).toEqual([]);
        });
    }
  }
}
