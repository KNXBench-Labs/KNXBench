/** Tests catalogue installation targets and native keyboard selection across column preferences. */
import { test, expect, type Page } from "@playwright/test";

const item = {
  id: "cat-1", manufacturerId: "M-1", name: "Actuator", number: null, visibleDescription: null,
  productRefId: "P-1", hardware2programRefId: "HP-1",
};
const labels = {
  en: { installation: "Installation", placeholder: "Choose an installation" },
  de: { installation: "Anlage", placeholder: "Anlage wählen" },
};

async function mock(page: Page) {
  const posts: Record<string, unknown>[] = [];
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === "/api/catalog/manufacturers") return route.fulfill({ contentType: "application/json", body: "[]" });
    if (path === "/api/catalog/items") return route.fulfill({ contentType: "application/json", body: JSON.stringify([item]) });
    if (path === "/api/devices" && request.method() === "POST") {
      posts.push(request.postDataJSON());
      return route.fulfill({ contentType: "application/json",
        body: JSON.stringify({ tree: { installations: [] }, diagnostics: [], replayed: false }) });
    }
    return route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "fixture only" }) });
  });
  return posts;
}

for (const language of ["en", "de"] as const) {
  test(`${language} several installations: no implicit target, keys choose and create, id travels as installationId`, async ({ page }) => {
    await page.setViewportSize({ width: 360, height: 800 });
    const posts = await mock(page);
    await page.goto(`/e2e/catalog-installation-fixture.html?installations=2&lang=${language}`);
    // Focus is set programmatically; every selection and activation below is a real key press.
    await page.locator(".catalog-select").first().focus();
    await page.keyboard.press("Enter");
    const target = page.getByLabel(labels[language].installation, { exact: true });
    await expect(target).toBeVisible();
    await expect(target.locator("option").first()).toHaveText(labels[language].placeholder);
    const create = page.locator(".catalog-create-row button").first();
    await expect(create).toBeDisabled();
    await page.getByRole("textbox", { name: "Device name" }).or(page.getByRole("textbox", { name: "Gerätename" })).press("Enter");
    expect(posts).toEqual([]);
    await target.focus();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(target).toHaveValue("1");
    await expect(create).toBeEnabled();
    await create.focus();
    await page.keyboard.press(language === "en" ? "Enter" : "Space");
    await expect.poll(() => posts.length).toBe(1);
    expect(posts[0]).toMatchObject({ catalogItemId: "cat-1", installationId: 1 });
    expect(posts[0]).not.toHaveProperty("lineId");
  });
}

test("a single installation keeps the request body without installationId", async ({ page }) => {
  const posts = await mock(page);
  await page.goto("/e2e/catalog-installation-fixture.html");
  await page.locator(".search-result").first().click();
  await expect(page.getByLabel("Installation", { exact: true })).toHaveCount(0);
  await page.locator(".catalog-create-row button").first().click();
  await expect.poll(() => posts.length).toBe(1);
  expect(posts[0]).not.toHaveProperty("installationId");
  expect(posts[0]).not.toHaveProperty("lineId");
});

for (const theme of ["porcelain", "graphite"] as const) {
  for (const density of ["comfortable", "compact"] as const) {
    test(`a long installation name fits at 360px in ${theme}/${density}`, async ({ page }) => {
      await page.setViewportSize({ width: 360, height: 800 });
      await mock(page);
      await page.goto(`/e2e/catalog-installation-fixture.html?installations=2&long=1&theme=${theme}&density=${density}`);
      await page.locator(".search-result").first().click();
      await expect(page.getByLabel("Installation", { exact: true })).toBeVisible();
      await page.evaluate(() => document.fonts.ready);
      const hint = await page.locator("#catalog-installation-hint").boundingBox();
      const uniqueOption = await page.locator('input[name="uniqueNames"]').locator("..").boundingBox();
      expect(hint).not.toBeNull();
      expect(uniqueOption).not.toBeNull();
      expect(uniqueOption!.y).toBeGreaterThanOrEqual(hint!.y + hint!.height);
      const widths = await page.evaluate(() => {
        const workspace = document.querySelector<HTMLElement>(".catalog-workspace")!;
        const root = document.documentElement;
        return { workspace: [workspace.scrollWidth, workspace.clientWidth], page: [root.scrollWidth, root.clientWidth] };
      });
      expect(widths.workspace[0]).toBeLessThanOrEqual(widths.workspace[1]);
      expect(widths.page[0]).toBeLessThanOrEqual(widths.page[1]);
    });
  }
}

// These cases mount production CatalogBrowser and its real table/chooser/preferences.
const selectionItems = ["Alpha Relay", "Beta Sensor", "Gamma Dimmer"].map((name, index) => ({
  ...item, id: `selection-${index + 1}`, name, number: `ORD-${index + 1}`,
  nameLanguage: null, visibleDescriptionLanguage: null, sourceLanguage: null,
}));
const visibilityKey = "knxbench:table-column-visibility:v1:catalog-results";

async function selectionFixture(page: Page) {
  const unexpected: string[] = [];
  const errors: string[] = [];
  const posts: unknown[] = [];
  const origin = new URL(test.info().project.use.baseURL!).origin;
  page.on("pageerror", error => errors.push(error.message));
  await page.route("**/*", route => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.origin !== origin) { unexpected.push(request.url()); return route.abort(); }
    if (!url.pathname.startsWith("/api/")) return route.continue();
    if (request.method() === "GET" && url.pathname === "/api/catalog/manufacturers")
      return route.fulfill({ json: [{ id: "M-1", name: "Fictional maker" }] });
    if (request.method() === "GET" && url.pathname === "/api/catalog/items")
      return route.fulfill({ json: selectionItems });
    if (request.method() === "POST" && url.pathname === "/api/bus/discover")
      return route.fulfill({ json: { interfaces: [] } });
    if (request.method() === "POST" && url.pathname === "/api/devices") {
      posts.push(request.postDataJSON()); return route.abort("connectionreset");
    }
    unexpected.push(`${request.method()} ${url.pathname}`);
    return route.fulfill({ status: 404, json: { error: "fixture only" } });
  });
  return { unexpected, errors, posts };
}

async function tabToSelection(page: Page, index: number) {
  const search = page.locator('input[type="search"]');
  await search.click();
  const target = page.locator(".catalog-select").nth(index);
  let reached = false;
  for (let step = 0; step < 50; step++) {
    await page.keyboard.press("Tab");
    if (await target.evaluate(node => document.activeElement === node)) { reached = true; break; }
  }
  expect(reached, "bounded genuine Tab traversal reaches the intended row button").toBe(true);
  await expect(target).toBeFocused();
  const focus = await target.evaluate(node => {
    const style = getComputedStyle(node);
    return { visible: node.matches(":focus-visible"), outline: style.outlineStyle,
      width: parseFloat(style.outlineWidth), color: style.outlineColor };
  });
  expect(focus.visible).toBe(true);
  expect(focus.outline).not.toBe("none");
  expect(focus.width).toBeGreaterThan(0);
  expect(focus.color).not.toBe("rgba(0, 0, 0, 0)");
  return target;
}

for (const language of ["en", "de"] as const) {
  for (const [theme, width] of [["porcelain", 1440], ["graphite", 400]] as const) {
    test(`${language} catalogue direct selection survives hidden Product at ${width}px/${theme}`, async ({ page }) => {
      const f = await selectionFixture(page);
      await page.setViewportSize({ width, height: 1100 });
      await page.goto(`/e2e/catalog-installation-fixture.html?lang=${language}&theme=${theme}`);
      const buttons = page.locator(".catalog-select");
      const rows = page.locator(".catalog-table tbody tr");
      const selected = (id: number) => page.locator(`tr[data-item-id="selection-${id}"].selected`);
      await expect(buttons).toHaveCount(3);
      for (let i = 0; i < 3; i++) {
        await expect(rows.nth(i).getByRole("button")).toHaveCount(1);
        await expect(buttons.nth(i)).toHaveAccessibleName(selectionItems[i].name);
      }
      await tabToSelection(page, 0); await page.keyboard.press("Enter");
      await expect(selected(1)).toBeVisible();
      await rows.nth(1).locator("td").nth(2).click();
      await expect(selected(2)).toBeVisible();
      await page.locator(".table-column-chooser summary").click();
      await page.getByRole("checkbox", { name: language === "en" ? "Product" : "Produkt", exact: true }).uncheck();
      await page.locator(".table-column-chooser summary").click();
      const raw = await page.evaluate(key => localStorage.getItem(key), visibilityKey);
      expect(JSON.parse(raw!)).toMatchObject({ product: false, manufacturer: true });
      await expect(page.locator('th[data-column="product"]')).toHaveCount(0);
      await expect(buttons).toHaveCount(3);
      for (let i = 0; i < 3; i++) {
        await expect(rows.nth(i).getByRole("button")).toHaveCount(1);
        await expect(buttons.nth(i)).toHaveText("Fictional maker");
        await expect(buttons.nth(i)).toHaveAccessibleName(`${selectionItems[i].name} — Fictional maker`);
      }
      await tabToSelection(page, 0);
      await page.screenshot({ path: test.info().outputPath("hidden-product-tab-focus.png"), fullPage: true });
      await buttons.first().screenshot({ path: test.info().outputPath("selection-focus.png") });
      await page.keyboard.press("Enter");
      await expect(selected(1)).toBeVisible();
      await expect(buttons.first()).toHaveAttribute("aria-pressed", "true");
      await tabToSelection(page, 1); await page.keyboard.press("Space");
      await expect(selected(2)).toBeVisible();
      await expect(buttons.nth(1)).toHaveAttribute("aria-pressed", "true");
      await expect(buttons.first()).toHaveAttribute("aria-pressed", "false");
      await expect(page.locator(".catalog-create-row input").first()).toHaveValue("Beta Sensor");
      await rows.nth(2).locator("td").nth(1).click(); await expect(selected(3)).toBeVisible();
      const search = page.locator('input[type="search"]');
      await search.click(); await page.keyboard.press("ArrowDown");
      await page.keyboard.press("Enter");
      await expect(selected(2)).toBeVisible(); await expect(search).toBeFocused();
      // Sorting/resizing remain available with the moved button.
      await page.locator('th[data-column="number"] .catalog-sort').click();
      await expect(page.locator('th[data-column="number"]')).toHaveAttribute("aria-sort", "ascending");
      const handle = page.locator('th[data-column="number"] [role="separator"]');
      const previousWidth = Number(await handle.getAttribute("aria-valuenow"));
      await handle.focus(); await page.keyboard.press("ArrowRight");
      await expect.poll(async () => Number(await handle.getAttribute("aria-valuenow"))).toBeGreaterThan(previousWidth);
      await page.reload();
      await expect(buttons).toHaveCount(3);
      await expect(page.locator('th[data-column="product"]')).toHaveCount(0);
      expect(await page.evaluate(key => localStorage.getItem(key), visibilityKey)).toBe(raw);
      await tabToSelection(page, 2); await page.keyboard.press("Space");
      await expect(selected(3)).toBeVisible();
      expect(f.posts).toEqual([]); expect(f.unexpected).toEqual([]); expect(f.errors).toEqual([]);
    });
  }
}

test("hidden Product selection stays locked after an unresolved batch outcome", async ({ page }) => {
  const f = await selectionFixture(page);
  await page.goto("/e2e/catalog-installation-fixture.html");
  await page.locator(".table-column-chooser summary").click();
  await page.getByRole("checkbox", { name: "Product", exact: true }).uncheck();
  await page.locator(".table-column-chooser summary").click();
  await tabToSelection(page, 0); await page.keyboard.press("Enter");
  await page.locator('input[aria-label="Quantity"]').fill("3");
  await page.locator(".catalog-create-row button").first().click();
  await expect(page.locator(".catalog-retry")).toBeVisible();
  for (const button of await page.locator(".catalog-select").all()) await expect(button).toBeDisabled();
  await expect(page.locator(".catalog-select").first()).toHaveAttribute("aria-pressed", "true");
  await page.locator('tr[data-item-id="selection-2"] td').nth(1).click();
  await page.locator('input[type="search"]').click();
  await page.keyboard.press("ArrowDown"); await page.keyboard.press("Enter"); await page.keyboard.press("Space");
  await expect(page.locator("tr.selected")).toHaveAttribute("data-item-id", "selection-1");
  await expect(page.locator(".catalog-retry")).toBeVisible();
  await expect(page.locator(".catalog-create-row input")).toHaveCount(0);
  await expect(page.locator(".catalog-select").first()).toHaveAttribute("aria-pressed", "true");
  expect(f.posts[0]).toMatchObject({ catalogItemId: "selection-1", name: "Alpha Relay", quantity: 3 });
  expect(f.posts).toHaveLength(1); expect(f.unexpected).toEqual([]); expect(f.errors).toEqual([]);
});
