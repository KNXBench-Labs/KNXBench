/** UI-02 review: shared button roles, focus, hit targets and separators in every shipped theme. */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { intercept, openList } from "./communication-fixture";
import { controlPaint, separatorLine, type ControlRole } from "./control-roles";

// Every theme the app ships: the four built-ins and the bundled CRT pack.
const THEMES = ["porcelain", "graphite", "cupertino", "lcars", "user-modern-retro-green-crt"] as const;
// Narrow is below the 650px breakpoint (splitters hidden, panes stacked); wide is a desktop.
const WIDTHS = [600, 1440] as const;
// docs/contracts/controls-and-separators.md: no interactive control below 24px.
const FLOOR = 24;

async function expectRole(control: Locator, role: ControlRole, what: string) {
  const paint = await controlPaint(control);
  expect(paint.role, `${what}: ${JSON.stringify(paint)}`).toBe(role);
  expect(paint.height, `${what} height`).toBeGreaterThanOrEqual(FLOOR);
  expect(paint.width, `${what} width`).toBeGreaterThanOrEqual(FLOOR);
}

/** Keyboard focus (a Tab after a click marks keyboard modality) draws a visible outline. */
async function expectFocusRing(page: Page, control: Locator, what: string) {
  await control.focus();
  await page.keyboard.press("Shift+Tab");
  await page.keyboard.press("Tab");
  const ring = await control.evaluate((node) => {
    const style = getComputedStyle(node);
    return { focused: node === document.activeElement, visible: node.matches(":focus-visible"), style: style.outlineStyle, width: Number.parseFloat(style.outlineWidth), color: style.outlineColor };
  });
  expect(ring.focused, `${what} focused`).toBe(true);
  expect(ring.visible, `${what} :focus-visible`).toBe(true);
  expect(ring.style, `${what} outline`).not.toBe("none");
  expect(ring.width, `${what} outline width`).toBeGreaterThanOrEqual(2);
  expect(ring.color, `${what} outline colour`).not.toMatch(/rgba\(.*, 0\)$/);
}

async function box(control: Locator) {
  return control.evaluate((node) => ({ width: (node as HTMLElement).offsetWidth, height: (node as HTMLElement).offsetHeight }));
}

for (const theme of THEMES) for (const width of WIDTHS) {
  test(`roles, focus, hit targets and separators (${theme}, ${width}px)`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, "en", 8, theme);
    // Import findings put the explorer footer's diagnostic links on screen.
    state.tree = { ...state.tree, errors: 2, warnings: 3 };
    // One-letter labels: a row action or device link must not shrink to its glyph.
    state.nodes[0].name = "A";
    state.tree.installations[0].group_addresses[0].links[0].device_name = "A";
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await page.mouse.move(1, 1);

    // Dialog: secondary Cancel, primary Create, destructive Discard, secondary Keep.
    await page.getByRole("button", { name: en["toolbar.newProject"], exact: true }).first().click();
    const dialog = page.getByRole("dialog", { name: en["newProject.title"], exact: true });
    const cancel = dialog.getByRole("button", { name: en["newProject.cancel"], exact: true });
    const create = dialog.getByRole("button", { name: en["newProject.create"], exact: true });
    await expectRole(cancel, "secondary", "dialog Cancel");
    await expectRole(create, "primary", "dialog Create");
    await expectFocusRing(page, cancel, "dialog Cancel");
    await expectFocusRing(page, create, "dialog Create");
    await dialog.getByRole("textbox", { name: en["newProject.name"], exact: true }).fill("Fictional home");
    await cancel.click();
    await page.mouse.move(1, 1);
    const discard = dialog.locator(".project-wizard-discard").getByRole("button", { name: en["projectWizard.discard.confirm"], exact: true });
    const keep = dialog.locator(".project-wizard-discard").getByRole("button", { name: en["projectWizard.discard.keep"], exact: true });
    await expectRole(discard, "danger", "dialog Discard");
    await expectRole(keep, "secondary", "dialog Keep editing");
    await expectFocusRing(page, discard, "dialog Discard");
    await keep.click();
    await create.click();
    await page.getByRole("dialog", { name: en["projectWizard.done.title"], exact: true }).getByRole("button", { name: en["projectWizard.done.close"], exact: true }).click();

    // A notification's dismiss button is a full target around a small glyph.
    // (Creating a project announces itself in one.)
    const dismiss = page.locator(".toast:not(.toast--leaving) .toast-dismiss").first();
    await expect(dismiss).toBeVisible();
    const target = await box(dismiss);
    expect(target.width, "toast dismiss width").toBeGreaterThanOrEqual(FLOOR);
    expect(target.height, "toast dismiss height").toBeGreaterThanOrEqual(FLOOR);
    for (let round = 0; round < 20 && await page.locator(".toast:not(.toast--leaving) .toast-dismiss").count() > 0; round++) {
      await page.locator(".toast:not(.toast--leaving) .toast-dismiss").first().click();
    }

    // Toolbar: quiet text and icon buttons draw no box at rest.
    await page.mouse.move(1, 1);
    const toolbar = page.locator(".workbench-toolbar");
    const commands = toolbar.locator(".command-entry");
    await expectRole(commands, "quiet", "toolbar Commands");
    await expectRole(toolbar.getByRole("button", { name: en["toolbar.undo"], exact: true }), "quiet", "toolbar Undo icon");
    await expectFocusRing(page, commands, "toolbar Commands");
    await expectRole(toolbar.getByRole("button", { name: en["toolbar.save"], exact: true }), "primary", "toolbar Save");

    // Navigation separators at rest: splitter hairlines when wide, section rules when narrow.
    const navigation = page.locator(".workbench-panel-controls button[aria-expanded]").first();
    if (await navigation.getAttribute("aria-expanded") === "false") await navigation.click();
    for (const title of [en["workbench.workspaces"], en["workbench.projectExplorer"], en["workbench.diagnostics"]]) {
      const toggle = page.locator(".navigation-section-toggle", { hasText: title });
      if (await toggle.getAttribute("aria-expanded") === "false") await toggle.click();
    }
    await page.mouse.move(width - 5, 895);
    if (width > 650) {
      for (const name of [en["workbench.resizeNavigation"], en["workbench.resizeDiagnostics"]]) {
        const separator = page.getByRole("separator", { name, exact: true });
        await expect(separator).toBeVisible();
        const rest = await separatorLine(separator);
        expect(rest.thickness, name).toBeGreaterThanOrEqual(1);
        expect(rest.contrast, `${name} at rest: ${JSON.stringify(rest)}`).toBeGreaterThanOrEqual(1.2);
      }
    } else {
      await expect(page.getByRole("separator", { name: en["workbench.resizeNavigation"], exact: true })).toBeHidden();
      const sections = page.locator(".workbench-pane-left .navigation-section");
      expect(await sections.count()).toBeGreaterThanOrEqual(2);
      for (let index = 1; index < await sections.count(); index++) {
        const rule = await sections.nth(index).evaluate((node) => {
          const style = getComputedStyle(node);
          return { width: Number.parseFloat(style.borderTopWidth), style: style.borderTopStyle, color: style.borderTopColor };
        });
        expect(rule.width, `section ${index} rule`).toBeGreaterThanOrEqual(1);
        expect(rule.style, `section ${index} rule`).toBe("solid");
        expect(rule.color, `section ${index} rule colour`).not.toMatch(/rgba\(.*, 0\)$/);
      }
    }

    // Explorer footer: the import-finding counts are links with a full target.
    for (const link of await page.locator(".project-explorer footer .import-diagnostic-link").all()) {
      const target = await box(link);
      expect(target.height, "diagnostic link height").toBeGreaterThanOrEqual(FLOOR);
      expect(target.width, "diagnostic link width").toBeGreaterThanOrEqual(FLOOR);
    }
    await expect(page.locator(".project-explorer footer .import-diagnostic-link")).toHaveCount(2);

    // Table device links: quiet, and a full target even around a one-letter label.
    await page.getByRole("navigation", { name: en["workbench.navigation"], exact: true }).getByRole("button", { name: en["workbench.addresses"], exact: true }).click();
    for (const expand of await page.locator("[data-ga-expand-all]").all()) await expand.click();
    await page.locator(".address-table .table-select").first().click();
    const deviceLink = page.locator(".ga-workspace .address-links-panel .device-link").first();
    await expect(deviceLink).toHaveText("A");
    await page.mouse.move(1, 1);
    await expectRole(deviceLink, "quiet", "table device link");
    await expectFocusRing(page, deviceLink, "table device link");

    // Table row action: the one-letter device name in the devices table.
    await openList(page, "en");
    const rowAction = page.locator(".devices-table tbody td:nth-child(3) button").first();
    await expect(rowAction).toHaveText("A");
    await page.mouse.move(1, 1);
    await expectRole(rowAction, "quiet", "table row action");
    await expectFocusRing(page, rowAction, "table row action");
    await rowAction.click();

    // Inspector: Delete is destructive and keeps the form's spacing to the next field.
    const inspector = page.locator(".inspector").filter({ has: page.getByRole("button", { name: en["inspector.delete"], exact: true }) }).first();
    const remove = inspector.getByRole("button", { name: en["inspector.delete"], exact: true });
    await remove.scrollIntoViewIfNeeded();
    await page.mouse.move(1, 1);
    await expectRole(remove, "danger", "inspector Delete");
    await expectFocusRing(page, remove, "inspector Delete");
    const gap = await remove.evaluate((node) => {
      let next = node.nextElementSibling;
      while (next && next.getBoundingClientRect().height === 0) next = next.nextElementSibling;
      return next ? next.getBoundingClientRect().top - node.getBoundingClientRect().bottom : Number.POSITIVE_INFINITY;
    });
    expect(gap, "space between Delete and the next field (the form's 16px field rhythm)").toBeGreaterThanOrEqual(16);

    // A help tip keeps its small ring inside a full target.
    const panel = page.locator(".com-table-panel");
    await panel.getByRole("button", { name: en["comTable.flat"], exact: true }).click();
    await panel.locator('[data-object-id="1"] summary').click();
    const tip = panel.locator('[data-editor-id="1"] .help-tip-trigger').first();
    await tip.scrollIntoViewIfNeeded();
    const tipBox = await tip.evaluate((node) => {
      const ring = getComputedStyle(node, "::before");
      return { width: (node as HTMLElement).offsetWidth, height: (node as HTMLElement).offsetHeight, ring: ring.content !== "none" ? Number.parseFloat(ring.width) : 0 };
    });
    expect(tipBox.width, "help tip width").toBeGreaterThanOrEqual(FLOOR);
    expect(tipBox.height, "help tip height").toBeGreaterThanOrEqual(FLOOR);
    expect(tipBox.ring, "help tip ring stays small").toBeLessThanOrEqual(20);
    expect(tipBox.ring, "help tip ring drawn").toBeGreaterThan(0);
    if (process.env.KNX_CONTROLS_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_CONTROLS_CAPTURE_DIR, `controls-${theme}-${width}.png`) });
    expect(state.unexpected).toEqual([]);
  });
}
