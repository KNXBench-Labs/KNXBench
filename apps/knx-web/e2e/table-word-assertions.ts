/** Checks rendered token line boxes and the table's own horizontal scrolling boundary. */
import { expect, type Locator, type Page } from "@playwright/test";

export async function wordsIntact(cells: Locator) {
  await expect(cells.first()).toBeVisible();
  const broken = await cells.evaluateAll(elements => elements.flatMap(element => {
    const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    const failures: string[] = [];
    while (walker.nextNode()) {
      const node = walker.currentNode;
      for (const token of node.textContent!.matchAll(/\S+/g)) {
        const range = document.createRange();
        range.setStart(node, token.index!); range.setEnd(node, token.index! + token[0].length);
        if (range.getClientRects().length !== 1) failures.push(token[0]);
      }
    }
    return failures;
  }));
  expect(broken, "words and DPT tokens must not split into character fragments").toEqual([]);
}

export async function scrollsLocally(page: Page, table: Locator) {
  const viewport = table.locator(".resizable-table-scroll");
  expect(await viewport.evaluate(e => e.scrollWidth > e.clientWidth)).toBe(true);
  await viewport.evaluate(e => { e.scrollLeft = 100; });
  expect(await viewport.evaluate(e => e.scrollLeft)).toBeGreaterThan(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
}
