/** Verifies the add-device wizard in Chromium (en/de, 1440/400 px) with intercepted API. */

import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { achievementsFixtureAnswer } from "./achievements-fixture";

const item = {
  id: "M-0001_H-1_P-1_CI-1", manufacturerId: "M-0001", name: "Switch actuator", number: "SA 4",
  visibleDescription: null, productRefId: "P-1", hardware2programRefId: "HP-1",
  nameLanguage: null, visibleDescriptionLanguage: null, sourceLanguage: null,
};

interface CreateBody {
  lineId?: number; catalogItemId: string; name: string; quantity?: number; allocateAddresses?: boolean;
  installationId?: number; buildingPartId?: number; requestId?: string;
  expected?: { name: string; address?: string | null }[];
}

const labels = {
  en: {
    newProject: "New project…", newDialog: "New project", createProject: "Create project", createdDialog: "Project created",
    add: "+ Add device", dialog: "Add device", next: "Next", line: "Line", part: "Building part",
    quantity: "Quantity", allocate: "Assign free addresses on the line", create: "Create 2 devices",
    created: "2 devices created.", done: "Done", stale: "The project changed since this preview",
  },
  de: {
    newProject: "Neues Projekt…", newDialog: "Neues Projekt", createProject: "Projekt anlegen", createdDialog: "Projekt angelegt",
    add: "+ Gerät hinzufügen", dialog: "Gerät hinzufügen", next: "Weiter", line: "Linie", part: "Gebäudeteil",
    quantity: "Anzahl", allocate: "Freie Adressen der Linie vergeben", create: "2 Geräte anlegen",
    created: "2 Geräte angelegt.", done: "Fertig", stale: "Das Projekt hat sich seit dieser Vorschau geändert",
  },
} as const;

/**
 * A local stand-in for the server: one line 1.1 with `taken` addresses in
 * use and a kitchen. The preview names `Switch actuator N` and hands out the
 * next free addresses; a create whose `expected` disagrees is a `409`, as the
 * real route answers.
 */
async function interceptApi(page: Page, settings: Record<string, unknown> = {}) {
  const state = {
    unexpected: [] as string[],
    previews: [] as CreateBody[],
    creates: [] as CreateBody[],
    taken: [] as string[],
    current: {
      schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: true,
      group_address_style: "ThreeLevel", server_incarnation: "device-wizard-fixture", snapshot_revision: 1,
      installations: [{
        id: 0, name: "Installation 1", unassigned: [], group_addresses: [], group_ranges: [],
        topology: [{ id: 1, name: "Area 1", address: 1, lines: [{ id: 2, name: "Line 1.1", address: 1, devices: [] }] }],
        buildings: [{ id: 3, name: "House", kind: "Building", devices: [], children: [
          { id: 4, name: "Kitchen", kind: "Room", children: [], devices: [] },
        ] }],
      }],
    } as ProjectTree,
  };
  const plan = (body: CreateBody) => {
    const quantity = body.quantity ?? 1;
    let next = 1;
    return Array.from({ length: quantity }, (_, i) => {
      let address: string | null = null;
      if (body.allocateAddresses) {
        while (state.taken.includes(`1.1.${next}`)) next += 1;
        address = `1.1.${next++}`;
      }
      return { index: i + 1, name: quantity === 1 ? body.name : `${body.name} ${i + 1}`, address };
    });
  };
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    let body: unknown;
    let status = 200;
    if (path === "/api/auth/status" && method === "GET") {
      body = { required: false, authenticated: true };
    } else if (path === "/api/settings" && method === "GET") {
      body = { schemaVersion: 1, status: "ok", settings };
    } else if (path === "/api/project" && method === "GET") {
      body = state.current;
    } else if (path === "/api/project/new" && method === "POST") {
      // The fixture's project stands in for what the seed would build.
      body = state.current;
    } else if (path === "/api/bus/discover" && method === "POST") {
      body = { interfaces: [] };
    } else if (path === "/api/catalog/items" && method === "GET") {
      body = [item];
    } else if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers"].includes(path)) {
      body = [];
    } else if (path === "/api/devices/preview" && method === "POST") {
      const request = route.request().postDataJSON() as CreateBody;
      state.previews.push(request);
      body = { items: plan(request), diagnostics: [], installationId: 0 };
    } else if (path === "/api/devices" && method === "POST") {
      const request = route.request().postDataJSON() as CreateBody;
      state.creates.push(request);
      const items = plan(request);
      const same = JSON.stringify(items.map((i) => ({ name: i.name, address: i.address })))
        === JSON.stringify(request.expected);
      if (!same) {
        status = 409;
        body = { error: "the project changed since the preview", kind: "catalogPreviewStale" };
      } else {
        const devices = items.map((i) => ({
          id: 100 + i.index, name: i.name, address: i.address, description: null, com_object_count: 0,
        }));
        const installation = state.current.installations[0];
        installation.topology[0].lines[0].devices.push(...(devices as never[]));
        installation.buildings[0].children[0].devices.push(...(devices as never[]));
        state.current = { ...state.current, can_undo: true, snapshot_revision: (state.current.snapshot_revision ?? 0) + 1 };
        body = {
          tree: state.current, diagnostics: [], replayed: false,
          items: items.map((i) => ({ ...i, deviceId: 100 + i.index, diagnostics: [] })),
        };
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

/** Starts the app and opens the fixture project the way a user would: New project, Create, Done. */
async function openProject(page: Page, language: "en" | "de") {
  const l = labels[language];
  await page.goto("/");
  await page.getByRole("button", { name: l.newProject }).first().click();
  await page.getByRole("dialog", { name: l.newDialog, exact: true }).getByRole("button", { name: l.createProject }).click();
  await page.getByRole("dialog", { name: l.createdDialog }).getByRole("button", { name: l.done, exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
}

/** The innermost explorer row group of the room, then its own Add device row. */
async function openFromKitchen(page: Page, language: "en" | "de") {
  await openProject(page, language);
  const room = page.locator("li").filter({ has: page.locator("button.tree-label", { hasText: /^Kitchen \(/ }) }).last();
  await room.getByRole("button", { name: labels[language].add, exact: true }).click();
  return page.getByRole("dialog", { name: labels[language].dialog });
}

for (const language of ["en", "de"] as const) {
  const l = labels[language];
  test(`${language}: adds two allocated devices to the kitchen exactly as previewed, in one request`, async ({ page }) => {
    const state = await interceptApi(page, { uiLanguage: language });
    const dialog = await openFromKitchen(page, language);
    await expect(dialog).toBeVisible();

    await dialog.getByRole("button", { name: /Switch actuator/ }).click();
    await dialog.getByRole("button", { name: l.next, exact: true }).click();
    // Opened from the room: the room is already chosen, the line is not.
    await expect(dialog.getByLabel(l.part)).toHaveValue("4");
    await expect(dialog.getByLabel(l.line)).toHaveValue("");
    await dialog.getByLabel(l.line).selectOption("2");
    await dialog.getByRole("button", { name: l.next, exact: true }).click();
    await dialog.getByLabel(l.quantity, { exact: true }).fill("2");
    await dialog.getByRole("checkbox", { name: l.allocate }).check();
    await dialog.getByRole("button", { name: l.next, exact: true }).click();

    await expect(dialog.getByText("Switch actuator 2")).toBeVisible();
    await expect(dialog.getByText("1.1.2")).toBeVisible();
    expect(state.previews).toEqual([{
      lineId: 2, catalogItemId: item.id, name: "Switch actuator", quantity: 2,
      allocateAddresses: true, installationId: 0, buildingPartId: 4,
    }]);
    await dialog.getByRole("button", { name: l.create }).click();
    await expect(dialog.getByText(l.created)).toBeVisible();
    expect(state.creates).toHaveLength(1);
    expect(state.creates[0]).toMatchObject({
      lineId: 2, buildingPartId: 4, installationId: 0, quantity: 2, allocateAddresses: true,
      expected: [{ name: "Switch actuator 1", address: "1.1.1" }, { name: "Switch actuator 2", address: "1.1.2" }],
    });
    await dialog.getByRole("button", { name: l.done, exact: true }).click();
    await expect(dialog).toBeHidden();
    // Both new devices show under the room in the explorer.
    await expect(page.locator("button.tree-label", { hasText: "Switch actuator 2" })).toHaveCount(2);
    expect(state.unexpected).toEqual([]);
  });

  test(`${language}: a project that changed after the preview is refused and previewed again`, async ({ page }) => {
    const state = await interceptApi(page, { uiLanguage: language });
    const dialog = await openFromKitchen(page, language);
    await dialog.getByRole("button", { name: /Switch actuator/ }).click();
    await dialog.getByRole("button", { name: l.next, exact: true }).click();
    await dialog.getByLabel(l.line).selectOption("2");
    await dialog.getByRole("button", { name: l.next, exact: true }).click();
    await dialog.getByLabel(l.quantity, { exact: true }).fill("2");
    await dialog.getByRole("checkbox", { name: l.allocate }).check();
    await dialog.getByRole("button", { name: l.next, exact: true }).click();
    await expect(dialog.getByText("1.1.2")).toBeVisible();

    // Someone else takes 1.1.1 meanwhile.
    state.taken.push("1.1.1");
    await dialog.getByRole("button", { name: l.create }).click();
    await expect(dialog.getByText(l.stale)).toBeVisible();
    await expect(dialog.getByText("1.1.3")).toBeVisible();
    expect(state.creates).toHaveLength(1);
    expect(state.previews).toHaveLength(2);
    await dialog.getByRole("button", { name: l.create }).click();
    await expect(dialog.getByText(l.created)).toBeVisible();
    expect(state.creates[1].expected).toEqual([
      { name: "Switch actuator 1", address: "1.1.2" }, { name: "Switch actuator 2", address: "1.1.3" },
    ]);
    expect(state.unexpected).toEqual([]);
  });

  test(`${language}: fits a 400 px viewport on every step`, async ({ page }) => {
    await page.setViewportSize({ width: 400, height: 760 });
    const state = await interceptApi(page, { uiLanguage: language });
    await openProject(page, language);
    // On a stacked viewport the palette is the reliable entry point.
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("option", { name: new RegExp(`^${l.dialog}…`) }).click();
    const dialog = page.getByRole("dialog", { name: l.dialog });
    const fits = async () => {
      const box = (await dialog.boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(400);
      expect(await dialog.evaluate((el) => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1);
    };
    await fits();
    await dialog.getByRole("button", { name: /Switch actuator/ }).click();
    for (let step = 0; step < 3; step += 1) {
      await dialog.getByRole("button", { name: l.next, exact: true }).click();
      await fits();
    }
    await expect(dialog.getByText("Switch actuator", { exact: true }).first()).toBeVisible();
    expect(state.unexpected).toEqual([]);
  });
}
