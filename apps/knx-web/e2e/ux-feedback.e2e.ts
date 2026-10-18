/** Error cards stay and pause; Explorer counts open findings; motion categories apply. */
// Ported from the Codex ux-feedback branch (AP1, 219e4c3e) and narrowed to what
// main lacked: main already keeps every notification in the session log.
import { openNavigationSections } from "./disclosure-fixture";
import { expect, test } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { TOAST_EXIT_FALLBACK_MS, TOAST_MS } from "../src/toast";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

const SHOTS = process.env.UX_FEEDBACK_SHOTS;

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}: Explorer counts open log findings, error cards stay and pause`, async ({ page }) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 900 });
    await page.clock.install({ time: new Date("2026-10-14T10:00:00Z") });
    const theme = width === 400 ? "porcelain" : "graphite";
    const settings: Record<string, unknown> = { uiLanguage: language, theme, releaseChecksEnabled: false,
      achievementsEnabled: "false", onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.10" } };
    const tree: ProjectTree = { schema_version: 14, errors: 3, warnings: 2, import_report_available: true,
      can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel", installations: [] };
    const unexpected: string[] = [];
    let imports = 0;
    await page.route("**/api/**", (route) => {
      const req = route.request(); const path = new URL(req.url()).pathname; const method = req.method();
      const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
      const achievement = achievementsFixtureAnswer(method, path);
      if (achievement !== undefined) return ok(achievement);
      if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
      if (path === "/api/settings") {
        if (method === "PUT") Object.assign(settings, req.postDataJSON().settings);
        return ok({ schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings });
      }
      if (path === "/api/version") return ok({ version: "0.1.0-alpha.10" });
      if (path === "/api/legacy-password") return ok({ available: true, remembered: false, problem: null });
      if (path === "/api/project" && method === "GET") return imports ? ok(tree)
        : route.fulfill({ status: 400, contentType: "application/json", body: '{"error":"no project open"}' });
      if (path === "/api/bus/discover") return ok({ interfaces: [] });
      if (path === "/api/project/load-progress") return ok(null);
      if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(tree));
      if (path === "/api/fs/list") return ok([{ name: "synthetic.knxproj", is_dir: false }]);
      if (["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
      if (["/api/device-download/status", "/api/device-address/status"].includes(path)) {
        return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"no fixture operation"}' });
      }
      if (path === "/api/log") return ok([
        { timestamp: "2026-10-14T10:00:00Z", severity: "error", source: "import:validate", message: "Exact import error", location: "source.xml/Device", detail: "Original error detail" },
        { timestamp: "2026-10-14T10:00:01Z", severity: "warning", source: "import:unknown", message: "Exact import warning", location: "source.xml/Unknown", detail: "Original warning detail" },
      ]);
      if (path === "/api/project/import" && method === "POST") {
        imports++;
        if (imports === 1) return ok(tree);
        // Refusals answer late, as on a loaded machine: the next import must wait for the card.
        const body = JSON.stringify({ error: `Raw refused import ${imports}: ${"long/path/".repeat(35)}`, kind: "projectNotImportable" });
        return new Promise<void>((resolve) => setTimeout(() => resolve(route.fulfill({ status: 422, contentType: "application/json", body })), 1500));
      }
      unexpected.push(`${method} ${path}`); return route.abort();
    });
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("lang", language);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await page.locator(".workbench-toolbar").getByRole("button", { name: m["toolbar.settings"], exact: true }).click();
    for (const [category, value] of [["toasts", "off"], ["achievements", "subtle"], ["flow", "off"]]) {
      await page.locator(`[data-motion-category="${category}"]`).selectOption(value);
    }
    await expect.poll(() => settings.motionToasts).toBe("off");
    await expect.poll(() => settings.motionAchievements).toBe("subtle");
    await expect.poll(() => settings.motionFlow).toBe("off");
    await page.locator('[data-motion-category="flow"]').focus(); await page.keyboard.press("Escape");
    await expect(page.locator(".settings-panel")).toHaveCount(0);
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-motion-toasts", "off");
    await expect(page.locator("html")).toHaveAttribute("data-motion-achievements", "subtle");
    await expect(page.locator("html")).toHaveAttribute("data-motion-flow", "off");
    await page.locator(".workbench-toolbar").getByRole("button", { name: m["toolbar.settings"], exact: true }).click();
    await expect(page.locator('[data-motion-category="achievements"]')).toHaveValue("subtle");
    const globalMotion = page.getByRole("combobox", { name: m["settings.motionLevel"], exact: true });
    await globalMotion.selectOption("off");
    await expect(page.locator("html")).toHaveAttribute("data-motion-achievements", "off");
    await globalMotion.selectOption("standard");
    await expect(page.locator("html")).toHaveAttribute("data-motion-achievements", "subtle");
    if (SHOTS) {
      await page.locator(".motion-category-settings").scrollIntoViewIfNeeded();
      await page.screenshot({ path: `${SHOTS}/motion-${language}-${width}.png` });
    }
    await globalMotion.focus(); await page.keyboard.press("Escape");
    await expect(page.locator(".settings-panel")).toHaveCount(0);
    async function importProject() {
      await page.keyboard.press("Control+Shift+P");
      await page.getByRole("combobox").fill(m["toolbar.openProject"]);
      await page.getByRole("option").filter({ hasText: m["toolbar.openProject"] }).click();
      await page.getByRole("button", { name: "synthetic.knxproj", exact: true }).click();
    }
    await importProject();
    // KL-43 in the real stylesheet: Toasts=Off leaves the global Standard
    // level for everything else, but this card's entry runs for zero time.
    const duration = (selector: string) => page.locator(selector).first().evaluate((node) => getComputedStyle(node).animationDuration);
    expect(await duration(".toast--fun")).toBe("0s");

    // The Explorer lives in the navigation pane, which narrow layouts collapse.
    for (const severity of ["error", "warning"] as const) {
      const navigation = page.getByRole("button", { name: m["workbench.navigation"], exact: true });
      if (await navigation.getAttribute("aria-expanded") === "false") await navigation.click();
      await openNavigationSections(page);
      const count = page.locator(`.project-explorer footer .import-${severity}s button`);
      await count.focus(); await expect(count).toBeFocused(); await page.keyboard.press("Enter");
      await expect(page.locator(".log-panel-import-scope")).toBeFocused();
      await expect(page.locator(".log-panel-list")).toContainText(severity === "error" ? "Original error detail" : "Original warning detail");
      await expect(page.locator(".log-panel-list")).not.toContainText(severity === "error" ? "Original warning detail" : "Original error detail");
    }
    if (SHOTS) await page.screenshot({ path: `${SHOTS}/explorer-findings-${language}-${width}.png` });

    // Toasts=Subtle caps the card's entry at the Subtle duration.
    await page.locator(".workbench-toolbar").getByRole("button", { name: m["toolbar.settings"], exact: true }).click();
    await page.locator('[data-motion-category="toasts"]').selectOption("subtle");
    await expect.poll(() => settings.motionToasts).toBe("subtle");
    await page.locator('[data-motion-category="toasts"]').focus(); await page.keyboard.press("Escape");
    await expect(page.locator(".settings-panel")).toHaveCount(0);

    // Two refusals: the second card no longer replaces the first. Like a user, retry only
    // after the first refusal is shown; an open started during a running import is ignored.
    const cards = page.locator(".toast--error:not(.toast--leaving)");
    await importProject(); await expect(cards).toHaveCount(1);
    await importProject();
    await expect(cards).toHaveCount(2);
    await expect(cards.nth(0)).toContainText("Raw refused import 2:");
    await expect(cards.nth(1)).toContainText("Raw refused import 3:");
    // Subtle: 120 ms; the entry mirrors the exit, which runs twice as long.
    expect(await duration(".toast--error")).toBe("0.24s");
    if (SHOTS) await page.screenshot({ path: `${SHOTS}/two-errors-${language}-${width}.png` });

    // The pointer holds one card; the other runs out on its own clock. Hold
    // the bottom card: the stack sits on it, so it stays under the pointer
    // when the card above leaves (a card above would slide away, and Chromium
    // reports that mouseleave on the next rendered frame).
    await cards.nth(1).hover();
    await page.clock.runFor(3 * TOAST_MS);
    await expect(page.locator(".toast--error")).toHaveCount(1);
    await expect(page.locator(".toast--error")).toContainText("Raw refused import 3:");
    await page.mouse.move(1, 1);
    await page.clock.runFor(TOAST_MS + TOAST_EXIT_FALLBACK_MS);
    await expect(page.locator(".toast--error")).toHaveCount(0);

    expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
    expect(imports).toBe(3); expect(unexpected).toEqual([]);
  });
}
