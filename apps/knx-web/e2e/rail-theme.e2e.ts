/** Checks Rail primary paint in real interaction states and live motion cancellation. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { expect, test, type Locator } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { intercept } from "./communication-fixture";

/** Composite the two constant-colour primary layers using this browser's own colour parser. */
async function paintedContrast(control: Locator) {
  return control.evaluate((node) => {
    const style = getComputedStyle(node);
    const colors = style.backgroundImage.match(/(?:rgba?|color)\([^)]*\)/g) ?? [];
    // This probe is deliberately narrow: fail rather than guess when the primary
    // paint stops being two constant-colour gradients (veil, then accent).
    if (colors.length !== 4 || colors[0] !== colors[1] || colors[2] !== colors[3]) {
      throw new Error(`Primary paint is not two constant-colour layers: ${style.backgroundImage}`);
    }
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 1;
    const context = canvas.getContext("2d")!;
    context.fillStyle = style.backgroundColor;
    context.fillRect(0, 0, 1, 1);
    for (const color of [colors[2], colors[0]]) {
      context.fillStyle = color;
      context.fillRect(0, 0, 1, 1);
    }
    const background = Array.from(context.getImageData(0, 0, 1, 1).data).slice(0, 3);
    context.clearRect(0, 0, 1, 1);
    context.fillStyle = style.color;
    context.fillRect(0, 0, 1, 1);
    const foreground = Array.from(context.getImageData(0, 0, 1, 1).data).slice(0, 3);
    const luminance = (rgb: number[]) => rgb.map((v) => v / 255)
      .map((v) => v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)
      .reduce((sum, v, index) => sum + v * [0.2126, 0.7152, 0.0722][index], 0);
    const [low, high] = [luminance(foreground), luminance(background)].sort((a, b) => a - b);
    return { ratio: (high + 0.05) / (low + 0.05), foreground, background,
      image: style.backgroundImage, fill: style.backgroundColor };
  });
}

/** Settle native effects explicitly; this test checks final paint, not elapsed time. */
async function finishEffects(control: Locator) {
  await control.evaluate((node) => node.getAnimations().forEach((animation) => animation.finish()));
}

for (const theme of ["rail", "rail-dark"]) test(`${theme}: primary contrast at rest, hover and press`, async ({ page }, info) => {
  const state = await intercept(page, "en", 8, theme);
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  await page.getByRole("button", { name: en["toolbar.newProject"], exact: true }).first().click();
  const dialog = page.getByRole("dialog", { name: en["newProject.title"], exact: true });
  const button = dialog.getByRole("button", { name: en["newProject.create"], exact: true });
  await expect(button).toBeEnabled();
  await page.mouse.move(1, 1);
  const rest = await paintedContrast(button);
  await button.hover();
  await expect(button).toHaveJSProperty("disabled", false);
  expect(await button.evaluate((node) => node.matches(":hover"))).toBe(true);
  await finishEffects(button);
  const hover = await paintedContrast(button);
  let pressed;
  await page.mouse.down();
  try {
    expect(await button.evaluate((node) => node.matches(":active"))).toBe(true);
    await finishEffects(button);
    pressed = await paintedContrast(button);
  } finally {
    // Release away from Create: this probe must not create a project.
    await page.mouse.move(1, 1);
    await page.mouse.up();
  }
  await info.attach("primary-paint", { body: JSON.stringify({ theme, rest, hover, pressed }, null, 2), contentType: "application/json" });
  expect.soft(rest.ratio, "normal-size primary text at rest").toBeGreaterThanOrEqual(4.5);
  expect.soft(hover.ratio, "normal-size primary text on hover").toBeGreaterThanOrEqual(4.5);
  expect.soft(pressed.ratio, "normal-size primary text while pressed").toBeGreaterThanOrEqual(4.5);
  expect(state.projectOpen).toBe(false);
  expect(state.requests).not.toContain("POST /api/project/new");
  expect(state.unexpected).toEqual([]);
});

test("rail: theme Off, OS reduction and leaving Rail cancel the hover effect", async ({ page }) => {
  const state = await intercept(page, "en", 8, "rail");
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/");
  const root = page.locator("html");
  await expect(root).toHaveAttribute("data-motion-theme", "standard");
  const button = page.locator(".workbench-toolbar .command-entry");
  await button.hover();
  await expect(button).toHaveCSS("animation-name", "rail-sweep");
  await root.evaluate((node) => { (node as HTMLElement).dataset.motionTheme = "off"; });
  await expect(button).toHaveCSS("animation-name", "none");
  expect(await button.evaluate((node) => node.getAnimations().some((animation) =>
    animation instanceof CSSAnimation && animation.animationName.startsWith("rail-")))).toBe(false);
  await root.evaluate((node) => { (node as HTMLElement).dataset.motionTheme = "standard"; });
  await expect(button).toHaveCSS("animation-name", "rail-sweep");
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(button).toHaveCSS("animation-name", "none");
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await root.evaluate((node) => { delete (node as HTMLElement).dataset.presentation; (node as HTMLElement).dataset.theme = "graphite"; });
  await expect(button).toHaveCSS("animation-name", "none");
  expect(await page.locator(".workbench-toolbar").evaluate((node) => getComputedStyle(node, "::after").content)).toBe("none");
  expect(state.unexpected).toEqual([]);
});
