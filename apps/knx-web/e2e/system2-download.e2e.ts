/** Intercepted real-app wiring evidence, not backend or hardware acceptance. */
import { expect, test } from "@playwright/test";
import { openNavigationSections } from "./disclosure-fixture";
import { preview, status, recoveryStatus } from "../src/system2DownloadTestFixtures";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { System2Status } from "../src/system2DownloadApi";
import { messages as en } from "../src/messages/en";
const tree: ProjectTree = { schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "server", project_incarnation: 1, snapshot_revision: 1,
  installations: [{ id: 1, name: "Installation", buildings: [], unassigned: [], group_addresses: [], group_ranges: [], topology: [{ id: 2, name: "Area", address: 1, lines: [{ id: 3, name: "Line", address: 1, devices: [{ id: 10, name: "Configured button", address: "1.1.67", description: null, com_object_count: 0 }] }] }] }] };

test("Download mode → configured preview → ordinary consent → status/cancel → explicit original recovery", async ({ page }) => {
  const unexpected: string[] = []; const commands: { path: string; body: unknown }[] = [];
  let projectOpen = false; let operation: System2Status | null = null;
  await page.route("**/api/**", (route) => {
    const request = route.request(); const path = new URL(request.url()).pathname; const method = request.method();
    const json = (body: unknown, code = 200) => route.fulfill({ status: code, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status") return json({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return json({ schemaVersion: 1, status: "ok", settings: {} });
    if (path === "/api/project" && method === "GET") return projectOpen ? json(tree) : json({ error: "no project" }, 400);
    if (path === "/api/project/new" && method === "POST") { projectOpen = true; return json(tree); }
    if (path === "/api/bus/discover") return json({ interfaces: [] });
    if (path === "/api/bus/activity") return json({ serverIncarnation: "server", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] });
    if (path === "/api/device-readiness") return json({ devices: [], counts: {} });
    if (path === "/api/device-download/status") return json({ error: "no ordinary download" }, 404);
    if (path === "/api/product-languages" && method === "GET") return json([]);
    if (method === "GET" && ["/api/achievements", "/api/project/status-metrics", "/api/bus/monitor/telegrams"].includes(path)) return json({ error: "Not available in this isolated fixture" }, 404);
    if (path === "/api/version") return json({ version: "0.1.0-alpha.16" });
    if (path === "/api/device-system2/status" && method === "GET") return json(operation);
    if (path.startsWith("/api/device-system2/") && method === "POST") {
      commands.push({ path, body: request.postDataJSON() });
      if (path.endsWith("/prepare")) return json(preview);
      if (path.endsWith("/start")) { operation = { ...status, backupAvailable: true }; return json(operation); }
      if (path.endsWith("/cancel")) { operation = { ...recoveryStatus, state: "cancelled" }; return json(operation); }
      if (path.endsWith("/recover")) { operation = { ...status, state: "restored", completedSteps: 2, totalSteps: 2, backupAvailable: true }; return json(operation); }
    }
    if (method === "GET" && ["/api/recent-projects", "/api/catalog", "/api/manufacturers", "/api/product-databases", "/api/notes", "/api/library", "/api/project/sources"].includes(path)) return json([]);
    unexpected.push(`${method} ${path}`); return json({ error: "Unexpected fixture request" }, 404);
  });
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
