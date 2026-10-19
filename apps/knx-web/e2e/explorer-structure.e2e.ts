/** Owner feedback P2: the explorer shows what opens, counts what is folded, and draws its lines. */
import { openExplorerBranches, openNavigationSections } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

type Catalogue = { readonly [K in keyof typeof en]: string };
type Lines = "inherit" | "subtle" | "off";

const device = (id: number, name: string) => ({ id, name, address: `1.1.${id}`, description: null, com_object_count: 0 });
const ga = (id: number, address: string, name: string) => ({ id, name, address, range: null, dpts: [], links: [] });

/** `extra` adds that many unassigned devices, for a tree longer than the screen. */
function tree(extra = 0): ProjectTree {
  return {
    schema_version: 15, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "p2-fixture", project_incarnation: 1, snapshot_revision: 1,
    installations: [{
      id: 0, name: "Fictional house", buildings: [], group_ranges: [],
      topology: [{ id: 1, name: "Main", address: 1, lines: [{ id: 2, name: "Ground floor", address: 1,
        devices: [device(1, "Switch actuator"), device(2, "Dimmer"), device(3, "Blind actuator")] }] }],
      unassigned: [{ ...device(9, "Spare sensor"), address: null },
        ...Array.from({ length: extra }, (_, i) => ({ ...device(1000 + i, `Spare ${i}`), address: null }))],
      group_addresses: [ga(10, "1/0/1", "Hall light"), ga(11, "1/0/2", "Hall dimming")],
    }],
  };
}

async function serve(page: Page, language: "en" | "de", lines: Lines, extra = 0) {
  const unexpected: string[] = [];
  let projectOpen = false;
  await page.route("https://api.github.com/**", (route) => { unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme: "porcelain", motionLevel: "standard", motionLines: lines, releaseChecksEnabled: false,
      achievementsEnabled: "false", onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.11" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/project" && method === "GET") return projectOpen ? ok(tree(extra)) : fulfillNoProjectOpen(route);
    if (path === "/api/project/new" && method === "POST") { projectOpen = true; return ok(tree(extra)); }
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(tree(extra)));
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    unexpected.push(`${method} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"offline fixture only"}' });
  });
  return unexpected;
}

async function openExplorer(page: Page, m: Catalogue, width: number) {
  await page.setViewportSize({ width, height: 900 });
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
  await openNavigationSections(page);
  await openExplorerBranches(page);
  // The explorer lives in the navigation pane, which narrow layouts collapse.
  const navigation = page.getByRole("button", { name: m["workbench.navigation"], exact: true });
  if (await navigation.getAttribute("aria-expanded") === "false") await navigation.click();
  await expect(page.locator(".project-explorer")).toBeVisible();
}

const label = (page: Page, text: string) => page.locator(".project-explorer .tree-label", { hasText: new RegExp(`^${text}$`) });
const toggle = (page: Page, text: string) => page.locator(`.project-explorer .tree-toggle[aria-label="${text}"]`);

// The installed clock keeps flowing while the project opens, so where the
// twelve-second pulse falls in a fixed `runFor` window depends on how long
// opening took (on a loaded host it can start and end inside the window).
// Step the clock until a pulse has started instead of assuming its phase.
const PULSE_STEP_MS = 200;
async function runUntilPulse(page: Page) {
  const explorer = page.locator(".project-explorer");
  for (let elapsed = 0; elapsed <= 12_000 + PULSE_STEP_MS; elapsed += PULSE_STEP_MS) {
    if (await explorer.getAttribute("data-tree-pulse") === "") return;
    await page.clock.runFor(PULSE_STEP_MS);
  }
  throw new Error("no structure-line pulse within one interval");
}

/** Computed style of a nested row's stem (::before) and tick (::after). */
async function lineOf(page: Page, text: string) {
  return label(page, text).evaluate((el) => {
    const li = el.closest("li")!;
    const stem = getComputedStyle(li, "::before"), tick = getComputedStyle(li, "::after");
    return { stemWidth: stem.width, stemColor: stem.backgroundColor, stemAnimation: stem.animationName,
      tickHeight: tick.height, tickAnimation: tick.animationName };
  });
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: nodes read as expandable and say what they fold away`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const unexpected = await serve(page, language, "inherit");
    await openExplorer(page, m, width);
    const lineLabel = m["explorer.lineLabel"].replace("{address}", "1").replace("{name}", "Ground floor");
    const chevron = toggle(page, lineLabel);
    await expect(chevron.locator("svg.tree-chevron")).toBeVisible();
    const box = await chevron.boundingBox();
    expect(box!.width).toBeGreaterThanOrEqual(20);
    expect(box!.height).toBeGreaterThanOrEqual(20);
    expect(await chevron.locator("svg").evaluate((el) => getComputedStyle(el).transform)).not.toBe("none");

    await chevron.click();
    await expect(label(page, "1.1.1 Switch actuator")).toHaveCount(0);
    const count = chevron.locator("xpath=..").locator(".tree-count");
    await expect(count.locator("[aria-hidden='true']")).toHaveText("3");
    await expect(count.locator(".sr-only")).toHaveText(language === "de" ? "3 Einträge" : "3 entries");
    // The quarter turn back is a transition; wait for where it comes to rest.
    await expect(chevron.locator("svg")).toHaveCSS("transform", "none");
    if (process.env.KNX_P2_CAPTURE_DIR) await page.screenshot({ path: `${process.env.KNX_P2_CAPTURE_DIR}/collapsed-${language}-${width}.png` });

    await page.getByRole("button", { name: m["explorer.collapseAll"], exact: true }).click();
    await expect(label(page, "Spare sensor")).toHaveCount(0);
    await expect(toggle(page, m["explorer.topology"])).toHaveAttribute("aria-expanded", "false");
    await expect(toggle(page, "Fictional house")).toHaveAttribute("aria-expanded", "true");
    await page.getByRole("button", { name: m["explorer.expandAll"], exact: true }).click();
    await expect(label(page, "1.1.1 Switch actuator")).toBeVisible();
    await expect(label(page, "Spare sensor")).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
    expect(unexpected).toEqual([]);
  });
}

test("lines connect every child, grow below a node opened by hand and pulse about every twelve seconds", async ({ page }) => {
  await page.clock.install({ time: new Date("2026-10-14T12:00:00Z") });
  const unexpected = await serve(page, "en", "inherit");
  await openExplorer(page, en, 1440);
  await expect(page.locator("html")).toHaveAttribute("data-motion-lines", "standard");
  let line = await lineOf(page, "1.1.2 Dimmer");
  expect(line.stemWidth).toBe("1px");
  expect(line.stemColor).not.toBe("rgba(0, 0, 0, 0)");
  expect(line.tickHeight).toBe("1px");
  // Loading the project animates nothing.
  expect(line.stemAnimation).toBe("none");
  const lineLabel = "Line 1: Ground floor";
  await toggle(page, lineLabel).click();
  await toggle(page, lineLabel).click();
  line = await lineOf(page, "1.1.2 Dimmer");
  expect(line.stemAnimation).toBe("knx-tree-grow-down");
  expect(line.tickAnimation).toBe("knx-tree-grow-right");
  await page.clock.runFor(1_200);
  expect((await lineOf(page, "1.1.2 Dimmer")).stemAnimation).toBe("none");
  if (process.env.KNX_P2_CAPTURE_DIR) await page.locator(".project-explorer").screenshot({ path: `${process.env.KNX_P2_CAPTURE_DIR}/lines-standard.png` });
  const explorer = page.locator(".project-explorer");
  await expect(explorer).not.toHaveAttribute("data-tree-pulse");
  await runUntilPulse(page);
  // The explorer pane is short: the first device row is on screen, so it pulses.
  expect((await lineOf(page, "1.1.1 Switch actuator")).stemAnimation).toBe("knx-tree-pulse");
  await page.clock.runFor(2_400);
  await expect(explorer).not.toHaveAttribute("data-tree-pulse");
  expect((await lineOf(page, "1.1.1 Switch actuator")).stemAnimation).toBe("none");
  expect(unexpected).toEqual([]);
});

test("a pulse in a long tree touches only the rows on screen", async ({ page }) => {
  await page.clock.install({ time: new Date("2026-10-14T12:00:00Z") });
  await serve(page, "en", "inherit", 400);
  await openExplorer(page, en, 1440);
  // Expand-all renders 400+ rows; the pulse marks the rows on screen when it
  // starts, so the tree must be there before the clock reaches the pulse.
  await expect.poll(() => page.locator(".project-explorer .tree-root li li").count()).toBeGreaterThan(400);
  await runUntilPulse(page);
  const counts = await page.locator(".project-explorer").evaluate((el) => {
    const box = el.getBoundingClientRect();
    const marked = [...el.querySelectorAll<HTMLElement>("li[data-tree-pulse-row]")];
    return { rows: el.querySelectorAll(".tree-root li li").length, marked: marked.length,
      offscreen: marked.filter((li) => { const r = li.querySelector(".tree-row")!.getBoundingClientRect(); return r.bottom <= box.top || r.top >= box.bottom; }).length };
  });
  expect(counts.rows).toBeGreaterThan(400);
  expect(counts.marked).toBeGreaterThan(0);
  expect(counts.marked).toBeLessThan(80);
  expect(counts.offscreen).toBe(0);
  await page.clock.runFor(2_400);
  await expect(page.locator("li[data-tree-pulse-row]")).toHaveCount(0);
});

test("subtle structure-line motion grows the lines but never pulses", async ({ page }) => {
  await page.clock.install({ time: new Date("2026-10-14T12:00:00Z") });
  await serve(page, "en", "subtle");
  await openExplorer(page, en, 1440);
  await expect(page.locator("html")).toHaveAttribute("data-motion-lines", "subtle");
  await toggle(page, "Line 1: Ground floor").click();
  await toggle(page, "Line 1: Ground floor").click();
  expect((await lineOf(page, "1.1.2 Dimmer")).stemAnimation).toBe("knx-tree-grow-down");
  await page.clock.runFor(30_000);
  await expect(page.locator(".project-explorer")).not.toHaveAttribute("data-tree-pulse");
  await expect(page.locator("li[data-tree-pulse-row]")).toHaveCount(0);
});

test("off keeps the lines still, and so does the operating system's reduced motion", async ({ page }) => {
  await serve(page, "en", "off");
  await openExplorer(page, en, 1440);
  await expect(page.locator("html")).toHaveAttribute("data-motion-lines", "off");
  await toggle(page, "Line 1: Ground floor").click();
  await toggle(page, "Line 1: Ground floor").click();
  let line = await lineOf(page, "1.1.2 Dimmer");
  expect(line.stemWidth).toBe("1px");
  expect(line.stemAnimation).toBe("none");
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.evaluate(() => document.documentElement.setAttribute("data-motion-lines", "standard"));
  await toggle(page, "Line 1: Ground floor").click();
  await toggle(page, "Line 1: Ground floor").click();
  line = await lineOf(page, "1.1.2 Dimmer");
  expect(line.stemAnimation).toBe("none");
  expect(line.tickAnimation).toBe("none");
});
