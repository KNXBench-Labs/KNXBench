/** UI-03: File submenus open right, flip, expand inline when narrow, keep focus, honour motion. */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { openFileSubmenu } from "./file-menu";
import { HISTORY_FIXTURE } from "../src/projectHistory.fixture";
import { admitProjectHistory } from "../src/projectHistory";

const THEMES = ["porcelain", "graphite", "cupertino", "lcars", "user-modern-retro-green-crt"] as const;
type Catalogue = { readonly [K in keyof typeof en]: string };
const TRIGGERS = ["exchange", "history", "help"] as const;

async function serve(page: Page, { theme = "porcelain", motionLevel = "standard", language = "en", sessionRequired = false }:
  { theme?: string; motionLevel?: string; language?: "en" | "de"; sessionRequired?: boolean } = {}) {
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
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.14" } } });
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

  test("hover opens after a short intent delay and a diagonal move into the panel does not close it", async ({ page }) => {
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
