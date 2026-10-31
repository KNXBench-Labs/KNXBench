/** UI-03: File submenus open right, flip, expand inline when narrow, keep focus, honour motion. */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { openFileSubmenu } from "./file-menu";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";
import { HISTORY_FIXTURE } from "../src/projectHistory.fixture";
import { admitProjectHistory } from "../src/projectHistory";

const THEMES = ["porcelain", "graphite", "cupertino", "lcars", "user-modern-retro-green-crt"] as const;
type Catalogue = { readonly [K in keyof typeof en]: string };
const TRIGGERS = ["exchange", "history", "help"] as const;

async function serve(page: Page, { theme = "porcelain", motionLevel = "standard", language = "en", sessionRequired = false, uiScale, density }:
  { theme?: string; motionLevel?: string; language?: "en" | "de"; sessionRequired?: boolean; uiScale?: number; density?: "compact" | "comfortable" } = {}) {
  const unexpected: string[] = [];
  await page.route("https://api.github.com/**", (route) => { unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    if (path === "/api/auth/status") return ok({ required: sessionRequired, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme, motionLevel, releaseChecksEnabled: false, achievementsEnabled: "false",
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.14" }, ...(uiScale === undefined ? {} : { uiScale }), ...(density === undefined ? {} : { density }) } });
    if (path === "/api/settings" && method === "PUT") return ok({ schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/project" && method === "GET") return route.fulfill({ status: 400, contentType: "application/json", body: '{"error":"no project open"}' });
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    unexpected.push(`${method} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"file-menu fixture only"}' });
  });
  return unexpected;
}

async function openFile(page: Page) {
  await page.locator(".file-menu > summary").click();
  await expect(page.locator(".file-menu-content")).toBeVisible();
}

const trigger = (page: Page, id: string) => page.locator(`[data-file-submenu-trigger="${id}"]`);
const panel = (page: Page, id: string) => page.locator(`[data-file-submenu-panel="${id}"]`);
const box = async (locator: Locator) => (await locator.boundingBox())!;

async function viewportFit(page: Page, locator: Locator) {
  const b = await box(locator);
  const { width, height } = page.viewportSize()!;
  expect(b.x, "left edge").toBeGreaterThanOrEqual(-0.5);
  expect(b.y, "top edge").toBeGreaterThanOrEqual(-0.5);
  expect(b.x + b.width, "right edge").toBeLessThanOrEqual(width + 0.5);
  expect(b.y + b.height, "bottom edge").toBeLessThanOrEqual(height + 0.5);
}

/** WCAG contrast of a text colour against an opaque background, both read from computed style. */
async function contrast(item: Locator, surface: Locator) {
  return item.evaluate((node, surfaceNode) => {
    const parse = (value: string) => value.match(/[\d.]+/g)!.map(Number);
    const channel = (c: number) => { const v = c / 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; };
    const lum = ([r, g, b]: number[]) => 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
    const ink = parse(getComputedStyle(node).color);
    const paper = parse(getComputedStyle(surfaceNode as Element).backgroundColor);
    const [hi, lo] = [lum(ink), lum(paper)].sort((a, b) => b - a);
    return { ratio: (hi + 0.05) / (lo + 0.05), alpha: paper[3] ?? 1 };
  }, await surface.elementHandle());
}

test.describe("File submenus", () => {
  test.beforeEach(async ({ page }) => { await page.setViewportSize({ width: 1440, height: 900 }); });

  for (const language of ["en", "de"] as const) {
    test(`${language}: three submenus open to the right, fit the viewport and keep sibling exclusivity`, async ({ page }) => {
      const m: Catalogue = language === "de" ? de : en;
      const unexpected = await serve(page, { language });
      await page.goto("/");
      await openFile(page);
      await expect(page.locator(".file-submenu-trigger > span:first-child")).toHaveText([m["fileMenu.exchange"], m["fileMenu.history"], m["fileMenu.help"]]);
      // Frequent project actions stay on the first level.
      for (const name of [m["toolbar.newProject"], m["toolbar.openProject"], m["toolbar.openNativeProject"], m["toolbar.saveAs"]]) {
        await expect(page.locator(".file-menu-content > button", { hasText: name })).toBeVisible();
      }
      for (const id of TRIGGERS) {
        await expect(trigger(page, id)).toHaveAttribute("aria-haspopup", "menu");
        await expect(trigger(page, id)).toHaveAttribute("aria-expanded", "false");
        await expect(panel(page, id)).toBeHidden();
      }
      for (const id of TRIGGERS) {
        await trigger(page, id).click();
        await expect(trigger(page, id)).toHaveAttribute("aria-expanded", "true");
        await expect(panel(page, id)).toBeVisible();
        const parent = await box(page.locator(`[data-file-submenu="${id}"]`));
        const flyout = await box(panel(page, id));
        expect(flyout.x, `${id} opens to the right of its row`).toBeGreaterThanOrEqual(parent.x + parent.width);
        await expect(panel(page, id)).not.toHaveAttribute("data-side", "left");
        await viewportFit(page, panel(page, id));
        for (const other of TRIGGERS.filter((o) => o !== id)) await expect(panel(page, other)).toBeHidden();
      }
      expect(unexpected).toEqual([]);
    });
  }

  test("keyboard: ArrowRight/Enter/Space open, Arrow keys and Home/End move, ArrowLeft and Escape close step by step", async ({ page }) => {
    await serve(page);
    await page.goto("/");
    await openFile(page);
    const exchange = trigger(page, "exchange");
    await exchange.focus();
    await page.keyboard.press("ArrowRight");
    await expect(exchange).toHaveAttribute("aria-expanded", "true");
    const items = panel(page, "exchange").getByRole("menuitem");
    // No project is open: every Import / Export entry is disabled, so focus stays on the parent row.
    await expect(items.first()).toBeDisabled();
    await expect(exchange).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(exchange).toHaveAttribute("aria-expanded", "false");
    await expect(page.locator(".file-menu")).toHaveJSProperty("open", true);

    const help = trigger(page, "help");
    await help.focus();
    await page.keyboard.press("Enter");
    await expect(help).toHaveAttribute("aria-expanded", "true");
    const helpItems = panel(page, "help").getByRole("menuitem");
    await expect(helpItems.first()).toBeFocused();
    await page.keyboard.press("ArrowDown");
    await expect(helpItems.nth(1)).toBeFocused();
    await page.keyboard.press("End");
    await expect(helpItems.last()).toBeFocused();
    await page.keyboard.press("ArrowDown");
    await expect(helpItems.first()).toBeFocused();
    await page.keyboard.press("ArrowUp");
    await expect(helpItems.last()).toBeFocused();
    await page.keyboard.press("Home");
    await expect(helpItems.first()).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(help).toHaveAttribute("aria-expanded", "false");
    await expect(help).toBeFocused();

    await page.keyboard.press("Space");
    await expect(help).toHaveAttribute("aria-expanded", "true");
    await expect(helpItems.first()).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(help).toBeFocused();
    await expect(page.locator(".file-menu")).toHaveJSProperty("open", true);
    await page.keyboard.press("Escape");
    await expect(page.locator(".file-menu")).toHaveJSProperty("open", false);
    await expect(page.locator(".file-menu > summary")).toBeFocused();
  });

  test("an outside click closes the submenu with the menu, and a reopened menu starts folded", async ({ page }) => {
    await serve(page);
    await page.goto("/");
    await openFile(page);
    await openFileSubmenu(page, "help");
    await page.mouse.click(700, 600);
    await expect(page.locator(".file-menu")).toHaveJSProperty("open", false);
    await expect(panel(page, "help")).toBeHidden();
    await openFile(page);
    await expect(trigger(page, "help")).toHaveAttribute("aria-expanded", "false");
    await expect(panel(page, "help")).toBeHidden();
  });

  test("activating an item runs it once, closes the whole menu and returns focus sensibly", async ({ page }) => {
    await serve(page);
    await page.goto("/");
    await openFile(page);
    await openFileSubmenu(page, "help");
    await page.getByRole("menuitem", { name: en["toolbar.about"], exact: true }).click();
    await expect(page.getByRole("dialog", { name: en["about.title"], exact: true })).toHaveCount(1);
    await expect(page.locator(".file-menu")).toHaveJSProperty("open", false);
  });

  test("hover opens after a short intent delay and a diagonal move into the panel does not close it", { tag: "@exclusive" }, async ({ page }) => {
    await serve(page);
    await page.goto("/");
    await openFile(page);
    const exchange = trigger(page, "exchange");
    await exchange.hover();
    await expect(panel(page, "exchange")).toBeVisible();
    // Diagonal: from the row's centre across the sibling rows into the panel, in small steps.
    const from = await box(exchange);
    await panel(page, "exchange").getByRole("menuitem").last().scrollIntoViewIfNeeded();
    const target = await box(panel(page, "exchange").getByRole("menuitem").last());
    await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
    await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2, { steps: 12 });
    await page.waitForTimeout(500);
    await expect(panel(page, "exchange")).toBeVisible();
    await expect(panel(page, "history")).toBeHidden();
    // Resting on another top-level row folds the hover-opened submenu again.
    await page.locator(".file-menu-content > button", { hasText: en["toolbar.newProject"] }).hover();
    await expect(panel(page, "exchange")).toBeHidden();
    // Hovering the sibling swaps the open submenu.
    await trigger(page, "history").hover();
    await expect(panel(page, "history")).toBeVisible();
    await trigger(page, "help").hover();
    await expect(panel(page, "help")).toBeVisible();
    await expect(panel(page, "history")).toBeHidden();
  });

  test("flips to the left when the menu is anchored near the right edge", async ({ page }) => {
    await serve(page);
    await page.goto("/");
    // A menu that sits at the right edge of the toolbar leaves no room for a flyout on its right.
    await page.addStyleTag({ content: ".file-menu { order: 99; } .file-menu-content { left: auto; right: 0; }" });
    await openFile(page);
    await openFileSubmenu(page, "exchange");
    await expect(panel(page, "exchange")).toHaveAttribute("data-side", "left");
    const parent = await box(page.locator('[data-file-submenu="exchange"]'));
    const flyout = await box(panel(page, "exchange"));
    expect(flyout.x + flyout.width).toBeLessThanOrEqual(parent.x + 0.5);
    await viewportFit(page, panel(page, "exchange"));
  });

  test("stays inside the viewport at a mid width where neither side has room", async ({ page }) => {
    await serve(page);
    await page.setViewportSize({ width: 700, height: 700 });
    await page.goto("/");
    await openFile(page);
    await openFileSubmenu(page, "help");
    await viewportFit(page, panel(page, "help"));
  });

  for (const language of ["en", "de"] as const) {
    test(`${language} narrow 400px: the submenu expands inline under its row instead of flying out`, async ({ page }) => {
      const m: Catalogue = language === "de" ? de : en;
      await serve(page, { language });
      await page.setViewportSize({ width: 400, height: 800 });
      await page.goto("/");
      await openFile(page);
      await openFileSubmenu(page, "help");
      await expect(panel(page, "help")).toHaveCSS("position", "static");
      const row = await box(trigger(page, "help"));
      const inline = await box(panel(page, "help"));
      expect(inline.y, "inline panel sits below its row").toBeGreaterThanOrEqual(row.y + row.height - 0.5);
      await viewportFit(page, panel(page, "help"));
      expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
      await page.getByRole("menuitem", { name: m["toolbar.about"], exact: true }).click();
      await expect(page.getByRole("dialog", { name: m["about.title"], exact: true })).toBeVisible();
    });
  }

  test("motion: a short opacity and translate animation by default; 0 s with level Off, none with reduced motion", async ({ page }) => {
    await serve(page);
    await page.goto("/");
    await openFile(page);
    await openFileSubmenu(page, "help");
    const animation = (id: string) => panel(page, id).evaluate((node) => {
      const style = getComputedStyle(node);
      return { name: style.animationName, seconds: parseFloat(style.animationDuration) };
    });
    const standard = await animation("help");
    expect(standard.name).toBe("knx-file-submenu-in");
    expect(standard.seconds).toBeGreaterThan(0.1);
    expect(standard.seconds).toBeLessThanOrEqual(0.16);
  });

  test("motion level Off leaves no time to animate in", async ({ page }) => {
    await serve(page, { motionLevel: "off" });
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("data-motion-level", "off");
    await openFile(page);
    await openFileSubmenu(page, "help");
    await expect(panel(page, "help")).toHaveCSS("animation-duration", "0s");
  });

  test("OS reduced motion removes the animation", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await serve(page);
    await page.goto("/");
    await openFile(page);
    await openFileSubmenu(page, "help");
    await expect(panel(page, "help")).toHaveCSS("animation-name", "none");
  });

  test("the narrow layout fades instead of sliding", async ({ page }) => {
    await serve(page);
    await page.setViewportSize({ width: 400, height: 800 });
    await page.goto("/");
    await openFile(page);
    await openFileSubmenu(page, "help");
    await expect(panel(page, "help")).toHaveCSS("animation-name", "knx-file-submenu-fade");
  });

  for (const theme of THEMES) {
    test(`panel and rows stay legible and distinct (${theme})`, async ({ page }) => {
      await serve(page, { theme });
      await page.goto("/");
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await openFile(page);
      await openFileSubmenu(page, "help");
      const surface = panel(page, "help");
      const row = surface.getByRole("menuitem", { name: en["toolbar.about"], exact: true });
      const { ratio, alpha } = await contrast(row, surface);
      expect(alpha, "panel is opaque").toBe(1);
      expect(ratio, "row label against panel").toBeGreaterThanOrEqual(4.5);
      await expect(surface).not.toHaveCSS("border-top-width", "0px");
      const menuBackground = await page.locator(".file-menu-content").evaluate((n) => getComputedStyle(n).backgroundColor);
      expect(menuBackground).not.toBe("rgba(0, 0, 0, 0)");
      await viewportFit(page, surface);
    });
  }
});

/** An open fictional project with a project history and a comparison route; anything else is recorded. */
async function serveProject(page: Page, { compareAdded = 0, knxproj = false }: { compareAdded?: number; knxproj?: boolean } = {}) {
  const unexpected: string[] = [];
  const history = admitProjectHistory(structuredClone(HISTORY_FIXTURE));
  const emptyTable = { added: [], removed: [], changed: [], ambiguous: [] };
  const added = Array.from({ length: compareAdded }, (_, i) => [
    { etsId: null, address: `1/1/${i}` },
    { name: `Fictional group address with a deliberately long descriptive name number ${i}`, central: false, unfiltered: false, range: null },
  ]);
  const importDiagnostics = knxproj ? [
    { timestamp: "2026-10-18T10:00:00Z", severity: "warning", source: "import", message: "Fictional unmapped parameter", location: "P-0001", detail: null },
  ] : [];
  const report = { inputKind: knxproj ? "knxproj" : "knxdb", importReport: null, importDiagnostics, infoChanges: [], installations: [
    { id: 0, status: "matched", fieldChanges: [], areas: emptyTable, lines: emptyTable, devices: emptyTable, groupRanges: emptyTable,
      groupAddresses: { ...emptyTable, added }, buildings: emptyTable, functions: emptyTable }] };
  await page.route("https://api.github.com/**", (route) => { unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: {
      uiLanguage: "en", theme: "porcelain", motionLevel: "off", releaseChecksEnabled: false, achievementsEnabled: "false",
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.14" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.14" });
    if (path === "/api/project" && method === "GET") return ok(history.project);
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(history.project));
    if (path === "/api/project/history" && method === "GET") return ok(history);
    if (path === "/api/project/diff" && method === "POST") return ok(report);
    if (path === "/api/fs/list") return ok([{ name: knxproj ? "compare-fictional.knxproj" : "compare-fictional.knxdb", is_dir: false }]);
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    unexpected.push(`${method} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"file-menu project fixture only"}' });
  });
  return unexpected;
}

const fileControl = (page: Page) => page.locator(".file-menu > summary");

test.describe("File menu as a WAI-ARIA menu button", () => {
  test.beforeEach(async ({ page }) => { await page.setViewportSize({ width: 1440, height: 900 }); });

  test("the File control announces its popup and state, and the open menu has one tab stop per level", async ({ page }) => {
    await serveProject(page);
    await page.goto("/?view=editor");
    await expect(fileControl(page)).toHaveAttribute("aria-haspopup", "menu");
    await expect(fileControl(page)).toHaveAttribute("aria-expanded", "false");
    await openFile(page);
    await expect(fileControl(page)).toHaveAttribute("aria-expanded", "true");
    const stops = await page.evaluate(() => [...document.querySelectorAll('.file-menu [role="menu"]')].map((level) =>
      [...level.querySelectorAll<HTMLElement>("[data-file-menu-item]")]
        .filter((el) => el.closest('[role="menu"]') === level && el.tabIndex === 0).length));
    expect(stops).toEqual([1, 1, 1, 1]);
  });

  test("ArrowDown on File opens the menu onto its first entry; ArrowUp onto its last", async ({ page }) => {
    await serve(page);
    await page.goto("/");
    const items = page.locator(".file-menu-content > [data-file-menu-item]:not([disabled])");
    await fileControl(page).focus();
    await page.keyboard.press("ArrowDown");
    await expect(page.locator(".file-menu")).toHaveJSProperty("open", true);
    await expect(items.first()).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(fileControl(page)).toBeFocused();
    await page.keyboard.press("ArrowUp");
    // The last enabled entry of the first level is the Help row (a submenu trigger).
    await expect(trigger(page, "help")).toBeFocused();
  });

  test("Enter on File opens the menu with focus on its first entry", async ({ page }) => {
    await serve(page);
    await page.goto("/");
    await fileControl(page).focus();
    await page.keyboard.press("Enter");
    await expect(page.locator(".file-menu")).toHaveJSProperty("open", true);
    await expect(page.locator(".file-menu-content > [data-file-menu-item]:not([disabled])").first()).toBeFocused();
  });

  test("Tab and Shift+Tab close the whole menu, from the first level and from a submenu, and move on from File", async ({ page }) => {
    await serve(page);
    await page.goto("/");
    // Natural Tab order from File: what follows it in the document, never something inside the closed menu.
    const followsFile = () => page.evaluate(() => {
      const file = document.querySelector(".file-menu > summary")!;
      const active = document.activeElement!;
      return !document.querySelector(".file-menu")!.contains(active)
        && Boolean(file.compareDocumentPosition(active) & Node.DOCUMENT_POSITION_FOLLOWING);
    });
    await fileControl(page).focus();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Tab");
    await expect(page.locator(".file-menu")).toHaveJSProperty("open", false);
    expect(await followsFile()).toBe(true);

    await fileControl(page).focus();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Shift+Tab");
    await expect(page.locator(".file-menu")).toHaveJSProperty("open", false);
    await expect(fileControl(page)).not.toBeFocused();
    await expect(page.locator(".workbench-brand")).toBeFocused();

    await fileControl(page).focus();
    await page.keyboard.press("ArrowDown");
    await trigger(page, "help").focus();
    await page.keyboard.press("ArrowRight");
    await expect(panel(page, "help")).toBeVisible();
    await page.keyboard.press("Tab");
    await expect(page.locator(".file-menu")).toHaveJSProperty("open", false);
    await expect(panel(page, "help")).toBeHidden();
    expect(await followsFile()).toBe(true);
  });
});

test.describe("File menu holds entries only", () => {
  test.beforeEach(async ({ page }) => { await page.setViewportSize({ width: 1440, height: 900 }); });

  const levels = [{ name: "first level", submenu: null }, ...TRIGGERS.map((id) => ({ name: `${id} submenu`, submenu: id }))] as const;

  test("every focusable descendant of the open menu is a declared entry", async ({ page }) => {
    await serveProject(page);
    await page.goto("/?view=editor");
    await openFile(page);
    for (const id of TRIGGERS) await openFileSubmenu(page, id);
    const strays = await page.evaluate(() => [...document.querySelectorAll<HTMLElement>(
      ".file-menu-content button, .file-menu-content input, .file-menu-content select, .file-menu-content textarea, .file-menu-content a[href], .file-menu-content [tabindex]",
    )].filter((el) => !el.hasAttribute("data-file-menu-item") && !(el as HTMLButtonElement).disabled && el.getClientRects().length > 0)
      .map((el) => `${el.tagName} ${el.textContent}`));
    expect(strays).toEqual([]);
    await expect(page.locator(".file-menu-content .project-diff-panel, .file-menu-content [role=group]")).toHaveCount(0);
  });

  for (const level of levels) for (const shift of [false, true]) {
    test(`${shift ? "Shift+Tab" : "Tab"} from every entry of the ${level.name} closes File`, async ({ page }) => {
      await serveProject(page);
      await page.goto("/?view=editor");
      const scope = level.submenu ? panel(page, level.submenu) : page.locator(".file-menu-content");
      const entries = level.submenu ? "[data-file-menu-item]:not([disabled])" : ":scope > [data-file-menu-item]:not([disabled])";
      await openFile(page);
      if (level.submenu) await openFileSubmenu(page, level.submenu);
      const count = await scope.locator(entries).count();
      expect(count).toBeGreaterThan(0);
      for (let at = 0; at < count; at += 1) {
        if (!(await page.locator(".file-menu").evaluate((n: HTMLDetailsElement) => n.open))) await openFile(page);
        if (level.submenu && !(await panel(page, level.submenu).isVisible())) await openFileSubmenu(page, level.submenu);
        await scope.locator(entries).nth(at).focus();
        await page.keyboard.press(shift ? "Shift+Tab" : "Tab");
        await expect(page.locator(".file-menu")).toHaveJSProperty("open", false);
        for (const id of TRIGGERS) await expect(trigger(page, id)).toHaveAttribute("aria-expanded", "false");
      }
    });
  }
});

test.describe("File menu focus restoration", () => {
  test.beforeEach(async ({ page }) => { await page.setViewportSize({ width: 1440, height: 900 }); });

  const dialogs = [
    { name: "About", submenu: "help", entry: en["toolbar.about"], title: en["about.title"], close: en["about.close"] },
    { name: "Project history", submenu: "history", entry: en["projectHistory.title"], title: en["projectHistory.title"], close: en["projectHistory.close"] },
    { name: "Diagnostic export", submenu: "help", entry: en["diagnostics.title"], title: en["diagnostics.title"], close: en["support.close"] },
    // DIAG-03: opens at the problem type, which asks the server nothing.
    { name: "Report problem", submenu: "help", entry: en["reportProblem.button"], title: en["reportProblem.title"], close: en["reportProblem.close"] },
  ] as const;

  for (const dialog of dialogs) for (const how of ["Escape", "close button"] as const) {
    test(`${dialog.name} opened from its submenu returns focus to File on ${how}`, async ({ page }) => {
      const unexpected = await serveProject(page);
      await page.goto("/?view=editor");
      await openFile(page);
      await openFileSubmenu(page, dialog.submenu);
      await page.getByRole("menuitem", { name: dialog.entry, exact: true }).click();
      const opened = page.getByRole("dialog", { name: dialog.title, exact: true });
      await expect(opened).toBeVisible();
      if (how === "Escape") await page.keyboard.press("Escape");
      else await opened.getByRole("button", { name: dialog.close, exact: true }).click();
      await expect(opened).toBeHidden();
      await expect(fileControl(page)).toBeFocused();
      expect(unexpected).toEqual([]);
    });
  }
});

test.describe("File menu placement and comparison dialog", () => {
  test("the comparison result opens as a dialog inside the viewport, the menu closes and File gets focus back", async ({ page }) => {
    const unexpected = await serveProject(page, { compareAdded: 60 });
    await page.setViewportSize({ width: 800, height: 640 });
    await page.goto("/?view=editor");
    for (const how of ["Escape", "close button"] as const) {
      await openFile(page);
      await openFileSubmenu(page, "history");
      await page.getByRole("menuitem", { name: en["projectDiff.compareButton"], exact: true }).click();
      await expect(page.locator(".file-menu")).toHaveJSProperty("open", false);
      await page.getByRole("button", { name: "compare-fictional.knxdb", exact: true }).click();
      const report = page.getByRole("dialog", { name: en["projectDiff.title"], exact: true });
      await expect(report).toBeVisible();
      await expect(report).toContainText("Group addresses: 60 added");
      await expect(page.locator(".file-menu-content").getByRole("dialog")).toHaveCount(0);
      await viewportFit(page, report);
      // Expanding the entity tables grows the dialog; it still fits and scrolls inside itself.
      const toggles = report.locator(".project-diff-toggle");
      for (let i = 0; i < await toggles.count(); i += 1) await toggles.nth(i).click();
      await viewportFit(page, report);
      // The dialog is modal: Tab cycles inside it and never reaches the File control.
      for (let i = 0; i < 12; i += 1) {
        await page.keyboard.press("Tab");
        expect(await page.evaluate(() => document.activeElement?.closest('[role="dialog"]') !== null)).toBe(true);
      }
      if (how === "Escape") await page.keyboard.press("Escape");
      else await report.getByRole("button", { name: en["projectDiff.close"], exact: true }).click();
      await expect(report).toBeHidden();
      await expect(fileControl(page)).toBeFocused();
      await expect(page.locator(".file-menu")).toHaveJSProperty("open", false);
    }
    expect(unexpected).toEqual([]);
  });

  test("a .knxproj comparison keeps the ETS import report reachable by Tab and Shift+Tab, and Enter opens it", async ({ page }) => {
    const unexpected = await serveProject(page, { compareAdded: 3, knxproj: true });
    await page.goto("/?view=editor");
    await openFile(page);
    await openFileSubmenu(page, "history");
    await page.getByRole("menuitem", { name: en["projectDiff.compareButton"], exact: true }).click();
    await page.getByRole("button", { name: "compare-fictional.knxproj", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: en["projectDiff.title"], exact: true });
    await expect(dialog).toBeVisible();
    const disclosure = dialog.locator("summary");
    await expect(disclosure).toHaveText(/ETS import report: 1 diagnostic/);
    const onDisclosure = () => disclosure.evaluate((node) => node === document.activeElement);
    const inDialog = () => page.evaluate(() => document.activeElement?.closest('[role="dialog"]') !== null);
    const stops = await dialog.locator("summary, button").count();
    // Forward: the disclosure is one stop of a closed cycle, and focus never leaves the dialog.
    let forward = false;
    for (let i = 0; i < stops + 1; i += 1) {
      await page.keyboard.press("Tab");
      expect(await inDialog()).toBe(true);
      forward ||= await onDisclosure();
    }
    expect(forward).toBe(true);
    // Backward: Shift+Tab walks the same cycle and reaches it as well.
    let backward = false;
    for (let i = 0; i < stops + 1; i += 1) {
      await page.keyboard.press("Shift+Tab");
      expect(await inDialog()).toBe(true);
      backward ||= await onDisclosure();
    }
    expect(backward).toBe(true);
    // Enter on the focused disclosure opens it.
    await disclosure.focus();
    await expect(dialog.locator("details")).toHaveJSProperty("open", false);
    await page.keyboard.press("Enter");
    await expect(dialog.locator("details")).toHaveJSProperty("open", true);
    await expect(dialog).toContainText("Fictional unmapped parameter");
    // The opened report adds no stop that breaks the trap.
    for (let i = 0; i < stops + 2; i += 1) {
      await page.keyboard.press("Tab");
      expect(await inDialog()).toBe(true);
    }
    expect(unexpected).toEqual([]);
  });

  test("a short, wide viewport scrolls the menu so the last row (Logout) stays reachable", async ({ page }) => {
    await serve(page, { sessionRequired: true });
    await page.setViewportSize({ width: 1440, height: 500 });
    await page.goto("/");
    await openFile(page);
    const content = page.locator(".file-menu-content");
    const logout = page.locator(".file-menu-logout");
    await expect(logout).toHaveCount(1);
    await openFileSubmenu(page, "help");
    // Flyouts would be clipped by the scrolling menu, so the submenu expands inline.
    await expect(panel(page, "help")).toHaveCSS("position", "static");
    await expect(content).toHaveCSS("overflow-y", "auto");
    expect(await content.evaluate((n) => n.scrollHeight > n.clientHeight), "the open menu is taller than its box").toBe(true);
    await viewportFit(page, content);
    await logout.scrollIntoViewIfNeeded();
    const row = await box(logout);
    const menu = await box(content);
    expect(row.y, "Logout inside the menu box").toBeGreaterThanOrEqual(menu.y - 0.5);
    expect(row.y + row.height, "Logout inside the menu box").toBeLessThanOrEqual(menu.y + menu.height + 0.5);
    expect(row.y + row.height, "Logout inside the viewport").toBeLessThanOrEqual(500);
  });
});

/** True when the pointer at the node's centre, or at its text, lands on the node or a descendant. */
async function paintsOnTop(locator: Locator, where: "centre" | "label") {
  return locator.evaluate((node, spot) => {
    const range = document.createRange();
    range.selectNodeContents(node);
    const r = spot === "label" ? range.getBoundingClientRect() : node.getBoundingClientRect();
    const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    return hit !== null && node.contains(hit);
  }, where);
}

test.describe("File submenus just above the inline breakpoint (UISET-02)", () => {
  const WINDOWS = [
    ...[651, 660, 700, 760, 900].map((width) => ({ width, height: 800 })),
    { width: 900, height: 560 }, // short: the inline layout by height
    { width: 760, height: 601 }, // just taller than the short limit: flyouts with a moved-up panel
  ];
  for (const language of ["en", "de"] as const) {
    for (const { width, height } of WINDOWS) {
      test(`${language} ${width}x${height}: every submenu box stays in the viewport and every entry takes a real pointer`, async ({ page }) => {
        await serve(page, { language });
        await page.setViewportSize({ width, height });
        await page.goto("/");
        for (const id of TRIGGERS) {
          // One menu for all three submenus: consecutive real clicks, no fresh menu in between.
          if (id === TRIGGERS[0]) await openFile(page);
          await openFileSubmenu(page, id);
          await viewportFit(page, panel(page, id));
          const entries = panel(page, id).locator("[data-file-menu-item]");
          const count = await entries.count();
          expect(count, `${id} has entries`).toBeGreaterThan(0);
          for (let i = 0; i < count; i += 1) {
            const entry = entries.nth(i);
            const label = (await entry.textContent())?.trim();
            await viewportFit(page, entry);
            expect(await paintsOnTop(entry, "centre"), `${id} / ${label}: the pointer reaches the entry`).toBe(true);
            if (await entry.isEnabled()) await entry.click({ trial: true });
          }
          expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), "no sideways scroll").toBeLessThanOrEqual(1);
        }
        // One real activation per window: About opens from the last submenu whatever its layout.
        await page.getByRole("menuitem", { name: (language === "de" ? de : en)["toolbar.about"], exact: true }).click();
        await expect(page.getByRole("dialog", { name: (language === "de" ? de : en)["about.title"], exact: true })).toBeVisible();
      });
    }
  }

  test("the minimum width is one value: the panel states it and the stylesheet applies it", async ({ page }) => {
    await serve(page);
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await openFile(page);
    await openFileSubmenu(page, "exchange");
    const stated = await panel(page, "exchange").evaluate((n) => (n as HTMLElement).style.getPropertyValue("--file-submenu-min-width"));
    expect(stated).toMatch(/^\d+px$/);
    await expect(panel(page, "exchange")).toHaveCSS("min-width", stated);
  });
});

test.describe("The open File menu paints above the toolbar (UISET-03)", () => {
  for (const theme of ["porcelain", "graphite"] as const) {
    for (const width of [400, 1440]) {
      test(`${theme} ${width}px, no project: every first-level entry is reachable at its centre and its label`, async ({ page }) => {
        await serve(page, { theme });
        await page.setViewportSize({ width, height: 900 });
        await page.goto("/");
        await openFile(page);
        // The disabled toolbar controls (Search, Save) are what used to paint over the menu.
        await expect(page.locator(".workbench-toolbar button:disabled").first()).toBeVisible();
        const entries = page.locator(".file-menu-content > [data-file-menu-item], .file-menu-content > .file-submenu > [data-file-menu-item]");
        const count = await entries.count();
        expect(count).toBeGreaterThanOrEqual(7);
        for (let i = 0; i < count; i += 1) {
          const entry = entries.nth(i);
          const label = (await entry.textContent())?.trim();
          expect(await paintsOnTop(entry, "centre"), `${label}: centre`).toBe(true);
          expect(await paintsOnTop(entry, "label"), `${label}: label`).toBe(true);
        }
      });
    }
  }

  for (const width of [400, 1440]) {
    test(`${width}px: real-pointer clicks on New project and Open run their own handlers`, async ({ page }) => {
      await serve(page);
      let listed = 0;
      await page.route("**/api/fs/list**", (route) => { listed += 1; return route.fulfill({ status: 200, contentType: "application/json", body: "[]" }); });
      await page.setViewportSize({ width, height: 900 });
      await page.goto("/");
      await openFile(page);
      await page.getByRole("menuitem", { name: en["toolbar.newProject"], exact: true }).click();
      await expect(page.getByRole("dialog", { name: en["newProject.title"], exact: true })).toBeVisible();
      expect(listed, "New project does not browse files").toBe(0);
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog", { name: en["newProject.title"], exact: true })).toHaveCount(0);
      await openFile(page);
      await page.getByRole("menuitem", { name: en["toolbar.openProject"], exact: true }).click();
      await expect.poll(() => listed, "Open browses the mounted files").toBeGreaterThan(0);
    });
  }

  test("a modal opened from the menu paints above the toolbar and above a menu left open", async ({ page }) => {
    await serve(page);
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await openFile(page);
    await openFileSubmenu(page, "help");
    await page.getByRole("menuitem", { name: en["toolbar.about"], exact: true }).click();
    const dialog = page.getByRole("dialog", { name: en["about.title"], exact: true });
    await expect(dialog).toBeVisible();
    // The modal makes the page behind it inert, which alone would hide it from hit testing. Lift
    // that and force the menu open again, so the answer depends on paint order only.
    await page.evaluate(() => {
      document.querySelectorAll("[inert]").forEach((node) => node.removeAttribute("inert"));
      (document.querySelector(".file-menu") as HTMLDetailsElement).open = true;
    });
    const behind = [".workbench-toolbar button:disabled", ".file-menu-content > [data-file-menu-item]", ".file-menu > summary"];
    for (const selector of behind) {
      const target = page.locator(selector).first();
      await expect(target, selector).toBeVisible();
      const covered = await target.evaluate((node) => {
        const r = node.getBoundingClientRect();
        const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
        return hit !== null && hit.closest(".search-overlay") !== null;
      });
      expect(covered, `${selector} lies under the modal overlay`).toBe(true);
    }
  });
});

test.describe("Inline submenus ignore hover (UISET-02 round 2)", () => {
  // 400: the narrow media rule; 900x560: the short-window rule; 660: the per-instance inline strip.
  const INLINE_WINDOWS = [{ width: 400, height: 800 }, { width: 900, height: 560 }, { width: 660, height: 800 }];
  const expanded = (page: Page, id: string) => trigger(page, id).getAttribute("aria-expanded");

  /** Moves the real mouse onto the middle of a row and rests there, longer than any hover timer. */
  async function restOn(page: Page, locator: Locator) {
    await locator.scrollIntoViewIfNeeded();
    const b = await box(locator);
    await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, { steps: 6 });
    await page.waitForTimeout(500);
  }

  for (const language of ["en", "de"] as const) {
    for (const { width, height } of INLINE_WINDOWS) {
      test(`${language} ${width}x${height}: a hover never opens or swaps a submenu, a click does`, { tag: "@exclusive" }, async ({ page }) => {
        await serve(page, { language });
        await page.setViewportSize({ width, height });
        await page.goto("/");
        await openFile(page);
        const pairs = [["exchange", "history"], ["history", "help"]] as const;
        await openFileSubmenu(page, pairs[0][0]);
        await expect(panel(page, pairs[0][0]), "the opened panel is laid out inline").toHaveCSS("position", "static");
        for (const [open, next] of pairs) {
          await restOn(page, trigger(page, next));
          expect(await expanded(page, open), `${open} stays open under a resting mouse on ${next}`).toBe("true");
          expect(await expanded(page, next), `${next} stays closed without a click`).toBe("false");
          await expect(panel(page, open)).toBeVisible();
          await expect(panel(page, next)).toBeHidden();
          await trigger(page, next).click();
          await expect(trigger(page, next)).toHaveAttribute("aria-expanded", "true");
          await expect(panel(page, next)).toBeVisible();
          await expect(trigger(page, open)).toHaveAttribute("aria-expanded", "false");
          await expect(panel(page, open)).toBeHidden();
        }
        // A rest after the last click, wherever the rows went, changes nothing either.
        await page.waitForTimeout(500);
        await expect(trigger(page, "help")).toHaveAttribute("aria-expanded", "true");
        for (const id of ["exchange", "history"] as const) await expect(trigger(page, id)).toHaveAttribute("aria-expanded", "false");
      });

      test(`${language} ${width}x${height}: with nothing open a resting mouse opens nothing, and the keyboard still discloses`, { tag: "@exclusive" }, async ({ page }) => {
        await serve(page, { language });
        await page.setViewportSize({ width, height });
        await page.goto("/");
        await openFile(page);
        await restOn(page, trigger(page, "exchange"));
        for (const id of TRIGGERS) await expect(trigger(page, id)).toHaveAttribute("aria-expanded", "false");
        await trigger(page, "exchange").focus();
        await page.keyboard.press("ArrowRight");
        await expect(trigger(page, "exchange")).toHaveAttribute("aria-expanded", "true");
        // No project: every Import / Export entry is disabled, so focus stays on the row; Escape folds it.
        await page.keyboard.press("Escape");
        await expect(trigger(page, "exchange")).toHaveAttribute("aria-expanded", "false");
        await trigger(page, "help").focus();
        await page.keyboard.press("Enter");
        await expect(trigger(page, "help")).toHaveAttribute("aria-expanded", "true");
        await expect(panel(page, "help").getByRole("menuitem").first()).toBeFocused();
        await page.keyboard.press("ArrowLeft");
        await expect(trigger(page, "help")).toHaveAttribute("aria-expanded", "false");
        await expect(trigger(page, "help")).toBeFocused();
        await page.keyboard.press("Space");
        await expect(trigger(page, "help")).toHaveAttribute("aria-expanded", "true");
      });
    }

    test(`${language} 1440x900: a flyout still opens and swaps on hover intent`, { tag: "@exclusive" }, async ({ page }) => {
      await serve(page, { language });
      await page.setViewportSize({ width: 1440, height: 900 });
      await page.goto("/");
      await openFile(page);
      await restOn(page, trigger(page, "exchange"));
      await expect(panel(page, "exchange")).toBeVisible();
      await expect(panel(page, "exchange")).toHaveCSS("position", "absolute");
      await restOn(page, trigger(page, "history"));
      await expect(panel(page, "history")).toBeVisible();
      await expect(panel(page, "exchange")).toBeHidden();
    });
  }
});

/** The application scale the page renders with, read the way App writes it onto the root. */
const renderedScale = (page: Page) => page.evaluate(() =>
  Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--app-ui-scale")) || 1);

/**
 * Checks one open submenu at application scale `scale` against geometry the test reads itself
 * (DOMRect and the window, both visual px): a flyout lies inside the viewport, is at least the
 * 220 layout px minimum wide, uses exactly the room it was capped to and sits above the bottom
 * margin; an inline submenu only appears where neither side of its row holds that minimum in
 * layout px. Then every entry, scrolled into view, lies inside the viewport and takes the pointer.
 */
async function checkZoomedSubmenu(page: Page, id: string, scale: number) {
  // The slide-in starts 6 px to the side; geometry is read where the panel comes to rest.
  await panel(page, id).evaluate((node) => Promise.all(node.getAnimations().map((a) => a.finished)));
  const g = await page.evaluate((sid) => {
    const node = document.querySelector<HTMLElement>(`[data-file-submenu-panel="${sid}"]`)!;
    const row = node.parentElement!;
    const r = (b: DOMRect) => ({ left: b.left, top: b.top, right: b.right, bottom: b.bottom, width: b.width });
    const style = getComputedStyle(node);
    return {
      position: style.position, maxHeight: Number.parseFloat(style.maxHeight), perInstance: row.dataset.layout === "inline",
      side: node.dataset.side ?? "right", cap: node.style.getPropertyValue("--file-submenu-max-width"),
      shift: node.style.getPropertyValue("--file-submenu-shift"),
      panel: r(node.getBoundingClientRect()), row: r(row.getBoundingClientRect()), width: innerWidth, height: innerHeight,
    };
  }, id);
  const margin = 8 * scale;
  const minimum = 220 * scale;
  if (g.position === "absolute") {
    await viewportFit(page, panel(page, id));
    expect(g.panel.width, `${id}: at least the minimum width`).toBeGreaterThanOrEqual(minimum - 0.5);
    expect(g.panel.bottom, `${id}: above the bottom margin`).toBeLessThanOrEqual(g.height - margin + 1);
    expect(g.maxHeight * scale, `${id}: its height limit is the viewport's, in layout px`).toBeCloseTo(g.height - 24 * scale, 0);
    if (g.cap) {
      // A capped panel takes its side's whole room, and the cap is that room in layout px.
      if (g.side === "right") expect(g.panel.right, `${id}: capped to the right room`).toBeCloseTo(g.width - margin, 0);
      else expect(g.panel.left, `${id}: capped to the left room`).toBeCloseTo(margin, 0);
      expect(Number.parseFloat(g.cap) * scale, `${id}: the cap is layout px`).toBeCloseTo(g.panel.width, 0);
    }
    if (g.shift) {
      const stopsAtBottom = Math.abs(g.panel.bottom - (g.height - margin)) <= 1;
      const stopsAtTop = Math.abs(g.panel.top - margin) <= 1;
      expect(stopsAtBottom || stopsAtTop, `${id}: moved up exactly to the bottom margin (or the top one)`).toBe(true);
    }
  } else if (g.perInstance) {
    // Inline by placement: neither side of the row keeps the minimum, measured in layout px.
    const right = (g.width - margin - (g.row.right + 6 * scale)) / scale;
    const left = (g.row.left - 6 * scale - margin) / scale;
    expect(Math.max(right, left), `${id}: inline only where no side holds 220 layout px`).toBeLessThan(220 + 0.5);
  }
  await viewportFit(page, page.locator(".file-menu-content"));
  const entries = panel(page, id).locator("[data-file-menu-item]");
  const count = await entries.count();
  expect(count, `${id} has entries`).toBeGreaterThan(0);
  for (let i = 0; i < count; i += 1) {
    const entry = entries.nth(i);
    const label = (await entry.textContent())?.trim();
    await entry.scrollIntoViewIfNeeded();
    await viewportFit(page, entry);
    expect(await paintsOnTop(entry, "centre"), `${id} / ${label}: the pointer reaches the entry`).toBe(true);
    if (await entry.isEnabled()) await entry.click({ trial: true });
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), "no sideways scroll").toBeLessThanOrEqual(1);
}

/**
 * A weekday noon: no holiday and not late at night, so the app pushes no startup toast. Toasts
 * rightly paint above the menu, and at 150% the bottom-right toast reaches over a flyout's lower
 * entries; the hit tests are about the menu. Pinning the date (timers keep running) replaces a
 * fixed wait for a toast that may or may not come, which raced on a loaded host.
 */
// Local noon on purpose: Node and the browser share the host's time zone (the config sets no
// `timezoneId`), so the page sees 12:00 wherever the gate runs; a UTC noon is late night at +12.
const TOASTLESS_START = new Date("2026-10-14T12:00:00");
async function startWithoutToast(page: Page) {
  await page.clock.setFixedTime(TOASTLESS_START);
}
async function expectNoToast(page: Page) {
  await expect(page.locator(".toast")).toHaveCount(0);
}

test.describe("File submenus under application zoom (UISET2-01)", () => {
  // Widths at 150% put the room right of the menu (layout px = visual / 1.5) on both sides of the
  // 220 px minimum; at 80% the room the visual reading understates. 1440x620 and 1100x640 at 150%
  // are short in layout px (413 and 427) without reaching the 600 px media rule.
  const CASES = [
    ...[960, 990, 1010, 1040, 1080, 1440].map((width) => ({ scale: 1.5, width, height: 900 })),
    { scale: 1.5, width: 1440, height: 620 }, { scale: 1.5, width: 1100, height: 640 },
    ...[660, 690, 720, 760].map((width) => ({ scale: 0.8, width, height: 800 })),
    { scale: 0.8, width: 900, height: 601 },
  ];
  for (const language of ["en", "de"] as const) {
    for (const { scale, width, height } of CASES) {
      test(`${language} ${scale * 100}% ${width}x${height}: every submenu fits, holds its minimum and every entry takes a real pointer`, async ({ page }) => {
        const m: Catalogue = language === "de" ? de : en;
        const unexpected = await serve(page, { language, uiScale: scale });
        await page.setViewportSize({ width, height });
        await startWithoutToast(page);
        await page.goto("/");
        await expect.poll(() => renderedScale(page), "the stored scale reaches the root").toBe(scale);
        await expectNoToast(page);
        await openFile(page);
        for (const entry of await page.locator(".file-menu-content > [data-file-menu-item], .file-menu-content > .file-submenu > [data-file-menu-item]").all()) {
          await entry.scrollIntoViewIfNeeded();
          await viewportFit(page, entry);
        }
        for (const id of TRIGGERS) {
          await openFileSubmenu(page, id);
          await checkZoomedSubmenu(page, id, scale);
        }
        expect(unexpected).toEqual([]);
        await page.getByRole("menuitem", { name: m["toolbar.about"], exact: true }).click();
        await expect(page.getByRole("dialog", { name: m["about.title"], exact: true })).toBeVisible();
      });
    }
  }

  test("a zoom step with a submenu open places it again (Ctrl+Plus through the settings path)", async ({ page }) => {
    const unexpected = await serve(page);
    // 1000 px keeps a whole flyout at 100%; at 150% the same window is 667 layout px wide, which
    // leaves the room right of the menu near the 220 px minimum.
    await page.setViewportSize({ width: 1000, height: 900 });
    await startWithoutToast(page);
    await page.goto("/");
    await expectNoToast(page);
    await openFile(page);
    await openFileSubmenu(page, "help");
    await checkZoomedSubmenu(page, "help", 1);
    for (const expected of [1.1, 1.2, 1.3, 1.4, 1.5]) {
      await page.keyboard.press("Control+Equal");
      await expect.poll(() => renderedScale(page)).toBeCloseTo(expected, 5);
    }
    await expect(page.locator(".file-menu")).toHaveJSProperty("open", true);
    await expect(panel(page, "help")).toBeVisible();
    await checkZoomedSubmenu(page, "help", 1.5);
    expect(unexpected).toEqual([]);
  });
});

test.describe("File menu on a touchscreen (UISET2-02)", () => {
  test.use({ hasTouch: true });
  // 400: the narrow inline layout; 660: the per-instance inline strip above the breakpoint.
  for (const { width, height } of [{ width: 400, height: 800 }, { width: 660, height: 800 }]) {
    for (const language of ["en", "de"] as const) {
      test(`${language} ${width}x${height}: a tap on a sibling row opens it while another is open`, async ({ page }) => {
        const unexpected = await serve(page, { language });
        await page.setViewportSize({ width, height });
        await page.goto("/");
        await page.locator(".file-menu > summary").tap();
        await expect(page.locator(".file-menu-content")).toBeVisible();
        await trigger(page, "exchange").tap();
        await expect(trigger(page, "exchange")).toHaveAttribute("aria-expanded", "true");
        await expect(panel(page, "exchange"), "the open panel is laid out inline").toHaveCSS("position", "static");
        for (const [open, next] of [["exchange", "history"], ["history", "help"]] as const) {
          await trigger(page, next).scrollIntoViewIfNeeded();
          await trigger(page, next).tap();
          await expect(trigger(page, next), `${next} opens on a tap while ${open} is open`).toHaveAttribute("aria-expanded", "true");
          await expect(panel(page, next)).toBeVisible();
          await expect(trigger(page, open)).toHaveAttribute("aria-expanded", "false");
        }
        expect(unexpected).toEqual([]);
      });

      test(`${language} ${width}x${height}: a tap on a first-level action below an open submenu activates it`, async ({ page }) => {
        const m: Catalogue = language === "de" ? de : en;
        const unexpected = await serve(page, { language, sessionRequired: true });
        let logouts = 0;
        await page.route("**/api/auth/logout", (route) => { logouts += 1; return route.fulfill({ status: 200, contentType: "application/json", body: "{}" }); });
        await page.setViewportSize({ width, height });
        await page.goto("/");
        await page.locator(".file-menu > summary").tap();
        await trigger(page, "help").tap();
        await expect(trigger(page, "help")).toHaveAttribute("aria-expanded", "true");
        // Logout sits below the open Help panel: folding Help first would pull it up from under the finger.
        const logout = page.locator(".file-menu-logout");
        await logout.scrollIntoViewIfNeeded();
        await logout.tap();
        await expect.poll(() => logouts, "Logout runs on the tap").toBe(1);
        expect(unexpected).toEqual([]);
        // New project, above the open submenu, activates on a tap as well.
        await page.goto("/");
        await page.locator(".file-menu > summary").tap();
        await trigger(page, "exchange").tap();
        await expect(trigger(page, "exchange")).toHaveAttribute("aria-expanded", "true");
        await page.getByRole("menuitem", { name: m["toolbar.newProject"], exact: true }).tap();
        await expect(page.getByRole("dialog", { name: m["newProject.title"], exact: true })).toBeVisible();
      });
    }
  }
});

test.describe("File menu with a pen (UISET2-02)", () => {
  /** A pen press and release at the node's centre through Chromium's input domain (`pointerType: "pen"`). */
  async function penTap(page: Page, locator: Locator) {
    await locator.scrollIntoViewIfNeeded();
    const b = await box(locator);
    const at = { x: b.x + b.width / 2, y: b.y + b.height / 2, pointerType: "pen" as const };
    const cdp = await page.context().newCDPSession(page);
    await cdp.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...at });
    await cdp.send("Input.dispatchMouseEvent", { type: "mousePressed", button: "left", clickCount: 1, ...at });
    await cdp.send("Input.dispatchMouseEvent", { type: "mouseReleased", button: "left", clickCount: 1, ...at });
    await cdp.detach();
  }

  for (const { width, height } of [{ width: 400, height: 800 }, { width: 660, height: 800 }]) {
    test(`${width}x${height}: pen taps open a sibling row and run Logout below an open submenu`, async ({ page }) => {
      const unexpected = await serve(page, { sessionRequired: true });
      let logouts = 0;
      await page.route("**/api/auth/logout", (route) => { logouts += 1; return route.fulfill({ status: 200, contentType: "application/json", body: "{}" }); });
      await page.setViewportSize({ width, height });
      await page.goto("/");
      await page.evaluate(() => {
        const seen: string[] = [];
        addEventListener("pointerdown", (e) => seen.push(e.pointerType), true);
        (window as unknown as { penKinds: string[] }).penKinds = seen;
      });
      await penTap(page, page.locator(".file-menu > summary"));
      await expect(page.locator(".file-menu-content")).toBeVisible();
      await penTap(page, trigger(page, "exchange"));
      await expect(trigger(page, "exchange")).toHaveAttribute("aria-expanded", "true");
      await expect(panel(page, "exchange")).toHaveCSS("position", "static");
      await penTap(page, trigger(page, "history"));
      await expect(trigger(page, "history"), "history opens on a pen tap while exchange is open").toHaveAttribute("aria-expanded", "true");
      await expect(trigger(page, "exchange")).toHaveAttribute("aria-expanded", "false");
      await penTap(page, page.locator(".file-menu-logout"));
      await expect.poll(() => logouts, "Logout runs on a pen tap").toBe(1);
      expect(await page.evaluate(() => (window as unknown as { penKinds: string[] }).penKinds), "the browser reported pen presses").toEqual(["pen", "pen", "pen", "pen"]);
      expect(unexpected).toEqual([]);
    });
  }
});

test.describe("The File menu's first level under application zoom (UISET3-01)", () => {
  // In layout px (window / scale) 1440x601 and 1440x700 at 150% are 401 and 467 px tall, short of
  // the eight Comfortable rows with Logout, yet the window is taller than the 600 px media rule.
  // 900x800 and 660x800 at 150% are 600x533 and 440x533 layout px: narrow and short, with a
  // window wider than 650 px.
  const CASES = [
    { scale: 1.5, width: 1440, height: 601 }, { scale: 1.5, width: 1440, height: 700 },
    { scale: 1.5, width: 900, height: 800 }, { scale: 1.5, width: 660, height: 800 },
  ];
  for (const language of ["en", "de"] as const) {
    for (const { scale, width, height } of CASES) {
      test(`${language} ${scale * 100}% ${width}x${height} Comfortable with a session: every first-level row and Logout stay reachable, folded and with each submenu open`, async ({ page }) => {
        const unexpected = await serve(page, { language, uiScale: scale, density: "comfortable", sessionRequired: true });
        let logouts = 0;
        await page.route("**/api/auth/logout", (route) => { logouts += 1; return route.fulfill({ status: 200, contentType: "application/json", body: "{}" }); });
        await page.setViewportSize({ width, height });
        await startWithoutToast(page);
        await page.goto("/");
        await expect.poll(() => renderedScale(page), "the stored scale reaches the root").toBe(scale);
        await expect(page.locator("html")).toHaveAttribute("data-density", "comfortable");
        await expectNoToast(page);
        await openFile(page);
        await expect(page.locator(".file-menu-logout")).toHaveCount(1);
        await checkFirstLevel(page);
        for (const id of TRIGGERS) {
          await trigger(page, id).scrollIntoViewIfNeeded();
          await openFileSubmenu(page, id);
          await checkZoomedSubmenu(page, id, scale);
          await checkFirstLevel(page);
        }
        // Logout, the last row, scrolled to and pressed with the Help submenu still open.
        const logout = page.locator(".file-menu-logout");
        await logout.scrollIntoViewIfNeeded();
        await viewportFit(page, logout);
        await logout.click();
        await expect.poll(() => logouts, "Logout runs on a real click").toBe(1);
        expect(unexpected).toEqual([]);
      });
    }

    // 640x800 at 80% is 800x1000 layout px, yet the window is narrow: the toolbar takes its narrow
    // layout (a window rule, like every breakpoint outside the File menu) and the menu spans the
    // window. The window rule keeps every submenu inline there at every scale; a flyout beside a
    // menu that spans the window would have no room on either side anyway.
    test(`${language} 80% 640x800 Comfortable with a session: the narrow window keeps the inline layout and every entry is reachable`, async ({ page }) => {
      const unexpected = await serve(page, { language, uiScale: 0.8, density: "comfortable", sessionRequired: true });
      await page.setViewportSize({ width: 640, height: 800 });
      await startWithoutToast(page);
      await page.goto("/");
      await expect.poll(() => renderedScale(page), "the stored scale reaches the root").toBe(0.8);
      await expectNoToast(page);
      await openFile(page);
      await checkFirstLevel(page);
      for (const id of TRIGGERS) {
        await trigger(page, id).scrollIntoViewIfNeeded();
        await openFileSubmenu(page, id);
        await expect(panel(page, id), `${id} expands inline`).toHaveCSS("position", "static");
        await checkZoomedSubmenu(page, id, 0.8);
        await checkFirstLevel(page);
      }
      expect(unexpected).toEqual([]);
    });
  }
});

/**
 * Every first-level row of the open File menu (actions, submenu rows, Logout), scrolled into
 * view, lies inside the viewport and the menu box and takes a real pointer; the menu box itself
 * lies inside the viewport.
 */
async function checkFirstLevel(page: Page) {
  const content = page.locator(".file-menu-content");
  await viewportFit(page, content);
  const rows = await page.locator(".file-menu-content > [data-file-menu-item], .file-menu-content > .file-submenu > [data-file-menu-item]").all();
  expect(rows.length, "the first level has rows").toBeGreaterThanOrEqual(8);
  for (const row of rows) {
    const label = (await row.textContent())?.trim();
    await row.scrollIntoViewIfNeeded();
    await viewportFit(page, row);
    const r = await box(row);
    const menu = await box(content);
    expect(r.y + r.height, `${label}: inside the menu box`).toBeLessThanOrEqual(menu.y + menu.height + 0.5);
    expect(await paintsOnTop(row, "centre"), `${label}: the pointer reaches the row`).toBe(true);
    if (await row.isEnabled()) await row.click({ trial: true });
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), "no sideways scroll").toBeLessThanOrEqual(1);
}
