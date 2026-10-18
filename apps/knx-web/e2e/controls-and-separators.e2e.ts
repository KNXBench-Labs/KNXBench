/** UI-02: compact button variants, separators visible at rest, no routine instance source. */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { dismissToasts, intercept, openList, openProject as openCommunicationProject } from "./communication-fixture";

const THEMES = ["porcelain", "graphite", "cupertino", "lcars", "user-modern-retro-green-crt"] as const;
type Catalogue = { readonly [K in keyof typeof en]: string };

// Forty installations: an explorer taller than the column, as in workbench-splitters.e2e.ts.
const tree = {
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "controls-fixture", snapshot_revision: 1,
  installations: Array.from({ length: 40 }, (_, i) => ({
    id: i + 1, name: `Installation ${i + 1}`, topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [],
  })),
};

interface Fixture { unexpected: string[]; releaseCreate: () => void }

async function serve(page: Page, { theme = "porcelain", motionLevel = "standard", language = "en", holdCreate = false }:
  { theme?: string; motionLevel?: string; language?: "en" | "de"; holdCreate?: boolean } = {}): Promise<Fixture> {
  const unexpected: string[] = [];
  let projectOpen = false;
  let releaseCreate = () => {};
  const created = holdCreate ? new Promise<void>((resolve) => { releaseCreate = resolve; }) : Promise.resolve();
  await page.route("https://api.github.com/**", (route) => { unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme, motionLevel, releaseChecksEnabled: false, achievementsEnabled: "false",
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.14" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/project" && method === "GET") {
      return projectOpen ? ok(tree) : route.fulfill({ status: 400, contentType: "application/json", body: '{"error":"no project open"}' });
    }
    if (path === "/api/project/new" && method === "POST") { await created; projectOpen = true; return ok(tree); }
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    unexpected.push(`${method} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"controls fixture only"}' });
  });
  return { unexpected, releaseCreate: () => releaseCreate() };
}

async function createProject(page: Page, m: Catalogue) {
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
}

async function unfold(page: Page, title: string) {
  const toggle = page.locator(".navigation-section-toggle", { hasText: title });
  if (await toggle.getAttribute("aria-expanded") === "false") await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
}

async function openNavigation(page: Page) {
  const navigation = page.locator(".workbench-panel-controls button[aria-expanded]").first();
  if (await navigation.getAttribute("aria-expanded") === "false") await navigation.click();
}

/** WCAG-style contrast between a separator's painted line and the pane colour behind it. */
async function separatorLine(separator: Locator) {
  return separator.evaluate((node) => {
    const probe = document.createElement("div");
    document.body.append(probe);
    const rgb = (value: string) => {
      probe.style.color = value;
      const m = getComputedStyle(probe).color.match(/[\d.]+/g)!.map(Number);
      return { r: m[0], g: m[1], b: m[2], a: m[3] ?? 1 };
    };
    const opaque = (element: Element | null): { r: number; g: number; b: number } => {
      if (!element) return { r: 255, g: 255, b: 255 };
      const c = rgb(getComputedStyle(element).backgroundColor);
      if (c.a >= 0.999) return c;
      const under = opaque(element.parentElement);
      return { r: c.r * c.a + under.r * (1 - c.a), g: c.g * c.a + under.g * (1 - c.a), b: c.b * c.a + under.b * (1 - c.a) };
    };
    const line = getComputedStyle(node, "::before");
    const paint = rgb(line.backgroundColor);
    const behind = opaque(node.parentElement);
    const area = rgb(getComputedStyle(node).backgroundColor);
    probe.remove();
    const mixed = { r: paint.r * paint.a + behind.r * (1 - paint.a), g: paint.g * paint.a + behind.g * (1 - paint.a), b: paint.b * paint.a + behind.b * (1 - paint.a) };
    const lum = ({ r, g, b }: { r: number; g: number; b: number }) => {
      const c = [r, g, b].map((v) => { const s = v / 255; return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4; });
      return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    };
    const [hi, lo] = [lum(mixed), lum(behind)].sort((x, y) => y - x);
    const box = node.getBoundingClientRect();
    const own = Number.parseFloat(line.height === "auto" ? "0" : line.height);
    return { contrast: (hi + 0.05) / (lo + 0.05), thickness: Math.min(own, box.height), content: line.content, paint: line.backgroundColor, area: area.a, width: box.width };
  });
}

for (const theme of THEMES) {
  test(`navigation separators are visible at rest and highlighted on hover (${theme})`, async ({ page }) => {
    const f = await serve(page, { theme });
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await openNavigation(page);
    // Without a project there is no splitter; the two sections still read as separate blocks.
    const sections = page.locator(".workbench-pane-left .navigation-section");
    await expect(sections).toHaveCount(2);
    const ruled = await sections.nth(1).evaluate((node) => {
      const style = getComputedStyle(node);
      return { width: Number.parseFloat(style.borderTopWidth), style: style.borderTopStyle, color: style.borderTopColor };
    });
    expect(ruled.width).toBeGreaterThanOrEqual(1);
    expect(ruled.style).toBe("solid");
    await createProject(page, en);
    await unfold(page, en["workbench.workspaces"]);
    await unfold(page, en["workbench.projectExplorer"]);
    await unfold(page, en["workbench.diagnostics"]);
    // The pointer rests on the canvas, far from every separator.
    await page.mouse.move(900, 450);
    for (const name of [en["workbench.resizeNavigation"], en["workbench.resizeDiagnostics"]]) {
      const separator = page.getByRole("separator", { name, exact: true });
      await expect(separator).toBeVisible();
      const rest = await separatorLine(separator);
      expect(rest.content, name).not.toBe("none");
      expect(rest.thickness, name).toBeGreaterThanOrEqual(1);
      expect(rest.contrast, `${name} contrast at rest: ${JSON.stringify(rest)}`).toBeGreaterThanOrEqual(1.2);
      // A full-width rule, not a stub: it spans the column like the pane edge does.
      expect(rest.width, name).toBeGreaterThan(150);
      const box = (await separator.boundingBox())!;
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await expect.poll(async () => (await separatorLine(separator)).paint, { message: `${name} hover` }).not.toBe(rest.paint);
      expect((await separatorLine(separator)).area, `${name} hover band`).toBeGreaterThan(0);
      await page.mouse.move(900, 450);
      await expect.poll(async () => (await separatorLine(separator)).paint, { message: `${name} back at rest` }).toBe(rest.paint);
      // Keyboard focus is announced by the same highlight plus the focus ring.
      // A key press after focusing marks keyboard modality; down and up again leaves the height as it was.
      await separator.focus();
      await page.keyboard.press("ArrowDown");
      await page.keyboard.press("ArrowUp");
      const focused = await separator.evaluate((node) => ({ visible: node.matches(":focus-visible"), outline: getComputedStyle(node).outlineStyle }));
      expect(focused.visible).toBe(true);
      expect(focused.outline).not.toBe("none");
      await separator.blur();
    }
    // The pane's vertical edge is drawn at rest too.
    const edge = await page.locator(".workbench-pane-left").evaluate((node) => Number.parseFloat(getComputedStyle(node).borderRightWidth));
    expect(edge).toBeGreaterThanOrEqual(1);
    expect(f.unexpected).toEqual([]);
  });
}

test("the first resize right after unfolding moves by the dragged distance, by pointer and by keyboard", async ({ page }) => {
  const f = await serve(page);
  await page.setViewportSize({ width: 1400, height: 900 });
  await page.goto("/");
  await openNavigation(page);
  await createProject(page, en);
  // Sections start folded; unfold one and drag at once, before anything else settles.
  await unfold(page, en["workbench.workspaces"]);
  const block = page.getByRole("navigation", { name: en["workbench.navigation"], exact: true });
  const separator = page.getByRole("separator", { name: en["workbench.resizeNavigation"], exact: true });
  const before = (await block.boundingBox())!.height;
  const box = (await separator.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2 + 40, { steps: 6 });
  await page.mouse.up();
  expect((await block.boundingBox())!.height).toBeCloseTo(before + 40, 0);
  expect(Number(await separator.getAttribute("aria-valuenow"))).toBeCloseTo((await block.boundingBox())!.height, 0);

  await unfold(page, en["workbench.diagnostics"]);
  const diagnostics = page.getByRole("navigation", { name: en["toolbar.busMonitor"], exact: true });
  const lower = page.getByRole("separator", { name: en["workbench.resizeDiagnostics"], exact: true });
  const start = (await diagnostics.boundingBox())!.height;
  await lower.focus();
  await page.keyboard.press("ArrowUp");
  expect((await diagnostics.boundingBox())!.height).toBeCloseTo(start + 16, 0);
  // Folding and unfolding keeps the chosen height (collapse state is the section's own).
  const toggle = page.locator(".navigation-section-toggle", { hasText: en["workbench.diagnostics"] });
  await toggle.click();
  await expect(diagnostics).toBeHidden();
  await toggle.click();
  expect((await diagnostics.boundingBox())!.height).toBeCloseTo(start + 16, 0);
  expect(f.unexpected).toEqual([]);
});

/** Layout geometry (unaffected by transforms) and painted state of every action in the row. */
async function rowState(actions: Locator) {
  return actions.evaluateAll((nodes) => nodes.map((node) => {
    const element = node as HTMLElement;
    const style = getComputedStyle(element);
    return {
      text: element.textContent ?? "", left: element.offsetLeft, top: element.offsetTop, width: element.offsetWidth, height: element.offsetHeight,
      background: `${style.backgroundColor} ${style.backgroundImage}`, border: style.borderTopColor, transform: style.transform,
      cursor: style.cursor, opacity: Number(style.opacity), color: style.color,
    };
  }));
}

/** Lets the controls' own colour/scale transitions finish (bounded: they last at most 120 ms). */
const settled = (page: Page) => page.evaluate(() => Promise.race([
  Promise.allSettled(document.getAnimations().filter((animation) => animation instanceof CSSTransition).map((animation) => animation.finished)),
  new Promise((resolve) => setTimeout(resolve, 1000)),
]));

for (const theme of ["porcelain", "graphite"] as const) for (const width of [1440, 400]) for (const language of ["en", "de"] as const) {
  test(`button variants: rest, hover, press, focus, disabled and pending without layout shift (${theme}, ${width}px, ${language})`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const f = await serve(page, { theme, language, holdCreate: true });
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
    const dialog = page.getByRole("dialog", { name: m["newProject.title"], exact: true });
    const actions = dialog.locator(".new-project-actions button");
    const cancel = dialog.getByRole("button", { name: m["newProject.cancel"], exact: true });
    const back = dialog.getByRole("button", { name: m["projectWizard.back"], exact: true });
    const next = dialog.getByRole("button", { name: m["projectWizard.next"], exact: true });
    const create = dialog.getByRole("button", { name: m["newProject.create"], exact: true });
    await expect(back).toBeDisabled();
    const rest = await rowState(actions);
    const surface = await page.evaluate(() => { const p = document.createElement("i"); p.style.color = "var(--knx-surface)"; document.body.append(p); const c = getComputedStyle(p).color; p.remove(); return c; });
    for (const action of rest) {
      // Compact, yet never below the density's control height (and never below 24px).
      expect(action.height, action.text).toBeGreaterThanOrEqual(Math.max(24, 32));
      expect(action.height, action.text).toBeLessThanOrEqual(40);
      expect(action.transform, action.text).toBe("none");
    }
    const [cancelRest, backRest, nextRest, createRest] = rest;
    // Secondary is the default; primary is the one accent-filled action.
    expect(cancelRest.background.startsWith(surface), JSON.stringify(cancelRest)).toBe(true);
    expect(nextRest.background).toBe(cancelRest.background);
    expect(createRest.background).not.toBe(cancelRest.background);
    expect(backRest.cursor).toBe("not-allowed");
    expect(backRest.opacity).toBeLessThan(1);
    // Labels stay whole: no button clips its text.
    for (const clipped of await actions.evaluateAll((nodes) => nodes.map((node) => node.scrollWidth - node.clientWidth))) expect(clipped).toBeLessThanOrEqual(1);

    for (const target of [cancel, create]) {
      const index = target === cancel ? 0 : 3;
      await target.hover();
      await settled(page);
      const hovered = await rowState(actions);
      expect(hovered[index].background, `${rest[index].text} hover`).not.toBe(rest[index].background);
      expect(hovered[index].transform, `${rest[index].text} hover never scales`).toBe("none");
      expect(hovered.map(({ left, top, width: w, height }) => [left, top, w, height]))
        .toEqual(rest.map(({ left, top, width: w, height }) => [left, top, w, height]));
      await page.mouse.down();
      await settled(page);
      const pressed = await rowState(actions);
      expect(pressed[index].transform, `${rest[index].text} press`).not.toBe("none");
      expect(pressed[index].background, `${rest[index].text} press`).not.toBe(hovered[index].background);
      expect(pressed.map(({ left, top, width: w, height }) => [left, top, w, height]))
        .toEqual(rest.map(({ left, top, width: w, height }) => [left, top, w, height]));
      await page.mouse.move(1, 1);
      await page.mouse.up();
    }
    // Disabled: hovering changes nothing.
    await back.hover({ force: true });
    await settled(page);
    expect((await rowState(actions))[1].background).toBe(backRest.background);

    // Keyboard focus shows a ring; the ring is an outline, so nothing moves.
    await page.mouse.move(1, 1);
    await cancel.focus();
    await page.keyboard.press("Shift+Tab");
    await page.keyboard.press("Tab");
    const ring = await cancel.evaluate((node) => ({ visible: node.matches(":focus-visible"), style: getComputedStyle(node).outlineStyle, width: Number.parseFloat(getComputedStyle(node).outlineWidth) }));
    expect(ring).toEqual({ visible: true, style: "solid", width: 2 });

    // Destructive: the discard confirmation reads as a warning, not as another primary.
    await dialog.getByRole("textbox", { name: m["newProject.name"], exact: true }).fill("Fictional home");
    await cancel.click();
    const discard = dialog.locator(".project-wizard-discard").getByRole("button", { name: m["projectWizard.discard.confirm"], exact: true });
    const keep = dialog.locator(".project-wizard-discard").getByRole("button", { name: m["projectWizard.discard.keep"], exact: true });
    const error = await page.evaluate(() => { const p = document.createElement("i"); p.style.color = "var(--knx-error-color)"; document.body.append(p); const c = getComputedStyle(p).color; p.remove(); return c; });
    const [keepState, discardState] = await rowState(dialog.locator(".project-wizard-discard button"));
    expect(keepState.text).toBe(m["projectWizard.discard.keep"]);
    expect(discardState.color).toBe(error);
    expect(discardState.background).not.toBe(createRest.background);
    await keep.click();

    // Pending: the busy create keeps its place and says so.
    await create.click();
    const busy = dialog.getByRole("button", { name: m["newProject.creating"], exact: true });
    await expect(busy).toHaveAttribute("aria-busy", "true");
    const [, , , busyState] = await rowState(actions);
    expect(busyState.cursor).toBe("progress");
    // The row is end-aligned, so the busy label keeps the right edge and the line.
    expect([busyState.left + busyState.width, busyState.top]).toEqual([createRest.left + createRest.width, createRest.top]);
    f.releaseCreate();
    await expect(page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true })).toBeVisible();
    expect(f.unexpected).toEqual([]);
  });
}

for (const setting of ["motion off", "reduced motion"] as const) {
  test(`pressing a button does not animate with ${setting}`, async ({ page }) => {
    const f = await serve(page, { motionLevel: setting === "motion off" ? "off" : "standard" });
    if (setting === "reduced motion") await page.emulateMedia({ reducedMotion: "reduce" });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: en["toolbar.newProject"], exact: true }).first().click();
    const dialog = page.getByRole("dialog", { name: en["newProject.title"], exact: true });
    const cancel = dialog.getByRole("button", { name: en["newProject.cancel"], exact: true });
    const before = await rowState(cancel);
    await cancel.hover();
    await page.mouse.down();
    const pressed = await rowState(cancel);
    expect(pressed[0].transform).toBe("none");
    // Feedback stays, as an instant colour change.
    expect(pressed[0].background).not.toBe(before[0].background);
    expect(await cancel.evaluate((node) => getComputedStyle(node).transitionDuration.split(",").every((d) => Number.parseFloat(d) === 0))).toBe(true);
    await page.mouse.move(1, 1);
    await page.mouse.up();
    expect(f.unexpected).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) {
  test(`${language}: the routine device-instance source is not shown; the technical reference keeps it`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    await page.setViewportSize({ width: 1440, height: 1000 });
    const state = await intercept(page, language, 8, "graphite");
    state.objects[0].dpt_layer = "Instance";
    state.objects[0].description_layer = "Instance";
    state.objects[3].dpt_layer = "UserEdit";
    await openCommunicationProject(page, language);
    await dismissToasts(page, language);
    await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    await panel.getByRole("button", { name: m["comTable.flat"], exact: true }).click();
    const instance = m["valueSource.label"].replace("{source}", m["valueSource.instance"]);
    await panel.locator('[data-object-id="1"] summary').click();
    const editor = panel.locator('[data-editor-id="1"]');
    await expect(editor.locator(".dpt-field")).toBeVisible();
    await expect(editor.locator(".inspector-field-heading .provenance-badge")).toHaveCount(0);
    await expect(editor.getByText(instance, { exact: true })).toBeHidden();
    // Deliberately opened: the technical reference names the source layer.
    await editor.locator(".dpt-reference summary").click();
    const recorded = editor.locator(".dpt-reference .provenance-badge");
    await expect(recorded).toBeVisible();
    await expect(recorded).toHaveText(instance);
    await expect(recorded).toHaveAttribute("data-source-layer", "Instance");
    // Other sources keep their heading badge.
    await panel.locator('[data-object-id="4"] summary').click();
    await expect(panel.locator('[data-editor-id="4"] .dpt-field .inspector-field-heading .provenance-badge'))
      .toHaveText(m["valueSource.label"].replace("{source}", m["valueSource.userEdit"]));
    // The served object, i.e. the API data, still carries both layers; nothing was written.
    expect(state.objects[0].dpt_layer).toBe("Instance");
    expect(state.objects[0].description_layer).toBe("Instance");
    expect(state.requests.filter((request) => request.startsWith("POST /api/com-object"))).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });
}

for (const theme of ["porcelain", "graphite"] as const) {
  test(`the toolbar's Save stays an accent primary under hover and press (${theme})`, async ({ page }) => {
    const f = await serve(page, { theme });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await createProject(page, en);
    const save = page.locator(".workbench-toolbar").getByRole("button", { name: en["toolbar.save"], exact: true });
    await expect(save).toBeEnabled();
    const accent = await page.evaluate(() => { const p = document.createElement("i"); p.style.color = "var(--knx-accent)"; document.body.append(p); const c = getComputedStyle(p).color; p.remove(); return c; });
    await page.mouse.move(1, 1);
    const [rest] = await rowState(save);
    await save.hover();
    await settled(page);
    const [hovered] = await rowState(save);
    await page.mouse.down();
    await settled(page);
    const [pressed] = await rowState(save);
    await page.mouse.move(1, 1);
    await page.mouse.up();
    // Before UI-02 the toolbar's quiet hover repainted Save pale under its white label.
    for (const state of [rest, hovered, pressed]) expect(state.background.startsWith(accent), JSON.stringify(state)).toBe(true);
    expect(hovered.background).not.toBe(rest.background);
    expect(pressed.background).not.toBe(hovered.background);
    expect(f.unexpected).toEqual([]);
  });
}
