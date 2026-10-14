/** Installation labels, explanation and scoped name editing in the full workbench, offline. */
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { join } from "node:path";

async function serve(page: Page, language: "en" | "de", theme: string) {
  const original: ProjectTree = {
    schema_version: 12, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "installation-node-fixture", project_incarnation: 1, snapshot_revision: 1,
    installations: [
      { id: 0, name: "Main", topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [] },
      { id: 2, name: "", topology: [{ id: 20, name: "Annex area", address: 2, lines: [] }],
        buildings: [], unassigned: [], group_addresses: [], group_ranges: [] },
    ],
  };
  let current = structuredClone(original);
  const writes: Array<{ method: string; path: string; body: unknown }> = [];
  const unexpected: string[] = [];
  // Like the real server: no project until the test creates one (ADR-0127).
  let projectOpen = false;
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    let body: unknown;
    let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme, motionLevel: "off", achievementsEnabled: "false", autosaveEnabled: "false",
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.8" },
    } };
    else if (path === "/api/settings" && method === "PUT") body = { schemaVersion: 1, status: "ok", settings: request.postDataJSON().settings };
    else if (path === "/api/project" && method === "GET") { if (projectOpen) body = current; else { status = 400; body = { error: "no project open" }; } }
    else if (path === "/api/project/new" && method === "POST") { projectOpen = true; body = current; }
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) body = [];
    else if (path === "/api/installations/2" && method === "PATCH") {
      // Goal A2 binding, checked like the server: stale context or name is a 409.
      const sent = request.postDataJSON() as { name: string; expectedName: string; serverIncarnation: string;
        projectIncarnation: number; snapshotRevision: number };
      writes.push({ method, path, body: sent });
      const shown = current.installations.find((installation) => installation.id === 2)!;
      if (sent.serverIncarnation !== current.server_incarnation || sent.projectIncarnation !== current.project_incarnation
        || sent.snapshotRevision !== current.snapshot_revision || sent.expectedName !== shown.name) {
        return route.fulfill({ status: 409, contentType: "application/json", body: JSON.stringify({ error: "stale", kind: "renameConflict" }) });
      }
      current = { ...current, snapshot_revision: current.snapshot_revision! + 1, can_undo: true, can_redo: false, is_modified: true,
        installations: current.installations.map((installation) => installation.id === 2 ? { ...installation, name: sent.name.trim() } : installation) };
      body = current;
    } else if ((path === "/api/undo" || path === "/api/redo") && method === "POST") {
      writes.push({ method, path, body: null });
      const undo = path === "/api/undo";
      current = { ...current, snapshot_revision: current.snapshot_revision! + 1, can_undo: !undo, can_redo: undo, is_modified: !undo,
        installations: current.installations.map((installation) => installation.id === 2 ? { ...installation, name: undo ? "" : "Annex 🛠" } : installation) };
      body = current;
    } else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else {
      unexpected.push(`${method} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"offline fixture only"}' });
    }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return { writes, unexpected, original };
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400])
  for (const theme of width === 1440 ? [language === "de" ? "porcelain" : "graphite", "user-modern-retro-green-crt"] : [language === "de" ? "porcelain" : "graphite"]) {
  test(`installation node explains and edits the correct installation (${language}, ${width}px, ${theme})`, async ({ page }) => {
    const m = language === "de" ? de : en;

    const { writes, unexpected, original } = await serve(page, language, theme);
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.setViewportSize({ width, height: 1000 });
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).click();
    await page.getByRole("dialog").getByRole("button", { name: m["newProject.create"], exact: true }).click();
    await page.getByRole("dialog").getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
    const fallback = m["installation.unnamed"].replace("{id}", "2");
    const explorer = page.locator(".project-explorer");
    if (!await explorer.isVisible()) await page.locator(".workbench-panel-controls").getByRole("button", { name: m["workbench.navigation"], exact: true }).click();
    const label = explorer.locator("button.tree-label", { hasText: new RegExp(`^${fallback.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`) });
    await expect(label).toHaveAttribute("title", `${m["installation.description"]} ${m["installation.selectHint"]}`);
    await label.focus();
    await page.keyboard.press("Enter");
    const properties = page.locator(".workbench-pane-right .inspector");
    await expect(properties.getByRole("heading", { name: fallback, exact: true })).toBeVisible();
    await expect(properties.getByText(m["installation.description"], { exact: true })).toBeVisible();
    const field = properties.getByRole("textbox", { name: m["inspector.name"], exact: true });
    await expect(field).toHaveValue("");
    expect(writes).toEqual([]);
    if (process.env.KNX_INSTALLATION_CAPTURE) await properties.screenshot({ path: join(process.env.KNX_INSTALLATION_CAPTURE, `installation-${language}-${width}-${theme}.png`) });
    const box = await properties.boundingBox();
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    await field.fill("discarded draft"); await field.press("Escape"); await field.press("Tab");
    await expect(field).toHaveValue(""); expect(writes).toEqual([]);
    await field.fill("Annex 🛠"); await field.press("Enter");
    await expect(properties.getByRole("heading", { name: "Annex 🛠", exact: true })).toBeVisible();
    expect(writes).toEqual([{ method: "PATCH", path: "/api/installations/2", body: { name: "Annex 🛠", expectedName: "",
      serverIncarnation: "installation-node-fixture", projectIncarnation: 1, snapshotRevision: 1 } }]);
    // Use the application Undo/Redo shortcuts outside the name input, not the browser's text undo.
    await properties.getByRole("heading", { name: "Annex 🛠", exact: true }).click();
    await page.keyboard.press("Control+z");
    await expect(properties.getByRole("heading", { name: fallback, exact: true })).toBeVisible();
    await expect(field).toHaveValue("");
    await page.keyboard.press("Control+Shift+z");
    await expect(properties.getByRole("heading", { name: "Annex 🛠", exact: true })).toBeVisible();
    expect(original.installations.map((installation) => installation.name)).toEqual(["Main", ""]);
    expect(writes.map((write) => write.path)).toEqual(["/api/installations/2", "/api/undo", "/api/redo"]);
    expect(unexpected).toEqual([]); expect(errors).toEqual([]);
  });
}
