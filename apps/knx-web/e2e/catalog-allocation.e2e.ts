/** MODEL-04: the catalog's opt-in address and unique-name options, in a real browser. */
import { test, expect, type Page } from "@playwright/test";

const item = {
  id: "cat-1", manufacturerId: "M-1", name: "Actuator", number: null, visibleDescription: null,
  productRefId: "P-1", hardware2programRefId: "HP-1",
};
const allocated = [["1.1.2", "Actuator 3"], ["1.1.4", "Actuator 4"], ["1.1.5", "Actuator 5"]]
  .map(([address, name], i) => ({ index: i + 1, deviceId: i + 1, name, address, diagnostics: [] }));
const labels = {
  en: { quantity: "Quantity", allocate: "Assign free addresses on the line", unique: "Keep names unique", address: "address" },
  de: { quantity: "Anzahl", allocate: "Freie Adressen der Linie vergeben", unique: "Namen eindeutig halten", address: "Adresse" },
};

async function mock(page: Page, answer: { status: number; body: unknown }) {
  const posts: Record<string, unknown>[] = [];
  const unexpected: string[] = [];
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === "/api/catalog/manufacturers") return route.fulfill({ contentType: "application/json", body: "[]" });
    if (path === "/api/catalog/items") return route.fulfill({ contentType: "application/json", body: JSON.stringify([item]) });
    if (path === "/api/devices" && request.method() === "POST") {
      posts.push(request.postDataJSON());
      return route.fulfill({ status: answer.status, contentType: "application/json", body: JSON.stringify(answer.body) });
    }
    unexpected.push(`${request.method()} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "fixture only" }) });
  });
  return { posts, unexpected };
}

async function prepare(page: Page, language: "en" | "de") {
  await page.goto(`/e2e/catalog-retry-fixture.html?lang=${language}`);
  await page.locator(".search-result").first().click();
  await page.getByLabel(labels[language].quantity, { exact: true }).fill("3");
  const allocate = page.getByRole("checkbox", { name: labels[language].allocate });
  const unique = page.getByRole("checkbox", { name: labels[language].unique });
  await expect(allocate).not.toBeChecked();
  await expect(unique).not.toBeChecked();
  return { allocate, unique };
}

for (const language of ["en", "de"] as const) {
  test(`${language} catalog sends both chosen options and lists the allocated addresses`, async ({ page }) => {
    const { posts, unexpected } = await mock(page, {
      status: 200, body: { tree: { installations: [] }, diagnostics: [], items: allocated, replayed: false },
    });
    const { allocate, unique } = await prepare(page, language);
    await allocate.check();
    await unique.check();
    await page.locator(".catalog-create-row button").first().click();
    const created = page.locator(".catalog-created-item");
    await expect(created).toHaveCount(3);
    await expect(created.nth(0)).toContainText(`${labels[language].address} 1.1.2`);
    await expect(created.nth(2)).toContainText("Actuator 5");
    expect(posts).toHaveLength(1);
    expect(posts[0]).toMatchObject({ lineId: 7, catalogItemId: "cat-1", quantity: 3, allocateAddresses: true, uniqueNames: true });
    expect(unexpected).toEqual([]);
  });

  test(`${language} catalog shows a refused allocation and creates nothing`, async ({ page }) => {
    const { posts, unexpected } = await mock(page, {
      status: 400, body: { error: "line 1.1 has 2 free device addresses, 3 requested" },
    });
    const { allocate } = await prepare(page, language);
    await allocate.check();
    await page.locator(".catalog-create-row button").first().click();
    await expect(page.locator(".field-error")).toContainText("2 free device addresses, 3 requested");
    await expect(page.locator(".catalog-retry")).toHaveCount(0);
    await expect(page.locator(".catalog-created-item")).toHaveCount(0);
    expect(posts).toHaveLength(1);
    expect(posts[0]).toMatchObject({ allocateAddresses: true });
    expect(posts[0]).not.toHaveProperty("uniqueNames");
    expect(unexpected).toEqual([]);
  });
}
