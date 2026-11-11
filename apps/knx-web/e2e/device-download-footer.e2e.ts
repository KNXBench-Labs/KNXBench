/** Issue #43 in a browser: a running download stays visible in the status bar and leads back. */
import { expect, test, type Page } from "@playwright/test";

const tree = {
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "download-footer", snapshot_revision: 1,
  installations: [{ id: 1, name: "Installation 1", buildings: [], unassigned: [], group_addresses: [], group_ranges: [], topology: [] }],
};

async function open(page: Page, width: number) {
  const progress = { completedSteps: 5, writtenOctets: 524 };
  const writes: string[] = [];
  let projectOpen = false;
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    const json = (body: unknown, status = 200) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status") return json({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return json({ schemaVersion: 1, status: "ok", settings: {} });
    if (path === "/api/project" && method === "GET") return projectOpen ? json(tree) : json({ error: "no project open" }, 400);
    if (path === "/api/project/new" && method === "POST") { projectOpen = true; return json(tree); }
    if (path === "/api/bus/discover") return json({ interfaces: [] });
    if (path === "/api/bus/activity" && method === "GET") {
      return json({ serverIncarnation: "fixture", coverage: "partial", historyState: "disabled", oneShot: [], oneShotDropped: 0,
        busyLocks: ["deviceDownload"], untracked: [],
        sessions: [{ kind: "deviceDownload", id: 3, address: "1.1.67", state: "running", totalSteps: 14, totalOctets: 1457, ...progress }] });
    }
    if (method === "GET") return path.startsWith("/api/device-download/") || path.startsWith("/api/bus/")
      ? json({ error: "not in this fixture" }, 404) : json([]);
    writes.push(`${method} ${path}`);
    return json({ error: "not in this fixture" }, 404);
  });
  await page.setViewportSize({ width, height: 900 });
  await page.goto("/");
  return { progress, writes };
}

for (const width of [1400, 400]) {
  test(`#43 at ${width}px: the status bar names the running download, follows it and opens its tab`, async ({ page }) => {
    const { progress, writes } = await open(page, width);
    const footer = page.locator(".download-footer-progress");
    const text = footer.locator(".task-status-text");
    await expect(text).toHaveText("Download to 1.1.67: 5 of 14 steps done · 35 % of data written");
    await expect(footer.getByRole("progressbar", { name: "Data written and read back for 1.1.67" })).toBeVisible();
    progress.completedSteps = 9; progress.writtenOctets = 1100;
    await expect(text).toHaveText("Download to 1.1.67: 9 of 14 steps done · 75 % of data written", { timeout: 6_000 });
    const box = await footer.boundingBox();
    expect(box!.x + box!.width).toBeLessThanOrEqual(width);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    await footer.getByRole("button", { name: "Show the download to 1.1.67" }).click();
    await expect(page.getByRole("navigation", { name: "Bus diagnostics" }).getByRole("button", { name: "Download to device", exact: true }))
      .toHaveAttribute("aria-current", "page");
    expect(writes).toEqual([]);
  });
}
