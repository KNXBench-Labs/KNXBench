/** Built-App Reko boundaries and exact captured-response joins in EN/DE at 400/1440 pixels. */
import { expect, test, type Locator, type Page, type TestInfo } from "@playwright/test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { onlineFixture, openOnline, dismissOnlineStartupToasts, MODERN, LEGACY, BACKUP } from "./online-catalogue-fixture";
import { ONLINE_BASE } from "../src/onlineCatalogueApi";

const captureRoot = process.env.KNXBENCH_REKO_CAPTURE_DIR;
async function fit(page: Page, surface: Locator, width: number) {
  await expect(surface).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  expect(await surface.evaluate(node => node.scrollWidth - node.clientWidth)).toBeLessThanOrEqual(1);
  const box = (await surface.boundingBox())!;
  expect(box.x).toBeGreaterThanOrEqual(0); expect(box.x + box.width).toBeLessThanOrEqual(width + 1);
}
async function shot(surface: Locator, info: TestInfo, name: string) {
  await surface.scrollIntoViewIfNeeded();
  await surface.screenshot({ path: info.outputPath(`${name}.png`) });
}
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}: manual sync, deferred login, verified download and consented server feed`, async ({ page }, info) => {
    const { m, state: f } = await onlineFixture(page, language, width);
    const errors: string[] = []; page.on("pageerror", error => errors.push(error.message));
    expect(f.calls.some(call => call.path.startsWith("/api/online-catalogue"))).toBe(false);
    const { online, catalogue } = await openOnline(page, language);
    // StrictMode may replay read-only mount effects; no other route or method is allowed.
    expect(new Set(f.calls.filter(call => call.path.startsWith("/api/online-catalogue")).map(call => `${call.method} ${call.path}`)))
      .toEqual(new Set(["GET /api/online-catalogue", `GET ${ONLINE_BASE}`]));
    await online.getByRole("button", { name: m["online.sync"], exact: true }).click();
    await expect(online).toContainText(m["online.notModified"]);
    await expect(online).toContainText("2026-01-03T00:00:00Z");
    await online.getByRole("textbox", { name: m["online.searchLabel"], exact: true }).fill("FC-4");
    await online.getByRole("button", { name: m["online.search"], exact: true }).click();
    await online.getByRole("button", { name: /^Fictional dimmer/ }).click();
    await expect(online.locator(".online-catalogue-details")).toContainText(language === "de" ? "Applikation 0001 · Version 02 · Maske 0701" : "Application 0001 · version 02 · mask 0701");
    await fit(page, online, width); await shot(online, info, "selected");
    await shot(online.locator(".online-catalogue-details"), info, "entry-details");
    await online.getByRole("button", { name: m["online.download"], exact: true }).click();
    const account = page.getByRole("dialog", { name: m["online.account"], exact: true });
    await expect(account).toContainText(MODERN.fileName); await fit(page, account, width); await shot(account, info, "account");
    const closeAccount = account.getByRole("button", { name: m["online.close"], exact: true });
    await closeAccount.scrollIntoViewIfNeeded(); await expect(closeAccount).toBeInViewport();
    await shot(account, info, "account-footer");
    expect(f.calls.filter(call => call.path.includes("/entries/"))).toEqual([]);
    await account.getByRole("textbox", { name: m["online.user"], exact: true }).fill("fictional-user");
    await account.getByLabel(m["online.password"], { exact: true }).fill("fictional-account-secret");
    await account.getByRole("button", { name: m["online.saveAccount"], exact: true }).click();
    await expect(account).toHaveCount(0);
    await expect(online).toContainText(m["online.downloaded"]);
    expect(f.jobs.creates).toEqual([]); expect(f.jobs.applies).toEqual([]);
    expect(f.calls.filter(call => call.path.includes("/entries/"))).toEqual([{ method: "POST", path: `${ONLINE_BASE}/entries/${MODERN.id}/download?sha256=${MODERN.sha256}`, body: null }]);
    expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain("fictional-account-secret");
    await online.getByRole("button", { name: m["online.import"], exact: true }).click();
    const selection = online.locator("[data-catalogue-job-selection]");
    await selection.getByRole("combobox", { name: m["catalogJob.profile"], exact: true }).selectOption("large");
    await expect(selection.getByRole("button", { name: m["catalogJob.start"], exact: true })).toBeDisabled();
    await selection.getByRole("checkbox", { name: m["catalogJob.largeRisk"], exact: true }).check();
    expect(f.jobs.creates).toEqual([]);
    await selection.getByRole("button", { name: m["catalogJob.start"], exact: true }).click();
    const job = catalogue.locator("section.catalogue-job");
    await expect(job).toContainText(m["catalogUpdate.nothingYet"]);
    expect(f.jobs.creates).toHaveLength(1); expect(f.jobs.uploads).toEqual([]); expect(f.jobs.feeds).toHaveLength(1);
    expect(f.jobs.creates[0]).toMatchObject({ sourceName: MODERN.fileName, declaredBytes: String(MODERN.size), profile: "large", allowLargePackage: true });
    expect(f.jobs.applies).toEqual([]); await fit(page, job, width); await shot(job, info, "preview");
    const apply = job.getByRole("button", { name: m["catalogUpdate.apply"], exact: true });
    await expect(apply).toBeDisabled(); await job.getByRole("checkbox", { name: m["catalogJob.confirm"], exact: true }).check(); await apply.click();
    await expect(job).toContainText(m["catalogJob.state.succeeded"]); await expect(job).toContainText(BACKUP);
    expect(f.jobs.applies).toHaveLength(1); expect(f.unexpected).toEqual([]); expect(errors).toEqual([]);
  });

  test(`${language} ${width}: cancellation waits for terminal status and stale selection never retries`, async ({ page }, info) => {
    const { m, state: f } = await onlineFixture(page, language, width);
    f.status = { ...f.status, credentials: { state: "stored", mode: "token", user: null, problem: null } }; f.downloadMode = "running";
    const { online } = await openOnline(page, language);
    await online.getByRole("button", { name: m["online.search"], exact: true }).click();
    await online.getByRole("button", { name: /^Fictional dimmer/ }).click();
    await online.getByRole("button", { name: m["online.download"], exact: true }).click();
    await online.getByRole("button", { name: m["online.cancelDownload"], exact: true }).click();
    await expect(online).toContainText(m["online.cancelling"].replace("{entry}", MODERN.id));
    await expect(online.getByRole("button", { name: m["online.cancelDownload"], exact: true })).toBeDisabled();
    await shot(online.locator(".online-catalogue-progress"), info, "stopping");
    f.status = { ...f.status, download: { ...f.status.download, state: "failed", error: { kind: "cancelled" } } };
    await expect(online.getByRole("alert")).toHaveCount(1);
    await expect(online.getByRole("alert")).toContainText(m["online.error.cancelled"]);
    f.downloadMode = "selectionChanged";
    await online.getByRole("button", { name: m["online.download"], exact: true }).click();
    await expect(online.getByRole("alert")).toHaveCount(1);
    await expect(online.getByRole("alert")).toContainText(m["online.error.selectionChanged"]);
    await fit(page, online, width); await shot(online.locator(".online-catalogue-error"), info, "stale-selection");
    expect(f.calls.filter(call => call.method === "POST" && call.path.includes("/download?"))).toHaveLength(2);
    f.downloadMode = "dailyLimit";
    f.status = { ...f.status, limit: { kind: "dailyLimit", resetAt: "2026-01-04T00:00:00Z", retryAfterSecs: 30 } };
    await online.getByRole("button", { name: m["online.download"], exact: true }).click();
    await expect(online.getByRole("alert")).toHaveCount(1);
    await expect(online.getByRole("alert")).toContainText(m["online.error.dailyLimit"]);
    await expect(online.getByRole("alert")).toContainText("2026-01-04T00:00:00Z");
    await expect(online.getByRole("button", { name: m["online.download"], exact: true })).toBeDisabled();
    await fit(page, online, width); await shot(online.getByRole("alert"), info, "quota");
    expect(f.calls.filter(call => call.method === "POST" && call.path.includes("/download?"))).toHaveLength(3);
    expect(f.jobs.creates).toEqual([]); expect(f.legacy).toEqual([]); expect(f.unexpected).toEqual([]);
  });

  test(`${language} ${width}: local pagination, unsupported VD1 and separate legacy file password`, async ({ page }, info) => {
    const { m, state: f } = await onlineFixture(page, language, width);
    const { online } = await openOnline(page, language);
    await online.getByRole("button", { name: m["online.search"], exact: true }).click();
    await online.getByRole("button", { name: /^Fictional unsupported/ }).click();
    await expect(online.locator(".online-catalogue-details")).toContainText(m["online.unsupported"]);
    await expect(online.getByRole("button", { name: m["online.import"], exact: true })).toHaveCount(0);
    await online.getByRole("button", { name: m["online.next"], exact: true }).click();
    await expect(online).toContainText("Fictional page two"); await expect(online.locator(".online-catalogue-details")).toHaveCount(0);
    await online.getByRole("button", { name: m["online.previous"], exact: true }).click();
    await online.getByRole("button", { name: /^Fictional legacy/ }).click();
    await online.getByRole("button", { name: m["online.import"], exact: true }).click();
    const password = page.getByRole("dialog", { name: language === "de" ? "Passwort für fictional.vd4" : "Password for fictional.vd4", exact: true });
    await expect(password).toBeVisible(); await fit(page, password, width); await shot(password, info, "legacy-password");
    await password.getByLabel(language === "de" ? "Passwort" : "Password", { exact: true }).fill("fictional-file-secret");
    await password.getByRole("checkbox").check();
    await password.getByRole("button", { name: language === "de" ? "Importieren" : "Import", exact: true }).click();
    await expect(password).toHaveCount(0);
    await expect(online).toContainText(language === "de" ? "Alte Produktdatenbank importiert" : "Legacy product database imported");
    expect(f.legacy).toEqual([{ password: null, remember: false }, { password: "fictional-file-secret", remember: true }]);
    expect(f.calls.filter(call => call.path.includes("/install-legacy?"))).toEqual(f.legacy.map(body => ({ method: "POST", path: `${ONLINE_BASE}/entries/${LEGACY.id}/install-legacy?sha256=${LEGACY.sha256}`, body })));
    expect(f.jobs.creates).toEqual([]); expect(f.calls.filter(call => call.path.endsWith("/credentials"))).toEqual([]); expect(f.unexpected).toEqual([]);
  });

  test(`${language} ${width}: settings account, token clearing and changed-origin status`, async ({ page }, info) => {
    const { m, state: f } = await onlineFixture(page, language, width);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("option", { name: new RegExp(`^${m["toolbar.settings"]}`) }).click();
    const section = page.locator(".online-catalogue-settings");
    await dismissOnlineStartupToasts(page, language);
    expect(f.calls.filter(call => call.path.startsWith(ONLINE_BASE))).toEqual([]);
    await section.getByRole("button", { name: m["online.account"], exact: true }).click();
    await section.getByRole("combobox", { name: m["online.authMode"], exact: true }).selectOption("token");
    await section.getByLabel(m["online.token"], { exact: true }).fill("fictional-settings-token");
    await section.getByRole("button", { name: m["online.saveAccount"], exact: true }).click();
    await expect(section).toContainText(m["online.saved"]);
    await expect(section.getByLabel(m["online.token"], { exact: true })).toHaveValue("");
    expect(JSON.stringify(f.settingsWrites)).not.toContain("fictional-settings-token");
    expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain("fictional-settings-token");
    await section.getByLabel(m["online.baseUrl"], { exact: true }).fill("https://different.example/");
    await section.getByRole("button", { name: m["online.saveAddress"], exact: true }).click();
    await expect(section).toContainText(m["online.secretProblem"]);
    await expect(section.getByRole("link", { name: m["online.registration"], exact: true })).toHaveAttribute("href", "https://different.example/");
    await fit(page, section, width); await shot(section, info, "settings");
    await section.getByRole("button", { name: m["online.forget"], exact: true }).click();
    await expect(section).toContainText(m["online.credentialsEmpty"]);
    expect(f.calls.filter(call => call.path.endsWith("/config"))).toEqual([{ method: "PUT", path: `${ONLINE_BASE}/config`, body: { baseUrl: "https://different.example/" } }]);
    expect(f.unexpected).toEqual([]);
  });

  // No default skip record: capture-backed cases exist only when the producer supplies inputs.
  if (captureRoot) test(`${language} ${width}: original mounted backend status/search bytes reach the built UI`, async ({ page }, info) => {
    const statusPath = join(captureRoot, "status-synced.json"); const searchPath = join(captureRoot, "search-program-versions.json");
    const status = readFileSync(statusPath); const search = readFileSync(searchPath);
    const digest = (bytes: Buffer) => createHash("sha256").update(bytes).digest("hex");
    const before = [digest(status), digest(search)];
    const { m, state: f } = await onlineFixture(page, language, width, { status, search });
    const { online } = await openOnline(page, language);
    await online.getByRole("button", { name: m["online.search"], exact: true }).click();
    await online.getByRole("button", { name: /^Fictional capture-modern/ }).click();
    await expect(online).toContainText("Contoso Lighting");
    const details = online.locator(".online-catalogue-details");
    await expect(details).toContainText("Dimmer 4-fold");
    for (const version of ["01", "02"]) {
      await expect(details).toContainText(language === "de" ? `Applikation 0001 · Version ${version} · Maske 0000` : `Application 0001 · version ${version} · mask 0000`);
    }
    await online.getByRole("button", { name: m["online.refreshStatus"], exact: true }).click();
    await expect(details).toContainText("Fictional capture-modern");
    await expect(online.getByRole("button", { name: /^Fictional capture-vd1/ })).toContainText(m["online.unsupported"]);
    await expect(online.getByRole("button", { name: /^Fictional capture-vd2/ })).toContainText(m["online.unsupported"]);
    await fit(page, online, width); await shot(details, info, "producer-join");
    expect(f.calls.filter(call => call.path.startsWith(ONLINE_BASE)).every(call => call.method === "GET")).toBe(true);
    expect(f.unexpected).toEqual([]);
    expect([digest(readFileSync(statusPath)), digest(readFileSync(searchPath))]).toEqual(before);
  });
}
