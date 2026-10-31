/** Exercises pre-plan refusal help offline: instructions link and report shortcut, no upload. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { intercept, openProject, dismissToasts, openList } from "./communication-fixture";
import { answerBundleRoute, bundleLog, initialPreviewCount, BUNDLE_RECORDING, bundlePreview, type BundleLog } from "./report-problem-fixture";

import type { BundlePreview, BundleQuery, BundleRecordingSummary } from "../src/diagnosticBundleShape";
import { openFileSubmenu } from "./file-menu";

for (const language of ["en", "de"] as const) {
  for (const width of [1440, 400]) {
    test(`${language} ${width}: refusal help opens instructions and the report wizard before planning`, async ({ page }) => {
      const m = language === "en" ? en : de;
      const theme = width === 400 ? "graphite" : "porcelain";
      await page.setViewportSize({ width, height: 1000 });
      const state = await intercept(page, language, 8, theme);
      const calls: string[] = [];
      const bundle = bundleLog();
      await page.route("**/api/diagnostics/**", async (route) => { if (!(await answerBundleRoute(route, bundle))) await route.fallback(); });
      await page.route("**/api/device-readiness", (route) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({
        devices: [{ address: "1.1.2", name: "PRIVATE customer", programRef: "M-0001_A-0001-01-0000", readiness: "unsupported", category: "byte-order", detail: "CUSTOMER_SECRET 1.1.2 /home/private token=secret", steps: null, octets: null }], counts: { unsupported: 1 },
      }) }));
      await page.route("**/api/device-download/plan", (route) => {
        calls.push("plan"); return route.fulfill({ status: 422, contentType: "application/json", body: JSON.stringify({ error: "PRIVATE configuration detail", kind: "downloadPreparationRefused", category: "modules" }) });
      });
      await page.context().route("https://github.com/**", (route) => route.fulfill({ contentType: "text/html", body: "<title>Offline link target</title><p>No real submission.</p>" }));
      await openProject(page, language); await dismissToasts(page, language);
      await page.keyboard.press("Control+Shift+P");
      await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
      await page.getByRole("combobox").press("Enter");
      await page.getByRole("button", { name: m["deviceDownload.title"], exact: true }).click();
      const panel = page.locator(".device-download-panel");
      await panel.locator("[data-download-device]").selectOption("1");
      const help = panel.locator("[data-support-guidance]");
      await expect(help).toContainText(m["deviceDownload.readiness.reason.byteOrder"]);
      await expect(help.locator("[data-next-action=requestSupport]")).toHaveText(m["deviceDownload.help.action.requestSupport"]);
      await expect(panel.locator(".device-download-plan,.device-download-verify")).toHaveCount(0);
      expect(calls).toEqual([]);
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      for (const selector of ["[data-support-instructions]"]) {
        const link = help.locator(selector);
        const href = await link.getAttribute("href");
        expect(href).not.toContain("CUSTOMER_SECRET"); expect(href).not.toContain("1.1.2"); expect(href).not.toContain("private");
        const popupPromise = page.waitForEvent("popup"); await link.click();
        const popup = await popupPromise; await popup.waitForLoadState();
        expect(popup.url()).toBe(href);
        expect(popup.url()).toContain(language === "de" ? "DEUTSCH.md" : "README.md");
        await popup.close();
      }
      // DIAG-03: the issue and mail links became one shortcut into the report wizard, this device preselected.
      await expect(help.locator("a")).toHaveCount(1);
      await expect(panel.locator("[data-report-problem-open]")).toHaveCount(1);
      await help.locator("[data-report-problem-open=support-notice]").click();
      const wizard = page.getByRole("dialog", { name: m["reportProblem.title"], exact: true });
      await expect(wizard.locator("[aria-current=step]")).toContainText(m["reportProblem.step.contents"]);
      await expect.poll(() => bundle.previews.length).toBe(await initialPreviewCount(page));
      expect(bundle.previews[0]).toMatchObject({ problemType: "deviceProgramming", deviceId: 1 });
      for (const query of bundle.previews) {
        expect(query).toMatchObject({ problemType: "deviceProgramming", deviceId: 1, operation: { selection: "none" } });
      }
      expect(JSON.stringify(bundle.previews[0])).not.toContain("CUSTOMER_SECRET");
      expect(bundle.exports).toEqual([]);
      await page.keyboard.press("Escape");
      await expect(wizard).toHaveCount(0);
      await help.locator("summary").click();
      await expect(help.locator("textarea")).toHaveValue("byte-order\nCUSTOMER_SECRET 1.1.2 /home/private token=secret");
      await help.scrollIntoViewIfNeeded();
      const geometry = await help.evaluate((element) => {
        const box = element.getBoundingClientRect();
        const form = element.closest("form")!;
        const style = getComputedStyle(form);
        return { left: box.left, right: box.right, width: box.width,
          available: form.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight),
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth };
      });
      expect(geometry.left).toBeGreaterThanOrEqual(0); expect(geometry.right).toBeLessThanOrEqual(width); expect(geometry.overflow).toBeLessThanOrEqual(1);
      expect(geometry.width, "help spans the configuration instead of a narrow grid cell").toBeGreaterThanOrEqual(geometry.available - 2);
      await expect(help.locator("[data-copy-technical]")).toBeVisible();
      if (process.env.KNX_SUPPORT_CAPTURE_DIR) await help.screenshot({ path: join(process.env.KNX_SUPPORT_CAPTURE_DIR, `${language}-${width}-${theme}.png`) });
      await panel.getByRole("button", { name: m["deviceDownload.preparePlan"], exact: true }).click();
      await expect(help).toContainText(m["deviceDownload.readiness.reason.modules"]);
      await expect(help.locator("textarea")).toHaveValue("downloadPreparationRefused\nmodules\nPRIVATE configuration detail");
      await expect(panel.locator(".device-download-plan,.device-download-verify")).toHaveCount(0);
      expect(calls).toEqual(["plan"]);
      expect(state.requests.filter((r) => r.includes("/device-download/start") || r.includes("/device-download/report"))).toEqual([]);
      expect(state.unexpected).toEqual([]);
    });
  }
}


type Language = "en" | "de";
const DEVICE_A = 1;
const DEVICE_B = 2;
const CONTEXT = { serverIncarnation: "devices-browser", projectIncarnation: 1, revision: 1 };
/** The newest finished run; the guard below proves it targets A, the closed preview view carries no target. */
const RUN_A: BundleRecordingSummary = { ...BUNDLE_RECORDING, end: { state: "known", outcome: "succeeded" } };

/** The server's answer for `query`: the newest run is adopted unless none is named, and a foreign device is refused. */
function guardedPreview(query: BundleQuery): BundlePreview | "bundleTargetMismatch" {
  const none = query.operation !== undefined && "selection" in query.operation;
  if (query.operation === undefined && query.deviceId !== undefined && query.deviceId !== DEVICE_A) return "bundleTargetMismatch";
  const base = bundlePreview(query);
  const sections = none ? base.sections.filter((section) => section.kind !== "operation") : base.sections;
  return { ...base, operationRecordingId: none ? null : RUN_A.id, recordings: [RUN_A], sections,
    offers: base.offers.map((offer) => none && offer.kind === "operation" ? { ...offer, selected: false } : offer),
    totalBytes: sections.reduce((sum, section) => sum + section.bytes, 0) };
}

/** Strict project routes plus the bundle routes; the preview route enforces the guard like the server. */
async function routes(page: Page, language: Language) {
  const state = await intercept(page, language);
  state.tree = { ...state.tree, server_incarnation: CONTEXT.serverIncarnation,
    project_incarnation: CONTEXT.projectIncarnation, snapshot_revision: CONTEXT.revision };
  const log: BundleLog = bundleLog();
  const external: string[] = [];
  page.on("request", (request) => {
    const { protocol, hostname } = new URL(request.url());
    if ((protocol === "http:" || protocol === "https:") && hostname !== "127.0.0.1") external.push(request.url());
  });
  await page.route("**/api/diagnostics/**", async (route) => {
    const request = route.request();
    if (new URL(request.url()).pathname === "/api/diagnostics/bundle/preview" && request.method() === "POST") {
      const query = request.postDataJSON() as BundleQuery;
      expect(query.context).toEqual(CONTEXT);
      log.previews.push(query);
      const answer = guardedPreview(query);
      if (answer === "bundleTargetMismatch") return route.fulfill({ status: 409, contentType: "application/json", body: JSON.stringify({ error: answer }) });
      return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(answer) });
    }
    if (!(await answerBundleRoute(route, log))) {
      state.unexpected.push(`${request.method()} ${new URL(request.url()).pathname}`);
      await route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "strict recovery fixture" }) });
    }
  });
  return { state, log, external };
}

async function openWizard(page: Page, m: typeof en | typeof de, width: number) {
  if (width === 1440) {
    await page.locator(".file-menu > summary").click();
    await openFileSubmenu(page, "help");
    await page.locator(".file-menu [data-report-problem-open=menu]").click();
  } else {
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["reportProblem.button"].replace("…", ""));
    await page.getByRole("combobox").press("Enter");
  }
}

async function expectFits(page: Page, width: number) {
  const box = await page.locator(".report-problem").boundingBox();
  expect(box).not.toBeNull();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
}

/** Optional evidence images, bounded to the failure and recovery states and the footer; synthetic data only. */
async function shot(page: Page, name: string, footer = false) {
  const directory = process.env.DIAG03_RECOVERY_SHOTS;
  if (!directory) return;
  const dialog = page.locator(".report-problem");
  await dialog.screenshot({ path: join(directory, `${name}.png`) });
  if (!footer) return;
  const actions = dialog.locator(".report-problem-actions");
  await actions.scrollIntoViewIfNeeded();
  await actions.screenshot({ path: join(directory, `${name}-footer.png`) });
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: choosing another device after the newest run is refused, then continuing without an operation`, async ({ page }) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 900 });
    const { state, log, external } = await routes(page, language);
    await openProject(page, language); await dismissToasts(page, language);
    await openWizard(page, m, width);
    const dialog = page.getByRole("dialog", { name: m["reportProblem.title"], exact: true });
    await dialog.locator("[data-report-type=deviceReading]").check();
    await dialog.locator("[data-report-next]").click();

    // The first preview names no device, so the server adopts run A; the picker offers device B.
    await expect(dialog.locator("[data-report-total]")).toBeVisible();
    await expect(dialog.locator("[data-report-operation]")).toHaveValue(RUN_A.id);
    expect(log.previews.at(-1)).not.toHaveProperty("deviceId");
    expect(log.previews.at(-1)).not.toHaveProperty("operation");
    const asked = log.previews.length;
    await dialog.locator("[data-report-device]").selectOption(String(DEVICE_B));

    // Same context, refused device: no reviewed content, no way forward, but the correction stays in reach.
    const notice = dialog.locator("[data-report-target-mismatch]");
    await expect(notice).toHaveText(m["reportProblem.targetMismatch"]);
    await expect(dialog.locator("[data-report-without-operation]")).toHaveText(m["reportProblem.withoutOperation"]);
    await expect(dialog.locator("[data-report-device]")).toHaveValue(String(DEVICE_B));
    await expect(dialog.locator("[data-report-refresh], [data-report-total], [data-report-operation]")).toHaveCount(0);
    await expect(dialog.locator(".project-wizard-step-link").nth(2)).toBeDisabled();
    await expect(dialog.locator(".project-wizard-step-link").nth(3)).toBeDisabled();
    await expect(dialog.locator("[data-report-next]")).toBeDisabled();
    expect(log.previews).toHaveLength(asked + 1);
    expect(log.previews.at(-1)).toMatchObject({ problemType: "deviceReading", deviceId: DEVICE_B });
    expect(log.previews.at(-1)).not.toHaveProperty("operation");
    await expectFits(page, width);
    await shot(page, `${language}-${width}-refused`, true);

    // Explicit keyboard reset: tab from the device choice to the recovery action, keeping the same device.
    const reset = dialog.locator("[data-report-without-operation]");
    await dialog.locator("[data-report-device]").focus();
    await page.keyboard.press("Tab");
    await expect(reset).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(dialog.locator("[data-report-total]")).toBeVisible();
    await expect(notice).toHaveCount(0);
    expect(log.previews).toHaveLength(asked + 2);
    expect(log.previews.at(-1)).toMatchObject({ problemType: "deviceReading", deviceId: DEVICE_B, operation: { selection: "none" } });
    await expect(dialog.locator("[data-report-device]")).toHaveValue(String(DEVICE_B));
    await expect(dialog.locator("[data-report-operation]")).toHaveValue("");
    await expect(dialog.locator("[aria-current=step]")).toContainText(m["reportProblem.step.contents"]);
    expect(log.exports).toEqual([]);
    await expectFits(page, width);
    await shot(page, `${language}-${width}-recovered`);

    // The recovered review needs its own consent before anything is built.
    await dialog.locator("[data-report-next]").click(); await dialog.locator("[data-report-next]").click();
    await expect(dialog.locator("[data-report-consent]")).not.toBeChecked();
    await expect(dialog.locator("[data-report-save]")).toBeDisabled();
    await expect(dialog.locator("button[data-report-mail]")).toBeDisabled();
    await expect(dialog.locator("a[data-report-mail]")).toHaveCount(0);
    expect(log.exports).toEqual([]);
    await dialog.locator("[data-report-consent]").check();
    const download = page.waitForEvent("download");
    await dialog.locator("[data-report-save]").click();
    await download;
    expect(log.exports).toHaveLength(1);
    expect(log.exports[0]).toMatchObject({ deviceId: DEVICE_B, operation: { selection: "none" }, consent: true });
    expect(state.unexpected).toEqual([]);
    expect(external).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) {
  test(`${language}: the device Diagnostics report button names no operation and adopts no run`, async ({ page }) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width: 1440, height: 900 });
    const { state, log, external } = await routes(page, language);
    await openProject(page, language); await dismissToasts(page, language); await openList(page, language);
    await page.locator(".devices-table tbody tr", { hasText: "Actuator 2" }).locator("td:nth-child(3) button").click();
    await page.getByRole("tab", { name: m["parameters.diagnosticsTab"], exact: true }).click();
    await page.locator(`#device-panel-${DEVICE_B}-3 [data-device-report]`).click();

    const dialog = page.getByRole("dialog", { name: m["reportProblem.title"], exact: true });
    await expect(dialog.locator("[data-report-total]")).toBeVisible();
    expect(log.previews.length).toBeGreaterThan(0);
    for (const query of log.previews) {
      expect(query).toMatchObject({ problemType: "deviceReading", deviceId: DEVICE_B, operation: { selection: "none" } });
    }
    await expect(dialog.locator("[data-report-target-mismatch]")).toHaveCount(0);
    await expect(dialog.locator("[data-report-device]")).toHaveValue(String(DEVICE_B));
    await expect(dialog.locator("[data-report-operation]")).toHaveValue("");
    expect(log.exports).toEqual([]);
    expect(state.unexpected).toEqual([]);
    expect(external).toEqual([]);
  });
}
