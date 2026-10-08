/** U19: the synthetic flow study renders, stays keyboard reachable, honours freeze and motion. */
import { expect, test, type Page } from "@playwright/test";

async function open(page: Page, query = "") {
  const requests: string[] = [];
  page.on("request", (request) => { if (new URL(request.url()).pathname.startsWith("/api/")) requests.push(request.url()); });
  await page.setViewportSize({ width: 1280, height: 760 });
  await page.goto(`/e2e/flow-study-fixture.html${query}`);
  await expect(page.getByText("Synthetic study")).toBeVisible();
  await expect.poll(() => page.locator("g.flow-node").count()).toBeGreaterThan(4);
  return requests;
}

const admitted = async (page: Page) => Number((await page.getByTestId("counters").textContent())!.match(/Admitted (\d+)/)![1]);

test("renders a synthetic map without any API request and names an activity leader", async ({ page }) => {
  const requests = await open(page);
  await expect(page.getByTestId("leader")).toHaveText(/^9\.\d+\.\d+$/);
  await expect(page.locator("path.flow-edge").first()).toHaveAttribute("marker-end", "url(#flow-arrow)");
  expect(requests).toEqual([]);
});

test("reaches a device and its current values with the keyboard alone", async ({ page }) => {
  await open(page);
  const list = page.getByRole("list", { name: "Devices and group nodes" });
  // Generated nodes can change the sorted first entry between focus and assertion.
  const id = await list.getByRole("button").first().innerText();
  const device = list.getByRole("button", { name: id, exact: true });
  await device.focus();
  await expect(device).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(device).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("inspector")).toContainText("Current values");
});

test("freeze keeps every position while traffic and counters continue", async ({ page }) => {
  await open(page);
  await page.getByRole("button", { name: "Freeze layout" }).click();
  await expect(page.getByRole("button", { name: "Freeze layout" })).toHaveAttribute("aria-pressed", "true");
  const positions = () => page.locator("g.flow-node").evaluateAll((nodes) => nodes.map((node) => node.getAttribute("transform")));
  const before = await positions();
  const count = await admitted(page);
  await page.waitForTimeout(1500);
  expect((await positions()).slice(0, before.length)).toEqual(before);
  expect(await admitted(page)).toBeGreaterThan(count);
});

test("with motion off nothing moves or pulses, but values still arrive", async ({ page }) => {
  await open(page, "?motion=off");
  await page.waitForTimeout(1500);
  const visiblePulses = await page.locator("circle.flow-pulse").evaluateAll((dots) => dots.filter((dot) => (dot as SVGElement).style.display !== "none").length);
  expect(visiblePulses).toBe(0);
  expect(await page.evaluate(() => (window as unknown as { __flowStudy: { metrics: { layoutSteps: number } } }).__flowStudy.metrics.layoutSteps)).toBe(0);
  await expect.poll(() => page.locator("text.flow-badge").evaluateAll((texts) => texts.filter((text) => text.textContent).length)).toBeGreaterThan(0);
});
