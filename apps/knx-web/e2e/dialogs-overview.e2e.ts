/** Built dialogs and project status: labels, fit, centring, focus, report links. */
import { openNavigationSections } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import { HISTORY_FIXTURE } from "../src/projectHistory.fixture";
import { admitProjectHistory } from "../src/projectHistory";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { GITHUB_RELEASES_API } from "../src/githubReleases";
import { achievementsFixtureAnswer } from "./achievements-fixture";

type Language = "en" | "de";
type Catalogue = { readonly [K in keyof typeof en]: string };
const CAPTURE = process.env.KNX_DIALOGS_CAPTURE_DIR;
const PALETTES = ["porcelain", "graphite", "cupertino", "lcars", "user-modern-retro-green-crt"];

interface Fixture { unexpected: string[]; logRequests: number }

/** A modified, imported fictional project; every other request is refused and recorded. */
async function fixture(page: Page, language: Language, theme: string, width: number, counts: Partial<ProjectTree> = {}, releaseChecks = true): Promise<Fixture> {
  const tree = { ...admitProjectHistory(structuredClone(HISTORY_FIXTURE)).project, schema_version: 13,
    errors: 2, warnings: 7, import_report_available: true, is_modified: true, ...counts };
  const log = [
    ...Array.from({ length: 7 }, (_, i) => ({ timestamp: `2026-10-12T08:00:0${i}Z`, severity: "warning", source: i % 2 ? "import:unknown" : "import:conflict", message: `fictional warning ${i + 1}`, location: null, detail: null })),
    ...Array.from({ length: 2 }, (_, i) => ({ timestamp: `2026-10-12T08:01:0${i}Z`, severity: "error", source: "import:parse", message: `fictional error ${i + 1}`, location: null, detail: null })),
    { timestamp: "2026-10-12T08:02:00Z", severity: "info", source: "import", message: "imported fictional.knxproj", location: null, detail: null },
  ];
  const f: Fixture = { unexpected: [], logRequests: 0 };
  await page.setViewportSize({ width, height: 900 });
  // Release information comes from a synthetic feed: nothing leaves the machine.
  await page.route("https://api.github.com/**", (route) => {
    if (route.request().url() !== GITHUB_RELEASES_API) f.unexpected.push(route.request().url());
    return route.fulfill({ contentType: "application/json", headers: { "Access-Control-Allow-Origin": "*" },
      body: JSON.stringify([{ tag_name: "v0.1.0-alpha.10", draft: false, prerelease: true, published_at: "2026-10-11T10:00:00Z" }]) });
  });
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme, motionLevel: "off", releaseChecksEnabled: releaseChecks,
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.9" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.9+gfictional-build-with-a-long-suffix" });
    if (path === "/api/project" && method === "GET") return ok(tree);
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/log") { f.logRequests += 1; return ok(log); }
    if (path === "/api/fs/list") return ok([{ name: "other-fictional.knxproj", is_dir: false }]);
    // The server refuses to replace unsaved edits; nothing is created.
    if (path === "/api/project/new" && method === "POST") return route.fulfill({ status: 409, contentType: "application/json",
      body: JSON.stringify({ error: "the open project has unsaved changes", kind: "projectUnsavedChanges" }) });
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    if (["/api/device-download/status", "/api/device-address/status"].includes(path)) {
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"no fixture operation"}' });
    }
    f.unexpected.push(`${method} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"strict dialogs fixture"}' });
  });
  await page.goto("/?view=editor");
  await expect(page.locator("html")).toHaveAttribute("lang", language);
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  return f;
}

/** WCAG contrast of each element's text against its composited background. */
async function contrasts(page: Page, selector: string): Promise<{ text: string; ratio: number }[]> {
  return page.locator(selector).evaluateAll((nodes) => {
    const parse = (value: string) => {
      const m = value.match(/rgba?\(([^)]+)\)/) ?? value.match(/color\(srgb ([^)]+)\)/);
      if (!m) return [0, 0, 0, 0];
      const parts = m[1].split(/[\s,/]+/).filter(Boolean).map(Number);
      const scale = value.startsWith("color(") ? 255 : 1;
      return [parts[0] * scale, parts[1] * scale, parts[2] * scale, parts[3] ?? 1];
    };
    const background = (node: Element | null): number[] => {
      if (!node) return [255, 255, 255, 1];
      const [r, g, b, a] = parse(getComputedStyle(node).backgroundColor);
      if (a >= 0.999) return [r, g, b, 1];
      const under = background(node.parentElement);
      return [r * a + under[0] * (1 - a), g * a + under[1] * (1 - a), b * a + under[2] * (1 - a), 1];
    };
    const luminance = ([r, g, b]: number[]) => {
      const c = [r, g, b].map((v) => { const s = v / 255; return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4; });
      return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    };
    const over = ([r, g, b, a]: number[], under: number[]) => [r * a + under[0] * (1 - a), g * a + under[1] * (1 - a), b * a + under[2] * (1 - a), 1];
    return nodes.map((node) => {
      const style = getComputedStyle(node);
      const base = background(node);
      // UI-02: a primary action paints the theme's gradient; judge the text
      // against every colour stop (over the fill beneath) and keep the worst.
      // An image that is not made of colour stops cannot be judged (-1).
      let behind = [base];
      if (style.backgroundImage !== "none") {
        if (/url\(/.test(style.backgroundImage)) return { text: node.textContent ?? "", ratio: -1 };
        const stops = style.backgroundImage.match(/rgba?\([^)]+\)|color\(srgb [^)]+\)/g) ?? [];
        if (stops.length === 0) return { text: node.textContent ?? "", ratio: -1 };
        behind = stops.map((stop) => over(parse(stop), base));
      }
      const [r, g, b, a] = parse(style.color);
      const ratios = behind.map((bg) => {
        const fg = [r * a + bg[0] * (1 - a), g * a + bg[1] * (1 - a), b * a + bg[2] * (1 - a)];
        const [l1, l2] = [luminance(fg), luminance(bg)].sort((x, y) => y - x);
        return (l1 + 0.05) / (l2 + 0.05);
      });
      return { text: node.textContent ?? "", ratio: Math.min(...ratios) };
    });
  });
}

async function noHorizontalOverflow(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
}

async function openReplaceDialog(page: Page, m: Catalogue) {
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["toolbar.openProject"]);
  await page.getByRole("option").filter({ hasText: m["toolbar.openProject"] }).click();
  await page.getByRole("button", { name: "other-fictional.knxproj", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: m["replace.title"], exact: true });
  await expect(dialog).toBeVisible();
  return dialog;
}

async function openAbout(page: Page, m: Catalogue) {
  await page.locator(".file-menu > summary").click();
  await page.getByRole("button", { name: m["toolbar.about"], exact: true }).click();
  const dialog = page.getByRole("dialog", { name: m["about.title"], exact: true });
  await expect(dialog).toBeVisible();
  return dialog;
}

for (const language of ["en", "de"] as const) for (const theme of ["porcelain", "graphite"]) for (const width of [1440, 400]) {
  test(`${language} ${theme} ${width}: dialogs and project status`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const tag = `${language}-${theme}-${width}`;
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, language, theme, width);

    // Project status: three labelled sections, exact counts, real links.
    await openNavigationSections(page);
    await page.locator(".workbench-navigation").getByRole("button", { name: m["workbench.overview"], exact: true }).click();
    const dashboard = page.locator(".dashboard");
    await expect(dashboard.getByRole("heading", { level: 3 })).toHaveText([m["dashboard.section.contents"], m["dashboard.section.import"], m["dashboard.section.file"]]);
    await expect(dashboard.locator(".dashboard-import-state")).toHaveText(m["dashboard.importState.errors"]);
    const warnings = dashboard.locator(".dashboard-finding-link--warning");
    await expect(warnings).toHaveText(`${m["dashboard.showInLog"]}7`);
    await expect(dashboard.locator(".dashboard-finding-link--error")).toHaveText(`${m["dashboard.showInLog"]}2`);
    // Within each card, plain numbers and link digits share one right edge.
    const spread = await dashboard.locator(".dashboard-section").evaluateAll((sections) => sections.map((section) => {
      const edges = [...section.querySelectorAll(".dashboard-row:not(:has(.dashboard-value-note)) .dashboard-value")]
        .map((n) => Math.round(n.getBoundingClientRect().right));
      return Math.max(...edges) - Math.min(...edges);
    }));
    expect(spread.every((value) => value <= 2), JSON.stringify(spread)).toBe(true);
    await noHorizontalOverflow(page);
    if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `status-${tag}.png`), fullPage: true });

    // Keyboard: Tab reaches the warnings link with a visible focus ring, Enter opens the log.
    await page.locator(".dashboard > h2").click();
    let reached = false;
    for (let i = 0; i < 80 && !reached; i += 1) {
      await page.keyboard.press("Tab");
      reached = await warnings.evaluate((node) => node === document.activeElement);
    }
    expect(reached).toBe(true);
    const ring = await warnings.evaluate((node) => ({ visible: node.matches(":focus-visible"), style: getComputedStyle(node).outlineStyle, width: parseFloat(getComputedStyle(node).outlineWidth) }));
    expect(ring.visible).toBe(true);
    expect(ring.style).not.toBe("none");
    expect(ring.width).toBeGreaterThan(0);
    if (CAPTURE) {
      const box = (await warnings.boundingBox())!;
      await page.screenshot({ path: join(CAPTURE, `status-focus-${tag}.png`), clip: { x: box.x - 24, y: box.y - 16, width: box.width + 48, height: box.height + 32 } });
    }
    await page.keyboard.press("Enter");
    const scope = page.locator(".log-panel-import-scope");
    await expect(scope).toBeFocused();
    await expect(scope.getByRole("status")).toHaveText(m["logPanel.importScope.match"].replace("{findings}", m["logPanel.importScope.warnings.other"].replace("{count}", "7")));
    await expect(page.locator(".log-entry-message")).toHaveCount(7);
    await expect(page.locator(".log-entry-message").filter({ hasText: "fictional error" })).toHaveCount(0);
    await noHorizontalOverflow(page);
    if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `log-scope-${tag}.png`) });
    await scope.getByRole("button", { name: m["logPanel.importScope.showAll"], exact: true }).click();
    // Every server entry is back (UI notifications such as the release notice may join them).
    await expect(page.locator(".log-entry-message").filter({ hasText: /fictional (warning|error) \d|imported fictional/ })).toHaveCount(10);
    await expect(page.locator(".log-panel-import-scope")).toHaveCount(0);
    expect(f.logRequests).toBeGreaterThan(0);
    await openNavigationSections(page);
    await page.locator(".workbench-navigation").getByRole("button", { name: m["workbench.overview"], exact: true }).click();

    // Unsaved changes: three visibly labelled answers, compact panel.
    const replace = await openReplaceDialog(page, m);
    const answers = replace.locator("footer button");
    await expect(answers).toHaveText([m["replace.cancel"], m["replace.discard"], m["replace.save"]]);
    await expect(answers.first()).toBeFocused();
    for (const { text, ratio } of await contrasts(page, ".replace-confirm footer button")) {
      expect(ratio, `${text} contrast`).toBeGreaterThanOrEqual(4.5);
    }
    const panel = (await replace.boundingBox())!;
    expect(panel.x).toBeGreaterThanOrEqual(0);
    expect(panel.x + panel.width).toBeLessThanOrEqual(width);
    expect(panel.height).toBeLessThan(width === 400 ? 470 : 330);
    for (const box of await answers.evaluateAll((nodes) => nodes.map((n) => n.getBoundingClientRect().toJSON()))) {
      expect(box.right).toBeLessThanOrEqual(panel.x + panel.width);
      expect(box.bottom).toBeLessThanOrEqual(panel.y + panel.height);
    }
    expect(await replace.evaluate((node) => node.scrollWidth - node.clientWidth)).toBeLessThanOrEqual(1);
    const rows = new Set(await answers.evaluateAll((nodes) => nodes.map((n) => Math.round(n.getBoundingClientRect().top))));
    expect(rows.size).toBe(width === 400 ? 3 : 1);
    if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `unsaved-${tag}.png`) });
    await answers.first().click();
    await expect(replace).toHaveCount(0);

    // About: compact, centred, complete, with the update notice.
    const about = await openAbout(page, m);
    await about.getByRole("button", { name: m["updates.check"], exact: true }).click();
    await expect(about.locator(".release-update-panel")).toContainText("0.1.0-alpha.10");
    await expect(about.getByRole("link", { name: m["updates.releaseDetails"], exact: true })).toBeVisible();
    await expect(about).toContainText(m["about.licenceValue"]);
    await expect(about).toContainText(m["about.independence"]);
    await expect(about).toContainText(m["about.trademark"]);
    await expect(about.locator(".about-dialog-version")).toHaveText("0.1.0-alpha.9+gfictional-build-with-a-long-suffix");
    const centre = await about.evaluate((node) => {
      const box = node.getBoundingClientRect();
      const mid = box.left + box.width / 2;
      const close = node.querySelector(".about-dialog-footer button")!.getBoundingClientRect();
      const texts = [...node.querySelectorAll("h2, dt, dd, p")].map((n) => getComputedStyle(n).textAlign);
      return { closeOffset: Math.abs(close.left + close.width / 2 - mid), texts, height: box.height, right: box.right, left: box.left,
        overflow: node.scrollWidth - node.clientWidth };
    });
    expect(centre.closeOffset).toBeLessThanOrEqual(2);
    expect(new Set(centre.texts)).toEqual(new Set(["center"]));
    expect(centre.left).toBeGreaterThanOrEqual(0);
    expect(centre.right).toBeLessThanOrEqual(width);
    expect(centre.overflow).toBeLessThanOrEqual(1);
    expect(centre.height).toBeLessThan(width === 400 ? 760 : 560);
    if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `about-${tag}.png`) });
    await about.getByRole("button", { name: m["about.close"], exact: true }).click();
    await expect(about).toHaveCount(0);

    expect(errors).toEqual([]);
    expect(f.unexpected).toEqual([]);
  });
}

test("every shipped palette keeps the unsaved-changes answers legible", async ({ page }) => {
  for (const theme of PALETTES) {
    const f = await fixture(page, "en", theme, 1440);
    const dialog = await openReplaceDialog(page, en);
    await expect(dialog.locator("footer button")).toHaveText([en["replace.cancel"], en["replace.discard"], en["replace.save"]]);
    for (const { text, ratio } of await contrasts(page, ".replace-confirm footer button")) {
      expect(ratio, `${theme}: ${text}`).toBeGreaterThanOrEqual(4.5);
    }
    expect(f.unexpected).toEqual([]);
    await page.unrouteAll({ behavior: "ignoreErrors" });
  }
});

test("zero findings and a missing report are stated, not linked", async ({ page }) => {
  const f = await fixture(page, "en", "porcelain", 1440, { errors: 0, warnings: 0, import_report_available: false });
  await openNavigationSections(page);
  await page.locator(".workbench-navigation").getByRole("button", { name: en["workbench.overview"], exact: true }).click();
  const dashboard = page.locator(".dashboard");
  await expect(dashboard.locator(".dashboard-import-state")).toHaveText(en["dashboard.importState.unavailable"]);
  await expect(dashboard.locator('[role="note"]')).toContainText(en["diagnostics.importReportUnavailable"]);
  await expect(dashboard.locator(".dashboard-finding-link")).toHaveCount(0);
  await expect(dashboard.locator(".dashboard-issues dd")).toHaveText(["0", "0"]);
  if (CAPTURE) await page.screenshot({ path: join(CAPTURE, "status-unavailable-en-porcelain-1440.png") });
  expect(f.unexpected).toEqual([]);
});

for (const [language, theme, width] of [["en", "porcelain", 1440], ["de", "graphite", 1440], ["de", "porcelain", 400], ["en", "graphite", 400]] as const) {
  test(`${language} ${theme} ${width}: new-project conflict answers share the same roles`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    // No startup release notice: its toast would cover the narrow frame.
    const f = await fixture(page, language, theme, width, {}, false);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.newProject"]);
    await page.getByRole("option").filter({ hasText: m["toolbar.newProject"] }).first().click();
    await page.getByRole("button", { name: m["newProject.create"], exact: true }).click();
    const conflict = page.locator(".new-project-conflict");
    await expect(conflict).toBeVisible();
    await expect(conflict.locator("button")).toHaveText([m["newProject.conflictKeep"], m["newProject.conflictDiscard"], m["newProject.conflictSave"]]);
    for (const { text, ratio } of await contrasts(page, ".new-project-conflict button")) {
      expect(ratio, `${text} contrast`).toBeGreaterThanOrEqual(4.5);
    }
    await expect(page.locator(".toast--update")).toHaveCount(0);
    const save = conflict.locator("button").last();
    await save.scrollIntoViewIfNeeded();
    expect(await save.evaluate((node) => { const r = node.getBoundingClientRect(); return document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2) === node; })).toBe(true);
    await noHorizontalOverflow(page);
    if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `new-project-conflict-${language}-${theme}-${width}.png`) });
    expect(f.unexpected).toEqual([]);
  });
}
