/** Legacy .vd4 install in Chromium (en/de, 1440/400 px): password dialog and remember. */

import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { achievementsFixtureAnswer } from "./achievements-fixture";

const labels = {
  en: {
    newProject: "New project…", newDialog: "New project", createProject: "Create project", createdDialog: "Project created",
    done: "Done", wizard: "Add device", catalog: "Product catalog", catalogTitle: "Device catalog", password: "Password for marvin.vd4", field: "Password",
    remember: "Remember this password", import: "Import", encrypted: "is encrypted",
    imported: "Legacy product database imported: 1 application program.", nowRemembered: "It is now remembered.",
  },
  de: {
    newProject: "Neues Projekt…", newDialog: "Neues Projekt", createProject: "Projekt anlegen", createdDialog: "Projekt angelegt",
    done: "Fertig", wizard: "Gerät hinzufügen", catalog: "Produktkatalog", catalogTitle: "Gerätekatalog", password: "Passwort für marvin.vd4", field: "Passwort",
    remember: "Dieses Passwort merken", import: "Importieren", encrypted: "ist verschlüsselt",
    imported: "Alte Produktdatenbank importiert: 1 Applikationsprogramm.", nowRemembered: "Es ist jetzt gemerkt.",
  },
} as const;

const report = {
  payloadSha256: "a".repeat(64), originalSha256: "b".repeat(64), namespace: "LX1A2B3C4D", skipped: false,
  programs: ["M-1092_A-LX1A2B3C4D-300"], catalogItems: 1, parameters: 10, parameterRefs: 12, comObjectRefs: 3,
  translations: 8, diagnostics: [{ kind: "secret-withheld", detail: "1 value of device.DEVICE_BCU_PASSWORD withheld" }],
  password: "given", remembered: true, rememberProblem: null,
};

/** A stand-in server: an encrypted .vd4 asks for a password, then imports with it. */
async function interceptApi(page: Page, settings: Record<string, unknown>) {
  const state = {
    unexpected: [] as string[], packageInstalls: 0, installs: [] as { password: boolean; remember: boolean }[],
  };
  const current = {
    schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: true,
    group_address_style: "ThreeLevel", server_incarnation: "legacy-install-fixture", snapshot_revision: 1,
    installations: [{
      id: 0, name: "Installation 1", unassigned: [], group_addresses: [], group_ranges: [],
      topology: [{ id: 1, name: "Area 1", address: 1, lines: [{ id: 2, name: "Line 1.1", address: 1, devices: [] }] }],
      buildings: [],
    }],
  } as ProjectTree;
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    let body: unknown;
    let status = 200;
    if (path === "/api/auth/status" && method === "GET") {
      body = { required: false, authenticated: true };
    } else if (path === "/api/settings" && method === "GET") {
      body = { schemaVersion: 1, status: "ok", settings };
    } else if ((path === "/api/project" && method === "GET") || (path === "/api/project/new" && method === "POST")) {
      body = current;
    } else if (path === "/api/bus/discover" && method === "POST") {
      body = { interfaces: [] };
    } else if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) {
      body = [];
    } else if (path === "/api/catalog/install" && method === "POST") {
      // The package installer recognises a renamed legacy database by content.
      state.packageInstalls += 1;
      status = 422;
      body = { error: "legacy ETS3 product database (EX-IM) is not a .knxprod package", kind: "legacyProductDatabase" };
    } else if (path === "/api/catalog/install-legacy" && method === "POST") {
      const form = route.request().postDataBuffer()?.toString("latin1") ?? "";
      const install = { password: form.includes('name="password"'), remember: form.includes('name="remember"') };
      state.installs.push(install);
      if (!install.password) {
        status = 422;
        body = { error: "a password is required", kind: "legacyPasswordRequired" };
      } else {
        body = report;
      }
    } else if (achievementsFixtureAnswer(method, path) !== undefined) {
      body = achievementsFixtureAnswer(method, path);
    } else {
      state.unexpected.push(`${method} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
    }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

async function openWizard(page: Page, language: "en" | "de") {
  const l = labels[language];
  await page.goto("/");
  await page.getByRole("button", { name: l.newProject }).first().click();
  await page.getByRole("dialog", { name: l.newDialog, exact: true }).getByRole("button", { name: l.createProject }).click();
  await page.getByRole("dialog", { name: l.createdDialog }).getByRole("button", { name: l.done, exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("option", { name: new RegExp(`^${l.wizard}…`) }).click();
  return page.getByRole("dialog", { name: l.wizard });
}

for (const language of ["en", "de"] as const) {
  const l = labels[language];
  for (const width of [1440, 400]) {
    test(`${language} ${width}px: an encrypted .vd4 asks for its password and imports with it`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      const state = await interceptApi(page, { uiLanguage: language });
      const wizard = await openWizard(page, language);
      await wizard.locator('input[type="file"]').setInputFiles({
        name: "marvin.vd4", mimeType: "application/octet-stream", buffer: Buffer.from("PK\u0003\u0004fixture"),
      });
      const dialog = page.getByRole("dialog", { name: l.password });
      await expect(dialog).toBeVisible();
      await expect(dialog).toContainText(l.encrypted);
      const box = (await dialog.boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(width);
      expect(await dialog.evaluate((el) => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1);
      await expect(dialog.getByLabel(l.field, { exact: true })).toBeFocused();
      await dialog.getByLabel(l.field, { exact: true }).fill("fixture-password");
      await dialog.getByLabel(l.remember).check();
      await dialog.getByRole("button", { name: l.import, exact: true }).click();
      await expect(dialog).toHaveCount(0);
      await expect(wizard).toContainText(l.imported);
      await expect(wizard).toContainText(l.nowRemembered);
      expect(state.installs).toEqual([{ password: false, remember: false }, { password: true, remember: true }]);
      expect(state.unexpected).toEqual([]);
    });
  }
}

test("en 1440px: the catalog sends a renamed legacy database on to the legacy installer", async ({ page }) => {
  const l = labels.en;
  await page.setViewportSize({ width: 1440, height: 800 });
  const state = await interceptApi(page, { uiLanguage: "en" });
  const wizard = await openWizard(page, "en");
  await page.keyboard.press("Escape");
  await expect(wizard).toHaveCount(0);
  await page.getByRole("button", { name: l.catalog, exact: true }).first().click();
  const catalog = page.getByRole("region", { name: l.catalogTitle });
  await catalog.locator('input[type="file"]').setInputFiles({
    name: "renamed.knxprod", mimeType: "application/zip", buffer: Buffer.from("PK\u0003\u0004fixture"),
  });
  const dialog = page.getByRole("dialog", { name: "Password for renamed.knxprod" });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel(l.field, { exact: true }).fill("fixture-password");
  await dialog.getByRole("button", { name: l.import, exact: true }).click();
  await expect(catalog).toContainText(l.imported);
  // Each attempt asks the package route first, which names the legacy database again.
  expect(state.packageInstalls).toBe(2);
  expect(state.installs).toEqual([{ password: false, remember: false }, { password: true, remember: false }]);
  expect(state.unexpected).toEqual([]);
});
