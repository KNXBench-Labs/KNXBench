/** Checks shared toast cards, expiry, dismissal and motion in an offline Chromium fixture. */
import { expect, test, type Locator, type Page } from "@playwright/test";

async function open(page: Page, query = "") {
  const unexpected: string[] = [];
  await page.route("**/api/**", async (route) => {
    unexpected.push(route.request().url());
    await route.abort();
  });
  await page.goto(`/e2e/toast-fixture.html?${query}`);
  return unexpected;
}

async function cardStyle(card: Locator) {
  return card.evaluate((el) => {
    const s = getComputedStyle(el);
    return {
      border: s.borderTopColor, stripe: s.borderLeftColor, stripeWidth: s.borderLeftWidth,
      minWidth: s.minWidth, radius: s.borderRadius, padding: s.padding,
      background: s.backgroundColor, shadow: s.boxShadow, animation: s.animationName,
    };
  });
}

for (const theme of ["graphite", "porcelain", "cupertino", "lcars"]) {
  for (const width of [1440, 400]) {
    test(`${theme} at ${width}px: standard toasts use the achievement card without a fake badge`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      const unexpected = await open(page, `theme=${theme}&lang=${width === 400 ? "de" : "en"}`);
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await page.getByRole("button", { name: "Status", exact: true }).click();
      await page.getByRole("button", { name: "Error", exact: true }).click();
      await page.getByRole("button", { name: "Achievement", exact: true }).click();
      const status = page.locator(".toast--fun");
      const error = page.locator(".toast--error");
      const achievement = page.locator(".toast--achievement");
      const standardStyle = await cardStyle(status);
      const achievementStyle = await cardStyle(achievement);
      expect(standardStyle).toEqual(achievementStyle);
      const errorStyle = await cardStyle(error);
      expect(errorStyle.stripe).not.toBe(standardStyle.stripe);
      expect({ ...errorStyle, stripe: standardStyle.stripe }).toEqual(standardStyle);
      await expect(status).toHaveAttribute("role", "status");
      await expect(error).toHaveAttribute("role", "alert");
      await expect(error.locator(".toast-hint")).toContainText(width === 400 ? "Englisch" : "English");
      await expect(page.locator(".toast--fun .achievement-badge, .toast--error .achievement-badge")).toHaveCount(0);
      for (const card of [status, error, achievement]) {
        const box = await card.boundingBox();
        expect(box!.x).toBeGreaterThanOrEqual(0);
        expect(box!.x + box!.width).toBeLessThanOrEqual(width);
      }
      expect(unexpected).toEqual([]);
    });
  }
}

test("all toast kinds stay nine seconds and use the same automatic exit", async ({ page }) => {
  await page.clock.install({ time: new Date("2026-10-08T12:00:00Z") });
  await page.clock.pauseAt(new Date("2026-10-08T12:00:01Z"));
  const unexpected = await open(page);
  for (const name of ["Status", "Error", "Achievement"]) await page.getByRole("button", { name, exact: true }).click();
  await page.clock.runFor(8999);
  await expect(page.locator(".toast:not(.toast--leaving)")).toHaveCount(3);
  await page.clock.runFor(1);
  await expect(page.locator(".toast--leaving")).toHaveCount(3);
  for (const card of await page.locator(".toast").all()) expect((await cardStyle(card)).animation).toBe("knx-achievement-out");
  await page.clock.runFor(1000);
  await expect(page.locator(".toast")).toHaveCount(0);
  expect(unexpected).toEqual([]);
});

for (const kind of ["fun", "error"]) {
  test(`${kind}: manual dismissal ends at animationend, before the fallback`, async ({ page }) => {
    await open(page);
    await page.getByRole("button", { name: kind === "fun" ? "Status" : "Error", exact: true }).click();
    const card = page.locator(`.toast--${kind}`);
    await card.getByRole("button", { name: "Dismiss" }).click();
    await expect(card).toHaveClass(/toast--leaving/);
    expect((await cardStyle(card)).animation).toBe("knx-achievement-out");
    await expect(card).toHaveCount(0, { timeout: 800 });
  });
}

test("motion off: dismissal snaps every toast out without a visible slide", async ({ page }) => {
  const unexpected = await open(page, "motion=off");
  await expect(page.locator("html")).toHaveAttribute("data-motion-level", "off");
  for (const name of ["Status", "Error", "Achievement"]) await page.getByRole("button", { name, exact: true }).click();
  for (const kind of ["fun", "error", "achievement"]) {
    const card = page.locator(`.toast--${kind}`);
    await expect(card).toHaveCSS("animation-duration", "0s");
    await card.getByRole("button").click();
    await expect(card).toHaveCount(0, { timeout: 500 });
  }
  expect(unexpected).toEqual([]);
});

test("reduced motion: dismissed toasts are invisible until the fallback removes them", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.clock.install({ time: new Date("2026-10-08T12:00:00Z") });
  await page.clock.pauseAt(new Date("2026-10-08T12:00:01Z"));
  const unexpected = await open(page);
  for (const name of ["Status", "Error", "Achievement"]) await page.getByRole("button", { name, exact: true }).click();
  await expect(page.locator(".toast")).toHaveCount(3);
  for (const kind of ["fun", "error", "achievement"]) await page.locator(`.toast--${kind} button`).click();
  for (const kind of ["fun", "error", "achievement"]) {
    const card = page.locator(`.toast--${kind}`);
    expect((await cardStyle(card)).animation).toBe("none");
    await expect(card).toHaveCSS("opacity", "0");
    await expect(card).toHaveCSS("pointer-events", "none");
  }
  await page.clock.runFor(1000);
  await expect(page.locator(".toast")).toHaveCount(0);
  expect(unexpected).toEqual([]);
});

test("long unbroken messages fit a narrow viewport without hiding the dismiss button", async ({ page }) => {
  await page.setViewportSize({ width: 400, height: 900 });
  await open(page);
  await page.getByRole("button", { name: "Long status" }).click();
  const card = page.locator(".toast--fun");
  expect(await card.evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true);
  await expect(card.getByRole("button", { name: "Dismiss" })).toBeInViewport();
});
