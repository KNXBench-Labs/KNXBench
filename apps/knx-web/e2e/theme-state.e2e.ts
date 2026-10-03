/** Exercises real representative engineering components with fully intercepted local settings. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { expect, test, type Page } from "@playwright/test";
import { themeStatePacks } from "./theme-state-data";
import { canonicalJson } from "../src/canonicalJson";

type Palette = { id: string; resolved: string; scheme: "light" | "dark"; accents: boolean };
const palettes: Palette[] = [
  { id: "porcelain", resolved: "porcelain", scheme: "light", accents: true },
  { id: "graphite", resolved: "graphite", scheme: "dark", accents: true },
  { id: "cupertino", resolved: "cupertino", scheme: "light", accents: false },
  { id: "neon-grid", resolved: "neon-grid", scheme: "dark", accents: false },
  { id: "bitcoin-defi", resolved: "bitcoin-defi", scheme: "dark", accents: false },
  { id: "system", resolved: "porcelain", scheme: "light", accents: true },
  { id: "system", resolved: "graphite", scheme: "dark", accents: true },
  { id: "user-orchid", resolved: "user-orchid", scheme: "light", accents: true },
  { id: "user-midnight", resolved: "user-midnight", scheme: "dark", accents: true },
];

async function openRepresentative(page: Page, palette = palettes[1]) {
  const unexpected: string[] = [];
  const writes: Record<string, unknown>[] = [];
  const packs = themeStatePacks();
  let settings: Record<string, unknown> = { theme: palette.id, accent: "mint", density: "compact",
    uiLanguage: "en", motionStyle: "apple", motionLevel: "off", uiThemePacks: Object.fromEntries(packs.map((pack) => [pack.id, pack])),
    foreign: { preserve: "synthetic closing marker" } };
  await page.emulateMedia({ colorScheme: palette.scheme, reducedMotion: "reduce" });
  page.on("pageerror", (error) => unexpected.push(error.message));
  page.on("console", (message) => { if (message.type() === "error" || message.type() === "warning") unexpected.push(message.text()); });
  await page.route("**/*", async (route) => {
    const request = route.request(), url = new URL(request.url());
    if (url.origin !== "http://127.0.0.1:4173") { unexpected.push(url.origin); await route.abort(); return; }
    if (url.pathname === "/api/settings") {
      if (request.method() === "PUT") {
        const payload = request.postDataJSON() as { settings: Record<string, unknown>; expectedSettings?: Record<string, unknown> };
        writes.push(payload.settings);
        if (payload.expectedSettings && Object.entries(payload.expectedSettings).some(([key, value]) =>
          canonicalJson(settings[key] ?? null) !== canonicalJson(value))) {
          unexpected.push("fixture refused an invalid conditional write"); await route.abort(); return;
        }
        settings = { ...settings, ...payload.settings };
      } else if (request.method() !== "GET") { unexpected.push(request.method()); await route.abort(); return; }
      await route.fulfill({ json: { schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings } });
    } else if (url.pathname.startsWith("/api/")) { unexpected.push(url.pathname); await route.abort(); }
    else await route.continue();
  });
  await page.goto("/e2e/theme-manager-fixture.html?representative=1");
  await expect(page.getByRole("heading", { name: "Representative theme state fixture", exact: true })).toBeVisible({ timeout: 2_000 });
  await expect(page.locator("html")).toHaveAttribute("data-theme", palette.resolved);
  await expect(page.locator("html")).toHaveCSS("color-scheme", palette.scheme);
  // Explicit rendered-style negative controls. Normal acceptance must leave this unset.
  const roleControl = process.env.KNXBENCH_U18_ROLE_CONTROL;
  if (roleControl) {
    const overrides: Record<string, string> = {
      selection: '.address-table tr[aria-selected="true"] { background: var(--knx-surface) !important; }',
      focus: 'input:focus-visible { border-bottom-color: var(--knx-border) !important; box-shadow: none !important; }',
      validation: '.field-error { color: var(--knx-muted) !important; }',
    };
    if (!Object.hasOwn(overrides, roleControl)) throw new Error("Unknown U18 rendered-style control");
    await page.addStyleTag({ content: overrides[roleControl] });
  }
  return { writes, settings: () => settings, check: () => expect(unexpected).toEqual([]) };
}

/** Resolve CSS roles in this browser, including color-mix rather than guessing serialization. */
async function color(page: Page, expression: string) {
  return page.evaluate((value) => {
    const probe = document.createElement("span"); probe.style.color = value;
    document.body.append(probe); const resolved = getComputedStyle(probe).color; probe.remove(); return resolved;
  }, expression);
}

test("graphite exposes representative actual editor, table, inspector and diagnostic state", async ({ page }) => {
  const mock = await openRepresentative(page);
  await expect(page.getByLabel("Representative engineering states", { exact: true })).toBeVisible();
  mock.check();
});

for (const palette of palettes) test(`${palette.id} ${palette.scheme}: real state roles, keyboard focus, selection, validation, disabled dialog and accent`, async ({ page }, info) => {
  const mock = await openRepresentative(page, palette);
  const table = page.getByLabel("Representative address table", { exact: true });
  const inspector = page.getByLabel("Representative device inspector", { exact: true });
  const address = inspector.getByRole("textbox", { name: "Device number", exact: true });
  await expect(inspector.getByText("Synthetic actuator", { exact: true })).toBeVisible();
  await expect(address).toHaveValue("12");
  await expect(inspector.locator(".inspector")).toHaveCSS("color", await color(page, "var(--knx-foreground)"));

  const row = table.getByRole("row", { name: /1\/1\/1.*Lighting/ });
  await row.getByRole("button", { name: "1/1/1", exact: true }).click();
  await expect(row).toHaveAttribute("aria-selected", "true");
  await expect(row).toHaveCSS("background-color", await color(page, "color-mix(in srgb, var(--knx-accent) 14%, var(--knx-surface))"));
  const filter = table.locator(".address-filter");
  await filter.fill("1/1/2"); await expect(row).toHaveCount(0);
  await expect(table.getByRole("row", { name: /1\/1\/2.*Heating/ })).toBeVisible();
  await filter.fill(""); await expect(row).toHaveAttribute("aria-selected", "true");
  // Tab through actual inspector fields; focus is keyboard-driven, not a CSS class stub.
  await address.focus(); await page.keyboard.press("Tab");
  await expect(address).not.toBeFocused();
  await page.keyboard.press("Shift+Tab"); await expect(address).toBeFocused();
  await expect(address).toHaveCSS("border-bottom-color", await color(page, "var(--knx-accent)"));
  const focusRing = await address.evaluate((element) => getComputedStyle(element).boxShadow);
  expect(focusRing).toContain(await color(page, "color-mix(in srgb, var(--knx-accent) 25%, transparent)"));
  expect(focusRing).toContain("2px");
  await address.fill("256"); await address.press("Enter");
  const error = inspector.locator(".individual-address-field .field-error");
  await expect(error).toBeVisible(); await expect(error).not.toBeEmpty();
  await expect(error).toHaveCSS("color", await color(page, "var(--knx-error-color)"));
  await expect(address).toHaveValue("256"); // Invalid draft remains editable; persisted projection stays untouched.
  expect(mock.writes).toEqual([]); // Validation, selection and inspection have no domain/settings effects.
  await expect(page.getByRole("status", { name: "Representative diagnostic" })).toContainText("tokens.--knx-bg");
  await page.screenshot({ path: info.outputPath("representative-states.png"), fullPage: true });

  const opener = page.getByRole("button", { name: "Open sample dialog", exact: true });
  await opener.click();
  const dialog = page.getByRole("dialog", { name: "Sample diagnostic dialog", exact: true });
  const closer = dialog.getByRole("button", { name: "Close sample dialog", exact: true });
  await expect(closer).toBeFocused();
  await expect(dialog.getByRole("button", { name: "Unavailable sample action", exact: true })).toBeDisabled();
  await expect(dialog).toContainText("accents.blue.--knx-accent");
  await expect(dialog).toHaveCSS("color", await color(page, "var(--knx-foreground)"));
  await page.keyboard.press("Tab"); await expect(closer).toBeFocused();
  await page.keyboard.press("Shift+Tab"); await expect(closer).toBeFocused();
  await page.screenshot({ path: info.outputPath("representative-dialog.png"), fullPage: true });
  await page.keyboard.press("Escape"); await expect(dialog).toHaveCount(0); await expect(opener).toBeFocused();
  expect(mock.writes).toEqual([]);

  await page.getByRole("button", { name: "Open settings", exact: true }).click();
  const accent = page.getByRole("combobox", { name: "Accent color", exact: true });
  if (palette.accents) {
    await expect(accent).toBeEnabled();
    const before = await color(page, "var(--knx-accent)");
    await accent.selectOption("blue"); await expect(page.locator("html")).toHaveAttribute("data-accent", "blue");
    expect(await color(page, "var(--knx-accent)")).not.toBe(before);
    await expect.poll(() => mock.writes.length).toBe(1);
    expect(mock.writes[0]).toEqual({ accent: "blue" });
  } else {
    await expect(accent).toBeDisabled();
    await expect(accent).toHaveAttribute("aria-describedby", "settings-accent-unavailable-hint");
    expect(mock.writes).toEqual([]);
  }
  await page.keyboard.press("Escape");
  await expect(row).toHaveAttribute("aria-selected", "true");
  await expect(row).toHaveCSS("background-color", await color(page, "color-mix(in srgb, var(--knx-accent) 14%, var(--knx-surface))"));
  const pack = themeStatePacks().find((candidate) => candidate.id === palette.id);
  if (pack) expect(await page.locator("html").evaluate((element) => (element as HTMLElement).style.getPropertyValue("--knx-accent")))
    .toBe(pack.accents?.blue?.["--knx-accent"]);
  expect(mock.settings().foreign).toEqual({ preserve: "synthetic closing marker" });
  mock.check();
});
