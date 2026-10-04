/** DATA-03: a catalog batch lost in transit is resent once, under the same requestId. */
import { test, expect, type Page } from "@playwright/test";

const item = {
  id: "cat-1", manufacturerId: "M-1", name: "Actuator", number: null, visibleDescription: null,
  productRefId: "P-1", hardware2programRefId: "HP-1",
};
const created = [1, 2, 3].map((index) => ({ index, deviceId: index, name: `Actuator ${index}`, diagnostics: [] }));

async function mock(page: Page, incarnationAfterLoss: string) {
  const posts: unknown[] = [];
  const unexpected: string[] = [];
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === "/api/catalog/manufacturers") return route.fulfill({ contentType: "application/json", body: "[]" });
    if (path === "/api/catalog/items") return route.fulfill({ contentType: "application/json", body: JSON.stringify([item]) });
    if (path === "/api/devices" && request.method() === "POST") {
      posts.push(request.postDataJSON());
      // The first submit is lost on the way back: the client cannot know
      // whether the server committed it.
      if (posts.length === 1) return route.abort("connectionreset");
      return route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({ tree: { installations: [] }, diagnostics: [], items: created, replayed: true }),
      });
    }
    if (path === "/api/project") {
      return route.fulfill({ contentType: "application/json",
        body: JSON.stringify({ installations: [], server_incarnation: incarnationAfterLoss }) });
    }
    unexpected.push(`${request.method()} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "fixture only" }) });
  });
  return { posts, unexpected };
}

async function submitThree(page: Page, language: string) {
  await page.goto(`/e2e/catalog-retry-fixture.html?lang=${language}`);
  await page.locator(".search-result").first().click();
  await page.locator('input[aria-label="' + (language === "de" ? "Anzahl" : "Quantity") + '"]').fill("3");
  await page.locator(".catalog-create-row button").first().click();
}

for (const language of ["en", "de"] as const) {
  test(`${language} catalog resends a lost batch once with the same requestId`, async ({ page }) => {
    const { posts, unexpected } = await mock(page, "fixture-incarnation-1");
    await submitThree(page, language);
    const retry = page.locator(".catalog-retry");
    await expect(retry).toBeVisible();
    await retry.click();
    await expect(page.locator(".catalog-created-item")).toHaveCount(3);
    await expect(retry).toHaveCount(0);
    expect(posts).toHaveLength(2);
    expect(posts[1]).toEqual(posts[0]);
    expect((posts[0] as { requestId?: string }).requestId).toMatch(/^[A-Za-z0-9_-]{1,128}$/);
    expect(posts[0]).toMatchObject({ lineId: 7, catalogItemId: "cat-1", name: "Actuator", quantity: 3 });
    expect(unexpected).toEqual([]);
  });

  test(`${language} catalog does not resend once the server has restarted`, async ({ page }) => {
    const { posts, unexpected } = await mock(page, "fixture-incarnation-2");
    await submitThree(page, language);
    await page.locator(".catalog-retry").click();
    await expect(page.locator(".catalog-retry")).toHaveCount(0);
    await expect(page.locator(".field-error")).toContainText(
      language === "de" ? "neu gestartet" : "The server has restarted");
    expect(posts).toHaveLength(1);
    expect(unexpected).toEqual([]);
  });
}
