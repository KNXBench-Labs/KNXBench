/** Seed visibility once per tab so reload cannot mask automatic preference writes. */
import { expect, type Page } from "@playwright/test";
import { COLUMN_VISIBILITY_KEY } from "../src/useTableColumnVisibility";

export async function seedVisibility(page: Page, tableId: string, columns: readonly { id: string }[], only?: string) {
  const key = COLUMN_VISIBILITY_KEY + tableId;
  const raw = JSON.stringify(Object.fromEntries(columns.map(({ id }) => [id, id === only])));
  await page.addInitScript(({ key, raw }) => {
    const marker = `seeded:${key}`;
    if (!sessionStorage.getItem(marker)) {
      localStorage.setItem(key, raw);
      sessionStorage.setItem(marker, "yes");
    }
  }, { key, raw });
  return async () => expect(await page.evaluate(key => localStorage.getItem(key), key)).toBe(raw);
}
