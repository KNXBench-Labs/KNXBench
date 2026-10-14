/** Production workbench integration; every API request is intercepted, no bus traffic. */
import { expect, test, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import manifest from "../package.json" with { type: "json" };
const { version } = manifest;
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";

test.use({ timezoneId: "UTC" });

async function fixture(page: Page, language: "en" | "de", theme: "graphite" | "porcelain", locked: boolean) {
  // Keep late-night/holiday startup notices independent of the runner clock.
  await page.clock.setFixedTime(new Date("2026-10-11T12:00:00.000Z"));
  const unexpected: string[] = [];
  const settings = { uiLanguage: language, theme, motionLevel: "off", achievementsEnabled: false,
    onboardingGuide: { seenStage: "alpha", version } };
  await page.addInitScript(settings => localStorage.setItem("knx-desktop:settings-cache", JSON.stringify({ schemaVersion: 1, settings })), settings);
  const raw = "Synthetic refusal\nExact diagnostic: <address> ü";
  const backend = { timestamp: "2026-10-11T10:00:00.000Z", severity: "error" as const, source: "import", message: raw, location: "P-SYNTH/0.xml", detail: "Backend detail\nDo not flatten this." };
  const requests: string[] = [];
  await page.route("**/api/**", route => {
    const req = route.request(), path = new URL(req.url()).pathname, method = req.method();
    requests.push(`${method} ${path}`);
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status") return ok({ required: locked, authenticated: !locked });
    if (!locked) {
      const achievement = achievementsFixtureAnswer(method, path);
      if (achievement !== undefined) return ok(achievement);
      if (path === "/api/settings" && method === "GET") return ok({ schemaVersion: 1, status: "ok", settings });
      if (path === "/api/version") return ok({ version });
      if (path === "/api/bus/discover") return ok({ interfaces: [] });
      if (path === "/api/project/load-progress") return ok(null);
      if (path === "/api/project" && method === "GET") return fulfillNoProjectOpen(route);
      if (path === "/api/fs/list") return ok([{ name: "synthetic.knxproj", is_dir: false }]);
      if (path === "/api/project/import") return route.fulfill({ status: 422, contentType: "application/json", body: JSON.stringify({ error: raw, kind: "projectNotImportable" }) });
      if (path === "/api/log") return ok([backend, { ...backend, detail: "Distinct same-message occurrence" }]);
      if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    }
    unexpected.push(`${method} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"unexpected fixture request"}' });
  });
  return { unexpected, requests, raw, backend };
}
async function capture(page: Page, name: string) {
  if (process.env.LOGIN_FEEDBACK_SCREENSHOTS) await page.screenshot({ path: `${process.env.LOGIN_FEEDBACK_SCREENSHOTS}/${name}.png` });
}
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) for (const theme of ["graphite", "porcelain"] as const) {
  const id = `${language}-${width}-${theme}`;
  test(`${id}: login has loaded branding and build version without authenticated reads`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const f = await fixture(page, language, theme, true);
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    const logo = page.locator(".login-logo");
    await expect(logo).toBeVisible();
    expect(await logo.evaluate(image => (image as HTMLImageElement).complete && (image as HTMLImageElement).naturalWidth > 0)).toBe(true);
    await expect(page.locator(".login-version")).toHaveText(`v${version}`);
    await expect(page.locator(".login-title")).toHaveText("KNXBench");
    await expect(page.locator("#login-password")).toBeFocused();
    const box = await page.locator(".login-panel").boundingBox();
    expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
    await capture(page, `login-${id}`);
    // Development StrictMode may repeat the unauthenticated status probe.
    expect(f.unexpected).toEqual([]); expect(f.requests.length).toBeGreaterThan(0);
    expect([...new Set(f.requests)]).toEqual(["GET /api/auth/status"]);
  });
  test(`${id}: toast opens its exact record; backend messages expose selectable and copyable details`, async ({ page, context }) => {
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);
    await page.setViewportSize({ width, height: 900 });
    const m = language === "de" ? de : en;
    const f = await fixture(page, language, theme, false);
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("lang", language);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(page.getByRole("button", { name: m["toolbar.commands"], exact: true })).toBeVisible();
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.openProject"]);
    await page.getByRole("option").filter({ hasText: m["toolbar.openProject"] }).click();
    await page.getByRole("button", { name: "synthetic.knxproj", exact: true }).click();
    const toast = page.locator(".toast--error");
    await expect(toast).toHaveAttribute("role", "alert");
    await toast.getByRole("button", { name: m["toast.openMessage"], exact: true }).focus();
    await page.keyboard.press("Enter");
    const details = page.getByRole("dialog", { name: m["logPanel.details"], exact: true });
    await expect(details).toBeVisible();
    await expect(details.locator("textarea")).toBeFocused();
    const record = JSON.parse(await details.locator("textarea").inputValue());
    expect(record.source).toBe("ui:toast:error"); expect(record.message).toBe(f.raw);
    await expect(page.locator(".log-entry--selected")).toContainText("ui:toast:error");
    await details.getByRole("button", { name: m["logPanel.copyMessage"], exact: true }).click();
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(f.raw);
    await capture(page, `toast-details-${id}`);
    await details.getByRole("button", { name: m["logPanel.closeDetails"], exact: true }).click();
    await expect(details).toBeHidden();
    await expect(page.locator('.log-entry[data-log-key^="ui:"] .log-entry-open')).toBeFocused();
    const row = page.locator('.log-entry[data-log-key^="server:0:"]');
    await row.locator(".log-entry-open").click();
    await expect(details).toBeVisible();
    expect(JSON.parse(await details.locator("textarea").inputValue())).toEqual(f.backend);
    await details.getByRole("button", { name: m["logPanel.copyAll"], exact: true }).click();
    expect(JSON.parse(await page.evaluate(() => navigator.clipboard.readText()))).toEqual(f.backend);
    await details.locator("textarea").focus(); await page.keyboard.press("Control+A");
    expect(await details.locator("textarea").evaluate(field => (field as HTMLTextAreaElement).selectionEnd - (field as HTMLTextAreaElement).selectionStart)).toBe((await details.locator("textarea").inputValue()).length);
    const box = await details.boundingBox();
    expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width);
    expect(box!.y).toBeGreaterThanOrEqual(0); expect(box!.y + box!.height).toBeLessThanOrEqual(900);
    await capture(page, `backend-details-${id}`);
    await page.keyboard.press("Escape");
    // #13: one small Export… button; the scope is chosen in its dialog.
    await page.locator(".log-panel-export").click();
    await page.locator('.log-export-panel input[value="all"]').check();
    const downloading = page.waitForEvent("download");
    await page.locator(".log-export-confirm").click();
    const download = await downloading;
    expect(download.suggestedFilename()).toBe("session-log.json");
    const path = await download.path(); expect(path).not.toBeNull();
    const exported = JSON.parse(await readFile(path!, "utf8"));
    expect(exported.version).toBe(1); expect(exported.capacity).toBe(16000);
    expect(exported.entries).toEqual([f.backend, { ...f.backend, detail: "Distinct same-message occurrence" }]); // Never add the UI toast as a fake server record.
    await page.keyboard.press("Escape"); await expect(details).toBeHidden();
    expect(f.unexpected).toEqual([]); expect(f.requests.filter(r => r === "POST /api/project/import")).toHaveLength(1);
  });
}

for (const target of ["toast", "log-row"] as const) {
  test(`${target}: message action stays flat on hover inside its bounded container`, async ({ page }) => {
    await page.setViewportSize({ width: 400, height: 900 });
    const f = await fixture(page, "en", "graphite", false);
    await page.goto("/");
    await page.getByRole("button", { name: en["toolbar.commands"], exact: true }).click();
    await page.getByRole("combobox").fill(en["toolbar.openProject"]);
    await page.getByRole("option").filter({ hasText: en["toolbar.openProject"] }).click();
    await page.getByRole("button", { name: "synthetic.knxproj", exact: true }).click();
    const toastAction = page.locator(".toast--error").getByRole("button", { name: en["toast.openMessage"], exact: true });
    if (target === "log-row") {
      await toastAction.click();
      await page.getByRole("dialog", { name: en["logPanel.details"], exact: true })
        .getByRole("button", { name: en["logPanel.closeDetails"], exact: true }).click();
    }
    const action = target === "toast" ? toastAction : page.locator('.log-entry[data-log-key^="server:0:"] .log-entry-open');
    await action.hover();
    expect(await action.evaluate(async node => {
      getComputedStyle(node).transform; // Flush styles before waiting for finite transitions.
      await Promise.all(node.getAnimations().filter(animation => animation.effect?.getComputedTiming().iterations !== Infinity).map(animation => animation.finished));
      const style = getComputedStyle(node);
      return { transform: style.transform, shadow: style.boxShadow };
    })).toEqual({ transform: "none", shadow: "none" });
    expect(f.unexpected).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) {
  test(`${language}-400: simulated Clipboard API refusal selects exact raw details for manual Ctrl+C`, async ({ page, context }) => {
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);
    await page.addInitScript(() => Object.defineProperty(navigator.clipboard, "writeText", {
      value: () => Promise.reject(new Error("synthetic Clipboard API refusal")),
    }));
    await page.setViewportSize({ width: 400, height: 900 });
    const m = language === "de" ? de : en;
    const f = await fixture(page, language, "graphite", false);
    await page.goto("/");
    // Wait for the mounted workbench, not only document navigation/auth fetch.
    await page.getByRole("button", { name: m["toolbar.commands"], exact: true }).click();
    await page.getByRole("combobox").fill(m["toolbar.openProject"]);
    await page.getByRole("option").filter({ hasText: m["toolbar.openProject"] }).click();
    await page.getByRole("button", { name: "synthetic.knxproj", exact: true }).click();
    await page.locator(".toast--error").getByRole("button", { name: m["toast.openMessage"], exact: true }).click();
    const details = page.getByRole("dialog", { name: m["logPanel.details"], exact: true });
    const text = details.locator("textarea");
    const raw = JSON.parse(await text.inputValue());
    expect(raw.message).toBe(f.raw);
    await details.getByRole("button", { name: m["logPanel.copyAll"], exact: true }).click();
    await expect(details.getByRole("alert")).toHaveText(m["logPanel.copyFailed"]);
    await expect(text).toBeFocused();
    expect(await text.evaluate(field => (field as HTMLTextAreaElement).selectionStart)).toBe(0);
    expect(await text.evaluate(field => (field as HTMLTextAreaElement).selectionEnd)).toBe((await text.inputValue()).length);
    await page.keyboard.press("Control+C");
    expect(JSON.parse(await page.evaluate(() => navigator.clipboard.readText()))).toEqual(raw);
    await capture(page, `clipboard-fallback-${language}-400`);
    expect(f.unexpected).toEqual([]);
  });
}
