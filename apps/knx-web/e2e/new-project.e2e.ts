/** Verifies the new-project wizard in Chromium with every API request locally intercepted. */

import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { achievementsFixtureAnswer } from "./achievements-fixture";

const styles = ["ThreeLevel", "TwoLevel", "Free"] as const;

interface SeedPart { name: string; kind: string; children: SeedPart[] }
interface SeedRequest {
  name: string;
  installationName: string;
  groupAddressStyle: string;
  seed?: {
    areas: { name: string; address: number; lines: { name: string; address: number }[] }[];
    buildings: SeedPart[];
    groupRanges: { name: string; main: number; middles: { name: string; middle: number }[] }[];
  };
}

/** A local stand-in for the server that projects the seed the way the real tree does. */
async function interceptApi(page: Page, settings: Record<string, unknown> = {}) {
  const state = {
    unexpected: [] as string[],
    requests: [] as SeedRequest[],
    current: {
      schema_version: 11, errors: 0, warnings: 0, can_undo: false,
      can_redo: false, is_modified: false, group_address_style: "ThreeLevel",
      server_incarnation: "new-project-fixture", snapshot_revision: 0,
      installations: [],
    } as ProjectTree,
  };
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    let body: unknown;
    if (path === "/api/auth/status" && method === "GET") {
      body = { required: false, authenticated: true };
    } else if (path === "/api/settings" && method === "GET") {
      body = { schemaVersion: 1, status: "ok", settings };
    } else if (path === "/api/project" && method === "GET") {
      body = state.current;
    } else if (path === "/api/bus/discover" && method === "POST") {
      // The sidebar's automatic search is intercepted, never a real UDP scan.
      body = { interfaces: [] };
    } else if (path === "/api/project/new" && method === "POST") {
      const request = route.request().postDataJSON() as SeedRequest;
      state.requests.push(request);
      let id = 0;
      const next = () => ++id;
      const part = (p: SeedPart): unknown => ({ id: next(), name: p.name, kind: p.kind, children: p.children.map(part), devices: [] });
      const ranges = (request.seed?.groupRanges ?? []).flatMap((main) => {
        const mainId = next();
        return [
          { id: mainId, name: main.name, start: `${main.main}/0/0`, end: `${main.main}/7/255`, parent: null },
          ...main.middles.map((m) => ({
            id: next(), name: m.name, start: `${main.main}/${m.middle}/0`, end: `${main.main}/${m.middle}/255`, parent: mainId,
          })),
        ];
      });
      state.current = {
        ...state.current,
        group_address_style: request.groupAddressStyle as ProjectTree["group_address_style"],
        snapshot_revision: 1,
        installations: [{
          id: 0, name: request.installationName, unassigned: [], group_addresses: [],
          topology: (request.seed?.areas ?? []).map((a) => ({
            id: next(), name: a.name, address: a.address,
            lines: a.lines.map((l) => ({ id: next(), name: l.name, address: l.address, devices: [] })),
          })),
          buildings: (request.seed?.buildings ?? []).map(part),
          group_ranges: ranges,
        }],
      } as ProjectTree;
      body = state.current;
    } else if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) {
      body = [];
    } else if (achievementsFixtureAnswer(method, path) !== undefined) {
      body = achievementsFixtureAnswer(method, path);
    } else {
      state.unexpected.push(`${method} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
    }
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

for (const style of styles) {
  test(`creates a ${style} project with the default topology and opens the catalog from the last page`, async ({ page }) => {
    const projectName = `Browser ${style}`;
    const installationName = `Installation ${style}`;
    const state = await interceptApi(page);

    await page.goto("/");
    await page.getByRole("button", { name: "New project…" }).click();

    const dialog = page.getByRole("dialog", { name: "New project" });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText("A new project with one installation.")).toBeVisible();
    await expect(dialog.getByLabel("Project name")).toHaveValue("Untitled project");
    await expect(dialog.getByLabel("Installation name")).toHaveValue("Installation 1");
    await expect(dialog.getByLabel("Project language")).toHaveValue("en");

    await dialog.getByLabel("Project name").fill(projectName);
    await dialog.getByLabel("Installation name").fill(installationName);
    await dialog.getByLabel("Group address style").selectOption(style);

    const responsePromise = page.waitForResponse(
      (response) => response.url().endsWith("/api/project/new") && response.request().method() === "POST",
    );
    // The fast path: Enter in a details field creates the project.
    await dialog.getByLabel("Project name").press("Enter");
    const response = await responsePromise;
    expect(response.status()).toBe(200);
    expect(response.request().postDataJSON()).toEqual({
      name: projectName,
      installationName,
      language: "en",
      groupAddressStyle: style,
      discardChanges: false,
      seed: {
        areas: [{ name: "Area 1", address: 1, lines: [{ name: "Line 1.1", address: 1, mediumRef: "MT-0" }] }],
        buildings: [],
        groupRanges: [],
      },
    });

    const done = page.getByRole("dialog", { name: "Project created" });
    await expect(done).toContainText(`${projectName} is open. It is not saved yet`);
    await expect(done.getByRole("button", { name: "Done" })).toBeFocused();
    await expect(page.locator("button.tree-label").filter({ hasText: installationName })).toBeVisible();

    await done.getByRole("button", { name: "Add devices now" }).click();
    await expect(done).toBeHidden();
    await expect(page.getByRole("region", { name: "Device catalog" })).toBeVisible();

    await page.getByRole("button", { name: "Project", exact: true }).click();
    await expect(page.getByLabel("Group address style")).toHaveValue(style);
    expect(state.unexpected).toEqual([]);
  });
}

test("walks every step: building, preset group structure, review and create", async ({ page }) => {
  const state = await interceptApi(page);
  await page.goto("/");
  await page.getByRole("button", { name: "New project…" }).click();
  const dialog = page.getByRole("dialog", { name: "New project" });
  const steps = dialog.getByRole("list", { name: "Wizard steps" });

  await dialog.getByRole("button", { name: "Next" }).click();
  await expect(dialog.getByRole("heading", { name: "Step 2 of 5: Topology" })).toBeVisible();
  await dialog.getByRole("button", { name: "Add line" }).click();
  await expect(dialog.getByLabel("Line name")).toHaveCount(2);
  // Enter inside a structure editor never creates the project.
  await dialog.getByLabel("Line name").nth(1).press("Enter");
  expect(state.requests).toEqual([]);

  await dialog.getByRole("button", { name: "Next" }).click();
  await dialog.getByRole("button", { name: "Add building" }).click();
  await dialog.getByRole("button", { name: "Add floors" }).click();
  await expect(dialog.getByLabel("Floor name")).toHaveCount(3);
  await dialog.getByRole("button", { name: "Add room" }).first().click();

  await dialog.getByRole("button", { name: "Next" }).click();
  await dialog.getByRole("button", { name: "Apply preset (replaces the list)" }).click();
  await expect(dialog.getByLabel("Main group name")).toHaveCount(5);
  await expect(dialog.getByLabel("Middle group name")).toHaveCount(15);

  // A duplicate number is caught on its step and blocks Create until fixed.
  await dialog.getByLabel("Main group number").nth(1).fill("1");
  await expect(dialog.getByRole("button", { name: "Create project" })).toBeDisabled();
  await expect(steps.getByRole("button", { name: /Group structure/ })).toContainText("1 to fix");
  await steps.getByRole("button", { name: /Review/ }).click();
  await dialog.getByRole("button", { name: "Go to Group structure" }).click();
  await dialog.getByLabel("Main group number").nth(1).fill("2");

  await steps.getByRole("button", { name: /Review/ }).click();
  await expect(dialog.getByText("Ready to create.")).toBeVisible();
  await dialog.getByRole("button", { name: "Create project" }).click();

  await expect(page.getByRole("dialog", { name: "Project created" })).toBeVisible();
  const seed = state.requests[0].seed!;
  expect(seed.areas[0].lines.map((l) => l.name)).toEqual(["Line 1.1", "Line 1.2"]);
  expect(seed.buildings[0].children.map((c) => c.name)).toEqual(["Floor 1", "Floor 2", "Floor 3"]);
  expect(seed.buildings[0].children[0].children.map((c) => c.name)).toEqual(["Room 1"]);
  expect(seed.groupRanges.map((r) => [r.main, r.name, r.middles.length])).toEqual([
    [1, "Lighting", 3], [2, "Shading", 3], [3, "Heating", 3], [4, "Ventilation", 3], [5, "Central functions", 3],
  ]);
  await page.getByRole("button", { name: "Done" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(state.unexpected).toEqual([]);
});

test("asks before throwing away entries and keeps them on Escape", async ({ page }) => {
  const state = await interceptApi(page);
  await page.goto("/");
  await page.getByRole("button", { name: "New project…" }).click();
  const dialog = page.getByRole("dialog", { name: "New project" });
  await dialog.getByLabel("Project name").fill("Villa");
  await page.keyboard.press("Escape");
  await expect(dialog.getByText("Discard your entries?")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog.getByText("Discard your entries?")).toBeHidden();
  await expect(dialog.getByLabel("Project name")).toHaveValue("Villa");
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await dialog.getByRole("button", { name: "Discard", exact: true }).click();
  await expect(dialog).toBeHidden();
  expect(state.requests).toEqual([]);
  expect(state.unexpected).toEqual([]);
});

for (const [language, labels] of [
  ["en", { newProject: "New project…", dialog: "New project", next: "Next", building: "Add building", floors: "Add floors" }],
  ["de", { newProject: "Neues Projekt…", dialog: "Neues Projekt", next: "Weiter", building: "Gebäude hinzufügen", floors: "Etagen hinzufügen" }],
] as const) {
  test(`fits a 400 px viewport on every step (${language})`, async ({ page }) => {
    await page.setViewportSize({ width: 400, height: 760 });
    const state = await interceptApi(page, { uiLanguage: language });
    await page.goto("/");
    await page.getByRole("button", { name: labels.newProject }).first().click();
    const dialog = page.getByRole("dialog", { name: labels.dialog });
    const fits = async () => {
      const box = (await dialog.boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(400);
      expect(await dialog.evaluate((el) => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1);
    };
    await fits();
    await dialog.getByRole("button", { name: labels.next }).click();
    await fits();
    await dialog.getByRole("button", { name: labels.next }).click();
    await dialog.getByRole("button", { name: labels.building }).click();
    await dialog.getByRole("button", { name: labels.floors }).click();
    await fits();
    await dialog.getByRole("button", { name: labels.next }).click();
    await fits();
    await dialog.getByRole("button", { name: labels.next }).click();
    await fits();
    expect(state.unexpected).toEqual([]);
  });
}
