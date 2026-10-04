/** KL-60: a 3,300-entry diff table stays a small DOM, scrolls to every entry and filters. */
import { expect, test, type Page } from "@playwright/test";

const TOTAL = 3300;
const rows = (page: Page) => page.locator(".project-diff-viewport .project-diff-entries > li");

async function open(page: Page) {
  await page.route("**/api/**", (route) => route.fulfill({ status: 404, body: "{}" }));
  await page.goto("/e2e/diff-virtual-fixture.html");
  await page.getByRole("button", { name: `Group addresses (${TOTAL})` }).click();
  await expect(page.getByRole("status")).toHaveText(`${TOTAL} of ${TOTAL} entries shown.`);
}

const settle = (page: Page) =>
  page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));

test("renders a bounded window and reaches every entry by scrolling, with real row heights", async ({ page }) => {
  await open(page);
  const viewport = page.locator(".project-diff-viewport");
  expect(await rows(page).count()).toBeLessThan(120);
  expect(await page.locator(".project-diff-viewport li").count()).toBeLessThan(120);
  const seen = new Set<number>();
  for (let step = 0; step < 2000; step++) {
    await settle(page);
    for (const value of await rows(page).evaluateAll((items) => items.map((item) => item.getAttribute("aria-posinset")))) {
      seen.add(Number(value));
    }
    const done = await viewport.evaluate((element) => {
      if (element.scrollTop + element.clientHeight >= element.scrollHeight - 1) return true;
      element.scrollTop += element.clientHeight * 0.8;
      return false;
    });
    if (done) break;
  }
  expect(seen.size).toBe(TOTAL);
  expect(Math.min(...seen)).toBe(1);
  expect(Math.max(...seen)).toBe(TOTAL);
  await expect(rows(page).last()).toContainText("3/2/99");
  await expect(rows(page).last()).toBeInViewport();
  expect(await rows(page).count()).toBeLessThan(120);
});

test("scrolls with the keyboard from the focused list", async ({ page }) => {
  await open(page);
  const viewport = page.getByRole("region", { name: "Entries (scrollable)" });
  await viewport.focus();
  await page.keyboard.press("End");
  await expect(rows(page).last()).toHaveAttribute("aria-posinset", String(TOTAL));
  await expect(rows(page).last()).toBeInViewport();
  await page.keyboard.press("Home");
  await expect(rows(page).first()).toHaveAttribute("aria-posinset", "1");
});

test("keeps visible rows steady while unmeasured rows above them grow", async ({ page }) => {
  await open(page);
  const viewport = page.getByRole("region", { name: "Entries (scrollable)" });
  await viewport.focus();
  await page.keyboard.press("End");
  await expect(rows(page).last()).toHaveAttribute("aria-posinset", String(TOTAL));
  for (let step = 0; step < 15; step++) {
    await settle(page);
    // An anchor row near the top must move down by exactly the scrolled
    // distance, even though the rows entering above it get measured taller.
    const moved = await viewport.evaluate(async (element) => {
      const frame = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const box = element.getBoundingClientRect();
      const anchor = document.elementFromPoint(box.left + 20, box.top + box.height * 0.2)?.closest("li[aria-posinset]");
      if (!anchor) return { error: "no anchor" };
      const position = anchor.getAttribute("aria-posinset");
      const before = anchor.getBoundingClientRect().top;
      const distance = Math.round(box.height * 0.6);
      element.scrollTop -= distance;
      await frame();
      await frame();
      const after = element.querySelector(`li[aria-posinset="${position}"]`)?.getBoundingClientRect().top;
      return { position, delta: after === undefined ? null : after - before, distance };
    });
    expect(moved).not.toHaveProperty("error");
    expect(Math.abs((moved.delta ?? Number.NaN) - (moved.distance ?? 0)), JSON.stringify(moved)).toBeLessThanOrEqual(2);
  }
});

test("filters by text and by status, and the window follows the filter", async ({ page }) => {
  await open(page);
  const search = page.getByRole("searchbox", { name: "Filter entries" });
  await search.fill("changed 29");
  await expect(page.getByRole("status")).toHaveText(`11 of ${TOTAL} entries shown.`);
  // Tall changed rows: the window may hold fewer than all 11; every rendered one matches.
  await expect(rows(page).first()).toHaveAttribute("aria-setsize", "11");
  await expect(rows(page).first()).toContainText("Changed 29");
  expect((await rows(page).allTextContents()).every((text) => /Changed 29\d?\b/.test(text))).toBe(true);
  await search.fill("");
  await page.getByRole("button", { name: "changed (300)" }).click();
  await expect(page.getByRole("button", { name: "changed (300)" })).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByRole("status")).toHaveText(`300 of ${TOTAL} entries shown.`);
  await expect(rows(page).first()).toHaveAttribute("aria-setsize", "300");
  await expect(rows(page).first()).toContainText("Changed 0");
  await search.fill("nothing like this");
  await expect(page.getByText("No entry matches the filter.")).toBeVisible();
  await expect(rows(page)).toHaveCount(0);
  await search.press("Escape");
  await expect(search).toHaveValue("");
  await expect(page.getByRole("status")).toHaveText(`300 of ${TOTAL} entries shown.`);
});
