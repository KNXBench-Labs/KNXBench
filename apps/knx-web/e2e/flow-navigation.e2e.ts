/** Verifies real-editor Flow links, including external and in-flight project replacement. */
import { expect, test, type BrowserContext, type Page } from "@playwright/test";
import { snapshot, telegram, node } from "./telegram-flow-server";
import { achievementsFixtureAnswer } from "./achievements-fixture";

function project(revision = 1) {
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    server_incarnation: "fixture", snapshot_revision: revision, group_address_style: "ThreeLevel",
    installations: [{ id: 1, name: "House", topology: [], buildings: [], group_ranges: [],
      unassigned: ["Switch", "Dimmer", "Blind"].map((name, i) => ({ id: i + 1, name, address: `1.1.${i + 1}`, description: null, com_object_count: 0 })),
      group_addresses: [{ id: 10, name: "Light", address: "1/0/1", range: null, dpts: ["DPST-1-1"], links: [] }] }] };
}
async function editor(page: Page, context: BrowserContext) {
  const state = { tree: project(), requests: [] as string[], unexpected: [] as string[], holdProject: false, releases: [] as (() => void)[] };
  await context.route("**/api/**", async route => {
    const r = route.request(); const path = new URL(r.url()).pathname; const method = r.method();
    state.requests.push(`${method} ${path}`);
    const json = (body: unknown, status = 200) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status" && method === "GET") return json({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return json({ schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: { uiLanguage: "en", onboardingGuide: { seenStage: "alpha" }, accent: "violet", density: "compact", motionLevel: "off" } });
    if (path === "/api/version" && method === "GET") return json({ version: "0.1.0-alpha.2+gfixture" });
    if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return json([]);
    if (path === "/api/project/new" && method === "POST") return json(state.tree);
    if (path === "/api/project" && method === "GET") {
      const tree = structuredClone(state.tree);
      if (state.holdProject) await new Promise<void>(resolve => state.releases.push(resolve));
      return json(tree);
    }
    if (path === "/api/bus/discover" && method === "POST") return json({ interfaces: [] });
    if (path === "/api/bus/monitor/telegrams" && method === "GET") {
      const since = Number(new URL(r.url()).searchParams.get("since") ?? 0);
      return json({ sessionId: 1, serverIncarnation: "fixture", contextStatus: "current", projectOpen: true,
        status: "active", nextSince: 2, droppedBefore: 0, telegrams: since <= 1 ? [telegram(1, "1", "On")] : [], flowGeneration: "1" });
    }
    if (path === "/api/bus/monitor/flow-snapshot" && method === "GET") return json(snapshot(1, "1", [2]));
    if (path === "/api/device/2" && method === "GET") return json({ ...state.tree.installations[0].unassigned[1], com_objects: [], product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" } });
    if (path === "/api/device/2/parameters" && method === "GET") return json({ programId: null, sections: [], stale: [], diagnostics: [] });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return json(achievements);
    state.unexpected.push(`${method} ${path}`); return json({ error: "offline fixture only" }, 404);
  });
  await page.setViewportSize({ width: 1500, height: 1000 });
  await page.goto("/");
  await page.getByRole("button", { name: "New project…" }).first().click();
  await page.getByRole("dialog", { name: "New project", exact: true }).getByRole("button", { name: "Create project" }).click();
  await page.getByRole("navigation", { name: "Bus monitor", exact: true }).getByRole("button", { name: "Bus monitor", exact: true }).first().click();
  await page.getByRole("tab", { name: "Flow", exact: true }).click();
  await expect(node(page, "Dimmer")).toBeVisible();
  return state;
}

test("device and GA links select the exact entity in the main editor and retain the source Flow camera", async ({ page, context }) => {
  const state = await editor(page, context);
  await page.getByRole("button", { name: "Zoom in", exact: true }).click();
  const camera = await page.locator("g[data-zoom]").getAttribute("transform");
  await node(page, "Dimmer").click();
  const link = page.getByRole("button", { name: "Open device in project", exact: true });
  await expect(link).toBeEnabled();
  const milestone = state.requests.length;
  await link.click();
  await expect(page.locator(".device-workspace h2")).toHaveText("Dimmer");
  expect(state.requests.slice(milestone)).toContain("GET /api/device/2");
  expect(state.requests.slice(milestone).every(r => r.startsWith("GET "))).toBe(true);
  await page.getByRole("navigation", { name: "Bus monitor", exact: true }).getByRole("button", { name: "Bus monitor", exact: true }).first().click();
  await expect(page.locator("g[data-zoom]")).toHaveAttribute("transform", camera!);
  await expect(page.locator(".flow-inspector h3")).toHaveText("Dimmer");
  await page.locator(".flow-values").getByRole("button", { name: "1/0/1", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Light", exact: true })).toBeVisible();
  expect(state.unexpected).toEqual([]);
});

test("a Flow-window link changes selection only in the main editor", async ({ page, context }) => {
  // LAN HTTP is not a secure context: randomUUID may be absent even though
  // getRandomValues and BroadcastChannel work. The real editor must still boot.
  await page.addInitScript(() => Object.defineProperty(window.crypto, "randomUUID", { value: undefined, configurable: true }));
  const state = await editor(page, context);
  const pending = page.waitForEvent("popup");
  await page.getByRole("button", { name: "Open in new window", exact: true }).click();
  const popup = await pending;
  await expect(node(popup, "Dimmer")).toBeVisible();
  await node(popup, "Dimmer").click();
  await popup.getByRole("button", { name: "Open device in project", exact: true }).click();
  await expect(page.locator(".device-workspace h2")).toHaveText("Dimmer");
  await expect(popup.locator(".device-workspace, .project-explorer")).toHaveCount(0);
  await expect(node(popup, "Dimmer")).toBeVisible();
  await popup.locator(".flow-values").getByRole("button", { name: "1/0/1", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Light", exact: true })).toBeVisible();
  expect(state.unexpected).toEqual([]); await popup.close();
});

test("external replacement with reused ids is refused at the server-revision check", async ({ page, context }) => {
  const state = await editor(page, context);
  await node(page, "Dimmer").click(); state.tree = project(2);
  await page.getByRole("button", { name: "Open device in project", exact: true }).click();
  await expect(page.locator(".flow-inspector [role=status]")).toContainText("Not available");
  await expect(page.locator(".device-workspace")).toHaveCount(0);
  expect(state.requests).not.toContain("GET /api/device/2");
  expect(state.unexpected).toEqual([]);
});

test("a delayed navigation reply cannot select an entity after the editor replaces its project", async ({ page, context }) => {
  const state = await editor(page, context); state.holdProject = true;
  await node(page, "Dimmer").click();
  await page.getByRole("button", { name: "Open device in project", exact: true }).click();
  await expect.poll(() => state.releases.length).toBe(1);
  await page.locator(".file-menu summary").click();
  await page.getByRole("button", { name: "New project…", exact: true }).click();
  state.tree = project(2);
  await page.getByRole("dialog", { name: "New project", exact: true }).getByRole("button", { name: "Create project" }).click();
  state.releases[0]();
  await expect(page.locator(".flow-inspector [role=status]")).toContainText("Not available");
  await expect(page.locator(".device-workspace")).toHaveCount(0);
  expect(state.requests).not.toContain("GET /api/device/2");
  expect(state.unexpected).toEqual([]);
});
