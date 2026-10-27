/** ADR-0093: the catalogue workspace sends an explicit installation for a line-less device. */
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
