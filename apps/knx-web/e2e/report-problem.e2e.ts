/** Report problem wizard at the real shell with every API call intercepted (DIAG-03). */
import { expect, test, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { PROBLEM_TYPES } from "../src/diagnosticBundleShape";
import { defaultSelection } from "../src/reportProblem";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { openFileSubmenu } from "./file-menu";
import {
  answerBundleRoute, BUNDLE_FILE_NAME, BUNDLE_INACTIVE_MISSING, BUNDLE_MAIL_DRAFT, BUNDLE_OLDER,
  BUNDLE_RECORDING, BUNDLE_WITHHELD, bundleLog, bundlePreview, SECTION_TEXT_MARKER, ticked, type BundleLog,
} from "./report-problem-fixture";

type Language = "en" | "de";
type Messages = Record<keyof typeof en, string>;

async function intercept(page: Page, language: Language) {
  const unexpected: string[] = [];
  const log: BundleLog = bundleLog();
  const support: { path: string; body: Record<string, unknown> }[] = [];
  const supportToken = "b".repeat(64);
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    const json = (body: unknown, status = 200) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
    if (await answerBundleRoute(route, log)) return;
    if (["/api/support/preview", "/api/support/diagnose"].includes(path) && method === "POST") {
      const body = request.postDataJSON() as Record<string, unknown>;
      support.push({ path, body });
      if (path.endsWith("diagnose") && (body.consent !== true || body.token !== supportToken)) return json({ error: "read refused" }, 400);
      return json({ token: supportToken, digest: "c".repeat(64), report: { support: "unknown" }, summary: "Retained identity result",
        localAddress: "1.1.67", mask: null, profile: "descriptor-only-v1", reads: ["DeviceDescriptorRead type 0"], deadlineSeconds: 30 });
    }
    if (path === "/api/auth/status") return json({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return json({ schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme: "porcelain", releaseChecksEnabled: false, motionLevel: "off", onboardingGuide: { seenStage: "alpha" } } });
    if (path === "/api/project") return json({ error: "no project open" }, 400);
    if (path === "/api/project/load-progress") return json(null);
    if (path === "/api/log") return json([]);
    if (path === "/api/legacy-password") return json({ available: true, remembered: false, problem: null });
    if (path === "/api/product-languages") return json([]);
    if (path === "/api/bus/discover") return json({ interfaces: [] });
    if (path === "/api/version") return json({ version: "0.1.0-alpha.18" });
    if (["/api/device-download/status", "/api/device-address/status", "/api/device-system2/status"].includes(path)) return json({ error: "no operation" }, 404);
    const achievement = achievementsFixtureAnswer(method, path);
    if (achievement !== undefined) return json(achievement);
    unexpected.push(`${method} ${path}`);
    return json({ error: "strict report-problem fixture" }, 404);
  });
  return { unexpected, log, support };
}

async function openFromMenu(page: Page, m: Messages) {
  await page.locator(".file-menu > summary").click();
  await openFileSubmenu(page, "help");
  const entry = page.locator(".file-menu [data-report-problem-open=menu]");
  await expect(entry).toHaveText(m["reportProblem.button"]);
  await entry.click();
}

async function openFromPalette(page: Page, m: Messages) {
  await expect(page.locator(".file-menu > summary")).toBeVisible();
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["reportProblem.button"].replace("…", ""));
  await page.getByRole("combobox").press("Enter");
}

/** Presses Tab until `selector` holds focus; fails instead of looping forever. */
async function tabTo(page: Page, selector: string) {
  for (let i = 0; i < 60; i += 1) {
    if (await page.evaluate((s) => document.activeElement?.matches(s) ?? false, selector)) return;
    await page.keyboard.press("Tab");
  }
  throw new Error(`focus never reached ${selector}`);
}

async function expectFits(page: Page, width: number) {
  const dialog = page.locator(".report-problem");
  const box = await dialog.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  expect(await dialog.evaluate((el) => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1);
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: every problem type preselects O1, preview, consent, ZIP and mail draft`, async ({ page }) => {
    const m: Messages = language === "de" ? de : en;
    const { unexpected, log } = await intercept(page, language);
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    if (width === 1440) await openFromMenu(page, m); else await openFromPalette(page, m);
    const dialog = page.getByRole("dialog", { name: m["reportProblem.title"], exact: true });
    await expect(dialog).toBeVisible();
    await expect(dialog.locator("[data-report-type]")).toHaveCount(5);
    expect(log.previews).toEqual([]);
    await expectFits(page, width);

    const next = dialog.locator("[data-report-next]");
    const back = dialog.getByRole("button", { name: m["projectWizard.back"], exact: true });
    for (const type of PROBLEM_TYPES) {
      await dialog.locator(`[data-report-type=${type}]`).check();
      await next.click();
      await expect(dialog.locator("[data-report-total]")).toBeVisible();
      await expect.poll(() => log.previews.at(-1)?.problemType).toBe(type);
      expect(log.previews.at(-1)!.sections).toEqual(defaultSelection(type));
      // The rows follow the server's answer, which arrives after the request: read them until it shows.
      await expect(dialog.locator(".report-problem-loading")).toHaveCount(0);
      const checked = () => dialog.locator("[data-report-section]").evaluateAll((rows) => rows
        .filter((row) => (row.querySelector("input[type=checkbox]:not([data-report-earlier])") as HTMLInputElement | null)?.checked)
        .map((row) => (row as HTMLElement).dataset.reportSection));
      // Ticked = O1 defaults the server could build; this fixture names no device.
      const available = bundlePreview(log.previews.at(-1)!).offers.filter((o) => o.unavailable === null).map((o) => o.kind);
      await expect.poll(checked, { message: type }).toEqual(defaultSelection(type).filter((kind) => available.includes(kind)));
      if (type === "deviceProgramming" || type === "deviceReading") {
        await expect(dialog.locator("[data-report-section=device] [data-report-unavailable]"))
          .toHaveText(m["reportProblem.unavailable"].replace("{reason}", m["reportProblem.reason.noDevice"]));
      }
      await expect(dialog.locator("[data-report-section=environment] input")).toBeDisabled();
      if (type !== "projectImport" && type !== "catalogue") {
        const reason = dialog.locator("[data-report-section=busMonitor] [data-report-unavailable]");
        await expect(reason).toHaveText(m["reportProblem.unavailable"].replace("{reason}", m["reportProblem.reason.busMonitorOff"]));
        await expect(dialog.locator("[data-report-section=busMonitor] input")).toBeDisabled();
      }
      await expectFits(page, width);
      await back.click();
    }

    await dialog.locator("[data-report-type=deviceProgramming]").check();
    await next.click();
    const operationSelect = dialog.locator("[data-report-operation]");
    await expect(operationSelect).toBeVisible();
    await expect(operationSelect.locator("option")).toHaveCount(5);
    const lifecycle = language === "de"
      ? [
        [BUNDLE_RECORDING.id, "fehlgeschlagen", "4 KiB"],
        [BUNDLE_OLDER.id, "nicht abgeschlossen", "2 KiB"],
        [BUNDLE_INACTIVE_MISSING.id, "aufgezeichnetes Ende nicht verfügbar", "3 KiB"],
        [BUNDLE_WITHHELD.id, "Ergebnis zurückgehalten", "1 KiB"],
      ] as const
      : [
        [BUNDLE_RECORDING.id, "failed", "4 KiB"],
        [BUNDLE_OLDER.id, "unfinished", "2 KiB"],
        [BUNDLE_INACTIVE_MISSING.id, "recorded end unavailable", "3 KiB"],
        [BUNDLE_WITHHELD.id, "outcome withheld", "1 KiB"],
      ] as const;
    for (const [id, outcome, size] of lifecycle) {
      const option = operationSelect.locator(`option[value="${id}"]`);
      await expect(option, `${id} must remain reachable`).toHaveCount(1);
      await expect(option).toContainText(id);
      await expect(option).toContainText(m["reportProblem.operation.recorded"]);
      await expect(option).toContainText(outcome);
      await expect(option).toContainText(size);
    }
    await expect(operationSelect.locator(`option[value="${BUNDLE_WITHHELD.id}"]`))
      .not.toContainText(language === "de" ? "nicht abgeschlossen" : "unfinished");
    await dialog.locator(`[data-report-earlier="${BUNDLE_OLDER.id}"]`).check();
    await expect.poll(() => log.previews.at(-1)?.earlierRecordingIds).toEqual([BUNDLE_OLDER.id]);
    await next.click();
    const operation = dialog.locator("[data-report-preview^='operation:']");
    await expect(operation.locator("[data-report-markers] li")).toHaveText([
      m["reportProblem.marker.addresses"], m["reportProblem.marker.parameterValues"], m["reportProblem.marker.networkAddresses"]]);
    await expect(dialog.locator("[data-report-removed]")).toHaveText(m["reportProblem.removed"]);
    await expect(dialog.locator("[data-report-not-removed]")).toHaveText(m["reportProblem.notRemoved"]);
    await dialog.locator("[data-report-preview=sessionLog] [data-report-show]").click();
    await expect(dialog.locator("[data-report-preview=sessionLog] pre")).toContainText(SECTION_TEXT_MARKER);
    expect(log.sections.map((s) => s.key)).toEqual(["sessionLog"]);
    await expectFits(page, width);

    await next.click();
    const save = dialog.locator("[data-report-save]");
    await expect(save).toBeDisabled();
    await expect(dialog.locator("button[data-report-mail]")).toBeDisabled();
    await expect(dialog.locator("a[href^='mailto:']")).toHaveCount(0);
    await expect(dialog.locator("[data-report-mail-note]")).toHaveText(m["reportProblem.mailNote"]);
    expect(log.exports).toEqual([]);
    await dialog.locator("[data-report-consent]").check();
    const download = page.waitForEvent("download");
    await save.click();
    const file = await download;
    expect(file.suggestedFilename()).toBe(BUNDLE_FILE_NAME);
    expect((await readFile((await file.path())!)).subarray(0, 2).toString("latin1")).toBe("PK");
    expect(log.exports).toHaveLength(1);
    expect(log.exports[0]).toMatchObject({ problemType: "deviceProgramming", consent: true, earlierRecordingIds: [BUNDLE_OLDER.id] });
    await expect(dialog.locator("button[data-report-mail]")).toBeDisabled();
    await expect(dialog.locator("[data-report-file-confirm]")).not.toBeChecked();
    await expect(dialog.locator("[data-report-requested]")).toContainText(BUNDLE_FILE_NAME);
    await dialog.locator("[data-report-file-confirm]").check();
    const mail = dialog.locator("a[data-report-mail]");
    await expect(mail).toBeFocused();
    const href = (await mail.getAttribute("href"))!;
    expect(href).toBe(BUNDLE_MAIL_DRAFT);
    expect(href).not.toContain(SECTION_TEXT_MARKER);
    expect(href).not.toMatch(/attach(ment)?=/i);
    await expect(dialog.locator("[data-report-requested]")).toContainText(BUNDLE_FILE_NAME);
    await expect(dialog.getByRole("alert")).toHaveCount(0);
    await expectFits(page, width);
    await dialog.getByRole("button", { name: m["reportProblem.close"], exact: true }).click();
    await expect(dialog).toHaveCount(0);
    expect(unexpected).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) {
  test(`${language} keyboard only: palette, all four steps, consent, save and Escape`, async ({ page }) => {
    const m: Messages = language === "de" ? de : en;
    const { unexpected, log } = await intercept(page, language);
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await expect(page.locator(".file-menu > summary")).toBeVisible();
    await page.keyboard.press("Control+Shift+P");
    await page.keyboard.type(m["reportProblem.button"].replace("…", ""));
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog", { name: m["reportProblem.title"], exact: true });
    await expect(dialog).toBeVisible();
    await expect(dialog.locator(".project-wizard-step-title")).toBeFocused();
    // Step 1: the radio group takes arrow keys; user interface is the default, catalogue sits above it.
    await tabTo(page, "[data-report-type]:checked");
    await page.keyboard.press("ArrowUp");
    await expect(dialog.locator("[data-report-type=catalogue]")).toBeChecked();
    await tabTo(page, "[data-report-next]");
    await page.keyboard.press("Enter");
    await expect.poll(() => log.previews.at(-1)?.problemType).toBe("catalogue");
    await expect(dialog.locator(".project-wizard-step-title")).toBeFocused();
    // Step 2: untick the session log by keyboard; the server is asked again.
    await tabTo(page, "[data-report-section=sessionLog] input[type=checkbox]");
    await page.keyboard.press("Space");
    await expect.poll(() => log.previews.at(-1) && ticked(log.previews.at(-1)!)).toEqual(["catalogue"]);
    await tabTo(page, "[data-report-next]:not([disabled])");
    await page.keyboard.press("Enter");
    // Step 3: open one section's text with the keyboard.
    await tabTo(page, "[data-report-preview=catalogue] [data-report-show]");
    await page.keyboard.press("Enter");
    await expect(dialog.locator("[data-report-preview=catalogue] pre")).toContainText(SECTION_TEXT_MARKER);
    await tabTo(page, "[data-report-next]");
    await page.keyboard.press("Enter");
    // Step 4: consent, then save.
    await tabTo(page, "[data-report-consent]");
    await page.keyboard.press("Space");
    await tabTo(page, "[data-report-save]");
    const download = page.waitForEvent("download");
    await page.keyboard.press("Enter");
    expect((await download).suggestedFilename()).toBe(BUNDLE_FILE_NAME);
    await expect(dialog.locator("[data-report-file-confirm]")).toBeFocused();
    await expect(dialog.locator("button[data-report-mail]")).toBeDisabled();
    await page.keyboard.press("Space");
    await expect(dialog.locator("a[data-report-mail]")).toBeFocused();
    expect(log.exports.map((e) => [e.problemType, e.consent])).toEqual([["catalogue", true]]);
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    expect(unexpected).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: no-project manual acquisition has separate read and share consent`, async ({ page }) => {
    const m: Messages = language === "de" ? de : en;
    const { unexpected, log, support } = await intercept(page, language);
    await page.setViewportSize({ width, height: 900 }); await page.goto("/");
    await page.locator(".file-menu > summary").click(); await openFileSubmenu(page, "help");
    await page.locator("[data-identity-read-open]").click();
    const acquire = page.getByRole("dialog", { name: m["identityRead.title"], exact: true });
    await expect(acquire).toBeVisible(); expect(support).toEqual([]); expect(log.previews).toEqual([]);
    await acquire.locator("[data-support-local-address]").fill("16.1.67");
    await expect(acquire.locator("[data-support-preview]")).toBeDisabled();
    await acquire.locator("[data-support-local-address]").fill("1.1.67");
    await acquire.locator("[data-support-preview]").click();
    await expect(acquire.locator("[data-support-preview-content]")).toContainText("descriptor-only-v1");
    expect(support).toEqual([{ path: "/api/support/preview", body: { input: { includeContext: false, contextRights: false, functionalCheck: "not-checked" }, deviceId: null, address: "1.1.67" } }]);
    await expect(acquire.locator("[data-report-save], a[href^='mailto:']")).toHaveCount(0);
    await acquire.locator("[data-support-gateway]").fill("192.0.2.10:3671");
    await expect(acquire.locator("[data-support-read]")).toBeDisabled();
    await acquire.locator("[data-support-live-consent]").check(); await acquire.locator("[data-support-read]").click();
    const handoff = acquire.locator("[data-report-problem-open=support-read]");
    await expect(handoff).toBeVisible();
    expect(support[1]).toEqual({ path: "/api/support/diagnose", body: { token: "b".repeat(64), gateway: "192.0.2.10:3671", consent: true } });
    await handoff.click(); await expect(acquire).toHaveCount(0);
    const report = page.getByRole("dialog", { name: m["reportProblem.title"], exact: true });
    await expect(report).toBeVisible();
    await expect.poll(() => log.previews.at(-1)?.operation).toEqual({ relatedKey: "supportToken", relatedValue: "b".repeat(64) });
    expect(log.previews.at(-1)!).not.toHaveProperty("deviceId");
    await report.locator("[data-report-next]").click(); await report.locator("[data-report-next]").click();
    await expect(report.locator("[data-report-consent]")).not.toBeChecked();
    await expect(report.locator("[data-report-save]")).toBeDisabled();
    await expectFits(page, width); expect(log.exports).toEqual([]); expect(support).toHaveLength(2);
    expect(unexpected).toEqual([]);
  });
}

test("Delete recordings asks first and runs once", async ({ page }) => {
  const { unexpected, log } = await intercept(page, "en");
  await page.goto("/");
  await openFromMenu(page, en);
  const dialog = page.getByRole("dialog", { name: en["reportProblem.title"], exact: true });
  await dialog.locator("[data-report-type=deviceReading]").check();
  await dialog.locator("[data-report-next]").click();
  await dialog.locator("[data-report-delete]").click();
  const confirm = dialog.locator("[data-report-delete-confirm]");
  await expect(confirm).toHaveText(new RegExp(en["reportProblem.delete.question"].slice(0, 30)));
  expect(log.deletes).toBe(0);
  await confirm.getByRole("button", { name: en["reportProblem.delete.confirm"], exact: true }).click();
  await expect(dialog.locator("[data-report-deleted]")).toHaveText("Deleted 2 recordings.");
  expect(log.deletes).toBe(1);
  expect(unexpected).toEqual([]);
});
