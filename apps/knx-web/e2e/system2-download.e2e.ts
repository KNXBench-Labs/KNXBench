/** Intercepted real-app wiring evidence, not backend or hardware acceptance. */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test, type Page } from "@playwright/test";
import { messages as de } from "../src/messages/de";
import { dismissToasts, openProject } from "./communication-fixture";
import { openNavigationSections } from "./disclosure-fixture";
import { preview, status, recoveryStatus, restoreFailedStatus, downloadFailedStatus, wireV1, finalStepStatus, finalDownloadMismatch, finalRestoreRead } from "../src/system2DownloadTestFixtures";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { System2Status, System2StatusWireV1 } from "../src/system2DownloadApi";
import { messages as en } from "../src/messages/en";
const tree: ProjectTree = { schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "server", project_incarnation: 1, snapshot_revision: 1,
  installations: [{ id: 1, name: "Installation", buildings: [], unassigned: [], group_addresses: [], group_ranges: [], topology: [{ id: 2, name: "Area", address: 1, lines: [{ id: 3, name: "Line", address: 1, devices: [{ id: 10, name: "Configured button", address: "1.1.67", description: null, com_object_count: 0 }] }] }] }] };

async function interceptSystem2(page: Page, language: "en" | "de" = "en") {
  const unexpected: string[] = []; const commands: { path: string; body: unknown }[] = [];
  const state = { projectOpen: false, operation: null as System2Status | System2StatusWireV1 | null, raw: null as string | null, reportRaw: null as string | null, busyLocks: [] as string[], report: null as unknown, reports: [] as string[] };
  await page.route("**/api/**", (route) => {
    const request = route.request(); const path = new URL(request.url()).pathname; const method = request.method();
    const json = (body: unknown, code = 200) => route.fulfill({ status: code, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status") return json({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return json({ schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, releaseChecksEnabled: false, motionLevel: "off", onboardingGuide: { seenStage: "alpha" },
    } });
    if (path === "/api/project" && method === "GET") return state.projectOpen ? json(tree) : json({ error: "no project" }, 400);
    if (path === "/api/project/new" && method === "POST") { state.projectOpen = true; return json(tree); }
    if (path === "/api/bus/discover") return json({ interfaces: [] });
    if (path === "/api/bus/activity") return json({ serverIncarnation: "server", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: state.busyLocks, untracked: [] });
    if (path === "/api/device-readiness") return json({ devices: [], counts: {} });
    if (path === "/api/device-download/status") return json({ error: "no ordinary download" }, 404);
    if (path === "/api/product-languages" && method === "GET") return json([]);
    if (method === "GET" && ["/api/achievements", "/api/project/status-metrics", "/api/bus/monitor/telegrams"].includes(path)) return json({ error: "Not available in this isolated fixture" }, 404);
    if (path === "/api/version") return json({ version: "0.1.0-alpha.16" });
    if (path === "/api/device-system2/status" && method === "GET") {
      // A captured body is served byte for byte; it is never parsed or patched here.
      if (state.raw !== null) return route.fulfill({ status: 200, contentType: "application/json", body: state.raw });
      return json(state.operation);
    }
    if (path === "/api/device-system2/report" && method === "GET") {
      state.reports.push(new URL(request.url()).search);
      // A captured report is served byte for byte, like a captured status.
      if (state.reportRaw !== null) return route.fulfill({ status: 200, contentType: "application/json", body: state.reportRaw });
      return json(state.report);
    }
    if (path.startsWith("/api/device-system2/") && method === "POST") {
      commands.push({ path, body: request.postDataJSON() });
      if (path.endsWith("/prepare")) return json(preview);
      if (path.endsWith("/start")) { state.operation = { ...status, backupAvailable: true }; return json(state.operation); }
      if (path.endsWith("/cancel")) { state.operation = { ...recoveryStatus, state: "cancelled" }; return json(state.operation); }
      if (path.endsWith("/recover")) { state.operation = { ...status, state: "restored", completedSteps: 2, totalSteps: 2, backupAvailable: true }; return json(state.operation); }
    }
    if (method === "GET" && ["/api/recent-projects", "/api/catalog", "/api/manufacturers", "/api/product-databases", "/api/notes", "/api/library", "/api/project/sources"].includes(path)) return json([]);
    unexpected.push(`${method} ${path}`); return json({ error: "Unexpected fixture request" }, 404);
  });
  return { state, commands, unexpected };
}

test("Download mode → configured preview → ordinary consent → status/cancel → explicit original recovery", async ({ page }) => {
  const { commands, unexpected } = await interceptSystem2(page);
  await page.setViewportSize({ width: 1440, height: 1000 }); await page.goto("/");
  await page.getByRole("button", { name: "New project…" }).click();
  await page.getByRole("dialog", { name: "New project" }).getByRole("button", { name: "Create project" }).click();
  await page.getByRole("dialog", { name: "Project created" }).getByRole("button", { name: "Done" }).click();
  await openNavigationSections(page);
  await page.getByRole("navigation", { name: "Bus monitor" }).getByRole("button", { name: "Bus monitor" }).first().click();
  await page.getByRole("button", { name: "Download to device", exact: true }).click();
  await page.locator("[data-download-device]").selectOption("10"); await page.locator("[data-download-gateway]").fill("192.0.2.1:3671");
  await page.locator("[data-download-mode]").selectOption("system2");
  await expect(page.locator("[data-system2-prepare]")).toBeEnabled(); expect(commands).toEqual([]);
  await page.locator("[data-system2-prepare]").click();
  const shown = page.locator("[data-system2-preview]"); await expect(shown).toContainText(preview.configurationSha256);
  await expect(shown).toContainText(preview.warnings[0]); await expect(page.locator("[data-system2-start]")).toBeDisabled();
  expect(commands).toEqual([{path: "/api/device-system2/prepare", body: {deviceId:10, sourceMode:"retainedSources"}}]);
  await page.locator("[data-system2-track]").selectOption("knxbenchBestEffort");
  for (const [key, phrase] of Object.entries({ confirmation: preview.confirmationPhrase, hypothesisConfirmation: preview.hypothesisPhrase, downloadConfirmation: preview.downloadPhrase, releaseConfirmation: preview.releasePhrase })) await page.locator(`[data-system2-phrase='${key}']`).fill(phrase);
  await page.locator("[data-system2-start]").scrollIntoViewIfNeeded();
  if (process.env.SYSTEM2_UI_CAPTURE) await page.locator("[data-system2-consent]").screenshot({ path: `${process.env.SYSTEM2_UI_CAPTURE}/configured-preview.png` });
  await page.locator("[data-system2-start]").click();
  const consent = page.locator(".programming-consent-dialog"); await expect(consent).toBeVisible(); expect(commands).toHaveLength(1);
  await expect(consent).toContainText("1.1.67 — Configured button");
  await consent.getByRole("button", { name: en["programmingConsent.confirm"], exact: true }).click();
  await expect(page.locator("[data-system2-status]")).toContainText(status.operationId); await page.locator("[data-system2-cancel]").click();
  await expect(page.locator("[data-system2-status]")).toContainText(en["system2.state.cancelled"]);
  expect(commands).toHaveLength(3);
  for (const [key, phrase] of Object.entries({ recoveryConfirmation: "Recover original actual state", downloadConfirmation: preview.downloadPhrase, releaseConfirmation: preview.releasePhrase })) await page.locator(`[data-system2-recovery='${key}']`).fill(phrase);
  await page.locator("[data-system2-recover]").click(); await expect(consent).toBeVisible(); expect(commands).toHaveLength(3);
  await consent.getByRole("button", { name: en["programmingConsent.confirm"], exact: true }).click();
  await expect(page.locator("[data-system2-status]")).toContainText(en["system2.state.restored"]);
  expect(commands[3]).toEqual({path: "/api/device-system2/recover", body: {operationId:status.operationId, gateway:"192.0.2.1:3671", recoveryConfirmation:"Recover original actual state", downloadConfirmation:preview.downloadPhrase, releaseConfirmation:preview.releasePhrase}});
  if (process.env.SYSTEM2_UI_CAPTURE) await page.locator("[data-system2-status]").screenshot({ path: `${process.env.SYSTEM2_UI_CAPTURE}/explicit-recovery.png` });
  expect(unexpected).toEqual([]);
});

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} own System-2 locks and pre-journal refusal at ${width}px`, async ({ page }) => {
    const m = language === "en" ? en : de;
    const foreign: string[] = [];
    await page.route("**/*", route => {
      const url = new URL(route.request().url());
      if (url.hostname === "127.0.0.1") return route.continue();
      foreign.push(url.origin); return route.abort();
    });
    const { state, commands, unexpected } = await interceptSystem2(page, language);
    state.busyLocks = ["managementOperation"];
    await page.setViewportSize({ width, height: 1000 });
    await openProject(page, language); await dismissToasts(page, language);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
    await page.getByRole("combobox").press("Enter");
    await page.getByRole("button", { name: m["deviceDownload.title"], exact: true }).click();
    await page.locator("[data-download-mode]").selectOption("system2");
    // Positive control: a genuine foreign management lock is still shown.
    await expect(page.locator(".device-download-holder")).toContainText(m["gatewayRefusal.busy.connecting.managementOperation"]);
    state.operation = status;
    state.busyLocks = ["system2Operation", "deviceDownload", "managementOperation", "busMonitorOrGroupWrite", "lineScan"];
    await page.locator("[data-system2-refresh]").click();
    await expect(page.locator("[data-system2-status]")).toContainText(status.operationId);
    await expect(page.locator(".device-download-holder")).toHaveCount(0);
    state.operation = { ...status, state: "failed", refusal: "system2ActivityUnavailable" };
    state.busyLocks = [];
    await page.locator("[data-system2-refresh]").click();
    const reason = page.locator("[data-system2-refusal]");
    await expect(reason).toHaveText(m["system2.refusal"].replace("{reason}", "system2ActivityUnavailable"));
    await expect(page.locator("[data-system2-nothing-written]")).toBeVisible();
    await reason.scrollIntoViewIfNeeded();
    const box = await reason.boundingBox(); expect(box).not.toBeNull();
    expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    if (process.env.SYSTEM2_UI_CAPTURE) await page.locator("[data-system2-status]").screenshot({
      path: `${process.env.SYSTEM2_UI_CAPTURE}/refusal-${language}-${width}.png`,
    });
    expect(commands).toEqual([]); expect(unexpected).toEqual([]); expect(foreign).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} safe pointer-failure diagnosis reaches the mounted panel at ${width}px`, async ({ page }) => {
    const m = language === "en" ? en : de;
    const { state, commands, unexpected } = await interceptSystem2(page, language);
    await page.setViewportSize({ width, height: 1000 });
    await openProject(page, language); await dismissToasts(page, language);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
    await page.getByRole("combobox").press("Enter");
    await page.getByRole("button", { name: m["deviceDownload.title"], exact: true }).click();
    await page.locator("[data-download-mode]").selectOption("system2");
    const refresh = async (operation: System2Status | System2StatusWireV1 | null) => { state.operation = operation; await page.locator("[data-system2-refresh]").click(); };
    const observed = page.locator("[data-system2-diagnostics]");
    // Both runs: the historical download stays distinct from the current restore reason.
    await refresh(restoreFailedStatus);
    await expect(observed).toContainText(m["system2.diagnostics"]);
    await expect(page.locator("[data-system2-diagnostic='download']")).toContainText(m["system2.diagnostic.reason.unsupportedWidth"]);
    await expect(page.locator("[data-system2-diagnostic='restore']")).toContainText(m["system2.diagnostic.reason.elementCount"]);
    await expect(page.locator("[data-system2-diagnostic='restore']")).toContainText(m["system2.diagnostic.run.restore"]);
    const shown = (await observed.innerText());
    for (const raw of ["unsupportedWidth", "elementCount", "zeroElementsWithData", "propertyId", "/private", "recovery-key"]) expect(shown).not.toContain(raw);
    const box = await observed.boundingBox(); expect(box).not.toBeNull();
    expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    if (process.env.SYSTEM2_UI_CAPTURE) await observed.screenshot({ path: `${process.env.SYSTEM2_UI_CAPTURE}/diagnosis-${language}-${width}.png` });
    // Only the prior download is known: the restore reason is shown as unavailable.
    await refresh({ ...restoreFailedStatus, failureDiagnostics: restoreFailedStatus.failureDiagnostics.slice(0, 1) });
    await expect(page.locator("[data-system2-diagnostic='download']")).toBeVisible();
    await expect(page.locator("[data-system2-diagnostic='restore']")).toHaveCount(0);
    await expect(page.locator("[data-system2-diagnostic-unavailable='restore']")).toBeVisible();
    // An old v1 status carries no diagnosis: unavailable, and never inferred from the message.
    await refresh({ ...wireV1(downloadFailedStatus), refusal: "recoveryRefused", message: "unsupported width: 3 octets" });
    await expect(page.locator("[data-system2-diagnostic-unavailable='download']")).toBeVisible();
    await expect(page.locator("[data-system2-diagnostic]")).toHaveCount(0);
    // A new operation never keeps a stale diagnosis.
    await refresh(downloadFailedStatus);
    await expect(page.locator("[data-system2-diagnostic='download']")).toBeVisible();
    await refresh({ ...status, operationId: "f".repeat(32) });
    await expect(page.locator("[data-system2-status]")).toContainText("f".repeat(32));
    await expect(observed).toHaveCount(0);
    expect(commands).toEqual([]); expect(unexpected).toEqual([]);
  });
}

// Opt-in join against the backend's own emitted status bodies (private capture directory, never committed).
// Source statuses: download object 1 elementCount (2 elements, 2 octets); restore object 2 unsupportedWidth (1 element, 3 octets).
const capture = process.env.KNXBENCH_S2_DIAGNOSTIC_CAPTURE;
const download = { run: "download", reason: "elementCount", object: 1, elements: 2, octets: 2 } as const;
const restore = { run: "restore", reason: "unsupportedWidth", object: 2, elements: 1, octets: 3 } as const;
const CAPTURED = [
  ["download-failed.json", [download]], ["reopened-download.json", [download]], ["restoring.json", [download]],
  ["restore-failed.json", [download, restore]], ["reopened-restore.json", [download, restore]],
] as const;
// Register private-capture witnesses only for the explicit opt-in invocation. The default
// gate must contain only executable cases, never skipped placeholders for absent private input.
if (capture) for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} backend-emitted status bodies render their two labeled reasons at ${width}px`, async ({ page }, testInfo) => {
    const m = language === "en" ? en : de;
    const { state, commands, unexpected } = await interceptSystem2(page, language);
    await page.setViewportSize({ width, height: 1000 });
    await openProject(page, language); await dismissToasts(page, language);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
    await page.getByRole("combobox").press("Enter");
    await page.getByRole("button", { name: m["deviceDownload.title"], exact: true }).click();
    await page.locator("[data-download-mode]").selectOption("system2");
    for (const [file, expected] of CAPTURED) {
      const body = readFileSync(join(capture!, file), "utf8");
      await testInfo.attach(file, { body, contentType: "application/json" });
      state.raw = body;
      // Two refreshes: the retained status must read back identically.
      for (let read = 0; read < 2; read++) {
        await page.locator("[data-system2-refresh]").click();
        const block = page.locator("[data-system2-diagnostics]");
        await expect(block).toBeVisible();
        await expect(block.locator("[data-system2-diagnostic]")).toHaveCount(expected.length);
        for (const d of expected) {
          const entry = m["system2.diagnostic.entry"]
            .replace("{run}", m[`system2.diagnostic.run.${d.run}`]).replace("{reason}", m[`system2.diagnostic.reason.${d.reason}`])
            .replace("{object}", String(d.object)).replace("{property}", "7").replace("{elements}", String(d.elements)).replace("{octets}", String(d.octets));
          await expect(block.locator(`[data-system2-diagnostic='${d.run}']`)).toHaveText(entry);
        }
        await expect(block.locator("[data-system2-diagnostic-unavailable]")).toHaveCount(0);
        const shown = await block.innerText();
        for (const raw of ["b7e5", "919293", "/home/", "/tmp/", "/mnt/", "elementCount", "unsupportedWidth"]) expect(shown).not.toContain(raw);
      }
      if (process.env.SYSTEM2_UI_CAPTURE) await page.locator("[data-system2-status]").screenshot({
        path: `${process.env.SYSTEM2_UI_CAPTURE}/emitted-${file.replace(".json", "")}-${language}-${width}.png`,
      });
    }
    const box = await page.locator("[data-system2-diagnostics]").boundingBox(); expect(box).not.toBeNull();
    expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(commands).toEqual([]); expect(unexpected).toEqual([]);
  });
}

// Opt-in join for the final-step cause: the router's own failed and reopened status bytes and its report,
// served untouched, through the production decoder, the real panel and the local report save.
// Source run: the download's final program-version read disagrees with the plan, so no pointer fact exists.
const FINAL_BODIES = ["final-failed.json", "final-reopened.json"] as const;
if (capture) for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} backend-emitted final-step bodies and report render and save unchanged at ${width}px`, async ({ page }, testInfo) => {
    const m = language === "en" ? en : de;
    const { state, commands, unexpected } = await interceptSystem2(page, language);
    const reportRaw = readFileSync(join(capture!, "final-report.json"), "utf8");
    state.reportRaw = reportRaw;
    await page.setViewportSize({ width, height: 1000 });
    await openProject(page, language); await dismissToasts(page, language);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
    await page.getByRole("combobox").press("Enter");
    await page.getByRole("button", { name: m["deviceDownload.title"], exact: true }).click();
    await page.locator("[data-download-mode]").selectOption("system2");
    const operationId = JSON.parse(readFileSync(join(capture!, FINAL_BODIES[0]), "utf8")).operationId as string;
    for (const file of FINAL_BODIES) {
      const body = readFileSync(join(capture!, file), "utf8");
      await testInfo.attach(file, { body, contentType: "application/json" });
      state.raw = body;
      // Two refreshes: the retained status must read back identically.
      for (let read = 0; read < 2; read++) {
        await page.locator("[data-system2-refresh]").click();
        await expect(page.locator("[data-system2-result-text]")).toContainText(m["system2.result.transferCompleteVerificationUnfinished"]);
        const section = page.getByRole("region", { name: m["system2.finalStep.heading"] });
        await expect(section).toBeVisible();
        await expect(section.locator("[data-system2-final-step]")).toHaveCount(1);
        const line = section.locator("[data-system2-final-step='download']");
        await expect(line).toHaveText(m["system2.finalStep.download"]
          .replace("{category}", m["system2.finalStep.category.verificationMismatch"]).replace("{stage}", m["system2.finalStep.stage.programVersion"]));
        await expect(line).toHaveAttribute("data-system2-final-category", "verificationMismatch");
        await expect(line).toHaveAttribute("data-system2-final-stage", "programVersion");
        await expect(section.locator("[data-system2-final-step='restore']")).toHaveCount(0);
        await expect(section.locator("[data-system2-final-step-unavailable]")).toHaveCount(0);
        const shown = await section.innerText();
        for (const raw of ["verificationMismatch", "programVersion", operationId, "/home/", "/tmp/", "/mnt/"]) expect(shown).not.toContain(raw);
      }
    }
    const download = page.waitForEvent("download");
    await page.locator("[data-system2-report-save]").click();
    const file = await download;
    expect(file.suggestedFilename()).toBe("knxbench-system2-report.json");
    const { readFile } = await import("node:fs/promises");
    const saved = await readFile((await file.path())!, "utf8");
    // The saved file is the emitted report, decoded and written with the panel's own two-space layout.
    expect(JSON.parse(saved)).toEqual(JSON.parse(reportRaw));
    expect(saved).toBe(JSON.stringify(JSON.parse(reportRaw), null, 2));
    expect(JSON.parse(saved).finalStepFailures).toEqual([{ run: "download", category: "verificationMismatch", stage: "programVersion" }]);
    for (const forbidden of [operationId, "recoveryPhrase", "local_diagnostic", "192.0.2.1"]) expect(saved).not.toContain(forbidden);
    await expect(page.locator("[data-system2-report-saved]")).toHaveText(m["system2.report.saved"]);
    expect(state.reports).toEqual([`?operationId=${operationId}`]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(commands).toEqual([]); expect(unexpected).toEqual([]);
  });
}

// Issue #31: the ended run's result is explained from the journal facts and its report is saved locally.
const unfinishedRun: System2Status = { ...downloadFailedStatus, failureDiagnostics: [], completedSteps: 26, totalSteps: 27, downloadResult: "transferCompleteVerificationUnfinished",
  downloadCompletedSteps: 26, downloadTotalSteps: 27, projectSettlement: "persisted" };
const unfinishedReport = { format: 3, kind: "knxbench-system2-download-summary", support: "EXPERIMENTAL BEST-EFFORT; HARDWARE COMPATIBILITY AND RECOVERY UNVERIFIED",
  evidence: "local-journal-only", download: { outcome: "failed", result: "transferCompleteVerificationUnfinished", completedSteps: 26, totalSteps: 27 },
  recovery: null, mutationPossible: true, prestateDiffers: false, failureDiagnostics: [], finalStepFailures: [], executionBuild: { state: "unknown" }, exporterBuild: "0.1.0-alpha.1+g1234abcd" };
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} unverified System-2 result and local report save at ${width}px`, async ({ page }) => {
    const m = language === "en" ? en : de;
    const { state, commands, unexpected } = await interceptSystem2(page, language);
    state.operation = unfinishedRun; state.report = unfinishedReport;
    await page.setViewportSize({ width, height: 1000 });
    await openProject(page, language); await dismissToasts(page, language);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
    await page.getByRole("combobox").press("Enter");
    await page.getByRole("button", { name: m["deviceDownload.title"], exact: true }).click();
    await page.locator("[data-download-mode]").selectOption("system2");
    const result = page.locator("[data-system2-result-text]");
    await expect(result).toContainText(m["system2.result.transferCompleteVerificationUnfinished"]);
    await expect(page.locator("[data-system2-report-guidance]")).toContainText("issue@knxbench.com");
    await expect(page.locator("[data-system2-settlement='persisted']")).toContainText(m["system2.settlement.persisted"]);
    const download = page.waitForEvent("download");
    await page.locator("[data-system2-report-save]").click();
    const file = await download;
    expect(file.suggestedFilename()).toBe("knxbench-system2-report.json");
    const { readFile } = await import("node:fs/promises");
    const text = await readFile((await file.path())!, "utf8");
    expect(JSON.parse(text)).toEqual(unfinishedReport);
    for (const forbidden of [unfinishedRun.address, unfinishedRun.deviceName, unfinishedRun.operationId, "recoveryPhrase", "192.0.2.1"]) expect(text).not.toContain(forbidden);
    await expect(page.locator("[data-system2-report-saved]")).toHaveText(m["system2.report.saved"]);
    expect(state.reports).toEqual([`?operationId=${unfinishedRun.operationId}`]);
    // A running operation offers neither result nor report.
    state.operation = { ...status, operationId: "f".repeat(32) }; await page.locator("[data-system2-refresh]").click();
    await expect(page.locator("[data-system2-result-text]")).toHaveCount(0);
    await expect(page.locator("[data-system2-report-save]")).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(commands).toEqual([]); expect(unexpected).toEqual([]);
  });
}

// Final-step failure diagnostic: a v4 body's typed final-step cause reaches the mounted panel through the real decoder.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} final-step cause of an unverified System-2 download at ${width}px`, async ({ page }) => {
    const m = language === "en" ? en : de;
    const { state, commands, unexpected } = await interceptSystem2(page, language);
    state.raw = JSON.stringify(finalStepStatus);
    await page.setViewportSize({ width, height: 1000 });
    await openProject(page, language); await dismissToasts(page, language);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
    await page.getByRole("combobox").press("Enter");
    await page.getByRole("button", { name: m["deviceDownload.title"], exact: true }).click();
    await page.locator("[data-download-mode]").selectOption("system2");
    await expect(page.locator("[data-system2-result-text]")).toContainText(m["system2.result.transferCompleteVerificationUnfinished"]);
    const section = page.getByRole("region", { name: m["system2.finalStep.heading"] });
    await expect(section).toBeVisible();
    const download = section.locator("[data-system2-final-step='download']");
    await expect(download).toHaveText(m["system2.finalStep.download"].replace("{category}", m["system2.finalStep.category.verificationMismatch"]).replace("{stage}", m["system2.finalStep.stage.allocation"]));
    await expect(section.locator("[data-system2-final-step='restore']")).toHaveCount(0);
    // The recovery's own cause appears beside the frozen download cause, never in its place.
    state.raw = JSON.stringify({ ...finalStepStatus, state: "restoreFailed", restoreAttempted: true, finalStepFailures: [finalDownloadMismatch, finalRestoreRead] });
    await page.locator("[data-system2-refresh]").click();
    await expect(section.locator("[data-system2-final-step='restore']")).toHaveText(m["system2.finalStep.restore"].replace("{category}", m["system2.finalStep.category.readFailed"]).replace("{stage}", m["system2.finalStep.stage.loadedState"]));
    await expect(download).toContainText(m["system2.finalStep.category.verificationMismatch"]);
    // An older journal without a typed cause says so and guesses nothing.
    state.raw = JSON.stringify({ ...finalStepStatus, finalStepFailures: [] });
    await page.locator("[data-system2-refresh]").click();
    await expect(section.locator("[data-system2-final-step-unavailable='download']")).toHaveText(m["system2.finalStep.unavailable"]);
    await expect(section.locator("[data-system2-final-step]")).toHaveCount(0);
    if (process.env.SYSTEM2_UI_CAPTURE) await section.screenshot({ path: `${process.env.SYSTEM2_UI_CAPTURE}/final-step-${language}-${width}.png` });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(commands).toEqual([]); expect(unexpected).toEqual([]);
  });
}
