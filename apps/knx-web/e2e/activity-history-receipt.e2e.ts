/** Verify run receipts in Chromium through the real parent with intercepted API requests. */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import fixtures from "../src/downloadReceipt.fixtures.json" with { type: "json" };

const SHOTS = process.env.RECEIPT_SHOTS ?? "";
type Row = Record<string, unknown>;

/** Strict page of shared fixture rows, one of them with the longest legal program id and version. */
function pageOf() {
  const long = structuredClone(fixtures.completePersisted) as Row;
  const program = ((long.downloadEvidence as Row).preparation as Row).program as Row;
  program.id = "A-<>&'".repeat(22).slice(0, 128);
  program.applicationVersion = "V".repeat(32);
  const rows: Row[] = [
    long,
    structuredClone(fixtures.persistenceFailed) as Row,
    structuredClone(fixtures.changedSource) as Row,
    structuredClone(fixtures.appliedWithInterruptedCheck) as Row,
    structuredClone(fixtures.interruptedPartially) as Row,
    structuredClone(fixtures.notAttempted) as Row,
    structuredClone(fixtures.unavailablePreparation) as Row,
    structuredClone(fixtures.preparationOnlyUnknown) as Row,
    structuredClone(fixtures.legacy) as Row,
  ];
  const entries = rows.map((row, index) => ({ sequence: index + 1, serverIncarnation: "synthetic-server", interrupted: false, ...row, id: index + 1 }));
  return { format: 2, coverage: "partial", durability: "persistent", hasMore: false, nextCursor: entries.length,
    untracked: ["deviceIdentify", "groupWrite", "serialAddress", "deviceDownload", "addressProgramming", "busMonitor", "lineScan"], entries };
}

/**
 * Captures one complete history entry. The workbench is a hidden-overflow scroller, so an entry
 * taller than the viewport is clipped by an element screenshot: grow the viewport to the entry
 * (width unchanged), scroll it to the top, wait for fonts and a frame, require every text rect of
 * the entry to lie inside every clipping ancestor, capture, then restore the viewport.
 */
async function captureWhole(page: Page, entry: Locator, path: string) {
  const original = page.viewportSize()!;
  const height = Math.ceil((await entry.boundingBox())!.height) + 80;
  await page.setViewportSize({ width: original.width, height: Math.max(original.height, height) });
  await entry.evaluate((node) => node.scrollIntoView({ block: "start" }));
  await page.evaluate(() => document.fonts.ready);
  await page.evaluate(() => new Promise<void>((done) => requestAnimationFrame(() => done())));
  const clipped = await entry.evaluate((node) => {
    const clips: DOMRect[] = [new DOMRect(0, 0, innerWidth, innerHeight)];
    for (let e = node.parentElement; e; e = e.parentElement) {
      if (getComputedStyle(e).overflowY !== "visible") clips.push(e.getBoundingClientRect());
    }
    const outside: string[] = []; let texts = 0;
    const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
    for (let text = walker.nextNode(); text; text = walker.nextNode()) {
      if (!(text.textContent ?? "").trim()) continue;
      texts += 1;
      const range = document.createRange(); range.selectNodeContents(text);
      const box = range.getBoundingClientRect();
      if (clips.some((clip) => box.top < clip.top - 0.5 || box.bottom > clip.bottom + 0.5)) outside.push((text.textContent ?? "").trim().slice(0, 30));
    }
    return { texts, outside };
  });
  expect(clipped.texts, "the entry has text").toBeGreaterThan(20);
  expect(clipped.outside, "text outside a clipping ancestor or the viewport at capture").toEqual([]);
  await entry.screenshot({ path });
  await page.setViewportSize(original);
}

async function open(page: Page, requests: string[], language: "en" | "de") {
  await page.route("**/api/**", (route) => {
    const request = route.request(); const url = new URL(request.url());
    requests.push(`${request.method()} ${url.pathname}${url.search}`);
    const history = url.pathname === "/api/bus/history";
    return route.fulfill({ status: history ? 200 : 404, contentType: "application/json",
      body: JSON.stringify(history ? pageOf() : { error: "intercepted fixture only" }) });
  });
  await page.goto(`/e2e/activity-history-fixture.html?lang=${language}`);
  await expect.poll(() => requests).toEqual(["POST /api/bus/discover", "GET /api/bus/monitor/telegrams?since=0"]);
  const messages = language === "de" ? de : en;
  await page.getByRole("button", { name: messages["activityHistory.title"], exact: true }).click();
  await expect(page.locator(".activity-history-entry")).toHaveCount(9);
}

for (const language of ["en", "de"] as const) {
  for (const width of [400, 1440]) {
    test(`${language} ${width}px: receipt states, strict admission, no overflow, only intercepted GETs`, async ({ page }) => {
      const m = language === "de" ? de : en;
      await page.setViewportSize({ width, height: 900 });
      const requests: string[] = [];
      await open(page, requests, language);
      expect(requests).toEqual(["POST /api/bus/discover", "GET /api/bus/monitor/telegrams?since=0", "GET /api/bus/history?after=0&limit=50"]);

      const receipts = page.locator(".download-receipt");
      await expect(receipts).toHaveCount(9);
      await expect(receipts.nth(0)).toContainText(m["downloadReceipt.project.persisted"]);
      await expect(receipts.nth(1)).toContainText(m["downloadReceipt.partsComputed"]);
      await expect(receipts.nth(1)).not.toContainText(m["downloadReceipt.partsApplied"]);
      await expect(receipts.nth(2)).toContainText(m["downloadReceipt.source.changedOrUnknown"]);
      await expect(receipts.nth(3).locator("xpath=..").locator(".post-download-check")).toHaveAttribute("data-check", "interrupted");
      await expect(receipts.nth(3)).toHaveAttribute("data-settlement", "persisted");
      await expect(receipts.nth(4)).toContainText(m["downloadReceipt.source.notChecked"]);
      await expect(receipts.nth(5)).toContainText(m["downloadReceipt.project.notAttempted"]);
      await expect(receipts.nth(6)).toContainText(m["downloadReceipt.scope.partial-both"]);
      await expect(receipts.nth(6)).toContainText(m["downloadReceipt.plan.unavailable"]);
      await expect(receipts.nth(7)).toContainText(m["downloadReceipt.settlement.absent"]);
      await expect(receipts.nth(8)).toContainText(m["downloadReceipt.absent"]);
      // Structure: a heading per receipt, a labelled table for every applied/computed settlement.
      await expect(page.locator(".download-receipt h4")).toHaveText(Array(9).fill(m["downloadReceipt.title"]));
      await expect(page.locator(".download-receipt-parts")).toHaveCount(5);
      await expect(page.locator(".download-receipt-parts thead th").first()).toHaveText(m["downloadReceipt.part"]);

      // The 128-byte id and 32-byte version stay inside the viewport; no body overflow.
      const geometry = await page.evaluate(() => {
        const viewport = innerWidth;
        const outside = [...document.querySelectorAll<HTMLElement>(".download-receipt, .download-receipt code")]
          .filter((node) => node.getBoundingClientRect().right > viewport + 0.5).length;
        return { viewport, document: document.documentElement.scrollWidth, outside };
      });
      expect(geometry.document, JSON.stringify(geometry)).toBeLessThanOrEqual(geometry.viewport);
      expect(geometry.outside, JSON.stringify(geometry)).toBe(0);

      // Word integrity, measured per word with a Range: no ordinary word, heading
      // word or table-cell word may be split across lines. Code (ids, hashes) may wrap.
      const split = await page.evaluate(() => {
        const broken: string[] = [];
        const nodes = document.querySelectorAll<HTMLElement>(".download-receipt h4, .download-receipt h5, .download-receipt p, .download-receipt dt, .download-receipt dd, .download-receipt th, .download-receipt td");
        for (const node of nodes) {
          const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
          for (let text = walker.nextNode(); text; text = walker.nextNode()) {
            if ((text.parentElement as HTMLElement).closest("code")) continue;
            for (const match of (text.textContent ?? "").matchAll(/\S+/g)) {
              const range = document.createRange();
              range.setStart(text, match.index!); range.setEnd(text, match.index! + match[0].length);
              const lines = new Set([...range.getClientRects()].map((rect) => Math.round(rect.top)));
              if (lines.size > 1) broken.push(match[0]);
            }
          }
        }
        return broken;
      });
      expect(split, "words split across lines").toEqual([]);

      // A table wider than the pane scrolls inside its own container; the container fits the viewport.
      const tables = await page.evaluate(() => [...document.querySelectorAll<HTMLElement>(".download-receipt-parts")].map((table) => {
        let box: HTMLElement = table;
        while (box.parentElement && !/auto|scroll/.test(getComputedStyle(box).overflowX)) box = box.parentElement;
        // The widest word of each column plus the cell padding is the least width the columns can have.
        const rows = [...table.querySelectorAll("tr")].map((row) => [...row.children] as HTMLElement[]);
        const columns = rows[0].map((_, index) => Math.max(...rows.map((row) => {
          const cell = row[index]; const walker = document.createTreeWalker(cell, NodeFilter.SHOW_TEXT);
          let widest = 0;
          for (let text = walker.nextNode(); text; text = walker.nextNode()) {
            for (const match of (text.textContent ?? "").matchAll(/\S+/g)) {
              const range = document.createRange();
              range.setStart(text, match.index!); range.setEnd(text, match.index! + match[0].length);
              widest = Math.max(widest, range.getBoundingClientRect().width);
            }
          }
          const style = getComputedStyle(cell);
          return widest + parseFloat(style.paddingLeft) + parseFloat(style.paddingRight);
        })));
        const rect = table.getBoundingClientRect(); const boxRect = box.getBoundingClientRect();
        return { scrolls: /auto|scroll/.test(getComputedStyle(box).overflowX), boxRight: boxRect.right,
          boxClient: box.clientWidth, boxScroll: box.scrollWidth, tableWidth: rect.width, least: columns.reduce((a, b) => a + b, 0), viewport: innerWidth };
      }));
      expect(tables).toHaveLength(5);
      for (const table of tables) {
        expect(table.scrolls, JSON.stringify(table)).toBe(true);
        expect(table.boxRight, JSON.stringify(table)).toBeLessThanOrEqual(table.viewport + 0.5);
        expect(table.tableWidth + 0.5, JSON.stringify(table)).toBeGreaterThanOrEqual(table.least);
        // Genuinely too wide for its container: the container scrolls locally.
        if (table.least > table.boxClient + 0.5) expect(table.boxScroll, JSON.stringify(table)).toBeGreaterThan(table.boxClient);
      }
      await expect(page.locator("html")).toHaveAttribute("lang", language);
      if (SHOTS) {
        // Bounded shots of the real rows: applied, computed, applied beside the interrupted check.
        for (const [role, index] of [["applied", 0], ["computed", 1], ["interrupted-check", 3]] as const) {
          await captureWhole(page, page.locator(".activity-history-entry").nth(index), `${SHOTS}/receipt-${language}-${width}-${role}.png`);
        }
      }
    });
  }
}
